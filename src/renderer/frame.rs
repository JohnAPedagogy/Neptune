//! The borrowed handle the render-loop closure gets for one frame.

use std::path::PathBuf;

use super::clock::FrameTime;
use super::renderer::RenderState;
use crate::cameras::Camera;
use crate::core::Scene;
use crate::input::InputState;
use crate::ui::UiDrawList;

/// Everything a single frame lets you do.
///
/// `Frame` borrows the renderer's GPU state for exactly the duration of one
/// call to the render-loop closure — the lifetime `'a` is what stops it from
/// being stashed somewhere and used after the frame has been presented.
pub struct Frame<'a> {
    state: &'a mut RenderState,
    input: &'a InputState,
    time: FrameTime,
    exit_requested: &'a mut bool,
}

impl<'a> Frame<'a> {
    pub(super) fn new(
        state: &'a mut RenderState,
        input: &'a InputState,
        time: FrameTime,
        exit_requested: &'a mut bool,
    ) -> Self {
        Frame {
            state,
            input,
            time,
            exit_requested,
        }
    }

    /// Draws `scene` through `camera`.
    ///
    /// Call this at most once per frame: each call acquires, records, submits
    /// and presents a swapchain image.
    pub fn render(&mut self, scene: &Scene, camera: &dyn Camera) {
        self.state.render(scene, camera);
    }

    /// Also writes the next frame this `Frame` renders to `path` as a PNG.
    ///
    /// Call it *before* [`Frame::render`]: the capture is a copy recorded into
    /// the same command buffer as the draw calls, so it can only be arranged
    /// while the frame is still being built. A request made after `render` (or
    /// on a frame that never renders) simply carries over to the next frame
    /// that does draw.
    ///
    /// The image is the window's framebuffer at its current size, written with
    /// a fully opaque alpha channel. Missing parent directories are created.
    ///
    /// This blocks until the GPU has finished the frame, so it is a tool for
    /// screenshots and documentation, not something to call every frame.
    /// Failures (an unwritable path, a surface that cannot be copied from) are
    /// reported on stderr rather than returned — capturing a picture is never
    /// load-bearing for the program that asked for it.
    ///
    /// ```no_run
    /// # use neptune::prelude::*;
    /// # fn demo(frame: &mut Frame, scene: &Scene, camera: &dyn Camera) {
    /// frame.save_screenshot("screenshot.png");
    /// frame.render(scene, camera);
    /// # }
    /// ```
    pub fn save_screenshot(&mut self, path: impl Into<PathBuf>) {
        self.state.request_screenshot(path.into());
    }

    /// Queues `draw_list` to be drawn as a second, screen-space pass on top of
    /// the frame [`Frame::render`] is about to draw.
    ///
    /// Call this **before** [`Frame::render`], not after: like
    /// [`Frame::save_screenshot`], the UI pass is recorded into the *same*
    /// command buffer `render` builds in one shot (acquire, record, submit,
    /// present), so it has to be queued before that call runs. A request made
    /// after `render` carries over to the next frame that renders, the same as
    /// a late screenshot request does.
    ///
    /// ```no_run
    /// # use neptune::prelude::*;
    /// # fn demo(frame: &mut Frame, scene: &Scene, camera: &dyn Camera, mut ui: Ui) {
    /// let (w, h) = frame.size();
    /// let mut frame_ui = ui.begin(frame.input().mouse(), (w as f32, h as f32), Vec2::ZERO, 260.0);
    /// frame_ui.label("hello", Color::WHITE);
    /// frame.render_ui(frame_ui.finish());
    /// frame.render(scene, camera);
    /// # }
    /// ```
    pub fn render_ui(&mut self, draw_list: UiDrawList) {
        self.state.request_ui(draw_list);
    }

    /// Seconds since the previous frame — multiply your per-second velocities
    /// by this to stay frame-rate independent.
    pub fn delta_seconds(&self) -> f32 {
        self.time.delta_seconds
    }

    /// Seconds since the render loop started.
    pub fn elapsed_seconds(&self) -> f32 {
        self.time.elapsed_seconds
    }

    /// Keyboard state for this frame.
    pub fn input(&self) -> &InputState {
        self.input
    }

    /// Current framebuffer size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.state.size()
    }

    /// Framebuffer width divided by height — feed this to a camera so it
    /// tracks window resizes.
    pub fn aspect_ratio(&self) -> f32 {
        self.state.aspect_ratio()
    }

    /// Asks the render loop to stop after this frame.
    pub fn exit(&mut self) {
        *self.exit_requested = true;
    }
}
