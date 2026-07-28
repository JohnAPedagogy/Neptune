//! Viewpoints and projections.

#[allow(clippy::module_inception)]
mod camera;
mod orthographic;
mod perspective;

pub use camera::Camera;
pub use orthographic::OrthographicCamera;
pub use perspective::PerspectiveCamera;
