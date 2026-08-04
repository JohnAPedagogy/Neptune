//! Widget calls: everything beyond `label` (which lives on `UiFrame` itself
//! in `context.rs`, since it needs no persistent state).

use std::ops::RangeInclusive;

use crate::input::MouseButton;
use crate::math::{Aabb2d, Color, Vec2};

use super::context::{Ui, UiFrame};
use super::layout::WidgetId;

/// Standard row height for every widget in this file.
const ROW_HEIGHT: f32 = 22.0;
/// Left column reserved for a widget's label.
const LABEL_WIDTH: f32 = 90.0;
/// Right column reserved for a slider's live value readout.
const VALUE_WIDTH: f32 = 56.0;

/// What a widget call reports about this frame — the dat.gui `.onChange`
/// translated to a return value instead of a registered callback (see
/// `neptune-imgui-plus-datgui.md` §2).
#[derive(Debug, Clone, Copy, Default)]
pub struct Response {
    pub(crate) changed: bool,
}

impl Response {
    pub fn changed(&self) -> bool {
        self.changed
    }
}

impl<'a> UiFrame<'a> {
    /// A draggable value in `range`, dat.gui's `gui.add(obj, 'prop', min, max)`.
    pub fn slider(&mut self, label: &str, value: &mut f32, range: RangeInclusive<f32>) -> Response {
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT);
        let track = Aabb2d::new(
            Vec2::new(row.min.x + LABEL_WIDTH, row.min.y + 6.0),
            Vec2::new(row.max.x - VALUE_WIDTH, row.max.y - 6.0),
        );

        self.push_text(row.min, label, Color::WHITE);
        self.push_quad(track, Color::rgba(0.2, 0.2, 0.24, 1.0));

        let (min, max) = (*range.start(), *range.end());
        let mut changed = false;

        if self.mouse.just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.mouse.position() {
                if track.contains_point(Vec2::new(x, y)) {
                    self.ui.active_drag = Some(id);
                }
            }
        }

        if self.mouse.held(MouseButton::Left) && self.ui.active_drag == Some(id) {
            if let Some((x, _)) = self.mouse.position() {
                let t = ((x - track.min.x) / track.size().x.max(1.0)).clamp(0.0, 1.0);
                let new_value = min + t * (max - min);
                if new_value != *value {
                    *value = new_value;
                    changed = true;
                }
            }
        }

        if self.mouse.just_released(MouseButton::Left) && self.ui.active_drag == Some(id) {
            self.ui.active_drag = None;
        }

        let t = ((*value - min) / (max - min).max(f32::EPSILON)).clamp(0.0, 1.0);
        let thumb_w = 8.0;
        let thumb_x = track.min.x + t * (track.size().x - thumb_w).max(0.0);
        let thumb = Aabb2d::new(
            Vec2::new(thumb_x, track.min.y),
            Vec2::new(thumb_x + thumb_w, track.max.y),
        );
        self.push_quad(thumb, Color::rgba(0.6, 0.7, 1.0, 1.0));
        self.push_text(
            Vec2::new(row.max.x - VALUE_WIDTH + 4.0, row.min.y),
            &format!("{value:.2}"),
            Color::WHITE,
        );

        Response { changed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{MouseButton, MouseState};
    use crate::math::Vec2;
    use crate::text::Font;
    use winit::dpi::PhysicalPosition;
    use winit::event::ElementState;

    fn ui() -> Option<Ui> {
        let atlas = Font::system_default().ok()?.atlas(24.0).ok()?;
        Some(Ui::new(atlas))
    }

    fn press_at(mouse: &mut MouseState, x: f64, y: f64) {
        mouse.handle_cursor_moved(PhysicalPosition::new(x, y));
        mouse.handle_button_event(MouseButton::Left, ElementState::Pressed);
    }

    #[test]
    fn dragging_inside_the_track_updates_the_value_and_reports_changed() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;

        // Frame 1: press at the track's left edge (label reserves 90px).
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 95.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.slider("Speed", &mut value, 0.0..=10.0);
            assert!(response.changed(), "the press itself starts and applies a drag");
        }
        mouse.end_frame();

        // Frame 2: drag to the middle of the track.
        mouse.handle_cursor_moved(PhysicalPosition::new(180.0, 11.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.slider("Speed", &mut value, 0.0..=10.0);
            assert!(response.changed());
        }
        assert!(value > 0.0 && value < 10.0, "got {value}");
    }

    #[test]
    fn clicking_outside_the_track_does_not_start_a_drag() {
        let Some(mut ui) = ui() else { return };
        let mut value = 3.0f32;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 5.0, 500.0); // far outside any row

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.slider("Speed", &mut value, 0.0..=10.0);
        assert!(!response.changed());
        assert_eq!(value, 3.0);
    }

    #[test]
    fn the_value_never_leaves_its_range() {
        let Some(mut ui) = ui() else { return };
        let mut value = 5.0f32;
        let mut mouse = MouseState::new();
        // Press far to the right of the track — clamps to max, not off-scale.
        press_at(&mut mouse, 10_000.0, 11.0);

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.slider("Speed", &mut value, 0.0..=10.0);
        assert!((0.0..=10.0).contains(&value), "got {value}");
    }

    #[test]
    fn releasing_the_button_stops_the_drag() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 95.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.slider("Speed", &mut value, 0.0..=10.0);
        }
        mouse.end_frame();
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);

        let stalled = value;
        // No button held: moving the mouse must not move the slider.
        mouse.handle_cursor_moved(PhysicalPosition::new(250.0, 11.0));
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.slider("Speed", &mut value, 0.0..=10.0);
        assert_eq!(value, stalled);
    }
}
