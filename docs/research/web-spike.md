# Web Feasibility Spike (Phase 1b)

**Feature**: [specs/002-web-feasibility-spike](../../specs/002-web-feasibility-spike/spec.md) |
**Research**: [research.md](../../specs/002-web-feasibility-spike/research.md) | **Date**: 2026-10-12
**Raw evidence**: [spike/web-results.md](../../spike/web-results.md) (approach B) ·
[spike/pyodide-wheel/NOTES.md](../../spike/pyodide-wheel/NOTES.md) (approach A)

## 1. Summary and recommendation

> **Status: Accepted by the owner (2026-10-12).** It's recorded in the engine spec §9 and its decision log.

**It works.** `examples/01_centered_ship.py` runs **unchanged** in the browser, in Chrome and Edge,
with both **WebGPU** and **WebGL2**. The ship renders 99×75, centered to the same half pixel as on
desktop, about **2 seconds** after opening the page.

**Recommendation: adopt approach B for the web** (Pyodide + the engine as a separate wasm-bindgen
module), and **revisit approach A** once upstream unblocks it (likely 2027).
- **B works today**, using wgpu's *supported* web target and the *unchanged* renderer and core.
- The only engine change it needed was a real bug fix that also benefits desktop (sRGB view formats).
- **A is blocked upstream.** That was confirmed hands-on: the wgpu-linked Pyodide wheel builds but
  can't load. A's Python-wheel half already works (A1), so switching later is cheap once the blockers clear.
- **A′** (a second, hand-written WebGL2 renderer) and **C** (RustPython) both cost more and have no
  advantage while B works.

**The main costs of B** (see Risks):
- a second binding layer (a Python shim) that must stay in sync with the PyO3 bindings
- higher per-call overhead (Python → JS → WASM)
- about 7–11 MB of first-load download (most of it the Python runtime)

## 2. Findings per approach

### Approach A: engine built as a Pyodide wheel (wgpu inside)

- **Status**: **blocked** (hands-on, 2026-10-12)
- **Evidence**:
  - **A1, core only: proven.**
    - A PyO3 extension using the Ferrocade core's data layer (with the new `render` feature off) built in WSL2 with **stable Rust 1.99**, emsdk 5.0.3, and maturin 1.15 (`--target wasm32-unknown-emscripten -i python3.14`, Pyodide rustflags `-sSIDE_MODULE=2`).
    - It produced `ferrocade_a1-0.0.1-cp314-cp314-pyemscripten_2026_0_wasm32.whl` (227 KB) and installed with `micropip` in Pyodide 314.0.7.
    - It printed `texture_size=(99, 75)` and `store_smoke=2`: PNG decoding, the texture cache and the sprite store all work inside Pyodide.
  - **A2, wgpu linked: blocked.**
    - The wheel builds (836 KB), but its `.so` carries **4,040 wasm-bindgen imports** from `__wbindgen_placeholder__`.
    - Pyodide fails with `WebAssembly.instantiate(): Import #115 "__wbindgen_placeholder__": module is not an object or function`.
    - This reproduces gfx-rs/wgpu#10274.
