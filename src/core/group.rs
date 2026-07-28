//! A named container whose transform applies to everything inside it.

use std::any::Any;

use super::object3d::Object3D;
use crate::math::Transform;

/// A node with a transform and children but nothing of its own to draw.
///
/// Moving a `Group` moves everything in it: the renderer multiplies the
/// group's matrix into each child's before drawing.
pub struct Group {
    pub name: String,
    pub transform: Transform,
    children: Vec<Box<dyn Object3D>>,
}

impl Group {
    pub fn new(name: impl Into<String>) -> Self {
        Group {
            name: name.into(),
            transform: Transform::IDENTITY,
            children: Vec::new(),
        }
    }

    /// Moves `child` into the group and returns its index.
    pub fn add(&mut self, child: impl Object3D + 'static) -> usize {
        self.children.push(Box::new(child));
        self.children.len() - 1
    }

    pub fn get(&self, index: usize) -> Option<&dyn Object3D> {
        self.children.get(index).map(|c| c.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut dyn Object3D> {
        self.children.get_mut(index).map(|c| c.as_mut())
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl Object3D for Group {
    fn transform(&self) -> &Transform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn children(&self) -> &[Box<dyn Object3D>] {
        &self.children
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
    use crate::geometry::{BoxGeometry, BufferGeometry, SimpleVertex};
    use crate::materials::MeshBasicMaterial;
    use crate::math::{Color, Vec3};
    use crate::objects::Mesh;

    fn cube() -> Mesh<BufferGeometry<SimpleVertex>, MeshBasicMaterial> {
        Mesh::new(BoxGeometry::cube(1.0), MeshBasicMaterial::new(Color::RED))
    }

    #[test]
    fn a_new_group_is_empty_and_named() {
        let g = Group::new("pipes");
        assert_eq!(g.name, "pipes");
        assert!(g.is_empty());
        assert_eq!(g.transform(), &Transform::IDENTITY);
    }

    #[test]
    fn add_returns_sequential_indices() {
        let mut g = Group::new("pipes");
        assert_eq!(g.add(cube()), 0);
        assert_eq!(g.add(cube()), 1);
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn children_are_reachable_and_mutable() {
        let mut g = Group::new("pipes");
        g.add(cube());
        g.get_mut(0)
            .expect("child 0 exists")
            .transform_mut()
            .position = Vec3::Y;
        assert_eq!(g.get(0).unwrap().transform().position, Vec3::Y);
        assert_eq!(Object3D::children(&g).len(), 1);
    }

    #[test]
    fn a_group_has_nothing_of_its_own_to_draw() {
        assert!(Group::new("empty").renderable().is_none());
    }
}
