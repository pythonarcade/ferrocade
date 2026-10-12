---

description: "Task list for Phase 5 — Collision Detection"
---

# Tasks: Phase 5 — Collision Detection

**Input**: Design documents from `specs/007-collision-detection/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included. SC-001 (exact against references), SC-002/SC-003 (benchmark), and SC-004 (example checks) require them.

**Organization**:
- **US1 (P1):** colliders on sprites
- **US2 (P1):** sprite queries (`collides_with`, `collides_with_list`)
- **US3 (P2):** pair queries (`collisions_with`, `collisions`)
- **US4 (P2):** the example, on desktop and in the browser

The shape math (world transforms, narrow phase) and the store wiring are foundational.

**Owner-approved for this phase**: a writable `Window.title`; the benchmark density from research R9.

## Format: `[ID] [P?] [Story] Description`

**Global rules**:
- No new crates.
- All collision math is `f64`, and touching counts as colliding.
- Angles are degrees, counter-clockwise.
- Run Rust commands with `export PATH="$HOME/.cargo/bin:$PATH"`. Run `maturin develop` before pytest after Rust changes.
- Never hold the store lock while calling Python.

---

## Phase 1: Setup

- [ ] T001 Create the module skeleton `crates/ferrocade/src/collision/{mod.rs, shape.rs, narrow.rs, grid.rs, query.rs}` and add `pub mod collision;` to `crates/ferrocade/src/lib.rs`. It must compile without the `render` feature.

---

## Phase 2: Foundational (blocks all stories)

- [ ] T002 In `crates/ferrocade/src/collision/mod.rs`, define:
  - `slotmap::new_key_type! { pub struct ColliderId; }`
  - `pub enum Shape { Circle { radius: f64 }, Box { width: f64, height: f64 }, Aabb { width: f64, height: f64 } }`
  - `pub struct Collider { pub shape: Shape, pub offset: (f64, f64), pub(crate) attached_to: Option<SpriteId> }`
  - `pub fn validate(shape, offset) -> Result<()>`: radius/width/height "finite and ≥ 0", offset components "finite", else `Error::InvalidArgument` with a clear message
- [ ] T003 In `crates/ferrocade/src/collision/shape.rs`:
  - `pub enum WorldShape { Circle { c: (f64, f64), r: f64 }, Obb { c, half: (f64, f64), cos: f64, sin: f64 } }` and `pub struct Bounds { min, max }`
  - `pub fn world_shape(collider: &Collider, center, scale, angle_deg) -> WorldShape`:
    - Circle and Box offsets are multiplied by |scale| and rotated counter-clockwise by the angle.
    - AABB offsets are scaled, not rotated.
    - Sizes are multiplied by |scale|.
    - A Box at a multiple of 90° (within 1e-9 after normalizing to [0, 360)) becomes axis-aligned (cos = 1, sin = 0), with half extents swapped at 90°/270°.
  - `bounds(&WorldShape) -> Bounds`
  - Unit tests for spec US1 scenarios 2–5 (rotation, AABB not rotating, scale 2 → radius 20, offset (20, 0) at 90° → above the center; the AABB offset stays (20, 0)).
- [ ] T004 In `crates/ferrocade/src/collision/narrow.rs`, implement `pub fn overlaps(a: &WorldShape, b: &WorldShape) -> bool`:
  - circle–circle: `d² ≤ (r₁ + r₂)²`
  - circle–box: rotate into the box frame, clamp, `≤ r²`
  - box–box: separating axis test (2 axes if both are axis-aligned, else 4), with `≤` overlap
  
  Unit tests:
  - exact touching, and a 1e-9 gap, for each pair type
  - spec US1 scenario 1 (radius 10 at x=119 hits, at x=121 misses)
  - boxes at 0/45/90/180°
  - zero-size shapes
  - a deterministic pseudo-random comparison against a **point-sampling reference** (sample a dense grid over the bounds' intersection; skip cases where the sampled minimum gap is within a tolerance of 0)
- [ ] T005 In `crates/ferrocade/src/store.rs`:
  - add `colliders: SlotMap<ColliderId, Collider>` to `SpriteStore` and `collider: Option<ColliderId>` to `SpriteState`
  - add `content_version: u64` to `ListState`, incremented in `mark_dirty` and on append/remove
  - add `collision_cache: Option<collision::query::CollisionCache>` (placeholder type until T012)
  - methods:
    - `create_collider(shape, offset) -> Result<ColliderId>` (validated)
    - `destroy_collider(id)` (detaches first)
    - `set_collider_shape(id, shape)` and `set_collider_offset(id, offset)` (validated; bump the attached sprite's lists' `content_version`)
    - `attach_collider(sprite, Option<ColliderId>) -> Result<()>`: `InvalidArgument` "collider is already attached to another sprite" if it's attached elsewhere; replacing detaches the previous one; bumps the versions
    - `collider(id) -> Result<&Collider>`
  - `destroy_sprite` detaches the sprite's collider
  - Rust tests: attach/replace/detach, the double-attach error, a version bump on move/attach/resize, and destroying a sprite detaches

**Checkpoint**: `cargo test -p ferrocade` and clippy (desktop + `--no-default-features`) pass.

---

## Phase 3: User Story 1 — Give sprites colliders (P1) 🎯 MVP part 1

**Goal**: the Python collider classes and `Sprite.collider`.

**Independent test**: attach, replace, detach; sizes, offsets and errors (V2, US1 part).

- [ ] T006 [US1] Create `crates/ferrocade-py/src/py_collider.rs`:
  - a `#[pyclass(subclass, name = "Collider")]` base holding `id: Option<ColliderId>`, with properties `offset` (get/set, a tuple) and `sprite` (get, `Sprite | None`; store a `Weak`-free approach: keep `Option<Py<PySprite>>` set and cleared by the sprite's `collider` setter)
  - `CircleCollider(radius, *, offset=(0, 0))` with `radius` get/set
  - `BoxCollider(width, height, *, offset=(0, 0))` and `AABBCollider(width, height, *, offset=(0, 0))` with `width`/`height` get/set; all `extends=Collider`, using the `__new__(*args, **kwargs)` + `__init__` pattern from `py_shapes.rs`
  - `Drop` destroys the core collider
  - `__repr__` for each class
  - register them in `crates/ferrocade-py/src/lib.rs` and export them from `python/ferrocade/__init__.py`
  
  **Watch for a reference cycle:** the sprite holds the collider and the collider references the sprite. Either implement `__traverse__`/`__clear__` on both, or make `Collider.sprite` look up the owner through the store, with the sprite → Python object mapping kept by `PySprite`. Prefer the simplest approach that leaks nothing (check with `gc` and `weakref` in a test).
