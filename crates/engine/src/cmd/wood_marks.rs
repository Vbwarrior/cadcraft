//! WoodCraft Phase 6, group 1: the orientation mark (V) and reference marks
//! (`plans/Wood_Workflow_Scope.md` "Orientation V" and "Marks"). Each is stored in the part's
//! JSON and drawn as a small, open (never closed) polyline in the part's own block, so
//! `woodedges`' outline search (which only looks at a closed polyline or a circle) never
//! mistakes a mark for the outline.

use std::sync::Arc;

use cadcraft_doc::geom::PolyVertex;
use cadcraft_doc::{Entity, EntityKind, LwPolyline};
use cadcraft_geom::Vec2;
use cadcraft_wood::data::{Face, OrientationMark, ReferenceMark};
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

/// How large a mark's own geometry is, in drawing units — independent of any text or dimension
/// scale, since a mark is a small fixed-size glyph like a center mark, not an annotation.
const MARK_SIZE: f64 = 0.25;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodorientationmark", "Set Wood Part Orientation Mark", run_woodorientationmark)
            .params("{handle?, face: \"A\"|\"B\", edge, at: [x,y]}: the V's point touches the named edge; drawn on the given face"),
        CommandSpec::new("woodorientationmarkclear", "Clear Wood Part Orientation Mark", run_woodorientationmarkclear)
            .params("{handle?}: back to the default (face A, top-left corner, pointing at the top edge; nothing is drawn)"),
        CommandSpec::new("woodreferencemarkadd", "Add Wood Part Reference Mark", run_woodreferencemarkadd)
            .params("{handle?, face: \"A\"|\"B\", at: [x,y], label}"),
        CommandSpec::new("woodreferencemarkdelete", "Delete Wood Part Reference Mark", run_woodreferencemarkdelete)
            .params("{handle?, label}: removes the first reference mark with that label"),
        CommandSpec::new("woodmarks", "Wood Part Marks", run_woodmarks)
            .params("{handle?} (default: the selection) → {orientationMark, referenceMarks: [...]}")
            .noundo(),
    ]
}

fn face_param(cmd: &str, p: &Value) -> Result<Face> {
    match str_param(p, "face") {
        Some("A") => Ok(Face::A),
        Some("B") => Ok(Face::B),
        _ => Err(bad(cmd, "`face` must be \"A\" or \"B\"")),
    }
}

fn face_label(f: Face) -> &'static str {
    match f {
        Face::A => "A",
        Face::B => "B",
    }
}

/// A small V: two short lines meeting at `at`, point touching the outline, opening upward. An
/// open (never closed) polyline, so it is never read as the part's outline.
fn v_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let half = MARK_SIZE / 2.0;
    let left = at + Vec2::new(-half, MARK_SIZE);
    let right = at + Vec2::new(half, MARK_SIZE);
    let pl = LwPolyline {
        vertices: vec![PolyVertex::new(left), PolyVertex::new(at), PolyVertex::new(right)],
        closed: false,
        const_width: 0.0,
        elevation: 0.0,
        plinegen: false,
    };
    Entity::new(doc.new_handle(), EntityKind::LwPolyline(pl))
}

/// A small cross at `at`, marking a reference point.
fn reference_mark_entity(doc: &mut cadcraft_doc::Drawing, at: Vec2) -> Entity {
    let half = MARK_SIZE / 2.0;
    let pl = LwPolyline {
        vertices: vec![
            PolyVertex::new(at + Vec2::new(-half, 0.0)),
            PolyVertex::new(at + Vec2::new(half, 0.0)),
            PolyVertex::new(at),
            PolyVertex::new(at + Vec2::new(0.0, -half)),
            PolyVertex::new(at + Vec2::new(0.0, half)),
        ],
        closed: false,
        const_width: 0.0,
        elevation: 0.0,
        plinegen: false,
    };
    Entity::new(doc.new_handle(), EntityKind::LwPolyline(pl))
}

/// Removes the given handles (every drawn mark being replaced or cleared) from the block.
fn remove_marks(doc: &mut cadcraft_doc::Drawing, guid: &str, handles: &[Handle]) {
    let Some(block) = doc.blocks.get_mut(guid) else { return };
    let b = Arc::make_mut(block);
    for h in handles {
        b.entities.remove(*h);
    }
}

fn add_entity_to_block(doc: &mut cadcraft_doc::Drawing, guid: &str, e: Entity) -> Result<Handle> {
    let block = doc.blocks.get_mut(guid).ok_or_else(|| EngineError::Other(format!("block \"{guid}\" not found")))?;
    let b = Arc::make_mut(block);
    let h = e.handle;
    b.entities.push(e);
    Ok(h)
}

