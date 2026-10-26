# The ant brain's design, reviewed: why it was built this way, what the work since showed, and whether it can combine two senses

*Brain-design review thread, 2026-10-26, at the owner's request (16:10Z: "review our original brain design ... then
adversarially review your new reported issue and suggestion and come up with a plan"). Reviews
[`brain-cannot-combine-senses-2026-10-26.md`](brain-cannot-combine-senses-2026-10-26.md) (the Teacher lane, PR #932).
**Status: review and plan; the plan's step 1 is an owner decision.** Experiments are offline fits and mutation searches
on two freshly logged worlds of colony 13, plus a world check; scripts and logs in §8. Pictures: [results page](https://claude.ai/artifact/WNBXM8rgXW7M5iovFkXoK5).*

## 1. In plain words

**The Teacher lane is right that the brain is bad at "when X, prefer steps that do Y", and right that a gate is the
fix. Two of its reasons are wrong, and the gate has to be built differently from how it proposes.**

- **What is actually wrong.** In today's step scorer a unit that should be *off* (a fed ant: ignore home) cannot be
  silent. It sits at its saturated level and adds a constant that shifts every option and the job choice. So any unit
  taught "go home when hungry" also changes what fed ants do. In the copies below, today's brain reaches high agreement
  with the come-home teacher only by rewriting everything else (agreement 0.88-0.91, but the fed ants' choices move by
  0.67-0.83 nats); held to the fed ants' old choices, it learns nothing (0.30, from 0.28). A gated unit is truly off at
  gate 0, so it learns the lesson from plain senses and leaves the rest alone: **0.87 agreement with the fed ants
  barely moved (0.04)**, from a hand-laid start; and a mutation search from nothing finds half of it (0.52 at 0.09)
  where the same search on today's brain finds nothing (0.29).
- **Saturation is not the cause.** It is real (bred in: the sharpness gene sits past its cap), but a gated unit works
  inside the same saturated colony 13, and lowering the cap did not help (nest9).
- **"A gradient fit on the existing weights plateaued" was the fit, not the brain.** The same unit and slots reach
  0.91 with a ten-times larger step. What a fit cannot do is reach the lesson *without* leaking into fed ants.
- **The home-low comparison is circular.** In these logs the "home, when low" input is non-zero on 98% of the decisions
  the teacher labels and on 0.02% of the rest: it *is* the teacher's rule (the same fuel constant), so a brain that
  reads it is reading the answer. It proves nothing about combining senses, which is the owner's objection in numbers.
- **The gate must be a clamp, not `1 + tanh`.** The brain may not use `tanh` (determinism). The legal smooth version,
  `1 + squash`, needs a pre-activation of 9 to close to a tenth, against 0.9 for `clamp(1 + y, 0, 2)`.

**The plan (§7):** build the clamp gate as layout 6, inert at zero and on by default; teach come-home into it from
plain senses and check it in the world against the home-low instinct; then let selection try with the gate slots open;
retire `PP_HOMELOW` and `PP_SOILWAY` only when a learned version matches them.

**In the world** (teacher off, far food that keeps moving, four worlds): the gated unit, reading only energy, the far
sense and the home bearing, starved **107** ants against **177** for the home-low instinct and **1,107** for colony 13,
with more food home. The same weights with the gate taken out starve every founder by frame 8,000.

## 2. Why the brain is built the way it is

The design of record is [`design/general-brain-design-rev3.md`](../design/general-brain-design-rev3.md) (2026-10-17).
Each choice that matters here, with the reason the design gives:

| Choice | Why it was made | Where |
|---|---|---|
| **Score options, then pick** (the body lists every legal step, cut, put-down; one shared scorer rates each) | Hands *which cell* to the brain instead of hand rules; one rule learned facing left works facing right; works for any number of options; drawable as a rose | rev 3 *How it decides* |
| **Context folded into every scorer unit once a decision**, then the same 16 units run on each option's features | Cost: the per-option part is only the features. This makes the scorer a one-hidden-layer network over (context, features), so context *can* change which option wins, through the units' bend | rev 3 step 3 |
| **A job head picks the verb** from context, memory, each verb's best target and a pull to the last verb | "Hungry, so eat" is one weight; commitment stops flicker (Creatures, Polyworld, Bibites) | rev 3 *Jobs and switching* |
| **The squared pick with a floor** (`(0.05 + (s+1)/2)^2`), no argmax | Argmax killed exploration (`dead-ends.md` P-10); a big floor walked ants into the air | rev 3, `mind.rs` `PICK_FLOOR` |
| **`x / (1 + |x|)` everywhere, no exp or tanh** | Cross-platform determinism: transcendentals are the platform libm's last bit | `brain.rs`, rev 3 |
| **Evolution only, fixed size, grow only by zero-entry slots** | NEAT rejected before (`creature-direction.md` D4: unreadable, slow); learning in life deferred | rev 3 *Learning*, *Size* |
| **A price per live link** | Sparse, modular brains (Clune 2013); later found causal for starvation (`dig-rung-design.md` §11) | rev 3, #899 |
| **"Modules later"**: copy the scorer when one cannot hold the jobs | The planned answer to "does one scorer hold five jobs?" (Schrum and Miikkulainen 2014). **Never built** | rev 3 *Jobs*, *Open questions* |

**A multiplicative path was never designed in.** Rev 3 does not discuss one; the closest it comes is the module copy.
The research lane named context-dependent gating (XdG, Masse 2018) on 2026-10-21 as "a design doc first"
([`field-map-and-ranked-list-2026-10-21.md`](field-map-and-ranked-list-2026-10-21.md) row 8;
[`q3-teaching-plus-selection-2026-10-21.md`](q3-teaching-plus-selection-2026-10-21.md) §3a). Nobody wrote that doc.

**The missing architecture review.** `mind.rs` cites `review-brain-architecture.md` (E5-E8, F9). It was written by the
ladder lane on 2026-10-18 into a shared folder that no longer exists, and was never committed (no branch or commit in
the repo holds it; searched every fetched branch and `git log -S`). Its findings survive only as summaries:
[`ladder/README.md`](../ladder/README.md) row `review-brain-architecture.md`, [`ladder/dig-rung-design.md`](../ladder/dig-rung-design.md) §5,
and the `mind.rs` comments. What it found, ten days before this report:

- **F1:** the job head cannot do an AND of this moment's senses (a weighted sum). Answered by eight hidden units in the
  job head, *hand-laid* with weights of ±40 (rev 2.1 of the dig rung).
- **E5:** the best target score sits at 0.95-0.99 in every state, so a scorer unit read through `best` can veto a job but
  not raise it. This is the saturation the Teacher lane measured again on 10-26.
- **F3:** in 79 toy runs, no weight grew from zero past about 9 under mutation; the gates need 40s. Hence "lay the
  units by hand; do not leave them to evolve".
- **F9:** the genome's length is its version only while no two layouts share one.

So the family of limits in the Teacher lane's report was found on 10-18 and answered *locally* each time, never in the
step scorer.

## 3. What the work since shows

The same limit has been routed around five times, each time with a hand-built part:

| When | Symptom | Workaround |
|---|---|---|
| 10-18 | Drop must mean "food at the nest yes, soil no": an XOR | Hand-laid job-head hidden units (±40 weights) |
| 10-22 | Every Dig option scores 0.97-0.99, so the cell choice is at chance | A separate evolvable dig head that re-ranks cuts (`PP_DIGHEAD`) |
| 10-22 | "Far from home" too blurry to gate a lesson | Borrowed context slots carrying a pre-made flag (`PP_FAR_CTX`, `PP_BOTTOM_CTX`) |
| 10-25 | Recall of a remembered place | A scripted tilt applied after the pick (`recall.rs`) |
| 10-26 | "Hungry and far, so go home"; "carrying soil, so go out" | `PP_HOMELOW`, `PP_SOILWAY`: instincts presented as senses |

What the copy work found, in order (all in [`teacher-training/`](../teacher-training/)):

- **10-21, step 0.5** ([`step05-why-copies-fail-2026-10-21.md`](../teacher-training/step05-why-copies-fail-2026-10-21.md)):
  with *every slot free*, today's brain copied a context-gated walk-home lesson to **0.87** held-out agreement while
  keeping 0.95 of its other skills, and the dig lesson to 0.96. It failed in the world for other reasons: the lesson
  spread to look-alike states, offline agreement was not performance, and 1,874 live links ran the ant dry.
- **10-26** (colony-selection log): the come-home copy onto colony 13's own links plateaued at 0.40-0.63, with the hold
  doing most of the limiting; the same lesson onto the pre-combined home-low input reached 0.79 with 16 weights.
- Selection never grew "go home when low" in ~10 batches, but kept it once given (nest10). Lowering the sharpness
  ceiling (nest9) did not help selection or the copy: **pinned scores were not the binding limit**.

## 4. The report's claims, tested

| The report's claim | Verdict | Evidence |
|---|---|---|
| Context can change which step wins only through the units' bend | **True, but that is how any one-layer network combines inputs**, and rev 3 meant it so. The real defect is narrower: an additive unit cannot be *off* without adding a constant (±its output weight) to every option | §5, the hand-laid ungated start leaks 0.53 nats where the hand-laid gate leaks 0.017 |
| Saturation shuts the path (75% of sums flat; hunger moves no slope) | **Measured right, wrong as the cause.** The pool is saturated because the sharpness gene sits at 6.7 against a cap of 4 (neutral to mutation) and output weights reach ±31; a fresh unit inside the same pool learns the lesson when gated | §5; `c13/m00` gene 9 = 6.73, outputs −30.8 … +12.6 |
| Hidden units feed only the job head | **True, and correctly seen as no help**: added to the scorer they would be one more additive term, the same as context | §2 |
| Zero weights rarely come alive | True (corrected in the report itself), and secondary | report §2 |
| A gradient fit on the existing weights also plateaued (0.40-0.46) | **Wrong as an architecture claim.** Same unit 8, same 80 slots: 0.60 at the report's step size and 0.91 at ten times it. The plateau is where the fit stops paying for leaks | §5 rows 2-3 |
| Home-low's 0.79 shows the brain needs the interaction pre-built | **Circular**: home-low is non-zero on 70,984 of 72,485 labelled decisions and 5 of 31,792 unlabelled ones; it carries the label | §5 |
| Lowering sharpness does not help | Agreed (nest9), and consistent with saturation not being the cause | log 2026-10-26 |
| Fix: `gate = 1 + tanh(...)` multiplying the unit's feature sum | **Right shape, illegal form.** No transcendental may enter a decision; `1 + squash` is legal but needs ten times the weight; `clamp(1 + y, 0, 2)` is legal and closes at y = −1 | §5, weight table |
| Test step 1: the ungated fit "must stay at ~0.45" as the positive control | **Confounded**: the ungated fit's agreement runs from 0.30 to 0.91 with the hold weight and step size alone. Compare at a matched hold, and report agreement with the hold's KL every time | §5 |
| Test step 1 by a gradient fit from zero | **A clamp gate cannot be tested that way**: from all-zero, both gated and ungated gradient fits stay at 0.30 under the strong hold (the gate runs to its edge and loses its slope). Use a hand-laid start or a mutation search | §5 rows 8-9 |

## 5. Experiments

**Setup.** Colony 13 (`assets/minds/std_pool.txt`, every member's weights on option feature 18 zeroed, which is
inert with `PP_LASTFOOD=off`) on two worlds (seeds 32007, 33007; 150,000 frames, `patches=20000:40:120`,
`PP_BED_W=448`), with the come-home teacher labelling only (`PP_CARRY=0`, `PP_HOMEFAR_HUNGRY=1`,
`PP_HOMEFAR_FUEL=0.012`, the Teacher lane's settings) and `PP_HOMELOW=0.012` written into the logs for the reference
arm (`PP_COPY_SAMPLE=40:4`): 123,981 labelled and 70,066 unlabelled decisions. Each arm moves only the named slots of
member 0, scored on 20,000 of each by `examples/homefit.rs` on the game's own `decide`: P(label) (the brain's chance of
picking a labelled option) and the hold, KL from the start genome's choices on unlabelled decisions. **P(label's verb)
was 0.96 in every arm, so the job head is not the limit; the step choice is.** Start: P(label) 0.28.

"Fresh" zeroes scorer unit 8 (context, feature and **verb** rows, output; its output weight is −0.64, so removing it
costs a hold of 0.017). Ungated arms train its context row, feature row and output (80 slots); gated arms its feature
row, output and a 35-slot gate row (`mind::GATE`, scratch, read only by `homefit`). Feature 18 is never trained in
the plain-senses arms. Mutation search (`es=`): 8,000 children, one slot moved by the in-run step (sd 1) each, kept if
the loss on a fixed 3,000 + 3,000 rows is no worse: a proxy for "can mutation plus selection find it", free of any
step budget but also free of the world's noise. Hold weight 1 (the Teacher lane's default) or 10.

