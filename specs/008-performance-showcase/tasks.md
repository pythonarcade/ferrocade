---

description: "Task list for the Performance Showcase"
---

# Tasks: Performance Showcase

**Input**: Design documents from `specs/008-performance-showcase/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included. SC-002 to SC-005 require automated checks; SC-001 and SC-006 are measured by the bench.

**Organization**:
- **US1 (P1):** engine bouncing
- **US2 (P1):** engine timings + stats line
- **US3 (P2):** Python mode
- **US4 (P2):** controls
- **US5 (P3):** browser

Engine timings are foundational: the example's stats line (US1 demo) and US2 both need them.

## Format: `[ID] [P?] [Story] Description`

**Global rules**:
- No new crates.
- Equal-mass, perfectly elastic circles only (owner decision).
- Contacts are resolved in one pass, in the order the Phase 5 queries return them.
- Use `web_time::Instant`.
- Run Rust with `export PATH="$HOME/.cargo/bin:$PATH"`. Run `maturin develop` before pytest.
- Write multi-line edit scripts to files (inline heredocs have failed in this environment).

---

## Phase 1: Setup

- [X] T001 Create `crates/ferrocade/src/collision/bounce.rs` (empty module with a doc comment) and add `pub mod bounce;` to `crates/ferrocade/src/collision/mod.rs`.

---

## Phase 2: Foundational — engine timings (blocks US1's stats line and US2)

- [X] T002 In `crates/ferrocade/src/store.rs`:
  - add `update_seconds: Cell<f64>` and `collision_seconds: Cell<f64>` to `SpriteStore`
  - time `update_list` (add to `update_seconds`), plus `collides`, `sprite_vs_list`, `list_vs_list`, `within_list` and, later, the bounce functions (add to `collision_seconds`), with `web_time::Instant`
  - add `pub fn take_frame_timings(&self) -> (f64, f64)`, returning (update_ms, collision_ms) and resetting both
  - add a Rust test that after an `update_list` and a `within_list` both are > 0, and that they're 0 after the take
- [X] T003 In `crates/ferrocade/src/window.rs`:
  - add `update_ms` and `collision_ms` to `FrameSnapshot`
  - `RollingStats` averages them like `render_cpu_ms`
  - `record_frame(interval, render_cpu_ms, draw_calls, instances, update_ms, collision_ms)`
  - update the existing tests
  
  In `crates/ferrocade/src/platform/desktop.rs`, call `store::global().take_frame_timings()` each frame and pass them in. In `crates/ferrocade-web/src/lib.rs`, do the same in `end_frame`, and extend `frame_stats()` to `[fps, frame_ms, render_cpu_ms, draw_calls, instances, update_ms, collision_ms]`.
- [X] T004 Add `update_ms`/`collision_ms` to `PyFrameStats` in `crates/ferrocade-py/src/py_window.rs` (getters + `__repr__`), the stubs in `python/ferrocade/_ferrocade.pyi`, and the web shim's `FrameStats` and `frame_stats` in `web/python/ferrocade/__init__.py`.

**Checkpoint**: `cargo test -p ferrocade`, clippy (desktop, no-render, wasm32), `pytest tests/test_window.py` pass.

---

## Phase 3: User Story 1 — Engine bouncing (P1) 🎯 MVP

**Goal**: `bounce_each_other()` and `bounce_off(walls)`, plus the scene.

**Independent test**: exact formula tests and a conservation simulation (V1); the example runs.

- [X] T005 [US1] In `crates/ferrocade/src/collision/bounce.rs`, add `impl SpriteStore`:
  - `pub fn bounce_within(&mut self, list: ListId) -> Result<usize>`:
    - get contacts from `self.within_list(list)` (sorted)
    - for each pair where **both** sprites have `Shape::Circle` colliders, recompute both world circles from current state (earlier contacts may have moved them)
    - let n = (c₂ − c₁)/|d|, or (1, 0) if |d| = 0, and vₙ = (v₁ − v₂)·n
    - if vₙ > 0: v₁ −= vₙ·n, v₂ += vₙ·n (through `set_velocity`)
    - with overlap o = r₁ + r₂ − |d|: if o > 0, move each sprite center by ∓ o/2·n (`set_position`)
    - count resolved contacts
  - `pub fn bounce_off(&mut self, list: ListId, walls: ListId) -> Result<usize>`: for each `(ball, wall)` from `list_vs_list(list, walls)` where the ball has a circle collider:
    - p = the closest point on the wall's world shape to the current ball center (circle: along the center line; box: clamp in the box frame)
    - if the center is inside a box: n = the outward normal of the least-penetrated face, and penetration = (half − |local|) on that axis + r
    - otherwise n = (c − p)/|c − p| and penetration = r − |c − p|
    - if v·n < 0: v −= 2(v·n)·n; if penetration > 0, move the ball by penetration·n
  - add both to `collision_seconds`
- [X] T006 [US1] Rust tests in `crates/ferrocade/src/collision/bounce.rs`:
  - head-on equal speeds swap (exact, within 1e-9)
  - an angled pair: normal components exchanged, tangential kept; momentum and energy conserved exactly (1e-9)
  - separating pairs aren't changed in velocity, but overlap is still removed
  - overlapping balls end touching (|d| = r₁ + r₂, within 1e-9)
  - wall reflection at 30°: speed kept, normal flipped
  - a ball center inside a wall is pushed out to the correct side
  - wall corner contact
  - non-circle sprites are skipped
  - **simulation**: 2,000 balls (radius 2–4, speeds 40–200) in a walled 1280×720 box, 600 updates of `update_list(1/60)` + `bounce_within` + `bounce_off` → kinetic energy within 1% of the start (SC-002), every ball center inside the walls, deterministic (two runs identical)
- [X] T007 [US1] Add `bounce_each_other()` and `bounce_off(walls)` to `crates/ferrocade-py/src/py_sprite_list.rs` (return `int`), with stubs + docstrings in `python/ferrocade/_ferrocade.pyi` (per contracts/python-api.md).
- [X] T008 [US1] Create `examples/06_showcase.py`:
  - 1280×720, `filter=fc.LINEAR`
  - four 10 px `RectangleSprite` walls (`fc.color.DIM_GRAY`) with `BoxCollider`s
  - `--balls N` (default 5,000), placed at random free spots (avoiding the walls)
  - `CircleSprite` radius 2–4, random CSS colors, `CircleCollider`, velocity 40–200 px/s in random directions, `random.Random(7)` for reproducibility
  - `on_update`: unless paused, `balls.update(dt)`, then a bounce step (engine mode: `balls.bounce_each_other(); balls.bounce_off(walls)`)
  - `on_draw` draws walls and balls
  - a module docstring with keys and options
  - it must run unchanged in the browser (guard `sys.argv` parsing)
- [X] T009 [US1] Create `tests/test_showcase.py` with US1 API tests (no display):
  - two balls head-on swap velocities after `bounce_each_other`
  - a ball moving into a wall reflects
  - return counts
  - box sprites in the ball list aren't moved
  - balls stay inside walls over 300 headless steps

---

## Phase 4: User Story 2 — See where the time goes (P1)

**Goal**: engine timings visible; a once-per-second stats line.

**Independent test**: `frame_stats` fields behave; a line prints ≥ once per 1.5 s (V2).

- [X] T010 [US2] In `examples/06_showcase.py`, accumulate `delta_time`. About every 1.0 s, print `balls=N fps=F render=Rms update=Ums collision=Cms mode=engine` (plus ` python_bounce=Pms` in Python mode), from `self.frame_stats` (2 decimals). Add `--seconds S` to exit after S seconds by raising `SystemExit` from `on_update` (desktop only).
- [X] T011 [US2] Add tests to `tests/test_showcase.py` (display):
  - a plain window's `frame_stats.update_ms == 0` and `collision_ms == 0`
  - a window that updates and bounces 2,000 balls has both > 0 after ~1.2 s
  - the example run with `--seconds 2.5` (via `subprocess` with `--balls 500`) prints at least 2 stats lines containing `balls=500`

---

## Phase 5: User Story 3 — Python mode (P2)

**Goal**: the same physics in Python, toggled with `T`.

**Independent test**: parity with engine mode after 60 updates (SC-004).

- [X] T012 [US3] In `examples/06_showcase.py`, add a module-level `python_bounce(balls, walls)`: the same formulas and order as T005, using `balls.collisions()` / `balls.collisions_with(walls)`, recomputing centers from `position` (balls have no offsets), reading radius from `collider.radius × scale`, and writing `velocity`/`position`. The window tracks `self.mode`; `T` toggles; Python-mode time is measured with `perf_counter` and reported in the stats line.
- [X] T013 [US3] Add a parity test to `tests/test_showcase.py`: build the same seeded scene twice (1,000 balls), run 60 steps of `update(1/60)` + engine bounce vs. `update(1/60)` + `python_bounce`, and compare all positions within 1e-6 px (import the example module like the earlier example tests).

---

## Phase 6: User Story 4 — Controls (P2)

**Goal**: add/remove 1,000, pause.

**Independent test**: injected keys change the count and pause motion (V2).

- [X] T014 [US4] In `examples/06_showcase.py` `on_key_press`:
  - `EQUAL`/`NUM_ADD`: add 1,000 balls at free spots
  - `MINUS`/`NUM_SUBTRACT`: remove the last 1,000 (not below 0; remove from the list, and drop references)
  - `T`: toggle mode
  - `SPACE`: pause/resume
- [X] T015 [US4] Add a display test to `tests/test_showcase.py`, driving the example with `_inject_key`: start 500 → `EQUAL` → 1,500 → `MINUS` ×2 → 0 (no error; it keeps running) → `EQUAL` → 1,000; `SPACE` freezes positions across 10 updates; `T` switches `mode` to `"python"`, then back.

---

## Phase 7: User Story 5 — Browser (P3)

**Goal**: the showcase runs in the browser with console stats.

**Independent test**: `check.mjs --showcase` (V5).

- [X] T016 [US5] Add `bounce_each_other(list) -> u32` and `bounce_off(list, walls) -> u32` to `crates/ferrocade-web/src/lib.rs`, and the matching `SpriteList` methods in `web/python/ferrocade/__init__.py`.
- [X] T017 [US5] Add `--showcase` to `tools/web-check/check.mjs`: wait up to 10 s for 2 console lines containing `balls=`; parse `fps=` from the last one (> 0); read the position of the last-created sprite (a ball: `ferrocadeEngine.get_sprite(index)`, with the index taken as the count of sprites created − 1, or expose `window.ferrocadeLastSprite` from the shim) twice, 0.5 s apart; it must change. Set outcome `showcase-failed` otherwise.

---

## Phase 8: Polish & cross-cutting

- [X] T018 Create `benches/showcase.py`:
  - headless (no window): for counts 2,000 → 20,000 (step 2,000), build the seeded scene and time 120 steps of update + bounce in engine mode (engine `update_ms` + `collision_ms` from `take`-style deltas: time with `perf_counter` around the calls)
  - the same in Python mode, up to the count where steps exceed 14 ms
  - also run `examples/06_showcase.py --balls 10000 --seconds 6` and parse its last stats line for fps and render ms
  - print a table plus PASS/FAIL for SC-001 (engine 10,000: update + collision + render < 14 ms and fps ≥ 59); exit 2 on FAIL
  
  If SC-001 fails, profile and optimize: avoid re-refreshing the ball cache between the two bounce calls; batch the `set_position`/`set_velocity` writes.
- [X] T019 [P] Update `examples/README.md` (06), `benches/README.md` (the showcase bench), and `docs/research/README.md`.
- [X] T020 Update `docs/spec/ENGINE_SPEC.md`:
  - mark the showcase milestone implemented, with the measured numbers
  - §3: `bounce_each_other`, `bounce_off`, `FrameStats.update_ms`/`collision_ms`
  - decision log: the scene size correction (1280×720, radius 2–4) and any deviations
- [X] T021 Write `docs/research/showcase-validation.md`: V1–V6 results, the max-count table (engine, Python, browser), and manual V7–V11 pending the owner.
- [X] T022 Full regression:
  - fmt, clippy for desktop / no-render / wasm32
  - `cargo test --workspace`
  - `maturin develop`, `pytest`
  - `gen_keys --check`, and the stub check against the runtime
  - the browser checks for 01, 01c, 02 `--motion`, 04 `--keys`, 05 `--collisions`, 06 `--showcase` (fresh server)
  - `benches/collisions.py` and `benches/showcase.py` (release)
- [X] T023 Mark all tasks `[X]` and commit locally. **Don't push** until the owner says so.

---

## Dependencies & Execution Order

- **T001** → **Foundational T002 → T003 → T004**.
- **US1 (T005–T009)**: T005 needs T001 (it can start alongside T002–T004). T005 → T006; T007 after T005; T008 after T007; T009 after T007.
- **US2 (T010–T011)**: after T004 and T008.
- **US3 (T012–T013)**: after T008.
- **US4 (T014–T015)**: after T012 (the `T` key toggles the mode).
- **US5 (T016–T017)**: T016 after T005 + T004; T017 after T008 + T016.
- **Polish (T018–T023)**: after the stories. T019 is parallel.

## Parallel Opportunities

- T005–T006 (bounce core) alongside T002–T004 (timings).
- T016 (web exports) alongside T010–T015 (example features).
- T019 alongside T020/T021.

## Implementation Strategy

1. **MVP = timings + US1**: engine bouncing proven by exact tests and the conservation simulation; the example runs.
2. Add **US2** (stats line); run the **bench** early to check SC-001.
3. Add **US3** (Python comparison) and **US4** (controls).
4. Add **US5** (browser), then polish, full regression, and a local commit. Wait for the owner before pushing.
