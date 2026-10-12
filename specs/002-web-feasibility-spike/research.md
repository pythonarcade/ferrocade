# Research: Phase 1b — Web Feasibility Spike

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12

These are desk-research findings from official docs, release notes, and upstream issues, gathered
before any hands-on work. The spike's job is to **confirm or refute them with evidence**.
Confidence levels are noted where sources disagreed or couldn't be verified.

---

## R1. Target Pyodide version

**Decision**: target **Pyodide 314.0.7**, the latest stable release (changelog dated 2026-09-14).
- It ships **CPython 3.14.2** and Emscripten 5.0.3.
- Its wheel platform tag is `pyemscripten_2026_0_wasm32` (PEP 783 renamed `pyodide_*` to `pyemscripten_*`).
- **For the spike, load it from the official CDN.** Vendoring it locally is a later decision, and the report records the download size either way.

**Rationale**:
- It's the current stable release. Python 3.14 meets the engine's 3.13 minimum (engine spec §6), so that minimum doesn't block the web path, as hoped.
- The 315.0.0a2 prerelease (Python 3.15, Emscripten 6.0.5) still isn't new enough for the wgpu fix (see R2).

**Sources**:
- pyodide.org/en/stable/project/changelog.html
- blog.pyodide.org/posts/314-release/
- pyodide.org/en/stable/development/abi/314.html

---

## R2. Approach A (wgpu engine inside a Pyodide wheel): expected to be **blocked**

**Findings (high confidence)**:
- **Building the wheel itself works today.** PyO3 + maturin build for `wasm32-unknown-emscripten` (Pydantic ships this way).
  - Pyodide 314 needs stable Rust ≥ 1.93, and no nightly.
  - Builds must be **version-specific** (`cp314`), not abi3.
  - **Linux/WSL2 only:** pyodide-build doesn't support Windows natively.