| # | Arm | Search | Hold | P(label) | Hold KL | Largest weight |
|---|---|---|---|---|---|---|
| 1 | Home-low instinct, 16 weights (reference) | gradient / mutation | 1 / 10 | 0.76 / 0.83 | 0.000 | 4.8 / 40 |
| 2 | Today's unit 8, the report's step (lr 0.05) | gradient | 1 | 0.60 | 0.57 | 10.5 |
| 3 | Today's unit 8, lr 0.5 | gradient | 1 | **0.91** | 0.83 | 47.8 |
| 4 | Today's unit 8 | gradient / mutation | 10 | 0.30 / 0.29 | 0.005 | 22.7 / 40 |
| 5 | Today's unit 8 | mutation | 1 | 0.61-0.63 | 0.69-0.76 | 37-40 |
| 6 | Fresh ungated unit | gradient / mutation | 1 | 0.88 / 0.82 | 0.67 / 0.87 | 54 / 40 |
| 7 | Fresh ungated unit | gradient / mutation | 10 | 0.30 / 0.29 | 0.007 / 0.005 | 46 / 40 |
| 8 | **Fresh gated unit (clamp)** | gradient / mutation | 1 | 0.70 / 0.87 | **0.044** / 0.33 | 38 / 40 |
| 9 | **Fresh gated unit (clamp)** | gradient / mutation | 10 | 0.30 / **0.52** | 0.013 / 0.086 | 20 / 40 |
| 10 | Hand-laid gate, no search | — | — | 0.44 | 0.017 | 22 |
| 11 | **Hand-laid gate, then mutation** | mutation | 10 | **0.87** | **0.043** | 40 |
| 12 | Hand-laid ungated unit (same rule, ×10 on its context row), then mutation | mutation | 10 | 0.72 | 0.53 | 40 |

