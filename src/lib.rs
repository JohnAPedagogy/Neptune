//! # Neptune
//!
//! A Three.js-inspired 3D graphics engine, written in Rust on top of
//! [`vulkano`]. Neptune is pedagogical: it exists to show that Rust's
//! ownership, borrowing and lifetime rules map cleanly onto the very real
//! problem of managing GPU resources.
//!
//! ```no_run
//! use neptune::prelude::*;
//!
//! let mut renderer = Renderer::new(RendererOptions {
//!     width: 1280,
//!     height: 720,
//!     title: "Neptune Demo",
//! });
//!
//! let mut scene = Scene::new();
//! let camera = PerspectiveCamera::new(75.0_f32.to_radians(), 1280.0 / 720.0, 0.1, 1000.0);
//!
//! let mut cube = Mesh::new(
//!     BoxGeometry::new(1.0, 1.0, 1.0),
//!     MeshBasicMaterial::new(Color::hex(0x00ff88)),
//! );
//! cube.transform.position.z = -5.0;
//! let cube_id = scene.add(cube);
//!
//! renderer.render_loop(move |frame| {
//!     if let Some(cube) = scene.get_mut(cube_id) {
//!         cube.transform_mut().rotation.y += frame.delta_seconds();
//!     }
//!     frame.render(&scene, &camera);
//! });
//! ```
//!
//! ## How it is put together
//!
//! - [`core`] — the scene graph: [`Scene`](core::Scene),
//!   [`Object3D`](core::Object3D), [`Group`](core::Group).
//! - [`objects`] — [`Mesh`](objects::Mesh), the thing you actually add.
//! - [`geometry`] — vertex data and the built-in shape constructors.
//! - [`materials`] — how surfaces are shaded, including textures.
//! - [`cameras`] — perspective and orthographic viewpoints.
//! - [`lights`] — light source types.
//! - [`math`] — colour, transforms, 2D collision, and `glam` re-exports.
//! - [`input`] — keyboard state.
//! - [`text`] — fonts, glyph atlases, and text meshes.
//! - [`renderer`] — the window and the frame loop.
//!
//! Everything Vulkan lives in a private `backend` module. No public type in
//! this crate mentions a Vulkano type, which is exactly the encapsulation
//! lesson the engine is here to teach.

mod backend;

pub mod cameras;
pub mod core;
pub mod geometry;
pub mod input;
pub mod lights;
pub mod materials;
pub mod math;
pub mod objects;
pub mod prelude;
pub mod renderer;
pub mod text;
