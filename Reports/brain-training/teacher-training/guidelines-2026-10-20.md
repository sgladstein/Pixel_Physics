# Teacher training: guidelines, and whether colony behaviours are affordable

*Written 2026-10-20 by the teacher-approach session, at the owner's request, after the discussion opened by
`Reports/handoff/teacher-approach-handoff.md` (on `claude/brain-investigation`). Status: **plan and guidelines, nothing
built.** The evidence is [`literature-2026-10-20.md`](literature-2026-10-20.md) (a review of ~80 papers, mostly read at
abstract level -- see its caveat) and the brain lane's investigation (`Reports/brain-training/investigation-2026-10-20.md`
on the same branch), cited below as "investigation part N".*

## The answer, first

**Colony behaviours are not infeasible, but they are unaffordable the way we train today, and the reason is not that
the simulation is slow. It is that training throws away almost everything a simulation tells it.** Each world gives the
genetic algorithm one noisy number. The same world contains hundreds of decisions, and a teacher can say for each one
what the ant should have done -- with no world-to-world luck in that answer. Copying a teacher is the one lever the
literature has measured at about 10x, repeatedly (kickstarting: 10x fewer steps and 42% better at the end; Nair et al.
2018: an order of magnitude on long sparse tasks). The rest of the saving comes from stacking several 2-5x levers on
evaluation, each of which has to be measured on our simulator before it is trusted.

The budget makes the stakes concrete. A 32-vCPU VM does 100 generations of the solo dig suite for about $0.65, so the
$100 buys about **15,000 solo generations**, or about **9,000 in a 4-ant box** (measured at 1.7x a solo box). The two
published colony-evolution runs closest to ours took **500 and 2,000 generations**, with networks of 33 weights or
hand-built rule blocks (Waibel, Floreano & Keller 2011; Ferrante et al. 2015). A Ferrante-sized run with 4 ants at 12,000
frames would cost roughly $40-50 at today's settings ($0.65 x 1.7 for four ants x 2 for the longer run, per 100
generations) -- half the budget for one experiment. With the levers below it is plausibly $5-15 -- an estimate, which
holds only if pairing and racing together deliver 3-8x on our simulator. Without them, colony training happens once or twice, total.

**Three cheap measurements decide how the budget should be split**, and all three can be taken locally, without VMs:

1. **How correlated two near-identical brains are on the same world** (rho). It sets how much paired worlds and racing
   can save: rho 0.75 halves the worlds a brain needs, rho 0.9 cuts them about fivefold.
2. **Which decisions a teacher can actually teach** (the copyability probe, guideline T5). It sets which skills copying
   can speed up.
3. **Whether a 2-4-ant short run ranks brains the way a full colony does.** It sets whether colony training is
   affordable at all.

## What we decided in discussion (2026-10-20)

- **Copying for individual skills; colony-level scoring, or a planner-teacher in a real crowd, for collective ones.**
- **Solo first, then colony** has always been the plan; nothing here changes it.
- The owner's insight that a teacher need not obey the old rules (local signals, true biology) stands -- with one limit,
  which the literature names and which is the first guideline below.

## Guidelines for a teacher

### T1. Privileged computation, never privileged information

The teacher may compute anything exactly -- pathfinding, geometry, counting -- **but only over what the ant has sensed or
could plausibly remember.** This is the one place the owner's "god-like teacher" idea needs a limit. When the teacher
decides from something the student cannot sense, the student does not learn the teacher's behaviour; it learns the
teacher's *average* behaviour over every situation that looks the same to it (Weihs et al. 2021, "the imitation gap";
Swamy et al. 2022). Worse, it never learns to *search*, because a teacher that knows where the food is never searches
(Vuorio et al. 2024).

The famous privileged-teacher successes do not contradict this: in "Learning by Cheating" (driving, 100% on the CARLA
benchmark) and the legged-robot teachers, the privileged input was either visible in the student's own camera or slowly
varying and recoverable from history (terrain, friction). Ours sorts the same way:

| Teacher knowledge | Can the ant recover it? | Use it? |
|---|---|---|
| Exact depth, row and wall/foot status of a candidate dig cell | Yes: every option carries 45 features (`DEPTH`, `UP`, `HEADROOM`...); investigation part 1 shows a foot target and a wall target read differently | **Yes** |
| Exact put-down rule (3 cells clear of the hole) | Mostly: `home_dist` and the drop hidden unit already track it (part 2) | **Yes** |
| Shortest path back to the nest | Partly: `home_dist`, `d_home` and trail scent | Only as far as the senses go |
| Where food is, three screens away | No | **No** -- teach the search, not the destination |
| "The colony needs three more diggers" | No, unless a sense stands in for it (C5) | Only through a proxy |

### T2. Label the brain's own runs, never the teacher's

