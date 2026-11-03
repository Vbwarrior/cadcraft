# Phase 11: Branding and housekeeping

Per `plans/WoodCraft_Implementation_Plan.md` "Phase 11: Branding and housekeeping". The three
items are housekeeping only — nothing behavioral changes, no commands added or changed.

## Files removed

- `docs/brand/` (all 9 files): the ArtCraft trademark logos and their license text
  (`LICENSE-brand.txt`). Not referenced from any code — the About dialog, native menu and
  `build.rs` mention "ArtCraft" in plain text (Discord link, affiliation line, copyright string)
  but never load these image files, so nothing else needed to change for this removal. Per
  Chris: only the brand asset folder is removed; upstream CADCraft's own shared UI text
  (`crates/ui-egui/src/dialogs.rs`, `apps/cadcraft/src/native_menu.rs`,
  `apps/cadcraft/build.rs`) is left as-is, since it is not WoodCraft's to change and the plan
  item names the folder specifically, not every mention of the name.

## Files changed

- `ATTRIBUTION.md` — removed the 8 rows for the deleted `docs/brand/` files and the "one
  exception is the ArtCraft brand" sentence from the policy paragraph (now stale since the
  folder no longer exists). `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE` are untouched/kept, per
  the plan.
- `NOTICE` — removed the paragraph about `docs/brand/`'s trademark terms (now false, since the
  folder is gone) and added a paragraph crediting CADCraft (this repository, upstream
  `storytold/cadcraft`) and OpenSCAD (the separate process WoodCraft exports to, not bundled,
  licensed under the GNU GPL v2 or later, per OpenSCAD's own Debian packaging metadata — the
  authoritative source, OpenSCAD's own `COPYING` file, was not consulted directly since this is
  attribution text, not a legal compliance review).
- `README.md`, WoodCraft section — added a short paragraph: wood-specific code lives in its own
  crate and command files to keep upstream merges easy (the existing principle in
  `plans/WoodCraft_Context.md` §2, now also stated in the README per Phase 11 item 3), credits
  CADCraft and OpenSCAD, and links to `plans/WoodCraft_Context.md` for the full plan. No other
  part of the README (which is otherwise upstream CADCraft's own, not WoodCraft-specific) was
  rewritten.
- `plans/WoodCraft_Context.md` and `CLAUDE.md`/`AGENTS.md` were **not** touched, per the
  context doc's own instruction to keep them and delete nothing.

## Tests

No code changed, so no new tests. `cargo xtask ci` run to confirm the `assets` step (which checks
every asset file against `ATTRIBUTION.md`) still passes after removing `docs/brand/` and its rows,
and that fmt/clippy/test/layers/wasm are unaffected.
