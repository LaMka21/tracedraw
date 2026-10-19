//! Sign & Cut, Effects and Patterns: settings and operations (hatching lines, grids, puzzles, circle fills, cast shadows and random copies).
//! The geometry is in `tracedraw_signcut::fx`; the panel section is
//! `ui/signcut_fx.rs`. See docs/behavior/sign-and-cut-fx.md.

use crate::app::App;
use egui::Ui;

/// The Effects and Patterns section's settings and last results.
#[derive(Debug, Clone, Default)]
pub struct Settings {}

/// Tools > Sign & Cut > Effects and Patterns.
pub fn menu(app: &mut App, ui: &mut Ui) {
    let _ = (app, ui);
}
