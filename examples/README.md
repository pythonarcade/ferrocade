# Examples

One runnable example per roadmap phase (see spec §10), numbered in order:
- `01_centered_ship.py`: Phase 1
- `01c_shapes.py`: Phase 1c (CSS colors, rectangle and circle sprites)
- `02_bouncing_ship.py`: Phase 2
- Phase 3's stress test lives in `benches/` (see `benches/README.md`)
- `04_keyboard_ship.py`: Phase 4. Arrow keys or WASD steer the ship; Space re-centers it.
  In the browser: `http://localhost:8000/web/?example=04_keyboard_ship` (click the game to focus it).
- `05_collisions.py`: Phase 5. Steer the ship (arrow keys or WASD) through drifting asteroids;
  touched asteroids disappear and the title counts hits. Gold circles use `CircleCollider`, tomato
  squares `BoxCollider`, light blue squares `AABBCollider`. Browser: `?example=05_collisions`.
