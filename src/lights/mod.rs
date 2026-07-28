//! Light sources.

mod ambient;
mod directional;
#[allow(clippy::module_inception)]
mod light;

pub use ambient::AmbientLight;
pub use directional::DirectionalLight;
pub use light::{Light, LightKind};
