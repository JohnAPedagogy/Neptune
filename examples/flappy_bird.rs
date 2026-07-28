//! A complete, playable Flappy Bird built on Neptune's public API.
//!
//! Run it with `cargo run --example flappy_bird`. Space flaps (or restarts
//! after a game over); Escape quits.
//!
//! The game is split into two halves on purpose:
//!
//! - [`logic`] is plain Rust: gravity, pipe scrolling/recycling, collision and
//!   scoring, over `glam` types and Task 1's `Aabb2d`. It never touches a
//!   window, a renderer, or the GPU, so it is covered by ordinary `#[test]`s
//!   at the bottom of the module and can be run with
//!   `cargo test --example flappy_bird`.
//! - `main` is the winit/render plumbing: it owns a `neptune::Scene`, keeps a
//!   handful of long-lived meshes in sync with `logic::GameState` every
//!   frame, and never invents any game rules of its own.

use std::path::Path;

use neptune::prelude::*;

use logic::{GameConfig, GameState};

/// Gravity/velocity integration, pipe scrolling and recycling, collision, and
/// scoring — everything a test can exercise without a GPU or a window.
mod logic {
    use neptune::prelude::{Aabb2d, Vec2};
    use rand::Rng;

    /// How far beyond the floor/ceiling a pipe's rectangle extends, so it
    /// visually (and for collision purposes) reaches off the edge of the
    /// screen rather than stopping exactly at the gap.
    const PIPE_OVERDRAW: f32 = 12.0;

