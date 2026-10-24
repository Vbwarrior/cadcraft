# Testing Phases 1-3 (so you can sign off on each merge)

No buttons for any of this yet — Phase 12 builds those. For now you type each command by name
at the command line, the same way you'd type `LINE` or `CIRCLE`. Lower case or upper case both
work.

For each phase below: do the steps, check the result looks right, then tell me either:
- **"merge phase 1"** (or 2, or 3) to approve it, or
- what looked wrong, and I'll fix it before asking again.

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

## After testing

Tell me which of Phase 1, 2, 3 are good to merge, and I'll merge those into `main` in order and
let you know when it's done.
