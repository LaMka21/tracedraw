//! Sign & Cut, Sign Making: settings and operations (LED modules, neon tubes, sign boxes, letter returns, tiling and carriers).
//! The geometry is in `tracedraw_signcut::sign`; the panel section is
//! `ui/signcut_sign.rs`. See docs/behavior/sign-and-cut-sign.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use egui::Ui;
use tracedraw_core::geometry::{Affine, BezPath, Point, Rect, Shape as _};
use tracedraw_core::{
    Color, Command, Fill, LayerId, LineCap, LineJoin, Shape, ShapeId, ShapeKind, Stroke, TextSpan,
};
use tracedraw_signcut::sign::carrier::{self, CarrierSpec};
use tracedraw_signcut::sign::led::{self, LedReport, LedSpec, Module};
use tracedraw_signcut::sign::neon::{self, NeonSpec};
use tracedraw_signcut::sign::returns::{self, MouldSpec, ReturnSpec, Strip};
use tracedraw_signcut::sign::sign_box::{self, BoxSpec};
use tracedraw_signcut::sign::tiles::{self, TileSpec};

/// The tools of the section, each under its own header.
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

    /// The i18n key of its header and menu item.
    pub fn key(self) -> &'static str {
        match self {
            Tool::Led => "signcut.sign_led",
            Tool::Neon => "signcut.sign_neon",
            Tool::SignBox => "signcut.sign_box",
            Tool::Returns => "signcut.sign_returns",
            Tool::Mould => "signcut.sign_mould",
            Tool::Tiles => "signcut.sign_tiles",
            Tool::Carrier => "signcut.sign_carrier",
        }
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileOrigin {
    /// The bottom-left corner of the selection.
    #[default]
    Selection,
    /// The page's origin (its bottom-left corner).
    Page,
}

/// What Neon Tubes made last.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NeonReport {
    pub tubes: usize,
    pub length: f64,
    pub supports: usize,
}

/// The Sign Making section's settings and last results.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The tool the menu last asked for (its header opens once).
    pub open: Option<Tool>,
    pub catalogue: Vec<Module>,
    /// The catalogue entry in use.
    pub module: usize,
    pub led: LedSpec,
    pub led_report: Option<LedReport>,
    pub neon: NeonSpec,
    /// Draw tubes as their outline (both sides) instead of a thick line.
    pub neon_outline: bool,
    pub neon_report: Option<NeonReport>,
    pub sign_box: BoxSpec,
    /// The face when nothing is selected (mm).
    pub box_width: f64,
    pub box_height: f64,
    pub returns: ReturnSpec,
    /// Space between the strips and under the selection (mm).
    pub strip_gap: f64,
    pub labels: bool,
    /// Strips made and their total length (mm).
    pub returns_report: Option<(usize, f64)>,
    pub mould: MouldSpec,
    pub mould_report: Option<(usize, f64)>,
    pub tiles: TileSpec,
    pub tile_origin: TileOrigin,
    pub tile_labels: bool,
    pub tile_frames: bool,
    /// Columns, rows and pieces of the last division.
    pub tiles_report: Option<(usize, usize, usize)>,
    pub carrier: CarrierSpec,
    /// Pieces of the last carrier.
    pub carrier_report: Option<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            open: None,
            catalogue: led::catalogue(),
            module: 0,
            led: LedSpec::default(),
            led_report: None,
            neon: NeonSpec::default(),
            neon_outline: false,
            neon_report: None,
            sign_box: BoxSpec::default(),
            box_width: 1000.0,
            box_height: 500.0,
            returns: ReturnSpec::default(),
            strip_gap: 10.0,
            labels: true,
            returns_report: None,
            mould: MouldSpec::default(),
            mould_report: None,
            tiles: TileSpec::default(),
            tile_origin: TileOrigin::Selection,
            tile_labels: true,
            tile_frames: false,
            tiles_report: None,
            carrier: CarrierSpec::default(),
            carrier_report: None,
        }
    }
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

impl App {
    /// Open the panel at the Sign Making section, `tool`'s header open.
    pub fn open_sign_tool(&mut self, tool: Tool) {
        self.sc_sign.open = Some(tool);
        self.open_signcut(crate::signcut::Section::Sign);
    }

