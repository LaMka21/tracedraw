//! The Perimeter and Area windows: the outline length and the areas of
//! the selection (or the page) in the window's unit, measured when the
//! window opens and with Update.

use super::{
    calc_buttons, decimals, just_opened, num, readout, readout_w, stale_note, strong_row, unit_row,
};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_calc::{CalcUnit, MAX_PRICE};
use crate::signcut_tools::ToolId;
use crate::theme::Tokens;
use crate::ui::signcut_kit::{self as kit, Pressed};
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use tracedraw_signcut::measure::Measurement;

/// The measurement to show, taken when the window opens (or first
/// needed), and whether it is out of date.
fn measured(app: &mut App, ui: &Ui, key: &str) -> (Measurement, bool) {
    if just_opened(ui, key) || app.signcut.measured.is_none() {
        app.signcut_measure();
    }
    let stale = app.signcut_measure_outdated();
    let m = app.signcut.measured.map(|(_, m)| m).unwrap_or_default();
    (m, stale)
}

/// Close on Close, measure again on Update.
fn buttons(app: &mut App, ui: &mut Ui) -> bool {
    match calc_buttons(ui, |_| {}) {
        Some(Pressed::Update) => {
            app.signcut_measure();
            false
        }
        Some(Pressed::Close) => true,
        _ => false,
    }
}

fn len_text(u: CalcUnit, mm: f64) -> String {
    format!("{} {}", num(u.len(mm), decimals(u)), u.id())
}

fn area_text(u: CalcUnit, mm2: f64) -> String {
    format!("{} {}", num(u.area(mm2), decimals(u)), u.area_id())
}

/// Perimeter: the outline length, a price per unit of length and the
/// cost (length x price).
pub(super) fn perimeter(app: &mut App, ui: &mut Ui) -> bool {
    let (m, stale) = measured(app, ui, "perimeter");
    let mut more = false;
    let money_of = |app: &App, v: f64| app.sc_calc_money(v);
    let u = app.sc_calc.perimeter_unit;
    let len = u.len(m.length);
    let cost = money_of(app, len * app.sc_calc.perimeter_price.clamp(0.0, MAX_PRICE));
    let s = &mut app.sc_calc;
    kit::group(ui, &tr("signcut.wcalc_perimeter"), |ui| {
        strong_row(ui, &tr("signcut.wcalc_perimeter"), |ui| {
            readout(ui, &len_text(u, m.length), true, stale)
        });
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 60.0).max(20.0);
            let (r, _) = ui.allocate_exact_size(egui::vec2(w, 12.0), Sense::hover());
            ui.painter().hline(
                r.x_range(),
                r.center().y,
                Stroke::new(1.0, Tokens::border()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .link(tr("signcut.wcalc_more"))
                    .on_hover_text(tr("signcut.tool_area"))
                    .clicked()
                {
                    more = true;
                }
            });
        });
        kit::row(
            ui,
            &trf("signcut.wcalc_price_per", &[("unit", u.id())]),
            |ui| kit::number(ui, &mut s.perimeter_price, 0.0, MAX_PRICE, 2),
        );
        kit::row(ui, &tr("signcut.calc_cost"), |ui| {
            readout(ui, &cost, false, stale)
        });
        unit_row(ui, "perimeter", &mut s.perimeter_unit);
    });
    stale_note(ui, stale);
    if more {
        app.signcut_tool(ToolId::Area);
    }
    buttons(app, ui)
}

/// Which area a row of the Area window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaKind {
    Object,
    Outer,
    Holes,
    BoundBox,
    Perimeter,
}

/// Area: the net, outer, hole and bounding box areas and the perimeter,
/// each with a picture of what it counts.
pub(super) fn area(app: &mut App, ui: &mut Ui) -> bool {
    let (m, stale) = measured(app, ui, "area");
    let s = &mut app.sc_calc;
    let u = s.area_unit;
    kit::group(ui, &tr("signcut.area"), |ui| {
        egui::Frame::new()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, Tokens::border()))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let bbox = m.bbox.map(|b| {
                    format!(
                        "{} \u{d7} {}",
                        len_text(u, b.width().abs()),
                        len_text(u, b.height().abs())
                    )
                });
                for (kind, key, value, note) in [
                    (
                        AreaKind::Object,
                        "signcut.wcalc_object_area",
                        area_text(u, m.area),
                        None,
                    ),
                    (
                        AreaKind::Outer,
                        "signcut.outer_area",
                        area_text(u, m.outer_area),
                        None,
                    ),
                    (
                        AreaKind::Holes,
                        "signcut.wcalc_inner_area",
                        area_text(u, m.hole_area),
                        None,
                    ),
                    (
                        AreaKind::BoundBox,
                        "signcut.bbox_area",
                        area_text(u, m.bbox_area),
                        bbox.clone(),
                    ),
                    (
                        AreaKind::Perimeter,
                        "signcut.wcalc_perimeter",
                        len_text(u, m.length),
                        None,
                    ),
                ] {
                    area_row(ui, kind, &tr(key), &value, note.as_deref(), stale);
                }
            });
        ui.add_space(4.0);
        kit::row(ui, &tr("signcut.objects"), |ui| {
            readout_w(ui, kit::FIELD_W, &m.objects.to_string(), false, stale)
        });
        unit_row(ui, "area", &mut s.area_unit);
    });
    stale_note(ui, stale);
    buttons(app, ui)
}

