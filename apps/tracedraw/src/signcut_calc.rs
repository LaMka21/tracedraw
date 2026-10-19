//! Sign & Cut, Calculations: settings and operations (cutting time, cost and material volume and weight).
//! The geometry is in `tracedraw_signcut::calc`; the panel section is
//! `ui/signcut_calc.rs`. See docs/behavior/sign-and-cut-calc.md.
//!
//! The calculations read the drawing and change nothing in it: no
//! commands, no undo steps. The objects are measured once
//! ([`App::sc_calc_measure`]); time, cost and weight are then worked out
//! from that measurement and the current settings every time they are
//! shown, so changing a price or a profile shows at once.

use crate::app::App;
use crate::i18n::{tr, trf};
use egui::Ui;
use serde_json::{json, Value};
use tracedraw_core::geometry::Point;
use tracedraw_core::ShapeId;
use tracedraw_signcut::calc::cost::{
    self, CostReport, CostSpec, Material, MaterialUnit, Quantities, Service, ServiceUnit,
};
use tracedraw_signcut::calc::csv::Imported;
use tracedraw_signcut::calc::time::{self, TimeEstimate, TimeProfile, TravelOrder};
use tracedraw_signcut::calc::weight::{self, Density, VolumeWeight};
use tracedraw_signcut::calc::{self as calc, Job};

/// Most rows a list keeps (an import is cut there).
pub const MAX_ROWS: usize = 500;
/// Longest name kept in a list, in characters.
pub const MAX_NAME: usize = 120;
/// Sheet thickness accepted (mm).
pub const THICKNESS_RANGE: (f64, f64) = (0.0, 10_000.0);

/// The last measurement of the objects.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    /// The drawing revision it was taken at.
    pub revision: u64,
    /// The objects measured.
    pub targets: Vec<ShapeId>,
    /// The travel order it was measured with.
    pub order: TravelOrder,
    pub job: Job,
}

/// The editable lists of the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Materials,
    Services,
    Densities,
}

impl List {
    /// The file name an export suggests.
    pub fn file_name(self) -> &'static str {
        match self {
            List::Materials => "materials.csv",
            List::Services => "services.csv",
            List::Densities => "densities.csv",
        }
    }
}

/// The Calculations section's settings and last results.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Cutting time profiles and the one in use.
    pub profiles: Vec<TimeProfile>,
    pub profile: usize,
    pub order: TravelOrder,
    /// Material price list and the material billed (`None`: none).
    pub materials: Vec<Material>,
    pub material: Option<usize>,
    /// Service price list (each row says whether it is billed).
    pub services: Vec<Service>,
    pub spec: CostSpec,
    /// Shown after amounts (free text, may be empty).
    pub currency: String,
    /// Density list, the one in use and the sheet thickness (mm).
    pub densities: Vec<Density>,
    pub density: usize,
    pub thickness: f64,
    /// The last measurement.
    pub measured: Option<Measured>,
    /// The lists were read from the preferences folder (or that was
    /// tried).
    pub loaded: bool,
    /// Something kept across restarts changed since it was written.
    pub dirty: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            profiles: vec![
                TimeProfile::default(),
                TimeProfile {
                    name: "Thick film, two passes".into(),
                    cut_speed: 150.0,
                    travel_speed: 500.0,
                    lift_time: 0.3,
                    accel_factor: 1.3,
                    passes: 2,
                },
            ],
            profile: 0,
            order: TravelOrder::Nearest,
            materials: cost::default_materials(),
            material: Some(0),
            services: cost::default_services(),
            spec: CostSpec::default(),
            currency: String::new(),
            densities: weight::default_densities(),
            density: 0,
            thickness: 3.0,
            measured: None,
            loaded: false,
            dirty: false,
        }
    }
}

impl Settings {
    /// The profile in use (the default one when the list is empty).
    pub fn current_profile(&self) -> TimeProfile {
        self.profiles
            .get(self.profile)
            .cloned()
            .unwrap_or_default()
            .sanitized()
    }

