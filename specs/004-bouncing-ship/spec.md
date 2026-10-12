# Feature Specification: Phase 2 — Bouncing Ship (Movement)

**Feature Branch**: `004-bouncing-ship`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Phase 2: bouncing ship. The ship moves and bounces off the window edges, proving the fixed-timestep loop, on_update(delta_time), per-second velocities (velocity_x, velocity_y, angular_velocity) applied in Rust by SpriteList.update(), and dirty-buffer updates (engine spec §8, §10 Phase 2)."

**Context**: This is roadmap Phase 2 (`docs/spec/ENGINE_SPEC.md` §10). These points are already decided:
- **Motion units** are per second: pixels/second and degrees/second.
- **Updates** run on a fixed timestep, 60 per second by default.
- **Velocity names** are `velocity_x`, `velocity_y`, and `angular_velocity`.
- **Sprite data** lives in the engine, and per-frame bulk work runs in the engine, not in game code
  (engine spec §7, §8).

Phase 2 makes sprites move. Because `angular_velocity` is part of this phase, sprites also need to
be **drawn rotated**. Until now the angle was only stored.

## User Scenarios & Testing *(mandatory)*

The user is a **game developer** writing Python with Ferrocade.

### User Story 1 - Make sprites move with velocities (Priority: P1)

A developer gives a sprite a velocity, for example 200 pixels per second to the right and 150
upward. In the update callback they ask the sprite list to update, and every sprite moves according
to its velocity. The developer doesn't move sprites one at a time.

**Why this priority**: movement is the core of the phase and of almost every game.

**Independent Test**: give one sprite a known velocity, run for a measured time, and confirm its
position changed by velocity × time.

**Acceptance Scenarios**:

1. **Given** a sprite with `velocity_x = 200` and `velocity_y = 0`, **When** its list is updated
   once per update for 1 second, **Then** it has moved 200 pixels to the right (within 1 pixel).
2. **Given** a sprite with a velocity, **When** its list is *not* updated, **Then** the sprite
   doesn't move. Movement only happens when the developer asks.
3. **Given** a new sprite, **When** its velocities are read, **Then** they are all 0.
4. **Given** 1,000 moving sprites in one list, **When** the list is updated each update,
   **Then** the game stays at the display refresh rate on the development machine.
5. **Given** sprites with velocities, **When** they move, **Then** the drawn positions on the next
   frame match the updated positions.

---

### User Story 2 - Bounce the ship off the window edges (Priority: P2)

A developer writes the Phase 2 example: the ship starts in the middle with a diagonal velocity and
bounces off all four window edges, forever. The game code checks the edges and reverses the
velocity. The engine does the movement.

**Why this priority**: it's the phase's milestone demo, showing that game logic in Python and
movement in the engine work together.

**Independent Test**: run `examples/02_bouncing_ship.py`. The ship moves smoothly, never leaves
the window, and bounces off every edge.

**Acceptance Scenarios**:

1. **Given** the example running, **When** the ship reaches any edge, **Then** it reverses
   direction on that axis and stays fully inside the window.
2. **Given** the example running for 60 seconds, **When** observed, **Then** the ship has never
   left the window, and its speed looks constant.
3. **Given** the same example in the browser, **When** it runs, **Then** it behaves the same way.

---

### User Story 3 - Movement speed doesn't depend on frame rate (Priority: P3)

A developer's game runs at the same speed on a 60 Hz monitor, a 144 Hz monitor, and a slow machine
that drops frames. Updates happen at a fixed rate, independent of drawing.

**Why this priority**: it's essential for correctness, but it's mostly guaranteed by the engine's
fixed timestep. This story makes it visible and verified.

**Independent Test**: count updates and measure movement over a fixed wall-clock period, under
normal conditions and while drawing is artificially slowed down.

**Acceptance Scenarios**:

1. **Given** the default update rate, **When** the game runs for 10 seconds, **Then** the update
   callback has run 600 times (±2%), whatever the display refresh rate.
2. **Given** drawing that is artificially slowed to about 20 frames per second, **When** the game
   runs for 10 seconds, **Then** a moving sprite has covered the same distance (±2%) as at full
   frame rate.
3. **Given** a long stall (for example, the window being dragged for 2 seconds), **When** the game
   resumes, **Then** it doesn't try to "catch up" with a burst of hundreds of updates (at most 5
   updates run before the next draw, so the game skips ahead calmly).

---

### User Story 4 - Rotating sprites (Priority: P4)

A developer sets a sprite's angle, or gives it an angular velocity, and sees it drawn rotated around
its center, spinning steadily when it has an angular velocity.

**Why this priority**: `angular_velocity` is part of the phase's motion model, and it's only useful
if rotation is visible. It's lower priority than linear motion.

**Independent Test**: draw a sprite at angle 90 and confirm it's rotated a quarter turn in the
documented direction. Give it an angular velocity of 90 per second, and confirm it has turned 90
degrees after 1 second.

**Acceptance Scenarios**:

1. **Given** a sprite with `angle = 90`, **When** drawn, **Then** it appears rotated a quarter turn
   around its center, in the direction defined by FR-009.
2. **Given** a sprite with `angular_velocity = 90`, **When** its list is updated for 1 second,
   **Then** its angle has increased by 90 (within 1 degree).
3. **Given** rotated shape sprites (rectangles), **When** drawn, **Then** they rotate exactly like
   image sprites. Circles look unchanged, as expected.

---

### Edge Cases

