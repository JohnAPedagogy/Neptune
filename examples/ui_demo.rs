//! `orbit_cube`, plus live control panels: drag Speed, toggle Wireframe,
//! pick a Shading mode, recolour the cube, and fold away Advanced — all
//! drawn as a second, screen-space pass over the 3D scene. Demonstrates the
//! design in `neptune-imgui-plus-datgui.md`, including the draggable,
//! dockable `window` containers.
//!
//! Run it with `cargo run --example ui_demo`. Controls are `orbit_cube`'s
//! (left-drag orbits, scroll zooms, right-drag pans, Escape quits) plus the
//! two panels: drag a panel's title bar to move it, or drag it to within a
//! few pixels of a screen edge to dock it there. Panels eat clicks before
//! they reach the camera controls.

use neptune::prelude::*;

#[path = "common/capture.rs"]
mod capture;

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 1280,
        height: 720,
        title: "Neptune - ui_demo",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x14141c);

    let mut camera = OrbitCamera::new(75.0_f32.to_radians(), 1280.0 / 720.0, 0.1, 100.0)
        .with_distance(1.5);

    let cube = Mesh::new(
        BoxGeometry::new(1.0, 1.0, 1.0),
        MeshBasicMaterial::new(Color::hex(0x9fb4ff)).with_wireframe(true),
    );
    let cube_id = scene.add(cube);

    let atlas = Font::system_default()
        .and_then(|font| font.atlas(24.0))
        .expect("a system font is available");
    let mut ui = Ui::new(atlas);

    let mut speed = 0.8f32;
    let mut wireframe = true;
    let mut tint = Color::hex(0x9fb4ff);
    let shading_options = ["Flat", "Smooth"];
    let mut shading_idx = 0usize;
    let mut fov_deg = 75.0f32;

    let mut capture = capture::Capture::from_env();

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        if let Some(cube) = scene.get_mut(cube_id) {
            let transform = cube.transform_mut();
            transform.rotation.x += frame.delta_seconds() * speed;
            transform.rotation.y += frame.delta_seconds() * speed;
        }

        camera.aspect = frame.aspect_ratio();
        camera.update(frame.input());

        // `frame.scale_factor()` is the OS's real display scale; the extra
        // `* 1.4` is a deliberate demo-only bump so the panels read clearly
        // in a screenshot. The window width scales by the same factor so the
        // DPI-scaled columns still fit.
        let ppp = frame.scale_factor() * 1.4;
        ui.set_pixels_per_point(ppp);

        // Seat the two windows. `place_window` only takes effect before a
        // window has been drawn once, so from then on they remember where the
        // user dragged (or docked) them.
        ui.place_window("Controls", Vec2::new(16.0, 16.0));
        ui.place_window("Advanced", Vec2::new(16.0, 16.0 + 160.0 * ppp));

        let mouse = frame.input().mouse().clone();
        let (width, height) = frame.size();
        let mut ui_frame = ui.begin(&mouse, (width as f32, height as f32), Vec2::ZERO, 0.0);

        ui_frame.window("Controls", 280.0 * ppp, |ui| {
            ui.label("Controls", TextStyle::Heading, Color::WHITE);
            ui.slider("Speed", &mut speed, 0.0..=5.0);
            ui.checkbox("Wireframe", &mut wireframe);
            ui.dropdown("Shading", &shading_options, &mut shading_idx);
            ui.color_edit("Tint", &mut tint);
        });
        ui_frame.window("Advanced", 280.0 * ppp, |ui| {
            ui.slider("FOV", &mut fov_deg, 30.0..=120.0);
        });
        let draw_list = ui_frame.finish();

        if let Some(cube) = scene.get_mut_as::<Mesh<BufferGeometry<SimpleVertex>, MeshBasicMaterial>>(cube_id) {
            cube.material.wireframe = wireframe;
            cube.material.color = tint;
        }
        camera.fov = fov_deg.to_radians();

        frame.render_ui(draw_list);

        capture.update(frame);
        frame.render(&scene, &camera);
    });
}
