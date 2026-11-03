# Wood workflow scope: three independent programs (draft v3, Oct 22, 2026)

Replaces the Oct 21 draft. It does not change `Designer_Spec_v2.md`, `Designer_Plan.md`, `Master_Plan.md`, `Checklist.md`, `Implementation_Plan.md` or `Open_Questions.md`. Section 10 lists what would change in those files once this is approved. Nothing is implemented without Chris's go-ahead.

## 1. The three programs

| Program | Job | Stack |
|---|---|---|
| **Model Maker (MM)** | Design 3D-print models through OpenSCAD. Standalone. Toolset settled. No change. | C#/WPF |
| **WorkShop Pro (WP)** | Manage real wood projects. Imports parts, shows them in 3D (view only), holds the assembler, and has a light 2D composer for cutouts and cuts on boards and sheets with blemish marks. No CAD. | C#/WPF |
| **WoodCraft (WC)** | Independent 2D wood drafting with the layout tools. A fork of storytold/cadcraft. No 3D. | Rust/egui |

- **No cross talk.** No shared database. WC exports files. WP or MM pull them in on their own terms. Changes made in WP never change the WC original.
- **WoodCraft repo:** `~/dev/CadCraft` (inside WSL Ubuntu), fork `Vbwarrior/cadcraft`, upstream `storytold/cadcraft`, working branch `feature/wood-workflow`. Its own repo.
- **Standard 2D CAD stays untouched.** Wood rules live in their own module and tab.
- **Browser (WASM) build:** ignored.

## 2. Terms

- **Model space:** the CAD side. A part's own flat shape lives here. Axis letters are internal and never shown to users.
- **Real-world space:** the shop view, as if standing in front of the piece. The assembler works only here.
- **Directions:** Left / Right (across the front, X), Up / Down (vertical, Y), Push-in / Pull-out (depth, Z).
- **Orientation V:** the bridge between the two spaces (section 3).

## 3. WoodCraft drafting side

### Parts
- Each part is a closed outline plus properties. Thickness is always required and the wood add-on enforces it. Closed outlines only. Open lines are layout lines.
- Every WoodCraft part is a **finished part**. Material, rough stock size and finish are set in WP when the part is imported, not in WoodCraft.
- Each part has a stable **GUID**.
- **Faces:** face A is always the appearance face and face B is the back. For holes, Face is the face you drill from.
- **Thickness** is stored in the JSON only. The drawing is a flat outline, so it is not shown on screen unless a label or the properties palette displays it.
- **Common size:** a preset (width and thickness) picked when drawing a new part, so you only add the length by dragging or typing. The part stores the preset's name, width and thickness. WP reads the name to build the BOM. The preset list lives in WoodCraft.
- **Rounded and chamfered corners** are part of the outline, drawn with CADCraft's fillet and chamfer commands. There is no corner property.

### Edge names
- An edge is named by the direction its outside faces: `Top`, `Bottom`, `Left`, `Right`. Edges of the same kind get letters in clockwise order from the top-left (`Top A`, `Top B`).
- An angled edge is named for the side it faces plus "Angle" (`Top Angle`). One flat edge and one angled edge on the same side are `Top` and `Top Angle A`. Two angled edges and no flat one are `Top Angle A` and `Top Angle B`.
- The limit for how close to straight counts as flat is not set.

### Orientation V
- A capital V whose point touches the part's reference edge: "this side up, V points at the reference edge."
- Placed by hand on face A (front, cyan) or face B (back, dim gray). A marker only.
- **Default:** face A, top-left corner, pointing at the top edge. It is stored only when moved.

### Marks
- Saved points tied to a part, located from a named end and edge. They can be snapped to and listed for transfer onto the wood. Knobs, handles and hinges use marks.
- Reference marks are an array of zero or more elements, each with its own face, X, Y and label.

### Dado and groove planes
- Marked on face A or B with position, width and depth. All three required.
- Not modeled as a groove. It becomes a **snap plane** (flat, no thickness, belonging to its parent, sitting inside the parent at the groove floor).
- Two kinds: **Dado** (open channel, slides freely) and **Tenon-style groove** (snug, centered, moves only in and out).

### End treatments
- Each board has End A and End B. Options: None, Tenon, Dovetail, Split tenon, Tapered, Tongue, Groove, Mitered, Half-lap, Half-lap pin. A treatment never changes total length.

### Cuts
- Drill, saw and router operations and cut shapes (round hole, slot, rectangle pocket, dado, rabbet), each with face, depth and angle where it applies.

### Layout tools
- Tape measure, T-square, compass and chalk line reuse CADCraft's distance and polar entry, polar tracking and construction lines first. Only missing pieces get built.

## 4. How properties are stored

