# Adversarial review: the evidence behind step 1 (copying the three-rule teacher into s161)

*Reviewer lens: evidence and statistics. Reviewed 2026-10-21 against `measurements-2026-10-20.md`,
`step1-copy-design-2026-10-21.md` and `guidelines-2026-10-20.md`, the CSVs and scripts in `measurements/`, the logs
in `/home/user/m2` and `/home/user/m3`, and the scratch tree `/home/user/tp` (read only). Every number below marked
"recomputed" or "reran" was produced in this review. The scripts are in this scratchpad: `arms2.py`, `repro.py`,
`brainsharp.py`, `strand.py`. Nothing under the repo or `tp` was edited. One extra run, the dig-only teacher arm logged
on the 24 sealed worlds, was written to `digrun/`.*

## Verdict

The headline evidence holds up. Teacher-driven s161 reaches 20.54 full loads against the brain's 6.80. That gain
survives everything I threw at it: mirrored-twin clustering, a rebuild check, and the per-set split. The probe's
held-out scores also survive regrouping the folds so that mirrored twins never sit on opposite sides of a fold. **But
step 1 as designed cannot decide the one thing it says it decides.** Its bars treat a share below 0.2 as "copying
does not work on this brain". Two things are missing that such a reading needs. First, nothing shows the copier can
move this brain's behaviour at all. Second, nothing shows how the teacher's own gain responds to partial compliance.
The trace I ran suggests that response is a cliff, not a slope. Around this sit three more problems:

- The stated mechanism for the dig + home collapse ("s161 never learned to descend") is contradicted by s161's own
  logs.
- The design's "shuffled labels" control is the cross-world control that measurement 2 already found invalid for step
  choices.
- Memory is coupled to the parts being trained, and the ceiling ran with memory that disagrees with the forced action.
  So "memory as an observed input" and the ceiling itself both rest on an assumption the genome contradicts.

**Recommendation.** Build it. But first run four cheap checks of a few minutes each:

1. Compliance dose-response on the ceiling.
2. The ceiling rerun with memory consistent with the forced verb.
3. A positive control for the copier.
4. Standard errors clustered by seed.

And fix the shuffled-label control and the DAgger stopping rule before any result is read.

---

## BLOCKER

### B1. A low or partial share cannot be read as "copying does not work": no copier positive control, and the teacher's response to partial compliance is unmeasured and probably a cliff

**Claim.** Design section 6, bars: share < 0.2 means "copying does not work on this brain as designed". Share
0.2-0.5 means "copying works partly", and per-rule agreement says which rule did not transfer.

**Checked.**

- **The controls in section 6.** The zero-step round trip proves genome I/O. The forward and gradient checks prove the
  maths. Anchor-only proves the copier does no harm. **None shows the copier can move s161 to a known different
  behaviour.** So a share near 0 has two readings the design cannot tell apart: the copier or optimiser failed, or
  the brain cannot hold the lesson.
- **Partial teachers already cover the whole band, and they are not additive.** Recomputed from `ceiling/*.csv`:
  - down alone: share 0.25
  - dig alone: share 0.34
  - dig + home: share 0.28, with a bimodal distribution (p10 3, p90 20, 9 worlds under 3)
  - dig + down: share 0.74

  A copy that learns dig and home but under-learns down does not land "partially". It reproduces the dig + home
  collapse.
- **The descent margin is razor thin** (`strand.py`, my trace of `/home/user/m2/dighome-8row.log`). In the dig + home
  arm, look at the state "empty, below ground, no foot cell in reach, Walk chosen, a deeper step offered". In the 6
  stranded sealed worlds, s161 takes the deepest step 0.377-0.383 of the time and steps *up* 0.388-0.402. That is a
  random walk biased slightly upwards. In the worlds that were not stranded it is 0.40-0.43 against 0.34-0.38. The
  ceiling's rescue is ~80 "down" forces a world (`dig_down_home.log`: 7,719 over 96 worlds). How many of those a
  copy must reproduce, and how reliably, is not known. A copy at 90% compliance on "down" might sit anywhere between
  the dig + home regime and the full ceiling.

