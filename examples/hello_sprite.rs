//! The 2D half of the API: an orthographic camera, a textured quad, and text.
//!
//! This is the path a 2D game draws through — sprite quads and a glyph atlas,
//! both on the alpha-blended textured pipeline.
//!
//! Run it with `cargo run --example hello_sprite`. Press `Escape` to quit.

use neptune::prelude::*;

/// Builds a checkerboard texture in memory, so the example needs no asset file.
fn checkerboard(size: u32, squares: u32, a: Color, b: Color) -> Texture {
    let cell = (size / squares).max(1);
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let color = if ((x / cell) + (y / cell)).is_multiple_of(2) {
                a
            } else {
                b
            };
            for channel in color.to_array() {
                rgba.push((channel * 255.0) as u8);
            }
        }
    }
    Texture::from_rgba8(size, size, rgba).expect("checkerboard buffer is correctly sized")
}

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 960,
        height: 540,
        title: "Neptune — hello_sprite",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x101018);

    // 10 world units tall; the width follows the window's aspect ratio.
    const VIEW_HEIGHT: f32 = 10.0;
    let mut camera =
        OrthographicCamera::from_size(VIEW_HEIGHT * 16.0 / 9.0, VIEW_HEIGHT, -100.0, 100.0);

    let sprite = Mesh::new(
        PlaneGeometry::new(5.0, 5.0),
        SpriteMaterial::new(checkerboard(64, 8, Color::WHITE, Color::hex(0x4488ff))),
    );
    let sprite_id = scene.add(sprite);

    // The atlas is rasterised once and shared by every TextMesh built from it.
    let atlas = Font::system_default()
        .and_then(|font| font.atlas(64.0))
        .expect("a system font is available");
    let mut label = TextMesh::with_color(atlas, "score 0", Color::hex(0xffdd55));
    label.transform.position = Vec3::new(-2.0, -4.0, 0.0);
    label.transform.scale = Vec3::splat(1.5);
    let label_id = scene.add(label);

    let mut score = 0u32;
    let mut next_tick = 1.0f32;

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        camera.set_view_height(VIEW_HEIGHT, frame.aspect_ratio());

        if let Some(sprite) = scene.get_mut(sprite_id) {
            sprite.transform_mut().rotation.z = frame.elapsed_seconds() * 0.5;
        }

        // Rewriting the text every second exercises the in-place geometry
        // update: same GPU cache slot, no new texture.
        if frame.elapsed_seconds() > next_tick {
            next_tick += 1.0;
            score += 1;
            if let Some(label) = scene.get_mut_as::<TextMesh>(label_id) {
                label.set_text(&format!("score {score}"));
            }
        }

        frame.render(&scene, &camera);
    });
}
