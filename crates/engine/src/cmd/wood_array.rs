//! WoodCraft Phase 3: copy and array rules (`plans/Wood_Workflow_Scope.md` "Copy and array
//! rules"). A part's instances are plain block references sharing one block name (the GUID) —
//! there is no separate array object; "the array" is just the set of Inserts found by
//! [`wood_query::instances_of`]. Splitting an instance off gives it a new GUID, a copy of the
//! block's geometry under that new name, and a copy of the JSON entry; the rest stay shared.

use std::sync::Arc;

use cadcraft_doc::{Block, Entity, EntityKind};
use cadcraft_geom::Vec2;
use cadcraft_wood::{Part, guid::new_guid, store};
use serde_json::{Value, json};

use super::blocks::insert;
use super::wood_query::{instances_of, part_guid_of};
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodcopy", "Copy Wood Part", run_woodcopy)
            .params("{handle?, to: [x,y]} → {part: new guid, insert}: an independent copy, new GUID, display name \"<name> -Copy\""),
        CommandSpec::new("woodcopyarray", "Copy Wood Part Array", run_woodcopyarray)
            .params("{handle?, to: [x,y]} → {insert}: another instance of the same part, same GUID, sharing its entry"),
        CommandSpec::new("wooddelete", "Delete Wood Part", run_wooddelete)
            .params("{handles?} → {deleted}: erases the instance(s); removes a part's entry once its last instance is gone"),
        CommandSpec::new("woodsplitarray", "Split Wood Part Array", run_woodsplitarray)
            .params("{handles} → {changed: [{from, insert, part}]}: the picked instances get a new, independent GUID each; refused whole if any pick is not an instance of its array"),
        CommandSpec::new("woodremovefrom", "Remove From Wood Part Array", run_woodsplitarray)
            .params("{handles} → {changed: [{from, insert, part}]}: same operation as Split Wood Part Array, named for the \"remove from the array\" menu entry"),
        CommandSpec::new("woodexplodearray", "Explode Wood Part Array", run_woodexplodearray)
            .params("{handle?} → {changed: [{from, insert, part}]}: every instance of the part becomes its own independent part"),
    ]
}

/// Duplicate block `src_guid`'s geometry under a brand new GUID, and place one Insert of it at
/// `at`. Mirrors `blocks::make_block`'s own-handle-per-entity pattern, but for copying an
/// existing block's entities rather than consuming loose geometry.
fn duplicate_block(s: &mut Session, src_guid: &str, at: Vec2) -> Result<(String, Handle)> {
    let new_guid = new_guid();
    let doc = s.doc()?;
    let src = doc.block(src_guid).ok_or_else(|| EngineError::Other(format!("block \"{src_guid}\" not found")))?;
    let base = src.base;
    let description = src.description.clone();
    let ents: Vec<Entity> = src.entities.iter().map(|e| (**e).clone()).collect();

    let mut b = Block::new(&new_guid);
    b.base = base;
    b.description = description;
    let doc = s.doc_mut()?;
    for mut e in ents {
        e.handle = doc.new_handle();
        b.entities.push(e);
    }
    doc.blocks.insert(new_guid.clone(), Arc::new(b));
    let h = insert(s, &new_guid, at, 1.0, 0.0, &serde_json::Map::new())?;
    Ok((new_guid, h))
}

/// COPY WOOD PART: a wholly independent copy — new GUID, a copy of the block's geometry, a copy
/// of the JSON entry, display name `"<name> -Copy"`.
fn run_woodcopy(s: &mut Session, p: &Value) -> Result<Value> {
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad("woodcopy", "select exactly one wood part"));
    }
    let to = point_req("woodcopy", p, "to")?;
    let src_guid = part_guid_of(s.doc()?, "woodcopy", hs[0])?;

    // Geometry first, then the JSON; both in this one command, so one undo step.
    let (new_guid, insert_h) = duplicate_block(s, &src_guid, to)?;
    let mut part = store::get(s.doc()?)
        .map_err(|e| EngineError::Other(e.to_string()))?
        .parts
        .get(&src_guid)
        .cloned()
        .ok_or_else(|| bad("woodcopy", "the selected block is not a WoodCraft part"))?;
    part.name = format!("{} -Copy", part.name);
    part.revision = 1;
    store::update(s.doc_mut()?, |w| {
        w.parts.insert(new_guid.clone(), part);
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;

    Ok(json!({ "part": new_guid, "insert": insert_h.hex() }))
}

/// COPY WOOD PART ARRAY: another instance of the same part at `to` — same GUID, same (shared)
/// JSON entry. No JSON change: only another Insert.
fn run_woodcopyarray(s: &mut Session, p: &Value) -> Result<Value> {
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad("woodcopyarray", "select exactly one wood part"));
    }
    let to = point_req("woodcopyarray", p, "to")?;
    let guid = part_guid_of(s.doc()?, "woodcopyarray", hs[0])?;
    let h = insert(s, &guid, to, 1.0, 0.0, &serde_json::Map::new())?;
    Ok(json!({ "insert": h.hex() }))
}

