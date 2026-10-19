//! The Sign & Cut tool windows of effects and patterns: Effects (puzzle,
//! circle fill, cast shadow, random copies), Hatching Lines and Grid
//! (grid or dividing lines). Each has a preview of the result at the left
//! with the yellow status strip, the settings at the right (preset row,
//! method list with pictures, the method's settings, units), Show
//! Original, Apply (work the result out into the preview), OK (add it to
//! the drawing in one undo step) and Cancel. See
//! docs/behavior/sign-and-cut-fx.md.

mod picker;

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_fx::{Effect, GridMethod, Op, WindowState};
use crate::signcut_tools::ToolId;
use crate::ui::field::NumField;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::{Align, Color32, Layout, Response, Ui, Vec2};
use tracedraw_core::geometry::{BezPath, Rect, Shape as _};
use tracedraw_signcut::fx::{self, HatchStyle, ShadowKind, TabShape};

#[cfg(test)]
use picker::popup_id;

/// The tools whose windows this file draws.
pub const TOOLS: &[ToolId] = &[ToolId::Effects, ToolId::Hatching, ToolId::Grid];

/// A window's default size and whether it can be resized.
pub fn size(t: ToolId) -> (Vec2, bool) {
    match t {
        ToolId::Effects => (Vec2::new(920.0, 680.0), true),
        _ => (Vec2::new(860.0, 600.0), true),
    }
}

/// The operation the window of `t` runs with its current method.
fn op_of(app: &App, t: ToolId) -> Option<Op> {
    match t {
        ToolId::Effects => Some(app.sc_fx.effects.method.op()),
        ToolId::Hatching => Some(Op::Hatch),
        ToolId::Grid => Some(app.sc_fx.grid.method.op()),
        _ => None,
    }
}

fn state(app: &App, t: ToolId) -> &WindowState {
    match t {
        ToolId::Hatching => &app.sc_fx.win_hatch,
        ToolId::Grid => &app.sc_fx.win_grid,
        _ => &app.sc_fx.win_effects,
    }
}

fn state_mut(app: &mut App, t: ToolId) -> &mut WindowState {
    match t {
        ToolId::Hatching => &mut app.sc_fx.win_hatch,
        ToolId::Grid => &mut app.sc_fx.win_grid,
        _ => &mut app.sc_fx.win_effects,
    }
}

/// The window's result is current: worked out for this method from the
/// settings, selection and drawing as they are now.
fn is_fresh(app: &App, t: ToolId) -> bool {
    let Some(op) = op_of(app, t) else {
        return false;
    };
    let inputs = app.fx_inputs(op);
    state(app, t)
        .computed
        .as_ref()
        .is_some_and(|c| c.op == op && c.inputs == inputs)
}

/// The window's body. Returns true when it should close.
pub fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    if op_of(app, t).is_none() {
        return true;
    }
    // A result worked out from other settings, objects or drawing is gone.
    if !is_fresh(app, t) {
        state_mut(app, t).computed = None;
    }
    let height = (ui.available_height() - 60.0).max(380.0);
    ui.horizontal_top(|ui| {
        let left_w = (ui.available_width() - kit::SIDE_W - 12.0).max(300.0);
        ui.vertical(|ui| {
            ui.set_width(left_w);
            let (scene, strip) = scene(app, t);
            kit::preview(ui, Vec2::new(left_w, height - 22.0), &scene);
            kit::status_strip(ui, left_w, &strip);
        });
        ui.vertical(|ui| {
            ui.set_width(kit::SIDE_W);
            settings(app, ui, t, height);
        });
    });
    let fresh = is_fresh(app, t);
    kit::progress(ui, if fresh { 1.0 } else { 0.0 });
    let Some(op) = op_of(app, t) else {
        return true;
    };
    let can = !op.needs_selection() || !app.fx_sources(op).is_empty();
    let original = state(app, t).original;
    let pressed = kit::bottom(ui, Buttons::ApplyOkCancel, can, |ui| {
        if ui
            .add(egui::Button::new(tr("signcut.nw_show_original")).selected(original))
            .clicked()
        {
            let w = state_mut(app, t);
            w.original = !w.original;
        }
    });
    // The result also shows on the drawing while it is current.
    if fresh && !original {
        if let Some(c) = &state(app, t).computed {
            let outlines = c.outlines();
            app.docker_preview.extend(outlines);
        }
    }
    match pressed {
        Some(Pressed::Apply) => {
            apply(app, t);
            false
        }
        Some(Pressed::Ok) => {
            ok(app, t);
            true
        }
        Some(Pressed::Cancel) | Some(Pressed::Close) => {
            state_mut(app, t).computed = None;
            true
        }
        _ => false,
    }
}

