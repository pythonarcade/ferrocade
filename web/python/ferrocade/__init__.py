"""SPIKE: browser implementation of the ``ferrocade`` Python API (Phase 1b, approach B).

Runs inside Pyodide and forwards every call to the Rust engine compiled to WebAssembly
(``crates/ferrocade-web``), which the page loader exposes as ``js.ferrocadeEngine``.
It mirrors the desktop API (specs/001-centered-ship/contracts/python-api.md) for what the
Phase 1 example uses; differences are listed in specs/002-web-feasibility-spike/contracts.

Sprite data lives only in the engine: properties read and write it directly, never caching
values in Python.
"""

import enum
import os
import traceback
import weakref

import js
from pyodide.ffi import JsException, create_proxy

from . import color, key

__version__ = "0.1.0-web-spike"

__all__ = [
    "LINEAR",
    "NEAREST",
    "AABBCollider",
    "BoxCollider",
    "CircleCollider",
    "CircleSprite",
    "Collider",
    "Filter",
    "FrameStats",
    "RectangleSprite",
    "Sprite",
    "SpriteList",
    "Texture",
    "Window",
    "color",
    "key",
    "__version__",
]

_engine = js.ferrocadeEngine


def _call(method, *args):
    """Call an engine method, turning engine errors into the desktop's Python exceptions."""
    try:
        return method(*args)
    except JsException as e:
        message = str(e.js_error.message) if hasattr(e, "js_error") else str(e)
        if "only be called inside on_draw" in message:
            raise RuntimeError(message) from None
        raise ValueError(message) from None


class Filter(enum.Enum):
    """Texture filtering mode. There is no default: the window requires one."""

    NEAREST = "nearest"
    LINEAR = "linear"


NEAREST = Filter.NEAREST
LINEAR = Filter.LINEAR


def _check_filter(value, allow_none):
    if value is None and allow_none:
        return ""
    if not isinstance(value, Filter):
        raise TypeError(f"filter must be a ferrocade.Filter, not {type(value).__name__}")
    return value.value


class FrameStats:
    """Frame timing averaged over the last ~1 second (draw counts are from the last frame)."""

    def __init__(self, fps, frame_ms, render_cpu_ms, draw_calls, instances, update_ms=0.0,
                 collision_ms=0.0):
        self.fps, self.frame_ms, self.render_cpu_ms = fps, frame_ms, render_cpu_ms
        self.draw_calls, self.instances = draw_calls, instances
        self.update_ms, self.collision_ms = update_ms, collision_ms


