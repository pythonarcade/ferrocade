# Research: Phase 2 — Bouncing Ship (Movement)

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12

All decisions build on existing engine pieces: the store, `FixedTimestep`, the instance's
`angle` field, and the sprite shader. No external research was needed.

---

## R1. Where motion state lives, and how `update` works

**Decision**:
- `SpriteState` gains `velocity_x`, `velocity_y`, and `angular_velocity` (`f64`, default 0).
- The new store function `update_list(list, dt)` walks the list's sprites in order. For each sprite
  with **any non-zero velocity**, it does `center += velocity × dt` and `angle += angular_velocity × dt`.
  It then rewrites the sprite's instance in **every** list containing it and marks those slots dirty,
  using the existing `update()` helper path.
- Sprites with zero velocity are skipped entirely, so they cause no dirty marking and no upload (FR-007).
- `dt` must be finite and ≥ 0, otherwise `ValueError`. (A negative `dt` would mean running time
  backwards. It's rejected, to catch mistakes like passing a timestamp.)

**Rationale**:
- One call per list, with no per-sprite Python work (constitution II, FR-005).
- Reusing the membership-aware instance rewrite keeps multi-list sprites consistent.

**Alternatives considered**:
- **A global "update everything" on the store.** Rejected: the owner and spec chose an explicit, per-list update (Arcade-style), so the game controls what moves.
- **Updating instances in place without the per-membership rewrite.** Rejected: it breaks sprites that belong to several lists.

**Note for later (the showcase)**: dirty ranges are min/max spans. When all sprites move, the
whole buffer is uploaded each frame. That's optimal for "everything moves", and it's the case the
showcase needs.

---

## R2. Rotation in the shader (counter-clockwise)

**Decision**: the vertex shader rotates each corner offset by `angle`, converted to radians, around
the sprite center:

```text
offset  = (corner − 0.5) × size
rotated = (offset.x·cos a − offset.y·sin a,  offset.x·sin a + offset.y·cos a)
world   = center + rotated
```

- The projection is y-up, so this standard rotation is **counter-clockwise** on screen (FR-009, the
  owner's decision).
- `width`/`height` stay the unrotated sizes (FR-010).
- Circles are unaffected visually.
- Angle 0 gives `cos = 1` and `sin = 0` exactly, so **every existing pixel test is unaffected**.

**Rationale**: a few lines of WGSL, it works in WebGL2, and the instance already carries `angle`.

**Verification**: an offscreen test draws a non-square sprite (a 60×20 rectangle) at angle 90 and
checks that its bounding box becomes 20×60. A second test uses an asymmetric marker to confirm the
*direction*: a small square offset toward +x in an image texture appears toward +y after +90°.
That needs a two-tone test texture.

---

## R3. Fixed-timestep guarantees (verification only)

**Decision**: no engine changes. `FixedTimestep` already runs updates at a fixed rate, with at most
5 per frame and the backlog beyond that dropped (Phase 1).

**Phase 2 adds tests that prove it from Python:**
- **Rate:** run a window for 3 s and count `on_update` calls. Expect 180 (±3%).
- **Slow drawing:** the same, but `on_draw` sleeps 50 ms (about 20 FPS). Expect still about 180,
  and a sprite moving at 100 px/s covers about 300 px (±3%).
- **Stall cap:** the existing Rust timestep test covers the 5-step cap. A 3 s check keeps the
  pytest suite fast. SC-002's 10 s figure is checked manually (quickstart B4).

---

## R4. Python API

**Decision**:
- `Sprite.velocity_x`, `Sprite.velocity_y`, and `Sprite.angular_velocity` (read/write, `float`).
- `Sprite.velocity`, a tuple getter and setter that sets both in one boundary crossing, mirroring `position`.
- `SpriteList.update(delta_time: float) -> None`.
- **Non-finite values** raise `ValueError` (the same validation as positions).
- **`SpriteList.update` outside a frame is allowed.** It isn't a drawing call, so it isn't restricted
  to `on_update`, which lets tests and tools step lists directly.

**Rationale**: matches the engine spec §8 names and the existing property style.

---

## R5. Web parity

**Decision**:
- **Engine (`crates/ferrocade-web`):** add `get_velocity(sprite) -> [vx, vy, va]`,
  `set_velocity(sprite, vx, vy)`, `set_angular_velocity(sprite, va)`, and `update_list(list, dt)`.
- **Shim:** the matching properties and `SpriteList.update`.
- **Rotation** works in the browser automatically, because the shader is shared.
- **Verification:** the browser check runs `?example=02_bouncing_ship`, takes two screenshots about
  1 s apart, and confirms that the ship moved and stays inside the canvas.

---

## R6. Example design (`examples/02_bouncing_ship.py`)

**Decision**:
- **Default mode:** the ship starts centered with velocity (220, 170) px/s and a slow spin
  (`angular_velocity = 45`). In `on_update`, it calls `self.sprites.update(dt)`, then checks the
  edges using the sprite's unrotated half-size. On a hit, it flips the velocity component and
  clamps the position back inside, so the ship never sticks.
- **`--many` mode:** 1,000 random shapes with random velocities. They bounce off the edges via the
  same Python check in a loop, and the frame time is printed every 2 s (the startup excluded).
- **This measures what the showcase needs to know:** Python edge checks over 1,000 sprites, at
  about 4 property reads each, give about 4,000 boundary crossings per update. Record the cost.

**Rationale**: the milestone demo, plus a first real measurement of Python↔Rust call overhead
under load, which feeds the collision-API design (§11 Q6).
