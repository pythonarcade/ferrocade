---

description: "Task list for Phase 3 — Sprite Stress Test (Performance)"
---

# Tasks: Phase 3 — Sprite Stress Test (Performance)

**Input**: Design documents from `specs/005-sprite-stress-test/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/python-api.md](contracts/python-api.md), [quickstart.md](quickstart.md)

**Tests**: included for the new engine API (`frame_stats`, `show_fps`). The benchmark runs
themselves are the phase's main verification (P2–P4 in quickstart.md).

**Organization**:
- **US1:** find Ferrocade's limits per scene (it includes the 10,000-sprite target)
- **US2:** compare with Arcade 4.0.0.dev8 and pygame-ce
- **US3:** the FPS readout

The frame statistics are foundational: the stress tool (US1) and the readout (US3) both use them.

## Format: `[ID] [P?] [Story] Description`

**Global rules**:
- No new Ferrocade dependencies. Arcade and pygame-ce are **dev-only**, listed in `benches/requirements.txt`.
- No vsync-off API, and no engine-side bounce (research R1, R2).
- Benchmark with the release build (`maturin develop --release`).
- Commit locally after each group; **push only when the owner says so**.

---

## Phase 1: Setup

- [X] T001 Create `benches/requirements.txt` with `arcade==4.0.0.dev8` and `pygame-ce==2.5.8` (with a comment saying dev-only, and that Arcade pins `pyglet==3.0.dev11`). Add `benches/results/` to `.gitignore`. Install with `uv pip install -r benches/requirements.txt`, then confirm that `import arcade, pygame` works and print both versions.
- [X] T002 [P] Create `benches/README.md` (replacing the placeholder text) with the commands from quickstart.md, the sustainability rule (`fps ≥ 59` and work < 14 ms), and a note to always benchmark `maturin develop --release`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: engine-side timing that the stress tool and the readout both need.

- [X] T003 Add `render_cpu_ms: f64` to `FrameStats` in `crates/ferrocade/src/render/sprites.rs` and `crates/ferrocade/src/render/mod.rs`. `render_frame` measures its CPU time with `web_time::Instant`, **"excluding the surface-acquire wait"**: start the timer after `acquire()` returns, and stop it after `queue.submit`, before `present`.
- [X] T004 Add rolling frame statistics to `crates/ferrocade/src/window.rs`: a `FrameStatsWindow` that accumulates per-frame (interval, `render_cpu_ms`) and rolls every ~1 s into `fps`, `frame_ms` (average interval), and `render_cpu_ms` (average), with `draw_calls` and `instances` taken from the last frame. Store it in `WindowState` (shared, readable from the bindings). "Before the first second, report the values so far (fps can be 0)."
- [X] T005 Update the stats once per frame in `crates/ferrocade/src/platform/desktop.rs` (in `redraw`, after `render_frame`), using the frame interval already computed for the timestep.
- [X] T006 Add a read-only `frame_stats` property to `Window` in `crates/ferrocade-py/src/py_window.rs`, returning a new frozen `#[pyclass(name = "FrameStats")]` with `fps`, `frame_ms`, `render_cpu_ms`, `draw_calls`, `instances` (all 0 before `run()`). Register it, then export and stub it in `python/ferrocade/_ferrocade.pyi` (and add `FrameStats` to `python/ferrocade/__init__.py`).
- [X] T007 [P] Add the web parity in `crates/ferrocade-web/src/lib.rs`: `frame_stats() -> Vec<f64>` (`[fps, frame_ms, render_cpu_ms, draw_calls, instances]`), updated in `end_frame` from the frame interval passed by the shim. Add `Window.frame_stats` to the web shim `web/python/ferrocade/__init__.py`, returning a simple object with the same attributes.
- [X] T008 Add tests to `tests/test_window.py` (`@pytest.mark.display`): run a window for ~1.5 s, then check `frame_stats.fps` is in [55, 65], `frame_ms` is about 16.7 (±2), `render_cpu_ms` > 0 and < `frame_ms`, and `draw_calls`/`instances` match a list of 3 sprites. Add a non-display test that `frame_stats` is all zeros before `run()`. Run `cargo test --workspace` and `pytest`.

**Checkpoint**: the engine reports its own frame timing.

---

## Phase 3: User Story 1 - Find the engine's limits, scene by scene (Priority: P1) 🎯 MVP

**Goal**: `benches/stress.py` finds the maximum sustainable sprite count per Ferrocade scene, with a time breakdown, and checks the **10,000 engine-moved sprites at 60 FPS** target.

**Independent Test**: `python benches/stress.py --engine ferrocade` prints the maximum N and the dominant cost for all four scenes. The engine-moved scene reaches ≥ 10,000 (exit code 0).