class Window:
    def __init__(self, width=800, height=600, title="Ferrocade", *, filter, update_rate=1 / 60):
        _check_filter(filter, allow_none=False)
        if width <= 0 or height <= 0:
            raise ValueError(f"window width and height must be > 0, got {width}x{height}")
        if not update_rate > 0:
            raise ValueError(f"update_rate must be > 0, got {update_rate}")
        self._width, self._height = int(width), int(height)
        self._title = title
        self._filter = filter
        self._update_rate = float(update_rate)
        self._proxy = None
        self._last = None
        self.show_fps = False
        self._held = set()  # keys held, from the engine's per-frame key events
        self._title_refreshed = 0.0

    width = property(lambda self: self._width)
    height = property(lambda self: self._height)
    @property
    def title(self):
        return self._title

    @title.setter
    def title(self, value):
        self._title = str(value)
        self._title_refreshed = 0.0  # refresh the tab title on the next frame
    filter = property(lambda self: self._filter)
    update_rate = property(lambda self: self._update_rate)

    @property
    def frame_stats(self):
        fps, frame_ms, render_cpu_ms, draw_calls, instances, update_ms, collision_ms = (
            _engine.frame_stats())
        return FrameStats(fps, frame_ms, render_cpu_ms, int(draw_calls), int(instances),
                          update_ms, collision_ms)

    def on_draw(self):
        pass

    def on_update(self, delta_time):
        pass

    def on_key_press(self, key, modifiers):
        pass

    def on_key_release(self, key, modifiers):
        pass

    def is_key_pressed(self, key):
        if not isinstance(key, int):
            raise TypeError(f"key must be an int (a ferrocade.key constant), not {type(key).__name__}")
        return key in self._held

    def _deliver_keys(self):
        # One engine call per frame: flat [code, pressed, modifiers, ...] triples.
        events = _engine.take_key_events()
        for i in range(0, len(events), 3):
            code, pressed, modifiers = events[i], events[i + 1], events[i + 2]
            if pressed:
                self._held.add(code)
                self.on_key_press(code, modifiers)
            else:
                self._held.discard(code)
                self.on_key_release(code, modifiers)

    def clear(self, color=None):
        rgba = tuple(color) if color is not None else (32, 32, 32)
        if len(rgba) not in (3, 4):
            raise ValueError(f"color must have 3 or 4 components (RGB or RGBA), got {len(rgba)}")
        r, g, b, *a = rgba
        _call(_engine.clear, r, g, b, a[0] if a else 255)

    def run(self):
        """Start the game loop on requestAnimationFrame and return immediately.

        Web difference: the browser can't block, so unlike desktop this does not wait for
        the window to close.
        """
        _call(_engine.configure, self._width, self._height, self._filter.value, self._update_rate)
        js.document.title = self._title
        self._proxy = create_proxy(self._frame)
        js.requestAnimationFrame(self._proxy)

    def _frame(self, timestamp_ms):
        try:
            now = timestamp_ms / 1000.0
            elapsed = 0.0 if self._last is None else now - self._last
            self._last = now
            self._deliver_keys()
            for _ in range(_engine.advance(elapsed)):
                self.on_update(self._update_rate)
            _engine.begin_frame()
            try:
                self.on_draw()
            finally:
                _call(_engine.end_frame)
            if now - self._title_refreshed >= 1.0:
                self._title_refreshed = now
                s = self.frame_stats
                js.document.title = (f"{self._title} — {s.fps:.0f} FPS, {s.frame_ms:.2f} ms"
                                     if self.show_fps else self._title)
        except Exception:
            self._proxy.destroy()
            self._proxy = None
            js.ferrocadeReportError(traceback.format_exc())
            return
        js.requestAnimationFrame(self._proxy)


class Texture:
    _handles = {}  # path -> engine texture handle (the engine also caches decoded data)

    def __init__(self, path, *, filter=None):
        self._filter_name = _check_filter(filter, allow_none=True)
        self._filter = filter
        self._path = os.fspath(path)
        handle = Texture._handles.get(self._path)
        if handle is None:
            with open(self._path, "rb") as f:  # FileNotFoundError names the path, as on desktop
                data = f.read()
            handle = _call(_engine.load_texture, self._path, data)
            Texture._handles[self._path] = handle
        self._handle = handle
        self._width, self._height = (int(v) for v in _engine.texture_size(handle))

    width = property(lambda self: self._width)
    height = property(lambda self: self._height)
    path = property(lambda self: self._path)
    filter = property(lambda self: self._filter)

    def __eq__(self, other):
        if not isinstance(other, Texture):
            return NotImplemented
        return (self._path, self._filter) == (other._path, other._filter)

    def __hash__(self):
        return hash((self._path, self._filter))


