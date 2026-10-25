# Bed selection: plants trained the way the ants are, 2026-10-24

Owner, 2026-10-24: *"Is there anything we can learn from the in colony
training the creatures have shifted to that applies to plants"*, then
*"Have you started testing the lessons"*. What the ant line learned that
carries over (`Reports/lanes/` and the colony-training and scoring
reviews of 2026-10-22):

1. **Train where it will live.** The ant moved once selection ran inside the
   colony rather than in arenas. The plant line found the same thing alone:
   roles re-fitted on rungs grew 1.6-5x and smothered their neighbours
   (`../refit/`), while roles bred in the mixed bed kept 3.0 roles a bed
   against 2.3 and gave the shipped shrub (`../hold/seeds/`).
2. **Select whole groups, and never on one summed score.** Colony selection
   was the ant line's strongest lever (3x food), but a summed colony score
   was won by not digging. Here a plant's own seed set would reward whatever
   wins its patch -- a tree that shades out the rest -- so beds are ranked
   on coexistence, by their **weakest** role.
3. **The training world must reach the late game.** The ants' 200k rounds
   never reached a big nest; the plant rungs (10-60k) never reach a closed
   canopy or a dying tree, which happen between 150k and 450k.
4. **Keep a pool, and keep mutation low.** The ants' product is a mixture
   of genomes (owner, 2026-10-22), and heavy mutation eroded an unpaid
   skill (digging).

## The mode (`plant_m0 mode=bedsel`)

Each round runs `beds` fresh four-role beds (`mix=roles`) for `round`
frames with in-bed heredity on: fate mutation at the game's 0.30, param
mutation `pmut=0.1`, the genotype jitter and founder variants. Each bed's
founders are given genomes from the pool (cycled per role; round 0 has
none, so the species files). At the round's end every bed offers its
`keep=8` leafiest plants per role (then its waiting seed); beds are ranked
by roles holding 100+ leaf, then by the weakest role's leaf; the top half's
offerings are the next round's pool. Fresh beds every round, so what
carries over is plants, not a lucky bed's soil. `pick=random` ranks by a
seeded shuffle: the control for "the selection did it" against "heredity
and time did it". After the last round, `save=` writes the pool (`ron`) and
a **common garden** judges it: `garden` fresh beds (seeds 9001..), heredity
off, shipped species files against founders from the pool, paired by
seed; `load=` with `rounds=0` judges a saved pool again. The pool reloads
to the same garden exactly, and the full run reproduced frame for frame.

## Results

`beds=8 round=150000 rounds=8 loadfail=0 rain=steady spells=20000,40000
ground=patchy mix=roles`, engine of main at `828d5ce` (roots required,
nutrient by root, old age on the plant clock; seed settling and sun scald
still off). Mean roles a bed per round (`select.txt`, `random.txt`):

| round | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| selected | 3.12 | 3.25 | 3.50 | 3.50 | 3.88 | 3.88 | 4.00 | 3.88 |
| random | 3.12 | 3.75 | 3.25 | 3.12 | 3.25 | 3.62 | 3.50 | 3.75 |

**The judge**, roles holding 100+ leaf, paired fresh beds, heredity off:

| founders | garden | roles a bed | all four | creeper alive | pool better / worse |
|---|---|---|---|---|---|
| shipped | 8 x 300,000 | 2.50 | 0 / 8 | 2 / 8 | |
| pool, random pick | 8 x 300,000 | 3.12 | 2 / 8 | 6 / 8 | |
| pool, selected | 8 x 300,000 | **3.75** | **6 / 8** | **8 / 8** | 8 / 0 |
| shipped | 16 x 450,000 | 2.56 | 1 / 16 | 2 / 16 | |
| pool, selected | 16 x 450,000 | **3.44** | **8 / 16** | **16 / 16** | 8 / 0 (8 tied) |

Leaf over the 16 beds at 450,000 (`judge450.txt`): tree 42,692 -> 28,322,
shrub 11,663 -> 14,721, herb 17,270 -> 8,942, creeper 270 -> 3,516.

- **Selecting whole beds works, and is not only heredity.** The random-pick
  pool beats shipped too (natural selection inside each bed), but the
  selected pool beats it on 5 of 8 paired beds, ties 2, loses 1.
- **The creeper lives.** It was absent from every bed of this plan's
  long runs; in the selected pool it is alive on every judge bed.
- **It holds at 450,000**, the length every earlier community run used.
- Pictures (300,000 frames, shipped above, pool below):
  https://claude.ai/artifact/TdFfeHk6hUCV69m61xBreX
- The pool: `pool-r8.ron`, 128 genomes, 32 per role.

## On the engine with seed settling and sun scald (2026-10-24)

