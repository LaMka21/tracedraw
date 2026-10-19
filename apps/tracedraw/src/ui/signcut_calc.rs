//! The Sign & Cut panel's Calculations section: cutting time, cost and
//! volume and weight of the selection (or the page), with the profiles
//! and the price and density lists edited in place. Nothing here changes
//! the drawing. See docs/behavior/sign-and-cut-calc.md.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::signcut_calc::{fmt_weight, List, Settings, MAX_NAME, THICKNESS_RANGE};
use crate::ui::field::NumField;
use crate::ui::signcut_docker::{dim, fmt_area, fmt_len, length_field};
use egui::{RichText, Ui};
use tracedraw_core::geometry::Shape as _;
use tracedraw_signcut::calc::cost::{
    Material, MaterialUnit, QtyUnit, Service, ServiceUnit, MAX_COPIES,
};
use tracedraw_signcut::calc::time::{self, TimeProfile, TravelOrder};
use tracedraw_signcut::calc::weight::Density;

/// Width of the name fields of the lists.
const NAME_WIDTH: f32 = 170.0;

pub fn section(app: &mut App, ui: &mut Ui) {
    app.sc_calc.ensure_loaded();
    let kept = app.sc_calc.to_json();
    dim(
        ui,
        tr(if app.selection.is_empty() {
            "signcut.whole_page"
        } else {
            "signcut.selection_only"
        }),
    );
    if ui.button(tr("signcut.calc_calculate")).clicked() {
        app.sc_calc_run();
    }
    let outdated = app.sc_calc_outdated();
    if app.sc_calc.measured.is_none() {
        dim(ui, tr("signcut.calc_none_yet"));
    } else if outdated {
        dim(ui, tr("signcut.calc_outdated"));
    }
    ui.add_space(4.0);
    time_block(app, ui);
    ui.separator();
    cost_block(app, ui, outdated);
    ui.separator();
    weight_block(app, ui);
    // Keep what changed for the next start, once nothing is being typed.
    if app.sc_calc.to_json() != kept {
        app.sc_calc.dirty = true;
    }
    if app.sc_calc.dirty && ui.ctx().memory(|m| m.focused().is_none()) {
        app.sc_calc.save_if_dirty();
    }
}

/// Seconds as h:mm:ss and in seconds.
fn fmt_time(seconds: f64) -> String {
    let (h, m, s) = time::hms(seconds);
    format!("{h}:{m:02}:{s:02}  ({seconds:.1} s)")
}

/// A speed field in document units per second, holding mm/s.
fn speed_field(ui: &mut Ui, u: Units, mm_s: &mut f64) -> bool {
    let mut v = u.world_from_mm(*mm_s);
    let (lo, hi) = time::SPEED_RANGE;
    let changed = ui
        .add_sized(
            [90.0, 22.0],
            NumField::new(&mut v)
                .range(u.world_from_mm(lo)..=u.world_from_mm(hi))
                .max_decimals(2)
                .suffix(format!(" {}/s", u.short())),
        )
        .changed();
    if changed {
        *mm_s = u.world_to_mm(v);
    }
    changed
}

fn name_field(ui: &mut Ui, name: &mut String) {
    if ui
        .add(egui::TextEdit::singleline(name).desired_width(NAME_WIDTH))
        .changed()
        && (name.chars().count() > MAX_NAME || name.chars().any(char::is_control))
    {
        *name = name
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_NAME)
            .collect();
    }
}

/// A small delete button for a list row.
fn delete_button(ui: &mut Ui, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new("\u{2715}").small())
        .on_hover_text(tr("signcut.calc_delete"))
        .clicked()
}

/// Add, Import and Export buttons under a list.
fn list_buttons(app: &mut App, ui: &mut Ui, list: List) -> bool {
    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        add = ui.button(tr("signcut.calc_add")).clicked();
        if ui.button(tr("signcut.calc_import")).clicked() {
            app.sc_calc_import(list);
        }
        if ui.button(tr("signcut.calc_export")).clicked() {
            app.sc_calc_export(list);
        }
    });
    add
}

/// `index` after the row `removed` went: `None` when it was that row.
fn shift(index: usize, removed: usize) -> Option<usize> {
    match index.cmp(&removed) {
        std::cmp::Ordering::Less => Some(index),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(index - 1),
    }
}

