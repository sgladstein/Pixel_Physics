# Adversarial review, CODE vs DESIGN: step 1 copy design (2026-10-21)

Design: `Reports/brain-training/teacher-training/step1-copy-design-2026-10-21.md`. Code: worktree `/home/user/tp`
(branch `scratch/teacher-probes`, teacher uncommitted on top of `920daf21`), `origin/main` for `protect`. s161 gen 336 =
`/home/user/m1/parent.txt`; logs `/home/user/m2/*.log`, `/home/user/m3/*.log`. Nothing under either tree was edited.

## Verdict

The design's statement of the choice rule is right, and so is its description of the scratch teacher's three rules:
the code does what the design's table says, the scored settle is safe, and the counters are wired correctly. **The copier
cannot be built as written, for two reasons.** (1) The "same Rust code, in label-only mode" cannot produce a label that
does not depend on the brain's pick. `teach` and `walk_home` return `Keep` both when the teacher has nothing to say and
when the brain already did the right thing, and the loss table puts losses only on disagreements and sends everything
else to the anchor. So the KL anchor pulls the very states the lesson is about back to s161, and the pull gets stronger
as the student improves. Under DAgger the dig-force lesson settles near P(Dig | foot in reach) = 0.56 instead of 1, and
the veto lesson does not move s161's 1.1% wall-dig rate at all (arithmetic in B1). (2) The decision log leaves out the
TurnAround and Rest options. TurnAround's job reads its best target with weight +2.0 in s161, so once the scorer
changes, the Python forward pass cannot compute P(verb). Its job-score reproduction only passes because it reads the
logged `best`. Three MAJOR problems follow. The shuffled-labels control is the cross-world move that section 1a itself
showed is invalid. In the ceiling run, a forced dig leaves the brain's memory recording the verb the brain itself
picked, so 20.54 is a ceiling for an ant whose memory does not match its actions. And freezing `MEM` does not freeze
memory, because the trainable scorer feeds memory through `all_best`/`all_mean` at every decision. Each fix is small and
local. None of them changes the ceiling arms already measured, except M2, which needs one 2-minute rerun.

---

## BLOCKER

### B1. Label-only mode built on `teach`/`walk_home` produces labels that depend on the brain's pick; agreement reads as abstention, and the anchor then cancels the lesson

**Evidence.**
- `mind.rs:3605` `Some(i) if !(chosen.verb == Verb::Dig && foot(&chosen)) => Force(i)`. When a foot cell is in reach and
  the brain *did* pick a foot dig, control falls through to `Keep`.
- `mind.rs:3609` `None if chosen.verb == Verb::Dig => Veto`. When no foot cell is in reach and the brain did *not* pick
  Dig, the result is `Keep`.
- `mind.rs:3653` `walk_home` returns `None` unless the brain's pick is a Walk. `:3676` (`dd > depth(pick)`) and `:3691`
  (`d(best) < d(pick)`) return `None` when the brain already took a labelled step.
- Design 1b says it "computes its call at every decision and writes it ... beside the brain's own pick"; section 2's
  table defines losses only for force, veto and down/home *corrections*; "teacher abstains -> anchor KL(p_s161 || p_copy)".

**What is wrong.** The teacher's *policy* is a function of the state: Dig at a foot cell whenever one is in reach (empty,
not shut in); never Dig when none is; given Walk, a step in the deepest set (below ground) or the nearest-to-mouth set
(on the surface). The *code* only reports where the brain's pick departs from that policy. Logged as designed, a decision
where the brain happened to sample correctly is indistinguishable from a decision where the teacher abstains. Both go to
the anchor, which pulls them back to s161's own distribution on exactly the states the lesson is about. Down/home labels
are also never computed on decisions where the brain sampled a non-Walk verb, although the target distribution given Walk
is defined there.

**Consequence (arithmetic, anchor weight 1).**
- **Dig force.** s161 digs at a foot cell about 20% of the time when one is in reach (measurement 2). With a student
  that samples Dig at rate p_s, the expected loss is (1-p_s)(-log p) + p_s * KL(0.2 || p). This is minimised at
  p = 1 - 0.8 p_s. Round 0 (p_s = 0.2) gives 0.84. The DAgger fixed point p = p_s is **0.56**. The rounds then move
  *down* from round 0, so section 4's stop rule ("gain under one s.e.") ends the run after round 1 and reads the result as
  saturation.
