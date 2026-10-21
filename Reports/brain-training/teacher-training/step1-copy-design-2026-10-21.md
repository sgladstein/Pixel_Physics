# Step 1 design (v2): copying the three-rule teacher into s161

*Design doc, 2026-10-21, by the teacher-approach session. **v2, after the adversarial review**: three reviewers
(method, evidence, code) attacked v1; all 16 BLOCKER/MAJOR findings were accepted and the checks they asked for were
run -- [`step1-review-2026-10-21.md`](step1-review-2026-10-21.md) has every finding, its disposition and the check
results, and [`review/`](review/) the full reviews. Status: **revised, not built** beyond the teacher itself (the
scratch patch, `measurements/option-logger-and-teacher.patch`). Plan of record: [`guidelines-2026-10-20.md`](guidelines-2026-10-20.md);
evidence: [`measurements-2026-10-20.md`](measurements-2026-10-20.md). Owner rulings it rests on: copy for individual
skills; test a hand-written teacher and self-imitation side by side; selection stays the judge; no put-down rule; a
design doc and an adversarial review before building.*

## What step 1 does, and what it decides

**It teaches s161 gen 336 the three calls that take it from 6.80 to 20.66 full loads a world at 8 rows when the
teacher makes them -- dig only where a foot cell is in reach, go down when empty in the shaft, walk home when empty
on the surface -- by gradient descent on the brain's own choice probabilities, and writes a normal genome file.** All
local; no VMs.

It decides **whether copying can move a trained brain most of the way to what its teacher is worth, without breaking
what it already does.** The bar is set from what a copied brain *could* reach, not from the teacher:

- **The reachable ceiling.** A brain cannot obey perfectly: its choice rule keeps every option at a floor of
  probability, and `squash` saturates slowly. Drawing every covered decision from the saturated policy, scores at +-1
  give 20.34 +- 0.40 full loads; at +-0.9, **18.23 +- 0.37** (the copy will report how far its scores actually reach).
- **The compliance curve.** Applying each call with probability q: q 0.25 -> 11.21, 0.5 -> 15.14, 0.75 -> 18.35, 0.9 ->
  19.91. Smooth and concave, no collapse at any q, so a partial copy has a readable meaning.
- **The bar: 12.5 full loads or more** (half the reachable gain at +-0.9) on the 96 evaluation worlds, fetch and trail at
  or above the kept-skill floor, live links and energy at the bell no worse than s161's, and a GIF the owner passes.

The self-imitation arm runs the same copier on labels from the brain's own outcomes, with the same rounds, as the
owner ruled.

## 1. The teacher

Exactly the ceiling test's three rules, memory made consistent after review (the ant now remembers the verb the teacher
carried out; a positive control checks the update every decision):

| Rule | Stage | State label (pure function of the state) | Abstains |
|---|---|---|---|
| **dig** | verb, then target | empty jaws: **must-dig** if a foot cell is in reach (target: the deepest foot cells), **must-not-dig** otherwise | shut in (no Walk offered) |
| **down** | target, given Walk | empty, below ground, no foot cell in reach, a deeper step offered: the deepest steps | no deeper step |
| **home** | target, given Walk | empty, on the surface, 3+ columns from the mouth: the steps nearest the mouth's column | within 3 columns |

**The labels are functions of the state, not of the brain's draw** (review finding 1). The acting teacher only
intervened where the brain's sampled pick was wrong; v1 copied those interventions and anchored every other decision to
s161, which cancels the lesson (the dig veto would not have transferred at any anchor weight). v2's teacher emits, at
every decision, which of its states the decision is in and the labelled option set -- whatever the brain picked.

**Ceiling, measured** (seed-clustered): dig + down + home 20.66 +- 0.37, no seed under 3. **Rules interact through the
brain's state**: dig + home without down strands the ant in the shaft in 7 of 48 seeds; why its descent bias collapses
there is untraced (the first explanation was withdrawn in review), and down removes the collapse. All three rules are
copied together.

**Copyability, measured on s161's own runs** (held out by world; within-decision shuffle as the negative control, which
replaced a cross-world control the review showed invalid for step choices):

| Rule | Floor | Brain now | Best model, all option features | Through the brain's own channel |
|---|---|---|---|---|
| dig (foot in reach?) | 0.971 | 0.966 | recall 0.98, precision 0.99 | senses + memory only: recall 0.65 / 0.87; + the deepest offered cell's depth: 0.83 / 0.97 |
| down (deepest step) | 0.406 | 0.619 | 1.000 (shuffled: 0.40) | the step options themselves carry `DEPTH`: same channel |
| home (nearest the mouth) | 0.399 | 0.531 | 0.995-0.998 (shuffled: 0.40-0.41) | the step options carry `HOME`: same channel |

