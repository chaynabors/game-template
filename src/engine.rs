use std::{
    f32::consts::TAU,
    net::{SocketAddr, UdpSocket},
    sync::{mpsc::Receiver, Arc},
    time::Instant,
};

use glam::{vec3, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use tracing::error;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{camera::Camera, graphics::GraphicsSubsystem};

const WINDOW_SIZE: LogicalSize<u32> = LogicalSize::new(1280, 720);

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SynchronizedState {
    player_transforms: [Mat4; 4],
}

impl SynchronizedState {
    fn new() -> Self {
        Self::default()
    }
}

pub struct Engine {
    event_loop: EventLoop<()>,
    window: Arc<Window>,
    scale_factor: f64,
    graphics: GraphicsSubsystem,
    camera: Camera,
    sync_state: SynchronizedState,
    incoming_state: Receiver<SynchronizedState>,
}

impl Engine {
    pub fn new(address: Option<SocketAddr>) -> anyhow::Result<Self> {
        let event_loop = EventLoop::new()?;
        let window = Arc::new(
            WindowBuilder::new()
                .with_inner_size(WINDOW_SIZE)
                .build(&event_loop)?,
        );
        let scale_factor = window.scale_factor();
        let graphics = GraphicsSubsystem::new(&event_loop, window.clone())?;

        let camera = Camera {
            position: Vec3::new(-1.0, 1.0, -1.0),
            target: Vec3::ZERO,
            fov: 80_f32.to_radians(),
            near: 0.01,
        };

        let sync_state = SynchronizedState::new();

        let (tx, rx) = std::sync::mpsc::channel();
        let socket = UdpSocket::bind("127.0.0.1:12345")?;
        if let Some(addr) = address {
            socket.connect(addr);
        }

        std::thread::spawn(move || {
            // messages sometimes get segmented
            // messages don't always get to the receiver
            // messages don't always come in the same order that were sent

            let mut bytes = vec![];
            while let Ok(bytes_read) = socket.recv(&mut bytes) {
                if bytes.len() >= std::mem::size_of::<SynchronizedState>() {
                    // tx.send(rmp_serde:: SynchronizedState::)
                }
            }
        });

        Ok(Self {
            event_loop,
            window,
            scale_factor,
            graphics,
            camera,
            sync_state,
            incoming_state: rx,
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let start = Instant::now();
        self.event_loop.run(|event, elwt| match event {
            Event::WindowEvent { window_id, event } => {
                if window_id == self.window.id() {
                    match event {
                        WindowEvent::Resized(new_size) => {
                            if let Err(err) = self.graphics.resize_window(new_size) {
                                error!(%err, "Failed to resize the window");
                                elwt.exit();
                            }
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            self.scale_factor = scale_factor;
                        }
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Destroyed => (),
                        WindowEvent::RedrawRequested => {
                            // if let Some((surface, view)) = self.gfx.get_frame() {
                            //     let mut encoder = self.gfx.device.create_command_encoder(
                            //         &CommandEncoderDescriptor {
                            //             label: Some("encoder"),
                            //         },
                            //     );

                            //     let mut render_pass =
                            //         encoder.begin_render_pass(&RenderPassDescriptor {
                            //             label: None,
                            //             color_attachments: &[Some(RenderPassColorAttachment {
                            //                 view: &view,
                            //                 resolve_target: None,
                            //                 ops: Operations {
                            //                     load: LoadOp::Clear(Color::BLACK),
                            //                     store: StoreOp::Store,
                            //                 },
                            //             })],
                            //             depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                            //                 view: &self.gfx.depth_texture_view,
                            //                 depth_ops: Some(Operations {
                            //                     load: LoadOp::Clear(0.0),
                            //                     store: StoreOp::Store,
                            //                 }),
                            //                 stencil_ops: None,
                            //             }),
                            //             timestamp_writes: None,
                            //             occlusion_query_set: None,
                            //         });

                            //     render_pass.set_pipeline(&self.mesh_pipeline);
                            //     render_pass.set_vertex_buffer(0, self.turtle.positions.slice(..));
                            //     render_pass.set_vertex_buffer(1, self.turtle.colors.slice(..));
                            //     render_pass.set_index_buffer(self.turtle.indices.slice(..), IndexFormat::Uint16);

                            //     for transform in self.sync_state.player_transforms {
                            //         render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&PushConstants {
                            //             mvp: self.camera.view_projection(self.window_state.aspect_ratio()) * transform,
                            //         }));

                            //         render_pass.draw_indexed(0..self.turtle.index_count, 0, 0..1);
                            //     }

                            //     drop(render_pass);

                            //     self.gfx.submit([encoder.finish()]);
                            //     self.window_state.window.pre_present_notify();
                            //     surface.present();
                            // }
                        }
                        _ => (),
                    }
                }
            }
            Event::AboutToWait => {
                let elapsed = start.elapsed().as_secs_f32();
                self.sync_state.player_transforms[0] = Mat4::from_rotation_translation(
                    Quat::from_axis_angle(Vec3::Y, elapsed * 6.23 / TAU),
                    vec3(elapsed.sin(), 0.0, elapsed.cos()).normalize() * 3.0,
                );
                self.camera.position = vec3(0.2, 0.5, 0.2).normalize() * 8.0;
            }
            _ => (),
        })?;

        Ok(())
    }
}
