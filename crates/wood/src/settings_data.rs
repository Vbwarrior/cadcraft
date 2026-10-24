//! WoodCraft's own settings: drafting tolerances, build-size limits, Common size presets, and
//! the drop-down lists (joint type, edge profile, glue, screw, hardware, end treatment).
//! `plans/WoodCraft_Implementation_Plan.md` Phase 4; defaults and seed data from
//! `plans/WoodCraft_Decisions.md` and the `Lists` tab of `plans/WoodCraft_Property_Names.xlsx`.

use serde::{Deserialize, Serialize};

/// The whole settings file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WoodSettings {
    /// Degrees. An edge is flat only when exactly horizontal or vertical; any angle beyond this
    /// tolerance is an angle. Default 0: any angle at all is an angle.
    #[serde(rename = "straightnessTolerance")]
    pub straightness_tolerance: f64,
    /// `"in"` or `"mm"`.
    pub units: String,
    #[serde(rename = "minimumJointSpacing")]
    pub minimum_joint_spacing: f64,
    /// Fit tolerance by joint type name (the `Joint type` list).
    #[serde(rename = "jointToleranceByType")]
    pub joint_tolerance_by_type: std::collections::BTreeMap<String, f64>,
    /// The three build-size limits (smallest, middle, largest). The check that uses them is
    /// skipped until all three are set.
    #[serde(rename = "buildSizeLimits")]
    pub build_size_limits: BuildSizeLimits,
    /// Block export while fastener warnings exist, instead of refusing it. Default off.
    #[serde(rename = "blockExportOnFastenerWarnings")]
    pub block_export_on_fastener_warnings: bool,
    /// Minutes between background git commits of a dirty drawing. Default 1.
    #[serde(rename = "gitTimerMinutes")]
    pub git_timer_minutes: f64,
    /// Common size presets, by name.
    #[serde(rename = "commonSizePresets")]
    pub common_size_presets: Vec<CommonSizePreset>,
    /// The drop-down lists.
    pub lists: WoodLists,
}

impl Default for WoodSettings {
    fn default() -> Self {
        WoodSettings {
            straightness_tolerance: 0.0,
            units: "in".into(),
            minimum_joint_spacing: 0.0,
            joint_tolerance_by_type: std::collections::BTreeMap::new(),
            build_size_limits: BuildSizeLimits::default(),
            block_export_on_fastener_warnings: false,
            git_timer_minutes: 1.0,
            common_size_presets: seed_common_sizes(),
            lists: WoodLists::seeded(),
        }
    }
}

/// Smallest, middle and largest build-size limits. `None` until the user sets it; the
/// build-size check is skipped until all three are `Some`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BuildSizeLimits {
    pub smallest: Option<f64>,
    pub middle: Option<f64>,
    pub largest: Option<f64>,
}

impl BuildSizeLimits {
    /// All three limits are set, so the build-size check applies.
    pub fn is_active(&self) -> bool {
        self.smallest.is_some() && self.middle.is_some() && self.largest.is_some()
    }
}

/// One Common size preset: a name plus the width and thickness it fills in on a new part.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommonSizePreset {
    pub name: String,
    pub width: f64,
    pub thickness: f64,
}

impl CommonSizePreset {
    fn new(name: &str, width: f64, thickness: f64) -> Self {
        CommonSizePreset { name: name.into(), width, thickness }
    }
}

/// Standard lumber sizes, 1x2 through 2x8, at their real (not nominal) inch dimensions.
fn seed_common_sizes() -> Vec<CommonSizePreset> {
    vec![
        CommonSizePreset::new("1x2", 1.5, 0.75),
        CommonSizePreset::new("1x3", 2.5, 0.75),
        CommonSizePreset::new("1x4", 3.5, 0.75),
        CommonSizePreset::new("1x6", 5.5, 0.75),
        CommonSizePreset::new("1x8", 7.25, 0.75),
        CommonSizePreset::new("2x2", 1.5, 1.5),
        CommonSizePreset::new("2x3", 2.5, 1.5),
        CommonSizePreset::new("2x4", 3.5, 1.5),
        CommonSizePreset::new("2x6", 5.5, 1.5),
        CommonSizePreset::new("2x8", 7.25, 1.5),
    ]
}

