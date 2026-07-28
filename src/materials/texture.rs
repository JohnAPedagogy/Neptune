//! CPU-side texture data, decoded but not yet uploaded.
//!
//! A `Texture` is a *handle to pixels*, not a GPU resource. The renderer
//! uploads it the first time it is drawn and caches the result under the
//! texture's [`TextureId`], so cloning a `Texture` around the scene is cheap
//! and never duplicates GPU memory.

use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A process-unique identity for one texture; the key of the renderer's GPU
/// texture cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(pub(crate) u64);

static NEXT_TEXTURE_ID: AtomicU64 = AtomicU64::new(1);

/// Why a texture could not be loaded.
#[derive(Debug)]
pub enum TextureError {
    /// The image file could not be read or decoded.
    Decode(image::ImageError),
    /// The supplied pixel buffer does not match `width * height * 4`.
    SizeMismatch { expected: usize, got: usize },
}

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureError::Decode(err) => write!(f, "failed to decode image: {err}"),
            TextureError::SizeMismatch { expected, got } => write!(
                f,
                "pixel buffer has {got} bytes but {expected} were expected for the given size"
            ),
        }
    }
}

impl std::error::Error for TextureError {}

impl From<image::ImageError> for TextureError {
    fn from(err: image::ImageError) -> Self {
        TextureError::Decode(err)
    }
}

#[derive(Debug)]
struct TextureData {
    id: TextureId,
    width: u32,
    height: u32,
    /// Tightly packed, row-major, 8-bit RGBA.
    rgba: Vec<u8>,
}

/// Decoded RGBA8 image data, cheap to clone.
#[derive(Debug, Clone)]
pub struct Texture {
    data: Arc<TextureData>,
}

impl Texture {
    /// Wraps a pre-decoded, tightly packed RGBA8 buffer.
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, TextureError> {
        let expected = width as usize * height as usize * 4;
        if rgba.len() != expected {
            return Err(TextureError::SizeMismatch {
                expected,
                got: rgba.len(),
            });
        }
        Ok(Texture {
            data: Arc::new(TextureData {
                id: TextureId(NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed)),
                width,
                height,
                rgba,
            }),
        })
    }

    /// Decodes an image file (PNG, JPEG, ... — whatever the `image` crate
    /// supports) into RGBA8.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, TextureError> {
        let decoded = image::open(path)?.into_rgba8();
        let (width, height) = decoded.dimensions();
        Texture::from_rgba8(width, height, decoded.into_raw())
    }

    /// Decodes an image already in memory — e.g. one embedded with
    /// `include_bytes!`.
    pub fn from_encoded_bytes(bytes: &[u8]) -> Result<Self, TextureError> {
        let decoded = image::load_from_memory(bytes)?.into_rgba8();
        let (width, height) = decoded.dimensions();
        Texture::from_rgba8(width, height, decoded.into_raw())
    }

    /// A 1x1 opaque white texture, useful as a neutral stand-in.
    pub fn white() -> Self {
        Texture::from_rgba8(1, 1, vec![255, 255, 255, 255])
            .expect("1x1 white texture is always valid")
    }

    pub fn id(&self) -> TextureId {
        self.data.id
    }

    pub fn width(&self) -> u32 {
        self.data.width
    }

    pub fn height(&self) -> u32 {
        self.data.height
    }

    /// Width divided by height. Handy for sizing a sprite quad.
    pub fn aspect_ratio(&self) -> f32 {
        self.data.width as f32 / self.data.height.max(1) as f32
    }

    /// The raw RGBA8 bytes, as the upload path sees them.
    pub fn rgba(&self) -> &[u8] {
        &self.data.rgba
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rgba8_accepts_a_correctly_sized_buffer() {
        let t = Texture::from_rgba8(2, 2, vec![0u8; 16]).expect("2x2 RGBA is 16 bytes");
        assert_eq!(t.width(), 2);
        assert_eq!(t.height(), 2);
        assert_eq!(t.rgba().len(), 16);
    }

    #[test]
    fn from_rgba8_rejects_a_short_buffer() {
        let err = Texture::from_rgba8(2, 2, vec![0u8; 15]).unwrap_err();
        assert!(matches!(
            err,
            TextureError::SizeMismatch {
                expected: 16,
                got: 15
            }
        ));
    }

    #[test]
    fn ids_are_unique_and_survive_cloning() {
        let a = Texture::white();
        let b = Texture::white();
        assert_ne!(a.id(), b.id());
        assert_eq!(a.clone().id(), a.id());
    }

    #[test]
    fn aspect_ratio_is_width_over_height() {
        let t = Texture::from_rgba8(4, 2, vec![0u8; 32]).unwrap();
        assert_eq!(t.aspect_ratio(), 2.0);
    }
}
