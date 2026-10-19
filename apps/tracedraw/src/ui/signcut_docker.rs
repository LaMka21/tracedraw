//! The Sign & Cut panel: Measure (length and area), Clean Up (duplicate
//! objects, duplicate edges, crossings) and Size (Set Size, Reference
//! Scale, Replace). See docs/behavior/sign-and-cut.md.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::signcut::{Section, CROSSING_MARK_PX};
use crate::theme::Tokens;
use egui::{RichText, Ui};
use tracedraw_core::geometry::{Circle, Shape as _};
use tracedraw_signcut::sizing::{Anchor, RefMeasure, ReplaceFit, SizeRule};

pub fn signcut(app: &mut App, ui: &mut Ui) {
    let open = app.signcut.open.take();
    let header = |key: &str, section: Section| {
        let h = egui::CollapsingHeader::new(RichText::new(tr(key)).strong())
            .id_salt(("signcut", key))
            .default_open(section == Section::Measure);
        match open {
            Some(s) if s == section => h.open(Some(true)),
            _ => h,
        }
    };
    header("signcut.measure", Section::Measure).show(ui, |ui| measure(app, ui));
    header("signcut.cleanup", Section::Cleanup).show(ui, |ui| cleanup(app, ui));
    header("signcut.size", Section::Size).show(ui, |ui| size(app, ui));
    header("signcut.area_cut", Section::Cut)
        .show(ui, |ui| crate::ui::signcut_cut::section(app, ui));
    header("signcut.area_nest", Section::Nest)
        .show(ui, |ui| crate::ui::signcut_nest::section(app, ui));
    header("signcut.area_calc", Section::Calc)
        .show(ui, |ui| crate::ui::signcut_calc::section(app, ui));
    header("signcut.area_sign", Section::Sign)
        .show(ui, |ui| crate::ui::signcut_sign::section(app, ui));
    header("signcut.area_cnc", Section::Cnc)
        .show(ui, |ui| crate::ui::signcut_cnc::section(app, ui));
    header("signcut.area_fx", Section::Fx).show(ui, |ui| crate::ui::signcut_fx::section(app, ui));
    // Mark the crossings found, while the drawing is as it was.
    let r = CROSSING_MARK_PX / f64::from(app.view.zoom.max(1e-3));
    for p in app.signcut_crossing_marks() {
        app.docker_preview.push(Circle::new(p, r).to_path(0.01));
    }
}

pub(crate) fn dim(ui: &mut Ui, text: String) {
    ui.label(RichText::new(text).color(Tokens::text_dim()).size(11.0));
}

/// A length field in the document's units, holding millimetres.
pub(crate) fn length_field(ui: &mut Ui, u: Units, mm: &mut f64, min: f64) -> bool {
    let mut v = u.world_from_mm(*mm);
    let changed = ui
        .add_sized(
            [90.0, 22.0],
            crate::ui::field::NumField::new(&mut v)
                .range(u.world_from_mm(min)..=1_000_000.0)
                .max_decimals(3)
                .suffix(format!(" {}", u.short()))
                .unit_mm(u.mm()),
        )
        .changed();
    if changed {
        *mm = u.world_to_mm(v);
    }
    changed
}

pub(crate) fn fmt_len(u: Units, mm: f64) -> String {
    format!("{:.2} {}", u.world_from_mm(mm), u.short())
}

pub(crate) fn fmt_area(u: Units, mm2: f64) -> String {
    let k = u.world_from_mm(1.0);
    let m2 = mm2 / 1_000_000.0;
    format!(
        "{:.2} {}\u{b2}  ({:.4} m\u{b2})",
        mm2 * k * k,
        u.short(),
        m2
    )
}

