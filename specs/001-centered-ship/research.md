# Research: Phase 1 — Window and Centered Ship

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12

Each section resolves one Technical Context unknown or design question.

---

## R1. Dependency versions

**Decision**: pin to these current stable releases (checked against crates.io and PyPI on 2026-10-12):

| Package | Version | Role |
|---|---|---|
| `wgpu` | 30.0 | Rendering (D3D12/Vulkan on Windows; Metal, WebGPU/WebGL2 later) |
| `winit` | 0.30 | Window + event loop (`ApplicationHandler` API) |
| `pyo3` | 0.29 | Python bindings (`abi3-py313`) |
| `image` | 0.25 | Decoding (features: `png`, `jpeg`, `bmp`, `gif`) |
| `web-time` | 1.1 | `Instant` that works on desktop and WASM |
| `bytemuck` | 1.25 | Safe casting of instance structs to GPU bytes |
| `pollster` | 1.0 | Blocking on async GPU init (desktop platform layer only) |
| `slotmap` | 1.1 | Stable IDs for sprites and lists in the store |
| `thiserror` | 2.0 | Error types in the core |
| `log` / `env_logger` | 0.4 / 0.11 | Diagnostics (`RUST_LOG`) |
| `maturin` (PyPI) | 1.15 | Build/install the extension module |
| `pytest` (PyPI) | 9.1 | Python API tests |

**Rationale**: these are the current stable versions. winit 0.30's `ApplicationHandler` is event-driven and shared with the web backend, which constitution III requires.

**Alternatives considered**: SDL3 for windowing. Rejected because winit is the standard wgpu pairing and was already decided (engine spec §5).

---

## R2. Calling Python callbacks from the winit event loop

**Decision**:
- `Window.run()` (Rust) holds the GIL and runs the winit event loop on the calling (main) thread.
- The core defines a Python-free trait, `GameCallbacks { update(dt), draw() }`. The bindings implement it by calling the Python object's `on_update` / `on_draw` methods (so subclass overrides are found by normal attribute lookup).
- **Exceptions:** if a callback raises, the bindings store the `PyErr`, ask the event loop to exit, and re-raise it from `run()` once the loop stops (spec edge case: exceptions propagate).
- **Ctrl+C:** call `py.check_signals()` once per frame, so `KeyboardInterrupt` works while the loop runs.

**Rationale**: keeps the core independent of Python (constitution I). Normal method lookup makes subclassing work as users expect.

**Alternatives considered**: running the loop on a background thread. Rejected because macOS and the web require the main thread (engine spec §9 rule 9).

---

## R3. Event loop lifetime (one per process)

**Decision**:
- winit allows creating only **one `EventLoop` per process**. Create it lazily on the first `run()` and keep it in a thread-local.
- On desktop, run it with `run_app_on_demand` (`EventLoopExtRunOnDemand`), so `run()` can be called again after a window closes (sequential windows, and multiple windows across pytest tests).
- The OS window is created in `ApplicationHandler::resumed` (required by winit 0.30), not in `Window.__init__`. `Window.__init__` only stores the configuration, so `self.width` / `self.height` work immediately in user code (as in the spec §3 example).

**Rationale**: matches winit's rules. The script continues after `run()` returns (FR-003).

**Alternatives considered**: `run_app` (consumes the loop, so `run()` could only be called once per process). Rejected because it breaks tests and sequential windows.

**Web note**: `run_on_demand` doesn't exist on the web. The web build will use `spawn_app` instead, inside the platform layer (Phase 1b).

---

## R4. Making `Window` and `Sprite` subclassable with `super().__init__(...)`

**Decision**:
- Native classes use `#[pyclass(subclass)]`.
- `#[new]` accepts `(*args, **kwargs)` and creates a default-initialized object.
- A Rust `__init__` method (supported by PyO3 0.29) takes the real arguments, so `class Game(fc.Window): def __init__(self): super().__init__(800, 600, "Ferrocade", filter=fc.NEAREST)` works as in the spec example.
- **Verify early (first implementation task)** that `__init__` works under `abi3-py313`. The PyO3 guide's `__init__` example is gated on non-limited-API builds, but that gate relates to subclassing `PyDict`, not to `__init__` itself.
- **Fallback if it doesn't work:** thin pure-Python subclasses in `python/ferrocade/` define `__init__` and call a native `_init(...)`. Property access stays native either way.

