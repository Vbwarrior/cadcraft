//! WoodCraft part data. Pure data and storage: no commands and no UI.
//!
//! The part properties live as JSON in an XRECORD under the key `WOODCRAFT_PARTS` of the
//! drawing's named object dictionary. [`store`] reads and writes that XRECORD through the
//! drawing's `foreign` data, so undo, redo, copy-on-write and Save carry it with the geometry.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod data;
pub mod edges;
pub mod export;
pub mod guid;
pub mod settings_data;
pub mod settings_store;
pub mod store;
pub mod warnings;

pub use data::{CommonSize, Part, WoodData};
pub use edges::{Edge, EdgeError, Side};
pub use export::{Outline, OutlinePoint, ProjectDocument, ProjectInfo, part_entry, scad_for_part};
pub use settings_data::{BuildSizeLimits, CommonSizePreset, ListName, WoodLists, WoodSettings};
pub use settings_store::SettingsError;
pub use store::{KEY, WoodError};

/// The wood file extension (the content is DXF).
pub const EXTENSION: &str = "wood";
