# Restart notes (Nov 1, 2026 — current status, overwrite this file each checkpoint)

Read this first in the next session, before anything else. It says exactly where we stopped and
why, so the next Claude Code session does not have to rediscover any of it by reading the whole
project. This file is overwritten (not appended) at each checkpoint — it always describes "now,"
not history. Phase history lives in `changes/phase-N.md` once a phase is actually finished.

## Why this file exists right now

No crash or mount problem this time — this is a normal mid-phase checkpoint, written because the
user asked for a standing restart note on every session so a future agent (after a shutdown, a
context reset, or a dropped connection) can resume without re-reading the whole codebase.

**Important environment finding this checkpoint:** a full `cargo xtask ci` (which runs `cargo
test --workspace`, uncapped) was attempted and left running for **over a day** (confirmed still
alive and making progress, not hung, each time it was checked) before the user said to kill it —
it's not viable at this cadence on this machine's `/mnt/d` DrvFS mount. `CARGO_BUILD_JOBS` as an
env var does **not** reliably cap concurrent top-level `rustc` processes the way `--jobs`/`-j` on
the cargo invocation itself would; `cargo xtask ci` builds its `cargo test --workspace` call with
no jobs flag, so there is currently no way to cap it from outside. After killing it, the user
confirmed: **scope checks to the crates actually touched, don't run the full workspace gate for
routine verification.** Also: after killing a cargo run, orphaned rustc child processes can keep
running for a long time afterward on this filesystem (observed: a trivial `cargo test -p
cadcraft-wood` reported "Finished ... in 1946m" — i.e. the *build step alone* took over 32 hours
of wall time — purely because leftover processes from the killed run were still competing for
I/O/CPU). If something looks absurdly slow right after a kill, check `ps aux | grep -E
"rustc|cargo"` for a process count before assuming the new command itself is broken.

## Where we are: Phase 9 (git history), mid-build, not yet committed

Branch: `feature/wood-phase-9-history`, branched from `feature/wood-phase-8-export` (Phase 8 is
already merged to `main` — see `changes/phase-8.md`).

Plan reference: `plans/WoodCraft_Implementation_Plan.md` "Phase 9: History (git)" (4 numbered
requirements: dirty flag, background timer with a commit message listing commands run, commit on
exit, local-only commits never pushed, tests with a fake clock).

### What's done and tested

