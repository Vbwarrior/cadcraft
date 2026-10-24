//! The part properties as the JSON document A of `WoodCraft_Part_JSON_Draft.md`.
//! Keys are the full property names from the property sheet, spelled exactly as in the sheet.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The whole document stored in the drawing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WoodData {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    /// `"in"` or `"mm"` for the whole document.
    pub units: String,
    #[serde(rename = "propertyListVersion")]
    pub property_list_version: u32,
    /// Parts by GUID. The GUID is also the block name in the drawing.
    pub parts: BTreeMap<String, Part>,
}

impl Default for WoodData {
    fn default() -> Self {
        WoodData { schema_version: 1, units: "in".into(), property_list_version: 1, parts: BTreeMap::new() }
    }
}

/// Which side of the part a property belongs to: A is always the appearance face, B the back.
/// For a hole, this is the face it is drilled from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Face {
    A,
    B,
}

/// The orientation mark (a capital V, placed on one face, its point touching a named edge:
/// "this side up, V points at the reference edge"). A single value; left out of the JSON
/// entirely at its default (face A, top-left corner, pointing at the top edge) per
/// `WoodCraft_Part_JSON_Draft.md`'s "stored only when moved."
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrientationMark {
    #[serde(rename = "Face")]
    pub face: Face,
    #[serde(rename = "Edge")]
    pub edge: String,
}

/// One reference mark: a saved point tied to a part, located from a named end and edge, with a
/// label. Zero or more per part; used by the assembler to snap and listed for transfer onto the
/// wood.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceMark {
    #[serde(rename = "Face")]
    pub face: Face,
    #[serde(rename = "X")]
    pub x: f64,
    #[serde(rename = "Y")]
    pub y: f64,
    #[serde(rename = "Label")]
    pub label: String,
}

/// One position in a hole or mark entry's `At` list: `WoodCraft_Part_JSON_Draft.md`'s "same
/// shape, many places" rule — entries that differ only by position share one entry, with an
/// `At` list of positions, instead of repeating the shared values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct At {
    #[serde(rename = "X")]
    pub x: f64,
    #[serde(rename = "Y")]
    pub y: f64,
}

/// One hole entry: the values shared by every hole in its `at` list (face, diameter, and the
/// kind-specific extra fields the sheet names), plus the positions themselves. A kind that has
/// no depth (`Hole-Through`) or no extras leaves those fields out, per "a property that is not
/// used is left out."
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoleEntry {
    #[serde(rename = "Face")]
    pub face: Face,
    #[serde(rename = "Dia")]
    pub dia: f64,
    #[serde(rename = "Depth", default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f64>,
    /// `Hole-Counterbore`: bore diameter. `Hole-Countersink`: sink diameter.
    /// `Hole-Pocket-Screw`: drill angle (degrees).
    #[serde(rename = "Extra 1", default, skip_serializing_if = "Option::is_none")]
    pub extra1: Option<f64>,
    /// `Hole-Counterbore`: bore depth. `Hole-Countersink`: angle (degrees).
    #[serde(rename = "Extra 2", default, skip_serializing_if = "Option::is_none")]
    pub extra2: Option<f64>,
    #[serde(rename = "At")]
    pub at: Vec<At>,
}

/// One cut entry: the values shared by every cut in its `at` list, plus the positions
/// themselves. Which fields a kind actually uses varies (`Cut-Rabbet` has no position at all —
/// "along an edge" — and `Cut-Slot`/`Cut-Notch` have no depth), so every field here is optional
/// and the command layer enforces which ones a given kind requires; a field a kind does not use
/// is left out, per "a property that is not used is left out."
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CutEntry {
    /// A face letter (`A`/`B`) or a named edge (as `woodedges` names them, e.g. `"Top"`).
    #[serde(rename = "Face / Edge")]
    pub face_edge: String,
    #[serde(rename = "Length")]
    pub length: f64,
    #[serde(rename = "Width")]
    pub width: f64,
    #[serde(rename = "Depth", default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f64>,
    #[serde(rename = "Angle", default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<f64>,
    /// Empty for `Cut-Rabbet` ("along an edge": no start position).
    #[serde(rename = "At", default, skip_serializing_if = "Vec::is_empty")]
    pub at: Vec<At>,
}

/// One Mortise, Tenon or Biscuit entry: the values shared by every instance in its `at` list,
/// plus the positions themselves. `Tolerance` (the snug-fit allowance) applies to Mortise and
/// Tenon only; Biscuit has neither `Depth` nor `Tolerance`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JoineryEntry {
    #[serde(rename = "Face / Edge")]
    pub face_edge: String,
    #[serde(rename = "Length")]
    pub length: f64,
    #[serde(rename = "Width")]
    pub width: f64,
    #[serde(rename = "Depth", default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f64>,
    #[serde(rename = "Tolerance", default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    #[serde(rename = "At")]
    pub at: Vec<At>,
}

