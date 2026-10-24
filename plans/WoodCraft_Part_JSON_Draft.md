# WoodCraft part JSON: draft v1 (Oct 22, 2026)

Draft for Chris to change. Nothing here is built. Property names come from `WoodCraft_Property_Names.xlsx`. Items marked **(proposal)** are my suggestion, not a decision.

## 1. Two documents

| | Document A: drawing data | Document B: export |
|---|---|---|
| Where | XRECORD `WOODCRAFT_PARTS` inside the WoodCraft file | A JSON file plus one SCAD file per part, written by Export |
| Who writes it | WoodCraft, on save | WoodCraft, on Export |
| Who reads it | WoodCraft, on open | WorkShop Pro (or Model Maker) |
| Holds | Part properties only. No geometry, because the geometry is already in the drawing. | The same properties plus the outline taken from the drawing, the instance count, and the SCAD file name. |

Placement lists are **not** in either document. The assembler is in WP, so WP produces them. (The Oct 22 scope draft listed a placement list under the WC export. That was wrong and should be removed.)

## 2. Rules for both

- Every key is the full property name from the sheet, spelled exactly as in the sheet (`Hole-Through`, `Common size`). A key never changes after the first release.
- `schemaVersion` is an integer at the top of both documents.
- `units` is `"in"` or `"mm"` for the whole document **(proposal)**. All lengths are plain decimal numbers in those units. The user interface shows fractions if wanted.
- Angles are degrees. Face is `"A"` or `"B"`. Face A is always the appearance face.
- Parts are keyed by GUID. The GUID is also the block name in the drawing.
- One-per-part properties are single values: `Thickness`, `Grain direction`, `Common size`, `Orientation mark`.
- Material and rough stock size are not WoodCraft properties. Every WoodCraft part is a finished part. Both are set in WP when the part is imported, where the rough size is added for the initial cuts.
- Everything else may occur many times, so it is an array of entries.
- **Same shape, many places:** when entries differ only by position, one entry holds the shared values once and an `At` list holds the positions. When anything else differs (a different label, for example), write separate entries.
- A property that is not used is left out. No empty values.

## 3. Document A: stored in the drawing

```json
{
  "schemaVersion": 1,
  "units": "in",
  "propertyListVersion": 1,
  "parts": {
    "3f2a9c10-0000-4000-8000-000000000001": {
      "name": "Leg",
      "revision": 3,
      "Thickness": 1.5,
      "Grain direction": 90,
      "Common size": { "Preset name": "2x4", "Width": 3.5, "Thickness": 1.5 },
      "Hole-Through": [
        { "Face": "A", "Dia": 0.25, "At": [ { "X": 1.5, "Y": 2.25 }, { "X": 1.5, "Y": 4.5 } ] }
      ],
      "Cut-Dado": [
        { "Face": "A", "Start X": 0, "Start Y": 10, "Length": 3.5, "Width": 0.75, "Depth": 0.25 }
      ],
      "Reference mark": [
        { "Face": "A", "X": 0, "Y": 0, "Label": "Datum" }
      ]
    }
  }
}
```

- `name` is the display name. `Orientation mark` is left out here, which means the default: face A, top-left corner, pointing at the top edge.
- `propertyListVersion` is the version of `property-codes.json` the part was built under, so WP can decode it with the right list.
- `revision` goes up by one each time the part's entry changes **(proposal)**, so WP can tell a changed part from an unchanged one on re-import.

## 4. Document B: export

Folder written by Export:

```
MyProject/
  project.json
  models/
    3f2a9c10-0000-4000-8000-000000000001.scad
```

```json
{
  "schemaVersion": 1,
  "units": "in",
  "propertyListVersion": 1,
  "project": { "name": "Hall table" },
  "parts": [
    {
      "id": "3f2a9c10-0000-4000-8000-000000000001",
      "name": "Leg",
      "revision": 3,
      "instances": 4,
      "model": "models/3f2a9c10-0000-4000-8000-000000000001.scad",
      "outline": [ [0, 0, 0], [3.5, 0, 0], [3.5, 38, 0], [0, 38, 0] ],
      "properties": { "…same as the part entry in Document A, without name and revision…" }
    }
  ]
}
```

