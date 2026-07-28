//! A one-shot, environment-variable-driven screenshot trigger, shared by the
//! examples via `#[path = "common/capture.rs"] mod capture;`.
//!
//! The examples are interactive windows, so without this there is no way to
//! grab a frame at a chosen moment from a script or a CI job — someone has to
//! sit there and press a key at the right instant. Setting `NEPTUNE_SCREENSHOT`
//! to an output path makes an example draw its usual frames, save the first one
//! at or after `NEPTUNE_SCREENSHOT_AFTER` seconds (default 2), and then quit.
//!
//! ```text
//! NEPTUNE_SCREENSHOT=out.png NEPTUNE_SCREENSHOT_AFTER=1.5 \
//!     cargo run --example hello_cube
//! ```
//!
//! With neither variable set every method here is inert, so the examples behave
//! exactly as they always have when run by hand.
//!
//! Files under `examples/` subdirectories are not auto-discovered as examples by
//! Cargo, so this one is only ever compiled as part of an example that includes
//! it.

use std::path::PathBuf;

use neptune::prelude::Frame;

/// How long to let an example run before capturing, when
/// `NEPTUNE_SCREENSHOT_AFTER` is unset. Long enough for a window to be mapped,
/// the swapchain to settle and any startup stall to wash out of the clock.
const DEFAULT_DELAY_SECONDS: f32 = 2.0;

/// The capture request an example was launched with, if any.
pub struct Capture {
    path: Option<PathBuf>,
    after_seconds: f32,
    taken: bool,
}

impl Capture {
    /// Reads `NEPTUNE_SCREENSHOT` and `NEPTUNE_SCREENSHOT_AFTER`.
    ///
    /// An unset (or empty) `NEPTUNE_SCREENSHOT` yields an inert `Capture`; an
    /// unparseable `NEPTUNE_SCREENSHOT_AFTER` falls back to the default delay.
    pub fn from_env() -> Self {
        let path = std::env::var("NEPTUNE_SCREENSHOT")
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);

        let after_seconds = std::env::var("NEPTUNE_SCREENSHOT_AFTER")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(DEFAULT_DELAY_SECONDS);

        Capture {
            path,
            after_seconds,
            taken: false,
        }
    }

    /// Arranges the capture once the deadline has passed, and asks the example
    /// to exit on the frame after.
    ///
    /// Call it immediately before `frame.render(..)`: `save_screenshot` marks
    /// the frame that `render` is about to draw, so the saved image is the
    /// finished one, complete with everything the closure just updated.
    ///
    /// The exit is deferred to the following frame rather than fired alongside
    /// the request. That costs one extra frame and buys a free retry: if the
    /// requested frame is the one where the swapchain goes stale and `render`
    /// bails out early, the request is still pending and the next frame — which
    /// also renders, since the loop only acts on the exit after the closure
    /// returns — captures it instead.
    pub fn update(&mut self, frame: &mut Frame) {
        let Some(path) = self.path.as_ref() else {
            return;
        };

        if self.taken {
            frame.exit();
        } else if frame.elapsed_seconds() >= self.after_seconds {
            frame.save_screenshot(path);
            self.taken = true;
        }
    }
}
