use std::path::{Path, PathBuf};

const CIRCUIT_ASSETS_DIR: &str = "assets/circuit";

/// Path to a circuit's top-down ground texture, e.g.
/// `assets/circuit/donut_plains_1/base.png`.
pub fn base_png_path(circuit: &str) -> PathBuf {
    Path::new(CIRCUIT_ASSETS_DIR).join(circuit).join("base.png")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_png_path_joins_the_circuit_assets_dir_and_filename() {
        assert_eq!(
            base_png_path("donut_plains_1"),
            PathBuf::from("assets/circuit/donut_plains_1/base.png")
        );
    }

    #[test]
    fn base_png_path_works_for_a_different_circuit_name() {
        assert_eq!(
            base_png_path("rainbow_road"),
            PathBuf::from("assets/circuit/rainbow_road/base.png")
        );
    }
}
