//! Sign & Cut, Nesting: settings and operations (placing objects on sheets and rolls with the least waste).
//! The geometry is in `tracedraw_signcut::nest`; the panel section is
//! `ui/signcut_nest.rs`. See docs/behavior/sign-and-cut-nest.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use egui::Ui;
use tracedraw_core::geometry::{Rect, Shape as _};
use tracedraw_core::{Color, Command, Shape, ShapeId, ShapeKind, Stroke};
use tracedraw_signcut::measure;
use tracedraw_signcut::nest::fill::{self, FillSettings};
use tracedraw_signcut::nest::rects::{self, RectItem, RectSettings};
use tracedraw_signcut::nest::shapes::{self, Rotations, ShapeSettings};
use tracedraw_signcut::nest::{Corner, Layout, RollDir, SheetSpec};

/// The three tools of the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Rects,
    Shapes,
    Fill,
}

/// What the last run did, for the panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    pub tool: Tool,
    /// Objects placed (copies made, for Fill Sheet) and objects given.
    pub placed: usize,
    pub total: usize,
    pub sheets: usize,
    pub roll: bool,
    /// Used length (mm): the roll's, or the last sheet's.
    pub length: f64,
    /// 0 to 1.
    pub utilisation: f64,
    pub rounds: usize,
    pub stopped_by_time: bool,
}

/// The Nesting section's settings and last results.
#[derive(Debug, Clone)]
pub struct Settings {
    pub tool: Tool,
    /// Sheets take the page's size.
    pub use_page: bool,
    /// Width, height, margin, spacing, corner, roll and its direction.
    pub sheet: SheetSpec,
    /// Add a frame (hairline rectangle) for each sheet or roll used.
    pub frames: bool,
    /// Rectangle nesting may turn boxes 90 degrees.
    pub rotate90: bool,
    pub rotations: Rotations,
    pub holes: bool,
    /// Outline accuracy of shape nesting (mm).
    pub tolerance: f64,
    /// Run the improvement loop.
    pub improve: bool,
    pub rounds: usize,
    /// Seconds the improvement loop may take.
    pub time_limit: f64,
    pub seed: u64,
    /// Fill Sheet: gap between copies (mm).
    pub gap: f64,
    pub fill_rotate: bool,
    /// Fill Sheet: at most this many copies (0: as many as fit).
    pub fill_limit: usize,
    pub report: Option<Report>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tool: Tool::Rects,
            use_page: true,
            sheet: SheetSpec::default(),
            frames: true,
            rotate90: true,
            rotations: Rotations::Quarter,
            holes: true,
            tolerance: 0.5,
            improve: true,
            rounds: 100,
            time_limit: 5.0,
            seed: 1,
            gap: 2.0,
            fill_rotate: true,
            fill_limit: 0,
            report: None,
        }
    }
}

/// The colour of sheet frames.
pub const FRAME_COLOR: Color = Color::Rgb {
    r: 0.0,
    g: 0.55,
    b: 0.85,
};

/// Tools > Sign & Cut > Nesting: each item opens the panel at its tool.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let doc = app.has_document() && !app.show_welcome;
    for (tool, key) in [
        (Tool::Rects, "menu.tools.signcut_nest_rects"),
        (Tool::Shapes, "menu.tools.signcut_nest_shapes"),
        (Tool::Fill, "menu.tools.signcut_nest_fill"),
    ] {
        if crate::ui::menus::item(ui, key, "", doc) {
            app.sc_nest.tool = tool;
            app.open_signcut(crate::signcut::Section::Nest);
        }
    }
}

impl App {
    /// The material the tools use: the settings' sheet, or the page's
    /// size for sheets when asked.
    pub fn nest_sheet(&self) -> SheetSpec {
        if self.sc_nest.sheet.roll {
            self.sc_nest.sheet
        } else {
            self.nest_plain_sheet()
        }
    }

    /// The sheet of Fill Sheet: never a roll.
    pub fn nest_plain_sheet(&self) -> SheetSpec {
        let mut s = self.sc_nest.sheet;
        s.roll = false;
        if self.sc_nest.use_page {
            let p = self.page_size();
            s.width = p.width;
            s.height = p.height;
        }
        s
    }

    /// The objects to place: the selection, unlocked, each once (an
    /// object inside a selected group goes with its group).
    pub fn nest_targets(&self) -> Vec<ShapeId> {
        let doc = self.doc();
        self.selection
            .iter()
            .copied()
            .filter(|id| doc.find_shape(*id).is_some_and(|s| !s.locked))
            .filter(|id| {
                !doc.ancestors(*id)
                    .iter()
                    .any(|a| self.selection.contains(a))
            })
            .collect()
    }