- `crates/engine/src/history.rs` — the `Historian` type. Pure logic (`plan`/`plan_with`,
  `next_due`, `record_command`) is `#[cfg]`-unconditional and unit-tested with a fake clock (no
  wall-clock sleep in tests); the native-only half (`tick`, `shutdown`, git2-backed `commit`,
  `write_and_commit`) is gated `#[cfg(not(target_arch = "wasm32"))]`.
  - A `.wood` project commits immediately when a new entity is added; an edit to existing
    entities instead starts a clock and commits `gitTimerMinutes` minutes later; the commit
    message lists the command ids run since the last commit (falls back to "WoodCraft autosave"
    if the list is empty, e.g. a program-driven write with no `Session::execute` calls).
  - Non-`.wood` drawings and drawings with no path are never touched by this module at all.
  - `git_timer_minutes()` / `set_git_timer_minutes()` — a process-wide default (1.0, matching
    `cadcraft_wood::settings_data::WoodSettings::default().git_timer_minutes`), same static-field
    pattern as `cadcraft_engine::autosave::savetime()`/`set_savetime()`. Nothing yet calls
    `set_git_timer_minutes` from the real settings file (see "Not yet done" below).
  - `Session::execute` and the interactive-command completion path both call
    `self.history.record_command(uid, path, command_id)` right after they detect the drawing
    actually changed (`crates/engine/src/lib.rs`, two call sites, each commented "WoodCraft Phase
    9"). This is already wired — every command that changes a `.wood` drawing gets logged.
  - 8 unit tests in `history::tests`, all passing (confirmed via
    `cargo test -p cadcraft-engine --lib history`): timer-delayed commit, immediate commit on new
    entity (twice in a row), non-`.wood` drawings ignored, unsaved drawings ignored, saving
    clears the pending commit, timer-off (`minutes=0`) still commits new entities at once, the
    git-timer-minutes getter/setter round-trips and clamps negative input to 0, closed drawings
    are forgotten (no memory leak in the `HashMap<u64, Tracked>`).
- `crates/engine/Cargo.toml` — added `git2 = { version = "0.19", default-features = false }`
  under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` (no network transport feature,
  so no OpenSSL/libssh2 system dependency; native-only since libgit2 is a C dependency that can't
  target wasm32 — WoodCraft's browser build is explicitly out of scope per
  `plans/WoodCraft_Context.md` §6 "the browser (WASM) build is ignored").
- `crates/engine/src/lib.rs` — `pub mod history;`, a `pub history: history::Historian` field on
  `Session`, initialized in `Session::new`/`Session::default`, plus the two `record_command` call
  sites described above.
- **UI wiring (just added, built but not yet test-run or committed):**
  - `crates/ui-egui/src/history.rs` (new file) — `CadApp::history_tick(&mut self, ctx)` (called
    every frame, mirrors `autosave_tick`'s `std::mem::take`-then-restore trick since `tick` takes
    `&Session` and can't run on `self.session.history` in place) and
    `CadApp::shutdown_history(&mut self)` (same take/restore trick, calls `Historian::shutdown`).
    Both are `#[cfg(not(target_arch = "wasm32"))]`-gated inside, no-ops on wasm.
  - `crates/ui-egui/src/lib.rs` — added `pub mod history;` (alphabetically after `hatchdlg`,
    before `mtext_editor`) and one line `self.history_tick(ctx);` right after the existing
    `self.autosave_tick(ctx);` call in `logic()`.
  - `apps/cadcraft/src/main.rs` — added `self.cad.shutdown_history();` right after
    `self.cad.shutdown_autosave();` in `on_exit()`.
  - **Confirmed compiling and passing**: `cargo build -p cadcraft-engine -p cadcraft-ui-egui`
    finished clean (exit 0, 55m on this machine's `/mnt/d`). `cargo test -p cadcraft-engine --lib
    history -j 6` — all 8 tests pass (the 7 from before plus the new
    `git_timer_minutes_defaults_to_one_and_round_trips`).

### Not yet done for Phase 9

1. **Resolved this checkpoint.** The full `cargo xtask ci` run described above was killed per the
   user's decision (see the environment finding at the top of this file). In its place, these
   scoped checks were run and all passed clean:
   - `cargo build -p cadcraft-engine -p cadcraft-ui-egui -j 6` — clean
   - `cargo test -p cadcraft-engine --lib history -j 6` — 8/8 passed
   - `cargo test -p cadcraft-wood -j 6` — 41/41 passed
   - `cargo clippy -p cadcraft-engine -p cadcraft-ui-egui --all-targets -j 6` — clean, zero
     warnings beyond the pre-existing hard-linking notice
   - `cargo xtask layers` — clean, no violations
   - `cargo fmt --all -- --check` (workspace-wide; fmt-check is cheap, no compilation, so this
     one ran at full workspace scope safely) — clean
   - **Still genuinely outstanding, deliberately not run this session:** the full
     `cargo test --workspace` sweep (every crate, every integration test binary — the thing that
     made `cargo xtask ci` take over a day). This branch has NOT had a full-workspace test run
     since Phase 9's changes landed. It should get one before merging to `main`, run in a
     dedicated window where it can be left alone for many hours without being mistaken for
     hung, ideally from a session that won't need other `/mnt/d` cargo activity concurrently
     (concurrent cargo invocations seem to badly starve each other on this filesystem).
2. **Nothing reads `gitTimerMinutes` from the real WoodCraft settings file yet.**
   `cadcraft_wood::settings_store::get()` returns it
   (`crates/wood/src/settings_data.rs:29-31`, field `git_timer_minutes`, default `1.0`), but
   `ui-egui` does not currently depend on `cadcraft-wood` at all (confirmed: no
   `cadcraft-wood`/`cadcraft_wood` reference anywhere in `crates/ui-egui/`). Decision made this
   session: **loading WoodCraft settings into the UI is a Phase 12 (UI) concern**, per
   `plans/WoodCraft_Context.md` §0 "back end first, UI last" and the implementation plan's own
   Phase 12 item "Settings dialog." Phase 9 ships with the engine-side default (1.0 minute) and a
   working setter (`history::set_git_timer_minutes`) that Phase 12's settings dialog will call
   once it exists — this was a judgement call, not something asked for explicitly; flag to Chris
   if he'd rather wire the settings read now instead of in Phase 12.
3. **`changes/phase-9.md` already written** (this checkpoint) — no further action needed unless
   the CI run above surfaces something that changes the facts it describes.
4. **Committed.** Commit `ba613b6` on `feature/wood-phase-9-history`: "Add Phase 9: local git
   history for .wood project files" (11 files changed, 772 insertions, 129 deletions). Includes
   this file (`plans/RESTART_NOTES.md`) — confirmed `plans/` is actually tracked in this fork
   (not gitignored, despite upstream's own singular `plan/` folder being gitignored — a
   different folder, don't confuse the two). **Not pushed** (never push without being asked) and
   **not merged to `main`** (only after Chris tests this phase and says "merge phase 9").
   `.vs/` remains untracked, correctly left alone.
5. **Changelog already updated** (this checkpoint) — `CHANGELOG.md`'s `[Unreleased]` section now
   has both the Phase 9 entry and a backfilled Phase 8 entry that its own commit had skipped
   (logged as decision 23 in `plans/WoodCraft_Decisions.md`). Include this file in the commit.

### CPU/build-speed note for the next session

This machine is WSL2 with the repo on `/mnt/d` (DrvFS) — file I/O there is noticeably slower than
native Linux filesystem, and a `cargo build`/`cargo test` on this crate can take several minutes
(observed: lib build ~3m38s, full `--lib` test rebuild ~11m, the full 180-file
`cargo test -p cadcraft-engine` integration suite rebuild took over an hour and was killed as
unnecessarily broad for a small change). The user asked to cap parallelism so builds don't
saturate all cores (`nproc` = 12) while other work happens alongside — use `-j 6` on `cargo
build`/`cargo test` invocations this session, rather than the default `-j 12`. This is a
per-invocation flag, not a repo-wide `.cargo/config.toml` change — no one has asked for that yet.

For a small logic-only change to `crates/engine/src/history.rs`, prefer the narrow
`cargo test -p cadcraft-engine --lib history` (just the engine lib's own unit tests) over the
full `cargo test -p cadcraft-engine` (rebuilds and runs all 180 separate integration-test
binaries, since every one links the engine library and cargo correctly invalidates all of them
on any library source change — this is expected cargo behavior, not a bug). Save the full
`cargo xtask ci` for right before the actual commit, per the project's quality-gate rule.

## Next steps once resumed

Phase 9 is code-complete and committed (`ba613b6`, scoped checks all green — see above). What's
left:

1. **Owed before merging to `main`:** a full-workspace test run (`cargo test --workspace` or
   `cargo xtask ci`), which this session could not complete in reasonable time on this machine
   (see the environment finding at the top of this file). Run it in a dedicated window, alone,
   with nothing else touching `/mnt/d` cargo state concurrently, and budget many hours. Don't
   assume it's hung if it's still spawning/changing rustc processes.
2. Tell Chris in plain words what now works (git history for `.wood` projects — commits on a
   timer or on a new part, commits on exit, never pushes), per `plans/WoodCraft_Context.md` §8.
3. Once Chris has tested it and says "merge phase 9" (and the full test run above has passed),
   merge `feature/wood-phase-9-history` into `main`. Never merge, push, or delete a branch
   without him saying so first.
4. Then move to Phase 10 (layout tools) or Phase 10a/11, per the plan's dependency order — "9 and
   10 can move any time after Phase 1."
5. Phase 12 (not yet started) will need to: add a `cadcraft-wood` dependency to `crates/ui-egui`,
   load `WoodSettings` at startup, and call `cadcraft_engine::history::set_git_timer_minutes` with
   the loaded `gitTimerMinutes` value (and again whenever the settings dialog changes it) — see
   decision 22 in `plans/WoodCraft_Decisions.md` for why this was deferred rather than done now.
