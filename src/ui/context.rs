//! The persistent widget state (`Ui`) and the per-frame builder (`UiFrame`)
//! every widget call in `widgets.rs` extends.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::input::InputState;
use crate::materials::Texture;
use crate::math::{Aabb2d, Color, Vec2};
use crate::text::GlyphAtlas;

use super::draw_list::{UiDrawList, UiPrimitive};
use super::layout::{Layout, WidgetId};
use super::text::layout_text;

/// Which edge of the screen a panel is snapped to. A panel docked to an edge
/// is laid out flush against it, sharing the edge with any other panel docked
/// the same way — see [`UiFrame::window`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockEdge {
    Left,
    Right,
    Top,
    Bottom,
}

/// A named font-size tier, matching egui's `TextStyle` in spirit: each
/// variant's [`TextStyle::px`] is a *logical* pixel size at
/// `pixels_per_point == 1.0`. [`UiFrame::push_text`] multiplies it by
/// [`Ui::pixels_per_point`] to get the size actually rasterized, so the same
/// call reads consistently across displays once DPI is wired up (see
/// [`Ui::set_pixels_per_point`]).
///
/// Sizes are chosen to keep `Body` at the size every widget already used
/// before this tiering existed (no visual change at the default zoom), with
/// a smaller and a larger tier added around it, proportioned like egui's own
/// defaults (Small 9 / Body 13 / Heading 18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    Small,
    Body,
    Button,
    Heading,
}

impl TextStyle {
    /// Base size in logical pixels, before DPI scaling.
    pub const fn px(self) -> f32 {
        match self {
            TextStyle::Small => 10.0,
            TextStyle::Body => 14.0,
            TextStyle::Button => 14.0,
            TextStyle::Heading => 20.0,
        }
    }
}

/// The look every widget draws with: row heights, column widths, and the
/// colour palette. One `Style` lives on [`Ui`]; widgets read it instead of
/// hard-coded constants, so a demo can restyle the whole panel by swapping it
/// (see `neptune-gui-plus-dat-plan.md` Task 23). Metric fields are logical
/// pixels at `pixels_per_point == 1.0`, exactly like [`TextStyle::px`].
#[derive(Debug, Clone)]
pub struct Style {
    /// Standard row height for every widget row.
    pub row_height: f32,
    /// Left column reserved for a widget's label.
    pub label_width: f32,
    /// Right column reserved for a slider's live value readout.
    pub value_width: f32,
    /// Pixel height of a window's draggable title bar.
    pub header_height: f32,
    /// Dragging a window's header to within this distance of a screen edge
    /// snaps it to that edge.
    pub dock_threshold: f32,
    /// Gap between windows docked to the same screen edge.
    pub dock_gutter: f32,
    /// Window background.
    pub panel_bg: Color,
    /// Window title bar.
    pub panel_header: Color,
    /// Slider track, dropdown header, checkbox off-box, input box.
    pub widget_bg: Color,
    /// Open dropdown/submenu option rows.
    pub option_bg: Color,
    /// Collapsible folder header.
    pub folder_bg: Color,
    /// Button face.
    pub button_bg: Color,
    /// Slider thumb, progress fill, caret, focus border.
    pub accent: Color,
    /// Checkbox on-fill.
    pub checkbox_on: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary/dimmed text (placeholders, readouts).
    pub dim: Color,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            row_height: 22.0,
            label_width: 90.0,
            value_width: 56.0,
            header_height: 24.0,
            dock_threshold: 24.0,
            dock_gutter: 8.0,
            panel_bg: Color::rgba(0.15, 0.16, 0.19, 0.96),
            panel_header: Color::rgba(0.22, 0.23, 0.28, 0.98),
            widget_bg: Color::rgba(0.2, 0.2, 0.24, 1.0),
            option_bg: Color::rgba(0.15, 0.15, 0.18, 1.0),
            folder_bg: Color::rgba(0.12, 0.12, 0.15, 1.0),
            button_bg: Color::rgba(0.25, 0.26, 0.31, 1.0),
            accent: Color::rgba(0.6, 0.7, 1.0, 1.0),
            checkbox_on: Color::rgba(0.4, 0.8, 0.5, 1.0),
            text: Color::WHITE,
            dim: Color::rgba(0.7, 0.7, 0.75, 1.0),
        }
    }
}

