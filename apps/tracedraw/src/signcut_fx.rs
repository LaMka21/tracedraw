//! Sign & Cut, Effects and Patterns: settings and operations (hatching
//! lines, grids, puzzles, circle fills, cast shadows and random copies).
//! The geometry is in `tracedraw_signcut::fx`; the windows are
//! `ui/signcut_win_fx.rs`. Each operation is worked out first
//! ([`App::fx_compute`], nothing changes) and then applied in one undo
//! step ([`App::fx_commit`]). See docs/behavior/sign-and-cut-fx.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_tools::ToolId;
use egui::Ui;
use tracedraw_core::geometry::{Affine, BezPath, Point, Rect, Shape as _};
use tracedraw_core::{Color, Command, Fill, LayerId, Shape, ShapeId, ShapeKind, Stroke};
use tracedraw_signcut::fx::{
    self, CircleSpec, GridSpec, HatchSpec, PuzzleSpec, Region, ScatterSpec, ShadowSpec,
};

/// The methods of the Effects window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Effect {
    #[default]
    Puzzle,
    Circles,
    Shadow,
    Copies,
}

impl Effect {
    #[cfg(test)]
    pub const ALL: [Effect; 4] = [
        Effect::Puzzle,
        Effect::Circles,
        Effect::Shadow,
        Effect::Copies,
    ];

    /// The operation this method runs.
    pub fn op(self) -> Op {
        match self {
            Effect::Puzzle => Op::Puzzle,
            Effect::Circles => Op::Circles,
            Effect::Shadow => Op::Shadow,
            Effect::Copies => Op::Copies,
        }
    }
}

/// The methods of the Grid window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GridMethod {
    /// A grid of cells.
    #[default]
    Grid,
    /// Lines dividing a box into equal strips.
    Lines,
}

impl GridMethod {
    pub fn op(self) -> Op {
        match self {
            GridMethod::Grid => Op::Grid,
            GridMethod::Lines => Op::Lines,
        }
    }
}

/// The Effects window's settings (what its presets keep).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EffectSettings {
    pub method: Effect,
    pub puzzle: PuzzleSpec,
    /// The puzzle as separate pieces (else the cut lines only).
    pub puzzle_pieces: bool,
    pub circles: CircleSpec,
    pub shadow: ShadowSpec,
    pub shadow_color: [u8; 3],
    /// 1 is opaque.
    pub shadow_opacity: f64,
    pub scatter: ScatterSpec,
}

impl Default for EffectSettings {
    fn default() -> Self {
        EffectSettings {
            method: Effect::Puzzle,
            puzzle: PuzzleSpec::default(),
            puzzle_pieces: false,
            circles: CircleSpec::default(),
            shadow: ShadowSpec::default(),
            shadow_color: [64, 64, 64],
            shadow_opacity: 0.5,
            scatter: ScatterSpec::default(),
        }
    }
}

/// The Grid window's settings.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GridSettings {
    pub method: GridMethod,
    pub grid: GridSpec,
    /// The grid fills the selection's box (else it starts at the origin
    /// with the cell size).
    pub fill_selection: bool,
    /// The grid as rectangles (else as lines).
    pub cells: bool,
    /// The grid's bottom-left corner on the page (mm).
    pub origin: Point,
    /// Dividing lines: how many and which way.
    pub lines_count: usize,
    pub lines_vertical: bool,
}

impl Default for GridSettings {
    fn default() -> Self {
        GridSettings {
            method: GridMethod::Grid,
            grid: GridSpec::default(),
            fill_selection: false,
            cells: false,
            origin: Point::new(10.0, 10.0),
            lines_count: 3,
            lines_vertical: false,
        }
    }
}

/// One window's state: the result worked out with Apply.
#[derive(Debug, Clone, Default)]
pub struct WindowState {
    pub computed: Option<Computed>,
    /// Show the objects as they are instead of the result.
    pub original: bool,
}

/// The Effects and Patterns settings and window states (`app.sc_fx`).
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub effects: EffectSettings,
    pub hatch: HatchSpec,
    pub grid: GridSettings,
    /// The shape the random copies go into (else the page).
    pub container: Option<ShapeId>,
    pub win_effects: WindowState,
    pub win_hatch: WindowState,
    pub win_grid: WindowState,
}

/// An operation of the area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Hatch,
    Grid,
    Lines,
    Puzzle,
    Circles,
    Shadow,
    Copies,
}

impl Op {
    /// The undo label.
    pub fn label(self) -> &'static str {
        match self {
            Op::Hatch => "Hatching",
            Op::Grid => "Grid",
            Op::Lines => "Create Lines",
            Op::Puzzle => "Puzzle",
            Op::Circles => "Circle Fill",
            Op::Shadow => "Cast Shadow",
            Op::Copies => "Random Copies",
        }
    }

    /// Works on the selection (else it needs none).
    pub fn needs_selection(self) -> bool {
        !matches!(self, Op::Grid | Op::Lines)
    }
}

