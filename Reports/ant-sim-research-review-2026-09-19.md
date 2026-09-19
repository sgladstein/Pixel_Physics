# Review of an outside literature survey on ant simulation — what the engine already does, what it has tried, and what is worth taking

*2026-09-19. Review of
[`ant-sim-literature-review-external-2026-09-19.md`](ant-sim-literature-review-external-2026-09-19.md),
a survey written by someone who knew only that this project is an ant-based
simulation game. **Docs only; nothing built, nothing run.** Every claim about
the engine below was checked against the tree or the repo's own measured
record and says where. **The copy of the survey received ends mid-sentence in
its §9**, so its demography section and the four species-parameter tables its
TL;DR promises are not reviewed here; §6 says what waits on them.*

## 0. The answer, stated once

**The survey's architecture is this engine's architecture, and its central
recommendation was taken before it was written.** Stigmergy-first, agents on
local rules over an evaporating field, a Deneubourg choice function, no
pathfinding — that is `stigmergy-research.md` (status *implemented*),
`src/sim/pheromone.rs` and `src/sim/brain.rs`. Of the survey's ~40 named
mechanisms, this engine ships **14** (one of them, the home bearing, in the
tree and switched off), has designed and priced **4**, has built and measured
**6** as rejected or inert, and cannot stage **7** because the world is a
side-view section. **Three are genuinely new and worth a run.**

The survey names five things existing games omit — multiple pheromone types,
quorum decisions, metabolic scaling, path integration, topochemical
construction — and calls that gap "exactly where realism gains lie". For this
engine the measured binding constraints are elsewhere, and the record says so
with numbers:

| the survey's five gaps | here |
|---|---|
| multiple pheromone types | three planes ship (home, food, alarm), each with its own lifetime; a fourth is held until a consumer exists |
| quorum nest choice | no colony ever relocates; no verb, no need yet |
| metabolic scaling | every joule is charged per body cell (`ant.ron:118-119`); linear, not ¾-power, and moot at two cells |
| path integration | **built and switched off.** The state ships (`organism.rs:6079` `forage_anchor`) and so does the verb — a fill-weighted homeward re-roll in `tumble` (`creature.rs:11716-11830`, commit `da4a461a`) — gated by `CreatureDef::home_bias`, which no species authors, so it ships at 0.0 |
| topochemical construction | curvature-attracted deposition is wired (`ant.ron:1470`) and measured at *a tenth steeper*, not a wall |

What actually binds, measured: **0–8% of ants find food without a trail** and
the colony cannot lay one itself (`pheromone-master` §3.2); the **return leg
worked for the first time on 2026-09-18** (0 → 511 deliveries over 18 seeds);
**the nest has no purpose** — no granary, no eggs — by owner ruling
(`nest-biology` §10); and the ground is **one to two orders of magnitude too
shallow** for the nests the literature describes (`nest-biology` finding 4).
None of the survey's five is any of those.

**The three things worth taking**, priced in §3:

1. **A re-test the survey's own numbers call for.** Deposition that rises
   toward the food (Beckers 1992; Czaczkes 2024, *22x more within 10 cm of the
   food*) was built here as a food-charged odometer on `EmitB` and **rejected
   on 2026-09-17** — one day before the return leg was fixed. Its rejection
   condition is now met. Cost: one archived rider, one run.
2. **Negative feedback on the emitter, not the reader.** Czaczkes, Grüter &
   Ratnieks 2013: crowded ants lay 5.6x less. Here that is one authored wire,
   `(Crowding, EmitB, −w)`, in the genome and losable by a line, and it aims
   at §Z7 — the food trail is deaf on purpose because at full gain the colony
   converges on patches it has already eaten.
3. **The "no-entry" mark** (Robinson et al. 2005) is the literature's answer
   to the same §Z7, from the other side: an ant leaving an exhausted site marks
   the way as *not worth it*. Expressible here as a *negative deposit* on the
   food plane — no fourth plane, no new reader — but it needs an emitter
   condition that is state, so it costs a hidden unit.

**One thing found on the way, and it is the owner's.** The survey's path
integration — *"maintain a running home vector per ant"* — is in the tree:
`da4a461a`, *"the return leg gets a verb: a fill-weighted homeward tumble, off
by default."* It reads `forage_anchor`, it is keyed on crop fill rather than
`Carrying` for a measured reason (`organism.rs:4141` doc), and it ships at
`home_bias: 0.0` with **no lab dial, no wiki line, and no landed measurement
I could find**. Under the standing ruling — *ship new behaviours on; a default
that looks wrong is to register and report, never to tune* — that is a
question to put to the owner rather than a default to leave.

