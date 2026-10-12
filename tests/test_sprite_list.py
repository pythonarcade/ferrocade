import gc
import weakref

import pytest

import ferrocade as fc

SHIP = "assets/images/ship.png"


def sprites(n):
    return [fc.Sprite(SHIP, center_x=i) for i in range(n)]


def test_append_len_contains():
    lst = fc.SpriteList()
    a, b = sprites(2)
    lst.append(a)
    assert len(lst) == 1
    assert a in lst
    assert b not in lst


def test_append_twice_rejected():
    lst = fc.SpriteList()
    (a,) = sprites(1)
    lst.append(a)
    with pytest.raises(ValueError):
        lst.append(a)


def test_remove_keeps_order():
    lst = fc.SpriteList()
    a, b, c = sprites(3)
    for s in (a, b, c):
        lst.append(s)
    lst.remove(b)
    assert list(lst) == [a, c]
    assert [s.center_x for s in lst] == [0.0, 2.0]
    with pytest.raises(ValueError):
        lst.remove(b)


def test_iteration_yields_same_objects_including_subclasses():
    class Ship(fc.Sprite):
        pass

    lst = fc.SpriteList()
    plain, sub = fc.Sprite(SHIP), Ship(SHIP)
    lst.append(plain)
    lst.append(sub)
    items = list(lst)
    assert items[0] is plain
    assert items[1] is sub


def test_sprite_in_two_lists():
    first, second = fc.SpriteList(), fc.SpriteList()
    (a,) = sprites(1)
    first.append(a)
    second.append(a)
    assert a in first and a in second
    first.remove(a)
    assert a not in first and a in second


def test_list_keeps_sprite_alive():
    lst = fc.SpriteList()
    s = fc.Sprite(SHIP)
    ref = weakref.ref(s)
    lst.append(s)
    del s
    gc.collect()
    assert ref() is not None
    lst.remove(ref())
    gc.collect()
    assert ref() is None


def test_draw_outside_on_draw_raises():
    with pytest.raises(RuntimeError, match="on_draw"):
        fc.SpriteList().draw()


class StopTest(Exception):
    pass


@pytest.mark.display
def test_draw_inside_on_draw():
    class Game(fc.Window):
        def __init__(self):
            super().__init__(320, 240, "lists", filter=fc.NEAREST)
            self.empty = fc.SpriteList()
            self.full = fc.SpriteList()
            for i in range(10):
                self.full.append(fc.Sprite(SHIP, center_x=30 * i, center_y=120))
            self.frames = 0

        def on_draw(self):
            self.clear()
            self.empty.draw()
            self.full.draw()
            # Move a sprite between frames: exercises dirty uploads.
            next(iter(self.full)).center_x += 1
            self.frames += 1
            if self.frames == 10:
                raise StopTest

    with pytest.raises(StopTest):
        Game().run()
