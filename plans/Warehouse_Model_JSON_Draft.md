# Model Warehouse: JSON draft v3 (Nov 3, 2026)

Draft for Chris to change. Nothing here is built. Items marked **(proposal)** are a suggestion, not a decision. Read with `Warehouse_Plan.md`.

## 1. Who writes and who reads

- The **importer** is the only writer. It creates, renames and deletes everything in the warehouse.
- **WoodCraft, WorkShop Pro and Model Maker** only read. They never create, rename or delete anything in the warehouse.
- If a model is missing, a consumer shows a clear "model not found" message and carries on. It never crashes.

## 2. Folder

- Default location: `%APPDATA%\Workshop\warehouse\` **(proposal)**. Neutral, because four programs use it.
- Each program's own settings file holds a "warehouse folder" path that defaults to this. Decision 15 (each program has its own settings file) is unchanged.
- Every subfolder is a category or a category group. A folder that holds a `category.json` is a category. A folder that holds a `model.json` is an entry.
- Revisions sit side by side as separate folders, for example `Butt hinge r1` and `Butt hinge r2` **(proposal)**.

```
warehouse/
  Hardware/
    Hinges/
      category.json
      Butt hinge r1/              (a master)
        model.json
        master.scad
        icon.png
      Hinge 3 x 1.5 r1/           (a model made from it)
        model.json
        model.scad
        model.stl
        icon.png
    Knobs/
      Round knob r1/              (a mesh import)
        model.json
        model.dae
        model.stl
        icon.png                  (made by Chris)
