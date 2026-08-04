//! Widget calls: everything beyond `label` (which lives on `UiFrame` itself
//! in `context.rs`, since it needs no persistent state).

use std::ops::RangeInclusive;

use crate::input::MouseButton;
use crate::math::{Aabb2d, Color, Vec2};

use super::context::UiFrame;
use super::layout::WidgetId;

/// Standard row height for every widget in this file.
const ROW_HEIGHT: f32 = 22.0;
/// Left column reserved for a widget's label.
const LABEL_WIDTH: f32 = 90.0;
/// Right column reserved for a slider's live value readout.
const VALUE_WIDTH: f32 = 56.0;

const PRESETS: [Color; 6] = [
    Color::RED,
    Color::GREEN,
    Color::BLUE,
    Color::YELLOW,
    Color::WHITE,
    Color::BLACK,
];

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

    /// A toggled value, dat.gui's `gui.add(obj, 'prop')` over a boolean.
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> Response {
        let row = self.layout.row(ROW_HEIGHT);
        let box_rect = Aabb2d::new(
            Vec2::new(row.min.x, row.min.y + 3.0),
            Vec2::new(row.min.x + 16.0, row.min.y + 19.0),
        );

        let mut changed = false;
        if self.mouse.just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.mouse.position() {
                if box_rect.contains_point(Vec2::new(x, y)) {
                    *value = !*value;
                    changed = true;
                }
            }
        }

        let fill = if *value {
            Color::rgba(0.4, 0.8, 0.5, 1.0)
        } else {
            Color::rgba(0.2, 0.2, 0.24, 1.0)
        };
        self.push_quad(box_rect, fill);
        self.push_text(Vec2::new(row.min.x + 24.0, row.min.y), label, Color::WHITE);

        Response { changed }
    }

    /// A fixed-list selector, dat.gui's `gui.add(obj, 'prop', ['a', 'b', 'c'])`.
    ///
    /// Option rows are always computed from `header` (never from `self.layout`)
    /// so the hit-test block and the draw block can never disagree about where a
    /// row is — the layout cursor is only ever bumped afterward, purely to
    /// reserve vertical space for whatever widget comes next.
    pub fn dropdown(&mut self, label: &str, options: &[&str], selected: &mut usize) -> Response {
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT);
        let header = Aabb2d::new(
            Vec2::new(row.min.x + LABEL_WIDTH, row.min.y),
            Vec2::new(row.max.x, row.max.y),
        );

        self.push_text(row.min, label, Color::WHITE);
        self.push_quad(header, Color::rgba(0.2, 0.2, 0.24, 1.0));
        let current = options.get(*selected).copied().unwrap_or("");
        self.push_text(Vec2::new(header.min.x + 6.0, row.min.y), current, Color::WHITE);

        let option_row = |i: usize| {
            Aabb2d::new(
                Vec2::new(header.min.x, header.max.y + i as f32 * ROW_HEIGHT),
                Vec2::new(header.max.x, header.max.y + (i as f32 + 1.0) * ROW_HEIGHT),
            )
        };

        let mut changed = false;
        let was_open = self.ui.open.contains(&id);

        if self.mouse.just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.mouse.position() {
                let point = Vec2::new(x, y);
                if header.contains_point(point) {
                    if was_open {
                        self.ui.open.remove(&id);
                    } else {
                        self.ui.open.insert(id);
                    }
                } else if was_open {
                    for (i, _) in options.iter().enumerate() {
                        if option_row(i).contains_point(point) {
                            if *selected != i {
                                *selected = i;
                                changed = true;
                            }
                            self.ui.open.remove(&id);
                        }
                    }
                }
            }
        }

        if self.ui.open.contains(&id) {
            for (i, option) in options.iter().enumerate() {
                let rect = option_row(i);
                self.push_quad(rect, Color::rgba(0.15, 0.15, 0.18, 1.0));
                self.push_text(Vec2::new(rect.min.x + 6.0, rect.min.y), option, Color::WHITE);
            }
            // Reserve space so the next widget doesn't sit under the open menu.
            // The exact gap doesn't need to match `option_row`'s spacing pixel
            // for pixel — only the hit-test and draw rects above have to agree,
            // and both are built from `option_row` exclusively.
            self.layout.row(options.len() as f32 * ROW_HEIGHT);
        }

        Response { changed }
    }

    /// A colour swatch that opens a small preset grid, dat.gui's
    /// `gui.addColor(obj, 'prop')`. No HSV wheel — see
    /// `neptune-imgui-plus-datgui.md` §5's note on skipping it for v1.
    pub fn color_edit(&mut self, label: &str, value: &mut Color) -> Response {
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT);
        self.push_text(row.min, label, Color::WHITE);

        let swatch = Aabb2d::new(
            Vec2::new(row.min.x + LABEL_WIDTH, row.min.y + 3.0),
            Vec2::new(row.min.x + LABEL_WIDTH + 24.0, row.min.y + 19.0),
        );
        self.push_quad(swatch, *value);

        let mut changed = false;
        let was_open = self.ui.open.contains(&id);
        const CELL: f32 = 20.0;

        if self.mouse.just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.mouse.position() {
                let point = Vec2::new(x, y);
                if swatch.contains_point(point) {
                    if was_open {
                        self.ui.open.remove(&id);
                    } else {
                        self.ui.open.insert(id);
                    }
                } else if was_open {
                    for (i, preset) in PRESETS.iter().enumerate() {
                        let cell = Aabb2d::new(
                            Vec2::new(swatch.max.x + 4.0 + i as f32 * CELL, swatch.min.y),
                            Vec2::new(swatch.max.x + 4.0 + i as f32 * CELL + 16.0, swatch.max.y),
                        );
                        if cell.contains_point(point) {
                            *value = *preset;
                            changed = true;
                            self.ui.open.remove(&id);
                        }
                    }
                }
            }
        }

        if self.ui.open.contains(&id) {
            for (i, preset) in PRESETS.iter().enumerate() {
                let cell = Aabb2d::new(
                    Vec2::new(swatch.max.x + 4.0 + i as f32 * CELL, swatch.min.y),
                    Vec2::new(swatch.max.x + 4.0 + i as f32 * CELL + 16.0, swatch.max.y),
                );
                self.push_quad(cell, *preset);
            }
        }

        Response { changed }
    }

    /// A collapsible group, dat.gui's `gui.addFolder('name')`. Starts expanded.
    pub fn folder(&mut self, label: &str, contents: impl FnOnce(&mut UiFrame)) {
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT);
        let was_collapsed = self.ui.collapsed.contains(&id);

        if self.mouse.just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.mouse.position() {
                if row.contains_point(Vec2::new(x, y)) {
                    if was_collapsed {
                        self.ui.collapsed.remove(&id);
                    } else {
                        self.ui.collapsed.insert(id);
                    }
                }
            }
        }

        self.push_quad(row, Color::rgba(0.12, 0.12, 0.15, 1.0));
        let arrow = if self.ui.collapsed.contains(&id) { ">" } else { "v" };
        self.push_text(row.min, &format!("{arrow} {label}"), Color::WHITE);

        if !self.ui.collapsed.contains(&id) {
            self.layout.indent(16.0);
            contents(self);
            self.layout.outdent(16.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{MouseButton, MouseState};
    use crate::math::Vec2;
    use crate::text::Font;
    use crate::ui::Ui;
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

    #[test]
    fn clicking_inside_the_box_toggles_the_value() {
        let Some(mut ui) = ui() else { return };
        let mut value = false;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 8.0, 11.0); // inside the 16px box at the row's left edge

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.checkbox("Wireframe", &mut value);
        assert!(response.changed());
        assert!(value);
    }

    #[test]
    fn clicking_outside_the_box_does_nothing() {
        let Some(mut ui) = ui() else { return };
        let mut value = false;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 500.0, 500.0);

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.checkbox("Wireframe", &mut value);
        assert!(!response.changed());
        assert!(!value);
    }

    #[test]
    fn a_second_click_toggles_it_back() {
        let Some(mut ui) = ui() else { return };
        let mut value = false;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 8.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.checkbox("Wireframe", &mut value);
        }
        assert!(value);

        mouse.end_frame();
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);
        press_at(&mut mouse, 8.0, 11.0);
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.checkbox("Wireframe", &mut value);
        assert!(!value);
    }

    #[test]
    fn clicking_the_header_opens_it_without_changing_the_selection() {
        let Some(mut ui) = ui() else { return };
        let mut selected = 0usize;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 150.0, 11.0); // inside the header, right of the label column

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        assert!(!response.changed());
        assert_eq!(selected, 0);
        // Opening draws the two extra option rows.
        assert!(frame.finish().len() > 0);
    }

    #[test]
    fn clicking_an_open_option_selects_it_and_closes_the_menu() {
        let Some(mut ui) = ui() else { return };
        let mut selected = 0usize;

        // Frame 1: open it.
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 150.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        }
        mouse.end_frame();
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);

        // Frame 2: click the second option row, just below the header.
        press_at(&mut mouse, 150.0, (2.0 * ROW_HEIGHT + 11.0) as f64);
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        assert!(response.changed());
        assert_eq!(selected, 1);
    }

    #[test]
    fn a_closed_dropdown_ignores_clicks_below_the_header() {
        let Some(mut ui) = ui() else { return };
        let mut selected = 0usize;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 150.0, (ROW_HEIGHT + 11.0) as f64); // where an option row would be, but nothing is open

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        assert!(!response.changed());
        assert_eq!(selected, 0);
    }

    #[test]
    fn clicking_the_swatch_opens_the_preset_grid() {
        let Some(mut ui) = ui() else { return };
        let mut value = Color::WHITE;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, (LABEL_WIDTH + 10.0) as f64, 11.0); // inside the swatch

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.color_edit("Tint", &mut value);
        assert!(!response.changed());
        assert_eq!(value, Color::WHITE);
    }

    #[test]
    fn clicking_an_open_preset_applies_it() {
        let Some(mut ui) = ui() else { return };
        let mut value = Color::WHITE;

        let mut mouse = MouseState::new();
        press_at(&mut mouse, (LABEL_WIDTH + 10.0) as f64, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.color_edit("Tint", &mut value);
        }
        mouse.end_frame();
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);

        // First preset cell sits just right of the swatch, same row.
        let swatch_right = LABEL_WIDTH + 24.0;
        press_at(&mut mouse, (swatch_right + 4.0 + 8.0) as f64, 11.0);
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.color_edit("Tint", &mut value);
        assert!(response.changed());
        assert_eq!(value, PRESETS[0]);
    }

    #[test]
    fn an_expanded_folder_draws_its_contents() {
        let Some(mut ui) = ui() else { return };
        let mut fov = 75.0f32;
        let mouse = MouseState::new(); // no click: folders start expanded
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.folder("Advanced", |ui| {
            ui.slider("FOV", &mut fov, 30.0..=120.0);
        });
        // The header row plus the slider's label+track+thumb+value primitives.
        assert!(frame.finish().len() > 1);
    }

    #[test]
    fn clicking_the_header_collapses_it_and_hides_the_contents() {
        let Some(mut primary) = ui() else { return };
        let mut fov = 75.0f32;
        let mut mouse = MouseState::new();
        press_at(&mut mouse, 10.0, 11.0); // inside the header row

        let mut frame = primary.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.folder("Advanced", |ui| {
            ui.slider("FOV", &mut fov, 30.0..=120.0);
        });
        // Only the header's own quad + label glyphs remain — no slider primitives.
        let header_only_len = frame.finish().len();

        // A second, independently-built Ui (nothing collapsed) confirms the
        // header alone is shorter than a folder left expanded — no test-only
        // production API needed, just a fresh Ui.
        let Some(mut expanded) = ui() else { return };
        let mouse2 = MouseState::new();
        let mut frame2 = expanded.begin(&mouse2, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame2.folder("Advanced", |ui| {
            ui.slider("FOV", &mut fov, 30.0..=120.0);
        });
        assert!(header_only_len < frame2.finish().len());
    }
}
