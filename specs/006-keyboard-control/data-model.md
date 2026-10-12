# Data Model: Phase 4 — Keyboard Control

All state lives in the Rust core (`crates/ferrocade/src/input.rs`). It's shared through
`WindowState` on desktop and owned by the web `Engine` in the browser.

## Key
- A physical key position. Rust `enum Key` with an explicit `u16` code per variant.
- **Fields (table row):** `code: u16` (stable Python value), `w3c: &'static str` (UI Events `code`
  name), `py_name: &'static str` (constant name in `fc.key`).
- **Coverage (FR-004):** `A`–`Z`; `KEY_0`–`KEY_9`; `F1`–`F12`; `LEFT`, `RIGHT`, `UP`, `DOWN`;
  `SPACE`, `ENTER`, `ESCAPE`, `TAB`, `BACKSPACE`, `INSERT`, `DELETE`, `HOME`, `END`, `PAGE_UP`,
  `PAGE_DOWN`, `CAPS_LOCK`; `LSHIFT`, `RSHIFT`, `LCTRL`, `RCTRL`, `LALT`, `RALT`, `LSYSTEM`,
  `RSYSTEM`; `NUM_0`–`NUM_9`, `NUM_ADD`, `NUM_SUBTRACT`, `NUM_MULTIPLY`, `NUM_DIVIDE`,
  `NUM_DECIMAL`, `NUM_ENTER`, `NUM_LOCK`; `MINUS`, `EQUAL`, `BRACKET_LEFT`, `BRACKET_RIGHT`,
  `BACKSLASH`, `SEMICOLON`, `APOSTROPHE`, `GRAVE`, `COMMA`, `PERIOD`, `SLASH`.
- **Validation:** codes and W3C names are unique (tested). Keys outside the table aren't reported.

## Modifiers
- A `u8` bit set: `MOD_SHIFT = 1`, `MOD_CTRL = 2`, `MOD_ALT = 4`, `MOD_SYSTEM = 8`.
- Derived from the held set after applying an event (either side counts).

## RawKeyEvent (platform → core)
- `key: Key`, `pressed: bool`, `repeat: bool`. Queued in arrival order.

## KeyEvent (core → game)
- `key: Key`, `pressed: bool`, `modifiers: u8`. Produced only by `Keyboard::drain()`.

## Keyboard
- **Fields:** `queue: Vec<RawKeyEvent>`, `held: set of Key` (a 128-bit bitset; codes are below 128).
- **Operations:**
  - `push(raw)`: queue it (platform or test injection).
  - `release_all()`: queue a release for each held key (focus loss).
  - `drain() -> Vec<KeyEvent>`: apply the queue in order and return accepted events.
  - `is_held(key) -> bool`.
- **State transitions per key:**

  | Current | Raw event | Result |
  |---|---|---|
  | up | press (not repeat) | → held; emit press |
  | up | press (repeat) | ignored |
  | held | press (any) | ignored (OS repeat or duplicate) |
  | held | release | → up; emit release |
  | up | release | ignored |
