/// `map/map.h`'s `Map::MODE7_FOV_HALF`.
pub const MODE7_FOV_HALF: f32 = 0.5;

pub const CAMERA_NEAR: f32 = 0.1;
pub const CAMERA_FAR: f32 = 1000.0;

/// Converts Mode7's half-FOV parameter to a real perspective camera's
/// vertical field of view, in radians (`kart02nport.md` §4.1).
pub fn mode7_half_fov_to_vertical_fov(mode7_fov_half: f32) -> f32 {
    2.0 * mode7_fov_half.atan()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_fov_of_one_gives_a_right_angle_fov() {
        // atan(1.0) = pi/4, doubled = pi/2 (90 degrees) exactly.
        let fov = mode7_half_fov_to_vertical_fov(1.0);
        assert!(
            (fov - std::f32::consts::FRAC_PI_2).abs() < 1e-6,
            "fov was {fov}"
        );
    }

    #[test]
    fn the_games_mode7_fov_half_converts_to_about_53_degrees() {
        let fov = mode7_half_fov_to_vertical_fov(MODE7_FOV_HALF);
        assert!((fov - 0.9272952).abs() < 1e-5, "fov was {fov}");
        assert!((fov.to_degrees() - 53.13).abs() < 0.01);
    }

    #[test]
    fn a_smaller_half_fov_gives_a_narrower_fov() {
        assert!(mode7_half_fov_to_vertical_fov(0.3) < mode7_half_fov_to_vertical_fov(0.5));
    }
}