/// State that cannot be reconstructed from a single frame's widget calls —
/// which widget is being dragged, which dropdown is open, which folder is
/// collapsed — kept across frames so an immediate-mode call site still gets
/// drag/click/toggle behavior. See `neptune-imgui-plus-datgui.md` §4.
pub struct Ui {
    pub(crate) atlas: Arc<GlyphAtlas>,
    /// The look every widget draws with — see [`Style`].
    pub(crate) style: Style,
    /// Which widget currently owns the text cursor. At most one `text_input`
    /// (or a `drag_value` being edited) has focus; it is the only widget that
    /// consumes the frame's typed characters.
    pub(crate) focus: Option<WidgetId>,
    /// Each focused widget's caret position, a byte index into its `String`.
    /// Kept in `Ui` because the string itself lives with the caller.
    pub(crate) caret: HashMap<WidgetId, usize>,
    /// Where a `drag_value` press started, so a press-and-release with no
    /// movement can be told apart from a real drag (the former switches to
    /// type-to-edit).
    pub(crate) press_pos: Option<(WidgetId, Vec2)>,
    /// The live text buffer of a `drag_value` currently being typed into —
    /// present only while that widget owns the edit box.
    pub(crate) edit_buf: HashMap<WidgetId, String>,
    /// A single shared 1x1 white texture: solid-color quads (panels, tracks,
    /// checkboxes) all reuse this one `TextureId`, so the renderer's texture
    /// cache (`backend::texture::TextureCache`) uploads and binds it once
    /// instead of once per widget per frame.
    pub(crate) white: Texture,
    pub(crate) active_drag: Option<WidgetId>,
    pub(crate) open: HashSet<WidgetId>,
    pub(crate) collapsed: HashSet<WidgetId>,
    /// Panels in first-seen order — the stable order docked panels share an
    /// edge in (see [`UiFrame::window`]).
    pub(crate) panels: Vec<WidgetId>,
    /// Each panel's top-left corner, the last place it was drawn.
    pub(crate) panel_origin: HashMap<WidgetId, Vec2>,
    /// Each panel's `(width, height)` from the last frame it drew, so docked
    /// neighbours can relayout against it.
    pub(crate) panel_size: HashMap<WidgetId, Vec2>,
    /// Which panels are snapped to which screen edge.
    pub(crate) docked: HashMap<WidgetId, DockEdge>,
    /// Where a panel grab started, relative to the panel's origin — the drag
    /// anchor that keeps a window from jumping when its header is grabbed.
    pub(crate) grab_offset: Option<Vec2>,
    /// The mouse position where a panel grab started. A grab only *becomes* a
    /// drag (and undocks / re-snaps a window) once the cursor moves away from
    /// here, so a plain click on a title bar never moves or un-docks the panel.
    pub(crate) grab_start: Option<Vec2>,
    /// The DPI/zoom multiplier every widget's text size and layout metrics
    /// are scaled by. `1.0` means "one logical pixel is one physical
    /// pixel" — see [`Ui::set_pixels_per_point`].
    pixels_per_point: f32,
}

impl Ui {
    /// `atlas` is drawn from once, shared by every label this `Ui` renders —
    /// build it the same way `TextMesh`/`orbital_stats.md` do, e.g.
    /// `Font::system_default()?.atlas(24.0)?`.
    pub fn new(atlas: Arc<GlyphAtlas>) -> Self {
        Ui {
            atlas,
            style: Style::default(),
            focus: None,
            caret: HashMap::new(),
            press_pos: None,
            edit_buf: HashMap::new(),
            white: Texture::white(),
            active_drag: None,
            open: HashSet::new(),
            collapsed: HashSet::new(),
            panels: Vec::new(),
            panel_origin: HashMap::new(),
            panel_size: HashMap::new(),
            docked: HashMap::new(),
            grab_offset: None,
            grab_start: None,
            pixels_per_point: 1.0,
        }
    }

    /// The look every widget draws with. Restyle the whole panel by replacing
    /// this, e.g. `ui.set_style(Style { row_height: 28.0, ..ui.style() })`.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Replaces the [`Style`] every widget draws with.
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    /// Whether any widget owns the text cursor this frame. A host app may want
    /// to know, e.g. so `Escape` closes a text box instead of quitting.
    pub fn has_focus(&self) -> bool {
        self.focus.is_some()
    }

