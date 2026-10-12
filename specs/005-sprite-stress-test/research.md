# Research: Phase 3 — Sprite Stress Test (Performance)

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-12

---

## R1. Measuring headroom despite vsync (FR-004)

**Problem**: the engine presents with `Fifo` (vsync), so frame time is pinned at about 16.7 ms
until the work exceeds it. That hides how much headroom is left.

**Decision**: measure **work time**, not just frame time.
- **Engine side:** `render_frame` records its own CPU duration (queue sync, uploads, pass recording,
  submit) **excluding** the surface-acquire wait. This is added to `FrameStats` as `render_cpu_ms`,
  alongside `draw_calls` and `instances`.
- **Python side:** the stress tool times its own `on_update` (split into the engine `update` call
  and game code) and `on_draw` (queueing) with `time.perf_counter()`.
- **Sustainable means both** of these hold over a 3 s window (after a 1 s warm-up):
  - the measured FPS is ≥ 59
  - the per-frame work total (update + game code + draw queueing + render CPU) is < 14 ms, leaving
    ~2.7 ms of the 16.7 ms budget for GPU and OS jitter

**Rationale**: the CPU work total shows headroom above vsync, and the FPS condition catches GPU-bound
limits that CPU timing can't see. **No vsync-off option is added**, so there's no new user-facing
API for a measurement concern.

**Alternatives considered**:
- A `Window(vsync=False)` option (`PresentMode::AutoNoVsync`): gives raw maximum FPS, but adds an API
  surface only benchmarking needs. Deferred; it could become a backlog item if the owner wants it.
- GPU timestamp queries: not in the WebGL2 baseline, and too much machinery for this phase.

---

## R2. Keeping the engine-moved scene free of per-sprite Python work

**Problem**: in Phase 2, bouncing was Python code. Running that per sprite in scene (b) would
measure Python, not the engine (it's 15× the engine cost per sprite, as Phase 2 measured).

**Decision**: there's **no engine-side bounce in this phase**. That's collision and response
territory (§11, and the showcase), which constitution V says not to decide during implementation.
- Scene (b) instead uses **slow drift**: sprites start spread across the window with small random
  velocities (≤ 20 px/s) and random spins, so they stay on screen for a measurement window.
- Between ramp steps, the tool **re-scatters** the sprites (a Python operation once per step, not
  per frame).
- The engine still moves and re-uploads **every sprite every frame**: every velocity is non-zero,
  so every slot is dirty. That's the worst case for uploads, which is exactly what the target is about.

**Rationale**: it measures the engine's move-plus-upload-plus-draw path honestly, without
pre-empting the collision API design.

**Alternative considered**: a minimal `SpriteList.update(dt, bounds=…)` bounce. Rejected *for now*.
The showcase will need a real, designed response mechanism, which is why this is flagged.

---

## R3. Ramp and search strategy (FR-003, SC-004)

**Decision**: per scene, run an **exponential ramp** (1k, 2k, 4k, …) until a step fails, then a
**binary search** between the last pass and the first failure, down to a 5% resolution.
- Each step: 1 s warm-up, then a 3 s measurement.
- Typically 8–12 steps per scene, at about 4–5 minutes for all four scenes. That fits the
  10-minute SC-004 budget.
- The sprite count changes by adding or removing sprites in the existing list. The window isn't
  recreated, so GPU buffer growth is part of the measurement.

---

## R4. Frame-rate readout (FR-008)

**Decision**:
- `Window.show_fps` (bool property, default `False`). When it's on, the engine appends
  `" — 60 FPS, 4.12 ms"` to the window title about once per second, and restores the plain title
  when it's turned off.
- This needs an engine-side title setter (winit `set_title`) on desktop and `document.title` in the
  web shim.
- `Window.frame_stats` (read-only) returns a small stats object for tools:
  `fps`, `frame_ms`, `render_cpu_ms`, `draw_calls`, `instances`.

**Rationale**: one line to enable it (SC-005). When it's off, the cost is a single flag check per
frame. The stats object lets the stress tool and the future showcase read the engine's own numbers.

---

## R5. Comparison engines (US2, owner decision)

**Decision**:
- **Python Arcade 3.3.3** (GPU-batched `SpriteList`; it has a Python 3.14 wheel).
- **Pygame:** classic `pygame` 2.6.1 has **no Python 3.14 Windows wheel**, so use **`pygame-ce`
  2.5.8** (the community edition: the same `import pygame` API, with an actively maintained 3.14 wheel).
- **Installation:** both go into the **dev venv only**, via a `benches/requirements.txt`. They're
  never Ferrocade dependencies.
- **Equivalent scenes:**

  | Scene | Arcade | Pygame |
  |---|---|---|
  | Static | `SpriteList.draw()` | `Group.draw(screen)` |
  | Engine-moved (drift) | `SpriteList.update()` with `change_x`/`change_y` scaled per frame (Arcade moves per frame) | `Group.update()` with sprite `update` moving rects (CPU) |
  | Python-moved | A Python loop setting `center_x`/`center_y` | A Python loop setting `rect.center` |
  | Shapes | `SpriteSolidColor`/`SpriteCircle` | Surfaces filled once, blitted |

- **The same image** (ship scaled 0.25), the same 800×600 window, and the same ramp and
  sustainability rule. Each engine's own timing is measured with `perf_counter` around update and draw.

**Fairness notes**, recorded in the report:
- Arcade's "engine-moved" is still a Python-level loop internally.
- Pygame is CPU-blitting by design.
- Both run with vsync off or on according to their defaults. Where uncapped, they're measured with
  the same "≥ 59 FPS and work < 14 ms" rule.

---

## R6. Likely bottlenecks to watch (FR-010)

These are hypotheses to confirm or refute:
1. **The per-frame full-buffer upload** at 10k+ moving sprites: 48 bytes × N, which is 480 KB at
   10k. Cheap.
2. **`update_list` cloning the member list** (`list.sprites.clone()`) every update: an allocation
   per frame. Easy to fix if it shows up.
3. **The per-sprite instance rewrite**, which loops over memberships. It's O(N) with small constants.
4. **The Python-moved scene:** dominated by property-call overhead (Phase 2: about 1.3 µs per sprite
   per check). Expect roughly a 10k-sprite ceiling at about 4 property writes per sprite.

The tool's breakdown (R1) attributes time to each stage, so fixes target measured costs.