```

## 3. Rules for all warehouse JSON

Same as the part JSON: full-name keys, `schemaVersion`, one `units` value per file, plain decimal lengths, angles in degrees. A key never changes after the first release. A property that is not used is left out. No empty values.

Units: the stored value never changes. Each calling app shows and reads its own unit (Model Maker mm, WoodCraft inches, with a "both" option). The icon image is a visual reference only.

## 4. category.json

```json
{
  "schemaVersion": 1,
  "name": "Hinges",
  "description": "Butt hinges and similar leaf hinges.",
  "searchKeys": [
    { "Name": "Length", "Description": "Overall length along the knuckle", "Type": "length" },
    { "Name": "Width",  "Description": "Overall width, leaf edge to leaf edge", "Type": "length" }
  ]
}
```

Every master in the category must have a variable with each search key's name.

## 5. model.json

Every entry has `kind` (`master`, `model` or `mesh`), `family` (a GUID, the same across revisions) and `revision` (a whole number, starting at 1). Entries are never changed after saving. A change is a new entry with the same `family` and the next `revision`.

### Master

```json
{
  "schemaVersion": 1,
  "units": "in",
  "kind": "master",
  "name": "Butt hinge",
  "family": "6c1d2a40-0000-4000-8000-000000000010",
  "revision": 1,
  "variables": [
    { "Name": "Length", "Description": "Overall length along the knuckle", "Type": "length", "Default": 3.0 },
    { "Name": "Width", "Description": "Overall width, leaf edge to leaf edge", "Type": "length", "Default": 3.0 },
    { "Name": "Leaf thickness", "Description": "Thickness of one leaf (T in the drawing)", "Type": "length", "Default": 0.09 },
    { "Name": "Hole count", "Description": "Screw holes per leaf", "Type": "count", "Default": 3 }
  ],
  "iconViews": "TFR",
  "iconLayout": 2,
  "files": { "SCAD": "master.scad", "Icon": "icon.png" }
}
```

### Model (made from a master)

```json
{
  "schemaVersion": 1,
  "units": "in",
  "kind": "model",
  "name": "Hinge 3 x 1.5",
  "family": "6c1d2a40-0000-4000-8000-000000000011",
  "revision": 1,
  "master": { "family": "6c1d2a40-0000-4000-8000-000000000010", "revision": 1 },
  "values": { "Length": 3.0, "Width": 1.5, "Leaf thickness": 0.09, "Hole count": 3 },
  "size": { "Length": 3.0, "Width": 1.5, "Height": 0.3 },
  "outline": [ [0, 0, 0], [1.5, 0, 0], [1.5, 3.0, 0], [0, 3.0, 0] ],
  "iconViews": "TFR",
  "iconLayout": 2,
  "files": { "SCAD": "model.scad", "STL": "model.stl", "Icon": "icon.png" }
}
```

- `values` holds one value for each variable of the master. The variable definitions are not repeated. They come from the master.
- `size` is the bounding box of the model.
- `outline` is the flat top view, a closed `[x, y, bulge]` path, the same as the part export **(proposal)**. WoodCraft draws it. OpenSCAD's `projection()` gives straight segments, so round shapes come out as short segments, not arcs.
- `model.scad` is self-contained (the template with the values filled in), so the model does not depend on the master folder **(proposal)**.

### Mesh (imported)

Same as a Model, but with no `master` and no `values`, and `files` holds the 3D file as imported:

```json
"files": { "3D": "model.dae", "STL": "model.stl", "Icon": "icon.png" }
```

### 5a. Icon views and layouts

- `iconViews` is a string of view letters. The first is the primary view. Proposed letters: T top, B bottom, F front, K back, L left, R right, I isometric. Not confirmed.
- `iconLayout` is a layout number, 1 to 7. Mesh entries have neither, because Chris makes those icons.
- Slots are numbered in the order shown. Letter 1 fills slot 1, letter 2 fills slot 2, and so on.
- Each slot is `[x, y, width, height]` as a fraction of the icon box, with y measured down from the top. The icon box is about 160 wide by 139 tall (from the wireframe sheet), so it is slightly wider than tall.

| Layout | Look (from the wireframe sheet) | Slots |
|---|---|---|
| 1 | Two equal columns | `[0,0,.5,1]` `[.5,0,.5,1]` |
| 2 | Tall left column, right column split in two | `[0,0,.5,1]` `[.5,0,.5,.5]` `[.5,.5,.5,.5]` |
| 3 | Tall left column, right column split in three | `[0,0,.5,1]` `[.5,0,.5,.333]` `[.5,.333,.5,.333]` `[.5,.667,.5,.333]` |
| 4 | One full view with a small inset in the top-right corner | `[0,0,1,1]` `[.5,0,.5,.5]` |
| 5 | Wide top half, bottom half split in two | `[0,0,1,.5]` `[0,.5,.5,.5]` `[.5,.5,.5,.5]` |
| 6 | Top half and bottom half | `[0,0,1,.5]` `[0,.5,1,.5]` |
| 7 | One full view | `[0,0,1,1]` |

- Layouts are numbered left to right, top row then bottom row, as on the sheet. Renumber if you want a different order.
- In layout 4 the inset covers the top-right corner of the primary view.

## 6. What a consumer reads

To search: `category.json` (the key names) and each entry's `values` (or `size` for a Mesh, if the keys match) **(proposal)**. To place: `name`, `family`, `revision`, `units`, `size`, `outline` and the files. A project stores `family` and `revision`, and loads that exact revision.

## 7. Open items

1. WorkShop Pro reads the `.dae` or the `.stl`? Decide later. Models made from masters have no `.dae`, only the STL and the SCAD, so WorkShop Pro will need to read the STL either way.
2. Model Maker uses the STL or the `model.scad` (it already has an imported-code shape)? Does the STL need to be in millimetres?
3. Tolerance for "equals" in search (for example 1/64 in). Setting or fixed?
4. Outline origin: the model's corner or its centre? Which view is "top" for a hinge lying flat?
5. Do consumers read every `model.json`, or should the importer also write one index file per warehouse for fast search?
6. Should consumers be able to place a master (with its defaults), or only Models and Meshes?
7. Icon views: settled by Chris (letters plus one of seven layouts, see 5a). Still to confirm: the letter set, the layout numbering and the icon size in pixels. An open state such as a 270 degree hinge is a master variable, not a view.
8. Search keys for Mesh entries: do they carry `values` for the search keys, typed in at import?
9. Folder names for revisions and how the importer names a new model.
10. Where the importer lives in the repo and what OpenSCAD path setting it uses.