    pub fn current_material(&self) -> Option<&Material> {
        self.material.and_then(|i| self.materials.get(i))
    }

    pub fn current_density(&self) -> Option<&Density> {
        self.densities.get(self.density)
    }

    /// Indices back in range after a list changed.
    pub fn fix_indices(&mut self) {
        if self.profile >= self.profiles.len() {
            self.profile = self.profiles.len().saturating_sub(1);
        }
        if self.material.is_some_and(|i| i >= self.materials.len()) {
            self.material = None;
        }
        if self.density >= self.densities.len() {
            self.density = self.densities.len().saturating_sub(1);
        }
    }

    /// The list as CSV text.
    pub fn export_csv(&self, list: List) -> String {
        match list {
            List::Materials => cost::materials_to_csv(&self.materials),
            List::Services => cost::services_to_csv(&self.services),
            List::Densities => weight::densities_to_csv(&self.densities),
        }
    }

    /// Replace a list with the rows read from CSV `text`. A text with no
    /// usable row leaves the list as it was. Returns (rows read, rows
    /// left out).
    pub fn import_csv(&mut self, list: List, text: &str) -> (usize, usize) {
        fn take<T>(got: Imported<T>, mut name: impl FnMut(&mut T)) -> (Vec<T>, usize) {
            let over = got.items.len().saturating_sub(MAX_ROWS);
            let mut items: Vec<T> = got.items.into_iter().take(MAX_ROWS).collect();
            items.iter_mut().for_each(&mut name);
            (items, got.skipped + over)
        }
        let (n, skipped) = match list {
            List::Materials => {
                let (items, skipped) = take(cost::materials_from_csv(text), |m| {
                    m.name = clean_name(&m.name)
                });
                let n = items.len();
                if n > 0 {
                    self.materials = items;
                    self.material = Some(0);
                }
                (n, skipped)
            }
            List::Services => {
                let (items, skipped) = take(cost::services_from_csv(text), |s| {
                    s.name = clean_name(&s.name)
                });
                let n = items.len();
                if n > 0 {
                    self.services = items;
                }
                (n, skipped)
            }
            List::Densities => {
                let (items, skipped) = take(weight::densities_from_csv(text), |d| {
                    d.name = clean_name(&d.name)
                });
                let n = items.len();
                if n > 0 {
                    self.densities = items;
                    self.density = 0;
                }
                (n, skipped)
            }
        };
        if n > 0 {
            self.dirty = true;
        }
        self.fix_indices();
        (n, skipped)
    }

    /// What is kept across restarts, as JSON.
    pub fn to_json(&self) -> Value {
        json!({
            "kind": FILE_KIND,
            "version": 1,
            "profiles": self.profiles.iter().map(|p| json!({
                "name": p.name,
                "cut_speed": p.cut_speed,
                "travel_speed": p.travel_speed,
                "lift_time": p.lift_time,
                "accel_factor": p.accel_factor,
                "passes": p.passes,
            })).collect::<Vec<_>>(),
            "profile": self.profile,
            "order": match self.order {
                TravelOrder::Nearest => "nearest",
                TravelOrder::AsDrawn => "as_drawn",
            },
            "materials": self.materials.iter().map(|m| json!({
                "name": m.name,
                "unit": m.unit.id(),
                "price": m.price,
                "roll_width": m.roll_width,
            })).collect::<Vec<_>>(),
            "material": self.material,
            "services": self.services.iter().map(|s| json!({
                "name": s.name,
                "unit": s.unit.id(),
                "price": s.price,
                "selected": s.selected,
            })).collect::<Vec<_>>(),
            "reserve": self.spec.reserve,
            "min_width": self.spec.min_width,
            "min_height": self.spec.min_height,
            "copies": self.spec.copies,
            "currency": self.currency,
            "densities": self.densities.iter().map(|d| json!({
                "name": d.name,
                "g_cm3": d.g_cm3,
            })).collect::<Vec<_>>(),
            "density": self.density,
            "thickness": self.thickness,
        })
    }

