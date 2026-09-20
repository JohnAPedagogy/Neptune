# ncubed

A reproduction of The Cherno's multiplayer game "Cubed" as a Rust application
on the Neptune engine. The goal is the **player experience**: a window showing
a 3D plane with one cube per user currently logged in to the game. Design,
plans, and gaps are tracked in
`cubed00.md` (`resources/ee/dds/006 Special AInWebGraphics/006.6 Graphics/Neptune/cubed00.md`).

## What you see and do

- A flat plane in a 3D perspective view.
- One cube per connected player. Your cube is magenta; everyone else's cycles
  a small palette so players are distinguishable.
- WASD moves your cube; every other cube tracks its player's movements.
- When someone connects, a cube appears; when they leave, it disappears
  (the server drops players silent for ~3 s, so a cube vanishes a moment
  after its client dies).
- A fixed elevated camera so the whole arena is visible.
- Press Escape to quit.

## Workspace layout

```
ncubed/
  Cargo.toml               # workspace
  crates/
    ncubed-common/         # PlayerId, PlayerState, Packet; serde + postcard wire format
    ncubed-server/         # headless: no window, no Neptune. Binds UDP, ~30 Hz snapshots
    ncubed-client/         # Neptune app: window, plane, cube pool, WASD, network thread
```

The server is deliberately independent of the render engine (a server has no
window). The only crate both sides depend on is `ncubed-common`, which owns the
`Packet` enum:

```rust
pub enum Packet {
    Welcome(PlayerId),
    Players { players: Vec<(PlayerId, PlayerState)> },
}
```

One UDP datagram per packet, serialised with `serde` + `postcard`. The wire
format is exercised by round-trip unit tests in `ncubed-common`.

## How to run

From the `ncubed` workspace root:

```text
# 1. Start the server (default 0.0.0.0:8192)
cargo run -p ncubed-server

# 2. In other terminals, one client per player
cargo run -p ncubed-client            # connects to 127.0.0.1:8192
cargo run -p ncubed-client 127.0.0.1:9000   # custom server address
cargo run -p ncubed-server 0.0.0.0:9000     # server on another port
```

Each client prints `connected - you are player N` once it has been welcomed.

Two clients running side by side is the definition of done (cubed00.md §6,
M3): each window shows the other client's cube as a 3D block alongside its own.

## Tests

```text
cargo test --workspace
```

Covers wire-format round-trips (`ncubed-common`), the server state machine
(welcome/update/prune) **and** a real loopback UDP handshake
(`ncubed-server`), and the client's cube-pool rendering logic as part of the
build (`ncubed-client`).

## What was reproduced vs. deliberately left out

Reproduced: a screen of blocks, one per logged-in user; WASD movement with the
reference feel (`speed = 150`, velocity damped by `10 * dt`); join/leave
appear/disappear; a headless authoritative server; channels instead of locks
on the render path.

Left out (deliberately — see cubed00.md §2.3): nothing from the C++ internals.
No GameNetworkingSockets, no Walnut layers, no ImGui, no hand-rolled Vulkan,
no textures, no real meshes, no chat, no colour/kick/history menus.

Engine boundaries, not failures (cubed00.md §5.1): Neptune has no networking
(G1, handled at the game layer), no headless mode (G2, the server never
imports Neptune), and no `Scene::remove` (G3, worked around with a cube pool
that toggles `visible`).
