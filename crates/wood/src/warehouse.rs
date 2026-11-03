//! WoodCraft Phase 10a: reads the Model Warehouse folder tree. WoodCraft only reads this folder
//! (`plans/WoodCraft_Decisions.md` item 12) — a separate C# / WPF importer app writes it. Every
//! subfolder of the warehouse root is a category; a model's own folder holds its 3D file
//! (`.dae` preferred, `.obj` fallback), an icon (`icon.png`, made by Chris, optional) and a
//! `model.json` describing its flat top-view outline, per `plans/WoodCraft_Decisions.md` item 25
//! (the warehouse model JSON this crate reads):
//!
//! ```json
//! { "schemaVersion": 1, "name": "SOSS 100 invisible hinge",
//!   "outline": [[0,0,0],[2,0,0],[2,1,0],[0,1,0]] }
//! ```
//!
//! `outline` is the same `[x, y, bulge]` closed-path convention `crate::export::Outline` already
//! writes for a part (a bulge of 0 is a straight segment to the next point). The 3D file and the
//! icon are found by file name in the model's own folder, not referenced in the JSON — one less
//! thing that can point at a missing file. All paths read here are treated as hostile (file
//! system contents, not program input, but still outside our control): sizes are capped,
//! traversal is one level at a time with no recursion past the two fixed levels (root ->
//! category -> model), and nothing here panics on a malformed or oversized file; it is skipped
//! or reported as an error for that one entry only.

use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::export::OutlinePoint;

/// A `model.json` is kept to this many bytes. Generous for a top-view outline, small enough that
/// a corrupt or hostile file cannot stall a folder scan.
pub const MAX_MODEL_JSON_BYTES: u64 = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum WarehouseError {
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    #[error("{0}: the model file is {1} bytes, over the {MAX_MODEL_JSON_BYTES} byte limit")]
    TooLarge(String, u64),
    #[error("{0}: invalid model JSON: {1}")]
    Json(String, serde_json::Error),
    #[error("the warehouse folder does not exist: {0}")]
    NoSuchFolder(String),
}

/// One category: a subfolder of the warehouse root, or of another category.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    /// The folder name (not a full path).
    pub name: String,
    /// This category's path, relative to the warehouse root, with `/` separators.
    pub path: String,
}

/// One model: a subfolder holding `model.json` plus its 3D file and optional icon.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelSummary {
    /// The model's display name, from `model.json`.
    pub name: String,
    /// This model's folder, relative to the warehouse root, with `/` separators. Also the id
    /// used to place it and to look it up again later.
    pub path: String,
    /// The icon file's path relative to the warehouse root, if `icon.png` exists in the
    /// model's folder.
    pub icon: Option<String>,
    /// The outline's axis-aligned width and height, in the model's own units.
    pub size: (f64, f64),
}

/// A model's full detail: everything in [`ModelSummary`] plus the outline itself and the 3D
/// file's path, for placing it in a drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDetail {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    pub outline: Vec<OutlinePoint>,
    /// The 3D file's path relative to the warehouse root: the model folder's own `.dae` file if
    /// one exists, else its `.obj` file, else `None`.
    pub model_file: Option<String>,
}

#[derive(Deserialize)]
struct ModelJson {
    #[allow(dead_code)]
    #[serde(rename = "schemaVersion", default)]
    schema_version: u32,
    name: String,
    outline: Vec<OutlinePoint>,
}

