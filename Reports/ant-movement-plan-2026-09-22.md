# The ant's walk: what is built, what the research says it should be, and the plan

*2026-09-22. Plan of record, agreed with the owner the same day. **Steps 1
and 2 (the decision trace, and the drop, cone and homeward counters) are
built; results in
[`ant-decision-census-2026-09-22.md`](ant-decision-census-2026-09-22.md),
step 2 in its §10. Nothing after them is.** The mechanism it builds on is described in
[`how-the-ant-works.md`](how-the-ant-works.md), the living reference, which
is read instead of re-deriving the ant from reports. This report is the plan
and the reasoning; that document is the ant.*

*Revised the same day, with the owner's agreement, after writing
`how-the-ant-works.md` from the source turned up five things that change the
order and some details. The revisions are folded in, not appended; §1 lists
them.*

*Revised again 2026-09-23, after step 1's results. The changes are to the
instruments and scenes, not the design: C1–C3 are re-scoped (§5), S0's
setting is stated (§6), stage 1's limits on the bed are stated (§4i), and the
drop census starts at the anchor (§8).*

## 0. The answer, stated once

- **The shipped ant picks its direction almost entirely at random.** Stepping
  is one probability (`Move`), and the pheromone reaches only that. Direction
  is a three-cell forward cone with no pheromone or home term, plus a random
  re-roll when a step is not taken. The one directed element is the homeward
  re-roll, and only laden ants get it. That is how a bacterium finds food,
  not how an ant does.
- **Side view changes the problem, and it changes differently in each
  setting.** On flat ground and in tunnels the only choice is *keep going or
  turn around*. At corners, stems and junctions it is a branch choice. Inside
  a canopy it is full open-ground steering. The design must work in all
  three.
- **Laden and empty ants need different navigation.** Research gives the
  laden ant a home vector and the empty ant a trail to follow with direction
  from knowing where home is. The shipped empty ant has **nothing that aims
  it**, and the trail it reads (B) has a gradient that points back at the
  nest.
- **The plan:**
  1. Instrument first, with per-tick traces as the primary evidence and
     counters reconciled against them.
  2. Build six small scenes (S0–S5) with written predictions.
  3. Investigate why drops are blocked before changing the drop.
  4. Replace the cone-plus-re-roll with **one chooser over the moves the ant
     can actually make**, scored by turning preference, a home term and a
     trail term. The trail is read where the ant would step and integrated
     over time. The chooser's weights are brain outputs, so they can evolve.
     **Build it in two stages** (§4i): turning preference and the home term
     first, judged on exploring and getting home; then the trail terms.
- **Exploring matters as much as getting home.** A fed empty ant on flat
  ground is predicted to reverse as often as it steps (S0), which fits the
  line's standing finding that discovery of food is the binding constraint.
  S0 is the first scene, and exploration is the first thing the new chooser
  is judged on.

## 1. Owner rulings, 2026-09-22

| Ruling | |
|---|---|
| The revised design (§4) | agreed |
| Brain outputs for the chooser's weights from the start | agreed; re-deriving every species' `mutation_rate` is accepted, since most creatures will be updated after the ants anyway |
| Food memory for empty ants | **no, for now** |
| Counters reconciled against per-tick traces | at first implementation, and whenever results are confusing, unexpected, or an issue has dragged on; not on every run |
| Letting a drop reach past the eight neighbours | **deferred**: first understand why ants are blocked (§8) |
| Traces | per-tick brain, decision, position and environment traces have been more informative on this line than counters, so both are used, with the trace primary |
| Revisions after the living reference was written | agreed, and folded in: exploration first (§0, §6 S0); the chooser in two stages (§4i); the home pull set by distance, not crop fill (§4a); falling independent of the step roll (§4e); the drop treated as possibly a nest-digging problem (§8); the trail-B experiment narrowed to its one essential arm (§7) |
| Digesting the crop while carrying it | **not changed.** A test of `digest_hunger_weight` waits for the owner's go-ahead (§8b) |

## 2. Corrections to the record

**2a. `what-controls-creature-movement-2026-09-22.md` §7's *"`home_bias`
does not visibly bias the re-roll"* is very likely a measuring artifact.**

