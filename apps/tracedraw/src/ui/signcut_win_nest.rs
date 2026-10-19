//! The windows of Shape Nesting, Rectangle Nesting and Fill Sheet: the
//! preview of the material and the objects at the left with the yellow
//! status strip, the settings at the right (preset row, Main and Layout
//! tabs, units), Show Original, Apply (work it out into the preview), OK
//! (apply it to the drawing) and Cancel. See
//! docs/behavior/sign-and-cut-windows.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_nest::{Computed, Tool};
use crate::signcut_tools::ToolId;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::{Ui, Vec2};
use tracedraw_core::geometry::{Affine, BezPath, Rect, Shape as _};
use tracedraw_signcut::nest::shapes::Rotations;
use tracedraw_signcut::nest::{Corner, RollDir};

/// One nesting window's state.
#[derive(Debug, Clone, Default)]
pub struct NestWindow {
    /// The last layout worked out with Apply.
    pub computed: Option<Computed>,
    /// Show the objects where they are now instead of the layout.
    pub original: bool,
    /// The Layout tab is open (else Main).
    pub layout_tab: bool,
}

/// The three nesting windows' states.
#[derive(Debug, Clone, Default)]
pub struct States {
    pub rects: NestWindow,
    pub shapes: NestWindow,
    pub fill: NestWindow,
}

impl States {
    fn get(&mut self, tool: Tool) -> &mut NestWindow {
        match tool {
            Tool::Rects => &mut self.rects,
            Tool::Shapes => &mut self.shapes,
            Tool::Fill => &mut self.fill,
        }
    }
}

/// The nesting tool of a toolbar entry.
pub fn tool_of(t: ToolId) -> Option<Tool> {
    match t {
        ToolId::ShapeNesting => Some(Tool::Shapes),
        ToolId::RectNesting => Some(Tool::Rects),
        ToolId::FillSheet => Some(Tool::Fill),
        _ => None,
    }
}

/// The window's default size.
pub const SIZE: Vec2 = Vec2::new(1000.0, 700.0);

/// Draw the body of the window of `tool`. Returns true to close it.
pub fn body(app: &mut App, ui: &mut Ui, tool: Tool) -> bool {
    let before = serde_json::to_value(&app.sc_nest).ok();
    let height = (ui.available_height() - 60.0).max(420.0);
    let mut pressed = None;
    ui.horizontal_top(|ui| {
        let left_w = (ui.available_width() - kit::SIDE_W - 12.0).max(320.0);
        ui.vertical(|ui| {
            ui.set_width(left_w);
            let (scene, strip) = scene(app, tool);
            kit::preview(ui, Vec2::new(left_w, height - 22.0), &scene);
            kit::status_strip(ui, left_w, &strip);
        });
        ui.vertical(|ui| {
            ui.set_width(kit::SIDE_W);
            settings(app, ui, tool);
        });
    });
    let fresh = app
        .sc_win_nest
        .get(tool)
        .computed
        .as_ref()
        .is_some_and(|c| c.revision == app.engine.revision());
    kit::progress(ui, if fresh { 1.0 } else { 0.0 });
    let has = !app.nest_targets().is_empty();
    let original = app.sc_win_nest.get(tool).original;
    pressed = pressed.or(kit::bottom(ui, Buttons::ApplyOkCancel, has, |ui| {
        if ui
            .add(egui::Button::new(tr("signcut.nw_show_original")).selected(original))
            .clicked()
        {
            let w = app.sc_win_nest.get(tool);
            w.original = !w.original;
        }
    }));
    // A changed setting makes the preview out of date.
    if serde_json::to_value(&app.sc_nest).ok() != before {
        app.sc_win_nest.get(tool).computed = None;
    }
    match pressed {
        Some(Pressed::Apply) => {
            let c = app.nest_compute(tool);
            if let Some(c) = &c {
                app.status = summary(app, c);
            }
            let w = app.sc_win_nest.get(tool);
            w.computed = c;
            w.original = false;
            false
        }
        Some(Pressed::Ok) => {
            let fresh = app
                .sc_win_nest
                .get(tool)
                .computed
                .clone()
                .filter(|c| c.revision == app.engine.revision());
            let c = fresh.or_else(|| app.nest_compute(tool));
            app.sc_win_nest.get(tool).computed = None;
            if let Some(c) = c {
                app.nest_commit(&c);
            }
            true
        }
        Some(Pressed::Cancel) | Some(Pressed::Close) => {
            app.sc_win_nest.get(tool).computed = None;
            true
        }
        _ => false,
    }
}

