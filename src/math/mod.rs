//! Small math types layered on top of [`glam`].
//!
//! Neptune does not reimplement linear algebra — `Vec2`/`Vec3`/`Mat4`/`Quat` are
//! re-exported straight from `glam`. This module only adds the handful of types
//! `glam` has no opinion about: colour, a TRS transform, and a 2D AABB.

mod collision;
mod color;
mod transform;

pub use collision::Aabb2d;
pub use color::Color;
pub use transform::Transform;

pub use glam::{EulerRot, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
