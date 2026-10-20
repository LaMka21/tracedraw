//! The traceCUT Measure docker (Window > Panels > traceCUT Measure): the
//! outline length and the areas of the selection, or of the whole page
//! when nothing is selected, in the drawing's units, measured again
//! whenever the drawing or the selection changes. See
//! docs/behavior/tracecut-measure.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{RichText, Ui};

/// A length in the drawing's units with `d` decimals: "123.45 mm".
fn len_text(app: &App, mm: f64, d: usize) -> String {
    let u = app.units;
    format!("{:.*} {}", d, u.world_from_mm(mm), u.short())
}

/// An area in the square of the drawing's unit: "1234.5 mm²".
fn area_text(app: &App, mm2: f64, d: usize) -> String {
    let u = app.units;
    let k = u.world_from_mm(1.0);
    format!("{:.*} {}\u{b2}", d, mm2 * k * k, u.short())
}

/// One row: the name on the left, the value on the right.
fn row(ui: &mut Ui, label: &str, value: &str, strong: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(Tokens::text_dim()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let v = RichText::new(value).color(Tokens::text());
            ui.label(if strong { v.strong() } else { v });
        });
    });
}

pub fn measure_docker(app: &mut App, ui: &mut Ui) {
    if !app.has_document() || app.show_welcome {
        return;
    }
    let m = app.tracecut_measurement();
    let d = app.settings.precision.min(10) as usize;
    let what = tr(if app.selection.is_empty() {
        "tracecut.measuring_page"
    } else {
        "tracecut.measuring_selection"
    });
    ui.label(RichText::new(what).color(Tokens::text_dim()).size(11.0));
    ui.separator();
    row(
        ui,
        &tr("tracecut.perimeter"),
        &len_text(app, m.length, d),
        true,
    );
    row(ui, &tr("tracecut.area"), &area_text(app, m.area, d), true);
    row(
        ui,
        &tr("tracecut.outer_area"),
        &area_text(app, m.outer_area, d),
        false,
    );
    row(
        ui,
        &tr("tracecut.hole_area"),
        &area_text(app, m.hole_area, d),
        false,
    );
    ui.separator();
    let size = m.bbox.map_or_else(
        || "-".to_string(),
        |b| {
            format!(
                "{} \u{d7} {}",
                len_text(app, b.width().abs(), d),
                len_text(app, b.height().abs(), d)
            )
        },
    );
    row(ui, &tr("tracecut.bbox"), &size, false);
    row(
        ui,
        &tr("tracecut.bbox_area"),
        &area_text(app, m.bbox_area, d),
        false,
    );
    ui.separator();
    row(ui, &tr("tracecut.objects"), &m.objects.to_string(), false);
    row(
        ui,
        &tr("tracecut.paths"),
        &trf(
            "tracecut.paths_value",
            &[
                ("closed", &m.closed_paths.to_string()),
                ("open", &m.open_paths.to_string()),
            ],
        ),
        false,
    );
    row(ui, &tr("tracecut.nodes"), &m.nodes.to_string(), false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Units;

    #[test]
    fn values_follow_the_drawings_unit() {
        let mut app = App::headless();
        app.units = Units::Centimeters;
        assert_eq!(len_text(&app, 200.0, 1), "20.0 cm");
        assert_eq!(area_text(&app, 1200.0, 1), "12.0 cm\u{b2}");
        app.units = Units::Millimeters;
        assert_eq!(area_text(&app, 1200.0, 0), "1200 mm\u{b2}");
    }
}
