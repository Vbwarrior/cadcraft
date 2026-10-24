# Testing Phases 1-6 (so you can sign off on each merge)

No buttons for any of this yet — Phase 12 builds those. For now you type each command by name
at the command line, the same way you'd type `LINE` or `CIRCLE`. Lower case or upper case both
work.

For each phase below: do the steps, check the result looks right, then tell me either:
- **"merge phase 1"** (or 2, 3, 4, 5, 6) to approve it, or
- what looked wrong, and I'll fix it before asking again.

**Phase 6 note:** this phase is large (six separate groups of part properties — marks, holes,
cuts, joints, edge profiles, fasteners). It's being built and tested one group at a time, each
on this same branch. This guide gets a new section added as each group is finished, so you don't
have to wait for all six before trying the ones that are ready.

You can test the phases in any order, or all in one sitting.

---

## Phase 1: `.wood` project files

Four commands: `woodnew`, `woodopen`, `woodsave`, `woodsaveas`.

1. Start the app.
2. Type `woodnew` and press Enter. This should give you a fresh, empty drawing meant for a wood
   project (behind the scenes it already has an empty parts list attached — nothing to see yet).
3. Draw anything (a rectangle is fine: type `rectang`, pick two corners).
4. Type `woodsaveas` and give it a file name, for example `test1.wood`. It should save without
   complaint.
5. Type `woodnew` again (a new blank drawing).
6. Type `woodopen` and open `test1.wood`. Your rectangle should come back.
7. Now try opening a plain, ordinary `.dxf` file (not a `.wood` file) with `woodopen`. **It
   should refuse**, with a message saying it's not a WoodCraft project. That refusal is the
   point of the test — a `.wood` project and a normal drawing are not supposed to mix.

**What to look for:** Steps 4-6 round-trip your drawing correctly. Step 7 refuses cleanly
instead of opening something it shouldn't.

---

## Phase 2: reading and changing a part's properties

First you need a part to test with. A "part" means a closed shape (a rectangle, a closed
polyline, or a circle) turned into a wood part with the `woodpart` command.

1. Type `woodnew` for a clean start.
2. Draw a rectangle: `rectang`, then two corners.
3. Select the rectangle you just drew (click it).
4. Type `woodpart`. It will ask for a name and a thickness — give it something like `Leg` and
   `0.75`.
5. Type `woodparts` (plural, no selection needed). You should see your one part listed, with the
   name `Leg` and thickness `0.75`.
6. Select the part (click it) and type `woodpartinfo`. You should see its full details printed.
7. With it still selected, type `woodthickness` and give a new number, like `1.5`. Then run
   `woodpartinfo` again — the thickness should have changed.
8. Same idea with `woodname` (give it a new name) and `woodgrain` (give it a direction in
   degrees, or leave it blank to clear it). Check with `woodpartinfo` after each.

**What to look for:** the part always shows its given name, never some internal code. Each
property change actually sticks when you check it afterward.

---

## Phase 3: copying parts and shared arrays

Keep using the `Leg` part from above (or make a fresh one).

1. Select the part and type `woodcopy`, then pick a point to drop the copy. Type `woodparts` —
   you should now see two parts, and the new one's name should end in `-Copy`.
2. Select the **original** part (not the copy) and type `woodcopyarray`, then pick a point. Type
   `woodparts` again. You should **still see only two parts** — this one didn't make a new part,
   it made another copy that shares the same one.
3. Select either of the two linked copies from step 2 and try `woodthickness` with a new number.
   **It should refuse**, and tell you to run Split Array first. That refusal is expected — you're
   not supposed to be able to quietly change just one shared copy.
4. Select one of those same linked copies and type `woodsplitarray`. Now try `woodthickness`
   again on it — this time it should work, because splitting gave it its own independent entry.
5. Type `wooddelete` on any one instance. Type `woodparts` to confirm it's gone. If you delete
   the very last copy of a part, its whole entry disappears from the list; if other copies of it
   still exist, the entry stays.

**What to look for:** step 2 doesn't create a new entry (still two parts, not three). Step 3
correctly refuses. Step 4 correctly allows it again after splitting. Step 5 only removes the
part's entry once nothing is left pointing at it.

---

## Phase 4: settings, Common size presets and lists

