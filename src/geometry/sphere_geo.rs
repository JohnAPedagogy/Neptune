//! A UV sphere, the `THREE.SphereGeometry` equivalent.

use std::f32::consts::PI;

use super::buffer::BufferGeometry;
use super::vertex::SimpleVertex;

/// Constructor namespace for a latitude/longitude sphere centred on the origin.
pub struct SphereGeometry;

// These are constructor namespaces, not types: `new` deliberately returns a
// `BufferGeometry`, exactly as `THREE.BoxGeometry` returns a buffer geometry.
#[allow(clippy::new_ret_no_self)]
impl SphereGeometry {
    /// Builds a sphere of `radius` tessellated into `width_segments` columns of
    /// longitude and `height_segments` rows of latitude.
    ///
    /// Both segment counts are clamped to a minimum of 3 and 2 respectively —
    /// below that there is no closed surface to build.
    pub fn new(
        radius: f32,
        width_segments: u32,
        height_segments: u32,
    ) -> BufferGeometry<SimpleVertex> {
        let w = width_segments.max(3);
        let h = height_segments.max(2);

        let mut vertices = Vec::with_capacity(((w + 1) * (h + 1)) as usize);
        for y in 0..=h {
            let v = y as f32 / h as f32;
            let phi = v * PI; // 0 at the north pole, PI at the south
            for x in 0..=w {
                let u = x as f32 / w as f32;
                let theta = u * 2.0 * PI;

                let normal = [-phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
                let position = [normal[0] * radius, normal[1] * radius, normal[2] * radius];
                vertices.push(SimpleVertex::new(position, normal, [u, v]));
            }
        }

        let row = w + 1;
        let mut indices = Vec::with_capacity((w * h * 6) as usize);
        for y in 0..h {
            for x in 0..w {
                let a = y * row + x;
                let b = a + 1;
                let c = a + row;
                let d = c + 1;

                // Skip the degenerate triangle at each pole, where two corners
                // of the quad collapse onto the same point.
                if y != 0 {
                    indices.extend_from_slice(&[a, c, b]);
                }
                if y != h - 1 {
                    indices.extend_from_slice(&[b, c, d]);
                }
            }
        }

        BufferGeometry::new(vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Geometry;

    #[test]
    fn vertex_grid_has_expected_size() {
        let g = SphereGeometry::new(1.0, 8, 6);
        assert_eq!(Geometry::vertices(&g).len(), (8 + 1) * (6 + 1));
    }

    #[test]
    fn every_vertex_sits_on_the_radius() {
        let g = SphereGeometry::new(2.5, 12, 8);
        for v in Geometry::vertices(&g) {
            let r = (v.position[0].powi(2) + v.position[1].powi(2) + v.position[2].powi(2)).sqrt();
            assert!((r - 2.5).abs() < 1e-4, "radius was {r}");
        }
    }

    #[test]
    fn normals_are_unit_length() {
        let g = SphereGeometry::new(3.0, 10, 10);
        for v in Geometry::vertices(&g) {
            let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
            assert!((len - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn indices_stay_in_range_and_form_triangles() {
        let g = SphereGeometry::new(1.0, 7, 5);
        let count = Geometry::vertices(&g).len() as u32;
        let indices = Geometry::indices(&g);
        assert_eq!(indices.len() % 3, 0);
        assert!(indices.iter().all(|&i| i < count));
    }

    #[test]
    fn segment_counts_are_clamped_to_a_valid_minimum() {
        let g = SphereGeometry::new(1.0, 0, 0);
        assert_eq!(Geometry::vertices(&g).len(), (3 + 1) * (2 + 1));
        assert!(!Geometry::indices(&g).is_empty());
    }
}
