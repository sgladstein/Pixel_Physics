# Teacher training, measurements 1-3: paired worlds, what a teacher can teach, and what the teacher is worth

*Measured 2026-10-20 by the teacher-approach session, locally (4 cores, no VMs), on s161 gen 336
(`Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt`) at the current training step: 8-row shaft,
12,000 frames, budget 2400. The plan these feed is [`guidelines-2026-10-20.md`](guidelines-2026-10-20.md); the tools and
raw data are in [`measurements/`](measurements/). Status: **done; all three change the plan.** Measurement 3, the ceiling test, ran 2026-10-21.*

## The answer, first

1. **Comparing a child with its parent on the same worlds saves nothing at the trainer's mutation size.** A child made
   by the trainer's rule and its parent are almost uncorrelated world by world (median 0.05), so a paired difference is
   about twice as noisy as a raw score. The noise is not "some worlds are hard": two good brains, s161 and s162, correlate
   0.12. It is the run's own luck, reshuffled by any real change of behaviour. **The only cures inside evaluation are more
   worlds or longer runs** -- which makes a signal per *decision*, copying, worth more than the guidelines assumed.
2. **All three dig corrections a teacher would make are visible in the brain's own inputs.** A linear model, given only
   what the brain senses, remembers and sees of its options, reproduces the teacher's call on held-out worlds 94-100% of
   the time, where the brain today agrees 53-55% on put-down and walk-home. Both controls behave.
3. **The wall cuts are not a wrong pick among cells; they are digging in the wrong place.** When the brain digs, the
   cells in reach are almost never a mix of foot and wall (9 mixed decisions of 535). **65% of its digs are made where no
   foot cell is in reach**, and where one is, it digs only 20% of the time. The teacher's correction belongs to "dig here
   or not", not to "which cell".

