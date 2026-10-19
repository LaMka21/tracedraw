//! The Sign & Cut tool windows of measuring, calculations and arranging:
//! Perimeter, Area, Cost, Cutting Time, Volume and Weight (calculators
//! with Update and Close), Set Size and Replace (command windows with
//! Close) and Reference Scale (Apply, OK, Cancel). See
//! docs/behavior/sign-and-cut-calc.md.

mod arrange;
mod cost;
mod measure;
mod time;
mod weight;

use crate::app::App;
use crate::i18n::tr;
use crate::signcut_calc::CalcUnit;
use crate::signcut_tools::ToolId;
use crate::theme::Tokens;
use crate::ui::signcut_kit::{self as kit, Pressed};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Painter, Pos2, Rect, Response, RichText,
    Sense, Stroke, StrokeKind, Ui, Vec2,
};

/// The tools whose windows this file draws.
pub const TOOLS: &[ToolId] = &[
    ToolId::Perimeter,
    ToolId::Area,
    ToolId::Cost,
    ToolId::CuttingTime,
    ToolId::Weight,
    ToolId::SetSize,
    ToolId::ReferenceScale,
    ToolId::Replace,
];

/// A window's default size and whether it can be resized.
pub fn size(t: ToolId) -> (Vec2, bool) {
    match t {
        ToolId::Perimeter => (Vec2::new(310.0, 250.0), false),
        ToolId::Area => (Vec2::new(280.0, 470.0), false),
        ToolId::Cost => (Vec2::new(880.0, 330.0), true),
        ToolId::CuttingTime => (Vec2::new(400.0, 320.0), true),
        ToolId::Weight => (Vec2::new(310.0, 560.0), false),
        ToolId::SetSize => (Vec2::new(300.0, 660.0), false),
        ToolId::ReferenceScale => (Vec2::new(320.0, 400.0), false),
        ToolId::Replace => (Vec2::new(400.0, 290.0), false),
        _ => (Vec2::new(380.0, 300.0), true),
    }
}

/// The window's body. Returns true when it should close.
pub fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    let calc = matches!(
        t,
        ToolId::Perimeter | ToolId::Area | ToolId::Cost | ToolId::CuttingTime | ToolId::Weight
    );
    if !calc {
        return arrange_body(app, ui, t);
    }
    app.sc_calc.ensure_loaded();
    let kept = app.sc_calc.to_json();
    let close = match t {
        ToolId::Perimeter => measure::perimeter(app, ui),
        ToolId::Area => measure::area(app, ui),
        ToolId::Cost => cost::window(app, ui),
        ToolId::CuttingTime => time::window(app, ui),
        _ => weight::window(app, ui),
    };
    // Keep what changed for the next start, once nothing is being typed.
    if app.sc_calc.to_json() != kept {
        app.sc_calc.dirty = true;
    }
    if app.sc_calc.dirty && ui.ctx().memory(|m| m.focused().is_none()) {
        app.sc_calc.save_if_dirty();
    }
    close
}

fn arrange_body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    match t {
        ToolId::SetSize => arrange::set_size(app, ui),
        ToolId::ReferenceScale => arrange::reference_scale(app, ui),
        ToolId::Replace => arrange::replace(app, ui),
        _ => true,
    }
}

/// True on the first frame a window is drawn after it was closed (or
/// ever): the calculators then measure the objects as they are.
fn just_opened(ui: &Ui, key: &str) -> bool {
    let nr = ui.ctx().cumulative_pass_nr();
    let id = egui::Id::new(("signcut_wcalc_seen", key));
    let last: Option<u64> = ui.data(|d| d.get_temp(id));
    ui.data_mut(|d| d.insert_temp(id, nr));
    last.is_none_or(|l| l.saturating_add(1) < nr)
}

/// A result shown in a field's frame that cannot be typed in; bold for
/// the main result, grey while it is out of date.
fn readout(ui: &mut Ui, text: &str, strong: bool, stale: bool) -> Response {
    readout_w(ui, kit::FIELD_W, text, strong, stale)
}

fn readout_w(ui: &mut Ui, width: f32, text: &str, strong: bool, stale: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::hover());
    let p = ui.painter_at(r);
    p.rect(
        r,
        CornerRadius::ZERO,
        Tokens::field(),
        Stroke::new(1.0, Tokens::control_border()),
        StrokeKind::Inside,
    );
    let font = if strong {
        crate::theme::bold(13.0)
    } else {
        FontId::proportional(13.0)
    };
    let color = if stale {
        Tokens::text_dim()
    } else {
        Tokens::text()
    };
    p.text(
        Pos2::new(r.left() + 5.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    resp.on_hover_text(text)
}

/// A form row with a bold label.
fn strong_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_sized(
            [kit::LABEL_W.min(ui.available_width() * 0.55), 22.0],
            egui::Label::new(RichText::new(label).strong()).truncate(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), add)
            .inner
    })
    .inner
}

