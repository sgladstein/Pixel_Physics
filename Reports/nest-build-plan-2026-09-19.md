# Building the nest — a staged plan

*2026-09-19. Written at the owner's request after he asked whether the nest
the two research reports describe is possible in this engine, and for the
steps. **Docs only**: no `src/`, `assets/`, `examples/` or test change, and
§7 is about why — the files this plan touches took 56 landings in the last
seven days.*

**Rests on** [`nest-biology-2026-09-19.md`](nest-biology-2026-09-19.md) (what
a nest is for, and the owner's ruling that it has no purpose here yet) and
[`nest-biology-digging-signals-2026-09-19.md`](nest-biology-digging-signals-2026-09-19.md)
(what ants dig by). **Neither is repeated.** This is the build plan those two
imply, priced.

## 0. The verdict

**Yes, and it is cheaper than either research report implied — because four
things both of them treated as needing construction are already in the tree.**
The plan is four small stages and one new behaviour, and the first stage is
one weight in one file.

**The one-line diagnosis it is built on**, from the digging-signals report:
*the nest is a shallow lens because nothing tells an ant which way is forward
and nothing tells it when to widen.* This plan gives it a forward (§4.1–§4.2)
and a when-to-widen (§4.3).

---

## 1. What was verified in the tree, and how

Four claims that change the plan's shape. Each was checked in the source, not
inferred from a report.

| claim | check | verdict |
|---|---|---|
| **The dig target is already directional** | `sed -n 8716,8720p src/sim/creature.rs` | **True, and it is the load-bearing finding.** `let heading = …s.heading; let (dx,dy) = DIRS[heading as usize]; let (tx,ty) = (x+dx, y+dy);` — **there is no target selection at all.** An ant digs the one cell it faces. So "which way is forward" is already expressed as `heading`, and `Turn` is a live output every species wires |
| **A dug void already stays open** | `sed -n 9493,9513p src/sim/creature.rs`; `assets/materials/packedsoil.ron` | **True.** `line_burrow` converts all 8 neighbours of a dug cell to their `packs_into` material, and `packedsoil` is `self_supporting` *"so that a tunnel roof cannot fall in"*. **`burrow_probe`'s "gallery gone in 5 frames" is a hand-carved void with no lining** — not what an ant produces. This is the gate that could have killed the whole plan and it is already passed, which is why the coordinator's box holds a standing lens rather than nothing |
| **`Persist` is unwired** | `grep -rn "Persist" assets/species/*.ron` | **True** — every hit is a comment explaining why it is *not* wired. Its own doc calls it the number deciding whether a creature commutes or mills, i.e. heading maintenance |
| **Contents already exist without eggs** | `grep -n "live_seed" src/sim/creature.rs:8753`; `seeds_carried` | **True.** Seeds are picked up, carried and set down, and the dig verb **explicitly refuses** to take a `live_seed` as spoil (`world.dig_diverted_seed += 1`). So a set-down seed is a persistent object an ant put somewhere — which is what §4.3 needs |

**Why the first row matters so much.** It explains, cleanly and after the
fact, why both dig-*target* rules in `dead-ends.md` failed across their whole
ranges: **they weighted *whether* to dig, not *where*, because where was
never a choice.** Weighting the roll by how buried or how covered the target
was could only ever change the rate. The entries' shared re-test note —
*the colony already tunnels* — is consistent with this and neither entry
needs reopening.

---

## 2. What is genuinely missing

**One sense.** Nothing in the dig decision is oriented with respect to
gravity, depth, or the axis of an existing tunnel. The digging-signals report
found gravity plus the angle of repose to be the measured driver of tunnel
direction (*PNAS* 2021: piecewise-linear descent, near-vertical at the top,
at or below ~40° in the bulk, **downward from the surface and upward when
started mid-medium**), and this engine has gravity, `Powder` soil and a
two-angle repose model in `update.rs` — and no way for an ant to read any of
them.

**Everything else the plan needs is a wire or a value at a call site.**

---

## 3. A correction to the digging-signals report's ranking

**That report ranked `Persist` third and called it "the knob you happen to
have". With a heading-directed dig target that was wrong-headed, and the
coordinator's instinct was better than my ranking.**

- **`Persist` is the *straightness* half.** An ant that holds its heading
  digs a line; an ant that re-rolls its heading every tick digs a lens. That
  is not a substitute for direction, it is the other half of it.
- **A gravity-biased `Turn` is the *direction* half.**
- **They are complementary, not competing**, and the report presented them as
  alternatives. Its §5.2 conclusion — that persistence is documented as what
  operates *"in the absence of external effects"*, which a tunnel is not —
  stands as a statement about the biology and was the wrong basis for a
  ranking about this engine, where heading is the only thing the dig verb
  reads.

The ordering in §4 reflects the correction: straightness first, because it is
free, and direction second, because it is not.

---

## 4. The stages

Each stage is independently visible, has a check that **can fail**, and names
what it costs. They are ordered cheapest-first, and **every one of them is a
`review.py` card** — this is a judge-by-eye subsystem and the owner's
playtest reports have overturned three models that all looked correct in
tests.

### 4.0 Stage 0 — the control, before anything changes

**Three of the four facts in §1 are source reads, not measurements.** A scene
that contradicts the code looks like a bug in the code, and this plan would
be built on all four.

Confirm in the running box: lining actually on (`PIXEL_PHYSICS_BURROW_LINING`
defaults on — check it is not off in the harness), the dig actually following
`heading`, a set-down seed actually persisting past germination, and **the
current void's aspect ratio as the baseline every later stage is read
against**.

- **Cost**: one probe run. No code.
- **The check that can fail**: the coordinator measured ants spanning
  46 columns × 2 rows. If the standing void is *not* wider than it is deep,
  the premise of the whole plan is wrong and Stage 1 has nothing to fix.
- **Positive control**: the lining ablation switch exists
  (`PIXEL_PHYSICS_BURROW_LINING=off`) and changes nothing else. Run it once —
  if the void is the same with lining off, lining is not what holds it open
  and §1's second row is wrong.

### 4.1 Stage 1 — straightness: wire `Persist`

The smallest change in the plan and the one most likely to be
disproportionate. **One weight, one species file, no new code.**

- **What**: give a digging ant a reason to hold its heading, through the
  already-free `Persist` output.
- **Cost**: **zero** new slots, zero `mutation_rate` change, zero baselines
  voided. It is an authored weight on an output row that already exists and
  is already zero.
- **The check that can fail**: **aspect ratio of the standing void** against
  Stage 0's baseline. Success is a void longer along one axis than the other.
  Today's is 46 × 2 the wrong way round.
- **What it does not do**: it cannot choose a *direction*. A persistent
  heading with no bias digs a straight tunnel in a random direction, which is
  progress and is not a nest. Expect a horizontal gallery, not a shaft.
- **Risk**: a straight tunnel that instantly refills reads identically to no
  tunnel. Read `roofed` (which plateaus), never `cells lost` (which never
  settles) — the cascade rule.

### 4.2 Stage 2 — direction: one sense, wired to `Turn`

The expensive stage, and the cost is nameable, which is what makes it
scoped.

**What the sense should be. Two candidates, and the second is the one to
build:**

- **(a) Signed depth below the founding surface.** Simple, and the datum
  already exists (`room_surface` computes it per column). **But it is a
  global scalar**, which is the thing both research reports argue against:
  every ant at a given depth reads the same number, and a global reading
  cannot produce a spatial pattern.
- **(b) Local vertical asymmetry of enclosure** — curvature or occupancy
  sampled above the head against below it. **This is the one to build.** It
  is the same family as `SurfaceCurvature`, which is the only input that
  varies per ant (16 distinct values among 51 ants) and the only intervention
  that has worked (2.3x roofed chamber). It is a lattice count, not a field
  read, so it has none of the block-nearest degeneracy that has produced four
  bugs on three lines.

**The cost, priced from `brain.rs`'s own doc:**

- **An input column is 24 live slots.** `live_slots()` rises, and **every
  species' `mutation_rate` must be re-derived in the same change** — there is
  a guard named for exactly this
  (`the_live_slot_count_is_pinned_because_mutation_rate_is_derived_from_it`).
- It **changes the draw sequence `brain::mutate` walks**, so two births are
  not comparable across the change *even at a re-derived rate*.
- `random_genome` draws more values, so **every `creature_space` baseline
  taken before this is void.**

**That is the budget, and by `CLAUDE.md`'s rule it is part of the work rather
than scope creep. It is finite and it can be named, which is the test.**

**A cheaper route considered and rejected**: repurpose `MoistureGrad`'s
writer, since that channel is measured inert and costs nothing to redefine.
**Rejected** — it is also wired to `Drop` in the deposition scene at a
measured 2.94x, so redefining what it computes reallocates a shared term with
no way to hold the deposition arm fixed. That is the `phototropism_dir` shape
exactly, and it trades Stage 2's finite cost for an unbounded one.

- **The check that can fail**: **depth reached**, and the **slope of the
  resulting tunnel against the ~40° repose figure**. A tunnel that descends
  at 80° is not obeying the material and says the sense is overriding physics
  rather than steering within it.
- **Build (a) as the control, not as the feature**: it is cheap, it shares the
  writer's plumbing, and if (b) shows nothing while (a) does, the local
  reading is not the discriminator I claim it is.

### 4.3 Stage 3 — widening: contents trigger a chamber

The stage that makes it a nest rather than a burrow, and the one the biology
is most emphatic about: **workers excavate only tunnels and no chambers
unless brood or fungus is relocated to a spot** (Römer & Roces, *PLOS ONE*
2014).

- **What**: a set-down seed near the site raises the dig urge *locally*, so
  the ant widens around it rather than advancing past it.
- **Cost**: uses existing seed carrying and existing contact adjacency.
  **No eggs needed for a first version** — §1's fourth row is what buys this.
- **The check that can fail**: **chamber area around seeds against chamber
  area away from them, paired inside one run.** A paired comparison, not a
  run against a remembered number: outcomes here have enormous spread and a
  single arm is a sample from a wide distribution.
- **The honest caveat**: a set-down seed germinates, so it is a **timer, not
  a permanent object**. A chamber triggered by one will lose its trigger. That
  is not purely a defect — it is also exactly the *cull the germinated seed*
  behaviour the first report's §3.1 asked for, and the two want building
  together.
- **Where eggs come back**: an egg is a better trigger than a seed on every
  axis (it does not germinate, it is the measured trigger, and it is what the
  first report's §11 argues for). **This stage is the cheap proof that the
  widening rule works; eggs are what make it stay working.**

### 4.4 Stage 4 — the pellet relay

Deliberately last, and the only item in the plan that needs a **new
behaviour** rather than a wire.

- **What**: separate the excavator from the carrier. In the biology a pellet
  leaves the face in a chain — up to ~12 workers over 2 m, short-distance
  carriers dropping after a few centimetres (Pielström & Roces 2013). In this
  engine one animal digs and hauls, which is why excavation and refill cancel.
- **Cost**: a new verb or a rework of `DropSpoil`. The dearest item here.
- **Nothing above depends on it**, which is why it is last.
- **Cheap precursor worth doing inside Stage 3 instead**: make **fresh spoil
  near the face raise the dig urge**. That is the one stigmergic cue in ant
  excavation with a positive result, `spoil` is already a distinct material,
  and it is a material adjacency test rather than a new verb. **Watch the
  interaction**: spoil `needs_footing`, and a rule that sends ants to dig at
  their own heap puts the digger under the overhang. That interaction is not
  in the literature and is ours to measure.

---

## 5. What not to build

From both research reports, with the reason each is closed rather than merely
deprioritised:

- **A digging pheromone.** Tested directly in *Acromyrmex lundi* — fresh
  digging face against one aged an hour — and **null** (Pielström & Roces
  2015). Closed on evidence.
- **A CO₂ field.** Tested from atmospheric to 10% for dumpsite choice and
  **not used**, while humidity and temperature were (Römer, Bollazzi & Roces
  2019). Closed on evidence, where the first report had closed it only on
  cost.
- **A third dig-target rule.** Burial and cover are both dead across their
  whole ranges, and §1 explains why: there is no target to prefer.
- **Any further work on the `Crowding` reading.** Three interventions each
  achieved their own precondition and moved the nest not at all, and the
  biology says why — the regulation is largely the falling chance of
  *encountering the face* as space grows, which no sensor can reproduce. If
  density is to be wired at all, wire it to **rest**, not to `Dig`.
- **An entrance-convergence rule.** The digging-signals report has the
  function of a single entrance and not its origin; anything built here would
  be invention.

---

## 6. How this plan could be wrong

- **Stage 1 may do nothing, for a reason not in this plan.** `heading` is
  set by `Turn` and by whatever else moves it; if something re-rolls heading
  faster than `Persist` can hold it, a weight on `Persist` is a term in a
  sum that loses. **Read what writes `heading` before authoring the weight.**
- **Stage 2's sense may be blind at the scale that matters.**
  `surface_curvature` *"cannot see a feature wider than its own disc"* — at
  radius 2 a five-wide slot reads convex. A chamber is wider than five cells.
  A vertical-asymmetry reading built the same way inherits that limit.
- **Stage 3's effect may be swamped by placement.** `wiki/ants.md` records
  that an ant *"sets its cargo down wherever it happens to be, not only at
  the door"*, so seeds may simply not arrive near the site often enough for a
  paired comparison to have a positive arm. **Check the arrival rate before
  building the widening rule** — that is the cheap pre-flight, and it is the
  *which pixels does this lever move* question asked early rather than late.
- **All four stages together may still not produce a shaft**, because a shaft
  is vertical and this world is 80 rows deep at the lab bed and ~22 effective
  outdoors. The first report's §2.5 scale finding is not repealed by any of
  this, and **the metres-per-cell convention is still unstated.**
- **Nothing here has been checked against a live branch.** `deadendindex
  --touching` cannot see a mechanism that exists only as prose, and its
  recall is 2 of 5 on replay, so silence is not evidence.

---

## 7. File ownership and landing discipline

**Stages 1–3 all land in `src/sim/creature.rs` and `assets/species/*.ron`,
which are among the most contested files in the repository.** Measured today
with `branchcheck.sh --who-touched src/sim/creature.rs`: **56 changes
touching it reached `main` in the last seven days, the most recent six hours
ago.**

So, for whoever builds this:

- **Run `bash scripts/branchcheck.sh --who-touched src/sim/creature.rs`
  immediately before writing**, not from this report. A roster in a document
  is a claim about the past; a reassignment drafted before that command was
  run is a claim about work that may already exist.
- **Land each stage separately and quickly** rather than holding a
  four-stage diff. The window in which another session's work cannot compile
  is the window you created.
- This report is docs-only for that reason: the plan is safe to write down
  while another lane is live in the files, and the code is not.

---

## 8. What to put in front of the owner

Every stage is judge-by-eye, so every stage is a card — and per the house
rules, **the discrete event count goes in the card's `meta`**, because a
picture of a nest cannot say whether the mechanism fired.

| stage | the card | the count beside it |
|---|---|---|
| 0 | the void as it stands, as the baseline | `roofed`, `digs`, aspect ratio |
| 1 | before/after on one seed, paired | `digs`, and the aspect ratio both ways |
| 2 | a filmstrip, because the question is whether it *goes* somewhere | depth reached, slope, `digs` |
| 3 | blind A/B — seeds present against seeds absent | chamber area each side, seed arrivals |
| 4 | the face, with and without a relay | pellets moved, standing spoil at the face |

**Stage 2 wants a GIF rather than a grid** (`filmstrip gif=1`): a contact
sheet of stills cannot answer whether a tunnel is advancing or just standing
there wider.
