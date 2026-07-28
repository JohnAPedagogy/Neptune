//! Parallel rays from an infinitely distant source — sunlight.

use super::light::{Light, LightKind};
use crate::math::{Color, Vec3};

/// Lights surfaces according to how they face a fixed direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    pub color: Color,
    pub intensity: f32,
    /// The direction the light travels, normalised on construction.
    pub direction: Vec3,
}

impl DirectionalLight {
    /// Creates a light shining straight down (`-Y`).
    pub fn new(color: Color, intensity: f32) -> Self {
        DirectionalLight {
            color,
            intensity,
            direction: Vec3::NEG_Y,
        }
    }

    /// Sets the direction the light travels. A zero vector is ignored.
    pub fn with_direction(mut self, direction: Vec3) -> Self {
        if let Some(normalized) = direction.try_normalize() {
            self.direction = normalized;
        }
        self
    }
}

impl Light for DirectionalLight {
    fn kind(&self) -> LightKind {
        LightKind::Directional
    }

    fn color(&self) -> Color {
        self.color
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn direction(&self) -> Option<Vec3> {
        Some(self.direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_shining_downward() {
        let light = DirectionalLight::new(Color::WHITE, 1.0);
        assert_eq!(light.kind(), LightKind::Directional);
        assert_eq!(light.direction(), Some(Vec3::NEG_Y));
    }

    #[test]
    fn with_direction_normalises() {
        let light =
            DirectionalLight::new(Color::WHITE, 1.0).with_direction(Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(light.direction, Vec3::Z);
    }

    #[test]
    fn a_zero_direction_is_ignored() {
        let light = DirectionalLight::new(Color::WHITE, 1.0).with_direction(Vec3::ZERO);
        assert_eq!(light.direction, Vec3::NEG_Y);
    }
}
