import os
from collections.abc import Iterator
from typing import Final, final

__version__: str

@final
class Filter:
    """Texture filtering mode. There is no default: the window requires one."""

    NEAREST: Final[Filter]
    """Crisp pixels with no smoothing (pixel art)."""
    LINEAR: Final[Filter]
    """Smooth interpolation (photos, scaled or rotated art)."""

NEAREST: Final[Filter]
LINEAR: Final[Filter]

Color = tuple[int, int, int] | tuple[int, int, int, int]

@final
class FrameStats:
    """Frame timing averaged over the last ~1 second (draw counts are from the last frame)."""

    @property
    def fps(self) -> float: ...
    @property
    def frame_ms(self) -> float:
        """Average time between frames, in milliseconds."""
    @property
    def render_cpu_ms(self) -> float:
        """Average engine render CPU time per frame, excluding the wait for vsync."""
    @property
    def draw_calls(self) -> int: ...
    @property
    def instances(self) -> int: ...
    @property
    def update_ms(self) -> float:
        """Average engine time per frame moving sprites (`SpriteList.update`), in ms."""
    @property
    def collision_ms(self) -> float:
        """Average engine time per frame in collision queries and bounce calls, in ms."""

class Window:
    """A game window. Subclass it and override `on_draw` / `on_update`, then call `run()`."""

    def __init__(
        self,
        width: int = 800,
        height: int = 600,
        title: str = "Ferrocade",
        *,
        filter: Filter,
        update_rate: float = 1 / 60,
    ) -> None:
        """`filter` is required: choose `NEAREST` (pixel art) or `LINEAR` (smooth)."""

    @property
    def width(self) -> int:
        """Width in logical pixels (updates when the window is resized)."""
    @property
    def height(self) -> int:
        """Height in logical pixels (updates when the window is resized)."""
    title: str
    """Window title; assign to change it (the FPS readout is appended when `show_fps` is on)."""
    @property
    def filter(self) -> Filter:
        """Filter used by textures that don't set their own."""
    @property
    def update_rate(self) -> float:
        """Seconds between `on_update` calls."""
    show_fps: bool
    """Show FPS and frame time in the window title (refreshed about once per second)."""
    @property
    def frame_stats(self) -> FrameStats:
        """Frame timing of the running window (all zeros before `run()`)."""
    def run(self) -> None:
        """Open the window and run until it closes. Re-raises exceptions from callbacks."""
    def clear(self, color: Color | None = None) -> None:
        """Fill the window with `color` (default dark gray). Only valid inside `on_draw`."""
    def on_draw(self) -> None:
        """Called once per displayed frame. Override to draw."""
    def on_update(self, delta_time: float) -> None:
        """Called at a fixed rate; `delta_time` equals `update_rate`. Override to update."""
    def on_key_press(self, key: int, modifiers: int) -> None:
        """Called once when a key goes down; operating-system auto-repeat is ignored.

        `key` is a `ferrocade.key` constant (a physical key position). `modifiers` holds the
        `key.MOD_*` flags held after the event. Delivered before the frame's `on_update` calls.
        """
    def on_key_release(self, key: int, modifiers: int) -> None:
        """Called once when a key goes up. Held keys are released when the window loses focus."""
    def is_key_pressed(self, key: int) -> bool:
        """Whether `key` is held, as of the key events delivered so far."""
    def _inject_key(self, key: int, pressed: bool, repeat: bool = False) -> None:
        """Private (tests): queue a key event as if the platform had sent it."""
    def _inject_focus_lost(self) -> None:
        """Private (tests): act as if the window lost focus."""

@final
class Texture:
    """A loaded image. Loading the same file again shares the same image data."""

    def __init__(self, path: str | os.PathLike[str], *, filter: Filter | None = None) -> None:
        """`filter` overrides the window's filter for this texture; `None` uses the window's."""

    @property
    def width(self) -> int:
        """Width in pixels."""
    @property
    def height(self) -> int:
        """Height in pixels."""
    @property
    def path(self) -> str:
        """Canonical absolute path of the image file."""
    @property
    def filter(self) -> Filter | None:
        """Filter override, or `None` to use the window's filter."""
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class Sprite:
    """An image placed in the window. Coordinates are pixels, origin bottom-left, y up."""

    def __init__(
        self,
        texture: Texture | str | os.PathLike[str],
        center_x: float = 0.0,
        center_y: float = 0.0,
        scale: float = 1.0,
    ) -> None: ...

    center_x: float
    center_y: float
    position: tuple[float, float]
    """`(center_x, center_y)`; setting it updates both in one call."""
    scale: float
    angle: float
    """Rotation in degrees (stored; rotation is not drawn yet)."""
    texture: Texture
    """Shape sprites use a built-in texture; assigning one raises TypeError."""
    velocity_x: float
    """Pixels per second (+x is right). Applied by `SpriteList.update`."""
    velocity_y: float
    """Pixels per second (+y is up). Applied by `SpriteList.update`."""
    velocity: tuple[float, float]
    """`(velocity_x, velocity_y)`; setting it updates both in one call."""
    angular_velocity: float
    """Degrees per second; positive is counter-clockwise. Applied by `SpriteList.update`."""
    color: tuple[int, int, int, int]
    """Tint (r, g, b, a), 0-255; accepts 3- or 4-tuples. White draws the texture unchanged."""
    @property
    def width(self) -> float:
        """Drawn width in pixels: base size (or texture width) × |scale|."""
    @property
    def height(self) -> float:
        """Drawn height in pixels: base size (or texture height) × |scale|."""
    collider: Collider | None
    """The attached collider, or `None`. Assign one to attach it (replacing any previous one);
    assign `None` to detach. A collider can be attached to one sprite at a time."""
    def collides_with(self, other: Sprite) -> bool:
        """Whether this sprite's collider touches `other`'s (touching counts). `False` if either
        has no collider, and for the sprite itself."""
    def collides_with_list(self, sprites: SpriteList) -> list[Sprite]:
        """The sprites in `sprites` whose colliders touch this sprite's, in list order (never
        this sprite). One engine call; the result is a new list."""

