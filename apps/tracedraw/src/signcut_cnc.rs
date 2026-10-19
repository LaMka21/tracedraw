//! Sign & Cut, CNC and Laser: settings and operations (G-code, hot wire
//! paths, finger joints, door panels and centre lines). Each operation
//! that changes the drawing is split into a compute step (the windows'
//! previews; the drawing is not touched) and a commit step (one undo
//! step). The geometry is in `tracedraw_signcut::cnc`; the windows are
//! `ui/signcut_win_cnc.rs`. See docs/behavior/sign-and-cut-cnc.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_tools::ToolId;
use egui::Ui;
use tracedraw_core::geometry::{Affine, BezPath, Point};
use tracedraw_core::{Color, Command, Fill, PageId, Shape, ShapeId, ShapeKind, Stroke};
use tracedraw_signcut::cnc::centerline::CenterSpec;
use tracedraw_signcut::cnc::door::{Door, DoorSpec};
use tracedraw_signcut::cnc::fingerbox::{BoxSpec, Panel};
use tracedraw_signcut::cnc::gcode::{self, Cut, Job, Machine, Summary, Tool};
use tracedraw_signcut::cnc::joints::{self, JointSpec};

/// The CNC and Laser tools' settings, last results and window state.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The machine the G-code is written for.
    pub machine: Machine,
    /// Tools and the one shown in the Tools tab.
    pub tools: Vec<Tool>,
    pub tool: usize,
    /// Jobs by outline colour, cut in this order.
    pub jobs: Vec<Job>,
    /// Show the tool paths on the drawing.
    pub preview: bool,
    /// What the last export or preview amounted to.
    pub last: Option<Summary>,
    /// Tool paths of the preview and what they were made from.
    preview_cache: Option<(PreviewKey, PlanPreview)>,
    /// Hot wire: start point on the page (mm), the machine and the tool
    /// of its G-code.
    pub wire_start_x: f64,
    pub wire_start_y: f64,
    pub wire_machine: Machine,
    pub wire_tool: usize,
    /// Finger-joint box.
    pub box_spec: BoxSpec,
    /// Finger joints on edges; with `joint_pair` two objects are joined
    /// at their nearest edges, else every long straight edge is cut.
    pub joint: JointSpec,
    pub joint_pair: bool,
    /// Door panels.
    pub door: DoorSpec,
    /// Centre lines.
    pub centre: CenterSpec,
    /// The windows' tabs and previews.
    pub win: WinState,
}

/// What the preview's tool paths depend on.
#[derive(Debug, Clone, PartialEq)]
struct PreviewKey {
    revision: u64,
    selection: Vec<ShapeId>,
    page: PageId,
    machine: Machine,
    tools: Vec<Tool>,
    jobs: Vec<Job>,
}

/// The tool paths of the jobs for a preview, each in its job's colour,
/// and the time they take at the tools' feeds (minutes).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlanPreview {
    pub paths: Vec<([u8; 3], BezPath)>,
    pub minutes: f64,
}

/// The G-code window's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GcodeTab {
    #[default]
    Machine,
    Tools,
    Jobs,
}

/// What a preview was made from: the drawing, the selection, the page
/// and the settings. A preview is current while its stamp is.
#[derive(Debug, Clone, PartialEq)]
pub struct Stamp<S> {
    revision: u64,
    selection: Vec<ShapeId>,
    page: PageId,
    spec: S,
}

/// A preview with what it was made from.
pub type Cached<S, T> = Option<(Stamp<S>, T)>;

/// Finger joints worked out: each changed object with its new outline
/// (page space) and the edges that take fingers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Joined {
    pub changed: Vec<(ShapeId, BezPath)>,
    pub edges: Vec<(Point, Point)>,
}

