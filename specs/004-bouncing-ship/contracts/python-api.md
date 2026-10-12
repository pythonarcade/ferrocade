# Contract: `ferrocade` Python API additions (Phase 2)

**Feature**: [../spec.md](../spec.md) | **Base**: [Phase 1](../../001-centered-ship/contracts/python-api.md), [Phase 1c](../../003-colors-shape-sprites/contracts/python-api.md)

The desktop module and the browser shim MUST both satisfy this.

## `Sprite` motion

```python
sprite.velocity_x = 200.0          # pixels per second (+x is right)
sprite.velocity_y = -50.0          # pixels per second (+y is up)
sprite.velocity = (200.0, -50.0)   # both in one call; reads back as a tuple
sprite.angular_velocity = 90.0     # degrees per second, counter-clockwise
sprite.angle = 45.0                # now drawn rotated (counter-clockwise), around the center
```

| Behavior | Contract |
|---|---|
| Defaults | All velocities are `0.0`. |
| Validation | NaN or ±inf raises `ValueError`. |
| Exactness | Values read back exactly as set. |
| Drawing | `angle` rotates the sprite counter-clockwise around its center. `width`/`height` stay unrotated. |

## `SpriteList.update`

```python
class SpriteList:
    def update(self, delta_time: float) -> None: ...
```

| Behavior | Contract |
|---|---|
| Effect | Every sprite in the list: position += velocity × `delta_time`, and angle += angular_velocity × `delta_time`. |
| Validation | `delta_time` must be finite and ≥ 0, otherwise `ValueError`. |
| Where | Typically called from `on_update(delta_time)`, but allowed anywhere. |
| Membership | Moves only this list's sprites. A sprite in two lists moves once per list updated. |
| Timing | The changes appear on the next drawn frame. |

## Example (`examples/02_bouncing_ship.py`, sketch)

```python
import ferrocade as fc


class Bounce(fc.Window):
    def __init__(self):
        super().__init__(800, 600, "Bouncing ship", filter=fc.NEAREST)
        self.ship = fc.Sprite("assets/images/ship.png", self.width / 2, self.height / 2)
        self.ship.velocity = (220, 170)
        self.ship.angular_velocity = 45
        self.sprites = fc.SpriteList()
        self.sprites.append(self.ship)

    def on_update(self, delta_time):
        self.sprites.update(delta_time)
        s, half_w, half_h = self.ship, self.ship.width / 2, self.ship.height / 2
        if not half_w <= s.center_x <= self.width - half_w:
            s.velocity_x = -s.velocity_x
            s.center_x = min(max(s.center_x, half_w), self.width - half_w)
        if not half_h <= s.center_y <= self.height - half_h:
            s.velocity_y = -s.velocity_y
            s.center_y = min(max(s.center_y, half_h), self.height - half_h)

    def on_draw(self):
        self.clear()
        self.sprites.draw()


Bounce().run()
```
