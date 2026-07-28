//! TTF loading and glyph-atlas rasterisation, on top of `ab_glyph`.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ab_glyph::{Font as AbFont, FontVec, PxScale, ScaleFont};

use crate::materials::{Texture, TextureError};
use crate::math::Vec2;

/// Widest atlas row before packing wraps to the next line.
const ATLAS_MAX_WIDTH: u32 = 512;
/// Transparent gutter between glyphs, so linear filtering cannot bleed one
/// glyph's edge into its neighbour.
const GLYPH_PADDING: u32 = 1;
/// The characters an atlas covers: printable 7-bit ASCII.
const ATLAS_CHARS: std::ops::RangeInclusive<u8> = 32..=126;

/// Why a font could not be loaded.
#[derive(Debug)]
pub enum FontError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The bytes are not a font this parser understands.
    InvalidFont,
    /// No font was found at any of the locations searched.
    NotFound,
    /// The rasterised atlas could not be turned into a texture.
    Texture(TextureError),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::Io(err) => write!(f, "failed to read font file: {err}"),
            FontError::InvalidFont => write!(f, "the data is not a valid TrueType font"),
            FontError::NotFound => write!(f, "no system font found at any known location"),
            FontError::Texture(err) => write!(f, "failed to build the glyph atlas texture: {err}"),
        }
    }
}

impl std::error::Error for FontError {}

impl From<std::io::Error> for FontError {
    fn from(err: std::io::Error) -> Self {
        FontError::Io(err)
    }
}

/// Where [`Font::system_default`] looks, in order. Neptune bundles no font of
/// its own — shipping one would mean shipping its licence — so a system face
/// is used instead, and any project that cares which face it gets should call
/// [`Font::from_file`] with a font it ships itself.
const SYSTEM_FONT_PATHS: &[&str] = &[
    // Windows
    r"C:\Windows\Fonts\consola.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    r"C:\Windows\Fonts\segoeui.ttf",
    // Linux
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    // macOS
    "/Library/Fonts/Arial.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
];

/// A loaded typeface.
///
/// Loading a font is not the same as being able to draw it: call
/// [`Font::atlas`] to rasterise it at a specific pixel size, once, into a
/// texture that every [`TextMesh`](super::TextMesh) then shares.
pub struct Font {
    inner: FontVec,
}