/// The CNC windows' state: the open tab and the previews.
#[derive(Debug, Clone, Default)]
pub struct WinState {
    pub gcode_tab: GcodeTab,
    /// The job shown in the Jobs tab.
    pub job: usize,
    /// The colours were looked for when the G-code window first drew.
    pub found: bool,
    pub centre: Cached<CenterSpec, Vec<(BezPath, bool)>>,
    pub wire: Cached<(f64, f64), Option<Vec<Point>>>,
    pub joints: Cached<(JointSpec, bool), Result<Joined, String>>,
    pub door: Cached<DoorSpec, Option<Door>>,
    pub boxes: Cached<BoxSpec, Option<Vec<(Panel, BezPath)>>>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            machine: Machine::router(),
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
            wire_machine: Machine::hot_wire(),
            wire_tool: 3,
            box_spec: BoxSpec::default(),
            joint: JointSpec::default(),
            joint_pair: true,
            door: DoorSpec::default(),
            centre: CenterSpec::default(),
            win: WinState::default(),
        }
    }
}

impl Settings {
    /// Remove tool `i` (one always stays); jobs that used it take the
    /// first tool, the others keep theirs.
    pub fn remove_tool(&mut self, i: usize) -> bool {
        if self.tools.len() <= 1 || i >= self.tools.len() {
            return false;
        }
        self.tools.remove(i);
        let fix = |t: &mut usize| {
            if *t == i {
                *t = 0;
            } else if *t > i {
                *t -= 1;
            }
        };
        for j in &mut self.jobs {
            fix(&mut j.tool);
        }
        fix(&mut self.wire_tool);
        fix(&mut self.tool);
        true
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
    pub fn cnc_machine(&self) -> &Machine {
        &self.sc_cnc.machine
    }

    /// The tool paths of every switched-on job, in cutting order, from
    /// the machine's zero.
    pub fn cnc_plan(&self) -> Vec<Cut> {
        self.cnc_plan_colored()
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }

    /// As [`App::cnc_plan`], each tool path with its job's colour.
    pub fn cnc_plan_colored(&self) -> Vec<([u8; 3], Cut)> {
        let m = self.cnc_machine();
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
            cuts.extend(more.into_iter().map(|c| (job.color, c)));
        }
        cuts
    }

    /// The G-code of the jobs, and what it amounts to; `None` when there
    /// is nothing to cut.
    pub fn cnc_gcode(&mut self) -> Option<String> {
        let cuts = self.cnc_plan();
        let m = self.cnc_machine().clone();
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
        self.cnc_preview_colored()
            .paths
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }

    /// The tool paths with their jobs' colours and the time they take,
    /// made again only when the drawing, the selection or the settings
    /// change; `last` follows them.
    pub fn cnc_preview_colored(&mut self) -> PlanPreview {
        let key = PreviewKey {
            revision: self.engine.revision(),
            selection: self.selection.clone(),
            page: self.page,
            machine: self.cnc_machine().clone(),
            tools: self.sc_cnc.tools.clone(),
            jobs: self.sc_cnc.jobs.clone(),
        };
        if let Some((k, p)) = &self.sc_cnc.preview_cache {
            if *k == key {
                return p.clone();
            }
        }
        let cuts = self.cnc_plan_colored();
        let size = self.page_size();
        let plain: Vec<Cut> = cuts.iter().map(|(_, c)| c.clone()).collect();
        self.sc_cnc.last = Some(gcode::summary(
            self.cnc_machine(),
            &plain,
            size.width,
            size.height,
        ));
        let mut minutes = 0.0;
        for c in &plain {
            if let Some(t) = self.sc_cnc.tools.get(c.tool) {
                if t.feed.is_finite() && t.feed > 0.0 {
                    minutes += c.length() * c.depths.len().max(1) as f64 / t.feed;
                }
            }
        }
        let p = PlanPreview {
            paths: cuts
                .iter()
                .map(|(col, c)| (*col, polyline(&c.preview(), false)))
                .collect(),
            minutes,
        };
        self.sc_cnc.preview_cache = Some((key, p.clone()));
        p
    }

