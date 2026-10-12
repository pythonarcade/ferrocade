//! Collision detection: colliders attached to sprites, exact shape tests, and batched queries.
//!
//! Colliders are separate entities (owner decision, engine spec §11): each lives in the store
//! under its own `ColliderId` and is attached to at most one sprite, whose position, scale, and
//! rotation it follows. Sizes and offsets are in the sprite's unscaled pixels. Queries compute
//! world shapes from sprite state, use a per-list grid cached by the list's content version, and
//! return all results at once. Detection only: nothing is moved.

pub mod bounce;
pub mod grid;
pub mod narrow;
pub mod query;
pub mod shape;

use crate::store::SpriteId;
use crate::{Error, Result};

slotmap::new_key_type! {
    pub struct ColliderId;
}

/// A collider's shape, in the sprite's unscaled pixels. New shapes are added as variants (with
/// world-transform, bounds, and narrow-phase arms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Circle {
        radius: f64,
    },
    /// A box that rotates with its sprite.
    Box {
        width: f64,
        height: f64,
    },
    /// An axis-aligned box: never rotates, and neither does its offset.
    Aabb {
        width: f64,
        height: f64,
    },
}

#[derive(Clone, Debug)]
pub struct Collider {
    pub shape: Shape,
    /// Center relative to the sprite center, in unscaled sprite pixels.
    pub offset: (f64, f64),
    pub(crate) attached_to: Option<SpriteId>,
}

impl Collider {
    pub fn attached_to(&self) -> Option<SpriteId> {
        self.attached_to
    }
}

/// Check sizes (finite, ≥ 0; zero is a point or line) and the offset (finite).
pub fn validate(shape: &Shape, offset: (f64, f64)) -> Result<()> {
    let sizes: &[(&str, f64)] = match shape {
        Shape::Circle { radius } => &[("radius", *radius)],
        Shape::Box { width, height } | Shape::Aabb { width, height } => {
            &[("width", *width), ("height", *height)]
        }
    };
    for &(name, v) in sizes {
        if !(v.is_finite() && v >= 0.0) {
            return Err(Error::InvalidArgument(format!(
                "collider {name} must be a finite number >= 0, got {v}"
            )));
        }
    }
    for (name, v) in [("x", offset.0), ("y", offset.1)] {
        if !v.is_finite() {
            return Err(Error::InvalidArgument(format!(
                "collider offset {name} must be a finite number, got {v}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation() {
        assert!(validate(&Shape::Circle { radius: 0.0 }, (0.0, 0.0)).is_ok());
        assert!(validate(&Shape::Circle { radius: -1.0 }, (0.0, 0.0)).is_err());
        assert!(
            validate(
                &Shape::Box {
                    width: f64::NAN,
                    height: 1.0
                },
                (0.0, 0.0)
            )
            .is_err()
        );
        assert!(
            validate(
                &Shape::Aabb {
                    width: 1.0,
                    height: f64::INFINITY
                },
                (0.0, 0.0)
            )
            .is_err()
        );
        assert!(validate(&Shape::Circle { radius: 1.0 }, (f64::NAN, 0.0)).is_err());
    }
}
