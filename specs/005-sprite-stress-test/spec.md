# Feature Specification: Phase 3 — Sprite Stress Test (Performance)

**Feature Branch**: `005-sprite-stress-test`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Phase 3"

**Context**: This is roadmap Phase 3 (`docs/spec/ENGINE_SPEC.md` §10): *draw and move a large number
of sprites at a steady frame rate*, proving that the performance design (engine spec §7: batching,
data kept in the engine, minimal Python↔engine conversion) pays off.

The roadmap names four scenes:
- N static sprites
- N moved by the engine's list update
- N moved from a Python loop
- (optionally) N moved via bulk array views

It also says to compare against Python Arcade and Pygame. The targets were deliberately left "TBD,
set after first measurements".

Phases 1c and 2 already measured:
- 1,000 shapes at 60 FPS (vsync-limited)
- 0.087 ms to move 1,000 sprites in the engine, vs. 1.34 ms for Python per-sprite checks

## User Scenarios & Testing *(mandatory)*

There are two users:
- the **project owner**, who needs evidence of how far the engine scales and where it stops
- **game developers**, who want to know how many sprites they can use and which coding pattern is fast

### User Story 1 - Find the engine's limits, scene by scene (Priority: P1)

The owner runs a stress-test program that ramps the number of sprites up in each scene until the
frame rate can no longer hold the display rate. It reports the maximum sustainable count per scene,
plus a breakdown of where frame time goes (moving, drawing, game code).

**Why this priority**: it's the phase's core question: how many sprites, and what limits it?

**Independent Test**: run the stress test. For each scene it prints a maximum sprite count at the
target frame rate, and a time breakdown.

**Acceptance Scenarios**:

1. **Given** the static scene, **When** the sprite count increases, **Then** the tool reports the
   highest count that still renders at the display refresh rate.
2. **Given** the engine-moved scene, **When** it runs, **Then** the tool reports the maximum count
   and the per-frame time spent moving and drawing.
3. **Given** the Python-loop scene, **When** it runs, **Then** the tool reports the maximum count
   and how much of each frame is spent in game code.
4. **Given** any scene, **When** a limit is reached, **Then** the report names the dominant cost
   (drawing, engine update, or game code), so bottlenecks are documented rather than guessed.

---

### User Story 2 - Compare with other Python engines on the same scenes (Priority: P2)

The owner runs equivalent scenes in other Python game engines on the same machine, and sees
Ferrocade's results side by side with theirs.

**Why this priority**: the comparison gives the results meaning and shows whether the design goal
(beating CPU-bound Python engines) is met. It depends on US1's scenes existing.

**Independent Test**: run the comparison scripts. A table shows each engine's maximum sprite count
per scene, on the same machine and window size.

**Acceptance Scenarios**:

1. **Given** the same scene definitions (sprite image, window size, motion rules), **When** run in
   each compared engine, **Then** results are recorded with the engine version and machine.
2. **Given** the comparison table, **When** read, **Then** it's clear which scenes Ferrocade wins,
   loses, or ties, and by roughly how much.

**Compared engines (owner decision, 2026-10-12):** **Python Arcade** (the GPU-batched peer) and
**Pygame** (the CPU-blitting baseline). They're installed only in the development environment, not as Ferrocade dependencies.

---

### User Story 3 - A visible frame-rate readout for developers (Priority: P3)

A developer running any Ferrocade game can turn on a frame-rate and frame-time readout, without
writing their own timing code.

**Why this priority**: useful for every later phase and for the showcase, but not required to answer
the phase's question (the stress tool reports its own numbers).

**Independent Test**: turn the readout on in an example. Frame rate and frame time appear and update
about once a second.

**Acceptance Scenarios**:

1. **Given** any window, **When** the developer enables the readout, **Then** the current FPS and
   average frame time are shown (in the window title, until text rendering exists) and update
   about once per second.
2. **Given** the readout is off (the default), **When** the game runs, **Then** nothing changes.

---

### Edge Cases

- **Vsync caps the frame rate:** frame time is capped at the refresh interval, which hides
  headroom. The tool must also measure *work time* (CPU time per frame, excluding the wait for
  vsync), so results are meaningful below the cap.
