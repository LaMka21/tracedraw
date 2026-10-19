//! Sign & Cut, Cutting: settings and operations (plotter output, weeding, bridges, cut order, registration and crop marks).
//! The geometry is in `tracedraw_signcut::cut`; the panel section is
//! `ui/signcut_cut.rs`. See docs/behavior/sign-and-cut-cut.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut::Section;
use egui::Ui;
use tracedraw_core::geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _};
use tracedraw_core::{Color, Command, Fill, LayerId, Shape, ShapeId, ShapeKind, Stroke};
use tracedraw_signcut::cut::bridges::{self, BridgeCount, BridgeSpec};
use tracedraw_signcut::cut::hpgl::{self, Plot, Profile};
use tracedraw_signcut::cut::marks::{self, CropSpec, RegSpec};
use tracedraw_signcut::cut::order::{self, Route, RouteItem, RouteSpec};
use tracedraw_signcut::cut::weed::{self, WeedSpec};

/// Object data key marking the objects these tools make.
pub const TAG_KEY: &str = "signcut";
/// Its values: weeding cuts, registration marks, the cut contour, crop marks.
pub const TAG_WEEDING: &str = "weeding";
pub const TAG_REGISTRATION: &str = "registration marks";
pub const TAG_CONTOUR: &str = "cut contour";
pub const TAG_CROP: &str = "crop marks";

/// Printed only: never cut, never weeded around.
const PRINTED: [&str; 2] = [TAG_REGISTRATION, TAG_CROP];
/// Everything these tools made.
const MADE: [&str; 4] = [TAG_WEEDING, TAG_REGISTRATION, TAG_CONTOUR, TAG_CROP];

/// The name of the spot ink of the cut contour.
pub const CONTOUR_INK: &str = "Cut contour";

/// Colour of weeding cuts.
fn weed_color() -> Color {
    Color::rgb8(0, 150, 255)
}

/// What the registration marks tool puts on the cut contour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Contour {
    /// Copies of the objects' outlines.
    #[default]
    Outlines,
    /// The objects' bounding box.
    BoundingBox,
    /// No cut contour.
    None,
}

/// The last plotter job made, for the panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotSummary {
    pub outlines: usize,
    pub cut_length: f64,
    pub travel: f64,
    pub width: f64,
    pub height: f64,
    pub bytes: usize,
}

/// A planned route and the drawing it was planned for (the preview).
#[derive(Debug, Clone)]
pub struct RouteCache {
    pub revision: u64,
    pub targets: Vec<ShapeId>,
    pub spec: RouteSpec,
    pub moves: Vec<(Point, Point)>,
    pub before: f64,
    pub after: f64,
}

/// The Cutting section's settings and last results.
#[derive(Debug, Clone)]
pub struct Settings {
    pub weed: WeedSpec,
    /// Bridge length and minimum outline (the count is below).
    pub bridge: BridgeSpec,
    pub bridge_count: u32,
    pub bridge_every: f64,
    /// Bridges by distance (else a count per outline).
    pub bridge_by_distance: bool,
    pub route: RouteSpec,
    /// Show the planned travel on the drawing.
    pub show_route: bool,
    pub route_cache: Option<RouteCache>,
    /// Travel before and after the last Optimise.
    pub last_route: Option<(f64, f64)>,
    pub profiles: Vec<Profile>,
    pub profile: usize,
    /// The plotter output follows the route optimizer (else the drawing order).
    pub plot_optimize: bool,
    /// Where Send writes (a port, a shared printer, a device file).
    pub port: String,
    pub last_plot: Option<PlotSummary>,
    pub reg: RegSpec,
    /// Marks inside the page instead of around the objects.
    pub reg_page: bool,
    pub contour: Contour,
    pub crop: CropSpec,
    /// Crop marks around each object (else around all of them).
    pub crop_each: bool,
    /// Outline width of crop marks (mm).
    pub crop_width: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            weed: WeedSpec::default(),
            bridge: BridgeSpec::default(),
            bridge_count: 2,
            bridge_every: 100.0,
            bridge_by_distance: false,
            route: RouteSpec::default(),
            show_route: false,
            route_cache: None,
            last_route: None,
            profiles: vec![Profile::default()],
            profile: 0,
            plot_optimize: true,
            port: String::new(),
            last_plot: None,
            reg: RegSpec::default(),
            reg_page: false,
            contour: Contour::Outlines,
            crop: CropSpec::default(),
            crop_each: false,
            crop_width: 0.25,
        }
    }
}

