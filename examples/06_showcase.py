"""Performance showcase: thousands of balls bouncing off the walls and each other.

The engine moves every ball, detects every contact, and (by default) does the bouncing too, with
two calls per update. Press T to switch the bouncing to Python, computed from the engine's
batched collision results, and watch the collision time change.

Keys:
    = or keypad +   add 1,000 balls
    - or keypad -   remove 1,000 balls
    T               switch bouncing between the engine and Python
    Space           pause / resume

About once per second the console shows the ball count, FPS, and the engine's per-frame render,
update (movement), and collision times.

Run from the repository root:
    python examples/06_showcase.py                       # starts with 5,000 balls
    python examples/06_showcase.py --balls 10000         # the 10,000-ball target
    python examples/06_showcase.py --seconds 6           # exit after 6 seconds (measurements)
In the browser: http://localhost:8000/web/?example=06_showcase (open the developer console).
"""

import math
import random
import sys
import time

import ferrocade as fc

WIDTH, HEIGHT = 1280, 720
WALL = 10
BATCH = 1000
K = fc.key


def option(name, default):
    """`--name value` from the command line (desktop only; the browser has no arguments)."""
    argv = getattr(sys, "argv", [])
    if name in argv[:-1]:
        return float(argv[argv.index(name) + 1])
    return default


def python_bounce(balls, walls):
    """The engine's bounce rules, in Python: same contacts, same order, same formulas."""
    for a, b in balls.collisions():
        if not (isinstance(a.collider, fc.CircleCollider) and isinstance(b.collider, fc.CircleCollider)):
            continue
        (x1, y1), (x2, y2) = a.position, b.position
        r1, r2 = a.collider.radius * abs(a.scale), b.collider.radius * abs(b.scale)
        dx, dy = x2 - x1, y2 - y1
        dist = math.sqrt(dx * dx + dy * dy)
        nx, ny = (dx / dist, dy / dist) if dist > 0 else (1.0, 0.0)
        (v1x, v1y), (v2x, v2y) = a.velocity, b.velocity
        vn = (v1x - v2x) * nx + (v1y - v2y) * ny
        if vn > 0:
            a.velocity = (v1x - vn * nx, v1y - vn * ny)
            b.velocity = (v2x + vn * nx, v2y + vn * ny)
        overlap = r1 + r2 - dist
        if overlap > 0:
            half = overlap / 2
            a.position = (x1 - half * nx, y1 - half * ny)
            b.position = (x2 + half * nx, y2 + half * ny)
    for ball, wall in balls.collisions_with(walls):
        if not isinstance(ball.collider, fc.CircleCollider):
            continue
        (cx, cy), r = ball.position, ball.collider.radius * abs(ball.scale)
        (wx, wy) = wall.position
        hw, hh = wall.collider.width * abs(wall.scale) / 2, wall.collider.height * abs(wall.scale) / 2
        lx, ly = cx - wx, cy - wy
        px, py = min(max(lx, -hw), hw), min(max(ly, -hh), hh)
        if (px, py) != (lx, ly):
            ex, ey = lx - px, ly - py
            dist = math.sqrt(ex * ex + ey * ey)
            if dist > r:
                continue
            nx, ny, depth = ex / dist, ey / dist, r - dist
        else:
            gap_x, gap_y = hw - abs(lx), hh - abs(ly)
            if gap_x <= gap_y:
                nx, ny, depth = (1.0 if lx >= 0 else -1.0), 0.0, gap_x + r
            else:
                nx, ny, depth = 0.0, (1.0 if ly >= 0 else -1.0), gap_y + r
        vx, vy = ball.velocity
        vn = vx * nx + vy * ny
        if vn < 0:
            ball.velocity = (vx - 2 * vn * nx, vy - 2 * vn * ny)
        if depth > 0:
            ball.position = (cx + depth * nx, cy + depth * ny)


