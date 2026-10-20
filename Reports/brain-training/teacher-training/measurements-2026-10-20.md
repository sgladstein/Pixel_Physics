# Teacher training, measurements 1 and 2: paired worlds, and what a teacher can teach

*Measured 2026-10-20 by the teacher-approach session, locally (4 cores, no VMs), on s161 gen 336
(`Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt`) at the current training step: 8-row shaft,
12,000 frames, budget 2400. The plan these feed is [`guidelines-2026-10-20.md`](guidelines-2026-10-20.md); the tools and
raw data are in [`measurements/`](measurements/). Status: **done; both change the plan.***

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
  worlds, **s162 gen 338 scored 7.50 full loads against s161's 6.80** (+0.70, s.e. 0.38): at 4 rows s161 was ahead (7.21
  against 6.21 on 24 sealed worlds), so the dig-8 runs may have started from the weaker of the two at depth.

## Measurement 2: can the brain copy a teacher's call from what it senses?

**Setup.** The brain lane's decision logger extended (`measurements/option-logger.patch`, scratch, never for main) to
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

**Two control readings to keep in view.** D1b's positive control reads 0.96-0.97, not 1: a sharp threshold on one
standardised input among 124 is fitted slightly soft in 600 steps, so ~0.97 is this probe's ceiling for such labels. D3's
negative control reads 0.49 against a uniform floor of 0.40: walk options share a layout across worlds (which heading is
"towards the middle of the box"), and a model learns that prior from any world's labels. The teacher's 0.995 is far
above both.

## What changes in the plan

- **Guidelines, cost table:** paired worlds move from "the cheapest win" to *measured, no saving at the trainer's step
  size*; the reason (per-world noise is trajectory luck) moves into the case for copying.
- **Guidelines T3:** the first correction is "dig only where a foot cell is in reach" (a dig-or-not call), not "which
  cell to cut".
- **Next (step 1 of the plan):** the one-day partial teacher for D1b, D2 and D3, copied offline onto s161 with the
  round-trip control. Judge the copy against s161 by **unpaired means over many worlds** (96, with their standard
  errors), not by a paired test: measurement 1 shows a brain that behaves differently decorrelates from its source.

## Reproduce

```
# worktree from scratch/brain-instruments, patch applied
git apply Reports/brain-training/teacher-training/measurements/option-logger.patch
cargo build --release --example train --example reliab --example mutants   # mutants: measurements/mutants.rs.txt
# measurement 1
target/release/examples/mutants parent.txt 30 4242 kids                     # the trainer's rule
target/release/examples/reliab 8 12000 rho8.csv parent.txt parent-copy.txt s162-g338.txt kids/m*.txt   # ~9 min on 4 cores
python3 -I rho.py rho8.csv parent.txt parent-copy.txt s162-g338.txt
# measurement 2
PP_MIND_LOG=s161-8row.log PIXEL_PHYSICS_MUTATION=off target/release/examples/train brain=mind challenge=soil step=4 \
  shaft=8 foot=1 budget=2400 frames=12000 holdout=24 sealed=1 start=parent.txt score=parent.txt threads=1 noise=1
python3 -I -u probe.py s161-8row.log 4 D1b      # then D2, D3; ~15-20 min each, single-threaded numpy
```
