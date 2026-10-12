# Data Model: Phase 3 — Sprite Stress Test (Performance)

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Engine additions

### FrameStats (core)

| Field | Type | Meaning |
|---|---|---|
| `draw_calls` | `u32` | Draw calls in the last frame (existing). |
| `instances` | `u32` | Instances drawn in the last frame (existing). |
| `render_cpu_ms` | `f64` | **New.** CPU time of `render_frame`, excluding the surface-acquire wait (R1). |

### Window frame statistics (exposed to Python as `Window.frame_stats`)

| Field | Type | Meaning |
|---|---|---|
| `fps` | `float` | Frames per second, averaged over the last ~1 s. |
| `frame_ms` | `float` | Average wall-clock time between frames over the last ~1 s. |
| `render_cpu_ms` | `float` | Average `render_cpu_ms` over the last ~1 s. |
| `draw_calls`, `instances` | `int` | From the most recent frame. |

**Rules**:
- `frame_stats` is read-only, and updated by the game loop once per frame (cheap: a running sum
  plus a once-per-second roll).
- Before the first second of running, it reports the values so far (`fps` can be 0).

### Window FPS readout

| Field | Type | Rules |
|---|---|---|
| `show_fps` | `bool`, default `False` | When `True`, the title becomes `"{title} — {fps:.0f} FPS, {frame_ms:.2f} ms"`, refreshed about once per second. Setting it back to `False` restores `{title}` on the next refresh. |

## Benchmark entities (`benches/`)

| Entity | Fields |
|---|---|
| **Scene** | `name` (`static`, `engine_moved`, `python_moved`, `shapes`), `engine` (`ferrocade`, `arcade`, `pygame`), `setup(n)`, `step(dt)` |
| **Measurement** | `scene`, `engine`, `n`, `fps`, `frame_ms_avg`, `frame_ms_p95`, `update_ms`, `game_ms`, `draw_ms`, `render_cpu_ms` (Ferrocade only), `sustainable` (bool) |
| **Result** | `scene`, `engine`, `max_sustainable_n`, `dominant_cost`, `machine`, `versions`, `date` |

**Sustainable rule** (R1): `fps ≥ 59` **and** `update_ms + game_ms + draw_ms + render_cpu_ms < 14`
(for other engines, their measured update + draw work).

**Output**: `benches/results/<date>-<machine>.json` (gitignored), and the summarized results committed
in `docs/research/phase3-benchmarks.md`.
