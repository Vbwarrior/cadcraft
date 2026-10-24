//! WoodCraft Phase 4: settings, Common size presets and the managed lists
//! (`plans/WoodCraft_Implementation_Plan.md` Phase 4). All global, file-backed state
//! (`%APPDATA%\WoodCraft\settings.json`, read and written by [`cadcraft_wood::settings_store`]),
//! never drawing state: no selection, no undo.

use cadcraft_wood::settings_data::ListName;
use cadcraft_wood::{CommonSizePreset, WoodSettings};
use serde_json::Value;

use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodsettingsget", "WoodCraft Settings", run_woodsettingsget)
            .params("{} → the settings file's contents")
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodsettingsset", "Set WoodCraft Settings", run_woodsettingsset)
            .params(
                "{straightnessTolerance?, units?: \"in\"|\"mm\", minimumJointSpacing?, \
                 jointToleranceByType?: {type: number}, buildSizeLimits?: {smallest?, middle?, largest?}, \
                 blockExportOnFastenerWarnings?: bool, gitTimerMinutes?} → the settings file's contents after the change",
            )
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodsizeadd", "Add Common Size Preset", run_woodsizeadd)
            .params("{name, width, thickness} → the settings file's contents after the change")
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodsizerename", "Rename Common Size Preset", run_woodsizerename)
            .params("{name, newName} → the settings file's contents after the change")
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodsizedelete", "Delete Common Size Preset", run_woodsizedelete)
            .params("{name} → the settings file's contents after the change")
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodlistadd", "Add List Entry", run_woodlistadd)
            .params(
                "{list: \"Joint type\"|\"Edge profile\"|\"Glue type\"|\"Screw type\"|\"Hardware type\"|\"End treatment\", entry} → \
                 the settings file's contents after the change",
            )
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodlistrename", "Rename List Entry", run_woodlistrename)
            .params("{list, entry, newEntry} → the settings file's contents after the change")
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodlistdelete", "Delete List Entry", run_woodlistdelete)
            .params("{list, entry} → the settings file's contents after the change")
            .enabled(always)
            .noundo(),
    ]
}

fn settings_value(s: &WoodSettings) -> Result<Value> {
    serde_json::to_value(s).map_err(|e| EngineError::Other(e.to_string()))
}

fn settings_error(cmd: &str, e: cadcraft_wood::SettingsError) -> EngineError {
    EngineError::BadParams { cmd: cmd.to_string(), msg: e.to_string() }
}

/// WOODCRAFT SETTINGS: the settings file's contents (the defaults, seeded with the Common
/// size presets and the managed lists, if no file exists yet).
fn run_woodsettingsget(_s: &mut Session, _p: &Value) -> Result<Value> {
    let settings = cadcraft_wood::settings_store::get().map_err(|e| settings_error("woodsettingsget", e))?;
    settings_value(&settings)
}

fn positive_number(cmd: &str, p: &Value, key: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(v) => {
            v.as_f64().filter(|x| x.is_finite() && *x >= 0.0).map(Some).ok_or_else(|| bad(cmd, format!("`{key}` must be a number at or above zero")))
        }
    }
}