- **The test's blind spot.** It identified a tumble as *"heading changed, no
  displacement"*. That cannot see the homeward re-roll's most common
  outcome: re-picking the heading a laden ant already has, because it
  already faces home.
- **The wrong baseline.** It compared against 37.5% ("uniform over 8"). The
  re-roll is uniform over usable headings; on flat ground that is 2, where
  chance is 50%.
- **The firing counter it said does not exist does exist**:
  `creature_stats.tumbles_homeward`. On 2026-09-20
  (`ant-navigation-plan-2026-09-20.md` §4b, 36 seeds) it read 10.5% of all
  tumbles at `home_bias` 1.0, and `carry->nest` doubled, 123 → 254.5 (34
  better / 2 worse / 0 tied).
- **The archived per-ant trace disagrees with the claim.**
  `data/laden-census-seed1-gap90-2026-09-21.csv.gz` covers 93,860 laden ant
  frames on one seed. Of those frames, `HomeAligned` reads 0 on 50,618
  (mostly on the anchor), negative on 24,870 and positive on 18,372.
  - There are 659 episodes of facing away from home. **Their median length
    is 18 frames, 3 decisions**, and p90 is 48 frames. That is consistent
    with a working re-roll.
  - **50 of them last 10 or more decisions, with the ant standing in one
    cell, and those 50 hold 56% of all facing-away time** (13,986 of 24,870
    frames).
  - Episodes end facing home 353 times and at exactly 0 (perpendicular, or
    back on the anchor) 296 times.
  - **One seed**, so this rules out "the lever is inert" without proving it
    works; the positive control (§6) settles it.

The long tail has a concrete candidate, predicted in S2 and S3: the way home
is obstructed, the argmax keeps re-picking the same least-bad heading, and
`(HomeAligned, Move, 3.0)` forbids stepping in any direction that points
away from home.

Reproduce: read the CSV, group rows by `id`, and measure runs of consecutive
frames with `HomeAligned < 0`.

**2b. The `PIXEL_PHYSICS_HOME_TARGET=nest` arm (`ant-forage-bed-and-gates-2026-09-21.md`
§8f–§8g) moved only half the wiring.** The `HomeAligned` sensor follows
`home_target()`, but `home_weighted_pick` still aims at `forage_anchor`. In
that arm the throttle and the re-roll aim at different places, so its null
result does not mean what it says.

**2c. `ant-sim-research-review-2026-09-19.md` §2.11 says `Persist` is the
ant's reversal rate on a flat floor. It is not.** On flat ground only
straight ahead survives the cone (§R4), so `Persist` does nothing there. The
reversal rate is (1 − p_move) × 0.5 × ½: the failed roll, the tumble chance,
and a uniform pick between forward and back.

**2d. The hypothesis raised while planning, that laden ants dig themselves
into pockets, is wrong.** `act` returns from the drop branch whenever the
crop holds food, so a laden ant never reaches the dig code. The
`packedsoil` that boxes in laden ants is **burrow lining**: `line_burrow`
turns the soil around every dug cell into `packedsoil`, and the digging is
done by **empty** nestmates, whose dig urge rises with crowding at the nest
(hidden units 5–6). §8 restates the hypothesis accordingly.

## 3. The mechanism against the research

Sources: the external survey
(`ant-sim-literature-review-external-2026-09-19.md` §3, §4, §6), our review
of it (`ant-sim-research-review-2026-09-19.md` §2.5, §7) and
`ant-navigation-plan-2026-09-20.md` §2. Items marked *(outside our reports)*
come from the wider literature and should be checked before anything is
built on them.

### 3a. By setting

| Setting | Usable moves | What today's side sensors read (6 cells out on the forward diagonals) | Research equivalent |
|---|---|---|---|
| Flat ground, 1-wide tunnel, wall face | forward or back | Surface: sky and rock. Tunnel: rock, plus scent leaked through rock, since the plane ignores terrain | a 1-D trail: keep going or turn around |
| Corners, ledges, stem bases, tunnel junctions | 3–5 | mixed | Deneubourg's branch choice (the double bridge) |
| Inside a canopy | up to 8 | tissue that may carry trail, but a point 6 cells out need not be reachable | open-ground trail following (osmotropotaxis), the case the literature describes |
| Nest / burrow | 0–1 | — | delivery; real foragers mostly unload near the entrance *(outside our reports)* |

