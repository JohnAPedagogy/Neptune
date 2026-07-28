# Neptune Engine + Flappy Bird Example + Documentation — Implementation Plan

## Context

Neptune is a pedagogical, Three.js-inspired 3D graphics engine written in Rust over `vulkano`,
designed as a companion project for the book *Rust the Hard Parts*. Its architecture is already
fully specified in two design docs in the (separate, non-git) book source tree:

- `resources/ee/dds/005 Dev/005.13 Specific Languages/rustut/hardparts/arch/neptune_plan.md` —
  base architecture: module structure (§3), dependency choices (§4), ownership model (§5),
  public API shape (§7).
- `resources/ee/dds/005 Dev/005.13 Specific Languages/rustut/hardparts/arch/neptune2d.md` — gap
  analysis for making Neptune capable of a Flappy-Bird-shaped 2D game, with P0/P1/P2/P3
  priorities and, per gap, either a recommended 3rd-party crate or a DIY approach.

This plan builds the REAL engine (not stubs) in this repo (`Lenovo/Neptune`, a fresh repo
currently holding only `LICENSE`/`README.md`), implements a Flappy Bird example on top of it,
and writes two documentation additions back into the book source tree.

**Reference implementation to model the Vulkan backend on:** a separate, already-verified,
compiling single-file Vulkano 0.35.2 + winit 0.30.13 triangle program exists at
`C:\work\Favorites\Lenovo\ee0\revised\pblog\unleashed\triangle\src\main.rs` (a sibling repo,
`Lenovo/ee0`). It is confirmed via `cargo check`/`cargo build`/live-run to compile and render
with the CURRENT (0.35.2 / 0.30.13) API shapes, which differ from older/memorized Vulkano
examples in several load-bearing ways (see Global Constraints). Treat that file as ground truth
for backend init/render-loop code shape, not as something to copy verbatim (it's a flat
single-file demo; Neptune needs the same operations split across `neptune_plan.md` §3's module
tree, with a real public API in front of them).

