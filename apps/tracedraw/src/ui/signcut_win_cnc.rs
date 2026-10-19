//! The Sign & Cut tool windows of CNC and Laser. G-code has a big window
//! of its own (`signcut_win_cnc/gcode.rs`); Hot Wire Path, Finger
//! Joints, Centre Line, Finger-Joint Box and Door Panels share one
//! layout: the preview at the left with the yellow status strip, the
//! settings at the right (a preset row for the two generators), Apply
//! (work the result out into the preview), OK (apply it in one undo
//! step and close) and Cancel. See docs/behavior/sign-and-cut-cnc.md.

mod gcode;

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_cnc::{Cached, Stamp};
use crate::signcut_tools::ToolId;
use crate::ui::signcut_docker::fmt_len;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::{Color32, Ui, Vec2};
use tracedraw_core::geometry::{BezPath, Circle, Point, Rect, Shape as _};
use tracedraw_signcut::cnc::door::InnerShape;
use tracedraw_signcut::cnc::fingerbox;

/// The tools whose windows this file draws.
pub const TOOLS: &[ToolId] = &[
    ToolId::GCode,
    ToolId::CentreLine,
    ToolId::HotWire,
    ToolId::FingerJointBox,
    ToolId::FingerJoints,
    ToolId::DoorPanels,
];

/// A window's default size and whether it can be resized.
pub fn size(t: ToolId) -> (Vec2, bool) {
    match t {
        ToolId::GCode => (Vec2::new(1000.0, 700.0), true),
        ToolId::FingerJointBox | ToolId::DoorPanels => (Vec2::new(900.0, 600.0), true),
        _ => (Vec2::new(820.0, 520.0), true),
    }
}

/// The window's body. Returns true when it should close.
pub fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    if t == ToolId::GCode {
        return gcode::body(app, ui);
    }
    match tool_of(t) {
        Some(g) => tool_body(app, ui, g),
        None => true,
    }
}

/// The windows with Apply, OK and Cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gen {
    Wire,
    Joints,
    Centre,
    Box,
    Door,
}

fn tool_of(t: ToolId) -> Option<Gen> {
    match t {
        ToolId::HotWire => Some(Gen::Wire),
        ToolId::FingerJoints => Some(Gen::Joints),
        ToolId::CentreLine => Some(Gen::Centre),
        ToolId::FingerJointBox => Some(Gen::Box),
        ToolId::DoorPanels => Some(Gen::Door),
        _ => None,
    }
}

/// Draw a window of `g`. Returns true to close it.
fn tool_body(app: &mut App, ui: &mut Ui, g: Gen) -> bool {
    // The quick tools follow their settings; Centre Line waits for Apply.
    if g != Gen::Centre {
        refresh(app, g, false);
    }
    let height = (ui.available_height() - 60.0).max(360.0);
    ui.horizontal_top(|ui| {
        let left_w = (ui.available_width() - kit::SIDE_W - 12.0).max(300.0);
        ui.vertical(|ui| {
            ui.set_width(left_w);
            let (scene, strip) = scene(app, g);
            kit::preview(ui, Vec2::new(left_w, height - 22.0), &scene);
            kit::status_strip(ui, left_w, &strip);
        });
        ui.vertical(|ui| {
            ui.set_width(kit::SIDE_W);
            egui::ScrollArea::vertical()
                .id_salt(("cnc_settings", g as u8))
                .max_height((height - 40.0).max(200.0))
                .show(ui, |ui| settings(app, ui, g));
            kit::units_row(ui, &mut app.units);
        });
    });
    kit::progress(ui, if fresh(app, g) { 1.0 } else { 0.0 });
    let enabled = match g {
        Gen::Wire | Gen::Joints | Gen::Centre => !app.cnc_outlines().is_empty(),
        Gen::Box | Gen::Door => true,
    };
    let mut save = false;
    let pressed = kit::bottom(ui, Buttons::ApplyOkCancel, enabled, |ui| {
        if g == Gen::Wire {
            save = kit::push(ui, &tr("signcut.wcnc_save"), enabled);
        }
    });
    if save {
        app.cnc_export_wire_gcode();
    }
    match pressed {
        Some(p) => press(app, g, p),
        None => false,
    }
}

