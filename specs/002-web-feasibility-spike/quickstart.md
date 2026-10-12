# Quickstart & Validation: Phase 1b — Web Feasibility Spike

**Feature**: [spec.md](spec.md) | **Contracts**: [contracts/web-engine-api.md](contracts/web-engine-api.md)

## Approach B: browser build (Windows, PowerShell, from the repository root)

### Prerequisites (one-time)

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # must match Cargo.lock's wasm-bindgen exactly
```

You also need Edge or Chrome, plus internet access, because Pyodide 314.0.7 loads from its CDN.

### Build and serve

```powershell
cargo build -p ferrocade-web --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/ferrocade_web.wasm
python -m http.server 8000          # from the repo root
# open http://localhost:8000/web/            (WebGPU, falling back to WebGL2)
# open http://localhost:8000/web/?backend=webgl2   (force WebGL2)
```

These steps are wrapped in `tools\build_web.ps1` (add `-Serve` to also start the server).

**Verified 2026-10-12** with rustc 1.99.0, wasm-bindgen 0.2.129, wgpu 30.0.1, Pyodide 314.0.7, and current Chrome and Edge on Windows 11.

## Approach A: wheel attempt (WSL2 Ubuntu)

> **Gate:** this installs rustup, emsdk (Emscripten 5.0.3), and pyodide-build into the WSL home
> directory. Confirm with the owner before doing it (research.md R9).

```bash
bash spike/pyodide-wheel/setup_wsl_toolchain.sh   # one-time (owner approved 2026-10-12)
bash spike/pyodide-wheel/build.sh                 # A1: core-only wheel (loads and runs in Pyodide)
bash spike/pyodide-wheel/build.sh render          # A2: links wgpu; builds, but fails to load (wgpu#10274)
```
Results are in `spike/pyodide-wheel/NOTES.md`.

## Validation scenarios

| # | Scenario | How | Expected | Covers |
|---|---|---|---|---|
| W1 | Report completeness | Read `docs/research/web-spike.md` | A, A′, B, and C each have a status plus evidence or a reason. There's a single **Proposed** recommendation, risks, and next steps. | US1, FR-001–004, FR-013, SC-001/002 |
| W2 | Ship in the browser (WebGPU) | Open `/web/` in Edge or Chrome | The loading states show, then a dark gray canvas with the ship centered and crisp. The console logs `backend=…` and the ship-visible time. | US2, FR-005–007, SC-003 |
| W3 | WebGL2 fallback | Open `/web/?backend=webgl2` | Same image, and the console shows the GL backend. | FR-009 |
| W4 | Centering pixel check | Open with `?diagnostics=1`; the loader reads the canvas pixels after the first frame and logs the bounding box | Centered within 1 px at 99×75. | SC-003 |
| W5 | Unchanged example | `git diff --no-index` of the example the loader runs against `examples/01_centered_ship.py` | 0 lines (≤ 5 allowed, each justified). | SC-004, FR-008 |
| W6 | Load time | DevTools with cache disabled, then enabled; 3 runs each | Median cold ≤ 30 s and warm ≤ 10 s, or recorded as a finding. | SC-005, FR-009 |
| W7 | Download size | DevTools Network panel, cold | Runtime, engine, and asset bytes recorded. | FR-009 |
| W8 | Error display | Temporarily rename `assets/images/ship.png` on the server | A readable error panel naming the file. Not blank. | FR-007 |
| W9 | Desktop regression | `cargo test --workspace`, `maturin develop`, `pytest` | Everything passes. | FR-011, SC-007 |
| W10 | Reproducibility | Follow this guide from a clean checkout | W2 reproduces without undocumented steps. | US3, FR-014, SC-006 |
