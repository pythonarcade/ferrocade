# Research: Phase 5 — Collision Detection

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12
**Architecture (owner-decided)**: engine spec §11, [docs/design/collision.md](../../docs/design/collision.md)

## R1. Where colliders live in the core

- **Decision**: colliders are entries in a new slotmap inside the existing `SpriteStore`
  (`ColliderId`), next to sprites and lists. `SpriteState` gains `collider: Option<ColliderId>`, and
  each `Collider` records `attached_to: Option<SpriteId>`. The code lives in a new module tree,
  `crates/ferrocade/src/collision/` (`shape.rs`, `narrow.rs`, `grid.rs`, `query.rs`), with no PyO3.
- **Rationale**: queries need sprite transforms and list membership, which already sit behind the
  store's single lock. A separate store would need two locks per query. Separate ids keep colliders
  "separate entities" (owner decision 1), with their own Python objects and lifetimes.
- **Alternatives**: collider fields inlined in `SpriteState` (that contradicts decision 1 and blocks
  colliders without sprites later); a second global store (lock juggling).

## R2. Shapes and world transforms (expandable)

- **Decision**: `enum Shape { Circle { radius }, Box { width, height }, Aabb { width, height } }`
  plus `offset: (f64, f64)`, all in unscaled sprite pixels. A world shape is computed per query from
  the sprite's `f64` center, `scale`, and `angle` (degrees, counter-clockwise):
  - **Circle:** the center is sprite center + R(angle)·(offset·|scale|), and the radius is
    radius·|scale|.
  - **Box:** the same center rule; half extents are (w, h)/2·|scale|; its axes are the sprite angle.
  - **AABB:** the center is sprite center + offset·|scale| (no rotation); half extents are scaled.
  
  `enum WorldShape { Circle { c, r }, Obb { c, half, cos, sin } }`. An AABB is an OBB with
  cos = 1, sin = 0. A `Box` whose angle is a multiple of 90° is also normalized to axis-aligned
  (swapping half extents at 90°/270°), so it gets the fast test.
- **Expandability**: a new shape adds a `Shape` variant, a world-transform arm, a bounds arm, and
  narrow-phase arms. Convex shapes can later share a generic SAT/GJK path (design doc §2).
- **Alternatives**: trait objects per shape (dynamic dispatch, and pair tests still need a match);
  storing world shapes on every sprite and updating them on every move (that costs work even for
  sprites nobody queries; FR-012).

## R3. Narrow phase (exact, touching counts)

- **Circle–circle**: |c₁ − c₂|² ≤ (r₁ + r₂)².
- **Circle–box** (rotated or axis-aligned): rotate the circle center into the box frame, clamp it to
  the half extents, and collide if the clamped distance² ≤ r².
- **Box–box**: the separating axis test on the 2 + 2 face axes (only 2 when both are axis-aligned).
  Touching means the projections overlap with ≤.
- All math in `f64`. The tests use exact cases at the boundary (touching, a 1e-9 gap) plus
  randomized comparisons against an independent reference (R8).

## R4. Broad phase: a uniform grid per list, cached by version

