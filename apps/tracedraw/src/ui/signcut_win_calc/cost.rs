//! The Cost window: a table with a row per Add (the selected objects,
//! or the page), its material and services from the price lists and the
//! totals; under it, on request, the material and service lists and the
//! job settings.

use super::{num, tabs};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_calc::{CostLists, List, MAX_NAME, MAX_PRICE};
use crate::theme::Tokens;
use crate::ui::signcut_kit::{self as kit, Pressed};
use egui::{Color32, RichText, Sense, Stroke, Ui, Vec2};
use tracedraw_core::geometry::Shape as _;
use tracedraw_signcut::calc::cost::{
    Material, MaterialUnit, QtyUnit, Service, ServiceUnit, MAX_COPIES,
};

/// A list row's name field: one line, at most [`MAX_NAME`] characters.
pub(super) fn name_field(ui: &mut Ui, name: &mut String, width: f32) {
    if ui
        .add(egui::TextEdit::singleline(name).desired_width(width))
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
pub(super) fn delete_button(ui: &mut Ui) -> bool {
    ui.add(egui::Button::new("\u{2715}").small())
        .on_hover_text(tr("signcut.calc_delete"))
        .clicked()
}

/// Add, Import and Export buttons under a list; true when Add was
/// pressed.
pub(super) fn list_buttons(app: &mut App, ui: &mut Ui, list: List) -> bool {
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
pub(super) fn shift(index: usize, removed: usize) -> Option<usize> {
    match index.cmp(&removed) {
        std::cmp::Ordering::Less => Some(index),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(index - 1),
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

/// What a click in the table asks for.
enum RowAction {
    Select(usize),
    Delete(usize),
}

pub(super) fn window(app: &mut App, ui: &mut Ui) -> bool {
    let lists_h = if app.sc_calc.cost_lists == CostLists::Hidden {
        0.0
    } else {
        230.0
    };
    let table_h = (ui.available_height() - lists_h - 44.0).max(110.0);
    egui::ScrollArea::both()
        .id_salt("signcut_wcalc_cost_table")
        .max_height(table_h)
        .min_scrolled_height(table_h.min(160.0))
        .auto_shrink([false, true])
        .show(ui, |ui| table(app, ui));
    if app.sc_calc.cost_lists != CostLists::Hidden {
        ui.add_space(4.0);
        lists(app, ui);
    }
    // The billed blocks of the rows that are current, on the drawing.
    let rev = app.engine.revision();
    let r = app.sc_calc.spec.sanitized().reserve;
    for row in app
        .sc_calc
        .cost_rows
        .iter()
        .filter(|row| row.revision == rev)
    {
        for b in &row.job.blocks {
            app.docker_preview.push(b.inflate(r, r).to_path(0.1));
        }
    }
    let shown = app.sc_calc.cost_lists != CostLists::Hidden;
    let mut add = false;
    let mut toggle = false;
    let pressed = super::calc_buttons(ui, |ui| {
        add = kit::push(ui, &tr("signcut.calc_add"), true);
        toggle = ui
            .add(
                egui::Button::new(tr("signcut.calc_price_lists"))
                    .min_size(Vec2::new(86.0, 24.0))
                    .selected(shown),
            )
            .clicked();
    });
    if add {
        app.sc_calc_add_row();
    }
    if toggle {
        app.sc_calc.cost_lists = if shown {
            CostLists::Hidden
        } else {
            CostLists::Materials
        };
    }
    match pressed {
        Some(Pressed::Update) => {
            app.sc_calc_update_rows();
            false
        }
        Some(Pressed::Close) => true,
        _ => false,
    }
}

/// The table: a row per Add and the totals.
fn table(app: &mut App, ui: &mut Ui) {
    let rev = app.engine.revision();
    let costs: Vec<_> = app
        .sc_calc
        .cost_rows
        .iter()
        .map(|r| app.sc_calc_row_cost(r))
        .collect();
    let money = |v: f64| app.sc_calc_money(v);
    let totals: Vec<String> = {
        let m: f64 = costs.iter().map(|c| c.material_cost).sum();
        let s: f64 = costs.iter().map(|c| c.services_cost).sum();
        let t: f64 = costs.iter().map(|c| c.report.total).sum();
        vec![money(m), money(s), money(t)]
    };
    let row_money: Vec<[String; 3]> = costs
        .iter()
        .map(|c| {
            [
                money(c.material_cost),
                money(c.services_cost),
                money(c.report.total),
            ]
        })
        .collect();
    // The material quantity summed when every row has the same unit.
    let qty_total = {
        let units: Vec<Option<QtyUnit>> = costs.iter().map(|c| c.material_unit).collect();
        match units.first() {
            Some(Some(u)) if units.iter().all(|x| *x == Some(*u)) => Some(format!(
                "{} {}",
                num(costs.iter().map(|c| c.material_qty).sum::<f64>(), 3),
                qty_unit(*u)
            )),
            _ => None,
        }
    };
    let s = &mut app.sc_calc;
    let services: Vec<String> = s
        .services
        .iter()
        .filter(|sv| sv.selected)
        .map(|sv| sv.name.clone())
        .collect();
    let services = if services.is_empty() {
        tr("signcut.wcalc_no_services")
    } else {
        services.join(", ")
    };
    let materials: Vec<(Option<usize>, String)> =
        std::iter::once((None, tr("signcut.calc_no_material")))
            .chain(
                s.materials
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (Some(i), m.name.clone())),
            )
            .collect();
    let mut action = None;
    // A material picked in a row is the one new rows take.
    let mut chosen: Option<Option<usize>> = None;
    let head = |ui: &mut Ui, key: &str| {
        ui.label(RichText::new(tr(key)).strong());
    };
    egui::Grid::new("signcut_wcalc_cost_grid")
        .num_columns(9)
        .striped(true)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            for key in [
                "signcut.wcalc_col_no",
                "signcut.calc_name",
                "signcut.wcalc_material",
                "signcut.wcalc_col_mat_qty",
                "signcut.wcalc_col_mat_cost",
                "signcut.calc_services_list",
                "signcut.wcalc_col_srv_cost",
                "signcut.calc_total",
            ] {
                head(ui, key);
            }
            ui.label("");
            ui.end_row();
            for (i, row) in s.cost_rows.iter_mut().enumerate() {
                let Some(c) = costs.get(i) else { continue };
                ui.horizontal(|ui| {
                    swatch(ui, row.colour);
                    let current = row.revision == rev;
                    let text = RichText::new((i + 1).to_string());
                    let text = if current {
                        text
                    } else {
                        text.color(Tokens::text_dim())
                    };
                    let hint = if current {
                        tr("signcut.wcalc_row_select")
                    } else {
                        tr("signcut.wcalc_outdated")
                    };
                    if ui
                        .add(egui::Button::new(text).frame(false))
                        .on_hover_text(hint)
                        .clicked()
                    {
                        action = Some(RowAction::Select(i));
                    }
                });
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut row.name)
                            .id_salt(("signcut_wcalc_row_name", i))
                            .desired_width(110.0),
                    )
                    .changed()
                {
                    row.name = row
                        .name
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(MAX_NAME)
                        .collect();
                }
                egui::ComboBox::from_id_salt(("signcut_wcalc_row_mat", i))
                    .selected_text(
                        materials
                            .iter()
                            .find(|(k, _)| *k == row.material)
                            .map(|(_, n)| n.clone())
                            .unwrap_or_default(),
                    )
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for (k, n) in &materials {
                            if ui.selectable_value(&mut row.material, *k, n).clicked() {
                                chosen = Some(*k);
                            }
                        }
                    });
                ui.label(match c.material_unit {
                    Some(u) => format!("{} {}", num(c.material_qty, 3), qty_unit(u)),
                    None => "-".into(),
                });
                let m = row_money.get(i).cloned().unwrap_or_default();
                ui.label(&m[0]);
                ui.add(egui::Label::new(&services).truncate())
                    .on_hover_text(&services);
                ui.label(&m[1]);
                ui.label(RichText::new(&m[2]).strong());
                if delete_button(ui) {
                    action = Some(RowAction::Delete(i));
                }
                ui.end_row();
            }
            if s.cost_rows.is_empty() {
                ui.label("");
                ui.label(RichText::new(tr("signcut.wcalc_cost_empty")).color(Tokens::text_dim()));
                ui.end_row();
            } else {
                ui.label("");
                ui.label(RichText::new(tr("signcut.calc_total")).strong());
                ui.label("");
                ui.label(qty_total.unwrap_or_default());
                ui.label(RichText::new(&totals[0]).strong());
                ui.label("");
                ui.label(RichText::new(&totals[1]).strong());
                ui.label(RichText::new(&totals[2]).strong());
                ui.end_row();
            }
        });
    if let Some(m) = chosen {
        app.sc_calc.material = m;
    }
    match action {
        Some(RowAction::Select(i)) => {
            if let Some(r) = app.sc_calc.cost_rows.get(i) {
                let ids: Vec<_> = r
                    .targets
                    .iter()
                    .copied()
                    .filter(|id| app.doc().find_shape(*id).is_some())
                    .collect();
                app.select(ids);
            }
        }
        Some(RowAction::Delete(i)) if i < app.sc_calc.cost_rows.len() => {
            app.sc_calc.cost_rows.remove(i);
        }
        _ => {}
    }
}

