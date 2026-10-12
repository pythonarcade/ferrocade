---

description: "Task list for Phase 1b — Web Feasibility Spike"
---

# Tasks: Phase 1b — Web Feasibility Spike

**Input**: Design documents from `specs/002-web-feasibility-spike/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/web-engine-api.md](contracts/web-engine-api.md), [quickstart.md](quickstart.md)

**Tests**: this is a spike, so verification uses the **validation scenarios W1–W10** (quickstart.md) and an in-browser pixel check, plus the existing desktop suites for regression (FR-011). The one new automated test covers the additive core API (`load_from_bytes`).

**Organization**: grouped by the spec's user stories:
- **US1:** a decision backed by evidence (approach A experiments, paper assessments, the report)
- **US2:** the centered ship in a browser (approach B)
- **US3:** a repeatable local web build

**Execution note**: phases are listed in priority order. The **recommended execution order** is Setup, then Foundational, then **US2**, then US1, then US3, then Polish, because the US1 recommendation (T021) needs US2's evidence and the US1 approach-A work is owner-gated (T010).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on incomplete tasks)
- **[Story]**: US1 / US2 / US3

**Rules for all tasks**:
- Mark spike-only shortcuts in code with a `// SPIKE:` / `# SPIKE:` comment (FR-012).
- Never break the desktop build (FR-011).
- Don't record the web approach as decided in the engine spec (FR-015).

---

## Phase 1: Setup

- [X] T001 Install the approach-B toolchain on Windows: `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.129` (it must match the `wasm-bindgen` version in `Cargo.lock`). Record the exact versions (`rustc --version`, `wasm-bindgen --version`) for the report.
- [X] T002 Scaffold `crates/ferrocade-web` and add it to the workspace `members` in `Cargo.toml`.
  - **`crates/ferrocade-web/Cargo.toml`:** `crate-type = ["cdylib", "rlib"]`, `publish = false`.
  - **Dependencies:** `ferrocade`, `wgpu` (workspace), `wasm-bindgen = "=0.2.129"`, `wasm-bindgen-futures`, `js-sys`, `web-sys` (features `HtmlCanvasElement`, `Window`, `Document`, `console`), `console_error_panic_hook`.
  - **`crates/ferrocade-web/src/lib.rs`:** wrap the whole crate body in `#![cfg(target_arch = "wasm32")]`, so `cargo build --workspace` on Windows still succeeds.
- [X] T003 [P] Add `spike/pyodide-wheel/target/`, `spike/pyodide-wheel/dist/` and `tools/web-check/node_modules/` to `.gitignore` (`web/pkg/` is already ignored).
- [X] T004 [P] Create the report skeleton `docs/research/web-spike.md`, with the sections from data-model.md "SpikeReport":
  1. Summary and recommendation, marked **Proposed (pending owner acceptance)**
  2. Per-approach findings (A, A′, B, C), each with `status` / `evidence` / `measurements` / `blockers` / `engine_impact` subsections
  3. Comparison
  4. Engine impact (a table with the class column: *done in spike* / *required before shipping web* / *nice to have*)
  5. Risks
  6. Next steps
  7. Reproduction steps
  - Pre-fill R1–R10 desk findings and sources from research.md under each approach as "desk evidence".

---

## Phase 2: Foundational (Blocking Prerequisites)

**⚠️ The core must compile for `wasm32-unknown-unknown` before any approach-B work.**

- [X] T005 Make `crates/ferrocade` build for `wasm32-unknown-unknown`.
  - In `crates/ferrocade/Cargo.toml`, add `[target.'cfg(target_arch = "wasm32")'.dependencies] wgpu = { workspace = true, features = ["webgpu", "webgl"] }`.
  - Make sure `winit` is only a dependency on `cfg(not(target_arch = "wasm32"))`, since the desktop platform module is already cfg-gated.
  - Run `cargo check -p ferrocade --target wasm32-unknown-unknown` and fix any wasm-only errors **without changing desktop behavior**.
