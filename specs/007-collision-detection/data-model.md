# Data Model: Phase 5 — Collision Detection

All state lives in the Rust core (`SpriteStore`, plus `crates/ferrocade/src/collision/`). Python
objects only hold ids.

## Collider (core)
- **Id**: `ColliderId` (slotmap key).
- **Fields**: `shape: Shape`, `offset: (f64, f64)` (unscaled sprite pixels), `attached_to: Option<SpriteId>`.
- **Shape**: `Circle { radius }` | `Box { width, height }` (rotates) | `Aabb { width, height }` (never rotates).
- **Validation**: radius, width, height finite and ≥ 0; offset components finite. Violations →
  `InvalidArgument` (Python `ValueError`).
- **Rules**: attached to at most one sprite. Attaching an already-attached collider to another
  sprite is an error. Destroying a sprite detaches its collider.

## SpriteState (extended)
- `collider: Option<ColliderId>`, at most one in Phase 5. The design allows a list later.

## World shape (derived, per query)
- `Circle { center, radius }` | `Obb { center, half_extents, cos, sin }` (an AABB is cos = 1, sin = 0).
- Computed from the sprite's center, |scale|, and angle (degrees, CCW) plus the collider:
  - Circle and Box offsets rotate with the sprite.
  - AABB offsets don't.
  - Sizes and offsets are multiplied by |scale|.

## ListState (extended)
- `content_version: u64`: bumped whenever instance data is marked dirty, on append/remove, and when
  a member's collider is attached, detached, or resized.
- `collision_cache: Option<CollisionCache>`.

## CollisionCache (per list, lazy)
- `version: u64` (the `content_version` it was built from).
- `shapes: Vec<Option<WorldShape>>` and `bounds: Vec<Aabb>`, per slot (`None` = no collider).
- `grid: Option<Grid>` (`None` below 32 colliders → all pairs).
- **Grid**: origin, cell size, dims (≤ 1024 per axis), `cell_start: Vec<u32>`, `items: Vec<u32>`,
  `large: Vec<u32>` (objects covering more than 16 cells).

## Query results
| Query | Core result | Python result |
|---|---|---|
| `a.collides_with(b)` | `bool` | `bool` |
| `s.collides_with_list(L)` | ascending slots in L, excluding s | `list[Sprite]` |
| `A.collisions_with(B)` | `(slot_a, slot_b)` sorted | `list[tuple[Sprite, Sprite]]` |
| `L.collisions()` | `(i, j)`, i < j, sorted | `list[tuple[Sprite, Sprite]]` |
