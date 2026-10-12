---

description: "Task list for Phase 1 — Window and Centered Ship"
---

# Tasks: Phase 1 — Window and Centered Ship

**Input**: Design documents from `specs/001-centered-ship/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included. The spec requires automated verification (SC-002 via the offscreen render test, SC-006 and SC-008 via tests), and plan.md defines the Rust and pytest suites. Within each story, write the tests alongside or before the implementation.

**Organization**: grouped by user story (US1 window, US2 centered image, US3 sharing and placement) so each can be built and verified as an increment.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on incomplete tasks)
- **[Story]**: US1 / US2 / US3 (user story phases only)

## Path Conventions

These follow plan.md "Project Structure":
- Rust core: `crates/ferrocade/src/`, Rust tests: `crates/ferrocade/tests/`
- Bindings: `crates/ferrocade-py/src/`
- Python package: `python/ferrocade/`, pytest: `tests/`, example: `examples/`

**Global rules for every task** (constitution):
- No PyO3 in `crates/ferrocade`.
- No `std::time::Instant` (use `web_time`).
- Platform-specific code only in `crates/ferrocade/src/platform/`.
- WGSL only, and `wgpu::Limits::downlevel_webgl2_defaults()`.
- No required threads.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the workspace builds, the extension module imports, and the riskiest binding question (R4) is answered first.

- [X] T001 Create the root Cargo workspace in `Cargo.toml`.
  - Settings: `members = ["crates/ferrocade", "crates/ferrocade-py"]`, `resolver = "3"`, edition 2024.
  - `[workspace.dependencies]` pins the versions from research.md R1: wgpu 30, winit 0.30, pyo3 0.29, image 0.25 (`default-features = false`, features `png`, `jpeg`, `bmp`, `gif`), web-time 1.1, bytemuck 1.25 (feature `derive`), pollster 1.0, slotmap 1.1, smallvec 1, thiserror 2.0, log 0.4, env_logger 0.11.
  - Add `[profile.dev.package."*"] opt-level = 2`, so image decoding and wgpu are usable in debug builds.
- [X] T002 [P] Create `crates/ferrocade/Cargo.toml` (workspace deps, **no pyo3**) and `crates/ferrocade/src/lib.rs`.
  - `lib.rs` declares the modules `error`, `assets`, `texture`, `store`, `instance`, `frame`, `timestep`, `color`, `window`, `render`, `platform`, with empty files for each per the plan.md tree.
- [X] T003 [P] Create `crates/ferrocade-py/Cargo.toml` and `crates/ferrocade-py/src/lib.rs`.
  - Cargo settings: `crate-type = ["cdylib"]`, `name = "_ferrocade"`, pyo3 with features `abi3-py313`, a path dependency on `ferrocade`.
  - `lib.rs`: a `#[pymodule] fn _ferrocade` that sets `__version__` from `CARGO_PKG_VERSION` and calls `env_logger::try_init()` (so `RUST_LOG` works).
- [X] T004 [P] Create the root `pyproject.toml` (maturin backend per research.md R12).
  - `[project] name = "ferrocade"`, `requires-python = ">=3.13"`, `dynamic = ["version"]`.
  - `[tool.maturin] manifest-path = "crates/ferrocade-py/Cargo.toml"`, `python-source = "python"`, `module-name = "ferrocade._ferrocade"`, `features = ["pyo3/extension-module"]`.
  - `[tool.pytest.ini_options] testpaths = ["tests"]`, and register the marker `display`.
- [X] T005 [P] Create the Python package skeleton: `python/ferrocade/__init__.py` (imports `__version__` from `._ferrocade`), an empty `python/ferrocade/py.typed`, and `python/ferrocade/_ferrocade.pyi` (with only `__version__: str` for now).
- [X] T006 Create the dev environment and confirm the build.
  - Commands: `uv venv`, `.venv\Scripts\Activate.ps1`, `uv pip install maturin pytest`, `maturin develop`.
  - Verify that `python -c "import ferrocade as fc; print(fc.__version__)"` prints `0.1.0`. Fix any build issue before continuing.
- [X] T007 **R4 verification spike.** In `crates/ferrocade-py/src/lib.rs`, add a temporary `#[pyclass(subclass)] _InitProbe`.
  - Its `#[new]` takes `#[pyo3(signature = (*args, **kwargs))]` and creates default state. Its Rust `__init__(width: u32, height: u32, *, filter: &str)` stores the values. It has a getter `width`.
  - Add `tests/test_init_probe.py`. It defines `class G(_InitProbe): def __init__(self): super().__init__(800, 600, filter="x")`, asserts `G().width == 800`, and asserts that a subclass calling `super().__init__(800, 600)` raises `TypeError` (missing `filter`).
  - **If it passes under abi3:** record "R4 confirmed: native `__init__`" in `specs/001-centered-ship/research.md` R4.
  - **If not:** record that the fallback is used (pure-Python subclasses in `python/ferrocade/` with `__init__` calling a native `_init`), and apply the fallback in T024/T040.
  - Then delete `_InitProbe` and `tests/test_init_probe.py`.
