//! Orthographic projection — the 2D workhorse.

use super::camera::Camera;
use crate::math::{Mat4, Transform};

/// A camera with no perspective foreshortening: world units map linearly to
/// screen space.
///
/// This is what a 2D game renders through. [`OrthographicCamera::from_size`]
/// is usually what you want — it gives you a view volume measured in world
/// units, so a quad of width `1.0` is always the same fraction of the screen
/// regardless of depth.
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
    pub transform: Transform,
}

impl OrthographicCamera {
    /// Builds a camera from explicit view-volume bounds.
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        OrthographicCamera {
            left,
            right,
            bottom,
            top,
            near,
            far,
            transform: Transform::IDENTITY,
        }
    }

    /// Builds a camera showing a `width` x `height` window of the world,
    /// centred on the camera's position, with `+Y` up.
    pub fn from_size(width: f32, height: f32, near: f32, far: f32) -> Self {
        let (hw, hh) = (width * 0.5, height * 0.5);
        OrthographicCamera::new(-hw, hw, -hh, hh, near, far)
    }

    /// Resizes the view volume to `height` world units tall and
    /// `height * aspect` wide, keeping it centred.
    pub fn set_view_height(&mut self, height: f32, aspect: f32) {
        let hh = height * 0.5;
        let hw = hh * aspect;
        self.left = -hw;
        self.right = hw;
        self.bottom = -hh;
        self.top = hh;
    }

    /// Moves the camera in the XY plane without changing its depth.
    pub fn set_center(&mut self, x: f32, y: f32) {
        self.transform.position.x = x;
        self.transform.position.y = y;
    }
}

impl Camera for OrthographicCamera {
    fn view_matrix(&self) -> Mat4 {
        self.transform.matrix().inverse()
    }

    fn proj_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    fn camera() -> OrthographicCamera {
        OrthographicCamera::from_size(20.0, 10.0, 0.1, 100.0)
    }

    #[test]
    fn from_size_centres_the_view_volume() {
        let cam = camera();
        assert_eq!((cam.left, cam.right), (-10.0, 10.0));
        assert_eq!((cam.bottom, cam.top), (-5.0, 5.0));
    }

    #[test]
    fn the_view_volume_edges_map_to_clip_space_edges() {
        let cam = camera();
        let m = cam.proj_matrix();
        let right = m * glam::Vec4::new(10.0, 0.0, -1.0, 1.0);
        let top = m * glam::Vec4::new(0.0, 5.0, -1.0, 1.0);
        assert!((right.x - 1.0).abs() < 1e-5, "{}", right.x);
        assert!((top.y - 1.0).abs() < 1e-5, "{}", top.y);
    }

    #[test]
    fn depth_is_independent_of_screen_position() {
        // The defining property of an orthographic projection: no divide by w.
        let m = camera().proj_matrix();
        let a = m * glam::Vec4::new(0.0, 0.0, -50.0, 1.0);
        let b = m * glam::Vec4::new(9.0, 4.0, -50.0, 1.0);
        assert_eq!(a.w, 1.0);
        assert!((a.z - b.z).abs() < 1e-6);
    }

    #[test]
    fn near_and_far_map_to_zero_and_one() {
        let cam = camera();
        let m = cam.proj_matrix();
        let near = m * glam::Vec4::new(0.0, 0.0, -cam.near, 1.0);
        let far = m * glam::Vec4::new(0.0, 0.0, -cam.far, 1.0);
        assert!(near.z.abs() < 1e-4, "{}", near.z);
        assert!((far.z - 1.0).abs() < 1e-4, "{}", far.z);
    }

    #[test]
    fn set_view_height_applies_the_aspect_to_width_only() {
        let mut cam = camera();
        cam.set_view_height(8.0, 2.0);
        assert_eq!((cam.bottom, cam.top), (-4.0, 4.0));
        assert_eq!((cam.left, cam.right), (-8.0, 8.0));
    }

    #[test]
    fn panning_the_camera_shifts_what_is_visible() {
        let mut cam = camera();
        cam.set_center(3.0, -2.0);
        let seen = cam.view_matrix().transform_point3(Vec3::new(3.0, -2.0, 0.0));
        assert!(seen.length() < 1e-5, "{seen:?}");
    }
}