fn time_block(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.calc_time"));
    let s = &mut app.sc_calc;
    ui.horizontal(|ui| {
        ui.label(tr("signcut.calc_profile"));
        let current = s
            .profiles
            .get(s.profile)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("sc_calc_profile")
            .selected_text(current)
            .width(170.0)
            .show_ui(ui, |ui| {
                for (i, p) in s.profiles.iter().enumerate() {
                    ui.selectable_value(&mut s.profile, i, p.name.as_str());
                }
            });
    });
    egui::CollapsingHeader::new(tr("signcut.calc_edit_profile"))
        .id_salt("sc_calc_profile_edit")
        .show(ui, |ui| profile_editor(ui, u, s));
    ui.horizontal(|ui| {
        ui.label(tr("signcut.calc_order"));
        ui.radio_value(
            &mut s.order,
            TravelOrder::Nearest,
            tr("signcut.calc_order_nearest"),
        );
        ui.radio_value(
            &mut s.order,
            TravelOrder::AsDrawn,
            tr("signcut.calc_order_drawn"),
        );
    });
    let Some(t) = app.sc_calc_time() else {
        return;
    };
    let p = app.sc_calc.current_profile();
    egui::Grid::new("sc_calc_time")
        .num_columns(2)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            let mut row = |key: &str, value: String| {
                ui.label(tr(key));
                ui.label(value);
                ui.end_row();
            };
            row("signcut.calc_cut_length", fmt_len(u, t.cut_length));
            row("signcut.calc_cutting", fmt_time(t.cut_s));
            row(
                "signcut.calc_travel",
                format!(
                    "{}, {}",
                    fmt_len(u, t.travel.travel_length),
                    fmt_time(t.travel_s)
                ),
            );
            row(
                "signcut.calc_lifts",
                format!(
                    "{}, {}",
                    trf(
                        "signcut.calc_lifts_value",
                        &[
                            ("n", &t.travel.outlines.to_string()),
                            ("t", &format!("{:.2}", p.lift_time)),
                        ],
                    ),
                    fmt_time(t.lift_s)
                ),
            );
            row("signcut.calc_passes", t.passes.to_string());
            ui.label(RichText::new(tr("signcut.calc_total")).strong());
            ui.label(RichText::new(fmt_time(t.total_s)).strong());
            ui.end_row();
        });
    if t.travel.as_drawn_fallback {
        dim(ui, tr("signcut.calc_fallback"));
    }
}

fn profile_editor(ui: &mut Ui, u: Units, s: &mut Settings) {
    let n = s.profiles.len();
    if let Some(p) = s.profiles.get_mut(s.profile) {
        egui::Grid::new("sc_calc_profile_grid")
            .num_columns(2)
            .spacing([8.0, 3.0])
            .show(ui, |ui| {
                ui.label(tr("signcut.calc_name"));
                name_field(ui, &mut p.name);
                ui.end_row();
                ui.label(tr("signcut.calc_cut_speed"));
                speed_field(ui, u, &mut p.cut_speed);
                ui.end_row();
                ui.label(tr("signcut.calc_travel_speed"));
                speed_field(ui, u, &mut p.travel_speed);
                ui.end_row();
                ui.label(tr("signcut.calc_lift_time"));
                ui.add_sized(
                    [90.0, 22.0],
                    NumField::new(&mut p.lift_time)
                        .range(0.0..=time::MAX_LIFT_TIME)
                        .max_decimals(3)
                        .suffix(" s"),
                );
                ui.end_row();
                ui.label(tr("signcut.calc_accel"));
                ui.add_sized(
                    [90.0, 22.0],
                    NumField::new(&mut p.accel_factor)
                        .range(time::ACCEL_RANGE.0..=time::ACCEL_RANGE.1)
                        .max_decimals(2),
                );
                ui.end_row();
                ui.label(tr("signcut.calc_passes"));
                ui.add_sized(
                    [90.0, 22.0],
                    NumField::new(&mut p.passes).range(1..=time::MAX_PASSES),
                );
                ui.end_row();
            });
    }
    ui.horizontal(|ui| {
        if ui.button(tr("signcut.calc_add")).clicked() && n < crate::signcut_calc::MAX_ROWS {
            let mut p = s.current_profile();
            p.name = tr("signcut.calc_new_profile");
            s.profiles.push(p);
            s.profile = s.profiles.len() - 1;
        }
        if ui
            .add_enabled(n > 1, egui::Button::new(tr("signcut.calc_delete")))
            .clicked()
            && s.profile < n
        {
            s.profiles.remove(s.profile);
            s.fix_indices();
        }
    });
    if s.profiles.is_empty() {
        s.profiles.push(TimeProfile::default());
        s.profile = 0;
    }
}

