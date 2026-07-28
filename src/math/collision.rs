//! Axis-aligned 2D collision volumes.
//!
//! Deliberately hand-rolled rather than pulling in `parry2d`: rectangle overlap
//! is a dozen lines of `glam::Vec2` and the exercise is part of the point.

use glam::Vec2;

/// An axis-aligned 2D bounding box.
///
/// The invariant `min <= max` on both axes is established by every constructor;
/// building one directly through the public fields is your responsibility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb2d {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2d {
    /// Creates a box from two corners, normalising them so `min <= max`.
    pub fn new(a: Vec2, b: Vec2) -> Self {
        Aabb2d {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// Creates a box of `size` centred on `center`.
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size.abs() * 0.5;
        Aabb2d {
            min: center - half,
            max: center + half,
        }
    }

    /// The centre point of the box.
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// The full width and height of the box.
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Whether this box overlaps `other`. Boxes that merely touch along an edge
    /// count as intersecting.
    pub fn intersects(&self, other: &Aabb2d) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Whether `point` lies inside (or exactly on) the box.
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Returns a copy shifted by `delta`.
    pub fn translated(&self, delta: Vec2) -> Self {
        Aabb2d {
            min: self.min + delta,
            max: self.max + delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(x0: f32, y0: f32, x1: f32, y1: f32) -> Aabb2d {
        Aabb2d::new(Vec2::new(x0, y0), Vec2::new(x1, y1))
    }

    #[test]
    fn new_normalises_swapped_corners() {
        let b = Aabb2d::new(Vec2::new(3.0, 4.0), Vec2::new(-1.0, -2.0));
        assert_eq!(b.min, Vec2::new(-1.0, -2.0));
        assert_eq!(b.max, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn from_center_size_is_centred() {
        let b = Aabb2d::from_center_size(Vec2::new(5.0, 5.0), Vec2::new(2.0, 4.0));
        assert_eq!(b.min, Vec2::new(4.0, 3.0));
        assert_eq!(b.max, Vec2::new(6.0, 7.0));
        assert_eq!(b.center(), Vec2::new(5.0, 5.0));
        assert_eq!(b.size(), Vec2::new(2.0, 4.0));
    }

    #[test]
    fn overlapping_boxes_intersect() {
        assert!(boxed(0.0, 0.0, 2.0, 2.0).intersects(&boxed(1.0, 1.0, 3.0, 3.0)));
    }

    #[test]
    fn contained_box_intersects() {
        let outer = boxed(-5.0, -5.0, 5.0, 5.0);
        let inner = boxed(-1.0, -1.0, 1.0, 1.0);
        assert!(outer.intersects(&inner));
        assert!(inner.intersects(&outer));
    }

    #[test]
    fn disjoint_on_x_does_not_intersect() {
        assert!(!boxed(0.0, 0.0, 1.0, 1.0).intersects(&boxed(2.0, 0.0, 3.0, 1.0)));
    }

    #[test]
    fn disjoint_on_y_does_not_intersect() {
        assert!(!boxed(0.0, 0.0, 1.0, 1.0).intersects(&boxed(0.0, 2.0, 1.0, 3.0)));
    }

    #[test]
    fn edge_touching_counts_as_intersecting() {
        assert!(boxed(0.0, 0.0, 1.0, 1.0).intersects(&boxed(1.0, 0.0, 2.0, 1.0)));
    }

    #[test]
    fn intersection_is_symmetric() {
        let a = boxed(0.0, 0.0, 4.0, 1.0);
        let b = boxed(3.0, -1.0, 5.0, 0.5);
        assert_eq!(a.intersects(&b), b.intersects(&a));
        assert!(a.intersects(&b));
    }

    #[test]
    fn contains_point_covers_interior_and_border() {
        let b = boxed(0.0, 0.0, 2.0, 2.0);
        assert!(b.contains_point(Vec2::new(1.0, 1.0)));
        assert!(b.contains_point(Vec2::new(0.0, 2.0)));
        assert!(!b.contains_point(Vec2::new(2.1, 1.0)));
    }

    #[test]
    fn translated_moves_both_corners() {
        let b = boxed(0.0, 0.0, 1.0, 1.0).translated(Vec2::new(10.0, -10.0));
        assert_eq!(b.min, Vec2::new(10.0, -10.0));
        assert_eq!(b.max, Vec2::new(11.0, -9.0));
    }

    #[test]
    fn a_bird_passing_a_pipe_gap_does_not_collide() {
        // The Flappy Bird case this type exists for: bird between two pipes.
        let bird = Aabb2d::from_center_size(Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.5));
        let top_pipe = boxed(-0.4, 1.0, 0.4, 5.0);
        let bottom_pipe = boxed(-0.4, -5.0, 0.4, -1.0);
        assert!(!bird.intersects(&top_pipe));
        assert!(!bird.intersects(&bottom_pipe));

        let fallen_bird = bird.translated(Vec2::new(0.0, -0.9));
        assert!(fallen_bird.intersects(&bottom_pipe));
    }
}
