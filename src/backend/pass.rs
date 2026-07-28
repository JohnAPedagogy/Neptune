//! Render pass and framebuffers.

use std::sync::Arc;

use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};

/// Depth format Neptune asks for. `D32_SFLOAT` is mandatory-supported as a
/// depth attachment on every Vulkan implementation.
pub(crate) const DEPTH_FORMAT: Format = Format::D32_SFLOAT;

/// A single subpass writing one colour attachment and one depth attachment.
///
/// Unlike the reference triangle, Neptune renders 3D geometry, so the depth
/// buffer is not optional here — without it the far faces of a cube paint over
/// the near ones.
pub(crate) fn create_render_pass(device: &Arc<Device>, color_format: Format) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                format: color_format,
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth: {
                format: DEPTH_FORMAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
        },
        pass: {
            color: [color],
            depth_stencil: {depth},
        },
    )
    .expect("failed to create render pass")
}

/// Allocates the depth attachment for a given swapchain size.
pub(crate) fn create_depth_view(
    memory_allocator: &Arc<StandardMemoryAllocator>,
    extent: [u32; 2],
) -> Arc<ImageView> {
    let image = Image::new(
        memory_allocator.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: DEPTH_FORMAT,
            extent: [extent[0], extent[1], 1],
            usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )
    .expect("failed to allocate depth image");

    ImageView::new_default(image).expect("failed to create depth image view")
}

/// One framebuffer per swapchain image, all sharing the single depth view.
pub(crate) fn create_framebuffers(
    render_pass: &Arc<RenderPass>,
    images: &[Arc<Image>],
    depth_view: &Arc<ImageView>,
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let color_view = ImageView::new_default(image.clone())
                .expect("failed to create swapchain image view");
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![color_view, depth_view.clone()],
                    ..Default::default()
                },
            )
            .expect("failed to create framebuffer")
        })
        .collect()
}
