use std::sync::Arc;

use anyhow::Result;
use wgpu::{Color, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp};
use winit::{dpi::LogicalSize, event::{Event, WindowEvent}, event_loop::EventLoop, window::{Window, WindowBuilder}};

use crate::graphics_context::GraphicsContext;

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

pub struct Engine<'a> {
    event_loop: EventLoop<()>,
    window: Arc<Window>,
    gfx: GraphicsContext<'a>,
}

impl<'a> Engine<'a> {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        let window_size = LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);
        let window = Arc::new(WindowBuilder::new().with_inner_size(window_size).build(&event_loop)?);
        let gfx = GraphicsContext::new(window.clone(), WINDOW_WIDTH, WINDOW_HEIGHT)?;

        Ok(Self {
            event_loop,
            window,
            gfx,
        })
    }

    pub fn run(mut self) -> Result<()> {
        self.event_loop.run(|event, elwt| {
            match event {
                Event::WindowEvent { window_id, event } => if window_id == self.window.id() {
                    match event {
                        WindowEvent::Resized(new_size) => self.gfx.resize(new_size.width, new_size.height),
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Destroyed => (),
                        WindowEvent::RedrawRequested => {
                            if let Some((surface, view)) = self.gfx.get_frame() {
                                let mut encoder = self.gfx.create_command_encoder("primary");

                                encoder.begin_render_pass(&RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[Some(RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: Operations {
                                            load: LoadOp::Clear(Color::BLACK),
                                            store: StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });
                                
                                let commands = encoder.finish();
                                
                                self.gfx.submit([commands]);
                                surface.present();
                            }
                        },
                        _ => (),
                    }
                },
                _ => (),
            }
        })?;

        Ok(())
    }
}
