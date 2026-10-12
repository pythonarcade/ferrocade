# Implementation Plan: Phase 3 — Sprite Stress Test (Performance)

**Branch**: `005-sprite-stress-test` (spec directory; work happens on `main` locally) | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-sprite-stress-test/spec.md`

## Summary

**The stress tool:**
- Build `benches/stress.py`, which ramps the sprite count in four scenes (static, engine-moved,
  Python-moved, shapes) until the frame rate can't be sustained.
- It measures **work time** (engine update, game code, draw queueing, and engine render CPU), so
  headroom shows even under vsync.
- It finds each scene's maximum sustainable count, names the dominant cost, and writes a results
  table, run against Ferrocade, **Arcade 4.0 pre-release (`4.0.0.dev8`)**, and **pygame-ce 2.5.8** (both chosen by the owner).

**The target:** 10,000 engine-moved sprites at 60 FPS.

**Small engine additions:** `FrameStats.render_cpu_ms`, `Window.frame_stats`, and an opt-in
`Window.show_fps` title readout.

**If the target is missed:** profile and optimize the measured bottleneck (the hypotheses are in R6).

## Technical Context

**Language/Version**: Rust stable 1.99; Python 3.14 venv (≥ 3.13 supported).

**Primary Dependencies**: there are no new Ferrocade dependencies. **Dev-only** benchmark dependencies: `arcade==4.0.0.dev8` (pulls `pyglet==3.0.dev11`) and `pygame-ce==2.5.8` (in `benches/requirements.txt`).

**Storage**: JSON results in `benches/results/` (gitignored). The summary is committed in `docs/research/phase3-benchmarks.md`.

**Testing**:
- `cargo test` (stats populated)
- `pytest` (`frame_stats`, `show_fps`)
- the benchmark runs themselves (P2–P4)

**Target Platform**: the Windows dev machine (reference). The browser is informational.

**Project Type**: a performance investigation, plus small engine instrumentation and a readout API.

**Performance Goals**: **≥ 10,000 engine-moved sprites at 60 FPS** (FR-006). The full benchmark takes ≤ 10 minutes (SC-004).

**Constraints**:
- No vsync-off API (R1).
- No engine-side bounce (R2, out of scope until the collision design).
- Instrumentation costs ~0 when unused.
- The instance stays 48 bytes.

**Scale/Scope**: about 150 lines of Rust and Python for the stats and readout, about 500 lines of benchmark Python across three engines, and the report.

There are no NEEDS CLARIFICATION items left. The owner answered all three spec questions (target, comparison engines, NumPy deferred).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre | Post-design |
|---|---|---|---|
| I. Python API, Rust Core | Instrumentation in the core. A thin API. | ✅ | ✅ `render_cpu_ms` and the stats roll-up live in Rust. The bindings expose read-only stats and a bool. |
| II. Minimize conversion | Bulk work in Rust, measured. | ✅ | ✅ This phase *verifies* II with numbers. Scene (b) keeps per-sprite Python work at zero (R2). |
| III. Portable | Web parity for the API; no platform-only features. | ✅ | ✅ `show_fps` works on desktop (`set_title`) and web (`document.title`). Timing uses `web_time`. The web numbers are informational. |
| IV. Simple first | No speculative features. | ✅ | ✅ No vsync toggle, no GPU timers, no engine bounce. Optimizations happen only if measurements demand them. |
| V. Spec-driven decisions | Owner decisions followed; open designs untouched. | ✅ | ✅ The target and engines are as decided. The NumPy path is deferred. The collision and response design isn't pre-empted (R2). Arcade 4.0 dev8 and pygame-ce were both chosen by the owner. |
| VI. Phase-based delivery | Runnable tool plus criteria. | ✅ | ✅ `benches/stress.py`, P1–P7, and the report. |

**Result**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/005-sprite-stress-test/
├── plan.md
├── research.md        # R1–R6
├── data-model.md
├── quickstart.md      # P1–P7
├── contracts/python-api.md
├── checklists/requirements.md
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/ferrocade/src/render/mod.rs + sprites.rs   # FrameStats.render_cpu_ms (excludes acquire wait)
crates/ferrocade/src/window.rs                    # rolling frame statistics (fps, frame_ms, render_cpu_ms)
crates/ferrocade/src/platform/desktop.rs          # update stats per frame; show_fps title refresh
crates/ferrocade-py/src/py_window.rs              # show_fps property; frame_stats -> FrameStats object
python/ferrocade/_ferrocade.pyi                   # stubs
tests/test_window.py                              # frame_stats + show_fps tests
crates/ferrocade-web/src/lib.rs + web/python/ferrocade/__init__.py  # stats + show_fps parity
benches/requirements.txt                          # arcade==4.0.0.dev8, pygame-ce==2.5.8 (dev-only)
benches/stress.py                                 # ramp/search driver, table + JSON output
benches/scenes_ferrocade.py                       # the four scenes for Ferrocade
benches/scenes_arcade.py                          # equivalent scenes for Arcade
benches/scenes_pygame.py                          # equivalent scenes for pygame-ce
examples/02_bouncing_ship.py                      # --fps flag (sets show_fps)
docs/research/phase3-benchmarks.md                # results, breakdown, bottlenecks, fairness notes
```

**Structure Decision**: the benchmarks live in `benches/`, as the repo layout planned. The engine
changes are small and additive.

## Implementation Order (for /speckit-tasks)

1. **Instrumentation:** `render_cpu_ms`, rolling window stats, `Window.frame_stats`, and `show_fps`
   (desktop and web), plus tests.
2. **Ferrocade scenes and the ramp/search driver.** Run the quick and target measurements.
3. **If the target is missed:** profile the dominant cost and optimize (R6 hypotheses), then re-measure.
4. **The Arcade and pygame-ce scenes** (dev venv installs), then the full comparison run.
5. **The report** (`phase3-benchmarks.md`), the example `--fps` flag, regression, and spec updates.

## Complexity Tracking

No violations.
