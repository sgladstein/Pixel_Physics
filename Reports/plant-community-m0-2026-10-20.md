# M0 of the plant community plan: baselines and feasibility (2026-10-20)

**Status: complete.** Development, cost, copy, the shrub trace, the 42
community beds, the load-failure check and the flood curve are all in. Plan of record:
[`plant-community-plan-2026-10-20.md`](plant-community-plan-2026-10-20.md)
§8.1. Logs: `Reports/plant-community-2026-10-20/m0/`.

## 1. What this changes in the plan, in short

**The community results** (§6), first:

- **Today's bed holds one to three kinds**, and the survivors mostly do not
  replace themselves.
- **Ants make room for the annual herb** (5 of 8 beds with ants, 0 of 12
  without). The colonist role exists.
- **Grass fails at recruitment, and the seed pile is the same failure.**
  Under the lab default, seedlings sprouting on the pile are felled and grass
  dies out. Under the owner's setting (load failure off), they live, the pile
  never clears (40 deep), and grass carpets the bed.
- **The default mister floods every bed past about 200,000 frames** (§Z38),
  so the first M1 step is the watering. With the mister off, the soil has
  the dry-top, wet-deep profile a rooting-depth niche needs.
- **The slice is crowded.** 62–86% of shoot tips touch another plant, and a
  quarter to a half are blocked from growing up. A6's passage rule is
  warranted.
- **Tree and conifer canopies pass a third of the light** to the soil (an
  understory is possible). **Shrubs pass 86%**: the canopy role needs a
  denser start or training.

**The development and feasibility results** (§2–§5):

1. **The per-plant key freezes the dice; it does not shrink them.** One herb
   genome, one spot, eight developmental seeds: 128–382 cells, a spread of
   31%. Today's spot-to-spot spread is 41%. Part A has to make the dice
   smaller (A2–A3). The key choice (A4) only matters afterwards.
2. **The shrub's "half stop, half grow" has a found cause, and it is two
   rules, not one coin** (§3). It is the first concrete target for A3.
3. **Water already moves plants the right way, by more than the noise.**
   Light, as M0 tested it, barely does. The test was too weak, not the
   engine: lamps 17 columns apart light the bench almost evenly. The light
   arm is redesigned with a real shade.
4. **The ladder's cheap rungs are cheaper than planned** (cost tracks
   activity, not area), and **copying a grown bed works**, which is what makes
   the invasion rung affordable.

## 2. Development (M0.1–M0.3)

### 2.1 The dice

Size spread of one genome. Outdoor test bed, 12,000 frames, 8 plants per row;
`clone_variance shift=1` for spot to spot, and the new `seeds=1` arm for seed
to seed.

| species | today, spot to spot | per-plant key, spot to spot | per-plant key, seed to seed |
|---|---|---|---|
| herb | 0.41 (134–387 cells) | 0.04 (128–142) | **0.31** (128–382) |
| shrub | **1.33**: 22–55 cells or 1,131–1,863, nothing between | 0.02 (all 8 stalled at ~45) | 0.29 (all 8 stalled, 26–64) |
| grass | 0.22 | 0.22 | 0.29 |

- **Herb: the key freezes, it does not shrink.** With every spot sharing one
  seed the spread falls to 0.04. With the seed varied it is back to 0.31.
  The dice are as big as ever; the key only makes every plant roll the same
  numbers.
- **Grass: the key does not reach it.** Its spot-to-spot spread is 0.22
  under both keys. Grass tillers are drawn from world-keyed dice
  (`plant.rs:10779`, per the code survey), so the per-plant key never
  touches the part of grass that varies.
- **Shrub: there is no middle.** §3 traces why.

**Controls.** The same seed in the same spot reads the same plant: under the
per-plant key the spot arm reads 0.02–0.04, essentially zero. The seed arm
reads non-zero for every species, so the instrument can see dice.

### 2.2 The surroundings

One founder alone in the trainer's lab bed, 20,000 frames, 6 world seeds per
arm (`plant_m0 mode=plastic`). Each shift is in units of the spread inside an
arm. That spread includes the soil's texture, so it is the noise a trainer
faces.

| species | dim − bright (between lamps against under one) | dry − wet (mister off against Light) |
|---|---|---|
| herb | shoot −0.15, height −0.25, root −0.43 | **root +1.51, root depth +1.31**, shoot +0.20 |
| grass | **shoot −1.07, root −1.88**, height +0.52 | small (shoot +0.29) |
| shrub | swamped by the stall lottery (shoot spread larger than its mean); leaf share +0.81 | root +0.70, root depth +0.59 |

