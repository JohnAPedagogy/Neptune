//! Flat, unlit colour — the `THREE.MeshBasicMaterial` equivalent.

use super::material::{Material, MaterialBinding, MaterialId};
use crate::math::Color;

/// Shades every fragment with one flat colour, ignoring all lights.
///
/// Deliberately unlit, exactly like its Three.js namesake: a spinning cube
/// drawn with it reads as a silhouette, which is the honest result. Set
/// [`wireframe`](MeshBasicMaterial::wireframe) to draw only the triangle edges,
/// the `MeshBasicMaterial({ wireframe: true })` look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBasicMaterial {
    pub color: Color,
    /// Draw triangle edges instead of filled faces.
    pub wireframe: bool,
}

impl MeshBasicMaterial {
    pub fn new(color: Color) -> Self {
        MeshBasicMaterial {
            color,
            wireframe: false,
        }
    }

    /// Flips the material to wireframe rendering.
    pub fn with_wireframe(mut self, wireframe: bool) -> Self {
        self.wireframe = wireframe;
        self
    }
}

impl Default for MeshBasicMaterial {
    fn default() -> Self {
        MeshBasicMaterial::new(Color::WHITE)
    }
}

impl Material for MeshBasicMaterial {
    fn material_id(&self) -> MaterialId {
        if self.wireframe {
            MaterialId::BasicWireframe
        } else {
            MaterialId::Basic
        }
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

    #[test]
    fn wireframe_selects_the_wireframe_pipeline() {
        let m = MeshBasicMaterial::new(Color::RED).with_wireframe(true);
        assert_eq!(m.material_id(), MaterialId::BasicWireframe);
        assert_eq!(m.bind().color, Color::RED);
        assert_eq!(MeshBasicMaterial::new(Color::RED).material_id(), MaterialId::Basic);
    }
}
