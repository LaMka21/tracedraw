//! The Sign & Cut tool windows of sign making: LED Modules and Neon Tubes
//! (tool windows with a catalogue or tabs and command buttons, Close),
//! Letter Returns, Acrylic Mould and Sign Box (a preview at the left, the
//! settings with a preset row and a method at the right; Apply works the
//! result out into the preview, OK applies it in one undo step), Smart
//! Divide and Carrier (small windows; Apply applies, the grid or the
//! carrier is marked on the drawing). See
//! docs/behavior/sign-and-cut-windows.md and sign-and-cut-sign.md.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::signcut_sign::{led_summary, Plan, Role, TileMethod, TileOrigin, NEON_DIAMETERS};
use crate::signcut_tools::ToolId;
use crate::ui::signcut_docker::{fmt_area, fmt_len};
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::{Color32, RichText, Ui, Vec2};
use tracedraw_core::geometry::{BezPath, Point, Shape as _};
use tracedraw_core::ShapeId;
use tracedraw_signcut::sign::carrier::CarrierMethod;
use tracedraw_signcut::sign::led::{LedMode, Module};
use tracedraw_signcut::sign::neon::NeonKind;
use tracedraw_signcut::sign::returns::{ReturnFace, ReturnMethod};
use tracedraw_signcut::sign::sign_box::BoxMethod;
use tracedraw_signcut::sign::Sides;

/// The tools whose windows this file draws.
pub const TOOLS: &[ToolId] = &[
    ToolId::LedModules,
    ToolId::NeonTubes,
    ToolId::LetterReturns,
    ToolId::AcrylicMould,
    ToolId::SignBox,
    ToolId::SmartDivide,
    ToolId::Carrier,
];

/// What a cached result was made from: the drawing's revision, the
/// selection and the settings.
type Key = (u64, Vec<ShapeId>, String);

/// A window with a preview: the worked-out plan and the last result.
#[derive(Debug, Clone, Default)]
pub struct PreviewState {
    pub plan: Option<Plan>,
    /// Why the last Apply made nothing.
    pub error: Option<String>,
    /// Strips and their length after the last OK.
    pub report: Option<(usize, f64)>,
    /// The preview before Apply.
    cache: Option<(Key, Scene)>,
}

/// The tabs of the LED window's right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LedTab {
    Curve,
    #[default]
    Fill,
    Tools,
}

/// The LED window.
#[derive(Debug, Clone, Default)]
pub struct LedState {
    pub tab: LedTab,
    /// The left side shows the module editor (else the catalogue).
    pub edit: bool,
    /// Show the layout on the drawing.
    pub show: bool,
    /// Gap of the Fix Overlaps command.
    pub fix_gap: f64,
    cache: Option<(Key, Vec<BezPath>)>,
}

/// The tabs of the neon window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeonTab {
    #[default]
    Neon,
    Tools,
    Options,
}

/// The tabs of the sign box window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxTab {
    #[default]
    Size,
    Options,
}

/// Every sign making window's state.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub led: LedState,
    pub neon_tab: NeonTab,
    pub box_tab: BoxTab,
    pub sign_box: PreviewState,
    pub returns: PreviewState,
    pub mould: PreviewState,
    /// Show the carrier on the drawing.
    pub carrier_show: bool,
    tiles_cache: Option<(Key, Vec<BezPath>, String)>,
    carrier_cache: Option<(Key, BezPath)>,
}

/// A window's default size and whether it can be resized.
pub fn size(t: ToolId) -> (Vec2, bool) {
    match t {
        ToolId::LedModules => (Vec2::new(640.0, 480.0), false),
        ToolId::NeonTubes => (Vec2::new(330.0, 520.0), false),
        ToolId::LetterReturns | ToolId::AcrylicMould => (Vec2::new(1000.0, 700.0), true),
        ToolId::SignBox => (Vec2::new(960.0, 720.0), true),
        ToolId::SmartDivide => (Vec2::new(350.0, 430.0), false),
        ToolId::Carrier => (Vec2::new(330.0, 470.0), false),
        _ => (Vec2::new(380.0, 300.0), true),
    }
}

/// The window's body. Returns true when it should close.
pub fn body(app: &mut App, ui: &mut Ui, t: ToolId) -> bool {
    match t {
        ToolId::LedModules => led_window(app, ui),
        ToolId::NeonTubes => neon_window(app, ui),
        ToolId::LetterReturns => preview_window(app, ui, Kind::Returns),
        ToolId::AcrylicMould => preview_window(app, ui, Kind::Mould),
        ToolId::SignBox => preview_window(app, ui, Kind::SignBox),
        ToolId::SmartDivide => tiles_window(app, ui),
        ToolId::Carrier => carrier_window(app, ui),
        _ => true,
    }
}

/// The cache key of the selection and `settings`.
fn key<T: serde::Serialize>(app: &App, settings: &T) -> Key {
    (
        app.engine.revision(),
        app.sign_targets(),
        serde_json::to_string(settings).unwrap_or_default(),
    )
}

/// A two-way switch drawn as tabs.
fn tabs<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, String)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (v, label) in options {
            if ui.selectable_label(*value == *v, label).clicked() && *value != *v {
                *value = *v;
                changed = true;
            }
        }
    });
    changed
}

/// A heading in bold.
fn heading(ui: &mut Ui, key: &str) {
    ui.vertical_centered(|ui| ui.label(RichText::new(tr(key)).strong()));
}

/// A full-width command button.
fn command(ui: &mut Ui, key: &str, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(tr(key)).min_size(Vec2::new(ui.available_width(), 26.0)),
    )
    .clicked()
}

/// Four side fields with an All field and a Set button.
fn sides_group(ui: &mut Ui, title: &str, u: Units, s: &mut Sides, id: &str) {
    kit::group(ui, title, |ui| {
        for (key, v) in [
            ("signcut.wsign_left", &mut s.left),
            ("signcut.wsign_right", &mut s.right),
            ("signcut.wsign_bottom", &mut s.bottom),
            ("signcut.wsign_top", &mut s.top),
        ] {
            kit::row(ui, &tr(key), |ui| kit::length(ui, u, v, 0.0, 10_000.0));
        }
        kit::separator(ui);
        let all_id = egui::Id::new(("signcut_sides_all", id));
        let mut all: f64 = ui.data(|d| d.get_temp(all_id)).unwrap_or(s.left);
        ui.horizontal(|ui| {
            if ui.button(tr("signcut.wsign_set")).clicked() {
                *s = Sides::all(all);
            }
            ui.label(tr("signcut.wsign_all"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                kit::length(ui, u, &mut all, 0.0, 10_000.0);
            });
        });
        ui.data_mut(|d| d.insert_temp(all_id, all));
    });
}

// ---------------------------------------------------------------------
// Letter Returns, Acrylic Mould, Sign Box: preview windows.

/// The three preview windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Returns,
    Mould,
    SignBox,
}

fn state(app: &mut App, kind: Kind) -> &mut PreviewState {
    let w = &mut app.sc_sign.win;
    match kind {
        Kind::Returns => &mut w.returns,
        Kind::Mould => &mut w.mould,
        Kind::SignBox => &mut w.sign_box,
    }
}

/// The settings of a preview window as JSON (to see a change).
fn settings_json(app: &App, kind: Kind) -> Option<serde_json::Value> {
    let s = &app.sc_sign;
    match kind {
        Kind::Returns => serde_json::to_value(s.returns).ok(),
        Kind::Mould => serde_json::to_value(s.mould).ok(),
        Kind::SignBox => serde_json::to_value(&s.sign_box).ok(),
    }
}