/// The colour swatch of a row (a crossed box when the objects have no
/// plain fill).
fn swatch(ui: &mut Ui, colour: Option<[u8; 3]>) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    let p = ui.painter();
    match colour {
        Some([red, g, b]) => {
            p.rect_filled(r, 0.0, Color32::from_rgb(red, g, b));
        }
        None => {
            p.line_segment(
                [r.left_top(), r.right_bottom()],
                Stroke::new(1.0, Tokens::text_dim()),
            );
        }
    }
    p.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Inside,
    );
}

/// The lists under the table: materials, services, job settings.
fn lists(app: &mut App, ui: &mut Ui) {
    let mut tab = app.sc_calc.cost_lists;
    tabs(
        ui,
        &mut tab,
        &[
            (CostLists::Materials, tr("signcut.calc_materials")),
            (CostLists::Services, tr("signcut.calc_services_list")),
            (CostLists::Job, tr("signcut.wcalc_job")),
        ],
    );
    app.sc_calc.cost_lists = tab;
    egui::ScrollArea::vertical()
        .id_salt("signcut_wcalc_cost_lists")
        .max_height(190.0)
        .auto_shrink([false, true])
        .show(ui, |ui| match tab {
            CostLists::Materials => material_editor(app, ui),
            CostLists::Services => service_editor(app, ui),
            CostLists::Job => job_settings(app, ui),
            CostLists::Hidden => {}
        });
}