- [X] T008 [P] Create `tests/conftest.py`. Register the `display` marker, and skip `@pytest.mark.display` tests when the env var `FERROCADE_HEADLESS=1` is set.

**Checkpoint**: `maturin develop` works, and the subclassing approach is confirmed.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: core types that every story uses. Everything except T015/T016 can be done in parallel.

**⚠️ No user story work starts until this phase is complete.**

- [X] T009 [P] Implement `crates/ferrocade/src/error.rs` with a `thiserror` enum `Error`.
  - Variants: `NotFound { path }`, `Io { path, source }`, `Decode { path, reason }`, `Unsupported { path, reason }`, `NotInFrame { op: &'static str }` (message: `"{op}() can only be called inside on_draw()"`), `InvalidArgument(String)`, `Gpu(String)`, `EventLoop(String)`.
  - Add `pub type Result<T> = std::result::Result<T, Error>`.
- [X] T010 [P] Implement the asset loader in `crates/ferrocade/src/assets.rs`.
  - `pub fn canonicalize(path: &Path) -> Result<PathBuf>`: resolves relative paths against the current working directory. A missing file returns `Error::NotFound { path }`.
  - `pub fn read_bytes(path: &Path) -> Result<Vec<u8>>`: a filesystem implementation under `#[cfg(not(target_arch = "wasm32"))]`, and a wasm32 stub that returns `Error::Unsupported` (the web loader comes in Phase 1b).
- [X] T011 [P] Add `pub enum Filter { Nearest, Linear }` (`Copy`, `Eq`, `Hash`, `Debug`) to `crates/ferrocade/src/texture.rs`. There is deliberately no `Default` impl (research.md R11: "there is no default filter").
- [X] T012 [P] Implement `crates/ferrocade/src/color.rs`.
  - `pub fn parse_rgba(components: &[i64]) -> Result<[u8; 4]>`. It accepts a length of 3 or 4 (alpha defaults to 255) and each component must be 0–255; anything else is `Error::InvalidArgument`.
  - `pub fn to_linear_f32(rgba: [u8; 4]) -> [f64; 4]`: an sRGB→linear conversion for RGB (alpha stays linear), because the surface is sRGB and wgpu clear colors are linear.
  - `pub const DARK_GRAY: [u8; 4] = [32, 32, 32, 255]`.
  - Unit tests in the same file.
- [X] T013 [P] Implement `crates/ferrocade/src/timestep.rs` per data-model.md.
  - `pub struct FixedTimestep { step: f64, accumulator: f64, max_steps_per_frame: u32 }`, with `max_steps_per_frame` set to 5.
  - `new(step)`, and `advance(elapsed: f64) -> u32`, which returns the number of updates to run. "Excess accumulated time is discarded" when the cap is hit.
- [X] T014 [P] Write `crates/ferrocade/tests/timestep.rs`, covering:
  - `advance(1/60)` with a step of 1/60 returns 1
  - `advance(0.5)` returns 5 and leaves the accumulator below `step`
  - fractional accumulation across calls
  - zero elapsed returns 0
- [X] T015 Create the skeleton of `crates/ferrocade/src/store.rs`: the `slotmap::new_key_type! { pub struct SpriteId; pub struct ListId; }` keys, plus empty `SpriteStore` and global-accessor stubs (filled in by US2/US3).
- [X] T016 Implement `crates/ferrocade/src/frame.rs` (depends on T015 for `ListId`).
  - A thread-local `FrameQueue { active: bool, commands: Vec<FrameCommand> }`, with `enum FrameCommand { Clear([f64; 4]), DrawList(ListId) }`.
  - `pub fn begin()` and `pub fn end() -> Vec<FrameCommand>`.
  - `pub fn push(cmd, op: &'static str) -> Result<()>`. It returns `Error::NotInFrame { op }` when not active ("Enqueuing while inactive raises RuntimeError").
  - Unit tests in the same file.
