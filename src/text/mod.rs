//! Text rendering: fonts, glyph atlases, and the meshes that draw them.

mod font;
mod text_mesh;

pub use font::{Font, FontError, Glyph, GlyphAtlas};
pub use text_mesh::TextMesh;
