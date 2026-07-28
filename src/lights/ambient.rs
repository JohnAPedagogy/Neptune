//! Uniform illumination from everywhere at once.

use super::light::{Light, LightKind};
use crate::math::Color;

/// Lights every surface equally, regardless of orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientLight {
    pub color: Color,
    pub intensity: f32,
}

impl AmbientLight {
    pub fn new(color: Color, intensity: f32) -> Self {
        AmbientLight { color, intensity }
    }
}

impl Light for AmbientLight {
    fn kind(&self) -> LightKind {
        LightKind::Ambient
    }

    fn color(&self) -> Color {
        self.color
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_its_kind_colour_and_intensity() {
        let light = AmbientLight::new(Color::hex(0x404060), 0.35);
        assert_eq!(light.kind(), LightKind::Ambient);
        assert_eq!(light.color(), Color::hex(0x404060));
        assert_eq!(light.intensity(), 0.35);
    }

    #[test]
    fn has_no_direction() {
        assert!(AmbientLight::new(Color::WHITE, 1.0).direction().is_none());
    }
}
