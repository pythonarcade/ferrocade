# Contract: `ferrocade` Python API additions (Phase 3)

**Feature**: [../spec.md](../spec.md)

These two small, opt-in additions to `Window` serve developers (the readout) and tools (the stats).
The desktop module and the browser shim MUST both satisfy them.

```python
class Window:
    show_fps: bool                 # default False; read/write
    @property
    def frame_stats(self) -> FrameStats: ...   # read-only

class FrameStats:                  # read-only snapshot
    fps: float            # frames per second, averaged over ~1 s
    frame_ms: float       # average time between frames, ~1 s window
    render_cpu_ms: float  # average engine render CPU time per frame (excludes the vsync wait)
    draw_calls: int       # last frame
    instances: int        # last frame
```

| Behavior | Contract |
|---|---|
| `show_fps = True` | The window title shows `"<title> — <fps> FPS, <frame_ms> ms"`, refreshed about once per second. In the browser, `document.title` is updated. |
| `show_fps = False` (default) | The title is just `<title>`. There's no measurable overhead. |
| `frame_stats` | Valid while the window runs. Before `run()`, all values are 0. |

## Benchmark tool (not part of the `ferrocade` package)

```text
python benches/stress.py [--scene static|engine_moved|python_moved|shapes|all] [--engine ferrocade|arcade|pygame|all]
                         [--quick] [--out benches/results/]
```

| Behavior | Contract |
|---|---|
| Default | All scenes, all engines that are installed. It prints a summary table and writes JSON. |
| `--quick` | A coarser ramp (for smoke testing, under 1 minute). |
| Missing engine | Skipped with a note ("arcade not installed"). Not an error. |
| Exit code | 0, unless the Ferrocade engine-moved scene misses the 10,000-sprite target, which gives 2 (the result is still reported). |
