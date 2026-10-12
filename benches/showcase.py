"""Showcase benchmark: maximum bouncing-ball counts, engine vs. Python bouncing, and the target.

Target (SC-001): 10,000 balls at 60 FPS in engine mode on the development machine, with per-frame
work (update + collision + render) under 14 ms.

Part 1 (headless): for growing ball counts, time 120 showcase steps (move + bounce) per mode
with no window, and report the largest count whose step stays under 14 ms.
Part 2 (windowed): run `examples/06_showcase.py --balls 10000` for a few seconds and read its
stats lines (real FPS and the engine's render/update/collision times).

Run from the repository root after `maturin develop --release`:
    python benches/showcase.py
"""

import importlib.util
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUDGET_MS = 14.0
STEPS = 120


def load_example():
    spec = importlib.util.spec_from_file_location("showcase", ROOT / "examples" / "06_showcase.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def step_ms(example, balls, mode):
    window = example.Showcase(balls=balls)
    window.mode = mode
    for _ in range(10):  # warm-up
        window.step(1 / 60)
    times = []
    for _ in range(STEPS if mode == "engine" else 30):
        t0 = time.perf_counter()
        window.step(1 / 60)
        times.append((time.perf_counter() - t0) * 1000)
    return statistics.median(times)


def headless(example):
    print(f"Headless step time (move + bounce), median of {STEPS} steps; budget {BUDGET_MS} ms\n")
    print(f"{'balls':>7}  {'engine ms':>10}  {'python ms':>10}")
    best = {"engine": 0, "python": 0}
    python_done = False
    for n in range(2000, 40001, 2000):
        engine = step_ms(example, n, "engine")
        if engine < BUDGET_MS:
            best["engine"] = n
        python = None
        if not python_done:
            python = step_ms(example, n, "python")
            if python < BUDGET_MS:
                best["python"] = n
            else:
                python_done = True  # it only gets slower
        python_text = f"{python:10.2f}" if python is not None else f"{'-':>10}"
        print(f"{n:>7}  {engine:10.2f}  {python_text}", flush=True)
        if engine >= BUDGET_MS:
            break
    return best


def windowed(balls=10000, seconds=6):
    out = subprocess.run(
        [sys.executable, str(ROOT / "examples" / "06_showcase.py"), "--balls", str(balls),
         "--seconds", str(seconds)],
        cwd=ROOT, capture_output=True, text=True, timeout=120).stdout
    lines = [line for line in out.splitlines() if line.startswith("balls=")]
    last = lines[-1] if lines else ""
    values = dict(re.findall(r"(\w+)=([\d.]+)", last))
    return last, {k: float(v) for k, v in values.items() if k != "mode"}


def main():
    example = load_example()
    best = headless(example)
    print(f"\nLargest count under {BUDGET_MS} ms per step: engine {best['engine']:,}, "
          f"python {best['python']:,}")

    last, v = windowed()
    print(f"\nWindowed, 10,000 balls (last stats line):\n  {last}")
    work = v.get("render", 0) + v.get("update", 0) + v.get("collision", 0)
    met = v.get("fps", 0) >= 59 and work < BUDGET_MS and v.get("balls") == 10000
    print(f"  work {work:.2f} ms; SC-001 (10,000 balls, fps >= 59, work < {BUDGET_MS} ms): "
          f"{'PASS' if met else 'FAIL'}")
    return 0 if met else 2


if __name__ == "__main__":
    sys.exit(main())