- [X] T006 Add `pub fn load_from_bytes(path: &Path, bytes: &[u8]) -> Result<Arc<TextureData>>` to `crates/ferrocade/src/texture.rs`.
  - It uses the same cache. The key is `path` as given, not canonicalized, because the web has no filesystem.
  - On a miss it decodes `bytes`, with the same `Error::Decode { path, reason }` mapping.
  - Refactor `load()` to canonicalize, read via `assets::read_bytes`, and then share the decode and insert code (the disk-read counter only counts `load()` reads).
- [X] T007 [P] Add tests to `crates/ferrocade/tests/texture_cache.rs`: `load_from_bytes("mem/ship.png", ship_bytes)` gives 99×75, a second call returns the same `Arc` (`Arc::ptr_eq`), and garbage bytes give `Error::Decode` mentioning `mem/bad.png`.
- [X] T008 [P] Add `pub fn backend_name(&self) -> String` to `crates/ferrocade/src/render/mod.rs` (it records the adapter's `get_info().backend` at creation). Confirm `Renderer::with_surface` has no desktop-only code paths on wasm32 (no `pollster`).
- [X] T009 Desktop regression gate: `cargo test --workspace`, `maturin develop`, and `pytest` all pass (W9). `cargo check -p ferrocade-web --target wasm32-unknown-unknown` passes.

**Checkpoint**: the core builds for the browser, and the desktop is unchanged.

---

## Phase 3: User Story 1 - A web strategy decision backed by evidence (Priority: P1)

**Goal**: one report that gives A, A′, B, and C a verdict backed by evidence, and ends with a **Proposed** recommendation.

**Independent Test (W1)**: the owner reads `docs/research/web-spike.md` alone and can state the recommended approach, its evidence, the risks, and the next steps.

### Approach A: Pyodide wheel (owner-gated)

- [X] T010 [US1] **Owner gate.** Ask the owner for permission to install the approach-A toolchain in WSL2 Ubuntu 22.04's home directory: rustup (stable ≥ 1.93, target `wasm32-unknown-emscripten`), emsdk at Pyodide 314's Emscripten **5.0.3**, Python 3.14, and `pyodide-build`. That's a few GB.
  - Record the answer in `spike/pyodide-wheel/NOTES.md`.
  - **If declined:** set approach A's status to `blocked`, with desk evidence only (cite wgpu#10274, pyodide#6476, the winit platform error) and the note "hands-on not attempted: owner declined toolchain install". Skip T011–T015.
- [ ] T011 [US1] Add a default-on Cargo feature `render` to `crates/ferrocade/Cargo.toml`. It gates the `render` and `platform` modules plus the `wgpu`/`winit`/`pollster` dependencies (`lib.rs` uses `#[cfg(feature = "render")]`).
  - This lets the core's data layer (texture, store, timestep, color, frame) build without wgpu.
  - Verify that the desktop builds are unchanged (`cargo test --workspace`, `maturin develop`).
- [ ] T012 [US1] Create the A1 experiment crate `spike/pyodide-wheel/` (outside the workspace; its own `Cargo.toml` with `[workspace]`).
  - `pyo3 = "0.29"` **without** abi3, and `ferrocade = { path = "../../crates/ferrocade", default-features = false }`.
  - A `#[pymodule] ferrocade_a1` exposing `texture_size(path: str, data: bytes) -> tuple[int, int]` (via `load_from_bytes`) and `store_smoke() -> int`, which creates 3 sprites in a list, removes one, and returns the length.
  - Add `spike/pyodide-wheel/build.sh` (WSL), which sources emsdk 5.0.3, sets `CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS` from `pyodide config get rustflags`, and runs `pyodide build` (or `maturin build --target wasm32-unknown-emscripten -i python3.14`).
- [ ] T013 [US1] Build A1 in WSL2.
  - Expect a wheel tagged `cp314-cp314-pyemscripten_2026_0_wasm32`.
  - Add `spike/pyodide-wheel/smoke.html`, which loads Pyodide 314.0.7 from the CDN, runs `micropip.install` on the local wheel URL, and calls both functions with `ship.png` bytes. Serve it over `python -m http.server`.
  - Record the results verbatim (the wheel filename, the console output, any errors) in `spike/pyodide-wheel/NOTES.md`.
- [ ] T014 [US1] A2: rebuild with `default-features = true` (render enabled), and add a function that calls `pollster`-free `Renderer::offscreen`.
  - Record the exact failure, with build or link output, in `spike/pyodide-wheel/NOTES.md`. The expected failures are wgpu#10274 `__wbindgen_placeholder__` imports, or winit "platform not supported".
  - If it unexpectedly builds and loads, record that and tell the owner before continuing.
- [ ] T015 [US1] Write approach A's findings into `docs/research/web-spike.md`: the status (`blocked` or `proven`), A1 and A2 evidence, tool versions, blockers with upstream links and how to tell when they're resolved, and the engine impact (feature-gated core: *done in spike* / *nice to have*).

### Paper assessments

- [ ] T016 [P] [US1] Write the **A′** assessment into `docs/research/web-spike.md`: an Emscripten wheel with a hand-written WebGL2 renderer (glow) plus an Emscripten platform layer. Cover the components it needs, the estimated size relative to the current renderer, the cost of maintaining two renderers, and when it would make sense. Status: `assessed on paper`.
- [ ] T017 [P] [US1] Write the **C** assessment (RustPython) into `docs/research/web-spike.md`: CPython compatibility gaps, no PyO3 (bindings rewritten against `rustpython_vm`), single-binary upside, and maturity. Optionally do a quick check of the current RustPython browser build instructions and record what was found. Status: `assessed on paper` (or `not attempted` with the reason).

### Decision (needs US2 results)

- [ ] T018 [US1] Write the **B** findings into `docs/research/web-spike.md` from the US2 evidence (T031–T036): status, evidence, measurements (download bytes, cold and warm time-to-ship, graphics paths, centering, example diff lines), and the documented web behavior differences (contract §1 table).
- [ ] T019 [US1] Fill in the **Engine impact** table in `docs/research/web-spike.md`.
  - Classify every change: `load_from_bytes` (done), wasm32 wgpu features (done), the `render` feature (done if T011 ran), `ferrocade-web` crate (done, spike-quality), the Python shim (spike, so it has to become a real package), the hard-coded asset list (required before shipping web), winit web backend for input (required before Phase 4 on web), shared API tests across both bindings (required), vendoring Pyodide versus CDN (nice to have).
  - List every `SPIKE:` marker.
- [ ] T020 [US1] Write the **Comparison** (A / A′ / B / C against: works today, game-code changes, renderer count, binding layers, per-call cost, download size, build host, maintenance) and the **Risks** (API drift between the two bindings, Python→JS→WASM call overhead, Pyodide download size, wgpu web maturity, `run()` behaving differently on the web).
- [ ] T021 [US1] Write the **Summary and recommendation**: one approach (expected: B now, with A revisited when Pyodide adopts Emscripten ≥ 6.0.10 and wgpu#10515 lands), or "none viable yet" plus what would change that. Add concrete **Next steps**, and mark the recommendation **Proposed (pending owner acceptance)**.
- [ ] T022 [US1] In `docs/spec/ENGINE_SPEC.md` §9 "Web", add one line: "Phase 1b spike results: `docs/research/web-spike.md` (recommendation pending owner acceptance)". Do **not** change the decision log or mark any approach as decided (FR-015).

**Checkpoint**: W1, the report is complete and gives a clear decision.

---

## Phase 4: User Story 2 - The centered ship runs in a browser (Priority: P2)

**Goal**: `examples/01_centered_ship.py`, unchanged, renders in Edge or Chrome through Pyodide 314 + `ferrocade-web` (WebGPU, falling back to WebGL2).

**Independent Test (W2)**: build, serve, and open `/web/`. After the loading states, the ship appears centered and crisp on a dark gray canvas.

### Engine (Rust, `crates/ferrocade-web/src/lib.rs`)

- [ ] T023 [US2] Implement `Engine::create(canvas: HtmlCanvasElement, backend: String) -> Promise<Engine>` (`#[wasm_bindgen]` async).
  - Install `console_error_panic_hook`.
  - Create the `wgpu::Instance` with backends `BROWSER_WEBGPU | GL` for `"auto"` or `GL` for `"webgl2"`.
  - Create the surface via `wgpu::SurfaceTarget::Canvas(canvas)`, then `Renderer::with_surface` at the canvas size.
  - Store the renderer, the window filter (default `Nearest` until `configure`), the logical size, and `FixedTimestep`.
  - Add a `backend_name` getter that uses `Renderer::backend_name`.
  - Errors reject the promise with a readable message (for example "no WebGPU or WebGL2 adapter").
- [ ] T024 [US2] Implement the frame API per contract §2.
  - `configure(width, height, filter: "nearest" | "linear", update_rate)`: resizes the canvas and the surface, and sets the timestep and filter.
  - `advance(elapsed) -> u32` via `FixedTimestep::advance`.
  - `begin_frame()` calls `frame::begin()`.
  - `clear(r, g, b, a)` uses `color::parse_rgba`, then `frame::push(Clear)`. Errors keep the desktop message ("color components must be 0-255…").
  - `end_frame()` calls `frame::end()`, then `renderer.render_frame(&cmds, &mut store::global(), logical_size, filter)`.
- [ ] T025 [US2] Implement the texture, sprite, and list API per contract §2. Use `u32` handle tables (`Vec<Option<SpriteId>>`, `Vec<Option<ListId>>`, `Vec<TextureRef>`) so IDs stay JS-number safe.
  - **Textures:** `load_texture(path, bytes: Vec<u8>) -> u32` via `texture::load_from_bytes`, and `texture_size`.
  - **Sprites:** `create_sprite(texture, filter: "" | "nearest" | "linear", cx, cy, scale)`, `destroy_sprite`, `get_sprite -> Vec<f64>` (`[center_x, center_y, scale, angle]`), `set_center_x`, `set_center_y`, `set_position`, `set_scale`, `set_angle`.
  - **Lists:** `create_list`, `destroy_list`, `append`, `remove`, and `draw_list` (`frame::push(DrawList)`).
  - Every core `Error` maps to `JsError` with `error.to_string()`, so the messages match desktop.

### Browser side

- [ ] T026 [P] [US2] Write the shim `web/python/ferrocade/__init__.py`, implementing contract §1 on top of the engine JS module (registered as `ferrocade_engine`).
  - **`Filter`:** an `enum.Enum` with `NEAREST`/`LINEAR`, plus the module aliases.
  - **`Window`:**
    - `__init__(self, width=800, height=600, title="Ferrocade", *, filter, update_rate=1/60)`, raising the same `ValueError`/`TypeError` as desktop
    - read-only properties
    - `clear(color=None)`, which maps the JS error to `ValueError`, and `RuntimeError` outside a frame
    - `run()`: `create_proxy(frame)` with `js.requestAnimationFrame`, re-armed each frame. Each frame runs `n = engine.advance(dt)`, `on_update(update_rate)` × n, `engine.begin_frame()`, `on_draw()`, `engine.end_frame()`. An exception stops the loop, destroys the proxy, and calls `js.ferrocadeReportError(traceback)`. `run()` returns immediately.
  - **`Texture(path, *, filter=None)`:** reads the bytes with `open(path, "rb")` from Pyodide's filesystem (a missing file raises `FileNotFoundError` naming the path), calls `engine.load_texture`, and implements `__eq__`/`__hash__` on (path, filter).
  - **`Sprite`:** `__init__(texture | path, center_x=0.0, center_y=0.0, scale=1.0)`, properties forwarding to the engine (no Python-side copies), `width`/`height` from the texture size × `abs(scale)`, `__del__` calling `destroy_sprite`. Subclassable.
  - **`SpriteList`:** `append`/`remove`/`draw`/`__len__`/`__contains__`/`__iter__`, keeping Python references in order.
  - Anything unsupported raises `NotImplementedError("... not available in the web spike")`. Mark the module docstring `SPIKE:`.
- [ ] T027 [P] [US2] Copy `python/ferrocade/color.py` to `web/python/ferrocade/color.py` unchanged. Add a `# SPIKE: copied from python/ferrocade/color.py; share it once the web package is real` header.
- [ ] T028 [P] [US2] Create `web/index.html`: a `<canvas id="game">`, a status line (`#status`), a hidden error panel (`#error`, `<pre>`), a module script `loader.js`, and minimal centered styling.
- [ ] T029 [US2] Create `web/loader.js` (ES module) per contract §3.
  - `performance.mark("start")`. Status "Loading Python runtime…", then `loadPyodide()` from `https://cdn.jsdelivr.net/pyodide/v314.0.7/full/pyodide.mjs`.
  - Status "Starting graphics…": import `./pkg/ferrocade_web.js`, `await init()`, `await Engine.create(canvas, new URLSearchParams(location.search).get("backend") ?? "auto")`, and log `backend=<engine.backend_name>`.
  - Status "Loading assets…": `// SPIKE: hard-coded asset list`. Fetch `../assets/images/ship.png` and write it to `pyodide.FS` at `assets/images/ship.png` (create the directories). Fetch `python/ferrocade/__init__.py` and `color.py` into `/home/pyodide/ferrocade/`.
  - `pyodide.registerJsModule("ferrocade_engine", engine)`. Define `window.ferrocadeReportError` to show the `#error` panel.
  - Fetch `../examples/01_centered_ship.py` **unchanged** and run it with `pyodide.runPythonAsync`.
  - After the first `end_frame`: `performance.mark("ship-visible")` and log the ms since start (hook it via a one-shot wrapper around `engine.end_frame`).
  - Any rejected step shows the error panel with the step name and reason.
- [ ] T030 [US2] Add the centering diagnostics to `web/loader.js` (and update contract §2: pixel readback lives in the loader, not in `Engine`). Immediately after the first rendered frame, *in the same task*, draw the game canvas into an offscreen 2D canvas, `getImageData`, compute the bounding box of non-background pixels (background `rgb(32,32,32)`), and log the size and the center offset from the canvas center. Enable it only with `?diagnostics=1`.

### Validation (results go to the report in T018)

- [ ] T031 [US2] Create the automated browser check `tools/web-check/` (`package.json` with `puppeteer-core`, plus `check.mjs`).
  - It launches the installed Chrome or Edge (headless `new`, with `--enable-unsafe-webgpu` where needed) against `http://localhost:8000/web/?diagnostics=1[&backend=webgl2]`.
  - It captures the console (backend, ship-visible ms, centering), the network transfer sizes, and a screenshot to `tools/web-check/out/`, then exits non-zero on page errors.
  - Add `tools/web-check/out/` to `.gitignore`.
- [ ] T032 [US2] **W2 + W4 (WebGPU):** build (T033 script or manual), serve, and run the check on `/web/?diagnostics=1`. The ship should be centered within 1 px at 99×75 and the backend should be WebGPU. Save the screenshot path and console log into `spike/web-results.md`. If headless WebGPU is unavailable, record that and rely on T035 plus the owner's check.
- [ ] T033 [US2] Create `tools/build_web.ps1`: it runs `cargo build -p ferrocade-web --target wasm32-unknown-unknown --release` and then `wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/ferrocade_web.wasm`, and prints the serve and open instructions. With `-Serve`, it also starts `python -m http.server 8000` from the repo root.
- [ ] T034 [US2] **W3 (WebGL2):** run the check with `&backend=webgl2`. Expect the same centering result with a GL backend. Record it in `spike/web-results.md`.
- [ ] T035 [US2] **W5–W8:**
  - W5: confirm the loader runs the fetched `examples/01_centered_ship.py` byte-for-byte (diff lines = 0).
  - W6: cold and warm time-to-ship, as the median of 3 runs each, from the check script (cold = fresh browser profile).
  - W7: download bytes split into Pyodide, engine wasm+js, and assets.
  - W8: temporarily serve without `ship.png` (rename it, then restore) and confirm the error panel names the file.
  - Record everything in `spike/web-results.md`.
- [ ] T036 [US2] **W9:** desktop regression (`cargo test --workspace`, `maturin develop`, `pytest`) all pass. Ask the owner to open `/web/` in Edge or Chrome and confirm visually; record their confirmation in `spike/web-results.md`.

**Checkpoint**: US2 is complete (or its blockers are documented), and the evidence feeds T018.

---

## Phase 5: User Story 3 - A repeatable local web build on Windows (Priority: P3)

**Goal**: anyone can rebuild and preview the web version from a clean checkout on the Windows dev machine.

**Independent Test (W10)**: clean checkout, then the documented steps, then the browser example appears.

- [ ] T037 [US3] Update `specs/002-web-feasibility-spike/quickstart.md` and the report's **Reproduction steps** with the exact verified commands and versions (Rust, wasm-bindgen-cli 0.2.129, Pyodide 314.0.7, the browser version). Add a "Web build" block to `CLAUDE.md` "Build & test" (`tools/build_web.ps1 -Serve`, then open `http://localhost:8000/web/`).
- [ ] T038 [US3] **W10:** clone the repo into a temporary directory (`git clone . %TEMP%\ferrocade-w10`), follow only the documented steps, and run the T031 check against it. Record the result, and fix any undocumented step found.
- [ ] T039 [P] [US3] If T012–T014 ran, copy the verified WSL build steps for approach A from `spike/pyodide-wheel/NOTES.md` into the report's Reproduction steps, marked "Approach A experiment (WSL2)".

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T040 Audit the spike markers: `grep -rn "SPIKE:"` across `crates/ web/ tools/ spike/`. Every hit must appear in the report's engine-impact table (FR-012).
- [ ] T041 Lint: `cargo clippy --workspace --all-targets -- -D warnings` (desktop) and `cargo clippy -p ferrocade-web --target wasm32-unknown-unknown -- -D warnings`. Run `cargo fmt --all`.
- [ ] T042 **W1 final review** of `docs/research/web-spike.md` against data-model.md: every approach has exactly one status, evidence or a reason, and specific blockers; the recommendation is **Proposed**; risks and next steps are present; SC-001 to SC-007 are each addressed. Update `docs/research/README.md` to link it.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (T001–T004):** T001 and T002 come first. T003 and T004 can run in parallel.
- **Foundational (T005–T009):** T005 comes before T006 and T008. T007 depends on T006. T009 gates everything after it.
- **US2 (T023–T036):** depends on Foundational.
  - T023 → T024 → T025 (same file).
  - T026–T028 can run in parallel with the engine work.
  - T029/T030 need T025–T028. T031–T036 need a buildable page.
- **US1 (T010–T022):**
  - A-path T011–T015 depends on the **owner gate T010**.
  - T016/T017 can run anytime after Setup.
  - T018–T021 need the US2 results (T032–T036).
  - T022 needs T021.
- **US3 (T037–T039):** after US2. T039 after T014.
- **Polish (T040–T042):** last.

### User Story Dependencies

- **US2** is the evidence engine and is independent of US1 and US3.
- **US1**'s paper and A-path parts are independent; its final decision depends on US2.
- **US3** depends on US2 (there must be something to reproduce).

### Parallel Opportunities

- **Setup:** T003, T004.
- **Foundational:** T007, T008 (after T006/T005).
- **US2:** T026, T027, T028 alongside T023–T025.
- **US1:** T016, T017 at any point, and the A-path in WSL (T012–T014) alongside US2 on Windows.
- **US3:** T039 alongside T037.

---

## Parallel Example: User Story 2

```text
# While the engine crate is being written (T023–T025, one file):
Task: "T026 Python shim in web/python/ferrocade/__init__.py"
Task: "T027 copy color.py to web/python/ferrocade/color.py"
Task: "T028 web/index.html"
```

---

## Implementation Strategy

### Evidence first

1. Setup, then Foundational (the core builds for wasm32, and the desktop is unchanged).
2. **US2:** get the ship into the browser with approach B, then measure it (T031–T036). **Stop and review** with the owner.
3. **US1:** the owner gate for A (T010). Run A1/A2 if approved. Write the paper assessments and then the report with a **Proposed** recommendation.
4. **US3:** reproducibility (W10). Then Polish.

### If approach B fails

Document the exact blocker (T018). Escalate A′ from a paper assessment to a time-boxed hands-on attempt, after checking with the owner. The report still delivers a decision (US1 stands on its own).

### Commit cadence

Commit locally after each logical group. **Push only when the owner says so.**

---

## Notes

- IDs are sequential in listing (priority) order. Follow the **recommended execution order** in the note at the top (US2 before US1's decision tasks).
- This is spike code: correctness and evidence matter, polish doesn't. Everything temporary is marked `SPIKE:`.
- If a finding contradicts research.md, the hands-on evidence wins. Update research.md and tell the owner.
