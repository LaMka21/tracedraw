//! The Sign & Cut tools: every function the Sign & Cut toolbar offers,
//! its group on the toolbar, its window and where it is drawn. Several
//! tool windows can be open at once; they do not block the drawing.
//! See docs/behavior/sign-and-cut-windows.md.

use crate::app::App;
use crate::i18n::tr;

/// The nine buttons of the Sign & Cut toolbar, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Nesting,
    Cutting,
    Measure,
    SignMaking,
    Effects,
    Construct,
    Cad,
    Arrange,
    Panel,
}

impl Group {
    pub const ALL: [Group; 9] = [
        Group::Nesting,
        Group::Cutting,
        Group::Measure,
        Group::SignMaking,
        Group::Effects,
        Group::Construct,
        Group::Cad,
        Group::Arrange,
        Group::Panel,
    ];

    /// i18n key of the button's tooltip.
    pub fn key(self) -> &'static str {
        match self {
            Group::Nesting => "signcut.group_nesting",
            Group::Cutting => "signcut.group_cutting",
            Group::Measure => "signcut.group_measure",
            Group::SignMaking => "signcut.group_sign",
            Group::Effects => "signcut.group_effects",
            Group::Construct => "signcut.group_construct",
            Group::Cad => "signcut.group_cad",
            Group::Arrange => "signcut.group_arrange",
            Group::Panel => "signcut.group_panel",
        }
    }

    /// The tools under the button, in menu order (empty for the panel
    /// button, which shows and hides the Sign & Cut panel).
    pub fn tools(self) -> &'static [ToolId] {
        use ToolId::*;
        match self {
            Group::Nesting => &[ShapeNesting, RectNesting, FillSheet],
            Group::Cutting => &[
                PlotterOutput,
                WeedingLines,
                SeparateCurves,
                FindCrossings,
                FindDuplicates,
                RemoveDuplicateEdges,
                PrintAndCut,
                CropMarks,
            ],
            Group::Measure => &[Perimeter, Area, Cost, CuttingTime, Weight],
            Group::SignMaking => &[
                LedModules,
                Grommets,
                NeonTubes,
                VariableData,
                LetterReturns,
                AcrylicMould,
                GCode,
            ],
            Group::Effects => &[Effects, Hatching, Grid],
            Group::Construct => &[
                SignBox,
                SmartDivide,
                Carrier,
                CentreLine,
                JoinCurves,
                ReduceNodes,
                Corners,
            ],
            Group::Cad => &[
                DxfExport,
                Bridges,
                ReferenceScale,
                HotWire,
                CutOrder,
                FingerJointBox,
                FingerJoints,
                DoorPanels,
            ],
            Group::Arrange => &[AlignDistribute, SetSize, Replace],
            Group::Panel => &[],
        }
    }
}

/// One function of the Sign & Cut toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    // Nesting
    ShapeNesting,
    RectNesting,
    FillSheet,
    // Cutting
    PlotterOutput,
    WeedingLines,
    SeparateCurves,
    FindCrossings,
    FindDuplicates,
    RemoveDuplicateEdges,
    PrintAndCut,
    CropMarks,
    // Measure
    Perimeter,
    Area,
    Cost,
    CuttingTime,
    Weight,
    // Sign making
    LedModules,
    Grommets,
    NeonTubes,
    VariableData,
    LetterReturns,
    AcrylicMould,
    GCode,
    // Effects
    Effects,
    Hatching,
    Grid,
    // Construct
    SignBox,
    SmartDivide,
    Carrier,
    CentreLine,
    JoinCurves,
    ReduceNodes,
    Corners,
    // CAD
    DxfExport,
    Bridges,
    ReferenceScale,
    HotWire,
    CutOrder,
    FingerJointBox,
    FingerJoints,
    DoorPanels,
    // Arrange
    AlignDistribute,
    SetSize,
    Replace,
}

