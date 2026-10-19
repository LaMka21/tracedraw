//! Sign & Cut, Sign Making: settings and operations (LED modules, neon
//! tubes, sign boxes, letter returns, acrylic moulds, smart divide and
//! carriers). The geometry is in `tracedraw_signcut::sign`; the tool
//! windows are `ui/signcut_win_sign.rs`. Letter returns, acrylic moulds
//! and sign boxes are worked out first into a [`Plan`] (shown in the
//! window's preview, nothing in the drawing changes) and then committed
//! in one undo step. See docs/behavior/sign-and-cut-sign.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use egui::Ui;
use tracedraw_core::geometry::{Affine, BezPath, Point, Rect, Shape as _};
use tracedraw_core::{
    Color, Command, Fill, LayerId, LineCap, LineJoin, Shape, ShapeId, ShapeKind, Stroke, TextSpan,
};
use tracedraw_signcut::sign::carrier::{self, CarrierSpec};
use tracedraw_signcut::sign::led::{self, LedReport, LedSpec, Module, Placement};
use tracedraw_signcut::sign::neon::{self, ContourSpec, NeonKind, NeonSpec};
use tracedraw_signcut::sign::returns::{self, MouldSpec, ReturnMethod, ReturnSpec, Strip};
use tracedraw_signcut::sign::sign_box::{self, BoxSpec};
use tracedraw_signcut::sign::tiles::{self, TileGrid, TileSpec};

/// The tools of sign making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Led,
    Neon,
    SignBox,
    Returns,
    Mould,
    Tiles,
    Carrier,
}

impl Tool {
    pub const ALL: [Tool; 7] = [
        Tool::Led,
        Tool::Neon,
        Tool::SignBox,
        Tool::Returns,
        Tool::Mould,
        Tool::Tiles,
        Tool::Carrier,
    ];

    fn menu_key(self) -> &'static str {
        match self {
            Tool::Led => "menu.tools.signcut_sign_led",
            Tool::Neon => "menu.tools.signcut_sign_neon",
            Tool::SignBox => "menu.tools.signcut_sign_box",
            Tool::Returns => "menu.tools.signcut_sign_returns",
            Tool::Mould => "menu.tools.signcut_sign_mould",
            Tool::Tiles => "menu.tools.signcut_sign_tiles",
            Tool::Carrier => "menu.tools.signcut_sign_carrier",
        }
    }
}

/// Where the tile grid starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TileOrigin {
    /// The bottom-left corner of the selection.
    #[default]
    Selection,
    /// The page's origin (its bottom-left corner).
    Page,
}

/// How Smart Divide sizes its tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TileMethod {
    /// Tiles of a given size (the material's), from a grid origin.
    #[default]
    Size,
    /// A number of columns and rows that just covers the objects.
    Table,
}

/// What Neon Tubes made or measured last.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NeonReport {
    pub tubes: usize,
    pub length: f64,
    pub supports: usize,
}

/// The standard tube diameters offered (mm).
pub const NEON_DIAMETERS: [f64; 9] = [6.0, 8.0, 10.0, 12.0, 14.0, 15.0, 16.0, 18.0, 20.0];

/// LED Modules: the catalogue and the layout.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LedSettings {
    pub catalogue: Vec<Module>,
    /// The catalogue entry in use.
    pub module: usize,
    pub spec: LedSpec,
}

impl Default for LedSettings {
    fn default() -> Self {
        LedSettings {
            catalogue: led::catalogue(),
            module: 0,
            spec: LedSpec::default(),
        }
    }
}

/// Neon Tubes.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NeonSettings {
    pub kind: NeonKind,
    pub spec: NeonSpec,
    pub contour: ContourSpec,
    /// Draw tubes as their outline (both sides) instead of a thick line.
    pub outline: bool,
    /// The diameter is typed in rather than picked from the standard ones.
    pub custom_diameter: bool,
}

/// Sign Box.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BoxSettings {
    pub spec: BoxSpec,
    /// The face is the selection's box when something is selected.
    pub use_selection: bool,
    /// The face otherwise (mm), centred on the page.
    pub width: f64,
    pub height: f64,
    /// Write the size on the face.
    pub legend: bool,
    pub prefix: String,
}

impl Default for BoxSettings {
    fn default() -> Self {
        BoxSettings {
            spec: BoxSpec::default(),
            use_selection: true,
            width: 1000.0,
            height: 500.0,
            legend: false,
            prefix: String::new(),
        }
    }
}

/// Letter Returns.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ReturnsSettings {
    pub method: ReturnMethod,
    pub spec: ReturnSpec,
    /// Length of the corner marks (Marked method).
    pub mark_length: f64,
    /// Also make each letter's face, inset by the material thickness.
    pub make_face: bool,
    /// Space between the strips and under the selection (mm).
    pub strip_gap: f64,
    pub labels: bool,
}

impl Default for ReturnsSettings {
    fn default() -> Self {
        ReturnsSettings {
            method: ReturnMethod::Notched,
            spec: ReturnSpec::default(),
            mark_length: 5.0,
            make_face: false,
            strip_gap: 10.0,
            labels: true,
        }
    }
}

/// Acrylic Mould.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MouldSettings {
    pub spec: MouldSpec,
    pub strip_gap: f64,
    pub labels: bool,
    /// Density of the casting resin (g/ml).
    pub density: f64,
}

impl Default for MouldSettings {
    fn default() -> Self {
        MouldSettings {
            spec: MouldSpec::default(),
            strip_gap: 10.0,
            labels: true,
            density: 1.18,
        }
    }
}

/// Smart Divide.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TileSettings {
    pub method: TileMethod,
    pub spec: TileSpec,
    pub cols: usize,
    pub rows: usize,
    pub origin: TileOrigin,
    pub labels: bool,
    pub frames: bool,
}

impl Default for TileSettings {
    fn default() -> Self {
        TileSettings {
            method: TileMethod::Size,
            spec: TileSpec::default(),
            cols: 2,
            rows: 1,
            origin: TileOrigin::Selection,
            labels: true,
            frames: false,
        }
    }
}

/// The sign making tools' settings, last results and window states.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// The tool the menu or toolbar last asked for.
    pub open: Option<Tool>,
    pub led: LedSettings,
    pub led_report: Option<LedReport>,
    pub neon: NeonSettings,
    pub neon_report: Option<NeonReport>,
    pub sign_box: BoxSettings,
    pub returns: ReturnsSettings,
    pub mould: MouldSettings,
    pub tiles: TileSettings,
    /// Columns, rows and pieces of the last division.
    pub tiles_report: Option<(usize, usize, usize)>,
    pub carrier: CarrierSpec,
    /// Pieces of the last carrier.
    pub carrier_report: Option<usize>,
    /// The windows' states.
    pub win: crate::ui::signcut_win_sign::State,
}

/// Colours of the made objects.
pub const NEON_COLOR: [u8; 3] = [255, 40, 150];
pub const CUT_COLOR: [u8; 3] = [230, 0, 0];
pub const FOLD_COLOR: [u8; 3] = [0, 90, 230];
pub const STRIP_COLOR: [u8; 3] = [0, 120, 200];
pub const CARRIER_COLOR: [u8; 3] = [0, 150, 80];
pub const TILE_FRAME_COLOR: [u8; 3] = [140, 140, 140];

fn rgb(c: [u8; 3]) -> Color {
    Color::rgb8(c[0], c[1], c[2])
}

fn hairline(c: [u8; 3]) -> Option<Stroke> {
    Some(Stroke::hairline(rgb(c)))
}

/// A dashed line for folds: 3 mm on, 2 mm off.
fn dashed(c: [u8; 3]) -> Option<Stroke> {
    Some(Stroke {
        dash: vec![15.0, 10.0],
        ..Stroke::new(rgb(c), 0.2)
    })
}

