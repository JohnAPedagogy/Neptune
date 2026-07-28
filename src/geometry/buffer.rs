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
#[derive(Debug)]
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

/// Cloning produces an independent geometry, with a *fresh* identity.
///
/// A derived `Clone` would copy `id` and `revision` verbatim, and [`GeometryId`]
/// is the key of the renderer's GPU buffer cache — so the copy would share the
/// original's cache slot while owning its own CPU-side vertices. Edit either
/// one and both draw whatever was uploaded last; leave their revisions
/// disagreeing and the cache re-uploads every frame. Since the two halves can
/// diverge, the clone is simply a new geometry that starts out holding the same
/// mesh: new id, revision back to zero.
///
/// (`Texture` avoids the same trap the other way round, by keeping its id inside
/// an `Arc`, where cloning genuinely does mean "the same GPU upload".)
impl<V: Vertex> Clone for BufferGeometry<V> {
    fn clone(&self) -> Self {
        BufferGeometry::new(self.vertices.clone(), self.indices.clone())
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
    fn cloning_mints_a_fresh_id_and_resets_the_revision() {
        let mut original = BufferGeometry::new(vec![vertex(0.0)], vec![0]);
        original.set_mesh(vec![vertex(1.0), vertex(2.0)], vec![0, 1]);
        assert_eq!(original.revision(), 1);

        let first = original.clone();
        let second = original.clone();

        assert_ne!(
            first.id(),
            original.id(),
            "a clone must not alias the original's GPU cache slot"
        );
        assert_ne!(
            first.id(),
            second.id(),
            "nor may two clones alias each other"
        );
        assert_eq!(first.revision(), 0, "a fresh id has never been uploaded");

        // The mesh data itself is still copied faithfully.
        assert_eq!(
            BufferGeometry::vertices(&first),
            BufferGeometry::vertices(&original)
        );
        assert_eq!(first.indices(), original.indices());
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
