# Ferrocade Constitution

## Core Principles

### I. Python API, Rust Core

- Game developers MUST be able to build complete games using only Python. The entire public API is
  Python calls, and users never write, read, or compile Rust.
- All engine internals (rendering, windowing, timing, sprite storage, collision) MUST live in the Rust
  core (`crates/ferrocade/`), which MUST NOT depend on Python or PyO3.
- The PyO3 bindings (`crates/ferrocade-py/`) MUST stay a thin wrapper over the core.

**Rationale:** Python gives users approachability, and Rust gives the engine its performance. A
Python-free core keeps the web build free to choose its own binding strategy.

### II. Minimize Python↔Rust Conversion

- Engine state (sprite positions, velocities, transforms) MUST be stored natively in Rust, never as
  Python attributes. Python reads and writes go through Rust getters/setters.
- `Sprite` stores `f64` (exact Python round-trips). Each `SpriteList` owns GPU-layout `f32` buffers,
  written by setters and tracked with dirty flags.
- Drawing MUST NOT perform per-sprite Python↔Rust conversion. Unchanged sprites cost zero Python time
  per frame.
- Per-frame bulk work (motion, collision checks) MUST run in Rust. APIs MUST offer batched forms
  (e.g. `position = (x, y)`, list-level `update()`) so common work needs few boundary crossings.
- Performance claims MUST be backed by the benchmark scenes defined in the engine spec.

**Rationale:** boxing and unboxing Python floats across the boundary every frame is the main
performance cost this engine exists to avoid.

### III. Portable by Design

- Supported platforms are **Windows, macOS, and web browsers (WASM)**. The prototype targets Windows,
  but no change may close off the macOS or web port.
- Platform-specific code MUST stay inside the platform layer. Engine code uses wgpu and winit only.
- Rendering MUST work within wgpu's WebGL2 downlevel limits by default. Shaders MUST be WGSL.
- The main loop MUST be event-driven (no blocking loop), and GPU initialization MUST be async-friendly.
- Code MUST NOT use `std::time::Instant` directly (use `web-time`) and MUST NOT require threads in the
  core.
- File access MUST go through the asset loader. Code MUST NOT assume a filesystem.

**Rationale:** these constraints are cheap to follow from day one and very expensive to retrofit.

### IV. 2D Only, Simple First

- Ferrocade is a 2D engine. Code MUST NOT add 3D abstractions, depth buffers, or generality "for
  later".
- Each feature MUST be built in the simplest form that meets its phase goal. Additional abstraction
  requires a concrete, current need.

**Rationale:** a small, focused engine that works beats a general one that doesn't. Scope creep is
the main risk to a from-scratch engine.

### V. Spec-Driven, User-Confirmed Decisions

- `docs/spec/ENGINE_SPEC.md` is the source of truth for architecture and design. Feature specs in
  `specs/` MUST align with it and reference it rather than duplicate it.
- Design decisions MUST be confirmed by the project owner before they are recorded as decided.
  Unconfirmed ideas MUST be marked *Proposed* or **TBD**.
- Every confirmed decision MUST be added to the engine spec (its section and its dated Decision Log).
- Open designs (currently the collision system, §11, and the browser Python runtime, §9) MUST NOT be
  settled during implementation without the owner's approval.

**Rationale:** the engine is being designed collaboratively and incrementally. A single written
record prevents drift and silent decisions.

### VI. Phase-Based Delivery

- Work proceeds through the roadmap phases in the engine spec (§10). Each phase is one Spec Kit
  feature (`specs/NNN-<phase-name>/`).
- Each phase MUST end with a runnable example in `examples/` that demonstrates its goal, plus its
  stated "done when" criteria met (or documented as unmet with reasons).
- A phase SHOULD leave the engine in a working state on Windows. Regressions in earlier phases'
  examples MUST be fixed before a phase is complete.

**Rationale:** small, demonstrable milestones keep the engine working end to end at every step.

## Technical Constraints

- **Stack:** Rust (stable, MSVC toolchain on Windows), wgpu (rendering), winit (windowing/input),
  PyO3 + maturin (bindings/packaging), `image` crate (decoding), WGSL (shaders).
- **Coordinates:** origin at the bottom-left, y up, units in pixels. Angles are in degrees.
- **Motion:** all rates are per second (px/s, deg/s). Velocity properties are named `velocity_x`,
  `velocity_y`, `angular_velocity`.
- **Timing:** fixed-timestep updates (default 60 Hz) with capped catch-up. Drawing happens once per
  displayed frame.
- **Rendering:** orthographic projection, no depth buffer, and draw order defines layering. Alpha
  blending is enabled.
- **Images:** PNG is required. Other formats are enabled only when nearly free (`image` crate features).
- **Collision:** detection MUST run in Rust. Its design is open (engine spec §11).
- **API style:** users subclass `Window` and override callbacks (`on_draw`, `on_update`, input
  callbacks).

## Development Workflow

- **Documents:**
  - `docs/spec/ENGINE_SPEC.md`: engine architecture and decision log.
  - `specs/NNN-*/`: per-phase feature spec, plan, and tasks (Spec Kit).
  - `docs/design/`: in-depth subsystem designs.
  - `docs/research/`: spike and benchmark results.
- **Feature flow:** `/speckit-specify` → `/speckit-clarify` (as needed) → `/speckit-plan` →
  `/speckit-tasks` → `/speckit-implement`. Plans MUST include a Constitution Check against these
  principles.
- **Testing:** Rust unit tests live in the crates. Python API tests live in `tests/`. Performance
  is verified with `benches/`. Each phase's example serves as its acceptance demo.
- **Version control:** keep commits small and focused. Generated build artifacts are never committed.

## Governance

- This constitution takes precedence over other project practices. `CLAUDE.md` and the engine spec
  MUST stay consistent with it.
- Amendments require the project owner's approval. Each amendment MUST be reflected in the engine
  spec's Decision Log.
- Versioning follows semantic versioning:
  - MAJOR: removing or redefining a principle.
  - MINOR: adding a principle or section, or materially expanding guidance.
  - PATCH: clarifications and wording.
- Compliance is checked in every plan's Constitution Check. Any violation MUST be justified in the
  plan's Complexity Tracking or resolved before implementation.

**Version**: 1.0.0 | **Ratified**: 2026-10-12 | **Last Amended**: 2026-10-12
