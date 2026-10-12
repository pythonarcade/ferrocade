# Phase 2 Validation Record: Bouncing Ship (Movement)

**Feature**: [specs/004-bouncing-ship](../../specs/004-bouncing-ship/spec.md) | **Guide**: [quickstart.md](../../specs/004-bouncing-ship/quickstart.md)
**Date**: 2026-10-12. **Machine**: Windows 11 Pro, NVIDIA RTX 3500 Ada Laptop GPU, rustc 1.99.0, Python 3.14.4 (venv), Chrome (headless), Pyodide 314.0.7.

| # | Scenario | Result | Evidence |
|---|---|---|---|
| B1 | Rust tests | ✅ | 43 passed. Motion: velocity (200, −100) and 90°/s over 60 × 1/60 s gives (+200, −100) and +90° (within 1e-6). Zero-velocity sprites aren't dirtied. A moving sprite's instance is rewritten in every list. `dt` and velocity validation. Rotation: a 60×20 rectangle at 90° renders 20×60, centered. A 40×40 square at 45° gives a ~57×57 bounding box. A two-tone image confirms **positive angles rotate counter-clockwise**. All earlier pixel tests are unchanged (angle 0). |
| B2 | Python tests | ✅ | 86 passed (twice in a row). Velocity API: defaults, exact round trip, validation, the `velocity` tuple. `SpriteList.update`: 1 s within 1e-6, validation, multi-list, shapes and subclasses. **Timing:** about 180 updates in 3 s with the same distance covered, both at full rate and with drawing slowed to ~20 FPS (a 50 ms sleep), and about 90 updates at `update_rate=1/30` with the same distance. |
| B3 | Bouncing ship (desktop) | ✅ | `--check`: **the ship stayed inside for 60 s (3,601 updates).** It spins counter-clockwise (verified in the browser screenshot). |
| B4 | 10 s update count | ✅ | `--count`: **601 updates in 10.00 s (60.10/s)**. The target is 600 ±2%. |
| B5 | 1,000 bouncing shapes | ✅ | `--many`: **16.66 ms/frame (60 FPS, vsync-limited)**, with Python edge checks on every sprite each update. |
| B6 | Browser parity | ✅ | `?example=02_bouncing_ship` runs **unchanged** on WebGPU and WebGL2. The `--motion` check shows the ship moved 161 px in 1 s through the real browser loop and stayed inside the canvas. The screenshot shows it rotated. |
| B7 | Regression | ✅ | In the browser, the Phase 1 example is still centered (350,262,448,336; 99×75) and Phase 1c still uses 4 draw calls. `cargo fmt`, plus clippy `-D warnings` on the workspace, the no-render core, and wasm32: clean. The stubs match the runtime down to individual `Sprite` and `SpriteList` members. |

## Measurement: engine bulk work vs. per-sprite Python work

1,000 moving sprites, averaged over 600 updates (no window):

| Work per update | Time |
|---|---|
| Engine `SpriteList.update` (moves all 1,000) | **0.087 ms** |
| Python edge checks (`bounce()` per sprite, about 4 property reads and occasional writes) | **1.34 ms** (15× the engine cost) |

Both fit easily in the 16.7 ms frame budget at this scale. But per-sprite Python work costs an order
of magnitude more than the engine's bulk operation. This is direct evidence for the hybrid collision
direction (engine spec §11 Q6): keep detection and responses in Rust, and give Python events or
batches rather than per-sprite polling.

## Findings during implementation

- **Deterministic motion:** two browser runs read identical positions to 13 decimal places, because
  the fixed timestep makes motion a pure function of the update count. That's useful for future
  replay and networking.
- **Test fix:** the timing tests needed a 0.5 s warm-up. The first counted update could arrive as
  part of a post-startup catch-up batch (up to 5), which inflated the count (95 instead of 90 once).
- **Rotated bounds:** edge bouncing uses the unrotated size (per the spec), so a rotated ship's
  corners can poke slightly past an edge. Rotation-aware bounds belong with collisions (Phase 5).