/// The preview: the material and the objects, as the last Apply placed
/// them, or where they are now; and the status strip's text.
fn scene(app: &mut App, tool: Tool) -> (Scene, String) {
    let mut scene = Scene::default();
    let rev = app.engine.revision();
    let units = app.units;
    let state = app.sc_win_nest.get(tool).clone();
    let computed = state
        .computed
        .filter(|c| c.revision == rev && !state.original);
    match computed {
        Some(c) => {
            let sheets = match c.tool {
                Tool::Fill => usize::from(!c.copies.is_empty()),
                _ if c.spec.roll => 1,
                _ => c.layout.sheets.max(1),
            };
            for i in 0..sheets {
                sheet_outline(&mut scene, c.spec.sheet_rect(i, c.layout.length.max(1.0)));
            }
            let mut placed_bounds: Option<Rect> = None;
            let mut place = |scene: &mut Scene, t: Affine, outlines: &[BezPath]| {
                for o in outlines {
                    let p = t * o.clone();
                    let b = p.bounding_box();
                    if b.x0.is_finite() && b.y0.is_finite() {
                        placed_bounds = Some(placed_bounds.map_or(b, |a| a.union(b)));
                    }
                    scene.add(p, colors::PART, 1.0);
                }
            };
            let (n, total) = if c.tool == Tool::Fill {
                for t in &c.copies {
                    for o in &c.outlines {
                        place(&mut scene, *t, o);
                    }
                }
                (c.copies.len(), c.copies.len())
            } else {
                for (o, p) in c.outlines.iter().zip(&c.layout.placements) {
                    if let Some(p) = p {
                        place(&mut scene, p.transform, o);
                    }
                }
                (c.layout.placed(), c.ids.len())
            };
            let util = c.layout.utilisation() * 100.0;
            if let Some(b) = placed_bounds {
                scene.captions.push(format!(
                    "{} x {}   {:.0} %",
                    crate::ui::signcut_docker::fmt_len(units, b.width()),
                    crate::ui::signcut_docker::fmt_len(units, b.height()),
                    util
                ));
            }
            scene.captions.push(trf(
                "signcut.nw_usage",
                &[("u", &format!("{util:.1}")), ("n", &sheets.to_string())],
            ));
            let strip = trf(
                "signcut.nw_strip",
                &[
                    ("sheets", &sheets.to_string()),
                    ("n", &n.to_string()),
                    ("total", &total.to_string()),
                ],
            );
            (scene, strip)
        }
        None => {
            let spec = if tool == Tool::Fill {
                app.nest_plain_sheet()
            } else {
                app.nest_sheet()
            };
            let doc = app.doc();
            let mut count = 0;
            for id in app.nest_targets() {
                if let Some(s) = doc.shape_on_page(id) {
                    count += 1;
                    for o in tracedraw_signcut::outlines(&s) {
                        scene.add(o, colors::ORIGINAL, 1.0);
                    }
                }
            }
            let len = if spec.roll {
                spec.width.max(1.0) * 2.0
            } else {
                spec.height
            };
            sheet_outline(&mut scene, spec.sheet_rect(0, len));
            let strip = if count == 0 {
                tr("signcut.nest_select")
            } else {
                trf("signcut.nw_strip_ready", &[("n", &count.to_string())])
            };
            (scene, strip)
        }
    }
}

/// A sheet: its outline, the bottom edge in red.
fn sheet_outline(scene: &mut Scene, r: Rect) {
    let r = r.abs();
    let mut p = BezPath::new();
    p.move_to((r.x0, r.y0));
    p.line_to((r.x0, r.y1));
    p.line_to((r.x1, r.y1));
    p.line_to((r.x1, r.y0));
    scene.add(p, colors::SHEET, 1.0);
    let mut e = BezPath::new();
    e.move_to((r.x0, r.y0));
    e.line_to((r.x1, r.y0));
    scene.add(e, colors::SHEET_EDGE, 1.5);
}

/// The status line after Apply.
fn summary(app: &App, c: &Computed) -> String {
    if c.tool == Tool::Fill {
        return trf("signcut.nest_filled", &[("n", &c.copies.len().to_string())]);
    }
    trf(
        "signcut.nw_applied",
        &[
            ("n", &c.layout.placed().to_string()),
            ("total", &c.ids.len().to_string()),
            ("sheets", &c.layout.sheets.to_string()),
            ("u", &format!("{:.1}", c.layout.utilisation() * 100.0)),
            (
                "len",
                &crate::ui::signcut_docker::fmt_len(app.units, c.layout.length),
            ),
        ],
    )
}

