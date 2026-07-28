//! Turning a scene graph into recorded draw calls.

use std::sync::Arc;

use glam::Mat4;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::render_pass::RenderPass;

use super::context::VulkanContext;
use super::shaders::PushConstants;
use super::texture::TextureCache;
use super::upload::GeometryCache;
use crate::core::{Object3D, Scene};
use crate::renderer::pipeline::PipelineCache;

/// The three GPU-resource caches the recorder needs, kept together so they can
/// be borrowed as one.
pub(crate) struct RenderCaches {
    pub pipelines: PipelineCache,
    pub geometries: GeometryCache,
    pub textures: TextureCache,
}

impl RenderCaches {
    pub(crate) fn new(ctx: &VulkanContext) -> Self {
        RenderCaches {
            pipelines: PipelineCache::new(),
            geometries: GeometryCache::new(),
            textures: TextureCache::new(ctx),
        }
    }
}

/// Records a draw call for every visible, renderable object in `scene`.
///
/// Objects are drawn in insertion order; there is no depth sorting, so a scene
/// mixing opaque and alpha-blended objects should add the opaque ones first.
pub(crate) fn record_scene(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ctx: &VulkanContext,
    render_pass: &Arc<RenderPass>,
    caches: &mut RenderCaches,
    scene: &Scene,
    view_proj: Mat4,
) {
    for object in scene.objects() {
        record_object(
            builder,
            ctx,
            render_pass,
            caches,
            object,
            view_proj,
            Mat4::IDENTITY,
        );
    }
}

fn record_object(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ctx: &VulkanContext,
    render_pass: &Arc<RenderPass>,
    caches: &mut RenderCaches,
    object: &dyn Object3D,
    view_proj: Mat4,
    parent: Mat4,
) {
    if !object.visible() {
        return;
    }

    let world = parent * object.transform().matrix();

    if let Some(renderable) = object.renderable() {
        record_draw(
            builder,
            ctx,
            render_pass,
            caches,
            &renderable,
            view_proj * world,
        );
    }

    for child in object.children() {
        record_object(
            builder,
            ctx,
            render_pass,
            caches,
            child.as_ref(),
            view_proj,
            world,
        );
    }
}

fn record_draw(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ctx: &VulkanContext,
    render_pass: &Arc<RenderPass>,
    caches: &mut RenderCaches,
    renderable: &crate::core::Renderable<'_>,
    mvp: Mat4,
) {
    let material = renderable.material;
    let binding = material.bind();

    let pipeline = caches
        .pipelines
        .get_or_create(&ctx.device, render_pass, material.material_id());
    let layout = pipeline.layout().clone();

    // Empty geometry is legitimate — an empty string has no glyph quads.
    let Some(gpu) = caches
        .geometries
        .get_or_upload(&ctx.memory_allocator, renderable.geometry)
    else {
        return;
    };
    let (vertex_buffer, index_buffer, index_count) = (
        gpu.vertex_buffer.clone(),
        gpu.index_buffer.clone(),
        gpu.index_count,
    );

    builder
        .bind_pipeline_graphics(pipeline)
        .expect("failed to bind graphics pipeline");

    if let Some(texture) = binding.texture {
        let set_layout = layout
            .set_layouts()
            .first()
            .expect("a textured pipeline always declares descriptor set 0");
        let descriptor_set = caches.textures.get_or_upload(ctx, set_layout, texture);
        builder
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                layout.clone(),
                0,
                descriptor_set,
            )
            .expect("failed to bind texture descriptor set");
    }

    builder
        .push_constants(
            layout,
            0,
            PushConstants {
                mvp: mvp.to_cols_array_2d(),
                color: binding.color.to_array(),
            },
        )
        .expect("failed to push per-draw constants")
        .bind_vertex_buffers(0, vertex_buffer)
        .expect("failed to bind vertex buffer")
        .bind_index_buffer(index_buffer)
        .expect("failed to bind index buffer");

    unsafe {
        builder
            .draw_indexed(index_count, 1, 0, 0, 0)
            .expect("failed to record indexed draw");
    }
}