/// What an operation adds, worked out without changing the drawing.
#[derive(Debug, Clone)]
pub enum Plan {
    /// Open curves (one per object) in one group.
    Hatch {
        layer: LayerId,
        curves: Vec<(BezPath, Stroke)>,
        lines: usize,
    },
    /// New objects with the default look in one group.
    Kinds {
        layer: LayerId,
        kinds: Vec<ShapeKind>,
    },
    /// Per object: its id, layer, the object (for its look) and the cut
    /// lines or pieces.
    Puzzle {
        pieces: bool,
        items: Vec<(ShapeId, LayerId, Shape, Vec<BezPath>)>,
    },
    /// Per object: its layer and its circles.
    Circles {
        items: Vec<(LayerId, Vec<fx::Disc>)>,
    },
    /// Per object: the top-level object it goes behind, its layer and the
    /// shadow's outline.
    Shadow {
        items: Vec<(ShapeId, LayerId, BezPath)>,
        color: [u8; 3],
        opacity: f64,
    },
    /// The objects copied and the places of the copies.
    Copies {
        layer: LayerId,
        sources: Vec<Shape>,
        places: Vec<Affine>,
        want: usize,
    },
}

/// A worked-out operation and what it was worked out from.
#[derive(Debug, Clone)]
pub struct Computed {
    pub op: Op,
    /// The settings, selection and drawing it was worked out from; it is
    /// current while [`App::fx_inputs`] still gives this.
    pub inputs: String,
    pub plan: Plan,
}

impl Computed {
    /// How many objects (or hatch lines, cut lines, pieces) it adds.
    pub fn count(&self) -> usize {
        match &self.plan {
            Plan::Hatch { lines, .. } => *lines,
            Plan::Kinds { kinds, .. } => kinds.len(),
            Plan::Puzzle { items, .. } => items.iter().map(|i| i.3.len()).sum(),
            Plan::Circles { items } => items.iter().map(|i| i.1.len()).sum(),
            Plan::Shadow { items, .. } => items.len(),
            Plan::Copies { places, .. } => places.len(),
        }
    }

    /// The result in a few words ("12 puzzle piece(s)").
    pub fn describe(&self) -> String {
        let n = self.count().to_string();
        let key = match &self.plan {
            Plan::Hatch { .. } => "signcut.wfx_res_hatch",
            Plan::Kinds { .. } => "signcut.wfx_res_objects",
            Plan::Puzzle { pieces: true, .. } => "signcut.wfx_res_pieces",
            Plan::Puzzle { .. } => "signcut.wfx_res_cut_lines",
            Plan::Circles { .. } => "signcut.wfx_res_circles",
            Plan::Shadow { .. } => "signcut.wfx_res_shadows",
            Plan::Copies { want, .. } => {
                return trf(
                    "signcut.wfx_res_copies",
                    &[("n", &n), ("want", &want.to_string())],
                )
            }
        };
        trf(key, &[("n", &n)])
    }

    /// The new outlines in page space, for previews.
    pub fn outlines(&self) -> Vec<BezPath> {
        match &self.plan {
            Plan::Hatch { curves, .. } => curves.iter().map(|(p, _)| p.clone()).collect(),
            Plan::Kinds { kinds, .. } => kinds.iter().filter_map(kind_path).collect(),
            Plan::Puzzle { items, .. } => items.iter().flat_map(|i| i.3.clone()).collect(),
            Plan::Circles { items } => items
                .iter()
                .flat_map(|(_, d)| d.iter())
                .map(|d| tracedraw_core::geometry::Circle::new(d.center, d.radius).to_path(0.05))
                .collect(),
            Plan::Shadow { items, .. } => items.iter().map(|i| i.2.clone()).collect(),
            Plan::Copies {
                sources, places, ..
            } => {
                let outlines: Vec<BezPath> = sources
                    .iter()
                    .flat_map(tracedraw_signcut::outlines)
                    .collect();
                places
                    .iter()
                    .flat_map(|t| outlines.iter().map(move |o| *t * o.clone()))
                    .collect()
            }
        }
    }
}

/// The page-space outline of a new rectangle or curve.
fn kind_path(k: &ShapeKind) -> Option<BezPath> {
    match k {
        ShapeKind::Rect { rect, .. } => Some(rect.to_path(0.1)),
        ShapeKind::Path { path, .. } => Some(path.clone()),
        _ => None,
    }
}

/// A hairline in the colour of `s`'s outline, else of its plain fill,
/// else black: the lines an effect draws for an object.
fn hairline_for(s: &Shape) -> Stroke {
    let color = match (&s.stroke, &s.fill) {
        (Some(st), _) => st.color,
        (None, Fill::Solid(c)) => *c,
        _ => Color::BLACK,
    };
    Stroke::hairline(color)
}

fn path_of(lines: &[Vec<Point>]) -> BezPath {
    let mut p = BezPath::new();
    for l in lines {
        p.extend(fx::polyline_path(l));
    }
    p
}

fn open_curve(id: ShapeId, path: BezPath, stroke: Stroke) -> Shape {
    let mut s = Shape::new(
        id,
        ShapeKind::Path {
            path,
            closed: false,
        },
    );
    s.fill = Fill::None;
    s.stroke = Some(stroke);
    s
}

fn rect_polygon(p: Rect) -> Vec<Point> {
    vec![
        Point::new(p.x0, p.y0),
        Point::new(p.x1, p.y0),
        Point::new(p.x1, p.y1),
        Point::new(p.x0, p.y1),
    ]
}

impl App {
    /// The selected objects the effects work on, in stacking order,
    /// locked ones left out. Unlike measuring, an empty selection is
    /// not the whole page.
    pub fn fx_targets(&self) -> Vec<ShapeId> {
        if self.selection.is_empty() {
            return Vec::new();
        }
        let doc = self.doc();
        self.signcut_targets()
            .into_iter()
            .filter(|id| doc.find_shape(*id).is_some_and(|s| !s.locked))
            .collect()
    }

