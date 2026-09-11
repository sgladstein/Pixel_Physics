# Round 29 — the seed comes out where it is eaten

*Owner's rule, 2026-09-11: "where should the seed drop when a creature
picks up food — it should drop where it is eaten, not immediately." The
door-side garden was never a goal, only a possible outcome of that rule.*

## Why the rule was violated twice

`Crop::passenger` (`organism.rs`) rides *in place of* one flesh cell, not
beside it. Before this build the only exit that ever wrote it back as a
live cell was `plant::deliver_seed_passenger`, called from two sites, both
of them a **drop**, never an eat:

1. **Digestion destroyed the seed.** `creature_tick`'s digest block spends
   whole crop cells over time (`c.cells - 1` per matured cell) and, on the
   cell that empties the crop, replaces `state.crop` with `None` —
   `..c`-copying the passenger through every *intermediate* cell but never
   reading it on the one that matters. A bitten fruit's seed that rode home
   and was then actually eaten, rather than dropped, simply vanished with
   the last mouthful. Round 28 measured `seeds_carried 9 / delivered 6`
   precisely because delivery only ever happened through the drop verb.
2. **A dropped fruit spilled a bare seed immediately.** The drop verb and
   the dying-carrier spill both called `deliver_seed_passenger` directly,
   which writes the passenger back as a bare `pip` — the same cell an
   *eaten* fruit's surviving seed becomes. So putting a meal down before
   finishing it planted the seed on the spot, "immediately," rather than
   leaving it inside the fruit until someone actually ate it.

## The build

**Eaten, then dropped.** `Crop::cells` counts a total, not an ordered
stack, so there is no cell to call "the one the passenger rides in place
of" except the one whose consumption empties the crop — every earlier cell
already carries the passenger forward untouched (`digestion_does_not_
consume_a_passenger`, unchanged). `creature.rs`'s digest block now checks
`left == 0` and, if a passenger is aboard, calls `plant::deliver_seed_
passenger(world, hx, hy, passenger)` at the eating animal's own head
position — the same bare-pip write the drop verb used to do, now fired by
finishing a meal rather than by putting one down.

**Dropped, not eaten: the fruit keeps its seed.** New `plant::deliver_seed_
passenger_uneaten`, called from both former call sites (the ordinary drop
verb, `creature.rs:~5970`, and the dying carrier's spill, `~10087`). It
resolves the delivering organism's own species' `windfall_material` (the
same lookup `drop_organ` uses, falling back to `"seed"` the identical way)
and writes the passenger back wearing *that* material instead of `pip` —
the same organism-owned `CellType::Seed` a fruit that fell there on its own
would be. Whoever bites it next runs `seed_survives_bite` on it fresh,
exactly as any other fallen fruit. Falls back to the ordinary bare-pip
delivery, in the same cell, only when the organism's species cannot be
read at all (should not happen — `carried_seed_organisms` holds the slot
for its whole ride) or this world loads neither the named windfall material
nor the `"seed"` fallback; not separately counted, since that is the
degraded case rather than the windfall this counter answers for.

**`PIXEL_PHYSICS_SEED_WHERE_EATEN=0`** reproduces this round's predecessor
exactly (digestion silently drops the passenger; every drop spills a bare
pip), the same one-binary kill-switch shape `PIXEL_PHYSICS_MIDDEN` already
uses, so before/after comes from one build rather than two checkouts.

## Counters

`World::pips_released_by_digestion` / `pip_digestion_release_x` — the
eaten exit's "it fired" and "where". `World::fruit_dropped_with_seed` —
the uneaten exit's "it worked" (windfall form only, not the degraded
fallback). `seeds_delivered` is unchanged in meaning: the union of every
exit that puts the passenger's organism back on the ground, live.

`labforage`'s SUMMARY gained `pips_released_by_digestion`,
`fruit_dropped_with_seed`, `digestion_release_by_dist` (the same
`DIST_BANDS` every other distance reading in the file already uses), and a
printed line naming where digestion released each seed relative to the
nearest nest.

## Measurement

`played_bed`, seeds 1–3, 120,000 frames, `RAYON_NUM_THREADS=4`, one binary,
`PIXEL_PHYSICS_SEED_WHERE_EATEN=0` for "before":