The hand-laid gate (rows 10-11) is a least-squares fit of the gate's linear part, from this decision's context and
memory, onto the teacher's own fuel rule `clamp(1 − e/(1 + 0.012·dx), 0, 1)`: it reads 0.002 on average where the rule
is shut (above 0.1 on 0.7% of those decisions), with the unit's `f::HOME` weight 3 and output 20. **Is the rule's
condition in the senses at all?** A linear probe on this decision's context and memory separates the teacher's
decisions from the rest at AUC 0.999 (energy alone 0.986): the brain is not missing a sense, it is missing a way to use
one as a switch.

**Reading.** Down any column the pattern holds: under a hold that keeps fed ants as they were, nothing ungated moves
(rows 4, 7), whatever the search; under a light hold the ungated brain reaches the teacher only by changing fed ants'
choices by 0.6-0.9 nats (rows 3, 5, 6). The gate is the only plain-senses arm that reaches the teacher while leaving the
rest nearly alone (rows 8, 11), and the only one a blind mutation search finds anything of under the strong hold
(row 9). Row 12 is the mechanism: the same rule hand-laid into an ungated unit starts at a hold of 0.53, because a shut
unit sits at its saturated level, not at zero.

**Why a clamp.** The pre-activation a gate needs to shut to a tenth or a thirtieth of open, from open at 0:

| Shut to | Additive softsign unit (shift) | Gate `1 + squash(y)` | Gate `1 + tanh(y)` (not allowed) | Gate `clamp(1 + y, 0, 2)` |
|---|---|---|---|---|
| 1/3 | 0.73 | 2.0 | 0.80 | 0.67 |
| 1/10 | 2.16 | 9.0 | 1.47 | 0.90 |
| 1/30 | 4.48 | 29 | 2.04 | 0.97 |

The additive unit's shift looks cheap, but it is a shift of the unit's whole sum, so a shut unit still adds its output
weight to every option (row 12). The squash gate is legal and worse than useless at closing; the clamp shuts exactly.
A clamp's flat edges stop a gradient fit (rows 8-9, gradient, strong hold), which is why it is tested here by
mutation search and a hand-laid start, as the brain trains.

**World check** (teacher off, far moving food, 200,000 frames, worlds 1007-4007, the 30 members each given the fitted
unit 8, the gate rows read from `PP_GATE_FILE`):

| Pool | Starved, worlds 1007 / 2007 / 3007 / 4007 | Starved, total | Food home, total | Alive at the end |
|---|---|---|---|---|
| Colony 13 (`std_pool`), no instinct | 175 / 301 / 305 / 326 | 1,107 | 14,520 | 79 / 48 / 47 / 46 |
| Home-low pool with its instinct (`PP_HOMELOW=0.006`, PR #928) | 81 / 22 / 52 / 22 | 177 | 15,325 | 75 / 77 / 69 / 78 |
| **Gated unit from plain senses, hand-laid start + search (row 11)** | **13 / 20 / 48 / 26** | **107** | **16,138** | 77 / 77 / 75 / 74 |
| Gated unit, search from zero (row 9's arm at hold 10) | 111 / 139 / 216 / 225 | 691 | 14,151 | 73 / 73 / 71 / 55 |
| Control: row 11's pool with its gates removed (gate = 1) | 24 / 24 / 24 / 24, every founder, by frame 8,000 | 96 | 0 | 0 / 0 / 0 / 0 |

**The gated unit, reading only plain senses, starves fewer ants than the home-low instinct in total (107 against
177) and on three of four worlds (4007: 26 against 22), and brings more food home on three of four (4007 again the
exception).** Four worlds is a small sample and the two arms' spreads overlap; what it shows is that the gated
unit matches the instinct, not that it beats it. The search-from-zero gate, which matched half the
lesson offline, halves colony 13's starvation but stays well short of the instinct. The control removes only the
gate rows from the winning pool: without them the same unit pulls every ant toward home whatever its fuel, and the
colony is dead by frame 8,000 on all four worlds. That is the leak of §1 in the world, and the positive control that
the gate file was read at all.

**Caveats.** One lesson (come-home), one unit (8), two logged worlds, member 0's fit copied to the pool; offline
agreement is not performance (step 0.5 showed that twice); the mutation search judges on fixed rows, without the
world's noise, so it is an upper bound on what in-run selection can see. The winning gate's start was fitted to the teacher's own fuel rule, so it is a taught lesson, as every
copy is, not one evolution found; from zero, the search found half of it (691 starved). The world check gives all
30 members one shared gate file, a scratch shortcut: in layout 6 each genome carries its own gate rows.

## 6. The report's six questions, answered

1. **Why option scoring with an additive scorer, and hidden units on the job head only? Was a gated path
   considered?** Option scoring hands target choice to the brain and shares one rule across directions; context goes
   into every scorer unit so the scorer is a one-layer network over context and features, the cheapest shape that can
   combine them (§2). A multiplicative path was never designed: rev 3 does not discuss one, the 10-18 architecture
   review found the AND limit in the *job head* and fixed it there with hand-laid hidden units, and the research lane
   named context-dependent gating on 10-21 as "a design doc first" that nobody wrote. `review-brain-architecture.md`
   was never committed; its findings survive only in summaries (§2).
2. **Is saturation architectural or bred?** Bred. The forager seed set the sharpness gene to 3 (cap 4); colony 13
   reads 6.73, past the cap, where mutation cannot see it, and its output weights reach ±31. A fresh unit is not
   saturated. But saturation is not the binding limit: a gated fresh unit learns the lesson inside this same
   saturated pool (§5 rows 8-11), and nest9 found lowering the cap no help. Worth fixing anyway, cheaply: clamp the
   gene itself to its range so a mutation can bring it back.
3. **Does anything shipped depend on context reaching the scorer only additively?** No. At all-zero gate weights the
   gate is exactly 1, and the scratch build computes nothing unless a gate is set, so every saved genome scores as now.
   The recall tilt reweights the pick after the scores; the dig head re-ranks Dig's cuts after the scores; the lay gate
   is a rider past the core; the guest seat cuts the option list before scoring. What the layout change touches:
   `GENOME_LEN` (riders are found by `GENOME_LEN + head + LAY_LEN`), `train::is_mind`, the pool loader's `layout=`
   header (`enter_last_food`), `inrun::slots()` (the gate slots must be mutable and addable), and the Python copiers
   that hardcode 2,797 (`copier_c.py` and its kin). **Append the gate block after the job head's hidden units** so every
   existing index stays put, as the hidden units were appended; version by the header (`layout=6`), not by length (F9).
4. **Is the bottleneck the search, with the architecture a red herring?** Both are real, and the architecture is not
   a red herring. Under the same mutation search and the same hold, today's unit finds nothing (0.29) and the gated
   unit finds half the lesson (0.52); from a hand-laid start the gate reaches 0.87 at a hold of 0.04 and the ungated
   unit cannot get below a hold of 0.53. The search matters too: the report's own gradient plateau was its step size
   (0.60 → 0.91), and from zero no gradient fit moves either brain under the strong hold.
5. **Frame cost and the tools.** Per decision, a gated unit's row is 35 multiply-adds, once; per option one extra
   multiply per gated unit. Against ~16 units × ~45 features × ~15 options per decision this is under 1%, and zero for
   a genome with no live gate weight (skip the block, as the hidden units are skipped). Gate weights are live links and
   are billed at the link price; a useful gate is 3-10 links against the pool's ~1,900. Measure it with `antcost` and
   `examples/ascii.rs` before it lands, as the 1.3x frame gate requires.
6. *(Not asked, found.)* **A copy's agreement means nothing without its hold.** Today's unit 8 "reaches" 0.91 by
   moving fed ants' choices 0.83 nats. Every copy result should be reported as the pair (agreement, hold KL), and an
   arm compared at a matched hold.

## 7. Plan

Each step has a pass bar, and a step that fails stops the line. Steps 1-2 are the owner's decision (§1); the rest is
the Teacher lane's, as owner of the one ant line.

| Step | What | Pass | Owner |
|---|---|---|---|
| **0** | **Correct the record**: `dead-ends.md`'s homefit entry ("re-test never on the existing links"; "the general lesson is to give a graded habit a sense that is 0 outside the state it is for") points here; the report's §2-3 claims carry the verdicts of §4 | docs only | this review (done in this PR) |
| **1** | **Build the clamp gate as layout 6**: `gate_u = clamp(1 + A_u·[context, memory, 1], 0, 2)` multiplying unit `u`'s feature sum; 16 × 35 slots appended after `HID_OUT`; header-versioned loader; skipped when a unit's gate row is all zero; slots open to mutation and to structural adds in `inrun`. **Default on** (owner rule): it is identity at zero, so "on" means present and trainable | every shipped pool byte-identical (guard test, put the fault back to see it go red); worst frame within noise in `ascii` and `antcost` | brain code: Teacher lane or a build thread |
| **2** | **Teach come-home into a gated unit from plain senses** (energy, the graded far sense, `f::HOME`), from the hand-laid start of §5 refined by mutation search, the copy applied to the pool. **First pass done here on 4 worlds** (107 starved against the instinct's 177); can run today on the scratch `PP_GATE_FILE` | 16 worlds and the owner's lab box, teacher off, far moving food: starvation within the home-low pool's spread, food home no lower, open nest no smaller, no colony lost | Teacher lane |
| **3** | **Selection with the gate open**: one batch from that pool (and one from colony 13 with gates at zero) against a matched control, 32 × 48, far moving food | starvation falling across rounds in the gated arm, judged by tracing the individuals that starved, not only the garden medians | Teacher lane |
| **4** | **Retire the instincts**: `PP_HOMELOW` off for good; `PP_SOILWAY` replaced by a plain always-on "way to the door along the nest" feature on every Walk, for the gate to switch on with `c::SPOIL` | the learned version matches the instinct's 16-world numbers (open nest, starved) | Teacher lane |
| **5** | **Use the gate for the open problems** that hit the same wall: "nest too small, so dig" (the Dig job), the dig head's cell choice, the job head's hand-laid hidden units | each by its own lesson and world check | later |

**Rules this review adds** (to `Reports/how-we-test.md` and project memory):

- **Before calling an input a sense, measure its overlap with the teacher's labels.** An input that is non-zero only
  where the teacher fires is the label, not a sense (home-low: 98% of labelled decisions, 0.02% of the rest).
- **Report every copy as (agreement, hold KL) and compare arms at a matched hold.** Agreement alone ranked a fit that
  rewrote fed ants' choices (0.91 at 0.83 nats) above one that left them alone.
- **A "fresh" scorer unit is zeroed in all three of its rows**, context, feature *and* verb: a verb row left live is a
  per-verb constant its output weight carries into every score (it cost this review one full set of arms).
- **Test a clamp gate by mutation search or from a hand-laid start, not by a gradient fit from zero**: at its flat
  edges it has no slope.
- **No transcendental in a decision applies to new brain parts too**: a proposal written with `tanh` or `exp` must be
  re-derived in a legal form before it is costed.

## 8. Reproducing

The scripts, the hand-laid gate start (`init_gate.txt`), the winning fit of unit 8 (`unit8_gated_fit.raw.txt`) and
its gate rows (`gate_GI.txt`, for `PP_GATE_FILE`) are in [`brain-design-review-2026-10-26/`](brain-design-review-2026-10-26/);
the two copy logs (~1 GB each) are not kept. `split.py` turns `assets/minds/std_pool.txt` into one sparse file per
member, `mkarm.py FIT OUT [GATE]` writes the 30-member pool with unit 8 replaced, `gateinit.py` least-squares the
gate onto the teacher's fuel rule, `probe.py` is the linear probe. To rebuild:

- **Logs**: `examples/inrun founders=<colony 13's 30 members, feature 18 zeroed>:1,... seed=32007 frames=150000
  patches=20000:40:120 sel=off` with `PP_LASTFOOD=off PP_HOMELOW=0.012 PP_CARRY=0 PP_CARRY_HUNGRY=2
  PP_CARRY_HUNGRYONLY=1 PP_CARRY_OUT=0 PP_CARRY_STORE=1 PP_HOMEFAR=1 PP_HOMEFAR_HUNGRY=1 PP_HOMEFAR_FUEL=0.012
  PP_BED_W=448 PP_COPY_LOG=... PP_COPY_SAMPLE=40:4`; again at 33007.
- **Fits** (`examples/homefit.rs`, this PR's options): `unit=8 rows=all|feat drop=18 [fresh=1] [gate=1] [init=FILE]
  [es=8000 batch=3000] [lr=0.5] [wkeep=10]`; `PP_GATE_FORM=squash` for the squash gate. Row numbers in §5 map to
  these flags one to one.
- **World check**: the fitted unit 8 rows copied into every member, `PP_GATE_FILE` for the gate rows (scratch, read by
  `mind::decide` only when set), worlds 1007-4007, teacher off.

The scratch code (`mind::GATE`, `PP_GATE_FILE`, `PP_GATE_FORM`, homefit's `rows`/`fresh`/`gate`/`init`/`es`) is
inert unless set: the game and training never read it.
