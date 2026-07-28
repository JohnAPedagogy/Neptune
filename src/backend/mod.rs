//! Everything Vulkan.
//!
//! This module is private and has **no** `pub` re-exports: nothing in
//! Neptune's public API mentions a Vulkano type, so a consumer never writes
//! `use vulkano::...`. That encapsulation is the point — swap what is in here
//! and the API above it is unchanged.

pub(crate) mod command;
pub(crate) mod context;
pub(crate) mod pass;
pub(crate) mod screenshot;
pub(crate) mod shaders;
pub(crate) mod surface;
pub(crate) mod texture;
pub(crate) mod upload;
