# Training ant brains in parallel and merging them later: is it realistic?

*Written 2026-10-23 by the "Parallel training with weight merging" thread, at Scotttt's request. Status: literature
from memory (not re-checked online), the project's own record, and three small local tests on 4 cores (12-24 seeds
each), not a 64-colony batch. Handed to the Teacher lane, which owns the one ant line. Data and scripts:
[`parallel-merge-2026-10-23/`](parallel-merge-2026-10-23/) beside this file; copies in the project files at
`parallel-merge/`.*

## The answer

**What decides whether merging weights works is how much each parallel run is allowed to change, not what it
trains.** (The owner's question was general; dig versus forage was only an example in the brief, used in Test 3.)

- **Runs that each change only a small, separate set of weights merge well.** Two lanes can each teach the ant
  something in parallel and the two can be merged into one brain, **if each lane changes only its own weights**. The
  merged brain then keeps both lessons (Test 1: the home sense and the crater refusal, merged, brought home 431 food
  against 309 for the better parent, 22 of 24 worlds, with the same low starvation).
- **Runs of ordinary selection change nearly every weight, and merge to a compromise at best.** Test 2 answers the
  general question: two colonies of the *same* run under the *same* score drifted into a digger and a forager, and every
  way of merging their weights gave an ant halfway between them, never one with both strengths.
- **Parallel training that does not merge weights works**: colonies train side by side and swap whole ants, each
  keeping its own brain. Colony selection already does this (section 5, point 2).

**The example case, a dig island and a forage island merged afterwards, is the worst case of the same problem.**
Three reasons, all measured below:

1. **A one-job island does not produce a specialist; it produces a broken ant.** A colony scored only on digging still
   has to eat, so 6 of 8 dig-only colonies died out in 8 rounds, and the survivors did not dig more than the joint
   colonies (nest 15 cells against 28). Digging and foraging are learned *against each other*: the hard part is
   *when* to do which, and neither island ever sees that choice.
2. **Merging selection-trained brains gives a compromise at best and a wreck at worst.** Two colonies of the same
   colsel20 arm that had drifted into a digger and a forager merged to a point between them (average: nest 372, food
   6,409, between the parents' 445 / 4,564 and 183 / 9,340), never to both. Merging the dig-only and forage-only
   islands was worse than either parent: the average starved 8.6 ants a colony against 1.8, crossover lost the nest,
   and adding both islands' changes to the founder ("task arithmetic") **collapsed all 12 of 12 colonies**.
3. **Unpaid skills erode** (already known: colony-training review 2026-10-22, dig-loss). Each island loses whatever
   it is not paid for, so the merge would have to restore it from the other parent, which tests 2 and 3 show it does
   not.

**What to do instead** (section 5): keep parallelism where it already works. Islands that each train the *whole* job
and swap whole ants (the colony selection that already runs is exactly this), extra islands on GitHub Actions if
compute is the reason for wanting parallel training, and parallel *lessons* that each touch only their own weights,
merged by slot union and then given a few rounds of joint selection.

## 1. Why the idea is plausible on paper

The ant brain is a **fixed scaffold** (2,797 slots; 3,279 with the evolvable dig head), so slot *i* means the same
thing in every ant. That removes the classic obstacle to merging neural networks: two networks trained apart usually
implement the same function with their units in a different order, and must be re-aligned before they can be averaged
(the "competing conventions" problem in neuroevolution; permutation alignment in the deep-learning merging work). Here
every ant also descends from one founder line (h1 -> c1 -> hA1 -> hcs), which is the other condition the merging
literature asks for.

What that literature shows (from memory, not re-checked):

- **Weight averaging** ("model soups", linear mode connectivity) works when the models were fine-tuned a *short*
  distance from one shared start and stay in the same basin. Far apart, the average sits on a ridge and performs worse
  than either.
- **Task arithmetic** (add each fine-tune's change to the base) and its repairs (TIES: trim small changes, resolve sign
  conflicts) work when the changes are small and touch mostly different weights.
- **Periodic averaging during training** (federated averaging, DiLoCo-style local steps) works because the copies are
  re-averaged often, so they never drift far apart. That is a training scheme, not "train separately, merge later".
- **Island models** with migration are the evolutionary version, and they swap whole individuals, not averaged ones.
- **Distillation** (Distral, policy distillation) merges by teaching one student from several teachers; in this repo,
  that is the copy/DAgger route.

So the literature's own conditions are: **small, mostly disjoint changes from a shared start**. The tests below check
how far the ant's training meets them.

## 2. What the project had already tried

- **Crossover inside a colony** (`cross=` in `examples/inrun.rs`, colony-training review, 2026-10-22): food up, but
  digging wrecked (45 cuts against 128; 5 of 11 colonies under 20).
- **Copies (distillation/DAgger) into shipped weights** often failed: the dig copy into hA1 starved the colony; the
  trail rule copied at 55-64% fit and died more; the spoil refusal copied as "never let go of spoil". **The copies that
  worked trained a narrow lesson**, and the far-from-home sense worked best of all by training **only the new sense's
  weights** (`only_allow`), "so nothing near home can change". That recipe is the reason Test 1 below works.
- The fixed scaffold was kept precisely so that crossover would stay meaningful (`Reports/dead-ends.md`, creature
  direction D4); `mind::crossover_group` takes whole units (about 60 groups), never single slots.

## 3. The three tests

All tests: a colony on a fresh bed world, 24 ants, 200,000 frames, **selection off** (a common garden), compute branch
`claude/project-thread-8y5ak0` at ecd0de23, body rules off. "Food home" is food put down within 8 columns of the door;
"nest" and "crater" are `scratch/inrun/nestcount.py` on the final map. Paired counts are worlds out of n where the arm
is higher / lower than the reference.

### Test 1: two lessons taught in parallel, merged by slot union (24 worlds)

hcs (the home-vector sense, 59 slots changed from hA1) and r4 (the crater refusal copy, 1,057 slots changed, mostly
by tiny amounts) were made from hA1 by two different lanes on the same day. **They touch no slot in common**, so the
merge is simply hA1 with both lanes' changes written in. All arms run with `PP_HOMEVEC=40:30`; hA1 and r4 get the 8
slots hcs zeroed for the new sense.

| arm | food home | starved of 24 | collapsed (<12 alive) | nest | crater | tank |
|---|---|---|---|---|---|---|
| hA1 (base) | 229 | 14.9 | 16 | 30 | 95 | 96 |
| r4 (crater lesson) | 250 | 17.6 | 18 | 30 | 90 | 88 |
| hcs (home lesson) | 309 | 1.4 | 0 | 20 | 120 | 172 |
| **merged (slot union)** | **431** (22/24 above hcs) | **1.8** | **0** | 29 | 116 | 136 |
| average of r4 and hcs | 379 | 2.7 | 0 | 21 | 118 | 155 |

The merged brain keeps hcs's survival and adds food on top. The averaged one is worse on both, as it must be: with
disjoint changes, averaging just halves each lesson. **Caveat:** r4's own crater effect is weak on this build (95 -> 90;
it was 39 -> 23 on the build it was made on), so this shows the merge keeps one lesson whole and does not damage the
other; it cannot show the crater lesson surviving a merge.

### Test 2: two islands of the same selection arm, merged four ways (16 worlds)

colsel20's light arm (hcs + evolvable dig head, 64 colonies, 47 rounds) ran as islands with refounding. By round 47 its
colonies had split into two clusters (between clusters about 290 slots differ by more than 1; within a colony about
30). Colony c3 had become a digger and c27 a forager. Each arm is a garden of 30 founders.

| arm | food home | starved | nest | crater |
|---|---|---|---|---|
| c3 pool (digger) | 4,564 | 0.4 | **445** | 10 |
| c27 pool (forager) | **9,340** | 4.1 | 183 | 4 |
| mix: 15 ants of each | 5,863 | 1.9 | 343 | 4 |
| crossover children | 6,043 | 3.0 | 331 | 5 |
| average of pairs | 6,409 | 2.5 | 372 | 7 |

Every merge lands between the parents; none has both the digger's nest and the forager's food. The average sits
slightly above the straight line between the parents (on that line, food 6,409 would come with a nest of about 344;
it has 372), so between close relatives averaging is safe and gives a mild bonus. It is still a compromise, not a
combination. (Food home here includes the larder churn noted on 2026-10-23; the order of arms does not depend on it.)

### Test 3: a dig-only island and a forage-only island, merged (12 worlds)

From hcs, three islands of 8 colonies, 8 rounds of within-colony selection each (`rounds.sh`): **J** the standard
score, **F** the same with digging unpaid (`w_dig=0 w_clear=0`), **D** digging only (all food pay 0). Colonies that
reached round 8: J 7 of 8, F 4 of 8, **D 2 of 8**. The survivors' pools were merged pairwise (F colony with D colony)
and judged against J's.

| arm | food home | starved | collapsed | nest | nest cuts |
|---|---|---|---|---|---|
| J (joint island) | 6,264 | 6.2 | 2 | 28 | 12 |
| F (forage only) | 5,540 | 1.8 | 0 | 24 | 5 |
| D (dig only) | 1,027 | 8.2 | 3 | 15 | 0.5 |
| mix: 15 F + 15 D ants | 3,630 | 2.1 | 0 | 22 | 8 |
| crossover F x D | 1,656 | 4.2 | 0 | 12 | 3 |
| average F, D | 2,759 | 8.6 | 2 | 17 | 4 |
| task sum (hcs + dF + dD) | 576 | **18.8** | **12 of 12** | 18 | 2 |

The dig-only island did not learn to dig (nest 15, almost no lasting cuts); it lost foraging and starved. Eight rounds
moved these brains far from hcs (distance 77-107, against 55 for colsel20's 47 rounds), and their changes overlap, so
adding them double-counts and wrecks the brain. Mixing whole ants is the least bad merge, and only because the F ants
carry the colony. Digging is low in every arm here because body rules were off and 8 rounds is short; the comparison
between arms is what counts.

## 4. Reading

- **The condition that matters is "small, disjoint, and protected".** Test 1's lessons met it because the lanes that
  made them restricted which weights could move. Selection with the current mutation (every live link stepped with
  probability 0.03, plus 10 structural events per child) moves *every* live weight a little and some a lot, so two
  selected lines are never disjoint; after 8 rounds they are far apart.
- **The ant's skills share one decision.** Dig versus forage is decided in the same job stage, and the dig thread found
  the gain from its teacher was *when* to dig, not *which cell*. A merge can only combine what each parent learned; the
  arbitration between jobs has to be learned where both jobs exist.
- **The pool rule already gives the diversity islands are for.** Test 2 shows colony selection's islands drifting into
  different kinds of colony on their own, under one score and one brain design, with no castes. Keeping a pool that
  holds several such lines is the owner's "pool, not one brain", and migration of whole ants is how island models
  share them. Weight merging adds nothing there.

## 5. Recommendation

1. **Do not build dig and forage training lanes that are merged later.** One-job islands die or forget (Test 3), and
   merging selected brains is a compromise at best (Test 2).
2. **If the goal is more training throughput, run more islands of the full job.** Colony selection is already an
   island model (64 colonies, the bottom half refounded from the top half's ants each round). Rounds exchange only the
   carried ants, so islands tolerate slow links: a group of 8-16 colonies per GitHub Actions runner (the repo is public,
   `train.yml` already runs jobs there, about 2 physical cores each, 6 h a job, 20 jobs shared with CI), swapping
   carries every round or every few rounds, would add compute without the 64-core VM. Islands can also run *different
   worlds* (food distance, moving food, plants) under the same full score, which keeps the diversity Test 2 shows
   without ever paying one job alone. This needs a small `colsel.py` change (read migrants from a results branch) and
   a workflow; it is the one piece here worth building, and only if compute is the bottleneck.
3. **Keep parallel lessons, and give them a merge rule.** Lanes that add a lesson or a sense should train only that
   lesson's weights (the far-from-home sense recipe, `only_allow`), save the slot mask with the genome, and merge onto
   the current founder by slot union (`build_merges.py lesson_union`, which refuses overlaps). Overlapping slots go to
   the lane that owns the founder to decide, and every merged founder gets a few rounds of joint selection before it is
   judged, so the job arbitration can re-tune.
4. **When two close relatives must become one founder, average, don't sum.** Between siblings of the same arm
   averaging was safe and slightly better than crossover or mixing (Test 2). Task arithmetic over selected lines
   collapsed every colony (Test 3); do not use it.

**Not settled here:** whether the crater lesson survives a slot-union merge (r4 was too weak on this build to tell);
how often islands must swap ants (a migration-interval sweep belongs with the Teacher lane's batches); and whether
islands on Actions, with their slower runners, are worth the setup against queueing on the VM.
