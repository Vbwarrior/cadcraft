# Phase 3: copy and array rules

Copying a part, sharing one part across several instances, deleting instances (with cleanup once
the last one is gone), and splitting instances back into independent parts — exactly
`plans/Wood_Workflow_Scope.md`'s "Copy and array rules". There is no separate array object in
WoodCraft's data: "the array" is simply the set of Insert entities in the drawing that reference
one part's GUID, found by scanning (CADCraft's own native ARRAY/`ArrayParams` mechanism is a
different, heavier feature and is not used here).

## Files added

- `crates/engine/src/cmd/wood_array.rs` — six commands:
  - `woodcopy` ("Copy Wood Part") — a wholly independent copy: new GUID, a copy of the block's
    geometry under that GUID, a copy of the JSON entry, display name `"<name> -Copy"` (the exact
    wording from the scope doc). Editing the copy never affects the original.
  - `woodcopyarray` ("Copy Wood Part Array") — another instance of the same part at a new
    location: same GUID, same shared JSON entry, no JSON change at all.
  - `wooddelete` ("Delete Wood Part") — erases the picked instance(s); once a GUID's last
    instance is gone, its JSON entry is removed too.
  - `woodsplitarray` ("Split Wood Part Array") and `woodremovefrom` ("Remove From Wood Part
    Array") — the same underlying operation under the two menu names the scope doc gives it: each
    picked instance gets its own new GUID, its own copy of the block's geometry and its own copy
    of the JSON entry; the rest stay shared on the original GUID. If any picked handle is not
    actually an instance of a wood part, the whole command is refused and nothing changes.
  - `woodexplodearray` ("Explode Wood Part Array") — the same split, applied to every instance of
    the selected part at once, so nothing is shared afterward.
  - Helpers: `duplicate_block` (clones an existing block's geometry under a brand-new GUID name
    and places one Insert of it — there was no existing "duplicate this block" primitive in
    CADCraft to reuse; ordinary COPY only clones the Insert entity itself, never the block
    definition), `split_one` (one instance's split, shared by Split Array/Remove From and Explode
    Array), `gc_if_empty` (removes a GUID's JSON entry once none of its instances remain — used
    by Split Array/Explode Array once they empty the original GUID, in addition to `wooddelete`'s
    own last-instance cleanup).
- `crates/engine/tests/wood_array.rs` — 9 tests: Copy makes an independent part and editing it
  doesn't touch the original; Copy refuses without a part selected; Copy Array adds a shared
  instance with no new JSON entry; editing a part with more than one instance is refused, naming
  Split Array; Delete only removes the entry once the last instance is gone; Split Array gives
  picked instances their own GUID and leaves the rest shared; Split Array refuses the whole
  operation, unchanged, if any pick isn't an instance; Explode Array splits every instance; undo
  and redo carry a split (back to one shared part, then back to two independent ones).

## Files changed

- `crates/engine/src/cmd/wood_query.rs`:
  - Added `pub(super) fn instances_of(d, guid) -> Vec<Handle>` — every Insert in model space
    referencing a GUID (there is no stored array object; this is the complete reusable "how many
    instances does this part have" scan).
  - Added `pub(super) fn part_guid_of(doc, cmd, h) -> Result<String>` — the GUID one handle is an
    instance of (factored out of `one_part_guid`'s body so the Phase 3 commands, which work on
    several handles, can reuse it per-handle).
  - `one_part_guid` (used by `woodthickness`/`woodgrain`/`woodname`) now also refuses when the
    part has more than one instance, pointing to Split Array — per the scope doc's "Editing only
    some instances: refused. The user must run Split Array first," read as applying to any
    command that changes a part's shared entry.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_array;` and `v.extend(wood_array::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the six new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-engine --test wood_array` — 9 passed (one bug caught and fixed during
  development: Explode Array and a fully-emptying Split Array were not removing the original
  GUID's now-empty JSON entry; `gc_if_empty` fixed it)
- `cargo test -p cadcraft-engine --test wood_query` — still 11 passed (the new edit-refusal check
  doesn't affect the existing single-instance tests)
- `cargo test -p cadcraft-engine` (the whole crate) — 1088 tests across every test file, all
  passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean, no change to the layering table
- Checked by hand in the running app (release build, control channel): made a part, copied it
  (saw "Leg -Copy" as an independent second entry), copy-arrayed the original (saw the part count
  stay at 2, confirming the shared entry), tried to change the shared part's thickness (correctly
  refused, pointing to Split Array), ran Split Array on the array instance (down to one instance
  of the original GUID, editing it now allowed again), deleted that last instance (its entry was
  removed, leaving exactly the two independent parts), and confirmed on screen (zoomed to
  extents) that exactly two rectangles remained, at their expected positions.

## Notes

- `plans/Wood_Workflow_Scope.md`'s "Copy and array rules" subsection was the only source of truth
  for this phase's behavior; `plans/WoodCraft_Decisions.md` has nothing on arrays/copy. "Split
  Array" and "Remove From" are worded almost identically there and are implemented as the same
  operation under two command ids/labels, matching the two menu entries the scope doc names.
- The "picked items turn reddish until confirmed" part of Split Array's description is a Phase 12
  UI concern (the data operation itself — confirmed subset gets a new GUID — is what this phase
  implements).
- `wooddelete` is its own command, not an extension of the generic `erase`/`delete` command (whose
  `"delete"` id is already an alias of `erase`) — CADCraft's erase has no hook for "something else
  depended on this," so wood's cleanup needed a separate command, per the rule to keep wood code
  in its own files and make the smallest possible change elsewhere.
