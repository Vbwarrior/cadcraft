//! WoodCraft Phase 6, group 4: joinery and end treatments
//! (`plans/WoodCraft_Property_Names.xlsx`'s "Joinery (tenon, mortise, etc.)" group: `Mortise`,
//! `Tenon`, `Joint type`, `Biscuit`; and the "Edges and ends" group's `End treatment`).
//!
//! Mortise, Tenon and Biscuit are positioned features sharing one shape, same pattern as Holes
//! and Cuts (`Face / Edge`, `Length`, `Width`, optional `Depth`/`Tolerance`, an `At` list).
//! `Joint type` is a selector — which joint a part's end or edge makes and to which other
//! part — with no position. `End treatment` is a per-end/edge choice from the settings list
//! Phase 4 already seeds (`Lists` → "End treatment"), validated against that live list.

use std::sync::Arc;

use cadcraft_doc::geom::PolyVertex;
use cadcraft_doc::{Entity, EntityKind, LwPolyline};
use cadcraft_geom::Vec2;
use cadcraft_wood::data::{At, EndTreatmentEntry, JoineryEntry, JointTypeEntry, Part};
use cadcraft_wood::settings_data::ListName;
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

const MARK_HALF: f64 = 0.15;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodjoineryadd", "Add Wood Part Joinery", run_woodjoineryadd).params(
            "{handle?, kind: \"Mortise\"|\"Tenon\"|\"Biscuit\", faceEdge, at: [x,y], length, width, depth?, tolerance?}: \
             `depth`/`tolerance` apply to Mortise/Tenon only, refused for Biscuit",
        ),
        CommandSpec::new("woodjoinerydelete", "Delete Wood Part Joinery", run_woodjoinerydelete)
            .params("{handle?, kind, at}: removes one position; removes the whole entry once its last position is gone"),
        CommandSpec::new("woodjoinery", "Wood Part Joinery", run_woodjoinery)
            .params("{handle?, kind?} (default: Mortise, Tenon and Biscuit) → {joinery: {kind: [{faceEdge, length, width, depth?, tolerance?, at}]}}")
            .noundo(),
        CommandSpec::new("woodjointtypeset", "Set Wood Part Joint Type", run_woodjointtypeset)
            .params("{handle?, jointCode, matingPartId, tolerance?}: replaces any existing Joint type entry"),
        CommandSpec::new("woodjointtypeclear", "Clear Wood Part Joint Type", run_woodjointtypeclear).params("{handle?}"),
        CommandSpec::new("woodendtreatmentset", "Set Wood Part End Treatment", run_woodendtreatmentset).params(
            "{handle?, edgeEnd, profile}: `profile` must be in the WoodCraft settings' End treatment list; replaces any entry for the same `edgeEnd`",
        ),
        CommandSpec::new("woodendtreatmentclear", "Clear Wood Part End Treatment", run_woodendtreatmentclear).params("{handle?, edgeEnd}"),
        CommandSpec::new("woodjointinfo", "Wood Part Joint Info", run_woodjointinfo)
            .params("{handle?} (default: the selection) → {jointType, endTreatments: [...]}")
            .noundo(),
    ]
}

const JOINERY_KINDS: &[&str] = &["Mortise", "Tenon", "Biscuit"];

fn joinery_field<'a>(part: &'a Part, kind: &str) -> Option<&'a Vec<JoineryEntry>> {
    match kind {
        "Mortise" => Some(&part.mortise),
        "Tenon" => Some(&part.tenon),
        "Biscuit" => Some(&part.biscuit),
        _ => None,
    }
}
fn joinery_field_mut<'a>(part: &'a mut Part, kind: &str) -> Option<&'a mut Vec<JoineryEntry>> {
    match kind {
        "Mortise" => Some(&mut part.mortise),
        "Tenon" => Some(&mut part.tenon),
        "Biscuit" => Some(&mut part.biscuit),
        _ => None,
    }
}

