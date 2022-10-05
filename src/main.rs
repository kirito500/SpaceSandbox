use std::io::Read;
use std::sync::Arc;
use ash::{vk};
use ash::vk::{BufferUsageFlags};
use byteorder::ByteOrder;
use gltf::{Semantic};
use gltf::buffer::{Source};
use gltf::json::accessor::ComponentType;


use log::*;

use SpaceSandbox::*;
use SpaceSandbox::asset_server::AssetServer;
use SpaceSandbox::MaterialTexture::{Diffuse, MetallicRoughness, Normal};
use SpaceSandbox::task_server::{TaskServer, TaskState};
use SpaceSandbox::ui::*;
use SpaceSandbox::game::*;

// for time measure wolfpld/tracy

fn init_rayon() {
    rayon::ThreadPoolBuilder::default()
        .num_threads(3)
        .build_global().unwrap();
}

fn main() {
    init_logger();
    init_rayon();

    let mut game = Game::default();
    let mut assets = AssetServer::new(&game);

    let mut camera = RenderCamera::new(&game.gb.allocator);
    camera.aspect = (game.gb.swapchain.extent.width as f32) / (game.gb.swapchain.extent.height as f32);
    camera.update_projectionmatrix();

    let mut gray_draw = SingleTexturePipeline::new(&game.gb, &camera).unwrap();

    let mut gbuffer_draw = GBufferFillPipeline::new(&game.gb, &camera).unwrap();

    let mut gbuffer_buf = vec![];
    for i in 0..3 {
        let tex = Arc::new(TextureSafe::new(
            &game.gb.allocator,
            &game.gb.device,
        vk::Extent2D {
                width : 1024,
                height : 1024
            },
            vk::Format::R32G32B32A32_SFLOAT,
            false));
        gbuffer_buf.push(tex);
    }
    {
        let tex = Arc::new(TextureSafe::new_depth(
            &game.gb.allocator,
            &game.gb.device,
            vk::Extent2D {
                width : 1024,
                height : 1024
            }));
        gbuffer_buf.push(tex);
    }

    info!("Finish loading");

    unsafe {
        game.gb.device.device_wait_idle().unwrap();
    }

    let command_buffers = create_commandbuffers(
        &game.gb.device,
        &game.pools,
        game.gb.swapchain.imageviews.len()
    ).unwrap();

    let mut show_task_list = false;
    let mut show_gltf = true;

    let mut fps_counter = FpsCounter::default();
    let mut api_window = ApiInfoWindow::new(&game.gb);
    let mut gltf_select = SelectGltfWindow::new(&assets);

    use winit::event::{Event, WindowEvent};

    game.simple_loop(
     move |game, event, _, controlflow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                unsafe {
                    info!("Finishing.....");
                    game.gb.device.device_wait_idle().expect("Waiting problem");
                }

                *controlflow = winit::event_loop::ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // doing the work here (later)
                game.gb.window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if let winit::event::KeyboardInput {
                    state: winit::event::ElementState::Pressed,
                    virtual_keycode: Some(keycode),
                    ..
                } = input
                {
                    let mut move_vector = nalgebra::Vector3::new(0.0, 0.0 ,0.0);
                    match keycode {
                        winit::event::VirtualKeyCode::Right => {
                            move_vector.x = 1.0;
                        }
                        winit::event::VirtualKeyCode::Left => {
                            move_vector.x = -1.0;
                        }
                        winit::event::VirtualKeyCode::Up => {
                            move_vector.y = 1.0;
                        }
                        winit::event::VirtualKeyCode::Down => {
                            move_vector.y = -1.0;
                        }
                        winit::event::VirtualKeyCode::PageUp => {
                            move_vector.z = 1.0;
                        }
                        winit::event::VirtualKeyCode::PageDown => {
                            move_vector.z = -1.0;
                        }
                        _ => {}
                    }

                    move_vector *= 0.1f32;
                    let frw = camera.view_direction;
                    let up = -camera.down_direction;
                    let right = camera.get_right_vector();
                    let mut dist = camera.position.magnitude();
                    dist += move_vector.z * dist;

                    let dp : nalgebra::Vector3<f32> = move_vector.x * right + up.scale( move_vector.y) + frw.scale(move_vector.z);
                    camera.position = camera.position + dp * dist;
                    camera.position = camera.position.normalize().scale(dist);
                    camera.view_direction = -camera.position.normalize();
                    camera.down_direction = camera.view_direction.cross(&right);

                    info!("{:#?}", camera.position);
                }
            }
            Event::RedrawRequested(_) => {
                //render here (later)
                // info!("Start frame!");
                let image_index = game.gb.next_frame();

                unsafe {

                    game.gui.integration.begin_frame();

                    egui::TopBottomPanel::top(0).show(&game.gui.integration.context(), |ui| {
                        ui.horizontal(|ui| {
                            if ui.button(format!("{} tasks running", game.task_server.get_task_count())).clicked() {
                                show_task_list = true;
                            }
                            if ui.button(format!("{:?}", &gray_draw.mode)).clicked() {
                                match gray_draw.mode {
                                    MaterialTexture::Diffuse => {
                                        gray_draw.mode = Normal;
                                    }
                                    MaterialTexture::Normal => {
                                        gray_draw.mode = MetallicRoughness;
                                    }
                                    MaterialTexture::MetallicRoughness => {
                                        gray_draw.mode = Diffuse;
                                    }
                                }
                            }
                            fps_counter.draw(ui);
                            api_window.draw(ui);
                        });
                    });

                    if show_gltf {
                        egui::Window::new("Select gltf").show(
                            &game.gui.integration.context(), |ui| {
                                if gltf_select.draw(ui, &mut assets, game) {
                                    show_gltf = false;
                                }
                            }
                        );
                    }

                    if show_task_list {
                        let win_res = egui::Window::new("Task list")
                            .show(&game.gui.integration.context(), |ui| {

                            if ui.button("Close").clicked() {
                                show_task_list = false;
                            }
                            let tasks = game.task_server.clone_task_list();
                            for t in tasks {
                                let state = t.get_state();
                                match state {
                                    TaskState::Created => {
                                        ui.label(t.get_name());
                                    }
                                    TaskState::Running => {
                                        ui.colored_label(egui::color::Color32::GREEN, t.get_name());
                                    }
                                    TaskState::Finished => {
                                        ui.colored_label(egui::color::Color32::RED, t.get_name());
                                    }
                                }
                            }
                        });
                    }

                    let (_, shapes) = game.gui.integration.end_frame(&mut game.gb.window);
                    let clipped_meshes = game.gui.integration.context().tessellate(shapes);

                    camera.update_viewmatrix();
                    camera.update_inner_buffer();

                    unsafe {
                        game.gb.device.begin_command_buffer(command_buffers[image_index as usize], &vk::CommandBufferBeginInfo::builder()).unwrap();
                    }

                    
                    gray_draw.update_commandbuffer(
                        command_buffers[image_index as usize],
                        &game.gb.device,
                        &game.gb.swapchain,
                        &game.render_server.render_models,
                        &assets.texture_server,
                        image_index as usize
                    ).unwrap();

                    gbuffer_draw.process(
                        command_buffers[image_index as usize],
                        &gbuffer_buf,
                            &game.render_server,
                            &assets);

                    game.gui.integration.paint(command_buffers[image_index as usize], image_index as usize, clipped_meshes);

                    unsafe {
                        game.gb.device.end_command_buffer(command_buffers[image_index as usize]).unwrap();
                    }

                    game.gb.end_frame(&command_buffers, image_index);

                    assets.texture_server.sync_tick();

                    unsafe {
                        // info!("Wait device");
                        // game.gb.device.device_wait_idle().unwrap();
                    }
                };
            }
            _ => {}
        }
    });
}

