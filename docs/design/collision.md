# Collision System Design

**Status:** architecture decided by the owner (2026-10-12). Engine spec §11 has the summary and
the decision log. Items marked *Proposed* are details to settle in the Phase 5 spec and plan.

## 1. Decisions (owner, 2026-10-12)

| # | Question | Decision |
|---|---|---|
| 1 | Where hit boxes live | **Colliders are separate entities**, not a built-in property of `Sprite` (Unity/Godot style). |
| 2 | Shapes | Start with **circle, axis-aligned box, rotated box**. The design must make adding more shapes later (convex polygon, auto hit box from texture alpha, capsule, pixel mask) a contained change. |
| 3 | What is checked | **List-based queries** (Arcade style): game code asks about a sprite or a `SpriteList`. Declared collision pairs checked automatically every frame are in the **backlog**. |
| 4 | Results to Python | **Batched queries first.** Enter/exit events and built-in Rust responses (bounce/stop/solid) are **ideas for the showcase**, not committed. Per-pair per-frame callbacks are avoided by default. |
| 5 | Broad phase | **Spatial hash grid per list**, rebuilt only when the list's positions change (reusing dirty tracking). Very small lists test every pair. |
| 6 | Build vs. crate | **Write our own.** No `parry2d` / `rapier2d`. |

Unchanged from earlier decisions: detection runs in Rust with no per-object Python work, it must
scale to Phase 3 sprite counts, it must work in WASM (single-threaded), and Phase 5 is
detection only (no physical response).

## 2. Architecture

```text
Python                     Rust core (crates/ferrocade/src/collision/)
──────────────────         ──────────────────────────────────────────────────────────
CircleCollider ─┐          ColliderStore (slotmap): ColliderId → Collider { shape, offset, ... }
BoxCollider    ─┤ attach   SpriteState gains `colliders` (Phase 5: at most one)
                ▼          world shapes: computed from sprite position/scale/angle + collider,
sprite.collider            cached and recomputed when the sprite is dirty
                           per-list SpatialGrid: built from world-shape bounds, rebuilt when the
SpriteList queries ──────▶ list's positions changed since the last query
                           narrow phase: shape-pair tests (enum dispatch)
```

- **Colliders are their own objects** with their own handle (`ColliderId`), the same pattern as
  sprites and textures in the store. A collider is attached to a sprite and follows its transform.
  The data model lets a sprite hold several colliders later (a list, with one allowed in Phase 5).
- **Shapes are an enum** (`Shape::Circle`, `Shape::Aabb`, `Shape::Obb`). The narrow phase is a match
  on shape pairs. Adding a shape means adding a variant, its bounds and world-transform functions,
  and its pair tests. Convex shapes can share a generic separating-axis (SAT) or GJK fallback, so
  the work grows with the number of shapes, not with the number of pairs.
- **World-space shapes** are derived in Rust from the sprite's `f64` state and cached. They're
  recomputed only for sprites whose position, scale, or angle changed (the same dirty signal that
  drives GPU uploads).
- **Broad phase:** each `SpriteList` that is queried keeps a uniform grid of collider bounds. It's
  rebuilt lazily on the next query after the list's sprites moved, and static lists (walls) never
  rebuild. Cell size is automatic (*Proposed:* about 2× the median collider size). Below a small
  count (*Proposed:* 32), queries test every pair instead.
- **Queries return batches:** one boundary crossing per query, with the pair loop in Rust.

## 3. Python API sketch (*Proposed*, for the Phase 5 spec)

```python
ship.collider = fc.CircleCollider(radius=20)                 # attach (follows the sprite)
wall.collider = fc.BoxCollider(width, height)                # rotates with the sprite (rotated box)
tile.collider = fc.BoxCollider(32, 32, rotates=False)        # axis-aligned box (fast path)

ship.collides_with(rock) -> bool
ship.collides_with_list(rocks) -> list[Sprite]               # one query, one crossing
bullets.collisions_with(rocks) -> list[tuple[Sprite, Sprite]]
balls.collisions() -> list[tuple[Sprite, Sprite]]            # pairs within one list
```

- Sprites with no collider never collide.
- Collider `offset=(x, y)` positions a shape relative to the sprite center (rotating with it).
- Axis-aligned vs. rotated box: one `BoxCollider` class with `rotates=` (*Proposed*), or two classes.

## 4. Open questions for the Phase 5 spec

1. **Default colliders.** Should shape sprites (`CircleSprite`, `RectangleSprite`) get a matching
   collider automatically, or must every collider be attached explicitly? (Explicit fits "separate
   entities"; automatic is friendlier.)
2. **Colliders without sprites** (invisible triggers): in Phase 5, or later? They'd need their own
   position and their own list type.
3. **Box naming and options:** the exact class names, and whether scale applies to colliders.
4. **Query result types:** lists of sprites vs. a lighter result object; deduplication and ordering
   of pairs.
5. **Performance targets:** for example, all pairs within a 10,000-sprite list in under N ms.

## 5. Later (not Phase 5)

- **Backlog:** declared collision pairs checked every frame (`collisions.watch(a, b)`).
- **Showcase ideas:** enter/exit events delivered in a batch after each update, and built-in Rust
  responses (bounce, stop, solid walls).
- **Shapes:** convex polygon, auto hit box from texture alpha (cached per texture), capsule, pixel
  mask.
- **Broad phase:** a tree (BVH/quadtree) for large static level geometry.
- **Tunneling:** swept tests for fast objects.
- **Bulk results** as NumPy index arrays (with §7 NumPy views).
