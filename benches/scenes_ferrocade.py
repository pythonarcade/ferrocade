"""Ferrocade versions of the stress-test scenes (driven by benches/stress.py)."""

import random
import time

import ferrocade as fc

from stress import HEIGHT, SPRITE_IMAGE, SPRITE_SCALE, WIDTH, Done

DRIFT = 20.0  # px/s: slow enough that sprites stay on screen during a measurement window


def unsupported(scene):
    return None


def _random_position():
    return random.uniform(20, WIDTH - 20), random.uniform(20, HEIGHT - 20)


class Scene:
    """N sprites in one list; `resize` changes N between measurement steps."""

    def __init__(self, kind):
        self.kind = kind
        self.texture = fc.Texture(SPRITE_IMAGE)
        self.colors = list(fc.color.CSS_COLORS.values())
        self.sprites = []
        self.list = fc.SpriteList()
        # python_moved keeps its own positions and velocities, as a Python game would.
        self.py_state = []

    def _make(self):
        x, y = _random_position()
        if self.kind == "shapes":
            size = random.uniform(4, 12)
            if random.random() < 0.5:
                sprite = fc.RectangleSprite(size, size, random.choice(self.colors), x, y)
            else:
                sprite = fc.CircleSprite(size / 2, random.choice(self.colors), x, y)
        else:
            sprite = fc.Sprite(self.texture, x, y, SPRITE_SCALE)
        if self.kind in ("engine_moved", "shapes"):
            sprite.velocity = (random.uniform(-DRIFT, DRIFT), random.uniform(-DRIFT, DRIFT))
            sprite.angular_velocity = random.uniform(-90, 90)
        return sprite

    def resize(self, n):
        if n < len(self.sprites):
            # Shrinking: rebuild the list (removing thousands one at a time would be O(n^2)).
            self.sprites = self.sprites[:n]
            self.py_state = self.py_state[:n]
            self.list = fc.SpriteList()
            for s in self.sprites:
                self.list.append(s)
        while len(self.sprites) < n:
            s = self._make()
            self.sprites.append(s)
            self.list.append(s)
            if self.kind == "python_moved":
                self.py_state.append([s.center_x, s.center_y,
                                      random.uniform(-DRIFT, DRIFT), random.uniform(-DRIFT, DRIFT)])
        if self.kind in ("engine_moved", "shapes"):
            for s in self.sprites:  # rescatter once per step so drift never leaves the window
                s.position = _random_position()

    def update(self, dt):
        """Returns (engine update ms, game code ms)."""
        if self.kind in ("engine_moved", "shapes"):
            t0 = time.perf_counter()
            self.list.update(dt)
            return (time.perf_counter() - t0) * 1000, 0.0
        if self.kind == "python_moved":
            t0 = time.perf_counter()
            for sprite, st in zip(self.sprites, self.py_state):
                st[0] = (st[0] + st[2] * dt) % WIDTH
                st[1] = (st[1] + st[3] * dt) % HEIGHT
                sprite.position = (st[0], st[1])
            return 0.0, (time.perf_counter() - t0) * 1000
        return 0.0, 0.0


class StressWindow(fc.Window):
    def __init__(self, scene, ramp):
        super().__init__(WIDTH, HEIGHT, f"Ferrocade stress: {scene}", filter=fc.NEAREST)
        self.scene = Scene(scene)
        self.ramp = ramp
        self.scene.resize(ramp.n)
        self.update_ms = self.game_ms = 0.0

    def on_update(self, delta_time):
        u, g = self.scene.update(delta_time)
        self.update_ms += u
        self.game_ms += g

    def on_draw(self):
        t0 = time.perf_counter()
        self.clear()
        self.scene.list.draw()
        draw_ms = (time.perf_counter() - t0) * 1000
        # render_cpu_ms is the engine's rolling average of its own render work.
        next_n = self.ramp.frame(time.perf_counter(), self.update_ms, self.game_ms, draw_ms,
                                 self.frame_stats.render_cpu_ms)
        self.update_ms = self.game_ms = 0.0
        if next_n is not None:
            self.scene.resize(next_n)


def run(scene, ramp):
    try:
        StressWindow(scene, ramp).run()
    except Done:
        pass
