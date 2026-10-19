//! The Plotter Output window: a preview of the job as the plotter cuts it
//! at the left (the outlines in cut order, their start points and
//! directions, the moves with the knife up, the weeding frame and the
//! plotter's origin), the settings at the right (the preset row holds the
//! plotter profiles; tabs Job, Plotter and Knife), Save to file, Send and
//! Close at the bottom.

use super::{add_moves, col, columns, dot, mark_size, note, rect_path, text_field};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_cut::{PlotResult, PlotTab};
use crate::ui::signcut_docker::fmt_len;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::Ui;
use tracedraw_core::geometry::{BezPath, Point, Vec2};
use tracedraw_signcut::cut::hpgl::{Origin, MAX_PASSES};

/// The window. Returns true to close it.
pub fn body(app: &mut App, ui: &mut Ui) -> bool {
    let basis = app.sc_cut_plot_basis();
    if !matches!(&app.sc_cut.win.plot, Some((b, _)) if *b == basis) {
        let r = app.sc_cut_plot_compute();
        app.sc_cut.win.plot = Some((basis, r));
    }
    let u = app.units;
    let (scene, strip, has) = match &app.sc_cut.win.plot {
        Some((_, r)) => (scene(r, u), strip(r, u), r.summary.outlines > 0),
        None => (Scene::default(), tr("signcut.cut_nothing"), false),
    };
    columns(app, ui, "plotter", &scene, &strip, settings);
    let (mut save, mut send) = (false, false);
    let can_send = !app.sc_cut.plotter.port.trim().is_empty();
    let pressed = kit::bottom(ui, Buttons::Close, true, |ui| {
        save = kit::push(ui, &tr("signcut.cut_save"), has);
        if cfg!(not(target_arch = "wasm32")) {
            send = kit::push(ui, &tr("signcut.cut_send"), has && can_send);
        }
    });
    if save {
        app.sc_cut_save_plt();
    }
    #[cfg(not(target_arch = "wasm32"))]
    if send {
        let _ = app.sc_cut_send();
    }
    #[cfg(target_arch = "wasm32")]
    let _ = send;
    if matches!(pressed, Some(Pressed::Close)) {
        app.sc_cut.win.plot = None;
        return true;
    }
    false
}

/// A cut of the layout as a path.
fn poly_path(pts: &[Point], closed: bool) -> BezPath {
    let mut p = BezPath::new();
    for (i, q) in pts.iter().enumerate() {
        if i == 0 {
            p.move_to(*q);
        } else {
            p.line_to(*q);
        }
    }
    if closed && pts.len() > 2 {
        p.close_path();
    }
    p
}

/// The preview of the job, turned and mirrored as the plotter cuts it.
fn scene(r: &PlotResult, u: crate::app::Units) -> Scene {
    let mut scene = Scene::default();
    let job = &r.layout;
    let Some(bbox) = job.bbox else {
        return scene;
    };
    let size = mark_size(Some(bbox));
    let grow = bbox.width().max(bbox.height()) * 0.08;
    scene.extent = Some(bbox.inflate(grow, grow));
    let cuts = if job.frame.is_some() {
        &job.cuts[..job.cuts.len().saturating_sub(1)]
    } else {
        &job.cuts[..]
    };
    if let Some(f) = job.frame {
        scene.add(rect_path(f), col::WEED, 1.5);
    }
    // The moves with the knife up, from the origin.
    let mut moves = Vec::new();
    let mut at = job.origin;
    for c in &job.cuts {
        let (Some(first), Some(last)) = (c.pts.first(), c.pts.last()) else {
            continue;
        };
        if (*first - at).hypot() > 1e-9 {
            moves.push((at, *first));
        }
        at = if c.closed { *first } else { *last };
    }
    for c in cuts {
        scene.add(poly_path(&c.pts, c.closed), colors::PART, 1.0);
    }
    add_moves(&mut scene, &moves);
    for c in cuts {
        let Some(first) = c.pts.first() else { continue };
        dot(&mut scene, *first, size, col::START);
        let dir = c.pts.iter().skip(1).find_map(|q| {
            let v = *q - *first;
            let l = v.hypot();
            (l > 1e-9).then(|| v / l)
        });
        if let Some(d) = dir {
            super::arrow(&mut scene, *first, d, size, col::START);
        }
    }
    origin_mark(&mut scene, job.origin, job.corner, grow);
    scene.captions.push(format!(
        "{} x {}",
        fmt_len(u, r.summary.width),
        fmt_len(u, r.summary.height)
    ));
    scene
}

