//! The flat pattern of a folded sign box (cassette, light box).
//!
//! The face W x H with a flange of the box depth on each side, folded
//! back along the face's edges. The corners are notched out: each flange
//! is the face's side long less a corner gap at each end, so the folded
//! flanges do not touch. An optional return lip folds once more along
//! each flange's outer edge; its ends are cut at 45 degrees (it is the
//! lip length shorter at each end) so the lips can fold in at the
//! corners. Optional round relief holes at the four face corners stop
//! the material tearing where two folds meet. Optional fixing holes sit
//! in the middle of each flange's depth, evenly spread at most the hole
//! spacing apart. No bend allowance: the sizes are the flat sizes.

use tracedraw_core::geometry::{BezPath, Point, Rect};
use tracedraw_core::shaping::{self, Op};

/// The box settings (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxSpec {
    /// Depth of the box: the flanges' width.
    pub depth: f64,
    /// Return lip on each flange (0: none).
    pub lip: f64,
    /// Corner gap at each end of a flange.
    pub corner_gap: f64,
    /// Diameter of the relief holes at the face corners (0: none).
    pub relief: f64,
    /// Fixing holes in the flanges.
    pub holes: bool,
    pub hole_spacing: f64,
    pub hole_diameter: f64,
}

impl Default for BoxSpec {
    fn default() -> Self {
        BoxSpec {
            depth: 50.0,
            lip: 0.0,
            corner_gap: 0.5,
            relief: 0.0,
            holes: false,
            hole_spacing: 300.0,
            hole_diameter: 5.0,
        }
    }
}

/// The flat pattern: the cut outline, the fold lines and the holes.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPattern {
    pub outline: BezPath,
    pub folds: Vec<(Point, Point)>,
    pub holes: Vec<Point>,
    pub hole_diameter: f64,
    /// The face, where the pattern was built around.
    pub face: Rect,
}