- **Decision**: each `ListState` gets a `content_version: u64`, incremented whenever its instance
  data is marked dirty. That already happens on every position, scale, angle, texture, or color
  change and on `update_list` movement. Structural changes (append/remove) also bump it, and so do
  collider attach, detach, and resize (through the sprite's lists). A query looks at the list's
  `CollisionCache { version, world shapes + bounds per slot, grid }` and rebuilds only if the
  version changed (FR-012, SC-003).
- **Grid**: dense, over the bounding box of all collider bounds. Cell size = 2 × the median bounds
  extent (at least 1 px), with dimensions capped at 1024×1024 (the cell size grows if needed).
  Objects are bucketed with a two-pass counting sort into flat arrays (cell start offsets + object
  indices): no hashing, cache-friendly, and no allocation after warm-up.
- **Oversized objects**: an object covering more than 16 cells goes into a separate "large" list
  that is tested against everything. This keeps mixed sizes correct without blowing up cell lists.
- **Small lists**: below 32 colliders, queries skip the grid and test every pair.
- **Pair deduplication**: a pair sharing several cells is reported only from the cell containing the
  pair's bounds-intersection minimum corner (the standard reference-cell trick), so there are no
  hash sets. Results are then sorted by (first slot, second slot) for the deterministic order
  FR-007/FR-008 require.
- **Alternatives**: a hashed grid (slower to build; not needed for bounded 2D scenes); sort-and-sweep
  (good, but not the owner's choice); a BVH (backlog, for static level geometry).

## R5. Queries and returning Python objects

- **Core API** (`collision::query`): `collides(a, b) -> bool`, `sprite_vs_list(sprite, list) ->
  Vec<u32>` (slots, ascending, excluding the sprite itself), `list_vs_list(a, b) -> Vec<(u32, u32)>`
  (the grid is built on `b`, and each collider in `a` probes it), `within_list(list) -> Vec<(u32, u32)>`.
- **Bindings**: `PySpriteList` already keeps `refs: Vec<Py<PySprite>>` in list order, so slot
  indices map straight to the existing Python objects. Each query is one store lock and one
  boundary crossing, and it builds a Python `list` of sprites or of 2-tuples (FR-010).
- **Snapshots**: results are new Python lists. Later changes don't affect them.

## R6. Collider lifetime and Python objects

- **Decision**: a base `Collider` pyclass (`subclass`) with `CircleCollider`, `BoxCollider`, and
  `AABBCollider` as `extends=Collider` classes, holding a `ColliderId`. The pattern matches
  `RectangleSprite`/`CircleSprite` extending `Sprite`.
  - `sprite.collider` is a property on `PySprite`, which keeps a `Py<PyCollider>` so the object stays
    alive (and identical) while attached.
  - Setting it attaches in the core: the core raises an error if the collider is attached to another
    sprite (assumption in the spec). `None` detaches.
  - Dropping the Python collider (it can only be dropped once it's detached) destroys the core entry.
    Dropping a sprite detaches its collider.
- **Properties**: `radius` / `width` / `height` / `offset` are readable and writable, validated as
  finite and ≥ 0 (FR-011). Writes bump the attached sprite's lists' versions.

## R7. Window title for the example's hit count

- **Need**: FR-014 shows the hit count in the window title, but `Window.title` is read-only today.
- **Decision**: make `Window.title` writable. The desktop loop applies a changed title on the next
  frame through `WindowState`, and composes it with the `show_fps` readout. The web shim sets
  `document.title`. This is a small API addition required by the spec (noted for the owner in the
  plan summary).

## R8. Verifying correctness (SC-001)

- **Rust**: narrow-phase unit tests with exact boundary cases for every shape pair, rotations at
  0/45/90/180°, and scale and offset cases from the spec's acceptance scenarios. Also property
  tests (deterministic pseudo-random): (a) grid queries equal the all-pairs narrow phase for
  thousands of random mixed shapes, sizes over two orders of magnitude, and rotations/scales/offsets;
  (b) the narrow phase agrees with an independent **sampling reference** (dense point sampling of
  both shapes' areas, skipping cases within a small tolerance of touching).
- **Python** (`tests/test_collision.py`): the spec's acceptance scenarios with known geometry, plus a
  randomized check that `collisions()` equals the brute force over `collides_with` for every pair
  (the reference written in the test, as the spec requires).

## R9. Performance target (SC-002) and how it's measured

- **Targets (owner)**: all colliding pairs among 10,000 moving sprites in one list < 2 ms per frame;
  one sprite vs. 10,000 < 0.5 ms. Measured on the dev machine with a release build.
- **Scene definition (assumption)**: the number of returned pairs depends heavily on density, and
  building Python tuples for hundreds of thousands of pairs would dominate any engine. The
  benchmark uses a **showcase-like density**: 10,000 circle and box colliders of 4–12 px, spread
  over 1920×1080 with random velocities and rotations, moved by `SpriteList.update` every frame (so
  the grid rebuilds every frame). That's about 2,000–4,000 overlapping pairs. The tool also reports
  the same scene packed into 800×600 (far denser) for information.
- **Measured**: Python-side wall time of the query call, including returning results (what a game
  pays), plus the engine-internal time split into rebuild and query. Median and p95 over 300 frames.
- **Tool**: `benches/collisions.py` (scenes `pairs_10k`, `sprite_vs_10k`, `pairs_10k_dense`), printing
  pass/fail against the targets. Browser numbers are informational.

## R10. Browser

- The web crate gets collider and query exports: `create_collider(kind, a, b, ox, oy) -> handle`,
  setters, `set_sprite_collider(sprite, collider_or_minus_1)`, `collides(a, b)`,
  `collides_with_list(sprite, list) -> Uint32Array`, `collisions_with(list_a, list_b)` and
  `collisions(list) -> Uint32Array` (flat pairs). The shim maps slots to its `SpriteList._sprites`.
  One call per query.
- `check.mjs --collisions`: the example seeds its random layout (a fixed seed), so runs are
  repeatable. The check holds an arrow key for 2 s, which crosses the asteroid field, then reads the
  hit count from `document.title` and asserts it went up.
