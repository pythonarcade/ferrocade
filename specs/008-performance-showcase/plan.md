# Implementation Plan: Performance Showcase

**Branch**: `008-performance-showcase` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/008-performance-showcase/spec.md`

## Summary

A walled scene of thousands of bouncing balls that shows rendering, movement, and collision speed:
- **Engine bouncing** through two explicit calls, `balls.bounce_each_other()` and
  `balls.bounce_off(walls)`: equal-mass elastic circles, resolved in one deterministic pass over
  the Phase 5 contacts.
- **A Python mode** that applies the same formulas from `collisions()` results, for comparison.
- **Engine-measured** `update_ms` and `collision_ms` in `frame_stats`, printed to the console once
  per second.
- Keys to add or remove balls, toggle the mode, and pause.
- A browser version.

The scene is 1280×720 with radius 2–4 px balls (this corrects an infeasible spec assumption; see
research R1). Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust stable (2024 edition), Python ≥ 3.13 (abi3)

**Primary Dependencies**: existing only (no physics crates)

**Storage**: N/A

**Testing**: `cargo test` (exact bounce formulas, conservation simulation), pytest (`tests/test_showcase.py`: parity, stats, example keys), `benches/showcase.py`, `check.mjs --showcase`

**Target Platform**: Windows (dev), macOS, browser

**Project Type**: library (game engine) with Python bindings and a web build, plus a showcase example

**Performance Goals**: SC-001, 10,000 balls at 60 FPS in engine mode on desktop with work < 14 ms. Expected budget: update ≈ 0.2 ms, collisions (within-list detect + resolve, wall pass) ≈ 1.5–2.5 ms, render ≈ 1–1.5 ms.

**Constraints**: constitution II (one call per bounce pass; no per-pair Python in engine mode), III (no threads; `web_time`), IV (equal-mass elastic circles only; no general physics), V (owner decisions in §10 and the decision log)

**Scale/Scope**: about 1 new core module (`collision/bounce.rs`), timing fields, bindings for 2 methods + 2 stats fields, web exports + shim, 1 example, 1 bench, 1 test file

## Constitution Check

*GATE: must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. Python API, Rust core | Bounce math and timing live in the core. The bindings forward one call each. | ✅ |
| II. Minimize conversion | Engine mode: two engine calls per update, no per-pair Python. The Python mode exists deliberately as the comparison (owner decision) and isn't the default. | ✅ |
| III. Portable | Pure Rust math; `web_time::Instant`; no threads. The browser gets the same calls. | ✅ |
| IV. 2D, simple first | Only equal-mass elastic circles and static walls, one pass per update; no solver, no mass or restitution. | ✅ |
| V. Spec-driven | Implements the owner's showcase decisions. The scene and ball size correction (R1) changes my spec assumption, not an owner decision, and is flagged in the completion report. | ✅ |
| VI. Phase delivery | `examples/06_showcase.py`, plus regression of earlier examples. | ✅ |

**Post-design re-check**: still passes. No complexity exceptions.

## Project Structure

### Documentation (this feature)

```text
specs/008-performance-showcase/
├── plan.md
├── research.md        # R1–R8
├── data-model.md
├── quickstart.md      # V1–V11
├── contracts/
│   └── python-api.md
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/ferrocade/src/
├── collision/bounce.rs   # NEW: bounce_within, bounce_off (closest point on wall shapes), tests
├── collision/mod.rs      # pub mod bounce
├── store.rs              # update/collision timing accumulators (Cell<f64>) + take_frame_timings
├── window.rs             # FrameSnapshot/RollingStats: update_ms, collision_ms
└── platform/desktop.rs   # pass the frame timings into record_frame

crates/ferrocade-py/src/
├── py_sprite_list.rs     # bounce_each_other, bounce_off
└── py_window.rs          # FrameStats.update_ms / collision_ms

crates/ferrocade-web/src/lib.rs     # bounce exports; timings in end_frame/frame_stats
web/python/ferrocade/__init__.py    # bounce methods, FrameStats fields
python/ferrocade/_ferrocade.pyi
examples/06_showcase.py             # NEW
tests/test_showcase.py              # NEW
benches/showcase.py                 # NEW
tools/web-check/check.mjs           # --showcase
specs/008-performance-showcase/spec.md   # assumptions corrected (R1)
docs/spec/ENGINE_SPEC.md, docs/research/showcase-validation.md
```

**Structure Decision**: bouncing belongs with collision (`collision/bounce.rs`), because it consumes
the Phase 5 contacts and world shapes.

## Complexity Tracking

No violations to justify.