    /// The objects `op` works on: the targets, for random copies without
    /// the container.
    pub fn fx_sources(&self, op: Op) -> Vec<ShapeId> {
        if !op.needs_selection() {
            return Vec::new();
        }
        let container = self.fx_container();
        self.fx_targets()
            .into_iter()
            .filter(|id| op != Op::Copies || Some(*id) != container)
            .collect()
    }

    /// The layer holding `id` (through its groups), else the active one.
    fn fx_layer_of(&self, id: ShapeId) -> Option<LayerId> {
        let doc = self.doc();
        let top = doc.top_level_of(id).unwrap_or(id);
        doc.locate(top)
            .ok()
            .map(|(l, _)| l)
            .or_else(|| self.active_layer())
    }

    /// Page bounds of the selection.
    pub fn fx_selection_bounds(&self) -> Option<Rect> {
        let doc = self.doc();
        self.selection
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .map(|s| s.bounds())
            .filter(|b| {
                b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()
            })
            .reduce(|a, b| a.union(b))
    }

    /// What a result of `op` depends on: its settings, the objects it
    /// works on and the drawing's revision. A worked-out result is
    /// current while this stays the same.
    pub fn fx_inputs(&self, op: Op) -> String {
        let s = &self.sc_fx;
        let settings = match op {
            Op::Hatch => serde_json::to_string(&s.hatch),
            Op::Grid | Op::Lines => serde_json::to_string(&s.grid),
            Op::Puzzle | Op::Circles | Op::Shadow => serde_json::to_string(&s.effects),
            Op::Copies => {
                serde_json::to_string(&s.effects).map(|j| format!("{j} {:?}", self.fx_container()))
            }
        }
        .unwrap_or_default();
        let selection: Vec<ShapeId> = if op.needs_selection() {
            self.fx_targets()
        } else {
            self.selection.clone()
        };
        format!("{op:?} {} {selection:?} {settings}", self.engine.revision())
    }

    /// Work `op` out with the current settings and selection; nothing in
    /// the drawing changes. `None` with the reason in the status line
    /// when there is nothing to do.
    pub fn fx_compute(&mut self, op: Op) -> Option<Computed> {
        let ids = self.fx_sources(op);
        if op.needs_selection() && ids.is_empty() {
            self.status = tr("signcut.fx_select_first");
            return None;
        }
        let plan = match op {
            Op::Hatch => self.fx_plan_hatch(&ids),
            Op::Grid => self.fx_plan_grid(),
            Op::Lines => self.fx_plan_lines(),
            Op::Puzzle => self.fx_plan_puzzle(&ids),
            Op::Circles => self.fx_plan_circles(&ids),
            Op::Shadow => self.fx_plan_shadow(&ids),
            Op::Copies => self.fx_plan_copies(&ids),
        };
        let c = plan.map(|plan| Computed {
            op,
            inputs: self.fx_inputs(op),
            plan,
        });
        // Copies report how many of the asked-for number fit, even none.
        if c.as_ref()
            .is_none_or(|c| c.count() == 0 && op != Op::Copies)
        {
            self.status = tr("signcut.fx_nothing");
            return None;
        }
        c
    }

    fn fx_plan_hatch(&self, ids: &[ShapeId]) -> Option<Plan> {
        let spec = self.sc_fx.hatch;
        let doc = self.doc();
        let mut lines = 0;
        let curves: Vec<(BezPath, Stroke)> = ids
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .filter_map(|s| {
                let found = fx::hatch(&tracedraw_signcut::outlines(&s), &spec);
                lines += found.len();
                (!found.is_empty()).then(|| (path_of(&found), hairline_for(&s)))
            })
            .collect();
        let layer = ids.first().and_then(|id| self.fx_layer_of(*id))?;
        Some(Plan::Hatch {
            layer,
            curves,
            lines,
        })
    }

    /// The grid's origin and cells: filling the selection's box when
    /// asked and something is selected, else from the settings' origin.
    pub fn fx_grid_layout(&self) -> Option<(Point, GridSpec)> {
        let s = &self.sc_fx.grid;
        if s.fill_selection {
            if let Some(b) = self.fx_selection_bounds() {
                return Some((Point::new(b.x0, b.y0), s.grid.fill(b)?));
            }
        }
        Some((s.origin, s.grid))
    }

    fn fx_plan_grid(&self) -> Option<Plan> {
        let (origin, spec) = self.fx_grid_layout()?;
        let layer = self.active_layer()?;
        let kinds: Vec<ShapeKind> = if self.sc_fx.grid.cells {
            fx::grid_cells(origin, &spec)
                .into_iter()
                .map(|rect| ShapeKind::Rect {
                    rect,
                    radius: 0.0,
                    corners: None,
                })
                .collect()
        } else {
            fx::grid_lines(origin, &spec)
                .into_iter()
                .map(|l| ShapeKind::Path {
                    path: fx::polyline_path(&l),
                    closed: false,
                })
                .collect()
        };
        Some(Plan::Kinds { layer, kinds })
    }

    /// The box the dividing lines divide: the selection's, else the page.
    pub fn fx_lines_box(&self) -> Rect {
        self.fx_selection_bounds()
            .unwrap_or_else(|| self.page_rect())
    }

