//! Sign & Cut, Cutting: settings and operations (plotter output, weeding, bridges, cut order, registration and crop marks).
//! The geometry is in `tracedraw_signcut::cut`; the panel section is
//! `ui/signcut_cut.rs`. See docs/behavior/sign-and-cut-cut.md.

use crate::app::App;
use egui::Ui;

/// The Cutting section's settings and last results.
#[derive(Debug, Clone, Default)]
pub struct Settings {}

/// Tools > Sign & Cut > Cutting.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let _ = (app, ui);
}
