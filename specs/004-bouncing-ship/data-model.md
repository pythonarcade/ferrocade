# Data Model: Phase 2 — Bouncing Ship (Movement)

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

These are the changes relative to the Phase 1 and 1c data models.

## SpriteState (core) additions

| Field | Type | Rules |
|---|---|---|
| `velocity_x` | `f64` px/s | Default 0. **Must be finite**, otherwise `ValueError`. |
| `velocity_y` | `f64` px/s | Default 0. **Must be finite**, otherwise `ValueError`. |
| `angular_velocity` | `f64` deg/s | Default 0. **Must be finite**, otherwise `ValueError`. Positive is counter-clockwise. |

**Velocity setters don't touch the instance.** Velocities aren't drawn, so setting them marks
nothing dirty.

## Store operation: `update_list(list, dt)`

| Rule | |
|---|---|
| Input | `dt` in seconds, **finite and ≥ 0**, otherwise `ValueError`. |
| Effect | For each sprite in the list, in order, with any non-zero velocity: `center_x += velocity_x·dt`, `center_y += velocity_y·dt`, `angle += angular_velocity·dt`. Then rewrite its instance in every list containing it and mark those slots dirty. |
| Zero-velocity sprites | Untouched. Not marked dirty. |
| Membership | Only sprites in *this* list move. A sprite in two updated lists moves twice (documented). |
| Angle | Accumulates without wrapping. |

## Instance / shader

There's no layout change; the instance stays 48 bytes. The shader now *uses* `angle` (offset 16)
to rotate the quad's corners around the center, **counter-clockwise for positive degrees**.

## Python API additions

| Member | Kind | Notes |
|---|---|---|
| `Sprite.velocity_x`, `Sprite.velocity_y` | `float` read/write | px/s |
| `Sprite.angular_velocity` | `float` read/write | deg/s, counter-clockwise |
| `Sprite.velocity` | `tuple[float, float]` read/write | Both components in one call |
| `SpriteList.update(delta_time)` | method | Allowed anywhere, not just inside `on_update` |

These are inherited by `RectangleSprite`, `CircleSprite`, and user subclasses.