/// SET WOOD PART ORIENTATION MARK: replaces any existing one.
fn run_woodorientationmark(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodorientationmark";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face = face_param(cmd, p)?;
    let edge = str_param(p, "edge").map(str::trim).filter(|e| !e.is_empty()).ok_or_else(|| bad(cmd, "`edge` is required"))?.to_string();
    let at = point_req(cmd, p, "at")?;
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let old_marks: Vec<Handle> = doc_mark_handles(s.doc()?, &guid, MarkKind::Orientation);
    let doc = s.doc_mut()?;
    remove_marks(doc, &guid, &old_marks);
    let entity = v_mark_entity(doc, at);
    let handle = add_entity_to_block(doc, &guid, entity)?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.orientation_mark = Some(OrientationMark { face, edge: edge.clone() });
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(json!({ "mark": handle.hex() }))
}

/// CLEAR WOOD PART ORIENTATION MARK: back to the unstored default; removes its drawn mark.
fn run_woodorientationmarkclear(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodorientationmarkclear";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let marks = doc_mark_handles(s.doc()?, &guid, MarkKind::Orientation);
    remove_marks(s.doc_mut()?, &guid, &marks);
    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.orientation_mark = None;
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

/// ADD WOOD PART REFERENCE MARK.
fn run_woodreferencemarkadd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodreferencemarkadd";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let face = face_param(cmd, p)?;
    let at = point_req(cmd, p, "at")?;
    let label = str_param(p, "label").map(str::trim).filter(|l| !l.is_empty()).ok_or_else(|| bad(cmd, "`label` is required"))?.to_string();
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let entity = reference_mark_entity(s.doc_mut()?, at);
    let handle = add_entity_to_block(s.doc_mut()?, &guid, entity)?;

    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.reference_marks.push(ReferenceMark { face, x: at.x, y: at.y, label: label.clone() });
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(json!({ "mark": handle.hex() }))
}

/// DELETE WOOD PART REFERENCE MARK: removes the first one with the given label, and its drawn
/// mark (found by position, since the JSON entry itself carries no handle).
fn run_woodreferencemarkdelete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodreferencemarkdelete";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let label = str_param(p, "label").ok_or_else(|| bad(cmd, "`label` is required"))?.to_string();
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;

    let removed_at = store::update(s.doc_mut()?, |w| {
        let part = w.parts.get_mut(&guid)?;
        let i = part.reference_marks.iter().position(|m| m.label == label)?;
        let m = part.reference_marks.remove(i);
        Some(Vec2::new(m.x, m.y))
    })
    .map_err(|e| EngineError::Other(e.to_string()))?
    .ok_or_else(|| bad(cmd, "no reference mark has that label"))?;

    // The drawn mark nearest the removed entry's position, among this part's reference marks.
    if let Some(block) = s.doc()?.block(&guid) {
        let victim = block.entities.iter().find(|e| is_reference_mark_at(e, removed_at)).map(|e| e.handle);
        if let Some(h) = victim {
            remove_marks(s.doc_mut()?, &guid, &[h]);
        }
    }
    ok()
}

/// WOOD PART MARKS: the part's orientation mark (if set) and its reference marks.
fn run_woodmarks(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodmarks";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;
    let orientation = part.orientation_mark.as_ref().map(|m| json!({ "face": face_label(m.face), "edge": m.edge }));
    let references: Vec<Value> =
        part.reference_marks.iter().map(|m| json!({ "face": face_label(m.face), "x": m.x, "y": m.y, "label": m.label })).collect();
    Ok(json!({ "orientationMark": orientation, "referenceMarks": references }))
}

enum MarkKind {
    Orientation,
}

/// The handles, in the part's block, of every drawn mark of the given kind — an open 3-vertex
/// polyline for the orientation V (there is at most one), found structurally rather than by any
/// stored tag, since a mark is deliberately plain geometry with no metadata of its own.
fn doc_mark_handles(doc: &cadcraft_doc::Drawing, guid: &str, kind: MarkKind) -> Vec<Handle> {
    let Some(block) = doc.block(guid) else { return Vec::new() };
    match kind {
        MarkKind::Orientation => block
            .entities
            .iter()
            .filter(|e| matches!(&e.kind, EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 3))
            .map(|e| e.handle)
            .collect(),
    }
}

/// Whether `e` is a 5-vertex open polyline (a reference mark's cross) centered at `at`.
fn is_reference_mark_at(e: &Entity, at: Vec2) -> bool {
    match &e.kind {
        EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 5 => pl.vertices.get(2).is_some_and(|v| v.p.dist(at) < 1e-6),
        _ => false,
    }
}
