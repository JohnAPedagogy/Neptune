//! Swapchain, framebuffers, and resize handling.

use std::sync::Arc;

use vulkano::format::Format;
use vulkano::image::ImageUsage;
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use winit::window::Window;

use super::context::VulkanContext;
use super::pass::{create_depth_view, create_framebuffers};

/// Everything sized by the window: the swapchain, its framebuffers, the depth
/// attachment, and the viewport recorded into every command buffer.
///
/// All of it is thrown away and rebuilt by [`SurfaceState::recreate`] whenever
/// the window resizes or the driver reports the swapchain stale.
pub(crate) struct SurfaceState {
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub depth_view: Arc<ImageView>,
    pub viewport: Viewport,
    pub recreate_needed: bool,
}

/// The colour format the swapchain will use — queried before the render pass
/// is built, since the pass must be created against it.
pub(crate) fn preferred_surface_format(ctx: &VulkanContext, surface: &Arc<Surface>) -> Format {
    ctx.device
        .physical_device()
        .surface_formats(surface, Default::default())
        .expect("failed to query surface formats")[0]
        .0
}

impl SurfaceState {
    pub(crate) fn new(
        ctx: &VulkanContext,
        surface: &Arc<Surface>,
        window: &Window,
        render_pass: &Arc<RenderPass>,
        image_format: Format,
    ) -> Self {
        let capabilities = ctx
            .device
            .physical_device()
            .surface_capabilities(surface, Default::default())
            .expect("failed to query surface capabilities");

        let window_size = window.inner_size();
        let extent = [window_size.width.max(1), window_size.height.max(1)];

        let (swapchain, images) = Swapchain::new(
            ctx.device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count.max(2),
                image_format,
                image_extent: extent,
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha: capabilities
                    .supported_composite_alpha
                    .into_iter()
                    .next()
                    .expect("surface supports no composite alpha mode"),
                present_mode: PresentMode::Fifo,
                ..Default::default()
            },
        )
        .expect("failed to create swapchain");

        let depth_view = create_depth_view(&ctx.memory_allocator, extent);
        let framebuffers = create_framebuffers(render_pass, &images, &depth_view);

        SurfaceState {
            swapchain,
            framebuffers,
            depth_view,
            viewport: Viewport {
                offset: [0.0, 0.0],
                extent: [extent[0] as f32, extent[1] as f32],
                depth_range: 0.0..=1.0,
            },
            recreate_needed: false,
        }
    }

    /// Current swapchain size in pixels.
    pub(crate) fn extent(&self) -> [u32; 2] {
        self.swapchain.image_extent()
    }

    /// Width divided by height, for cameras that track the window.
    pub(crate) fn aspect_ratio(&self) -> f32 {
        let [w, h] = self.extent();
        w as f32 / h.max(1) as f32
    }

    /// Rebuilds the swapchain and everything sized with it.
    pub(crate) fn recreate(
        &mut self,
        ctx: &VulkanContext,
        render_pass: &Arc<RenderPass>,
        extent: [u32; 2],
    ) {
        let extent = [extent[0].max(1), extent[1].max(1)];

        let (swapchain, images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: extent,
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

        self.depth_view = create_depth_view(&ctx.memory_allocator, extent);
        self.framebuffers = create_framebuffers(render_pass, &images, &self.depth_view);
        self.swapchain = swapchain;
        self.viewport.extent = [extent[0] as f32, extent[1] as f32];
        self.recreate_needed = false;
    }
}