**Two corrections the survey needs before anyone builds from it here** (§4):
evaporation is *not* the parameter on a one-cell trail — diffusion is the
eraser, measured, and the shipped decay rate is inert; and a new brain input
is not "cheap to implement" in this engine — it re-derives `mutation_rate` in
six species files and voids every stored baseline.

---

## 1. What was checked, and how

Read: the survey; `wiki/ants.md` in full (the bar); `pheromone-master-2026-09-17.md`
in full; `stigmergy-research.md` in full; the module docs of `pheromone.rs`
and `brain.rs`; every authored wire in `assets/species/ant.ron`; the openings
and decision tables of `nest-biology-2026-09-19.md`, `nest-digging-plan-2026-09-19.md`,
`nest-design-2026-09-14.md` §2, `colony-economy-design-2026-09-09.md`,
`creature-signature-and-castes-2026-09-06.md`, `creature-stacking-design-2026-09-17.md`;
the two nest reports still on `claude/nest-biology-research` (PR #472) and the
trail-lifetime work on `claude/upbeat-shannon-cez0w4` (unlanded, 20 ahead);
`Reports/dead-ends.md` grepped by mechanism, per `CLAUDE.md`, for every
mechanism the survey proposes; the open-bug index.

Not done: no binary was built, no scene was run, no card was posted. Every
number here is the repo's own, cited to the document that measured it.

---

## 2. Section by section — the survey against the tree

### 2.1 Stigmergy-first, and the modelling choice (survey §1–2)

| survey | engine | verdict |
|---|---|---|
| emerge from local rules on an evaporating field; never script top-down; never A* | `emergent-world-architecture.md` §1: agents interact only through the world. No pathfinding exists; an ant reads one cell ahead at `sensor_offset: 6` (`ant.ron:1033`) | **same design, arrived at from architecture rather than from Grassé** (`stigmergy-research.md` preamble says exactly this) |
| Deneubourg choice, `n≈2, k≈20` | `creature::choose_weighted`, `p ∝ (k+s)²`; replacing it with `min_by`/argmax is a named dead end (`dead-ends.md:1061`, `:1099`) — *"the noise is load-bearing"* | ships |
| Weber-law individual response (Perna 2012) reproduces the collective sigmoid | `BrainInput::PheroAAlong = (ahead − here)/(ahead + here + 1)` — a Weber relative difference, scale-free in ratio (`pheromone.rs`, `Scent` doc) | ships |
| hybrid: agents + grid field + ODEs for demographics | agents + two CA-resolution planes. **No ODE layer, on purpose**: the lab exists to watch individuals evolve; off-camera catch-up is `ecological-lod-design.md` | not a gap |
| population threshold (Grassé ~50) | `wiki/ants.md`: *"fifty looks like a colony, five looks like a bug"*; `stigmergy-research.md` §1 | ships |

One thing the survey's §2 does not say and the engine had to measure: **a
pheromone field cannot live on the coarse field grid.** A sixth `FieldCell`
channel at `FIELD_SCALE = 8` was rejected because two trails four cells apart
produced a bit-identical field — *impossible* to resolve at any sensor offset
(`pheromone.rs` module doc; `dead-ends.md:1213`). The planes are their own
CA-resolution buffers. The survey's "PDE-like diffusion on GPU" advice is fine
in the continuum and would have failed here.

### 2.2 Pheromone dynamics (survey §3)

**What ships.** Two trail planes plus an alarm plane, `u16` 8.8 fixed point,
per-pass multiplicative decay through a strict-decrease LUT, a 3x3 blend, and
deposit only on a successful move — the omission naive builds make, and a
dead end here (`dead-ends.md:1059`). Constants, all measured rather than
chosen: `PHEROMONE_INTERVAL 12`, `DIFFUSE 0.25`, `DECAY_RHO 0.03`,
`DEPOSIT 40`, `ALARM_RHO 0.35` (`pheromone.rs:110-325`).

**Where the survey and the record disagree, the record wins:**

- *"Species-specific half-life is the single most important parameter."*
  Here the lifetime is **per plane, set by what the plane is for**, and the
  survey's own §3 lists the reason without drawing it: alarm is news and
  trail is memory. `ALARM_RHO` established that; the unlanded
  `claude/upbeat-shannon-cez0w4` branch splits the two trail planes on the
  same axis — the way home at rho 0, the food trail keeping 0.03 — and
  measures round trips up *better than ten times* at gap 90 over 36 seeds,
  with the same keeping on the food plane *tried and negative* (a third as
  many ants ever found food, because the colony walked to bare ground). The
  survey's numbers (Pharaoh 9 min, Argentine 30 min–4 h, army ants days) are
  a spread the engine reproduces by *purpose*, not by species — and the
  species half exists too: an ant's emitter is an odometer whose decay is a
  heritable weight, so *"a lineage can lay a longer trail, or a shorter one"*
  (`wiki/ants.md`, "How long the fade lasts").
- *"Per-step multiplicative decay plus a diffusion step"* — correct, and
  **the decay term is inert on a one-cell trail.** Measured
  (`pheromone-lifetime-and-wiring-2026-09-14.md`): setting `DECAY_RHO` to
  zero leaves an unreinforced trail's 144-frame life unchanged, because a
  one-cell line loses 16.7% per pass to the 3x3 blend against decay's 2.9%.
  Any parameter table that lists evaporation rates and not diffusion will
  mis-tune this engine.
- *Beckers 1992 / Czaczkes 2024: deposition rises with distance from the nest
  and is 22x higher near the food.* The engine's home plane has the
  **opposite** ramp by design — a nest-charged odometer (`ant.ron:1873`,
  `:2028`), strongest at the door — because channel A is a home-range mark,
  not a food trail (survey §3's "home-range / nest-marking" type;
  `nest-design` §2 item 3). The food plane is **flat**: one wire,
  `(CarryingFood, EmitB, 2.5)` (`ant.ron:1240`), and `CarryingFood` is a
  boolean — 1.0 for any food in the crop (`creature.rs:5209`, *"one question
  per sensor"*) — so every food-trail deposit is the same strength whatever
  the load is worth. The survey's shape — strongest at the food, fading toward home —
  is exactly the *food-charged odometer on `EmitB`* that was built as
  `trailfollow` riders and **rejected on 2026-09-17** (`dead-ends.md:1925`):
  *"the premise is right, the fit is exact, the emitter demonstrably works,
  and the colony does worse the more of it there is."* See §3 item 1 for why
  that rejection is due a re-test.
- *Multiple types are cheap.* A plane is O(world) per pass and ~40 MB at the
  shipped world size (the alarm plane is allocated lazily on the first bite
  for that reason, `creature.rs:18395-18436`). The standing rule is no third
  trail channel until a consumer exists (`stigmergy-research.md` §8).

**What the survey has that the engine does not:**

- **A negative / "no-entry" pheromone** (Robinson 2005, 2008): nothing in the
  tree repels except the alarm at contact. §3 item 3.
- **Crowding downregulates deposition** (Czaczkes 2013): `Crowding` reaches
  `Move` (`ant.ron:1176`) and `Dig`, never `EmitB`. §3 item 2. Note that the
  *reader*-side negative feedback the survey's Grüter citation asks for is
  already the engine's anti-ossification mechanism: `dead-ends.md:1067` says
  in as many words that turning up evaporation does not fix ossification and
  the crowding input does.
- **Alarm: attracted at low concentration, repelled at high.** Here the alarm
  is one graded reading through two wires, `(Alarm, Move, −1.0)` (stop) and
  `(Alarm, Attack, 2.0)` (`ant.ron:1528`, `:1542`), spread as an *active
  space* rather than a substance so the middle and edge of a fight read
  different numbers (`lanes/evolution-lab-pheromones.md`). The
  low-attract/high-repel curve is one more wire on the same input; nothing
  that ships is born using it and the owner has left the alarm's meaning
  deliberately unruled.
- **Osmotropotaxis** (left–right antennal comparison): built as
  `PheroALateral`/`PheroBLateral` and **dead for a surface walker** — on a
  one-dimensional floor the side sensors read floor and air (the Jones /
  Physarum three-sensor dead end; `pheromone-master` §7 step 5). Replaced by
  the along-heading difference. A side-view consequence, §5.
- **Trail-geometry polarity** (Jackson 2004, the bifurcation angle): discussed
  in `pheromone-trail-direction-2026-09-16.md`'s biology section; a
  side-view trail has almost no bifurcations to read.

### 2.3 Foraging and recruitment (survey §4)

| survey | engine, measured | verdict |
|---|---|---|
| mass recruitment via trail | a hand-laid food trail is decisive — **92% of ants reach food against 3%** unaided, 4 of 6 colonies alive against 0 (`pheromone-master` §3.2); **the colony cannot lay one itself** (`self ≡ mute`); discovery is the binding constraint | mechanism works; bootstrap does not |
| trail modulated by food quality (Beckers 1992) | **absent.** `EmitB` reads `CarryingFood`, a boolean (`creature.rs:5209`); a load's worth reaches nothing the emitter reads. The graded input exists — `Carrying` is crop fill — but under `SPOIL_IS_CARGO` it reads 1.0 for a pellet of dirt, so a fill-graded emitter needs a food-only graded sense, which is a slot bump | one of the arms of §3 item 1 |
| decay rate tuned to ephemeral vs persistent resources | `Pheromones::set_channel_rho` and `set_channel_diffuse` are per-plane dials (`pheromone.rs:869`, `:898`); `labforage bdecay=` sweeps it | dials exist; the sweep `dead-ends.md:1869` names as the re-test has not been run |
| negative feedback preserves flexibility (Grüter) | `(Crowding, Move, −0.3)`; food route left near-deaf **on purpose** — read at full gain the colony *"eats out its own doorstep"*, a quarter of the animals against three quarters in a mirrored race (`wiki/ants.md`; §Z7) | the survey's remedy is the engine's open problem |
| tandem running, group recruitment | no leader–follower verb. `KinBearing`/`KinNear` exist (`brain.rs`), so *follow the ant ahead* is authorable — and blocked by §R4 (`Turn` inert on flat ground) | not staged; not needed for a mass-recruiting ant |
| solitary foraging + path integration | see §2.5 | |
| trail + route memory synergy (Czaczkes 2011) | `BrainOutput::Persist` is the straight-ahead score; no route memory | none proposed |

### 2.4 Traffic and lanes (survey §5)

**Cannot be staged.** Couzin & Franks' three lanes and Dussutour's
bidirectional flow are top-down results on a two-dimensional floor. This
world is a vertical section: the trail is a one-dimensional line along the
ground and there is no lateral axis to form a lane in. The engine's answers
to the same complaint — *"a trail that works puts every ant on one line, and
a line that cannot overlap is a queue"* — are the third attempt on colony
traffic (`creature-stacking-design-2026-09-17.md` §1): founding spacing,
climb-over (`climbs_over_kin`), pass-through swap (`passes_through_kin`, off),
and now **stacking** (`animals per cell`, off by default, PR #465).

**Right-of-way for laden ants** (Dussutour 2009: laden inbound ants given way
in ~80% of encounters) is the one transferable rule, and the engine has the
*opposite* priority today: a laden long ant *defers* to a jam near the door
(`longant.ron` `traffic_defer_max`). A priority predicate on the existing swap
verb — a laden ant may pass through an unladen nestmate, never the reverse —
is cheap. It is ranked low because **congestion is measured not to be the
fault**: pass-through cut blocked moves ~90% and *reduced* intake 35%
(`pheromone-master` §3.8).

### 2.5 Navigation (survey §6)

**The survey and the repo's design of record agree, independently, and the
survey adds one thing.**

- *Path integration supplies direction; the trail is a contextual modulator;
  trails are isotropic.* `pheromone-master` §6 says this in the same words,
  and `nest-design` §2 ranks the same five cues in the same order. As shipped
  a laden ant homes the way a bacterium does — *"keeps walking while the home
  scent is getting stronger, stops and turns when it is not"*
  (`wiki/ants.md`, "What is not finished") — which both sources call the
  wrong primitive. **The right one is built and switched off.**
  `forage_anchor` is the world coordinate of the last nest touch, re-anchored
  at every contact (`organism.rs:6079`; `creature.rs:1699`, `:10252`), and
  `home_weighted_pick` (`creature.rs:11716-11830`) reads it inside `tumble`:
  at probability `home_bias × crop_fill` the re-roll picks the viable heading
  nearest the anchor instead of a uniform one. It sets `heading` directly,
  which is why it is in `tumble` and not on `Turn` — *"the only site that
  sets direction and the only one that works on flat ground, where §R4 kills
  `Turn`"* (`pheromone-master` §7.26) — so it is **not** blocked on §R4. It
  is gated on `CreatureDef::home_bias`, `#[serde(default)]`, authored by no
  species, so the shipped ant runs a bit-identical stream to the one before
  it existed. Commit `da4a461a`; no wiki line, no lab dial. The earlier
  design — `HomeBearing`/`HomeDistance` as brain inputs driving `Turn`
  (`nest-design` §8C, master §7 step 4) — is superseded by it in the master's
  own second banner.
- *Exact, not integrated.* The survey says "maintain a running home vector"
  and expects drift. Here the vector is exact — no error, no drift — and
  `nest-design` §2 names the cost: the error and the search *"are what make a
  real return look like searching rather than teleporting."* The master
  report's instruction stands: ship it exact and add an error term only if it
  reads as teleporting, judged by eye.
- **What the survey adds, and what it does not.** The *systematic search*
  (Müller & Wehner 1994 — an ant whose vector reaches zero without a nest
  spirals out from where the nest should be) is **moot with an exact anchor**:
  the anchor is the last door contact, so a vector of zero *is* the door, and
  `home_weighted_pick` returns `None` within one cell for exactly that reason.
  It becomes worth having only if an error term is ever added. What does
  transfer is the **weighting by vector length** — a long vector dominates the
  trail, a short one defers to it (`pheromone-master` §6, the *Veromessor*
  result) — which `home_bias × fill` does not do: it weights by how full the
  ant is, never by how far out it is. The master's `HomeDistance` was that
  term. Priced with item 4 in §3.
- *Visual panorama, central-complex models, mushroom bodies*: no analogue,
  none proposed (`nest-design` §2 item 5), and the survey itself says a
  functional approximation is enough. The engine's bearings
  (`PreyBearing`, `KinBearing`, `ThreatBearing`, `BloomBearing`) are that
  approximation; a home bearing would be the fifth.

### 2.6 Nest excavation and construction (survey §7)

**Read `nest-biology-2026-09-19.md` §10 before any of this: the owner ruled
that the nest has no purpose in this game yet**, and every construction
mechanism is in service of a function the box does not have. That ruling is
prior to the survey's whole §7.

| survey | engine | record |
|---|---|---|
| Tschinkel: chambers large and stacked near the surface, smaller and further apart with depth | **no depth term anywhere in the dig**; the world is 16–40 cm of soil against a 2 m nest; no metres-per-cell convention has ever been written | `nest-biology` findings 3–4: the most decision-relevant thing in that report, and a *scale* decision |
| Buhl 2005: logistic excavation, volume ∝ workers | per-ant dig, so proportional by construction; `ROOM_TARGET` hyperbola slows and never stops, which matches Rasse & Deneubourg | `nest-biology` §2.3 |
| Bardunias & Su: depressions are digging cues | `SurfaceCurvature → Dig` measured at **2.3x** roofed chamber with the sign reversing; not shipped | `nest-digging-plan` Stage 3 |
| Khuong 2016: pellets marked with a building pheromone, deposition ∝ local pellet density → pillars → chambers | pellets exist (`spoil`, cemented, `needs_footing`); deposition prefers curvature at `(SurfaceCurvature, DropSpoil, 0.169)` (`ant.ron:1470`), measured *a tenth steeper* and *"not enough to build a wall out of"*; the curvature wire on **food** drops was removed on 2026-09-19 because a food heap is curved ground and the forager put its dinner back on the larder | **the deposition half of Khuong is in no plan** — §3 item 6 |
| a *digging* pheromone at the face | **measured negative** (Bruce 2015, validated through PubMed); the termite cement pheromone is *"contested and weakly supported"* | `nest-digging-plan` §3; the digging-signals report on PR #472 |
| Toffin: density at the perimeter → budding | built as `(Crowding, Dig, 0.6)`, fired, and produced no buds — because `Crowding` is a colony-wide scalar every ant at the door reads alike; the local reading is built default-off (`PIXEL_PHYSICS_CROWDING_LOCAL`) | `dead-ends.md:1788`; `nest-biology` finding 1; `nest-digging-plan` Stage 2 |
| Goldman lab: clogging avoidance in confined digging | the two spoil dead ends (refill the tunnel; drop loose) are the clogging problem; `digbox` at 1,200 ants is *"4x beyond the largest fitted group, so the lens may be a jam"* | `dead-ends.md:1087`, `:1093`; `nest-entrance-dimensions` |
| termite mounds, TERMES, wasp lattices | no analogue; no consumer | — |

The survey's §7 headline — Khuong as *"the template for physics/pixel-based
digging"* — is half right for this engine. Khuong is a *construction* model:
pellets deposited above ground, attracted to existing pellets. The engine's
nest line is a *digging* problem (galleries that stay dug, chambers that never
form), and its plan of record has already chosen Toffin's self-amplification
in physical form — *fresh spoil attracts digging* — as the one candidate left
(`nest-digging-plan` status note). Khuong's rule is the same shape pointed at
the other verb, and it is the piece the owner has already judged by eye three
times: the anthill.

### 2.7 Division of labour (survey §8)

- **Response thresholds are the brain.** Each ant has heritable weights and
  a bias per output, a `squash` nonlinearity, and per-birth mutation
  (`brain.rs` module doc; `ant.ron` throughout — `(FoodAdjacent, Feed, 0.8)`,
  `(Crowding, Dig, 0.6)`, `(KinNeed, Share, 1.9)`). Bonabeau's model is a
  special case of it: a threshold per task per worker, individually varied.
  The stimulus reduction his model needs is done by the world (digging
  lowers `Crowding`; feeding lowers `KinNeed`). Nothing to build.
- **Interaction rate** (Gordon): `KinNear`, `Crowding` (a local count) and
  `KinNeed` are contact-rate proxies; no explicit encounter integrator, and no
  task identity on the cuticle — the scent signature encodes *kin*, not job
  (`creature-signature-and-castes` §1a). Nothing missing that a wire could
  not add.
- **Morphological castes**: the general mechanism shipped 2026-09-06 and
  ships *on* — a parent hands its child one number (`Provision`), the child's
  body is its genes shifted by heritable developmental weights (`Made`), and
  *"nothing in the box says what a caste is"* (`wiki/ants.md`). The survey's
  "implement as fixed roles with size-linked stats" is the authored version
  the owner ruled out (`creature-signature-and-castes` §0).
- **Age polyethism** (Mersch 2013): **no age input exists**, and the nest
  biology report decided *no worker age* (its lane note, decision list).
  Lifespan ships (`life_half_life: 40000`, `ant.ron:47`). An `Age` input is
  the cheapest way to let a line *find* a temporal caste — one appended slot,
  lawful under the reserve — but it is a `BRAIN_INPUTS` bump with the tax §4
  describes. Flagged, not recommended.
- **Reserve / idle workers**: `colony-economy-design` §4b found *"the ant has
  no idle state, and it is authored that way"*; hunger and rest shipped since
  (`(Energy, Move, −1.75)`, `Stillness`), so *"a well-fed ant mostly rests"*.
  The survey's point is met.

### 2.8 Life cycle and energetics (survey §9, as far as it arrived)

- No queen, no egg: an ant *buds* (`wiki/ants.md`, "New ants"). A queen
  regime exists behind `PIXEL_PHYSICS_BREEDING=queen`
  (`creature.rs:9308-9370`) and the coordinator's standing ruling is that a
  queen is *"three authored values over existing mechanisms, never a type the
  engine knows."* Eggs are designed, not built (`nest-biology` §11).
- No brood stages, no temperature-dependent development. `FieldCell` carries
  a depth-graded temperature with the diurnal cycle separable exactly
  (`nest-biology` finding, §1 table), and nothing but `(TempAboveAmb, Turn,
  −0.8)` reads it.
- Sigmoidal colony growth: the engine got boom-and-crash first and settles it
  with lifespan — *"the colony's size settles near how fast it breeds times
  how long an ant lives"*.
- Metabolic scaling: charged per cell, linear (`idle_cost_per_cell`,
  `move_cost_per_cell`). The survey's sentence on it is where the paste
  stops.

---

## 3. What to take, ranked

Ordered by what it costs against what it could move, with the dead end that
bears on each and the instrument that would score it. **None of these should
land in the same change as another**: every one adds a term to a shared
weighted sum, and `CLAUDE.md`'s *a correct mechanism at inherited constants is
a regression* makes a joint result unattributable.

1. **Re-test the food-charged odometer on `EmitB`** (survey §3, Beckers /
   Czaczkes 2024). Rejected 2026-09-17 (`dead-ends.md:1925`) on a bed where
   **no laden ant ever got home** — the master report's own banner warns
   *"every `self` number in this document predates a working return leg."*
   The return leg was fixed 2026-09-18 and the way home keeps on the unlanded
   lifetime branch. A trail that is strongest at the food and fades toward the
   nest is only readable *by an ant walking outward from the nest*, which is
   the leg that now exists. The cheaper arm to run first is the survey's
   *quality modulation*: today the food trail is laid at one strength for any
   load (§2.3), and a fill-graded emitter is the odometer's zeroth step.
   **Cost:** the `trailfollow` riders are archived; one 12-seed run at gap 90
   per arm. **Score on:** ants reaching food, never deliveries (recruitment,
   not homing). **Falsifier:** the `mute` arm.
2. **`(Crowding, EmitB, −w)`** (survey §3, Czaczkes 2013). One wire in
   `ant.ron`; in the genome, so a line can lose it; no new state, no slot.
   What it buys is the survey's *"self-organised negative feedback that
   downregulates recruitment in crowded parts of a network"* — the deposit-side
   half of what `Crowding → Move` does on the reader side, and the mechanism
   `dead-ends.md:1869` says would change §Z7's rejection: a trail that stops
   recruiting once its patch is crowded. **Cost:** derive `w` from the
   `Crowding` band actually occupied (the nest plan measured the gate
   compressing 26x across it — check the input's range before authoring
   against it). **Score on:** the mirrored race in `dead-ends.md:1860`'s
   condition — `labforage plant=` with a patchy larder, readers against
   non-readers. **Falsifier:** if the race still goes 1:3 against readers,
   the deposit side is not the lever.
3. **The no-entry mark** (survey §3, Robinson 2005/2008). The one mechanism in
   the survey that targets §Z7 by name: mark the way to an exhausted site as
   repellent, and mark it *longer* than the attractant lasts (78 min against
   33 in the ants). **Do not build a fourth plane.** A negative deposit on
   channel B — subtract at the vacated cell — needs no new reader: the
   existing along-gradient simply points elsewhere. What it needs is the
   *emitter condition*, which is state: *I was at food and now there is none
   and I am empty-handed*. That is a hidden unit with recurrence, authorable
   in `ant.ron` without a slot bump. **Cost:** the `EmitB` output is unsigned
   today; a signed emit, or a second output, is the engine change. **Score
   on:** the same patchy-larder race. **Rank it after item 2**, because item 2
   is one line and this is a verb.
4. **Switch the home bearing on, and put a dial on it** (survey §6). Not
   new — it is built (`home_bias`, §2.5) — but the survey is a second
   independent source saying the trail is the wrong primitive for homing, and
   the mechanism sits in the tree at zero with no dial and no wiki line. Two
   things to do, in the owner's hands rather than a lane's: expose
   `home_bias` on the species page beside `scent_drift`, and decide whether
   it ships on under *ship everything on*. The number to have first is a
   six-seed sweep of the bias on `trailfollow mode=gap`, scored on
   `carry@nest` and `trips` and paired against the unlanded lifetime split,
   because the two are the same verb approached from the plane side and the
   animal side and **must not land together**. Expect the same bill the
   lifetime branch found: trips up, intake down on a hungry bed, until a
   granary makes a delivery worth something. **Cost:** none in code; a
   species-file field and a sweep. The survey's *vector-length weighting*
   (§2.5) is the follow-on, and that one is a slot bump — leave it until the
   bias alone has a number.
5. **Trail lifetime by purpose, not by species** — nothing to build; a
   framing to carry into the species tables when they arrive (§6). Where the
   survey's Table 1 lists a half-life per species, the engine's knob is a
   plane's `rho`/`diffuse` pair and a species' emitter odometer, and the
   lifetime that matters is measured in *round trips* (`pherolife sweep=rho`
   prints it that way).
6. **Pellet-attracted deposition** (survey §7, Khuong 2016). The deposition
   half of the nest plan's Stage 4: a `DropSpoil` preference for cells
   adjacent to *existing* spoil. `dead-ends.md:1095` says how to build it —
   *"a wired instinct on its own brain output rather than a coefficient in
   `act`, so a lineage can lose it"* — and the geometric proxy already wired
   (`SurfaceCurvature`) is the reason to expect a small effect: a heap is
   curved and that read a tenth steeper. The difference Khuong's rule buys is
   *self-amplification on a marker with a 20-minute lifetime*, which curvature
   does not have. **Score by eye**: the mound is the artifact the owner has
   reported three times. **Gate:** after the nest plan's Stage 0 (the census
   undercounts the nest 3x) or the number is wrong.
7. **Laden right-of-way** (survey §5). One predicate on `try_swap_with_kin`.
   Low, because congestion is measured not to bind, and because the swap
   itself reduced intake. Keep on the list for the day stacking ships on.

**Not to take**, with the reason: lanes and bidirectional traffic (no lateral
axis, §5); the bifurcation-angle polarity (no bifurcations); osmotropotaxis
(built, dead on a surface); tandem running and quorum nest choice (no verb,
no need, no relocation); central-complex and mushroom-body models (the survey
itself says approximate); a digging pheromone (measured negative); termite
and wasp construction (no consumer, and the nest has no purpose yet); fixed
authored castes (owner ruling); an ODE demography layer (the lab is about
individuals).

---

## 4. Where the survey is wrong for this engine

Stated so nobody builds from the survey's sentence where the record has the
number.

1. **"Evaporation drives selection" — on a one-cell trail, diffusion is the
   eraser.** `DECAY_RHO` at zero changes an unreinforced trail's life by
   nothing; `DIFFUSE` alone moves it 144 → 432 frames
   (`pheromone-lifetime-and-wiring`). The survey reasons in the continuum,
   where a trail has width. The same error in the other direction: `DIFFUSE`
   at 0.1 on a `u8` plane *did nothing at all* — the blended value rounded to
   zero before reaching the second cell (`dead-ends.md:1207`).
2. **"Cheap to implement" is not cheap here.** Every new sense is a
   `BRAIN_INPUTS` bump: `live_slots` moves, `mutation_rate` is re-derived in
   six species files, every `creature_space` baseline is void, and the
   `plainspeak` name-length guard has two characters of slack
   (`pheromone-master` §7 step 4). The survey's list of "all cheap"
   mechanisms is four inputs and two outputs.
3. **The gradient of a trail does not encode direction, and the survey says
   so in §6 while §3 assumes it.** The engine paid for this: the food plane
   is laid only while laden, so its older end is the food end and its
   gradient points at the *nest* — *"the food half is not shapeless, it is
   shaped backwards"* (`pheromone-trail-direction`). A choice function over
   concentrations picks a branch; it cannot pick a way along a line.
4. **Narrow storage takes the gradient away before it takes the trail.** The
   survey never mentions quantisation. The `u8` plane held a trail whose far
   half read an along-gradient of exactly 0.000 while still standing; the
   `u16` widening bought a ten-times-longer readable trail with no constant
   touched (`pheromone.rs` `Scent` doc; `decaying-gradient-quantization-2026-09-15.md`).
5. **The five "gaps in existing games" are not this engine's gaps.** §0.
6. **Species parameters are not what makes a Pharaoh ant and a leaf-cutter
   "different games" here.** What differs between colonies in this engine is
   heritable and found — pace, gut, scent, patience, eyes, and a caste a line
   may reach — under the owner's standing direction to *expose, not tune*.
   A species file is a starting point that *"ships inert"* (README,
   *Parameter-genome status*). Tables of half-lives and walking speeds are
   useful as the *range* a dial should reach, not as values to author.

---

## 5. The side-view caveat, which cuts across everything

`stigmergy-research.md` §7 stated it before any ant existed and it decides
which half of the survey transfers: **every foraging diagram in this
literature is top-down, and this engine is a vertical section.** The
double-bridge needs terrain to stage two routes; lanes have no axis to form
in; a trail has almost no bifurcations; the side antennae read floor and air.
The survey's §3–§5 are the top-down half. Its §7 is the side-view half —
every real nest cross-section and every one of Toffin's arenas is a vertical
section — and it transfers, once the ground is deep enough to hold a nest.
`nest-entrance-dimensions-2026-09-19.md` makes the rule quantitative: sort
every quantity by the plane it was measured in before converting it.

---

## 6. Pending — the rest of the survey

The copy received stops at *"Metabolic scaling:"* in its §9. Not reviewed,
because not received: the rest of §9 (metabolic scaling, colony growth, the
life cycle), any section on existing games, and **Tables 1–4**, the
species-parameter tables the TL;DR refers to. When they arrive:

- the tables go against `assets/species/ant.ron`'s scalars and the lab's
  parameter page, as *ranges a dial should reach* (§4 item 6);
- metabolic scaling goes against the per-cell cost model and
  `creature-body-extent-2026-08-30.md`;
- the life cycle goes against `nest-biology` §11 (eggs) and
  `creature-reproduction-economics.md` §2.3 (the founding queen converts her
  body into brood — already the engine's *fission* candidate).

---

## 7. What this review rests on

Repo documents cited: `stigmergy-research.md`; `pheromone-master-2026-09-17.md`
and the three reports under it; `nest-design-2026-09-14.md`;
`nest-biology-2026-09-19.md`; `nest-digging-plan-2026-09-19.md`;
`nest-shape-three-negatives-2026-09-19.md`; `nest-entrance-dimensions-2026-09-19.md`;
`colony-economy-design-2026-09-09.md`; `creature-signature-and-castes-2026-09-06.md`;
`creature-stacking-design-2026-09-17.md`; `decaying-gradient-quantization-2026-09-15.md`;
`Reports/dead-ends.md` (entries at the lines cited); `Reports/open-bugs-handoff.md`
(§R4, §T2, §Z6, §Z7); `wiki/ants.md`; `lanes/evolution-lab-coordinator.md`
(owner rulings); `lanes/evolution-lab-pheromones.md`;
`lanes/nest-biology-research.md`; `lanes/nest-digging-handoff-2026-09-19.md`.
Unlanded: `nest-biology-digging-signals-2026-09-19.md` and
`nest-build-plan-2026-09-19.md` on `claude/nest-biology-research` (PR #472);
the trail-lifetime split on `claude/upbeat-shannon-cez0w4`.

Source cited: `src/sim/pheromone.rs`, `src/sim/brain.rs`, `src/sim/creature.rs`,
`src/sim/organism.rs`, `assets/species/ant.ron`, at the lines given.
