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
    @property
    def width(self) -> float:
        """Drawn width in pixels: texture width × |scale|."""
    @property
    def height(self) -> float:
        """Drawn height in pixels: texture height × |scale|."""

@final
class SpriteList:
    """An ordered batch of sprites, drawn together. Later sprites draw on top."""

    def __init__(self) -> None: ...
    def append(self, sprite: Sprite) -> None: ...
    def remove(self, sprite: Sprite) -> None: ...
    def draw(self) -> None:
        """Draw all sprites. Only valid inside `on_draw`."""
    def __len__(self) -> int: ...
    def __contains__(self, sprite: object) -> bool: ...
    def __iter__(self) -> Iterator[Sprite]: ...

def _debug_texture_disk_reads() -> int: ...