/// What a bottom button does. Returns true to close the window.
fn press(app: &mut App, g: Gen, p: Pressed) -> bool {
    match p {
        Pressed::Apply => {
            refresh(app, g, true);
            app.status = scene(app, g).1;
            false
        }
        Pressed::Ok => {
            commit(app, g);
            true
        }
        _ => {
            clear(app, g);
            true
        }
    }
}

/// The stored result of `slot` when it was made from `stamp`.
fn current<S: PartialEq, T: Clone>(slot: &Cached<S, T>, stamp: &Stamp<S>) -> Option<T> {
    slot.as_ref()
        .filter(|(s, _)| s == stamp)
        .map(|(_, t)| t.clone())
}

/// Work the preview of `g` out again when what it depends on changed (or
/// always with `force`). The drawing is not changed.
fn refresh(app: &mut App, g: Gen, force: bool) {
    let c = &app.sc_cnc;
    match g {
        Gen::Wire => {
            let st = app.cnc_stamp((c.wire_start_x, c.wire_start_y));
            if force || current(&app.sc_cnc.win.wire, &st).is_none() {
                let r = app.cnc_wire_points();
                app.sc_cnc.win.wire = Some((st, r));
            }
        }
        Gen::Joints => {
            let st = app.cnc_stamp((c.joint, c.joint_pair));
            if force || current(&app.sc_cnc.win.joints, &st).is_none() {
                let r = app.cnc_joints_compute();
                app.sc_cnc.win.joints = Some((st, r));
            }
        }
        Gen::Centre => {
            let st = app.cnc_stamp(c.centre);
            if force || current(&app.sc_cnc.win.centre, &st).is_none() {
                let r = app.cnc_center_compute();
                app.sc_cnc.win.centre = Some((st, r));
            }
        }
        Gen::Box => {
            let st = app.cnc_stamp(c.box_spec);
            if force || current(&app.sc_cnc.win.boxes, &st).is_none() {
                let r = app.cnc_box_compute();
                app.sc_cnc.win.boxes = Some((st, r));
            }
        }
        Gen::Door => {
            let st = app.cnc_stamp(c.door);
            if force || current(&app.sc_cnc.win.door, &st).is_none() {
                let r = app.cnc_door_compute();
                app.sc_cnc.win.door = Some((st, r));
            }
        }
    }
}

/// Whether the preview of `g` shows the drawing and settings as they are.
fn fresh(app: &App, g: Gen) -> bool {
    let c = &app.sc_cnc;
    let w = &c.win;
    match g {
        Gen::Wire => current(&w.wire, &app.cnc_stamp((c.wire_start_x, c.wire_start_y))).is_some(),
        Gen::Joints => current(&w.joints, &app.cnc_stamp((c.joint, c.joint_pair))).is_some(),
        Gen::Centre => current(&w.centre, &app.cnc_stamp(c.centre)).is_some(),
        Gen::Box => current(&w.boxes, &app.cnc_stamp(c.box_spec)).is_some(),
        Gen::Door => current(&w.door, &app.cnc_stamp(c.door)).is_some(),
    }
}

/// Forget the preview of `g`.
fn clear(app: &mut App, g: Gen) {
    let w = &mut app.sc_cnc.win;
    match g {
        Gen::Wire => w.wire = None,
        Gen::Joints => w.joints = None,
        Gen::Centre => w.centre = None,
        Gen::Box => w.boxes = None,
        Gen::Door => w.door = None,
    }
}

