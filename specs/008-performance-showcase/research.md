# Research: Performance Showcase

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12

## R1. Scene size and ball size (corrects a spec assumption)

- **Finding**: the spec's default of radius 3–6 px in 800×600 can't hold 10,000 balls. The total
  ball area (about 10,000 × π × 4.5² ≈ 636,000 px²) exceeds the 480,000 px² window, so balls would
  pile up instead of bouncing.
- **Decision**: a **1280×720** scene and **radius 2–4 px**. That's about 290,000 px² of balls in
  921,600 px², roughly 30% coverage: crowded but gas-like, with visible bouncing. The showcase
  **starts with 5,000 balls**, and the 10,000 target (SC-001) is reached with the add key or a
  command-line count. The spec's Assumptions are updated to match; this is flagged for the owner.
- **Speeds** 40–200 px/s: at 60 Hz, at most 3.3 px per update, below the smallest diameter, so
  nothing tunnels through a ball or a 10 px wall.

## R2. API: explicit bounce calls on `SpriteList`

- **Decision** (the owner chose explicit calls):
  - `balls.bounce_each_other() -> int`: resolves touching pairs within the list; returns the
    contact count.
  - `balls.bounce_off(walls: SpriteList) -> int`: resolves ball–wall contacts; returns the count.
  - The game calls them each update, after `balls.update(dt)`: first `bounce_each_other()`, then
    `bounce_off(walls)`, so the last correction keeps balls inside the walls.
- **Alternatives**: one combined call (less flexible); a declared response property (backlog,
  declared pairs).

## R3. Physics (equal-mass, perfectly elastic, one pass)

- **Ball–ball** (circle colliders only): with world centers c₁ and c₂, the normal is
  n = (c₂ − c₁)/|c₂ − c₁| (or (1, 0) if the centers coincide). Let vₙ = (v₁ − v₂)·n.
  - If vₙ > 0 (approaching): v₁ −= vₙ·n and v₂ += vₙ·n. That's the equal-mass elastic exchange of
    the normal components. Momentum and energy are conserved exactly in exact arithmetic.
  - Overlap o = r₁ + r₂ − |c₂ − c₁|. If o > 0, move each sprite o/2 apart along n (its sprite
    center moves; the collider follows).
- **Ball–wall** (any wall collider): find the closest point p on the wall shape to the ball center
  c, with n = (c − p)/|c − p|.
  - If c is inside a box wall, n is the face normal of the least-penetrated axis, and the
    penetration counts as distance + radius.
  - If v·n < 0: v −= 2(v·n)·n (reflection; speed unchanged).
  - Push the ball out by r − |c − p| along n.
- **Order**: contacts come from the Phase 5 queries (`within_list` / `list_vs_list`), already
  sorted. They're resolved sequentially in that order, each using the current (already updated)
  velocities and positions. That's deterministic, and it's reproducible in Python with the same
  float operations (SC-004).
- **Non-circle sprites** in the ball list are skipped, as the spec assumes.
- **Alternatives**: iterative solvers or impulse accumulation (physics-engine territory; not
  needed for an elastic-gas demo; backlog).

## R4. Engine-measured update and collision time

- **Decision**: the store accumulates per-frame seconds in `Cell<f64>` fields: `update_seconds`
  (inside `update_list`) and `collision_seconds` (inside every query and bounce call), using
  `web_time::Instant`. Each frame, the game loop takes and resets them and passes them into
  `record_frame`.
  - `FrameSnapshot` (and Python `FrameStats`) gains `update_ms` and `collision_ms`, averaged over
    the same ~1 s window as `render_cpu_ms`.
  - Several updates in one frame add up.
  - The browser engine does the same in `end_frame`.
- **Rationale**: every call that should be counted already runs inside the store, so there's no
  new global state and no threads (constitution III). The `Instant` cost (~25 ns per call) is
  negligible next to the work being timed.
- **Alternatives**: timing in Python (the owner chose engine timings); per-call profiling hooks
  (more machinery).

## R5. Python mode (the comparison)

- The example's `python_bounce(balls, walls)` uses `balls.collisions()` and
  `balls.collisions_with(walls)` (the engine detects), then applies **the same formulas in the
  same order** in Python, reading and writing `position` / `velocity` / `collider` through the
  normal API.
- The Python bounce time is measured with `perf_counter` (FR-006). The engine's collision time
  still includes the two detection queries.
- **Parity test** (SC-004): the same seeded scene runs 60 updates in each mode, and the positions
  are compared (tolerance 1e-6 px; identical float operations should give the same results).

## R6. Stats line and controls

- Once per second (by accumulated `delta_time`), print:
  `balls=5000 fps=60.0 render=1.10ms update=0.09ms collision=1.40ms mode=engine` (plus
  `python_bounce=…ms` in Python mode). The values come from `frame_stats`.
- Keys: `EQUAL`/`NUM_ADD` add 1,000 (placed at random free spots inside the walls), `MINUS`/
  `NUM_SUBTRACT` remove the last 1,000, `T` toggles the mode, `SPACE` pauses (it skips
  `update` and bouncing).
- Command line (desktop): `--balls N` sets the start count, and `--seconds S` runs S seconds,
  then exits (for measurements).

## R7. Measuring the targets (SC-001, SC-006)

- `benches/showcase.py` runs the showcase scene headlessly at each count. No window is needed: it
  calls the same update, bounce, and query steps and times them. It also runs a windowed pass via
  `examples/06_showcase.py --balls N --seconds 5` to read the real FPS and render time from the
  printed stats.
- It reports the maximum ball count with work < 14 ms for engine and Python modes. SC-001 passes
  if engine mode sustains 10,000 at 60 FPS. The browser maximum is found by hand with the add key
  and reported (informational).

## R8. Browser

- New web exports: `bounce_each_other(list) -> u32`, `bounce_off(list, walls) -> u32`, and
  `frame_stats` extended with update and collision ms. The shim gets the same methods and fields.
- Python `print` in Pyodide goes to the browser console.
- `check.mjs --showcase`: load `06_showcase` and wait for two console lines containing `balls=`,
  then assert that FPS > 0 and that the balls moved (sprite positions read twice).