- [X] T017 [P] Implement `crates/ferrocade/src/window.rs`.
  - **`WindowConfig { width: u32, height: u32, title: String, update_rate: f64, filter: Filter }`**, with `validate()`: "width/height must be > 0", "update_rate must be > 0", otherwise `Error::InvalidArgument`.
  - **`WindowState` (`Arc<Mutex<…>>`)** holds the logical width and height. They update on resize, and while minimized "the last non-zero size is kept".
  - **`pub trait GameCallbacks { type Error; fn update(&mut self, dt: f64) -> Result<(), Self::Error>; fn draw(&mut self) -> Result<(), Self::Error>; fn check_interrupt(&mut self) -> Result<(), Self::Error>; }`**
  - **`pub enum RunError<E> { Callback(E), Engine(Error) }`**
- [X] T018 [P] Implement `crates/ferrocade-py/src/errors.rs`: `fn to_pyerr(e: ferrocade::Error) -> PyErr`.
  - `NotFound` maps to `PyFileNotFoundError` (the message includes the path).
  - `Decode` / `Unsupported` / `InvalidArgument` map to `PyValueError` (the message includes the path and the reason where applicable).
  - `NotInFrame` / `EventLoop` / `Gpu` map to `PyRuntimeError`.
  - `Io` maps to `PyOSError`.
- [X] T019 [P] Implement `crates/ferrocade-py/src/py_filter.rs`.
  - A `#[pyclass(eq, frozen, name = "Filter")] enum PyFilter { NEAREST, LINEAR }`, with conversion to and from `ferrocade::texture::Filter`.
  - Register it in `lib.rs` and add the module attributes `NEAREST` and `LINEAR`.
  - Export `Filter`, `NEAREST`, `LINEAR` from `python/ferrocade/__init__.py`, and add them to `python/ferrocade/_ferrocade.pyi`.

**Checkpoint**: `cargo test -p ferrocade` passes (timestep, color, frame), and `import ferrocade as fc; fc.NEAREST` works.

---

## Phase 3: User Story 1 - Open a game window (Priority: P1) 🎯 MVP

**Goal**: a Python subclass of `fc.Window` (with the required `filter=`) opens a window, clears it to a color, gets `on_draw`/`on_update` calls, and closes cleanly. Exceptions propagate.

**Independent Test**: a script with only a `Window` subclass whose `on_draw` calls `self.clear(fc.color.BLACK)`. The window appears with the right size, title, and color, and closing it ends the script with exit code 0.

### Tests for User Story 1

- [X] T020 [P] [US1] Write the window tests that don't need a display in `tests/test_window.py`:
  - `fc.Window(800, 600, "x")` raises `TypeError` (missing `filter`)
  - `filter="nearest"` raises `TypeError`
  - `width=0`, `height=0`, or `update_rate=0` raises `ValueError`
  - a subclass's `super().__init__(800, 600, "T", filter=fc.NEAREST)` works, and `width`/`height`/`title`/`filter`/`update_rate` are readable before `run()`
  - `clear()` outside `on_draw` raises `RuntimeError` with a message containing "on_draw"
  - `clear((300, 0, 0))` inside a frame raises `ValueError` (this can be tested via a display test)
- [X] T021 [P] [US1] Write the window tests that need a display in `tests/test_window.py` (`@pytest.mark.display`). Stop the loop by raising a custom `StopTest` from a callback after N frames, inside `pytest.raises(StopTest)`.
  - `on_update` receives `delta_time == update_rate`
  - `on_draw` is called repeatedly
  - an exception from `on_draw` propagates out of `run()`, with a traceback that includes the user's `on_draw` frame
  - `run()` can be called a second time on a new window in the same process
  - `clear()` with a 3- or 4-tuple works inside `on_draw`

### Implementation for User Story 1

- [X] T022 [US1] Implement the clear-only Renderer in `crates/ferrocade/src/render/mod.rs`.
  - **Init:** `pub async fn Renderer::new(instance, target)`, where the target is a surface or an offscreen texture. It requests an adapter, then a device with `wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())`.
  - **Surface:** prefer an sRGB surface format, `PresentMode::Fifo`.
  - **`resize(physical_w, physical_h)`:** skip when either is 0.
  - **`render_frame(commands, logical_size)`:**
    - acquire the frame (on `Lost`/`Outdated`, reconfigure and skip; on `OutOfMemory`, return `Error::Gpu`)
    - start with the first `Clear` color, or `color::to_linear_f32(DARK_GRAY)` if there isn't one
    - each later `Clear` starts a new pass with `LoadOp::Clear`
    - `DrawList` is a no-op until US2
    - submit and present
  - No depth attachment.
