//! traceCUT in the application: which objects its tools work on and
//! their outlines in page space (the geometry is in `tracedraw-tracecut`).
//! See docs/behavior/tracecut-measure.md.

use crate::app::App;
use tracedraw_core::geometry::BezPath;
use tracedraw_core::ShapeId;
use tracedraw_tracecut::measure::{self, Measurement};

/// What the Measure panel last measured, and for what.
#[derive(Debug, Clone, Default)]
pub struct TraceCut {
    measured: Option<(MeasureKey, Measurement)>,
}

/// A measurement holds while the drawing, its page and the objects
/// measured stay the same.
#[derive(Debug, Clone, PartialEq)]
struct MeasureKey {
    doc: usize,
    page: tracedraw_core::PageId,
    revision: u64,
    targets: Vec<ShapeId>,
}

impl App {
    /// The objects traceCUT works on: the selection, or with nothing
    /// selected every visible, unlocked object of the page, in drawing
    /// order (objects picked inside groups after, in the order picked).
    pub fn tracecut_targets(&self) -> Vec<ShapeId> {
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
        for id in &self.selection {
            if !ordered.contains(id) && doc.find_shape(*id).is_some() {
                ordered.push(*id);
            }
        }
        ordered
    }

    /// The outlines of each object, in page space.
    fn tracecut_outlines(&self, ids: &[ShapeId]) -> Vec<Vec<BezPath>> {
        let doc = self.doc();
        ids.iter()
            .map(|id| {
                doc.shape_on_page(*id)
                    .map(|s| tracedraw_tracecut::outlines(&s))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Outline length and areas of the targets, measured again only when
    /// the drawing, the page or the targets changed.
    pub fn tracecut_measurement(&mut self) -> Measurement {
        let key = MeasureKey {
            doc: self.active_doc,
            page: self.page,
            revision: self.engine.revision(),
            targets: self.tracecut_targets(),
        };
        if let Some((k, m)) = &self.tracecut.measured {
            if *k == key {
                return *m;
            }
        }
        let m = measure::measure(&self.tracecut_outlines(&key.targets));
        self.tracecut.measured = Some((key, m));
        m
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn rect(app: &mut App, x: f64, w: f64, h: f64) -> tracedraw_core::ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, 10.0, x + w, 10.0 + h),
            radius: 0.0,
            corners: None,
        })
        .expect("rect")
    }

    #[test]
    fn the_selection_or_else_the_whole_page_is_measured() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 30.0, 20.0);
        let _b = rect(&mut app, 60.0, 10.0, 10.0);
        app.select(vec![a]);
        let m = app.tracecut_measurement();
        assert_eq!(m.objects, 1);
        assert!((m.length - 100.0).abs() < 1e-9);
        assert!((m.area - 600.0).abs() < 1e-9);
        app.select(Vec::new());
        let m = app.tracecut_measurement();
        assert_eq!(m.objects, 2);
        assert!((m.length - 140.0).abs() < 1e-9);
        assert!((m.area - 700.0).abs() < 1e-9);
    }

    #[test]
    fn a_change_to_the_drawing_is_measured_again() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 30.0, 20.0);
        app.select(vec![a]);
        assert!((app.tracecut_measurement().area - 600.0).abs() < 1e-9);
        // Twice the width: twice the area.
        app.run(tracedraw_core::Command::TransformShapes {
            shapes: vec![a],
            transform: tracedraw_core::geometry::Affine::scale_non_uniform(2.0, 1.0),
        });
        assert!((app.tracecut_measurement().area - 1200.0).abs() < 1e-9);
        app.undo();
        assert!((app.tracecut_measurement().area - 600.0).abs() < 1e-9);
    }

    #[test]
    fn hidden_and_locked_objects_count_only_when_selected() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 10.0);
        let b = rect(&mut app, 40.0, 10.0, 10.0);
        app.run(tracedraw_core::Command::SetVisible {
            shapes: vec![b],
            visible: false,
        });
        app.select(Vec::new());
        assert_eq!(app.tracecut_measurement().objects, 1);
        app.select(vec![a, b]);
        assert_eq!(app.tracecut_measurement().objects, 2);
    }
}
