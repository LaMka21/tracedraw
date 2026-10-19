//! The G-code window: the tool paths at the left, the machine preset row
//! and the Machine, Tools and Jobs tabs at the right, Save G-code and
//! Show the tool paths at the bottom left, Close at the right. Nothing
//! in the drawing changes. See docs/behavior/sign-and-cut-cnc.md.

use super::{rect_path, swatch};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_cnc::{hex, GcodeTab};
use crate::ui::signcut_docker::fmt_len;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, PresetStore, Pressed, Scene};
use egui::{Color32, Ui, Vec2};
use tracedraw_core::geometry::{Rect, Shape as _};
use tracedraw_signcut::cnc::gcode::{CommentStyle, Direction, LeadIn, Machine, Origin, Side};

/// The preset list of machines (also offered by the hot wire window).
pub const MACHINES: &str = "gcode_machine";

/// Offer the three built-in machines while the machine presets are
/// empty.
pub fn seed_machines(store: &mut PresetStore) {
    let p = store.tool(MACHINES);
    if !p.items.is_empty() {
        return;
    }
    for m in [Machine::router(), Machine::laser(), Machine::hot_wire()] {
        if let Ok(v) = serde_json::to_value(&m) {
            p.items.insert(m.name.clone(), v);
        }
    }
    if p.current.is_none() {
        p.current = Some(Machine::router().name);
    }
}

/// Draw the window's body. Returns true to close it.
pub fn body(app: &mut App, ui: &mut Ui) -> bool {
    if !app.sc_cnc.win.found {
        app.sc_cnc.win.found = true;
        if app.sc_cnc.jobs.is_empty() {
            app.cnc_find_colors();
        }
    }
    let height = (ui.available_height() - 50.0).max(420.0);
    ui.horizontal_top(|ui| {
        let left_w = (ui.available_width() - kit::SIDE_W - 12.0).max(320.0);
        ui.vertical(|ui| {
            ui.set_width(left_w);
            let (scene, strip) = scene(app);
            kit::preview(ui, Vec2::new(left_w, height - 22.0), &scene);
            kit::status_strip(ui, left_w, &strip);
        });
        ui.vertical(|ui| {
            ui.set_width(kit::SIDE_W);
            settings(app, ui, height);
        });
    });
    if app.sc_cnc.preview && !app.sc_cnc.jobs.is_empty() {
        let paths = app.cnc_preview();
        app.docker_preview.extend(paths);
    }
    let can_save = !app.sc_cnc.jobs.is_empty();
    let mut save = false;
    let show = &mut app.sc_cnc.preview;
    let pressed = kit::bottom(ui, Buttons::Close, true, |ui| {
        save = kit::push(ui, &tr("signcut.wcnc_save"), can_save);
        ui.checkbox(show, tr("signcut.cnc_show_paths"));
    });
    if save {
        app.cnc_export_gcode();
    }
    matches!(pressed, Some(Pressed::Close))
}

/// The preview: the objects in grey, the tool paths in their jobs'
/// colours, the machine's table and zero point; the strip's text.
pub fn scene(app: &mut App) -> (Scene, String) {
    let mut scene = Scene::default();
    let plan = app.cnc_preview_colored();
    let units = app.units;
    let mut ext: Option<Rect> = None;
    let mut grow = |b: Rect| {
        if b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite() {
            ext = Some(ext.map_or(b, |e| e.union(b)));
        }
    };
    for p in app.cnc_outlines() {
        grow(p.bounding_box());
        scene.add(p, colors::ORIGINAL, 1.0);
    }
    for (c, p) in &plan.paths {
        grow(p.bounding_box());
        scene.add(p.clone(), path_color(*c), 1.5);
    }
    // The table and the zero point, in page space.
    let m = app.cnc_machine();
    let size = app.page_size();
    let off = m.origin.offset(size.width, size.height);
    let table = m.origin.table(m.width, m.height) - off;
    scene.add(rect_path(table), colors::SHEET, 1.0);
    let zero = tracedraw_core::geometry::Point::ZERO - off;
    let r = ext.map_or(5.0, |e| (e.width().max(e.height()) / 60.0).max(0.5));
    scene.add(
        tracedraw_core::geometry::Circle::new(zero, r).to_path(0.01),
        colors::SHEET_EDGE,
        2.0,
    );
    scene.extent = ext.or(Some(table));
    let last = app.sc_cnc.last;
    let strip = match last {
        Some(s) if s.cuts > 0 => {
            let mut t = trf(
                "signcut.wcnc_strip",
                &[
                    ("n", &s.cuts.to_string()),
                    ("len", &fmt_len(units, s.length)),
                    ("min", &format!("{:.1}", plan.minutes)),
                ],
            );
            if !s.on_table {
                t.push_str("    ");
                t.push_str(&tr("signcut.cnc_off_table"));
            }
            t
        }
        _ if app.sc_cnc.jobs.is_empty() => tr("signcut.wcnc_no_jobs"),
        _ => tr("signcut.cnc_nothing"),
    };
    if let Some(b) = last.and_then(|s| s.bounds) {
        scene.captions.push(format!(
            "{} x {}",
            fmt_len(units, b.width()),
            fmt_len(units, b.height())
        ));
    }
    (scene, strip)
}

