# Contract: Web Engine and Loader (approach B)

**Feature**: [../spec.md](../spec.md) | **Data model**: [../data-model.md](../data-model.md)

The spike defines three interfaces:
1. the **game-facing Python API** in the browser, which is the *same contract as desktop*
2. the **engine JS API** exported by `ferrocade-web`
3. the **loader page** behavior

Only (1) is user-facing. (2) and (3) are spike-level and may change.

## 1. Game-facing Python API (browser)

It must satisfy [specs/001-centered-ship/contracts/python-api.md](../../001-centered-ship/contracts/python-api.md)
for everything the Phase 1 example uses: `Window(width, height, title, *, filter, update_rate)`,
`on_draw`, `on_update`, `clear`, `run`, `Texture`, `Sprite`, `SpriteList` (append and draw),
`Filter`/`NEAREST`/`LINEAR`, and `color`.

**Documented web differences** (they go into the report):

| Behavior | Desktop | Browser (spike) |
|---|---|---|
| `Window.run()` | Blocks until the window closes, then returns. | Starts the requestAnimationFrame loop and **returns immediately**. |
| Callback exception | Re-raised from `run()`. | Stops the loop, and is shown on the page plus the browser console. |
| Window size and title | OS window. | The canvas is resized to `width`×`height` CSS pixels, and `document.title` is set from `title`. |
| Ctrl+C | `KeyboardInterrupt`. | Not applicable. |
| Unimplemented parts of the API | — | Raise `NotImplementedError("... not available in the web spike")`. Never silently ignored. |

## 2. Engine JS API (`ferrocade-web`, wasm-bindgen)

All IDs are `u32` handles in engine-side tables, so they stay JS-number safe. Numbers are f64 at
the boundary, stored exactly as on desktop (f64 in the store, f32 in GPU buffers).

```ts
class Engine {
  /** Async GPU init on the canvas. backend: "auto" (WebGPU, then WebGL2) | "webgl2". */
  static create(canvas: HTMLCanvasElement, backend: string): Promise<Engine>;

  configure(width: number, height: number, filter: "nearest" | "linear", updateRate: number): void;
  readonly backendName: string;                 // e.g. "BrowserWebGpu" / "Gl", for the report

  // Frame loop (driven by the Python shim's requestAnimationFrame callback)
  advance(elapsedSeconds: number): number;      // number of on_update calls to run
  beginFrame(): void;
  clear(r: number, g: number, b: number, a: number): void;   // 0-255
  endFrame(): void;                             // render queued commands; throws on GPU error

  // Textures: bytes come from Pyodide's filesystem; cached by path (decoded once)
  loadTexture(path: string, bytes: Uint8Array): number;      // texture handle
  textureSize(texture: number): [number, number];

  // Sprites
  createSprite(texture: number, filter: "" | "nearest" | "linear",
               centerX: number, centerY: number, scale: number): number;
  destroySprite(sprite: number): void;
  getSprite(sprite: number): Float64Array;      // [center_x, center_y, scale, angle]
  setCenterX(sprite: number, v: number): void;
  setCenterY(sprite: number, v: number): void;
  setPosition(sprite: number, x: number, y: number): void;
  setScale(sprite: number, v: number): void;
  setAngle(sprite: number, v: number): void;

  // Sprite lists
  createList(): number;
  destroyList(list: number): void;
  append(list: number, sprite: number): void;   // throws "already in this sprite list"
  remove(list: number, sprite: number): void;   // throws "not in this sprite list"
  drawList(list: number): void;                 // only between beginFrame/endFrame

  /** Spike diagnostics: canvas pixels, for the centering check. */
  readPixels(): Promise<Uint8Array>;
}
```

**Error contract**: engine errors become JS `Error`s with the same messages as desktop (e.g.
`"draw() can only be called inside on_draw()"`). The shim converts them to the same Python
exception types the desktop module raises (`ValueError`, `RuntimeError`, `FileNotFoundError`).

## 3. Loader page (`web/index.html` + `web/loader.js`)

| Step | Visible state | On failure |
|---|---|---|
| 1. Load the Pyodide runtime (CDN, pinned to 314.0.7) | "Loading Python runtime…" | Error panel: "Could not load Python runtime: <reason>" |
| 2. Load the engine WASM and run `Engine.create(canvas, backend)` | "Starting graphics…" | Error panel with the reason (for example, no WebGPU and no WebGL2) |
| 3. Write the assets into Pyodide's filesystem (`assets/images/ship.png`) | "Loading assets…" | Error panel naming the file |
| 4. Register the shim package and engine, then run `examples/01_centered_ship.py` (fetched unchanged) | Canvas visible | A Python traceback in the error panel |
| 5. First frame rendered | `performance.mark("ship-visible")`, with the timing logged to the console | — |

**URL parameters**: `?backend=webgl2` forces WebGL2.

**Serving**: any static file server, run from the repository root so that `examples/`, `assets/`
and `web/` are reachable. The engine's `.wasm`/`.js` output goes to `web/pkg/` (gitignored).