- [X] T023 [US1] Implement `crates/ferrocade/src/platform/mod.rs`: `pub fn run<C: GameCallbacks>(config: &WindowConfig, state: WindowState, callbacks: &mut C) -> Result<(), RunError<C::Error>>`. It dispatches to `desktop::run` under `#[cfg(not(target_arch = "wasm32"))]`, and the wasm32 arm returns `Error::Unsupported` (Phase 1b).
- [X] T024 [US1] Implement `crates/ferrocade/src/platform/desktop.rs` (research.md R2, R3, R9, R10).
  - **Event loop:**
    - a thread-local `OnceCell<EventLoop<()>>`, created on first use
    - run with `EventLoopExtRunOnDemand::run_app_on_demand`, so `run()` can be called again
    - `ControlFlow::Wait`
  - **`ApplicationHandler`:**
    - **`resumed`:** create the window with `LogicalSize(config.width, config.height)` and the title, create the surface, `pollster::block_on(Renderer::new(..))`, then `request_redraw()`.
    - **`RedrawRequested`:**
      1. `n = timestep.advance(elapsed via web_time::Instant)`, then call `callbacks.update(step)` n times
      2. `frame::begin()`, `callbacks.draw()`, `cmds = frame::end()` (always `end`, even on error)
      3. `renderer.render_frame(cmds)`, `callbacks.check_interrupt()`, `window.request_redraw()`
    - **`Resized` / `ScaleFactorChanged`:** `renderer.resize(physical)`, and update `WindowState` with the logical size (keeping the last non-zero size).
    - **`CloseRequested`:** `event_loop.exit()`.
  - **Errors:** any callback error is stored, triggers `exit()`, and is returned as `RunError::Callback`.
  - On exit, drop the window and the renderer.
- [X] T025 [US1] Implement `Window` in `crates/ferrocade-py/src/py_window.rs` using the R4 pattern confirmed in T007.
  - **Class:** `#[pyclass(subclass)]`, with a `#[new]` that accepts `(*args, **kwargs)` and creates defaults.
  - **`__init__(self, width=800, height=600, title="Ferrocade", *, filter: Filter, update_rate=1/60)`:** validates via `WindowConfig::validate`. A missing `filter` produces Python's `TypeError`.
  - **Read-only getters:** `width`, `height` (from `WindowState`), `title`, `filter`, `update_rate`.
  - **Default methods:** no-op `on_draw(self)` and `on_update(self, delta_time)`.
  - **`clear(self, color=(32,32,32))`:** `color::parse_rgba`, then `frame::push(Clear, "clear")`.
  - **`run(slf)`:**
    - raise `RuntimeError` unless `threading.current_thread() is threading.main_thread()`
    - build `PyCallbacks { obj: Py<PyAny> }`: `update` calls `obj.call_method1("on_update", (dt,))`, `draw` calls `obj.call_method0("on_draw")`, and `check_interrupt` calls `py.check_signals()`
    - call `platform::run`, then re-raise a `RunError::Callback(PyErr)` unchanged, and map `RunError::Engine` via `to_pyerr`
  - Register it in `lib.rs`.
- [X] T026 [US1] Create `python/ferrocade/color.py` with `BLACK=(0,0,0)`, `WHITE=(255,255,255)`, `DARK_GRAY=(32,32,32)`, `RED`, `GREEN`, `BLUE`, `CORNFLOWER_BLUE=(100,149,237)`. Export `Window` and `color` from `python/ferrocade/__init__.py`, and add the `Window` stubs (signature per contracts/python-api.md) to `python/ferrocade/_ferrocade.pyi`.
- [X] T027 [US1] Run `maturin develop` and `pytest tests/test_window.py`, and check the independent test manually (an 800×600 "Ferrocade" window, the clear color, closing gives exit 0, Ctrl+C raises `KeyboardInterrupt`).

**Checkpoint**: US1 is complete. A blank window works end to end from Python.

---

## Phase 4: User Story 2 - Draw an image centered on screen (Priority: P2)

**Goal**: `Texture`, `Sprite`, and `SpriteList` (append and draw) plus the sprite pipeline. `examples/01_centered_ship.py` shows the ship centered and crisp (`filter=fc.NEAREST`).

**Independent Test**: `python examples/01_centered_ship.py` shows the ship centered at 99×75 with transparency, and the offscreen render test passes.

### Tests for User Story 2

- [X] T028 [P] [US2] Write `crates/ferrocade/tests/texture_cache.rs`, covering:
  - loading `assets/images/ship.png` gives 99×75 with `pixels.len() == 99*75*4`
  - a missing path gives `Error::NotFound` whose message contains the file name
  - a temp file with garbage bytes and a `.png` name gives `Error::Decode` mentioning the path
  - a temp `.txt` file gives `Decode`/`Unsupported`