Every foraging-bed measurement so far comes from rows 1 and 4, while in the
outdoor game fruit grows on plants, so reaching food means climbing (rows 2
and 3).

### 3b. By leg

| | Research | Shipped ant |
|---|---|---|
| **Laden** | The home vector steers; a long vector dominates, a short one defers to local cues. Lays recruitment trail, most near the food (22x within 10 cm; Czaczkes 2024). Right of way in head-on meetings (~80%, *Atta*) | Home direction is a permission to step, plus a homeward re-roll when stopped. Trail A is read as a throttle. Trail B is laid at a constant rate, so the most goes where ants stall. Pays more per step, but does not step less often |
| **Empty** | Naive ants explore with a persistent (correlated) random walk. Recruits follow the trail, with direction from path integration (walk away from home). Trail plus memory makes them ~25% faster and ~30% straighter (Czaczkes 2011). Unsuccessful ants return | **Nothing aims it.** Re-rolls are uniform. Trail B is read as a throttle, and its gradient points at the nest. On flat ground, stepping and reversing are equally likely per decision (S0) |

**Kept:** the split of who lays and who reads each trail. Laden ants lay B
and read A; empty ants lay A and read B. That is the standard two-pheromone
arrangement and it already keeps each ant off its own trail. **Changed:**
how each leg reads its trail, and giving the empty ant a direction.

### 3c. Reading the trail over time

Real ants compare scent across the body (one antenna against the other:
Hangartner 1967 *(outside our reports)*; Perna 2012) and over time (now against a moment ago, as
they walk and sweep their antennae; Draft et al. 2018 *(outside our
reports)*). Ants with one antenna removed still follow trails, with more
zigzag. Receptors adapt to the background, so behaviour follows contrast
rather than level. In our corridors the time comparison is the natural one:
the ant's own cell is always a legal place, never rock or sky.

