//! The Sign & Cut panel's Sign Making section: LED modules, neon tubes,
//! sign boxes, letter returns, liquid acrylic moulds, smart divide and
//! carriers. See docs/behavior/sign-and-cut-sign.md.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::signcut_sign::{led_summary, TileOrigin, Tool};
use crate::ui::signcut_docker::{dim, fmt_len, length_field};
use egui::{RichText, Ui};
use tracedraw_core::geometry::Shape as _;
use tracedraw_signcut::sign::led::{LedMode, Module};
use tracedraw_signcut::sign::returns::ReturnFace;
use tracedraw_signcut::sign::sign_box;

pub fn section(app: &mut App, ui: &mut Ui) {
    let open = app.sc_sign.open.take();
    if app.selection.is_empty() {
        dim(ui, tr("signcut.sign_select_hint"));
    }
    for tool in Tool::ALL {
        let h = egui::CollapsingHeader::new(RichText::new(tr(tool.key())).strong())
            .id_salt(("signcut_sign", tool.key()))
            .default_open(false);
        let h = if open == Some(tool) {
            h.open(Some(true))
        } else {
            h
        };
        h.show(ui, |ui| match tool {
            Tool::Led => led(app, ui),
            Tool::Neon => neon(app, ui),
            Tool::SignBox => sign_box_ui(app, ui),
            Tool::Returns => returns(app, ui),
            Tool::Mould => mould(app, ui),
            Tool::Tiles => tiles(app, ui),
            Tool::Carrier => carrier(app, ui),
        });
    }
}

/// A labelled row.
fn row<R>(ui: &mut Ui, key: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.label(tr(key));
        add(ui)
    })
    .inner
}

/// A plain number field.
fn num(ui: &mut Ui, v: &mut f64, min: f64, max: f64, decimals: usize, suffix: &str) -> bool {
    if !v.is_finite() {
        *v = min;
    }
    ui.add_sized(
        [90.0, 22.0],
        crate::ui::field::NumField::new(v)
            .range(min..=max)
            .max_decimals(decimals)
            .suffix(suffix),
    )
    .changed()
}

fn apply(ui: &mut Ui, key: &str, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new(tr(key)))
        .clicked()
}

fn led(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_sign;
    // The catalogue.
    if s.catalogue.is_empty() {
        s.catalogue = tracedraw_signcut::sign::led::catalogue();
    }
    s.module = s.module.min(s.catalogue.len().saturating_sub(1));
    row(ui, "signcut.sign_module", |ui| {
        let current = s
            .catalogue
            .get(s.module)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("signcut_sign_module")
            .selected_text(current)
            .width(180.0)
            .show_ui(ui, |ui| {
                for (i, m) in s.catalogue.iter().enumerate() {
                    ui.selectable_value(&mut s.module, i, m.name.clone());
                }
            });
    });
    ui.horizontal(|ui| {
        if ui.button(tr("signcut.sign_module_add")).clicked() {
            let copy = s.catalogue.get(s.module).cloned().map(|mut m| {
                m.name = format!("{} (2)", m.name);
                m
            });
            if let Some(m) = copy {
                s.catalogue.push(m);
                s.module = s.catalogue.len() - 1;
            }
        }
        if ui
            .add_enabled(
                s.catalogue.len() > 1,
                egui::Button::new(tr("signcut.sign_module_delete")),
            )
            .clicked()
            && s.module < s.catalogue.len()
        {
            s.catalogue.remove(s.module);
            s.module = s.module.min(s.catalogue.len().saturating_sub(1));
        }
    });
    if let Some(m) = s.catalogue.get_mut(s.module) {
        module_editor(ui, u, m);
    }
    ui.separator();
    row(ui, "signcut.sign_layout", |ui| {
        let label = |m: LedMode| {
            tr(match m {
                LedMode::Matrix => "signcut.sign_mode_matrix",
                LedMode::Shifted => "signcut.sign_mode_shifted",
                LedMode::Centre => "signcut.sign_mode_centre",
                LedMode::Outline => "signcut.sign_mode_outline",
            })
        };
        egui::ComboBox::from_id_salt("signcut_sign_led_mode")
            .selected_text(label(s.led.mode))
            .width(170.0)
            .show_ui(ui, |ui| {
                for m in [
                    LedMode::Matrix,
                    LedMode::Shifted,
                    LedMode::Centre,
                    LedMode::Outline,
                ] {
                    ui.selectable_value(&mut s.led.mode, m, label(m));
                }
            });
    });
    if matches!(s.led.mode, LedMode::Matrix | LedMode::Shifted) {
        row(ui, "signcut.sign_row_pitch", |ui| {
            length_field(ui, u, &mut s.led.row_pitch, 0.1)
        });
    }
    row(ui, "signcut.sign_module_pitch", |ui| {
        length_field(ui, u, &mut s.led.module_pitch, 0.1)
    });
    row(ui, "signcut.sign_edge", |ui| {
        length_field(ui, u, &mut s.led.edge, 0.0)
    });
    if s.led.mode == LedMode::Outline {
        row(ui, "signcut.sign_outline_offset", |ui| {
            length_field(ui, u, &mut s.led.outline_offset, 0.0)
        });
    }
    ui.checkbox(&mut s.led.rotate, tr("signcut.sign_rotate"));
    let has = !app.selection.is_empty();
    if apply(ui, "signcut.sign_led_apply", has) {
        app.sign_led_layout();
    }
    if let Some(r) = app.sc_sign.led_report {
        dim(ui, led_summary(&r));
        dim(
            ui,
            trf("signcut.sign_led_diodes", &[("n", &r.diodes.to_string())]),
        );
    }
}

