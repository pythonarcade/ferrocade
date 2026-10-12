# Implementation Plan: Phase 4 — Keyboard Control

**Branch**: `006-keyboard-control` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-keyboard-control/spec.md`

## Summary

Add keyboard input that flows from the platform (winit on desktop, DOM `KeyboardEvent`s in the
browser) through the Rust core to Python. The new pieces:
- `on_key_press(key, modifiers)` / `on_key_release(key, modifiers)` callbacks
- `fc.key` constants identified by **physical** position
- `Window.is_key_pressed(key)`
- an example that steers the ship with the arrow keys and WASD

A core `Keyboard` (in `crates/ferrocade/src/input.rs`) owns the key table, the event queue, the
held set, repeat filtering, modifiers, and focus-loss release. Both platforms feed it raw events.
The loop drains it once per frame, before the fixed-timestep updates. Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust stable (2024 edition), Python ≥ 3.13 (abi3), WGSL unchanged

**Primary Dependencies**: winit 0.30 (`KeyEvent::physical_key`, `repeat`, `WindowEvent::Focused`), PyO3 0.29, wasm-bindgen 0.2.129, web-sys (new features: `KeyboardEvent`, `FocusEvent`, `EventTarget`, `HtmlElement`, `AddEventListenerOptions`, `VisibilityState`), Pyodide 314.0.7. No new crates.

**Storage**: N/A

**Testing**: `cargo test` (the `input.rs` unit tests), pytest with private key injection (`tests/test_keyboard.py`), and the headless browser check `check.mjs --keys`

**Target Platform**: Windows (prototype), macOS (portable), browser (approach B, WebGPU/WebGL2)

**Project Type**: library (game engine) with Python bindings and a web build

**Performance Goals**: no measurable per-frame cost with no keys pressed (SC-006). One boundary crossing per frame on the web (`take_key_events`).

**Constraints**: constitution III (input handling in the platform layer and core; no threads), constitution II (no per-sprite work), and callbacks never run inside `on_update` or `on_draw`

**Scale/Scope**: ~70 keys; about 6 source files touched plus 1 new core module, 1 generated Python module, 1 example, 1 test file

## Constitution Check

*GATE: must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. Python API, Rust core | Key state, filtering and the key table live in the core (`input.rs`, no PyO3). The bindings only forward callbacks and expose `is_key_pressed`. | ✅ |
| II. Minimize conversion | Keys are plain ints. One web boundary crossing per frame. No per-sprite work. Polling on the web uses a Python-side set fed by the per-frame batch. | ✅ |
| III. Portable | winit mapping only in `platform/desktop.rs`. Browser listeners in the web crate. W3C code names are shared. No threads, no `std::time`. | ✅ |
| IV. 2D, simple first | Only keyboard. No mouse, gamepad, text input, remapping or `Window.close()`. | ✅ |
| V. Spec-driven | Implements engine spec §5/§10 Phase 4. The spec's defaults (auto-repeat ignored, focus-loss release, bit-flag modifiers, key names) are Proposed until the owner confirms them; recorded in §5 after confirmation. | ✅ (pending owner review of defaults) |
| VI. Phase delivery | `examples/04_keyboard_ship.py`, plus regression of earlier examples and web checks. | ✅ |

**Post-design re-check**: still passes. No complexity exceptions.

## Project Structure

### Documentation (this feature)

```text
specs/006-keyboard-control/
├── plan.md              # this file
├── research.md          # R1–R7 decisions
├── data-model.md        # Key, Modifiers, Keyboard state machine
├── quickstart.md        # V1–V11
├── contracts/
│   └── python-api.md    # fc.key, callbacks, is_key_pressed
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/ferrocade/src/
├── input.rs             # NEW: Key table (code, W3C name, Python name), Keyboard, KeyEvent, modifiers
├── window.rs            # WindowState gains a Keyboard; GameCallbacks gains key_press/key_release (default Ok)
├── lib.rs               # pub mod input
└── platform/desktop.rs  # KeyboardInput → push, Focused(false) → release_all, drain before updates; winit KeyCode → Key map

crates/ferrocade-py/src/
├── py_window.rs         # on_key_press/on_key_release defaults, is_key_pressed, _inject_key, _inject_focus_lost; PyCallbacks dispatch
└── lib.rs               # _key_table() for gen_keys.py and the sync test

crates/ferrocade-web/
├── Cargo.toml           # web-sys features
└── src/lib.rs           # canvas keydown/keyup/blur + visibilitychange listeners, preventDefault rule, take_key_events()

python/ferrocade/
├── key.py               # NEW, generated: int constants, MOD_* flags, name()
├── __init__.py          # export key
└── _ferrocade.pyi       # callbacks, is_key_pressed, _key_table

web/
├── index.html           # canvas tabindex="0"
├── loader.js            # copy key.py; focus canvas on start and click
└── python/ferrocade/__init__.py   # import key; drain events per frame; callbacks; is_key_pressed

tools/gen_keys.py        # NEW: write/--check key.py from _key_table()
tools/web-check/check.mjs # --keys mode
examples/04_keyboard_ship.py       # NEW
tests/test_keyboard.py             # NEW
docs/spec/ENGINE_SPEC.md           # §5 input section, §10 Phase 4 status, decision log (owner-confirmed items only)
```

**Structure Decision**: follows the existing layout. Input is a new core module beside `window.rs`,
and platform specifics stay in `platform/desktop.rs` and the web crate.

## Complexity Tracking

No violations to justify.
