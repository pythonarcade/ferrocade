---

description: "Task list for Phase 2 — Bouncing Ship (Movement)"
---

# Tasks: Phase 2 — Bouncing Ship (Movement)

**Input**: Design documents from `specs/004-bouncing-ship/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included. The spec requires automated verification: movement distance (SC-001), update
rate under slowed drawing (SC-002), and rotation by pixel check (SC-005).

**Organization**:
- **US1:** velocities and `SpriteList.update`
- **US2:** the bouncing-ship example
- **US3:** frame-rate independence
- **US4:** rotation

## Format: `[ID] [P?] [Story] Description`

**Global rules**:
- The instance stays 48 bytes.
- Angle 0 must stay pixel-identical (existing tests pass unchanged).
- No per-sprite Python work for movement.
- Desktop and web get the same API.
- Commit locally after each group; **push only when the owner says so**.

---

## Phase 1: Setup

No setup is needed: there are no new dependencies or tools, and the toolchain and test harness
come from earlier phases.

---

## Phase 2: Foundational (Blocking Prerequisites)

- [ ] T001 Add `velocity_x: f64`, `velocity_y: f64`, and `angular_velocity: f64` (all default `0.0`) to `SpriteState` in `crates/ferrocade/src/store.rs`, initialized in `create_sprite`.
  - Add the setters `set_velocity_x`, `set_velocity_y`, `set_velocity(id, vx, vy)`, and `set_angular_velocity`. Each one **"must be finite, otherwise `ValueError`"** (`Error::InvalidArgument`, via `check_finite`). They do **not** mark anything dirty, since velocities aren't drawn.
- [ ] T002 Regression gate: `cargo test --workspace` and `pytest` still pass (nothing is visible yet).

**Checkpoint**: the motion state exists in the core.

---

## Phase 3: User Story 1 - Make sprites move with velocities (Priority: P1) 🎯 MVP

**Goal**: velocity properties on every sprite, plus a one-call `SpriteList.update(delta_time)` on desktop and web.

**Independent Test**: a sprite with `velocity_x = 200`, stepped through a list for 1 s of updates, has moved 200 px (±1).

### Tests for User Story 1

- [ ] T003 [P] [US1] Add to `crates/ferrocade/tests/store.rs`:
  - `update_list` with velocity (200, −100) and dt = 1/60 × 60 steps moves the sprite by (200, −100) ±1e-6
  - `angular_velocity` 90 for 1 s adds 90° to the angle
  - zero-velocity sprites are **not** dirtied (`take_dirty` before, then `dirty() == None` after the update)
  - a moving sprite in two lists has its instance rewritten in both
  - `dt` of NaN or −1 gives `InvalidArgument`
  - velocity setters reject NaN and ±inf
- [ ] T004 [P] [US1] Create `tests/test_motion.py` (no display needed), covering:
  - the velocity defaults are `0.0`
  - `velocity_x`/`velocity_y`/`angular_velocity`/`velocity` round-trip exactly, and `velocity = (1.5, -2.25)` reads back as a tuple
  - NaN and ±inf raise `ValueError`
  - `SpriteList.update(1/60)` called 60 times moves the sprite by velocity × 1 s (±1e-6) and the angle by `angular_velocity`
  - `update` with `delta_time` −1 or NaN raises `ValueError`
  - a sprite in two lists moves twice when both are updated
  - shape sprites and subclasses move too

### Implementation for User Story 1

- [ ] T005 [US1] Add `pub fn update_list(&mut self, list: ListId, dt: f64) -> Result<()>` to `crates/ferrocade/src/store.rs`, per data-model.md:
  - validate **"`dt` finite and ≥ 0, otherwise `ValueError`"**
  - for each sprite in the list with any non-zero velocity: `center += velocity·dt` and `angle += angular_velocity·dt`
  - then rewrite its instance in every membership and mark those slots dirty (reuse the `update()` helper logic)
  - zero-velocity sprites stay untouched
- [ ] T006 [US1] Add the getters and setters for `velocity_x`, `velocity_y`, `angular_velocity`, and the `velocity` tuple (one call to `set_velocity`) to `crates/ferrocade-py/src/py_sprite.rs`.
- [ ] T007 [US1] Add `fn update(&self, delta_time: f64) -> PyResult<()>` to `crates/ferrocade-py/src/py_sprite_list.rs`, calling `store::global().update_list(self.id, delta_time)`. It's allowed outside frames.
- [ ] T008 [US1] Add stubs to `python/ferrocade/_ferrocade.pyi`: `Sprite.velocity_x/velocity_y/angular_velocity: float`, `Sprite.velocity: tuple[float, float]`, and `SpriteList.update(self, delta_time: float) -> None`, with docstrings (units, and that counter-clockwise is positive).
- [ ] T009 [US1] Add to the web engine `crates/ferrocade-web/src/lib.rs`: `get_velocity(sprite) -> Vec<f64>` (`[vx, vy, va]`), `set_velocity(sprite, vx, vy)`, `set_angular_velocity(sprite, va)`, and `update_list(list, dt)`, with errors mapped through `js_err`.
- [ ] T010 [US1] Add the matching properties (`velocity_x`, `velocity_y`, `angular_velocity`, `velocity`) and `SpriteList.update(delta_time)` to the web shim `web/python/ferrocade/__init__.py`, with the same validation as desktop (`ValueError`).
- [ ] T011 [US1] Run T003 and T004 (`cargo test`, `maturin develop`, `pytest`) and `cargo check -p ferrocade-web --target wasm32-unknown-unknown`.

**Checkpoint**: US1 is complete. Sprites move via one list call.

---

## Phase 4: User Story 2 - Bounce the ship off the window edges (Priority: P2)

**Goal**: the milestone example, on desktop and web.

**Independent Test**: `python examples/02_bouncing_ship.py` shows the ship bouncing off all four edges, never leaving the window.

- [ ] T012 [US2] Create `examples/02_bouncing_ship.py` from the contract sketch.
  - **Default mode:** the ship centered, velocity (220, 170), `angular_velocity = 45`. `on_update` calls `self.sprites.update(delta_time)`, then for each axis, if the sprite's unrotated half-size bounds are outside the window, it flips that velocity component and clamps the position back inside.
  - **`--many`:** 1,000 random `RectangleSprite`/`CircleSprite` shapes (sizes 4–24, random CSS colors, random velocities ±50–250 px/s), bounced by the same per-sprite Python check in a loop, with the frame time printed every 2 s (startup excluded, like `01c_shapes.py`).
  - **`--count`:** count `on_update` calls and print the count and rate after 10 s of wall-clock time, then exit by raising `SystemExit` from `on_update`.
- [ ] T013 [US2] Run the default mode for about 60 s, using a check variant that tracks bounds. Add an internal `--check` flag that asserts the ship stays inside the window every update and prints `OK` after 60 s. Then run `--many` and record the frame time (B3, B5).
- [ ] T014 [US2] Browser motion check: add a `--motion` flag to `tools/web-check/check.mjs`. It takes a second screenshot 1 s after the first, computes the bounding box of non-background pixels in each, and asserts that the box moved and stays inside the canvas. Run it on `http://localhost:8000/web/?example=02_bouncing_ship` with `tools\build_web.ps1` (B6).