- [X] T029 [P] [US2] Write `crates/ferrocade/tests/store.rs` (basic), covering:
  - create a sprite (center 400,300, scale 1, ship texture) and a list, append it, and assert that `instances[0]` has center `[400.0, 300.0]` and size `[99.0, 75.0]`, with a dirty range covering slot 0
  - "Appending the same sprite twice to the same list raises ValueError" (`Error::InvalidArgument`)
  - "Non-finite values (NaN/inf) raise ValueError"
- [X] T030 [P] [US2] Write `tests/test_texture.py` (basic), covering:
  - `fc.Texture("assets/images/ship.png")`: width 99, height 75, `path` is absolute and ends with `ship.png`, `filter is None`
  - created before any `Window`
  - a missing file gives `FileNotFoundError` with the name in the message
  - a corrupt file (`tmp_path`) gives `ValueError` with the path in the message (SC-006)
- [X] T031 [P] [US2] Write `tests/test_sprite.py` (basic), covering:
  - `Sprite(texture)` and `Sprite("assets/images/ship.png")` are equivalent (same `width`/`height`/`texture ==`)
  - defaults: `center_x == center_y == 0.0`, `scale == 1.0`
  - `width == 99.0` and `height == 75.0`
  - `center_x = 0.1` reads back exactly `0.1`
  - `float('nan')` and `float('inf')` raise `ValueError`
  - `class Ship(fc.Sprite)` with `super().__init__("assets/images/ship.png", 400, 300)` works and can set extra attributes
- [X] T032 [P] [US2] Write `crates/ferrocade/tests/render_offscreen.rs`.
  - **Skip** (pass with a printed note) if no adapter is available.
  - **Setup:** render to an 800×600 offscreen target with a `DARK_GRAY` clear, one list holding the ship at (400,300) with window filter `Nearest`, then read the pixels back.
  - **Assertions:**
    - the bounding box of the non-background pixels is 99×75 (±1), centered at (400,300) within 1 px (y measured from the bottom; SC-002)
    - a known transparent pixel in the ship's corner equals the background
    - a sprite at (0,0) shows only in the bottom-left quadrant around the origin

### Implementation for User Story 2

- [X] T033 [US2] Implement the texture cache in `crates/ferrocade/src/texture.rs` (research.md R8, data-model.md TextureData).
  - **`TextureData { id: TextureId, path: PathBuf, width: u32, height: u32, pixels: Arc<[u8]> }`**, where `TextureId` is a `u64` from a global atomic counter.
  - **`TextureCache`:** a global `Mutex<HashMap<PathBuf, Arc<TextureData>>>`.
  - **`pub fn load(path) -> Result<Arc<TextureData>>`:** canonicalize via `assets`, return the cached entry on a hit, otherwise `assets::read_bytes` → `image::load_from_memory` → `to_rgba8()`. Map decode errors to `Decode { path, reason }`.
  - **Strong references** ("textures are never unloaded" in Phase 1).
  - **A disk-read counter** `pub fn disk_reads() -> u64`, used to test SC-008.
- [X] T034 [P] [US2] Implement `crates/ferrocade/src/instance.rs`.
  - The `#[repr(C)]`, `bytemuck::Pod` struct `Instance { center: [f32;2], size: [f32;2], angle: f32, _pad0: [f32;3], uv_rect: [f32;4] }` ("48 bytes in total"). Add a const assert that `size_of == 48`.
  - `Instance::from_sprite(&SpriteState)`: size = `texture.width/height * |scale|`, and `uv_rect = (0,0,1,1)`.
- [X] T035 [US2] Implement the core of `crates/ferrocade/src/store.rs` (research.md R5, data-model.md).
  - **A global `Mutex<SpriteStore>`**, with slotmaps for `sprites: SlotMap<SpriteId, SpriteState>` and `lists: SlotMap<ListId, ListState>`.
  - **`SpriteState`:** `center_x`, `center_y`, `scale`, `angle: f64`, `texture: (Arc<TextureData>, Option<Filter>)`, `memberships: SmallVec<[(ListId, u32); 2]>`.
  - **`ListState`:** `sprites: Vec<SpriteId>`, `instances: Vec<Instance>`, `slot_textures: Vec<(TextureId, Option<Filter>)>`, `dirty: Option<Range<usize>>`, `structure_version: u64`.
  - **Functions:**
    - `create_sprite(texture, center_x, center_y, scale)` (validate finite)
    - `destroy_sprite(id)` (also removes it from every list it belongs to)
    - `create_list()`, `destroy_list(id)` ("removes its memberships from all of its sprites")
    - `append(list, sprite)` (push, add the membership `(list, index)`, mark the new slot dirty, bump `structure_version`; a duplicate raises `InvalidArgument`)
    - getters, and `take_dirty(list) -> Option<(Range, &[Instance])>` for the renderer