/// OK: apply the previewed result when it is current, else work it out
/// now; one undo step.
fn commit(app: &mut App, g: Gen) {
    refresh(app, g, false);
    let w = std::mem::take(&mut app.sc_cnc.win);
    app.sc_cnc.win = crate::signcut_cnc::WinState {
        gcode_tab: w.gcode_tab,
        job: w.job,
        found: w.found,
        ..Default::default()
    };
    match g {
        Gen::Wire => {
            let pts = w.wire.and_then(|(_, r)| r);
            app.cnc_wire_commit(pts.as_deref());
        }
        Gen::Joints => {
            let r = w
                .joints
                .map_or_else(|| Err(tr("signcut.cnc_joint_none")), |(_, r)| r);
            app.cnc_joints_commit(r);
        }
        Gen::Centre => {
            let r = w.centre.map(|(_, r)| r).unwrap_or_default();
            app.cnc_center_commit(r);
        }
        Gen::Box => {
            app.cnc_box_commit(w.boxes.and_then(|(_, r)| r));
        }
        Gen::Door => {
            app.cnc_door_commit(w.door.and_then(|(_, r)| r));
        }
    }
}

/// A rectangle as a closed path.
pub(crate) fn rect_path(r: Rect) -> BezPath {
    let r = r.abs();
    let mut p = BezPath::new();
    p.move_to((r.x0, r.y0));
    p.line_to((r.x1, r.y0));
    p.line_to((r.x1, r.y1));
    p.line_to((r.x0, r.y1));
    p.close_path();
    p
}

/// A small colour square.
pub(crate) fn swatch(ui: &mut Ui, c: [u8; 3]) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(r, 2.0, Color32::from_rgb(c[0], c[1], c[2]));
    ui.painter().rect_stroke(
        r,
        2.0,
        egui::Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
}

/// A hint under a group's fields.
fn hint(ui: &mut Ui, key: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr(key))
                .size(11.0)
                .color(crate::theme::Tokens::text_dim()),
        )
        .wrap(),
    );
}

/// Bounds of the paths, when finite.
fn bounds<'a>(paths: impl IntoIterator<Item = &'a BezPath>) -> Option<Rect> {
    let mut ext: Option<Rect> = None;
    for p in paths {
        let b = p.bounding_box();
        if b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite() {
            ext = Some(ext.map_or(b, |e| e.union(b)));
        }
    }
    ext
}

/// Colours of generated cut and groove lines in the previews.
const RED: Color32 = Color32::from_rgb(0xE0, 0x20, 0x20);
const BLUE: Color32 = Color32::from_rgb(0x1E, 0x40, 0xE0);

