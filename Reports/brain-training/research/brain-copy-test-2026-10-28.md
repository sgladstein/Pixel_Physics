# Can the brain copy the hand-built ant? The copy test (2026-10-28)

**Status:** measurement and verdict, lane 3 of the plan of 2026-10-27 (`/mnt/project-files/planning/lane-briefs-2026-10-27.md`).
No engine change ships: the tools are scratch (`scratch/copytest/`) plus two `SCRATCH` hooks in the shadow and
relabelling paths, which change nothing unless `PP_SHADOW` or `PP_OLDLAB` is set. Data: [`brain-copy-test-2026-10-28/`](brain-copy-test-2026-10-28/).
Picture page: https://claude.ai/artifact/58VtSJ9xTa73F1QNoCksYX

The question: the trained pool (std_pool) collapses to 4-5 ants in `two_larders`, `scattered_larder` and
`feast_and_famine`, where the hand-built ant keeps dozens to hundreds. Is that the brain's design (it cannot
express what the hand-built ant does) or training (it can, and selection never finds it)? Fit the brain directly
to the hand-built ant's choices and put the copy in the same boxes.

## Answer

**Partly, and the brain's shape is not the main limit.** The best copy keeps a colony of **13.0** ants (median mean
headcount over the 60k frames after landing) against std_pool's **3.8** (higher on 24 of 24 paired seeds,
p = 1e-7) and the hand-built ant's **38.0** (lower on 24 of 24). Two separate things hold the copies back:

1. **The brain's running bill kills a landing colony.** Every live link costs energy every tick. On one tank of
   food, a 1,500-link brain loses about a fifth of its colony in the first 1,500 frames, and a 2,700-link copy
   lives 4.8 where the same copy with the bill off lives 15.4 (24/24). Training never sees this: its colonies land
   with three tanks (`grants=3`).
2. **A behaviour gap that the brain's shape does not explain.** The copies match the hand-built ant's choices about
   90% as well as an unconstrained network given the same senses (held-out P(label) 0.38 against 0.41), and that
   network itself gets only 41%. Adding what the hand-built ant did one decision earlier does not raise it
   (0.414 either way). Bill-free, the best copy (18.2) is still below the hand-built ant on 24 of 24 seeds.

So the bottleneck is the training environment (the bill at a thin landing) plus something the scorer's inputs do
not carry, not how the scorer combines them. That the missing piece is a *sense* is the likeliest reading of a
low ceiling, not something traced here. For std_pool itself the bill is the smaller part: bill-free it lives 5.8 (24/24
above 3.8, still 0/24 alive at the end) against the hand-built ant's 38.

## How it was done

- **Labels.** The hand-built ant played seeds 1-6 of each box while a mind watched (`PP_SHADOW=1`, the old ant
  acts, the mind logs its options and the old ant's pick, `PP_SHADOW_SAMPLE` keeping 1 in 20 plain walk/rest/turn
  rows). A fix was needed first: under shadow, `every_ant_has_one` said every ant had a mind, so the nest skipped
  building the mound and escape ways (`creature.rs`, `minds_only`) that the acting hand-built ant needs, and the
  shadowed colony lived 342 against 555 unshadowed. The fix returns false under shadow or relabelling. With the fix the two are identical. Seeds 1-4 train, 5-6 are held out.
- **Fit.** `scratch/copytest/brain.py` is the mind's `decide` rebuilt in torch (scorer, gate, job head, hidden
  units, the two weighted draws with `PICK_FLOOR`), checked against `copyreplay` by `check.py`. `copyfit.py` fits a
  whole genome by Adam on the log chance of the old ant's pick. Turn around is off in every std_pool member and its
  switch is a threshold a gradient cannot move, so `switches=1` turns it on first.
  - **c1** weights decisions as they happen, so walking and resting dominate.
  - **c2** weights every logged row equally (`plain_k=1`), so pick up, put down and dig count about 20 times more.
  - **c3** is c2 with a proximal L1 step (`prox=x`) that drives links to exact zero: x = 0.05, 0.2 and 0.6 give
    2,284, 804 and 390 billed links.
