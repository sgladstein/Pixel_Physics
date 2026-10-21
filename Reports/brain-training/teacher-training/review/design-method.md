# Adversarial review: `teacher-design-2026-10-21.md`, method lens

*Reviewer's lens: imitation learning, planning, learning from teachers that see more than the student
("privileged" teachers), and whether the claims follow from the evidence. 2026-10-21. No repo file edited. Evidence
is cited as file:line in `Reports/brain-training/teacher-training/`. Papers carry a verification level: **[abstract]**
means I read the abstract via SciSpace; **[summary]** means a web-search summary only; **[recalled]** means from memory,
unverified. Consensus had used up its monthly quota and arXiv/PMLR were blocked by the proxy, so nothing below is
full-text verified. PubMed does not index this machine-learning work.*

## Verdict

**Sound architecture, wrong next step.** The four-part teacher (goal, computation, value per option, lesson), the
gauntlet, "the planner proposes, the search disposes" and the honest spill-back prediction are good, and should stay.
But three things go wrong.

- **The order of work skips the bottleneck.** The doc names the student as the bottleneck (section 7), then schedules
  three days of planner building (section 10) before any experiment on that bottleneck.
- **The new lesson form is untested and probably weakens step lessons.** It is adopted for every teacher, but on the
  doc's own numbers it would likely have weakened the one lesson that copied.
- **One decision drops a lesson.** It "replaces" walk-home with a lesson about a different decision. The empty return
  trip then has no teacher, and the nest planner later brings back the very label that broke fetch.

All three can be fixed cheaply, mostly offline on logs that already exist.

---

## BLOCKER

### B1. The order of work does not test the bottleneck the doc itself names

**Claim.** Section 7: "the student, not the teacher, limited the result ... Better teachers will hit the same wall".
Section 10 then orders the work: finish step 1, owner picks a blueprint, build the soil planner (~1 day), search
instrument, soil stages 2-7, nest planner (~2 days).

**Why it is weak.** Section 7 lists five levers against the wall. Two of them are marked "New; cheap" (the context
probe) or "Not proposed yet" (capacity) (lines 326-327), and **neither appears in section 10**. Meanwhile the evidence
that the loop itself is not working has grown since the doc was written:

- One DAgger round made the all-rules copy worse (8.64 -> 6.34, step1-results:61). One made the go-down copy worse
  too (10.57 -> 9.50, step1-results:117). DAgger is the method of retraining on the student's own states, relabelled
  by the teacher.