**This was tried once and rejected** (`dead-ends.md`, *"hidden unit 7 as a
temporal comparator"*, 2026-09-19, a 36-seed coin flip). It failed for three
reasons this plan avoids:

- It was built from the brain's squashing neurons, which cannot hold an
  exact running average: the unit read 0.036 for an input of 0.100, so it
  measured *level* rather than *change*.
- It was given to laden ants reading trail A, under their own feet.
- It fed `Move`.

Its own re-test condition, terrain the spatial read cannot handle (a trunk
or a tunnel), is exactly what scenes S2–S5 build.

## 4. The design

Everything below goes behind one switch, opted into by the ant's wiring.
With the switch off, every species' movement code path is unchanged. Adding
brain outputs changes `live_slots` and so every species' `mutation_rate`,
which moves breeding scenes from birth one. The owner has accepted that.

### 4a. One chooser over the usable moves

Each decision, score every usable heading (the predicate `tumble` already
uses: enterable and footed) and pick with `choose_weighted`, keeping its
randomness. The score is the sum of:

- a **turning preference** by the angle from the current heading: straight
  preferred, reversing least likely. Its sharpness comes from `Persist`, and
  `Turn` keeps its left/right bias;
- a **home term**: gain × the cosine to the home vector, weighted by the
  vector's length. It switches on when the ant carries **anything**, and is
  **not** scaled by how full the crop is. Today's re-roll scales by fill, and
  the ant digests its cargo on the way home, so the longer it has been lost
  the less it is steered home, which is backwards;
- a **trail term** per plane (§4b);
- **footing**, from `Caution`, as today.

How this behaves in each setting:

- **Flat ground and tunnels:** it becomes the keep-going-or-turn-around
  decision, and turning around becomes something the ant can choose.
- **Junctions:** it is Deneubourg's branch choice.
- **Canopy:** it is full steering.

§R4 disappears, because the ant no longer depends on the three-cell cone.

### 4b. Where the trail is read: spatially, where the ant would walk

For each usable heading, sample the scent at the 1–2 cells the ant would
step into. That is about an antenna's reach for a 2-cell ant, and it
replaces the fixed points 6 cells out that fall in rock or sky.

### 4c. And over time, at the ant's own cell

The engine keeps two exact running averages per plane per ant: a short one
("now") and a long one ("background").

- **When they update:** only on a step, reading the new cell **before** the
  ant deposits there, so its own mark never enters the history.
- **What the brain gets:** one new input per plane, `(short − long) / (short
  + long + guard)`.
- **What it drives:**
  - falling means the trail is being lost, so turn more and consider
    reversing;
  - rising means go straight;
  - the long average also rescales §4b's spatial samples, so a faint trail
    in a quiet area and a strong one near the busy nest both read as "the
    route".
- **What it replaces:** it supersedes `PheroARise`, which is the old
  comparator's input.

### 4d. The terms differ by leg

- **Laden:**
  - Home gain is positive, and it dominates when far from home.
  - The trail term reads A, most useful near the nest, where the home fix is
    weakest.
  - It never reads B, its own trail.
- **Empty:**
  - A weak "away from home" gain gives *which way* along a route.
  - The trail term reads B as a **saturating presence** ("is this branch a
    route"), not "which end is stronger". That makes B's backward-pointing
    gradient irrelevant.
- **Constraint:** on a single line of trail, the home term must beat the
  trail term, or the backward gradient returns through the forward-versus-back
  comparison.

Leg switching is by **direct wires from `CarryingFood` to the gains**, not by
gated hidden units: a shut hidden unit is not neutral (`squash(−45) =
−0.978`; `ant-return-leg-result-2026-09-20.md`).

### 4e. Turning without stopping

- **A bad reading means "turn more", never "freeze"**: `Tumble` becomes the
  chooser's randomness.
- **Trail presence gives a modest speed increase**, and the gradient
  throttle on `Move` (hidden units 0–3) is retired.
- **`(HomeAligned, Move, 3.0)` becomes a speed adjustment with a floor**, so
  an ant can walk around an obstacle.
- **Falling stops depending on the brain.** Today the support check runs
  only inside `step_chain`, after a successful step roll. So an ant whose
  footing is dug away with `P(move) = 0` hangs in the air, and a fall counts
  as a move and lays trail. The check runs every tick regardless of the roll,
  and a fall is not a move. This belongs in stage 1, because stage 1 changes
  when ants step.
- **`Move` becomes an activity level only.** Once navigation leaves it, what
  remains is how active the ant is. Today that is set almost entirely by one
  clamp: `Energy` reads `energy / 200` capped at 1, so every fed ant reads
  1.0 and steps at 0.20 per decision.

### 4f. New brain outputs

A signed home gain and one trail gain per plane (names provisional). The
existing `Persist`, `Caution`, `Turn` and `Tumble` stay, with their jobs
redefined behind the switch.

### 4g. Rejected alternatives

| Alternative | Why not |
|---|---|
| Wire the existing lateral sensors into `Turn` and fix §R4 with in-place rotation | keeps 45° per step; reads sky, rock and leaked scent; still cannot turn around |
| Home vector only; trails used just to choose the direction out of the nest | drops the emergent trail network the owner wants |
| Pathfinding or flow fields | against the engine's stigmergy-first design |
| Food memory | owner ruling, not now |

### 4h. Costs and risks

- **`step_chain` is shared by every walking creature**, hence the switch and
  a check that the switch-off path is unchanged.
- **Frame cost.** Checking up to 8 headings per decision instead of 3 has a
  cost that must be measured with `examples/ascii` before it ships.
  Decisions happen once per 6 frames per ant.
- **Constants calibrated against today's behaviour** must be re-derived as
  part of the work, not inherited:
  - the `Move` bias and `Energy` weight, including whether `Energy` should
    still saturate at `start_energy`;
  - `DEPOSIT` and the trail fade rates;
  - `home_bias`, `Stillness`;
  - every constant sized against the throttle.

  Each stage in §4i re-derives the ones it touches.

### 4i. Two stages, each judged on its own scenes

| Stage | What it adds | What it replaces | Judged on |
|---|---|---|---|
| **1** | the chooser with turning preference, home term and footing; falling every tick | the random re-roll, the homeward argmax, `(HomeAligned, Move, 3.0)` as a gate | S0 (how far an empty ant gets, and how quickly it finds food), S1, S2, S3 |
| **2** | the trail terms, spatial (§4b) and temporal (§4c) | the trail-gradient throttle (hidden units 0–3) | S4, S5, then one paired run on the colony bed |

**Why two stages:** the turning preference alone should change exploration a
lot. Adding the trail at the same moment would make it impossible to say
which change did what, and would re-derive the stepping constants twice at
once. Stage 1 still has to keep laden ants aimed, so the home term cannot
wait for stage 2.

**What stage 1 cannot show on the colony bed.** Step 1 found empty ants
frozen by trail B's throttle on most of their decisions (census report §4).
Stage 1 leaves that throttle in place, so on the bed the colony will still
look frozen after stage 1, and a bed run then would say nothing about stage
1 itself. Stage 1 is judged on the scenes only.

## 5. Measuring: traces first, counters reconciled against them

**The rule.** Every counter below gets two checks at first build, and again
whenever results are confusing:

- **Agreement.** It must **match the per-tick trace exactly** on the same
  run; if they disagree, one of them is wrong.
- **A positive control:** a scene whose answer is known, run to confirm the
  counter reports it.

The trace covers **every** ant that reaches the state in question, not one
focal ant. Each row carries the inputs, the choice made, and the reason (the
gate that decided it).

**What already exists.** `trailfollow trace` and `ladencsv` hold per-tick
position, emits, the scent under and ahead of the ant, `HomeAligned`, the
along readings, `p_move`, `drop_urge` and `free8`. `onetrail` is a bare slab
with one ant, no nest, food or colony, and optional decay. There are also
`creature_stats.tumbles_homeward`, `p_move_hist`, and the blocked census
(`PIXEL_PHYSICS_BLOCKED_CENSUS`). **Extend these; do not build a new
harness.**

| | Counts | Trace columns that must reconcile with it |
|---|---|---|
| **C4** (first) | each decision's outcome (stepped / tumbled / nothing / blocked / reversed / fell), by **leg** and by **setting** (the number of usable headings: 2 corridor, 3–5 junction, 6–8 open, 0–1 pocket) | leg, setting mask, roll results, branch taken |
| **C1** | homeward re-roll: calls; exits by gate (no usable heading, not carrying, fill 0, on the anchor, roll failed); firings; **whether the chosen heading pointed toward home, across, or away** | the same, per tumble; and the firings must equal the old `tumbles_homeward` |
| **C2** | drop: rolled and lost; rolled, won and placed (at the nest or not); won with nowhere to put it | the same, plus the roll, `free8` and the eight neighbours' materials; placed must equal `drops`, placed at the nest must equal `deliveries` |
| **C3** | the forward cone: which candidate was taken (left, straight, right), and **`Turn` requests discarded because the side asked for scored 0** | `Turn`, the three final scores, the pick; the picks must equal `stepped` |

C4 answers the owner's point directly: **where laden and empty ants actually
spend their decisions, and where each one stalls.**

**Re-scoped after step 1**, because the trace showed three things:

- **C1 is mostly built already.** The trace carries every homeward reason
  and the chosen heading's cosine. What is left is a counter to reconcile
  against it, and the old `tumbles_homeward`, which the counter must match.
- **The drop is decided in `act`, before the move, so the move trace cannot
  see it.** C2 extends the same row with the drop's result, rather than
  adding a second trace.
- **`Turn` is far too small on the bed to steer anything.** Its only wire
  is temperature, which stays near ambient. Over 72 runs its largest value
  is 0.031, and it reaches 0.001 on 0.38% of gap-90 decisions and almost
  never at the other gaps. That raises a side candidate from 0.6 to at most
  0.63. So a count of "Turn requests discarded" on the bed would count
  requests that could not have turned anything, and C3 as first written
  could not have told us much. C3 now records the cone's three scores and
  which one was taken, which explains the direction of every step in
  S0–S3. The discarded-Turn count stays, beside the largest `Turn` seen,
  with a positive control that feeds `Turn` in directly.
  *(This bullet first said `Turn` was exactly 0 on all 69,836 decisions of
  one run. That was the CSV printing it to four decimals: nonzero values
  below 0.00005 read as 0.0000. The CSV now prints it in full.)*

## 6. Scenes, each with a written prediction

**Built on `onetrail`'s pattern and the `creature.rs` test-world helpers**
(`test_world`, `spawn`, `register_nest_site`, `STONE.with_attached(true)`),
run through the real scheduler and the parallel driver the app uses.

### Setup checklist, asserted in code at setup

- **Materials:** walls of **undiggable** material; no free soil, since soil
  is a powder that slumps and ants dig it.
- **Home:** `forage_anchor` set explicitly, because it defaults to the spawn
  cell and re-anchors on every nest contact. Nest material only where the
  scene intends it.
- **Terms that drift:** `Stillness` (saturates after 192 still ticks),
  `Energy` (a hungrier ant moves more), and digestion shrinking the crop.
  Pin them, or log them every tick.
- **One ant**, unless crowding is the subject: `Crowding` and `KinNeed` are
  in the sum.
- **Ambient temperature**, since temperature is the only wire to `Turn`.
- **Arms:** trail laying on and off, which separates reading its own mark.
  Starting facing home and facing away, as separate arms.
- **Setup:** away from world edges; pace pinned, so decisions are exactly 1
  in 6 frames.
- **Runs:** at least 24 seeds, paired, reporting distributions; print the
  parsed key's cardinality.
- **The prediction below is written before the scene is run.**

### The scenes

**S0. Flat slab, empty ant, no trail: how an explorer moves.**
Prediction from the formula: at `Energy` 1, each decision is a step with
probability 0.2, a reversal with 0.2, a same-heading re-roll with 0.2, and
nothing with 0.4. **Stepping and reversing are equally likely, so an empty
ant jitters rather than explores.** At `Energy` 0.5 it steps 0.53 and
reverses 0.12. Research expects long runs. This is the baseline the turning
preference must change. **Run first.** Measure distance covered per decision,
and, on a flat slab with food 90 cells out, time to first find it. This is
the first thing stage 1 is judged on.

**S0 tests a corridor, not the bed's usual setting.** A bare slab gives two
usable headings, east and west. On the bed, 55% of decisions are at
junctions, and most junctions there offer an upward move: a wall, a burrow
mouth, or a nestmate to climb. The cone behaves differently there. With
`Turn` near 0, a footed side candidate scores 0.6 against straight's 1.6, so
straight is taken about 75% of the time and each side about 13%. S2's wall
base is the nearest single-ant scene to that.

**S1. Flat slab, laden ant, home 40 cells away** (no nest material, so no
trail A is laid and the trail throttle reads 0).
Prediction: it works today. There are two usable headings; facing away gives
`P(move) = 0`, a tumble at 0.5 per decision, and the re-roll re-aims it at
chance `fill`. Facing home it steps at 0.76 per decision, and a failed roll
re-picks the same homeward heading. **It should arrive in about 50–60
decisions (300–360 frames), with the facing-away start about two decisions
slower.** If it fails, the fault is below the colony bed. If it passes, the
slow return measured on the bed comes from terrain.

**S2. The same, with a wall of height 1, 2, 3, 6 or 12 between it and
home.** In side view, a detour means climbing over.
Prediction: **every extra cell of height divides the chance of getting over
sharply, and from height 4 the climb is forbidden outright.**
- At the base, the re-roll picks straight up (cosine 0 beats straight back),
  and the ant steps at 0.2 per decision.
- On the face the only usable moves are up and down. Climbing points it away
  from a level home, because the home vector gains a downward component, so
  `P(move)` upward falls to 0.15, 0.09, 0.02 and then 0 at heights 1 to 4.
- Every failed roll gives a 0.5 × `fill` chance that the re-roll's argmax
  picks **down**.
- So the chance of gaining the next cell before being turned back is about
  1 in 4 at the first cell, 1 in 6 at the second and 1 in 20 at the third.
  After that, only `Stillness` gets it over.

This directly tests §2a's long-tail explanation. Hold the seed fixed per
height, so arms stay comparable.

**S3. A U-bend tunnel whose exit requires walking away from home first**
(home lies beyond the tunnel wall).
Prediction: **trapped.** The leg away from home is throttled to 0, and the
re-roll keeps aiming it at the wall. This is the obstruction deadlock in its
purest form. Run with the reversal rule on and off, or the rule answers for
the steering. The existing dead-end test (`a_laden_ant_in_a_dead_end_corridor_…`)
is the plain version, and should pass.

**S4. Junction, empty recruit, trail B on one branch.**
Two forms: a stone tunnel fork, and a stone pillar (climb versus continue),
so nothing grows during the run. Swap which branch carries the trail,
because climbing and level ground have different base rates. Include a
no-trail control.
Prediction today: **the first branch choice matches the no-trail control**,
because the cone has no pheromone term. Only the time spent on the trailed
branch differs.

**S5. An open stone lattice, where every cell gives footing; then a real
canopy for short runs.**
Prediction today: a laden ant exits homeward reasonably, since the throttle
plus the re-roll act as a crude aim. An empty ant following B through the
lattice matches the no-trail control. The real canopy comes second, because
of growth, day/night light, and walking through tissue.

## 7. Trail B: who reads it, and the experiment that says what it does

**Established from the source** (details in `how-the-ant-works.md` §7 and
§11):

- **Who lays it:** only laden ants (`CarryingFood → EmitB`).
- **Who reads it:** only empty ants, through the forward difference, into
  hidden units 2–3, into `Move`. That is, *whether* to step, never *which
  way*.
- **What nobody reads:** its strength (`PheroBFront`) and its sides
  (`PheroBLateral`), in any species. Nothing in the engine outside the brain
  reads it.
- **The other eight species files** saturate the reader to nothing, and lay
  B on `Carrying`, which includes spoil.

**Suspected harms, neither separately established:**

- **Direction:** the gradient points at the nest, so recruits get pushed
  home.
- **Trapping:** a gradient-follower stalls at local peaks. §7c of the bed
  report found the stall sitting on the trail's hotspot.

**Evidence of net harm:** muting B takes second laps 87 → 182, better in 19
seeds of 24. Muting removes both laying and reading, so it cannot say which
one hurts.

**The experiment** (folded into step 1 of §9). **The question that still
matters is whether laying B hurts on its own**, because the way B is read
today is being replaced in stage 2 anyway, while laying carries forward into
the new design.

- **The essential arm:** reader off, laying on (the two `PheroBAlong` wires
  zeroed), paired against shipped. Nobody has run it.
  - If it matches shipped, the harm is in the laying (beacons where ants
    stall, congestion), and stage 2 inherits it.
  - If it matches muted, the harm is in the reading, which stage 2 removes.
- **Optional, if the essential arm is ambiguous:** trace every empty ant on
  trail-B cells, every decision: position, facing food or nest, B underfoot
  and ahead, units 2–3's contribution to `Move`, `p_move`, and outcome.
  Predictions:
  - **Direction:** on the trail, facing food reads negative and facing the
    nest positive.
  - **Trapping:** stalls cluster at B's local peaks.

**Setup:** a hand-laid trail for the direction test, since its ramp is
known. The colony's own trail for the harm test. The bed corrections from
`ant-forage-bed-and-gates-2026-09-21.md` §2 apply
(`PIXEL_PHYSICS_COLONY_SPACING=2`, `layfrom=founders`).

## 8. Drops: understand the blockage before changing the drop

**Known** (`ant-forage-bed-and-gates-2026-09-21.md` §10, §12): on 60.8% of
the ticks a laden ant wants to drop, no neighbour is empty. The drop's roll
is spent before it looks, and the failure is recorded nowhere. What is in
the way is `packedsoil` 24–56%, nestmates 26–38%, and the rest soil, nest
material, spoil and fruit.

**Five explanations**, each predicting a different answer to *"what filled
the eight cells around the ant, and when"*:

| | Hypothesis | Prediction |
|---|---|---|
| H1 | The colony's diggers made the pocket: empty nestmates dig when the nest is crowded, and each dig lines the walls with `packedsoil` | neighbours that were soil at frame 0 and were turned into `packedsoil` by digging |
| H2 | The nest is made of material, so being "at the nest" means touching nest cells, with little void | failures sit where nest material is densest |
| H3 | Earlier drops of food or spoil filled the free cells | neighbours changed from their frame-0 state by drop events |
| H4 | Nestmates | creature cells |
| H5 | The home fix leads laden ants underground | the last nest cell touched is below the surface |

**The census.** Start at the anchor: 52.5% of laden tumbles happen there
(census report §6), and the bed report already puts failed drops there.
Trace every laden ant from pickup to drop or loss. At each
failed drop, classify every neighbour by comparing it against the world at
frame 0 and the dig/drop event log, and record the same for successful
drops. Then run one controlled scene: a nest with a known empty chamber,
with digging on and off.

**It may be a nest-digging problem rather than a drop problem.** From the
source:

- Empty ants dig when the nest is short of roofed room per ant (the target
  is 2 cells per ant), and every dug cell turns the soil around it into
  `packedsoil`.
- Every successful drop fills one free cell.

So the colony builds pockets sized for ants, with no space set aside for
food. If H1 and H3 carry the census, the fix is probably in how the nest is
dug (chambers with open space) or where food is handed over, not in how far a
drop reaches. From outside our reports: real foragers mostly unload near the
entrance, into chambers or to nestmates.

### 8b. An open question for the owner: digesting the crop while carrying it

In real ants the crop is a "social stomach", whose contents are mostly not
digested on the way home *(outside our reports; check before building on
it)*. Here digestion runs every tick the crop holds food, and
`ant-return-leg-result-2026-09-20.md` found that makes delivery "giving up
your supply", which costs the colony its second trips. A switch already
exists: `digest_hunger_weight`, which makes digestion wait for hunger, is
authored at 0.0 for the ant. It is a colony-economy change rather than a
movement one, so **it is not tested without the owner's go-ahead.**

## 9. Order of work

1. **C4 and the full trace, on today's code, today's beds.** Trace only; no
   behaviour change. Includes §7's essential trail-B arm. **Done 2026-09-22:
   [`ant-decision-census-2026-09-22.md`](ant-decision-census-2026-09-22.md).**
   Empty ants are frozen on trail-B peaks, and the reader-off arm is
   confounded on the ramp bed (that report's §7a).
2. **C1–C3**, as re-scoped in §5, each reconciled against the trace and
   given a positive control. At the same time, fix the four contradicting
   source comments listed in `how-the-ant-works.md` §14. **Done
   2026-09-23: census report §10.** At gap 90, 52% of the drops a laden ant
   wins at the nest find no room, mostly because of nestmates and burrow
   lining. The four comments are fixed, and a fifth of the same kind was
   found and fixed.
3. **Scenes S0–S5 on today's code**, **S0 first**, with the predictions
   above, as the baseline.
4. **The drop census (§8)**, in parallel with step 3.
5. **Chooser stage 1** (§4i), behind its switch: turning preference, home
   term, falling every tick. Judged on S0–S3.
6. **Chooser stage 2**: the trail terms. Judged on S4–S5, then one paired
   run on the colony bed.

Each of steps 5 and 6 updates `how-the-ant-works.md` in the same change.

## 10. What this rests on

- **Source:** `creature.rs` (`creature_tick`, `sense`, `act`, `step_chain`,
  `tumble`, `home_weighted_pick`, `home_target`, `line_burrow`),
  `brain.rs` (`eval_brain`), `pheromone.rs`, `organism.rs`,
  `assets/species/*.ron`, read at `bb65d507`.
- **Data:** `Reports/data/laden-census-seed1-gap90-2026-09-21.csv.gz` (§2a).
- **Reports:**
  - [`how-the-ant-works.md`](how-the-ant-works.md);
  - [`what-controls-creature-movement-2026-09-22.md`](what-controls-creature-movement-2026-09-22.md),
    superseded by the former;
  - [`ant-forage-bed-and-gates-2026-09-21.md`](ant-forage-bed-and-gates-2026-09-21.md);
  - [`ant-navigation-plan-2026-09-20.md`](ant-navigation-plan-2026-09-20.md);
  - [`ant-return-leg-result-2026-09-20.md`](ant-return-leg-result-2026-09-20.md);
  - [`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md);
  - [`ant-sim-literature-review-external-2026-09-19.md`](ant-sim-literature-review-external-2026-09-19.md);
  - [`dead-ends.md`](dead-ends.md), the temporal comparator entry.
