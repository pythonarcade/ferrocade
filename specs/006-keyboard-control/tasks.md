---

description: "Task list for Phase 4 — Keyboard Control"
---

# Tasks: Phase 4 — Keyboard Control

**Input**: Design documents from `specs/006-keyboard-control/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included. The spec's success criteria (SC-001, SC-004, SC-005) require automated checks: Rust unit tests for the core `Keyboard`, pytest through private key injection, and the headless browser check.

**Organization**:
- **US1 (P1):** key press/release callbacks
- **US2 (P1):** steer the ship (the example)
- **US3 (P2):** `is_key_pressed` polling
- **US4 (P2):** the browser build

The core key table and `Keyboard` state machine are foundational: every story uses them.

**Owner-confirmed choices (2026-10-12)**:
- key names: `KEY_0`–`KEY_9`, `NUM_*`, `PAGE_UP`/`PAGE_DOWN`, `BRACKET_LEFT`/`BRACKET_RIGHT`, `LSYSTEM`/`RSYSTEM`, `MOD_SYSTEM`
- modifiers reflect the held keys **after** the event
- no Escape-to-quit (no `Window.close()` in this phase)

## Format: `[ID] [P?] [Story] Description`

**Global rules**:
- No new crates. Only new `web-sys` features.
- Never hold the `WindowState` lock while calling Python.
- Run Rust commands with `export PATH="$HOME/.cargo/bin:$PATH"`. Run `maturin develop` after every Rust change before running pytest.

---

## Phase 1: Setup

- [ ] T001 Add the web-sys features `KeyboardEvent`, `FocusEvent`, `Event`, `EventTarget`, `HtmlElement`, `AddEventListenerOptions`, `VisibilityState` to `crates/ferrocade-web/Cargo.toml`, and confirm `cargo check -p ferrocade-web --target wasm32-unknown-unknown` passes.

---

## Phase 2: Foundational (blocks all stories)

- [ ] T002 Create `crates/ferrocade/src/input.rs` and add `pub mod input;` to `crates/ferrocade/src/lib.rs`. It must compile **without** the `render` feature (no winit types here). Contents:
  - `#[repr(u16)] pub enum Key` with explicit codes, all below 128, plus a `const KEYS: &[KeyInfo]` table where `KeyInfo { key, code: u16, w3c: &'static str, py_name: &'static str }`. Coverage, exactly as in data-model.md: `A`–`Z` (w3c `KeyA`…); `KEY_0`–`KEY_9` (`Digit0`…); `F1`–`F12`; `LEFT`/`RIGHT`/`UP`/`DOWN` (`ArrowLeft`…); `SPACE`, `ENTER`, `ESCAPE`, `TAB`, `BACKSPACE`, `INSERT`, `DELETE`, `HOME`, `END`, `PAGE_UP`, `PAGE_DOWN`, `CAPS_LOCK`; `LSHIFT`/`RSHIFT` (`ShiftLeft`/`ShiftRight`), `LCTRL`/`RCTRL` (`ControlLeft`…), `LALT`/`RALT` (`AltLeft`…), `LSYSTEM`/`RSYSTEM` (`MetaLeft`/`MetaRight`); `NUM_0`–`NUM_9` (`Numpad0`…), `NUM_ADD`, `NUM_SUBTRACT`, `NUM_MULTIPLY`, `NUM_DIVIDE`, `NUM_DECIMAL`, `NUM_ENTER`, `NUM_LOCK`; `MINUS`, `EQUAL`, `BRACKET_LEFT`, `BRACKET_RIGHT`, `BACKSLASH`, `SEMICOLON`, `APOSTROPHE` (`Quote`), `GRAVE` (`Backquote`), `COMMA`, `PERIOD`, `SLASH`.
  - `Key::from_code(u16) -> Option<Key>`, `Key::from_w3c(&str) -> Option<Key>`, `key.code()`, `key.py_name()`.
  - Modifier constants `MOD_SHIFT = 1`, `MOD_CTRL = 2`, `MOD_ALT = 4`, `MOD_SYSTEM = 8`.
  - `pub struct RawKeyEvent { key, pressed, repeat }` and `pub struct KeyEvent { key, pressed, modifiers: u8 }`.
  - `pub struct Keyboard { queue: Vec<RawKeyEvent>, held: u128 }` with:
    - `push(raw)`
    - `release_all()`: queues a release for each held key
    - `drain() -> Vec<KeyEvent>`: applies the data-model.md state table: up + press(not repeat) → held, emit; up + press(repeat) → ignore; held + press → ignore; held + release → up, emit; up + release → ignore. Modifiers are computed from `held` **after** applying the event.
    - `is_held(key)`