Eight commands, all global — they don't need a drawing open or anything selected, because
they work on one settings file shared by every drawing (not something inside a drawing).

1. Type `woodsettingsget`. You'll see the whole settings file printed: things like straightness
   tolerance, units, the three build-size limits (all empty at first), 10 Common size presets
   (lumber sizes 1x2 through 2x8), and six lists (Joint type, Edge profile, Glue type, Screw
   type, Hardware type, End treatment).
2. Type `woodsettingsset` and give it `units: mm`. Run `woodsettingsget` again — units should
   now say `mm`, and nothing else should have changed.
3. Try `woodsettingsset` with `units: cm` (not a real choice). **It should refuse**, saying units
   must be `in` or `mm`.
4. Type `woodsizeadd` and give it a name, width and thickness, for example `Scrap`, `2`, `0.5`.
   Run `woodsettingsget` — you should see 11 presets now, with your new one at the end.
5. Try `woodsizeadd` again with the exact same name, `Scrap`. **It should refuse** — you can't
   have two presets with the same name.
6. Type `woodsizerename` on `Scrap`, giving it a new name like `Offcut`. Then `woodsizedelete` on
   `Offcut`. Check with `woodsettingsget` after each — the name changes, then it's gone, back
   down to 10 presets.
7. Type `woodlistadd` with `list: Glue type` and `entry: Hide glue`. Check `woodsettingsget` —
   the Glue type list should now have 4 entries. Then `woodlistdelete` the same entry to remove
   it again.

**What to look for:** step 2 changes only `units`, nothing else. Step 3 refuses cleanly. Steps
4-6 add, rename and delete a Common size preset correctly, refusing a duplicate name. Step 7
does the same for a list entry.

**One thing to know:** this is the first phase that doesn't touch a drawing file at all — it
writes to one settings file on your computer, shared across every drawing you ever open. On
Windows that's `%APPDATA%\WoodCraft\settings.json`. If you want to reset it back to just the
defaults at any point, delete that file and it'll be recreated fresh the next time any wood
settings command runs.

---

## Phase 5: reading a part's edges

One command: `woodedges`. This reads a part's outline and names each side (Top, Bottom, Left,
Right), tells you whether it's a straight flat edge or an angled one, and whether it's an arc
(a rounded corner from Fillet).

1. Draw a rectangle: `rectang`, two corners.
2. Select it and run `woodpart` with a name and thickness, same as before.
3. Select the part and type `woodedges`. You should see four edges named exactly `Bottom`,
   `Right`, `Top`, `Left`, each marked flat, none of them an arc.
4. Make a shape with one slanted side instead — draw a polyline with `pline` where one side
   leans (not perfectly up-and-down or side-to-side), close it, and make it a wood part the same
   way. Run `woodedges` on it. The leaning side should come back named something like
   `Right Angle` instead of just `Right`, and marked **not** flat.
5. If you'd like to try a rounded corner: draw a rectangle, run `fillet` on one corner with a
   small radius, then make that into a wood part and run `woodedges`. The rounded corner should
   come back marked as an arc.

**What to look for:** step 3's four names are exactly Bottom/Right/Top/Left with nothing extra.
Step 4's leaning side is named with "Angle" and marked not flat; the three straight sides keep
their plain names. Step 5's rounded corner is marked as an arc.

**One more thing you can try:** the straightness tolerance setting (from Phase 4) controls how
close to flat counts as flat. Run `woodsettingsset` with `straightnessTolerance: 2` (2 degrees)
before step 4 — a side that's leaning by less than 2 degrees should then come back marked flat
instead of angled. Run `woodsettingsset` with `straightnessTolerance: 0` afterward to put it
back to the default.

---

## Phase 6, group 1: orientation mark and reference marks

Five commands: `woodorientationmark`, `woodorientationmarkclear`, `woodreferencemarkadd`,
`woodreferencemarkdelete`, `woodmarks`.

1. Draw a rectangle, make it a wood part (same as before).
2. Select the part and type `woodorientationmark`. It will ask for a face (`A` or `B`), an edge
   name (type something like `Top`), and a point to place it. Pick a point right above the top
   edge.
3. Zoom to extents. You should see a small **V** shape sitting just above that edge, its point
   touching it.
4. Type `woodmarks` on the same part. You should see the orientation mark's face and edge
   printed back.
