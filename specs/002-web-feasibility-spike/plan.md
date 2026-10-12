# Implementation Plan: Phase 1b — Web Feasibility Spike

**Branch**: `002-web-feasibility-spike` (spec directory; work happens on `main` locally) | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-web-feasibility-spike/spec.md`

## Summary

Find out how Ferrocade games (Python) can run in a browser, and back the answer with evidence.

**Desk research ([research.md](research.md)) shows approach A is blocked upstream:**
- wgpu 30 doesn't build for Emscripten (gfx-rs/wgpu#10274).
- winit has no Emscripten backend.
- Pyodide won't adopt the Emscripten version the fix needs until Python 3.16 (~2027).

**The plan:**
- **A (time-boxed):** reproduce that blocker hands-on in WSL2 (A1: core-only Pyodide wheel; A2: add wgpu).
- **B (main hands-on path):** build approach B:
  - a `wasm32-unknown-unknown` wasm-bindgen engine crate (`crates/ferrocade-web`) wrapping the unchanged Rust core and renderer, rendering into a canvas through WebGPU or WebGL2
  - a pure-Python `ferrocade` shim running in Pyodide 314 that implements the desktop API, so `examples/01_centered_ship.py` runs unchanged
- **A′ and C:** assess on paper.
- **Output:** `docs/research/web-spike.md`, with a **Proposed** recommendation, measurements, and engine-impact classification.

## Technical Context

**Language/Version**:
- Rust stable 1.99, with targets `wasm32-unknown-unknown` (B) and `wasm32-unknown-emscripten` (A, in WSL).
- Python 3.14 (Pyodide 314.0.7) in the browser. Desktop remains ≥ 3.13.
- JavaScript (ES modules) for the loader.

**Primary Dependencies**:
- B: `ferrocade` core, wgpu 30 (`webgpu` + `webgl` features on wasm32), wasm-bindgen 0.2.129 (matching `Cargo.lock`), web-sys, js-sys, wasm-bindgen-futures, console_error_panic_hook.
- A: PyO3 0.29 (non-abi3), maturin/pyodide-build, emsdk 5.0.3.

**Storage**: N/A (assets are written into Pyodide's in-memory filesystem by the loader).

**Testing**:
- Validation scenarios W1–W10 in [quickstart.md](quickstart.md), including an in-browser pixel check (W4).
- Desktop regression: `cargo test --workspace` + `pytest` (W9).
- Spike code isn't unit-tested beyond the existing core tests.

**Target Platform**: Chromium-based browsers (Edge or Chrome) on Windows 11. The build host is Windows (B) and WSL2 Ubuntu 22.04 (A).

**Project Type**: a feasibility spike. It produces a research report, a new wasm-bindgen crate, a web loader, and a browser Python shim.

**Performance Goals**: the ship appears ≤ 30 s cold and ≤ 10 s warm (SC-005), and the download size is measured (FR-009).

**Constraints**:
- The game code stays unchanged (≤ 5 justified lines).
- The desktop build and tests keep passing.
- Spike shortcuts are marked `SPIKE:`.
- Constitution III (WebGL2 limits, non-blocking loop, async init, asset loader) is tested, not bypassed.

**Scale/Scope**: the Phase 1 scene only. Roughly 300–500 lines of Rust (`ferrocade-web`), ~250 lines of Python (shim), ~150 lines of JS/HTML (loader), and the report.

There are no NEEDS CLARIFICATION items left. All unknowns are resolved or turned into explicit spike experiments in research.md (R1–R10). Two gates need the owner:
1. Installing the WSL toolchain for A (R9).
2. Accepting the final recommendation (FR-015).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre-research | Post-design |
|---|---|---|---|
| I. Python API, Rust Core | Users write only Python. The core stays Python-free. Bindings are thin. | ✅ | ✅ The game-facing API is identical. B uses the core unchanged, and the web layer is two thin wrappers (wasm-bindgen crate + Python shim). See Complexity Tracking for the second binding layer. |
| II. Minimize conversion | Data stays in Rust, and calls are batched. | ✅ | ⚠️ Accepted for the spike: each property call crosses Python → JS → WASM, which costs more than PyO3. The store remains the single source of truth (no Python-side copies). The cost is **measured and reported**, and bulk APIs (Phase 2/5) mitigate it. No violation of the data-ownership rule. |
| III. Portable by design | Platform code isolated, WebGL2 limits, async init, non-blocking loop, asset loader, no threads. | ✅ | ✅ This spike *is* the test of III. Web platform code lives only in `crates/ferrocade-web`. The renderer keeps `downlevel_webgl2_defaults`. Init is async. The loop uses requestAnimationFrame. Bytes go through the asset seam (`load_from_bytes`). |
| IV. 2D only, simple first | Minimal and focused. | ✅ | ✅ The Phase 1 scene only. No winit on the web yet. Hard-coded asset list (marked SPIKE). |
| V. Spike-driven decisions | Owner confirms. Open designs untouched. | ✅ | ✅ The recommendation stays **Proposed** until accepted. The engine spec §9 decision isn't recorded early. Collisions are untouched. |
| VI. Phase-based delivery | Runnable example plus done criteria. | ✅ | ✅ The browser example plus W1–W10. The desktop example is unaffected. |

**Result**: PASS. One justified complexity item is below.

## Project Structure

### Documentation (this feature)

```text
specs/002-web-feasibility-spike/
├── plan.md
├── research.md          # R1–R10: desk research + spike decisions
├── data-model.md        # findings entities + browser runtime objects
├── quickstart.md        # build/serve steps + validation W1–W10
├── contracts/
│   └── web-engine-api.md   # browser Python API diffs, engine JS API, loader behavior
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml                         # + crates/ferrocade-web member
crates/ferrocade/                  # core: small, cfg-safe changes only
├── Cargo.toml                     # wgpu features for wasm32 (webgpu + webgl)
└── src/texture.rs                 # + load_from_bytes(path, bytes) (cached by path)
crates/ferrocade-web/              # NEW: wasm-bindgen engine for approach B
├── Cargo.toml                     # crate-type cdylib; wasm-bindgen 0.2.129, web-sys, wasm-bindgen-futures
└── src/lib.rs                     # Engine: create(canvas), configure, advance, begin/end frame,
                                   #   textures, sprites, lists, readPixels (contract §2)
