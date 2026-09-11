# Round 28, fourth build — the midden

*Follow-on to `Reports/lanes/evolution-lab-garden-rot.md` (PR #325, merged).
That lane closed the mechanism -- `pip_checks` now fires -- and reported a
threshold rather than chasing it: soil water passed 2 of 454 checks
(0.9%) on `played_bed`, readings clustering at exactly 0.00. This lane
answers *why*.*

## The drop-cell measurement

`nest.ron` authors no `water_capacity` field at all (default 0); `soil.ron`
authors `water_capacity: 1000`. `Behavior::Germinate`'s own soil-water
check (`plant.rs`) reads exactly that field off the cell one row below the
seed. `deliver_seed_passenger`'s drop site is wherever `NEIGHBOURS_8` finds
the first empty cell from the dropping ant, and `ant.ron`'s `AtNest`
instinct biases every drop toward the ground `creature::paint_nest_patch`
lays -- a bare `Solid`, the same nest material, every time. So a pip set
down at the nest door is dry **by construction**, not by circumstance: no
amount of the colony's own traffic or the lamp's placement changes it,
because the material it is standing on cannot hold water at all. That
settles the cost fork in the brief: build (a), the midden, not (b),
watering the nest material -- changing what the nest itself is would also
change the colony's digging and the bed's moisture, which is a much larger
lever than this round needs.

## The build: the midden, revised once from the brief's literal spec

`plant::find_midden_site` (`src/sim/plant.rs`), called from
`deliver_seed_passenger` before the cell is written: if the ground the pip
would land on does not clear `site_holds_enough_water`, search outward --
nearest column first, alternating sides, out to `MIDDEN_SEARCH_COLUMNS`
(6) -- at the surface `creature::colony_surface` already defines, and
redirect to the first candidate that clears it. A bound on work, never a
gate on whether the pip is set down: nothing found in reach leaves the pip
exactly where it would have landed before this build. Two counters,
`World::pips_set_on_soil`/`pips_set_on_nest`, read the outcome (not only
whether the redirect fired) on every delivery.

**The first version shipped nothing and was caught before it did.** The
brief's own literal wording was `water_capacity > 0`, and that is what the
first pass checked. Measured on `played_bed` before landing it: it
relocated pips onto real, water-*capable* soil at a 100%/67% rate across
two seeds and moved `plants_from_pip` **not at all** (2->2, 0->0) -- the
redirected ground could hold water and was not currently holding enough of
it, the exact "dry by circumstance" reading the brief itself named as the
*other* possibility. A material check cannot tell "can hold water" from
"is holding water", and only the second is what `Behavior::Germinate`
actually asks. Revised `site_holds_enough_water` to the real two-part test
that check runs: `water_capacity > 0` **and**
`update::plant_available_fraction(ground) >= threshold`, with `threshold`
read live off the delivering organism's own species
(`seed_water_threshold`, herb ships 0.15) rather than a constant.

**Three tests**, `src/sim/plant.rs`. Two on the first pass:
`a_pip_set_on_dry_nest_ground_is_relocated_to_the_nearest_wet_soil_within_reach`
(nest ground left, real wet soil four columns right -- inside the 6-column
reach but past the first search ring, so a search bug cannot pass by
accident; put the fault back by hand at `MIDDEN_SEARCH_COLUMNS = 0`,
watched it go red on `MaterialId(54)`, restored) and
`a_pip_with_no_wet_soil_within_reach_still_lands_where_it_was_dropped` (the
size-cap half). A third guards the revision itself,
`the_midden_skips_soil_that_can_hold_water_but_is_not_holding_enough_of_it`
-- real `soil` material two columns out with `aux` left dry, genuinely wet
soil one ring further; put the fault back (reverted the predicate to
material-only), watched it land on the dry-but-capable soil at x+2 instead
of skipping to x+5, restored.

## Measurement, 3 seeds × 120,000 frames, `played_bed`, before/after

Before = `PIXEL_PHYSICS_MIDDEN=0` (the runtime kill switch), same binary,
same seed -- reproduces PR #325's own baseline table exactly (seed 2: 8
delivered / 213 checks / 2 water-OK / 2 `plants_from_pip`; seed 3: 4 / 241
/ 0 / 0), which is itself the positive control that the switch does what
it claims.