5. Run `woodorientationmark` again with a different face/edge/point. The old V should disappear
   and only the new one shows — there should never be two.
6. Type `woodorientationmarkclear`. The V should vanish, and `woodmarks` should show no
   orientation mark again.
7. Type `woodreferencemarkadd`, giving it a face, a point, and a label like `Datum`. A small
   cross should appear at that point (it's small — you may need to zoom in close to see it).
8. Type `woodmarks` again — you should see your reference mark listed with its label.
9. Type `woodreferencemarkdelete` with that same label. It should disappear from both the
   drawing and `woodmarks`.

**What to look for:** the V always touches the edge you placed it near. Step 5 never leaves two
V's behind. Step 6 actually removes the drawing from view, not just the data. A copy
(`woodcopy`) of a part with reference marks carries them along too, if you want to try that.

---

## Phase 6, group 2: holes

Three commands: `woodholeadd`, `woodholedelete`, `woodholes`.

1. Using the part from above, select it and type `woodholeadd`. It asks for a kind (type
   `Hole-Through`), a face, a point, and a diameter.
2. Zoom to extents. You should see a small **X** mark where you placed it.
3. Add a second through hole with the same diameter and face, at a different point.
4. Type `woodholes`. You should see one `Hole-Through` entry listing **both** points together —
   not two separate entries. That's expected: holes that are otherwise identical share one entry.
5. Now add a third hole of a different diameter. Run `woodholes` again — this one should show up
   as its own separate entry, since it isn't identical to the first two.
6. Type `woodholedelete` and give it one of the two matching points from step 3. `woodholes`
   should now show that entry with only the one remaining point.

**What to look for:** step 4 never doubles up identical holes — one entry, two points. Step 5
creates a new entry instead of merging. Step 6's delete only removes the one point you named.

---

## Phase 6, group 3: cuts

Three commands: `woodcutadd`, `woodcutdelete`, `woodcuts`.

1. Select the part and type `woodcutadd`. Give it kind `Cut-Dado`, a face/edge (type `A`), a
   point, a length, a width, and a depth.
2. Zoom to extents. You should see a small square where you placed it.
3. Type `woodcuts`. You should see your dado listed with the values you gave it.
4. Try adding a `Cut-Slot` without an angle — it should refuse, since a slot needs one.
5. Try adding a `Cut-Rabbet` **with** a point — it should refuse, since a rabbet runs along a
   whole edge and doesn't take a position. Add it again without a point; that should work.
6. Type `woodcutdelete` with kind `Cut-Rabbet` and no point — it should remove that whole entry
   in one go, since a rabbet has nothing to pick a position from.

**What to look for:** each cut kind asks for only the fields that make sense for it, and
refuses when you give it one it doesn't use or leave out one it needs. A rabbet never asks for
a position.

---

## Phase 6, group 4: joinery, joint type and end treatments

Eight commands: `woodjoineryadd`, `woodjoinerydelete`, `woodjoinery`, `woodjointtypeset`,
`woodjointtypeclear`, `woodendtreatmentset`, `woodendtreatmentclear`, `woodjointinfo`.

1. Select the part and type `woodjoineryadd`. Give it kind `Mortise`, a face/edge, a point,
   length, width, and depth. Zoom to extents — you should see a small triangle mark.
2. Type `woodjoinery` — you should see your mortise listed.
3. Type `woodjointtypeset` and give it a joint code (like `Tenon`) and a mating part ID (any
   text — it's meant to be another part's identifier). Type `woodjointinfo` — you should see it
   listed.
4. Type `woodendtreatmentset` with an edge/end (like `End A`) and a profile. Try a made-up
   profile name first — it should refuse, since the profile has to be one from the End
   treatment list (the same list from Phase 4's settings). Then try a real one, like `Tenon`.
5. Type `woodjointinfo` again — you should see your end treatment listed alongside the joint
   type.
6. Set another end treatment for the same `End A` with a different profile — it should replace
   the old one, not add a second.

**What to look for:** step 4 correctly refuses a profile that isn't in the settings list. Step
6 replaces rather than duplicates. The joint type always shows just one entry, however many
times you set it.

---

## After testing

Tell me which of Phase 1, 2, 3, 4, 5, 6 (or a part of 6, since it's being built in groups) are
good to merge, and I'll merge those into `main` in order and let you know when it's done.
