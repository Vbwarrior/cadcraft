# Phase 6, group 3: cuts

The third of Phase 6's six property groups. Covers the property sheet's "Cuts into the part"
group: `Cut-Dado`, `Cut-Groove`, `Cut-Rabbet`, `Cut-Slot`, `Cut-Notch`, `Cut-Pocket`. Unlike
Holes, the six kinds do not all share the same field set.

## Files added

- `crates/engine/src/cmd/wood_cuts.rs` — three commands, parameterized by `kind` (one of the six
  exact sheet names), same approach as Holes:
  - `woodcutadd` ("Add Wood Part Cut") — `{kind, faceEdge, length, width, at?, depth?, angle?}`.
    Which fields are required depends on the kind: `at` for every kind except `Cut-Rabbet`
    ("along an edge": no start position); `depth` for `Cut-Dado`/`Cut-Groove`/`Cut-Rabbet`/
    `Cut-Pocket`; `angle` only for `Cut-Slot` ("through slot": no depth). Giving a field a kind
    doesn't use, or leaving out one it requires, is refused. Positions that differ only by
    location collapse into one entry's `At` list, same rule as Holes.
  - `woodcutdelete` ("Delete Wood Part Cut") — `{kind, at?}`: removes the position at `at` (for
    a kind that has positions), or the one entry outright (`Cut-Rabbet`, which has no `at` to
    name); once an entry's last position is gone, the whole entry is removed.
  - `woodcuts` ("Wood Part Cuts") — `{kind?}` (default: every kind) → every cut entry.
  - Each cut draws as a small square marker (an open 5-vertex polyline), never a closed shape —
    same reasoning as Holes' X mark, so a cut is never at risk of being read as the part's
    outline.

- `crates/engine/tests/wood_cuts.rs` — 12 tests: a new part has no cuts; a Dado round-trips;
  a Dado without `depth` is refused; a Slot requires `angle` (refused without it) and never
  gets a `depth`; a Notch needs neither `depth` nor `angle`; a Rabbet is refused if given an
  `at`, and accepted (with an empty `At` list) without one; two Dados with identical
  faceEdge/length/width/depth at different positions collapse into one entry; deleting one
  Dado position leaves the other; deleting a Rabbet entry needs no position argument; an
  unknown kind is refused; a cut's drawn mark is never read as the outline; cuts are carried
  along when the part is copied.

## Files changed

- `crates/wood/src/data.rs`: added `CutEntry` (`Face / Edge`, `Length`, `Width`, optional
  `Depth`/`Angle`, and an `At` list — every field beyond Length/Width is optional at the data
  level, since which ones a given kind uses varies; the *command* layer enforces which fields
  each kind requires, not the struct itself) and six new `Part` fields (`cut_dado`,
  `cut_groove`, `cut_rabbet`, `cut_slot`, `cut_notch`, `cut_pocket`), each `Vec<CutEntry>`, left
  out of the JSON when empty.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_cuts;` and `v.extend(wood_cuts::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the three new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (unaffected; `CutEntry` round-trips through the
  existing `store` tests)
- `cargo test -p cadcraft-engine --test wood_cuts` — 12 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1130 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, added a
  Dado cut at a position and a Rabbet cut (no position), confirmed `woodcuts` returned both
  correctly (the Rabbet's `at` an empty list); confirmed on screen (screenshot) that the Dado's
  square mark draws at its correct position on the left edge, and the Rabbet's mark (which has
  no position to draw at) sits at the origin, with the outline unaffected.

## Notes

- While writing `is_cut_mark_at` (finding the drawn mark that matches a deleted entry's
  position), an early version computed the mark's first-corner position with the wrong sign
  (`at - Vec2::new(-h, -h)`, which is the *opposite* corner from the one `cut_mark_entity`
  actually draws, rather than `at + Vec2::new(-h, -h)`). Caught and fixed before it was ever
  tested — a mismatched sign like this would have made `woodcutdelete` silently fail to remove
  the right drawn mark (or remove the wrong one) for any cut that wasn't at the origin, which
  the test suite's own position-specific tests (`deleting_a_dado_position_leaves_the_other`)
  would have caught regardless, but it's worth noting as the kind of small geometry mistake
  that's easy to make and easy to miss without a second look.
- `plans/Wood_Workflow_Scope.md`'s "Dado and groove planes" section describes a derived,
  WorkShop-Pro-side concept — a flat, zero-thickness snap plane the assembler builds from a
  dado or groove cut's own position/width/depth, so parts can slide and lock together. Nothing
  about a "snap plane" is stored separately here: a `Cut-Dado`/`Cut-Groove` entry already
  carries everything WP needs to build its own plane from, and building that plane is
  explicitly WP's job, not WoodCraft's.
- As with Holes, `CutEntry`'s fields are all optional beyond the two every kind shares
  (`Length`, `Width`) rather than six separate per-kind structs — the six kinds differ only in
  which optional fields they use, not in having genuinely different shapes, so one struct plus
  command-level validation (`needs_at`/`needs_depth`/`needs_angle`) keeps the data model small
  without losing the sheet's actual constraints.