class Sprite:
    def __init__(self, texture, center_x=0.0, center_y=0.0, scale=1.0):
        if not isinstance(texture, Texture):
            if not isinstance(texture, (str, os.PathLike)):
                raise TypeError(
                    f"texture must be a Texture or a file path, not {type(texture).__name__}"
                )
            texture = Texture(texture)
        self._texture = texture
        self._id = _call(
            _engine.create_sprite,
            texture._handle,
            texture._filter_name,
            float(center_x),
            float(center_y),
            float(scale),
        )

    def _get(self, index):
        return _call(_engine.get_sprite, self._id)[index]

    center_x = property(lambda self: self._get(0), lambda self, v: _call(_engine.set_center_x, self._id, float(v)))
    center_y = property(lambda self: self._get(1), lambda self, v: _call(_engine.set_center_y, self._id, float(v)))
    scale = property(lambda self: self._get(2), lambda self, v: _call(_engine.set_scale, self._id, float(v)))
    angle = property(lambda self: self._get(3), lambda self, v: _call(_engine.set_angle, self._id, float(v)))

    @property
    def position(self):
        values = _call(_engine.get_sprite, self._id)
        return (values[0], values[1])

    @position.setter
    def position(self, value):
        x, y = value
        _call(_engine.set_position, self._id, float(x), float(y))

    @property
    def texture(self):
        return self._texture

    @texture.setter
    def texture(self, value):
        raise NotImplementedError("changing a sprite's texture is not available in the web spike")

    def _velocity(self, index):
        return _call(_engine.get_velocity, self._id)[index]

    @property
    def velocity_x(self):
        return self._velocity(0)

    @velocity_x.setter
    def velocity_x(self, value):
        _call(_engine.set_velocity, self._id, float(value), self._velocity(1))

    @property
    def velocity_y(self):
        return self._velocity(1)

    @velocity_y.setter
    def velocity_y(self, value):
        _call(_engine.set_velocity, self._id, self._velocity(0), float(value))

    @property
    def velocity(self):
        v = _call(_engine.get_velocity, self._id)
        return (v[0], v[1])

    @velocity.setter
    def velocity(self, value):
        vx, vy = value
        _call(_engine.set_velocity, self._id, float(vx), float(vy))

    @property
    def angular_velocity(self):
        return self._velocity(2)

    @angular_velocity.setter
    def angular_velocity(self, value):
        _call(_engine.set_angular_velocity, self._id, float(value))

    @property
    def color(self):
        return tuple(int(c) for c in _call(_engine.get_color, self._id))

    @color.setter
    def color(self, value):
        r, g, b, a = _rgba(value)
        _call(_engine.set_color, self._id, r, g, b, a)

    def _base_size(self):
        return (self._texture.width, self._texture.height)

    @property
    def width(self):
        return self._base_size()[0] * abs(self.scale)

    @property
    def height(self):
        return self._base_size()[1] * abs(self.scale)

    @property
    def collider(self):
        return getattr(self, "_collider", None)

    @collider.setter
    def collider(self, value):
        if value is not None and not isinstance(value, Collider):
            raise TypeError(f"collider must be a ferrocade.Collider or None, not {type(value).__name__}")
        _call(_engine.set_sprite_collider, self._id, -1 if value is None else value._id)
        old = getattr(self, "_collider", None)
        if old is not None and old is not value:
            old._owner = None
        if value is not None:
            value._owner = weakref.ref(self)
        self._collider = value

    def collides_with(self, other):
        return bool(_call(_engine.collides, self._id, other._id))

    def collides_with_list(self, sprites):
        slots = _call(_engine.collides_with_list, self._id, sprites._id)
        members = sprites._sprites
        return [members[i] for i in slots]

    def __del__(self):
        engine_id = getattr(self, "_id", None)
        if engine_id is not None:
            _engine.destroy_sprite(engine_id)


def _rgba(value):
    rgba = tuple(value)
    if len(rgba) not in (3, 4):
        raise ValueError(f"color must have 3 or 4 components (RGB or RGBA), got {len(rgba)}")
    if not all(isinstance(c, int) and 0 <= c <= 255 for c in rgba):
        raise ValueError(f"color components must be 0-255, got {rgba}")
    return rgba if len(rgba) == 4 else (*rgba, 255)


def _check_size(name, value):
    value = float(value)
    if not (value > 0 and value != float("inf")):  # also rejects NaN
        raise ValueError(f"shape {name} must be a finite number > 0, got {value}")
    return value


class _BuiltinShapes:
    """Stands in for the built-in shape texture (path reported like desktop)."""

    path = "<builtin:shapes>"
    filter = None


class RectangleSprite(Sprite):
    """A solid-color rectangle; no image needed."""

    def __init__(self, width, height, color, center_x=0.0, center_y=0.0):
        self._size = (_check_size("width", width), _check_size("height", height))
        r, g, b, a = _rgba(color)
        self._texture = _BuiltinShapes()
        self._id = _call(_engine.create_shape_sprite, "rectangle", self._size[0], self._size[1],
                         "", float(center_x), float(center_y), r, g, b, a)

    def _base_size(self):
        return self._size

    @property
    def texture(self):
        return self._texture

    @texture.setter
    def texture(self, value):
        raise TypeError("shape sprites use a built-in texture; it can't be replaced")