/// The preview of `g` and the strip's text.
fn scene(app: &App, g: Gen) -> (Scene, String) {
    let u = app.units;
    let c = &app.sc_cnc;
    let w = &c.win;
    let mut scene = Scene::default();
    let originals = |scene: &mut Scene| {
        let o = app.cnc_outlines();
        for p in &o {
            scene.add(p.clone(), colors::ORIGINAL, 1.0);
        }
        o
    };
    match g {
        Gen::Wire => {
            let o = originals(&mut scene);
            let start = Point::new(c.wire_start_x, c.wire_start_y);
            let ext = bounds(&o).map(|b| b.union(Rect::from_points(start, start)));
            let r = ext.map_or(2.0, |e| (e.width().max(e.height()) / 80.0).max(0.2));
            let st = app.cnc_stamp((c.wire_start_x, c.wire_start_y));
            let strip = match current(&w.wire, &st) {
                Some(Some(pts)) => {
                    let len = tracedraw_signcut::cnc::polyline_length(&pts, false);
                    scene.add(crate::signcut_cnc::polyline(&pts, false), RED, 1.5);
                    scene.captions.push(fmt_len(u, len));
                    trf(
                        "signcut.wcnc_wire_strip",
                        &[
                            ("n", &closed_count(&o).to_string()),
                            ("len", &fmt_len(u, len)),
                        ],
                    )
                }
                Some(None) => tr("signcut.cnc_no_closed"),
                None => tr("signcut.wcnc_press_apply"),
            };
            if start.x.is_finite() && start.y.is_finite() {
                scene.add(Circle::new(start, r).to_path(0.01), RED, 2.0);
            }
            (scene, strip)
        }
        Gen::Joints => {
            let st = app.cnc_stamp((c.joint, c.joint_pair));
            for p in app.cnc_joint_targets() {
                scene.add(p, colors::ORIGINAL, 1.0);
            }
            let strip = match current(&w.joints, &st) {
                Some(Ok(j)) => {
                    for (_, p) in &j.changed {
                        scene.add(p.clone(), colors::PART, 1.0);
                    }
                    for (a, b) in &j.edges {
                        let mut e = BezPath::new();
                        e.move_to(*a);
                        e.line_to(*b);
                        scene.add(e, BLUE, 3.0);
                    }
                    trf(
                        "signcut.wcnc_joint_strip",
                        &[
                            ("n", &j.changed.len().to_string()),
                            ("e", &j.edges.len().to_string()),
                        ],
                    )
                }
                Some(Err(e)) => e,
                None => tr("signcut.wcnc_press_apply"),
            };
            (scene, strip)
        }
        Gen::Centre => {
            let o = originals(&mut scene);
            let st = app.cnc_stamp(c.centre);
            let strip = match current(&w.centre, &st) {
                Some(lines) if !lines.is_empty() => {
                    for (p, _) in &lines {
                        scene.add(p.clone(), BLUE, 1.5);
                    }
                    trf(
                        "signcut.wcnc_centre_strip",
                        &[("n", &lines.len().to_string())],
                    )
                }
                Some(_) => tr("signcut.cnc_no_center"),
                None if o.is_empty() => tr("signcut.cnc_no_center"),
                None => trf(
                    "signcut.wcnc_centre_ready",
                    &[("n", &closed_count(&o).to_string())],
                ),
            };
            (scene, strip)
        }
        Gen::Box => {
            let st = app.cnc_stamp(c.box_spec);
            let spec = c.box_spec;
            let strip = match current(&w.boxes, &st) {
                Some(Some(panels)) => {
                    for (_, p) in &panels {
                        scene.add(p.clone(), RED, 1.0);
                    }
                    let (ow, od, oh) = fingerbox::outside_size(&spec);
                    let t = spec.thickness;
                    let n = |l: f64| fingerbox::finger_count(l, t, spec.finger).to_string();
                    scene.captions.push(format!(
                        "{} x {} x {}",
                        fmt_len(u, ow),
                        fmt_len(u, od),
                        fmt_len(u, oh)
                    ));
                    scene.captions.push(trf(
                        "signcut.wcnc_box_fingers",
                        &[("w", &n(ow)), ("d", &n(od)), ("h", &n(oh))],
                    ));
                    if let Some(b) = bounds(panels.iter().map(|(_, p)| p)) {
                        trf(
                            "signcut.wcnc_box_strip",
                            &[
                                ("n", &panels.len().to_string()),
                                ("w", &fmt_len(u, b.width())),
                                ("h", &fmt_len(u, b.height())),
                            ],
                        )
                    } else {
                        tr("signcut.cnc_box_bad")
                    }
                }
                Some(None) => tr("signcut.cnc_box_bad"),
                None => tr("signcut.wcnc_press_apply"),
            };
            (scene, strip)
        }
        Gen::Door => {
            if let [id] = &app.selection[..] {
                if let Some(s) = app.doc().shape_on_page(*id) {
                    scene.add(s.page_path(), colors::ORIGINAL, 1.0);
                }
            }
            let st = app.cnc_stamp(c.door);
            let strip = match current(&w.door, &st) {
                Some(Some(d)) => {
                    let b = d.outer.bounding_box();
                    scene.add(d.outer.clone(), RED, 1.0);
                    scene.add(d.inner.clone(), RED, 1.0);
                    for groove in &d.grooves {
                        scene.add(groove.clone(), BLUE, 1.0);
                    }
                    scene.captions.push(format!(
                        "{} x {}",
                        fmt_len(u, b.width()),
                        fmt_len(u, b.height())
                    ));
                    trf(
                        "signcut.wcnc_door_strip",
                        &[
                            ("w", &fmt_len(u, b.width())),
                            ("h", &fmt_len(u, b.height())),
                            ("n", &d.grooves.len().to_string()),
                        ],
                    )
                }
                Some(None) => tr("signcut.cnc_door_bad"),
                None => tr("signcut.wcnc_press_apply"),
            };
            (scene, strip)
        }
    }
}

