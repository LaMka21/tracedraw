//! Sign & Cut, Nesting: settings and operations (placing objects on sheets and rolls with the least waste).
//! The geometry is in `tracedraw_signcut::nest`; the panel section is
//! `ui/signcut_nest.rs`. See docs/behavior/sign-and-cut-nest.md.

use crate::app::App;
use egui::Ui;

/// The Nesting section's settings and last results.
#[derive(Debug, Clone, Default)]
pub struct Settings {}

/// Tools > Sign & Cut > Nesting.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let _ = (app, ui);
}
