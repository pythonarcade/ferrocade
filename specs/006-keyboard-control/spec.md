# Feature Specification: Phase 4 — Keyboard Control

**Feature Branch**: `006-keyboard-control`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Phase 4: keyboard control. Steer the ship with the keyboard, proving input events flow from winit → Rust → Python callbacks (engine spec §5, §10 Phase 4): on_key_press(key, modifiers) / on_key_release(key, modifiers) Window callbacks, key constants (fc.key.LEFT, fc.key.A, ...) using physical key codes so WASD works on any layout, and optional polling via window.is_key_pressed(key). Done when arrow keys/WASD move the ship smoothly, multiple held keys work (diagonals), and nothing is lost on key repeat. Must also work in the browser build. Owner note: the collision system architecture (§11) must be decided either before Phase 5 or as part of Phase 5."

**Context**: This is roadmap Phase 4 (`docs/spec/ENGINE_SPEC.md` §5, §10). These points are already decided:
- Users subclass `Window` and override callbacks (`on_draw`, `on_update`, and now input callbacks).
- **Keys are identified by physical position**, not by the character they type, so WASD sits in the same place on QWERTY, AZERTY, Dvorak, and other layouts.
- Motion is per second, with fixed-timestep updates (Phase 2). Callback exceptions propagate and stop the game (Phase 1).
- The game runs on Windows, macOS (portable by design), and in the browser (approach B).

**Out of scope**: mouse, gamepad, text entry (typing characters, IME), and key remapping. Mouse and gamepad stay in the backlog.

**Related owner note (not part of this phase)**: the collision system architecture (engine spec §11) must be decided **before or as part of Phase 5**. This phase doesn't touch collisions.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - React to key presses and releases (Priority: P1)

A game developer overrides `on_key_press(key, modifiers)` and `on_key_release(key, modifiers)` on their `Window` subclass. When the player presses or releases a key, the matching method is called once with a key constant (for example `fc.key.SPACE`) and the modifier keys held at that moment (Shift, Ctrl, Alt, and the system/Command key).

**Why this priority**: event callbacks are the foundation of all keyboard input. Polling and the example are built on the same key data.

**Independent Test**: a test window presses and releases keys (simulated input) and records the callback calls. Each press yields exactly one `on_key_press` call and each release exactly one `on_key_release` call, with the right key and modifiers.

**Acceptance Scenarios**:

1. **Given** a window that overrides both callbacks, **When** the player presses and releases Space, **Then** `on_key_press(fc.key.SPACE, modifiers)` is called once, followed by `on_key_release(fc.key.SPACE, modifiers)` once.
2. **Given** the player holds Shift, **When** they press A, **Then** the modifiers passed for A include Shift, and checking for Ctrl reports it isn't held.
3. **Given** the player holds a key long enough for the operating system to auto-repeat, **When** repeats occur, **Then** no extra `on_key_press` calls are made. There is one press when the key goes down and one release when it comes up.
4. **Given** a window that doesn't override the callbacks, **When** keys are pressed, **Then** nothing happens and no error occurs.
5. **Given** a keyboard with a non-QWERTY layout (for example AZERTY), **When** the player presses the key in the physical position of QWERTY's W, **Then** the game receives `fc.key.W`.

---

### User Story 2 - Steer the ship with the keyboard (Priority: P1)

A player runs the Phase 4 example and steers the ship with the arrow keys or WASD. The ship moves while a key is held and stops when it's released. Holding two directions (for example Up and Right) moves it diagonally. The ship stays inside the window.

**Why this priority**: this is the phase's acceptance demo ("done when"), and it proves the whole path from key to motion.

**Independent Test**: run the example and steer by hand. An automated check drives the example with simulated input and confirms the ship's position changes in the expected directions.

**Acceptance Scenarios**:

