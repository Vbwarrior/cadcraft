//! WoodCraft Phase 6, group 6 (part 1): glue and fasteners
//! (`plans/WoodCraft_Property_Names.xlsx`'s "Glue and fasteners" group: `Glue`, `Screw`, `Nail`,
//! `Dowel`). `Glue` is a per-edge/face selection with no position (same shape as End treatment
//! and Edge profile); `Screw`, `Nail` and `Dowel` are positioned features with an `At` list,
//! same pattern as Holes and Cuts. `Screw`'s `Type` is checked against the WoodCraft settings'
//! Screw type list; `Glue`'s `Type` against the Glue type list. Nails have no type — only
//! diameter and length (`Wood_Workflow_Scope.md`).

use std::sync::Arc;

use cadcraft_doc::geom::PolyVertex;
use cadcraft_doc::{Entity, EntityKind, LwPolyline};
use cadcraft_geom::Vec2;
use cadcraft_wood::data::{At, FastenerEntry, GlueEntry, NailEntry};
use cadcraft_wood::settings_data::ListName;
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

const MARK_HALF: f64 = 0.12;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodglueset", "Set Wood Part Glue", run_woodglueset).params(
            "{handle?, faceEdge, type}: `type` must be in the WoodCraft settings' Glue type list; replaces any entry for the same `faceEdge`",
        ),
        CommandSpec::new("woodglueclear", "Clear Wood Part Glue", run_woodglueclear).params("{handle?, faceEdge}"),
        CommandSpec::new("woodscrewadd", "Add Wood Part Screw", run_woodscrewadd)
            .params("{handle?, faceEdge, type, at: [x,y], size}: `type` must be in the WoodCraft settings' Screw type list"),
        CommandSpec::new("woodscrewdelete", "Delete Wood Part Screw", run_woodscrewdelete).params("{handle?, at: [x,y]}"),
        CommandSpec::new("woodnailadd", "Add Wood Part Nail", run_woodnailadd).params("{handle?, faceEdge, at: [x,y], diameter, length}"),
        CommandSpec::new("woodnaildelete", "Delete Wood Part Nail", run_woodnaildelete).params("{handle?, at: [x,y]}"),
        CommandSpec::new("wooddowelfastenadd", "Add Wood Part Dowel Fastener", run_wooddowelfastenadd).params("{handle?, faceEdge, at: [x,y], size}"),
        CommandSpec::new("wooddowelfastendelete", "Delete Wood Part Dowel Fastener", run_wooddowelfastendelete).params("{handle?, at: [x,y]}"),
        CommandSpec::new("woodfasteners", "Wood Part Fasteners", run_woodfasteners)
            .params("{handle?} (default: the selection) → {glue, screws, nails, dowels}")
            .noundo(),
    ]
}

fn fastener_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let pl = LwPolyline {
        vertices: vec![
            PolyVertex::new(at + Vec2::new(0.0, MARK_HALF)),
            PolyVertex::new(at + Vec2::new(0.0, -MARK_HALF)),
            PolyVertex::new(at),
            PolyVertex::new(at + Vec2::new(-MARK_HALF, 0.0)),
            PolyVertex::new(at + Vec2::new(MARK_HALF, 0.0)),
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

fn is_fastener_mark_at(e: &Entity, at: Vec2) -> bool {
    match &e.kind {
        EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 5 => pl.vertices.get(2).is_some_and(|v| v.p.dist(at) < 1e-6),
        _ => false,
    }
}

fn face_edge_param(cmd: &str, p: &Value) -> Result<String> {
    str_param(p, "faceEdge").map(str::trim).filter(|f| !f.is_empty()).map(str::to_string).ok_or_else(|| bad(cmd, "`faceEdge` is required"))
}

/// SET WOOD PART GLUE: `type` must be one of the live settings' Glue type list.
fn run_woodglueset(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodglueset";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face_edge = face_edge_param(cmd, p)?;
    let glue_type = str_param(p, "type").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`type` is required"))?.to_string();
    let settings = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;
    if !ListName::GlueType.get(&settings.lists).iter().any(|v| v == &glue_type) {
        return Err(bad(cmd, "`type` is not in the WoodCraft settings' Glue type list"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.glue.retain(|g| g.face_edge != face_edge);
            part.glue.push(GlueEntry { face_edge, glue_type });
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

fn run_woodglueclear(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodglueclear";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face_edge = face_edge_param(cmd, p)?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        let before = part.glue.len();
        part.glue.retain(|g| g.face_edge != face_edge);
        part.glue.len() != before
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "no glue is set for that face/edge"));
    }
    ok()
}

/// ADD WOOD PART SCREW: `type` must be one of the live settings' Screw type list.
fn run_woodscrewadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodscrewadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face_edge = face_edge_param(cmd, p)?;
    let screw_type = str_param(p, "type").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`type` is required"))?.to_string();
    let at = point_req(cmd, p, "at")?;
    let size = size_param(cmd, p, "size", false)?.ok_or_else(|| bad(cmd, "`size` (number above zero) is required"))?;
    let settings = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;
    if !ListName::ScrewType.get(&settings.lists).iter().any(|v| v == &screw_type) {
        return Err(bad(cmd, "`type` is not in the WoodCraft settings' Screw type list"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = fastener_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let matching =
            part.screw.iter_mut().find(|sc| sc.face_edge == face_edge && sc.fastener_type.as_deref() == Some(screw_type.as_str()) && sc.size == size);
        match matching {
            Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
            None => part.screw.push(FastenerEntry {
                face_edge: face_edge.clone(),
                fastener_type: Some(screw_type.clone()),
                size,
                at: vec![At { x: at.x, y: at.y }],
            }),
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

fn run_woodscrewdelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodscrewdelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        for entry in part.screw.iter_mut() {
            if let Some(i) = entry.at.iter().position(|pos| Vec2::new(pos.x, pos.y).dist(at) < 1e-6) {
                entry.at.remove(i);
                part.screw.retain(|e| !e.at.is_empty());
                return true;
            }
        }
        false
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "no screw sits at that position"));
    }
    remove_mark_at(s, &guid, at)?;
    ok()
}

/// ADD WOOD PART NAIL: nails have no type, only diameter and length.
fn run_woodnailadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodnailadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face_edge = face_edge_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let diameter = size_param(cmd, p, "diameter", false)?.ok_or_else(|| bad(cmd, "`diameter` (number above zero) is required"))?;
    let length = size_param(cmd, p, "length", false)?.ok_or_else(|| bad(cmd, "`length` (number above zero) is required"))?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = fastener_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let matching = part.nail.iter_mut().find(|n| n.face_edge == face_edge && n.diameter == diameter && n.length == length);
        match matching {
            Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
            None => part.nail.push(NailEntry { face_edge: face_edge.clone(), diameter, length, at: vec![At { x: at.x, y: at.y }] }),
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

fn run_woodnaildelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodnaildelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        for entry in part.nail.iter_mut() {
            if let Some(i) = entry.at.iter().position(|pos| Vec2::new(pos.x, pos.y).dist(at) < 1e-6) {
                entry.at.remove(i);
                part.nail.retain(|e| !e.at.is_empty());
                return true;
            }
        }
        false
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "no nail sits at that position"));
    }
    remove_mark_at(s, &guid, at)?;
    ok()
}

