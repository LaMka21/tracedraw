//! The Sign & Cut panel: Measure (length and area), Clean Up (duplicate
//! objects, duplicate edges, crossings) and Size (Set Size, Reference
//! Scale, Replace). See docs/behavior/sign-and-cut.md.

use crate::app::{App, Units};
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{RichText, Ui};

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