1. **Given** the example is running, **When** the player holds Right (or D), **Then** the ship moves right at a steady speed and stops when the key is released.
2. **Given** the player holds Up and Right together, **When** time passes, **Then** the ship moves diagonally up and to the right, at the same overall speed as straight movement.
3. **Given** the player holds Left and Right together, **When** time passes, **Then** the ship doesn't move horizontally (the directions cancel).
4. **Given** the ship is at a window edge, **When** the player keeps pushing toward that edge, **Then** the ship stays fully inside the window.
5. **Given** the player taps a key many times quickly, **When** they stop, **Then** the ship is not left moving (no lost releases).

---

### User Story 3 - Ask whether a key is held (Priority: P2)

Instead of tracking presses and releases themselves, a developer calls `self.is_key_pressed(fc.key.LEFT)` inside `on_update` to check whether a key is currently held. That suits continuous movement.

**Why this priority**: a convenience that removes common bookkeeping. The callbacks alone are enough to build any game, so this comes second.

**Independent Test**: with simulated input, a test checks that `is_key_pressed` is true between a key's press and its release and false otherwise.

**Acceptance Scenarios**:

1. **Given** the player is holding Left, **When** `on_update` checks `is_key_pressed(fc.key.LEFT)`, **Then** it returns `True`.
2. **Given** the player has released Left, **When** the next `on_update` checks it, **Then** it returns `False`.
3. **Given** a key was pressed and released between two updates, **When** the developer relies on callbacks, **Then** both callbacks are still delivered. (Polling may report `False` for that key, which is expected; the callbacks are the reliable record.)

---

### User Story 4 - Same keyboard behavior in the browser (Priority: P2)

The Phase 4 example runs unchanged in the browser. Pressing keys while the game canvas has focus steers the ship, and the arrow keys and Space don't scroll the web page.

**Why this priority**: the web is a supported platform (constitution III). It's P2 because it reuses the desktop behavior.

**Independent Test**: the headless browser check sends key events to the example page and confirms the ship moves and the page doesn't scroll.

**Acceptance Scenarios**:

1. **Given** the example is open in the browser and the canvas has focus, **When** the player holds Right, **Then** the ship moves right just as on desktop.
2. **Given** the page is taller than the window, **When** the player presses the arrow keys or Space on the canvas, **Then** the page doesn't scroll.
3. **Given** the same game code, **When** it runs on desktop and in the browser, **Then** the same key constants and modifier checks work in both.

### Edge Cases

- **Focus loss:** if the window (or browser tab) loses focus while keys are held, every held key is treated as released, and `on_key_release` is delivered for each. That prevents a "stuck" key from moving the ship forever. Keys still held when focus returns are not re-pressed automatically; the player presses them again.
- **Unsupported keys:** keys without a named constant (for example rare media keys) aren't reported in this phase.
- **Same key, both sides:** left and right Shift (and Ctrl, Alt, system) are separate keys with separate constants. The modifier check reports "Shift" if either is held.
- **Callback errors:** an exception raised in `on_key_press` or `on_key_release` stops the game and is shown to the developer, the same as other callbacks.
- **Event timing:** key events that arrive between frames are delivered in the order they happened, before the next `on_update`, so `on_update` always sees up-to-date key state.
- **Many keys at once:** holding several keys at once works up to the limit of the physical keyboard (some keyboards can't report certain combinations; that's a hardware limit, not an engine one).
- **Invalid argument:** calling `is_key_pressed` with something that isn't a key raises a clear error.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `Window` MUST support overridable `on_key_press(key, modifiers)` and `on_key_release(key, modifiers)` callbacks. Not overriding them MUST be valid.
- **FR-002**: Each physical key press MUST produce exactly one `on_key_press`, and each release exactly one `on_key_release`. Operating-system auto-repeat MUST NOT produce extra press calls.
- **FR-003**: Keys MUST be identified by physical position, independent of the keyboard layout.
- **FR-004**: The engine MUST provide named key constants in `fc.key` covering at least: letters A–Z, digits 0–9 (top row), F1–F12, the arrow keys, Space, Enter, Escape, Tab, Backspace, Insert, Delete, Home, End, Page Up, Page Down, left/right Shift, Ctrl, Alt and system (Windows/Command) keys, the numeric keypad digits and operators, and common punctuation keys.
- **FR-005**: `modifiers` MUST tell the game which of Shift, Ctrl, Alt, and system (Command on macOS, Windows key on Windows) were held at the moment of the event, in a form that is easy to test (for example `modifiers & fc.key.MOD_SHIFT`). Either side of a modifier counts as held.
- **FR-006**: `Window.is_key_pressed(key)` MUST return whether the key is currently held, reflecting all key events received before the current `on_update`.
- **FR-007**: When the window loses focus, all held keys MUST be released, with `on_key_release` delivered for each.
- **FR-008**: Key events MUST be delivered between updates, in arrival order, never in the middle of an `on_update` or `on_draw` call.
- **FR-009**: Exceptions raised in the key callbacks MUST propagate the same way as in other callbacks.
- **FR-010**: A Phase 4 example (`examples/04_keyboard_ship.py`) MUST let the player steer the ship with the arrow keys and WASD. It MUST move at a constant speed (per second) while keys are held, keep the same speed diagonally, cancel opposite directions, and keep the ship inside the window.
- **FR-011**: The browser build MUST support the same callbacks, constants, modifiers, and polling, so the example runs unchanged. Keys the game receives MUST NOT also scroll or otherwise act on the page.
- **FR-012**: Key handling MUST add no noticeable per-frame cost when no keys are pressed, and MUST NOT add per-sprite work (constitution II).
- **FR-013**: The type stubs MUST describe the new callbacks, constants, and `is_key_pressed`.

