//! Sign & Cut, CNC and Laser: settings and operations (G-code, hot wire paths, finger joints, door panels and centre lines).
//! The geometry is in `tracedraw_signcut::cnc`; the panel section is
//! `ui/signcut_cnc.rs`. See docs/behavior/sign-and-cut-cnc.md.

use crate::app::App;
use egui::Ui;

/// The CNC and Laser section's settings and last results.
#[derive(Debug, Clone, Default)]
pub struct Settings {}

/// Tools > Sign & Cut > CNC and Laser.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let _ = (app, ui);
}
