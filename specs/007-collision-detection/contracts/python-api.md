# Contract: Phase 5 Python API additions

These are additions to the Phase 4 API. They behave the same on desktop and in the browser.

## Collider classes

```python
class Collider:                     # base class; not constructed directly
    offset: tuple[float, float]     # relative to the sprite center, unscaled sprite pixels
    @property
    def sprite(self) -> Sprite | None: ...   # the sprite it's attached to

class CircleCollider(Collider):
    def __init__(self, radius: float, *, offset: tuple[float, float] = (0, 0)) -> None: ...
    radius: float

class BoxCollider(Collider):        # rotates with the sprite
    def __init__(self, width: float, height: float, *, offset=(0, 0)) -> None: ...
    width: float
    height: float

class AABBCollider(Collider):       # never rotates; its offset doesn't rotate either
    def __init__(self, width: float, height: float, *, offset=(0, 0)) -> None: ...
    width: float
    height: float
```

- **Units**: sizes and offsets are in the sprite's **unscaled** pixels and scale with
  `sprite.scale`. Circle and Box offsets rotate with the sprite (counter-clockwise degrees).
- **Validation**: sizes must be finite and ≥ 0 (zero is a point or line); offsets must be finite.
  Otherwise `ValueError`.
- Properties are writable. Changes take effect for the next query.

## `Sprite.collider: Collider | None`

- Assigning attaches (replacing any previous collider, which becomes unattached). `None` detaches.
- A collider can be attached to one sprite at a time. Assigning one that's attached elsewhere
  raises `ValueError`.
- Sprites have no collider until one is assigned. Shape sprites are no exception.

## Queries

```python
sprite.collides_with(other: Sprite) -> bool
sprite.collides_with_list(sprites: SpriteList) -> list[Sprite]
sprites.collisions_with(others: SpriteList) -> list[tuple[Sprite, Sprite]]
sprites.collisions() -> list[tuple[Sprite, Sprite]]
```

- **Touching counts.** Sprites without a collider never collide.
- `collides_with_list`: list order, never the sprite itself.
- `collisions_with`: each (a in `sprites`, b in `others`) pair once, ordered by a's then b's
  position. A sprite in both lists is never paired with itself.
- `collisions`: each pair within the list once, as (earlier, later), ordered by list position.
- Results are new lists (snapshots). It's safe to move or remove sprites while looping over them.
- `collides_with(self)` is `False`.

## `Window.title` (now writable)

- `self.title = f"Hits: {n}"` updates the window title (the browser tab title on the web). With
  `show_fps` on, the FPS readout is appended to the current title.
