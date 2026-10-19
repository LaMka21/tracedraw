//! The Sign & Cut panel's operations: measuring, cut-file clean-up and
//! sizing. The geometry lives in `tracedraw-signcut`; this module picks
//! the objects, turns results into commands and keeps what the panel
//! shows. See docs/behavior/sign-and-cut.md.

use crate::app::App;
use tracedraw_core::geometry::{Affine, BezPath, Point, Rect, Shape as _, Vec2};
use tracedraw_core::{Command, Shape, ShapeId, ShapeKind};
use tracedraw_signcut::cleanup::{self, EdgeResult};
use tracedraw_signcut::crossings::{self, Crossing};
use tracedraw_signcut::measure::{self, Measurement};
use tracedraw_signcut::sizing::{self, Anchor, Frame, RefMeasure, ReplaceFit, SizeSpec};

/// A section of the panel, opened from the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Measure,
    Cleanup,
    Size,
    Cut,
    Nest,
    Calc,
    Sign,
    Cnc,
    Fx,
}

/// The panel's settings and its last results.
#[derive(Debug, Clone)]
pub struct SignCutSettings {
    /// The section the menu last asked for (opened once, then free).
    pub open: Option<Section>,
    /// Tolerance of the clean-up tools (mm).
    pub tolerance: f64,
    /// Find Duplicates deletes them (else selects them).
    pub delete_duplicates: bool,
    /// Find Crossings also looks between objects.
    pub crossings_between: bool,
    pub size: SizeSpec,
    /// The width and height fields keep their values while switched off.
    pub size_width: f64,
    pub size_height: f64,
    pub use_width: bool,
    pub use_height: bool,
    pub ref_measure: RefMeasure,
    /// The reference object and its value when it was picked (mm).
    pub reference: Option<(ShapeId, f64)>,
    /// The real size the reference stands for (mm).
    pub ref_wanted: f64,
    /// Reference Scale changes every object on the page (else the
    /// selection with the reference).
    pub ref_whole_page: bool,
    pub ref_anchor: Anchor,
    /// The master object of Replace.
    pub master: Option<ShapeId>,
    pub replace_fit: ReplaceFit,
    pub replace_turn: bool,
    pub replace_keep: bool,
    /// Last measurement, with the drawing revision it was taken at.
    pub measured: Option<(u64, Measurement)>,
    /// Last crossings found, with the drawing revision.
    pub crossings: Option<(u64, Vec<Crossing>)>,
}

impl Default for SignCutSettings {
    fn default() -> Self {
        SignCutSettings {
            open: None,
            tolerance: 0.05,
            delete_duplicates: false,
            crossings_between: false,
            size: SizeSpec::default(),
            size_width: 100.0,
            size_height: 100.0,
            use_width: true,
            use_height: false,
            ref_measure: RefMeasure::Width,
            reference: None,
            ref_wanted: 1000.0,
            ref_whole_page: true,
            ref_anchor: Anchor::BottomLeft,
            master: None,
            replace_fit: ReplaceFit::Fit,
            replace_turn: true,
            replace_keep: false,
            measured: None,
            crossings: None,
        }
    }
}

/// Radius of the marks drawn at crossings, in screen pixels.
pub const CROSSING_MARK_PX: f64 = 6.0;

impl App {
    /// Open the Sign & Cut panel at `section`.
    pub fn open_signcut(&mut self, section: Section) {
        self.show_dockers = true;
        self.docker_tab = crate::app::DockerTab::SignCut;
        self.signcut.open = Some(section);
    }

    /// The objects the tools work on, in stacking order (bottom first):
    /// the selection, or with nothing selected every visible, unlocked
    /// object of the page.
    pub fn signcut_targets(&self) -> Vec<ShapeId> {
        let doc = self.doc();
        let mut ordered: Vec<ShapeId> = Vec::new();
        for layer in self.page_layers() {
            for s in &layer.shapes {
                if self.selection.is_empty() {
                    if layer.visible && !layer.locked && s.visible && !s.locked {
                        ordered.push(s.id);
                    }
                } else if self.selection.contains(&s.id) {
                    ordered.push(s.id);
                }
            }
        }
        // Objects picked inside groups come after, in the order picked.
        for id in &self.selection {
            if !ordered.contains(id) && doc.find_shape(*id).is_some() {
                ordered.push(*id);
            }
        }
        ordered
    }