    fn fx_plan_lines(&self) -> Option<Plan> {
        let b = self.fx_lines_box();
        let layer = self.active_layer()?;
        let g = &self.sc_fx.grid;
        let kinds = fx::across_lines(b, g.lines_count, g.lines_vertical)
            .into_iter()
            .map(|l| ShapeKind::Path {
                path: fx::polyline_path(&l),
                closed: false,
            })
            .collect();
        Some(Plan::Kinds { layer, kinds })
    }

    fn fx_plan_puzzle(&self, ids: &[ShapeId]) -> Option<Plan> {
        let spec = self.sc_fx.effects.puzzle;
        let pieces = self.sc_fx.effects.puzzle_pieces;
        let doc = self.doc();
        let mut items = Vec::new();
        for id in ids {
            let Some(s) = doc.shape_on_page(*id) else {
                continue;
            };
            let parts = fx::puzzle(&tracedraw_signcut::outlines(&s), &spec, pieces);
            if parts.is_empty() {
                continue;
            }
            if let Some(layer) = self.fx_layer_of(*id) {
                items.push((*id, layer, s, parts));
            }
        }
        Some(Plan::Puzzle { pieces, items })
    }

    fn fx_plan_circles(&self, ids: &[ShapeId]) -> Option<Plan> {
        let spec = self.sc_fx.effects.circles;
        let doc = self.doc();
        let items = ids
            .iter()
            .filter_map(|id| Some((self.fx_layer_of(*id)?, doc.shape_on_page(*id)?)))
            .map(|(l, s)| (l, fx::circle_fill(&tracedraw_signcut::outlines(&s), &spec)))
            .filter(|(_, d)| !d.is_empty())
            .collect();
        Some(Plan::Circles { items })
    }

    fn fx_plan_shadow(&self, ids: &[ShapeId]) -> Option<Plan> {
        let e = &self.sc_fx.effects;
        let spec = e.shadow;
        let opacity = if e.shadow_opacity.is_finite() {
            e.shadow_opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let doc = self.doc();
        let mut items = Vec::new();
        for id in ids {
            let Some(s) = doc.shape_on_page(*id) else {
                continue;
            };
            let outline = fx::shadow_outline(&tracedraw_signcut::outlines(&s));
            let Some(t) = fx::shadow_transform(s.bounds(), &spec) else {
                continue;
            };
            if outline.elements().is_empty() {
                continue;
            }
            let top = doc.top_level_of(*id).unwrap_or(*id);
            if let Some(layer) = self.fx_layer_of(top) {
                items.push((top, layer, t * outline));
            }
        }
        Some(Plan::Shadow {
            items,
            color: e.shadow_color,
            opacity,
        })
    }

    /// The region the random copies go into: the container's inside, or
    /// the page.
    pub fn fx_copies_region(&self) -> Region {
        let doc = self.doc();
        match self.fx_container().and_then(|c| doc.shape_on_page(c)) {
            Some(c) => {
                Region::from_paths(&tracedraw_signcut::outlines(&c), fx::region::FLATTEN_TOL)
            }
            None => Region::from_polygons(vec![rect_polygon(self.page_rect())]),
        }
    }

    fn fx_plan_copies(&self, ids: &[ShapeId]) -> Option<Plan> {
        let doc = self.doc();
        let region = self.fx_copies_region();
        let sources: Vec<Shape> = ids.iter().filter_map(|id| doc.shape_on_page(*id)).collect();
        let bounds = sources
            .iter()
            .map(|s| s.bounds())
            .reduce(|a, b| a.union(b))?;
        let layer = self.active_layer()?;
        let spec = self.sc_fx.effects.scatter;
        let places = fx::scatter(bounds, &region, &spec);
        Some(Plan::Copies {
            layer,
            sources,
            places,
            want: spec.count,
        })
    }

    /// Add each set of `groups` as one group on its layer, then run
    /// `extra`, all in one undo step; select the new groups. Returns them.
    fn fx_add_groups(
        &mut self,
        label: &'static str,
        groups: Vec<(LayerId, Vec<Shape>)>,
        extra: Vec<Command>,
    ) -> Vec<ShapeId> {
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for (layer, children) in groups {
            if children.is_empty() {
                continue;
            }
            let gid = self.engine.new_shape_id();
            let mut g = Shape::new(gid, ShapeKind::Group { children });
            g.stroke = None;
            cmds.push(Command::AddShape { layer, shape: g });
            ids.push(gid);
        }
        if ids.is_empty() {
            return Vec::new();
        }
        cmds.extend(extra);
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return Vec::new();
        }
        self.select(ids.clone());
        ids
    }

    /// A new object of `kind` with the document's default look.
    fn fx_default_shape(&mut self, kind: ShapeKind) -> Shape {
        let mut s = Shape::new(self.engine.new_shape_id(), kind);
        self.doc().defaults.apply_to(&mut s);
        s
    }

