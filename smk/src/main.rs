mod camera_params;
mod capture;
mod circuit_paths;
mod ground;

use std::f32::consts::FRAC_PI_2;

use neptune::prelude::*;

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 1280,
        height: 720,
        title: "Süper Mario Kart — smk (M1: static track)",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x14141c);

    // --- Ground plane: Donut Plains 1's top-down map texture. ---
    let base_texture = Texture::from_file(circuit_paths::base_png_path("donut_plains_1"))
        .expect("Donut Plains 1's base.png should decode (see Task 3's asset copy)");
    let (ground_w, ground_h) =
        ground::ground_plane_size(base_texture.aspect_ratio(), ground::WORLD_SIZE);

    let mut ground_mesh = Mesh::new(
        PlaneGeometry::new(ground_w, ground_h),
        SpriteMaterial::new(base_texture),
    );
    // PlaneGeometry faces +Z by default; rotate -90 degrees around X so it
    // lies flat with its texture facing +Y (up), like a floor.
    ground_mesh.transform.rotation.x = -FRAC_PI_2;
    scene.add(ground_mesh);

    // --- Orbit camera: mouse-drag inspection, matching orbit_cube.rs. ---
    let fov = camera_params::mode7_half_fov_to_vertical_fov(camera_params::MODE7_FOV_HALF);
    let mut camera = OrbitCamera::new(
        fov,
        1280.0 / 720.0,
        camera_params::CAMERA_NEAR,
        camera_params::CAMERA_FAR,
    )
    .with_distance(ground::WORLD_SIZE * 0.75)
    .with_target(Vec3::ZERO);
    // Start looking down at the track from a three-quarter angle, not
    // straight down from the pole (which OrbitCamera's min_polar avoids
    // anyway) and not edge-on.
    camera.polar = 1.0; // radians; ~57 degrees down from straight above

    let mut capture = capture::Capture::from_env();

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        camera.aspect = frame.aspect_ratio();
        camera.update(frame.input());

        capture.update(frame);
        frame.render(&scene, &camera);
    });
}
