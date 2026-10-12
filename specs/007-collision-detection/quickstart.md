# Quickstart: Phase 5 — Collision Detection validation

Prerequisites: the Phase 4 setup (`.venv`, maturin, pytest, the web toolchain, `tools/web-check`).

## Automated

| # | Command | Expected |
|---|---|---|
| V1 | `cargo test --workspace` | Collision tests pass: exact boundary cases for every shape pair, rotation/scale/offset rules, grid = all-pairs on thousands of random mixed shapes, narrow phase = sampling reference (SC-001) |
| V2 | `maturin develop`, then `pytest tests/test_collision.py` | Spec acceptance scenarios (US1–US3), errors for invalid sizes and double attachment, snapshot semantics, and `collisions()` = brute force over `collides_with` |
| V3 | `maturin develop --release`, then `python benches/collisions.py` | `pairs_10k` median < **2 ms**, `sprite_vs_10k` median < **0.5 ms** (SC-002); `pairs_10k_static` repeated queries ≈ 0 (SC-003); the dense scene is reported for information |
| V4 | `pytest tests/test_collision.py -k example` | The example run with injected keys records hits (US4) |
| V5 | Web build + `node check.mjs "...?example=05_collisions" --collisions` (fresh server) | The hit count rises in the browser (SC-004) |
| V6 | Full regression: lint, `pytest`, the earlier browser checks, `gen_keys --check`, stubs | Pass |

## Manual (owner)

| # | Steps | Expected |
|---|---|---|
| V7 | `python examples/05_collisions.py`; steer into asteroids of each shape | Each touched asteroid disappears; the title count goes up by one |
| V8 | Fly close past asteroids without touching | Nothing disappears |
| V9 | The same in the browser (`?example=05_collisions`) | The same behavior |