    /// Apply a worked-out result in one undo step and select what it
    /// made. Returns its count (0 when nothing was added).
    pub fn fx_commit(&mut self, c: &Computed) -> usize {
        let label = c.op.label();
        let n = c.count();
        let done = match &c.plan {
            Plan::Hatch { layer, curves, .. } => {
                let children = curves
                    .iter()
                    .map(|(p, st)| open_curve(self.engine.new_shape_id(), p.clone(), st.clone()))
                    .collect();
                !self
                    .fx_add_groups(label, vec![(*layer, children)], Vec::new())
                    .is_empty()
            }
            Plan::Kinds { layer, kinds } => {
                let children = kinds
                    .iter()
                    .map(|k| self.fx_default_shape(k.clone()))
                    .collect();
                !self
                    .fx_add_groups(label, vec![(*layer, children)], Vec::new())
                    .is_empty()
            }
            Plan::Puzzle { pieces, items } => {
                let mut groups = Vec::new();
                let mut gone = Vec::new();
                for (id, layer, s, parts) in items {
                    let children = parts
                        .iter()
                        .map(|path| {
                            let nid = self.engine.new_shape_id();
                            if *pieces {
                                let kind = ShapeKind::Path {
                                    path: path.clone(),
                                    closed: true,
                                };
                                let mut c = Shape::new(nid, kind);
                                if matches!(s.kind, ShapeKind::Group { .. }) {
                                    self.doc().defaults.apply_to(&mut c);
                                } else {
                                    c.fill = s.fill.clone();
                                    c.stroke = s.stroke.clone();
                                }
                                c
                            } else {
                                open_curve(nid, path.clone(), hairline_for(s))
                            }
                        })
                        .collect();
                    groups.push((*layer, children));
                    gone.push(*id);
                }
                let extra = if *pieces && !gone.is_empty() {
                    vec![Command::DeleteShapes { shapes: gone }]
                } else {
                    Vec::new()
                };
                !self.fx_add_groups(label, groups, extra).is_empty()
            }
            Plan::Circles { items } => {
                let groups = items
                    .iter()
                    .map(|(layer, discs)| {
                        let children = discs
                            .iter()
                            .map(|d| {
                                let (c, r) = (d.center, d.radius);
                                self.fx_default_shape(ShapeKind::Ellipse {
                                    rect: Rect::new(c.x - r, c.y - r, c.x + r, c.y + r),
                                    arc: None,
                                })
                            })
                            .collect();
                        (*layer, children)
                    })
                    .collect();
                !self.fx_add_groups(label, groups, Vec::new()).is_empty()
            }
            Plan::Shadow {
                items,
                color,
                opacity,
            } => {
                let [r, g, b] = *color;
                let mut cmds = Vec::new();
                let mut new_ids = Vec::new();
                for (top, layer, path) in items {
                    let sid = self.engine.new_shape_id();
                    let kind = ShapeKind::Path {
                        path: path.clone(),
                        closed: true,
                    };
                    let mut s = Shape::new(sid, kind);
                    s.fill = Fill::Solid(Color::rgb8(r, g, b));
                    s.stroke = None;
                    s.opacity = *opacity;
                    cmds.push(Command::AddShape {
                        layer: *layer,
                        shape: s,
                    });
                    cmds.push(Command::OrderRelative {
                        shapes: vec![sid],
                        reference: *top,
                        in_front: false,
                    });
                    new_ids.push(sid);
                }
                self.fx_run(label, cmds, new_ids)
            }
            Plan::Copies {
                layer,
                sources,
                places,
                want,
            } => {
                let mut cmds = Vec::new();
                let mut new_ids = Vec::new();
                for t in places {
                    let copies: Vec<Shape> = sources
                        .iter()
                        .map(|s| {
                            let id = self.engine.new_shape_id();
                            let mut c = crate::app::reid_pub(s.clone(), id, &mut self.engine);
                            c.transform = *t * s.transform;
                            c.uid = None;
                            c
                        })
                        .collect();
                    let shape = if copies.len() == 1 {
                        copies.into_iter().next()
                    } else {
                        let gid = self.engine.new_shape_id();
                        let mut g = Shape::new(gid, ShapeKind::Group { children: copies });
                        g.stroke = None;
                        Some(g)
                    };
                    if let Some(shape) = shape {
                        new_ids.push(shape.id);
                        cmds.push(Command::AddShape {
                            layer: *layer,
                            shape,
                        });
                    }
                }
                // Fewer copies than asked (or none) is reported, not an error.
                let ok = self.fx_run(label, cmds, new_ids);
                self.status = trf(
                    "signcut.fx_copies_done",
                    &[("n", &n.to_string()), ("want", &want.to_string())],
                );
                return if ok { n } else { 0 };
            }
        };
        if !done {
            self.status = tr("signcut.fx_nothing");
            return 0;
        }
        self.status = match &c.plan {
            Plan::Hatch { .. } => trf("signcut.fx_hatch_done", &[("n", &n.to_string())]),
            Plan::Kinds { .. } => trf("signcut.fx_objects_done", &[("n", &n.to_string())]),
            Plan::Puzzle { pieces: true, .. } => {
                trf("signcut.fx_pieces_done", &[("n", &n.to_string())])
            }
            Plan::Puzzle { .. } => trf("signcut.fx_cut_lines_done", &[("n", &n.to_string())]),
            Plan::Circles { .. } => trf("signcut.fx_circles_done", &[("n", &n.to_string())]),
            Plan::Shadow { .. } => trf("signcut.fx_shadows_done", &[("n", &n.to_string())]),
            Plan::Copies { .. } => String::new(),
        };
        n
    }

    /// Run `cmds` as one undo step and select `new_ids`; false when there
    /// is nothing to run or the engine refuses.
    fn fx_run(&mut self, label: &'static str, cmds: Vec<Command>, new_ids: Vec<ShapeId>) -> bool {
        if new_ids.is_empty() {
            return false;
        }
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.select(new_ids);
        true
    }

    /// Take the selected object as the container of the random copies.
    pub fn fx_pick_container(&mut self) -> bool {
        let [id] = self.selection[..] else {
            self.status = tr("signcut.select_one");
            return false;
        };
        self.sc_fx.container = Some(id);
        true
    }

