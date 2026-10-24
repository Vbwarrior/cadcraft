//! WoodCraft Phase 6, group 5: edge profile, bevel and miter
//! (`plans/WoodCraft_Property_Names.xlsx`'s "Edges and ends" group: `Edge profile`, `Bevel`,
//! `Miter`). Each is a per-edge/end choice, same shape as End treatment (group 4): no position,
//! no drawn mark, at most one entry per `Edge / End` name. `Edge profile`'s `Profile` is checked
//! against the WoodCraft settings' Edge profile list, same as End treatment's `Profile` was
//! checked against its own list.

use cadcraft_wood::data::{AngleEntry, EdgeProfileEntry, Part};
use cadcraft_wood::settings_data::ListName;
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodedgeprofileset", "Set Wood Part Edge Profile", run_woodedgeprofileset)
            .params("{handle?, edgeEnd, profile, size}: `profile` must be in the WoodCraft settings' Edge profile list; replaces any entry for the same `edgeEnd`"),
        CommandSpec::new("woodedgeprofileclear", "Clear Wood Part Edge Profile", run_woodedgeprofileclear).params("{handle?, edgeEnd}"),
        CommandSpec::new("woodbevelset", "Set Wood Part Bevel", run_woodbevelset).params("{handle?, edgeEnd, angle}: replaces any entry for the same `edgeEnd`"),
        CommandSpec::new("woodbevelclear", "Clear Wood Part Bevel", run_woodbevelclear).params("{handle?, edgeEnd}"),
        CommandSpec::new("woodmiterset", "Set Wood Part Miter", run_woodmiterset).params("{handle?, edgeEnd, angle}: replaces any entry for the same `edgeEnd`"),
        CommandSpec::new("woodmiterclear", "Clear Wood Part Miter", run_woodmiterclear).params("{handle?, edgeEnd}"),
        CommandSpec::new("woodedgeinfo", "Wood Part Edge Properties", run_woodedgeinfo)
            .params("{handle?} (default: the selection) → {edgeProfiles, bevels, miters}")
            .noundo(),
    ]
}

fn edge_end_param(cmd: &str, p: &Value) -> Result<String> {
    str_param(p, "edgeEnd").map(str::trim).filter(|e| !e.is_empty()).map(str::to_string).ok_or_else(|| bad(cmd, "`edgeEnd` is required"))
}

/// SET WOOD PART EDGE PROFILE: `profile` must be one of the live settings' Edge profile list.
fn run_woodedgeprofileset(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodedgeprofileset";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_end = edge_end_param(cmd, p)?;
    let profile = str_param(p, "profile").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`profile` is required"))?.to_string();
    let size = size_param(cmd, p, "size", false)?.ok_or_else(|| bad(cmd, "`size` (number above zero) is required"))?;
    let settings = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;
    if !ListName::EdgeProfile.get(&settings.lists).iter().any(|v| v == &profile) {
        return Err(bad(cmd, "`profile` is not in the WoodCraft settings' Edge profile list"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.edge_profile.retain(|e| e.edge_end != edge_end);
            part.edge_profile.push(EdgeProfileEntry { edge_end, profile, size });
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

fn run_woodedgeprofileclear(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodedgeprofileclear";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_end = edge_end_param(cmd, p)?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        let before = part.edge_profile.len();
        part.edge_profile.retain(|e| e.edge_end != edge_end);
        part.edge_profile.len() != before
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "no edge profile is set for that edge/end"));
    }
    ok()
}

fn angle_field_mut(part: &mut Part, which: AngleKind) -> &mut Vec<AngleEntry> {
    match which {
        AngleKind::Bevel => &mut part.bevel,
        AngleKind::Miter => &mut part.miter,
    }
}

#[derive(Clone, Copy)]
enum AngleKind {
    Bevel,
    Miter,
}

fn run_angle_set(s: &mut Session, p: &Value, cmd: &str, which: AngleKind) -> Result<Value> {
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_end = edge_end_param(cmd, p)?;
    let angle =
        p.get("angle").and_then(Value::as_f64).filter(|a| a.is_finite()).ok_or_else(|| bad(cmd, "`angle` (number of degrees) is required"))?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            let field = angle_field_mut(part, which);
            field.retain(|e| e.edge_end != edge_end);
            field.push(AngleEntry { edge_end, angle });
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

fn run_angle_clear(s: &mut Session, p: &Value, cmd: &str, which: AngleKind) -> Result<Value> {
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_end = edge_end_param(cmd, p)?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        let field = angle_field_mut(part, which);
        let before = field.len();
        field.retain(|e| e.edge_end != edge_end);
        field.len() != before
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "nothing is set for that edge/end"));
    }
    ok()
}

fn run_woodbevelset(s: &mut Session, p: &Value) -> Result<Value> {
    run_angle_set(s, p, "woodbevelset", AngleKind::Bevel)
}
fn run_woodbevelclear(s: &mut Session, p: &Value) -> Result<Value> {
    run_angle_clear(s, p, "woodbevelclear", AngleKind::Bevel)
}
fn run_woodmiterset(s: &mut Session, p: &Value) -> Result<Value> {
    run_angle_set(s, p, "woodmiterset", AngleKind::Miter)
}
fn run_woodmiterclear(s: &mut Session, p: &Value) -> Result<Value> {
    run_angle_clear(s, p, "woodmiterclear", AngleKind::Miter)
}

/// WOOD PART EDGE PROPERTIES: the part's Edge profile, Bevel and Miter entries.
fn run_woodedgeinfo(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodedgeinfo";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let edge_profiles: Vec<Value> =
        part.edge_profile.iter().map(|e| json!({ "edgeEnd": e.edge_end, "profile": e.profile, "size": e.size })).collect();
    let bevels: Vec<Value> = part.bevel.iter().map(|e| json!({ "edgeEnd": e.edge_end, "angle": e.angle })).collect();
    let miters: Vec<Value> = part.miter.iter().map(|e| json!({ "edgeEnd": e.edge_end, "angle": e.angle })).collect();
    Ok(json!({ "edgeProfiles": edge_profiles, "bevels": bevels, "miters": miters }))
}
