//! The built-in GLSL, compiled to SPIR-V at build time by `vulkano-shaders`.
//!
//! Two fragment shaders share one vertex shader and one push-constant block:
//! a `mat4` model-view-projection matrix and an RGBA colour, 80 bytes total,
//! comfortably inside the 128-byte guaranteed push-constant budget.

use vulkano::buffer::BufferContents;

/// The push-constant block both pipelines declare. Layout must stay in lockstep
/// with the `PushConstants` block in the GLSL below.
#[derive(BufferContents, Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct PushConstants {
    pub mvp: [[f32; 4]; 4],
    pub color: [f32; 4],
}

pub(crate) mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;
            layout(location = 2) in vec2 uv;

            layout(location = 0) out vec2 v_uv;
            layout(location = 1) out vec3 v_normal;

            layout(push_constant) uniform PushConstants {
                mat4 mvp;
                vec4 color;
            } pc;

            void main() {
                v_uv = uv;
                v_normal = normal;

                vec4 clip = pc.mvp * vec4(position, 1.0);
                // glam builds right-handed, Y-up matrices; Vulkan's clip space
                // is Y-down. Flip here so world +Y is screen up.
                clip.y = -clip.y;
                gl_Position = clip;
            }
        ",
    }
}

/// Flat, unlit colour — `MeshBasicMaterial`.
pub(crate) mod fs_basic {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec2 v_uv;
            layout(location = 1) in vec3 v_normal;

            layout(location = 0) out vec4 f_color;

            layout(push_constant) uniform PushConstants {
                mat4 mvp;
                vec4 color;
            } pc;

            void main() {
                f_color = pc.color;
            }
        ",
    }
}

/// Sampled texture multiplied by a tint — `SpriteMaterial` and text.
pub(crate) mod fs_sprite {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec2 v_uv;
            layout(location = 1) in vec3 v_normal;

            layout(location = 0) out vec4 f_color;

            layout(set = 0, binding = 0) uniform sampler2D tex;

            layout(push_constant) uniform PushConstants {
                mat4 mvp;
                vec4 color;
            } pc;

            void main() {
                vec4 texel = texture(tex, v_uv);
                vec4 result = texel * pc.color;
                if (result.a < 0.004) {
                    discard;
                }
                f_color = result;
            }
        ",
    }
}
