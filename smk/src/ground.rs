/// Half-width baseline (in world units) used as the world-space scale for a
/// circuit's ground plane.
pub const WORLD_SIZE: f32 = 40.0;

/// Computes aspect-ratio-preserving (width, height) dimensions, in world
/// units, for a ground plane whose largest side is `world_size`.
pub fn ground_plane_size(texture_aspect_ratio: f32, world_size: f32) -> (f32, f32) {
    if texture_aspect_ratio >= 1.0 {
        (world_size, world_size / texture_aspect_ratio)
    } else {
        (world_size * texture_aspect_ratio, world_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_texture_gives_a_square_plane() {
        assert_eq!(ground_plane_size(1.0, 40.0), (40.0, 40.0));
    }

    #[test]
    fn wide_texture_shrinks_the_height() {
        assert_eq!(ground_plane_size(2.0, 40.0), (40.0, 20.0));
    }

    #[test]
    fn tall_texture_shrinks_the_width() {
        assert_eq!(ground_plane_size(0.5, 40.0), (20.0, 40.0));
    }
}
