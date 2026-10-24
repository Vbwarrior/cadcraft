# Phase 6, group 4: joinery and end treatments

The fourth of Phase 6's six property groups. Covers the property sheet's "Joinery (tenon,
mortise, etc.)" group (`Mortise`, `Tenon`, `Joint type`, `Biscuit`) and the "Edges and ends"
group's `End treatment`.

## Files added

- `crates/engine/src/cmd/wood_joints.rs` — eight commands:
  - `woodjoineryadd` / `woodjoinerydelete` / `woodjoinery` ("Add/Delete/Wood Part Joinery") —
    same pattern as Holes and Cuts: Mortise, Tenon and Biscuit share one shape (`Face / Edge`,
    `Length`, `Width`, optional `Depth`/`Tolerance`, an `At` list), parameterized by `kind`.
    Biscuit refuses `depth`/`tolerance` outright (the sheet gives it neither field), rather than
    silently accepting and discarding them. Positions that differ only by location collapse
    into one entry, same rule as Holes/Cuts.
  - `woodjointtypeset` / `woodjointtypeclear` ("Set/Clear Wood Part Joint Type") — `Joint type`
    is a selector (`Joint code`, `Mating Part ID`, optional `Tolerance`), not a positioned
    feature — no `At`, no drawn mark. A part has at most one; setting it again replaces rather
    than adding a second entry.
  - `woodendtreatmentset` / `woodendtreatmentclear` ("Set/Clear Wood Part End Treatment") —
    `{edgeEnd, profile}`; `profile` is checked against the live WoodCraft settings' End
    treatment list (the one Phase 4 seeds and lets the user edit) and refused if it isn't
    there. Setting a treatment for an edge/end that already has one replaces it rather than
    adding a duplicate, so "End A" and "End B" (or any other edge/end name) each hold at most
    one treatment at a time.
  - `woodjointinfo` ("Wood Part Joint Info") — a part's Joint type (or `null`) and its list of
    End treatments, in one call.
  - Mortise/Tenon/Biscuit draw as a small open triangle marker, same reasoning as the other
    groups' marks: never a shape `woodedges` could mistake for the outline. Joint type and End
    treatment draw nothing — they are not positioned features, the sheet gives them no X/Y.
- `crates/engine/tests/wood_joints.rs` — 12 tests: a new part has no joinery; a Mortise
  round-trips; Biscuit refuses `depth`/`tolerance`; two matching Tenons collapse into one entry;
  deleting one position leaves the other; a joinery mark is never read as the outline; Joint
  type set/replace/clear round-trips; an End treatment not in the settings list is refused; an
  End treatment is correctly replaced when set twice for the same edge/end; End A and End B are
  independent; clearing an End treatment that isn't set is refused; joinery and Joint type are
  carried along when the part is copied.

## Files changed

- `crates/wood/src/data.rs`: added `JoineryEntry` (`Face / Edge`, `Length`, `Width`, optional
  `Depth`/`Tolerance`, `At`), `JointTypeEntry` (`Joint code`, `Mating Part ID`, optional
  `Tolerance`, no `At`), `EndTreatmentEntry` (`Edge / End`, `Profile`), and five new `Part`
  fields (`mortise`, `tenon`, `biscuit`: `Vec<JoineryEntry>`; `joint_type`: `Vec<JointTypeEntry>`;
  `end_treatment`: `Vec<EndTreatmentEntry>`), each left out of the JSON when empty.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_joints;` and `v.extend(wood_joints::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the eight new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 22 passed (unaffected)
- `cargo test -p cadcraft-engine --test wood_joints` — 12 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1142 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, added a
  Mortise, set its Joint type, set an End A treatment to "Tenon" (a real settings-list value);
  confirmed `woodendtreatmentset` with a made-up profile name was correctly refused, with the
  refusal message visible at the command line too; confirmed on screen (screenshot) that the
  Mortise's triangle mark draws at its correct position on the left edge.

## Notes

- `End treatment` is modeled as an array (`Vec<EndTreatmentEntry>`), not a fixed two-slot
  structure for "End A"/"End B" specifically — the scope doc says "Each board has End A and End
  B," but the sheet's own row gives `End treatment` the same array shape as every other
  multi-occurrence property, and nothing requires the edge/end name to literally be "End A" or
  "End B" (a part could reasonably have treatments named by its own edges, matching
  `woodedges`' naming, for an edge that isn't strictly an "end"). The command layer enforces
  only "the same edge/end name can't appear twice," not a hard cap of two, which is the simplest
  reading that doesn't contradict either document.
- `profile` validation reads the live settings list on every `woodendtreatmentset` call rather
  than caching it — consistent with how `woodedges` already reads the live straightness
  tolerance setting on every call (Phase 5) — so a profile the user just added to the list in
  settings is immediately usable, and one just removed is immediately rejected.
- `Joint type`'s `Joint code` field is free text here (not validated against the "Joint type"
  settings list from Phase 4), unlike `End treatment`'s `profile`. The sheet's row for `Joint
  type` gives its first parameter as "Joint code," not explicitly tied to the `Lists` tab's
  "Joint type" list the way `End treatment`'s `Profile` is tied to the "End treatment" list in
  `Wood_Workflow_Scope.md`'s own wording ("Lists... each part stores the chosen text"); treating
  the two differently follows what each source actually says rather than assuming they must
  work identically. This is worth Chris's eye if the intent was for both to validate the same
  way — flagged here rather than guessed at silently.