    /// Each target in page space with its outlines.
    fn nest_parts(&self, ids: &[ShapeId]) -> Vec<(Shape, Vec<tracedraw_core::geometry::BezPath>)> {
        let doc = self.doc();
        ids.iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .map(|s| {
                let o = tracedraw_signcut::outlines(&s);
                (s, o)
            })
            .collect()
    }

    /// Place the selected objects by their bounding boxes. One undo step.
    pub fn nest_rectangles(&mut self) -> Option<Layout> {
        let ids = self.nest_targets();
        if ids.is_empty() {
            self.status = tr("signcut.nest_select");
            return None;
        }
        let parts = self.nest_parts(&ids);
        let items: Vec<RectItem> = parts
            .iter()
            .map(|(s, o)| RectItem {
                bounds: outline_bounds(o).unwrap_or_else(|| s.bounds()),
                area: measure::measure_object(o).area,
            })
            .collect();
        let settings = RectSettings {
            sheet: self.nest_sheet(),
            rotate: self.sc_nest.rotate90,
        };
        let layout = rects::nest_rects(&items, &settings);
        let ids: Vec<ShapeId> = parts.iter().map(|(s, _)| s.id).collect();
        self.nest_apply("Nest Rectangles", Tool::Rects, &ids, &layout)
            .then_some(layout)
    }

    /// Place the selected objects by their true outlines. One undo step.
    pub fn nest_shapes(&mut self) -> Option<Layout> {
        let ids = self.nest_targets();
        if ids.is_empty() {
            self.status = tr("signcut.nest_select");
            return None;
        }
        let parts = self.nest_parts(&ids);
        let outlines: Vec<Vec<_>> = parts.iter().map(|(_, o)| o.clone()).collect();
        let n = &self.sc_nest;
        let settings = ShapeSettings {
            sheet: self.nest_sheet(),
            rotations: n.rotations,
            holes: n.holes,
            tolerance: n.tolerance,
            rounds: if n.improve { n.rounds } else { 0 },
            seed: n.seed,
        };
        let limit = if n.time_limit.is_finite() {
            n.time_limit.clamp(0.0, 600.0)
        } else {
            5.0
        };
        let start = web_time::Instant::now();
        let mut out_of_time = || start.elapsed().as_secs_f64() > limit;
        let layout = shapes::nest_shapes(&outlines, &settings, &mut out_of_time);
        let ids: Vec<ShapeId> = parts.iter().map(|(s, _)| s.id).collect();
        self.nest_apply("Nest Shapes", Tool::Shapes, &ids, &layout)
            .then_some(layout)
    }

    /// Move the objects as `layout` says, add the sheet frames, report.
    fn nest_apply(
        &mut self,
        label: &'static str,
        tool: Tool,
        ids: &[ShapeId],
        layout: &Layout,
    ) -> bool {
        let spec = self.nest_sheet();
        let mut cmds: Vec<Command> = Vec::new();
        let mut moved = Vec::new();
        for (id, p) in ids.iter().zip(&layout.placements) {
            if let Some(p) = p {
                cmds.push(Command::TransformShapes {
                    shapes: vec![*id],
                    transform: p.transform,
                });
                moved.push(*id);
            }
        }
        let report = Report {
            tool,
            placed: moved.len(),
            total: ids.len(),
            sheets: layout.sheets,
            roll: spec.roll,
            length: layout.length,
            utilisation: layout.utilisation(),
            rounds: layout.rounds,
            stopped_by_time: layout.stopped_by_time,
        };
        self.sc_nest.report = Some(report);
        if cmds.is_empty() {
            self.status = tr("signcut.nest_nothing_fits");
            return false;
        }
        if self.sc_nest.frames && layout.sheets > 0 {
            if let Some(frames) = self.nest_frames(&spec, layout.sheets, layout.length) {
                cmds.push(frames);
            }
        }
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.select(moved);
        self.status = nest_status(self.units, &report);
        true
    }