/// Apply: work the result out into the preview; the drawing does not
/// change. Returns true when there is a result.
pub fn apply(app: &mut App, t: ToolId) -> bool {
    let Some(op) = op_of(app, t) else {
        return false;
    };
    let c = app.fx_compute(op);
    if let Some(c) = &c {
        app.status = trf("signcut.wfx_strip_result", &[("what", &c.describe())]);
    }
    let w = state_mut(app, t);
    let has = c.is_some();
    w.computed = c;
    w.original = false;
    has
}

/// OK: add the previewed result when it is still current, else work it
/// out and add it, in one undo step. Returns the count added.
pub fn ok(app: &mut App, t: ToolId) -> usize {
    let Some(op) = op_of(app, t) else {
        return 0;
    };
    let fresh = if is_fresh(app, t) {
        state_mut(app, t).computed.take()
    } else {
        None
    };
    state_mut(app, t).computed = None;
    match fresh.or_else(|| app.fx_compute(op)) {
        Some(c) => app.fx_commit(&c),
        None => 0,
    }
}

/// The page-space outline of a rectangle.
fn rect_path(r: Rect) -> BezPath {
    r.to_path(0.1)
}

/// The preview (the objects as they are, and the result when it is
/// current) and the status strip's text.
fn scene(app: &App, t: ToolId) -> (Scene, String) {
    let mut scene = Scene::default();
    let Some(op) = op_of(app, t) else {
        return (scene, String::new());
    };
    let doc = app.doc();
    let sources = app.fx_sources(op);
    let w = state(app, t);
    let result = w.computed.as_ref().filter(|_| !w.original);
    // What the result goes into or onto.
    match op {
        Op::Copies => match app.fx_container().and_then(|c| doc.shape_on_page(c)) {
            Some(c) => {
                for o in tracedraw_signcut::outlines(&c) {
                    scene.add(o, colors::SHEET, 1.0);
                }
            }
            None => scene.add(rect_path(app.page_rect()), colors::SHEET, 1.0),
        },
        Op::Grid => {
            let fills = app.sc_fx.grid.fill_selection && !app.selection.is_empty();
            if !fills {
                scene.add(rect_path(app.page_rect()), colors::SHEET, 1.0);
            }
        }
        Op::Lines if app.selection.is_empty() => {
            scene.add(rect_path(app.page_rect()), colors::SHEET, 1.0);
        }
        _ => {}
    }
    // A shadow goes under its object.
    if let (Op::Shadow, Some(c)) = (op, result) {
        let [r, g, b] = app.sc_fx.effects.shadow_color;
        for o in c.outlines() {
            scene.add(o, Color32::from_rgb(r, g, b), 1.5);
        }
    }
    let shown: Vec<_> = if op.needs_selection() {
        sources.clone()
    } else {
        app.selection.clone()
    };
    for id in &shown {
        if let Some(s) = doc.shape_on_page(*id) {
            for o in tracedraw_signcut::outlines(&s) {
                scene.add(o, colors::ORIGINAL, 1.0);
            }
        }
    }
    if let Some(c) = result {
        if op != Op::Shadow {
            for o in c.outlines() {
                scene.add(o, colors::PART, 1.0);
            }
        }
    }
    let strip = if op.needs_selection() && sources.is_empty() {
        tr("signcut.fx_select_first")
    } else if let Some(c) = result {
        trf("signcut.wfx_strip_result", &[("what", &c.describe())])
    } else if op.needs_selection() {
        trf(
            "signcut.wfx_strip_ready",
            &[("n", &sources.len().to_string())],
        )
    } else {
        tr("signcut.wfx_strip_page")
    };
    (scene, strip)
}