web/
├── index.html                     # canvas + status/error panel
├── loader.js                      # Pyodide 314.0.7 (CDN) + engine + assets + run example; timings
├── python/ferrocade/__init__.py   # browser shim implementing the desktop API (contract §1)
├── python/ferrocade/color.py      # copied from python/ferrocade/color.py (same constants)
└── pkg/                           # wasm-bindgen output (gitignored)
tools/build_web.ps1                # build + wasm-bindgen + (optional) serve
spike/pyodide-wheel/               # approach A experiment (A1/A2), WSL build script + notes
docs/research/web-spike.md         # THE deliverable: findings, measurements, recommendation (Proposed)
```

**Structure Decision**:
- The web engine is a separate crate, so the desktop crates gain no web dependencies (constitution III: platform code isolated). The `ferrocade` core changes are limited to an additive `load_from_bytes` and cfg-gated wgpu features.
- Approach A experiments live under `spike/` (clearly throwaway).
- The browser shim lives under `web/python/`, not `python/`, so it can't be confused with the desktop package.

## Implementation Order (for /speckit-tasks)

1. **B setup:**
   - add the `wasm32-unknown-unknown` target
   - install `wasm-bindgen-cli` 0.2.129
   - scaffold `crates/ferrocade-web`
   - make the core compile for wasm32 (wgpu features, `load_from_bytes`)
2. **B engine:** `Engine.create` (canvas surface, backend option), frame API, textures, sprites, lists, `readPixels`.
3. **B browser side:** loader (Pyodide CDN, assets into its filesystem, run the unchanged example), Python shim, error/loading UI, timing marks, `tools/build_web.ps1`.
4. **B validation:** W2–W8 (WebGPU, WebGL2, pixel check, diff, timings, sizes, error display), and W9 desktop regression.
5. **A (owner-gated):** WSL toolchain, then A1 (core-only cp314 Pyodide wheel), then A2 (add wgpu; record the failure).
6. **Report:** write `docs/research/web-spike.md`: A, A′, B, C findings, comparison, engine-impact classification, risks, a Proposed recommendation, reproduction steps. Then W1 and W10.

B is ordered before A because B is the likely viable path and needs no environment changes. A depends on the owner approving the WSL install. Both are reported regardless of order.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Second binding layer on the web (pure-Python shim + wasm-bindgen crate) next to the PyO3 bindings | PyO3 extensions can't run in the browser through the supported wgpu path (Emscripten is blocked until ~2027), so the browser needs its own route from Python to the core | **A′** (Emscripten wheel + hand-written WebGL2 renderer) avoids the second Python layer but adds a *second renderer and platform layer*, which is far more code to keep in sync. **C** (RustPython) replaces the interpreter itself. Drift risk is mitigated later by running the shared API test suite against both bindings. |
