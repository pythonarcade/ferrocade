"""Sprite stress test: find the most sprites each scene sustains at 60 FPS, per engine.

Run from the repository root (see benches/README.md):
    python benches/stress.py [--scene NAME|all] [--engine ferrocade|arcade|pygame|all] [--quick]

A step is *sustainable* when, over the measurement window, FPS >= 59 and the per-frame work
(update + game code + draw queueing + engine render CPU) is under 14 ms. Work time is used
because vsync pins frame time at ~16.7 ms and would hide the headroom.
"""

import argparse
import datetime
import importlib
import json
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path

SCENES = ("static", "engine_moved", "python_moved", "shapes")
ENGINES = ("ferrocade", "arcade", "pygame")
WIDTH, HEIGHT = 800, 600
SPRITE_IMAGE = "assets/images/ship.png"
SPRITE_SCALE = 0.25

MIN_FPS = 59.0
MAX_WORK_MS = 14.0
TARGET_SCENE, TARGET_ENGINE, TARGET_COUNT = "engine_moved", "ferrocade", 10_000


@dataclass
class Measurement:
    n: int
    fps: float
    frame_ms_avg: float
    frame_ms_p95: float
    update_ms: float
    game_ms: float
    draw_ms: float
    render_cpu_ms: float
    sustainable: bool

    @property
    def work_ms(self):
        return self.update_ms + self.game_ms + self.draw_ms + self.render_cpu_ms


class Done(Exception):
    """Raised by scene runners to leave their game loop when the ramp is finished."""


@dataclass
class Ramp:
    """Drives one scene: decides sprite counts, collects measurements, finds the maximum.

    Scene runners call `frame(...)` once per displayed frame and act on its answer:
    an int means "resize to this many sprites", None means "keep going"; `Done` is raised
    when the search is complete.
    """

    quick: bool = False
    start_n: int = 1000
    max_n: int = 1_000_000
    measurements: list = field(default_factory=list)
    n: int = 0
    _phase: str = "warmup"
    _phase_end: float = 0.0
    _frames: list = field(default_factory=list)
    _last_frame: float | None = None
    _passed: int = 0
    _failed: int | None = None

    def __post_init__(self):
        self.warmup_s, self.measure_s = (0.5, 1.0) if self.quick else (1.0, 3.0)
        self.resolution = 0.15 if self.quick else 0.05
        self.n = self.start_n

    def frame(self, now, update_ms, game_ms, draw_ms, render_cpu_ms=0.0):
        interval = None if self._last_frame is None else now - self._last_frame
        self._last_frame = now
        if self._phase_end == 0.0:
            self._phase_end = now + self.warmup_s
        if self._phase == "warmup":
            if now >= self._phase_end:
                self._phase, self._phase_end, self._frames = "measure", now + self.measure_s, []
            return None
        if interval is not None:
            self._frames.append((interval, update_ms, game_ms, draw_ms, render_cpu_ms))
        if now < self._phase_end:
            return None
        return self._finish_step()

    def _finish_step(self):
        frames = self._frames
        intervals = sorted(f[0] for f in frames)
        duration = sum(intervals)
        mean = lambda i: statistics.fmean(f[i] for f in frames)  # noqa: E731
        m = Measurement(
            n=self.n,
            fps=len(frames) / duration if duration else 0.0,
            frame_ms_avg=duration / len(frames) * 1000,
            frame_ms_p95=intervals[int(len(intervals) * 0.95) - 1] * 1000,
            update_ms=mean(1), game_ms=mean(2), draw_ms=mean(3), render_cpu_ms=mean(4),
            sustainable=False,
        )
        m.sustainable = m.fps >= MIN_FPS and m.work_ms < MAX_WORK_MS
        self.measurements.append(m)
        print(f"    n={m.n:>8,}  fps={m.fps:6.1f}  work={m.work_ms:6.2f} ms "
              f"(update {m.update_ms:.2f} | game {m.game_ms:.2f} | draw {m.draw_ms:.2f} "
              f"| render {m.render_cpu_ms:.2f})  {'ok' if m.sustainable else 'FAIL'}", flush=True)

        if m.sustainable:
            self._passed = max(self._passed, m.n)
        else:
            self._failed = m.n if self._failed is None else min(self._failed, m.n)
        if self._failed is None:  # still ramping up
            next_n = m.n * 2
            if next_n > self.max_n:
                raise Done
        else:
            low, high = self._passed, self._failed
            if high - low <= max(low * self.resolution, 50):
                raise Done
            next_n = (low + high) // 2
        self.n = next_n
        self._phase, self._phase_end, self._last_frame = "warmup", 0.0, None
        return next_n

    @property
    def max_sustainable(self):
        return self._passed

    def dominant_cost(self):
        failing = [m for m in self.measurements if not m.sustainable]
        if not failing:
            return "none (never failed)"
        m = min(failing, key=lambda x: x.n)
        if m.fps < MIN_FPS and m.work_ms < MAX_WORK_MS:
            return "GPU/presentation (frame rate dropped with CPU work under budget)"
        parts = {"engine update": m.update_ms, "game code": m.game_ms,
                 "draw queueing": m.draw_ms, "engine render CPU": m.render_cpu_ms}
        return max(parts, key=lambda k: parts[k])