- **Blockers**: all three must be resolved; re-test with `spike/pyodide-wheel/build.sh render` plus `smoke.html`.
  1. wgpu must support Emscripten (gfx-rs/wgpu#10515 is open). It requires Emscripten ≥ 6.0.10.
  2. Pyodide must ship an Emscripten ≥ 6.0.10 ABI (pyodide#6476). That's planned with the Python 3.16 release.
  3. wasm-bindgen must work in dynamically linked side modules (wasm-bindgen PR #5326).
  - Also, winit has no Emscripten backend, so A would still need its own platform layer.
- **Engine impact**: the core's `render` feature (done, and generally useful: a headless data layer). Wheels must be version-specific (cp314, no abi3), and Emscripten builds need Linux/WSL2.

### Approach A′: Pyodide wheel with a hand-written WebGL2 renderer

- **Status**: **assessed on paper**
- **What it would take**:
  - A second renderer: glow over Emscripten's WebGL2 context, with a GLSL ES 3.0 version of the sprite shader, texture upload, instance buffers and batching. That's roughly the size of today's wgpu renderer (~600 lines), and it has to stay in sync forever.
  - A second platform layer: Emscripten html5.h callbacks and a requestAnimationFrame main loop. femtovg and Slint use this pattern.
  - A1's wheel pipeline, which is already proven.
- **Pros**: a single Python binding (PyO3 everywhere), and no Python→JS hop per call.
- **Cons**:
  - Two renderers and two platform layers violate the spirit of "one wgpu renderer everywhere" and double the rendering bug surface (the sRGB issue found in this spike is exactly the kind of difference that would multiply).
  - No WebGPU.
  - Linux-only builds.
- **When it makes sense**: only if B's per-call overhead proves unacceptable in Phase 3 *and* A is still blocked.

### Approach B: Pyodide + separate engine WASM module

- **Status**: **proven** (hands-on, 2026-10-12)
- **Evidence** (details in `spike/web-results.md`):
  - **Setup:** `crates/ferrocade-web` (wasm-bindgen, about 280 lines) wraps the unchanged core and renderer. A pure-Python shim (`web/python/ferrocade`, about 250 lines) implements the game-facing API in Pyodide 314.0.7. A loader page (about 115 lines) ties them together. Everything builds **natively on Windows**.
  - **WebGPU:** Chrome and Edge report `backend=BrowserWebGpu`. **WebGL2:** `?backend=webgl2` gives `backend=Gl`.
  - **Pixel check in the browser:** the ship's bounding box is (350,262)–(448,336), **99×75, center offset −0.5 px** on all three runs. That's identical to the desktop offscreen test.
  - **Unchanged code:** the executed game source is byte-identical to `examples/01_centered_ship.py` (**0 changed lines**).
  - **Errors:** a missing asset shows a readable error panel naming the file.
  - **No desktop regression:** 29 Rust and 43 pytest tests pass, and clippy is clean (desktop and wasm32).
- **Measurements**:

  | Metric | Result | Target |
  |---|---|---|
  | Time to ship visible, cold (median of 3) | **2.08 s** | ≤ 30 s ✅ |
  | Time to ship visible, warm (median of 3) | **1.73 s** | ≤ 10 s ✅ |
  | Download: Pyodide runtime | 6.24 MB (compressed by the CDN) | Recorded |
  | Download: engine wasm + glue | 4.90 MB uncompressed (wasm gzips to 1.38 MB; `wasm-opt` not yet applied) | Recorded |
  | Download: assets | 2.6 KB | Recorded |
  | Graphics paths | WebGPU ✅, WebGL2 ✅ | At least one |
  | Centering | Within 0.5 px, 99×75 | Within 1 px ✅ |
  | Example diff | 0 lines | ≤ 5 ✅ |

- **A real engine bug found and fixed**: browser WebGPU canvases offer only *non-sRGB* formats, so the first run drew a black background and dark colors. The renderer now configures an **sRGB view format** on non-sRGB surfaces, so colors match desktop exactly. This would have bitten any WebGPU port.
- **Web behavior differences** (game-facing):

  | Behavior | Desktop | Browser |
  |---|---|---|
  | `Window.run()` | Blocks until the window closes | Starts the requestAnimationFrame loop and **returns immediately** |
  | Callback exception | Re-raised from `run()` | Stops the loop, shown on the page and in the console |
  | Window | OS window, title bar | The canvas is sized to the window size, and `document.title` is set |
  | Ctrl+C / closing | Supported | Not applicable (closing the tab ends it) |
  | Not yet in the shim | — | Changing a sprite's `texture` raises `NotImplementedError` (spike scope) |

### Approach C: RustPython

- **Status**: **assessed on paper** (not attempted because B succeeded)
- **Assessment**:
  - RustPython 0.5.0 targets CPython 3.15 semantics but calls itself "not totally production-ready".
  - It has **no CPython C API**, so no PyO3. The bindings would be rewritten against `rustpython_vm`, giving a third binding system.
  - Third-party packages with C extensions (numpy and so on) won't work, which matters for users who expect "real Python".
  - **Upside:** a single `wasm32-unknown-unknown` binary containing the interpreter, wgpu, and winit. No Pyodide download, and no Python→JS hop.
  - Its current browser build setup couldn't be verified (the docs were unavailable).
- **When it makes sense**: only if download size becomes critical *and* compatibility gaps are acceptable. That's unlikely for a teaching and hobby engine whose users expect standard Python.

## 3. Comparison

| | A (wheel + wgpu) | A′ (wheel + own WebGL2) | **B (Pyodide + wasm-bindgen)** | C (RustPython) |
|---|---|---|---|---|
| Works today | ❌ blocked upstream | Untested (feasible) | **✅ proven** | Untested |
| Game code changes | 0 (expected) | 0 (expected) | **0 (measured)** | Likely some (compatibility gaps) |
| Renderers to maintain | 1 | **2** | **1** | 1 |
| Python binding layers | 1 (PyO3) | 1 (PyO3) | **2** (PyO3 + web shim) | 2 (PyO3 + rustpython_vm) |
| Per-call cost from Python | Low (direct) | Low | **Higher** (Python→JS→WASM) | Medium |
| WebGPU | Yes, once unblocked | No | **Yes** (+ WebGL2) | Yes |
| First-load download | Pyodide + wheel | Pyodide + wheel | **Pyodide 6.2 MB + engine 1.4–4.9 MB** | Interpreter + engine (smaller) |
| Build host | Linux/WSL2 | Linux/WSL2 | **Windows native** | Any |
| Real CPython | Yes | Yes | **Yes** | No |

## 4. Engine impact

| Change | Class | Spike shortcut? |
|---|---|---|
| `texture::load_from_bytes` (cache keyed by path; for filesystem-less platforms) | Done in spike | No (production quality, tested) |
| wgpu `webgpu` + `webgl` features on wasm32; winit desktop-only | Done in spike | No |
| `Renderer::backend_name()` | Done in spike | No |
| **sRGB view format on non-sRGB surfaces** (bug fix, needed for WebGPU) | Done in spike | No |
| Core `render` feature (data layer builds without wgpu) | Done in spike | No (useful for headless and Emscripten use) |
| `crates/ferrocade-web` (wasm-bindgen Engine) | Done, **spike quality** | Yes: `SPIKE:` (no tests, u32 handle tables never reuse slots, no texture swap) |
| Browser Python shim `web/python/ferrocade` | Done, **spike quality** | Yes: must become a real package that's tested and versioned with desktop |
| ~~`web/python/ferrocade/color.py` copy~~ | **Done in Phase 1c**: the loader fetches the desktop `python/ferrocade/color.py` | — |
| Hard-coded asset list in `web/loader.js` | Required before shipping web | Yes: needs an asset manifest or bundler |
| Shared API test suite run against both bindings (desktop PyO3 and web shim) | Required before shipping web | No (new) |
| winit web backend (or DOM events) for input | Required before Phase 4 on the web | No (new) |
| `wasm-opt` plus a compressing server or CDN for the engine | Nice to have | No |
| Vendoring Pyodide instead of loading it from the CDN | Nice to have (offline or self-hosted) | No |
| Approach-A experiment (`spike/pyodide-wheel/`) | Keep for re-testing A later | Yes |

The `SPIKE:` markers are in `crates/ferrocade-web/*`, `web/*`, `tools/build_web.ps1`,
`tools/web-check/*`, and `spike/pyodide-wheel/*`.

## 5. Risks

1. **API drift between the two bindings** (PyO3 desktop vs. the web shim). *Mitigation:* run one pytest API suite against both. Generate or check the shim against the `.pyi` stubs.
2. **Per-call overhead** (Python → JS → WASM) could hurt games that touch thousands of sprites from Python each frame. *Mitigation:* measure it in Phase 3 (add a web scene to the stress test), and lean on Rust-side bulk APIs (`SpriteList.update()`, collisions), as the constitution already requires.
3. **Download size** (~7.6 MB compressed, mostly Pyodide) may be too much for casual web games. *Mitigation:* `wasm-opt`, compression, CDN caching (Pyodide is shared across sites), and a loading screen.
4. **`run()` behaves differently** on the web (it returns immediately). *Mitigation:* document it, and encourage the pattern where `run()` is the last line.
5. **wgpu's web backends are younger than its native ones.** The sRGB surprise is one example. *Mitigation:* keep the browser pixel check (`tools/web-check`) in regular use.
6. **Dependence on Pyodide's release cadence** (Python version per release, plus Emscripten pins) for both B (runtime) and a future A.

## 6. Next steps

1. **Owner decision:** accept or reject this recommendation. If it's accepted, record "web = approach B (Pyodide + wasm-bindgen engine); revisit A when wgpu#10515 + Pyodide ≥ Emscripten 6.0.10 land" in the engine spec §9 and its decision log.
2. Continue the roadmap with **Phase 2 (bouncing ship)** on desktop, and keep the web example working as a smoke test.
3. Before shipping web support, do the "required before shipping web" rows in §4: a real web package, a shared API test suite, an asset manifest, and shared `color.py`.
4. Add a web variant of the **Phase 3 stress test** to quantify the per-call overhead (risk 2).
5. Every few months, re-run `spike/pyodide-wheel/build.sh render` to see whether approach A has been unblocked.

## 7. Reproduction steps

**Approach B (Windows, PowerShell, from the repository root):**

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
tools\build_web.ps1 -Serve            # builds web\pkg, serves the repo root on :8000
# open http://localhost:8000/web/  (add ?backend=webgl2 to force WebGL2, ?diagnostics=1 for the pixel check)

# Automated check (optional; needs Node 20+ and Chrome or Edge):
cd tools\web-check; npm install; node check.mjs "http://localhost:8000/web/?diagnostics=1"
```

**Approach A experiment (WSL2 Ubuntu 22.04):**

```bash
bash spike/pyodide-wheel/setup_wsl_toolchain.sh     # one-time: rustup, emsdk 5.0.3, Python 3.14 + pyodide-build
bash spike/pyodide-wheel/build.sh                   # A1 wheel (core only)
bash spike/pyodide-wheel/build.sh render            # A2 wheel (links wgpu; expected to fail at load)
# with the repo served on :8000 (Windows):
#   node tools/web-check/smoke_a.mjs ferrocade_a1-0.0.1-cp314-cp314-pyemscripten_2026_0_wasm32.whl
```

## Sources

- Pyodide changelog: https://pyodide.org/en/stable/project/changelog.html
- Pyodide 314 ABI: https://pyodide.org/en/stable/development/abi/314.html
- wgpu Emscripten breakage: https://github.com/gfx-rs/wgpu/issues/10274
- wgpu Emscripten fix PR: https://github.com/gfx-rs/wgpu/pull/10515
- Pyodide Emscripten 6.0.10 request: https://github.com/pyodide/pyodide/issues/6476
- wgpu web platform docs: https://wgpu.rs/doc/wgpu/documentation/platforms/web/index.html
- Pydantic's Emscripten wheels: https://pydantic.dev/articles/emscripten-wheels-pydantic
- PEP 783 (Emscripten packaging): https://peps.python.org/pep-0783/
- RustPython: https://github.com/RustPython/RustPython