    /// Outlines of each object in page space.
    fn signcut_outlines(&self, ids: &[ShapeId]) -> Vec<Vec<BezPath>> {
        let doc = self.doc();
        ids.iter()
            .map(|id| {
                doc.shape_on_page(*id)
                    .map(|s| tracedraw_signcut::outlines(&s))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Measure the targets' length and area.
    pub fn signcut_measure(&mut self) -> Measurement {
        let ids = self.signcut_targets();
        let m = measure::measure(&self.signcut_outlines(&ids));
        self.signcut.measured = Some((self.engine.revision(), m));
        m
    }

    /// Find objects repeating an earlier one; delete or select them.
    /// Returns how many were found.
    pub fn signcut_find_duplicates(&mut self) -> usize {
        let ids = self.signcut_targets();
        let found = cleanup::find_duplicates(&self.signcut_outlines(&ids), self.signcut.tolerance);
        let dups: Vec<ShapeId> = found
            .iter()
            .filter_map(|(i, _)| ids.get(*i).copied())
            .filter(|id| !self.is_locked_shape(*id))
            .collect();
        let n = dups.len();
        if n == 0 {
            self.status = crate::i18n::tr("signcut.no_duplicates");
            return 0;
        }
        if self.signcut.delete_duplicates {
            self.selection.retain(|id| !dups.contains(id));
            if let Err(e) = self.engine.run_batch(
                "Delete Duplicates",
                &[Command::DeleteShapes { shapes: dups }],
            ) {
                self.status = e.to_string();
                return 0;
            }
            self.status = crate::i18n::trf("signcut.duplicates_deleted", &[("n", &n.to_string())]);
        } else {
            self.select(dups);
            self.status = crate::i18n::trf("signcut.duplicates_selected", &[("n", &n.to_string())]);
        }
        n
    }

    /// Take out the stretches of outline that run along an earlier edge.
    /// Returns the length taken out (mm).
    pub fn signcut_remove_duplicate_edges(&mut self) -> f64 {
        let leaves = self.signcut_leaves(&self.signcut_targets());
        let doc = self.doc();
        let shapes: Vec<Shape> = leaves
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .collect();
        let outlines: Vec<Vec<BezPath>> = shapes.iter().map(|s| vec![s.page_path()]).collect();
        let results = cleanup::remove_duplicate_edges(&outlines, self.signcut.tolerance);
        let mut cmds = Vec::new();
        let mut deleted = Vec::new();
        let mut total = 0.0;
        for (shape, r) in shapes.iter().zip(results) {
            match r {
                EdgeResult::Unchanged => {}
                EdgeResult::Changed {
                    path,
                    removed_length,
                } => {
                    let Some(inv) = invert(shape.transform) else {
                        continue;
                    };
                    total += removed_length;
                    cmds.push(Command::SetShapeKind {
                        shape: shape.id,
                        kind: ShapeKind::Path {
                            path: inv * path,
                            closed: false,
                        },
                    });
                }
                EdgeResult::Removed { removed_length } => {
                    total += removed_length;
                    deleted.push(shape.id);
                }
            }
        }
        if cmds.is_empty() && deleted.is_empty() {
            self.status = crate::i18n::tr("signcut.no_duplicate_edges");
            return 0.0;
        }
        if !deleted.is_empty() {
            self.selection.retain(|id| !deleted.contains(id));
            cmds.push(Command::DeleteShapes { shapes: deleted });
        }
        if let Err(e) = self.engine.run_batch("Remove Duplicate Edges", &cmds) {
            self.status = e.to_string();
            return 0.0;
        }
        let u = self.units;
        self.status = crate::i18n::trf(
            "signcut.duplicate_edges_removed",
            &[(
                "len",
                &format!("{:.2} {}", u.world_from_mm(total), u.short()),
            )],
        );
        total
    }

    /// The objects among `ids` whose outline the edge clean-up may
    /// rewrite: curves and basic shapes, groups opened, unlocked.
    fn signcut_leaves(&self, ids: &[ShapeId]) -> Vec<ShapeId> {
        fn walk(s: &Shape, out: &mut Vec<ShapeId>) {
            if s.locked {
                return;
            }
            match &s.kind {
                ShapeKind::Group { children } => {
                    for c in children {
                        walk(c, out);
                    }
                }
                ShapeKind::Rect { .. }
                | ShapeKind::Ellipse { .. }
                | ShapeKind::Polygon { .. }
                | ShapeKind::Path { .. }
                    if s.effects.is_empty() =>
                {
                    out.push(s.id);
                }
                _ => {}
            }
        }
        let doc = self.doc();
        let mut out = Vec::new();
        for id in ids {
            if let Some(s) = doc.find_shape(*id) {
                walk(s, &mut out);
            }
        }
        out
    }

    /// Find crossings and select the objects that have them.
    pub fn signcut_find_crossings(&mut self) -> usize {
        let ids = self.signcut_targets();
        let found = crossings::find_crossings(
            &self.signcut_outlines(&ids),
            self.signcut.crossings_between,
            self.signcut.tolerance,
        );
        let n = found.len();
        let mut with: Vec<ShapeId> = Vec::new();
        for c in &found {
            for i in [c.a, c.b] {
                if let Some(id) = ids.get(i) {
                    if !with.contains(id) {
                        with.push(*id);
                    }
                }
            }
        }
        self.signcut.crossings = Some((self.engine.revision(), found));
        if n == 0 {
            self.status = crate::i18n::tr("signcut.no_crossings");
        } else {
            self.select(with);
            self.status = crate::i18n::trf("signcut.crossings_found", &[("n", &n.to_string())]);
        }
        n
    }

    /// The crossings to mark on the drawing: those of the last search,
    /// while the drawing has not changed since.
    pub fn signcut_crossing_marks(&self) -> Vec<Point> {
        match &self.signcut.crossings {
            Some((rev, c)) if *rev == self.engine.revision() => c.iter().map(|c| c.point).collect(),
            _ => Vec::new(),
        }
    }

    /// Give each selected object the size of the panel. Returns how many
    /// changed.
    pub fn signcut_set_size(&mut self) -> usize {
        let mut spec = self.signcut.size;
        spec.width = self.signcut.use_width.then_some(self.signcut.size_width);
        spec.height = self.signcut.use_height.then_some(self.signcut.size_height);
        let doc = self.doc();
        let cmds: Vec<Command> = self
            .selection
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .filter(|s| !s.locked)
            .filter_map(|s| {
                sizing::size_transform(s.bounds(), &spec).map(|t| Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: t,
                })
            })
            .collect();
        let n = cmds.len();
        if n == 0 {
            return 0;
        }
        if let Err(e) = self.engine.run_batch("Set Size", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        n
    }

    /// Take the selected object as the reference of Reference Scale.
    pub fn signcut_pick_reference(&mut self) -> bool {
        let [id] = self.selection[..] else {
            self.status = crate::i18n::tr("signcut.select_one");
            return false;
        };
        let Some(s) = self.doc().shape_on_page(id) else {
            return false;
        };
        let len = tracedraw_signcut::outlines(&s)
            .iter()
            .map(measure::length)
            .sum();
        let v = sizing::reference_value(self.signcut.ref_measure, s.bounds(), len);
        self.signcut.reference = Some((id, v));
        self.signcut.ref_wanted = v;
        true
    }

    /// The reference's value now (it may have changed since it was
    /// picked); `None` when it is gone.
    pub fn signcut_reference_value(&self) -> Option<f64> {
        let (id, _) = self.signcut.reference?;
        let s = self.doc().shape_on_page(id)?;
        let len = tracedraw_signcut::outlines(&s)
            .iter()
            .map(measure::length)
            .sum();
        Some(sizing::reference_value(
            self.signcut.ref_measure,
            s.bounds(),
            len,
        ))
    }

    /// Scale so that the reference measures what it stands for.
    pub fn signcut_reference_scale(&mut self) -> Option<f64> {
        let (rid, _) = self.signcut.reference?;
        let current = self.signcut_reference_value()?;
        let factor = sizing::reference_factor(current, self.signcut.ref_wanted)?;
        let doc = self.doc();
        let mut ids: Vec<ShapeId> = if self.signcut.ref_whole_page {
            self.page_layers()
                .iter()
                .filter(|l| !l.locked)
                .flat_map(|l| l.shapes.iter())
                .filter(|s| !s.locked)
                .map(|s| s.id)
                .collect()
        } else {
            self.selection.clone()
        };
        if !ids.contains(&rid) && doc.top_level_of(rid).is_none_or(|t| !ids.contains(&t)) {
            ids.push(rid);
        }
        let bounds = ids
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .map(|s| s.bounds())
            .reduce(|a, b| a.union(b))?;
        let at = self.signcut.ref_anchor.point(bounds);
        let t = sizing::scale_about(at, factor, factor);
        if let Err(e) = self.engine.run_batch(
            "Reference Scale",
            &[Command::TransformShapes {
                shapes: ids,
                transform: t,
            }],
        ) {
            self.status = e.to_string();
            return None;
        }
        self.signcut.reference = Some((rid, self.signcut.ref_wanted));
        Some(factor)
    }

    /// Take the selected object as the master of Replace.
    pub fn signcut_pick_master(&mut self) -> bool {
        let [id] = self.selection[..] else {
            self.status = crate::i18n::tr("signcut.select_one");
            return false;
        };
        self.signcut.master = Some(id);
        true
    }

    /// Put a copy of the master in the place of each selected object.
    /// Returns how many were replaced.
    pub fn signcut_replace(&mut self) -> usize {
        let Some(mid) = self.signcut.master else {
            self.status = crate::i18n::tr("signcut.no_master");
            return 0;
        };
        let doc = self.doc();
        let Some(master) = doc.shape_on_page(mid) else {
            self.signcut.master = None;
            self.status = crate::i18n::tr("signcut.no_master");
            return 0;
        };
        let turn = self.signcut.replace_turn;
        let Some(mframe) = frame_of(&master, turn) else {
            return 0;
        };
        let targets: Vec<(Shape, tracedraw_core::LayerId)> = self
            .selection
            .iter()
            .filter(|id| **id != mid)
            .filter_map(|id| {
                let s = doc.shape_on_page(*id)?;
                let top = doc.top_level_of(*id).unwrap_or(*id);
                let (layer, _) = doc.locate(top).ok()?;
                (!s.locked).then_some((s, layer))
            })
            .collect();
        if targets.is_empty() {
            return 0;
        }
        let mut cmds = Vec::new();
        let mut new_ids = Vec::new();
        let mut gone = Vec::new();
        for (target, layer) in targets {
            let Some(tframe) = frame_of(&target, turn) else {
                continue;
            };
            let t = sizing::replace_transform(mframe, tframe, self.signcut.replace_fit, turn);
            let id = self.engine.new_shape_id();
            let mut copy = crate::app::reid_pub(master.clone(), id, &mut self.engine);
            copy.transform = t * master.transform;
            copy.uid = None;
            cmds.push(Command::AddShape { layer, shape: copy });
            new_ids.push(id);
            gone.push(target.id);
        }
        if !self.signcut.replace_keep && !gone.is_empty() {
            cmds.push(Command::DeleteShapes { shapes: gone });
        }
        let n = new_ids.len();
        if let Err(e) = self.engine.run_batch("Replace", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.select(new_ids);
        n
    }

    fn is_locked_shape(&self, id: ShapeId) -> bool {
        self.doc().find_shape(id).is_some_and(|s| s.locked)
    }
}

/// The inverse of `t`, when it has one.
fn invert(t: Affine) -> Option<Affine> {
    let det = t.determinant();
    (det.is_finite() && det.abs() > 1e-12).then(|| t.inverse())
}

/// An object's box with its turn: centre, size along its own axes and
/// angle (no turn: the page-space bounds).
fn frame_of(s: &Shape, turn: bool) -> Option<Frame> {
    let angle = if turn {
        sizing::transform_angle(s.transform)
    } else {
        0.0
    };
    let unturn = Affine::rotate(-angle);
    let outlines = tracedraw_signcut::outlines(s);
    let b: Rect = outlines
        .iter()
        .map(|p| (unturn * p.clone()).bounding_box())
        .filter(|b| b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite())
        .reduce(|a, b| a.union(b))?;
    Some(Frame {
        center: Affine::rotate(angle) * b.center(),
        size: Vec2::new(b.width(), b.height()),
        angle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer to draw on")
    }

    fn bounds(app: &App, id: ShapeId) -> Rect {
        app.doc().shape_on_page(id).expect("shape").bounds()
    }

    #[test]
    fn measuring_adds_up_the_selection() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 20.0, 0.0, 10.0, 20.0);
        rect(&mut app, 100.0, 0.0, 5.0, 5.0);
        app.select(vec![a, b]);
        let m = app.signcut_measure();
        assert_eq!(m.objects, 2);
        assert!((m.length - 100.0).abs() < 1e-9);
        assert!((m.area - 300.0).abs() < 1e-9);
        // Nothing selected: the whole page.
        app.select(vec![]);
        assert_eq!(app.signcut_measure().objects, 3);
    }

    #[test]
    fn duplicates_are_selected_or_deleted_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        rect(&mut app, 50.0, 0.0, 10.0, 10.0);
        assert_eq!(app.signcut_find_duplicates(), 1);
        assert_eq!(app.selection, vec![b]);
        app.select(vec![]);
        app.signcut.delete_duplicates = true;
        assert_eq!(app.signcut_find_duplicates(), 1);
        assert!(app.doc().find_shape(b).is_none());
        assert!(app.doc().find_shape(a).is_some());
        assert_eq!(app.engine.undo_label(), Some("Delete Duplicates"));
        assert!(app.engine.undo().is_ok());
        assert!(app.doc().find_shape(b).is_some());
    }

    #[test]
    fn a_shared_side_is_taken_out_of_the_upper_object() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 10.0, 0.0, 10.0, 10.0);
        app.select(vec![a, b]);
        let removed = app.signcut_remove_duplicate_edges();
        assert!((removed - 10.0).abs() < 1e-6);
        let sb = app.doc().shape_on_page(b).expect("b stays");
        assert!(matches!(sb.kind, ShapeKind::Path { closed: false, .. }));
        let len = measure::length(&sb.page_path());
        assert!((len - 30.0).abs() < 1e-6);
        // Still where it was.
        let bb = sb.bounds();
        assert!((bb.x0 - 10.0).abs() < 1e-9 && (bb.x1 - 20.0).abs() < 1e-9);
        assert!(matches!(
            app.doc().shape_on_page(a).map(|s| s.kind),
            Some(ShapeKind::Rect { .. })
        ));
        assert_eq!(app.engine.undo_label(), Some("Remove Duplicate Edges"));
    }