- [X] T036 [P] [US2] Write `crates/ferrocade/src/render/sprite.wgsl`.
  - **Vertex stage:** build the quad corner from `vertex_index` (6 vertices, two triangles), using the per-instance attributes `center` (loc 0), `size` (loc 1), `angle` (loc 2, unused), and `uv_rect` (loc 3). Position = `center + (corner - 0.5) * size`, transformed by the uniform `projection: mat4x4<f32>` (group 0). The UV comes from `uv_rect`, with v flipped so the image is upright under a y-up projection.
  - **Fragment stage:** `textureSample(t, s, uv)` (group 1).
- [X] T037 [US2] Implement `crates/ferrocade/src/render/gpu_texture.rs`.
  - Lazy upload of `TextureData` to an `Rgba8UnormSrgb` texture, keyed by `TextureId`.
  - Two samplers (`Nearest`, `Linear`), both `ClampToEdge`.
  - Bind groups cached per `(TextureId, Filter)`.
  - The bind group layout for texture plus sampler.
- [X] T038 [US2] Implement `crates/ferrocade/src/render/gpu_list.rs`.
  - A per-`ListId` GPU instance buffer (`VERTEX | COPY_DST`) that grows by doubling (recreated when there isn't enough room).
  - Each frame, upload only `take_dirty` via `queue.write_buffer`.
  - Compute batch runs (cached by `structure_version`): consecutive slots with the same `(TextureId, effective filter)`, where the effective filter is the override, otherwise the window filter.
  - `draw(pass)` issues one `draw(0..6, run_range)` per run.
- [X] T039 [US2] Extend `crates/ferrocade/src/render/mod.rs`.
  - Create the sprite pipeline: `sprite.wgsl`, an instance vertex buffer layout matching `Instance` (stride 48), `BlendState::ALPHA_BLENDING`, no depth.
  - A projection uniform: an orthographic matrix over `(0..logical_w, 0..logical_h)`, bottom-left origin.
  - Execute `DrawList(id)` commands through `gpu_list` and `gpu_texture`, using the window `filter` passed into `render_frame`.
  - An offscreen target mode plus `read_pixels() -> Vec<u8>`, used by T032.
  - Drop the GPU mirrors with the renderer.
- [X] T040 [US2] Implement `Texture` in `crates/ferrocade-py/src/py_texture.rs`.
  - **Class:** `#[pyclass(frozen, name = "Texture")]` (final).
  - **`#[new]` `(path: PathBuf /* str|PathLike */, *, filter: Option<PyFilter> = None)`:** calls `texture::load` and stores `(Arc<TextureData>, Option<Filter>)`.
  - **Getters:** `width`, `height`, `path` (a string), `filter`.
  - **Equality and hashing:** `__eq__` and `__hash__` on `(path, filter)`.
  - Errors go through `to_pyerr`.
  - Register it, then export it and add its stubs.
- [X] T041 [US2] Implement `Sprite` in `crates/ferrocade-py/src/py_sprite.rs` (R4 pattern, subclassable).
  - **Struct and construction:** `#[pyclass(subclass, weakref)]` holding a `SpriteId`. `__init__(texture: Texture | str | PathLike, center_x=0.0, center_y=0.0, scale=1.0)`, where a path goes through `texture::load` with no override.
  - **Getters:** `center_x`, `center_y`, `position`, `scale`, `angle`, `texture`, `width`, `height` (= texture size × |scale|).
  - **Setters are deferred to US3**, but `center_x`/`center_y` are set at construction.
  - **`Drop`:** calls `store::destroy_sprite`.
  - Register it, then export it and add its stubs (the full property set per the contract).
- [X] T042 [US2] Implement `SpriteList` in `crates/ferrocade-py/src/py_sprite_list.rs`.
  - **Class:** `#[pyclass(name = "SpriteList")]` (final), holding a `ListId` plus `py_refs: Vec<Py<PyAny>>` that keep member sprites alive, in the same order.
  - **Methods:** `__init__`, `append(sprite)` (calls `store::append`, then pushes `py_ref`), `draw()` (`frame::push(DrawList(id), "draw")`), and `__len__`.
  - **`Drop`:** calls `store::destroy_list`.
  - Register it, then export it and add its stubs.
- [X] T043 [US2] Create `examples/01_centered_ship.py`. It uses exactly the "Usage" code from contracts/python-api.md (`filter=fc.NEAREST`, `assets/images/ship.png`, centered at `self.width / 2, self.height / 2`) and is run from the repo root.
- [X] T044 [US2] Run `cargo test -p ferrocade`, `maturin develop`, `pytest`, and `python examples/01_centered_ship.py`, and verify quickstart V3 (centered, 99×75, crisp edges, transparency).

**Checkpoint**: the Phase 1 milestone (window plus centered ship) is reached.

---

## Phase 5: User Story 3 - Reuse images across sprites and control placement (Priority: P3)

**Goal**: live sprite setters that update every list containing the sprite, list `remove`/`contains`/`iter`, sprites in multiple lists, filter overrides, and a verified read-once cache.

**Independent Test**: create 10 sprites sharing a texture at different positions and scales, draw them, then move one. All 10 appear, the file was read once, and the moved sprite appears at its new position on the next frame.

### Tests for User Story 3

- [X] T045 [P] [US3] Extend `crates/ferrocade/tests/store.rs`, covering:
  - a sprite in two lists: setting `center_x` updates both lists' `instances[slot]` and widens both dirty ranges
  - removing the middle sprite of 3 shifts the later slot down, updates its membership slot, and marks the shifted range dirty
  - removing a sprite that isn't in the list gives `InvalidArgument`
  - destroying a list clears the memberships
  - setting the texture updates `slot_textures` and bumps `structure_version`
  - scale 2.0 gives instance size `[198, 150]`
- [X] T046 [P] [US3] Extend `tests/test_texture.py`, covering:
  - `Texture(p) == Texture(p)` and their hashes are equal
  - `Texture(p) != Texture(p, filter=fc.LINEAR)`
  - creating 1,000 `Sprite(p)` objects plus `Texture(p, filter=fc.LINEAR)` leaves `fc._ferrocade._debug_texture_disk_reads()` increased by exactly 1 for a fresh file (copy `ship.png` to `tmp_path`; SC-008, FR-023)
  - `filter="linear"` raises `TypeError`
- [X] T047 [P] [US3] Write `tests/test_sprite_list.py`, covering:
  - `append`/`len`/`in`
  - appending a duplicate raises `ValueError`
  - `remove` preserves order, and removing a missing sprite raises `ValueError`
  - iteration yields the same objects (`is`), including subclass instances
  - one sprite in two lists
  - the list keeps the sprite alive (`weakref` to the sprite is still alive after `del`)
  - an empty list's `draw()` inside a frame doesn't raise (display test)
- [X] T048 [P] [US3] Extend `tests/test_sprite.py`, covering:
  - `scale = 2.0` gives `width == 198.0` and `height == 150.0`
  - `position = (10.5, 20.25)` round-trips, and so do `center_x`/`center_y`
  - reassigning `texture` changes `width`/`height`
  - `angle` round-trips
- [X] T049 [P] [US3] Extend `crates/ferrocade/tests/render_offscreen.rs`, covering:
  - 10 sprites sharing one texture all render (10 separate blobs)
  - after moving one sprite via the store setter, the next frame shows it at the new position and not the old one
  - overlapping sprites: the one appended later is on top
  - a `Linear` override renders intermediate edge colors, while `Nearest` has none at a 2× scale

### Implementation for User Story 3

- [X] T050 [US3] Add setters to `crates/ferrocade/src/store.rs`: `set_center_x`, `set_center_y`, `set_position`, `set_scale`, `set_angle`, `set_texture`.
  - "Every setter updates the f64 field, then recomputes the f32 Instance and writes it into each membership's list at slot, widening that list's dirty range."
  - `set_texture` also updates `slot_textures` and bumps `structure_version`.
  - Validate that values are finite.
- [X] T051 [US3] Add `remove(list, sprite)` and `contains(list, sprite)` to `crates/ferrocade/src/store.rs`. Removing "shifts later slots down by one (updating those sprites' membership slots), and marks the shifted range dirty". Removing a sprite that isn't in the list gives `InvalidArgument`.
- [X] T052 [US3] Wire the setters for `center_x`, `center_y`, `position`, `scale`, `angle`, and `texture` in `crates/ferrocade-py/src/py_sprite.rs`. `texture` accepts only a `Texture`.
- [X] T053 [US3] Add `remove`, `__contains__`, and `__iter__` to `crates/ferrocade-py/src/py_sprite_list.rs`. `__iter__` iterates over a snapshot of `py_refs`, and `remove` keeps `py_refs` in sync.
- [X] T054 [US3] Expose the private `_debug_texture_disk_reads()` in `crates/ferrocade-py/src/lib.rs` (wrapping `texture::disk_reads`). Make sure `Texture(path, filter=...)` overrides reach `slot_textures` and the batching in `gpu_list.rs`.

**Checkpoint**: all three user stories work, and the full Rust and pytest suites pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T055 [P] Review `python/ferrocade/_ferrocade.pyi` and `python/ferrocade/__init__.py` against contracts/python-api.md: every public name, signature, and property type, with `filter` keyword-only and required on `Window` (FR-021).
- [ ] T056 [P] Update `CLAUDE.md` "Environment": replace "Build/test commands: TBD" with the setup, build, and test commands from quickstart.md. Add a "Building from source" pointer to `README.md`.
- [ ] T057 Portability audit (constitution III) of `crates/ferrocade/src`:
  - no `std::time::Instant`
  - no `winit`/`pollster` use outside `platform/`
  - no `std::thread` spawning
  - limits are `downlevel_webgl2_defaults`
  - shaders are WGSL only
  - Fix anything found.
- [ ] T058 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `maturin develop`, and `pytest`. Everything must pass cleanly.
- [ ] T059 Run quickstart.md validations V1 to V10 (including V7b), and record the results (pass/fail, notes, machine) in `docs/research/phase1-validation.md`.
- [ ] T060 Update `docs/spec/ENGINE_SPEC.md`: mark Phase 1 complete in §10, add decision log entries for implementation decisions (the R4 outcome, and any deviations from the plan), and note the known limitation "run_app_on_demand is desktop-only; web uses spawn_app (Phase 1b)".

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1):** T001 comes first. T002 to T005 can then run in parallel, followed by T006 and T007 (R4 decides the pattern used in T025/T041).
- **Foundational (Phase 2):** depends on Setup. T015 comes before T016, and the rest can run in parallel.
- **US1 (Phase 3):** depends on Foundational.
- **US2 (Phase 4):** depends on Foundational, and on US1's renderer and platform loop (T022 to T025) for drawing on screen. The data classes (T033 to T035, T040 to T042) and their tests can start right after Foundational, in parallel with US1.
- **US3 (Phase 5):** depends on US2's store and bindings (T035, T041, T042).
- **Polish (Phase 6):** depends on all the stories.