/// A job's colour as drawn on the white preview (too light ones blue).
fn path_color(c: [u8; 3]) -> Color32 {
    let luma = 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2]);
    if luma > 200.0 {
        colors::ACCENT
    } else {
        Color32::from_rgb(c[0], c[1], c[2])
    }
}

/// The settings column: the machine preset row, the tabs, the units.
fn settings(app: &mut App, ui: &mut Ui, height: f32) {
    seed_machines(&mut app.sc_presets);
    kit::preset_row(ui, &mut app.sc_presets, MACHINES, &mut app.sc_cnc.machine);
    ui.add_space(4.0);
    let tab = &mut app.sc_cnc.win.gcode_tab;
    ui.horizontal(|ui| {
        for (t, key) in [
            (GcodeTab::Machine, "signcut.wcnc_tab_machine"),
            (GcodeTab::Tools, "signcut.wcnc_tab_tools"),
            (GcodeTab::Jobs, "signcut.wcnc_tab_jobs"),
        ] {
            if ui.selectable_label(*tab == t, tr(key)).clicked() {
                *tab = t;
            }
        }
    });
    let tab = *tab;
    egui::ScrollArea::vertical()
        .id_salt(("cnc_gcode_settings", tab as u8))
        .max_height((height - 110.0).max(200.0))
        .show(ui, |ui| match tab {
            GcodeTab::Machine => machine_tab(app, ui),
            GcodeTab::Tools => tools_tab(app, ui),
            GcodeTab::Jobs => jobs_tab(app, ui),
        });
    kit::units_row(ui, &mut app.units);
}

fn origin_key(o: Origin) -> &'static str {
    match o {
        Origin::BottomLeft => "signcut.cnc_origin_bl",
        Origin::BottomRight => "signcut.cnc_origin_br",
        Origin::TopLeft => "signcut.cnc_origin_tl",
        Origin::TopRight => "signcut.cnc_origin_tr",
        Origin::Center => "signcut.cnc_origin_c",
    }
}

