# Quickstart & Validation: Phase 1c — CSS Colors and Shape Sprites

**Feature**: [spec.md](spec.md) | **API**: [contracts/python-api.md](contracts/python-api.md)

The build is the same as Phase 1 (`maturin develop`) and the web spike (`tools\build_web.ps1`). See
the root `CLAUDE.md` "Build & test".

## Validation scenarios

| # | Scenario | How | Expected | Covers |
|---|---|---|---|---|
| Q1 | Rust tests | `cargo test --workspace` | Pass. They include: the tint is exact for opaque shape colors, image sprites with a white tint are pixel-identical to before, rectangle and circle sizes are within 1 px, the circle is round (corners transparent), and **1,000 interleaved shapes give 1 draw call**. | FR-007, FR-011–013; SC-003–005 |
| Q2 | Python tests | `pytest` | Pass. They include: color constants (148, pairs, spec values), `Sprite.color` default and validation, shape constructors and validation, subclassing, and `texture` assignment on a shape raising `TypeError`. | FR-001–006, FR-009–012, FR-016 |
| Q3 | CSS values against a real CSS engine | `node tools/web-check/check_colors.mjs` | All 148 names match the values Chrome computes for the CSS keywords. 0 missing, 0 extra. | SC-001 |
| Q4 | Desktop example | `python examples/01c_shapes.py` | Rectangles and circles in named colors at the stated positions. The `NEAREST` circle has stepped edges and the `LINEAR` circle is smooth. The ship is tinted. | US2, US3; FR-015 |
| Q5 | Phase 1 unchanged | `python examples/01_centered_ship.py` plus the existing pixel tests | Identical to before (dark gray background, untinted ship). | FR-017; SC-005 |
| Q6 | Browser parity | `tools\build_web.ps1 -Serve`, then the check script on `/web/?example=01c_shapes` | The same shapes and colors as desktop. Compare the screenshots. | FR-014; SC-006 |
| Q7 | 1,000-shape scene | `python examples/01c_shapes.py --many` | It holds the refresh rate (the title bar shows the frame time). | SC-004 |
