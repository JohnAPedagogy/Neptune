//! The root container. Owns every object and light in the world.

use super::object3d::Object3D;
use crate::lights::Light;
use crate::math::Color;

/// The handle [`Scene::add`] hands back. It is just the object's slot index,
/// which stays valid for the life of the scene.
pub type ObjectId = usize;

/// Everything that gets drawn, plus the lights that illuminate it.
///
/// `Scene` *owns* its contents: `add` moves an object in and it lives there
/// until the scene is dropped. Access afterwards goes through
/// [`Scene::get`]/[`Scene::get_mut`], which borrow.
pub struct Scene {
    objects: Vec<Box<dyn Object3D>>,
    lights: Vec<Box<dyn Light>>,
    /// Colour the framebuffer is cleared to each frame.
    pub background: Color,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            objects: Vec::new(),
            lights: Vec::new(),
            background: Color::rgba(0.02, 0.02, 0.05, 1.0),
        }
    }

    /// Moves `object` into the scene and returns its id.
    pub fn add(&mut self, object: impl Object3D + 'static) -> ObjectId {
        self.objects.push(Box::new(object));
        self.objects.len() - 1
    }

    /// Moves a light into the scene.
    ///
    /// The built-in materials are unlit, so lights are currently inert data —
    /// the types exist and are queryable, but no shader reads them yet.
    pub fn add_light(&mut self, light: impl Light + 'static) -> usize {
        self.lights.push(Box::new(light));
        self.lights.len() - 1
    }

    pub fn get(&self, id: ObjectId) -> Option<&dyn Object3D> {
        self.objects.get(id).map(|o| o.as_ref())
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut dyn Object3D> {
        self.objects.get_mut(id).map(|o| o.as_mut())
    }

    /// Borrows an object back as its original concrete type.
    pub fn get_as<T: Object3D>(&self, id: ObjectId) -> Option<&T> {
        self.get(id)?.as_any().downcast_ref::<T>()
    }

    /// Mutably borrows an object back as its original concrete type.
    pub fn get_mut_as<T: Object3D>(&mut self, id: ObjectId) -> Option<&mut T> {
        self.get_mut(id)?.as_any_mut().downcast_mut::<T>()
    }

    /// Iterates every object of one concrete type, mutably.
    ///
    /// This is Neptune's answer to an ECS query, built from nothing but
    /// `Any` + `downcast_mut` over the `Vec<Box<dyn Object3D>>` the scene
    /// already owns.
    pub fn query_mut<T: Object3D>(&mut self) -> impl Iterator<Item = &mut T> {
        self.objects
            .iter_mut()
            .filter_map(|o| o.as_any_mut().downcast_mut::<T>())
    }

    /// Iterates every object of one concrete type.
    pub fn query<T: Object3D>(&self) -> impl Iterator<Item = &T> {
        self.objects
            .iter()
            .filter_map(|o| o.as_any().downcast_ref::<T>())
    }

    /// Every top-level object, in insertion order.
    pub fn objects(&self) -> impl Iterator<Item = &dyn Object3D> {
        self.objects.iter().map(|o| o.as_ref())
    }

    /// Every light in the scene.
    pub fn lights(&self) -> impl Iterator<Item = &dyn Light> {
        self.lights.iter().map(|l| l.as_ref())
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Drops every object and light. Ids handed out before this call are no
    /// longer meaningful.
    pub fn clear(&mut self) {
        self.objects.clear();
        self.lights.clear();
    }
}

impl Default for Scene {
    fn default() -> Self {
        Scene::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{BoxGeometry, BufferGeometry, SimpleVertex, SphereGeometry};
    use crate::lights::AmbientLight;
    use crate::materials::MeshBasicMaterial;
    use crate::math::Vec3;
    use crate::objects::Mesh;

    type BasicMesh = Mesh<BufferGeometry<SimpleVertex>, MeshBasicMaterial>;

    fn cube() -> BasicMesh {
        Mesh::new(BoxGeometry::cube(1.0), MeshBasicMaterial::default())
    }

    fn sphere() -> BasicMesh {
        Mesh::new(
            SphereGeometry::new(1.0, 8, 8),
            MeshBasicMaterial::default(),
        )
    }

    #[test]
    fn a_new_scene_is_empty() {
        let scene = Scene::new();
        assert!(scene.is_empty());
        assert_eq!(scene.len(), 0);
        assert_eq!(scene.lights().count(), 0);
    }

    #[test]
    fn add_returns_sequential_ids() {
        let mut scene = Scene::new();
        assert_eq!(scene.add(cube()), 0);
        assert_eq!(scene.add(sphere()), 1);
        assert_eq!(scene.len(), 2);
    }

    #[test]
    fn get_mut_reaches_the_transform_of_an_erased_object() {
        let mut scene = Scene::new();
        let id = scene.add(cube());
        scene
            .get_mut(id)
            .expect("just-added object exists")
            .transform_mut()
            .rotation
            .y = 1.5;
        assert_eq!(scene.get(id).unwrap().transform().rotation.y, 1.5);
    }

    #[test]
    fn get_out_of_range_is_none() {
        let scene = Scene::new();
        assert!(scene.get(7).is_none());
    }

    #[test]
    fn get_as_downcasts_back_to_the_concrete_type() {
        let mut scene = Scene::new();
        let id = scene.add(cube());
        assert!(scene.get_as::<BasicMesh>(id).is_some());
        scene
            .get_mut_as::<BasicMesh>(id)
            .expect("cube is a BasicMesh")
            .transform
            .position = Vec3::X;
        assert_eq!(scene.get(id).unwrap().transform().position, Vec3::X);
    }

    #[test]
    fn query_mut_visits_every_object_of_that_type() {
        let mut scene = Scene::new();
        scene.add(cube());
        scene.add(sphere());
        scene.add(cube());

        let mut visited = 0;
        for mesh in scene.query_mut::<BasicMesh>() {
            mesh.transform.position.y += 1.0;
            visited += 1;
        }
        assert_eq!(visited, 3);
        assert!(scene.query::<BasicMesh>().all(|m| m.transform.position.y == 1.0));
    }

    #[test]
    fn lights_are_stored_separately_from_objects() {
        let mut scene = Scene::new();
        scene.add(cube());
        scene.add_light(AmbientLight::new(Color::WHITE, 0.5));
        assert_eq!(scene.len(), 1);
        assert_eq!(scene.lights().count(), 1);
    }

    #[test]
    fn clear_drops_everything() {
        let mut scene = Scene::new();
        scene.add(cube());
        scene.add_light(AmbientLight::new(Color::WHITE, 1.0));
        scene.clear();
        assert!(scene.is_empty());
        assert_eq!(scene.lights().count(), 0);
    }
}
