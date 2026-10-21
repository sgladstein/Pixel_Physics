# Adversarial review: the copying method in `step1-copy-design-2026-10-21.md`

*Reviewer's lens: the copier itself (losses, anchor, DAgger, revived links, clipping, bars, controls, the self-imitation
arm). 2026-10-21. Every number below was computed this session from the logs in `/home/user/m2/` and `/home/user/m3/`,
the s161 gen 336 genome (`tp/Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt`) and the code in
`/home/user/tp`. The scripts are in this scratchpad (`chk.py` parses a log into per-decision records; `a1.py`-`a4.py` run
the checks). Nothing under `Pixel_Physics` or `tp` was edited.*

## Verdict

**Build, but only with named changes. Do not build it as designed.** The core idea is sound and worth building:
gradient descent on the exact log-probabilities of the brain's own two-stage choice rule, labels written by the same
Rust teacher that set the ceiling, and DAgger on the student's own runs. As specified, though, four things would make
the result uninterpretable:

1. **Labels depend on what the brain happened to draw.** The design copies the acting teacher's calls, and the acting
   teacher only calls when the brain's sampled pick was wrong. Every decision where the draw happened to be right falls
   into the anchor, which holds it at s161's own probability. By an exact per-state calculation, this leaves the dig
   veto where s161 already is at any anchor weight, and makes the anchor weight a direct cap on how far force, down and
   home can move.
2. **A dense gradient will switch on hundreds to ~1,600 genome slots.** A slot counts as a live link from |w| >= 1e-6,
   and the soil arena bills every live link against its 2,400 J budget every tick. So the copy is scored under a cost
   its objective cannot see. On my estimate, the extra links alone would run the ants dry around frame 9,300.
3. **The ceiling cannot be reached through `choose_weighted`.** `PICK_FLOOR` leaves at least ~2.8 wall digs a world even
   at full saturation (realistically ~5), against the ceiling's 0. That makes the 0.5-share bar an aspiration, not a
   measurement.
4. **Two of the six controls cannot fail.** The anchor-only control starts at the anchor's exact minimum (zero
   gradient). The shuffled-labels control is the cross-world move that the measurements report already found invalid
   for step choices.

All four fixes are cheap. The largest is a change of objective: distil the ceiling *policy* itself, which is a
closed-form function of s161's distribution. That change removes most of the anchor-weight trade-off rather than
tuning it.

---

## BLOCKER

### B1. Labels are conditioned on the brain's sampled pick, and the anchor covers every decision where the draw agreed

**Claim / design point.** Section 2's loss table conditions each call on the draw: force is "foot in reach, brain did
not Dig there", veto is "Dig chosen, no foot cell in reach", and "teacher abstains -> anchor KL(p_s161 || p_copy)".
Section 1b requires the label-only mode to "equal the calls the acting teacher made, decision for decision".

**Evidence.** The acting teacher is pick-conditional throughout:

- `mind.rs:3605`: Force only `if !(chosen.verb == Dig && foot(chosen))`.
- `mind.rs:3609`: Veto only `if chosen.verb == Dig`.
- `mind.rs:3676`: down acts only `if dd > here && dd > depth(pick)`.
- `mind.rs:3691`: home acts only `if d(best) < d(pick)`.

So in label-only mode every decision where the brain's draw already agreed comes out as "Keep", and the design anchors
it to s161.

**Size of the effect.** Take the expected loss at one state over the brain's own draws, reduce it to a binary choice,
and minimise it numerically:

| Anchor weight λ | force P(Dig \| foot), now 0.20 | down P(deepest), now 0.60 | veto P(Dig \| no foot), p99 state, now 0.26 | veto, p90 state, now 0.0175 |
|---|---|---|---|---|
| 0.1 | 0.980 | 0.948 | 0.058 | 0.0149 |
| 1 | 0.840 | 0.760 | 0.192 | 0.0172 |
| 3 | 0.657 | 0.673 | 0.233 | 0.0174 |
| 10 | 0.429 | 0.625 | 0.251 | 0.0175 |

The table rests on these counts, measured on s161's log:

- 1,157 empty decisions a world with Dig offered and no foot cell in reach.
- The brain samples Dig in 1.3% of them (14.5 a world). Only those 1.3% get the veto loss; the other 98.7% get the
  anchor.
- The anchor is stiff exactly where it matters. Its curvature at a small probability p is about 1/p, so holding a
  1%-probability verb in place is cheap for it.

**What goes wrong.**

- The dig veto, worth most of the dig rule's +4.74 alone, does not transfer at any weight in the sweep.
- The "three settings of one knob" sweep becomes a sweep of how far force, down and home are *allowed* to move, not a
  drift-versus-lesson trade-off.
