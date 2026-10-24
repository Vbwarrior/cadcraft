# Phase 6, group 1: orientation mark (V) and reference marks

The first of Phase 6's six property groups — `plans/WoodCraft_Implementation_Plan.md`'s "Add the
properties from `WoodCraft_Property_Names.xlsx` in groups": a command, the JSON entry, a simple
marker in the drawing, a query command, tests, for each group. This group covers the sheet's
"Marks" rows (`Orientation mark (V)`, `Reference mark`) and `plans/Wood_Workflow_Scope.md`'s
"Orientation V" and "Marks" sections.

Given the size of Phase 6 (six distinct property groups, each needing its own data, commands,
markers and tests), each group lands as its own changes record and commit on this same
`feature/wood-phase-6-properties` branch, rather than one giant commit at the end — so a problem
in a later group never risks the ones already working.

## Files added

- `crates/engine/src/cmd/wood_marks.rs` — five commands:
  - `woodorientationmark` ("Set Wood Part Orientation Mark") — sets the part's one orientation
    mark (`Face`, `Edge`, plus `at` for where the drawn V's point sits); replaces any existing
    one, both in the JSON and in the drawing.
  - `woodorientationmarkclear` ("Clear Wood Part Orientation Mark") — back to the unstored
    default (face A, top-left corner, pointing at the top edge); removes the drawn mark.
  - `woodreferencemarkadd` ("Add Wood Part Reference Mark") — adds one entry to the part's
    Reference mark array (`Face`, `X`, `Y`, `Label`) and draws a small cross at its position.
  - `woodreferencemarkdelete` ("Delete Wood Part Reference Mark") — removes the first reference
    mark with the given label, and the drawn cross nearest its stored position.
  - `woodmarks` ("Wood Part Marks") — the part's orientation mark (or `null`) and its reference
    marks.
  - A mark is drawn as a small, **open** polyline inside the part's own block: a 3-vertex V for
    the orientation mark, a 5-vertex cross for each reference mark. Both are deliberately never
    `closed`, so `woodedges`' outline search (which only looks at a closed polyline or a circle)
    skips them automatically — no change to that function was needed.
- `crates/engine/tests/wood_marks.rs` — 6 tests: a new part has no marks; setting the
  orientation mark round-trips through `woodmarks`; setting it twice replaces the drawn V rather
  than adding a second one; clearing it removes the drawn mark, leaving only the outline in the
  block; reference marks add, list and delete by label (deleting a missing label is refused);
  reference marks are carried along when the part is copied (`woodcopy`), since Copy clones
  every entity in the source block, not just the outline.

## Files changed

- `crates/wood/src/data.rs`: added `Face` (`A`/`B`, serialized as those exact letters — the
  sheet's own value), `OrientationMark` (`Face`, `Edge`), `ReferenceMark` (`Face`, `X`, `Y`,
  `Label`), and two new fields on `Part`: `orientation_mark: Option<OrientationMark>` (left out
  of the JSON entirely at its default, per the JSON draft's "stored only when moved") and
  `reference_marks: Vec<ReferenceMark>` (left out when empty). `Part::new` updated to initialize
  both.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_marks;` and `v.extend(wood_marks::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the five new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (unchanged by this group; the new structs round-trip
  through the existing `store` tests without any change there)
- `cargo test -p cadcraft-engine --test wood_marks` — 6 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1107 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, set its
  orientation mark, and confirmed on screen (screenshot, zoomed to extents) that a small V is
  drawn with its point touching the part's top edge exactly where placed; added a reference mark
  and confirmed `woodmarks` returns both correctly.

## Notes

- A mark's drawn geometry carries no metadata of its own (no XDATA, no special layer or
  linetype) — the commands that manage it find "the V" or "the matching cross" structurally (an
  open 3-vertex or 5-vertex polyline), which keeps a mark as plain, minimal geometry as the scope
  doc calls for ("A marker only"). This is deliberately the same idiom CADCraft's own CENTERMARK
  command already uses for a lightweight, selectable-but-minimal annotation glyph, rather than
  inventing a new entity kind.
- The sheet's row for `Orientation mark (V)` lists only `Face` and `Edge` as its stored
  parameters — the drawn V's exact screen position is not itself a stored property (per "stored
  only when moved," only the fact that it moved, and where it points, matters); this
  implementation still takes an explicit `at` point for the command to know where to draw it,
  since there is no interactive placement yet (that's Phase 12's job).
