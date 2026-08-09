//! Widget calls: everything beyond `label` (which lives on `UiFrame` itself
//! in `context.rs`, since it needs no persistent state).

use std::ops::RangeInclusive;

use crate::input::{KeyCode, MouseButton};
use crate::materials::Texture;
use crate::math::{Aabb2d, Color, Vec2};

use super::context::{DockEdge, TextStyle, UiFrame};
use super::draw_list::UiPrimitive;
use super::layout::{Layout, WidgetId};
use super::text::layout_text;

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
    /// Set by `button`: the button was pressed this frame.
    pub(crate) clicked: bool,
    /// Set by `selectable_label`: the row was clicked this frame.
    pub(crate) selected: bool,
}

impl Response {
    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn clicked(&self) -> bool {
        self.clicked
    }

    pub fn selected(&self) -> bool {
        self.selected
    }
}

impl<'a> UiFrame<'a> {
    /// A draggable value in `range`, dat.gui's `gui.add(obj, 'prop', min, max)`.
    pub fn slider(&mut self, label: &str, value: &mut f32, range: RangeInclusive<f32>) -> Response {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(self.ui.style.row_height * ppp);
        let track = Aabb2d::new(
            Vec2::new(row.min.x + self.ui.style.label_width * ppp, row.min.y + 6.0 * ppp),
            Vec2::new(row.max.x - self.ui.style.value_width * ppp, row.max.y - 6.0 * ppp),
        );

        self.push_text(row.min, label, TextStyle::Body, Color::WHITE);
        self.push_quad(track, self.ui.style.widget_bg);

        let (min, max) = (*range.start(), *range.end());
        let mut changed = false;

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                if track.contains_point(Vec2::new(x, y)) {
                    self.ui.active_drag = Some(id);
                }
            }
        }

        if self.input.mouse().held(MouseButton::Left) && self.ui.active_drag == Some(id) {
            if let Some((x, _)) = self.input.mouse().position() {
                let t = ((x - track.min.x) / track.size().x.max(1.0)).clamp(0.0, 1.0);
                let new_value = min + t * (max - min);
                if new_value != *value {
                    *value = new_value;
                    changed = true;
                }
            }
        }

        if self.input.mouse().just_released(MouseButton::Left) && self.ui.active_drag == Some(id) {
            self.ui.active_drag = None;
        }

        let t = ((*value - min) / (max - min).max(f32::EPSILON)).clamp(0.0, 1.0);
        let thumb_w = 8.0 * ppp;
        let thumb_x = track.min.x + t * (track.size().x - thumb_w).max(0.0);
        let thumb = Aabb2d::new(
            Vec2::new(thumb_x, track.min.y),
            Vec2::new(thumb_x + thumb_w, track.max.y),
        );
        self.push_quad(thumb, self.ui.style.accent);
        self.push_text(
            Vec2::new(row.max.x - self.ui.style.value_width * ppp + 4.0 * ppp, row.min.y),
            &format!("{value:.2}"),
            TextStyle::Body,
            Color::WHITE,
        );

        Response {
            changed,
            ..Default::default()
        }
    }

    /// A toggled value, dat.gui's `gui.add(obj, 'prop')` over a boolean.
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> Response {
        let ppp = self.pixels_per_point();
        let row = self.layout.row(self.ui.style.row_height * ppp);
        let box_rect = Aabb2d::new(
            Vec2::new(row.min.x, row.min.y + 3.0 * ppp),
            Vec2::new(row.min.x + 16.0 * ppp, row.min.y + 19.0 * ppp),
        );

        let mut changed = false;
        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                if box_rect.contains_point(Vec2::new(x, y)) {
                    *value = !*value;
                    changed = true;
                }
            }
        }

        let fill = if *value {
            self.ui.style.checkbox_on
        } else {
            self.ui.style.widget_bg
        };
        self.push_quad(box_rect, fill);
        self.push_text(
            Vec2::new(row.min.x + 24.0 * ppp, row.min.y),
            label,
            TextStyle::Body,
            Color::WHITE,
        );

        Response {
            changed,
            ..Default::default()
        }
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
        let row_height = self.ui.style.row_height * ppp;
        let row = self.layout.row(row_height);
        let header = Aabb2d::new(
            Vec2::new(row.min.x + self.ui.style.label_width * ppp, row.min.y),
            Vec2::new(row.max.x, row.max.y),
        );

        self.push_text(row.min, label, TextStyle::Body, Color::WHITE);
        self.push_quad(header, self.ui.style.widget_bg);
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

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
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
                self.push_quad(rect, self.ui.style.option_bg);
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

        Response {
            changed,
            ..Default::default()
        }
    }

    /// A colour swatch that opens a small preset grid, dat.gui's
    /// `gui.addColor(obj, 'prop')`. No HSV wheel — see
    /// `neptune-imgui-plus-datgui.md` §5's note on skipping it for v1.
    pub fn color_edit(&mut self, label: &str, value: &mut Color) -> Response {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(self.ui.style.row_height * ppp);
        self.push_text(row.min, label, TextStyle::Body, Color::WHITE);

        let swatch = Aabb2d::new(
            Vec2::new(row.min.x + self.ui.style.label_width * ppp, row.min.y + 3.0 * ppp),
            Vec2::new(row.min.x + self.ui.style.label_width * ppp + 24.0 * ppp, row.min.y + 19.0 * ppp),
        );
        self.push_quad(swatch, *value);

        let mut changed = false;
        let was_open = self.ui.open.contains(&id);
        let cell = 20.0 * ppp;

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
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

        // RGB numeric fields on a second row (Task 26): drag each channel in
        // 0..1, reusing the same pixel-to-value drag as `drag_value`. The
        // preset grid above stays where it was, so the two never overlap.
        let rgb_row = self.layout.row(self.ui.style.row_height * ppp);
        let x0 = rgb_row.min.x + self.ui.style.label_width * ppp;
        let per = ((rgb_row.max.x - x0) / 3.0).max(1.0);
        for (ci, (letter, component)) in ["R", "G", "B"]
            .iter()
            .zip([&mut value.r, &mut value.g, &mut value.b])
            .enumerate()
        {
            let channel_id = WidgetId::new(&format!("{label}.{letter}"), 0);
            let rect = Aabb2d::new(
                Vec2::new(x0 + ci as f32 * per, rgb_row.min.y),
                Vec2::new(x0 + (ci as f32 + 1.0) * per, rgb_row.max.y),
            );
            if self.drag_number(channel_id, rect, component, 0.0..=1.0) {
                changed = true;
            }
            self.push_quad(rect, self.ui.style.widget_bg);
            self.push_text(
                Vec2::new(rect.min.x + 3.0 * ppp, rgb_row.min.y),
                &format!("{letter} {:.2}", *component),
                TextStyle::Body,
                self.ui.style.text,
            );
        }

        Response {
            changed,
            ..Default::default()
        }
    }

    /// A collapsible group, dat.gui's `gui.addFolder('name')`. Starts expanded.
    pub fn folder(&mut self, label: &str, contents: impl FnOnce(&mut UiFrame)) {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(self.ui.style.row_height * ppp);
        let was_collapsed = self.ui.collapsed.contains(&id);

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                if row.contains_point(Vec2::new(x, y)) {
                    if was_collapsed {
                        self.ui.collapsed.remove(&id);
                    } else {
                        self.ui.collapsed.insert(id);
                    }
                }
            }
        }

        self.push_quad(row, self.ui.style.folder_bg);
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
    /// to within [`Style::dock_threshold`] of a screen edge snaps the window to
    /// that edge ([`DockEdge`]); windows docked to the same edge share it,
    /// stacking along the edge in the order they first appeared.
    ///
    /// Position and dock edge persist across frames (keyed by `label`), so the
    /// panel floats exactly where the user left it and re-snaps to the same
    /// edge when dragged there again. While a window is docked its origin is
    /// computed from the edge, so its `width`/height and those of its docked
    /// neighbours drive the layout.
    pub fn window(&mut self, label: &str, width: f32, contents: impl FnOnce(&mut UiFrame<'_>)) {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 1);
        let header_h = self.ui.style.header_height * ppp;
        let threshold = self.ui.style.dock_threshold * ppp;
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

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                let header = Aabb2d::new(origin, Vec2::new(origin.x + width, origin.y + header_h));
                if header.contains_point(Vec2::new(x, y)) {
                    self.ui.active_drag = Some(id);
                    self.ui.grab_offset = Some(Vec2::new(x, y) - origin);
                    self.ui.grab_start = Some(Vec2::new(x, y));
                }
            }
        }

        if self.ui.active_drag == Some(id) {
            if self.input.mouse().held(MouseButton::Left) {
                if let Some((mx, my)) = self.input.mouse().position() {
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
            if self.input.mouse().just_released(MouseButton::Left) {
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
        self.push_quad(panel, self.ui.style.panel_bg);
        let header = Aabb2d::new(origin, Vec2::new(origin.x + width, origin.y + header_h));
        self.push_quad(header, self.ui.style.panel_header);
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
        let gutter = self.ui.style.dock_gutter * ppp;
        let height = self
            .ui
            .panel_size
            .get(&id)
            .map(|size| size.y)
            .unwrap_or(self.ui.style.header_height * ppp);
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
                    .unwrap_or(Vec2::new(width, self.ui.style.header_height * ppp));
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

    /// A tappable full-width row, dat.gui's `gui.add(obj, 'method')` trigger.
    /// Reports `Response::clicked` for exactly the frame the press lands.
    pub fn button(&mut self, label: &str) -> Response {
        let ppp = self.pixels_per_point();
        let row = self.layout.row(self.ui.style.row_height * ppp);
        let mut clicked = false;
        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                clicked = row.contains_point(Vec2::new(x, y));
            }
        }
        self.push_quad(row, self.ui.style.button_bg);
        // Centre the label on the row.
        let px = TextStyle::Body.px() * ppp;
        let w = self.ui.atlas.measure(label) * (px / self.ui.atlas.line_height().max(1.0));
        self.push_text(
            Vec2::new(row.min.x + (row.size().x - w).max(0.0) * 0.5, row.min.y),
            label,
            TextStyle::Body,
            self.ui.style.text,
        );
        Response {
            changed: false,
            clicked,
            selected: false,
        }
    }

    /// A taller, non-interactive section header drawn on the panel background
    /// colour — a heading between groups of widgets.
    pub fn heading(&mut self, label: &str) {
        let ppp = self.pixels_per_point();
        let row = self.layout.row((TextStyle::Heading.px() + 10.0) * ppp);
        self.push_quad(row, self.ui.style.panel_bg);
        self.push_text(
            Vec2::new(row.min.x, row.min.y + 4.0 * ppp),
            label,
            TextStyle::Heading,
            self.ui.style.text,
        );
    }

    /// A 1px horizontal rule dividing widget groups.
    pub fn separator(&mut self) {
        let row = self.layout.row(1.0);
        self.push_quad(row, self.ui.style.dim);
    }

    /// Parks the *next* widget on the current line, right of the previous one
    /// (`neptune-gui-plus-dat-plan.md` Task 19).
    pub fn same_line(&mut self) {
        self.layout.same_line();
    }

    /// Runs `contents` with every widget laid out left-to-right on one shared
    /// line instead of top-to-bottom (`neptune-gui-plus-dat-plan.md` Task 19).
    pub fn horizontal(&mut self, contents: impl FnOnce(&mut UiFrame)) {
        self.layout.begin_horizontal();
        contents(self);
        self.layout.end_horizontal();
    }

    /// A single-line text field. Click to focus; typing inserts at the caret,
    /// Backspace/Delete delete, Left/Right move the caret, Home/End jump,
    /// Enter or Escape drop focus. The only widget that reads the keyboard
    /// (`neptune-gui-plus-dat-plan.md` Task 20, the `G4` gap in `cubed00.md`).
    pub fn text_input(&mut self, label: &str, value: &mut String) -> Response {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(self.ui.style.row_height * ppp);
        self.push_text(row.min, label, TextStyle::Body, self.ui.style.text);
        let box_rect = Aabb2d::new(
            Vec2::new(row.min.x + self.ui.style.label_width * ppp, row.min.y),
            Vec2::new(row.max.x, row.max.y),
        );
        let changed = self.input_box(id, box_rect, value);
        Response {
            changed,
            clicked: false,
            selected: false,
        }
    }

    /// The shared edit box behind [`UiFrame::text_input`] and a `drag_value`
    /// being typed into: owns focus, caret motion, deletion and insertion, and
    /// draws the box, its (clipped) text and the caret. Returns whether the
    /// text changed this frame.
    fn input_box(&mut self, id: WidgetId, rect: Aabb2d, value: &mut String) -> bool {
        let ppp = self.pixels_per_point();
        let mut focused = self.ui.focus == Some(id);
        let mut changed = false;

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                let point = Vec2::new(x, y);
                if rect.contains_point(point) {
                    self.ui.focus = Some(id);
                    let caret = self.caret_at_x(value, rect, x, ppp).min(value.len());
                    self.ui.caret.insert(id, caret);
                } else if focused {
                    self.ui.focus = None;
                }
            }
        }

        // Re-read after the click block: a click that just landed grants focus
        // this very frame, and the same frame's keystrokes must be consumed.
        focused = self.ui.focus == Some(id);

        if focused {
            let mut caret = self.ui.caret.get(&id).copied().unwrap_or(0).min(value.len());

            // Caret motion first, so Home/End/arrows act before any deletion.
            if self.input.just_pressed(KeyCode::Home) {
                caret = 0;
            }
            if self.input.just_pressed(KeyCode::End) {
                caret = value.len();
            }
            if self.input.just_pressed(KeyCode::ArrowLeft) {
                caret = value[..caret]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            if self.input.just_pressed(KeyCode::ArrowRight) {
                if caret < value.len() {
                    caret += value[caret..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                }
            }
            if self.input.just_pressed(KeyCode::Backspace) && caret > 0 {
                let start = value[..caret]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.replace_range(start..caret, "");
                caret = start;
                changed = true;
            }
            if self.input.just_pressed(KeyCode::Delete) && caret < value.len() {
                let end = value[caret..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| caret + i)
                    .unwrap_or(value.len());
                value.replace_range(caret..end, "");
                changed = true;
            }
            for ch in self.input.text() {
                value.insert_str(caret, ch);
                caret += ch.len();
                changed = true;
            }
            if self.input.just_pressed(KeyCode::Enter)
                || self.input.just_pressed(KeyCode::NumpadEnter)
                || self.input.just_pressed(KeyCode::Escape)
            {
                self.ui.focus = None;
            }

            self.ui.caret.insert(id, caret);
        }

        // Draw the box, the text (clipped to its tail when it overflows), and
        // the caret while focused.
        self.push_quad(rect, self.ui.style.widget_bg);
        let px = TextStyle::Body.px() * ppp;
        let scale = px / self.ui.atlas.line_height().max(1.0);
        let pad = 5.0 * ppp;
        let box_w = (rect.size().x - pad * 2.0).max(1.0);
        let start = self.visible_start(value, scale, box_w);
        self.push_text(
            Vec2::new(rect.min.x + pad, rect.min.y),
            &value[start..],
            TextStyle::Body,
            self.ui.style.text,
        );
        if focused {
            let caret = self.ui.caret.get(&id).copied().unwrap_or(0);
            if caret >= start {
                let caret_x = rect.min.x + pad + self.ui.atlas.measure(&value[start..caret]) * scale;
                let caret_rect = Aabb2d::new(
                    Vec2::new(caret_x, rect.min.y + 4.0 * ppp),
                    Vec2::new(caret_x + 1.0, rect.max.y - 4.0 * ppp),
                );
                self.push_quad(caret_rect, self.ui.style.accent);
            }
        }

        changed
    }

    /// The leftmost byte index of the longest *suffix* of `value` whose
    /// rendered width fits in `box_w` — the text shown in an overflowing input
    /// box, so the caret's end of a long string stays in view.
    fn visible_start(&self, value: &str, scale: f32, box_w: f32) -> usize {
        let mut start = value.len();
        while start > 0 {
            let prev = value[..start]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            if self.ui.atlas.measure(&value[prev..]) * scale <= box_w {
                start = prev;
            } else {
                break;
            }
        }
        start
    }

    /// The byte index of the character whose left edge is nearest `x` inside
    /// `rect` — where a click places the caret.
    fn caret_at_x(&self, value: &str, rect: Aabb2d, x: f32, ppp: f32) -> usize {
        let px = TextStyle::Body.px() * ppp;
        let scale = px / self.ui.atlas.line_height().max(1.0);
        let pad = 5.0 * ppp;
        let mut best = 0usize;
        let mut best_dist = f32::MAX;
        for (byte, _) in value.char_indices() {
            let w = self.ui.atlas.measure(&value[..byte]) * scale + pad;
            let dist = (rect.min.x + w - x).abs();
            if dist < best_dist {
                best_dist = dist;
                best = byte;
            }
        }
        let w = self.ui.atlas.measure(value) * scale + pad;
        if (rect.min.x + w - x).abs() < best_dist {
            best = value.len();
        }
        best
    }

    /// A draggable numeric readout, dat.gui's drag-to-edit number. Drag
    /// horizontally to change (the `range`'s span maps over the box's width);
    /// a plain click switches the box into type-to-edit via the shared
    /// [`UiFrame::input_box`] (`neptune-gui-plus-dat-plan.md` Task 21).
    pub fn drag_value(&mut self, label: &str, value: &mut f32, range: RangeInclusive<f32>) -> Response {
        let ppp = self.pixels_per_point();
        let id = WidgetId::new(label, 0);
        let row = self.layout.row(self.ui.style.row_height * ppp);
        self.push_text(row.min, label, TextStyle::Body, self.ui.style.text);
        let box_rect = Aabb2d::new(
            Vec2::new(row.min.x + self.ui.style.label_width * ppp, row.min.y),
            Vec2::new(row.max.x, row.max.y),
        );

        let mut changed = false;

        if let Some(mut buf) = self.ui.edit_buf.remove(&id) {
            // Type-to-edit: the box is a live `text_input` over the number.
            self.input_box(id, box_rect, &mut buf);
            if self.ui.focus == Some(id) {
                self.ui.edit_buf.insert(id, buf);
            } else {
                // Focus left this frame (Enter/Escape/click-away): commit.
                if let Ok(parsed) = buf.trim().parse::<f32>() {
                    let clamped = parsed.clamp(*range.start(), *range.end());
                    if clamped != *value {
                        *value = clamped;
                        changed = true;
                    }
                }
            }
        } else {
            changed = self.drag_number(id, box_rect, value, range.clone());
            // A press-and-release with no movement switches to type-to-edit.
            if self.input.mouse().just_released(MouseButton::Left) {
                if let Some((pid, press)) = self.ui.press_pos {
                    if pid == id {
                        self.ui.press_pos = None;
                        let was_click = self
                            .input
                            .mouse()
                            .position()
                            .map(|(x, y)| (Vec2::new(x, y) - press).length_squared() < 4.0)
                            .unwrap_or(false);
                        if was_click {
                            self.ui.edit_buf.insert(id, format!("{value:.2}"));
                            self.ui.focus = Some(id);
                        }
                    }
                }
            }
            self.push_quad(box_rect, self.ui.style.widget_bg);
            self.push_text(
                Vec2::new(box_rect.min.x + 5.0 * ppp, row.min.y),
                &format!("{value:.2}"),
                TextStyle::Body,
                self.ui.style.text,
            );
        }

        Response {
            changed,
            clicked: false,
            selected: false,
        }
    }

    /// The drag half of a numeric field: press inside `rect` to grab, move to
    /// adjust `value` by pixels (the `range`'s span maps over `rect`'s width),
    /// release to let go. Records the press position for click detection.
    fn drag_number(
        &mut self,
        id: WidgetId,
        rect: Aabb2d,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> bool {
        let mut changed = false;
        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                if rect.contains_point(Vec2::new(x, y)) {
                    self.ui.active_drag = Some(id);
                    self.ui.press_pos = Some((id, Vec2::new(x, y)));
                }
            }
        }
        if self.input.mouse().held(MouseButton::Left) && self.ui.active_drag == Some(id) {
            let (dx, _) = self.input.mouse().delta();
            if dx != 0.0 {
                let span = *range.end() - *range.start();
                *value = (*value + span * dx / rect.size().x.max(1.0))
                    .clamp(*range.start(), *range.end());
                changed = true;
            }
        }
        if self.input.mouse().just_released(MouseButton::Left) && self.ui.active_drag == Some(id) {
            self.ui.active_drag = None;
        }
        changed
    }

    /// Draws a texture as a read-only, centred quad, dat.gui's preview image —
    /// and the widget `ui_demo` uses to show the render target
    /// (`neptune-gui-plus-dat-plan.md` Task 22).
    pub fn image(&mut self, texture: &Texture, size: Vec2) {
        let row = self.layout.row(size.y);
        let x = row.min.x + (row.size().x - size.x).max(0.0) * 0.5;
        self.draw_list.push(UiPrimitive {
            rect: Aabb2d::new(
                Vec2::new(x, row.min.y),
                Vec2::new(x + size.x, row.max.y),
            ),
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            color: Color::WHITE,
            texture: texture.clone(),
        });
    }

    /// A read-only 0..1 progress bar, full width (`neptune-gui-plus-dat-plan.md`
    /// Task 24). The fraction is clamped, so overflow paints a full bar.
    pub fn progress_bar(&mut self, fraction: f32) {
        let ppp = self.pixels_per_point();
        let row = self.layout.row(self.ui.style.row_height * ppp);
        let track = Aabb2d::new(
            Vec2::new(row.min.x, row.min.y + 6.0 * ppp),
            Vec2::new(row.max.x, row.max.y - 6.0 * ppp),
        );
        self.push_quad(track, self.ui.style.widget_bg);
        let f = fraction.clamp(0.0, 1.0);
        let fill = Aabb2d::new(
            track.min,
            Vec2::new(track.min.x + f * track.size().x, track.max.y),
        );
        self.push_quad(fill, self.ui.style.accent);
    }

    /// Schedules a small hover label for the most recently drawn widget; it is
    /// painted on top in [`UiFrame::finish`] (`neptune-gui-plus-dat-plan.md`
    /// Task 25). Call it immediately after the widget it describes.
    pub fn tooltip(&mut self, text: &str) {
        if let Some((x, y)) = self.input.mouse().position() {
            if let Some(row) = self.last_row() {
                if row.contains_point(Vec2::new(x, y)) {
                    self.tooltip = Some((Vec2::new(x, y), text.to_string()));
                }
            }
        }
    }

    /// A single-choice row: reports `Response::selected` for the frame it is
    /// clicked, and is drawn highlighted when `is_selected`
    /// (`neptune-gui-plus-dat-plan.md` Task 27).
    pub fn selectable_label(&mut self, label: &str, is_selected: bool) -> Response {
        let ppp = self.pixels_per_point();
        let row = self.layout.row(self.ui.style.row_height * ppp);
        let mut selected = false;
        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                selected = row.contains_point(Vec2::new(x, y));
            }
        }
        let bg = if is_selected {
            self.ui.style.accent.with_alpha(0.35)
        } else {
            Color::TRANSPARENT
        };
        self.push_quad(row, bg);
        self.push_text(
            Vec2::new(row.min.x + 4.0 * ppp, row.min.y),
            label,
            TextStyle::Body,
            self.ui.style.text,
        );
        Response {
            changed: false,
            clicked: false,
            selected,
        }
    }

    /// A menu bar: one column per `entries` entry, laid out left-to-right;
    /// clicking a column opens its submenu (`entries[i].1`), which overlays
    /// whatever drew after it because its rows are pushed to the deferred layer
    /// (`neptune-gui-plus-dat-plan.md` Task 29). A chosen item lands in
    /// `choice` as `(menu_index, item_index)`.
    pub fn menu_bar(
        &mut self,
        entries: &[(&str, &[&str])],
        choice: &mut Option<(usize, usize)>,
    ) -> Response {
        let ppp = self.pixels_per_point();
        let row_height = self.ui.style.row_height * ppp;
        let submenu_w = 160.0 * ppp;
        let px = TextStyle::Body.px() * ppp;
        let scale = px / self.ui.atlas.line_height().max(1.0);

        // Layout pass: one compact column per entry on a shared line, each
        // wide enough for its label (not full-panel-width, which horizontal
        // mode would give).
        let bar_row = self.layout.row(row_height);
        let baseline_y = bar_row.min.y;
        let mut rows = Vec::with_capacity(entries.len());
        let mut x = bar_row.min.x;
        for (label, _) in entries {
            let w = self.ui.atlas.measure(label) * scale + 24.0 * ppp;
            rows.push(Aabb2d::new(
                Vec2::new(x, baseline_y),
                Vec2::new(x + w, baseline_y + row_height),
            ));
            x += w;
        }

        let was_open: Vec<WidgetId> = self.ui.open.iter().copied().collect();
        let mut changed = false;

        if self.input.mouse().just_pressed(MouseButton::Left) {
            if let Some((x, y)) = self.input.mouse().position() {
                let point = Vec2::new(x, y);
                let mut consumed = false;
                for (mi, (label, items)) in entries.iter().enumerate() {
                    let id = WidgetId::new(label, 0);
                    if rows[mi].contains_point(point) {
                        if self.ui.open.contains(&id) {
                            self.ui.open.remove(&id);
                        } else {
                            self.ui.open.clear();
                            self.ui.open.insert(id);
                        }
                        consumed = true;
                    } else if was_open.contains(&id) {
                        for (si, _) in items.iter().enumerate() {
                            let item_row = Aabb2d::new(
                                Vec2::new(rows[mi].min.x, rows[mi].max.y + si as f32 * row_height),
                                Vec2::new(
                                    rows[mi].min.x + submenu_w,
                                    rows[mi].max.y + (si as f32 + 1.0) * row_height,
                                ),
                            );
                            if item_row.contains_point(point) {
                                *choice = Some((mi, si));
                                changed = true;
                                self.ui.open.clear();
                                consumed = true;
                            }
                        }
                    }
                }
                if !consumed {
                    self.ui.open.clear();
                }
            }
        }

        // Draw pass: the bar, then any open submenu into the deferred layer.
        for (mi, (label, items)) in entries.iter().enumerate() {
            let id = WidgetId::new(label, 0);
            let row = rows[mi];
            self.push_quad(row, self.ui.style.widget_bg);
            self.push_text(
                Vec2::new(row.min.x + 6.0 * ppp, row.min.y),
                label,
                TextStyle::Body,
                self.ui.style.text,
            );
            if self.ui.open.contains(&id) {
                for (si, item) in items.iter().enumerate() {
                    let item_row = Aabb2d::new(
                        Vec2::new(row.min.x, row.max.y + si as f32 * row_height),
                        Vec2::new(row.min.x + submenu_w, row.max.y + (si as f32 + 1.0) * row_height),
                    );
                    self.deferred.push(UiPrimitive {
                        rect: item_row,
                        uv_min: [0.0, 0.0],
                        uv_max: [1.0, 1.0],
                        color: self.ui.style.option_bg,
                        texture: self.ui.white.clone(),
                    });
                    let glyphs = layout_text(
                        &self.ui.atlas,
                        item,
                        Vec2::new(item_row.min.x + 6.0 * ppp, item_row.min.y),
                        TextStyle::Body.px() * ppp,
                        self.ui.style.text,
                        self.ui.atlas.texture().clone(),
                    );
                    self.deferred.extend(glyphs);
                }
            }
        }

        Response {
            changed,
            clicked: false,
            selected: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputState, MouseButton};
    use crate::math::Vec2;
    use crate::text::Font;
    use crate::ui::Ui;
    use winit::dpi::PhysicalPosition;
    use winit::event::ElementState;

    // These match the default [`Style`]; the tests never restyle, so the old
    // named constants keep their meaning here after the Task 23 hoist.
    const ROW_HEIGHT: f32 = 22.0;
    const LABEL_WIDTH: f32 = 90.0;
    const DOCK_GUTTER: f32 = 8.0;

    fn ui() -> Option<Ui> {
        let atlas = Font::system_default().ok()?.atlas(24.0).ok()?;
        Some(Ui::new(atlas))
    }

    fn press_at(input: &mut InputState, x: f64, y: f64) {
        input
            .mouse_mut()
            .handle_cursor_moved(PhysicalPosition::new(x, y));
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Pressed);
    }

    #[test]
    fn dragging_inside_the_track_updates_the_value_and_reports_changed() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;

        // Frame 1: press at the track's left edge (label reserves 90px).
        let mut input = InputState::new();
        press_at(&mut input, 95.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.slider("Speed", &mut value, 0.0..=10.0);
            assert!(response.changed(), "the press itself starts and applies a drag");
        }
        input.end_frame();

        // Frame 2: drag to the middle of the track.
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(180.0, 11.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.slider("Speed", &mut value, 0.0..=10.0);
            assert!(response.changed());
        }
        assert!(value > 0.0 && value < 10.0, "got {value}");
    }

    #[test]
    fn clicking_outside_the_track_does_not_start_a_drag() {
        let Some(mut ui) = ui() else { return };
        let mut value = 3.0f32;
        let mut input = InputState::new();
        press_at(&mut input, 5.0, 500.0); // far outside any row

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.slider("Speed", &mut value, 0.0..=10.0);
        assert!(!response.changed());
        assert_eq!(value, 3.0);
    }

    #[test]
    fn the_value_never_leaves_its_range() {
        let Some(mut ui) = ui() else { return };
        let mut value = 5.0f32;
        let mut input = InputState::new();
        // Press far to the right of the track — clamps to max, not off-scale.
        press_at(&mut input, 10_000.0, 11.0);

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.slider("Speed", &mut value, 0.0..=10.0);
        assert!((0.0..=10.0).contains(&value), "got {value}");
    }

    #[test]
    fn releasing_the_button_stops_the_drag() {
        let Some(mut ui) = ui() else { return };
        let mut value = 0.0f32;
        let mut input = InputState::new();
        press_at(&mut input, 95.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.slider("Speed", &mut value, 0.0..=10.0);
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);

        let stalled = value;
        // No button held: moving the mouse must not move the slider.
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(250.0, 11.0));
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.slider("Speed", &mut value, 0.0..=10.0);
        assert_eq!(value, stalled);
    }

    #[test]
    fn clicking_inside_the_box_toggles_the_value() {
        let Some(mut ui) = ui() else { return };
        let mut value = false;
        let mut input = InputState::new();
        press_at(&mut input, 8.0, 11.0); // inside the 16px box at the row's left edge

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.checkbox("Wireframe", &mut value);
        assert!(response.changed());
        assert!(value);
    }

    #[test]
    fn clicking_outside_the_box_does_nothing() {
        let Some(mut ui) = ui() else { return };
        let mut value = false;
        let mut input = InputState::new();
        press_at(&mut input, 500.0, 500.0);

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.checkbox("Wireframe", &mut value);
        assert!(!response.changed());
        assert!(!value);
    }

    #[test]
    fn a_second_click_toggles_it_back() {
        let Some(mut ui) = ui() else { return };
        let mut value = false;
        let mut input = InputState::new();
        press_at(&mut input, 8.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.checkbox("Wireframe", &mut value);
        }
        assert!(value);

        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);
        press_at(&mut input, 8.0, 11.0);
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.checkbox("Wireframe", &mut value);
        assert!(!value);
    }

    #[test]
    fn clicking_the_header_opens_it_without_changing_the_selection() {
        let Some(mut ui) = ui() else { return };
        let mut selected = 0usize;
        let mut input = InputState::new();
        press_at(&mut input, 150.0, 11.0); // inside the header, right of the label column

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let mut input = InputState::new();
        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);

        // Frame 2: click the second option row, just below the header.
        press_at(&mut input, 150.0, (2.0 * ROW_HEIGHT + 11.0) as f64);
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        assert!(response.changed());
        assert_eq!(selected, 1);
    }

    #[test]
    fn a_closed_dropdown_ignores_clicks_below_the_header() {
        let Some(mut ui) = ui() else { return };
        let mut selected = 0usize;
        let mut input = InputState::new();
        press_at(&mut input, 150.0, (ROW_HEIGHT + 11.0) as f64); // where an option row would be, but nothing is open

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.dropdown("Shading", &["Flat", "Smooth"], &mut selected);
        assert!(!response.changed());
        assert_eq!(selected, 0);
    }

    #[test]
    fn clicking_the_swatch_opens_the_preset_grid() {
        let Some(mut ui) = ui() else { return };
        let mut value = Color::WHITE;
        let mut input = InputState::new();
        press_at(&mut input, (LABEL_WIDTH + 10.0) as f64, 11.0); // inside the swatch

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.color_edit("Tint", &mut value);
        assert!(!response.changed());
        assert_eq!(value, Color::WHITE);
    }

    #[test]
    fn clicking_an_open_preset_applies_it() {
        let Some(mut ui) = ui() else { return };
        let mut value = Color::WHITE;

        let mut input = InputState::new();
        press_at(&mut input, (LABEL_WIDTH + 10.0) as f64, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.color_edit("Tint", &mut value);
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);

        // First preset cell sits just right of the swatch, same row.
        let swatch_right = LABEL_WIDTH + 24.0;
        press_at(&mut input, (swatch_right + 4.0 + 8.0) as f64, 11.0);
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        let response = frame.color_edit("Tint", &mut value);
        assert!(response.changed());
        assert_eq!(value, PRESETS[0]);
    }

    #[test]
    fn an_expanded_folder_draws_its_contents() {
        let Some(mut ui) = ui() else { return };
        let mut fov = 75.0f32;
        let input = InputState::new(); // no click: folders start expanded
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let mut input = InputState::new();
        press_at(&mut input, 10.0, 11.0); // inside the header row

        let mut frame = primary.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.folder("Advanced", |ui| {
            ui.slider("FOV", &mut fov, 30.0..=120.0);
        });
        // Only the header's own quad + label glyphs remain — no slider primitives.
        let header_only_len = frame.finish().len();

        // A second, independently-built Ui (nothing collapsed) confirms the
        // header alone is shorter than a folder left expanded — no test-only
        // production API needed, just a fresh Ui.
        let Some(mut expanded) = ui() else { return };
        let input2 = InputState::new();
        let mut frame2 = expanded.begin(&input2, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let input = InputState::new();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let mut input = InputState::new();

        // Frame 1: place the window at the origin (no interaction).
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        input.end_frame();

        // Frame 2: grab the header at (150, 11). The origin is (0,0), so the
        // grab offset is the click point itself.
        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        input.end_frame();

        // Frame 3: still held, cursor moved by (100, 20): the origin follows.
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(250.0, 31.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
            assert_eq!(
                frame.ui.panel_origin[&window_id("Settings")],
                Vec2::new(100.0, 20.0)
            );
        }
        input.end_frame();

        // Frame 4: released — moving the cursor without a held button must not
        // move the window.
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(400.0, 200.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let mut input = InputState::new();

        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        input.end_frame();

        // Drag to the left edge while still held.
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(4.0, 200.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let mut input = InputState::new();

        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Settings", 200.0, |ui| {
                ui.slider("Speed", &mut value, 0.0..=10.0);
            });
        }
        input.end_frame();

        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(796.0, 200.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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
        let mut input = InputState::new();

        // Dock "Main" to the left edge.
        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
        }
        input.end_frame();
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(4.0, 200.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
            assert_eq!(frame.ui.docked[&window_id("Main")], DockEdge::Left);
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);
        input.end_frame();

        // Park "Side" out of Main's way first: press at (150,11) moves its
        // origin to (400,300), which no longer overlaps Main's header.
        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
        }
        input.end_frame();
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(550.0, 311.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
            assert_eq!(
                frame.ui.panel_origin[&window_id("Side")],
                Vec2::new(400.0, 300.0)
            );
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);
        input.end_frame();

        // Re-grab "Side" where its header now is — clear of Main's — and drag
        // it to the left edge: it stacks below Main on the same edge.
        press_at(&mut input, 560.0, 311.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.window("Main", 200.0, |ui| {
                ui.slider("Speed", &mut speed, 0.0..=10.0);
            });
            frame.window("Side", 180.0, |ui| {
                ui.slider("FOV", &mut fov, 30.0..=120.0);
            });
        }
        input.end_frame();
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(4.0, 500.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
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

    fn press_key(input: &mut InputState, code: crate::input::KeyCode) {
        input.set_key(code, ElementState::Pressed, false);
    }

    #[test]
    fn clicking_the_button_reports_clicked_for_exactly_one_frame() {
        let Some(mut ui) = ui() else { return };
        let mut input = InputState::new();
        press_at(&mut input, 10.0, 11.0); // inside the button row
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            assert!(frame.button("Reset").clicked());
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);
        input.end_frame();
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            assert!(!frame.button("Reset").clicked());
        }
    }

    #[test]
    fn clicking_outside_the_button_does_not_click_it() {
        let Some(mut ui) = ui() else { return };
        let mut input = InputState::new();
        press_at(&mut input, 500.0, 500.0);
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        assert!(!frame.button("Reset").clicked());
    }

    #[test]
    fn heading_and_separator_draw_something() {
        let Some(mut ui) = ui() else { return };
        let input = InputState::new();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.heading("Advanced");
        frame.separator();
        assert!(frame.finish().len() > 0);
    }

    #[test]
    fn same_line_puts_two_widgets_on_one_row() {
        let Some(mut ui) = ui() else { return };
        let mut a = false;
        let mut b = false;
        let input = InputState::new();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.checkbox("A", &mut a);
        frame.same_line();
        frame.checkbox("B", &mut b);
        let list = frame.finish();
        // The two checkbox box-quads share a baseline, side by side.
        let boxes: Vec<_> = list
            .primitives
            .iter()
            .filter(|p| p.color == ui.style().widget_bg)
            .collect();
        assert_eq!(boxes.len(), 2, "one box quad per checkbox");
        assert_eq!(boxes[0].rect.min.y, boxes[1].rect.min.y);
        assert!(boxes[1].rect.min.x > boxes[0].rect.max.x);
    }

    #[test]
    fn typing_into_a_focused_text_input_inserts_and_backspace_deletes() {
        let Some(mut ui) = ui() else { return };
        let mut text = String::from("ab");
        let mut input = InputState::new();

        // Frame 1: click into the box (label column is 90px) and type "x".
        press_at(&mut input, 150.0, 11.0);
        input.push_typed("x");
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.text_input("Name", &mut text);
            assert!(response.changed());
            assert_eq!(text, "abx");
            assert!(frame.ui.focus.is_some());
        }
        input.end_frame();

        // Frame 2: Backspace removes the character before the caret.
        press_key(&mut input, KeyCode::Backspace);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.text_input("Name", &mut text);
            assert_eq!(text, "ab");
        }
        input.end_frame();

        // Frame 3: Enter drops focus.
        press_key(&mut input, KeyCode::Enter);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.text_input("Name", &mut text);
            assert!(frame.ui.focus.is_none());
        }
    }

    #[test]
    fn an_unfocused_text_input_ignores_keystrokes() {
        let Some(mut ui) = ui() else { return };
        let mut text = String::from("ab");
        let mut input = InputState::new();
        input.push_typed("x"); // never clicked into the box
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.text_input("Name", &mut text);
        assert_eq!(text, "ab");
    }

    #[test]
    fn dragging_a_drag_value_adjusts_it_by_pixels() {
        let Some(mut ui) = ui() else { return };
        let mut value = 5.0f32;
        let mut input = InputState::new();
        // Press inside the box (which starts at the 90px label column).
        press_at(&mut input, 150.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.drag_value("Health", &mut value, 0.0..=10.0);
        }
        input.end_frame();
        // Still held, drag right by 40px.
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(190.0, 11.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.drag_value("Health", &mut value, 0.0..=10.0);
            assert!(response.changed());
        }
        assert!(value > 5.0, "got {value}");
    }

    #[test]
    fn image_pushes_one_primitive_with_the_textures_id() {
        let Some(mut ui) = ui() else { return };
        let input = InputState::new();
        let texture = crate::materials::Texture::white();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.image(&texture, Vec2::new(60.0, 60.0));
        let list = frame.finish();
        assert_eq!(list.len(), 1);
        assert_eq!(list.primitives[0].texture.id(), texture.id());
    }

    #[test]
    fn progress_bar_draws_a_track_and_a_fill() {
        let Some(mut ui) = ui() else { return };
        let input = InputState::new();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.progress_bar(0.5);
        let list = frame.finish();
        // track + fill quads, nothing else.
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn a_progress_fraction_over_one_is_clamped_to_full() {
        let Some(mut ui) = ui() else { return };
        let input = InputState::new();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        frame.progress_bar(1.7);
        let list = frame.finish();
        let fill = &list.primitives[1].rect;
        let track = &list.primitives[0].rect;
        assert_eq!(fill.max.x, track.max.x, "fill reaches the track's right edge");
    }

    #[test]
    fn clicking_a_selectable_label_reports_selected() {
        let Some(mut ui) = ui() else { return };
        let mut input = InputState::new();
        press_at(&mut input, 10.0, 11.0);
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
        assert!(frame.selectable_label("Option 1", false).selected());
    }

    #[test]
    fn dragging_the_red_channel_of_color_edit_changes_it() {
        let Some(mut ui) = ui() else { return };
        let mut value = Color::WHITE;
        let mut input = InputState::new();

        // Row 2 (the RGB row) starts below row 1 (22px + 4px padding). The R
        // field is the left third of the box area, which begins at x = 90.
        press_at(&mut input, 110.0, 33.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.color_edit("Tint", &mut value);
        }
        input.end_frame();

        // Still held, drag right: the red channel climbs and clamps to 1.0.
        input.mouse_mut().handle_cursor_moved(PhysicalPosition::new(210.0, 33.0));
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.color_edit("Tint", &mut value);
            assert!(response.changed());
        }
        assert!(value.r >= 1.0 - 1e-3, "clamped to 1.0, got {}", value.r);
        assert_eq!(value.g, 1.0, "green untouched");
        assert_eq!(value.b, 1.0, "blue untouched");
    }

    #[test]
    fn clicking_a_menu_column_opens_its_submenu_and_choosing_selects() {
        let Some(mut ui) = ui() else { return };
        let entries: &[(&str, &[&str])] = &[("File", &["New", "Open"]), ("Edit", &["Undo"])];
        let mut choice = None;

        // Frame 1: click the first column to open it.
        let mut input = InputState::new();
        press_at(&mut input, 6.0, 11.0);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            frame.menu_bar(entries, &mut choice);
            assert!(frame.ui.open.contains(&WidgetId::new("File", 0)));
        }
        input.end_frame();
        input
            .mouse_mut()
            .handle_button_event(MouseButton::Left, ElementState::Released);
        input.end_frame();

        // Frame 2: click the first submenu item, just below the File column.
        press_at(&mut input, 6.0, 11.0 + ROW_HEIGHT as f64);
        {
            let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 260.0);
            let response = frame.menu_bar(entries, &mut choice);
            assert!(response.changed());
            assert_eq!(choice, Some((0, 0)));
        }
    }
}