    /// Settings read from [`Settings::to_json`] text: what is missing or
    /// unreadable keeps its default; `None` when the text is not such a
    /// file.
    pub fn from_json(text: &str) -> Option<Settings> {
        let v: Value = serde_json::from_str(text).ok()?;
        if v.get("kind").and_then(Value::as_str) != Some(FILE_KIND) {
            return None;
        }
        let mut s = Settings::default();
        let num = |o: &Value, k: &str| o.get(k).and_then(Value::as_f64).filter(|x| x.is_finite());
        let name_of = |o: &Value, k: &str| o.get(k).and_then(Value::as_str).map(clean_name);
        let list = |k: &str| -> Vec<Value> {
            v.get(k)
                .and_then(Value::as_array)
                .map(|a| a.iter().take(MAX_ROWS).cloned().collect())
                .unwrap_or_default()
        };
        let d = TimeProfile::default();
        let profiles: Vec<TimeProfile> = list("profiles")
            .iter()
            .filter_map(|o| {
                Some(
                    TimeProfile {
                        name: name_of(o, "name")?,
                        cut_speed: num(o, "cut_speed").unwrap_or(d.cut_speed),
                        travel_speed: num(o, "travel_speed").unwrap_or(d.travel_speed),
                        lift_time: num(o, "lift_time").unwrap_or(d.lift_time),
                        accel_factor: num(o, "accel_factor").unwrap_or(d.accel_factor),
                        passes: o
                            .get("passes")
                            .and_then(Value::as_u64)
                            .map_or(1, |p| p.min(u64::from(time::MAX_PASSES)) as u32),
                    }
                    .sanitized(),
                )
            })
            .collect();
        if !profiles.is_empty() {
            s.profiles = profiles;
        }
        let materials: Vec<Material> = list("materials")
            .iter()
            .filter_map(|o| {
                Some(Material {
                    name: name_of(o, "name")?,
                    unit: MaterialUnit::from_id(o.get("unit")?.as_str()?)?,
                    price: num(o, "price")?.max(0.0),
                    roll_width: num(o, "roll_width").unwrap_or(0.0).max(0.0),
                })
            })
            .collect();
        if !materials.is_empty() {
            s.materials = materials;
        }
        let services: Vec<Service> = list("services")
            .iter()
            .filter_map(|o| {
                Some(Service {
                    name: name_of(o, "name")?,
                    unit: ServiceUnit::from_id(o.get("unit")?.as_str()?)?,
                    price: num(o, "price")?.max(0.0),
                    selected: o.get("selected").and_then(Value::as_bool).unwrap_or(false),
                })
            })
            .collect();
        if !services.is_empty() {
            s.services = services;
        }
        let densities: Vec<Density> = list("densities")
            .iter()
            .filter_map(|o| {
                Some(Density {
                    name: name_of(o, "name")?,
                    g_cm3: num(o, "g_cm3").filter(|g| *g > 0.0)?,
                })
            })
            .collect();
        if !densities.is_empty() {
            s.densities = densities;
        }
        let index = |k: &str| v.get(k).and_then(Value::as_u64).map(|i| i as usize);
        if let Some(i) = index("profile") {
            s.profile = i;
        }
        if v.get("material").is_some_and(Value::is_null) {
            s.material = None;
        } else if let Some(i) = index("material") {
            s.material = Some(i);
        }
        if let Some(i) = index("density") {
            s.density = i;
        }
        s.order = match v.get("order").and_then(Value::as_str) {
            Some("as_drawn") => TravelOrder::AsDrawn,
            _ => TravelOrder::Nearest,
        };
        let spec = CostSpec {
            reserve: num(&v, "reserve").unwrap_or(s.spec.reserve),
            min_width: num(&v, "min_width").unwrap_or(0.0),
            min_height: num(&v, "min_height").unwrap_or(0.0),
            copies: v
                .get("copies")
                .and_then(Value::as_u64)
                .map_or(1, |c| c.min(u64::from(cost::MAX_COPIES)) as u32),
        };
        s.spec = spec.sanitized();
        if let Some(c) = name_of(&v, "currency") {
            s.currency = c;
        }
        if let Some(t) = num(&v, "thickness") {
            s.thickness = t.clamp(THICKNESS_RANGE.0, THICKNESS_RANGE.1);
        }
        s.fix_indices();
        Some(s)
    }

