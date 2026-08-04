//! Turning a scene graph into recorded draw calls.

use std::sync::Arc;

use glam::Mat4;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::device::Device;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::RenderPass;

use super::context::VulkanContext;
use super::shaders::PushConstants;
use super::texture::TextureCache;
use super::upload::{GeometryCache, upload_indices, upload_vertices};
use crate::core::{Object3D, Scene};
use crate::geometry::SimpleVertex;
use crate::renderer::pipeline::PipelineCache;
use crate::ui::UiDrawList;
use crate::ui::draw_list::UiPrimitive;

/// The three GPU-resource caches the recorder needs, kept together so they can
/// be borrowed as one.
pub(crate) struct RenderCaches {
    pub pipelines: PipelineCache,
    pub geometries: GeometryCache,
    pub textures: TextureCache,
    /// The one pipeline every UI primitive draws with, built lazily on first
    /// use — there is only ever one "material kind" in the UI pass, unlike
    /// the main scene's per-`MaterialId` `PipelineCache`.
    ui_pipeline: Option<Arc<GraphicsPipeline>>,
}

impl RenderCaches {
    pub(crate) fn new(ctx: &VulkanContext) -> Self {
        RenderCaches {
            pipelines: PipelineCache::new(),
            geometries: GeometryCache::new(),
            textures: TextureCache::new(ctx),
            ui_pipeline: None,
        }
    }

    pub(crate) fn get_or_create_ui_pipeline(
        &mut self,
        device: &Arc<Device>,
        ui_render_pass: &Arc<RenderPass>,
    ) -> Arc<GraphicsPipeline> {
        self.ui_pipeline
            .get_or_insert_with(|| {
                crate::renderer::pipeline::build_ui_pipeline(device, ui_render_pass)
            })
            .clone()
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

/// Draws every primitive in `draw_list`, in call order (no depth attachment
/// in the UI pass, so later primitives simply paint over earlier ones — see
/// `neptune-imgui-plus-datgui.md` §5). One draw call per primitive, same
/// shape as `record_draw`'s one-draw-call-per-object, just against a single
/// shared pipeline instead of a per-material one.
pub(crate) fn record_ui(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ctx: &VulkanContext,
    ui_render_pass: &Arc<RenderPass>,
    caches: &mut RenderCaches,
    draw_list: &UiDrawList,
    view_proj: Mat4,
) {
    if draw_list.is_empty() {
        return;
    }

    let pipeline = caches.get_or_create_ui_pipeline(&ctx.device, ui_render_pass);
    let layout = pipeline.layout().clone();

    builder
        .bind_pipeline_graphics(pipeline)
        .expect("failed to bind the UI pipeline");

    for primitive in &draw_list.primitives {
        let (vertices, indices) = quad_mesh(primitive);
        let vertex_buffer = upload_vertices(&ctx.memory_allocator, &vertices);
        let index_buffer = upload_indices(&ctx.memory_allocator, &indices);

        let set_layout = layout
            .set_layouts()
            .first()
            .expect("the UI pipeline always declares descriptor set 0");
        let descriptor_set = caches.textures.get_or_upload(ctx, set_layout, &primitive.texture);

        builder
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, descriptor_set)
            .expect("failed to bind the UI texture descriptor set")
            .push_constants(
                layout.clone(),
                0,
                PushConstants {
                    mvp: view_proj.to_cols_array_2d(),
                    color: primitive.color.to_array(),
                },
            )
            .expect("failed to push UI per-quad constants")
            .bind_vertex_buffers(0, vertex_buffer)
            .expect("failed to bind UI vertex buffer")
            .bind_index_buffer(index_buffer)
            .expect("failed to bind UI index buffer");

        unsafe {
            builder
                .draw_indexed(indices.len() as u32, 1, 0, 0, 0)
                .expect("failed to record a UI draw");
        }
    }
}

/// Two triangles covering `primitive.rect`, with `primitive`'s UV rect
/// mapped so the top-left screen corner gets `uv_min` — the direct pixel-Y-
/// down analogue of `text_mesh.rs::layout`'s world-Y-up quad.
fn quad_mesh(primitive: &UiPrimitive) -> (Vec<SimpleVertex>, Vec<u32>) {
    let (x0, y0) = (primitive.rect.min.x, primitive.rect.min.y);
    let (x1, y1) = (primitive.rect.max.x, primitive.rect.max.y);
    let [u0, v0] = primitive.uv_min;
    let [u1, v1] = primitive.uv_max;
    const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

    let vertices = vec![
        SimpleVertex::new([x0, y0, 0.0], NORMAL, [u0, v0]), // top-left
        SimpleVertex::new([x1, y0, 0.0], NORMAL, [u1, v0]), // top-right
        SimpleVertex::new([x1, y1, 0.0], NORMAL, [u1, v1]), // bottom-right
        SimpleVertex::new([x0, y1, 0.0], NORMAL, [u0, v1]), // bottom-left
    ];
    let indices = vec![0, 1, 2, 2, 3, 0];
    (vertices, indices)
}