- **Non-finite velocities** (NaN or infinity): rejected with a clear error, like positions.
- **Very large velocities** (e.g. 100,000 px/s): the sprite moves that far. It isn't clamped.
  Keeping it in bounds is the game's job.
- **Negative velocities**: move left or down (y points up).
- **A sprite in two lists, with both lists updated**: it moves twice per update. Updating is per
  list, and that's the developer's responsibility (it's documented).
- **Angles beyond 360 or negative**: allowed. They're drawn equivalently and not wrapped (reading
  `angle` returns exactly what accumulated).
- **The update rate changes the step**: with `update_rate = 1/30`, each update moves sprites by
  velocity / 30. The per-second speed is the same.
- **The window is minimized**: updates may pause or skip ahead safely (at most 5 per frame). The
  game neither crashes nor races ahead.
- **Bounce at the exact edge**: the example keeps the ship fully inside (it corrects the position as
  well as flipping the velocity), so the ship never "sticks" to an edge.

## Requirements *(mandatory)*

### Functional Requirements

**Velocities**

- **FR-001**: Every sprite (image or shape) MUST have readable and writable `velocity_x` and
  `velocity_y` (pixels per second) and `angular_velocity` (degrees per second), all defaulting to 0.
- **FR-002**: Velocity values MUST round-trip exactly, and MUST reject non-finite numbers with a
  clear error.
- **FR-003**: Developers MUST be able to set both linear velocity components in one operation
  (`velocity = (vx, vy)`), mirroring `position`.

**Updating**

- **FR-004**: A sprite list MUST provide an update operation that takes the time step in seconds and
  advances every sprite in the list: position += velocity × time step, and angle += angular velocity
  × time step.
- **FR-005**: The update MUST be performed entirely by the engine in one call per list. Its cost must
  not grow with per-sprite work in game code.
- **FR-006**: Updating MUST be explicit. Sprites move only when their list is updated (typically from
  the update callback, using the time step it receives).
- **FR-007**: Positions changed by an update MUST appear on the next drawn frame, with the same
  efficiency rules as other changes. Only changed sprites are re-sent for drawing, and sprites with
  zero velocity cost nothing to draw again.

**Timing**

- **FR-008**: The update callback MUST run at the window's fixed update rate (default 60 per second),
  independently of the display refresh rate, and receive the constant time step. After a stall, at
  most 5 updates may run before the next draw, and the backlog beyond that is dropped.

**Rotation**

- **FR-009**: Sprites MUST be drawn rotated by their `angle` (degrees) around their center.
  A positive angle rotates **counter-clockwise** (the math convention, consistent with the y-up
  coordinate system: a sprite heading at `angle` moves along `(cos(angle), sin(angle))`).
  *(Owner decision, 2026-10-12, matching Unity and Bevy rather than Arcade 3.x.)*
- **FR-010**: Rotation MUST apply equally to image sprites and shape sprites, and MUST NOT change a
  sprite's reported `width`/`height` (they stay unrotated sizes).

**Delivery**

- **FR-011**: `examples/02_bouncing_ship.py` MUST show the ship bouncing off all four edges using a
  velocity, the list update, and edge checks in the update callback.
- **FR-012**: The example MUST also offer a mode with many moving, bouncing sprites (e.g. 1,000
  shapes) that prints the frame time, as a stepping stone to the performance showcase.
- **FR-013**: All of the above MUST work the same way in the browser build.
- **FR-014**: The type information for editors MUST cover the new properties and the update
  operation.
- **FR-015**: All Phase 1 and 1c behavior MUST stay unchanged (existing tests keep passing).

### Key Entities *(include if feature involves data)*

- **Sprite motion state**: per sprite, `velocity_x`, `velocity_y` (px/s) and `angular_velocity`
  (deg/s). Stored with the rest of the sprite's data in the engine.
- **Sprite list update**: one operation per list per step. It advances all members by
  velocity × time step.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A sprite with velocity v moves v × t pixels over t seconds of updates, within 1 pixel
  for t = 1 s (verified automatically).
- **SC-002**: Over 10 seconds, the update callback runs 600 times ±2% at the default rate, both at
  full frame rate and with drawing slowed to about 20 frames per second.
- **SC-003**: 1,000 moving, edge-bouncing sprites (moved by the engine, with bounces checked in game
  code) run at the display refresh rate on the development machine.
- **SC-004**: The bouncing-ship example runs for 60 seconds without the ship ever leaving the window.
- **SC-005**: A sprite at angle 90 is drawn as an exact quarter turn counter-clockwise
  (verified by pixel check), and a spinning sprite turns 90 degrees per second at `angular_velocity = 90`.
- **SC-006**: Moving a sprite with velocities requires no per-sprite code in the update callback
  beyond the single list update (plus game logic such as edge checks).
- **SC-007**: The example behaves the same way in the browser (the ship bounces and stays inside).

## Assumptions

- **Explicit update call:** developers call the list update in their update callback (Arcade-style).
  The engine does not update lists automatically. A list's update only moves its own sprites.
- **Edge bouncing is game code** in this phase (Python checks the edges and flips the velocity).
  Engine-side responses (built-in bounce, walls) come with collisions (Phase 5 and the showcase).
- **No interpolation** between updates in this phase. Drawing shows the latest updated positions.
  Smoothing for high refresh-rate displays is a later improvement (engine spec §8).
- **Pivot:** rotation is around the sprite's center. Custom pivots are out of scope.
- **Angle:** stored as accumulated degrees (no wrapping).
- **No acceleration, friction, or gravity** in this phase. Those are later physics features.
