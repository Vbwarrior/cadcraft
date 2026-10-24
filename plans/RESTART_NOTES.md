# Restart notes (Oct 23, 2026 — write this before Chris restarts the PC)

Read this first in the next session, before anything else. It says exactly where we stopped and
why, so the next Claude Code session does not have to rediscover any of it.

## Why we're restarting

The `/mnt/d` WSL mount lost its correct file ownership after a mid-session remount (needed to fix
a stale "No such device" error on the drive). The replacement mount is missing `uid=1000,gid=1000`
(every ad-hoc `mount -t drvfs ... -o uid=1000,gid=1000` attempt was silently ignored — WSL only
applies those through its own automount on boot, not a manual mount call). Result: every new file
cargo writes under `target/` comes back owned by `root`, and cargo's internal link-or-copy step for
build scripts intermittently fails with "Operation not permitted" — a different file each time,
unpredictably. Deleting and rebuilding `target/` does not fix it (new files are still root-owned).
The clean fix is `wsl --shutdown` from PowerShell (outside WSL) and reopening, which re-runs WSL's
own automount and restores correct ownership. Chris is shutting down and restarting the PC, which
achieves the same thing.

**First thing to check in the new session:** run `touch /mnt/d/Rust/CadCraft/.ownertest && ls -la
/mnt/d/Rust/CadCraft/.ownertest && rm /mnt/d/Rust/CadCraft/.ownertest` — it should show owner
`chris chris`, not `root root`. If it still shows root, the mount issue persists and needs more
troubleshooting before any build will be reliable.

## Where we are: Phase 0 (baseline), not yet committed

Branch: `feature/wood-phase-0-baseline` (renamed from `feature/wood-workflow`, which had no
commits of its own — this is still the first commit on the branch, not yet made).

### What's already done and working (tested before the mount broke)

All of this passed cleanly in an earlier, working build (before the ownership problem started):
- `cargo build --workspace` — clean
- `cargo test -p cadcraft-wood` — 3/3 pass
- `cargo test -p cadcraft-engine --test wood_makepart` — 3/3 pass (incl. undo/redo)
- `cargo test -p cadcraft-engine --test wood_store_roundtrip` — 3/3 pass (incl. save/open)
- `cargo test -p cadcraft-ui-egui --lib i18n` — 44/44 pass
- `cargo clippy -p cadcraft-wood -p cadcraft-engine -p cadcraft-ui-egui --all-targets` — zero
  warnings
- `cargo xtask layers` — `cadcraft-wood` registered L2, depends only on `doc`, no violations
- The app itself runs (`cargo run --release -p cadcraft -- --sample --control 9999`), confirmed
  with a screenshot via the control channel.

None of this has regressed as far as we know — the *code* didn't change, only the mount broke
mid-session while re-running the same tests after unrelated edits (adding `plans/`, `changes/`,
the `CLAUDE.md` link). So this should all still pass once the mount is fixed. Re-run the four test
commands plus clippy and layers to confirm before committing.

### Uncommitted changes sitting in the working tree right now

Everything below is real, intentional, already-reviewed work — not half-finished. `git status`
should show:

**Modified (tracked files):**
- `CLAUDE.md` — appended `@plans/WoodCraft_Context.md` as the last line (per
  `changes/phase-0.md` Edit 7). Note: this file is now LF; the other 5 below are still CRLF
  (pre-existing repo-wide drift, see below — harmless, don't "fix" it as part of this).
- `Cargo.lock` — picked up the new `cadcraft-wood` package automatically.
- `Cargo.toml` (root) — added `cadcraft-wood = { path = "crates/wood" }` under
  `[workspace.dependencies]`.
- `crates/engine/Cargo.toml` — added `cadcraft-wood = { workspace = true }`.
- `crates/engine/src/cmd/mod.rs` — added `mod wood;` and `v.extend(wood::specs());`.
- `crates/ui-egui/src/i18n/uk.tsv` — appended the Ukrainian line for "Make Wood Part...".
- `xtask/src/layers.rs` — added `("wood", Class::Layer(2))` to the layering table.

**New (untracked) files that need to be `git add`ed:**
- `crates/wood/` (the whole new crate: `Cargo.toml`, `src/lib.rs`, `src/data.rs`, `src/guid.rs`,
  `src/store.rs`)
- `crates/engine/src/cmd/wood.rs` (the `woodpart` command)
- `crates/engine/tests/wood_makepart.rs`
- `crates/engine/tests/wood_store_roundtrip.rs`
- `plans/` (the whole folder — plan docs copied in from `PLANS & CHANGES/Updates/wood_pack/plans/`,
  plus this file)
- `changes/phase-0.md` (the Phase 0 change record)

**Pre-existing, not ours, do not touch or commit:**
- `crates/engine/tests/WoodCraft_Property_Names.xlsx`, `crates/engine/tests/woodcraft_xrecord_undo.rs`,
  `crates/io/tests/woodcraft_xrecord_roundtrip.rs` — these existed before this session (earlier
  XRECORD survival testing per `WoodCraft_Context.md` §9 "done and tested earlier"). Leave as is.
- `.vs/` — Visual Studio workspace state, unrelated, harmless, not ours.
- `wood_slice1/` — the original (now-superseded) slice-1 drop at repo root. Superseded by
  `wood_slice1b` and then `wood_pack` (both under `PLANS & CHANGES/Updates/`). Safe to delete once
  Chris confirms, but I haven't deleted it unprompted — ask, or just leave it; it's gitignored-worthy
  scratch, not tracked.
- `"PLANS & CHANGES"/` (with the space in the name) — the staging folder Chris drops new
  instruction packs into at the repo root. Still useful as a source location; not meant to be
  committed into the wood crate itself. Leave it.

### Known, already-understood, non-blocking oddity: CRLF vs LF

The working tree has pre-existing CRLF line endings on nearly every tracked file in the whole
repo (not something this session caused — confirmed present in `README.md` and other untouched
files before any wood work started), while git's HEAD has LF. Running a bare `cargo fmt` once
reformatted all 728 files as a side effect (normalizing to LF) — that was reverted; only the 5
files actually edited for Phase 0 were kept, still in their original CRLF form, so their diffs
look huge (every line "changed") but the real content delta is just what's listed above. Do not
run a repo-wide `cargo fmt` or try to "fix" this line-ending mismatch — it's out of scope for the
wood branch and is a separate decision for Chris. `rustfmt` run scoped to just the wood files
(not the whole repo) made no changes, confirming they're already correctly formatted.

### Git safe.directory

After an earlier mount hiccup, git needed
`git config --global --add safe.directory /mnt/d/Rust/CadCraft` (Chris approved this). It's a
global config, so it should still be set after the restart — but if git says "dubious ownership"
again, that's the fix (ask Chris first, per his rule, even though he already approved it once).

## Next steps once the new session starts and the mount is confirmed healthy

1. Re-run the four Phase 0 test commands + clippy + `cargo xtask layers` to reconfirm (should all
   still pass — nothing in the code changed, only the mount broke).
2. `git add` the new files listed above (plans/, changes/, crates/wood/, crates/engine/src/cmd/wood.rs,
   crates/engine/tests/wood_makepart.rs, crates/engine/tests/wood_store_roundtrip.rs) plus the
   modified tracked files, and commit on `feature/wood-phase-0-baseline`. This is the actual
   **Phase 0 commit** — not yet made. Do NOT run a full `cargo xtask ci` blindly before this commit
   without checking its `fmt --check` step doesn't try to reformat the whole repo (see CRLF note
   above) — if it fails only on the pre-existing CRLF drift, that's expected and not a reason to
   block the wood-only commit; use judgement or ask Chris.
3. After Phase 0's commit, tell Chris in plain words what now works (per
   `plans/WoodCraft_Context.md` §8's "give Chris a short plain summary" rule), then move on to
   **Phase 1: `.wood` files as commands** per `plans/WoodCraft_Implementation_Plan.md`. Per the
   implementation plan's own rules, branch for Phase 1 is `feature/wood-phase-1-<short-name>`,
   branched from `feature/wood-phase-0-baseline` (not from `main` — Phase 0 isn't merged to main
   yet; Chris validates and says "merge phase 0" first, separately).
4. Per `plans/WoodCraft_Context.md` §0: work through the plan phase by phase without stopping to
   ask, using `plans/WoodCraft_Decisions.md` for anything already decided, and the simplest option
   (logged there with a reason) for anything not yet decided. Only stop for: an unauthorized merge
   to `main`, a push, deleting a branch or user files, a non-permissive-license crate, or a test
   failure that can't be fixed.
5. Toolchain: rustup/cargo 1.99.1 (stable) and `build-essential` plus `libxkbcommon-x11-0` are
   already installed in this WSL user account (`~/.cargo`, `~/.rustup`) — should survive the
   restart since they're in the Linux filesystem, not on `/mnt/d`. No need to reinstall.
