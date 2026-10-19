//! The Cutting Time window: tabs Time (profile, estimated time, the
//! share spent cutting, price per hour and the total), Parameters (the
//! profile's values and the travel order) and Preview (the outlines and
//! the moves between them).

use super::{calc_buttons, just_opened, num, readout, readout_w, stale_note, tabs};
use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::signcut_calc::{TimeTab, MAX_PRICE};
use crate::ui::field::NumField;
use crate::ui::signcut_kit::{self as kit, colors, Pressed, Scene};
use egui::{Color32, Ui, Vec2};
use tracedraw_core::geometry::BezPath;
use tracedraw_signcut::calc::time::{self, TimeProfile, TravelOrder};

/// Seconds as "1 h 2 min 5 s".
pub(super) fn fmt_hms(seconds: f64) -> String {
    let (h, m, s) = time::hms(seconds);
    trf(
        "signcut.wcalc_hms",
        &[
            ("h", &h.to_string()),
            ("m", &m.to_string()),
            ("s", &s.to_string()),
        ],
    )
}

pub(super) fn window(app: &mut App, ui: &mut Ui) -> bool {
    if just_opened(ui, "cutting_time") || app.sc_calc.measured.is_none() {
        app.sc_calc_measure();
    }
    let mut tab = app.sc_calc.time_tab;
    tabs(
        ui,
        &mut tab,
        &[
            (TimeTab::Time, tr("signcut.wcalc_tab_time")),
            (TimeTab::Parameters, tr("signcut.wcalc_tab_parameters")),
            (TimeTab::Preview, tr("signcut.wcalc_tab_preview")),
        ],
    );
    app.sc_calc.time_tab = tab;
    let stale = app.sc_calc_outdated();
    match tab {
        TimeTab::Time => time_tab(app, ui, stale),
        TimeTab::Parameters => parameters(app, ui),
        TimeTab::Preview => preview(app, ui, stale),
    }
    stale_note(ui, stale);
    let share = app
        .sc_calc_time()
        .filter(|t| t.total_s > 0.0)
        .map_or(0.0, |t| t.cut_s / t.total_s);
    ui.add_space(4.0);
    kit::progress(ui, share as f32);
    match calc_buttons(ui, |_| {}) {
        Some(Pressed::Update) => {
            app.sc_calc_measure();
            false
        }
        Some(Pressed::Close) => true,
        _ => false,
    }
}

/// The profile list, with a button that opens its values.
fn profile_row(app: &mut App, ui: &mut Ui) {
    let s = &mut app.sc_calc;
    ui.horizontal(|ui| {
        let current = s
            .profiles
            .get(s.profile)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("signcut_wcalc_profile")
            .selected_text(egui::RichText::new(current).strong())
            .width((ui.available_width() - 44.0).max(120.0))
            .show_ui(ui, |ui| {
                for (i, p) in s.profiles.iter().enumerate() {
                    ui.selectable_value(&mut s.profile, i, p.name.as_str());
                }
            });
        if ui
            .add(egui::Button::new("\u{2630}").min_size(Vec2::new(30.0, 24.0)))
            .on_hover_text(tr("signcut.wcalc_tab_parameters"))
            .clicked()
        {
            s.time_tab = TimeTab::Parameters;
        }
    });
}

fn time_tab(app: &mut App, ui: &mut Ui, stale: bool) {
    profile_row(app, ui);
    ui.add_space(4.0);
    let t = app.sc_calc_time();
    let price = app.sc_calc_time_price();
    let money = |v: f64| app.sc_calc_money(v);
    let total = price.map(|(p, _)| money(p));
    let per_m2 = price
        .and_then(|(_, m)| m)
        .map(|v| trf("signcut.wcalc_per_m2", &[("v", &money(v))]));
    kit::row(ui, &tr("signcut.wcalc_estimated"), |ui| {
        readout(
            ui,
            &t.map_or_else(|| "-".into(), |t| fmt_hms(t.total_s)),
            true,
            stale,
        )
    });
    kit::row(ui, &tr("signcut.wcalc_efficiency"), |ui| {
        let e = t.filter(|t| t.total_s > 0.0).map_or_else(
            || "-".into(),
            |t| format!("{:.0} %", t.cut_s / t.total_s * 100.0),
        );
        readout(ui, &e, false, stale)
    });
    let s = &mut app.sc_calc;
    ui.horizontal(|ui| {
        ui.add_sized(
            [kit::LABEL_W * 0.75, 22.0],
            egui::Label::new(tr("signcut.wcalc_price_hour")).truncate(),
        );
        ui.add_sized(
            [kit::FIELD_W * 0.8, 22.0],
            NumField::new(&mut s.price_per_hour)
                .range(0.0..=MAX_PRICE)
                .max_decimals(2),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            readout_w(
                ui,
                kit::FIELD_W,
                per_m2.as_deref().unwrap_or("-"),
                false,
                stale,
            );
        });
    });
    kit::row(ui, &tr("signcut.calc_total"), |ui| {
        readout(ui, total.as_deref().unwrap_or("-"), true, stale)
    });
}

/// A speed field in document units per second, holding mm/s.
fn speed(ui: &mut Ui, u: Units, mm_s: &mut f64) {
    let mut v = u.world_from_mm(*mm_s);
    let (lo, hi) = time::SPEED_RANGE;
    if ui
        .add_sized(
            [kit::FIELD_W, 22.0],
            NumField::new(&mut v)
                .range(u.world_from_mm(lo)..=u.world_from_mm(hi))
                .max_decimals(2)
                .suffix(format!(" {}/s", u.short())),
        )
        .changed()
        && v.is_finite()
    {
        *mm_s = u.world_to_mm(v).clamp(lo, hi);
    }
}