/// SET WOODCRAFT SETTINGS: changes only the fields given; every other setting is left as it was.
fn run_woodsettingsset(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodsettingsset";
    let straightness = positive_number(cmd, p, "straightnessTolerance")?;
    let units = match str_param(p, "units") {
        None => None,
        Some("in") => Some("in"),
        Some("mm") => Some("mm"),
        Some(_) => return Err(bad(cmd, "`units` must be \"in\" or \"mm\"")),
    };
    let min_joint_spacing = positive_number(cmd, p, "minimumJointSpacing")?;
    let joint_tolerance_by_type = match p.get("jointToleranceByType") {
        None => None,
        Some(Value::Object(map)) => {
            let mut out = std::collections::BTreeMap::new();
            for (k, v) in map {
                let n = v
                    .as_f64()
                    .filter(|x| x.is_finite() && *x >= 0.0)
                    .ok_or_else(|| bad(cmd, "`jointToleranceByType` values must be numbers at or above zero"))?;
                out.insert(k.clone(), n);
            }
            Some(out)
        }
        Some(_) => return Err(bad(cmd, "`jointToleranceByType` must be an object of joint type name to number")),
    };
    let build_size_limits = match p.get("buildSizeLimits") {
        None => None,
        Some(v @ Value::Object(_)) => {
            let get = |key: &str| -> Result<Option<f64>> { positive_number(cmd, v, key) };
            Some((get("smallest")?, get("middle")?, get("largest")?))
        }
        Some(_) => return Err(bad(cmd, "`buildSizeLimits` must be an object")),
    };
    let block_export = p
        .get("blockExportOnFastenerWarnings")
        .map(|v| v.as_bool().ok_or_else(|| bad(cmd, "`blockExportOnFastenerWarnings` must be a bool")))
        .transpose()?;
    let git_timer = positive_number(cmd, p, "gitTimerMinutes")?;

    let settings = cadcraft_wood::settings_store::update(|w| {
        if let Some(v) = straightness {
            w.straightness_tolerance = v;
        }
        if let Some(v) = units {
            w.units = v.to_string();
        }
        if let Some(v) = min_joint_spacing {
            w.minimum_joint_spacing = v;
        }
        if let Some(v) = joint_tolerance_by_type {
            w.joint_tolerance_by_type = v;
        }
        if let Some((smallest, middle, largest)) = build_size_limits {
            if smallest.is_some() {
                w.build_size_limits.smallest = smallest;
            }
            if middle.is_some() {
                w.build_size_limits.middle = middle;
            }
            if largest.is_some() {
                w.build_size_limits.largest = largest;
            }
        }
        if let Some(v) = block_export {
            w.block_export_on_fastener_warnings = v;
        }
        if let Some(v) = git_timer {
            w.git_timer_minutes = v;
        }
        w.clone()
    })
    .map_err(|e| settings_error(cmd, e))?;
    settings_value(&settings)
}

/// ADD COMMON SIZE PRESET: refused if the name is already used.
fn run_woodsizeadd(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodsizeadd";
    let name = str_param(p, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad(cmd, "`name` is required"))?.to_string();
    let width = size_param(cmd, p, "width", false)?.ok_or_else(|| bad(cmd, "`width` (number above zero) is required"))?;
    let thickness = size_param(cmd, p, "thickness", false)?.ok_or_else(|| bad(cmd, "`thickness` (number above zero) is required"))?;

    let settings = cadcraft_wood::settings_store::update(|w| {
        if w.common_size_presets.iter().any(|preset| preset.name.eq_ignore_ascii_case(&name)) {
            return Err(bad(cmd, "a Common size preset with that name already exists"));
        }
        w.common_size_presets.push(CommonSizePreset { name: name.clone(), width, thickness });
        Ok(w.clone())
    })
    .map_err(|e| settings_error(cmd, e))??;
    settings_value(&settings)
}

/// RENAME COMMON SIZE PRESET: refused if `name` does not exist, or `newName` is already used.
fn run_woodsizerename(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodsizerename";
    let name = str_param(p, "name").ok_or_else(|| bad(cmd, "`name` is required"))?.to_string();
    let new_name = str_param(p, "newName").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad(cmd, "`newName` is required"))?.to_string();

    let settings = cadcraft_wood::settings_store::update(|w| {
        if !name.eq_ignore_ascii_case(&new_name) && w.common_size_presets.iter().any(|preset| preset.name.eq_ignore_ascii_case(&new_name)) {
            return Err(bad(cmd, "a Common size preset with that name already exists"));
        }
        let preset = w
            .common_size_presets
            .iter_mut()
            .find(|preset| preset.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| bad(cmd, "no Common size preset has that name"))?;
        preset.name = new_name.clone();
        Ok(w.clone())
    })
    .map_err(|e| settings_error(cmd, e))??;
    settings_value(&settings)
}

