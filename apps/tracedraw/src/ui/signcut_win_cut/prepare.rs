//! The windows of Weeding Lines, Bridges and Cut Order: the preview at
//! the left (worked out with Apply, out of date once a setting or the
//! drawing changes), the settings at the right, Apply, OK and Cancel.

use super::{add_all, add_moves, add_starts, bounds, col, columns, mark_size, radio_row, scope};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_cut::{Basis, MADE, PRINTED};
use crate::ui::signcut_docker::fmt_len;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::Ui;
use tracedraw_core::geometry::{BezPath, Circle, Shape as _};
use tracedraw_signcut::cut::order::{Direction, StartAt};
use tracedraw_signcut::cut::weed::{WeedMode, MAX_DIVIDERS};

/// The strip before Apply: how many objects there are, or that there is
/// nothing to work on.
fn ready_strip(n: usize) -> String {
    if n == 0 {
        tr("signcut.cut_nothing")
    } else {
        trf("signcut.wcut_ready", &[("n", &n.to_string())])
    }
}

/// The objects in grey, as they are now.
fn originals(app: &mut App, skip: &[&str]) -> Scene {
    let mut scene = Scene::default();
    add_all(
        &mut scene,
        &app.sc_cut_preview_objects(skip),
        colors::ORIGINAL,
        1.0,
    );
    scene
}

/// The bottom row with the progress bar; `fresh` fills it.
fn buttons(ui: &mut Ui, fresh: bool, has: bool, left: impl FnOnce(&mut Ui)) -> Option<Pressed> {
    kit::progress(ui, if fresh { 1.0 } else { 0.0 });
    kit::bottom(ui, Buttons::ApplyOkCancel, has, left)
}

/// Weeding Lines.
pub fn weeding(app: &mut App, ui: &mut Ui) -> bool {
    let basis = app.sc_cut_weed_basis();
    let has = !basis.targets.is_empty();
    let fresh = matches!(&app.sc_cut.win.weed, Some((b, _)) if *b == basis);
    let (scene, strip) = match app.sc_cut.win.weed.as_ref().filter(|_| fresh) {
        Some((_, r)) => {
            let mut scene = Scene::default();
            add_all(&mut scene, &r.objects, colors::PART, 1.0);
            add_all(&mut scene, &r.cuts, col::WEED, 1.5);
            (scene, weed_strip(r.cuts.len(), basis.targets.len()))
        }
        None => (originals(app, &MADE), ready_strip(basis.targets.len())),
    };
    columns(app, ui, "weeding", &scene, &strip, weeding_settings);
    let pressed = buttons(ui, fresh, has, |_| {});
    match pressed {
        Some(Pressed::Apply) => {
            let r = app.sc_cut_weed_compute();
            app.status = weed_strip(r.cuts.len(), basis.targets.len());
            app.sc_cut.win.weed = Some((basis, r));
            false
        }
        Some(Pressed::Ok) => {
            let cuts = match app.sc_cut.win.weed.take() {
                Some((b, r)) if b == basis => r.cuts,
                _ => app.sc_cut_weed_compute().cuts,
            };
            app.sc_cut_weed_commit(cuts);
            true
        }
        Some(_) => {
            app.sc_cut.win.weed = None;
            true
        }
        None => false,
    }
}

fn weed_strip(n: usize, objects: usize) -> String {
    trf(
        "signcut.wcut_weed_strip",
        &[("n", &n.to_string()), ("objects", &objects.to_string())],
    )
}

