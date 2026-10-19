//! Cast shadow: the transform that turns an object's outline into its
//! flat shadow.
//!
//! - On a wall: the outline moved by a distance in a direction (the
//!   object stands off a wall behind it).
//! - On the floor: the outline flattened to a fraction of its height and
//!   slanted, standing on the object's bottom edge; it falls in front
//!   (below the bottom edge) or behind (above it, mostly hidden by the
//!   object).

use tracedraw_core::geometry::{Affine, Rect, Vec2};

/// Steepest floor slant, degrees either way.
pub const MAX_SKEW: f64 = 80.0;
/// Range of the floor's height factor.
pub const MIN_SCALE: f64 = 0.01;
pub const MAX_SCALE: f64 = 5.0;

/// Where the shadow falls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowKind {
    #[default]
    Wall,
    Floor,
}

/// The settings of the shadow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowSpec {
    pub kind: ShadowKind,
    /// Wall: the direction of the move, degrees counter-clockwise from x.
    pub angle: f64,
    /// Wall: the length of the move (mm).
    pub distance: f64,
    /// Floor: the shadow's height as a fraction of the object's (0.3).
    pub scale: f64,
    /// Floor: the slant, degrees from upright; positive leans right.
    pub skew: f64,
    /// Floor: the shadow falls in front, below the bottom edge (else
    /// behind, above it).
    pub in_front: bool,
}

impl Default for ShadowSpec {
    fn default() -> Self {
        ShadowSpec {
            kind: ShadowKind::Wall,
            angle: -45.0,
            distance: 3.0,
            scale: 0.3,
            skew: 45.0,
            in_front: true,
        }
    }
}

/// The page-space transform from an object with page bounds `bounds` to
/// its shadow; `None` for settings or bounds that are not finite.
pub fn shadow_transform(bounds: Rect, spec: &ShadowSpec) -> Option<Affine> {
    if ![bounds.x0, bounds.y0, bounds.x1, bounds.y1]
        .iter()
        .all(|v| v.is_finite())
    {
        return None;
    }
    let b = bounds.abs();
    match spec.kind {
        ShadowKind::Wall => {
            if !(spec.angle.is_finite() && spec.distance.is_finite()) {
                return None;
            }
            let a = spec.angle.to_radians();
            Some(Affine::translate(
                Vec2::new(a.cos(), a.sin()) * spec.distance,
            ))
        }
        ShadowKind::Floor => {
            if !(spec.scale.is_finite() && spec.skew.is_finite()) {
                return None;
            }
            let k = spec.scale.clamp(MIN_SCALE, MAX_SCALE);
            let lean = spec.skew.clamp(-MAX_SKEW, MAX_SKEW).to_radians().tan();
            let y0 = b.y0;
            // A point h above the bottom edge goes h * k from it (down in
            // front, up behind) and h * k * tan(skew) sideways.
            let d = if spec.in_front { -k } else { k };
            let c = k * lean;
            Some(Affine::new([1.0, 0.0, c, d, -c * y0, y0 - d * y0]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Point;

    #[test]
    fn a_wall_shadow_moves_by_the_distance() {
        let spec = ShadowSpec {
            angle: -90.0,
            distance: 5.0,
            ..Default::default()
        };
        let t = shadow_transform(Rect::new(0.0, 0.0, 10.0, 10.0), &spec).expect("finite");
        let p = t * Point::new(3.0, 4.0);
        assert!((p.x - 3.0).abs() < 1e-9 && (p.y + 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_floor_shadow_stands_on_the_bottom_edge() {
        let spec = ShadowSpec {
            kind: ShadowKind::Floor,
            scale: 0.5,
            skew: 45.0,
            in_front: true,
            ..Default::default()
        };
        let r = Rect::new(10.0, 20.0, 30.0, 60.0);
        let t = shadow_transform(r, &spec).expect("finite");
        // The bottom edge stays.
        let p = t * Point::new(10.0, 20.0);
        assert!((p - Point::new(10.0, 20.0)).hypot() < 1e-9);
        // The top (40 up) goes 20 down and 20 right.
        let q = t * Point::new(10.0, 60.0);
        assert!((q - Point::new(30.0, 0.0)).hypot() < 1e-9, "{q:?}");
        let behind = ShadowSpec {
            in_front: false,
            ..spec
        };
        let q = shadow_transform(r, &behind).expect("finite") * Point::new(10.0, 60.0);
        assert!((q - Point::new(30.0, 40.0)).hypot() < 1e-9, "{q:?}");
    }

    #[test]
    fn extreme_and_bad_settings() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let floor = ShadowSpec {
            kind: ShadowKind::Floor,
            skew: 90.0,
            scale: 100.0,
            ..Default::default()
        };
        let t = shadow_transform(r, &floor).expect("clamped");
        assert!(t.as_coeffs().iter().all(|c| c.is_finite()));
        for spec in [
            ShadowSpec {
                distance: f64::NAN,
                ..Default::default()
            },
            ShadowSpec {
                kind: ShadowKind::Floor,
                skew: f64::INFINITY,
                ..Default::default()
            },
        ] {
            assert!(shadow_transform(r, &spec).is_none());
        }
        assert!(
            shadow_transform(Rect::new(0.0, f64::NAN, 1.0, 1.0), &ShadowSpec::default()).is_none()
        );
    }
}