    /// Read the kept settings when that has not been tried yet. Unit tests
    /// never read the preferences folder they share.
    pub fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if cfg!(test) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        let text = crate::web::storage_get(WEB_KEY);
        #[cfg(not(target_arch = "wasm32"))]
        let text = file_path().and_then(|p| {
            let too_big = std::fs::metadata(&p).is_ok_and(|m| m.len() > MAX_FILE);
            if too_big {
                log::warn!("{} is too large to read", p.display());
                return None;
            }
            std::fs::read_to_string(&p).ok()
        });
        if let Some(read) = text.as_deref().and_then(Settings::from_json) {
            let measured = self.measured.take();
            *self = Settings {
                measured,
                loaded: true,
                ..read
            };
        }
    }

    /// Keep the settings for the next start (when they changed).
    pub fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        if cfg!(test) {
            return;
        }
        let text = self.to_json().to_string();
        #[cfg(target_arch = "wasm32")]
        crate::web::storage_set(WEB_KEY, &text);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(p) = file_path() {
            let tmp = p.with_extension("json.tmp");
            let done = crate::files::write_creating(&tmp, text.as_bytes())
                .and_then(|_| std::fs::rename(&tmp, &p));
            if let Err(e) = done {
                log::warn!(
                    "could not keep the calculation lists in {}: {e}",
                    p.display()
                );
            }
        }
    }
}

/// The settings file's kind tag.
const FILE_KIND: &str = "tracedraw-signcut-calc";
/// Largest settings file read (bytes).
#[cfg(not(target_arch = "wasm32"))]
const MAX_FILE: u64 = 4 << 20;
#[cfg(target_arch = "wasm32")]
const WEB_KEY: &str = "tracedraw-signcut-calc";

/// Where the settings are kept (desktop).
#[cfg(not(target_arch = "wasm32"))]
fn file_path() -> Option<std::path::PathBuf> {
    crate::settings::config_dir().map(|d| d.join("signcut_calc.json"))
}

/// A list row's name: one line, at most [`MAX_NAME`] characters.
pub fn clean_name(name: &str) -> String {
    let one: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_NAME)
        .collect();
    one.trim().to_string()
}

/// Every result of the section at once (for the status line and tests).
#[derive(Debug, Clone, PartialEq)]
pub struct Results {
    pub time: TimeEstimate,
    pub cost: CostReport,
    pub weight: Option<VolumeWeight>,
}

impl App {
    /// Measure the objects the Sign & Cut tools work on (the selection,
    /// else the page) for the calculations.
    pub fn sc_calc_measure(&mut self) -> Job {
        let targets = self.signcut_targets();
        let doc = self.doc();
        let outlines: Vec<_> = targets
            .iter()
            .map(|id| {
                doc.shape_on_page(*id)
                    .map(|s| tracedraw_signcut::outlines(&s))
                    .unwrap_or_default()
            })
            .collect();
        let order = self.sc_calc.order;
        // The head starts at the page's origin (bottom-left corner).
        let job = calc::job(&outlines, Point::ZERO, order);
        self.sc_calc.measured = Some(Measured {
            revision: self.engine.revision(),
            targets,
            order,
            job: job.clone(),
        });
        job
    }

