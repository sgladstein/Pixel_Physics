# Birth brakes: proposal (2026-10-08)

**Status: proposal, for review before any build.** The owner chose two brakes
(2026-10-08): *a full ant stops eating* and *feed before you lay*, each its
own switch, off by default. The trace they rest on is
[`boom-and-bust-2026-10-08/README.md`](boom-and-bust-2026-10-08/README.md)
(on branch `claude/eloquent-johnson-axjvu1-boom-bust`, PR 660, until it
lands): a colony beside a large finite store breeds to 400-500 ants, eats it
in ~40k frames, and every adult dies within 3k-20k frames of the last bite,
the median ant holding about 1,500 frames of living and 174-201 brood left
starving. Measured unless marked *inferred*.

## What a fix has to do

**The main test is renewable food** (owner, 2026-10-08: *"a colony will of
course die with a non renewable source of food. That's not an issue"*). On
`steady_income` (10 cells of provisions every 1,000 frames) the colony
should settle at a size the income carries and build up a reserve, rather
than breeding every surplus away and swinging between boom and famine. That
is the owner's playtest complaint: *"we cannot build up food because our
population grows quickly as we get more food"*. `boom_bust` is a secondary
check on how fast births react when food runs short (the brood left
starving at the end is the symptom). Colony size is a floor, not the verdict
(owner's rule).

## Brake 1: a full ant stops banking food (`digest_hunger_weight`)

**This already exists**, built and shipped off on 2026-09-18:
`CreatureDef::digest_hunger_weight` (`creature.rs`'s digest block). At
weight `w`, digestion runs at full rate up to `start_energy` (200 J) and is
scaled by `1 - w + w * hunger` above it, `hunger` falling from 1 at
`start_energy` to 0 at `reproduce_threshold` (1,100 J). What is not
digested stays in the crop as cargo, to be put down or shared. It reads
the ant's own bank only (a local signal), and it never slows a hungry ant:
below `start_energy` it is full rate.

**Why "stop eating" became "stop digesting".** For a forager, eating *is*
filling the crop, and the crop is how food gets home. Gating the bite would
stop delivery too. The crop as a social stomach, with passage to the
ant's own gut regulated by need, is the biology (sources to check before
citing in code: the proventriculus as the crop's valve, e.g. Eisner and
Brown 1958; crop-load regulation of foraging, Mailleux et al. 2000s). The
rich ants in the trace hold 1,100-4,700 J because they digest every load
they carry at the same rate, however full.

**Its dead-ends entry** (`Reports/dead-ends.md`, `digest_hunger_weight`)
says re-test *when a colony gets rich enough that a meaningful share of its
ants sit above `start_energy`*. On `boom_bust` they are: the richest tenth
hold 1,100-4,700 J at 33-37k. So this is a re-test on its own condition, and
the only build needed was a parameter row so `deeptrace set=` and the lab's
parameter page can reach it (`digest_hunger_weight` was missing from the
lab's "every scalar an ant is made of" list).

**First measurement** (this branch, `boom_bust`, the owner's playtest
switches, seeds 1-4, 120k frames): see the table below once filled.

**Known risk, from the curve itself:** at `w = 1` digestion stops at
`reproduce_threshold`, and the body laying bar (`LAY_BAR=body`) is that same
1,100 J (scaled by each ant's heritable `reproduce_at`), so a bank fed only
by digestion approaches the bar and never reaches it. Seed 1 at 20k: 51
ants and no brood, against 187 ants and 125 brood without it. That is a
birth *stop*, not a brake. A partial weight (0.5: half rate at the bar)
leaves the bar reachable.

## Brake 2: feed before you lay (`FEED_FIRST`, to build)

**The rule.** An ant that could lay (its bank has cleared its bar and its
brain's `Lay` output said yes, in `try_bud`) first asks whether a hungry
larva of its colony is within reach. If one is, it does not lay this tick.
Counted as `feed_first_held`. It lays only when it smells none.

- **The cue is local:** `brood::nearest_hungry_larva` within a reach `R`
  around the head, the same "hungry" `larva_scent` reads (short of its
  pupation target).
- **Parts** (one switch, named parts, per the owner's rule):
  - `hold`: the veto above, nothing else. The adult already feeds a larva it
    touches (`brood::nurse` makes the richest kin adult on the eight
    neighbouring cells give 0.25 of its surplus over 200 J), so a held layer
    that is beside the larva feeds it with no new code.
  - `seek` (only if `hold` alone does nothing): a held layer is turned
    toward the scent, through the existing `NurseSeek` path.
- **Hunger safety:** it vetoes laying only, never eating or walking to food.
- **Biology** (cited in the 2026-10-07 plan, §6 step 2; check before citing
  in code): Orlova et al. 2020; Cassill & Tschinkel 1995. Workers and queens
  in food-short colonies feed brood before producing more.

**The known catch, and the first thing the review must check.** The egg-cap
census (`Reports/handoff/2026-10-07/egg-cap/handoff.md`) found layers stand
10+ rows from the larvae: eggs land in the top of the door shaft, larvae lie
at its bottom. With `R` at `NURSE_SCENT_REACH` (6) a layer may never smell a
larva, and `hold` would be inert. **Before choosing `R`, measure the
layer-to-nearest-hungry-larva distance at each lay** on `boom_bust`. The
egg-cap census patch records the layers; joining it to brood positions gives
the distance distribution, and `R` is set from it with headroom (CLAUDE.md:
bars from measurement, never on the measured value).

**What it is for, in this trace.** 174-201 brood are left to starve at
food-out because eggs kept coming (7-37 in the last 5k frames) while larvae
were already short. `hold` stops exactly those eggs, if layers can smell the
hungry larvae.

## Test plan

1. `digest_hunger_weight` at 0 and 0.5 on `steady_income`, seeds 1-4, 300k
   (running since ~03:00 UTC), with 0 / 0.5 / 1 on `boom_bust` as the
   reaction check. Read: colony size over time and its swing, food left on
   the ground (the reserve), J held in crops, eggs per J.
2. The layer-to-larva distance census, then `FEED_FIRST=hold` built off,
   tested against off on the same seeds.
3. Both together.
4. Whichever moves the end's shape: 12 seeds, plus heap 90 on the goal box
   as the floor (it must not shrink colonies that live on endless food,
   beyond what the trace explains).

Each arm scored on its own metric (`digest_hunger_weight`: J held in crops,
how long food lasts, eggs per J; `FEED_FIRST`: eggs held, larvae starved per
egg), then the end's shape (frames from food-out to the last adult; brood
left behind).

## Measured: `digest_hunger_weight` on `boom_bust`

`boom_bust`, the owner's playtest switches, seeds 1-4, 120k frames, binary
`main` 6ffda5db plus the parameter row. **Identity:** the weight-0 arm's
`stats.csv` is byte-identical to the 2026-10-08 `play` runs (`f55b33b8`)
for every one of its 115 rows, on all four seeds.

| weight | peak ants | eating stops | eggs | raised | larvae starved by 120k |
|---|---|---|---|---|---|
| 0 | 407-508 (39-46k) | 42-50k | 649-724 | 475-546 | 112-117 |
| **0.5** | 392-442 (46-51k) | **54-67k** | 603-617 | 468-493 | **65-85** |
| 1 | 52-70 | 83k-past 120k | 24-39 | 24-39 | 0 |

- **0.5 brakes births and stretches the food:** 10% fewer eggs, the store
  lasts 10-17k frames longer (25-35%), larvae starved down about 40%, and
  the number raised barely moves. Fewer eggs are wasted on larvae that will
  starve.
- **1 stops breeding:** the colony holds at 50-70 ants and the store lasts
  two to three times as long. That is the asymptote at the laying bar
  predicted above.
- The death after the last bite is still abrupt at 0.5 (2-7k frames): it
  slows the boom but does not bank a reserve in bodies. Whether the crop
  cargo it holds back reaches the ground or the store as a reserve is the
  `steady_income` question.

**`steady_income` (the main test)**, weights 0 and 0.5, seeds 1-4, 300k
frames, the same switches and binary. The income (10 cells, 9,600 J, every
1,000 frames) carries about 60 ants.

| weight | seed | ants 100-300k (min / mean / max) | ants at 300k | adults starved | larvae starved | eggs |
|---|---|---|---|---|---|---|
| 0 | 1 | 2 / 49 / 94 | 2 | 299 | 26 | 578 |
| 0 | 2 | 42 / 66 / 87 | 52 | 208 | 0 | 610 |
| 0 | 3 | 29 / 58 / 97 | 81 | 416 | 97 | 969 |
| 0 | 4 | 6 / 56 / 112 | 6 | 302 | 31 | 609 |
| 0.5 | 1 | 38 / 67 / 116 | 110 | 104 | 1 | 576 |
| 0.5 | 2 | 33 / 60 / 97 | 36 | 191 | 1 | 548 |
| 0.5 | 3 | 20 / 64 / 140 | 42 | 244 | 21 | 633 |
| 0.5 | 4 | 27 / 62 / 114 | 114 | 214 | 0 | 656 |

- **On a steady income the shipped colony still swings and can nearly die**:
  two of four seeds fell to 2 and 6 ants by 300k (seed 1 from 89 at 190k,
  seed 4 from 109 at 170k) with food arriving every 1,000 frames the whole
  time; the food then piled up unwanted (442-507 cells at the heap).
- **At 0.5 no seed fell below 20**, adults starved are lower on 4 of 4
  (104-244 against 208-416), and larvae starved are near zero.
- **It still swings** (seed 3: 102 -> 23 -> 139), so births still follow
  food; it damps the boom rather than ending it. That is the case for
  `FEED_FIRST` alongside it.
- Four seeds and a small colony: indicative, not the bar. The bar is 12
  seeds, and an income that carries a colony of a few hundred is the next
  `steady_income` to run.
