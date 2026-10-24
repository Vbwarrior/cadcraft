//! WoodCraft Phase 6, group 2: holes (`plans/WoodCraft_Property_Names.xlsx`'s Holes group,
//! `Hole-Through`, `Hole-Blind`, `Hole-Counterbore`, `Hole-Countersink`, `Hole-Pocket-Screw`,
//! `Hole-Shelf-Pin`, `Hole-Dowel`). All seven kinds share one shape (`Face`, `Dia`, an optional
//! `Depth` and up to two kind-specific `Extra` fields, plus an `At` list of positions) and so
//! share one set of commands, told which kind by a `kind` parameter using the sheet's own name.
//!
//! Positions that differ only by location share one entry (`WoodCraft_Part_JSON_Draft.md`'s
//! "same shape, many places"): adding a hole whose Face/Dia/Depth/Extras exactly match an
//! existing entry appends to that entry's `At` list instead of creating a new entry.

use std::sync::Arc;

use cadcraft_doc::geom::PolyVertex;
use cadcraft_doc::{Entity, EntityKind, LwPolyline};
use cadcraft_geom::Vec2;
use cadcraft_wood::data::{At, Face, HoleEntry, Part};
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

/// Half the width of a hole's drawn cross marker.
const MARK_HALF: f64 = 0.1;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodholeadd", "Add Wood Part Hole", run_woodholeadd).params(
            "{handle?, kind: \"Hole-Through\"|\"Hole-Blind\"|\"Hole-Counterbore\"|\"Hole-Countersink\"|\"Hole-Pocket-Screw\"|\"Hole-Shelf-Pin\"|\"Hole-Dowel\", \
             face: \"A\"|\"B\", at: [x,y], dia, depth?, extra1?, extra2?}: adds a position to a matching existing entry, or a new entry",
        ),
        CommandSpec::new("woodholedelete", "Delete Wood Part Hole", run_woodholedelete)
            .params("{handle?, kind, at: [x,y]}: removes one position; removes the whole entry once its last position is gone"),
        CommandSpec::new("woodholes", "Wood Part Holes", run_woodholes)
            .params("{handle?, kind?} (default: every kind) → {holes: {kind: [{face, dia, depth?, extra1?, extra2?, at: [[x,y],...]}]}}")
            .noundo(),
    ]
}

/// The sheet's seven Hole kind names, and the field on `Part` each one is stored in.
fn hole_field<'a>(part: &'a Part, kind: &str) -> Option<&'a Vec<HoleEntry>> {
    match kind {
        "Hole-Through" => Some(&part.hole_through),
        "Hole-Blind" => Some(&part.hole_blind),
        "Hole-Counterbore" => Some(&part.hole_counterbore),
        "Hole-Countersink" => Some(&part.hole_countersink),
        "Hole-Pocket-Screw" => Some(&part.hole_pocket_screw),
        "Hole-Shelf-Pin" => Some(&part.hole_shelf_pin),
        "Hole-Dowel" => Some(&part.hole_dowel),
        _ => None,
    }
}

fn hole_field_mut<'a>(part: &'a mut Part, kind: &str) -> Option<&'a mut Vec<HoleEntry>> {
    match kind {
        "Hole-Through" => Some(&mut part.hole_through),
        "Hole-Blind" => Some(&mut part.hole_blind),
        "Hole-Counterbore" => Some(&mut part.hole_counterbore),
        "Hole-Countersink" => Some(&mut part.hole_countersink),
        "Hole-Pocket-Screw" => Some(&mut part.hole_pocket_screw),
        "Hole-Shelf-Pin" => Some(&mut part.hole_shelf_pin),
        "Hole-Dowel" => Some(&mut part.hole_dowel),
        _ => None,
    }
}

const HOLE_KINDS: &[&str] =
    &["Hole-Through", "Hole-Blind", "Hole-Counterbore", "Hole-Countersink", "Hole-Pocket-Screw", "Hole-Shelf-Pin", "Hole-Dowel"];

fn kind_param(cmd: &str, p: &Value) -> Result<String> {
    let text = str_param(p, "kind").ok_or_else(|| bad(cmd, "`kind` is required"))?;
    if HOLE_KINDS.contains(&text) { Ok(text.to_string()) } else { Err(bad(cmd, "`kind` must be one of the seven Hole- property names")) }
}