    /// The selected objects the tools work on, in stacking order, locked
    /// ones left out.
    fn sign_targets(&self) -> Vec<ShapeId> {
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
    fn sign_outlines(&self, ids: &[ShapeId]) -> Vec<Vec<BezPath>> {
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
    fn sign_bounds(outlines: &[Vec<BezPath>]) -> Option<Rect> {
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

    /// The module in use.
    pub fn sign_module(&self) -> Option<&Module> {
        self.sc_sign.catalogue.get(self.sc_sign.module)
    }

    /// Lay LED modules out in the selected shapes as one group. Returns
    /// how many modules were placed.
    pub fn sign_led_layout(&mut self) -> usize {
        let Some((ids, layer)) = self.sign_need_selection() else {
            return 0;
        };
        let Some(module) = self.sign_module().cloned() else {
            self.status = tr("signcut.sign_no_module");
            return 0;
        };
        let spec = self.sc_sign.led;
        let mut placements = Vec::new();
        for paths in self.sign_outlines(&ids) {
            placements.extend(led::layout(&paths, &module, &spec));
        }
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

    /// Neon tubes along the selected curves, as one group. Returns the
    /// number of tubes.
    pub fn sign_neon(&mut self) -> usize {
        let Some((ids, layer)) = self.sign_need_selection() else {
            return 0;
        };
        let spec = self.sc_sign.neon;
        let paths: Vec<BezPath> = self.sign_outlines(&ids).into_iter().flatten().collect();
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
        for tube in &result.tubes {
            let shape = if self.sc_sign.neon_outline {
                let outline = neon::tube_outline(tube, d);
                self.sign_path(outline, Fill::None, hairline(NEON_COLOR))
            } else {
                let stroke = Stroke {
                    cap: LineCap::Round,
                    join: LineJoin::Round,
                    ..Stroke::new(rgb(NEON_COLOR), d)
                };
                self.sign_path(tube.path.clone(), Fill::None, Some(stroke))
            };
            children.push(shape);
        }
        let hole = spec.hole_diameter;
        if hole.is_finite() && hole > 0.0 {
            for p in &result.supports {
                let c = tracedraw_signcut::sign::circle(*p, hole);
                let s = self.sign_path(c, Fill::None, hairline([0, 0, 0]));
                children.push(s);
            }
        }
        let group = self.sign_group("signcut.sign_neon_group", children);
        self.sign_add("Neon Tubes", layer, vec![group]);
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

    /// The face of the sign box: the selection's bounds, or the panel's
    /// width and height centred on the page.
    pub fn sign_box_face(&self) -> Rect {
        let ids = self.sign_targets();
        if let Some(b) = Self::sign_bounds(&self.sign_outlines(&ids)) {
            return b;
        }
        let c = self.page_rect().center();
        let (w, h) = (self.sc_sign.box_width, self.sc_sign.box_height);
        Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0)
    }

    /// The flat pattern of a sign box as one group.
    pub fn sign_make_box(&mut self) -> bool {
        let ids = self.sign_targets();
        let Some(layer) = self.sign_layer(&ids) else {
            self.status = tr("signcut.sign_no_layer");
            return false;
        };
        let face = self.sign_box_face();
        let Some(pattern) = sign_box::sign_box(face, &self.sc_sign.sign_box) else {
            self.status = tr("signcut.sign_box_bad");
            return false;
        };
        let mut children =
            vec![self.sign_path(pattern.outline.clone(), Fill::None, hairline(CUT_COLOR))];
        for (a, b) in &pattern.folds {
            let line = tracedraw_core::geometry::polyline_path(&[*a, *b], false);
            let s = self.sign_path(line, Fill::None, dashed(FOLD_COLOR));
            children.push(s);
        }
        if pattern.hole_diameter > 0.0 {
            for p in &pattern.holes {
                let c = tracedraw_signcut::sign::circle(*p, pattern.hole_diameter);
                let s = self.sign_path(c, Fill::None, hairline(CUT_COLOR));
                children.push(s);
            }
        }
        let group = self.sign_group("signcut.sign_box_group", children);
        let added = self.sign_add("Sign Box", layer, vec![group]);
        if added.is_empty() {
            return false;
        }
        let u = self.units;
        let bb = pattern.outline.bounding_box();
        self.status = trf(
            "signcut.sign_box_done",
            &[
                ("w", &crate::ui::signcut_docker::fmt_len(u, bb.width())),
                ("h", &crate::ui::signcut_docker::fmt_len(u, bb.height())),
            ],
        );
        true
    }

    /// Strips laid out under the selection, with their numbers.
    fn sign_strip_shapes(&mut self, strips: &[Strip], below: Rect, notch: f64) -> Vec<Shape> {
        let at = returns::stack(strips, below.x0, below.y0, self.sc_sign.strip_gap);
        let mut out = Vec::new();
        for (s, p) in strips.iter().zip(at) {
            let path = returns::strip_path(s, notch, p);
            let shape = self.sign_path(path, Fill::None, hairline(STRIP_COLOR));
            out.push(shape);
            if self.sc_sign.labels {
                let in_object = strips.iter().filter(|o| o.object == s.object).count();
                let size = (s.height * 0.4).clamp(1.0, 20.0);
                let label = returns::strip_label(s, in_object);
                let pos = Point::new(p.x + size * 0.5, p.y + (s.height - size) / 2.0);
                let t = self.sign_label(label, pos, size);
                out.push(t);
            }
        }
        out
    }

    /// Letter return strips of the selected letters, under them, as one
    /// group. Returns the number of strips.
    pub fn sign_returns(&mut self) -> usize {
        let Some((ids, layer)) = self.sign_need_selection() else {
            return 0;
        };
        let outlines = self.sign_outlines(&ids);
        let Some(bounds) = Self::sign_bounds(&outlines) else {
            return 0;
        };
        let spec = self.sc_sign.returns;
        let strips = returns::returns(&outlines, &spec);
        let total: f64 = strips.iter().map(|s| s.length).sum();
        self.sc_sign.returns_report = Some((strips.len(), total));
        if strips.is_empty() {
            self.status = tr("signcut.sign_returns_none");
            return 0;
        }
        let children = self.sign_strip_shapes(&strips, bounds, spec.notch_size);
        let group = self.sign_group("signcut.sign_returns_group", children);
        self.sign_add("Letter Returns", layer, vec![group]);
        let u = self.units;
        self.status = trf(
            "signcut.sign_strips_done",
            &[
                ("n", &strips.len().to_string()),
                ("len", &crate::ui::signcut_docker::fmt_len(u, total)),
            ],
        );
        strips.len()
    }

    /// The liquid acrylic mould of the selected letters: the bases over
    /// the letters and the wall strips under them, as one group. Returns
    /// the number of strips.
    pub fn sign_mould(&mut self) -> usize {
        let Some((ids, layer)) = self.sign_need_selection() else {
            return 0;
        };
        let outlines = self.sign_outlines(&ids);
        let Some(bounds) = Self::sign_bounds(&outlines) else {
            return 0;
        };
        let (bases, strips) = returns::mould(&outlines, &self.sc_sign.mould);
        let total: f64 = strips.iter().map(|s| s.length).sum();
        self.sc_sign.mould_report = Some((strips.len(), total));
        if strips.is_empty() {
            self.status = tr("signcut.sign_returns_none");
            return 0;
        }
        let mut children = Vec::new();
        for b in bases.into_iter().filter(|b| !b.elements().is_empty()) {
            let s = self.sign_path(b, Fill::None, hairline(CUT_COLOR));
            children.push(s);
        }
        // The strips go under the bases, which reach out by the wall.
        let wall = self.sc_sign.mould.wall;
        let wall = if wall.is_finite() { wall.max(0.0) } else { 0.0 };
        let below = bounds.inflate(wall, wall);
        children.extend(self.sign_strip_shapes(&strips, below, 0.0));
        let group = self.sign_group("signcut.sign_mould_group", children);
        self.sign_add("Acrylic Mould", layer, vec![group]);
        let u = self.units;
        self.status = trf(
            "signcut.sign_strips_done",
            &[
                ("n", &strips.len().to_string()),
                ("len", &crate::ui::signcut_docker::fmt_len(u, total)),
            ],
        );
        strips.len()
    }

    /// The tile grid over the selection, with the panel's settings.
    pub fn sign_tile_grid(&self) -> Option<tiles::TileGrid> {
        let ids = self.sign_targets();
        let bounds = Self::sign_bounds(&self.sign_outlines(&ids))?;
        let origin = match self.sc_sign.tile_origin {
            TileOrigin::Selection => Point::new(bounds.x0, bounds.y0),
            TileOrigin::Page => Point::ZERO,
        };
        tiles::grid(bounds, origin, &self.sc_sign.tiles)
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
        let origin = match self.sc_sign.tile_origin {
            TileOrigin::Selection => Point::new(bounds.x0, bounds.y0),
            TileOrigin::Page => Point::ZERO,
        };
        let spec = self.sc_sign.tiles;
        let Some(grid) = tiles::grid(bounds, origin, &spec) else {
            self.status = tr("signcut.sign_tiles_bad");
            return 0;
        };
        let mut groups = Vec::new();
        let mut parts = 0usize;
        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let rect = grid.rect(col, row);
                let shift = Affine::translate(grid.spread(col, row, spec.gap));
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
                if self.sc_sign.tile_frames {
                    let frame = shift * rect.to_path(0.01);
                    let mut f = self.sign_path(frame, Fill::None, hairline(TILE_FRAME_COLOR));
                    f.stroke = f.stroke.map(|s| Stroke {
                        dash: vec![30.0, 20.0],
                        ..s
                    });
                    children.push(f);
                }
                if self.sc_sign.tile_labels {
                    let size = (rect.width().min(rect.height()) * 0.04).clamp(3.0, 50.0);
                    let at = shift * Point::new(rect.x0 + size * 0.5, rect.y1 - size * 1.5);
                    let t = self.sign_label(grid.label(col, row), at, size);
                    children.push(t);
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

    /// One carrier outline around the selected letters, put under the
    /// lowest of them. Returns the new object.
    pub fn sign_carrier(&mut self) -> Option<ShapeId> {
        let (ids, _) = self.sign_need_selection()?;
        let outline = carrier::carrier(&self.sign_outlines(&ids), &self.sc_sign.carrier);
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

/// Tools > Sign & Cut > Sign Making: each item opens the panel at its
/// tool.
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
        app.sc_sign.module = 0;
        app.sc_sign.led = LedSpec {
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
        app.sc_sign.neon_outline = true;
        app.select(vec![l]);
        app.sign_neon();
        let group = *top_ids(&app).last().expect("the group");
        let kids = children(&app, group);
        assert!(matches!(kids[0].kind, ShapeKind::Path { closed: true, .. }));
    }

    #[test]
    fn sign_box_uses_the_selection_or_the_panel_size() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 100.0, 60.0);
        app.select(vec![a]);
        app.sc_sign.sign_box.depth = 20.0;
        app.sc_sign.sign_box.holes = true;
        app.sc_sign.sign_box.hole_spacing = 1000.0;
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
        // Nothing selected: the panel's size on the page's centre.
        app.select(vec![]);
        app.sc_sign.box_width = 300.0;
        app.sc_sign.box_height = 200.0;
        app.sc_sign.sign_box.holes = false;
        assert!(app.sign_make_box());
        let group = *top_ids(&app).last().expect("the group");
        let bb = children(&app, group)[0].page_path().bounding_box();
        assert!((bb.width() - 340.0).abs() < 1e-6);
        assert!((bb.center().x - app.page_rect().center().x).abs() < 1e-6);
    }

    #[test]
    fn returns_lay_strips_under_the_letters() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 100.0, 100.0, 100.0);
        let b = rect(&mut app, 150.0, 100.0, 50.0, 100.0);
        app.select(vec![a, b]);
        app.sc_sign.returns.depth = 40.0;
        app.sc_sign.returns.overlap = 10.0;
        assert_eq!(app.sign_returns(), 2);
        let (n, total) = app.sc_sign.returns_report.expect("a report");
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
        assert!(matches!(kids[1].kind, ShapeKind::Text { .. }));
    }

    #[test]
    fn mould_adds_bases_and_wall_strips() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 100.0, 100.0, 100.0);
        app.select(vec![a]);
        app.sc_sign.labels = false;
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
    fn smart_divide_replaces_objects_with_tiles() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 250.0, 120.0);
        app.select(vec![a]);
        app.sc_sign.tiles = TileSpec {
            width: 100.0,
            height: 100.0,
            overlap: 10.0,
            gap: 5.0,
        };
        app.sc_sign.tile_frames = true;
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
    fn carrier_goes_under_the_letters() {
        let mut app = App::headless();
        let below = rect(&mut app, 500.0, 500.0, 10.0, 10.0);
        let a = rect(&mut app, 0.0, 0.0, 50.0, 50.0);
        let b = rect(&mut app, 65.0, 0.0, 50.0, 50.0);
        app.select(vec![a, b]);
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
    fn the_menu_opens_the_panel_at_a_tool() {
        let mut app = App::headless();
        app.open_sign_tool(Tool::Tiles);
        assert_eq!(app.sc_sign.open, Some(Tool::Tiles));
        assert!(app
            .sc_windows
            .is_open(crate::signcut_tools::ToolId::SmartDivide));
    }
}
