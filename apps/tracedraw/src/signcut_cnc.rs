//! Sign & Cut, CNC and Laser: settings and operations (G-code, hot wire paths, finger joints, door panels and centre lines).
//! The geometry is in `tracedraw_signcut::cnc`; the panel section is
//! `ui/signcut_cnc.rs`. See docs/behavior/sign-and-cut-cnc.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut::Section;
use egui::Ui;
use tracedraw_core::geometry::{Affine, BezPath, Point};
use tracedraw_core::{Color, Command, Fill, Shape, ShapeId, ShapeKind, Stroke};
use tracedraw_signcut::cnc::gcode::{self, Cut, Job, Machine, Summary, Tool};

/// The CNC and Laser section's settings and last results.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Machines and the one used.
    pub machines: Vec<Machine>,
    pub machine: usize,
    /// Tools and the one shown in the panel.
    pub tools: Vec<Tool>,
    pub tool: usize,
    /// Jobs by outline colour, cut in this order.
    pub jobs: Vec<Job>,
    /// Show the tool paths on the drawing.
    pub preview: bool,
    /// What the last export or preview amounted to.
    pub last: Option<Summary>,
    /// Tool paths of the preview and what they were made from.
    preview_cache: Option<(PreviewKey, Vec<BezPath>)>,
    /// Hot wire: start point on the page (mm) and the tool of its G-code.
    pub wire_start_x: f64,
    pub wire_start_y: f64,
    pub wire_tool: usize,
}

/// What the preview's tool paths depend on.
#[derive(Debug, Clone, PartialEq)]
struct PreviewKey {
    revision: u64,
    selection: Vec<ShapeId>,
    page: tracedraw_core::PageId,
    machine: Option<Machine>,
    tools: Vec<Tool>,
    jobs: Vec<Job>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            machines: vec![Machine::router(), Machine::laser(), Machine::hot_wire()],
            machine: 0,
            tools: vec![
                Tool::end_mill(3.0),
                Tool::end_mill(6.0),
                Tool::laser_beam(),
                Tool::wire(),
            ],
            tool: 0,
            jobs: Vec::new(),
            preview: true,
            last: None,
            preview_cache: None,
            wire_start_x: 0.0,
            wire_start_y: 0.0,
            wire_tool: 3,
        }
    }
}

/// The outline colour of an object: its outline's, else its plain
/// fill's, else black.
fn outline_color(s: &Shape) -> [u8; 3] {
    match (&s.stroke, &s.fill) {
        (Some(st), _) => st.color.to_rgb8(),
        (None, Fill::Solid(c)) => c.to_rgb8(),
        _ => [0, 0, 0],
    }
}

/// Each drawn object of `shape` (groups opened, a clip frame by its
/// frame) with its outline colour, in page space.
fn colored(shape: &Shape, parent: Affine, out: &mut Vec<([u8; 3], BezPath)>) {
    let t = parent * shape.transform;
    match &shape.kind {
        ShapeKind::Group { children } => {
            for c in children {
                colored(c, t, out);
            }
        }
        ShapeKind::ClipFrame { frame, .. } => colored(frame, t, out),
        _ => {
            let p = t * shape.local_path();
            if !p.elements().is_empty() {
                out.push((outline_color(shape), p));
            }
        }
    }
}

/// `#rrggbb` of a colour.
pub fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

impl App {
    /// The targets' outlines with their outline colours, in page space.
    pub fn cnc_colored_outlines(&self) -> Vec<([u8; 3], BezPath)> {
        let doc = self.doc();
        let mut out = Vec::new();
        for id in self.signcut_targets() {
            if let Some(s) = doc.shape_on_page(id) {
                colored(&s, Affine::IDENTITY, &mut out);
            }
        }
        out
    }

    /// Add a job for each outline colour of the targets that has none.
    /// Returns how many were added.
    pub fn cnc_find_colors(&mut self) -> usize {
        let mut added = 0;
        for (c, _) in self.cnc_colored_outlines() {
            if !self.sc_cnc.jobs.iter().any(|j| j.color == c) {
                self.sc_cnc.jobs.push(Job::new(c));
                added += 1;
            }
        }
        self.status = trf("signcut.cnc_colors_found", &[("n", &added.to_string())]);
        added
    }