    /// The container while it is still in the drawing.
    pub fn fx_container(&self) -> Option<ShapeId> {
        self.sc_fx
            .container
            .filter(|id| self.doc().find_shape(*id).is_some())
    }
}

/// Run an operation at once, without the window (tests).
#[cfg(test)]
impl App {
    /// Work `op` out and apply it at once. Returns its count.
    pub fn fx_run_op(&mut self, op: Op) -> usize {
        match self.fx_compute(op) {
            Some(c) => self.fx_commit(&c),
            None => 0,
        }
    }

    /// Fill the selected closed shapes with hatch lines: one open curve
    /// per object, all in one group. Returns the number of lines.
    pub fn fx_hatch(&mut self) -> usize {
        self.fx_run_op(Op::Hatch)
    }

    /// Make the grid: one curve per line or one rectangle per cell, in
    /// one group. Returns the number of objects.
    pub fn fx_grid(&mut self) -> usize {
        self.fx_run_op(Op::Grid)
    }

    /// Lines across the selection's box (or the page). Returns how many.
    pub fn fx_lines(&mut self) -> usize {
        self.fx_run_op(Op::Lines)
    }

    /// Cut the selected shapes into a puzzle: per object one group of cut
    /// lines over it, or of pieces in its place. Returns the number of
    /// lines or pieces.
    pub fn fx_puzzle(&mut self) -> usize {
        self.fx_run_op(Op::Puzzle)
    }

    /// Fill the selected closed shapes with circles, one group per object.
    /// Returns the number of circles.
    pub fn fx_circle_fill(&mut self) -> usize {
        self.fx_run_op(Op::Circles)
    }

    /// Give each selected object a flat shadow, filled with the shadow
    /// colour and opacity, placed right behind the object (behind its
    /// group when it is inside one). Returns how many.
    pub fn fx_cast_shadow(&mut self) -> usize {
        self.fx_run_op(Op::Shadow)
    }

    /// Copy the selection (the container left out) to random places
    /// inside the container or the page. Each place gets one copy of
    /// each object, grouped when there are several. Returns how many
    /// places were filled.
    pub fn fx_random_copies(&mut self) -> usize {
        self.fx_run_op(Op::Copies)
    }
}

