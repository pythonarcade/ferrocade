import math

import pytest

import ferrocade as fc

SHIP = "assets/images/ship.png"


def test_from_path_equals_from_texture():
    a = fc.Sprite(SHIP)
    b = fc.Sprite(fc.Texture(SHIP))
    assert a.texture == b.texture
    assert (a.width, a.height) == (b.width, b.height) == (99.0, 75.0)


def test_defaults():
    s = fc.Sprite(SHIP)
    assert (s.center_x, s.center_y, s.scale, s.angle) == (0.0, 0.0, 1.0, 0.0)


def test_constructor_values():
    s = fc.Sprite(SHIP, center_x=400, center_y=300, scale=0.5)
    assert s.position == (400.0, 300.0)
    assert (s.width, s.height) == (49.5, 37.5)


def test_values_round_trip_exactly():
    s = fc.Sprite(SHIP)
    s.center_x = 0.1
    s.center_y = 1 / 3
    s.angle = 12.345
    assert s.center_x == 0.1
    assert s.center_y == 1 / 3
    assert s.angle == 12.345


def test_position_sets_both():
    s = fc.Sprite(SHIP)
    s.position = (10.5, 20.25)
    assert (s.center_x, s.center_y) == (10.5, 20.25)


def test_scale_changes_size():
    s = fc.Sprite(SHIP)
    s.scale = 2.0
    assert (s.width, s.height) == (198.0, 150.0)
    s.scale = -1.0
    assert s.width == 99.0


@pytest.mark.parametrize("value", [math.nan, math.inf, -math.inf])
def test_non_finite_values_rejected(value):
    with pytest.raises(ValueError):
        fc.Sprite(SHIP, center_x=value)
    s = fc.Sprite(SHIP)
    for name in ("center_x", "center_y", "scale", "angle"):
        with pytest.raises(ValueError):
            setattr(s, name, value)


def test_texture_reassignment(tmp_path, make_png):
    s = fc.Sprite(SHIP, scale=2.0)
    s.texture = fc.Texture(make_png(tmp_path / "small.png", 10, 4))
    assert (s.texture.width, s.texture.height) == (10, 4)
    assert (s.width, s.height) == (20.0, 8.0)


def test_texture_must_be_texture_or_path():
    with pytest.raises(TypeError):
        fc.Sprite(42)
    s = fc.Sprite(SHIP)
    with pytest.raises(TypeError):
        s.texture = "assets/images/ship.png"


def test_subclass():
    class Ship(fc.Sprite):
        def __init__(self):
            super().__init__(SHIP, 400, 300)
            self.lives = 3

    ship = Ship()
    assert ship.position == (400.0, 300.0)
    assert ship.lives == 3


def test_subclass_without_super_init_gives_clear_error():
    class Broken(fc.Sprite):
        def __init__(self):
            pass

    with pytest.raises(RuntimeError, match="super"):
        Broken().center_x


def test_color_defaults_to_white():
    assert fc.Sprite(SHIP).color == (255, 255, 255, 255)


def test_color_set_and_round_trip():
    s = fc.Sprite(SHIP)
    s.color = (255, 0, 0)
    assert s.color == (255, 0, 0, 255)
    s.color = (10, 20, 30, 40)
    assert s.color == (10, 20, 30, 40)
    s.color = fc.color.LIGHT_PINK
    assert s.color == (*fc.color.LIGHT_PINK, 255)


@pytest.mark.parametrize("bad", [(1, 2), (256, 0, 0), (-1, 0, 0)])
def test_color_validation(bad):
    with pytest.raises(ValueError):
        fc.Sprite(SHIP).color = bad


def test_shape_color_can_change():
    r = fc.RectangleSprite(10, 10, fc.color.RED)
    r.color = fc.color.GREEN
    assert r.color == (0, 128, 0, 255)
