//! WoodCraft Phase 6, group 3: cuts (`plans/WoodCraft_Property_Names.xlsx`'s "Cuts into the
//! part" group: `Cut-Dado`, `Cut-Groove`, `Cut-Rabbet`, `Cut-Slot`, `Cut-Notch`, `Cut-Pocket`).
//! Each kind uses a different subset of the group's common parameters (`Face / Edge`, `Start
//! X`/`Start Y`, `Length`, `Width`, `Depth`, `Angle`): `Cut-Rabbet` has no start position ("along
//! an edge"); `Cut-Slot` has an angle but no depth ("through slot"); `Cut-Notch` has neither.
//! Positions that differ only by location share one entry (an `At` list), same as Holes.
//!
//! A dado or groove cut's position, width and depth are exactly what WorkShop Pro's assembler
//! reads to build its own snap plane (`plans/Wood_Workflow_Scope.md`'s "Dado and groove
//! planes") — nothing extra is stored here for that; the snap plane itself is WP's concern.

use std::sync::Arc;

use cadcraft_doc::geom::PolyVertex;
use cadcraft_doc::{Entity, EntityKind, LwPolyline};
use cadcraft_geom::Vec2;
use cadcraft_wood::data::{At, CutEntry, Part};
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

const MARK_HALF: f64 = 0.15;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodcutadd", "Add Wood Part Cut", run_woodcutadd).params(
            "{handle?, kind: \"Cut-Dado\"|\"Cut-Groove\"|\"Cut-Rabbet\"|\"Cut-Slot\"|\"Cut-Notch\"|\"Cut-Pocket\", \
             faceEdge, length, width, at?: [x,y], depth?, angle?}: `at` is required except for Cut-Rabbet; \
             `depth` is required for Cut-Dado/Cut-Groove/Cut-Pocket; `angle` is required for Cut-Slot",
        ),
        CommandSpec::new("woodcutdelete", "Delete Wood Part Cut", run_woodcutdelete).params(
            "{handle?, kind, at?}: removes one position (or the one entry, for Cut-Rabbet); removes the whole entry once its last position is gone",
        ),
        CommandSpec::new("woodcuts", "Wood Part Cuts", run_woodcuts)
            .params("{handle?, kind?} (default: every kind) → {cuts: {kind: [{faceEdge, length, width, depth?, angle?, at: [[x,y],...]}]}}")
            .noundo(),
    ]
}

const CUT_KINDS: &[&str] = &["Cut-Dado", "Cut-Groove", "Cut-Rabbet", "Cut-Slot", "Cut-Notch", "Cut-Pocket"];

fn needs_at(kind: &str) -> bool {
    kind != "Cut-Rabbet"
}
fn needs_depth(kind: &str) -> bool {
    matches!(kind, "Cut-Dado" | "Cut-Groove" | "Cut-Rabbet" | "Cut-Pocket")
}
fn needs_angle(kind: &str) -> bool {
    kind == "Cut-Slot"
}

fn cut_field<'a>(part: &'a Part, kind: &str) -> Option<&'a Vec<CutEntry>> {
    match kind {
        "Cut-Dado" => Some(&part.cut_dado),
        "Cut-Groove" => Some(&part.cut_groove),
        "Cut-Rabbet" => Some(&part.cut_rabbet),
        "Cut-Slot" => Some(&part.cut_slot),
        "Cut-Notch" => Some(&part.cut_notch),
        "Cut-Pocket" => Some(&part.cut_pocket),
        _ => None,
    }
}

fn cut_field_mut<'a>(part: &'a mut Part, kind: &str) -> Option<&'a mut Vec<CutEntry>> {
    match kind {
        "Cut-Dado" => Some(&mut part.cut_dado),
        "Cut-Groove" => Some(&mut part.cut_groove),
        "Cut-Rabbet" => Some(&mut part.cut_rabbet),
        "Cut-Slot" => Some(&mut part.cut_slot),
        "Cut-Notch" => Some(&mut part.cut_notch),
        "Cut-Pocket" => Some(&mut part.cut_pocket),
        _ => None,
    }
}

fn kind_param(cmd: &str, p: &Value) -> Result<String> {
    let text = str_param(p, "kind").ok_or_else(|| bad(cmd, "`kind` is required"))?;
    if CUT_KINDS.contains(&text) { Ok(text.to_string()) } else { Err(bad(cmd, "`kind` must be one of the six Cut- property names")) }
}

fn optional_number(cmd: &str, p: &Value, key: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Some).ok_or_else(|| bad(cmd, format!("`{key}` must be a number"))),
    }
}

