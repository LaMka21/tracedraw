//! Sign & Cut, Effects and Patterns: settings and operations (hatching lines, grids, puzzles, circle fills, cast shadows and random copies).
//! The geometry is in `tracedraw_signcut::fx`; the panel section is
//! `ui/signcut_fx.rs`. See docs/behavior/sign-and-cut-fx.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use egui::Ui;
use tracedraw_core::geometry::{BezPath, Point, Rect};
use tracedraw_core::{Color, Command, Fill, LayerId, Shape, ShapeId, ShapeKind, Stroke};
use tracedraw_signcut::fx::{
    self, CircleSpec, GridSpec, HatchSpec, PuzzleSpec, Region, ScatterSpec, ShadowSpec,
};

/// The Effects and Patterns section's settings.
#[derive(Debug, Clone)]
pub struct Settings {
    pub hatch: HatchSpec,
    pub grid: GridSpec,
    /// The grid fills the selection's box (else it starts at the origin
    /// with the cell size).
    pub grid_fill_selection: bool,
    /// The grid as rectangles (else as lines).
    pub grid_cells: bool,
    /// The grid's bottom-left corner on the page (mm).
    pub grid_origin: Point,
    /// Show the grid's lines on the drawing while the section is open.
    pub grid_preview: bool,
    /// Create Lines: how many and which way.
    pub lines_count: usize,
    pub lines_vertical: bool,
    pub puzzle: PuzzleSpec,
    /// The puzzle as separate pieces (else the cut lines only).
    pub puzzle_pieces: bool,
    pub circles: CircleSpec,
    pub shadow: ShadowSpec,
    pub shadow_color: [u8; 3],
    /// 1 is opaque.
    pub shadow_opacity: f64,
    pub scatter: ScatterSpec,
    /// The shape the random copies go into (else the page).
    pub container: Option<ShapeId>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hatch: HatchSpec::default(),
            grid: GridSpec::default(),
            grid_fill_selection: false,
            grid_cells: false,
            grid_origin: Point::new(10.0, 10.0),
            grid_preview: false,
            lines_count: 3,
            lines_vertical: false,
            puzzle: PuzzleSpec::default(),
            puzzle_pieces: false,
            circles: CircleSpec::default(),
            shadow: ShadowSpec::default(),
            shadow_color: [64, 64, 64],
            shadow_opacity: 0.5,
            scatter: ScatterSpec::default(),
            container: None,
        }
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
    fn fx_selection_bounds(&self) -> Option<Rect> {
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

    fn fx_need_selection(&mut self) -> Option<Vec<ShapeId>> {
        let ids = self.fx_targets();
        if ids.is_empty() {
            self.status = tr("signcut.fx_select_first");
            None
        } else {
            Some(ids)
        }
    }

    /// Fill the selected closed shapes with hatch lines: one open curve
    /// per object, all in one group. Returns the number of lines.
    pub fn fx_hatch(&mut self) -> usize {
        let Some(ids) = self.fx_need_selection() else {
            return 0;
        };
        let spec = self.sc_fx.hatch;
        let doc = self.doc();
        let found: Vec<(BezPath, Stroke, usize)> = ids
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .filter_map(|s| {
                let lines = fx::hatch(&tracedraw_signcut::outlines(&s), &spec);
                (!lines.is_empty()).then(|| (path_of(&lines), hairline_for(&s), lines.len()))
            })
            .collect();
        let Some(layer) = ids.first().and_then(|id| self.fx_layer_of(*id)) else {
            return 0;
        };
        let n: usize = found.iter().map(|(_, _, n)| n).sum();
        let children: Vec<Shape> = found
            .into_iter()
            .map(|(path, stroke, _)| open_curve(self.engine.new_shape_id(), path, stroke))
            .collect();
        if self
            .fx_add_groups("Hatching", vec![(layer, children)], Vec::new())
            .is_empty()
        {
            self.status = tr("signcut.fx_nothing");
            return 0;
        }
        self.status = trf("signcut.fx_hatch_done", &[("n", &n.to_string())]);
        n
    }

    /// The grid's origin and cells: filling the selection's box when
    /// asked and something is selected, else from the panel's origin.
    pub fn fx_grid_layout(&self) -> Option<(Point, GridSpec)> {
        let s = &self.sc_fx;
        if s.grid_fill_selection {
            if let Some(b) = self.fx_selection_bounds() {
                return Some((Point::new(b.x0, b.y0), s.grid.fill(b)?));
            }
        }
        Some((s.grid_origin, s.grid))
    }

    /// Make the grid: one curve per line or one rectangle per cell, in
    /// one group. Returns the number of objects.
    pub fn fx_grid(&mut self) -> usize {
        let Some((origin, spec)) = self.fx_grid_layout() else {
            self.status = tr("signcut.fx_nothing");
            return 0;
        };
        let Some(layer) = self.active_layer() else {
            return 0;
        };
        let kinds: Vec<ShapeKind> = if self.sc_fx.grid_cells {
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
        self.fx_add_kinds("Grid", layer, kinds)
    }

    /// Add `kinds` as new objects with the default fill and outline, in
    /// one group. Returns how many.
    fn fx_add_kinds(
        &mut self,
        label: &'static str,
        layer: LayerId,
        kinds: Vec<ShapeKind>,
    ) -> usize {
        let children: Vec<Shape> = kinds
            .into_iter()
            .map(|k| {
                let mut s = Shape::new(self.engine.new_shape_id(), k);
                self.doc().defaults.apply_to(&mut s);
                s
            })
            .collect();
        let n = children.len();
        if self
            .fx_add_groups(label, vec![(layer, children)], Vec::new())
            .is_empty()
        {
            self.status = tr("signcut.fx_nothing");
            return 0;
        }
        self.status = trf("signcut.fx_objects_done", &[("n", &n.to_string())]);
        n
    }

    /// The box Create Lines divides: the selection's, else the page.
    pub fn fx_lines_box(&self) -> Rect {
        self.fx_selection_bounds()
            .unwrap_or_else(|| self.page_rect())
    }

    /// Lines across the selection's box (or the page). Returns how many.
    pub fn fx_lines(&mut self) -> usize {
        let b = self.fx_lines_box();
        let Some(layer) = self.active_layer() else {
            return 0;
        };
        let kinds = fx::across_lines(b, self.sc_fx.lines_count, self.sc_fx.lines_vertical)
            .into_iter()
            .map(|l| ShapeKind::Path {
                path: fx::polyline_path(&l),
                closed: false,
            })
            .collect();
        self.fx_add_kinds("Create Lines", layer, kinds)
    }

    /// Cut the selected shapes into a puzzle: per object one group of cut
    /// lines over it, or of pieces in its place. Returns the number of
    /// lines or pieces.
    pub fn fx_puzzle(&mut self) -> usize {
        let Some(ids) = self.fx_need_selection() else {
            return 0;
        };
        let spec = self.sc_fx.puzzle;
        let pieces = self.sc_fx.puzzle_pieces;
        let doc = self.doc();
        let mut found: Vec<(ShapeId, Shape, Vec<BezPath>)> = Vec::new();
        for id in &ids {
            let Some(s) = doc.shape_on_page(*id) else {
                continue;
            };
            let parts = fx::puzzle(&tracedraw_signcut::outlines(&s), &spec, pieces);
            if !parts.is_empty() {
                found.push((*id, s, parts));
            }
        }
        let mut groups = Vec::new();
        let mut gone = Vec::new();
        let mut n = 0;
        for (id, s, parts) in found {
            let Some(layer) = self.fx_layer_of(id) else {
                continue;
            };
            n += parts.len();
            let children: Vec<Shape> = parts
                .into_iter()
                .map(|path| {
                    let nid = self.engine.new_shape_id();
                    if pieces {
                        let mut c = Shape::new(nid, ShapeKind::Path { path, closed: true });
                        if matches!(s.kind, ShapeKind::Group { .. }) {
                            self.doc().defaults.apply_to(&mut c);
                        } else {
                            c.fill = s.fill.clone();
                            c.stroke = s.stroke.clone();
                        }
                        c
                    } else {
                        open_curve(nid, path, hairline_for(&s))
                    }
                })
                .collect();
            groups.push((layer, children));
            gone.push(id);
        }
        let extra = if pieces && !gone.is_empty() {
            vec![Command::DeleteShapes { shapes: gone }]
        } else {
            Vec::new()
        };
        if self.fx_add_groups("Puzzle", groups, extra).is_empty() {
            self.status = tr("signcut.fx_nothing");
            return 0;
        }
        let key = if pieces {
            "signcut.fx_pieces_done"
        } else {
            "signcut.fx_cut_lines_done"
        };
        self.status = trf(key, &[("n", &n.to_string())]);
        n
    }

    /// Fill the selected closed shapes with circles, one group per object.
    /// Returns the number of circles.
    pub fn fx_circle_fill(&mut self) -> usize {
        let Some(ids) = self.fx_need_selection() else {
            return 0;
        };
        let spec = self.sc_fx.circles;
        let doc = self.doc();
        let found: Vec<(ShapeId, Vec<fx::Disc>)> = ids
            .iter()
            .filter_map(|id| Some((*id, doc.shape_on_page(*id)?)))
            .map(|(id, s)| (id, fx::circle_fill(&tracedraw_signcut::outlines(&s), &spec)))
            .filter(|(_, d)| !d.is_empty())
            .collect();
        let mut groups = Vec::new();
        let mut n = 0;
        for (id, discs) in found {
            let Some(layer) = self.fx_layer_of(id) else {
                continue;
            };
            n += discs.len();
            let children = discs
                .into_iter()
                .map(|d| {
                    let r = d.radius;
                    let kind = ShapeKind::Ellipse {
                        rect: Rect::new(
                            d.center.x - r,
                            d.center.y - r,
                            d.center.x + r,
                            d.center.y + r,
                        ),
                        arc: None,
                    };
                    let mut s = Shape::new(self.engine.new_shape_id(), kind);
                    self.doc().defaults.apply_to(&mut s);
                    s
                })
                .collect();
            groups.push((layer, children));
        }
        if self
            .fx_add_groups("Circle Fill", groups, Vec::new())
            .is_empty()
        {
            self.status = tr("signcut.fx_nothing");
            return 0;
        }
        self.status = trf("signcut.fx_circles_done", &[("n", &n.to_string())]);
        n
    }

    /// Give each selected object a flat shadow, filled with the shadow
    /// colour and opacity, placed right behind the object (behind its
    /// group when it is inside one). Returns how many.
    pub fn fx_cast_shadow(&mut self) -> usize {
        let Some(ids) = self.fx_need_selection() else {
            return 0;
        };
        let spec = self.sc_fx.shadow;
        let [r, g, b] = self.sc_fx.shadow_color;
        let fill = Fill::Solid(Color::rgb8(r, g, b));
        let opacity = if self.sc_fx.shadow_opacity.is_finite() {
            self.sc_fx.shadow_opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let doc = self.doc();
        let mut found: Vec<(ShapeId, BezPath)> = Vec::new();
        for id in &ids {
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
            found.push((top, t * outline));
        }
        let mut cmds = Vec::new();
        let mut new_ids = Vec::new();
        for (top, path) in found {
            let Some(layer) = self.fx_layer_of(top) else {
                continue;
            };
            let sid = self.engine.new_shape_id();
            let mut s = Shape::new(sid, ShapeKind::Path { path, closed: true });
            s.fill = fill.clone();
            s.stroke = None;
            s.opacity = opacity;
            cmds.push(Command::AddShape { layer, shape: s });
            cmds.push(Command::OrderRelative {
                shapes: vec![sid],
                reference: top,
                in_front: false,
            });
            new_ids.push(sid);
        }
        let n = new_ids.len();
        if n == 0 {
            self.status = tr("signcut.fx_nothing");
            return 0;
        }
        if let Err(e) = self.engine.run_batch("Cast Shadow", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.select(new_ids);
        self.status = trf("signcut.fx_shadows_done", &[("n", &n.to_string())]);
        n
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

    /// Copy the selection (the container left out) to random places
    /// inside the container or the page. Each place gets one copy of
    /// each object, grouped when there are several. Returns how many
    /// places were filled.
    pub fn fx_random_copies(&mut self) -> usize {
        let container = self.fx_container();
        let ids: Vec<ShapeId> = self
            .fx_targets()
            .into_iter()
            .filter(|id| Some(*id) != container)
            .collect();
        if ids.is_empty() {
            self.status = tr("signcut.fx_select_first");
            return 0;
        }
        let doc = self.doc();
        let region = match container.and_then(|c| doc.shape_on_page(c)) {
            Some(c) => {
                Region::from_paths(&tracedraw_signcut::outlines(&c), fx::region::FLATTEN_TOL)
            }
            None => Region::from_polygons(vec![{
                let p = self.page_rect();
                vec![
                    Point::new(p.x0, p.y0),
                    Point::new(p.x1, p.y0),
                    Point::new(p.x1, p.y1),
                    Point::new(p.x0, p.y1),
                ]
            }]),
        };
        let sources: Vec<Shape> = ids.iter().filter_map(|id| doc.shape_on_page(*id)).collect();
        let Some(bounds) = sources.iter().map(|s| s.bounds()).reduce(|a, b| a.union(b)) else {
            return 0;
        };
        let Some(layer) = self.active_layer() else {
            return 0;
        };
        let want = self.sc_fx.scatter.count;
        let places = fx::scatter(bounds, &region, &self.sc_fx.scatter);
        let mut cmds = Vec::new();
        let mut new_ids = Vec::new();
        for t in &places {
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
                cmds.push(Command::AddShape { layer, shape });
            }
        }
        let n = new_ids.len();
        if n > 0 {
            if let Err(e) = self.engine.run_batch("Random Copies", &cmds) {
                self.status = e.to_string();
                return 0;
            }
            self.select(new_ids);
        }
        self.status = trf(
            "signcut.fx_copies_done",
            &[("n", &n.to_string()), ("want", &want.to_string())],
        );
        n
    }
}

/// Tools > Sign & Cut > Effects and Patterns: each item runs with the
/// panel's settings and opens the panel at the section.
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::signcut::Section;
    use crate::ui::menus::{item, sep};
    let doc = app.has_document() && !app.show_welcome;
    let sel = doc && !app.selection.is_empty();
    if item(ui, "menu.tools.signcut_fx_panel", "", doc) {
        app.open_signcut(Section::Fx);
    }
    sep(ui);
    if item(ui, "menu.tools.signcut_fx_hatch", "", sel) {
        app.open_signcut(Section::Fx);
        app.fx_hatch();
    }
    if item(ui, "menu.tools.signcut_fx_grid", "", doc) {
        app.open_signcut(Section::Fx);
        app.fx_grid();
    }
    if item(ui, "menu.tools.signcut_fx_lines", "", doc) {
        app.open_signcut(Section::Fx);
        app.fx_lines();
    }
    if item(ui, "menu.tools.signcut_fx_puzzle", "", sel) {
        app.open_signcut(Section::Fx);
        app.fx_puzzle();
    }
    if item(ui, "menu.tools.signcut_fx_circles", "", sel) {
        app.open_signcut(Section::Fx);
        app.fx_circle_fill();
    }
    if item(ui, "menu.tools.signcut_fx_shadow", "", sel) {
        app.open_signcut(Section::Fx);
        app.fx_cast_shadow();
    }
    if item(ui, "menu.tools.signcut_fx_copies", "", sel) {
        app.open_signcut(Section::Fx);
        app.fx_random_copies();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;
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
        app.sc_fx.grid = GridSpec {
            cols: 3,
            rows: 2,
            cell_w: 10.0,
            cell_h: 5.0,
        };
        app.sc_fx.grid_origin = Point::new(20.0, 30.0);
        app.select(vec![]);
        assert_eq!(app.fx_grid(), 4 + 3);
        assert_eq!(app.engine.undo_label(), Some("Grid"));
        let g = app.selection[0];
        let b = app.doc().shape_on_page(g).expect("grid").bounds();
        assert!((b.x0 - 20.0).abs() < 1e-9 && (b.x1 - 50.0).abs() < 1e-9);
        assert!((b.y1 - 40.0).abs() < 1e-9);
        app.sc_fx.grid_cells = true;
        assert_eq!(app.fx_grid(), 6);
        let kids = children(&app, app.selection[0]);
        assert!(kids
            .iter()
            .all(|k| matches!(k.kind, ShapeKind::Rect { .. })));
        // Fill the selection's box.
        let r = rect(&mut app, 100.0, 100.0, 60.0, 20.0);
        app.select(vec![r]);
        app.sc_fx.grid_fill_selection = true;
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
        app.sc_fx.lines_count = 2;
        assert_eq!(app.fx_lines(), 2);
        assert_eq!(app.engine.undo_label(), Some("Create Lines"));
        let kids = children(&app, app.selection[0]);
        let b = kids[1].bounds();
        assert!((b.y0 - 20.0).abs() < 1e-9 && (b.width() - 40.0).abs() < 1e-9);
        app.select(vec![]);
        app.sc_fx.lines_vertical = true;
        app.sc_fx.lines_count = 1;
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
        app.sc_fx.puzzle_pieces = true;
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
        app.sc_fx.circles.count = 25;
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
        app.sc_fx.shadow = ShadowSpec {
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
        app.sc_fx.shadow = ShadowSpec {
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
        app.sc_fx.scatter.count = 12;
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
}
