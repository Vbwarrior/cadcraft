//! WoodCraft Phase 6, group 6 (part 2): hardware markers
//! (`plans/WoodCraft_Property_Names.xlsx`'s "Hardware markers" group: `Hinge`, `Drawer slide`,
//! `Drawer Pull`). All three share one shape (`Face`, `Type`, position, optional `Size`), told
//! apart by a `kind` parameter — same approach as Holes, Cuts and Joinery. `Type` is checked
//! against the WoodCraft settings' Hardware type list. Hinge and Drawer Pull are "location
//! only" (no `Size`); Drawer slide has one.

use std::sync::Arc;

use cadcraft_doc::geom::PolyVertex;
use cadcraft_doc::{Entity, EntityKind, LwPolyline};
use cadcraft_geom::Vec2;
use cadcraft_wood::data::{At, Face, HardwareEntry, Part};
use cadcraft_wood::settings_data::ListName;
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

const MARK_HALF: f64 = 0.2;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodhardwareadd", "Add Wood Part Hardware", run_woodhardwareadd).params(
            "{handle?, kind: \"Hinge\"|\"Drawer slide\"|\"Drawer Pull\", face, type, at: [x,y], size?}: \
             `type` must be in the WoodCraft settings' Hardware type list; `size` only applies to Drawer slide",
        ),
        CommandSpec::new("woodhardwaredelete", "Delete Wood Part Hardware", run_woodhardwaredelete).params("{handle?, kind, at: [x,y]}"),
        CommandSpec::new("woodhardware", "Wood Part Hardware", run_woodhardware)
            .params("{handle?, kind?} (default: every kind) → {hardware: {kind: [{face, type, size?, at}]}}")
            .noundo(),
    ]
}

const HARDWARE_KINDS: &[&str] = &["Hinge", "Drawer slide", "Drawer Pull"];

fn needs_size(kind: &str) -> bool {
    kind == "Drawer slide"
}

fn hardware_field<'a>(part: &'a Part, kind: &str) -> Option<&'a Vec<HardwareEntry>> {
    match kind {
        "Hinge" => Some(&part.hinge),
        "Drawer slide" => Some(&part.drawer_slide),
        "Drawer Pull" => Some(&part.drawer_pull),
        _ => None,
    }
}
fn hardware_field_mut<'a>(part: &'a mut Part, kind: &str) -> Option<&'a mut Vec<HardwareEntry>> {
    match kind {
        "Hinge" => Some(&mut part.hinge),
        "Drawer slide" => Some(&mut part.drawer_slide),
        "Drawer Pull" => Some(&mut part.drawer_pull),
        _ => None,
    }
}

fn kind_param(cmd: &str, p: &Value) -> Result<String> {
    let text = str_param(p, "kind").ok_or_else(|| bad(cmd, "`kind` is required"))?;
    if HARDWARE_KINDS.contains(&text) { Ok(text.to_string()) } else { Err(bad(cmd, "`kind` must be \"Hinge\", \"Drawer slide\" or \"Drawer Pull\"")) }
}

fn face_param(cmd: &str, p: &Value) -> Result<Face> {
    match str_param(p, "face") {
        Some("A") => Ok(Face::A),
        Some("B") => Ok(Face::B),
        _ => Err(bad(cmd, "`face` must be \"A\" or \"B\"")),
    }
}

fn hardware_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let pl = LwPolyline {
        vertices: vec![
            PolyVertex::new(at + Vec2::new(-MARK_HALF, 0.0)),
            PolyVertex::new(at),
            PolyVertex::new(at + Vec2::new(MARK_HALF, 0.0)),
            PolyVertex::new(at),
            PolyVertex::new(at + Vec2::new(0.0, MARK_HALF)),
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

fn is_hardware_mark_at(e: &Entity, at: Vec2) -> bool {
    match &e.kind {
        EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 5 => pl.vertices.get(1).is_some_and(|v| v.p.dist(at) < 1e-6),
        _ => false,
    }
}

/// ADD WOOD PART HARDWARE.
fn run_woodhardwareadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodhardwareadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let face = face_param(cmd, p)?;
    let hardware_type = str_param(p, "type").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`type` is required"))?.to_string();
    let at = point_req(cmd, p, "at")?;
    let size = match p.get("size") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_f64().filter(|x| x.is_finite() && *x > 0.0).ok_or_else(|| bad(cmd, "`size` must be a number above zero"))?),
    };
    if needs_size(&kind) && size.is_none() {
        return Err(bad(cmd, "`size` is required for Drawer slide"));
    }
    if !needs_size(&kind) && size.is_some() {
        return Err(bad(cmd, "`size` is not used for this kind"));
    }
    let settings = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?;
    if !ListName::HardwareType.get(&settings.lists).iter().any(|v| v == &hardware_type) {
        return Err(bad(cmd, "`type` is not in the WoodCraft settings' Hardware type list"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = hardware_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let entries = hardware_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown hardware kind"))?;
        let matching = entries.iter_mut().find(|e| e.face == face && e.hardware_type == hardware_type && e.size == size);
        match matching {
            Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
            None => entries.push(HardwareEntry { face, hardware_type: hardware_type.clone(), size, at: vec![At { x: at.x, y: at.y }] }),
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

fn run_woodhardwaredelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodhardwaredelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| -> Result<bool> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let entries = hardware_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown hardware kind"))?;
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
        return Err(bad(cmd, "no hardware of that kind sits at that position"));
    }

    if let Some(block) = s.doc()?.block(&guid) {
        let victim = block.entities.iter().find(|e| is_hardware_mark_at(e, at)).map(|e| e.handle);
        if let Some(h) = victim
            && let Some(blk) = s.doc_mut()?.blocks.get_mut(&guid)
        {
            Arc::make_mut(blk).entities.remove(h);
        }
    }
    ok()
}

fn hardware_entry_value(e: &HardwareEntry) -> Value {
    json!({
        "face": match e.face { Face::A => "A", Face::B => "B" },
        "type": e.hardware_type,
        "size": e.size,
        "at": e.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>(),
    })
}

/// WOOD PART HARDWARE: every hardware entry, by kind (or just the one `kind` given).
fn run_woodhardware(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodhardware";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let kinds: Vec<&str> = match str_param(p, "kind") {
        Some(k) if HARDWARE_KINDS.contains(&k) => vec![k],
        Some(_) => return Err(bad(cmd, "`kind` must be \"Hinge\", \"Drawer slide\" or \"Drawer Pull\"")),
        None => HARDWARE_KINDS.to_vec(),
    };
    let mut hardware = serde_json::Map::new();
    for k in kinds {
        if let Some(entries) = hardware_field(part, k)
            && !entries.is_empty()
        {
            hardware.insert(k.to_string(), Value::Array(entries.iter().map(hardware_entry_value).collect()));
        }
    }
    Ok(json!({ "hardware": hardware }))
}