- **Dig veto.** s161 samples Dig in 1.1% of the decisions with no foot cell in reach. Only those decisions get the veto
  loss; the other 98.9% are anchored at s161's own P(Dig) = 0.011. The optimum is p ~ 0.0109: **the veto lesson
  effectively vanishes.** It is the half of the dig rule that removes the wall cuts (65% of s161's digs).
- A lighter anchor softens this but does not remove it, and the anchor-weight sweep (section 6) would be choosing between
  the lesson and itself.

**Fix.** Split each rule into a pure *state label* function and use it for both modes:
- `foot_set(opts)`, plus a `dig_label` that is one of: must-dig / must-not-dig / abstain (shut in, carrying, no Dig
  offered).
- `down_set(opts, at)` and `home_set(opts, at)` over Walk options, computed whenever Walk is offered and the
  precondition holds, *regardless of the sampled verb*.

The acting mode becomes "act if the pick is outside the set". The label-only mode logs the sets. Losses then apply to
every decision whose rule applies: -log P(Dig) - log P(foot set | Dig); -log(1 - P(Dig)); -log P(set | Walk). The anchor
covers only decisions where no rule applies.

### B2. The decision log omits options the verb stage depends on, so P(verb) cannot be recomputed for any genome but s161

**Evidence.**
- `mind.rs:3755` logs only `Verb::Walk | Verb::Dig | Verb::Drop` options. TurnAround (target: the tail), Rest (the head),
  and PickUp/Eat/Feed are left out.
- In `decide` (`mind.rs:1558`, `:1575ff`) each present verb's job reads `best[v]` over *its own* options, and P(verb) is
  normalised over all present verbs (`:1599-1601`).
- s161: TurnAround's job weight on `best` is **+1.997** (Dig -0.311, Drop -0.439, Rest 0).
- `brainmath.py` reproduces job scores from the *logged* `best` vector, not from options. That is why its reproduction
  passes.

**Consequence.** Any change to `S_CTX`/`S_FEAT`/`S_OUT` changes TurnAround's score, so it changes P(TurnAround) and,
through the normalisation, P(Dig) and every other verb's probability. The copier cannot compute that from these logs.
The verb-stage anchor KL for later DAgger rounds cannot be computed either, and neither can memory's `all_best` and
`all_mean` (see M3). The planned forward check can still pass, because at s161's own weights the logged `best` is right.
The gradient check can pass too, because it compares the gradient only against its own forward pass.

**Fix.** Log every option (drop the filter at `:3755`). Log at full f32 precision (MINOR 1). Add a forward check on a
*perturbed* genome: `decide` is pure (doc at `:1505-1511`), so a small Rust replay of logged decisions under a random
perturbation of every trainable block, compared against the Python P(verb) and P(target), catches what the s161-only
check cannot.

---

## MAJOR

### M1. The "shuffled labels" control is the cross-world move the design itself showed is invalid for step choices

**Evidence.** Section 6: "Shuffled labels (teacher labels moved to other worlds' decisions) | no gain over s161".
Section 1a: moving labels to another world's decision "still scored 0.98-0.99 ... because in the shaft the step options
come in the same order almost every time". The run is in `/home/user/m2/probe-D4.out`: the negative control reads
0.982 / 0.993 against a floor of 0.406.

**Consequence.** For down (and partly home, where the control reads 0.49 against 0.40), moved labels still mark the
right step by its position. A copy trained on them learns much of the real lesson and shows a gain. The control then
"fails", or is read as "any training helps", with no defect in the copier.

**Fix.**
- Target labels: shuffle within each decision, among its own options (as in 1a).
- The dig verb label: permute must-dig / must-not-dig among decisions within the same world, and stratify by carrying
  and depth band so the base rate is kept.

### M2. In the ceiling run, the brain's memory does not match what was done (forced digs, veto redraws), so 20.54 is the ceiling of a slightly different ant

**Evidence.**
- `decide` updates memory from the verb *it* drew: `mind.rs:1613`, input `row[CONTEXT + 2 + verb]`, and `state.mem =
  fresh` at `:1626`.
- On `Teach::Force(i)` (`:1746-1751`) only `d.pick` and `last_verb` are changed. `mem` keeps the brain's own verb and
  `d.emits` keeps the brain's memory.
- On a veto (`:1752-1763`) memory is recomputed by the redraw, but with `all_best`/`all_mean` (`:1613`) taken over the
  *reduced* list, without the Dig options. A brain that declined Dig itself would have them over the full list.
- s161: memory unit 5 (rate 0.5) reads "Walk was chosen" at **-1.39** and `all_mean` at **+1.89**. Unit 11 reads
  `all_best` at **+2.04**. Unit 8 reads "Dig was chosen" at +0.11.
- The dig, down and home arm forced 1,694 digs and redrew 736 vetoes over 96 worlds (`/home/user/m3/dig_down_home.log`).

**Consequence.** The bar's denominator (20.54, and so the 13.7 bar) was measured on an ant that, after a forced dig,
remembers the verb it would have picked instead of the dig. The direction of the error is unknown. A copy that digs on
its own has self-consistent memory, so it is aiming at a slightly different target from the one measured.

**Fix.** On Force, restore `mem0` and recompute the memory update (and emits) with the forced verb. Factor `decide`'s
memory block into a function taking `(verb, all_best, all_mean)`. On a veto, compute `all_best`/`all_mean` over the full
list. Re-run the dig, down, home arm (about 2 minutes, `reliab`) and report both numbers. If they differ by more than
the noise, the bar moves.

### M3. Freezing `MEM`/`RATE` does not freeze memory: the trainable scorer drives memory at every decision

**Evidence.**
- Memory's input includes `row[CONTEXT] * all_best + row[CONTEXT + 1] * all_mean` (`mind.rs:1613`). Both are outputs of
  the scorer (`:1559-1566`), which is trainable (design section 3).
- s161's weights on these inputs are 1.89 (unit 5), 2.04 (unit 11) and ±0.3-0.35 (units 1, 8 and 9), from
  `parent.txt`.
- Eight of the 12 memory units have rate 0 (raw `RATE` at or below -0.5), so memory is mostly *fresh each decision*:
  squash of the previous decision's inputs.

**Consequence.** Section 8 risk 2 frames the danger as s161's memory "encoding where in the trip I am" in a way the new
behaviour invalidates. The more direct effect is mechanical. Every scorer change the copier makes, including the revived
`DEPTH` link section 3 is counting on, shifts `all_mean`/`all_best` and so the next decision's memory, which the copier
treats as a fixed observed input. Training data (memory logged under s161) and deployment (memory under the copy)
diverge from the first step. The anchor cannot see it, because it compares distributions at logged memory. DAgger
reduces it only by relabelling. The per-decision gradient points the wrong way exactly where memory carries the
dig/walk switch.

**Fix.**
- Cheap and nearly exact here: a **one-step truncated BPTT** through the fresh-memory path. With rates at 0 for 8 units,
  mem_t is a squash of (ctx, all_best, all_mean, verb, mem) at step t-1. This needs the previous decision's full option
  list (B2).
- At minimum, report how far memory drifts: copy versus s161 memory on the copy's own logged runs, per unit.

---

## MINOR

1. **Logged precision vs the "f32 rounding" forward check.**
   - Evidence: features are written `{x:.3}` (`mind.rs:3757`); ctx, mem, jobs and scores are written `{:.4}`.
   - Problem: s161's weights reach ±40 at sharpness 4, so 3-decimal inputs cannot reproduce *unsaturated* scores to f32
     rounding. The section 6 bar is unreachable, or would be loosened by eye.
   - Fix: log full precision (`{:?}` or bit patterns), or set the tolerance from the logging precision.

2. **The `O` line's pick is an index into the *unfiltered* option list.**
   - Evidence: `line(format!("O,...,{}", d.pick, ...))` writes `d.pick` against the filtered `all`.
   - Problem: once TurnAround or Rest precede the Dig/Drop entries, the index does not point into the logged list.
     `probe.py` matches by target coordinates from the `M` line, so it is unaffected; a copier that used the index would
     be wrong.
   - Fix: this disappears with B2's "log every option".

3. **Gates that kill gradients if ported literally.**
   - Evidence: `hidden_units` computes only units some job reads (`mind.rs:1492`, any `HID_OUT != 0.0`), and the job loop
     skips zero weights (`:1588`). The function is still smooth (a zero weight contributes zero), but a forward that
     mirrors the gate returns dL/dHID_OUT = 0 for unread units.
   - Detail: s161 has units 4 and 5 unread with *non-zero* `HID` rows (12 and 8 links), so they are revivable only by an
     ungated forward.
   - Detail: `brainmath.py`'s gate differs from `decide`'s. It requires `on[v]` and `abs > 1e-6`, while `decide` uses any
     verb and `!= 0`.
   - Fix: compute every hidden unit in the forward pass used for gradients, and match `decide`'s gate exactly for the
     reproduction check.

4. **"Clipped to the start's own range per block" is not "the range mutation explores".**
   - Evidence: `train::mutate` clamps every slot to ±40 (`train/mod.rs:1085`).
   - Problem: per-block start ranges are narrower (s161's `S_VERB` block tops out at +1.07, so a verb column could
     never exceed it). The `HID` block holds -60, outside mutation's range.
   - Fix: clip to ±40, which is what mutation does.

5. **Saturation makes plain gradient descent stall, and nothing in section 6 would notice.**
   - Evidence: s161 sharpness is clamped at 4 (raw `SHARP` 3.68). Scores sit at about ±0.95-0.99 (`mind.rs` `HID` doc),
     where ds/draw = 4(1-|s|)^2 ~ 0.004-0.01. Dig's job constant is -6.30, so w_Dig ~ 0.02 < k = 0.05: P(Dig) is set by
     the floor, and d log P / d sum is tiny.
   - Problem: the gradient check passes on a vanishing gradient.
   - Fix: specify the optimiser (Adam or per-block normalised steps), and add a "can it overfit 100 labelled decisions"
     control.

6. **The dig target label.**
   - The table says "Dig, at the deepest foot cell". The acting code forces the deepest (ties to the mouth's column,
     `:3596-3602`) but `Keep`s any foot dig.
   - The loss "target in foot cells given Dig" matches the `Keep` semantics, which is fine; the table wording should
     match it.

7. **Veto redraw that finds nothing** (`:1758`).
   - If `decide` on `allowed` returns `None`, `d` stays the vetoed Dig, `redrawn` stays `None`, and memory stays at
     `mem0` (not updated): the veto silently fails.
   - Unreachable for s161, since a veto requires a Walk option and Walk's switch is on (1.235). It needs an assert or a
     counter before reuse on another brain.

8. **Acting-mode logs record the brain's pick, not what was done.**
   - Evidence: `scratch_log::decision` is called before the teacher (`:1738`), and veto redraws are not logged.
   - Consequences: never train on an acting run's log. 1b's control cannot read "the calls the acting teacher made"
     from it without a new log line.
   - Also, that control is close to a tautology. Both modes run the same draws, so they agree trivially until the first
     intervention, which is the only non-`Keep` call compared per world. With B1's state labels, check every decision of
     an acting run instead: Force target in the set, a veto exactly when the label is must-not-dig, a Keep pick in the
     set whenever the rule applies. That needs no trajectory matching.

9. **The settle-exclusion check covers only 4 arms.**
   - The `no facts` counter appears only in the dig+down, dig+down+home, dig+down+home+drop and down arms' logs
     (`/home/user/m3`). The dig, home, drop, dig+home and dig+drop+home arms ran on a build without it.
   - The 20.54 arm *is* covered; the "dig+home collapses" arm is not, so it cannot be confirmed from its log that the
     teacher was off in its settle.

10. **`protect` description.**
    - It is accurate in effect: on `origin/main`, `mind::protect` removes `verb_slots(v)` (switch, job row, `HID_OUT`
      row, `S_VERB` column, memory's "v chosen" input) for `Suite::untested_verbs`, which is PickUp/Eat/Feed when soil
      is learned with fetch and trail kept.
    - Two corrections: soil *allows* those verbs (`SOIL_VERBS` includes them), they just never have a target; and
      `protect` is a no-op for a single arena (`untested_verbs` defaults to empty).
    - The Python copier must reproduce `verb_slots` itself; there is no protect list on this branch.
    - s161's Eat job constant is already 0 (the "don't eat the cargo" -10 was lost), so freezing keeps it broken.

11. **Walk-home's within-decision control is not on record yet.** `/home/user/m2/probe-D3b.out` holds only the header,
    and `probe.py ... D3` was still running during this review. The design says it "is re-run".

12. **The anchor "at both stages" is underspecified.** Use the joint KL over (verb, target): KL_verb plus
    E_{v ~ p_s161}[KL_target|v]. Otherwise the target-stage term is undefined for verbs the copy picks and s161 did not.

---

## Verified correct

- **Choice rule.**
  - `choose_weighted` (`creature.rs:492`) picks with probability (k + max(s,0))^2 / sum, and `decide` passes
    w = (j+1)/2 and (s+1)/2 with k = `PICK_FLOOR` = 0.05 (`mind.rs:250`, `:1600-1606`).
  - j and s are outputs of x/(1+|x|), so they lie in (-1,1) and w lies in (0,1): the `max(0)` clamp never binds.
  - The target stage draws only among the chosen verb's options (`:1604`). The function is C1 (squash) and piecewise
    smooth (`best` is a max).
  - `SWITCH`, `SHARP`, `PULL` and `RATE` are clamped functions, and in s161 they sit *at* their clamps: `SHARP` raw 3.68
    (clamped to 4), `PULL` raw 2.71 (clamped to 1), 8 rates at 0. Freezing them costs nothing, since their gradient is 0.
    `last_verb` and `mem` are logged (`M` line fields 9 and 11; mem is the pre-decision `mem0`).
- **The veto loss matches the redraw exactly.** The redraw keeps ctx, restores `mem0` and `last0`, and every non-Dig
  verb's `best`, hidden units and pull are unchanged, so the redraw's verb distribution is P(v)/(1-P(Dig)).
- **Block map.**
  - The scorer (`S_CTX`, `S_FEAT`, `S_VERB`, `S_OUT`) feeds both stages; the job stage sees options only through
    `best[v]`.
  - `JOB` and `HID_OUT` (with `HID` through `hid`) feed the verb stage only.
  - `MEM` and `RATE` feed the next decision only; `EMIT` feeds nothing a decision reads, except later scent senses.
  - `trainable` on this branch moves allowed switches, `SHARP`, `PULL`, `RATE`, `MEM` and the allowed `EMIT` rows, so
    freezing them in the copy is a subset of what selection moves and is consistent.
  - `DEPTH` -> scorer: weight 0 in all 8 live scorer units of s161, as stated.
- **Teacher rules against the design's table.**
  - **dig:** empty jaws only. A foot cell = ground at frame 0, never cut, `depth >= shaft`. This is identical to the
    scorer's S1 test in `soil_trip_at` (`was_ground && first cut && d >= s1_rows`, with `s1_rows = shaft` because
    `reliab` sets `foot: true`). The cut-log check is the right test, since a re-dug refilled cell earns no S1. Shut-in
    abstains before the rule.
  - **down:** empty, `depth(head) > 0`, no foot cell in the (post-veto) list, deepest step, only if deeper than here and
    than the pick.
  - **home:** surface (`depth <= 0`), at least 3 columns from the mouth, 2 or more walks, nearest to the mouth's column.
  - `walk_home` runs on the redrawn list after a veto and indexes into it (`:1765-1773`, `:1777-1778`), so the final
    act is consistent.
- **Depth and mouth.** `TeachCtx::depth` is the same code as `depth_below_ground`. The datum is uniform over the shaft's
  columns ±3 in all 24 logged worlds; steps begin at column 118 or 137. `mouth_x` = 128.5, from the founding cut's top row
  (34 cells, columns 124-132), in every logged world. The tick's (x, y) is the head ("the active site sits on the head",
  `creature.rs:7778`).
- **Drop zone.** `gap > SOIL_AWAY - 1 + margin` is equivalent to `soil_away` when margin is 0 (drop is not in the copied
  teacher).
- **TEACH thread-local.** Set after the world is built and cleared before the settle (`challenges.rs:1950`, `:1958`); each
  `reliab` world runs whole on one worker thread. In the settle, `teach` returns `Keep` and counts `no facts`, and
  `walk_home` returns early on `t.as_ref()?`. The settle ants hold the `rest` preset. No rule can act in the scored settle,
  and 36,604-36,610 no-facts decisions over 96 worlds (381 a world) matches the measurement report.
- **Counters and probe.** `COUNTS[8]` is "down forced", and `teach_report` prints it as such. `probe.py`'s labels are
  state-based and match the acting rules' set semantics, which supports the copyability numbers quoted in section 1.
