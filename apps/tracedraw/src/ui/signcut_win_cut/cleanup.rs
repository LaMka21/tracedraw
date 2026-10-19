//! The small clean-up windows: Find Duplicates, Remove Duplicate Edges
//! and Find Crossings. Each has the tolerance, its options and a line
//! with the last result. The operations are in `signcut.rs`.

use super::{note, scope};
use crate::app::App;
use crate::i18n::tr;
use crate::ui::signcut_kit::{self as kit, Buttons, Pressed};
use egui::Ui;
use tracedraw_core::geometry::{Circle, Shape as _};

/// The tolerance row (shared by the three windows).
fn tolerance(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    kit::row(ui, &tr("signcut.rn_tolerance"), |ui| {
        kit::length(ui, u, &mut app.signcut.tolerance, 0.001, 100.0)
    });
}

/// The last result, or what to press.
fn result(ui: &mut Ui, last: &Option<String>, hint: &str) {
    ui.add_space(4.0);
    match last {
        Some(r) => {
            ui.label(egui::RichText::new(r).strong());
        }
        None => note(ui, hint),
    }
}

/// Find Duplicates: selects or deletes the objects that repeat an
/// earlier one.
pub fn duplicates(app: &mut App, ui: &mut Ui) -> bool {
    kit::group(ui, &tr("signcut.duplicates"), |ui| {
        tolerance(app, ui);
        ui.checkbox(
            &mut app.signcut.delete_duplicates,
            tr("signcut.delete_duplicates"),
        );
    });
    scope(app, ui);
    result(
        ui,
        &app.sc_cut.win.duplicates,
        &tr("signcut.wcut_press_apply"),
    );
    match kit::bottom(ui, Buttons::ApplyOkCancel, true, |_| {}) {
        Some(Pressed::Apply) => {
            run_duplicates(app);
            false
        }
        Some(Pressed::Ok) => {
            run_duplicates(app);
            app.sc_cut.win.duplicates = None;
            true
        }
        Some(_) => {
            app.sc_cut.win.duplicates = None;
            true
        }
        None => false,
    }
}

fn run_duplicates(app: &mut App) -> usize {
    let n = app.signcut_find_duplicates();
    app.sc_cut.win.duplicates = Some(app.status.clone());
    n
}

/// Remove Duplicate Edges: takes out stretches of outline that run along
/// an earlier edge.
pub fn edges(app: &mut App, ui: &mut Ui) -> bool {
    kit::group(ui, &tr("signcut.duplicate_edges"), |ui| {
        tolerance(app, ui);
        note(ui, &tr("signcut.duplicate_edges_hint"));
    });
    scope(app, ui);
    result(ui, &app.sc_cut.win.edges, &tr("signcut.wcut_press_apply"));
    match kit::bottom(ui, Buttons::ApplyOkCancel, true, |_| {}) {
        Some(Pressed::Apply) => {
            run_edges(app);
            false
        }
        Some(Pressed::Ok) => {
            run_edges(app);
            app.sc_cut.win.edges = None;
            true
        }
        Some(_) => {
            app.sc_cut.win.edges = None;
            true
        }
        None => false,
    }
}

fn run_edges(app: &mut App) -> f64 {
    let removed = app.signcut_remove_duplicate_edges();
    app.sc_cut.win.edges = Some(app.status.clone());
    removed
}

/// Find Crossings: selects the objects whose outlines cross and marks
/// the crossings on the drawing while the window is open. Changes
/// nothing in the drawing, so it has Find and Close.
pub fn crossings(app: &mut App, ui: &mut Ui) -> bool {
    let r = crate::signcut::CROSSING_MARK_PX / f64::from(app.view.zoom.max(1e-3));
    let marks = app.signcut_crossing_marks();
    for p in &marks {
        app.docker_preview
            .push(Circle::new(*p, r).to_path(r * 0.05));
    }
    kit::group(ui, &tr("signcut.crossings"), |ui| {
        tolerance(app, ui);
        ui.checkbox(
            &mut app.signcut.crossings_between,
            tr("signcut.crossings_between"),
        );
    });
    scope(app, ui);
    // The result holds while the drawing is as it was searched.
    let last = if marks.is_empty() && app.sc_cut.win.crossings.is_some() {
        Some(tr("signcut.no_crossings"))
    } else {
        app.sc_cut.win.crossings.clone()
    };
    result(ui, &last, &tr("signcut.wcut_press_find"));
    let mut find = false;
    let pressed = kit::bottom(ui, Buttons::Close, true, |ui| {
        find = kit::push(ui, &tr("signcut.wcut_find"), true);
    });
    if find {
        app.signcut_find_crossings();
        app.sc_cut.win.crossings = Some(app.status.clone());
    }
    if matches!(pressed, Some(Pressed::Close)) {
        app.sc_cut.win.crossings = None;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signcut_tools::ToolId;
    use tracedraw_core::geometry::{BezPath, Rect};
    use tracedraw_core::ShapeKind;

    fn square(app: &mut App, x: f64) -> tracedraw_core::ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, 0.0, x + 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .expect("layer")
    }

    #[test]
    fn duplicates_window_draws_and_finds_in_one_step() {
        let mut app = App::headless();
        square(&mut app, 0.0);
        square(&mut app, 0.0);
        let rev = app.engine.revision();
        for delete in [false, true] {
            app.signcut.delete_duplicates = delete;
            super::super::draw_headless(&mut app, ToolId::FindDuplicates);
        }
        assert_eq!(app.engine.revision(), rev, "drawing it changes nothing");
        assert_eq!(run_duplicates(&mut app), 1);
        assert_eq!(app.engine.undo_label(), Some("Delete Duplicates"));
        assert!(app.sc_cut.win.duplicates.is_some());
        super::super::draw_headless(&mut app, ToolId::FindDuplicates);
    }

    #[test]
    fn edges_window_draws_and_removes_in_one_step() {
        let mut app = App::headless();
        square(&mut app, 0.0);
        square(&mut app, 10.0);
        let rev = app.engine.revision();
        super::super::draw_headless(&mut app, ToolId::RemoveDuplicateEdges);
        assert_eq!(app.engine.revision(), rev, "drawing it changes nothing");
        assert!(run_edges(&mut app) > 0.0);
        assert_eq!(app.engine.undo_label(), Some("Remove Duplicate Edges"));
        assert!(app.sc_cut.win.edges.is_some());
        super::super::draw_headless(&mut app, ToolId::RemoveDuplicateEdges);
    }

    #[test]
    fn crossings_window_marks_what_it_found() {
        let mut app = App::headless();
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 10.0));
        p.line_to((10.0, 0.0));
        p.line_to((0.0, 10.0));
        p.close_path();
        app.new_shape(ShapeKind::Path {
            path: p,
            closed: true,
        })
        .expect("layer");
        for between in [false, true] {
            app.signcut.crossings_between = between;
            super::super::draw_headless(&mut app, ToolId::FindCrossings);
        }
        let rev = app.engine.revision();
        assert_eq!(app.signcut_find_crossings(), 1);
        app.sc_cut.win.crossings = Some(app.status.clone());
        app.docker_preview.clear();
        super::super::draw_headless(&mut app, ToolId::FindCrossings);
        assert_eq!(app.docker_preview.len(), 1, "the crossing is marked");
        assert_eq!(app.engine.revision(), rev, "nothing in the drawing changed");
    }
}
