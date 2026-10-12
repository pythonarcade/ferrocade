# Data Model: Performance Showcase

## Bounce contact (core, transient)
- **Ball–ball**: `(slot_i, slot_j)` from `within_list`, resolved in order. Inputs: both world
  circle centers and radii, plus velocities. Outputs: new velocities (normal components
  exchanged if approaching), and positions separated by overlap / 2 each.
- **Ball–wall**: `(ball_slot, wall_slot)` from `list_vs_list`, resolved in order. Inputs: the ball
  circle, the wall world shape (circle or box), and the ball velocity. Outputs: a reflected
  velocity (if moving into the wall) and the ball pushed out.
- **Rules**:
  - Only sprites whose collider is a circle are moved.
  - Walls are never modified.
  - The return value is the number of contacts resolved.

## SpriteStore (extended)
- `update_seconds: Cell<f64>` and `collision_seconds: Cell<f64>`: accumulated since the last
  `take_frame_timings()`.
- `take_frame_timings() -> (update_ms, collision_ms)`: read and reset (called once per frame by
  the game loop).

## FrameSnapshot / FrameStats (extended)
- Existing fields: `fps`, `frame_ms`, `render_cpu_ms`, `draw_calls`, `instances`.
- New: `update_ms`, `collision_ms`. These are per-frame engine time averaged over ~1 s, and 0 when
  unused.

## Showcase scene (example state)
- `walls`: 4 `RectangleSprite`s with `BoxCollider`s, 10 px thick, around 1280×720.
- `balls`: N `CircleSprite`s (radius 2–4) with `CircleCollider`s, random CSS colors, and
  velocities of 40–200 px/s in random directions.
- `mode`: `"engine"` | `"python"`; `paused: bool`; a stats timer.
