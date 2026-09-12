# The round trip — why nothing came home, round 29

*2026-09-12. Closes the first half of `open-bugs-handoff.md` §T2 and opens a
second, narrower bug under it. Instrument: `examples/nestdoor.rs`.*

## The diagnosis, in one line

**The colony's doorstep was the only impermeable ground on the bed, so it
stood under a permanent puddle, and an ant cannot step into water.**

`paint_nest_patch` converts ~53 surface columns into `nest`, which authors no
`water_capacity` — the default 0, "holds none at all, and never absorbs an
adjacent `Liquid`" — while `soil` and `packedsoil` both hold 1,000. The mist
soaked in everywhere else and stood on the door.
`landing_is_placeable_through_tissue` wants `World::is_empty` and a liquid
cell is not empty, so the film is a **wall**: `adjacent_nest` goes false for
the whole colony at once, and `AtNest`, `nest_visits` and `deliveries` freeze
on one frame while `pickups` goes on climbing — §T2's reported shape exactly.

## What the counters ruled in and out

`nestdoor`, played bed, `RAYON_NUM_THREADS=1`, 120,000 frames, sampled every
10,000. Each hypothesis got a counter that can move only under it.

| hypothesis | counter | verdict |
|---|---|---|
| **buried by spoil** | cover histogram over the patch | **no** — seed 1's cover is `water` 47–49 of 53; `packedsoil` is 1 |
| **dug away** | patch cells no longer `nest` | **no** — `lost 0` on seed 1 for the whole run |
| **drowned** | liquid over the patch vs the same width of ground beside it | **yes** — **89–91 against 17–19** |
| **nobody paths home** | distinct ants 8-adjacent to a nest cell per window | fires *as well*, and separately |
| **unloads en route** (§T2's leading hypothesis) | pickups / drops / deliveries per window | **stale** — the `drop_urge * moisture_gradient` product it names has not existed since 2026-09-02 |
| **only founders delivered** | `life.deliveries` by generation | **no** — later generations deliver once the door is open |

The seed split §T2 could not explain falls straight out of the water pair:
**seed 1 floods (89 vs 17) and never booms; seed 3 does not (2–3 vs 0–1) and
booms.** Same code, same bed.

## The fix

**Drains in the threshold.** `paint_nest_patch` leaves every third column as
the ordinary ground it was, so a one-cell film has **one cell** to travel
before the earth drinks it, instead of 26. Nothing else changes: not the
material, not the ledger, not the moisture field, not what germinates where.
Ablate with **`PIXEL_PHYSICS_NEST_DRAINS=off`** (`=<n>` sweeps the period),
which lays the unbroken patch that shipped until today — one binary, two arms.

**Two more principled fixes were built first and are worse**, both about
`nest.ron` rather than this loop, both in `dead-ends.md`: a `water_capacity`
on a `Solid` **aliases `Cell::aux`** against `structural::tick` (the door
becomes a water *sink*, and every counter that was looking read it as a clean
win), and making it a `Powder` to legalise that turns `player::footing` from
`Hard` to `Soft` so the gnome wades through a nest wall.

Guards in `creature.rs`: `rain_does_not_stand_on_the_nest_patch` (two
soakings — a door that drains the first and not the second is the same bug on
the second day) and `the_nest_patch_is_still_continuous_enough_to_walk_home_to`
(the obvious way to break this fix is to widen the comb until there is nowhere
to walk home to). Both watched going red with the drains removed.

## Before and after, at 120,000 frames

`nestdoor`, played bed, `RAYON_NUM_THREADS=1`, sampled every 10,000. The
paired arm is the same binary at `PIXEL_PHYSICS_NEST_DRAINS=off`.

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| water on the patch / on the same width beside it | **89–91 / 17–19 → 0–19 / 1–3** | 44–69 / 20–34 → 8–15 / 7–12 | 2–3 / 0–1 → 0–19 / 0–6 |
| nest cells with air beside them | **5–6 of 53 → 18–34 of 36** | 8–26 of 45 → 16–30 of 35 | 21–27 of 37 → 7–21 of 25 |
| patch cells destroyed by the end | 0 → 0 | **14 → 0** | 10 → 4 |
| `nest_visits` | **731 → 2,730** | 5,810 → 4,530 | **4,320 → 7,160** |
| deliveries | **232 → 608** | 7,967 → 3,255 | **2,355 → 8,845** |
| born / alive at 120k | **139 / 36 → 581 / 174** | 210 / 129 → 264 / 199 | 712 / 501 → 693 / 340 |
| deepest generation | **7 → 26** | 7 → 10 | 20 → 19 |

Pooled: deliveries 10,554 → 12,708 with 2 of 3 seeds up, `nest_visits`
10,861 → **14,420** with 2 of 3 up, and the colony at 120,000 frames alive
36/129/501 → **174/199/340**.

**Read the door rows, not the colony rows.** The door census measures the
mechanism directly and moves the same way on both seeds; two runs that diverge
on one frame are different worlds by the next, and `deliveries` is this repo's
noisiest column — `instruments.md` prices it at 154–980 across six seeds of an
*unchanged* ant. That is how seed 2 loses deliveries while ending with more
ants, more births and three more generations.

## What is NOT fixed, and do not re-derive it

**Deliveries still fall to zero for long stretches, and after the fix it is
not the door.** Seed 1's last four windows read deliveries 0 with 18–21 nest
cells standing open, 2–5 distinct ants touching the door in each window, and
*closest anyone came* = **1** in every window of the run. The door is
reachable and being reached. What has moved is the laden ants: their mean
distance from home climbs **54 → 143 cells** across the run while the colony
grows to 174–361. They forage further and do not come back.

So door access and home-finding are two causes, and only the first was §T2's.

Three things this lane measured that a later session should not repeat:

- **`rain=0` clears the door on its own** (seed 1: water 51 → 0, nest cells
  with air beside them 2 → 53 by frame 20,000). That is the control that
  pinned the cause, never a fix — the bed's plants need the mister and that
  arm crashes the colony for its own reasons.
- **The first two fixes were about `nest.ron` and both are worse**, for
  reasons that have nothing to do with the door — see `dead-ends.md`. The
  general form is worth more than either: before giving a material a data
  field, check which `Cell::aux` readings that material already has, because
  the compiler cannot see a tagged union whose tag is material data; and a
  material's `kind` is an affordance table for every system at once.
- **A spoil rule over the door is still unbuilt, and the case for it is now
  open rather than closed.** On the water-capacity arm the correlation between
  standable door cells and ants at the door ran the *wrong* way, which read as
  "burial is not the binding constraint". On the shipped arm seed 1's door
  goes `stnd` 34 → 18 and `covd` 14 → 31 as the colony passes 170 ants, with
  `packedsoil` 8–10 of the cover — real, and small beside the 143-cell
  laden-ant distance in the same windows. Measure the walk first.

## Where to look next

The walk, which another lane owns. An ant has no idea where home is; channel
A's home trail is laid only by animals that have touched the nest recently, so
a colony that ranges to 143 cells lays a trail nobody is near. `nestdoor`'s
*closest anyone came* and its laden-distance column are the pair to read.
