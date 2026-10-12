"""Collision benchmark: query times against the Phase 5 targets (owner, 2026-10-12).

Targets on the development machine (release build):
- all colliding pairs among 10,000 moving sprites in one list: median < 2 ms per frame
- one sprite against a list of 10,000: median < 0.5 ms

Scene (research R9): 10,000 mixed circle / box / axis-aligned box colliders of 4-12 px over
1920x1080, with random velocities and spins, moved by `SpriteList.update` every frame (so the
grid rebuilds every frame). Times are what a game pays: the Python call, including building
the result. No window is needed.

Run from the repository root after `maturin develop --release`:
    python benches/collisions.py
"""

import random
import statistics
import sys
import time

import ferrocade as fc

N = 10_000
FRAMES, WARMUP = 300, 30
TARGETS = {"pairs_10k": 2.0, "sprite_vs_10k": 0.5}


def make_list(width, height, seed=1, moving=True):
    rng = random.Random(seed)
    sprites = fc.SpriteList()
    for i in range(N):
        size = rng.uniform(4, 12)
        s = fc.RectangleSprite(size, size, (255, 255, 255), rng.uniform(0, width),
                               rng.uniform(0, height))
        kind = i % 3
        if kind == 0:
            s.collider = fc.CircleCollider(size / 2)
        elif kind == 1:
            s.collider = fc.BoxCollider(size, size)
        else:
            s.collider = fc.AABBCollider(size, size)
        if moving:
            s.velocity = (rng.uniform(-60, 60), rng.uniform(-60, 60))
            s.angular_velocity = rng.uniform(-90, 90)
        sprites.append(s)
    return sprites


def measure(name, sprites, query):
    times, results = [], 0
    for frame in range(WARMUP + FRAMES):
        sprites.update(1 / 60)
        t0 = time.perf_counter()
        result = query()
        elapsed = (time.perf_counter() - t0) * 1000
        if frame >= WARMUP:
            times.append(elapsed)
            results = len(result)
    times.sort()
    median, p95 = statistics.median(times), times[int(len(times) * 0.95) - 1]
    target = TARGETS.get(name)
    verdict = "" if target is None else ("PASS" if median < target else "FAIL")
    target_text = f"< {target} ms" if target else "(info)"
    print(f"{name:<18} median {median:7.3f} ms   p95 {p95:7.3f} ms   results {results:>6,}   "
          f"target {target_text:<9} {verdict}", flush=True)
    return verdict != "FAIL"


def main():
    print(f"{N:,} colliders, {FRAMES} timed frames after {WARMUP} warm-up frames\n")
    ok = True

    sprites = make_list(1920, 1080)
    ok &= measure("pairs_10k", sprites, sprites.collisions)

    probe = fc.RectangleSprite(40, 40, (255, 255, 255), 960, 540)
    probe.collider = fc.BoxCollider(40, 40)
    ok &= measure("sprite_vs_10k", sprites, lambda: probe.collides_with_list(sprites))

    static = make_list(1920, 1080, moving=False)
    ok &= measure("pairs_10k_static", static, static.collisions)

    dense = make_list(800, 600)
    ok &= measure("pairs_10k_dense", dense, dense.collisions)

    print("\nAll targets met." if ok else "\nA target was MISSED.")
    return 0 if ok else 2


if __name__ == "__main__":
    sys.exit(main())
