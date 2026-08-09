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
    /// The most recent row rect, for `same_line` and `tooltip`.
    last_row: Option<Aabb2d>,
    /// When set, the next [`Layout::row`] parks itself on the current line,
    /// right of the last row, instead of advancing `cursor_y`.
    next_x: Option<f32>,
    /// While true, every row advances x (columns) along one shared line until
    /// [`Layout::end_horizontal`] — `UiFrame::horizontal`'s mode.
    horizontal: bool,
    /// `cursor_y` when horizontal mode began, restored by `end_horizontal`.
    horizontal_start_y: f32,
    /// Tallest row seen while horizontal, so `end_horizontal` can reserve the
    /// right amount of vertical space for the whole row.
    horizontal_max_h: f32,
}

impl Layout {
    /// Vertical gap between rows, in pixels.
    pub(crate) const PADDING: f32 = 4.0;

    pub(crate) fn new(origin: Vec2, width: f32) -> Self {
        Layout {
            origin,
            cursor_y: 0.0,
            width,
            last_row: None,
            next_x: None,
            horizontal: false,
            horizontal_start_y: 0.0,
            horizontal_max_h: 0.0,
        }
    }

    /// Allocates the next row, `height` pixels tall and the panel's current
    /// width, and advances the cursor past it (plus [`Layout::PADDING`]).
    pub(crate) fn row(&mut self, height: f32) -> Aabb2d {
        let rect = if self.horizontal {
            // Column on the shared line, to the right of the previous column.
            let x = self.last_row.map(|r| r.max.x).unwrap_or(self.origin.x);
            let y = self.origin.y + self.horizontal_start_y;
            self.horizontal_max_h = self.horizontal_max_h.max(height);
            Aabb2d::new(
                Vec2::new(x, y),
                Vec2::new(self.origin.x + self.width, y + height),
            )
        } else if let Some(x) = self.next_x.take() {
            // `same_line`: the immediate next widget sits beside the last row.
            let y = self.last_row
                .map(|r| r.min.y)
                .unwrap_or(self.origin.y + self.cursor_y);
            Aabb2d::new(
                Vec2::new(x, y),
                Vec2::new(self.origin.x + self.width, y + height),
            )
        } else {
            let y0 = self.origin.y + self.cursor_y;
            self.cursor_y += height + Layout::PADDING;
            Aabb2d::new(
                Vec2::new(self.origin.x, y0),
                Vec2::new(self.origin.x + self.width, y0 + height),
            )
        };
        self.last_row = Some(rect);
        rect
    }

    /// Parks the next [`Layout::row`] on the current line, to the right of the
    /// last row — `UiFrame::same_line`.
    pub(crate) fn same_line(&mut self) {
        if !self.horizontal {
            self.next_x = self.last_row.map(|r| r.max.x);
        }
    }

    /// Enters horizontal mode: every subsequent row becomes a column on one
    /// shared line. [`Layout::end_horizontal`] restores normal vertical flow,
    /// reserving just the tallest column's height.
    pub(crate) fn begin_horizontal(&mut self) {
        self.horizontal = true;
        self.horizontal_start_y = self.cursor_y;
        self.horizontal_max_h = 0.0;
        self.next_x = None;
    }

    /// Leaves horizontal mode and reserves one row's worth of vertical space.
    pub(crate) fn end_horizontal(&mut self) {
        self.horizontal = false;
        self.cursor_y = self.horizontal_start_y + self.horizontal_max_h + Layout::PADDING;
        self.last_row = None;
        self.next_x = None;
    }

    /// The most recent row rect, for `tooltip` (hover over the last widget).
    pub(crate) fn last_row(&self) -> Option<Aabb2d> {
        self.last_row
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

    #[test]
    fn same_line_places_the_next_row_beside_the_last_without_advancing_y() {
        let mut layout = Layout::new(Vec2::new(0.0, 0.0), 100.0);
        let first = layout.row(20.0);
        layout.same_line();
        let second = layout.row(20.0);
        assert_eq!(second.min.x, first.max.x);
        assert_eq!(second.min.y, first.min.y);
        // The row after a same_line pair flows below the original line.
        let third = layout.row(20.0);
        assert_eq!(third.min.y, first.max.y + Layout::PADDING);
    }

    #[test]
    fn horizontal_rows_are_columns_on_one_line() {
        let mut layout = Layout::new(Vec2::new(0.0, 0.0), 100.0);
        layout.begin_horizontal();
        let a = layout.row(20.0);
        let b = layout.row(20.0);
        let c = layout.row(20.0);
        assert_eq!(a.min.y, b.min.y);
        assert_eq!(b.min.x, a.max.x);
        assert_eq!(c.min.x, b.max.x);
        layout.end_horizontal();
        // Vertical space reserved is just one tall column plus padding.
        assert_eq!(layout.cursor_y(), 20.0 + Layout::PADDING);
    }

    #[test]
    fn last_row_tracks_the_most_recent_row() {
        let mut layout = Layout::new(Vec2::new(0.0, 0.0), 100.0);
        assert!(layout.last_row().is_none());
        let row = layout.row(20.0);
        assert_eq!(layout.last_row(), Some(row));
    }
}
