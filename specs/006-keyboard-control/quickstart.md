# Quickstart: Phase 4 — Keyboard Control validation

Prerequisites: the Phase 3 setup (`.venv` with maturin and pytest), plus the web toolchain and
`tools/web-check` from Phase 1b.

## Automated

| # | Command | Expected |
|---|---|---|
| V1 | `cargo test --workspace` | `input.rs` tests pass: repeat/duplicate filtering, modifiers, `release_all`, ordering, unique codes, winit mapping covers every key |
| V2 | `maturin develop`, then `pytest tests/test_keyboard.py` | Injected presses and releases give exactly one callback each (including injected repeats), `is_key_pressed` follows them, focus loss releases every held key, exceptions propagate, and 100 random taps leave no key held (SC-001, SC-004) |
| V3 | `python tools/gen_keys.py --check` | `python/ferrocade/key.py` matches the engine's key table |
| V4 | `tools\build_web.ps1 -Serve`, then `node check.mjs "http://localhost:8000/web/?diagnostics=1&example=04_keyboard_ship" --keys` | Holding ArrowRight moves the ship right (x up, y unchanged), and the page doesn't scroll (SC-005) |
| V5 | The Phase 1–3 browser checks and `pytest` | Still pass |

## Manual (owner)

| # | Steps | Expected |
|---|---|---|
| V6 | `python examples/04_keyboard_ship.py`. Hold each arrow key, then each of WASD | The ship moves in that direction at a steady speed, and stops immediately on release (SC-002) |
| V7 | Hold Up+Right, then Left+Right | Diagonal movement at the same speed; opposite keys cancel (SC-003) |
| V8 | Push into each edge | The ship stays fully inside the window |
| V9 | Hold Right, then Alt+Tab to another window | The ship stops (focus loss releases the key) |
| V10 | Press Space | The ship returns to the center |
| V11 | Open the browser example and repeat V6–V10 | The same behavior, and the arrow keys don't scroll the page |
