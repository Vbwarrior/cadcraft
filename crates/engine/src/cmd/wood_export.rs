//! WoodCraft Phase 8: Export. Writes Document B (`plans/WoodCraft_Part_JSON_Draft.md` §4) to a
//! folder: `project.json` plus one SCAD file per part under `models/`. Respects the
//! block-on-fastener-warnings setting and the per-export acknowledge/override list
//! (`Wood_Workflow_Scope.md` "Warnings": "An override lasts for one export only. The next
//! export asks again" — a successful export clears `Session::acknowledged_wood_warnings`).

use cadcraft_doc::EntityKind;
use cadcraft_wood::export::{Outline, ProjectDocument, ProjectInfo, part_entry, scad_for_part};
use cadcraft_wood::warnings::{WarningKind, part_warnings};
use cadcraft_wood::{settings_store, store};
use serde_json::{Value, json};

use super::wood_query::instances_of;
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodexport", "Export Project...", run_woodexport)
            .params(
                "{path: folder, projectName: string, openscadCheck?: bool (default false)} → \
                 {path, parts: number, blocked?: [warning ids], openscad?: {ok, parts: [{part, ok, message?}]}}: \
                 writes project.json and models/<guid>.scad; refused while unacknowledged fastener warnings exist \
                 and the blockExportOnFastenerWarnings setting is on",
            )
            .noundo(),
    ]
}

/// The part's outline, read from its block in local coordinates (same lookup as `woodedges`
/// and `part_dimensions` in `wood_warnings.rs`): the one closed polyline or circle `woodpart`
/// converted the selection into.
fn part_outline(doc: &cadcraft_doc::Drawing, guid: &str) -> Option<Outline> {
    let block = doc.block(guid)?;
    block.entities.iter().find_map(|e| match &e.kind {
        EntityKind::LwPolyline(pl) if pl.closed => Some(Outline::Polyline(pl.vertices.iter().map(|v| [v.p.x, v.p.y, v.bulge]).collect())),
        EntityKind::Circle(c) => Some(Outline::Circle { cx: c.center.x, cy: c.center.y, r: c.radius }),
        _ => None,
    })
}

/// EXPORT PROJECT: writes Document B to `path`. Blocked while any fastener warning is neither
/// fixed nor acknowledged this session and the setting requires it; otherwise writes
/// `project.json` and `models/<guid>.scad` per part, then clears the session's acknowledged
/// warnings (the override is for this export only).
fn run_woodexport(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodexport";
    let path = str_param(p, "path").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`path` is required"))?;
    let project_name = str_param(p, "projectName").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`projectName` is required"))?;
    let openscad_check = p.get("openscadCheck").and_then(Value::as_bool).unwrap_or(false);

    let doc = s.doc()?;
    let data = store::get(doc).map_err(|e| EngineError::Other(e.to_string()))?;
    let settings = settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;

    let blocked = blocking_fastener_warnings(doc, &data, &settings, &s.acknowledged_wood_warnings);
    if !blocked.is_empty() {
        return Ok(json!({ "path": path, "parts": 0, "blocked": blocked }));
    }

    let models_dir = std::path::Path::new(path).join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| bad(cmd, format!("{path}: {e}")))?;

    let mut parts = Vec::new();
    for (guid, part) in &data.parts {
        let outline = part_outline(doc, guid).ok_or_else(|| bad(cmd, format!("part \"{}\" has no outline to export", part.name)))?;
        let instances = instances_of(doc, guid).len() as u32;
        let model_rel = format!("models/{guid}.scad");
        let scad = scad_for_part(part, &outline);
        std::fs::write(models_dir.join(format!("{guid}.scad")), scad).map_err(|e| bad(cmd, format!("{path}: {e}")))?;
        parts.push(part_entry(guid, part, &outline, instances, &model_rel));
    }

    let document = ProjectDocument {
        schema_version: 1,
        units: data.units.clone(),
        property_list_version: data.property_list_version,
        project: ProjectInfo { name: project_name.to_string() },
        parts: parts.clone(),
    };
    let json_text = serde_json::to_string_pretty(&document).map_err(|e| EngineError::Other(e.to_string()))?;
    let project_path = std::path::Path::new(path).join("project.json");
    std::fs::write(&project_path, json_text).map_err(|e| bad(cmd, format!("{path}: {e}")))?;

    let openscad = if openscad_check { Some(check_with_openscad(&models_dir, &data)) } else { None };

    s.acknowledged_wood_warnings.clear();

    let mut result = json!({ "path": path, "parts": parts.len() });
    if let Some(check) = openscad {
        result["openscad"] = check;
    }
    Ok(result)
}

/// Every fastener warning id that is neither fixed nor acknowledged this session, when the
/// `blockExportOnFastenerWarnings` setting requires it to be clear before export. Empty when
/// the setting is off, or every current fastener warning has already been acknowledged.
fn blocking_fastener_warnings(
    doc: &cadcraft_doc::Drawing,
    data: &cadcraft_wood::WoodData,
    settings: &cadcraft_wood::WoodSettings,
    acknowledged: &std::collections::HashSet<String>,
) -> Vec<String> {
    if !settings.block_export_on_fastener_warnings {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (guid, part) in &data.parts {
        for w in part_warnings(guid, part, settings, part_dimensions(doc, guid, part.thickness)) {
            if w.kind == WarningKind::Fastener && !acknowledged.contains(&w.id) {
                out.push(w.id);
            }
        }
    }
    out
}

/// Same lookup `wood_warnings.rs`'s `part_dimensions` uses, duplicated here rather than shared
/// across modules for one bounds call — see that module for the rationale (build-size warnings
/// need the block's outline, which only the engine can read).
fn part_dimensions(doc: &cadcraft_doc::Drawing, guid: &str, thickness: f64) -> Option<(f64, f64, f64)> {
    let block = doc.block(guid)?;
    let outline =
        block.entities.iter().find(|e| matches!(&e.kind, EntityKind::LwPolyline(pl) if pl.closed) || matches!(&e.kind, EntityKind::Circle(_)))?;
    let bounds = cadcraft_doc::entity_bounds(doc, outline, 0);
    Some((bounds.width(), bounds.height(), thickness))
}

/// Runs `openscad -o out.stl in.scad` for each written SCAD file as an optional check that it
/// renders (`Wood_Workflow_Scope.md` §5: "OpenSCAD is called as a separate process ... not
/// embedded"). Missing OpenSCAD on the machine is reported per part, not treated as an export
/// failure — the files are already written; this is a courtesy check only.
fn check_with_openscad(models_dir: &std::path::Path, data: &cadcraft_wood::WoodData) -> Value {
    let mut results = Vec::new();
    let mut all_ok = true;
    for guid in data.parts.keys() {
        let scad_path = models_dir.join(format!("{guid}.scad"));
        let stl_path = models_dir.join(format!("{guid}.stl"));
        let outcome = std::process::Command::new("openscad").arg("-o").arg(&stl_path).arg(&scad_path).output();
        let entry = match outcome {
            Ok(out) if out.status.success() => json!({ "part": guid, "ok": true }),
            Ok(out) => {
                all_ok = false;
                json!({ "part": guid, "ok": false, "message": String::from_utf8_lossy(&out.stderr).trim().to_string() })
            }
            Err(e) => {
                all_ok = false;
                json!({ "part": guid, "ok": false, "message": format!("could not run openscad: {e}") })
            }
        };
        results.push(entry);
    }
    json!({ "ok": all_ok, "parts": results })
}