/// All subpaths of the path are closed.
fn all_closed(p: &BezPath) -> bool {
    use tracedraw_core::geometry::PathEl;
    let moves = p
        .elements()
        .iter()
        .filter(|e| matches!(e, PathEl::MoveTo(_)))
        .count();
    let closes = p
        .elements()
        .iter()
        .filter(|e| matches!(e, PathEl::ClosePath))
        .count();
    moves > 0 && moves == closes
}

/// Points per millimetre of text size.
const PT_PER_MM: f64 = 72.0 / 25.4;

/// What a part of a plan is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A cut outline (box pattern, mould base, letter face).
    Cut,
    /// A dashed fold line.
    Fold,
    /// A return or wall strip.
    Strip,
    /// A corner mark on a strip.
    Mark,
    /// A hole.
    Hole,
}

impl Role {
    fn stroke(self) -> Option<Stroke> {
        match self {
            Role::Cut | Role::Hole => hairline(CUT_COLOR),
            Role::Fold => dashed(FOLD_COLOR),
            Role::Strip | Role::Mark => hairline(STRIP_COLOR),
        }
    }
}

/// The mould's material figures.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MouldFigures {
    /// Outline length of the letters (mm).
    pub perimeter: f64,
    /// Area of the letters (mm2).
    pub area: f64,
    /// Resin to fill them to the fill depth (ml).
    pub volume: f64,
    /// Its mass (g).
    pub mass: f64,
}

/// A worked-out result of Letter Returns, Acrylic Mould or Sign Box:
/// page-space paths with their roles and text labels, made for the
/// drawing as it was at `revision` with the objects `ids`.
#[derive(Debug, Clone)]
pub struct Plan {
    pub revision: u64,
    pub ids: Vec<ShapeId>,
    pub items: Vec<(BezPath, Role)>,
    /// Text, baseline start and height (mm).
    pub labels: Vec<(String, Point, f64)>,
    /// Strips made and their total length (a box: 1 and 0).
    pub count: usize,
    pub length: f64,
    /// The pattern's size (a box).
    pub size: Option<(f64, f64)>,
    pub mould: Option<MouldFigures>,
}

impl Plan {
    fn new(app: &App, ids: Vec<ShapeId>) -> Self {
        Plan {
            revision: app.engine.revision(),
            ids,
            items: Vec::new(),
            labels: Vec::new(),
            count: 0,
            length: 0.0,
            size: None,
            mould: None,
        }
    }

    /// The plan is for the drawing and selection as they are now.
    pub fn is_current(&self, app: &App) -> bool {
        self.revision == app.engine.revision() && self.ids == app.sign_targets()
    }
}

/// The number of distinct outlines of each object among the strips.
fn outlines_per_object(strips: &[Strip]) -> std::collections::HashMap<usize, usize> {
    let mut seen: std::collections::HashMap<usize, std::collections::BTreeSet<usize>> =
        Default::default();
    for s in strips {
        seen.entry(s.object).or_default().insert(s.contour);
    }
    seen.into_iter().map(|(k, v)| (k, v.len())).collect()
}

impl App {
    /// Open the window of a sign making tool.
    pub fn open_sign_tool(&mut self, tool: Tool) {
        self.sc_sign.open = Some(tool);
        self.open_signcut(crate::signcut::Section::Sign);
    }

    /// The selected objects the tools work on, in stacking order, locked
    /// ones left out.
    pub fn sign_targets(&self) -> Vec<ShapeId> {
        if self.selection.is_empty() {
            return Vec::new();
        }
        let doc = self.doc();
        self.signcut_targets()
            .into_iter()
            .filter(|id| doc.find_shape(*id).is_some_and(|s| !s.locked))
            .collect()
    }

