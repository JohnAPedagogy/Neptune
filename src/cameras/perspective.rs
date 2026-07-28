//! Perspective projection — the 3D default.

use super::camera::Camera;
use crate::math::{Mat4, Transform, Vec3};

/// A pinhole camera with a vertical field of view.
///
/// Mirrors `THREE.PerspectiveCamera`: `PerspectiveCamera::new(fov, aspect,
/// near, far)`, with `fov` in **radians** (use `75.0_f32.to_radians()`).
pub struct PerspectiveCamera {
    /// Vertical field of view, in radians.
    pub fov: f32,
    /// Viewport width divided by height.
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    /// Where the camera is and which way it faces. The view matrix is this
    /// transform inverted.
    pub transform: Transform,
}

impl PerspectiveCamera {
    pub fn new(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        PerspectiveCamera {
            fov,
            aspect,
            near,
            far,
            transform: Transform::IDENTITY,
        }
    }

    /// Points the camera at `target`, keeping its current position.
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let view = Mat4::look_at_rh(self.transform.position, target, up);
        let (scale, rotation, translation) = view.inverse().to_scale_rotation_translation();
        let (y, x, z) = rotation.to_euler(glam::EulerRot::YXZ);
        self.transform.position = translation;
        self.transform.rotation = Vec3::new(x, y, z);
        self.transform.scale = scale;
    }
}

impl Camera for PerspectiveCamera {
    fn view_matrix(&self) -> Mat4 {
        self.transform.matrix().inverse()
    }

    fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn camera() -> PerspectiveCamera {
        PerspectiveCamera::new(FRAC_PI_2, 16.0 / 9.0, 0.1, 100.0)
    }

    #[test]
    fn an_untransformed_camera_has_an_identity_view() {
        assert_eq!(camera().view_matrix(), Mat4::IDENTITY);
    }

    #[test]
    fn moving_the_camera_moves_the_world_the_other_way() {
        let mut cam = camera();
        cam.transform.position = Vec3::new(0.0, 0.0, 5.0);
        let seen = cam.view_matrix().transform_point3(Vec3::ZERO);
        assert!((seen - Vec3::new(0.0, 0.0, -5.0)).length() < 1e-5, "{seen:?}");
    }

    #[test]
    fn projection_maps_the_near_plane_to_zero_depth() {
        let cam = camera();
        let p = cam.proj_matrix() * glam::Vec4::new(0.0, 0.0, -cam.near, 1.0);
        assert!((p.z / p.w).abs() < 1e-4, "near depth was {}", p.z / p.w);
    }

    #[test]
    fn projection_maps_the_far_plane_to_one_depth() {
        let cam = camera();
        let p = cam.proj_matrix() * glam::Vec4::new(0.0, 0.0, -cam.far, 1.0);
        assert!((p.z / p.w - 1.0).abs() < 1e-3, "far depth was {}", p.z / p.w);
    }

    #[test]
    fn a_wider_aspect_squeezes_x_more_than_y() {
        let cam = PerspectiveCamera::new(FRAC_PI_2, 2.0, 0.1, 100.0);
        let m = cam.proj_matrix();
        assert!(m.x_axis.x < m.y_axis.y);
    }

    #[test]
    fn look_at_points_the_camera_down_negative_z_toward_the_target() {
        let mut cam = camera();
        cam.transform.position = Vec3::new(0.0, 0.0, 10.0);
        cam.look_at(Vec3::ZERO, Vec3::Y);
        let seen = cam.view_matrix().transform_point3(Vec3::ZERO);
        assert!((seen - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-4, "{seen:?}");
    }

    #[test]
    fn view_proj_is_proj_times_view() {
        let mut cam = camera();
        cam.transform.position = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(
            cam.view_proj_matrix(),
            cam.proj_matrix() * cam.view_matrix()
        );
    }
}