    /// The machine in use.
    pub fn cnc_machine(&self) -> Option<&Machine> {
        self.sc_cnc.machines.get(self.sc_cnc.machine)
    }

    /// The tool paths of every switched-on job, in cutting order, from
    /// the machine's zero.
    pub fn cnc_plan(&self) -> Vec<Cut> {
        let Some(m) = self.cnc_machine() else {
            return Vec::new();
        };
        let size = self.page_size();
        let mut pos = Point::ZERO - m.origin.offset(size.width, size.height);
        let outlines = self.cnc_colored_outlines();
        let mut cuts = Vec::new();
        for job in self.sc_cnc.jobs.iter().filter(|j| j.enabled) {
            let Some(tool) = self.sc_cnc.tools.get(job.tool) else {
                continue;
            };
            let paths: Vec<BezPath> = outlines
                .iter()
                .filter(|(c, _)| *c == job.color)
                .map(|(_, p)| p.clone())
                .collect();
            if paths.is_empty() {
                continue;
            }
            let (more, end) = gcode::plan_job(job, tool, &paths, pos);
            pos = end;
            cuts.extend(more);
        }
        cuts
    }

    /// The G-code of the jobs, and what it amounts to; `None` when there
    /// is nothing to cut.
    pub fn cnc_gcode(&mut self) -> Option<String> {
        let cuts = self.cnc_plan();
        let m = self.cnc_machine()?.clone();
        let size = self.page_size();
        let s = gcode::summary(&m, &cuts, size.width, size.height);
        self.sc_cnc.last = Some(s);
        if cuts.is_empty() {
            self.status = tr("signcut.cnc_nothing");
            return None;
        }
        Some(gcode::emit(
            &m,
            &self.sc_cnc.tools,
            &cuts,
            size.width,
            size.height,
        ))
    }

    /// Save the G-code of the jobs to a `.nc` file.
    pub fn cnc_export_gcode(&mut self) {
        let Some(text) = self.cnc_gcode() else {
            return;
        };
        save_gcode(self, text, "job.nc");
    }

    /// The tool paths to show on the drawing (page space), made again
    /// only when the drawing, the selection or the settings change.
    pub fn cnc_preview(&mut self) -> Vec<BezPath> {
        let key = PreviewKey {
            revision: self.engine.revision(),
            selection: self.selection.clone(),
            page: self.page,
            machine: self.cnc_machine().cloned(),
            tools: self.sc_cnc.tools.clone(),
            jobs: self.sc_cnc.jobs.clone(),
        };
        if let Some((k, paths)) = &self.sc_cnc.preview_cache {
            if *k == key {
                return paths.clone();
            }
        }
        let cuts = self.cnc_plan();
        if let Some(m) = self.cnc_machine() {
            let size = self.page_size();
            self.sc_cnc.last = Some(gcode::summary(m, &cuts, size.width, size.height));
        }
        let paths: Vec<BezPath> = cuts.iter().map(|c| polyline(&c.preview(), false)).collect();
        self.sc_cnc.preview_cache = Some((key, paths.clone()));
        paths
    }
}

/// Width of the hairline outline given to new curves (mm).
pub const HAIRLINE: f64 = 0.0762;

impl App {
    /// The targets' outlines in page space, one list per object.
    fn cnc_outlines(&self) -> Vec<BezPath> {
        self.cnc_colored_outlines()
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }

