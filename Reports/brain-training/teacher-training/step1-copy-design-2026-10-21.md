# Step 1 design: copying the three-rule teacher into s161

*Design doc, 2026-10-21, by the teacher-approach session. Status: **for adversarial review; nothing in it is built
except the teacher itself** (the ceiling test's scratch patch). Plan of record:
[`guidelines-2026-10-20.md`](guidelines-2026-10-20.md); evidence: [`measurements-2026-10-20.md`](measurements-2026-10-20.md).
Owner rulings it rests on (2026-10-20/21): copy for individual skills; test a hand-written teacher and self-imitation
side by side; selection stays the judge ("one general brain, found by selection"); no put-down rule; big redesigns get
a design doc and an adversarial review before building.*

## What step 1 does, and what it decides

**It teaches s161 gen 336 the three calls that took it from 6.8 to 20.5 full loads a world at 8 rows when the teacher
made them -- dig only where a foot cell is in reach, go down when empty in the shaft, walk home when empty on the
surface -- by gradient descent on the brain's own choice probabilities, and writes a normal genome file.** Everything
runs locally; no VMs.

It decides one thing: **whether copying can move a trained brain most of the way to its teacher's ceiling without
breaking what it already does.** The bar to go on (to step 2, a selection run started from the copy) is a copy that
captures half the ceiling's gain -- **13.7 full loads a world or more** on the 96 evaluation worlds, against s161's 6.80
and the teacher's 20.54 -- with fetch and trail at or above the kept-skill floor, and a GIF the owner passes.

A second arm runs the same copier on **self-imitation** labels (the brain's own decisions, kept by what followed them),
as the owner ruled. The label check below predicts it captures less: it can reinforce what the brain already does right,
and it has no example of the call the brain never makes.

## 1. The teacher

Exactly the ceiling test's teacher (`scratch_log::teach` / `walk_home`, `measurements/option-logger-and-teacher.patch`),
three rules, no put-down rule:

| Rule | Stage of the brain's choice | Teacher's call | Abstains |
|---|---|---|---|
| **dig** | verb, then target | empty jaws, a foot cell in reach: Dig, at the deepest foot cell. A Dig with no foot cell in reach: not Dig | shut in (no Walk offered) |
| **down** | target, given Walk | empty, below ground, no foot cell in reach: the deepest step, if deeper than here | no deeper step |
| **home** | target, given Walk | empty, on the surface, 3+ columns from the mouth: the step nearest the mouth's column | within 3 columns |

**Ceiling, measured:** dig + down + home driving s161: 20.54 +- 0.27 full loads (96 worlds), no world under 3; each rule
alone and in pairs in the measurements report. **Copyability, measured on s161's own runs** (held out by world, from the
brain's own senses, memory and option features; both controls passed):

| Rule | Probe's question | Floor | Brain now | Linear | 16 units |
|---|---|---|---|---|---|
| dig (veto + force) | is a foot cell in reach? | 0.971 | 0.966 | recall 0.98, precision 0.99 | 1.000 |
| down | which step is deepest? | 0.406 | 0.619 | 1.000 | 1.000 |
| home | which step is nearest the mouth? | 0.399 | 0.531 | 0.995 | 0.998 |

### 1a. The go-down probe, and a control that had to be replaced

"Go down" was added after the probe ran, so it was probed separately (D4: 5,192 decisions, 24 worlds): 1.000 held out,
linear and 16 units, against a floor of 0.41; the brain takes the deepest step 62% of the time today. **The original
negative control failed here** -- labels moved to another world's same-size decision still scored 0.98-0.99 -- because
in the shaft the step options come in the same order almost every time, so a label's *position* predicts it wherever
it lands. That control is replaced by a within-decision one (each decision's labels shuffled among its own options),
which reads 0.401 / 0.405 against the 0.406 floor. Walk-home's control (0.49 against 0.40) had the same cause and is
re-run the same way (measurements report).

**T1 audit (privileged computation, never privileged information).** Every fact the teacher reads is in the ant's own
inputs or computed from them: foot-in-reach is the target's depth and whether it was ground (each option's `DEPTH`,
`SOIL`, `CUT` features); "deepest step" is each walk option's `DEPTH`; "nearest the mouth" is carried by the options'
`HOME` and the `home_dist`/`d_home` senses. What the teacher has that the ant does not is *exactness* (the shaft
argument, the frame-0 ground), which is computation, not information. The probe numbers are the evidence.

### 1b. One teacher, two modes

The labels come from **the same Rust code that drove the ceiling test**, run in a new label-only mode
(`PP_TEACH_LABEL=dig,down,home`): it computes its call at every decision and writes it into the decision log beside the
brain's own pick, and does not act. A Python port of the rules would be a second implementation that the ceiling test
never validated, and the two would drift. The label-only mode is checked against the acting mode: on the same run, the
labels it writes must equal the calls the acting teacher made, decision for decision, up to the first decision where
they differ in what was done (positive control: one world logged both ways).

## 2. What is copied, at which stage

The brain chooses in two stages (`mind::decide`): a **verb** from the job scores, then a **target** of that verb from
the scorer. Each stage picks index *i* with probability

    p_i = (k + w_i)^2 / sum_j (k + w_j)^2,   w = (s + 1) / 2 in [0, 1],   k = PICK_FLOOR = 0.05

(`creature::choose_weighted`), smooth in every weight that reaches *s*, so the copy is ordinary gradient descent on
log-probabilities -- no reinforcement-learning estimator is needed. The teacher's calls map onto the stages:

| Teacher call | Loss on that decision |
|---|---|
| dig, force (foot in reach, brain did not Dig there) | -log P(verb = Dig) - log P(target in foot cells given Dig) |
| dig, veto (Dig chosen, no foot cell in reach) | -log (1 - P(verb = Dig)) -- push off Dig; which verb replaces it is the brain's (as the ceiling test's redraw) |
| down / home (given Walk) | -log sum over labelled steps of P(target) |
| teacher abstains | **anchor**: KL(p_s161 \|\| p_copy) at both stages -- stay what you were |

The anchor is the guard against drift everywhere the teacher is silent (guideline T3), and it is the cheap stand-in for
the kept-skill floor during copying (section 5 adds the kept arenas to it). Its weight is the one free trade-off of the
copier: the sweep in section 6 sets it.

## 3. What may change, and what may not

- **Trainable:** the target scorer (`S_CTX`, `S_FEAT`, `S_VERB`, `S_OUT`), the job heads (`JOB`, `HID_OUT`) and the hidden
  units (`HID`) -- the weights between the inputs and the two choices.
- **Frozen in v1:** memory (`MEM`, `RATE`), emits (`EMIT`), the global scalars (`SWITCH`, `SHARP`, `PULL`), and every slot
  the brain lane's `protect=` leaves alone (links serving only verbs the soil arena never offers -- the "don't eat the
  cargo" case).
- **Memory is an observed input, not trained:** each logged decision carries the memory the brain actually had, and the
  copier reads it as a sense. Training memory needs backpropagation through time, which RL would need too; v1 does
  without, and the review should say whether that is safe (section 8).
- **Dead links may revive.** A pruned link is a zero weight in a flat genome, so a dense gradient can bring it back --
  in particular part 2's missing `DEPTH` -> scorer link, which the dig rule probably needs (with senses alone the dig
  call's recall is 0.65; with the deepest option's depth it is 0.83). The copier reports every revived link. The
  selection run that follows must protect them, or pruning and the per-connection cost delete them in a generation.
