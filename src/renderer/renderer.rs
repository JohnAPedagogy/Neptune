//! The window, the event loop, and the per-frame GPU state behind them.

use std::sync::Arc;
use std::time::Instant;

use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo,
    SubpassContents, SubpassEndInfo,
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

use super::frame::Frame;
use crate::backend::command::{RenderCaches, record_scene};
use crate::backend::context::{VulkanContext, create_instance};
use crate::backend::pass::create_render_pass;
use crate::backend::surface::{SurfaceState, preferred_surface_format};
use crate::cameras::Camera;
use crate::core::Scene;
use crate::input::InputState;

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

        let now = Instant::now();
        let mut app = App {
            instance: self.instance.clone(),
            options: self.options,
            state: None,
            callback,
            input: InputState::new(),
            started_at: now,
            last_frame_at: now,
        };

        event_loop
            .run_app(&mut app)
            .expect("the OS event loop exited with an error");
    }

    pub fn options(&self) -> RendererOptions {
        self.options
    }
}

/// Everything that only exists once there is a window: the device, the
/// swapchain, the GPU caches, and the fence chain.
pub(super) struct RenderState {
    window: Arc<Window>,
    ctx: VulkanContext,
    render_pass: Arc<RenderPass>,
    surface_state: SurfaceState,
    caches: RenderCaches,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
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
        let surface_state =
            SurfaceState::new(&ctx, &surface, &window, &render_pass, image_format);

        let caches = RenderCaches::new(&ctx);
        let previous_frame_end = Some(sync::now(ctx.device.clone()).boxed());

        RenderState {
            window,
            ctx,
            render_pass,
            surface_state,
            caches,
            previous_frame_end,
        }
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
            Ok(future) => self.previous_frame_end = Some(future.boxed()),
            Err(Validated::Error(VulkanError::OutOfDate)) => {
                self.surface_state.recreate_needed = true;
                self.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
            }
            Err(err) => {
                eprintln!("neptune: dropped a frame ({err})");
                self.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
            }
        }
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
    started_at: Instant,
    last_frame_at: Instant,
}

impl<F> App<F>
where
    F: FnMut(&mut Frame),
{
    fn draw_frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_frame_at).as_secs_f32();
        let elapsed_seconds = now.duration_since(self.started_at).as_secs_f32();
        self.last_frame_at = now;

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
        let mut frame = Frame::new(
            state,
            &self.input,
            delta_seconds,
            elapsed_seconds,
            &mut exit_requested,
        );
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
        self.last_frame_at = Instant::now();
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
