# Phase 5: edges and outline analysis

Reading a wood part's outline into a named, ordered list of edges —
`plans/WoodCraft_Implementation_Plan.md` Phase 5, `plans/Wood_Workflow_Scope.md`'s "Edge names".
Pure geometry: nothing here changes a drawing.

## Files added

- `crates/wood/src/edges.rs` — the edge model and the analysis itself:
  - `Side` (`Top`/`Bottom`/`Left`/`Right`) and `Edge` (name, side, flat, start, end, and an
    `arc: Option<Arc>` set only for an edge that is an arc).
  - `polyline_edges(vertices, closed, straightness_tolerance_deg)` — walks the outline's
    segments (reusing `cadcraft_doc::geom::Polyline::segments()`, which already resolves every
    polyline bulge into a line or an arc — this is also how a fillet or chamfer's own arc or
    line is read, with no extra "did this come from a fillet" tracking needed), normalizes the
    loop to a counter-clockwise walk if it was drawn clockwise (by the sign of
    `Polyline::area()`, so naming comes out the same either way), classifies each segment by the
    side its own outward normal faces and whether it is flat (a line within the straightness
    tolerance of horizontal or vertical; an arc is never flat), then names every edge.
  - `circle_edges()` — a circle has no sides to name; returns an empty list.
  - Naming follows the scope doc exactly: edges on the same side get letters in clockwise order
    (`Top A`, `Top B`); an angled edge is named for its side plus "Angle"; a side with one flat
    and one angled edge is named `Top` and `Top Angle A`; a side with two angled edges and no
    flat one is `Top Angle A` and `Top Angle B`.
  - 10 tests: a plain rectangle; the same rectangle drawn clockwise names the same; an L-shape
    letters its two Top edges; one angled edge on a side names plain `Angle`; two angled edges on
    a side get letters; a flat and an angled edge sharing a side; a fillet's rounded (convex)
    corner reads back as an arc, never flattened; a fillet at a concave (inside) corner, same
    result; the straightness tolerance setting lets a near-flat edge count as flat; too few
    segments is refused.
- `crates/engine/src/cmd/wood_edges.rs` — one command, `woodedges` ("Wood Part Edges"): resolves
  the selected or named instance to its part (reusing `wood_query`'s `part_guid_of`), reads the
  one outline entity in that part's block, reads the live straightness tolerance from
  `cadcraft_wood::settings_store`, and returns the part's edges as JSON (`name`, `side`, `flat`,
  `arc`, `start`, `end`).
- `crates/engine/tests/wood_edges.rs` — 6 tests: a rectangle part reads back its four edges;
  `woodedges` defaults to the current selection; refused without exactly one part selected;
  refused on a handle that isn't a wood part; a circle part has no named edges; `woodedges`
  reads the live straightness tolerance setting (a near-flat edge changes from angled to flat
  after `woodsettingsset` raises the tolerance).

## Files changed

- `crates/wood/src/lib.rs`: added `pub mod edges;` and re-exports (`Edge`, `EdgeError`, `Side`).
- `crates/engine/src/cmd/mod.rs`: added `mod wood_edges;` and `v.extend(wood_edges::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended the Ukrainian line for "Wood Part Edges".
- `README.md`: added a WoodCraft section (see "Notes" below — not scoped to this phase alone).

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (10 new edge tests plus the 12 already there)
- `cargo test -p cadcraft-engine --test wood_edges` — 6 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1101 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc` (the edge
  analysis reaches `cadcraft-geom`'s types through `cadcraft_doc::geom`, the existing
  re-export, so no new crate dependency was needed)
- Checked by hand in the running app (release build, control channel): made a rectangular part
  and read its edges back as `Bottom`, `Right`, `Top`, `Left`, all flat, no arcs; made a
  trapezoid part (one leaning side) and confirmed its leaning edge came back as `Right Angle`
  (not flat) while the other three kept their plain names.

## Notes

- **A real bug was caught and fixed while writing this phase's own tests**, not in the
  production code's outward-facing logic but in how edges were first meant to be classified.
  The first version picked a segment's "facing side" by comparing its midpoint's direction from
  the whole shape's centroid — this works for symmetric shapes but gives the wrong side for a
  short diagonal edge near a corner of a lopsided outline (confirmed by hand: on one five-sided
  test shape, a cut corner's midpoint sat closer to the centroid's rightward side than its
  upward side, even though the edge plainly faced up). Replaced with the standard, always-correct
  method: each segment's own outward normal (for a line, the perpendicular to its direction,
  turned to point away from the interior once the loop is walked counter-clockwise; for an arc,
  the direction from the arc's center, flipped for a concave corner). All 10 edge tests, including
  one added specifically to exercise a concave fillet, pass with this method.
- The plan's "arcs from fillet and chamfer stay arcs" turned out to need no special handling at
  all: `Polyline::segments()` (already used by every other geometric feature — hatch boundaries,
  rendering, offset) resolves a zero-bulge vertex to a straight line and a nonzero-bulge vertex
  to an arc, and a fillet's rounded corner is exactly the latter; a chamfer's straight cut is
  exactly the former. There is no "did this come from a fillet" flag anywhere in the codebase,
  and Phase 5 doesn't need one — an arc is an arc regardless of how it got there.
- `plans/Wood_Workflow_Scope.md`'s "Edge names" section does not give the straightness
  tolerance's own default a second time (it's `plans/WoodCraft_Decisions.md` decision #1 that
  sets it to 0 degrees, already wired up in Phase 4's settings); this phase simply reads that
  live setting on every `woodedges` call rather than caching it.
- The README update mentioned above documents every WoodCraft command built across Phases 0-5,
  not just this one — it was committed as its own change on this branch rather than folded into
  the Phase 5 commit, since it describes cumulative state rather than this phase's work alone.
