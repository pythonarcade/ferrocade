# Ferrocade: AI Assistant Instructions

Ferrocade is a 2D game engine built from scratch. Users write games in **Python**, and the backend is **Rust** (wgpu rendering, winit windowing, PyO3 bindings, maturin builds).

## Constitution (core rules)
The project's non-negotiable principles live in the Spec Kit constitution and are imported here:

@.specify/memory/constitution.md

## Source of truth
- **`docs/spec/ENGINE_SPEC.md`** is the living architecture spec and decision log. Read the relevant sections before making design or implementation decisions.
- When the user makes or confirms a decision, update the spec section **and** add a row to its Decision Log (dated). Mark unconfirmed ideas as *Proposed* or **TBD**. Never record a suggestion as decided without the user confirming it.
- **Spec Kit** handles per-phase work. Each roadmap phase (spec §10) is a feature in `specs/NNN-<name>/` (spec.md, plan.md, tasks.md), created with the `speckit-*` skills.
- Larger subsystem designs go in `docs/design/`, and spike or benchmark results go in `docs/research/`.

## Project layout
- `crates/ferrocade/`: Rust engine core (no Python dependency)
- `crates/ferrocade-py/`: PyO3 bindings (thin wrapper)
- `python/ferrocade/`: Python package (re-exports, helpers, `.pyi` stubs)
- `examples/`: one runnable example per roadmap phase (`01_centered_ship.py`, ...)
- `assets/`: images, sounds, fonts used by examples, tests, and benchmarks
- `tests/`: Python API tests. Rust tests live in the crates.
- `benches/`: performance scenes. `web/`: browser build. `tools/`: dev scripts.
- `specs/`: Spec Kit feature specs. `.specify/`: Spec Kit config, templates, scripts, constitution.

## Environment
- Development machine: Windows 11, PowerShell (Git Bash also available).
- Toolchain: Rust stable (`x86_64-pc-windows-msvc`, via rustup), Visual Studio Build Tools 2026 (MSVC + Windows SDK), Python 3.13, uv.
- Spec Kit Python scripts may crash on Windows with console encoding errors. Run them with `PYTHONIOENCODING=utf-8` (or set `PYTHONUTF8=1`).
- The Claude Code shell may not have `~/.cargo/bin` on PATH. Prefix commands with `export PATH="$HOME/.cargo/bin:$PATH"` (Git Bash) if `cargo` isn't found.

## Build & test
Run these from the repository root (PowerShell shown; the Git Bash equivalents are similar):

```powershell
uv venv; .venv\Scripts\Activate.ps1; uv pip install maturin pytest   # one-time setup
maturin develop                 # build + install the extension into .venv (after every Rust change)
cargo test --workspace          # Rust tests, incl. offscreen GPU render checks (skip without a GPU)
pytest                          # Python API tests; set FERROCADE_HEADLESS=1 to skip real-window tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
python examples/01_centered_ship.py
```

The full validation guide for Phase 1 is in `specs/001-centered-ship/quickstart.md`.