**Checkpoint**: US2 is complete. The milestone demo works on desktop and web.

---

## Phase 5: User Story 3 - Movement speed doesn't depend on frame rate (Priority: P3)

**Goal**: prove the fixed timestep from Python, under normal and slowed drawing.

**Independent Test**: about 180 updates in 3 s, at full and at ~20 FPS drawing, with the same distance covered.

- [ ] T015 [P] [US3] Add to `tests/test_motion.py` (`@pytest.mark.display`):
  - a window with the default update rate, run for 3.0 s of wall-clock time (stop via a `StopTest` raised from `on_update` once `time.perf_counter()` passes the deadline), counting `on_update` calls: expect 180 ±3%, and a sprite at 100 px/s, updated each tick, moved 300 px ±3%
  - the same with `on_draw` sleeping 0.05 s (about 20 FPS): the same expectations
  - `update_rate=1/30`: about 90 calls in 3 s, with the same distance
- [ ] T016 [US3] Run T015 and `python examples/02_bouncing_ship.py --count`: expect 600 ±2% after 10 s (B4). Record the results.

**Checkpoint**: US3 is complete. Frame-rate independence is verified.

---

## Phase 6: User Story 4 - Rotating sprites (Priority: P4)

**Goal**: sprites are drawn rotated around their center, counter-clockwise for positive angles.

