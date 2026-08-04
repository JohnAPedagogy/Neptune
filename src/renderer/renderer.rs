//! The window, the event loop, and the per-frame GPU state behind them.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo, PrimaryAutoCommandBuffer,
    RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo,
};
use vulkano::instance::Instance;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::{self, Surface, SwapchainPresentInfo};
use vulkano::sync::{self, GpuFuture};
use vulkano::{Validated, VulkanError};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use super::clock::FrameClock;
use super::frame::Frame;
use crate::backend::command::{RenderCaches, record_scene, record_ui};
use crate::backend::context::{VulkanContext, create_instance};
use crate::backend::pass::{create_render_pass, create_ui_render_pass};
use crate::backend::screenshot::{self, ScreenshotError};
use crate::backend::surface::{SurfaceState, preferred_surface_format};
use crate::cameras::{Camera, OrthographicCamera};
use crate::core::Scene;
use crate::input::InputState;
use crate::ui::UiDrawList;

/// How the window is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererOptions {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

impl Default for RendererOptions {
    fn default() -> Self {
        RendererOptions {
            width: 1280,
            height: 720,
            title: "Neptune",
        }
    }
}

/// Owns the window, the GPU, and the frame loop.
///
/// Nothing about `Renderer`'s public surface mentions Vulkan: you construct it
/// with plain window options and drive it with a closure.
pub struct Renderer {
    options: RendererOptions,
    instance: Arc<Instance>,
    /// Taken by [`Renderer::render_loop`], which consumes the loop.
    event_loop: Option<EventLoop<()>>,
}

impl Renderer {
    /// Creates the Vulkan instance and the event loop the window will live on.
    ///
    /// The window itself is not created until [`Renderer::render_loop`] runs —
    /// winit only hands out a window from inside its `resumed` callback.
    pub fn new(options: RendererOptions) -> Self {
        let event_loop = EventLoop::new().expect("failed to create the OS event loop");
        event_loop.set_control_flow(ControlFlow::Poll);
        let instance = create_instance(&event_loop);

        Renderer {
            options,
            instance,
            event_loop: Some(event_loop),
        }
    }

    /// Runs the frame loop, calling `callback` once per frame.
    ///
    /// This blocks until the window closes or the closure calls
    /// [`Frame::exit`]. The closure is `FnMut`, so it owns whatever game state
    /// it captures.
    ///
    /// Can only be called once per `Renderer`; the event loop is consumed.
    pub fn render_loop<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Frame),
    {
        let event_loop = self
            .event_loop
            .take()
            .expect("render_loop may only be called once per Renderer");

        let mut app = App {
            instance: self.instance.clone(),
            options: self.options,
            state: None,
            callback,
            input: InputState::new(),
            clock: FrameClock::new(Instant::now()),
        };

        event_loop
            .run_app(&mut app)
            .expect("the OS event loop exited with an error");
    }
}

/// Everything that only exists once there is a window: the device, the
/// swapchain, the GPU caches, and the fence chain.
pub(super) struct RenderState {
    window: Arc<Window>,
    ctx: VulkanContext,
    render_pass: Arc<RenderPass>,
    ui_render_pass: Arc<RenderPass>,
    surface_state: SurfaceState,
    caches: RenderCaches,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    /// Where the next rendered frame should also be written as a PNG, set by
    /// [`Frame::save_screenshot`] and taken by the next [`RenderState::render`].
    pending_screenshot: Option<PathBuf>,
    /// The next UI draw list to record, set by [`Frame::render_ui`] and
    /// taken by the next [`RenderState::render`] — same fold-into-the-same-
    /// command-buffer pattern as `pending_screenshot`.
    pending_ui: Option<UiDrawList>,
}

impl RenderState {
    fn new(instance: &Arc<Instance>, window: Arc<Window>) -> Self {
        let surface = Surface::from_window(instance.clone(), window.clone())
            .expect("failed to create a Vulkan surface for the window");

        let ctx = VulkanContext::new(instance, &surface);

        // The render pass is built against the swapchain's colour format, so
        // the format has to be settled before either of them exists.
        let image_format = preferred_surface_format(&ctx, &surface);
        let render_pass = create_render_pass(&ctx.device, image_format);
        let ui_render_pass = create_ui_render_pass(&ctx.device, image_format);
        let surface_state = SurfaceState::new(
            &ctx,
            &surface,
            &window,
            &render_pass,
            &ui_render_pass,
            image_format,
        );

        let caches = RenderCaches::new(&ctx);
        let previous_frame_end = Some(sync::now(ctx.device.clone()).boxed());

        RenderState {
            window,
            ctx,
            render_pass,
            ui_render_pass,
            surface_state,
            caches,
            previous_frame_end,
            pending_screenshot: None,
            pending_ui: None,
        }
    }

    /// Marks the next rendered frame for capture into `path`.
    pub(super) fn request_screenshot(&mut self, path: PathBuf) {
        self.pending_screenshot = Some(path);
    }

