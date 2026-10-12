"""Ferrocade: a 2D game engine with a Python API and a Rust core."""

from . import color, key
from ._ferrocade import (
    LINEAR,
    NEAREST,
    AABBCollider,
    BoxCollider,
    CircleCollider,
    CircleSprite,
    Collider,
    Filter,
    FrameStats,
    RectangleSprite,
    Sprite,
    SpriteList,
    Texture,
    Window,
    __version__,
)

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
