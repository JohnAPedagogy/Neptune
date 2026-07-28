//! Geometry + material + transform: the thing you actually put in a scene.

use std::any::Any;

use crate::core::{Object3D, Renderable};
use crate::geometry::Geometry;
use crate::materials::Material;
use crate::math::Transform;

/// A drawable object: one geometry, shaded by one material, placed by one
/// transform.
///
/// Generic over both halves so a `Mesh<BoxGeometry, MeshBasicMaterial>` is a
/// distinct, statically dispatched type — right up until [`Scene::add`] boxes
/// it into a `dyn Object3D`.
///
/// [`Scene::add`]: crate::core::Scene::add
pub struct Mesh<G, M> {
    pub geometry: G,
    pub material: M,
    pub transform: Transform,
    pub visible: bool,
}

impl<G, M> Mesh<G, M> {
    pub fn new(geometry: G, material: M) -> Self {
        Mesh {
            geometry,
            material,
            transform: Transform::IDENTITY,
            visible: true,
        }
    }

    /// Builder-style transform override, for the common
    /// `Mesh::new(..).with_transform(..)` shape.
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }
}

impl<G, M> Object3D for Mesh<G, M>
where
    G: Geometry + 'static,
    M: Material + 'static,
{
    fn transform(&self) -> &Transform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn renderable(&self) -> Option<Renderable<'_>> {
        Some(Renderable {
            geometry: &self.geometry,
            material: &self.material,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{BoxGeometry, GeometryId};
    use crate::materials::{MaterialId, MeshBasicMaterial};
    use crate::math::{Color, Vec3};

    #[test]
    fn a_new_mesh_is_visible_at_the_origin() {
        let mesh = Mesh::new(BoxGeometry::cube(1.0), MeshBasicMaterial::new(Color::RED));
        assert!(mesh.visible);
        assert_eq!(mesh.transform, Transform::IDENTITY);
    }

    #[test]
    fn transform_fields_are_directly_assignable() {
        let mut mesh = Mesh::new(BoxGeometry::cube(1.0), MeshBasicMaterial::default());
        mesh.transform.position.z = -5.0;
        mesh.transform.rotation.y += 0.25;
        assert_eq!(mesh.transform.position, Vec3::new(0.0, 0.0, -5.0));
        assert_eq!(mesh.transform.rotation.y, 0.25);
    }

    #[test]
    fn renderable_exposes_the_geometry_and_material_it_was_built_with() {
        let geometry = BoxGeometry::cube(2.0);
        let expected_id: GeometryId = crate::geometry::Geometry::geometry_id(&geometry);
        let mesh = Mesh::new(geometry, MeshBasicMaterial::default());

        let renderable = mesh.renderable().expect("a mesh always has something to draw");
        assert_eq!(renderable.geometry.geometry_id(), expected_id);
        assert_eq!(renderable.material.material_id(), MaterialId::Basic);
    }

    #[test]
    fn an_invisible_mesh_reports_itself_as_such() {
        let mut mesh = Mesh::new(BoxGeometry::cube(1.0), MeshBasicMaterial::default());
        mesh.visible = false;
        assert!(!Object3D::visible(&mesh));
    }
}
