//! Vertex layouts that can be uploaded to a GPU vertex buffer.

use bytemuck::{Pod, Zeroable};
use vulkano::pipeline::graphics::vertex_input::Vertex as VulkanoVertex;

/// Anything that can live in a Neptune vertex buffer.
///
/// The bounds are what the upload path actually needs: `Pod` so the CPU-side
/// slice can be reinterpreted as bytes, and `Send + Sync + 'static` so a
/// geometry can be shared across threads (Ch 30+).
pub trait Vertex: Pod + Send + Sync + 'static {
    /// Size of one vertex in bytes — the buffer stride.
    fn stride() -> usize {
        size_of::<Self>()
    }
}

/// The vertex format every built-in geometry and shader in Neptune uses:
/// position, normal, and a UV coordinate.
///
/// `#[repr(C)]` plus `Pod`/`Zeroable` make the byte layout well-defined (and
/// are what makes this uploadable, via Vulkano's blanket `BufferContents` impl
/// for `Pod` types); the `#[format(..)]` attributes are what Vulkano reads to
/// build the pipeline's vertex input state.
#[derive(VulkanoVertex, Pod, Zeroable, Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct SimpleVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
}

impl SimpleVertex {
    pub const fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        SimpleVertex {
            position,
            normal,
            uv,
        }
    }
}

impl Vertex for SimpleVertex {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_vertex_has_no_padding() {
        // 3 + 3 + 2 floats. If this ever gains padding, `bytemuck::cast_slice`
        // in the upload path would start shipping uninitialised bytes.
        assert_eq!(SimpleVertex::stride(), 8 * size_of::<f32>());
    }

    #[test]
    fn simple_vertex_casts_to_bytes() {
        let v = [SimpleVertex::new(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [0.5, 0.5],
        )];
        let bytes: &[u8] = bytemuck::cast_slice(&v);
        assert_eq!(bytes.len(), SimpleVertex::stride());
    }
}
