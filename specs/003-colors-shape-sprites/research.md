# Research: Phase 1c — CSS Colors and Shape Sprites

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12

---

## R1. Source of truth for CSS colors, and how it's verified

**Decision**:
- `python/ferrocade/color.py` holds the **148 CSS Color Level 4 named colors** as UPPER_SNAKE
  constants, plus `TRANSPARENT` and `DEFAULT_BACKGROUND`. It's generated once by
  `tools/gen_css_colors.py` from an embedded name→hex table, and checked in. The file is plain
  constants, so it's readable and greppable.
- **Name rule:** split the CSS keyword at its English word boundaries and join with underscores
  (`lightgoldenrodyellow` → `LIGHT_GOLDENROD_YELLOW`). An explicit name map in the generator
  avoids guessing the boundaries.
- **Authoritative verification:** a check script loads a page in headless Chrome and asks the
  browser's own CSS engine for each of the 148 keywords (`getComputedStyle`). It compares the
  answers with `fc.color` and also confirms the count (148) and that no CSS name is missing. Running
  it is part of validation (Q3). A pytest test checks the internal consistency: count, value
  ranges, the GRAY/GREY pairs, and the specific spec values.
- **One file for desktop and web (FR-005):** the web loader fetches `python/ferrocade/color.py`
  directly, and the copy in `web/python/ferrocade/color.py` is deleted. That resolves one of the
  web spike's "required before shipping" items.

**Rationale**: generating the file avoids hand-typing 148 hex values, and checking against a real
CSS engine is far more trustworthy than a second hand-typed table.

**Alternatives considered**:
- Building the colors at import time from a JSON file. Rejected because it's less readable, and
  editors can't see the constants.
- Hand-typing the list. Rejected as error-prone.

---

## R2. Per-sprite tint in the GPU instance (and keeping it 48 bytes)

**Decision**:
- **New layout:** `Instance { center: [f32;2], size: [f32;2], angle: f32, color: [u8;4], _pad: [f32;2], uv_rect: [f32;4] }`,
  still 48 bytes. `color` sits at byte offset 20, in what used to be padding.
- **Vertex format:** `Unorm8x4` (normalized in hardware). It's WebGL2-compatible.
- **Color space:** tint values are **sRGB 0-255**, as users write them. The vertex shader converts
  RGB to linear with the exact sRGB transfer function before multiplying (alpha stays linear).
  - Textures are sampled as sRGB, so they already arrive linear, and output goes through an sRGB view.
  - So `white texture × tint` writes back **exactly** the user's 0-255 color (SC-003). A white tint
    is exactly ×1.0, so image sprites stay pixel-identical (SC-005).
- **Default tint:** white (255,255,255,255).
- **Dirty tracking:** setting it marks just that slot dirty, like any other property (FR-008).

**Rationale**: no growth in instance size (constitution II), and color-exact results.

**Alternatives considered**:
- f32x4 linear color: it needs 16 bytes and would grow the instance to 64 bytes.
- Converting to linear on the CPU and storing as Unorm8: that loses precision in dark colors and
  breaks exact round trips.

---

## R3. Built-in shape texture: one atlas for rectangles and circles

**Decision**:
- **One built-in 1024×512 RGBA texture** holds both shapes: the left 512×512 half is solid white,
  and the right half is a white disc with an anti-aliased edge (alpha falls off over ~1.5 texels at
  the edge). It's generated in Rust at first use and cached like any texture, under the reserved key
  `<builtin:shapes>` (not a filesystem path).
- **Selecting the shape:** `uv_rect` picks it.
  - **Rectangles** use an *inset* region in the middle of the white half (u 0.125–0.375,
    v 0.25–0.75), so linear filtering can never bleed in edge texels.
  - **Circles** use the right half exactly (u 0.5–1.0, v 0–1).
  - `uv_rect` is already in the instance and shader, so no shader change is needed for selection.
- **Batching:** rectangles and circles share **one texture**, so any mix of shapes in a list (any
  sizes, any colors) forms **a single draw call**, as long as they share the effective filter. This
  meets SC-004 even when rectangles and circles are interleaved.
- **Circle filter (FR-012):** circles use the window filter unless created with `filter=`, in which
  case `slot_textures` carries that override. A circle with a different effective filter starts a
  new batch, which is correct and expected.
  - With `NEAREST`, circles drawn up to 512 px across look crisp and stepped, and larger ones show
    texel steps, which suits pixel art. With `LINEAR`, they're smooth at any reasonable size.

