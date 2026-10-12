# Implementation Plan: Phase 1 — Window and Centered Ship

**Branch**: `001-centered-ship` (spec directory; work happens on `main` locally) | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-centered-ship/spec.md`

## Summary

Deliver the first working slice of Ferrocade:
- A Python developer subclasses `fc.Window`, loads `ship.png` as an `fc.Texture`, places an
  `fc.Sprite` at the window center, adds it to an `fc.SpriteList`, and sees it drawn.
- Technical approach:
  - A Python-free Rust core (`crates/ferrocade`) holds sprite data in a central store. Each list
    keeps GPU-layout f32 instance buffers with dirty tracking.
  - It renders 2D instanced quads with wgpu under WebGL2 limits, and runs an event-driven winit loop
    with a fixed timestep.
  - A thin PyO3 layer (`crates/ferrocade-py`) exposes the API and dispatches `on_draw`/`on_update` to
    Python subclasses.
  - Everything builds with maturin as an abi3 (3.13+) extension.
- Details: [research.md](research.md), [data-model.md](data-model.md),
  [contracts/python-api.md](contracts/python-api.md).

## Technical Context

**Language/Version**: Rust stable 1.99 (MSVC toolchain, edition 2024). Python ≥ 3.13 (abi3).

**Primary Dependencies**: wgpu 30, winit 0.30, PyO3 0.29, image 0.25, web-time 1.1, bytemuck,
pollster, slotmap, thiserror, log/env_logger. Build: maturin 1.15 (see [research.md R1](research.md#r1-dependency-versions)).

**Storage**: N/A (image files are read through the asset loader).

**Testing**:
- `cargo test -p ferrocade`: unit tests plus an offscreen GPU render test.
- `pytest tests`: Python API tests.
- Manual acceptance via `examples/01_centered_ship.py` (see [quickstart.md](quickstart.md)).

**Target Platform**: Windows 11 (prototype). The code stays portable to macOS and the web (constitution III).

**Project Type**: a library, consisting of a Rust engine crate, a PyO3 extension crate, and a Python package.

**Performance Goals**:
- A static scene runs at the monitor refresh rate.
- Drawing an unchanged list does zero per-sprite work and uploads nothing.
- Phase 3 sets numeric targets.

**Constraints**:
- WebGL2 downlevel limits and WGSL only.
- No blocking main loop, and async-friendly GPU init.
- No `std::time::Instant` and no required threads.
- Logical-pixel coordinates.

**Scale/Scope**: 4 public classes and a color module, about 2–3k lines of Rust plus about 200 lines of Python and stubs, and 1 example.

All Technical Context unknowns are resolved in research.md. Texture filtering has **no default**: `filter` is required on `Window`, and textures can override it (R11, owner decision). The Phase 1 example uses `fc.NEAREST`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre-research | Post-design |
|---|---|---|---|
| I. Python API, Rust Core | Public API is Python only. The core has no PyO3 dependency. The bindings are thin. | ✅ | ✅ All engine logic is in `crates/ferrocade`. `ferrocade-py` only converts arguments and dispatches callbacks (R2). |
| II. Minimize conversion | Data stored in Rust. f64 sprite data, f32 GPU buffers, dirty tracking. No per-sprite conversion when drawing. Batched setters. | ✅ | ✅ Central store plus per-list instance buffers (R5). Setters write the instances. Draw uploads the dirty range only. `position` tuple setter. |
| III. Portable by design | Platform code isolated. WebGL2 limits, WGSL. Event-driven loop, async init. `web-time`. Asset loader. No required threads. | ✅ | ✅ R3/R7/R8/R9. `pollster` and `run_app_on_demand` are confined to the desktop platform module (`platform/desktop.rs`). The global store uses a `Mutex` (works single-threaded on WASM). |
| IV. 2D only, simple first | No 3D or depth. Minimal features. | ✅ | ✅ No depth buffer. The angle is stored but not rendered. No color class (tuples). Cache without unloading. List `remove` is O(n). |
| V. Spec-driven decisions | Follows engine spec decisions. Open designs untouched. | ✅ | ✅ Matches §3, §5 to §9. Doesn't touch collisions (§11) or the browser runtime choice (§9). The texture filtering design (R11) was decided by the owner during planning. |
| VI. Phase-based delivery | Runnable example plus "done when" criteria. | ✅ | ✅ `examples/01_centered_ship.py` and quickstart V1 to V10. |

**Result**: PASS. No violations, so Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-centered-ship/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R1–R13
├── data-model.md        # Phase 1: entities, rules, states
├── quickstart.md        # Phase 1: build + validation scenarios V1–V10
├── contracts/
│   └── python-api.md    # Phase 1: public Python API contract
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 (/speckit-tasks; not created here)
```

### Source Code (repository root)