impl Settings {
    /// The bridge settings with the count chosen.
    pub fn bridge_spec(&self) -> BridgeSpec {
        BridgeSpec {
            count: if self.bridge_by_distance {
                BridgeCount::Every(self.bridge_every)
            } else {
                BridgeCount::PerOutline(self.bridge_count)
            },
            ..self.bridge
        }
    }

    /// The plotter profile in use.
    pub fn current_profile(&self) -> Profile {
        self.profiles.get(self.profile).cloned().unwrap_or_default()
    }
}

/// The tag a shape carries, if any.
fn tag_of(s: &Shape) -> Option<&str> {
    s.data
        .iter()
        .find(|(k, _)| k == TAG_KEY)
        .map(|(_, v)| v.as_str())
}

/// The inverse of `t`, when it has one.
fn invert(t: Affine) -> Option<Affine> {
    let det = t.determinant();
    (det.is_finite() && det.abs() > 1e-12).then(|| t.inverse())
}

/// True when every subpath of `path` is closed.
fn all_closed(path: &BezPath) -> bool {
    let els = path.elements();
    let starts = els
        .iter()
        .filter(|e| matches!(e, PathEl::MoveTo(_)))
        .count();
    let closes = els
        .iter()
        .filter(|e| matches!(e, PathEl::ClosePath))
        .count();
    starts > 0 && starts == closes
}

fn finite_box(paths: &[BezPath]) -> Option<Rect> {
    paths
        .iter()
        .filter(|p| !p.elements().is_empty())
        .map(|p| p.bounding_box())
        .filter(|b| b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite())
        .reduce(|a, b| a.union(b))
}

/// A path object with the given outline and fill.
fn path_shape(id: ShapeId, path: BezPath, fill: Fill, stroke: Option<Stroke>) -> Shape {
    let closed = all_closed(&path);
    let mut s = Shape::new(id, ShapeKind::Path { path, closed });
    s.fill = fill;
    s.stroke = stroke;
    s
}

/// One plan of the route optimizer: a layer's objects in stacking order,
/// their places in the layer, and the route.
struct LayerPlan {
    layer: LayerId,
    ids: Vec<ShapeId>,
    slots: Vec<usize>,
    route: Route,
}

impl App {
    /// The objects the Cutting tools work on (see `signcut_targets`),
    /// without those these tools made with one of `skip`.
    pub fn sc_cut_targets(&self, skip: &[&str]) -> Vec<ShapeId> {
        let doc = self.doc();
        self.signcut_targets()
            .into_iter()
            .filter(|id| {
                let own = doc.find_shape(*id).and_then(tag_of);
                let top = doc
                    .top_level_of(*id)
                    .and_then(|t| doc.find_shape(t))
                    .and_then(tag_of);
                !own.into_iter().chain(top).any(|t| skip.contains(&t))
            })
            .collect()
    }

