# Contract: Phase 4 Python API additions

These are additions to the existing API (`specs/004-bouncing-ship/contracts`). They behave the same
on desktop and in the browser unless noted.

## `ferrocade.key` (module, also `fc.key`)

```python
fc.key.A ... fc.key.Z            # letters, by physical position (QWERTY names)
fc.key.KEY_0 ... fc.key.KEY_9    # top-row digits
fc.key.F1 ... fc.key.F12
fc.key.LEFT, RIGHT, UP, DOWN
fc.key.SPACE, ENTER, ESCAPE, TAB, BACKSPACE, INSERT, DELETE, HOME, END, PAGE_UP, PAGE_DOWN, CAPS_LOCK
fc.key.LSHIFT, RSHIFT, LCTRL, RCTRL, LALT, RALT, LSYSTEM, RSYSTEM   # SYSTEM = Windows / Command key
fc.key.NUM_0 ... NUM_9, NUM_ADD, NUM_SUBTRACT, NUM_MULTIPLY, NUM_DIVIDE, NUM_DECIMAL, NUM_ENTER, NUM_LOCK
fc.key.MINUS, EQUAL, BRACKET_LEFT, BRACKET_RIGHT, BACKSLASH, SEMICOLON, APOSTROPHE, GRAVE,
        COMMA, PERIOD, SLASH

fc.key.MOD_SHIFT, MOD_CTRL, MOD_ALT, MOD_SYSTEM    # bit flags for `modifiers`

fc.key.name(key: int) -> str     # e.g. name(fc.key.LEFT) == "LEFT"; ValueError for unknown codes
```

- All constants are plain `int`s.
- Letter and digit constants name the key's **position** on a US QWERTY keyboard. On AZERTY,
  the key labeled Z reports `fc.key.W`.

## `Window` callbacks (override in a subclass)

```python
def on_key_press(self, key: int, modifiers: int) -> None: ...
def on_key_release(self, key: int, modifiers: int) -> None: ...
```

- Called once per press and once per release. OS auto-repeat is ignored.
- Delivered before the frame's `on_update` calls, in the order the keys were pressed and released.
- `modifiers` is the set of modifier keys held after the event: `modifiers & fc.key.MOD_SHIFT`.
- When the window loses focus, `on_key_release` is called for every held key.
- The defaults do nothing. Exceptions propagate out of `run()`, as with other callbacks.

## `Window.is_key_pressed(key: int) -> bool`

- `True` while the key is held. The value reflects every key event delivered before the current
  callback.
- A press and release that both happen between two frames are delivered to the callbacks, but
  polling won't see the key as held.
- `TypeError` if `key` isn't an `int`. Unknown codes return `False`.

## Private, test-only (desktop)

- `Window._inject_key(key, pressed, repeat=False)`: queues a raw key event as if the platform had
  sent it.
- `Window._inject_focus_lost()`: acts like the window losing focus.

## Browser specifics

- Keys reach the game while the canvas has focus. The page focuses it on start and on click.
- Recognized keys don't trigger page behavior (scrolling, quick-find), except with Ctrl, Alt or
  Meta held, and except Tab and F1–F12.