Flappy Bird sprite assets (4 bird-flap frames) are available at:
`C:\work\Favorites\Lenovo\ee0\revised\grfx\game\bevy\ballgame\fb\` — `f0.png`, `f1.png`, `f2.png`,
`f3.png` (ignore the stray `f3.jpg` duplicate; use the `.png` versions for consistency). There is
no pipe sprite, background, ground, or font asset provided — pipes and ground are solid-color
geometry (`MeshBasicMaterial`), and score text uses Neptune's `ab_glyph`-based text rendering
(no font file needed to source — `ab_glyph` can load a system font or a small bundled one; see
Task 2).

## Global Constraints

Copy these into every task dispatch verbatim where relevant — they bind all four tasks.

1. **Rust edition 2024**, crate name `neptune` for the engine, matching the verified triangle
   reference's `Cargo.toml` shape (`edition = "2024"`).
2. **Vulkano 0.35.2 + winit 0.30.13 API shapes only.** Do NOT use memorized older-Vulkano
   patterns — several are removed/changed in these versions:
   - `GraphicsPipeline::new(device, cache, GraphicsPipelineCreateInfo { .. })` — the old
     `GraphicsPipeline::start()...build()` fluent builder no longer exists.
   - winit 0.30 requires implementing the `ApplicationHandler` trait and calling
     `event_loop.run_app(&mut app)` — there is no more `EventLoop::run(closure)`.
   - `vulkano::swapchain::Surface::from_window(instance, window)` — no separate `vulkano-win`
     crate is needed or should be added.
   - Buffers: `Buffer::from_iter`/`Buffer::from_data` + `BufferCreateInfo` + `AllocationCreateInfo`
     via a `StandardMemoryAllocator`; command buffers via `StandardCommandBufferAllocator`.
   - When in doubt about exact API shape, read
     `C:\work\Favorites\Lenovo\ee0\revised\pblog\unleashed\triangle\src\main.rs` (already
     `cargo check`-clean, fully commented with which Vulkan "step" each block implements) rather
     than reconstructing from general Vulkan/Vulkano knowledge.
3. **Build environment (already solved on this machine, nothing to redo):**
   `vulkano-shaders` pulls in `shaderc-sys`, which compiles a C++ library from source on first
   build. Two Windows/MSVC fixes are ALREADY set as persistent user-level environment variables
   on this machine: `CMAKE_POLICY_VERSION_MINIMUM=3.5` and `CARGO_TARGET_DIR=D:\Programs\vkb`.
   Implementers do not need to set these themselves, but build output for EVERY crate on this
   machine goes to `D:\Programs\vkb`, not a project-local `target/` — don't be confused if no
   local `target/` directory appears after a build; check `D:\Programs\vkb` instead, or run
   `cargo build` and trust its own reported paths in output.
4. **Module structure** follows `arch/neptune_plan.md` §3 exactly: `core/`, `cameras/`,
   `geometry/`, `materials/`, `objects/`, `lights/`, `math/`, `renderer/`, `backend/`,
   `examples/`, plus `lib.rs` and `prelude.rs`. `backend/` is private — no `pub` items are
   re-exported from it anywhere. Consumers only ever `use neptune::prelude::*`.
5. **Dependency choices are already decided — do not substitute or add alternatives:**
   `glam` (math: `Vec2`/`Vec3`/`Mat4`/`Quat`), `bytemuck` (Pod casts for GPU upload), `vulkano`
   0.35.2 + `vulkano-shaders` 0.35.0 (rendering), `winit` 0.30.13 (window/input), `image` (texture
   decoding), `ab_glyph` (text rasterization). Do NOT add `bevy_ecs`, `hecs`, `specs`, or any ECS
   crate, and do NOT add `parry2d`/`rapier2d` — per `neptune2d.md`'s own reasoning, entity
   management stays `Vec<Box<dyn Object3D>>` + `Any`/`downcast_mut`, and collision stays a
   hand-rolled `Aabb2d` (both are deliberate teaching content, not gaps to fill with a library).
6. **Public API shape** must match `neptune_plan.md` §2 (Three.js mapping table) and §7 (end-state
   example) — `Scene::new()`, `scene.add(mesh)`, `Renderer::new(opts)`,
   `renderer.render_loop(|frame| { frame.render(&scene, &camera); })`,
   `PerspectiveCamera::new(fov, aspect, near, far)`, `BoxGeometry::new(w, h, d)`,
   `MeshBasicMaterial::new(color)`, `mesh.transform.position`/`.rotation`, `Color::hex(0xrrggbb)`.
   A consumer's `examples/*.rs` file should never contain `use vulkano::...`.
7. **Testing bar:** every module gets at minimum a `cargo check` pass. Pure-data types with no
   GPU dependency (`Color`, `Transform`, `Aabb2d` and its `intersects()`, delta-time helpers) get
   real `#[test]` unit tests, per `neptune_plan.md` §9 Layer 1 — no GPU required for these. Full
   headless-PPM-snapshot rendering (§9 Layer 2) is explicitly OUT OF SCOPE for this plan — the
   verification bar for anything GPU-touching is: it compiles, and launching the built example
   binary produces a live window that runs for several seconds without panicking (the same
   bar used to verify the triangle reference — see Task 1's Verification section for the exact
   command pattern).
8. **Commit as you go** — small, reviewable commits per logical unit of work (e.g., one commit
   for module scaffolding, one per major feature), not one giant commit per task.
9. Tasks 3 and 4 write into a **separate, non-git directory**
   (`C:\work\Favorites\resources\ee\dds\005 Dev\005.13 Specific Languages\rustut\hardparts\` and
   its `arch/` subfolder) that is NOT part of this repository. Do not attempt `git add`/`git
   commit` for those files — there is no git repo there. Edit the files directly at their
   absolute paths.

---

## Task 1: Neptune engine core — `.gitignore` + P0/P1 2D-capable 3D engine

**Where:** this repo (`Lenovo/Neptune`), on the current branch.

### Part A — `.gitignore`

Add a standard Rust `.gitignore` at the repo root (the usual `cargo new` template plus common
editor/OS cruft: `/target`, `Cargo.lock` is fine to KEEP tracked since this is an application-like
workspace with examples people will want reproducible builds for — track `Cargo.lock`, ignore
`/target`, `*.pdb`, `.vscode/` if not already meaningfully configured, `.DS_Store`).

### Part B — Engine scaffold + real rendering (mirrors `neptune_plan.md` §3, §7)

Create the `neptune` crate (`cargo init --lib` at repo root, edition 2024) with this module tree
and, in each module, real (not `todo!()`-stubbed) implementations:

- **`math/`**: `Color { r, g, b, a: f32 }` with `Color::new(r,g,b)`, `Color::hex(0xrrggbb)`,
  `Color::RED` etc. constants; `Transform { position: Vec3, rotation: Vec3, scale: Vec3 }` with
  `Transform::matrix() -> Mat4` (built via `glam`). `#[test]` coverage for both.
- **`core/`**: `Object3D` trait (`fn transform(&self) -> &Transform`, `fn transform_mut(&mut self)
  -> &mut Transform`, plus whatever `downcast`-friendly bound is needed — implement
  `std::any::Any` support so a later `query_mut::<T>()` helper is possible, though the helper
  itself is optional/stretch, not required for this task). `Scene { objects: Vec<Box<dyn
  Object3D>> }` with `new()`, `add(obj: impl Object3D + 'static)`, `get(&self, id) -> Option<&dyn
  Object3D>`, `get_mut(&mut self, id) -> Option<&mut dyn Object3D>`. `Group` container (named,
  holds child `Object3D`s) per §3 — minimal is fine (a `Vec` + name field).
- **`cameras/`**: `Camera` trait (`fn view_matrix(&self) -> Mat4`, `fn proj_matrix(&self) ->
  Mat4`). `PerspectiveCamera::new(fov, aspect, near, far)`. **P1 gap from neptune2d.md §1:**
  `OrthographicCamera` mirroring `PerspectiveCamera`, built on `glam::Mat4::orthographic_rh`.
- **`geometry/`**: `Vertex` trait + `SimpleVertex { position: [f32;3], normal: [f32;3], uv:
  [f32;2] }` (`#[repr(C)]`, `bytemuck::Pod`/`Zeroable`, plus whatever Vulkano vertex-format
  derive the verified triangle reference uses). `BufferGeometry<V: Vertex> { vertices: Vec<V>,
  indices: Vec<u32> }`. `BoxGeometry::new(w,h,d)`, `SphereGeometry::new(r, w_segments,
  h_segments)`, `PlaneGeometry::new(w,h)` as constructors producing `BufferGeometry<SimpleVertex>`
  with correct vertex/index data (a plane is also what 2D sprites/pipes render as — reuse it,
  don't build a separate 2D-only quad type).
- **`materials/`**: `Material` trait (`fn material_id(&self) -> MaterialId` or similar,
  `fn bind(...)` hook the renderer needs). `MeshBasicMaterial::new(Color)` (flat, unlit).
  **P1 gap from neptune2d.md §1:** a textured/sprite material variant (name it
  `SpriteMaterial` or extend `MeshBasicMaterial` with an optional texture — implementer's
  choice, document which) that decodes an image file via the `image` crate and uploads it as a
  Vulkano sampled texture, bound in the fragment shader instead of (or blended with) the flat
  color. This is the foundation Task 2's bird sprites and Task 1's own P1 text rendering both
  build on.
- **`objects/`**: `Mesh<G: Geometry, M: Material>` (geometry + material + `Transform`,
  `impl Object3D`).
- **`lights/`**: `Light` trait, `AmbientLight::new(color, intensity)`,
  `DirectionalLight::new(color, intensity)`. (Renderer wiring for lighting can be minimal/no-op
  in the shader for this task — the *types* must exist and be constructible per the public API
  table; a shippable unlit-only render path is acceptable, lighting math itself is not blocking.)
- **`renderer/`**: `Renderer { .. }` with **zero public Vulkano types** in its signature —
  `Renderer::new(RendererOptions { width, height, title })`,
  `renderer.render_loop(|frame| { .. })` (an `FnMut(&mut Frame)` loop driving a real winit
  `ApplicationHandler`, per Global Constraint #2), `Frame<'a>` (lifetime-bound per-frame handle,
  `frame.render(&scene, &camera)`). `PipelineCache: HashMap<MaterialId, Arc<GraphicsPipeline>>`
  so each material type compiles its pipeline once and reuses it.
- **`backend/`** (private, no `pub` re-exports anywhere): `VulkanContext` (instance, device,
  queues), `SurfaceState` (surface, swapchain, image views, resize handling — model on the
  triangle reference's `recreate_swapchain`), `RenderPass`/framebuffers, `upload.rs` (generic
  `upload_vertices<V: Vertex>`/buffer upload), texture upload path for the P1 `SpriteMaterial`
  (decode via `image`, upload via Vulkano `Image`+sampler).
- **P0 gaps from `neptune2d.md` §1** (all DIY per that doc, no new crates):
  - `input/` module: `InputState` wrapping winit `KeyCode` press/release, exposing
    `just_pressed(key)`/`held(key)`, fed from the `ApplicationHandler`'s `window_event`.
  - Delta-time: tracked via `std::time::Instant` inside the render loop /`Frame`, exposed to the
    `render_loop` closure (e.g. `frame.delta_seconds() -> f32`).
  - `math/collision.rs`: `Aabb2d { min: Vec2, max: Vec2 }` + `intersects(&self, other: &Aabb2d) ->
    bool`. `#[test]` coverage.
- **P1 text rendering** (`text/` module per neptune2d.md §3 module list): `Font` (thin `ab_glyph`
  wrapper — loading a bundled/system TTF is fine, document which font and where it comes from),
  `TextMesh` (rasterizes glyphs to a texture atlas once, renders as textured quads reusing the
  `SpriteMaterial`/textured-material path above). This is what Task 2's score display needs.
- **`prelude.rs`**: curated `pub use` of everything a consumer needs, so `use
  neptune::prelude::*;` alone suffices for the §7 end-state example.
- **`examples/hello_cube.rs`**: reproduce `neptune_plan.md` §7's end-state example nearly
  verbatim (spinning cube, `MeshBasicMaterial`, `PerspectiveCamera`, `render_loop`) — this is the
  smoke test that the whole public API actually holds together.

### Verification

- `cargo check` and `cargo test` clean across the whole crate.
- `cargo build --example hello_cube` succeeds; launch it (e.g. via `Start-Process`, wait a few
  seconds, confirm the process is still running / didn't exit with an error — same bar used to
  verify the triangle reference) and confirm no panic/crash. A human/reviewer cannot see pixels
  render correctly from a diff, so a live-process-survives-N-seconds check plus code review of
  the render-loop logic is the bar, not pixel-perfect visual confirmation.
- Report the exact `cargo build`/run commands used (including any env-var notes) in the task
  report, since Task 2 depends on this crate.

---

## Task 2: Flappy Bird example using the Neptune API

**Depends on:** Task 1 (needs the `neptune` crate and its public API to exist first — do not
start until Task 1's review is clean).

**Where:** `examples/flappy_bird.rs` (plus any small supporting files under `examples/` if truly
needed, e.g. `examples/flappy_bird_assets/` if assets must be copied in — see below) in this
repo, on the current branch.

### Assets

Bird-flap sprite frames exist at
`C:\work\Favorites\Lenovo\ee0\revised\grfx\game\bevy\ballgame\fb\f0.png` through `f3.png` (use
the `.png` files; ignore the stray `f3.jpg`). Copy these four files into this repo (e.g.
`examples/assets/flappy_bird/f0.png`..`f3.png`) rather than referencing the external path, so the
example is self-contained and doesn't break if that sibling repo changes. There is no pipe
sprite, background, ground, or font asset provided:

- **Bird**: use the four PNG frames via Task 1's `SpriteMaterial`, cycling frames for a simple
  flap animation (e.g. advance frame every ~100ms) — this is the neptune2d.md §9
  "sprite-sheet/flipbook animation" P2 item in miniature; a fixed 4-frame cycle is enough, no
  need for a general flipbook system.
- **Pipes**: solid-color rectangles via `PlaneGeometry` + `MeshBasicMaterial` (no texture) —
  matches neptune2d.md's own guidance that flat-color quads are an acceptable placeholder.
- **Ground/background**: a solid-color plane or just a clear color; do not invent new asset
  requirements.
- **Score**: Task 1's `TextMesh`/`ab_glyph` text rendering.

### Game logic

A complete, playable Flappy Bird: bird falls under gravity (using Task 1's delta-time), flaps
upward on spacebar (Task 1's `InputState`), pipes scroll left and spawn with a randomized gap
(`rand` crate — fine to add, it's orthogonal utility, not one of the "already decided" engine
dependencies but also not something neptune2d.md flagged as a gap needing a decision), collision
via Task 1's `Aabb2d::intersects` ends the game, score increments per pipe passed and renders via
`TextMesh`, spacebar restarts after game-over. Use `OrthographicCamera` (Task 1's P1 addition) for
a proper 2D view rather than faking 2D with perspective.

### Verification

- `cargo build --example flappy_bird` succeeds.
- Launch it the same way Task 1's `hello_cube` was verified (live process survives several
  seconds without panicking); additionally, since this has real interactive logic, add
  `#[test]` coverage for the parts that don't need a GPU/window at all — gravity/velocity
  integration math, collision-triggers-game-over logic, score-increment logic — by factoring
  that logic into plain functions/structs the tests can call directly, separate from the
  winit/render plumbing (this also demonstrates the Task 1 architecture is reusable, not just a
  monolith).
- Report the exact commands and what the live-run check showed.

---

## Task 3: `chapter_additions.md` documentation addition

**Where:** `C:\work\Favorites\resources\ee\dds\005 Dev\005.13 Specific Languages\rustut\hardparts\arch\chapter_additions.md`
(absolute path — separate, non-git directory; no branch/commit applies here, edit the file
directly). Read the existing file first — it's a per-chapter workshop content plan with sections
"Neptune Workshop 1" through "Neptune Workshop 32", each tied to a specific book chapter.

**Depends on:** Tasks 1 and 2 (documents what was actually built — read their task reports and
the actual code in this repo before writing, so the addition reflects reality, not aspiration).

**Task:** Add a new section documenting the Neptune2D / Flappy Bird work as an addendum to the
existing per-chapter plan, following the file's existing style (see Workshop 1's structure for
the template: **Chapter:**, **File added/changed:**, **Content:** with code excerpts). Specifically:

1. A short preface note explaining that `arch/neptune2d.md`'s P0/P1 gap-closure plan has now been
   implemented for real in the `Lenovo/Neptune` repo (branch `neptune`), not just planned.
2. One workshop-style entry per major addition actually built in Task 1 (input handling, delta
   time, AABB collision, orthographic camera, textured/sprite material, text rendering) — each
   with a real code excerpt from the actual implementation (not invented pseudocode), and a note
   on which existing book chapter's concept it reinforces (e.g. the textured-material texture
   upload path reinforces Ch15's generic buffer upload; `Aabb2d` reinforces Ch4/Ch12 Copy
   struct math; `InputState` reinforces Ch26 encapsulation — use your judgment matching the
   existing file's per-chapter mapping style, cross-check against `neptune_plan.md` §6's "what
   compiles after each chapter" table for chapter-number plausibility).
3. A closing entry for the Flappy Bird example itself, in the spirit of the existing capstone
   framing (`neptune_plan.md` §8 lists "Flappy Bird with Bevy" as Capstone 2 and "Neptune Engine"
   as Capstone 4) — frame this as "Flappy Bird *on* Neptune" being the concrete proof that
   Capstone 4's engine is capable of what Capstone 2 built on Bevy, without re-adding an ECS (tie
   back to `neptune2d.md`'s explicit reasoning for why Neptune stayed ECS-free).

Match the existing file's heading levels, horizontal-rule section separators, and tone exactly —
a reader should not be able to tell this section was added later.

### Verification

Read the file back; confirm the new section is well-formed Markdown (matching tables/code fences
render correctly) and that every code excerpt quoted actually exists verbatim in the Task 1/2
repo (copy-paste from the real files, don't retype from memory).

---

## Task 4: Flappy Bird tutorial document

**Where:** new file `C:\work\Favorites\resources\ee\dds\005 Dev\005.13 Specific Languages\rustut\hardparts\arch\workshop_flappybird_neptune.md`
(absolute path — same non-git directory as Task 3; create this as a new file).

**Depends on:** Tasks 1 and 2 (this is a tutorial for the real, working example — read the actual
code before writing, same grounding requirement as Task 3).

**Task:** Write a tutorial document, in the same spirit and structure as the existing
`arch/workshop01_vulkano_triangle.md` (read it first for the template: numbered/sectioned
conceptual walkthrough, mental-model diagrams in fenced ` ```text ` blocks, a "what Neptune hides
vs. exposes" style table, a complete-code walkthrough section, a closing exercise) — but for
"building Flappy Bird on top of Neptune" instead of "drawing a triangle in Vulkano". Cover:

1. Why Flappy Bird is a reasonable Neptune stretch target despite Neptune being a 3D engine (tie
   back to `neptune2d.md`'s gap analysis and fill philosophy — DIY for pedagogical gaps,
   3rd-party only for genuinely orthogonal solved problems).
2. The gaps that had to close and how (input, delta-time, AABB collision, orthographic camera,
   sprite material, text rendering) — cross-reference `neptune2d.md`'s table for each, then show
   the REAL resulting code from Task 1/2 (not aspirational pseudocode).
3. A walkthrough of the actual game loop structure in `examples/flappy_bird.rs` — gravity/flap,
   pipe spawning/scrolling, collision, scoring, restart — with real excerpts.
4. A short "what's still missing vs. a real 2D engine" closing note, pointing at `neptune2d.md`'s
   Phase 2 items that this example deliberately didn't need (physics engine, tilemaps, debug UI,
   etc.) so a reader understands the scope boundary.
5. An exercise section in the same spirit as workshop01's §9 (e.g. "extend the pipe gap
   randomization", "add a second bird color using the same SpriteMaterial path").

### Verification

Read the file back; confirm code excerpts match the real Task 1/2 implementation verbatim, and
that it reads coherently as a standalone tutorial (a reader who hasn't seen this plan should be
able to follow it using only this doc plus the two referenced arch docs).
