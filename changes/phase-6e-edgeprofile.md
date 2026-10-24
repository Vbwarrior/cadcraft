# Phase 6, group 5: edge profile, bevel and miter

The fifth of Phase 6's six property groups. Covers the property sheet's "Edges and ends" group's
`Edge profile`, `Bevel` and `Miter` (`End treatment`, the fourth property in that group, was
already done in group 4). None of the three are positioned features — no `At` list, no drawn
mark — they are per-edge/end choices, same shape as group 4's End treatment.

## Files added

- `crates/engine/src/cmd/wood_edgeprofile.rs` — seven commands:
  - `woodedgeprofileset` / `woodedgeprofileclear` ("Set/Clear Wood Part Edge Profile") —
    `{edgeEnd, profile, size}`; `profile` is checked against the live WoodCraft settings' Edge
    profile list (seeded in Phase 4: Round-over, Chamfer, Cove, Bead, Cove and bead, Ogee,
    Roman ogee, Classical, Bullnose) and refused if it isn't there, same approach as group 4's
    End treatment. Setting a profile for an edge/end that already has one replaces it.
  - `woodbevelset` / `woodbevelclear` ("Set/Clear Wood Part Bevel") and `woodmiterset` /
    `woodmiterclear` ("Set/Clear Wood Part Miter") — `{edgeEnd, angle}`; Bevel and Miter share
    one internal shape (`AngleEntry`) since the sheet gives them identical parameters (just
    `Edge / End` and `Angle`), only the key they're stored under differs.
  - `woodedgeinfo` ("Wood Part Edge Properties") — a part's Edge profile, Bevel and Miter
    entries together, one call.
- `crates/engine/tests/wood_edgeprofile.rs` — 9 tests: a new part has none of the three; a
  profile not in the settings list is refused; setting a profile twice for the same edge
  replaces it; clearing one that isn't set is refused; Bevel and Miter round-trip independently
  of each other and of Edge profile; setting a Bevel twice replaces it; clearing a Bevel that
  isn't set is refused; clearing one Miter leaves another on a different edge untouched; all
  three are carried along when the part is copied.

## Files changed

- `crates/wood/src/data.rs`: added `EdgeProfileEntry` (`Edge / End`, `Profile`, `Size`) and
  `AngleEntry` (`Edge / End`, `Angle` — shared by Bevel and Miter), and three new `Part` fields
  (`edge_profile: Vec<EdgeProfileEntry>`, `bevel: Vec<AngleEntry>`, `miter: Vec<AngleEntry>`),
  each left out of the JSON when empty.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_edgeprofile;` and
  `v.extend(wood_edgeprofile::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the seven new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (unaffected)
- `cargo test -p cadcraft-engine --test wood_edgeprofile` — 9 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1151 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — one real warning caught
  and fixed (an unneeded explicit lifetime on `angle_field_mut`; rechecked clean after)
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, set its
  Top edge's profile to "Round-over" (a real settings-list value), confirmed a made-up profile
  name was correctly refused; set a Bevel on the Left edge; confirmed `woodedgeinfo` returned
  both correctly, with Miters correctly empty. No drawn mark to check visually for this group
  (no position data — same as Joint type and End treatment in group 4).

## Notes

- Bevel and Miter are the same shape at the data level (`AngleEntry`), split only by which
  `Part` field and JSON key they go under — matching exactly what the sheet gives them (both
  are just `Edge / End` plus `Angle`), rather than two near-duplicate structs.
- This group draws no marker in the drawing, unlike every group before it (Marks, Holes, Cuts,
  Joinery). The implementation plan's own instruction ("a simple marker in the drawing") applies
  naturally to positioned features; Edge profile, Bevel, Miter, Joint type and End treatment are
  all per-edge/end *selections* with no X/Y to mark, so there's nothing to draw — consistent
  with how group 4's Joint type and End treatment already worked the same way.