/// Every immediate subfolder of `root` (or of `root` joined with `under`, when given), as
/// categories. Hidden folders (starting with `.`) are skipped. Entries that cannot be read (a
/// permissions error, a name that is not valid UTF-8) are skipped rather than failing the whole
/// listing — one bad entry should not hide the rest of the warehouse.
pub fn categories(root: &Path, under: Option<&str>) -> Result<Vec<Category>, WarehouseError> {
    let dir = match under {
        Some(rel) => root.join(safe_relative(rel)),
        None => root.to_path_buf(),
    };
    if !dir.is_dir() {
        return Err(WarehouseError::NoSuchFolder(dir.display().to_string()));
    }
    let mut out = Vec::new();
    for entry in read_dir_sorted(&dir)? {
        if !entry.path.is_dir() {
            continue;
        }
        let Some(name) = entry.path.file_name().and_then(OsStr::to_str) else { continue };
        if name.starts_with('.') {
            continue;
        }
        let rel = match under {
            Some(rel) => format!("{rel}/{name}"),
            None => name.to_string(),
        };
        out.push(Category { name: name.to_string(), path: rel });
    }
    Ok(out)
}

/// Every model directly inside `root`/`under` (a folder holding its own `model.json`), skipping
/// subfolders with no `model.json` (further categories) and any model whose JSON fails to read
/// or parse (reported as a `WarehouseError` entry rather than silently dropped, so a bad model
/// is visible, but does not stop the rest of the listing).
pub fn models(root: &Path, under: &str) -> Result<Vec<Result<ModelSummary, WarehouseError>>, WarehouseError> {
    let dir = root.join(safe_relative(under));
    if !dir.is_dir() {
        return Err(WarehouseError::NoSuchFolder(dir.display().to_string()));
    }
    let mut out = Vec::new();
    for entry in read_dir_sorted(&dir)? {
        if !entry.path.is_dir() {
            continue;
        }
        let model_json = entry.path.join("model.json");
        if !model_json.is_file() {
            continue;
        }
        let Some(name) = entry.path.file_name().and_then(OsStr::to_str) else { continue };
        let rel = format!("{under}/{name}");
        out.push(read_summary(&rel, &entry.path, &model_json));
    }
    Ok(out)
}

fn read_summary(rel: &str, folder: &Path, model_json: &Path) -> Result<ModelSummary, WarehouseError> {
    let json = read_model_json(model_json)?;
    let icon = folder.join("icon.png").is_file().then(|| format!("{rel}/icon.png"));
    Ok(ModelSummary { name: json.name, path: rel.to_string(), icon, size: outline_size(&json.outline) })
}

/// One model's full detail, by its warehouse-relative path (as [`models`] returns it).
pub fn model_detail(root: &Path, path: &str) -> Result<ModelDetail, WarehouseError> {
    let folder = root.join(safe_relative(path));
    let model_json = folder.join("model.json");
    let json = read_model_json(&model_json)?;
    let icon = folder.join("icon.png").is_file().then(|| format!("{path}/icon.png"));
    let model_file =
        [("model.dae", "dae"), ("model.obj", "obj")].iter().find(|(file, _)| folder.join(file).is_file()).map(|(file, _)| format!("{path}/{file}"));
    Ok(ModelDetail { name: json.name, path: path.to_string(), icon, outline: json.outline, model_file })
}

fn read_model_json(path: &Path) -> Result<ModelJson, WarehouseError> {
    let label = path.display().to_string();
    let mut file = fs::File::open(path).map_err(|e| WarehouseError::Io(label.clone(), e))?;
    let mut bytes = Vec::new();
    file.by_ref().take(MAX_MODEL_JSON_BYTES + 1).read_to_end(&mut bytes).map_err(|e| WarehouseError::Io(label.clone(), e))?;
    if bytes.len() as u64 > MAX_MODEL_JSON_BYTES {
        return Err(WarehouseError::TooLarge(label, bytes.len() as u64));
    }
    let json: ModelJson = serde_json::from_slice(&bytes).map_err(|e| WarehouseError::Json(label, e))?;
    Ok(json)
}

