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

19. **(Oct 24, 2026)** Phase 6 group 4 (joinery): `Joint type`'s `Joint code` field is NOT
    validated against the "Joint type" list seeded in WoodCraft settings (Phase 4), while `End
    treatment`'s `Profile` field IS validated against the "End treatment" list. This follows
    `Wood_Workflow_Scope.md`'s own wording literally — it ties lists to "the chosen text" for
    End treatment but doesn't say the same for Joint type's "Joint code," and the property
    sheet's row for `Joint type` doesn't name the `Lists` tab the way `Edge profile`/`End
    treatment` do. Took the simplest reading (validate only where a source actually says to)
    rather than assuming symmetry. Flagged for Chris: if `Joint code` should also be restricted
    to the "Joint type" settings list, say so and it'll be made consistent with `End treatment`.

20. **(Oct 24, 2026)** Phase 7 (warnings engine): no document in `plans/` gives a numeric
    formula for "joint fit warnings (too tight or too loose against the tolerance settings)."
    Implemented reading: a Mortise/Tenon entry's own `Tolerance` field is the gap checked; it
    must be at least `minimumJointSpacing` (too tight otherwise), and if the part has a `Joint
    type` entry whose `Joint code` matches a key in `jointToleranceByType`, the tolerance must
    not exceed `minimumJointSpacing` plus that type's extra allowance (too loose otherwise),
    per the settings description "extra gap above the minimum." Biscuit is exempt (the sheet
    gives it no `Tolerance`). `JointTypeEntry`'s own `tolerance` field is not used by this
    check. Flagged for Chris: if the real rule differs, say so and the check will be corrected.

21. **(Oct 24, 2026)** Phase 8 (Export): the generated SCAD's geometry is literal and
    axis-aligned, not a router-profile model. Holes become straight cylinders (drilled from the
    stated `Face`, full thickness for `Hole-Through`, else the hole's own `Depth`); cuts become
    axis-aligned boxes at their `At` position and `Length`/`Width`/`Depth` (a `Cut-Rabbet`, which
    has no position, cuts once at the origin — WP repositions it against the named edge on
    import). Nothing carves the actual shape of `Edge profile` (round-over, chamfer curvature,
    and so on), `Bevel` or `Miter` into the model; those stay properties for WP/MM to read, per
    the scope doc's own split between WoodCraft (properties) and WP/MM (3D work). Took the
    simplest reading consistent with `Wood_Workflow_Scope.md` §5's own wording ("subtracts each
    hole and cut, so a rendered part has real holes") rather than inventing profile geometry no
    document asks for. Flagged for Chris: if the SCAD should carve real router-profile shapes
    instead of staying flat, that's a scope change, not a bug, and the geometry will be redone.

22. **(Oct 28, 2026)** Phase 9 (history/git): reading `gitTimerMinutes` from the WoodCraft
    settings file into the running app is left for Phase 12, not done now. The engine side is
    fully wired (a setter, `history::set_git_timer_minutes`, and a tested default matching the
    settings file's own default of 1.0 minute) — only the "load settings into the UI" step is
    deferred, because `crates/ui-egui` has no dependency on `cadcraft-wood` yet and adding one,
    plus a settings dialog to edit the value, is explicitly Phase 12 work per the implementation
    plan and the "back end first, UI last" rule. Flagged for Chris: if the settings read should
    happen sooner, say so and it will be moved up.

23. **(Oct 28, 2026)** Also noticed and fixed while on Phase 9: Phase 8's commit never touched
    `CHANGELOG.md`, leaving its `[Unreleased]` entry missing. Backfilled it in the Phase 9
    commit (per the global Changelog Rules' "do not skip entries for small fixes") rather than
    leaving the gap. Nothing else in the changelog was reordered or reworded.

24. **(Nov 3, 2026, Chris)** The "Chris tests and validates before merge" gate (§8) is about
    phases with something to actually click or drive by hand. A backend-only phase (commands
    only, nothing in the UI yet — true for everything before Phase 12) has nothing Chris can
    interact with beyond the command line or control channel, so for those, passing unit tests
    plus a clean `cargo xtask ci` counts as validated; ask to merge instead of waiting on
    hands-on testing the phase doesn't support. The full gate applies again once a phase adds a
    UI. Recorded in `plans/WoodCraft_Context.md` §8. First applied to merge Phase 10.

25. **(Nov 3, 2026, Chris)** Phase 10a storage: a placed warehouse model (hinge, knob, slide,
    etc.) is hardware the user bought, not a wood part being cut, so it is never added to
    `WoodData.parts`/`WOODCRAFT_PARTS`. It gets its own hidden DXF record, `WOODCRAFT_COMPONENTS`
    (new XRECORD, key `WOODCRAFT_COMPONENTS`, same read/write pattern as `crates/wood/src/
    store.rs`'s `WOODCRAFT_PARTS`), listing every placed instance by its own id — the block name
    of the insert that carries its flat outline in the drawing, exactly like a part's GUID is its
    block name — against `{ model, name }`: the warehouse path it was placed from, and its
    display name at the time it was placed. Export's `components` list
    (`plans/WoodCraft_Part_JSON_Draft.md` "Components (Phase 10a)") reads this record the same
    way `woodexport` reads `WOODCRAFT_PARTS` today, and copies each component's 3D file into the
    export folder alongside the per-part SCAD files.

26. **(Nov 3, 2026)** Phase 10a also needed a warehouse model's own JSON (`model.json` in each
    model's folder) and none was drafted anywhere, so the simplest shape was chosen and written
    down rather than asking: `{ "schemaVersion": 1, "name": "...", "outline": [[x,y,bulge],...] }`
    — the outline in the same `[x, y, bulge]` convention Document B's part `outline` already
    uses. The model's 3D file and icon are found by plain file name in the model's own folder
    (`model.dae` preferred, `model.obj` fallback; `icon.png` if present) rather than referenced
    in the JSON, so nothing there can point at a missing file. Recorded in
    `crates/wood/src/warehouse.rs`'s module doc; the separate C# / WP importer app
    (`Workshop_Decisions.md` item 14) needs to write folders matching this shape. The warehouse
    folder setting (`warehouseFolder` in WoodCraft's settings file) defaults to `None`, meaning
    the per-platform default location next to the settings file itself (a `warehouse` folder
    there), per `WoodCraft_Decisions.md` item 12's `%APPDATA%\WoodCraft\warehouse\` default.

