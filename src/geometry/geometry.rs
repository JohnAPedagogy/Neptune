//! The trait the renderer sees when it needs vertices to draw.

use super::vertex::SimpleVertex;

/// A process-unique identity for one geometry, used as the key of the
/// renderer's GPU buffer cache so two draws of the same geometry upload once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeometryId(pub(crate) u64);

/// CPU-side vertex data the renderer can upload and draw.
///
/// Implemented for [`BufferGeometry<SimpleVertex>`](super::BufferGeometry),
/// which is what every built-in constructor (`BoxGeometry`, `SphereGeometry`,
/// `PlaneGeometry`) produces.
pub trait Geometry {
    fn vertices(&self) -> &[SimpleVertex];
    fn indices(&self) -> &[u32];

    /// Stable identity, assigned at construction.
    fn geometry_id(&self) -> GeometryId;

    /// Bumped whenever the vertex or index data changes, so the renderer knows
    /// to re-upload a geometry it has already cached. Static geometries never
    /// move off zero.
    fn revision(&self) -> u64 {
        0
    }
}
