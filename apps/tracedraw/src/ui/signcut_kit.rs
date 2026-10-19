//! Building blocks of the Sign & Cut tool windows: the movable window in
//! the dialog chrome, titled group boxes, form rows (label left, field
//! right), number, length and list fields, the preset row with its
//! settings button, a preview canvas with a yellow status strip, a thin
//! progress bar and the button row (Apply, OK, Cancel or Update, Close).
//! See docs/behavior/sign-and-cut-windows.md.

use crate::app::Units;
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{
    Align, Align2, Color32, Context, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect,
    Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use tracedraw_core::geometry::{BezPath, PathEl, Point};

/// Width of the labels of form rows.
pub const LABEL_W: f32 = 150.0;
/// Width of number fields and lists in form rows.
pub const FIELD_W: f32 = 110.0;
/// Width of the settings column of windows with a preview.
pub const SIDE_W: f32 = 300.0;
/// The status strip under a preview.
pub const STRIP: Color32 = Color32::from_rgb(0xFF, 0xFF, 0x99);

/// A Sign & Cut tool window: the dialog chrome (white title bar with the
/// close button, light grey body), movable, not modal. `raise` brings it
/// to the front. Returns true when its close button was clicked.
///
/// Every window has a set size, `default_size` at first: its content is
/// laid out in that size and never makes the window grow (fields that
/// take the available width would otherwise widen it frame after frame).
/// Windows that are not `resizable` keep at least their default size.
pub fn tool_window(
    ctx: &Context,
    id: egui::Id,
    title: &str,
    default_size: Vec2,
    resizable: bool,
    raise: bool,
    add: impl FnOnce(&mut Ui),
) -> bool {
    let mut closed = false;
    let window = egui::Window::new(title)
        .id(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size(default_size)
        .default_pos(ctx.content_rect().center() - default_size / 2.0)
        .constrain(true)
        .min_size(if resizable {
            Vec2::new(240.0, 160.0)
        } else {
            default_size
        })
        .frame(
            Frame::new()
                .fill(Tokens::title_bar())
                .stroke(Stroke::new(1.0, Tokens::window_border()))
                .shadow(crate::theme::shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(60),
                }))
                .inner_margin(Margin::ZERO),
        );
    let shown = window.show(ctx, |ui| {
        let top = ui.cursor().min;
        let width = ui.available_width().max(default_size.x);
        ui.add_space(super::chrome::TITLE_BAR);
        Frame::new()
            .fill(Tokens::panel())
            .inner_margin(Margin::same(10))
            .outer_margin(Margin {
                left: super::chrome::EDGE as i8,
                right: super::chrome::EDGE as i8,
                top: 0,
                bottom: super::chrome::EDGE as i8,
            })
            .show(ui, |ui| {
                super::chrome::form_style(ui);
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 5.0);
                add(ui);
            });
        let width = ui.min_rect().width().max(width);
        let bar = Rect::from_min_size(top, Vec2::new(width, super::chrome::TITLE_BAR));
        if super::chrome::title_bar(ui, bar, title, &format!("{id:?}")) {
            closed = true;
        }
    });
    if raise {
        if let Some(r) = shown {
            ctx.move_to_top(r.response.layer_id);
        }
    }
    closed
}

/// A titled group box: a light border with the title over its top line.
pub fn group<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(8.0);
    let font = FontId::proportional(12.0);
    let inner = Frame::new()
        .stroke(Stroke::new(1.0, Tokens::border()))
        .inner_margin(Margin {
            left: 8,
            right: 8,
            top: 12,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    let r = inner.response.rect;
    let galley = ui
        .painter()
        .layout_no_wrap(title.to_string(), font, Tokens::text());
    let pos = Pos2::new(r.left() + 8.0, r.top() - galley.size().y / 2.0);
    ui.painter().rect_filled(
        Rect::from_min_size(
            pos - Vec2::new(3.0, 0.0),
            galley.size() + Vec2::new(6.0, 0.0),
        ),
        0.0,
        Tokens::panel(),
    );
    ui.painter().galley(pos, galley, Tokens::text());
    inner.inner
}

/// A thin line between parts of a group.
pub fn separator(ui: &mut Ui) {
    ui.add_space(2.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        r.x_range(),
        r.center().y,
        Stroke::new(1.0, Tokens::border()),
    );
    ui.add_space(2.0);
}

/// A form row: the label at the left, `add` at the right edge.
pub fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_sized(
            [LABEL_W.min(ui.available_width() * 0.55), 22.0],
            egui::Label::new(label).truncate(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), add)
            .inner
    })
    .inner
}