Train on decisions from runs the *brain* drove, with the teacher labelling each one (DAgger, Ross et al. 2011). This is
mandatory for us, not a refinement, because the brain has 12 memory units: a student with memory trained on the
teacher's own trajectories learns to repeat its own last action ("latching") and gets **worse** when given more history
(Swamy et al. 2022; the "copycat" problem, Wen et al. 2020). The teacher is a program, so a label costs CPU, not a person:
label every decision the brain makes.

### T3. Score every option, and let the teacher abstain

The teacher should give **every option a score** (for a dig cell: the cost to finish the trip from it, including the
carry), not only its pick. Soft targets carry the ranking -- which wall cell is nearly as good as the foot -- and copy
better (Hinton et al. 2015; Rusu et al. 2016). It also fits the house law that an outcome is a distribution.

Where the teacher has no confident answer it **abstains**, and that decision contributes nothing to the copy loss. This is
what keeps the long tail out (the ant on a ledge, the pellet falling back, two rules fighting): the teacher writes rules
only for the decisions we can state exactly. For the 8-row dig step that is three, all from investigation part 1:

1. **Which cell to cut**: the foot, not the wall one row short (3.2 wasted trips a world at 4 rows, 5.6 at 8).
2. **When to put a load down**: about 4-5 columns out, not the median 10.
3. **When an empty ant turns back**: sooner (45% of its surface time is 10+ columns out).

### T4. Selection stays the judge

Copying changes where selection starts, or adds a term to what it sees; it never replaces it. This keeps the owner's
"one general brain, found by selection" ruling intact: the brain has the same structure and senses, and a copy that
lowers the real score dies. Two ways to plug it in, in the order to try them:

- **Offline copy, then select** (the handoff's planned test). Copy in Python with the exact brain math
  (`brainmath.py`, median error 0.000025), write a normal genome file, judge it, and use the survivors as training
  starts. No VMs.
- **Copy agreement as a fitness term**, later: score = outcome - lambda x copy-loss, with lambda decayed over a run so the
  student can pass its teacher (kickstarting's annealing). It needs no gradients and is measured on the runs the GA
  already does; per decision it is far less noisy than the outcome.

**A dead link can be revived by copying, and selection can delete it again.** A pruned connection is a plain zero weight
in the genome (`src/train/mod.rs` `mutate`), so a copying gradient can bring back the target-depth link the dig scorer
lacks (part 2). Pruning and the per-connection energy cost will delete it in the next run unless it pays its way, so
link protection (the brain lane's work) must cover links that copying revives.

### T5. Check copyability before training anything

Before spending a training run, log (what the brain sensed, the teacher's score for every option) from the brain's own
runs, fit the simplest predictor of the teacher's choice from the brain's senses, and report the agreement **per decision
type**. Run it with both controls the repo requires:

- **Positive control:** a label that is a pure function of current senses must reach ~100%.
- **Negative control:** labels shuffled across worlds must fall to the majority-label floor.

A decision type stuck near the floor is an imitation gap. Either take it out of the copy loss, or rewrite the teacher
(T1), or add the missing sense -- and the probe says which sense is missing. **The brain lane's logger records only the
chosen option's features; extend it to the whole option list first** (handoff, "What the log lacks").

### T6. Judge by the score on paired worlds, never by agreement

Low copy loss proves nothing: an agent can match its teacher's labels and still do badly when it drives itself (Pfrommer
et al. 2023). Every copied brain is judged against its parent on the same sealed worlds, paired per world (full loads,
unpaid trips, put-down distance), with fetch and trail against the fixed s91 floor -- and **a GIF for the owner before
anything passes**.

### T7. Self-imitation needs a bar above the noise, and must rank by speed

Per-world noise is about ten times the difference between brains (part 4), so a trip that paid is weak evidence that its
decisions were good. Two rules:

- Copy only trips that beat the brain's own typical result by more than the noise measured from repeat runs of the same
  brain on the same world.
- Rank successful trips by **time**, not only by paying. Every full load today also contains the 10-column carry and the
  overshoot, so copying "trips that paid" copies the waste along with the foot cut.

Expected outcome (a forecast, not a finding): the hand-written teacher wins on which cell to cut, where geometry is
computable; self-imitation competes only where nobody can write the rule.

### T8. The first test, including its controls

The handoff's test stands, with three additions:

1. **Round-trip control first**: copy the brain onto its own unchanged choices and confirm it scores the same as s161
   offline. If the copying code alone moves the score, nothing after it can be trusted.
2. **Score each correction on its own number**: wall cuts per world, median carry distance, and surface time 10+ columns
   out. Full loads are too noisy to see one correction.
3. **The one-day time box is a measurement**: record what the day produced and what it did not, as evidence on how hard
   the teacher is to write.

## Guidelines for colony behaviours

### C1. Clone the colony and score the colony

Every ant shares one genome, so the colony is a clone and scoring the colony is the textbook setting: same-genome teams
selected as a team do best on cooperative tasks (Waibel, Keller & Floreano 2009, *IEEE Trans. Evol. Comput.*), and with
relatedness 1 helping the colony is always favoured (Waibel, Floreano & Keller 2011, *PLoS Biology*). There is no
altruism problem, only a search problem: one number per colony. Cut its noise with a **decomposed colony score** (cells
dug + food home + brood raised, each attributable) rather than one end-state number.

### C2. The world must charge for switching jobs, or no division of labour will appear

**This is the most important result for castes, and it is about the arena, not the learner.** Ferrante et al. 2015
evolved identical 4-robot teams on team score alone: on a slope, where carrying items down cost less than walking them,
13 of 22 runs evolved a split into droppers and collectors. **On flat ground, none did.** Digital organisms (Goldsby et
al. 2012) and threshold models (Duarte et al. 2012) agree: if switching tasks costs nothing, a generalist wins, and no
training method will produce castes. So **the first colony experiment varies the world, not the brain**: separate the
work sites in space, make a load costly to hand over, or put a warm-up on a new task.

### C3. The ingredients for castes are already wired -- protect them through the solo ladder

Identical brains can only behave differently if something differs between ants, and the brain already has all three
things the literature says are needed:

- **`SELF`**, a fixed random value per ant (`src/sim/creature/mind.rs:1451`): the individual difference that threshold
  models need.
- **`AGE`**: young ants nurse and older ants forage in most real species (Mersch et al. 2013: three groups by place, moved
  through by age).
- **12 memory units**: feedforward networks only evolved specialisation fixed in the genes; adding self-feedback let it
  evolve over a much wider range (Duarte, Scholtens & Weissing 2012). In clonal raider ants, experience alone made
  lasting specialists: workers made to succeed at foraging kept foraging a month later (Ravary et al. 2007).

The danger is the solo ladder. In a solo arena **crowding always reads 0** (part 2's switch-off test changed nothing),
and anything that never matters gets pruned (part 3). Link protection should cover the links from crowding, `SELF` and
`AGE`, not only the fetch links. *To check:* `SELF` is keyed on the ant's id, so if the solo arena's ant always has the
same id, `SELF` is a constant the brain may be using as a bias -- and a colony would then hand each ant a value the
brain has never seen.

### C4. Rooms sized by crowding: at least two ants, and a teacher that reads crowding

No ant measures nest volume. *Messor* groups dig on a saturating curve and end with volume nearly proportional to
worker number; a model reproduces it with no explicit stop rule (Buhl et al. 2005). Crowding at the dig face shapes the
cavity (Toffin et al. 2009). So:

- **No solo rung can teach room sizing.** The first useful rung has 2+ ants. (Or fake the crowd in a solo arena: feed the
  crowding sense a virtual population divided by the room's size, which keeps the "digging lowers crowding" loop. It is
  cheap, but its crowd is smoother than a real one, so it must be checked once in a real colony.)
- A teacher rule such as "dig while crowding at the face is above c" is **copyable by construction**: its label is a
  function of what the ant senses. It is a threshold rule, not a planner.
- **The acceptance test is cheap**: the same brain in colonies of N and 2N, and the dug area should roughly double, on a
  saturating time course. Unequal workload is expected (Aguilar et al. 2018: tunnels dig best when some ants idle).

### C5. A planner-teacher works in a crowd only through a sense that stands in for the group

Robots copying a central planner reach near-planner performance and scale to teams 100x larger -- **but through explicit
messages between robots** (Tolstaya et al. 2019; Li et al. 2020). Copying from immediate neighbours' states alone is
reported insufficient (Chen et al. 2023), and **no study was found that teaches through a shared field like scent**. So a
planner that says "this colony needs three more diggers" is the imitation gap at its worst, unless some sense carries
that need. The best candidate from biology: harvester ants set their foraging rate from **how often they meet foragers
coming home with food** (Prabhakar, Dektar & Gordon 2012) -- a local encounter rate, which a memory unit could integrate
or a new sense could supply. Run the copyability probe on the planner's labels in a real crowd before writing more
planner; where it fails, add a local sense rather than a bigger brain, and fall back to colony scoring.

### C6. Train small, vary the size, check at full size

Ferrante's controllers evolved at 4 robots ran at 20 with 4.4x the output (5x ideal); the swarm-bot work drew team size
at random between 4 and 8 and held at larger sizes. It breaks where crowding changes behaviour -- exactly the room-sizing
regime -- so train across the density range, and post-check only the elites at full colony size. Carry **several
lineages** into the colony rungs: the best small-colony brains are not always the best seeds for larger colonies (Long
et al. 2020).

## Guidelines for keeping old skills

**An old skill survives only if selection still sees it, or its links are shielded.** A connection cost makes networks
modular, but the modules do not line up with skills, and a link no current arena tests is still taxed -- which is how
"don't eat the cargo" died (part 3). Cheapest first:

1. **Rehearsal**: score each brain on the current arena plus one earlier arena drawn at random, perhaps on a short run
   (about 1.1-1.5x the evaluation cost).
2. **A retention gate** -- the fixed `keepref=` floor, in use.
3. **Shield links whose removal hurts an earlier rung** -- the link protection being built, a gradient-free stand-in for
   EWC (Kirkpatrick et al. 2017).
4. **A milestone pool**: re-insert children of past champions (D'Silva et al. 2005).
5. **Freeze** a mastered skill's links (strongest; least compatible with "one brain found by selection").

## Guidelines for cutting cost

| Lever | What it saves (literature) | Measure first, on our simulator |
|---|---|---|
| Copying (individual skills) | ~10x fewer steps (Schmitt et al. 2018; Nair et al. 2018) | Copyability per decision type (T5) |
| Paired worlds | Worlds needed x 2(1 - rho) against a well-measured parent | rho of two near-identical brains |
| Racing to the cut line | 29-76% below the best fixed world count (Martin & Collins 2026); ours already 48 to 22 | Most mutants are clearly worse (4.9 vs 6.6 loads): let them leave after 1-4 worlds; parents carry their worlds forward |
| Cheap rungs (few ants, short runs) | Successive halving 5-30x (Hyperband; a distant domain) | Rank correlation of cheap vs full on ~50 brains |
| Inherited fitness | Few-sample evolution competitive on 1,000+ weights (Morse & Stanley 2016) | Bias from lucky early scores |
| A surrogate that only discards | "Several times fewer" evaluations (Gaier et al. 2018) | Never let it promote |
| GPU simulation | 100-10,000x for grid multi-agent sims (JaxMARL) | A rewrite of the cell sweep; profile before considering |

Two notes on the table:

- **Our paired worlds are already built right.** The usual failure is one shared random stream, where the first
  different decision reshuffles every later draw and the pairing is lost (arXiv 2409.02086). Every brain choice here
  draws from its own stream, keyed by world seed, ant, frame and purpose (`mind.rs:1732`). What is left is physical
  divergence once brains act differently, and rho measures exactly that.
- **Before choosing between racing and novelty search, find out what blocked the 600-generation step.** If better mutants
  appeared and were thrown away, it was noise, and racing and pairing fix it. If none appeared, it was a dead end in the
  search, and shaping or quality-diversity fixes it. A trace of the line's mutants separates the two.

## Proposed order of work

| Step | What | Needs VMs? | Decides |
|---|---|---|---|
| 0a | Measure rho: parent and a one-mutation child on the same 100 sealed worlds | No | How much pairing and racing can save |
| 0b | Extend the mind logger to every option; run the copyability probe on the three dig corrections, with both controls | No | Whether the hand teacher can teach them |
| 1 | The one-day partial teacher and self-imitation, copied offline, judged paired against s161 with the round-trip control (T8) | No | Whether copying beats selection for a dig step |
| 2 | Racing to the cut line on paired differences, A/B'd against the current trainer | Yes, short | The evaluation saving, measured |
| 3 | First colony arena, **varying the world**: 4 ants, two work sites, with and without a switching cost; does a split appear with a solo-trained brain plus `SELF`/`AGE`? Measure the cheap-vs-full rank correlation alongside | Yes | Whether colony training is affordable, and whether the arena makes castes pay |
| 4 | Room sizing at 2-8 ants with the crowding threshold teacher; acceptance test N vs 2N | Yes | Whether crowd behaviour can be taught |
| later | Reinforcement learning head-to-head with selection on one step, design doc first (standing ruling) | -- | -- |

Before step 0: link protection should cover crowding, `SELF` and `AGE`, so the solo dig steps the brain lane is running
do not delete them in the meantime.

## What nobody has measured -- where we are on our own

- **No study trains a network this small (~700 links) by copying**: every distilled student in the literature has at
  least ~10,000 parameters.
- **No study teaches through a shared field like scent**: multi-robot copying works through explicit messages.
- **No controlled study compares solo-first training with training the colony from the start.** The closest evidence
  (Ferrante's smooth search with pre-built skills; Christensen & Dorigo 2006, where staging *across* intertwined skills
  bought nothing) says solo-first should work when colony behaviour is mostly *choosing among skills the ant already has,
  under new cues* -- and may not when it needs a new skill. Carrying several lineages is the hedge.

These are the three places where our own measurements are the only evidence there will be, which is why steps 0a, 0b and
the cheap-vs-full correlation in step 3 come before any money is spent.