def machine_description():
    gpu = "unknown"
    try:
        gpu = subprocess.run(
            ["powershell", "-NoProfile", "-Command", "(Get-CimInstance Win32_VideoController).Name"],
            capture_output=True, text=True, timeout=20).stdout.strip().replace("\r\n", "; ")
    except (OSError, subprocess.SubprocessError):
        pass
    return {"os": platform.platform(), "cpu": platform.processor(), "gpu": gpu,
            "python": platform.python_version()}


def engine_versions():
    versions = {}
    for name, module, attr in (("ferrocade", "ferrocade", "__version__"),
                               ("arcade", "arcade", "__version__"),
                               ("pygame-ce", "pygame", "version.ver")):
        try:
            obj = importlib.import_module(module)
            for part in attr.split("."):
                obj = getattr(obj, part)
            versions[name] = obj
        except ImportError:
            versions[name] = None
    try:
        versions["ferrocade_commit"] = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    except OSError:
        pass
    return versions


def run_isolated(engine, scene, quick):
    """Run one engine/scene in a fresh subprocess, so no run inherits another's state.

    (Arcade/pyglet throttles later windows when `arcade.run()` is called repeatedly in one
    process, and isolation is fairer for every engine anyway.) Returns a result dict.
    """
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "result.json"
        cmd = [sys.executable, "-u", __file__, "--engine", engine, "--scene", scene,
               "--child-out", str(out)] + (["--quick"] if quick else [])
        subprocess.run(cmd, check=False)
        if not out.exists():
            return {"engine": engine, "scene": scene, "max_sustainable_n": None,
                    "dominant_cost": None, "note": "run failed (see output above)",
                    "measurements": []}
        return json.loads(out.read_text(encoding="utf-8"))


def run_engine_scene(engine, scene, quick):
    """Run one scene in one engine; returns (Ramp, error message or None)."""
    try:
        module = importlib.import_module(f"scenes_{engine}")
    except ImportError as e:
        return None, f"{engine} not available ({e.name} not installed)"
    ramp = Ramp(quick=quick)
    reason = module.unsupported(scene)
    if reason:
        return None, f"n/a: {reason}"
    print(f"\n== {engine} / {scene}", flush=True)
    module.run(scene, ramp)
    return ramp, None


def main():
    parser = argparse.ArgumentParser(description="Sprite stress test")
    parser.add_argument("--scene", default="all", choices=("all",) + SCENES)
    parser.add_argument("--engine", default="all", choices=("all",) + ENGINES)
    parser.add_argument("--quick", action="store_true", help="coarser, faster ramp (smoke test)")
    parser.add_argument("--out", default="benches/results")
    parser.add_argument("--child-out", help=argparse.SUPPRESS)  # internal: one isolated run
    args = parser.parse_args()

    sys.path.insert(0, str(Path(__file__).parent))
    # Scene modules do `from stress import ...`; make that this running module, not a copy,
    # so the `Done` they raise is the one we catch.
    sys.modules.setdefault("stress", sys.modules[__name__])
    scenes = SCENES if args.scene == "all" else (args.scene,)
    engines = ENGINES if args.engine == "all" else (args.engine,)
    started = time.perf_counter()

    if args.child_out:
        # Isolated child run: exactly one engine and scene.
        ramp, note = run_engine_scene(args.engine, args.scene, args.quick)
        Path(args.child_out).write_text(json.dumps({
            "engine": args.engine, "scene": args.scene,
            "max_sustainable_n": ramp.max_sustainable if ramp else None,
            "dominant_cost": ramp.dominant_cost() if ramp else None,
            "note": note,
            "measurements": [asdict(m) | {"work_ms": m.work_ms} for m in ramp.measurements] if ramp else [],
        }), encoding="utf-8")
        return 0

    results = [run_isolated(engine, scene, args.quick) for engine in engines for scene in scenes]

    print("\n" + "=" * 78)
    print(f"{'engine':<10} {'scene':<14} {'max sprites @60':>16}   dominant cost at the limit")
    print("-" * 78)
    for r in results:
        value = f"{r['max_sustainable_n']:,}" if r["max_sustainable_n"] is not None else (r["note"] or "-")
        print(f"{r['engine']:<10} {r['scene']:<14} {value:>16}   {r['dominant_cost'] or ''}")
    elapsed = time.perf_counter() - started
    print(f"\nTotal time: {elapsed / 60:.1f} min")

    report = {"date": datetime.datetime.now().isoformat(timespec="seconds"),
              "quick": args.quick, "machine": machine_description(), "versions": engine_versions(),
              "rule": {"min_fps": MIN_FPS, "max_work_ms": MAX_WORK_MS},
              "results": results, "elapsed_s": elapsed}
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    path = out / f"{datetime.datetime.now():%Y%m%d-%H%M%S}-{platform.node()}.json"
    path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"Results: {path}")

    target = next((r for r in results if r["engine"] == TARGET_ENGINE and r["scene"] == TARGET_SCENE), None)
    if target and target["max_sustainable_n"] is not None:
        met = target["max_sustainable_n"] >= TARGET_COUNT
        print(f"Target ({TARGET_COUNT:,} {TARGET_ENGINE} {TARGET_SCENE} sprites at 60 FPS): "
              f"{'MET' if met else 'MISSED'} ({target['max_sustainable_n']:,})")
        return 0 if met else 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
