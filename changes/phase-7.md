# Phase 7: warnings engine

The warnings list (`plans/Wood_Workflow_Scope.md` "Warnings"): a running list, like Visual
Studio's Error List, computed fresh from a drawing's current wood parts and the live settings —
"a warning appears when found and disappears when fixed." Three kinds: fastener warnings, joint
fit warnings, build-size warnings. Any warning can be acknowledged for one session, never saved
to the file.

## Files added

- `crates/wood/src/warnings.rs` — pure, stateless logic (no drawing access, mirroring how
  `edges.rs` holds the edge-naming algorithm separately from its engine command):
  - `Warning` (a stable `id`, `kind`, `part_guid`, `message`) and `WarningKind`
    (`Fastener`/`JointFit`/`BuildSize`).
  - `fastener_warnings(part_guid, part)` — any hole whose `Depth` (Blind, Counterbore,
    Countersink, Pocket-Screw, Shelf-Pin, Dowel) or nail whose `Length` exceeds the part's
    `Thickness`: "a fastener that goes through a part." `Hole-Through` is exempt by
    construction — it has no `Depth` field at all, since going all the way through is its whole
    point.
  - `joint_fit_warnings(part_guid, part, settings)` — a Mortise or Tenon entry's own
    `Tolerance` (the snug-fit allowance) checked against a range built from
    `minimumJointSpacing` (the floor) and, when the part has a `Joint type` entry, that type's
    extra tolerance from `jointToleranceByType` (the ceiling above the floor). Biscuit is exempt
    (the sheet gives it no `Tolerance`). See Notes on how this reading was chosen.
  - `build_size_warnings(part_guid, dimensions, settings)` — the part's three physical
    dimensions, sorted smallest to largest, each checked against the matching build-size limit;
    skipped entirely until all three limits are set, reusing `BuildSizeLimits::is_active()` from
    Phase 4.
  - `part_warnings(part_guid, part, settings, dimensions)` — all three checks for one part in
    one call; `dimensions` is `Option` since only the engine (not this crate) can read a part's
    outline from the drawing.
  - 12 tests: a deep/shallow hole and a long nail each tested for the fastener check; a through
    hole never warns regardless of depth (it has no depth field); warning IDs are stable across
    repeated computation (needed so acknowledgement sticks to the same real-world problem); a
    too-tight tolerance warns, an acceptable one doesn't, a too-loose one (against a joint
    type's extra tolerance) warns; Biscuit is exempt; build size is skipped until all three
    limits are set, warns on whichever sorted dimension is over, and is silent when everything
    fits.
- `crates/engine/src/cmd/wood_warnings.rs` — three commands:
  - `woodwarnings` ("WoodCraft Warnings") — every current warning across every wood part in the
    drawing, computed fresh: reads live settings, and for each part looks up its block's one
    outline entity and computes its bounds (`cadcraft_doc::entity_bounds`) for the build-size
    check — reusing the exact "find the part's outline entity" pattern `wood_edges.rs` already
    established. Each returned warning carries `acknowledged: bool` from the session's own
    acknowledged set.
  - `woodwarningacknowledge` / `woodwarningunacknowledge` ("Acknowledge/Un-acknowledge Wood
    Warning") — add or remove one warning ID from `Session::acknowledged_wood_warnings`. Global
    commands (`.enabled(always)`, no drawing needed), `.noundo()` (session state, not drawing
    state, is never part of undo/redo).
- `crates/engine/tests/wood_warnings.rs` — 9 tests: a new part has no warnings; a deep hole
  warns, a shallow one doesn't; acknowledging a warning marks it without removing it from the
  list, and un-acknowledging reverts that; deleting the hole that caused a warning makes it
  disappear (the "disappears when fixed" rule, proven end to end through real commands, not
  just the pure-logic layer); a through hole never warns no matter how thin the part; a tight
  joint tolerance warns; a build-size warning appears only once all three limits are set;
  warnings from two different parts are reported separately, each with its own part GUID.

## Files changed

- `crates/engine/src/lib.rs`: added `Session::acknowledged_wood_warnings:
  std::collections::HashSet<String>` — session-only, never part of `Drawing`/`WoodData`, never
  saved to a file, never carried by undo/redo (per the scope doc: "An override lasts for one
  export only. The next export asks again. Nothing about it is saved in the file"). Initialized
  empty in `Session::new()`.
- `crates/wood/src/lib.rs`: added `pub mod warnings;`.
- `crates/engine/src/cmd/mod.rs`: added `mod wood_warnings;` and
  `v.extend(wood_warnings::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the three new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 34 passed (12 new warnings tests, 22 already there)
- `cargo test -p cadcraft-engine --test wood_warnings` — 9 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1178 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — three real warnings caught
  and fixed (the `field_reassign_with_default` pattern in test helper code, same lint hit back
  in Phase 4); clean after
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc`
- Checked by hand in the running app (release build, control channel): made a part, added a
  hole deeper than the part's thickness, confirmed `woodwarnings` reported it correctly;
  acknowledged it and confirmed it stayed listed but marked `acknowledged: true`; deleted the
  hole and confirmed the warning disappeared entirely — the full "appears, can be acknowledged,
  disappears when fixed" lifecycle, verified end to end.

## Notes

- **The joint-fit rule is a judgment call, logged in `plans/WoodCraft_Decisions.md` (#20)** —
  no document in `plans/` states the exact numeric formula for "too tight or too loose." The
  scope doc only says tolerance-per-type is "extra gap above the minimum." The reading
  implemented: a Mortise/Tenon entry's own `Tolerance` field is the gap being checked; it must
  be at least `minimumJointSpacing` (too tight otherwise), and if the part names a `Joint type`
  with a matching entry in `jointToleranceByType`, it must not exceed `minimumJointSpacing` plus
  that type's extra allowance (too loose otherwise). Biscuit is exempt since the sheet gives it
  no `Tolerance` field to check. `JointTypeEntry`'s own separate `tolerance` field is not used
  by this check (it's a property of the joint-type selector itself, not a per-instance gap to
  validate) — only `jointToleranceByType` from settings, keyed by `joint_code`, is read for the
  "extra allowance" half of the range.
- Build-size and fastener warnings compute directly, needing no judgment call — the scope doc
  states both precisely enough to implement without guessing.
- `Session::acknowledged_wood_warnings` is deliberately not yet reset anywhere — the scope doc's
  "lasts for one export only" rule needs an actual export command to reset it against, and
  Export is Phase 8. The field and its acknowledge/un-acknowledge commands are ready; wiring the
  reset into Export is left for that phase, noted here so it isn't forgotten.
