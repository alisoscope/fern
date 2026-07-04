
use crate::graphics::PresentationSubSystemImpl;
use crate::prelude::*;
use crate::io::{IoSystem, IoState};
use crate::graphics;

pub struct App {
    //window: sdl3::video::Window,
    pub presentation: graphics::PresentationSubSystem,
    pub graphics: graphics::GraphicsSystem,
    sdl_video: sdl3::VideoSubsystem,
    io: IoSystem,
    sdl: sdl3::Sdl,
}

pub struct AppContext<'a> {
    pub presentation: &'a mut graphics::PresentationSubSystem,
    pub graphics: &'a graphics::GraphicsSystem,
    pub aspect_ratio: f32,
    pub io_state: &'a IoState,
}

impl App {
    pub fn new(title: &str) -> anyhow::Result<Self> {
        let sdl = sdl3::init()?;
        let event_pump = sdl.event_pump()?;
        let sdl_video = sdl.video()?;

        let window = sdl_video.window(title, 800, 640)
            .resizable()
            .build()?;

        let (graphics, presentation)
            = graphics::wgpu_backend::WGpuGraphicsSystem::init(window)?;

        let (graphics, presentation) = (graphics.into(), presentation.into()); 

        Ok(Self {
            presentation,
            graphics,
            sdl_video,
            io: IoSystem::new(event_pump),
            sdl,
        })
    }
    pub fn main_loop<F: FnMut(AppContext<'_>) -> anyhow::Result<()>>(&mut self, mut f: F) -> anyhow::Result<()> {
        let mut io_state = IoState::new(&self.io);

        self.sdl.mouse().set_relative_mouse_mode(self.presentation.window(), true);

        'main_loop: loop {
            // poll events
            // window resize -> vulkan recreate swapchain
            // create io structure (allows game logic to access keyboard, mouse etc.)
            // execute systems, rendering systems are executed on vulkan rendering threads
            // wait for vulkan to finish presenting

            if self.io.poll(&mut io_state)? {
                break;
            }


            let (width, height) = self.presentation.window_size();
            let aspect_ratio = (width as f32) / (height as f32);

            let a = AppContext {
                presentation: &mut self.presentation,
                graphics: &self.graphics,
                aspect_ratio,
                io_state: &io_state,
            };

            f(a)?;
        }

        Ok(())
    }
}