impl Font {
    /// Parses a font from bytes already in memory — e.g. `include_bytes!`.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, FontError> {
        FontVec::try_from_vec(bytes)
            .map(|inner| Font { inner })
            .map_err(|_| FontError::InvalidFont)
    }

    /// Loads a `.ttf`/`.otf` file from disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, FontError> {
        Font::from_bytes(std::fs::read(path)?)
    }

    /// Loads the first font found at a known system location.
    ///
    /// Which face you get is platform dependent — Consolas on Windows,
    /// DejaVu Sans on most Linux distributions, Arial on macOS. Use it for
    /// debug overlays and prototypes; ship your own font for anything whose
    /// appearance matters.
    pub fn system_default() -> Result<Self, FontError> {
        for candidate in SYSTEM_FONT_PATHS {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                return Font::from_file(path);
            }
        }
        Err(FontError::NotFound)
    }

    /// Rasterises printable ASCII at `px` pixels into one texture atlas.
    ///
    /// Do this once and share the result: every [`TextMesh`](super::TextMesh)
    /// built from the same atlas draws from the same GPU texture, so changing
    /// a score every frame rebuilds only CPU-side quads.
    pub fn atlas(&self, px: f32) -> Result<Arc<GlyphAtlas>, FontError> {
        let px = px.max(1.0);
        let scale = PxScale::from(px);
        let scaled = self.inner.as_scaled(scale);
        let line_height = scaled.height() + scaled.line_gap();

        // Pass 1: rasterise every glyph on its own.
        let mut rasters = Vec::new();
        for byte in ATLAS_CHARS {
            let ch = byte as char;
            let id = self.inner.glyph_id(ch);
            let advance = scaled.h_advance(id);

            let Some(outlined) = self.inner.outline_glyph(id.with_scale(scale)) else {
                // Whitespace and other glyphs with no outline still advance.
                rasters.push(Raster {
                    ch,
                    width: 0,
                    height: 0,
                    bearing: Vec2::ZERO,
                    advance,
                    coverage: Vec::new(),
                });
                continue;
            };

            let bounds = outlined.px_bounds();
            let width = bounds.width().ceil().max(0.0) as u32;
            let height = bounds.height().ceil().max(0.0) as u32;
            let mut coverage = vec![0.0f32; (width * height) as usize];
            outlined.draw(|x, y, c| {
                if x < width && y < height {
                    coverage[(y * width + x) as usize] = c;
                }
            });

            rasters.push(Raster {
                ch,
                width,
                height,
                // px_bounds is relative to the pen position, y growing down.
                bearing: Vec2::new(bounds.min.x, bounds.min.y),
                advance,
                coverage,
            });
        }

        // Pass 2: pack the rasters into rows.
        let mut placements = Vec::with_capacity(rasters.len());
        let (mut pen_x, mut pen_y, mut row_height, mut atlas_width) =
            (GLYPH_PADDING, GLYPH_PADDING, 0u32, 0u32);
        for raster in &rasters {
            if pen_x + raster.width + GLYPH_PADDING > ATLAS_MAX_WIDTH && pen_x > GLYPH_PADDING {
                pen_x = GLYPH_PADDING;
                pen_y += row_height + GLYPH_PADDING;
                row_height = 0;
            }
            placements.push((pen_x, pen_y));
            pen_x += raster.width + GLYPH_PADDING;
            row_height = row_height.max(raster.height);
            atlas_width = atlas_width.max(pen_x);
        }
        let atlas_width = atlas_width.max(1);
        let atlas_height = (pen_y + row_height + GLYPH_PADDING).max(1);

        // Pass 3: blit coverage into a white RGBA image, alpha = coverage.
        let mut rgba = vec![0u8; (atlas_width * atlas_height * 4) as usize];
        let mut glyphs = HashMap::with_capacity(rasters.len());
        for (raster, (ox, oy)) in rasters.iter().zip(placements) {
            for y in 0..raster.height {
                for x in 0..raster.width {
                    let alpha = (raster.coverage[(y * raster.width + x) as usize] * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    let i = (((oy + y) * atlas_width + ox + x) * 4) as usize;
                    rgba[i] = 255;
                    rgba[i + 1] = 255;
                    rgba[i + 2] = 255;
                    rgba[i + 3] = alpha;
                }
            }

            glyphs.insert(
                raster.ch,
                Glyph {
                    uv_min: [
                        ox as f32 / atlas_width as f32,
                        oy as f32 / atlas_height as f32,
                    ],
                    uv_max: [
                        (ox + raster.width) as f32 / atlas_width as f32,
                        (oy + raster.height) as f32 / atlas_height as f32,
                    ],
                    size: Vec2::new(raster.width as f32, raster.height as f32),
                    bearing: raster.bearing,
                    advance: raster.advance,
                },
            );
        }

        let texture = Texture::from_rgba8(atlas_width, atlas_height, rgba)
            .map_err(FontError::Texture)?;

        Ok(Arc::new(GlyphAtlas {
            texture,
            glyphs,
            line_height,
            px,
        }))
    }
}

struct Raster {
    ch: char,
    width: u32,
    height: u32,
    bearing: Vec2,
    advance: f32,
    coverage: Vec<f32>,
}

/// Where one character lives in a [`GlyphAtlas`], and how to lay it out.
///
/// All measurements are in atlas pixels; `bearing` is the offset from the pen
/// position to the glyph's top-left corner, with `y` growing downward, as
/// typography conventionally measures it.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub size: Vec2,
    pub bearing: Vec2,
    pub advance: f32,
}