/// The settings column: preset row, method list, the method's settings
/// and the units.
fn settings(app: &mut App, ui: &mut Ui, t: ToolId, height: f32) {
    match t {
        ToolId::Effects => {
            kit::preset_row(ui, &mut app.sc_presets, "effects", &mut app.sc_fx.effects);
            ui.add_space(4.0);
            let items: Vec<(Effect, String, picker::Picture)> = vec![
                (Effect::Puzzle, tr("signcut.fx_puzzle"), picker::puzzle),
                (Effect::Circles, tr("signcut.fx_circles"), picker::circles),
                (Effect::Shadow, tr("signcut.fx_shadow"), picker::shadow),
                (Effect::Copies, tr("signcut.fx_copies"), picker::copies),
            ];
            picker::method_picker(ui, "effects", &mut app.sc_fx.effects.method, &items);
        }
        ToolId::Hatching => {
            kit::preset_row(ui, &mut app.sc_presets, "hatching", &mut app.sc_fx.hatch);
            ui.add_space(4.0);
            let items: Vec<(HatchStyle, String, picker::Picture)> = vec![
                (
                    HatchStyle::Lines,
                    tr("signcut.wfx_parallel"),
                    picker::parallel,
                ),
                (
                    HatchStyle::Snake,
                    tr("signcut.fx_style_snake"),
                    picker::snake,
                ),
                (
                    HatchStyle::Grid,
                    tr("signcut.fx_style_grid"),
                    picker::crossed,
                ),
            ];
            picker::method_picker(ui, "hatching", &mut app.sc_fx.hatch.style, &items);
        }
        _ => {
            kit::preset_row(ui, &mut app.sc_presets, "grid", &mut app.sc_fx.grid);
            ui.add_space(4.0);
            let items: Vec<(GridMethod, String, picker::Picture)> = vec![
                (GridMethod::Grid, tr("signcut.fx_grid"), picker::grid),
                (
                    GridMethod::Lines,
                    tr("signcut.wfx_dividing"),
                    picker::dividing,
                ),
            ];
            picker::method_picker(ui, "grid", &mut app.sc_fx.grid.method, &items);
        }
    }
    egui::ScrollArea::vertical()
        .id_salt(("fx_settings", t.key()))
        .max_height((height - 170.0).max(120.0))
        .show(ui, |ui| match t {
            ToolId::Effects => match app.sc_fx.effects.method {
                Effect::Puzzle => puzzle(app, ui),
                Effect::Circles => circles(app, ui),
                Effect::Shadow => shadow(app, ui),
                Effect::Copies => copies(app, ui),
            },
            ToolId::Hatching => hatching(app, ui),
            _ => match app.sc_fx.grid.method {
                GridMethod::Grid => grid(app, ui),
                GridMethod::Lines => dividing(app, ui),
            },
        });
    ui.add_space(6.0);
    kit::units_row(ui, &mut app.units);
}

/// An angle field in degrees of the row width.
fn degrees(ui: &mut Ui, v: &mut f64, min: f64, max: f64) -> Response {
    ui.add_sized(
        [kit::FIELD_W, 22.0],
        NumField::new(v)
            .range(min..=max)
            .max_decimals(2)
            .suffix("\u{b0}"),
    )
}

