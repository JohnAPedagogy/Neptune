//! What a material has to tell the renderer.

use std::sync::atomic::{AtomicU64, Ordering};

use super::texture::Texture;
use crate::math::Color;

/// Which built-in pipeline a material draws with.
///
/// This is the key of the renderer's pipeline cache: every material *instance*
/// that reports the same `MaterialId` shares one compiled pipeline, and differs
/// only in the per-draw data returned by [`Material::bind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MaterialId {
    /// Flat, unlit colour. Opaque, depth-tested and depth-written.
    Basic,
    /// Textured quad, tinted by a colour, alpha-blended and depth-read-only.
    Sprite,
}

/// Everything the renderer needs to record one draw call with this material.
///
/// Returned by [`Material::bind`] — the hook a material implements instead of
/// touching any GPU type itself.
#[derive(Debug, Clone, Copy)]
pub struct MaterialBinding<'a> {
    /// Flat colour for [`MaterialId::Basic`]; a multiplied tint for
    /// [`MaterialId::Sprite`].
    pub color: Color,
    /// The texture sampled by the fragment shader, for textured pipelines.
    pub texture: Option<&'a Texture>,
}

/// How a surface is shaded.
pub trait Material {
    /// Which pipeline this material draws with.
    fn material_id(&self) -> MaterialId;

    /// Stable per-instance identity, used to cache the descriptor set built
    /// from this material's bindings.
    fn instance_id(&self) -> MaterialInstanceId;

    /// The per-draw data the renderer pushes to the GPU.
    fn bind(&self) -> MaterialBinding<'_>;
}

/// Lets a `Box<dyn Material>` be used wherever a `Material` is expected, so a
/// `Mesh` can carry a material chosen at runtime.
impl<M: Material + ?Sized> Material for Box<M> {
    fn material_id(&self) -> MaterialId {
        (**self).material_id()
    }

    fn instance_id(&self) -> MaterialInstanceId {
        (**self).instance_id()
    }

    fn bind(&self) -> MaterialBinding<'_> {
        (**self).bind()
    }
}

/// A process-unique identity for one material *instance*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialInstanceId(pub(crate) u64);

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

impl MaterialInstanceId {
    pub(crate) fn next() -> Self {
        MaterialInstanceId(NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Object3D;
    use crate::geometry::BoxGeometry;
    use crate::materials::{MeshBasicMaterial, SpriteMaterial, Texture};
    use crate::math::Color;
    use crate::objects::Mesh;

    #[test]
    fn instance_ids_are_unique() {
        assert_ne!(MaterialInstanceId::next(), MaterialInstanceId::next());
    }

    #[test]
    fn a_boxed_material_forwards_to_the_material_inside() {
        let boxed: Box<dyn Material> = Box::new(MeshBasicMaterial::new(Color::RED));
        assert_eq!(boxed.material_id(), MaterialId::Basic);
        assert_eq!(boxed.bind().color, Color::RED);
    }

    #[test]
    fn a_mesh_can_carry_a_material_chosen_at_runtime() {
        // The Ch22 payoff: the material type is erased, but the mesh still
        // renders, because `Box<dyn Material>` is itself a `Material`.
        let textured = false;
        let material: Box<dyn Material> = if textured {
            Box::new(SpriteMaterial::new(Texture::white()))
        } else {
            Box::new(MeshBasicMaterial::new(Color::BLUE))
        };

        let mesh = Mesh::new(BoxGeometry::cube(1.0), material);
        let renderable = mesh.renderable().expect("a mesh always draws something");
        assert_eq!(renderable.material.material_id(), MaterialId::Basic);
        assert_eq!(renderable.material.bind().color, Color::BLUE);
    }
}
