//! The one trait everything in a scene implements.

use std::any::Any;

use crate::geometry::Geometry;
use crate::materials::Material;
use crate::math::Transform;

/// The drawable half of an [`Object3D`], handed to the renderer once per frame.
///
/// Note what is *not* here: no buffers, no pipelines, no Vulkan types at all.
/// An object describes what it is made of; the renderer decides how that
/// becomes GPU work.
pub struct Renderable<'a> {
    pub geometry: &'a dyn Geometry,
    pub material: &'a dyn Material,
}

/// Anything a [`Scene`](super::Scene) can hold.
///
/// The `Any` supertrait is what makes [`Scene::query_mut`](super::Scene::query_mut)
/// possible: a `Box<dyn Object3D>` can be downcast back to the concrete type
/// that was moved in.
pub trait Object3D: Any {
    /// This object's local transform.
    fn transform(&self) -> &Transform;

    /// Mutable access to the local transform — the `mesh.position.set(..)`
    /// equivalent once the concrete type has been erased.
    fn transform_mut(&mut self) -> &mut Transform;

    /// Whether the renderer should draw this object and its children.
    fn visible(&self) -> bool {
        true
    }

    /// What to draw for this object itself, if anything. Containers such as
    /// [`Group`](super::Group) return `None` and contribute only children.
    fn renderable(&self) -> Option<Renderable<'_>> {
        None
    }

    /// Children drawn relative to this object's transform.
    fn children(&self) -> &[Box<dyn Object3D>] {
        &[]
    }

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}
