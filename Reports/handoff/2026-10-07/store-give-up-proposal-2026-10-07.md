# Proposal: each ant gives up the store on its own (Nest race, 2026-10-07 06:20 UTC), for Deep trace to check

**What it is for.** With the storeroom on (`NEST_STORE=on,pick=20,jaws,sky,meal,smell=10`), hungry ants are drawn down to the store to eat. Today one colony-wide count decides for every ant at once whether the store is worth going to: 8 food cells or more (`STORE_EAT_MIN`, which the code calls "a guess"). The traces show that this one count kills in two ways:

- **It lets a whole group go at once, late and weak.** When the store is eaten below 8 cells, every ant on the store pull is turned to the walk-out in the same rebuild.
  - b30smell s2: 47 of 47 starvers had their last store pull at 104,626-104,726, at a median 0.43 of the grant. 38 of the 89 let go then starved without reaching the top.
  - Deep trace's s1: 35 of 114 let go starved without reaching the top. Ants let go at 0.7 or more nearly all got out.
- **It holds a group at a store too small for it.** When the store stays at 8 cells or more, nobody is let go however long they wait without eating.
  - b90smell s4: 125 of 125 starvers were still on the store pull 10 frames before death, at 0.01 of the grant.
  - 170 ants were let go under 0.3 of the grant; 124 of them starved without reaching the top.

Evidence: `store-arms/sky-meal/laybar-body-results.md` (the last six bullets) and Deep trace's `store-edge-s1-deep-trace-2026-10-07.md` (section "Followed").

## The rule

**An ant that waits at the store without eating gives the store up, by itself, and walks out.**

- Each ant remembers when it last ate or digested food (a new per-ant frame, `store_wait_from`). It is reset by any bite, any crop food, or being back over its grant.
- **While on the eat pull,** if `STORE_PATIENCE` frames pass with no food, the ant sets a latch `store_gave_up`.
- **While the latch is set:**
  - the store eat pull does not fire for that ant;
  - the "at the store, don't walk out" rule (`store_feeds_here`) does not hold it;
  - it is on the walk-out (HUNGRY_OUT), as with the store empty.
- **The latch clears** when the ant is back over its grant, or is out of the nest (any zone but nest).

So release is staggered by each ant's own failure to eat. It is not set by one rebuild, and it does not wait until the ant is too weak to climb.

**Local signal:** the ant's own wait. That fits Scott's rule that ants act on local signals, with global ones only as a labelled stopgap.

**Biology:** none cited yet. This is a stopgap of the "temporary artificial rule" kind. A real ant at an empty or crowded larder does not wait to starve; it turns to foraging (Gordon 2013, foraging set by return rate, doi 10.1038/nature12137). That is the closest match I know, and it is an inference.

**`STORE_PATIENCE`:**
- From the traces: a fed ant at the store bites within a few hundred frames. Ants let go at 0.7 of the grant or more got to the top in a median 272-478 frames.
- An ant burns about one grant in about 10k frames (from the 0.43-at-release, 1,800-frames-to-death figures; inferred).
- **Default 1,000 frames.** That is about 0.1 of a grant spent waiting. Swept at 500 and 2,000.

## Parts (one switch with named parts, off by default)

`PIXEL_PHYSICS_STORE_PATIENCE=off|on|<frames>[,gate=N]`

- `on` = 1,000 frames, with the colony-wide gate left at 8.
- `gate=1` also drops the colony-wide count to 1 cell.
  - The trap the gate was built for (one or two crumbs drawing every hungry ant and holding it) is exactly what per-ant patience undoes locally.
  - This part tests whether the global count can go.

## What it does NOT address

**Seed 4 at heap 30.** That was famine at the top: 149 ants let go at 0.7 or more, 92% reached the top, and 60 starved after reaching it, in and under a shaft packed with ants. A per-ant release cannot help there.

The shaft packing appears in both arms (55-73 of 80 shaft cells hold ants from 18k to 34k). It needs its own trace.

## Test

**Base:** LAY_BAR=body, the store parts above, NEEDS_FIRST, CARRY_HOME and DOOR_COLUMN all on. Seeds 1-4 to 150k, heap 30 and heap 90.

**Arms:**
- patience off (existing runs b30smell and b90smell);
- `on`;
- `on,gate=1`;
- `500`;
- `2000`.

**Judged on these counts, in this order:**
1. **The traced problem.** Among ants released by the store pull, how many starved without reaching the top (releasefunnel.py, by window). Today:
   - s1: 35 (off-switch run: 0);
   - s2: 38;
   - heap-90 s4: 24 + 124.
2. Starved 20-150k, and where (starvewhere_deep.py). Each cluster over 50 is classed as crowd or famine and traced.
3. Fed and staying deep, as N of M, split by door column, with the fed/stay check.
4. Store size and store bites, so the rule does not empty the nest of eaters.
5. Trip food: trip deliveries and forage returns.

**Then default-on needs** Scott's bar: 12 seeds to 300k at heap 30 and 90, clearly better overall or the traced problem fixed, deaths traced, Deep trace's written yes, and CI green. None of that is asked for here.

## Asked of Deep trace

1. Does the rule address what your s1 trace and my s2 and heap-90 traces show?
2. Is 1,000 frames a sound default, and are the sweep points right?
3. Should `gate=1` be in the first test, or wait?
4. Anything in the code I am missing:
   - the eat pull is in `store_pull`;
   - `store_feeds_here` blocks the walk-out;
   - the meal part (`keeps_home_meal`) keeps a picked-up bite.

## Status (06:35 UTC): ON HOLD after Deep trace's review
- Seeds 1 and 2 are dropped as evidence. Those ants were failing to climb out while holding soil long before the store pull let go.
- On heap 90, storewait.py gives the clock weak leverage.
- The heap-90 store looks buried by spoil from the need-drop of half-grant soil holders. See laybar-body-results.md, the last bullet.
- Not built.