fn weeding_settings(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let w = &mut app.sc_cut.weed;
    kit::group(ui, &tr("signcut.wcut_method"), |ui| {
        kit::row(ui, &tr("signcut.wcut_method"), |ui| {
            let opts = [
                (WeedMode::Lines, tr("signcut.cut_weed_lines")),
                (WeedMode::Boxes, tr("signcut.cut_weed_boxes")),
            ];
            kit::choice(ui, "wcut_weed_mode", &mut w.mode, &opts)
        });
    });
    match w.mode {
        WeedMode::Lines => {
            kit::group(ui, &tr("signcut.cut_weed_group"), |ui| {
                kit::row(ui, &tr("signcut.cut_gap"), |ui| {
                    kit::length(ui, u, &mut w.gap, 0.0, 1000.0)
                });
                kit::row(ui, &tr("signcut.cut_shorten"), |ui| {
                    kit::length(ui, u, &mut w.shorten, 0.0, 1000.0)
                });
                kit::row(ui, &tr("signcut.cut_dividers"), |ui| {
                    let mut n = w.dividers as usize;
                    let r = kit::count(ui, &mut n, 0, MAX_DIVIDERS as usize);
                    w.dividers = n.min(MAX_DIVIDERS as usize) as u32;
                    r
                });
            });
        }
        WeedMode::Boxes => {
            kit::group(ui, &tr("signcut.wcut_boxes"), |ui| {
                kit::row(ui, &tr("signcut.cut_box_x"), |ui| {
                    kit::length(ui, u, &mut w.box_x, 0.0, 1000.0)
                });
                kit::row(ui, &tr("signcut.cut_box_y"), |ui| {
                    kit::length(ui, u, &mut w.box_y, 0.0, 1000.0)
                });
            });
        }
    }
    kit::group(ui, &tr("signcut.wcut_frame"), |ui| {
        ui.checkbox(&mut w.frame, tr("signcut.wcut_frame_add"));
        ui.add_enabled_ui(w.frame, |ui| {
            kit::row(ui, &tr("signcut.nw_margin"), |ui| {
                kit::length(ui, u, &mut w.margin, 0.0, 10_000.0)
            });
        });
    });
    scope(app, ui);
}

/// Bridges.
pub fn bridges(app: &mut App, ui: &mut Ui) -> bool {
    let basis = app.sc_cut_bridges_basis();
    let has = !basis.targets.is_empty();
    let fresh = matches!(&app.sc_cut.win.bridges, Some((b, _)) if *b == basis);
    let (scene, strip) = match app.sc_cut.win.bridges.as_ref().filter(|_| fresh) {
        Some((_, r)) => {
            let mut scene = Scene::default();
            add_all(&mut scene, &r.objects, colors::ORIGINAL, 1.0);
            let pieces: Vec<BezPath> = r.changes.iter().map(|(_, p)| p.clone()).collect();
            add_all(&mut scene, &pieces, colors::PART, 1.5);
            let size = mark_size(bounds(&r.objects));
            for c in &r.centres {
                scene.add(
                    Circle::new(*c, size * 1.5).to_path(size * 0.05),
                    col::TRAVEL,
                    1.5,
                );
            }
            let strip = bridges_strip(r.bridges, r.changes.len());
            (scene, strip)
        }
        None => (originals(app, &PRINTED), ready_strip(basis.targets.len())),
    };
    columns(app, ui, "bridges", &scene, &strip, bridges_settings);
    match buttons(ui, fresh, has, |_| {}) {
        Some(Pressed::Apply) => {
            let r = app.sc_cut_bridges_compute();
            app.status = bridges_strip(r.bridges, r.changes.len());
            app.sc_cut.win.bridges = Some((basis, r));
            false
        }
        Some(Pressed::Ok) => {
            let r = match app.sc_cut.win.bridges.take() {
                Some((b, r)) if b == basis => r,
                _ => app.sc_cut_bridges_compute(),
            };
            app.sc_cut_bridges_commit(&r);
            true
        }
        Some(_) => {
            app.sc_cut.win.bridges = None;
            true
        }
        None => false,
    }
}

fn bridges_strip(n: usize, outlines: usize) -> String {
    trf(
        "signcut.wcut_bridges_strip",
        &[("n", &n.to_string()), ("outlines", &outlines.to_string())],
    )
}

fn bridges_settings(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_cut;
    kit::group(ui, &tr("signcut.cut_bridges"), |ui| {
        kit::row(ui, &tr("signcut.cut_bridge_length"), |ui| {
            kit::length(ui, u, &mut s.bridge.length, 0.01, 1000.0)
        });
        kit::separator(ui);
        radio_row(
            ui,
            &mut s.bridge_by_distance,
            false,
            &tr("signcut.cut_bridge_count"),
            |ui| {
                let mut n = s.bridge_count as usize;
                kit::count(ui, &mut n, 1, 1000);
                s.bridge_count = n.clamp(1, 1000) as u32;
            },
        );
        radio_row(
            ui,
            &mut s.bridge_by_distance,
            true,
            &tr("signcut.cut_bridge_every"),
            |ui| {
                kit::length(ui, u, &mut s.bridge_every, 1.0, 100_000.0);
            },
        );
        kit::separator(ui);
        kit::row(ui, &tr("signcut.cut_bridge_min"), |ui| {
            kit::length(ui, u, &mut s.bridge.min_outline, 0.0, 100_000.0)
        });
    });
    scope(app, ui);
}