**The dig rule is the hard one**: the brain's job stage sees the dig options only through the best dig target's score,
s161 reads that score at -0.311 on Dig (the sign has to flip), and 62% of dig scores sit above 0.95, so today that
channel barely separates foot from wall (AUC 0.613). **But the verb stage compares jobs, and the walk scores are not
saturated** (measured 2026-10-21 on s161's 24 sealed-world log, 32,613 empty-jawed decisions with a Dig offered, AUC for
"a foot cell is in reach"): the best dig score 0.666 (median 0.974 with a foot cell, 0.967 without); best dig minus the
mean of all options 0.534; best dig minus second-best dig 0.463; **best dig minus best walk 0.888** (median +0.538 against
+0.007) -- at the foot the ant's best step scores about 0.44, elsewhere about 0.96. The Walk job's weight on its own
best-step score is **exactly 0 in s161** (slot 1498, pruned; Turn reads its own at +2.0). That one connection lets
"walking looks poor here" hand the choice to Dig, with no change to the brain's design. It is the first entry on the
allow-list (section 3).

**Why not change the squash instead** (owner's question, 2026-10-21): `x / (1 + |x|)` is there for determinism (no
transcendental in anything a decision reads, `brain.rs`, P-19), for the bounded pick that keeps every option reachable,
and as the network's nonlinearity. The dig channel's trouble is where the scores sit on it (raw scores in the tens,
sharpness at its clamp of 4), not its shape: a squash that bends faster would put them at 0.9999 and separate less, and
this one's gradient falls off as 1/(1+|x|)^2 where `tanh`'s falls off exponentially. A change to what a job reads (its
best target's lead, or the unsquashed score) remains a candidate if the copy shows the channel cannot carry the lesson --
and the copier is what keeps such a change cheap later: an existing brain can be distilled into the changed design
instead of retrained by selection.

**T1 audit** (privileged computation, never privileged information): foot-in-reach is each option's `DEPTH`, `SOIL`,
`CUT`; deepest step is each step's `DEPTH`; nearest the mouth is each step's `HOME`, with `home_dist`/`d_home`. The
teacher's advantage is exactness (the shaft argument, the frame-0 ground), not information.

## 2. The copy target: the ceiling policy, as a distribution

The brain chooses in two stages (`mind::decide`): a verb from the job scores, then a target of that verb from the
scorer, each by `choose_weighted`: p_i = (k + w_i)^2 / sum_j (k + w_j)^2 with w = (s + 1)/2 in [0, 1] and k = 0.05 --
smooth in every weight that reaches s, so the copy is gradient descent on exact log-probabilities.

**The target, pi\*, is the policy that set the ceiling, written from s161's own probabilities** (review finding 1):

| State | Verb stage target | Target stage target |
|---|---|---|
| must-dig | Dig with probability 1 | the deepest foot cells, uniformly |
| must-not-dig | s161's own verb distribution renormalised without Dig (the ceiling's redraw) | s161's |
| down applies (given Walk) | s161's | the deepest steps, uniformly |
| home applies (given Walk) | s161's | the nearest-to-mouth steps, uniformly |
| no rule applies | s161's own distribution | s161's |

**Loss: KL(pi\* || p_copy) at both stages, on every decision.** There is no separate anchor: where no rule applies the
target *is* s161, and where a rule applies the stage it does not touch stays s161's. Losses are normalised per rule (each
rule's decisions carry equal total weight, and the no-rule decisions their own share) so that the ~50:1 majority of
unlabelled decisions cannot drown the dig lesson (finding 14).

**Fetch and trail decisions** -- logged from s161 *and* from each round's student on those arenas -- join with target
s161 (finding 13): the scorer is shared, and "head home" is the same vector an empty forager must not simply follow.

## 3. What may change

- **Trainable:** the target scorer (`S_CTX`, `S_FEAT`, `S_VERB`, `S_OUT`), the job heads (`JOB`, `HID_OUT`), the hidden
  units (`HID`) -- **live slots only, plus a named allow-list** (finding 2). Every live link is billed against the
  soil arena's 2,400 J each tick, and a dense gradient could switch on up to ~1,600 slots (ants would run dry near frame
  9,300). The allow-list starts with the links the dig lesson plausibly needs: **the Walk job's weight on its own best-step
score (slot 1498, 0 in s161)** and `DEPTH` into live scorer units; any
  revived weight that stays under the live threshold is reset to exactly 0 each step; the inputs of scorer units and
  hidden units that nothing reads are zeroed before training, so revival cannot wake weights that drifted unselected.
  **Live links and energy left at the bell are reported for every genome written.**
- **Frozen:** memory weights (`MEM`, `RATE`), emits (`EMIT`), `SWITCH`, `PULL`, and every slot serving only verbs the soil
  arena offers no target for (PickUp, Eat, Feed -- what `protect=` covers in a suite; it is a no-op in a single arena,
  so the copier applies it itself). `SHARP` sits at its clamp of 4; it stays.
- **Memory is not frozen just because its weights are** (finding 9): the scorer feeds memory through all_best and
  all_mean, and the chosen verb is a memory input. The copier backpropagates one step through that path (8 of 12
  memory units have rate 0, so one step is near exact); memory's own weights stay put.
- **Weights stay within mutation's range (+-40)**, so selection can carry on from the copy.