    fn sc_cut_outlines(&self, ids: &[ShapeId]) -> Vec<Vec<BezPath>> {
        let doc = self.doc();
        ids.iter()
            .map(|id| {
                doc.shape_on_page(*id)
                    .map(|s| tracedraw_signcut::outlines(&s))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// A new group of `children` named `name` and tagged `tag`.
    fn sc_cut_group(&mut self, name: &str, tag: &str, children: Vec<Shape>) -> Shape {
        let id = self.engine.new_shape_id();
        let mut g = Shape::new(id, ShapeKind::Group { children });
        g.fill = Fill::None;
        g.stroke = None;
        g.name = Some(name.to_string());
        g.data.push((TAG_KEY.to_string(), tag.to_string()));
        g
    }

    /// Add weeding cuts around and between the objects, grouped, in one
    /// step. Returns how many cuts were added.
    pub fn sc_cut_weed(&mut self) -> usize {
        let ids = self.sc_cut_targets(&MADE);
        let outlines = self.sc_cut_outlines(&ids);
        let paths = weed::weeding(&outlines, &self.sc_cut.weed);
        let Some(layer) = self.active_layer() else {
            return 0;
        };
        if paths.is_empty() {
            self.status = tr("signcut.cut_nothing");
            return 0;
        }
        let stroke = Stroke::hairline(weed_color());
        let children: Vec<Shape> = paths
            .into_iter()
            .map(|p| {
                let id = self.engine.new_shape_id();
                path_shape(id, p, Fill::None, Some(stroke.clone()))
            })
            .collect();
        let n = children.len();
        let group = self.sc_cut_group(&tr("signcut.cut_weed_group"), TAG_WEEDING, children);
        let gid = group.id;
        if let Err(e) = self.engine.run_batch(
            "Weeding Lines",
            &[Command::AddShape {
                layer,
                shape: group,
            }],
        ) {
            self.status = e.to_string();
            return 0;
        }
        self.select(vec![gid]);
        self.status = trf("signcut.cut_weed_done", &[("n", &n.to_string())]);
        n
    }

    /// The objects among `ids` whose outline can take bridges: curves and
    /// basic shapes without live effects, groups opened, unlocked.
    fn sc_cut_leaves(&self, ids: &[ShapeId]) -> Vec<ShapeId> {
        fn walk(s: &Shape, out: &mut Vec<ShapeId>) {
            if s.locked || tag_of(s).is_some_and(|t| PRINTED.contains(&t)) {
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

    /// Break the closed outlines of the objects into pieces with bridges
    /// between them, in one step. Returns how many bridges were made.
    pub fn sc_cut_bridges(&mut self) -> usize {
        let leaves = self.sc_cut_leaves(&self.sc_cut_targets(&PRINTED));
        let spec = self.sc_cut.bridge_spec();
        let doc = self.doc();
        let mut cmds = Vec::new();
        let mut total = 0;
        for id in leaves {
            let Some(s) = doc.shape_on_page(id) else {
                continue;
            };
            let Some(done) = bridges::bridge_path(&s.page_path(), &spec) else {
                continue;
            };
            let Some(inv) = invert(s.transform) else {
                continue;
            };
            total += done.bridges;
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path {
                    path: inv * done.path,
                    closed: false,
                },
            });
        }
        if cmds.is_empty() {
            self.status = tr("signcut.cut_no_bridges");
            return 0;
        }
        if let Err(e) = self.engine.run_batch("Bridges", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.status = trf("signcut.cut_bridges_done", &[("n", &total.to_string())]);
        total
    }

    /// Plan the cut order: each layer's unlocked top-level objects among
    /// the targets, layer by layer from the bottom one, each layer
    /// starting where the previous one ended.
    fn sc_cut_plans(&self) -> Vec<LayerPlan> {
        let targets = self.sc_cut_targets(&PRINTED);
        let doc = self.doc();
        let mut from = self.sc_cut.route.from;
        let mut plans = Vec::new();
        for layer in self.page_layers() {
            if layer.locked {
                continue;
            }
            let mut ids = Vec::new();
            let mut slots = Vec::new();
            for (i, s) in layer.shapes.iter().enumerate() {
                if !s.locked && targets.contains(&s.id) {
                    ids.push(s.id);
                    slots.push(i);
                }
            }
            if ids.is_empty() {
                continue;
            }
            let items: Vec<RouteItem> = ids
                .iter()
                .map(|id| {
                    let s = doc.shape_on_page(*id);
                    let editable = s.as_ref().is_some_and(|s| {
                        s.effects.is_empty()
                            && matches!(
                                s.kind,
                                ShapeKind::Rect { .. }
                                    | ShapeKind::Ellipse { .. }
                                    | ShapeKind::Polygon { .. }
                                    | ShapeKind::Path { .. }
                            )
                    });
                    RouteItem {
                        paths: s
                            .map(|s| tracedraw_signcut::outlines(&s))
                            .unwrap_or_default(),
                        editable,
                    }
                })
                .collect();
            let spec = RouteSpec {
                from,
                ..self.sc_cut.route
            };
            let route = order::plan(&items, &spec);
            from = route.end;
            plans.push(LayerPlan {
                layer: layer.id,
                ids,
                slots,
                route,
            });
        }
        plans
    }

    /// Reorder the objects (and their subpaths, start points and
    /// directions) for the shortest travel, in one step. Returns the
    /// travel before and after (mm).
    pub fn sc_cut_order(&mut self) -> Option<(f64, f64)> {
        let plans = self.sc_cut_plans();
        if plans.is_empty() {
            self.status = tr("signcut.cut_nothing");
            return None;
        }
        let doc = self.doc();
        let mut cmds = Vec::new();
        let (mut before, mut after) = (0.0, 0.0);
        for plan in &plans {
            before += plan.route.travel_before;
            after += plan.route.travel_after;
            for (k, item) in plan.route.order.iter().enumerate() {
                if let (Some(id), Some(slot)) = (plan.ids.get(*item), plan.slots.get(k)) {
                    cmds.push(Command::Reorder {
                        shape: *id,
                        layer: plan.layer,
                        index: *slot,
                    });
                }
            }
            for (i, changed) in plan.route.changed.iter().enumerate() {
                if !changed {
                    continue;
                }
                let (Some(id), Some(cuts)) = (plan.ids.get(i), plan.route.cuts.get(i)) else {
                    continue;
                };
                let Some(inv) = doc.shape_on_page(*id).and_then(|s| invert(s.transform)) else {
                    continue;
                };
                let mut path = BezPath::new();
                for c in cuts {
                    path.extend(c.elements().iter().copied());
                }
                let closed = all_closed(&path);
                cmds.push(Command::SetShapeKind {
                    shape: *id,
                    kind: ShapeKind::Path {
                        path: inv * path,
                        closed,
                    },
                });
            }
        }
        if let Err(e) = self.engine.run_batch("Cut Order", &cmds) {
            self.status = e.to_string();
            return None;
        }
        let u = self.units;
        self.status = trf(
            "signcut.cut_order_done",
            &[
                ("before", &crate::ui::signcut_docker::fmt_len(u, before)),
                ("after", &crate::ui::signcut_docker::fmt_len(u, after)),
            ],
        );
        self.sc_cut.last_route = Some((before, after));
        self.sc_cut.route_cache = None;
        Some((before, after))
    }

    /// The planned travel moves and the travel before and after, kept
    /// while the drawing, the objects and the settings stay the same.
    pub fn sc_cut_route_preview(&mut self) -> &RouteCache {
        let targets = self.sc_cut_targets(&PRINTED);
        let revision = self.engine.revision();
        let spec = self.sc_cut.route;
        let fresh = self
            .sc_cut
            .route_cache
            .as_ref()
            .is_some_and(|c| c.revision == revision && c.targets == targets && c.spec == spec);
        if !fresh {
            let plans = self.sc_cut_plans();
            let cache = RouteCache {
                revision,
                targets,
                spec,
                moves: plans
                    .iter()
                    .flat_map(|p| p.route.moves.iter().copied())
                    .collect(),
                before: plans.iter().map(|p| p.route.travel_before).sum(),
                after: plans.iter().map(|p| p.route.travel_after).sum(),
            };
            self.sc_cut.route_cache = Some(cache);
        }
        self.sc_cut.route_cache.get_or_insert_with(|| RouteCache {
            revision,
            targets: Vec::new(),
            spec,
            moves: Vec::new(),
            before: 0.0,
            after: 0.0,
        })
    }

    /// The plotter job for the objects (registration and crop marks left
    /// out), in route order when `plot_optimize` is on.
    pub fn sc_cut_plot(&self) -> Option<Plot> {
        let ids = self.sc_cut_targets(&PRINTED);
        let outlines = self.sc_cut_outlines(&ids);
        let cuts: Vec<BezPath> = if self.sc_cut.plot_optimize {
            let items: Vec<RouteItem> = outlines
                .into_iter()
                .map(|paths| RouteItem {
                    paths,
                    editable: true,
                })
                .collect();
            let route = order::plan(&items, &self.sc_cut.route);
            route
                .order
                .iter()
                .filter_map(|i| route.cuts.get(*i))
                .flatten()
                .cloned()
                .collect()
        } else {
            outlines.into_iter().flatten().collect()
        };
        let plot = hpgl::plot(&cuts, &self.sc_cut.current_profile());
        (plot.outlines > 0).then_some(plot)
    }

    /// Make the plotter job and keep its summary for the panel.
    fn sc_cut_job(&mut self) -> Option<Plot> {
        let plot = self.sc_cut_plot();
        self.sc_cut.last_plot = plot.as_ref().map(|p| PlotSummary {
            outlines: p.outlines,
            cut_length: p.cut_length,
            travel: p.travel,
            width: p.width,
            height: p.height,
            bytes: p.text.len(),
        });
        if plot.is_none() {
            self.status = tr("signcut.cut_nothing");
        }
        plot
    }

    /// Save the plotter job as a `.plt` file chosen in a dialog.
    pub fn sc_cut_save_plt(&mut self) {
        let Some(plot) = self.sc_cut_job() else {
            return;
        };
        crate::files::Dialog::new()
            .add_filter("HPGL", &["plt", "hpgl"])
            .set_file_name("cut.plt")
            .save_file(self, move |app, path| {
                match crate::files::write(&path, plot.text.as_bytes()) {
                    Ok(()) => {
                        app.status = trf(
                            "signcut.cut_saved",
                            &[("path", &path.display().to_string())],
                        )
                    }
                    Err(e) => app.status = e.to_string(),
                }
            });
    }

    /// Write the plotter job to the port or share typed in the panel.
    /// Returns the bytes written.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sc_cut_send(&mut self) -> Result<usize, String> {
        let port = self.sc_cut.port.trim().to_string();
        if port.is_empty() {
            let e = tr("signcut.cut_no_port");
            self.status = e.clone();
            return Err(e);
        }
        let Some(plot) = self.sc_cut_job() else {
            return Err(self.status.clone());
        };
        match send_bytes(std::path::Path::new(&port), plot.text.as_bytes()) {
            Ok(()) => {
                self.status = trf(
                    "signcut.cut_sent",
                    &[("n", &plot.text.len().to_string()), ("port", &port)],
                );
                Ok(plot.text.len())
            }
            Err(e) => {
                self.status = e.to_string();
                Err(e.to_string())
            }
        }
    }

    /// The spot ink of the cut contour, and the command adding it to the
    /// drawing when it is not there yet.
    fn sc_cut_contour_ink(&self) -> (Color, Option<Command>) {
        let doc = self.doc();
        if let Some(ink) = doc.inks.iter().find(|i| i.name == CONTOUR_INK) {
            return (ink.color(1.0), None);
        }
        let id = doc.inks.iter().map(|i| i.id).max().map_or(1, |m| m + 1);
        let ink = tracedraw_core::document::Ink {
            id,
            name: CONTOUR_INK.to_string(),
            cmyk: [0.0, 1.0, 0.0, 0.0],
        };
        let color = ink.color(1.0);
        let mut inks = doc.inks.clone();
        inks.push(ink);
        (color, Some(Command::SetInks { inks }))
    }

    /// Add registration marks (and a cut contour) around the objects or
    /// inside the page, in one step. Returns how many marks were added.
    pub fn sc_cut_registration(&mut self) -> usize {
        let ids = self.sc_cut_targets(&MADE);
        let outlines = self.sc_cut_outlines(&ids);
        let art = finite_box(&outlines.iter().flatten().cloned().collect::<Vec<_>>());
        let area = if self.sc_cut.reg_page {
            Some(self.page_rect())
        } else {
            art
        };
        let Some(area) = area else {
            self.status = tr("signcut.cut_nothing");
            return 0;
        };
        let Some(layer) = self.active_layer() else {
            return 0;
        };
        let spec = RegSpec {
            inside: self.sc_cut.reg_page,
            ..self.sc_cut.reg
        };
        let found = marks::registration_marks(area, &spec);
        if found.is_empty() {
            self.status = tr("signcut.cut_nothing");
            return 0;
        }
        let n = found.len();
        let mut cmds = Vec::new();
        let children: Vec<Shape> = found
            .into_iter()
            .map(|p| {
                let id = self.engine.new_shape_id();
                path_shape(id, p, Fill::Solid(Color::BLACK), None)
            })
            .collect();
        let reg = self.sc_cut_group(&tr("signcut.cut_reg_group"), TAG_REGISTRATION, children);
        let mut select = vec![reg.id];
        cmds.push(Command::AddShape { layer, shape: reg });
        let contour: Vec<BezPath> = match (self.sc_cut.contour, art) {
            (Contour::Outlines, _) => outlines
                .into_iter()
                .map(|paths| {
                    let mut p = BezPath::new();
                    for q in paths {
                        p.extend(q.elements().iter().copied());
                    }
                    p
                })
                .filter(|p| !p.elements().is_empty())
                .collect(),
            (Contour::BoundingBox, Some(b)) => vec![b.to_path(0.01)],
            _ => Vec::new(),
        };
        if !contour.is_empty() {
            let (color, ink) = self.sc_cut_contour_ink();
            if let Some(c) = ink {
                cmds.insert(0, c);
            }
            let stroke = Stroke::hairline(color);
            let children: Vec<Shape> = contour
                .into_iter()
                .map(|p| {
                    let id = self.engine.new_shape_id();
                    path_shape(id, p, Fill::None, Some(stroke.clone()))
                })
                .collect();
            let g = self.sc_cut_group(CONTOUR_INK, TAG_CONTOUR, children);
            select.push(g.id);
            cmds.push(Command::AddShape { layer, shape: g });
        }
        if let Err(e) = self.engine.run_batch("Registration Marks", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.select(select);
        self.status = trf("signcut.cut_reg_done", &[("n", &n.to_string())]);
        n
    }

    /// Add crop marks around each object or around all of them, grouped,
    /// in one step. Returns how many corners were marked.
    pub fn sc_cut_crop_marks(&mut self) -> usize {
        let ids = self.sc_cut_targets(&MADE);
        let outlines = self.sc_cut_outlines(&ids);
        let boxes: Vec<Rect> = if self.sc_cut.crop_each {
            outlines.iter().filter_map(|o| finite_box(o)).collect()
        } else {
            finite_box(&outlines.into_iter().flatten().collect::<Vec<_>>())
                .into_iter()
                .collect()
        };
        let found: Vec<BezPath> = boxes
            .iter()
            .flat_map(|b| marks::crop_marks(*b, &self.sc_cut.crop))
            .collect();
        let Some(layer) = self.active_layer() else {
            return 0;
        };
        if found.is_empty() {
            self.status = tr("signcut.cut_nothing");
            return 0;
        }
        let width = if self.sc_cut.crop_width.is_finite() {
            self.sc_cut.crop_width.max(0.0)
        } else {
            Stroke::HAIRLINE
        };
        let stroke = Stroke::new(Color::Registration, width);
        let n = found.len();
        let children: Vec<Shape> = found
            .into_iter()
            .map(|p| {
                let id = self.engine.new_shape_id();
                path_shape(id, p, Fill::None, Some(stroke.clone()))
            })
            .collect();
        let g = self.sc_cut_group(&tr("signcut.cut_crop_group"), TAG_CROP, children);
        let gid = g.id;
        if let Err(e) = self
            .engine
            .run_batch("Crop Marks", &[Command::AddShape { layer, shape: g }])
        {
            self.status = e.to_string();
            return 0;
        }
        self.select(vec![gid]);
        self.status = trf("signcut.cut_crop_done", &[("n", &n.to_string())]);
        n
    }
}

/// Write bytes to a port, a shared printer or a device file as it is,
/// or to a file (made or replaced).
#[cfg(not(target_arch = "wasm32"))]
fn send_bytes(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return crate::files::write(path, bytes);
    }
    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(mut f) => {
            f.write_all(bytes)?;
            f.flush()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => crate::files::write(path, bytes),
        Err(e) => Err(e),
    }
}

/// Tools > Sign & Cut > Cutting.
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::ui::menus::{item, sep};
    let doc = app.has_document() && !app.show_welcome;
    if item(ui, "menu.tools.signcut_cut_panel", "", true) {
        app.open_signcut(Section::Cut);
    }
    sep(ui);
    if item(ui, "menu.tools.signcut_cut_weed", "", doc) {
        app.open_signcut(Section::Cut);
        app.sc_cut_weed();
    }
    if item(ui, "menu.tools.signcut_cut_bridges", "", doc) {
        app.open_signcut(Section::Cut);
        app.sc_cut_bridges();
    }
    if item(ui, "menu.tools.signcut_cut_order", "", doc) {
        app.open_signcut(Section::Cut);
        app.sc_cut_order();
    }
    if item(ui, "menu.tools.signcut_cut_plotter", "", doc) {
        app.open_signcut(Section::Cut);
    }
    sep(ui);
    if item(ui, "menu.tools.signcut_cut_registration", "", doc) {
        app.open_signcut(Section::Cut);
        app.sc_cut_registration();
    }
    if item(ui, "menu.tools.signcut_cut_crop", "", doc) {
        app.open_signcut(Section::Cut);
        app.sc_cut_crop_marks();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_signcut::cut::order::{Direction, StartAt};
    use tracedraw_signcut::measure;

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer to draw on")
    }

    fn group_children(app: &App, id: ShapeId) -> Vec<Shape> {
        match app.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(ShapeKind::Group { children }) => children,
            _ => Vec::new(),
        }
    }

    fn layer_order(app: &App) -> Vec<ShapeId> {
        app.page_layers()
            .iter()
            .flat_map(|l| l.shapes.iter().map(|s| s.id))
            .collect()
    }

    #[test]
    fn weeding_adds_one_group_of_hairlines_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 20.0, 20.0);
        let b = rect(&mut app, 50.0, 10.0, 20.0, 20.0);
        app.select(vec![a, b]);
        let n = app.sc_cut_weed();
        // Four lines each, one shared between them, and the frame.
        assert_eq!(n, 8);
        assert_eq!(app.engine.undo_label(), Some("Weeding Lines"));
        let [g] = app.selection[..] else {
            panic!("the group is selected")
        };
        let kids = group_children(&app, g);
        assert_eq!(kids.len(), 8);
        for k in &kids {
            let s = k.stroke.as_ref().expect("outline");
            assert_eq!(s.width, Stroke::HAIRLINE);
            assert!(matches!(k.fill, Fill::None));
        }
        assert_eq!(app.doc().find_shape(g).and_then(tag_of), Some(TAG_WEEDING));
        // Weeding again with nothing selected leaves the old cuts out.
        app.select(vec![]);
        assert_eq!(app.sc_cut_weed(), 8);
        assert!(app.engine.undo().is_ok());
        assert!(app.engine.undo().is_ok());
        assert_eq!(layer_order(&app), vec![a, b]);
    }