/// The pattern of a box whose face is `face`. `None` when the sizes do
/// not make a box (a side or the depth zero, or not finite).
pub fn sign_box(face: Rect, spec: &BoxSpec) -> Option<BoxPattern> {
    let f = face.abs();
    let fin = |v: f64| v.is_finite();
    if !(fin(f.x0) && fin(f.y0) && fin(f.x1) && fin(f.y1)) {
        return None;
    }
    let (w, h) = (f.width(), f.height());
    let d = spec.depth;
    if !(w > 1e-6 && h > 1e-6 && fin(d) && d > 1e-6) || w.max(h).max(d) > 1e7 {
        return None;
    }
    let clean = |v: f64| if fin(v) { v.max(0.0) } else { 0.0 };
    let side = w.min(h);
    let g = clean(spec.corner_gap).min(side / 4.0);
    // The lip may take at most half of the shortest flange each side.
    let lip = clean(spec.lip).min(((side - 2.0 * g) / 2.0) * 0.99);
    let (x0, y0, x1, y1) = (f.x0, f.y0, f.x1, f.y1);
    let p = Point::new;
    let mut pts: Vec<Point> = Vec::new();
    // Bottom flange, left to right.
    pts.push(p(x0, y0));
    pts.push(p(x0 + g, y0));
    pts.push(p(x0 + g, y0 - d));
    if lip > 0.0 {
        pts.push(p(x0 + g + lip, y0 - d - lip));
        pts.push(p(x1 - g - lip, y0 - d - lip));
    }
    pts.push(p(x1 - g, y0 - d));
    pts.push(p(x1 - g, y0));
    pts.push(p(x1, y0));
    // Right flange, bottom to top.
    pts.push(p(x1, y0 + g));
    pts.push(p(x1 + d, y0 + g));
    if lip > 0.0 {
        pts.push(p(x1 + d + lip, y0 + g + lip));
        pts.push(p(x1 + d + lip, y1 - g - lip));
    }
    pts.push(p(x1 + d, y1 - g));
    pts.push(p(x1, y1 - g));
    pts.push(p(x1, y1));
    // Top flange, right to left.
    pts.push(p(x1 - g, y1));
    pts.push(p(x1 - g, y1 + d));
    if lip > 0.0 {
        pts.push(p(x1 - g - lip, y1 + d + lip));
        pts.push(p(x0 + g + lip, y1 + d + lip));
    }
    pts.push(p(x0 + g, y1 + d));
    pts.push(p(x0 + g, y1));
    pts.push(p(x0, y1));
    // Left flange, top to bottom.
    pts.push(p(x0, y1 - g));
    pts.push(p(x0 - d, y1 - g));
    if lip > 0.0 {
        pts.push(p(x0 - d - lip, y1 - g - lip));
        pts.push(p(x0 - d - lip, y0 + g + lip));
    }
    pts.push(p(x0 - d, y0 + g));
    pts.push(p(x0, y0 + g));
    // Drop repeated points (a zero gap).
    pts.dedup_by(|a, b| (*a - *b).hypot() < 1e-9);
    if pts.len() > 1 && (pts[0] - pts[pts.len() - 1]).hypot() < 1e-9 {
        pts.pop();
    }
    let mut outline = tracedraw_core::geometry::polyline_path(&pts, true);
    let relief = clean(spec.relief).min(side / 2.0);
    if relief > 1e-6 {
        let mut holes = BezPath::new();
        for c in [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)] {
            holes.extend(super::circle(c, relief).elements().iter().copied());
        }
        outline = shaping::overlay(&outline, &holes, Op::Trim);
    }
    let mut folds = vec![
        (p(x0 + g, y0), p(x1 - g, y0)),
        (p(x1, y0 + g), p(x1, y1 - g)),
        (p(x1 - g, y1), p(x0 + g, y1)),
        (p(x0, y1 - g), p(x0, y0 + g)),
    ];
    if lip > 0.0 {
        folds.extend([
            (p(x0 + g, y0 - d), p(x1 - g, y0 - d)),
            (p(x1 + d, y0 + g), p(x1 + d, y1 - g)),
            (p(x1 - g, y1 + d), p(x0 + g, y1 + d)),
            (p(x0 - d, y1 - g), p(x0 - d, y0 + g)),
        ]);
    }
    let mut holes = Vec::new();
    let spacing = spec.hole_spacing;
    if spec.holes && fin(spacing) && spacing > 1e-6 {
        // Each flange: along its length, at half the depth.
        let flanges = [
            (p(x0 + g, y0 - d / 2.0), p(x1 - g, y0 - d / 2.0)),
            (p(x1 + d / 2.0, y0 + g), p(x1 + d / 2.0, y1 - g)),
            (p(x1 - g, y1 + d / 2.0), p(x0 + g, y1 + d / 2.0)),
            (p(x0 - d / 2.0, y1 - g), p(x0 - d / 2.0, y0 + g)),
        ];
        for (a, b) in flanges {
            let len = (b - a).hypot();
            let n = (len / spacing - 1e-9).ceil().max(1.0);
            if n > 10_000.0 {
                continue;
            }
            let n = n as usize;
            for k in 0..n {
                let t = (k as f64 + 0.5) / n as f64;
                holes.push(a + (b - a) * t);
            }
        }
    }
    Some(BoxPattern {
        outline,
        folds,
        holes,
        hole_diameter: clean(spec.hole_diameter),
        face: f,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;

    #[test]
    fn a_box_has_a_cross_shaped_pattern() {
        let face = Rect::new(0.0, 0.0, 1000.0, 500.0);
        let spec = BoxSpec {
            corner_gap: 0.0,
            ..Default::default()
        };
        let b = sign_box(face, &spec).expect("a box");
        let bb = b.outline.bounding_box();
        assert!((bb.width() - 1100.0).abs() < 1e-9);
        assert!((bb.height() - 600.0).abs() < 1e-9);
        // Face plus four flanges, the corner squares cut away.
        let area = 1000.0 * 500.0 + 2.0 * 1000.0 * 50.0 + 2.0 * 500.0 * 50.0;
        assert!((b.outline.area().abs() - area).abs() < 1e-6);
        assert_eq!(b.folds.len(), 4);
        assert!(b.holes.is_empty());
    }

    #[test]
    fn corner_gaps_and_lips_shorten_the_flanges() {
        let face = Rect::new(0.0, 0.0, 400.0, 200.0);
        let spec = BoxSpec {
            depth: 40.0,
            lip: 10.0,
            corner_gap: 1.0,
            ..Default::default()
        };
        let b = sign_box(face, &spec).expect("a box");
        assert_eq!(b.folds.len(), 8);
        let bb = b.outline.bounding_box();
        assert!((bb.width() - (400.0 + 2.0 * 50.0)).abs() < 1e-9);
        // Bottom flange 398 x 40, its lip a trapezoid 398 / 378 x 10.
        let face_a = 400.0 * 200.0;
        let flanges = 2.0 * (398.0 * 40.0) + 2.0 * (198.0 * 40.0);
        let lips = 2.0 * ((398.0 + 378.0) / 2.0 * 10.0) + 2.0 * ((198.0 + 178.0) / 2.0 * 10.0);
        assert!((b.outline.area().abs() - (face_a + flanges + lips)).abs() < 1e-6);
        // The first fold runs along the face's bottom edge.
        let (a, c) = b.folds[0];
        assert!((a.y).abs() < 1e-9 && (c.y).abs() < 1e-9 && (c.x - a.x - 398.0).abs() < 1e-9);
    }

    #[test]
    fn fixing_holes_are_spread_along_each_flange() {
        let face = Rect::new(0.0, 0.0, 1000.0, 500.0);
        let spec = BoxSpec {
            holes: true,
            hole_spacing: 300.0,
            corner_gap: 0.0,
            ..Default::default()
        };
        let b = sign_box(face, &spec).expect("a box");
        // 1000 / 300: 4 holes each long side, 500 / 300: 2 each short side.
        assert_eq!(b.holes.len(), 12);
        assert!(b.holes.iter().any(|h| (h.y + 25.0).abs() < 1e-9));
    }

    #[test]
    fn relief_holes_take_material_at_the_corners() {
        let face = Rect::new(0.0, 0.0, 300.0, 300.0);
        let plain = sign_box(face, &BoxSpec::default()).expect("a box");
        let relieved = sign_box(
            face,
            &BoxSpec {
                relief: 6.0,
                ..Default::default()
            },
        )
        .expect("a box");
        assert!(relieved.outline.area().abs() < plain.outline.area().abs() - 50.0);
    }

    #[test]
    fn degenerate_sizes_make_no_box() {
        let spec = BoxSpec::default();
        assert!(sign_box(Rect::new(0.0, 0.0, 0.0, 100.0), &spec).is_none());
        assert!(sign_box(Rect::new(0.0, 0.0, f64::NAN, 100.0), &spec).is_none());
        let flat = BoxSpec {
            depth: 0.0,
            ..Default::default()
        };
        assert!(sign_box(Rect::new(0.0, 0.0, 10.0, 10.0), &flat).is_none());
        let wild = BoxSpec {
            depth: 5.0,
            lip: f64::INFINITY,
            corner_gap: f64::NAN,
            relief: -3.0,
            holes: true,
            hole_spacing: f64::NAN,
            hole_diameter: f64::NAN,
        };
        let b = sign_box(Rect::new(0.0, 0.0, 10.0, 10.0), &wild).expect("a box");
        assert!(b.holes.is_empty());
        assert_eq!(b.hole_diameter, 0.0);
    }
}