fn compute(app: &App, kind: Kind) -> Result<Plan, String> {
    match kind {
        Kind::Returns => app.sign_returns_plan(),
        Kind::Mould => app.sign_mould_plan(),
        Kind::SignBox => app.sign_box_plan(),
    }
}

fn commit(app: &mut App, kind: Kind, plan: &Plan) -> bool {
    match kind {
        Kind::Returns => app.sign_strips_commit(plan, false) > 0,
        Kind::Mould => app.sign_strips_commit(plan, true) > 0,
        Kind::SignBox => app.sign_box_commit(plan),
    }
}

/// The current plan of a window, when it is for the drawing as it is.
fn fresh_plan(app: &mut App, kind: Kind) -> Option<Plan> {
    let plan = state(app, kind).plan.clone()?;
    plan.is_current(app).then_some(plan)
}

fn preview_window(app: &mut App, ui: &mut Ui, kind: Kind) -> bool {
    let before = settings_json(app, kind);
    // The box pattern is cheap: it follows the settings without Apply.
    if kind == Kind::SignBox && fresh_plan(app, kind).is_none() {
        let p = app.sign_box_plan();
        let st = state(app, kind);
        match p {
            Ok(p) => {
                st.plan = Some(p);
                st.error = None;
            }
            Err(e) => {
                st.plan = None;
                st.error = Some(e);
            }
        }
    }
    let height = (ui.available_height() - 60.0).max(420.0);
    ui.horizontal_top(|ui| {
        let left_w = (ui.available_width() - kit::SIDE_W - 12.0).max(320.0);
        ui.vertical(|ui| {
            ui.set_width(left_w);
            let (scene, strip) = scene(app, kind);
            kit::preview(ui, Vec2::new(left_w, height - 22.0), &scene);
            kit::status_strip(ui, left_w, &strip);
        });
        ui.vertical(|ui| {
            ui.set_width(kit::SIDE_W);
            ui.set_max_height(height);
            match kind {
                Kind::Returns => returns_settings(app, ui),
                Kind::Mould => mould_settings(app, ui),
                Kind::SignBox => box_settings(app, ui),
            }
        });
    });
    let fresh = fresh_plan(app, kind).is_some();
    kit::progress(ui, if fresh { 1.0 } else { 0.0 });
    let enabled = kind == Kind::SignBox || !app.sign_targets().is_empty();
    let pressed = kit::bottom(ui, Buttons::ApplyOkCancel, enabled, |_| {});
    // A changed setting makes the preview out of date.
    if settings_json(app, kind) != before {
        let st = state(app, kind);
        st.plan = None;
        st.cache = None;
    }
    match pressed {
        Some(Pressed::Apply) => {
            let p = compute(app, kind);
            let st = state(app, kind);
            match p {
                Ok(p) => {
                    st.plan = Some(p);
                    st.error = None;
                }
                Err(e) => {
                    st.plan = None;
                    st.error = Some(e.clone());
                    app.status = e;
                }
            }
            false
        }
        Some(Pressed::Ok) => {
            let plan = match fresh_plan(app, kind) {
                Some(p) => Ok(p),
                None => compute(app, kind),
            };
            state(app, kind).plan = None;
            match plan {
                Ok(p) => {
                    commit(app, kind, &p);
                }
                Err(e) => app.status = e,
            }
            true
        }
        Some(Pressed::Cancel) | Some(Pressed::Close) => {
            state(app, kind).plan = None;
            true
        }
        _ => false,
    }
}

/// The colour a plan's part is previewed in.
fn role_color(r: Role) -> Color32 {
    match r {
        Role::Cut | Role::Hole => colors::SHEET_EDGE,
        Role::Fold | Role::Mark => colors::ACCENT,
        Role::Strip => colors::PART,
    }
}

/// A small dot (a circle outline) for the preview.
fn dot(p: Point, d: f64) -> BezPath {
    tracedraw_signcut::sign::circle(p, d)
}

/// The preview: the letters and, after Apply, the plan; and the status
/// strip's text.
fn scene(app: &mut App, kind: Kind) -> (Scene, String) {
    let units = app.units;
    if let Some(plan) = fresh_plan(app, kind) {
        let mut scene = Scene::default();
        for o in app.sign_outlines(&plan.ids).into_iter().flatten() {
            scene.add(o, colors::ORIGINAL, 1.0);
        }
        for (p, r) in &plan.items {
            scene.add(p.clone(), role_color(*r), 1.0);
        }
        let strip = match kind {
            Kind::SignBox => {
                let (w, h) = plan.size.unwrap_or_default();
                let text = trf(
                    "signcut.wsign_shape_size",
                    &[("w", &fmt_len(units, w)), ("h", &fmt_len(units, h))],
                );
                scene.captions.push(text.clone());
                text
            }
            _ => {
                let text = trf(
                    "signcut.wsign_strips",
                    &[
                        ("n", &plan.count.to_string()),
                        ("len", &fmt_len(units, plan.length)),
                    ],
                );
                scene.captions.push(text.clone());
                text
            }
        };
        return (scene, strip);
    }
    if let Some(e) = state(app, kind).error.clone() {
        if kind == Kind::SignBox {
            return (Scene::default(), e);
        }
    }
    // Before Apply: the letters, cached.
    let k = match kind {
        Kind::Returns => key(app, &app.sc_sign.returns),
        Kind::Mould => key(app, &app.sc_sign.mould),
        Kind::SignBox => key(app, &app.sc_sign.sign_box),
    };
    let ids = k.1.clone();
    let cached = state(app, kind)
        .cache
        .as_ref()
        .filter(|(ck, _)| *ck == k)
        .map(|(_, s)| s.clone());
    let scene = match cached {
        Some(s) => s,
        None => {
            let s = letters_scene(app, kind, &ids);
            state(app, kind).cache = Some((k, s.clone()));
            s
        }
    };
    let strip = if ids.is_empty() {
        tr("signcut.sign_select")
    } else {
        trf("signcut.wsign_ready", &[("n", &ids.len().to_string())])
    };
    (scene, strip)
}

/// The selected letters; for returns each outline's start (red dot), its
/// direction (a short line) and the corners that get a notch or a mark
/// (yellow dots).
fn letters_scene(app: &App, kind: Kind, ids: &[ShapeId]) -> Scene {
    let mut scene = Scene::default();
    let outlines = app.sign_outlines(ids);
    let Some(bounds) = App::sign_bounds(&outlines) else {
        return scene;
    };
    let mark = (bounds.width().max(bounds.height()) * 0.012).max(0.5);
    let r = app.sc_sign.returns;
    for paths in &outlines {
        for p in paths {
            scene.add(p.clone(), colors::PART, 1.0);
            if kind != Kind::Returns {
                continue;
            }
            for path in tracedraw_signcut::sign::closed_subpaths(p) {
                use tracedraw_core::geometry::ParamCurve;
                let Some(seg) = path.segments().next() else {
                    continue;
                };
                let start = seg.start();
                scene.add(dot(start, mark), colors::SHEET_EDGE, 2.0);
                let d = seg.eval(0.01) - start;
                let len = d.hypot();
                if len > 1e-12 {
                    let tip = start + d / len * mark * 4.0;
                    let mut arrow = BezPath::new();
                    arrow.move_to(start);
                    arrow.line_to(tip);
                    scene.add(arrow, colors::SHEET_EDGE, 2.0);
                }
                if r.method != ReturnMethod::Plain {
                    let corners =
                        tracedraw_signcut::sign::returns::corners(&path, r.spec.notch_angle);
                    for at in corners {
                        if let Some(q) = point_at(&path, at) {
                            scene.add(dot(q, mark), Color32::from_rgb(0xE0, 0xB0, 0x00), 2.0);
                        }
                    }
                }
            }
        }
    }
    scene
}