fn material_editor(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_calc;
    let mut removed = None;
    for (i, m) in s.materials.iter_mut().enumerate() {
        ui.push_id(("signcut_wcalc_mat", i), |ui| {
            ui.horizontal(|ui| {
                name_field(ui, &mut m.name, 220.0);
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
                    crate::ui::field::NumField::new(&mut m.price)
                        .range(0.0..=MAX_PRICE)
                        .max_decimals(2),
                );
                if m.unit == MaterialUnit::Roll {
                    ui.label(tr("signcut.calc_roll_width"));
                    kit::length(ui, u, &mut m.roll_width, 1.0, 100_000.0);
                }
                if delete_button(ui) {
                    removed = Some(i);
                }
            });
        });
    }
    if let Some(i) = removed {
        s.remove_material(i);
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
        ui.push_id(("signcut_wcalc_srv", i), |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut sv.selected, "")
                    .on_hover_text(tr("signcut.wcalc_billed"));
                name_field(ui, &mut sv.name, 200.0);
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
                    crate::ui::field::NumField::new(&mut sv.price)
                        .range(0.0..=MAX_PRICE)
                        .max_decimals(2),
                );
                if delete_button(ui) {
                    removed = Some(i);
                }
            });
        });
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

/// The job settings: reserve, minimum billed size, copies, currency.
fn job_settings(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_calc;
    ui.set_max_width(kit::SIDE_W + 40.0);
    kit::row(ui, &tr("signcut.calc_reserve"), |ui| {
        kit::length(ui, u, &mut s.spec.reserve, 0.0, 10_000.0)
    });
    kit::row(ui, &tr("signcut.wcalc_min_width"), |ui| {
        kit::length(ui, u, &mut s.spec.min_width, 0.0, 100_000.0)
    });
    kit::row(ui, &tr("signcut.wcalc_min_height"), |ui| {
        kit::length(ui, u, &mut s.spec.min_height, 0.0, 100_000.0)
    });
    kit::row(ui, &tr("signcut.calc_copies"), |ui| {
        let mut c = s.spec.copies as usize;
        let r = kit::count(ui, &mut c, 1, MAX_COPIES as usize);
        if r.changed() {
            s.spec.copies = (c as u32).clamp(1, MAX_COPIES);
        }
        r
    });
    kit::row(ui, &tr("signcut.calc_currency"), |ui| {
        if ui
            .add_sized(
                [kit::FIELD_W, 22.0],
                egui::TextEdit::singleline(&mut s.currency),
            )
            .changed()
        {
            s.currency = s.currency.chars().take(12).collect();
        }
    });
    ui.label(
        RichText::new(trf(
            "signcut.wcalc_cost_hint",
            &[("profile", &s.current_profile().name)],
        ))
        .size(11.0)
        .color(Tokens::text_dim()),
    );
}

#[cfg(test)]
mod tests {
    use super::super::tests::{app_with_rects, frame};
    use super::*;

    #[test]
    fn the_cost_window_draws_its_table_and_every_list() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_rects();
        let rev = app.engine.revision();
        // Empty, then with a row.
        frame(&ctx, |ui| {
            window(&mut app, ui);
        });
        assert!(app.sc_calc.cost_rows.is_empty());
        assert!(app.sc_calc_add_row());
        assert!(app.sc_calc_add_row());
        for lists in [
            CostLists::Hidden,
            CostLists::Materials,
            CostLists::Services,
            CostLists::Job,
        ] {
            app.sc_calc.cost_lists = lists;
            app.docker_preview.clear();
            frame(&ctx, |ui| {
                window(&mut app, ui);
            });
        }
        // The billed blocks of both rows (two objects each) on the drawing.
        assert_eq!(app.docker_preview.len(), 4);
        assert_eq!(app.engine.revision(), rev);
        // Empty lists draw too.
        app.sc_calc.materials.clear();
        app.sc_calc.services.clear();
        app.sc_calc.fix_indices();
        frame(&ctx, |ui| {
            window(&mut app, ui);
        });
        assert!(app.sc_calc.cost_rows.iter().all(|r| r.material.is_none()));
    }

    #[test]
    fn row_indices_follow_a_deleted_row() {
        assert_eq!(shift(0, 2), Some(0));
        assert_eq!(shift(2, 2), None);
        assert_eq!(shift(3, 2), Some(2));
    }
}
