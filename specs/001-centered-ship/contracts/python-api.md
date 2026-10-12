# Contract: `ferrocade` Python API (Phase 1)

**Feature**: [../spec.md](../spec.md) | **Data model**: [../data-model.md](../data-model.md)

This is the public interface that Phase 1 delivers. The type stubs
(`python/ferrocade/_ferrocade.pyi`) MUST match this contract. Anything not listed here is
private (leading underscore) and can change.

```python
import ferrocade as fc
```

## Module contents

| Name | Kind | FR |
|---|---|---|
| `fc.Window` | class (subclassable) | FR-001 to FR-006 |
| `fc.Texture` | class (final) | FR-007 to FR-011 |
| `fc.Sprite` | class (subclassable) | FR-012 to FR-015 |
| `fc.SpriteList` | class (final) | FR-016 to FR-018 |
| `fc.Filter` | enum: `Filter.NEAREST`, `Filter.LINEAR` | FR-022, FR-023 |
| `fc.NEAREST`, `fc.LINEAR` | aliases for `Filter.NEAREST` / `Filter.LINEAR` | FR-022, FR-023 |
| `fc.color` | module of RGB tuples (`BLACK`, `WHITE`, `DARK_GRAY`, …) | FR-005 |
| `fc.__version__` | `str` | — |

`Color = tuple[int, int, int] | tuple[int, int, int, int]`, with each component from 0 to 255.

---

## `Window`

```python
class Window:
    def __init__(self, width: int = 800, height: int = 600,
                 title: str = "Ferrocade", *, filter: Filter,
                 update_rate: float = 1 / 60) -> None: ...

    width: int          # read-only, logical pixels; updates on resize
    height: int         # read-only
    title: str          # read-only in Phase 1
    filter: Filter      # read-only in Phase 1; default filter for textures without an override
    update_rate: float  # read-only, seconds per update

    def run(self) -> None: ...
    def clear(self, color: Color = fc.color.DARK_GRAY) -> None: ...

    # Override in subclasses (default implementations do nothing)
    def on_draw(self) -> None: ...
    def on_update(self, delta_time: float) -> None: ...
```

| Behavior | Contract |
|---|---|
| Construction | `width`, `height` > 0 and `update_rate` > 0, otherwise `ValueError`. Creating the object does **not** open an OS window. |
| `filter` (required) | Keyword-only, with **no default**. Leaving it out raises `TypeError` (missing required argument `filter`). Values other than a `Filter` raise `TypeError`. |
| `run()` | Opens the window, calls `on_update(update_rate)` at a fixed rate and `on_draw()` once per displayed frame, and returns when the window is closed. Must be called on the main thread (`RuntimeError` otherwise). Can be called again after it returns. |
| Callback exceptions | An exception raised in `on_draw` / `on_update` closes the window and is **re-raised from `run()`** with its original traceback. |
| Ctrl+C | Raises `KeyboardInterrupt` from `run()` and closes the window. |
| `clear()` | Only valid inside `on_draw` (`RuntimeError` otherwise). Fills the canvas with `color`. Color components outside 0–255, or a tuple length other than 3 or 4, raise `ValueError`. |
| No `clear()` in a frame | The frame starts filled with `fc.color.DARK_GRAY`. |
| Subclassing | `class Game(fc.Window): def __init__(self): super().__init__(800, 600, "Title", filter=fc.NEAREST)` MUST work. Overridden `on_draw` / `on_update` are called. |

---

## `Filter`

```python
class Filter(enum):        # native enum
    NEAREST   # crisp pixels; no smoothing (pixel art)
    LINEAR    # smooth interpolation (photos, scaled/rotated art)

NEAREST = Filter.NEAREST
LINEAR = Filter.LINEAR
```

There is deliberately **no default value** anywhere in the API. The window requires one, and textures can override it.

---

## `Texture`

