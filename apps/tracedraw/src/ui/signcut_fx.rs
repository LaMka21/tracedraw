//! The Sign & Cut panel's Effects and Patterns section: hatching, grids
//! and lines, puzzle, circle fill, cast shadow and random copies. See
//! docs/behavior/sign-and-cut-fx.md.

use crate::app::{App, Units};
use crate::i18n::tr;
use crate::ui::field::NumField;
use crate::ui::signcut_docker::{dim, length_field};
use egui::Ui;
use tracedraw_signcut::fx::{self, HatchStyle, ShadowKind, TabShape};

/// A labelled row.
fn row(ui: &mut Ui, key: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(tr(key));
        add(ui);
    });
}

fn count(ui: &mut Ui, v: &mut usize, max: usize) {
    ui.add(NumField::new(v).range(1..=max));
}

fn degrees(ui: &mut Ui, v: &mut f64, min: f64, max: f64) {
    ui.add(
        NumField::new(v)
            .range(min..=max)
            .max_decimals(2)
            .suffix("\u{b0}"),
    );
}

/// A fraction shown in per cent.
fn percent(ui: &mut Ui, v: &mut f64, min: f64, max: f64) {
    let mut p = *v * 100.0;
    if ui
        .add(
            NumField::new(&mut p)
                .range(min * 100.0..=max * 100.0)
                .max_decimals(1)
                .suffix(" %"),
        )
        .changed()
    {
        *v = p / 100.0;
    }
}

fn seed(ui: &mut Ui, v: &mut u64) {
    ui.add(NumField::new(v).range(0..=999_999_999u64));
    if ui
        .small_button(tr("signcut.fx_new_seed"))
        .on_hover_text(tr("signcut.fx_new_seed_hint"))
        .clicked()
    {
        *v = fx::Rng::new(*v).next_u64() % 1_000_000_000;
    }
}

fn apply(ui: &mut Ui, enabled: bool, key: &str) -> bool {
    ui.add_enabled(enabled, egui::Button::new(tr(key)))
        .clicked()
}

pub fn section(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let has = !app.selection.is_empty();
    dim(ui, tr("signcut.fx_hint"));
    hatch(app, ui, u, has);
    ui.separator();
    grid(app, ui, u);
    ui.separator();
    puzzle(app, ui, has);
    ui.separator();
    circles(app, ui, u, has);
    ui.separator();
    shadow(app, ui, u, has);
    ui.separator();
    copies(app, ui, u);
}

fn hatch(app: &mut App, ui: &mut Ui, u: Units, has: bool) {
    ui.strong(tr("signcut.fx_hatch"));
    let h = &mut app.sc_fx.hatch;
    row(ui, "signcut.fx_style", |ui| {
        for (s, key) in [
            (HatchStyle::Lines, "signcut.fx_style_lines"),
            (HatchStyle::Snake, "signcut.fx_style_snake"),
            (HatchStyle::Grid, "signcut.fx_style_grid"),
        ] {
            ui.radio_value(&mut h.style, s, tr(key));
        }
    });
    row(ui, "signcut.fx_angle", |ui| {
        degrees(ui, &mut h.angle, -360.0, 360.0)
    });
    row(ui, "signcut.fx_spacing", |ui| {
        length_field(ui, u, &mut h.spacing, fx::hatch::MIN_SPACING);
    });
    row(ui, "signcut.fx_inset", |ui| {
        length_field(ui, u, &mut h.inset, 0.0);
    });
    if apply(ui, has, "signcut.fx_hatch_apply") {
        app.fx_hatch();
    }
}

