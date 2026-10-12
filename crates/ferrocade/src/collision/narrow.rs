//! Exact overlap tests between world shapes. Touching counts as overlapping.

use super::shape::WorldShape;

pub fn overlaps(a: &WorldShape, b: &WorldShape) -> bool {
    use WorldShape::*;
    match (*a, *b) {
        (Circle { c: c1, r: r1 }, Circle { c: c2, r: r2 }) => {
            let (dx, dy) = (c2.0 - c1.0, c2.1 - c1.1);
            dx * dx + dy * dy <= (r1 + r2) * (r1 + r2)
        }
        (Circle { c, r }, Obb { .. }) => circle_box(c, r, b),
        (Obb { .. }, Circle { c, r }) => circle_box(c, r, a),
        (Obb { .. }, Obb { .. }) => box_box(a, b),
    }
}

/// Circle vs. box: move the circle center into the box's frame and clamp it to the box.
fn circle_box(center: (f64, f64), r: f64, b: &WorldShape) -> bool {
    let WorldShape::Obb { c, half, cos, sin } = *b else {
        unreachable!("circle_box needs a box")
    };
    let (dx, dy) = (center.0 - c.0, center.1 - c.1);
    let (lx, ly) = (dx * cos + dy * sin, -dx * sin + dy * cos);
    let (qx, qy) = (
        lx - lx.clamp(-half.0, half.0),
        ly - ly.clamp(-half.1, half.1),
    );
    qx * qx + qy * qy <= r * r
}

