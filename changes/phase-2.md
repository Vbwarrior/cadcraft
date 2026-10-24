# Phase 2: working copy and query commands

Reading a part's properties, listing every part, substituting a part's display name for its
block name (GUID), and the first three undoable property commands (Thickness, Grain direction,
display name). Also: an 8 MB cap on the stored part JSON.

## Files added

- `crates/engine/src/cmd/wood_query.rs` — six commands:
  - `woodparts` ("List Wood Parts") — every part in the drawing, as `{guid, name, thickness,
    revision}`.
  - `woodpartinfo` ("Wood Part Properties") — the selected (or named) part's full JSON.
  - `wooddisplayname` ("Display Name For Block") — a wood part's display name for its block name,
    or the block name unchanged if it isn't a wood part. The GUID itself is never returned.
  - `woodthickness` ("Set Part Thickness") — undoable; refuses zero or negative.
  - `woodgrain` ("Set Part Grain Direction") — undoable; omitting or nulling the parameter clears
    it.
  - `woodname` ("Rename Wood Part") — undoable; changes only the display name (`Part.name` in the
    JSON); the block name (the GUID) is never touched.
  - A shared `one_part_guid` helper: resolves "the selection, or an explicit handle" (via the
    existing `targets()` helper) down to exactly one Insert whose block is a known wood part,
    with a clear error otherwise (nothing selected, more than one, not a block reference, or not
    a wood part).
- `crates/engine/tests/wood_query.rs` — 11 tests covering all six commands: listing (including an
  empty drawing), reading a part's properties by handle and by current selection, refusing a
  non-wood block and no selection, display-name substitution for a real part and for an unrelated
  block name, each property command's undo/redo, and each property command refusing bad input
  without changing anything.

## Files changed

- `crates/wood/src/store.rs`:
  - Added `pub const MAX_BYTES: usize = 8 * 1024 * 1024;` and a `WoodError::TooLarge(usize)`
    variant.
  - `put()` now checks the JSON's byte length before writing anything, and returns
    `Err(WoodError::TooLarge(..))` instead — nothing is mutated on the drawing when the data is
    too large. `update()` already calls `put()` at the end, so this check reaches it for free.
  - Added `pub fn display_name(d: &Drawing, block: &str) -> String` — the Phase 2 "display_name"
    helper, kept here (not in `data.rs`) since it needs to read the drawing's stored JSON.
  - Added a unit test: a part with an oversized `other` field is refused by `put`, and nothing is
    written to the drawing.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_query;` and `v.extend(wood_query::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the six new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 4 passed (the three from Phase 0, plus the new 8 MB cap test)
- `cargo test -p cadcraft-engine --test wood_query` — 11 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1079 tests across every test file, all
  passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine -p cadcraft-ui-egui --all-targets` — clean,
  zero warnings
- `cargo xtask layers` — clean, no change to the layering table
- Checked by hand in the running app (release build, control channel): made a part, set its
  thickness, grain direction and display name one command at a time, read its full properties
  back (exact property names `Thickness`/`Grain direction` as in the property sheet), listed all
  parts, checked the display-name substitution both for the real part and for an unrelated block
  name, then typed `UNDO` at the command line and confirmed exactly the rename stepped back (the
  command history showed "Rename Wood Part" as the undone step) while the other two changes and
  the revision count reflected it correctly.

## Notes

- The plan's Property Names sheet (`plans/WoodCraft_Property_Names.xlsx`) confirmed the exact
  spelling already used in `data.rs`'s serde renames (`Thickness`, `Grain direction`, `Common
  size`) and confirmed the part's display name (`name`) is a structural JSON field, not a
  property-sheet entry — so `woodname` changes `Part.name` directly, not an `other` property.
- No existing "display name" substitution convention existed anywhere else in the codebase to
  follow (checked; the closest hit, `Drawing::anonymous_block_name`, generates internal names in
  the opposite direction). `display_name()` is a new, first-of-its-kind helper for this codebase.
