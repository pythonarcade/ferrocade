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

import js
from pyodide.ffi import JsException, create_proxy

from . import color

__version__ = "0.1.0-web-spike"

__all__ = [
    "LINEAR",
    "NEAREST",
    "Filter",
    "Sprite",
    "SpriteList",
    "Texture",
    "Window",
    "color",
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

    width = property(lambda self: self._width)
    height = property(lambda self: self._height)
    title = property(lambda self: self._title)
    filter = property(lambda self: self._filter)
    update_rate = property(lambda self: self._update_rate)

    def on_draw(self):
        pass

    def on_update(self, delta_time):
        pass

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
            for _ in range(_engine.advance(elapsed)):
                self.on_update(self._update_rate)
            _engine.begin_frame()
            try:
                self.on_draw()
            finally:
                _call(_engine.end_frame)
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

    @property
    def width(self):
        return self._texture.width * abs(self.scale)

    @property
    def height(self):
        return self._texture.height * abs(self.scale)

    def __del__(self):
        engine_id = getattr(self, "_id", None)
        if engine_id is not None:
            _engine.destroy_sprite(engine_id)


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

    def draw(self):
        _call(_engine.draw_list, self._id)

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