**Outcome (T007, 2026-10-12)**: **confirmed.** With PyO3 0.29 and `abi3-py313`, a native `__init__` on a `#[pyclass(subclass)]` (with `#[new]` accepting `*args, **kwargs`) works with a Python subclass calling `super().__init__(800, 600, filter=...)`. A missing required keyword raises `TypeError`. The fallback isn't needed.

**Rationale**: users subclass `Window` now and will subclass `Sprite` later (e.g. `class Player(fc.Sprite)`).

**Alternatives considered**: making `Window` purely Python. Rejected because it adds an indirection layer for every callback, with no benefit.

---

## R5. Where sprite data lives (constitution II)

**Decision**: a **central store in the core**: `SpriteStore`, held in a process-global `Mutex` (uncontended locking is cheap, and it works on WASM).
- **Sprites:** a slotmap of `SpriteState`. It holds the f64 fields (center_x, center_y, scale, angle), a texture reference, and a list of memberships `(ListId, slot)`.
- **Lists:** a slotmap of `ListState`. It holds an ordered `Vec<SpriteId>`, a GPU-layout `Vec<Instance>` (f32), the texture of each slot, and a dirty range.
- **Python objects are thin handles:**
  - `Sprite` holds a `SpriteId`. Getters read f64s, and setters update the f64 and then write the f32 instance into every list the sprite belongs to, widening each list's dirty range.
  - `SpriteList` holds a `ListId`.
  - When a Python handle is dropped, its entry is removed from the store (a list's handle also clears the memberships it holds).
- **No per-sprite work at draw time:** drawing uploads only the dirty range of each list's instance buffer.

**Rationale**: implements engine spec §7 exactly. Membership back-references support one sprite in many lists (FR-018). A central store also prepares for Rust-side `update()` (Phase 2) and collision checks (Phase 5).

**Alternatives considered**:
- Each `Sprite` pyclass holding its own fields, with lists re-gathering every frame. Rejected because it means per-sprite work every frame.
- `#[pyclass(unsendable)]` with `Rc<RefCell<…>>`. Rejected because it panics on cross-thread access and doesn't work with free-threaded Python.

---

## R6. How `SpriteList.draw()` reaches the GPU (re-entrancy)

**Decision**: a **per-frame draw queue**.
- While `on_draw` runs, `Window.clear(color)` and `SpriteList.draw()` only append commands (`Clear(color)`, `DrawList(ListId)`) to a thread-local queue.
- After `on_draw` returns, the platform layer runs the queue with the renderer:
  - acquire the surface texture
  - sync each list's dirty range (`queue.write_buffer`)
  - lazily upload any textures not yet on the GPU
  - record render passes (the first `Clear` sets the load op), then submit and present
- Calling `draw()` or `clear()` outside `on_draw` raises `RuntimeError`.

**Rationale**: avoids re-entrant mutable access to the renderer from Python calls in the middle of a frame. It's simple, and it keeps the renderer entirely inside Rust.

**Trade-off (documented)**: a list is drawn with its state at the end of `on_draw`. Drawing the same list twice in one frame with changes in between would show the final state both times. That's acceptable for Phase 1. Revisit if it ever matters (e.g. snapshot the instance range at enqueue time).

**Alternatives considered**: a thread-local "current frame" that holds `&mut Renderer` during `on_draw`. Rejected because it needs unsafe or complex lifetime juggling.

---

## R7. Renderer design (2D, WebGL2-compatible)

**Decision**:
- **Limits:** `wgpu::Limits::downlevel_webgl2_defaults()`, using the adapter's resolution.
- **Shader:** one WGSL pipeline for textured quads.
  - The unit quad comes from `vertex_index` (6 vertices, no vertex buffer).
  - Per-instance vertex attributes (`step_mode: Instance`): `center: vec2<f32>`, `size: vec2<f32>`, `angle: f32`, `uv_rect: vec4<f32>`. That's 36 bytes per instance, padded to 48 for alignment (the instance struct is `#[repr(C)]` + `bytemuck::Pod`).
  - Angle is stored but not applied in Phase 1 (rotation is out of scope). Wiring it in later is a one-line shader change.
- **Projection:** a uniform buffer with an orthographic matrix mapping `(0..width, 0..height)` in **logical pixels** to clip space. Bottom-left origin and y-up fall out naturally.
- **Batching:** within a list, consecutive sprites that share a texture form one instanced draw call. Draw order follows list order (FR-017).
- **Textures:**
  - Format is `Rgba8UnormSrgb`. One bind group (texture + sampler) per texture, created on first use.
  - The renderer keeps the GPU texture map (`TextureId → GpuTexture`), so CPU texture objects never hold wgpu types. This avoids `Send`/`Sync` issues on WASM and supports future multi-window setups.
- **Blending:** straight-alpha `BlendState::ALPHA_BLENDING`.
- **Surface:**
  - Present mode `Fifo` (vsync, available everywhere; SC-007), with an sRGB surface format.
  - On resize, reconfigure. When the size is zero (minimized), skip configuring and rendering.
  - On `SurfaceError::Lost`/`Outdated`, reconfigure and skip the frame. On `OutOfMemory`, return an error.
- **Instance buffers:** each list has a GPU buffer that grows (capacity doubling, recreating the buffer) when needed. Normally only the dirty range is written.

**Rationale**: satisfies constitution III (WebGL2 limits, WGSL) and II (dirty uploads), and meets the 2D-only rule (no depth buffer).

---

## R8. Texture loading and cache

**Decision**:
- `Texture(path)` resolves the path relative to the working directory, canonicalizes it, and looks it up in a global cache keyed by the canonical path.
- On a cache miss, it reads the file through the **asset loader** (`assets::read_bytes(path)`; on desktop this is the filesystem, and on web it will be a fetch or preload), decodes with `image`, converts to RGBA8, and stores `Arc<TextureData { id, width, height, path, pixels }>`.
- Errors (missing file, unsupported format, decode failure) map to Python exceptions whose message includes the path and the reason:
  - a missing file raises `FileNotFoundError`
  - an unsupported or corrupt file raises `ValueError`
- **The cache holds strong references in Phase 1** (textures are never unloaded). Unloading is deferred until a later phase needs it.

**Rationale**: FR-007 to FR-011, SC-006 and SC-008. Images can load before any GPU exists (decoding is CPU-only, and the upload is lazy).

**Note**: `ship.png` is 99×75 and indexed-color (palette with transparency). `image` expands it to RGBA8.

---

## R9. Fixed timestep and frame pacing

**Decision**:
- **Loop:** use `ControlFlow::Wait`, and call `window.request_redraw()` at the end of each `RedrawRequested`. Fifo presentation then paces the loop to the monitor refresh rate (on the web, this maps to `requestAnimationFrame`).
- **Each redraw:**
  1. add the elapsed real time (`web_time::Instant`) to an accumulator
  2. run `on_update(update_rate)` while the accumulator holds at least `update_rate`, at most **5 times per frame** (the leftover time is dropped to avoid a spiral of death)
  3. run `on_draw` and render
- The timestep logic is a pure, unit-tested core function, `FixedTimestep::advance(elapsed) -> n_updates`.

**Rationale**: implements engine spec §8. Phase 1 only needs the callbacks to work (FR-004). Phase 2 verifies the timing.

---

## R10. HiDPI (display scaling)

**Decision**:
- Create the window with `LogicalSize(width, height)`. `Window.width` and `Window.height` report the logical size.
- The surface is configured at the physical size, and the projection uses the logical size, so content scales automatically.
- Handle `ScaleFactorChanged` and `Resized` by reconfiguring.

**Rationale**: matches the spec assumption (logical pixels). Centering stays correct at any scale factor.

---

## R11. Texture filtering: an explicit choice, no default

**Decision (owner, 2026-10-12)**: **there is no default filter.**
- **Required at the window:** `Window.__init__` takes a **required keyword-only** `filter` argument (`fc.NEAREST` or `fc.LINEAR`). It sets the filter for every texture that doesn't override it. Leaving it out raises `TypeError` (Python's normal missing-argument error).
- **Optional per-texture override:** `Texture(path, filter=fc.LINEAR)` overrides the window filter for that texture. `filter=None` (the default for `Texture`) means "use the window's filter".
- **The Phase 1 example uses `filter=fc.NEAREST`.**
- **Implementation:**
  - The filter is a property of the *sampler*, not of the pixel data. `TextureData` (pixels, keyed by path) stays shared and is read from disk once.
  - A Python `Texture` is the pair (`Arc<TextureData>`, `Option<Filter>`).
  - The renderer keeps two samplers (nearest and linear) and creates bind groups per `(TextureId, Filter)`.
  - The effective filter is resolved at draw time (override, otherwise the window filter). This works because textures can exist before the window does.
  - Batching groups consecutive sprites with the same `(TextureId, effective Filter)`.