| seed | arm | delivered | on soil (final) | water OK / A2 checks | plants_from_pip | rate among A2 deliveries |
|---|---|---|---|---|---|---|
| 1 | before | 0 | — | — | 0 | — |
| 1 | after | 0 | — | — | 0 | — |
| 2 | before | 8 | 0/8 | 2/205 | 2 | 2/8 = 25% |
| 2 | after | 3 | 2/3 | 2/322 | 2 | **2/3 = 67%** |
| 3 | before | 4 | 0/4 | 0/131 | 0 | 0/4 = 0% |
| 3 | after | 3 | 3/3 | 0/102 | 0 | 0/3 = 0% |

Seed 1 delivers nothing in either arm (no windfall ever survives a bite
this run) -- the correct quiet reading, not a repeat of the bug.

**Seed 2: a majority, and both germinations fired on their very first
check.** One (347,161) was already wet without any redirect; one
(144,162) is the midden's own -- `pips_set_on_soil` moved 0/8 -> 2/3, and
the germination rate among deliveries moved 25% -> 67%.

**Seed 3: the redirect fires 100% of the time and still delivers nothing,
and this is a real, unresolved finding rather than a bug in the counter.**
All three deliveries landed on ground that read wet enough *at the moment
of delivery* (`pips_set_on_soil` 0/4 -> 3/3) -- yet the two that lived long
enough to be checked read exactly `0.00` a few frames later
(`SEED_TICK_INTERVAL` is 4). Live hypothesis, not chased further this
round: `pip` is a `Powder` that falls through organisms and settles by the
full 2D physics, not merely down -- `find_midden_site` tests the ground
under `creature::colony_surface`'s chosen surface cell, and the grain can
still slide off that exact cell onto a different, drier one before its
first Germinate check, the same "a freshly delivered pip can fall for
several ticks before it rests" gotcha this file's own `deliver_seed_
passenger` doc already names for a different reason. Reported per this
round's own instruction (report the threshold, don't chase it) rather than
building a settle-aware placement this round does not have the measurement
to size.

**None of the four germinations across both arms stood within 32 columns
of the founding nest (`plants_from_pip_near_nest` = 0 on every run).** The
drop verb puts cargo down wherever the ant happens to be, not only at the
nest (`creature.rs`'s own doc on the drop site) -- "put down: wherever the
ant happens to be, not only at the nest: 'put down' is one verb." The
midden only ever relocates a pip a handful of columns from wherever the
ant already dropped it, so it cannot pull a garden to the door on its own.
**The next lever, for whoever picks this up: the garden comes up at the
door only when the ant carries the pip home in the first place** -- today
`seeds_delivered` means "dropped anywhere", and `AtNest`'s own weight
biases a drop toward the nest without making it the only place a laden
ant ever lets go. Out of this round's scope (it touches the drop verb in
`creature.rs`, not the set-down this lane owns), and reported rather than
built for the same reason garden-rot reported the water threshold instead
of chasing it.

## No card this round

The coordinator's revised card brief asked for a nest-door frame sequence,
20,000+ frames past first delivery, seedlings visibly growing. Two
problems, both real findings rather than a rendering failure: (1) neither
of seed 2's two germinations stood within 32 columns of the nest (above),
so there is no "at the door" to film -- a doorside card would either be
staged (misleading) or empty (uninformative); (2) `labgif` could not find
either to film in the first place -- see `Reports/instruments.md`'s
`labgif` row, appended this lane: **`labgif` (`Lab::tick_for_harness`) and
`labforage` (`LabBox`'s own loop) do not reproduce each other's trajectory
on the same seed and the same rain setting from frame 0** -- probed
directly (`A2_DEBUG=1`, no drawing) on seeds 2 and 3, 140,000 frames each,
zero deliveries in either, where `labforage` on the identical seeds
delivered 3-8. So a `labforage`-measured event cannot be rendered by
`labgif` at all today, on any seed this lane tried. Left unchased -- the
two harnesses' divergence is a `labgif`/`Lab` question, not a
`deliver_seed_passenger` one. The numbers above are the deliverable this
round.

## Files touched

- `src/sim/plant.rs` -- `site_holds_enough_water`, `seed_water_threshold`,
  `find_midden_site`, `MIDDEN_SEARCH_COLUMNS`, the redirect and
  `PIXEL_PHYSICS_MIDDEN=0` kill switch in `deliver_seed_passenger`, three
  new tests.
- `src/sim/world.rs` -- `pips_set_on_soil`/`pips_set_on_nest`.
- `examples/labforage.rs` -- SUMMARY line append (`pips_set_on_soil`,
  `pips_set_on_nest`), main's fields kept first.
- `wiki/ants.md`, `wiki/plants.md` -- freshness-noted updates to the garden
  paragraph and "The forest floor".