fn kind_param(cmd: &str, p: &Value) -> Result<String> {
    let text = str_param(p, "kind").ok_or_else(|| bad(cmd, "`kind` is required"))?;
    if JOINERY_KINDS.contains(&text) { Ok(text.to_string()) } else { Err(bad(cmd, "`kind` must be \"Mortise\", \"Tenon\" or \"Biscuit\"")) }
}

fn optional_number(cmd: &str, p: &Value, key: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Some).ok_or_else(|| bad(cmd, format!("`{key}` must be a number"))),
    }
}

fn joinery_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let pl = LwPolyline {
        vertices: vec![
            PolyVertex::new(at + Vec2::new(0.0, -MARK_HALF)),
            PolyVertex::new(at + Vec2::new(MARK_HALF, MARK_HALF)),
            PolyVertex::new(at + Vec2::new(-MARK_HALF, MARK_HALF)),
            PolyVertex::new(at + Vec2::new(0.0, -MARK_HALF)),
        ],
        closed: false,
        const_width: 0.0,
        elevation: 0.0,
        plinegen: false,
    };
    Entity::new(doc.new_handle(), EntityKind::LwPolyline(pl))
}

fn add_entity_to_block(doc: &mut cadcraft_doc::Drawing, guid: &str, e: Entity) -> Result<Handle> {
    let block = doc.blocks.get_mut(guid).ok_or_else(|| EngineError::Other(format!("block \"{guid}\" not found")))?;
    let b = Arc::make_mut(block);
    let h = e.handle;
    b.entities.push(e);
    Ok(h)
}

fn is_joinery_mark_at(e: &Entity, at: Vec2) -> bool {
    match &e.kind {
        EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 4 => {
            pl.vertices.first().is_some_and(|v| v.p.dist(at + Vec2::new(0.0, -MARK_HALF)) < 1e-6)
        }
        _ => false,
    }
}