fn grid(app: &mut App, ui: &mut Ui, u: Units) {
    ui.strong(tr("signcut.fx_grid"));
    let s = &mut app.sc_fx;
    row(ui, "signcut.fx_columns", |ui| {
        count(ui, &mut s.grid.cols, fx::grid::MAX_SIDE)
    });
    row(ui, "signcut.fx_rows", |ui| {
        count(ui, &mut s.grid.rows, fx::grid::MAX_SIDE)
    });
    ui.checkbox(&mut s.grid_fill_selection, tr("signcut.fx_fill_selection"));
    if !s.grid_fill_selection {
        row(ui, "signcut.fx_cell_width", |ui| {
            length_field(ui, u, &mut s.grid.cell_w, 0.01);
        });
        row(ui, "signcut.fx_cell_height", |ui| {
            length_field(ui, u, &mut s.grid.cell_h, 0.01);
        });
        row(ui, "signcut.fx_origin", |ui| {
            length_field(ui, u, &mut s.grid_origin.x, -1_000_000.0);
            length_field(ui, u, &mut s.grid_origin.y, -1_000_000.0);
        });
    }
    ui.horizontal(|ui| {
        ui.radio_value(&mut s.grid_cells, false, tr("signcut.fx_as_lines"));
        ui.radio_value(&mut s.grid_cells, true, tr("signcut.fx_as_cells"));
    });
    ui.checkbox(&mut s.grid_preview, tr("signcut.fx_preview"));
    if s.grid_preview {
        if let Some((origin, spec)) = app.fx_grid_layout() {
            for l in fx::grid_lines(origin, &spec) {
                app.docker_preview.push(fx::polyline_path(&l));
            }
        }
    }
    if apply(ui, true, "signcut.fx_grid_apply") {
        app.fx_grid();
    }
    ui.add_space(4.0);
    ui.strong(tr("signcut.fx_lines"));
    dim(ui, tr("signcut.fx_lines_hint"));
    let s = &mut app.sc_fx;
    row(ui, "signcut.fx_count", |ui| {
        count(ui, &mut s.lines_count, fx::grid::MAX_LINES)
    });
    ui.horizontal(|ui| {
        ui.radio_value(&mut s.lines_vertical, false, tr("signcut.fx_horizontal"));
        ui.radio_value(&mut s.lines_vertical, true, tr("signcut.fx_vertical"));
    });
    if apply(ui, true, "signcut.fx_lines_apply") {
        app.fx_lines();
    }
}

fn puzzle(app: &mut App, ui: &mut Ui, has: bool) {
    ui.strong(tr("signcut.fx_puzzle"));
    let s = &mut app.sc_fx;
    let p = &mut s.puzzle;
    row(ui, "signcut.fx_columns", |ui| {
        count(ui, &mut p.cols, fx::puzzle::MAX_SIDE)
    });
    row(ui, "signcut.fx_rows", |ui| {
        count(ui, &mut p.rows, fx::puzzle::MAX_SIDE)
    });
    row(ui, "signcut.fx_tab_size", |ui| {
        percent(ui, &mut p.tab, 0.0, fx::puzzle::MAX_TAB)
    });
    row(ui, "signcut.fx_randomness", |ui| {
        percent(ui, &mut p.randomness, 0.0, 1.0)
    });
    row(ui, "signcut.fx_tab_shape", |ui| {
        ui.radio_value(
            &mut p.shape,
            TabShape::Classic,
            tr("signcut.fx_tab_classic"),
        );
        ui.radio_value(&mut p.shape, TabShape::Wave, tr("signcut.fx_tab_wave"));
    });
    row(ui, "signcut.fx_seed", |ui| seed(ui, &mut p.seed));
    ui.radio_value(&mut s.puzzle_pieces, false, tr("signcut.fx_cut_lines"));
    ui.radio_value(&mut s.puzzle_pieces, true, tr("signcut.fx_pieces"));
    if apply(ui, has, "signcut.fx_puzzle_apply") {
        app.fx_puzzle();
    }
}

fn circles(app: &mut App, ui: &mut Ui, u: Units, has: bool) {
    ui.strong(tr("signcut.fx_circles"));
    let c = &mut app.sc_fx.circles;
    row(ui, "signcut.fx_min_radius", |ui| {
        length_field(ui, u, &mut c.r_min, 0.01);
    });
    row(ui, "signcut.fx_max_radius", |ui| {
        length_field(ui, u, &mut c.r_max, 0.01);
    });
    row(ui, "signcut.fx_gap", |ui| {
        length_field(ui, u, &mut c.gap, 0.0);
    });
    row(ui, "signcut.fx_max_count", |ui| {
        count(ui, &mut c.count, fx::circles::MAX_COUNT)
    });
    row(ui, "signcut.fx_seed", |ui| seed(ui, &mut c.seed));
    if apply(ui, has, "signcut.fx_circles_apply") {
        app.fx_circle_fill();
    }
}