    /// The frames of `sheets` sheets (or of the roll used) as one group.
    fn nest_frames(&mut self, spec: &SheetSpec, sheets: usize, length: f64) -> Option<Command> {
        let layer = self.active_layer()?;
        let count = if spec.roll { 1 } else { sheets };
        let mut children = Vec::new();
        for i in 0..count {
            let rect = spec.sheet_rect(i, length);
            let mut s = Shape::new(
                self.engine.new_shape_id(),
                ShapeKind::Rect {
                    rect,
                    radius: 0.0,
                    corners: None,
                },
            );
            s.fill = tracedraw_core::Fill::None;
            s.stroke = Some(Stroke {
                color: FRAME_COLOR,
                ..Stroke::default()
            });
            s.name = Some(format!("{} {}", tr("signcut.nest_sheet_name"), i + 1));
            children.push(s);
        }
        let mut group = Shape::new(self.engine.new_shape_id(), ShapeKind::Group { children });
        group.name = Some(tr("signcut.nest_frames_name"));
        Some(Command::AddShape {
            layer,
            shape: group,
        })
    }

    /// Fill the sheet with copies of the selection, as one block, in a
    /// grid. The selection moves to the first place. One undo step.
    /// Returns the number of places filled (the original's included).
    pub fn nest_fill(&mut self) -> usize {
        let ids = self.nest_targets();
        if ids.is_empty() {
            self.status = tr("signcut.nest_select");
            return 0;
        }
        let parts = self.nest_parts(&ids);
        let Some(bounds) = parts
            .iter()
            .filter_map(|(s, o)| outline_bounds(o).or_else(|| Some(s.bounds())))
            .reduce(|a, b| a.union(b))
        else {
            return 0;
        };
        let n = &self.sc_nest;
        let settings = FillSettings {
            sheet: self.nest_plain_sheet(),
            gap: n.gap,
            rotate: n.fill_rotate,
            limit: n.fill_limit,
        };
        let copies = fill::fill_sheet(bounds, &settings);
        let report = Report {
            tool: Tool::Fill,
            placed: copies.len(),
            total: copies.len(),
            sheets: usize::from(!copies.is_empty()),
            roll: false,
            length: 0.0,
            utilisation: 0.0,
            rounds: 0,
            stopped_by_time: false,
        };
        self.sc_nest.report = Some(report);
        let Some(first) = copies.first() else {
            self.status = tr("signcut.nest_nothing_fits");
            return 0;
        };
        let ids: Vec<ShapeId> = parts.iter().map(|(s, _)| s.id).collect();
        let mut cmds = vec![Command::TransformShapes {
            shapes: ids.clone(),
            transform: first.transform,
        }];
        let doc = self.doc();
        let layers: Vec<Option<tracedraw_core::LayerId>> = parts
            .iter()
            .map(|(s, _)| {
                let top = doc.top_level_of(s.id).unwrap_or(s.id);
                doc.locate(top).ok().map(|(l, _)| l)
            })
            .collect();
        let mut new_ids = ids.clone();
        for c in copies.iter().skip(1) {
            for ((s, _), layer) in parts.iter().zip(&layers) {
                let Some(layer) = *layer else {
                    continue;
                };
                let id = self.engine.new_shape_id();
                let mut copy = crate::app::reid_pub(s.clone(), id, &mut self.engine);
                copy.transform = c.transform * s.transform;
                copy.uid = None;
                cmds.push(Command::AddShape { layer, shape: copy });
                new_ids.push(id);
            }
        }
        if let Err(e) = self.engine.run_batch("Fill Sheet", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.select(new_ids);
        self.status = trf("signcut.nest_filled", &[("n", &copies.len().to_string())]);
        copies.len()
    }
}

/// The box around the outlines, when they have a finite one.
fn outline_bounds(o: &[tracedraw_core::geometry::BezPath]) -> Option<Rect> {
    o.iter()
        .map(|p| p.bounding_box())
        .filter(|b| [b.x0, b.y0, b.x1, b.y1].iter().all(|v| v.is_finite()))
        .reduce(|a, b| a.union(b))
}

/// The status line after a nesting run.
pub fn nest_status(u: crate::app::Units, r: &Report) -> String {
    let len = crate::ui::signcut_docker::fmt_len(u, r.length);
    trf(
        if r.roll {
            "signcut.nest_done_roll"
        } else {
            "signcut.nest_done_sheets"
        },
        &[
            ("placed", &r.placed.to_string()),
            ("total", &r.total.to_string()),
            ("sheets", &r.sheets.to_string()),
            ("util", &format!("{:.1}", r.utilisation * 100.0)),
            ("len", &len),
        ],
    )
}

/// The corner's label key.
pub fn corner_key(c: Corner) -> &'static str {
    match c {
        Corner::BottomLeft => "signcut.nest_corner_bl",
        Corner::BottomRight => "signcut.nest_corner_br",
        Corner::TopLeft => "signcut.nest_corner_tl",
        Corner::TopRight => "signcut.nest_corner_tr",
    }
}