/// The point `s` along a path.
fn point_at(path: &BezPath, s: f64) -> Option<Point> {
    use tracedraw_core::geometry::{ParamCurve, ParamCurveArclen};
    let mut at = 0.0;
    for seg in path.segments() {
        let l = seg.arclen(1e-4);
        if !l.is_finite() {
            continue;
        }
        if s <= at + l + 1e-9 {
            let t = if l > 0.0 {
                seg.inv_arclen((s - at).max(0.0), 1e-4)
            } else {
                0.0
            };
            return Some(seg.eval(t));
        }
        at += l;
    }
    None
}

fn returns_settings(app: &mut App, ui: &mut Ui) {
    kit::preset_row(
        ui,
        &mut app.sc_presets,
        "letter_returns",
        &mut app.sc_sign.returns,
    );
    let u = app.units;
    let r = &mut app.sc_sign.returns;
    kit::row(ui, &tr("signcut.wsign_method"), |ui| {
        let opts = [
            (ReturnMethod::Notched, tr("signcut.wsign_ret_notched")),
            (ReturnMethod::Marked, tr("signcut.wsign_ret_marked")),
            (ReturnMethod::Plain, tr("signcut.wsign_ret_plain")),
        ];
        kit::choice(ui, "sign_ret_method", &mut r.method, &opts)
    });
    egui::ScrollArea::vertical()
        .id_salt("sign_returns_settings")
        .max_height(ui.available_height() - 30.0)
        .show(ui, |ui| {
            kit::group(ui, &tr("signcut.wsign_strip"), |ui| {
                heading(
                    ui,
                    match r.method {
                        ReturnMethod::Notched => "signcut.wsign_ret_notched",
                        ReturnMethod::Marked => "signcut.wsign_ret_marked",
                        ReturnMethod::Plain => "signcut.wsign_ret_plain",
                    },
                );
                kit::row(ui, &tr("signcut.wsign_depth"), |ui| {
                    kit::length(ui, u, &mut r.spec.depth, 0.1, 10_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_thickness"), |ui| {
                    kit::length(ui, u, &mut r.spec.thickness, 0.0, 100.0)
                });
                kit::row(ui, &tr("signcut.wsign_length_on"), |ui| {
                    let opts = [
                        (ReturnFace::Inner, tr("signcut.sign_face_inner")),
                        (ReturnFace::Middle, tr("signcut.sign_face_middle")),
                        (ReturnFace::Outer, tr("signcut.sign_face_outer")),
                    ];
                    kit::choice(ui, "sign_ret_face", &mut r.spec.face, &opts)
                });
                kit::row(ui, &tr("signcut.wsign_overlap"), |ui| {
                    kit::length(ui, u, &mut r.spec.overlap, 0.0, 10_000.0)
                });
                ui.checkbox(&mut r.make_face, tr("signcut.wsign_make_face"));
                if r.method != ReturnMethod::Plain {
                    kit::separator(ui);
                    kit::row(ui, &tr("signcut.wsign_min_angle"), |ui| {
                        kit::number(ui, &mut r.spec.notch_angle, 0.0, 180.0, 1)
                    });
                    if r.method == ReturnMethod::Notched {
                        kit::row(ui, &tr("signcut.wsign_notch_size"), |ui| {
                            kit::length(ui, u, &mut r.spec.notch_size, 0.0, 1000.0)
                        });
                    } else {
                        kit::row(ui, &tr("signcut.wsign_mark_length"), |ui| {
                            kit::length(ui, u, &mut r.mark_length, 0.0, 1000.0)
                        });
                    }
                    kit::row(ui, &tr("signcut.wsign_bend_comp"), |ui| {
                        kit::length(ui, u, &mut r.spec.bend_comp, 0.0, 100.0)
                    });
                }
                kit::separator(ui);
                let mut split = r.spec.max_length > 0.0;
                if ui
                    .checkbox(&mut split, tr("signcut.wsign_use_max_length"))
                    .changed()
                {
                    r.spec.max_length = if split { 2000.0 } else { 0.0 };
                }
                ui.add_enabled_ui(split, |ui| {
                    kit::row(ui, &tr("signcut.wsign_max_length"), |ui| {
                        let mut v = r.spec.max_length.max(1.0);
                        let resp = kit::length(ui, u, &mut v, 1.0, 100_000.0);
                        if split {
                            r.spec.max_length = v;
                        }
                        resp
                    });
                });
            });
            kit::group(ui, &tr("signcut.nw_options"), |ui| {
                kit::row(ui, &tr("signcut.wsign_row_gap"), |ui| {
                    kit::length(ui, u, &mut r.strip_gap, 0.0, 10_000.0)
                });
                ui.checkbox(&mut r.labels, tr("signcut.wsign_number_strips"));
                kit::units_row(ui, &mut app.units);
            });
        });
}

fn mould_settings(app: &mut App, ui: &mut Ui) {
    kit::preset_row(
        ui,
        &mut app.sc_presets,
        "acrylic_mould",
        &mut app.sc_sign.mould,
    );
    let u = app.units;
    let figures = fresh_plan(app, Kind::Mould).and_then(|p| p.mould);
    let m = &mut app.sc_sign.mould;
    egui::ScrollArea::vertical()
        .id_salt("sign_mould_settings")
        .max_height(ui.available_height() - 30.0)
        .show(ui, |ui| {
            kit::group(ui, &tr("signcut.wsign_contours"), |ui| {
                kit::row(ui, &tr("signcut.wsign_outside"), |ui| {
                    kit::length(ui, u, &mut m.spec.wall, 0.0, 1000.0)
                });
                let mut inside = m.spec.inside > 0.0;
                if ui
                    .checkbox(&mut inside, tr("signcut.wsign_inside_contour"))
                    .changed()
                {
                    m.spec.inside = if inside { 2.0 } else { 0.0 };
                }
                ui.add_enabled_ui(inside, |ui| {
                    kit::row(ui, &tr("signcut.wsign_inside"), |ui| {
                        let mut v = m.spec.inside.max(0.01);
                        let resp = kit::length(ui, u, &mut v, 0.01, 1000.0);
                        if inside {
                            m.spec.inside = v;
                        }
                        resp
                    });
                });
            });
            kit::group(ui, &tr("signcut.wsign_wall_strip"), |ui| {
                kit::row(ui, &tr("signcut.wsign_fill_depth"), |ui| {
                    kit::length(ui, u, &mut m.spec.depth, 0.1, 1000.0)
                });
                kit::row(ui, &tr("signcut.wsign_overlap"), |ui| {
                    kit::length(ui, u, &mut m.spec.overlap, 0.0, 10_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_row_gap"), |ui| {
                    kit::length(ui, u, &mut m.strip_gap, 0.0, 10_000.0)
                });
                ui.checkbox(&mut m.labels, tr("signcut.wsign_number_strips"));
            });
            kit::group(ui, &tr("signcut.nw_material"), |ui| {
                kit::row(ui, &tr("signcut.wsign_density"), |ui| {
                    kit::number(ui, &mut m.density, 0.0, 30.0, 3)
                });
                kit::units_row(ui, &mut app.units);
            });
            kit::group(ui, &tr("signcut.wsign_output"), |ui| match figures {
                Some(f) => {
                    let line = |ui: &mut Ui, key: &str, v: String| {
                        kit::row(ui, &tr(key), |ui| ui.label(RichText::new(v).strong()));
                    };
                    line(ui, "signcut.wsign_perimeter", fmt_len(u, f.perimeter));
                    line(ui, "signcut.wsign_area", fmt_area(u, f.area));
                    line(ui, "signcut.wsign_volume", format!("{:.1} ml", f.volume));
                    line(ui, "signcut.wsign_mass", format!("{:.1} g", f.mass));
                }
                None => {
                    ui.label(tr("signcut.wsign_output_hint"));
                }
            });
        });
}

fn box_settings(app: &mut App, ui: &mut Ui) {
    kit::preset_row(
        ui,
        &mut app.sc_presets,
        "sign_box",
        &mut app.sc_sign.sign_box,
    );
    let u = app.units;
    let sel = App::sign_bounds(&app.sign_outlines(&app.sign_targets()));
    let tab = &mut app.sc_sign.win.box_tab;
    let b = &mut app.sc_sign.sign_box;
    kit::row(ui, &tr("signcut.wsign_method"), |ui| {
        let opts = [
            (BoxMethod::Classic, tr("signcut.wsign_box_classic")),
            (BoxMethod::CornerTabs, tr("signcut.wsign_box_tabs")),
        ];
        kit::choice(ui, "sign_box_method", &mut b.spec.method, &opts)
    });
    tabs(
        ui,
        tab,
        &[
            (BoxTab::Size, tr("signcut.wsign_tab_size")),
            (BoxTab::Options, tr("signcut.wsign_tab_options")),
        ],
    );
    let tab = *tab;
    egui::ScrollArea::vertical()
        .id_salt("sign_box_settings")
        .max_height(ui.available_height() - 30.0)
        .show(ui, |ui| match tab {
            BoxTab::Size => {
                kit::group(ui, &tr("signcut.wsign_size"), |ui| {
                    ui.checkbox(&mut b.use_selection, tr("signcut.wsign_use_selection"));
                    let fixed = b.use_selection && sel.is_some();
                    ui.add_enabled_ui(!fixed, |ui| {
                        kit::row(ui, &tr("signcut.width"), |ui| {
                            kit::length(ui, u, &mut b.width, 1.0, 100_000.0)
                        });
                        kit::row(ui, &tr("signcut.height"), |ui| {
                            kit::length(ui, u, &mut b.height, 1.0, 100_000.0)
                        });
                    });
                    if ui
                        .add_enabled(
                            sel.is_some(),
                            egui::Button::new(tr("signcut.wsign_get_size")),
                        )
                        .clicked()
                    {
                        if let Some(r) = sel {
                            b.width = r.width().max(1.0);
                            b.height = r.height().max(1.0);
                        }
                    }
                });
                sides_group(
                    ui,
                    &tr("signcut.wsign_first_fold"),
                    u,
                    &mut b.spec.depth,
                    "depth",
                );
                sides_group(
                    ui,
                    &tr("signcut.wsign_second_fold"),
                    u,
                    &mut b.spec.lip,
                    "lip",
                );
            }
            BoxTab::Options => {
                kit::group(ui, &tr("signcut.wsign_corners"), |ui| {
                    kit::row(ui, &tr("signcut.wsign_corner_gap"), |ui| {
                        kit::length(ui, u, &mut b.spec.corner_gap, 0.0, 1000.0)
                    });
                    kit::row(ui, &tr("signcut.wsign_relief"), |ui| {
                        kit::length(ui, u, &mut b.spec.relief, 0.0, 1000.0)
                    });
                    if b.spec.method == BoxMethod::CornerTabs {
                        kit::row(ui, &tr("signcut.wsign_tab_width"), |ui| {
                            kit::length(ui, u, &mut b.spec.tab_width, 0.0, 1000.0)
                        });
                    }
                });
                kit::group(ui, &tr("signcut.wsign_holes"), |ui| {
                    ui.checkbox(&mut b.spec.holes, tr("signcut.wsign_fixing_holes"));
                    ui.add_enabled_ui(b.spec.holes, |ui| {
                        kit::row(ui, &tr("signcut.wsign_hole_spacing"), |ui| {
                            kit::length(ui, u, &mut b.spec.hole_spacing, 1.0, 100_000.0)
                        });
                        kit::row(ui, &tr("signcut.wsign_hole_diameter"), |ui| {
                            kit::length(ui, u, &mut b.spec.hole_diameter, 0.0, 1000.0)
                        });
                    });
                });
                kit::group(ui, &tr("signcut.wsign_legend"), |ui| {
                    ui.checkbox(&mut b.legend, tr("signcut.wsign_add_legend"));
                    ui.add_enabled_ui(b.legend, |ui| {
                        kit::row(ui, &tr("signcut.wsign_prefix"), |ui| {
                            ui.add_sized(
                                [kit::FIELD_W, 22.0],
                                egui::TextEdit::singleline(&mut b.prefix),
                            )
                        });
                    });
                });
                kit::units_row(ui, &mut app.units);
            }
        });
}

// ---------------------------------------------------------------------
// LED Modules.

fn mode_label(m: LedMode) -> String {
    tr(match m {
        LedMode::Matrix => "signcut.wsign_led_matrix",
        LedMode::Shifted => "signcut.wsign_led_shifted",
        LedMode::Centre => "signcut.wsign_led_centre",
        LedMode::Outline => "signcut.wsign_led_offset",
        LedMode::Island => "signcut.wsign_led_island",
        LedMode::Random => "signcut.wsign_led_random",
        LedMode::OnCurve => "signcut.wsign_led_on_curve",
        LedMode::AroundCurve => "signcut.wsign_led_around",
    })
}

fn led_window(app: &mut App, ui: &mut Ui) -> bool {
    let height = (ui.available_height() - 40.0).max(380.0);
    // The tab follows the mode when the window opens.
    if app.sc_sign.win.led.tab != LedTab::Tools {
        app.sc_sign.win.led.tab = if app.sc_sign.led.spec.mode.on_curve() {
            LedTab::Curve
        } else {
            LedTab::Fill
        };
    }
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(280.0);
            ui.set_min_height(height);
            led_left(app, ui);
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.set_width((ui.available_width()).max(300.0));
            ui.set_min_height(height);
            led_right(app, ui);
        });
    });
    // Show the layout on the drawing.
    if app.sc_sign.win.led.show {
        let k = key(app, &app.sc_sign.led);
        let cached = app
            .sc_sign
            .win
            .led
            .cache
            .as_ref()
            .filter(|(ck, _)| *ck == k)
            .map(|(_, p)| p.clone());
        let paths = match cached {
            Some(p) => p,
            None => {
                let paths: Vec<BezPath> = match app.sign_led_placements() {
                    Some((ps, m)) => ps
                        .iter()
                        .map(|p| {
                            tracedraw_signcut::sign::led::module_path(
                                &m,
                                app.sc_sign.led.spec.rotate,
                                *p,
                            )
                        })
                        .collect(),
                    None => Vec::new(),
                };
                app.sc_sign.win.led.cache = Some((k, paths.clone()));
                paths
            }
        };
        app.docker_preview.extend(paths);
    }
    matches!(
        kit::bottom(ui, Buttons::Close, true, |_| {}),
        Some(Pressed::Close)
    )
}

