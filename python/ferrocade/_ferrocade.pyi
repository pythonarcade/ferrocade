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
