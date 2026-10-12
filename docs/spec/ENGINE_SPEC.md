# Ferrocade Engine Spec

A game engine built from scratch. Users write games in Python, and a Rust backend does the heavy lifting.

**Working name:** Ferrocade (*ferro*, iron, a nod to Rust, + *arcade*). It may change later.
- Python: `pip install ferrocade`, `import ferrocade as fc`
- Rust crates: `ferrocade` (core engine). The PyO3 bindings crate is internal.
- Was free on PyPI and crates.io as of 2026-10-12 (not yet reserved; no trademark search done).

> Living document: specs are added here as they're decided. Items marked **TBD** are still open.

---

## 1. Goals

- **Users develop in Python.** The public API is entirely Python calls.
- **Backend in Rust** for performance. Users never need to touch Rust.
- Build the engine from scratch: write our own engine rather than wrapping an existing one.
- **Target platforms: Windows, macOS, and web browsers (WASM).** The prototype is Windows-only, but every design decision must keep the port to macOS and web open (see §9).
- **2D only for the prototypes.** 3D is out of scope: we don't design for it, add abstractions for it, or pay costs for it. If 3D ever happens, it will be its own design effort later.

## 2. Architecture Overview

```
┌──────────────────────────────┐
│   Game code (Python)         │  ← user writes this
├──────────────────────────────┤
│   Python API / bindings      │  (PyO3)
├──────────────────────────────┤
│   Engine core (Rust)         │
│   ├─ Renderer (wgpu)         │
│   ├─ Windowing/input (winit) │
│   └─ ... TBD subsystems      │
├──────────────────────────────┤
│   OS graphics API            │
│   Vulkan / Metal / D3D12 /   │
│   OpenGL-GLES fallback       │
└──────────────────────────────┘
```

## 3. Python API: Version 1

### v1 goal

> Open a window and draw `ship.png` in the center of the screen.

### Class design

We compared the class structure of other engines:

| Engine | Core classes | Notes |
|---|---|---|
| **Python Arcade** | `Window`, `Sprite`, `SpriteList`, `Texture`, `View`, `Camera2D` | `SpriteList` batches sprites into GPU buffers. This is the closest model to ours. |
| **Pygame** | `Surface`, `sprite.Sprite`, `sprite.Group`, `display` | `Group` is equivalent to `SpriteList`. CPU blitting, no GPU batching. |
| **Unity** | `GameObject` + components (`SpriteRenderer`, `Transform`), `Scene`, `Camera` | Composition-based. Powerful, but heavy for a v1. |
| **Godot** | `Node` tree (`Sprite2D`, `Node2D`, `Camera2D`), `Texture2D` | Scene graph. |

**Verdict:** `Window` / `Sprite` / `SpriteList` is a good foundation. It matches Arcade's proven design, and `SpriteList` maps cleanly to GPU batching (one instance buffer gives one draw call per texture), which is exactly where Rust + wgpu pays off.

**Addition (adopted): a public `Texture` class**, kept separate from `Sprite`:
- A sprite is *where* and *how* something is drawn (position, scale, rotation). A texture is *what* is drawn (the GPU image).
- 100 enemy ships should share **one** GPU texture, not upload `ship.png` 100 times.
- Users can create a `Texture` explicitly and share it between sprites. For convenience, `Sprite("ship.png")` also accepts a path and loads it through the same texture cache.

**Deferred to later versions:**
- `View` / `Scene`: screens such as menu, game, and game over (Arcade-style views).
- `Camera`: scrolling and zoom.
- Components or ECS (Unity/Bevy-style): revisit only if the simple object model becomes limiting.

### v1 classes

#### `Window`
- Opens an OS window with a wgpu surface and owns the main loop.
- Constructor: `Window(width, height, title, *, filter, update_rate=1/60)`. `filter` is **required** (see "Texture filtering" below).
- Properties: `width`, `height`, `filter`
- Methods: `run()` (blocks and runs the event loop), `clear(color=...)`
- Overridable callbacks: `on_draw()` (once per displayed frame), `on_update(delta_time)` (fixed rate; `delta_time` == `update_rate`)
- Rust side: winit event loop, wgpu device/queue/surface, frame presentation.

#### `Texture`
- An image loaded onto the GPU, shareable between any number of sprites.
- Constructor: `Texture(path, *, filter=None)`. Loading goes through a cache keyed by path, so loading the same file twice shares the same image data. `filter` optionally overrides the window's filter for this texture.
- Properties (read-only): `width`, `height` (in pixels), `path`, `filter`
- Rust side: decodes the image (`image` crate), uploads it to a wgpu texture, and creates a sampler and bind group. The cache holds reference-counted handles.
- **Lazy GPU upload:** a `Texture` can be created before the `Window` exists. The file is decoded right away, and the GPU upload happens on first use once a wgpu device is available.

#### `Sprite`
- Holds a reference to a texture plus its transform.
- Constructor: `Sprite(texture, center_x=0, center_y=0, scale=1.0)`, where `texture` is either a `Texture` or a file path (a path is loaded through the texture cache).
- Properties: `texture` (can be reassigned, e.g. to swap images), `center_x`, `center_y`, `position` (tuple; sets both in one call), `scale`, `angle` (degrees; can be stubbed in v1), `width`, `height` (texture size × scale)
- Motion properties (per second, see §8): `velocity_x`, `velocity_y`, `angular_velocity`. They're not needed for the v1 goal, but the storage is planned from the start.
- Rust side: transform data plus a texture handle.

#### `SpriteList`
- Holds sprites and draws them in a batch.
- Methods: `append(sprite)`, `remove(sprite)`, `draw()`, `len()`
- Rust side: an instance buffer of sprite transforms, with one draw call per texture.

### Image formats (decided)
- **v1 requirement: PNG** (including transparency).
- Long-term goal: support many formats.
- Decoding uses the Rust `image` crate, where each format is a Cargo feature flag. Adding a format costs almost no code, only some binary size and compile time.
- **v1 enabled formats:** `png` (required), plus `jpeg`, `bmp` and `gif` (first frame only) because they are effectively free.
- **Later, low effort:** `webp`, `tga`, `qoi`, `ico` (just enable more features).
- **Later, real work:** animated GIF/WebP (needs the animation system), compressed GPU formats (KTX2/DDS/Basis), and SVG (needs a vector rasterizer such as `resvg`).
- Unsupported or corrupt files raise a clear Python exception naming the file and format.

### Texture filtering (decided): no default
- The choice between **nearest** (crisp pixels, for pixel art) and **linear** (smooth, for scaled or rotated or photographic art) depends on the game, and this question comes up repeatedly. **The engine has no default.**
- **`Window(..., filter=fc.NEAREST | fc.LINEAR)` is a required keyword argument.** Leaving it out is a `TypeError`. It applies to every texture without an override.
- **Per-texture override:** `Texture(path, filter=fc.LINEAR)`. `None` (the default for `Texture`) means "use the window's filter".
- The filter is a sampler choice, not part of the image data, so overriding it never reloads the file.
- Design details: `specs/001-centered-ship/research.md` R11.