/// Box vs. box: separating axis test on both boxes' axes.
fn box_box(a: &WorldShape, b: &WorldShape) -> bool {
    let (
        WorldShape::Obb {
            c: ca,
            half: ha,
            cos: ac,
            sin: as_,
        },
        WorldShape::Obb {
            c: cb,
            half: hb,
            cos: bc,
            sin: bs,
        },
    ) = (*a, *b)
    else {
        unreachable!("box_box needs two boxes")
    };
    let d = (cb.0 - ca.0, cb.1 - ca.1);
    let a_axes = [(ac, as_), (-as_, ac)];
    let b_axes = [(bc, bs), (-bs, bc)];
    let aligned = as_ == 0.0 && bs == 0.0 && ac == bc;
    let dot = |p: (f64, f64), q: (f64, f64)| p.0 * q.0 + p.1 * q.1;
    let radius = |half: (f64, f64), axes: &[(f64, f64); 2], axis: (f64, f64)| {
        half.0 * dot(axes[0], axis).abs() + half.1 * dot(axes[1], axis).abs()
    };
    let axes: &[(f64, f64)] = if aligned {
        &a_axes
    } else {
        &[a_axes[0], a_axes[1], b_axes[0], b_axes[1]]
    };
    axes.iter()
        .all(|&axis| dot(d, axis).abs() <= radius(ha, &a_axes, axis) + radius(hb, &b_axes, axis))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::collision::shape::bounds;

    pub fn circle(x: f64, y: f64, r: f64) -> WorldShape {
        WorldShape::Circle { c: (x, y), r }
    }

    pub fn obb(x: f64, y: f64, w: f64, h: f64, angle_deg: f64) -> WorldShape {
        let a = angle_deg.to_radians();
        WorldShape::Obb {
            c: (x, y),
            half: (w / 2.0, h / 2.0),
            cos: a.cos(),
            sin: a.sin(),
        }
    }

    pub fn aabb(x: f64, y: f64, w: f64, h: f64) -> WorldShape {
        WorldShape::Obb {
            c: (x, y),
            half: (w / 2.0, h / 2.0),
            cos: 1.0,
            sin: 0.0,
        }
    }

    #[test]
    fn circles_touching_counts() {
        // Spec US1 scenario 1: radius 10 at x=100 vs. radius 10 at x=119 / 121.
        assert!(overlaps(
            &circle(100.0, 100.0, 10.0),
            &circle(119.0, 100.0, 10.0)
        ));
        assert!(!overlaps(
            &circle(100.0, 100.0, 10.0),
            &circle(121.0, 100.0, 10.0)
        ));
        assert!(overlaps(&circle(0.0, 0.0, 1.0), &circle(2.0, 0.0, 1.0)));
        assert!(!overlaps(
            &circle(0.0, 0.0, 1.0),
            &circle(2.0 + 1e-9, 0.0, 1.0)
        ));
    }

    #[test]
    fn circle_and_boxes() {
        let b = aabb(0.0, 0.0, 20.0, 10.0);
        assert!(overlaps(&circle(15.0, 0.0, 5.0), &b)); // touches the right face
        assert!(!overlaps(&circle(15.0 + 1e-9, 0.0, 5.0), &b));
        assert!(overlaps(&b, &circle(13.0, 8.0, 5.0))); // near the corner (3, 3) away: 4.24 < 5
        assert!(!overlaps(&b, &circle(14.0, 9.0, 5.0))); // (4, 4) away: 5.66 > 5
        // A box rotated 90° swaps its extents.
        let r = obb(0.0, 0.0, 20.0, 10.0, 90.0);
        assert!(overlaps(&circle(0.0, 14.0, 4.0), &r));
        assert!(!overlaps(&circle(14.0, 0.0, 4.0), &r));
    }

    #[test]
    fn boxes() {
        let a = aabb(0.0, 0.0, 10.0, 10.0);
        assert!(overlaps(&a, &aabb(10.0, 0.0, 10.0, 10.0))); // edges touch
        assert!(!overlaps(&a, &aabb(10.0 + 1e-9, 0.0, 10.0, 10.0)));
        // A 45° square reaches √2·5 ≈ 7.07 along x.
        let diamond = obb(0.0, 0.0, 10.0, 10.0, 45.0);
        assert!(overlaps(&diamond, &aabb(11.0, 0.0, 8.0, 8.0))); // gap 7.07 + 4 > 11
        assert!(!overlaps(&diamond, &aabb(11.5, 0.0, 8.0, 8.0)));
        // Corner-to-corner near miss that only the rotated box's axes separate.
        assert!(!overlaps(&diamond, &aabb(9.0, 9.0, 8.0, 8.0)));
        assert!(overlaps(&obb(0.0, 0.0, 10.0, 10.0, 180.0), &a));
    }

    #[test]
    fn zero_size_shapes() {
        let point = circle(5.0, 5.0, 0.0);
        assert!(overlaps(&point, &aabb(0.0, 0.0, 10.0, 10.0))); // on the corner
        assert!(!overlaps(&point, &aabb(0.0, 0.0, 9.0, 9.0)));
        let line = aabb(0.0, 0.0, 10.0, 0.0);
        assert!(overlaps(&line, &circle(0.0, 1.0, 1.0)));
    }

    fn contains(shape: &WorldShape, p: (f64, f64)) -> bool {
        overlaps(shape, &circle(p.0, p.1, 0.0))
    }

    /// Shrink a shape by `d` on every side; `None` if nothing is left. A point in both shrunk
    /// shapes has a disk of radius `d` around it inside both originals, which always contains a
    /// sample point of step `d`.
    fn deflate(shape: &WorldShape, d: f64) -> Option<WorldShape> {
        match *shape {
            WorldShape::Circle { c, r } => (r >= d).then_some(WorldShape::Circle { c, r: r - d }),
            WorldShape::Obb { c, half, cos, sin } => {
                (half.0 >= d && half.1 >= d).then_some(WorldShape::Obb {
                    c,
                    half: (half.0 - d, half.1 - d),
                    cos,
                    sin,
                })
            }
        }
    }

    /// A tiny deterministic generator for reproducible random tests.
    pub struct Lcg(pub u64);
    impl Lcg {
        pub fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.next()
        }
    }

    pub fn random_shape(rng: &mut Lcg, area: f64, max_size: f64) -> WorldShape {
        let (x, y) = (rng.range(0.0, area), rng.range(0.0, area));
        match (rng.next() * 3.0) as u32 {
            0 => circle(x, y, rng.range(0.0, max_size / 2.0)),
            1 => aabb(x, y, rng.range(0.0, max_size), rng.range(0.0, max_size)),
            _ => obb(
                x,
                y,
                rng.range(0.0, max_size),
                rng.range(0.0, max_size),
                rng.range(0.0, 360.0),
            ),
        }
    }

    /// Compare against point sampling: a sampled point inside both shapes proves an overlap;
    /// if sampling finds none, any reported overlap must be shallower than the sample step.
    #[test]
    fn agrees_with_point_sampling() {
        let mut rng = Lcg(7);
        let h = 0.5;
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..600 {
            let a = random_shape(&mut rng, 60.0, 40.0);
            let b = random_shape(&mut rng, 60.0, 40.0);
            let (ba, bb) = (bounds(&a), bounds(&b));
            let lo = (ba.min.0.max(bb.min.0), ba.min.1.max(bb.min.1));
            let hi = (ba.max.0.min(bb.max.0), ba.max.1.min(bb.max.1));
            let mut sampled = false;
            let mut x = (lo.0 / h).floor() * h;
            'outer: while x <= hi.0 {
                let mut y = (lo.1 / h).floor() * h;
                while y <= hi.1 {
                    if contains(&a, (x, y)) && contains(&b, (x, y)) {
                        sampled = true;
                        break 'outer;
                    }
                    y += h;
                }
                x += h;
            }
            let result = overlaps(&a, &b);
            if sampled {
                assert!(result, "sampling found an overlap: {a:?} {b:?}");
                hits += 1;
            } else {
                if result && let (Some(da), Some(db)) = (deflate(&a, h), deflate(&b, h)) {
                    assert!(
                        !overlaps(&da, &db),
                        "deep overlap missed by sampling? {a:?} {b:?}"
                    );
                }
                misses += 1;
            }
            assert_eq!(result, overlaps(&b, &a), "symmetry: {a:?} {b:?}");
        }
        assert!(hits > 100 && misses > 100, "{hits} hits, {misses} misses");
    }
}
