# Implementation Plan: Phase 2 — Bouncing Ship (Movement)

**Branch**: `004-bouncing-ship` (spec directory; work happens on `main` locally) | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-bouncing-ship/spec.md`

## Summary

- **Motion:** sprites gain `velocity_x`, `velocity_y`, `angular_velocity` (and a `velocity`
  tuple), stored in the Rust sprite store.
- **Update:** `SpriteList.update(delta_time)` advances the whole list in one engine call. Only
  sprites that actually move are marked dirty.
- **Rotation:** the shader starts using the instance's `angle` and rotates quads counter-clockwise
  around their centers.
- **Timing:** the fixed timestep is unchanged, but it's now proven from Python (rate under normal
  and slowed drawing).
- **Example:** `examples/02_bouncing_ship.py`: the ship bounces and spins, plus a `--many` mode
  with 1,000 bouncing shapes and a `--count` mode.
- **Web:** the same API in the browser.

## Technical Context

**Language/Version**: Rust stable 1.99; Python ≥ 3.13 (desktop), 3.14 (Pyodide 314).

**Primary Dependencies**: no new dependencies.

**Storage**: N/A.

**Testing**:
- `cargo test` (store motion, rotation pixel tests)
- `pytest` (API plus timing display tests)
- the browser check (motion between screenshots)

**Target Platform**: Windows desktop plus browsers (approach B).

**Project Type**: an engine library feature.

**Performance Goals**: 1,000 moving, bouncing sprites at the display refresh rate. The update is a single engine call per list.

**Constraints**:
- The instance stays 48 bytes.
- Angle 0 is pixel-identical to before.
- No per-sprite Python work for movement.
- WebGL2-compatible shader math.

**Scale/Scope**: about 250 lines of Rust and Python, 1 example, and tests.

There are no NEEDS CLARIFICATION items left. The rotation direction was decided by the owner: counter-clockwise.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre | Post-design |
|---|---|---|---|
| I. Python API, Rust Core | Motion math in Rust. Python API only. | ✅ | ✅ `update_list` lives in the core store. The bindings and web engine are thin wrappers. |
| II. Minimize conversion | Bulk per-frame work in Rust; dirty tracking. | ✅ | ✅ One call per list moves N sprites. Zero-velocity sprites aren't dirtied. The `velocity` tuple setter is a single crossing. |
| III. Portable | WGSL, WebGL2-safe, web parity. | ✅ | ✅ The rotation uses only `cos`/`sin` in the vertex stage. The web engine and shim get the same API. |
| IV. 2D, simple first | No physics creep. | ✅ | ✅ Velocities only: no acceleration, friction, or interpolation. Bounces stay in game code. |
| V. Spec-driven decisions | Owner decisions followed. | ✅ | ✅ Per-second units, fixed timestep, the velocity names (§8), and CCW rotation (decided 2026-10-12). |
| VI. Phase-based delivery | Example plus criteria. | ✅ | ✅ `examples/02_bouncing_ship.py` and B1–B7. |

**Result**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/004-bouncing-ship/
├── plan.md
├── research.md        # R1–R6
├── data-model.md
├── quickstart.md      # B1–B7
├── contracts/python-api.md
├── checklists/requirements.md
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/ferrocade/src/store.rs            # velocity fields + setters; update_list(list, dt)
crates/ferrocade/src/render/sprite.wgsl  # rotate corner offsets by angle (CCW)
crates/ferrocade/tests/store.rs          # motion, dirty, multi-list, dt validation
crates/ferrocade/tests/render_offscreen.rs  # rotation size + direction, angle-0 unchanged
crates/ferrocade-py/src/py_sprite.rs     # velocity_x/y, angular_velocity, velocity
crates/ferrocade-py/src/py_sprite_list.rs  # update(delta_time)
python/ferrocade/_ferrocade.pyi          # stubs
tests/test_motion.py                     # NEW: API + timing display tests
crates/ferrocade-web/src/lib.rs          # get/set velocity, set_angular_velocity, update_list
web/python/ferrocade/__init__.py         # shim properties + SpriteList.update
tools/web-check/check.mjs                # optional second screenshot (--motion) to prove movement
examples/02_bouncing_ship.py             # NEW: bounce + spin; --many; --count
```

**Structure Decision**: this extends existing files. The new files are one example and one test module.

## Implementation Order (for /speckit-tasks)

1. **Core:** velocity fields, then `update_list`, then the store tests.
2. **Shader rotation:** then the rotation pixel tests (size and direction), confirming angle 0 is unchanged.
3. **Bindings:** the properties, then `SpriteList.update`, then the stubs, then pytest (including the timing tests).
4. **Example:** default, `--many`, and `--count`. Measure.
5. **Web:** the engine methods, then the shim, then the browser motion check.
6. **Regression, lint, validation record, and spec updates.**

## Complexity Tracking

No violations.
