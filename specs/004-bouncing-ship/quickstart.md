# Quickstart & Validation: Phase 2 — Bouncing Ship (Movement)

**Feature**: [spec.md](spec.md) | **API**: [contracts/python-api.md](contracts/python-api.md)

The build is the same as in earlier phases (see `CLAUDE.md`, "Build & test").

| # | Scenario | How | Expected | Covers |
|---|---|---|---|---|
| B1 | Rust tests | `cargo test --workspace` | They pass, including: `update_list` moves by v·dt (1 s within 1 px), zero-velocity sprites aren't dirtied, multi-list consistency, `dt` validation, angle 90 turns a 60×20 rectangle into 20×60, a direction test shows CCW, and the existing pixel tests are unchanged. | FR-001–010, SC-001, SC-005 |
| B2 | Python tests | `pytest` | They pass, including: velocity properties (defaults, round trip, validation, the tuple), `SpriteList.update`, and display tests showing about 180 `on_update` calls in 3 s at full and slowed (~20 FPS) drawing, with the same distance covered. | FR-001–008, FR-014, SC-002 (short form) |
| B3 | Bouncing ship (desktop) | `python examples/02_bouncing_ship.py`, for 60 s | It moves smoothly and spins slowly counter-clockwise. It bounces off all four edges and never leaves the window. | US2, US4; SC-004 |
| B4 | 10 s update count | `python examples/02_bouncing_ship.py --count` | It prints the update count after 10 s: 600 ±2%. | SC-002 |
| B5 | 1,000 bouncing shapes | `python examples/02_bouncing_ship.py --many` | It holds the refresh rate. Record the frame time. | SC-003 |
| B6 | Browser parity | `tools\build_web.ps1 -Serve`, then `node tools/web-check/check.mjs "http://localhost:8000/web/?example=02_bouncing_ship"` | It runs unchanged, the ship moves between two screenshots, and stays inside the canvas. | FR-013, SC-007 |
| B7 | Regression | The Phase 1 and 1c examples, the web default example, and clippy on all targets | Unchanged and clean. | FR-015 |
