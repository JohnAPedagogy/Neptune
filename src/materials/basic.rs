//! Flat, unlit colour — the `THREE.MeshBasicMaterial` equivalent.

use super::material::{Material, MaterialBinding, MaterialId, MaterialInstanceId};
use crate::math::Color;

/// Shades every fragment with one flat colour, ignoring all lights.
///
/// Deliberately unlit, exactly like its Three.js namesake: a spinning cube
/// drawn with it reads as a silhouette, which is the honest result.
#[derive(Debug, Clone)]
pub struct MeshBasicMaterial {
    pub color: Color,
    instance_id: MaterialInstanceId,
}

impl MeshBasicMaterial {
    pub fn new(color: Color) -> Self {
        MeshBasicMaterial {
            color,
            instance_id: MaterialInstanceId::next(),
        }
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

    fn instance_id(&self) -> MaterialInstanceId {
        self.instance_id
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
    fn cloning_preserves_the_instance_identity() {
        // Clones share a pipeline *and* a descriptor set; they are the same
        // material as far as the renderer's caches are concerned.
        let m = MeshBasicMaterial::new(Color::RED);
        assert_eq!(m.clone().instance_id(), m.instance_id());
    }

    #[test]
    fn distinct_materials_get_distinct_instance_ids() {
        let a = MeshBasicMaterial::new(Color::RED);
        let b = MeshBasicMaterial::new(Color::RED);
        assert_ne!(a.instance_id(), b.instance_id());
    }
}
