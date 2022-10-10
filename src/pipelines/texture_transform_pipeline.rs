use std::{sync::Arc, collections::HashMap};

use ash::vk;
use ash::vk::CommandBuffer;

use log::*;
use crate::{ApiBase, FramebufferStorage, RenderPassSafe, DescriptorPoolSafe, DeviceSafe, SwapchainSafe, GraphicBase, RenderCamera, ServerTexture, Pools, TextureServer, TextureSafe, TextureTransform, FramebufferSafe, GPUMesh, BufferSafe};


pub struct TextureTransformPipeline {
    framebuffers : FramebufferStorage,
    pub renderpass : Arc<RenderPassSafe>,
    api : ApiBase,
    descriptor_pool : Arc<DescriptorPoolSafe>,
    descriptor_sets_texture : HashMap<usize, vk::DescriptorSet>,
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub descriptor_set_layouts : Vec<vk::DescriptorSetLayout>,
    extent : vk::Extent2D,
    input_formats : Vec<vk::Format>,
    output_formats : Vec<vk::Format>,
    plane : BufferSafe
}

impl Drop for TextureTransformPipeline {
    fn drop(&mut self) {
        unsafe {
            info!("Destroying grayscale pipeline...");
            self.api.device.device_wait_idle();

            unsafe {
                for dsl in &self.descriptor_set_layouts {
                    self.api.device.destroy_descriptor_set_layout(*dsl, None);
                }
                self.api.device.destroy_pipeline(self.pipeline, None);
                self.api.device.destroy_pipeline_layout(self.layout, None);
            }
        }
    }
}

