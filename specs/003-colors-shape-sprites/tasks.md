---

description: "Task list for Phase 1c — CSS Colors and Shape Sprites"
---

# Tasks: Phase 1c — CSS Colors and Shape Sprites

**Input**: Design documents from `specs/003-colors-shape-sprites/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included. The spec requires automated verification: the CSS values (SC-001), exact
colors and sizes (SC-003), the batching count (SC-004), and pixel-identical image sprites (SC-005).

**Organization**:
- **US1:** CSS colors
- **US2:** shape sprites
- **US3:** tint and fade

The tint *core* (the instance color, the store field, and the shader) is foundational because both
US2 and US3 use it.

## Format: `[ID] [P?] [Story] Description`

**Global rules**:
- No new dependencies.
- The instance stays **48 bytes**.
- Image sprites with the default tint must stay **pixel-identical** (the Phase 1 tests keep passing).
- Desktop and web keep the same API.
- Commit locally after each group; **push only when the owner says so**.

---

## Phase 1: Setup

- [X] T001 **R5 verification spike** (as Phase 1's R4).
  - In `crates/ferrocade-py/src/lib.rs`, add a temporary `#[pyclass(extends = PySprite, subclass)] _ShapeProbe`.
  - Its `#[new]` has `#[pyo3(signature = (*_args, **_kwargs))]` and returns `PyClassInitializer::from(PySprite::empty()).add_subclass(_ShapeProbe {})`. Add `PySprite::empty()` in `crates/ferrocade-py/src/py_sprite.rs`. Its `__init__(self_, size: f64)` creates a store sprite from `assets/images/ship.png` and sets the base `PySprite` id.
  - Add `tests/test_shape_probe.py`, which checks that `class W(_ShapeProbe): def __init__(self): super().__init__(5.0)` works, that `isinstance(W(), fc.Sprite)` holds, that `W().center_x == 0.0`, and that the object can be appended to an `fc.SpriteList`.
  - **If it passes under abi3:** record "R5 confirmed" in `specs/003-colors-shape-sprites/research.md` R5.
  - **Otherwise:** record the fallback (Python subclasses calling a native `_init_shape`) and apply it in T020.
  - Then delete the probe and its test.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: core tint support plus frame stats. Required by US2 (shapes are tinted white textures) and US3.

- [X] T002 Rename `color::DARK_GRAY` to `color::DEFAULT_BACKGROUND` (same value `[32, 32, 32, 255]`) in `crates/ferrocade/src/color.rs`, and update its uses in `crates/ferrocade/src/render/mod.rs`, `crates/ferrocade/tests/render_offscreen.rs`, and `crates/ferrocade-py/src/py_window.rs`.
- [X] T003 Change `crates/ferrocade/src/instance.rs` to the data-model.md layout: `center [f32;2]` @0, `size [f32;2]` @8, `angle f32` @16, **`color [u8;4]` @20**, `_pad [f32;2]` @24, `uv_rect [f32;4]` @32.
  - Keep the const assert that it's 48 bytes. Add const asserts on `offset_of!(Instance, color) == 20` and `offset_of!(Instance, uv_rect) == 32`.
  - `Instance::from_sprite` copies `sprite.color`, `sprite.uv_rect`, and the size `base_size.unwrap_or(texture size) × |scale|`.
- [X] T004 Extend `SpriteState` in `crates/ferrocade/src/store.rs`:
  - add `color: [u8; 4]` (default `[255,255,255,255]`), `base_size: Option<(f64, f64)>` (default `None`), and `uv_rect: [f32; 4]` (default `[0,0,1,1]`)
  - add `pub fn set_color(&mut self, id, rgba: [u8; 4]) -> Result<()>`, using the existing `update()` (so it marks dirty slots)
  - add `pub fn create_shape(&mut self, texture: TextureRef, uv_rect: [f32;4], base_size: (f64, f64), color: [u8;4], center_x, center_y) -> Result<SpriteId>`, which validates "w, h finite and > 0, otherwise `ValueError`" (`Error::InvalidArgument`) and that the centers are finite