/// A small square outline, deliberately never a closed polyline or circle, so it is never read
/// as the part's outline by `woodedges`.
fn cut_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let pl = LwPolyline {
        vertices: vec![
            PolyVertex::new(at + Vec2::new(-MARK_HALF, -MARK_HALF)),
            PolyVertex::new(at + Vec2::new(MARK_HALF, -MARK_HALF)),
            PolyVertex::new(at + Vec2::new(MARK_HALF, MARK_HALF)),
            PolyVertex::new(at + Vec2::new(-MARK_HALF, MARK_HALF)),
            PolyVertex::new(at + Vec2::new(-MARK_HALF, -MARK_HALF)),
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

/// Whether `e` is a cut mark (the small square this module draws) centered at `at`: its first
/// corner sits `at - (MARK_HALF, MARK_HALF)`.
fn is_cut_mark_at(e: &Entity, at: Vec2) -> bool {
    match &e.kind {
        EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 5 => {
            let corner = at + Vec2::new(-MARK_HALF, -MARK_HALF);
            pl.vertices.first().is_some_and(|v| v.p.dist(corner) < 1e-6)
        }
        _ => false,
    }
}

/// ADD WOOD PART CUT.
fn run_woodcutadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodcutadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let face_edge = str_param(p, "faceEdge").map(str::trim).filter(|f| !f.is_empty()).ok_or_else(|| bad(cmd, "`faceEdge` is required"))?.to_string();
    let length = size_param(cmd, p, "length", false)?.ok_or_else(|| bad(cmd, "`length` (number above zero) is required"))?;
    let width = size_param(cmd, p, "width", false)?.ok_or_else(|| bad(cmd, "`width` (number above zero) is required"))?;
    let depth = optional_number(cmd, p, "depth")?;
    let angle = optional_number(cmd, p, "angle")?;
    let at = point_param(p, "at");

    if needs_at(&kind) && at.is_none() {
        return Err(bad(cmd, "`at` is required for this kind"));
    }
    if !needs_at(&kind) && at.is_some() {
        return Err(bad(cmd, "`at` is not used for Cut-Rabbet"));
    }
    if needs_depth(&kind) && depth.is_none() {
        return Err(bad(cmd, "`depth` is required for this kind"));
    }
    if needs_angle(&kind) && angle.is_none() {
        return Err(bad(cmd, "`angle` is required for Cut-Slot"));
    }

    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let mark_at = at.unwrap_or(Vec2::ZERO);
    let entity = cut_mark_entity(s.doc_mut()?, mark_at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| -> Result<()> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let cuts = cut_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown cut kind"))?;
        if let Some(at) = at {
            let matching =
                cuts.iter_mut().find(|c| c.face_edge == face_edge && c.length == length && c.width == width && c.depth == depth && c.angle == angle);
            match matching {
                Some(entry) => entry.at.push(At { x: at.x, y: at.y }),
                None => cuts.push(CutEntry { face_edge: face_edge.clone(), length, width, depth, angle, at: vec![At { x: at.x, y: at.y }] }),
            }
        } else {
            cuts.push(CutEntry { face_edge: face_edge.clone(), length, width, depth, angle, at: Vec::new() });
        }
        Ok(())
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    Ok(json!({ "mark": handle.hex() }))
}

/// DELETE WOOD PART CUT.
fn run_woodcutdelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodcutdelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let kind = kind_param(cmd, p)?;
    let at = point_param(p, "at");
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed = store::update(s.doc_mut()?, |w| -> Result<bool> {
        let part = w.parts.get_mut(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
        let cuts = cut_field_mut(part, &kind).ok_or_else(|| bad(cmd, "unknown cut kind"))?;
        if !needs_at(&kind) {
            if cuts.is_empty() {
                return Ok(false);
            }
            cuts.clear();
            return Ok(true);
        }
        let Some(at) = at else { return Err(bad(cmd, "`at` is required for this kind")) };
        for entry in cuts.iter_mut() {
            if let Some(i) = entry.at.iter().position(|pos| Vec2::new(pos.x, pos.y).dist(at) < 1e-6) {
                entry.at.remove(i);
                cuts.retain(|e| !e.at.is_empty());
                return Ok(true);
            }
        }
        Ok(false)
    })
    .map_err(|e| EngineError::Other(e.to_string()))??;
    if !removed {
        return Err(bad(cmd, "no cut of that kind sits at that position"));
    }

    if let Some(at) = at
        && let Some(block) = s.doc()?.block(&guid)
    {
        let victim = block.entities.iter().find(|e| is_cut_mark_at(e, at)).map(|e| e.handle);
        if let Some(h) = victim
            && let Some(blk) = s.doc_mut()?.blocks.get_mut(&guid)
        {
            Arc::make_mut(blk).entities.remove(h);
        }
    }
    ok()
}

fn cut_entry_value(e: &CutEntry) -> Value {
    json!({
        "faceEdge": e.face_edge,
        "length": e.length,
        "width": e.width,
        "depth": e.depth,
        "angle": e.angle,
        "at": e.at.iter().map(|a| json!([a.x, a.y])).collect::<Vec<_>>(),
    })
}

/// WOOD PART CUTS: every cut entry, by kind (or just the one `kind` given).
fn run_woodcuts(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodcuts";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let kinds: Vec<&str> = match str_param(p, "kind") {
        Some(k) if CUT_KINDS.contains(&k) => vec![k],
        Some(_) => return Err(bad(cmd, "`kind` must be one of the six Cut- property names")),
        None => CUT_KINDS.to_vec(),
    };
    let mut cuts = serde_json::Map::new();
    for k in kinds {
        if let Some(entries) = cut_field(part, k)
            && !entries.is_empty()
        {
            cuts.insert(k.to_string(), Value::Array(entries.iter().map(cut_entry_value).collect()));
        }
    }
    Ok(json!({ "cuts": cuts }))
}
