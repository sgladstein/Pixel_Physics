# A skill ladder without forgetting: weight protection, gates and stepping stones

*Started 2026-10-21 by the ant-brain research lane. Status: literature and
offline analysis only, nothing built. Sent to the brain lane on 2026-10-21.*

**Why:** each rung of the ladder (trail, then scarce-food foraging, then
digging, then brood) has to keep the skills below it. Fetch and trail are
held as floors. The rig already does four things about this:

- `protect=` leaves alone the 219 slots that serve only verbs the learning
  arena never offers.
- A named-slot protect list is planned, with review, for links a copy
  revives.
- `keepref=` measures the kept arenas against a fixed reference brain, so
  that a chain of runs cannot give up 20% per link.
- The gated suite (`src/train/challenges.rs`, `Suite`) makes each kept arena
  a pass/fail gate at `forget=0.8` of its reference.

The question here is whether the field's tools would add anything to those:
EWC, SI and Fisher importance; milestone pools; CMOEA; lexicase; POET.

**What this does not repeat:** the teacher lane's
`notes/curricula_and_skill_reuse.md`, which already covers incremental
evolution, behaviour chaining, modularity and connection cost, the milestone
pool, EWC, POET, Enhanced POET, teacher–student curricula and PAIRED.

**Tags:** [FT] full text read, [ABS] abstract only, [GK] general knowledge,
*here* the project's own numbers or code.

**Dead ends checked:** `Reports/dead-ends.md` has no entry for weight
protection, Fisher importance, gates, lexicase or CMOEA (grepped by
mechanism, 2026-10-21).

---

## 1. A correction to the record: what "weight protection" in evolution actually is

**Answer: Early 2019 is not "lower mutation on important weights". It is an
L2 anchor in the fitness, with every old weight weighted equally, and it
only did well together with a connection cost.**

Two places misstate it:

- Investigation part 5's bibliography says "weight protection (EWC adapted
  to evolution: lower mutation on weights important to old tasks)".
- The teacher lane's notes flag it as unverified.

What the paper actually did. Early 2019 (arXiv 1904.03178) [FT]:

- **The rule:** fitness = F_B(θ) − λ₁ Σᵢ (θᵢ − θ*_A,i)² − λ₂ Σⱼ θⱼ².
  - i runs over the links that existed after task A, j over links grown
    since.
  - There is **no Fisher information**: "all of the existing connections are
    equally important to the original task".
  - Mutation is untouched.
- **The best setting** was λ₁ = 0.035 and λ₂ = 0.02. Good trade-offs had λ₁
  1.5–2.5 times λ₂, so changing an old link should cost more than growing a
  new one.
- **The setup:** recurrent nets of 108 parameters, four classic-control
  tasks learned in sequence, 40 generations each, 30 runs. NSGA-II ran with
  network size as a second objective, applied every fifth generation.

Mean normalised fitness of the best individual, over 30 runs:

| Method | Task 1 (first learned) | Overall | Best run, overall |
|---|---|---|---|
| Normal | 0.151 | 0.651 | 0.818 |
| Modularity (size objective) | 0.130 | 0.647 | 0.787 |
| Weight protection | 0.294 | 0.684 | 0.959 |
| Weight protection + modularity | **0.415** | **0.734** | 0.961 |

- The size pressure helped only together with protection. That matches
  Ellefsen, Mouret & Clune 2015 (in investigation part 5): a connection cost
  helps only when the old skills are in the fitness.
- **For this rig:**
  - The wire cost (`wire=`) is the size pressure.
  - The keep gate is a *function-space* anchor: it scores behaviour on the
    old arenas, not distance in weights.
  - Benjamin, Rolnick & Kording 2019 (ICLR; arXiv 1805.08289) [FT] found a
    function-space anchor on a small memory beat EWC's parameter-space
    anchor (Q3 report, §3a).
  - So the rig already has the stronger form of both halves of Early's
    winning combination. A parameter-space L2 term added to the fitness would
    add nothing it lacks.

## 2. Graded (Fisher) protection: a mutation-side saving, worth one predictive check

**Answer:**

- Fisher-scaled protection would not stop forgetting better than the gate
  does. Its use is to **stop making children the gate will veto**.
- It is worth building only if a link's kept-arena Fisher importance predicts
  which children fail the gate. One offline check settles that.