**Independent Test**: a 60×20 rectangle at 90° renders 20×60. A marker on the +x side of an image ends up on the +y side.

- [ ] T017 [P] [US4] Add to `crates/ferrocade/tests/render_offscreen.rs` (one store per renderer):
  - a `create_rectangle(60, 20, RED, 400, 300)` at `angle = 90` has a bounding box of 20×60 (±1), centered at (400, 300) ±1
  - **direction:** a two-tone 20×10 test image (left half blue, right half red, written with the `image` crate) at `angle = 90` has its red pixels **above** the center (y > 300) and its blue pixels below. A positive angle is counter-clockwise.
  - `angle = 45` on a 40×40 square gives a bounding box of about 57×57 (±2)
  - the angle-0 renders are unchanged (the existing tests cover this)
- [ ] T018 [US4] Update the vertex shader in `crates/ferrocade/src/render/sprite.wgsl`: `let a = radians(in.angle); let offset = (corner - vec2(0.5)) * in.size; let rotated = vec2(offset.x*cos(a) - offset.y*sin(a), offset.x*sin(a) + offset.y*cos(a)); world = in.center + rotated`. Remove the "not applied yet" comments in `crates/ferrocade/src/instance.rs` and the shader.
- [ ] T019 [US4] Run T017 plus the full `cargo test --workspace`. Confirm in the browser (the default `?diagnostics=1` check is still centered, and the spinning ship rotates in `?example=02_bouncing_ship`).

**Checkpoint**: all four stories are complete.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T020 Full regression and lint:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade --no-default-features --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade-web --target wasm32-unknown-unknown -- -D warnings`
  - `cargo test --workspace`, `maturin develop`, `pytest`
  - the Phase 1 and 1c examples (unchanged), and the web checks for `01_centered_ship` and `01c_shapes`
- [ ] T021 [P] Check `python/ferrocade/_ferrocade.pyi` against the runtime (no missing or extra names) and against contracts/python-api.md.
- [ ] T022 Record B1–B7 (with frame times, update counts, and the cost of the Python edge checks over 1,000 sprites in `--many`) in `docs/research/phase2-validation.md`, linked from `docs/research/README.md`. Mark Phase 2 implemented in `docs/spec/ENGINE_SPEC.md` §10, and add decision log entries for any deviations.

---

## Dependencies & Execution Order

- **T001 → T002** come before every story.
- **US1 (T003–T011):** T005 → T006/T007 → T008. The web tasks (T009–T010) follow T005. T011 runs last.
- **US2 (T012–T014):** needs US1. Rotation looks better after US4, but isn't required.
- **US3 (T015–T016):** needs US1 (for the distance checks). It's independent of US2 and US4.
- **US4 (T017–T019):** needs only the foundation (the angle already exists). It can run in parallel with US1–US3, since it touches the shader and render tests only.
- **Polish (T020–T022):** last.

### Parallel Opportunities

- T003 and T004 (US1 tests); T015 (US3) and T017 (US4) alongside US1 implementation work; T021.

## Parallel Example: after the foundation

```text
Task: "T003 store motion tests in crates/ferrocade/tests/store.rs"
Task: "T004 Python motion tests in tests/test_motion.py"
Task: "T017 rotation pixel tests in crates/ferrocade/tests/render_offscreen.rs"
```

## Implementation Strategy

1. Foundation (T001–T002).
2. **US1 (MVP):** velocities plus `update`, on desktop and web.
3. **US4:** rotation (small and independent; it makes the demo look right).
4. **US2:** the example and the browser motion check.
5. **US3:** the timing proofs.
6. Polish, the validation record, and spec updates. Commit locally. **Push only on the owner's word.**