### Colors (decided)
- **`fc.color` provides every CSS named color** (CSS Color Module Level 4: the 148 named colors, plus `TRANSPARENT`). The values match CSS exactly, so web color references work as-is.
- **Naming:** `UPPER_SNAKE` versions of the CSS keywords, e.g. `CORNFLOWER_BLUE` (100, 149, 237), `DARK_SLATE_GRAY` (47, 79, 79), `REBECCA_PURPLE` (102, 51, 153). CSS's `gray`/`grey` spelling pairs are both included, as in CSS.
- **CSS wins on conflicts:** `GREEN` is (0, 128, 0) (use `LIME` for (0, 255, 0)), and `DARK_GRAY` is (169, 169, 169).
- **Non-CSS engine colors** are named separately. The default clear color (32, 32, 32) becomes **`DEFAULT_BACKGROUND`**.
  - *This replaces the Phase 1 constants `GREEN = (0, 255, 0)` and `DARK_GRAY = (32, 32, 32)`.*
- **Format:** RGB tuples of 0-255 integers, as today (`TRANSPARENT` is RGBA `(0, 0, 0, 0)`). Any 3- or 4-tuple is still accepted wherever a color goes.
- The desktop and web packages share one generated or copied list. That's part of the web "required before shipping" item to share `color.py`.

### Shape sprites (decided): solid-color rectangles and circles without images
- **Requirement:** create sprites that are solid-color **rectangles** or **circles** of any size, with no image file needed. They behave like any other sprite: position, scale, sprite lists, batching, and collisions later.
- **Design: tinted shared textures.**
  - Each sprite gets a **`color`** (tint, RGBA) that's multiplied with its texture in the shader. **Default: white (255, 255, 255, 255), which means unchanged**, so existing image sprites look the same.
    - The tint is stored in the GPU instance's existing padding as 4 normalized bytes (`Unorm8x4`), so the instance stays **48 bytes** (constitution II).
    - Like every sprite property, it lives in the Rust store and changing it marks just that slot dirty.
  - **Rectangles** share one small built-in **white texture**, and the sprite's size comes from width × height.
  - **Circles** share one built-in **white circle texture** (high resolution, anti-aliased edge), drawn at diameter × diameter.
  - Because shapes of any size or color reuse the same textures, **they batch into the same draw calls** as each other.
- **Side benefit:** `sprite.color` also tints and fades image sprites (for example, a damage flash, or alpha fade-out).
- **API (decided 2026-10-12):** dedicated sprite classes that extend `Sprite`:
  - `fc.RectangleSprite(width, height, color, center_x=0, center_y=0)`
  - `fc.CircleSprite(radius, color, center_x=0, center_y=0, *, filter=None)`
  - Game classes extend them as usual (`class Wall(fc.RectangleSprite)`), and every sprite gets a `color` property.
- **Circle edges** follow the window's filter (stepped with `NEAREST`, smooth with `LINEAR`). A circle may override this with `filter=`, the same rule as `Texture(..., filter=...)`.

### Coordinate system (decided)
- The origin `(0, 0)` is the **bottom-left** corner of the window, with **y going up** (same as Arcade and OpenGL).
- Units are pixels. `center_x`/`center_y` give the sprite's center point.
- `angle` is in degrees. **A positive angle rotates counter-clockwise** (the math convention, consistent with y-up: heading `angle` means direction `(cos, sin)`). Decided 2026-10-12; Unity and Bevy do the same, while Arcade 3.x and pyglet use clockwise.

### API style (decided)
- Users **subclass `Window`** and override callbacks (`on_draw`, `on_update`, and later input callbacks).

### v1 target usage

```python
import ferrocade as fc


class Game(fc.Window):
    def __init__(self):
        super().__init__(800, 600, "Ferrocade", filter=fc.NEAREST)
        self.sprites = fc.SpriteList()
        ship_texture = fc.Texture("ship.png")
        ship = fc.Sprite(ship_texture, center_x=self.width / 2, center_y=self.height / 2)
        self.sprites.append(ship)
        # Shorthand, using the same cached texture: fc.Sprite("ship.png", ...)

    def on_draw(self):
        self.clear()
        self.sprites.draw()


Game().run()
```

### v1 open questions
- [x] Coordinate system: bottom-left origin, y up
- [x] API style: subclass `Window`
- [x] Image formats: PNG required for v1; see "Image formats" below

## 4. Rendering

### Decision: wgpu as the graphics abstraction

