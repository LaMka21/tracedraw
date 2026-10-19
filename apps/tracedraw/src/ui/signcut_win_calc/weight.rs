//! The Volume and Weight window: the volume and edge area for a sheet
//! thickness, the mass from a density list, and a price per mass, area
//! or volume.

use super::{calc_buttons, decimals, just_opened, num, readout, stale_note, unit_row};
use crate::app::App;
use crate::i18n::tr;
use crate::signcut_calc::{List, MassUnit, PricePer, MAX_PRICE, THICKNESS_RANGE};
use crate::ui::signcut_kit::{self as kit, Pressed};
use egui::Ui;
use tracedraw_signcut::calc::weight::Density;

/// Density (g/cm3) accepted.
const DENSITY_RANGE: (f64, f64) = (0.001, 100.0);

pub(super) fn window(app: &mut App, ui: &mut Ui) -> bool {
    if just_opened(ui, "weight") || app.sc_calc.measured.is_none() {
        app.sc_calc_measure();
    }
    let stale = app.sc_calc_outdated();
    let w = app.sc_calc_weight();
    let price = app.sc_calc_weight_price().map(|p| app.sc_calc_money(p));
    let (area, length) = app.sc_calc.measured.as_ref().map_or((0.0, 0.0), |m| {
        (m.job.measurement.area, m.job.measurement.length)
    });
    let s = &mut app.sc_calc;
    let u = s.volume_unit;
    let d = decimals(u);
    kit::group(ui, &tr("signcut.calc_volume"), |ui| {
        let (volume, edge) = w.map_or((0.0, length * s.thickness), |w| (w.volume, w.edge_area));
        kit::row(ui, &tr("signcut.calc_volume"), |ui| {
            readout(
                ui,
                &format!("{} {}", num(u.volume(volume), d + 2), u.volume_id()),
                true,
                stale,
            )
        });
        kit::row(ui, &tr("signcut.area"), |ui| {
            readout(
                ui,
                &format!("{} {}", num(u.area(area), d + 2), u.area_id()),
                false,
                stale,
            )
        });
        kit::row(ui, &tr("signcut.calc_edge_area"), |ui| {
            readout(
                ui,
                &format!("{} {}", num(u.area(edge), d + 2), u.area_id()),
                false,
                stale,
            )
        });
        kit::row(ui, &tr("signcut.wcalc_thickness"), |ui| {
            let mut t = u.len(s.thickness);
            let r = ui.add_sized(
                [kit::FIELD_W, 22.0],
                crate::ui::field::NumField::new(&mut t)
                    .range(u.len(THICKNESS_RANGE.0)..=u.len(THICKNESS_RANGE.1))
                    .max_decimals(4)
                    .suffix(format!(" {}", u.id())),
            );
            if r.changed() && t.is_finite() {
                s.thickness = (t * u.mm()).clamp(THICKNESS_RANGE.0, THICKNESS_RANGE.1);
            }
        });
        unit_row(ui, "volume", &mut s.volume_unit);
    });
    kit::group(ui, &tr("signcut.wcalc_mass"), |ui| {
        kit::row(ui, &tr("signcut.wcalc_material"), |ui| {
            let opts: Vec<(usize, String)> = s
                .densities
                .iter()
                .enumerate()
                .map(|(i, d)| (i, d.name.clone()))
                .collect();
            if opts.is_empty() {
                ui.label(tr("signcut.wcalc_no_density"));
                false
            } else {
                kit::choice(ui, "signcut_wcalc_density", &mut s.density, &opts)
            }
        });
        kit::row(ui, &tr("signcut.wcalc_density"), |ui| {
            match s.densities.get_mut(s.density) {
                Some(dn) => {
                    // Shown in kg/m3, kept in g/cm3.
                    let mut kg_m3 = dn.g_cm3 * 1000.0;
                    let r = kit::number(
                        ui,
                        &mut kg_m3,
                        DENSITY_RANGE.0 * 1000.0,
                        DENSITY_RANGE.1 * 1000.0,
                        2,
                    );
                    if r.changed() && kg_m3.is_finite() {
                        dn.g_cm3 = (kg_m3 / 1000.0).clamp(DENSITY_RANGE.0, DENSITY_RANGE.1);
                    }
                }
                None => {
                    readout(ui, "-", false, true);
                }
            }
        });
        kit::row(ui, &tr("signcut.units"), |ui| {
            let opts: Vec<(MassUnit, String)> = MassUnit::ALL
                .iter()
                .map(|m| (*m, m.id().to_string()))
                .collect();
            kit::choice(ui, "signcut_wcalc_mass_unit", &mut s.mass_unit, &opts)
        });
        let mass = w.map_or_else(
            || "-".to_string(),
            |w| {
                format!(
                    "{} {}",
                    num(w.weight / s.mass_unit.grams(), 6),
                    s.mass_unit.id()
                )
            },
        );
        kit::row(ui, &tr("signcut.wcalc_mass"), |ui| {
            readout(ui, &mass, true, stale)
        });
        ui.add_space(2.0);
        if ui
            .add_sized(
                [ui.available_width(), 24.0],
                egui::Button::new(tr("signcut.wcalc_density_editor")).selected(s.density_editor),
            )
            .clicked()
        {
            s.density_editor = !s.density_editor;
        }
    });
    if app.sc_calc.density_editor {
        kit::group(ui, &tr("signcut.calc_densities"), |ui| {
            density_editor(app, ui)
        });
    }
    let s = &mut app.sc_calc;
    kit::group(ui, &tr("signcut.wcalc_price"), |ui| {
        kit::row(ui, &tr("signcut.calc_cost"), |ui| {
            kit::number(ui, &mut s.weight_price, 0.0, MAX_PRICE, 4)
        });
        kit::row(ui, &tr("signcut.wcalc_cost_per"), |ui| {
            let opts: Vec<(PricePer, String)> = PricePer::ALL
                .iter()
                .map(|p| {
                    (
                        *p,
                        match p {
                            PricePer::Mass => s.mass_unit.id().to_string(),
                            PricePer::SquareMetre => "m\u{b2}".to_string(),
                            PricePer::CubicMetre => "m\u{b3}".to_string(),
                        },
                    )
                })
                .collect();
            kit::choice(ui, "signcut_wcalc_price_per", &mut s.weight_per, &opts)
        });
        kit::row(ui, &tr("signcut.wcalc_price"), |ui| {
            readout(ui, price.as_deref().unwrap_or("-"), true, stale)
        });
    });
    stale_note(ui, stale);
    match calc_buttons(ui, |_| {}) {
        Some(Pressed::Update) => {
            app.sc_calc_measure();
            false
        }
        Some(Pressed::Close) => true,
        _ => false,
    }
}

