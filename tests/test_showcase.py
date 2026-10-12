"""Performance showcase: bounce calls, engine timings, Python-mode parity, and the example."""

import importlib.util
import math
import subprocess
import sys
from pathlib import Path

import pytest

import ferrocade as fc

ROOT = Path(__file__).resolve().parent.parent


def load_example(monkeypatch=None):
    if monkeypatch is not None:
        monkeypatch.chdir(ROOT)
    spec = importlib.util.spec_from_file_location("showcase", ROOT / "examples" / "06_showcase.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def ball(x, y, r, v):
    b = fc.CircleSprite(r, (255, 255, 255), x, y)
    b.collider = fc.CircleCollider(r)
    b.velocity = v
    return b


def wall(x, y, w, h):
    s = fc.RectangleSprite(w, h, (128, 128, 128), x, y)
    s.collider = fc.AABBCollider(w, h)
    return s


def sprite_list(*sprites):
    lst = fc.SpriteList()
    for s in sprites:
        lst.append(s)
    return lst


# --- US1: bounce calls (no display) -------------------------------------------------------


def test_head_on_balls_swap_velocities():
    a, b = ball(0, 0, 5, (10, 0)), ball(10, 0, 5, (-10, 0))
    balls = sprite_list(a, b)
    assert balls.bounce_each_other() == 1
    assert a.velocity == pytest.approx((-10, 0)) and b.velocity == pytest.approx((10, 0))


def test_ball_reflects_off_a_wall():
    floor = wall(0, -5, 200, 10)
    b = ball(0, 2, 3, (40, -30))
    assert sprite_list(b).bounce_off(sprite_list(floor)) == 1
    assert b.velocity == pytest.approx((40, 30))
    assert b.position[1] == pytest.approx(3), "pushed out to touching"


def test_box_sprites_in_the_ball_list_are_not_moved():
    box = wall(0, 0, 10, 10)
    box.velocity = (5, 0)
    b = ball(6, 0, 3, (-5, 0))
    assert sprite_list(box, b).bounce_each_other() == 0
    assert box.velocity == (5, 0) and b.velocity == (-5, 0)


def test_balls_stay_inside_the_walls(monkeypatch):
    example = load_example(monkeypatch)
    w = example.Showcase(balls=1500)
    for _ in range(300):
        w.step(1 / 60)
    for b in w.ball_list:
        x, y = b.position
        assert example.WALL <= x <= example.WIDTH - example.WALL
        assert example.WALL <= y <= example.HEIGHT - example.WALL


# --- US3: Python mode matches the engine (no display) -------------------------------------


def test_python_mode_matches_engine_mode(monkeypatch):
    example = load_example(monkeypatch)
    engine, python = example.Showcase(balls=1000), example.Showcase(balls=1000)
    python.mode = "python"
    for _ in range(60):
        engine.step(1 / 60)
        python.step(1 / 60)
    worst = max(math.dist(a.position, b.position)
                for a, b in zip(engine.ball_list, python.ball_list))
    assert worst <= 1e-6, worst


# --- US2: engine timings ------------------------------------------------------------------


class StopTest(Exception):
    pass


@pytest.mark.display
def test_frame_stats_timings():
    class Plain(fc.Window):
        n = 0

        def on_update(self, delta_time):
            Plain.n += 1
            if Plain.n > 80:
                raise StopTest

    plain = Plain(320, 240, "t", filter=fc.NEAREST)
    with pytest.raises(StopTest):
        plain.run()
    assert plain.frame_stats.update_ms == 0 and plain.frame_stats.collision_ms == 0

    class Busy(fc.Window):
        n = 0

        def __init__(self):
            super().__init__(320, 240, "t", filter=fc.NEAREST)
            self.balls = sprite_list(*[ball(10 + (i % 60) * 5, 10 + (i // 60) * 5, 2, (20, 10))
                                       for i in range(2000)])

        def on_update(self, delta_time):
            self.balls.update(delta_time)
            self.balls.bounce_each_other()
            Busy.n += 1
            if Busy.n > 80:
                raise StopTest

    busy = Busy()
    with pytest.raises(StopTest):
        busy.run()
    assert busy.frame_stats.update_ms > 0 and busy.frame_stats.collision_ms > 0


@pytest.mark.display
def test_example_prints_stats_lines():
    result = subprocess.run(
        [sys.executable, str(ROOT / "examples" / "06_showcase.py"), "--balls", "500", "--seconds", "2.5"],
        cwd=ROOT, capture_output=True, text=True, timeout=60)
    lines = [line for line in result.stdout.splitlines() if line.startswith("balls=500")]
    assert result.returncode == 0, result.stderr
    assert len(lines) >= 2, result.stdout
    assert "collision=" in lines[-1] and "mode=engine" in lines[-1]


# --- US4: controls --------------------------------------------------------------------------


@pytest.mark.display
def test_keys_add_remove_pause_and_toggle(monkeypatch):
    example = load_example(monkeypatch)
    K = fc.key
    seen = {}

    class Driven(example.Showcase):
        n = 0
        script = {
            2: (K.EQUAL,), 4: (K.MINUS,), 6: (K.MINUS,), 8: (K.NUM_ADD,),
            10: (K.SPACE,), 22: (K.SPACE,), 24: (K.T,), 26: (K.T,),
        }

        def on_update(self, delta_time):
            for key in self.script.get(self.n, ()):
                self._inject_key(key, True)
                self._inject_key(key, False)
            seen[self.n] = (len(self.ball_list), self.paused, self.mode,
                            self.ball_list[0].position if self.ball_list else None)
            super().on_update(delta_time)
            self.n += 1
            if self.n > 30:
                raise StopTest

    w = Driven(balls=500)
    with pytest.raises(StopTest):
        w.run()
    counts = [seen[n][0] for n in sorted(seen)]
    assert 1500 in counts and 0 in counts and counts[-1] == 1000
    paused = [n for n in sorted(seen) if seen[n][1]]
    assert paused, "pause took effect"
    frozen = [seen[n][3] for n in paused[1:]]
    assert len(set(frozen)) == 1, "nothing moves while paused"
    assert any(seen[n][2] == "python" for n in seen) and seen[max(seen)][2] == "engine"
