//! A flat quad — also the geometry every 2D sprite and text glyph draws with.

use super::buffer::BufferGeometry;
use super::vertex::SimpleVertex;

/// Constructor namespace for a quad in the XY plane facing `+Z`.
pub struct PlaneGeometry;

// These are constructor namespaces, not types: `new` deliberately returns a
// `BufferGeometry`, exactly as `THREE.BoxGeometry` returns a buffer geometry.
#[allow(clippy::new_ret_no_self)]
impl PlaneGeometry {
    /// Builds a `width` x `height` quad centred on the origin, facing `+Z`.
    ///
    /// UVs run `(0,0)` at the top-left corner to `(1,1)` at the bottom-right,
    /// matching the row order of a decoded image, so a texture applied to this
    /// quad appears the right way up.
    pub fn new(width: f32, height: f32) -> BufferGeometry<SimpleVertex> {
        PlaneGeometry::with_uv_rect(width, height, [0.0, 0.0], [1.0, 1.0])
    }

    /// Builds a quad that samples only a sub-rectangle of its texture.
    ///
    /// This is the sprite-sheet / glyph-atlas entry point: `uv_min` and
    /// `uv_max` select the region of the atlas the quad shows.
    pub fn with_uv_rect(
        width: f32,
        height: f32,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
    ) -> BufferGeometry<SimpleVertex> {
        let (hx, hy) = (width * 0.5, height * 0.5);
        let n = [0.0, 0.0, 1.0];

        let vertices = vec![
            SimpleVertex::new([-hx, -hy, 0.0], n, [uv_min[0], uv_max[1]]),
            SimpleVertex::new([hx, -hy, 0.0], n, [uv_max[0], uv_max[1]]),
            SimpleVertex::new([hx, hy, 0.0], n, [uv_max[0], uv_min[1]]),
            SimpleVertex::new([-hx, hy, 0.0], n, [uv_min[0], uv_min[1]]),
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];

        BufferGeometry::new(vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Geometry;

    #[test]
    fn plane_is_two_triangles() {
        let g = PlaneGeometry::new(1.0, 1.0);
        assert_eq!(Geometry::vertices(&g).len(), 4);
        assert_eq!(Geometry::indices(&g).len(), 6);
    }

    #[test]
    fn plane_is_flat_on_z() {
        let g = PlaneGeometry::new(3.0, 7.0);
        assert!(Geometry::vertices(&g).iter().all(|v| v.position[2] == 0.0));
        assert!(Geometry::vertices(&g).iter().all(|v| v.normal == [0.0, 0.0, 1.0]));
    }

    #[test]
    fn plane_spans_the_requested_size() {
        let g = PlaneGeometry::new(4.0, 2.0);
        let v = Geometry::vertices(&g);
        assert_eq!(v[0].position, [-2.0, -1.0, 0.0]);
        assert_eq!(v[2].position, [2.0, 1.0, 0.0]);
    }

    #[test]
    fn uv_rect_selects_a_sub_region() {
        let g = PlaneGeometry::with_uv_rect(1.0, 1.0, [0.25, 0.5], [0.75, 1.0]);
        let v = Geometry::vertices(&g);
        // bottom-left vertex samples the bottom-left of the requested rect
        assert_eq!(v[0].uv, [0.25, 1.0]);
        // top-right vertex samples the top-right of the requested rect
        assert_eq!(v[2].uv, [0.75, 0.5]);
    }
}