### Key Entities

- **Key**: a named constant identifying one physical key position (for example `fc.key.A`, `fc.key.LEFT`, `fc.key.LSHIFT`).
- **Modifiers**: the set of modifier keys (Shift, Ctrl, Alt, system) held at the time of a key event.
- **Key state**: the set of keys currently held, which the engine maintains and `is_key_pressed` reads.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In automated tests, 100% of simulated key presses and releases produce exactly one matching callback each, including while a key auto-repeats.
- **SC-002**: The ship starts moving on the first frame after a key is pressed and stops on the first frame after release (within one frame, about 17 ms at 60 FPS).
- **SC-003**: Diagonal movement covers the same distance per second as straight movement, within 1%.
- **SC-004**: After 100 rapid random key taps (including several keys at once), no key is reported as held once all keys are released.
- **SC-005**: The example behaves the same on desktop and in the browser (same keys, same movement), confirmed by the automated browser check.
- **SC-006**: With no keys pressed, frame time is unchanged from Phase 3 (within measurement noise).
- **SC-007**: A developer can add keyboard steering to the Phase 2 example in under 20 lines of Python.

## Assumptions

- **Auto-repeat is ignored** for game callbacks: one press, one release. This matches game-engine norms (Arcade, Unity, Godot's default action handling). A repeat flag or text-typing input can come later with text input.
- **Focus loss releases keys** (FR-007), with release callbacks. This is the safer choice for games; some engines skip the callbacks, but stuck movement is a common bug.
- **Modifiers are bit flags** (`fc.key.MOD_SHIFT`, `MOD_CTRL`, `MOD_ALT`, `MOD_SYSTEM`), as in Arcade/pyglet. The exact names are confirmed in the plan.
- **Key constant names** follow Arcade where practical (`fc.key.LEFT`, `fc.key.A`, `fc.key.SPACE`, `fc.key.ENTER`), with `L`/`R` prefixes for sided modifiers (`fc.key.LSHIFT`).
- **Polling is updated between fixed-timestep updates.** A press and release that both happen between two updates are seen by the callbacks but may be missed by polling (documented).
- **The example's ship speed** is a constant chosen in the example (for example 300 px/s), and the ship doesn't rotate.
- **In the browser**, keys reach the game only when the canvas has focus. The loader gives the canvas focus on load and when clicked.
- **Testing** uses simulated key events injected through a test-only path on desktop, and synthetic keyboard events in the headless browser check, since real key presses can't be automated reliably.