fn qty_unit(u: QtyUnit) -> String {
    tr(match u {
        QtyUnit::SquareMetre => "signcut.calc_qty_m2",
        QtyUnit::Metre => "signcut.calc_qty_m",
        QtyUnit::Piece => "signcut.calc_qty_piece",
        QtyUnit::Hour => "signcut.calc_qty_hour",
    })
}

fn material_unit(u: MaterialUnit) -> String {
    tr(match u {
        MaterialUnit::Area => "signcut.calc_unit_m2",
        MaterialUnit::Roll => "signcut.calc_unit_roll",
    })
}

fn service_unit(u: ServiceUnit) -> String {
    tr(match u {
        ServiceUnit::Metre => "signcut.calc_unit_m",
        ServiceUnit::SquareMetre => "signcut.calc_unit_m2",
        ServiceUnit::Piece => "signcut.calc_unit_piece",
        ServiceUnit::Hour => "signcut.calc_unit_hour",
    })
}

fn cost_block(app: &mut App, ui: &mut Ui, outdated: bool) {
    let u = app.units;
    ui.strong(tr("signcut.calc_cost"));
    dim(ui, tr("signcut.calc_cost_hint"));
    let s = &mut app.sc_calc;
    ui.horizontal(|ui| {
        ui.label(tr("signcut.calc_material"));
        let name = |m: Option<&Material>| {
            m.map_or_else(|| tr("signcut.calc_no_material"), |m| m.name.clone())
        };
        egui::ComboBox::from_id_salt("sc_calc_material")
            .selected_text(name(s.current_material()))
            .width(190.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.material, None, tr("signcut.calc_no_material"));
                for (i, m) in s.materials.iter().enumerate() {
                    ui.selectable_value(&mut s.material, Some(i), m.name.as_str());
                }
            });
    });
    egui::Grid::new("sc_calc_spec")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            ui.label(tr("signcut.calc_reserve"));
            length_field(ui, u, &mut s.spec.reserve, 0.0);
            ui.end_row();
            ui.label(tr("signcut.calc_min_size"));
            ui.horizontal(|ui| {
                length_field(ui, u, &mut s.spec.min_width, 0.0);
                ui.label("\u{d7}");
                length_field(ui, u, &mut s.spec.min_height, 0.0);
            });
            ui.end_row();
            ui.label(tr("signcut.calc_copies"));
            ui.add_sized(
                [90.0, 22.0],
                NumField::new(&mut s.spec.copies).range(1..=MAX_COPIES),
            );
            ui.end_row();
            ui.label(tr("signcut.calc_currency"));
            if ui
                .add(egui::TextEdit::singleline(&mut s.currency).desired_width(60.0))
                .changed()
            {
                s.currency = s.currency.chars().take(12).collect();
            }
            ui.end_row();
        });
    ui.label(tr("signcut.calc_services"));
    let money = |v: f64, c: &str| {
        let c = c.trim();
        if c.is_empty() {
            format!("{v:.2}")
        } else {
            format!("{v:.2} {c}")
        }
    };
    for sv in &mut s.services {
        let text = format!(
            "{} ({} {})",
            sv.name,
            money(sv.price, &s.currency),
            service_unit(sv.unit)
        );
        ui.checkbox(&mut sv.selected, text);
    }
    egui::CollapsingHeader::new(tr("signcut.calc_price_lists"))
        .id_salt("sc_calc_price_lists")
        .show(ui, |ui| {
            ui.strong(tr("signcut.calc_materials"));
            material_editor(app, ui);
            ui.add_space(4.0);
            ui.strong(tr("signcut.calc_services_list"));
            service_editor(app, ui);
        });
    let Some(c) = app.sc_calc_cost() else {
        return;
    };
    egui::Grid::new("sc_calc_cost")
        .num_columns(4)
        .striped(true)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            for key in [
                "signcut.calc_item",
                "signcut.calc_quantity",
                "signcut.calc_unit_price",
                "signcut.calc_amount",
            ] {
                ui.label(RichText::new(tr(key)).strong());
            }
            ui.end_row();
            for l in &c.lines {
                ui.label(&l.name);
                ui.label(format!("{:.3} {}", l.quantity, qty_unit(l.unit)));
                ui.label(format!("{:.2}", l.unit_price));
                ui.label(format!("{:.2}", l.amount));
                ui.end_row();
            }
            ui.label(RichText::new(tr("signcut.calc_total")).strong());
            ui.label("");
            ui.label("");
            ui.label(RichText::new(app.sc_calc_money(c.total)).strong());
            ui.end_row();
        });
    dim(
        ui,
        trf(
            "signcut.calc_billed_area",
            &[("area", &fmt_area(u, c.billed_area))],
        ),
    );
    if let Some(r) = c.roll {
        let mut text = trf("signcut.calc_roll", &[("len", &fmt_len(u, r.length))]);
        if r.turned {
            text = format!("{text}; {}", tr("signcut.calc_roll_turned"));
        }
        if r.lanes > 1 {
            text = format!(
                "{text}; {}",
                trf("signcut.calc_roll_lanes", &[("n", &r.lanes.to_string())])
            );
        }
        dim(ui, text);
    }
    // The billed blocks on the drawing while the measurement is current.
    if !outdated {
        if let Some(m) = &app.sc_calc.measured {
            let r = app.sc_calc.spec.sanitized().reserve;
            for b in &m.job.blocks {
                app.docker_preview.push(b.inflate(r, r).to_path(0.1));
            }
        }
    }
}

