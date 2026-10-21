# Search teacher: a design draft

*Draft, 2026-10-21, by a research agent for the teacher-approach session; its place in the plan and decisions on its
open questions are in [`teacher-design-2026-10-21.md`](teacher-design-2026-10-21.md) section 8. Written for the owner. Nothing here is built or measured; every number marked "estimate" has to be
measured before anything relies on it. The literature behind it is in
[`notes/search-teacher-literature.md`](notes/search-teacher-literature.md), tagged by how far each claim could be checked (full
texts were blocked, so most of it is at abstract level). Ground truth for the project:
`Reports/brain-training/teacher-training/` (the guidelines, the step 1 copier design and results, measurements 1-3).*

## What it does, and why

**A search teacher decides what the ant should have done by trying it.** At a decision, it copies the world once for
each option the ant has: step left, step down, dig this cell, put the load down, rest. In each copy it makes the ant
take that option, lets the ant's own brain carry on for a while, and scores how the copy turned out. The option whose
copies turned out best becomes the lesson, and the existing copier teaches it to the brain exactly as it taught "go
down".

**Why it is worth building.** Every teacher so far has been something a person wrote. The three hand rules took s161
from 6.8 to 20.5 full loads a world when they drove it, but they took a day to write, and they still got something wrong
on paper: "dig only at the foot" strands the ant unless "go down" comes with it. Rooms sized by crowding, many ants,
foraging and brood are far past what anyone can write rules for. A search teacher needs no rule. It needs only two
things: the simulation, which can already be copied mid-run (`World` derives `Clone`), and a way to score an outcome.
Three of its properties matter here:

| Property | Why it matters here |
|---|---|
| **It respects what the brain does around a decision.** The copies continue with the brain's own behaviour, so a lesson that would strand the ant scores badly and is never taught. | This is the exact failure the hand rules hit twice: dig + home stranded the ant, and the copied dig rule collapsed it to 3.35. |
| **It scores every option, not only its pick.** The lesson is graded: which wall cell is nearly as good as the foot. | Guideline T3, and the house law that an outcome is a distribution. |
| **It is privileged computation, not privileged information**, provided its lessons can be predicted from what the ant senses. | T1. Section 6 is the check. |

**Where it sits.** It is the third kind of teacher, after the hand rules (step 1) and self-imitation (built, not yet
run). It is the first kind that grows with the task instead of with somebody's rule-writing. The first test is on the
8-row dig, where we already know the right answer for two calls, so a search teacher that cannot find them can be caught
before anyone trusts it on a task where nobody knows the answer.

**What the literature says to expect.** This is a published recipe, not an invention: "approximate policy iteration by
rollouts and a classifier" (Lagoudakis & Parr 2003; Lazaric et al. 2016).

- **One step of it is worth a lot.** Tesauro & Galperin (1996) cut backgammon error rates by a factor of 5 or more, for
  base policies from random to TD-Gammon. Rollout algorithms in general give "a consistent and substantial improvement
  over the underlying heuristic" (Bertsekas 1999).
- **Repeated, it becomes expert iteration, and AlphaZero is this loop at scale.** Anthony et al. 2017 is the
  non-AlphaZero case.
- **What decides the cost is where the rollouts go.** Spreading them like a bandit, and stopping as soon as one option
  clearly wins, saved an order of magnitude (Dimitrakakis & Lagoudakis 2008).
- **It can fail two ways.** A search can do *worse* than the brain it starts from if it prunes options carelessly
  (Issakkimuthu et al. 2020). And it can know things the student cannot act on, which is the imitation gap again
  (sections 6 and 8).

## 1. What the teacher computes at one decision

### 1.1 The procedure

The decision is taken from a run the *student* drove (DAgger, T2). At the start of the frame in which the ant will
decide:

1. **Snapshot** the world: `W0 = world.clone()`. The ant's memory, energy and genome live in `world.minds`, so they
   come along.
2. **Choose candidates**: every option the ant is offered at this decision (typically 5-15), except where 1.2 trims
   the list.
3. **For each candidate a and each repeat r = 1..K**:
   - clone `W0`;
   - force option a at the ant's next decision. The ant's memory must record a as its own choice, as the label-only
     teacher already does;
   - *from that decision onward*, switch the copy to random stream r (section 2);
   - run H frames with the rollout policy (1.3);
   - optionally run a short settle (1.4);
   - score the outcome V (1.4).
