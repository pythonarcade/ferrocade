# Contract: Showcase API additions

These are additions to the Phase 5 API. They're the same on desktop and in the browser.

## `SpriteList.bounce_each_other() -> int`

- Bounces the list's sprites off each other as **equal-mass, perfectly elastic circles**. Touching
  pairs (from the same detection as `collisions()`) are resolved once each, in list order:
  - if they're approaching, they exchange their velocity components along the line between
    their centers
  - if they overlap, they're pushed apart equally
- Only sprites with a `CircleCollider` are bounced. Others are skipped.
- Returns the number of contacts resolved. One engine call.

## `SpriteList.bounce_off(walls: SpriteList) -> int`

- Bounces this list's sprites off `walls`, which never move. If a sprite moves into a wall, the
  velocity component into the wall is reversed (speed unchanged), and a sprite overlapping a wall
  is pushed out.
- Walls may use any collider. Only sprites with a `CircleCollider` are bounced.
- Returns the number of contacts resolved. One engine call.

**Typical update order**: `balls.update(dt)`, then `balls.bounce_each_other()`, then
`balls.bounce_off(walls)`.

## `FrameStats` (new fields)

- `update_ms: float`: engine time per frame spent moving sprites (`SpriteList.update`), averaged
  over ~1 s.
- `collision_ms: float`: engine time per frame spent in collision queries and bounce calls.
- Both are 0 when unused.

## `examples/06_showcase.py`

- Keys: `=`/keypad `+` add 1,000 balls; `-`/keypad `-` remove 1,000; `T` toggles engine/Python
  bouncing; `Space` pauses.
- Desktop options: `--balls N` (default 5,000) and `--seconds S` (run S seconds, then exit).
- Console, about once per second:
  `balls=… fps=… render=…ms update=…ms collision=…ms mode=engine|python [python_bounce=…ms]`.
