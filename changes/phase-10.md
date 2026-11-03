# Phase 10: Layout tools (commands)

Per `plans/WoodCraft_Implementation_Plan.md` "Phase 10: Layout tools (commands)" and
`plans/Wood_Workflow_Scope.md`'s reuse map: measuring, angles and construction lines are already
covered by CADCraft's own `dist`, `xline`, `circle` and `ray`. Phase 10 builds no new geometry —
each shop tool is a thin wrapper that takes the shop's own inputs and calls straight into the
existing CADCraft command, so the command line, JSON interface and scripts can say "Tape
Measure" / "T-Square" / "Compass" / "Chalk Line" instead of the CAD terms. Also adds the
marks-for-transfer listing named in the same phase.

No menu placement yet, like every WoodCraft command so far — that is Phase 12, placed by Chris.

## Files added

- `crates/engine/src/cmd/wood_layout.rs` — five new commands:
  - `woodtapemeasure` — the shop name for `dist`. Calls `run_dist` directly; reports distance and
    angle between two points. No undo (reports only, like `dist`).
  - `woodtsquare` — a construction line through a given point, square (90 degrees) to a named
    edge of a selected wood part. Reads the part's outline the same way `woodedges` does, finds
    the named edge, takes its direction, and calls `xline` (`run_xline2`) with the perpendicular
    direction through the point. Refuses unless exactly one part is selected, and refuses an
    unknown edge name.
  - `woodcompass` — the shop name for `circle`, struck from a center point. Calls `run_circle`
    directly.
  - `woodchalkline` — the shop name for `ray` (an unbounded straight line through two points).
    Calls `run_ray` directly.
  - `woodmarkstransfer` — reads the part's orientation mark and reference marks (the same data
    `woodmarks` returns) and flattens them into one ordered list (orientation first, then
    reference marks in their stored order), each tagged with a `kind` field, for printing or
    carrying over as a transfer checklist onto the wood. No undo (reports only).
  - Each wrapper calls its target function in-process (not through a nested
    `Session::execute`), so only one undo snapshot is pushed, carrying the wrapper's own label —
    the same pattern `run_ellipse` uses when calling `run_isocircle`.

- `crates/engine/tests/wood_layout.rs` — 14 tests: each tool behaves like the command it wraps
  (distance/angle math, drawing an unbounded line, striking a circle, refusing a non-positive
  radius, undo/redo), the T-square refuses more than one part selected and an unknown edge name,
  and the marks-transfer listing is empty for a new part, lists orientation then reference marks
  in order, and leaves the drawing unchanged.

## Files changed

- `crates/engine/src/cmd/mod.rs` — registered the new `wood_layout` module and its `specs()`.
- `crates/engine/src/cmd/draw.rs` — `run_circle` and `run_ray` made `pub(crate)` so
  `wood_layout` can call them directly (were private to the module).
- `crates/engine/src/cmd/inquiry.rs` — `run_dist` made `pub(crate)` for the same reason.
- `crates/engine/src/cmd/wood_edges.rs` — the `Outline` enum made `pub(super)` so `wood_layout`
  can read a part's outline the same way `woodedges` does.
- `crates/engine/src/cmd/wood_marks.rs` — `face_label` made `pub(super)` so `wood_layout` can
  label faces the same way `woodmarks` does.
- No renames; only visibility widened on existing private items, so nothing about `dist`,
  `circle`, `ray`, `Outline` or `face_label` changed for their existing callers.

## Tests

`cargo test -p cadcraft-engine --test wood_layout`: 14 passed.
`cargo xtask ci`: all 6 steps passed (fmt, clippy, test, assets, layers, wasm).
