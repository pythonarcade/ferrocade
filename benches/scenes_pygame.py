"""pygame-ce versions of the stress-test scenes.

Notes on equivalence:
- Pygame blits on the CPU, so "draw" includes the actual rendering (fill + blits + flip).
- "engine_moved" uses `Group.update()` (Pygame's own update path; a Python loop internally).
- **No spin**: rotating each sprite per frame would need `transform.rotate`, which measures
  image rotation rather than sprite throughput, so Pygame scenes don't rotate (documented).
- The loop is capped at 60 FPS with `Clock.tick(60)`, mirroring vsync; the tick wait is
  not counted as work.
- Pygame's y axis points down; it doesn't matter for these scenes.
"""

import random
import time

import pygame

from stress import HEIGHT, SPRITE_IMAGE, SPRITE_SCALE, WIDTH, Done

DRIFT = 20.0


def unsupported(scene):
    return None


def _random_position():
    return random.uniform(20, WIDTH - 20), random.uniform(20, HEIGHT - 20)


class Mover(pygame.sprite.Sprite):
    def __init__(self, image, x, y, vx=0.0, vy=0.0):
        super().__init__()
        self.image = image
        self.rect = image.get_rect(center=(round(x), round(y)))
        self.x, self.y, self.vx, self.vy = x, y, vx, vy

    def update(self, dt):
        self.x += self.vx * dt
        self.y += self.vy * dt
        self.rect.center = (round(self.x), round(self.y))


class Scene:
    def __init__(self, kind):
        self.kind = kind
        ship = pygame.image.load(SPRITE_IMAGE).convert_alpha()
        w, h = ship.get_size()
        self.ship = pygame.transform.smoothscale(
            ship, (max(1, round(w * SPRITE_SCALE)), max(1, round(h * SPRITE_SCALE))))
        self.sprites = []
        self.group = pygame.sprite.Group()

    def _make(self):
        x, y = _random_position()
        if self.kind == "shapes":
            size = random.randint(4, 12)
            color = [random.randint(0, 255) for _ in range(3)]
            image = pygame.Surface((size, size), pygame.SRCALPHA)
            if random.random() < 0.5:
                image.fill(color)
            else:
                pygame.draw.circle(image, color, (size / 2, size / 2), size / 2)
        else:
            image = self.ship
        moving = self.kind in ("engine_moved", "shapes", "python_moved")
        v = (random.uniform(-DRIFT, DRIFT), random.uniform(-DRIFT, DRIFT)) if moving else (0, 0)
        return Mover(image, x, y, *v)

    def resize(self, n):
        if n < len(self.sprites):
            self.sprites = self.sprites[:n]
            self.group = pygame.sprite.Group(self.sprites)
        while len(self.sprites) < n:
            s = self._make()
            self.sprites.append(s)
            self.group.add(s)
        if self.kind in ("engine_moved", "shapes"):
            for s in self.sprites:
                s.x, s.y = _random_position()

    def update(self, dt):
        if self.kind in ("engine_moved", "shapes"):
            t0 = time.perf_counter()
            self.group.update(dt)
            return (time.perf_counter() - t0) * 1000, 0.0
        if self.kind == "python_moved":
            t0 = time.perf_counter()
            for s in self.sprites:
                s.x = (s.x + s.vx * dt) % WIDTH
                s.y = (s.y + s.vy * dt) % HEIGHT
                s.rect.center = (round(s.x), round(s.y))
            return 0.0, (time.perf_counter() - t0) * 1000
        return 0.0, 0.0


def run(scene, ramp):
    pygame.init()
    screen = pygame.display.set_mode((WIDTH, HEIGHT))
    pygame.display.set_caption(f"pygame-ce stress: {scene}")
    clock = pygame.time.Clock()
    world = Scene(scene)
    world.resize(ramp.n)
    try:
        while True:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    return
            dt = clock.tick(60) / 1000.0
            update_ms, game_ms = world.update(dt)
            t0 = time.perf_counter()
            screen.fill((32, 32, 32))
            world.group.draw(screen)
            pygame.display.flip()
            draw_ms = (time.perf_counter() - t0) * 1000
            next_n = ramp.frame(time.perf_counter(), update_ms, game_ms, draw_ms)
            if next_n is not None:
                world.resize(next_n)
    except Done:
        pass
    finally:
        pygame.quit()