    /// Queues `draw_list` to be drawn as the next frame's screen-space UI pass.
    pub(super) fn request_ui(&mut self, draw_list: UiDrawList) {
        self.pending_ui = Some(draw_list);
    }

    /// Reclaims finished frames and rebuilds the swapchain if it went stale.
    fn prepare(&mut self, window_size: PhysicalSize<u32>) {
        self.previous_frame_end
            .as_mut()
            .expect("previous_frame_end is always Some between frames")
            .cleanup_finished();

        if self.surface_state.recreate_needed {
            self.surface_state.recreate(
                &self.ctx,
                &self.render_pass,
                &self.ui_render_pass,
                [window_size.width, window_size.height],
            );
        }
    }

    /// Acquires, records, submits and presents one frame.
    pub(super) fn render(&mut self, scene: &Scene, camera: &dyn Camera) {
        let (image_index, is_suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.surface_state.swapchain.clone(), None) {
                Ok(result) => result,
                Err(Validated::Error(VulkanError::OutOfDate)) => {
                    self.surface_state.recreate_needed = true;
                    return;
                }
                Err(err) => panic!("failed to acquire a swapchain image: {err}"),
            };

        if is_suboptimal {
            self.surface_state.recreate_needed = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            self.ctx.command_buffer_allocator.clone(),
            self.ctx.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("failed to start recording a command buffer");

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some(scene.background.to_array().into()),
                        Some(1.0f32.into()),
                    ],
                    ..RenderPassBeginInfo::framebuffer(
                        self.surface_state.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .expect("failed to begin the render pass")
            .set_viewport(
                0,
                [self.surface_state.viewport.clone()].into_iter().collect(),
            )
            .expect("failed to set the viewport");

        record_scene(
            &mut builder,
            &self.ctx,
            &self.render_pass,
            &mut self.caches,
            scene,
            camera.view_proj_matrix(),
        );

        builder
            .end_render_pass(SubpassEndInfo::default())
            .expect("failed to end the render pass");

        // The UI pass rides in the same command buffer, right after the 3D scene:
        // a second begin/end_render_pass pair over the same swapchain image.
        // Vulkan guarantees ordering between sequentially recorded commands in one
        // primary command buffer, so no manual barrier is needed between the two
        // passes (see `neptune-imgui-plus-datgui.md` §3).
        if let Some(draw_list) = self.pending_ui.take() {
            if !draw_list.is_empty() {
                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            // The UI attachment's load_op is `Load`, not `Clear`, so
                            // its clear_values entry must be `None` — see vulkano's
                            // `RenderPassBeginInfo` validation.
                            clear_values: vec![None],
                            ..RenderPassBeginInfo::framebuffer(
                                self.surface_state.ui_framebuffers[image_index as usize].clone(),
                            )
                        },
                        SubpassBeginInfo {
                            contents: SubpassContents::Inline,
                            ..Default::default()
                        },
                    )
                    .expect("failed to begin the UI render pass")
                    .set_viewport(
                        0,
                        [self.surface_state.viewport.clone()].into_iter().collect(),
                    )
                    .expect("failed to set the UI viewport");

                let (width, height) = self.size();
                // top=0, bottom=height: pixel Y-down (matching MouseState::position)
                // maps directly onto this camera with no coordinate flip — see
                // `neptune-imgui-plus-datgui.md` §3.
                let ui_camera =
                    OrthographicCamera::new(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);

                record_ui(
                    &mut builder,
                    &self.ctx,
                    &self.ui_render_pass,
                    &mut self.caches,
                    &draw_list,
                    ui_camera.view_proj_matrix(),
                );

                builder
                    .end_render_pass(SubpassEndInfo::default())
                    .expect("failed to end the UI render pass");
            }
        }

        // A requested screenshot rides along in this same command buffer: the
        // copy is recorded after the render pass has stored its colour
        // attachment, so it reads the finished frame, and before the present,
        // so nothing else has touched the image yet. Vulkano's automatic
        // synchronisation inserts the PresentSrc -> TransferSrcOptimal barrier
        // and the one back again.
        let capture = self.pending_screenshot.take().and_then(|path| {
            match self.record_screenshot_copy(&mut builder, image_index) {
                Ok(buffer) => Some((path, buffer)),
                Err(err) => {
                    eprintln!("neptune: screenshot failed: {err}");
                    None
                }
            }
        });

        let command_buffer = builder.build().expect("failed to build the command buffer");

