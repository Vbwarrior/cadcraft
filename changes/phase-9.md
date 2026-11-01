# Phase 9: History (git)

Local git history for `.wood` project files, per `plans/WoodCraft_Implementation_Plan.md` "Phase
9: History (git)": a dirty flag already exists (`DocState::is_dirty`); a background timer checks
it and, if the drawing is dirty and no command is running, writes and commits the file to a
local git repository rooted at the project file's own folder — never pushed. Adding a new object
commits at once instead of waiting for the timer. The app also commits on normal exit.

## Files added

- `crates/engine/src/history.rs` — the `Historian` type, following the same shape as
  `crate::autosave` (a pure, testable `plan`/`plan_with` with no I/O, and a native-only
  `tick`/`shutdown` that carries it out):
  - `record_command()` — called by `Session::execute` and the interactive-command completion
    path right after a command is found to have actually changed a drawing; records the command
    id against that drawing's uid, filtered to only `.wood` projects (a drawing with no path, or
    a path not ending in `.wood`, is never tracked — ordinary CADCraft drawings are untouched).
  - `plan()`/`plan_with()` — decides what to commit at a given time: a dirty `.wood` project that
    grew a new entity since its last commit is committed at once; one with only edited (not new)
    entities waits `gitTimerMinutes` minutes from when the change was first seen. A drawing that
    is no longer dirty (saved, or never changed) is dropped from tracking with nothing to commit.
  - `next_due()` — seconds until the next commit is due, so the host can schedule a repaint/wake.
  - `git_timer_minutes()`/`set_git_timer_minutes()` — a process-wide setting (default 1.0 minute,
    matching `cadcraft_wood::settings_data::WoodSettings::default().git_timer_minutes`), the same
    static-field pattern as `cadcraft_engine::autosave::savetime()`/`set_savetime()`. A negative
    value is clamped to 0 (timer off; a new object still commits at once).
  - `tick()` (native only) — called every frame: writes and commits what `plan()` says is due,
    using the drawing's own `.wood` write path (the same DXF writer `woodsave` uses), then
    initializes a git repository in the project file's folder if none exists yet, stages the
    file and commits it. The commit message lists the commands run since the last commit, oldest
    first (joined with ", "), or "WoodCraft autosave" if nothing was recorded (e.g. a
    program-driven write that never went through `Session::execute`).
  - `shutdown()` (native only) — commits every dirty `.wood` project right away, ignoring the
    timer, for a normal app exit.
  - A failed write or commit is logged and otherwise ignored — never surfaced to the user as an
    error and never allowed to disturb editing, matching autosave's own failure handling.
  - 8 unit tests with a fake clock (no wall-clock sleeping): the first entity in a `.wood`
    project commits at once; an edit (not a new entity) instead starts a clock and commits
    `gitTimerMinutes` minutes later, with the right commit message; a second new entity before
    the next commit also commits at once; a non-`.wood` drawing is never committed; an unsaved
    drawing (no path) is never committed; saving clears a pending commit; `gitTimerMinutes = 0`
    still commits new entities at once; the timer setting round-trips and clamps a negative
    value to 0; closing a drawing stops tracking it (no unbounded growth in the tracking map).

