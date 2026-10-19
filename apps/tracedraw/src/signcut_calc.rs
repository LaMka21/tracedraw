//! Sign & Cut, Calculations: settings and operations (cutting time, cost and material volume and weight).
//! The geometry is in `tracedraw_signcut::calc`; the panel section is
//! `ui/signcut_calc.rs`. See docs/behavior/sign-and-cut-calc.md.

use crate::app::App;
use egui::Ui;

/// The Calculations section's settings and last results.
#[derive(Debug, Clone, Default)]
pub struct Settings {}

/// Tools > Sign & Cut > Calculations.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let _ = (app, ui);
}
