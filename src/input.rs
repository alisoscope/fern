use std::collections::HashMap;
use std::hash;
use smallvec::SmallVec;
use crate::prelude::*;

pub const fn fnv1a(x: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64        = 0x00000100000001b3;

    let mut hash = FNV_OFFSET_BASIS;

    let mut i = 0;
    while i < x.len() {
        hash = hash ^ x[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }

    hash
}

#[derive(Debug)]
pub struct InputSystem {
    frame: u64,
    window_resized: Option<(u32, u32)>,
    input_action_names: IdentityHashMap<ActionKey, String>,
    input_action_states: IdentityHashMap<ActionKey, ActionState>,
    binding_to_actions: HashMap<ActionBinding, SmallVec<[ActionKey; 4]>>,
    pending_registered_actions: crossbeam_queue::SegQueue<(String, ActionBinding)>,
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            frame: 0,
            window_resized: None,
            input_action_names: HashMap::default(),
            input_action_states: HashMap::default(),
            binding_to_actions: HashMap::default(),
            pending_registered_actions: crossbeam_queue::SegQueue::new(),
        }
    }
    fn flush_pending_action_registration(&mut self) {
        while let Some((name, binding)) = self.pending_registered_actions.pop_mut() {
            let key = ActionKey::from_name(name.as_str());

            if let Some(old_name) = self.input_action_names.get(&key) {
                // sanity check that the input_actions and input_action_names both exist for this hash
                debug_assert!(self.input_action_states.get(&key).is_some());
                // check for hash collisions
                if old_name != &name {
                    panic!("InputSystem action name collision detected: {} == {}.", old_name, name);
                }
            } else {
                // this key hasn't been seen before, insert it into both hashmaps
                self.input_action_names.insert(key, name);
                self.input_action_states.insert(key, ActionState::default());
            }

            // update the binding_to_actions
            if let Some(actions) = self.binding_to_actions.get_mut(&binding) {
                // does the binding already contain the action? if it does do nothing
                if actions.iter().find(|&&contained_key| contained_key == key).is_none() {
                    // otherwise insert it
                    actions.push(key);
                }
            } else {
                // if the binding doesn't exist, create it with this action
                self.binding_to_actions.insert(binding, SmallVec::from_slice(&[key]));
            }
        }
    }
    /// Update the entire [`InputSystem`] state and poll events from [`sdl3::EventPump`].
    /// This must only be called once per frame, before any behaviour that relies on inputs being polled.
    pub fn update(&mut self, event_pump: &mut sdl3::EventPump) -> bool {
        // update frame counter
        self.frame += 1;

        // register pending actions
        self.flush_pending_action_registration();

        for event in event_pump.poll_iter() {
            use sdl3::event::Event;
            use sdl3::event::WindowEvent;
            let mut update_binding = |binding, state| {
                if let Some(keys) = self.binding_to_actions.get(&binding) {
                    for key in keys {
                        if let Some(old_state) = self.input_action_states.get_mut(key) {
                            *old_state = state;
                        }
                    }
                }
            };

            match event {
                Event::KeyDown { scancode: Some(scancode), .. } => update_binding(scancode.into(), ActionState::Down(self.frame)),
                Event::KeyUp { scancode: Some(scancode), .. } => update_binding(scancode.into(), ActionState::Up(self.frame)),
                Event::MouseButtonDown { mouse_btn, .. } => update_binding(mouse_btn.into(), ActionState::Down(self.frame)),
                Event::MouseButtonUp { mouse_btn, .. } => update_binding(mouse_btn.into(), ActionState::Up(self.frame)),
                Event::Quit { .. } => return true,
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(width, height) => self.window_resized = Some((width as u32, height as u32)),
                    _ => (),
                }
                _ => (),
            }
        }

        false
    }
    /// Returns a `Some((window_width, window_height))` if a window resize event has been polled since the last call to [`Self::take_window_resized`], otherwise `None`.
    pub fn take_window_resized(&mut self) -> Option<(u32, u32)> {
        self.window_resized.take()
    }
}

impl InputSystem {
    /// Gets the current frame.
    pub fn frame(&self) -> u64 {
        self.frame
    }
    /// Starts the process of registering an action. The action will only be available after a call
    /// to [`Self::update`], which should be called once at the beginning of every frame.
    pub fn register_action<T: ToString>(&self, name: T, action_binding: ActionBinding) -> ActionKey {
        let name = name.to_string();
        let key = ActionKey::from_name(name.as_str());
        self.pending_registered_actions.push((
            name, action_binding,
        ));
        
        key
    }
    pub fn get_action(&self, name: &str) -> Option<ActionKey> {
        let key = ActionKey::from_name(name);
        if self.input_action_names.contains_key(&key) {
            Some(key)
        } else {
            None
        }
    }
    /// Returns `true` if the action is down, `false` if it is up or doesn't exist.
    pub fn action_down(&self, action: ActionKey) -> bool {
        if let Some(ActionState::Down(_)) = self.input_action_states.get(&action) {
            true
        } else {
            false
        }
    }
    /// Returns `true` if the action is up, `false` if it is down or doesn't exist.
    pub fn action_up(&self, action: ActionKey) -> bool {
        if let Some(ActionState::Up(_)) = self.input_action_states.get(&action) {
            true
        } else {
            false
        }
    }
    /// Returns `true` if the action was pressed this frame, `false` otherwise or if it doesn't exist.
    pub fn action_pressed(&self, action: ActionKey) -> bool {
        if let Some(ActionState::Down(frame)) = self.input_action_states.get(&action) {
            *frame == self.frame
        } else {
            false
        }
    }
    /// Returns `true` if the action was released this frame, `false` otherwise or if it doesn't exist.
    pub fn action_released(&self, action: ActionKey) -> bool {
        if let Some(ActionState::Up(frame)) = self.input_action_states.get(&action) {
            *frame == self.frame
        } else {
            false
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum ActionBinding {
    Scancode(sdl3::keyboard::Scancode),
    MouseButton(sdl3::mouse::MouseButton),
}

impl From<sdl3::keyboard::Scancode> for ActionBinding {
    fn from(value: sdl3::keyboard::Scancode) -> Self {
        Self::Scancode(value)
    }
}

impl From<sdl3::mouse::MouseButton> for ActionBinding {
    fn from(value: sdl3::mouse::MouseButton) -> Self {
        Self::MouseButton(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionState {
    Up(u64),
    Down(u64),
}

impl Default for ActionState {
    fn default() -> Self {
        Self::Up(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionKey(u64);

impl ActionKey {
    fn from_name(name: &str) -> Self {
        Self(fnv1a(name.as_bytes()))
    }
}

impl hash::Hash for ActionKey {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0);
    }
}

type IdentityHashMap<S, V> = HashMap<S, V, hash::BuildHasherDefault<IdentityHasher>>;

#[derive(Default, Debug)]
struct IdentityHasher(u64);

impl hash::Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _bytes: &[u8]) {
        unreachable!("This hasher should only be used to hash u64 values with a custom hash::Hash implementation.");
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}
