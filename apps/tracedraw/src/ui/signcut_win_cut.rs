//! The Sign & Cut tool windows of cutting and clean-up: Plotter Output
//! (`plotter`), Weeding Lines, Bridges and Cut Order (`prepare`), Print
//! and Cut Marks and Crop Marks (`marks`), and the small clean-up windows
//! Find Duplicates, Remove Duplicate Edges and Find Crossings (`cleanup`).
//! See docs/behavior/sign-and-cut-cut.md.

mod cleanup;
mod marks;
mod plotter;
mod prepare;

use crate::app::App;
use crate::i18n::tr;
use crate::signcut_tools::ToolId;
use crate::ui::signcut_kit::{self as kit, Scene};
use egui::{Color32, Ui, Vec2};
use tracedraw_core::geometry::{BezPath, Circle, ParamCurve, PathEl, Point, Rect, Shape as _};

/// The tools whose windows this file draws.
pub const TOOLS: &[ToolId] = &[
    ToolId::PlotterOutput,
    ToolId::WeedingLines,
    ToolId::PrintAndCut,
    ToolId::CropMarks,
    ToolId::Bridges,
    ToolId::CutOrder,
    ToolId::FindCrossings,
    ToolId::FindDuplicates,
    ToolId::RemoveDuplicateEdges,
];

/// A window's default size and whether it can be resized.
pub fn size(t: ToolId) -> (Vec2, bool) {
    match t {
        ToolId::PlotterOutput => (Vec2::new(1000.0, 700.0), true),
        ToolId::WeedingLines
        | ToolId::Bridges
        | ToolId::CutOrder
        | ToolId::PrintAndCut
        | ToolId::CropMarks => (Vec2::new(900.0, 620.0), true),
        _ => (Vec2::new(360.0, 230.0), false),
    }
}

/// The window's body. Returns true when it should close.
pub fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    match t {
        ToolId::PlotterOutput => plotter::body(app, ui),
        ToolId::WeedingLines => prepare::weeding(app, ui),
        ToolId::Bridges => prepare::bridges(app, ui),
        ToolId::CutOrder => prepare::cut_order(app, ui),
        ToolId::PrintAndCut => marks::print_and_cut(app, ui),
        ToolId::CropMarks => marks::crop(app, ui),
        ToolId::FindDuplicates => cleanup::duplicates(app, ui),
        ToolId::RemoveDuplicateEdges => cleanup::edges(app, ui),
        ToolId::FindCrossings => cleanup::crossings(app, ui),
        _ => true,
    }
}

/// Colours of the previews besides the kit's.
mod col {
    use egui::Color32;
    /// Weeding cuts and the weeding frame.
    pub const WEED: Color32 = Color32::from_rgb(0x00, 0x96, 0xFF);
    /// Moves with the knife up.
    pub const TRAVEL: Color32 = Color32::from_rgb(0xE0, 0x20, 0x20);
    /// Start points and directions.
    pub const START: Color32 = Color32::from_rgb(0xE0, 0x20, 0x20);
    /// The plotter's origin.
    pub const ORIGIN: Color32 = Color32::from_rgb(0x10, 0x90, 0x30);
    /// The cut contour.
    pub const CONTOUR: Color32 = Color32::from_rgb(0xE0, 0x00, 0xA0);
    /// Boxes and frames that are not cut.
    pub const GUIDE: Color32 = Color32::from_rgb(0x99, 0xBB, 0xDD);
}

/// The two-column body of a window with a preview: the preview with its
/// status strip at the left, the settings (scrolled) and the Units row
/// at the right.
fn columns(
    app: &mut App,
    ui: &mut Ui,
    id: &str,
    scene: &Scene,
    strip: &str,
    settings: impl FnOnce(&mut App, &mut Ui),
) {
    let height = (ui.available_height() - 60.0).max(380.0);
    ui.horizontal_top(|ui| {
        let left_w = (ui.available_width() - kit::SIDE_W - 12.0).max(300.0);
        ui.vertical(|ui| {
            ui.set_width(left_w);
            kit::preview(ui, Vec2::new(left_w, height - 22.0), scene);
            kit::status_strip(ui, left_w, strip);
        });
        ui.vertical(|ui| {
            ui.set_width(kit::SIDE_W);
            egui::ScrollArea::vertical()
                .id_salt(("signcut_cut_settings", id))
                .max_height(height - 34.0)
                .show(ui, |ui| settings(app, ui));
            kit::units_row(ui, &mut app.units);
        });
    });
}

/// A single-line text field of the row width (a widget the kit lacks).
fn text_field(ui: &mut Ui, text: &mut String) -> egui::Response {
    ui.add_sized([kit::FIELD_W, 22.0], egui::TextEdit::singleline(text))
}

/// A radio button with a field at the right edge, the field greyed out
/// while the button is not chosen.
fn radio_row<T: PartialEq + Copy>(
    ui: &mut Ui,
    current: &mut T,
    value: T,
    label: &str,
    field: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.radio_value(current, value, label);
        let on = *current == value;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(on, field);
        });
    });
}

