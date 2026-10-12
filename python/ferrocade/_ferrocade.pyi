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