```text
Cargo.toml                      # workspace: crates/ferrocade, crates/ferrocade-py
pyproject.toml                  # maturin build config, requires-python >=3.13

crates/ferrocade/               # Rust core — NO PyO3 dependency
├── Cargo.toml
├── src/
│   ├── lib.rs                  # public core API, re-exports
│   ├── error.rs                # Error enum (thiserror): Io, Decode, Unsupported, NotInFrame, Gpu…
│   ├── assets.rs               # asset loader: read_bytes(path) (fs on desktop; web later)
│   ├── texture.rs              # TextureData, TextureId, TextureCache (path → Arc), Filter enum
│   ├── store.rs                # SpriteStore: SpriteState, ListState, memberships, dirty ranges
│   ├── instance.rs             # #[repr(C)] Instance (Pod), built from SpriteState
│   ├── frame.rs                # FrameQueue (thread-local): Clear / DrawList commands
│   ├── timestep.rs             # FixedTimestep::advance
│   ├── color.rs                # color parsing/validation helpers (0–255 → linear f32)
│   ├── window.rs               # WindowConfig, WindowState, GameCallbacks trait
│   ├── render/
│   │   ├── mod.rs              # Renderer: device/queue, surface or offscreen target, frame execution
│   │   ├── gpu_texture.rs      # lazy texture upload; nearest + linear samplers; bind groups per (TextureId, Filter)
│   │   ├── gpu_list.rs         # per-list instance buffer, growth, dirty upload, batch runs
│   │   └── sprite.wgsl         # instanced quad shader
│   └── platform/
│       ├── mod.rs              # run(config, callbacks) entry; cfg-selects backend
│       └── desktop.rs          # winit ApplicationHandler, run_app_on_demand, pollster init
└── tests/
    ├── store.rs                # membership, dirty range, removal, multi-list
    ├── texture_cache.rs        # read-once, error kinds/messages
    ├── timestep.rs             # accumulator + cap
    └── render_offscreen.rs     # renders ship.png list to texture; asserts centered bbox (skips w/o GPU)

crates/ferrocade-py/            # PyO3 bindings — thin
├── Cargo.toml                  # pyo3 = { features = ["abi3-py313"] }, crate-type = ["cdylib"]
└── src/
    ├── lib.rs                  # #[pymodule] _ferrocade: registers classes
    ├── py_window.rs            # Window pyclass(subclass): __new__/__init__, run, clear; PyCallbacks
    ├── py_texture.rs           # Texture pyclass: __init__(path, *, filter=None), props, __eq__/__hash__
    ├── py_filter.rs            # Filter enum pyclass (NEAREST, LINEAR) + module aliases
    ├── py_sprite.rs            # Sprite pyclass(subclass): props → store setters
    ├── py_sprite_list.rs       # SpriteList pyclass: append/remove/draw/len/contains/iter
    └── errors.rs               # core Error → FileNotFoundError / ValueError / RuntimeError

python/ferrocade/
├── __init__.py                 # re-exports Window, Texture, Sprite, SpriteList, Filter, NEAREST, LINEAR, color, __version__
├── color.py                    # named RGB tuples
├── _ferrocade.pyi              # type stubs matching contracts/python-api.md
└── py.typed

examples/01_centered_ship.py    # milestone example (run from repo root)

tests/                          # pytest
├── conftest.py                 # display-availability marker / skip logic
├── test_texture.py             # loading, sharing, errors, filter override equality (SC-006, SC-008)
├── test_sprite.py              # properties, exact round-trip, non-finite rejection, subclassing
├── test_sprite_list.py         # append/remove/contains/iter, multi-list, ordering
└── test_window.py              # subclassing, filter required (TypeError), draw/clear outside on_draw raises, callback exception
                                #   propagates, run() twice (display-marked)
```

**Structure Decision**:
- This follows the repository layout in engine spec §13: a Cargo workspace with the core and binding crates, a maturin-built Python package in `python/ferrocade`, and Python tests in `tests/`.
- Platform-specific code lives only in `crates/ferrocade/src/platform/`.
- The offscreen render path in `render/` lets SC-002 be verified automatically, without a window.

## Implementation Order (for /speckit-tasks)

1. **Scaffold and verify toolchain:**
   - workspace, both crates, `pyproject.toml`, an empty `_ferrocade` module importable via `maturin develop`
   - **Verify R4 early:** a Rust `__init__` on an abi3 subclassable pyclass, called from a Python subclass's `super().__init__(...)`
2. **Core data (no GPU):** `error`, `assets`, `texture` + cache, `store` + `instance`, `timestep`, `color`, `frame`, with Rust tests.
3. **Bindings for the data classes:** `Texture`, `Sprite`, `SpriteList`, plus stubs and pytest tests (all runnable without a window).
4. **Renderer:** device init (async fn), the shader, GPU textures, GPU list buffers and batching, frame execution, and the offscreen render test.
5. **Platform loop and `Window`:** winit app, surface, resize/minimize/scale handling, fixed timestep, callbacks, exception and Ctrl+C handling, `run()` twice.
6. **Example, quickstart validation (V1 to V10), and docs:** add the build/test commands to `CLAUDE.md`, and record any decisions in the engine spec decision log.

## Complexity Tracking

No constitution violations. This section is intentionally empty.
