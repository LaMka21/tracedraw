//! Neon tubes along centre lines.
//!
//! Each subpath of the selected curves is a tube's centre line. A line
//! longer than the maximum tube length is split into the fewest tubes
//! that keep under it; each split is moved to the nearest straight
//! stretch (a run of straight segments at least three tube diameters
//! long, the split kept a diameter from its ends) within the range that
//! keeps the tube count, else it stays at the even split. Support holes
//! go along each tube, evenly spread, at most the support spacing apart.

use super::{finite_path, FLAT_TOL};
use tracedraw_core::geometry::{
    BezPath, ParamCurve, ParamCurveArclen, PathSeg, Point, Shape as _, Vec2,
};
use tracedraw_core::shaping;

const ACC: f64 = 1e-4;

/// The tube settings (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeonSpec {
    pub diameter: f64,
    /// Longest tube.
    pub max_length: f64,
    /// Largest distance between support holes along a tube (0: none).
    pub support_spacing: f64,
    pub hole_diameter: f64,
}

impl Default for NeonSpec {
    fn default() -> Self {
        NeonSpec {
            diameter: 12.0,
            max_length: 1500.0,
            support_spacing: 300.0,
            hole_diameter: 5.0,
        }
    }
}

/// One tube: its centre line and length.
#[derive(Debug, Clone, PartialEq)]
pub struct Tube {
    pub path: BezPath,
    pub length: f64,
    /// A whole closed loop (not split).
    pub closed: bool,
}

/// The tubes and the support holes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NeonResult {
    pub tubes: Vec<Tube>,
    pub supports: Vec<Point>,
}

impl NeonResult {
    pub fn total_length(&self) -> f64 {
        self.tubes.iter().map(|t| t.length).sum()
    }
}

/// Most tubes one run makes.
pub const MAX_TUBES: usize = 10_000;

/// Split the centre lines into tubes and place the support holes.
pub fn neon(paths: &[BezPath], spec: &NeonSpec) -> NeonResult {
    let d = if spec.diameter.is_finite() {
        spec.diameter.abs()
    } else {
        0.0
    };
    let max = if spec.max_length.is_finite() && spec.max_length > 1.0 {
        spec.max_length
    } else {
        f64::INFINITY
    };
    let mut out = NeonResult::default();
    for path in paths.iter().filter(|p| finite_path(p)) {
        for sub in crate::subpaths(path) {
            let segs: Vec<(PathSeg, f64)> = sub
                .path
                .segments()
                .map(|s| (s, s.arclen(ACC)))
                .filter(|(_, l)| l.is_finite())
                .collect();
            let total: f64 = segs.iter().map(|s| s.1).sum();
            if total < 1e-6 {
                continue;
            }
            let cuts = split_points(&segs, total, max, d);
            let Some(cuts) = cuts else {
                continue;
            };
            let mut bounds = vec![0.0];
            bounds.extend(cuts.iter().copied());
            bounds.push(total);
            let whole = bounds.len() == 2;
            for w in bounds.windows(2) {
                let (a, b) = (w[0], w[1]);
                let mut piece = extract(&segs, a, b);
                if whole && sub.closed {
                    piece.close_path();
                }
                out.tubes.push(Tube {
                    path: piece,
                    length: b - a,
                    closed: whole && sub.closed,
                });
                if spec.support_spacing.is_finite() && spec.support_spacing > 1e-6 {
                    let len = b - a;
                    let n = (len / spec.support_spacing - 1e-9).ceil().max(1.0);
                    if n < 100_000.0 {
                        let n = n as usize;
                        for k in 0..n {
                            let s = a + (k as f64 + 0.5) * len / n as f64;
                            if let Some(p) = point_at(&segs, s) {
                                out.supports.push(p);
                            }
                        }
                    }
                }
                if out.tubes.len() >= MAX_TUBES {
                    return out;
                }
            }
        }
    }
    out
}

/// Where to split a line of `total` length into tubes of at most `max`:
/// `None` when it would take too many tubes.
fn split_points(segs: &[(PathSeg, f64)], total: f64, max: f64, d: f64) -> Option<Vec<f64>> {
    if total <= max + 1e-9 {
        return Some(Vec::new());
    }
    let needed = (total / max - 1e-9).ceil();
    if !needed.is_finite() || needed > MAX_TUBES as f64 {
        return None;
    }
    let runs = straight_runs(segs, (3.0 * d).max(1.0));
    let margin = d.max(0.5);
    let mut cuts = Vec::new();
    let mut p = 0.0;
    while total - p > max + 1e-9 {
        let left = ((total - p) / max - 1e-9).ceil();
        let target = p + (total - p) / left;
        // Any split in [lo, hi] keeps the tube count.
        let lo = (total - (left - 1.0) * max).max(p + 1e-6);
        let hi = p + max;
        let mut best: Option<f64> = None;
        for (rs, re) in &runs {
            let a = (rs + margin).max(lo);
            let b = (re - margin).min(hi);
            if a > b {
                continue;
            }
            let c = target.clamp(a, b);
            if best.is_none_or(|x| (x - target).abs() > (c - target).abs()) {
                best = Some(c);
            }
        }
        let cut = best.unwrap_or(target);
        cuts.push(cut);
        p = cut;
        if cuts.len() > MAX_TUBES {
            return None;
        }
    }
    Some(cuts)
}

