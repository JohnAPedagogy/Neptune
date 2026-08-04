//! The flat list of textured quads a frame's widgets build, handed to
//! `Frame::render_ui` and walked once by `backend::command::record_ui`.
//!
//! Kept outside `Scene` entirely: `Scene::add` is permanent ownership with no
//! `remove`, which is a straight mismatch against an immediate-mode widget
//! tree rebuilt from scratch every frame (see `neptune-imgui-plus-datgui.md`
//! §4).

use crate::materials::Texture;
use crate::math::{Aabb2d, Color};

/// One textured, tinted quad — a panel background, a slider track, a glyph,
/// all reduced to the same shape record, rich enough to be issued as a single
/// draw call by `backend::command::record_ui`.
#[derive(Debug, Clone)]
pub(crate) struct UiPrimitive {
    pub rect: Aabb2d,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: Color,
    pub texture: Texture,
}

/// Every quad one frame's widgets drew, in call order (painter's algorithm —
/// no depth attachment in the UI pass, so draw order is the only occlusion
/// rule; see `neptune-imgui-plus-datgui.md` §5).
#[derive(Debug, Clone, Default)]
pub struct UiDrawList {
    pub(crate) primitives: Vec<UiPrimitive>,
}

impl UiDrawList {
    pub(crate) fn new() -> Self {
        UiDrawList {
            primitives: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, primitive: UiPrimitive) {
        self.primitives.push(primitive);
    }

    pub(crate) fn extend(&mut self, primitives: Vec<UiPrimitive>) {
        self.primitives.extend(primitives);
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::Texture;
    use crate::math::{Aabb2d, Color, Vec2};

    fn quad() -> UiPrimitive {
        UiPrimitive {
            rect: Aabb2d::new(Vec2::ZERO, Vec2::new(10.0, 10.0)),
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            color: Color::WHITE,
            texture: Texture::white(),
        }
    }

    #[test]
    fn a_fresh_draw_list_is_empty() {
        let list = UiDrawList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn push_appends_one_primitive() {
        let mut list = UiDrawList::new();
        list.push(quad());
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn extend_appends_every_primitive() {
        let mut list = UiDrawList::new();
        list.extend(vec![quad(), quad(), quad()]);
        assert_eq!(list.len(), 3);
    }
}
