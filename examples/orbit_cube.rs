//! A Three.js hello-cube brought to Neptune: a wireframe cube you can look at
//! from every angle with the orbit controls.
//!
//! This is a port of the Three.js OrbitControls starter, with each piece mapped
//! to its Neptune equivalent:
//!
//! - `THREE.PerspectiveCamera` + `THREE.OrbitControls` → [`OrbitCamera`]
//! - `camera.position.z = 1.5` → [`OrbitCamera::with_distance`]
//! - `new THREE.MeshNormalMaterial({ wireframe: true })` →
//!   [`MeshBasicMaterial::with_wireframe`]
//! - the `resize` listener → [`Frame::aspect_ratio`]
//! - the `animate()` rotation → a delta-scaled [`Transform::rotation`] tweak
//!
//! Controls:
//!
//! - **left-drag** — orbit around the cube
//! - **scroll** — zoom in and out
//! - **right-drag** — pan the view
//! - **Escape** — quit
//!
//! Run it with `cargo run --example orbit_cube`. Setting `NEPTUNE_SCREENSHOT`
//! saves a frame and exits instead of waiting to be closed — see [`capture`].

use neptune::prelude::*;

#[path = "common/capture.rs"]
mod capture;

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 1280,
        height: 720,
        title: "Neptune — orbit_cube",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x14141c);

    // The starter's `camera.position.z = 1.5`, as an orbit around the origin.
    let mut camera = OrbitCamera::new(75.0_f32.to_radians(), 1280.0 / 720.0, 0.1, 100.0)
        .with_distance(1.5);

    let cube = Mesh::new(
        BoxGeometry::new(1.0, 1.0, 1.0),
        MeshBasicMaterial::new(Color::hex(0x9fb4ff)).with_wireframe(true),
    );
    let cube_id = scene.add(cube);

    let mut capture = capture::Capture::from_env();

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        // The original's per-frame `rotation += 0.01`, delta-scaled so the spin
        // rate is the same on a 60Hz and a 144Hz screen.
        if let Some(cube) = scene.get_mut(cube_id) {
            let transform = cube.transform_mut();
            transform.rotation.x += frame.delta_seconds() * 0.8;
            transform.rotation.y += frame.delta_seconds() * 0.8;
        }

        // `window.addEventListener('resize', ...)`: keep the projection honest.
        camera.aspect = frame.aspect_ratio();

        // The `OrbitControls` the starter attaches to its camera.
        camera.update(frame.input());

        // No-op unless NEPTUNE_SCREENSHOT is set; must come before `render`.
        capture.update(frame);
        frame.render(&scene, &camera);
    });
}