/// The Units list of a calculator (lengths, areas and volumes in that
/// unit, its square and cube).
fn unit_row(ui: &mut Ui, id: &str, unit: &mut CalcUnit) -> bool {
    kit::row(ui, &tr("signcut.units"), |ui| {
        let opts: Vec<(CalcUnit, String)> = CalcUnit::ALL
            .iter()
            .map(|u| (*u, u.id().to_string()))
            .collect();
        kit::choice(ui, ("signcut_wcalc_unit", id), unit, &opts)
    })
}

/// The grey note under results that are out of date.
fn stale_note(ui: &mut Ui, stale: bool) {
    if stale {
        ui.label(
            RichText::new(tr("signcut.wcalc_outdated"))
                .size(11.0)
                .color(Tokens::text_dim()),
        );
    }
}

/// `v` with up to `max` decimals, trailing zeros dropped.
fn num(v: f64, max: usize) -> String {
    if !v.is_finite() {
        return "-".into();
    }
    crate::ui::field::format_number(v, false, 0, Some(max))
}

/// The decimals a value in `u` (or its square or cube) is shown with.
fn decimals(u: CalcUnit) -> usize {
    match u {
        CalcUnit::Mm | CalcUnit::Cm => 2,
        CalcUnit::In => 3,
        CalcUnit::M | CalcUnit::Ft => 4,
    }
}

/// What a calculator's bottom row asked for: Update or Close.
fn calc_buttons(ui: &mut Ui, left: impl FnOnce(&mut Ui)) -> Option<Pressed> {
    kit::bottom(ui, kit::Buttons::UpdateClose, true, left)
}

/// A full-width push button with a small picture at its left (drawn by
/// `pic` into the rect it is given).
fn picture_button(
    ui: &mut Ui,
    text: &str,
    enabled: bool,
    pic: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let size = Vec2::new(ui.available_width(), 26.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(size, sense);
    let p = ui.painter_at(r);
    let fill = if enabled && resp.hovered() {
        Tokens::tool_hover()
    } else {
        Tokens::control_fill()
    };
    p.rect(
        r,
        CornerRadius::same(2),
        fill,
        Stroke::new(1.0, Tokens::control_border()),
        StrokeKind::Inside,
    );
    let color = if enabled {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    let icon = Rect::from_center_size(Pos2::new(r.left() + 18.0, r.center().y), Vec2::splat(16.0));
    pic(&p, icon, color);
    p.text(
        Pos2::new(r.left() + 34.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        color,
    );
    enabled && resp.clicked()
}

/// A small tab bar choosing `current`, with a line under it.
fn tabs<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, items: &[(T, String)]) {
    ui.horizontal(|ui| {
        for (t, label) in items {
            if ui.selectable_label(*current == *t, label).clicked() {
                *current = *t;
            }
        }
    });
    kit::separator(ui);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect as GRect;
    use tracedraw_core::ShapeKind;

    /// Draw `f` once in a headless frame.
    pub(super) fn frame(ctx: &egui::Context, f: impl FnOnce(&mut Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
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

    pub(super) fn app_with_rects() -> App {
        let mut app = App::headless();
        let mut ids = Vec::new();
        for i in 0..2 {
            let x = i as f64 * 40.0;
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: GRect::new(x, 0.0, x + 30.0, 20.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("a layer"),
            );
        }
        app.select(ids);
        app
    }

    #[test]
    fn every_calc_window_draws_and_changes_nothing() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_rects();
        let rev = app.engine.revision();
        for t in TOOLS {
            frame(&ctx, |ui| {
                let _ = body(&mut app, ui, *t);
            });
            // Drawn again (not just opened).
            frame(&ctx, |ui| {
                let _ = body(&mut app, ui, *t);
            });
        }
        assert_eq!(app.engine.revision(), rev);
        // The calculators measured on opening.
        assert!(app.signcut.measured.is_some());
        assert!(app.sc_calc.measured.is_some());
    }

    #[test]
    fn a_window_counts_as_opened_after_a_frame_without_it() {
        let ctx = crate::theme::ui_context();
        let mut seen = Vec::new();
        for draw in [true, true, false, true] {
            frame(&ctx, |ui| {
                if draw {
                    seen.push(just_opened(ui, "test"));
                }
            });
        }
        assert_eq!(seen, vec![true, false, true]);
    }

    #[test]
    fn numbers_drop_trailing_zeros() {
        assert_eq!(num(0.885, 3), "0.885");
        assert_eq!(num(2.5, 3), "2.5");
        assert_eq!(num(f64::NAN, 3), "-");
    }
}
