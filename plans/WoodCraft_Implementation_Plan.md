# WoodCraft implementation plan (Oct 23, 2026, rev 2)

The working order for WoodCraft (WC), the Rust fork of CADCraft. It goes with `WoodCraft_Context.md` (read that first), `WoodCraft_Decisions.md` and `Workshop_Decisions.md` (every decision made, so nothing is asked twice) and `Wood_Workflow_Scope.md` v3. WorkShop Pro (WP) and Model Maker (MM) work is listed at the end only as hand-off points.

**Replaces for WC:** the Designer parts of `Implementation_Plan.md` (Oct 21). Those phases are retired by the three-program split.

## Build order rule (Chris, Oct 23)

**Back end first, UI last.** Phases 1 to 11 build the logic and wire every action as a command (CADCraft's "everything is a command" design), so each one can be run from the command line, the JSON interface and scripts, and tested without any screen. **No buttons, panels, menus or layout until Phase 12.** Phase 12 is Chris's: he decides where each button and panel goes, and the wiring is already done.

To make that possible, the back end must also expose what a panel will need as commands that return data (for example a part's properties, the warnings list, the settings), so the UI only calls them.

## Rules for every phase

- Work through the phases in order without stopping for questions. When something is unclear, take the default in `WoodCraft_Decisions.md`; if it is not there, choose the simplest option, write the choice and the reason in the Decisions file, and carry on.
- Stop only for these: a merge to `main` that Chris has not asked for, a push, deleting a branch or a user's files, a new crate with a non-permissive licence, or a failing test that you cannot fix. Report each in plain words.
- After each step run the tests for the crate you changed. Run `cargo fmt` and `cargo xtask ci` before every commit. Do not move on while a test fails.
- **One branch per phase** (`feature/wood-phase-<N>-<short-name>`), stacked in order. Chris validates a phase and says "merge phase N"; only then merge it into `main`, in phase order. Unvalidated phases stay on their own branches. Never `main` directly, never push. Details in `WoodCraft_Context.md` section 8.
- Smallest possible change to CADCraft files. New wood code goes in `crates/wood` or `crates/engine/src/cmd/wood.rs` (split into more files there when it grows).
- No `unwrap`, `expect` or `panic` in non-test code. Hostile input never crashes.
- Every wood command: changes JSON and geometry in one step, has a test for undo and redo, refuses bad input without changing anything, and has a `uk.tsv` line for its label.
- No renaming, refactoring or restyling of existing code. Property names come from the sheet exactly.
- After each phase, write `changes/phase-N.md` listing every file added or changed. Plan documents live in `plans/`.
- After each phase, give Chris a short plain summary: what now works, how to try it from the command line, what the tests show.

## Phase 0: Baseline

1. Unzip `wood_slice1b.zip`, make the edits in `changes/phase-0.md` (including the `xtask/src/layers.rs` row).
2. Run `cargo test -p cadcraft-wood`, `cargo test -p cadcraft-engine --test wood_makepart`, `cargo test -p cadcraft-engine --test wood_store_roundtrip`, `cargo test -p cadcraft-ui-egui i18n`.
3. Fix any compile or test failure. Likely trouble spot: the first Save of a brand-new drawing not writing the XRECORD (first test in `wood_store_roundtrip.rs`). If so, change how a new file gets its named object dictionary, in `store::put` only.
4. `cargo fmt`, then `cargo xtask ci`. Commit on `feature/wood-phase-0-baseline`.

**Check:** the four test commands and `cargo xtask ci` pass.

## Phase 1: `.wood` files as commands

1. Commands for new, open and save of a wood project: new makes an empty drawing that already holds an empty part JSON; save writes DXF text (`write_dxf`) with the `.wood` extension; open accepts `.wood` only and refuses plain DXF with a clear message.
2. Standard 2D Open and Save stay exactly as they are.
3. Tests: save and reopen keeps geometry and JSON; opening a plain DXF as a wood project is refused.

## Phase 2: Working copy and query commands

1. Selecting an instance reads its block name (the GUID) and finds the part. Commands return a part's properties, the list of parts, and the display name for a block.
2. A `display_name(block)` helper: wood part blocks give the display name, every other block gives its own name. The GUID is never shown.
3. Commands to change Thickness, Grain direction and display name (undoable).
4. A cap of 8 MB on the stored JSON, with an error instead of a crash, and a test with a large file.

## Phase 3: Copy rules (commands)

Each with undo and redo tests:
1. **Copy:** new GUID, copy of the entry, name `<name> -Copy`.
2. **Copy Array:** same GUID, another instance.
3. **Delete:** removing the last instance removes the entry.
4. **Split Array**, **Explode Array**, **Remove From.** Split Array takes the picked items, refuses if any is outside the array, and returns which items would change. The reddish highlight itself is Phase 12.
5. Editing only some instances is refused with a message that points to Split Array.

## Phase 4: Settings, presets and lists

1. Settings file `%APPDATA%\WoodCraft\settings.json`, read and changed through commands.
2. Settings: straightness tolerance (degrees, default **0**, meaning any angle at all is an angle), units (`in` or `mm`), minimum joint spacing, tolerance per joint type, three build-size limits (no default; the check is skipped until all three are set), block export on fastener warnings (default off), git timer minutes (default 1).
3. Common size presets: add, rename, delete; seeded with standard lumber sizes (1x2 through 2x8 at real inch sizes). Choosing one when making a part fills width and thickness.
4. Lists: joint type, edge profile (with Size), glue, screw, hardware, end treatment; seeded from the `Lists` tab of `WoodCraft_Property_Names.xlsx`.

## Phase 5: Edges and outline analysis

1. Read a part's outline into a list of edges: flat or angled, which side it faces. An edge is flat only when exactly horizontal or vertical (within the straightness setting, default 0, and ignoring rounding noise smaller than 1e-9).
2. Name edges by the scope's rules (Top, Top A, Top Angle, Top Angle A, ...).
3. Arcs from fillet and chamfer stay arcs; ellipses and splines become short straight segments.
4. Tests: rectangle, L-shape, one angled edge, two angled edges, a flat plus an angled edge on one side.

## Phase 6: Property commands

Add the properties from `WoodCraft_Property_Names.xlsx` in groups. For each group: a command, the JSON entry, a simple marker in the drawing, a query command, tests. Names exactly as in the sheet.

1. Orientation mark (V) and Reference marks.
2. Holes (Through, Blind, Counterbore, Countersink, Pocket-Screw, Shelf-Pin, Dowel). Positions that differ only by location share one entry with an `At` list.
3. Cuts (Dado, Groove, Rabbet, Slot, Notch, Pocket) and dado snap planes.
4. Joints (Mortise, Tenon, Joint type, Biscuit) and End treatments.
5. Edge profile, Bevel, Miter.
6. Fasteners and hardware (Glue, Screw, Nail, Dowel, Hinge, Drawer slide, Drawer Pull).

## Phase 7: Warnings engine

1. The warnings list as queryable state: appears when found, disappears when fixed.
2. Fastener warnings, joint fit warnings, build-size warnings (smallest, middle, largest against the three limits).
3. Acknowledge and override commands, per export only, nothing saved in the file.
4. The block-export-on-fastener-warnings setting.

## Phase 8: Export

1. `project.json` and `models/<guid>.scad` per part, exactly as document B in the JSON draft.
2. SCAD builds the part from the outline and subtracts every hole and cut.
3. OpenSCAD runs as a separate process (`openscad -o out.stl in.scad`) as an optional check that each SCAD renders. No new dependency.
4. Export respects the block setting and the per-export override.
5. Tests on a sample project; the JSON is checked against the draft's rules (full-name keys, no empty values).

**Freeze the JSON format here** before the WP importer is built.

## Phase 9: History (git)

1. Dirty flag set by commands.
2. Background timer commits if dirty and no command is running. The message lists the commands since the last commit. Commit on exit. Adding a new object commits at once and restarts the timer.
3. Local commits only, no push. The timer never blocks the program.
4. Tests for the dirty and timer logic with a fake clock.

## Phase 10: Layout tools (commands)

Reuse CADCraft first: distance and polar entry, polar tracking, XLINE and RAY. Build only what is missing: tape measure, T-square, compass, chalk line. Marks tied to a part and listed for transfer onto the wood.

## Phase 10a: Model Warehouse back end

See `plans/Warehouse_Plan.md` and `plans/Warehouse_Model_JSON_Draft.md` for the full design (masters, models and meshes; family/revision history; categories with typed search keys; icon views and layouts). Both are drafts: the JSON format is not frozen until Chris settles `Warehouse_Model_JSON_Draft.md`'s open items (step W0). Nothing below is built against that format until then.

1. Setting for the warehouse folder (default `%APPDATA%\Workshop\warehouse\`).
2. Commands that return the folder tree (categories), the models in a folder (name, icon path, size) and one model's details.
3. A command that places a model: puts its flat top-view outline (from the model's JSON) in the drawing as a part-like block, with a link to the 3D model.
4. Export copies the models the project uses into the export folder and lists them in `project.json` (the JSON draft gets a `components` list; record the change in `changes/` and `plans/`).
5. Tests with a small sample warehouse folder.
6. A search command: each search key has a From/To pair — one filled means equals (within a small tolerance), both filled means an inclusive range, both empty means the key is ignored. Compared in millimetres. No match returns the closest entries, marked "closest". Shows the highest revision of each family. The full rule is in `Warehouse_Plan.md`'s "Search" section; do not restate or diverge from it here, since the same rule is also implemented in WorkShop Pro and Model Maker and must match exactly.
7. The link to a placed model, and each entry in `project.json`'s `components` list, must store the model's `family` and `revision` (not just its warehouse path).
8. A query command that reports whether a placed model has a higher revision available in the warehouse. It only reports — it never switches the placed model's revision by itself.
9. A missing model gives a clear "model not found" message. It never crashes.

The importer that fills the warehouse is a separate C# / WPF app (see `Workshop_Decisions.md` item 14 and `plans/Warehouse_Plan.md`), not part of this plan. It is the only writer; WoodCraft, WorkShop Pro and Model Maker only read the warehouse.

## Phase 11: Branding and housekeeping

1. Remove the ArtCraft brand (`docs/brand/`); keep `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`.
2. Attribution text for CADCraft and OpenSCAD (the splash screen itself is Phase 12).
3. README for WoodCraft. Keep the upstream merge route documented. Keep `AGENTS.md` and `CLAUDE.md`; add the one-line link to the Context doc.

## Phase 12: UI (last, with Chris)

Only now do buttons, panels, menus and layout get built, and Chris chooses where each goes. Everything below already works as a command by this point.

1. Wood project entry: how the user starts a wood project (Chris decides; the default is File menu entries "New Wood Project" and "Open Wood Project").
2. Wood properties palette: shows and edits the selected part through the Phase 2 and 6 commands.
3. Warnings list panel with acknowledge and override.
4. Settings dialog; Common size picker; list managers.
5. Display-name substitution in the block list and command prompts.
6. Split Array reddish highlight and confirm.
7. Model Warehouse: a button that shows the top folders as buttons, drilling down folder by folder, until the models show as thumbnails.
8. Wood tool group in Tool Sets; menu entries; shortcuts; splash screen with attribution.
9. Chris places and arranges everything; Claude Code wires each control to its command.

## Dependencies

- Phase 0 before everything.
- Phase 1 before 2; 2 before 3.
- Phase 4 before 6 (lists feed the property commands).
- Phase 5 before 8 (the export needs the outline and edge names).
- Phase 6 before 7 and 8.
- Phase 10a after Phase 8 (the export carries the components list).
- Phases 9 and 10 can move any time after Phase 1.
- Phase 12 last.

## Hand-off points to the other programs (separate plans)

- **WP importer:** starts after the JSON format is frozen (end of Phase 8). Reads `project.json` and the SCAD files into Parts and BOM lines. Changes in WP never change the WC original.
- **WP assembler and 3D viewer, light 2D composer, fresh code-first database:** WP plan. Not blocking WC.
- **MM:** imports a part's SCAD when Chris wants it for 3D printing. No change to MM's toolset.
- The old Workshop docs (`Master_Plan.md`, `Designer_Plan.md`, `Designer_Spec_v2.md`, `Checklist.md`, `Open_Questions.md`) need to be revised against the three-program scope. Separate job.