- The veto trains on about 700 decisions in round 0 instead of about 55,000.
- The label-only-equals-acting control would pass, and would certify the bug.

**Fix: distil the ceiling policy, not its interventions.** The ceiling teacher is a deterministic transform of s161's
distribution at each decision, so π_ceiling can be written in closed form from s161's logged probabilities:

- **Foot in reach, empty, not shut in.** π_ceiling = s161's mass on its own foot-cell digs, plus everything else moved
  onto (Dig, the deepest foot cell). This is the Force/Keep logic at 3605.
- **No foot in reach, Dig offered.** π_ceiling(verb) = s161's verb distribution with Dig removed and the rest
  renormalised. This is exactly the redraw at `mind.rs:1752-1763`.
- **Given Walk, down or home in domain.** s161's mass on steps already as deep (or as near the mouth) as the best is
  kept; the rest goes to the teacher's step.
- **Everywhere else.** π_ceiling = π_s161, so the anchor is the same loss.

Minimise KL(π_ceiling || π_copy) at both stages on every decision, the fetch and trail arenas included.

What this buys:

- No λ trades against the lesson inside its domain. λ (or arena weights) now decides only how much the out-of-domain
  and kept-skill states count.
- The veto gets the replacement mix the ceiling actually measured. The design claims "which verb replaces it is the
  brain's (as the ceiling test's redraw)", but `-log(1-P(Dig))` does not preserve the ratios among the other verbs:
  the mass goes wherever the gradient sends it.
- The section 1b control can stay, re-scoped: the label-only targets must match the acting teacher on every decision
  where the acting teacher intervened, and must equal s161's distribution elsewhere.

### B2. A dense gradient switches on hundreds to ~1,600 links, and the soil arena bills every live link from its energy budget

**Claim / design point.** Section 3: "Dead links may revive... The copier reports every revived link."

**Evidence.**

- *What counts as live.* `MindGenome::live` counts every slot from `S_CTX` on with |w| >= 1e-6 (`mind.rs:241, 276`).
- *Live links are billed.* Every tick costs `synapse_fraction * start_energy * live` (`mind.rs:1675-1680`). The soil
  arena keeps that bill at the species' own price on purpose: "a mind that grows links or digs to no purpose runs dry
  before the bell" (`challenges.rs:1381-1385`); "a fully live one lost all six ants by frame ~1,300"
  (`challenges.rs:913-917`, a different budget).
- *Room to grow.* s161 has 706 live links. The trainable blocks hold 2,117 slots, 529 of them live, so up to 1,588
  more can switch on. Any optimiser step of Adam size on a nonzero gradient lands far above 1e-6.

**Estimate of the cost.** Assumptions: `ant.ron` (start_energy 200, synapse_fraction 2.22e-6) and a 6-frame tick.

- 706 links cost about 0.31 J a tick, about 630 J over 12,000 frames.
- In s161's log the energy sense falls from 0.99 to 0.29 of 2,400 J by frame 12,000, about 1,680 J spent. So the brain
  already takes about 37% of the spend.
- With every trainable slot live, the brain tax is about 2,040 J by itself. The total would be about 3,100 J, more than
  the budget, so the ants run dry near frame 9,300.
- `energy` is also one of the brain's own inputs, so a fast-falling energy sense moves its behaviour as well.

**Revival brings in whole unselected detectors, not single links.**

- 7 of the 16 scorer units have output weight 0 but 11-22 live fan-in links each. Those weights drifted under mutation
  with nothing selecting them.
- Hidden units 4 and 5 are unread, with 12 and 8 live inputs.
- A faithful gradient on `S_OUT` and `HID_OUT` is nonzero (d/dw = squash(z)). It revives these units with their
  arbitrary fan-in, in every arena, including fetch and trail, where the anchor is thinnest.
- The one link the design wants, `DEPTH`, is live today only into unit 9, which is one of the dead units (out weight 0).

**What goes wrong.**

- Every control from "anchor only" on, and the copy itself, is scored with a 2-3x brain tax.
- A good lesson can score below s161, and the copier's objective cannot see why.
- "Reports every revived link" will report several hundred of them.

**Fix.**

- Mask the gradient to live slots by default.
- Allow an explicit, named list of slots to revive: `DEPTH` into the live scorer units, the Walk job's `best[Walk]`
  slot (0 today), perhaps a handful more.
- After each step, a proximal L1 or hard-threshold step that sets small revived weights back to exactly 0.
- Before training, zero the fan-in of dead units and unread hidden units. That changes no behaviour, so any revival
  starts clean.
