# Performance Showcase Validation

**Feature**: [specs/008-performance-showcase](../../specs/008-performance-showcase/spec.md) | **Date**: 2026-10-12
**Machine**: Windows 11, NVIDIA RTX 3500 Ada, release build, Chrome for the browser

## Summary

Thousands of balls bounce off the walls and each other: `examples/06_showcase.py`.
- **The engine bounces** by default (`balls.bounce_each_other()` + `balls.bounce_off(walls)`), as
  equal-mass, perfectly elastic circles.
- **`T` switches to Python bouncing** (the same formulas, computed from `collisions()` results).
- **Engine timings:** `FrameStats` gains engine-measured `update_ms` and `collision_ms`, printed
  to the console every second.
- **Target met:** 10,000 balls at 60 FPS with 3.0 ms of engine work per frame.

All automated checks pass. The manual checks (V7–V11) are pending the owner.

## Results

**Windowed, 10,000 balls, engine bouncing (SC-001):**
`balls=10000 fps=60.1 render=0.44ms update=0.16ms collision=2.40ms` → work **3.0 ms** (budget 14 ms). ✅ PASS

**Largest ball count with the move + bounce step under 14 ms** (headless, `benches/showcase.py`, SC-006):

| Balls | Engine bouncing | Python bouncing |
|---|---|---|
| 2,000 | 0.21 ms | 0.37 ms |
| 6,000 | 0.68 ms | 1.88 ms |
| 10,000 | 1.37 ms | 5.80 ms |
| 14,000 | 2.62 ms | 12.10 ms |
| 16,000 | 3.16 ms | 17.32 ms (over) |
| 32,000 | 13.20 ms | — |
| **Max under 14 ms** | **32,000** | **14,000** |

Both modes detect contacts in the engine. Python bouncing costs grow with the number of contacts,
each paying several Python property reads and writes. At 10,000 balls, Python bouncing is about
4× slower than engine bouncing, and it runs out of budget at about 15,000 balls (vs. 32,000 for
the engine). The headless numbers exclude rendering, which adds about 0.4 ms at 10,000 balls.

**Browser (WebGPU, informational):** stepping the add key in headless Chrome, the showcase held
60 FPS from 5,000 up to **at least 20,000 balls** (the end of the ramp; engine collision time
about 8 ms at 20,000). WebGL2 ran 5,000 balls at 59.9 FPS.

## Automated checks (quickstart V1–V6)

| # | Check | Result |
|---|---|---|
| V1 | `cargo test --workspace` | ✅ 82 passed. New: 9 bounce tests (exact head-on swap; an angled pair conserves momentum and energy within 1e-9 and keeps tangential components; separating pairs keep velocities but lose overlap; 30° wall reflection keeps speed; a ball inside a wall goes out the near side; corner contacts; non-circle sprites skipped; timing accumulators reset) and a **2,000-ball, 600-update simulation**: energy within 1%, every ball inside, identical on repeat (SC-002, SC-003). Also a new regression test (see Fix below) |
| V2 | `pytest` | ✅ 135 passed (8 new in `tests/test_showcase.py`): bounce API, balls stay inside over 300 steps, **engine and Python modes match after 60 updates within 1e-6 px** (SC-004), `update_ms`/`collision_ms` are 0 when unused and > 0 when used, the example prints at least 2 stats lines in 2.5 s (SC-005), and injected keys add/remove/pause/toggle |
| V3 | `benches/showcase.py` | ✅ SC-001 PASS (above) |
| V4 | Windowed 10,000 run | ✅ fps 60.1 |
| V5 | `check.mjs …example=06_showcase --showcase` | ✅ WebGPU: 2 stats lines, fps 59.9, the newest ball moved 44 px in 0.5 s. ✅ WebGL2 too |
| V6 | Regression: fmt, clippy (desktop / no-render / wasm32), `gen_keys --check`, stubs, browser checks 01/01c/02/04/05, `benches/collisions.py` (Phase 5 targets still met: 1.51 ms / 0.23 ms) | ✅ |

## Fix found along the way (affects Phase 5)

A list whose collider count dropped below 32 after its grid was built kept using the old grid,
which referred to slots that no longer existed. The showcase's remove key hit it as a panic. The
grid is now used only while the list has at least 32 colliders, and a Rust regression test covers
shrinking lists. Engine time spent before `run()` is also no longer counted in the first frame.

## Manual checks (owner)

| # | Steps | Expected | Status |
|---|---|---|---|
| V7 | `python examples/06_showcase.py` | Smooth bouncing; stats every second | pending |
| V8 | `=` several times, then `-` | The count changes by 1,000; FPS and times change in the stats | pending |
| V9 | `T` | "mode=python" with a `python_bounce` time; `T` again returns to the engine | pending |
| V10 | `Space` twice | Pause, then resume | pending |
| V11 | Browser `?example=06_showcase` with the developer console open | The same behavior | pending |

## Notes

- The scene is 1280×720 with radius 2–4 px balls (corrected at planning; see research R1).
- Contacts are resolved in one pass per update. Dense clusters can keep small overlaps that
  resolve over the next few updates. That's fine for an elastic-gas demo, and a general physics
  solver stays in the backlog.
