# Feature Specification: Phase 5 — Collision Detection

**Feature Branch**: `007-collision-detection`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Phase 5: collisions (engine spec §10 Phase 5, §11, docs/design/collision.md). Detect collisions between the ship and other sprites, with detection in Rust, fast enough for Phase 3 sprite counts." (The command was run without arguments right after the owner finished the collision design; the description is taken from the roadmap.)

**Clarifications (owner, 2026-10-12)**: no collider debug view in Phase 5 (backlog); speed target: all pairs among 10,000 moving sprites < 2 ms, one sprite vs. 10,000 < 0.5 ms.

**Context**: This is roadmap Phase 5. The architecture is already decided by the owner (engine spec §11, `docs/design/collision.md`):
- **Colliders are separate objects**, attached **explicitly** to sprites, following the sprite's position, rotation, and scale. Sprites without a collider never collide.
- **Shapes:** `CircleCollider`, `BoxCollider` (rotates with the sprite), `AABBCollider` (never rotates). More shapes come later, so the design must allow adding them.
- **Scale is coupled:** collider sizes and offsets are in the sprite's unscaled pixels and grow or shrink with the sprite's scale. Offsets rotate with the sprite (not for `AABBCollider`).
- **Game code asks** (list-based queries). Results come back in **batches**, not one callback per pair.
- **Detection only:** nothing is pushed apart or bounced by the engine in this phase.
- Detection runs in the engine core. Python only receives results.

**Out of scope**: colliders without a sprite, collision pairs the engine checks automatically every frame, enter/exit events, built-in responses (bounce, stop, solid), polygon and other shapes, swept tests for fast objects, contact details (normals, depth), and a debug view that draws colliders (owner: backlog). These are in the backlog or listed as showcase ideas.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Give sprites colliders (Priority: P1)

A developer attaches a collider to a sprite: a circle for a round asteroid, a rotating box for a ship or wall, or an axis-aligned box for an upright pickup. They can size it, offset it from the sprite center, replace it, or remove it. The collider follows the sprite as it moves, rotates, and scales.

**Why this priority**: every query needs colliders. This is the foundation.

**Independent Test**: attach each collider type to a sprite, move, rotate, and scale the sprite, and check that `collides_with` against a fixed probe sprite changes exactly when the shapes start and stop overlapping.

**Acceptance Scenarios**:

1. **Given** a sprite with a `CircleCollider` of radius 10 at (100, 100), **When** another circle-collider sprite of radius 10 is placed at (119, 100), **Then** they collide; at (121, 100) they don't.
2. **Given** a sprite with a `BoxCollider`, **When** the sprite rotates 45°, **Then** its collider rotates too: a probe near a corner of the unrotated box no longer collides, and one near a corner of the rotated box does.
3. **Given** a sprite with an `AABBCollider`, **When** the sprite rotates, **Then** the collider stays axis-aligned.
4. **Given** a sprite with scale 2 and a `CircleCollider` of radius 10, **When** collisions are checked, **Then** the collider acts as radius 20.
5. **Given** a collider with offset (20, 0) on a sprite rotated 90°, **When** collisions are checked, **Then** the collider is centered 20 px *above* the sprite center (offset rotates). For an `AABBCollider`, the offset stays (20, 0).
6. **Given** a sprite without a collider, **When** it overlaps other sprites, **Then** it's never reported as colliding.
7. **Given** a sprite with a collider, **When** the developer sets its collider to `None`, **Then** it stops colliding; assigning a new collider replaces the old one.

---

### User Story 2 - Ask what a sprite is touching (Priority: P1)

A developer asks whether the ship touches a specific sprite, or which sprites in a list (asteroids, coins) it touches, and gets the answer back in one call.

**Why this priority**: the core use case in the roadmap ("the ship collides with many objects").

**Independent Test**: place a ship among sprites at known positions and check that the returned list contains exactly the overlapping ones.

**Acceptance Scenarios**:

1. **Given** a ship and an asteroid that overlap, **When** the game asks `ship.collides_with(asteroid)`, **Then** the answer is `True`; for non-overlapping sprites, `False`.
2. **Given** a list of 1,000 asteroids, 3 of which overlap the ship, **When** the game asks `ship.collides_with_list(asteroids)`, **Then** exactly those 3 asteroids are returned, in the list's order.
3. **Given** the ship is in the list it checks against, **When** the query runs, **Then** the ship isn't reported as colliding with itself.

---

### User Story 3 - Find all colliding pairs between lists or within a list (Priority: P2)

A developer asks which bullets hit which asteroids (two lists), or which balls in one list overlap each other, and gets all pairs back at once.

**Why this priority**: needed for games with many interacting objects and for the later showcase. Story 2 alone covers the single-ship case.

**Independent Test**: with known positions, compare the returned pairs with a brute-force reference computed in the test.

**Acceptance Scenarios**:

1. **Given** a list of bullets and a list of asteroids, **When** the game asks `bullets.collisions_with(asteroids)`, **Then** every overlapping (bullet, asteroid) pair is returned exactly once.
2. **Given** one list of balls, **When** the game asks `balls.collisions()`, **Then** every overlapping pair within the list is returned exactly once, never a ball with itself, and never both (a, b) and (b, a).
3. **Given** the same positions, **When** the query runs twice, **Then** the results are identical and in the same order.

---

### User Story 4 - Collisions in the example, on desktop and in the browser (Priority: P2)

A player steers the ship (Phase 4 controls) through a field of drifting asteroids of mixed shapes. Asteroids the ship touches disappear, and the window title shows how many have been hit. The same example runs unchanged in the browser.

