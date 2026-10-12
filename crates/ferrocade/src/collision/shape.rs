//! World-space shapes: a collider placed by its sprite's position, scale, and rotation.

use super::{Collider, Shape};

/// A collider in window coordinates. Axis-aligned boxes are `Obb` with `cos = 1, sin = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldShape {
    Circle {
        c: (f64, f64),
        r: f64,
    },
    Obb {
        c: (f64, f64),
        /// Half width and half height along the box's own axes.
        half: (f64, f64),
        cos: f64,
        sin: f64,
    },
}

/// An axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: (f64, f64),
    pub max: (f64, f64),
}

impl Bounds {
    pub fn overlaps(&self, other: &Bounds) -> bool {
        self.min.0 <= other.max.0
            && other.min.0 <= self.max.0
            && self.min.1 <= other.max.1
            && other.min.1 <= self.max.1
    }
}

/// (cos, sin) of a counter-clockwise angle in degrees, plus the number of quarter turns when
/// the angle is a multiple of 90° (exact values then, so boxes stay axis-aligned).
fn rotation(angle_deg: f64) -> (f64, f64, Option<u8>) {
    let a = angle_deg.rem_euclid(360.0);
    let k = (a / 90.0).round();
    if (a - k * 90.0).abs() < 1e-9 {
        let quarter = (k as i64).rem_euclid(4) as u8;
        let (c, s) = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)][quarter as usize];
        (c, s, Some(quarter))
    } else {
        let (sin, cos) = a.to_radians().sin_cos();
        (cos, sin, None)
    }
}

/// Place `collider` on a sprite at `center` with `scale` and `angle_deg` (counter-clockwise).
/// Sizes and offsets scale with |scale|; circle and box offsets rotate with the sprite, while
/// axis-aligned boxes and their offsets don't rotate.
pub fn world_shape(
    collider: &Collider,
    center: (f64, f64),
    scale: f64,
    angle_deg: f64,
) -> WorldShape {
    let k = scale.abs();
    let (ox, oy) = (collider.offset.0 * k, collider.offset.1 * k);
    let centered = ox == 0.0 && oy == 0.0;
    // Only rotating boxes and offset circles need the rotation (skips trigonometry otherwise).
    let needs_rotation = match collider.shape {
        Shape::Box { .. } => true,
        Shape::Circle { .. } => !centered,
        Shape::Aabb { .. } => false,
    };
    let (cos, sin, quarter) = if needs_rotation {
        rotation(angle_deg)
    } else {
        (1.0, 0.0, Some(0))
    };
    let rotated = if centered {
        center
    } else {
        (
            center.0 + ox * cos - oy * sin,
            center.1 + ox * sin + oy * cos,
        )
    };
    match collider.shape {
        Shape::Circle { radius } => WorldShape::Circle {
            c: rotated,
            r: radius * k,
        },
        Shape::Box { width, height } => {
            let half = (width * k / 2.0, height * k / 2.0);
            match quarter {
                // At a multiple of 90° the box is axis-aligned: use the fast form.
                Some(q) => WorldShape::Obb {
                    c: rotated,
                    half: if q % 2 == 1 { (half.1, half.0) } else { half },
                    cos: 1.0,
                    sin: 0.0,
                },
                None => WorldShape::Obb {
                    c: rotated,
                    half,
                    cos,
                    sin,
                },
            }
        }
        Shape::Aabb { width, height } => WorldShape::Obb {
            c: (center.0 + ox, center.1 + oy),
            half: (width * k / 2.0, height * k / 2.0),
            cos: 1.0,
            sin: 0.0,
        },
    }
}

pub fn bounds(shape: &WorldShape) -> Bounds {
    match *shape {
        WorldShape::Circle { c, r } => Bounds {
            min: (c.0 - r, c.1 - r),
            max: (c.0 + r, c.1 + r),
        },
        WorldShape::Obb { c, half, cos, sin } => {
            let ex = half.0 * cos.abs() + half.1 * sin.abs();
            let ey = half.0 * sin.abs() + half.1 * cos.abs();
            Bounds {
                min: (c.0 - ex, c.1 - ey),
                max: (c.0 + ex, c.1 + ey),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collider(shape: Shape, offset: (f64, f64)) -> Collider {
        Collider {
            shape,
            offset,
            attached_to: None,
        }
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn scale_multiplies_sizes() {
        let c = collider(Shape::Circle { radius: 10.0 }, (0.0, 0.0));
        assert_eq!(
            world_shape(&c, (5.0, 5.0), 2.0, 0.0),
            WorldShape::Circle {
                c: (5.0, 5.0),
                r: 20.0
            }
        );
        // Negative scale (mirroring) still uses the magnitude.
        assert!(matches!(
            world_shape(&c, (0.0, 0.0), -2.0, 0.0),
            WorldShape::Circle { r, .. } if r == 20.0
        ));
    }

    #[test]
    fn offsets_rotate_except_for_aabb() {
        let circle = collider(Shape::Circle { radius: 1.0 }, (20.0, 0.0));
        match world_shape(&circle, (100.0, 100.0), 1.0, 90.0) {
            WorldShape::Circle { c, .. } => assert!(close(c, (100.0, 120.0)), "{c:?}"),
            other => panic!("{other:?}"),
        }
        let aabb = collider(
            Shape::Aabb {
                width: 4.0,
                height: 2.0,
            },
            (20.0, 0.0),
        );
        match world_shape(&aabb, (100.0, 100.0), 1.0, 90.0) {
            WorldShape::Obb { c, half, cos, sin } => {
                assert!(close(c, (120.0, 100.0)));
                assert_eq!((half, cos, sin), ((2.0, 1.0), 1.0, 0.0));
            }
            other => panic!("{other:?}"),
        }
        // Scaled offset: scale 2 doubles it.
        match world_shape(&circle, (0.0, 0.0), 2.0, 0.0) {
            WorldShape::Circle { c, .. } => assert!(close(c, (40.0, 0.0))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn boxes_rotate_and_snap_to_axis_aligned_at_quarter_turns() {
        let b = collider(
            Shape::Box {
                width: 40.0,
                height: 10.0,
            },
            (0.0, 0.0),
        );
        match world_shape(&b, (0.0, 0.0), 1.0, 90.0) {
            WorldShape::Obb { half, cos, sin, .. } => {
                assert_eq!((half, cos, sin), ((5.0, 20.0), 1.0, 0.0))
            }
            other => panic!("{other:?}"),
        }
        match world_shape(&b, (0.0, 0.0), 1.0, -180.0) {
            WorldShape::Obb { half, cos, sin, .. } => {
                assert_eq!((half, cos, sin), ((20.0, 5.0), 1.0, 0.0))
            }
            other => panic!("{other:?}"),
        }
        match world_shape(&b, (0.0, 0.0), 1.0, 45.0) {
            WorldShape::Obb { cos, sin, .. } => {
                assert!((cos - sin).abs() < 1e-12 && cos > 0.7);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bounds_cover_rotated_boxes() {
        let s = WorldShape::Obb {
            c: (0.0, 0.0),
            half: (10.0, 0.0),
            cos: std::f64::consts::FRAC_1_SQRT_2,
            sin: std::f64::consts::FRAC_1_SQRT_2,
        };
        let b = bounds(&s);
        let e = 10.0 * std::f64::consts::FRAC_1_SQRT_2;
        assert!(close(b.max, (e, e)) && close(b.min, (-e, -e)));
    }
}
