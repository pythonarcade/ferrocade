"""Phase 2: a ship that moves, spins, and bounces off the window edges.

The engine moves every sprite (one `update` call per list); Python only checks the edges.

Run from the repository root:
    python examples/02_bouncing_ship.py            # the bouncing, spinning ship
    python examples/02_bouncing_ship.py --many     # 1,000 bouncing shapes; prints the frame time
    python examples/02_bouncing_ship.py --count    # counts on_update calls for 10 s, then exits
    python examples/02_bouncing_ship.py --check    # asserts the ship stays inside for 60 s
    Add --fps to any mode to show FPS and frame time in the window title.
"""

import random
import sys
import time

import ferrocade as fc

WIDTH, HEIGHT = 800, 600


def bounce(sprite, width, height):
    """Reverse a sprite's velocity at the window edges and keep it fully inside."""
    half_w, half_h = sprite.width / 2, sprite.height / 2
    x, y = sprite.position
    vx, vy = sprite.velocity
    if not half_w <= x <= width - half_w:
        vx = -vx
        x = min(max(x, half_w), width - half_w)
    if not half_h <= y <= height - half_h:
        vy = -vy
        y = min(max(y, half_h), height - half_h)
    if (vx, vy) != sprite.velocity:
        sprite.velocity = (vx, vy)
        sprite.position = (x, y)


class Bounce(fc.Window):
    def __init__(self, mode):
        super().__init__(WIDTH, HEIGHT, "Bouncing ship", filter=fc.NEAREST)
        self.mode = mode
        self.show_fps = "--fps" in sys.argv
        self.sprites = fc.SpriteList()
        if mode == "many":
            self.add_many(1000)
        else:
            self.ship = fc.Sprite("assets/images/ship.png", self.width / 2, self.height / 2)
            self.ship.velocity = (220, 170)
            self.ship.angular_velocity = 45
            self.sprites.append(self.ship)
        self.updates = 0
        self.started = None
        self.frames = 0
        self.last_report = None

    def add_many(self, count):
        colors = list(fc.color.CSS_COLORS.values())
        for _ in range(count):
            size = random.uniform(4, 24)
            x = random.uniform(size, WIDTH - size)
            y = random.uniform(size, HEIGHT - size)
            if random.random() < 0.5:
                shape = fc.RectangleSprite(size, size, random.choice(colors), x, y)
                shape.angular_velocity = random.uniform(-180, 180)
            else:
                shape = fc.CircleSprite(size / 2, random.choice(colors), x, y)
            speed = lambda: random.choice((-1, 1)) * random.uniform(50, 250)  # noqa: E731
            shape.velocity = (speed(), speed())
            self.sprites.append(shape)

    def on_update(self, delta_time):
        now = time.perf_counter()
        if self.started is None:
            self.started = now
        self.updates += 1

        self.sprites.update(delta_time)  # the engine moves everything
        for sprite in self.sprites:
            bounce(sprite, self.width, self.height)

        if self.mode == "check":
            s = self.ship
            assert s.width / 2 <= s.center_x <= self.width - s.width / 2, s.position
            assert s.height / 2 <= s.center_y <= self.height - s.height / 2, s.position
            if now - self.started >= 60:
                print(f"OK: the ship stayed inside for 60 s ({self.updates} updates)")
                raise SystemExit(0)
        if self.mode == "count" and now - self.started >= 10:
            print(f"{self.updates} updates in {now - self.started:.2f} s "
                  f"({self.updates / (now - self.started):.2f} per second)")
            raise SystemExit(0)

    def on_draw(self):
        self.clear()
        self.sprites.draw()
        if self.mode != "many":
            return
        now = time.perf_counter()
        if self.last_report is None:  # don't count startup
            self.last_report = now
            return
        self.frames += 1
        if now - self.last_report >= 2.0:
            ms = (now - self.last_report) / self.frames * 1000
            print(f"{len(self.sprites)} sprites: {ms:.2f} ms/frame "
                  f"({self.frames / (now - self.last_report):.0f} FPS)")
            self.frames, self.last_report = 0, now


mode = next((a[2:] for a in sys.argv[1:] if a in ("--many", "--count", "--check")), "default")
Bounce(mode).run()
