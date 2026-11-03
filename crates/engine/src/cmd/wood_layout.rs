//! WoodCraft Phase 10: layout tools, under the shop names Chris uses at the bench. Per
//! `plans/Wood_Workflow_Scope.md`'s own reuse map ("Measuring, angles, construction lines" are
//! covered by CADCraft's distance/polar entry, polar tracking, XLINE and RAY) and
//! `plans/WoodCraft_Implementation_Plan.md` Phase 10 ("reuse CADCraft first ... build only what
//! is missing"), none of the four tools below needed new geometry or math: each is a thin
//! command that takes the shop's own inputs and calls straight into the existing CADCraft
//! command that already does the work, so the command line, JSON interface and scripts can all
//! say "Tape Measure" / "T-Square" / "Compass" / "Chalk Line" instead of the CAD terms. No
//! `.menu()` path yet, like every other WoodCraft command so far: menu placement is Phase 12,
//! placed by Chris (`plans/WoodCraft_Context.md` §0 "back end first, UI last").
//!
//! - **Tape measure** -> `dist` (two points, reports the distance and angle).
//! - **T-square** -> `xline`, forced to 90 degrees off a named edge of a wood part (reusing
//!   [`cadcraft_wood::edges`], the same read `woodedges` uses).
//! - **Compass** -> `circle`, struck from a center point (per Chris: a compass scribes a circle
//!   or arc, not an angle bisector).
//! - **Chalk line** -> `ray`, snapped straight through two points across a board.
//!
//! Each wrapper calls the target function directly (in-process, same as `run_ellipse` calling
//! `run_isocircle`), not through a nested `Session::execute`, so only one undo snapshot is ever
//! pushed and it carries the wrapper's own label.
//!
//! This file also adds the "marks listed for transfer onto the wood" query named in Phase 10
//! (no document gives it a schema; the simplest reading is the same data `woodmarks` already
//! returns, flattened into one ordered list with a `kind` field, suited for printing a transfer
//! checklist rather than round-tripping JSON).

use cadcraft_geom::Vec2;
use cadcraft_wood::edges::{circle_edges, polyline_edges};
use cadcraft_wood::store;
use serde_json::{Value, json};

use super::wood_edges::Outline;
use super::wood_marks::face_label;
use super::wood_query::part_guid_of;
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodtapemeasure", "Tape Measure", run_woodtapemeasure)
            .params("{p1, p2} -> {distance, angle, dx, dy}: the distance and angle between two points")
            .noundo()
            .transparent(),
        CommandSpec::new("woodtsquare", "T-Square", run_woodtsquare)
            .params("{handle?, edge, at: [x,y]} -> a construction line through `at`, square to the named edge of the selected (or named) wood part"),
        CommandSpec::new("woodcompass", "Compass", run_woodcompass)
            .params("{center, radius} | {center, diameter}: strikes a circle from a center point"),
        CommandSpec::new("woodchalkline", "Chalk Line", run_woodchalkline)
            .params("{base, through}: an unbounded straight line through two points"),
        CommandSpec::new("woodmarkstransfer", "Wood Part Marks for Transfer", run_woodmarkstransfer)
            .params("{handle?} (default: the selection) -> {marks: [{kind: \"Orientation\"|\"Reference\", face, x, y, label}]}: every mark on the part, in one list, for marking onto the wood")
            .noundo(),
    ]
}

/// TAPE MEASURE: the shop name for `dist`. Calls it directly (not through `Session::execute`)
/// so the result and any error come back exactly as `dist` already produces them, with no
/// second undo bookkeeping (this command is itself `.noundo()`, like `dist`).
fn run_woodtapemeasure(s: &mut Session, p: &Value) -> Result<Value> {
    super::inquiry::run_dist(s, p)
}

/// CHALK LINE: the shop name for `ray` (a straight, unbounded line snapped through two points
/// across a board).
fn run_woodchalkline(s: &mut Session, p: &Value) -> Result<Value> {
    super::draw::run_ray(s, p)
}

/// COMPASS: the shop name for `circle`, struck from a center point.
fn run_woodcompass(s: &mut Session, p: &Value) -> Result<Value> {
    super::draw::run_circle(s, p)
}

/// T-SQUARE: a construction line through `at`, square (90 degrees) to the named edge of a wood
/// part. Reads the part's outline the same way `woodedges` does, finds the named edge, takes its
/// direction from `start`/`end`, and asks `xline` for the perpendicular through `at`.
fn run_woodtsquare(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodtsquare";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let edge_name = str_param(p, "edge").map(str::trim).filter(|e| !e.is_empty()).ok_or_else(|| bad(cmd, "`edge` is required"))?;
    let at = point_req(cmd, p, "at")?;

    let doc = s.doc()?;
    let guid = part_guid_of(doc, cmd, hs[0])?;
    let block = doc.block(&guid).ok_or_else(|| bad(cmd, "the part's block was removed"))?;
    let outline = block.entities.iter().find_map(|e| match &e.kind {
        cadcraft_doc::EntityKind::LwPolyline(pl) if pl.closed => Some(Outline::Polyline(pl.vertices.clone())),
        cadcraft_doc::EntityKind::Circle(_) => Some(Outline::Circle),
        _ => None,
    });
    let tolerance = cadcraft_wood::settings_store::get().map_err(|e| EngineError::Other(e.to_string()))?.straightness_tolerance;
    let edges = match outline {
        Some(Outline::Circle) => circle_edges(),
        Some(Outline::Polyline(vertices)) => polyline_edges(&vertices, true, tolerance).map_err(|e| bad(cmd, e.to_string()))?,
        None => return Err(bad(cmd, "the part's block has no outline to read")),
    };
    let edge = edges.iter().find(|e| e.name == edge_name).ok_or_else(|| bad(cmd, "no edge has that name"))?;

    let dir = edge.end - edge.start;
    if dir.len() < 1e-12 {
        return Err(bad(cmd, "the named edge has no direction"));
    }
    let square_to = dir.perp();
    let through = Vec2::new(at.x + square_to.x, at.y + square_to.y);
    super::draw2::run_xline2(s, &json!({ "base": [at.x, at.y], "through": [through.x, through.y] }))
}

/// WOOD PART MARKS FOR TRANSFER: the orientation mark and every reference mark, in one ordered
/// list (orientation first, then reference marks in their stored order), each tagged with its
/// `kind`, for printing or carrying over as a transfer checklist.
fn run_woodmarkstransfer(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodmarkstransfer";
    let hs = targets(s, p)?;
    if hs.len() != 1 {
        return Err(bad(cmd, "select exactly one wood part"));
    }
    let guid = part_guid_of(s.doc()?, cmd, hs[0])?;
    let data = store::get(s.doc()?).map_err(|e| EngineError::Other(e.to_string()))?;
    let part = data.parts.get(&guid).ok_or_else(|| bad(cmd, "the part was removed"))?;

    let mut marks: Vec<Value> = Vec::new();
    if let Some(m) = &part.orientation_mark {
        marks.push(json!({ "kind": "Orientation", "face": face_label(m.face), "edge": m.edge }));
    }
    for m in &part.reference_marks {
        marks.push(json!({ "kind": "Reference", "face": face_label(m.face), "x": m.x, "y": m.y, "label": m.label }));
    }
    Ok(json!({ "marks": marks }))
}
