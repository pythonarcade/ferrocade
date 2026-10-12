"""Phase 1c: solid-color shapes, CSS named colors, and tinting.

Run from the repository root:
    python examples/01c_shapes.py            # a few shapes and a tinted ship
    python examples/01c_shapes.py --many     # 1,000 random shapes; prints the frame time
"""

import random
import sys
import time

import ferrocade as fc

WIDTH, HEIGHT = 800, 600


class Shapes(fc.Window):
    def __init__(self, many=False):
        super().__init__(WIDTH, HEIGHT, "Ferrocade shapes", filter=fc.NEAREST)
        self.sprites = fc.SpriteList()
        if many:
            self.add_many(1000)
        else:
            self.add_showcase()
        self.frames = 0
        self.last_report = None  # set on the first frame, so startup time isn't counted

    def add_showcase(self):
        add = self.sprites.append
        add(fc.RectangleSprite(600, 40, fc.color.CORNFLOWER_BLUE, 400, 60))
        add(fc.RectangleSprite(40, 300, fc.color.DARK_SLATE_GRAY, 60, 330))
        add(fc.RectangleSprite(40, 300, fc.color.DARK_SLATE_GRAY, 740, 330))
        # Same circle twice: the window filter (NEAREST, stepped edge) vs. a LINEAR override.
        add(fc.CircleSprite(60, fc.color.ORANGE, 220, 300))
        add(fc.CircleSprite(60, fc.color.ORANGE, 580, 300, filter=fc.LINEAR))
        add(fc.CircleSprite(30, (255, 255, 255, 128), 400, 420))  # semi-transparent
        ship = fc.Sprite("assets/images/ship.png", center_x=400, center_y=300)
        ship.color = fc.color.LIGHT_PINK  # tinted image sprite
        add(ship)

    def add_many(self, count):
        colors = list(fc.color.CSS_COLORS.values())
        for _ in range(count):
            x, y = random.uniform(0, WIDTH), random.uniform(0, HEIGHT)
            size = random.uniform(4, 24)
            if random.random() < 0.5:
                shape = fc.RectangleSprite(size, size * random.uniform(0.5, 2), random.choice(colors), x, y)
            else:
                shape = fc.CircleSprite(size / 2, random.choice(colors), x, y)
            self.sprites.append(shape)

    def on_draw(self):
        self.clear()
        self.sprites.draw()
        now = time.perf_counter()
        if self.last_report is None:
            self.last_report = now
            return
        self.frames += 1
        if now - self.last_report >= 2.0:
            ms = (now - self.last_report) / self.frames * 1000
            print(f"{len(self.sprites)} sprites: {ms:.2f} ms/frame ({self.frames / (now - self.last_report):.0f} FPS)")
            self.frames, self.last_report = 0, now


Shapes(many="--many" in sys.argv).run()