/// A fraction shown in per cent, of the row width.
fn percent(ui: &mut Ui, v: &mut f64, min: f64, max: f64) -> Response {
    let mut p = *v * 100.0;
    let r = ui.add_sized(
        [kit::FIELD_W, 22.0],
        NumField::new(&mut p)
            .range(min * 100.0..=max * 100.0)
            .max_decimals(1)
            .suffix(" %"),
    );
    if r.changed() && p.is_finite() {
        *v = (p / 100.0).clamp(min, max);
    }
    r
}

/// The seed row and, under it at the right, the New Seed button.
fn seed(ui: &mut Ui, v: &mut u64) {
    kit::row(ui, &tr("signcut.nw_seed"), |ui| {
        ui.add_sized(
            [kit::FIELD_W, 22.0],
            NumField::new(v).range(0..=999_999_999u64),
        )
    });
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button(tr("signcut.wfx_new_seed"))
                .on_hover_text(tr("signcut.fx_new_seed_hint"))
                .clicked()
            {
                *v = fx::Rng::new(*v).next_u64() % 1_000_000_000;
            }
        });
    });
}

fn puzzle(app: &mut App, ui: &mut Ui) {
    let e = &mut app.sc_fx.effects;
    let p = &mut e.puzzle;
    kit::group(ui, &tr("signcut.wfx_g_pieces"), |ui| {
        kit::row(ui, &tr("signcut.wfx_columns"), |ui| {
            kit::count(ui, &mut p.cols, 1, fx::puzzle::MAX_SIDE)
        });
        kit::row(ui, &tr("signcut.wfx_rows"), |ui| {
            kit::count(ui, &mut p.rows, 1, fx::puzzle::MAX_SIDE)
        });
    });
    kit::group(ui, &tr("signcut.wfx_g_tabs"), |ui| {
        kit::row(ui, &tr("signcut.wfx_tab_shape"), |ui| {
            let opts = [
                (TabShape::Classic, tr("signcut.fx_tab_classic")),
                (TabShape::Wave, tr("signcut.fx_tab_wave")),
            ];
            kit::choice(ui, "fx_tab_shape", &mut p.shape, &opts)
        });
        kit::row(ui, &tr("signcut.wfx_tab_size"), |ui| {
            percent(ui, &mut p.tab, 0.0, fx::puzzle::MAX_TAB)
        });
        kit::row(ui, &tr("signcut.wfx_randomness"), |ui| {
            percent(ui, &mut p.randomness, 0.0, 1.0)
        });
        kit::separator(ui);
        seed(ui, &mut p.seed);
    });
    kit::group(ui, &tr("signcut.wfx_g_result"), |ui| {
        ui.radio_value(&mut e.puzzle_pieces, false, tr("signcut.fx_cut_lines"));
        ui.radio_value(&mut e.puzzle_pieces, true, tr("signcut.fx_pieces"));
    });
}

fn circles(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let c = &mut app.sc_fx.effects.circles;
    kit::group(ui, &tr("signcut.wfx_g_circles"), |ui| {
        kit::row(ui, &tr("signcut.wfx_min_radius"), |ui| {
            kit::length(ui, u, &mut c.r_min, 0.01, 100_000.0)
        });
        kit::row(ui, &tr("signcut.wfx_max_radius"), |ui| {
            kit::length(ui, u, &mut c.r_max, 0.01, 100_000.0)
        });
        kit::row(ui, &tr("signcut.nw_gap"), |ui| {
            kit::length(ui, u, &mut c.gap, 0.0, 10_000.0)
        });
        kit::row(ui, &tr("signcut.wfx_max_count"), |ui| {
            kit::count(ui, &mut c.count, 1, fx::circles::MAX_COUNT)
        });
        kit::separator(ui);
        seed(ui, &mut c.seed);
    });
}