fn machine_tab(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let m = &mut app.sc_cnc.machine;
    kit::group(ui, &tr("signcut.wcnc_machine"), |ui| {
        kit::row(ui, &tr("signcut.wcnc_name"), |ui| {
            ui.add_sized(
                [kit::FIELD_W, 22.0],
                egui::TextEdit::singleline(&mut m.name),
            )
        });
        kit::row(ui, &tr("signcut.wcnc_table_w"), |ui| {
            kit::length(ui, u, &mut m.width, 1.0, 100_000.0)
        });
        kit::row(ui, &tr("signcut.wcnc_table_h"), |ui| {
            kit::length(ui, u, &mut m.height, 1.0, 100_000.0)
        });
        kit::row(ui, &tr("signcut.wcnc_zero"), |ui| {
            let opts: Vec<(Origin, String)> = Origin::ALL
                .iter()
                .map(|o| (*o, tr(origin_key(*o))))
                .collect();
            kit::choice(ui, "cnc_origin", &mut m.origin, &opts)
        });
        ui.checkbox(&mut m.z_axis, tr("signcut.cnc_z_axis"));
    });
    kit::group(ui, &tr("signcut.wcnc_output"), |ui| {
        kit::row(ui, &tr("signcut.wcnc_decimals"), |ui| {
            kit::count(ui, &mut m.decimals, 0, 6)
        });
        kit::row(ui, &tr("signcut.wcnc_comments"), |ui| {
            let opts = [
                (CommentStyle::Parentheses, "( )".to_string()),
                (CommentStyle::Semicolon, ";".to_string()),
                (CommentStyle::Off, tr("signcut.cnc_comments_off")),
            ];
            kit::choice(ui, "cnc_comments", &mut m.comments, &opts)
        });
    });
    kit::group(ui, &tr("signcut.wcnc_lines"), |ui| {
        ui.label(
            egui::RichText::new(tr("signcut.cnc_template_hint"))
                .size(11.0)
                .color(crate::theme::Tokens::text_dim()),
        );
        let z = m.z_axis;
        for (key, text, on) in [
            ("signcut.wcnc_header", &mut m.header, true),
            ("signcut.wcnc_footer", &mut m.footer, true),
            ("signcut.wcnc_tool_change", &mut m.tool_change, true),
            ("signcut.wcnc_beam_on", &mut m.beam_on, !z),
            ("signcut.wcnc_beam_off", &mut m.beam_off, !z),
        ] {
            ui.add_enabled_ui(on, |ui| {
                ui.label(tr(key));
                ui.add(
                    egui::TextEdit::multiline(text)
                        .id_salt(("cnc_lines", key))
                        .desired_rows(2)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            });
        }
    });
}

fn tools_tab(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_cnc;
    if s.tool >= s.tools.len() {
        s.tool = 0;
    }
    kit::group(ui, &tr("signcut.wcnc_tools"), |ui| {
        kit::row(ui, &tr("signcut.wcnc_tool"), |ui| {
            let opts: Vec<(usize, String)> = s
                .tools
                .iter()
                .enumerate()
                .map(|(i, t)| (i, t.name.clone()))
                .collect();
            kit::choice(ui, "cnc_tool_pick", &mut s.tool, &opts)
        });
        ui.horizontal(|ui| {
            if ui.button(tr("signcut.cnc_add")).clicked() {
                let mut t = s.tools.get(s.tool).cloned().unwrap_or_default();
                t.name = format!("{} 2", t.name);
                s.tools.push(t);
                s.tool = s.tools.len() - 1;
            }
            if ui
                .add_enabled(
                    s.tools.len() > 1,
                    egui::Button::new(tr("signcut.cnc_remove")),
                )
                .clicked()
            {
                let i = s.tool;
                s.remove_tool(i);
            }
        });
    });
    let Some(t) = s.tools.get_mut(s.tool) else {
        return;
    };
    kit::group(ui, &tr("signcut.wcnc_tool"), |ui| {
        kit::row(ui, &tr("signcut.wcnc_name"), |ui| {
            ui.add_sized(
                [kit::FIELD_W, 22.0],
                egui::TextEdit::singleline(&mut t.name),
            )
        });
        kit::row(ui, &tr("signcut.wcnc_diameter"), |ui| {
            kit::length(ui, u, &mut t.diameter, 0.0, 1000.0)
        });
        kit::row(ui, &tr("signcut.wcnc_rpm"), |ui| {
            kit::number(ui, &mut t.rpm, 0.0, 100_000.0, 0)
        });
        kit::row(ui, &tr("signcut.wcnc_feed"), |ui| {
            kit::number(ui, &mut t.feed, 1.0, 100_000.0, 1)
        });
        kit::row(ui, &tr("signcut.wcnc_plunge"), |ui| {
            kit::number(ui, &mut t.plunge, 1.0, 100_000.0, 1)
        });
        kit::row(ui, &tr("signcut.wcnc_max_depth"), |ui| {
            kit::length(ui, u, &mut t.max_depth, 0.0, 1000.0)
        });
        kit::row(ui, &tr("signcut.wcnc_safe_z"), |ui| {
            kit::length(ui, u, &mut t.safe_z, 0.0, 1000.0)
        });
        kit::separator(ui);
        ui.checkbox(&mut t.park, tr("signcut.cnc_park"));
        ui.add_enabled_ui(t.park, |ui| {
            kit::row(ui, &tr("signcut.wcnc_park_x"), |ui| {
                kit::length(ui, u, &mut t.park_x, -100_000.0, 100_000.0)
            });
            kit::row(ui, &tr("signcut.wcnc_park_y"), |ui| {
                kit::length(ui, u, &mut t.park_y, -100_000.0, 100_000.0)
            });
        });
    });
}

fn jobs_tab(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let mut find = false;
    kit::group(ui, &tr("signcut.cnc_jobs"), |ui| {
        let s = &mut app.sc_cnc;
        let tools: Vec<String> = s.tools.iter().map(|t| t.name.clone()).collect();
        let n = s.jobs.len();
        if s.win.job >= n {
            s.win.job = n.saturating_sub(1);
        }
        if n == 0 {
            ui.label(tr("signcut.wcnc_no_jobs"));
        } else {
            egui::ScrollArea::vertical()
                .id_salt("cnc_jobs_table")
                .max_height(150.0)
                .show(ui, |ui| {
                    egui::Grid::new("cnc_jobs_grid")
                        .num_columns(4)
                        .striped(true)
                        .spacing([6.0, 2.0])
                        .show(ui, |ui| {
                            ui.label("");
                            ui.strong(tr("signcut.wcnc_col_colour"));
                            ui.strong(tr("signcut.wcnc_tool"));
                            ui.strong(tr("signcut.wcnc_depth"));
                            ui.end_row();
                            for (i, job) in s.jobs.iter_mut().enumerate() {
                                ui.push_id(("cnc_job_row", i), |ui| {
                                    ui.checkbox(&mut job.enabled, "");
                                });
                                ui.horizontal(|ui| {
                                    swatch(ui, job.color);
                                    if ui
                                        .selectable_label(s.win.job == i, hex(job.color))
                                        .clicked()
                                    {
                                        s.win.job = i;
                                    }
                                });
                                let name = tools.get(job.tool).cloned().unwrap_or_default();
                                ui.add(egui::Label::new(name).truncate());
                                ui.label(fmt_len(u, job.depth));
                                ui.end_row();
                            }
                        });
                });
        }
        ui.horizontal(|ui| {
            find = ui.button(tr("signcut.cnc_find_colors")).clicked();
            let i = s.win.job;
            if ui
                .add_enabled(n > 0 && i > 0, egui::Button::new("\u{2191}"))
                .on_hover_text(tr("signcut.wcnc_up"))
                .clicked()
            {
                s.jobs.swap(i - 1, i);
                s.win.job = i - 1;
            }
            if ui
                .add_enabled(i + 1 < n, egui::Button::new("\u{2193}"))
                .on_hover_text(tr("signcut.wcnc_down"))
                .clicked()
            {
                s.jobs.swap(i, i + 1);
                s.win.job = i + 1;
            }
            if ui
                .add_enabled(n > 0, egui::Button::new(tr("signcut.cnc_remove")))
                .clicked()
                && i < s.jobs.len()
            {
                s.jobs.remove(i);
            }
        });
        ui.label(
            egui::RichText::new(tr("signcut.cnc_jobs_hint"))
                .size(11.0)
                .color(crate::theme::Tokens::text_dim()),
        );
    });
    if find {
        app.cnc_find_colors();
    }
    let s = &mut app.sc_cnc;
    let tools: Vec<(usize, String)> = s
        .tools
        .iter()
        .enumerate()
        .map(|(i, t)| (i, t.name.clone()))
        .collect();
    let Some(job) = s.jobs.get_mut(s.win.job) else {
        return;
    };
    kit::group(
        ui,
        &trf("signcut.wcnc_job", &[("c", &hex(job.color))]),
        |ui| {
            kit::row(ui, &tr("signcut.wcnc_tool"), |ui| {
                kit::choice(ui, "cnc_job_tool", &mut job.tool, &tools)
            });
            kit::row(ui, &tr("signcut.wcnc_depth"), |ui| {
                kit::length(ui, u, &mut job.depth, 0.0, 1000.0)
            });
            kit::row(ui, &tr("signcut.wcnc_passes"), |ui| {
                let mut p = job.passes as usize;
                let r = kit::count(ui, &mut p, 1, 100);
                job.passes = u32::try_from(p).unwrap_or(1);
                r
            });
            kit::row(ui, &tr("signcut.wcnc_side"), |ui| {
                let opts = [
                    (Side::On, tr("signcut.cnc_side_on")),
                    (Side::Outside, tr("signcut.cnc_side_outside")),
                    (Side::Inside, tr("signcut.cnc_side_inside")),
                ];
                kit::choice(ui, "cnc_job_side", &mut job.side, &opts)
            });
            kit::row(ui, &tr("signcut.wcnc_direction"), |ui| {
                let opts = [
                    (Direction::Climb, tr("signcut.cnc_climb")),
                    (Direction::Conventional, tr("signcut.cnc_conventional")),
                ];
                kit::choice(ui, "cnc_job_dir", &mut job.direction, &opts)
            });
            ui.checkbox(&mut job.inner_first, tr("signcut.cnc_inner_first"));
            kit::separator(ui);
            kit::row(ui, &tr("signcut.wcnc_lead_in"), |ui| {
                let opts = [
                    (LeadIn::None, tr("signcut.cnc_lead_none")),
                    (LeadIn::Straight, tr("signcut.cnc_lead_straight")),
                    (LeadIn::Arc, tr("signcut.cnc_lead_arc")),
                ];
                kit::choice(ui, "cnc_job_lead", &mut job.lead_in, &opts)
            });
            ui.add_enabled_ui(job.lead_in != LeadIn::None, |ui| {
                kit::row(ui, &tr("signcut.wcnc_lead_length"), |ui| {
                    kit::length(ui, u, &mut job.lead_length, 0.0, 1000.0)
                });
            });
            kit::separator(ui);
            kit::row(ui, &tr("signcut.wcnc_bridges"), |ui| {
                let mut b = job.bridges as usize;
                let r = kit::count(ui, &mut b, 0, 100);
                job.bridges = u32::try_from(b).unwrap_or(0);
                r
            });
            ui.add_enabled_ui(job.bridges > 0, |ui| {
                kit::row(ui, &tr("signcut.wcnc_bridge_length"), |ui| {
                    kit::length(ui, u, &mut job.bridge_length, 0.0, 1000.0)
                });
                kit::row(ui, &tr("signcut.wcnc_bridge_height"), |ui| {
                    kit::length(ui, u, &mut job.bridge_height, 0.0, 1000.0)
                });
            });
        },
    );
}
