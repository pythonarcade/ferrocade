# Approach A experiment notes (Pyodide wheel)

SPIKE: throwaway experiment for specs/002-web-feasibility-spike (tasks T010–T015).

## Owner gate (T010)
- 2026-10-12: owner **approved** installing the approach-A toolchain in WSL2 Ubuntu 22.04 (home directory only).
- Setup script: `spike/pyodide-wheel/setup_wsl_toolchain.sh`

## Toolchain (installed 2026-10-12 via setup_wsl_toolchain.sh)
- WSL2 Ubuntu 22.04.5, rustc 1.99.0 (stable) + target `wasm32-unknown-emscripten`
- emsdk / emcc **5.0.3** (matches Pyodide 314)
- Python 3.14.8 venv: pyodide-build 0.39.1 (pyodide CLI 0.5.0), maturin 1.15.0
- Pyodide xbuildenv 314.0.7: python 3.14.2, emscripten 5.0.3, rustflags `-C link-arg=-sSIDE_MODULE=2`
- One hiccup: a pyenv shim named `uv` shadowed the real uv; the script now calls `$HOME/.local/bin/uv`.

## A1: core data layer only (no wgpu) ✅ PROVEN
- Engine change: the core got a default-on `render` feature. A1 uses `ferrocade` with `default-features = false`.
- Build: `bash spike/pyodide-wheel/build.sh` (maturin `--target wasm32-unknown-emscripten -i python3.14`, Pyodide rustflags)
  - Stable Rust; no nightly, no `-Zbuild-std`, no abi3. 12.6 s release build.
  - Output: `ferrocade_a1-0.0.1-cp314-cp314-pyemscripten_2026_0_wasm32.whl` (226,758 bytes). maturin 1.15 emits the PEP 783 `pyemscripten_2026_0` tag correctly.
- Load: `smoke.html` in headless Chrome, Pyodide 314.0.7, `micropip.install(<local URL>, deps=False)`:
  ```
  texture_size=(99, 75)
  store_smoke=2
  A1-SMOKE-OK
  ```
  The core's PNG decode, texture cache and sprite store all work inside a Pyodide extension.

## A2: with wgpu (`--features render`) ❌ BLOCKED (reproduces gfx-rs/wgpu#10274)
- Build: `bash spike/pyodide-wheel/build.sh render` **succeeds** (36.8 s). The wheel is 835,710 bytes, and the `.so` is 2,658,601 bytes.
  Side modules leave imports unresolved at link time, so the failure only appears when loading.
- The `.so` imports from the wasm-bindgen modules `__wbindgen_placeholder__` and `__wbindgen_externref_xform__`, with **4,040** `__wbg_*` imports. These are web-sys/js-sys bindings that only wasm-bindgen's post-processor can satisfy.
- Load in Pyodide 314.0.7 fails:
  ```
  Error: Failed to load dynamic library .../ferrocade_a1.cpython-314-wasm32-emscripten.so:
  TypeError: WebAssembly.instantiate(): Import #115 "__wbindgen_placeholder__": module is not an object or function
  ```
- **How to tell when it's resolved**: all three of these need to land:
  1. wgpu merges Emscripten support (gfx-rs/wgpu#10515 or similar), and the `__wbindgen_placeholder__` imports disappear from an emscripten build.
  2. Pyodide ships an Emscripten ≥ 6.0.10 ABI (pyodide#6476; planned with Python 3.16).
  3. wasm-bindgen supports side modules / dynamic linking (wasm-bindgen PR #5326).
  Re-run `build.sh render` plus `smoke.html` to check.