```python
class Texture:
    def __init__(self, path: str | os.PathLike[str], *,
                 filter: Filter | None = None) -> None: ...
    width: int             # read-only, pixels
    height: int            # read-only, pixels
    path: str              # read-only, canonical absolute path
    filter: Filter | None  # read-only; None = use the window's filter
```

| Behavior | Contract |
|---|---|
| Loading | The relative path is resolved against the current working directory. Works before any `Window` exists. |
| Sharing | All `Texture` objects for the same canonical file share one loaded image, and the file is read from disk once per process, whatever their `filter`. `Texture(a) == Texture(b)` (with equal hashes) when they have the same canonical file **and** the same `filter` override. |
| Filter override | `filter=None` (default) uses the window's filter at draw time. `filter=fc.LINEAR` or `fc.NEAREST` overrides it for sprites using this texture. Values other than a `Filter`/`None` raise `TypeError`. |
| Formats | PNG (required, including transparency). JPEG, BMP, and GIF (first frame) also load. |
| Missing file | `FileNotFoundError`, with a message containing the path. |
| Unsupported or corrupt file | `ValueError`, with a message containing the path and the reason. |

---

## `Sprite`

```python
class Sprite:
    def __init__(self, texture: Texture | str | os.PathLike[str],
                 center_x: float = 0.0, center_y: float = 0.0,
                 scale: float = 1.0) -> None: ...

    center_x: float                      # read/write, px
    center_y: float                      # read/write, px
    position: tuple[float, float]        # read/write, (center_x, center_y)
    scale: float                         # read/write
    angle: float                         # read/write, degrees (not rendered in Phase 1)
    texture: Texture                     # read/write
    width: float                         # read-only: texture.width * abs(scale)
    height: float                        # read-only: texture.height * abs(scale)
```

| Behavior | Contract |
|---|---|
| Construction from a path | Equivalent to `Sprite(Texture(path), ...)` (shared cache, no filter override, so it uses the window's filter). The same errors as `Texture`. |
| Coordinates | Pixels, origin at the bottom-left of the window, y up. The position is the sprite's center. |
| Values | Non-finite numbers (NaN, ±inf) raise `ValueError`. Values read back exactly as set (stored as 64-bit floats). |
| Visibility of changes | Any change appears on the next displayed frame of every list containing the sprite. |
| Subclassing | `class Ship(fc.Sprite): def __init__(self): super().__init__("ship.png", 400, 300)` MUST work, and attributes added by the subclass are allowed. |

---

## `SpriteList`

```python
class SpriteList:
    def __init__(self) -> None: ...
    def append(self, sprite: Sprite) -> None: ...
    def remove(self, sprite: Sprite) -> None: ...
    def draw(self) -> None: ...
    def __len__(self) -> int: ...
    def __contains__(self, sprite: Sprite) -> bool: ...
    def __iter__(self) -> Iterator[Sprite]: ...
```

| Behavior | Contract |
|---|---|
| `append` | Adds to the end (drawn on top). Appending a sprite already in this list raises `ValueError`. A sprite can be in multiple lists. |
| `remove` | Removes the sprite and preserves the order of the rest. Removing a sprite that isn't in the list raises `ValueError`. |
| `draw` | Only valid inside `on_draw` (`RuntimeError` otherwise). Draws all sprites with alpha blending, in list order. An empty list draws nothing. |
| Iteration | Yields the same Python objects that were appended (subclass instances included), in order. |
| Lifetime | A list keeps its sprites alive while they are members. |

---

## Usage (acceptance example, `examples/01_centered_ship.py`)

```python
import ferrocade as fc


class Game(fc.Window):
    def __init__(self):
        super().__init__(800, 600, "Ferrocade", filter=fc.NEAREST)
        self.sprites = fc.SpriteList()
        ship = fc.Sprite("assets/images/ship.png",
                         center_x=self.width / 2, center_y=self.height / 2)
        self.sprites.append(ship)

    def on_draw(self):
        self.clear()
        self.sprites.draw()


Game().run()
```
