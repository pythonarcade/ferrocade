# Phase 3 Benchmarks: Sprite Stress Test

**Feature**: [specs/005-sprite-stress-test](../../specs/005-sprite-stress-test/spec.md) | **Tool**: [benches/](../../benches/README.md)
**Date**: 2026-10-12

## Setup

| | |
|---|---|
| **Machine** | Windows 11 Pro 10.0.26200, Intel Core (family 6 model 186), **two GPUs: NVIDIA RTX 3500 Ada Laptop + Intel Iris Xe (integrated)** |
| **Versions** | Ferrocade 0.1.0 (commit `5baae24`, release build), Arcade **4.0.0.dev8** (pyglet 3.0.dev11), **pygame-ce 2.5.8**, Python 3.14.4 |
| **Window, sprite** | 800×600; `ship.png` (99×75) at scale 0.25 (about 25×19 px); shapes 4–12 px |
| **Rule** | Sustainable = FPS ≥ 59 **and** work < 14 ms/frame (update + game code + draw queueing + engine render CPU), over 3 s after a 1 s warm-up. Doubling ramp, then binary search to 5%. Each engine/scene runs in its own process. |
| **Full run** | 9.2 min (the SC-004 budget is 10 min) |

## Results: maximum sprites at 60 FPS

| Scene | **Ferrocade** (integrated GPU, default) | **Ferrocade** (NVIDIA, experiment) | Arcade 4.0.0.dev8 | pygame-ce 2.5.8 |
|---|---|---|---|---|
| static | 112,000 · GPU | **≥ 512,000** (ramp cap) | ≥ 512,000 (ramp cap) | 30,000 · drawing |
| **engine_moved** (target 10,000) | **100,000** · GPU | **≥ 512,000** · ~12.8 ms CPU | 8,000 · update | 16,500 · drawing |
| python_moved | 48,000 · game code | (same: CPU-bound) | 16,000 · game code | 18,500 · drawing |
| shapes | ≥ 512,000 (ramp cap) | not run | 8,000 · update | 20,000 · update |

The text after each count is the dominant cost at the limit.

**Target verdict: met.** 100,000 engine-moved sprites at 60 FPS on the *integrated* GPU (10×
the 10,000 target), and over 512,000 on the discrete GPU (more than 51×). No optimization was needed (T012).

## Where the time goes

At each engine's limit, ms per frame:

| Engine / scene | Update | Game code | Draw queueing | Render CPU | Work | FPS |
|---|---|---|---|---|---|---|
| Ferrocade engine_moved @100k (iGPU) | 2.75 | 0 | 0.01 | 1.12 | 3.89 | 60.3 |
| Ferrocade engine_moved @512k (dGPU) | 8.46 | 0 | — | 4.37 | 12.83 | 60.0 |
| Ferrocade python_moved @48k | 0 | 12.30 | 0.01 | 0.41 | 12.72 | 60.3 |
| Ferrocade shapes @512k (iGPU) | 9.26 | 0 | 0.01 | 2.39 | 11.66 | 60.0 |
| Arcade engine_moved @8k | 10.10 | 0 | 0.49 | — | 10.60 | 91.0* |
| Arcade python_moved @16k | 0 | 10.76 | 0.55 | — | 11.32 | 85.2* |
| pygame-ce engine_moved @16.5k | 6.05 | 0 | 7.68 | — | 13.73 | 60.4 |

\* Arcade ran above 60 FPS: its `vsync=True` request didn't take effect under pyglet 3.0.dev11, so
it was paced by its 1/240 update and draw rates. The work-time rule still applies.

**Key numbers:**
- **The engine moves a sprite in about 17 ns** (release build): 100k in 1.7–2.8 ms, 512k in 8.5 ms.
  Render CPU (uploading every moved instance plus submitting) adds about 1 ms per 100k.
- **Arcade's `SpriteList.update` costs about 1.3 µs per sprite** (a Python-level loop), about **75×**
  Ferrocade's engine update. That's the core of the 8k vs. 100k+ gap.
- **Python-loop movement** costs about 0.25 µs per sprite in Ferrocade vs. about 0.67 µs in Arcade:
  setting one Ferrocade `position` tuple (one boundary crossing) is cheaper than Arcade's Python
  property chain.
- **Static sprites cost about 0 CPU in Ferrocade** (0.37 ms render CPU at any count). Nothing is
  re-uploaded when nothing changes (constitution II, confirmed).

## Finding: Ferrocade defaulted to the integrated GPU

wgpu's default adapter selection (no power preference) picked the **Intel Iris Xe** on this
dual-GPU laptop. That's why Ferrocade's GPU-bound scenes (static, engine_moved) topped out around
100–112k while Arcade (OpenGL, on the NVIDIA GPU) drew over 512k static sprites.

A temporary experiment with `PowerPreference::HighPerformance` selected the **NVIDIA RTX 3500**:
static and engine_moved then both reached the 512k ramp cap. The new `Renderer::adapter_name()`
diagnostic shows which GPU is in use.

**Decision (owner, 2026-10-12): option A.** Ferrocade now prefers the high-performance GPU by default
(`PowerPreference::HighPerformance`). The other options were (B) keeping the low-power default with an
opt-in, and (C) a setting with a recommended default. The results table above was measured before
this change; the NVIDIA column shows the new default's behavior on this machine.

## Fairness notes

- **Pacing:** Ferrocade uses vsync (Fifo). Arcade was set to vsync with 1/240 update and draw rates,
  because sleep-based pacing on Windows skipped frames (16/33 ms frames at ~40–50 FPS with almost no
  load). Its vsync didn't actually engage (see above). pygame-ce is capped with `Clock.tick(60)`.
  The work-time rule makes the comparison about CPU cost under any pacing.
- **Isolation:** each engine/scene runs in a fresh process. Repeated `arcade.run()` calls in one
  process throttled later windows to ~40 FPS.
- **Arcade's "engine_moved"** is its own `SpriteList.update()`, which is a Python loop. It has no
  native bulk update, so this is Arcade's real equivalent path.
- **Pygame blits on the CPU**, so its drawing cost grows with the sprite count. Its scenes don't
  rotate sprites (per-frame `transform.rotate` would measure image rotation instead).
- **GPU assignment:** Arcade and Pygame used whatever GPU their windowing stack chose. Ferrocade's
  default was the integrated GPU, which disadvantages it in the GPU-bound scenes.
- **Ramp cap:** "≥ 512,000" means the next doubling (1,024,000) exceeded the tool's limit of
  1,000,000. The true maximum is higher.

## Web (informational)

The browser build runs the same engine, and the Phase 2 `--motion` check showed the bouncing ship
moving through the real loop. Per-call overhead from Pyodide → JS → WASM wasn't stress-measured
in this phase. A web variant of `python_moved` remains a follow-up (web spike risk 2).

## Recommendations

1. ~~Decide the GPU preference default.~~ Done: high-performance GPU by default (see the finding above).
2. **For the showcase:** movement and detection in the engine (as decided). The 75× update gap and
   the 0.25–0.67 µs per-sprite Python costs show that per-sprite Python callbacks must be avoided
   for thousands of objects.
3. **Raise the ramp cap** (or switch to a time-boxed search) for future GPU benchmarks, so the
   discrete-GPU limits can be found.
4. **Optional later:** a vsync-off benchmark mode, if raw FPS numbers are wanted. That's in the backlog.
