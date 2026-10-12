//! Broad phase: a uniform grid over one list's collider bounds.
//!
//! Objects are bucketed into cells with a two-pass counting sort (flat arrays, no hashing).
//! A pair sharing several cells is reported only from its "reference cell" (the cell holding the
//! minimum corner of the two cell ranges' intersection), so no deduplication sets are needed.
//! Objects covering many cells go to a separate list that is tested against everything.

use super::shape::Bounds;

/// Grid dimensions are capped per axis (the cell size grows instead).
const MAX_DIM: usize = 1024;
/// Objects covering more cells than this are kept out of the cells.
const MAX_CELLS_PER_OBJECT: usize = 16;

#[derive(Debug, Default)]
pub struct Grid {
    origin: (f64, f64),
    inv_cell: f64,
    dims: (usize, usize),
    /// Items of cell `c` are `items[cell_start[c]..cell_start[c + 1]]`.
    cell_start: Vec<u32>,
    items: Vec<u32>,
    /// Cell range `[x0, y0, x1, y1]` per object index (unused for absent or large objects).
    ranges: Vec<[u32; 4]>,
    large: Vec<u32>,
    is_large: Vec<bool>,
    /// Reused working buffers.
    scratch_f64: Vec<f64>,
    scratch_u32: Vec<u32>,
}

impl Grid {
    /// Build over `bounds[i]` for every present object `i`.
    pub fn build(bounds: &[Option<Bounds>]) -> Grid {
        let present: Vec<u32> = (0..bounds.len() as u32)
            .filter(|&i| bounds[i as usize].is_some())
            .collect();
        let mut grid = Grid::default();
        grid.rebuild(bounds, &present);
        grid
    }

    /// Rebuild in place over `bounds[i]` for each `i` in `present`, reusing allocations (a
    /// moving list rebuilds every frame).
    pub fn rebuild(&mut self, bounds: &[Option<Bounds>], present: &[u32]) {
        self.ranges.clear();
        self.ranges.resize(bounds.len(), [0; 4]);
        self.is_large.clear();
        self.is_large.resize(bounds.len(), false);
        self.large.clear();
        self.items.clear();
        if present.is_empty() {
            self.cell_start.clear();
            self.cell_start.extend([0, 0]);
            self.dims = (1, 1);
            self.inv_cell = 1.0;
            return;
        }

        let (mut min, mut max) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
        self.scratch_f64.clear();
        for &i in present {
            let b = bounds[i as usize].unwrap();
            min = (min.0.min(b.min.0), min.1.min(b.min.1));
            max = (max.0.max(b.max.0), max.1.max(b.max.1));
            self.scratch_f64
                .push((b.max.0 - b.min.0).max(b.max.1 - b.min.1));
        }
        let mid = self.scratch_f64.len() / 2;
        let median = *self
            .scratch_f64
            .select_nth_unstable_by(mid, f64::total_cmp)
            .1;
        let span = (max.0 - min.0, max.1 - min.1);
        let mut cell = (2.0 * median).max(1.0);
        let longest = span.0.max(span.1);
        if longest / cell >= MAX_DIM as f64 {
            cell = longest / (MAX_DIM as f64 - 1.0);
        }
        self.origin = min;
        self.inv_cell = 1.0 / cell;
        self.dims = (
            ((span.0 * self.inv_cell) as usize + 1).min(MAX_DIM),
            ((span.1 * self.inv_cell) as usize + 1).min(MAX_DIM),
        );

        // Pass 1: count items per cell (and pick out large objects).
        let cells = self.dims.0 * self.dims.1;
        self.cell_start.clear();
        self.cell_start.resize(cells + 1, 0);
        for &i in present {
            let r = self.cell_range(&bounds[i as usize].unwrap());
            let covered = (r[2] - r[0] + 1) as usize * (r[3] - r[1] + 1) as usize;
            if covered > MAX_CELLS_PER_OBJECT {
                self.is_large[i as usize] = true;
                self.large.push(i);
                continue;
            }
            self.ranges[i as usize] = r;
            for y in r[1]..=r[3] {
                for x in r[0]..=r[2] {
                    self.cell_start[y as usize * self.dims.0 + x as usize] += 1;
                }
            }
        }
        // Prefix sums turn the counts into start offsets.
        let mut total = 0u32;
        for c in self.cell_start.iter_mut() {
            let n = *c;
            *c = total;
            total += n;
        }
        // Pass 2: place items.
        self.scratch_u32.clear();
        self.scratch_u32.extend_from_slice(&self.cell_start);
        self.items.resize(total as usize, 0);
        for &i in present {
            if self.is_large[i as usize] {
                continue;
            }
            let r = self.ranges[i as usize];
            for y in r[1]..=r[3] {
                for x in r[0]..=r[2] {
                    let c = y as usize * self.dims.0 + x as usize;
                    self.items[self.scratch_u32[c] as usize] = i;
                    self.scratch_u32[c] += 1;
                }
            }
        }
    }