fn outline_size(points: &[OutlinePoint]) -> (f64, f64) {
    if points.is_empty() {
        return (0.0, 0.0);
    }
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for [x, y, _] in points {
        if x.is_finite() {
            min_x = min_x.min(*x);
            max_x = max_x.max(*x);
        }
        if y.is_finite() {
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return (0.0, 0.0);
    }
    (max_x - min_x, max_y - min_y)
}

/// A relative warehouse path, with any `..` segment or absolute/rooted piece dropped, so a
/// hostile `under`/`path` value (from a command parameter) can never walk the scan outside the
/// warehouse root.
fn safe_relative(rel: &str) -> PathBuf {
    rel.split(['/', '\\']).filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..").collect()
}

struct DirEntry {
    path: PathBuf,
}

/// `fs::read_dir`, collected and sorted by file name, so a listing is stable across platforms
/// and across runs (directory order is not guaranteed otherwise). Entries that error while being
/// read are skipped.
fn read_dir_sorted(dir: &Path) -> Result<Vec<DirEntry>, WarehouseError> {
    let read = fs::read_dir(dir).map_err(|e| WarehouseError::Io(dir.display().to_string(), e))?;
    let mut out: Vec<DirEntry> = read.filter_map(|e| e.ok()).map(|e| DirEntry { path: e.path() }).collect();
    out.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("woodcraft-warehouse-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap_or(());
        path
    }

    fn write_model(folder: &Path, name: &str, outline: &str) {
        fs::create_dir_all(folder).unwrap_or(());
        fs::write(folder.join("model.json"), format!(r#"{{"schemaVersion":1,"name":"{name}","outline":{outline}}}"#)).unwrap_or(());
    }

    #[test]
    fn categories_lists_immediate_subfolders_only() {
        let root = temp_dir("categories");
        fs::create_dir_all(root.join("Hardware/Hinges")).unwrap_or(());
        fs::create_dir_all(root.join("Hardware/Knobs")).unwrap_or(());
        fs::write(root.join("not-a-folder.txt"), "x").unwrap_or(());

        let top = categories(&root, None).unwrap();
        assert_eq!(top, vec![Category { name: "Hardware".into(), path: "Hardware".into() }]);

        let under = categories(&root, Some("Hardware")).unwrap();
        assert_eq!(
            under,
            vec![
                Category { name: "Hinges".into(), path: "Hardware/Hinges".into() },
                Category { name: "Knobs".into(), path: "Hardware/Knobs".into() },
            ]
        );
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn categories_skips_hidden_folders() {
        let root = temp_dir("hidden");
        fs::create_dir_all(root.join(".git")).unwrap_or(());
        fs::create_dir_all(root.join("Hardware")).unwrap_or(());
        assert_eq!(categories(&root, None).unwrap(), vec![Category { name: "Hardware".into(), path: "Hardware".into() }]);
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn categories_on_a_missing_folder_is_an_error() {
        let root = temp_dir("missing");
        assert!(categories(&root, Some("Nope")).is_err());
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn models_reads_the_summary_from_model_json_and_finds_the_icon() {
        let root = temp_dir("models");
        write_model(&root.join("Hardware/Hinges/soss-100"), "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");
        fs::write(root.join("Hardware/Hinges/soss-100/icon.png"), [0u8; 4]).unwrap_or(());
        write_model(&root.join("Hardware/Hinges/soss-200"), "SOSS 200", "[[0,0,0],[3,0,0],[3,1.5,0],[0,1.5,0]]");

        let list = models(&root, "Hardware/Hinges").unwrap();
        let ok: Vec<&ModelSummary> = list.iter().map(|r| r.as_ref().unwrap()).collect();
        assert_eq!(ok.len(), 2);
        assert_eq!(ok[0].name, "SOSS 100");
        assert_eq!(ok[0].path, "Hardware/Hinges/soss-100");
        assert_eq!(ok[0].icon, Some("Hardware/Hinges/soss-100/icon.png".to_string()));
        assert_eq!(ok[0].size, (2.0, 1.0));
        assert_eq!(ok[1].icon, None);
        assert_eq!(ok[1].size, (3.0, 1.5));
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn models_skips_subfolders_with_no_model_json() {
        let root = temp_dir("skip");
        fs::create_dir_all(root.join("Hardware/Hinges/not-a-model")).unwrap_or(());
        write_model(&root.join("Hardware/Hinges/soss-100"), "SOSS 100", "[[0,0,0],[1,0,0],[1,1,0],[0,1,0]]");
        let list = models(&root, "Hardware/Hinges").unwrap();
        assert_eq!(list.len(), 1);
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn a_malformed_model_json_is_reported_for_that_entry_only() {
        let root = temp_dir("malformed");
        fs::create_dir_all(root.join("Hardware/bad")).unwrap_or(());
        fs::write(root.join("Hardware/bad/model.json"), "{not json").unwrap_or(());
        write_model(&root.join("Hardware/good"), "Good", "[[0,0,0],[1,0,0],[1,1,0],[0,1,0]]");

        let list = models(&root, "Hardware").unwrap();
        assert_eq!(list.len(), 2);
        let bad = list.iter().find(|r| r.is_err()).unwrap();
        assert!(matches!(bad, Err(WarehouseError::Json(_, _))));
        let good = list.iter().find(|r| r.is_ok()).unwrap().as_ref().unwrap();
        assert_eq!(good.name, "Good");
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn an_oversized_model_json_is_refused() {
        let root = temp_dir("oversized");
        let folder = root.join("Hardware/huge");
        fs::create_dir_all(&folder).unwrap_or(());
        let filler = "x".repeat(MAX_MODEL_JSON_BYTES as usize + 1024);
        fs::write(folder.join("model.json"), format!(r#"{{"name":"Huge","outline":[],"filler":"{filler}"}}"#)).unwrap_or(());
        let list = models(&root, "Hardware").unwrap();
        assert_eq!(list.len(), 1);
        assert!(matches!(list[0], Err(WarehouseError::TooLarge(_, _))));
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn model_detail_finds_the_dae_file_over_the_obj_file() {
        let root = temp_dir("detail-dae");
        let folder = root.join("Hardware/soss-100");
        write_model(&folder, "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");
        fs::write(folder.join("model.dae"), "<dae/>").unwrap_or(());
        fs::write(folder.join("model.obj"), "o").unwrap_or(());

        let detail = model_detail(&root, "Hardware/soss-100").unwrap();
        assert_eq!(detail.name, "SOSS 100");
        assert_eq!(detail.model_file, Some("Hardware/soss-100/model.dae".to_string()));
        assert_eq!(detail.outline.len(), 4);
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn model_detail_falls_back_to_obj_when_there_is_no_dae() {
        let root = temp_dir("detail-obj");
        let folder = root.join("Hardware/knob-1");
        write_model(&folder, "Knob", "[[0,0,0],[1,0,0],[1,1,0],[0,1,0]]");
        fs::write(folder.join("model.obj"), "o").unwrap_or(());

        let detail = model_detail(&root, "Hardware/knob-1").unwrap();
        assert_eq!(detail.model_file, Some("Hardware/knob-1/model.obj".to_string()));
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn model_detail_with_no_3d_file_reports_none_rather_than_failing() {
        let root = temp_dir("detail-none");
        write_model(&root.join("Hardware/placeholder"), "Placeholder", "[]");
        let detail = model_detail(&root, "Hardware/placeholder").unwrap();
        assert_eq!(detail.model_file, None);
        fs::remove_dir_all(root).unwrap_or(());
    }

    #[test]
    fn a_dot_dot_path_cannot_escape_the_warehouse_root() {
        let root = temp_dir("escape");
        fs::create_dir_all(root.join("Hardware")).unwrap_or(());
        // Would resolve outside `root` if `..` were honored; must instead be read as a plain
        // (missing) subfolder name, never walking above `root`.
        assert!(matches!(categories(&root, Some("../../etc")), Err(WarehouseError::NoSuchFolder(_))));
        fs::remove_dir_all(root).unwrap_or(());
    }
}
