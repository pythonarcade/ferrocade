# Phase 1c Validation Record: CSS Colors and Shape Sprites

**Feature**: [specs/003-colors-shape-sprites](../../specs/003-colors-shape-sprites/spec.md) | **Guide**: [quickstart.md](../../specs/003-colors-shape-sprites/quickstart.md)
**Date**: 2026-10-12. **Machine**: Windows 11 Pro, NVIDIA RTX 3500 Ada Laptop GPU, rustc 1.99.0, Python 3.14.4 (venv), Chrome/Edge (headless), Pyodide 314.0.7.

| # | Scenario | Result | Evidence |
|---|---|---|---|
| Q1 | Rust tests | ✅ | 37 passed. Offscreen GPU tests: a 50×20 red rectangle is exactly 50×20, centered, with interior pixels exactly (255,0,0). A radius-15 circle is 30×30 with its center pixel exactly (0,0,255) and background corners (round). Alpha 128 blends correctly. **1,000 interleaved rectangles and circles = 1 draw call, 1,000 instances.** A `LINEAR` circle has more anti-aliased edge pixels than a `NEAREST` one at radius 20, 100, and 280. The red tint zeroes G/B and keeps transparent pixels; alpha 128 changes the pixels; resetting to white is pixel-identical to the original. All Phase 1 pixel tests are unchanged. |
| Q2 | Python tests | ✅ | 71 passed: the color set (148, GRAY/GREY pairs, spec values), `Sprite.color` (default, round trip, validation), shape constructors and validation, the `filter=` override, `TypeError` on assigning a texture to a shape, `Wall(RectangleSprite)` / `Ball(CircleSprite)` subclasses, mixing in lists, and drawing inside `on_draw`. |
| Q3 | CSS values against Chrome | ✅ | `check_colors.mjs`: **all 148 CSS colors match Chrome's computed values**, and each is a valid CSS keyword. |
| Q4 | Desktop example | ✅ (owner to view) | `python examples/01c_shapes.py` runs at a steady **60 FPS (16.67 ms, vsync)**. The window can't be screenshotted from the agent shell, but the same scene was verified in the browser (Q6) and by the pixel tests. |
| Q5 | Phase 1 unchanged | ✅ | The Phase 1 pixel tests are unchanged and passing. In the browser, `01_centered_ship` still gives bbox 350,262,448,336 (99×75, −0.5 px) and 1 draw call. |
| Q6 | Browser parity | ✅ | `?example=01c_shapes` runs **unchanged** in Chrome on WebGPU and on WebGL2. The screenshot shows the walls, the bar, a stepped `NEAREST` circle beside a smooth `LINEAR` circle, a semi-transparent circle, and the pink-tinted ship. **4 draw calls**, matching the design: rectangles + circle, the `LINEAR` circle, the alpha circle, and the ship (another texture). |
| Q7 | 1,000-shape scene | ✅ | `--many`: 1,000 random shapes at **16.6 ms/frame (60 FPS, vsync-limited)** on desktop. |

## Lint

`cargo fmt --check`, plus `cargo clippy -D warnings` for the workspace (all targets), the no-render core, and the wasm32 web crate: all clean. `_ferrocade.pyi` matches the runtime exactly.

## Findings during implementation

- **Mipmapped shape atlas (a deviation from plan R3).** The circle-smoothness test showed that a `LINEAR` circle drawn *smaller* than the atlas disc came out as jagged as `NEAREST`, because there were no mipmaps. **Fix:** the built-in atlas is uploaded with 6 mip levels (2×2 box filter), and the linear sampler uses linear mipmap filtering. To prevent bleeding between the halves at small mip levels, the disc was shrunk to radius 200 with a gap from the white half (`CIRCLE_UV` updated). Image textures are unaffected (1 mip level).
- **Test pitfall, seen twice:** renderer caches are keyed by `ListId`, so tests that use *several* `SpriteStore`s with one renderer can collide. The engine only has one global store, so this isn't a product bug. Tests now share one store per renderer.
