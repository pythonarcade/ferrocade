# Phase 1 Validation Record

**Feature**: [specs/001-centered-ship](../../specs/001-centered-ship/spec.md) | **Guide**: [quickstart.md](../../specs/001-centered-ship/quickstart.md)
**Date**: 2026-10-12

**Machine**: Windows 11 Pro 10.0.26200, NVIDIA RTX 3500 Ada Generation Laptop GPU, rustc 1.99.0, Python 3.14.4 (the venv's interpreter; the wheel is abi3 for 3.13+)

## Results

| # | Scenario | Result | Notes |
|---|---|---|---|
| V1 | Rust tests | ✅ Pass | 27 tests. The 6 offscreen GPU render tests ran (not skipped) and checked: ship centered within 1 px at 99×75, transparent texels show the background, bottom-left origin, 10 sprites sharing a texture, a moved sprite appears at its new position on the next frame, later sprites draw on top, and a linear override smooths while nearest doesn't. |
| V2 | Python API tests | ✅ Pass | 43 tests, including real-window display tests (callbacks, exception propagation, `run()` twice, drawing lists inside `on_draw`). |
| V3 | Centered ship (visual) | ⏳ Owner check | The example launched and ran for over 4 s with no stderr output. An automated screenshot wasn't possible because the agent's shell doesn't share the interactive desktop. Pixel-level centering is covered by V1. |
| V4 | Clean close via X button | ⏳ Owner check | Needs the interactive desktop. The close path (`CloseRequested`, then `run()` returns) is the same one the display tests exercise. |
| V5 | Resize ×20, minimize/restore ×10 | ⏳ Owner check | Needs the interactive desktop. |
| V6 | 150% display scaling | ⏳ Owner check | Needs a change to the display settings. |
| V7 | Bad image path | ✅ Pass | `FileNotFoundError: file not found: 'nope.png'` |
| V7b | Filter required | ✅ Pass | `TypeError: Window.__init__() missing 1 required keyword argument: 'filter'` |
| V8 | Exception in a callback | ✅ Pass | `ValueError: boom`, and the traceback includes the user's `on_draw` line. |
| V9 | Ctrl+C | ⏳ Owner check | Needs an interactive terminal (`py.check_signals()` runs once per frame). |
| V10 | New-developer check | ⏳ Owner check | Someone unfamiliar with the project follows quickstart.md. |

## Other checks

- **Portability audit (constitution III):** clean.
  - The only `Instant` is `web_time::Instant`.
  - winit and pollster appear only in `crates/ferrocade/src/platform/`.
  - No thread spawning.
  - The device uses `Limits::downlevel_webgl2_defaults()`.
  - The only shader is WGSL.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** clean. **`cargo fmt --all`:** applied.
- **Type stubs:** `_ferrocade.pyi` matches the runtime module exactly (no missing or extra names).

## Notes for later phases

- **The renderer caches GPU list data by `ListId`.** This is correct with the single global store the engine uses. A renderer shared between *separate* `SpriteStore` instances (tests only) could see colliding IDs, so tests use one store per renderer.
- **`run_app_on_demand` is desktop-only.** The web build needs `spawn_app` (Phase 1b).