    /// The current DPI/zoom multiplier.
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    /// Sets the DPI/zoom multiplier every widget's text and layout metrics
    /// scale by. Feed it the OS's real display scale
    /// ([`Frame::scale_factor`](crate::renderer::Frame::scale_factor)), an
    /// app-chosen accessibility zoom, or both multiplied together. Clamped
    /// to a small positive minimum so a stray `0.0` cannot collapse every
    /// widget to nothing.
    pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) {
        self.pixels_per_point = pixels_per_point.max(0.1);
    }

    /// Sets where `label`'s window first appears (its top-left corner), before
    /// the user has dragged it anywhere — a startup layout hook for demo
    /// screens. No-ops once the window has been drawn and remembered its own
    /// position, so it can never fight the user's drags.
    pub fn place_window(&mut self, label: &str, origin: Vec2) {
        let id = WidgetId::new(label, 1);
        self.panel_origin.entry(id).or_insert(origin);
    }

    /// Starts one frame's panel, laid out top-to-bottom from `origin`,
    /// `width` pixels wide. `input` and `screen` should come straight from
    /// `frame.input()` and `frame.size()`. The whole frame's input snapshot —
    /// mouse *and* keyboard — is borrowed here because `text_input` is the one
    /// widget that reads keystrokes, not just clicks.
    pub fn begin<'a>(
        &'a mut self,
        input: &'a InputState,
        screen: (f32, f32),
        origin: Vec2,
        width: f32,
    ) -> UiFrame<'a> {
        UiFrame {
            ui: self,
            input,
            screen,
            layout: Layout::new(origin, width),
            draw_list: UiDrawList::new(),
            deferred: Vec::new(),
            tooltip: None,
        }
    }
}

/// Everything one frame's widget calls need: a mutable borrow of the
/// persistent [`Ui`] state, this frame's input snapshot (mouse + keyboard),
/// and the draw list being built up call by call.
pub struct UiFrame<'a> {
    pub(crate) ui: &'a mut Ui,
    pub(crate) input: &'a InputState,
    /// Window size in pixels, in the same Y-down space widget rects use —
    /// read by `window` to clamp drags and decide dock edges.
    pub(crate) screen: (f32, f32),
    pub(crate) layout: Layout,
    pub(crate) draw_list: UiDrawList,
    /// Primitives painted after the main list — what open menu submenus and
    /// tooltips go into, so they overlay whatever widgets drew after them
    /// (z-order; `neptune-gui-plus-dat-plan.md` Task 29).
    pub(crate) deferred: Vec<UiPrimitive>,
    /// A tooltip scheduled by the last widget: `(cursor, text)`. Drawn in
    /// [`UiFrame::finish`] so it always sits on top.
    pub(crate) tooltip: Option<(Vec2, String)>,
}

impl<'a> UiFrame<'a> {
    /// Ends the frame, handing back the primitives `Frame::render_ui` draws.
    /// Paints any scheduled tooltip on top, then the deferred menu layer.
    pub fn finish(mut self) -> UiDrawList {
        if let Some((cursor, text)) = self.tooltip.take() {
            let ppp = self.pixels_per_point();
            let px = TextStyle::Small.px() * ppp;
            let text_w = self
                .ui
                .atlas
                .measure(&text)
                * (px / self.ui.atlas.line_height().max(1.0));
            let h = (TextStyle::Small.px() + 8.0) * ppp;
            let w = text_w + 12.0 * ppp;
            let tip = Aabb2d::new(
                Vec2::new(cursor.x + 12.0 * ppp, cursor.y + 14.0 * ppp),
                Vec2::new(cursor.x + 12.0 * ppp + w, cursor.y + 14.0 * ppp + h),
            );
            self.push_quad(tip, Color::rgba(0.1, 0.1, 0.12, 0.95));
            self.push_text(
                Vec2::new(tip.min.x + 4.0 * ppp, tip.min.y + 2.0 * ppp),
                &text,
                TextStyle::Small,
                Color::WHITE,
            );
        }
        self.draw_list.primitives.extend(self.deferred.drain(..));
        self.draw_list
    }

    /// The row the most recent widget call drew, or `None` before any widget —
    /// what `tooltip` and `same_line` key off.
    pub(crate) fn last_row(&self) -> Option<Aabb2d> {
        self.layout.last_row()
    }