    /// Each target's outlines in page space.
    pub fn sign_outlines(&self, ids: &[ShapeId]) -> Vec<Vec<BezPath>> {
        let doc = self.doc();
        ids.iter()
            .map(|id| {
                doc.shape_on_page(*id)
                    .map(|s| tracedraw_signcut::outlines(&s))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// The layer new objects go to: the first target's (when it can take
    /// them), else the active layer.
    fn sign_layer(&self, ids: &[ShapeId]) -> Option<LayerId> {
        let doc = self.doc();
        ids.first()
            .and_then(|id| doc.top_level_of(*id))
            .and_then(|top| doc.locate(top).ok())
            .map(|(l, _)| l)
            .filter(|l| doc.layer(*l).is_ok_and(|l| !l.locked))
            .or_else(|| self.active_layer())
    }

    /// The page-space bounds of the outlines.
    pub fn sign_bounds(outlines: &[Vec<BezPath>]) -> Option<Rect> {
        outlines
            .iter()
            .flatten()
            .map(|p| p.bounding_box())
            .filter(|b| {
                b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()
            })
            .reduce(|a, b| a.union(b))
    }

    fn sign_path(&mut self, path: BezPath, fill: Fill, stroke: Option<Stroke>) -> Shape {
        let closed = all_closed(&path);
        let mut s = Shape::new(self.engine.new_shape_id(), ShapeKind::Path { path, closed });
        s.fill = fill;
        s.stroke = stroke;
        s
    }

    /// A small number or name as text, its baseline starting at `at`,
    /// `size` millimetres high.
    fn sign_label(&mut self, text: String, at: Point, size: f64) -> Shape {
        let mut s = Shape::new(
            self.engine.new_shape_id(),
            ShapeKind::Text {
                spans: vec![TextSpan::new(
                    text,
                    self.text_font.clone(),
                    (size * PT_PER_MM).clamp(2.0, 400.0),
                )],
                origin: at,
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        s.fill = Fill::Solid(Color::BLACK);
        s.stroke = None;
        s
    }

    fn sign_group(&mut self, name: &str, children: Vec<Shape>) -> Shape {
        let mut g = Shape::new(self.engine.new_shape_id(), ShapeKind::Group { children });
        g.name = Some(tr(name));
        g
    }

    /// Add `shapes` to `layer` in one undo step labelled `label`, and
    /// return their ids.
    fn sign_add(
        &mut self,
        label: &'static str,
        layer: LayerId,
        shapes: Vec<Shape>,
    ) -> Vec<ShapeId> {
        let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        let cmds: Vec<Command> = shapes
            .into_iter()
            .map(|shape| Command::AddShape { layer, shape })
            .collect();
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return Vec::new();
        }
        ids
    }

    fn sign_need_selection(&mut self) -> Option<(Vec<ShapeId>, LayerId)> {
        let ids = self.sign_targets();
        if ids.is_empty() {
            self.status = tr("signcut.sign_select");
            return None;
        }
        let Some(layer) = self.sign_layer(&ids) else {
            self.status = tr("signcut.sign_no_layer");
            return None;
        };
        Some((ids, layer))
    }

    /// Put a plan into the drawing as one group in one undo step. Returns
    /// the group's id.
    pub fn sign_commit(
        &mut self,
        plan: &Plan,
        label: &'static str,
        group_name: &str,
    ) -> Option<ShapeId> {
        let Some(layer) = self.sign_layer(&plan.ids) else {
            self.status = tr("signcut.sign_no_layer");
            return None;
        };
        let mut children = Vec::new();
        for (path, role) in &plan.items {
            if path.elements().is_empty() {
                continue;
            }
            let s = self.sign_path(path.clone(), Fill::None, role.stroke());
            children.push(s);
        }
        for (text, at, size) in &plan.labels {
            let t = self.sign_label(text.clone(), *at, *size);
            children.push(t);
        }
        if children.is_empty() {
            return None;
        }
        let group = self.sign_group(group_name, children);
        self.sign_add(label, layer, vec![group]).first().copied()
    }

    /// The module in use.
    pub fn sign_module(&self) -> Option<&Module> {
        self.sc_sign.led.catalogue.get(self.sc_sign.led.module)
    }

    /// Where LED modules go in the selection with the current settings
    /// (nothing changes). `None` without a selection or a module.
    pub fn sign_led_placements(&self) -> Option<(Vec<Placement>, Module)> {
        let ids = self.sign_targets();
        if ids.is_empty() {
            return None;
        }
        let module = self.sign_module()?.clone();
        let spec = self.sc_sign.led.spec;
        let mut placements = Vec::new();
        for paths in self.sign_outlines(&ids) {
            placements.extend(led::layout(&paths, &module, &spec));
        }
        if spec.fix_overlaps && ids.len() > 1 {
            let (fw, fh) = module.footprint(spec.rotate);
            let gap = if spec.overlap_gap.is_finite() {
                spec.overlap_gap.max(0.0)
            } else {
                0.0
            };
            placements = led::remove_overlaps(placements, fw, fh, gap, module.round);
        }
        placements.truncate(led::MAX_MODULES);
        Some((placements, module))
    }

    /// Lay LED modules out in the selected shapes as one group. Returns
    /// how many modules were placed.
    pub fn sign_led_layout(&mut self) -> usize {
        let Some((_, layer)) = self.sign_need_selection() else {
            return 0;
        };
        let Some((placements, module)) = self.sign_led_placements() else {
            self.status = tr("signcut.sign_no_module");
            return 0;
        };
        let spec = self.sc_sign.led.spec;
        let report = led::report(&module, placements.len());
        self.sc_sign.led_report = Some(report);
        if placements.is_empty() {
            self.status = tr("signcut.sign_led_none");
            return 0;
        }
        let fill = Fill::Solid(rgb(module.color));
        let edge = [
            module.color[0] / 2,
            module.color[1] / 2,
            module.color[2] / 2,
        ];
        let children: Vec<Shape> = placements
            .iter()
            .map(|p| {
                let path = led::module_path(&module, spec.rotate, *p);
                self.sign_path(path, fill.clone(), hairline(edge))
            })
            .collect();
        let group = self.sign_group("signcut.sign_led_group", children);
        self.sign_add("LED Modules", layer, vec![group]);
        self.status = led_summary(&report);
        placements.len()
    }

    /// The LED groups of the current page (groups named as LED Modules
    /// names them).
    fn sign_led_groups(&self) -> Vec<ShapeId> {
        let name = tr("signcut.sign_led_group");
        self.page_layers()
            .iter()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| l.shapes.iter())
            .filter(|s| {
                !s.locked
                    && s.name.as_deref() == Some(name.as_str())
                    && matches!(s.kind, ShapeKind::Group { .. })
            })
            .map(|s| s.id)
            .collect()
    }

    /// Select every LED group of the page. Returns how many.
    pub fn sign_led_select_all(&mut self) -> usize {
        let ids = self.sign_led_groups();
        let n = ids.len();
        self.select(ids);
        self.status = trf("signcut.wsign_led_found", &[("n", &n.to_string())]);
        n
    }

    /// Remove the modules of the selected LED groups that come closer than
    /// `gap` to a module kept before them (their boxes compared), in one
    /// undo step. Returns how many were removed.
    pub fn sign_led_fix_overlaps(&mut self, gap: f64) -> usize {
        let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
        let groups = self.sign_led_groups();
        let doc = self.doc();
        let mut cmds = Vec::new();
        let mut removed = 0;
        for id in self
            .sign_targets()
            .into_iter()
            .filter(|id| groups.contains(id))
        {
            let Some(ShapeKind::Group { children }) = doc.find_shape(id).map(|s| s.kind.clone())
            else {
                continue;
            };
            let before = children.len();
            let mut kept: Vec<(Shape, Rect)> = Vec::new();
            for c in children {
                let b = c.page_path().bounding_box();
                let grown = b.inflate(gap / 2.0, gap / 2.0);
                let clash = kept.iter().any(|(_, k)| {
                    let o = k.inflate(gap / 2.0, gap / 2.0).intersect(grown);
                    o.width() > 1e-9 && o.height() > 1e-9
                });
                if clash {
                    removed += 1;
                } else {
                    kept.push((c, b));
                }
            }
            if kept.len() < before {
                cmds.push(Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::Group {
                        children: kept.into_iter().map(|(s, _)| s).collect(),
                    },
                });
            }
        }
        if cmds.is_empty() {
            self.status = tr("signcut.wsign_led_no_overlaps");
            return 0;
        }
        if let Err(e) = self.engine.run_batch("Fix LED Overlaps", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.status = trf("signcut.wsign_led_fixed", &[("n", &removed.to_string())]);
        removed
    }

    /// The tubes' centre lines of the selection: its curves, or the
    /// contours inside its closed shapes.
    fn sign_neon_lines(&self, ids: &[ShapeId]) -> Vec<BezPath> {
        let n = &self.sc_sign.neon;
        let outlines = self.sign_outlines(ids);
        match n.kind {
            NeonKind::CentreLine => outlines.into_iter().flatten().collect(),
            NeonKind::Contour => outlines
                .iter()
                .flat_map(|paths| neon::contour_lines(paths, n.spec.diameter, &n.contour))
                .collect(),
        }
    }

    /// Neon tubes along the selected curves, as one group. Returns the
    /// number of tubes.
    pub fn sign_neon(&mut self) -> usize {
        self.sign_neon_parts(true, true)
    }

    /// Neon tubes (`tubes`) and their support holes (`holes`) for the
    /// selection, as one group in one undo step. Returns the number of
    /// tubes worked out.
    pub fn sign_neon_parts(&mut self, tubes: bool, holes: bool) -> usize {
        let Some((ids, layer)) = self.sign_need_selection() else {
            return 0;
        };
        let spec = self.sc_sign.neon.spec;
        let paths = self.sign_neon_lines(&ids);
        let result = neon::neon(&paths, &spec);
        let report = NeonReport {
            tubes: result.tubes.len(),
            length: result.total_length(),
            supports: result.supports.len(),
        };
        self.sc_sign.neon_report = Some(report);
        if result.tubes.is_empty() {
            self.status = tr("signcut.sign_neon_none");
            return 0;
        }
        let d = if spec.diameter.is_finite() {
            spec.diameter.abs().max(0.1)
        } else {
            12.0
        };
        let mut children = Vec::new();
        if tubes {
            for tube in &result.tubes {
                let shape = if self.sc_sign.neon.outline {
                    let outline = neon::tube_outline(tube, d);
                    self.sign_path(outline, Fill::None, hairline(NEON_COLOR))
                } else {
                    self.sign_path(tube.path.clone(), Fill::None, Some(tube_stroke(d)))
                };
                children.push(shape);
            }
        }
        let hole = spec.hole_diameter;
        if holes && hole.is_finite() && hole > 0.0 {
            for p in &result.supports {
                let c = tracedraw_signcut::sign::circle(*p, hole);
                let s = self.sign_path(c, Fill::None, hairline([0, 0, 0]));
                children.push(s);
            }
        }
        if children.is_empty() {
            self.status = tr("signcut.sign_neon_none");
            return 0;
        }
        let group = self.sign_group("signcut.sign_neon_group", children);
        let label = match (tubes, holes) {
            (true, true) => "Neon Tubes",
            (true, false) => "Divide Tubes",
            _ => "Support Holes",
        };
        self.sign_add(label, layer, vec![group]);
        let u = self.units;
        self.status = trf(
            "signcut.sign_neon_done",
            &[
                ("n", &report.tubes.to_string()),
                ("len", &crate::ui::signcut_docker::fmt_len(u, report.length)),
                ("holes", &report.supports.to_string()),
            ],
        );
        report.tubes
    }

    /// The selected curves (and the curves in selected groups) drawn as
    /// thick lines become their tube outlines, in one undo step. Returns
    /// how many were turned.
    pub fn sign_neon_to_outlines(&mut self) -> usize {
        let doc = self.doc();
        let to_outline = |s: &Shape| -> Option<Shape> {
            let ShapeKind::Path { path, closed } = &s.kind else {
                return None;
            };
            let w = s.stroke.as_ref().map(|k| k.width)?;
            if !(w.is_finite() && w > Stroke::HAIRLINE + 1e-9) {
                return None;
            }
            let tube = neon::Tube {
                path: path.clone(),
                length: 0.0,
                closed: *closed,
            };
            let outline = neon::tube_outline(&tube, w);
            if outline.elements().is_empty() {
                return None;
            }
            let mut out = s.clone();
            out.kind = ShapeKind::Path {
                path: outline,
                closed: true,
            };
            out.fill = Fill::None;
            out.stroke = s.stroke.as_ref().map(|k| Stroke::hairline(k.color));
            Some(out)
        };
        let mut cmds = Vec::new();
        let mut turned = 0;
        for id in self.sign_targets() {
            let Some(s) = doc.find_shape(id) else {
                continue;
            };
            match &s.kind {
                ShapeKind::Group { children } => {
                    let mut changed = false;
                    let kids: Vec<Shape> = children
                        .iter()
                        .map(|c| match to_outline(c) {
                            Some(o) => {
                                changed = true;
                                turned += 1;
                                o
                            }
                            None => c.clone(),
                        })
                        .collect();
                    if changed {
                        cmds.push(Command::SetShapeKind {
                            shape: id,
                            kind: ShapeKind::Group { children: kids },
                        });
                    }
                }
                _ => {
                    if let Some(o) = to_outline(s) {
                        turned += 1;
                        cmds.push(Command::SetShapeKind {
                            shape: id,
                            kind: o.kind,
                        });
                        cmds.push(Command::SetFill {
                            shapes: vec![id],
                            fill: Fill::None,
                        });
                        cmds.push(Command::SetStroke {
                            shapes: vec![id],
                            stroke: o.stroke,
                        });
                    }
                }
            }
        }
        if cmds.is_empty() {
            self.status = tr("signcut.wsign_neon_no_tubes");
            return 0;
        }
        if let Err(e) = self.engine.run_batch("Tubes to Outlines", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.status = trf(
            "signcut.wsign_neon_converted",
            &[("n", &turned.to_string())],
        );
        turned
    }

    /// How many tubes the selection would make and their length, without
    /// changing anything.
    pub fn sign_neon_measure(&mut self) -> Option<NeonReport> {
        let ids = self.sign_targets();
        if ids.is_empty() {
            self.status = tr("signcut.sign_select");
            return None;
        }
        let result = neon::neon(&self.sign_neon_lines(&ids), &self.sc_sign.neon.spec);
        let report = NeonReport {
            tubes: result.tubes.len(),
            length: result.total_length(),
            supports: result.supports.len(),
        };
        self.sc_sign.neon_report = Some(report);
        let u = self.units;
        self.status = trf(
            "signcut.wsign_neon_measured",
            &[
                ("n", &report.tubes.to_string()),
                ("len", &crate::ui::signcut_docker::fmt_len(u, report.length)),
                ("holes", &report.supports.to_string()),
            ],
        );
        Some(report)
    }

    /// The face of the sign box: the selection's bounds (when the window
    /// says so and something is selected), or the window's width and
    /// height centred on the page.
    pub fn sign_box_face(&self) -> Rect {
        let b = &self.sc_sign.sign_box;
        if b.use_selection {
            let ids = self.sign_targets();
            if let Some(bounds) = Self::sign_bounds(&self.sign_outlines(&ids)) {
                return bounds;
            }
        }
        let c = self.page_rect().center();
        let (w, h) = (b.width, b.height);
        Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0)
    }

    /// Work the sign box pattern out (nothing changes).
    pub fn sign_box_plan(&self) -> Result<Plan, String> {
        let face = self.sign_box_face();
        let b = &self.sc_sign.sign_box;
        let pattern =
            sign_box::sign_box(face, &b.spec).ok_or_else(|| tr("signcut.sign_box_bad"))?;
        let mut plan = Plan::new(self, self.sign_targets());
        plan.items.push((pattern.outline.clone(), Role::Cut));
        for (a, c) in &pattern.folds {
            plan.items.push((
                tracedraw_core::geometry::polyline_path(&[*a, *c], false),
                Role::Fold,
            ));
        }
        if pattern.hole_diameter > 0.0 {
            for p in &pattern.holes {
                plan.items.push((
                    tracedraw_signcut::sign::circle(*p, pattern.hole_diameter),
                    Role::Hole,
                ));
            }
        }
        let bb = pattern.outline.bounding_box();
        plan.size = Some((bb.width(), bb.height()));
        plan.count = 1;
        if b.legend {
            let u = self.units;
            let d = b.spec.depth.clean();
            let deep = d.left.max(d.right).max(d.bottom).max(d.top);
            let text = format!(
                "{}{} x {} x {}",
                b.prefix,
                crate::ui::signcut_docker::fmt_len(u, face.width()),
                crate::ui::signcut_docker::fmt_len(u, face.height()),
                crate::ui::signcut_docker::fmt_len(u, deep)
            );
            let size = (face.width().min(face.height()) * 0.06).clamp(3.0, 100.0);
            let at = Point::new(face.x0 + size, face.center().y - size / 2.0);
            plan.labels.push((text, at, size));
        }
        Ok(plan)
    }

    /// The flat pattern of a sign box as one group.
    #[cfg(test)]
    pub fn sign_make_box(&mut self) -> bool {
        let plan = match self.sign_box_plan() {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return false;
            }
        };
        self.sign_box_commit(&plan)
    }

    /// Put a worked-out sign box into the drawing.
    pub fn sign_box_commit(&mut self, plan: &Plan) -> bool {
        if self
            .sign_commit(plan, "Sign Box", "signcut.sign_box_group")
            .is_none()
        {
            return false;
        }
        let u = self.units;
        let (w, h) = plan.size.unwrap_or_default();
        self.status = trf(
            "signcut.sign_box_done",
            &[
                ("w", &crate::ui::signcut_docker::fmt_len(u, w)),
                ("h", &crate::ui::signcut_docker::fmt_len(u, h)),
            ],
        );
        true
    }

    /// Strips laid out under `below`, with their marks and numbers, into
    /// the plan.
    fn sign_plan_strips(
        plan: &mut Plan,
        strips: &[Strip],
        below: Rect,
        (gap, labels): (f64, bool),
        (notch, marks): (f64, f64),
    ) {
        let at = returns::stack(strips, below.x0, below.y0, gap);
        let per_object = outlines_per_object(strips);
        for (s, p) in strips.iter().zip(at) {
            plan.items
                .push((returns::strip_path(s, notch, p), Role::Strip));
            for (a, b) in returns::strip_marks(s, marks, p) {
                plan.items.push((
                    tracedraw_core::geometry::polyline_path(&[a, b], false),
                    Role::Mark,
                ));
            }
            if labels {
                let size = (s.height * 0.4).clamp(1.0, 20.0);
                let n = per_object.get(&s.object).copied().unwrap_or(1);
                let label = returns::strip_label(s, n);
                let pos = Point::new(p.x + size * 0.5, p.y + (s.height - size) / 2.0);
                plan.labels.push((label, pos, size));
            }
        }
        plan.count = strips.len();
        plan.length = strips.iter().map(|s| s.length).sum();
    }

    /// Work the letter returns of the selection out (nothing changes).
    pub fn sign_returns_plan(&self) -> Result<Plan, String> {
        let ids = self.sign_targets();
        if ids.is_empty() {
            return Err(tr("signcut.sign_select"));
        }
        let outlines = self.sign_outlines(&ids);
        let bounds = Self::sign_bounds(&outlines).ok_or_else(|| tr("signcut.sign_returns_none"))?;
        let r = self.sc_sign.returns;
        let mut spec = r.spec;
        let (notch, marks) = match r.method {
            ReturnMethod::Notched => (spec.notch_size, 0.0),
            ReturnMethod::Marked => (0.0, r.mark_length),
            ReturnMethod::Plain => {
                spec.notch_angle = 0.0;
                (0.0, 0.0)
            }
        };
        let strips = returns::returns(&outlines, &spec);
        if strips.is_empty() {
            return Err(tr("signcut.sign_returns_none"));
        }
        let mut plan = Plan::new(self, ids);
        if r.make_face {
            for paths in &outlines {
                let f = returns::letter_face(paths, spec.thickness);
                if !f.elements().is_empty() {
                    plan.items.push((f, Role::Cut));
                }
            }
        }
        Self::sign_plan_strips(
            &mut plan,
            &strips,
            bounds,
            (r.strip_gap, r.labels),
            (notch, marks),
        );
        Ok(plan)
    }

    /// Letter return strips of the selected letters, under them, as one
    /// group. Returns the number of strips.
    #[cfg(test)]
    pub fn sign_returns(&mut self) -> usize {
        match self.sign_returns_plan() {
            Ok(plan) => self.sign_strips_commit(&plan, false),
            Err(e) => {
                self.sc_sign.win.returns.report = None;
                self.status = e;
                0
            }
        }
    }

    /// Put worked-out strips (returns, or a mould with `mould`) into the
    /// drawing. Returns the number of strips.
    pub fn sign_strips_commit(&mut self, plan: &Plan, mould: bool) -> usize {
        let (label, name) = if mould {
            ("Acrylic Mould", "signcut.sign_mould_group")
        } else {
            ("Letter Returns", "signcut.sign_returns_group")
        };
        if self.sign_commit(plan, label, name).is_none() {
            return 0;
        }
        let state = if mould {
            &mut self.sc_sign.win.mould
        } else {
            &mut self.sc_sign.win.returns
        };
        state.report = Some((plan.count, plan.length));
        let u = self.units;
        self.status = trf(
            "signcut.sign_strips_done",
            &[
                ("n", &plan.count.to_string()),
                ("len", &crate::ui::signcut_docker::fmt_len(u, plan.length)),
            ],
        );
        plan.count
    }

    /// Work the liquid acrylic mould of the selection out (nothing
    /// changes).
    pub fn sign_mould_plan(&self) -> Result<Plan, String> {
        let ids = self.sign_targets();
        if ids.is_empty() {
            return Err(tr("signcut.sign_select"));
        }
        let outlines = self.sign_outlines(&ids);
        let bounds = Self::sign_bounds(&outlines).ok_or_else(|| tr("signcut.sign_returns_none"))?;
        let m = self.sc_sign.mould;
        let (bases, strips) = returns::mould(&outlines, &m.spec);
        if strips.is_empty() {
            return Err(tr("signcut.sign_returns_none"));
        }
        let mut plan = Plan::new(self, ids);
        for b in bases.into_iter().filter(|b| !b.elements().is_empty()) {
            plan.items.push((b, Role::Cut));
        }
        for i in returns::mould_inside(&outlines, &m.spec) {
            if !i.elements().is_empty() {
                plan.items.push((i, Role::Cut));
            }
        }
        // The strips go under the bases, which reach out by the wall.
        let wall = m.spec.wall;
        let wall = if wall.is_finite() { wall.max(0.0) } else { 0.0 };
        let below = bounds.inflate(wall, wall);
        Self::sign_plan_strips(
            &mut plan,
            &strips,
            below,
            (m.strip_gap, m.labels),
            (0.0, 0.0),
        );
        let fig = tracedraw_signcut::measure::measure(&outlines);
        let depth = if m.spec.depth.is_finite() {
            m.spec.depth.max(0.0)
        } else {
            0.0
        };
        let density = if m.density.is_finite() {
            m.density.max(0.0)
        } else {
            0.0
        };
        let volume = fig.area.max(0.0) * depth / 1000.0;
        plan.mould = Some(MouldFigures {
            perimeter: fig.length,
            area: fig.area,
            volume,
            mass: volume * density,
        });
        Ok(plan)
    }

    /// The liquid acrylic mould of the selected letters: the bases over
    /// the letters and the wall strips under them, as one group. Returns
    /// the number of strips.
    #[cfg(test)]
    pub fn sign_mould(&mut self) -> usize {
        match self.sign_mould_plan() {
            Ok(plan) => self.sign_strips_commit(&plan, true),
            Err(e) => {
                self.status = e;
                0
            }
        }
    }

    /// The tile grid over `bounds` with the window's settings.
    fn sign_grid_over(&self, bounds: Rect) -> Option<TileGrid> {
        let t = &self.sc_sign.tiles;
        match t.method {
            TileMethod::Size => {
                let origin = match t.origin {
                    TileOrigin::Selection => Point::new(bounds.x0, bounds.y0),
                    TileOrigin::Page => Point::ZERO,
                };
                tiles::grid(bounds, origin, &t.spec)
            }
            TileMethod::Table => tiles::grid_by_count(bounds, t.cols, t.rows, t.spec.overlap),
        }
    }

    /// The tile grid over the selection, with the window's settings.
    pub fn sign_tile_grid(&self) -> Option<TileGrid> {
        let ids = self.sign_targets();
        let bounds = Self::sign_bounds(&self.sign_outlines(&ids))?;
        self.sign_grid_over(bounds)
    }

    /// Divide the selected objects into tiles: each tile a group of the
    /// objects' parts inside it (their fill and outline kept), the
    /// originals replaced, in one undo step. Returns the number of tiles
    /// made.
    pub fn sign_divide(&mut self) -> usize {
        let Some((ids, layer)) = self.sign_need_selection() else {
            return 0;
        };
        // Objects with bitmaps stay whole and are left out.
        let doc = self.doc();
        let ids: Vec<ShapeId> = ids
            .into_iter()
            .filter(|id| doc.find_shape(*id).is_some_and(|s| !has_bitmap(s)))
            .collect();
        let pieces: Vec<(Shape, Vec<BezPath>)> = ids
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .flat_map(|s| {
                let mut out = Vec::new();
                leaves(&s, Affine::IDENTITY, &mut out);
                out
            })
            .collect();
        if pieces.is_empty() {
            self.status = tr("signcut.sign_select");
            return 0;
        }
        let all: Vec<Vec<BezPath>> = pieces.iter().map(|(_, p)| p.clone()).collect();
        let Some(bounds) = Self::sign_bounds(&all) else {
            return 0;
        };
        let t = self.sc_sign.tiles;
        let Some(grid) = self.sign_grid_over(bounds) else {
            self.status = tr("signcut.sign_tiles_bad");
            return 0;
        };
        let mut groups = Vec::new();
        let mut parts = 0usize;
        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let rect = grid.rect(col, row);
                let shift = Affine::translate(grid.spread(col, row, t.spec.gap));
                let mut children = Vec::new();
                for (leaf, paths) in &pieces {
                    let part = tiles::clip(paths, rect);
                    if part.elements().is_empty() {
                        continue;
                    }
                    let mut s =
                        self.sign_path(shift * part, leaf.fill.clone(), leaf.stroke.clone());
                    s.opacity = leaf.opacity;
                    children.push(s);
                }
                if children.is_empty() {
                    continue;
                }
                parts += children.len();
                if t.frames {
                    let frame = shift * rect.to_path(0.01);
                    let mut f = self.sign_path(frame, Fill::None, hairline(TILE_FRAME_COLOR));
                    f.stroke = f.stroke.map(|s| Stroke {
                        dash: vec![30.0, 20.0],
                        ..s
                    });
                    children.push(f);
                }
                if t.labels {
                    let size = (rect.width().min(rect.height()) * 0.04).clamp(3.0, 50.0);
                    let at = shift * Point::new(rect.x0 + size * 0.5, rect.y1 - size * 1.5);
                    let label = self.sign_label(grid.label(col, row), at, size);
                    children.push(label);
                }
                let mut g = Shape::new(self.engine.new_shape_id(), ShapeKind::Group { children });
                g.name = Some(trf(
                    "signcut.sign_tile_name",
                    &[("t", &grid.label(col, row))],
                ));
                groups.push(g);
            }
        }
        if groups.is_empty() {
            self.status = tr("signcut.sign_tiles_bad");
            return 0;
        }
        let n = groups.len();
        let new_ids: Vec<ShapeId> = groups.iter().map(|g| g.id).collect();
        let mut cmds: Vec<Command> = groups
            .into_iter()
            .map(|shape| Command::AddShape { layer, shape })
            .collect();
        cmds.push(Command::DeleteShapes { shapes: ids });
        if let Err(e) = self.engine.run_batch("Smart Divide", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.select(new_ids);
        self.sc_sign.tiles_report = Some((grid.cols, grid.rows, parts));
        self.status = trf(
            "signcut.sign_tiles_done",
            &[
                ("n", &n.to_string()),
                ("cols", &grid.cols.to_string()),
                ("rows", &grid.rows.to_string()),
            ],
        );
        n
    }

    /// The carrier outline of the selection with the current settings
    /// (nothing changes).
    pub fn sign_carrier_outline(&self) -> BezPath {
        let ids = self.sign_targets();
        if ids.is_empty() {
            return BezPath::new();
        }
        carrier::make(&self.sign_outlines(&ids), &self.sc_sign.carrier)
    }

    /// One carrier outline around the selected letters, put under the
    /// lowest of them. Returns the new object.
    pub fn sign_carrier(&mut self) -> Option<ShapeId> {
        let (ids, _) = self.sign_need_selection()?;
        let outline = carrier::make(&self.sign_outlines(&ids), &self.sc_sign.carrier);
        if outline.elements().is_empty() {
            self.sc_sign.carrier_report = None;
            self.status = tr("signcut.sign_carrier_none");
            return None;
        }
        let pieces = outline
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        // Under the lowest target, in its layer.
        let doc = self.doc();
        let lowest = ids
            .first()
            .and_then(|id| doc.top_level_of(*id))
            .and_then(|top| doc.locate(top).ok())
            .filter(|(l, _)| doc.layer(*l).is_ok_and(|l| !l.locked));
        let layer = match lowest {
            Some((l, _)) => l,
            None => self.sign_layer(&ids)?,
        };
        let shape = self.sign_path(outline, Fill::None, hairline(CARRIER_COLOR));
        let id = shape.id;
        let mut cmds = vec![Command::AddShape { layer, shape }];
        if let Some((l, index)) = lowest {
            cmds.push(Command::Reorder {
                shape: id,
                layer: l,
                index,
            });
        }
        if let Err(e) = self.engine.run_batch("Carrier", &cmds) {
            self.status = e.to_string();
            return None;
        }
        self.sc_sign.carrier_report = Some(pieces);
        self.status = trf("signcut.sign_carrier_done", &[("n", &pieces.to_string())]);
        Some(id)
    }
}

/// A tube drawn as a thick line with round ends.
fn tube_stroke(d: f64) -> Stroke {
    Stroke {
        cap: LineCap::Round,
        join: LineJoin::Round,
        ..Stroke::new(rgb(NEON_COLOR), d)
    }
}

/// The status line after an LED layout.
pub fn led_summary(r: &LedReport) -> String {
    trf(
        "signcut.sign_led_done",
        &[
            ("n", &r.modules.to_string()),
            ("w", &format!("{:.2}", r.power)),
            ("price", &format!("{:.2}", r.price)),
            ("psu", &supply_text(r)),
        ],
    )
}

/// The power supply suggestion as text ("60 W", "2 x 300 W").
pub fn supply_text(r: &LedReport) -> String {
    match r.supply {
        Some(s) if s.count > 1 => format!("{} \u{d7} {:.0} W", s.count, s.watts),
        Some(s) => format!("{:.0} W", s.watts),
        None => "-".to_string(),
    }
}

/// The object holds a bitmap somewhere.
fn has_bitmap(s: &Shape) -> bool {
    match &s.kind {
        ShapeKind::Bitmap { .. } => true,
        ShapeKind::Group { children } => children.iter().any(has_bitmap),
        ShapeKind::ClipFrame { frame, contents } => {
            has_bitmap(frame) || contents.iter().any(has_bitmap)
        }
        _ => false,
    }
}

/// The drawn members of `s` (groups opened) with their outlines in page
/// space; `s` carries its page transform.
fn leaves(s: &Shape, parent: Affine, out: &mut Vec<(Shape, Vec<BezPath>)>) {
    let t = parent * s.transform;
    if let ShapeKind::Group { children } = &s.kind {
        for c in children {
            leaves(c, t, out);
        }
        return;
    }
    let mut on_page = s.clone();
    on_page.transform = t;
    let paths = tracedraw_signcut::outlines(&on_page);
    if !paths.is_empty() {
        out.push((on_page, paths));
    }
}

/// Tools > Sign & Cut > Sign Making: each item opens its tool's window.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let doc = app.has_document() && !app.show_welcome;
    for tool in Tool::ALL {
        if crate::ui::menus::item(ui, tool.menu_key(), "", doc) {
            app.open_sign_tool(tool);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_signcut::sign::led::LedMode;
    use tracedraw_signcut::sign::Sides;

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer to draw on")
    }

    fn line(app: &mut App, pts: &[(f64, f64)]) -> ShapeId {
        let pts: Vec<Point> = pts.iter().map(|p| Point::new(p.0, p.1)).collect();
        app.new_shape(ShapeKind::Path {
            path: tracedraw_core::geometry::polyline_path(&pts, false),
            closed: false,
        })
        .expect("a layer to draw on")
    }

    fn children(app: &App, id: ShapeId) -> Vec<Shape> {
        match app.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(ShapeKind::Group { children }) => children,
            _ => Vec::new(),
        }
    }