/// DELETE WOOD PART: erase the instance(s); a part whose last instance is erased has its JSON
/// entry removed too (`Wood_Workflow_Scope.md`: "Delete: deleting the last instance of a GUID
/// removes its entry").
fn run_wooddelete(s: &mut Session, p: &Value) -> Result<Value> {
    let hs = targets(s, p)?;
    let doc = s.doc()?;
    let mut touched: Vec<String> = hs.iter().filter_map(|h| part_guid_of(doc, "wooddelete", *h).ok()).collect();
    touched.sort();
    touched.dedup();

    let d = s.doc_mut()?;
    let mut n = 0usize;
    for h in &hs {
        if d.remove_entity(*h).is_some() {
            n += 1;
        }
    }
    s.set_selection(Vec::new());

    let doc = s.doc()?;
    let gone: Vec<String> = touched.into_iter().filter(|g| instances_of(doc, g).is_empty()).collect();
    if !gone.is_empty() {
        store::update(s.doc_mut()?, |w| {
            for g in &gone {
                w.parts.remove(g);
            }
        })
        .map_err(|e| EngineError::Other(e.to_string()))?;
    }
    Ok(json!({ "deleted": n }))
}

/// One instance split off into its own part: its old block (array) name, its Insert's handle
/// (unchanged — it still names the SAME Insert, now pointing at the new block) and its new GUID.
fn split_one(s: &mut Session, from_guid: &str, h: Handle, part: &Part) -> Result<Value> {
    let at = match s.doc()?.entity(h).map(|e| &e.kind) {
        Some(EntityKind::Insert(ins)) => ins.insert.xy(),
        _ => Vec2::ZERO,
    };
    let (new_guid, new_insert) = duplicate_block(s, from_guid, at)?;
    if let Some(ins) = s.doc()?.entity(h).map(|e| &e.kind)
        && let EntityKind::Insert(i) = ins
    {
        let rotation = i.rotation;
        let scale = i.scale;
        s.doc_mut()?.modify_entity(new_insert, |e| {
            if let EntityKind::Insert(ni) = &mut e.kind {
                ni.rotation = rotation;
                ni.scale = scale;
            }
        })?;
    }
    s.doc_mut()?.remove_entity(h);
    let mut copy = part.clone();
    copy.revision = 1;
    store::update(s.doc_mut()?, |w| {
        w.parts.insert(new_guid.clone(), copy);
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(json!({ "from": from_guid, "insert": new_insert.hex(), "part": new_guid }))
}

/// Remove `guid`'s JSON entry if none of its instances remain (all were split off). Mirrors
/// DELETE WOOD PART's own last-instance cleanup.
fn gc_if_empty(s: &mut Session, guid: &str) -> Result<()> {
    if !instances_of(s.doc()?, guid).is_empty() {
        return Ok(());
    }
    store::update(s.doc_mut()?, |w| {
        w.parts.remove(guid);
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(())
}

/// SPLIT WOOD PART ARRAY (and REMOVE FROM, the same operation): the picked instances each get a
/// new, independent GUID and a copy of the entry; refused whole, with nothing changed, if any
/// picked handle is not an instance of its part.
fn run_woodsplitarray(s: &mut Session, p: &Value) -> Result<Value> {
    let hs = targets(s, p)?;
    if hs.is_empty() {
        return Err(bad("woodsplitarray", "select at least one instance"));
    }
    let doc = s.doc()?;
    let mut guids = Vec::with_capacity(hs.len());
    for h in &hs {
        guids.push(part_guid_of(doc, "woodsplitarray", *h)?);
    }
    let data = store::get(doc).map_err(|e| EngineError::Other(e.to_string()))?;

    let mut changed = Vec::with_capacity(hs.len());
    let mut touched: Vec<String> = guids.clone();
    for (h, guid) in hs.into_iter().zip(guids) {
        let part = data.parts.get(&guid).ok_or_else(|| bad("woodsplitarray", "the part was removed"))?.clone();
        changed.push(split_one(s, &guid, h, &part)?);
    }
    touched.sort();
    touched.dedup();
    for guid in &touched {
        gc_if_empty(s, guid)?;
    }
    Ok(json!({ "changed": changed }))
}

/// EXPLODE WOOD PART ARRAY: every instance of the selected part becomes its own independent
/// part — Split Array applied to the whole array at once.
fn run_woodexplodearray(s: &mut Session, p: &Value) -> Result<Value> {
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad("woodexplodearray", "select exactly one instance of the array"));
    }
    let doc = s.doc()?;
    let guid = part_guid_of(doc, "woodexplodearray", hs[0])?;
    let all = instances_of(doc, &guid);
    let part = store::get(doc)
        .map_err(|e| EngineError::Other(e.to_string()))?
        .parts
        .get(&guid)
        .cloned()
        .ok_or_else(|| bad("woodexplodearray", "the part was removed"))?;

    let mut changed = Vec::with_capacity(all.len());
    for h in all {
        changed.push(split_one(s, &guid, h, &part)?);
    }
    gc_if_empty(s, &guid)?;
    Ok(json!({ "changed": changed }))
}