fn led_left(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let w = &mut app.sc_sign.win.led;
    tabs(
        ui,
        &mut w.edit,
        &[
            (false, tr("signcut.wsign_catalogue")),
            (true, tr("signcut.wsign_edit_module")),
        ],
    );
    let edit = w.edit;
    let s = &mut app.sc_sign.led;
    if s.catalogue.is_empty() {
        s.catalogue = tracedraw_signcut::sign::led::catalogue();
    }
    s.module = s.module.min(s.catalogue.len().saturating_sub(1));
    if edit {
        if let Some(m) = s.catalogue.get_mut(s.module) {
            module_editor(ui, u, m);
        }
    } else {
        egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, crate::theme::Tokens::border()))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [190.0, 18.0],
                        egui::Label::new(RichText::new(tr("signcut.wsign_name")).strong()),
                    );
                    ui.label(RichText::new(tr("signcut.wsign_power")).strong());
                });
                egui::ScrollArea::vertical()
                    .id_salt("sign_led_catalogue")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for (i, m) in s.catalogue.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                                ui.painter().rect_filled(
                                    r,
                                    0.0,
                                    Color32::from_rgb(m.color[0], m.color[1], m.color[2]),
                                );
                                if ui
                                    .add_sized(
                                        [176.0, 18.0],
                                        egui::Button::selectable(s.module == i, m.name.as_str())
                                            .truncate(),
                                    )
                                    .clicked()
                                {
                                    s.module = i;
                                }
                                ui.label(format!("{:.2}", m.power));
                            });
                        }
                    });
            });
        if let Some(m) = s.catalogue.get(s.module) {
            let size = if m.round {
                trf("signcut.wsign_diode_size", &[("d", &fmt_len(u, m.width))])
            } else {
                trf(
                    "signcut.wsign_module_size",
                    &[("w", &fmt_len(u, m.width)), ("h", &fmt_len(u, m.height))],
                )
            };
            ui.label(size);
            ui.label(trf(
                "signcut.wsign_module_info",
                &[
                    ("n", &m.diodes.to_string()),
                    ("w", &format!("{:.2}", m.power)),
                    ("price", &format!("{:.2}", m.price)),
                ],
            ));
        }
    }
    kit::separator(ui);
    kit::row(ui, &tr("signcut.wsign_rotate"), |ui| {
        let opts = [
            (false, tr("signcut.wsign_along")),
            (true, tr("signcut.wsign_across")),
        ];
        kit::choice(ui, "sign_led_rotate", &mut s.spec.rotate, &opts)
    });
    kit::row(ui, &tr("signcut.wsign_fix_overlaps"), |ui| {
        let opts = [(false, tr("signcut.nw_no")), (true, tr("signcut.nw_yes"))];
        kit::choice(ui, "sign_led_fix", &mut s.spec.fix_overlaps, &opts)
    });
    ui.add_enabled_ui(s.spec.fix_overlaps, |ui| {
        kit::row(ui, &tr("signcut.wsign_overlap_gap"), |ui| {
            kit::length(ui, u, &mut s.spec.overlap_gap, 0.0, 1000.0)
        });
    });
    kit::units_row(ui, &mut app.units);
    let s = &mut app.sc_sign.led;
    ui.horizontal(|ui| {
        if ui.button(tr("signcut.sign_module_add")).clicked() {
            let copy = s.catalogue.get(s.module).cloned().map(|mut m| {
                m.name = format!("{} (2)", m.name);
                m
            });
            if let Some(m) = copy {
                s.catalogue.push(m);
                s.module = s.catalogue.len() - 1;
            }
        }
        if ui
            .add_enabled(
                s.catalogue.len() > 1,
                egui::Button::new(tr("signcut.sign_module_delete")),
            )
            .clicked()
            && s.module < s.catalogue.len()
        {
            s.catalogue.remove(s.module);
            s.module = s.module.min(s.catalogue.len().saturating_sub(1));
        }
    });
}