    fn top_ids(app: &App) -> Vec<ShapeId> {
        app.page_layers()
            .iter()
            .flat_map(|l| l.shapes.iter().map(|s| s.id))
            .collect()
    }

    #[test]
    fn led_layout_adds_one_group_and_reports_power() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 200.0, 100.0);
        app.select(vec![a]);
        // 40 x 12 modules, 0.48 W: rows 20 apart, modules 50 apart, edge 5.
        app.sc_sign.led.module = 0;
        app.sc_sign.led.spec = LedSpec {
            mode: LedMode::Matrix,
            row_pitch: 20.0,
            module_pitch: 50.0,
            edge: 5.0,
            ..Default::default()
        };
        let before = top_ids(&app).len();
        let n = app.sign_led_layout();
        // Rows floor((90 - 12) / 20) + 1 = 4, 4 a row.
        assert_eq!(n, 16);
        assert_eq!(top_ids(&app).len(), before + 1);
        assert_eq!(app.engine.undo_label(), Some("LED Modules"));
        let group = *top_ids(&app).last().expect("the group");
        assert_eq!(children(&app, group).len(), 16);
        let r = app.sc_sign.led_report.expect("a report");
        assert!((r.power - 16.0 * 0.48).abs() < 1e-9);
        assert_eq!(r.supply.map(|s| s.watts), Some(30.0));
        assert_eq!(app.selection, vec![a]);
        assert!(app.engine.undo().is_ok());
        assert_eq!(top_ids(&app).len(), before);
    }

    #[test]
    fn led_layout_needs_a_selection() {
        let mut app = App::headless();
        rect(&mut app, 0.0, 0.0, 200.0, 100.0);
        app.select(vec![]);
        assert_eq!(app.sign_led_layout(), 0);
        assert_eq!(app.status, tr("signcut.sign_select"));
        assert!(app.sign_led_placements().is_none());
    }

    #[test]
    fn leds_follow_an_open_curve() {
        let mut app = App::headless();
        let l = line(&mut app, &[(0.0, 0.0), (300.0, 0.0)]);
        app.select(vec![l]);
        app.sc_sign.led.spec = LedSpec {
            mode: LedMode::OnCurve,
            module_pitch: 100.0,
            ..Default::default()
        };
        let rev = app.engine.revision();
        let (p, _) = app.sign_led_placements().expect("placements");
        assert_eq!(p.len(), 4);
        assert_eq!(app.engine.revision(), rev);
        assert_eq!(app.sign_led_layout(), 4);
    }

    #[test]
    fn led_groups_are_found_and_their_overlaps_fixed() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 200.0, 100.0);
        app.select(vec![a]);
        // Modules 40 wide only 30 apart along the outline: they overlap.
        app.sc_sign.led.spec = LedSpec {
            mode: LedMode::OnCurve,
            module_pitch: 30.0,
            ..Default::default()
        };
        let n = app.sign_led_layout();
        assert!(n > 10);
        assert_eq!(app.sign_led_select_all(), 1);
        let group = app.selection[0];
        let removed = app.sign_led_fix_overlaps(0.0);
        assert!(removed > 0);
        assert_eq!(children(&app, group).len(), n - removed);
        assert_eq!(app.engine.undo_label(), Some("Fix LED Overlaps"));
        // Nothing overlaps any more.
        assert_eq!(app.sign_led_fix_overlaps(0.0), 0);
    }

    #[test]
    fn neon_splits_a_long_line_and_adds_support_holes() {
        let mut app = App::headless();
        let l = line(&mut app, &[(0.0, 0.0), (2000.0, 0.0)]);
        app.select(vec![l]);
        assert_eq!(app.sign_neon(), 2);
        let r = app.sc_sign.neon_report.expect("a report");
        assert!((r.length - 2000.0).abs() < 1e-6);
        // Two tubes of 1000: 4 holes each.
        assert_eq!(r.supports, 8);
        let group = *top_ids(&app).last().expect("the group");
        let kids = children(&app, group);
        assert_eq!(kids.len(), 10);
        let tube = &kids[0];
        assert_eq!(tube.stroke.as_ref().map(|s| s.width), Some(12.0));
        assert_eq!(app.engine.undo_label(), Some("Neon Tubes"));
        // As outlines: closed shapes.
        app.sc_sign.neon.outline = true;
        app.select(vec![l]);
        app.sign_neon();
        let group = *top_ids(&app).last().expect("the group");
        let kids = children(&app, group);
        assert!(matches!(kids[0].kind, ShapeKind::Path { closed: true, .. }));
    }

    #[test]
    fn neon_tools_divide_make_holes_convert_and_measure() {
        let mut app = App::headless();
        let l = line(&mut app, &[(0.0, 0.0), (2000.0, 0.0)]);
        app.select(vec![l]);
        let rev = app.engine.revision();
        let r = app.sign_neon_measure().expect("a report");
        assert_eq!((r.tubes, r.supports), (2, 8));
        assert_eq!(app.engine.revision(), rev);
        // Tubes only.
        assert_eq!(app.sign_neon_parts(true, false), 2);
        assert_eq!(app.engine.undo_label(), Some("Divide Tubes"));
        let tubes = *top_ids(&app).last().expect("the group");
        assert_eq!(children(&app, tubes).len(), 2);
        // Holes only.
        app.select(vec![l]);
        app.sign_neon_parts(false, true);
        assert_eq!(app.engine.undo_label(), Some("Support Holes"));
        let holes = *top_ids(&app).last().expect("the group");
        assert_eq!(children(&app, holes).len(), 8);
        // The thick tube lines become outlines in one step.
        app.select(vec![tubes]);
        assert_eq!(app.sign_neon_to_outlines(), 2);
        assert_eq!(app.engine.undo_label(), Some("Tubes to Outlines"));
        let kids = children(&app, tubes);
        assert!(kids
            .iter()
            .all(|k| matches!(k.kind, ShapeKind::Path { closed: true, .. })));
        assert!((kids[0].page_path().bounding_box().height() - 12.0).abs() < 0.1);
        // Nothing thick left.
        assert_eq!(app.sign_neon_to_outlines(), 0);
    }

    #[test]
    fn contour_neon_runs_inside_closed_shapes() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 200.0, 200.0);
        app.select(vec![a]);
        app.sc_sign.neon.kind = NeonKind::Contour;
        app.sc_sign.neon.contour = ContourSpec {
            gap: 4.0,
            count: 2,
            spacing: 10.0,
            round: false,
        };
        app.sc_sign.neon.spec.support_spacing = 0.0;
        assert_eq!(app.sign_neon(), 2);
        let r = app.sc_sign.neon_report.expect("a report");
        // Insets 10 and 32: squares of 180 and 136.
        assert!(
            (r.length - 4.0 * (180.0 + 136.0)).abs() < 0.1,
            "{}",
            r.length
        );
    }

    #[test]
    fn sign_box_uses_the_selection_or_the_window_size() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 100.0, 60.0);
        app.select(vec![a]);
        app.sc_sign.sign_box.spec.depth = Sides::all(20.0);
        app.sc_sign.sign_box.spec.holes = true;
        app.sc_sign.sign_box.spec.hole_spacing = 1000.0;
        assert!(app.sign_make_box());
        assert_eq!(app.engine.undo_label(), Some("Sign Box"));
        let group = *top_ids(&app).last().expect("the group");
        let kids = children(&app, group);
        // Outline, four folds, one hole a flange.
        assert_eq!(kids.len(), 1 + 4 + 4);
        let outline = kids[0].page_path();
        let bb = outline.bounding_box();
        assert!((bb.width() - 140.0).abs() < 1e-6 && (bb.height() - 100.0).abs() < 1e-6);
        assert!(!kids[1]
            .stroke
            .as_ref()
            .expect("a fold line")
            .dash
            .is_empty());
        // Nothing selected: the window size on the centre of the page.
        app.select(vec![]);
        app.sc_sign.sign_box.width = 300.0;
        app.sc_sign.sign_box.height = 200.0;
        app.sc_sign.sign_box.spec.holes = false;
        assert!(app.sign_make_box());
        let group = *top_ids(&app).last().expect("the group");
        let bb = children(&app, group)[0].page_path().bounding_box();
        assert!((bb.width() - 340.0).abs() < 1e-6);
        assert!((bb.center().x - app.page_rect().center().x).abs() < 1e-6);
        // With "use the selection" off the size fields count.
        app.select(vec![a]);
        app.sc_sign.sign_box.use_selection = false;
        let plan = app.sign_box_plan().expect("a plan");
        let (w, h) = plan.size.expect("a size");
        assert!((w - 340.0).abs() < 1e-6 && (h - 240.0).abs() < 1e-6);
    }

    #[test]
    fn sign_box_plan_changes_nothing_and_writes_a_legend() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 400.0, 200.0);
        app.select(vec![a]);
        app.sc_sign.sign_box.legend = true;
        app.sc_sign.sign_box.prefix = "Box ".into();
        let rev = app.engine.revision();
        let plan = app.sign_box_plan().expect("a plan");
        assert_eq!(app.engine.revision(), rev);
        assert!(plan.is_current(&app));
        assert_eq!(plan.labels.len(), 1);
        assert!(plan.labels[0].0.starts_with("Box "));
        assert!(app.sign_box_commit(&plan));
        assert_ne!(app.engine.revision(), rev);
        assert!(!plan.is_current(&app));
        let group = *top_ids(&app).last().expect("the group");
        assert!(children(&app, group)
            .iter()
            .any(|k| matches!(k.kind, ShapeKind::Text { .. })));
        assert!(app.engine.undo().is_ok());
        assert!(app.doc().find_shape(group).is_none());
    }

    #[test]
    fn returns_lay_strips_under_the_letters() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 100.0, 100.0, 100.0);
        let b = rect(&mut app, 150.0, 100.0, 50.0, 100.0);
        app.select(vec![a, b]);
        app.sc_sign.returns.spec.depth = 40.0;
        app.sc_sign.returns.spec.overlap = 10.0;
        assert_eq!(app.sign_returns(), 2);
        let (n, total) = app.sc_sign.win.returns.report.expect("a report");
        assert_eq!(n, 2);
        assert!((total - (410.0 + 310.0)).abs() < 1e-6);
        assert_eq!(app.engine.undo_label(), Some("Letter Returns"));
        let group = *top_ids(&app).last().expect("the group");
        let kids = children(&app, group);
        // Two strips with their numbers.
        assert_eq!(kids.len(), 4);
        let strip = kids[0].page_path().bounding_box();
        assert!((strip.width() - 410.0).abs() < 1e-6);
        assert!((strip.y1 - 90.0).abs() < 1e-6, "{strip:?}");
        assert!(matches!(kids[3].kind, ShapeKind::Text { .. }));
    }

    #[test]
    fn returns_plan_follows_the_method() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 100.0, 100.0, 100.0);
        app.select(vec![a]);
        app.sc_sign.returns.labels = false;
        let rev = app.engine.revision();
        // Notched: one strip with three V notches.
        let notched = app.sign_returns_plan().expect("a plan");
        assert_eq!(notched.items.len(), 1);
        // Marked: a plain strip and two marks a corner.
        app.sc_sign.returns.method = ReturnMethod::Marked;
        let marked = app.sign_returns_plan().expect("a plan");
        assert_eq!(marked.items.len(), 1 + 6);
        assert!(marked.items.iter().skip(1).all(|(_, r)| *r == Role::Mark));
        assert!(
            notched.items[0].0.elements().len() > marked.items[0].0.elements().len(),
            "the notches add nodes"
        );
        // Plain, split at 150 and with the face.
        app.sc_sign.returns.method = ReturnMethod::Plain;
        app.sc_sign.returns.spec.max_length = 150.0;
        app.sc_sign.returns.make_face = true;
        let plain = app.sign_returns_plan().expect("a plan");
        assert_eq!(plain.count, 3);
        assert_eq!(plain.items[0].1, Role::Cut);
        let face = plain.items[0].0.bounding_box();
        assert!((face.width() - 98.0).abs() < 0.01);
        assert_eq!(app.engine.revision(), rev);
        // Committed in one step.
        assert_eq!(app.sign_strips_commit(&plain, false), 3);
        assert_eq!(app.engine.undo_label(), Some("Letter Returns"));
        app.select(vec![]);
        assert!(app.sign_returns_plan().is_err());
    }

    #[test]
    fn mould_adds_bases_and_wall_strips() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 100.0, 100.0, 100.0);
        app.select(vec![a]);
        app.sc_sign.mould.labels = false;
        assert_eq!(app.sign_mould(), 1);
        assert_eq!(app.engine.undo_label(), Some("Acrylic Mould"));
        let group = *top_ids(&app).last().expect("the group");
        let kids = children(&app, group);
        assert_eq!(kids.len(), 2);
        let base = kids[0].page_path().bounding_box();
        assert!((base.width() - 104.0).abs() < 0.01);
        let strip = kids[1].page_path().bounding_box();
        assert!((strip.height() - 5.0).abs() < 1e-9);
        assert!(strip.y1 < base.y0);
    }

    #[test]
    fn mould_plan_works_out_the_resin() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 100.0, 100.0, 100.0);
        app.select(vec![a]);
        app.sc_sign.mould.spec.inside = 5.0;
        app.sc_sign.mould.density = 1.2;
        let plan = app.sign_mould_plan().expect("a plan");
        // Base, inner contour, strip.
        assert_eq!(plan.items.len(), 3);
        let f = plan.mould.expect("figures");
        assert!((f.perimeter - 400.0).abs() < 1e-6);
        assert!((f.area - 10_000.0).abs() < 1e-6);
        // 10000 mm2 x 5 mm = 50 ml, 60 g.
        assert!((f.volume - 50.0).abs() < 1e-9 && (f.mass - 60.0).abs() < 1e-9);
    }

    #[test]
    fn smart_divide_replaces_objects_with_tiles() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 250.0, 120.0);
        app.select(vec![a]);
        app.sc_sign.tiles.spec = TileSpec {
            width: 100.0,
            height: 100.0,
            overlap: 10.0,
            gap: 5.0,
        };
        app.sc_sign.tiles.frames = true;
        // Columns at 0, 90, 180; rows at 0, 90.
        assert_eq!(app.sign_divide(), 6);
        assert!(app.doc().find_shape(a).is_none());
        assert_eq!(app.selection.len(), 6);
        assert_eq!(app.sc_sign.tiles_report, Some((3, 2, 6)));
        assert_eq!(app.engine.undo_label(), Some("Smart Divide"));
        // The first tile: piece, frame and label; the piece keeps the fill.
        let kids = children(&app, app.selection[0]);
        assert_eq!(kids.len(), 3);
        let piece = kids[0].page_path().bounding_box();
        assert_eq!(piece, Rect::new(0.0, 0.0, 100.0, 100.0));
        // The second column moved by the gap.
        let kids = children(&app, app.selection[1]);
        let piece = kids[0].page_path().bounding_box();
        assert!((piece.x0 - 95.0).abs() < 1e-6, "{piece:?}");
        assert!(app.engine.undo().is_ok());
        assert!(app.doc().find_shape(a).is_some());
    }

    #[test]
    fn smart_divide_by_table() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 300.0, 100.0);
        app.select(vec![a]);
        app.sc_sign.tiles.method = TileMethod::Table;
        app.sc_sign.tiles.cols = 3;
        app.sc_sign.tiles.rows = 1;
        app.sc_sign.tiles.spec.overlap = 0.0;
        app.sc_sign.tiles.labels = false;
        let g = app.sign_tile_grid().expect("a grid");
        assert_eq!((g.cols, g.rows), (3, 1));
        assert_eq!(app.sign_divide(), 3);
        let piece = children(&app, app.selection[1])[0]
            .page_path()
            .bounding_box();
        assert!((piece.x0 - 100.0).abs() < 1e-6 && (piece.width() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn carrier_goes_under_the_letters() {
        let mut app = App::headless();
        let below = rect(&mut app, 500.0, 500.0, 10.0, 10.0);
        let a = rect(&mut app, 0.0, 0.0, 50.0, 50.0);
        let b = rect(&mut app, 65.0, 0.0, 50.0, 50.0);
        app.select(vec![a, b]);
        let rev = app.engine.revision();
        assert!(!app.sign_carrier_outline().elements().is_empty());
        assert_eq!(app.engine.revision(), rev);
        let id = app.sign_carrier().expect("a carrier");
        assert_eq!(app.sc_sign.carrier_report, Some(1));
        let order = top_ids(&app);
        let pos = |x: ShapeId| order.iter().position(|o| *o == x).expect("on the page");
        assert!(pos(below) < pos(id) && pos(id) < pos(a));
        let bb = app.doc().shape_on_page(id).expect("carrier").bounds();
        assert!((bb.width() - 135.0).abs() < 0.1);
        assert_eq!(app.engine.undo_label(), Some("Carrier"));
    }

    #[test]
    fn a_rectangle_carrier_has_its_margins() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 50.0, 50.0);
        let b = rect(&mut app, 65.0, 0.0, 50.0, 50.0);
        app.select(vec![a, b]);
        app.sc_sign.carrier.method = carrier::CarrierMethod::Rectangle;
        app.sc_sign.carrier.margins = Sides::all(5.0);
        let id = app.sign_carrier().expect("a carrier");
        let bb = app.doc().shape_on_page(id).expect("carrier").bounds();
        assert!((bb.width() - 125.0).abs() < 0.1 && (bb.height() - 60.0).abs() < 0.1);
    }

    #[test]
    fn the_menu_opens_a_tool_window() {
        let mut app = App::headless();
        app.open_sign_tool(Tool::Tiles);
        assert_eq!(app.sc_sign.open, Some(Tool::Tiles));
        assert!(app
            .sc_windows
            .is_open(crate::signcut_tools::ToolId::SmartDivide));
    }
}