/// The settings column: preset row, tabs, units.
fn settings(app: &mut App, ui: &mut Ui, tool: Tool) {
    let key = match tool {
        Tool::Rects => "rect_nesting",
        Tool::Shapes => "shape_nesting",
        Tool::Fill => "fill_sheet",
    };
    kit::preset_row(ui, &mut app.sc_presets, key, &mut app.sc_nest);
    ui.add_space(4.0);
    let w = app.sc_win_nest.get(tool);
    ui.horizontal(|ui| {
        if ui
            .selectable_label(!w.layout_tab, tr("signcut.nw_tab_main"))
            .clicked()
        {
            w.layout_tab = false;
        }
        if ui
            .selectable_label(w.layout_tab, tr("signcut.nw_tab_layout"))
            .clicked()
        {
            w.layout_tab = true;
        }
    });
    let layout_tab = w.layout_tab;
    egui::ScrollArea::vertical()
        .id_salt(("nest_settings", key))
        .max_height(ui.available_height() - 40.0)
        .show(ui, |ui| {
            if layout_tab {
                layout(app, ui, tool);
            } else {
                main(app, ui, tool);
            }
        });
    kit::units_row(ui, &mut app.units);
}

fn main(app: &mut App, ui: &mut Ui, tool: Tool) {
    let u = app.units;
    let page = app.page_size();
    let n = &mut app.sc_nest;
    let roll = n.sheet.roll && tool != Tool::Fill;
    kit::group(ui, &tr("signcut.nw_sheet_size"), |ui| {
        let fixed = n.use_page && !roll;
        ui.add_enabled_ui(!fixed, |ui| {
            kit::row(
                ui,
                &tr(if roll {
                    "signcut.nw_roll_width"
                } else {
                    "signcut.nw_sheet_width"
                }),
                |ui| kit::length(ui, u, &mut n.sheet.width, 1.0, 100_000.0),
            );
            if !roll {
                kit::row(ui, &tr("signcut.nw_sheet_height"), |ui| {
                    kit::length(ui, u, &mut n.sheet.height, 1.0, 100_000.0)
                });
            }
        });
        if !roll {
            ui.horizontal(|ui| {
                ui.checkbox(&mut n.use_page, tr("signcut.nw_use_page"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(!n.use_page, egui::Button::new("\u{21c5}"))
                        .on_hover_text(tr("signcut.nw_swap"))
                        .clicked()
                    {
                        std::mem::swap(&mut n.sheet.width, &mut n.sheet.height);
                    }
                });
            });
            if n.use_page {
                n.sheet.width = page.width;
                n.sheet.height = page.height;
            }
        }
        kit::separator(ui);
        kit::row(ui, &tr("signcut.nw_margin"), |ui| {
            kit::length(ui, u, &mut n.sheet.margin, 0.0, 10_000.0)
        });
    });
    match tool {
        Tool::Fill => {
            kit::group(ui, &tr("signcut.nw_copies"), |ui| {
                kit::row(ui, &tr("signcut.nw_gap"), |ui| {
                    kit::length(ui, u, &mut n.gap, 0.0, 10_000.0)
                });
                ui.checkbox(&mut n.fill_rotate, tr("signcut.nw_fill_turn"));
                kit::row(ui, &tr("signcut.nw_limit"), |ui| {
                    kit::count(ui, &mut n.fill_limit, 0, 10_000)
                });
            });
        }
        _ => {
            kit::group(ui, &tr("signcut.nw_options"), |ui| {
                kit::row(ui, &tr("signcut.nw_min_distance"), |ui| {
                    kit::length(ui, u, &mut n.sheet.spacing, 0.0, 10_000.0)
                });
                if tool == Tool::Shapes {
                    ui.checkbox(&mut n.holes, tr("signcut.nw_allow_inside"));
                    kit::row(ui, &tr("signcut.nw_search_best"), |ui| {
                        let opts = [(false, tr("signcut.nw_no")), (true, tr("signcut.nw_yes"))];
                        kit::choice(ui, "nest_improve", &mut n.improve, &opts)
                    });
                    ui.add_enabled_ui(n.improve, |ui| {
                        kit::row(ui, &tr("signcut.nw_search_time"), |ui| {
                            kit::number(ui, &mut n.time_limit, 0.0, 600.0, 1)
                        });
                        kit::row(ui, &tr("signcut.nw_search_rounds"), |ui| {
                            kit::count(ui, &mut n.rounds, 0, 100_000)
                        });
                    });
                    kit::separator(ui);
                    kit::row(ui, &tr("signcut.nw_turns"), |ui| {
                        let opts = [
                            (
                                Rotations::None,
                                tr(crate::signcut_nest::rotations_key(Rotations::None)),
                            ),
                            (
                                Rotations::Half,
                                tr(crate::signcut_nest::rotations_key(Rotations::Half)),
                            ),
                            (
                                Rotations::Quarter,
                                tr(crate::signcut_nest::rotations_key(Rotations::Quarter)),
                            ),
                            (
                                Rotations::Eighth,
                                tr(crate::signcut_nest::rotations_key(Rotations::Eighth)),
                            ),
                        ];
                        kit::choice(ui, "nest_turns", &mut n.rotations, &opts)
                    });
                    kit::row(ui, &tr("signcut.nw_accuracy"), |ui| {
                        kit::length(ui, u, &mut n.tolerance, 0.01, 50.0)
                    });
                } else {
                    ui.checkbox(&mut n.rotate90, tr("signcut.nw_turn90"));
                }
                kit::row(ui, &tr("signcut.nw_origin"), |ui| {
                    let opts: Vec<(Corner, String)> = Corner::ALL
                        .iter()
                        .map(|c| (*c, tr(crate::signcut_nest::corner_key(*c))))
                        .collect();
                    kit::choice(ui, "nest_corner", &mut n.sheet.corner, &opts)
                });
            });
        }
    }
}

