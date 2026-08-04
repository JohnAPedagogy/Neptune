//! One compiled `GraphicsPipeline` per material kind, built on first use.

use std::collections::HashMap;
use std::sync::Arc;

use vulkano::device::Device;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::{CompareOp, DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::{Vertex as VulkanoVertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{RenderPass, Subpass};
use vulkano::shader::EntryPoint;

use crate::backend::shaders;
use crate::geometry::SimpleVertex;
use crate::materials::MaterialId;

/// Maps each [`MaterialId`] to the pipeline that draws it.
///
/// Compilation is expensive and identical for every instance of a material
/// kind, so it happens once, lazily, the first time such a material is drawn.
pub(crate) struct PipelineCache {
    pipelines: HashMap<MaterialId, Arc<GraphicsPipeline>>,
}

impl PipelineCache {
    pub(crate) fn new() -> Self {
        PipelineCache {
            pipelines: HashMap::new(),
        }
    }

    /// Returns the pipeline for `id`, compiling it if this is its first use.
    pub(crate) fn get_or_create(
        &mut self,
        device: &Arc<Device>,
        render_pass: &Arc<RenderPass>,
        id: MaterialId,
    ) -> Arc<GraphicsPipeline> {
        self.pipelines
            .entry(id)
            .or_insert_with(|| build_pipeline(device, render_pass, id))
            .clone()
    }
}

fn build_pipeline(
    device: &Arc<Device>,
    render_pass: &Arc<RenderPass>,
    id: MaterialId,
) -> Arc<GraphicsPipeline> {
    let vs = entry_point(
        shaders::vs::load(device.clone()).expect("failed to load the shared vertex shader"),
    );
    let fs = match id {
        MaterialId::Basic | MaterialId::BasicWireframe => entry_point(
            shaders::fs_basic::load(device.clone())
                .expect("failed to load the flat-colour fragment shader"),
        ),
        MaterialId::Sprite => entry_point(
            shaders::fs_sprite::load(device.clone())
                .expect("failed to load the textured fragment shader"),
        ),
    };

    let vertex_input_state = SimpleVertex::per_vertex()
        .definition(&vs)
        .expect("SimpleVertex does not match the vertex shader's inputs");

    let stages = vec![
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];

    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(stages.iter())
            .into_pipeline_layout_create_info(device.clone())
            .expect("failed to derive the pipeline layout from the shader stages"),
    )
    .expect("failed to create pipeline layout");

    let subpass = Subpass::from(render_pass.clone(), 0).expect("render pass has no subpass 0");

    // Opaque geometry writes depth; alpha-blended sprites test against it but
    // must not write, or a transparent quad would occlude whatever is behind it.
    let (blend, depth) = match id {
        MaterialId::Basic | MaterialId::BasicWireframe => (None, DepthState::simple()),
        MaterialId::Sprite => (
            Some(AttachmentBlend::alpha()),
            DepthState {
                write_enable: false,
                compare_op: CompareOp::LessOrEqual,
            },
        ),
    };

    // Wireframe drops filled faces and rasterises the triangle edges as lines.
    let rasterization_state = match id {
        MaterialId::BasicWireframe => RasterizationState {
            polygon_mode: PolygonMode::Line,
            ..Default::default()
        },
        _ => RasterizationState::default(),
    };

    GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState::default()),
            rasterization_state: Some(rasterization_state),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState {
                    blend,
                    ..Default::default()
                },
            )),
            depth_stencil_state: Some(DepthStencilState {
                depth: Some(depth),
                ..Default::default()
            }),
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .expect("failed to compile graphics pipeline")
}

fn entry_point(module: Arc<vulkano::shader::ShaderModule>) -> EntryPoint {
    module
        .entry_point("main")
        .expect("shader module has no `main` entry point")
}

/// Builds the one pipeline the UI pass draws every primitive with: the same
/// vertex shader and the sprite fragment shader (tinted, textured, alpha
/// blended) as `MaterialId::Sprite`, but against `ui_render_pass` — which,
/// unlike the main pass, has no depth attachment, so `depth_stencil_state`
/// is `None` rather than reusing `MaterialId::Sprite`'s depth-test-no-write
/// state. A pipeline is tied to the render pass (strictly, a *compatible*
/// one) it was built against, so the main pass's already-cached Sprite
/// pipeline cannot simply be reused here.
pub(crate) fn build_ui_pipeline(
    device: &Arc<Device>,
    ui_render_pass: &Arc<RenderPass>,
) -> Arc<GraphicsPipeline> {
    let vs = entry_point(
        shaders::vs::load(device.clone()).expect("failed to load the shared vertex shader"),
    );
    let fs = entry_point(
        shaders::fs_sprite::load(device.clone())
            .expect("failed to load the textured fragment shader"),
    );

    let vertex_input_state = SimpleVertex::per_vertex()
        .definition(&vs)
        .expect("SimpleVertex does not match the vertex shader's inputs");

    let stages = vec![
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];

    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(stages.iter())
            .into_pipeline_layout_create_info(device.clone())
            .expect("failed to derive the pipeline layout from the shader stages"),
    )
    .expect("failed to create pipeline layout");

    let subpass =
        Subpass::from(ui_render_pass.clone(), 0).expect("UI render pass has no subpass 0");

    GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState::default()),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState {
                    blend: Some(AttachmentBlend::alpha()),
                    ..Default::default()
                },
            )),
            depth_stencil_state: None,
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .expect("failed to compile the UI pipeline")
}