class RectangleSprite(Sprite):
    """A solid-color rectangle; no image needed."""

    def __init__(
        self,
        width: float,
        height: float,
        color: Color,
        center_x: float = 0.0,
        center_y: float = 0.0,
    ) -> None: ...

class CircleSprite(Sprite):
    """A solid-color circle; no image needed. Its edge follows the window filter unless overridden."""

    def __init__(
        self,
        radius: float,
        color: Color,
        center_x: float = 0.0,
        center_y: float = 0.0,
        *,
        filter: Filter | None = None,
    ) -> None: ...
    @property
    def radius(self) -> float:
        """Drawn radius in pixels: radius × |scale|."""

@final
class SpriteList:
    """An ordered batch of sprites, drawn together. Later sprites draw on top."""

    def __init__(self) -> None: ...
    def append(self, sprite: Sprite) -> None: ...
    def remove(self, sprite: Sprite) -> None: ...
    def update(self, delta_time: float) -> None:
        """Move every sprite by its velocities over `delta_time` seconds."""
    def draw(self) -> None:
        """Draw all sprites. Only valid inside `on_draw`."""
    def collisions_with(self, others: SpriteList) -> list[tuple[Sprite, Sprite]]:
        """Every touching (sprite from this list, sprite from `others`) pair once, in list order.
        A sprite in both lists is never paired with itself."""
    def bounce_each_other(self) -> int:
        """Bounce this list's circle-collider sprites off each other as equal-mass, perfectly
        elastic circles (touching pairs exchange velocity along the line between centers;
        overlaps are pushed apart). Returns the number of contacts resolved."""
    def bounce_off(self, walls: SpriteList) -> int:
        """Bounce this list's circle-collider sprites off `walls`, which never move (velocity into
        a wall is reversed; overlaps are pushed out). Returns the number of contacts resolved."""
    def collisions(self) -> list[tuple[Sprite, Sprite]]:
        """Every touching pair within this list once, as (earlier, later) in list order."""
    def __len__(self) -> int: ...
    def __contains__(self, sprite: object) -> bool: ...
    def __iter__(self) -> Iterator[Sprite]: ...

class Collider:
    """Base class of the colliders. Attach one with `sprite.collider = ...`.

    Sizes and offsets are in the sprite's unscaled pixels and scale with `sprite.scale`.
    """

    offset: tuple[float, float]
    """Center relative to the sprite center. Rotates with the sprite, except for `AABBCollider`."""
    @property
    def sprite(self) -> Sprite | None:
        """The sprite this collider is attached to, or `None`."""

class CircleCollider(Collider):
    """A circle collider."""

    def __init__(self, radius: float, *, offset: tuple[float, float] = (0.0, 0.0)) -> None:
        """`radius` must be finite and >= 0 (unscaled sprite pixels)."""
    radius: float

class BoxCollider(Collider):
    """A box collider that rotates with its sprite (its offset rotates too)."""

    def __init__(
        self, width: float, height: float, *, offset: tuple[float, float] = (0.0, 0.0)
    ) -> None:
        """`width` and `height` must be finite and >= 0 (unscaled sprite pixels)."""
    width: float
    height: float

class AABBCollider(Collider):
    """An axis-aligned box collider: it never rotates, and neither does its offset."""

    def __init__(
        self, width: float, height: float, *, offset: tuple[float, float] = (0.0, 0.0)
    ) -> None:
        """`width` and `height` must be finite and >= 0 (unscaled sprite pixels)."""
    width: float
    height: float

def _debug_texture_disk_reads() -> int: ...
def _key_table() -> list[tuple[str, int]]: ...
