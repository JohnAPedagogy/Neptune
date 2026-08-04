//! Widget calls: everything beyond `label` (which lives on `UiFrame` itself
//! in `context.rs`, since it needs no persistent state).

use std::ops::RangeInclusive;

use crate::input::MouseButton;
use crate::math::{Aabb2d, Color, Vec2};

use super::context::{DockEdge, TextStyle, UiFrame};
use super::layout::{Layout, WidgetId};

/// Standard row height for every widget in this file.
const ROW_HEIGHT: f32 = 22.0;
/// Left column reserved for a widget's label.
const LABEL_WIDTH: f32 = 90.0;
/// Right column reserved for a slider's live value readout.
const VALUE_WIDTH: f32 = 56.0;
/// Pixel height of a window's draggable title bar.
const HEADER_HEIGHT: f32 = 24.0;
/// Dragging a window's header to within this distance of a screen edge snaps
/// it to that edge ([`DockEdge`]).
const DOCK_THRESHOLD: f32 = 24.0;
/// Gap between windows docked to the same screen edge.
const DOCK_GUTTER: f32 = 8.0;

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
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT * ppp);
        let track = Aabb2d::new(
            Vec2::new(row.min.x + LABEL_WIDTH * ppp, row.min.y + 6.0 * ppp),
            Vec2::new(row.max.x - VALUE_WIDTH * ppp, row.max.y - 6.0 * ppp),
        );

        self.push_text(row.min, label, TextStyle::Body, Color::WHITE);
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
        let thumb_w = 8.0 * ppp;
        let thumb_x = track.min.x + t * (track.size().x - thumb_w).max(0.0);
        let thumb = Aabb2d::new(
            Vec2::new(thumb_x, track.min.y),
            Vec2::new(thumb_x + thumb_w, track.max.y),
        );
        self.push_quad(thumb, Color::rgba(0.6, 0.7, 1.0, 1.0));
        self.push_text(
            Vec2::new(row.max.x - VALUE_WIDTH * ppp + 4.0 * ppp, row.min.y),
            &format!("{value:.2}"),
            TextStyle::Body,
            Color::WHITE,
        );

        Response { changed }
    }

    /// A toggled value, dat.gui's `gui.add(obj, 'prop')` over a boolean.
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> Response {
        let ppp = self.pixels_per_point();
        let row = self.layout.row(ROW_HEIGHT * ppp);
        let box_rect = Aabb2d::new(
            Vec2::new(row.min.x, row.min.y + 3.0 * ppp),
            Vec2::new(row.min.x + 16.0 * ppp, row.min.y + 19.0 * ppp),
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
        self.push_text(
            Vec2::new(row.min.x + 24.0 * ppp, row.min.y),
            label,
            TextStyle::Body,
            Color::WHITE,
        );

        Response { changed }
    }

    /// A fixed-list selector, dat.gui's `gui.add(obj, 'prop', ['a', 'b', 'c'])`.
    ///
    /// Option rows are always computed from `header` (never from `self.layout`)
    /// so the hit-test block and the draw block can never disagree about where a
    /// row is — the layout cursor is only ever bumped afterward, purely to
    /// reserve vertical space for whatever widget comes next.
    pub fn dropdown(&mut self, label: &str, options: &[&str], selected: &mut usize) -> Response {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row_height = ROW_HEIGHT * ppp;
        let row = self.layout.row(row_height);
        let header = Aabb2d::new(
            Vec2::new(row.min.x + LABEL_WIDTH * ppp, row.min.y),
            Vec2::new(row.max.x, row.max.y),
        );

        self.push_text(row.min, label, TextStyle::Body, Color::WHITE);
        self.push_quad(header, Color::rgba(0.2, 0.2, 0.24, 1.0));
        let current = options.get(*selected).copied().unwrap_or("");
        self.push_text(
            Vec2::new(header.min.x + 6.0 * ppp, row.min.y),
            current,
            TextStyle::Body,
            Color::WHITE,
        );

        let option_row = |i: usize| {
            Aabb2d::new(
                Vec2::new(header.min.x, header.max.y + i as f32 * row_height),
                Vec2::new(header.max.x, header.max.y + (i as f32 + 1.0) * row_height),
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
                self.push_text(
                    Vec2::new(rect.min.x + 6.0 * ppp, rect.min.y),
                    option,
                    TextStyle::Body,
                    Color::WHITE,
                );
            }
            // Reserve space so the next widget doesn't sit under the open menu.
            // The exact gap doesn't need to match `option_row`'s spacing pixel
            // for pixel — only the hit-test and draw rects above have to agree,
            // and both are built from `option_row` exclusively.
            self.layout.row(options.len() as f32 * row_height);
        }

        Response { changed }
    }

    /// A colour swatch that opens a small preset grid, dat.gui's
    /// `gui.addColor(obj, 'prop')`. No HSV wheel — see
    /// `neptune-imgui-plus-datgui.md` §5's note on skipping it for v1.
    pub fn color_edit(&mut self, label: &str, value: &mut Color) -> Response {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT * ppp);
        self.push_text(row.min, label, TextStyle::Body, Color::WHITE);

        let swatch = Aabb2d::new(
            Vec2::new(row.min.x + LABEL_WIDTH * ppp, row.min.y + 3.0 * ppp),
            Vec2::new(row.min.x + LABEL_WIDTH * ppp + 24.0 * ppp, row.min.y + 19.0 * ppp),
        );
        self.push_quad(swatch, *value);

        let mut changed = false;
        let was_open = self.ui.open.contains(&id);
        let cell = 20.0 * ppp;

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
                        let cell_rect = Aabb2d::new(
                            Vec2::new(swatch.max.x + 4.0 * ppp + i as f32 * cell, swatch.min.y),
                            Vec2::new(
                                swatch.max.x + 4.0 * ppp + i as f32 * cell + 16.0 * ppp,
                                swatch.max.y,
                            ),
                        );
                        if cell_rect.contains_point(point) {
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
                let cell_rect = Aabb2d::new(
                    Vec2::new(swatch.max.x + 4.0 * ppp + i as f32 * cell, swatch.min.y),
                    Vec2::new(
                        swatch.max.x + 4.0 * ppp + i as f32 * cell + 16.0 * ppp,
                        swatch.max.y,
                    ),
                );
                self.push_quad(cell_rect, *preset);
            }
        }

        Response { changed }
    }

    /// A collapsible group, dat.gui's `gui.addFolder('name')`. Starts expanded.
    pub fn folder(&mut self, label: &str, contents: impl FnOnce(&mut UiFrame)) {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(ROW_HEIGHT * ppp);
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
        self.push_text(
            row.min,
            &format!("{arrow} {label}"),
            TextStyle::Body,
            Color::WHITE,
        );

        if !self.ui.collapsed.contains(&id) {
            let indent = 16.0 * ppp;
            self.layout.indent(indent);
            contents(self);
            self.layout.outdent(indent);
        }
    }

    /// A draggable, dockable panel: a title bar the user grabs to move the
    /// whole window, with `contents` laid out below it. Dragging the title bar
    /// to within [`DOCK_THRESHOLD`] of a screen edge snaps the window to that
    /// edge ([`DockEdge`]); windows docked to the same edge share it, stacking
    /// along the edge in the order they first appeared.
    ///
    /// Position and dock edge persist across frames (keyed by `label`), so the
    /// panel floats exactly where the user left it and re-snaps to the same
    /// edge when dragged there again. While a window is docked its origin is
    /// computed from the edge, so its `width`/height and those of its docked
    /// neighbours drive the layout.
    pub fn window(&mut self, label: &str, width: f32, contents: impl FnOnce(&mut UiFrame<'_>)) {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 1);
        let header_h = HEADER_HEIGHT * ppp;
        let threshold = DOCK_THRESHOLD * ppp;
        let float_margin = 24.0 * ppp;

        if !self.ui.panels.contains(&id) {
            self.ui.panels.push(id);
        }

        // Where the window was when the frame began: a docked window lives at
        // its edge-computed origin, a floating one where the user left it.
        let mut docked = self.ui.docked.get(&id).copied();
        let mut origin = if let Some(edge) = docked {
            self.docked_origin(id, edge, width)
        } else {
            self.ui.panel_origin.get(&id).copied().unwrap_or(Vec2::ZERO)
        };

        if self.mouse.just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.mouse.position() {
                let header = Aabb2d::new(origin, Vec2::new(origin.x + width, origin.y + header_h));
                if header.contains_point(Vec2::new(x, y)) {
                    self.ui.active_drag = Some(id);
                    self.ui.grab_offset = Some(Vec2::new(x, y) - origin);
                    self.ui.grab_start = Some(Vec2::new(x, y));
                }
            }
        }

        if self.ui.active_drag == Some(id) {
            if self.mouse.held(MouseButton::Left) {
                if let Some((mx, my)) = self.mouse.position() {
                    let grab_mouse = Vec2::new(mx, my);
                    // A grab only becomes a drag once the cursor leaves the
                    // press point, so a click on a title bar neither moves nor
                    // un-docks the window — and a grab near the top edge is
                    // not mistaken for a dock-to-top drag.
                    let moving = self
                        .ui
                        .grab_start
                        .map(|start| (grab_mouse - start).length_squared() > 1.0)
                        .unwrap_or(false);
                    if moving {
                        if docked.is_some() {
                            // Dragging a docked window away floats it; it
                            // re-snaps when dragged near an edge below.
                            self.ui.docked.remove(&id);
                        }
                        let grab = self.ui.grab_offset.unwrap_or(Vec2::ZERO);
                        origin = grab_mouse - grab;
                        origin.x =
                            origin.x.clamp(-width + float_margin, self.screen.0 - float_margin);
                        origin.y = origin.y.clamp(0.0, self.screen.1 - float_margin);

                        let snapped = if mx <= threshold {
                            Some(DockEdge::Left)
                        } else if mx >= self.screen.0 - threshold {
                            Some(DockEdge::Right)
                        } else if my <= threshold {
                            Some(DockEdge::Top)
                        } else if my >= self.screen.1 - threshold {
                            Some(DockEdge::Bottom)
                        } else {
                            None
                        };
                        if let Some(edge) = snapped {
                            self.ui.docked.insert(id, edge);
                            docked = Some(edge);
                        } else {
                            self.ui.docked.remove(&id);
                            docked = None;
                        }
                    }
                }
            }
            if self.mouse.just_released(MouseButton::Left) {
                self.ui.active_drag = None;
            }
        }

        let origin = if let Some(edge) = docked {
            self.docked_origin(id, edge, width)
        } else {
            origin
        };

        // Lay the contents out below the header at full width, then splice the
        // panel chrome in front of them so the background never covers them.
        let prev_layout = self.layout.clone();
        self.layout = Layout::new(Vec2::new(origin.x, origin.y + header_h), width);
        let draw_start = self.draw_list.primitives.len();
        contents(self);
        let content_h = self.layout.cursor_y();
        self.layout = prev_layout;

        let panel_h = header_h + content_h;
        let panel = Aabb2d::new(origin, Vec2::new(origin.x + width, origin.y + panel_h));
        let contents_prims = self.draw_list.primitives.split_off(draw_start);
        self.push_quad(panel, Color::rgba(0.15, 0.16, 0.19, 0.96));
        let header = Aabb2d::new(origin, Vec2::new(origin.x + width, origin.y + header_h));
        self.push_quad(header, Color::rgba(0.22, 0.23, 0.28, 0.98));
        self.push_text(
            Vec2::new(origin.x + 8.0 * ppp, origin.y + 5.0 * ppp),
            label,
            TextStyle::Button,
            Color::WHITE,
        );
        self.draw_list.primitives.extend(contents_prims);

        self.ui.panel_origin.insert(id, origin);
        self.ui.panel_size.insert(id, Vec2::new(width, panel_h));
    }

    /// The edge-computed origin of a docked window: flush against the edge,
    /// past every sibling docked to the same edge that first appeared before
    /// it. Stored sizes from the previous frame drive the stacking, so a
    /// window's own height does not need to be known before its contents run.
    fn docked_origin(&self, id: WidgetId, edge: DockEdge, width: f32) -> Vec2 {
        let ppp = self.pixels_per_point();
        let gutter = DOCK_GUTTER * ppp;
        let height = self
            .ui
            .panel_size
            .get(&id)
            .map(|size| size.y)
            .unwrap_or(HEADER_HEIGHT * ppp);
        let mut x = gutter;
        let mut y = gutter;
        for sibling in &self.ui.panels {
            if self.ui.docked.get(sibling) == Some(&edge) {
                if *sibling == id {
                    break;
                }
                let size = self
                    .ui
                    .panel_size
                    .get(sibling)
                    .copied()
                    .unwrap_or(Vec2::new(width, HEADER_HEIGHT * ppp));
                match edge {
                    DockEdge::Left | DockEdge::Right => y += size.y + gutter,
                    DockEdge::Top | DockEdge::Bottom => x += size.x + gutter,
                }
            }
        }
        match edge {
            DockEdge::Left => Vec2::new(gutter, y),
            DockEdge::Right => Vec2::new(self.screen.0 - width - gutter, y),
            DockEdge::Top => Vec2::new(x, gutter),
            DockEdge::Bottom => Vec2::new(x, self.screen.1 - height - gutter),
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

    fn window_id(label: &str) -> WidgetId {
        WidgetId::new(label, 1)
    }

    #[test]
    fn a_window_draws_its_background_before_its_contents() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;
        let mouse = MouseState::new();
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.window("Settings", 200.0, |ui| {
            ui.slider("Speed", &mut value, 0.0..=10.0);
        });
        let list = frame.finish();
        // bg + header + header label, then the slider's own label/track/thumb/value.
        assert!(list.len() >= 6, "got {}", list.len());
        // The first primitive is the panel background, flush with the origin.
        assert_eq!(list.primitives[0].rect.min, Vec2::ZERO);
    }

    #[test]
    fn dragging_the_header_moves_the_window_and_it_stays_put() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;
        let mut mouse = MouseState::new();

        // Frame 1: place the window at the origin (no interaction).
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        mouse.end_frame();

        // Frame 2: grab the header at (150, 11). The origin is (0,0), so the
        // grab offset is the click point itself.
        press_at(&mut mouse, 150.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        mouse.end_frame();

        // Frame 3: still held, cursor moved by (100, 20): the origin follows.
        mouse.handle_cursor_moved(PhysicalPosition::new(250.0, 31.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
            assert_eq!(
                frame.ui.panel_origin[&window_id("Settings")],
                Vec2::new(100.0, 20.0)
            );
        }
        mouse.end_frame();

        // Frame 4: released — moving the cursor without a held button must not
        // move the window.
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);
        mouse.handle_cursor_moved(PhysicalPosition::new(400.0, 200.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
            assert_eq!(
                frame.ui.panel_origin[&window_id("Settings")],
                Vec2::new(100.0, 20.0)
            );
        }
    }

    #[test]
    fn dragging_a_window_to_the_left_edge_docks_it_flush_to_the_gutter() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;
        let mut mouse = MouseState::new();

        press_at(&mut mouse, 150.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        mouse.end_frame();

        // Drag to the left edge while still held.
        mouse.handle_cursor_moved(PhysicalPosition::new(4.0, 200.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
            assert_eq!(frame.ui.docked[&window_id("Settings")], DockEdge::Left);
            // Flush to the gutter, not parked at the drag clamp position.
            assert_eq!(
                frame.ui.panel_origin[&window_id("Settings")],
                Vec2::new(DOCK_GUTTER, DOCK_GUTTER)
            );
        }
    }

    #[test]
    fn docking_to_the_right_edge_anchors_the_window_to_it() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;
        let mut mouse = MouseState::new();

        press_at(&mut mouse, 150.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        mouse.end_frame();

        mouse.handle_cursor_moved(PhysicalPosition::new(796.0, 200.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
            assert_eq!(frame.ui.docked[&window_id("Settings")], DockEdge::Right);
            // Right edge minus width minus gutter, and a right-flush panel.
            let origin = frame.ui.panel_origin[&window_id("Settings")];
            assert_eq!(origin.x, 800.0 - 200.0 - DOCK_GUTTER);
        }
    }

    #[test]
    fn windows_docked_to_the_same_edge_stack_along_it_in_first_seen_order() {
        let Some(mut ui) = ui() else { return };
        let mut speed = 0.0f32;
        let mut fov = 75.0f32;
        let mut mouse = MouseState::new();

        // Dock "Main" to the left edge.
        press_at(&mut mouse, 150.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
        }
        mouse.end_frame();
        mouse.handle_cursor_moved(PhysicalPosition::new(4.0, 200.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
            assert_eq!(frame.ui.docked[&window_id("Main")], DockEdge::Left);
        }
        mouse.end_frame();
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);
        mouse.end_frame();

        // Park "Side" out of Main's way first: press at (150,11) moves its
        // origin to (400,300), which no longer overlaps Main's header.
        press_at(&mut mouse, 150.0, 11.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
        }
        mouse.end_frame();
        mouse.handle_cursor_moved(PhysicalPosition::new(550.0, 311.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
            assert_eq!(
                frame.ui.panel_origin[&window_id("Side")],
                Vec2::new(400.0, 300.0)
            );
        }
        mouse.end_frame();
        mouse.handle_button_event(MouseButton::Left, ElementState::Released);
        mouse.end_frame();

        // Re-grab "Side" where its header now is — clear of Main's — and drag
        // it to the left edge: it stacks below Main on the same edge.
        press_at(&mut mouse, 560.0, 311.0);
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
        }
        mouse.end_frame();
        mouse.handle_cursor_moved(PhysicalPosition::new(4.0, 500.0));
        {
            let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
            let main_h = frame.ui.panel_size[&window_id("Main")].y;
            let side = frame.ui.panel_origin[&window_id("Side")];
            assert_eq!(side.x, DOCK_GUTTER);
            assert_eq!(side.y, main_h + 2.0 * DOCK_GUTTER);
            assert_eq!(frame.ui.docked[&window_id("Side")], DockEdge::Left);
        }
    }
}