fn module_editor(ui: &mut Ui, u: Units, m: &mut Module) {
    egui::Grid::new("signcut_sign_module_grid")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            ui.label(tr("signcut.sign_name"));
            ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(170.0));
            ui.end_row();
            ui.label(tr("signcut.sign_round"));
            ui.checkbox(&mut m.round, "");
            ui.end_row();
            ui.label(tr(if m.round {
                "signcut.sign_diameter"
            } else {
                "signcut.width"
            }));
            length_field(ui, u, &mut m.width, 0.1);
            ui.end_row();
            if !m.round {
                ui.label(tr("signcut.height"));
                length_field(ui, u, &mut m.height, 0.1);
                ui.end_row();
            }
            ui.label(tr("signcut.sign_diodes"));
            let mut d = f64::from(m.diodes);
            if num(ui, &mut d, 0.0, 1000.0, 0, "") {
                m.diodes = d.round().clamp(0.0, 1000.0) as u32;
            }
            ui.end_row();
            ui.label(tr("signcut.sign_power"));
            num(ui, &mut m.power, 0.0, 10_000.0, 3, " W");
            ui.end_row();
            ui.label(tr("signcut.sign_price"));
            num(ui, &mut m.price, 0.0, 1_000_000.0, 2, "");
            ui.end_row();
            ui.label(tr("signcut.sign_color"));
            ui.color_edit_button_srgb(&mut m.color);
            ui.end_row();
        });
}

fn neon(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_sign;
    dim(ui, tr("signcut.sign_neon_hint"));
    row(ui, "signcut.sign_tube_diameter", |ui| {
        length_field(ui, u, &mut s.neon.diameter, 0.1)
    });
    row(ui, "signcut.sign_max_length", |ui| {
        length_field(ui, u, &mut s.neon.max_length, 10.0)
    });
    row(ui, "signcut.sign_support_spacing", |ui| {
        length_field(ui, u, &mut s.neon.support_spacing, 0.0)
    });
    row(ui, "signcut.sign_hole_diameter", |ui| {
        length_field(ui, u, &mut s.neon.hole_diameter, 0.0)
    });
    ui.checkbox(&mut s.neon_outline, tr("signcut.sign_neon_outline"));
    let has = !app.selection.is_empty();
    if apply(ui, "signcut.sign_neon_apply", has) {
        app.sign_neon();
    }
    if let Some(r) = app.sc_sign.neon_report {
        dim(
            ui,
            trf(
                "signcut.sign_neon_done",
                &[
                    ("n", &r.tubes.to_string()),
                    ("len", &fmt_len(u, r.length)),
                    ("holes", &r.supports.to_string()),
                ],
            ),
        );
    }
}