- `crates/ui-egui/src/history.rs` — drives the engine's `Historian` from the running app, the
  same way `crate::autosave`'s `autosave_tick`/`shutdown_autosave` drive `Autosaver`:
  - `CadApp::history_tick()` — called every frame; ticks `Session::history` and asks egui to wake
    up again when the next commit is due.
  - `CadApp::shutdown_history()` — called on normal exit; commits every dirty `.wood` project.
  - Both take `Session::history` out with `std::mem::take` before calling into it and put it back
    after, since `Historian::tick`/`shutdown` take `&Session` and can't run on a field already
    borrowed out of the same `Session` (the same trick the engine's own unit tests use).
  - A no-op on wasm (git2 is native-only; the WoodCraft browser build is out of scope).

## Files changed

- `crates/engine/Cargo.toml` — added `git2 = { version = "0.19", default-features = false }`
  under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` (MIT; default features off so
  no HTTPS/SSH network transport and no OpenSSL/libssh2 system dependency is pulled in — this
  module only ever runs `init`/`add`/`commit` against a local repository, never a remote).
- `crates/engine/src/lib.rs`:
  - Added `pub mod history;`.
  - Added `pub history: history::Historian` to `Session`, initialized in `Session::new`.
  - `Session::execute` and the interactive-command completion path each gained a few lines that
    call `self.history.record_command(uid, path, command_id)` right after detecting the drawing
    actually changed — the same place each already pushes an undo snapshot.
- `crates/ui-egui/src/lib.rs` — added `pub mod history;` and one call,
  `self.history_tick(ctx);`, right after the existing `self.autosave_tick(ctx);` in `logic()`.
- `apps/cadcraft/src/main.rs` — added `self.cad.shutdown_history();` right after
  `self.cad.shutdown_autosave();` in `on_exit()`.

## Tests run (all pass)

- `cargo test -p cadcraft-engine --lib history -j 6` — 8 passed
- `cargo build -p cadcraft-engine -p cadcraft-ui-egui -j 6` — clean
- `cargo clippy -p cadcraft-engine -p cadcraft-ui-egui --all-targets -j 6` — clean, zero warnings
  beyond the pre-existing, unrelated filesystem hard-linking notice `/mnt/d` prints for every
  crate's build script
- `cargo xtask layers` — clean; `OK: 17 crates, no layering violations` (`cadcraft-engine` is
  still L4, `cadcraft-ui-egui` still L5; `cadcraft-wood` unaffected, still L2 depending only on
  `doc`)
- `cargo fmt -p cadcraft-engine -p cadcraft-ui-egui` — no changes (already correctly formatted)
- `cargo xtask ci` — run before committing; see the commit for the result
- The full 180-file `cargo test -p cadcraft-engine` integration suite was not re-run in full for
  this change (it would rebuild and run every integration test binary in the crate — well over
  an hour on this machine's `/mnt/d`); `cargo xtask ci`, run before the commit, covers it.

## Notes

- **No host actually reads `gitTimerMinutes` from the WoodCraft settings file yet.** The field
  exists (`cadcraft_wood::settings_data::WoodSettings::git_timer_minutes`,
  `crates/wood/src/settings_store.rs`'s `get()`/`update()`), and the engine exposes a setter
  (`history::set_git_timer_minutes`) for a host to call once it has loaded the settings — but
  `crates/ui-egui` does not depend on `cadcraft-wood` at all today, and wiring that up (loading
  WoodCraft settings into the running app, and a settings dialog to edit them) is squarely a
  Phase 12 item per the implementation plan ("Settings dialog") and the "back end first, UI
  last" rule in `plans/WoodCraft_Context.md` §0. Phase 9 therefore ships with the engine-side
  default (1.0 minute, matching the settings default) wired and tested; Phase 12 will call
  `set_git_timer_minutes` once the settings dialog loads the real value. Flagged for Chris: if
  this read should happen sooner than Phase 12, say so and it will be moved up.
- A `CHANGELOG.md [Unreleased]` entry for **Phase 8 (export)** was missing — that phase's commit
  did not touch `CHANGELOG.md`. Backfilled it in the same commit as this phase's own entry, per
  the user's global Changelog Rules ("do not skip entries for small fixes"); nothing else in the
  existing changelog was reordered or rewritten.
- Commit failures (a locked `.git`, a read-only folder, and so on) are only logged, never shown
  to the user and never block editing — consistent with how autosave already treats a failed
  write. There is no user-visible indicator yet that history is behind; if Chris wants one, that
  is a Phase 12 UI decision, not a Phase 9 one.
- `git2`'s default signature (`repo.signature()`, which normally reads the user's own configured
  git identity) is used when available, falling back to a fixed `WoodCraft <woodcraft@localhost>`
  signature only if no git identity is configured at all (a fresh machine with no global
  `user.name`/`user.email` set) — so commits made by WoodCraft's own background timer carry
  Chris's real git identity when he has one set, not a synthetic one, while still never failing
  outright on a machine where he hasn't configured git yet.