    #[test]
    fn bridges_turn_outlines_into_open_pieces_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 100.0, 100.0);
        app.select(vec![a]);
        assert_eq!(app.sc_cut_bridges(), 2);
        let s = app.doc().shape_on_page(a).expect("still there");
        assert!(matches!(s.kind, ShapeKind::Path { closed: false, .. }));
        assert!((measure::length(&s.page_path()) - 396.0).abs() < 1e-6);
        assert_eq!(app.engine.undo_label(), Some("Bridges"));
        // By distance: one every 100 mm on 396 mm of open pieces: nothing
        // closed is left, so nothing more happens.
        app.sc_cut.bridge_by_distance = true;
        assert_eq!(app.sc_cut_bridges(), 0);
        assert!(app.engine.undo().is_ok());
        assert_eq!(app.sc_cut_bridges(), 4);
    }

    #[test]
    fn cut_order_reorders_the_layer_and_reports_travel() {
        let mut app = App::headless();
        let far = rect(&mut app, 300.0, 0.0, 5.0, 5.0);
        let near = rect(&mut app, 0.0, 0.0, 5.0, 5.0);
        let mid = rect(&mut app, 100.0, 0.0, 5.0, 5.0);
        app.select(vec![far, near, mid]);
        let (before, after) = app.sc_cut_order().expect("planned");
        assert!(after < before, "{before} {after}");
        assert_eq!(layer_order(&app), vec![near, mid, far]);
        assert_eq!(app.engine.undo_label(), Some("Cut Order"));
        assert_eq!(app.sc_cut.last_route, Some((before, after)));
        // Rectangles already starting at their nearest node stay rectangles.
        assert!(matches!(
            app.doc().find_shape(near).map(|s| s.kind.clone()),
            Some(ShapeKind::Rect { .. })
        ));
        // Clockwise turns them into curves, still where they were.
        app.sc_cut.route.direction = Direction::Clockwise;
        app.sc_cut.route.start = StartAt::Keep;
        app.sc_cut_order();
        let s = app.doc().shape_on_page(mid).expect("mid");
        assert!(matches!(s.kind, ShapeKind::Path { closed: true, .. }));
        let b = s.bounds();
        assert!((b.x0 - 100.0).abs() < 1e-9 && (b.x1 - 105.0).abs() < 1e-9);
        assert!(app.engine.undo().is_ok());
        assert!(app.engine.undo().is_ok());
        assert_eq!(layer_order(&app), vec![far, near, mid]);
    }

    #[test]
    fn route_preview_is_kept_until_something_changes() {
        let mut app = App::headless();
        let a = rect(&mut app, 50.0, 0.0, 5.0, 5.0);
        rect(&mut app, 0.0, 0.0, 5.0, 5.0);
        let moves = app.sc_cut_route_preview().moves.clone();
        assert_eq!(moves.len(), 1, "{moves:?}");
        let rev = app.sc_cut_route_preview().revision;
        app.run(Command::TransformShapes {
            shapes: vec![a],
            transform: Affine::translate((10.0, 0.0)),
        });
        assert_ne!(app.sc_cut_route_preview().revision, rev);
        assert!((app.sc_cut_route_preview().after - 60.0).abs() < 1e-9);
    }

    #[test]
    fn plotter_output_follows_the_route_and_leaves_marks_out() {
        let mut app = App::headless();
        rect(&mut app, 100.0, 0.0, 10.0, 10.0);
        rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        app.sc_cut.profiles[0].knife_offset = 0.0;
        app.sc_cut.profiles[0].overcut = 0.0;
        let plot = app.sc_cut_plot().expect("a job");
        assert_eq!(plot.outlines, 2);
        // The near square first, from the job's bottom-left corner.
        assert!(
            plot.text
                .starts_with("IN;PU0,0;PD400,0,400,400,0,400,0,0;PU4000,0;"),
            "{}",
            plot.text
        );
        app.sc_cut.plot_optimize = false;
        let drawn = app.sc_cut_plot().expect("a job");
        assert!(drawn.text.starts_with("IN;PU4000,0;"), "{}", drawn.text);
        // Registration marks are printed, not cut; the contour is cut.
        app.select(vec![]);
        app.sc_cut.contour = Contour::BoundingBox;
        assert_eq!(app.sc_cut_registration(), 4);
        app.select(vec![]);
        let with = app.sc_cut_plot().expect("a job");
        assert_eq!(with.outlines, 3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn send_writes_the_job_to_the_port_path() {
        let mut app = App::headless();
        rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        assert!(app.sc_cut_send().is_err(), "no port typed");
        let path = std::env::temp_dir().join(format!("tracedraw-cut-{}.plt", std::process::id()));
        let _ = std::fs::write(&path, vec![b'x'; 5000]);
        app.sc_cut.port = path.display().to_string();
        let n = app.sc_cut_send().expect("written");
        let back = std::fs::read(&path).expect("read back");
        assert_eq!(back.len(), n);
        assert!(back.starts_with(b"IN;"));
        assert_eq!(app.sc_cut.last_plot.map(|p| p.bytes), Some(n));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn registration_marks_and_cut_contour_in_one_step() {
        let mut app = App::headless();
        let a = rect(&mut app, 50.0, 50.0, 100.0, 40.0);
        let ink_count = app.doc().inks.len();
        app.select(vec![a]);
        assert_eq!(app.sc_cut_registration(), 4);
        assert_eq!(app.engine.undo_label(), Some("Registration Marks"));
        assert_eq!(app.selection.len(), 2);
        let reg = app.selection[0];
        let contour = app.selection[1];
        let marks = group_children(&app, reg);
        assert_eq!(marks.len(), 4);
        assert!(marks.iter().all(|m| m.fill == Fill::Solid(Color::BLACK)));
        let b = marks
            .iter()
            .map(|m| m.bounds())
            .reduce(|x, y| x.union(y))
            .expect("marks");
        assert!(
            (b.x0 - 35.0).abs() < 1e-9 && (b.y1 - 105.0).abs() < 1e-9,
            "{b:?}"
        );
        let c = group_children(&app, contour);
        assert_eq!(c.len(), 1);
        assert_eq!(
            app.doc().find_shape(contour).and_then(|s| s.name.clone()),
            Some(CONTOUR_INK.to_string())
        );
        let doc = app.doc();
        let ink = doc
            .inks
            .iter()
            .find(|i| i.name == CONTOUR_INK)
            .expect("the ink");
        assert_eq!(doc.inks.len(), ink_count + 1);
        assert_eq!(c[0].stroke.as_ref().map(|s| s.color), Some(ink.color(1.0)));
        // Again: the ink is not added twice; on the page the marks go inside.
        app.sc_cut.reg_page = true;
        app.sc_cut.contour = Contour::None;
        app.select(vec![a]);
        assert_eq!(app.sc_cut_registration(), 4);
        assert_eq!(app.doc().inks.len(), ink_count + 1);
        assert_eq!(app.selection.len(), 1);
        let page = app.page_rect();
        for m in group_children(&app, app.selection[0]) {
            let mb = m.bounds();
            assert!(mb.x0 >= page.x0 + 10.0 - 1e-9 && mb.x1 <= page.x1 - 10.0 + 1e-9);
        }
    }

    #[test]
    fn crop_marks_around_each_object_or_all() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 50.0, 0.0, 10.0, 10.0);
        app.select(vec![a, b]);
        assert_eq!(app.sc_cut_crop_marks(), 4);
        assert_eq!(app.engine.undo_label(), Some("Crop Marks"));
        let kids = group_children(&app, app.selection[0]);
        assert_eq!(kids.len(), 4);
        let s = kids[0].stroke.as_ref().expect("outline");
        assert_eq!(s.color, Color::Registration);
        assert!((s.width - 0.25).abs() < 1e-12);
        app.sc_cut.crop_each = true;
        app.select(vec![a, b]);
        assert_eq!(app.sc_cut_crop_marks(), 8);
    }

    #[test]
    fn tools_on_an_empty_page_change_nothing() {
        let mut app = App::headless();
        assert_eq!(app.sc_cut_weed(), 0);
        assert_eq!(app.sc_cut_bridges(), 0);
        assert!(app.sc_cut_order().is_none());
        assert!(app.sc_cut_plot().is_none());
        assert_eq!(app.sc_cut_registration(), 0);
        assert_eq!(app.sc_cut_crop_marks(), 0);
        assert!(app.engine.undo_label().is_none());
        // On the page the marks need no objects.
        app.sc_cut.reg_page = true;
        assert_eq!(app.sc_cut_registration(), 4);
    }
}
