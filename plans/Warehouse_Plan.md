# Model Warehouse plan (Nov 3, 2026, rev 3)

Read with `Warehouse_Model_JSON_Draft.md`, `Workshop_Decisions.md` item 14 and `WoodCraft_Implementation_Plan.md` Phase 10a. Nothing starts without Chris's go-ahead.

Rev 2 adds categories, masters, a properties panel, search and revisions (Chris, Nov 3). The program that writes the warehouse is still called the importer in the other documents. It now does more than import. The name is unchanged.

## Rules

- One folder on disk. The importer is the only writer. WoodCraft, WorkShop Pro and Model Maker only read it.
- No program launches the importer or talks to it. They only read the folder.
- Everything is made in the importer: masters, new models from masters, icons, outlines. Other programs only search and place.
- A missing model gives a clear "model not found" message, never a crash. Exports copy the models they use, so exported projects stay safe.
- Each program has its own settings file with a warehouse folder path (default `%APPDATA%\Workshop\warehouse\`).
- This replaces the SketchUp "Imported Component" idea in the old Designer spec. Do not build both.

## Three kinds of entry

| Kind | What it is | Changes after saving? |
|---|---|---|
| **Master** | A parametric OpenSCAD template with its list of variables. A category may have several (plain hinge, decorative hinge). | Never. A change is a new revision. |
| **Model** | A copy of a master with your values filled in. Saved as its own entry in the same category. | Never. A change is a new revision. |
| **Mesh** | A model imported from a `.dae` or `.obj` file (for example from SketchUp). No variables. | Never. A change is a new revision. |

Consumers list and place Models and Meshes. Masters are only for the importer (open item 6).

## Categories and variables

- A category is a folder with a `category.json`. It lists the **search keys**: the few values every master in that category must provide (for example Length and Width for hinges).
- Each master has its own **variable list**: name, description, type (length, angle, count or choice), default. A decorative hinge can have extra variables.
- The importer shows the variable list in a **properties panel**. The descriptions show in the panel. You tweak the values and save the result as a new Model.
- Same pattern as WoodCraft's Common size presets and Lists (a definition of fields, then values you pick or edit), but it is a separate list kept in the warehouse. See "Overlaps".

## Revisions

- Every entry has a `family` (a GUID that stays the same across revisions) and a `revision` number.
- Entries are never overwritten. A fix or change is saved as a new entry with the same family and revision plus 1.
- A project that places a warehouse model stores its family and revision. It always loads that exact revision.
- On open, a consumer checks the warehouse for a higher revision of the same family. If there is one, it shows a notice. It never switches by itself.

## Search (done in WoodCraft, WorkShop Pro and Model Maker)

- Each search key has two boxes, From and To.
- One box filled: equals (within a small tolerance, open item 3).
- Both boxes filled: a range, inclusive.
- Both empty: that key is ignored.
- An entry must meet every key that has a value.
- If nothing matches, show the closest entries, ranked by how far off they are, and mark them "closest".
- Compare in one unit (millimetres) after converting from each entry's `units`, so 3 in and 76.2 mm match.
- Show the highest revision of each family. Show an older revision only when a project asks for it.

The same rule is written three times (Rust, C#, C#). Keep it exactly as above and use the same test cases in each program.

## Units (Chris, Nov 3)

- The stored value never changes. Each file keeps one `units` value and plain numbers.
- Each calling app shows and reads its own unit. Model Maker uses mm. WoodCraft uses inches, through its existing units setting (Phase 4). WorkShop Pro reads a bare number as inches (Workshop decision 5). The importer has its own display setting.
- A third choice, "both" (for example `1.5" / 38.10 mm`), is available on the same setting.
- Search converts to millimetres inside, so a search typed in inches finds a model stored in mm.
- The icon image is a visual reference only. Its units do not matter.

## Icon views and layouts (Chris, Nov 3)

- Each view has one letter. The user types the letters in order in a text box, for example `TFR`. The first letter is the primary view, then the second, and so on.
- Proposed letters: T top, B bottom, F front, K back, L left, R right, I isometric. Not confirmed.
- The user also picks one of seven layouts. Each layout is a list of numbered slots. The first letter fills slot 1, the next fills slot 2, and so on. Extra slots stay empty. Extra letters are ignored.
- A master holds a default view string and layout. The properties panel lets the user change both before saving a model. The choice is stored in the model.
- OpenSCAD renders each view as its own PNG. The importer puts them in their slots and saves one `icon.png`.
- The layouts are stored as plain numbers in the importer's code, not as image files. They come from Chris's wireframe sheet (see section 5a of `Warehouse_Model_JSON_Draft.md`).
- Chris's sheet has seven layouts, not the six first expected. All seven are recorded.

## Steps

**W0: Freeze the format.** Chris reviews `Warehouse_Model_JSON_Draft.md` and settles its open items. Freeze it before any program reads it.

**W1a: Importer, mesh.** Imports `.dae` (or `.obj`), asks for units, shows the size to confirm, links Chris's icon, makes the top-view outline, writes `model.stl` and `model.json`. Manages folders. No mesh simplifying.

**W1b: Importer, categories and masters.** Category definitions with search keys. Masters with variable lists. Properties panel. Save as a new Model. Masters and Models are immutable; changes make a new revision.

**W1c: Importer, OpenSCAD output.** The importer runs OpenSCAD as a separate process (setting for its path). For each Model it writes the STL, the top-view outline (from `projection()`) and the icon PNG (views and layout chosen by the user, see "Icon views and layouts"). Mesh entries keep Chris's icon.

**W2: WoodCraft reads it.** Phase 10a: folder tree, models in a folder, one model's details, place the outline, export copies used models. Add a search command (the rule above) and the family and revision on the link and in the `components` list. Buttons and thumbnails stay in Phase 12.

**W3: WorkShop Pro reads it.** Search, 3D view of a placed model, position and rotation, BOM. Newer-revision notice.

**W4: Model Maker reads it.** Search, add a model through OpenSCAD. Needs the Add list from the bench plan, so it comes after the bench work.

## Order

W0, then W1a, W1b, W1c. W2, W3 and W4 can follow in any order after W1a. Search needs W1b. Parametric Models need W1c. W2 fits after WoodCraft Phase 8 as already planned. W4 waits for Model Maker's Add list.

## Overlaps with other plans

- **Imported Component** (old Designer spec): retired, replaced by the warehouse.
- **Decision 14** (you make the icons): now only for Mesh entries. The importer makes icons for Models.
- **Phase 10a item 2** (list models in a folder): now also search.
- **Part `revision`** in the part JSON is a different thing. It counts changes to a part's entry. The warehouse revision is a new immutable entry.
- **Search rule** is written in three programs. Spec it once (above) and test it the same way.

## Open

See section 7 of `Warehouse_Model_JSON_Draft.md`.