/// Cut Order.
pub fn cut_order(app: &mut App, ui: &mut Ui) -> bool {
    let basis = app.sc_cut_route_basis();
    let has = !basis.targets.is_empty();
    let fresh = matches!(&app.sc_cut.win.route, Some((b, _)) if *b == basis);
    let original = app.sc_cut.win.route_original;
    let u = app.units;
    let (scene, strip, overlay) = match app.sc_cut.win.route.as_ref().filter(|_| fresh) {
        Some((_, r)) => {
            let mut scene = Scene::default();
            let size = mark_size(bounds(&r.drawn));
            let (cuts, moves) = if original {
                (&r.drawn, &r.moves_before)
            } else {
                (&r.planned, &r.moves)
            };
            add_all(&mut scene, cuts, colors::PART, 1.0);
            add_moves(&mut scene, moves);
            add_starts(&mut scene, cuts, size);
            let saved = if r.before > 1e-9 {
                (1.0 - r.after / r.before) * 100.0
            } else {
                0.0
            };
            let strip = trf(
                "signcut.wcut_route_strip",
                &[
                    ("before", &fmt_len(u, r.before)),
                    ("after", &fmt_len(u, r.after)),
                    ("saved", &format!("{saved:.0}")),
                ],
            );
            (scene, strip, moves.clone())
        }
        None => (
            originals(app, &PRINTED),
            ready_strip(basis.targets.len()),
            Vec::new(),
        ),
    };
    // The route on the drawing, while the window is open.
    if app.sc_cut.show_route {
        for (a, b) in overlay {
            let mut p = BezPath::new();
            p.move_to(a);
            p.line_to(b);
            app.docker_preview.push(p);
        }
    }
    columns(app, ui, "cut_order", &scene, &strip, order_settings);
    let pressed = buttons(ui, fresh, has, |ui| {
        if ui
            .add(egui::Button::new(tr("signcut.nw_show_original")).selected(original))
            .clicked()
        {
            app.sc_cut.win.route_original = !original;
        }
    });
    match pressed {
        Some(Pressed::Apply) => {
            apply_route(app, basis);
            false
        }
        Some(Pressed::Ok) => {
            app.sc_cut.win.route = None;
            app.sc_cut_order();
            true
        }
        Some(_) => {
            app.sc_cut.win.route = None;
            true
        }
        None => false,
    }
}

/// Plan the route into the preview.
fn apply_route(app: &mut App, basis: Basis) {
    let r = app.sc_cut_route_compute();
    let u = app.units;
    app.status = trf(
        "signcut.cut_travel",
        &[
            ("before", &fmt_len(u, r.before)),
            ("after", &fmt_len(u, r.after)),
        ],
    );
    app.sc_cut.win.route = Some((basis, r));
    app.sc_cut.win.route_original = false;
}

fn order_settings(app: &mut App, ui: &mut Ui) {
    let r = &mut app.sc_cut.route;
    kit::group(ui, &tr("signcut.cut_order"), |ui| {
        kit::row(ui, &tr("signcut.cut_direction"), |ui| {
            let opts = [
                (Direction::Keep, tr("signcut.cut_keep")),
                (Direction::Clockwise, tr("signcut.cut_dir_cw")),
                (Direction::CounterClockwise, tr("signcut.cut_dir_ccw")),
            ];
            kit::choice(ui, "wcut_direction", &mut r.direction, &opts)
        });
        kit::row(ui, &tr("signcut.cut_start"), |ui| {
            let opts = [
                (StartAt::Nearest, tr("signcut.cut_start_nearest")),
                (StartAt::BottomLeft, tr("signcut.cut_start_bottom_left")),
                (StartAt::Keep, tr("signcut.cut_keep")),
            ];
            kit::choice(ui, "wcut_start", &mut r.start, &opts)
        });
        ui.checkbox(&mut r.inner_first, tr("signcut.cut_inner_first"));
    });
    kit::group(ui, &tr("signcut.wcut_preview"), |ui| {
        ui.checkbox(&mut app.sc_cut.show_route, tr("signcut.cut_show_route"));
    });
    if let Some((before, after)) = app.sc_cut.last_route {
        let u = app.units;
        super::note(
            ui,
            &trf(
                "signcut.cut_order_done",
                &[
                    ("before", &fmt_len(u, before)),
                    ("after", &fmt_len(u, after)),
                ],
            ),
        );
    }
    scope(app, ui);
}