/// DELETE COMMON SIZE PRESET.
fn run_woodsizedelete(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodsizedelete";
    let name = str_param(p, "name").ok_or_else(|| bad(cmd, "`name` is required"))?.to_string();

    let settings = cadcraft_wood::settings_store::update(|w| {
        let before = w.common_size_presets.len();
        w.common_size_presets.retain(|preset| !preset.name.eq_ignore_ascii_case(&name));
        if w.common_size_presets.len() == before {
            return Err(bad(cmd, "no Common size preset has that name"));
        }
        Ok(w.clone())
    })
    .map_err(|e| settings_error(cmd, e))??;
    settings_value(&settings)
}

fn list_param(cmd: &str, p: &Value) -> Result<ListName> {
    let text = str_param(p, "list").ok_or_else(|| bad(cmd, "`list` is required"))?;
    ListName::parse(text).ok_or_else(|| {
        bad(cmd, "`list` must be one of \"Joint type\", \"Edge profile\", \"Glue type\", \"Screw type\", \"Hardware type\", \"End treatment\"")
    })
}

/// ADD LIST ENTRY: refused if the entry already exists in that list.
fn run_woodlistadd(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodlistadd";
    let list = list_param(cmd, p)?;
    let entry = str_param(p, "entry").map(str::trim).filter(|e| !e.is_empty()).ok_or_else(|| bad(cmd, "`entry` is required"))?.to_string();

    let settings = cadcraft_wood::settings_store::update(|w| {
        let items = list.get_mut(&mut w.lists);
        if items.iter().any(|existing| existing.eq_ignore_ascii_case(&entry)) {
            return Err(bad(cmd, "that entry already exists in the list"));
        }
        items.push(entry.clone());
        Ok(w.clone())
    })
    .map_err(|e| settings_error(cmd, e))??;
    settings_value(&settings)
}

/// RENAME LIST ENTRY: refused if `entry` does not exist, or `newEntry` is already used.
fn run_woodlistrename(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodlistrename";
    let list = list_param(cmd, p)?;
    let entry = str_param(p, "entry").ok_or_else(|| bad(cmd, "`entry` is required"))?.to_string();
    let new_entry = str_param(p, "newEntry").map(str::trim).filter(|e| !e.is_empty()).ok_or_else(|| bad(cmd, "`newEntry` is required"))?.to_string();

    let settings = cadcraft_wood::settings_store::update(|w| {
        let items = list.get_mut(&mut w.lists);
        if !entry.eq_ignore_ascii_case(&new_entry) && items.iter().any(|existing| existing.eq_ignore_ascii_case(&new_entry)) {
            return Err(bad(cmd, "that entry already exists in the list"));
        }
        let item =
            items.iter_mut().find(|existing| existing.eq_ignore_ascii_case(&entry)).ok_or_else(|| bad(cmd, "no entry in that list has that text"))?;
        *item = new_entry.clone();
        Ok(w.clone())
    })
    .map_err(|e| settings_error(cmd, e))??;
    settings_value(&settings)
}

/// DELETE LIST ENTRY.
fn run_woodlistdelete(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodlistdelete";
    let list = list_param(cmd, p)?;
    let entry = str_param(p, "entry").ok_or_else(|| bad(cmd, "`entry` is required"))?.to_string();

    let settings = cadcraft_wood::settings_store::update(|w| {
        let items = list.get_mut(&mut w.lists);
        let before = items.len();
        items.retain(|existing| !existing.eq_ignore_ascii_case(&entry));
        if items.len() == before {
            return Err(bad(cmd, "no entry in that list has that text"));
        }
        Ok(w.clone())
    })
    .map_err(|e| settings_error(cmd, e))??;
    settings_value(&settings)
}
