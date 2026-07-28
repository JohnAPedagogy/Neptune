//! A run of text, laid out as textured quads over a shared glyph atlas.

use std::any::Any;
use std::sync::Arc;

use super::font::GlyphAtlas;
use crate::core::{Object3D, Renderable};
use crate::geometry::{BufferGeometry, SimpleVertex};
use crate::materials::SpriteMaterial;
use crate::math::{Color, Transform};

/// A string rendered as one quad per glyph.
///
/// The quads are sized so one line of text is exactly `1.0` world unit tall,
/// which makes `transform.scale` a direct "text height in world units" knob.
/// The origin sits on the baseline at the left edge of the first glyph.
///
/// Changing the text with [`TextMesh::set_text`] rewrites the CPU-side quads in
/// place. The atlas texture is untouched, so a per-frame score update costs no
/// GPU upload beyond the new vertex buffer.
pub struct TextMesh {
    pub transform: Transform,
    pub visible: bool,
    atlas: Arc<GlyphAtlas>,
    geometry: BufferGeometry<SimpleVertex>,
    material: SpriteMaterial,
    text: String,
    width: f32,
}

impl TextMesh {
    /// Lays out `text` using `atlas`, tinted white.
    pub fn new(atlas: Arc<GlyphAtlas>, text: &str) -> Self {
        TextMesh::with_color(atlas, text, Color::WHITE)
    }

    /// Lays out `text` using `atlas`, tinted `color`.
    pub fn with_color(atlas: Arc<GlyphAtlas>, text: &str, color: Color) -> Self {
        let material = SpriteMaterial::with_tint(atlas.texture().clone(), color);
        let (vertices, indices, width) = layout(&atlas, text);

        TextMesh {
            transform: Transform::IDENTITY,
            visible: true,
            geometry: BufferGeometry::new(vertices, indices),
            material,
            text: text.to_string(),
            width,
            atlas,
        }
    }