/// A number field of the row width.
pub fn number(ui: &mut Ui, v: &mut f64, min: f64, max: f64, decimals: usize) -> Response {
    ui.add_sized(
        [FIELD_W, 22.0],
        crate::ui::field::NumField::new(v)
            .range(min..=max)
            .max_decimals(decimals),
    )
}

/// A whole-number field of the row width.
pub fn count(ui: &mut Ui, v: &mut usize, min: usize, max: usize) -> Response {
    let mut f = *v as f64;
    let r = ui.add_sized(
        [FIELD_W, 22.0],
        crate::ui::field::NumField::new(&mut f)
            .range(min as f64..=max as f64)
            .max_decimals(0),
    );
    if r.changed() && f.is_finite() {
        *v = f.round().clamp(min as f64, max as f64) as usize;
    }
    r
}

/// A length field in the document's units holding millimetres.
pub fn length(ui: &mut Ui, u: Units, mm: &mut f64, min_mm: f64, max_mm: f64) -> Response {
    let mut v = u.world_from_mm(*mm);
    let r = ui.add_sized(
        [FIELD_W, 22.0],
        crate::ui::field::NumField::new(&mut v)
            .range(u.world_from_mm(min_mm)..=u.world_from_mm(max_mm))
            .max_decimals(3)
            .suffix(format!(" {}", u.short()))
            .unit_mm(u.mm()),
    );
    if r.changed() {
        *mm = u.world_to_mm(v).clamp(min_mm, max_mm);
    }
    r
}

/// A drop-down list of the row width; returns true when the choice changed.
pub fn choice<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut T,
    options: &[(T, String)],
) -> bool {
    let before = *value;
    let shown = options
        .iter()
        .find(|(o, _)| *o == *value)
        .map(|(_, l)| l.clone())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .width(FIELD_W)
        .show_ui(ui, |ui| {
            for (o, l) in options {
                ui.selectable_value(value, *o, l);
            }
        });
    *value != before
}

/// The Units row: the document's units, which every length field of the
/// window follows.
pub fn units_row(ui: &mut Ui, units: &mut Units) -> bool {
    row(ui, &tr("signcut.units"), |ui| {
        let options: Vec<(Units, String)> = Units::ALL.iter().map(|u| (*u, u.label())).collect();
        choice(ui, "signcut_units", units, &options)
    })
}

/// The named presets of one tool, kept as JSON values.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ToolPresets {
    /// The preset chosen last.
    #[serde(default)]
    pub current: Option<String>,
    #[serde(default)]
    pub items: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Every tool's presets, kept in `signcut_presets.json` in the settings
/// folder (in memory only where there is none, as in the browser).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PresetStore {
    #[serde(default)]
    pub tools: std::collections::BTreeMap<String, ToolPresets>,
    #[serde(skip)]
    loaded: bool,
}

impl PresetStore {
    fn path() -> Option<std::path::PathBuf> {
        if cfg!(test) {
            return None;
        }
        crate::settings::config_dir().map(|d| d.join("signcut_presets.json"))
    }

    fn load(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(p) = Self::path() else { return };
        let Ok(text) = crate::files::read_to_string(&p) else {
            return;
        };
        match serde_json::from_str::<PresetStore>(&text) {
            Ok(s) => self.tools = s.tools,
            Err(e) => log::warn!("signcut presets: {e}"),
        }
    }

    fn save(&self) {
        let Some(p) = Self::path() else { return };
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = crate::files::write_creating(&p, text) {
                    log::warn!("signcut presets: {e}");
                }
            }
            Err(e) => log::warn!("signcut presets: {e}"),
        }
    }

    /// The presets of `tool`, read from disk the first time.
    pub fn tool(&mut self, tool: &str) -> &mut ToolPresets {
        self.load();
        self.tools.entry(tool.to_string()).or_default()
    }
}

