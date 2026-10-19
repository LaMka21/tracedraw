//! Sign & Cut, Calculations: settings and operations (cutting time, cost and material volume and weight).
//! The geometry is in `tracedraw_signcut::calc`; the windows are in
//! `ui/signcut_win_calc.rs`. See docs/behavior/sign-and-cut-calc.md.
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
use tracedraw_core::geometry::{BezPath, Point};
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
/// Highest price accepted.
pub const MAX_PRICE: f64 = 1.0e9;

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
    /// The outlines measured (page space), for the preview.
    pub paths: Vec<BezPath>,
}

/// A length unit of the calculators' Units lists. Areas and volumes are
/// shown in its square and cube.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalcUnit {
    Mm,
    Cm,
    #[default]
    M,
    In,
    Ft,
}

impl CalcUnit {
    pub const ALL: [CalcUnit; 5] = [
        CalcUnit::Mm,
        CalcUnit::Cm,
        CalcUnit::M,
        CalcUnit::In,
        CalcUnit::Ft,
    ];

    /// Millimetres per unit.
    pub fn mm(self) -> f64 {
        match self {
            CalcUnit::Mm => 1.0,
            CalcUnit::Cm => 10.0,
            CalcUnit::M => 1000.0,
            CalcUnit::In => 25.4,
            CalcUnit::Ft => 304.8,
        }
    }

    /// The unit's symbol, also its name in the settings file.
    pub fn id(self) -> &'static str {
        match self {
            CalcUnit::Mm => "mm",
            CalcUnit::Cm => "cm",
            CalcUnit::M => "m",
            CalcUnit::In => "in",
            CalcUnit::Ft => "ft",
        }
    }

    pub fn from_id(id: &str) -> Option<CalcUnit> {
        CalcUnit::ALL.into_iter().find(|u| u.id() == id)
    }

    /// `mm` millimetres in this unit.
    pub fn len(self, mm: f64) -> f64 {
        mm / self.mm()
    }

    /// `mm2` square millimetres in this unit squared.
    pub fn area(self, mm2: f64) -> f64 {
        mm2 / (self.mm() * self.mm())
    }

    /// `mm3` cubic millimetres in this unit cubed.
    pub fn volume(self, mm3: f64) -> f64 {
        mm3 / (self.mm() * self.mm() * self.mm())
    }

    /// The symbol of the square of the unit.
    pub fn area_id(self) -> String {
        format!("{}\u{b2}", self.id())
    }

    /// The symbol of the cube of the unit.
    pub fn volume_id(self) -> String {
        format!("{}\u{b3}", self.id())
    }
}

/// The unit a mass is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MassUnit {
    G,
    #[default]
    Kg,
    T,
    Lb,
}

impl MassUnit {
    pub const ALL: [MassUnit; 4] = [MassUnit::G, MassUnit::Kg, MassUnit::T, MassUnit::Lb];

    /// Grams per unit.
    pub fn grams(self) -> f64 {
        match self {
            MassUnit::G => 1.0,
            MassUnit::Kg => 1000.0,
            MassUnit::T => 1_000_000.0,
            MassUnit::Lb => 453.592_37,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            MassUnit::G => "g",
            MassUnit::Kg => "kg",
            MassUnit::T => "t",
            MassUnit::Lb => "lb",
        }
    }

    pub fn from_id(id: &str) -> Option<MassUnit> {
        MassUnit::ALL.into_iter().find(|u| u.id() == id)
    }
}

/// What the price of the Volume and Weight window is given per.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PricePer {
    /// Per unit of the mass list.
    #[default]
    Mass,
    /// Per square metre of net area.
    SquareMetre,
    /// Per cubic metre of volume.
    CubicMetre,
}

impl PricePer {
    pub const ALL: [PricePer; 3] = [PricePer::Mass, PricePer::SquareMetre, PricePer::CubicMetre];