/// A `Joint type` entry: which joint a part's end or edge makes, and to which other part — a
/// selector, not a positioned feature, so it has no `At` list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointTypeEntry {
    #[serde(rename = "Joint code")]
    pub joint_code: String,
    #[serde(rename = "Mating Part ID")]
    pub mating_part_id: String,
    #[serde(rename = "Tolerance", default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
}

/// An `End treatment` entry: one end or edge's chosen treatment (from the WoodCraft settings
/// list seeded in Phase 4). Each board has End A and End B; a treatment never changes the
/// part's total length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndTreatmentEntry {
    #[serde(rename = "Edge / End")]
    pub edge_end: String,
    #[serde(rename = "Profile")]
    pub profile: String,
}

/// An `Edge profile` entry: one edge or end's chosen router profile (from the WoodCraft
/// settings list seeded in Phase 4) and its size (for example a 3/4" round-over).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeProfileEntry {
    #[serde(rename = "Edge / End")]
    pub edge_end: String,
    #[serde(rename = "Profile")]
    pub profile: String,
    #[serde(rename = "Size")]
    pub size: f64,
}

/// A `Bevel` or `Miter` entry: one edge or end's angle. Both properties share this shape; only
/// the key they are stored under differs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AngleEntry {
    #[serde(rename = "Edge / End")]
    pub edge_end: String,
    #[serde(rename = "Angle")]
    pub angle: f64,
}

/// A `Glue` entry: which face or edge gets glue, and of what type (from the WoodCraft settings'
/// Glue type list). Not a positioned feature — no `X`/`Y`, no `At`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlueEntry {
    #[serde(rename = "Face / Edge")]
    pub face_edge: String,
    #[serde(rename = "Type")]
    pub glue_type: String,
}

/// A `Screw` or `Dowel` entry: the values shared by every instance in its `at` list, plus the
/// positions themselves. `Type` applies to Screw only (from the settings' Screw type list);
/// Dowel has no type, only a `Size`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FastenerEntry {
    #[serde(rename = "Face / Edge")]
    pub face_edge: String,
    #[serde(rename = "Type", default, skip_serializing_if = "Option::is_none")]
    pub fastener_type: Option<String>,
    #[serde(rename = "Size")]
    pub size: f64,
    #[serde(rename = "At")]
    pub at: Vec<At>,
}

/// A `Nail` entry: defined by diameter and length (`Wood_Workflow_Scope.md`: "Nails are defined
/// by diameter and length"), not by a settings-list type or a single `Size`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NailEntry {
    #[serde(rename = "Face / Edge")]
    pub face_edge: String,
    #[serde(rename = "Diameter")]
    pub diameter: f64,
    #[serde(rename = "Length")]
    pub length: f64,
    #[serde(rename = "At")]
    pub at: Vec<At>,
}

/// A `Hinge`, `Drawer slide` or `Drawer Pull` entry: a hardware marker's face, type (from the
/// settings' Hardware type list) and position. `Size` applies to Drawer slide only; Hinge and
/// Drawer Pull are "location only."
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HardwareEntry {
    #[serde(rename = "Face")]
    pub face: Face,
    #[serde(rename = "Type")]
    pub hardware_type: String,
    #[serde(rename = "Size", default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(rename = "At")]
    pub at: Vec<At>,
}

