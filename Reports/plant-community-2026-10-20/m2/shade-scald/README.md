# A5: a price for shade tolerance, and the dark floor under a tree (2026-10-22)

Plan step 7 (`Reports/plant-community-plan-2026-10-20.md`, A5.1-A5.2). The
shade genes were built and measured on 2026-10-21; the open item was the
price. `SHADE_LEAF_PRICE` 0.9 made tolerance *neutral* for a lone shrub in
bright light (larger on 8 of 16), never a loss, because a lone plant there
is sized by development and not carbon, so an income cut changes nothing it
builds (`../../m1/shade-price/`). Selection had duly put tolerance where it
was free: the seed-trained herb carries `shade_tolerate` 0.53, the trained
shrub 0.

## The price: sun scald (`World::shade_scald`, off by default)

A tolerant plant's leaves now shed in bright light, the mirror of the shade
roll: `SCALD_RATE * shade_death * shade_tolerate * glare^3` per upkeep
tick, glare against `World::open_light`, booked `DeathCause::Scorched` and
counted in `World::shed_scald`. A leaf lost is a cost to what the plant
*builds*, which is the one thing that reaches a plant that is not short of
carbon. Thin shade leaves moved into the sun photo-inhibit and bleach; that
is the real trade. The draw is taken only for a tolerant plant with the
switch on, so every shipped species keeps its dice (the `scald=0` arm
reproduces the old run to the cell, `runs/scaldrate_ctl.txt`).

Lone shrub, `shade_tolerate` 0.7, 16 seeds paired with the same shrub with
no tolerance, 60,000 frames (`plant_m0 mode=plastic axis=shade`; one
`SEED` line per seed in `runs/`):

| scald rate | bright: tolerant smaller | median size ratio | scald events, median |
|---|---|---|---|
| off | 9 of 16 | 0.93 | 0 |
| 1 | 10 of 16 | 0.98 | 19 |
| 4 | 10 of 16 | 0.93 | 70 |
| **16 (shipped value)** | **13 of 16** | **0.74** | 170 |
| 64 | 14 of 16 | 0.64 | 305 |
| 256 | 15 of 16 | 0.44 | 410 |

In shade the scald does not fire (0 events at lamp 0.2, all rates), and at
lamp 0.4 it is negligible (22 events at rate 64, size ratio 1.03). The
creeper, whose leaves lie fully exposed on the ground, pays more: smaller on
16 of 16 in bright light at 16 (shoot 91 -> 36), and 6 of 16 alive against
3 in shade (`runs/creeper_scald16_*`).

**What tolerance buys, by light** (lamp fraction; the lone shrub's seed site
reads 0.6 x lamp of full light):

| lamp | light at the leaves, of open | alive, no tolerance | alive, tolerance 0.7 |
|---|---|---|---|
| 1.0 | ~0.7 | 16/16 | 16/16 (smaller with scald) |
| 0.4 | ~0.35 | 15/16 | 14/16 (neutral) |
| **0.2** | ~0.18 | **0/16** | **5/16** |
| 0.1 | ~0.09 | 2/16 | 3/16 |

`train_plant::arena::Station::SHADE` (lamp 0.2) is that third row: where
tolerance earns its keep. On the trainer's whole-life rung the trade shows
on both sides (`runs/rungs_shrub_tolerance_scald_*.txt`, 8 layouts): in
bright light the end shoot falls 407 -> 401 -> 347 -> 285 as tolerance goes
0 -> 0.35 -> 0.7 -> 1.0 with scald (flat without it); in the shade station
establishment rises 1/8 -> 2/8 -> 2/8 -> 4/8 either way.

## Training did not pick tolerance up

`plantrain rung=hold neighbours=tree stations=bright,shade` (beside the
shipped tree; pop 16, 16 generations, 8 genes a mutation at sigma 0.9;
`train/`). **Trap: `rung=life` ignores `stations=`** -- it grows every bed
in the bright `bed` -- so a shade station needs `hold` or `grow`.

| run | final set vs shipped | champion `shade_tolerate` | shade station, est |
|---|---|---|---|
| shrub, scald on | +0.50, ahead 8/16 | 0.05 | 1/12 -> 10/12 (beside tree) |
| shrub, scald off (control) | +0.30, ahead 7/16 | 0.05 | 1/12 -> 11/12 |
| creeper, scald on | -0.01, ahead 8/16 | 0.0 | 1/12 -> 12/12 (life) |