- **Where:** the JSON is stored as an XRECORD under the key `WOODCRAFT_PARTS` in the DXF's named object dictionary, in a DXF with a custom WoodCraft file type, extension `.wood` (decided Oct 22). The file content is DXF; reading works by content, and the wood Save writes the DXF itself and names it `.wood`. The wood workflow opens only its own file type. The standard 2D set keeps its normal files and does not open wood files.
- **Tested:** the XRECORD survives open and save and a second save (`crates/io/tests/woodcraft_xrecord_roundtrip.rs`, passed). It lives in the drawing's "foreign" data in memory, and undo and redo restore it with the geometry (`crates/engine/tests/woodcraft_xrecord_undo.rs`, passed). No change to CADCraft's reader, writer or drawing struct is needed.
- **Link:** the block name is the part's GUID. Nobody edits it. The JSON is keyed by GUID.
- **Display name:** the user sees and renames only the display name, which is stored in the JSON as GUID → name. Everywhere WC shows a block name for a wood part it shows the display name instead. Plain 2D blocks keep their own names.
- **Working copy:** on open (new or existing), the JSON is read into an in-memory working copy of every part's properties. All wood commands change that copy and the geometry together, in one step, and save writes both. Instance-specific data (position, rotation) stays with the drawing's own block instance, not in the JSON.
- **Referencing a change:** select the instance, read its block name (the GUID), look the GUID up in the working copy, change the entry.
- **Delete:** deleting the last instance of a GUID removes its entry.
- **Long JSON:** a long text goes in several XRECORD groups, as CADCraft does for its own data.
- **Property list:** the keys are the full property names (no codes), listed with their parameters in `WoodCraft_Property_Names.xlsx` (33 properties). A key never changes after the first release. WC writes the list to `%APPDATA%\WoodCraft\property-codes.json`, with a version number. WP may check it for a newer version on startup and keep every version it has loaded. Each part stores the version it was built under. Whether WP still needs this file, now that keys are full names, is an open item.
- **Value rules:** one `units` value (`in` or `mm`) per document, all lengths plain decimals, angles in degrees, face `A` or `B`. Single-value properties: Thickness, Grain direction, Common size, Orientation mark. Everything else is an array of entries. Entries that differ only by position share one entry, with an `At` list of positions.
- **Lists** (end treatment, joint type, screw type, glue type, hardware type, edge profile) live in WoodCraft settings as drop-downs and each part stores the chosen text. WP has its own lists and can set or change those values on import, and its changes never change the WoodCraft original. A starting set is on the `Lists` tab of the sheet.
- **Edge profile** is a list value with a Size (for example a 3/4 round-over). Bevel and Miter stay as separate angle properties.
- **Nails** are defined by diameter and length.
- **Revision:** each part carries a counter that goes up by one each time its entry changes, so WP can tell a changed part from an unchanged one on re-import.

### Copy and array rules
- **Copy:** new GUID and its own copy of the entry. An independent part. The display name becomes `Leg -Copy` until the user changes it. Two parts may share a display name, since the GUID is the key.
- **Copy Array:** same GUID and shared entry. Another instance with its own location.
- **Instances:** copies of one GUID share one entry. Each copy's position and rotation belong to its own block instance in the drawing, not to the JSON. The export carries the count only.
- **Editing all instances together:** changes the shared entry. They stay one part.
- **Editing only some instances:** refused. The user must run Split Array first.
- **Split Array:** the user picks items, which turn reddish until confirmed. The confirmed items get a new GUID and a copy of the entry. If any selected item is outside the array, the command is cancelled.
- **Explode Array:** every instance becomes its own part with its own GUID and entry.
- **Remove From:** the selected items leave the array and each becomes its own part.
- **Add To:** dropped, because Copy Array covers it.

### History (git)
- Local commits only. No push.
- Commands set a dirty flag. A background timer (default one minute, a setting) checks it. If dirty and no command is in progress, it takes a snapshot, writes the file and commits. The commit message lists the commands since the last commit. WC also commits on exit.
- Adding a new object commits immediately and restarts the timer.
- Ordinary geometry changes commit the same way.
- A crash can lose up to a minute. Accepted.

### Warnings
- WoodCraft keeps a running **warnings list**, like the Error List in Visual Studio. A warning appears when found and disappears when fixed.
- Three kinds go in it: fastener warnings (for example a fastener that goes through a part), joint fit warnings (too tight or too loose against the tolerance settings) and build-size warnings.
- A WoodCraft **setting** blocks the export while fastener warnings exist, because they would damage the model. It can also be left off.
- Any warning can be acknowledged and overridden when the user thinks it is wrong. It stays in the list, marked as bypassed, and does not block the export. An override lasts for one export only. The next export asks again. Nothing about it is saved in the file. This replaces the earlier "acknowledge once per axis" rule for build-size warnings.
- Warnings are never exported. WP reads values only.

## 5. WoodCraft export (to WP and MM)

- A folder with `project.json` and one SCAD file per part under `models/`. The format is in `WoodCraft_Part_JSON_Draft.md`.
- The export carries a project name (no WP project ID), and for each part: GUID, display name, revision, instance count, SCAD file name, the outline as a closed `[x, y, bulge]` path taken from the drawing (arcs exact, ellipses and splines as short straight segments), and the properties.
- The SCAD file builds the part from the outline and subtracts each hole and cut, so a rendered part has real holes.
- Fasteners and hardware are exported and WP turns them into lines in its materials list.
- No placement list. The assembler is in WP, so WP produces placements.
- OpenSCAD is called as a separate process (`openscad -o out.stl in.scad`), not embedded.

