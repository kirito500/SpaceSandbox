use std::fs::File;
use std::ops::Deref;
use std::os::raw::c_char;
use std::sync::Arc;
use ash::{Device, Entry, Instance, vk};
use ash::extensions::{ext::DebugUtils, khr::Surface};
use ash::extensions::khr::Swapchain;
use ash::vk::{DeviceQueueCreateInfo, Handle, PhysicalDevice, PhysicalDeviceProperties, RenderPass, SurfaceKHR, SwapchainKHR};

use log::*;
use simplelog::*;
use winit::platform::unix::WindowExtUnix;
use winit::window::Window;

const EngineName : &str = "Rewin engine";
const AppName : &str = "SpaceSandbox";

pub mod swapchain_safe;
pub mod surface_safe;
pub mod instance_safe;
pub mod debug_layer;
pub mod vulkan_init_utils;
pub mod example_pipeline;
pub mod buffer_safe;

pub use swapchain_safe::*;
pub use surface_safe::*;
pub use instance_safe::*;
pub use debug_layer::*;
pub use vulkan_init_utils::*;
use example_pipeline::*;
pub use buffer_safe::*;

pub struct GraphicBase {
    pub instance : Arc<InstanceSafe>,
    pub debug : DebugDongXi,
    pub surfaces : Arc<SurfaceSafe>,
    pub physical_device : PhysicalDevice,
    pub physical_device_properties: vk::PhysicalDeviceProperties,
    pub queue_families : QueueFamilies,
    pub queues : Queues,
    pub device : Arc<DeviceSafe>,
    pub swapchain : SwapchainSafe,

    pub window : winit::window::Window,
    pub entry : Entry,
    pub allocator : Arc<AllocatorSafe>
}

impl GraphicBase {
    pub fn init(window : Window) -> Self {
        let entry = unsafe {ash::Entry::load().unwrap() };

        let mut extension_name_pointers : Vec<*const c_char> =
            ash_window::enumerate_required_extensions(&window).unwrap()
                .iter()
                .map(|&name| name.as_ptr())
                .collect();


        let layer_names = vec!["VK_LAYER_KHRONOS_validation"];
        let instance = Arc::new(init_instance(&entry, &layer_names, &window));
        let debug = DebugDongXi::init(&entry, &instance).unwrap();

        let (physical_device, physical_device_properties) = GetDefaultPhysicalDevice(&instance);

        let qfamindices = GetGraphicQueue(&instance, &physical_device);
        let (logical_device, queues) = GetLogicalDevice(
            &layer_names,
            &instance,
            physical_device,
            &qfamindices);
        let device = Arc::new(DeviceSafe {inner : logical_device.clone(), instance : instance.clone()});

        let surface = Arc::new(SurfaceSafe::new(&window, &instance, &entry));

        let swapchain = SwapchainSafe::new(
            &surface,
            physical_device,
            &qfamindices,
            &device,
            &instance);

        info!("Creating allocator create info...");
        
        let allocator_create_info = vk_mem::AllocatorCreateInfo {
            physical_device,
            device: logical_device.clone(),
            instance: instance.inner.clone(),
            flags: Default::default(),
            preferred_large_heap_block_size: 0,
            frame_in_use_count: 0,
            heap_size_limits: None
        };
        info!("Creating allocator...");
        let mut allocator =
            Arc::new(AllocatorSafe {
                inner : vk_mem::Allocator::new(&allocator_create_info).unwrap()
            });

        info!("Finished creating GraphicBase");

        Self {
            window,
            entry,
            instance,
            debug,
            surfaces : surface,
            physical_device,
            physical_device_properties,
            queue_families : qfamindices,
            queues,
            device,
            swapchain,
            allocator
        }
    }

    pub fn wrap_render_pass(&self, pass : RenderPass) -> RenderPassSafe {
        RenderPassSafe {
            inner : pass,
            device : self.device.clone()
        }
    }
}

pub struct AllocatorSafe {
    pub inner : vk_mem::Allocator
}

