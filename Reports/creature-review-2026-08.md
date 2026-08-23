# Creature work review: state of play after the S1–S4 merge, and the re-prioritised to-do list

**Status: review + proposed plan, 2026-08-23.** Written the day the S1–S4
merge landed, from the code on `main`, the review-queue verdicts, and every
creature document in `Reports/` — not from any one plan. It deliberately
re-prioritises rather than restates: `Reports/creature-evolution-plan.md`
remains the plan of record for *how* to build S5–S8, and nothing here
contradicts its dead-end register. What this document adds is (a) what the
merge and its follow-ups changed about the priorities, (b) work the staged
plan does not contain at all, and (c) the decisions that are currently
blocking, none of which have ever been put on the review queue.

Sources are cited inline. Where a number is quoted it comes from the named
report or from `README.md`'s post-merge status sections, which supersede the
plan's own "As built" notes (`README.md` M18 S1–S4: "every S4 number in it
predates this merge and is superseded by the numbers here").

---

## 1. Where the creature line actually stands (one page)

**Shipped and merged (2026-08-23, `7b97c13` + follow-ups `da252dc`,
`5a9e594`):**

- S1–S4 of the evolution plan: the Feed/Dig split, `synapse_fraction`,
  crowding that no longer counts the animal's own tail; the 584-slot genome
  with reserved dimensions and a manifest hash (only 268 slots live);
  food worth on the material (`food_energy`/`food_class`) with corpse worth
  per-cell in `aux`; litter shed to the floor, edible, rotting to soil at
  per-material rates. The §13l energy pump is closed and its sealed-world
  guards are live tests.
- The foraging-range instrument (`forage_trips`/`forage_reach`, replacing
  the loitering-counting `nest_visits`), with bars on the `ascii` gate.
- Frame cost: **rotting litter is cheaper than the bare canopy** (colony
  scene mean 3.121 → 2.979 ms). The S4-era "+45% mean" number is dead; do
  not re-litigate litter on cost grounds.

**What does not exist:** reproduction, inheritance, mutation (no operator in
the tree — `brain.rs` names one prospectively), any heritable trait outside
brain weights, any per-individual genome variation (every ant is a byte
clone of its species), predation that fires, queens/eggs, worldgen-placed
colonies (`Y` is the only spawn path). `food_class` is authored on six
materials and read by nothing until S5's `gut_bias`.

**The live problem measurements, in priority order:**

