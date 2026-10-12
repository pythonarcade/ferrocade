"""Python Arcade 4.0 (pre-release) versions of the stress-test scenes.

Written against arcade 4.0.0.dev8. Notes on equivalence:
- Arcade's `change_x`/`change_y`/`change_angle` are per *frame* (`Sprite.update` scales
  delta_time by 60), so per-second drift is divided by 60.
- `SpriteList.update()` is a Python-level loop over the sprites (Arcade has no engine-side
  bulk update), so "engine_moved" here measures Arcade's own update path.
- Drawing is synchronous OpenGL in `on_draw`, so draw_ms includes Arcade's render work.
- Pacing: vsync on (like Ferrocade) with update/draw rates of 1/240, so the display paces frames. With
  vsync off, pyglet's sleep-based pacing intermittently dropped to ~40-50 FPS regardless of load
  (frames of 16 or 33 ms). The process also requests 1 ms Windows timer resolution.
"""

import ctypes
import random
import sys
import time

import arcade

from stress import HEIGHT, SPRITE_IMAGE, SPRITE_SCALE, WIDTH, Done

DRIFT = 20.0  # px/s, as in the Ferrocade scenes


def unsupported(scene):
    return None


def _random_position():
    return random.uniform(20, WIDTH - 20), random.uniform(20, HEIGHT - 20)


class Scene:
    def __init__(self, kind):
        self.kind = kind
        self.texture = arcade.load_texture(SPRITE_IMAGE)
        self.sprites = []
        self.list = arcade.SpriteList()
        self.py_state = []
        self.colors = [c for name, c in vars(arcade.color).items()
                       if name.isupper() and isinstance(c, tuple)]

    def _make(self):
        x, y = _random_position()
        if self.kind == "shapes":
            size = random.randint(4, 12)
            color = random.choice(self.colors)
            if random.random() < 0.5:
                sprite = arcade.SpriteSolidColor(size, size, x, y, color)
            else:
                sprite = arcade.SpriteCircle(max(2, size // 2), color, center_x=x, center_y=y)
        else:
            sprite = arcade.Sprite(self.texture, SPRITE_SCALE, x, y)
        if self.kind in ("engine_moved", "shapes"):
            sprite.change_x = random.uniform(-DRIFT, DRIFT) / 60
            sprite.change_y = random.uniform(-DRIFT, DRIFT) / 60
            sprite.change_angle = random.uniform(-90, 90) / 60
        return sprite

    def resize(self, n):
        if n < len(self.sprites):
            self.sprites = self.sprites[:n]
            self.py_state = self.py_state[:n]
            self.list = arcade.SpriteList()
            self.list.extend(self.sprites)
        while len(self.sprites) < n:
            s = self._make()
            self.sprites.append(s)
            self.list.append(s)
            if self.kind == "python_moved":
                self.py_state.append([s.center_x, s.center_y,
                                      random.uniform(-DRIFT, DRIFT), random.uniform(-DRIFT, DRIFT)])
        if self.kind in ("engine_moved", "shapes"):
            for s in self.sprites:
                s.position = _random_position()

    def update(self, dt):
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


class StressWindow(arcade.Window):
    def __init__(self, scene, ramp):
        # Paced like Ferrocade: vsync on, with update/draw rates above the refresh rate so the
        # display (not pyglet's sleep-based scheduler) sets the pace: about one update and one
        # draw per frame. Arcade requires update_rate <= draw_rate; Sprite.update scales by the
        # real delta_time, so per-second speeds stay correct.
        super().__init__(WIDTH, HEIGHT, f"Arcade stress: {scene}", vsync=True,
                         update_rate=1 / 240, draw_rate=1 / 240)
        self.scene = Scene(scene)
        self.ramp = ramp
        self.scene.resize(ramp.n)
        self.update_ms = self.game_ms = 0.0
        self.finished = False

    def on_update(self, delta_time):
        u, g = self.scene.update(delta_time)
        self.update_ms += u
        self.game_ms += g

    def on_draw(self):
        if self.finished:
            return
        t0 = time.perf_counter()
        self.clear()
        self.scene.list.draw()
        draw_ms = (time.perf_counter() - t0) * 1000
        try:
            next_n = self.ramp.frame(time.perf_counter(), self.update_ms, self.game_ms, draw_ms)
        except Done:
            self.finished = True
            self.close()
            return
        self.update_ms = self.game_ms = 0.0
        if next_n is not None:
            self.scene.resize(next_n)


def run(scene, ramp):
    # pyglet paces frames with sleeps when vsync is off; Windows' default ~15.6 ms timer
    # granularity makes those sleeps overshoot into whole skipped frames (16 or 33 ms frames
    # even with almost no work). Ask for 1 ms resolution for the benchmark process.
    if sys.platform == "win32":
        ctypes.windll.winmm.timeBeginPeriod(1)
    StressWindow(scene, ramp)
    arcade.run()
