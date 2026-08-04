//! Lays out a string as glyph quads directly in pixel space (Y-down, origin
//! top-left) — the UI analogue of `text::text_mesh`'s world-space `layout()`,
//! but with no sign flip: `ab_glyph`'s own bearing convention is already
//! "y growing down" (`text/font.rs:239`), which is this module's native space.

use crate::materials::Texture;
use crate::math::{Aabb2d, Color, Vec2};
use crate::text::GlyphAtlas;

use super::draw_list::UiPrimitive;

/// Builds one [`UiPrimitive`] per visible glyph of `text`, `px_height` pixels
/// tall, with `origin` as the top-left of the line.
pub(crate) fn layout_text(
    atlas: &GlyphAtlas,
    text: &str,
    origin: Vec2,
    px_height: f32,
    color: Color,
    texture: Texture,
) -> Vec<UiPrimitive> {
    let scale = px_height / atlas.line_height().max(1.0);
    let mut primitives = Vec::new();
    let mut pen_x = origin.x;

    for ch in text.chars() {
        let Some(glyph) = atlas.glyph(ch) else {
            continue;
        };

        if glyph.size.x > 0.0 && glyph.size.y > 0.0 {
            let x0 = pen_x + glyph.bearing.x * scale;
            let x1 = x0 + glyph.size.x * scale;
            let y0 = origin.y + glyph.bearing.y * scale;
            let y1 = y0 + glyph.size.y * scale;

            primitives.push(UiPrimitive {
                rect: Aabb2d::new(Vec2::new(x0, y0), Vec2::new(x1, y1)),
                uv_min: glyph.uv_min,
                uv_max: glyph.uv_max,
                color,
                texture: texture.clone(),
            });
        }

        pen_x += glyph.advance * scale;
    }

    primitives
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::Texture;
    use crate::math::{Color, Vec2};
    use crate::text::Font;

    /// Skips the test on a machine with no system font, matching
    /// `text_mesh.rs`'s existing test convention.
    fn atlas() -> Option<std::sync::Arc<crate::text::GlyphAtlas>> {
        Font::system_default().ok()?.atlas(24.0).ok()
    }

    #[test]
    fn each_visible_glyph_becomes_one_primitive() {
        let Some(atlas) = atlas() else { return };
        let primitives = layout_text(&atlas, "AB", Vec2::ZERO, 16.0, Color::WHITE, Texture::white());
        assert_eq!(primitives.len(), 2);
    }

    #[test]
    fn whitespace_advances_without_emitting_a_primitive() {
        let Some(atlas) = atlas() else { return };
        let primitives = layout_text(&atlas, "A B", Vec2::ZERO, 16.0, Color::WHITE, Texture::white());
        assert_eq!(primitives.len(), 2);
    }

    #[test]
    fn an_empty_string_produces_no_primitives() {
        let Some(atlas) = atlas() else { return };
        let primitives = layout_text(&atlas, "", Vec2::ZERO, 16.0, Color::WHITE, Texture::white());
        assert!(primitives.is_empty());
    }

    #[test]
    fn glyphs_carry_the_requested_color_and_texture() {
        let Some(atlas) = atlas() else { return };
        let texture = Texture::white();
        let primitives = layout_text(&atlas, "A", Vec2::ZERO, 16.0, Color::RED, texture.clone());
        assert_eq!(primitives[0].color, Color::RED);
        assert_eq!(primitives[0].texture.id(), texture.id());
    }

    #[test]
    fn a_later_glyph_sits_to_the_right_of_an_earlier_one() {
        let Some(atlas) = atlas() else { return };
        let primitives = layout_text(&atlas, "AB", Vec2::ZERO, 16.0, Color::WHITE, Texture::white());
        assert!(primitives[1].rect.min.x >= primitives[0].rect.min.x);
    }
}