/// The managed drop-down lists. Each is a plain, ordered list of text entries the user may add
/// to, rename within, or remove from; seeded from the `Lists` tab of
/// `plans/WoodCraft_Property_Names.xlsx`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WoodLists {
    #[serde(rename = "Joint type")]
    pub joint_type: Vec<String>,
    #[serde(rename = "Edge profile")]
    pub edge_profile: Vec<String>,
    #[serde(rename = "Glue type")]
    pub glue_type: Vec<String>,
    #[serde(rename = "Screw type")]
    pub screw_type: Vec<String>,
    #[serde(rename = "Hardware type")]
    pub hardware_type: Vec<String>,
    #[serde(rename = "End treatment")]
    pub end_treatment: Vec<String>,
}

impl WoodLists {
    fn seeded() -> Self {
        let strings = |items: &[&str]| items.iter().map(|s| s.to_string()).collect();
        WoodLists {
            joint_type: strings(&["Tenon", "Dovetail", "Split tenon", "Tapered", "Tongue", "Groove", "Half-lap", "Half-lap pin", "Dado", "Rabbet"]),
            edge_profile: strings(&["Round-over", "Chamfer", "Cove", "Bead", "Cove and bead", "Ogee", "Roman ogee", "Classical", "Bullnose"]),
            glue_type: strings(&["Wood glue", "CA", "Epoxy"]),
            screw_type: strings(&["Phillips", "Flat head"]),
            hardware_type: strings(&["Knob", "Handle", "Hinge", "Latch", "Drawer slide"]),
            end_treatment: strings(&[
                "None",
                "Tenon",
                "Dovetail",
                "Split tenon",
                "Tapered",
                "Tongue",
                "Groove",
                "Mitered",
                "Half-lap",
                "Half-lap pin",
            ]),
        }
    }
}

/// Which list `name` names, for the add/rename/delete commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListName {
    JointType,
    EdgeProfile,
    GlueType,
    ScrewType,
    HardwareType,
    EndTreatment,
}

impl ListName {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Joint type" => Some(ListName::JointType),
            "Edge profile" => Some(ListName::EdgeProfile),
            "Glue type" => Some(ListName::GlueType),
            "Screw type" => Some(ListName::ScrewType),
            "Hardware type" => Some(ListName::HardwareType),
            "End treatment" => Some(ListName::EndTreatment),
            _ => None,
        }
    }

    pub fn get(self, lists: &WoodLists) -> &Vec<String> {
        match self {
            ListName::JointType => &lists.joint_type,
            ListName::EdgeProfile => &lists.edge_profile,
            ListName::GlueType => &lists.glue_type,
            ListName::ScrewType => &lists.screw_type,
            ListName::HardwareType => &lists.hardware_type,
            ListName::EndTreatment => &lists.end_treatment,
        }
    }

    pub fn get_mut(self, lists: &mut WoodLists) -> &mut Vec<String> {
        match self {
            ListName::JointType => &mut lists.joint_type,
            ListName::EdgeProfile => &mut lists.edge_profile,
            ListName::GlueType => &mut lists.glue_type,
            ListName::ScrewType => &mut lists.screw_type,
            ListName::HardwareType => &mut lists.hardware_type,
            ListName::EndTreatment => &mut lists.end_treatment,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_match_the_decided_defaults() {
        let s = WoodSettings::default();
        assert_eq!(s.straightness_tolerance, 0.0);
        assert_eq!(s.units, "in");
        assert!(!s.block_export_on_fastener_warnings);
        assert_eq!(s.git_timer_minutes, 1.0);
        assert!(!s.build_size_limits.is_active());
        assert_eq!(s.common_size_presets.len(), 10);
        assert_eq!(s.common_size_presets[0], CommonSizePreset::new("1x2", 1.5, 0.75));
    }

    #[test]
    fn build_size_limits_are_active_only_once_all_three_are_set() {
        let mut b = BuildSizeLimits::default();
        assert!(!b.is_active());
        b.smallest = Some(1.0);
        b.middle = Some(2.0);
        assert!(!b.is_active());
        b.largest = Some(3.0);
        assert!(b.is_active());
    }

    #[test]
    fn list_name_parses_the_six_exact_sheet_names() {
        for (text, expected) in [
            ("Joint type", ListName::JointType),
            ("Edge profile", ListName::EdgeProfile),
            ("Glue type", ListName::GlueType),
            ("Screw type", ListName::ScrewType),
            ("Hardware type", ListName::HardwareType),
            ("End treatment", ListName::EndTreatment),
        ] {
            assert_eq!(ListName::parse(text), Some(expected));
        }
        assert_eq!(ListName::parse("Nonsense"), None);
    }
}
