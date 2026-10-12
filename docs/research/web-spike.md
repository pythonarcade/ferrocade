# Web Feasibility Spike (Phase 1b)

**Feature**: [specs/002-web-feasibility-spike](../../specs/002-web-feasibility-spike/spec.md) |
**Research**: [research.md](../../specs/002-web-feasibility-spike/research.md) | **Started**: 2026-10-12

## 1. Summary and recommendation

> **Status: Proposed (pending owner acceptance).** *(Filled in at the end of the spike, T021.)*

## 2. Findings per approach

### Approach A: engine built as a Pyodide wheel (wgpu inside)

- **Status**: *TBD (hands-on attempt approved by the owner 2026-10-12)*
- **Desk evidence** (research.md R2):
  - wgpu 30 doesn't build for `wasm32-unknown-emscripten` (gfx-rs/wgpu#10274, open).
  - The fix (gfx-rs/wgpu#10515) needs Emscripten ≥ 6.0.10. Pyodide plans to move only with Python 3.16 (pyodide#6476).
  - winit has no Emscripten backend.
  - Building a PyO3 wheel works on Linux/WSL only, and must be version-specific (cp314), not abi3.
- **Hands-on evidence**: *TBD (A1 core-only wheel, A2 with wgpu)*
- **Blockers**: *TBD*
- **Engine impact**: *TBD*

### Approach A′: Pyodide wheel with a hand-written WebGL2 renderer

- **Status**: *TBD (assessed on paper)*
- **Desk evidence** (research.md R3): glow over Emscripten WebGL plus html5.h callbacks, the pattern femtovg and Slint use. It would mean a second renderer and a second platform layer.

### Approach B: Pyodide + separate engine WASM module

- **Status**: *TBD (main hands-on path)*
- **Desk evidence** (research.md R4–R7): `wasm32-unknown-unknown` + wasm-bindgen is wgpu's supported web target (WebGPU + WebGL2), and it builds natively on Windows. Python reaches the engine through Pyodide's JS FFI.
- **Evidence**: *TBD*
- **Measurements**: *TBD (download size, time to ship, graphics paths, centering, example diff)*
- **Web behavior differences**: *TBD*

### Approach C: RustPython

- **Status**: *TBD (assessed on paper)*
- **Desk evidence** (research.md R8): RustPython 0.5.0 targets 3.15 semantics but is "not totally production-ready". It has no C API (so no PyO3), and native modules go through `rustpython_vm`.

## 3. Comparison

*TBD*

## 4. Engine impact

| Change | Class | Spike shortcut? |
|---|---|---|
| *TBD* | | |

## 5. Risks

*TBD*

## 6. Next steps

*TBD*

## 7. Reproduction steps

*TBD*

## Sources

- Pyodide changelog: https://pyodide.org/en/stable/project/changelog.html
- Pyodide 314 ABI: https://pyodide.org/en/stable/development/abi/314.html
- wgpu Emscripten breakage: https://github.com/gfx-rs/wgpu/issues/10274
- wgpu Emscripten fix PR: https://github.com/gfx-rs/wgpu/pull/10515
- Pyodide Emscripten 6.0.10 request: https://github.com/pyodide/pyodide/issues/6476
- wgpu web platform docs: https://wgpu.rs/doc/wgpu/documentation/platforms/web/index.html
- Pydantic's Emscripten wheels: https://pydantic.dev/articles/emscripten-wheels-pydantic
- RustPython: https://github.com/RustPython/RustPython
