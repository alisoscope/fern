
use crate::prelude::*;

/// Not thread safe
pub struct IoSystem {
    event_pump: sdl3::EventPump,
}

impl IoSystem {
    pub fn new(event_pump: sdl3::EventPump) -> Self {
        Self {
            event_pump,
        }
    }
    pub fn poll(&mut self, state: &mut IoState) -> anyhow::Result<bool> {
        let mut should_quit = false;
        let mut scroll = 0f32;

        for event in self.event_pump.poll_iter() {
            use sdl3::event::Event;
            match event {
                //_ => log_debug!(LogType::Warning, "Unhandled SDL3 event: {:?}.", event),
                Event::Quit { .. } => should_quit = true,
                Event::MouseWheel { y, .. } => {
                    scroll += y;
                },
                _ => (),
            }
        }

        state.mouse_state = self.event_pump.mouse_state();
        state.relative_mouse_state = self.event_pump.relative_mouse_state();
        state.scroll = scroll;
        
        Ok(should_quit)
    }
}

pub struct IoState {
    resized_window: bool,
    pub scroll: f32,
    pub mouse_state: sdl3::mouse::MouseState,
    pub relative_mouse_state: sdl3::mouse::RelativeMouseState,
}

impl IoState {
    pub fn new(io: &IoSystem) -> Self {
        Self {
            resized_window: false,
            scroll: 0f32,
            mouse_state: io.event_pump.mouse_state(),
            relative_mouse_state: io.event_pump.relative_mouse_state(),
        }
    }
}