/// The stretches (start and end along the line) where the line runs
/// straight for at least `min_len`.
fn straight_runs(segs: &[(PathSeg, f64)], min_len: f64) -> Vec<(f64, f64)> {
    let mut runs = Vec::new();
    let mut run: Option<(f64, f64, Vec2)> = None;
    let mut at = 0.0;
    for (seg, len) in segs {
        let dir = straight_dir(seg);
        match (dir, run.as_mut()) {
            (Some(d), Some((_, end, rd))) if rd.dot(d) > (1.0f64).to_radians().cos() => {
                *end = at + len;
            }
            (Some(d), _) => {
                if let Some((s, e, _)) = run.take() {
                    if e - s >= min_len {
                        runs.push((s, e));
                    }
                }
                run = Some((at, at + len, d));
            }
            (None, _) => {
                if let Some((s, e, _)) = run.take() {
                    if e - s >= min_len {
                        runs.push((s, e));
                    }
                }
            }
        }
        at += len;
    }
    if let Some((s, e, _)) = run {
        if e - s >= min_len {
            runs.push((s, e));
        }
    }
    runs
}

/// The unit direction of a segment that is straight (its control points
/// within 0.05 mm of its chord).
fn straight_dir(seg: &PathSeg) -> Option<Vec2> {
    let (a, b, ctrl): (Point, Point, Vec<Point>) = match *seg {
        PathSeg::Line(l) => (l.p0, l.p1, Vec::new()),
        PathSeg::Quad(q) => (q.p0, q.p2, vec![q.p1]),
        PathSeg::Cubic(c) => (c.p0, c.p3, vec![c.p1, c.p2]),
    };
    let ab = b - a;
    let len = ab.hypot();
    if len < 1e-9 {
        return None;
    }
    let ok = ctrl
        .iter()
        .all(|p| super::seg_dist(*p, a, b) <= FLAT_TOL && (*p - a).dot(ab) >= -1e-9);
    ok.then(|| ab / len)
}

/// The piece of the line between two lengths along it.
pub(crate) fn extract(segs: &[(PathSeg, f64)], from: f64, to: f64) -> BezPath {
    let mut out = BezPath::new();
    let mut at = 0.0;
    for (seg, len) in segs {
        let (s0, s1) = (at, at + len);
        at = s1;
        if s1 <= from + 1e-12 || s0 >= to - 1e-12 || *len <= 0.0 {
            continue;
        }
        let t0 = if from > s0 {
            seg.inv_arclen(from - s0, ACC)
        } else {
            0.0
        };
        let t1 = if to < s1 {
            seg.inv_arclen(to - s0, ACC)
        } else {
            1.0
        };
        if t1 <= t0 {
            continue;
        }
        let part = seg.subsegment(t0..t1);
        if out.elements().is_empty() {
            out.move_to(part.start());
        }
        match part {
            PathSeg::Line(l) => out.line_to(l.p1),
            PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
            PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
        }
    }
    out
}

/// The point at a length along the line.
pub(crate) fn point_at(segs: &[(PathSeg, f64)], s: f64) -> Option<Point> {
    let mut at = 0.0;
    let mut last = None;
    for (seg, len) in segs {
        if s <= at + len {
            let t = if *len > 0.0 {
                seg.inv_arclen((s - at).max(0.0), ACC)
            } else {
                0.0
            };
            return Some(seg.eval(t));
        }
        at += len;
        last = Some(seg.end());
    }
    last
}

