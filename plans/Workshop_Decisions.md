# Workshop decisions log: WorkShop Pro, Model Maker, Importer and shared items

Decisions Chris made on Oct 23, 2026 for everything that is not WoodCraft. WoodCraft's are in `WoodCraft_Decisions.md`. Claude Code adds a dated line for each choice it makes on its own, with the reason.

## WorkShop Pro (WP)

1. **Assembler's saved result:** a placement list (each part's ID with position and rotation). The 3D view is rebuilt from the parts each time a project opens.
2. **Glue price:** the glue type gets a **Landed cost** field (what the glue costs Chris per use). The materials-list line gets a **Project cost** field, filled from the glue type's Landed cost, so it is Chris's cost with no markup.
3. **Shop-wide settings** live in the existing `ShopSetup`.
4. **Outline format:** add an optional `Bulge` to the existing `{X,Y}` points in the cutout editor's JSON. Old data keeps working and arcs are kept. WoodCraft's `[x, y, bulge]` outlines import into it.
5. **Typing sizes:** fields accept fractions, feet-and-inches and decimals. `1-1/2` means 1.5. A number with no unit means inches (the smallest unit).
6. **Marks:** blemish marks, the Sheet Layout flip, rotate and flaw marks, and WoodCraft part marks and orientation share one saved format. Read Sheet Layout first and match it.
7. **Board Kit:** comment out all of it, including the dead Board Console button and its handlers, with a note at each place saying what it was and why it is commented out, so it can be restored. Delete nothing.
8. **Settings file:** WP has its own, `%APPDATA%\WorkShopPro\settings.json`.
9. **Command line:** none for WP.

## Model Maker (MM)

10. **Parked until the UI pass:** the Grid tool toggles, the grid redo (levels and switch points), the camera orbit and fixed pivot, and Marker and Bookmark. Do not touch them.
11. **Shapes:** no separate cone. The Cylinder shape gets a bottom diameter and a top diameter (equal is straight, a smaller top is a taper, a top of 0 is a point) and each end can be flat or rounded with a rounding size. Claude Code checks in the code which of Hole, Move, Rotate and Flip already exist and records it.
12. **Settings file:** MM has its own, `%APPDATA%\ModelMaker\settings.json`.
13. **Command line:** a separate small console project built on Model Maker's plain-logic library, with plain-text output. Live-view actions are excluded.

## Model Warehouse importer (new, small C# / WPF app in the Workshop repo)

14. Standalone app. Puts a model into the warehouse folder, links its icon (Chris makes the icons), and makes the model's flat top-view outline. On import it asks for the model's units, shows the resulting size to confirm, and does not simplify the mesh. Formats: Collada `.dae` preferred, `.obj` fallback. The warehouse folder is the one WoodCraft reads (see `WoodCraft_Decisions.md` item 12).

## Shared and process

15. **Key bindings:** Ctrl plus a letter, or Ctrl+Shift plus a letter. Stored in each program's settings file. Remapping in a screen is part of the UI pass.
16. **Branches:** if Claude Code finds uncommitted work on `main`, it moves it to a new `feature/` branch, commits `CHANGELOG.md` and `README.md` there, and carries on. It never commits to `main`.
17. **Claude Code runs on Chris's PC,** so the old question about running the command line from WSL is closed.

## Closed because the three-program split retires them

Not asked one by one. Chris can reopen any of them.
- WP Designer page: build state, shared Core reference, sheet points and cylinder, Designer grid and Floor + Wall styles.
- Part Workbench proposals (snap operations to marks, ordered operations list, copy an operation, tool per operation). Operations now come from WoodCraft's properties.
- The old Designer-only items: 10 ft cube space, height warning, work-area size.

## Separate plans, not started

- WP code-first database design, and the WP renderer and importer for WoodCraft's export.
- WP assembler, 3D viewer and light 2D composer.

## Decisions Claude Code makes on its own (add below)

