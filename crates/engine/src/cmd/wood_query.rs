//! WoodCraft Phase 2: reading a part's properties, listing parts, the display-name substitution,
//! and the commands that change Thickness, Grain direction and a part's display name. The block
//! name (the GUID) is never shown to the user; only [`cadcraft_wood::store::display_name`] is.

use cadcraft_doc::EntityKind;
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodparts", "List Wood Parts", run_woodparts).params("{} → {parts: [{guid, name, thickness, revision}]}").noundo(),
        CommandSpec::new("woodpartinfo", "Wood Part Properties", run_woodpartinfo)
            .params("{handle?} (default: the selection) → the part's JSON")
            .noundo(),
        CommandSpec::new("wooddisplayname", "Display Name For Block", run_wooddisplayname)
            .params("{block: name} → {name}: the part's display name, or the block name unchanged")
            .noundo(),
        CommandSpec::new("woodthickness", "Set Part Thickness", run_woodthickness).params("{handle?, thickness: number (above zero)}"),
        CommandSpec::new("woodgrain", "Set Part Grain Direction", run_woodgrain)
            .params("{handle?, grainDirection?: degrees (omit or null to clear)}"),
        CommandSpec::new("woodname", "Rename Wood Part", run_woodname).params("{handle?, name: \"new display name\"}"),
    ]
}

/// The one selected or named Insert's block name (the GUID), and an error naming the problem
/// otherwise (nothing selected, more than one, or not a wood part).
fn one_part_guid(s: &mut Session, cmd: &str, p: &Value) -> Result<String> {
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let doc = s.doc()?;
    let block = match doc.entity(hs[0]).map(|e| &e.kind) {
        Some(EntityKind::Insert(ins)) => ins.block.clone(),
        _ => return Err(bad(cmd, "the selected object is not a block reference")),
    };
    if store::get(doc).map_err(|e| EngineError::Other(e.to_string()))?.parts.contains_key(&block) {
        Ok(block)
    } else {
        Err(bad(cmd, "the selected block is not a WoodCraft part"))
    }
}

/// The summary of one part used by LIST WOOD PARTS.
fn summary(guid: &str, part: &cadcraft_wood::Part) -> Value {
    json!({ "guid": guid, "name": part.name, "thickness": part.thickness, "revision": part.revision })
}

/// LIST WOOD PARTS: every part in the drawing's WoodCraft data, by GUID.
fn run_woodparts(s: &mut Session, _p: &Value) -> Result<Value> {
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let parts: Vec<Value> = data.parts.iter().map(|(guid, part)| summary(guid, part)).collect();
    Ok(json!({ "parts": parts }))
}

/// WOOD PART PROPERTIES: the full JSON of one part (the selection, or `handle`).
fn run_woodpartinfo(s: &mut Session, p: &Value) -> Result<Value> {
    let guid = one_part_guid(s, "woodpartinfo", p)?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad("woodpartinfo", "the part was removed"))?;
    serde_json::to_value(part).map_err(|e| EngineError::Other(e.to_string()))
}

/// DISPLAY NAME FOR BLOCK: what the user should see for a block name — a wood part's display
/// name if `block` names one, otherwise `block` itself. The GUID itself is never shown.
fn run_wooddisplayname(s: &mut Session, p: &Value) -> Result<Value> {
    let block = str_param(p, "block").ok_or_else(|| bad("wooddisplayname", "`block` is required"))?;
    Ok(json!({ "name": store::display_name(s.doc()?, block) }))
}

/// SET PART THICKNESS: changes just the Thickness of one part.
fn run_woodthickness(s: &mut Session, p: &Value) -> Result<Value> {
    let guid = one_part_guid(s, "woodthickness", p)?;
    let thickness =
        size_param("woodthickness", p, "thickness", false)?.ok_or_else(|| bad("woodthickness", "`thickness` (number above zero) is required"))?;
    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.thickness = thickness;
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

/// SET PART GRAIN DIRECTION: changes just the Grain direction of one part; omitting or nulling
/// `grainDirection` clears it.
fn run_woodgrain(s: &mut Session, p: &Value) -> Result<Value> {
    let guid = one_part_guid(s, "woodgrain", p)?;
    let grain = match p.get("grainDirection") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad("woodgrain", "`grainDirection` must be a number of degrees"))?),
    };
    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.grain_direction = grain;
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}

/// RENAME WOOD PART: changes just the display name of one part. The block name (the GUID) is
/// never touched.
fn run_woodname(s: &mut Session, p: &Value) -> Result<Value> {
    let guid = one_part_guid(s, "woodname", p)?;
    let name = str_param(p, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad("woodname", "`name` is required"))?.to_string();
    store::update(s.doc_mut()?, |w| {
        if let Some(part) = w.parts.get_mut(&guid) {
            part.name = name;
        }
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;
    ok()
}
