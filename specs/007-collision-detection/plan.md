# Implementation Plan: Phase 5 — Collision Detection

**Branch**: `007-collision-detection` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/007-collision-detection/spec.md`

## Summary

Add detection-only collisions, following the owner-decided architecture (engine spec §11,
`docs/design/collision.md`):
- Explicitly attached `CircleCollider` / `BoxCollider` / `AABBCollider` objects, with coupled
  scale and rotating offsets.
- Four batched queries: `collides_with`, `collides_with_list`, `collisions_with`, `collisions`.
- A per-list uniform grid, cached by a list content version, so unchanged lists cost nothing.
- Exact narrow-phase tests in `f64`. Everything is written from scratch in a new core module,
  `crates/ferrocade/src/collision/`.

Also: a writable `Window.title` (needed for the example's hit count), browser support, a collision
benchmark against the owner's targets (10,000 moving sprites, all pairs < 2 ms; one vs. 10,000
< 0.5 ms), and `examples/05_collisions.py`. Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust stable (2024 edition), Python ≥ 3.13 (abi3)

**Primary Dependencies**: existing only (slotmap, smallvec, PyO3 0.29, wasm-bindgen 0.2.129). **No collision crates** (owner decision 6).

**Storage**: N/A

**Testing**: `cargo test` (narrow-phase exact cases, grid = all-pairs property tests, sampling reference), pytest (`tests/test_collision.py`), `benches/collisions.py` (release build), `check.mjs --collisions`

**Target Platform**: Windows (dev), macOS, browser (WebGPU/WebGL2), single-threaded core

**Project Type**: library (game engine) with Python bindings and a web build

**Performance Goals**: SC-002. 10,000 moving sprites in one list, all colliding pairs < 2 ms per query, including the Python result; one sprite vs. 10,000 < 0.5 ms. Scene density as defined in research R9. Unchanged lists: repeated queries ≈ 0 (SC-003).

**Constraints**: constitution II (one boundary crossing per query, no per-pair Python), III (no threads, WASM-safe, no `std::time`), IV (only three shapes, no physics response), V (architecture already owner-approved; details here are plan-level)

**Scale/Scope**: about 4 new core files (~900 lines including tests), bindings for 4 classes + 4 queries, web exports + shim, 1 example, 1 test file, 1 bench

## Constitution Check

*GATE: must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. Python API, Rust core | Shapes, the grid, narrow phase, and queries live in `crates/ferrocade/src/collision/` (no PyO3). The bindings hold ids and map slots to existing Python objects. | ✅ |
| II. Minimize conversion | One crossing per query. Transforms are read from Rust `f64` state. The grid is cached by version, so unqueried or unchanged lists cost nothing. A benchmark backs the claims (V3). | ✅ |
| III. Portable | Pure Rust math, no threads, no platform code. The browser build exposes the same queries. | ✅ |
| IV. 2D, simple first | Only circle, rotated box, and AABB. No response, events, or debug view (backlog). The shape enum is extensible without generic machinery. | ✅ |
| V. Spec-driven | Implements owner decisions (§11 and the decision log). Plan-level choices (grid parameters, version counter, writable title) are recorded here. The `Window.title` setter is a small API addition the spec's example requires; it's flagged in the completion report. | ✅ |
| VI. Phase delivery | `examples/05_collisions.py`, plus regression of earlier examples (desktop + browser). | ✅ |

**Post-design re-check**: still passes. No complexity exceptions.

## Project Structure

### Documentation (this feature)

```text
specs/007-collision-detection/
├── plan.md            # this file
├── research.md        # R1–R10
├── data-model.md      # Collider, world shapes, list cache, query results
├── quickstart.md      # V1–V9
├── contracts/
│   └── python-api.md  # collider classes, Sprite.collider, queries, writable title
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/ferrocade/src/
├── collision/
│   ├── mod.rs        # Collider, ColliderId, Shape, validation
│   ├── shape.rs      # WorldShape, world transform (scale, CCW rotation, offsets), bounds
│   ├── narrow.rs     # circle/box tests (touching counts) + exact tests + sampling-reference tests
│   ├── grid.rs       # uniform grid (counting sort, large-object list, reference-cell dedup)
│   └── query.rs      # collides, sprite_vs_list, list_vs_list, within_list; cache by version; property tests
├── store.rs          # collider slotmap; SpriteState.collider; attach/detach/resize; ListState.content_version + collision_cache
├── window.rs         # writable title (WindowState) composed with show_fps
├── platform/desktop.rs  # apply title changes
└── lib.rs            # pub mod collision

crates/ferrocade-py/src/
├── py_collider.rs    # NEW: Collider base + CircleCollider / BoxCollider / AABBCollider
├── py_sprite.rs      # collider property, collides_with, collides_with_list
├── py_sprite_list.rs # collisions_with, collisions
├── py_window.rs      # title setter
└── lib.rs            # register classes

crates/ferrocade-web/src/lib.rs     # collider + query exports
web/python/ferrocade/__init__.py    # collider classes, Sprite.collider, queries, title setter
python/ferrocade/__init__.py, _ferrocade.pyi
examples/05_collisions.py           # NEW
tests/test_collision.py             # NEW
benches/collisions.py               # NEW
tools/web-check/check.mjs           # --collisions
docs/spec/ENGINE_SPEC.md            # §11 status, §10 Phase 5 status, decision log
docs/research/phase5-validation.md  # NEW
```

**Structure Decision**: collision is a new core module tree. The store gains only the ids and
fields that tie colliders to sprites and lists.

## Complexity Tracking

No violations to justify.