fn measure(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    dim(
        ui,
        tr(if app.selection.is_empty() {
            "signcut.whole_page"
        } else {
            "signcut.selection_only"
        }),
    );
    if ui.button(tr("signcut.measure_now")).clicked() {
        app.signcut_measure();
    }
    let Some((rev, m)) = app.signcut.measured else {
        return;
    };
    if rev != app.engine.revision() {
        dim(ui, tr("signcut.outdated"));
    }
    egui::Grid::new("signcut_measure")
        .num_columns(2)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            let mut row = |key: &str, value: String| {
                ui.label(tr(key));
                ui.label(value);
                ui.end_row();
            };
            row("signcut.objects", m.objects.to_string());
            row("signcut.length", fmt_len(u, m.length));
            row("signcut.area", fmt_area(u, m.area));
            row("signcut.outer_area", fmt_area(u, m.outer_area));
            row("signcut.hole_area", fmt_area(u, m.hole_area));
            row("signcut.bbox_area", fmt_area(u, m.bbox_area));
            if let Some(b) = m.bbox {
                row(
                    "signcut.bbox_size",
                    format!(
                        "{} \u{d7} {}",
                        fmt_len(u, b.width().abs()),
                        fmt_len(u, b.height().abs())
                    ),
                );
            }
            row(
                "signcut.paths",
                trf(
                    "signcut.paths_value",
                    &[
                        ("closed", &m.closed_paths.to_string()),
                        ("open", &m.open_paths.to_string()),
                    ],
                ),
            );
            row("signcut.nodes", m.nodes.to_string());
        });
}

fn cleanup(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.horizontal(|ui| {
        ui.label(tr("signcut.tolerance"));
        length_field(ui, u, &mut app.signcut.tolerance, 0.001);
    });
    dim(
        ui,
        tr(if app.selection.is_empty() {
            "signcut.whole_page"
        } else {
            "signcut.selection_only"
        }),
    );
    ui.add_space(4.0);
    ui.strong(tr("signcut.duplicates"));
    ui.checkbox(
        &mut app.signcut.delete_duplicates,
        tr("signcut.delete_duplicates"),
    );
    if ui.button(tr("signcut.find_duplicates")).clicked() {
        app.signcut_find_duplicates();
    }
    ui.add_space(4.0);
    ui.strong(tr("signcut.duplicate_edges"));
    dim(ui, tr("signcut.duplicate_edges_hint"));
    if ui.button(tr("signcut.remove_duplicate_edges")).clicked() {
        app.signcut_remove_duplicate_edges();
    }
    ui.add_space(4.0);
    ui.strong(tr("signcut.crossings"));
    ui.checkbox(
        &mut app.signcut.crossings_between,
        tr("signcut.crossings_between"),
    );
    if ui.button(tr("signcut.find_crossings")).clicked() {
        app.signcut_find_crossings();
    }
    let marks = app.signcut_crossing_marks().len();
    if marks > 0 {
        dim(
            ui,
            trf("signcut.crossings_found", &[("n", &marks.to_string())]),
        );
    }
}