/// The turns' label key.
pub fn rotations_key(r: Rotations) -> &'static str {
    match r {
        Rotations::None => "signcut.nest_turn_none",
        Rotations::Half => "signcut.nest_turn_half",
        Rotations::Quarter => "signcut.nest_turn_quarter",
        Rotations::Eighth => "signcut.nest_turn_eighth",
    }
}

/// The roll direction's label key.
pub fn roll_dir_key(d: RollDir) -> &'static str {
    match d {
        RollDir::Right => "signcut.nest_roll_right",
        RollDir::Up => "signcut.nest_roll_up",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Affine, Rect};

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer to draw on")
    }

    fn bounds(app: &App, id: ShapeId) -> Rect {
        let s = app.doc().shape_on_page(id).expect("shape");
        outline_bounds(&tracedraw_signcut::outlines(&s)).expect("bounds")
    }

    fn top_level_count(app: &App) -> usize {
        app.page_layers().iter().map(|l| l.shapes.len()).sum()
    }

    fn small_sheet(app: &mut App, w: f64, h: f64) {
        app.sc_nest.use_page = false;
        app.sc_nest.sheet.width = w;
        app.sc_nest.sheet.height = h;
        app.sc_nest.sheet.spacing = 0.0;
        app.sc_nest.sheet.margin = 0.0;
    }

    #[test]
    fn rectangle_nesting_moves_the_selection_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 300.0, 300.0, 50.0, 50.0);
        let b = rect(&mut app, 400.0, 300.0, 50.0, 50.0);
        let c = rect(&mut app, 500.0, 300.0, 60.0, 60.0);
        small_sheet(&mut app, 100.0, 100.0);
        let before = top_level_count(&app);
        app.select(vec![a, b, c]);
        let l = app.nest_rectangles().expect("nested");
        assert_eq!((l.placed(), l.sheets), (3, 2));
        // The largest goes first: on sheet 1 at the corner.
        let bc = bounds(&app, c);
        assert!(bc.x0.abs() < 1e-9 && bc.y0.abs() < 1e-9);
        // One square on the second sheet, 20 mm to the right.
        let xs = [bounds(&app, a).x0, bounds(&app, b).x0];
        assert!(xs.iter().any(|x| *x >= 120.0 - 1e-9));
        // The frames: one group of two sheets.
        assert_eq!(top_level_count(&app), before + 1);
        assert_eq!(app.engine.undo_label(), Some("Nest Rectangles"));
        let r = app.sc_nest.report.expect("report");
        assert_eq!((r.placed, r.total, r.sheets), (3, 3, 2));
        assert!(app.engine.undo().is_ok());
        assert_eq!(top_level_count(&app), before);
        assert!((bounds(&app, c).x0 - 500.0).abs() < 1e-9);
    }

    #[test]
    fn shape_nesting_uses_the_outlines_and_the_time_limit() {
        let mut app = App::headless();
        let mut tri = tracedraw_core::geometry::BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((50.0, 0.0));
        tri.line_to((0.0, 50.0));
        tri.close_path();
        let a = app
            .new_shape(ShapeKind::Path {
                path: tri.clone(),
                closed: true,
            })
            .expect("layer");
        let b = app
            .new_shape(ShapeKind::Path {
                path: tri,
                closed: true,
            })
            .expect("layer");
        app.run(Command::TransformShapes {
            shapes: vec![b],
            transform: Affine::translate((200.0, 0.0)),
        });
        small_sheet(&mut app, 52.0, 52.0);
        app.sc_nest.frames = false;
        app.sc_nest.rounds = 3;
        app.sc_nest.time_limit = 30.0;
        let before = top_level_count(&app);
        app.select(vec![a, b]);
        let l = app.nest_shapes().expect("nested");
        assert_eq!((l.placed(), l.sheets), (2, 1));
        assert_eq!(l.rounds, 3);
        for id in [a, b] {
            let bb = bounds(&app, id);
            assert!(bb.x0 > -1e-6 && bb.y1 < 52.0 + 1e-6);
        }
        assert_eq!(top_level_count(&app), before, "no frames asked for");
        assert_eq!(app.engine.undo_label(), Some("Nest Shapes"));
        // A zero time limit still gives the first layout.
        app.sc_nest.time_limit = 0.0;
        app.sc_nest.rounds = 1000;
        let l = app.nest_shapes().expect("nested");
        assert!(l.stopped_by_time || l.rounds < 1000);
        assert_eq!(l.placed(), 2);
    }

    #[test]
    fn a_roll_gets_one_frame_of_the_length_used() {
        let mut app = App::headless();
        let ids: Vec<ShapeId> = (0..4)
            .map(|i| rect(&mut app, 300.0 + 40.0 * i as f64, 300.0, 30.0, 30.0))
            .collect();
        app.sc_nest.sheet.roll = true;
        app.sc_nest.sheet.width = 70.0;
        app.sc_nest.sheet.spacing = 0.0;
        app.select(ids);
        let l = app.nest_rectangles().expect("nested");
        assert!((l.length - 60.0).abs() < 1e-9);
        let layer = app
            .page_layers()
            .last()
            .map(|l| l.shapes.clone())
            .expect("layer");
        let frames = layer.last().expect("frames");
        let ShapeKind::Group { children } = &frames.kind else {
            panic!("a group of frames");
        };
        assert_eq!(children.len(), 1);
        let ShapeKind::Rect { rect: fb, .. } = children[0].kind else {
            panic!("a rectangle");
        };
        assert_eq!(fb, Rect::new(0.0, 0.0, 60.0, 70.0));
        assert_eq!(
            children[0].stroke.as_ref().map(|s| s.color),
            Some(FRAME_COLOR)
        );
        assert!(app.status.contains("60"));
    }

    #[test]
    fn fill_sheet_copies_the_selection_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 300.0, 300.0, 20.0, 10.0);
        small_sheet(&mut app, 100.0, 50.0);
        app.sc_nest.gap = 2.0;
        app.sc_nest.sheet.margin = 5.0;
        app.sc_nest.fill_rotate = false;
        let before = top_level_count(&app);
        app.select(vec![a]);
        assert_eq!(app.nest_fill(), 12);
        assert_eq!(top_level_count(&app), before + 11);
        assert_eq!(app.selection.len(), 12);
        let ba = bounds(&app, a);
        assert!((ba.x0 - 5.0).abs() < 1e-9 && (ba.y0 - 5.0).abs() < 1e-9);
        assert_eq!(app.engine.undo_label(), Some("Fill Sheet"));
        assert!(app.engine.undo().is_ok());
        assert_eq!(top_level_count(&app), before);
        // A limit.
        app.sc_nest.fill_limit = 3;
        app.select(vec![a]);
        assert_eq!(app.nest_fill(), 3);
    }

    #[test]
    fn nothing_selected_or_nothing_fitting_changes_nothing() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 500.0, 500.0);
        small_sheet(&mut app, 100.0, 100.0);
        app.select(vec![]);
        assert!(app.nest_rectangles().is_none());
        assert!(app.nest_shapes().is_none());
        assert_eq!(app.nest_fill(), 0);
        let undo = app.engine.undo_label();
        app.select(vec![a]);
        assert!(app.nest_rectangles().is_none());
        assert!(app.nest_shapes().is_none());
        assert_eq!(app.nest_fill(), 0);
        assert_eq!(app.engine.undo_label(), undo);
        // Bad settings do not panic.
        app.sc_nest.sheet.width = f64::NAN;
        app.sc_nest.tolerance = f64::INFINITY;
        app.sc_nest.time_limit = f64::NAN;
        assert!(app.nest_shapes().is_none());
    }

    /// The section draws every tool, before and after a run, and shows
    /// the material on the drawing.
    #[test]
    fn the_section_draws_each_tool() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let a = rect(&mut app, 300.0, 300.0, 20.0, 20.0);
        app.select(vec![a]);
        let frame = |app: &mut App| {
            app.docker_preview.clear();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::signcut_nest::section(app, ui));
            out.textures_delta.clear();
            // egui may run a frame more than once.
            assert!(!app.docker_preview.is_empty());
        };
        for tool in [Tool::Rects, Tool::Shapes, Tool::Fill] {
            app.sc_nest.tool = tool;
            frame(&mut app);
            app.sc_nest.sheet.roll = true;
            frame(&mut app);
            app.sc_nest.sheet.roll = false;
        }
        app.sc_nest.tool = Tool::Rects;
        assert!(app.nest_rectangles().is_some());
        frame(&mut app);
        let page = app.page_size();
        let shown = app.docker_preview[0].bounding_box();
        assert!((shown.width() - page.width).abs() < 1e-6);
    }

    #[test]
    fn the_page_size_is_the_sheet_by_default() {
        let app = App::headless();
        let s = app.nest_sheet();
        let p = app.page_size();
        assert_eq!((s.width, s.height), (p.width, p.height));
    }
}
