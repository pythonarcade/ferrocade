"""Phase 5: fly the ship through an asteroid field; asteroids it touches disappear.

Controls: arrow keys or WASD move the ship, and Space returns it to the center. The window
title shows how many asteroids you've hit.

Asteroid colors show their collider type:
- gold circles: CircleCollider
- tomato squares: BoxCollider (rotates with the sprite)
- light blue squares: AABBCollider (stays upright even while the sprite spins)

Colliders are attached explicitly, and the engine does the detection: one
`collides_with_list` call per update.

Run from the repository root:
    python examples/05_collisions.py
In the browser: http://localhost:8000/web/?example=05_collisions (click the game first).
"""

import math
import random

import ferrocade as fc

WIDTH, HEIGHT = 800, 600
SPEED = 300  # ship speed, pixels per second
ROCKS = 150

K = fc.key
C = fc.color


class Collisions(fc.Window):
    def __init__(self):
        super().__init__(WIDTH, HEIGHT, "Collisions — hits: 0", filter=fc.NEAREST)
        self.ship = fc.Sprite("assets/images/ship.png", WIDTH / 2, HEIGHT / 2)
        # The ship's image has transparent corners, so its box is a little smaller.
        w, h = self.ship.width / self.ship.scale, self.ship.height / self.ship.scale
        self.ship.collider = fc.BoxCollider(w * 0.8, h * 0.8)
        self.ships = fc.SpriteList()
        self.ships.append(self.ship)
        self.rocks = fc.SpriteList()
        self.hits = 0
        self.add_rocks()

    def add_rocks(self):
        rng = random.Random(5)  # the same field every run
        for i in range(ROCKS):
            size = rng.uniform(10, 26)
            x, y = rng.uniform(0, WIDTH), rng.uniform(0, HEIGHT)
            if math.hypot(x - WIDTH / 2, y - HEIGHT / 2) < 90:
                x = (x + WIDTH / 2) % WIDTH  # keep the start area clear
            kind = i % 3
            if kind == 0:
                rock = fc.CircleSprite(size / 2, C.GOLD, x, y)
                rock.collider = fc.CircleCollider(size / 2)
            elif kind == 1:
                rock = fc.RectangleSprite(size, size, C.TOMATO, x, y)
                rock.collider = fc.BoxCollider(size, size)
            else:
                rock = fc.RectangleSprite(size, size, C.LIGHT_SKY_BLUE, x, y)
                rock.collider = fc.AABBCollider(size, size)
            rock.velocity = (rng.uniform(-40, 40), rng.uniform(-40, 40))
            rock.angular_velocity = rng.uniform(-60, 60)
            self.rocks.append(rock)

    def held(self, *keys):
        return any(self.is_key_pressed(k) for k in keys)

    def on_update(self, delta_time):
        # Steer (as in the Phase 4 example).
        dx = self.held(K.RIGHT, K.D) - self.held(K.LEFT, K.A)
        dy = self.held(K.UP, K.W) - self.held(K.DOWN, K.S)
        length = math.hypot(dx, dy)
        if length:
            dx, dy = dx / length, dy / length
        self.ship.velocity = (dx * SPEED, dy * SPEED)
        self.ships.update(delta_time)
        half_w, half_h = self.ship.width / 2, self.ship.height / 2
        x, y = self.ship.position
        clamped = (min(max(x, half_w), WIDTH - half_w), min(max(y, half_h), HEIGHT - half_h))
        if clamped != (x, y):
            self.ship.position = clamped

        # Drift the asteroids, wrapping at the edges.
        self.rocks.update(delta_time)
        for rock in self.rocks:
            rx, ry = rock.position
            if not (-20 <= rx <= WIDTH + 20 and -20 <= ry <= HEIGHT + 20):
                rock.position = (rx % WIDTH, ry % HEIGHT)

        # Collisions: one engine query; remove what the ship touched.
        hit = self.ship.collides_with_list(self.rocks)
        for rock in hit:
            self.rocks.remove(rock)
        if hit:
            self.hits += len(hit)
            self.title = f"Collisions — hits: {self.hits}"

    def on_key_press(self, key, modifiers):
        if key == K.SPACE:
            self.ship.position = (WIDTH / 2, HEIGHT / 2)

    def on_draw(self):
        self.clear()
        self.rocks.draw()
        self.ships.draw()


if __name__ == "__main__":
    Collisions().run()