- **wgpu 30 doesn't build for Emscripten** (gfx-rs/wgpu#10274, open). wasm-bindgen/web-sys are unconditional wasm32 dependencies, which leaves unresolved `__wbindgen_placeholder__` imports.
  - The proposed fix (wgpu PR #10515, from the wgpu-py maintainer) still isn't merged, and it requires **Emscripten ≥ 6.0.10**.
- **Pyodide won't move to Emscripten ≥ 6.0.10 until its Python 3.16 release** (pyodide#6476), roughly 2027. Even then, wasm-bindgen needs static linking, which side-module wheels don't have yet (wasm-bindgen PR #5326).
- **winit doesn't support Emscripten at all.** It fails to compile with "platform not supported".

**Decision**: attempt A **hands-on, time-boxed**, to turn the desk research into evidence:
- **A1:** build a minimal PyO3 wheel for Pyodide 314 in WSL2 that imports and calls into the `ferrocade` core (texture cache, store) *without* wgpu. This proves (or disproves) the wheel pipeline.
- **A2:** add the wgpu renderer to that build and record the exact failure (expected: the #10274 symptoms).
- If A2 unexpectedly works, continue A toward the full example.

**Rationale**: FR-001 requires attempting A first. A cheap reproduction of the upstream blocker is stronger evidence than a citation alone. A1 also tells us whether the Python-in-wheel parts of A are ready for when the blocker clears.

**Alternatives considered**: skipping A entirely on the strength of the desk research. Rejected because the spec requires an attempt, and A1's positive result matters for the future.

---

## R3. Variant A′: Pyodide wheel with a non-wgpu WebGL2 renderer

**Finding**: inside an Emscripten wheel, rendering could use a small custom WebGL2 backend (glow over Emscripten's WebGL context) plus a custom Emscripten platform layer (html5.h callbacks), with no wgpu and no winit. femtovg and Slint use this pattern. The wgpu maintainers note that this "used to work" for wgpu's own GLES path, but it isn't maintained.

**Decision**: **assess on paper only, not built in the spike.** The report covers its cost.

**Rationale**:
- A′ means maintaining **two renderers** (wgpu on desktop, hand-written WebGL2 on the web) and two platform layers. That's a large, permanent engine change.
- The spike should first establish whether B can avoid it.
- A′ is the fallback if B is unacceptable (for example, if the per-call cost proves too high).

---

## R4. Approach B: Pyodide + a separate engine WASM module (primary hands-on path)

**Findings (medium-high confidence)**:
- wgpu's **officially supported** web target is `wasm32-unknown-unknown` + wasm-bindgen, with WebGPU plus a WebGL2 fallback.
- **This toolchain runs natively on Windows:** `rustup target add wasm32-unknown-unknown`, plus `wasm-bindgen-cli`.
- A wasm-bindgen module and Pyodide can coexist on one page. Python calls the module through Pyodide's JS FFI (`pyodide.registerJsModule` or `js.*`).
- **The cost:** every call crosses Python → JS → WASM, so the API should be batched.
- **No shipped project** was found that drives Rust/wgpu rendering from Pyodide Python. wgpu-py's pure-Python WebGPU backend is the nearest relative.

**Decision**: build **B** as the main hands-on attempt:
- **Engine:** a new crate, `crates/ferrocade-web` (wasm-bindgen), wraps the existing `ferrocade` core (store, texture cache, renderer) and exposes a small JS API (see [contracts/web-engine-api.md](contracts/web-engine-api.md)).
- **Python side:** a pure-Python `ferrocade` package for the browser (`web/python/ferrocade/`) implements the **same public API** as the desktop module (`Window`, `Texture`, `Sprite`, `SpriteList`, `Filter`, `color`), forwarding to the JS engine. This lets the unchanged `examples/01_centered_ship.py` run.

**Rationale**:
- It uses the supported wgpu path and the existing renderer unchanged.
- It keeps the core Python-free (constitution I), exactly as §9 anticipated.
- It needs no WSL.

**Trade-off recorded for the report**:
- **Two binding layers:** PyO3 on desktop, and a Python shim plus wasm-bindgen on the web. The API can drift between them; mitigate this later with shared API tests.
- **Per-property calls cost more on the web** (Python → JS → WASM). Measure it, and push bulk work (Phase 2 `update()`, Phase 5 collisions) into Rust.

---

## R5. Web platform layer for B (no winit in the spike)

**Decision**:
- **Surface:** `ferrocade-web` creates the wgpu surface directly from an `HtmlCanvasElement` (`wgpu::SurfaceTarget::Canvas`). That's enough for the Phase 1 scene, which has no input.
- **winit's web backend (`spawn_app`) is not used in the spike.** It's noted as needed when input arrives in Phase 4.
- **GPU init is async:** JS awaits `Engine.create(canvas)` before any Python runs. This matches constitution III's "async-friendly GPU init".
- **Backends:** request WebGPU first, falling back to WebGL2. A URL flag (`?backend=webgl2`) forces WebGL2, so both paths can be verified (FR-009).
- **Engine changes this needs** (all classified in the report):
  1. `Renderer::with_surface` must accept a canvas surface; it already takes a `wgpu::Surface`.
  2. `texture::load_from_bytes(path, bytes)` is needed, because `assets::read_bytes` is a stub on wasm32 (see R6).
  3. The `pollster`/desktop modules must be cfg-gated off wasm32; they already are.
  4. wgpu needs its `webgpu` + `webgl` features enabled for wasm32.

**Alternatives considered**: winit + `spawn_app` now. Deferred because the spike doesn't need input, and winit's web event loop adds moving parts to an already uncertain spike.

---

## R6. Asset loading on the web

**Decision**:
- The loader fetches the files the example uses (`assets/images/ship.png`) and writes them into **Pyodide's virtual filesystem** at the same relative paths. The Python game code (`fc.Sprite("assets/images/ship.png")`) is then unchanged.
- The Python shim's `Texture(path)` reads the bytes from that filesystem and passes them, together with the path, to `engine.load_texture(path, bytes)`. Rust keeps a cache keyed by path, so a file is read and decoded once.

**Rationale**: zero changes to game code (SC-004), and it uses the asset-loader seam that constitution III already requires.

**Spike shortcut**: the asset list is hard-coded in the loader. A real manifest or bundler is a "before shipping" item.

---

## R7. Game loop on the web

**Decision**: the browser `Window.run()` (Python shim):
1. creates `create_proxy(frame)` and calls `js.requestAnimationFrame(proxy)`, re-arming it every frame
2. **returns immediately**, since the browser can't block
3. in each frame: `n = engine.advance(elapsed)` (reusing Rust's `FixedTimestep`), then `on_update(dt)` n times, then `engine.begin_frame()`, `on_draw()`, `engine.end_frame()` (renders the queued commands)

An exception in a callback stops the loop and is shown on the page (FR-007).

**Rationale**: requestAnimationFrame is vsync-locked and the recommended pattern. Reusing the Rust timestep keeps desktop and web timing identical.

**Behavior difference for the report**: on the web, `run()` returns immediately instead of blocking until the window closes. Game code that puts statements after `run()` would behave differently. The Phase 1 example doesn't.

---

## R8. Approach C (RustPython): assess on paper

**Findings (medium confidence)**:
- RustPython 0.5.0 targets CPython 3.15 semantics and is "not totally production-ready".
- It has no CPython C API, so no PyO3 and no C extensions.
- It can register native Rust modules in the same binary, so in principle RustPython + wgpu + winit could be one `wasm32-unknown-unknown` binary.
- Its current browser build setup couldn't be verified (its wasm README was unavailable).

**Decision**: a **written assessment only**, unless both A and B fail.

**Rationale**: it means a different interpreter and a different binding system than desktop (with its own compatibility gaps), so it's the least attractive option if B works.

---

## R9. Build environments

| Path | Where | Toolchain to install |
|---|---|---|
| A1/A2 (Pyodide wheel) | **WSL2 Ubuntu 22.04** (present: Python 3.11, Node 12; **no** Rust or emsdk) | rustup (stable ≥ 1.93, target `wasm32-unknown-emscripten`), emsdk at Pyodide's exact Emscripten version (5.0.3), Python 3.14 + `pyodide-build` (or maturin with Pyodide's rustflags) |
| B (wasm-bindgen engine) | **Windows, natively** | `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli` at the exact version matching the crate. No WSL needed. |
| Serving and preview | Windows | `python -m http.server` (or similar) from `web/`. Edge or Chrome (both present). |

**Note**: installing the WSL toolchain (A) changes the owner's WSL environment. It's a few GB in the user's home directory and doesn't touch system packages beyond what emsdk needs. Confirm with the owner before installing, as a task gate.

---

## R10. Measurements (FR-009, SC-005)

**Decision**:
- **Download size:** total transferred bytes for a cold load, split into Pyodide runtime, engine `.wasm`, and assets. Use the browser DevTools Network panel (or a headless-browser script).
- **Time to ship-visible:** from navigation start to the first rendered frame containing the ship (a `performance.now()` mark logged by the loader after the first `end_frame`). Measure cold (cache disabled) and warm, and take the median of 3 runs.
- **Graphics paths:** WebGPU (default in Chromium) and WebGL2 (forced via `?backend=webgl2`). Record which ones render correctly.
- **Centering check:** read back the canvas pixels in the browser (`readPixels`/`toDataURL`) and compute the bounding box, the same check as the desktop offscreen test. Record the result.