class Showcase(fc.Window):
    def __init__(self, balls=5000, seconds=None):
        super().__init__(WIDTH, HEIGHT, "Ferrocade showcase", filter=fc.LINEAR)
        self.rng = random.Random(7)  # the same scene every run
        self.colors = list(fc.color.CSS_COLORS.values())
        self.walls = fc.SpriteList()
        for x, y, w, h in ((WIDTH / 2, WALL / 2, WIDTH, WALL), (WIDTH / 2, HEIGHT - WALL / 2, WIDTH, WALL),
                           (WALL / 2, HEIGHT / 2, WALL, HEIGHT), (WIDTH - WALL / 2, HEIGHT / 2, WALL, HEIGHT)):
            wall = fc.RectangleSprite(w, h, fc.color.DIM_GRAY, x, y)
            wall.collider = fc.AABBCollider(w, h)  # walls are axis-aligned and never move
            self.walls.append(wall)
        self.balls = fc.SpriteList()
        self.ball_list = []  # the balls in creation order (removal takes the newest)
        self.add_balls(int(balls))
        self.mode = "engine"
        self.paused = False
        self.seconds = seconds
        self.elapsed = 0.0
        self.since_report = 0.0
        self.python_ms = 0.0
        self.python_frames = 0

    def add_balls(self, count):
        for _ in range(count):
            r = self.rng.uniform(2, 4)
            x = self.rng.uniform(WALL + r, WIDTH - WALL - r)
            y = self.rng.uniform(WALL + r, HEIGHT - WALL - r)
            ball = fc.CircleSprite(r, self.rng.choice(self.colors), x, y)
            ball.collider = fc.CircleCollider(r)
            speed, heading = self.rng.uniform(40, 200), self.rng.uniform(0, 2 * math.pi)
            ball.velocity = (speed * math.cos(heading), speed * math.sin(heading))
            self.balls.append(ball)
            self.ball_list.append(ball)

    def remove_balls(self, count):
        for _ in range(min(count, len(self.ball_list))):
            self.balls.remove(self.ball_list.pop())

    def step(self, delta_time):
        """One update: move, then bounce off each other, then off the walls."""
        self.balls.update(delta_time)
        if self.mode == "engine":
            self.balls.bounce_each_other()
            self.balls.bounce_off(self.walls)
        else:
            started = time.perf_counter()
            python_bounce(self.balls, self.walls)
            self.python_ms += (time.perf_counter() - started) * 1000
            self.python_frames += 1

    def on_update(self, delta_time):
        if not self.paused:
            self.step(delta_time)
        self.elapsed += delta_time
        self.since_report += delta_time
        if self.since_report >= 1.0:
            self.since_report = 0.0
            self.report()
        if self.seconds is not None and self.elapsed >= self.seconds:
            raise SystemExit(0)

    def report(self):
        s = self.frame_stats
        line = (f"balls={len(self.ball_list)} fps={s.fps:.1f} render={s.render_cpu_ms:.2f}ms "
                f"update={s.update_ms:.2f}ms collision={s.collision_ms:.2f}ms mode={self.mode}")
        if self.mode == "python" and self.python_frames:
            line += f" python_bounce={self.python_ms / self.python_frames:.2f}ms"
        self.python_ms, self.python_frames = 0.0, 0
        print(line, flush=True)

    def on_key_press(self, key, modifiers):
        if key in (K.EQUAL, K.NUM_ADD):
            self.add_balls(BATCH)
        elif key in (K.MINUS, K.NUM_SUBTRACT):
            self.remove_balls(BATCH)
        elif key == K.T:
            self.mode = "python" if self.mode == "engine" else "engine"
        elif key == K.SPACE:
            self.paused = not self.paused

    def on_draw(self):
        self.clear()
        self.balls.draw()
        self.walls.draw()


if __name__ == "__main__":
    seconds = option("--seconds", None)
    window = Showcase(balls=int(option("--balls", 5000)), seconds=seconds)
    try:
        window.run()
    except SystemExit:
        pass