    /// A stamp of what the windows' previews depend on now.
    pub fn cnc_stamp<S>(&self, spec: S) -> Stamp<S> {
        Stamp {
            revision: self.engine.revision(),
            selection: self.selection.clone(),
            page: self.page,
            spec,
        }
    }
}

/// Width of the hairline outline given to new curves (mm).
pub const HAIRLINE: f64 = 0.0762;

impl App {
    /// The targets' outlines in page space, one list per object.
    pub fn cnc_outlines(&self) -> Vec<BezPath> {
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
        let paths = paths.into_iter().map(|(p, c)| (p, c, color)).collect();
        self.cnc_add_colored(label, paths, group)
    }

    /// As [`App::cnc_add_curves`], each curve with its own outline colour.
    pub(crate) fn cnc_add_colored(
        &mut self,
        label: &'static str,
        paths: Vec<(BezPath, bool, Color)>,
        group: bool,
    ) -> Vec<ShapeId> {
        let paths: Vec<(BezPath, bool, Color)> = paths
            .into_iter()
            .filter(|(p, _, _)| !p.elements().is_empty())
            .collect();
        let Some(layer) = self.active_layer() else {
            return Vec::new();
        };
        if paths.is_empty() {
            return Vec::new();
        }
        let mut shapes: Vec<Shape> = paths
            .into_iter()
            .map(|(path, closed, color)| {
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
    #[cfg(test)]
    pub fn cnc_hot_wire(&mut self) -> Option<ShapeId> {
        let pts = self.cnc_wire_points();
        self.cnc_wire_commit(pts.as_deref())
    }

    /// Add `pts` (a hot wire path worked out before) as one open curve
    /// with a red hairline, one undo step. Returns its id.
    pub fn cnc_wire_commit(&mut self, pts: Option<&[Point]>) -> Option<ShapeId> {
        let Some(pts) = pts else {
            self.status = tr("signcut.cnc_no_closed");
            return None;
        };
        let len = tracedraw_signcut::cnc::polyline_length(pts, false);
        let ids = self.cnc_add_curves(
            "Hot Wire Path",
            vec![(polyline(pts, false), false)],
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

    /// The hot wire path as G-code in X and Y for the hot wire machine.
    pub fn cnc_wire_gcode(&mut self) -> Option<String> {
        let Some(pts) = self.cnc_wire_points() else {
            self.status = tr("signcut.cnc_no_closed");
            return None;
        };
        let m = self.sc_cnc.wire_machine.clone();
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

impl App {
    /// The panels of the finger-joint box laid out from the page's
    /// bottom left plus the gap; `None` when the sizes cannot make a box.
    pub fn cnc_box_compute(&self) -> Option<Vec<(Panel, BezPath)>> {
        let spec = self.sc_cnc.box_spec;
        let g = if spec.gap.is_finite() {
            spec.gap.max(0.0)
        } else {
            0.0
        };
        tracedraw_signcut::cnc::fingerbox::box_panels(&spec, Point::new(g, g))
    }

    /// Add the panels of the finger-joint box, grouped. Returns the
    /// group's id.
    #[cfg(test)]
    pub fn cnc_finger_box(&mut self) -> Option<ShapeId> {
        let panels = self.cnc_box_compute();
        self.cnc_box_commit(panels)
    }

    /// Add `panels` (worked out before) as closed red curves in one
    /// group, one undo step.
    pub fn cnc_box_commit(&mut self, panels: Option<Vec<(Panel, BezPath)>>) -> Option<ShapeId> {
        let Some(panels) = panels else {
            self.status = tr("signcut.cnc_box_bad");
            return None;
        };
        let paths = panels.into_iter().map(|(_, p)| (p, true)).collect();
        let ids = self.cnc_add_curves("Finger-Joint Box", paths, CUT_RED, true);
        ids.first().copied()
    }
}

impl App {
    /// The targets whose outline can be rewritten (curves and basic
    /// shapes without live effects, unlocked; groups opened), each in
    /// page space.
    fn cnc_editable(&self) -> Vec<Shape> {
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
        let mut ids = Vec::new();
        for id in self.signcut_targets() {
            if let Some(s) = doc.find_shape(id) {
                walk(s, &mut ids);
            }
        }
        ids.iter().filter_map(|id| doc.shape_on_page(*id)).collect()
    }

    /// The objects that can take finger joints, in page space (for the
    /// preview).
    pub fn cnc_joint_targets(&self) -> Vec<BezPath> {
        self.cnc_editable().iter().map(|s| s.page_path()).collect()
    }

    /// Work the finger joints out: into the nearest straight edges of
    /// two objects (mating patterns), or into every long straight edge
    /// of each object. The drawing is not changed; `Err` says why
    /// nothing would change.
    pub fn cnc_joints_compute(&self) -> Result<Joined, String> {
        let shapes = self.cnc_editable();
        let spec = self.sc_cnc.joint;
        let mut out = Joined::default();
        if self.sc_cnc.joint_pair {
            let [a, b] = &shapes[..] else {
                return Err(tr("signcut.cnc_joint_two"));
            };
            if let Some((pa, pb, ea, eb)) = joints::between(&a.page_path(), &b.page_path(), &spec) {
                out.changed.push((a.id, pa));
                out.changed.push((b.id, pb));
                out.edges.push((ea.a, ea.b));
                out.edges.push((eb.a, eb.b));
            }
        } else {
            for s in &shapes {
                let path = s.page_path();
                if let Some((p, _)) = joints::on_all_edges(&path, &spec) {
                    out.changed.push((s.id, p));
                    out.edges.extend(
                        joints::straight_edges(&path, spec.min_length)
                            .into_iter()
                            .map(|e| (e.a, e.b)),
                    );
                }
            }
        }
        if out.changed.is_empty() {
            return Err(tr("signcut.cnc_joint_none"));
        }
        Ok(out)
    }

    /// Cut finger joints. One undo step; returns how many objects
    /// changed.
    #[cfg(test)]
    pub fn cnc_finger_joints(&mut self) -> usize {
        let j = self.cnc_joints_compute();
        self.cnc_joints_commit(j)
    }

    /// Replace the outlines of the objects of `joined` (worked out
    /// before) in one undo step. Returns how many objects changed.
    pub fn cnc_joints_commit(&mut self, joined: Result<Joined, String>) -> usize {
        let joined = match joined {
            Ok(j) => j,
            Err(e) => {
                self.status = e;
                return 0;
            }
        };
        let doc = self.doc();
        let cmds: Vec<Command> = joined
            .changed
            .iter()
            .filter_map(|(id, p)| {
                let s = doc.shape_on_page(*id)?;
                let det = s.transform.determinant();
                (det.is_finite() && det.abs() > 1e-12).then(|| Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Path {
                        path: s.transform.inverse() * p.clone(),
                        closed: true,
                    },
                })
            })
            .collect();
        if cmds.is_empty() {
            self.status = tr("signcut.cnc_joint_none");
            return 0;
        }
        let n = cmds.len();
        if let Err(e) = self.engine.run_batch("Finger Joints", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        n
    }
}

impl App {
    /// The door's outer box: the one selected object's, else the
    /// panel's size centred on the page.
    pub fn cnc_door_rect(&self) -> tracedraw_core::geometry::Rect {
        let spec = self.sc_cnc.door;
        let doc = self.doc();
        match &self.selection[..] {
            [id] => doc.shape_on_page(*id).map(|s| s.bounds()),
            _ => None,
        }
        .unwrap_or_else(|| {
            let c = self.page_rect().center();
            tracedraw_core::geometry::Rect::from_center_size(c, (spec.width, spec.height))
        })
    }

    /// Work the door out in its box; `None` when the frame leaves no
    /// inner panel.
    pub fn cnc_door_compute(&self) -> Option<Door> {
        tracedraw_signcut::cnc::door::door(&self.sc_cnc.door, self.cnc_door_rect())
    }

    /// Draw a door panel: in the box of the one selected object, else
    /// of the panel's size centred on the page. Outer and inner outlines
    /// red, grooves blue, grouped, one undo step. Returns the group.
    #[cfg(test)]
    pub fn cnc_door(&mut self) -> Option<ShapeId> {
        let d = self.cnc_door_compute();
        self.cnc_door_commit(d)
    }

    /// Add a door worked out before, one undo step.
    pub fn cnc_door_commit(&mut self, d: Option<Door>) -> Option<ShapeId> {
        let Some(d) = d else {
            self.status = tr("signcut.cnc_door_bad");
            return None;
        };
        let mut paths = vec![(d.outer, true, CUT_RED), (d.inner, true, CUT_RED)];
        for g in d.grooves {
            paths.push((g, true, GROOVE_BLUE));
        }
        self.cnc_add_colored("Door Panel", paths, true)
            .first()
            .copied()
    }
}

impl App {
    /// The centre lines of the targets' closed shapes (each drawn object
    /// on its own, holes by even-odd), page space, with whether each
    /// closes. The drawing is not changed.
    pub fn cnc_center_compute(&self) -> Vec<(BezPath, bool)> {
        let spec = self.sc_cnc.centre;
        let mut paths = Vec::new();
        for p in self.cnc_outlines() {
            for (pts, closed) in tracedraw_signcut::cnc::centerline::center_lines(&p, &spec) {
                paths.push((polyline(&pts, closed), closed));
            }
        }
        paths
    }

    /// Add the centre lines of the targets' closed shapes as curves with
    /// a blue hairline, grouped, in one undo step. Returns how many
    /// lines.
    #[cfg(test)]
    pub fn cnc_center_lines(&mut self) -> usize {
        let paths = self.cnc_center_compute();
        self.cnc_center_commit(paths)
    }

    /// Add centre lines worked out before, one undo step.
    pub fn cnc_center_commit(&mut self, paths: Vec<(BezPath, bool)>) -> usize {
        let n = paths.len();
        if n == 0 {
            self.status = tr("signcut.cnc_no_center");
            return 0;
        }
        if self
            .cnc_add_curves("Centre Line", paths, GROOVE_BLUE, true)
            .is_empty()
        {
            return 0;
        }
        self.status = trf("signcut.cnc_center_done", &[("n", &n.to_string())]);
        n
    }
}

/// Outline colour of generated engraving and groove lines.
pub const GROOVE_BLUE: Color = Color::Rgb {
    r: 0.1,
    g: 0.2,
    b: 0.9,
};

/// Outline colour of generated cut lines.
pub const CUT_RED: Color = Color::Rgb {
    r: 0.9,
    g: 0.1,
    b: 0.1,
};

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

/// Tools > Sign & Cut > CNC and Laser: each item opens its tool window.
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::ui::menus::item;
    let doc = app.has_document() && !app.show_welcome;
    if item(ui, "menu.tools.signcut_cnc_gcode", "", doc) {
        app.open_signcut(crate::signcut::Section::Cnc);
    }
    for (key, t) in [
        ("menu.tools.signcut_cnc_hot_wire", ToolId::HotWire),
        ("menu.tools.signcut_cnc_box", ToolId::FingerJointBox),
        ("menu.tools.signcut_cnc_joints", ToolId::FingerJoints),
        ("menu.tools.signcut_cnc_door", ToolId::DoorPanels),
        ("menu.tools.signcut_cnc_center", ToolId::CentreLine),
    ] {
        if item(ui, key, "", doc) {
            app.signcut_tool(t);
        }
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
    fn gcode_preview_has_job_colours_and_a_time() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 20.0, 20.0);
        outline(&mut app, a, Color::rgb8(255, 0, 0));
        app.cnc_find_colors();
        app.sc_cnc.jobs[0].side = gcode::Side::On;
        app.sc_cnc.jobs[0].depth = 3.0;
        let p = app.cnc_preview_colored();
        assert_eq!(p.paths.len(), 1);
        assert_eq!(p.paths[0].0, [255, 0, 0]);
        // 80 mm in two passes (deepest pass 1.5 mm) at 1200 mm/min.
        assert!((p.minutes - 160.0 / 1200.0).abs() < 1e-9, "{}", p.minutes);
    }

    #[test]
    fn removing_a_tool_keeps_the_jobs_on_their_tools() {
        let mut s = Settings::default();
        let mut j = Job::new([0, 0, 0]);
        j.tool = 2;
        s.jobs.push(j.clone());
        j.tool = 1;
        s.jobs.push(j);
        assert!(s.remove_tool(1));
        assert_eq!(s.tools.len(), 3);
        assert_eq!(s.jobs[0].tool, 1);
        assert_eq!(s.jobs[1].tool, 0);
        assert_eq!(s.wire_tool, 2);
        assert!(!s.remove_tool(9));
        while s.tools.len() > 1 {
            assert!(s.remove_tool(0));
        }
        assert!(!s.remove_tool(0));
    }

    #[test]
    fn settings_round_trip_as_presets() {
        let mut m = Machine::laser();
        m.decimals = 4;
        let v = serde_json::to_value(&m).expect("json");
        assert_eq!(serde_json::from_value::<Machine>(v).ok(), Some(m));
        // Missing fields take their defaults.
        let b: BoxSpec = serde_json::from_str(r#"{"width": 300.0}"#).expect("box");
        assert_eq!(b.width, 300.0);
        assert_eq!(b.thickness, BoxSpec::default().thickness);
        let d: DoorSpec = serde_json::from_str(r#"{"inner": "Arch"}"#).expect("door");
        assert_eq!(d.inner, tracedraw_signcut::cnc::door::InnerShape::Arch);
        let j: Job = serde_json::from_str("{}").expect("job");
        assert_eq!(j, Job::default());
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
        assert!(g.starts_with("(Hot wire)"));
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

    #[test]
    fn finger_box_is_one_group_of_panels_in_one_step() {
        let mut app = App::headless();
        app.sc_cnc.box_spec.open_top = true;
        let g = app.cnc_finger_box().expect("a box");
        let s = app.doc().shape_on_page(g).expect("group");
        let ShapeKind::Group { children } = &s.kind else {
            panic!("not a group");
        };
        assert_eq!(children.len(), 5);
        assert!(children
            .iter()
            .all(|c| matches!(c.kind, ShapeKind::Path { closed: true, .. })));
        assert_eq!(app.engine.undo_label(), Some("Finger-Joint Box"));
        assert!(app.engine.undo().is_ok());
        assert!(app.doc().find_shape(g).is_none());
        // Sizes that cannot make a box add nothing.
        app.sc_cnc.box_spec.thickness = 200.0;
        assert!(app.cnc_finger_box().is_none());
    }

    #[test]
    fn finger_joints_rewrite_two_objects_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 100.0, 50.0);
        let b = rect(&mut app, 0.0, 60.0, 100.0, 50.0);
        let c = rect(&mut app, 200.0, 0.0, 90.0, 20.0);
        // Two objects needed for a pair.
        app.select(vec![a]);
        assert_eq!(app.cnc_finger_joints(), 0);
        app.select(vec![a, b]);
        let j = app.cnc_joints_compute().expect("joints");
        assert_eq!(j.edges.len(), 2);
        assert_eq!(app.cnc_finger_joints(), 2);
        assert_eq!(app.engine.undo_label(), Some("Finger Joints"));
        let pa = app.doc().shape_on_page(a).expect("a");
        assert!(matches!(pa.kind, ShapeKind::Path { closed: true, .. }));
        // Still in place, with slots in its top edge.
        let ba = pa.bounds();
        assert!((ba.width() - 100.0).abs() < 1e-6 && (ba.height() - 50.0).abs() < 1e-6);
        let m = tracedraw_signcut::measure::measure(&[vec![pa.page_path()]]);
        assert!((m.area - (5000.0 - 5.0 * 100.0 / 11.0 * 3.0)).abs() < 1e-6);
        // Every long edge of one object.
        app.sc_cnc.joint_pair = false;
        app.sc_cnc.joint.min_length = 50.0;
        app.select(vec![c]);
        assert_eq!(app.cnc_joints_compute().expect("joints").edges.len(), 2);
        assert_eq!(app.cnc_finger_joints(), 1);
        let m = tracedraw_signcut::measure::measure(&[vec![app
            .doc()
            .shape_on_page(c)
            .expect("c")
            .page_path()]]);
        assert!((m.area - (1800.0 - 2.0 * 4.0 * 30.0)).abs() < 1e-6);
    }

    #[test]
    fn door_panel_fills_the_selected_box_or_the_page_centre() {
        let mut app = App::headless();
        app.sc_cnc.door.grooves = 1;
        let g = app.cnc_door().expect("a door");
        let s = app.doc().shape_on_page(g).expect("group");
        let b = s.bounds();
        let c = app.page_rect().center();
        assert!((b.width() - 400.0).abs() < 1e-6 && (b.height() - 600.0).abs() < 1e-6);
        assert!((b.center() - c).hypot() < 1e-6);
        let ShapeKind::Group { children } = &s.kind else {
            panic!("not a group");
        };
        // Outer, inner and one groove, the groove blue.
        assert_eq!(children.len(), 3);
        assert_eq!(
            children[2].stroke.as_ref().map(|s| s.color),
            Some(GROOVE_BLUE)
        );
        assert_eq!(app.engine.undo_label(), Some("Door Panel"));
        // In the box of the selected object.
        let r = rect(&mut app, 10.0, 10.0, 300.0, 500.0);
        app.select(vec![r]);
        let g = app.cnc_door().expect("a door");
        let b = app.doc().shape_on_page(g).expect("group").bounds();
        assert!((b.x0 - 10.0).abs() < 1e-6 && (b.width() - 300.0).abs() < 1e-6);
        // A frame too wide for it: nothing.
        app.select(vec![r]);
        app.sc_cnc.door.frame = 200.0;
        assert!(app.cnc_door().is_none());
    }

    #[test]
    fn centre_lines_of_a_bar_and_a_ring_in_one_step() {
        let mut app = App::headless();
        let bar = rect(&mut app, 0.0, 0.0, 100.0, 10.0);
        use tracedraw_core::geometry::Shape as _;
        let mut ring = Rect::new(0.0, 50.0, 40.0, 90.0).to_path(0.01);
        ring.extend(Rect::new(10.0, 60.0, 30.0, 80.0).to_path(0.01));
        let r = app
            .new_shape(ShapeKind::Path {
                path: ring,
                closed: true,
            })
            .expect("ring");
        app.select(vec![bar, r]);
        assert_eq!(app.cnc_center_lines(), 2);
        assert_eq!(app.engine.undo_label(), Some("Centre Line"));
        let g = app.selection[0];
        let s = app.doc().shape_on_page(g).expect("group");
        let ShapeKind::Group { children } = &s.kind else {
            panic!("not a group");
        };
        assert!(matches!(
            children[0].kind,
            ShapeKind::Path { closed: false, .. }
        ));
        assert!(matches!(
            children[1].kind,
            ShapeKind::Path { closed: true, .. }
        ));
        let b = children[0].page_path().bounding_box();
        assert!((b.center().y - 5.0).abs() < 0.3 && (b.width() - 90.0).abs() < 1.5);
        // Nothing closed: nothing added.
        let mut line = BezPath::new();
        line.move_to((0.0, 150.0));
        line.line_to((10.0, 150.0));
        let l = app
            .new_shape(ShapeKind::Path {
                path: line,
                closed: false,
            })
            .expect("line");
        app.select(vec![l]);
        assert_eq!(app.cnc_center_lines(), 0);
    }
}
