//! Vertex data: the layout, the container, and the built-in shape constructors.

mod box_geo;
mod buffer;
#[allow(clippy::module_inception)]
mod geometry;
mod plane_geo;
mod sphere_geo;
mod vertex;

pub use box_geo::BoxGeometry;
pub use buffer::BufferGeometry;
pub use geometry::{Geometry, GeometryId};
pub use plane_geo::PlaneGeometry;
pub use sphere_geo::SphereGeometry;
pub use vertex::{SimpleVertex, Vertex};
