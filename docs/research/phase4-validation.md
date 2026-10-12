# Phase 4 Validation: Keyboard Control

**Feature**: [specs/006-keyboard-control](../../specs/006-keyboard-control/spec.md) | **Date**: 2026-10-12
**Machine**: Windows 11, NVIDIA RTX 3500 Ada (high-performance GPU, now the default), Chrome for the browser checks

## Summary

Keyboard input works end to end on desktop and in the browser (WebGPU and WebGL2):
- `on_key_press` / `on_key_release` callbacks
- physical-key `fc.key` constants
- `is_key_pressed` polling
- focus-loss release
- `examples/04_keyboard_ship.py`

All automated checks pass. The manual checks (V6–V11) are pending the owner.

## Automated results (quickstart V1–V5)

| # | Check | Result |
|---|---|---|
| V1 | `cargo test --workspace` | ✅ 54 passed. New: 7 `input.rs` tests (unique table, round-trips, repeat/duplicate filtering, order, modifiers after the event, `release_all`, 100 random events) and 1 desktop test (every key has a winit code whose name matches the W3C name) |
| V2 | `pytest` | ✅ 105 passed (16 new in `tests/test_keyboard.py`) |
| V2a | One callback per press/release, repeats ignored (SC-001) | ✅ |
| V2b | 100 random multi-key taps → no key held, presses = releases (SC-004) | ✅ |
| V2c | Example: right arrow moves exactly 300 px/s with no vertical drift; the ship stops on release | ✅ |
| V2d | Example: diagonal speed within 1% of straight speed (SC-003) | ✅ (exact) |
| V2e | Example: opposite keys cancel, edges clamp, Space re-centers | ✅ |
| V3 | `python tools/gen_keys.py --check` | ✅ up to date. Also, the stubs match the runtime |
| V4 | `check.mjs …example=04_keyboard_ship --keys` | ✅ WebGPU: holding ArrowRight for 1 s moved the ship **300 px** right, 0 px vertically; `scrollY` stayed 0 on a 3000 px page; the ship stopped on release. ✅ WebGL2 (`&backend=webgl2`) passed too |
| V5 | Earlier browser checks (`01_centered_ship`, `01c_shapes`, `02_bouncing_ship --motion`) and lint (fmt, clippy desktop / no-render / wasm) | ✅ |

**Frame time (SC-006):** `benches/stress.py --engine ferrocade --scene engine_moved` (release build)
still reaches the 512,000 ramp cap at 60 FPS (update 8.0 ms + render 4.2 ms = 12.3 ms of work),
matching Phase 3's discrete-GPU result. A `--quick` run once failed a 1-second step at 58.9 FPS
with only 1.7 ms of work. That was a single frame-pacing stall, and the full run didn't repeat it.

## Manual checks (owner)

| # | Steps | Expected | Status |
|---|---|---|---|
| V6 | `python examples/04_keyboard_ship.py`; hold each arrow key and each of WASD | Steady movement; stops immediately on release | pending |
| V7 | Up+Right, then Left+Right | The same speed diagonally; opposite keys cancel | pending |
| V8 | Push into each edge | The ship stays fully inside | pending |
| V9 | Hold Right, then Alt+Tab away | The ship stops | pending |
| V10 | Space | The ship returns to the center | pending |
| V11 | The same in the browser (`?example=04_keyboard_ship`) | The same behavior; the page doesn't scroll | pending |

## Notes

- **Polling vs. callbacks:** a press and release that both happen between two frames reach the
  callbacks but not `is_key_pressed` (tested, documented in the API contract).
- **Modifiers** come from the engine's own held-key set, so a modifier pressed before the window got
  focus isn't reported until it's pressed again.
- **Test injection:** the private `Window._inject_key` / `_inject_focus_lost` feed the same queue as
  winit, so the tests exercise the real per-frame delivery path. Real OS key events are covered by
  the manual checks, and in the browser by puppeteer's DOM key events.
