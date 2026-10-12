# Data Model: Phase 1c — CSS Colors and Shape Sprites

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

This document covers the changes relative to [Phase 1's data model](../001-centered-ship/data-model.md).

## Named colors (`python/ferrocade/color.py`)

| Item | Rule |
|---|---|
| CSS colors | Exactly **148** names: the UPPER_SNAKE forms of the CSS Color Level 4 keywords. Each value is an RGB 3-tuple of ints 0-255 equal to the CSS value. |
| Spelling pairs | Every CSS `gray`/`grey` pair exists under both names with identical values (e.g. `DARK_GRAY` and `DARK_GREY` are both (169,169,169)). |
| Extras (non-CSS) | `TRANSPARENT = (0, 0, 0, 0)` and `DEFAULT_BACKGROUND = (32, 32, 32)`. |
| Removed or changed | `GREEN` is now (0,128,0) (it was (0,255,0); use `LIME`). `DARK_GRAY` is now (169,169,169) (it was (32,32,32); use `DEFAULT_BACKGROUND`). |
| Sharing | The same file is used on desktop and web. |

## SpriteState (core) changes

| Field | Type | Rules |
|---|---|---|
| `color` | `[u8; 4]` RGBA (sRGB) | Default `[255,255,255,255]`. Setting it validates 3- or 4-component input (0-255; alpha defaults to 255), then rewrites the instance in every list and marks those slots dirty. |
| `base_size` | `Option<(f64, f64)>` | `None` for image sprites, meaning the texture's size is used. `Some((w, h))` for shapes: **w, h finite and > 0**, otherwise `ValueError`. |
| `uv_rect` | `[f32; 4]` | Defaults to the whole texture `(0,0,1,1)`. Shapes use their atlas region (R3). |

**Derived values:** the drawn size and the `width`/`height` properties are
`base_size.unwrap_or(texture size) × |scale|`.

## Instance (GPU layout) changes

```
#[repr(C)] Pod, 48 bytes (unchanged size)
offset  0  center:  [f32; 2]
offset  8  size:    [f32; 2]
offset 16  angle:   f32
offset 20  color:   [u8; 4]    // Unorm8x4, sRGB; the shader converts RGB to linear and multiplies
offset 24  _pad:    [f32; 2]
offset 32  uv_rect: [f32; 4]
```

## Built-in shape texture

| Item | Rule |
|---|---|
| Key | `<builtin:shapes>`: reserved, never a filesystem path. Generated once per process. |
| Pixels | 1024×512 RGBA. Left half: opaque white. Right half: a white disc with an anti-aliased alpha edge on a transparent background. |
| Regions (`uv_rect`) | Rectangle: `(0.125, 0.25, 0.375, 0.75)` (inset, so it never samples edge texels). Circle: `(0.5, 0.0, 1.0, 1.0)`. |
| Filter | Rectangles: the window filter (it makes no visual difference). Circles: the window filter, or their own `filter=` override. |

## Python types

| Type | Extends | Constructor | Extra properties |
|---|---|---|---|
| `Sprite` | — | unchanged | **`color`** (read/write; returns a 4-tuple) |
| `RectangleSprite` | `Sprite` | `(width, height, color, center_x=0.0, center_y=0.0)` | — |
| `CircleSprite` | `Sprite` | `(radius, color, center_x=0.0, center_y=0.0, *, filter=None)` | `radius` (read-only: base radius × \|scale\|) |

**Rules:**
- **Shape `texture`:** reading `texture` on a shape returns a `Texture` whose `path` is
  `"<builtin:shapes>"`.
- **Shape `texture` assignment:** assigning a texture to a shape **raises `TypeError`**
  ("shape sprites use a built-in texture"), to avoid an ambiguous base size and region.
- **Lists and positions:** all existing list and position semantics are unchanged.

## Renderer stats (testing aid)

`FrameStats { draw_calls: u32, instances: u32 }` for the most recent frame. It's read-only, and
used by tests to verify batching (SC-004).
