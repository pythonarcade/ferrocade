# Feature Specification: Performance Showcase

**Feature Branch**: `008-performance-showcase`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Performance showcase milestone (engine spec §10, after Phase 5)." (The command was run without arguments right after the owner decided the showcase options; the description comes from the roadmap and those decisions.)

**Context**: This is the milestone after Phase 5 (`docs/spec/ENGINE_SPEC.md` §10). These points are already decided by the owner (2026-10-12):
- **Scene:** a window with a wall around the border and many small balls, all moving and bouncing off the walls and each other. It shows **rendering, movement, and collision speed**.
- **Bouncing:** in the engine by default, through **explicit calls each update** (for example "bounce these balls off those walls" and "bounce these balls off each other"). A key switches to bouncing **in Python** from the engine's batched collision results, so the difference is visible.
- **Physics:** **equal-mass, perfectly elastic circles** bouncing off each other and off **flat (rectangular) walls**.
- **Numbers:** printed to the **console about once per second**. There's no on-screen display in this milestone.
- **Timings come from the engine:** the frame statistics gain engine-measured **update** and **collision** times, next to the existing render time.
- Movement and detection always run in the engine.

**Out of scope**: on-screen text or graphs, enter/exit collision events, mass or bounciness settings, bouncing boxes or rotation from impacts, declared collision pairs, and side-by-side runs against other engines.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Thousands of balls bouncing, handled by the engine (Priority: P1)

A viewer runs the showcase and sees a walled window full of small colored balls flying around, bouncing off the walls and off each other, smoothly at the display's frame rate. The game code asks the engine to do the bouncing with two calls per update.

**Why this priority**: this is the showcase itself.

**Independent Test**: run the scene with a known number of balls and check, automatically, that no ball leaves the walled area, that colliding balls separate and change direction, and that total motion energy and momentum are conserved within a small tolerance.

**Acceptance Scenarios**:

1. **Given** two balls moving straight at each other with equal speed, **When** they collide head-on, **Then** they move apart with their speeds swapped (equal-mass elastic collision).
2. **Given** a ball hitting a wall at an angle, **When** it bounces, **Then** its speed is unchanged and only the velocity part pointing into the wall is reversed.
3. **Given** two balls that start out overlapping, **When** the bounce step runs, **Then** they're pushed apart so they no longer overlap, rather than sticking together.
4. **Given** 10,000 balls in the scene, **When** it runs for 10 seconds, **Then** no ball ends up outside the walls, and the total kinetic energy stays within 1% of its starting value.
5. **Given** balls that don't touch anything, **When** the bounce step runs, **Then** their motion is unaffected.

---

### User Story 2 - See where the time goes (Priority: P1)

About once per second, the showcase prints a line to the console: the number of balls, the frame rate, and the time per frame spent rendering, moving, and handling collisions (detection plus bouncing), as measured by the engine. Any game can read the same engine timings.

**Why this priority**: showing the three speeds is the purpose of the milestone.

**Independent Test**: run any window, read the frame statistics, and check that the update and collision times are present, non-negative, and grow when more balls are added. Check that the showcase prints a line about every second.

**Acceptance Scenarios**:

1. **Given** the showcase is running, **When** a second passes, **Then** one console line shows ball count, FPS, render ms, update ms, and collision ms, plus the bouncing mode.
2. **Given** a game that doesn't use collisions, **When** it reads the frame statistics, **Then** the collision time is 0.
3. **Given** the ball count doubles, **When** the next lines print, **Then** the update and collision times increase accordingly.

---

### User Story 3 - Compare engine bouncing with Python bouncing (Priority: P2)

The viewer presses a key to switch bouncing from the engine to Python: the same physics, computed in Python from the engine's batched collision results. The console shows the bouncing mode and how the frame time changes. Pressing the key again switches back.

**Why this priority**: it shows the value of the hybrid design (the heavy work in the engine). The scene works without it.

**Independent Test**: run the same starting scene for the same number of updates in each mode and check that the results match within a small tolerance (the same physics). Check that the Python mode's time is reported.

**Acceptance Scenarios**:

1. **Given** the showcase in engine mode, **When** the viewer presses the toggle key, **Then** bouncing switches to Python, and the console line says "Python" and shows the time spent in the Python bounce code.
2. **Given** the same starting positions, **When** each mode runs 60 updates, **Then** the balls end up in the same places (within a small tolerance).

---

### User Story 4 - Add and remove balls, pause (Priority: P2)

The viewer presses keys to add 1,000 balls, remove 1,000 balls, and pause or resume, to explore where the frame rate starts to drop.

**Why this priority**: it makes the showcase interactive and shows the limits. It's optional for the core demo.

**Independent Test**: with injected key presses, check that the ball count changes by 1,000 per press (never below 0) and that pause stops all movement.

**Acceptance Scenarios**:

1. **Given** 5,000 balls, **When** the viewer presses the add key, **Then** there are 6,000 balls, and new balls appear inside the walls without overlapping the walls.
2. **Given** 500 balls, **When** the viewer presses the remove key, **Then** there are 0 balls and the showcase keeps running.
3. **Given** the showcase is running, **When** the viewer presses pause, **Then** all balls stop; pressing it again resumes motion.

