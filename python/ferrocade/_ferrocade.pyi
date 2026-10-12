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
    @property
    def title(self) -> str: ...
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
    def __len__(self) -> int: ...
    def __contains__(self, sprite: object) -> bool: ...
    def __iter__(self) -> Iterator[Sprite]: ...

def _debug_texture_disk_reads() -> int: ...