/// The preset row at the top of a settings column: a list of the saved
/// presets and a settings button that shows the row to save, rename or
/// delete them. Picking a preset loads it into `value`. Returns true when
/// `value` was replaced.
pub fn preset_row<T>(ui: &mut Ui, store: &mut PresetStore, tool: &str, value: &mut T) -> bool
where
    T: serde::Serialize + serde::de::DeserializeOwned + Default,
{
    let mut replaced = false;
    let id = egui::Id::new(("signcut_presets", tool));
    let mut editing: bool = ui.data(|d| d.get_temp(id.with("edit"))).unwrap_or(false);
    let mut name: String = ui
        .data(|d| d.get_temp(id.with("name")))
        .unwrap_or_else(|| store.tool(tool).current.clone().unwrap_or_default());
    let mut save = false;
    ui.horizontal(|ui| {
        let presets = store.tool(tool);
        let current = presets.current.clone();
        let shown = current.clone().unwrap_or_else(|| tr("signcut.preset_none"));
        let mut picked: Option<String> = None;
        Frame::new()
            .fill(Tokens::field())
            .stroke(Stroke::new(1.0, Tokens::control_border()))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(tr("signcut.preset"))
                            .size(10.0)
                            .color(Tokens::text_dim()),
                    );
                    egui::ComboBox::from_id_salt(id.with("list"))
                        .selected_text(RichText::new(shown).strong())
                        .width(ui.available_width().clamp(120.0, SIDE_W - 70.0))
                        .show_ui(ui, |ui| {
                            for k in presets.items.keys() {
                                if ui
                                    .selectable_label(Some(k) == current.as_ref(), k)
                                    .clicked()
                                {
                                    picked = Some(k.clone());
                                }
                            }
                        });
                });
            });
        if let Some(k) = picked {
            if let Some(v) = presets.items.get(&k) {
                if let Ok(t) = serde_json::from_value::<T>(v.clone()) {
                    *value = t;
                    replaced = true;
                }
            }
            presets.current = Some(k.clone());
            name = k;
            store.save();
        }
        if gear_button(ui, editing)
            .on_hover_text(tr("signcut.preset_manage"))
            .clicked()
        {
            editing = !editing;
        }
    });
    if editing {
        ui.horizontal(|ui| {
            ui.add_sized(
                [140.0, 22.0],
                egui::TextEdit::singleline(&mut name).hint_text(tr("signcut.preset_name")),
            );
            let named = !name.trim().is_empty();
            if ui
                .add_enabled(named, egui::Button::new(tr("signcut.preset_save")))
                .clicked()
            {
                save = true;
            }
            if ui
                .add_enabled(named, egui::Button::new(tr("signcut.preset_delete")))
                .clicked()
            {
                let presets = store.tool(tool);
                presets.items.remove(name.trim());
                if presets.current.as_deref() == Some(name.trim()) {
                    presets.current = None;
                }
                store.save();
            }
            if ui.button(tr("signcut.preset_reset")).clicked() {
                *value = T::default();
                replaced = true;
            }
        });
    }
    if save {
        let key = name.trim().to_string();
        if let Ok(v) = serde_json::to_value(&*value) {
            let presets = store.tool(tool);
            presets.items.insert(key.clone(), v);
            presets.current = Some(key);
            store.save();
        }
    }
    ui.data_mut(|d| {
        d.insert_temp(id.with("edit"), editing);
        d.insert_temp(id.with("name"), name);
    });
    replaced
}

/// A square button with a gear drawn on it.
fn gear_button(ui: &mut Ui, on: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
    let fill = if on || resp.hovered() {
        Tokens::tool_hover()
    } else {
        Tokens::control_fill()
    };
    let p = ui.painter();
    p.rect(
        r,
        CornerRadius::ZERO,
        fill,
        Stroke::new(1.0, Tokens::control_border()),
        StrokeKind::Inside,
    );
    let c = r.center();
    let col = Tokens::text();
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        let d = Vec2::new(a.cos(), a.sin());
        p.line_segment([c + d * 7.0, c + d * 11.0], Stroke::new(3.0, col));
    }
    p.circle_stroke(c, 7.5, Stroke::new(2.0, col));
    p.circle_filled(c, 3.0, col);
    resp
}

/// What a preview shows: page-space outlines with their colours, the
/// extent to fit (the outlines' bounds when `None`) and lines of text
/// under the drawing.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub items: Vec<(BezPath, Color32, f32)>,
    pub extent: Option<tracedraw_core::geometry::Rect>,
    pub captions: Vec<String>,
}

impl Scene {
    pub fn add(&mut self, path: BezPath, color: Color32, width: f32) {
        self.items.push((path, color, width));
    }
}