    /// Add `paths` (page space) as new curves with a hairline outline of
    /// `color` and no fill, grouped when there are several and `group`
    /// is on, in one undo step named `label`. The new objects are
    /// selected; returns their ids (the group's when grouped).
    pub(crate) fn cnc_add_curves(
        &mut self,
        label: &'static str,
        paths: Vec<(BezPath, bool)>,
        color: Color,
        group: bool,
    ) -> Vec<ShapeId> {
        let paths: Vec<(BezPath, bool)> = paths
            .into_iter()
            .filter(|(p, _)| !p.elements().is_empty())
            .collect();
        let Some(layer) = self.active_layer() else {
            return Vec::new();
        };
        if paths.is_empty() {
            return Vec::new();
        }
        let mut shapes: Vec<Shape> = paths
            .into_iter()
            .map(|(path, closed)| {
                let id = self.engine.new_shape_id();
                let mut s = Shape::new(id, ShapeKind::Path { path, closed });
                s.fill = Fill::None;
                s.stroke = Some(Stroke::new(color, HAIRLINE));
                s
            })
            .collect();
        if group && shapes.len() > 1 {
            let gid = self.engine.new_shape_id();
            let children = std::mem::take(&mut shapes);
            shapes.push(Shape::new(gid, ShapeKind::Group { children }));
        }
        let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        let cmds: Vec<Command> = shapes
            .into_iter()
            .map(|shape| Command::AddShape { layer, shape })
            .collect();
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return Vec::new();
        }
        self.select(ids.clone());
        ids
    }

    /// The hot wire path through the targets' closed outlines from the
    /// start point.
    pub fn cnc_wire_points(&self) -> Option<Vec<Point>> {
        let start = Point::new(self.sc_cnc.wire_start_x, self.sc_cnc.wire_start_y);
        tracedraw_signcut::cnc::hotwire::wire_path(&self.cnc_outlines(), start)
    }

    /// Add the hot wire path as one open curve. Returns its id.
    pub fn cnc_hot_wire(&mut self) -> Option<ShapeId> {
        let Some(pts) = self.cnc_wire_points() else {
            self.status = tr("signcut.cnc_no_closed");
            return None;
        };
        let len = tracedraw_signcut::cnc::polyline_length(&pts, false);
        let ids = self.cnc_add_curves(
            "Hot Wire Path",
            vec![(polyline(&pts, false), false)],
            Color::rgb8(230, 40, 20),
            false,
        );
        let u = self.units;
        self.status = trf(
            "signcut.cnc_wire_done",
            &[("len", &format!("{:.2} {}", u.world_from_mm(len), u.short()))],
        );
        ids.first().copied()
    }

    /// The hot wire path as G-code in X and Y for the machine in use.
    pub fn cnc_wire_gcode(&mut self) -> Option<String> {
        let Some(pts) = self.cnc_wire_points() else {
            self.status = tr("signcut.cnc_no_closed");
            return None;
        };
        let m = self.cnc_machine()?.clone();
        let tool = self.sc_cnc.tools.get(self.sc_cnc.wire_tool)?.clone();
        let size = self.page_size();
        Some(gcode::emit_xy(&m, &tool, &pts, size.width, size.height))
    }

    /// Save the hot wire G-code to a file.
    pub fn cnc_export_wire_gcode(&mut self) {
        let Some(text) = self.cnc_wire_gcode() else {
            return;
        };
        save_gcode(self, text, "hot-wire.nc");
    }
}

/// Offer `text` for saving as a G-code file named `name`.
fn save_gcode(app: &mut App, text: String, name: &str) {
    crate::files::Dialog::new()
        .add_filter("G-code", &["nc", "gcode", "ngc", "tap"])
        .set_file_name(name)
        .save_file(app, move |app, path| {
            app.status = match crate::files::write(&path, text) {
                Ok(()) => trf("status.exported", &[("path", &path.display().to_string())]),
                Err(e) => e.to_string(),
            };
        });
}

/// A polyline as a path.
pub(crate) fn polyline(pts: &[Point], closed: bool) -> BezPath {
    let mut p = BezPath::new();
    let mut it = pts.iter().filter(|q| q.x.is_finite() && q.y.is_finite());
    if let Some(first) = it.next() {
        p.move_to(*first);
        for q in it {
            p.line_to(*q);
        }
        if closed {
            p.close_path();
        }
    }
    p
}

