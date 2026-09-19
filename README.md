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
| ui_demo | `cargo run --example ui_demo` | `orbit_cube` plus draggable/dockable control panels (dat.gui-style widgets) |

All examples support a one-shot screenshot mode for documentation:

```text
NEPTUNE_SCREENSHOT=out.png NEPTUNE_SCREENSHOT_AFTER=1.5 cargo run --example hello_cube
```

## Tracing (debug logging)

`ui_demo` is wired up with [`tracing`](https://docs.rs/tracing), off by
default. Enable it with the `RUST_LOG` env var, filtered to this crate's
target:

```text
RUST_LOG=neptune=trace cargo run --example ui_demo
```

That prints one nested span per `render_loop` iteration, covering:

- `draw_frame` — the whole per-frame render call
- `record_scene` / `record_draw` — walking the scene graph and recording one object's draw call
- `record_ui` — recording the UI draw list (tagged with its primitive count)
- `get_or_create` / `get_or_create_ui_pipeline` — pipeline cache lookups, with the compile time visible whenever a new `GraphicsPipeline` has to be built (tagged with the `MaterialId` for the scene-pipeline cache)

Each line has `time.busy`/`time.idle`, so a slow frame or an unexpected
pipeline (re)compile shows up directly. Output goes to **stdout**, not
stderr. Any `RUST_LOG` level below `trace` (e.g. `neptune=debug` or
`neptune=info`) stays silent — the spans are pinned to `trace` on purpose, so
this instrumentation never appears unless explicitly asked for. To trace a
narrower slice, target a specific module instead of the whole crate, e.g.
`RUST_LOG=neptune::backend::command=trace`.