    /// The bird: a single point mass under gravity, nudged upward by flaps.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Bird {
        pub position: Vec2,
        pub velocity: f32,
        pub size: Vec2,
    }

    impl Bird {
        pub fn new(position: Vec2, size: Vec2) -> Self {
            Bird {
                position,
                velocity: 0.0,
                size,
            }
        }

        /// Sets the upward velocity a flap gives, overriding whatever the
        /// bird was doing before (so mashing space near the ground still
        /// saves you).
        pub fn flap(&mut self, flap_velocity: f32) {
            self.velocity = flap_velocity;
        }

        /// Semi-implicit Euler: velocity updates first, then position uses the
        /// *new* velocity for the step. Frame-rate independent as long as
        /// `dt` is the real elapsed time.
        pub fn integrate(&mut self, dt: f32, gravity: f32) {
            self.velocity += gravity * dt;
            self.position.y += self.velocity * dt;
        }

        pub fn aabb(&self) -> Aabb2d {
            Aabb2d::from_center_size(self.position, self.size)
        }
    }

    /// One pipe pair: a gap of fixed size centred on `gap_center`, scrolling
    /// left at a fixed speed, recycled once it scrolls off-screen.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Pipe {
        pub x: f32,
        pub gap_center: f32,
        /// Whether the bird has already been credited for passing this pipe,
        /// so scoring only fires once per pipe per pass.
        pub scored: bool,
    }

    impl Pipe {
        /// The upper obstacle: from the top of the gap up to (and past) the
        /// ceiling.
        pub fn top_aabb(&self, config: &GameConfig) -> Aabb2d {
            let gap_top = self.gap_center + config.pipe_gap * 0.5;
            Aabb2d::new(
                Vec2::new(self.x - config.pipe_width * 0.5, gap_top),
                Vec2::new(
                    self.x + config.pipe_width * 0.5,
                    config.ceiling_y + PIPE_OVERDRAW,
                ),
            )
        }

        /// The lower obstacle: from the bottom of the gap down to (and past)
        /// the floor.
        pub fn bottom_aabb(&self, config: &GameConfig) -> Aabb2d {
            let gap_bottom = self.gap_center - config.pipe_gap * 0.5;
            Aabb2d::new(
                Vec2::new(
                    self.x - config.pipe_width * 0.5,
                    config.floor_y - PIPE_OVERDRAW,
                ),
                Vec2::new(self.x + config.pipe_width * 0.5, gap_bottom),
            )
        }
    }

    /// Every tunable number the game plays with, gathered so a test can build
    /// a small, fast variant and `main` can build the real one from the same
    /// type.
    #[derive(Debug, Clone, Copy)]
    pub struct GameConfig {
        pub gravity: f32,
        pub flap_velocity: f32,
        pub floor_y: f32,
        pub ceiling_y: f32,
        pub bird_start: Vec2,
        pub bird_size: Vec2,
        pub pipe_width: f32,
        pub pipe_gap: f32,
        pub pipe_speed: f32,
        pub pipe_spacing: f32,
        pub first_pipe_x: f32,
        pub pipe_count: usize,
        pub gap_center_min: f32,
        pub gap_center_max: f32,
        /// A pipe scrolled left of this x is considered off-screen and gets
        /// recycled to the back of the train.
        pub respawn_x: f32,
    }

    /// A ground plane, expressed the same way a pipe's obstacles are: an
    /// `Aabb2d` reaching far past the edges of the world so the bird cannot
    /// slip past it.
    fn ground_aabb(floor_y: f32) -> Aabb2d {
        const REACH: f32 = 1.0e5;
        Aabb2d::new(Vec2::new(-REACH, -REACH), Vec2::new(REACH, floor_y))
    }

    /// The whole simulation: one bird, a fixed-size pool of recycled pipes, a
    /// score, and a game-over latch.
    pub struct GameState {
        pub bird: Bird,
        pub pipes: Vec<Pipe>,
        pub score: u32,
        pub game_over: bool,
        pub config: GameConfig,
    }

    impl GameState {
        pub fn new(config: GameConfig, rng: &mut impl Rng) -> Self {
            let mut pipes = Vec::with_capacity(config.pipe_count);
            let mut x = config.first_pipe_x;
            for _ in 0..config.pipe_count {
                pipes.push(Pipe {
                    x,
                    gap_center: rng.gen_range(config.gap_center_min..=config.gap_center_max),
                    scored: false,
                });
                x += config.pipe_spacing;
            }

            GameState {
                bird: Bird::new(config.bird_start, config.bird_size),
                pipes,
                score: 0,
                game_over: false,
                config,
            }
        }

        /// Flaps the bird, unless the game has already ended.
        pub fn flap(&mut self) {
            if !self.game_over {
                self.bird.flap(self.config.flap_velocity);
            }
        }

        /// Rebuilds a fresh game from the same config, for the spacebar
        /// restart after a game over.
        pub fn restart(&mut self, rng: &mut impl Rng) {
            *self = GameState::new(self.config, rng);
        }

        /// Advances the simulation by `dt` seconds. A no-op once the game has
        /// ended, so the frozen state a caller reads back is stable.
        pub fn update(&mut self, dt: f32, rng: &mut impl Rng) {
            if self.game_over {
                return;
            }

            self.bird.integrate(dt, self.config.gravity);
            // The ceiling is a soft clamp, not a death: bonking your head is
            // forgiving in the reference game too.
            if self.bird.position.y > self.config.ceiling_y {
                self.bird.position.y = self.config.ceiling_y;
                self.bird.velocity = 0.0;
            }

            for pipe in &mut self.pipes {
                pipe.x -= self.config.pipe_speed * dt;
            }

            // Recycle any pipe that has scrolled off the left edge to the
            // back of the train, with a freshly randomised gap. Walking the
            // running maximum (rather than recomputing it per pipe) keeps
            // multiple same-frame recyclees from landing on top of each
            // other.
            let mut back_x = self
                .pipes
                .iter()
                .map(|pipe| pipe.x)
                .fold(f32::MIN, f32::max);
            for pipe in &mut self.pipes {
                if pipe.x < self.config.respawn_x {
                    back_x += self.config.pipe_spacing;
                    pipe.x = back_x;
                    pipe.gap_center =
                        rng.gen_range(self.config.gap_center_min..=self.config.gap_center_max);
                    pipe.scored = false;
                }
            }

            let bird_aabb = self.bird.aabb();
            if bird_aabb.intersects(&ground_aabb(self.config.floor_y)) {
                self.game_over = true;
            }
            for pipe in &self.pipes {
                if bird_aabb.intersects(&pipe.top_aabb(&self.config))
                    || bird_aabb.intersects(&pipe.bottom_aabb(&self.config))
                {
                    self.game_over = true;
                }
            }

            for pipe in &mut self.pipes {
                if !pipe.scored && pipe.x + self.config.pipe_width * 0.5 < self.bird.position.x {
                    pipe.scored = true;
                    self.score += 1;
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        fn rng() -> StdRng {
            StdRng::seed_from_u64(7)
        }

        /// Small, fast numbers — not the real game's tuning — chosen so tests
        /// can place pipes and the bird with simple round coordinates.
        fn config() -> GameConfig {
            GameConfig {
                gravity: -10.0,
                flap_velocity: 5.0,
                floor_y: -5.0,
                ceiling_y: 5.0,
                bird_start: Vec2::new(0.0, 0.0),
                bird_size: Vec2::new(0.5, 0.5),
                pipe_width: 1.0,
                pipe_gap: 2.0,
                pipe_speed: 10.0,
                pipe_spacing: 5.0,
                first_pipe_x: 3.0,
                pipe_count: 2,
                gap_center_min: -2.0,
                gap_center_max: 2.0,
                respawn_x: -6.0,
            }
        }

        #[test]
        fn gravity_pulls_the_bird_down_over_time() {
            let mut bird = Bird::new(Vec2::ZERO, Vec2::splat(0.5));
            bird.integrate(1.0, -10.0);
            assert_eq!(bird.velocity, -10.0);
            assert_eq!(bird.position.y, -10.0);
        }

        #[test]
        fn flapping_sets_an_upward_velocity_regardless_of_current_fall_speed() {
            let mut bird = Bird::new(Vec2::ZERO, Vec2::splat(0.5));
            bird.integrate(1.0, -10.0);
            bird.flap(5.0);
            assert_eq!(bird.velocity, 5.0);
        }

        #[test]
        fn integrate_uses_the_updated_velocity_within_the_same_step() {
            let mut bird = Bird::new(Vec2::ZERO, Vec2::splat(0.5));
            bird.velocity = 2.0;
            bird.integrate(0.5, -4.0);
            // velocity: 2.0 + (-4.0 * 0.5) = 0.0
            assert_eq!(bird.velocity, 0.0);
            // position: 0.0 + (new velocity 0.0) * 0.5 = 0.0
            assert_eq!(bird.position.y, 0.0);
        }

        #[test]
        fn falling_past_the_floor_ends_the_game() {
            let mut game = GameState::new(config(), &mut rng());
            game.bird.position.y = config().floor_y - 10.0;
            game.update(0.0, &mut rng());
            assert!(game.game_over);
        }

        #[test]
        fn once_game_over_further_updates_are_no_ops() {
            let mut game = GameState::new(config(), &mut rng());
            game.bird.position.y = config().floor_y - 10.0;
            game.update(0.0, &mut rng());
            assert!(game.game_over);

            let frozen = game.bird.position;
            game.update(1.0, &mut rng());
            assert_eq!(game.bird.position, frozen);
        }

        #[test]
        fn flap_does_nothing_after_game_over() {
            let mut game = GameState::new(config(), &mut rng());
            game.bird.position.y = config().floor_y - 10.0;
            game.update(0.0, &mut rng());
            assert!(game.game_over);

            let velocity_before = game.bird.velocity;
            game.flap();
            assert_eq!(game.bird.velocity, velocity_before);
        }

        #[test]
        fn a_bird_inside_the_gap_does_not_collide_with_either_pipe() {
            let mut game = GameState::new(config(), &mut rng());
            game.pipes[0].x = 0.0;
            game.pipes[0].gap_center = 0.0;
            game.bird.position = Vec2::new(0.0, 0.0);

            game.update(0.0, &mut rng());
            assert!(!game.game_over);
        }

        #[test]
        fn a_bird_hitting_a_pipe_ends_the_game() {
            let mut game = GameState::new(config(), &mut rng());
            game.pipes[0].x = 0.0;
            game.pipes[0].gap_center = 3.0; // gap is well above the bird
            game.bird.position = Vec2::new(0.0, 0.0);

            game.update(0.0, &mut rng());
            assert!(game.game_over);
        }

        #[test]
        fn score_increments_exactly_once_per_pipe_passed() {
            let mut game = GameState::new(config(), &mut rng());
            // Just behind the bird (bird_start.x == 0.0), but not yet past
            // this test's respawn boundary of -6.0.
            game.pipes[0].x = -3.0;

            game.update(0.0, &mut rng());
            assert_eq!(game.score, 1);

            game.update(0.0, &mut rng());
            assert_eq!(game.score, 1, "an already-scored pipe must not score again");
        }

        #[test]
        fn pipes_recycle_off_the_left_edge_with_a_fresh_gap() {
            let mut game = GameState::new(config(), &mut rng());
            game.pipes[0].x = config().respawn_x - 1.0;
            game.pipes[0].scored = true;
            let other_pipe_x = game.pipes[1].x;

            game.update(0.0, &mut rng());

            assert!(
                game.pipes[0].x > other_pipe_x,
                "a recycled pipe moves to the back of the train"
            );
            assert!(!game.pipes[0].scored, "a recycled pipe can be scored again");
            let cfg = config();
            assert!(game.pipes[0].gap_center >= cfg.gap_center_min - 1e-4);
            assert!(game.pipes[0].gap_center <= cfg.gap_center_max + 1e-4);
        }

        #[test]
        fn restart_resets_score_bird_and_game_over() {
            let mut game = GameState::new(config(), &mut rng());
            game.bird.position.y = config().floor_y - 10.0;
            game.update(0.0, &mut rng());
            assert!(game.game_over);

            game.restart(&mut rng());
            assert!(!game.game_over);
            assert_eq!(game.score, 0);
            assert_eq!(game.bird.position, config().bird_start);
        }
    }
}

/// Concrete alias for the sprite mesh type stored in the scene, so
/// `Scene::get_mut_as` can downcast back to it.
type SpriteMesh = Mesh<BufferGeometry<SimpleVertex>, SpriteMaterial>;

/// Decodes the four bird-flap frames once at startup. Per Task 1's note that
/// the texture cache never evicts, this is the only place these files are
/// read — the render loop only ever clones the already-decoded `Texture`
/// handles it built here.
fn load_bird_frames() -> Vec<Texture> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/assets/flappy_bird");
    (0..4)
        .map(|i| {
            let path = dir.join(format!("f{i}.png"));
            Texture::from_file(&path)
                .unwrap_or_else(|err| panic!("failed to decode {}: {err}", path.display()))
        })
        .collect()
}

const VIEW_HEIGHT: f32 = 10.0;
const BIRD_ANIMATION_FRAME_SECONDS: f32 = 0.12;

fn main() {
    let mut renderer = Renderer::new(RendererOptions {
        width: 480,
        height: 800,
        title: "Neptune — flappy_bird",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x4ec0ca);

    let mut camera =
        OrthographicCamera::from_size(VIEW_HEIGHT * 480.0 / 800.0, VIEW_HEIGHT, -100.0, 100.0);

    let frames = load_bird_frames();
    let bird_height = 1.0_f32;
    let bird_width = bird_height * frames[0].aspect_ratio();

    let config = GameConfig {
        gravity: -16.0,
        flap_velocity: 6.0,
        floor_y: -4.2,
        ceiling_y: 4.6,
        bird_start: Vec2::new(-1.5, 0.0),
        bird_size: Vec2::new(bird_width * 0.8, bird_height * 0.8),
        pipe_width: 0.8,
        pipe_gap: 2.2,
        pipe_speed: 2.4,
        pipe_spacing: 3.0,
        first_pipe_x: 3.0,
        pipe_count: 4,
        gap_center_min: -2.3,
        gap_center_max: 2.7,
        respawn_x: -4.5,
    };

    let mut rng = rand::thread_rng();
    let mut game = GameState::new(config, &mut rng);

    // --- Static/long-lived scene objects, built once. ---
    //
    // Opaque objects (pipes, ground) go in before the alpha-blended bird and
    // text, since the renderer does not depth- or alpha-sort: draw order is
    // insertion order. Pipes are added before the ground so the ground reads
    // as sitting in front of the pipes' lower ends, as if they were planted
    // in it.
    let pipe_color = MeshBasicMaterial::new(Color::hex(0x4caf50));
    let mut pipe_ids: Vec<(ObjectId, ObjectId)> = Vec::with_capacity(game.pipes.len());
    for _ in 0..game.pipes.len() {
        let top_id = scene.add(Mesh::new(PlaneGeometry::new(1.0, 1.0), pipe_color));
        let bottom_id = scene.add(Mesh::new(PlaneGeometry::new(1.0, 1.0), pipe_color));
        pipe_ids.push((top_id, bottom_id));
    }

    let mut ground = Mesh::new(
        PlaneGeometry::new(40.0, 3.0),
        MeshBasicMaterial::new(Color::hex(0xded895)),
    );
    ground.transform.position = Vec3::new(0.0, config.floor_y - 1.5, 0.0);
    scene.add(ground);

    let mut bird_mesh = Mesh::new(
        PlaneGeometry::new(bird_width, bird_height),
        SpriteMaterial::new(frames[0].clone()),
    );
    bird_mesh.transform.position = Vec3::new(config.bird_start.x, config.bird_start.y, 0.0);
    let bird_id = scene.add(bird_mesh);

    let atlas = Font::system_default()
        .and_then(|font| font.atlas(64.0))
        .expect("a system font is available");

    let mut score_text = TextMesh::with_color(atlas.clone(), "Score: 0", Color::WHITE);
    score_text.transform.position = Vec3::new(-2.6, 4.0, 0.0);
    score_text.transform.scale = Vec3::splat(0.9);
    let score_id = scene.add(score_text);

    let mut game_over_text = TextMesh::with_color(atlas, "GAME OVER - SPACE", Color::hex(0xff5555));
    game_over_text.transform.position = Vec3::new(-2.3, 0.0, 0.0);
    game_over_text.transform.scale = Vec3::splat(0.3);
    game_over_text.visible = false;
    let game_over_id = scene.add(game_over_text);

    // --- Per-frame mutable state the closure captures. ---
    let mut animation_timer = 0.0f32;
    let mut animation_frame = 0usize;
    let mut last_displayed_score = 0u32;

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        camera.set_view_height(VIEW_HEIGHT, frame.aspect_ratio());

        let dt = frame.delta_seconds();
        if frame.input().just_pressed(KeyCode::Space) {
            if game.game_over {
                game.restart(&mut rng);
            } else {
                game.flap();
            }
        }
        game.update(dt, &mut rng);

        // Sync the bird: position from the simulation, a small tilt from
        // vertical speed, and a flip through the four flap frames.
        if let Some(bird) = scene.get_mut(bird_id) {
            let transform = bird.transform_mut();
            transform.position.x = game.bird.position.x;
            transform.position.y = game.bird.position.y;
            transform.rotation.z = (game.bird.velocity * 0.06).clamp(-0.6, 1.0);
        }

        if !game.game_over {
            animation_timer += dt;
            if animation_timer >= BIRD_ANIMATION_FRAME_SECONDS {
                animation_timer -= BIRD_ANIMATION_FRAME_SECONDS;
                animation_frame = (animation_frame + 1) % frames.len();
                if let Some(bird) = scene.get_mut_as::<SpriteMesh>(bird_id) {
                    bird.material.texture = frames[animation_frame].clone();
                }
            }
        }

        // Sync every pipe pair's transform from its logical rectangle. The
        // base geometry is a static 1x1 unit quad; only position and scale
        // change, so no geometry is ever rebuilt after startup.
        for (pipe, &(top_id, bottom_id)) in game.pipes.iter().zip(pipe_ids.iter()) {
            let top = pipe.top_aabb(&game.config);
            if let Some(obj) = scene.get_mut(top_id) {
                let transform = obj.transform_mut();
                let center = top.center();
                let size = top.size();
                transform.position = Vec3::new(center.x, center.y, 0.0);
                transform.scale = Vec3::new(size.x, size.y, 1.0);
            }

            let bottom = pipe.bottom_aabb(&game.config);
            if let Some(obj) = scene.get_mut(bottom_id) {
                let transform = obj.transform_mut();
                let center = bottom.center();
                let size = bottom.size();
                transform.position = Vec3::new(center.x, center.y, 0.0);
                transform.scale = Vec3::new(size.x, size.y, 1.0);
            }
        }

        if game.score != last_displayed_score {
            last_displayed_score = game.score;
            if let Some(text) = scene.get_mut_as::<TextMesh>(score_id) {
                text.set_text(&format!("Score: {}", game.score));
            }
        }

        if let Some(text) = scene.get_mut_as::<TextMesh>(game_over_id) {
            text.visible = game.game_over;
        }

        frame.render(&scene, &camera);
    });
}