- **Water works.** A herb in a dry bed puts down more and deeper roots, by
  more than the noise. That is the allocation rule (`plant.rs:11555`) doing
  what a real plant does.
- **The light arm failed its own control.** The control said bright against
  dim must move size, and for herb and shrub it does not. The reason is the
  bed: with lamps 17 columns apart the bench is lit almost evenly, so
  "between lamps" is not dim. Grass, which sits at the soil, does respond. The
  light arm is redesigned for M1 with a real shade: a cover, or a canopy
  plant overhead. No light-response number from this run is used.

### 2.3 Genes

Not re-measured. The plan's figures stand: herb size heritability 0.034 under
today's key and 0.650 under the per-plant key (rethink §6.8), and P1's
gate (trainer plan §13). §2.1 says why the per-plant number is high: every
clone rolls the same dice. It is not a sign that the dice are small.

## 3. Why shrubs either stall or take off

**Asked by the owner, 2026-10-20:** *"Half stop at just a few cells and half
grow to full bushes, not sure why."* Traced with `planttrace life=1` on the
played bed, four seeds, 60,000 frames, no ants. Every shrub was followed
from seed to frame 60,000 (`m0/shrubtrace/`).

**The shrub's architecture makes it fragile by design.** Every shoot ends in
a flower after 4 nodes (`shrub.ron:143`), so the plant goes on only through
new tips: laterals spawned at nodes, and dormant buds breaking later.

**Two plants, the same spot in two beds:**

| frame 1,000 | shoot cells | growing shoot tips | income / upkeep per tick |
|---|---|---|---|
| grew (bed 1, id 3: 1,124 cells by frame 60,000) | 16 | 3 | 0.016 / 0.002 |
| stalled (bed 2, id 3: 20 cells for 48,000 frames) | 9 | **0** | 0.019 / 0.002 |

At frame 500 both had one tip, and the one that stalled was slightly ahead.
Between frames 500 and 1,000 it lost every shoot tip (flowered or gone
stale). The other kept three, which grew leaves and raised its income past
the cost of a new tip by frame 4,000.

**Then a rule makes the stall permanent.** A dormant bud breaks only if
`floor((noon income − maintenance) / growth cost) ≥ 1` (`break_buds`,
`plant.rs:10915`). That is **a per-tick flow, not the plant's savings.** A
shrub tip costs 0.2. The stalled seedling earns a surplus of about 0.017 a
tick, so it needs roughly 12 times its leaf area before any bud can break.
Its savings climbed the whole time (0.019 → 0.139 by frame 4,500) and never
count. It sat like that until frame 48,900, when it set a seed.

**So the lottery is two rules:**

1. **Dice in the first ~1,000 frames decide whether a seedling's few tips
   survive** until its leaves pay for growth (tips that miss four times go
   stale; tips flower after 4 nodes).
2. **Bud break that reads income, not savings, makes losing that roll
   permanent.** Thickening killing buds (chance 0.3 each for shrub) can
   only make it worse.

**What real plants do instead** (plan §3.1, branching row): bud outgrowth is
promoted by sugar availability, with sucrose acting as a signal against the
tip's auxin. A seedling that loses its tip resprouts from an axillary bud on
its reserves, so the outcome is graded: a late, smaller shrub, not a frozen
seedling. **For A3:** bud break reads stored carbon (savings) as well as
flow, and a tip that is short of carbon pauses rather than retiring. Both are
measured against this trace: the share of shrubs that stall, and the spread
of shrub size, should fall together.

**Not yet traced:** which roll, in those 500 frames, decides whether a
seedling's tips survive (`planttrace growlog=1 only=ID` on the stalled id
would say). The fix above does not depend on it, because the second rule is
what turns a bad roll into a permanent outcome.

## 4. Feasibility (M0.7, M0.8)

**Arena cost** (one herb founder, one thread, with the other three cores
busy, so these are upper bounds):

| bed | 3,000 frames | 10,000 frames |
|---|---|---|
| 24 × 120 | **0.09 s** | 0.29 s |
| 96 × 120 | 0.21 s | 0.77 s |
| 24 × 320 | 0.21 s | 0.63 s |
| 192 × 320 | 0.59 s | 2.23 s |

