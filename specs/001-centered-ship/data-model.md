# Data Model: Phase 1 — Window and Centered Ship

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

This document describes the entities, where their data lives, how they relate, the rules
that keep them valid, and their states. The Python-facing API is in
[contracts/python-api.md](contracts/python-api.md).

## Overview

```
Python handles (thin)                  Rust core (crates/ferrocade)
─────────────────────                  ─────────────────────────────────────────
fc.Window ─────────────────────────▶ WindowConfig + WindowState (size, scale)
                                       └─ runs ─▶ App (winit) ─▶ Renderer (wgpu)
fc.Texture ── Arc ─────────────────▶ TextureData  ◀── TextureCache (path → Arc)
fc.Sprite ─── SpriteId ────────────▶ SpriteStore.sprites[SpriteId] : SpriteState
fc.SpriteList ── ListId ───────────▶ SpriteStore.lists[ListId]     : ListState
                                       Renderer.gpu_textures[TextureId] (lazy)
                                       Renderer.gpu_lists[ListId]      (lazy)
                                       FrameQueue (thread-local, during on_draw)
```

## Entities

### WindowConfig / WindowState (core) and `Window` (Python)

| Field | Type | Rules |
|---|---|---|
| `width`, `height` | logical px (`u32` stored, reported as `int`) | Must be > 0 at construction. Updated on resize. Reads 0 while minimized is **not** reported: the last non-zero size is kept. |
| `title` | `str` | Any string. |
| `update_rate` | seconds (`f64`) | Must be > 0. Default `1/60`. |
| `filter` | `Filter` (`Nearest` \| `Linear`) | **Required, no default** (R11). The filter used for textures without an override. |
| `scale_factor` | `f64` | Internal (HiDPI). Not exposed in Phase 1. |
| `running` | `bool` | Internal. True between `run()` start and loop exit. |

**States**:

```
Configured ──run()──▶ Running ──close button / exception / Ctrl+C──▶ Closed
    ▲                                                                   │
    └──────────────────── run() again (allowed) ◀───────────────────────┘
```

