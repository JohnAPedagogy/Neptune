//! The trait every light source implements.

use crate::math::{Color, Vec3};

/// Which kind of illumination a light contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightKind {
    /// Uniform illumination from every direction.
    Ambient,
    /// Parallel rays from an infinitely distant source.
    Directional,
}

/// A source of illumination in a [`Scene`](crate::core::Scene).
///
/// The built-in materials are unlit, so nothing in the current render path
/// reads these values yet — the types exist, are constructible, and are
/// queryable, ready for the lit pipeline to arrive.
pub trait Light {
    fn kind(&self) -> LightKind;
    fn color(&self) -> Color;
    fn intensity(&self) -> f32;

    /// The direction the light travels, for lights that have one.
    fn direction(&self) -> Option<Vec3> {
        None
    }
}
