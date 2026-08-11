//! Frame-coherent keyboard and mouse state.

use std::collections::HashSet;

use super::mouse::MouseState;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::PhysicalKey;

pub use winit::keyboard::KeyCode;

/// Which keys are down, and which went down *this* frame.
///
/// The renderer feeds this from the window event stream and hands it to the
/// render-loop closure via [`Frame::input`](crate::renderer::Frame::input).
/// Nothing else in the engine touches it, so a game can poll it whenever it
/// likes during its frame. The same holds for the [`MouseState`] it carries:
/// cursor deltas, scroll, and button edges, all reset once per frame.
#[derive(Debug, Default, Clone)]
pub struct InputState {
    held: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
    /// Characters typed since the last frame boundary, in event order — what a
    /// focused text field consumes to insert. Auto-repeat counts, so holding a
    /// key types a stream of characters, exactly like the OS's own editing
    /// behaviour.
    text: Vec<String>,
    mouse: MouseState,
}

impl InputState {
    pub fn new() -> Self {
        InputState::default()
    }

    /// The mouse half of the frame's input.
    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    /// Mutable access for the renderer's event handler, which feeds raw winit
    /// mouse events into the [`MouseState`]. Nothing public reaches it.
    pub(crate) fn mouse_mut(&mut self) -> &mut MouseState {
        &mut self.mouse
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

    /// The characters typed since the last frame boundary, in event order.
    /// Empty for a frame with no keystrokes; non-printing keys (arrows,
    /// modifier, function) never appear here — they surface as `KeyCode`s via
    /// [`InputState::just_pressed`] instead.
    pub fn text(&self) -> &[String] {
        &self.text
    }

    /// Records one winit key event: captures any character the key produces
    /// (on press *and* auto-repeat, so held keys keep typing) and updates the
    /// held/edge sets, ignoring keys with no physical code.
    pub(crate) fn handle_key_event(&mut self, event: &KeyEvent) {
        if event.state == ElementState::Pressed {
            if let Some(text) = &event.text {
                self.push_typed(text);
            }
        }
        if let PhysicalKey::Code(code) = event.physical_key {
            self.set_key(code, event.state, event.repeat);
        }
    }

    /// Appends a typed character (or run of characters, e.g. a composed dead
    /// key pair) to this frame's text buffer. Split out of
    /// [`InputState::handle_key_event`] so it can be tested without fabricating
    /// a platform-specific winit `KeyEvent`.
    pub(crate) fn push_typed(&mut self, text: &str) {
        if !text.is_empty() {
            self.text.push(text.to_string());
        }
    }

    /// The state machine behind [`InputState::handle_key_event`], split out so
    /// it can be tested (and driven by widget tests) without fabricating a
    /// platform-specific winit event.
    pub(crate) fn set_key(&mut self, code: KeyCode, state: ElementState, repeat: bool) {
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
        self.text.clear();
        self.mouse.end_frame();
    }

    /// Drops all held keys. Used when the window loses focus, so a key held at
    /// the moment focus is lost does not stay stuck down forever.
    pub(crate) fn release_all(&mut self) {
        for key in self.held.drain() {
            self.just_released.insert(key);
        }
        self.mouse.release_all();
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

    #[test]
    fn typed_characters_are_captured_in_order_and_cleared_each_frame() {
        let mut input = InputState::new();
        input.push_typed("h");
        input.push_typed("i");
        assert_eq!(input.text(), &["h".to_string(), "i".to_string()]);

        input.end_frame();
        assert!(input.text().is_empty(), "text does not survive the frame");
    }

    #[test]
    fn empty_typed_text_is_ignored() {
        let mut input = InputState::new();
        input.push_typed("");
        assert!(input.text().is_empty());
    }
}
