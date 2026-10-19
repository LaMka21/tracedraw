//! The Sign & Cut panel's Nesting section: the material (sheets or a
//! roll), then Rectangles, Shapes or Fill Sheet. See
//! docs/behavior/sign-and-cut-nest.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_nest::{corner_key, nest_status, roll_dir_key, rotations_key, Tool};
use crate::ui::field::NumField;
use crate::ui::signcut_docker::{dim, fmt_len, length_field};
use egui::Ui;
use tracedraw_core::geometry::Shape as _;
use tracedraw_signcut::nest::shapes::Rotations;
use tracedraw_signcut::nest::{Corner, RollDir};

pub fn section(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.horizontal(|ui| {
        for (tool, key) in [
            (Tool::Rects, "signcut.nest_tool_rects"),
            (Tool::Shapes, "signcut.nest_tool_shapes"),
            (Tool::Fill, "signcut.nest_tool_fill"),
        ] {
            ui.selectable_value(&mut app.sc_nest.tool, tool, tr(key));
        }
    });
    let n = app.selection.len();
    dim(
        ui,
        if n == 0 {
            tr("signcut.nest_select")
        } else {
            trf("signcut.nest_selected", &[("n", &n.to_string())])
        },
    );
    ui.add_space(4.0);

    // The material.
    let tool = app.sc_nest.tool;
    let page = app.page_size();
    let s = &mut app.sc_nest;
    if tool != Tool::Fill {
        ui.horizontal(|ui| {
            ui.radio_value(&mut s.sheet.roll, false, tr("signcut.nest_sheets"));
            ui.radio_value(&mut s.sheet.roll, true, tr("signcut.nest_roll"));
        });
    }
    let roll = s.sheet.roll && tool != Tool::Fill;
    egui::Grid::new("signcut_nest_material")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            if roll {
                ui.label(tr("signcut.nest_roll_width"));
                length_field(ui, u, &mut s.sheet.width, 1.0);
                ui.end_row();
                ui.label(tr("signcut.nest_roll_dir"));
                egui::ComboBox::from_id_salt("signcut_nest_roll_dir")
                    .selected_text(tr(roll_dir_key(s.sheet.roll_dir)))
                    .show_ui(ui, |ui| {
                        for d in [RollDir::Right, RollDir::Up] {
                            ui.selectable_value(&mut s.sheet.roll_dir, d, tr(roll_dir_key(d)));
                        }
                    });
                ui.end_row();
            } else {
                ui.label("");
                ui.checkbox(&mut s.use_page, tr("signcut.nest_use_page"));
                ui.end_row();
                if s.use_page {
                    ui.label(tr("signcut.nest_sheet_size"));
                    ui.label(format!(
                        "{} \u{d7} {}",
                        fmt_len(u, page.width),
                        fmt_len(u, page.height)
                    ));
                    ui.end_row();
                } else {
                    ui.label(tr("signcut.width"));
                    length_field(ui, u, &mut s.sheet.width, 1.0);
                    ui.end_row();
                    ui.label(tr("signcut.height"));
                    length_field(ui, u, &mut s.sheet.height, 1.0);
                    ui.end_row();
                }
            }
            ui.label(tr("signcut.nest_margin"));
            length_field(ui, u, &mut s.sheet.margin, 0.0);
            ui.end_row();
            if tool != Tool::Fill {
                ui.label(tr("signcut.nest_spacing"));
                length_field(ui, u, &mut s.sheet.spacing, 0.0);
                ui.end_row();
            }
            ui.label(tr("signcut.nest_corner"));
            egui::ComboBox::from_id_salt("signcut_nest_corner")
                .selected_text(tr(corner_key(s.sheet.corner)))
                .show_ui(ui, |ui| {
                    for c in Corner::ALL {
                        ui.selectable_value(&mut s.sheet.corner, c, tr(corner_key(c)));
                    }
                });
            ui.end_row();
        });
    if tool != Tool::Fill {
        ui.checkbox(&mut s.frames, tr("signcut.nest_frames"));
    }
    ui.separator();

    let has = n > 0;
    match tool {
        Tool::Rects => {
            dim(ui, tr("signcut.nest_rects_hint"));
            ui.checkbox(&mut s.rotate90, tr("signcut.nest_rotate90"));
            if ui
                .add_enabled(has, egui::Button::new(tr("signcut.nest_rects_apply")))
                .clicked()
            {
                app.nest_rectangles();
            }
        }
        Tool::Shapes => {
            dim(ui, tr("signcut.nest_shapes_hint"));
            egui::Grid::new("signcut_nest_shapes")
                .num_columns(2)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    ui.label(tr("signcut.nest_turns"));
                    egui::ComboBox::from_id_salt("signcut_nest_turns")
                        .selected_text(tr(rotations_key(s.rotations)))
                        .show_ui(ui, |ui| {
                            for r in Rotations::ALL {
                                ui.selectable_value(&mut s.rotations, r, tr(rotations_key(r)));
                            }
                        });
                    ui.end_row();
                    ui.label(tr("signcut.nest_tolerance"));
                    length_field(ui, u, &mut s.tolerance, 0.01);
                    ui.end_row();
                });
            ui.checkbox(&mut s.holes, tr("signcut.nest_holes"));
            ui.checkbox(&mut s.improve, tr("signcut.nest_improve"));
            ui.add_enabled_ui(s.improve, |ui| {
                egui::Grid::new("signcut_nest_improve")
                    .num_columns(2)
                    .spacing([8.0, 3.0])
                    .show(ui, |ui| {
                        ui.label(tr("signcut.nest_rounds"));
                        ui.add(NumField::new(&mut s.rounds).range(1..=100_000));
                        ui.end_row();
                        ui.label(tr("signcut.nest_time_limit"));
                        ui.add(
                            NumField::new(&mut s.time_limit)
                                .range(0.1..=600.0)
                                .max_decimals(1)
                                .suffix(" s"),
                        );
                        ui.end_row();
                        ui.label(tr("signcut.nest_seed"));
                        ui.add(NumField::new(&mut s.seed).range(0..=999_999));
                        ui.end_row();
                    });
            });
            if ui
                .add_enabled(has, egui::Button::new(tr("signcut.nest_shapes_apply")))
                .clicked()
            {
                app.nest_shapes();
            }
        }
        Tool::Fill => {
            dim(ui, tr("signcut.nest_fill_hint"));
            egui::Grid::new("signcut_nest_fill")
                .num_columns(2)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    ui.label(tr("signcut.nest_gap"));
                    length_field(ui, u, &mut s.gap, 0.0);
                    ui.end_row();
                    ui.label(tr("signcut.nest_limit"));
                    ui.add(NumField::new(&mut s.fill_limit).range(0..=10_000));
                    ui.end_row();
                });
            ui.checkbox(&mut s.fill_rotate, tr("signcut.nest_fill_rotate"));
            if ui
                .add_enabled(has, egui::Button::new(tr("signcut.nest_fill_apply")))
                .clicked()
            {
                app.nest_fill();
            }
        }
    }

    // The last result.
    if let Some(r) = app.sc_nest.report.filter(|r| r.tool == tool) {
        ui.add_space(4.0);
        if r.tool == Tool::Fill {
            dim(
                ui,
                trf("signcut.nest_filled", &[("n", &r.placed.to_string())]),
            );
        } else {
            dim(ui, nest_status(u, &r));
            if r.rounds > 0 {
                dim(
                    ui,
                    trf("signcut.nest_rounds_done", &[("n", &r.rounds.to_string())]),
                );
            }
            if r.stopped_by_time {
                dim(ui, tr("signcut.nest_stopped"));
            }
        }
    }

    // Show the material on the drawing.
    let preview = if roll {
        let spec = app.nest_sheet();
        let len = app
            .sc_nest
            .report
            .filter(|r| r.roll && r.length > 0.0)
            .map_or(page.width.max(page.height), |r| r.length);
        spec.sheet_rect(0, len)
    } else {
        app.nest_plain_sheet().sheet_rect(0, 0.0)
    };
    if preview.width() > 0.0 && preview.height() > 0.0 {
        app.docker_preview.push(preview.to_path(0.1));
    }
}