/// Every printable ASCII character of one font at one size, rasterised into a
/// single texture.
pub struct GlyphAtlas {
    texture: Texture,
    glyphs: HashMap<char, Glyph>,
    line_height: f32,
    px: f32,
}

impl GlyphAtlas {
    /// The atlas image, ready to hand to a
    /// [`SpriteMaterial`](crate::materials::SpriteMaterial).
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Metrics for one character, if the atlas covers it.
    pub fn glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(&ch)
    }

    /// Baseline-to-baseline distance, in atlas pixels.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// The pixel size this atlas was rasterised at.
    pub fn px(&self) -> f32 {
        self.px
    }

    /// Width of `text` in atlas pixels, ignoring line breaks.
    pub fn measure(&self, text: &str) -> f32 {
        text.chars()
            .filter_map(|ch| self.glyphs.get(&ch))
            .map(|g| g.advance)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skips the test rather than failing it on a machine with no system font.
    fn system_font() -> Option<Font> {
        Font::system_default().ok()
    }

    #[test]
    fn invalid_bytes_are_rejected() {
        assert!(matches!(
            Font::from_bytes(vec![0, 1, 2, 3]),
            Err(FontError::InvalidFont)
        ));
    }

    #[test]
    fn a_missing_file_reports_io() {
        assert!(matches!(
            Font::from_file("no-such-font-846213.ttf"),
            Err(FontError::Io(_))
        ));
    }

    #[test]
    fn an_atlas_covers_printable_ascii() {
        let Some(font) = system_font() else {
            return;
        };
        let atlas = font.atlas(32.0).expect("atlas rasterisation succeeds");

        assert!(atlas.glyph('A').is_some());
        assert!(atlas.glyph('~').is_some());
        assert!(atlas.glyph(' ').is_some());
        assert!(atlas.glyph('\u{4e2d}').is_none());
        assert_eq!(atlas.px(), 32.0);
        assert!(atlas.line_height() > 0.0);
    }

    #[test]
    fn glyph_uvs_stay_inside_the_atlas() {
        let Some(font) = system_font() else {
            return;
        };
        let atlas = font.atlas(24.0).expect("atlas rasterisation succeeds");
        for byte in 32u8..=126 {
            let glyph = atlas.glyph(byte as char).expect("ASCII is covered");
            assert!(glyph.uv_min[0] >= 0.0 && glyph.uv_max[0] <= 1.0);
            assert!(glyph.uv_min[1] >= 0.0 && glyph.uv_max[1] <= 1.0);
            assert!(glyph.uv_min[0] <= glyph.uv_max[0]);
            assert!(glyph.uv_min[1] <= glyph.uv_max[1]);
        }
    }

    #[test]
    fn whitespace_advances_but_draws_nothing() {
        let Some(font) = system_font() else {
            return;
        };
        let atlas = font.atlas(24.0).expect("atlas rasterisation succeeds");
        let space = atlas.glyph(' ').expect("space is covered");
        assert!(space.advance > 0.0);
        assert_eq!(space.size, Vec2::ZERO);
    }

    #[test]
    fn measuring_a_longer_string_gives_a_wider_result() {
        let Some(font) = system_font() else {
            return;
        };
        let atlas = font.atlas(24.0).expect("atlas rasterisation succeeds");
        assert!(atlas.measure("1234") > atlas.measure("12"));
        assert_eq!(atlas.measure(""), 0.0);
    }

    #[test]
    fn the_atlas_texture_has_pixels() {
        let Some(font) = system_font() else {
            return;
        };
        let atlas = font.atlas(16.0).expect("atlas rasterisation succeeds");
        let texture = atlas.texture();
        assert!(texture.width() > 0 && texture.height() > 0);
        assert!(
            texture.rgba().chunks_exact(4).any(|px| px[3] > 0),
            "a rasterised atlas must contain at least one opaque texel"
        );
    }
}