/// A tube drawn as its outline: both sides of the centre line at half
/// the diameter, round ends (a closed loop gives two rings).
pub fn tube_outline(tube: &Tube, diameter: f64) -> BezPath {
    if !(diameter.is_finite() && diameter > 0.0) {
        return BezPath::new();
    }
    let mut pts: Vec<Point> = Vec::new();
    tracedraw_core::geometry::flatten(&tube.path, FLAT_TOL, &mut |el| match el {
        tracedraw_core::geometry::PathEl::MoveTo(p)
        | tracedraw_core::geometry::PathEl::LineTo(p) => pts.push(p),
        _ => {}
    });
    if tube.closed {
        if let Some(f) = pts.first().copied() {
            pts.push(f);
        }
    }
    let band = shaping::stroke_band(&pts, diameter, false);
    // A loop's ends meet: weld the band with itself to clean the joint.
    if tube.closed {
        let r = band.bounding_box();
        let all =
            tracedraw_core::geometry::Rect::new(r.x0 - 1.0, r.y0 - 1.0, r.x1 + 1.0, r.y1 + 1.0)
                .to_path(0.01);
        return shaping::overlay(&band, &all, shaping::Op::Intersect);
    }
    band
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Circle, Rect};

    fn line(pts: &[(f64, f64)]) -> BezPath {
        let mut p = BezPath::new();
        for (i, q) in pts.iter().enumerate() {
            if i == 0 {
                p.move_to(*q);
            } else {
                p.line_to(*q);
            }
        }
        p
    }

    #[test]
    fn a_short_line_is_one_tube() {
        let r = neon(&[line(&[(0.0, 0.0), (1000.0, 0.0)])], &NeonSpec::default());
        assert_eq!(r.tubes.len(), 1);
        assert!((r.total_length() - 1000.0).abs() < 1e-6);
        // 1000 / 300: 4 supports at 125, 375, 625, 875.
        assert_eq!(r.supports.len(), 4);
        assert!((r.supports[0].x - 125.0).abs() < 1e-6);
    }

    #[test]
    fn a_long_line_splits_into_tubes_under_the_maximum() {
        let r = neon(&[line(&[(0.0, 0.0), (4000.0, 0.0)])], &NeonSpec::default());
        assert_eq!(r.tubes.len(), 3);
        assert!(r.tubes.iter().all(|t| t.length <= 1500.0 + 1e-6));
        assert!((r.total_length() - 4000.0).abs() < 1e-6);
        // Even splits on a straight line.
        assert!((r.tubes[0].length - 4000.0 / 3.0).abs() < 1e-3);
    }

    #[test]
    fn splits_move_to_a_straight_stretch() {
        // A quarter circle arc of 1000 mm, then 1000 mm straight. Even
        // split at 1000 lies at the arc's end: it moves into the straight
        // part, a diameter in.
        let r0 = 2000.0 / std::f64::consts::PI;
        let mut p = BezPath::new();
        p.move_to((r0, 0.0));
        let a = tracedraw_core::geometry::Arc::new(
            (0.0, 0.0),
            (r0, r0),
            0.0,
            std::f64::consts::FRAC_PI_2,
            0.0,
        );
        a.append_iter(0.01).for_each(|el| p.push(el));
        p.line_to((-1000.0, r0));
        let spec = NeonSpec {
            max_length: 1100.0,
            ..Default::default()
        };
        let r = neon(&[p], &spec);
        assert_eq!(r.tubes.len(), 2);
        assert!(r.tubes.iter().all(|t| t.length <= 1100.0 + 1e-6));
        // The split lies on the straight part (y = r0).
        let end = r.tubes[0]
            .path
            .elements()
            .last()
            .and_then(|e| e.end_point());
        let end = end.expect("an end");
        assert!((end.y - r0).abs() < 1e-3, "{end:?}");
        assert!(end.x <= -12.0 + 1e-6, "{end:?}");
    }

    #[test]
    fn a_closed_loop_stays_closed_and_gets_two_rings() {
        let c = Circle::new((0.0, 0.0), 100.0).to_path(1e-4);
        let r = neon(&[c], &NeonSpec::default());
        assert_eq!(r.tubes.len(), 1);
        assert!(r.tubes[0].closed);
        assert!((r.total_length() - 2.0 * std::f64::consts::PI * 100.0).abs() < 0.01);
        let o = tube_outline(&r.tubes[0], 12.0);
        let rings = crate::subpaths(&o);
        assert_eq!(rings.len(), 2, "{o:?}");
        let b = o.bounding_box();
        assert!((b.width() - 212.0).abs() < 0.5);
    }

    #[test]
    fn an_open_tube_outline_has_round_ends() {
        let r = neon(&[line(&[(0.0, 0.0), (100.0, 0.0)])], &NeonSpec::default());
        let o = tube_outline(&r.tubes[0], 12.0);
        let b = o.bounding_box();
        assert!((b.width() - 112.0).abs() < 0.1 && (b.height() - 12.0).abs() < 0.1);
    }

    #[test]
    fn no_supports_when_the_spacing_is_zero_and_bad_input_is_skipped() {
        let spec = NeonSpec {
            support_spacing: 0.0,
            ..Default::default()
        };
        let r = neon(&[line(&[(0.0, 0.0), (100.0, 0.0)])], &spec);
        assert!(r.supports.is_empty());
        let r = neon(
            &[line(&[(0.0, 0.0), (f64::NAN, 0.0)]), BezPath::new()],
            &NeonSpec {
                diameter: f64::NAN,
                max_length: f64::INFINITY,
                support_spacing: f64::NAN,
                hole_diameter: -1.0,
            },
        );
        assert!(r.tubes.is_empty());
        // A tiny maximum would make too many tubes: the line is skipped.
        let r = neon(
            &[line(&[(0.0, 0.0), (1e12, 0.0)])],
            &NeonSpec {
                max_length: 2.0,
                ..Default::default()
            },
        );
        assert!(r.tubes.is_empty());
        assert!(tube_outline(
            &Tube {
                path: Rect::new(0.0, 0.0, 1.0, 1.0).to_path(0.1),
                length: 4.0,
                closed: true
            },
            f64::NAN
        )
        .elements()
        .is_empty());
    }
}