/// The plotter's 0,0: a dot and its two axes pointing into the job.
fn origin_mark(scene: &mut Scene, o: Point, corner: Origin, len: f64) {
    let sx = match corner {
        Origin::BottomLeft | Origin::TopLeft => 1.0,
        Origin::BottomRight | Origin::TopRight => -1.0,
    };
    let sy = match corner {
        Origin::BottomLeft | Origin::BottomRight => 1.0,
        Origin::TopLeft | Origin::TopRight => -1.0,
    };
    for d in [Vec2::new(sx, 0.0), Vec2::new(0.0, sy)] {
        let mut p = BezPath::new();
        p.move_to(o);
        p.line_to(o + d * len);
        scene.add(p, col::ORIGIN, 2.0);
        super::arrow(scene, o + d * (len * 0.8), d, len * 0.06, col::ORIGIN);
    }
    dot(scene, o, len * 0.08, col::ORIGIN);
}

fn strip(r: &PlotResult, u: crate::app::Units) -> String {
    if r.summary.outlines == 0 {
        return tr("signcut.cut_nothing");
    }
    trf(
        "signcut.wcut_plot_strip",
        &[
            ("n", &r.summary.outlines.to_string()),
            ("len", &fmt_len(u, r.summary.cut_length)),
            ("travel", &fmt_len(u, r.summary.travel)),
        ],
    )
}

/// The settings column: the plotter profile presets and the tabs.
fn settings(app: &mut App, ui: &mut Ui) {
    kit::preset_row(
        ui,
        &mut app.sc_presets,
        "plotter_output",
        &mut app.sc_cut.plotter,
    );
    ui.add_space(4.0);
    let tab = &mut app.sc_cut.win.plot_tab;
    ui.horizontal(|ui| {
        for (t, key) in [
            (PlotTab::Job, "signcut.wcut_tab_job"),
            (PlotTab::Plotter, "signcut.wcut_tab_plotter"),
            (PlotTab::Knife, "signcut.wcut_tab_knife"),
        ] {
            if ui.selectable_label(*tab == t, tr(key)).clicked() {
                *tab = t;
            }
        }
    });
    match app.sc_cut.win.plot_tab {
        PlotTab::Job => job_tab(app, ui),
        PlotTab::Plotter => plotter_tab(app, ui),
        PlotTab::Knife => knife_tab(app, ui),
    }
    super::scope(app, ui);
}

fn origin_label(o: Origin) -> String {
    tr(match o {
        Origin::BottomLeft => "signcut.cut_origin_bl",
        Origin::BottomRight => "signcut.cut_origin_br",
        Origin::TopLeft => "signcut.cut_origin_tl",
        Origin::TopRight => "signcut.cut_origin_tr",
    })
}

fn job_tab(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let pl = &mut app.sc_cut.plotter;
    kit::group(ui, &tr("signcut.cut_order"), |ui| {
        ui.checkbox(&mut pl.optimize, tr("signcut.cut_plot_optimize"));
        note(ui, &tr("signcut.wcut_order_hint"));
    });
    let p = &mut pl.profile;
    kit::group(ui, &tr("signcut.wcut_placement"), |ui| {
        kit::row(ui, &tr("signcut.cut_origin"), |ui| {
            let opts: Vec<(Origin, String)> =
                Origin::ALL.iter().map(|o| (*o, origin_label(*o))).collect();
            kit::choice(ui, "wcut_plot_origin", &mut p.origin, &opts)
        });
        ui.checkbox(&mut p.mirror, tr("signcut.cut_mirror"));
        ui.checkbox(&mut p.rotate, tr("signcut.cut_rotate"));
    });
    kit::group(ui, &tr("signcut.wcut_frame"), |ui| {
        ui.checkbox(&mut p.weed_frame, tr("signcut.wcut_frame_add"));
        ui.add_enabled_ui(p.weed_frame, |ui| {
            kit::row(ui, &tr("signcut.nw_margin"), |ui| {
                kit::length(ui, u, &mut p.weed_margin, 0.0, 10_000.0)
            });
        });
    });
}