fn shadow(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let e = &mut app.sc_fx.effects;
    let sh = &mut e.shadow;
    kit::group(ui, &tr("signcut.wfx_g_shadow"), |ui| {
        kit::row(ui, &tr("signcut.wfx_falls"), |ui| {
            let opts = [
                (ShadowKind::Wall, tr("signcut.fx_on_wall")),
                (ShadowKind::Floor, tr("signcut.fx_on_floor")),
            ];
            kit::choice(ui, "fx_shadow_kind", &mut sh.kind, &opts)
        });
        kit::separator(ui);
        match sh.kind {
            ShadowKind::Wall => {
                kit::row(ui, &tr("signcut.wfx_direction"), |ui| {
                    degrees(ui, &mut sh.angle, -360.0, 360.0)
                });
                kit::row(ui, &tr("signcut.wfx_distance"), |ui| {
                    kit::length(ui, u, &mut sh.distance, 0.0, 100_000.0)
                });
            }
            ShadowKind::Floor => {
                kit::row(ui, &tr("signcut.wfx_height"), |ui| {
                    percent(
                        ui,
                        &mut sh.scale,
                        fx::shadow::MIN_SCALE,
                        fx::shadow::MAX_SCALE,
                    )
                });
                kit::row(ui, &tr("signcut.wfx_slant"), |ui| {
                    degrees(
                        ui,
                        &mut sh.skew,
                        -fx::shadow::MAX_SKEW,
                        fx::shadow::MAX_SKEW,
                    )
                });
                ui.checkbox(&mut sh.in_front, tr("signcut.fx_in_front"));
            }
        }
    });
    kit::group(ui, &tr("signcut.wfx_g_fill"), |ui| {
        kit::row(ui, &tr("signcut.wfx_colour"), |ui| {
            ui.color_edit_button_srgb(&mut e.shadow_color)
        });
        kit::row(ui, &tr("signcut.wfx_opacity"), |ui| {
            percent(ui, &mut e.shadow_opacity, 0.0, 1.0)
        });
    });
}

fn copies(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let one = app.selection.len() == 1;
    let container = app.fx_container();
    kit::group(ui, &tr("signcut.wfx_g_container"), |ui| {
        ui.label(tr(if container.is_some() {
            "signcut.fx_container_set"
        } else {
            "signcut.fx_no_container"
        }));
        if ui
            .add_enabled(one, egui::Button::new(tr("signcut.fx_pick_container")))
            .clicked()
        {
            app.fx_pick_container();
        }
        if ui
            .add_enabled(
                container.is_some(),
                egui::Button::new(tr("signcut.fx_clear_container")),
            )
            .clicked()
        {
            app.sc_fx.container = None;
        }
    });
    let s = &mut app.sc_fx.effects.scatter;
    kit::group(ui, &tr("signcut.wfx_g_copies"), |ui| {
        kit::row(ui, &tr("signcut.wfx_count"), |ui| {
            kit::count(ui, &mut s.count, 1, fx::scatter::MAX_COUNT)
        });
        kit::row(ui, &tr("signcut.wfx_turn_from"), |ui| {
            degrees(ui, &mut s.rot_min, -360.0, 360.0)
        });
        kit::row(ui, &tr("signcut.wfx_turn_to"), |ui| {
            degrees(ui, &mut s.rot_max, -360.0, 360.0)
        });
        kit::row(ui, &tr("signcut.wfx_scale_from"), |ui| {
            percent(ui, &mut s.scale_min, fx::scatter::MIN_SCALE, 100.0)
        });
        kit::row(ui, &tr("signcut.wfx_scale_to"), |ui| {
            percent(ui, &mut s.scale_max, fx::scatter::MIN_SCALE, 100.0)
        });
        kit::row(ui, &tr("signcut.nw_min_distance"), |ui| {
            kit::length(ui, u, &mut s.min_dist, 0.0, 100_000.0)
        });
        kit::separator(ui);
        seed(ui, &mut s.seed);
    });
}