fn material_editor(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_calc;
    let mut removed = None;
    for (i, m) in s.materials.iter_mut().enumerate() {
        ui.push_id(("sc_calc_mat", i), |ui| {
            ui.horizontal(|ui| {
                name_field(ui, &mut m.name);
                if delete_button(ui, true) {
                    removed = Some(i);
                }
            });
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("unit")
                    .selected_text(material_unit(m.unit))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for unit in [MaterialUnit::Area, MaterialUnit::Roll] {
                            ui.selectable_value(&mut m.unit, unit, material_unit(unit));
                        }
                    });
                ui.add_sized(
                    [80.0, 22.0],
                    NumField::new(&mut m.price)
                        .range(0.0..=1.0e9)
                        .max_decimals(2),
                );
                if m.unit == MaterialUnit::Roll {
                    ui.label(tr("signcut.calc_roll_width"));
                    length_field(ui, u, &mut m.roll_width, 1.0);
                }
            });
        });
        ui.add_space(2.0);
    }
    if let Some(i) = removed {
        s.materials.remove(i);
        s.material = s.material.and_then(|m| shift(m, i));
    }
    if list_buttons(app, ui, List::Materials) {
        let s = &mut app.sc_calc;
        if s.materials.len() < crate::signcut_calc::MAX_ROWS {
            s.materials.push(Material {
                name: tr("signcut.calc_new_material"),
                unit: MaterialUnit::Area,
                price: 0.0,
                roll_width: 1000.0,
            });
        }
    }
}

fn service_editor(app: &mut App, ui: &mut Ui) {
    let s = &mut app.sc_calc;
    let mut removed = None;
    for (i, sv) in s.services.iter_mut().enumerate() {
        ui.push_id(("sc_calc_srv", i), |ui| {
            ui.horizontal(|ui| {
                name_field(ui, &mut sv.name);
                if delete_button(ui, true) {
                    removed = Some(i);
                }
            });
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("unit")
                    .selected_text(service_unit(sv.unit))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for unit in ServiceUnit::ALL {
                            ui.selectable_value(&mut sv.unit, unit, service_unit(unit));
                        }
                    });
                ui.add_sized(
                    [80.0, 22.0],
                    NumField::new(&mut sv.price)
                        .range(0.0..=1.0e9)
                        .max_decimals(2),
                );
            });
        });
        ui.add_space(2.0);
    }
    if let Some(i) = removed {
        s.services.remove(i);
    }
    if list_buttons(app, ui, List::Services) {
        let s = &mut app.sc_calc;
        if s.services.len() < crate::signcut_calc::MAX_ROWS {
            s.services.push(Service {
                name: tr("signcut.calc_new_service"),
                unit: ServiceUnit::Metre,
                price: 0.0,
                selected: true,
            });
        }
    }
}

