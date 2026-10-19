//! The Sign & Cut panel's Cutting section: weeding, bridges, cut order,
//! plotter output, registration marks and crop marks. See
//! docs/behavior/sign-and-cut-cut.md.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::signcut_cut::Contour;
use crate::ui::field::NumField;
use crate::ui::signcut_docker::{dim, fmt_len, length_field};
use egui::Ui;
use tracedraw_core::geometry::BezPath;
use tracedraw_signcut::cut::hpgl::{Origin, Profile, MAX_PASSES};
use tracedraw_signcut::cut::marks::MarkStyle;
use tracedraw_signcut::cut::order::{Direction, StartAt};
use tracedraw_signcut::cut::weed::{WeedMode, MAX_DIVIDERS};

pub fn section(app: &mut App, ui: &mut Ui) {
    dim(
        ui,
        tr(if app.selection.is_empty() {
            "signcut.whole_page"
        } else {
            "signcut.selection_only"
        }),
    );
    weeding(app, ui);
    ui.separator();
    bridges(app, ui);
    ui.separator();
    cut_order(app, ui);
    ui.separator();
    plotter(app, ui);
    ui.separator();
    registration(app, ui);
    ui.separator();
    crop(app, ui);
}

/// A label and a length field on one row.
fn len_row(ui: &mut Ui, u: Units, key: &str, mm: &mut f64, min: f64) {
    ui.horizontal(|ui| {
        ui.label(tr(key));
        length_field(ui, u, mm, min);
    });
}

/// A label and a number field on one row.
fn num_row<N: egui::emath::Numeric>(
    ui: &mut Ui,
    key: &str,
    v: &mut N,
    range: std::ops::RangeInclusive<N>,
    suffix: &str,
) {
    ui.horizontal(|ui| {
        ui.label(tr(key));
        ui.add_sized(
            [90.0, 22.0],
            NumField::new(v).range(range).max_decimals(3).suffix(suffix),
        );
    });
}

fn weeding(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.cut_weeding"));
    let w = &mut app.sc_cut.weed;
    ui.horizontal(|ui| {
        ui.radio_value(&mut w.mode, WeedMode::Lines, tr("signcut.cut_weed_lines"));
        ui.radio_value(&mut w.mode, WeedMode::Boxes, tr("signcut.cut_weed_boxes"));
    });
    match w.mode {
        WeedMode::Lines => {
            len_row(ui, u, "signcut.cut_gap", &mut w.gap, 0.0);
            len_row(ui, u, "signcut.cut_shorten", &mut w.shorten, 0.0);
            num_row(
                ui,
                "signcut.cut_dividers",
                &mut w.dividers,
                0..=MAX_DIVIDERS,
                "",
            );
        }
        WeedMode::Boxes => {
            len_row(ui, u, "signcut.cut_box_x", &mut w.box_x, 0.0);
            len_row(ui, u, "signcut.cut_box_y", &mut w.box_y, 0.0);
        }
    }
    ui.horizontal(|ui| {
        ui.checkbox(&mut w.frame, tr("signcut.cut_frame"));
        ui.add_enabled_ui(w.frame, |ui| length_field(ui, u, &mut w.margin, 0.0));
    });
    if ui.button(tr("signcut.cut_weed_apply")).clicked() {
        app.sc_cut_weed();
    }
}

fn bridges(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.cut_bridges"));
    let s = &mut app.sc_cut;
    len_row(
        ui,
        u,
        "signcut.cut_bridge_length",
        &mut s.bridge.length,
        0.01,
    );
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut s.bridge_by_distance,
            false,
            tr("signcut.cut_bridge_count"),
        );
        ui.add_enabled_ui(!s.bridge_by_distance, |ui| {
            ui.add_sized(
                [60.0, 22.0],
                NumField::new(&mut s.bridge_count).range(1..=1000u32),
            )
        });
    });
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut s.bridge_by_distance,
            true,
            tr("signcut.cut_bridge_every"),
        );
        ui.add_enabled_ui(s.bridge_by_distance, |ui| {
            length_field(ui, u, &mut s.bridge_every, 1.0)
        });
    });
    len_row(
        ui,
        u,
        "signcut.cut_bridge_min",
        &mut s.bridge.min_outline,
        0.0,
    );
    if ui.button(tr("signcut.cut_bridge_apply")).clicked() {
        app.sc_cut_bridges();
    }
}

