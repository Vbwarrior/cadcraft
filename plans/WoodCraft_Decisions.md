# WoodCraft decisions log

Every decision made so far, so Claude Code never asks twice. Claude Code adds a dated line for each choice it makes on its own, with the reason. Chris can change any line.

## Settled (Oct 22 to 23, 2026)

1. **Straightness:** an edge is flat only when exactly horizontal or vertical. Any angle is an angle. Stored as a setting (degrees) with default **0**. Rounding noise below 1e-9 is ignored.
2. **Display name:** wherever a wood part's block name would show, show the display name. Never show the GUID.
3. **Very large JSON:** cap of 8 MB with an error message, and a test.
4. **`property-codes.json`:** dropped. Keys are full names. Each part stores `propertyListVersion`.
5. **Third build-size limit:** no built-in value. The check is skipped until all three limits are set.
6. **Settings file:** `%APPDATA%\WoodCraft\settings.json`.
7. **Fork rules files:** keep upstream `AGENTS.md` and `CLAUDE.md`. Add a one-line link to the WoodCraft Context doc. Delete nothing.
8. **Common size presets:** seeded with standard lumber sizes (1x2 through 2x8, real inch sizes), editable.
9. **Fastener warnings blocking export:** off by default. When on, export is refused until each warning is fixed or overridden for that export.
10. **Property names:** exactly as in `WoodCraft_Property_Names.xlsx`.
11. **Build order:** back end first and everything wired as commands; UI (buttons, panels, menus, layout, entering wood mode) last, in Phase 12, placed by Chris.

12. **Model Warehouse (library of imported models), lives in WoodCraft.** A folder on disk, default `%APPDATA%\WoodCraft\warehouse\` (a setting). Every subfolder is a category; the UI shows folders as buttons, drilling down, until the models show as thumbnails. A model's folder holds its 3D file (Collada `.dae` preferred, `.obj` fallback), its icon (made by Chris) and a small JSON (including the top-view outline). WoodCraft only reads the warehouse. The back end (scan, list, place, export) is Phase 10a; the buttons and thumbnail view are Phase 12.
13. **Placing a warehouse model:** a flat top-view outline goes in the drawing, with the 3D model linked to it. On export the used models are copied into the export folder and listed in `project.json`.
14. **Importer:** a separate small C# / WPF app (see `Workshop_Decisions.md`) puts models into the warehouse, links the icon, makes the top-view outline. Units are asked and the resulting size confirmed; meshes are not simplified.
15. **Settings:** each program has its own file. WoodCraft's is `%APPDATA%\WoodCraft\settings.json`.
16. **Key bindings** (for the UI pass): Ctrl plus a letter, or Ctrl+Shift plus a letter; stored in the settings file.
17. **Branches and merging:** one branch per phase, stacked in order. Chris validates a phase and says "merge phase N"; only then it merges into `main`, in phase order. Unvalidated phases stay on their branches. No push.

## Decisions Claude Code makes on its own (add below)

18. **(Oct 23, 2026, Chris)** Chris's global CLAUDE.md rules (changelog, README, research-temp,
    git discipline, etc.) apply in full to this repo and this fork, in Rust as much as in C#.
    Nothing here is .NET-only by default; only use a rule's own stated scope (e.g. "xUnit" is
    .NET-specific wording, but its *intent*, tests alongside every new public item, still holds
    here via WoodCraft's own test rules). A `CHANGELOG.md` was added at the repo root per the
    global Changelog Rules, with a backfilled Phase 0 entry (`chore/changelog-setup` branch, not
    yet merged). README updates: skipped for Phase 0 (no user-facing change yet — no UI exists
    before Phase 12), but required starting Phase 1 onward wherever usage (CLI/JSON commands)
    changes, in the same commit as the change, per the global rule. **Clarified same day by
    Chris:** README content is application overview, usage, setup and configuration for the user
    — never a record of what was implemented or when. `changes/phase-N.md` and `CHANGELOG.md`
    are where implementation history goes; README only describes the current, user-facing state.