fn module_editor(ui: &mut Ui, u: Units, m: &mut Module) {
    kit::row(ui, &tr("signcut.wsign_name"), |ui| {
        ui.add_sized(
            [kit::FIELD_W, 22.0],
            egui::TextEdit::singleline(&mut m.name),
        )
    });
    ui.checkbox(&mut m.round, tr("signcut.wsign_round"));
    kit::row(
        ui,
        &tr(if m.round {
            "signcut.wsign_diameter"
        } else {
            "signcut.width"
        }),
        |ui| kit::length(ui, u, &mut m.width, 0.1, 10_000.0),
    );
    if !m.round {
        kit::row(ui, &tr("signcut.height"), |ui| {
            kit::length(ui, u, &mut m.height, 0.1, 10_000.0)
        });
    }
    kit::row(ui, &tr("signcut.wsign_diodes"), |ui| {
        let mut d = m.diodes as usize;
        let r = kit::count(ui, &mut d, 0, 1000);
        if r.changed() {
            m.diodes = d.min(1000) as u32;
        }
        r
    });
    kit::row(ui, &tr("signcut.wsign_power_w"), |ui| {
        kit::number(ui, &mut m.power, 0.0, 10_000.0, 3)
    });
    kit::row(ui, &tr("signcut.wsign_price"), |ui| {
        kit::number(ui, &mut m.price, 0.0, 1_000_000.0, 2)
    });
    kit::row(ui, &tr("signcut.wsign_color"), |ui| {
        ui.color_edit_button_srgb(&mut m.color)
    });
}

