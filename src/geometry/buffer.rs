//! The generic vertex + index container every built-in geometry produces.

use std::sync::atomic::{AtomicU64, Ordering};

use super::geometry::{Geometry, GeometryId};
use super::vertex::{SimpleVertex, Vertex};

static NEXT_GEOMETRY_ID: AtomicU64 = AtomicU64::new(1);

fn next_geometry_id() -> GeometryId {
    GeometryId(NEXT_GEOMETRY_ID.fetch_add(1, Ordering::Relaxed))
}

/// An indexed triangle mesh held on the CPU.
///
/// Generic over the vertex type so a project can define its own layout; the
/// built-in shaders understand [`SimpleVertex`], and `Geometry` (the trait the
/// renderer draws through) is implemented for that instantiation.
#[derive(Debug, Clone)]
pub struct BufferGeometry<V: Vertex> {
    vertices: Vec<V>,
    indices: Vec<u32>,
    id: GeometryId,
    revision: u64,
}

impl<V: Vertex> BufferGeometry<V> {
    /// Creates a geometry from vertex and index data.
    pub fn new(vertices: Vec<V>, indices: Vec<u32>) -> Self {
        BufferGeometry {
            vertices,
            indices,
            id: next_geometry_id(),
            revision: 0,
        }
    }

    /// Creates a geometry with a trivial `0..n` index list.
    pub fn from_vertices(vertices: Vec<V>) -> Self {
        let indices = (0..vertices.len() as u32).collect();
        BufferGeometry::new(vertices, indices)
    }

    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn id(&self) -> GeometryId {
        self.id
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Replaces the mesh data in place, keeping the geometry's identity.
    ///
    /// The identity is what the renderer's buffer cache is keyed on, so a
    /// geometry that is rewritten every frame (dynamic text, a stretched bar)
    /// re-uses one cache slot instead of leaking a new one per update.
    pub fn set_mesh(&mut self, vertices: Vec<V>, indices: Vec<u32>) {
        self.vertices = vertices;
        self.indices = indices;
        self.revision = self.revision.wrapping_add(1);
    }
}

impl Geometry for BufferGeometry<SimpleVertex> {
    fn vertices(&self) -> &[SimpleVertex] {
        &self.vertices
    }

    fn indices(&self) -> &[u32] {
        &self.indices
    }

    fn geometry_id(&self) -> GeometryId {
        self.id
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32) -> SimpleVertex {
        SimpleVertex::new([x, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0])
    }

    #[test]
    fn ids_are_unique_per_geometry() {
        let a = BufferGeometry::new(vec![vertex(0.0)], vec![0]);
        let b = BufferGeometry::new(vec![vertex(0.0)], vec![0]);
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn from_vertices_builds_sequential_indices() {
        let g = BufferGeometry::from_vertices(vec![vertex(0.0), vertex(1.0), vertex(2.0)]);
        assert_eq!(g.indices(), &[0, 1, 2]);
    }

    #[test]
    fn set_mesh_keeps_id_and_bumps_revision() {
        let mut g = BufferGeometry::new(vec![vertex(0.0)], vec![0]);
        let id = g.id();
        assert_eq!(g.revision(), 0);

        g.set_mesh(vec![vertex(1.0), vertex(2.0)], vec![0, 1]);
        assert_eq!(g.id(), id);
        assert_eq!(g.revision(), 1);
        assert_eq!(BufferGeometry::vertices(&g).len(), 2);
    }
}
