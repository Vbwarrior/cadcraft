# Phase 10a: Model Warehouse back end

Per `plans/WoodCraft_Implementation_Plan.md` "Phase 10a: Model Warehouse back end" and
`plans/WoodCraft_Decisions.md` item 12: the Model Warehouse is a folder on disk (filled by a
separate C# / WPF importer app, not part of this plan) holding categorized 3D hardware models
(hinges, knobs, drawer slides, and the like). WoodCraft only reads this folder: lists its
categories and models, reads one model's detail, and places a model's flat top-view outline into
the drawing, linked back to the warehouse model. Export copies each placed model's 3D file into
the export folder and lists it in `project.json`'s new `components` array.

No menu placement yet, like every WoodCraft command so far — that is Phase 12.

## New decisions recorded (not in the plan before this phase)

Two things this phase needed were not settled anywhere, so the simplest option was chosen and
written down in `plans/WoodCraft_Decisions.md` items 25 and 26 rather than asked:

- **A placed model is not a wood part.** It gets its own hidden DXF record,
  `WOODCRAFT_COMPONENTS`, separate from `WOODCRAFT_PARTS` — hardware the user bought is not wood
  being cut.
- **The warehouse model's own JSON** (`model.json` in each model's folder) now has a shape:
  `{ "schemaVersion": 1, "name": "...", "outline": [[x,y,bulge],...] }`, the same `[x, y, bulge]`
  convention Document B's part `outline` already uses. The 3D file (`model.dae` preferred,
  `model.obj` fallback) and icon (`icon.png`) are found by plain file name in the model's own
  folder, not referenced in the JSON.

## Files added

- `crates/wood/src/warehouse.rs` — pure, read-only scan of the warehouse folder tree. No
  filesystem writes; no panics on a malformed or hostile folder (a `..` in a path parameter
  cannot escape the warehouse root; an oversized or malformed `model.json` is reported as an
  error for that one model only, never panics or stops the rest of a listing):
  - `categories(root, under)` — the immediate subfolders of the root, or of `under`.
  - `models(root, under)` — every model directly inside a folder (one with its own `model.json`),
    as a `Result` per entry so one bad model doesn't hide the rest.
  - `model_detail(root, path)` — one model's full detail: name, outline, icon path if present,
    and its 3D file's path (`.dae` preferred over `.obj`).
  - 16 tests.
- `crates/wood/src/components.rs` — `WOODCRAFT_COMPONENTS`, a second XRECORD in the drawing's
  named object dictionary, read and written exactly the way `crates/wood/src/store.rs` already
  does for `WOODCRAFT_PARTS` (same chunking, same ASCII escaping, same byte cap), but for placed
  warehouse components instead of wood parts. Keyed by the component's own id (the block name of
  its insert, like a part's GUID), each entry holds `{ model, name }`. 6 tests, including one
  that checks parts and components stay in separate records.
- `crates/engine/src/cmd/wood_warehouse.rs` — four commands:
  - `woodwarehousefolders` — categories at the root or under a given path. No undo (reports
    only).
  - `woodwarehousemodels` — models in a folder, plus any that failed to read, as separate lists.
    No undo.
  - `woodwarehousemodeldetail` — one model's full detail by its warehouse-relative path. No undo.
  - `woodwarehouseplace` — draws the model's outline (as a closed polyline, offset to the given
    point) and blocks it the same way `woodpart` does, then records the link in the component
    store. Geometry and JSON change in one command, so one undo step restores both.
- `crates/engine/tests/wood_warehouse.rs` — 11 tests: folder listing and drilling down, model
  listing (including a malformed model reported, not failing the whole listing), model detail,
  placement (drawn at the right point, linked to the right component, kept separate from
  `WOODCRAFT_PARTS`, undoes and redoes both the geometry and the component record), and refusing
  an unknown model, a model with no outline, and missing parameters without changing anything.

## Files changed

- `crates/wood/src/settings_data.rs` — added `warehouseFolder: Option<String>` to
  `WoodSettings` (default `None`, meaning the per-platform default location).
- `crates/wood/src/settings_store.rs` — added `warehouse_dir()`: the `warehouseFolder` setting
  if set, else a `warehouse` folder next to the settings file.
- `crates/wood/src/export.rs` — `ProjectDocument` gained a `components` field (left out of the
  JSON entirely when empty, so every export before this phase is unchanged) and a new
  `component_entry()` helper for Document B's `components` array entries.
- `crates/wood/src/lib.rs` — exported the two new modules and their public types.
- `crates/engine/src/cmd/wood_settings.rs` — `woodsettingsset` can now set or clear
  `warehouseFolder`, the same way it already sets every other setting.
- `crates/engine/src/cmd/wood_export.rs` — `woodexport` now also reads the component store,
  copies each placed component's 3D file into `models/` (named `<id>-<original file name>`, so
  it cannot collide with a part's own `<guid>.scad`), and lists them in `project.json`'s
  `components` array. A component whose warehouse model can no longer be found is still listed,
  with its model file left `null`, rather than failing the whole export.
- `crates/engine/src/cmd/mod.rs` — registered the new `wood_warehouse` module and its `specs()`.
- `crates/engine/tests/wood_export.rs` — two new tests: a placed component is listed and its
  model file copied on export; an export with no components leaves the `components` key out of
  `project.json` entirely.
- `crates/ui-egui/src/i18n/uk.tsv` — Ukrainian labels for the four new commands.
- `plans/WoodCraft_Part_JSON_Draft.md` — added the "Components (Phase 10a)" section documenting
  the new `components` array.
- `plans/WoodCraft_Decisions.md` — items 25 and 26 (see "New decisions recorded" above).

## Tests

`cargo test -p cadcraft-wood -p cadcraft-engine`: all passed (58 in `cadcraft-wood`, the full
engine suite including the hostile-params and interactive-command watchdogs over every
registered command).
`cargo xtask ci`: all 6 steps passed (fmt, clippy, test, assets, layers, wasm).