impl Deref for AllocatorSafe {
    type Target = vk_mem::Allocator;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for AllocatorSafe {
    fn drop(&mut self) {
        info!("Destroy allocator");
        unsafe {
            self.inner.destroy();
        }
    }
}

impl Drop for GraphicBase {
    fn drop(&mut self) {
        info!("Destroy GraphicBase");
    }
}

pub struct QueueFamilies {
    graphics_q_index: u32,
    transfer_q_index: u32,
}

pub struct Queues {
    pub graphics_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
}

pub struct DeviceSafe {
    pub inner : Device,
    instance : Arc<InstanceSafe>
}

impl Drop for DeviceSafe {
    fn drop(&mut self) {
        info!("Destroy device");
        unsafe {
            self.inner.destroy_device(None);
        }
    }
}

impl Deref for DeviceSafe {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct RenderPassSafe {
    pub inner : RenderPass,
    device : Arc<DeviceSafe>
}

impl Drop for RenderPassSafe {
    fn drop(&mut self) {
        info!("Destroy RenderPass");
        unsafe {
            self.device.destroy_render_pass(self.inner, None);
        }
    }
}

impl Deref for RenderPassSafe {
    type Target = RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}




pub fn init_renderpass(
   base : &GraphicBase
) -> Result<RenderPassSafe, vk::Result> {
    let attachments = [vk::AttachmentDescription::builder()
        .format(
            base.surfaces
                .get_formats(base.physical_device)?
                .first()
                .unwrap()
                .format,
        )
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .samples(vk::SampleCountFlags::TYPE_1)
        .build()];
    let color_attachment_references = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
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

pub struct Pools {
    commandpool_graphics: vk::CommandPool,
    commandpool_transfer: vk::CommandPool,
    device : Arc<DeviceSafe>
}

impl Drop for Pools {
    fn drop(&mut self) {
        info!("Destroy command pools");
        unsafe {
            self.device.destroy_command_pool(self.commandpool_graphics, None);
            self.device.destroy_command_pool(self.commandpool_transfer, None);
        }
    }
}

impl Pools {
    pub fn init(
        logical_device: &Arc<DeviceSafe>,
        queue_families: &QueueFamilies,
    ) -> Result<Pools, vk::Result> {
        let graphics_commandpool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.graphics_q_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let commandpool_graphics =
            unsafe { logical_device.create_command_pool(&graphics_commandpool_info, None) }?;
        let transfer_commandpool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.transfer_q_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let commandpool_transfer =
            unsafe { logical_device.create_command_pool(&transfer_commandpool_info, None) }?;

        Ok(Pools {
            commandpool_graphics,
            commandpool_transfer,
            device : logical_device.clone()
        })
    }
}

pub fn create_commandbuffers(
    logical_device: &ash::Device,
    pools: &Pools,
    amount: usize,
) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let commandbuf_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.commandpool_graphics)
        .command_buffer_count(amount as u32);
    unsafe { logical_device.allocate_command_buffers(&commandbuf_allocate_info) }
}

pub fn update_commandbuffer(
    commandbuffer : vk::CommandBuffer,
    logical_device: &ash::Device,
    renderpass: &vk::RenderPass,
    swapchain: &SwapchainSafe,
    pipeline: &ExamplePipeline,
    meshes : &Vec<GPUMesh>,
    descriptor_sets : &[vk::DescriptorSet],
    i : usize
) -> Result<(), vk::Result> {
    let commandbuffer_begininfo = vk::CommandBufferBeginInfo::builder();
    unsafe {
        logical_device.begin_command_buffer(commandbuffer, &commandbuffer_begininfo)?;
    }
    let clearvalues = [vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.08, 1.0],
        },
    }];
    let renderpass_begininfo = vk::RenderPassBeginInfo::builder()
        .render_pass(*renderpass)
        .framebuffer(swapchain.framebuffers[i])
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        })
        .clear_values(&clearvalues);
    unsafe {
        logical_device.cmd_begin_render_pass(
            commandbuffer,
            &renderpass_begininfo,
            vk::SubpassContents::INLINE,
        );
        logical_device.cmd_bind_pipeline(
            commandbuffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
        );
        logical_device.cmd_bind_descriptor_sets(
            commandbuffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.layout,
            0,
            &[descriptor_sets[i]],
            &[]
        );
        for mesh in meshes {
            logical_device.cmd_bind_vertex_buffers(commandbuffer, 0, &[mesh.pos_data.buffer], &[0]);
            logical_device.cmd_bind_index_buffer(commandbuffer, mesh.index_data.buffer, 0, vk::IndexType::UINT32);
            logical_device.cmd_draw_indexed(commandbuffer, mesh.vertex_count, 1, 0, 0, 0);
        }

        logical_device.cmd_end_render_pass(commandbuffer);
        logical_device.end_command_buffer(commandbuffer)?;
    }

    Ok(())
}

