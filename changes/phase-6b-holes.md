# Phase 6, group 2: holes

The second of Phase 6's six property groups. Covers the property sheet's Holes group: seven
kinds sharing one shape — `Hole-Through`, `Hole-Blind`, `Hole-Counterbore`, `Hole-Countersink`,
`Hole-Pocket-Screw`, `Hole-Shelf-Pin`, `Hole-Dowel` — each a JSON array of entries, each entry
holding the values shared by one or more holes plus an `At` list of their positions, per
`WoodCraft_Part_JSON_Draft.md`'s "same shape, many places" rule.

## Files added

- `crates/engine/src/cmd/wood_holes.rs` — three commands, all parameterized by a `kind` (one of
  the seven exact sheet names) rather than one command per kind, since all seven share the same
  shape:
  - `woodholeadd` ("Add Wood Part Hole") — `{kind, face, at, dia, depth?, extra1?, extra2?}`.
    If an existing entry of that kind already has the same `face`/`dia`/`depth`/`extra1`/
    `extra2`, the new position is appended to its `At` list instead of creating a new entry —
    this is the JSON draft's position-collapsing rule, implemented as "does a matching entry
    already exist" rather than a later cleanup pass.
  - `woodholedelete` ("Delete Wood Part Hole") — `{kind, at}`: removes the one position at `at`
    from whichever entry of that kind contains it; once an entry's last position is gone, the
    whole entry is removed.
  - `woodholes` ("Wood Part Holes") — `{kind?}` (default: every kind) → every hole entry, keyed
    by its exact sheet name, each with `at` as a plain list of `[x, y]` pairs.
  - Each hole is drawn as a small **X-shaped** marker (an open 5-vertex polyline), deliberately
    never a circle — `woodedges`' outline search accepts either a closed polyline or a circle as
    a part's outline, and a hole marker drawn as a circle, while safe today (the outline is
    always the first entity in a block, and the search takes the first match), would be a
    needless trap for a later change. An X avoids that question entirely.
- `crates/engine/tests/wood_holes.rs` — 11 tests: a new part has no holes; adding one round-trips
  through `woodholes`; two holes with the same face/dia/depth at different positions collapse
  into one entry with two `At` positions; a different diameter makes a separate entry; a
  Counterbore keeps its Extra 1/Extra 2 fields; deleting one of two positions leaves the entry
  with the other; deleting the last position removes the whole entry; deleting a position that
  isn't there is refused; an unknown kind is refused; a hole's drawn mark is never read as the
  part's outline (`woodedges` still finds exactly four edges with a hole mark present); holes are
  carried along when the part is copied.

## Files changed

- `crates/wood/src/data.rs`: added `At` (`X`, `Y` — one position in an `At` list) and `HoleEntry`
  (`Face`, `Dia`, optional `Depth`/`Extra 1`/`Extra 2`, and the `At` list), and seven new `Part`
  fields (`hole_through`, `hole_blind`, `hole_counterbore`, `hole_countersink`,
  `hole_pocket_screw`, `hole_shelf_pin`, `hole_dowel`), each `Vec<HoleEntry>`, left out of the
  JSON when empty. `Part::new` updated to initialize all seven.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_holes;` and `v.extend(wood_holes::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the three new command labels.
- `crates/engine/tests/wood_store_roundtrip.rs`: **fixed a stale Phase 0 test**, not a production
  bug — see Notes.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (unaffected; the new structs round-trip through the
  existing `store` tests)
- `cargo test -p cadcraft-engine --test wood_holes` — 11 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1118 tests, all passed, zero regressions
  (after the `wood_store_roundtrip.rs` fix below)
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, added two
  Hole-Through holes at different positions with the same face and diameter, confirmed
  `woodholes` collapsed them into one entry with both positions in `At`; confirmed on screen
  (screenshot) that both holes draw as small X marks at their correct positions, with the
  outline unaffected; confirmed `woodedges` still correctly reads the four rectangle edges with
  the hole marks present in the block.

## Notes

- **A real test failure turned up while running the full suite** — `wood_store_roundtrip.rs`'s
  `part_data_survives_save_and_open`, a Phase 0 test, started failing. This was not a bug in the
  new code: that test's `sample()` helper predates the Holes group and manually built a
  `Hole-Through` entry as raw JSON inside `Part`'s `other` catch-all field (the only place such
  data could live before this phase). Now that `Hole-Through` is a typed field, the same JSON
  correctly deserializes into `hole_through` instead of `other` — exactly the intended effect of
  promoting a property out of the catch-all, matching `data.rs`'s own documented design ("every
  other property is kept in `other` exactly as read, so a later version of the property list is
  never lost on save" — implying a property *this* version knows about belongs in a named field,
  not `other`). Fixed by updating `sample()` to build the hole data through the typed
  `hole_through`/`HoleEntry`/`At` structs instead of hand-written JSON, which is both the correct
  fix and a preview of what every future phase's tests will need as more properties are promoted
  out of `other`.
- One command set (`woodholeadd`/`woodholedelete`/`woodholes`) serves all seven hole kinds via a
  `kind` parameter, rather than seven near-identical command triples. This keeps the file far
  smaller than one-command-per-kind would, and the sheet's seven kinds already share one
  parameter shape (`Face`, `Dia`, `Depth`, `Extra 1`, `Extra 2`), so there was nothing kind-specific
  for separate commands to express beyond the `kind` string itself.