/// A white preview canvas of `size` with the scene fitted in it, y up.
pub fn preview(ui: &mut Ui, size: Vec2, scene: &Scene) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter_at(rect);
    p.rect(
        rect,
        CornerRadius::ZERO,
        Color32::WHITE,
        Stroke::new(1.0, Tokens::border()),
        StrokeKind::Inside,
    );
    let caption_h = 22.0 * scene.captions.len() as f32;
    let area = Rect::from_min_max(
        rect.min + Vec2::splat(16.0),
        rect.max - Vec2::new(16.0, 16.0 + caption_h),
    );
    let mut ext = scene.extent;
    if ext.is_none() {
        for (path, _, _) in &scene.items {
            let b = tracedraw_core::geometry::Shape::bounding_box(path);
            if b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite() {
                ext = Some(ext.map_or(b, |e| e.union(b)));
            }
        }
    }
    if let Some(e) = ext {
        let (ew, eh) = (e.width().max(1e-6), e.height().max(1e-6));
        let s = ((area.width() as f64) / ew).min((area.height() as f64) / eh);
        if s.is_finite() && s > 0.0 {
            let ox = area.center().x as f64 - (e.x0 + ew / 2.0) * s;
            let oy = area.center().y as f64 + (e.y0 + eh / 2.0) * s;
            let to = |q: Point| Pos2::new((ox + q.x * s) as f32, (oy - q.y * s) as f32);
            let tol = (0.5 / s).max(1e-4);
            for (path, color, width) in &scene.items {
                let mut pts: Vec<Pos2> = Vec::new();
                let mut start: Option<Pos2> = None;
                let flush = |pts: &mut Vec<Pos2>| {
                    if pts.len() > 1 {
                        p.add(egui::Shape::line(
                            std::mem::take(pts),
                            Stroke::new(*width, *color),
                        ));
                    } else {
                        pts.clear();
                    }
                };
                tracedraw_core::geometry::flatten(path, tol, &mut |el| match el {
                    PathEl::MoveTo(q) => {
                        flush(&mut pts);
                        let s = to(q);
                        start = Some(s);
                        pts.push(s);
                    }
                    PathEl::LineTo(q) => pts.push(to(q)),
                    PathEl::ClosePath => {
                        if let Some(s) = start {
                            pts.push(s);
                        }
                    }
                    _ => {}
                });
                flush(&mut pts);
            }
        }
    }
    for (i, c) in scene.captions.iter().enumerate() {
        p.text(
            Pos2::new(area.left(), area.bottom() + 6.0 + 22.0 * i as f32),
            Align2::LEFT_TOP,
            c,
            FontId::monospace(16.0),
            Color32::from_gray(40),
        );
    }
    resp
}

/// The yellow status strip under a preview.
pub fn status_strip(ui: &mut Ui, width: f32, text: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, STRIP);
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::border()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        Pos2::new(r.left() + 4.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(12.0),
        Color32::BLACK,
    );
}

/// A thin full-width progress bar (`f` from 0 to 1).
pub fn progress(ui: &mut Ui, f: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, Tokens::control_fill());
    let mut done = r;
    done.set_width(r.width() * f.clamp(0.0, 1.0));
    ui.painter()
        .rect_filled(done, 0.0, Color32::from_rgb(0x06, 0xB0, 0x25));
}

/// The buttons at the bottom right of a window.
#[allow(dead_code)] // OkCancel and Ok: for the tool windows still to come
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buttons {
    ApplyOkCancel,
    OkCancel,
    Ok,
    UpdateClose,
    Close,
}

/// Which bottom button was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressed {
    Apply,
    Ok,
    Cancel,
    Update,
    Close,
}

/// The bottom row: `left` draws extra actions at the left, the standard
/// buttons sit at the right (Update at the left for calculators).
pub fn bottom(
    ui: &mut Ui,
    buttons: Buttons,
    enabled: bool,
    left: impl FnOnce(&mut Ui),
) -> Option<Pressed> {
    let mut out = None;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if buttons == Buttons::UpdateClose && push(ui, &tr("signcut.update"), enabled) {
            out = Some(Pressed::Update);
        }
        left(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut b = |ui: &mut Ui, key: &str, p: Pressed, on: bool| {
                if push(ui, &tr(key), on) {
                    out = Some(p);
                }
            };
            match buttons {
                Buttons::ApplyOkCancel => {
                    b(ui, "signcut.cancel", Pressed::Cancel, true);
                    b(ui, "signcut.ok", Pressed::Ok, enabled);
                    b(ui, "signcut.apply", Pressed::Apply, enabled);
                }
                Buttons::OkCancel => {
                    b(ui, "signcut.cancel", Pressed::Cancel, true);
                    b(ui, "signcut.ok", Pressed::Ok, enabled);
                }
                Buttons::Ok => b(ui, "signcut.ok", Pressed::Ok, enabled),
                Buttons::UpdateClose | Buttons::Close => {
                    b(ui, "signcut.close", Pressed::Close, true)
                }
            }
        });
    });
    out
}

/// A push button of the dialog size.
pub fn push(ui: &mut Ui, text: &str, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(text).min_size(Vec2::new(86.0, 24.0)),
    )
    .clicked()
}

/// Colours the previews use.
pub mod colors {
    use egui::Color32;
    pub const SHEET: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);
    pub const SHEET_EDGE: Color32 = Color32::from_rgb(0xE0, 0x20, 0x20);
    pub const PART: Color32 = Color32::from_rgb(0x20, 0x20, 0x20);
    pub const ORIGINAL: Color32 = Color32::from_rgb(0x99, 0x99, 0x99);
    #[allow(dead_code)]
    pub const ACCENT: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xD9);
}