- `instances` is the number of copies of that GUID in the drawing (Copy Array). WP uses it for the BOM count. A shelf used several times is one part with a count.
- `outline` is a closed path of `[x, y, bulge]` points, the same way CADCraft stores polylines **(proposal)**. A bulge of 0 is a straight segment and anything else is an arc. It comes from the drawing geometry, so it is written only on export.
- There is no WP project ID. WP assigns its own when the project is imported.

## 5. Decisions (all eight settled, Oct 22)

1. **Units and numbers:** one `units` value per document, plain decimals. The user interface shows fractions.
2. **Outline format:** the `[x, y, bulge]` path, the way CADCraft stores polylines. Circles and arcs are exact. Ellipses and splines are exported as short straight segments at a chosen precision.
3. **Revision:** a counter on each part that goes up by one each time its entry changes.
4. **Edge names:** an edge is named by the direction its outside faces: `Top`, `Bottom`, `Left`, `Right`. When several edges face the same way they get letters (`Top A`, `Top B`) in clockwise order, starting from the top-left of the part. An angled edge is named for the side it faces plus "Angle": `Top Angle`, `Left Angle`, and so on. Naming on one side (shown for the top):
   - one flat edge: `Top`
   - two flat edges: `Top A`, `Top B` (clockwise)
   - one angled edge and nothing else: `Top Angle`
   - one flat edge and one angled edge: `Top` and `Top Angle A` (the angled edge takes a letter once a flat edge shares its side)
   - two angled edges and no flat one: `Top Angle A`, `Top Angle B`5. **Lists:** WoodCraft keeps its own lists (joint, edge profile, glue, fastener, hardware) and stores the chosen text in each part. WP has its own lists and can set or change those values on import, and its changes never change the WoodCraft original. Material is set in WP only.
6. **Project ID:** dropped from the export. The export carries a project name only.
7. **Holes:** holes and cuts stay as properties. The SCAD file builds the part from the outline and subtracts each hole and cut, so the rendered part has real holes.
8. **Instance data:** the export carries the count only. The assembler in WP places parts by hand.

**Also decided:** fasteners and hardware are exported, and WP turns them into lines in its materials list. WoodCraft holds the tolerance settings and shows the fastener warnings.

## Corners and edge shapes (decided Oct 22)

- **Rounded and chamfered corners** are part of the outline, drawn with CADCraft's own fillet and chamfer commands. The outline path already holds arcs and straight segments, so there is no corner property.
- **Edge shapes** along the thickness (round-over, chamfer, ogee and so on) are the `Edge profile` property, picked from a drop-down list managed in WoodCraft settings, with a size. The separate `Round-over` and `Chamfer` properties were removed. `Bevel` and `Miter` stay as angle properties.

## Warnings (decided Oct 22)

- WoodCraft keeps a running **warnings list**, like the Error List in Visual Studio. A warning appears when WoodCraft finds the problem and disappears when it is fixed.
- Three kinds go in the list: fastener warnings (for example a fastener that goes through a part), joint fit warnings (too tight or too loose against the tolerance settings) and build-size warnings.
- A **setting** in WoodCraft blocks the export while fastener warnings exist, because they would damage the model. With it on, the export refuses until they are fixed. It can also be left off.
- Any warning can be **acknowledged and overridden** when the user thinks it is wrong. It stays in the list, marked as bypassed, and does not block the export.
- An override lasts **for one export only**. The next export asks again. Nothing about overrides is saved in the file.
- Build-size warnings follow the same rule. This replaces the earlier "acknowledge once per axis, then ignored" rule.
- Warnings are never exported. WP reads values only.
- Fasteners and hardware are exported and become lines in WP's materials list.

## 6. Open items

1. **Straightness limit:** how close to straight an edge must be to count as straight rather than angled (settled in principle, limit not set).
2. ~~Part ID property~~ Removed. The GUID is the part's ID.
3. ~~Fastener warnings~~ Decided: WP reads values only and gets no warnings. See "Warnings" below.
4. ~~Ruff parameters~~ Removed. Rough size is set in WP on import.
5. **Lists' contents:** started from the WP design on the new `Lists` sheet of `WoodCraft_Property_Names.xlsx`. Nails have no type: a nail is defined by diameter and length. Finish is a WP-only property and is not in WoodCraft. Edge profile is a drop-down list managed in WoodCraft settings, started with common router profiles (Round-over, Chamfer, Cove, Bead, Cove and bead, Ogee, Roman ogee, Classical, Bullnose). Each takes a Size.
6. **Property names:** Chris is still free to change them in the sheet before the first release.
