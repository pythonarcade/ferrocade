# Quickstart: Performance Showcase validation

Prerequisites: the Phase 5 setup.

## Automated

| # | Command | Expected |
|---|---|---|
| V1 | `cargo test --workspace` | Bounce tests: head-on swap and angled wall reflection match the formulas within 1e-9 (SC-003); overlaps separate; wall corners and inside-the-wall cases push out correctly; non-circle sprites skipped; 10 s random simulation keeps energy within 1% and every ball inside (SC-002); timing accumulators reset per frame |
| V2 | `maturin develop`, then `pytest tests/test_showcase.py` | API contract; `frame_stats.update_ms` / `collision_ms` present, 0 when unused, > 0 when used; engine and Python modes match after 60 updates within 1e-6 px (SC-004); example keys add/remove/toggle/pause (injected) and a stats line at least every 1.5 s (SC-005) |
| V3 | `maturin develop --release`, then `python benches/showcase.py` | Engine mode sustains **10,000 balls** with work < 14 ms (SC-001). The maximum counts for engine and Python modes are reported (SC-006) |
| V4 | `python examples/06_showcase.py --balls 10000 --seconds 6` (release) | Stats lines show fps ≈ 60 at 10,000 balls (SC-001, windowed) |
| V5 | Web build + `node check.mjs "...?example=06_showcase" --showcase` (fresh server) | Stats lines appear and balls move (US5) |
| V6 | Full regression (lint, all tests, earlier browser checks, collision and stress benches) | Pass |

## Manual (owner)

| # | Steps | Expected |
|---|---|---|
| V7 | `python examples/06_showcase.py` | Balls bounce smoothly off walls and each other; the console prints stats every second |
| V8 | Press `=` several times, then `-` | The ball count rises and falls by 1,000; the FPS changes visibly in the stats |
| V9 | Press `T` | The mode switches to Python; collision handling gets slower in the stats; `T` again switches back |
| V10 | Press `Space` twice | Pause, then resume |
| V11 | The browser version (`?example=06_showcase`), with the developer console open | The same behavior; note the maximum count |