    /// The DPI/zoom multiplier this frame is drawing at — see
    /// [`Ui::set_pixels_per_point`].
    pub fn pixels_per_point(&self) -> f32 {
        self.ui.pixels_per_point
    }

    /// Pushes a flat-colored quad, using the shared white texture.
    pub(crate) fn push_quad(&mut self, rect: Aabb2d, color: Color) {
        self.draw_list.push(UiPrimitive {
            rect,
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            color,
            texture: self.ui.white.clone(),
        });
    }

    /// Pushes `text`'s glyph quads at `style`'s size (scaled by
    /// [`Ui::pixels_per_point`]), top-left anchored at `origin`.
    pub(crate) fn push_text(&mut self, origin: Vec2, text: &str, style: TextStyle, color: Color) {
        let px = style.px() * self.ui.pixels_per_point;
        let primitives = layout_text(
            &self.ui.atlas,
            text,
            origin,
            px,
            color,
            self.ui.atlas.texture().clone(),
        );
        self.draw_list.extend(primitives);
    }

    /// Draws `text` as its own row, at `style`'s size. The first widget
    /// method — the rest live in `widgets.rs`.
    pub fn label(&mut self, text: &str, style: TextStyle, color: Color) {
        let ppp = self.pixels_per_point();
        let row = self.layout.row((style.px() + 4.0) * ppp);
        self.push_text(Vec2::new(row.min.x, row.min.y), text, style, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputState;
    use crate::math::Vec2;
    use crate::text::Font;

    fn ui() -> Option<Ui> {
        let atlas = Font::system_default().ok()?.atlas(24.0).ok()?;
        Some(Ui::new(atlas))
    }

    fn input() -> InputState {
        InputState::new()
    }

    #[test]
    fn a_frame_with_no_widgets_produces_an_empty_draw_list() {
        let Some(mut ui) = ui() else { return };
        let input = input();
        let frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 200.0);
        assert!(frame.finish().is_empty());
    }

    #[test]
    fn push_quad_appends_one_primitive_with_the_shared_white_texture() {
        let Some(mut ui) = ui() else { return };
        let input = input();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 200.0);
        let white_id = frame.ui.white.id();
        frame.push_quad(
            crate::math::Aabb2d::new(Vec2::ZERO, Vec2::new(10.0, 10.0)),
            crate::math::Color::RED,
        );
        let list = frame.finish();
        assert_eq!(list.len(), 1);
        assert_eq!(list.primitives[0].texture.id(), white_id);
        assert_eq!(list.primitives[0].color, crate::math::Color::RED);
    }

    #[test]
    fn label_pushes_the_texts_glyph_primitives() {
        let Some(mut ui) = ui() else { return };
        let input = input();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.label("AB", TextStyle::Body, crate::math::Color::WHITE);
        assert_eq!(frame.finish().len(), 2);
    }

    #[test]
    fn pixels_per_point_defaults_to_one_and_is_settable() {
        let Some(mut ui) = ui() else { return };
        assert_eq!(ui.pixels_per_point(), 1.0);
        ui.set_pixels_per_point(2.0);
        assert_eq!(ui.pixels_per_point(), 2.0);
    }

    #[test]
    fn a_non_positive_pixels_per_point_is_clamped() {
        let Some(mut ui) = ui() else { return };
        ui.set_pixels_per_point(0.0);
        assert!(ui.pixels_per_point() > 0.0);
    }

    #[test]
    fn place_window_seeds_a_windows_first_position() {
        let Some(mut ui) = ui() else { return };
        ui.place_window("Settings", Vec2::new(40.0, 60.0));
        let input = input();
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.window("Settings", 200.0, |_| {});
        assert_eq!(
            frame.ui.panel_origin[&WidgetId::new("Settings", 1)],
            Vec2::new(40.0, 60.0)
        );
    }

    #[test]
    fn scaling_pixels_per_point_grows_the_labels_glyph_rects() {
        let Some(mut ui) = ui() else { return };
        let input = input();

        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.label("A", TextStyle::Body, crate::math::Color::WHITE);
        let baseline_width = frame.finish().primitives[0].rect.size().x;

        ui.set_pixels_per_point(2.0);
        let mut frame = ui.begin(&input, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.label("A", TextStyle::Body, crate::math::Color::WHITE);
        let scaled_width = frame.finish().primitives[0].rect.size().x;

        assert!(scaled_width > baseline_width, "{scaled_width} vs {baseline_width}");
    }
}