        let future = self
            .previous_frame_end
            .take()
            .expect("previous_frame_end is always Some between frames")
            .join(acquire_future)
            .then_execute(self.ctx.queue.clone(), command_buffer)
            .expect("failed to submit the command buffer")
            .then_swapchain_present(
                self.ctx.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.surface_state.swapchain.clone(),
                    image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                if let Some((path, buffer)) = capture {
                    // The only place Neptune ever blocks on the GPU mid-loop.
                    // The fence covers the whole submission, copy included, so
                    // once it is signalled the read-back buffer holds this
                    // frame. Screenshots are a one-shot debugging/documentation
                    // tool, not a hot path, so the stall is the right trade.
                    future
                        .wait(None)
                        .expect("the captured frame's fence never signalled");
                    let extent = self.surface_state.extent();
                    let format = self.surface_state.swapchain.image_format();
                    match screenshot::write_png(&buffer, extent, format, &path) {
                        Ok(()) => eprintln!("neptune: wrote screenshot to {}", path.display()),
                        Err(err) => eprintln!("neptune: screenshot failed: {err}"),
                    }
                }
                self.previous_frame_end = Some(future.boxed());
            }
            Err(Validated::Error(VulkanError::OutOfDate)) => {
                self.rearm_dropped_capture(capture);
                self.surface_state.recreate_needed = true;
                self.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
            }
            Err(err) => {
                eprintln!("neptune: dropped a frame ({err})");
                self.rearm_dropped_capture(capture);
                self.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
            }
        }
    }

    /// Re-queues a screenshot whose frame was never submitted.
    ///
    /// A dropped frame takes the copy command down with it, so without this the
    /// request would be silently swallowed and a caller that asked for a
    /// screenshot and then exited would get no file and no explanation.
    fn rearm_dropped_capture(&mut self, capture: Option<(PathBuf, Subbuffer<[u8]>)>) {
        if let Some((path, _)) = capture {
            self.pending_screenshot = Some(path);
        }
    }

    /// Allocates a read-back buffer and records the copy of swapchain image
    /// `image_index` into it.
    ///
    /// Split out of [`RenderState::render`] purely so the fallible setup can
    /// use `?` and be reported in one place; the returned buffer is not
    /// readable until the frame's fence signals.
    fn record_screenshot_copy(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        image_index: u32,
    ) -> Result<Subbuffer<[u8]>, ScreenshotError> {
        if !self.surface_state.transfer_src {
            return Err(ScreenshotError::NotSupported);
        }
        // Fail before recording anything if the format is one this build has no
        // unpacking rule for, rather than capturing bytes nothing can decode.
        screenshot::swaps_red_and_blue(self.surface_state.swapchain.image_format())?;

        let extent = self.surface_state.extent();
        let buffer = screenshot::readback_buffer(&self.ctx.memory_allocator, extent)?;
        let image = self.surface_state.images[image_index as usize].clone();

        builder
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(image, buffer.clone()))
            .expect("failed to record the screenshot copy");

        Ok(buffer)
    }

    pub(super) fn aspect_ratio(&self) -> f32 {
        self.surface_state.aspect_ratio()
    }

    pub(super) fn size(&self) -> (u32, u32) {
        let [w, h] = self.surface_state.extent();
        (w, h)
    }
}

/// The winit side of the loop. Generic over the user's closure so it is called
/// through a static, monomorphised call — no boxing per frame.
struct App<F> {
    instance: Arc<Instance>,
    options: RendererOptions,
    state: Option<RenderState>,
    callback: F,
    input: InputState,
    clock: FrameClock,
}

impl<F> App<F>
where
    F: FnMut(&mut Frame),
{
    fn draw_frame(&mut self, event_loop: &ActiveEventLoop) {
        // Ticked before the early returns below: a minimized window still
        // closes off the frame, so un-minimizing does not deliver one enormous
        // delta covering the whole time the window was hidden.
        let time = self.clock.tick(Instant::now());

        let Some(state) = self.state.as_mut() else {
            return;
        };

        let window_size = state.window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            // Minimized: there is no surface to draw into.
            return;
        }

        state.prepare(window_size);

        let mut exit_requested = false;
        let mut frame = Frame::new(state, &self.input, time, &mut exit_requested);
        (self.callback)(&mut frame);

        self.input.end_frame();

        if exit_requested {
            event_loop.exit();
        }
    }
}

impl<F> ApplicationHandler for App<F>
where
    F: FnMut(&mut Frame),
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.options.title)
            .with_inner_size(PhysicalSize::new(self.options.width, self.options.height));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("failed to create the window"),
        );

        self.state = Some(RenderState::new(&self.instance, window));
        // Window, device and swapchain creation took real time but was not a
        // frame; the first frame should not be billed for it.
        self.clock.discard_gap(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                if let Some(state) = self.state.as_mut() {
                    state.surface_state.recreate_needed = true;
                }
            }
            WindowEvent::KeyboardInput { ref event, .. } => {
                self.input.handle_key_event(event);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_mut().handle_cursor_moved(position);
            }
            WindowEvent::CursorLeft { .. } => {
                self.input.mouse_mut().handle_cursor_left();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.mouse_mut().handle_button_event(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.mouse_mut().handle_scroll(delta);
            }
            WindowEvent::Focused(false) => self.input.release_all(),
            WindowEvent::RedrawRequested => self.draw_frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_describe_a_720p_window() {
        let options = RendererOptions::default();
        assert_eq!((options.width, options.height), (1280, 720));
        assert_eq!(options.title, "Neptune");
    }
}