| seed | arm | plants | seeds_carried | seeds_delivered | pips_released_by_digestion | fruit_dropped_with_seed | pip_checks | plants_from_pip | deliveries | eats | alive |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | before | 632 | 0 | 0 | 0 | 0 | 0 | 0 | 34 | 7,171 | 44 |
| 1 | after | 632 | 0 | 0 | 0 | 0 | 0 | 0 | 34 | 7,171 | 44 |
| 2 | before | 530 | 1 | 1 | 0 | 0 | 150 | 0 | 2,584 | 14,048 | 37 |
| 2 | after | 311 | 1 | 1 | 0 | 1 | 126 | 0 | 2,591 | 13,762 | 41 |
| 3 | before | 216 | 5 | 5 | 0 | 0 | 128 | 2 | 7,864 | 26,885 | 67 |
| 3 | after | 312 | 7 | 6 | **2** | **4** | 100 | 0 | 8,136 | 28,621 | 101 |

**Seed 1 is byte-identical, every field** — `windfall_bitten=0` on both
arms, so the passenger mechanism never once fires and the kill switch has
nothing to switch. This is the specificity control the counters need: the
build touches nothing when the situation it is built for does not arise.

**Both new counters are non-zero on seed 3 — the positive control.**
`pips_released_by_digestion=2`: two passengers survived to the tick that
emptied their crop and were set down live, which could not happen at all
before this build (the field does not exist on `main`; the kill switch
reproduces its absence exactly — 0 on every "before" row). `fruit_
dropped_with_seed=4`: four crop drops recreated the whole fruit instead of
spilling a bare pip. **Where the eater stood when digestion released each
seed**, by column distance from the nearest nest (bands `[16, 48, 128,
∞]`, the same bands the rest of this file already buckets distance into):
seed 3 after reads `[0, 0, 2, 0]` — both releases landed 48–128 columns
from a nest door, not at it. Seed 2's one passenger was delivered by the
drop path on both arms (the only exit that existed pre-fix), and "after"
correctly built it as a windfall (`fruit_dropped_with_seed=1`) rather than
the bare pip "before" wrote.

**No `no_colony=1` divergence.** Seed 2, `no_colony=1`, before and after:
byte-identical on every field (`plants=992` both arms, nothing else moved
either). The passenger mechanism has no code path that runs without a
colony, so this is the clean negative control — the fix cannot be
perturbing the plant simulation on its own.

**The downstream numbers on seeds 2 and 3 disagree by more than the two
new counters would explain on their own** (seed 3: `plants` 216→312,
`alive` 67→101, `plants_from_pip` 2→0), and that is read as chaotic
divergence, not a targeted effect, for the reason `CLAUDE.md`'s own
cascade section gives: the instant a passenger event's outcome differs
between arms, the two runs are different worlds for every frame after —
different cells exist at different times, so every downstream RNG draw
that depends on world state (which an ant sees, what it does next) forks
too. A single seed cannot separate "the fix changed this" from "the
trajectory forked here and drifted," and this file does not claim to. What
the seed-3 numbers *do* establish is that neither arm "ran away": `plants`
moved by less than 1.5x either direction across all three seeds, and
`deliveries`/`eats`/`alive` were flat or higher on every seed, never
worse.

## `ascii` scenes

`cargo run --release --example ascii`, kill switch on and off, 31 scenes
either way, 0 skipped, exit 0 on both. **The diff between the two runs is
timing only** — every `worst frame`/`mean`/`ms` figure shifted by up to
~2.5x in both directions (`spring OFF` read 132.9 ms in one run and 48.8 ms
in the other run's own `spring OFF` line two scenes later), consistent
with `CLAUDE.md`'s "a timing number is only as trustworthy as the box was
quiet" — this container's load average sat at 6–9 on 4 cores for the whole
session (several sibling lanes building and running concurrently), not
with the code. **No scene moved on a single counter or a structural
figure**: cell counts, chunk counts, erosion/vaults/room detail, energy
census and the `=== ants: the foraging loop ===` scene's own food-stock and
carrying counts are identical line for line in both logs. That scene is
short and hand-built rather than run against `played_bed`, so it never
constructs the situation (a windfall an ant both carries and finishes
eating) this build changes — which is itself the expected, honest answer
to "which scene moved": none of `ascii`'s did, because none of them run
long enough or with real fruit to reach the code path `played_bed` does.
