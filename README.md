# Ferrocade

A 2D game engine with a Python API and a Rust core, rendering through wgpu. Planned platforms: Windows, macOS, and the web.

> Early development: nothing runnable yet. See the [engine spec](docs/spec/ENGINE_SPEC.md) for the design and roadmap.

```python
import ferrocade as fc


class Game(fc.Window):
    def __init__(self):
        super().__init__(800, 600, "Ferrocade")
        self.sprites = fc.SpriteList()
        self.sprites.append(fc.Sprite("assets/images/ship.png", center_x=400, center_y=300))

    def on_draw(self):
        self.clear()
        self.sprites.draw()


Game().run()
```

## Repository layout

| Path | Contents |
|---|---|
| `docs/spec/` | Engine spec: the living design document and decision log |
| `docs/design/` | In-depth subsystem designs |
| `docs/research/` | Spikes, benchmark results, engine comparisons |
| `docs/guides/` | User-facing guides (later) |
| `crates/ferrocade/` | Rust engine core |
| `crates/ferrocade-py/` | PyO3 bindings |
| `python/ferrocade/` | Python package |
| `examples/` | One example per roadmap phase |
| `assets/` | Images, sounds, fonts |
| `tests/` | Python API tests |
| `benches/` | Performance scenes |
| `web/` | Browser (WASM/Pyodide) build |
| `tools/` | Developer scripts |
| `CLAUDE.md` | Instructions for AI coding assistants |