/// The values of the profile in use, Add and Delete, the travel order.
fn parameters(app: &mut App, ui: &mut Ui) {
    profile_row(app, ui);
    let u = app.units;
    let s = &mut app.sc_calc;
    let n = s.profiles.len();
    if let Some(p) = s.profiles.get_mut(s.profile) {
        kit::row(ui, &tr("signcut.calc_name"), |ui| {
            super::cost::name_field(ui, &mut p.name, kit::FIELD_W);
        });
        kit::row(ui, &tr("signcut.calc_cut_speed"), |ui| {
            speed(ui, u, &mut p.cut_speed)
        });
        kit::row(ui, &tr("signcut.calc_travel_speed"), |ui| {
            speed(ui, u, &mut p.travel_speed)
        });
        kit::row(ui, &tr("signcut.calc_lift_time"), |ui| {
            ui.add_sized(
                [kit::FIELD_W, 22.0],
                NumField::new(&mut p.lift_time)
                    .range(0.0..=time::MAX_LIFT_TIME)
                    .max_decimals(3)
                    .suffix(" s"),
            )
        });
        kit::row(ui, &tr("signcut.calc_accel"), |ui| {
            kit::number(
                ui,
                &mut p.accel_factor,
                time::ACCEL_RANGE.0,
                time::ACCEL_RANGE.1,
                2,
            )
        });
        kit::row(ui, &tr("signcut.calc_passes"), |ui| {
            let mut v = p.passes as usize;
            let r = kit::count(ui, &mut v, 1, time::MAX_PASSES as usize);
            if r.changed() {
                p.passes = (v as u32).clamp(1, time::MAX_PASSES);
            }
            r
        });
    }
    kit::row(ui, &tr("signcut.wcalc_order"), |ui| {
        let opts = [
            (TravelOrder::Nearest, tr("signcut.calc_order_nearest")),
            (TravelOrder::AsDrawn, tr("signcut.calc_order_drawn")),
        ];
        kit::choice(ui, "signcut_wcalc_order", &mut s.order, &opts)
    });
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

/// The outlines measured, the moves between them in red, and the parts
/// of the time.
fn preview(app: &mut App, ui: &mut Ui, stale: bool) {
    let mut scene = Scene::default();
    let u = app.units;
    if let Some(m) = &app.sc_calc.measured {
        for p in &m.paths {
            scene.add(p.clone(), colors::PART, 1.0);
        }
        for (a, b) in &m.job.moves {
            let mut l = BezPath::new();
            l.move_to(*a);
            l.line_to(*b);
            scene.add(l, colors::SHEET_EDGE, 1.0);
        }
        // The start point, a small cross at the page's origin.
        let s = (m
            .job
            .measurement
            .bbox
            .map_or(10.0, |b| b.width().max(b.height()))
            / 60.0)
            .max(0.5);
        let mut c = BezPath::new();
        c.move_to((-s, 0.0));
        c.line_to((s, 0.0));
        c.move_to((0.0, -s));
        c.line_to((0.0, s));
        scene.add(c, Color32::from_rgb(0x06, 0xB0, 0x25), 2.0);
    }
    let h = (ui.available_height() - 150.0).max(120.0);
    kit::preview(ui, Vec2::new(ui.available_width(), h), &scene);
    let Some(t) = app.sc_calc_time() else {
        return;
    };
    let len = |mm: f64| crate::ui::signcut_docker::fmt_len(u, mm);
    for (key, value) in [
        ("signcut.calc_cut_length", len(t.cut_length)),
        ("signcut.calc_cutting", fmt_hms(t.cut_s)),
        (
            "signcut.calc_travel",
            format!("{}, {}", len(t.travel.travel_length), fmt_hms(t.travel_s)),
        ),
        (
            "signcut.calc_lifts",
            format!(
                "{}, {}",
                num(t.travel.outlines as f64, 0),
                fmt_hms(t.lift_s)
            ),
        ),
    ] {
        kit::row(ui, &tr(key), |ui| {
            readout_w(ui, kit::FIELD_W + 60.0, &value, false, stale)
        });
    }
    if t.travel.as_drawn_fallback {
        ui.label(
            egui::RichText::new(tr("signcut.calc_fallback"))
                .size(11.0)
                .color(crate::theme::Tokens::text_dim()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{app_with_rects, frame};
    use super::*;

    #[test]
    fn the_time_window_draws_every_tab() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_rects();
        for tab in [TimeTab::Time, TimeTab::Parameters, TimeTab::Preview] {
            app.sc_calc.time_tab = tab;
            frame(&ctx, |ui| {
                window(&mut app, ui);
            });
            assert_eq!(app.sc_calc.time_tab, tab);
        }
        let m = app.sc_calc.measured.as_ref().expect("measured");
        assert_eq!(m.paths.len(), 2);
        assert_eq!(m.job.moves.len(), 2);
        // No profile left: one comes back.
        app.sc_calc.profiles.clear();
        app.sc_calc.time_tab = TimeTab::Parameters;
        frame(&ctx, |ui| {
            window(&mut app, ui);
        });
        assert_eq!(app.sc_calc.profiles.len(), 1);
    }

    #[test]
    fn times_read_as_hours_minutes_seconds() {
        assert_eq!(fmt_hms(3725.0), "1 h 2 min 5 s");
        assert_eq!(fmt_hms(f64::NAN), "0 h 0 min 0 s");
    }
}