Cost tracks activity, not area: 8× the width costs about 2.8×, and a 120-row
box costs half a 320-row one. **Rung 1 is about 0.1 core-seconds a bed**,
against the plan's 0.3. The played bed, full of plants, is about 1 core-second
per 1,000 frames.

**Copying a grown bed works.** A bed was grown for 20,000 frames, copied
twice, and both copies stepped 5,000 more: hash-equal, and equal to the
original carried on with its own particles. A species registered into a copy
and sown there germinated. It had one shoot cell after 5,000 frames, sown
among grown herbs, so its growth there is not a feasibility question.

## 5. Instruments, and the controls that failed

- **"Tips with no free neighbour" never fired**, even for herbs sown in a
  tight row: tips steer away first. It is replaced by **contact** (a tip
  touching another plant) and **blocked upward** (a tip's way up closed by
  another plant). Control: tight row 2 of 4 tips in contact at frame 2,000; a
  lone plant 0 until its own seedlings are up at 8,000.
- **"Light under cover" counted a grass blade at the soil as cover**, and
  was confounded with distance from a lamp. It is now each column's surface
  light divided by the same column's light 100 rows up, under a canopy (a
  plant cell 6+ rows up) against open columns.
- **The shrub control stalled.** A lone shrub at a lamp sat at 8–14 leaf
  cells for 40,000 frames, which is §3's lottery showing up in a control.
  The control was moved to herb.

## 6. The community beds (M0.4–M0.6)

**Setup** (`plant_m0 mode=community`):

