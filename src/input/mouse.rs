//! Frame-coherent mouse state.

use std::collections::HashSet;

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseScrollDelta};

pub use winit::event::MouseButton;

/// Which buttons are down, where the cursor is, and how much it moved and
/// scrolled this frame.
///
/// The renderer feeds this from the window event stream; it rides inside
/// [`InputState`](crate::input::InputState) and reaches the render-loop closure
/// via [`Frame::input`](crate::renderer::Frame::input). The per-frame numbers —
/// cursor delta and net scroll — are accumulated across whatever events arrive
/// during one frame and reset by [`MouseState::end_frame`], so a camera can ask
/// "how far did the mouse move since the last frame" without tracking anything
/// itself.
#[derive(Debug, Default, Clone)]
pub struct MouseState {
    held: HashSet<MouseButton>,
    just_pressed: HashSet<MouseButton>,
    just_released: HashSet<MouseButton>,
    /// Cursor position this frame, in pixels relative to the window's top-left
    /// corner. `None` while the cursor is outside the window.
    position: Option<(f32, f32)>,
    /// Where the last `CursorMoved` put the cursor, for measuring deltas.
    last_position: Option<(f32, f32)>,
    /// Cumulative cursor movement since the last frame boundary, in pixels.
    delta: (f32, f32),
    /// Cumulative wheel/touchpad motion since the last frame boundary, counted
    /// in the event's own units (a wheel notch is roughly ±1).
    scroll: f32,
}

impl MouseState {
    pub fn new() -> Self {
        MouseState::default()
    }

    /// Whether `button` is currently held down.
    pub fn held(&self, button: MouseButton) -> bool {
        self.held.contains(&button)
    }

    /// Whether `button` went down during this frame.
    pub fn just_pressed(&self, button: MouseButton) -> bool {
        self.just_pressed.contains(&button)
    }

    /// Whether `button` came up during this frame.
    pub fn just_released(&self, button: MouseButton) -> bool {
        self.just_released.contains(&button)
    }

    /// Whether any button at all went down this frame.
    pub fn any_just_pressed(&self) -> bool {
        !self.just_pressed.is_empty()
    }

    /// Current cursor position in pixels relative to the window's top-left
    /// corner, or `None` if the cursor has not entered the window.
    pub fn position(&self) -> Option<(f32, f32)> {
        self.position
    }

    /// Cursor movement since the last frame boundary, as `(x, y)` pixels.
    ///
    /// Deliberately not raw pointer position: a camera control wants the *delta*
    /// from the previous frame, and winit only reports absolute positions, so
    /// the subtraction happens here, once, where the events arrive.
    pub fn delta(&self) -> (f32, f32) {
        self.delta
    }