    /// The last measurement no longer matches the drawing, the objects or
    /// the travel order.
    pub fn sc_calc_outdated(&self) -> bool {
        match &self.sc_calc.measured {
            None => true,
            Some(m) => {
                m.revision != self.engine.revision()
                    || m.order != self.sc_calc.order
                    || m.targets != self.signcut_targets()
            }
        }
    }

    /// Cutting time of the last measurement with the profile in use.
    pub fn sc_calc_time(&self) -> Option<TimeEstimate> {
        let m = self.sc_calc.measured.as_ref()?;
        Some(time::estimate(
            m.job.measurement.length,
            &m.job.travel,
            &self.sc_calc.current_profile(),
        ))
    }

    /// Cost of the last measurement with the material and services
    /// chosen (the hours of the hourly services from the cutting time).
    pub fn sc_calc_cost(&self) -> Option<CostReport> {
        let m = self.sc_calc.measured.as_ref()?;
        let t = self.sc_calc_time()?;
        let q = Quantities {
            blocks: m.job.blocks.clone(),
            cut_length: m.job.measurement.length,
            hours: t.total_s / 3600.0,
        };
        let s = &self.sc_calc;
        Some(cost::cost(&q, s.current_material(), &s.services, &s.spec))
    }

    /// Volume and weight of the last measurement in the material and
    /// thickness chosen (`None` without a measurement or a density).
    pub fn sc_calc_weight(&self) -> Option<VolumeWeight> {
        let m = self.sc_calc.measured.as_ref()?;
        let d = self.sc_calc.current_density()?;
        Some(weight::volume_weight(
            m.job.measurement.area,
            m.job.measurement.length,
            self.sc_calc.thickness,
            d.g_cm3,
        ))
    }

    /// Measure and work out everything; the status line sums it up.
    pub fn sc_calc_run(&mut self) -> Option<Results> {
        self.sc_calc_measure();
        let r = Results {
            time: self.sc_calc_time()?,
            cost: self.sc_calc_cost()?,
            weight: self.sc_calc_weight(),
        };
        let (h, m, s) = time::hms(r.time.total_s);
        let w = r.weight.map_or(0.0, |w| w.weight);
        self.status = trf(
            "signcut.calc_status",
            &[
                ("time", &format!("{h}:{m:02}:{s:02}")),
                ("cost", &self.sc_calc_money(r.cost.total)),
                ("weight", &fmt_weight(w)),
            ],
        );
        Some(r)
    }

    /// An amount with the currency.
    pub fn sc_calc_money(&self, v: f64) -> String {
        let c = self.sc_calc.currency.trim();
        if c.is_empty() {
            format!("{v:.2}")
        } else {
            format!("{v:.2} {c}")
        }
    }

    /// Replace a list with a CSV file picked by the user.
    pub fn sc_calc_import(&mut self, list: List) {
        crate::files::Dialog::new()
            .add_filter("CSV", &["csv", "txt"])
            .pick_file(self, move |app, path| {
                match crate::files::read_to_string(&path) {
                    Ok(text) => {
                        app.sc_calc_import_text(list, &text);
                    }
                    Err(e) => app.status = e.to_string(),
                }
            });
    }

    /// Replace a list with CSV text; the status line says what was read.
    pub fn sc_calc_import_text(&mut self, list: List, text: &str) -> (usize, usize) {
        let (n, skipped) = self.sc_calc.import_csv(list, text);
        self.status = if n == 0 {
            tr("signcut.calc_import_none")
        } else {
            trf(
                "signcut.calc_imported",
                &[("n", &n.to_string()), ("skipped", &skipped.to_string())],
            )
        };
        (n, skipped)
    }

    /// Save a list as a CSV file (a download in a browser).
    pub fn sc_calc_export(&mut self, list: List) {
        let text = self.sc_calc.export_csv(list);
        crate::files::Dialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(list.file_name())
            .save_file(self, move |app, path| {
                app.status = match crate::files::write(&path, text.as_bytes()) {
                    Ok(()) => trf(
                        "signcut.calc_exported",
                        &[("path", &path.display().to_string())],
                    ),
                    Err(e) => e.to_string(),
                };
            });
    }
}

