# Plants that live with no roots (2026-10-23)

**Status:** finding and fix, **on by default since 2026-10-23** (the
owner's choice, "turn it on everywhere"); `PIXEL_PHYSICS_ROOTS_REQUIRED=0`
restores the old engine. It landed off first (PR #837). Off is bit-identical to main (the off arm below reproduces the
plant-community thread's `leafdepth/community/d1` numbers exactly: tree leaf
36,429, roles a bed 2.62, light under cover 0.155).

## The complaint

The owner, on the four-role lab bed (`plant_m0 mode=community mix=roles`,
seed 5, 450,000 frames): *"I still often see plants like this one which seem
to have no roots but are still surviving"*, then *"roots seem more just a
visual but they should actually be functional. This is a simulation."*

## What it is

Not a render problem. The marked plant is a tree with **1,332 leaves and zero
root cells**, drinking through the trunk cells that touch the soil
(`plant_m0 rootcensus=1 below=150` draws the bed 150 rows deep and again with
each plant in one colour, roots darker: nothing below it). Leafy plants (5+
leaves) with no root touching soil, 8 beds:

| frame | today | of leafy plants | their leaf |
|---|---|---|---|
| 150,000 | 53 | 248 | 2,243 |
| 450,000 | 78 | 236 | 9,341 |

## How a plant gets there, traced

1. **Die-back eats the roots first.** A plant whose upkeep outruns its income
   sheds tissue, and die-back may take neither foliage nor anything beside a
   leaf or a tip. In a small leafy plant the only cells left to it are bare
   root cells. Of 173 root cells lost by small plants (12 or fewer root
   cells) by frame 50,000 on seed 5, **171 were die-back's**
   (`DeathCause::Starved`), all at water status 1.00. The plant was short of
   sugar, not water, and it cut off its water to pay.
2. **Stems drink.** `Absorb` runs on every `MatureBody`, shoot as well as
   root, so the one to three stem cells lying against the ground kept a herb's
   demand met. Rootless herbs read `uptake == demand`.
3. **No roots reads as full nutrient** (`nutrient_status` defers to 1.0 with
   no root cells, for the seedling's sake).
4. **Nothing can root it again.** `break_root_tips` only turns an existing
   root cell into a tip. The doc on `rootless_dries_default` says thirst
   sends roots looking for soil, but that holds only for a plant that still
   has root wood.
5. A seed lying on something a root cannot enter (litter, wood) also
   germinates with no companion root (`germinate`). That is rarer here.

## The fix (`World::roots_required`)

- `absorb_water` runs on root tissue only (rootwood or a root tip).
- Die-back passes over a plant's last `DIEBACK_ROOT_FLOOR` (2) root cells.
- A plant past its seed with no root reads nutrient 0.
- A thirsty plant with no root to branch from sprouts an **adventitious
  root** from a stem cell against moist ground (never into free water), in
  root material, and its thirst is read from water status rather than the
  root zone's "nothing to say" 1.0.

A rootless plant now either roots again (and the roots show) or dries out
leaf by leaf through `drought_death`. Guard:
`a_herb_that_loses_its_roots_lives_on_its_stem_only_with_roots_required_off`.
Off, a herb stripped of its roots stays rootless and drinks 22.5 through its
stem. On, it sprouts 3 roots and drinks 0 through stems.

## Measured: 8 beds, paired, 450,000 frames, one binary

(`summary.txt`, logs `{off,on}_s{1..8}.txt`, runner `job.sh`)

| at 450k | off (today) | on |
|---|---|---|
| rootless leafy plants | 78 of 236 | **9 of 239** |
| leaf on rootless plants | 9,341 | **259** |
| tree leaf | 36,429 | 35,227 |
| shrub leaf | 1,925 | **5,353** (higher on 6 of 8 beds) |
| herb leaf | 5,104 | 5,208 |
| creeper leaf | 97 | 0 |
| roles a bed (100+ leaf) | 2.62 | **2.88** |
| light under cover | 0.155 | 0.170 |
| run time, mean | 401 s | 408 s (noise) |

Each switch-on run sprouted 34 to 230 adventitious roots by 450k, and
die-back passed over 3,396 to 9,440 last-root cells. Two beds went the other
way (seed 3: 42 leafy plants to 15; seed 6: trees closed the bed, 2 roles),
which is within this bed's known spread (the plant-community plan's seed
repeatability notes). The understory gains because rootless trees no longer
hold crowns up on trunk-drinking.

## Probe

`plant_m0 rootcensus=1` at each `shots=` frame prints `ROOTS` (one row a
plant: roots in and below the picture, contact roots, water and nutrient
terms), `ROOTLOG` (history of every rootless plant: roots/contact/shoot/stem
cells on the ground every 1,000 frames), `ROOTLOSS` (what each lost root cell
of a small plant became, plus the switch's two counters) and writes
`_owners.png`. Use `below=150` to see the whole root depth.

**The pictures shown to the owner** are in `rootless-plants-2026-10-23/img/`:
[the marked rootless tree](rootless-plants-2026-10-23/img/marked-tree-seed5-450k.png)
(seed 5, 450k, roots required off),
[today against roots required](rootless-plants-2026-10-23/img/today-vs-roots-required-450k.png)
and [its owner map](rootless-plants-2026-10-23/img/today-vs-roots-required-450k_owners.png)
(each plant one colour, roots darker), and
[nutrient by root](rootless-plants-2026-10-23/img/nutrient-by-root-450k.png)
(three paired beds, base on top).

## Side finding: a growing tip hanging off a leaf

Turning the switch on reddened `shedding_every_leaf_does_not_disconnect_the_stem`.
Swept over 16 individuals of its scene (8,000 frames), shedding every leaf
cuts one cell off the stem, a growing tip whose only neighbour is a leaf, on
**5 of 16 trees with the switch off and 7 of 16 with it on**. The guard looks
at only the first individual with a leaf, which was clean on the old engine
and is not on the new one. So the defect predates this switch and the guard
was already blind to it. The guard now runs with `roots_required` off, and
its doc carries these numbers. Not fixed here.

## Nutrient grows with root (`World::nutrient_by_root`, on), 2026-10-24

The owner asked whether a plant can live on a single root. For water, no:
uptake is per soil face and the store is `WATER_SCALE` per drinking root.
For nutrient, yes: `nutrient_status` was the **mean** richness over the
faces the roots touch, so one root in rich soil read as fully fed. The owner
said to test the fix. `World::nutrient_by_root` makes the status
the smaller of that mean and the **sum** over the faces divided by what the
foliage needs, `NUTRIENT_FACES_PER_LEAF` (0.5) faces of full soil per leaf or
growing tip. Both nutrient prices read it.

**The constant comes from the stand.** Over 319 plants (5+ foliage cells,
seeds 1-4, 50k and 150k, roots required on), soil faces of root per foliage
cell were p10 0.06, p25 0.26, p50 1.46, p75 2.85. Big trees (500+ foliage)
had a median of 0.29. At 0.5 the median plant of every role but tree is
untouched. A median tree reads about 0.58 of its mean, which the
Michaelis-Menten curve (`Km` 0.05) turns into about 0.95 of its income. The
bottom tenth reads short.

Guard: `a_herb_on_one_root_reads_short_of_nutrient_only_with_nutrient_by_root_on`.
A herb with 64 foliage cells on one root reads **0.022** with the switch on,
against **0.345** (its patch's own richness) off.

**8 beds paired, 450k, roots required on in both arms**
(`rootless-plants-2026-10-23/nutrient-by-root/`):

| | base | + nutrient by root |
|---|---|---|
| root cells per foliage cell, 150k / 450k | 0.68 / 0.81 | 0.80 / 0.86 |
| faces per foliage, bottom tenth, 450k | 0.07 | 0.14 |
| nutrient status, bottom tenth, 450k | 0.34 | 0.15 |
| tree leaf, 150k / 450k | 35,637 / 35,227 | 30,511 / 31,243 |
| shrub leaf, 150k / 450k | 7,139 / 5,353 | 8,191 / 4,882 |
| herb leaf, 150k / 450k | 4,881 / 5,208 | 5,295 / 5,210 |
| roles a bed, 150k / 450k | 3.00 / 2.88 | 3.12 / 2.88 |
| light under cover, 450k | 0.170 | 0.185 |
| run time, mean | 335 s | 321 s |

So plants carry more root per leaf, the root-starved tail is taxed as
intended, trees grow about 11-14% less leaf, and the understory is about the
same by 450k. Seen in pictures, the beds differ about as much as any two
seeds do. The base arm reproduces the switch-on arm above exactly. Landed
off in PR #846; the owner then chose **on** (2026-10-24), and
`PIXEL_PHYSICS_NUTRIENT_BY_ROOT=0` restores the old engine.

## How realistic is the nutrient model? (2026-10-24)

The owner asked, after turning `nutrient_by_root` on. The working doc is
<https://claude.ai/artifact/HELUJxoRzov7XNEvGSBcaE>; this section is its
record. Sources were read through search summaries, not opened in full.

**The core matches the textbook.** Real uptake grows with absorbing root
surface touching soil, saturates per unit surface (Michaelis-Menten, the
Barber-Cushman model's kinetic), and demand is set mainly by leaves, which
hold most of a plant's nitrogen. A nutrient-short plant shifts growth to
roots (Brouwer's functional equilibrium; held up qualitatively in Poorter
and Nagel's review, *Aust. J. Plant Physiol.* 27:595, 2000). The engine has all of
these: face-summed uptake, `Km` 0.05, `NUTRIENT_FACES_PER_LEAF`, the 8x
build price with root tips exempt, and the income multiplier.

**Missing, by size of the gap:**

| Real plants | Engine | |
|---|---|---|
| Roots proliferate in rich patches (Drew 1975, barley laterals in a fed patch) | Tips steer toward water only; `soil_nutrient_fraction` is read only by `nutrient_status` | missing |
| An internal store: luxury uptake, remobilised later (*Bistorta*: stores met 56-100% of early-season shoot N) | Status recomputed from soil every organism tick | missing |
| Resorption before leaf fall: ~62% of N, ~65% of P (Vergutz et al. 2012, *Ecol. Monogr.* 82:205) -- the reason leaves yellow first | A shed leaf takes nothing back | missing |
| Nitrate moves with soil water (mass flow); phosphate diffuses only 0.2-1 mm (IPNI) | Soil nutrient never moves; inside the plant it rides the sap (`sap_flow`) | missing in soil |
| Depletion around roots; neighbours compete | Per-cell deficit; one face draws at the recovery rate (period 45 = draw cadence), so a cell runs down only under 2+ faces | partly |
| Litter returns nutrient | Time recovery instead; litter crediting reverted twice as a pump (`dead-ends.md`, biomass 1,718 -> 2,652) | simplified on purpose |
| Mycorrhizal hyphae (clover P depletion to 11.7 cm with, ~1 cm without) | none | missing |
| Several nutrients | one | fine at play scale |

**Ideas, ranked by what the owner would see:**

1. **Pale hungry leaves** -- leaf colour toward yellow-green as
   `nutrient_status` falls (real N chlorosis). Render only; quantise it so
   it changes in a few steps, or it keeps the dirty-rect skip from working.
   Recommended first: the 8-bed run above showed the mechanism works and
   cannot be seen.
2. **Roots toward rich soil** -- add a nutrient term beside the water lean
   in the tip's direction choice. Moves where roots go, not how many, so it
   leaves `allocate_to_frontier`'s calibrated sum alone.
3. **A store refilled by resorption** -- one float per plant; roots fill it,
   building draws it, a shed leaf returns ~60%. Gives a graded fade and
   yellowing before leaf fall. Changes what today's prices read, so it needs
   its own paired run and a re-derivation of `Km` and `NUTRIENT_STARVED_COST`.
4. **Nutrient carried by soil water** -- the biggest frame cost; measure first.
5. **A nitrogen-fixing species** -- must not rebuild the litter pump.
6. **Root fungi** -- a new living cell type; the largest build.

## Still open

- Under `nutrient_by_root`, grass's foliage is its growing tips only
  (`leaves_in_row` counts `Leaf` and `GrowingTip`), so grass is barely taxed.
  There is no grass in the four-role bed.
- 9 of 239 leafy plants still read rootless at 450k with the switch on. Most
  are plants on their way out, and a few herbs read water status 1.0. Not
  traced.

## Pale hungry leaves: idea 1, built and on (2026-10-27)

A plant's leaves fade toward yellow-green (`render::PALE_LEAF_TARGET`) in
three steps as it loses income to nutrient shortage. The step is keyed on
`plant::nutrient_hunger`, which is `1 -` the nutrient income multiplier, so
it shows what the shortage costs the plant rather than the raw status (soil
at status 0.5 is worth 0.95 of full income and draws as fed). Step edges at
15%, 40% and 65% of income lost, pulled 0.3, 0.55 and 0.8 of the way to the
target, with 0.04 of hysteresis so a plant on an edge does not flicker. A
rootless plant reads status 0 under `roots_required` and so draws fully
yellow. Render only: no rule reads a leaf's colour. On by default;
`PIXEL_PHYSICS_PALE_LEAVES=0` is the old look.

**How it keeps the dirty-rect skip.** `nutrient_status` is rewritten every
organism tick with no cell write, so no chunk is dirtied when it moves. The
renderer works out every plant's step once per draw, compares it with the
step that world was last drawn at (per world: the lab draws other chambers'
thumbnails through the same renderer), and unions only the leaf boxes of
plants whose step moved into the dirty region. Guard:
`a_plant_that_goes_hungry_repaints_with_no_cell_written`, checked to fail
with those boxes left out.

**Measured** (`labshot scenario=played_bed drawevery=1 drawfrom=440000`,
seed 1, both arms one binary, `PIXEL_PHYSICS_PALE_LEAVES=0` against on;
the world is identical, only the picture differs):

| frame | leafy plants | step 0 / 1 / 2 / 3 | hunger p90 |
|---|---|---|---|
| 150,000 | 14 | 12 / 0 / 1 / 1 | 0.07 |
| 300,000 | 10 | 9 / 0 / 0 / 1 | 0.18 |
| 450,000 | 21 | 20 / 1 / 0 / 0 | 0.05 |

One or two plants are pale at a time; the step-3 plants are rootless herbs.
Draw cost over frames 440,000-450,000, drawn every frame: mean 7.096 ->
7.162 ms (+0.9%), p99 11.98 -> 12.03 ms, pixels redrawn per frame 157,807 in
both (this bed redraws almost the whole screen every frame anyway).
`ascii`'s full-screen render stress scene, 8 alternating pairs on a loaded
machine: worst render frame higher with pale leaves on 4 of 8 (sign test
p = 1.0), medians 0.64 ms on against 0.71 ms off. Two full `ascii` passes per
arm were too noisy to read: a scene the change cannot reach (CA + field, no
drawing) read 119 ms against 16 ms on one pair and 1,342 against 15 on the
other, three latecensus runs sharing the four cores.

Pictures (each pair the same world and frame, old look then new): <https://claude.ai/artifact/WTLCb74bBWoiu8nKNvz9F9>, copies in `rootless-plants-2026-10-23/pale/` -- [the rootless herb](rootless-plants-2026-10-23/pale/herb_rootless_150k.png), [a herb at step 2](rootless-plants-2026-10-23/pale/herb_062_150k.png), [a tree at step 1](rootless-plants-2026-10-23/pale/tree_038_450k.png), and a stretch of the bed [before](rootless-plants-2026-10-23/pale/bed150_off.png) and [after](rootless-plants-2026-10-23/pale/bed150_on.png).
`labshot` now prints a `pale leaves` census per stop with the hungriest
plants' leaf boxes, and `latecensus` a `kinds` line (plants / leaf cells /
pale plants per species).
