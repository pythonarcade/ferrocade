import math
import time

import pytest

import ferrocade as fc

SHIP = "assets/images/ship.png"


def test_velocity_defaults():
    s = fc.Sprite(SHIP)
    assert (s.velocity_x, s.velocity_y, s.angular_velocity) == (0.0, 0.0, 0.0)
    assert s.velocity == (0.0, 0.0)


def test_velocity_round_trip():
    s = fc.Sprite(SHIP)
    s.velocity_x = 0.1
    s.velocity_y = -2.25
    s.angular_velocity = 12.5
    assert (s.velocity_x, s.velocity_y, s.angular_velocity) == (0.1, -2.25, 12.5)
    s.velocity = (1.5, -3.0)
    assert s.velocity == (1.5, -3.0)


@pytest.mark.parametrize("bad", [math.nan, math.inf, -math.inf])
def test_velocity_validation(bad):
    s = fc.Sprite(SHIP)
    for name in ("velocity_x", "velocity_y", "angular_velocity"):
        with pytest.raises(ValueError):
            setattr(s, name, bad)
    with pytest.raises(ValueError):
        s.velocity = (0.0, bad)


def test_update_moves_by_velocity():
    s = fc.Sprite(SHIP, 100, 100)
    s.velocity = (200, -100)
    s.angular_velocity = 90
    lst = fc.SpriteList()
    lst.append(s)
    for _ in range(60):
        lst.update(1 / 60)
    assert s.center_x == pytest.approx(300, abs=1e-6)
    assert s.center_y == pytest.approx(0, abs=1e-6)
    assert s.angle == pytest.approx(90, abs=1e-6)


def test_no_update_no_motion():
    s = fc.Sprite(SHIP, 10, 10)
    s.velocity = (100, 100)
    assert s.position == (10.0, 10.0)


@pytest.mark.parametrize("bad", [-1.0, math.nan, math.inf])
def test_update_validation(bad):
    with pytest.raises(ValueError):
        fc.SpriteList().update(bad)


def test_sprite_in_two_lists_moves_per_list_updated():
    s = fc.Sprite(SHIP)
    s.velocity_x = 10
    a, b = fc.SpriteList(), fc.SpriteList()
    a.append(s)
    b.append(s)
    a.update(1.0)
    b.update(1.0)
    assert s.center_x == 20.0


def test_shapes_and_subclasses_move():
    class Ball(fc.CircleSprite):
        def __init__(self):
            super().__init__(5, fc.color.ORANGE)
            self.velocity = (3, 4)

    r = fc.RectangleSprite(10, 10, fc.color.RED)
    r.velocity_y = 7
    lst = fc.SpriteList()
    ball = Ball()
    lst.append(ball)
    lst.append(r)
    lst.update(2.0)
    assert ball.position == (6.0, 8.0)
    assert r.center_y == 14.0


# --- Frame-rate independence (needs a display) ---------------------------------------------


class StopTest(Exception):
    pass


def run_timed(seconds, update_rate=1 / 60, draw_delay=0.0):
    """Run a window for `seconds` of wall-clock time; return (updates, distance moved)."""

    class Timed(fc.Window):
        def __init__(self):
            super().__init__(320, 240, "timing", filter=fc.NEAREST, update_rate=update_rate)
            self.sprite = fc.RectangleSprite(10, 10, fc.color.WHITE, 0, 120)
            self.sprite.velocity_x = 100
            self.sprites = fc.SpriteList()
            self.sprites.append(self.sprite)
            self.updates = 0
            self.start_x = None
            self.counting_from = None
            self.deadline = None

        def on_update(self, delta_time):
            # Warm up first: window startup can cause one catch-up batch of updates, which
            # would otherwise be counted against a measuring window that hadn't really begun.
            now = time.perf_counter()
            self.sprites.update(delta_time)
            if self.counting_from is None:
                self.counting_from = now + 0.5
            elif self.deadline is None and now >= self.counting_from:
                self.deadline = now + seconds
                self.start_x = self.sprite.center_x
            elif self.deadline is not None:
                if now >= self.deadline:
                    raise StopTest
                self.updates += 1

        def on_draw(self):
            self.clear()
            self.sprites.draw()
            if draw_delay:
                time.sleep(draw_delay)

    window = Timed()
    with pytest.raises(StopTest):
        window.run()
    return window.updates, window.sprite.center_x - window.start_x


@pytest.mark.display
@pytest.mark.parametrize("draw_delay", [0.0, 0.05], ids=["full-rate", "slow-drawing"])
def test_update_rate_independent_of_drawing(draw_delay):
    updates, distance = run_timed(3.0, draw_delay=draw_delay)
    assert updates == pytest.approx(180, rel=0.03)
    assert distance == pytest.approx(300, rel=0.03)


@pytest.mark.display
def test_slower_update_rate_same_speed():
    updates, distance = run_timed(3.0, update_rate=1 / 30)
    assert updates == pytest.approx(90, rel=0.03)
    assert distance == pytest.approx(300, rel=0.03)
