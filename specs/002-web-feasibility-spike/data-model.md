# Data Model: Phase 1b — Web Feasibility Spike

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

This spike produces two kinds of data: **findings** (which make up the report) and the **browser
runtime objects** that approach B introduces.

## Findings

### CandidateApproach

| Field | Values / type | Rules |
|---|---|---|
| `id` | `A`, `A′`, `B`, `C` | A′ is the "wheel + custom WebGL2 renderer" variant (R3). |
| `status` | `proven` \| `blocked` \| `not attempted` \| `assessed on paper` | Every approach ends with exactly one status (SC-001). |
| `evidence` | list of Evidence | Required for `proven`/`blocked`. A `not attempted` or `assessed on paper` status needs a written reason. |
| `measurements` | Measurement | Only for `proven`. |
| `blockers` | list of text | Each blocker is specific enough that someone can later check whether it's resolved (upstream issue link, error text, version). |
| `engine_impact` | list of EngineChange | — |

### Evidence

| Field | Type | Rules |
|---|---|---|
| `step` | text | What was run, with the versions of tools and runtime. |
| `result` | text | Observed output: error text, console log, screenshot path, or pixel check. |
| `date` | ISO date | — |

### Measurement

| Field | Type | Rules |
|---|---|---|
| `download_bytes` | `{runtime, engine, assets, total}` | Cold load, measured (not estimated). |
| `time_to_ship_ms` | `{cold, warm}` | The median of 3 runs. Targets: ≤ 30 000 cold, ≤ 10 000 warm (SC-005). Over-target values are recorded, not hidden. |
| `graphics_paths` | `{webgpu: ok/fail/untested, webgl2: …}` | At least one must be `ok` for `proven`. |
| `centering` | bounding box + center offset | Must be within 1 px (SC-003). |
| `example_diff_lines` | integer | ≤ 5, with each line justified (SC-004). |

### EngineChange

| Field | Values | Rules |
|---|---|---|
| `description` | text | — |
| `class` | `done in spike` \| `required before shipping web` \| `nice to have` | FR-010. |
| `spike_shortcut` | bool | If true, it's also marked in the code with a `SPIKE:` comment (FR-012). |

### SpikeReport (`docs/research/web-spike.md`)

The sections, in order:
1. Summary and recommendation
2. Per-approach findings (one CandidateApproach each)
3. Comparison table
4. Engine impact
5. Risks
6. Next steps
7. Reproduction steps

The recommendation is marked **Proposed** until the owner accepts it (FR-015).

## Browser runtime (approach B)

```
Pyodide (CPython 3.14 in WASM)                  ferrocade-web (Rust → wasm32-unknown-unknown)
──────────────────────────────                  ─────────────────────────────────────────────
web/python/ferrocade (pure-Python shim)          Engine  (owns Renderer + global SpriteStore)
  Window  ── rAF proxy ─────────────────────▶      advance(elapsed) / begin_frame / end_frame
  Texture ── path, bytes ───────────────────▶      load_texture → texture id, width, height
  Sprite  ── sprite id ─────────────────────▶      create_sprite / set_* / get_* / destroy
  SpriteList ── list id ────────────────────▶      create_list / append / remove / draw / destroy
                              (calls go Python → JS → WASM via Pyodide's FFI)
```

| Entity | Lives in | Key state | Rules |
|---|---|---|---|
| `Engine` | ferrocade-web | Renderer (canvas surface), window filter, logical size, `FixedTimestep` | Created async **before** Python runs. There's one per page. |
| Shim `Window` | Python | the engine handle, rAF proxy, `running` flag | `run()` arms requestAnimationFrame and returns. A callback exception stops the loop and reports the error. `filter` is required, as on desktop. |
| Shim `Texture` | Python | `path`, `filter`, engine texture id, width/height | Bytes come from Pyodide's filesystem, and the engine caches by path (read and decoded once). Equality uses (path, filter), as on desktop. |
| Shim `Sprite` | Python | engine sprite id | Every property reads or writes the engine (no Python-side copies), keeping the store as the single source of truth. `__del__` destroys the engine sprite. |
| Shim `SpriteList` | Python | engine list id, Python refs to its members | Same semantics as desktop (order, duplicate check, multiple membership). |

**ID representation**: slotmap keys are 64-bit, and JS numbers are only exact up to 2^53. The web
API passes IDs as `u32` indices into an engine-side handle table (or as BigInt). The choice is an
implementation detail recorded in the contract.