impl TextureTransformPipeline {
    fn get_img_desc_set(logical_device : Arc<DeviceSafe>) -> vk::DescriptorSetLayout {
        let descriptorset_layout_binding_descs = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build()];
        let descriptorset_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&descriptorset_layout_binding_descs);
        let descriptorsetlayout = unsafe {
            logical_device.create_descriptor_set_layout(&descriptorset_layout_info, None)
        }.unwrap();
        descriptorsetlayout
    }

    fn init_base_pipeline(
        logical_device: &Arc<DeviceSafe>,
        swapchain: &SwapchainSafe,
        renderpass: &RenderPassSafe,
        input_formats : &[vk::Format],
        output_formats : &[vk::Format]) -> Result<(vk::Pipeline, vk::PipelineLayout, Vec<vk::DescriptorSetLayout>), Box<dyn std::error::Error>> {
            let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
                vk_shader_macros::include_glsl!("./shaders/screen_space/shader.vert", kind: vert),
            );
            let vertexshader_module =
                unsafe { logical_device.create_shader_module(&vertexshader_createinfo, None)? };
            let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder()
                .code(vk_shader_macros::include_glsl!("./shaders/screen_space/gamma.frag"));
            let fragmentshader_module =
                unsafe { logical_device.create_shader_module(&fragmentshader_createinfo, None)? };
            let mainfunctionname = std::ffi::CString::new("main").unwrap();
            let vertexshader_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertexshader_module)
                .name(&mainfunctionname);
            let fragmentshader_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragmentshader_module)
                .name(&mainfunctionname);
            let shader_stages = vec![vertexshader_stage.build(), fragmentshader_stage.build()];
    
            let vertex_attrib_descs = [vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 0,
                    offset: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                }];
    
            let vertex_binding_descs = [vk::VertexInputBindingDescription {
                binding: 0,
                stride: 4 * 3,
                input_rate: vk::VertexInputRate::VERTEX,
            }];
    
            let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_attrib_descs)
                .vertex_binding_descriptions(&vertex_binding_descs);
    
            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let viewports = [vk::Viewport {
                x: 0.,
                y: 0.,
                width: swapchain.extent.width as f32,
                height: swapchain.extent.height as f32,
                min_depth: 0.,
                max_depth: 1.,
            }];
            let scissors = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent,
            }];
    
            let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewports(&viewports)
                .scissors(&scissors);
            let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
                .line_width(1.0)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .cull_mode(vk::CullModeFlags::NONE)
                .polygon_mode(vk::PolygonMode::FILL);
            let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let colourblend_attachments = vec![vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .build(); output_formats.len()];
            let colourblend_info =
                vk::PipelineColorBlendStateCreateInfo::builder().attachments(&colourblend_attachments);
    
            let descriptorset_layout_binding_descs = [vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build()];
            let descriptorset_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&descriptorset_layout_binding_descs);
            let descriptorsetlayout = unsafe {
                logical_device.create_descriptor_set_layout(&descriptorset_layout_info, None)
            }?;
    
            let desc_set_color = TextureTransformPipeline::get_img_desc_set(logical_device.clone());
    
            let desclayouts = vec![desc_set_color; input_formats.len()];
            let pipelinelayout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&desclayouts);
    
            let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
    
            let pipelinelayout =
                unsafe { logical_device.create_pipeline_layout(&pipelinelayout_info, None) }?;
            let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_info)
                .input_assembly_state(&input_assembly_info)
                .viewport_state(&viewport_info)
                .rasterization_state(&rasterizer_info)
                .multisample_state(&multisampler_info)
                .depth_stencil_state(&depth_stencil_info)
                .color_blend_state(&colourblend_info)
                .layout(pipelinelayout)
                .render_pass(renderpass.inner)
                .subpass(0);
            let graphicspipeline = unsafe {
                logical_device
                    .create_graphics_pipelines(
                        vk::PipelineCache::null(),
                        &[pipeline_info.build()],
                        None,
                    )
                    .expect("A problem with the pipeline creation")
            }[0];
            unsafe {
                logical_device.destroy_shader_module(fragmentshader_module, None);
                logical_device.destroy_shader_module(vertexshader_module, None);
            }
            Ok((
                graphicspipeline,
                pipelinelayout,
                desclayouts
            ))
        }
    

    pub fn init_renderpass(
        base : &GraphicBase,
        input_formats : &[vk::Format],
        output_formats : &[vk::Format]
        ) -> Result<RenderPassSafe, vk::Result> {

            let attachments : Vec<vk::AttachmentDescription> = output_formats.iter().map(|f| {
               vk::AttachmentDescription::builder()
                   .load_op(vk::AttachmentLoadOp::CLEAR)
                   .store_op(vk::AttachmentStoreOp::STORE)
                   .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                   .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                   .initial_layout(vk::ImageLayout::UNDEFINED)
                   .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                   .samples(vk::SampleCountFlags::TYPE_1)
                   .format(*f)
                   .build()
            }).collect();

            let color_attachment_references : Vec<vk::AttachmentReference> = output_formats.iter().enumerate().map(|(idx, f)| {
                vk::AttachmentReference {
                    attachment : idx as u32,
                    layout : vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
                }
            }).collect();

            let subpasses = [vk::SubpassDescription::builder()
                .color_attachments(&color_attachment_references)
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .build()];
            let subpass_dependencies = [vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_subpass(0)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(
                    vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                )
                .build()];
            let renderpass_info = vk::RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(&subpasses)
                .dependencies(&subpass_dependencies);
            let renderpass = unsafe { base.device.create_render_pass(&renderpass_info, None)? };
        
            Ok(base.wrap_render_pass(renderpass))
        }

    pub fn new(
        graphic_base : &GraphicBase,
        pools : &Pools,
        input_formats : &[vk::Format],
        output_formats : &[vk::Format]) -> Result<Self, vk::Result> {
        let renderpass = TextureTransformPipeline::init_renderpass(&graphic_base, input_formats, output_formats).unwrap();

        let (pipeline, pipeline_layout, descriptor_set_layouts) =
        TextureTransformPipeline::init_base_pipeline(
                &graphic_base.device,
                &graphic_base.swapchain,
                &renderpass,
                input_formats,
                output_formats).unwrap();

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty : vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count : 1024
            },
        ];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1024)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe {
            graphic_base.device.create_descriptor_pool(&descriptor_pool_info, None)
        }.unwrap();

        let renderpass = Arc::new(renderpass);
        let framebuffer_storage = FramebufferStorage::new(&renderpass);

        let mut plane_buf = BufferSafe::new(
            &graphic_base.allocator,
            4 * 3 * 6,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            gpu_allocator::MemoryLocation::CpuToGpu
        ).unwrap();

        let plane_raw_data = [
            -1.0_f32, -1.0, 0.0,
            -1.0, 1.0, 0.0,
            1.0, -1.0, 0.0,
            -1.0, 1.0, 0.0,
            1.0, 1.0, 0.0,
            1.0, -1.0, 0.0
        ];

        plane_buf.fill(&plane_raw_data).unwrap();

        Ok(Self {
            pipeline,
            renderpass,
            descriptor_pool : Arc::new(DescriptorPoolSafe { pool: descriptor_pool, device: graphic_base.device.clone() }),
            descriptor_sets_texture : HashMap::new(),
            framebuffers: framebuffer_storage,
            layout: pipeline_layout,
            descriptor_set_layouts,
            extent : graphic_base.swapchain.extent,
            api : graphic_base.get_api_base(pools),
            input_formats : input_formats.to_vec(),
            output_formats : output_formats.to_vec(),
            plane : plane_buf
        })
    }

    fn update_tex_desc(&mut self, tex: &Arc<TextureSafe>) {
        unsafe {
            if self.descriptor_sets_texture.contains_key(&tex.index) == false {
                let imageinfo = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(tex.imageview)
                    .sampler(tex.sampler)
                    .build();

                info!("image layout {:?}", imageinfo.image_layout);

                let desc_layouts_texture =
                    vec![self.descriptor_set_layouts[0]; 1];
                let descriptor_set_allocate_info_texture = vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(self.descriptor_pool.pool)
                    .set_layouts(&desc_layouts_texture);
                self.descriptor_sets_texture.insert(tex.index, self.api.device.allocate_descriptor_sets(
                    &descriptor_set_allocate_info_texture).unwrap()[0]);

                let mut descriptorwrite_image = vk::WriteDescriptorSet::builder()
                    .dst_set(self.descriptor_sets_texture[&tex.index])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .build();

                descriptorwrite_image.descriptor_count = 1;
                descriptorwrite_image.p_image_info = &imageinfo;
                self.api.device.update_descriptor_sets(&[descriptorwrite_image], &[]);
            }
        }
    }
}