- [X] T009 [US1] Create `benches/scenes_ferrocade.py` with the four scenes from data-model.md, all in an 800×600 window using `assets/images/ship.png` at scale 0.25:
  - `static`: N sprites placed once and never updated.
  - `engine_moved`: "slow drift". Random positions inside the window, velocities ≤ 20 px/s, random `angular_velocity` ±90. Each update calls `sprite_list.update(dt)` only, with **no per-sprite Python work**. Add `rescatter()` to reset positions between ramp steps.
  - `python_moved`: N sprites whose `position` is set by a Python loop each update (a drift computed in Python).
  - `shapes`: N mixed `RectangleSprite`/`CircleSprite` (sizes 4–12, random CSS colors) with engine drift, as in `engine_moved`.
  - Every scene exposes `resize(n)` (adds or removes sprites in the existing list, without recreating the window) and per-update timing hooks: `update_ms` (the engine update call) and `game_ms` (Python scene logic).
- [X] T010 [US1] Create the driver `benches/stress.py` (research R1, R3).
  - **Measurement:** one window per engine run. Per step: set N, wait 1 s (warm-up), then measure for 3 s:
    - `fps`, average and p95 frame time (from `perf_counter` deltas in `on_draw`)
    - `update_ms` and `game_ms`, plus `draw_ms` (timed around `on_draw`'s queue calls)
    - `render_cpu_ms` (from `window.frame_stats`)
  - **Sustainable** = `fps ≥ 59` **and** `update_ms + game_ms + draw_ms + render_cpu_ms < 14`.
  - **Ramp:** 1k, 2k, 4k, … until a step fails, then binary search to 5% resolution.
  - **The dominant cost** is the largest work component at the first failing N.
  - **CLI:** `--scene`, `--engine`, `--quick` (coarser ramp, 0.5 s + 1 s steps), `--out benches/results/`.
  - **Output:** a summary table on stdout, plus JSON `benches/results/<date>-<machine>.json` with all measurements, the machine description (CPU, GPU, OS), and versions.
  - **Exit code:** **2 if the Ferrocade `engine_moved` maximum is < 10,000**, otherwise 0.
- [X] T011 [US1] `maturin develop --release`, then run `python benches/stress.py --quick --engine ferrocade` (P2), then `python benches/stress.py --scene engine_moved --engine ferrocade` (P3). Record the maximum N and the breakdown.
- [X] T012 [US1] **If P3 is below 10,000:** profile the dominant cost and fix it.
  - Candidates from research R6: avoid cloning `list.sprites` in `store::update_list` (iterate by index), batch the dirty-range marking, and cut per-sprite `Instance` rewrites for sprites that are in only one list.
  - Re-run P3 after each change, and record the before/after numbers.
  - **If P3 already passes**, still record the dominant cost and headroom at 10,000. This task is then just measurement.
- [X] T013 [US1] Run `python benches/stress.py --engine ferrocade` (all four scenes, full ramp) and keep the JSON for the report.

**Checkpoint**: US1 is complete. Ferrocade's limits are known, and the target is evaluated.

---

## Phase 4: User Story 2 - Compare with other Python engines on the same scenes (Priority: P2)

**Goal**: the same scenes in Arcade 4.0.0.dev8 and pygame-ce, with results in one table.

**Independent Test**: `python benches/stress.py` prints a table with the maximum N per scene for all three engines (or "n/a" with a reason).

- [X] T014 [P] [US2] Create `benches/scenes_arcade.py` against **Arcade 4.0.0.dev8**. Check each API against the installed package source (`arcade.Window`, `arcade.Sprite`, `arcade.SpriteList`, `SpriteSolidColor`, `SpriteCircle`, and the update and draw hooks), since 4.0 may differ from 3.x. Implement:
  - `static`
  - `engine_moved`: Arcade's `SpriteList.update()` with per-frame `change_x`/`change_y` equal to px/s ÷ 60, and `change_angle` for spin
  - `python_moved`: a Python loop setting `center_x`/`center_y`
  - `shapes`: solid-color and circle sprites
  - Use the same image, scale, window size, `resize(n)`, and timing hooks as the Ferrocade scenes.
  - Mark anything with no equivalent as "n/a", with the reason.
- [X] T015 [P] [US2] Create `benches/scenes_pygame.py` (pygame-ce 2.5.8). It uses an 800×600 display, `pygame.sprite.Group`, the ship image scaled once with `pygame.transform.smoothscale`, and `pygame.time.Clock`.
  - `static`: `Group.draw`.
  - `engine_moved`: `Group.update()` with sprites moving float positions into `rect`. Rotation: either pre-rendered rotation frames, or the spin omitted and documented (rotating each frame with `transform.rotate` would measure something else).
  - `python_moved`: an explicit loop setting `rect.center`.
  - `shapes`: pre-filled `Surface`s, plus circles drawn once to surfaces.
  - Use the same `resize(n)` and timing hooks. Note in the scene docstring that Pygame blits on the CPU.
- [X] T016 [US2] Extend `benches/stress.py` so `--engine arcade|pygame|all` drives those scene modules through the same ramp and sustainability rule. For engines without `render_cpu_ms`, work = update + game + draw time, measured in Python. Skip uninstalled engines with a note.
- [X] T017 [US2] Run the full comparison `python benches/stress.py` (P4). It must complete in ≤ 10 minutes. Keep the JSON.

**Checkpoint**: US2 is complete. The comparison table exists.

---

## Phase 5: User Story 3 - A visible frame-rate readout for developers (Priority: P3)

**Goal**: `Window.show_fps = True` shows FPS and frame time in the title on desktop and web.

**Independent Test**: `python examples/02_bouncing_ship.py --many --fps` shows a title like `"Bouncing ship — 60 FPS, 16.67 ms"` that updates about once per second.

- [X] T018 [US3] Add `show_fps: bool` to `WindowState`, with a title refresh in `crates/ferrocade/src/window.rs` and `crates/ferrocade/src/platform/desktop.rs`. When the stats roll over (about 1 s) and `show_fps` is set, call `window.set_title(format!("{title} — {fps:.0} FPS, {frame_ms:.2} ms"))`. When it's switched off, restore `title` once. When off, the per-frame cost is just a flag check.
- [X] T019 [US3] Add the `show_fps` property (read/write, default `False`) to `crates/ferrocade-py/src/py_window.rs`, with a stub in `python/ferrocade/_ferrocade.pyi`. Add the web shim's `Window.show_fps`, which updates `document.title` on the same ~1 s roll, in `web/python/ferrocade/__init__.py`.
- [X] T020 [US3] Add a display test to `tests/test_window.py`: set `show_fps = True`, run ~1.5 s, and confirm no error, with `frame_stats.fps > 0`. (The title text itself is the winit window's and can't be read back. The format is unit-tested by factoring it into a pure helper in `crates/ferrocade/src/window.rs` with a Rust test.) Add a `--fps` flag to `examples/02_bouncing_ship.py` that sets `self.show_fps = True` (P5).

**Checkpoint**: all three stories are complete.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T021 Write `docs/research/phase3-benchmarks.md`:
  - the machine and versions (Ferrocade commit, Arcade 4.0.0.dev8, pyglet 3.0.dev11, pygame-ce 2.5.8, Python 3.14)
  - the result table (maximum N per scene × engine)
  - the time breakdown per scene at the limit
  - the dominant costs
  - the **target verdict (10,000 engine-moved)**
  - the optimizations made in T012, with before/after numbers
  - fairness notes (Arcade's update is a Python-level loop, Pygame blits on the CPU, vsync handling)
  - an informational web measurement (the default `02_bouncing_ship --many` in the browser with `show_fps`)
  - recommendations for the showcase
  - Link it from `docs/research/README.md`.
- [X] T022 Run the regression and lint:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade --no-default-features --all-targets -- -D warnings`
  - `cargo clippy -p ferrocade-web --target wasm32-unknown-unknown -- -D warnings`
  - `cargo test --workspace`, `maturin develop`, `pytest`
  - the stub check against the runtime
  - the browser checks for `01_centered_ship`, `01c_shapes`, and `02_bouncing_ship --motion`
- [X] T023 Update `docs/spec/ENGINE_SPEC.md`: mark Phase 3 implemented in §10 with the headline numbers, add decision log entries for any optimizations or deviations, and add the backlog items found (for example, a vsync-off benchmark mode if useful).

---

## Dependencies & Execution Order

- **Setup (T001–T002)** comes first. T002 is parallel.
- **Foundational (T003–T008):** T003 → T004 → T005 → T006. T007 runs after T004. T008 runs last.
- **US1 (T009–T013):** after Foundational. T009 → T010 → T011 → T012 → T013.
- **US2 (T014–T017):** after T010 (the driver exists) and T001 (installs). T014 and T015 are parallel, then T016, then T017.
- **US3 (T018–T020):** after Foundational. It's independent of US1 and US2.
- **Polish (T021–T023):** last.

### Parallel Opportunities

- T002, T007, T014 + T015, and US3 alongside US1/US2 (different files).

## Parallel Example: User Story 2

```text
Task: "T014 Arcade 4.0 scenes in benches/scenes_arcade.py"
Task: "T015 pygame-ce scenes in benches/scenes_pygame.py"
```

## Implementation Strategy

1. Setup plus the foundation (the engine timing).
2. **US1 (MVP):** Ferrocade's limits and the 10,000-sprite target. Optimize if needed.
3. **US3:** the readout (small; useful while benchmarking).
4. **US2:** the comparison engines, then the full run.
5. The report, regression, and spec updates. Commit locally. **Push only on the owner's word.**