**What Fisher importance would mean here:**

- The brain's choice is a softmax over options. The empirical Fisher
  importance of link i on a kept arena is
  Fᵢ = E_s[ Σ_a p(a|s) (∂ log p(a|s) / ∂wᵢ)² ], averaged over that arena's
  logged decisions.
- It is computable offline with the Python copy of the brain that the copier
  already differentiates.
- EWC's penalty Σᵢ Fᵢ δᵢ² is then a second-order prediction of how much a
  child's mutation δ changes the kept arena's choices.

**How it could be used (*here*, inference):**

- Scale each link's mutation step by 1/√(1 + λFᵢ), and gate pruning on Fᵢ as
  well as on |w| (Q1 §1).
- This is Lehman et al.'s safe mutation (Q1 §1) with the sensitivity measured
  on the *kept* arenas' states instead of the learning arena's.
- It generalises today's binary `protect=`, which gives Fᵢ = ∞ to a verb the
  arena never offers and 0 to everything else.

**Cheapest test, with no new training:**

1. Log s171's decisions on fetch and trail with the teacher lane's option
   logger, and compute Fᵢ for each live link in Python. This takes minutes.
2. Take the 50 arm-C children from the brain lane's 05:00 mutation test
   (regenerated deterministically if not saved). Score each on the kept fetch
   and trail arenas, at the suite's kept-arena settings. Assuming a kept-arena
   world costs about what a soil world does, that is about half an hour of
   CPU.
3. Rank-correlate each child's kept-arena ratio against three predictors:
   - Σ Fᵢ δᵢ² (Fisher);
   - Σ δᵢ² (unweighted);
   - "lost a link above 5" (Q1).

**Reading:**

- If Fisher clearly beats both alternatives (for instance, Spearman 0.6
  against 0.3), graded protection is worth a design doc. Every child it keeps
  under the gate is an evaluation saved.
- If it does not, the binary `protect=` plus gated pruning is enough.

**What not to build.** SI (Zenke, Poole & Ganguli 2017) [GK] estimates
importance from the path a weight took under gradient descent. A GA has no
such path, and the copier's path is short and teacher-shaped, so SI has
nothing to integrate here.

## 3. The gate is a noisy binary decision: measure how often it vetoes on noise

**Answer:** the suite's own documentation says "a gate that vetoes often is a
noise source of its own", and already prints each kept arena's `_under` part.
One analysis of logs the rig already writes says whether the gate is vetoing
children that are truly above the floor. If it is, a narrow ramp is the
graded fix.

**Why it can bite:**

- The floor is read on each kept arena's folded score, against an unpaired
  calibration mean.
- A child whose true kept-arena ratio is 0.85 is vetoed whenever its measured
  ratio falls under 0.80. With a measurement standard error of s, that
  happens with probability Φ((0.80 − 0.85)/s): 31% at s = 0.1, and 2% at
  s = 0.025.
- Each false veto multiplies a good child's fitness by a tenth. That is a
  large loss of selection signal, concentrated on exactly the children that
  are learning the new rung while keeping the old ones.

**The field's answers [GK]:**

- **Stochastic ranking** (Runarsson & Yao 2000): compare by fitness alone with
  a fixed probability (about 0.45), and by constraint violation otherwise.
  This keeps near-feasible stepping stones.
- **ε-constraint methods** (Takahama & Sakai): treat a violation below ε as
  feasible, with ε shrinking over the run.
- Both exist because a hard constraint discards the individuals next to the
  boundary, which are the ones most likely to cross it.

**Cheapest test: zero CPU, on existing logs.**

- From a past gated run's per-child output (each kept arena's ratio and its
  `_under` flag), take the vetoed children.
- Count how many had a measured ratio within one standard error of the floor.
  The suite records each arena's calibration spread in `Suite::sds`.
- **Reading:**
  - If most vetoes are near the floor, the gate is mostly vetoing on noise.
  - The graded fix (*here*, inference): multiply by 1 above 0.85, by 0.1
    below 0.75, and interpolate in log between.
  - That keeps the gate's job, which is that a kept skill's ratio never pays
    for the learned arena, everywhere except the band that noise decides.
- **Caveat from the suite's own history:** the gate replaced `min(ratio, 1)`
  because the ratio's downside noise drove selection (s81 gave up soil 62.7 →
  41.3). A ramp only 0.1 wide lets that noise in only inside the band.