fn sign_box_ui(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let has = !app.selection.is_empty();
    dim(ui, tr("signcut.sign_box_hint"));
    let s = &mut app.sc_sign;
    if !has {
        row(ui, "signcut.width", |ui| {
            length_field(ui, u, &mut s.box_width, 1.0)
        });
        row(ui, "signcut.height", |ui| {
            length_field(ui, u, &mut s.box_height, 1.0)
        });
    }
    let b = &mut s.sign_box;
    row(ui, "signcut.sign_depth", |ui| {
        length_field(ui, u, &mut b.depth, 0.1)
    });
    row(ui, "signcut.sign_lip", |ui| {
        length_field(ui, u, &mut b.lip, 0.0)
    });
    row(ui, "signcut.sign_corner_gap", |ui| {
        length_field(ui, u, &mut b.corner_gap, 0.0)
    });
    row(ui, "signcut.sign_relief", |ui| {
        length_field(ui, u, &mut b.relief, 0.0)
    });
    ui.checkbox(&mut b.holes, tr("signcut.sign_fixing_holes"));
    if b.holes {
        row(ui, "signcut.sign_hole_spacing", |ui| {
            length_field(ui, u, &mut b.hole_spacing, 1.0)
        });
        row(ui, "signcut.sign_hole_diameter", |ui| {
            length_field(ui, u, &mut b.hole_diameter, 0.0)
        });
    }
    // Show the pattern on the drawing.
    let face = app.sign_box_face();
    if let Some(p) = sign_box::sign_box(face, &app.sc_sign.sign_box) {
        let bb = p.outline.bounding_box();
        dim(
            ui,
            trf(
                "signcut.sign_box_size",
                &[
                    ("w", &fmt_len(u, bb.width())),
                    ("h", &fmt_len(u, bb.height())),
                ],
            ),
        );
        app.docker_preview.push(p.outline);
        for (a, b) in p.folds {
            app.docker_preview
                .push(tracedraw_core::geometry::polyline_path(&[a, b], false));
        }
    }
    if apply(ui, "signcut.sign_box_apply", true) {
        app.sign_make_box();
    }
}

fn returns(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_sign;
    let r = &mut s.returns;
    row(ui, "signcut.sign_depth", |ui| {
        length_field(ui, u, &mut r.depth, 0.1)
    });
    row(ui, "signcut.sign_overlap", |ui| {
        length_field(ui, u, &mut r.overlap, 0.0)
    });
    row(ui, "signcut.sign_thickness", |ui| {
        length_field(ui, u, &mut r.thickness, 0.0)
    });
    row(ui, "signcut.sign_face", |ui| {
        for (f, key) in [
            (ReturnFace::Inner, "signcut.sign_face_inner"),
            (ReturnFace::Middle, "signcut.sign_face_middle"),
            (ReturnFace::Outer, "signcut.sign_face_outer"),
        ] {
            ui.radio_value(&mut r.face, f, tr(key));
        }
    });
    row(ui, "signcut.sign_notch_angle", |ui| {
        num(ui, &mut r.notch_angle, 0.0, 180.0, 1, "\u{b0}")
    });
    row(ui, "signcut.sign_notch_size", |ui| {
        length_field(ui, u, &mut r.notch_size, 0.0)
    });
    strip_layout(ui, u, s);
    let has = !app.selection.is_empty();
    if apply(ui, "signcut.sign_returns_apply", has) {
        app.sign_returns();
    }
    if let Some((n, len)) = app.sc_sign.returns_report {
        dim(
            ui,
            trf(
                "signcut.sign_strips_done",
                &[("n", &n.to_string()), ("len", &fmt_len(u, len))],
            ),
        );
    }
}

