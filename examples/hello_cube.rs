//! The smoke test for Neptune's whole public API: a spinning cube.
//!
//! Note the one import. No `use vulkano::...` appears anywhere in this file,
//! and that is the point.
//!
//! Run it with `cargo run --example hello_cube`. Press `Escape` to quit.

use neptune::prelude::*;

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 1280,
        height: 720,
        title: "Neptune — hello_cube",
    });

    let mut scene = Scene::new();

    let mut camera = PerspectiveCamera::new(75.0_f32.to_radians(), 1280.0 / 720.0, 0.1, 1000.0);

    let geometry = BoxGeometry::new(1.0, 1.0, 1.0);
    let material = MeshBasicMaterial::new(Color::hex(0x00ff88));
    let mut cube = Mesh::new(geometry, material);
    cube.transform.position.z = -5.0;

    let cube_id = scene.add(cube);

    // Unlit materials ignore these, but a scene is where lights live and this
    // is what adding them looks like.
    scene.add_light(AmbientLight::new(Color::WHITE, 0.2));
    scene.add_light(
        DirectionalLight::new(Color::WHITE, 0.8).with_direction(Vec3::new(-0.4, -1.0, -0.6)),
    );

    let mut angle: f32 = 0.0;

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        // Delta time keeps the spin rate the same on a 60Hz and a 144Hz screen.
        angle += frame.delta_seconds();

        if let Some(cube) = scene.get_mut(cube_id) {
            let transform = cube.transform_mut();
            transform.rotation.y = angle;
            transform.rotation.x = angle * 0.5;
        }

        // Keep the projection honest when the window is resized.
        camera.aspect = frame.aspect_ratio();

        frame.render(&scene, &camera);
    });
}