    /// Replaces the string, rebuilding the quads. A no-op if the text is
    /// unchanged, so calling it every frame is cheap.
    pub fn set_text(&mut self, text: &str) {
        if self.text == text {
            return;
        }
        let (vertices, indices, width) = layout(&self.atlas, text);
        self.geometry.set_mesh(vertices, indices);
        self.text = text.to_string();
        self.width = width;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The tint multiplied into the glyph coverage.
    pub fn color(&self) -> Color {
        self.material.tint
    }

    pub fn set_color(&mut self, color: Color) {
        self.material.tint = color;
    }

    /// Width of the laid-out text in world units, before `transform.scale`.
    /// Offset the transform by `-width / 2.0` to centre it.
    pub fn width(&self) -> f32 {
        self.width
    }
}

/// Builds one quad per drawable glyph, in a space where one line is 1.0 tall.
fn layout(atlas: &GlyphAtlas, text: &str) -> (Vec<SimpleVertex>, Vec<u32>, f32) {
    // Atlas pixels -> world units.
    let s = 1.0 / atlas.line_height().max(1.0);
    const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut pen_x = 0.0f32;

    for ch in text.chars() {
        let Some(glyph) = atlas.glyph(ch) else {
            continue;
        };

        if glyph.size.x > 0.0 && glyph.size.y > 0.0 {
            let x0 = (pen_x + glyph.bearing.x) * s;
            let x1 = x0 + glyph.size.x * s;
            // `bearing.y` grows downward from the baseline; world Y grows up.
            let y1 = -glyph.bearing.y * s;
            let y0 = y1 - glyph.size.y * s;

            let [u0, v0] = glyph.uv_min;
            let [u1, v1] = glyph.uv_max;

            let base = vertices.len() as u32;
            vertices.extend_from_slice(&[
                SimpleVertex::new([x0, y0, 0.0], NORMAL, [u0, v1]),
                SimpleVertex::new([x1, y0, 0.0], NORMAL, [u1, v1]),
                SimpleVertex::new([x1, y1, 0.0], NORMAL, [u1, v0]),
                SimpleVertex::new([x0, y1, 0.0], NORMAL, [u0, v0]),
            ]);
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
        }

        pen_x += glyph.advance;
    }

    (vertices, indices, pen_x * s)
}

impl Object3D for TextMesh {
    fn transform(&self) -> &Transform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn renderable(&self) -> Option<Renderable<'_>> {
        Some(Renderable {
            geometry: &self.geometry,
            material: &self.material,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Font;

    /// Returns `None` on a machine with no system font, so the suite still
    /// passes there.
    fn atlas() -> Option<Arc<GlyphAtlas>> {
        Font::system_default().ok()?.atlas(32.0).ok()
    }

    #[test]
    fn each_visible_glyph_becomes_a_quad() {
        let Some(atlas) = atlas() else { return };
        let mesh = TextMesh::new(atlas, "123");
        let geometry = mesh.renderable().unwrap().geometry;
        assert_eq!(geometry.vertices().len(), 3 * 4);
        assert_eq!(geometry.indices().len(), 3 * 6);
    }

    #[test]
    fn whitespace_advances_without_emitting_a_quad() {
        let Some(atlas) = atlas() else { return };
        let mesh = TextMesh::new(atlas, "1 2");
        let geometry = mesh.renderable().unwrap().geometry;
        assert_eq!(geometry.vertices().len(), 2 * 4);
        assert!(mesh.width() > 0.0);
    }

    #[test]
    fn an_empty_string_produces_no_geometry() {
        let Some(atlas) = atlas() else { return };
        let mesh = TextMesh::new(atlas, "");
        let geometry = mesh.renderable().unwrap().geometry;
        assert!(geometry.vertices().is_empty());
        assert_eq!(mesh.width(), 0.0);
    }

    #[test]
    fn one_line_of_text_is_one_world_unit_tall() {
        let Some(atlas) = atlas() else { return };
        let mesh = TextMesh::new(atlas, "Hg");
        let geometry = mesh.renderable().unwrap().geometry;
        let top = geometry
            .vertices()
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        let bottom = geometry
            .vertices()
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MAX, f32::min);
        assert!(
            (top - bottom) > 0.3 && (top - bottom) < 1.2,
            "ascender-to-descender was {} world units",
            top - bottom
        );
    }

    #[test]
    fn set_text_rewrites_geometry_in_place() {
        let Some(atlas) = atlas() else { return };
        let mut mesh = TextMesh::new(atlas, "1");
        let id = mesh.renderable().unwrap().geometry.geometry_id();

        mesh.set_text("1234");
        let geometry = mesh.renderable().unwrap().geometry;
        assert_eq!(geometry.geometry_id(), id, "the GPU cache slot is reused");
        assert_eq!(geometry.revision(), 1, "the cache is told to re-upload");
        assert_eq!(geometry.vertices().len(), 4 * 4);
        assert_eq!(mesh.text(), "1234");
    }

    #[test]
    fn set_text_with_the_same_string_does_not_bump_the_revision() {
        let Some(atlas) = atlas() else { return };
        let mut mesh = TextMesh::new(atlas, "42");
        mesh.set_text("42");
        assert_eq!(mesh.renderable().unwrap().geometry.revision(), 0);
    }

    #[test]
    fn colour_is_a_tint_on_the_sprite_material() {
        let Some(atlas) = atlas() else { return };
        let mut mesh = TextMesh::with_color(atlas, "9", Color::RED);
        assert_eq!(mesh.color(), Color::RED);
        assert_eq!(mesh.renderable().unwrap().material.bind().color, Color::RED);

        mesh.set_color(Color::BLUE);
        assert_eq!(mesh.renderable().unwrap().material.bind().color, Color::BLUE);
    }

    #[test]
    fn a_longer_string_is_wider() {
        let Some(atlas) = atlas() else { return };
        let short = TextMesh::new(atlas.clone(), "1");
        let long = TextMesh::new(atlas, "1111");
        assert!(long.width() > short.width());
    }
}