The pool was selected before seed settling and sun scald went on (#876).
Re-judged on 8 fresh paired beds at 450,000 frames with both on
(`engine2/`, `plant_m0 mode=bedsel rounds=0 load=pool-r8.ron garden=8
gframes=450000`):

| load failure | shipped | pool | pool better / worse | creeper alive shipped / pool |
|---|---|---|---|---|
| off (`judge-loadfail0.txt`) | 2.62 | **3.12** | 3 / 1 | 2 / 8 -> 5 / 8 |
| on (`judge-loadfail1.txt`) | 2.88 | **3.12** | 3 / 1 | 2 / 8 -> 5 / 8 |

The gain is about half what it was on the engine it was bred on (2.56 ->
3.44), but it holds and the creeper still lives. A pool bred on this engine
should get the rest back.

## In the lab game, with the colony (2026-10-25)

The lab box now founds every pooled species (tree, shrub, herb, creeper)
from `assets/plant_pools/roles4.ron`, this pool (`LabBox::plant_pool`;
`PIXEL_PHYSICS_PLANT_POOL=0` is off, `=PATH` another pool). `played_bed`
plants 8 of its 18 founders from it (1 tree, 3 shrubs, 4 herbs; the
scrambler and grass are not pooled). `World::new` holds no pool, so the
outdoor world is untouched, and the plant trainer's arena founds from the
file it scores.

`latecensus`, paired seeds x 500,000 frames, one thread; means over the
10 samples, then medians over seeds. Two checks:

- **First** (`lab/`, 12 seeds): founders took their pool genome at
  planting. CI then showed that gave every lab rack copy the same plants,
  and #878 moved founding to germination under the world seed.
- **On the founding that shipped** (`lab2/`, 24 seeds, both arms on one
  build, main 53f8c55). The owner asked for 12 more seeds before choosing.
  The off arm of `lab/` could not be reused: main moved between the two
  builds, and its seed 1 differs at every sample.

| `lab2/`, 24 seeds | pool off | pool on | on higher / lower |
|---|---|---|---|
| plants standing | 98 | **119** | 17 / 7 |
| leaf energy (J) | 79,341 | 55,921 | 2 / 22 |
| edible food | 2,906 | **2,197** | 2 / 22 |
| ants (mean over the run) | 137 | 143 | 13 / 11 |
| ants at 500k | 69 | 41 | 11 / 13 |
| starved by 500k | 139 | **218** | 18 / 6 |
| colonies extinct by 500k | 3 / 24 | **6 / 24** | |

The first check, for the record: plants 96 -> 135, leaf 67,410 -> 60,465,
and ants 150 -> 115 (fewer on 8 of 12). Its colony drop did not hold up.

**More plants, but a quarter less food, so more ants starve.** The pool
trades the shipped tree's big crown for more, smaller plants. That is the
same trade as in the plant-only bed, where tree leaf went 42,692 -> 28,322.
Leaf and edible food fall by about a quarter on 22 of 24 seeds. The colony
is the same size on average, but more ants starve (18 of 24 seeds) and
twice as many colonies die out. **That is a measured harm to the main game,
so the lab founds from species files by default again** (2026-10-25,
`plant_pool::pool_switch`; `PIXEL_PHYSICS_PLANT_POOL=1` or `plant_m0 pool=1`
turns it on). The pool was bred with no colony in the bed, so the lesson is
the first one above: train where it will live. The next pool is bred in the
lab bed with the ants, and ranked on food as well as kinds.

Pictures, seed 1 at 50k / 100k / 150k (`planttrace shots=`, pool off above,
on below): https://claude.ai/artifact/Fdq8vHHXbRFL8sxVx2PnxM. The pool's
tree is lankier and more open, with more plants along the bed.

## Bred on the lab bed, with the colony (2026-10-25)

Lesson 1 again: train where the plant will live. `plant_m0 mode=bedsel
bed=played` runs `played_bed` as the game builds it, through
`lab::tick_world` (timeline, mister), with the ants landing at frame 6,000.
The pooled roles are tree, shrub and herb (the creeper is carried over
untouched). Beds are ranked by kinds holding 100+ leaf first, then by the
colony's mean edible food over the round (`lab::census`, every 25k). These
are two keys and never a sum, so food cannot buy back a lost kind. The run
started from `roles4.ron` and went 8 rounds × 8 beds × 150k frames
(`played/select.txt`, pool `played/pool-played-r8.ron`).

**The per-round food did not move**: mean edible 589, 516, 569, 566, 592,
577, 555, 570. Kinds a bed wandered between 2.1 and 2.9.

**The judge**: `latecensus mister=1` (the game's mister; the earlier
checks above ran without it), 12 paired seeds × 500k, three arms on one
build (`played/judge/`). Medians, with "lower" counted against species
files:

| | species files | old pool (roles4) | new pool (played) |
|---|---|---|---|
| plants standing | 113 | 135 | 122 |
| edible food | 2,597 | 2,260 (lower 8 / 12) | 2,272 (lower 8 / 12) |
| leaf energy (J) | 66,548 | 56,522 | 57,836 |
| ants (run mean) | 161 | 180 | 168 |
| starved by 500k | 91 | 195 | 148 |
| alive at 500k | 12 / 12 | 10 / 12 | 10 / 12 |

With the mister on, the pool's harm is milder: food is down an eighth, not
a quarter, and the colony is no smaller. But starvation still doubles and
two more colonies die out. The new pool is the old one within noise: it is
higher on edible food on 5 of 12 seeds and lower starved on its median
only. **Breeding with the colony did not give the food back** at this
round length. A round of 150k frames over 8 beds is dominated by
bed-to-bed spread, so the food key never saw a signal to select on. The
pool stays off; the record is in `Reports/dead-ends.md`.

**Not yet known:** what changed in the genomes (which params and fates the
pool moved), and whether food can be selected at all on this bed: it would
need longer rounds, many more beds a round, or a food score from what the
ants ate rather than what stood.