- **Addressing mode:** clamp-to-edge in both cases.

**Rationale**: which filter is right depends on the game's art style (pixel art vs. smooth or scaled images). Any default is wrong for a large share of users, and this question comes up again and again. Making it required forces a one-time conscious choice, and the override handles games that mix styles.

**Note**: `ship.png` is 99×75 (odd dimensions). Centered at (400, 300), its edges fall on half pixels. With nearest filtering they stay crisp, though they snap to one side by up to half a pixel. That still meets SC-002 (centered within 1 px).

**Alternatives considered**:
- A linear default (Arcade-style) or a nearest default. Rejected because any default silently suits one art style and not the other.
- Required on every texture. Rejected as too verbose, since every load site would need to repeat it.

---

## R12. Build, install, and test tooling

**Decision**:
- **Layout:** a root `Cargo.toml` defines a workspace with members `crates/ferrocade` and `crates/ferrocade-py`.
- **Root `pyproject.toml`** (maturin backend):
  - `[tool.maturin] manifest-path = "crates/ferrocade-py/Cargo.toml"`
  - `python-source = "python"`
  - `module-name = "ferrocade._ferrocade"`
  - `features = ["pyo3/extension-module"]`
  - `requires-python = ">=3.13"`
- **The `python/ferrocade/` package contains:**
  - `__init__.py`, which re-exports `Window`, `Texture`, `Sprite`, `SpriteList` and `Color` helpers
  - `_ferrocade.pyi` (type stubs, FR-021)
  - `py.typed`
