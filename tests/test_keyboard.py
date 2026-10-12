"""Keyboard input: callbacks, polling, focus loss, and the generated key constants.

Real key presses can't be automated, so display tests inject raw key events with the private
`Window._inject_key`, which feeds the same queue the platform uses. Events injected during one
frame are delivered at the start of the next frame, before its `on_update` calls.
"""

import random
import subprocess
import sys
from pathlib import Path

import pytest

import ferrocade as fc
from ferrocade import key

ROOT = Path(__file__).resolve().parent.parent


class StopTest(Exception):
    """Raised from a callback to end a display test's event loop."""


class ScriptedWindow(fc.Window):
    """Runs `script[n](self)` on update number n, records key callbacks, and stops after
    `updates` updates."""

    def __init__(self, script, updates=None, watch=()):
        super().__init__(320, 240, "keyboard test", filter=fc.NEAREST)
        self.script = script
        self.updates = updates if updates is not None else max(script, default=0) + 10
        self.watch = watch
        self.n = 0
        self.events = []
        self.polled = []  # (update number, {watched key: is_key_pressed})

    def on_update(self, delta_time):
        self.polled.append((self.n, {k: self.is_key_pressed(k) for k in self.watch}))
        if self.n in self.script:
            self.script[self.n](self)
        self.n += 1
        if self.n > self.updates:
            raise StopTest

    def on_key_press(self, k, modifiers):
        self.events.append(("press", k, modifiers))

    def on_key_release(self, k, modifiers):
        self.events.append(("release", k, modifiers))


def run(window):
    with pytest.raises(StopTest):
        window.run()
    return window


def press(k, repeat=False):
    return lambda w: w._inject_key(k, True, repeat)


def release(k):
    return lambda w: w._inject_key(k, False)


def steps(*actions):
    return lambda w: [a(w) for a in actions]


# --- No display needed --------------------------------------------------------------------


def test_key_constants_and_names():
    assert key.LEFT != key.RIGHT
    assert key.name(key.LEFT) == "LEFT"
    assert key.name(key.KEY_0) == "KEY_0"
    assert key.name(key.LSYSTEM) == "LSYSTEM"
    assert {key.MOD_SHIFT, key.MOD_CTRL, key.MOD_ALT, key.MOD_SYSTEM} == {1, 2, 4, 8}
    with pytest.raises(ValueError):
        key.name(9999)
    assert fc.key is key


def test_generated_key_module_is_up_to_date():
    result = subprocess.run([sys.executable, str(ROOT / "tools" / "gen_keys.py"), "--check"],
                            capture_output=True, text=True)
    assert result.returncode == 0, result.stdout + result.stderr


def test_is_key_pressed_before_run_and_argument_checks():
    w = fc.Window(320, 240, "t", filter=fc.NEAREST)
    assert w.is_key_pressed(key.A) is False
    assert w.is_key_pressed(9999) is False
    with pytest.raises(TypeError):
        w.is_key_pressed("a")
    with pytest.raises(ValueError, match="unknown key"):
        w._inject_key(9999, True)


# --- Needs a display ----------------------------------------------------------------------


@pytest.mark.display
def test_press_and_release_are_delivered_once_each():
    w = run(ScriptedWindow({2: press(key.SPACE), 6: release(key.SPACE)}))
    assert w.events == [("press", key.SPACE, 0), ("release", key.SPACE, 0)]


@pytest.mark.display
def test_auto_repeat_is_ignored():
    w = run(ScriptedWindow({
        2: press(key.A),
        3: steps(*[press(key.A, repeat=True)] * 3),
        4: press(key.A),  # duplicate press without the repeat flag
        6: release(key.A),
    }))
    assert w.events == [("press", key.A, 0), ("release", key.A, 0)]


@pytest.mark.display
def test_modifiers_reflect_held_keys():
    w = run(ScriptedWindow({
        2: press(key.LSHIFT),
        4: press(key.A),
        6: steps(release(key.A), release(key.LSHIFT)),
    }))
    assert w.events == [
        ("press", key.LSHIFT, key.MOD_SHIFT),
        ("press", key.A, key.MOD_SHIFT),
        ("release", key.A, key.MOD_SHIFT),
        ("release", key.LSHIFT, 0),
    ]
    assert not w.events[1][2] & key.MOD_CTRL


@pytest.mark.display
def test_focus_loss_releases_held_keys():
    w = run(ScriptedWindow({
        2: steps(press(key.LEFT), press(key.W)),
        5: lambda w: w._inject_focus_lost(),
    }, watch=(key.LEFT, key.W)))
    releases = {k for kind, k, _ in w.events if kind == "release"}
    assert releases == {key.LEFT, key.W}
    assert w.polled[-1][1] == {key.LEFT: False, key.W: False}


@pytest.mark.display
def test_callback_exception_propagates():
    class Boom(ScriptedWindow):
        def on_key_press(self, k, modifiers):
            raise ValueError("key boom")

    with pytest.raises(ValueError, match="key boom"):
        Boom({2: press(key.ENTER)}).run()


