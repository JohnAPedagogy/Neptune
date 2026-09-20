use std::path::{Path, PathBuf};

const CIRCUIT_ASSETS_DIR: &str = "assets/circuit";

/// The crate's asset root: prefers the current working directory if it
/// already has an `assets/` folder (the common case — running from `smk/`
/// directly), and falls back to the compiled-in manifest directory
/// otherwise (e.g. when invoked via `cargo run --manifest-path ...` from
/// elsewhere). Preferring CWD over an always-baked-in `CARGO_MANIFEST_DIR`
/// keeps a relocated build of this binary working if run from beside its
/// own `assets/` folder.
pub fn asset_root() -> PathBuf {
    let cwd_assets = Path::new("assets");
    if cwd_assets.is_dir() {
        PathBuf::new()
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }
}

/// Path to a circuit's top-down ground texture under `root`, e.g.
/// `<root>/assets/circuit/donut_plains_1/base.png`. Takes the root
/// explicitly so this stays a pure, trivially-testable function — see
/// `asset_root()` for how callers resolve `root` in practice.
pub fn base_png_path(root: &Path, circuit: &str) -> PathBuf {
    root.join(CIRCUIT_ASSETS_DIR).join(circuit).join("base.png")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_png_path_joins_the_circuit_assets_dir_and_filename() {
        assert_eq!(
            base_png_path(Path::new(""), "donut_plains_1"),
            PathBuf::from("assets/circuit/donut_plains_1/base.png")
        );
    }

    #[test]
    fn base_png_path_works_for_a_different_circuit_name() {
        assert_eq!(
            base_png_path(Path::new("/some/root"), "rainbow_road"),
            PathBuf::from("/some/root/assets/circuit/rainbow_road/base.png")
        );
    }
}