- [ ] T003 Add unit tests in `crates/ferrocade/src/input.rs` (`#[cfg(test)]`):
  - every code and W3C name is unique, and codes are < 128
  - `from_code` and `from_w3c` round-trip for every key
  - an auto-repeat press is ignored, and so is a duplicate press while held
  - a release without a press is ignored
  - order is preserved (press A, press B, release A → three events in order)
  - modifiers: pressing `LSHIFT` reports `MOD_SHIFT`, its release doesn't; `A` with `RCTRL` held reports `MOD_CTRL`
  - `release_all` emits releases for exactly the held keys
  - 100 pseudo-random press/release/repeat events, then `release_all` + `drain` → nothing held
- [ ] T004 In `crates/ferrocade/src/window.rs`:
  - add a `Keyboard` to `WindowState`'s inner state, with methods `push_key(RawKeyEvent)`, `release_all_keys()`, `drain_keys() -> Vec<KeyEvent>`, `is_key_held(Key) -> bool`
  - reset the keyboard in `reset_stats()`'s run-start path, or a new `reset_input()` called at run start
  - extend `GameCallbacks` with `fn key_press(&mut self, key: Key, modifiers: u8) -> Result<(), Self::Error> { Ok(()) }` and `key_release` (default methods)
- [ ] T005 In `crates/ferrocade/src/platform/desktop.rs`:
  - add `fn map_key(winit::keyboard::KeyCode) -> Option<Key>` covering every `Key` (winit names: `SuperLeft`/`SuperRight` for Meta)
  - handle `WindowEvent::KeyboardInput { event, .. }`: when `event.physical_key` is `PhysicalKey::Code(c)` and maps to a key, `state.push_key(RawKeyEvent { key, pressed: event.state.is_pressed(), repeat: event.repeat })`
  - `WindowEvent::Focused(false)` → `state.release_all_keys()`
  - in `redraw`, before the update loop: `for e in self.state.drain_keys()` call `callbacks.key_press`/`key_release`, and on `Err` call `self.fail(...)` like the other callbacks
  - add a Rust test (`#[cfg(test)]` in desktop.rs) that every `Key` has a `KeyCode` mapping (iterate the winit codes the table names; check that the mapped set equals all keys)

**Checkpoint**: `cargo test -p ferrocade` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

---

## Phase 3: User Story 1 — React to key presses and releases (P1) 🎯 MVP

**Goal**: Python `Window` subclasses receive `on_key_press` / `on_key_release` with `fc.key` constants and modifiers.

**Independent test**: inject keys from `on_update` and check the recorded callback calls (V2).

- [ ] T006 [US1] In `crates/ferrocade-py/src/lib.rs`, add `_key_table() -> list[tuple[str, int]]` (Python name and code, in table order) plus the MOD values (for example, return them as extra `("MOD_SHIFT", 1)` entries, or as a second function `_mod_table()`).
- [ ] T007 [US1] Create `tools/gen_keys.py`: it imports `ferrocade._ferrocade`, writes `python/ferrocade/key.py` (module docstring, one `NAME = code` line per key in table order, the `MOD_*` constants, a `_NAMES` dict, and `def name(key: int) -> str` raising `ValueError` for unknown codes), and with `--check` exits 1 if the file differs. Run it to generate `python/ferrocade/key.py`.
- [ ] T008 [US1] Export `key` from `python/ferrocade/__init__.py` (`from . import key`, and add `"key"` to `__all__`).
- [ ] T009 [US1] In `crates/ferrocade-py/src/py_window.rs`:
  - add default `on_key_press(self, key, modifiers)` and `on_key_release(self, key, modifiers)` methods that do nothing
  - add private `_inject_key(key: int, pressed: bool, repeat: bool = False)` (`ValueError` for unknown codes) and `_inject_focus_lost()`, which call `state.push_key` / `release_all_keys`
  - implement `GameCallbacks::key_press`/`key_release` in `PyCallbacks` as `call_method1("on_key_press", (key.code(), modifiers))`