Both shrub runs found the same lineage (same seed, and scald cannot touch a
plant with no tolerance), and it establishes in the shade station by
*shape* -- longer plastochron and internode, more juvenile branching -- not
by tolerance. The creeper raised its photosynthesis rate (0.5 -> 0.82 on the
tip), which has no price at all; that is the free lever the soil-water
thread named, and while it is free, selection will take it before any
priced trait.

## The dark floor

The real target is a grown stand. In the plant community lane's stand probe
(`plant_m0 mode=stand`, a tree+herb+creeper stand grown 60,000 frames, 192
wide), the floor under the tree reads **2-3% of open light**
(`runs/stand_trace_seed1.txt`) -- a closed forest's 1-5%, so the canopy is
not too opaque. Every sown shrub failed, shipped and tolerant alike, and so
did the residents' own herb and creeper over the window.

Traced one seedling at a time (`plant_m0 ... trace=500 whys=1`, lamp 0.05,
seed site 5% of open):

1. A shrub seed under the stand never germinates: `Germinate.light_threshold`
   is 0.1 absolute, and the floor reads 0.05-0.08.
2. Where it can germinate (5%), the seedling's only shoot tip is `TooPoor`
   four ticks running, retires on `ORGANISM_STALE_LIMIT` (~180 frames), and
   the plant is booked `LostVitalTissue` **with its carbon unspent**. A
   90% cheaper upkeep (`PIXEL_PHYSICS_SHADE_UPKEEP_CUT`, 0.5/0.75/0.9) and a
   seed carrying its parent's stake (`endow=0.25`; planted seeds carry 0)
   change nothing: 0/12 alive at lamp 0.05 in every arm.
3. **`World::seedling_waits` (off by default)**: a poor shoot tip does not
   age while its plant's leaves earn nothing, so a seedling waits for its
   first leaf. It cannot reach the dead end it sits beside (`found_candidate`
   on the carbon gate grew starved trees forever), because a plant with any
   income retires tips as before. Lamp 0.05, tolerance 1.0: 0/12 -> 2/12
   alive; untolerant 0/12 either way.
4. The waiting seedling then meets two more walls: its first node cannot
   afford a leaf (`OrganUnaffordable`), and its root cell dies unpaid.

5. **Judged in the stand, 12 seeds** (`runs/stand/`: 192 wide, stand grown
   60,000 frames, 4 seeds sown, 80,000-frame window, scald on). Shrub and
   creeper, shipped and with `shade_tolerate` 1.0, `seedling_waits` off and
   on. **Alive at the end: 0 of 12 in all eight arms.** A sown seedling
   reaches 1-5 shoot cells on 3-7 of 12 beds and is gone by 30,000 frames.
   Tolerance changes the trajectory slightly (the arms diverge on 2 seeds),
   never the outcome. `seedling_waits` changes the residents too (tree
   plants 16 -> 12 on seed 1): it is a world rule, not a shrub trait.

**So tolerance, however priced, cannot open this understory on its own.**
The light is realistic; what fails is a seedling's first weeks, at three
gates in order -- germination's absolute light threshold, the first growth
step (partly fixed by `seedling_waits`), and the first leaf and the root
that must be paid for before any income arrives. Each is an engine rule
shared by every species, so each needs its own measured change, and
selection will keep finding the free photosynthesis rate before any of
them. A tree that leaves gaps is the other route. Owner's decision, open
at hand-off (2026-10-22).

## Switches and instruments

- `World::shade_scald` (off), `PIXEL_PHYSICS_SHADE_SCALD=1` (trainer beds),
  `PIXEL_PHYSICS_SCALD_RATE` (sweep); `plant::SCALD_RATE` 16.
- `World::seedling_waits` (off), `World::seedling_waits_held`.
- `PIXEL_PHYSICS_SHADE_UPKEEP_CUT` sweeps `SHADE_UPKEEP_CUT`.
- `plant::endow_seed`, `plant_m0 endow=X`: give a planted seed a bred
  seed's carbon.
- `plant_m0 scald= waits=`, and `SEED` lines now print `shed shade`,
  `scald` and `waits` counts.
- `Station::SHADE` (lamp 0.2) in the trainer.
