//! Frame-coherent keyboard state.

use std::collections::HashSet;

use winit::event::{ElementState, KeyEvent};
use winit::keyboard::PhysicalKey;

pub use winit::keyboard::KeyCode;

/// Which keys are down, and which went down *this* frame.
///
/// The renderer feeds this from the window event stream and hands it to the
/// render-loop closure via [`Frame::input`](crate::renderer::Frame::input).
/// Nothing else in the engine touches it, so a game can poll it whenever it
/// likes during its frame.
#[derive(Debug, Default, Clone)]
pub struct InputState {
    held: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        InputState::default()
    }

    /// Whether `key` is currently down.
    pub fn held(&self, key: KeyCode) -> bool {
        self.held.contains(&key)
    }

    /// Whether `key` went down during this frame. Auto-repeat does not count,
    /// so this fires exactly once per physical press — which is what a "flap"
    /// input wants.
    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }

    /// Whether `key` came up during this frame.
    pub fn just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }

    /// Whether any key at all went down this frame.
    pub fn any_just_pressed(&self) -> bool {
        !self.just_pressed.is_empty()
    }

    /// Records one winit key event, ignoring keys with no physical code.
    pub(crate) fn handle_key_event(&mut self, event: &KeyEvent) {
        if let PhysicalKey::Code(code) = event.physical_key {
            self.set_key(code, event.state, event.repeat);
        }
    }

    /// The state machine behind [`InputState::handle_key_event`], split out so
    /// it can be tested without fabricating a platform-specific winit event.
    fn set_key(&mut self, code: KeyCode, state: ElementState, repeat: bool) {
        match state {
            ElementState::Pressed => {
                // `repeat` is the OS auto-repeat; the key is already held, so
                // it must not re-trigger `just_pressed`.
                if !repeat && self.held.insert(code) {
                    self.just_pressed.insert(code);
                }
            }
            ElementState::Released => {
                if self.held.remove(&code) {
                    self.just_released.insert(code);
                }
            }
        }
    }

    /// Clears the edge-triggered sets. Called once per frame, after the user's
    /// render-loop closure has had a chance to read them.
    pub(crate) fn end_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Drops all held keys. Used when the window loses focus, so a key held at
    /// the moment focus is lost does not stay stuck down forever.
    pub(crate) fn release_all(&mut self) {
        for key in self.held.drain() {
            self.just_released.insert(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(input: &mut InputState, code: KeyCode) {
        input.set_key(code, ElementState::Pressed, false);
    }

    fn auto_repeat(input: &mut InputState, code: KeyCode) {
        input.set_key(code, ElementState::Pressed, true);
    }

    fn release(input: &mut InputState, code: KeyCode) {
        input.set_key(code, ElementState::Released, false);
    }

    #[test]
    fn a_fresh_input_state_has_nothing_down() {
        let input = InputState::new();
        assert!(!input.held(KeyCode::Space));
        assert!(!input.just_pressed(KeyCode::Space));
        assert!(!input.any_just_pressed());
    }

    #[test]
    fn a_press_registers_as_both_held_and_just_pressed() {
        let mut input = InputState::new();
        press(&mut input, KeyCode::Space);
        assert!(input.held(KeyCode::Space));
        assert!(input.just_pressed(KeyCode::Space));
        assert!(input.any_just_pressed());
    }

    #[test]
    fn just_pressed_lasts_exactly_one_frame_but_held_persists() {
        let mut input = InputState::new();
        press(&mut input, KeyCode::Space);
        input.end_frame();
        assert!(input.held(KeyCode::Space));
        assert!(!input.just_pressed(KeyCode::Space));
    }

    #[test]
    fn auto_repeat_does_not_retrigger_just_pressed() {
        let mut input = InputState::new();
        press(&mut input, KeyCode::Space);
        input.end_frame();
        auto_repeat(&mut input, KeyCode::Space);
        assert!(input.held(KeyCode::Space));
        assert!(!input.just_pressed(KeyCode::Space));
    }

    #[test]
    fn releasing_clears_held_and_reports_just_released() {
        let mut input = InputState::new();
        press(&mut input, KeyCode::Space);
        input.end_frame();
        release(&mut input, KeyCode::Space);
        assert!(!input.held(KeyCode::Space));
        assert!(input.just_released(KeyCode::Space));
        input.end_frame();
        assert!(!input.just_released(KeyCode::Space));
    }

    #[test]
    fn keys_are_tracked_independently() {
        let mut input = InputState::new();
        press(&mut input, KeyCode::Space);
        press(&mut input, KeyCode::Escape);
        release(&mut input, KeyCode::Space);
        assert!(!input.held(KeyCode::Space));
        assert!(input.held(KeyCode::Escape));
    }

    #[test]
    fn losing_focus_releases_every_held_key() {
        let mut input = InputState::new();
        press(&mut input, KeyCode::KeyW);
        press(&mut input, KeyCode::KeyA);
        input.end_frame();

        input.release_all();
        assert!(!input.held(KeyCode::KeyW));
        assert!(!input.held(KeyCode::KeyA));
        assert!(input.just_released(KeyCode::KeyW));
    }
}
