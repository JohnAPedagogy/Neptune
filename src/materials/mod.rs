//! How surfaces are shaded.

mod basic;
#[allow(clippy::module_inception)]
mod material;
mod texture;
mod textured;

pub use basic::MeshBasicMaterial;
pub use material::{Material, MaterialBinding, MaterialId};
pub use texture::{Texture, TextureError, TextureId};
pub use textured::SpriteMaterial;
