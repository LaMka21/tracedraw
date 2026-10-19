//! Draws the open Sign & Cut tool windows each frame. Nesting has its own
//! window (`signcut_win_nest.rs`); the other tools show their section of
//! the Sign & Cut panel in a tool window until each gets a window of its
//! own. What a window marks on the drawing is kept for the next frame's
//! canvas (`App::sc_overlay`).

use crate::app::App;
use crate::i18n::tr;
use crate::signcut_tools::ToolId;
use crate::ui::signcut_kit::{self as kit, Buttons, Pressed};
use egui::{Context, Ui, Vec2};
use tracedraw_core::{Command, ShapeKind};

/// Draw every open tool window.
pub fn show(app: &mut App, ctx: &Context) {
    if !app.has_document() || app.show_welcome {
        return;
    }
    let raise = app.sc_windows_raise.take();
    let mut overlay = Vec::new();
    for t in app.sc_windows.open.clone() {
        let (size, resizable) = size_of(t);
        let id = egui::Id::new(("signcut_window", t.key()));
        let title = t.title();
        let mark = app.docker_preview.len();
        let mut close = false;
        let clicked_close =
            kit::tool_window(ctx, id, &title, size, resizable, raise == Some(t), |ui| {
                close = body(app, ui, t);
            });
        if app.docker_preview.len() > mark {
            overlay.extend(app.docker_preview.drain(mark..));
        }
        if close || clicked_close {
            app.sc_windows.close(t);
        }
    }
    app.sc_overlay = overlay;
}

/// A window's default size and whether it can be resized.
fn size_of(t: ToolId) -> (Vec2, bool) {
    if crate::ui::signcut_win_nest::tool_of(t).is_some() {
        return (crate::ui::signcut_win_nest::SIZE, true);
    }
    if crate::ui::signcut_win_cut::TOOLS.contains(&t) {
        return crate::ui::signcut_win_cut::size(t);
    }
    if crate::ui::signcut_win_calc::TOOLS.contains(&t) {
        return crate::ui::signcut_win_calc::size(t);
    }
    if crate::ui::signcut_win_sign::TOOLS.contains(&t) {
        return crate::ui::signcut_win_sign::size(t);
    }
    if crate::ui::signcut_win_cnc::TOOLS.contains(&t) {
        return crate::ui::signcut_win_cnc::size(t);
    }
    if crate::ui::signcut_win_fx::TOOLS.contains(&t) {
        return crate::ui::signcut_win_fx::size(t);
    }
    match t {
        ToolId::ReduceNodes => (Vec2::new(320.0, 160.0), false),
        _ => (Vec2::new(380.0, 300.0), true),
    }
}

/// A window's body. Returns true when it should close.
fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    if let Some(tool) = crate::ui::signcut_win_nest::tool_of(t) {
        return crate::ui::signcut_win_nest::body(app, ui, tool);
    }
    if crate::ui::signcut_win_cut::TOOLS.contains(&t) {
        return crate::ui::signcut_win_cut::body(app, ui, t);
    }
    if crate::ui::signcut_win_calc::TOOLS.contains(&t) {
        return crate::ui::signcut_win_calc::body(app, ui, t);
    }
    if crate::ui::signcut_win_sign::TOOLS.contains(&t) {
        return crate::ui::signcut_win_sign::body(app, ui, t);
    }
    if crate::ui::signcut_win_cnc::TOOLS.contains(&t) {
        return crate::ui::signcut_win_cnc::body(app, ui, t);
    }
    if crate::ui::signcut_win_fx::TOOLS.contains(&t) {
        return crate::ui::signcut_win_fx::body(app, ui, t);
    }
    if t == ToolId::ReduceNodes {
        return reduce_nodes(app, ui);
    }
    ui.label(tr("signcut.not_available"));
    matches!(
        kit::bottom(ui, Buttons::Close, true, |_| {}),
        Some(Pressed::Close)
    )
}

/// Reduce Nodes: remove nodes of the selected curves that the outline
/// does not need, within a tolerance. One undo step.
fn reduce_nodes(app: &mut App, ui: &mut Ui) -> bool {
    let u = app.units;
    kit::row(ui, &tr("signcut.rn_tolerance"), |ui| {
        kit::length(ui, u, &mut app.sc_reduce_tolerance, 0.001, 100.0)
    });
    let has = !app.selection.is_empty();
    match kit::bottom(ui, Buttons::ApplyOkCancel, has, |_| {}) {
        Some(Pressed::Apply) => {
            app.signcut_reduce_nodes();
            false
        }
        Some(Pressed::Ok) => {
            app.signcut_reduce_nodes();
            true
        }
        Some(_) => true,
        None => false,
    }
}

impl App {
    /// Reduce the nodes of the selected curves (rectangles, ellipses and
    /// other shapes are left alone). Returns the nodes removed.
    pub fn signcut_reduce_nodes(&mut self) -> usize {
        let tol = self.sc_reduce_tolerance.max(1e-6);
        let doc = self.doc();
        let mut removed = 0;
        let mut cmds = Vec::new();
        for id in &self.selection {
            let Some(s) = doc.find_shape(*id) else {
                continue;
            };
            if s.locked {
                continue;
            }
            if let ShapeKind::Path { path, closed } = &s.kind {
                let reduced = tracedraw_core::nodes::reduce_nodes(path, tol);
                let before = path.elements().len();
                let after = reduced.elements().len();
                if after < before {
                    removed += before - after;
                    cmds.push(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Path {
                            path: reduced,
                            closed: *closed,
                        },
                    });
                }
            }
        }
        if cmds.is_empty() {
            self.status = tr("signcut.rn_nothing");
            return 0;
        }
        if let Err(e) = self.engine.run_batch("Reduce Nodes", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.status = crate::i18n::trf("signcut.rn_done", &[("n", &removed.to_string())]);
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::BezPath;

    #[test]
    fn reduce_nodes_removes_nodes_on_a_straight_line() {
        let mut app = App::headless();
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        for i in 1..=10 {
            p.line_to((i as f64 * 10.0, 0.0));
        }
        let id = app
            .new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("layer");
        app.select(vec![id]);
        assert!(app.signcut_reduce_nodes() > 0);
        assert_eq!(app.engine.undo_label(), Some("Reduce Nodes"));
    }

    #[test]
    fn every_tool_window_draws() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for t in ToolId::all() {
            app.signcut_tool(t);
        }
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            show(&mut app, &ctx);
        });
        out.textures_delta.clear();
        assert!(!app.sc_windows.open.is_empty());
    }
}