- Every control and every copy reports `live` and the energy at the bell beside its full loads.

---

## MAJOR

### M1. The ceiling cannot be reached through the brain's choice rule, so the bars are aspirational

**Claim.** "Share of ceiling captured = (copy - 6.80)/(20.54 - 6.80)", with a bar of >= 0.5.

**Evidence.**

- *The floor is deliberate and fixed.* `PICK_FLOOR = 0.05` (`mind.rs:250`; its doc says a bottom option is drawn 0.23%
  as often as a top one, by design: dead end P-10).
- *s161 now.* On its log, the expected wall digs are ΣP(Dig | no foot) = 14.7 a world, matching the 14.5 it actually
  sampled.
- *The best a copy can do* (Dig's weight capped, the other verbs as logged):

  | Dig's weight held at | Wall digs a world |
  |---|---|
  | its absolute floor, j = -1 (needs a pre-activation of -∞) | 2.76 |
  | j = -0.9 | 6.69 |
  | s161's own median P (0.0042) | 4.8 |

  The ceiling had 0: the teacher vetoed 7.7 a world and removed every one (`m3/dig_down_home.log`).
- *Exposure depends on the policy.* Under the dig+home acting teacher, below-ground no-foot decisions rose to 1,426 a
  world, against 227 for s161 (`m2/dighome-8row.log`). A copy that strands even partly multiplies its leftover wall
  digs.
- *The copyability numbers do not measure this.* They are argmax accuracies of an unbounded softmax on standardised
  inputs (`probe.py:203, 222`), while "brain now" is a sampled rate. The two are not on one scale: a copy with the
  teacher's argmax can still sample the teacher's call 60% of the time.
- *The response may be a cliff.* The ceiling arms are strongly non-additive (dig+home collapses to 10.59 with sd 6.49).
  A copy at 80-90% compliance may land anywhere on that surface.

**What goes wrong.** A copy can do everything the choice rule allows, land at share 0.3-0.4, and be read as "copying
works partly; ask which rule did not transfer", when the answer is "the rule the ceiling used is not drawable".

**Fix (before building the copier, ~10 minutes of CPU).** Run a compliance ceiling: the acting teacher applies each
call with probability q, q ∈ {0.7, 0.9, 0.97}, plus a floor-limited arm. Leave the veto's leftover at the value
`choose_weighted` imposes with Dig at j = -0.9 and at j = -1. Then:

- Set the bar as a share of the floor-limited ceiling, with headroom from that curve.
- Make the per-correction numbers bars, not diagnostics (T8): wall cuts a world, and offline ΣP(Dig | no foot) per
  world on held-out decisions.

### M2. Two controls cannot fail, and a third cannot pass

1. **Anchor only.** On the anchor alone, the gradient of KL(p_s || p_θ) at θ = s161 is exactly zero (Σ∂q = 0). The
   optimiser never moves, so the control passes whatever the copier does. It cannot catch "an anchor that drifts" or
   "an optimiser that damages".
   - *Fix:* start the anchor-only run from a perturbed genome (s161 with one trainable block noised until it measurably
     drops loads, or a round-0 copy). Show the anchor brings it back toward 6.80 with fetch and trail intact.
   - Separately, run the full pipeline (revival, clipping, thresholding) on all-abstain labels from s161 and report
     `live`.
2. **Shuffled labels "moved to other worlds' decisions".** The measurements report found this control invalid for step
   choices: D4 read 0.98-0.99, because the shaft's steps come in a stereotyped order and a label's position carries it
   across worlds. In the copier, the "shuffled" down labels would still be mostly right, the control would *gain*, and
   it would be misread as "any training helps".
   - *Fix:* shuffle within each decision for step labels (as the probe now does). For the dig labels, permute within
     the domain while keeping each world's base rate.
3. **Forward check "to f32 rounding".** The logs carry context and memory to 4 decimals and option features to 3.
   - I recomputed 199,946 target scores from the logged features: median error 3.9e-5, p99 5.9e-3, max 0.043, with
     5.3% over 1e-3. `brainmath.py`'s worst job-score error is 0.0124. Large weights (S_FEAT ±40, S_OUT to -39.45,
     sharpness 4) magnify the rounding.
   - Also, about 10% of option rows (frames 12,000-14,000) come from the settle's rest mind, with logged scores 0.
     Those must be excluded from training and from the anchor.
   - *Fix:* log full precision in the new label-only mode (`{:e}`/hex), and drop settle decisions.

### M3. The kept-skill guard is thin, and the home lesson plausibly contradicts fetch on the same inputs