/// One row of the Area list: the picture, the name and the value.
fn area_row(
    ui: &mut Ui,
    kind: AreaKind,
    label: &str,
    value: &str,
    note: Option<&str>,
    stale: bool,
) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
        area_picture(ui.painter(), r, kind);
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(RichText::new(label).color(Color32::from_gray(30)));
            let color = if stale {
                Color32::from_gray(140)
            } else {
                Color32::BLACK
            };
            ui.label(RichText::new(value).strong().color(color));
            if let Some(n) = note {
                ui.label(RichText::new(n).size(10.0).color(Color32::from_gray(90)));
            }
        });
    });
    ui.add_space(2.0);
}

/// The picture of a row: a plate with a round hole (and a small part
/// beside it), filled where the row counts.
fn area_picture(p: &Painter, r: Rect, kind: AreaKind) {
    let fill = Color32::from_rgb(0x8F, 0xB8, 0xEE);
    let line = Stroke::new(1.0, Color32::from_gray(40));
    p.rect(
        r,
        CornerRadius::ZERO,
        Color32::WHITE,
        Stroke::new(1.0, Color32::from_gray(60)),
        StrokeKind::Inside,
    );
    let plate = Rect::from_min_max(r.min + egui::vec2(5.0, 7.0), r.min + egui::vec2(27.0, 33.0));
    let part = Rect::from_min_max(
        r.min + egui::vec2(29.0, 22.0),
        r.min + egui::vec2(35.0, 33.0),
    );
    let hole_c = Pos2::new(plate.center().x, plate.center().y);
    let hole_r = 5.5;
    if kind == AreaKind::BoundBox {
        let b = Rect::from_min_max(plate.min, part.max.max(plate.max));
        p.rect_filled(b, CornerRadius::ZERO, fill);
    }
    let round = CornerRadius::same(5);
    let outer_fill = matches!(kind, AreaKind::Object | AreaKind::Outer);
    if outer_fill {
        p.rect_filled(plate, round, fill);
        p.rect_filled(part, CornerRadius::ZERO, fill);
    }
    match kind {
        AreaKind::Object => {
            p.circle_filled(hole_c, hole_r, Color32::WHITE);
        }
        AreaKind::Holes => {
            p.circle_filled(hole_c, hole_r, fill);
        }
        _ => {}
    }
    let width = if kind == AreaKind::Perimeter {
        2.0
    } else {
        1.0
    };
    let outline = Stroke::new(width, line.color);
    p.rect_stroke(plate, round, outline, StrokeKind::Middle);
    p.rect_stroke(part, CornerRadius::ZERO, outline, StrokeKind::Middle);
    p.circle_stroke(hole_c, hole_r, outline);
}

#[cfg(test)]
mod tests {
    use super::super::tests::{app_with_rects, frame};
    use super::*;

    #[test]
    fn perimeter_and_area_show_the_selection_in_the_chosen_unit() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_rects();
        frame(&ctx, |ui| {
            perimeter(&mut app, ui);
        });
        let m = app.signcut.measured.map(|(_, m)| m).expect("measured");
        // Two rectangles of 30 x 20: 200 mm, 1200 mm2.
        assert!((m.length - 200.0).abs() < 1e-9);
        assert_eq!(len_text(CalcUnit::M, m.length), "0.2 m");
        assert_eq!(area_text(CalcUnit::Cm, m.area), "12 cm\u{b2}");
        app.sc_calc.perimeter_unit = CalcUnit::Mm;
        for kind in [false, true] {
            frame(&ctx, |ui| {
                if kind {
                    area(&mut app, ui);
                } else {
                    perimeter(&mut app, ui);
                }
            });
        }
        // Another selection: out of date until Update.
        let first = app.selection[0];
        app.select(vec![first]);
        assert!(app.signcut_measure_outdated());
        frame(&ctx, |ui| {
            area(&mut app, ui);
        });
        assert!(app.signcut_measure_outdated());
    }
}
