//! Textured, alpha-blended surfaces — sprites, and the glyph quads `text/`
//! builds on.
//!
//! Design note: the P1 "textured material" gap is closed with a *separate*
//! `SpriteMaterial` rather than an optional texture on
//! [`MeshBasicMaterial`](super::MeshBasicMaterial). The two need genuinely
//! different pipelines (alpha blending on, depth writes off, one extra
//! descriptor set), so keeping them apart avoids an `Option` that silently
//! changes the pipeline a material compiles to.

use std::path::Path;

use super::material::{Material, MaterialBinding, MaterialId, MaterialInstanceId};
use super::texture::{Texture, TextureError};
use crate::math::Color;

/// Draws a texture, multiplied by a tint colour, with alpha blending.
#[derive(Debug, Clone)]
pub struct SpriteMaterial {
    pub texture: Texture,
    /// Multiplied into the sampled texel. `Color::WHITE` leaves the texture
    /// untouched; a white glyph atlas tinted here is how coloured text works.
    pub tint: Color,
    instance_id: MaterialInstanceId,
}

impl SpriteMaterial {
    /// Wraps an already-decoded texture, untinted.
    pub fn new(texture: Texture) -> Self {
        SpriteMaterial::with_tint(texture, Color::WHITE)
    }

    /// Wraps a texture and multiplies it by `tint`.
    pub fn with_tint(texture: Texture, tint: Color) -> Self {
        SpriteMaterial {
            texture,
            tint,
            instance_id: MaterialInstanceId::next(),
        }
    }

    /// Decodes an image file and wraps it.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, TextureError> {
        Ok(SpriteMaterial::new(Texture::from_file(path)?))
    }

    /// The aspect ratio of the underlying texture, so a caller can size a quad
    /// without distorting the sprite.
    pub fn aspect_ratio(&self) -> f32 {
        self.texture.aspect_ratio()
    }
}

impl Material for SpriteMaterial {
    fn material_id(&self) -> MaterialId {
        MaterialId::Sprite
    }

    fn instance_id(&self) -> MaterialInstanceId {
        self.instance_id
    }

    fn bind(&self) -> MaterialBinding<'_> {
        MaterialBinding {
            color: self.tint,
            texture: Some(&self.texture),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_sprite_pipeline_and_its_texture() {
        let texture = Texture::white();
        let m = SpriteMaterial::new(texture.clone());
        assert_eq!(m.material_id(), MaterialId::Sprite);
        let binding = m.bind();
        assert_eq!(
            binding.texture.expect("sprite always has a texture").id(),
            texture.id()
        );
        assert_eq!(binding.color, Color::WHITE);
    }

    #[test]
    fn tint_is_reported_as_the_bind_colour() {
        let m = SpriteMaterial::with_tint(Texture::white(), Color::RED);
        assert_eq!(m.bind().color, Color::RED);
    }

    #[test]
    fn aspect_ratio_comes_from_the_texture() {
        let texture = Texture::from_rgba8(8, 2, vec![0u8; 8 * 2 * 4]).unwrap();
        assert_eq!(SpriteMaterial::new(texture).aspect_ratio(), 4.0);
    }
}