**Evidence.**

- The `HOME` option feature is "how much the option points home, by the body's home vector" (`options.rs:196-197`):
  the same feature an empty forager in fetch must *not* simply maximise.
- One scorer (`S_CTX`/`S_FEAT`/`S_OUT`) scores every option of every verb in every arena (`mind.rs:1528-1562`).
  Context reaches a score only by shifting a unit's operating point, so separating "empty on the soil surface -> go
  home" from "empty in fetch -> go out" takes saturated gating weights.
- Section 5 anchors only s161's own fetch and trail decisions, logged once. Drift on the *copy's* fetch and trail states
  is first seen at the final evaluation.
- The probe never tested this: D3 and D4 were fitted on their own domains only. A linear conditional logit also cancels
  every per-decision context term, so its 0.995 / 1.000 says nothing about whether context can switch the preference.

**Fix.**

- *Cheap pre-check:* in s161's own fetch runs, measure how often an empty forager on the surface takes the max-`HOME`
  step. If that is far from 1, fit a model with the brain's structure jointly on soil-home labels plus fetch's own
  choices, held out, and confirm the inputs separate them.
- DAgger the anchor too: log the student's fetch and trail runs each round and anchor there.
- Score fetch and trail every round, not only at the end.

### M4. The route the dig lesson needs is saturated, and the job stage already reads it with the wrong sign

**Evidence.**

- *The scores are saturated.* Sharpness is at its clamp (raw 3.68 -> 4.0, frozen; the gradient would be zero anyway),
  and S_OUT weights run to -39.45. So |sharpness × raw| has median 22.5 and p90 94, and the derivative of a score with
  respect to raw has median 0.007. 62% of dig-option scores have |s| > 0.95.
- *`best[Dig]` barely separates the cases.* Its AUC between foot-in-reach and no-foot states is only 0.613 (no-foot
  median 0.970, foot median 0.974).
- *The job stage's channel is wrong-signed or dead.* The Dig job reads `best[Dig]` with weight **-0.311**, and Walk's
  weight on `best[Walk]` is **0** (pruned). The Dig job pre-squash sits near saturation (j median -0.968).
- *The max routes almost nothing.* `best[v] = max` sends the gradient to one option among near-ties, so "DEPTH ->
  scorer -> best[Dig] -> job" (design risk 1) is a ~1e-3-scaled path through a kink.

**What goes wrong.** The lesson will likely be learned in the job head's direct context, memory and hidden weights.
The probe gives that channel recall 0.65 and precision 0.87, which leaks badly at the 30:1 base rate (1,157 no-foot
against 39 foot states a world). The finite-difference check will also throw false failures at near-tie maxima.

**Fix.**

- Make held-out (by world) mean P(label) per rule, and ΣP(Dig | no foot) per world, the copier's primary offline
  metrics, read *before* any simulation.
- Revive the one `DEPTH` link deliberately (allow-list, initialised at a non-trivial size) rather than hoping a 1e-3
  gradient grows it.
- In the finite-difference check, skip decisions whose top-two dig scores are within the finite-difference step's
  effect.

### M5. Unstated normalisation, and overfitting risk on the dig rule

**Evidence.**

- *Counts are badly unbalanced.* About 2,680 decisions a world are logged, of which about 39 are foot-in-reach and 14.5
  are sampled wall digs. At 48 worlds the anchor outnumbers the dig labels about 50:1.
- *The model is large for the data.* The dense trainable set (2,117 slots) is larger than the round-0 force set
  (~1,870 decisions).
- *The design says nothing about* per-term normalisation, a held-out split by world, or early stopping.

**What goes wrong.** The anchor weight's meaning depends on counts that change every DAgger round, and the dig rule
can be memorised world by world.

**Fix.**

- Per-term means with explicit weights.
- Hold out 25% of worlds within the pool and stop early on held-out P(label) and held-out KL.
- Restrict the trainable slots (B2's fix also does this).

---

## MINOR

- **The DAgger stop rule stops on noise.**
  - The section doesn't say which worlds decide it. On a 48-world set the per-round s.e. is 0.39-0.45 loads (per-world
    sd 2.7-3.1, from `m3/*.csv`): a share s.e. of about 0.03, and about 0.045 for a difference between rounds.
    "Rises by less than one s.e." then halts at round 1 more often than not if the true gains are modest.
  - *Fix:* evaluation is cheap (~2 minutes for 96 worlds), so run all 5 rounds and pick the round on ≥192 fresh
    training-style worlds, never the 96 holdout. Choose λ (if B1 leaves one) on the same set.
