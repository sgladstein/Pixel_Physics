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

**Not yet known:** what changed in the genomes (which params and fates the
pool moved), whether it holds under the colony (lab game), and with seed
settling and sun scald on (on since 2026-10-24, §8 of
`../shade-seedling/README.md`). One run per arm.
