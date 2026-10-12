"""Phase 1 milestone: open a window and draw the ship in the center.

Run from the repository root:  python examples/01_centered_ship.py
"""

import ferrocade as fc


class Game(fc.Window):
    def __init__(self):
        super().__init__(800, 600, "Ferrocade", filter=fc.NEAREST)
        self.sprites = fc.SpriteList()
        ship = fc.Sprite("assets/images/ship.png",
                         center_x=self.width / 2, center_y=self.height / 2)
        self.sprites.append(ship)

    def on_draw(self):
        self.clear()
        self.sprites.draw()


Game().run()