- **Memory as an observed input is low risk for this lesson.** Checked, details under "claims checked and found sound".
  - The cheap test before building: score the copier's held-out loss on a dig+down+home *acting* log, whose memory is
    roughly what a good copy will carry.
  - One wrinkle in the ceiling itself: on a Force, memory has already been updated with the brain's *sampled* verb,
    and only `last_verb` is overwritten (`mind.rs:1736, 1750`). So the ceiling ran with memory a copy would not have.
    Only memory unit 5 reads the Walk verb strongly (-1.39, rate 0.5), so the effect is likely small. A rerun of the
    ceiling with memory recomputed for the forced verb would close it (~2 minutes).
- **The clipping rule's rationale is wrong.**
  - Mutation explores ±40 (`train/mod.rs:1085`), not "the start's own range per block". s161's block ranges are
    asymmetric accidents: JOB [-18.66, +6.48], HID_OUT [-10.56, +4.82], S_OUT [-39.45, +19.56].
  - HID already holds a -60, outside mutation's clamp.
  - *Fix:* clip to ±40 and report how many weights hit it.
- **`-log(1-P)` is sound, but blind to the outcome.** It is a correct unlikelihood loss. It is linear at small P,
  though: taking 0.011 to 0.001 is worth 0.01 nats, against a whole wasted trip per wall dig. Under B1's distillation
  target this term disappears. In any case, report expected wall digs alongside the loss.
- **The self-imitation arm is not a like-for-like comparison.**
  - The teacher arm gets up to 5 on-policy DAgger rounds; self-imitation gets one, though it is naturally iterative
    (relabel the copy's own runs).
  - Its hindsight windows (100-400, 60-300 frames) were reported beside teacher agreement. If they were chosen by it,
    the arm is teacher-informed.
  - *Fix:* give it the same number of rounds, and fix its windows before seeing teacher agreement.
- **"Pruning and the per-connection cost delete them in a generation" overstates it.** A given live slot is pruned per
  child with probability pm × prune ≈ 0.009. The real threat to revived links is the synapse tax under selection (B2),
  not one-generation deletion. Link protection is still needed.
- **No autodiff, so hand-written gradients.** The gradients are hand-written through max, squash, verb switches and
  the `hidden_units` read gate (`mind.rs:1492`; the gradient into an unread unit's `HID_OUT` is the unit's true
  squashed value, not 0). Try `pip install autograd` through the proxy first. Otherwise the per-block
  finite-difference check is the only guard, and M4's kink caveat applies.

---

## Claims checked and found sound

- **The choice rule is as the design states it** (`creature.rs:492-512`: weight `(k + max(s, 0))²`). After the
  (s+1)/2 shift the `max` never binds, so p_i is smooth in every upstream weight, and exact log-probability training
  needs no RL estimator.
- **The target stage has ample headroom.** s161's P(label | Walk) is 0.60 for down and 0.50 for home. The floor allows
  ≈0.997 with 5-6 options. Force can go from 0.20 to about 0.99. Only the veto is floor-limited (M1).
- **The dig target loss is nearly vacuous, harmlessly.** Mixed foot and wall cells in reach are rare (9 of 535 digs), so
  `-log P(target in foot | Dig)` is almost always already satisfied.
- **Reusing the Rust teacher for labels is right.** A Python port would be a second, unvalidated teacher.
- **The bars are decidable at the stated noise.** On 96 worlds a copy's share s.e. is ≈0.03, so 0.2 and 0.5 are about
  9 s.e. apart. Noise is not the problem; headroom is (M1).
- **Evaluation is separated properly.** Choosing on a 48-world set and reading the 96 holdout once is sound,
  provided DAgger's stop rule also stays off the holdout.
- **Memory as an observed input holds for the dig and down verb stage.**
  - 8 of 12 memory units have rate 0: each is a squashed summary of the previous decision, so there is one step of
    history and no long recurrence to backpropagate through. Two have rate 0.5 and two 0.995 (half-life 138
    decisions).
  - In empty, below-ground, no-foot states, memory's share of the walk-minus-dig margin is +4.42 ± 0.12 under s161
    and +4.39 ± 0.09 under the teacher-driven dig+home run. Memory is nearly constant there and barely moves when the
    behaviour changes. DAgger's student-run logging covers the rest.
- **Pull does not compound leftover wall digs.** In empty no-foot states the last verb is never Dig (0 of 27,773):
  after a dig the ant is carrying.
- **DAgger with "retrain from s161 on the aggregate pool" is textbook DAgger** and affordable here: rounds are minutes
  and the forward pass is tiny. The convergence worry is the stop rule's noise, not the aggregation.