The forward pass extends `brainmath.py` to the target scorer, the hidden units (computed whether or not read, so an
unread unit's gradient is zero by its output weight, not by a gate), both choice stages and the one-step memory path;
gradients by hand in numpy. `best[v]` is a max: the gradient goes to the arg-max option, and the finite-difference
check is run away from near-ties.

## 4. Data: the student drives, the teacher labels

- **The log** (finding 3): every option of every decision, at full precision, plus the teacher's state label and label
  set; settle-phase (rest-mind) rows dropped.
- **Round 0:** s161 on 48 training-style worlds whose seeds are neither the 96 evaluation worlds nor the 192 selection
  worlds; teacher in label-only mode.
- **Rounds 1-5:** the previous copy drives, logged and labelled; its decisions join the pool; retrain from s161 on the
  pool. **All five rounds run** (finding 15); the round kept is the best on **192 fresh worlds**; the 96 evaluation worlds
  are read once, at the end.
- **Split and stopping** (finding 14): a held-out 20% of the pool by seed; training stops when its loss stops falling.

## 5. Controls (each must be able to fail, and each is run before any result is read)

| Control | Expect | Catches |
|---|---|---|
| Genome round trip (read, write back) | identical scores on all 96 worlds | I/O, layout |
| Forward replay against Rust `decide`, **on a perturbed genome**, from full-precision logs | every option's score and both stages' probabilities to f32 rounding | a forward pass that only matches s161 |
| Gradient check (finite differences, away from max ties) | agreement per block | a gradient that is not the forward's |
| **Positive control for the copier** (finding 6): copy s162's choices on s161's states | the copy's choice distribution moves to s162's on held-out states | a copier that cannot move this brain |
| Return to start: s161 + noise, target = s161 everywhere | back to s161's choices and to 6.80 within noise | an optimiser that drifts or damages |
| Shuffled labels: within-decision for steps, within-world for dig | no gain over s161 | "any training helps" |
| Label-only vs acting teacher on one world | identical label sets on every decision the acting teacher touched | two teachers |

## 6. Evaluation and bars

On the 96 evaluation worlds (48 seeds, standard errors clustered by seed), for the round chosen on the 192:

- full loads; **share of the reachable gain** = (copy - 6.80) / (18.23 - 6.80), with the compliance curve beside it;
- seeds under 3;
- fetch and trail against the s91 floor (`keepref=`), every round, not only at the end;
- per-rule agreement on the copy's own runs, and **how far its scores reach** (diagnostics, never bars);
- live links and energy at the bell;
- a GIF beside s161 and the teacher on sealed world 2.

| Outcome | Reading |
|---|---|
| >= 12.5 full loads, kept skills at floor, energy no worse, GIF passed | copying works; step 2 starts a selection run from the copy, with its revived links protected |
| 9.1-12.5 (share 0.2-0.5) | partial; per-rule agreement and score reach say which rule did not transfer |
| < 9.1, or kept skills below floor, or energy worse | does not work on this brain as designed; report what failed |

## 7. The self-imitation arm

Same copier, rounds, split, controls and evaluation; only the labels differ, and they come from **the brain's own
outcomes, per decision, in hindsight** (measurement 1: a run's total is mostly its own luck):

- **dig:** a dig whose pellet became a full load is a positive example; a dig that paid nothing is a must-not-dig
  example. *(v1 quoted these labels as "100% pure against the teacher"; that is true by definition -- a full load
  requires a foot cut -- and is withdrawn as evidence.)*
- **down / home:** a step whose decision was followed, within a window, by a foot cut / arrival at the mouth is a
  positive example; other steps carry no label (dropping a step is not evidence about which step was right).
- **Windows are chosen on self-imitation's own held-out outcome** (the copy's full loads on the 192 selection worlds),
  not by agreement with the teacher.
- **What it cannot do:** label the call the brain never makes -- the ~31 "dig here" states a world where it walked on
  (748 on 24 worlds). The prediction, to be tested, is a smaller share than the teacher arm.

## 8. Open questions and risks

1. **Can the copy reshape the dig channel?** Everything rides on foot cells scoring high and walls low inside a
   saturated scorer (AUC 0.613 today), and on revived or allow-listed links forming. Per-rule agreement and score reach
   will show it; the dig rule is the expected weak point.
2. **Why dig + home strands the ant** is untraced; down removes it in the teacher, but a copy that learns home and dig
   better than down would re-create it. Seeds under 3 are watched every round.
3. **Revived links under selection.** A gain that sits in revived links is deleted by the next selection run's pruning
   and wire cost unless protected -- a change to the brain lane's trainer, flagged for step 2.
4. **The walk-home rule against fetch.** The guard is the fetch arena in the pool and scored every round; a fetch loss
   with no dig gain means the shared scorer cannot hold both as wired.
5. **One general brain.** Structure, senses and choice rule are untouched, and the copy is a start for selection.

## 9. Cost and order of work

All local (4 cores). (1) Label-only mode and the full-precision log: ~2 hours. (2) The copier -- forward extension, the
one-step memory path, gradients, the allow-list and resets: about a day. (3) Controls: hours. (4) Round 0 and rounds
1-5 for both arms: CPU-hours. (5) Evaluation, GIF to the owner, results report. The owner decides on step 2.
