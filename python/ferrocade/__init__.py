"""Ferrocade: a 2D game engine with a Python API and a Rust core."""

from . import color
from ._ferrocade import LINEAR, NEAREST, Filter, Window, __version__

__all__ = ["LINEAR", "NEAREST", "Filter", "Window", "color", "__version__"]