    pub fn id(self) -> &'static str {
        match self {
            PricePer::Mass => "mass",
            PricePer::SquareMetre => "m2",
            PricePer::CubicMetre => "m3",
        }
    }

    pub fn from_id(id: &str) -> Option<PricePer> {
        PricePer::ALL.into_iter().find(|u| u.id() == id)
    }
}

/// The tab of the Cutting Time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeTab {
    #[default]
    Time,
    Parameters,
    Preview,
}

/// The part of the Cost window under the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CostLists {
    /// Only the table.
    #[default]
    Hidden,
    Materials,
    Services,
    Job,
}

/// A row of the Cost window's table: objects added together.
#[derive(Debug, Clone, PartialEq)]
pub struct CostRow {
    /// Typed by the user (empty at first).
    pub name: String,
    /// The objects of the row.
    pub targets: Vec<ShapeId>,
    /// The drawing revision the row was measured at.
    pub revision: u64,
    pub job: Job,
    /// The material billed (an index of the material list).
    pub material: Option<usize>,
    /// The fill colour of the row's first object with a plain fill.
    pub colour: Option<[u8; 3]>,
}

/// One row's cost, split into material and services.
#[derive(Debug, Clone, PartialEq)]
pub struct RowCost {
    pub report: CostReport,
    pub material_qty: f64,
    /// The material's quantity unit: m2 or running m (`None`: no material).
    pub material_unit: Option<cost::QtyUnit>,
    pub material_cost: f64,
    pub services_cost: f64,
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
    /// Perimeter window: its unit and the price per unit of length.
    pub perimeter_unit: CalcUnit,
    pub perimeter_price: f64,
    /// Area window: its unit.
    pub area_unit: CalcUnit,
    /// Volume and Weight window: units and the price.
    pub volume_unit: CalcUnit,
    pub mass_unit: MassUnit,
    pub weight_price: f64,
    pub weight_per: PricePer,
    /// Cutting Time window: the price of an hour of cutting.
    pub price_per_hour: f64,
    pub time_tab: TimeTab,
    /// Cost window: the table and the part shown under it.
    pub cost_rows: Vec<CostRow>,
    pub cost_lists: CostLists,
    /// Volume and Weight window: the density list is shown.
    pub density_editor: bool,
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
            perimeter_unit: CalcUnit::M,
            perimeter_price: 1.0,
            area_unit: CalcUnit::M,
            volume_unit: CalcUnit::Mm,
            mass_unit: MassUnit::Kg,
            weight_price: 1.0,
            weight_per: PricePer::Mass,
            price_per_hour: 100.0,
            time_tab: TimeTab::Time,
            cost_rows: Vec::new(),
            cost_lists: CostLists::Hidden,
            density_editor: false,
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
        let n = self.materials.len();
        for r in &mut self.cost_rows {
            if r.material.is_some_and(|i| i >= n) {
                r.material = None;
            }
        }
        if self.density >= self.densities.len() {
            self.density = self.densities.len().saturating_sub(1);
        }
    }

    /// Delete row `i` of the material list; what billed it bills no
    /// material, the indices after it move up.
    pub fn remove_material(&mut self, i: usize) {
        if i >= self.materials.len() {
            return;
        }
        self.materials.remove(i);
        let shift = |m: Option<usize>| match m {
            Some(m) if m == i => None,
            Some(m) if m > i => Some(m - 1),
            m => m,
        };
        self.material = shift(self.material);
        for r in &mut self.cost_rows {
            r.material = shift(r.material);
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
                    for r in &mut self.cost_rows {
                        r.material = Some(0);
                    }
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
            "perimeter_unit": self.perimeter_unit.id(),
            "perimeter_price": self.perimeter_price,
            "area_unit": self.area_unit.id(),
            "volume_unit": self.volume_unit.id(),
            "mass_unit": self.mass_unit.id(),
            "weight_price": self.weight_price,
            "weight_per": self.weight_per.id(),
            "price_per_hour": self.price_per_hour,
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
        let id = |k: &str| v.get(k).and_then(Value::as_str);
        if let Some(u) = id("perimeter_unit").and_then(CalcUnit::from_id) {
            s.perimeter_unit = u;
        }
        if let Some(u) = id("area_unit").and_then(CalcUnit::from_id) {
            s.area_unit = u;
        }
        if let Some(u) = id("volume_unit").and_then(CalcUnit::from_id) {
            s.volume_unit = u;
        }
        if let Some(u) = id("mass_unit").and_then(MassUnit::from_id) {
            s.mass_unit = u;
        }
        if let Some(u) = id("weight_per").and_then(PricePer::from_id) {
            s.weight_per = u;
        }
        let price = |k: &str| num(&v, k).map(|p| p.clamp(0.0, MAX_PRICE));
        if let Some(p) = price("perimeter_price") {
            s.perimeter_price = p;
        }
        if let Some(p) = price("weight_price") {
            s.weight_price = p;
        }
        if let Some(p) = price("price_per_hour") {
            s.price_per_hour = p;
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
            paths: outlines.into_iter().flatten().collect(),
        });
        job
    }

    /// Measure `ids` (page outlines) for a row of the Cost window.
    fn sc_calc_job_of(&self, ids: &[ShapeId]) -> (Job, Option<[u8; 3]>) {
        let doc = self.doc();
        let mut colour = None;
        let outlines: Vec<Vec<BezPath>> = ids
            .iter()
            .map(|id| match doc.shape_on_page(*id) {
                Some(s) => {
                    if colour.is_none() {
                        colour = plain_colour(&s);
                    }
                    tracedraw_signcut::outlines(&s)
                }
                None => Vec::new(),
            })
            .collect();
        (
            calc::job(&outlines, Point::ZERO, self.sc_calc.order),
            colour,
        )
    }

    /// Add the objects the tools work on (the selection, else the page)
    /// as a new row of the Cost window. Returns false when they have
    /// nothing to bill.
    pub fn sc_calc_add_row(&mut self) -> bool {
        let targets = self.signcut_targets();
        let (job, colour) = self.sc_calc_job_of(&targets);
        if job.blocks.is_empty() {
            self.status = tr("signcut.wcalc_cost_nothing");
            return false;
        }
        if self.sc_calc.cost_rows.len() >= MAX_ROWS {
            return false;
        }
        let material = self.sc_calc.material;
        self.sc_calc.cost_rows.push(CostRow {
            name: String::new(),
            targets,
            revision: self.engine.revision(),
            job,
            material,
            colour,
        });
        true
    }

    /// Measure every row of the Cost window again as the drawing is now;
    /// objects that are gone leave their row, a row with none left goes.
    pub fn sc_calc_update_rows(&mut self) {
        let rev = self.engine.revision();
        let rows = std::mem::take(&mut self.sc_calc.cost_rows);
        let mut kept = Vec::new();
        for mut r in rows {
            let doc = self.doc();
            r.targets.retain(|id| doc.find_shape(*id).is_some());
            let (job, colour) = self.sc_calc_job_of(&r.targets);
            if r.targets.is_empty() || job.blocks.is_empty() {
                continue;
            }
            r.job = job;
            r.colour = colour;
            r.revision = rev;
            kept.push(r);
        }
        self.sc_calc.cost_rows = kept;
    }

    /// The cost of a row of the Cost window with the lists as they are
    /// now (the hours of hourly services from its cutting time).
    pub fn sc_calc_row_cost(&self, row: &CostRow) -> RowCost {
        let s = &self.sc_calc;
        let t = time::estimate(
            row.job.measurement.length,
            &row.job.travel,
            &s.current_profile(),
        );
        let q = Quantities {
            blocks: row.job.blocks.clone(),
            cut_length: row.job.measurement.length,
            hours: t.total_s / 3600.0,
        };
        let material = row.material.and_then(|i| s.materials.get(i));
        let report = cost::cost(&q, material, &s.services, &s.spec);
        let m = report.lines.iter().find(|l| l.material);
        RowCost {
            material_qty: m.map_or(0.0, |l| l.quantity),
            material_unit: m.map(|l| l.unit),
            material_cost: m.map_or(0.0, |l| l.amount),
            services_cost: report
                .lines
                .iter()
                .filter(|l| !l.material)
                .map(|l| l.amount)
                .sum(),
            report,
        }
    }

    /// The price of the cutting time at the price per hour, and that
    /// price per square metre of net area (`None` without an area).
    pub fn sc_calc_time_price(&self) -> Option<(f64, Option<f64>)> {
        let t = self.sc_calc_time()?;
        let m = self.sc_calc.measured.as_ref()?;
        let price = t.total_s / 3600.0 * self.sc_calc.price_per_hour.clamp(0.0, MAX_PRICE);
        let m2 = m.job.measurement.area / 1_000_000.0;
        Some((price, (m2 > 1e-12).then(|| price / m2)))
    }

    /// The price of the Volume and Weight window: the price times the
    /// mass (in the mass unit), the net area (m2) or the volume (m3).
    pub fn sc_calc_weight_price(&self) -> Option<f64> {
        let w = self.sc_calc_weight()?;
        let s = &self.sc_calc;
        let qty = match s.weight_per {
            PricePer::Mass => w.weight / s.mass_unit.grams(),
            PricePer::SquareMetre => w.area / 1_000_000.0,
            PricePer::CubicMetre => w.volume / 1_000_000_000.0,
        };
        Some(qty * s.weight_price.clamp(0.0, MAX_PRICE))
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

/// The colour of a plain (solid) fill, for the swatch of a cost row.
fn plain_colour(s: &tracedraw_core::Shape) -> Option<[u8; 3]> {
    match &s.fill {
        tracedraw_core::Fill::Solid(c) => Some(c.to_rgb8()),
        _ => match &s.kind {
            tracedraw_core::ShapeKind::Group { children } => children.iter().find_map(plain_colour),
            _ => None,
        },
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

/// Tools > Sign & Cut > Calculations: each item opens its window and
/// measures the objects (Cost adds them as a row of its table).
pub fn menu(app: &mut App, ui: &mut Ui) {
    use crate::signcut_tools::ToolId;
    use crate::ui::menus::item;
    let doc = app.has_document() && !app.show_welcome;
    for (key, tool) in [
        ("menu.tools.signcut_calc_time", ToolId::CuttingTime),
        ("menu.tools.signcut_calc_cost", ToolId::Cost),
        ("menu.tools.signcut_calc_weight", ToolId::Weight),
    ] {
        if item(ui, key, "", doc) {
            app.sc_calc.ensure_loaded();
            if tool == ToolId::Cost {
                app.open_signcut(crate::signcut::Section::Calc);
                app.sc_calc_add_row();
            } else {
                app.signcut_tool(tool);
                app.sc_calc_run();
            }
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
    fn calc_cost_rows_follow_the_lists_and_the_drawing() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 990.0, 490.0);
        let b = rect(&mut app, 2000.0, 0.0, 90.0, 90.0);
        let s = &mut app.sc_calc;
        s.materials = vec![
            Material {
                name: "Board".into(),
                unit: MaterialUnit::Area,
                price: 20.0,
                roll_width: 0.0,
            },
            Material {
                name: "Film".into(),
                unit: MaterialUnit::Roll,
                price: 10.0,
                roll_width: 1000.0,
            },
        ];
        s.material = Some(0);
        s.services = vec![Service {
            name: "Cutting".into(),
            unit: ServiceUnit::Metre,
            price: 1.0,
            selected: true,
        }];
        s.spec.reserve = 5.0;
        let rev = app.engine.revision();
        app.select(vec![a]);
        assert!(app.sc_calc_add_row());
        app.select(vec![b]);
        assert!(app.sc_calc_add_row());
        assert_eq!(app.engine.revision(), rev);
        assert_eq!(app.sc_calc.cost_rows.len(), 2);
        let c = app.sc_calc_row_cost(&app.sc_calc.cost_rows[0].clone());
        assert!((c.material_cost - 10.0).abs() < 1e-9);
        assert!((c.material_qty - 0.5).abs() < 1e-9);
        assert!((c.services_cost - 2.96).abs() < 1e-9);
        // The second row on the roll: 100 mm of roll at 10 per metre.
        app.sc_calc.cost_rows[1].material = Some(1);
        let c = app.sc_calc_row_cost(&app.sc_calc.cost_rows[1].clone());
        assert!((c.material_cost - 1.0).abs() < 1e-9);
        assert_eq!(c.material_unit, Some(cost::QtyUnit::Metre));
        // Deleting the board: the first row bills no material, the
        // second keeps the film.
        app.sc_calc.remove_material(0);
        assert_eq!(app.sc_calc.cost_rows[0].material, None);
        assert_eq!(app.sc_calc.cost_rows[1].material, Some(0));
        // Update drops the objects that are gone.
        app.run(tracedraw_core::Command::DeleteShapes { shapes: vec![b] });
        app.sc_calc_update_rows();
        assert_eq!(app.sc_calc.cost_rows.len(), 1);
        assert_eq!(app.sc_calc.cost_rows[0].revision, app.engine.revision());
        // Nothing to bill: no row.
        app.run(tracedraw_core::Command::DeleteShapes { shapes: vec![a] });
        app.select(vec![]);
        assert!(!app.sc_calc_add_row());
    }

    #[test]
    fn calc_time_and_weight_prices() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 1000.0, 1000.0);
        app.select(vec![a]);
        app.sc_calc.profiles = vec![TimeProfile {
            name: "Test".into(),
            cut_speed: 100.0,
            travel_speed: 500.0,
            lift_time: 0.0,
            accel_factor: 1.0,
            passes: 9,
        }];
        app.sc_calc.profile = 0;
        app.sc_calc.price_per_hour = 100.0;
        app.sc_calc_run();
        // 4000 mm at 100 mm/s, 9 passes: 360 s = 0.1 h; 1 m2.
        let (price, per_m2) = app.sc_calc_time_price().expect("price");
        assert!((price - 10.0).abs() < 1e-9);
        assert!((per_m2.expect("area") - 10.0).abs() < 1e-9);
        app.sc_calc.thickness = 3.0;
        app.sc_calc.density = 0;
        app.sc_calc.weight_price = 2.0;
        app.sc_calc.mass_unit = MassUnit::Kg;
        app.sc_calc.weight_per = PricePer::Mass;
        assert!((app.sc_calc_weight_price().expect("price") - 7.14).abs() < 1e-9);
        app.sc_calc.weight_per = PricePer::SquareMetre;
        assert!((app.sc_calc_weight_price().expect("price") - 2.0).abs() < 1e-9);
        app.sc_calc.weight_per = PricePer::CubicMetre;
        assert!((app.sc_calc_weight_price().expect("price") - 0.006).abs() < 1e-12);
    }

    #[test]
    fn calc_units_convert() {
        assert_eq!(CalcUnit::M.len(1500.0), 1.5);
        assert_eq!(CalcUnit::Cm.area(100.0), 1.0);
        assert_eq!(CalcUnit::M.volume(1.0e9), 1.0);
        assert_eq!(CalcUnit::M.area_id(), "m\u{b2}");
        for u in CalcUnit::ALL {
            assert_eq!(CalcUnit::from_id(u.id()), Some(u));
        }
        for u in MassUnit::ALL {
            assert_eq!(MassUnit::from_id(u.id()), Some(u));
        }
        for u in PricePer::ALL {
            assert_eq!(PricePer::from_id(u.id()), Some(u));
        }
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
        s.perimeter_unit = CalcUnit::Ft;
        s.mass_unit = MassUnit::Lb;
        s.weight_per = PricePer::CubicMetre;
        s.price_per_hour = 55.5;
        let back = Settings::from_json(&s.to_json().to_string()).expect("read back");
        assert_eq!(back.perimeter_unit, CalcUnit::Ft);
        assert_eq!(back.mass_unit, MassUnit::Lb);
        assert_eq!(back.weight_per, PricePer::CubicMetre);
        assert_eq!(back.price_per_hour, 55.5);
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