/// The start point a test can look for.
#[cfg(test)]
fn first_start(r: &crate::signcut_cut::RouteResult) -> Option<tracedraw_core::geometry::Point> {
    super::starts(&r.planned).first().map(|(p, _)| *p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signcut_tools::ToolId;
    use tracedraw_core::geometry::{Point, Rect};
    use tracedraw_core::ShapeKind;

    fn app_with_parts() -> App {
        let mut app = App::headless();
        let mut ids = Vec::new();
        for x in [100.0, 0.0, 50.0] {
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: Rect::new(x, 10.0, x + 20.0, 30.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("layer"),
            );
        }
        app.select(ids);
        app
    }

    #[test]
    fn weeding_window_draws_both_methods_and_previews() {
        let mut app = app_with_parts();
        for mode in [WeedMode::Lines, WeedMode::Boxes] {
            app.sc_cut.weed.mode = mode;
            super::super::draw_headless(&mut app, ToolId::WeedingLines);
        }
        app.sc_cut.weed.mode = WeedMode::Lines;
        let rev = app.engine.revision();
        let basis = app.sc_cut_weed_basis();
        let r = app.sc_cut_weed_compute();
        assert!(!r.cuts.is_empty());
        app.sc_cut.win.weed = Some((basis, r));
        super::super::draw_headless(&mut app, ToolId::WeedingLines);
        assert_eq!(app.engine.revision(), rev, "Apply changes nothing");
        assert!(app.sc_cut.win.weed.is_some());
        // A changed setting makes the preview out of date.
        app.sc_cut.weed.gap = 3.0;
        assert_ne!(
            app.sc_cut_weed_basis(),
            app.sc_cut.win.weed.as_ref().expect("kept").0
        );
    }

    #[test]
    fn weeding_ok_adds_the_previewed_cuts_in_one_step() {
        let mut app = app_with_parts();
        let r = app.sc_cut_weed_compute();
        let n = r.cuts.len();
        assert_eq!(app.sc_cut_weed_commit(r.cuts), n);
        assert_eq!(app.engine.undo_label(), Some("Weeding Lines"));
    }

    #[test]
    fn bridges_window_draws_both_counts_and_previews() {
        let mut app = app_with_parts();
        for by_distance in [false, true] {
            app.sc_cut.bridge_by_distance = by_distance;
            super::super::draw_headless(&mut app, ToolId::Bridges);
        }
        let rev = app.engine.revision();
        let basis = app.sc_cut_bridges_basis();
        let r = app.sc_cut_bridges_compute();
        assert_eq!(r.changes.len(), 3);
        app.sc_cut.win.bridges = Some((basis, r));
        super::super::draw_headless(&mut app, ToolId::Bridges);
        assert_eq!(app.engine.revision(), rev, "Apply changes nothing");
        let r = app.sc_cut.win.bridges.take().expect("kept").1;
        assert!(app.sc_cut_bridges_commit(&r) > 0);
        assert_eq!(app.engine.undo_label(), Some("Bridges"));
    }

    #[test]
    fn cut_order_window_shows_the_planned_and_the_drawn_route() {
        let mut app = app_with_parts();
        super::super::draw_headless(&mut app, ToolId::CutOrder);
        let rev = app.engine.revision();
        let basis = app.sc_cut_route_basis();
        apply_route(&mut app, basis);
        assert_eq!(app.engine.revision(), rev, "Apply changes nothing");
        let r = app.sc_cut.win.route.as_ref().expect("planned").1.clone();
        assert!(r.after < r.before, "{} {}", r.before, r.after);
        assert_eq!(first_start(&r), Some(Point::new(0.0, 10.0)));
        app.sc_cut.show_route = true;
        for original in [false, true] {
            app.sc_cut.win.route_original = original;
            super::super::draw_headless(&mut app, ToolId::CutOrder);
        }
        // OK reorders the objects in one step.
        assert!(app.sc_cut_order().is_some());
        assert_eq!(app.engine.undo_label(), Some("Cut Order"));
    }
}
