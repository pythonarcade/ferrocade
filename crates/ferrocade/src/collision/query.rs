//! Batched collision queries over the store, with a per-list cache rebuilt only when the list's
//! content version changes (unchanged lists cost nothing on repeated queries).

use super::grid::Grid;
use super::narrow::overlaps;
use super::shape::{Bounds, WorldShape, bounds, world_shape};
use crate::store::{ListId, SpriteId, SpriteState, SpriteStore};
use crate::{Error, Result};

/// Lists with fewer colliders than this test every pair instead of building a grid.
const GRID_THRESHOLD: usize = 32;

/// World shapes, bounds, and the grid for one list, valid for `version`.
#[derive(Debug, Default)]
pub struct CollisionCache {
    /// `None` until first built (so version 0 lists are built too).
    version: Option<u64>,
    shapes: Vec<Option<WorldShape>>,
    bounds: Vec<Option<Bounds>>,
    /// Slots that have a collider.
    present: Vec<u32>,
    /// Built lazily: pair queries need it, but a single probe after a change is cheaper as a
    /// linear scan over the bounds. It's built on the second probe of an unchanged list.
    grid: Option<Grid>,
    grid_ready: bool,
    probes_since_refresh: u32,
    /// How many times this cache was rebuilt (for tests and diagnostics).
    pub(crate) rebuilds: u64,
}

impl CollisionCache {
    /// Build the grid if this list is big enough and it isn't built for the current contents.
    fn ensure_grid(&mut self) {
        if self.grid_ready {
            return;
        }
        if self.present.len() >= GRID_THRESHOLD {
            let mut grid = self.grid.take().unwrap_or_default();
            grid.rebuild(&self.bounds, &self.present);
            self.grid = Some(grid);
        }
        self.grid_ready = true;
    }

    fn grid(&self) -> Option<&Grid> {
        self.grid.as_ref().filter(|_| self.grid_ready)
    }
}

fn sprite_shape(store: &SpriteStore, sprite: &SpriteState) -> Option<WorldShape> {
    let collider = store.colliders.get(sprite.collider?)?;
    Some(world_shape(
        collider,
        (sprite.center_x, sprite.center_y),
        sprite.scale,
        sprite.angle,
    ))
}

fn missing_list() -> Error {
    Error::InvalidArgument("sprite list no longer exists".into())
}

impl SpriteStore {
    /// Make sure `list`'s collision cache matches its current contents.
    fn refresh_collision_cache(&mut self, list: ListId) -> Result<()> {
        let state = self.lists.get(list).ok_or_else(missing_list)?;
        if state.collision.version == Some(state.content_version) {
            return Ok(());
        }
        let version = state.content_version;
        let mut cache = std::mem::take(&mut self.lists[list].collision);
        cache.shapes.clear();
        cache.bounds.clear();
        cache.present.clear();
        for (slot, &id) in self.lists[list].sprites().iter().enumerate() {
            let shape = sprite_shape(self, &self.sprites[id]);
            if shape.is_some() {
                cache.present.push(slot as u32);
            }
            cache.bounds.push(shape.as_ref().map(bounds));
            cache.shapes.push(shape);
        }
        cache.grid_ready = false;
        cache.probes_since_refresh = 0;
        cache.version = Some(version);
        cache.rebuilds += 1;
        self.lists[list].collision = cache;
        Ok(())
    }

    /// Whether two sprites' colliders overlap or touch. `false` if either has no collider, or
    /// for a sprite with itself.
    pub fn collides(&self, a: SpriteId, b: SpriteId) -> Result<bool> {
        let (sa, sb) = (self.sprite(a)?, self.sprite(b)?);
        if a == b {
            return Ok(false);
        }
        Ok(match (sprite_shape(self, sa), sprite_shape(self, sb)) {
            (Some(x), Some(y)) => overlaps(&x, &y),
            _ => false,
        })
    }