- **Configured:** set by `__init__`. Size and title are readable, and no OS window exists yet.
- **Running:** the OS window exists and callbacks fire. `clear()` and `SpriteList.draw()` are only valid inside `on_draw`.
- **Closed:** the OS window is destroyed and `run()` has returned (or raised the callback's exception). GPU resources are dropped with the renderer, while CPU textures, sprites, and lists remain valid.

### TextureData (core) and `Texture` (Python)

| Field | Type | Rules |
|---|---|---|
| `id` | `TextureId` (u64, unique) | Assigned on first load. |
| `path` | canonical absolute path (`str` in Python) | Cache key. |
| `width`, `height` | `u32` px | Taken from the decoded image. Both > 0. |
| `pixels` | `Arc<[u8]>` RGBA8, straight alpha | CPU copy. Uploaded to the GPU lazily by each renderer. |

**The Python `Texture` handle** = (`Arc<TextureData>`, `filter_override: Option<Filter>`). The filter belongs to the handle (sampler choice), not to the pixel data.

**Rules**:
- **One `TextureData` per canonical path (FR-008, FR-023, SC-008).** Loading the same path again returns the same `Arc`, whatever the filter override. Python `Texture` objects compare equal (`==`) and hash equal when the path *and* the filter override match.
- **Effective filter** = `filter_override`, otherwise the running window's `filter`. It's resolved by the renderer at draw time, so textures can be created before the window.
- **Load failures** raise `FileNotFoundError` (missing) or `ValueError` (unsupported/corrupt). The message includes the path and the reason (FR-010).
- **Immutable after load.**
- The cache holds strong references in Phase 1 (no unloading).

### SpriteState (core) and `Sprite` (Python)

| Field | Type | Rules |
|---|---|---|
| `center_x`, `center_y` | `f64` px | Any finite value. Non-finite values (NaN/inf) raise `ValueError`. |
| `scale` | `f64` | Any finite value. 0 is allowed (invisible), and negative values are allowed (they mirror the sprite once rendering supports it; Phase 1 renders by absolute size). |
| `angle` | `f64` degrees | Stored. Not rendered in Phase 1. |
| `texture` | `Arc<TextureData>` + `Option<Filter>` (the handle) | Required. Can be replaced. |
| `memberships` | `SmallVec<(ListId, slot u32)>` | Internal. One entry per list containing this sprite. |
| `width`, `height` (derived) | `f64` px | `texture.width * |scale|`, `texture.height * |scale|`. Read-only. |
| `position` (derived) | `(f64, f64)` | Combined getter/setter for `(center_x, center_y)`. One crossing. |

**Rules**:
- **Every setter** updates the f64 field, then recomputes the f32 `Instance` and writes it into each membership's list at `slot`, widening that list's dirty range (constitution II).
- **Setting `texture`** also updates each list's per-slot texture (which can change batching).
- **Lifetime:** when the Python `Sprite` (and its subclass instance) is garbage-collected, the sprite is removed from the store. Lists hold the sprite's Python object while it's a member, so a sprite that is in a list stays alive.

### ListState (core) and `SpriteList` (Python)

| Field | Type | Rules |
|---|---|---|
| `sprites` | `Vec<SpriteId>` | Order = draw order (later = on top). |
| `py_refs` | (bindings) `Vec<Py<Sprite>>` | Keeps member sprites alive. Kept in the same order. |
| `instances` | `Vec<Instance>` (f32, GPU layout) | `instances[i]` mirrors `sprites[i]`. |
| `slot_textures` | `Vec<(TextureId, Option<Filter>)>` | Used for batching runs. Consecutive slots with the same texture and the same *effective* filter form one draw call. |
| `dirty` | `Option<Range<usize>>` | Instance range not yet uploaded. Full range after a structural change. |
| `structure_version` | `u64` | Bumped on append/remove, so the renderer knows when to rebuild batches. |

**Rules**:
- **`append(sprite)`** pushes to the end, adds the membership `(list, index)` to the sprite, and marks the new slot dirty. Appending the same sprite twice to the same list raises `ValueError`.
- **`remove(sprite)`** removes it and shifts later slots down by one (updating those sprites' membership slots), and marks the shifted range dirty. Removing a sprite that isn't in the list raises `ValueError`. O(n) is acceptable for Phase 1.
- **`len(list)`** gives the count, and `sprite in list` gives membership. Iteration yields the sprites in order (a convenience for Python code).
- **`draw()`** is valid only during `on_draw`, and enqueues `DrawList(id)`. An empty list draws nothing.
- **Dropping a list** removes its memberships from all of its sprites.

### Instance (core, GPU layout)

```
#[repr(C)] Pod
center:  [f32; 2]   // px
size:    [f32; 2]   // px (texture size × |scale|)
angle:   f32        // degrees (unused by the Phase 1 shader)
_pad0:   [f32; 3]
uv_rect: [f32; 4]   // (u0, v0, u1, v1); full texture = (0,0,1,1)
```

48 bytes in total (16-byte aligned for portability).

### FrameQueue (core, thread-local)

| Field | Type | Rules |
|---|---|---|
| `active` | `bool` | True only while `on_draw` runs. |
| `commands` | `Vec<FrameCommand>` | `Clear([f32;4])` or `DrawList(ListId)`, in call order. |

**Rules**:
- Enqueuing while inactive raises `RuntimeError("... can only be called inside on_draw()")`.
- **After `on_draw`:**
  - The renderer begins the frame with the first `Clear` color, or the default color if there's no `Clear`.
  - Later `Clear` commands clear again at that point in the frame.
  - Each `DrawList` syncs the list's dirty instances and draws its batches.

### FixedTimestep (core)

| Field | Type | Rules |
|---|---|---|
| `step` | `f64` s | Equal to `update_rate`. |
| `accumulator` | `f64` s | Grows with real elapsed time. |
| `max_steps_per_frame` | `u32` | 5. Excess accumulated time is discarded. |

`advance(elapsed) -> u32` returns how many `on_update(step)` calls to run before drawing.

## Relationships

- **Texture to Sprite:** one texture is used by many sprites.
- **Sprite to SpriteList:** many to many, through memberships with slot indices.
- **Window to SpriteList:** none directly. Lists are drawn from `on_draw` via the frame queue.
- **Renderer:** owns the GPU mirrors (`gpu_textures`, `gpu_lists`) keyed by ID. They're created lazily and dropped when the window closes.
  - It also owns two samplers (nearest, linear) and bind groups keyed by `(TextureId, Filter)`.