- **Startup and warm-up:** measurements exclude the first frames (window creation, shader
  compilation, the first texture upload).
- **Thermal and background noise:** each measurement is repeated or held long enough to be stable.
  The results note the machine and power settings.
- **The browser:** the same scenes can run on the web build for a web column, but the web is
  informational in this phase (the per-call cost is a known risk being measured, not a target).
- **Engines that can't do a scene** (for example, no bulk-array path): marked "n/a", not zero.

## Requirements *(mandatory)*

### Functional Requirements

**Scenes**

- **FR-001**: The stress test MUST provide these scenes, all using the same small image sprite
  (`ship.png`, scaled down) in an 800×600 window:
  - (a) **static**: N sprites that never move
  - (b) **engine-moved**: N sprites moved each update by the list update, bouncing off the edges in
    the engine-friendly way (see Assumptions)
  - (c) **Python-moved**: N sprites whose positions are updated by a Python loop each update
  - (d) **shapes**: N mixed rectangles and circles moved by the engine
- **FR-002**: Bulk array access (engine spec §7, "NumPy views") is **deferred** to the backlog
  *(owner decision, 2026-10-12)*. This phase measures the existing movement paths only.
- **FR-003**: For each scene, the tool MUST find the **maximum sustainable sprite count** at the
  target frame rate by ramping N up (and refining), and report it.

**Measurement**

- **FR-004**: The tool MUST report, per scene and N: the frame rate, the average and 95th-percentile
  frame time, and the **work time** per frame split into engine update, game code, and drawing
  submission, measured independently of the vsync wait.
- **FR-005**: Results MUST be written to a machine-readable file and to a human-readable summary,
  including the machine description, engine version, and date.
- **FR-006**: **Target (owner decision, 2026-10-12): 10,000 engine-moved sprites at 60 FPS** on
  the development machine (scene b). The static, Python-moved, and shapes scenes are measured and
  reported, but they aren't pass/fail.

**Comparison**

- **FR-007**: Equivalent scenes MUST be implemented for each compared engine (see the US2
  clarification), with matching window size, sprite image, and motion rules. Results go in the same table.

**Readout**

- **FR-008**: Windows MUST offer an opt-in frame-rate and frame-time readout, shown in the window
  title and updated about once per second. It's off by default.

**Delivery**

- **FR-009**: `benches/` MUST contain the stress-test tool and comparison scripts, with documented
  commands. `docs/research/phase3-benchmarks.md` MUST record the results and the bottleneck analysis.
- **FR-010**: If a target is missed, the report MUST identify the bottleneck with measurements, and
  propose concrete fixes (constitution VI: "met, or profiled and documented").
- **FR-011**: All earlier behavior and tests MUST remain unchanged.

### Key Entities *(include if feature involves data)*

- **Scene**: a named workload definition (sprite type, motion source, bounce rule).
- **Measurement**: one run of a scene at N sprites: frame stats, work-time breakdown, machine, and versions.
- **Result table**: the maximum sustainable N per scene and engine, plus the bottleneck notes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every scene has a recorded maximum sustainable sprite count on the development
  machine, together with a frame-time breakdown.
- **SC-002**: The phase's targets (FR-006) are met, or each miss is explained by measured
  bottlenecks with proposed fixes.
- **SC-003**: The comparison table covers every scene for each compared engine, or marks a scene
  "n/a" with the reason.
- **SC-004**: Running the full stress test takes no more than 10 minutes and needs one command.
- **SC-005**: The frame-rate readout can be enabled with one line and adds no measurable cost when off.

## Assumptions

- **The dev machine is the reference** (Windows 11, RTX 3500 Ada laptop GPU). The results note this.
  "Mid-range hardware" targets are interpreted relative to it.
- **Engine-friendly bouncing for scene (b):** edge bounces must not require per-sprite Python work,
  or the scene would measure Python rather than the engine. Phase 3 therefore needs either a minimal
  engine-side edge bounce or bounds handling for the stress scene. That's a small engine addition,
  decided in planning.
- **Vsync:** the measurement uses work time (and, where possible, a vsync-off presentation mode for
  the measurement build) so headroom above 60 FPS is visible.
- **Web measurements** are informational only.
- **No new rendering features:** this phase measures and, if needed, optimizes existing paths.