class CircleSprite(Sprite):
    """A solid-color circle; its edge follows the window filter unless `filter` overrides it."""

    def __init__(self, radius, color, center_x=0.0, center_y=0.0, *, filter=None):
        filter_name = _check_filter(filter, allow_none=True)
        diameter = 2 * _check_size("radius", radius)
        self._size = (diameter, diameter)
        r, g, b, a = _rgba(color)
        self._texture = _BuiltinShapes()
        self._id = _call(_engine.create_shape_sprite, "circle", diameter, diameter,
                         filter_name, float(center_x), float(center_y), r, g, b, a)

    def _base_size(self):
        return self._size

    @property
    def radius(self):
        return self._size[0] / 2 * abs(self.scale)

    @property
    def texture(self):
        return self._texture

    @texture.setter
    def texture(self, value):
        raise TypeError("shape sprites use a built-in texture; it can't be replaced")


class SpriteList:
    def __init__(self):
        self._id = _engine.create_list()
        self._sprites = []

    def append(self, sprite):
        _call(_engine.append, self._id, sprite._id)
        self._sprites.append(sprite)

    def remove(self, sprite):
        _call(_engine.remove, self._id, sprite._id)
        self._sprites.remove(sprite)

    def update(self, delta_time):
        _call(_engine.update_list, self._id, float(delta_time))

    def draw(self):
        _call(_engine.draw_list, self._id)

    def collisions_with(self, others):
        flat = _call(_engine.collisions_with, self._id, others._id)
        a, b = self._sprites, others._sprites
        return [(a[flat[k]], b[flat[k + 1]]) for k in range(0, len(flat), 2)]

    def bounce_each_other(self):
        return int(_call(_engine.bounce_each_other, self._id))

    def bounce_off(self, walls):
        return int(_call(_engine.bounce_off, self._id, walls._id))

    def collisions(self):
        flat = _call(_engine.collisions, self._id)
        s = self._sprites
        return [(s[flat[k]], s[flat[k + 1]]) for k in range(0, len(flat), 2)]

    def __len__(self):
        return len(self._sprites)

    def __contains__(self, sprite):
        return any(s is sprite for s in self._sprites)

    def __iter__(self):
        return iter(list(self._sprites))

    def __del__(self):
        engine_id = getattr(self, "_id", None)
        if engine_id is not None:
            _engine.destroy_list(engine_id)


class Collider:
    """Base class of the colliders. Sizes and offsets are in the sprite's unscaled pixels."""

    _kind = None

    def _create(self, a, b, offset):
        ox, oy = offset
        self._owner = None
        self._id = _call(_engine.create_collider, self._kind, float(a), float(b), float(ox), float(oy))
        self._a, self._b, self._offset = float(a), float(b), (float(ox), float(oy))

    @property
    def offset(self):
        return self._offset

    @offset.setter
    def offset(self, value):
        x, y = value
        _call(_engine.set_collider_offset, self._id, float(x), float(y))
        self._offset = (float(x), float(y))

    @property
    def sprite(self):
        return self._owner() if self._owner is not None else None

    def _set_size(self, a, b):
        _call(_engine.set_collider_shape, self._id, self._kind, float(a), float(b))
        self._a, self._b = float(a), float(b)

    def __del__(self):
        engine_id = getattr(self, "_id", None)
        if engine_id is not None:
            _engine.destroy_collider(engine_id)


class CircleCollider(Collider):
    """A circle collider; its offset rotates with the sprite."""

    _kind = "circle"

    def __init__(self, radius, *, offset=(0.0, 0.0)):
        self._create(radius, 0.0, offset)

    @property
    def radius(self):
        return self._a

    @radius.setter
    def radius(self, value):
        self._set_size(value, 0.0)

    def __repr__(self):
        return f"CircleCollider(radius={self._a:g})"


class _BoxBase(Collider):
    def __init__(self, width, height, *, offset=(0.0, 0.0)):
        self._create(width, height, offset)

    @property
    def width(self):
        return self._a

    @width.setter
    def width(self, value):
        self._set_size(value, self._b)

    @property
    def height(self):
        return self._b

    @height.setter
    def height(self, value):
        self._set_size(self._a, value)

    def __repr__(self):
        return f"{type(self).__name__}(width={self._a:g}, height={self._b:g})"


class BoxCollider(_BoxBase):
    """A box collider that rotates with its sprite (its offset rotates too)."""

    _kind = "box"


class AABBCollider(_BoxBase):
    """An axis-aligned box collider: it never rotates, and neither does its offset."""

    _kind = "aabb"
