"""Phase 4: steer the ship with the keyboard.

Controls: arrow keys or WASD move the ship (diagonals keep the same speed), and Space returns it
to the center. Keys are physical positions, so WASD works on any keyboard layout.

Movement uses polling (`is_key_pressed` in `on_update`); re-centering uses a key callback
(`on_key_press`). The engine moves the ship; Python only picks its velocity.

Run from the repository root:
    python examples/04_keyboard_ship.py
In the browser: http://localhost:8000/web/?example=04_keyboard_ship (click the game first).
"""

import math

import ferrocade as fc

WIDTH, HEIGHT = 800, 600
SPEED = 300  # pixels per second

K = fc.key


class KeyboardShip(fc.Window):
    def __init__(self):
        super().__init__(WIDTH, HEIGHT, "Keyboard ship", filter=fc.NEAREST)
        self.ship = fc.Sprite("assets/images/ship.png", WIDTH / 2, HEIGHT / 2)
        self.ships = fc.SpriteList()
        self.ships.append(self.ship)

    def held(self, *keys):
        return any(self.is_key_pressed(k) for k in keys)

    def on_update(self, delta_time):
        dx = self.held(K.RIGHT, K.D) - self.held(K.LEFT, K.A)
        dy = self.held(K.UP, K.W) - self.held(K.DOWN, K.S)
        length = math.hypot(dx, dy)
        if length:
            dx, dy = dx / length, dy / length
        self.ship.velocity = (dx * SPEED, dy * SPEED)
        self.ships.update(delta_time)

        # Keep the whole ship inside the window.
        half_w, half_h = self.ship.width / 2, self.ship.height / 2
        x, y = self.ship.position
        clamped = (min(max(x, half_w), self.width - half_w),
                   min(max(y, half_h), self.height - half_h))
        if clamped != (x, y):
            self.ship.position = clamped

    def on_key_press(self, key, modifiers):
        if key == K.SPACE:
            self.ship.position = (self.width / 2, self.height / 2)

    def on_draw(self):
        self.clear()
        self.ships.draw()


if __name__ == "__main__":
    KeyboardShip().run()
