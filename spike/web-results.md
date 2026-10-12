# Approach B results (Phase 1b web spike)

SPIKE: raw evidence for `docs/research/web-spike.md`. Recorded 2026-10-12 on the dev machine:
Windows 11 Pro 10.0.26200, NVIDIA RTX 3500 Ada Laptop GPU. The server was `python -m http.server`
on localhost (no compression). Checks ran with `tools/web-check/check.mjs` (puppeteer-core, headless "new").

## Toolchain
- rustc 1.99.0, target `wasm32-unknown-unknown`; wasm-bindgen(-cli) 0.2.129
- wgpu 30.0.1 (`webgpu` + `webgl` features on wasm32)
- Pyodide **314.0.7** (CPython 3.14.2) from cdn.jsdelivr.net
- Chrome and Edge (current stable, headless)

## W2: ship in the browser, WebGPU ✅
- `backend=BrowserWebGpu pyodide=314.0.7`, outcome `ship-visible`, no page errors.
- Screenshot: `tools/web-check/out/chrome-_diagnostics_1.png` (gitignored; regenerate with the check script).
- **The first run found a real bug:** the background was black. Browser WebGPU canvases offer only
  non-sRGB formats (bgra8unorm), but the renderer assumed an sRGB surface. **Fix (engine change, done):**
  configure the surface with an sRGB *view format* (`format.add_srgb_suffix()`) and render through
  that view. After the fix, the colors match desktop.

## W3: WebGL2 fallback ✅
- `?backend=webgl2` gives `backend=Gl`: the same image, the same centering result.
- Screenshot: `tools/web-check/out/chrome-_diagnostics_1_backend_webgl2.png`

## W4: centering ✅
The canvas pixels were read back in the same task as the first frame (`?diagnostics=1`):

| Browser / backend | bbox (x0,y0,x1,y1) | size | center offset |
|---|---|---|---|
| Chrome / WebGPU | 350,262,448,336 | 99×75 | −0.5, −0.5 px |
| Chrome / WebGL2 | 350,262,448,336 | 99×75 | −0.5, −0.5 px |
| Edge / WebGPU | 350,262,448,336 | 99×75 | −0.5, −0.5 px |

These match the desktop offscreen render test exactly. The −0.5 px offset comes from the odd 99×75 size on the pixel grid.

## W5: example unchanged ✅
The loader fetches `examples/01_centered_ship.py` and runs it as-is. The check script compares the
executed source with the file on the server: `exampleUnchanged: true`, so **0 changed lines**.

## W6: time to ship visible ✅ (targets: ≤ 30 s cold, ≤ 10 s warm)
Measured from navigation start to the first rendered frame (`performance.measure`), Chrome, WebGPU:

| Run | Cold (fresh profile) | Warm (cached) |
|---|---|---|
| 1 | 2075 ms | 1763 ms |
| 2 | 2000 ms | 1714 ms |
| 3 | 2089 ms | 1731 ms |
| **Median** | **2075 ms** | **1731 ms** |

Edge, cold: 2127 ms. Note that Pyodide came from the CDN over the real network. The engine and assets came from localhost.

## W7: download size (cold, encoded bytes on the wire)

| Part | Bytes | Notes |
|---|---|---|
| Pyodide runtime (CDN) | 6,242,458 | Compressed by the CDN |
| Engine (`ferrocade_web_bg.wasm` + JS glue) | 4,903,029 | **Uncompressed** (http.server). The wasm compresses to 1,380,466 bytes with gzip -9, before any `wasm-opt`. |
| Assets | 2,604 | `ship.png` |
| Other (page, loader, shim) | ~16,000 | |
| **Total** | **~11.2 MB** (about 7.6 MB with a compressing server) | |

## W8: error display ✅
With `assets/images/ship.png` temporarily removed, the page showed the error panel with
"could not load ../assets/images/ship.png: HTTP 404" (and status "Error"). There was no blank page.
The file was restored afterwards.

## W9: desktop regression ✅
After all the engine changes (wasm32 features, `load_from_bytes`, `backend_name`, the sRGB view format):
`cargo test --workspace` gives 29 passed, `pytest` gives 43 passed, and
`cargo clippy --workspace --all-targets -D warnings` plus `cargo clippy -p ferrocade-web --target wasm32-unknown-unknown -D warnings` are clean.

## W10: reproducibility from a clean clone ✅
`git clone` into a temp directory, then `tools\build_web.ps1` (only the documented steps), then serve on :8001,
then `check.mjs`: `ship-visible`, BrowserWebGpu, bbox 350,262,448,336 (99×75, −0.5 px), example unchanged,
2072 ms. The temp clone was deleted afterwards.

## Owner visual check
*Pending: open http://localhost:8000/web/ in Edge or Chrome while `tools\build_web.ps1 -Serve` runs.*