fn hatching(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let h = &mut app.sc_fx.hatch;
    kit::group(ui, &tr("signcut.fx_style_lines"), |ui| {
        kit::row(ui, &tr("signcut.wfx_angle"), |ui| {
            degrees(ui, &mut h.angle, -360.0, 360.0)
        });
        kit::row(ui, &tr("signcut.wfx_spacing"), |ui| {
            kit::length(ui, u, &mut h.spacing, fx::hatch::MIN_SPACING, 100_000.0)
        });
        kit::row(ui, &tr("signcut.wfx_inset"), |ui| {
            kit::length(ui, u, &mut h.inset, 0.0, 100_000.0)
        });
    });
}

fn grid(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let g = &mut app.sc_fx.grid;
    kit::group(ui, &tr("signcut.fx_grid"), |ui| {
        kit::row(ui, &tr("signcut.wfx_columns"), |ui| {
            kit::count(ui, &mut g.grid.cols, 1, fx::grid::MAX_SIDE)
        });
        kit::row(ui, &tr("signcut.wfx_rows"), |ui| {
            kit::count(ui, &mut g.grid.rows, 1, fx::grid::MAX_SIDE)
        });
    });
    kit::group(ui, &tr("signcut.wfx_g_place"), |ui| {
        ui.checkbox(&mut g.fill_selection, tr("signcut.fx_fill_selection"));
        ui.add_enabled_ui(!g.fill_selection, |ui| {
            kit::row(ui, &tr("signcut.wfx_cell_width"), |ui| {
                kit::length(ui, u, &mut g.grid.cell_w, 0.01, 100_000.0)
            });
            kit::row(ui, &tr("signcut.wfx_cell_height"), |ui| {
                kit::length(ui, u, &mut g.grid.cell_h, 0.01, 100_000.0)
            });
            kit::row(ui, &tr("signcut.wfx_left"), |ui| {
                kit::length(ui, u, &mut g.origin.x, -1_000_000.0, 1_000_000.0)
            });
            kit::row(ui, &tr("signcut.wfx_bottom"), |ui| {
                kit::length(ui, u, &mut g.origin.y, -1_000_000.0, 1_000_000.0)
            });
        });
    });
    kit::group(ui, &tr("signcut.wfx_g_output"), |ui| {
        ui.radio_value(&mut g.cells, false, tr("signcut.fx_as_lines"));
        ui.radio_value(&mut g.cells, true, tr("signcut.fx_as_cells"));
    });
}