- **Dev environment:** `uv venv` → `uv pip install maturin pytest` → `maturin develop` (debug) or `maturin develop --release` (to measure performance). These commands get documented in `CLAUDE.md` and `quickstart.md`.
- **Tests:**
  - **Rust (`cargo test -p ferrocade`):**
    - store (membership, dirty ranges, removal)
    - timestep math
    - texture cache (read-once, error mapping)
    - an **offscreen render test** that renders a sprite list into a texture and checks that the drawn ship's bounding box is centered within 1 px (SC-002). It's skipped if no GPU adapter is available.
  - **Python (`pytest tests/`):**
    - API behavior: properties, list operations, read-once cache (SC-008), error messages (SC-006)
    - subclassing
    - draw outside `on_draw` raises
    - an exception in a callback propagates (using a window that raises on its first update)
    - Window tests requiring a display are marked so they can be skipped in headless CI.
- **Manual acceptance:** `python examples/01_centered_ship.py` (SC-002 to SC-005, SC-007).

**Rationale**: maturin + PyO3 abi3 was already decided (engine spec §6). The offscreen test turns the visual milestone into an automated check.

---

## R13. Color representation

**Decision**:
- `clear(color=...)` accepts an RGB or RGBA tuple of ints from 0 to 255. The default is a dark gray (`(32, 32, 32)`), so transparent sprites are visibly composited.
- Common named colors are exposed as constants in a small Python `ferrocade.color` module (e.g. `fc.color.BLACK`), defined as tuples.

**Rationale**: 0–255 tuples match Arcade and Pygame conventions. A full `Color` class is unnecessary for Phase 1 (constitution IV).