fn shadow(app: &mut App, ui: &mut Ui, u: Units, has: bool) {
    ui.strong(tr("signcut.fx_shadow"));
    let s = &mut app.sc_fx;
    let sh = &mut s.shadow;
    ui.horizontal(|ui| {
        ui.radio_value(&mut sh.kind, ShadowKind::Wall, tr("signcut.fx_on_wall"));
        ui.radio_value(&mut sh.kind, ShadowKind::Floor, tr("signcut.fx_on_floor"));
    });
    match sh.kind {
        ShadowKind::Wall => {
            row(ui, "signcut.fx_direction", |ui| {
                degrees(ui, &mut sh.angle, -360.0, 360.0)
            });
            row(ui, "signcut.fx_distance", |ui| {
                length_field(ui, u, &mut sh.distance, 0.0);
            });
        }
        ShadowKind::Floor => {
            row(ui, "signcut.fx_shadow_height", |ui| {
                percent(
                    ui,
                    &mut sh.scale,
                    fx::shadow::MIN_SCALE,
                    fx::shadow::MAX_SCALE,
                )
            });
            row(ui, "signcut.fx_slant", |ui| {
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
    row(ui, "signcut.fx_colour", |ui| {
        ui.color_edit_button_srgb(&mut s.shadow_color);
    });
    row(ui, "signcut.fx_opacity", |ui| {
        percent(ui, &mut s.shadow_opacity, 0.0, 1.0)
    });
    if apply(ui, has, "signcut.fx_shadow_apply") {
        app.fx_cast_shadow();
    }
}

fn copies(app: &mut App, ui: &mut Ui, u: Units) {
    ui.strong(tr("signcut.fx_copies"));
    dim(ui, tr("signcut.fx_copies_hint"));
    let one = app.selection.len() == 1;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(one, egui::Button::new(tr("signcut.fx_pick_container")))
            .clicked()
        {
            app.fx_pick_container();
        }
        let set = app.fx_container().is_some();
        if ui
            .add_enabled(set, egui::Button::new(tr("signcut.fx_clear_container")))
            .clicked()
        {
            app.sc_fx.container = None;
        }
    });
    let container = app.fx_container();
    app.sc_fx.container = container;
    dim(
        ui,
        tr(if container.is_some() {
            "signcut.fx_container_set"
        } else {
            "signcut.fx_no_container"
        }),
    );
    let s = &mut app.sc_fx.scatter;
    row(ui, "signcut.fx_count", |ui| {
        count(ui, &mut s.count, fx::scatter::MAX_COUNT)
    });
    row(ui, "signcut.fx_rotation", |ui| {
        degrees(ui, &mut s.rot_min, -360.0, 360.0);
        ui.label("\u{2026}");
        degrees(ui, &mut s.rot_max, -360.0, 360.0);
    });
    row(ui, "signcut.fx_scale", |ui| {
        percent(ui, &mut s.scale_min, fx::scatter::MIN_SCALE, 100.0);
        ui.label("\u{2026}");
        percent(ui, &mut s.scale_max, fx::scatter::MIN_SCALE, 100.0);
    });
    row(ui, "signcut.fx_min_distance", |ui| {
        length_field(ui, u, &mut s.min_dist, 0.0);
    });
    row(ui, "signcut.fx_seed", |ui| seed(ui, &mut s.seed));
    let can = app.selection.iter().any(|id| Some(*id) != container);
    if apply(ui, can, "signcut.fx_copies_apply") {
        app.fx_random_copies();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn draw(app: &mut App) {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 2000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| section(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_section_draws_in_every_mode_and_previews_the_grid() {
        let mut app = App::headless();
        let r = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("layer");
        app.select(vec![r]);
        draw(&mut app);
        assert!(app.docker_preview.is_empty());
        app.sc_fx.grid_preview = true;
        app.sc_fx.shadow.kind = ShadowKind::Floor;
        app.sc_fx.grid_fill_selection = true;
        app.sc_fx.container = Some(r);
        draw(&mut app);
        let spec = app.sc_fx.grid;
        assert_eq!(app.docker_preview.len(), spec.cols + spec.rows + 2);
        assert_eq!(app.sc_fx.container, Some(r));
    }
}