fn cut_order(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.cut_order"));
    let r = &mut app.sc_cut.route;
    let dir_label = |d: Direction| {
        tr(match d {
            Direction::Keep => "signcut.cut_keep",
            Direction::Clockwise => "signcut.cut_dir_cw",
            Direction::CounterClockwise => "signcut.cut_dir_ccw",
        })
    };
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_direction"));
        egui::ComboBox::from_id_salt("sc_cut_direction")
            .selected_text(dir_label(r.direction))
            .width(140.0)
            .show_ui(ui, |ui| {
                for d in [
                    Direction::Keep,
                    Direction::Clockwise,
                    Direction::CounterClockwise,
                ] {
                    ui.selectable_value(&mut r.direction, d, dir_label(d));
                }
            });
    });
    let start_label = |s: StartAt| {
        tr(match s {
            StartAt::Keep => "signcut.cut_keep",
            StartAt::Nearest => "signcut.cut_start_nearest",
            StartAt::BottomLeft => "signcut.cut_start_bottom_left",
        })
    };
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_start"));
        egui::ComboBox::from_id_salt("sc_cut_start")
            .selected_text(start_label(r.start))
            .width(140.0)
            .show_ui(ui, |ui| {
                for s in [StartAt::Nearest, StartAt::BottomLeft, StartAt::Keep] {
                    ui.selectable_value(&mut r.start, s, start_label(s));
                }
            });
    });
    ui.checkbox(&mut r.inner_first, tr("signcut.cut_inner_first"));
    ui.checkbox(&mut app.sc_cut.show_route, tr("signcut.cut_show_route"));
    if app.sc_cut.show_route {
        let cache = app.sc_cut_route_preview();
        let (moves, before, after) = (cache.moves.clone(), cache.before, cache.after);
        for (a, b) in moves {
            let mut p = BezPath::new();
            p.move_to(a);
            p.line_to(b);
            app.docker_preview.push(p);
        }
        dim(
            ui,
            trf(
                "signcut.cut_travel",
                &[
                    ("before", &fmt_len(u, before)),
                    ("after", &fmt_len(u, after)),
                ],
            ),
        );
    }
    if ui.button(tr("signcut.cut_order_apply")).clicked() {
        app.sc_cut_order();
    }
    if let Some((before, after)) = app.sc_cut.last_route {
        dim(
            ui,
            trf(
                "signcut.cut_order_done",
                &[
                    ("before", &fmt_len(u, before)),
                    ("after", &fmt_len(u, after)),
                ],
            ),
        );
    }
}

fn origin_label(o: Origin) -> String {
    tr(match o {
        Origin::BottomLeft => "signcut.cut_origin_bl",
        Origin::BottomRight => "signcut.cut_origin_br",
        Origin::TopLeft => "signcut.cut_origin_tl",
        Origin::TopRight => "signcut.cut_origin_tr",
    })
}

fn text_row(ui: &mut Ui, key: &str, text: &mut String) {
    ui.horizontal(|ui| {
        ui.label(tr(key));
        ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
    });
}

fn profile_settings(ui: &mut Ui, u: Units, p: &mut Profile) {
    text_row(ui, "signcut.cut_name", &mut p.name);
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_origin"));
        egui::ComboBox::from_id_salt("sc_cut_origin")
            .selected_text(origin_label(p.origin))
            .width(120.0)
            .show_ui(ui, |ui| {
                for o in Origin::ALL {
                    ui.selectable_value(&mut p.origin, o, origin_label(o));
                }
            });
    });
    num_row(
        ui,
        "signcut.cut_units",
        &mut p.units_per_mm,
        1.0..=10_000.0,
        "",
    );
    text_row(ui, "signcut.cut_pen_up", &mut p.pen_up);
    text_row(ui, "signcut.cut_pen_down", &mut p.pen_down);
    text_row(ui, "signcut.cut_init", &mut p.init);
    text_row(ui, "signcut.cut_end", &mut p.end);
    num_row(
        ui,
        "signcut.cut_velocity",
        &mut p.velocity,
        0.0..=10_000.0,
        "",
    );
    num_row(ui, "signcut.cut_force", &mut p.force, 0.0..=10_000.0, "");
    len_row(ui, u, "signcut.cut_knife_offset", &mut p.knife_offset, 0.0);
    num_row(
        ui,
        "signcut.cut_corner_limit",
        &mut p.corner_limit,
        0.0..=180.0,
        "\u{b0}",
    );
    len_row(ui, u, "signcut.cut_overcut", &mut p.overcut, 0.0);
    ui.checkbox(&mut p.mirror, tr("signcut.cut_mirror"));
    ui.checkbox(&mut p.rotate, tr("signcut.cut_rotate"));
    num_row(ui, "signcut.cut_passes", &mut p.passes, 1..=MAX_PASSES, "");
    ui.horizontal(|ui| {
        ui.checkbox(&mut p.weed_frame, tr("signcut.cut_weed_frame"));
        ui.add_enabled_ui(p.weed_frame, |ui| {
            length_field(ui, u, &mut p.weed_margin, 0.0)
        });
    });
    ui.checkbox(&mut p.line_breaks, tr("signcut.cut_line_breaks"));
}

