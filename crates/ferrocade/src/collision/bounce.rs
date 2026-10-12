//! Bouncing: equal-mass, perfectly elastic circle responses (performance showcase).
//!
//! Contacts come from the collision queries (already sorted) and are resolved in one pass, in
//! order, each using the current velocities and positions. Only sprites with circle colliders
//! are moved; walls never move. The same steps, in the same order, can be reproduced in Python
//! from `collisions()` results.

use super::Shape;
use super::shape::{WorldShape, world_shape};
use crate::Result;
use crate::store::{ListId, SpriteId, SpriteStore};

impl SpriteStore {
    /// World shape of a sprite's collider, if it has one.
    fn collider_shape(&self, id: SpriteId) -> Option<WorldShape> {
        let s = self.sprites.get(id)?;
        let c = self.colliders.get(s.collider?)?;
        Some(world_shape(c, (s.center_x, s.center_y), s.scale, s.angle))
    }

    /// The sprite's world circle, if its collider is a circle.
    fn circle(&self, id: SpriteId) -> Option<((f64, f64), f64)> {
        let s = self.sprites.get(id)?;
        let c = self.colliders.get(s.collider?)?;
        match (c.shape, self.collider_shape(id)?) {
            (Shape::Circle { .. }, WorldShape::Circle { c, r }) => Some((c, r)),
            _ => None,
        }
    }

    fn velocity(&self, id: SpriteId) -> (f64, f64) {
        let s = &self.sprites[id];
        (s.velocity_x, s.velocity_y)
    }

    fn nudge(&mut self, id: SpriteId, dx: f64, dy: f64) -> Result<()> {
        let s = &self.sprites[id];
        let (x, y) = (s.center_x + dx, s.center_y + dy);
        self.set_position(id, x, y)
    }

    /// Bounce `list`'s circle sprites off each other as equal-mass, perfectly elastic circles.
    /// Approaching pairs exchange their velocity components along the line between centers;
    /// overlapping pairs are pushed apart equally. Returns the number of contacts resolved.
    pub fn bounce_within(&mut self, list: ListId) -> Result<usize> {
        let started = web_time::Instant::now();
        let result = self.bounce_within_untimed(list);
        self.add_collision_time(started);
        result
    }

    fn bounce_within_untimed(&mut self, list: ListId) -> Result<usize> {
        let pairs = self.within_list_untimed(list)?;
        let ids = self.list(list)?.sprites().to_vec();
        let mut resolved = 0;
        for (i, j) in pairs {
            let (a, b) = (ids[i as usize], ids[j as usize]);
            let (Some((c1, r1)), Some((c2, r2))) = (self.circle(a), self.circle(b)) else {
                continue;
            };
            let (dx, dy) = (c2.0 - c1.0, c2.1 - c1.1);
            let dist = (dx * dx + dy * dy).sqrt();
            let n = if dist > 0.0 {
                (dx / dist, dy / dist)
            } else {
                (1.0, 0.0)
            };
            let (v1, v2) = (self.velocity(a), self.velocity(b));
            let vn = (v1.0 - v2.0) * n.0 + (v1.1 - v2.1) * n.1;
            if vn > 0.0 {
                self.set_velocity(a, v1.0 - vn * n.0, v1.1 - vn * n.1)?;
                self.set_velocity(b, v2.0 + vn * n.0, v2.1 + vn * n.1)?;
            }
            let overlap = r1 + r2 - dist;
            if overlap > 0.0 {
                let half = overlap / 2.0;
                self.nudge(a, -half * n.0, -half * n.1)?;
                self.nudge(b, half * n.0, half * n.1)?;
            }
            resolved += 1;
        }
        Ok(resolved)
    }

    /// Bounce `list`'s circle sprites off `walls` (which never move): velocity into a wall is
    /// reversed (speed unchanged) and overlapping sprites are pushed out. Returns the number of
    /// contacts resolved.
    pub fn bounce_off(&mut self, list: ListId, walls: ListId) -> Result<usize> {
        let started = web_time::Instant::now();
        let result = self.bounce_off_untimed(list, walls);
        self.add_collision_time(started);
        result
    }