@pytest.mark.display
def test_window_without_overrides_accepts_keys():
    class Plain(fc.Window):
        n = 0

        def on_update(self, delta_time):
            if Plain.n == 2:
                self._inject_key(key.SPACE, True)
            Plain.n += 1
            if Plain.n > 10:
                raise StopTest

    with pytest.raises(StopTest):
        Plain(320, 240, "t", filter=fc.NEAREST).run()


@pytest.mark.display
def test_is_key_pressed_follows_delivered_events():
    seen_in_callback = []

    class Polling(ScriptedWindow):
        def on_key_press(self, k, modifiers):
            seen_in_callback.append(self.is_key_pressed(k))
            super().on_key_press(k, modifiers)

    w = run(Polling({2: press(key.LEFT), 8: release(key.LEFT)}, updates=14, watch=(key.LEFT,)))
    held = [n for n, state in w.polled if state[key.LEFT]]
    assert seen_in_callback == [True]
    assert held, "LEFT was never seen as held"
    assert held[0] > 2 and held[-1] <= 8, held
    assert held == list(range(held[0], held[-1] + 1)), "held without gaps"
    assert not w.polled[-1][1][key.LEFT]


@pytest.mark.display
def test_tap_within_one_frame_reaches_callbacks_but_not_polling():
    w = run(ScriptedWindow({2: steps(press(key.B), release(key.B))}, watch=(key.B,)))
    assert w.events == [("press", key.B, 0), ("release", key.B, 0)]
    assert not any(state[key.B] for _, state in w.polled)


@pytest.mark.display
def test_random_taps_leave_no_key_held():
    rng = random.Random(4)
    keys = [key.LEFT, key.RIGHT, key.UP, key.DOWN, key.W, key.A, key.S, key.D, key.LSHIFT]
    script = {}
    for n in range(2, 102):
        actions = [press(k) if rng.random() < 0.5 else release(k)
                   for k in rng.sample(keys, rng.randint(1, 3))]
        script[n] = steps(*actions)
    script[103] = steps(*[release(k) for k in keys])
    w = run(ScriptedWindow(script, watch=tuple(keys)))
    assert not any(w.polled[-1][1].values())
    presses = sum(1 for e in w.events if e[0] == "press")
    releases = sum(1 for e in w.events if e[0] == "release")
    assert presses == releases > 0


# --- The Phase 4 example ----------------------------------------------------------------


def load_example(monkeypatch):
    import importlib.util

    monkeypatch.chdir(ROOT)  # the example loads assets/ relative to the repository root
    spec = importlib.util.spec_from_file_location("keyboard_ship",
                                                  ROOT / "examples" / "04_keyboard_ship.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def drive_example(monkeypatch, script, updates):
    """Run the example's window with injected keys; returns ship positions after each update."""
    example = load_example(monkeypatch)

    class Driven(example.KeyboardShip):
        n = 0
        positions = []

        def on_update(self, delta_time):
            if self.n in script:
                script[self.n](self)
            super().on_update(delta_time)
            self.positions.append(self.ship.position)
            self.n += 1
            if self.n > updates:
                raise StopTest

    window = Driven()
    with pytest.raises(StopTest):
        window.run()
    return example, window


def moving_steps(positions):
    """Per-update displacements while the ship was moving."""
    steps_ = [(b[0] - a[0], b[1] - a[1]) for a, b in zip(positions, positions[1:])]
    return [s for s in steps_ if s != (0.0, 0.0)]


@pytest.mark.display
def test_example_moves_right_at_constant_speed(monkeypatch):
    example, w = drive_example(monkeypatch, {2: press(key.RIGHT), 32: release(key.RIGHT)}, 40)
    per_update = example.SPEED * w.update_rate
    moved = moving_steps(w.positions)
    assert 25 <= len(moved) <= 31, len(moved)
    for dx, dy in moved:
        assert dx == pytest.approx(per_update) and dy == 0
    assert w.positions[-1] == w.positions[-3], "the ship stops after the release"


@pytest.mark.display
def test_example_diagonal_keeps_the_same_speed(monkeypatch):
    example, w = drive_example(monkeypatch, {2: steps(press(key.UP), press(key.D))}, 20)
    per_update = example.SPEED * w.update_rate
    moved = moving_steps(w.positions)
    assert moved
    for dx, dy in moved:
        assert dx > 0 and dy > 0
        assert (dx * dx + dy * dy) ** 0.5 == pytest.approx(per_update, rel=0.01)


@pytest.mark.display
def test_example_opposite_keys_cancel_and_edges_clamp(monkeypatch):
    example, w = drive_example(monkeypatch, {
        2: steps(press(key.LEFT), press(key.RIGHT)),
        10: release(key.RIGHT),  # then LEFT alone, long enough to reach the edge
    }, 150)
    start = w.positions[0]
    assert w.positions[8] == start, "LEFT+RIGHT cancel"
    x, y = w.positions[-1]
    assert x == pytest.approx(w.ship.width / 2)
    assert y == start[1]


@pytest.mark.display
def test_example_space_recenters(monkeypatch):
    example, w = drive_example(monkeypatch, {
        2: press(key.UP), 20: steps(release(key.UP), press(key.SPACE)),
    }, 26)
    assert w.positions[-1] == (example.WIDTH / 2, example.HEIGHT / 2)
