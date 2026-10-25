# Phase 8: Export

Writes Document B (`plans/WoodCraft_Part_JSON_Draft.md` §4): a folder with `project.json` and
one SCAD file per part under `models/`. This is the last WoodCraft phase that touches the part
JSON shape — the plan says to "freeze the JSON format here" before the WP importer is built.

## Files added

- `crates/wood/src/export.rs` — pure data-to-text, no drawing and no filesystem access:
  - `Outline` — a part's outline, already read from the drawing in block-local coordinates: a
    polyline's `[x, y, bulge]` vertices, or a circle's center/radius, converted on demand to the
    same two-point bulge-1 path CADCraft's own polylines use for a half circle (so Document B's
    `outline` field is always one shape, per the draft's "Circles and arcs are exact").
  - `part_entry()` — builds one part's entry in Document B's `parts` array: `id`, `name`,
    `revision`, `instances`, `model`, `outline`, and `properties` (the same JSON as the part's
    Document A entry, with `name`/`revision` removed since they sit alongside, not inside,
    per the draft's own example).
  - `scad_for_part()` — the SCAD text: the outline linearly extruded to the part's `Thickness`,
    then a `difference()` subtracting a cylinder per hole position (depth from the hole's own
    `Depth`, or the full thickness for `Hole-Through`, drilled from the stated `Face`) and a box
    per cut position (`Length` × `Width` × `Depth`, or the full thickness if the cut has none).
    A `Cut-Rabbet` (which the property sheet gives no position) is cut once at the origin; WP
    repositions it against the named edge on import. This is literal, axis-aligned geometry
    from the outline and the stored numbers — not a finished router-profile model — matching
    the scope doc's own description of the SCAD's job ("subtracts each hole and cut, so a
    rendered part has real holes") without inventing anything beyond it.
  - `ProjectDocument`/`ProjectInfo` — the `project.json` shape: `schemaVersion`, `units`,
    `propertyListVersion`, `project.name`, `parts`.

- `crates/engine/src/cmd/wood_export.rs` — the `woodexport` command (`{path, projectName,
  openscadCheck?}`):
  - Reads the drawing's part JSON and the live WoodCraft settings, finds each part's outline by
    reading its block (same lookup `woodedges` and `wood_warnings.rs`'s `part_dimensions` use —
    there is exactly one closed polyline or circle per block, the one `woodpart` converted), and
    counts instances by scanning model-space Inserts (`wood_query::instances_of`, the same "no
    separate array object, just count the Inserts" approach Phase 3 already uses).
  - Refuses to write anything while `blockExportOnFastenerWarnings` is on and any current
    fastener warning (across every part) is not yet acknowledged this session — returns `{path,
    parts: 0, blocked: [warning ids]}` instead. Joint fit and build-size warnings never block
    export; only fastener warnings do, per `Wood_Workflow_Scope.md`'s "A WoodCraft setting
    blocks the export while fastener warnings exist, because they would damage the model."
  - On success, writes `project.json` (pretty-printed) and `models/<guid>.scad` per part under
    `path`, then clears `Session::acknowledged_wood_warnings` — the override from
    `woodwarningacknowledge` is for this export only, exactly as the scope doc says ("An
    override lasts for one export only. The next export asks again"). This is the first place
    that field is reset; Phase 7's own record flagged this as left for Export to wire in.
  - `openscadCheck: true` (optional) runs `openscad -o <guid>.stl <guid>.scad` as a separate
    process per part (`Wood_Workflow_Scope.md`'s own wording — "not embedded") and reports
    `{openscad: {ok, parts: [{part, ok, message?}]}}` alongside the normal result. OpenSCAD
    missing from the machine is reported per part as `ok: false` with a message, never treated
    as an export failure — the JSON and SCAD files are already written; this is a courtesy
    check only, and no new crate dependency was added for it.
  - `.noundo()`, matching every other file-writing WoodCraft command (`woodsave`, `woodopen`).

- `crates/engine/tests/wood_export.rs` — 13 tests: writes `project.json` and one SCAD per part;
  the outline is the part's rectangle in order; a three-instance Copy Array exports as one part
  entry with `instances: 3`; export is blocked while an unacknowledged fastener warning exists
  and the setting is on, and nothing is written in that case; acknowledging the warning lets
  one export through; the override does not carry over to the next export; the setting being
  off never blocks export; joint fit and build-size warnings never block export even with the
  setting on; a hole and a cut both appear in the generated SCAD; an empty project still writes
  a valid (empty) `parts` array; missing `path`/`projectName` are refused.

## Files changed

- `crates/wood/src/lib.rs`: added `pub mod export;` and re-exported `Outline`, `OutlinePoint`,
  `ProjectDocument`, `ProjectInfo`, `part_entry`, `scad_for_part`.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_export;` and `v.extend(wood_export::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended the Ukrainian line for the new command's label
  ("Export Project...").

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 41 passed (7 new export-module tests, 34 already there)
- `cargo test -p cadcraft-engine --test wood_export` — 13 passed
- `cargo test -p cadcraft-engine` (the whole crate) — all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed (the new label's translation found)
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings beyond
  the pre-existing, unrelated filesystem hard-linking notice `/mnt/d` prints for every crate
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- `cargo xtask wasm` — all 12 crates pass, including `cadcraft-wood` and `cadcraft-engine`
- `cargo fmt` (scoped to the touched crates) — applied, then verified clean with `--check`
- `cargo xtask ci` — run before committing; see the commit for the result
- Checked by hand, headless, with `cadcraft-cli run --sample --cmd '...'`: drew a rectangle,
  ran `woodpart`, `woodholeadd` (a `Hole-Through`), then `woodexport` — confirmed the written
  `project.json` carries the correct outline, instance count and properties, and the written
  `.scad` file's `difference()` correctly subtracts the hole's cylinder from the extruded
  outline. Also confirmed `openscadCheck: true` degrades gracefully (reports `ok: false` with
  a "could not run openscad" message per part, without blocking the export) on this machine,
  which has no `openscad` binary installed.

## Notes

- **The session restarted mid-project** (WSL `/mnt/d` lost its mount ownership again, same
  class of issue as the Phase 0 restart — see `plans/RESTART_NOTES.md`, which was about an
  earlier, already-resolved incident). This time the fix was simpler: the new session's mount
  already had correct `chris:chris` ownership and the toolchain was intact, so no `wsl
  --shutdown` was needed — just a confirmation build before resuming. Phases 0 through 7 were
  already merged to `main`; this phase branches from `main` as `feature/wood-phase-8-export`.
- `wasm32-unknown-unknown` was not installed in the fresh environment (`rustup target add
  wasm32-unknown-unknown`); this is a one-time toolchain setup, not a code issue, and is
  unrelated to the wood crate's own wasm-friendliness (it already had none of the egui/wgpu
  dependencies that would make it wasm-unsafe).
- The SCAD geometry is deliberately literal: holes are drilled straight down/up from the
  stated face, and cuts are axis-aligned boxes at their stored position and size. Nothing here
  tries to model a router's actual profile shape (round-over curvature, chamfer angle, and so
  on from `Edge profile`/`Bevel`/`Miter`) — those stay as properties for WP to read, per the
  scope doc's own split (WoodCraft stores the properties; WP/MM do the 3D work). If Chris wants
  the SCAD itself to carve a profile shape rather than leave it flat, that is a scope change to
  flag, not something assumed here.
- `instances_of` (Phase 2/3's own helper for counting Copy Array instances by scanning
  model-space Inserts) is reused as-is rather than duplicated — it was already `pub(super)` to
  the `cmd` module, so `wood_export.rs` calls it directly.