---

### User Story 5 - The showcase in the browser (Priority: P3)

The same showcase runs in the browser. The numbers print to the browser's developer console.

**Why this priority**: the web is a supported platform. The browser's numbers are reported, not judged against a target.

**Independent Test**: the headless browser check loads the showcase, confirms balls move and bounce, and reads at least one stats line from the console.

**Acceptance Scenarios**:

1. **Given** the browser build, **When** the showcase loads, **Then** balls bounce inside the walls, and stats lines appear in the console about once per second.

### Edge Cases

- **Fast balls** at the chosen speeds don't pass through walls or each other between updates. Speeds are limited so a ball moves less than its own radius per update.
- **Several balls touching at once** (clusters): each touching pair is resolved once per update. Small leftover overlaps are allowed and resolve over the following updates, without balls sticking or exploding apart.
- **A ball touching a wall and another ball at the same time:** both are handled, and the ball stays inside the walls.
- **Zero balls:** the showcase runs and prints zeros.
- **Window resized:** the walls stay where they were created (the scene has a fixed size). Resizing doesn't break the simulation.
- **Bounce calls with sprites that have no circle collider** (for example a box): those sprites aren't bounced (see Assumptions). Walls may use any collider shape.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The engine MUST provide an explicit call to bounce a list of sprites off a list of walls: on contact, the velocity component into the wall is reversed (perfectly elastic), and the sprite is moved out of the wall.
- **FR-002**: The engine MUST provide an explicit call to bounce the sprites of one list off each other as equal-mass, perfectly elastic circles: colliding pairs exchange the velocity components along the line between their centers, and overlapping pairs are pushed apart equally.
- **FR-003**: Bouncing MUST use the sprites' circle colliders for the moving sprites and any collider shape for walls. Walls never move.
- **FR-004**: Each bounce call MUST be one call from the game per list (or list pair) per update. No per-pair game code.
- **FR-005**: The frame statistics MUST include the engine's update time (moving sprites) and collision time (all collision queries and bounce calls) per frame, averaged like the existing values. They're zero when unused.
- **FR-006**: The showcase MUST print one console line about every second with: ball count, FPS, render time, update time, collision time, the bouncing mode, and (in Python mode) the time spent in the Python bounce code.
- **FR-007**: The showcase MUST offer a key that switches between engine bouncing and Python bouncing. The Python mode MUST implement the same physics using the engine's batched collision queries.
- **FR-008**: The showcase MUST offer keys to add 1,000 balls, remove 1,000 balls (not below 0), and pause or resume.
- **FR-009**: The scene MUST be a fixed-size walled area (walls built from rectangle sprites around the border) with small colored balls of random positions, colors, directions, and speeds.
- **FR-010**: The showcase MUST start with a default ball count that runs at the display's frame rate on the development machine in engine mode.
- **FR-011**: The showcase MUST run unchanged in the browser build, with the stats in the browser console.
- **FR-012**: The new engine calls and statistics MUST be described in the type stubs and the browser version.

### Key Entities

- **Ball**: a small circle sprite with a circle collider and a velocity.
- **Wall**: a rectangle sprite with a box collider that never moves.
- **Frame statistics (extended)**: adds update time and collision time per frame.
- **Stats line**: the once-per-second console summary.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In engine mode, **10,000 balls** bounce at **60 FPS** on the development machine, with the per-frame work (update + collision + render) under the Phase 3 sustainable budget (14 ms).
- **SC-002**: Over 10 seconds of simulation, total kinetic energy stays within 1% of its starting value and no ball leaves the walled area (automated).
- **SC-003**: Head-on and angled test collisions match the equal-mass elastic formulas within 1e-9 (automated).
- **SC-004**: Engine mode and Python mode produce the same ball positions after 60 updates from the same start, within 1e-6 px.
- **SC-005**: The console shows a stats line at least once every 1.5 seconds while running.
- **SC-006**: The maximum ball count at 60 FPS is measured and reported for engine mode, Python mode, and the browser (engine mode). Only engine mode on desktop has a target.

## Assumptions

- **Defaults for the owner's open items 6–7** (not objected to): keys to add/remove 1,000 balls and to toggle and pause; a target of 10,000 balls at 60 FPS in engine mode on desktop, with browser numbers reported only; this feature number with `examples/06_showcase.py`; no side-by-side comparison with other engines.
- **Key choices:** `+`/`=` add 1,000, `-` removes 1,000, `T` toggles the bouncing mode, `Space` pauses.
- **Ball sizes:** radius 3–6 px. All balls have **equal mass** regardless of size (owner decision: equal-mass physics).
- **Ball speeds:** 50–250 px/s, so balls move less than their radius per update at 60 Hz.
- **Walls:** four rectangles, 10 px thick, around an 800×600 area matching the window.
- **Only sprites with circle colliders are bounced.** Others in a bounce call are skipped. Walls can be any collider shape.
- **Contact resolution is one pass per update** over the touching pairs found by the engine's collision queries, in a fixed order, so results are deterministic and the same in both modes.
- **The browser** prints with the standard console. Its numbers are informational.