fn plotter_tab(app: &mut App, ui: &mut Ui) {
    let pl = &mut app.sc_cut.plotter;
    let p = &mut pl.profile;
    kit::group(ui, &tr("signcut.wcut_commands"), |ui| {
        kit::row(ui, &tr("signcut.cut_units"), |ui| {
            kit::number(ui, &mut p.units_per_mm, 1.0, 10_000.0, 3)
        });
        kit::row(ui, &tr("signcut.cut_pen_up"), |ui| {
            text_field(ui, &mut p.pen_up)
        });
        kit::row(ui, &tr("signcut.cut_pen_down"), |ui| {
            text_field(ui, &mut p.pen_down)
        });
        kit::row(ui, &tr("signcut.cut_init"), |ui| {
            text_field(ui, &mut p.init)
        });
        kit::row(ui, &tr("signcut.cut_end"), |ui| text_field(ui, &mut p.end));
        ui.checkbox(&mut p.line_breaks, tr("signcut.cut_line_breaks"));
    });
    kit::group(ui, &tr("signcut.wcut_speed_force"), |ui| {
        kit::row(ui, &tr("signcut.cut_velocity"), |ui| {
            kit::number(ui, &mut p.velocity, 0.0, 10_000.0, 2)
        });
        kit::row(ui, &tr("signcut.cut_force"), |ui| {
            kit::number(ui, &mut p.force, 0.0, 10_000.0, 2)
        });
    });
    if cfg!(not(target_arch = "wasm32")) {
        kit::group(ui, &tr("signcut.wcut_output"), |ui| {
            kit::row(ui, &tr("signcut.cut_port"), |ui| {
                text_field(ui, &mut pl.port)
            });
            note(ui, &tr("signcut.cut_port_hint"));
        });
    }
}

fn knife_tab(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let p = &mut app.sc_cut.plotter.profile;
    kit::group(ui, &tr("signcut.wcut_tab_knife"), |ui| {
        kit::row(ui, &tr("signcut.cut_knife_offset"), |ui| {
            kit::length(ui, u, &mut p.knife_offset, 0.0, 10.0)
        });
        kit::row(ui, &tr("signcut.wcut_corner_limit"), |ui| {
            kit::number(ui, &mut p.corner_limit, 0.0, 180.0, 1)
        });
        kit::row(ui, &tr("signcut.cut_overcut"), |ui| {
            kit::length(ui, u, &mut p.overcut, 0.0, 100.0)
        });
    });
    kit::group(ui, &tr("signcut.cut_passes"), |ui| {
        kit::row(ui, &tr("signcut.cut_passes"), |ui| {
            let mut n = p.passes as usize;
            let r = kit::count(ui, &mut n, 1, MAX_PASSES as usize);
            p.passes = n.clamp(1, MAX_PASSES as usize) as u32;
            r
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signcut_tools::ToolId;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn app_with_parts() -> App {
        let mut app = App::headless();
        for x in [100.0, 0.0] {
            app.new_shape(ShapeKind::Rect {
                rect: Rect::new(x, 0.0, x + 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("layer");
        }
        app
    }

    #[test]
    fn plotter_window_draws_every_tab() {
        let mut app = app_with_parts();
        let rev = app.engine.revision();
        for tab in [PlotTab::Job, PlotTab::Plotter, PlotTab::Knife] {
            app.sc_cut.win.plot_tab = tab;
            super::super::draw_headless(&mut app, ToolId::PlotterOutput);
        }
        assert_eq!(app.engine.revision(), rev);
        let (b, r) = app.sc_cut.win.plot.as_ref().expect("laid out");
        assert_eq!(*b, app.sc_cut_plot_basis());
        assert_eq!(r.summary.outlines, 2);
    }

    #[test]
    fn the_preview_shows_cuts_moves_starts_frame_and_origin() {
        let mut app = app_with_parts();
        app.sc_cut.plotter.profile.weed_frame = true;
        app.sc_cut.plotter.profile.origin = Origin::TopRight;
        let r = app.sc_cut_plot_compute();
        assert_eq!(r.layout.cuts.len(), 3);
        let s = scene(&r, app.units);
        // Frame, two cuts, three moves (to each cut and the frame), two
        // starts with arrows, the origin's two axes with arrows and dot.
        assert_eq!(s.items.len(), 1 + 2 + 3 + 4 + 5, "{}", s.items.len());
        assert_eq!(s.captions.len(), 1);
        assert!(!strip(&r, app.units).is_empty());
        // A changed profile setting lays the job out again.
        let basis = app.sc_cut_plot_basis();
        app.sc_cut.plotter.profile.mirror = true;
        assert_ne!(app.sc_cut_plot_basis(), basis);
    }

    #[test]
    fn nothing_to_cut_gives_an_empty_preview() {
        let app = App::headless();
        let r = app.sc_cut_plot_compute();
        assert!(scene(&r, app.units).items.is_empty());
        assert_eq!(strip(&r, app.units), tr("signcut.cut_nothing"));
    }
}
