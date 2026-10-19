//! The Sign & Cut panel: Measure (length and area), Clean Up (duplicate
//! objects, duplicate edges, crossings) and Size (Set Size, Reference
//! Scale, Replace). See docs/behavior/sign-and-cut.md.

use crate::app::{App, Units};
use crate::i18n::tr;
use egui::Ui;

/// The panel: every Sign & Cut tool by toolbar group; a click opens its
/// window (or the panel or dialog TraceDraw has for it).
pub fn signcut(app: &mut App, ui: &mut Ui) {
    use crate::signcut_tools::Group;
    app.signcut.open = None;
    let doc = app.has_document() && !app.show_welcome;
    for g in Group::ALL {
        if g.tools().is_empty() {
            continue;
        }
        crate::ui::chrome::section(ui, &tr(g.key()));
        for t in g.tools() {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                crate::ui::signcut_toolbar::draw_icon(ui.painter(), r, g);
                let open = app.sc_windows.is_open(*t);
                if ui
                    .add_enabled(
                        doc,
                        egui::Button::new(t.title()).selected(open).frame(false),
                    )
                    .clicked()
                {
                    app.signcut_tool(*t);
                }
            });
        }
    }
}

/// An area in square document units and in square metres.
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

pub(crate) fn fmt_len(u: Units, mm: f64) -> String {
    format!("{:.2} {}", u.world_from_mm(mm), u.short())
}