/// A weight in grams as g or kg.
pub fn fmt_weight(g: f64) -> String {
    if g.abs() >= 1000.0 {
        format!("{:.3} kg", g / 1000.0)
    } else {
        format!("{g:.1} g")
    }
}

/// Tools > Sign & Cut > Calculations: each item measures the objects,
/// opens the panel at the section and sums the results up in the status
/// line.
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::ui::menus::item;
    let doc = app.has_document() && !app.show_welcome;
    for key in [
        "menu.tools.signcut_calc_time",
        "menu.tools.signcut_calc_cost",
        "menu.tools.signcut_calc_weight",
    ] {
        if item(ui, key, "", doc) {
            app.open_signcut(crate::signcut::Section::Calc);
            app.sc_calc_run();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer to draw on")
    }

    #[test]
    fn calc_time_of_the_selection_with_a_profile() {
        let mut app = App::headless();
        // A 100 x 50 rectangle at the origin and one far away.
        let a = rect(&mut app, 0.0, 0.0, 100.0, 50.0);
        rect(&mut app, 500.0, 0.0, 10.0, 10.0);
        app.select(vec![a]);
        app.sc_calc.profiles = vec![TimeProfile {
            name: "Test".into(),
            cut_speed: 100.0,
            travel_speed: 500.0,
            lift_time: 0.5,
            accel_factor: 1.0,
            passes: 2,
        }];
        app.sc_calc.profile = 0;
        let r = app.sc_calc_run().expect("results");
        // 300 mm at 100 mm/s, twice: 6 s; no travel (it starts at the
        // origin); one lift per pass: 1 s.
        assert!((r.time.cut_s - 6.0).abs() < 1e-9);
        assert_eq!(r.time.travel.travel_length, 0.0);
        assert!((r.time.lift_s - 1.0).abs() < 1e-9);
        assert!((r.time.total_s - 7.0).abs() < 1e-9);
        assert!(!app.sc_calc_outdated());
        assert!(app.status.contains("0:00:07"), "{}", app.status);
        // Selecting something else makes the measurement out of date.
        app.select(vec![]);
        assert!(app.sc_calc_outdated());
        app.sc_calc_measure();
        assert_eq!(
            app.sc_calc.measured.as_ref().map(|m| m.targets.len()),
            Some(2)
        );
    }

    #[test]
    fn calc_changes_nothing_in_the_drawing() {
        let mut app = App::headless();
        rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let before = app.engine.revision();
        let undo = app.engine.undo_label().map(str::to_string);
        app.sc_calc_run();
        assert_eq!(app.engine.revision(), before);
        assert_eq!(app.engine.undo_label().map(str::to_string), undo);
    }

    #[test]
    fn calc_cost_bills_material_and_services() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 990.0, 490.0);
        app.select(vec![a]);
        let s = &mut app.sc_calc;
        s.materials = vec![Material {
            name: "Board".into(),
            unit: MaterialUnit::Area,
            price: 20.0,
            roll_width: 0.0,
        }];
        s.material = Some(0);
        s.services = vec![Service {
            name: "Cutting".into(),
            unit: ServiceUnit::Metre,
            price: 1.0,
            selected: true,
        }];
        s.spec = CostSpec {
            reserve: 5.0,
            min_width: 0.0,
            min_height: 0.0,
            copies: 1,
        };
        s.currency = "EUR".into();
        let r = app.sc_calc_run().expect("results");
        // 1000 x 500 with the reserve: 0.5 m2 x 20 = 10; 2.96 m x 1.
        assert_eq!(r.cost.lines.len(), 2);
        assert!((r.cost.lines[0].amount - 10.0).abs() < 1e-9);
        assert!((r.cost.lines[1].amount - 2.96).abs() < 1e-9);
        assert!((r.cost.total - 12.96).abs() < 1e-9);
        assert_eq!(app.sc_calc_money(r.cost.total), "12.96 EUR");
        // Another price shows at once, without measuring again.
        app.sc_calc.materials[0].price = 40.0;
        let c = app.sc_calc_cost().expect("cost");
        assert!((c.total - 22.96).abs() < 1e-9);
    }

    #[test]
    fn calc_weight_of_acrylic() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 1000.0, 1000.0);
        app.select(vec![a]);
        app.sc_calc.thickness = 3.0;
        app.sc_calc.density = 0;
        assert_eq!(app.sc_calc.densities[0].name, "Acrylic");
        let r = app.sc_calc_run().expect("results");
        let w = r.weight.expect("weight");
        assert!((w.weight - 3570.0).abs() < 1e-6);
        assert!((w.edge_area - 12_000.0).abs() < 1e-6);
        assert_eq!(fmt_weight(w.weight), "3.570 kg");
    }

    #[test]
    fn calc_csv_import_replaces_a_list_and_export_reads_back() {
        let mut app = App::headless();
        let (n, skipped) = app.sc_calc_import_text(
            List::Materials,
            "name;unit;price;roll_width_mm\nFoil;roll;7,5;1000\nBad;roll;1\n",
        );
        assert_eq!((n, skipped), (1, 1));
        assert_eq!(app.sc_calc.materials.len(), 1);
        assert_eq!(app.sc_calc.material, Some(0));
        assert!(app.sc_calc.dirty);
        // A file with nothing usable leaves the list alone.
        let (n, _) = app.sc_calc_import_text(List::Services, "nothing here\n");
        assert_eq!(n, 0);
        assert_eq!(app.sc_calc.services.len(), cost::default_services().len());
        // Export and import give the same list.
        let text = app.sc_calc.export_csv(List::Densities);
        app.sc_calc.densities.clear();
        app.sc_calc_import_text(List::Densities, &text);
        assert_eq!(app.sc_calc.densities, weight::default_densities());
    }

    #[test]
    fn calc_settings_survive_json_and_bad_files() {
        let mut s = Settings::default();
        s.currency = "CHF".into();
        s.thickness = 5.0;
        s.material = None;
        s.order = TravelOrder::AsDrawn;
        s.profiles[1].passes = 3;
        s.services[2].selected = true;
        let back = Settings::from_json(&s.to_json().to_string()).expect("read back");
        assert_eq!(back.currency, "CHF");
        assert_eq!(back.thickness, 5.0);
        assert_eq!(back.material, None);
        assert_eq!(back.order, TravelOrder::AsDrawn);
        assert_eq!(back.profiles, s.profiles);
        assert_eq!(back.materials, s.materials);
        assert_eq!(back.services, s.services);
        assert_eq!(back.densities, s.densities);
        // Not a settings file.
        assert!(Settings::from_json("{}").is_none());
        assert!(Settings::from_json("not json").is_none());
        // Wrong values keep their defaults, indices come back in range.
        let odd = r#"{"kind":"tracedraw-signcut-calc","profiles":[{"name":"x","cut_speed":-4,"passes":9999}],
            "profile":7,"material":99,"density":42,"thickness":1e300,"copies":0,
            "materials":[{"name":"m","unit":"litre","price":1}]}"#;
        let s = Settings::from_json(odd).expect("read");
        assert_eq!(s.profiles.len(), 1);
        assert_eq!(s.profiles[0].cut_speed, time::SPEED_RANGE.0);
        assert_eq!(s.profiles[0].passes, time::MAX_PASSES);
        assert_eq!(s.profile, 0);
        assert_eq!(s.material, None);
        assert_eq!(s.density, s.densities.len() - 1);
        assert_eq!(s.thickness, THICKNESS_RANGE.1);
        assert_eq!(s.spec.copies, 1);
        assert_eq!(s.materials, cost::default_materials());
    }
}
