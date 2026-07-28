//! Flat, unlit colour — the `THREE.MeshBasicMaterial` equivalent.

use super::material::{Material, MaterialBinding, MaterialId};
use crate::math::Color;

/// Shades every fragment with one flat colour, ignoring all lights.
///
/// Deliberately unlit, exactly like its Three.js namesake: a spinning cube
/// drawn with it reads as a silhouette, which is the honest result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBasicMaterial {
    pub color: Color,
}

impl MeshBasicMaterial {
    pub fn new(color: Color) -> Self {
        MeshBasicMaterial { color }
    }
}

impl Default for MeshBasicMaterial {
    fn default() -> Self {
        MeshBasicMaterial::new(Color::WHITE)
    }
}

impl Material for MeshBasicMaterial {
    fn material_id(&self) -> MaterialId {
        MaterialId::Basic
    }

    fn bind(&self) -> MaterialBinding<'_> {
        MaterialBinding {
            color: self.color,
            texture: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_basic_pipeline_and_no_texture() {
        let m = MeshBasicMaterial::new(Color::hex(0x00ff88));
        assert_eq!(m.material_id(), MaterialId::Basic);
        assert_eq!(m.bind().color, Color::hex(0x00ff88));
        assert!(m.bind().texture.is_none());
    }

    #[test]
    fn the_default_material_is_opaque_white() {
        assert_eq!(MeshBasicMaterial::default().color, Color::WHITE);
    }

    #[test]
    fn recolouring_changes_what_gets_pushed_to_the_gpu() {
        let mut m = MeshBasicMaterial::new(Color::RED);
        m.color = Color::BLUE;
        assert_eq!(m.bind().color, Color::BLUE);
    }
}