fn led_right(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let before = app.sc_sign.win.led.tab;
    tabs(
        ui,
        &mut app.sc_sign.win.led.tab,
        &[
            (LedTab::Curve, tr("signcut.wsign_tab_curve")),
            (LedTab::Fill, tr("signcut.wsign_tab_fill")),
            (LedTab::Tools, tr("signcut.wsign_tab_tools")),
        ],
    );
    let tab = app.sc_sign.win.led.tab;
    let spec = &mut app.sc_sign.led.spec;
    if tab != before {
        // A tab of placing methods starts at its first method.
        match tab {
            LedTab::Curve if !spec.mode.on_curve() => spec.mode = LedMode::OnCurve,
            LedTab::Fill if spec.mode.on_curve() => spec.mode = LedMode::Matrix,
            _ => {}
        }
    }
    match tab {
        LedTab::Curve | LedTab::Fill => {
            let modes: &[LedMode] = if tab == LedTab::Curve {
                &LedMode::CURVE
            } else {
                &LedMode::FILL
            };
            kit::row(ui, &tr("signcut.wsign_method"), |ui| {
                let opts: Vec<(LedMode, String)> =
                    modes.iter().map(|m| (*m, mode_label(*m))).collect();
                kit::choice(ui, "sign_led_mode", &mut spec.mode, &opts)
            });
            kit::group(ui, &tr("signcut.nw_tab_main"), |ui| {
                kit::row(ui, &tr("signcut.wsign_led_pitch"), |ui| {
                    kit::length(ui, u, &mut spec.module_pitch, 0.1, 100_000.0)
                });
                if matches!(
                    spec.mode,
                    LedMode::Matrix | LedMode::Shifted | LedMode::Island
                ) {
                    kit::row(ui, &tr("signcut.wsign_row_pitch"), |ui| {
                        kit::length(ui, u, &mut spec.row_pitch, 0.1, 100_000.0)
                    });
                }
                match spec.mode {
                    LedMode::OnCurve => {}
                    LedMode::AroundCurve => {
                        kit::row(ui, &tr("signcut.wsign_side_distance"), |ui| {
                            kit::length(ui, u, &mut spec.side_distance, 0.0, 10_000.0)
                        });
                    }
                    LedMode::Outline => {
                        kit::row(ui, &tr("signcut.wsign_outline_offset"), |ui| {
                            kit::length(ui, u, &mut spec.outline_offset, 0.0, 10_000.0)
                        });
                        kit::row(ui, &tr("signcut.wsign_edge_gap"), |ui| {
                            kit::length(ui, u, &mut spec.edge, 0.0, 10_000.0)
                        });
                    }
                    _ => {
                        kit::row(ui, &tr("signcut.wsign_edge_gap"), |ui| {
                            kit::length(ui, u, &mut spec.edge, 0.0, 10_000.0)
                        });
                    }
                }
                if spec.mode == LedMode::Random {
                    kit::row(ui, &tr("signcut.nw_seed"), |ui| {
                        let mut s = spec.seed.min(1_000_000_000) as usize;
                        let r = kit::count(ui, &mut s, 0, 1_000_000_000);
                        if r.changed() {
                            spec.seed = s as u64;
                        }
                        r
                    });
                }
            });
            ui.label(
                RichText::new(tr(match spec.mode {
                    LedMode::OnCurve => "signcut.wsign_led_on_curve_hint",
                    LedMode::AroundCurve => "signcut.wsign_led_around_hint",
                    LedMode::Outline => "signcut.wsign_led_offset_hint",
                    LedMode::Island => "signcut.wsign_led_island_hint",
                    LedMode::Random => "signcut.wsign_led_random_hint",
                    LedMode::Centre => "signcut.wsign_led_centre_hint",
                    _ => "signcut.wsign_led_matrix_hint",
                }))
                .color(crate::theme::Tokens::text_dim()),
            );
            if let Some(r) = app.sc_sign.led_report {
                ui.label(led_summary(&r));
            }
            ui.add_space(6.0);
            let has = !app.sign_targets().is_empty();
            ui.horizontal(|ui| {
                if kit::push(ui, &tr("signcut.wsign_create_leds"), has) {
                    app.sign_led_layout();
                }
                let show = app.sc_sign.win.led.show;
                if ui
                    .add(egui::Button::new(tr("signcut.wsign_show")).selected(show))
                    .on_hover_text(tr("signcut.wsign_show_hint"))
                    .clicked()
                {
                    app.sc_sign.win.led.show = !show;
                }
                let last = app.engine.undo_label() == Some("LED Modules");
                if kit::push(ui, &tr("signcut.wsign_undo_last"), last) {
                    if let Err(e) = app.engine.undo() {
                        app.status = e.to_string();
                    }
                }
            });
        }
        LedTab::Tools => {
            heading(ui, "signcut.nw_search");
            if command(ui, "signcut.wsign_led_select_all", true) {
                app.sign_led_select_all();
            }
            kit::separator(ui);
            heading(ui, "signcut.wsign_edit");
            let gap = &mut app.sc_sign.win.led.fix_gap;
            kit::row(ui, &tr("signcut.wsign_overlap_gap"), |ui| {
                kit::length(ui, u, gap, 0.0, 1000.0)
            });
            let gap = *gap;
            if command(ui, "signcut.wsign_led_fix", !app.sign_targets().is_empty()) {
                app.sign_led_fix_overlaps(gap);
            }
        }
    }
}

// ---------------------------------------------------------------------
// Neon Tubes.