- The dig-only collapse is still untraced (step1-results:143, "Not yet run: ... the dig rule's failure traced to its
  cause").
- The go-down copy's fetch cost varies by copy seed, from +0.5 to -1.9 trips (step1-results:105-114).

A planner produces labels. Labels are not what is failing. Building planners now adds more lessons for the same
copier to fail on, and the first planner's result (stage 4) will be uninterpretable: if it fails, the doc cannot say
whether the planner or the copier failed.

**What I would do first: a "step 0.5"**, all offline on the step-1 logs, about a day in total.

1. **Trace the dig-only collapse individual by individual** (the `funnel` skill; CLAUDE.md "Why did it do that"). Take
   every ant in the 15 seeds under 3 and log the inputs and the chosen option at each decision from the last
   successful foot cut to the end. Until this is done, "dig is coupled to descent" is a hypothesis, not a design input.
2. **Joint-fit capacity diagnostic.** Fit the lesson plus the anchors offline, three ways:
   - (a) the trainable slots as in step 1;
   - (b) every slot unfrozen, ignoring the energy bill;
   - (c) (a), but with fetch decisions given a one-bit arena-context input.

   Report the held-out loss on lesson states and on anchor states separately. Read it this way:
   - Only (b) fits both: the bottleneck is capacity or wiring (the student).
   - Only (c) fits both: the bottleneck is context (a sense, way out 4).
   - None fits both: the labels contradict each other on states the ant cannot tell apart. That is an imitation gap,
     a *teacher* problem, and it contradicts section 7.

   Also report the cosine between the lesson's and the anchors' gradients on shared slots. This is the standard
   interference diagnostic in multi-task learning (Yu et al. 2020, PCGrad **[summary]**), and it names which slots
   are contested.
3. **The context probe** (section 7's own lever) on walk-home: empty soil-arena surface ants against empty fetch
   foragers. It is the cheapest test in the doc.
4. **Retrofit the new lesson form to the three existing rules** (see B2) before building anything that depends on it.

**Go/no-go rule to write into section 10.** If, after step 0.5, neither dig nor walk-home copies (gain over 2 s.e.,
kept skills at floor), do not build planners. Switch to one of:

- copy agreement as a fitness term (T4's second mode);
- selection from the go-down copy with protected links;
- the brain lane's capacity change, if diagnostic 2(b) says so.

### B2. "One lesson form for every teacher" would probably have weakened the one lesson that copied

**Claim.** Section 1: the planner's cost becomes an advantage in full-load units; "tau is then chosen on the selection
worlds from {0.03, 0.1, 0.3}, as for search". Here tau is the temperature that sets how far the target may move from
the brain's own odds.

**Why it is wrong as stated.** The tau grid was set for search's big calls, worth ~0.3-0.5 full loads
(teacher-design:337). A step is worth ~0.02 loads (search design: "~0.02 at stake"; decisions ~5 frames apart,
search-teacher-design:301). The target multiplies an option's odds by exp(A/tau):

| tau | A step worth +0.02 loads gets its odds multiplied by |
|---|---|
| 0.3 | 1.07 |
| 0.1 | 1.22 |
| 0.03 | 1.95 |

The go-down lesson that copied used a **hard** target (the deepest steps, uniformly; step1-copy-design:99). s161 takes
the deepest step about 0.6 of the time (0.50-0.73), and the lesson pushed that to 1.0. Under the new form at tau =
0.1, with two or three step options, the target is ~0.65. That is roughly a tenth of the push that worked. Section 11
already lists "planner labels for steps are too weak to matter" as a risk. On the doc's own numbers it is the
expected outcome, not a risk.

**Change.**

- **Re-express go-down in the new form**: A = path saved to the shaft's foot x the exchange rate. Copy it at each tau.
  **The new form is adopted for steps only if it reproduces at least ~+3 loads.** This costs about one CPU-hour on
  existing logs.
- Either choose tau **per decision type**, or divide A by the decision's stake before the exponent. Weighting the
  loss by stake (search design 1.5 item 5) sets how much a decision counts. It does not set how sharp its target is.
- **Name the objective tau is chosen by**: full loads, subject to fetch and trail at the floor. Otherwise the smallest
  tau wins on soil and the fetch loss is found later.

### B3. Walk-home is "replaced" by a lesson about a different decision, and the nest planner brings the old one back

**Claim.** Section 2, the walk-home row: "Replace with the soil planner's carry path, which is a walk home *only while
loaded* ... The planner never labels an empty ant with 'go home'".

**Why it is wrong.**

- Walk-home labels **empty** ants on the surface, 3+ columns from the mouth, with the steps toward the mouth
  (step1-copy-design:42). That is the return trip to dig again.
- A loaded ant walks *away* from the mouth, out to the ring. The soil planner teaches that outward carry, and "Empty
  ants get no label from this planner" (line 159).
- So walk-home's lesson is **dropped, not replaced**. It is not a small one: +5.40 loads acting on s171
  (step1-results:129), and about +3.6 of the s161 ceiling (16.9 to 20.5, guidelines:94-101).
- The nest planner then labels empty ants' step options with "path length saved to the ant's assigned frontier cell"
  (line 251). For an empty ant on the surface, that frontier is down the shaft, through the mouth. That is walk-home
  again, with the same look-alike (the empty fetch forager) that broke fetch. The fetch problem has been moved to step
  5, not solved.

**Change.**

- Say in each arena who teaches the empty return trip.
- Run the context probe on that decision: empty soil-arena surface ant against empty fetch forager.
- Run the soil planner's stage 1 twice, with and without the walk-home rule acting. The difference is the price of
  dropping it.
- If the context probe fails, the honest options are a context sense (way out 4) or leaving the return trip to
  selection (step1-results' option c). A planner relabel does neither.

---

## MAJOR

### M1. Tau-regularisation does not address the mechanism behind step 1's lesson spread (review question 1)

**Claim.** Section 1, bullet 2: step 1's two failures "were both the lesson moving the brain too far where it was not
taught ... Coaching (He et al. 2012) and the regularised target are the literature's answer to exactly this."

**Why it is weak.**

- The regularised target limits how far the target moves **on the states that carry a label**. Spread is something
  else: shared weights generalise from the labelled states to *other* states that look alike.
- Step 1 already applied the strongest possible pull on those look-alike states: target = s161 itself on every
  anchor, at 10x weight (step1-results:68-72). The lesson still spread.
- A softer target shrinks the gradient that causes spread and the gradient that carries the lesson by about the same
  factor. You move along the same trade-off curve; you do not move the curve.
- Coaching (He, Daumé & Eisner 2012, via repo notes) chooses *which* oracle action to teach, the one closest to what
  the learner can do. It is not a temperature on a fixed lesson.

**Change.** Run the tau retrofit from B2 on walk-home and dig, and plot soil gain against fetch loss for each tau.
Plot the same curve for the knob step 1 already has, the per-group loss weight (round 0 vs 0b). **If the two curves
coincide, tau buys nothing against spread**: say so, and rest the anti-spread case on the context probe and the
capacity diagnostic.

**Citation.** The exp(A/tau) form is the MPO / AWR / REPS family of policy updates (Abdolmaleki et al. 2018; Peng et
al. 2019 **[recalled]**). As I recall it, Grill et al. 2020's exact solution is π̄ ∝ λ·π_θ/(α − q), from the other KL
direction **[recalled; abstract-level confirmation of the paper only]**. That form has heavier tails and never drives
an option to zero. Cite the family you actually use, or check Grill's section 5.

### M2. The exchange-rate conversion is biased in four named ways (review question 1)

**Claim.** Section 1: "one exchange rate per planner, measured, not chosen: for the soil planner, the full loads s161
completes per frame of carrying".

1. **Rate of whom?** s161 makes 6.8 loads in a run, the teacher-driven ant 20.5. The value of a frame triples as the
   student improves, so a rate fixed from s161 under-values time saved in later rounds. The search design rebuilds
   its value table every round for exactly this reason (search-teacher-design:323). Do the same here.
2. **Per what?** "Per frame of carrying" can mean loads ÷ total frames, or loads ÷ frames spent carrying (~245 of
   ~1,500 frames per load). The two differ about 6x, which is enough to move a step lesson across the whole tau grid.
3. **Frames, or joules?** Section 8.2 charges search outcomes in loads per joule (the arena has a fixed 2,400 J
   budget). The planners charge loads per frame. If energy binds, a frame of walking and a frame of digging are worth
   different amounts. So "one lesson form" is currently two units.
4. **Nearest site, under the planner's own continuation.** Step options are valued by the path to "the nearest site"
   (4.3 item 3). That is a potential (a fixed value per place) used *alone* as the lesson. Potential-based shaping
   leaves the optimal policy unchanged only when *added to* the true reward (Ng, Harada & Russell 1999 **[summary]**).
   Used on its own, it simply asserts the goal. This is the owner's suspected bias against long carries: a step
   toward a farther site where the pellet will *stay* is penalised against a nearer site where it may slide. The
   cost-to-go also assumes the ant will then walk the shortest path. When the learner is weaker than the reference,
   roll-outs should mix the reference and the learner (LOLS, Chang et al. 2015 **[abstract]**).

**Change.**

- Value each step by min over sites of (path cost x rate − expected stick value of that site), with stick values
  cached per ring cell.
- Take the rate from the current student, per round, and say whether it is per frame or per joule.
- **Give each planner its own "B1" check**, as the search design has: on ~100 sampled step decisions, does the
  planner's ranking agree with short rollouts (H ≈ 300 plus settle) under the brain's own continuation? Better still,
  use the planner as the *terminal value* inside short rollouts (THOR; search-teacher-design:338, role 1), rather
  than as a standalone label generator for steps.

### M3. The probe evidence is misread, and the misreading carries the "student is the bottleneck" conclusion (review questions 2 and 3)

**Claim.** Section 3: "Step 1 passed it for all three rules (94-100% held out) and still failed to copy two of them,
so the probe is necessary, not sufficient." Section 7: "All three rules passed the probe; two did not copy."

**Why it is wrong.**

- The 94-100% figures come from the *best model on all option features*.
- Through **the brain's own channel**, the dig rule was 0.65 / 0.87 recall (senses + memory), and the brain's dig
  score separates a foot cell from a wall at AUC 0.613 (step1-copy-design:59, 63-65). AUC is the chance that a random
  foot case outranks a random wall case; 0.5 is no separation.
- So the brain-channel probe **predicted the dig failure**. Go-down's channel was clean (`DEPTH` on each step, 1.000),
  and it copied.
- Walk-home's channel was also clean (0.995, `HOME` on each step), yet it spread. That is a *context* failure: the ant
  cannot tell the soil arena from the fetch arena. No probe of the rule's own label can see it.

Read this way, the probes worked: one predicted the dig failure, and the context probe (not yet run) is the predicted
gap for walk-home. That is evidence for "gate on the right probes", not for "the student cannot hold lessons".

**Change.**

- Make the stage-2 gate three numbers: all-features probe, brain-channel probe, context probe.
- Give each a numeric bar. "Well above the floor" (section 9, stage 2) is not a bar.
- State that the bars are uncalibrated (three data points). Re-word section 7's headline to "two of three rules
  failed a probe we did not gate on".

### M4. "Compute as much as it likes" contradicts the capacity-gap evidence; answer to review question 6

**Claim.** "The answer, first": "the teacher should compute as much as it likes; what it must not do is ask for
distinctions the ant cannot make".

**Why it is weak.** A stronger teacher can make a worse student even when the information is available.

- In distillation, a student distilled from a larger, more accurate teacher did worse than one distilled from a
  smaller teacher. The fix was an intermediate "teacher assistant" (Mirzadeh et al. 2020 **[summary]**). The repo's
  own notes already record this (Cho & Hariharan 2019; Seita's "zone of proximal development" result;
  `notes/privileged_teachers_imitation.md:159,186,247`).
- With ~700 live weights and a 16-unit scorer shared across every verb, this student is the extreme case.

**The literature on learning from planners says the planner should adapt to the student, not merely be gated after
the fact.** This is the answer to question 6, and it changes the design.

- **Guided policy search** optimises the trajectory teacher with a penalty for diverging from the policy, then fits
  the policy to it (Levine & Koltun 2013; Montgomery & Levine 2016, mirror-descent GPS **[abstract]**). **PLATO**
  (Kahn et al. 2017, in repo notes) does the same with a model-predictive controller.
- **A2D** (Warrington et al. 2021 **[summary]**) updates the privileged expert so that it maximises *the trainee's*
  reward. It outperforms imitating a fixed expert.
- **Dual policy iteration** (Sun, Gordon, Boots & Bagnell 2018 **[abstract]**) gives the general frame: a slow
  planner improved "with guidance from the reactive policy", and the reactive policy imitates it.
  Balakrishna et al. 2019 **[abstract]** extend DAgger's guarantees to a supervisor that changes during learning,
  which is this doc's plan (rules, then planners, then search).

**Change: make every planner student-aware.**

- Add −λ·log p_student(plan) to the planner's cost, so that among near-equal plans (frontier cells, ring sites,
  routes) it picks the one the brain already rates highest.
- Cheapest form: break ties within ε of optimal by the brain's own score.
- The probe stays as a check. It stops being the only filter between a planner and the copier.

### M5. Way out 3 is hand-writing rules again, and it makes the gate circular (review question 3)

**Claim.** Section 3, way out 3: "A goal built from the rules real ants follow is copyable by construction."

**Why it is weak.**

- If the goal is the output of local rules, the probe passes because the goal was chosen to pass it. A gate that
  cannot fail is not a gate (CLAUDE.md, "Ask what your number counts when nothing is wrong").
- Where the planner and the generating local rule agree, the planner is redundant: copy the rule.
- Where they disagree, the disagreement is by definition the part local senses may not carry, so it may be
  uncopyable.

So yes, it is the hand teacher again, with a planner as decoration. It also quietly narrows the owner's blueprint
choice to the shapes local rules make, and the owner should be told that.

**Change.** Be explicit.

- Under way out 3, the **teacher is the local rule set itself**: Khuong's template rule; Buhl et al. 2005's
  recruitment-and-density digging **[summary]**.
- The planner earns its place only on the **disagreement set**: stability trials, connectivity, "never drop a roof".
- Measure that set's size and probe it alone. If it is small, skip the nest planner and copy the rules.
- Tell the owner that picking way out 3 limits the blueprint to the local-rule family.

### M6. The colony tier: central assignment is privileged information, and the literature says local-only students fail at it (review question 4)

**Claim.** Section 6.1: a central planner assigns faces by minimum-cost assignment, and each ant copies its row.
Section 6.2: "no ant can tell which from where it stands", so sizing and stopping are taught as outcomes.

**Why it is weak.**

1. **Privileged information.** "Your face, given where the other eleven are" depends on the other ants. That breaks
   guideline T1 as written (guidelines:52-56). Section 3 amends T1 in passing ("The teacher may use anything"); the
   guidelines file is unchanged.
   - Once T1 is broken, Swamy et al.'s "latching" warning in T2 becomes live: off-policy copying with history does
     worse when the teacher acts on information the student lacks. This brain has 12 memory units.
2. **The evidence from multi-robot learning.**
   - Decentralised students imitating a central planner work when each agent *aggregates neighbours' information* (a
     graph neural network), as in Li et al. 2020 (in repo notes). Muthusamy et al. 2024 imitate the Hungarian
     assignment algorithm, then fine-tune with RL **[summary]**.
   - PRIMAL (Sartoretti et al. 2019 **[summary]**) needed RL alongside imitation of the central expert, and matched
     full-state planners only "in low obstacle-density" worlds.
   - "Relying solely on the states of immediate neighbors is insufficient to imitate a centralized control policy"
     (`notes/privileged_teachers_imitation.md:215`).
   - Narrow tunnel faces with twelve ants are the high-density case. Expect the probe to fail.
3. **"No ant can tell" contradicts the doc's own biology.**
   - Buhl et al. 2005 **[summary]** reproduce nest volume growing linearly with colony size from local feedback
     (recruitment and density).
   - Toffin et al. (section 5.1) tie budding to crowding at the wall.
   - Sizing and stopping are therefore candidates for a *local* lesson (read `CROWD` or `ROOM`) before they are
     handed to selection.
4. **Unpriced.** "Outcome" sends the hardest parts back to selection on a colony score. The guidelines priced one
   Ferrante-sized run at $40-50 of a $100 budget (guidelines:22-27). Section 6.2 should carry that number.

**Change.**

- **Pre-gate before building anything:** compute the assignment post hoc on existing 12-ant logs and probe it from
  the ant's features.
- If it fails, drop central assignment as a *teacher*. Keep it only as an option generator for the per-ant
  counterfactual search the search design already specifies: force one ant's option, let the others run, score the
  colony (SPARTA; Bertsekas). That is the same counterfactual credit assignment as COMA and difference rewards (repo
  notes report up to 10x faster learning with difference rewards).
- Try a local crowd-response teacher for sizing before outcome scoring.

### M7. DAgger has hurt twice, and the design reuses the loop without saying why

**Claim.** Implicit throughout: copy, then DAgger, then select. Section 1's target uses p_brain without saying which
brain.

**Why it matters.** Both DAgger rounds made the copy worse (B1). One plausible mechanism, testable in an hour:

- On the copy's *new* states, wherever no rule applies, the target is **s161**.
- So DAgger anchors the copy back to s161 precisely on the states where the copy's behaviour differs from s161's, and
  those are the states where s161 led it astray.

**Change.** Specify p_brain: s161, or the current student. Test an anchor-to-current-student variant against step 1's
DAgger round on the go-down copy before any planner relies on DAgger.

### M8. No program-level stop conditions; several stage controls cannot fail

- Section 11 has "Response" but no "abandon if" column. The search design has one (search-teacher-design:443-453).
  Add numbers.
- Stage 1's control, "a planner with its plan shuffled", is undefined. A ring shuffled to random radii will mostly
  land inside the 3-cell scoring zone and lose trivially. Use a **known-negative plan** instead: a ring whose crest
  sits inside the zone. Add a known-positive one: the hand rules.
- Stage 1's bar is "above the brain alone". The soil planner replaces part of a teacher that already scores 20.66.
  The bar should be **non-inferior to what it replaces**: soil planner + dig + down, acting, within 2 s.e. of dig +
  down + home.
- Stages 3 and 5 have no pass bar. Stage 2's bar is not numeric (M3).

---

## MINOR

1. **Stale against step1-results.** Several of the doc's numbers predate the latest results:
   - "kept skills held" (line 82) should now read "fetch roughly held, at a cost of up to ~2 trips"
     (step1-results:113).
   - The DAgger result on go-down (step1-results:117) is missing.
   - s171's go-down is worth only +2.65 acting (step1-results:128), so copying go-down into s171 buys less.
     Step 0 of section 10 is partly done; absorb it.
2. **A causal claim stated as fact.** Line 67-68 says dig "removed the wall cuts the ant's descent depended on". The
   cause is untraced (step1-results:143), and the wall-dig rate fell only from 0.012 to 0.004-0.010
   (step1-results:75).
   Line 83 also says "the ceiling test showed the coupling first". The ceiling test's dig rule *alone, acting* was
   fine (+4.74, guidelines:94-96); the coupling it found was dig + home. These are different findings.
3. **Trial drop (review question 2).** A 300-frame trial does not measure the arena's 2,000-frame settle, and it
   ignores later pellets landing on this one. Validate it: per pellet, record the trial value and whether the pellet
   counted at the bell, and report agreement. Price the 2,000-frame variant. "Milliseconds" (line 383) contradicts
   section 4.3's ~20 ms upper bound per trial. Leave the cache hit rate unclaimed until measured. The nest planner's
   trial cut per frontier cell and the assignment re-solve are not priced at all.
4. **The T1 amendment belongs in the guidelines file**, with the owner's date, so later sessions do not apply the old
   T1.
5. **"Cost grows linearly with the ants"** (6.1) is per labelled decision. The number of decisions also grows with
   the ants, so the total is quadratic unless the labelling is sampled. The search design says "sample"; carry that
   into the doc.

---

## SUGGESTION

1. **Review question 4 (order).** Move the **search pilot on dig-or-not (step 6) ahead of the soil planner (step 2)**.
   It targets the uncopied dig lesson directly. Its sharp prediction, that labels computed under the brain's own
   continuation should not veto the wall cuts descent needs (search-teacher-design:415-418), is the clean test of the
   coupling story. The soil planner targets nothing that failed: put-down was dropped, and walk-home's real content
   is the empty return trip.
2. **If the capacity diagnostic says capacity**, Option Iteration (Young & Sutton 2023 **[abstract]**) learns a *set*
   of option policies, each matching the search on some states, instead of one policy that must match everywhere.
   That is the brain-lane analogue of "one shared scorer cannot hold every lesson". Distral (Teh et al. 2017
   **[summary]**) shares a distilled policy rather than raw parameters, to limit interference between tasks.
3. **Ordering lessons by learning progress.** Teacher-Student Curriculum Learning (Matiisen et al. 2017 **[summary]**)
   picks the subtask with the steepest learning curve and revisits tasks being forgotten. It is a principled
   replacement for the hand order "go-down, then dig, then home". The fetch-forgetting signal is already measured
   every round.
4. **The self-imitation arm** is the control for "is a teacher needed at all", and it has still not run. Note that
   imitating past successes can provably fail to improve a policy without a correction (Eysenbach et al. 2022
   **[abstract]**). Check the arm's labels against that.
5. **Review question 5 (first colony experiment).** Before any teacher, run the existing brain in 2-ant and 4-ant
   boxes. Does dug area already scale with N, through `CROWD`? Vary the world's switching cost (C2). Both are
   teacher-free, cheap, and tell you whether the colony tier needs a teacher for sizing at all.

---

## What the doc gets right, and should not change

- **The four-part teacher frame** (goal, computation, value per option, lesson), with the rules as the degenerate
  planner and as the examiner. It is the right abstraction, and it lets the probe, copier and judge be reused.
- **Stage 1 = T9 for every planner**: act first, count every force and veto, GIF for the owner with counts in `meta`.
- **"The planner proposes, the search disposes"**, and search labels the big calls while planner cost-to-go labels
  the steps. This is the correct split by stake. It needs only M2's validation.
- **Section 4.4's honest mixed prediction.** The planner fixes *where pellets go*, not rim spill-back, and the fill is
  censused by cause. Also, "never forbid mound cuts by position", taken straight from the dead-ends record, with the
  re-test condition the record names (a planner can tell a passage from a scrape).
- **New senses only through the brain lane**, with new links at zero so existing brains are unchanged, and only when
  the probe names the missing fact.
- **The owner picks the blueprint by eye before the nest planner is built**, and the chamber-height finding goes to the
  owner as renders, not as a ruling.
- **Abstention as "the brain itself"**, so every new teacher plugs into the unchanged copier.
- **C2 kept: vary the world before the brain** for division of labour.
- **The context probe** (section 7). It is the best idea in the doc for the failure that actually happened. Schedule
  it first.

## Sources

- Levine & Koltun 2013, *Guided Policy Search*, ICML — [pmlr](http://proceedings.mlr.press/v28/levine13) [abstract]
- Montgomery & Levine 2016, *Guided Policy Search via Approximate Mirror Descent*, NeurIPS — [arXiv 1607.04614](https://arxiv.org/abs/1607.04614) [abstract]
- Sun, Gordon, Boots & Bagnell 2018, *Dual Policy Iteration*, NeurIPS — [arXiv 1805.10755](https://arxiv.org/abs/1805.10755) [abstract]
- Balakrishna et al. 2019, *On-Policy Robot Imitation Learning from a Converging Supervisor*, CoRL — [arXiv 1907.03423](https://arxiv.org/abs/1907.03423) [abstract]
- Chang et al. 2015, *Learning to Search Better than Your Teacher* (LOLS), ICML — [pmlr](http://proceedings.mlr.press/v37/changb15.html) [abstract]
- Young & Sutton 2023, *Iterative Option Discovery for Planning, by Planning* — [arXiv 2310.01569](https://arxiv.org/abs/2310.01569) [abstract]
- Eysenbach et al. 2022, *Imitating Past Successes can be Very Suboptimal*, NeurIPS — [arXiv 2206.03378](https://arxiv.org/abs/2206.03378) [abstract]
- Warrington et al. 2021, *Robust Asymmetric Learning in POMDPs* (A2D), ICML — [pmlr](https://proceedings.mlr.press/v139/warrington21a.html) [summary; also in repo notes]
- Mirzadeh et al. 2020, *Improved Knowledge Distillation via Teacher Assistant*, AAAI — [arXiv 1902.03393](https://arxiv.org/pdf/1902.03393) [summary]
- Yu et al. 2020, *Gradient Surgery for Multi-Task Learning* (PCGrad), NeurIPS — [arXiv 2001.06782](https://arxiv.org/pdf/2001.06782) [summary]
- Teh et al. 2017, *Distral*, NeurIPS — [arXiv 1707.04175](https://arxiv.org/abs/1707.04175) [summary]
- Sartoretti et al. 2019, *PRIMAL*, RA-L — [arXiv 1809.03531](https://arxiv.org/pdf/1809.03531) [summary]
- Muthusamy et al. 2024, *Generalizability of GNNs for Decentralized Unlabeled Motion Planning* — [arXiv 2409.19829](https://arxiv.org/pdf/2409.19829) [summary]
- Ng, Harada & Russell 1999, *Policy Invariance Under Reward Transformations*, ICML — [mlanthology](https://mlanthology.org/icml/1999/ng1999icml-policy/) [summary]
- Buhl, Deneubourg, Grimal & Theraulaz 2005, *Self-organized digging activity in ant colonies*, Behav Ecol Sociobiol 58:9-17 — [HAL](https://hal-univ-tlse3.archives-ouvertes.fr/hal-00093178v1) [summary]
- Matiisen et al. 2017, *Teacher-Student Curriculum Learning* — [arXiv 1707.00183](https://arxiv.org/pdf/1707.00183) [summary]
- Grill et al. 2020, *MCTS as Regularized Policy Optimization*, ICML — [pmlr](https://proceedings.mlr.press/v119/grill20a.html) [abstract-level only; the closed form is recalled]
- Abdolmaleki et al. 2018 (MPO); Peng et al. 2019 (AWR) — [recalled, not searched]
