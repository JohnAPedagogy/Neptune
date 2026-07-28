//! What the renderer needs from anything it renders through.

use crate::math::Mat4;

/// A viewpoint: where you are looking from, and how the world is projected.
pub trait Camera {
    /// World space to camera space.
    fn view_matrix(&self) -> Mat4;

    /// Camera space to clip space. Neptune uses a right-handed world with a
    /// `0..1` depth range, matching Vulkan.
    fn proj_matrix(&self) -> Mat4;

    /// The combined matrix the renderer actually pushes to the GPU.
    fn view_proj_matrix(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }
}

impl<C: Camera + ?Sized> Camera for &C {
    fn view_matrix(&self) -> Mat4 {
        (**self).view_matrix()
    }

    fn proj_matrix(&self) -> Mat4 {
        (**self).proj_matrix()
    }
}
