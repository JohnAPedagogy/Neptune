//! Keyboard and mouse input, surfaced from the private window event loop.

mod keyboard;
mod mouse;

pub use keyboard::{InputState, KeyCode};
pub use mouse::{MouseButton, MouseState};
