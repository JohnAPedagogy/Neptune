//! One-shot, environment-variable-driven screenshot trigger.
//!
//! Setting `NEPTUNE_SCREENSHOT` to an output path makes the program draw its
//! usual frames, save the first one at or after `NEPTUNE_SCREENSHOT_AFTER`
//! seconds (default 2), then quit. With neither variable set, every method
//! here is inert.

use std::path::PathBuf;

use neptune::prelude::Frame;

const DEFAULT_DELAY_SECONDS: f32 = 2.0;

pub struct Capture {
    path: Option<PathBuf>,
    after_seconds: f32,
    taken: bool,
}

impl Capture {
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