fn weight_block(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    ui.strong(tr("signcut.calc_weight"));
    dim(ui, tr("signcut.calc_weight_hint"));
    let s = &mut app.sc_calc;
    ui.horizontal(|ui| {
        ui.label(tr("signcut.calc_material"));
        let current = s
            .current_density()
            .map(|d| format!("{} ({} g/cm\u{b3})", d.name, d.g_cm3))
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("sc_calc_density")
            .selected_text(current)
            .width(190.0)
            .show_ui(ui, |ui| {
                for (i, d) in s.densities.iter().enumerate() {
                    ui.selectable_value(&mut s.density, i, d.name.as_str());
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label(tr("signcut.calc_thickness"));
        length_field(ui, u, &mut s.thickness, THICKNESS_RANGE.0);
        s.thickness = s.thickness.clamp(THICKNESS_RANGE.0, THICKNESS_RANGE.1);
    });
    egui::CollapsingHeader::new(tr("signcut.calc_densities"))
        .id_salt("sc_calc_densities")
        .show(ui, |ui| density_editor(app, ui));
    let Some(w) = app.sc_calc_weight() else {
        return;
    };
    egui::Grid::new("sc_calc_weight")
        .num_columns(2)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            let mut row = |key: &str, value: String| {
                ui.label(tr(key));
                ui.label(value);
                ui.end_row();
            };
            row("signcut.calc_net_area", fmt_area(u, w.area));
            row(
                "signcut.calc_volume",
                format!(
                    "{:.1} cm\u{b3}  ({:.3} l)",
                    w.volume / 1000.0,
                    w.volume / 1_000_000.0
                ),
            );
            row("signcut.calc_edge_area", fmt_area(u, w.edge_area));
            ui.label(RichText::new(tr("signcut.calc_weight_value")).strong());
            ui.label(RichText::new(fmt_weight(w.weight)).strong());
            ui.end_row();
        });
}

fn density_editor(app: &mut App, ui: &mut Ui) {
    let s = &mut app.sc_calc;
    let mut removed = None;
    for (i, d) in s.densities.iter_mut().enumerate() {
        ui.push_id(("sc_calc_den", i), |ui| {
            ui.horizontal_wrapped(|ui| {
                name_field(ui, &mut d.name);
                ui.add_sized(
                    [80.0, 22.0],
                    NumField::new(&mut d.g_cm3)
                        .range(0.001..=100.0)
                        .max_decimals(3)
                        .suffix(" g/cm\u{b3}"),
                );
                if delete_button(ui, true) {
                    removed = Some(i);
                }
            });
        });
    }
    if let Some(i) = removed {
        s.densities.remove(i);
        s.density = shift(s.density, i).unwrap_or(0);
        s.fix_indices();
    }
    if list_buttons(app, ui, List::Densities) {
        let s = &mut app.sc_calc;
        if s.densities.len() < crate::signcut_calc::MAX_ROWS {
            s.densities.push(Density {
                name: tr("signcut.calc_new_density"),
                g_cm3: 1.0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn frame(ctx: &egui::Context, f: impl FnOnce(&mut Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut f = Some(f);
        let mut out = ctx.run_ui(input, |ui| {
            if let Some(f) = f.take() {
                f(ui);
            }
        });
        out.textures_delta.clear();
    }

    #[test]
    fn the_calc_section_and_its_editors_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer");
        // Before and after measuring, with a roll material.
        frame(&ctx, |ui| section(&mut app, ui));
        assert!(app.docker_preview.is_empty());
        app.sc_calc_run();
        frame(&ctx, |ui| section(&mut app, ui));
        // The billed block shows on the drawing.
        assert_eq!(app.docker_preview.len(), 1);
        frame(&ctx, |ui| {
            let u = app.units;
            profile_editor(ui, u, &mut app.sc_calc);
            material_editor(&mut app, ui);
            service_editor(&mut app, ui);
            density_editor(&mut app, ui);
        });
        // Empty lists draw too.
        app.sc_calc.materials.clear();
        app.sc_calc.services.clear();
        app.sc_calc.densities.clear();
        app.sc_calc.fix_indices();
        frame(&ctx, |ui| section(&mut app, ui));
        assert!(app.sc_calc_weight().is_none());
    }

    #[test]
    fn row_indices_follow_a_deleted_row() {
        assert_eq!(shift(0, 2), Some(0));
        assert_eq!(shift(2, 2), None);
        assert_eq!(shift(3, 2), Some(2));
    }
}