    /// Cell range covering `b`, clamped to the grid.
    fn cell_range(&self, b: &Bounds) -> [u32; 4] {
        let cell = |v: f64, origin: f64, dim: usize| {
            (((v - origin) * self.inv_cell).floor().max(0.0) as usize).min(dim - 1) as u32
        };
        [
            cell(b.min.0, self.origin.0, self.dims.0),
            cell(b.min.1, self.origin.1, self.dims.1),
            cell(b.max.0, self.origin.0, self.dims.0),
            cell(b.max.1, self.origin.1, self.dims.1),
        ]
    }

    fn cell_items(&self, x: u32, y: u32) -> &[u32] {
        let c = y as usize * self.dims.0 + x as usize;
        &self.items[self.cell_start[c] as usize..self.cell_start[c + 1] as usize]
    }

    /// Visit every object whose cells meet `b`'s cells (each at most once), plus every large
    /// object. Callers still test the exact shapes.
    pub fn query(&self, b: &Bounds, mut visit: impl FnMut(u32)) {
        for &i in &self.large {
            visit(i);
        }
        if self.items.is_empty() {
            return;
        }
        let q = self.cell_range(b);
        for y in q[1]..=q[3] {
            for x in q[0]..=q[2] {
                for &i in self.cell_items(x, y) {
                    let r = &self.ranges[i as usize];
                    // Report only from the reference cell of the two ranges' overlap.
                    if x == r[0].max(q[0]) && y == r[1].max(q[1]) {
                        visit(i);
                    }
                }
            }
        }
    }

    /// Visit every candidate pair `(i, j)` with `i < j` once.
    pub fn pairs(&self, present: &[u32], mut visit: impl FnMut(u32, u32)) {
        for (n, &a) in self.large.iter().enumerate() {
            for &b in present {
                // Large × large pairs once (by order in `large`); large × small always.
                if b == a || (self.is_large[b as usize] && self.large[..=n].contains(&b)) {
                    continue;
                }
                visit(a.min(b), a.max(b));
            }
        }
        for y in 0..self.dims.1 as u32 {
            for x in 0..self.dims.0 as u32 {
                let cell = self.cell_items(x, y);
                for (k, &a) in cell.iter().enumerate() {
                    let ra = &self.ranges[a as usize];
                    for &b in &cell[k + 1..] {
                        let rb = &self.ranges[b as usize];
                        if x == ra[0].max(rb[0]) && y == ra[1].max(rb[1]) {
                            visit(a.min(b), a.max(b));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn b(x0: f64, y0: f64, x1: f64, y1: f64) -> Option<Bounds> {
        Some(Bounds {
            min: (x0, y0),
            max: (x1, y1),
        })
    }

    #[test]
    fn query_and_pairs_find_each_overlap_once() {
        let mut rng = crate::collision::narrow::tests::Lcg(3);
        let mut bounds = Vec::new();
        for i in 0..800 {
            if i % 10 == 0 {
                bounds.push(None); // no collider
                continue;
            }
            let (x, y) = (rng.range(0.0, 1000.0), rng.range(0.0, 600.0));
            // Mostly small, some large (to exercise the large-object list).
            let s = if i % 97 == 0 {
                rng.range(100.0, 400.0)
            } else {
                rng.range(1.0, 20.0)
            };
            bounds.push(b(x, y, x + s, y + s * rng.range(0.2, 1.5)));
        }
        let grid = Grid::build(&bounds);
        let present: Vec<u32> = (0..bounds.len() as u32)
            .filter(|&i| bounds[i as usize].is_some())
            .collect();

        let mut found = Vec::new();
        grid.pairs(&present, |i, j| found.push((i, j)));
        let set: HashSet<_> = found.iter().copied().collect();
        assert_eq!(set.len(), found.len(), "a pair was reported twice");
        for &i in &present {
            for &j in &present {
                if i < j {
                    let overlap = bounds[i as usize]
                        .unwrap()
                        .overlaps(&bounds[j as usize].unwrap());
                    if overlap {
                        assert!(set.contains(&(i, j)), "missed pair {i} {j}");
                    }
                }
            }
        }

        for &i in present.iter().step_by(7) {
            let mut seen = Vec::new();
            grid.query(&bounds[i as usize].unwrap(), |j| seen.push(j));
            let unique: HashSet<_> = seen.iter().copied().collect();
            assert_eq!(unique.len(), seen.len());
            for &j in &present {
                if bounds[i as usize]
                    .unwrap()
                    .overlaps(&bounds[j as usize].unwrap())
                {
                    assert!(unique.contains(&j), "query {i} missed {j}");
                }
            }
        }
        assert!(!grid.large.is_empty());
    }

    #[test]
    fn empty_and_single() {
        let grid = Grid::build(&[None, None]);
        grid.query(&b(0.0, 0.0, 1.0, 1.0).unwrap(), |_| {
            panic!("nothing to visit")
        });
        let grid = Grid::build(&[b(5.0, 5.0, 5.0, 5.0)]);
        let mut seen = vec![];
        grid.query(&b(0.0, 0.0, 5.0, 5.0).unwrap(), |j| seen.push(j));
        assert_eq!(seen, [0]);
        // A query far outside the grid still clamps into it; exact tests reject it later.
        grid.pairs(&[0], |_, _| panic!("no pairs"));
    }
}
