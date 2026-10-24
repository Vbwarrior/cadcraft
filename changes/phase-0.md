# Phase 0: baseline

New crate `cadcraft-wood` (part data model, XRECORD store, GUID generator) and the `woodpart`
engine command, wired into the existing workspace.

## Files added

- `crates/wood/Cargo.toml`
- `crates/wood/src/lib.rs` — crate root, `#![deny(clippy::unwrap_used, ...)]`
- `crates/wood/src/data.rs` — `WoodData`, `Part`, `CommonSize`
- `crates/wood/src/guid.rs` — `new_guid()`
- `crates/wood/src/store.rs` — `get`/`put`/`update` against the `WOODCRAFT_PARTS` XRECORD
- `crates/engine/src/cmd/wood.rs` — the `woodpart` command
- `crates/engine/tests/wood_makepart.rs` — geometry + JSON + undo/redo tests
- `crates/engine/tests/wood_store_roundtrip.rs` — XRECORD survives save/open, revision counting
- `plans/` — plan documents copied in (`WoodCraft_Context.md`, `WoodCraft_Decisions.md`,
  `WoodCraft_Implementation_Plan.md`, `WoodCraft_Part_JSON_Draft.md`, `Wood_Workflow_Scope.md`,
  `Workshop_Decisions.md`, `WoodCraft_Property_Names.xlsx`)
- `changes/` — this file and future per-phase records

## Files changed

- `Cargo.toml` (repo root): added `cadcraft-wood = { path = "crates/wood" }` to
  `[workspace.dependencies]`.
- `crates/engine/Cargo.toml`: added `cadcraft-wood = { workspace = true }`.
- `crates/engine/src/cmd/mod.rs`: added `mod wood;` and `v.extend(wood::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended the Ukrainian translation line for the
  "Make Wood Part..." label.
- `xtask/src/layers.rs`: registered `("wood", Class::Layer(2))` in the layering table.
- `CLAUDE.md`: appended `@plans/WoodCraft_Context.md` as the last line.
- `Cargo.lock`: picked up the new `cadcraft-wood` package.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 3 passed (GUID shape/uniqueness, JSON chunk split, non-ASCII
  escaping)
- `cargo test -p cadcraft-engine --test wood_makepart` — 3 passed (geometry+JSON together, undo
  and redo carry both, bad input refused and changes nothing)
- `cargo test -p cadcraft-engine --test wood_store_roundtrip` — 3 passed (empty by default,
  survives save/open twice, revision only rises on an actual change)
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed (including the two tests that would
  catch a missing translation)
- `cargo clippy -p cadcraft-wood -p cadcraft-engine -p cadcraft-ui-egui --all-targets` — clean,
  zero warnings
- `cargo xtask layers` — `cadcraft-wood` registered L2, depends only on `doc`, no violations

## Notes

- The working tree has pre-existing CRLF line endings on every tracked file (predates this work;
  git's history has LF). Running `cargo fmt` once reformatted the whole repo as a side effect of
  that mismatch; this was reverted to keep the commit scoped to wood files only. Not fixed here —
  a repo-wide line-ending decision is out of scope for this phase.
- Branch renamed from `feature/wood-workflow` to `feature/wood-phase-0-baseline` to match the
  plan's one-branch-per-phase rule.