## 4. Stepping stones across rungs: CMOEA, lexicase and milestone pools

**Answer:**

- **Not now.** With three or four skills and a population of 100, the gated
  suite already keeps the old skills.
- The CMOEA result says what to reach for when a *new* rung (brood, or
  colony tasks) stalls because the easy part takes over the population.

**CMOEA.** Huizinga & Clune 2018/2019 (Evolutionary Computation; arXiv
1807.03392) [FT]:

- **How it works:**
  - Keep a bin for every combination of subtasks: 2ⁿ − 1 bins, so 63 for six
    tasks.
  - Each bin selects only on its own subtasks, using the product of their
    scores, which penalises neglecting one.
  - Parents are drawn from all bins.
  - Its stated purpose: when one skill is easier, ordinary selection lets the
    easy lineage "outcompete the lineage of those specialized in jumping
    before they have the chance to adapt", and the stepping stone is lost.
- **Results:**
  - **Six-task hexapod:** CMOEA "either outperforms or is competitive with"
    NSGA-II, NSGA-III and ε-lexicase, and "substantially outperforms all
    controls" once modularity objectives are added.
  - **100-maze navigation:** CMOEA and ε-lexicase with a combined target were
    fastest early. A single bin caught up by about generation 200, and solved
    every training maze in more runs after 600.
  - Adding a linear combination of all objectives (their "combined target")
    improved the controls in almost all cases.
  - It ran at 1,000 offspring a generation.
- **Fit here:** with three skills, CMOEA is 7 bins. Every child is already
  scored on every arena by the suite, so the extra cost is population size,
  not evaluations. The maze result says the bins mostly buy *early* speed,
  which a ladder that starts each rung from a champion does not need.
- **Re-test when:** a rung's learned arena stalls while a kept arena is
  comfortably above its floor. That is the signature of the easy skill
  taking over.

**Lexicase on three or four cases is weak** (*here*, inference). It filters
by one case at a time in random order. With three noisy arena scores, that
is close to "pick the best on a random arena", and it inherits each arena's
noise in full. ε-lexicase helps with continuous scores, and its strength in
the CMOEA paper came with 100 cases.

**Milestone pool** (D'Silva et al. 2005, in the teacher lane's notes) is what
`keepref=` and the start genomes already are in spirit: fixed champions the
ladder measures against. Seeding their descendants back into a later rung's
population is the remaining half. It is worth trying only if a rung's run
loses a kept skill *below* its gate, which the gate currently prevents.

## 5. What the field does not offer, and is not needed

- **POET and open-ended curricula.** The teacher lane's notes cover them.
  Their advantage is on terrains generated beyond any hand-made ladder, and
  the owner's ladder is hand-designed on purpose (design rev 3).
- **Rehearsal.** The suite's kept arenas *are* rehearsal, every generation.
  The literature's rehearsal buffers exist for learners that cannot afford
  to revisit old tasks; a suite revisits them by construction.

---

## Ranked list for Q2

| # | Technique | Evidence | Cheapest test | Cost | Owner |
|---|---|---|---|---|---|
| 1 | Count gate vetoes within one standard error of the floor | here (Φ arithmetic); stochastic ranking and ε-constraint [GK] | read past runs' `_under` and kept ratios | 0 CPU | brain lane |
| 2 | A narrow log ramp (0.75–0.85) in place of the step | here, inference | only if #1 shows noise vetoes; an A/B run | as a normal run | brain lane |
| 3 | Fisher importance on kept arenas as a predictor of gate failures | EWC [GK]; Benjamin 2019 [FT]; Lehman 2018 [FT] | 50 arm-C children on fetch and trail, and three predictors | ~30 core-min plus minutes of Python | brain lane, with the teacher lane's logger |
| 4 | Correct the "weight protection" line in investigation part 5 | Early 2019 [FT] | — | a doc edit | brain lane |
| — | Parameter-space L2 anchor in the fitness | Early 2019 [FT]: it needs size pressure, and the rig has function-space gates | — | not recommended | — |
| — | CMOEA bins, lexicase, POET | Huizinga & Clune [FT]; inference | re-test when a rung's learned arena stalls with kept arenas above their floors | — | — |