**What is wrong.** The bars assume the share grows smoothly with how well each rule transferred. The one
measurement that bears on this (dig + home against dig + down + home) says the response has a cliff. And without a
copier positive control, a low share is uninterpretable.

**Fix (both cheap, before building the copier).**

1. **Compliance dose-response on the ceiling.** Add a per-decision draw to the scratch teacher: act with probability
   q, separately for the dig rule and for the down/home rules. Run `reliab` at q = 0.5, 0.7, 0.85, 0.95 (about 15
   seconds per arm on 4 cores; I timed `reliab` at 14.5 s for 96 worlds). Also run "dig + home at q = 1, down at
   q < 1", the realistic partial copy. Set the bars from that curve, not from "half the gain".
   - I attempted this in a copy of `tp` inside the scratchpad. The build was refused by the session's permission
     classifier, so it is not done.
2. **A copier positive control.** Distil a behaviour known to be reachable from s161's own weights into s161, with
   the same pipeline. Candidates: a trainer child whose per-world scores are in `rho8.csv`, s162 gen 338 (7.50), or
   simply the acting teacher's *executed* picks on round-0 states. Pass means it recovers that genome's score, or its
   per-rule agreement. Only then does "share < 0.2" mean anything about s161.

---

## MAJOR

### M1. The diagnosis "the brain had never learned to descend; it got down by cutting whatever wall was beside it" is contradicted by s161's own logs

