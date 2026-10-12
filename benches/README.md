# Benchmarks

The Phase 3 sprite stress test: it finds the maximum sprite count each scene sustains at 60 FPS,
for Ferrocade and for the comparison engines (Arcade 4.0 pre-release, pygame-ce).

## Setup (dev venv only)

```powershell
uv pip install --prerelease=allow -r benches/requirements.txt   # Arcade 4.0 pins a pyglet pre-release
maturin develop --release                                       # always benchmark the optimized build
```

## Run

```powershell
python benches/stress.py --quick                                   # smoke test, under 1 minute
python benches/stress.py --scene engine_moved --engine ferrocade   # the 10,000-sprite target
python benches/stress.py                                           # everything (up to about 10 minutes)
```

**Sustainable** means, over a 3 s window after a 1 s warm-up: **FPS ≥ 59 and work < 14 ms per
frame**. Work is engine update + game code + draw queueing + engine render CPU. The rule measures
work time because vsync pins frame time at about 16.7 ms and hides headroom.

Results go to `benches/results/` (gitignored). The summaries are in `docs/research/phase3-benchmarks.md`.