- [ ] T010 [US1] Add stubs to `python/ferrocade/_ferrocade.pyi`: `on_key_press`, `on_key_release` (with docstrings: once per press, repeats ignored, delivered before `on_update`, modifiers after the event, release on focus loss), `_inject_key`, `_inject_focus_lost`, `_key_table`. Add `python/ferrocade/key.py`-compatible typing (the module itself is plain Python).
- [ ] T011 [US1] Create `tests/test_keyboard.py` (skip real-window tests when `FERROCADE_HEADLESS=1`, the same way as `tests/test_window.py`; stop windows by raising a `StopTest` exception):
  - press+release Space → exactly `[("press", SPACE, 0), ("release", SPACE, 0)]`
  - injected repeats while held → still one press
  - `LSHIFT` held + `A` → `modifiers & MOD_SHIFT` and not `MOD_CTRL`
  - focus lost with `LEFT` and `W` held → a release for each
  - an exception in `on_key_press` propagates out of `run()`
  - a window without overrides accepts keys silently
  - `fc.key.name(fc.key.LEFT) == "LEFT"`, and `name(9999)` raises `ValueError`
  - `tools/gen_keys.py --check` passes (run via `subprocess`)

**Checkpoint**: `maturin develop`, then `pytest tests/test_keyboard.py` passes.

---

## Phase 4: User Story 3 — Ask whether a key is held (P2)

*(Done before US2 because the example uses polling.)*

**Goal**: `Window.is_key_pressed(key)`.

**Independent test**: polling is `True` between the delivered press and release (V2).

- [ ] T012 [US3] Add `is_key_pressed(self, key: int) -> bool` to `crates/ferrocade-py/src/py_window.rs` (`TypeError` for non-int via PyO3 extraction; unknown codes → `False`), plus a stub with its docstring in `python/ferrocade/_ferrocade.pyi`.
- [ ] T013 [US3] Add tests to `tests/test_keyboard.py`:
  - `True` in `on_key_press` and in the next `on_update` after a press, `False` after the release
  - press+release injected in the same frame → both callbacks delivered, polling `False` afterwards
  - 100 random injected taps across keys, then releases → no key is held (SC-004)
  - `is_key_pressed("a")` raises `TypeError`

---

## Phase 5: User Story 2 — Steer the ship (P1)

**Goal**: the phase's acceptance example.

**Independent test**: manual V6–V10. US4 automates it in the browser.

- [ ] T014 [US2] Create `examples/04_keyboard_ship.py`:
  - an 800×600 window, `filter=fc.NEAREST`, the ship centered in a `SpriteList`, `SPEED = 300` px/s
  - `on_update`: `dx = right - left`, `dy = up - down` from `is_key_pressed` for the arrow keys **or** WASD; normalize if both are nonzero (the same speed diagonally); set `ship.velocity = (dx * SPEED, dy * SPEED)`; call `self.ships.update(delta_time)`; then clamp `center_x`/`center_y` so the ship's half width/height stays inside the window
  - `on_key_press`: `SPACE` re-centers the ship
  - a module docstring explaining the controls
  - it must run unchanged in the browser (only API the web shim supports)
- [ ] T015 [US2] Add a desktop test to `tests/test_keyboard.py` that imports the example's window class (follow the pattern used for earlier examples in tests, if any; otherwise use `runpy`/`importlib` with `__name__` not `"__main__"`). Inject `RIGHT` for ~0.5 s and check `center_x` increased by about 300 × t (within 10%) and `center_y` didn't change. Inject `UP`+`RIGHT` and check the per-second distance is within 1% of 300 (SC-003). Check that holding `LEFT` long enough keeps the ship's left edge ≥ 0.

---

## Phase 6: User Story 4 — Same behavior in the browser (P2)

**Goal**: the example runs unchanged on the web with the same keys.

**Independent test**: `check.mjs --keys` (V4).

- [ ] T016 [US4] In `crates/ferrocade-web/src/lib.rs`:
  - hold an `Rc<RefCell<Keyboard>>` in `Engine`
  - in `create`, set the canvas `tabindex` to `"0"` and attach `keydown`/`keyup` listeners (map `event.code()` with `Key::from_w3c`; push `RawKeyEvent { repeat: event.repeat() }`; call `prevent_default()` for recognized keys unless `ctrl_key() || alt_key() || meta_key()` or the key is `TAB` or `F1`–`F12`)
  - attach `blur` on the canvas and the window, plus `visibilitychange` (when hidden) → `release_all()`
  - keep the `Closure`s alive (store them in `Engine`, or call `.forget()` once, since the engine lives for the page)
  - add `take_key_events() -> Vec<i32>` returning `drain()` flattened as `[code, pressed(0/1), modifiers, ...]`
  - add `focus()` that focuses the canvas