    /// Slots in `list` whose colliders touch `sprite`'s, ascending (never the sprite itself).
    pub fn sprite_vs_list(&mut self, sprite: SpriteId, list: ListId) -> Result<Vec<u32>> {
        self.refresh_collision_cache(list)?;
        let s = self.sprite(sprite)?;
        let Some(probe) = sprite_shape(self, s) else {
            return Ok(Vec::new());
        };
        let own_slot = s
            .memberships()
            .iter()
            .find(|(l, _)| *l == list)
            .map(|&(_, slot)| slot);
        let cache = &mut self.lists[list].collision;
        cache.probes_since_refresh += 1;
        if cache.probes_since_refresh >= 2 {
            cache.ensure_grid();
        }
        let mut hits = Vec::new();
        probe_cache(cache, &probe, |slot| {
            if Some(slot) != own_slot {
                hits.push(slot);
            }
        });
        hits.sort_unstable();
        Ok(hits)
    }

    /// Every touching `(slot in a, slot in b)` pair once, sorted. A sprite in both lists is never
    /// paired with itself.
    pub fn list_vs_list(&mut self, a: ListId, b: ListId) -> Result<Vec<(u32, u32)>> {
        self.refresh_collision_cache(a)?;
        self.refresh_collision_cache(b)?;
        self.lists[b].collision.ensure_grid();
        let (la, lb) = (&self.lists[a], &self.lists[b]);
        let (ca, cb) = (&la.collision, &lb.collision);
        let mut pairs = Vec::new();
        for &i in &ca.present {
            let shape = ca.shapes[i as usize].as_ref().unwrap();
            let id = la.sprites()[i as usize];
            probe_cache(cb, shape, |j| {
                if lb.sprites()[j as usize] != id {
                    pairs.push((i, j));
                }
            });
        }
        pairs.sort_unstable();
        Ok(pairs)
    }

    /// Every touching pair within `list` once, as `(earlier, later)` slots, sorted.
    pub fn within_list(&mut self, list: ListId) -> Result<Vec<(u32, u32)>> {
        self.refresh_collision_cache(list)?;
        self.lists[list].collision.ensure_grid();
        let cache = &self.lists[list].collision;
        let mut pairs = Vec::new();
        let mut test = |i: u32, j: u32| {
            let (bi, bj) = (
                cache.bounds[i as usize].unwrap(),
                cache.bounds[j as usize].unwrap(),
            );
            if bi.overlaps(&bj)
                && overlaps(
                    cache.shapes[i as usize].as_ref().unwrap(),
                    cache.shapes[j as usize].as_ref().unwrap(),
                )
            {
                pairs.push((i, j));
            }
        };
        match cache.grid() {
            Some(grid) => grid.pairs(&cache.present, test),
            None => {
                for (k, &i) in cache.present.iter().enumerate() {
                    for &j in &cache.present[k + 1..] {
                        test(i, j);
                    }
                }
            }
        }
        pairs.sort_unstable();
        Ok(pairs)
    }

    /// How many times `list`'s collision cache has been rebuilt (diagnostics and tests).
    pub fn collision_rebuilds(&self, list: ListId) -> Result<u64> {
        Ok(self.list(list)?.collision.rebuilds)
    }
}