/// The nine anchor points as a grid of small buttons.
pub(crate) fn anchor_picker(ui: &mut Ui, id: &str, anchor: &mut Anchor) {
    egui::Grid::new(id).spacing([2.0, 2.0]).show(ui, |ui| {
        for (i, a) in Anchor::ALL.iter().enumerate() {
            let on = *anchor == *a;
            let b = egui::Button::new(if on { "\u{25cf}" } else { "\u{25cb}" })
                .min_size(egui::vec2(18.0, 18.0))
                .selected(on);
            if ui.add(b).clicked() {
                *anchor = *a;
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
}

fn size(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let has = !app.selection.is_empty();
    // Set Size.
    ui.strong(tr("signcut.set_size"));
    let s = &mut app.signcut;
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_width, tr("signcut.width"));
        ui.add_enabled_ui(s.use_width, |ui| {
            length_field(ui, u, &mut s.size_width, 0.001)
        });
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_height, tr("signcut.height"));
        ui.add_enabled_ui(s.use_height, |ui| {
            length_field(ui, u, &mut s.size_height, 0.001)
        });
    });
    ui.checkbox(&mut s.size.proportional, tr("signcut.proportional"));
    ui.horizontal(|ui| {
        ui.label(tr("signcut.apply_to"));
        let label = |r: SizeRule| {
            tr(match r {
                SizeRule::Exact => "signcut.rule_exact",
                SizeRule::ShrinkOnly => "signcut.rule_shrink",
                SizeRule::GrowOnly => "signcut.rule_grow",
            })
        };
        egui::ComboBox::from_id_salt("signcut_rule")
            .selected_text(label(s.size.rule))
            .width(150.0)
            .show_ui(ui, |ui| {
                for r in [SizeRule::Exact, SizeRule::ShrinkOnly, SizeRule::GrowOnly] {
                    ui.selectable_value(&mut s.size.rule, r, label(r));
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label(tr("signcut.anchor"));
        anchor_picker(ui, "signcut_size_anchor", &mut s.size.anchor);
    });
    let can_size = has && (s.use_width || s.use_height);
    if ui
        .add_enabled(can_size, egui::Button::new(tr("docker.apply")))
        .clicked()
    {
        app.signcut_set_size();
    }
    ui.separator();

    // Reference Scale.
    ui.strong(tr("signcut.reference_scale"));
    dim(ui, tr("signcut.reference_hint"));
    let s = &mut app.signcut;
    ui.horizontal(|ui| {
        ui.label(tr("signcut.measure_by"));
        for (m, key) in [
            (RefMeasure::Width, "signcut.width"),
            (RefMeasure::Height, "signcut.height"),
            (RefMeasure::Length, "signcut.length"),
        ] {
            if ui.radio_value(&mut s.ref_measure, m, tr(key)).changed() {
                s.reference = None;
            }
        }
    });
    let one = app.selection.len() == 1;
    if ui
        .add_enabled(one, egui::Button::new(tr("signcut.pick_reference")))
        .clicked()
    {
        app.signcut_pick_reference();
    }
    match app.signcut_reference_value() {
        Some(v) => {
            dim(ui, trf("signcut.reference_now", &[("v", &fmt_len(u, v))]));
            let s = &mut app.signcut;
            ui.horizontal(|ui| {
                ui.label(tr("signcut.reference_wanted"));
                length_field(ui, u, &mut s.ref_wanted, 0.001);
            });
            ui.checkbox(&mut s.ref_whole_page, tr("signcut.whole_page_scale"));
            ui.horizontal(|ui| {
                ui.label(tr("signcut.anchor"));
                anchor_picker(ui, "signcut_ref_anchor", &mut s.ref_anchor);
            });
            if ui.button(tr("docker.apply")).clicked() {
                app.signcut_reference_scale();
            }
        }
        None => {
            app.signcut.reference = None;
            dim(ui, tr("signcut.no_reference"));
        }
    }
    ui.separator();

    // Replace.
    ui.strong(tr("signcut.replace"));
    dim(ui, tr("signcut.replace_hint"));
    if ui
        .add_enabled(one, egui::Button::new(tr("signcut.pick_master")))
        .clicked()
    {
        app.signcut_pick_master();
    }
    let master = app
        .signcut
        .master
        .filter(|id| app.doc().find_shape(*id).is_some());
    app.signcut.master = master;
    dim(
        ui,
        tr(if master.is_some() {
            "signcut.master_set"
        } else {
            "signcut.no_master"
        }),
    );
    let s = &mut app.signcut;
    for (f, key) in [
        (ReplaceFit::Keep, "signcut.fit_keep"),
        (ReplaceFit::Fit, "signcut.fit_fit"),
        (ReplaceFit::Stretch, "signcut.fit_stretch"),
    ] {
        ui.radio_value(&mut s.replace_fit, f, tr(key));
    }
    ui.checkbox(&mut s.replace_turn, tr("signcut.replace_turn"));
    ui.checkbox(&mut s.replace_keep, tr("signcut.replace_keep"));
    let can = master.is_some() && app.selection.iter().any(|id| Some(*id) != master);
    if ui
        .add_enabled(can, egui::Button::new(tr("signcut.replace_apply")))
        .clicked()
    {
        app.signcut_replace();
    }
}
