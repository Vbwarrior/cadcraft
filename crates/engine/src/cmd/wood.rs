//! WoodCraft commands. The part properties live in the drawing as JSON (see `cadcraft-wood`);
//! a command that changes a part changes the geometry and the JSON in the same step, so one
//! undo step restores both.

use cadcraft_doc::EntityKind;
use cadcraft_geom::Vec2;
use cadcraft_wood::{CommonSize, Part, guid::new_guid, store};
use serde_json::{Value, json};

use super::blocks::{Keep, make_block};
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodpart", "Make Wood Part...", run_woodpart).params(
            "{name: \"display name\", thickness: number (above zero), commonSize?: {name, width, thickness}, grainDirection?: degrees, handles?: [closed polyline or circle] (default: the selection)}",
        ),
    ]
}

/// MAKE WOOD PART: turn a closed outline into a part. The outline becomes a block named with a
/// new GUID (the user only ever sees the display name), replaced in the drawing by a reference
/// to it, and the part's properties are stored in the drawing's WoodCraft JSON.
fn run_woodpart(s: &mut Session, p: &Value) -> Result<Value> {
    let name = str_param(p, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad("woodpart", "`name` is required"))?.to_string();
    let thickness = size_param("woodpart", p, "thickness", false)?.ok_or_else(|| bad("woodpart", "`thickness` (number above zero) is required"))?;
    let common = common_size(p)?;
    let grain = match p.get("grainDirection") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad("woodpart", "`grainDirection` must be a number of degrees"))?),
    };

    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad("woodpart", "select exactly one closed outline"));
    }
    let closed = match s.doc()?.entity(hs[0]).map(|e| &e.kind) {
        Some(EntityKind::LwPolyline(pl)) => pl.closed,
        Some(EntityKind::Circle(_)) => true,
        _ => false,
    };
    if !closed {
        return Err(bad("woodpart", "the outline must be one closed polyline or circle"));
    }

    // Geometry first, then the JSON; both are in this one command, so one undo step.
    let guid = new_guid();
    let insert = make_block(s, &guid, Vec2::ZERO, &hs, Keep::Convert, &name)?;
    let mut part = Part::new(&name, thickness);
    part.common_size = common;
    part.grain_direction = grain;
    store::update(s.doc_mut()?, |w| {
        w.parts.insert(guid.clone(), part);
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;

    Ok(json!({ "part": guid, "name": name, "insert": insert.map(|h| h.hex()) }))
}

/// The optional `commonSize` parameter: the stock size name with its width and thickness.
fn common_size(p: &Value) -> Result<Option<CommonSize>> {
    let Some(c) = p.get("commonSize").filter(|c| !c.is_null()) else { return Ok(None) };
    let name = str_param(c, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad("woodpart", "`commonSize.name` is required"))?;
    let width = size_param("woodpart", c, "width", false)?.ok_or_else(|| bad("woodpart", "`commonSize.width` is required"))?;
    let thickness = size_param("woodpart", c, "thickness", false)?.ok_or_else(|| bad("woodpart", "`commonSize.thickness` is required"))?;
    Ok(Some(CommonSize { preset_name: name.to_string(), width, thickness }))
}
