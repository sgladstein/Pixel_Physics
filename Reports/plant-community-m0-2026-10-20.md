# M0 of the plant community plan: baselines and feasibility (2026-10-20)

**Status: in progress.** The development, cost, copy and shrub readings are
complete. The community beds (M0.4–M0.6, 42 runs of 450,000 frames) are
running; their section is filled in when they finish. Plan of record:
[`plant-community-plan-2026-10-20.md`](plant-community-plan-2026-10-20.md)
§8.1. Logs: `Reports/plant-community-2026-10-20/m0/`.

## 1. What this changes in the plan, in short

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

*Running: mix7 and the four roles, 12 seeds without ants and 8 with, 450,000
frames, plus the crippled-shrub and mister-off controls. This section is
filled in when they finish.*