/// The density list: a row per material (name, g/cm3, delete), Add,
/// Import and Export.
fn density_editor(app: &mut App, ui: &mut Ui) {
    let s = &mut app.sc_calc;
    let mut removed = None;
    egui::ScrollArea::vertical()
        .id_salt("signcut_wcalc_densities")
        .max_height(160.0)
        .show(ui, |ui| {
            for (i, d) in s.densities.iter_mut().enumerate() {
                ui.push_id(("signcut_wcalc_den", i), |ui| {
                    ui.horizontal(|ui| {
                        super::cost::name_field(ui, &mut d.name, 120.0);
                        ui.add_sized(
                            [80.0, 22.0],
                            crate::ui::field::NumField::new(&mut d.g_cm3)
                                .range(DENSITY_RANGE.0..=DENSITY_RANGE.1)
                                .max_decimals(3)
                                .suffix(" g/cm\u{b3}"),
                        );
                        if super::cost::delete_button(ui) {
                            removed = Some(i);
                        }
                    });
                });
            }
        });
    if let Some(i) = removed {
        s.densities.remove(i);
        s.density = super::cost::shift(s.density, i).unwrap_or(0);
        s.fix_indices();
    }
    if super::cost::list_buttons(app, ui, List::Densities) {
        let s = &mut app.sc_calc;
        if s.densities.len() < crate::signcut_calc::MAX_ROWS {
            s.densities.push(Density {
                name: tr("signcut.calc_new_density"),
                g_cm3: 1.0,
            });
            s.density = s.densities.len() - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{app_with_rects, frame};
    use super::*;

    #[test]
    fn the_weight_window_draws_with_and_without_densities() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_rects();
        app.sc_calc.density_editor = true;
        frame(&ctx, |ui| {
            window(&mut app, ui);
        });
        assert!(app.sc_calc_weight().is_some());
        app.sc_calc.densities.clear();
        app.sc_calc.fix_indices();
        frame(&ctx, |ui| {
            window(&mut app, ui);
        });
        assert!(app.sc_calc_weight().is_none());
    }
}