/// Call `hit(slot)` for each collider in `cache` touching `probe` (each at most once).
fn probe_cache(cache: &CollisionCache, probe: &WorldShape, mut hit: impl FnMut(u32)) {
    let pb = bounds(probe);
    let mut test = |slot: u32| {
        if let (Some(b), Some(shape)) = (&cache.bounds[slot as usize], &cache.shapes[slot as usize])
            && b.overlaps(&pb)
            && overlaps(probe, shape)
        {
            hit(slot);
        }
    };
    match cache.grid() {
        Some(grid) => grid.query(&pb, test),
        None => cache.present.iter().for_each(|&s| test(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::Shape;
    use crate::collision::narrow::tests::Lcg;

    /// A store with `n` random sprites (mixed colliders, rotations, scales, offsets) in one list.
    fn random_scene(seed: u64, n: usize, max_size: f64) -> (SpriteStore, ListId, Vec<SpriteId>) {
        let mut store = SpriteStore::new();
        let list = store.create_list();
        let mut rng = Lcg(seed);
        let mut ids = Vec::new();
        for i in 0..n {
            let id = store
                .create_rectangle(
                    10.0,
                    10.0,
                    [255; 4],
                    rng.range(0.0, 800.0),
                    rng.range(0.0, 600.0),
                )
                .unwrap();
            store.set_angle(id, rng.range(0.0, 360.0)).unwrap();
            store.set_scale(id, rng.range(0.0, 3.0)).unwrap();
            if i % 9 != 0 {
                let size = if i % 50 == 0 {
                    max_size * 10.0
                } else {
                    rng.range(1.0, max_size)
                };
                let shape = match i % 3 {
                    0 => Shape::Circle { radius: size / 2.0 },
                    1 => Shape::Box {
                        width: size,
                        height: rng.range(1.0, max_size),
                    },
                    _ => Shape::Aabb {
                        width: size,
                        height: rng.range(1.0, max_size),
                    },
                };
                let offset = (rng.range(-5.0, 5.0), rng.range(-5.0, 5.0));
                let c = store.create_collider(shape, offset).unwrap();
                store.attach_collider(id, Some(c)).unwrap();
            }
            store.append(list, id).unwrap();
            ids.push(id);
        }
        (store, list, ids)
    }

    fn brute_pairs(store: &SpriteStore, ids: &[SpriteId]) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                if store.collides(ids[i], ids[j]).unwrap() {
                    out.push((i as u32, j as u32));
                }
            }
        }
        out
    }

    #[test]
    fn queries_match_brute_force() {
        for (seed, n, size) in [(1, 10, 30.0), (2, 100, 30.0), (3, 1500, 20.0)] {
            let (mut store, list, ids) = random_scene(seed, n, size);
            let expected = brute_pairs(&store, &ids);
            assert!(!expected.is_empty() || n < 20, "seed {seed}: no overlaps");
            assert_eq!(store.within_list(list).unwrap(), expected, "seed {seed}");
            for (k, &probe) in ids.iter().enumerate().step_by(13) {
                let want: Vec<u32> = (0..ids.len() as u32)
                    .filter(|&j| store.collides(probe, ids[j as usize]).unwrap())
                    .collect();
                assert_eq!(
                    store.sprite_vs_list(probe, list).unwrap(),
                    want,
                    "probe {k}"
                );
            }
        }
    }

    #[test]
    fn list_vs_list_matches_brute_force_and_skips_shared_sprites() {
        let (mut store, a, ids) = random_scene(5, 600, 25.0);
        // Second list: every third sprite of the first (shared) plus some new ones.
        let b = store.create_list();
        let mut b_ids = Vec::new();
        for &id in ids.iter().step_by(3) {
            store.append(b, id).unwrap();
            b_ids.push(id);
        }
        for k in 0..100 {
            let id = store
                .create_rectangle(5.0, 5.0, [255; 4], (k * 8) as f64, 300.0)
                .unwrap();
            let c = store
                .create_collider(Shape::Circle { radius: 6.0 }, (0.0, 0.0))
                .unwrap();
            store.attach_collider(id, Some(c)).unwrap();
            store.append(b, id).unwrap();
            b_ids.push(id);
        }
        let mut want = Vec::new();
        for (i, &x) in ids.iter().enumerate() {
            for (j, &y) in b_ids.iter().enumerate() {
                if store.collides(x, y).unwrap() {
                    want.push((i as u32, j as u32));
                }
            }
        }
        assert!(!want.is_empty());
        assert_eq!(store.list_vs_list(a, b).unwrap(), want);
    }

    #[test]
    fn grid_is_built_lazily_for_probes() {
        let (mut store, list, ids) = random_scene(4, 300, 20.0);
        let first = store.sprite_vs_list(ids[5], list).unwrap();
        assert!(
            !store.list(list).unwrap().collision.grid_ready,
            "first probe scans"
        );
        let second = store.sprite_vs_list(ids[5], list).unwrap();
        assert!(
            store.list(list).unwrap().collision.grid_ready,
            "second probe builds the grid"
        );
        assert_eq!(first, second);
        store.set_center_x(ids[0], 3.0).unwrap();
        store.within_list(list).unwrap();
        assert!(
            store.list(list).unwrap().collision.grid_ready,
            "pair queries build it"
        );
    }

    #[test]
    fn cache_rebuilds_only_after_changes() {
        let (mut store, list, ids) = random_scene(9, 200, 20.0);
        store.within_list(list).unwrap();
        let after_first = store.collision_rebuilds(list).unwrap();
        for _ in 0..5 {
            store.within_list(list).unwrap();
            store.sprite_vs_list(ids[3], list).unwrap();
        }
        assert_eq!(store.collision_rebuilds(list).unwrap(), after_first);
        // Moving one sprite, or changing a collider, invalidates the cache.
        store.set_center_x(ids[4], 1.0).unwrap();
        store.within_list(list).unwrap();
        assert_eq!(store.collision_rebuilds(list).unwrap(), after_first + 1);
        let c = store.sprite(ids[1]).unwrap().collider.unwrap();
        store
            .set_collider_shape(c, Shape::Circle { radius: 50.0 })
            .unwrap();
        store.within_list(list).unwrap();
        assert_eq!(store.collision_rebuilds(list).unwrap(), after_first + 2);
    }

    #[test]
    fn attach_detach_and_errors() {
        let mut store = SpriteStore::new();
        let a = store
            .create_rectangle(10.0, 10.0, [255; 4], 0.0, 0.0)
            .unwrap();
        let b = store
            .create_rectangle(10.0, 10.0, [255; 4], 5.0, 0.0)
            .unwrap();
        let ca = store
            .create_collider(Shape::Circle { radius: 3.0 }, (0.0, 0.0))
            .unwrap();
        let cb = store
            .create_collider(Shape::Circle { radius: 3.0 }, (0.0, 0.0))
            .unwrap();
        assert!(!store.collides(a, b).unwrap(), "no colliders yet");
        store.attach_collider(a, Some(ca)).unwrap();
        store.attach_collider(b, Some(cb)).unwrap();
        assert!(store.collides(a, b).unwrap());
        assert!(!store.collides(a, a).unwrap());
        assert!(
            store.attach_collider(b, Some(ca)).is_err(),
            "attached elsewhere"
        );
        // Replacing detaches the old collider; detaching with None.
        let cc = store
            .create_collider(Shape::Circle { radius: 1.0 }, (0.0, 0.0))
            .unwrap();
        store.attach_collider(b, Some(cc)).unwrap();
        assert_eq!(store.collider(cb).unwrap().attached_to(), None);
        assert!(!store.collides(a, b).unwrap());
        store.attach_collider(a, None).unwrap();
        assert_eq!(store.collider(ca).unwrap().attached_to(), None);
        // Destroying a sprite frees its collider for reuse.
        store.destroy_sprite(b);
        assert_eq!(store.collider(cc).unwrap().attached_to(), None);
        store.attach_collider(a, Some(cc)).unwrap();
        // Destroying an attached collider detaches it.
        store.destroy_collider(cc);
        assert_eq!(store.sprite(a).unwrap().collider, None);
        assert!(
            store
                .create_collider(Shape::Circle { radius: -1.0 }, (0.0, 0.0))
                .is_err()
        );
    }

    #[test]
    fn rotation_scale_and_offset_follow_the_sprite() {
        let mut store = SpriteStore::new();
        let ship = store
            .create_rectangle(10.0, 10.0, [255; 4], 100.0, 100.0)
            .unwrap();
        let probe = store
            .create_rectangle(1.0, 1.0, [255; 4], 100.0, 120.0)
            .unwrap();
        let c = store
            .create_collider(Shape::Circle { radius: 2.0 }, (20.0, 0.0))
            .unwrap();
        store.attach_collider(ship, Some(c)).unwrap();
        let p = store
            .create_collider(Shape::Circle { radius: 0.0 }, (0.0, 0.0))
            .unwrap();
        store.attach_collider(probe, Some(p)).unwrap();
        assert!(!store.collides(ship, probe).unwrap());
        store.set_angle(ship, 90.0).unwrap(); // offset (20, 0) now points up
        assert!(store.collides(ship, probe).unwrap());
        // An AABB offset doesn't rotate.
        store
            .set_collider_shape(
                c,
                Shape::Aabb {
                    width: 4.0,
                    height: 4.0,
                },
            )
            .unwrap();
        assert!(!store.collides(ship, probe).unwrap());
        store.set_position(probe, 120.0, 100.0).unwrap();
        assert!(store.collides(ship, probe).unwrap());
        // Scale 2 doubles the offset (now 40 px right).
        store.set_scale(ship, 2.0).unwrap();
        assert!(!store.collides(ship, probe).unwrap());
        store.set_position(probe, 140.0, 100.0).unwrap();
        assert!(store.collides(ship, probe).unwrap());
    }
}