fn plotter(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.cut_plotter"));
    let s = &mut app.sc_cut;
    if s.profiles.is_empty() {
        s.profiles.push(Profile::default());
    }
    s.profile = s.profile.min(s.profiles.len() - 1);
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_profile"));
        let current = s
            .profiles
            .get(s.profile)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("sc_cut_profile")
            .selected_text(current)
            .width(120.0)
            .show_ui(ui, |ui| {
                for (i, p) in s.profiles.iter().enumerate() {
                    ui.selectable_value(&mut s.profile, i, p.name.clone());
                }
            });
        if ui.button(tr("signcut.cut_profile_add")).clicked() {
            let mut p = s.current_profile();
            p.name = format!("{} {}", p.name, s.profiles.len() + 1);
            s.profiles.push(p);
            s.profile = s.profiles.len() - 1;
        }
        if ui
            .add_enabled(
                s.profiles.len() > 1,
                egui::Button::new(tr("signcut.cut_profile_delete")),
            )
            .clicked()
        {
            s.profiles.remove(s.profile);
            s.profile = s.profile.min(s.profiles.len().saturating_sub(1));
        }
    });
    egui::CollapsingHeader::new(tr("signcut.cut_profile_settings"))
        .id_salt("sc_cut_profile_settings")
        .show(ui, |ui| {
            if let Some(p) = s.profiles.get_mut(s.profile) {
                profile_settings(ui, u, p);
            }
        });
    ui.checkbox(&mut s.plot_optimize, tr("signcut.cut_plot_optimize"));
    if ui.button(tr("signcut.cut_save")).clicked() {
        app.sc_cut_save_plt();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.horizontal(|ui| {
            ui.label(tr("signcut.cut_port"));
            ui.add(egui::TextEdit::singleline(&mut app.sc_cut.port).desired_width(120.0));
            if ui
                .add_enabled(
                    !app.sc_cut.port.trim().is_empty(),
                    egui::Button::new(tr("signcut.cut_send")),
                )
                .clicked()
            {
                let _ = app.sc_cut_send();
            }
        });
        dim(ui, tr("signcut.cut_port_hint"));
    }
    if let Some(p) = app.sc_cut.last_plot {
        dim(
            ui,
            trf(
                "signcut.cut_plot_summary",
                &[
                    ("n", &p.outlines.to_string()),
                    ("len", &fmt_len(u, p.cut_length)),
                    ("w", &fmt_len(u, p.width)),
                    ("h", &fmt_len(u, p.height)),
                ],
            ),
        );
    }
}

fn registration(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.cut_registration"));
    let s = &mut app.sc_cut;
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_around"));
        ui.radio_value(&mut s.reg_page, false, tr("signcut.cut_around_objects"));
        ui.radio_value(&mut s.reg_page, true, tr("signcut.cut_around_page"));
    });
    let style_label = |m: MarkStyle| {
        tr(match m {
            MarkStyle::Square => "signcut.cut_style_square",
            MarkStyle::Circle => "signcut.cut_style_circle",
            MarkStyle::Corner => "signcut.cut_style_corner",
        })
    };
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_style"));
        egui::ComboBox::from_id_salt("sc_cut_mark_style")
            .selected_text(style_label(s.reg.style))
            .width(120.0)
            .show_ui(ui, |ui| {
                for m in [MarkStyle::Square, MarkStyle::Circle, MarkStyle::Corner] {
                    ui.selectable_value(&mut s.reg.style, m, style_label(m));
                }
            });
    });
    if s.reg.style == MarkStyle::Corner {
        len_row(
            ui,
            u,
            "signcut.cut_corner_length",
            &mut s.reg.corner_length,
            0.1,
        );
        len_row(
            ui,
            u,
            "signcut.cut_corner_width",
            &mut s.reg.corner_width,
            0.01,
        );
    }
    len_row(ui, u, "signcut.cut_mark_size", &mut s.reg.size, 0.1);
    len_row(ui, u, "signcut.cut_offset", &mut s.reg.offset, 0.0);
    len_row(ui, u, "signcut.cut_mark_spacing", &mut s.reg.spacing, 0.0);
    let contour_label = |c: Contour| {
        tr(match c {
            Contour::Outlines => "signcut.cut_contour_outlines",
            Contour::BoundingBox => "signcut.cut_contour_box",
            Contour::None => "signcut.cut_contour_none",
        })
    };
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cut_contour"));
        egui::ComboBox::from_id_salt("sc_cut_contour")
            .selected_text(contour_label(s.contour))
            .width(120.0)
            .show_ui(ui, |ui| {
                for c in [Contour::Outlines, Contour::BoundingBox, Contour::None] {
                    ui.selectable_value(&mut s.contour, c, contour_label(c));
                }
            });
    });
    if ui.button(tr("signcut.cut_reg_apply")).clicked() {
        app.sc_cut_registration();
    }
}

fn crop(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.cut_crop"));
    let s = &mut app.sc_cut;
    len_row(ui, u, "signcut.length", &mut s.crop.length, 0.1);
    len_row(ui, u, "signcut.cut_offset", &mut s.crop.offset, 0.0);
    len_row(ui, u, "signcut.cut_crop_width", &mut s.crop_width, 0.01);
    ui.checkbox(&mut s.crop_each, tr("signcut.cut_crop_each"));
    if ui.button(tr("signcut.cut_crop_apply")).clicked() {
        app.sc_cut_crop_marks();
    }
}