fn face_param(cmd: &str, p: &Value) -> Result<Face> {
    match str_param(p, "face") {
        Some("A") => Ok(Face::A),
        Some("B") => Ok(Face::B),
        _ => Err(bad(cmd, "`face` must be \"A\" or \"B\"")),
    }
}

fn optional_number(cmd: &str, p: &Value, key: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Some).ok_or_else(|| bad(cmd, format!("`{key}` must be a number"))),
    }
}

/// A small, deliberately plain cross at `at` — never a circle, so it can never be mistaken for
/// a part's outline (which `woodedges` finds by looking for the first closed polyline or circle
/// in the block).
fn hole_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let pl = LwPolyline {
        vertices: vec![
            PolyVertex::new(at + Vec2::new(-MARK_HALF, -MARK_HALF)),
            PolyVertex::new(at + Vec2::new(MARK_HALF, MARK_HALF)),
            PolyVertex::new(at),
            PolyVertex::new(at + Vec2::new(-MARK_HALF, MARK_HALF)),
            PolyVertex::new(at + Vec2::new(MARK_HALF, -MARK_HALF)),
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

/// Whether `e` is a hole mark (the X-shaped 5-vertex open polyline this module draws) centered
/// at `at`.
fn is_hole_mark_at(e: &Entity, at: Vec2) -> bool {
    match &e.kind {
        EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 5 => pl.vertices.get(2).is_some_and(|v| v.p.dist(at) < 1e-6),
        _ => false,
    }
}

/// ADD WOOD PART HOLE.
fn run_woodholeadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodholeadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let face = face_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let dia = size_param(cmd, p, "dia", false)?.ok_or_else(|| bad(cmd, "`dia` (number above zero) is required"))?;
    let depth = optional_number(cmd, p, "depth")?;
    let extra1 = optional_number(cmd, p, "extra1")?;
    let extra2 = optional_number(cmd, p, "extra2")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = hole_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let holes = hole_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown hole kind"))?;
        let matching = holes.iter_mut().find(|h| h.face == face && h.dia == dia && h.depth == depth && h.extra1 == extra1 && h.extra2 == extra2);
        match matching {
            Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
            None => holes.push(HoleEntry { face, dia, depth, extra1, extra2, at: vec![At { x: at.x, y: at.y }] }),
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

/// DELETE WOOD PART HOLE: removes the position nearest `at` from whichever entry of `kind`
/// contains it; removes the whole entry once its last position is gone.
fn run_woodholedelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodholedelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| -> Result<bool> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let holes = hole_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown hole kind"))?;
        for entry in holes.iter_mut() {
            if let Some(i) = entry.at.iter().position(|pos| Vec2::new(pos.x, pos.y).dist(at) < 1e-6) {
                entry.at.remove(i);
                holes.retain(|e| !e.at.is_empty());
                return Ok(true);
            }
        }
        Ok(false)
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    if !removed {
        return Err(bad(cmd, "no hole of that kind sits at that position"));
    }

    if let Some(block) = s.doc()?.block(&guid) {
        let victim = block.entities.iter().find(|e| is_hole_mark_at(e, at)).map(|e| e.handle);
        if let Some(h) = victim
            && let Some(blk) = s.doc_mut()?.blocks.get_mut(&guid)
        {
            Arc::make_mut(blk).entities.remove(h);
        }
    }
    ok()
}

fn hole_entry_value(e: &HoleEntry) -> Value {
    json!({
        "face": match e.face { Face::A => "A", Face::B => "B" },
        "dia": e.dia,
        "depth": e.depth,
        "extra1": e.extra1,
        "extra2": e.extra2,
        "at": e.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>(),
    })
}

/// WOOD PART HOLES: every hole entry, by kind (or just the one `kind` given).
fn run_woodholes(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodholes";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let kinds: Vec<&str> = match str_param(p, "kind") {
        Some(k) if HOLE_KINDS.contains(&k) => vec![k],
        Some(_) => return Err(bad(cmd, "`kind` must be one of the seven Hole- property names")),
        None => HOLE_KINDS.to_vec(),
    };
    let mut holes = serde_json::Map::new();
    for k in kinds {
        if let Some(entries) = hole_field(part, k)
            && !entries.is_empty()
        {
            holes.insert(k.to_string(), Value::Array(entries.iter().map(hole_entry_value).collect()));
        }
    }
    Ok(json!({ "holes": holes }))
}