/// ADD WOOD PART JOINERY.
fn run_woodjoineryadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodjoineryadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let face_edge = str_param(p, "faceEdge").map(str::trim).filter(|f| !f.is_empty()).ok_or_else(|| bad(cmd, "`faceEdge` is required"))?.to_string();
    let at = point_req(cmd, p, "at")?;
    let length = size_param(cmd, p, "length", false)?.ok_or_else(|| bad(cmd, "`length` (number above zero) is required"))?;
    let width = size_param(cmd, p, "width", false)?.ok_or_else(|| bad(cmd, "`width` (number above zero) is required"))?;
    let depth = optional_number(cmd, p, "depth")?;
    let tolerance = optional_number(cmd, p, "tolerance")?;
    if kind == "Biscuit" && (depth.is_some() || tolerance.is_some()) {
        return Err(bad(cmd, "`depth` and `tolerance` are not used for Biscuit"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = joinery_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let entries = joinery_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown joinery kind"))?;
        let matching = entries
            .iter_mut()
            .find(|e| e.face_edge == face_edge && e.length == length && e.width == width && e.depth == depth && e.tolerance == tolerance);
        match matching {
            Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
            None => entries.push(JoineryEntry { face_edge: face_edge.clone(), length, width, depth, tolerance, at: vec![At { x: at.x, y: at.y }] }),
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

/// DELETE WOOD PART JOINERY.
fn run_woodjoinerydelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodjoinerydelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| -> Result<bool> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let entries = joinery_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown joinery kind"))?;
        for entry in entries.iter_mut() {
            if let Some(i) = entry.at.iter().position(|pos| Vec2::new(pos.x, pos.y).dist(at) < 1e-6) {
                entry.at.remove(i);
                entries.retain(|e| !e.at.is_empty());
                return Ok(true);
            }
        }
        Ok(false)
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    if !removed {
        return Err(bad(cmd, "no joinery of that kind sits at that position"));
    }

    if let Some(block) = s.doc()?.block(&guid) {
        let victim = block.entities.iter().find(|e| is_joinery_mark_at(e, at)).map(|e| e.handle);
        if let Some(h) = victim
            && let Some(blk) = s.doc_mut()?.blocks.get_mut(&guid)
        {
            Arc::make_mut(blk).entities.remove(h);
        }
    }
    ok()
}

fn joinery_entry_value(e: &JoineryEntry) -> Value {
    json!({
        "faceEdge": e.face_edge,
        "length": e.length,
        "width": e.width,
        "depth": e.depth,
        "tolerance": e.tolerance,
        "at": e.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>(),
    })
}

/// WOOD PART JOINERY.
fn run_woodjoinery(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodjoinery";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let kinds: Vec<&str> = match str_param(p, "kind") {
        Some(k) if JOINERY_KINDS.contains(&k) => vec![k],
        Some(_) => return Err(bad(cmd, "`kind` must be \"Mortise\", \"Tenon\" or \"Biscuit\"")),
        None => JOINERY_KINDS.to_vec(),
    };
    let mut joinery = serde_json::Map::new();
    for k in kinds {
        if let Some(entries) = joinery_field(part, k)
            && !entries.is_empty()
        {
            joinery.insert(k.to_string(), Value::Array(entries.iter().map(joinery_entry_value).collect()));
        }
    }
    Ok(json!({ "joinery": joinery }))
}

/// SET WOOD PART JOINT TYPE: replaces any existing entry (a part has at most one).
fn run_woodjointtypeset(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodjointtypeset";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let joint_code =
        str_param(p, "jointCode").map(str::trim).filter(|j| !j.is_empty()).ok_or_else(|| bad(cmd, "`jointCode` is required"))?.to_string();
    let mating_part_id =
        str_param(p, "matingPartId").map(str::trim).filter(|m| !m.is_empty()).ok_or_else(|| bad(cmd, "`matingPartId` is required"))?.to_string();
    let tolerance = optional_number(cmd, p, "tolerance")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.joint_type = vec![JointTypeEntry { joint_code, mating_part_id, tolerance }];
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

fn run_woodjointtypeclear(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodjointtypeclear";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.joint_type.clear();
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

/// SET WOOD PART END TREATMENT: `profile` must be one of the WoodCraft settings' End treatment
/// list entries; replaces any existing entry for the same `edgeEnd`.
fn run_woodendtreatmentset(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodendtreatmentset";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_end = str_param(p, "edgeEnd").map(str::trim).filter(|e| !e.is_empty()).ok_or_else(|| bad(cmd, "`edgeEnd` is required"))?.to_string();
    let profile = str_param(p, "profile").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`profile` is required"))?.to_string();
    let settings = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;
    if !ListName::EndTreatment.get(&settings.lists).iter().any(|v| v == &profile) {
        return Err(bad(cmd, "`profile` is not in the WoodCraft settings' End treatment list"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.end_treatment.retain(|e| e.edge_end != edge_end);
            part.end_treatment.push(EndTreatmentEntry { edge_end, profile });
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

fn run_woodendtreatmentclear(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodendtreatmentclear";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_end = str_param(p, "edgeEnd").ok_or_else(|| bad(cmd, "`edgeEnd` is required"))?.to_string();
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        let before = part.end_treatment.len();
        part.end_treatment.retain(|e| e.edge_end != edge_end);
        part.end_treatment.len() != before
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "no end treatment is set for that edge/end"));
    }
    ok()
}

/// WOOD PART JOINT INFO: the part's Joint type (or `null`) and its End treatments.
fn run_woodjointinfo(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodjointinfo";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
    let joint_type =
        part.joint_type.first().map(|j| json!({ "jointCode": j.joint_code, "matingPartId": j.mating_part_id, "tolerance": j.tolerance }));
    let end_treatments: Vec<Value> = part.end_treatment.iter().map(|e| json!({ "edgeEnd": e.edge_end, "profile": e.profile })).collect();
    Ok(json!({ "jointType": joint_type, "endTreatments": end_treatments }))
}
