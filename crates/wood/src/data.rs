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
            other: BTreeMap::new(),
        }
    }
}