- [X] T005 Update `crates/ferrocade/src/render/sprite.wgsl`.
  - Add `@location(4) color: vec4<f32>` (fed by `Unorm8x4`), pass it to the fragment stage, and convert RGB from sRGB to linear with the exact piecewise function (`c <= 0.04045 ? c/12.92 : pow((c+0.055)/1.055, 2.4)`), keeping alpha as is.
  - The fragment shader returns `textureSample(...) * color`.
  - In `crates/ferrocade/src/render/sprites.rs`, add vertex attribute 4: `Unorm8x4` at `offset_of!(Instance, color)`.
- [X] T006 Add frame statistics to `crates/ferrocade/src/render/mod.rs` and `crates/ferrocade/src/render/sprites.rs`: `pub struct FrameStats { pub draw_calls: u32, pub instances: u32 }`, filled during `render_frame` (each `pass.draw` adds one draw call and its instance count), plus `Renderer::last_frame_stats() -> FrameStats`.
- [X] T007 Foundation regression gate: `cargo test --workspace` (all Phase 1 offscreen pixel tests unchanged: white tint is ×1.0), `cargo check -p ferrocade-web --target wasm32-unknown-unknown`, `maturin develop`, and `pytest`. All must pass.

**Checkpoint**: the tint works in the core, and the Phase 1 rendering is identical.

---

## Phase 3: User Story 1 - Use any CSS color by name (Priority: P1) 🎯 MVP

**Goal**: `fc.color` has all 148 CSS colors, plus `TRANSPARENT` and `DEFAULT_BACKGROUND`, shared by desktop and web.

**Independent Test**: `pytest tests/test_color.py` passes, and `node tools/web-check/check_colors.mjs` reports all 148 matching Chrome.

- [X] T008 [P] [US1] Write `tools/gen_css_colors.py`.
  - It embeds the 148 CSS Color Level 4 keywords with their hex values, plus an **explicit keyword → UPPER_SNAKE name map** (e.g. `lightgoldenrodyellow` → `LIGHT_GOLDENROD_YELLOW`, `rebeccapurple` → `REBECCA_PURPLE`, `darkslategrey` → `DARK_SLATE_GREY`).
  - It writes `python/ferrocade/color.py`: a module docstring, then the constants sorted alphabetically as RGB int tuples, then `TRANSPARENT = (0, 0, 0, 0)` and `DEFAULT_BACKGROUND = (32, 32, 32)` in a separate "non-CSS" section, then `CSS_COLORS: dict[str, tuple[int, int, int]]` mapping each CSS keyword to its value (used by the checks).
  - It asserts that there are exactly 148 keywords and 148 unique names.
