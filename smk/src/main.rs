mod capture;
mod circuit_paths;
mod ground;

use neptune::prelude::*;

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 1280,
        height: 720,
        title: "Süper Mario Kart — smk (M1: static track)",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x14141c);

    let dummy_camera = PerspectiveCamera::new(75.0_f32.to_radians(), 1280.0 / 720.0, 0.1, 1000.0);

    let mut capture = capture::Capture::from_env();

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        capture.update(frame);
        frame.render(&scene, &dummy_camera);
        // M1 just renders the background with a dummy camera to enable screenshots.
        // Task 5 replaces this with the real camera and adds the ground mesh.
    });
}