**Claim.** Measurements section 3 ("Why each failure, traced"), guidelines T9 ("a veto needs a replacement... the
brain had only ever got down by digging"), and design section 1 (the reason the down rule exists).

**Recomputed** (`strand.py` over three logs: s161 alone, dig + home, and the dig-only arm I logged on the same 24
sealed worlds). The state is: empty, below ground, no foot cell in reach, a deeper step offered.

| Run | Brain takes deepest step | Brain steps up | Such decisions per world |
|---|---|---|---|
| s161 alone | 0.50-0.73 | 0.10-0.28 | ~150-350 |
| dig only (no stranding: mean score 171.5; 2 of 96 worlds under 3) | 0.41-0.60 | 0.19-0.37 | ~300-1,270 |
| dig + home | 0.38-0.43 | 0.34-0.40 | ~500-1,950, in every world, not only the 6 stranded |

s161's brain-chosen Dig rate in this state is 0.02-0.06 alone. On the probe's D4 set, its own scorer ranks the
deepest step first 73% of the time (argmax of the logged scores, `brainsharp.py`). So it descends by walking, not by
cutting.

**What is wrong.**

- The brain's descent bias *depends on its state*, and the teacher's rules shift that state: dig shifts it somewhat,
  dig + home shifts it almost to zero.
- The dig-only arm vetoes exactly the same wall cuts and does not strand. So the veto alone does not explain the
  collapse; the interaction with walk-home does.
- The real mechanism (memory? `since_nest`? energy?) is unknown.
- CLAUDE.md's rule: "why did it do that" is answered by tracing individuals with their inputs. One world was traced
  for *what* happened, not for which input made the brain stop descending.

**Why it matters for step 1.** Whatever makes the brain's walking unbiased after a walk-home is exactly what a copy
that freezes memory (M3) and is trained on round-0 states (s161-alone states, where descent is fine) will not see.
The D4 copyability probe ran on s161-alone states, where the brain already descends.

**Fix.**

- Reword the diagnosis to: "under dig + home, the brain's own descent bias in the shaft falls from ~0.62 against
  0.18 to ~0.40 against 0.37 in every world, and turns slightly upward in the stranded ones".
- Trace the inputs (ctx, memory) of the D4-state decisions, s161 alone against dig + home. Find which input differs.
- Re-run the D4 probe on the dig + home log, the states the copy will actually face.

### M2. The design's "shuffled labels" control is the invalidated cross-world control, and for the down rule it would teach the right answer

**Claim.** Design section 6 controls: "Shuffled labels (teacher labels moved to other worlds' decisions) | no gain
over s161 | 'any training helps'".

**Checked.** Measurement 2 found this exact construction invalid for step choices: the moved labels score 0.98-0.99 on
D4. I confirmed why with `repro.py`. A model given *only the option's position in the list* (one-hot) predicts the D4
label at **1.000** held out. Position-only reaches 0.44-0.49 for D3, against a floor of 0.40; that explains D3's old
0.49 control reading, so the measurements report's explanation is right.

**What is wrong.** Moved to another world's same-size decision, the down labels are still almost all correct labels.
A copier trained on them should learn "go down" and gain. The control would then "fail" for a reason that is not a
bug, or be explained away after the fact.

**Fix.** Use the within-decision shuffle for step choices, and the world-rotation shuffle (as `binary_report` does)
for the dig verb call. Better still, use a label from a privileged fact the ant cannot sense (for example, a
per-world random preferred column). That would also catch leakage, which a within-decision shuffle cannot: it destroys
all structure, so it reads the floor even when the folds leak.

### M3. Memory is not "frozen" by freezing MEM, and the ceiling ran with memory that disagrees with the forced action

**Claim.** Design section 3: "Memory is an observed input, not trained... the copier reads it as a sense". Section 8
question 2 asks whether this is safe.

**Checked.** My reading of the genome layout puts `VERBS = 9`, `MEM` at 1,788 and `M_IN` at 46 (this reproduces the
RATE genes at indices 11-13 in `parent.txt`). In `decide`, each memory unit reads `all_best` and `all_mean` (the
target scorer's outputs) and the chosen verb. In s161:

- **7 of 12 memory units have a nonzero weight on `all_best` or `all_mean`**, among them unit 5 (mean, 1.887) and
  unit 11 (best, 2.037).
- **11 of 12 read the chosen verb.**
- The scorer and the verb choice are what step 1 trains. So changing them changes every later memory, and the
  copier's gradient ignores that path.

Separately, in the ceiling (`mind.rs` diff, the `Teach::Force` branch), a forced dig sets `last_verb` but leaves
`mind.state.mem` as `decide` computed it from the brain's *own* sampled verb. The veto path does reset and recompute
memory. So on ~17.6 forced digs a world (`dig_down_home.log`: 1,694 over 96), the ceiling ant's memory says it did
what the brain chose, not what it did.

**What is wrong.**

- 20.54 is the ceiling of an agent no brain can be: one whose memory disagrees with its actions. It could sit above
  or below what a consistent brain reaches.
- The copier's round-0 data (s161 memory) is inconsistent with the trained scorer's memory.

**Fix.**

- Rerun the ceiling with a force implemented as "decide restricted to the forced option", which recomputes memory
  consistently, as the veto already does. It takes one arm at about 15 seconds. Re-derive the bar from that number.
- In the design, say that DAgger rounds are what repair the memory mismatch, and that round 0 alone is not evidence
  either way.

### M4. The "96 worlds" are 48 seeds, many of them exact duplicates; standard errors are understated and counts double-count

**Claim.** Measurements 1 and 3, and the design's evaluation: "96 worlds, with their standard errors", "no world under
3", "9 of 96 worlds".

**Recomputed** (`arms2.py`). `training_layouts` gives the two members of a mirrored pair the same seed. The soil bed
is mirrored and the mind's random stream is keyed by `world.seed`, so twins often run the same trajectory mirrored.

- Twins with *identical* full loads: none 14/48, dig + down + home 30/48, dig + down + home + drop 46/48.
- Correlation between twins: none 0.54, dig + down + home 0.86.
- In the logs, identical twins have identical decision counts (for example 4,716 and 4,716; 5,586 and 5,586).

Standard errors clustered by seed against the reported ones:

| Arm | Reported s.e. | Clustered s.e. |
|---|---|---|
| none | 0.32 | 0.39 |
| dig + down + home | 0.27 | 0.38 |
| dig + home | 0.66 | 0.84 |
| dig + drop + home | 0.67 | 0.93 |

Seed-level counts against the reported world counts:

- "9 of 96 worlds under 3" (dig + home) is 7 seeds, 2 of them with both twins.
- "6 of 24 stranded" is 5 seeds (k = 16 and 17 are identical).
- In measurement 1, s162 − s161 is +0.70 ± 0.46 clustered (1.5 s.e.), not ± 0.38.

**What is wrong.** Every "± s.e." and "n of 96" in the three documents treats twins as independent. This does not
change the headline: dig + down + home − none = +13.74 ± 0.43 clustered, and both 48-world sets agree (20.77 and
20.31 against 6.27 and 7.33). It does change the margins. The design will compare copies of nearby value (anchor
sweep, DAgger rounds, the self-imitation arm), and those comparisons need the clustered error.

**Fix.** Cluster by `(set, k // 2)` in `arms.py` and `rho.py`. Report seeds as well as worlds for tail counts. Or
evaluate the copy on one side of each pair plus fresh seeds, which buys more independent information per CPU-second.

### M5. The design's copyability table quotes the wide-channel dig number; the brain's own channel reads 0.65-0.83, and that number has no script

**Claim.** The design section 1 table gives dig: "recall 0.98, precision 0.99 | 16 units 1.000". The guidelines say
"94-100% from the brain's own inputs".

**Checked.** `repro.py` reproduces the wide-channel numbers exactly: 914 of 934 recall, 6 false alarms. That probe
gives a model the mean and max of all 45 features over every Dig option. The brain's verb stage sees options only
through the best Dig target's score. Through that channel the measurements report gives recall 0.65 (senses and
memory only) or 0.83 (plus the deepest offered cell's depth). **No saved script produces those two numbers.**
`probe.py`, the copy in `tp/scratch-analysis` and `m1/` contain no narrow-channel fit. The table row also mixes
recall/precision (linear) with accuracy (16 units), on a label that is 97% "no".

There is one more obstacle, from my reading of the layout: s161's Dig job weight on the best Dig target score is
**−0.311**. Today a better Dig target *lowers* P(Dig). The copy route "the scorer learns foot-ness, then best[Dig]
lifts P(Dig)" needs that sign flipped as well as the DEPTH link revived.

**What is wrong.** The most important rule (dig: +4.74 alone) is the least copyable through the brain's wiring, and
the design's headline table hides it behind the wide-channel number.

**Fix.**

- Put the narrow-channel row (0.65 / 0.83 recall) in the design table as the operative number. Mark the wide one as
  "information present".
- Check in the narrow-channel script.
- Better still, fit the probe through the brain's own two-stage forward pass (`brainmath.py` extended), which is what
  the copier will be.

### M6. The DAgger stopping rule reads the evaluation worlds

**Claim.** Design section 4 says to stop "when the share of ceiling captured (section 6) rises by less than one
standard error between rounds". Section 8 point 5 says the 96 evaluation worlds are not used for choosing, and the
final number is read once.

**What is wrong.** Section 6's share is defined on the 96. Reading it every round to decide when to stop is
selection on the evaluation set: up to 5 looks.

**Fix.** Compute the stopping share on the separate 48-world sweep set (or another fresh set). Read the 96 once at
the end.

### M7. The self-imitation arm is set up to lose, and its "label check" partly measures a definition

**Claim.** Design section 7: self-imitation labels "teacher agrees kept 1.00" on dig. The prediction is that
self-imitation captures clearly less.

**Checked.** I reran `sil.py` (a copy, on `s161-8row.log`). It reproduces the design's figures:

- dig: kept 0.30, kept agreement 1.000, dropped 0.056
- down: 0.809 / 0.533
- home: 0.644 / 0.355 at the defaults

But in `sil.py` a pellet is a full load only if `held["cls"] == "foot"` (`held["full"] = away and lift >= S and
held["cls"] == "foot"`). So "kept implies the teacher agrees" is true **by construction**, not a measurement. Two more
imbalances:

- The teacher arm gets up to 5 DAgger rounds on its own states. Self-imitation gets round 0 only (section 9's order:
  "DAgger rounds (teacher arm)"), although it can be iterated on the copy's own runs in exactly the same way.
- The purity of self-imitation labels is scored by agreement with the teacher, which presupposes the teacher is the
  answer. Guideline T7's two rules (a bar above the noise, rank by speed) are not applied.

**What is wrong.** The owner ruled the two arms run side by side. As designed, the comparison is unequal in data
(rounds), and its one "perfect" label statistic is a tautology.

**Fix.**

- Drop or footnote the 1.00. Report what share of the brain's *foot* cuts became full loads, which is the real filter
  rate.
- Give self-imitation the same number of on-policy rounds.
- Judge both arms only on the score, which the design already does for the bar.

---

## MINOR

1. **Measurement 1's "pairing would cost about 2x" is a framing, not a finding.** At rho ≈ 0, a paired comparison
   on n worlds equals an unpaired one in which the parent is also measured on n worlds. It is "2x worse" only against
   a parent whose mean is known exactly. The substance stands (recomputed):
   - Competitive children: median rho 0.114 and pairing factor 2.47, n = 5.
   - Rho of means over mirrored pairs: 0.09.
   - With clustered s.e., 24 of 30 children (not 25) are clearly worse.

   The rho between two good brains (0.12) is one pair of brains, with a 95% bootstrap CI over seeds of −0.18 to 0.36.
   "Per-world noise is the run's own luck, not the world's difficulty" rests on that one comparison plus children that
   are mostly broken (12 of 30 under 3). Broken children have little variance, which drives rho down on its own.
   Scope it to "this parent, at this step size, at a local optimum". The cost table now strikes paired worlds for all
   future training; for a parent on a slope (early rungs) it was never measured.
2. **The drop "coupling" story is short of arms and has an alternative explanation that was not ruled out.**
   - There is no home + drop arm and no dig + drop arm. "Drop helps only beside walk-home" is inferred from
     dig + drop + home against dig + home (+2.15 ± 0.94 clustered: possibly drop rescuing the dig + home strandings,
     not coupling with home) and from +0.42 beside dig + down + home (inside the noise, as the report says).
   - The GIF shows the rule building two posts beside the mouth. Posts that obstruct or lengthen the walk back into
     the shaft are an alternative cause of "put-down to next foot cut 655 → 992 frames". Margin 1 rules out sliding,
     not obstruction.
   - The lesson was promoted to guideline T9 ("behaviours the brain has coupled must be taught together").
   - Fix: run the 2x2 (drop × home) at fixed dig + down, and trace return paths against post positions.
3. **The negative controls that are accuracy at the floor are weak.** D1b's control reads 0.972 against a 0.972
   floor on a label that is 2.9% "yes". A model that learned nothing, and one with a modest leak, both read 0.972.
   Report recall/precision or AUC for the control too. D2's positive control has a 0.971 floor, so it is
   uninformative, but D2 is out of step 1.
4. **Fold leakage exists structurally but measures as negligible.** `folds_of` assigns world index i to fold i mod 4,
   so every mirrored pair (log worlds 2j and 2j+1) is split across folds. A test world's exact mirror image is often
   in training. Regrouping folds by pair (`repro.py`, linear):
   - D1b recall 913/934 (against 914/934), false alarms 8 (against 6)
   - D3 0.9936 (against 0.9946)
   - D4 1.000 (against 1.000)

   Make pair-folds the default anyway.
5. **D4's 1.000 says little.** Option position alone gets 1.000. The deepest step sits in a fixed slot, so "go down"
   is "take that slot". The informative D4 numbers are elsewhere: the brain's argmax already agrees 0.734, its
   sampled choice agrees 0.619, and the expected probability under `(k + w)^2` is 0.613. So for "down" the gap is
   partly sharpness, not knowledge. For "home" it is knowledge: argmax 0.552, sampled 0.531.
6. **The records claim controls that are not in the saved outputs.**
   - The design says both controls passed and walk-home's control "is re-run". `m2/probe-D3b.out` held only its
     header when the documents were written (the run was still going at 02:16).
   - D4's quoted within-decision control (0.401 / 0.405) is not in `m2/probe-D4.out`.
   - The narrow-channel numbers have no script (M5).
   - "sil.py reproduces every world's full loads exactly": the script checks nothing per world. The total (159 kept
     equals 159 full loads) does match.
7. **Forking paths on the ceiling are real but small here.** The down rule was invented after the dig + home failure,
   on the same 96 evaluation worlds. dig + down + home was then picked among 12 arms. Any winner's-curse inflation of
   20.54 is a few tenths at most against a +13.74 gain, and it makes the 13.7 bar slightly harder, not easier. It does
   mean the teacher was tuned on the evaluation worlds. Say so, and score the final copy on fresh seeds as well as
   the 96.
8. **Cost claims in the guidelines' "answer first".**
   - The arithmetic checks: $0.65 per 100 generations gives ~15,400 per $100, or ~9,050 at 1.7x. A Ferrante-sized run
     of 2,000 generations comes to about $44.
   - "$5-15 with the levers" needs copying to cut a *colony* run 3-8x. The same document decides "copying for
     individual skills; colony scoring for collective ones". Copying's measured value (step 1, if it works) is a
     solo-rung saving and says nothing about a colony run's generations.
   - The 10x literature figure comes from networks with at least ~10,000 parameters. The guidelines say so, but the
     answer-first paragraph does not.
   - Time claims are conservative: I measured `reliab` at 14.5 s for 96 worlds on 4 threads, and a logged 24-world
     run at 14.5 s single-threaded, against "two minutes" and "3 s per world".
9. **The bar reads "kept skills at floor" and "GIF passed", but the anchor sweep picks "the best" of 3 weights on a
   48-world set.** With clustered s.e. near 0.5 on 48 worlds, differences between anchors under ~1 load are noise.
   State that ties go to the heavier anchor.

---

## Verified as sound

- **The ceiling numbers reproduce exactly** from `ceiling/*.csv`: every mean, unpaired s.e., p10/median/p90 and
  10+ count in the table.
- **The arms came from two builds, and that does not matter.** The `dig`, `home`, `dig_home`, `drop` and
  `dig_drop_home` arms predate the build that added `down` and the no-facts counter. I reran `none`, `dig`, `home`,
  `dig,home` and `dig,drop,home` on the current binary:
  - per-world full loads are identical to the stored CSVs;
  - the teacher counters are identical;
  - every arm shows ~381 no-fact decisions a world, so the teacher never acted in the scored settle.
- **20.54 against 6.80 is robust.** It holds with seed-clustered s.e. (+13.74 ± 0.43), in both 48-world sets
  separately, and with no seed under 3.
- **`rho.py` is correct.** The positive control is exact (rho 1, every difference 0) and the shuffle control reads
  −0.002. "No saving from pairing at the trainer's step size" holds for competitive children (rho 0.11) and with
  twins averaged (rho 0.09).
- **The choice rule in design section 2 matches the logs.** The expected probability on the teacher's options,
  computed from the logged scores with `(k + w)^2`, k = 0.05, is 0.613 for D4 and 0.527 for D3. The observed rates
  are 0.619 and 0.531. Gradient descent on log-probabilities is a sound copier.
- **The D1b facts reproduce:** 934 foot-in-reach decisions, 186 digs among them (P = 0.199), P(dig | no foot) 0.011,
  535 digs, 65% without a foot cell, 748 untaken "dig here" calls.
- **The probe has no meaningful train/test leakage** once folds are grouped by mirrored pair (MINOR 4). The D3
  cross-world control's 0.49 is explained by option position, as the report says.
- **The self-imitation numbers in the design's table reproduce** (with the caveat in M7).