- [ ] T017 [US4] In `web/python/ferrocade/__init__.py`:
  - `from . import key` (and add it to `__all__`)
  - `Window` gets default `on_key_press`/`on_key_release`, a `self._held = set()`, and `is_key_pressed(key)` (`TypeError` for non-int; reads `_held`)
  - in `_frame`, before the update loop: `events = _engine.take_key_events()`; for each triple, update `_held`, then call `on_key_press` or `on_key_release`
- [ ] T018 [US4] In `web/loader.js`: also copy `../python/ferrocade/key.py` to `/home/pyodide/ferrocade/key.py`; after the engine is created, call `engine.focus()` and add a canvas `click` → `engine.focus()` listener. If `web/index.html` needs it, add `tabindex="0"` to the canvas there too.
- [ ] T019 [US4] Add `--keys` to `tools/web-check/check.mjs`:
  - make the page taller than the viewport so scrolling would show (`document.body.style.minHeight = "3000px"`)
  - `page.focus("#game")`, read sprite 0's position, `page.keyboard.down("ArrowRight")`, wait 1 s, read again, `keyboard.up`
  - pass if x increased by > 100, |Δy| < 1, and `window.scrollY === 0`; otherwise set outcome `"keys-failed"`
  - include the result as `keys` in the summary

**Checkpoint**: build the web package (Bash: `cargo build -p ferrocade-web --target wasm32-unknown-unknown --release` + `wasm-bindgen ...`), serve on a **fresh** port so no stale cache is served, and run `node check.mjs "http://localhost:<port>/web/?diagnostics=1&example=04_keyboard_ship" --keys` → exit 0.

---

## Phase 7: Polish & cross-cutting

- [ ] T020 [P] Update `examples/README.md` with `04_keyboard_ship.py` (controls and browser URL).
- [ ] T021 Update `docs/spec/ENGINE_SPEC.md`:
  - replace the §5 "TBD: keyboard/mouse callbacks" with the keyboard API (physical keys, callbacks, polling, auto-repeat ignored, focus-loss release, modifiers after the event, browser focus and `preventDefault` rule)
  - mark Phase 4 implemented in §10 with a pointer to `docs/research/phase4-validation.md`
  - add decision log rows for the owner-confirmed choices (key names; modifiers after the event; no Escape-to-quit / no `Window.close()` in Phase 4)
  - add `Window.close()` to the backlog
- [ ] T022 Write `docs/research/phase4-validation.md` (V1–V5 results, plus the manual V6–V11 checklist pending the owner) and link it from `docs/research/README.md`.
- [ ] T023 Full regression:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade --no-default-features --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade-web --target wasm32-unknown-unknown -- -D warnings`
  - `cargo test --workspace`
  - `maturin develop`, `pytest`
  - `python tools/gen_keys.py --check`, and the stub check against the runtime
  - the browser checks for `01_centered_ship`, `01c_shapes`, `02_bouncing_ship --motion`, and `04_keyboard_ship --keys` (fresh server)
  - a quick `benches/stress.py --quick --engine ferrocade --scene engine_moved` to confirm no frame-time regression (SC-006)
- [ ] T024 Mark all tasks `[X]` and commit locally. **Don't push** until the owner says so.

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T005)**: T002 → T003; T002 → T004 → T005.
- **US1 (T006–T011)**: after Foundational. T006 → T007 → T008; T009 → T010; T011 last.
- **US3 (T012–T013)**: after T009.
- **US2 (T014–T015)**: after US1 and US3 (the example uses callbacks and polling).
- **US4 (T016–T019)**: after Foundational and T007 (`key.py`). T016 → T017 → T018 → T019. It's independent of US2's desktop test, but its check uses the example (T014).
- **Polish (T020–T024)**: last. T020 is parallel.

## Parallel Opportunities

- T003 (tests) alongside T004 once T002 exists.
- T016 (web crate) alongside T009–T013 (Python bindings): different crates.
- T020 alongside T021/T022.

## Implementation Strategy

1. **MVP = Foundational + US1**: key callbacks on desktop, proven by injected tests.
2. Add **US3** (polling), then **US2** (the example) for the phase's "done when".
3. Add **US4** (browser parity) and its automated check.
4. Polish: spec, validation report, full regression. Commit locally and wait for the owner before pushing.
