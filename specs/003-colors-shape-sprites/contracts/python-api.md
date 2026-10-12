# Contract: `ferrocade` Python API additions (Phase 1c)

**Feature**: [../spec.md](../spec.md) | **Base contract**: [Phase 1 python-api.md](../../001-centered-ship/contracts/python-api.md)

These are the additions and changes to the Phase 1 contract. Everything else is unchanged.
The desktop module and the browser shim MUST both satisfy this.

## `fc.color`

```python
fc.color.ALICE_BLUE        # (240, 248, 255)
fc.color.CORNFLOWER_BLUE   # (100, 149, 237)
fc.color.REBECCA_PURPLE    # (102, 51, 153)
fc.color.GREEN             # (0, 128, 0)    CSS value
fc.color.LIME              # (0, 255, 0)
fc.color.DARK_GRAY         # (169, 169, 169) == fc.color.DARK_GREY
fc.color.TRANSPARENT       # (0, 0, 0, 0)
fc.color.DEFAULT_BACKGROUND  # (32, 32, 32)  default clear color (not a CSS name)
```

| Behavior | Contract |
|---|---|
| Coverage | All 148 CSS Color Level 4 named colors, each as the UPPER_SNAKE form of the keyword. |
| Values | Plain tuples of ints, identical to CSS. |
| `Window.clear()` with no argument | Clears to `DEFAULT_BACKGROUND` (unchanged from Phase 1). |

## `Sprite.color`

```python
sprite.color                      # -> (r, g, b, a), default (255, 255, 255, 255)
sprite.color = fc.color.RED       # 3-tuple: alpha becomes 255
sprite.color = (255, 255, 255, 128)
```

| Behavior | Contract |
|---|---|
| Effect | The sprite's image is multiplied by the color (per channel, /255, including alpha). White means unchanged. |
| Validation | A tuple length other than 3 or 4, or a component outside 0-255, raises `ValueError` (the same message style as `clear()`). |
| Timing | Shows on the next frame. |

## `fc.RectangleSprite`

```python
class RectangleSprite(Sprite):
    def __init__(self, width: float, height: float, color: Color,
                 center_x: float = 0.0, center_y: float = 0.0) -> None: ...
```

| Behavior | Contract |
|---|---|
| Size | Drawn at `width × height × |scale|` pixels. `sprite.width` and `sprite.height` report that. |
| Validation | `width`/`height` must be finite and > 0, otherwise `ValueError`. `color` is validated like `Sprite.color`. |
| `texture` | Readable (a built-in texture with `path == "<builtin:shapes>"`). Assigning one raises `TypeError`. |
| Subclassing | `class Wall(fc.RectangleSprite): def __init__(self): super().__init__(50, 20, fc.color.RED)` MUST work. |

## `fc.CircleSprite`

```python
class CircleSprite(Sprite):
    def __init__(self, radius: float, color: Color,
                 center_x: float = 0.0, center_y: float = 0.0,
                 *, filter: Filter | None = None) -> None: ...
    radius: float   # read-only: radius × |scale|
```

| Behavior | Contract |
|---|---|
| Size | Diameter `2 × radius × |scale|`. `width == height == 2 × radius`. |
| Edge | It follows the window's filter (stepped with `NEAREST`, smooth with `LINEAR`), unless `filter=` overrides it. |
| Validation | `radius` must be finite and > 0, otherwise `ValueError`. `filter` must be a `Filter` or `None`, otherwise `TypeError`. |
| Subclassing | `class Ball(fc.CircleSprite)` with `super().__init__(...)` MUST work. |

## Example (`examples/01c_shapes.py`, sketch)

```python
import ferrocade as fc


class Shapes(fc.Window):
    def __init__(self):
        super().__init__(800, 600, "Shapes", filter=fc.NEAREST)
        self.sprites = fc.SpriteList()
        self.sprites.append(fc.RectangleSprite(200, 40, fc.color.CORNFLOWER_BLUE, 400, 80))
        self.sprites.append(fc.CircleSprite(40, fc.color.ORANGE, 200, 300))
        self.sprites.append(fc.CircleSprite(40, fc.color.ORANGE, 600, 300, filter=fc.LINEAR))
        ship = fc.Sprite("assets/images/ship.png", 400, 300)
        ship.color = fc.color.LIGHT_PINK          # tinted image sprite
        self.sprites.append(ship)

    def on_draw(self):
        self.clear()
        self.sprites.draw()


Shapes().run()
```