### User Story Dependencies

- **US1:** independent (the MVP).
- **US2:** needs the US1 loop to show anything on screen. Its offscreen render test and data tests are independent of US1.
- **US3:** builds on the US2 store and bindings.

### Within Each User Story

- Write the tests together with or before the implementation, and make sure they pass at the checkpoint.
- Core (`crates/ferrocade`) comes before the bindings (`crates/ferrocade-py`), which come before the stubs and exports, then the example.

### Parallel Opportunities

- **Setup:** T002, T003, T004, T005, T008.
- **Foundational:** T009 to T014, T017, T018, T019.
- **US1 tests:** T020, T021.
- **US2 tests:** T028 to T032. T034 and T036 can run in parallel with other US2 implementation work.
- **US3 tests:** T045 to T049.
- **Polish:** T055, T056.

---

## Parallel Example: User Story 2

```text
# Tests (independent files):
Task: "T028 texture cache tests in crates/ferrocade/tests/texture_cache.rs"
Task: "T029 store basic tests in crates/ferrocade/tests/store.rs"
Task: "T030 Texture API tests in tests/test_texture.py"
Task: "T031 Sprite API tests in tests/test_sprite.py"
Task: "T032 offscreen render test in crates/ferrocade/tests/render_offscreen.rs"

# Independent implementation files:
Task: "T034 Instance struct in crates/ferrocade/src/instance.rs"
Task: "T036 WGSL shader in crates/ferrocade/src/render/sprite.wgsl"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup, including the R4 spike, then Foundational.
2. US1: a blank window from Python. **Stop and validate** (T027).

### Incremental Delivery

1. Add **US2**. The Phase 1 milestone (centered ship) is done, so demo it (T044).
2. Add **US3**: sharing, live placement, and filter overrides.
3. Polish: portability audit, lint, the V1 to V10 validation record, and spec updates.

### Commit Cadence

- Commit locally after each task or logical group. **Do not push** until the owner says to.

---

## Notes

- [P] = different files and no incomplete dependencies. [USn] = traceability to spec.md.
- Constraints are quoted from data-model.md and research.md so they aren't reinterpreted during implementation.
- If a task reveals a design problem, stop and raise it with the owner (constitution V). Don't silently change the contract.