4. **Estimate**: Q(a) is the mean of V over the K repeats. The comparison uses the *paired* differences
   D(a, b) = V(a, r) - V(b, r), with their standard errors.
5. **Turn the scores into a target distribution** for the copier (1.5).

**The salt switches only after the forced decision.** Frames before the decision must replay exactly, so that the
option list built in the copy is the one logged in the real run. Assert this with a hash of the option list on every
copy; an option index that means a different option in the copy is the cheapest silent bug available.

### 1.2 Which options to try

**Try all of them**, at least at first. With 5-15 options the list is short, and pruning by the brain's own
probabilities would hide exactly the calls the brain never makes. The ~31 "dig here" states a world where s161 walks on
are the case in point: self-imitation cannot label them (step 1 design section 7), and search can.

Two refinements, both from the literature:

- **Race the options.** Start every option at K0 = 2 repeats. After each round, drop any option whose paired difference
  to the leader is more than 2 standard errors below zero, and give the survivors more repeats, up to Kmax. This is
  sequential halving, as in Gumbel AlphaZero, or a Hoeffding/Bernstein race. Stop early when the leader is clear
  (Lan et al. 2021: for more than half of Go positions the network's first choice never changed under search).
- **Merge options that cannot differ.** Two put-down cells that are the same distance from the hole in the same pile
  are an example. This is optional: measure first how often it happens.

**Do not use hindsight.** Never pick the best option *per random stream* and then count wins. That is "determinization
in hindsight", which credits an option with luck only a clairvoyant could have (Yoon et al. 2008). Always compare means
over the same set of streams, with the first action fixed.

### 1.3 How the copy plays on after the first step: the rollout policy

| Rollout policy | What the label then means | For | Against |
|---|---|---|---|
| **R1: the brain itself (s161, or the current student)** | "Given that I keep being me, this option is best now." This is one step of policy improvement (Tesauro; Bertsekas). Copying it and repeating is expert iteration. | No human input anywhere, which is the point of the exercise. The labels respect the brain's own couplings. Improvement over the brain is guaranteed in expectation if Q is estimated well enough. | The brain is weak (6.8 loads), so few full loads land inside a short horizon and the outcome is coarse. A single deviation from a weak policy changes little. |
| **R2: the hand teacher plus the brain** (dig, down and home drive; the brain does the rest) | "Given that a strong player finishes the job, this option is best." This is AggreVaTe (Ross & Bagnell 2014): rollouts with the *expert's* cost-to-go after the learner's action. | Three times the loads in the same horizon, so a sharper signal. The guarantees are relative to the expert. | Limited by the hand rules. And if the student never reaches the teacher's level, labels computed for "a strong player afterwards" can mislead it. |
| **R3: a planner** (section 5) | "Given that a planner finishes the job." | Can be stronger than any rule set, and can be written for colony tasks. | Somebody has to write it, and its quality caps the labels. |

**Recommendation: R1 as the main arm, because it is the one that is not limited by what a person can write. R2 as its
control and accelerator.** If R2's labels copy and R1's do not, the difference is the rollout policy's strength, not
the search.

**Expert iteration** means R1 with the *current student* inside the rollouts, regenerated every round (section 4).

### 1.4 How long to play each copy, and how to score it

**The problem.** A full load is a pellet cut at the shaft's foot, carried up eight rows and put down three or more
cells clear of the hole. For s161 that takes:

| Stage | Median frames |
|---|---|
| Foot cut to put-down | 220-270 |
| Put-down to the next foot cut | 655 |
| One full load, on average | ~1,500 (6.8 loads in a run) |
| One full load under the hand teacher | ~490 (20.5 loads) |

The arena also scores only **after a 2,000-frame settle**, so a pellet left where it slides back counts for nothing. A
dig decision is therefore paid for hundreds of frames later. RUDDER (Arjona-Medina et al. 2019) is the standing warning:
forward-looking search pays for that delay by averaging over a number of futures that grows with the delay.

**The answer the literature gives** is truncated rollouts with an estimate of the value at the cut-off. Tesauro's
truncated rollouts beat his full ones, and Gabillon and Farahmand et al. analyse the same trade-off. THOR (Sun et al.
2018) gives the principle: the better the estimate of value-to-go at the cut-off, the shorter the horizon can be, down
to one step for a perfect one.

**Candidate outcome measures.** All are cheap to compute from what the arena already tracks (`soil_trip`, the frame-0
ground, the hole):

| # | Measure | Resolves small decisions? | Risk |
|---|---|---|---|
| M0 | **Full loads completed by H, after a short settle** (the arena's own definition) | No: mostly 0 or 1 at H = 1,200 | Very coarse; needs a large K |
| M1 | **Stage credit**: each pellet scored by the furthest stage of the soil trip it has reached (cut, put down, put down clear, carried up from below), at the arena's own stage weights | Partly | The weights are the arena's, which is good, but are not "full loads" |
| M2 | **M0 + completion probability of what is in flight**: for a pellet being carried, P(it becomes a full load given its stage and height), read from a lookup table built from s161's own logged runs. For an empty ant, the expected frames to its next foot cut given its depth and place, from the same logs, converted to loads at s161's rate. | Yes | It is a value estimate *for s161*. It goes stale once the student changes, so rebuild it every round. |
| M3 | **M0 + a fitted value of the end state**: regress "full loads in the next 3,000 frames" on a short state summary, from the logs | Yes | Can be wrong where data are thin. It is the Tesauro recipe. |
| M4 | **Time to the next milestone** (next foot cut, or next full load), censored at H | Yes, finest | Rewards hurrying over doing; needs M0 beside it |

**Charge energy on all of them.** Subtract the joules spent at s161's measured exchange rate of loads per joule. The
soil arena runs on a fixed 2,400 J budget, and a lesson that digs harder can run the ant dry before the bell.

**End each rollout with a short settle**, 200-300 frames under the rest mind, as the arena does at the bell. This keeps
the "posts beside the mouth" problem visible: a put-down that slides back must score as nothing.

**Start with M2, keeping M0 and M4 recorded beside it.** Validate it as follows; only the last check is decisive.

| Check | What | Pass |
|---|---|---|
| State-level | On logged s161 states, does V(state) rank states by the full loads s161 actually completes in the next 3,000 frames? | Spearman clearly above the M0-only baseline (bar set from the measurement, with headroom) |
| **Decision-level** (B1 in section 7) | On ~100 decisions, compare the short-horizon best option with the best option from rollouts **to the bell** (K = 16) | Agreement on the decisions where the to-bell answer is itself clear, at least ~90% of the to-bell answer's own repeat-agreement |
| End to end (B3 in section 7) | The search teacher *acting* scores more full loads than s161 alone | Above s161 by more than two clustered standard errors |

**Horizon.** One rollout records V at checkpoints H = 150, 300, 600, 1,200 and 2,400 frames and at the bell, so a
single run answers every horizon. Pick the shortest H whose decision-level agreement with the to-bell answer has stopped
rising. **H = 1,200 plus a 300-frame settle is the starting guess**: about one of s161's load cycles. Jiang et al.
(2015) are a reason to expect the best horizon to be shorter than the true one: past some depth, extra frames add noise
faster than they add information.

### 1.5 From scores to a lesson the copier can use

The copier wants a target distribution at each of the brain's two stages (verb, then target), with KL on every decision
(step 1 design section 2).

1. **Advantage over the brain.** A(a) = Q(a) - sum over b of p_brain(b) Q(b), where p_brain is the brain's own
   probability of choosing b at this decision.
2. **Shrink it by its noise.** A~(a) = sign(A) x max(0, |A| - 2 SE(A)). An option that is not clearly better or worse
   than the brain's average gets no push either way.
3. **Target**: pi\*(a) is proportional to p_brain(a) x exp(A~(a) / tau). This is the regularised policy-improvement
   step that AlphaZero's search approximates (Grill et al. 2020). It also keeps the lesson near what the brain can
   already do: coaching (He et al. 2012) and PLATO's "stay near the learner" make the same move against the imitation
   gap.
   - As tau goes to 0 the target becomes the best option; as tau grows it becomes s161.
   - tau is in full-load units. Choose it on the 192 selection worlds from {0.03, 0.1, 0.3}, never on the evaluation
     worlds.
4. **Abstain.** If no option clears its 2-SE band, pi\* is the brain itself. This is the step 1 design's "no rule
   applies" row, so the copier needs no change.
5. **Weight each decision by its stake.** Use the spread of Q across options (cost-sensitive classification,
   Lazaric et al. 2016), so a dig call worth half a load outweighs a step worth a hundredth. Keep the copier's per-type
   normalisation, so that the many small decisions cannot drown the few large ones (finding 14).
6. **Two-stage conversion.**
   - Verb-stage target: the sum of pi\* over each verb's options.
   - Target-stage target: pi\* within the brain's chosen verb.

**Why not argmax.** Hard labels from noisy estimates flip between near-ties. They also throw away the ranking T3 asks
for. And they would push the walk-home kind of lesson onto every look-alike state at full strength, which is how "walk
home" spread to loaded ants and broke fetch.

## 2. Noise: paired rollouts with the keyed random numbers

**The idea.** Two copies that differ only in the first option share every random draw that does not depend on what the
ant did. Draws are keyed by (world seed, ant, frame, purpose), and the cell sweep's draws by (seed, x, y, frame). So
grains fall the same way in both copies wherever the ant did not disturb them. Comparing options *within* a random
stream cancels the luck that both copies share. The variance of a difference falls by a factor of (1 - rho), where rho
is the correlation of the two options' outcomes.

**Why this design is already right, by the literature.** Buffalo, Pearson & Klein (2026) show that common random
numbers are valid only when "the same draw index corresponds to the same modeled event". A stateful random generator
breaks that the moment two runs diverge, and the fix they prescribe is draws keyed on an event identifier, which is
`rng::stream`. PEGASUS (Ng & Jordan 2000), Veness et al. (2011) for MCTS, and Yadav et al. (2026) for one-step rollout
planning all use the same principle.

**How a repeat gets its own luck.** In the copy, replace `world.seed` with `stream(seed, salt_r, ROLLOUT, 0)` at the
forced decision. Every stream then changes: the ant's choices and the physics alike. All options in repeat r use salt_r.

- **Check first that nothing reads `world.seed` as an identity.** On the bounded soil bed nothing should. Once M10
  streams terrain from the seed, swapping it would regenerate unexplored ground, and the salt must become a separate
  field mixed into `stream`.

**Where the pairing will leak.** After the first option, the two copies' ants decide at different frames: a dig takes
longer than a step. The ant's draw at frame t then serves different decisions in the two copies, and its choices
decorrelate. Measurement 1 is the far end of this: two different brains over a whole run correlate at about 0.05.
**Inside rollouts only**, the ant's draw could instead be keyed by *how many decisions it has made since the snapshot*
(seed, ant, decision index, purpose). Decision k would then use the same random number in both copies, and with the
inverse-CDF pick the same quantile of the brain's distribution. This is a hypothesis to measure, not a fix. It changes
nothing outside the teacher.

**How many repeats this buys.** To call an advantage Delta at 2 standard errors, K >= 8 sigma^2 (1 - rho) / Delta^2. For
a 1,200-frame rollout, sigma is roughly 0.7 full loads. That is an estimate, from a load about every 1,500 frames.

| Decision | Stake Delta, full loads (estimate) | K at rho 0 | K at rho 0.5 | K at rho 0.9 | K at rho 0.99 |
|---|---|---|---|---|---|
| Dig here or walk on | ~0.3-0.5 | 16-44 | 8-22 | 2-5 | 1 |
| Put down here or carry on | ~0.1-0.3 | 44-390 | 22-200 | 5-40 | 1-4 |
| One step (deeper or not) | ~0.02 (the down rule's +3.4 spread over ~216 such decisions a world) | ~9,800 | ~4,900 | ~980 | ~98 |

**This table is the most important prediction in the draft.**

- **Big single calls are cheap to label.** Dig or not is the call copying struggled with, and it is the cheap one.
- **Behaviours that pay only when repeated are nearly invisible to one-step search scored in loads.** "Go down" is
  exactly such a behaviour. For those, three levers help, cheapest first:
  - a finer measure (M4: frames to the next foot cut);
  - pairing that holds (rho near 1 at short H);
  - **persistent deviations**: force "the deepest step" at the next m step decisions, then hand back to the brain, and
    credit the label to all m. This is searching over options rather than primitive actions, which de Waard et al.
    (2016) found pays most "when a certain subgoal has to be reached".

**What to measure first (S1, about 4 minutes on 4 cores).**

Sample 200 decision states from s161's logged runs, stratified by type: empty below ground, empty on the surface,
carrying, and dig offered. For each, take two options (the brain's own pick and the most different alternative) × 16
salts, rolled 2,400 frames, recording V at every checkpoint. Report:

| Quantity | Meaning |
|---|---|
| **rho(H) per decision type**, as the median over states of the correlation, across salts, between the two options' outcomes | What pairing buys |
| var(difference) paired against unpaired (pair a with a different salt for b) | The saving, directly |
| sigma(H): the spread over salts of one option | Pure noise |
| The share of decisions where the two options' outcomes are identical on every salt | "This decision does not matter". If it is near 100%, something degenerate is going on: the tidiness tell |
| \|Delta\| / SE distribution per type | Which calls are labelable at what K |

Controls for S1, each able to fail:

| Control | Expect |
|---|---|
| Positive: same option, same salt, two copies | Identical outcome, rho exactly 1.000 |
| Positive: brain's own pick, *no* salt change | The copy reproduces the real run frame for frame |
| Negative: options paired across mismatched salts | rho near 0 |
| Specificity: the same option forced under two labels | Delta exactly 0 on every salt |

**rho also has a failure mode to check.** Wright & Ramsay (1979) found an inventory model in which common random numbers
induced *negative* correlation and increased variance. If any type shows rho < 0, unpaired sampling is the right choice
for it.

## 3. Cost

**Unit cost.** One 12,000-frame world takes ~0.7 CPU-s, so one simulated frame costs about **58 microseconds**.

**Unmeasured: the cost of a clone.** The bed is 256 cells wide, plus fields and minds. If a clone costs about 1 ms, the
32 clones of one decision add about 32 ms, a few per cent of the rollouts. Measure it first, with a criterion bench.

**Counts, from the logs.** A run has ~2,700 decisions. Of those:

- ~1,359 a world are empty-jawed with a Dig offered;
- ~39 a world have a foot cell in reach;
- s161 actually digs ~22 times a world.

**Price.** From the step 1 results, ~8 VM-hours on 32 vCPUs cost ~$3.50, so about **$0.014 per vCPU-hour**.

| Job | Decisions | Options × K × frames per decision | CPU-hours | On 4 local cores | VM cost |
|---|---|---|---|---|---|
| S1: pairing and noise, 200 states | 200 | 2 × 16 × 2,400 | 0.25 | 4 min | -- |
| B1: short horizon against to-bell, 100 decisions | 100 | 8 × 16 × ~6,000 | 1.2 | 20 min | -- |
| **Pilot labels: dig-or-not only**, 48 worlds, every foot-in-reach state plus 1 in 10 of the others (~175 a world) | 8,400 | 2 × 8 × 1,500 | 3.3 | 50 min | -- |
| Search *acting* on dig calls, 16 worlds (the ceiling, B3); racing at K0 = 2 on every empty decision with a Dig offered | ~21,700 | 2 × 2-8 × 1,500 | 2-8 | 0.5-2 h | -- |
| Full scope, 1 decision in 10 labelled, 48 worlds | 13,000 | 8 × 4 × 1,500 (half that with racing) | 5-10 | 1.3-2.5 h | ~$0.10 |
| Full scope, every decision labelled, 48 worlds | 130,000 | same | 50-100 | 12-25 h | ~$1 |
| One ExIt round (relabel with the new student inside the rollouts) | as above | as above | as above | as above | as above |

**For scale**, the two 100-generation selection runs at 8 rows (s171, s172) took ~8 VM-hours together, ~256
vCPU-hours, for 6.8 to 9.5 full loads. It is not clear from the report whether that figure is per run or for both. A
sampled search round is 2-4% of it, and labelling every decision is ~20-40%.

**Which decisions to label.**

| Choice | When | Why |
|---|---|---|
| **Pilot: dig-or-not only** | first | Large stake per decision; a known right answer to check against; the call copying struggled with |
| **1 decision in 10, stratified by type, plus every "rare and crucial" state** (foot reached, top of the shaft, carrying at the mouth) | the first full round | Consecutive decisions 5 frames apart are near-duplicates, and the rare states carry the lessons |
| Every decision | only if the 1-in-10 copy is clearly data-limited (held-out copy loss still falling with more data) | Costs about 10x |
| Skip decisions with one option, decisions in the settle, and decisions where racing finds every option inside 2 SE of the others (abstain) | always | They teach nothing |

## 4. How it plugs into the copier, DAgger and expert iteration

**The copier is unchanged.** It already takes a per-decision target pi\* at both stages, uses KL on every decision, and
treats abstentions as "target = s161". The search teacher writes pi\* into the same full-precision log that
`PP_TEACH_LABEL` fills, plus three columns per option: Q, SE and n. The copyability probe reads the same file.

**The loop.**

| Round | Who drives | Who is inside the rollouts | Labels | Train | Judge |
|---|---|---|---|---|---|
| 0 | s161 on the 48 training worlds | s161 (R1); in the control arm, hand teacher + s161 (R2) | search pi\* | copy from s161 on round 0 | 192 selection worlds; the 96 evaluation worlds once, at the end |
| 1-5, **DAgger** | the latest copy | **s161, fixed** | relabel the new copy's states; pool them | copy from s161 on the pool | same |
| 1-5, **expert iteration** | the latest copy | **the latest copy** | relabel; **use only the latest round's labels**, because Q has changed under them | continue from the latest copy | same |

- **DAgger with a fixed search teacher converges, at best, to "one step better than s161".** That is a ceiling worth
  knowing: Tesauro's factor-5 gain was a single step.
- **Expert iteration can go further, but each round must beat the last on full loads to be kept** (T4: selection stays
  the judge).
- **The "M2" value table must be rebuilt from the current student's own runs every expert-iteration round.** It is a
  value of the rollout policy.

**Selection after copying** is as step 1 proposed: a selection run from the copy, with the kept-skill floor, and with
any links the copy revived protected.

## 5. How it relates to the hand teacher and to the planner-teacher idea

**The hand teacher becomes the search's examiner, not its competitor.** On the states where a hand rule fires, the
search must agree with it, or explain why it does not (section 7, B2). Where they disagree, the ceiling test decides.

**Planners fit in four places, from least to most risky.**

| Role | Example | Comment |
|---|---|---|
| **Terminal value** (THOR's oracle) | Shortest-path length to the nearest clear put-down spot; path to the nearest foot cell; how far a heap would slide | Shortens the horizon a lot if accurate. It is privileged computation on the true state, and the rollout already uses the true state, so this adds no new information problem |
| **Option generator** | Nest-shape analysis proposes dig sites; heap analysis proposes where to put soil down | The search then *judges* the planner's proposals by playing them out, so a bad heuristic is caught instead of taught. The fix for "the drop rule built posts" |
| **Rollout policy** (R3, AggreVaTe) | A planner ant that takes direct routes to food and back | Labels become "best given a planner finishes the job". Strong, but capped by the planner, and the planner has to be written |
| **Teacher on its own** | The planner's action is the label | Not this design. It is the hand teacher again, with more code |

**For colony tasks, the planner's natural role is the second one.** The planner suggests and the search judges, with
the colony's decomposed score (C1) as the outcome.

**Several ants: search one ant at a time.** Copy the colony, force one ant's option, let every other ant run the shared
brain on its own keyed stream, and score the colony. This is SPARTA's single-agent search (Hanabi 24.08 to 24.61, with a
no-worse guarantee) and Bertsekas's one-agent-at-a-time rollout, whose cost grows linearly with the number of agents.
Pairing should still work, since every other ant's draws are keyed by its own id. Two warnings:

- The labels see every other ant's future, which is guideline C5's imitation gap at full strength. Run the probe on
  crowd decisions before believing them.
- Cost scales with ants × decisions, so sample.

## 6. The imitation gap for search labels

**What the search knows that the ant does not.** The search does *not* know the future luck: it averages over K streams,
which is what keeps it out of the hindsight trap. It does know the **whole current state**:

- the stone buried two cells below;
- loose soil out of sense range;
- exactly how a pile will slide.

So its Q is a function of things the ant cannot sense, and a student copying it learns the average lesson over
everything that looks the same to it (Weihs et al.; T1).

**How to check it.** This is the existing probe, plus one new control.

1. **Copyability probe** (T5, `probe.py`) on the search's best option, or its top set, per decision type, on held-out
   worlds:
   - positive control: a pure function of the senses;
   - negative control: labels shuffled *within a decision*, the control that replaced the invalid cross-world one.
2. **Test-retest (new).** Label the same 500 states twice with disjoint salt sets. The agreement between the two label
   sets is the **ceiling any student could reach**, because the labels themselves are only that repeatable.
   - **Probe ≈ test-retest:** the gap is noise. More K fixes it.
   - **Probe well below test-retest:** the gap is hidden information. More K does not help.

**What to do when a type fails.**

| Diagnosis | Remedy, cheapest first |
|---|---|
| Noise-limited | More K, or racing, for that type; or abstain |
| Information-limited, and the missing fact is local (stone under the cut) | **Information-matched rollouts**: give each salt a fresh draw of the parts of the world the ant has not sensed, from the bed generator. The label then averages over what the ant could not know, which teaches "how to search", not "where it is" (T1). Hard to build, so last |
| Information-limited, and the probe names a sense that would carry it | Add the sense (the probe says which) |
| Information-limited, and none of the above | Take the type out of the copy loss (abstain), and leave that call to selection |

**The regularised target in 1.5 is itself a guard.** It never asks the brain to move far from what it already does,
which is the coaching move (He et al. 2012), applied every decision.

## 7. Controls and the first experiment

**The question it answers.** Does a search teacher, given only the simulation and an outcome measure, produce dig-step
lessons better than the brain's own? And does copying them raise full loads? It is answered on the one call where a
search should find a large, cheap signal: **dig here, or walk on, when empty-jawed with a Dig offered**.

### A. The instrument (before any label is trusted)

| Step | What | Pass | Cost |
|---|---|---|---|
| A1 | Force-option hook: the forced option is the one logged (option-list hash matches); the ant's memory records it | 100% match | minutes |
| A2 | Determinism: same option and same salt give identical copies; the brain's own pick with no salt change reproduces the real run | Exactly identical | minutes |
| A3 | S1 (section 2): rho(H), sigma(H), stakes per type, with its controls | Controls behave. rho for the dig call high enough that K <= 8 resolves Delta ≈ 0.3 | 4 min |
| A4 | Clone cost | Under 10% of rollout cost | seconds |

### B. Are the labels right?

| Step | What | Pass | What it catches |
|---|---|---|---|
| B1 | Short horizon against to-bell, 100 decisions (1.4) | Agreement near the to-bell answer's own repeatability | A measure that does not track full loads |
| **B2a** | **Recovery of the hand teacher's clear call: "dig at the foot, not the wall".** On s161 states, search R1's verb-stage label against the hand rule's must-dig / must-not-dig, scored separately on the two (97% of states are "must-not") | Must-dig recall and must-not-dig precision well above s161's own agreement and the floor | A search that cannot find a +4.7-load lesson is broken. Stop here |
| B2b | Recovery of "go down", with M4 and persistent deviations (m = 10) | Agreement above the floor of 0.41 (a uniform pick) | **The sensitivity stress test.** Section 2 predicts that *one-step* search scored in loads fails this. Run it both ways and report both: the one-step failure is a finding, not a bug |
| B2c | Positive control with a known sign: carrying on the surface, put down *inside* the 3-cell zone against 3+ clear | Search prefers clear on nearly every such decision. The arena pays nothing inside the zone, so the difference is known to be non-zero | A measure blind to the settle |
| **B3** | **Search acting** (T9 for this teacher): search R1 makes the dig-or-not call live on 16 worlds; s161 does everything else. Count every force and veto. A GIF of sealed world 2 for the owner, with the force/veto counts in the card's `meta` | Full loads above s161 alone (6.8) by more than two clustered standard errors | If the search cannot improve the brain by driving it, copying it cannot either |

**B3's comparison points.** The hand dig rule alone, acting, scored 11.54. A search that lands near it found the lesson
by itself. **The sharp prediction to check is whether it avoids the hand rule's coupling failure.** The copied dig rule
collapsed the ant to 3.35, with 15 seeds under 3. The search labels are computed with the brain's own continuation, so
they should never veto a wall cut that the ant's descent depends on. Count seeds under 3.

### C. Does copying it work?

| Step | What | Pass |
|---|---|---|
| C1 | Copyability probe and test-retest (section 6) on the pilot labels | Probe well above floor; gap to test-retest stated |
| C2 | Copy round 0 (the dig call only) into quieted s161 (6.42), all copier controls as in step 1 | -- |
| C3 | Judge on the 96 evaluation worlds; standard errors clustered by seed; fetch and trail against the kept-skill floor; live links and energy at the bell; a GIF | **Beats 6.42 by more than two standard errors, with kept skills at floor.** Second bar: beats the hand dig-only copy (3.35) by a wide margin. Stretch: joins "go down" (10.57) for more than either alone |

**Controls for C, each able to fail.**

| Control | Expect | Catches |
|---|---|---|
| Plumbing positive: feed the hand teacher's labels through the *search's* label path and copier settings | Reproduces step 1's numbers (go-down copy ≈ 10.57) | A label-format or conversion bug |
| Shuffled search labels, within decision | No gain | "Any training helps" |
| Unpaired search: mismatched salts across options, same K | Labels noisier; copy gains less | Shows what pairing bought |
| Zero-horizon search (H = 5 frames) | Labels near uninformative; no gain | A gain coming from something other than looking ahead |
| Abstain everywhere (pi\* = s161) | Reproduces s161 | The round-trip control |

**Then, if C passes:** add put-down and step decisions (1 in 10, stratified), run the R2 arm beside R1, then expert
iteration rounds. Every round is judged on full loads, and the owner sees a GIF before anything passes.

## 8. Risks, and what would make me abandon it

| Risk | Signal | Response | Abandon if |
|---|---|---|---|
| **Noise swamps the comparisons** | S1: rho low (< 0.5) even at H = 300, and stakes small | Decision-index keying in rollouts; finer measure; persistent deviations; label only high-stake types | Even the dig call, the largest stake, needs K > ~50 at the chosen H (~9 s a decision; the pilot round alone goes over ~20 CPU-hours, and the smaller-stake types become unaffordable): the search is then a slower, worse self-imitation |
| **The short-horizon measure does not track full loads** | B1 agreement poor | Longer H; M3 instead of M2; planner terminal value | No measure reaches agreement with the to-bell answer at any affordable H |
| **The search does not beat the brain it starts from** | B3 at or below 6.8 | Check pruning and option forcing; check that each option's memory update matches what it would be if the brain had chosen it. The literature has search hurting its base when the choice rule is wrong (Issakkimuthu et al. 2020) | B3 fails after the instrument checks pass: there is nothing to copy |
| **It cannot find the known answer** | B2a fails | -- | B2a fails: stop and find out why before any further spend |
| **Labels are not copyable** (hidden information) | Probe well below test-retest | Section 6 remedies | Most of the label mass sits in types that fail the probe even after the regularised target |
| **The lesson spreads to look-alike states** (the walk-home problem) | Fetch or trail drops | Keep fetch/trail decisions in the pool with target s161; regularised target with a larger tau | The same as step 1's walk-home finding: the shared scorer cannot hold the lesson and the kept skill together. That is the brain's capacity, not the teacher's |
| **It exploits the simulation** | The outcome rises but the GIF looks wrong (posts by the mouth; pellets wedged where they count but no ant would put them) | The owner's eye is the bar; adjust the measure, not the brain | -- (a measure problem, fixable) |
| **One step is too little** | DAgger with fixed s161 rollouts plateaus low | Expert iteration (the student inside the rollouts) | ExIt rounds stop improving full loads while the copy loss keeps falling (Pfrommer: agreement is not performance) |
| **It costs more than selection for the same gain** | Full loads per CPU-hour, against selection's ~3 loads per 256 vCPU-hours (6.8 to 9.5) | Sampling and racing | Two rounds of honest tuning leave it behind selection *and* behind the hand-teacher copy per CPU-hour, on the dig step. On colony tasks there is no hand teacher, so that comparison changes |

## 9. What has to be built (scratch branch, never `main`)

| Piece | Where | Size (estimate) |
|---|---|---|
| Force-next-decision hook: the mind takes option j at its next decision and records it in memory as its own | `mind.rs`, beside the label-only teacher | small; mostly exists in the acting teacher's force path |
| Salt-at-decision: swap the copy's seed for a salted one right after the forced decision | the rollout driver | small |
| Rollout driver: snapshot, then options × salts in parallel threads; checkpointed V; mini-settle | new example (`searchlabel`), reusing the soil arena's step and scoring | about a day |
| Outcome measures M0-M4 and the M2 lookup table from logs | Python for the table; Rust for the scoring | half a day |
| pi\* writer into the copier's log format, with Q, SE and n columns | the logger patch | hours |
| S1/B1 analysis scripts | Python | hours |

**Order:** A1-A4, then S1, then B1, then B2a and B2c, then B3, then the GIF to the owner, then C. Each stage can stop
the work, and only B3 costs more than an hour.

## Open questions for the owner

1. **Main arm R1 (the brain inside the rollouts, no human rules) with R2 (hand teacher inside) as its control.** Or
   R2 first, because it is likelier to work?
2. **May the search teacher's outcome measure include an energy charge?** It changes what "best" means, and an ant that
   hurries may look wrong even when it scores.
3. **"Go down" is predicted to be invisible to one-step search scored in loads.** Is a persistent-deviation search
   ("take the deepest step for the next ten steps") still "not hand-written"? It names a behaviour class, but not a
   rule for when to use it.
