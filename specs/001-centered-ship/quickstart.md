# Quickstart & Validation: Phase 1 — Window and Centered Ship

**Feature**: [spec.md](spec.md) | **API**: [contracts/python-api.md](contracts/python-api.md)

This guide covers how to build Ferrocade from source and verify that Phase 1 works end to end.
Run all commands from the repository root in PowerShell.

## Prerequisites

| Tool | Check | Notes |
|---|---|---|
| Rust (stable, MSVC) | `cargo --version` | Install via rustup |
| Visual Studio Build Tools (C++ workload) | `cargo build` links successfully | Provides the MSVC linker and Windows SDK |
| Python 3.13+ | `python --version` | |
| uv | `uv --version` | Creates the virtual environment and installs tools |

## Setup (once)

```powershell
uv venv                                  # creates .venv (Python 3.13+)
.venv\Scripts\Activate.ps1
uv pip install maturin pytest
```

## Build & install (after every Rust change)

```powershell
maturin develop            # debug build, installs `ferrocade` into .venv
# maturin develop --release   # optimized build, used for performance checks
```

Expected result: `python -c "import ferrocade as fc; print(fc.__version__)"` prints a version.

## Validation scenarios

| # | Scenario | Command | Expected outcome | Covers |
|---|---|---|---|---|
| V1 | Rust unit and offscreen render tests | `cargo test -p ferrocade` | All pass. The offscreen test reports the ship's bounding box centered within 1 px (or is skipped with "no GPU adapter"). | FR-008, FR-014, FR-017; SC-002, SC-008 |
| V2 | Python API tests | `pytest tests` | All pass (display-dependent tests may be skipped in headless environments). | FR-001 to FR-021; SC-006, SC-008 |
| V3 | Centered ship (milestone) | `python examples/01_centered_ship.py` | An 800×600 window titled "Ferrocade" appears within 2 s, with a dark gray background and the ship in the exact center at its natural size (99×75), with crisp pixel edges (the example uses `filter=fc.NEAREST`) and transparent areas showing the background. | US1, US2; SC-002, SC-003, SC-007 |
| V4 | Clean close | Close V3's window with the X button | The script exits within 1 s with exit code 0 (`$LASTEXITCODE` is `0`) and no output on stderr. | FR-006; SC-004 |
| V5 | Resize and minimize | During V3, resize the window 20 times, then minimize and restore it 10 times | No crash, freeze, or error output. The ship keeps its coordinates (it is not stretched, and not re-centered). | Edge cases; SC-005 |
| V6 | Display scaling | Set Windows display scaling to 150% and rerun V3 | The window is 800×600 logical (larger in physical pixels), and the ship is still centered and crisp relative to the window. | Edge case (logical pixels) |
| V7 | Bad image path | `python -c "import ferrocade as fc; fc.Texture('nope.png')"` | `FileNotFoundError` whose message includes `nope.png`. | FR-010; SC-006 |
| V7b | Filter is required | `python -c "import ferrocade as fc; fc.Window(800, 600, 'x')"` | `TypeError` mentioning the missing `filter` argument. | FR-022 |
| V8 | Exception in a callback | Run a window whose `on_draw` raises `ValueError("boom")` | The window closes, and `run()` raises `ValueError: boom` with a traceback that points to the user's `on_draw` line. | Edge case (callback errors) |
| V9 | Ctrl+C | Run V3 from a terminal and press Ctrl+C | The window closes and `KeyboardInterrupt` is raised. | Edge case |
| V10 | New-developer check | Someone unfamiliar with the project follows this guide from a fresh clone | They get V3 running without help. | SC-009 |

## Troubleshooting

- **The link step fails with "link.exe returned an unexpected error"**: the MSVC Build Tools are missing, or Git's `link.exe` is being used instead. Install or repair the "Desktop development with C++" workload.
- **Spec Kit scripts crash with a `UnicodeEncodeError`**: set `PYTHONUTF8=1`.
- **There's no GPU adapter (VM or remote session)**: wgpu falls back to software rendering if one is available. Otherwise the offscreen test is skipped and V3 can't run on that machine.
