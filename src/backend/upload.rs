//! Getting CPU-side vertex data onto the GPU, once per geometry.

use std::collections::HashMap;
use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};

use crate::geometry::{Geometry, GeometryId, SimpleVertex, Vertex};

/// The GPU-side half of one [`Geometry`].
pub(crate) struct GpuGeometry {
    pub vertex_buffer: Subbuffer<[SimpleVertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub index_count: u32,
    /// The [`Geometry::revision`] this was uploaded from; a mismatch means the
    /// CPU-side data changed and the buffers must be rebuilt.
    revision: u64,
}

/// Uploads a vertex slice into a fresh device buffer.
///
/// Generic over the vertex type, so a project defining its own layout gets the
/// same upload path. The extra `BufferContents` bound lives here in the private
/// backend rather than on the public [`Vertex`] trait, which keeps Vulkano out
/// of Neptune's public API.
pub(crate) fn upload_vertices<V>(
    allocator: &Arc<StandardMemoryAllocator>,
    vertices: &[V],
) -> Subbuffer<[V]>
where
    V: Vertex + BufferContents + Copy,
{
    upload_buffer(allocator, BufferUsage::VERTEX_BUFFER, vertices)
}

/// Uploads an index slice into a fresh device buffer.
pub(crate) fn upload_indices(
    allocator: &Arc<StandardMemoryAllocator>,
    indices: &[u32],
) -> Subbuffer<[u32]> {
    upload_buffer(allocator, BufferUsage::INDEX_BUFFER, indices)
}

fn upload_buffer<T>(
    allocator: &Arc<StandardMemoryAllocator>,
    usage: BufferUsage,
    data: &[T],
) -> Subbuffer<[T]>
where
    T: BufferContents + Copy,
{
    Buffer::from_iter(
        allocator.clone(),
        BufferCreateInfo {
            usage,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        data.iter().copied(),
    )
    .expect("failed to upload buffer to the GPU")
}

/// Keeps one set of GPU buffers per geometry identity.
///
/// Drawing the same `BoxGeometry` a thousand times uploads it once; rewriting a
/// geometry in place (see `BufferGeometry::set_mesh`) bumps its revision and
/// re-uploads into the same cache slot rather than leaking a new one.
#[derive(Default)]
pub(crate) struct GeometryCache {
    entries: HashMap<GeometryId, GpuGeometry>,
}

impl GeometryCache {
    pub(crate) fn new() -> Self {
        GeometryCache::default()
    }

    pub(crate) fn get_or_upload(
        &mut self,
        allocator: &Arc<StandardMemoryAllocator>,
        geometry: &dyn Geometry,
    ) -> Option<&GpuGeometry> {
        let id = geometry.geometry_id();
        let revision = geometry.revision();

        let stale = match self.entries.get(&id) {
            Some(entry) => entry.revision != revision,
            None => true,
        };

        if stale {
            if geometry.vertices().is_empty() || geometry.indices().is_empty() {
                self.entries.remove(&id);
                return None;
            }
            let entry = GpuGeometry {
                vertex_buffer: upload_vertices(allocator, geometry.vertices()),
                index_buffer: upload_indices(allocator, geometry.indices()),
                index_count: geometry.indices().len() as u32,
                revision,
            };
            self.entries.insert(id, entry);
        }

        self.entries.get(&id)
    }
}