impl TextureTransform for TextureTransformPipeline {
    fn process(&mut self, cmd: CommandBuffer, fb: &Arc<FramebufferSafe>, input: Vec<Arc<TextureSafe>>) {

        let clearvalues = vec![
            vk::ClearValue {
                color : vk::ClearColorValue {
                    float32 : [0.0,0.0,0.0,0.0]
                }
            }; self.output_formats.len()
        ];

        let renderpass_begininfo = vk::RenderPassBeginInfo::builder()
            .render_pass(self.renderpass.inner)
            .framebuffer(fb.franebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent
            })
            .clear_values(&clearvalues);

        unsafe {

            for tex in &input {
                self.update_tex_desc(tex);
                tex.barrier(cmd,
                            vk::AccessFlags::SHADER_READ,
                            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            vk::PipelineStageFlags::FRAGMENT_SHADER);
            }

            self.api.device.cmd_begin_render_pass(
                cmd,
                &renderpass_begininfo,
                vk::SubpassContents::INLINE,
            );
            self.api.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            self.api.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.plane.buffer],
                &[0]);

            let desctiprors : Vec<vk::DescriptorSet> =
                input.iter().map(|tex| {
                    self.descriptor_sets_texture[&tex.index]
                }).collect();

            self.api.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &desctiprors,
                &[]
            );

            self.api.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.plane.buffer],
                &[0]);
            self.api.device.cmd_draw(cmd, 6, 1, 0, 0);

            self.api.device.cmd_end_render_pass(cmd);
        }
    }

    fn create_framebuffer(&mut self) -> Arc<FramebufferSafe> {
        let mut gbuffer_buf = vec![];
        for i in 0..self.output_formats.len() {
            let tex = Arc::new(TextureSafe::new(
                &self.api.allocator,
                &self.api.device,
                self.extent,
                self.output_formats[i],
                false));
            gbuffer_buf.push(tex);
        }
        self.framebuffers.get_framebuffer(&gbuffer_buf)
    }
}