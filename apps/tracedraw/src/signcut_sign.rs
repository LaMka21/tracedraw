//! Sign & Cut, Sign Making: settings and operations (LED modules, neon tubes, sign boxes, letter returns, tiling and carriers).
//! The geometry is in `tracedraw_signcut::sign`; the panel section is
//! `ui/signcut_sign.rs`. See docs/behavior/sign-and-cut-sign.md.

use crate::app::App;
use egui::Ui;

/// The Sign Making section's settings and last results.
#[derive(Debug, Clone, Default)]
pub struct Settings {}

/// Tools > Sign & Cut > Sign Making.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let _ = (app, ui);
}