**Implementation update (2026-10-12):** the atlas is **mipmapped** (6 levels), and the disc is radius 200 centered at (768,256), leaving a gap from the white half; `CIRCLE_UV` covers just the disc. Without mips, `LINEAR` circles drawn smaller than the disc came out jagged. See `docs/research/phase1c-validation.md`.

**Rationale**: maximum batching with no new pipeline, and the existing texture, cache, and
bind-group machinery is reused.

**Alternatives considered**:
- Separate rectangle and circle textures: interleaved shapes would break batches.
- A signed-distance-field circle in the shader: perfect edges at any size, but it needs a second
  shader path or an extra per-instance flag. Deferred until a need appears.

---

## R4. Sprite base size (shapes have their own size)

**Decision**: add `base_size: Option<(f64, f64)>` to `SpriteState`.
- The drawn size is `base_size.unwrap_or(texture size) × |scale|`, used by both the instance and the
  `width`/`height` properties.
- Rectangles get `(width, height)` and circles get `(2r, 2r)`. Image sprites keep `None`.
- Validation: the width, height, and radius must be finite and > 0, otherwise `ValueError` (spec
  edge case).

**Rationale**: the minimal change, and existing image behavior is untouched.

---

## R5. Python classes: `RectangleSprite` and `CircleSprite` extend `Sprite`

**Decision**:
- PyO3: `#[pyclass(extends = PySprite, subclass)]` for both.
- `#[new]` accepts `(*args, **kwargs)` and returns `PyClassInitializer::from(PySprite::empty()).add_subclass(...)`.
- The real setup happens in `__init__` (the same pattern R4 proved in Phase 1):
  - `RectangleSprite.__init__(self, width, height, color, center_x=0.0, center_y=0.0)`
  - `CircleSprite.__init__(self, radius, color, center_x=0.0, center_y=0.0, *, filter=None)`
  - Both create the store sprite with the built-in texture, the uv region, the base size, and the tint.
  - User subclasses (`class Wall(fc.RectangleSprite)`) call `super().__init__(...)` as usual.
- **Read-only extras:** `CircleSprite.radius` (= base radius × |scale|). `width`/`height` are
  inherited from `Sprite`.
- **Inherited `color`:** shape sprites take it as a required constructor argument. Image sprites
  default to white.
- **Outcome (T001, 2026-10-12): confirmed.** `#[pyclass(extends = PySprite, subclass)]` with `#[new]` taking `(*args, **kwargs)` and `__init__(slf: PyRefMut<Self>, ...)`, which calls `slf.into_super().set_id(..)`, works under abi3-py313. A Python subclass of it is an `fc.Sprite` and works in `SpriteList`. The fallback isn't needed.
- **Risk:** `extends=` combined with `subclass`, the `__init__` pattern, and abi3 must work
  together. **Verify first** (task 1), as R4 was in Phase 1. Fallback: Python subclasses in
  `python/ferrocade/` calling a native `_init_shape(...)`.

---

## R6. Web parity (FR-014)

**Decision**:
- **Engine API** (`crates/ferrocade-web`) gains `create_shape_sprite(kind: "rectangle" | "circle",
  width, height, filter, center_x, center_y, r, g, b, a)`, `set_color(sprite, r, g, b, a)`, and
  `get_color(sprite) -> [r,g,b,a]`. `get_sprite` stays as it is.
- **The web shim** adds a `Sprite.color` property and `RectangleSprite`/`CircleSprite` classes
  (Python subclasses of the shim's `Sprite`) with the same signatures.
- **The loader** fetches `../python/ferrocade/color.py` (the single source).
- **Verification:** the headless check renders the Phase 1c example in the browser, and a
  screenshot comparison against desktop confirms the same shapes and colors (SC-006).

---

## R7. Default background constant rename

**Decision**:
- **Rust:** rename `color::DARK_GRAY` to `color::DEFAULT_BACKGROUND` (same value), and update its
  uses (renderer default clear, `py_window` default, tests).
- **Python:** `fc.color.DEFAULT_BACKGROUND = (32, 32, 32)`, and `DARK_GRAY` becomes CSS's
  (169, 169, 169).
- `Window.clear()` with no argument still clears to (32, 32, 32) (FR-003, SC-005).

---

## R8. Measuring draw calls (SC-004)

**Decision**:
- The renderer counts draw calls per frame: `Renderer::last_frame_stats() -> FrameStats { draw_calls, instances }`.
- The Rust offscreen test asserts that **1,000 interleaved rectangles and circles (window filter)
  produce exactly 1 draw call**.
- Desktop frame rate is checked manually or in the example (the 1,000-shape scene with a frame-time readout in the title bar).

**Rationale**: an objective, automated check for the batching claim.
