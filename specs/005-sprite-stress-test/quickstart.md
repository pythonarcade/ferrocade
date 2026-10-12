# Quickstart & Validation: Phase 3 — Sprite Stress Test (Performance)

**Feature**: [spec.md](spec.md) | **API**: [contracts/python-api.md](contracts/python-api.md)

## Setup (one-time, dev venv only)

```powershell
uv pip install -r benches/requirements.txt     # arcade==4.0.0.dev8 (+ pyglet 3.0.dev11), pygame-ce==2.5.8
maturin develop --release                       # benchmark the optimized build
```

## Validation scenarios

| # | Scenario | How | Expected | Covers |
|---|---|---|---|---|
| P1 | Unit tests | `cargo test --workspace`, `pytest` | Pass, including `render_cpu_ms` populated, `frame_stats` values, and the `show_fps` title format (display test). | FR-008, FR-011 |
| P2 | Quick smoke | `python benches/stress.py --quick` | It completes in under 1 min and prints a table for all scenes and installed engines. | FR-001, FR-003 |
| P3 | **Target** | `python benches/stress.py --scene engine_moved --engine ferrocade` | Max sustainable N **≥ 10,000** (exit code 0). | FR-006, SC-002 |
| P4 | Full run | `python benches/stress.py` | It completes in ≤ 10 min and writes JSON plus a summary. | SC-001, SC-003, SC-004 |
| P5 | Readout | `python examples/02_bouncing_ship.py --many --fps` (a new flag sets `show_fps`) | The title shows FPS and ms, updating about once per second. | US3, SC-005 |
| P6 | Report | Read `docs/research/phase3-benchmarks.md` | A table with the max N per scene and engine, a time breakdown, the dominant cost, fairness notes, and fixes for any misses. | FR-009, FR-010 |
| P7 | Browser (informational) | `?example=02_bouncing_ship` plus a web stress scene | The numbers are recorded, not pass/fail. | Edge case (web) |