    /// Net scroll-wheel (or touchpad) motion since the last frame boundary.
    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    /// Records one `CursorMoved`, accumulating the step since the last one into
    /// the per-frame delta.
    pub(crate) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let pos = (position.x as f32, position.y as f32);
        if let Some(last) = self.last_position {
            self.delta.0 += pos.0 - last.0;
            self.delta.1 += pos.1 - last.1;
        }
        self.last_position = Some(pos);
        self.position = Some(pos);
    }

    /// Records `CursorLeft`; the next `CursorMoved` starts a fresh delta.
    pub(crate) fn handle_cursor_left(&mut self) {
        self.position = None;
        self.last_position = None;
    }

    /// Records one mouse-button event.
    pub(crate) fn handle_button_event(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.held.insert(button) {
                    self.just_pressed.insert(button);
                }
            }
            ElementState::Released => {
                if self.held.remove(&button) {
                    self.just_released.insert(button);
                }
            }
        }
    }

    /// Records one `MouseWheel` event, keeping the vertical component. Line and
    /// pixel deltas are counted in their own units; either is enough to drive a
    /// zoom, and the two never mix in practice.
    pub(crate) fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => position.y as f32,
        };
        self.scroll += y;
    }

    /// Clears the per-frame edge triggers, deltas and scroll. Called once per
    /// frame, after the render-loop closure has read them.
    pub(crate) fn end_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.delta = (0.0, 0.0);
        self.scroll = 0.0;
        // Measure the next frame's moves from where the cursor now sits, so the
        // first event of a frame never shows a jump across the boundary.
        self.last_position = self.position;
    }

    /// Drops all held buttons. Used when the window loses focus, mirroring
    /// [`InputState::release_all`](crate::input::InputState::release_all).
    pub(crate) fn release_all(&mut self) {
        for button in self.held.drain() {
            self.just_released.insert(button);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;

    fn move_to(mouse: &mut MouseState, x: f64, y: f64) {
        mouse.handle_cursor_moved(PhysicalPosition::new(x, y));
    }

    fn press(mouse: &mut MouseState, button: MouseButton) {
        mouse.handle_button_event(button, ElementState::Pressed);
    }

    fn release(mouse: &mut MouseState, button: MouseButton) {
        mouse.handle_button_event(button, ElementState::Released);
    }

    #[test]
    fn a_fresh_mouse_has_no_buttons_or_cursor() {
        let mouse = MouseState::new();
        assert!(!mouse.held(MouseButton::Left));
        assert!(!mouse.any_just_pressed());
        assert_eq!(mouse.position(), None);
        assert_eq!(mouse.delta(), (0.0, 0.0));
        assert_eq!(mouse.scroll(), 0.0);
    }

    #[test]
    fn cursor_moves_accumulate_into_a_delta() {
        let mut mouse = MouseState::new();
        move_to(&mut mouse, 10.0, 10.0);
        assert_eq!(mouse.delta(), (0.0, 0.0), "first move has no reference point");
        move_to(&mut mouse, 15.0, 13.0);
        assert_eq!(mouse.delta(), (5.0, 3.0));
        move_to(&mut mouse, 11.0, 11.0);
        assert_eq!(mouse.delta(), (1.0, 1.0));
    }

    #[test]
    fn end_frame_resets_delta_but_not_measurement_origin() {
        let mut mouse = MouseState::new();
        move_to(&mut mouse, 10.0, 10.0);
        move_to(&mut mouse, 15.0, 10.0);
        assert_eq!(mouse.delta(), (5.0, 0.0));

        mouse.end_frame();
        assert_eq!(mouse.delta(), (0.0, 0.0));

        move_to(&mut mouse, 18.0, 10.0);
        assert_eq!(mouse.delta(), (3.0, 0.0), "measured from the frame boundary");
    }

    #[test]
    fn leaving_the_window_clears_the_cursor() {
        let mut mouse = MouseState::new();
        move_to(&mut mouse, 10.0, 10.0);
        mouse.handle_cursor_left();
        assert_eq!(mouse.position(), None);
        assert_eq!(mouse.delta(), (0.0, 0.0));
    }

    #[test]
    fn a_button_press_registers_as_both_held_and_just_pressed() {
        let mut mouse = MouseState::new();
        press(&mut mouse, MouseButton::Right);
        assert!(mouse.held(MouseButton::Right));
        assert!(mouse.just_pressed(MouseButton::Right));
        assert!(mouse.any_just_pressed());
    }

    #[test]
    fn just_pressed_lasts_exactly_one_frame() {
        let mut mouse = MouseState::new();
        press(&mut mouse, MouseButton::Left);
        mouse.end_frame();
        assert!(mouse.held(MouseButton::Left));
        assert!(!mouse.just_pressed(MouseButton::Left));
    }

    #[test]
    fn releasing_clears_held_and_reports_just_released() {
        let mut mouse = MouseState::new();
        press(&mut mouse, MouseButton::Left);
        mouse.end_frame();
        release(&mut mouse, MouseButton::Left);
        assert!(!mouse.held(MouseButton::Left));
        assert!(mouse.just_released(MouseButton::Left));
    }

    #[test]
    fn losing_focus_releases_every_held_button() {
        let mut mouse = MouseState::new();
        press(&mut mouse, MouseButton::Left);
        press(&mut mouse, MouseButton::Right);
        mouse.release_all();
        assert!(!mouse.held(MouseButton::Left));
        assert!(!mouse.held(MouseButton::Right));
        assert!(mouse.just_released(MouseButton::Right));
    }

    #[test]
    fn scroll_accumulates_and_resets() {
        let mut mouse = MouseState::new();
        mouse.handle_scroll(MouseScrollDelta::LineDelta(0.0, 1.0));
        mouse.handle_scroll(MouseScrollDelta::LineDelta(0.0, -0.5));
        assert_eq!(mouse.scroll(), 0.5);
        mouse.end_frame();
        assert_eq!(mouse.scroll(), 0.0);
    }

    #[test]
    fn pixel_scroll_is_counted_in_pixels() {
        let mut mouse = MouseState::new();
        mouse.handle_scroll(MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 24.0)));
        assert_eq!(mouse.scroll(), 24.0);
    }
}
