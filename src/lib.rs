//! # Neptune
//!
//! A Three.js-inspired 3D graphics engine, written in Rust on top of
//! [`vulkano`]. Neptune is pedagogical: it exists to show that Rust's
//! ownership, borrowing and lifetime rules map cleanly onto the very real
//! problem of managing GPU resources.

mod backend;

pub mod cameras;
pub mod core;
pub mod geometry;
pub mod input;
pub mod lights;
pub mod materials;
pub mod math;
pub mod objects;
pub mod renderer;