    fn bounce_off_untimed(&mut self, list: ListId, walls: ListId) -> Result<usize> {
        let pairs = self.list_vs_list_untimed(list, walls)?;
        let balls = self.list(list)?.sprites().to_vec();
        let wall_ids = self.list(walls)?.sprites().to_vec();
        let mut resolved = 0;
        for (i, j) in pairs {
            let ball = balls[i as usize];
            let Some((c, r)) = self.circle(ball) else {
                continue;
            };
            let Some(wall) = self.collider_shape(wall_ids[j as usize]) else {
                continue;
            };
            let Some((n, penetration)) = push_out(c, r, &wall) else {
                continue;
            };
            let v = self.velocity(ball);
            let vn = v.0 * n.0 + v.1 * n.1;
            if vn < 0.0 {
                self.set_velocity(ball, v.0 - 2.0 * vn * n.0, v.1 - 2.0 * vn * n.1)?;
            }
            if penetration > 0.0 {
                self.nudge(ball, penetration * n.0, penetration * n.1)?;
            }
            resolved += 1;
        }
        Ok(resolved)
    }
}

/// Outward normal from `wall` toward a circle at `c` with radius `r`, and how far the circle
/// must move along it to stop overlapping (≤ 0 if only touching). `None` if they don't touch.
fn push_out(c: (f64, f64), r: f64, wall: &WorldShape) -> Option<((f64, f64), f64)> {
    match *wall {
        WorldShape::Circle { c: wc, r: wr } => {
            let (dx, dy) = (c.0 - wc.0, c.1 - wc.1);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r + wr {
                return None;
            }
            let n = if dist > 0.0 {
                (dx / dist, dy / dist)
            } else {
                (1.0, 0.0)
            };
            Some((n, r + wr - dist))
        }
        WorldShape::Obb {
            c: bc,
            half,
            cos,
            sin,
        } => {
            // Into the box frame.
            let (dx, dy) = (c.0 - bc.0, c.1 - bc.1);
            let (lx, ly) = (dx * cos + dy * sin, -dx * sin + dy * cos);
            let (px, py) = (lx.clamp(-half.0, half.0), ly.clamp(-half.1, half.1));
            let to_world = |x: f64, y: f64| (x * cos - y * sin, x * sin + y * cos);
            if (px, py) != (lx, ly) {
                // Center outside the box: push away from the closest point.
                let (ex, ey) = (lx - px, ly - py);
                let dist = (ex * ex + ey * ey).sqrt();
                if dist > r {
                    return None;
                }
                Some((to_world(ex / dist, ey / dist), r - dist))
            } else {
                // Center inside the box: leave through the least-penetrated face.
                let (gap_x, gap_y) = (half.0 - lx.abs(), half.1 - ly.abs());
                let (local_n, depth) = if gap_x <= gap_y {
                    ((if lx >= 0.0 { 1.0 } else { -1.0 }, 0.0), gap_x)
                } else {
                    ((0.0, if ly >= 0.0 { 1.0 } else { -1.0 }), gap_y)
                };
                Some((to_world(local_n.0, local_n.1), depth + r))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::narrow::tests::Lcg;

    fn ball(store: &mut SpriteStore, x: f64, y: f64, r: f64, v: (f64, f64)) -> SpriteId {
        let id = store
            .create_circle(r, [255; 4], x, y, None)
            .expect("circle sprite");
        let c = store
            .create_collider(Shape::Circle { radius: r }, (0.0, 0.0))
            .unwrap();
        store.attach_collider(id, Some(c)).unwrap();
        store.set_velocity(id, v.0, v.1).unwrap();
        id
    }

    fn wall(store: &mut SpriteStore, x: f64, y: f64, w: f64, h: f64) -> SpriteId {
        let id = store.create_rectangle(w, h, [255; 4], x, y).unwrap();
        let c = store
            .create_collider(
                Shape::Aabb {
                    width: w,
                    height: h,
                },
                (0.0, 0.0),
            )
            .unwrap();
        store.attach_collider(id, Some(c)).unwrap();
        id
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    fn list_of(store: &mut SpriteStore, ids: &[SpriteId]) -> ListId {
        let list = store.create_list();
        for &id in ids {
            store.append(list, id).unwrap();
        }
        list
    }

    #[test]
    fn head_on_swap() {
        let mut s = SpriteStore::new();
        let a = ball(&mut s, 0.0, 0.0, 5.0, (10.0, 0.0));
        let b = ball(&mut s, 10.0, 0.0, 5.0, (-10.0, 0.0));
        let l = list_of(&mut s, &[a, b]);
        assert_eq!(s.bounce_within(l).unwrap(), 1);
        assert!(close(s.velocity(a), (-10.0, 0.0)));
        assert!(close(s.velocity(b), (10.0, 0.0)));
    }

    #[test]
    fn angled_pair_conserves_momentum_and_energy() {
        let mut s = SpriteStore::new();
        let a = ball(&mut s, 0.0, 0.0, 4.0, (30.0, 5.0));
        let b = ball(&mut s, 5.0, 4.0, 4.0, (-7.0, -12.0)); // touching at an angle (overlapping)
        let l = list_of(&mut s, &[a, b]);
        let (v1, v2) = (s.velocity(a), s.velocity(b));
        let p0 = (v1.0 + v2.0, v1.1 + v2.1);
        let e0 = v1.0 * v1.0 + v1.1 * v1.1 + v2.0 * v2.0 + v2.1 * v2.1;
        s.bounce_within(l).unwrap();
        let (w1, w2) = (s.velocity(a), s.velocity(b));
        assert!(close((w1.0 + w2.0, w1.1 + w2.1), p0), "momentum");
        let e1 = w1.0 * w1.0 + w1.1 * w1.1 + w2.0 * w2.0 + w2.1 * w2.1;
        assert!((e1 - e0).abs() < 1e-9, "energy {e0} -> {e1}");
        // Tangential components are unchanged: n = (5, 4)/|·|, t = (-4, 5)/|·|.
        let t = (-4.0 / 41f64.sqrt(), 5.0 / 41f64.sqrt());
        let tan = |v: (f64, f64)| v.0 * t.0 + v.1 * t.1;
        assert!((tan(w1) - tan(v1)).abs() < 1e-9 && (tan(w2) - tan(v2)).abs() < 1e-9);
        // The overlap was removed: centers are exactly r1 + r2 apart.
        let (c1, c2) = (s.circle(a).unwrap().0, s.circle(b).unwrap().0);
        let d = ((c2.0 - c1.0).powi(2) + (c2.1 - c1.1).powi(2)).sqrt();
        assert!((d - 8.0).abs() < 1e-9, "{d}");
    }

    #[test]
    fn separating_pairs_keep_velocities_but_lose_overlap() {
        let mut s = SpriteStore::new();
        let a = ball(&mut s, 0.0, 0.0, 5.0, (-3.0, 0.0));
        let b = ball(&mut s, 6.0, 0.0, 5.0, (3.0, 0.0));
        let l = list_of(&mut s, &[a, b]);
        s.bounce_within(l).unwrap();
        assert!(close(s.velocity(a), (-3.0, 0.0)) && close(s.velocity(b), (3.0, 0.0)));
        assert!(close(s.circle(a).unwrap().0, (-2.0, 0.0)));
        assert!(close(s.circle(b).unwrap().0, (8.0, 0.0)));
    }

    #[test]
    fn wall_reflection_keeps_speed() {
        let mut s = SpriteStore::new();
        // Floor at y in [-10, 0]; ball touching it, moving down at 30° below horizontal.
        let floor = wall(&mut s, 0.0, -5.0, 200.0, 10.0);
        let (vx, vy) = (
            100.0 * 30f64.to_radians().cos(),
            -100.0 * 30f64.to_radians().sin(),
        );
        let b = ball(&mut s, 0.0, 3.0, 3.0, (vx, vy));
        let (balls, walls) = (list_of(&mut s, &[b]), list_of(&mut s, &[floor]));
        assert_eq!(s.bounce_off(balls, walls).unwrap(), 1);
        assert!(close(s.velocity(b), (vx, -vy)));
    }

    #[test]
    fn ball_inside_a_wall_is_pushed_out_the_near_side() {
        let mut s = SpriteStore::new();
        let w = wall(&mut s, 0.0, 0.0, 10.0, 100.0); // x in [-5, 5]
        let b = ball(&mut s, 4.0, 0.0, 2.0, (-50.0, 0.0)); // inside, nearer the right face
        let (balls, walls) = (list_of(&mut s, &[b]), list_of(&mut s, &[w]));
        s.bounce_off(balls, walls).unwrap();
        assert!(
            close(s.circle(b).unwrap().0, (7.0, 0.0)),
            "pushed out right"
        );
        assert!(close(s.velocity(b), (50.0, 0.0)), "reflected");
    }

    #[test]
    fn corner_contact_pushes_diagonally() {
        let mut s = SpriteStore::new();
        let w = wall(&mut s, 0.0, 0.0, 10.0, 10.0); // corner at (5, 5)
        let b = ball(&mut s, 6.0, 6.0, 2.0, (-1.0, -1.0));
        let (balls, walls) = (list_of(&mut s, &[b]), list_of(&mut s, &[w]));
        s.bounce_off(balls, walls).unwrap();
        let k = 5.0 + 2.0 / 2f64.sqrt();
        assert!(close(s.circle(b).unwrap().0, (k, k)));
        assert!(close(s.velocity(b), (1.0, 1.0)));
    }

    #[test]
    fn non_circle_sprites_are_not_bounced() {
        let mut s = SpriteStore::new();
        let a = wall(&mut s, 0.0, 0.0, 10.0, 10.0);
        s.set_velocity(a, 5.0, 0.0).unwrap();
        let b = ball(&mut s, 6.0, 0.0, 3.0, (-5.0, 0.0));
        let l = list_of(&mut s, &[a, b]);
        assert_eq!(s.bounce_within(l).unwrap(), 0);
        assert!(close(s.velocity(a), (5.0, 0.0)) && close(s.velocity(b), (-5.0, 0.0)));
    }

    #[test]
    fn timings_accumulate_and_reset() {
        let mut s = SpriteStore::new();
        let a = ball(&mut s, 0.0, 0.0, 5.0, (1.0, 0.0));
        let l = list_of(&mut s, &[a]);
        s.update_list(l, 1.0 / 60.0).unwrap();
        s.bounce_within(l).unwrap();
        let (update_ms, collision_ms) = s.take_frame_timings();
        assert!(update_ms > 0.0 && collision_ms > 0.0);
        assert_eq!(s.take_frame_timings(), (0.0, 0.0));
    }

    /// The showcase scene: balls in a walled 1280×720 box.
    fn scene(seed: u64, n: usize) -> (SpriteStore, ListId, ListId) {
        let mut s = SpriteStore::new();
        let (w, h, t) = (1280.0, 720.0, 10.0);
        let walls = [
            wall(&mut s, w / 2.0, t / 2.0, w, t),
            wall(&mut s, w / 2.0, h - t / 2.0, w, t),
            wall(&mut s, t / 2.0, h / 2.0, t, h),
            wall(&mut s, w - t / 2.0, h / 2.0, t, h),
        ];
        let mut rng = Lcg(seed);
        let mut balls = Vec::new();
        for _ in 0..n {
            let r = rng.range(2.0, 4.0);
            let (x, y) = (rng.range(t + r, w - t - r), rng.range(t + r, h - t - r));
            let (speed, dir) = (
                rng.range(40.0, 200.0),
                rng.range(0.0, std::f64::consts::TAU),
            );
            balls.push(ball(
                &mut s,
                x,
                y,
                r,
                (speed * dir.cos(), speed * dir.sin()),
            ));
        }
        let (bl, wl) = (list_of(&mut s, &balls), list_of(&mut s, &walls));
        (s, bl, wl)
    }

    fn energy(s: &SpriteStore, list: ListId) -> f64 {
        s.list(list)
            .unwrap()
            .sprites()
            .iter()
            .map(|&id| {
                let (vx, vy) = s.velocity(id);
                vx * vx + vy * vy
            })
            .sum()
    }

    #[test]
    fn simulation_conserves_energy_and_stays_inside() {
        let run = || {
            let (mut s, balls, walls) = scene(42, 2000);
            let e0 = energy(&s, balls);
            for _ in 0..600 {
                s.update_list(balls, 1.0 / 60.0).unwrap();
                s.bounce_within(balls).unwrap();
                s.bounce_off(balls, walls).unwrap();
            }
            let e1 = energy(&s, balls);
            assert!((e1 - e0).abs() / e0 < 0.01, "energy {e0} -> {e1}");
            let positions: Vec<(f64, f64)> = s
                .list(balls)
                .unwrap()
                .sprites()
                .iter()
                .map(|&id| s.circle(id).unwrap().0)
                .collect();
            for &(x, y) in &positions {
                assert!(
                    (10.0..=1270.0).contains(&x) && (10.0..=710.0).contains(&y),
                    "{x},{y}"
                );
            }
            positions
        };
        assert_eq!(run(), run(), "deterministic");
    }
}
