//! A stable per-widget identity, and the vertical-stack cursor that hands
//! each widget its row rectangle.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::math::{Aabb2d, Vec2};

/// A stable identity for one widget, derived from its label and an optional
/// salt for repeated labels (e.g. sliders built in a loop).
///
/// `DefaultHasher::new()` uses fixed keys (not `RandomState`'s per-process
/// random ones), so the same `(label, salt)` pair hashes identically on every
/// call within — and across — runs, which is what makes persistent state
/// keyed by `WidgetId` (drag/collapse/open-dropdown, in `Ui`) line up with
/// the same widget frame after frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WidgetId(u64);

impl WidgetId {
    pub(crate) fn new(label: &str, salt: u64) -> Self {
        let mut hasher = DefaultHasher::new();
        label.hash(&mut hasher);
        salt.hash(&mut hasher);
        WidgetId(hasher.finish())
    }
}

/// A top-to-bottom cursor over one panel's rectangle, handing each widget the
/// next row and letting `folder` indent/outdent its children. Clonable so a
/// container (`window`) can save the caller's cursor and restore it after
/// laying its own contents out.
#[derive(Clone)]
pub(crate) struct Layout {
    origin: Vec2,
    cursor_y: f32,
    width: f32,
}

impl Layout {
    /// Vertical gap between rows, in pixels.
    pub(crate) const PADDING: f32 = 4.0;

    pub(crate) fn new(origin: Vec2, width: f32) -> Self {
        Layout {
            origin,
            cursor_y: 0.0,
            width,
        }
    }

    /// Allocates the next row, `height` pixels tall and the panel's current
    /// width, and advances the cursor past it (plus [`Layout::PADDING`]).
    pub(crate) fn row(&mut self, height: f32) -> Aabb2d {
        let y0 = self.origin.y + self.cursor_y;
        self.cursor_y += height + Layout::PADDING;
        Aabb2d::new(
            Vec2::new(self.origin.x, y0),
            Vec2::new(self.origin.x + self.width, y0 + height),
        )
    }

    /// Shifts the left edge right by `dx`, shrinking the row width to match —
    /// what `folder` uses so its children sit indented under the header.
    pub(crate) fn indent(&mut self, dx: f32) {
        self.origin.x += dx;
        self.width -= dx;
    }

    /// Undoes a matching [`Layout::indent`].
    pub(crate) fn outdent(&mut self, dx: f32) {
        self.origin.x -= dx;
        self.width += dx;
    }

    /// Pixels of vertical space consumed so far, relative to `origin` — what a
    /// container (`window`) reads to learn how tall its contents grew this
    /// frame.
    pub(crate) fn cursor_y(&self) -> f32 {
        self.cursor_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec2;

    #[test]
    fn same_label_and_salt_produce_the_same_id() {
        assert_eq!(WidgetId::new("Speed", 0), WidgetId::new("Speed", 0));
    }

    #[test]
    fn a_different_label_produces_a_different_id() {
        assert_ne!(WidgetId::new("Speed", 0), WidgetId::new("Wireframe", 0));
    }

    #[test]
    fn a_different_salt_produces_a_different_id_for_the_same_label() {
        // The escape hatch for repeated labels, e.g. sliders in a loop.
        assert_ne!(WidgetId::new("Item", 0), WidgetId::new("Item", 1));
    }

    #[test]
    fn the_first_row_starts_at_the_layout_origin() {
        let mut layout = Layout::new(Vec2::new(10.0, 20.0), 200.0);
        let row = layout.row(24.0);
        assert_eq!(row.min, Vec2::new(10.0, 20.0));
        assert_eq!(row.max, Vec2::new(210.0, 44.0));
    }

    #[test]
    fn the_second_row_is_offset_by_the_first_rows_height_plus_padding() {
        let mut layout = Layout::new(Vec2::new(0.0, 0.0), 100.0);
        let first = layout.row(24.0);
        let second = layout.row(24.0);
        assert_eq!(second.min.y, first.max.y + Layout::PADDING);
    }

    #[test]
    fn indent_shrinks_the_row_width_from_the_left() {
        let mut layout = Layout::new(Vec2::new(0.0, 0.0), 100.0);
        layout.indent(16.0);
        let row = layout.row(20.0);
        assert_eq!(row.min.x, 16.0);
        assert_eq!(row.max.x, 100.0);
    }

    #[test]
    fn outdent_undoes_a_matching_indent() {
        let mut layout = Layout::new(Vec2::new(0.0, 0.0), 100.0);
        layout.indent(16.0);
        layout.outdent(16.0);
        let row = layout.row(20.0);
        assert_eq!(row.min.x, 0.0);
        assert_eq!(row.max.x, 100.0);
    }
}