fn strip_layout(ui: &mut Ui, u: Units, s: &mut crate::signcut_sign::Settings) {
    row(ui, "signcut.sign_strip_gap", |ui| {
        length_field(ui, u, &mut s.strip_gap, 0.0)
    });
    ui.checkbox(&mut s.labels, tr("signcut.sign_labels"));
}

fn mould(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_sign;
    dim(ui, tr("signcut.sign_mould_hint"));
    let m = &mut s.mould;
    row(ui, "signcut.sign_wall", |ui| {
        length_field(ui, u, &mut m.wall, 0.0)
    });
    row(ui, "signcut.sign_fill_depth", |ui| {
        length_field(ui, u, &mut m.depth, 0.1)
    });
    row(ui, "signcut.sign_overlap", |ui| {
        length_field(ui, u, &mut m.overlap, 0.0)
    });
    strip_layout(ui, u, s);
    let has = !app.selection.is_empty();
    if apply(ui, "signcut.sign_mould_apply", has) {
        app.sign_mould();
    }
    if let Some((n, len)) = app.sc_sign.mould_report {
        dim(
            ui,
            trf(
                "signcut.sign_strips_done",
                &[("n", &n.to_string()), ("len", &fmt_len(u, len))],
            ),
        );
    }
}

fn tiles(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_sign;
    let t = &mut s.tiles;
    row(ui, "signcut.width", |ui| {
        length_field(ui, u, &mut t.width, 1.0)
    });
    row(ui, "signcut.height", |ui| {
        length_field(ui, u, &mut t.height, 1.0)
    });
    row(ui, "signcut.sign_overlap", |ui| {
        length_field(ui, u, &mut t.overlap, 0.0)
    });
    row(ui, "signcut.sign_spread", |ui| {
        length_field(ui, u, &mut t.gap, 0.0)
    });
    row(ui, "signcut.sign_origin", |ui| {
        ui.radio_value(
            &mut s.tile_origin,
            TileOrigin::Selection,
            tr("signcut.sign_origin_selection"),
        );
        ui.radio_value(
            &mut s.tile_origin,
            TileOrigin::Page,
            tr("signcut.sign_origin_page"),
        );
    });
    ui.checkbox(&mut s.tile_labels, tr("signcut.sign_tile_labels"));
    ui.checkbox(&mut s.tile_frames, tr("signcut.sign_tile_frames"));
    // Show the grid on the drawing.
    let grid = app.sign_tile_grid();
    if let Some(g) = grid {
        dim(
            ui,
            trf(
                "signcut.sign_tiles_grid",
                &[("cols", &g.cols.to_string()), ("rows", &g.rows.to_string())],
            ),
        );
        for c in 0..g.cols {
            for r in 0..g.rows {
                app.docker_preview.push(g.rect(c, r).to_path(0.01));
            }
        }
    } else if !app.selection.is_empty() {
        dim(ui, tr("signcut.sign_tiles_bad"));
    }
    if apply(ui, "signcut.sign_tiles_apply", grid.is_some()) {
        app.sign_divide();
    }
    if let Some((c, r, n)) = app.sc_sign.tiles_report {
        dim(
            ui,
            trf(
                "signcut.sign_tiles_last",
                &[
                    ("cols", &c.to_string()),
                    ("rows", &r.to_string()),
                    ("n", &n.to_string()),
                ],
            ),
        );
    }
}

fn carrier(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let c = &mut app.sc_sign.carrier;
    row(ui, "signcut.sign_distance", |ui| {
        length_field(ui, u, &mut c.distance, 0.0)
    });
    row(ui, "signcut.sign_rounding", |ui| {
        length_field(ui, u, &mut c.rounding, 0.0)
    });
    ui.checkbox(&mut c.fill_holes, tr("signcut.sign_fill_holes"));
    let has = !app.selection.is_empty();
    if apply(ui, "signcut.sign_carrier_apply", has) {
        app.sign_carrier();
    }
    if let Some(n) = app.sc_sign.carrier_report {
        dim(
            ui,
            trf("signcut.sign_carrier_done", &[("n", &n.to_string())]),
        );
    }
}
