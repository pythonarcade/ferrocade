"""Collision detection: colliders, sprite queries, and pair queries (no display needed)."""

import gc
import math
import random
import weakref

import pytest

import ferrocade as fc

RED = (255, 0, 0)


def rect(x=0.0, y=0.0, w=10, h=10):
    return fc.RectangleSprite(w, h, RED, x, y)


def with_collider(sprite, collider):
    sprite.collider = collider
    return sprite


def circle_at(x, y, r=10.0):
    return with_collider(rect(x, y), fc.CircleCollider(r))


def sprite_list(*sprites):
    lst = fc.SpriteList()
    for s in sprites:
        lst.append(s)
    return lst


# --- US1: colliders -----------------------------------------------------------------------


def test_construct_read_and_write():
    c = fc.CircleCollider(5, offset=(1, 2))
    assert c.radius == 5 and c.offset == (1, 2) and c.sprite is None
    c.radius = 7.5
    c.offset = (-3, 4)
    assert (c.radius, c.offset) == (7.5, (-3, 4))
    for cls in (fc.BoxCollider, fc.AABBCollider):
        b = cls(20, 10)
        assert (b.width, b.height, b.offset) == (20, 10, (0, 0))
        b.width, b.height = 4, 6
        assert (b.width, b.height) == (4, 6)
        assert isinstance(b, fc.Collider)
    assert repr(fc.CircleCollider(3)) == "CircleCollider(radius=3)"
    assert repr(fc.AABBCollider(2, 3)) == "AABBCollider(width=2, height=3)"


@pytest.mark.parametrize("bad", [-1.0, math.nan, math.inf])
def test_invalid_sizes_raise(bad):
    with pytest.raises(ValueError):
        fc.CircleCollider(bad)
    with pytest.raises(ValueError):
        fc.BoxCollider(bad, 1)
    with pytest.raises(ValueError):
        fc.AABBCollider(1, bad)
    c = fc.CircleCollider(1)
    with pytest.raises(ValueError):
        c.radius = bad
    if not (bad == -1.0):  # negative offsets are fine; non-finite ones aren't
        with pytest.raises(ValueError):
            c.offset = (bad, 0)


def test_zero_size_is_allowed():
    point = with_collider(rect(5, 5), fc.CircleCollider(0))
    box = with_collider(rect(0, 0), fc.AABBCollider(10, 10))
    assert point.collides_with(box)  # on the corner


def test_attach_replace_detach():
    s = rect()
    assert s.collider is None, "no automatic colliders"
    assert fc.CircleSprite(5, RED).collider is None, "shape sprites included"
    first = fc.CircleCollider(3)
    s.collider = first
    assert s.collider is first and first.sprite is s
    second = fc.BoxCollider(4, 4)
    s.collider = second
    assert first.sprite is None and second.sprite is s
    s.collider = None
    assert s.collider is None and second.sprite is None


def test_one_sprite_at_a_time():
    c = fc.CircleCollider(3)
    a, b = rect(), rect()
    a.collider = c
    with pytest.raises(ValueError, match="already attached"):
        b.collider = c
    a.collider = c  # re-assigning to the same sprite is fine
    del a
    gc.collect()
    b.collider = c  # freed when its sprite was deleted
    assert c.sprite is b


def test_no_leak_between_sprite_and_collider():
    s = rect()
    c = fc.CircleCollider(3)
    s.collider = c
    ref_s, ref_c = weakref.ref(s), weakref.ref(c)
    del s, c
    gc.collect()
    assert ref_s() is None and ref_c() is None


def test_rotation_scale_and_offset_rules():
    # Spec US1 scenarios 2-5.
    ship = with_collider(rect(100, 100), fc.CircleCollider(2, offset=(20, 0)))
    probe = with_collider(rect(100, 120), fc.CircleCollider(0))
    assert not ship.collides_with(probe)
    ship.angle = 90  # the offset rotates counter-clockwise: now above the center
    assert ship.collides_with(probe)
    ship.collider = fc.AABBCollider(4, 4, offset=(20, 0))
    assert not ship.collides_with(probe)  # AABB offsets don't rotate
    probe.position = (120, 100)
    assert ship.collides_with(probe)
    ship.angle = 0
    ship.scale = 2  # sizes and offsets scale: offset 40, half size 4
    assert not ship.collides_with(probe)
    probe.position = (144, 100)
    assert ship.collides_with(probe)

    # A rotating box: a probe near the unrotated corner misses after a 45° turn.
    box = with_collider(rect(0, 0), fc.BoxCollider(20, 20))
    corner = with_collider(rect(9.5, 9.5), fc.CircleCollider(0))
    tip = with_collider(rect(14, 0), fc.CircleCollider(0))
    assert box.collides_with(corner) and not box.collides_with(tip)
    box.angle = 45
    assert not box.collides_with(corner) and box.collides_with(tip)
    aabb = with_collider(rect(0, 0), fc.AABBCollider(20, 20))
    aabb.angle = 45
    assert aabb.collides_with(corner), "an AABBCollider stays axis-aligned"


def test_circle_touching_counts():
    # Spec US1 scenario 1.
    a = circle_at(100, 100)
    assert a.collides_with(circle_at(119, 100))
    assert a.collides_with(circle_at(120, 100))  # touching
    assert not a.collides_with(circle_at(121, 100))


# --- US2: sprite queries ------------------------------------------------------------------


def test_collides_with_basics():
    a, b = circle_at(0, 0), circle_at(5, 0)
    assert a.collides_with(b) and b.collides_with(a)
    assert not a.collides_with(a)
    b.collider = None
    assert not a.collides_with(b), "sprites without colliders never collide"


