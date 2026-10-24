//! WoodCraft Phase 7: the warnings engine. A running list, like Visual Studio's Error List
//! (`plans/Wood_Workflow_Scope.md` "Warnings"): computed fresh every call from the drawing's
//! current parts and the live settings, never stored — a warning appears when found and
//! disappears once whatever caused it is fixed. Acknowledging one is session-only, never saved
//! to the file, and (per the scope doc) is meant to last for one export only; see
//! `Session::acknowledged_wood_warnings`.

use cadcraft_doc::EntityKind;
use cadcraft_wood::settings_store;
use cadcraft_wood::store;
use cadcraft_wood::warnings::{Warning, WarningKind, part_warnings};
use serde_json::{Value, json};

use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodwarnings", "WoodCraft Warnings", run_woodwarnings)
            .params("{} → {warnings: [{id, kind, partGuid, message, acknowledged}]}: every current warning across the drawing's wood parts")
            .noundo(),
        CommandSpec::new("woodwarningacknowledge", "Acknowledge Wood Warning", run_woodwarningacknowledge)
            .params("{id}: overrides one warning for this session only, until it is next recomputed as part of an export")
            .enabled(always)
            .noundo(),
        CommandSpec::new("woodwarningunacknowledge", "Un-acknowledge Wood Warning", run_woodwarningunacknowledge)
            .params("{id}: un-does an acknowledge; the warning blocks export again")
            .enabled(always)
            .noundo(),
    ]
}

fn kind_label(k: WarningKind) -> &'static str {
    match k {
        WarningKind::Fastener => "Fastener",
        WarningKind::JointFit => "JointFit",
        WarningKind::BuildSize => "BuildSize",
    }
}

/// The physical dimensions (width, height, thickness) of one part's outline, for the
/// build-size check — `None` if the part's block or outline can no longer be found (the part's
/// entry still gets its fastener and joint fit warnings; build size is simply skipped for it).
fn part_dimensions(doc: &cadcraft_doc::Drawing, guid: &str, thickness: f64) -> Option<(f64, f64, f64)> {
    let block = doc.block(guid)?;
    let outline =
        block.entities.iter().find(|e| matches!(&e.kind, EntityKind::LwPolyline(pl) if pl.closed) || matches!(&e.kind, EntityKind::Circle(_)))?;
    let bounds = cadcraft_doc::entity_bounds(doc, outline, 0);
    Some((bounds.width(), bounds.height(), thickness))
}

fn warning_value(w: &Warning, acknowledged: &std::collections::HashSet<String>) -> Value {
    json!({
        "id": w.id,
        "kind": kind_label(w.kind),
        "partGuid": w.part_guid,
        "message": w.message,
        "acknowledged": acknowledged.contains(&w.id),
    })
}

/// WOODCRAFT WARNINGS: every current warning, across every wood part in the drawing.
fn run_woodwarnings(s: &mut Session, _p: &Value) -> Result<Value> {
    let doc = s.doc()?;
    let data = store::get(doc).map_err(|e| EngineError::Other(e.to_string()))?;
    let settings = settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;

    let mut warnings = Vec::new();
    for (guid, part) in &data.parts {
        let dims = part_dimensions(doc, guid, part.thickness);
        warnings.extend(part_warnings(guid, part, &settings, dims));
    }
    let acknowledged = &s.acknowledged_wood_warnings;
    Ok(json!({ "warnings": warnings.iter().map(|w| warning_value(w, acknowledged)).collect::<Vec<_>>() }))
}

/// ACKNOWLEDGE WOOD WARNING: overrides it for this session, until the next export resets it
/// (per `Wood_Workflow_Scope.md`: "An override lasts for one export only. The next export asks
/// again"). Nothing here is saved to the file.
fn run_woodwarningacknowledge(s: &mut Session, p: &Value) -> Result<Value> {
    let id =
        str_param(p, "id").map(str::trim).filter(|i| !i.is_empty()).ok_or_else(|| bad("woodwarningacknowledge", "`id` is required"))?.to_string();
    s.acknowledged_wood_warnings.insert(id);
    ok()
}

fn run_woodwarningunacknowledge(s: &mut Session, p: &Value) -> Result<Value> {
    let id =
        str_param(p, "id").map(str::trim).filter(|i| !i.is_empty()).ok_or_else(|| bad("woodwarningunacknowledge", "`id` is required"))?.to_string();
    s.acknowledged_wood_warnings.remove(&id);
    ok()
}
