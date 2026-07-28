//! Position / rotation / scale, the one piece of state every `Object3D` owns.

use glam::{Mat4, Quat, Vec3};

/// A TRS transform: translation, Euler rotation (radians, YXZ order), and a
/// non-uniform scale.
///
/// Rotation is stored as Euler angles rather than a quaternion so that
/// `mesh.transform.rotation.y += 0.01` reads exactly like the Three.js code it
/// mirrors. The quaternion is derived on demand in [`Transform::matrix`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    /// Euler angles in radians, applied in YXZ order.
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    /// The identity transform: at the origin, unrotated, unit scale.
    pub const IDENTITY: Transform = Transform {
        position: Vec3::ZERO,
        rotation: Vec3::ZERO,
        scale: Vec3::ONE,
    };

    /// Creates an identity transform.
    pub fn new() -> Self {
        Transform::IDENTITY
    }

    /// Creates a transform at `position` with no rotation and unit scale.
    pub fn from_position(position: Vec3) -> Self {
        Transform {
            position,
            ..Transform::IDENTITY
        }
    }

    /// The rotation expressed as a quaternion.
    pub fn quat(&self) -> Quat {
        Quat::from_euler(
            glam::EulerRot::YXZ,
            self.rotation.y,
            self.rotation.x,
            self.rotation.z,
        )
    }

    /// The local-to-parent matrix, `T * R * S`.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.quat(), self.position)
    }

    /// Moves the transform by `delta` in parent space.
    pub fn translate(&mut self, delta: Vec3) -> &mut Self {
        self.position += delta;
        self
    }

    /// Adds `delta` (radians) to the Euler angles.
    pub fn rotate(&mut self, delta: Vec3) -> &mut Self {
        self.rotation += delta;
        self
    }

    /// Replaces the scale with a uniform value.
    pub fn set_uniform_scale(&mut self, scale: f32) -> &mut Self {
        self.scale = Vec3::splat(scale);
        self
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn approx_vec(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn identity_matrix_is_identity() {
        assert_eq!(Transform::new().matrix(), Mat4::IDENTITY);
    }

    #[test]
    fn translation_moves_the_origin() {
        let t = Transform::from_position(Vec3::new(1.0, 2.0, 3.0));
        let moved = t.matrix().transform_point3(Vec3::ZERO);
        assert!(approx_vec(moved, Vec3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn scale_multiplies_positions() {
        let mut t = Transform::new();
        t.scale = Vec3::new(2.0, 3.0, 4.0);
        let scaled = t.matrix().transform_point3(Vec3::ONE);
        assert!(approx_vec(scaled, Vec3::new(2.0, 3.0, 4.0)));
    }

    #[test]
    fn yaw_of_ninety_degrees_maps_x_to_negative_z() {
        let mut t = Transform::new();
        t.rotation.y = FRAC_PI_2;
        let rotated = t.matrix().transform_point3(Vec3::X);
        assert!(approx_vec(rotated, Vec3::new(0.0, 0.0, -1.0)), "{rotated:?}");
    }

    #[test]
    fn matrix_applies_scale_then_rotation_then_translation() {
        let mut t = Transform::from_position(Vec3::new(0.0, 5.0, 0.0));
        t.rotation.y = FRAC_PI_2;
        t.scale = Vec3::splat(2.0);
        // X axis scaled to length 2, yawed onto -Z, then lifted by 5 on Y.
        let p = t.matrix().transform_point3(Vec3::X);
        assert!(approx_vec(p, Vec3::new(0.0, 5.0, -2.0)), "{p:?}");
    }

    #[test]
    fn translate_and_rotate_accumulate() {
        let mut t = Transform::new();
        t.translate(Vec3::X).translate(Vec3::X);
        t.rotate(Vec3::Y * 0.5).rotate(Vec3::Y * 0.5);
        assert!(approx_vec(t.position, Vec3::new(2.0, 0.0, 0.0)));
        assert!(approx_vec(t.rotation, Vec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn set_uniform_scale_replaces_all_axes() {
        let mut t = Transform::new();
        t.scale = Vec3::new(9.0, 9.0, 9.0);
        t.set_uniform_scale(0.5);
        assert!(approx_vec(t.scale, Vec3::splat(0.5)));
    }

    #[test]
    fn quat_matches_matrix_rotation() {
        let mut t = Transform::new();
        t.rotation = Vec3::new(0.3, -0.7, 1.1);
        let from_quat = Mat4::from_quat(t.quat()).transform_point3(Vec3::X);
        let from_matrix = t.matrix().transform_point3(Vec3::X);
        assert!(approx_vec(from_quat, from_matrix));
    }
}
