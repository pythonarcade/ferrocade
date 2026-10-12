# Ferrocade: AI Assistant Instructions

Ferrocade is a 2D game engine built from scratch. Users write games in **Python**, and the backend is **Rust** (wgpu rendering, winit windowing, PyO3 bindings, maturin builds).

## Source of truth
- **`docs/spec/ENGINE_SPEC.md`** is the living spec. Read it before making design or implementation decisions.
- When the user makes or confirms a decision, update the spec section **and** add a row to its Decision Log (dated). Mark unconfirmed ideas as *Proposed* or **TBD**. Never record a suggestion as decided without the user confirming it.
- Larger subsystem designs go in `docs/design/`, and spike or benchmark results go in `docs/research/`.

## Project layout
- `crates/ferrocade/`: Rust engine core (no Python dependency)
- `crates/ferrocade-py/`: PyO3 bindings (thin wrapper)
- `python/ferrocade/`: Python package (re-exports, helpers, `.pyi` stubs)
- `examples/`: one runnable example per roadmap phase (`01_centered_ship.py`, ...)
- `assets/`: images, sounds, fonts used by examples, tests, and benchmarks
- `tests/`: Python API tests. Rust tests live in the crates.
- `benches/`: performance scenes. `web/`: browser build. `tools/`: dev scripts.

## Core rules (see spec for details)
- **2D only.** Don't add 3D abstractions.
- **Minimize Python↔Rust number conversion** (§7): sprite data lives in Rust (`f64`), `SpriteList` owns GPU-layout `f32` buffers with dirty tracking, and per-frame work happens in Rust.
- **Coordinates:** bottom-left origin, y up, pixels. **Motion:** per second. **Updates:** fixed timestep (default 60 Hz).
- **Portability** (§9): the prototype targets Windows, but code must stay portable to macOS and web.
  - No platform-specific code outside the platform layer.
  - WebGL2-level wgpu limits, WGSL shaders only.
  - No blocking main loop, async-friendly GPU init.
  - Use `web-time` instead of `std::time::Instant`, no required threads.
  - Load files through an asset loader.
- Collision detection runs in Rust. Its design is still open (§11), so don't pick one without the user.

## Environment
- Development machine: Windows 11, PowerShell (Git Bash also available).
- Build/test commands: TBD (to be added in Phase 1).