/// Tools > Sign & Cut > CNC and Laser.
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::ui::menus::item;
    let doc = app.has_document() && !app.show_welcome;
    if item(ui, "menu.tools.signcut_cnc_gcode", "", doc) {
        app.open_signcut(Section::Cnc);
        if app.sc_cnc.jobs.is_empty() {
            app.cnc_find_colors();
        }
    }
    if item(ui, "menu.tools.signcut_cnc_hot_wire", "", doc) {
        app.open_signcut(Section::Cnc);
        app.cnc_hot_wire();
    }
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

    fn outline(app: &mut App, id: ShapeId, c: Color) {
        app.run(Command::SetStroke {
            shapes: vec![id],
            stroke: Some(Stroke::new(c, 0.1)),
        });
    }

    #[test]
    fn gcode_jobs_by_outline_colour() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 20.0, 20.0);
        let b = rect(&mut app, 50.0, 10.0, 20.0, 20.0);
        outline(&mut app, a, Color::rgb8(255, 0, 0));
        outline(&mut app, b, Color::rgb8(0, 0, 255));
        assert_eq!(app.cnc_find_colors(), 2);
        assert_eq!(app.cnc_find_colors(), 0);
        assert_eq!(app.sc_cnc.jobs.len(), 2);
        // Red engraved on the line with the first tool, blue cut out
        // with the second.
        app.sc_cnc.jobs[0].side = gcode::Side::On;
        app.sc_cnc.jobs[0].depth = 0.5;
        app.sc_cnc.jobs[1].tool = 1;
        app.sc_cnc.jobs[1].depth = 6.0;
        let cuts = app.cnc_plan();
        assert_eq!(cuts.len(), 2);
        assert_eq!(cuts[0].tool, 0);
        assert_eq!(cuts[1].tool, 1);
        // The 6 mm tool's 6 mm depth in two passes of 3.
        assert_eq!(cuts[1].depths, vec![-3.0, -6.0]);
        let g = app.cnc_gcode().expect("G-code");
        assert!(g.starts_with("(Router)\nG21\nG90\nG17\n"));
        assert!(g.contains("T1 M6") && g.contains("T2 M6"));
        assert!(g.ends_with("M5\nM30\n"));
        let s = app.sc_cnc.last.expect("summary");
        assert_eq!(s.cuts, 2);
        assert!(s.on_table);
        // A job switched off is left out; the preview follows.
        assert_eq!(app.cnc_preview().len(), 2);
        app.sc_cnc.jobs[1].enabled = false;
        assert_eq!(app.cnc_preview().len(), 1);
        assert_eq!(app.cnc_plan().len(), 1);
        // Nothing to cut: no G-code. The drawing is never changed.
        app.sc_cnc.jobs.clear();
        assert!(app.cnc_gcode().is_none());
        assert!(app.engine.undo_label() != Some("G-code"));
    }

    #[test]
    fn gcode_follows_the_selection_and_opens_groups() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 20.0, 0.0, 10.0, 10.0);
        rect(&mut app, 40.0, 0.0, 10.0, 10.0);
        app.select(vec![a, b]);
        app.run(Command::Group { shapes: vec![a, b] });
        let group = app
            .page_layers()
            .iter()
            .flat_map(|l| l.shapes.iter())
            .find(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .map(|s| s.id)
            .expect("a group");
        app.select(vec![group]);
        app.cnc_find_colors();
        app.sc_cnc.jobs[0].side = gcode::Side::On;
        // The group's two members, not the third rectangle.
        assert_eq!(app.cnc_plan().len(), 2);
        app.select(vec![]);
        assert_eq!(app.cnc_plan().len(), 3);
    }

    #[test]
    fn hot_wire_path_is_one_open_curve_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 25.0, 0.0, 10.0, 10.0);
        app.select(vec![a, b]);
        let id = app.cnc_hot_wire().expect("a path");
        assert_eq!(app.selection, vec![id]);
        let s = app.doc().shape_on_page(id).expect("shape");
        assert!(matches!(s.kind, ShapeKind::Path { closed: false, .. }));
        let len = tracedraw_signcut::measure::length(&s.page_path());
        // Two outlines once, 10 mm in and 5 mm between, each twice.
        assert!((len - (80.0 + 2.0 * 15.0)).abs() < 1e-6, "{len}");
        assert_eq!(app.engine.undo_label(), Some("Hot Wire Path"));
        // XY G-code: no Z moves.
        app.select(vec![a, b]);
        let g = app.cnc_wire_gcode().expect("G-code");
        assert!(!g.contains('Z') && g.contains("G1 X"));
        // Nothing closed: nothing added.
        let mut line = BezPath::new();
        line.move_to((0.0, 50.0));
        line.line_to((10.0, 50.0));
        let l = app
            .new_shape(ShapeKind::Path {
                path: line,
                closed: false,
            })
            .expect("line");
        app.select(vec![l]);
        assert!(app.cnc_hot_wire().is_none());
    }
}