/// ADD WOOD PART DOWEL FASTENER: a glued dowel pin, distinct from `Hole-Dowel` (a drilled hole).
fn run_wooddowelfastenadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "wooddowelfastenadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face_edge = face_edge_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let size = size_param(cmd, p, "size", false)?.ok_or_else(|| bad(cmd, "`size` (number above zero) is required"))?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = fastener_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let matching = part.dowel.iter_mut().find(|d| d.face_edge == face_edge && d.size == size);
        match matching {
            Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
            None => part.dowel.push(FastenerEntry { face_edge: face_edge.clone(), fastener_type: None, size, at: vec![At { x: at.x, y: at.y }] }),
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

fn run_wooddowelfastendelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "wooddowelfastendelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| {
        let Some(part) = w.parts.get_mut(&guid) else { return false };
        for entry in part.dowel.iter_mut() {
            if let Some(i) = entry.at.iter().position(|pos| Vec2::new(pos.x, pos.y).dist(at) < 1e-6) {
                entry.at.remove(i);
                part.dowel.retain(|e| !e.at.is_empty());
                return true;
            }
        }
        false
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    if !removed {
        return Err(bad(cmd, "no dowel fastener sits at that position"));
    }
    remove_mark_at(s, &guid, at)?;
    ok()
}

fn remove_mark_at(s: &mut Session, guid: &str, at: Vec2) -> Result<()> {
    if let Some(block) = s.doc()?.block(guid) {
        let victim = block.entities.iter().find(|e| is_fastener_mark_at(e, at)).map(|e| e.handle);
        if let Some(h) = victim
            && let Some(blk) = s.doc_mut()?.blocks.get_mut(guid)
        {
            Arc::make_mut(blk).entities.remove(h);
        }
    }
    Ok(())
}

/// WOOD PART FASTENERS: the part's Glue, Screw, Nail and Dowel entries.
fn run_woodfasteners(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodfasteners";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let glue: Vec<Value> = part.glue.iter().map(|g| json!({ "faceEdge": g.face_edge, "type": g.glue_type })).collect();
    let screws: Vec<Value> = part
        .screw
        .iter()
        .map(|sc| json!({ "faceEdge": sc.face_edge, "type": sc.fastener_type, "size": sc.size, "at": sc.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>() }))
        .collect();
    let nails: Vec<Value> = part
        .nail
        .iter()
        .map(|n| json!({ "faceEdge": n.face_edge, "diameter": n.diameter, "length": n.length, "at": n.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>() }))
        .collect();
    let dowels: Vec<Value> = part
        .dowel
        .iter()
        .map(|d| json!({ "faceEdge": d.face_edge, "size": d.size, "at": d.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>() }))
        .collect();
    Ok(json!({ "glue": glue, "screws": screws, "nails": nails, "dowels": dowels }))
}