fn neon_window(app: &mut App, ui: &mut Ui) -> bool {
    let u = app.units;
    let height = (ui.available_height() - 40.0).max(380.0);
    tabs(
        ui,
        &mut app.sc_sign.win.neon_tab,
        &[
            (NeonTab::Neon, tr("signcut.wsign_tab_neon")),
            (NeonTab::Tools, tr("signcut.wsign_tab_tools")),
            (NeonTab::Options, tr("signcut.wsign_tab_options")),
        ],
    );
    let tab = app.sc_sign.win.neon_tab;
    let has = !app.sign_targets().is_empty();
    ui.vertical(|ui| {
        ui.set_min_height(height);
        match tab {
            NeonTab::Neon => {
                let n = &mut app.sc_sign.neon;
                kit::row(ui, &tr("signcut.wsign_neon_type"), |ui| {
                    let opts = [
                        (NeonKind::CentreLine, tr("signcut.wsign_neon_centre")),
                        (NeonKind::Contour, tr("signcut.wsign_neon_contour")),
                    ];
                    kit::choice(ui, "sign_neon_kind", &mut n.kind, &opts)
                });
                ui.label(
                    RichText::new(tr(match n.kind {
                        NeonKind::CentreLine => "signcut.sign_neon_hint",
                        NeonKind::Contour => "signcut.wsign_neon_contour_hint",
                    }))
                    .color(crate::theme::Tokens::text_dim()),
                );
                kit::separator(ui);
                kit::row(ui, &tr("signcut.wsign_tube_diameter"), |ui| {
                    let mut pick: Option<usize> = if n.custom_diameter {
                        None
                    } else {
                        NEON_DIAMETERS
                            .iter()
                            .position(|d| (d - n.spec.diameter).abs() < 1e-9)
                    };
                    let mut opts: Vec<(Option<usize>, String)> = NEON_DIAMETERS
                        .iter()
                        .enumerate()
                        .map(|(i, d)| (Some(i), fmt_len(u, *d)))
                        .collect();
                    opts.push((None, tr("signcut.wsign_custom")));
                    let changed = kit::choice(ui, "sign_neon_diameter", &mut pick, &opts);
                    if changed {
                        match pick.and_then(|i| NEON_DIAMETERS.get(i)) {
                            Some(d) => {
                                n.spec.diameter = *d;
                                n.custom_diameter = false;
                            }
                            None => n.custom_diameter = true,
                        }
                    }
                    changed
                });
                let custom = n.custom_diameter
                    || !NEON_DIAMETERS
                        .iter()
                        .any(|d| (d - n.spec.diameter).abs() < 1e-9);
                ui.add_enabled_ui(custom, |ui| {
                    kit::row(ui, &tr("signcut.wsign_custom_diameter"), |ui| {
                        kit::length(ui, u, &mut n.spec.diameter, 0.1, 1000.0)
                    });
                });
                if n.kind == NeonKind::Contour {
                    kit::row(ui, &tr("signcut.wsign_edge_gap"), |ui| {
                        kit::length(ui, u, &mut n.contour.gap, 0.0, 10_000.0)
                    });
                    kit::row(ui, &tr("signcut.wsign_tube_count"), |ui| {
                        kit::count(
                            ui,
                            &mut n.contour.count,
                            1,
                            tracedraw_signcut::sign::neon::MAX_CONTOURS,
                        )
                    });
                    kit::row(ui, &tr("signcut.wsign_tube_gap"), |ui| {
                        kit::length(ui, u, &mut n.contour.spacing, 0.0, 10_000.0)
                    });
                    ui.checkbox(&mut n.contour.round, tr("signcut.wsign_round_corners"));
                }
                ui.checkbox(&mut n.outline, tr("signcut.sign_neon_outline"));
                ui.add_space(8.0);
                if command(ui, "signcut.apply", has) {
                    app.sign_neon();
                }
            }
            NeonTab::Tools => {
                kit::group(ui, &tr("signcut.wsign_processing"), |ui| {
                    if command(ui, "signcut.wsign_neon_divide", has) {
                        app.sign_neon_parts(true, false);
                    }
                    if command(ui, "signcut.wsign_neon_supports", has) {
                        app.sign_neon_parts(false, true);
                    }
                    kit::separator(ui);
                    if command(ui, "signcut.wsign_neon_to_outlines", has) {
                        app.sign_neon_to_outlines();
                    }
                    kit::separator(ui);
                    if command(ui, "signcut.wsign_neon_measure", has) {
                        app.sign_neon_measure();
                    }
                });
            }
            NeonTab::Options => {
                let s = &mut app.sc_sign.neon.spec;
                kit::row(ui, &tr("signcut.wsign_support_distance"), |ui| {
                    kit::length(ui, u, &mut s.support_spacing, 0.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_max_tube"), |ui| {
                    kit::length(ui, u, &mut s.max_length, 10.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_min_tube"), |ui| {
                    kit::length(ui, u, &mut s.min_length, 0.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_hole_diameter"), |ui| {
                    kit::length(ui, u, &mut s.hole_diameter, 0.0, 1000.0)
                });
                kit::units_row(ui, &mut app.units);
            }
        }
        if let Some(r) = app.sc_sign.neon_report {
            ui.add_space(6.0);
            ui.label(trf(
                "signcut.sign_neon_done",
                &[
                    ("n", &r.tubes.to_string()),
                    ("len", &fmt_len(app.units, r.length)),
                    ("holes", &r.supports.to_string()),
                ],
            ));
        }
    });
    matches!(
        kit::bottom(ui, Buttons::Close, true, |_| {}),
        Some(Pressed::Close)
    )
}

// ---------------------------------------------------------------------
// Smart Divide and Carrier.

fn tiles_window(app: &mut App, ui: &mut Ui) -> bool {
    let u = app.units;
    let t = &mut app.sc_sign.tiles;
    kit::row(ui, &tr("signcut.wsign_method"), |ui| {
        let opts = [
            (TileMethod::Size, tr("signcut.wsign_tiles_size")),
            (TileMethod::Table, tr("signcut.wsign_tiles_table")),
        ];
        kit::choice(ui, "sign_tiles_method", &mut t.method, &opts)
    });
    kit::group(ui, &tr("signcut.nw_options"), |ui| {
        kit::row(ui, &tr("signcut.wsign_overlap"), |ui| {
            kit::length(ui, u, &mut t.spec.overlap, 0.0, 10_000.0)
        });
        match t.method {
            TileMethod::Size => {
                kit::row(ui, &tr("signcut.wsign_tile_width"), |ui| {
                    kit::length(ui, u, &mut t.spec.width, 1.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_tile_height"), |ui| {
                    kit::length(ui, u, &mut t.spec.height, 1.0, 100_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_grid_from"), |ui| {
                    let opts = [
                        (TileOrigin::Selection, tr("signcut.sign_origin_selection")),
                        (TileOrigin::Page, tr("signcut.sign_origin_page")),
                    ];
                    kit::choice(ui, "sign_tiles_origin", &mut t.origin, &opts)
                });
            }
            TileMethod::Table => {
                kit::row(ui, &tr("signcut.wsign_columns"), |ui| {
                    kit::count(ui, &mut t.cols, 1, 100)
                });
                kit::row(ui, &tr("signcut.wsign_rows"), |ui| {
                    kit::count(ui, &mut t.rows, 1, 100)
                });
            }
        }
        kit::separator(ui);
        kit::row(ui, &tr("signcut.wsign_spread"), |ui| {
            kit::length(ui, u, &mut t.spec.gap, 0.0, 10_000.0)
        });
        ui.checkbox(&mut t.labels, tr("signcut.sign_tile_labels"));
        ui.checkbox(&mut t.frames, tr("signcut.sign_tile_frames"));
        kit::separator(ui);
        kit::units_row(ui, &mut app.units);
    });
    // The grid, cached, marked on the drawing.
    let k = key(app, &app.sc_sign.tiles);
    let fresh = app
        .sc_sign
        .win
        .tiles_cache
        .as_ref()
        .is_some_and(|(ck, _, _)| *ck == k);
    if !fresh {
        let (paths, text) = match app.sign_tile_grid() {
            Some(g) => {
                let mut paths = Vec::new();
                for c in 0..g.cols {
                    for r in 0..g.rows {
                        paths.push(g.rect(c, r).to_path(0.01));
                    }
                }
                let text = trf(
                    "signcut.wsign_tiles_info",
                    &[
                        ("cols", &g.cols.to_string()),
                        ("rows", &g.rows.to_string()),
                        ("w", &fmt_len(app.units, g.size.x)),
                        ("h", &fmt_len(app.units, g.size.y)),
                    ],
                );
                (paths, text)
            }
            None if k.1.is_empty() => (Vec::new(), tr("signcut.sign_select")),
            None => (Vec::new(), tr("signcut.sign_tiles_bad")),
        };
        app.sc_sign.win.tiles_cache = Some((k, paths, text));
    }
    let mut ok_grid = false;
    if let Some((_, paths, text)) = &app.sc_sign.win.tiles_cache {
        ui.label(text.clone());
        ok_grid = !paths.is_empty();
        app.docker_preview.extend(paths.iter().cloned());
    }
    if let Some((c, r, n)) = app.sc_sign.tiles_report {
        ui.label(trf(
            "signcut.sign_tiles_last",
            &[
                ("cols", &c.to_string()),
                ("rows", &r.to_string()),
                ("n", &n.to_string()),
            ],
        ));
    }
    match kit::bottom(ui, Buttons::ApplyOkCancel, ok_grid, |_| {}) {
        Some(Pressed::Apply) => {
            app.sign_divide();
            false
        }
        Some(Pressed::Ok) => {
            app.sign_divide();
            true
        }
        Some(_) => true,
        None => false,
    }
}

fn carrier_window(app: &mut App, ui: &mut Ui) -> bool {
    let u = app.units;
    let c = &mut app.sc_sign.carrier;
    kit::row(ui, &tr("signcut.wsign_method"), |ui| {
        let opts = [
            (CarrierMethod::Contour, tr("signcut.wsign_carrier_contour")),
            (CarrierMethod::Rectangle, tr("signcut.wsign_carrier_rect")),
        ];
        kit::choice(ui, "sign_carrier_method", &mut c.method, &opts)
    });
    match c.method {
        CarrierMethod::Contour => {
            kit::group(ui, &tr("signcut.wsign_carrier_contour"), |ui| {
                kit::row(ui, &tr("signcut.wsign_distance"), |ui| {
                    kit::length(ui, u, &mut c.distance, 0.0, 10_000.0)
                });
                kit::row(ui, &tr("signcut.wsign_rounding"), |ui| {
                    kit::length(ui, u, &mut c.rounding, 0.0, 10_000.0)
                });
                ui.checkbox(&mut c.fill_holes, tr("signcut.sign_fill_holes"));
            });
        }
        CarrierMethod::Rectangle => {
            sides_group(
                ui,
                &tr("signcut.wsign_margins"),
                u,
                &mut c.margins,
                "carrier",
            );
            kit::group(ui, &tr("signcut.wsign_carrier_rect"), |ui| {
                kit::row(ui, &tr("signcut.wsign_corner_radius"), |ui| {
                    kit::length(ui, u, &mut c.corner_radius, 0.0, 10_000.0)
                });
            });
        }
    }
    kit::group(ui, &tr("signcut.nw_options"), |ui| {
        ui.checkbox(
            &mut app.sc_sign.win.carrier_show,
            tr("signcut.wsign_show_on_drawing"),
        );
        kit::units_row(ui, &mut app.units);
    });
    if app.sc_sign.win.carrier_show {
        let k = key(app, &app.sc_sign.carrier);
        let fresh = app
            .sc_sign
            .win
            .carrier_cache
            .as_ref()
            .is_some_and(|(ck, _)| *ck == k);
        if !fresh {
            let outline = app.sign_carrier_outline();
            app.sc_sign.win.carrier_cache = Some((k, outline));
        }
        if let Some((_, p)) = &app.sc_sign.win.carrier_cache {
            if !p.elements().is_empty() {
                app.docker_preview.push(p.clone());
            }
        }
    }
    if let Some(n) = app.sc_sign.carrier_report {
        ui.label(trf("signcut.sign_carrier_done", &[("n", &n.to_string())]));
    }
    let has = !app.sign_targets().is_empty();
    match kit::bottom(ui, Buttons::ApplyOkCancel, has, |_| {}) {
        Some(Pressed::Apply) => {
            app.sign_carrier();
            false
        }
        Some(Pressed::Ok) => {
            app.sign_carrier();
            true
        }
        Some(_) => true,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn app_with_letters() -> App {
        let mut app = App::headless();
        let mut ids = Vec::new();
        for i in 0..2 {
            let x = i as f64 * 120.0;
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: Rect::new(x, 50.0, x + 100.0, 150.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("layer"),
            );
        }
        app.select(ids);
        app
    }

    fn draw(app: &mut App, t: ToolId) -> bool {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1300.0, 950.0),
            )),
            ..Default::default()
        };
        let mut closed = false;
        let mut out = ctx.run_ui(input, |ui| {
            closed = body(app, ui, t);
        });
        out.textures_delta.clear();
        closed
    }

    #[test]
    fn every_window_draws_every_method_and_tab() {
        let mut app = app_with_letters();
        for t in TOOLS {
            assert!(!draw(&mut app, *t));
        }
        // LED: every tab and method, the editor, shown on the drawing.
        app.sc_sign.win.led.show = true;
        for edit in [false, true] {
            app.sc_sign.win.led.edit = edit;
            for m in LedMode::CURVE.iter().chain(LedMode::FILL.iter()) {
                app.sc_sign.led.spec.mode = *m;
                app.sc_sign.win.led.tab = LedTab::Fill;
                draw(&mut app, ToolId::LedModules);
            }
        }
        app.sc_sign.win.led.tab = LedTab::Tools;
        draw(&mut app, ToolId::LedModules);
        // Neon: each tab and kind.
        for tab in [NeonTab::Neon, NeonTab::Tools, NeonTab::Options] {
            app.sc_sign.win.neon_tab = tab;
            for kind in [NeonKind::CentreLine, NeonKind::Contour] {
                app.sc_sign.neon.kind = kind;
                draw(&mut app, ToolId::NeonTubes);
            }
        }
        // Returns: each method, before and after Apply.
        for m in [
            ReturnMethod::Notched,
            ReturnMethod::Marked,
            ReturnMethod::Plain,
        ] {
            app.sc_sign.returns.method = m;
            app.sc_sign.win.returns.plan = None;
            draw(&mut app, ToolId::LetterReturns);
            app.sc_sign.win.returns.plan = app.sign_returns_plan().ok();
            draw(&mut app, ToolId::LetterReturns);
        }
        // Mould before and after Apply.
        app.sc_sign.win.mould.plan = app.sign_mould_plan().ok();
        draw(&mut app, ToolId::AcrylicMould);
        // Box: both methods and tabs.
        for m in [BoxMethod::Classic, BoxMethod::CornerTabs] {
            app.sc_sign.sign_box.spec.method = m;
            for tab in [BoxTab::Size, BoxTab::Options] {
                app.sc_sign.win.box_tab = tab;
                draw(&mut app, ToolId::SignBox);
            }
        }
        // Smart Divide and Carrier: both methods.
        for m in [TileMethod::Size, TileMethod::Table] {
            app.sc_sign.tiles.method = m;
            draw(&mut app, ToolId::SmartDivide);
        }
        app.sc_sign.win.carrier_show = true;
        for m in [CarrierMethod::Contour, CarrierMethod::Rectangle] {
            app.sc_sign.carrier.method = m;
            draw(&mut app, ToolId::Carrier);
        }
        // Nothing selected.
        app.select(vec![]);
        for t in TOOLS {
            draw(&mut app, *t);
        }
    }

    #[test]
    fn apply_previews_and_changes_nothing() {
        let mut app = app_with_letters();
        let rev = app.engine.revision();
        for kind in [Kind::Returns, Kind::Mould, Kind::SignBox] {
            let plan = compute(&app, kind).expect("a plan");
            state(&mut app, kind).plan = Some(plan);
            let (scene, strip) = scene(&mut app, kind);
            assert!(scene.items.len() > 2, "{kind:?}");
            assert!(!scene.captions.is_empty() && !strip.is_empty());
        }
        assert_eq!(app.engine.revision(), rev);
    }

    #[test]
    fn ok_applies_the_previewed_plan_in_one_step() {
        for (kind, label) in [
            (Kind::Returns, "Letter Returns"),
            (Kind::Mould, "Acrylic Mould"),
            (Kind::SignBox, "Sign Box"),
        ] {
            let mut app = app_with_letters();
            let count = |app: &App| {
                app.page_layers()
                    .iter()
                    .map(|l| l.shapes.len())
                    .sum::<usize>()
            };
            let before = count(&app);
            let plan = compute(&app, kind).expect("a plan");
            assert!(commit(&mut app, kind, &plan));
            assert_eq!(app.engine.undo_label(), Some(label));
            assert_eq!(count(&app), before + 1);
            assert!(app.engine.undo().is_ok());
            assert_eq!(count(&app), before);
        }
    }

    #[test]
    fn a_changed_drawing_makes_the_preview_stale() {
        let mut app = app_with_letters();
        let plan = compute(&app, Kind::Returns).expect("a plan");
        app.sc_sign.win.returns.plan = Some(plan);
        assert!(fresh_plan(&mut app, Kind::Returns).is_some());
        let a = app.selection[0];
        app.select(vec![a]);
        assert!(fresh_plan(&mut app, Kind::Returns).is_none());
        // Before Apply the preview shows the letters with their start and
        // corner marks.
        let (scene, _) = scene(&mut app, Kind::Returns);
        // The square, its start dot and direction, three corners.
        assert_eq!(scene.items.len(), 1 + 2 + 3);
    }

    #[test]
    fn the_led_preview_marks_the_drawing() {
        let mut app = app_with_letters();
        app.sc_sign.win.led.show = true;
        app.docker_preview.clear();
        draw(&mut app, ToolId::LedModules);
        assert!(!app.docker_preview.is_empty());
        let n = app.docker_preview.len();
        // Cached: drawing again gives the same marks without work.
        app.docker_preview.clear();
        draw(&mut app, ToolId::LedModules);
        assert_eq!(app.docker_preview.len(), n);
    }
}