/// One part. The properties the first slice knows are fields; every other property is kept in
/// `other` exactly as read, so a later version of the property list is never lost on save.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    /// The display name the user gave it.
    pub name: String,
    /// Goes up by one each time the part's entry changes.
    pub revision: u32,
    #[serde(rename = "Thickness")]
    pub thickness: f64,
    #[serde(rename = "Grain direction", default, skip_serializing_if = "Option::is_none")]
    pub grain_direction: Option<f64>,
    #[serde(rename = "Common size", default, skip_serializing_if = "Option::is_none")]
    pub common_size: Option<CommonSize>,
    #[serde(rename = "Orientation mark", default, skip_serializing_if = "Option::is_none")]
    pub orientation_mark: Option<OrientationMark>,
    #[serde(rename = "Reference mark", default, skip_serializing_if = "Vec::is_empty")]
    pub reference_marks: Vec<ReferenceMark>,
    #[serde(rename = "Hole-Through", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_through: Vec<HoleEntry>,
    #[serde(rename = "Hole-Blind", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_blind: Vec<HoleEntry>,
    #[serde(rename = "Hole-Counterbore", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_counterbore: Vec<HoleEntry>,
    #[serde(rename = "Hole-Countersink", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_countersink: Vec<HoleEntry>,
    #[serde(rename = "Hole-Pocket-Screw", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_pocket_screw: Vec<HoleEntry>,
    #[serde(rename = "Hole-Shelf-Pin", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_shelf_pin: Vec<HoleEntry>,
    #[serde(rename = "Hole-Dowel", default, skip_serializing_if = "Vec::is_empty")]
    pub hole_dowel: Vec<HoleEntry>,
    #[serde(rename = "Cut-Dado", default, skip_serializing_if = "Vec::is_empty")]
    pub cut_dado: Vec<CutEntry>,
    #[serde(rename = "Cut-Groove", default, skip_serializing_if = "Vec::is_empty")]
    pub cut_groove: Vec<CutEntry>,
    #[serde(rename = "Cut-Rabbet", default, skip_serializing_if = "Vec::is_empty")]
    pub cut_rabbet: Vec<CutEntry>,
    #[serde(rename = "Cut-Slot", default, skip_serializing_if = "Vec::is_empty")]
    pub cut_slot: Vec<CutEntry>,
    #[serde(rename = "Cut-Notch", default, skip_serializing_if = "Vec::is_empty")]
    pub cut_notch: Vec<CutEntry>,
    #[serde(rename = "Cut-Pocket", default, skip_serializing_if = "Vec::is_empty")]
    pub cut_pocket: Vec<CutEntry>,
    #[serde(rename = "Mortise", default, skip_serializing_if = "Vec::is_empty")]
    pub mortise: Vec<JoineryEntry>,
    #[serde(rename = "Tenon", default, skip_serializing_if = "Vec::is_empty")]
    pub tenon: Vec<JoineryEntry>,
    #[serde(rename = "Biscuit", default, skip_serializing_if = "Vec::is_empty")]
    pub biscuit: Vec<JoineryEntry>,
    #[serde(rename = "Joint type", default, skip_serializing_if = "Vec::is_empty")]
    pub joint_type: Vec<JointTypeEntry>,
    #[serde(rename = "End treatment", default, skip_serializing_if = "Vec::is_empty")]
    pub end_treatment: Vec<EndTreatmentEntry>,
    #[serde(rename = "Edge profile", default, skip_serializing_if = "Vec::is_empty")]
    pub edge_profile: Vec<EdgeProfileEntry>,
    #[serde(rename = "Bevel", default, skip_serializing_if = "Vec::is_empty")]
    pub bevel: Vec<AngleEntry>,
    #[serde(rename = "Miter", default, skip_serializing_if = "Vec::is_empty")]
    pub miter: Vec<AngleEntry>,
    #[serde(rename = "Glue", default, skip_serializing_if = "Vec::is_empty")]
    pub glue: Vec<GlueEntry>,
    #[serde(rename = "Screw", default, skip_serializing_if = "Vec::is_empty")]
    pub screw: Vec<FastenerEntry>,
    #[serde(rename = "Nail", default, skip_serializing_if = "Vec::is_empty")]
    pub nail: Vec<NailEntry>,
    #[serde(rename = "Dowel", default, skip_serializing_if = "Vec::is_empty")]
    pub dowel: Vec<FastenerEntry>,
    #[serde(rename = "Hinge", default, skip_serializing_if = "Vec::is_empty")]
    pub hinge: Vec<HardwareEntry>,
    #[serde(rename = "Drawer slide", default, skip_serializing_if = "Vec::is_empty")]
    pub drawer_slide: Vec<HardwareEntry>,
    #[serde(rename = "Drawer Pull", default, skip_serializing_if = "Vec::is_empty")]
    pub drawer_pull: Vec<HardwareEntry>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// The stock size name (for the bill of materials in WorkShop Pro) with its width and thickness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommonSize {
    #[serde(rename = "Preset name")]
    pub preset_name: String,
    #[serde(rename = "Width")]
    pub width: f64,
    #[serde(rename = "Thickness")]
    pub thickness: f64,
}

impl Part {
    pub fn new(name: &str, thickness: f64) -> Part {
        Part {
            name: name.to_string(),
            revision: 1,
            thickness,
            grain_direction: None,
            common_size: None,
            orientation_mark: None,
            reference_marks: Vec::new(),
            hole_through: Vec::new(),
            hole_blind: Vec::new(),
            hole_counterbore: Vec::new(),
            hole_countersink: Vec::new(),
            hole_pocket_screw: Vec::new(),
            hole_shelf_pin: Vec::new(),
            hole_dowel: Vec::new(),
            cut_dado: Vec::new(),
            cut_groove: Vec::new(),
            cut_rabbet: Vec::new(),
            cut_slot: Vec::new(),
            cut_notch: Vec::new(),
            cut_pocket: Vec::new(),
            mortise: Vec::new(),
            tenon: Vec::new(),
            biscuit: Vec::new(),
            joint_type: Vec::new(),
            end_treatment: Vec::new(),
            edge_profile: Vec::new(),
            bevel: Vec::new(),
            miter: Vec::new(),
            glue: Vec::new(),
            screw: Vec::new(),
            nail: Vec::new(),
            dowel: Vec::new(),
            hinge: Vec::new(),
            drawer_slide: Vec::new(),
            drawer_pull: Vec::new(),
            other: BTreeMap::new(),
        }
    }
}