pub fn fill_commandbuffers(
    commandbuffers: &[vk::CommandBuffer],
    logical_device: &ash::Device,
    renderpass: &vk::RenderPass,
    swapchain: &SwapchainSafe,
    pipeline: &ExamplePipeline,
    meshes : &Vec<GPUMesh>,
    descriptor_sets : &[vk::DescriptorSet]
) -> Result<(), vk::Result> {
    for (i, &commandbuffer) in commandbuffers.iter().enumerate() {
        update_commandbuffer(commandbuffers[i],
            logical_device,
            renderpass,
            swapchain,
            pipeline,
                             meshes,
            descriptor_sets,
            i);
    }
    Ok(())
}

use nalgebra as na;
use vk_mem::ffi::VkResult;

pub struct Camera {
    viewmatrix : na::Matrix4<f32>,
    position: na::Vector3<f32>,
    view_direction: na::Unit<na::Vector3<f32>>,
    down_direction: na::Unit<na::Vector3<f32>>,
    fovy: f32,
    aspect: f32,
    near: f32,
    far: f32,
    projectionmatrix: na::Matrix4<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        let mut camera = Camera {
            viewmatrix: na::Matrix4::identity(),
            position: na::Vector3::new(0.0, 25.0, -100.0),
            view_direction: na::Unit::new_normalize(na::Vector3::new(0.0, 0.0, 1.0)),
            down_direction: na::Unit::new_normalize(na::Vector3::new(0.0, -1.0, 0.0)),
            fovy : std::f32::consts::FRAC_PI_3,
            aspect : 800.0 / 600.0,
            near : 0.1,
            far : 1000.0,
            projectionmatrix : na::Matrix4::identity()
        };
        camera.update_viewmatrix();
        camera.update_projectionmatrix();
        camera
    }
}

impl Camera {

    pub fn update_projectionmatrix(&mut self) {
        let d = 1.0 / (0.5 * self.fovy).tan();
        self.projectionmatrix = na::Matrix4::new(
            d / self.aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            d,
            0.0,
            0.0,
            0.0,
            0.0,
            self.far / (self.far - self.near),
            -self.near * self.far / (self.far - self.near),
            0.0,
            0.0,
            1.0,
            0.0,
        );
    }

    pub fn update_buffer(&self, buffer: &mut BufferSafe) {
        let data: [[[f32; 4]; 4]; 2] = [self.viewmatrix.into(), self.projectionmatrix.into()];
        buffer.fill(&data);
    }
    pub fn update_viewmatrix(&mut self) {
        let right = na::Unit::new_normalize(self.down_direction.cross(&self.view_direction));
        let m = na::Matrix4::new(
            right.x,
            right.y,
            right.z,
            -right.dot(&self.position), //
            self.down_direction.x,
            self.down_direction.y,
            self.down_direction.z,
            -self.down_direction.dot(&self.position), //
            self.view_direction.x,
            self.view_direction.y,
            self.view_direction.z,
            -self.view_direction.dot(&self.position), //
            0.0,
            0.0,
            0.0,
            1.0,
        );
        self.viewmatrix =  m;
    }
    pub fn move_forward(&mut self, distance: f32) {
        self.position += distance * self.view_direction.as_ref();
        self.update_viewmatrix();
    }
    pub fn move_backward(&mut self, distance: f32) {
        self.move_forward(-distance);
    }
    pub fn turn_right(&mut self, angle: f32) {
        let rotation = na::Rotation3::from_axis_angle(&self.down_direction, angle);
        self.view_direction = rotation * self.view_direction;
        self.update_viewmatrix();
    }
    pub fn turn_left(&mut self, angle: f32) {
        self.turn_right(-angle);
    }
    pub fn turn_up(&mut self, angle: f32) {
        let right = na::Unit::new_normalize(self.down_direction.cross(&self.view_direction));
        let rotation = na::Rotation3::from_axis_angle(&right, angle);
        self.view_direction = rotation * self.view_direction;
        self.down_direction = rotation * self.down_direction;
        self.update_viewmatrix();
    }
    pub fn turn_down(&mut self, angle: f32) {
        self.turn_up(-angle);
    }
}

pub struct GPUMesh {
    pub pos_data : BufferSafe,
    pub index_data : BufferSafe,
    pub vertex_count : u32
}


