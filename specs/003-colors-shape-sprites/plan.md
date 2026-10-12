# Implementation Plan: Phase 1c — CSS Colors and Shape Sprites

**Branch**: `003-colors-shape-sprites` (spec directory; work happens on `main` locally) | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-colors-shape-sprites/spec.md`

## Summary

**Colors:**
- `fc.color` gains all 148 CSS named colors (UPPER_SNAKE names, exact CSS values), plus
  `TRANSPARENT` and `DEFAULT_BACKGROUND`.
- The file is generated once, verified against Chrome's own CSS engine, and shared between desktop
  and web.

**Tint:**
- Every sprite gets a `color` tint, stored as 4 sRGB bytes in the GPU instance's spare padding, so
  the instance stays 48 bytes.
- The shader converts the tint to linear and multiplies, giving exact output colors.

**Shapes:**
- New `fc.RectangleSprite` and `fc.CircleSprite` classes (they extend `Sprite`) use **one built-in
  shape atlas** (a white region plus an anti-aliased disc), selected with `uv_rect`.
- So any mix of rectangles and circles in a list draws in **one draw call**.
- Circles follow the window filter, unless created with `filter=`.
- The same API goes into the web shim and engine.

## Technical Context

**Language/Version**: Rust stable 1.99; Python ≥ 3.13 (desktop), Python 3.14 in Pyodide 314 (web).

**Primary Dependencies**: no new dependencies. Existing: wgpu 30, winit 0.30, PyO3 0.29, image 0.25, wasm-bindgen 0.2.129.

**Storage**: N/A.

**Testing**:
- `cargo test --workspace`: tint exactness, shape geometry, the batching count (1,000 shapes = 1 draw call), and Phase 1 pixel tests unchanged.
- `pytest`: the color set, `Sprite.color`, the shape API, and subclassing.
- `tools/web-check/check_colors.mjs`: CSS values against Chrome.
- The headless browser check for web parity.

**Target Platform**: Windows desktop (prototype) and browsers (approach B). Portability rules as before.

**Project Type**: an engine library feature (core, PyO3 bindings, web shim and engine).

**Performance Goals**: 1,000 mixed shapes render in a single draw call and at the display refresh rate (SC-004). No instance-size growth.

**Constraints**:
- Image sprites with the default tint must stay pixel-identical (SC-005).
- WebGL2-compatible vertex formats (`Unorm8x4`).
- No new shader pipeline.

**Scale/Scope**: about 400 lines of Rust and Python plus a generated 160-line color module, 1 example, and tests.

There are no NEEDS CLARIFICATION items left. Both open API questions were answered by the owner (shape classes; circle filter follows the window, with an override).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre | Post-design |
|---|---|---|---|
| I. Python API, Rust Core | Python-only API. Core logic in Rust. | ✅ | ✅ The tint, base size, built-in atlas, and batching all live in `crates/ferrocade`. The bindings add thin classes. |
| II. Minimize conversion | Data in Rust, dirty tracking, no instance growth. | ✅ | ✅ `color` lives in `SpriteState` and is written into the instance on set (dirty slot only). The instance stays 48 bytes. One draw call for mixed shapes. |
| III. Portable | WebGL2 formats, WGSL, web parity. | ✅ | ✅ `Unorm8x4` is WebGL2-valid. The shader stays WGSL. The web shim and engine get the same API. One color source for both. |
| IV. 2D, simple first | No over-generalization. | ✅ | ✅ Filled rectangles and circles only. A fixed base size. No SDF shader. |
| V. Spec-driven decisions | Owner decisions followed. | ✅ | ✅ CSS names and values, `DEFAULT_BACKGROUND`, shape classes, and the circle filter rule all follow engine spec §3. |
| VI. Phase-based delivery | Example plus done criteria. | ✅ | ✅ `examples/01c_shapes.py` and Q1–Q7. |

**Result**: PASS. Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/003-colors-shape-sprites/
├── plan.md
├── research.md        # R1–R8
├── data-model.md
├── quickstart.md      # Q1–Q7
├── contracts/python-api.md
├── checklists/requirements.md
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
tools/gen_css_colors.py                 # NEW: generates python/ferrocade/color.py (148 CSS + extras)
tools/web-check/check_colors.mjs        # NEW: compares fc.color against Chrome's CSS engine
python/ferrocade/color.py               # REGENERATED: CSS colors, TRANSPARENT, DEFAULT_BACKGROUND
python/ferrocade/__init__.py            # export RectangleSprite, CircleSprite
python/ferrocade/_ferrocade.pyi         # Sprite.color, RectangleSprite, CircleSprite

crates/ferrocade/src/color.rs           # DARK_GRAY -> DEFAULT_BACKGROUND
crates/ferrocade/src/instance.rs        # color: [u8;4] at offset 20 (still 48 bytes)
crates/ferrocade/src/store.rs           # SpriteState.color / base_size / uv_rect; set_color; create_shape
crates/ferrocade/src/texture.rs         # builtin_shapes() atlas (cached under "<builtin:shapes>"), regions
crates/ferrocade/src/render/sprite.wgsl # location 4 color (unorm8x4): srgb->linear, multiply
crates/ferrocade/src/render/sprites.rs  # vertex attribute for color; FrameStats counting
crates/ferrocade/src/render/mod.rs      # last_frame_stats()
crates/ferrocade/tests/render_offscreen.rs  # tint exact, shapes geometry, batching = 1 draw call
crates/ferrocade/tests/store.rs         # color/base_size dirty + validation

crates/ferrocade-py/src/py_sprite.rs    # color property; PySprite::empty(); expose internals to subclasses
crates/ferrocade-py/src/py_shapes.rs    # NEW: RectangleSprite / CircleSprite (extends=PySprite, subclass)

crates/ferrocade-web/src/lib.rs         # create_shape_sprite, set_color, get_color
web/python/ferrocade/__init__.py        # Sprite.color, RectangleSprite, CircleSprite
web/python/ferrocade/color.py           # DELETED (the loader fetches python/ferrocade/color.py)
web/loader.js                           # fetch shared color.py; ?example= selects which example to run

examples/01c_shapes.py                  # NEW: shapes + tint; --many for the 1,000-shape scene
tests/test_color.py                     # NEW
tests/test_shapes.py                    # NEW
```

**Structure Decision**:
- This extends the existing layout. The only new modules are `py_shapes.rs` and the generator and check tools.
- The web copy of `color.py` goes away (single source).

## Implementation Order (for /speckit-tasks)

1. **Verify R5 first:** `#[pyclass(extends = PySprite, subclass)]` with `__new__(*args)` + `__init__` under abi3, subclassed from Python.
2. **Colors:** generator, then `color.py`, then the Rust rename, then tests, then the Chrome check.
3. **Core:** instance color, `SpriteState` color/base_size/uv_rect, the built-in atlas, the shader and vertex attribute, frame stats, then Rust tests (including Phase 1 pixel tests unchanged).
4. **Bindings:** `Sprite.color`, then `RectangleSprite`/`CircleSprite`, then stubs and exports, then pytest.
5. **Example:** `examples/01c_shapes.py` (plus `--many`).
6. **Web:** engine methods, then the shim classes, then the loader's shared `color.py` and `?example=`, then a parity screenshot.
7. **Regression and docs:** the full suites, clippy (desktop, no-render, wasm), and the engine spec decision log for any deviations.

## Complexity Tracking

No violations.
