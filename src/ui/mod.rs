//! An immediate-mode, dat.gui-flavoured widget layer, drawn as a second,
//! screen-space render pass. See `neptune-imgui-plus-datgui.md` for the
//! design rationale.
//!
//! ```no_run
//! use neptune::prelude::*;
//!
//! # fn demo(ui: &mut Ui, mouse: &MouseState, screen: (f32, f32), speed: &mut f32) {
//! let mut frame = ui.begin(mouse, screen, Vec2::ZERO, 260.0);
//! frame.slider("Speed", speed, 0.0..=5.0);
//! let draw_list = frame.finish();
//! # let _ = draw_list;
//! # }
//! ```

pub(crate) mod context;
pub(crate) mod draw_list;
pub(crate) mod layout;
pub(crate) mod text;
pub(crate) mod widgets;

pub use context::{TextStyle, Ui, UiFrame};
pub use draw_list::UiDrawList;
pub use widgets::Response;