    #[test]
    fn crossings_select_their_objects_and_mark_until_a_change() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 5.0, 5.0, 10.0, 10.0);
        app.select(vec![a, b]);
        assert_eq!(app.signcut_find_crossings(), 0);
        app.signcut.crossings_between = true;
        app.select(vec![a, b]);
        assert_eq!(app.signcut_find_crossings(), 2);
        assert_eq!(app.selection.len(), 2);
        assert_eq!(app.signcut_crossing_marks().len(), 2);
        app.run(Command::TransformShapes {
            shapes: vec![b],
            transform: Affine::translate((50.0, 0.0)),
        });
        assert!(app.signcut_crossing_marks().is_empty());
    }

    #[test]
    fn set_size_sizes_each_object_on_its_own() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 5.0);
        let b = rect(&mut app, 50.0, 0.0, 40.0, 10.0);
        app.select(vec![a, b]);
        app.signcut.size_width = 20.0;
        app.signcut.use_width = true;
        app.signcut.use_height = false;
        app.signcut.size.anchor = Anchor::BottomLeft;
        assert_eq!(app.signcut_set_size(), 2);
        let ba = bounds(&app, a);
        let bb = bounds(&app, b);
        assert!((ba.width() - 20.0).abs() < 1e-9 && (ba.height() - 10.0).abs() < 1e-9);
        assert!((bb.width() - 20.0).abs() < 1e-9 && (bb.height() - 5.0).abs() < 1e-9);
        assert!((bb.x0 - 50.0).abs() < 1e-9);
        assert_eq!(app.engine.undo_label(), Some("Set Size"));
    }

    #[test]
    fn reference_scale_brings_the_drawing_to_real_size() {
        let mut app = App::headless();
        let r = rect(&mut app, 0.0, 0.0, 25.0, 1.0);
        let other = rect(&mut app, 30.0, 0.0, 10.0, 10.0);
        app.select(vec![r]);
        assert!(app.signcut_pick_reference());
        app.signcut.ref_wanted = 100.0;
        app.signcut.ref_anchor = Anchor::BottomLeft;
        assert_eq!(app.signcut_reference_scale(), Some(4.0));
        assert!((bounds(&app, r).width() - 100.0).abs() < 1e-9);
        let bo = bounds(&app, other);
        assert!((bo.x0 - 120.0).abs() < 1e-9 && (bo.width() - 40.0).abs() < 1e-9);
    }

    #[test]
    fn replace_puts_the_master_in_each_place() {
        let mut app = App::headless();
        let m = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                arc: None,
            })
            .expect("layer");
        let a = rect(&mut app, 100.0, 100.0, 20.0, 20.0);
        let b = rect(&mut app, 200.0, 100.0, 40.0, 20.0);
        app.select(vec![m]);
        assert!(app.signcut_pick_master());
        app.select(vec![a, b, m]);
        app.signcut.replace_fit = ReplaceFit::Fit;
        assert_eq!(app.signcut_replace(), 2);
        assert!(app.doc().find_shape(a).is_none());
        assert!(app.doc().find_shape(b).is_none());
        assert_eq!(app.selection.len(), 2);
        let nb = bounds(&app, app.selection[1]);
        assert!((nb.width() - 20.0).abs() < 1e-6 && (nb.center().x - 220.0).abs() < 1e-6);
        assert!(matches!(
            app.doc().shape_on_page(app.selection[0]).map(|s| s.kind),
            Some(ShapeKind::Ellipse { .. })
        ));
        assert_eq!(app.engine.undo_label(), Some("Replace"));
    }
}
