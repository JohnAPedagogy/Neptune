//! The persistent widget state (`Ui`) and the per-frame builder (`UiFrame`)
//! every widget call in `widgets.rs` extends.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::input::MouseState;
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

/// State that cannot be reconstructed from a single frame's widget calls —
/// which widget is being dragged, which dropdown is open, which folder is
/// collapsed — kept across frames so an immediate-mode call site still gets
/// drag/click/toggle behavior. See `neptune-imgui-plus-datgui.md` §4.
pub struct Ui {
    pub(crate) atlas: Arc<GlyphAtlas>,
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
    /// `width` pixels wide. `mouse` and `screen` should come straight from
    /// `frame.input().mouse()` and `frame.size()`.
    pub fn begin<'a>(
        &'a mut self,
        mouse: &'a MouseState,
        screen: (f32, f32),
        origin: Vec2,
        width: f32,
    ) -> UiFrame<'a> {
        UiFrame {
            ui: self,
            mouse,
            screen,
            layout: Layout::new(origin, width),
            draw_list: UiDrawList::new(),
        }
    }
}

/// Everything one frame's widget calls need: a mutable borrow of the
/// persistent [`Ui`] state, this frame's mouse snapshot, and the draw list
/// being built up call by call.
pub struct UiFrame<'a> {
    pub(crate) ui: &'a mut Ui,
    pub(crate) mouse: &'a MouseState,
    /// Window size in pixels, in the same Y-down space widget rects use —
    /// read by `window` to clamp drags and decide dock edges.
    pub(crate) screen: (f32, f32),
    pub(crate) layout: Layout,
    pub(crate) draw_list: UiDrawList,
}

impl<'a> UiFrame<'a> {
    /// Ends the frame, handing back the primitives `Frame::render_ui` draws.
    pub fn finish(self) -> UiDrawList {
        self.draw_list
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
    use crate::input::MouseState;
    use crate::math::Vec2;
    use crate::text::Font;

    fn ui() -> Option<Ui> {
        let atlas = Font::system_default().ok()?.atlas(24.0).ok()?;
        Some(Ui::new(atlas))
    }

    #[test]
    fn a_frame_with_no_widgets_produces_an_empty_draw_list() {
        let Some(mut ui) = ui() else { return };
        let mouse = MouseState::new();
        let frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 200.0);
        assert!(frame.finish().is_empty());
    }

    #[test]
    fn push_quad_appends_one_primitive_with_the_shared_white_texture() {
        let Some(mut ui) = ui() else { return };
        let mouse = MouseState::new();
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 200.0);
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
        let mouse = MouseState::new();
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 200.0);
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
        let mouse = MouseState::new();
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.window("Settings", 200.0, |_| {});
        assert_eq!(
            frame.ui.panel_origin[&WidgetId::new("Settings", 1)],
            Vec2::new(40.0, 60.0)
        );
    }

    #[test]
    fn scaling_pixels_per_point_grows_the_labels_glyph_rects() {
        let Some(mut ui) = ui() else { return };
        let mouse = MouseState::new();

        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.label("A", TextStyle::Body, crate::math::Color::WHITE);
        let baseline_width = frame.finish().primitives[0].rect.size().x;

        ui.set_pixels_per_point(2.0);
        let mut frame = ui.begin(&mouse, (800.0, 600.0), Vec2::ZERO, 200.0);
        frame.label("A", TextStyle::Body, crate::math::Color::WHITE);
        let scaled_width = frame.finish().primitives[0].rect.size().x;

        assert!(scaled_width > baseline_width, "{scaled_width} vs {baseline_width}");
    }
}