/// A line of dim text that wraps.
fn note(ui: &mut Ui, text: &str) {
    ui.add(
        egui::Label::new(egui::RichText::new(text).color(crate::theme::Tokens::text_dim())).wrap(),
    );
}

/// Which objects the tools work on.
fn scope(app: &App, ui: &mut Ui) {
    note(
        ui,
        &tr(if app.selection.is_empty() {
            "signcut.whole_page"
        } else {
            "signcut.selection_only"
        }),
    );
}

/// The bounding box of the finite paths.
fn bounds(paths: &[BezPath]) -> Option<Rect> {
    paths
        .iter()
        .filter(|p| !p.elements().is_empty())
        .map(|p| p.bounding_box())
        .filter(|b| b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite())
        .reduce(|a, b| a.union(b))
}

/// The size of start marks for a drawing of extent `b`.
fn mark_size(b: Option<Rect>) -> f64 {
    b.map(|b| b.width().hypot(b.height()) * 0.006)
        .filter(|r| r.is_finite() && *r > 0.0)
        .unwrap_or(1.0)
}

fn add_all(scene: &mut Scene, paths: &[BezPath], color: Color32, width: f32) {
    for p in paths {
        scene.add(p.clone(), color, width);
    }
}

/// Thin lines for the moves with the knife up.
fn add_moves(scene: &mut Scene, moves: &[(Point, Point)]) {
    for (a, b) in moves {
        let mut p = BezPath::new();
        p.move_to(*a);
        p.line_to(*b);
        scene.add(p, col::TRAVEL, 1.0);
    }
}

/// A dot of radius `r` at `p`.
fn dot(scene: &mut Scene, p: Point, r: f64, color: Color32) {
    scene.add(Circle::new(p, r).to_path(r * 0.05), color, 2.5);
}

/// An arrow head from `p` pointing along `dir` (unit), `r` the dot size.
fn arrow(scene: &mut Scene, p: Point, dir: tracedraw_core::geometry::Vec2, r: f64, color: Color32) {
    let n = tracedraw_core::geometry::Vec2::new(-dir.y, dir.x);
    let base = p + dir * (r * 1.5);
    let mut t = BezPath::new();
    t.move_to(base + n * (r * 1.2));
    t.line_to(base + dir * (r * 3.0));
    t.line_to(base - n * (r * 1.2));
    t.close_path();
    scene.add(t, color, 2.0);
}

/// Where each subpath of `paths` starts and which way it runs.
fn starts(paths: &[BezPath]) -> Vec<(Point, Option<tracedraw_core::geometry::Vec2>)> {
    let mut out = Vec::new();
    for p in paths {
        for sp in crate::signcut_cut::split_subpaths(p) {
            let Some(PathEl::MoveTo(at)) = sp.elements().first().copied() else {
                continue;
            };
            let dir = sp.segments().find_map(|seg| {
                let v = seg.eval(0.05) - seg.start();
                let l = v.hypot();
                (l.is_finite() && l > 1e-12).then(|| v / l)
            });
            out.push((at, dir));
        }
    }
    out
}

/// Red start dots with direction arrows for every subpath of `paths`.
fn add_starts(scene: &mut Scene, paths: &[BezPath], r: f64) {
    for (at, dir) in starts(paths) {
        dot(scene, at, r, col::START);
        if let Some(d) = dir {
            arrow(scene, at, d, r, col::START);
        }
    }
}

/// A rectangle as a closed path.
fn rect_path(r: Rect) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((r.x0, r.y0));
    p.line_to((r.x1, r.y0));
    p.line_to((r.x1, r.y1));
    p.line_to((r.x0, r.y1));
    p.close_path();
    p
}

/// The headless frame the window tests draw in.
#[cfg(test)]
fn draw_headless(app: &mut App, t: ToolId) {
    let ctx = crate::theme::ui_context();
    let (size, _) = size(t);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            size + egui::vec2(100.0, 100.0),
        )),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        let _ = body(app, ui, t);
    });
    out.textures_delta.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_window_draws_with_nothing_on_the_page() {
        let mut app = App::headless();
        for t in TOOLS {
            draw_headless(&mut app, *t);
        }
        assert!(app.engine.undo_label().is_none());
    }

    #[test]
    fn start_marks_follow_the_first_segment() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((0.0, 10.0));
        let s = starts(&[p]);
        assert_eq!(s.len(), 1);
        let (at, dir) = s[0];
        assert_eq!(at, Point::new(0.0, 0.0));
        let d = dir.expect("a direction");
        assert!((d.y - 1.0).abs() < 1e-9 && d.x.abs() < 1e-9);
        assert_eq!(mark_size(None), 1.0);
        assert!(bounds(&[BezPath::new()]).is_none());
    }
}