1. **`cargo test` was red on `main` at the merge** (bug A,
   `open-bugs-handoff.md` §A), and CI then ran its gates as sequential
   steps of one job — so `ascii` and `acceptance.sh` were **skipped**, and
   "main is green" was not checkable from CI for any gate after
   `cargo test` (§H's process finding, verified on run 32604849243).
   *Correction, same day:* `ci.yml` has since been reworked — one job per
   gate, with bugs A/H/Y quarantined into named `continue-on-error` jobs —
   so the reporting is honest now. **The bugs themselves are still open**
   (confirmed locally: 847 passed, 1 failed — bug A), and `ascii` is
   non-gating until bug H closes.
2. **"The floor feeds the colony, and the colony stops ranging"**
   (`README.md` M18 S1–S4 known limitation #1): deliveries +17% but moves
   −31%, nest visits −36%, digs −46%. This is the owner's stated constraint
   ("I don't want ants sitting in one spot eating fallen leaves") arriving
   as a measurement. The litter rot rate is named there as a **design**
   knob pending a verdict that has not been asked for.
3. **The colony works an ~18-cell bubble and jams itself.** Post-merge: 98
   trips, deepest 18 cells, reach profile `[3858, 475, 185, 98, 1, 0, 0, 0]`
   (`README.md`); the 55-ant arm's deepest excursion is 12 cells against 42
   for a lone ant, with ~60% of moves blocked
   (`foraging-range-measurement.md` §3 — in flight, see item T0.2). Range,
   not price, has been the binding constraint three sessions running
   (creature-direction §13k/§13n/§13o), and the range cap is partly
   *traffic*.
4. **Two accounting holes the new economy opened**: a particle drops
   `Cell::aux`, so a blasted corpse silently reprices 1,020 → 120 (bug Z —
   no existing guard can see it), and nothing tallies a corpse destroyed by
   fire/decay/brush (`meat_lost` is documented as un-hooked,
   `world.rs:202-207`). Harmless today; evolutionary attractors the day S6
   turns selection on ("evolution is a fuzzer for your conservation laws",
   creature-direction §8).

**Decisions blocked on the owner, never posted to the queue** (checked: the
review inbox has zero unanswered cards and none was ever filed for these):
E5 (selection on individuals via a new solitary grazer vs colony/queen
selection — gates the whole shape of S6), the grazer's fantasy ("a new
ancestor species is a new animal on screen"), whether S5 ships alone, and
the litter-abundance target above. `creature-direction.md` §13p says E5 "is
still awaiting the owner"; it has been awaiting since 2026-08-18 because no
card was ever posted.

---

## 2. The to-do list

Tiered, not strictly serial: T0 is this week's unblocking work, and T2/T3
can run in parallel once T0.1 lands. Each item names its measurement — a
number, from a harness, with its known-good reading — per house rule.

### T0 — Unblock and decide (all cheap; do first)

- **T0.1 Close the two quarantined gates.** *(Corrected same day: the CI
  split half of this item landed in `ci.yml` before this report merged —
  one job per gate, bugs A/H/Y excluded by name into `continue-on-error`
  jobs. What remains is the bugs.)* Fix bug H's scene (the diagnosis steer
  in §H: the *scene* lost its moisture gradient; check what builds the
  gradient, not the deposition rule — and per `ci.yml`'s quarantine note,
  close it by giving the scene a real gradient or a continuous-margin
  guard, then re-gate the `ascii` job). For bug A, re-run its existing
  seed-sweep probe with litter in the world before believing any red or
  green (§A: the margin flips across the bar with litter volume; the
  2026-08-22 sweep says the lever is weak, not flaky — do not move the
  bar), then either fix the primed-site lever or leave the quarantine
  standing with the new sweep appended. *Known-good:* `ascii` gating
  again; the `known-red-*` jobs and their exclusions deleted the day each
  bug closes. **T0.4 and every future foraging claim needs `ascii` running
  clean.**
- **T0.2 Rescue `Reports/foraging-range-measurement.md`.** It exists only on
  `claude/creatures-m18-merge-ijdlnp` (2 commits ahead, otherwise fully
  superseded by main). Its instrument landed on `main` via `da252dc`; the
  357-line measured record — the 19-cell bubble, the crowd-jam numbers, the
  `FORAGE_TRIP_MIN` derivation, the "instrument was behaviour-neutral only
  on the second try" story — did not, and is unindexed. Land it with a
  status correction (its §0 records the S1–S3 branch as lost; that branch
  has since been found and merged) and its index line; then the branch is
  deletable. *Measurement:* `docscheck.sh` green; `branchcheck.sh` shows the
  branch merged-or-gone.
- **T0.3 Post the blocked decisions as review cards** (fire-and-forget, per
  the review skill):
  - **(a) E5 + the grazer fantasy + "does S5 ship alone"** — one card, the
    three §8 questions that gate S6's shape. Include the plan's own
    reversibility note (§7b colony selection returns the day a colony
    generation completes inside ~20,000 frames).
  - **(b) The abundance dial** — the known-limitation numbers above, plus a
    paired filmstrip of two litter rot rates, asking what scarcity is
    intended. This is the knob that decides whether leaving home is worth
    anything, and S7 only pays if the surface does not already feed a
    colony (evolution plan §2.4, "the owner's constraint, and why it is one
    constraint and not two").
- **T0.4 Re-baseline the standing guards on post-merge `main`, in one
  session.** Every number in the plan's §4 guard table (advantage
  +0.187/+0.247, ants-fed 0.42/0.55, the frame-cost pair) predates litter.
  Re-run them; re-derive `creature_space`'s scarcity band with an edible
  floor in the world (§2.4's third coupled call: colony-band food went 600 →
  ~47,000–58,000 on one seed, which invalidates the band the sweep was
  calibrated for); and build the one instrument the S1 notes named as
  missing and still is: **deliveries measured paired across seeds**, not on
  one `ascii` seed. While in the harnesses, fix the parameter-echo defects
  the megastudy lesson exists to prevent: `creature_space` prints "4
  beetles" against `BEETLES = 9` and echoes neither preset, `START_ENERGY`,
  `move_cost` nor base seed; `ant_ablation` does not echo its `terrain=` or
  `food=` mode; `forage_probe` does not echo its seed. *Known-good:* a log
  that names every parameter it ran under; guard numbers re-set from the
  new baseline with headroom.
- **T0.5 Docs honesty pass** (half a day): `wiki/ants.md` still says litter
  does not rot (doc-rot — it does, `wiki/plants.md` and the code agree);
  `dead-ends.md:795` still records the sealed-world test as `#[ignore]`d
  and failing (it is live and passes at an 80,000-frame horizon);
  `PLAN.md`'s M18 section still describes the never-built worm/binder/borer
  cycle with no pointer to the ant line, the brain, or S1–S4 (add the
  pointer; mark the cycle proposal superseded by the evolution plan unless
  the owner wants it revived); and four small code-comment defects from the
  survey (the orphaned `TUMBLE_ON_FAILED_MOVE` narrative stuck to
  `CROWDING_SCALE`, "the 14 brain inputs" (16), "248" as the tight-layout
  count (268 with Feed), `step_chain`'s unused `material_id` parameter).

### T1 — Close the two meat-accounting holes (before S6, deliberately)

- **T1.1 Bug Z: `Particle` must carry `aux` for `worth_in_aux` materials.**
  Fix shape per §Z: add the field, write it back only when the landing
  material declares `worth_in_aux` (a wet soil grain must not land claiming
  to be food). Add the guard that §Z notes cannot currently exist: a corpse
  census (total standing meat) before/after a blast through a corpse pile,
  asserted equal minus what the blast legitimately destroyed. *Known-good:*
  a thrown corpse lands worth what it left as; `rigid.rs`'s aux-less
  `BodyCell` stays as-is (documented deliberate).
- **T1.2 Hook the `meat_lost` seam.** A corpse destroyed by fire, decay,
  explosion or the brush books nothing today, which quietly loosens
  `max_standing_meat` from an invariant into a hope. One accounting call at
  the destruction seam (the same `World::set` choke point §13l's chain fix
  went through). The S3b living-flesh seam (a bitten live animal's stamp
  never becomes meat; `ant.ron`'s `food_energy == body_energy` equality is
  load-bearing) stays booked with S6 per the plan — note it on the same
  ledger doc comment so the two seams are found together.

### T2 — S5: diet as one heritable number (build as specced)

Build `gut_bias` exactly per evolution-plan §2.5 — the spec is good and
already half-staged in the data (`food_class` is authored on every food as
an axis position; `material.rs` names `gut_bias` in its doc). The parts to
hold onto: the matched-filter yield with no free dimension;
`FoodAdjacent` reading the same gene-dependent predicate as the eat verb (a
meat-gut animal stops *seeing* leaves); palette lerp by the trait; two
ancestor `.ron`s ±0.8 apart; **delete `CreatureDef::food` and bank the
~32-string-hash-per-tick saving S3 promised**. Measurement as written
there: the survival-vs-`gut_bias` curve, two-humped in a mixed scene,
single-peaked in a litter-only control, separation ≈ 0 with both ancestors
at 0, `placed` reported in every arm. Expect one hump first — the fix is
carrion ecology (seed carrion / raise the stamp), not the filter.

### T3 — Traffic and range (new work; not in S5–S8 at all)

The plan treats the small foraging bubble as a motivation problem the
ecology will fix. The measurements say a hard component is **traffic**: 60%
of moves blocked at 55 ants, deepest excursion 12 vs a lone ant's 42, and
the gridlock finding (27,386 blocked ticks, an unbroken wall of ants) was
severe enough that colony founding spaces ants four apart to this day.
Every verb counter climbs while it happens, which is why nothing saw it
until the range instrument existed.

Two candidate mechanisms, both cheap to test paired now that
`forage_reach` exists, both with their dead-end conditions on record:

- **Ants pass over nestmates** (a creature cell as conditional foothold or
  swap for same-species). Dead-end 775/829's own condition line says it
  "reopens if creatures gain pass-through or climb-over" — this is that
  re-test, done deliberately.
- **A dispersal drive** — the outbound half of run-and-tumble biased away
  from crowding. `Crowding` is already a brain input and Persist/Tumble/
  Caution are already genes, so this may be an *authoring* change plus a
  re-derived weight, not a mechanism.

*Measurement:* `forage_probe`'s reach profile, 8 seeds, paired arms —
known-good today reads deepest 12–18 with buckets ≥32 at zero; success is
weight appearing in the ≥32/≥64 buckets without deliveries falling. Judged
by eye on a filmstrip — a colony *streaming* along a trail instead of
milling at the mouth is precisely the satisfying, legible outcome the
project optimises for — and posted to the queue. This also gives T0.3(b)'s
abundance dial its counterweight: ants that can range vs a floor that does
not demand it.

### T4 — S6: reproduction (evolution switches on; needs E5 answered)

Build per evolution-plan §2.6 (budding, `reproduce_threshold >
start_energy` so doing nothing becomes extinction, per-weight mutation
width `MUT_ABS_FLOOR + MUT_REL·|w|` clamped ±40, both pre-flights: the
`MoistureLateral` spurious-constant probe and the published clonal-drift
band). The record adds five constraints the plan does not restate — all
already paid for elsewhere in this codebase:

1. **Never derive a child's genome or draws from its slot id** (dead-end
   670: generation wrap at 16 hands out bit-identical ids that cluster
   spatially). Draws keyed on the child's handle via `rng::stream`, stored
   at birth.
2. **A birth scheduled from inside a tick must not take the live heap out
   from under the scheduler** (dead-end 1094: a mid-tick-scheduled seed
   never grew, forever).
3. **The 4095-slot ceiling gets a release-mode guard** before any breeding
   run (population-dynamics acceptance 9g). `push_organism`'s range check
   is a `debug_assert` — CI's new debug job now compiles it, but release
   builds (the app, every long harness run) still don't, which is the
   shape §F4 flags as silent identity corruption.
4. **Close the S3b stamp seam here** (T1.2's note): a parent pays the
   child's stamp, and flesh destroyed without becoming corpse gets a sink.
5. **Test discipline: populations and paired comparisons only** (dead-end
   552 — every single-individual assert broke the day plant genotypes
   varied; creature outcomes already spread 0.103–0.541 across random
   genomes).

Ship with the counters first (`births`, `births_denied_no_space` — must
read 0 at this stage), a population readout, and an `ascii` re-run at the
largest population the economy actually produces; creatures were measured
free at 55 ants, and a breeding population is not 55.

### T5 — S7: two larders and a barrier — with the canopy on the table

Keep the reciprocal-transplant experiment exactly as specced (it is the
only instrument in the plan that can tell divergence from drift, and it
must run on generated terrain — dead-end 787 is three-for-three against
hand-built scenes). But before building the buried fungus larder, put a
**canopy option** on the queue: the owner's stated fantasy is ants digging
*and climbing trees*; ants measurably can climb (7–11 cells up with a tree
vs 0 on bare ground) and don't because nothing motivates it; the owner's
litter verdict was "ideally some would land in the crown but the vast
majority on the ground" — a small crown-retention fraction is his stated
ideal, shipped as all-to-ground for simplicity. A canopy larder is the
same shape as a buried one (food behind a barrier only some genomes
cross — the plan says this itself in §2.4), it reuses the retention knob
litter already has, and its barrier verb is the one the owner asked for.
Post both options as one card with mockups; build the one he picks. The
dig allele + proportional metabolic tax half of S7 stands either way.

### T6 — The predation probe (parked, cheap, after T2/T4)

Per evolution-plan §5: wire the beetle's instincts to the existing
channel-B along-gradient (one `.ron` edit **plus a rebuild** — assets are
`include_str!`ed), behind the pre-flight that prints total channel-B mass
and the fraction of prey heads within a sensor offset of a nonzero B cell.
If the trail barely exists where beetles are, the predator is blocked on
movement, not perception, and that is worth knowing before any fear-scent
channel is designed. The third pheromone plane stays behind its
settled-world cost gate (0.5 ms; the two existing planes measure 0.0014 ms
settled).

### T7 — S8: heritable anatomy (last, and only after its two pre-checks)

As specced (§2.8): the girth pre-check (`moves_blocked` ratio for girth
1/2/3 at a fixed brain on generated terrain — if wide bodies are more than
twice as blocked, width is not a gene) and the birth-placement decision
(no "too big → the birth does not happen" gate keyed on a heritable
trait), before any gene is written.

---

## 3. Standing risks — watch, don't build yet

- **Scheduler determinism** (`PLAN.md` issue #7): the `HashMap` drain order
  is the engine's one named real determinism violation, and creatures ride
  that scheduler. The both-drivers `ascii` diff is the guard; run it every
  stage, because after S6 any divergence contaminates every evolution
  result.
- **Pheromone planes vs M10**: world-sized, eagerly allocated (~84 MB at
  the 4x world) and the one channel the LOD design says must persist in
  full. `ecological-lod-design.md` is still awaiting sign-off; it is
  needed before streaming, not before S6 — but S6's "frozen means frozen"
  corollary (a creature that ages but does not eat off-camera is the worst
  of both) should be checked against it when reproduction lands.
- **The light channel swings 20:1 by design.** Nothing evolved gates on
  `LightHere` yet; the day a lineage does, decide deliberately whether
  that is nocturnality emerging (keep it) or aliasing (divide by
  `noon_equivalent_light`), rather than discovering it in a sweep.
- **Enrichment destabilises** (population-dynamics §0): every plant/water/
  worldgen improvement raises the ecology's amplitude, and the failure
  will be attributed to whatever shipped most recently. Budget a guard
  re-run after every big merge — this is what T0.4's re-baseline is, made
  routine.
- **The damp economy has known biases**: standing litter reads dry because
  it samples its own (air) block while 66–76% of decay events are damp
  ones; a litter blanket blocks rain soak entirely (§F1, live); soil `aux`
  ratchets wet with no soil-to-air drying (§F8). Moss- and rot-dependent
  behaviour sits on all three. None is urgent; all three belong on the
  table when the abundance dial (T0.3b) is tuned, because they *are* that
  dial's plumbing.
- **Grass, when it becomes plantable**, arrives with three latent creature
  interactions on record (§F4 immortal grass holding slots, §F5 grass
  seeds as an unbounded ant larder and a recruitment sink). Re-read §F
  before adding grass to any creature scene.

## 4. What this list deliberately does not do

- It does not reorder S5→S8's internal dependencies (S5 before S6 before
  S7/S8 stands; the dependency table in evolution-plan §3 is correct).
- It does not revive queens/colony lifecycle — E5's reversibility
  condition (a colony generation inside ~20,000 frames) is a measurement
  and stays one. But note stigmergy-research's floor: below ~50 workers
  colonies build nothing, so the colony *fantasy* ultimately needs
  reproduction for ants too; budding (S6) is the machinery it will reuse.
- It does not propose new pheromone channels, `Cell` widening, crossover,
  or topology evolution. Every one has a standing condition in the
  dead-end register; none of the conditions has changed.
- It does not schedule worldgen-placed colonies ("the world has ants
  before `Y` is pressed"). Worth wanting; blocked behind S6 by the
  extinction-is-default finding — a placed colony that cannot replenish is
  a timed corpse pile. Revisit when births exist.
