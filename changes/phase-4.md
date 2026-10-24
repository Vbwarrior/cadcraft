# Phase 4: settings, presets and lists

WoodCraft's own settings file, Common size presets, and the managed drop-down lists —
`plans/WoodCraft_Implementation_Plan.md` Phase 4. Unlike every earlier phase, none of this
lives in a drawing: it is one file on disk, shared by every drawing, at
`%APPDATA%\WoodCraft\settings.json` (the platform equivalent elsewhere — `~/Library/Application
Support/WoodCraft` on macOS, `$XDG_CONFIG_HOME/woodcraft` or `~/.config/woodcraft` on Linux).

## Files added

- `crates/wood/src/settings_data.rs` — the data model: `WoodSettings` (straightness tolerance,
  units, minimum joint spacing, joint tolerance by type, the three build-size limits, the
  block-export-on-fastener-warnings flag, the git timer minutes, the Common size presets, and
  the lists), `BuildSizeLimits` (three optional numbers; `is_active()` is true only once all
  three are set — the build-size check is skipped until then, per the decided default),
  `CommonSizePreset` (name, width, thickness), `WoodLists` (the six managed lists, one field
  each, named exactly as the property sheet's `Lists` tab), and `ListName` (parses one of the
  six exact sheet names, and gets/gets-mut the matching field — used by the add/rename/delete
  commands so they share one lookup instead of six copies of a match). `WoodSettings::default()`
  seeds the Common size presets with standard lumber sizes 1x2 through 2x8 at their real (not
  nominal) inch dimensions, and seeds the six lists from the `Lists` tab.
- `crates/wood/src/settings_store.rs` — reads and writes the settings file. Follows the same
  shape as CADCraft's own native workspace file (`apps/cadcraft/src/workspace.rs`): a byte cap
  (64 KiB), a `version` field, and a write that goes to a temp file first and is then renamed
  into place, so a crash or a full disk never leaves a half-written file behind. A settings file
  that is corrupt, oversized, or from a newer version is left exactly as it is (never
  overwritten) and reported as an error instead. `get()` returns the settings as they stand (the
  seeded defaults if no file exists yet); `update(|w| ...)` reads, lets you change fields, and
  writes back. A `set_test_path` function (test-only) lets each test point at its own file
  instead of the real per-platform path, without needing `std::env::set_var` (which is `unsafe`
  under this workspace's edition and therefore not usable here) — each test thread gets its own
  override through a thread-local, and `cargo test` gives every test its own thread, so this
  never crosses between tests.
- `crates/engine/src/cmd/wood_settings.rs` — eight commands, all global (no drawing needed, no
  selection, no undo — they touch the settings file, not a drawing):
  - `woodsettingsget` ("WoodCraft Settings") — the whole settings file's contents.
  - `woodsettingsset` ("Set WoodCraft Settings") — changes only the fields given; every field
    left out keeps its current value. `buildSizeLimits` can be given one limit at a time without
    disturbing the other two.
  - `woodsizeadd`, `woodsizerename`, `woodsizedelete` ("Add/Rename/Delete Common Size Preset") —
    refused if the name you're adding or renaming to already exists, or if the name you're
    renaming or deleting doesn't exist. Each returns the settings file's contents afterward.
  - `woodlistadd`, `woodlistrename`, `woodlistdelete` ("Add/Rename/Delete List Entry") — take a
    `list` (one of the six exact sheet names) and an `entry`; the same duplicate/missing refusals
    as the Common size commands.
- `crates/engine/tests/wood_settings.rs` — 10 tests: the seeded defaults come back correctly on
  first use; `woodsettingsset` changes only what's given and leaves the rest (including the
  seeded presets) alone; a bad `units` value is refused with nothing changed; build-size limits
  fill in one at a time; Common size preset add/rename/delete round-trips, with duplicate names
  and missing names refused; the same round-trip for a list entry; an unknown list name is
  refused; the settings commands work with no drawing open at all (`Session::empty()`, no
  `new_drawing`).

## Files changed

- `crates/wood/src/lib.rs`: added `pub mod settings_data; pub mod settings_store;` and re-exports
  (`BuildSizeLimits`, `CommonSizePreset`, `ListName`, `WoodLists`, `WoodSettings`,
  `SettingsError`).
- `crates/engine/src/cmd/mod.rs`: added `mod wood_settings;` and `v.extend(wood_settings::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the eight new command labels.

## Tests run (all pass)

- `cargo test -p cadcraft-wood` — 12 passed (the settings module's own tests: config directory
  resolution for each platform plus the `WOODCRAFT_CONFIG_DIR`/test-path override, a missing
  file reading as defaults without creating anything, a round trip through write and read, a
  corrupt/oversized/newer-version file being reported and left untouched, and `update` seeding
  defaults on first use and changing only what was asked)
- `cargo test -p cadcraft-engine --test wood_settings` — 10 passed
- `cargo test -p cadcraft-engine` (the whole crate) — 1095 tests, all passed, zero regressions
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44 passed
- `cargo clippy -p cadcraft-wood -p cadcraft-engine --all-targets` — clean, zero warnings
- `cargo xtask layers` — clean; `cadcraft-wood` still depends only on `cadcraft-doc` (the
  settings file uses only the standard library, so no new crate dependency was needed)
- Checked by hand in the running app (release build, control channel): `woodsettingsget` on a
  fresh settings file returned the 10 seeded lumber presets at their correct real-inch sizes and
  all six seeded lists; `woodsettingsset` changed only the fields given (units, git timer
  minutes) and left the rest at their defaults; a bad `units` value ("cm") was refused with a
  clear message; the three build-size limits filled in correctly one at a time without
  disturbing each other; `woodsizeadd`/`woodsizerename`/`woodsizedelete` round-tripped a "Scrap"
  preset correctly, refusing a duplicate name; `woodlistadd`/`woodlistdelete` round-tripped a
  "Hide glue" entry in the Glue type list. The settings file written during this check (the
  real, per-platform file, not a test's temp file) was deleted afterward so it doesn't leave
  test data where your own settings will later live.

## Notes

- Settings are intentionally *not* stored anywhere inside `cadcraft-wood`'s existing part-JSON
  machinery (`store.rs`, the drawing's XRECORD) — that mechanism is drawing-scoped (undo, redo,
  Save all carry it), while settings are explicitly global and shared across every drawing, so
  they needed their own, separate file-backed store. This is a new concept in the wood crate, not
  a reuse of the Phase 0 storage.
- `cadcraft_wood::data::CommonSize` (an existing struct — the size a *part* was given, stored in
  that part's own JSON) and the new `cadcraft_wood::settings_data::CommonSizePreset` (the
  settings-level list of *available* presets to choose from) are deliberately different types
  with different names, to avoid confusing "what this part is" with "what you could pick."
  Nothing here changes `CommonSize` or how a part records its chosen size — that link (choosing
  a preset fills in a new part's width and thickness) is a later phase's command, once parts can
  be made with a preset already in mind.
- "Edge profile" is stored as a plain list of names, the same shape as the other five lists. The
  property sheet's `Lists` tab lists edge profiles as plain text entries with no second column;
  "with Size" (the implementation plan's wording) refers to a profile needing a size *when a
  part uses it* (for example, a 3/4" round-over), which belongs to the part's own properties in
  a later phase, not to the settings-level list of profile names itself.
- Real lumber sizes (not nominal): 1x2 is 0.75" x 1.5", not 1" x 2", and so on through 2x8 at
  1.5" x 7.25" — standard actual dressed-lumber dimensions, matching the decision's "real inch
  sizes" wording.