- **Ceiling.** `ceiling.py` is a two-layer, 128-wide network over the same inputs (context, memory, last verb, each
  option's features and verb), with a softmax over the options and the same labels.
- **Exam.** `boxes.py` puts each brain in as the lab's only founder pool, on fresh seeds 11-18 per box, for 60,000
  frames after landing, one tank, the real lab rules (`labsurvive`). The deciding measure, named before launch, is
  `lived` (mean headcount over those 60k frames), paired by seed. `cmp.py` gives the sign test. 14 arms
  were compared, so the Bonferroni bar is p < 0.0036.
- **Links** are counted as the game bills them (`MindGenome::live`): every non-zero weight from the scorer to the
  end of the core genome. The dig head rides after the core in a genome file, comes off before the count and is
  not billed.
- **Horizon.** This is a 60k test. The Teacher lane's exam runs 600k, where the hand-built ant itself mostly
  dies in `feast_and_famine` and `scattered_larder` (0/8 alive in feast_and_famine). The two are different
  questions and are not reconciled here.

## Results

### How many ants each brain keeps alive

All arms: 24 colonies (8 seeds x 3 boxes), 60k frames after landing. "Higher than std" and "lower than hand-built"
count seeds. Every p below passes p < 0.0036 unless marked *.

| Brain | Billed links | Lived (median) | Alive at end | vs std_pool | vs hand-built |
|---|---|---|---|---|---|
| Hand-built ant | ~50 | 38.0 (two_larders 228.8, scattered 34.9, feast 35.5) | 24/24 | 24/24 higher | |
| std_pool | 1,523 | 3.8 | 0/24 | | 24/24 lower |
| c1 | 2,693 | 4.3 | 3/24 | 12 higher, 12 lower (p 1 *) | 24/24 lower |
| c2 | 2,693 | 4.8 | 2/24 | 17 higher, 6 lower (p 0.035 *) | 24/24 lower |
| c3, lightly pruned | 2,284 | 5.7 | 5/24 | 21 higher, 1 lower (p 1e-5) | 24/24 lower |
| **c3, pruned** | **804** | **13.0** | **13/24** | **24/24 higher** | 24/24 lower |
| c3, pruned hard | 390 | 8.9 | 10/24 | 24/24 higher | 24/24 lower |
| d1 (relabelled once) | 2,759 | 3.4 | 2/24 | 9 higher, 14 lower (p 0.41 *) | 24/24 lower |
| d2 (relabelled twice, pruned) | 884 | 8.9 | 12/24 | 24/24 higher | 24/24 lower |
| std_pool, bill off | 1,523 | 5.8 | 0/24 | 24/24 higher | 24/24 lower |
| c1, bill off | 2,693 | 13.8 | 10/24 | 24/24 higher | 24/24 lower |
| c2, bill off | 2,693 | 15.4 | 11/24 | 24/24 higher | 24/24 lower |
| c3 pruned, bill off | 804 | 18.2 | 16/24 | 24/24 higher | 24/24 lower |
| Hand-built, bill off | ~50 | 40.2 | 24/24 | 24/24 higher | 11 higher, 13 lower (p 0.84 *) |

Pairwise among copies:

- Pruned c3 beats c2: 24/24 seeds.
- Pruned c3 with the bill off beats it with the bill on: 23/24 seeds, p = 3e-6.
- Pruned hard against pruned: 10 higher and 14 lower, p = 0.54 (no difference). But pruned hard brought home
  less food: trip deliveries 32.5 against 106, median.

The hand-built ant pays a brain bill too, on its own synapse count (`creature.rs`, `synapse_tax`), but a small one:
with it off, its drain at landing goes 0.038 to 0.033 and its colony does not move (40.2 against 38.0, 11 higher
and 13 lower). `tax=` scales `synapse_fraction`, which only the three bill sites read, so the switch moves the
bill and nothing else.

### Why copies die at landing: the bill

feast_and_famine, seeds 11-14, grant=0, the first 1,500 frames after landing, 104 ants. Each figure is the mean
over seeds of the median ant (`labsurvive trace=250`). Nobody eats much this early, so the drain is the running cost.

| Brain | Billed links | Bill | Drain/tick (frames 250-750) | Energy at 1,500 | Starved by 1,500 | Digs |
|---|---|---|---|---|---|---|
| Hand-built | ~50 | on | 0.038 | 133 | 0 | 17 |
| Hand-built | ~50 | off | 0.033 | 135 | 0 | 16 |
| std_pool | 1,523 | on | 0.098 | 69 | 19.3 | 37 |
| std_pool | 1,523 | off | 0.064 | 105 | 0 | 35 |
| clean1_k3c23 (PR #959) | ~1,550 | on | 0.101 | 68 | 20.8 | 29 |
| clean1_k3c23 | ~1,550 | off | 0.062 | 103 | 0.3 | 30 |
| c3, pruned hard | 390 | on | 0.068 | 109 | ~1 | |
| c3, pruned | 804 | on | 0.077 | 101 | 3.5 | 225 |
| c3, lightly pruned | 2,284 | on | 0.111 | 65 | ~27 | |
| c2 | 2,693 | on | 0.104 | 71 | 23.5 | 156 |
| c2 | 2,693 | off | 0.040 | 140 | 0.3 | 153 |

At ~1,500 links, the bill is about 0.035-0.04 per tick, as much again as the hand-built ant's whole running cost.
It starves a fifth of a one-tank colony before it finds food. Bill-free, std_pool and clean1 still drain twice as
fast as the hand-built ant. They dig twice as much, which is behaviour and not bill.

Later in a run there is a second, untraced cause. On feast_and_famine seed 11, c2 with the bill off bites as often
as the hand-built ant, yet its median energy keeps falling where the hand-built ant's recovers (it reaches 68 by
5,250 frames and the colony is down to 57 of 104).

### How closely the copies match the hand-built ant's choices

P(label), the brain's chance of making the hand-built ant's move, on held-out seeds 5-6. These are decision-weighted
except where noted.

| Fit | Billed links | Overall | Step | Rest | Turn | Pick up | Put down | Dig |
|---|---|---|---|---|---|---|---|---|
| std_pool member 0, Turn around switched on (the fits' start) | 1,523 | 0.138 | 0.22 | 0.05 | 0.12 | 0.20 | 0.15 | 0.01 |
| c1 | 2,693 | 0.383 | 0.27 | 0.52 | 0.26 | 0.26 | 0.10 | 0.04 |
| c2 | 2,693 | 0.373 | 0.26 | 0.51 | 0.25 | 0.44 | 0.36 | 0.13 |
| c3, pruned | 804 | 0.353 | 0.26 | 0.47 | 0.20 | 0.37 | 0.29 | 0.15 |
| c3, pruned hard | 390 | 0.337 | 0.25 | 0.45 | 0.22 | 0.39 | 0.25 | 0.09 |
| ceiling (same senses, any function) | | 0.413 | 0.35 | 0.50 | 0.27 | 0.33 | 0.14 | 0.07 |
| ceiling + the previous decision | | 0.414 | 0.33 | 0.52 | 0.32 | 0.31 | 0.11 | 0.06 |

**What the copies match.** Fitting the rare moves up (c2, c3) buys pick up and put down at the cost of a little
walking, and that, more than overall P, is what made the colonies live. The best-living copy is not the best-scoring
one: the lighter c3 and c2 score higher and live worse, because they pay for ~1,500-1,900 more links.

**Where the ceiling stands.** The ceiling is low because the hand-built ant's choices depend on things the mind is not
given. It looks along rays, which a mind does not cast (`mind.rs`, the brain bill's comment), and it runs on its own
state machine. Neither reaches the mind's 22 context values, 12 memories or an option's 45 features. This is the
senses gap. Which sense matters most was not traced here. This agrees with the come-home copy of 2026-10-26 (`dead-ends.md`,
`homefit.rs`): copies of a teacher stalled at 0.40-0.63 on the existing inputs, and the same lesson on a sense that
carried the interaction (`PP_HOMELOW`) fit to 0.79 at once.

**What the copies do in their own states.** Relabelled in a clone of the world on its own states, c1 would have
picked up food in only about 0.5% of decisions there: the copies rarely reach food at all.

### Relabelling the copy on its own moves (DAgger)

A copy only ever trained on the hand-built ant's states drifts into states the hand-built ant never visits. The
standard fix is to run the copy, have the teacher label the copy's own states, and refit. `PP_OLDLAB=1
PP_OLDLAB_ALL=1` runs the hand-built ant's decision on a clone of the world at a random sample of the mind's
decisions (one in `PP_OLDLAB_K`, here 2) and logs it as the label.

- **Round 1 (d1, from c1):** test P on its own states 0.30. In the boxes it lived 3.4 against c1's 4.3 (no better
  than std_pool, 9 higher and 14 lower).
- **Round 2 (d2, from pruned c3):** relabelled on pruned c3's own states (18 runs, seeds 1-6), refit from c3 on
  those labels plus the original ones (`prox=0.2`, 884 billed links). Agreement on its own held-out states rose
  only from 0.270 to 0.283. In the boxes it lived 8.9 against c3's 13.0 (9 higher, 15 lower, p 0.31). Per box the
  result split: better in scattered_larder on 8 of 8 seeds (9.6 against 6.4) and worse in feast_and_famine on 8 of 8
  (5.2 against 13.0). Neither per-box split (p 0.008 each) passes the corrected bar. Food brought home fell from 106 to 28 trips.

Relabelling therefore did not close the gap. That fits the reading above: agreement on the copy's own states stays
near 0.28 (lower than the 0.41 the ceiling reaches on the hand-built ant's states, a different set of states), and
more labels on the same inputs do not add information the inputs lack.

### Layout 8 (writable memory)

The brief asked to try layout 8 (`PP_WRITE=on`). It was not fitted. A write gate only helps a copy if the hand-built
ant's next choice depends on something the mind could have written down earlier. The ceiling with the previous
decision's label and per-option history added did not move (0.413 against 0.414). On that evidence a recurrent
(backprop-through-time) fitter for the write gate, which `copyfit.py` does not have, is unlikely to pay. If someone
builds one, the test above is the bar it must beat.

## What this means for training

- The founding grant (`grants=3`) hides a cost selection never pays. It does not explain std_pool's collapse:
  bill-free, std_pool still dies.
- A pool can be made to keep a colony alive by copying, and pruning is a large part of why. Copying alone does not
  reach the hand-built ant, and neither the brain's shape nor more decisions of history closes the gap. The likeliest
  lever is an input the hand-built ant uses and the mind lacks. Which one was not traced; the come-home case
  (`PP_HOMELOW`) is the precedent.
- The pruned copy is a usable founder, sitting between std_pool and the hand-built ant at 60k. It was not run to 600k.

## Tools left behind (scratch)

| File | Does |
|---|---|
| `scratch/copytest/brain.py` | the mind's `decide` in torch; genome and pool readers (layout 5-8); copy-log parser |
| `scratch/copytest/check.py` | checks `brain.forward` against `copyreplay` on the same rows |
| `scratch/copytest/copyfit.py` | fits a genome to labels: `slots=`, `switches=`, `plain_k=`, `prox=`, held-out scoring |
| `scratch/copytest/ceiling.py` | the unconstrained network on the same inputs (`hist=1` adds the previous decision) |
| `scratch/copytest/boxes.py` | runs arms in lab boxes, resumable CSV; `ARM=FOUNDERS|ENV|ARGS` |
| `scratch/copytest/cmp.py` | paired sign test per box and overall |
| `examples/labsurvive.rs` | now prints `trip_deliveries` on its `RESULT` line |

Switches (all off unless set): `PP_SHADOW=1` (the old ant acts, the mind logs), `PP_SHADOW_SAMPLE=n`,
`PP_COPY_LOG=path`, `PP_COPY_SAMPLE=a:b`, `PP_OLDLAB=1` / `PP_OLDLAB_ALL=1` / `PP_OLDLAB_K=k` (the old ant labels the
mind's states). `labsurvive` cannot read a pool file. Split it into one genome file per member (pool member k = member
0 plus its own lines) and pass the list.