- [X] T009 [US1] Run `python tools/gen_css_colors.py` to regenerate `python/ferrocade/color.py`. Check that `GREEN == (0, 128, 0)`, `LIME == (0, 255, 0)`, `DARK_GRAY == (169, 169, 169)`, and `DEFAULT_BACKGROUND == (32, 32, 32)`.
- [X] T010 [P] [US1] Write `tests/test_color.py`, covering:
  - exactly 148 entries in `fc.color.CSS_COLORS`, and every entry exists as a module constant named by the map
  - all values are 3-tuples of ints in 0..255
  - every `*GRAY*` name has a `*GREY*` twin with an equal value
  - the spec values: `CORNFLOWER_BLUE == (100, 149, 237)`, `REBECCA_PURPLE == (102, 51, 153)`, `GREEN`, `LIME`, `DARK_GRAY`, `TRANSPARENT`, `DEFAULT_BACKGROUND`
  - `Window.clear()` with no argument still uses (32,32,32) (a display test that reads back via `RuntimeError`-free drawing; the pixel check is in T007's existing Rust tests)
- [X] T011 [P] [US1] Write `tools/web-check/check_colors.mjs` (puppeteer-core).
  - It loads `about:blank` in headless Chrome. For each keyword in a JSON dump of `CSS_COLORS` (produced by `python -c "import json, ferrocade.color as c; print(json.dumps(c.CSS_COLORS))"` and passed in through a file), it sets `div.style.color = keyword`, reads `getComputedStyle(div).color`, and compares the result with the Python value.
  - It also enumerates a hard-coded check that no extra keywords exist.
  - It prints mismatches, and exits non-zero on any mismatch or a count ≠ 148.
- [X] T012 [US1] Use a single source for the web: delete `web/python/ferrocade/color.py`, and change `web/loader.js` to fetch `../python/ferrocade/color.py` into `/home/pyodide/ferrocade/color.py`. Remove the corresponding `SPIKE:` item from the web-spike engine-impact table in `docs/research/web-spike.md` (mark it "done in Phase 1c").
- [X] T013 [US1] Run `pytest tests/test_color.py` and `node tools/web-check/check_colors.mjs` (Q2, Q3). Record the results for T034.

**Checkpoint**: US1 is complete. Named colors work everywhere `clear()` accepts a color.

---

## Phase 4: User Story 2 - Draw solid rectangles and circles without images (Priority: P2)

**Goal**: `fc.RectangleSprite` and `fc.CircleSprite` on desktop and web. Any mix draws in one call.

**Independent Test**: `python examples/01c_shapes.py` shows the shapes, and the Rust batching test reports 1 draw call for 1,000 interleaved shapes.

### Tests for User Story 2

- [X] T014 [P] [US2] Extend `crates/ferrocade/tests/render_offscreen.rs` (window filter `Nearest` unless stated), covering:
  - a 50×20 `RED` rectangle at (100,100): its bounding box is 50×20 ±1 and centered within 1 px, and its interior pixels are exactly (255,0,0)
  - a radius-15 `BLUE` circle at (200,100): its bounding box is 30×30 ±1, its center pixel is (0,0,255), and the bounding-box corner pixels are the background (round, not square)
  - a semi-transparent rectangle (alpha 128) over a white rectangle blends to roughly the midpoint (within ±3)
  - **1,000 interleaved rectangles and circles** with mixed sizes and colors in one list: `last_frame_stats().draw_calls == 1`
  - a circle with a `Linear` override gives at least 2 distinct partial-alpha edge values, while `Nearest` gives a crisper edge (fewer distinct edge values)
- [X] T015 [P] [US2] Extend `crates/ferrocade/tests/store.rs`, covering:
  - `create_shape` with width, height, or radius ≤ 0 or non-finite gives `InvalidArgument`
  - the instance size equals `base_size × |scale|`, and `set_scale(2.0)` doubles it
  - `set_color` rewrites the instance color in every list and marks those slots dirty
- [X] T016 [P] [US2] Write `tests/test_shapes.py`, covering:
  - `fc.RectangleSprite(50, 20, fc.color.RED, 100, 100)`: `width == 50.0`, `height == 20.0`, `color == (255, 0, 0, 255)`, `isinstance(..., fc.Sprite)`
  - `fc.CircleSprite(15, fc.color.BLUE)`: `width == height == 30.0`, `radius == 15.0`
  - `scale = 2` doubles width and radius
  - `ValueError` for size ≤ 0, NaN, or bad colors
  - `TypeError` for `filter="linear"`, and for assigning `texture` on a shape
  - `texture.path == "<builtin:shapes>"`
  - subclasses `class Wall(fc.RectangleSprite)` and `class Ball(fc.CircleSprite)` using `super().__init__(...)` with extra attributes
  - shapes work in `SpriteList` (append, remove, iteration identity), mixed with image sprites
  - a display test: drawing a list of shapes inside `on_draw` doesn't raise

### Implementation for User Story 2

- [X] T017 [US2] Add the built-in shape atlas to `crates/ferrocade/src/texture.rs`.
  - `pub fn builtin_shapes() -> Arc<TextureData>` generates, once and cached under the key `PathBuf::from("<builtin:shapes>")`, a 1024×512 RGBA image. The left 512×512 is opaque white. The right 512×512 is a white disc centered at (768,256) with radius 254, using an anti-aliased alpha edge (alpha = clamp(radius + 0.5 − distance, 0, 1) × 255) on a transparent background.
  - Add `pub const RECT_UV: [f32;4] = [0.125, 0.25, 0.375, 0.75]` and `pub const CIRCLE_UV: [f32;4] = [0.5, 0.0, 1.0, 1.0]`.
- [X] T018 [US2] Add `pub fn create_rectangle(&mut self, w, h, color, cx, cy)` and `pub fn create_circle(&mut self, radius, color, cx, cy, filter: Option<Filter>)` to `crates/ferrocade/src/store.rs`. They call `create_shape` with `builtin_shapes()`, the right UV, `base_size` `(w,h)` or `(2r,2r)`, and the `TextureRef.filter` override (circles only). Validate the radius as for width and height.
- [X] T019 [US2] Expose `pub(crate)` helpers in `crates/ferrocade-py/src/py_sprite.rs` so subclasses can set the id: `PySprite::empty()` and `fn set_id(&mut self, id)`. Reject `set_texture` on shape sprites with `TypeError("shape sprites use a built-in texture")`, detected by `base_size.is_some()` in the store.
- [X] T020 [US2] Create `crates/ferrocade-py/src/py_shapes.rs` (the T001 pattern, or its fallback).
  - `#[pyclass(extends = PySprite, subclass, name = "RectangleSprite")]` with `__init__(self_, width, height, color: Vec<i64>, center_x=0.0, center_y=0.0)`. It parses the color with `color::parse_rgba`, calls `store::create_rectangle`, and sets the base id.
  - `#[pyclass(extends = PySprite, subclass, name = "CircleSprite")]` with `__init__(self_, radius, color, center_x=0.0, center_y=0.0, *, filter: Option<PyFilter>=None)` and a read-only `radius` getter (`base radius × |scale|`).
  - Register both in `crates/ferrocade-py/src/lib.rs`.
- [X] T021 [US2] Export `RectangleSprite` and `CircleSprite` in `python/ferrocade/__init__.py`. Add their stubs (signatures exactly per contracts/python-api.md) to `python/ferrocade/_ferrocade.pyi`.
- [ ] T022 [US2] Create `examples/01c_shapes.py`, following the contract sketch (a rectangle, a `NEAREST` circle, a `LINEAR`-override circle, all in named colors; the tinted ship is added in T030).
  - With `--many`: 1,000 random shapes (mixed rectangles and circles, sizes 4–24, random CSS colors).
  - Print the average frame time to the console every 2 s (`time.perf_counter` in `on_draw`).
- [ ] T023 [US2] Add to `crates/ferrocade-web/src/lib.rs`: `create_shape_sprite(kind: &str /* "rectangle" | "circle" */, width, height, filter: &str, center_x, center_y, r, g, b, a) -> u32`, using `create_rectangle` or `create_circle`. Also add `draw_calls()`, which returns `last_frame_stats().draw_calls` (for the browser check).
- [ ] T024 [US2] Add `RectangleSprite(Sprite)` and `CircleSprite(Sprite)` to `web/python/ferrocade/__init__.py`, with the same signatures and validation as desktop (`ValueError`/`TypeError`). They're created through `create_shape_sprite`, and `CircleSprite` has `radius`.
- [ ] T025 [US2] Add `?example=<name>` to `web/loader.js` (default `01_centered_ship`). It fetches `../examples/<name>.py` and accepts only `[A-Za-z0-9_]` names. Keep `ferrocadeExampleSource` for the unchanged-source check.
- [ ] T026 [US2] Run T014–T016, `maturin develop`, `python examples/01c_shapes.py` (and `--many`, noting the frame times), and `tools\build_web.ps1`. Run `node tools/web-check/check.mjs "http://localhost:8000/web/?example=01c_shapes"` and compare its screenshot with a desktop screenshot of the same example (Q4, Q6, Q7).

**Checkpoint**: US2 is complete. Shapes draw on desktop and web in one batch.

---

## Phase 5: User Story 3 - Tint and fade any sprite (Priority: P3)

**Goal**: `sprite.color` read/write on every sprite (image or shape), on desktop and web.

**Independent Test**: tint the ship red and to half alpha, and confirm the change in a pixel test. Reset it to white, and confirm it's identical to the original.

- [X] T027 [P] [US3] Extend `crates/ferrocade/tests/render_offscreen.rs`, covering:
  - the ship with `set_color([255,0,0,255])`: every opaque ship pixel has G = B = 0 and R ≤ its original R, and transparent pixels are still the background
  - `set_color([255,255,255,128])` gives ship pixels about halfway between the original and the background (within ±3)
  - resetting to white makes the pixels identical to the untinted render
- [X] T028 [P] [US3] Add to `tests/test_sprite.py`:
  - the default `color == (255, 255, 255, 255)`
  - setting a 3-tuple gives alpha 255
  - a 4-tuple round-trips
  - `ValueError` for a 2-tuple, `(256, 0, 0)`, or `(-1, 0, 0)`
  - a shape's `color` can be changed after creation
- [X] T029 [US3] Add the `color` getter (returns a 4-tuple) and setter (accepts a 3- or 4-sequence, via `color::parse_rgba`, then `store::set_color`) to `crates/ferrocade-py/src/py_sprite.rs`. Add it to the `Sprite` stub in `python/ferrocade/_ferrocade.pyi`.
- [ ] T030 [US3] Add the web side: `set_color(sprite, r, g, b, a)` and `get_color(sprite) -> Vec<u8>` in `crates/ferrocade-web/src/lib.rs`, and the `color` property in `web/python/ferrocade/__init__.py`. Update `examples/01c_shapes.py` to add the ship tinted with `fc.color.LIGHT_PINK` (FR-015).
- [ ] T031 [US3] Run T027 and T028, rerun `python examples/01c_shapes.py`, and run the browser check on `?example=01c_shapes`. The ship should be tinted on both.

**Checkpoint**: all three stories are complete.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T032 Full regression and lint:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade --no-default-features --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade-web --target wasm32-unknown-unknown -- -D warnings`
  - `cargo test --workspace`, `maturin develop`, `pytest`
  - `python examples/01_centered_ship.py` (Q5: unchanged)
  - the web check on the default example (unchanged)
- [ ] T033 [P] Check `python/ferrocade/_ferrocade.pyi` against the runtime (no missing or extra names, the same comparison as Phase 1's T055) and against contracts/python-api.md.
- [ ] T034 Record Q1–Q7 results (including frame times, draw-call counts, and the Chrome color check) in `docs/research/phase1c-validation.md`, and link it from `docs/research/README.md`. Mark Phase 1c implemented in `docs/spec/ENGINE_SPEC.md` §10, with decision log entries for any deviations from the plan (for example, the R5 outcome).

---

## Dependencies & Execution Order

- **Setup (T001)** gates T019 and T020 (the binding pattern).
- **Foundational (T002–T007)** comes before every story. T003 → T004 → T005 (instance, then store, then shader). T006 is independent. T007 runs last.
- **US1 (T008–T013):** depends only on Foundational for T010's clear check. Otherwise it's independent of US2 and US3. T008 → T009 → T010/T011 → T012 → T013.
- **US2 (T014–T026):** depends on Foundational and T001. T017 → T018 → T019 → T020 → T021 → T022. The web work (T023–T025) comes after T018. T026 runs last.
- **US3 (T027–T031):** depends on Foundational. T029 can come before or after US2. T030 depends on T023 (web shapes) for the example update.
- **Polish (T032–T034):** last.

### Parallel Opportunities

- **US1:** T008, T010, T011 (different files).
- **US2 tests:** T014, T015, T016.
- **US3 tests:** T027, T028.
- **US1 alongside US2:** after Foundational, the color generation (US1) and the shape core (US2) touch different files.
- **Polish:** T033.

## Parallel Example: User Story 2

```text
Task: "T014 shape render tests in crates/ferrocade/tests/render_offscreen.rs"
Task: "T015 shape store tests in crates/ferrocade/tests/store.rs"
Task: "T016 shape API tests in tests/test_shapes.py"
```

## Implementation Strategy

1. T001 (de-risk the inheritance pattern), then Foundational (the tint core, with Phase 1 unchanged).
2. **US1 (MVP):** CSS colors, verified against Chrome.
3. **US2:** shapes on desktop, then web. Demo `01c_shapes`.
4. **US3:** the `color` property everywhere, plus the tinted ship in the example.
5. Polish, the validation record, and spec updates. Commit locally. **Push only on the owner's word.**

## Notes

- Constraints are quoted from data-model.md: "w, h finite and > 0", "48 bytes", the RECT_UV and CIRCLE_UV regions, and "the default color for new image sprites is white".
- If T001 fails, the fallback changes only T019 and T020. Record it in research.md.
