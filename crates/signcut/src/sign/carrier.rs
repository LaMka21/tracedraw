//! Carrier (backing board): one outline around letters.
//!
//! Each object's closed outlines (holes by the even-odd rule) are offset
//! outwards by the distance plus the rounding with round corners, all of
//! them welded together, then offset back inwards by the rounding: the
//! inner corners and the narrow gaps between letters get that radius
//! (a gap narrower than twice the distance plus the rounding closes).
//! Optionally the holes are filled: only the outer outlines stay.
//! Letters far apart give separate pieces.

use super::{outer_contours, region};
use tracedraw_core::geometry::{BezPath, Shape as _};
use tracedraw_core::shaping::{self, Join, Op};

/// The carrier settings (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarrierSpec {
    pub distance: f64,
    pub rounding: f64,
    pub fill_holes: bool,
}

impl Default for CarrierSpec {
    fn default() -> Self {
        CarrierSpec {
            distance: 10.0,
            rounding: 0.0,
            fill_holes: true,
        }
    }
}

/// The carrier's outline around the objects (their outlines in page
/// space). Empty when none of them has a closed outline.
pub fn carrier(objects: &[Vec<BezPath>], spec: &CarrierSpec) -> BezPath {
    let clean = |v: f64| if v.is_finite() { v.max(0.0) } else { 0.0 };
    let d = clean(spec.distance);
    let r = clean(spec.rounding);
    let mut acc: Option<BezPath> = None;
    for paths in objects {
        let reg = region(paths);
        if reg.elements().is_empty() {
            continue;
        }
        let grown = if d + r > 1e-9 {
            shaping::offset_with(&reg, d + r, Join::Round)
        } else {
            // Only simplified: welding needs clean outlines.
            shaping::overlay(&reg, &BezPath::new(), Op::Weld)
        };
        if grown.elements().is_empty() {
            continue;
        }
        acc = Some(match acc {
            None => grown,
            Some(a) => shaping::overlay(&a, &grown, Op::Weld),
        });
    }
    let Some(mut out) = acc else {
        return BezPath::new();
    };
    if r > 1e-9 {
        out = shaping::offset_with(&out, -r, Join::Round);
    }
    if spec.fill_holes {
        out = outer_contours(&out);
    }
    if out.area().abs() < 1e-9 {
        return BezPath::new();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    fn square(x: f64, y: f64, s: f64) -> BezPath {
        Rect::new(x, y, x + s, y + s).to_path(0.01)
    }

    fn pieces(p: &BezPath) -> usize {
        crate::subpaths(p).len()
    }

    #[test]
    fn close_letters_get_one_outline() {
        // Two squares 15 apart, distance 10: the offsets overlap.
        let c = carrier(
            &[vec![square(0.0, 0.0, 50.0)], vec![square(65.0, 0.0, 50.0)]],
            &CarrierSpec::default(),
        );
        assert_eq!(pieces(&c), 1);
        let b = c.bounding_box();
        assert!((b.width() - 135.0).abs() < 0.05 && (b.height() - 70.0).abs() < 0.05);
    }

    #[test]
    fn far_letters_stay_apart_unless_rounding_closes_the_gap() {
        let objs = [vec![square(0.0, 0.0, 50.0)], vec![square(80.0, 0.0, 50.0)]];
        let c = carrier(&objs, &CarrierSpec::default());
        assert_eq!(pieces(&c), 2);
        let c = carrier(
            &objs,
            &CarrierSpec {
                rounding: 10.0,
                ..Default::default()
            },
        );
        assert_eq!(pieces(&c), 1);
    }

    #[test]
    fn holes_are_filled_or_kept() {
        // A ring 200 wide with a 160 hole: offset 10 leaves a 140 hole.
        let mut o = square(0.0, 0.0, 200.0);
        o.extend(square(20.0, 20.0, 160.0));
        let filled = carrier(&[vec![o.clone()]], &CarrierSpec::default());
        assert_eq!(pieces(&filled), 1);
        let kept = carrier(
            &[vec![o]],
            &CarrierSpec {
                fill_holes: false,
                ..Default::default()
            },
        );
        assert_eq!(pieces(&kept), 2);
        assert!(kept.area().abs() < filled.area().abs());
    }

    #[test]
    fn nothing_closed_makes_no_carrier() {
        let mut open = BezPath::new();
        open.move_to((0.0, 0.0));
        open.line_to((10.0, 0.0));
        assert!(carrier(&[vec![open]], &CarrierSpec::default())
            .elements()
            .is_empty());
        assert!(carrier(&[], &CarrierSpec::default()).elements().is_empty());
        let odd = CarrierSpec {
            distance: f64::NAN,
            rounding: f64::INFINITY,
            fill_holes: true,
        };
        let c = carrier(&[vec![square(0.0, 0.0, 10.0)]], &odd);
        assert!((c.area().abs() - 100.0).abs() < 1e-6);
    }
}
