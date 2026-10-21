# Step 1 copy design: the adversarial review and what was done about it

*2026-10-21. Three independent reviewers attacked [`step1-copy-design-2026-10-21.md`](step1-copy-design-2026-10-21.md)
and its evidence from three sides -- the copying method, the evidence and statistics, the code against the design --
each verifying claims against the source and the data rather than the documents. Their full reports are in
[`review/`](review/) (`review-method.md`, `review-evidence.md`, `review-code.md`). This file is the disposition: every
BLOCKER and MAJOR finding, accepted or not, and what it changed.*

## Verdict

**All three: build, but not as designed.** The core method (gradient descent on the exact two-stage choice
probabilities, labels from the Rust teacher that set the ceiling, the student's own runs) is sound; the headline
evidence survives every check the evidence reviewer ran (the ceiling table reproduces exactly on the current binary;
+13.74 +- 0.43 clustered by mirrored pair; the probe holds with mirrored twins in one fold). But as specified the copier
would cancel its own lesson, could starve the ant, and was judged against a bar no brain can reach -- so a low result
would have meant nothing.

## Findings and dispositions

| # | Finding (reviewer) | Disposition |
|---|---|---|
| 1 | **Labels depend on the brain's draw** (method B1, code B1). The teacher only calls when the sampled pick was wrong, and the design anchored every other decision to s161 -- including the ones the draw got right. Optimum at one state: P(dig at a foot cell) 0.84 at anchor weight 1, the DAgger fixed point 0.56; the dig veto does not transfer at any weight. | **Accepted.** Pure state-label functions (the foot-cell set, must-dig / must-not-dig / abstain, the deepest-step set, the nearest-to-mouth set) applied to *every* decision their rule covers; the copy target is the ceiling *policy* as a full distribution (a force is the teacher's choice; a veto is s161 renormalised without Dig, the ceiling's own redraw; abstention is s161), minimised by KL on every decision. The anchor disappears as a separate term. |
| 2 | **Revived links would starve the ant** (method B2). A dense gradient switches on hundreds to ~1,600 slots; the soil arena bills every live link from its 2,400 J; fully live, ants run dry near frame 9,300. Revival also wakes 7 dead scorer units and 2 unread hidden units whose inputs drifted unselected. | **Accepted.** Gradient on live slots plus a named allow-list (e.g. `DEPTH` into a live scorer unit); revived weights below threshold reset to exactly 0 each step; dead units' inputs zeroed before training; live links and energy at the bell reported every run. |
| 3 | **The log cannot support a forward pass for any genome but s161** (code B2): only Walk, Dig and Drop options are logged, TurnAround's job reads its best target at weight +2.0, and the 3-4 decimal precision cannot pass a "reproduce to f32" check (p99 error 5.9e-3 measured). | **Accepted.** Log every option at full precision; drop the settle's rest-mind rows; a replay check against Rust `decide` on a *perturbed* genome. |
| 4 | **The ceiling's ant remembered something other than what it did** (code M2, evidence M3). A forced dig kept the memory update of the brain's own verb; a veto's redraw summed the options without Dig's. | **Accepted and fixed in the teacher** (`scratch_log::remember`, with a positive control: re-running the brain's own verb must reproduce `decide`'s memory, counted every decision). Ceiling re-run: see "Checks run" below. |
| 5 | **The bar is aspirational** (method M1, evidence B1). `choose_weighted`'s floor leaves ~2.8 wall digs a world even at full saturation; partial teachers span the whole share band and do not add (a cliff). | **Accepted.** Compliance curve (each call applied with probability q) and a floor-limited arm (each call obeyed only as often as a saturated brain would) run before any bar is set; the bar comes from the floor-limited ceiling. |
| 6 | **No control shows the copier can move this brain** (evidence B1). | **Accepted.** A positive control for the copier: copy a target that is known and reachable (s162's own choices on s161's states, or a synthetic shift of one option feature's weight) and recover it. |
| 7 | **The shuffled-label control is the invalid cross-world one** (method M2, code M1, evidence M2). | **Accepted.** Within-decision shuffle for step labels, within-world stratified shuffle for the dig label. |
| 8 | **Two controls cannot fail** (method M2): anchor-only starts at its own minimum; f32 reproduction is impossible from the log's precision. | **Accepted.** Anchor-only replaced by a perturbed start that must return to s161's scores; reproduction check against full-precision logs. |
| 9 | **Freezing `MEM` does not freeze memory** (code M3, evidence M3): the scorer feeds memory through all_best / all_mean, and the chosen verb is a memory input. 8 of 12 units have rate 0. | **Accepted.** One-step truncated backpropagation through the scorer-to-memory path (cheap and near exact at rate 0); memory's own weights stay frozen. The method reviewer measured memory's term in walk-vs-dig as unchanged under the teacher (+4.42 +- 0.12 vs +4.39 +- 0.09), so v1 risk is low. |
| 10 | **"The brain had never learned to descend" is contradicted by s161's own logs** (evidence M1): alone it takes the deepest step 0.50-0.73 of the time; dig-only (which vetoes the same wall cuts) does not strand; under dig + home the bias falls to ~0.40 vs 0.37 *in every world*. | **Accepted; the diagnosis is withdrawn** in the measurements report and guideline T9. The descent bias depends on the brain's state, which the teacher's rules shift; the memory mismatch (4) is a candidate cause, tested by the memory-consistent re-run. "Go down" fixed the outcome; the reason it was needed is open. |
| 11 | **The 96 worlds are 48 seeds** (evidence M4): mirrored twins, 14-30 of 48 identical; s.e. understated 20-40%; "9 of 96 under 3" is 7 seeds; s162's lead is +0.70 +- 0.46. | **Accepted.** `arms.py` clusters by seed; reported numbers corrected. |
| 12 | **The design quoted the wide-channel dig number** (evidence M5): 0.98 recall needs every option's features; through the brain's own channel 0.65-0.83 (no saved script); s161's Dig job reads its best-dig score at -0.311, so that sign must flip; 62% of dig scores sit above 0.95, so that channel barely separates foot from wall (AUC 0.613; method M4). | **Accepted.** The design quotes the narrow channel; the narrow-channel check is saved as a script; the copier is allowed to move sharpness (at its clamp of 4) only if the gradient needs it, reported. |
| 13 | **Fetch/trail guard is thin; walk-home may contradict fetch** (method M3): one scorer, and `HOME` is the same vector an empty forager must not simply follow. | **Accepted.** Fetch and trail scored every DAgger round; the student's fetch/trail runs join the pool; a pre-check of the teacher's home rule on fetch runs. |
| 14 | **Normalisation unstated; overfitting** (method M5): anchor decisions outnumber dig labels ~50:1; 2,117 trainable slots vs ~1,870 round-0 force examples. | **Accepted.** Per-rule loss normalisation, a held-out split by seed, early stopping on it. |
| 15 | **DAgger stopping reads the evaluation worlds; stops on noise** (evidence M6, method minor). | **Accepted.** All 5 rounds run; the round is chosen on 192 fresh worlds; the 96 evaluation worlds are read once. |
| 16 | **The self-imitation arm is set up to lose** (evidence M7, method minor): its dig purity of 1.00 is true by definition (a full load requires a foot cut); no DAgger rounds; labels scored by teacher agreement. | **Accepted.** The 1.00 is withdrawn as evidence; self-imitation gets the same rounds; its windows are set from its own outcomes, not teacher agreement. |

Minor findings (code: `protect` is a no-op for a single arena and covers PickUp/Eat/Feed; clip range should be
mutation's +-40, not the start's; saturation needs a stated optimiser; label-only vs acting agreement is near tautological;
method: windows; evidence: one-pair correlation interval, drop-coupling arms) are taken into the revised design; the
reports carry them.

## Checks run before revising (all local)

s161 gen 336, 96 training-style worlds = 48 seeds, 8-row shaft, 12,000 frames; standard errors clustered by seed
(`measurements/arms.py`); every arm's `remember` positive control read 0 mismatches.

| Arm (dig + down + home unless named) | Full loads | Share of the teacher's gain | Seeds under 3 |
|---|---|---|---|
| brain alone | 6.80 +- 0.39 | -- | 2 |
| teacher, memory now consistent (was 20.54) | **20.66 +- 0.37** | 1.00 | 0 |
| each call applied with probability 0.25 | 11.21 +- 0.41 | 0.32 | 1 |
| ... 0.5 | 15.14 +- 0.35 | 0.60 | 0 |
| ... 0.75 | 18.35 +- 0.36 | 0.83 | 0 |
| ... 0.9 | 19.91 +- 0.36 | 0.95 | 0 |
| floor-limited, lenient (leaks only where the brain's draw was wrong), scores +-1 / +-0.9 / +-0.8 | 20.55 / 20.92 / 20.35 | ~1.0 | 0 |
| **floor-limited, strict (every covered decision drawn from the saturated policy), scores +-1** | 20.34 +- 0.40 | 0.98 | 0 |
| **... scores reaching +-0.9** | **18.23 +- 0.37** | **0.83** | 0 |
| dig alone, memory consistent (was 11.54) | 11.96 +- 0.40 | -- | 1 |
| dig + home, memory consistent | 11.01 +- 0.96 | -- | **7** |

What they settle:

- **The memory mismatch did not inflate the ceiling** (20.54 -> 20.66) **and did not cause the stranding** (dig + home
  still collapses 7 of 48 seeds with memory consistent). Finding 10's open question stays open: the teacher's walk-home
  calls shift a state the brain's descent depends on, and the untraced input is a follow-up, not a blocker -- the
  taught set includes "go down", which removes the collapse.
- **No cliff under uniform partial compliance.** Applying every call with probability q gives a smooth, concave curve:
  half the calls bring 60% of the gain. The cliff the evidence review found is real for *subsets of rules* (dig + home
  without down), not for partial obedience to all three -- so a partly copied brain should land on this curve, and its
  per-rule agreement on its own runs reads off where.
- **The choice floor costs little at saturation and a lot short of it.** The strict floor leaks 1.5 wall digs a world
  at scores of +-1 (20.34) and 6.9 at +-0.9 (18.23; the method review estimated 6.7 at that level -- its mechanism is
  right). How far the copy can push its scores, against `squash`'s slow saturation and sharpness at its clamp of 4, is
  therefore a first-class quantity for the copier to report.
- **The bar moves**: from half the ceiling's gain (13.7) to half the *reachable* gain at scores of +-0.9 -- **12.5 full
  loads** -- read beside the compliance curve.
