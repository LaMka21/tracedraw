//! The Sign & Cut tool windows of effects and patterns. Until a tool has a
//! window of its own it shows its section of the panel. See
//! docs/behavior/sign-and-cut-windows.md.

use crate::app::App;
use crate::signcut_tools::ToolId;
use crate::ui::signcut_kit::{self as kit, Buttons, Pressed};
use egui::{Ui, Vec2};

/// The tools whose windows this file draws.
pub const TOOLS: &[ToolId] = &[ToolId::Effects, ToolId::Hatching, ToolId::Grid];

/// A window's default size and whether it can be resized.
pub fn size(_t: ToolId) -> (Vec2, bool) {
    (Vec2::new(380.0, 560.0), true)
}

/// The window's body. Returns true when it should close.
pub fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    let section: fn(&mut App, &mut Ui) = match t {
        ToolId::Effects => crate::ui::signcut_fx::section,
        ToolId::Hatching => crate::ui::signcut_fx::section,
        ToolId::Grid => crate::ui::signcut_fx::section,
        _ => return true,
    };
    let h = (ui.available_height() - 40.0).max(200.0);
    egui::ScrollArea::vertical()
        .id_salt(("signcut_window_body", t.key()))
        .max_height(h)
        .show(ui, |ui| section(app, ui));
    matches!(
        kit::bottom(ui, Buttons::Close, true, |_| {}),
        Some(Pressed::Close)
    )
}