fn layout(app: &mut App, ui: &mut Ui, tool: Tool) {
    let n = &mut app.sc_nest;
    kit::group(ui, &tr("signcut.nw_material"), |ui| {
        if tool == Tool::Fill {
            ui.label(tr("signcut.nw_fill_sheet_only"));
        } else {
            kit::row(ui, &tr("signcut.nw_material"), |ui| {
                let opts = [
                    (false, tr("signcut.nw_sheets")),
                    (true, tr("signcut.nw_roll")),
                ];
                kit::choice(ui, "nest_roll", &mut n.sheet.roll, &opts)
            });
            ui.add_enabled_ui(n.sheet.roll, |ui| {
                kit::row(ui, &tr("signcut.nw_roll_dir"), |ui| {
                    let opts = [
                        (
                            RollDir::Right,
                            tr(crate::signcut_nest::roll_dir_key(RollDir::Right)),
                        ),
                        (
                            RollDir::Up,
                            tr(crate::signcut_nest::roll_dir_key(RollDir::Up)),
                        ),
                    ];
                    kit::choice(ui, "nest_roll_dir", &mut n.sheet.roll_dir, &opts)
                });
            });
            ui.checkbox(&mut n.frames, tr("signcut.nw_frames"));
        }
    });
    if tool == Tool::Shapes {
        kit::group(ui, &tr("signcut.nw_search"), |ui| {
            kit::row(ui, &tr("signcut.nw_seed"), |ui| {
                let mut s = n.seed as usize;
                let r = kit::count(ui, &mut s, 0, 1_000_000_000);
                if r.changed() {
                    n.seed = s as u64;
                }
                r
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeKind;

    fn app_with_parts() -> App {
        let mut app = App::headless();
        let mut ids = Vec::new();
        for i in 0..3 {
            let x = i as f64 * 30.0;
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: Rect::new(x, 50.0, x + 20.0, 60.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("layer"),
            );
        }
        app.select(ids);
        app
    }

    #[test]
    fn apply_previews_and_changes_nothing() {
        let mut app = app_with_parts();
        let rev = app.engine.revision();
        let c = app.nest_compute(Tool::Rects).expect("layout");
        assert_eq!(c.layout.placed(), 3);
        assert_eq!(app.engine.revision(), rev);
        app.sc_win_nest.rects.computed = Some(c);
        let (scene, strip) = scene(&mut app, Tool::Rects);
        assert!(scene.items.len() >= 3);
        assert!(!strip.is_empty());
    }

    #[test]
    fn ok_applies_the_previewed_layout_in_one_step() {
        let mut app = app_with_parts();
        let c = app.nest_compute(Tool::Rects).expect("layout");
        assert!(app.nest_commit(&c));
        assert_eq!(app.engine.undo_label(), Some("Nest Rectangles"));
    }

    #[test]
    fn fill_sheet_previews_its_copies() {
        let mut app = app_with_parts();
        let c = app.nest_compute(Tool::Fill).expect("copies");
        assert!(!c.copies.is_empty());
        app.sc_win_nest.fill.computed = Some(c);
        let (scene, _) = scene(&mut app, Tool::Fill);
        assert!(scene.items.len() > 3);
    }

    #[test]
    fn the_window_draws_every_tool_and_tab() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_parts();
        for tool in [Tool::Rects, Tool::Shapes, Tool::Fill] {
            for tab in [false, true] {
                app.sc_win_nest.get(tool).layout_tab = tab;
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = body(&mut app, ui, tool);
                });
                out.textures_delta.clear();
            }
        }
    }
}