- [ ] T007 [US1] In `crates/ferrocade-py/src/py_sprite.rs`, add the `collider` property (get → the attached Python collider or `None`; set → `store.attach_collider`, keeping the Python object alive; `None` detaches). It's inherited by shape sprites.
- [ ] T008 [US1] Add stubs to `python/ferrocade/_ferrocade.pyi` for the `Collider` base, the three classes, and `Sprite.collider`, with docstrings (unscaled sprite pixels, coupled scale, offset rotation rules, one sprite at a time).
- [ ] T009 [US1] Create `tests/test_collision.py` with the US1 tests (no display needed):
  - construct each class; read and write sizes and offsets
  - `ValueError` for a negative, NaN or infinite size or offset; zero size is allowed
  - attach, replace (the old collider's `.sprite` becomes `None`), detach with `None`
  - attaching to a second sprite raises `ValueError`
  - shape sprites have no collider by default
  - deleting a sprite frees its collider for reuse
  - no leak (a `weakref` to a detached collider dies after `del` + `gc.collect()`)

---

## Phase 4: User Story 2 — Ask what a sprite is touching (P1) 🎯 MVP part 2

**Goal**: `collides_with`, `collides_with_list`.

**Independent test**: known geometry; the ship among 1,000 sprites returns exactly the overlapping ones (V2, US2 part).

- [ ] T010 [US2] In `crates/ferrocade/src/collision/grid.rs`, implement `Grid::build(bounds: &[Option<Bounds>]) -> Grid`:
  - a dense grid over all bounds, cell size = 2 × the median bounds extent (≥ 1 px), dims ≤ 1024 per axis (grow the cell size if needed)
  - a two-pass counting sort into `cell_start: Vec<u32>` + `items: Vec<u32>`
  - objects covering more than 16 cells go to `large: Vec<u32>`
  - `Grid::query(bounds, visit: impl FnMut(u32))`: visits each candidate at most once per query, using the reference-cell rule (report from the cell containing the max of the two mins), with large objects always visited
  - `Grid::pairs(visit: impl FnMut(u32, u32))`: each candidate pair once (i < j), via the reference-cell rule, plus large × everything
- [ ] T011 [US2] In `crates/ferrocade/src/collision/query.rs`:
  - `pub struct CollisionCache { version, shapes: Vec<Option<WorldShape>>, bounds: Vec<Option<Bounds>>, grid: Option<Grid> }`
  - `fn cache(store, list) -> &CollisionCache`: rebuild when `content_version` changed; no grid below 32 colliders
  - `pub fn collides(store, a: SpriteId, b: SpriteId) -> Result<bool>` (`false` if either has no collider, or if a == b)
  - `pub fn sprite_vs_list(store, sprite, list) -> Result<Vec<u32>>`: ascending slots, excluding the sprite itself
  - if the borrow structure makes caching inside `ListState` awkward, keep caches in a `HashMap<ListId, CollisionCache>` on the store and drop the entry in `destroy_list`
- [ ] T012 [US2] Rust property tests in `crates/ferrocade/src/collision/query.rs`: for seeds × (n = 10, 100, 2,000) random mixed shapes (sizes 1–200 px, random rotations, scales 0–3, offsets), `sprite_vs_list` equals brute-force `overlaps` over all slots. Also: a static list isn't rebuilt on repeated queries (check that the cache version didn't change and expose a rebuild counter under `cfg(test)`).
- [ ] T013 [US2] Add `collides_with(other)` and `collides_with_list(sprite_list)` to `crates/ferrocade-py/src/py_sprite.rs`. Map slots to the list's `refs` (add a crate-visible accessor on `PySpriteList`). Hold the store lock only while computing slots, then build the Python list. Add stubs.
- [ ] T014 [US2] Add US2 tests to `tests/test_collision.py`:
  - spec US2 scenarios (overlap/no overlap; 1,000 asteroids with 3 overlapping → exactly those 3 in list order; the ship in its own list isn't reported; `collides_with(self)` is `False`)
  - sprites without colliders are never reported
  - results are snapshots (removing sprites while looping)
  - moving a sprite between queries updates results

---

## Phase 5: User Story 3 — Pair queries (P2)

**Goal**: `collisions_with`, `collisions`.

**Independent test**: equal to the brute force over `collides_with` (V2, US3 part).

- [ ] T015 [US3] In `crates/ferrocade/src/collision/query.rs`:
  - `pub fn list_vs_list(store, a, b) -> Result<Vec<(u32, u32)>>`: probe b's grid with each collider in a; skip identical sprites; sort by (slot_a, slot_b)
  - `pub fn within_list(store, list) -> Result<Vec<(u32, u32)>>`: `Grid::pairs` + narrow phase; i < j; sorted
  - property tests: both equal brute force for random scenes, including sprites present in both lists and mixed sizes with large objects
- [ ] T016 [US3] Add `collisions_with(other_list)` and `collisions()` to `crates/ferrocade-py/src/py_sprite_list.rs`, returning `list[tuple[Sprite, Sprite]]` from `refs`. Add stubs.
- [ ] T017 [US3] Add US3 tests to `tests/test_collision.py`:
  - bullets × asteroids pairs
  - pairs within one list: exactly once, never self, never both orders
  - deterministic order across repeated calls
  - a randomized scene (300 sprites, mixed shapes, rotations, scales, offsets): `collisions()` equals the brute-force set over `collides_with`, and so does `collisions_with`

---

## Phase 6: User Story 4 — The example, on desktop and in the browser (P2)

**Goal**: `examples/05_collisions.py` runs on both platforms. The benchmark meets the targets.

**Independent test**: injected keys produce hits (V4), the browser check (V5), and the benchmark (V3).

- [ ] T018 [US4] Writable `Window.title`:
  - `WindowState` gets `title: String` and a `title_changed` flag
  - `desktop.rs` applies a changed title once per frame, composed with the `show_fps` readout (`fps_title(current_title, ...)`)
  - `py_window.rs` gets the setter; the stub is updated
  - the web shim setter sets `self._title` and `document.title`, and its show_fps refresh uses `self._title`
  - test in `tests/test_window.py`: setting the title inside `on_update` runs without error, and `title` reads back the new value
- [ ] T019 [US4] Create `examples/05_collisions.py`:
  - Phase 4 controls (copy the movement logic from `04_keyboard_ship.py`); the ship gets a `BoxCollider` of its unscaled size × 0.8
  - `random.seed(5)`; about 150 drifting, slowly spinning asteroids that wrap at the edges, mixing `CircleSprite` + `CircleCollider`, `RectangleSprite` + `BoxCollider`, and `RectangleSprite` + `AABBCollider` (a different color per collider type)
  - each `on_update`: `for rock in self.ship.collides_with_list(self.rocks): self.rocks.remove(rock)`; `self.hits += 1`; `self.title = f"Collisions — hits: {self.hits}"`
  - Space re-centers the ship
  - a docstring with controls and the color legend
  - it must run unchanged in the browser
- [ ] T020 [US4] Add an example test to `tests/test_collision.py` (load it like the Phase 4 example test): inject RIGHT for ~2 s from the center and assert `hits > 0` and `len(rocks)` decreased by `hits`.
- [ ] T021 [US4] Browser support:
  - `crates/ferrocade-web/src/lib.rs` exports `create_collider(kind: &str, a: f64, b: f64, ox: f64, oy: f64) -> u32`, `set_collider_shape`, `set_collider_offset`, `destroy_collider`, `set_sprite_collider(sprite: u32, collider: i32)` (−1 = none), `collides(a, b) -> bool`, `collides_with_list(sprite, list) -> Vec<u32>`, `collisions_with(list_a, list_b) -> Vec<u32>` (flat pairs), and `collisions(list) -> Vec<u32>`
  - `web/python/ferrocade/__init__.py` gets the `Collider`/`CircleCollider`/`BoxCollider`/`AABBCollider` classes (validation messages matching desktop), `Sprite.collider`, the four queries (mapping slots to `SpriteList._sprites`), and exports
- [ ] T022 [US4] Add `--collisions` to `tools/web-check/check.mjs`: focus the canvas, hold `ArrowRight` for 2 s, parse the hit count from `document.title`, and pass if it's > 0 (`keys` summary style).
- [ ] T023 [US4] Create `benches/collisions.py` (release build), per research R9:
  - scene `pairs_10k`: 10,000 mixed circle/box/AABB colliders of 4–12 px over 1920×1080, random velocities and angular velocities; each frame `list.update(1/60)`, then time `list.collisions()`
  - scene `sprite_vs_10k`: the same list moving; time `probe.collides_with_list(list)`
  - scene `pairs_10k_static`: no movement; repeated `collisions()`
  - scene `pairs_10k_dense`: the same as `pairs_10k` over 800×600 (informational)
  - 300 timed frames each after 30 warm-up frames; print the median, p95 and pair count; PASS/FAIL against 2 ms / 0.5 ms; exit 2 on FAIL
  - no window needed (queries don't require one)
  - if a target fails, profile it (rebuild vs. query vs. result building) and optimize: reuse buffers, avoid re-sorting, tune the cell size; record before/after

---

## Phase 7: Polish & cross-cutting

- [ ] T024 [P] Update `examples/README.md` (05: controls, color legend, browser URL) and `benches/README.md` (the collision benchmark).
- [ ] T025 Update `docs/spec/ENGINE_SPEC.md`:
  - §11: implementation status and the grid parameters
  - §10: mark Phase 5 implemented with the benchmark numbers
  - §3: the API table (collider classes, queries, writable `title`)
  - decision log: owner-approved writable `Window.title` and benchmark density, plus any deviations found during implementation
- [ ] T026 Write `docs/research/phase5-validation.md` (V1–V6 results, the benchmark table, and the manual V7–V9 checklist pending the owner) and link it from `docs/research/README.md`.
- [ ] T027 Full regression:
  - `cargo fmt --all`
  - clippy for desktop, `--no-default-features`, and wasm32 with `-D warnings`
  - `cargo test --workspace`
  - `maturin develop`, `pytest`
  - `tools/gen_keys.py --check`, and the stub check against the runtime
  - the browser checks for 01, 01c, 02 `--motion`, 04 `--keys`, and 05 `--collisions` (fresh server)
  - `benches/collisions.py` (release)
- [ ] T028 Mark all tasks `[X]` and commit locally. **Don't push** until the owner says so.

---

## Dependencies & Execution Order

- **T001** → **Foundational**: T002 → T003 → T004; T002 → T005.
- **US1 (T006–T009)**: after T005. T006 → T007 → T008; T009 last.
- **US2 (T010–T014)**: after T003–T005. T010 → T011 → T012; T013 after T007 and T011; T014 last.
- **US3 (T015–T017)**: after T011. T015 → T016 → T017.
- **US4 (T018–T023)**: T018 is independent (any time after Foundational). T019–T020 after US2. T021 after US3 (it exports all queries). T022 after T019 + T021. T023 after US3.
- **Polish (T024–T028)**: last. T024 is parallel.

## Parallel Opportunities

- T004 (narrow phase) alongside T005 (store wiring).
- T010 (grid) alongside T006–T009 (Python collider classes).
- T018 (writable title) alongside US2/US3.
- T023 (benchmark) alongside T021–T022 (browser).
- T024 alongside T025/T026.

## Implementation Strategy

1. **MVP = Foundational + US1 + US2**: colliders on sprites and "what is the ship touching", proven exact by property tests.
2. Add **US3** (pair queries) and run the **benchmark** early (T023) to catch performance gaps while the code is fresh.
3. Add **US4**: the title, the example, the browser.
4. Polish: spec, validation report, full regression. Commit locally and wait for the owner before pushing.
