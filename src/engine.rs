use anyhow::Result;
use winit::{event::{Event, WindowEvent}, event_loop::EventLoop, window::{Window, WindowBuilder}};

pub struct Engine {
    event_loop: EventLoop<()>,
    window: Window,
}

impl Engine {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new().build(&event_loop)?;

        Ok(Self {
            event_loop,
            window,
        })
    }

    pub fn run(self) -> Result<()> {
        self.event_loop.run(|event, elwt| {
            match event {
                Event::WindowEvent { window_id, event } => if window_id == self.window.id() {
                    match event {
                        WindowEvent::Resized(_) => (),
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Destroyed => (),
                        WindowEvent::RedrawRequested => (),
                        _ => (),
                    }
                },
                _ => (),
            }
        })?;

        Ok(())
    }
}
