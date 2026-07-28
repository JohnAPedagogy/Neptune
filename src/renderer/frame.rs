//! The borrowed handle the render-loop closure gets for one frame.

use super::renderer::RenderState;
use crate::cameras::Camera;
use crate::core::Scene;
use crate::input::InputState;

/// Everything a single frame lets you do.
///
/// `Frame` borrows the renderer's GPU state for exactly the duration of one
/// call to the render-loop closure — the lifetime `'a` is what stops it from
/// being stashed somewhere and used after the frame has been presented.
pub struct Frame<'a> {
    state: &'a mut RenderState,
    input: &'a InputState,
    delta_seconds: f32,
    elapsed_seconds: f32,
    exit_requested: &'a mut bool,
}

impl<'a> Frame<'a> {
    pub(super) fn new(
        state: &'a mut RenderState,
        input: &'a InputState,
        delta_seconds: f32,
        elapsed_seconds: f32,
        exit_requested: &'a mut bool,
    ) -> Self {
        Frame {
            state,
            input,
            delta_seconds,
            elapsed_seconds,
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

    /// Seconds since the previous frame — multiply your per-second velocities
    /// by this to stay frame-rate independent.
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// Seconds since the render loop started.
    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
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
