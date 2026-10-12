import math

import pytest

import ferrocade as fc

SHIP = "assets/images/ship.png"


def test_rectangle_basics():
    r = fc.RectangleSprite(50, 20, fc.color.RED, 100, 100)
    assert isinstance(r, fc.Sprite)
    assert (r.width, r.height) == (50.0, 20.0)
    assert r.position == (100.0, 100.0)
    assert r.color == (255, 0, 0, 255)


def test_circle_basics():
    c = fc.CircleSprite(15, fc.color.BLUE)
    assert isinstance(c, fc.Sprite)
    assert (c.width, c.height, c.radius) == (30.0, 30.0, 15.0)
    assert c.position == (0.0, 0.0)


def test_scale_resizes_shapes():
    r = fc.RectangleSprite(50, 20, fc.color.RED)
    c = fc.CircleSprite(15, fc.color.BLUE)
    r.scale = c.scale = 2
    assert (r.width, r.height) == (100.0, 40.0)
    assert c.radius == 30.0


def test_rgba_color():
    r = fc.RectangleSprite(10, 10, (1, 2, 3, 128))
    assert r.color == (1, 2, 3, 128)


@pytest.mark.parametrize("size", [0, -1, math.nan, math.inf])
def test_invalid_sizes(size):
    with pytest.raises(ValueError):
        fc.RectangleSprite(size, 10, fc.color.RED)
    with pytest.raises(ValueError):
        fc.RectangleSprite(10, size, fc.color.RED)
    with pytest.raises(ValueError):
        fc.CircleSprite(size, fc.color.RED)


@pytest.mark.parametrize("color", [(1, 2), (256, 0, 0), (-1, 0, 0), (1, 2, 3, 4, 5)])
def test_invalid_colors(color):
    with pytest.raises(ValueError):
        fc.RectangleSprite(10, 10, color)
    with pytest.raises(ValueError):
        fc.CircleSprite(10, color)


def test_circle_filter_override():
    fc.CircleSprite(10, fc.color.RED, filter=fc.LINEAR)
    fc.CircleSprite(10, fc.color.RED, filter=fc.NEAREST)
    with pytest.raises(TypeError):
        fc.CircleSprite(10, fc.color.RED, filter="linear")


def test_shape_texture_is_builtin_and_fixed():
    r = fc.RectangleSprite(10, 10, fc.color.RED)
    assert r.texture.path == "<builtin:shapes>"
    with pytest.raises(TypeError, match="built-in texture"):
        r.texture = fc.Texture(SHIP)


def test_subclassing():
    class Wall(fc.RectangleSprite):
        def __init__(self):
            super().__init__(50, 20, fc.color.DARK_SLATE_GRAY, 25, 10)
            self.solid = True

    class Ball(fc.CircleSprite):
        def __init__(self, x):
            super().__init__(8, fc.color.ORANGE, x, 100, filter=fc.LINEAR)
            self.speed = 3

    wall, ball = Wall(), Ball(40)
    assert wall.solid and wall.width == 50.0
    assert ball.speed == 3 and ball.radius == 8.0 and ball.center_x == 40.0


def test_shapes_in_sprite_lists_with_images():
    lst = fc.SpriteList()
    r = fc.RectangleSprite(10, 10, fc.color.RED)
    c = fc.CircleSprite(5, fc.color.BLUE)
    ship = fc.Sprite(SHIP)
    for s in (r, ship, c):
        lst.append(s)
    assert list(lst) == [r, ship, c]
    assert list(lst)[0] is r
    lst.remove(ship)
    assert list(lst) == [r, c]


class StopTest(Exception):
    pass


@pytest.mark.display
def test_shapes_draw_inside_on_draw():
    class Game(fc.Window):
        def __init__(self):
            super().__init__(320, 240, "shapes", filter=fc.NEAREST)
            self.shapes = fc.SpriteList()
            self.shapes.append(fc.RectangleSprite(100, 20, fc.color.CORNFLOWER_BLUE, 160, 40))
            self.shapes.append(fc.CircleSprite(30, fc.color.ORANGE, 160, 140))

        def on_draw(self):
            self.clear()
            self.shapes.draw()
            raise StopTest

    with pytest.raises(StopTest):
        Game().run()