- **The bed:** a 512-wide lab bed with lamps 33 apart (as the played bed),
  light rain, the lab's half pace, in-bed mutation off, and plant load
  failure on (the lab's default).
- **The sowing:** mix7 (2 each of herb, scrambler, shrub, creeper, grass,
  conifer, tree) or the four roles (3 each of shrub, creeper, herb, grass), at
  evenly spread columns that keep 210–310 bare.
- **Ants:** 52, founded at frame 6,000 in the ant arms.
- **The runs:** 450,000 frames, censused every 25,000; 12 world seeds without
  ants and 8 with.
- **Cost:** about 9 minutes a bed on one thread without ants, about 20 with.

Logs are in `m0/runs/`. Every figure is a median over seeds unless it says
otherwise.

### 6.1 Diversity today

| bed | kinds with 100+ leaf at 450k (min / p25 / median / max) | of those, still producing new plants in the last 100k | evenness (effective kinds ÷ sown) |
|---|---|---|---|
| four roles, no ants | 1 / 1 / **2** / 2 | **0** | 0.54 |
| four roles, with ants | 0 / 1 / **1** / 2 | 1 | 0.51 |
| mix7, no ants | 1 / 2 / **3** / 4 | 1 | 0.27 |
| mix7, with ants | 1 / 2 / **2** / 3 | 1 | 0.29 |

Seeds out of 12 (8 with ants) on which each kind holds 100+ leaf at 450k:

| kind | roles4 | roles4 + ants | mix7 | mix7 + ants |
|---|---|---|---|---|
| shrub | 9 | 4 | 3 | 2 |
| creeper | 11 | 1 | 2 | 0 |
| herb | **0** | **5** | 0 | 0 |
| grass | **0** | **0** | 1 | **0** |
| scrambler | — | — | 6 | 3 |
| tree | — | — | 10 | 5 |
| conifer | — | — | 10 | 7 |

- **Today's bed holds one to three kinds, and the survivors are mostly not
  replacing themselves.** In the four-role bed without ants, the median
  number of surviving kinds still producing new plants in the last 100,000
  frames is zero. The bed is living on its old individuals.
- **The canopy trees take mix7.** Tree and conifer survive on 10 of 12
  seeds; the low plants mostly do not.
- **Ants make room for the annual.** Herb holds on in 5 of 8 four-role beds
  with ants and 0 of 12 without. That is the annual colonist role living on
  disturbance (Grime's ruderal), and the clearest sign yet that the plan's
  roles exist in this engine.
- **Grass is lost everywhere**, falling in the second half on 12 of 12
  four-role beds (§6.2).

### 6.2 Why grass dies out

Traced over every grass on the played bed, seed 1, 450,000 frames, no ants
(`planttrace life=1`, `m0/grasstrace/`):

| stage | grass | lost there |
|---|---|---|
| set as seed | 20,188 | **16,512 rotted waiting**, 224 overrun |
| germinated | 2,860 (14%) | **1,322 felled at a median age of 142 frames**, 817 old age, 567 lost their tissue |
| established | 128 (0.6% of seed) | all set seed; 90 then died of old age |

**Grass fails at recruitment.**

- Most seed waits until it rots. The piles are this seed (§6.3).
- Most seedlings are felled within 142 frames of germinating. "Felled"
  means the seedling's tissue was cut loose and dropped as a falling body,
  which happens when a plant has no support path to the ground. A seedling
  that germinates on top of a seed pile has nothing solid under it, and the
  lab ships with plant load failure on.
- The owner's own playtest bed ran with load failure off (`replay.rs`). §6.7
  checks whether that alone keeps grass alive.

Since 2026-10-19 grass dies of old age (median about 60,000 frames). So a
grass stand that cannot recruit dies out.

### 6.3 Seed piles

| bed | seed cells above the soil, peak | deepest column, peak | at 450k |
|---|---|---|---|
| four roles, no ants | 2,612 (max 4,165) | 21 (max 28) | 35 |
| four roles, with ants | 1,564 | 19 | 65 |
| mix7, no ants | 1,178 (max 3,145) | 21 (max 31) | 70 |
| mix7, with ants | 726 | 11 | 81 |

- **The pile is living seed**, every seed cell carrying a waiting organism
  (`seedlive` equals `seed` at every row checked). It is mostly grass.
- **It peaks between frames 75,000 and 300,000, then shrinks as the grass
  dies out.** A pile that shrinks only because the plant making it died is
  not a fix.
- **Ants take some of it**: the peak is 40% lower with ants in both beds.
- Pictures of the four-role bed at frames 75,000 and 150,000 are in
  `m0/pics/`.

### 6.4 Water

**With the mister on Light, water is never short anywhere.** The driest any
band ever got is 0.616–0.629 of saturation in every bed, which is field
capacity, where the engine treats water as fully available. At 450,000 frames
the soil is wetter still (0.86–0.98). **There is no water niche in today's lab
bed.**

**With the mister off** (control, one seed), the top band dries to 0.24 by
450,000 frames while the deep bands hold 0.52–0.81. That is a real vertical
profile: dry above, wet below. It is the condition a shallow-against-deep
rooting niche needs, and the owner has allowed the watering to change. **For
M1: intermittent watering**, wet spells and dry spells, so the topsoil dries
between them and the deep soil does not. Measured by this profile before any
plant work relies on it. Rain off for good also killed herb and grass in that
bed, so off is not the answer.

### 6.5 The slice

| bed | shoot tips touching another plant | tips whose way up is blocked by another plant | gaps (3+ bare columns) per 100 columns, 450k |
|---|---|---|---|
| four roles, no ants | **0.62** | **0.24** | 2.6 |
| four roles, with ants | 0.50 | 0.14 | 2.0 |
| mix7, no ants | 0.21 | 0.08 | 1.8 |
| mix7, with ants | 0.28 | 0.09 | 1.2 |

- **The slice is crowded.** In the four-role bed, 62% of growing tips are
  touching another plant, and a quarter have their way up closed by one.
  The count includes kin (a plant's own seedlings are other organisms), so
  it is the crowding a plant meets, not only the crowding between kinds.
- That is high enough to justify **A6's passage rule**, which lets a tip push
  through another plant's leaf as it could in 3D. A6 measures it again after.
- Gaps of 3+ bare columns are rare: about 2 per 100 columns.

**Light at the soil**, as a fraction of the same column 100 rows up:

| bed | under a canopy | in the open |
|---|---|---|
| mix7, no ants | **0.31** | 0.80 |
| four roles, no ants | 0.86 | 0.89 |

- **Trees and conifers make a real understory:** a third of the light
  reaches the soil under them. That is in the range of an open woodland, a
  little bright for a closed one.
- **Shrubs barely shade** (0.86). The four-role bed's canopy role, started
  from the shrub, does not yet make the shade an understory role needs. The
  canopy role either starts from a denser species or is trained for it.

### 6.6 Controls

- **Crippled shrub** (photosynthesis ×0.1, four roles, seed 1): shrub leaf 0
  at 450k, against a median of 273 for the shipped shrub. The persistence
  measure sees a lost species.
- **Mister off:** the top band reads 0.24 against 0.62 or more with it on.
  The water measure sees a drying bed.
- **Determinism:** two runs of one bed are byte-identical (smoke test).
- **Not done here:** the 150,000-against-450,000 proxy check. It ranks
  *candidates*, and M0 has none; it moves to M2, where the first trained
  candidates exist.

### 6.7 Load failure off (the owner's setting)

The owner's playtest bed runs with plant load failure off (`replay.rs`). The
same four four-role beds (seeds 1–4, no ants), on and off (`m0/loadfail-off/`):

| | load failure on (lab default) | load failure off (owner) |
|---|---|---|
| grass leaf at 450k | 1 (extinct, 0 of 4) | **2,369 (4 of 4)** |
| seed cells on the soil, peak | 3,675 | **8,302** |
| deepest seed column | 25 | **40** |
| seed still on the soil at 450k | 34 | **8,302**: the pile never clears |
| kinds with 100+ leaf | 2 | 3 (herb gone in both) |
| reproducing in the last 100k (median) | 0 | 1 |
| shoot tips touching another plant / blocked upward | 0.62 / 0.24 | 0.86 / 0.54 |

**One switch flips grass from extinct to dominant, and neither is a mixed
bed.**

- **With load failure off**, a seedling that sprouts on the seed pile is
  not felled. Grass grows on top of its own seed, keeps the pile, and carpets
  the bed. That is the owner's "huge piles of seed" (picture:
  `m0/pics/seed-mound-loadfail-off.png`, a mound several times the plants'
  height with water trapped inside).
- **With it on**, those seedlings are felled and grass dies out.

**For the plan:** the seed pile is not a separate defect from grass's
recruitment. It is the same seed, either piling up and burying the bed or
rotting while grass dies. A6's seed fix is judged under the owner's setting,
where the pile is visible, as well as the lab default.

### 6.8 The mister floods the bed

Standing water above the soil (census `standing=`), seed 2, no ants
(`m0/flood/`):

| frame | four roles | four roles, load failure off | mix7 |
|---|---|---|---|
| 50,000 | 376 | 339 | 493 |
| 250,000 | 2,645 | 2,051 | 3,738 |
| 450,000 | **9,436** (deepest 25) | **6,864** (31) | **8,622** (58) |

- **It never levels off.** The mister was signed off over 120,000 frames
  (`lab/rain.rs`); past that the bed fills until every plant is under water
  (`m0/pics/flood-sequence.png`).
- **It is registered as bug §Z38**, because it touches every long run on a
  mister-on bed, the ant lane's included.
- **Some of what §6.1 calls late decline may be plants under water.** The
  four-role beds' "falling in the second half" (grass 12/12, herb 7/12) and
  "no new plants in the last 100k" are read after the lake forms. **Every
  §6.1 number is therefore a baseline of today's bed, flood included, not of
  the plants alone.** The M1 watering change re-runs them.


## 7. Corrections after the independent review (2026-10-20)

The review is in `plant-community-plan-2026-10-20.md` §12. What it changes
here:

- **Heredity was only partly off in the community beds** (§6). Fate and
  parameter mutation were zeroed. Per-birth jitter (σ 0.08) and founder
  variants (0.25) stayed on, because the harness never called
  `mutation_off_for_measuring`. The traces (`planttrace`) ran with mutation
  off, so they describe a slightly different world from the census they
  explain. **Every §6 number is "today's bed, heredity partly on"**; M1
  re-baselines with it off.
- **"Shrubs pass 86% of the light" is withdrawn.** The bed with the shrub
  crippled out reads the same (0.919 under "canopy" against 0.889 open), so
  the measure could not see shrubs. "Canopy" counted any organism cell,
  waiting seeds in 17–28-deep piles included, and rows were averaged across
  the flood. The tree and conifer figure (0.31) comes from beds where they are
  the tallest thing by far. It stands as indicative, not calibrated.
- **The contact counts (§6.5) include seeds and kin.** A seed is an
  organism, and the piles are all live seed. The "blocked upward" test also
  counts water as filled. The 62–86% figures are an upper bound on
  plant-to-plant crowding, not a measure of it.
- **"Water is never short anywhere" is a band mean across the whole width,**
  including the 100-column bare colony band, so it cannot see depletion under
  roots.
- **Sample sizes:** standing water, seed 2 only (saturation 0.86–0.98 on every
  seed supports the flood); mister off and the crippled shrub, one seed each;
  load failure off, 4 seeds. Acceptable as controls, not as estimates.