fn dividing(app: &mut App, ui: &mut Ui) {
    let g = &mut app.sc_fx.grid;
    kit::group(ui, &tr("signcut.wfx_dividing"), |ui| {
        kit::row(ui, &tr("signcut.wfx_count"), |ui| {
            kit::count(ui, &mut g.lines_count, 1, fx::grid::MAX_LINES)
        });
        kit::row(ui, &tr("signcut.wfx_direction"), |ui| {
            let opts = [
                (false, tr("signcut.fx_horizontal")),
                (true, tr("signcut.fx_vertical")),
            ];
            kit::choice(ui, "fx_lines_dir", &mut g.lines_vertical, &opts)
        });
        ui.add(egui::Label::new(tr("signcut.fx_lines_hint")).wrap());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeKind;

    fn app_with_rect() -> App {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(20.0, 20.0, 120.0, 90.0),
                radius: 0.0,
                corners: None,
            })
            .expect("layer");
        app.select(vec![id]);
        app
    }

    fn draw(app: &mut App, t: ToolId, popup: Option<&str>) {
        let ctx = crate::theme::ui_context();
        // Two frames: the first lays out, the second shows an opened list.
        for _ in 0..2 {
            if let Some(salt) = popup {
                egui::Popup::open_id(&ctx, popup_id(salt));
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let _ = body(app, ui, t);
            });
            out.textures_delta.clear();
        }
    }

    #[test]
    fn every_window_draws_every_method() {
        let mut app = app_with_rect();
        for m in Effect::ALL {
            app.sc_fx.effects.method = m;
            draw(&mut app, ToolId::Effects, None);
            assert!(apply(&mut app, ToolId::Effects), "{m:?}");
            draw(&mut app, ToolId::Effects, None);
        }
        app.sc_fx.effects.shadow.kind = ShadowKind::Floor;
        app.sc_fx.effects.method = Effect::Shadow;
        draw(&mut app, ToolId::Effects, Some("effects"));
        for s in [HatchStyle::Lines, HatchStyle::Snake, HatchStyle::Grid] {
            app.sc_fx.hatch.style = s;
            draw(&mut app, ToolId::Hatching, None);
            assert!(apply(&mut app, ToolId::Hatching), "{s:?}");
            draw(&mut app, ToolId::Hatching, None);
        }
        draw(&mut app, ToolId::Hatching, Some("hatching"));
        for m in [GridMethod::Grid, GridMethod::Lines] {
            app.sc_fx.grid.method = m;
            draw(&mut app, ToolId::Grid, None);
            assert!(apply(&mut app, ToolId::Grid), "{m:?}");
            draw(&mut app, ToolId::Grid, None);
        }
        draw(&mut app, ToolId::Grid, Some("grid"));
        // With nothing selected the effects say so and the grid still works.
        app.select(vec![]);
        draw(&mut app, ToolId::Effects, None);
        let (_, strip) = scene(&app, ToolId::Effects);
        assert_eq!(strip, tr("signcut.fx_select_first"));
        draw(&mut app, ToolId::Grid, None);
    }

    #[test]
    fn apply_previews_and_changes_nothing() {
        let mut app = app_with_rect();
        let rev = app.engine.revision();
        let (before, _) = scene(&app, ToolId::Effects);
        assert!(apply(&mut app, ToolId::Effects));
        assert_eq!(app.engine.revision(), rev);
        assert!(is_fresh(&app, ToolId::Effects));
        let (after, strip) = scene(&app, ToolId::Effects);
        // The default puzzle: five cut lines over the rectangle.
        assert_eq!(after.items.len(), before.items.len() + 5);
        assert!(strip.contains('5'), "{strip}");
        // Drawn while current, also on the drawing.
        let ctx = crate::theme::ui_context();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let _ = body(&mut app, ui, ToolId::Effects);
        });
        out.textures_delta.clear();
        assert_eq!(app.docker_preview.len(), 5);
        // Another setting makes it out of date.
        app.sc_fx.effects.puzzle.rows = 5;
        assert!(!is_fresh(&app, ToolId::Effects));
        app.docker_preview.clear();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let _ = body(&mut app, ui, ToolId::Effects);
        });
        out.textures_delta.clear();
        assert!(app.sc_fx.win_effects.computed.is_none());
        let (stale, _) = scene(&app, ToolId::Effects);
        assert_eq!(stale.items.len(), before.items.len());
        assert!(app.docker_preview.is_empty());
    }

    #[test]
    fn ok_adds_the_previewed_result_in_one_step() {
        let mut app = app_with_rect();
        for (t, label) in [
            (ToolId::Effects, "Puzzle"),
            (ToolId::Hatching, "Hatching"),
            (ToolId::Grid, "Grid"),
        ] {
            let target = app.selection.clone();
            assert!(apply(&mut app, t));
            let steps = app.engine.history_labels().0.len();
            assert!(ok(&mut app, t) > 0, "{t:?}");
            assert_eq!(app.engine.history_labels().0.len(), steps + 1);
            assert_eq!(app.engine.undo_label(), Some(label));
            assert!(state(&app, t).computed.is_none());
            app.select(target);
        }
        // OK without Apply works it out first.
        app.sc_fx.effects.method = Effect::Circles;
        let steps = app.engine.history_labels().0.len();
        assert!(ok(&mut app, ToolId::Effects) > 0);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        assert_eq!(app.engine.undo_label(), Some("Circle Fill"));
    }
}
