//! A rectangular cuboid, the `THREE.BoxGeometry` equivalent.

use super::buffer::BufferGeometry;
use super::vertex::SimpleVertex;

/// Constructor namespace for an axis-aligned box centred on the origin.
///
/// Like every built-in geometry it is a *constructor*, not a type: it hands
/// back a plain [`BufferGeometry<SimpleVertex>`].
pub struct BoxGeometry;

// These are constructor namespaces, not types: `new` deliberately returns a
// `BufferGeometry`, exactly as `THREE.BoxGeometry` returns a buffer geometry.
#[allow(clippy::new_ret_no_self)]
impl BoxGeometry {
    /// Builds a box of `width` x `height` x `depth`, centred on the origin.
    ///
    /// Each of the six faces gets its own four vertices so face normals stay
    /// flat and each face carries a full 0..1 UV square.
    pub fn new(width: f32, height: f32, depth: f32) -> BufferGeometry<SimpleVertex> {
        let (hx, hy, hz) = (width * 0.5, height * 0.5, depth * 0.5);

        // (normal, four corners in counter-clockwise order seen from outside)
        let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
            // +Z front
            (
                [0.0, 0.0, 1.0],
                [[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]],
            ),
            // -Z back
            (
                [0.0, 0.0, -1.0],
                [
                    [hx, -hy, -hz],
                    [-hx, -hy, -hz],
                    [-hx, hy, -hz],
                    [hx, hy, -hz],
                ],
            ),
            // +X right
            (
                [1.0, 0.0, 0.0],
                [[hx, -hy, hz], [hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz]],
            ),
            // -X left
            (
                [-1.0, 0.0, 0.0],
                [
                    [-hx, -hy, -hz],
                    [-hx, -hy, hz],
                    [-hx, hy, hz],
                    [-hx, hy, -hz],
                ],
            ),
            // +Y top
            (
                [0.0, 1.0, 0.0],
                [[-hx, hy, hz], [hx, hy, hz], [hx, hy, -hz], [-hx, hy, -hz]],
            ),
            // -Y bottom
            (
                [0.0, -1.0, 0.0],
                [
                    [-hx, -hy, -hz],
                    [hx, -hy, -hz],
                    [hx, -hy, hz],
                    [-hx, -hy, hz],
                ],
            ),
        ];

        // Bottom-left, bottom-right, top-right, top-left of the texture square.
        const UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (normal, corners) in faces {
            let base = vertices.len() as u32;
            for (corner, uv) in corners.into_iter().zip(UVS) {
                vertices.push(SimpleVertex::new(corner, normal, uv));
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
        }

        BufferGeometry::new(vertices, indices)
    }

    /// Builds a cube with equal sides.
    pub fn cube(size: f32) -> BufferGeometry<SimpleVertex> {
        BoxGeometry::new(size, size, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Geometry;

    #[test]
    fn box_has_six_quads() {
        let g = BoxGeometry::new(1.0, 1.0, 1.0);
        assert_eq!(Geometry::vertices(&g).len(), 24);
        assert_eq!(Geometry::indices(&g).len(), 36);
    }

    #[test]
    fn indices_stay_in_range() {
        let g = BoxGeometry::new(2.0, 3.0, 4.0);
        let count = Geometry::vertices(&g).len() as u32;
        assert!(Geometry::indices(&g).iter().all(|&i| i < count));
    }

    #[test]
    fn dimensions_are_centred_half_extents() {
        let g = BoxGeometry::new(2.0, 4.0, 6.0);
        let xs: Vec<f32> = Geometry::vertices(&g)
            .iter()
            .map(|v| v.position[0])
            .collect();
        let ys: Vec<f32> = Geometry::vertices(&g)
            .iter()
            .map(|v| v.position[1])
            .collect();
        let zs: Vec<f32> = Geometry::vertices(&g)
            .iter()
            .map(|v| v.position[2])
            .collect();
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 1.0);
        assert_eq!(ys.iter().cloned().fold(f32::MIN, f32::max), 2.0);
        assert_eq!(zs.iter().cloned().fold(f32::MIN, f32::max), 3.0);
        assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), -1.0);
        assert_eq!(ys.iter().cloned().fold(f32::MAX, f32::min), -2.0);
        assert_eq!(zs.iter().cloned().fold(f32::MAX, f32::min), -3.0);
    }

    #[test]
    fn every_normal_is_unit_length() {
        let g = BoxGeometry::cube(1.0);
        for v in Geometry::vertices(&g) {
            let len =
                (v.normal[0] * v.normal[0] + v.normal[1] * v.normal[1] + v.normal[2] * v.normal[2])
                    .sqrt();
            assert!((len - 1.0).abs() < 1e-6);
        }
    }
}