impl ToolId {
    /// i18n key of the tool's name (menu item and window title).
    pub fn key(self) -> &'static str {
        use ToolId::*;
        match self {
            ShapeNesting => "signcut.tool_shape_nesting",
            RectNesting => "signcut.tool_rect_nesting",
            FillSheet => "signcut.tool_fill_sheet",
            PlotterOutput => "signcut.tool_plotter_output",
            WeedingLines => "signcut.tool_weeding_lines",
            SeparateCurves => "signcut.tool_separate_curves",
            FindCrossings => "signcut.tool_find_crossings",
            FindDuplicates => "signcut.tool_find_duplicates",
            RemoveDuplicateEdges => "signcut.tool_remove_duplicate_edges",
            PrintAndCut => "signcut.tool_print_and_cut",
            CropMarks => "signcut.tool_crop_marks",
            Perimeter => "signcut.tool_perimeter",
            Area => "signcut.tool_area",
            Cost => "signcut.tool_cost",
            CuttingTime => "signcut.tool_cutting_time",
            Weight => "signcut.tool_weight",
            LedModules => "signcut.tool_led_modules",
            Grommets => "signcut.tool_grommets",
            NeonTubes => "signcut.tool_neon_tubes",
            VariableData => "signcut.tool_variable_data",
            LetterReturns => "signcut.tool_letter_returns",
            AcrylicMould => "signcut.tool_acrylic_mould",
            GCode => "signcut.tool_gcode",
            Effects => "signcut.tool_effects",
            Hatching => "signcut.tool_hatching",
            Grid => "signcut.tool_grid",
            SignBox => "signcut.tool_sign_box",
            SmartDivide => "signcut.tool_smart_divide",
            Carrier => "signcut.tool_carrier",
            CentreLine => "signcut.tool_centre_line",
            JoinCurves => "signcut.tool_join_curves",
            ReduceNodes => "signcut.tool_reduce_nodes",
            Corners => "signcut.tool_corners",
            DxfExport => "signcut.tool_dxf_export",
            Bridges => "signcut.tool_bridges",
            ReferenceScale => "signcut.tool_reference_scale",
            HotWire => "signcut.tool_hot_wire",
            CutOrder => "signcut.tool_cut_order",
            FingerJointBox => "signcut.tool_finger_joint_box",
            FingerJoints => "signcut.tool_finger_joints",
            DoorPanels => "signcut.tool_door_panels",
            AlignDistribute => "signcut.tool_align_distribute",
            SetSize => "signcut.tool_set_size",
            Replace => "signcut.tool_replace",
        }
    }

    /// The toolbar button the tool is under.
    #[cfg(test)]
    pub fn group(self) -> Group {
        Group::ALL
            .into_iter()
            .find(|g| g.tools().contains(&self))
            .unwrap_or(Group::Panel)
    }

    /// Every tool, in toolbar order.
    #[cfg(test)]
    pub fn all() -> impl Iterator<Item = ToolId> {
        Group::ALL
            .into_iter()
            .flat_map(|g| g.tools().iter().copied())
    }

    pub fn title(self) -> String {
        tr(self.key())
    }
}

/// The open tool windows, in the order they were opened.
#[derive(Debug, Clone, Default)]
pub struct Windows {
    pub open: Vec<ToolId>,
}

impl Windows {
    pub fn is_open(&self, t: ToolId) -> bool {
        self.open.contains(&t)
    }

    pub fn close(&mut self, t: ToolId) {
        self.open.retain(|o| *o != t);
    }
}

impl App {
    /// Choose a tool from the Sign & Cut toolbar or menu: open its window
    /// (or bring it forward), or run what TraceDraw already has for it.
    pub fn signcut_tool(&mut self, t: ToolId) {
        use crate::app::DockerTab;
        use crate::ui::dialogs::Dialog;
        match t {
            ToolId::SeparateCurves => self.break_apart(),
            ToolId::Grommets => self.dialog = Dialog::BorderGrommet(Default::default()),
            ToolId::VariableData => self.dialog = Dialog::PrintMerge(Default::default()),
            ToolId::JoinCurves => {
                self.show_dockers = true;
                self.docker_tab = DockerTab::JoinCurves;
            }
            ToolId::Corners => {
                self.show_dockers = true;
                self.docker_tab = DockerTab::Corners;
            }
            ToolId::AlignDistribute => {
                self.show_dockers = true;
                self.docker_tab = DockerTab::AlignDistribute;
            }
            ToolId::DxfExport => self.export(),
            _ => {
                // The sign making section opens at the tool asked for.
                use crate::signcut_sign::Tool as S;
                let sign = match t {
                    ToolId::LedModules => Some(S::Led),
                    ToolId::NeonTubes => Some(S::Neon),
                    ToolId::SignBox => Some(S::SignBox),
                    ToolId::LetterReturns => Some(S::Returns),
                    ToolId::AcrylicMould => Some(S::Mould),
                    ToolId::SmartDivide => Some(S::Tiles),
                    ToolId::Carrier => Some(S::Carrier),
                    _ => None,
                };
                if sign.is_some() {
                    self.sc_sign.open = sign;
                }
                if !self.sc_windows.is_open(t) {
                    self.sc_windows.open.push(t);
                }
                self.sc_windows_raise = Some(t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_is_on_exactly_one_button() {
        let all: Vec<ToolId> = ToolId::all().collect();
        for t in &all {
            assert_eq!(all.iter().filter(|x| *x == t).count(), 1, "{t:?}");
            assert_ne!(t.group(), Group::Panel);
        }
        assert_eq!(all.len(), 44);
    }

    #[test]
    fn every_tool_and_group_has_a_name() {
        for t in ToolId::all() {
            assert!(crate::i18n::has_english(t.key()), "{}", t.key());
        }
        for g in Group::ALL {
            assert!(crate::i18n::has_english(g.key()), "{}", g.key());
        }
    }

    #[test]
    fn choosing_a_tool_opens_its_window_once() {
        let mut app = App::headless();
        app.signcut_tool(ToolId::ShapeNesting);
        app.signcut_tool(ToolId::ShapeNesting);
        assert_eq!(app.sc_windows.open, vec![ToolId::ShapeNesting]);
        app.signcut_tool(ToolId::AlignDistribute);
        assert!(!app.sc_windows.is_open(ToolId::AlignDistribute));
        assert_eq!(app.docker_tab, crate::app::DockerTab::AlignDistribute);
    }
}