def test_ship_among_1000_asteroids():
    rng = random.Random(1)
    rocks = fc.SpriteList()
    expected = []
    for i in range(1000):
        x, y = rng.uniform(300, 2000), rng.uniform(300, 2000)  # far from the ship
        if i in (100, 500, 900):
            x, y = 100 + i / 100, 100
            expected.append(i)
        rocks.append(circle_at(x, y, 4))
    ship = with_collider(rect(100, 100), fc.BoxCollider(30, 30))
    members = list(rocks)
    assert ship.collides_with_list(rocks) == [members[i] for i in expected]


def test_sprite_in_its_own_list_is_not_reported():
    a, b = circle_at(0, 0), circle_at(5, 0)
    lst = sprite_list(a, b)
    assert a.collides_with_list(lst) == [b]


def test_results_are_snapshots_and_follow_moves():
    ship = circle_at(0, 0)
    rocks = sprite_list(*[circle_at(i * 3, 0, 1) for i in range(5)])
    hits = ship.collides_with_list(rocks)
    assert len(hits) == 4  # x = 0, 3, 6, 9 are within 11
    for rock in hits:
        rocks.remove(rock)  # safe while looping over the result
    assert len(rocks) == 1
    remaining = list(rocks)[0]
    assert ship.collides_with_list(rocks) == []
    remaining.position = (0, 0)
    assert ship.collides_with_list(rocks) == [remaining]
    # Changing a collider takes effect for the next query.
    ship.collider.radius = 0.5
    remaining.collider.radius = 0.5
    remaining.position = (5, 0)
    assert ship.collides_with_list(rocks) == []


def test_large_lists_use_the_grid_correctly():
    rng = random.Random(3)
    sprites = [with_collider(rect(rng.uniform(0, 400), rng.uniform(0, 400)),
                             fc.CircleCollider(rng.uniform(1, 6))) for _ in range(500)]
    lst = sprite_list(*sprites)
    probe = with_collider(rect(200, 200), fc.CircleCollider(30))
    assert probe.collides_with_list(lst) == [s for s in sprites if probe.collides_with(s)]


# --- US3: pair queries --------------------------------------------------------------------


def random_scene(rng, n, area=300.0):
    sprites = []
    for i in range(n):
        s = rect(rng.uniform(0, area), rng.uniform(0, area))
        s.angle = rng.uniform(0, 360)
        s.scale = rng.uniform(0.3, 2.5)
        offset = (rng.uniform(-4, 4), rng.uniform(-4, 4))
        kind = i % 4
        if kind == 0:
            s.collider = fc.CircleCollider(rng.uniform(1, 10), offset=offset)
        elif kind == 1:
            s.collider = fc.BoxCollider(rng.uniform(1, 20), rng.uniform(1, 20), offset=offset)
        elif kind == 2:
            s.collider = fc.AABBCollider(rng.uniform(1, 20), rng.uniform(1, 20), offset=offset)
        # kind 3: no collider
        sprites.append(s)
    return sprites


def brute_pairs(a, b, same_list):
    out = []
    for i, x in enumerate(a):
        for j, y in enumerate(b):
            if (j > i if same_list else x is not y) and x.collides_with(y):
                out.append((x, y))
    return out


def test_bullets_vs_asteroids():
    bullets = sprite_list(circle_at(0, 0, 1), circle_at(50, 0, 1), circle_at(200, 0, 1))
    rocks = sprite_list(circle_at(5, 0, 5), circle_at(52, 0, 5), circle_at(48, 0, 5))
    b, r = list(bullets), list(rocks)
    assert bullets.collisions_with(rocks) == [(b[0], r[0]), (b[1], r[1]), (b[1], r[2])]


def test_pairs_within_a_list_once_each():
    a, b, c = circle_at(0, 0), circle_at(5, 0), circle_at(100, 0)
    lst = sprite_list(a, b, c)
    assert lst.collisions() == [(a, b)]


def test_pair_queries_match_brute_force_and_are_deterministic():
    rng = random.Random(11)
    sprites = random_scene(rng, 300)
    lst = sprite_list(*sprites)
    expected = brute_pairs(sprites, sprites, same_list=True)
    assert expected, "the scene should have overlaps"
    assert lst.collisions() == expected
    assert lst.collisions() == expected, "same order on repeat"

    others = random_scene(rng, 120)
    shared = sprites[::10]
    other_list = sprite_list(*others, *shared)
    combined = others + shared
    assert lst.collisions_with(other_list) == brute_pairs(sprites, combined, same_list=False)


def test_empty_results():
    assert fc.SpriteList().collisions() == []
    lst = sprite_list(rect(), rect())  # no colliders
    assert lst.collisions() == []
    assert circle_at(0, 0).collides_with_list(lst) == []


# --- US4: the Phase 5 example (needs a display) --------------------------------------------


class StopTest(Exception):
    pass


@pytest.mark.display
def test_example_ship_hits_asteroids(monkeypatch):
    import importlib.util
    from pathlib import Path

    root = Path(__file__).resolve().parent.parent
    monkeypatch.chdir(root)
    spec = importlib.util.spec_from_file_location("collisions_example",
                                                  root / "examples" / "05_collisions.py")
    example = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(example)

    class Driven(example.Collisions):
        n = 0

        def on_update(self, delta_time):
            if self.n == 2:
                self._inject_key(fc.key.RIGHT, True)
            if self.n == 62:
                self._inject_key(fc.key.RIGHT, False)
                self._inject_key(fc.key.UP, True)
            super().on_update(delta_time)
            self.n += 1
            if self.n > 130:
                raise StopTest

    w = Driven()
    start = len(w.rocks)
    with pytest.raises(StopTest):
        w.run()
    assert w.hits > 0, "flying right then up through the field should hit something"
    assert len(w.rocks) == start - w.hits
    assert w.title == f"Collisions — hits: {w.hits}"
