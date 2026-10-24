# Phase 1: `.wood` files as commands

New commands to make, open and save a WoodCraft project file (`.wood`). A `.wood` file is DXF
content under a different extension. Standard 2D `open`/`qsave`/`saveas` are untouched.

## Files added

- `crates/engine/src/cmd/wood_file.rs` — the four commands:
  - `woodnew` ("New Wood Project...") — an empty drawing that already holds an empty part JSON.
  - `woodopen` ("Open Wood Project...") — accepts only a path ending in `.wood`; a plain `.dxf` (or
    any other extension) is refused with a message naming the problem.
  - `woodsave` ("Save Wood Project") — like QSAVE: saves to the project's known path, or asks for
    one the first time.
  - `woodsaveas` ("Save Wood Project As...") — like SAVEAS, but refuses a path not ending in
    `.wood`.
  - All four write and read DXF content through the host's existing `IoHooks` (the same mechanism
    `open`/`qsave`/`saveas` use), asking it for DXF bytes under a stand-in `.dxf` name rather than
    going around it, since `cadcraft-engine` is not allowed to depend on `cadcraft-io` directly
    (that crate is a lower layer, and the layering check would refuse it) — only `cadcraft-io`
    itself, and the host app that installs `IoHooks`, know how to turn a `Drawing` into DXF bytes.
- `crates/engine/tests/wood_new_open_save.rs` — 7 tests: an empty project's data is empty; save and
  reopen keeps geometry and the part JSON; saving with no path reuses the known one; saving with no
  path and none known yet is refused with a message pointing to `woodsaveas`; opening a plain
  `.dxf` as a wood project is refused; saving to a non-`.wood` path is refused; both open and save
  refuse bad input without crashing.

## Files changed

- `crates/engine/src/cmd/mod.rs`: added `mod wood_file;` and `v.extend(wood_file::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv`: appended Ukrainian lines for the four new command labels.

## Tests run (all pass)

- `cargo build -p cadcraft-engine` — clean
- `cargo test -p cadcraft-engine --test wood_new_open_save` — 7 passed
- `cargo test -p cadcraft-engine` (the whole crate, every existing test file plus the lib's own
  "run every command with hostile parameters" sweep) — 321 tests in the main suite plus every other
  test file, all passed, zero failures
- `cargo clippy -p cadcraft-wood -p cadcraft-engine -p cadcraft-ui-egui --all-targets` — clean,
  zero warnings
- `cargo xtask layers` — clean; `cadcraft-engine` still shows `cadcraft-io` as a dev-only
  dependency, confirming production code does not depend on it
- Checked by hand in the running app (release build, control channel): `woodnew` → drew a
  rectangle with `RECTANG` typed at the command line → `woodsaveas` to a `.wood` path → reopened
  that file in a fresh session and confirmed the rectangle (now a part block) and its properties
  came back → copied the same file to a `.dxf` name and confirmed `woodopen` refused it with the
  expected message, shown on the command line in a screenshot.

## Notes

- `cadcraft-io`'s own `write`/`read` functions only recognize the `dxf`, `dwg`, `dwt`, `svg`, `png`
  and `pdf` extensions by name; passing a `.wood` path straight through them (as a first attempt at
  this phase did) is refused with `unsupported file type`. The fix was to keep using the host's
  `IoHooks` (installed per the app, already how `open`/`qsave`/`saveas` reach the io layer without
  `cadcraft-engine` depending on it directly) and pass it a `.dxf`-named stand-in so it picks the
  DXF writer, then write the bytes it returns to the real `.wood` path ourselves. Reading needed no
  such trick: an unrecognized extension already falls back to trying a DXF parse.