- **Library:** [wgpu](https://wgpu.rs) (Rust).
- **Why:**
  - Runs natively and standalone, with no browser needed. It compiles into the executable.
  - One API that maps to Vulkan, Metal, D3D12 and OpenGL/GLES.
  - Keeps the option of WASM/WebGPU builds open (with WebGL2 as a fallback).
  - Can run headless for offscreen rendering, compute, and automated tests.
  - Mature and proven in production (Bevy, Firefox's WebGPU).
- **Rejected alternatives:**
  - Raw Vulkan/D3D12: too verbose for the scope, and still needs separate backends per platform.
  - OpenGL directly: simpler, but deprecated on macOS and stuck at 4.1 there.
  - Existing frameworks (SDL renderer, pyglet/arcade): don't fit the "from scratch" goal.

### Backend targets

| Platform | Status | Primary backend | Fallback |
|---|---|---|---|
| Windows | **Prototype target** | D3D12 / Vulkan | OpenGL |
| macOS | Required | Metal | — |
| Web | Required | WebGPU | WebGL2 |
| Linux | Unofficial (likely works for free) | Vulkan | OpenGL |

**GPU selection (decided 2026-10-12):** prefer the high-performance (discrete) GPU on machines with more than one. On a dual-GPU laptop, the default choice was the integrated GPU, which cut the GPU-bound Phase 3 scenes by about 5× (`docs/research/phase3-benchmarks.md`). On the web, this is only a hint to the browser.

### v1 rendering approach (2D)
- One textured-quad pipeline written in WGSL.
- Per-sprite instance data: position, size, rotation, and texture region.
- An orthographic projection matching window pixel coordinates (bottom-left origin).
- **No depth buffer.** Draw order is the layering: `SpriteList`s draw in the order their `draw()` is called, and sprites within a list draw in list order (later ones on top).
- Alpha blending on, for PNG transparency.

## 5. Windowing & Input

- **Library:** winit (pairs natively with wgpu surfaces).
- winit uses a top-left origin, so the engine flips mouse coordinates to bottom-left before passing them to Python.
- Note: the winit event loop must run on the main thread (required on macOS), and it calls back into Python.
- `Window.run()` blocks on desktop. On the web it hands control to the browser's frame loop and returns or never returns, depending on the binding approach (§9). User code should treat `run()` as the last line of the program.
- **Keyboard (implemented in Phase 4, 2026-10-12):**
  - `on_key_press(key, modifiers)` / `on_key_release(key, modifiers)` `Window` callbacks, and `Window.is_key_pressed(key)` polling.
  - Keys are **physical positions** (`fc.key.W` is the key in QWERTY's W position on any layout), named after the W3C UI Events `code` values in one core table (`crates/ferrocade/src/input.rs`) that both winit and the browser map to. `fc.key` (`python/ferrocade/key.py`) is generated from that table and shared by the web build.
  - Key names: `A`–`Z`, `KEY_0`–`KEY_9`, `F1`–`F12`, arrows, `SPACE`, `ENTER`, `ESCAPE`, `TAB`, `PAGE_UP`/`PAGE_DOWN`, `BRACKET_LEFT`/`BRACKET_RIGHT`, sided modifiers (`LSHIFT`, `RCTRL`, `LSYSTEM`, ...), and keypad `NUM_*`.
  - One press and one release per keystroke: operating-system auto-repeat is ignored.
  - `modifiers` = `MOD_SHIFT | MOD_CTRL | MOD_ALT | MOD_SYSTEM` flags for the keys held **after** the event (either side counts).
  - Events are queued and delivered once per frame, before that frame's `on_update` calls, so callbacks and polling always agree.
  - Losing focus releases every held key (with `on_key_release` for each).
  - Browser: keys reach the game while the canvas has focus (focused on start and on click). Game keys don't scroll or trigger page features; Ctrl/Alt/Meta combinations, F1–F12 and Tab keep their browser behavior.
- **TBD:** mouse callbacks, gamepad support (e.g. gilrs), text input.

## 6. Python Integration

- **Bindings:** PyO3, with maturin for building and packaging.
- **Minimum Python version: 3.13.** Backward compatibility isn't a goal yet, so the floor is set by what's useful, not by reach:
  - It matches the development machine (3.13), so there's no untested older version to support.
  - **It keeps the web path open:** Pyodide (§9) ships a specific CPython version that typically lags the newest release. A floor newer than Pyodide's Python would block the Pyodide option. Check Pyodide's current version in the Phase 1b spike.
  - Build with PyO3's stable ABI (`abi3-py313`), so one compiled wheel per platform works on 3.13 and every later version.
  - Going newer (3.14/3.15) brings no feature the engine needs today. Raise the floor later if a concrete benefit appears (e.g. free-threading or newer stable-ABI features).
- Rust owns the main loop, rendering, and hot paths, while Python owns game logic.
- Python callbacks (`on_draw`, `on_update`) are called from the Rust event loop while holding the GIL.
- **Alternative considered:** wgpu-py. It was rejected as the primary path because it puts rendering in Python.
- **TBD:** hot-reload of scripts.

## 7. Performance: Sprite Data & Float Handling

*(Strategy confirmed: minimize number conversion across the Python/Rust boundary.)*

### The problem
- A Python `float` is a heap-allocated object (a boxed `f64`). The GPU wants packed `f32` values in a contiguous buffer.
- If sprite data lives in Python objects, every frame needs a loop over each sprite: read the attributes (Python objects) → convert to `f32` → pack into a buffer → upload. That per-sprite, per-frame boundary crossing is the cost.
- A single `sprite.center_x += 1` from Python can never be free, because Python always produces a boxed float. The goal is to **make each access cheap, and to make most sprites need zero Python-side accesses per frame.**

### Strategy

**1. Native storage: sprite data lives in Rust, never in Python attributes.**
- `Sprite` is a `#[pyclass]` whose fields are plain Rust numbers. Python attributes are getters/setters.
- Setting `sprite.center_x = 5.0` converts **once**, at assignment. Nothing is stored as a Python float.
- `sprite.position = (x, y)` / `sprite.set_position(x, y)` set both values in one boundary crossing.

**2. GPU-ready buffers: each `SpriteList` owns its instance buffer in GPU layout.**
- When a sprite is appended, it records `(list, slot)` back-references.
- A setter writes the new value directly into each owning list's packed `f32` buffer and marks that slot dirty.
- `SpriteList.draw()` does **no per-sprite conversion**. It uploads only the dirty range (or nothing if unchanged), then issues the draw calls.
- Supports one sprite being in multiple lists (e.g. "all sprites" and "enemies").

**3. Move per-frame work into Rust.**
- Built-in motion properties (`velocity_x`, `velocity_y`, `angular_velocity`; see §8), applied by `SpriteList.update(delta_time)` in Rust in one call for all sprites.
- Python sets a velocity **once**, and Rust moves the sprite every tick with zero boundary crossings.
- Collision checks (`sprite.collides_with_list(...)`) run in Rust and return only the hits.
- Result: in a typical arcade game, most sprites are never touched from Python each frame.

**4. Bulk access: zero-copy NumPy views (post-v1, optional dependency).**
- `sprite_list.positions` returns a NumPy view over the Rust buffer (via `rust-numpy` / the buffer protocol). No copying.
- Vectorized updates such as `positions[:, 1] -= 2.0 * dt` touch thousands of sprites with a single Python operation.
- Writes through the view mark the whole buffer dirty.

**5. Cheap accessors in PyO3.**
- Getters/setters for hot properties are written directly in Rust (not routed through Python-level logic).
- Keep per-access overhead minimal (avoid unnecessary borrow/ownership checks; benchmark).

### Precision
- **Decided:** `Sprite` stores **`f64`** (matches Python's float, so values round-trip exactly: setting `0.1` reads back as `0.1`). The setter writes **`f32`** into each owning `SpriteList`'s GPU buffer.
- The f64→f32 conversion happens in Rust and is trivially cheap.
- Rejected: `f32`-only storage, because `sprite.center_x = 0.1` would read back as `0.10000000149...` and surprise `==` comparisons.

### Benchmarks to keep
- [ ] Draw N static sprites (should cost ~0 Python time per frame)
- [ ] Move N sprites from a Python loop (per-access cost)
- [ ] Move N sprites via `velocity_x`/`update()` (Rust path)
- [ ] Move N sprites via NumPy view (bulk path)
- [ ] Compare against Python Arcade and Pygame for the same scenes

## 8. Motion & Timing

*(Confirmed.)*

### Units: per second, not per frame
- All motion is expressed **per second**: velocity in pixels/second, angular velocity in degrees/second.
- **Why:** per-frame movement ties game speed to frame rate. The same game runs 2.4× faster on a 144 Hz monitor than at 60 Hz, and slows down when frames drop. Per-second movement is frame-rate independent.
- **What other engines do:**

| Engine | Units |
|---|---|
| Unity, Godot, Bevy, Unreal | Per second (multiply by delta time) |
| Pygame | Per frame by convention (user's choice) |
| Python Arcade | `change_x` is per frame |

- **Naming:** use `velocity_x` / `velocity_y` / `angular_velocity` instead of Arcade's `change_x`, so Arcade users don't assume per-frame semantics.

### Fixed timestep
- Pure variable delta time (`position += velocity * dt` with whatever `dt` the frame took) has problems: physics and collisions behave differently at different frame rates, fast objects can tunnel through walls during a lag spike, and replays are non-deterministic.
- **Solution: a fixed-timestep simulation loop in Rust** ("Fix Your Timestep" accumulator pattern):
  - `on_update(delta_time)` runs at a fixed rate (default **60 Hz**, configurable via `Window(update_rate=...)`), so `delta_time` is constant (1/60).
  - `on_draw()` runs once per displayed frame (vsync), independent of the update rate.
  - If the game falls behind, multiple updates run before the next draw, up to a cap (e.g. 5 per frame), to avoid a "spiral of death".
  - **Later:** interpolate draw positions between the last two updates for smooth motion on high refresh-rate displays.
- Users still write `velocity_x = 200` (pixels per second). Rust computes `x += velocity_x * dt` internally.

### Conversion cost
- Velocities are stored natively in Rust (same as positions, §7), and the `* dt` math happens in Rust.
- The only per-tick crossing is passing `delta_time` (one float) to `on_update`, which is negligible.

## 9. Platforms & Portability

### Targets
- **Windows:** prototype platform, developed first.
- **macOS:** Apple Silicon and Intel.
- **Web:** runs in the browser via WebAssembly.
- **Linux:** not a requirement, but wgpu + winit make it close to free, so don't break it.

### Rules for the prototype (so porting stays cheap)
1. **No platform-specific code outside a platform layer.** No Win32/D3D calls in engine code. Everything goes through wgpu and winit.
2. **WebGL2-compatible rendering.** Request wgpu's `downlevel_webgl2_defaults()` limits by default, so the renderer works on the weakest backend. This means no compute shaders or storage buffers in the core renderer. Instanced quads with vertex/instance buffers are fine. Higher features can be enabled per-backend later.
3. **WGSL shaders only.** naga translates them to HLSL, MSL, SPIR-V and GLSL for every backend.
4. **Don't block the main loop.** Browsers can't run a blocking `while` loop. Use winit's event-driven loop (`ApplicationHandler`) on all platforms, so desktop and web share the same code path.
5. **Treat GPU init as async.** Requesting an adapter/device is async on web. Structure init as a state machine or future, not `block_on` everywhere. (On desktop, blocking is fine behind the platform layer.)
6. **No `std::time::Instant` directly.** It panics on `wasm32-unknown-unknown`. Use the `web-time` crate (it re-exports `std::time` on desktop).
7. **No required threads in the core.** Threads on the web are restricted. Background work (asset decode, audio) must have a single-threaded fallback.
8. **Abstract file loading.** Browsers have no filesystem. `Texture("ship.png")` must work through an asset loader that reads files on desktop and fetches/preloads on web. Lazy GPU upload (§3) already helps here.
9. **Main-thread rule.** macOS (and the web) require the event loop and window on the main thread. `Window.run()` must be called from Python's main thread.

### Crate layout
Keep the engine core independent of Python, so the web build can choose its own binding strategy:
- `crates/ferrocade/`: Rust core (sprites, textures, timing, renderer via wgpu, platform via winit). No PyO3 dependency.
- `crates/ferrocade-py/`: PyO3 bindings, a thin wrapper that builds the native module for the `ferrocade` Python package.

See §13 for the full repository layout.

### Web: how Python runs in the browser (needs research)
This is the **hardest platform requirement**. The engine API is Python, so the browser needs a Python runtime.

| Option | How | Pros | Cons / risks |
|---|---|---|---|
| **A. Pyodide + engine compiled for Pyodide** | CPython-in-WASM (Pyodide). Build our PyO3 module as a Pyodide wheel (`wasm32-unknown-emscripten`, supported by maturin/PyO3). | Same Python API and same bindings as desktop. Real CPython, so user code works unchanged. | wgpu and winit primarily target `wasm32-unknown-unknown`, and their Emscripten support is limited. Might need a custom web platform layer. Large download (Pyodide ~10 MB+). |
| **B. Pyodide + separate engine WASM module** | Engine built as `wasm32-unknown-unknown` (where wgpu/winit web support is strongest), exposed to JS. A Python-side shim calls it through Pyodide's JS bridge. | Uses the best-supported wgpu/winit web path. | Two bindings to maintain. Python→JS→WASM calls are slower, which hurts the "minimize conversions" goal. Bulk APIs (§7) become more important. |
| **C. RustPython** | A Python interpreter written in Rust, compiled into the same WASM as the engine. | Single WASM module, no Emscripten. | Incomplete CPython compatibility, slower interpreter, no C-extension ecosystem. |

- **Decided (2026-10-12, after the Phase 1b spike): Option B.** Pyodide plus the engine as a separate wasm-bindgen module, with a browser Python package implementing the same API. It's proven to run the Phase 1 example unchanged on WebGPU and WebGL2.
- **Revisit Option A** when all of these land: wgpu Emscripten support (gfx-rs/wgpu#10515), a Pyodide ABI with Emscripten ≥ 6.0.10 (pyodide#6476, expected with Python 3.16), and wasm-bindgen side-module support. To re-test, run `spike/pyodide-wheel/build.sh render`. A's wheel pipeline (A1) already works.
- **Action:** run a **web feasibility spike early** (Phase 1b, right after v1 works on Windows). Build the v1 example (window + centered ship) under Pyodide before the engine grows. The result decides whether the platform layer needs a custom web backend.
- **Phase 1b spike results:** `docs/research/web-spike.md` (recommendation accepted 2026-10-12). Before shipping web support, do the "required before shipping web" items in its engine-impact table (a real web package, a shared API test suite run against both bindings, an asset manifest, a shared `color.py`).
- Web-specific notes for later: Pyodide can run in a Web Worker with `OffscreenCanvas`. Audio requires a user gesture before starting. Assets must be preloaded into Pyodide's virtual filesystem or fetched asynchronously.

### Distribution (later)
- Desktop: platform wheels via maturin (`win_amd64`, `macosx` universal2 or arm64 + x86_64), published to PyPI.
- macOS: shipped standalone games need code signing and notarization.
- Web: Pyodide wheel plus a small HTML/JS loader template.
- CI build matrix (GitHub Actions: Windows, macOS, web) once the prototype stabilizes.

## 10. Roadmap: Phases

Each phase ends with a runnable example in `examples/` that demonstrates its goal. Phases run on Windows unless noted.

### Phase 1: Window + centered ship (v1)
- **Status (2026-10-12): implemented on Windows.** All automated checks pass (27 Rust tests, including offscreen GPU render checks, and 43 pytest tests). The visual and interactive checks (quickstart V3–V6, V9, V10) are pending the owner. See `docs/research/phase1-validation.md`.
- **Goal:** open a window and draw `ship.png` centered (see §3).
- **Proves:** the whole stack works: Python → PyO3 → Rust → wgpu → screen.
- **Delivers:** `Window`, `Texture`, `Sprite`, `SpriteList`, a texture cache, the textured-quad pipeline, and PNG loading.
- **Done when:** `python examples/01_centered_ship.py` shows the ship centered, the window closes cleanly, and resizing doesn't crash.

### Phase 1b: Web feasibility spike (after Phase 1; can run in parallel with Phase 2)
- **Goal:** run the Phase 1 example in a browser (see §9, "Web").
- **Proves:** which Python-in-browser approach works (Pyodide wheel vs. separate WASM).
- **Done when:** the centered ship renders in a browser, or the spike documents why the approach fails and which option to try next.
- Quick macOS check here too, if hardware is available.

### Phase 1c: Colors and shape sprites
- **Status (2026-10-12): implemented** on desktop and web. See `docs/research/phase1c-validation.md` (1,000 mixed shapes = 1 draw call; all 148 colors verified against Chrome).
- **Goal:** CSS named colors in `fc.color`, and solid-color rectangle and circle sprites that need no image (see §3 "Colors" and "Shape sprites").
- **Proves:** per-sprite tint in the instance data (still 48 bytes, dirty-tracked), shared built-in textures, and batching across shapes.
- **Delivers:**
  - the full CSS color set, plus `DEFAULT_BACKGROUND`
  - `sprite.color`
  - rectangle and circle sprite creation
  - the same API in the web shim
  - an example, `examples/01c_shapes.py`
- **Done when:**
  - the example shows several rectangles and circles of different sizes and colors, drawn in few draw calls
  - an image sprite with the default color looks unchanged (the Phase 1 tests still pass)
  - colors match CSS values (tested)
  - it works on desktop and in the browser

### Phase 2: Bouncing ship (movement)
- **Status (2026-10-12): implemented** on desktop and web, with counter-clockwise rotation. See `docs/research/phase2-validation.md`: 601 updates in 10 s; 1,000 bouncing shapes at 60 FPS; the engine moves 1,000 sprites in 0.087 ms vs. 1.34 ms for Python edge checks.
- **Goal:** the ship moves and bounces off the window edges.
- **Proves:** fixed-timestep loop, `on_update(delta_time)`, per-second velocities, and dirty-buffer updates (§7, §8).
- **Delivers:** `velocity_x`/`velocity_y`/`angular_velocity`, `SpriteList.update()` (motion applied in Rust), and the `update_rate` setting.
- **Example logic:** Python checks edges in `on_update` and flips the velocity sign. Rust does the movement.
- **Done when:** the speed is identical at different monitor refresh rates (e.g. 60 Hz vs. 144 Hz, or vsync off), and the motion is smooth.

### Phase 3: Sprite stress test (performance)
- **Status (2026-10-12): implemented; target met.** See `docs/research/phase3-benchmarks.md`. Maximum engine-moved sprites at 60 FPS: **100,000** on the default (integrated Intel) GPU, and ≥ 512,000 on the NVIDIA GPU (temporary experiment). For comparison: Arcade 4.0.0.dev8 reached 8,000 and pygame-ce 16,500. Delivered: `benches/stress.py` (one process per engine/scene), `Window.frame_stats`, and `Window.show_fps` (title-bar readout). No optimization was needed. Follow-up: Ferrocade now prefers the high-performance GPU (§4).
- **Goal:** draw and move a large number of sprites at a steady frame rate.
- **Proves:** batching, GPU-layout buffers, and minimal conversion (§7) actually pay off.
- **Delivers:** an FPS / frame-time readout (console or title bar until text rendering exists), and the benchmark scenes from §7.
- **Scenes:** N static sprites, N moved via Rust `update()`, N moved from a Python loop. Optionally, N moved via NumPy views.
- **Target (decided 2026-10-12):** 10,000 engine-moved sprites at 60 FPS on the development machine. The other scenes are measured, not pass/fail. Compare with Python Arcade 4.0 (pre-release) and pygame-ce (dev-only installs) on the same scenes. The NumPy bulk path is deferred to the backlog.
- **Done when:** the targets are met, or the bottlenecks are profiled and documented.

### Phase 4: Keyboard control
- **Status (2026-10-12): implemented** on desktop and web (WebGPU and WebGL2). See `docs/research/phase4-validation.md`. Example: `examples/04_keyboard_ship.py`. Manual checks are pending the owner.
- **Goal:** steer the ship with the keyboard.
- **Proves:** input events flow from winit → Rust → Python callbacks.
- **Delivers:**
  - Callbacks: `on_key_press(key, modifiers)` and `on_key_release(key, modifiers)` (`Window` subclass overrides).
  - Key constants (e.g. `fc.key.LEFT`, `fc.key.A`), using physical key codes so WASD works on any keyboard layout.
  - Optional polling: `window.is_key_pressed(key)`, which suits continuous movement.
- **Done when:** arrow keys/WASD move the ship smoothly, holding multiple keys works (e.g. diagonals), and nothing is lost on key repeat.
- **Later in this area:** mouse (with coordinates flipped to bottom-left, §5), gamepad.

### Phase 5: Collisions
- **Goal:** detect collisions between the ship and other sprites (e.g. asteroids or walls).
- **Proves:** a Rust collision system that is fast enough for the Phase 3 sprite counts.
- **Requirement:** all collision detection runs in **Rust**. Python only receives results.
- **Design:** architecture decided (§11, `docs/design/collision.md`); the API details are settled in the Phase 5 spec.
- **Done when:** the ship collides correctly with many objects, and collision checks for Phase 3-scale lists fit within the frame budget.

### Milestone: Performance showcase demo (after Phase 5)
- **Requested by the owner (2026-10-12):** a demo that shows off Ferrocade's speed.
- **Scene:**
  - a window with a **wall around the border** (rectangle sprites, Phase 1c)
  - **lots of small sprites** inside (circles and/or images), **all moving** (Phase 2) and **bouncing off the walls and off each other**
- **It must visibly show three speeds:**
  1. **Rendering:** how many sprites are drawn per frame, and the frame time or FPS.
  2. **Moving:** the time to update every sprite's position each frame (Rust-side `update()`).
  3. **Collision checks:** the time to detect all sprite-vs-sprite and sprite-vs-wall contacts each frame (Phase 5, in Rust).
- **Builds on:** Phase 1c (walls and balls as shapes), Phase 2 (velocities, `SpriteList.update()`), Phase 3 (stress-test scenes, frame-time readout), and Phase 5 (Rust collision detection).
- **Needs beyond the current phases** (to decide when the demo is specified):
  - **Collision response:** "bouncing off each other" needs more than detection, namely reflecting or exchanging velocities on contact. Phase 5 is detect-only, and physics response is in the backlog.
    - **Owner direction:** Python manages collisions, while Rust does the time-expensive parts: movement and detection. Phase 5 delivers batched queries (§11). **Ideas for the showcase** (not committed): batched enter/exit events and built-in Rust responses (bounce, stop, solid walls).
    - **Demo idea:** a toggle between "bounce handled in Rust" and "bounce handled in Python from the collision results", so the frame-time difference shows the value of the hybrid design.
  - **Timing breakdown API:** engine-side per-frame timings for render, update, and collision. `FrameStats` from Phase 1c is a start.
  - **On-screen display of the numbers:** the title bar or console until text rendering exists, then an overlay.
  - **Controls:** add or remove sprites while running (e.g. keys from Phase 4) to find the limit.
  - **Web version:** the same demo in the browser, to show the web per-call cost (web spike risk 2).
- **Done when (draft):** thousands of sprites bounce correctly at the refresh rate on the dev machine, with live counts and the three timings shown, on desktop and in the browser.

### Later phases (unordered)
- Sound, text rendering, sprite sheets/animation, views/scenes, camera, a physics response (bouncing off, pushing), and mouse/gamepad input.

## 11. Collision System

*(Architecture decided by the owner, 2026-10-12. Full design: `docs/design/collision.md`. API details are Proposed until the Phase 5 spec.)*

### Decided architecture (2026-10-12)
1. **Colliders are separate entities** (Unity/Godot style), attached to sprites and following their transform; not a built-in property of `Sprite`. They're **attached explicitly** (no automatic colliders, even on shape sprites). Colliders without a sprite are in the backlog.
2. **Shapes:** start with **circle, axis-aligned box, and rotated box**, as `CircleCollider`, `AABBCollider` (never rotates), and `BoxCollider` (rotates with the sprite). **Collider scale is coupled** to `sprite.scale` (sizes are in unscaled sprite pixels, as in Unity, Godot, and Arcade). Collider **offsets** scale too, and rotate with the sprite for `CircleCollider` and `BoxCollider` (not for `AABBCollider`). The design must stay expandable (shape enum + pair tests, with a generic convex fallback) so convex polygons, auto hit boxes from texture alpha, capsules, and pixel masks can be added later.
3. **What is checked:** **list-based queries** (Arcade style). Declared pairs checked automatically every frame are in the backlog.
4. **Results to Python:** **batched queries** first. Enter/exit events and built-in Rust responses (bounce/stop/solid) are ideas for the showcase, not committed.
5. **Broad phase:** a **spatial hash grid per list**, rebuilt only when the list's positions changed (reusing dirty tracking); very small lists test every pair.
6. **Implementation:** **written from scratch** (no `parry2d`/`rapier2d`).

The sections below are the original research and options, kept for reference.

### Requirements so far
- Implemented in Rust, with no per-object Python work during collision checks.
- Must scale to Phase 3 sprite counts.
- Works on all platforms, including WASM (single-threaded fallback, §9).

### How other engines do it

| Engine | Structure | Shapes | API style |
|---|---|---|---|
| **Unity (2D)** | Colliders are **separate components** from rendering (`SpriteRenderer`) and physics (`Rigidbody2D`). Layers + a collision matrix decide what can hit what. | Box, Circle, Capsule, Polygon, Edge, Composite | Events: `OnCollisionEnter2D` / `OnTriggerEnter2D` (trigger = detect only, no physical response) |
| **Godot** | `CollisionShape2D` attached to a body or `Area2D`. Collision layers + masks. | Rectangle, Circle, Capsule, Convex/Concave polygon, Segment | Signals (`body_entered`) + direct queries |
| **Python Arcade** | A hit box polygon on each sprite, generated automatically from the texture's alpha channel. A spatial hash is optional per `SpriteList`. | Polygon (auto or manual) | Queries: `check_for_collision_with_list(sprite, list)` |
| **Pygame** | Rect on each sprite, with an optional mask. | Rect, circle, pixel mask | Queries: `spritecollide(sprite, group)` |

### Design questions to answer
1. **Where colliders live.** Do they belong to `Sprite` (simple, Arcade-like), or are they a separate collider object/system (Unity-like, which decouples collision from rendering and allows invisible colliders and multiple shapes per object)? The Unity separation is attractive. Possible middle ground: sprites get a default collider automatically, but colliders are their own type and can exist without a sprite.
2. **Shapes to support.** Candidates, roughly in order of cost:
   - **Circle:** cheapest, good for round objects and bullets.
   - **AABB** (axis-aligned box): cheap, but doesn't rotate.
   - **OBB** (rotated box): handles rotated sprites.
   - **Capsule:** good for characters.
   - **Convex polygon:** accurate hit boxes (SAT test).
   - **Concave polygon / mesh:** decomposed into convex parts, or used for static level geometry.
   - **Pixel-perfect:** most accurate and most expensive. Possibly an optional final check.
3. **Automatic hit boxes.** Generate a polygon from the texture's alpha channel (like Arcade), once per texture and cached alongside it.
4. **Broad phase** (quickly finding candidate pairs): spatial hash grid (simple, good for similar-sized sprites), sort-and-sweep, or a BVH/quadtree (better for mixed sizes and static geometry).
5. **Narrow phase** (exact shape tests): specialized circle/box tests, SAT for polygons, and GJK as a general fallback.
6. **API style: hybrid, with Rust doing the heavy work and Python handling the game logic** *(owner direction, 2026-10-12; mechanism Proposed)*.
   - **Principle:** movement and collision **detection** always run in Rust. Python is called only to *manage* collisions (game logic: score, sound, damage), never for the expensive per-frame work.
   - **Ways to deliver collisions to Python** (brainstorm):

     | # | Mechanism | Python calls per frame | Notes |
     |---|---|---|---|
     | 1 | Per-pair callback `on_collision(a, b)` | One per colliding pair | Convenient. The bottleneck at thousands of contacts. |
     | 2 | Batched query `for a, b in sprites.collisions():` | One fetch plus a Python loop | Detection crosses the boundary once. |
     | 3 | Enter/exit events `on_collision_enter/exit(a, b)` | Only when contact starts or ends | Unity-style. Resting contacts don't re-fire. |
     | 4 | Built-in responses in Rust (`collision_response = fc.BOUNCE`, solid walls) | Zero for the response | The fast path for physics-like behavior. |
     | 5 | Bulk index arrays (`collision_pairs()` returns NumPy arrays) | One | For vectorized code. Pairs with the §7 NumPy views. |

   - **Proposed combination** (to decide when the collision phase is specified):
     - Rust handles movement, detection, and **declared built-in responses** (#4: bounce, stop, solid).
     - Python receives **enter/exit events** (#3), delivered in a batch after each update and never in the middle of it.
     - **Batched queries** (#2) answer direct questions ("what is the ship touching?").
     - **Layers and masks** (question 7) limit both what's checked and which pairs produce Python events.
     - Per-pair every-frame callbacks (#1) are avoided by default.
7. **Filtering.** Collision layers/masks (Unity/Godot style), so bullets don't test against other bullets.
8. **Triggers vs. solid.** For Phase 5 everything is detect-only (triggers). A physical response (pushing apart, bouncing) belongs to a later physics system.
9. **Build vs. use a crate.** `parry2d` (collision only) and `rapier2d` (full physics) are mature Rust options. Writing our own fits the "from scratch" goal and keeps it small. Using parry2d is much faster to build. Decide during research, and consider whether the crate works under WASM (it does in rapier's case).

## 12. Backlog (post-v1)

- [ ] Mouse and gamepad input (keyboard is Phase 4)
- [ ] Sprite sheets (texture regions), animation
- [ ] `View`/`Scene` system
- [ ] `Camera`
- [ ] Text rendering
- [ ] Audio library
- [ ] NumPy zero-copy views on `SpriteList` (see §7)
- [ ] Physics response (after the collision system, §11)
- [ ] Angle-to-velocity helper: set a sprite's velocity from a heading (degrees, counter-clockwise) and a speed, so users rarely need `math.radians`/`cos`/`sin` (e.g. something like `sprite.set_velocity_from_angle(angle, speed)`; the exact API is TBD)
- [ ] Shapes/primitives drawing
- [ ] Collision: colliders without a sprite (invisible triggers with their own position); not Phase 5
- [ ] Collision: declared collision pairs checked automatically every frame (e.g. `collisions.watch(bullets, asteroids)`); needed for engine-driven events and responses (§11)
- [ ] Collision shapes beyond Phase 5: convex polygon, auto hit box from texture alpha, capsule, pixel mask (§11)
- [ ] `Window.close()`: close the window from game code (e.g. quit on Escape); left out of Phase 4
- [ ] GPU selection setting: let a game opt into the low-power GPU (the high-performance default is decided, §4); add only when needed
- [ ] Benchmarks: a vsync-off mode for raw FPS, and a higher ramp cap or time-boxed search, so discrete-GPU limits can be found
- [ ] Web stress measurement: a browser variant of the `python_moved` scene (Pyodide → JS → WASM call cost)
- [ ] Build/packaging/distribution (wheels via maturin)
- [ ] Testing strategy (headless wgpu rendering tests)

## 13. Repository Layout

```
ferrocade/                 # repo root
├─ README.md               # project overview
├─ CLAUDE.md               # instructions for AI coding assistants (imports the constitution)
├─ .claude/skills/         # Spec Kit skills for Claude Code (speckit-specify, -plan, -tasks, ...)
├─ .specify/               # Spec Kit: constitution (memory/), templates, scripts
├─ .gitignore, .gitattributes
├─ specs/                  # Spec Kit feature specs, one per roadmap phase: 001-centered-ship/, ...
├─ docs/
│  ├─ spec/ENGINE_SPEC.md  # this document: living spec + decision log
│  ├─ design/              # in-depth subsystem designs (e.g. collision.md)
│  ├─ research/            # spikes, benchmark results, engine comparisons
│  └─ guides/              # user-facing guides (later)
├─ crates/
│  ├─ ferrocade/           # Rust engine core
│  └─ ferrocade-py/        # PyO3 bindings
├─ python/ferrocade/       # Python package (re-exports, helpers, .pyi stubs)
├─ examples/               # one example per phase: 01_centered_ship.py, ...
├─ assets/                 # images/ (ship.png), later sounds/, fonts/
├─ tests/                  # Python API tests (Rust tests live in the crates)
├─ benches/                # performance scenes (§7, Phase 3)
├─ web/                    # browser build: Pyodide loader, HTML template (Phase 1b)
└─ tools/                  # developer scripts
```

- Phase 1 adds a root `Cargo.toml` (Cargo workspace over `crates/*`) and `pyproject.toml` (maturin config pointing at `crates/ferrocade-py`, with `python/` as the Python source dir).
- The spec stays a single document. When a subsystem design grows large, move the details to `docs/design/` and leave a summary plus link here.

### Spec Kit workflow
- The project uses [Spec Kit](https://github.com/github/spec-kit) (`specify` CLI) for per-phase work.
- **Constitution** (`.specify/memory/constitution.md`): the non-negotiable principles drawn from this spec's decisions. `CLAUDE.md` imports it, so AI assistants always see it.
- **Feature specs** (`specs/NNN-<name>/`): one per roadmap phase (§10), each with `spec.md`, `plan.md`, and `tasks.md`. Flow: `/speckit-specify` → `/speckit-clarify` → `/speckit-plan` → `/speckit-tasks` → `/speckit-implement`.
- **Division of roles:** this document holds engine-wide architecture and the decision log. Feature specs hold phase-specific requirements and tasks, and reference this document instead of copying it.

---

## Decision Log

| Date | Decision | Notes |
|---|---|---|
| 2026-10-12 | Working name: **Ferrocade** (`import ferrocade as fc`) | Can change later |
| 2026-10-12 | Repository layout (§13); crates `ferrocade` + `ferrocade-py` | Spec moved to `docs/spec/` |
| 2026-10-12 | Adopt Spec Kit; one feature spec per roadmap phase | §13 "Spec Kit workflow" |
| 2026-10-12 | Minimum Python 3.13, stable ABI (`abi3-py313`) | Owner: no backward-compat need; don't go newer without a benefit. §6 |
| 2026-10-12 | Phase 1 implemented: native `__init__` works for subclassable abi3 pyclasses (R4 confirmed; no Python wrapper classes needed) | `specs/001-centered-ship/research.md` R4 |
| 2026-10-12 | Known limitation: `run()` uses winit `run_app_on_demand` (desktop-only); the web build needs `spawn_app` | Phase 1b |
| 2026-10-12 | Texture filtering: no default; `filter=` required on `Window`, optional override on `Texture` | Recurring issue; Phase 1 example uses `NEAREST` |
| 2026-10-12 | Phase 1 spec (`specs/001-centered-ship`) defaults confirmed | Logical pixels, no re-center on resize, callback exceptions propagate |
| 2026-10-12 | Constitution v1.0.0 ratified (draft, pending owner review) | Six principles drawn from confirmed decisions |
| 2026-10-12 | Python-only public API, Rust backend | Users never write Rust |
| 2026-10-12 | wgpu for rendering | Native standalone, multi-backend, optional web |
| 2026-10-12 | v1 classes: `Window`, `Sprite`, `SpriteList` | Arcade-style. `SpriteList` = GPU batch |
| 2026-10-12 | Public `Texture` class + path-keyed cache | Shared GPU textures; `Sprite` accepts a `Texture` or a path |
| 2026-10-12 | v1 goal: window + `ship.png` centered | Milestone 1 |
| 2026-10-12 | Bottom-left origin, y up, pixel units | Arcade/OpenGL convention |
| 2026-10-12 | Subclass `Window` and override callbacks | Arcade-style API |
| 2026-10-12 | Image formats: PNG required; JPEG/BMP/GIF enabled for free via `image` crate features | More formats later |
| 2026-10-12 | Minimize Python↔Rust number conversion (sprite data native in Rust, GPU-layout buffers, dirty tracking) | §7 strategy confirmed |
| 2026-10-12 | `Sprite` stores f64; GPU buffers are f32 | Exact Python round-trips; conversion in Rust |
| 2026-10-12 | Motion units per second (px/s, deg/s) | Frame-rate independent |
| 2026-10-12 | Fixed-timestep updates, default 60 Hz; draw per vsync frame | Catch-up capped per frame; interpolation later |
| 2026-10-12 | Names: `velocity_x`, `velocity_y`, `angular_velocity` | Avoids Arcade's per-frame `change_x` meaning |
| 2026-10-12 | 2D only for prototypes; 3D out of scope | Ortho projection, no depth buffer, draw order = layering |
| 2026-10-12 | Platforms: Windows (prototype), macOS, web (WASM) | Portability rules in §9; Linux unofficial |
| 2026-10-12 | Renderer limited to WebGL2-level features by default | Guarantees the web fallback works |
| 2026-10-12 | Web: Option B (Pyodide + separate wasm-bindgen engine module); revisit A when upstream unblocks it | Owner accepted the Phase 1b spike recommendation (`docs/research/web-spike.md`) |
| 2026-10-12 | `fc.color` = all CSS named colors, UPPER_SNAKE names, CSS values win (GREEN=(0,128,0)); default background renamed `DEFAULT_BACKGROUND` (32,32,32) | §3 "Colors" |
| 2026-10-12 | Solid-color rectangle and circle sprites without images, via per-sprite `color` tint plus shared built-in white textures (instance stays 48 bytes) | §3 "Shape sprites"; API details proposed |
| 2026-10-12 | New Phase 1c (colors + shape sprites) before Phase 2 | §10 |
| 2026-10-12 | Shape API: `fc.RectangleSprite` / `fc.CircleSprite` classes (extend `Sprite`) | Chosen over factory functions and built-in textures, because it suits subclassing |
| 2026-10-12 | Circle edges follow the window filter, with a per-circle `filter=` override | Consistent with "no hidden default filter" |
| 2026-10-12 | Add a performance showcase milestone after Phase 5: walls plus many bouncing sprites, showing render, move, and collision speed | Owner request; collision response and timing display to be decided (§10) |
| 2026-10-12 | Built-in shape atlas is mipmapped (6 levels) and the disc is kept clear of the white half | Needed so `LINEAR` circles stay smooth when drawn small; found by Phase 1c tests |
| 2026-10-12 | Angles and angular velocity stay in degrees (deg, deg/s); an angle-to-velocity helper goes in the backlog | Owner confirmed degrees over radians |
| 2026-10-12 | Rotation: a positive angle is counter-clockwise (math convention) | Chosen for mathematical consistency with y-up; differs from Arcade 3.x |
| 2026-10-12 | Measured: per-sprite Python work costs about 15× the engine's bulk update (1,000 sprites: 1.34 ms vs. 0.087 ms) | Evidence for the hybrid collision design (§11 Q6); `docs/research/phase2-validation.md` |
| 2026-10-12 | Phase 3 target: 10,000 engine-moved sprites at 60 FPS; compare with Arcade and Pygame; NumPy views deferred | Owner answers to the Phase 3 spec questions |
| 2026-10-12 | Phase 3 comparison versions: Arcade 4.0 pre-release (`4.0.0.dev8`) and pygame-ce (not classic pygame) | Owner choice; both are dev-only benchmark installs |
| 2026-10-12 | Collision direction: Rust does movement and detection, Python manages collisions (game logic) | Owner direction; delivery mechanism Proposed (§11 Q6: built-in Rust responses plus batched enter/exit events plus queries) |
| 2026-10-12 | Phase 3 implemented: target met (100,000 engine-moved sprites at 60 FPS on the integrated GPU; Arcade 4.0.0.dev8 8,000; pygame-ce 16,500); no optimization needed | `docs/research/phase3-benchmarks.md` |
| 2026-10-12 | Phase 3 benchmark method: sustainable = FPS ≥ 59 and per-frame work < 14 ms; each engine/scene in its own process; Arcade paced with vsync and 1/240 rates, Pygame with `Clock.tick(60)` and no spin | Deviations from the plan, needed for fair Windows pacing; see the fairness notes in the report |
| 2026-10-12 | Prefer the high-performance (discrete) GPU by default (wgpu `PowerPreference::HighPerformance`) | Owner chose option A after the Phase 3 finding; §4 |
| 2026-10-12 | The collision architecture (§11) is decided before Phase 5 or as part of Phase 5 | Owner direction; the design itself stays open until then |
| 2026-10-12 | Phase 4 key names: `KEY_0`–`KEY_9`, `NUM_*`, `PAGE_UP`/`PAGE_DOWN`, `BRACKET_LEFT`/`BRACKET_RIGHT`, `LSYSTEM`/`RSYSTEM` and `MOD_SYSTEM` | Owner confirmed at Phase 4 planning; §5 |
| 2026-10-12 | Key modifiers report the keys held after the event; no `Window.close()` / Escape-to-quit in Phase 4 | Owner confirmed at Phase 4 planning; `Window.close()` added to the backlog |
| 2026-10-12 | Keyboard behavior: physical keys, auto-repeat ignored, focus loss releases held keys, events delivered once per frame before `on_update` | Phase 4 spec defaults; the owner reviewed the spec and plan without changes; §5 |
| 2026-10-12 | Collision architecture: separate collider entities; circle, axis-aligned box, rotated box first (expandable); list-based queries; batched query results first; spatial hash grid per list; written from scratch | Owner decisions on §11 questions 1–6; `docs/design/collision.md`. Declared pairs → backlog; enter/exit events and built-in responses → showcase ideas |
| 2026-10-12 | Colliders are attached explicitly (no automatic colliders); colliders without a sprite go to the backlog | Owner answers to the collision open questions 1–2; `docs/design/collision.md` |
| 2026-10-12 | Collider scale is coupled to `sprite.scale`; two box classes: `BoxCollider` (rotates) and `AABBCollider` (never rotates; name chosen by the owner) | Owner decisions; matches Unity/Godot/Arcade scaling; `docs/design/collision.md` |
| 2026-10-12 | Collider offsets rotate with the sprite (`CircleCollider`, `BoxCollider`); `AABBCollider` offsets don't rotate | Owner accepted the proposal; `docs/design/collision.md` |
| 2026-10-12 | Phased roadmap: 1 centered ship → 2 bouncing → 3 stress test → 4 keyboard → 5 collisions | §10 |
| 2026-10-12 | Collision detection implemented in Rust | Design open (§11) |
| 2026-10-12 | winit for windowing | Confirmed |
| 2026-10-12 | PyO3 + maturin for bindings | Confirmed |