- **Weights stay in the range mutation explores** (clipped to the start's own range per block), so selection can carry
  on from the copy.

The forward pass is `brainmath.py`'s, which already reproduces the brain's job scores from a genome file (median error
0.000025 over 105,854 scores), extended to the target scorer, the hidden units and both choice probabilities, with
gradients by hand in numpy (no autodiff in the environment). The extension is checked the same way: logged scores
reproduced, then a finite-difference check of every block's gradient.

## 4. Data: the student drives, the teacher labels (DAgger)

1. **Round 0:** s161 on 48 training-style 8-row worlds, logged with the teacher in label-only mode. Training and
   evaluation never share worlds: evaluation is the 96 `reliab` worlds (sets 9001/9002); training uses other seeds.
2. **Copy:** train on every labelled decision plus the anchor (section 2), write `copy-r0.txt`.
3. **Round k:** run `copy-r(k-1)` (the student drives) with label-only logging, add its decisions to the pool, retrain
   from s161 on the whole pool, write `copy-rk.txt`. Labels on the student's own states are the point: the ceiling test
   showed the states a changed ant reaches (mid-shaft with no wall to cut) are ones s161 never visited.
4. **Stop** when the share of ceiling captured (section 6) rises by less than one standard error between rounds, or at 5
   rounds.

A logged world takes about 3 seconds single-threaded; a round is minutes. The whole of step 1 is CPU-hours, not VM
money.

## 5. Kept skills

Fetch and trail must not pay for the dig lesson (investigation part 3 is the warning). Two guards:

- **During copying:** decisions logged from s161 on the fetch and trail arenas join the anchor (KL to s161's own
  distribution, nothing else), so the copier is pulled back towards s161 wherever the dig lesson would change them.
- **At evaluation:** fetch and trail scored as the brain lane scores them, against the fixed s91 floor (`keepref=`).
  Below the floor is a fail regardless of dig.

## 6. Controls, evaluation and bars

**Controls, run before any result is read:**

| Control | Expect | Catches |
|---|---|---|
| Zero steps: read the genome, write it back | identical scores on all 96 worlds | genome I/O, layout |
| Forward check | logged target and job scores reproduced to f32 rounding | a forward pass that is not the brain's |
| Gradient check | finite differences agree per block | a gradient that is not the forward's |
| Anchor only (no teacher labels) | within noise of 6.80, fetch and trail unchanged | an anchor that drifts, an optimiser that damages |
| Shuffled labels (teacher labels moved to other worlds' decisions) | no gain over s161 | "any training helps" |
| Label-only vs acting teacher | same calls decision for decision until first divergence | two teachers |

**Evaluation** (96 worlds, 8 rows, 12,000 frames, unpaired per measurement 1): full loads with standard error; **share of
ceiling captured** = (copy - 6.80) / (20.54 - 6.80); worlds under 3; fetch and trail against the floor; per-rule
agreement on the copy's own runs (diagnostic only, never a bar -- guideline T6); a GIF beside s161 and the teacher on
sealed world 2.

**Anchor sweep:** three anchor weights (light, medium, heavy) per arm; the bar is read on the best that keeps fetch and
trail above the floor. Three settings of one knob, not a search.

**Bars:**

| Outcome | Reading |
|---|---|
| share >= 0.5 (>= 13.7 loads), kept skills at floor, GIF passed | copying works; step 2 starts a selection run from the copy |
| share 0.2-0.5 | copying works partly; the review of the result asks which rule did not transfer (per-rule agreement says where) |
| share < 0.2, or kept skills below the floor | copying does not work on this brain as designed; stop and report what failed |

## 7. The self-imitation arm

Same copier, same anchor, same evaluation; only the labels differ. Labels are **per decision, in hindsight**
(measurement 1: a run's total is mostly its own luck, so whole-trip success is a weak label). Measured on s161's own 24
sealed-world run (`sil.py`, which reproduces every world's full loads exactly):

| Rule | Self-imitation label | Kept share | Teacher agrees: kept / dropped |
|---|---|---|---|
| dig | the dig's pellet became a full load | 0.30 | **1.00** / 0.06 |
| down | the next cut is a foot cut within 100-400 frames | 0.29-0.32 | 0.80-0.82 / 0.53-0.54 |
| home | within 3 columns of the mouth within 60-300 frames | 0.33-0.82 | 0.57-0.80 / 0.36-0.40 |

Kept decisions get the positive loss on the brain's own choice; dropped dig decisions get the veto loss (push off
Dig); dropped walk decisions get nothing (dropping a step is not evidence about which step was right). **What it cannot
do:** the teacher's 748 "dig here" calls the brain never made in that run have no example in self-imitation, and those
are the force half of the dig rule. The prediction, to be tested: self-imitation captures a clearly smaller share than
the teacher, mostly through the dig veto.

## 8. Risks and questions for the review

1. **Copyable is not copied.** The probe fitted separate models; the brain's own wiring is narrower (the job stage sees
   options only through the best target's score). Revived links are the likely route; if they do not form, the dig rule
   will under-transfer. Per-rule agreement on the copy's runs will show it.
2. **Memory as an observed input** freezes the memory weights at s161's; if s161's memory encodes "where in the trip am
   I" in a way the new behaviour invalidates, the copy's memory will mislead it. Is v1 without backpropagation through
   time safe, or does it hide the main failure?
3. **The anchor and the lesson pull against each other** on states adjacent to labelled ones. The sweep picks a weight;
   is three settings enough?
4. **Revived links and selection.** A copy whose gain sits in revived links is fragile under the next selection run's
   pruning and wire cost. Protecting them is a change to the brain lane's trainer, not this step's.
5. **The evaluation worlds were used to measure the ceiling.** They are not used for training or for choosing the anchor
   weight (the sweep reads a separate 48-world set); the final number is read once on the 96.
6. **Does this cross "one general brain, found by selection"?** The structure, senses and choice rule are untouched, and
   the copy is a start for selection, which can undo it. The owner has accepted copying for individual skills; the
   review should say whether anything here goes further.
7. **The one-day time box** (owner, 2026-10-20) covered building the teacher; the ceiling test and its two fixes used
   about half a day. The copier is a second piece of work and gets its own estimate below.

## 9. Cost and order of work

All local (4 cores). Label-only mode and logging: an hour. The copier (forward extension, gradients, checks): the main
piece, about a day. Rounds and evaluation: hours of CPU. Self-imitation arm: the same copier, labels from `sil.py`,
an hour more. **Order:** controls 1-3 -> round 0 for both arms -> anchor sweep -> DAgger rounds (teacher arm) ->
evaluation, GIF to the owner -> results report -> the owner decides on step 2.