/// Tools > Sign & Cut > Effects and Patterns: each item opens the window
/// of its tool with the method chosen.
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::ui::menus::item;
    let doc = app.has_document() && !app.show_welcome;
    if item(ui, "menu.tools.signcut_fx_hatch", "", doc) {
        app.signcut_tool(ToolId::Hatching);
    }
    for (method, key) in [
        (GridMethod::Grid, "menu.tools.signcut_fx_grid"),
        (GridMethod::Lines, "menu.tools.signcut_fx_lines"),
    ] {
        if item(ui, key, "", doc) {
            app.sc_fx.grid.method = method;
            app.signcut_tool(ToolId::Grid);
        }
    }
    for (method, key) in [
        (Effect::Puzzle, "menu.tools.signcut_fx_puzzle"),
        (Effect::Circles, "menu.tools.signcut_fx_circles"),
        (Effect::Shadow, "menu.tools.signcut_fx_shadow"),
        (Effect::Copies, "menu.tools.signcut_fx_copies"),
    ] {
        if item(ui, key, "", doc) {
            app.sc_fx.effects.method = method;
            app.open_signcut(crate::signcut::Section::Fx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_signcut::fx::{HatchStyle, ShadowKind};

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer to draw on")
    }

    fn children(app: &App, id: ShapeId) -> Vec<Shape> {
        match app.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(ShapeKind::Group { children }) => children,
            other => panic!("not a group: {other:?}"),
        }
    }

    fn layer_ids(app: &App) -> Vec<ShapeId> {
        app.page_layers()
            .iter()
            .flat_map(|l| l.shapes.iter().map(|s| s.id))
            .collect()
    }

    #[test]
    fn hatching_adds_one_group_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        rect(&mut app, 50.0, 0.0, 10.0, 10.0);
        // Nothing selected: nothing happens.
        app.select(vec![]);
        assert_eq!(app.fx_hatch(), 0);
        app.select(vec![a]);
        assert_eq!(app.fx_hatch(), 5);
        assert_eq!(app.engine.undo_label(), Some("Hatching"));
        let [g] = app.selection[..] else {
            panic!("one group selected");
        };
        let kids = children(&app, g);
        assert_eq!(kids.len(), 1);
        let path = kids[0].page_path();
        assert_eq!(path.segments().count(), 5);
        assert!(matches!(
            kids[0].kind,
            ShapeKind::Path { closed: false, .. }
        ));
        let width = kids[0].stroke.as_ref().map_or(0.0, |s| s.width);
        assert!((width - Stroke::HAIRLINE).abs() < 1e-12);
        // Snake: one zigzag.
        app.sc_fx.hatch.style = HatchStyle::Snake;
        app.select(vec![a]);
        assert_eq!(app.fx_hatch(), 1);
        assert!(app.engine.undo().is_ok());
        assert!(app.engine.undo().is_ok());
        assert_eq!(layer_ids(&app).len(), 2);
    }

    #[test]
    fn grids_come_as_lines_or_cells_and_fill_the_selection() {
        let mut app = App::headless();
        app.sc_fx.grid.grid = GridSpec {
            cols: 3,
            rows: 2,
            cell_w: 10.0,
            cell_h: 5.0,
        };
        app.sc_fx.grid.origin = Point::new(20.0, 30.0);
        app.select(vec![]);
        assert_eq!(app.fx_grid(), 4 + 3);
        assert_eq!(app.engine.undo_label(), Some("Grid"));
        let g = app.selection[0];
        let b = app.doc().shape_on_page(g).expect("grid").bounds();
        assert!((b.x0 - 20.0).abs() < 1e-9 && (b.x1 - 50.0).abs() < 1e-9);
        assert!((b.y1 - 40.0).abs() < 1e-9);
        app.sc_fx.grid.cells = true;
        assert_eq!(app.fx_grid(), 6);
        let kids = children(&app, app.selection[0]);
        assert!(kids
            .iter()
            .all(|k| matches!(k.kind, ShapeKind::Rect { .. })));
        // Fill the selection's box.
        let r = rect(&mut app, 100.0, 100.0, 60.0, 20.0);
        app.select(vec![r]);
        app.sc_fx.grid.fill_selection = true;
        assert_eq!(app.fx_grid(), 6);
        let kids = children(&app, app.selection[0]);
        let first = kids[0].bounds();
        assert!((first.width() - 20.0).abs() < 1e-9 && (first.height() - 10.0).abs() < 1e-9);
        assert!((first.x0 - 100.0).abs() < 1e-9 && (first.y0 - 100.0).abs() < 1e-9);
    }

    #[test]
    fn lines_divide_the_selection_or_the_page() {
        let mut app = App::headless();
        let r = rect(&mut app, 0.0, 0.0, 40.0, 30.0);
        app.select(vec![r]);
        app.sc_fx.grid.lines_count = 2;
        assert_eq!(app.fx_lines(), 2);
        assert_eq!(app.engine.undo_label(), Some("Create Lines"));
        let kids = children(&app, app.selection[0]);
        let b = kids[1].bounds();
        assert!((b.y0 - 20.0).abs() < 1e-9 && (b.width() - 40.0).abs() < 1e-9);
        app.select(vec![]);
        app.sc_fx.grid.lines_vertical = true;
        app.sc_fx.grid.lines_count = 1;
        assert_eq!(app.fx_lines(), 1);
        let kids = children(&app, app.selection[0]);
        let page = app.page_rect();
        assert!((kids[0].bounds().x0 - page.width() / 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_puzzle_adds_cut_lines_or_replaces_the_shape_with_pieces() {
        let mut app = App::headless();
        let r = rect(&mut app, 0.0, 0.0, 120.0, 90.0);
        app.select(vec![r]);
        assert_eq!(app.fx_puzzle(), 5);
        assert_eq!(app.engine.undo_label(), Some("Puzzle"));
        assert!(app.doc().find_shape(r).is_some());
        assert!(app.engine.undo().is_ok());
        app.sc_fx.effects.puzzle_pieces = true;
        app.select(vec![r]);
        assert_eq!(app.fx_puzzle(), 12);
        assert!(app.doc().find_shape(r).is_none());
        let kids = children(&app, app.selection[0]);
        assert_eq!(kids.len(), 12);
        let area: f64 = kids.iter().map(|k| k.page_path().area().abs()).sum();
        assert!((area - 120.0 * 90.0).abs() < 1e-6, "{area}");
        // The same seed gives the same puzzle after undo.
        let first = kids[5].page_path();
        assert!(app.engine.undo().is_ok());
        assert!(app.doc().find_shape(r).is_some());
        app.select(vec![r]);
        app.fx_puzzle();
        assert_eq!(children(&app, app.selection[0])[5].page_path(), first);
    }

    #[test]
    fn circle_fill_puts_circles_inside() {
        let mut app = App::headless();
        let r = rect(&mut app, 0.0, 0.0, 100.0, 100.0);
        app.select(vec![r]);
        app.sc_fx.effects.circles.count = 25;
        assert_eq!(app.fx_circle_fill(), 25);
        assert_eq!(app.engine.undo_label(), Some("Circle Fill"));
        let kids = children(&app, app.selection[0]);
        assert_eq!(kids.len(), 25);
        for k in &kids {
            assert!(matches!(k.kind, ShapeKind::Ellipse { .. }));
            let b = k.bounds();
            assert!(b.x0 >= 1.0 - 1e-6 && b.y0 >= 1.0 - 1e-6);
            assert!(b.x1 <= 99.0 + 1e-6 && b.y1 <= 99.0 + 1e-6);
        }
    }

    #[test]
    fn a_shadow_goes_right_behind_its_object() {
        let mut app = App::headless();
        let low = rect(&mut app, 100.0, 0.0, 10.0, 10.0);
        let a = rect(&mut app, 0.0, 0.0, 10.0, 20.0);
        let top = rect(&mut app, 100.0, 50.0, 10.0, 10.0);
        app.select(vec![a]);
        app.sc_fx.effects.shadow = ShadowSpec {
            kind: ShadowKind::Wall,
            angle: 0.0,
            distance: 5.0,
            ..Default::default()
        };
        assert_eq!(app.fx_cast_shadow(), 1);
        assert_eq!(app.engine.undo_label(), Some("Cast Shadow"));
        let sh = app.selection[0];
        assert_eq!(layer_ids(&app), vec![low, sh, a, top]);
        let s = app.doc().shape_on_page(sh).expect("shadow");
        assert!((s.opacity - 0.5).abs() < 1e-12);
        assert!(s.stroke.is_none());
        assert_eq!(s.fill, Fill::Solid(Color::rgb8(64, 64, 64)));
        let b = s.bounds();
        assert!((b.x0 - 5.0).abs() < 1e-9 && (b.y1 - 20.0).abs() < 1e-9);
        // On the floor, in front: flattened below the bottom edge.
        app.sc_fx.effects.shadow = ShadowSpec {
            kind: ShadowKind::Floor,
            scale: 0.5,
            skew: 0.0,
            in_front: true,
            ..Default::default()
        };
        app.select(vec![a]);
        assert_eq!(app.fx_cast_shadow(), 1);
        let sh = app.selection[0];
        let b = app.doc().shape_on_page(sh).expect("shadow").bounds();
        assert!((b.y0 + 10.0).abs() < 1e-9 && b.y1.abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn random_copies_land_inside_the_container_and_repeat_with_the_seed() {
        let mut app = App::headless();
        let c = rect(&mut app, 0.0, 0.0, 200.0, 100.0);
        let src = rect(&mut app, 300.0, 0.0, 6.0, 4.0);
        app.select(vec![c]);
        assert!(app.fx_pick_container());
        app.select(vec![src, c]);
        app.sc_fx.effects.scatter.count = 12;
        assert_eq!(app.fx_random_copies(), 12);
        assert_eq!(app.engine.undo_label(), Some("Random Copies"));
        let boxes: Vec<Rect> = app
            .selection
            .iter()
            .map(|id| app.doc().shape_on_page(*id).expect("copy").bounds())
            .collect();
        assert_eq!(boxes.len(), 12);
        for b in &boxes {
            assert!(b.x0 >= -1e-6 && b.y0 >= -1e-6);
            assert!(b.x1 <= 200.0 + 1e-6 && b.y1 <= 100.0 + 1e-6);
        }
        assert!(app.engine.undo().is_ok());
        app.select(vec![src, c]);
        assert_eq!(app.fx_random_copies(), 12);
        let again: Vec<Rect> = app
            .selection
            .iter()
            .map(|id| app.doc().shape_on_page(*id).expect("copy").bounds())
            .collect();
        assert_eq!(boxes, again);
        // Without a container the page takes them.
        app.sc_fx.container = None;
        app.select(vec![src]);
        assert_eq!(app.fx_random_copies(), 12);
        let page = app.page_rect();
        for id in &app.selection {
            let b = app.doc().shape_on_page(*id).expect("copy").bounds();
            assert!(b.x0 >= page.x0 - 1e-6 && b.x1 <= page.x1 + 1e-6);
            assert!(b.y0 >= page.y0 - 1e-6 && b.y1 <= page.y1 + 1e-6);
        }
    }

    #[test]
    fn computing_changes_nothing_and_goes_stale_with_the_inputs() {
        let mut app = App::headless();
        let r = rect(&mut app, 0.0, 0.0, 50.0, 40.0);
        app.select(vec![r]);
        for op in [
            Op::Hatch,
            Op::Grid,
            Op::Lines,
            Op::Puzzle,
            Op::Circles,
            Op::Shadow,
            Op::Copies,
        ] {
            let rev = app.engine.revision();
            let c = app.fx_compute(op).expect("a result");
            assert_eq!(app.engine.revision(), rev, "{op:?}");
            assert!(c.count() > 0, "{op:?}");
            assert!(!c.outlines().is_empty(), "{op:?}");
            assert_eq!(c.inputs, app.fx_inputs(op), "{op:?}");
        }
        let c = app.fx_compute(Op::Puzzle).expect("a puzzle");
        app.sc_fx.effects.puzzle.cols += 1;
        assert_ne!(c.inputs, app.fx_inputs(Op::Puzzle));
        // Committing a result is one undo step.
        let before = app.engine.history_labels().0.len();
        assert_eq!(app.fx_commit(&c), 5);
        assert_eq!(app.engine.history_labels().0.len(), before + 1);
        // Another selection makes it stale too.
        app.select(vec![r]);
        let c = app.fx_compute(Op::Hatch).expect("hatching");
        app.select(vec![]);
        assert_ne!(c.inputs, app.fx_inputs(Op::Hatch));
    }

    #[test]
    fn presets_of_the_windows_round_trip() {
        let mut e = EffectSettings {
            method: Effect::Shadow,
            ..Default::default()
        };
        e.shadow.kind = ShadowKind::Floor;
        e.puzzle.seed = 77;
        let v = serde_json::to_value(&e).expect("json");
        assert_eq!(serde_json::from_value::<EffectSettings>(v).ok(), Some(e));
        let g = GridSettings {
            method: GridMethod::Lines,
            lines_count: 7,
            ..Default::default()
        };
        let v = serde_json::to_value(&g).expect("json");
        assert_eq!(serde_json::from_value::<GridSettings>(v).ok(), Some(g));
        let h = HatchSpec {
            style: HatchStyle::Grid,
            ..Default::default()
        };
        let v = serde_json::to_value(h).expect("json");
        assert_eq!(serde_json::from_value::<HatchSpec>(v).ok(), Some(h));
        // Missing fields take their defaults.
        let partial: EffectSettings =
            serde_json::from_str(r#"{"method":"Copies"}"#).expect("partial");
        assert_eq!(partial.method, Effect::Copies);
        assert_eq!(partial.puzzle, PuzzleSpec::default());
    }
}