/// How many closed subpaths the outlines have.
fn closed_count(outlines: &[BezPath]) -> usize {
    outlines
        .iter()
        .flat_map(|p| tracedraw_signcut::cnc::flatten(p, 0.1))
        .filter(|c| c.closed)
        .count()
}

/// The settings column of `g`.
fn settings(app: &mut App, ui: &mut Ui, g: Gen) {
    let u = app.units;
    match g {
        Gen::Wire => {
            let s = &mut app.sc_cnc;
            kit::group(ui, &tr("signcut.wcnc_start"), |ui| {
                kit::row(ui, "X", |ui| {
                    kit::length(ui, u, &mut s.wire_start_x, -1_000_000.0, 1_000_000.0)
                });
                kit::row(ui, "Y", |ui| {
                    kit::length(ui, u, &mut s.wire_start_y, -1_000_000.0, 1_000_000.0)
                });
                hint(ui, "signcut.cnc_wire_hint");
            });
            gcode::seed_machines(&mut app.sc_presets);
            let names: Vec<String> = app
                .sc_presets
                .tool(gcode::MACHINES)
                .items
                .keys()
                .cloned()
                .collect();
            let s = &mut app.sc_cnc;
            let mut picked = None;
            kit::group(ui, &tr("signcut.wcnc_gcode"), |ui| {
                kit::row(ui, &tr("signcut.wcnc_machine"), |ui| {
                    let mut at = names
                        .iter()
                        .position(|n| *n == s.wire_machine.name)
                        .unwrap_or(usize::MAX);
                    let mut opts: Vec<(usize, String)> =
                        names.iter().cloned().enumerate().collect();
                    if at == usize::MAX {
                        opts.insert(0, (usize::MAX, s.wire_machine.name.clone()));
                    }
                    if kit::choice(ui, "cnc_wire_machine", &mut at, &opts) {
                        picked = names.get(at).cloned();
                    }
                });
                kit::row(ui, &tr("signcut.wcnc_tool"), |ui| {
                    let opts: Vec<(usize, String)> = s
                        .tools
                        .iter()
                        .enumerate()
                        .map(|(i, t)| (i, t.name.clone()))
                        .collect();
                    kit::choice(ui, "cnc_wire_tool", &mut s.wire_tool, &opts)
                });
            });
            if let Some(name) = picked {
                let v = app
                    .sc_presets
                    .tool(gcode::MACHINES)
                    .items
                    .get(&name)
                    .cloned();
                if let Some(m) = v.and_then(|v| serde_json::from_value(v).ok()) {
                    app.sc_cnc.wire_machine = m;
                }
            }
        }
        Gen::Joints => {
            let s = &mut app.sc_cnc;
            kit::group(ui, &tr("signcut.wcnc_edges"), |ui| {
                kit::row(ui, &tr("signcut.wcnc_method"), |ui| {
                    let opts = [
                        (true, tr("signcut.wcnc_joint_pair")),
                        (false, tr("signcut.wcnc_joint_all")),
                    ];
                    kit::choice(ui, "cnc_joint_method", &mut s.joint_pair, &opts)
                });
                hint(
                    ui,
                    if s.joint_pair {
                        "signcut.cnc_joint_pair"
                    } else {
                        "signcut.cnc_joint_all"
                    },
                );
                kit::row(ui, &tr("signcut.cnc_joint_min"), |ui| {
                    kit::length(ui, u, &mut s.joint.min_length, 0.0, 100_000.0)
                });
            });
            let j = &mut s.joint;
            kit::group(ui, &tr("signcut.wcnc_fingers"), |ui| {
                kit::row(ui, &tr("signcut.cnc_finger"), |ui| {
                    kit::length(ui, u, &mut j.finger, 0.5, 10_000.0)
                });
                kit::row(ui, &tr("signcut.cnc_joint_depth"), |ui| {
                    kit::length(ui, u, &mut j.depth, 0.1, 10_000.0)
                });
                ui.checkbox(&mut j.flip, tr("signcut.cnc_joint_flip"));
                hint(ui, "signcut.cnc_joints_hint");
            });
        }
        Gen::Centre => {
            let c = &mut app.sc_cnc.centre;
            kit::group(ui, &tr("signcut.wcnc_centre"), |ui| {
                kit::row(ui, &tr("signcut.wcnc_tolerance"), |ui| {
                    kit::length(ui, u, &mut c.tolerance, 0.01, 100.0)
                });
                kit::row(ui, &tr("signcut.cnc_center_prune"), |ui| {
                    kit::length(ui, u, &mut c.prune, 0.0, 10_000.0)
                });
                hint(ui, "signcut.cnc_center_hint");
            });
        }
        Gen::Box => {
            kit::preset_row(
                ui,
                &mut app.sc_presets,
                "finger_joint_box",
                &mut app.sc_cnc.box_spec,
            );
            let b = &mut app.sc_cnc.box_spec;
            kit::group(ui, &tr("signcut.wcnc_box_size"), |ui| {
                kit::row(ui, &tr("signcut.width"), |ui| {
                    kit::length(ui, u, &mut b.width, 1.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.cnc_box_depth"), |ui| {
                    kit::length(ui, u, &mut b.depth, 1.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.height"), |ui| {
                    kit::length(ui, u, &mut b.height, 1.0, 100_000.0)
                });
                ui.checkbox(&mut b.inside, tr("signcut.cnc_box_inside"));
                ui.checkbox(&mut b.open_top, tr("signcut.cnc_box_open"));
            });
            kit::group(ui, &tr("signcut.wcnc_material"), |ui| {
                kit::row(ui, &tr("signcut.cnc_thickness"), |ui| {
                    kit::length(ui, u, &mut b.thickness, 0.1, 1000.0)
                });
                kit::row(ui, &tr("signcut.cnc_finger"), |ui| {
                    kit::length(ui, u, &mut b.finger, 0.5, 10_000.0)
                });
                kit::row(ui, &tr("signcut.cnc_kerf"), |ui| {
                    kit::length(ui, u, &mut b.kerf, 0.0, 100.0)
                });
            });
            kit::group(ui, &tr("signcut.wcnc_layout"), |ui| {
                kit::row(ui, &tr("signcut.cnc_gap"), |ui| {
                    kit::length(ui, u, &mut b.gap, 0.0, 10_000.0)
                });
            });
        }
        Gen::Door => {
            kit::preset_row(ui, &mut app.sc_presets, "door_panels", &mut app.sc_cnc.door);
            let from_object = app.selection.len() == 1;
            let d = &mut app.sc_cnc.door;
            kit::group(ui, &tr("signcut.wcnc_door_size"), |ui| {
                ui.add_enabled_ui(!from_object, |ui| {
                    kit::row(ui, &tr("signcut.width"), |ui| {
                        kit::length(ui, u, &mut d.width, 1.0, 100_000.0)
                    });
                    kit::row(ui, &tr("signcut.height"), |ui| {
                        kit::length(ui, u, &mut d.height, 1.0, 100_000.0)
                    });
                });
                hint(ui, "signcut.cnc_door_hint");
            });
            kit::group(ui, &tr("signcut.wcnc_frame"), |ui| {
                kit::row(ui, &tr("signcut.cnc_door_frame"), |ui| {
                    kit::length(ui, u, &mut d.frame, 0.0, 10_000.0)
                });
                kit::row(ui, &tr("signcut.cnc_door_inner"), |ui| {
                    let opts = [
                        (InnerShape::Square, tr("signcut.cnc_door_square")),
                        (InnerShape::Rounded, tr("signcut.cnc_door_rounded")),
                        (InnerShape::Arch, tr("signcut.cnc_door_arch")),
                    ];
                    kit::choice(ui, "cnc_door_inner", &mut d.inner, &opts)
                });
                match d.inner {
                    InnerShape::Rounded => {
                        kit::row(ui, &tr("signcut.cnc_door_radius"), |ui| {
                            kit::length(ui, u, &mut d.radius, 0.0, 10_000.0)
                        });
                    }
                    InnerShape::Arch => {
                        kit::row(ui, &tr("signcut.cnc_door_rise"), |ui| {
                            kit::length(ui, u, &mut d.arch, 0.0, 10_000.0)
                        });
                    }
                    InnerShape::Square => {}
                }
            });
            kit::group(ui, &tr("signcut.wcnc_grooves"), |ui| {
                kit::row(ui, &tr("signcut.cnc_door_grooves"), |ui| {
                    let mut n = d.grooves as usize;
                    let r = kit::count(ui, &mut n, 0, 20);
                    d.grooves = u32::try_from(n).unwrap_or(0);
                    r
                });
                ui.add_enabled_ui(d.grooves > 0, |ui| {
                    kit::row(ui, &tr("signcut.cnc_door_groove_offset"), |ui| {
                        kit::length(ui, u, &mut d.groove_offset, 0.0, 10_000.0)
                    });
                    kit::row(ui, &tr("signcut.cnc_door_groove_spacing"), |ui| {
                        kit::length(ui, u, &mut d.groove_spacing, 0.0, 10_000.0)
                    });
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Color, Command, ShapeId, ShapeKind, Stroke};

    const GENS: [Gen; 5] = [Gen::Wire, Gen::Joints, Gen::Centre, Gen::Box, Gen::Door];

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("layer")
    }

    fn app_with_parts() -> (App, Vec<ShapeId>) {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 100.0, 50.0);
        let b = rect(&mut app, 10.0, 70.0, 100.0, 50.0);
        app.run(Command::SetStroke {
            shapes: vec![a],
            stroke: Some(Stroke::new(Color::rgb8(255, 0, 0), 0.1)),
        });
        app.select(vec![a, b]);
        (app, vec![a, b])
    }

    fn draw(app: &mut App, t: ToolId) {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            assert!(!body(app, ui, t));
        });
        out.textures_delta.clear();
    }

    #[test]
    fn every_window_draws_every_tab_and_method() {
        use crate::signcut_cnc::GcodeTab;
        let (mut app, _) = app_with_parts();
        for tab in [GcodeTab::Machine, GcodeTab::Tools, GcodeTab::Jobs] {
            app.sc_cnc.win.gcode_tab = tab;
            draw(&mut app, ToolId::GCode);
        }
        // The window found the outline colours when it first drew.
        assert_eq!(app.sc_cnc.jobs.len(), 2);
        for pair in [true, false] {
            app.sc_cnc.joint_pair = pair;
            draw(&mut app, ToolId::FingerJoints);
        }
        for inner in [InnerShape::Square, InnerShape::Rounded, InnerShape::Arch] {
            app.sc_cnc.door.inner = inner;
            draw(&mut app, ToolId::DoorPanels);
        }
        for t in [ToolId::HotWire, ToolId::CentreLine, ToolId::FingerJointBox] {
            draw(&mut app, t);
        }
        // Nothing selected and an empty page draw too.
        let mut empty = App::headless();
        for t in TOOLS {
            draw(&mut empty, *t);
        }
    }

    #[test]
    fn gcode_preview_shows_tool_paths_and_the_table() {
        let (mut app, _) = app_with_parts();
        app.cnc_find_colors();
        let (scene, strip) = gcode::scene(&mut app);
        // Two outlines, two tool paths, the table and the zero point.
        assert_eq!(scene.items.len(), 6);
        assert!(strip.contains('2'), "{strip}");
        let rev = app.engine.revision();
        draw(&mut app, ToolId::GCode);
        assert_eq!(app.engine.revision(), rev);
        // The tool paths go on the drawing while the box is ticked.
        assert!(!app.docker_preview.is_empty());
    }

    #[test]
    fn machine_presets_offer_the_built_in_machines() {
        let mut app = App::headless();
        gcode::seed_machines(&mut app.sc_presets);
        let p = app.sc_presets.tool(gcode::MACHINES);
        assert_eq!(p.items.len(), 3);
        assert_eq!(p.current.as_deref(), Some("Router"));
        let laser = p.items.get("Laser").cloned().expect("laser");
        let m: tracedraw_signcut::cnc::gcode::Machine =
            serde_json::from_value(laser).expect("machine");
        assert!(!m.z_axis);
        // A list the user changed is left alone.
        p.items.remove("Laser");
        gcode::seed_machines(&mut app.sc_presets);
        assert_eq!(app.sc_presets.tool(gcode::MACHINES).items.len(), 2);
    }

    #[test]
    fn apply_previews_and_changes_nothing() {
        let (mut app, _) = app_with_parts();
        app.sc_cnc.joint.min_length = 50.0;
        let rev = app.engine.revision();
        for g in GENS {
            assert!(!press(&mut app, g, Pressed::Apply));
            assert!(fresh(&app, g), "{g:?}");
            let (scene, strip) = scene(&app, g);
            assert!(!scene.items.is_empty(), "{g:?}");
            assert!(!strip.is_empty());
        }
        assert_eq!(app.engine.revision(), rev);
        assert!(app
            .sc_cnc
            .win
            .centre
            .as_ref()
            .is_some_and(|(_, l)| l.len() == 2));
        // A changed setting makes the preview out of date.
        app.sc_cnc.centre.prune += 1.0;
        assert!(!fresh(&app, Gen::Centre));
        app.sc_cnc.box_spec.width += 10.0;
        assert!(!fresh(&app, Gen::Box));
    }

    #[test]
    fn ok_applies_in_one_undo_step() {
        for (g, label) in [
            (Gen::Wire, "Hot Wire Path"),
            (Gen::Joints, "Finger Joints"),
            (Gen::Centre, "Centre Line"),
            (Gen::Box, "Finger-Joint Box"),
            (Gen::Door, "Door Panel"),
        ] {
            let (mut app, _) = app_with_parts();
            assert!(!press(&mut app, g, Pressed::Apply));
            let rev = app.engine.revision();
            assert!(press(&mut app, g, Pressed::Ok));
            assert_eq!(app.engine.undo_label(), Some(label), "{g:?}");
            assert!(app.engine.revision() > rev);
            assert!(!fresh(&app, g));
        }
    }

    #[test]
    fn ok_without_apply_works_it_out_and_cancel_changes_nothing() {
        let (mut app, _) = app_with_parts();
        assert!(press(&mut app, Gen::Centre, Pressed::Ok));
        assert_eq!(app.engine.undo_label(), Some("Centre Line"));
        let (mut app, _) = app_with_parts();
        let rev = app.engine.revision();
        assert!(!press(&mut app, Gen::Wire, Pressed::Apply));
        assert!(press(&mut app, Gen::Wire, Pressed::Cancel));
        assert_eq!(app.engine.revision(), rev);
        assert!(app.sc_cnc.win.wire.is_none());
    }
}
