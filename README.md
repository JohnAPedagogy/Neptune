# Neptune

A 3D render engine inspired by Three.js, built on Rust and Vulkano.

## Running the examples

Each example is an interactive window. Press `Escape` to quit.

| Example | Command | What it shows |
|---|---|---|
| hello_cube | `cargo run --example hello_cube` | A spinning cube — the engine's smoke test |
| hello_sprite | `cargo run --example hello_sprite` | 2D: an orthographic camera, a textured quad, and text |
| orbit_cube | `cargo run --example orbit_cube` | The orbit camera: left-drag rotates, scroll zooms, right-drag pans |
| flappy_bird | `cargo run --example flappy_bird` | A complete, playable Flappy Bird (Space flaps, Escape quits) |

All examples support a one-shot screenshot mode for documentation:

```text
NEPTUNE_SCREENSHOT=out.png NEPTUNE_SCREENSHOT_AFTER=1.5 cargo run --example hello_cube
```