4. **A teacher that only forbids strands the student.** Run directly on s161 (the teacher makes its calls, the brain
   does the rest), the first teacher's dig rule alone was worth +4.74 full loads, but paired with walk-home it collapsed
   9 of 96 worlds (7 of 48 seeds) to under 3: with its wall cuts vetoed the ant walked up and down the shaft for 6,000
   frames and never reached the foot. *(The first explanation given here -- "s161 had only ever got down by digging" --
   is withdrawn after review: alone it takes the deepest step 50-73% of the time, and dig alone does not strand; the
   combination with walk-home shifts a state the brain's descent depends on, and which input is not yet traced.)* One more rule, **go down when empty below ground with no
   foot cell in reach**, fixed it: **dig + go down + walk home reaches 20.5 full loads a world against the brain's 6.8
   (3.0x), with no world under 3.** The put-down rule hurt on its own (-0.97) and helps only beside walk-home (+0.4):
   the brain's long carry and its turn home are coupled.

## Measurement 1: are a parent and its children correlated on the same worlds?

**Setup.** `examples/reliab.rs` (the brain lane's harness) scores every genome on two independent 48-world sets drawn as
training draws them (`training_layouts`, mirrored pairs), per world. Children are made by `train::mutate` itself
(`measurements/mutants.rs.txt`). `rho.py` computes, against the parent on the same 96 worlds, the per-world correlation
and the **pairing factor** var(child - parent) / var(child): the share of worlds a race on paired differences needs
against a well-measured parent, compared with raw scores. Under 1 saves; over 1 costs.

**Controls.** Positive: the parent under a second file name -- correlation exactly 1.000000 and every world's
difference exactly 0, in every run (the simulation is deterministic and the pairing is right). Negative: the same pairs
with the worlds shuffled -- mean correlation -0.002 to -0.008.

| Children of s161 gen 336 | n | Weights changed | Median correlation, full loads | Median pairing factor | Full loads (parent 6.80) |
|---|---|---|---|---|---|
| One live weight x 1.001 | 10 | 1 | 1.000 (7 identical on all 96 worlds; others 0.89-0.99) | 0.00 | 6.74-6.91 |
| One live weight + N(0, 0.3) | 10 | 1 | 1.000 (6 identical; others 0.81-0.99) | 0.00 | 6.58-6.84 |
| pm 0.01, sigma 0.1 (round 3's smaller mutations) | 10 | 3-10 | 0.43 (0.08-0.90) | 1.4 | 5.67-6.50, all below the parent |
| **pm 0.03, sigma 0.3 (the dig-8 runs' rule)** | **30** | **22-45** | **0.05** (-0.20 to 0.38) | **3.6** | 0.43-6.53, median 3.5; none above the parent |
| s162 gen 338 (another good brain) | 1 | -- | 0.12 | 2.2 | 7.50 |

(Dig score reads the same: median 0.07 for the trainer's children, 0.14 for s162.)

**What it means.**

- **Paired worlds are not a lever at the trainer's step size.** Pairing pays only above a correlation of 0.5, i.e. for
  children a weight or two from the parent. The random-number design was never the problem (every draw is keyed by
  world, ant, frame and purpose, and the positive control confirms it): once behaviour really changes, every world's
  trajectory changes with it.
- **Per-world noise is the run's own luck, not the world's difficulty.** If some worlds were simply hard, two good
  brains would agree on which; s161 and s162 correlate 0.12. So a world carries very little information about a brain
  beyond its own outcome -- the case for per-decision labels (copying) gets stronger, not weaker.
- **Racing still stands**: it rests on most children being clearly worse, not on pairing. Here 25 of 30 trainer-rule
  children were more than two standard errors below the parent, and 12 of 30 fell under 3 full loads against 6.8.
- **For the brain lane, two observations, not recommendations.** (a) Every child of every mutation size was at or below
  the parent on average -- s161 gen 336 sits where almost every change hurts. (b) On these 96 training-style 8-row
  worlds, **s162 gen 338 scored 7.50 full loads against s161's 6.80** (+0.70, s.e. 0.46 clustered by seed; 0.38 unclustered): at 4 rows s161 was ahead (7.21
  against 6.21 on 24 sealed worlds), so the dig-8 runs may have started from the weaker of the two at depth.

## Measurement 2: can the brain copy a teacher's call from what it senses?

**Setup.** The brain lane's decision logger extended (`measurements/option-logger-and-teacher.patch`, scratch, never for main) to
write, per decision, **every Walk, Dig and Drop option** with its 45 features and the brain's score for it, and per
world the privileged facts a teacher reads: the frame-0 ground, the depth datum, the founding hole. The extended logger
does not change behaviour: the logged run gives 159 full loads over the 24 sealed worlds, 6.6 a world, the brain lane's
own figure. The cut classes reproduce theirs too (7.5 foot and 10.6 wall cuts a world at 12,000 frames, against 4.3 and
5.6 at 6,000).

`probe.py` builds each teacher's label from the privileged facts, then fits a model **on the brain's own inputs only**
(22 senses, 12 memories, and the offered options' features) and scores it on worlds it never saw (4 folds by world):

| Teacher's call | n | Floor | Brain now | Linear | 16 units | Positive control | Negative control (floor) |
|---|---|---|---|---|---|---|---|
| **D2** put down at the first cell 3 clear of the hole | 11,011 | 0.511 | **0.552** | 0.938 | **0.965** | 0.991 / 0.997 | 0.518 / 0.491 (0.543) |
| **D3** empty on the surface: step towards the mouth | 21,107 | 0.399 | **0.531** | 0.995 | **0.998** | 1.000 / 1.000 | 0.492 / 0.493 (0.399) |
| **D1b** dig only with a foot cell in reach | 32,613 | 0.971 | 0.966 | 0.999 | 1.000 | 0.959 / 0.972 | 0.972 / 0.972 (0.972) |
| D1 which cell, given a dig | 9 | -- | -- | -- | -- | -- | too few to fit |
| **D4** go down: empty below ground, no foot cell in reach, take the deepest step (added 2026-10-21) | 5,192 | 0.406 | 0.619 | 1.000 | 1.000 | 1.000 / 1.000 | 0.982 / 0.993 (0.406) -- **invalid, see below**; within-decision 0.401 / 0.405 (0.406) |

Floor: a uniform pick (D3) or the commoner answer (D1b, D2). Positive control: a label that is a pure function of the
inputs (D3: the option with the largest `UP`; D1b and D2: `home_dist` > 0.2). Negative control: the teacher's labels moved
onto another world's decisions.

**D1b, read properly** (97% of its labels are "no", so accuracy hides the point): of the brain's 535 digs, **186 had a
foot cell in reach (35%)**; with one in reach it digs 20% of the time, without one 1.1%. The linear copy, held out,
**catches 914 of 934 foot-in-reach cases (recall 0.98) with 6 false alarms (precision 0.99)**.

**Does the brain's wiring carry it?** The probe saw the options through mean and max summaries; the brain's job stage
sees them only through one number, the best option's score. With the narrower channel, linear, held out:

| | Senses + memory only | + one number: the deepest offered cell's depth |
|---|---|---|
| D1b recall / precision | 0.65 / 0.87 | **0.83 / 0.97** |
| D2 accuracy (recall / precision) | **0.87** (0.90 / 0.84) | 0.87 |

So "dig here?" needs the target scorer to pass depth on through its best score (part 2: today **no live link carries
`DEPTH` into the scorer** -- copying would have to revive one, guideline T4), and "put down now?" is mostly in the senses
already, chiefly `home_dist`.

**The cross-world negative control is invalid for step choices** (found on D4, 2026-10-21). It moves a decision's labels
to a same-size decision from another world, which keeps each label's position in the option list; in the shaft the
steps come in the same order almost every time, so position alone predicts the moved labels (0.98-0.99). The
replacement shuffles each decision's labels among its own options; it reads at the floor for D4. D3 is re-run with it.

**Two control readings to keep in view.** D1b's positive control reads 0.96-0.97, not 1: a sharp threshold on one
standardised input among 124 is fitted slightly soft in 600 steps, so ~0.97 is this probe's ceiling for such labels. D3's
negative control reads 0.49 against a uniform floor of 0.40: walk options share a layout across worlds (which heading is
"towards the middle of the box"), and a model learns that prior from any world's labels. The teacher's 0.995 is far
above both.

## Measurement 3: the ceiling test -- what is a teacher worth if the brain followed it exactly?

**Setup.** The hand-written teacher, built into the scratch branch (`measurements/option-logger-and-teacher.patch`,
`scratch_log::teach` and `walk_home` in `mind.rs`), makes its calls *in the running simulation* while s161 makes every
other decision. `PP_TEACH=dig,down,home,drop[,margin=N]` turns each rule on. It reads only what measurement 2 showed the
ant can sense -- the frame-0 ground, depth, the hole as it stands -- handed over by the soil arena for the run's frames and
**taken away before the scored settle**. Rules (a veto removes a verb and lets the brain redraw from what is left, so
the teacher never invents behaviour where it abstains):

| Rule | Force | Veto | Abstains |
|---|---|---|---|
| **dig** | empty jaws, a foot cell in reach: dig the deepest | a dig with no foot cell in reach | shut in (no Walk offered) |
| **down** | empty, below ground, no foot cell in reach: the deepest step, if it is deeper | -- | no deeper step |
| **home** | empty, on the surface, 3+ columns from the mouth, walking: the step nearest the mouth's column | -- | within 3 columns of the mouth |
| **drop** | carrying, a put-down 3+ cells clear of the hole offered: the clear one nearest the hole | a put-down inside the zone | shut in |

**Checks that it ran.** Every arm counts its forces and vetoes (all six rules fired in every arm that had them); the
teacher-off arm reproduces 6.80 with every counter at 0; decisions with the teacher on but no facts number 381 a world,
the settle's share (2,000 frames), so none escaped it; a rerun of one arm reproduces to the last digit. *One false start,
recorded because CLAUDE.md predicts it:* the first traced run of the drop rule was identical to the brain alone line for
line -- the logging binary had not been rebuilt after the teacher was added.

**Results** (s161 gen 336, 96 training-style worlds, 8-row shaft, 12,000 frames; unpaired, per measurement 1):

| Rules on | Full loads a world | vs brain alone | Worlds under 3 | p10 / median / p90 | Worlds at 10+ |
|---|---|---|---|---|---|
| none (brain alone) | 6.80 +- 0.32 | -- | 6 | 3 / 6 / 11 | 20 / 96 |
| drop | 5.83 +- 0.27 | -0.97 +- 0.41 | 5 | 3 / 6 / 9 | -- |
| drop, margin 1 | 5.84 +- 0.26 | -0.96 +- 0.41 | -- | -- | -- |
| home | 8.36 +- 0.30 | +1.56 +- 0.44 | 5 | 5 / 9 / 12 | -- |
| down | 10.21 +- 0.29 | +3.41 +- 0.43 | -- | -- | -- |
| dig | 11.54 +- 0.34 | +4.74 +- 0.46 | 2 | 7 / 12 / 15 | 72 / 96 |
| dig, home | 10.59 +- 0.66 | +3.79 +- 0.73 | 9 | 3 / 11 / 20 | -- |
| dig, drop, home | 12.74 +- 0.67 | +5.94 +- 0.74 | 7 | 4 / 13 / 21 | -- |
| dig, drop, home, margin 1 | 10.10 +- 0.61 | +3.30 +- 0.69 | -- | -- | -- |
| dig, down | 16.92 +- 0.23 | +10.11 +- 0.39 | 0 | 15 / 17 / 19 | 94 / 96 |
| **dig, down, home** | **20.54 +- 0.27** | **+13.74 +- 0.42** | **0** | **19 / 21 / 22** | **94 / 96** |
| dig, down, home, drop | 20.96 +- 0.25 | +14.16 +- 0.40 | 0 | 20 / 21 / 23 | 94 / 96 |

**Why each failure, traced.**

- **drop alone hurts** (24 sealed worlds, every pellet traced): the carry up got shorter (surface carrying 18.5% ->
  7.3% of decisions; foot cut to put-down median 270 -> 220 frames) but the way back down got longer (put-down to next
  foot cut median 655 -> 992 frames; empty on the surface 46% -> 58%), and foot cuts fell 7.5 -> 6.6 a world. The
  brain's turn home fires late, after it has gone far: put the load down at 4 columns and the empty ant walks on
  anyway. A wider clearance (margin 1) changes nothing, so it is not pellets sliding back. With walk-home taught too,
  drop becomes a small gain.
- **dig + home collapses some worlds** (sealed world 2 traced decision by decision): zero wall cuts in every world --
  the dig rule did its job -- but in 6 of 24 worlds the ant made 1-2 foot cuts and then spent 6,000+ frames walking up
  and down the 2-wide shaft between 1 and 7 rows down, a step down offered almost every time, never reaching the foot
  (12 decisions at depth 8, against 600+ in good worlds). No fallen pellet blocked it: every dig target it was offered
  was wall or rim. **down** removes the collapse. *Why the ant stopped descending is open, and the first answer given
  here was wrong* ("the brain had never learned to descend; it got down by cutting walls" -- withdrawn 2026-10-21): the
  evidence review recomputed that s161 alone takes the deepest step 0.50-0.73 of the time in this state, that a dig-only
  arm vetoing the same wall cuts does not strand, and that under dig + home the bias falls to ~0.40 against ~0.37
  stepping up in every world. Making the teacher's memory update consistent did not change it (7 of 48 seeds still
  collapse). The walk-home calls shift some state the brain's descent depends on; tracing which input is a follow-up.

**After review (2026-10-21), standard errors are clustered by seed**: the 96 worlds are 48 mirrored pairs and twins
often run identically, so the numbers above understate the error by 20-40% and count tails twice. The re-run with the
teacher's memory made consistent, clustered by seed, the compliance curve and the floor-limited arms are in
[`step1-review-2026-10-21.md`](step1-review-2026-10-21.md): the ceiling is **20.66 +- 0.37** (was 20.54 +- 0.27).

**What the owner should look at** (sealed world 2, 12,000 frames, a frame every 60): `measurements/brain-vs-teacher-world2.gif`
-- s161 alone (wall cuts scattered round the shaft) against s161 with dig, down and home (a clean shaft cut at the
foot); and `measurements/teacher-putdown-world2.gif` -- the teacher with and without the drop rule. **The drop rule
stacks pellets into two posts beside the mouth** (the "first clear spot" is the same spot each time), where the brain's
own put-downs spread into a low heap. It adds +0.4 loads, inside the noise; a posts-shaped mound is not what real ants
build, so it should not be taught as it stands.

## What changes in the plan

- **Guidelines, cost table:** paired worlds move from "the cheapest win" to *measured, no saving at the trainer's step
  size*; the reason (per-world noise is trajectory luck) moves into the case for copying.
- **Guidelines T3:** the first correction is "dig only where a foot cell is in reach" (a dig-or-not call), not "which
  cell to cut".
- **The teacher to copy is dig + down + home** (drop out until it builds a heap rather than posts), and **go down**
  needs a copyability check before it is copied -- it was not one of the probe's decisions (prediction: copyable, since
  every walk option carries `DEPTH`).
- **Next (step 1 of the plan):** the one-day partial teacher for D1b, D2 and D3, copied offline onto s161 with the
  round-trip control. Judge the copy against s161 by **unpaired means over many worlds** (96, with their standard
  errors), not by a paired test: measurement 1 shows a brain that behaves differently decorrelates from its source.

## Reproduce

```
# worktree from scratch/brain-instruments, patch applied
git apply Reports/brain-training/teacher-training/measurements/option-logger-and-teacher.patch
cargo build --release --example train --example reliab --example mutants   # mutants: measurements/mutants.rs.txt
# measurement 1
target/release/examples/mutants parent.txt 30 4242 kids                     # the trainer's rule
target/release/examples/reliab 8 12000 rho8.csv parent.txt parent-copy.txt s162-g338.txt kids/m*.txt   # ~9 min on 4 cores
python3 -I rho.py rho8.csv parent.txt parent-copy.txt s162-g338.txt
# measurement 2
PP_MIND_LOG=s161-8row.log PIXEL_PHYSICS_MUTATION=off target/release/examples/train brain=mind challenge=soil step=4 \
  shaft=8 foot=1 budget=2400 frames=12000 holdout=24 sealed=1 start=parent.txt score=parent.txt threads=1 noise=1
python3 -I -u probe.py s161-8row.log 4 D1b      # then D2, D3; ~15-20 min each, single-threaded numpy
# measurement 3 (rebuild BOTH examples after applying the patch)
PP_TEACH=dig,down,home target/release/examples/reliab 8 12000 dig_down_home.csv parent.txt   # ~2 min; prints the teacher's counts
python3 -I arms.py none.csv dig_down_home.csv
PP_TEACH=dig,down,home PP_GIF_EVERY=60 PP_GIF_DIR=frames PP_GIF_SEALED=2 target/release/examples/train ... pic=x.png score=parent.txt
python3 -I mkgif12.py <dir> out.gif 96 50 64 30 5 <sub1> "<label1>" <sub2> "<label2>"
```