**Why this priority**: this is the phase's acceptance demo. It depends on stories 1 and 2.

**Independent Test**: run the example and steer into asteroids. Automated checks drive it with injected keys (desktop) and synthetic key events (browser) and confirm the hit count increases.

**Acceptance Scenarios**:

1. **Given** the example is running, **When** the ship flies into an asteroid, **Then** that asteroid disappears and the hit count goes up by one.
2. **Given** the ship flies near but not into an asteroid, **When** their shapes don't overlap, **Then** nothing happens.
3. **Given** the browser build, **When** the same example runs, **Then** collisions behave the same.

### Edge Cases

- **Touching edges:** shapes that only touch (zero overlap) count as colliding. Separated by any positive gap, they don't.
- **Zero-size or invalid colliders:** a radius, width, or height that is negative, NaN, or infinite is rejected with a clear error. Zero size is allowed and acts as a point or line.
- **Sprite scale 0:** the collider shrinks to a point at the (scaled) offset.
- **Changing a sprite during a query loop:** results are a snapshot. Moving or removing sprites while iterating over the results doesn't affect the returned list.
- **Removing a sprite from a list after it was returned:** allowed. Later queries don't include it.
- **One collider object shared by two sprites:** *Assumed:* a collider attaches to one sprite at a time; attaching it to a second sprite raises a clear error. (A copy can be made for the second sprite.)
- **Empty lists, or no sprites with colliders:** queries return empty results.
- **Very large and very small colliders mixed in one list:** results stay correct (only speed may vary).
- **Fast objects passing through each other** between updates aren't detected in this phase (documented).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The engine MUST provide `CircleCollider(radius)`, `BoxCollider(width, height)`, and `AABBCollider(width, height)`, each with an optional offset from the sprite center.
- **FR-002**: Colliders MUST be attached explicitly (`sprite.collider = ...`), replaced by assigning another, and removed by assigning `None`. No sprite has a collider unless one is attached.
- **FR-003**: A collider MUST follow its sprite's position, scale, and (for circle and box colliders) rotation, including the offset. `AABBCollider` MUST stay axis-aligned and its offset MUST NOT rotate.
- **FR-004**: Collider sizes and offsets MUST be in the sprite's unscaled pixels and MUST scale with the sprite's scale.
- **FR-005**: `sprite.collides_with(other) -> bool` MUST report whether two sprites' colliders overlap or touch.
- **FR-006**: `sprite.collides_with_list(sprite_list) -> list[Sprite]` MUST return the sprites in the list that collide with the sprite, in list order, excluding the sprite itself.
- **FR-007**: `list_a.collisions_with(list_b) -> list[tuple[Sprite, Sprite]]` MUST return every colliding (a, b) pair exactly once, with a from list_a and b from list_b, in a deterministic order.
- **FR-008**: `sprite_list.collisions() -> list[tuple[Sprite, Sprite]]` MUST return every colliding pair within the list exactly once, ordered (earlier, later) by list position.
- **FR-009**: All collision tests MUST be exact for the supported shape pairs (circle–circle, circle–box, box–box in any mix of rotating and axis-aligned boxes), with touching counted as colliding.
- **FR-010**: Each query MUST cross from Python to the engine once and return all results together. No per-pair Python callbacks.
- **FR-011**: Invalid collider sizes or offsets (negative, NaN, infinite) MUST raise a clear error.
- **FR-012**: Collision checks MUST NOT slow down frames for sprites and lists that aren't queried, and lists whose sprites haven't moved MUST NOT redo work on repeated queries.
- **FR-013**: The collision feature MUST work the same way in the browser build.
- **FR-014**: A Phase 5 example (`examples/05_collisions.py`) MUST let the player steer the ship among drifting asteroids with mixed collider shapes. Touched asteroids are removed, and the hit count is shown in the window title.
- **FR-015**: Type stubs MUST describe the collider classes, the `collider` property, and the queries.

### Key Entities

- **Collider**: a shape (circle, rotating box, axis-aligned box) with a size and an offset, attached to at most one sprite.
- **Sprite (extended)**: has an optional collider.
- **Collision result**: a list of sprites (single-sprite queries) or a list of sprite pairs (list queries), returned as a snapshot.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In automated tests, query results match a brute-force reference exactly for thousands of random shapes, positions, rotations, scales, and offsets (no missed or extra collisions).
- **SC-002**: On the development machine, finding all colliding pairs among **10,000 moving sprites in one list** takes **under 2 ms** per frame, and checking **one sprite against a list of 10,000** takes **under 0.5 ms** (owner target, 2026-10-12).
- **SC-003**: A list of sprites that haven't moved costs no extra time for repeated queries in the same frame (within measurement noise).
- **SC-004**: The example shows correct hits on desktop and in the browser, confirmed by automated checks.
- **SC-005**: A developer can add "ship hits asteroid" behavior to the Phase 4 example in under 10 lines of Python.

## Assumptions

- **One collider per sprite** in Phase 5. The design allows several later.
- **Touching counts as colliding**, matching Arcade and most engines' overlap tests.
- **A collider object belongs to one sprite at a time.** Attaching it elsewhere raises an error.
- **Result ordering:** `collides_with_list` returns sprites in list order; pair queries return pairs ordered by the first element's list position, then the second's.
- **Positions are checked as they are** when the query runs (after `SpriteList.update`). There's no interpolation or swept testing.
- **No contact details** (normal, depth, contact points) in this phase. Detection only.
- **The example** uses the Phase 4 controls, about 100–200 asteroids, and shows the hit count in the title (no text rendering yet).
- **Automated tests** use known geometry and a brute-force reference written in the test.
