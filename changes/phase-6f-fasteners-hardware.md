# Phase 6, group 6: fasteners and hardware

The sixth and last of Phase 6's six property groups. Covers the property sheet's "Glue and
fasteners" group (`Glue`, `Screw`, `Nail`, `Dowel`) and "Hardware markers" group (`Hinge`,
`Drawer slide`, `Drawer Pull`). This completes Phase 6.

## Files added

- `crates/engine/src/cmd/wood_fasteners.rs` — nine commands for Glue, Screw, Nail and Dowel
  (the fastener, distinct from `Hole-Dowel` from group 2 — see Notes):
  - `woodglueset` / `woodglueclear` ("Set/Clear Wood Part Glue") — `{faceEdge, type}`; no
    position (the sheet gives Glue no X/Y — "which face or edge gets glue"); `type` checked
    against the WoodCraft settings' Glue type list; replaces any entry for the same `faceEdge`.
    Same shape as End treatment and Edge profile.
  - `woodscrewadd` / `woodscrewdelete` ("Add/Delete Wood Part Screw") — positioned, `type`
    checked against the settings' Screw type list; matching entries at different positions
    collapse into one entry's `At` list, same rule as Holes/Cuts/Joinery.
  - `woodnailadd` / `woodnaildelete` ("Add/Delete Wood Part Nail") — positioned, no `type` at
    all: a Nail is defined only by `Diameter` and `Length`
    (`Wood_Workflow_Scope.md`: "Nails are defined by diameter and length").
  - `wooddowelfastenadd` / `wooddowelfastendelete` ("Add/Delete Wood Part Dowel Fastener") —
    positioned, `{faceEdge, size}`, no type.
  - `woodfasteners` ("Wood Part Fasteners") — a part's Glue, Screw, Nail and Dowel entries
    together, one call.
  - Screw, Nail and Dowel fastener additions each draw a small plus-shaped marker.
- `crates/engine/src/cmd/wood_hardware.rs` — three commands for Hinge, Drawer slide and Drawer
  Pull, told apart by a `kind` parameter (same approach as Holes, Cuts and Joinery, since all
  three share one shape):
  - `woodhardwareadd` / `woodhardwaredelete` ("Add/Delete Wood Part Hardware") — `type` checked
    against the settings' Hardware type list; `size` required for Drawer slide and refused for
    Hinge/Drawer Pull ("location only" per the sheet).
  - `woodhardware` ("Wood Part Hardware") — every hardware entry, by kind (or just the one
    `kind` given).
  - Each hardware addition draws a small marker, distinct in shape from the fastener mark.
- `crates/engine/tests/wood_fasteners.rs` — 10 tests: a new part has none of the four; Glue type
  and Screw type are each checked against their settings list; setting Glue twice for the same
  edge replaces it; two matching Screws collapse into one entry; deleting one Screw position
  leaves the other; Nails have diameter and length but no `type` field at all; a Dowel fastener
  and a `Hole-Dowel` stay completely separate properties; a fastener mark is never read as the
  outline; all four are carried along when the part is copied.
- `crates/engine/tests/wood_hardware.rs` — 8 tests: a new part has no hardware; `type` is
  checked against the settings list; Drawer slide requires `size`, Hinge refuses one; two
  matching Hinges collapse into one entry; deleting one position leaves the other; a hardware
  mark is never read as the outline; an unknown kind is refused; hardware is carried along when
  the part is copied.

## Files changed

- `crates/wood/src/data.rs`: added `GlueEntry` (`Face / Edge`, `Type`, no position),
  `FastenerEntry` (`Face / Edge`, optional `Type`, `Size`, `At` — shared by Screw and Dowel,
  since both have the same shape minus Screw's `Type`), `NailEntry` (`Face / Edge`, `Diameter`,
  `Length`, `At` — its own shape, no `Type`, no `Size`), `HardwareEntry` (`Face`, `Type`,
  optional `Size`, `At` — shared by Hinge, Drawer slide and Drawer Pull), and seven new `Part`
  fields (`glue`, `screw`, `nail`, `dowel`, `hinge`, `drawer_slide`, `drawer_pull`), each left
  out of the JSON when empty.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_fasteners;` and `mod wood_hardware;`, and
  `v.extend(wood_fasteners::specs());` / `v.extend(wood_hardware::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the twelve new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (unaffected)
- `cargo test -p cadcraft-engine --test wood_fasteners --test wood_hardware` — 18 passed (one
  genuine test-authoring mistake caught and fixed during development: two hardware tests used
  `"Drawer Pull"` as a hardware *type* value, but the seeded Hardware type list only has
  `Knob`/`Handle`/`Hinge`/`Latch`/`Drawer slide` — there is no seeded "Drawer Pull" type.
  `woodhardwareadd` correctly refused it; fixed the tests to use `"Knob"` instead)
- `cargo test -p cadcraft-engine` (the whole crate) — 1169 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, set its Top
  edge's glue to "Wood glue," added a Hinge at a position; confirmed `woodfasteners` and
  `woodhardware` both returned correctly; confirmed on screen (screenshot) that the Hinge's mark
  draws at its correct position on the left edge, with the outline unaffected.

## Notes

- **The sheet deliberately lists `Dowel` twice** — once under Holes (`Hole-Dowel`, a drilled
  hole) and once under Glue and fasteners (`Dowel`, a glued dowel pin) — confirmed by the
  earlier research as intentional, distinct properties with different parameter shapes. A test
  (`dowel_fastener_is_distinct_from_hole_dowel`) specifically confirms adding one never touches
  the other.
- This closes out Phase 6. All six property groups from `WoodCraft_Property_Names.xlsx` are now
  implemented: Marks, Holes, Cuts, Joinery/End treatments, Edge profile/Bevel/Miter, and
  Fasteners/Hardware. Every property from the sheet has a command, a JSON entry, a query
  command, and tests, per the phase's own stated goal. Properties with a position draw a small,
  deliberately plain marker (never a shape `woodedges` could mistake for the outline);
  properties that are pure selections (Joint type, End treatment, Edge profile, Bevel, Miter,
  Glue) draw nothing, since they have no position to mark.
