//! WoodCraft Phase 5: the command that exposes [`cadcraft_wood::edges`] for a drawing's wood
//! part. A part's outline lives in its block's entities (there is exactly one: the closed
//! polyline or circle `woodpart` converted), so this reads that one entity and runs the edge
//! analysis on it, using the drawing's current straightness tolerance setting.

use cadcraft_doc::EntityKind;
use cadcraft_wood::edges::{circle_edges, polyline_edges};
use cadcraft_wood::{Edge, Side};
use serde_json::{Value, json};

use super::wood_query::part_guid_of;
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodedges", "Wood Part Edges", run_woodedges)
            .params("{handle?} (default: the selection) → {edges: [{name, side, flat, arc}]}: the part's outline read into named edges")
            .noundo(),
    ]
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Top => "Top",
        Side::Bottom => "Bottom",
        Side::Left => "Left",
        Side::Right => "Right",
    }
}

fn edge_value(e: &Edge) -> Value {
    json!({
        "name": e.name,
        "side": side_label(e.side),
        "flat": e.flat,
        "arc": e.arc.is_some(),
        "start": [e.start.x, e.start.y],
        "end": [e.end.x, e.end.y],
    })
}

/// WOOD PART EDGES: the selected (or named) part's outline, read into named edges.
fn run_woodedges(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodedges";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let doc = s.doc()?;
    let guid = part_guid_of(doc, cmd, hs[0])?;
    let block = doc.block(&guid).ok_or_else(|| bad(cmd, "the part's block was removed"))?;
    let outline = block.entities.iter().find_map(|e| match &e.kind {
        EntityKind::LwPolyline(pl) if pl.closed => Some(Outline::Polyline(pl.vertices.clone())),
        EntityKind::Circle(_) => Some(Outline::Circle),
        _ => None,
    });
    let tolerance = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?.straightness_tolerance;

    let edges = match outline {
        Some(Outline::Circle) => circle_edges(),
        Some(Outline::Polyline(vertices)) => polyline_edges(&vertices, true, tolerance).map_err(|e| bad(cmd, e.to_string()))?,
        None => return Err(bad(cmd, "the part's block has no outline to read")),
    };
    Ok(json!({ "edges": edges.iter().map(edge_value).collect::<Vec<_>>() }))
}

pub(super) enum Outline {
    Polyline(Vec<cadcraft_doc::geom::PolyVertex>),
    Circle,
}
