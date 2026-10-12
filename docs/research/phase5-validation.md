# Phase 5 Validation: Collision Detection

**Feature**: [specs/007-collision-detection](../../specs/007-collision-detection/spec.md) | **Design**: [docs/design/collision.md](../design/collision.md) | **Date**: 2026-10-12
**Machine**: Windows 11, NVIDIA RTX 3500 Ada (default high-performance GPU), release build for benchmarks, Chrome for browser checks

## Summary

Detection-only collisions work on desktop and in the browser (WebGPU and WebGL2):
- `CircleCollider`, `BoxCollider`, and `AABBCollider`, attached explicitly with `sprite.collider`
- coupled scale, rotating offsets (not for `AABBCollider`)
- four batched queries: `collides_with`, `collides_with_list`, `collisions_with`, `collisions`
- **Both speed targets are met.**

All automated checks pass. The manual checks (V7–V9) are pending the owner.

## Benchmark (SC-002, SC-003): `python benches/collisions.py`

10,000 mixed circle / box / axis-aligned box colliders of 4–12 px over 1920×1080, all moving and
spinning (`SpriteList.update` every frame, so the cache rebuilds every frame). Times are the full
Python call, including building the result. 300 frames after 30 warm-up frames.

| Scene | Median | p95 | Results | Target | |
|---|---|---|---|---|---|
| `pairs_10k`: `list.collisions()` | **1.53 ms** | 1.94 ms | 4,506 pairs | < 2 ms | ✅ PASS |
| `sprite_vs_10k`: `probe.collides_with_list(list)` | **0.23 ms** | 0.25 ms | 9 sprites | < 0.5 ms | ✅ PASS |
| `pairs_10k_static`: same, nothing moving | 0.83 ms | 1.05 ms | 6,046 pairs | info | |
| `pairs_10k_dense`: same as `pairs_10k` packed into 800×600 | 3.39 ms | 4.67 ms | 12,943 pairs | info | |

**Optimizations made to meet the 0.5 ms target** (the first version passed `pairs_10k` at
1.67 ms but missed `sprite_vs_10k` at **0.94 ms**):
1. **No trigonometry where it isn't needed.** Circles without an offset and axis-aligned boxes skip
   the rotation entirely, and rotating boxes use one `sin_cos` call. Together with (2):
   0.94 → **0.57 ms**.
2. **The grid rebuilds in place**, reusing its buffers instead of allocating every frame.
3. **The grid is built lazily.** A single probe right after the list changed scans the cached bounds
   linearly (10,000 bounds checks take microseconds). A second probe of the unchanged list builds
   the grid, so many probes per frame stay fast. Pair queries always use the grid.
   0.57 → **0.23 ms**.

The query itself is about 1 µs once the cache is fresh. Almost all the remaining cost is computing
10,000 world shapes after the sprites move.

**SC-003 (unchanged lists):** the cache isn't rebuilt when nothing changed. This is verified by a
Rust test with a rebuild counter: repeated queries don't rebuild, and a move or collider change
rebuilds exactly once. A repeated single-sprite query on a static 10,000-sprite list takes about
1 µs. A repeated *pair* query still enumerates pairs and builds the Python tuples each time
(0.83 ms for 6,046 pairs, `pairs_10k_static`). Caching pair results per version is a possible
later optimization; it isn't needed for the targets.

## Automated results (quickstart V1–V6)

| # | Check | Result |
|---|---|---|
| V1 | `cargo test --workspace` | ✅ 72 passed. New collision tests: validation; world transforms (scale, counter-clockwise offsets, AABB offsets don't rotate, quarter-turn boxes become axis-aligned); exact boundary cases for every shape pair (touching vs. a 1e-9 gap); 600 random shape pairs checked against an independent **point-sampling reference**; the grid against all-pairs on 800 mixed-size bounds (including large objects); `within_list` / `sprite_vs_list` / `list_vs_list` against brute force on scenes of 10 / 100 / 1,500 / 600 sprites with random rotations, scales 0–3, offsets, and 10× outliers; cache rebuild rules; lazy grid; attach/replace/detach/destroy rules |
| V2 | `pytest` | ✅ 127 passed (20 new in `tests/test_collision.py`, 2 new title tests in `tests/test_window.py`). This covers spec US1–US3 scenarios, errors, no leak between sprite and collider, snapshots, and a 300-sprite random scene where `collisions()` and `collisions_with()` equal the brute force over `collides_with` |
| V3 | `benches/collisions.py` | ✅ All targets met (above) |
| V4 | Example test (injected keys) | ✅ The ship records hits; rocks removed = hits; the title shows the count |
| V5 | `check.mjs …example=05_collisions --collisions` | ✅ WebGPU: holding ArrowRight for 2 s gave **11 hits** (title "Collisions — hits: 11"). ✅ WebGL2 passes too |
| V6 | Regression: fmt, clippy (desktop / no-render / wasm32), `gen_keys --check`, stubs vs. runtime, browser checks for 01, 01c, 02 `--motion`, 04 `--keys` | ✅ |

## Manual checks (owner)

| # | Steps | Expected | Status |
|---|---|---|---|
| V7 | `python examples/05_collisions.py`; steer into asteroids of each color | Each touched asteroid disappears; the title count goes up by one | pending |
| V8 | Fly close past asteroids without touching | Nothing disappears | pending |
| V9 | The same in the browser (`?example=05_collisions`) | The same behavior | pending |

## Notes

- The ship's `BoxCollider` is 80% of its image size, because the image has transparent corners.
  Automatic hit boxes from texture alpha are in the backlog.
- Fast objects can pass through each other between updates (no swept tests, documented). At
  60 updates per second this needs a speed above the objects' size × 60 px/s.
- The writable `Window.title` (owner-approved) composes with `show_fps`.
