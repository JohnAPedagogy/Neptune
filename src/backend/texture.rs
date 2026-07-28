//! Getting decoded pixels onto the GPU and into a descriptor set.

use std::collections::HashMap;
use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
};
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::sync::GpuFuture;

use super::context::VulkanContext;
use crate::materials::{Texture, TextureId};

/// Uploads RGBA8 pixels into a sampled device image.
///
/// Staging buffer -> `copy_buffer_to_image` -> fence wait. Blocking is fine
/// here: uploads happen once per texture, the first time it is drawn.
fn upload_texture(ctx: &VulkanContext, texture: &Texture) -> Arc<ImageView> {
    let staging = Buffer::from_iter(
        ctx.memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        texture.rgba().iter().copied(),
    )
    .expect("failed to create texture staging buffer");

    let image = Image::new(
        ctx.memory_allocator.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::R8G8B8A8_UNORM,
            extent: [texture.width(), texture.height(), 1],
            usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )
    .expect("failed to allocate texture image");

    let mut builder = AutoCommandBufferBuilder::primary(
        ctx.command_buffer_allocator.clone(),
        ctx.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .expect("failed to create texture upload command buffer");

    builder
        .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(staging, image.clone()))
        .expect("failed to record texture copy");

    let command_buffer = builder.build().expect("failed to build texture upload");

    vulkano::sync::now(ctx.device.clone())
        .then_execute(ctx.queue.clone(), command_buffer)
        .expect("failed to submit texture upload")
        .then_signal_fence_and_flush()
        .expect("failed to flush texture upload")
        .wait(None)
        .expect("texture upload never completed");

    ImageView::new_default(image).expect("failed to create texture image view")
}

/// Keeps one descriptor set per texture identity, so a sprite sheet or glyph
/// atlas shared by many objects is uploaded and bound once.
pub(crate) struct TextureCache {
    sampler: Arc<Sampler>,
    entries: HashMap<TextureId, Arc<DescriptorSet>>,
}

impl TextureCache {
    pub(crate) fn new(ctx: &VulkanContext) -> Self {
        let sampler = Sampler::new(
            ctx.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        )
        .expect("failed to create texture sampler");

        TextureCache {
            sampler,
            entries: HashMap::new(),
        }
    }

    pub(crate) fn get_or_upload(
        &mut self,
        ctx: &VulkanContext,
        layout: &Arc<DescriptorSetLayout>,
        texture: &Texture,
    ) -> Arc<DescriptorSet> {
        self.entries
            .entry(texture.id())
            .or_insert_with(|| {
                let view = upload_texture(ctx, texture);
                DescriptorSet::new(
                    ctx.descriptor_set_allocator.clone(),
                    layout.clone(),
                    [WriteDescriptorSet::image_view_sampler(
                        0,
                        view,
                        self.sampler.clone(),
                    )],
                    [],
                )
                .expect("failed to create texture descriptor set")
            })
            .clone()
    }
}