## 6. WorkShop Pro

- **Importer** turns the export into Parts and BOM lines.
- **3D viewer and assembler** (view only, built in WP). Parts snap edge to edge or face to face, the contact axis locks, then the user slides and types values (Left/Right, Up/Down, Push-in/Pull-out) measured from the V reference edges. Flush and Center helpers.
- **Fit checks** (warnings only): tenon in slot against minimum spacing and per-joint tolerance. Skipped until first setup.
- **Light 2D composer:** moves cutouts and cuts around on boards and sheets to avoid blemishes. Extends Sheet Layout and `CutoutOutlineEditorWindow`.
- **Fasteners and glue** become BOM lines. Fasteners view, red clearance warning, cylinder object and imported components carry over from the earlier plan.
- **Database:** a fresh code-first EF Core database, away from MVVM, with IDs owned by code. The old schema is a reference only. Data is written to local SQLite in AppData and backed up to SQL Server. The renderer is not built yet and is designed to use the per-part SCAD.
- **Quantity:** the parts-list quantity is a count. One shelf can be used several times.

## 7. Settings (shop-wide)

- **Minimum joint spacing:** per side, set at first setup, editable afterward.
- **Tolerance limit per joint type:** extra gap above the minimum. No built-in defaults.
- **Build size limits (three sizes):** a physical-opening rule. Compare sorted: smallest dimension against the smallest limit, middle against middle, largest against largest. All three must fit. Warning only, never a hard stop. It is a build-size warning in the warnings list and follows the per-export override rule (see Warnings).
- **Block export on fastener warnings:** on or off.
- **Lists and presets:** Common size presets, edge profiles, joint types, glue types, screw types and hardware types are managed here.

## 8. Licensing and attribution

- CADCraft is MIT OR Apache-2.0. Keep `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE`. Remove the ArtCraft brand (`docs/brand/`). `AGENTS.md` and `CLAUDE.md` in the fork can be removed (Chris's call).
- Give attribution to CADCraft and OpenSCAD on a splash screen and in the About docs, with links. Not legal advice.

## 9. Reuse map

| Need | Reuse |
|---|---|
| Measuring, angles, construction lines | CADCraft distance and polar entry, polar tracking, XLINE and RAY |
| Automation | CADCraft CLI, MCP server, LISP, `.scr` scripts |
| 3D display | HelixToolkit |
| Geometry, red check, end rule, fit checks | `Workshop.Designer.Core`, reworked for the tolerance settings |
| Layout on wood, flaw marks | Sheet Layout, `CutoutOutlineEditorWindow`, `Parts.GeometryData` |
| Notes, hardware and glue | `Note`, Hardware catalog, `Material` fields |

## 10. Retired from the earlier plan

- The 3D viewer and assembler no longer shared as a standalone C# window with WP.
- DXF plus CSV handoff. Replaced by the JSON and SCAD export.
- Designing inside WP, the Part Workbench, the 10 ft cube space, the unit-aware Designer grid, and the Designer page's settings and command line.
- Open Questions 2 and the key-binding items need review against this scope.
- The block-name property code (hex values, 3-letter codes, 255-byte limit) and the DXF-comment idea. The XRECORD replaces both. The property code sheet is still useful as the list of properties and their parameters.

## 11. Open items and unverified assumptions

1. **Real wood commands and undo:** the tests used a stand-in edit. The first real wood command must change the JSON and the geometry in one step, and be tested with undo and redo.
2. ~~Custom file extension~~ Decided: `.wood`.
3. **Display name:** how the block list, palette and command prompts substitute the display name still has to be designed.
4. **Very large JSON:** the reader reads at most a set amount of text per record for CADCraft's own records. Our record is kept as-is, but a very large one is untested.
5. **Straightness limit:** how close to straight an edge must be to count as flat.
6. **Property list file:** whether WP still needs `property-codes.json` now that keys are full names.
7. **Property names:** Chris may still change them in the sheet before the first release. Nail, hole, cut and other parameter formats are decided in the JSON schema (see `WoodCraft_Part_JSON_Draft.md`).
8. **Third size limit** value.
9. **Build route:** the wood module is Rust code on top of CADCraft's existing foreign-data storage. Not yet built. The environment I worked in could not compile it; Chris ran the two tests.
10. **Placement list versus mesh** as the assembler's saved result.
11. **WP database design and renderer design:** separate jobs, not blocking WoodCraft.
12. **Imported components, glue fixed charge, settings file location, cutout outline format.**
13. **Fork rules file** for Rust work, and whether to remove `AGENTS.md` and `CLAUDE.md`.

## 12. Working rules (unchanged)

- KISS. No renaming, refactoring or restyling without approval. Ask when intent is unclear.
- Feature branches only, never `main`. Merge only after Chris tests and says so.
- `CHANGELOG.md` entry in every commit, README kept current, tests for new public code.
- Claude cannot compile here; Chris builds and tests each step and reports back.
- Look for an existing library before proposing custom code.
