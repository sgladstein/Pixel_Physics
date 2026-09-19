# Review of an outside literature survey on ant simulation — what the engine already does, what it has tried, and what is worth taking

*2026-09-19. Review of
[`ant-sim-literature-review-external-2026-09-19.md`](ant-sim-literature-review-external-2026-09-19.md),
a survey written by someone who knew only that this project is an ant-based
simulation game. **Docs only; nothing built, and nothing run except the
bounded cost measurements in §8.** Every claim about
the engine below was checked against the tree or the repo's own measured
record and says where. The survey arrived in two parts the same day; §2
covers all nineteen of its sections, §5 its four parameter tables, §6 its
twelve staged recommendations. **§8 was added later the same day and is the
one part that ran anything**: a bounded set of cost measurements after the
owner challenged §2.13's reading of the field, which it corrects.*

## 0. The answer, stated once

**The survey's architecture is this engine's architecture, and its central
recommendation was taken before it was written.** Stigmergy-first, agents on
local rules over an evaporating field, a Deneubourg choice function, no
pathfinding — that is `stigmergy-research.md` (status *implemented*),
`src/sim/pheromone.rs` and `src/sim/brain.rs`. By the tally in §2.15, of the
survey's ~85 named mechanisms this engine ships **31**, has **4** built and
switched off, **7** designed and priced, **10** built and measured as rejected
or inert, **9** absent and cheap, and **24** it cannot stage or does not want.
**Three of the nine are worth a run now.**

The survey names five things existing games omit — multiple pheromone types,
quorum decisions, metabolic scaling, path integration, topochemical
construction — and calls that gap "exactly where realism gains lie". For this
engine the measured binding constraints are elsewhere, and the record says so
with numbers:

| the survey's five gaps | here |
|---|---|
| multiple pheromone types | three planes ship (home, food, alarm), each with its own lifetime; a fourth is held until a consumer exists |
| quorum nest choice | no colony ever relocates; no verb, and no nest worth leaving yet |
| metabolic scaling | every joule is charged per body cell (`ant.ron:118-119`); linear in ants by construction, and nobody has measured whether crowding already bends it (§3 item 8) |
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
   food*; the survey's §18 *Trigona* polarity trail is the same shape) was
   built here as a food-charged odometer on `EmitB` and **rejected on
   2026-09-17** — one day before the return leg was fixed. Its rejection
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

**Where this engine is ahead of the survey's field.** Its §19 lists as an
open problem *"realistic underground architecture coupled to digging physics
and soil mechanics"* and *"multi-pheromone ecosystems with nestmate
recognition seldom modelled together"*. This engine has a per-cell load
model, packed soil that holds a gallery, spoil pellets that need footing,
water that drowns a burrow, three pheromone planes and a heritable scent
signature, all in one tick. The nest line's problem is not that the physics
is missing; it is that the colony scratches the whole floor instead of
digging a nest (`nest-shape-three-negatives`).

**Two corrections the survey needs before anyone builds from it here** (§4):
evaporation is *not* the parameter on a one-cell trail — diffusion is the
eraser, measured, and the shipped decay rate is inert; and a new brain input
is not "cheap to implement" in this engine — it re-derives `mutation_rate` in
six species files and voids every stored baseline.

---

## 1. What was checked, and how

Read: the survey in full; `wiki/ants.md` in full (the bar); `pheromone-master-2026-09-17.md`
in full; `stigmergy-research.md` in full; the module docs of `pheromone.rs`
and `brain.rs`; every authored wire in `assets/species/ant.ron`; the
`tumble` verb and `home_weighted_pick` in `creature.rs`; the openings and
decision tables of `nest-biology-2026-09-19.md`, `nest-digging-plan-2026-09-19.md`,
`nest-design-2026-09-14.md` §2, `colony-economy-design-2026-09-09.md`,
`creature-signature-and-castes-2026-09-06.md`, `creature-stacking-design-2026-09-17.md`;
the two nest reports still on `claude/nest-biology-research` (PR #472) and the
trail-lifetime work on `claude/upbeat-shannon-cez0w4` (unlanded, 20 ahead);
`Reports/dead-ends.md` grepped by mechanism, per `CLAUDE.md`, for every
mechanism the survey proposes; the open-bug index.

Not done for §§0–7: no binary was built, no scene was run, no card was
posted; every number in those sections is the repo's own, cited to the
document that measured it. §8, added later, built the release examples and
ran four cost instruments; it says which and prints the commands.

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
  the load is worth. The survey's shape — strongest at the food, fading toward
  home — is exactly the *food-charged odometer on `EmitB`* that was built as
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
  the along-heading difference. A side-view consequence, §7.
- **Trail-geometry polarity** (Jackson 2004, the bifurcation angle): discussed
  in `pheromone-trail-direction-2026-09-16.md`'s biology section; a
  side-view trail has almost no bifurcations to read.

### 2.3 Foraging and recruitment (survey §4)

| survey | engine, measured | verdict |
|---|---|---|
| mass recruitment via trail | a hand-laid food trail is decisive — **92% of ants reach food against 3%** unaided, 4 of 6 colonies alive against 0 (`pheromone-master` §3.2); **the colony cannot lay one itself** (`self ≡ mute`); discovery is the binding constraint | mechanism works; bootstrap does not |
| trail modulated by food quality (Beckers 1992) | **absent.** `EmitB` reads `CarryingFood`, a boolean (`creature.rs:5209`); a load's worth reaches nothing the emitter reads. The graded input exists — `Carrying` is crop fill — but under `SPOIL_IS_CARGO` it reads 1.0 for a pellet of dirt, so a fill-graded emitter needs a food-only graded sense, which is a slot bump | one of the arms of §3 item 1 |
| decay rate tuned to ephemeral vs persistent resources | `Pheromones::set_channel_rho` and `set_channel_diffuse` are per-plane dials (`pheromone.rs:869`, `:898`); `labforage bdecay=` sweeps it | dials exist; the sweep `dead-ends.md:1869` names as the re-test has not been run |
| negative feedback preserves flexibility (Grüter) | `(Crowding, Move, −0.3)`; food route left near-deaf **on purpose** — read at full gain the colony *"eats out its own doorstep"*, a quarter of the animals against three quarters in a mirrored race (`wiki/ants.md`; §Z7) | the survey's remedy is the engine's open problem. **Superseded 2026-09-19 (Lane T):** the reader was de-saturated on the 18th (`ac02ac03`) and that change is the whole hand-trail effect — 22 colonies of 36 alive against 4; `ant-survey-trail-reevaluation-2026-09-19.md` §3 |
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
mechanism is already built.**

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
  approximation.

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

### 2.8 Life cycle, demography and energetics (survey §9)

| survey | engine | record |
|---|---|---|
| nuptial flight → claustral founding → nanitics → ergonomic growth → alates | a colony is **founded by a click**: ~50 workers stood on a patch (`wiki/ants.md`, "Placing a colony"); no queen, no alates, no flight. An ant *buds* — *"there is no queen and no egg"*. A queen regime exists behind `PIXEL_PHYSICS_BREEDING=queen` (`creature.rs:9308-9370`), and the standing ruling is that a queen is *"three authored values over existing mechanisms, never a type the engine knows"* | founding-by-one-queen is `colony-economy-design` §5c and `creature-reproduction-economics` §2.3/§3.2 (*fission*: the parent's body becomes the child's) — designed, not built |
| brood stages, temperature-dependent development | none. The substrate is there — `FieldCell` carries a depth-graded temperature with the day/night forcing separable exactly — and nothing reads it but `(TempAboveAmb, Turn, −0.8)` | eggs designed in `nest-biology` §11; the lane's decision list says no worker age |
| sigmoidal colony growth | the engine got boom-and-crash first: a colony *"finds a bed full of food, breeds into the hundreds, eats it to bare ground, and goes all at once"*; **lifespan** is what settles it — *"near how fast it breeds times how long an ant lives"* — and the fall is a slope, not a cliff | shipped 2026-09-12 (`wiki/ants.md`, "Ants get old") |
| hypometric metabolic scaling (exponent 0.75–0.93); per-capita metabolism falls with colony size; the group effect vanishes in isolation | **linear by construction**: every ant is billed per body cell for standing, walking and thinking (`ant.ron:118-119`), so colony burn is ants × per-ant burn, exponent 1.0. **But** locomotion is 52% of burn (`colony-economy-design` §2) and `(Crowding, Move, −0.3)` makes a crowded ant walk less — so a per-capita burn that *falls* with colony size may already be emergent, and nobody has measured it | §3 item 8 — a measurement, not a build |
| trophallaxis as a distribution network; famine relief | shipped on, 2026-09-09: `Share` output, `KinNeed` input, rich gives to poor, and since 2026-09-19 across a stacked cell. **First measurement is the opposite of famine relief**: in a bare box sharing *flattened the founders' spread of reserves back into sameness* and ended with fewer survivors on two runs of three — because the spread of reserves is exactly a rich-and-poor, and sharing removes it (`wiki/ants.md`, "Feeding each other") | the survey's "model as flow on the network" is what happened, and the flow erased the variance the founding cliff needed |
| food storage: repletes, seed caches, fungus gardens | **repletes are the crop** — `store_in_body`, the granary-versus-replete gene, was found redundant against the `Feed`/`Drop` weights (`dead-ends.md:1143`); **seed caching is half there** — a harvester *"became a sower"*: a bitten seed rides home as cargo and is set down where the meal ends, so the midden sprouts, but *"there is no granary in this box"* (`larder_probe`: ten cells in transit, resident 0); no fungus | `nest-biology` finding 7 is the argument for a granary a player can see, rob and lose |

### 2.9 Collective decisions — nest-site selection (survey §10)

**Not staged, and the prerequisite is not the mechanism.** No colony here
relocates: there is no candidate site, no assessment verb, no transport
verb. The survey calls emigration "compelling emergent gameplay", and it
would be — but a colony only leaves a nest that is *worth* something, and
by owner ruling this one is not yet (`nest-biology` §10). Quorum by
encounter rate is the one part the engine already computes: `Crowding` is a
5x5 kin count, which is Pratt's quorum signal; a second `NestSite` would be
the candidate. Rank it after eggs or a granary give the nest a purpose.

**The collective decision this engine does have is a different one.** A
colony *splits* when a lineage's scent drifts past the rest's tolerance:
`World::regroup_by_scent` finds the connected clusters of mutual kin and
names the new one (`ANT 1b`), *"and the two start to bite each other when
hungry"* — while a thread of ants walking between two mounds keeps them one
colony indefinitely (`wiki/ants.md`, "Who is family"). That is speciation
and colony fission arrived at with no quorum rule, and it is closer to the
survey's §13 supercolony biology than to its §10.

### 2.10 Collective transport and self-assembly (survey §11)

| survey | engine | verdict |
|---|---|---|
| cooperative transport of loads 10,000x an ant's weight | a load is one crop per ant; a corpse or fruit is carried a cell at a time. No shared load, no force vectors | no consumer; a two-cell ant has nothing to haul that needs two of them |
| rafts, towers, treadmilling | **an ant is footing for a nestmate** (`climbs_over_kin`: *"a nestmate is something to stand on, the same as a rock"*), so a pile of ants is physically expressible and *"an ant standing on a nestmate that walks away has further to drop"*; stacking (many in one cell) is the other axis, off by default | the substrate for a tower exists; nothing motivates one, and nothing that ships would build one |
| army-ant bridges by cost–benefit | same footing rule across a gap; no verb | not now |
| circular mills — the artifact of pure trail-following | the engine's word for it is in `pheromone.rs`'s module doc: *"One channel gets milling; two get commuting"* — the reason there are two planes | already designed around |

### 2.11 Movement and locomotion (survey §12)

- **Speed.** An ant here *"only moves on one frame in six even at full
  speed"* and is two cells long (`wiki/ants.md`), so it covers a twelfth of a
  body length per frame — about five body lengths a second if the app runs
  at sixty frames a second, against the survey's 2–10 for *Lasius* on a
  trail and ~108 for the silver ant. **Pace is heritable** since 2026-09-05:
  *"one lineage takes its turn twice as often as its neighbours and another
  half as often"*, charged per turn so the quick one starves first on a lean
  bed. A **laden ant walks slower and tires faster** (Table 2's *Atta* row).
- **Temperature dependence of speed, and of evaporation**: **absent**.
  `TempAboveAmb` reaches only `Turn`; the planes' `rho` is global. Both are
  one scale each and both would show in the outdoor game's day and seasons
  (`weather.md`). Note `CLAUDE.md`'s divide-the-oscillator-out rule: a trail
  that fades faster at noon is a designed cycle reaching a decision, fine on
  screen, and every measurement taken across it must remove the phase.
- **Correlated random walk**: on a one-dimensional floor a turning-angle
  distribution collapses to one number, the reversal rate, and that is
  `BrainOutput::Persist` — *"this engine's own milling-versus-commuting
  number"*. `Tumble` re-rolls uniformly among viable headings, and on a flat
  floor the viable set is mostly {left, right}. Not a gap.
- **Gait**: none; a two-cell body has no legs. **Body-size effects**: a bigger
  animal costs more (per cell), a longer one flows over broken ground better,
  and `crop_capacity` is a species scalar rather than a body-linked one — the
  survey's "larger workers carry more" is authorable through the
  developmental block, not automatic.

### 2.12 Inter-colony and interspecies interactions (survey §13)

| survey | engine | record |
|---|---|---|
| territoriality and warfare | **there is no enemy**: *"nobody attacks a stranger it is not going to eat"*; strangers are food if the gut digests them and furniture otherwise. Colonies split by scent and then *eat* each other when hungry; `Attack` (biting what you will not eat) exists and *"nothing that ships is born doing it"* | `why-colonies-do-not-fight-2026-09-14.md` is a whole report on this; `animal-conflict-research-2026-09-14.md` the biology |
| Argentine supercolonies: non-aggressive within, aggressive between | **this is the shipped default.** Every click is one family while they smell alike; *"the thread of ants between two mounds is the whole difference between one colony living in two places and two colonies"*, because a visiting ant re-mixes the mound's smell (`nest blend`, `nest uptake`, `nest scent drift` dials) — which is the real supercolony mechanism, continuous mixing | `wiki/ants.md`, "A nest is a place that holds a smell" |
| slave-making, social parasitism, aphid tending, fungus farming | none. The nearest thing to farming is the seed-carrying loop — *"the colony that gardens survives"* is the lab's stated direction — and it is dispersal, not cultivation | coordinator note, standing direction |
| army-ant raids; predator–prey | a beetle eats an ant *"because an ant is meat and the beetle is hungry"*; beetles measured **not yet frightening** — shelter is worth twice as much to an ant with or without predators in the world | `wiki/ants.md`, "A beetle can see"; `population-dynamics-research.md` on why two-species systems go extinct |

### 2.13 Validation, tools, games and other superorganisms (survey §14–18)

- **Tracking data and pattern-oriented modelling.** The survey's validation
  practice — match several emergent patterns at once, never one — is
  `CLAUDE.md`'s method under other names: order statistics over seeds, paired
  comparisons, *look before you measure*, the acceptance scenes and
  `seedsweep`. What this repo does not do is calibrate against real
  trajectories, and the owner has ruled it does not have to: *"It does not
  have to perfectly match how real ants, but we should take inspiration when
  we can"* (`pheromone-master` §6). The instruments that stand in for
  tracking are `labstats`, `latecensus`, `colonybooks`, the chronicle, the
  life record and the watch page.
- **GPU fields, ML, swarm robotics.** The planes are CPU, double-buffered
  Jacobi (`dead-ends.md:1119` says why not in-place), and cost 0.0014 ms
  *settled* at the shipped world; `pherocost` prices any size. **That
  figure is the wrong state to quote against a played colony — see §8**,
  where the planes and the field both scale with ants and at 129 ants the
  planes cost twice the field. Determinism is required (`PLAN.md`), which
  rules out the survey's RL and most GPU reductions as decision inputs. M10
  streaming is the known migration; the GPU field is conditional on §8.5.
- **ACO.** The survey's verdict — take the evaporation-plus-choice formalism
  and nothing else — is what `stigmergy-research.md` §3 did: it took the
  ρ band from the ACO literature as a first guess and the engine then
  measured that band inert on a one-cell trail.
- **The games.** *Empires of the Undergrowth*'s player verb — command by
  pheromone marker, never by order — is the lab's *hand in the box* (drag a
  scent trail, drop an alarm, fling an animal). *SimAnt*'s castes, trails and
  colony-versus-colony are all here in found rather than authored form. The
  hobbyist compute-shader sims' three-sensor agent is the design this engine
  tried and replaced (§2.2, osmotropotaxis). The survey's list of what games
  omit is answered in §0.
- **Other superorganisms.** BEEHAVE's demography-plus-energetics-plus-spatial
  foraging is the shape of `colonybooks` and the founding cliff. The
  **flitter** is the engine's bee: nectar-only, flying, and *"still about
  seven times short of feeding itself"*. *Trigona*'s polarity trail — more
  pheromone near food — is §3 item 1 again. Hive and mound thermoregulation:
  `nest-biology` §5 decided no ventilation and no CO₂, and named the
  unread temperature field as the cheapest thing in the report.

### 2.14 Open problems (survey §19)

Three of the survey's six are this engine's strengths, two are its stated
positions, one is shared. *Underground architecture coupled to digging
physics and soil mechanics*: here, and the problem is shape, not physics
(§0). *Multi-pheromone with nestmate recognition together*: three planes and
a heritable scent signature in one tick. *Individual variation*: every
number an ant has is heritable; *learning* is deliberately absent — the lab
is about evolution. *Uncalibrated*: yes, by ruling. *Integrating scales* and
*colony-level cognition as a whole*: shared, and the master report's
diagnosis of the trail line is one instance of it.

### 2.15 Scorecard

By survey section. *Ships* means in the tree and on; *off* means built,
default-off; *designed* means priced in a report and not built; *rejected*
means built or literature-tested here and measured negative or inert;
*cheap* means absent and expressible as a wire, a predicate or a run;
*cannot / not wanted* means unstageable in side view, ruled out, or without a
consumer. The classification is this review's; the rows above are the
evidence.

| survey § | ships | off | designed | rejected | cheap | cannot / not wanted |
|---|---|---|---|---|---|---|
| 1–2 stigmergy, modelling | 4 | | | 1 | | 1 |
| 3 pheromones | 5 | | | 4 | 4 | 1 |
| 4 foraging | 3 | | | 1 | | 2 |
| 5 traffic | | 1 | | 1 | 1 | 1 |
| 6 navigation | | 1 | 1 | | | 2 |
| 7 nest | 2 | 2 | 3 | 3 | 1 | 3 |
| 8 division of labour | 4 | | | | 1 | 1 |
| 9 life cycle, energetics | 4 | | 3 | | | 3 |
| 10 collective decisions | 1 | | | | | 1 |
| 11 transport, assembly | 1 | | | | | 4 |
| 12 movement | 3 | | | | 2 | 1 |
| 13 inter-colony | 3 | | | | | 4 |
| 14–18 validation, tools, games | 1 | | | | | 0 |
| **total** | **31** | **4** | **7** | **10** | **9** | **24** |

---

## 3. What to take, ranked

Ordered by what it costs against what it could move, with the dead end that
bears on each and the instrument that would score it. **None of these should
land in the same change as another**: every one adds a term to a shared
weighted sum, and `CLAUDE.md`'s *a correct mechanism at inherited constants is
a regression* makes a joint result unattributable.

1. **Re-test the food-charged odometer on `EmitB`** (survey §3, Beckers /
   Czaczkes 2024; §18 *Trigona*). Rejected 2026-09-17 (`dead-ends.md:1925`)
   on a bed where **no laden ant ever got home** — the master report's own
   banner warns *"every `self` number in this document predates a working
   return leg."* The return leg was fixed 2026-09-18 and the way home keeps
   on the unlanded lifetime branch. A trail that is strongest at the food and
   fades toward the nest is only readable *by an ant walking outward from the
   nest*, which is the leg that now exists. The cheaper arm to run first is
   the survey's *quality modulation*: today the food trail is laid at one
   strength for any load (§2.3), and a fill-graded emitter is the odometer's
   zeroth step. **Cost:** the `trailfollow` riders are archived; one 12-seed
   run at gap 90 per arm. **Score on:** ants reaching food, never deliveries
   (recruitment, not homing). **Falsifier:** the `mute` arm.
   **Outcome (Lane T, 2026-09-19):** inert at both reader gains, 36 seeds; the rejection stands, narrowed to the 2x2 that was never run.
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
   **Outcome (Lane T, 2026-09-19):** null at w = 2.35; the w = 4.5 survival gain is the emission cost falling, shown by a no-channel-B control. And §Z7's reader half was already resolved on the 18th (`ac02ac03`), which this item did not know.
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
   **Outcome (Lane T, 2026-09-19):** not built — `self ≡ mute`, so there is no recruitment to a stale patch for a no-entry mark to correct.
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
   framing carried into §5 against the survey's Table 1. Where the survey
   lists a half-life per species, the engine's knob is a plane's
   `rho`/`diffuse` pair and a species' emitter odometer, and the lifetime that
   matters is measured in *round trips* (`pherolife sweep=rho` prints it that
   way).
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
7. **Laden right-of-way** (survey §5, §12). One predicate on
   `try_swap_with_kin`. Low, because congestion is measured not to bind, and
   because the swap itself reduced intake. Keep on the list for the day
   stacking ships on.
8. **Measure whether colony burn is already hypometric** (survey §9, Table 3).
   A measurement, not a build, and cheap: `colonybooks` prints each colony's
   burn split into upkeep, walking and brains. Run one bed at four colony
   sizes — a dozen, fifty, two hundred, eight hundred — and read burn per ant.
   If it falls with size, `(Crowding, Move, −0.3)` has given the engine the
   survey's exponent for free and it is worth a wiki line. If it is flat, it
   is a known absence to *register*, under the ruling that a default that
   looks wrong is reported and never tuned. **Trap:** pin `RAYON_NUM_THREADS`
   and take the bed at one age — the coordinator note says bed age and plants
   paying the ants' bill are what a creature-cost harness measures otherwise.
9. **Temperature on pace and on evaporation** (survey §12). Two scalars, both
   absent, both visible only in the outdoor game's day and seasons. Low, and
   the second one is a designed oscillator reaching every trail measurement
   — divide it out or every `pherolife` number moves with the hour.

**Not to take**, with the reason: lanes and bidirectional traffic (no lateral
axis, §7); the bifurcation-angle polarity (no bifurcations); osmotropotaxis
(built, dead on a surface); tandem running and quorum nest choice (no verb,
no need, no nest worth leaving); central-complex and mushroom-body models
(the survey itself says approximate); a digging pheromone (measured
negative); termite and wasp construction (no consumer, and the nest has no
purpose yet); fixed authored castes (owner ruling); an ODE demography layer
(the lab is about individuals); procedural nests seeded from Tschinkel casts
(§4 item 8); cooperative transport, rafts and bridges (no load needs two
ants; no verb); learning and route memory (evolution, not learning, by
design); calibration against tracking data (owner ruling).

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
   useful as the *range* a dial should reach, not as values to author. §5.
7. **"The colony never adapts → increase evaporation"** (the survey's
   redesign threshold) is the wrong knob twice over: decay is inert on a
   one-cell trail, and `dead-ends.md:1067` records that more evaporation does
   not fix ossification here — the crowding input does. Its other half,
   *"ensure negative feedback is active"*, is right and is §3 items 2 and 3.
8. **"Seed procedural nest shapes from Tschinkel cast data"** contradicts
   the one rule this engine will not bend: nothing is painted where the
   world could produce it. The owner's *no paint* ruling on the nest
   (`nest-design` §13) retired even the door as a material. Tschinkel's
   profile is the *bar* a dug nest is judged against (`nest-biology`
   finding 3), never a template stamped into the ground.
9. **"Run the field on a GPU with ping-pong textures"** is sound and, at
   the populations any harness here can stand, unnecessary: the planes are
   already double-buffered, and a GPU reduction feeding a decision would
   cost the determinism `PLAN.md` requires. **Withdrawn as a flat "no" by
   §8**: the field and the planes scale with ants by a path the record had
   not measured, and the GPU field is now conditional on the owner's own
   per-phase number (§8.5 item 4).

---

## 5. The four tables against the dials

**Table 1 — persistence.** The engine's unit is not minutes but *round
trips*, and the comparison to make is the ratio of trail life to trip time,
which is what every species row implicitly reports.

| survey row | engine, measured |
|---|---|
| Pharaoh ~9 min attractive, ~78 min repellent; fire ant seconds–minutes | the **food plane** as shipped: readable for **0.49 of a round trip**, gone at 0.67 (`pherolife sweep=rho`, on its own 2,200-frame trip); 144 frames unreinforced before the `u16` widening. Pharaoh-like by ratio, and deliberately so — *"it is news about a patch"* |
| Argentine 30 min–4 h; *Lasius* ~20 min building marker | the **home plane** on the unlanded lifetime branch: readable for **1.06 round trips**, gone at 2.04 — and still gone, by blurring, which is the point |
| army ants days to a week | nothing here persists that long, and nothing should: a trail that outlives its patch is the named failure (§Z7). The nearest analogue is the nest's own smell, which never fades (`NestSite::scent`) |
| deposition ~0.5 units/s; 22x near the food | `DEPOSIT 40` per successful move, scaled by the emitter's output; the food emitter is flat (§2.3). The 22x is §3 item 1 |
| repellent effect lasts 2.4x the attractive one | §3 item 3's design note: the no-entry subtraction should outlast the deposit it cancels |

**Table 2 — speed.** Cells per frame is the unit; §2.11 has the conversion.
Two of the table's four rules ship (laden slower; bigger costs more) and two
do not (temperature; body size → faster). The one row that transfers as a
*design* rather than a number is the silver ant: solitary, path-integrating,
no trail — which is the `home_bias` ant with `EmitB` at zero, and is
authorable today as a species file.

**Table 3 — colony size, demography.**

| survey row | engine |
|---|---|
| *Temnothorax* 100s; *Lasius* 1,000s–10,000s | a founding is ~50; lab beds reach hundreds to ~3,000 (`evolution-lab-coordinator`: 3,099 at one stop; stacking measured at 2,000) — the *Lasius* range at the top, *Temnothorax* at the bottom |
| active foragers ~10% | **the opposite**: *"almost every ant is carrying food almost all the time — better than nine in ten"* (`wiki/ants.md`). There is no forager caste and no reserve caste; rest is per ant by energy. The survey's 10% is a *Cataglyphis* number, a solitary desert forager |
| hypometric burn | linear by construction; §3 item 8 |
| nest volume ∝ workers, logistic | by construction; and the room-per-ant gate reopens when the brood outgrows it (`wiki/ants.md`, "Digging, and the mound") |
| quorum ∝ adult workers | not applicable, §2.9 |

**Table 4 — decision parameters.** `n = 2` ships. `k` does not transfer:
`choose_weighted`'s `k` is in the candidate score's own units (the test
asserts at `k = 0.1`), not in pheromone concentration, so the survey's 20 is
not a value to author. Response thresholds and contact rates: §2.7. Quorum:
§2.9. Memory × pheromone: `Persist` is half of it and no memory is the
other half.

---

## 6. The survey's staged recommendations against the repo's plans

| survey step | here |
|---|---|
| 1. one field per pheromone; deposit, diffuse, decay per tick; a different half-life per pheromone **and species**; GPU ping-pong | done on the CPU, double-buffered (`dead-ends.md:1119`); per plane yes, per species through the emitter odometer. §4 items 1 and 9 |
| 2. CRW for search; three points ahead; Weber + noise; never A* | done, except the three-point sensor is dead on a surface and the along-heading reader replaced it (§2.2). The survey's two benchmarks are the wrong knobs here: trails not dissolving is a `DIFFUSE` question, and two equal sources cannot be staged on a flat floor — `pherolife mode=junction` is the engine's fork test |
| 3. trail + alarm + no-entry | two of three; §3 item 3 |
| 4. path integration + optional view familiarity | **built, off** (§2.5); §3 item 4. View familiarity: never |
| 5. right-of-way → lanes | no lanes possible; §3 item 7 |
| 6. thresholds + interaction rates + inactive reserve | done (§2.7) |
| 7. an ODE demographic layer | deliberately not; lifespan gives the plateau (§2.8) |
| 8. Khuong construction; procedural nests from casts | deposition half is §3 item 6; the casts are the bar, never a template (§4 item 8) |
| 9. quorum emigration | after the nest has a purpose (§2.9) |
| 10. spatial hashing, LOD, mean-field reserve | chunks that sleep and a scheduler are the LOD; off-camera is `ecological-lod-design.md`; mean-field never |
| 11. species parameters as dials | the lab's parameter page and the species files, under *expose, not tune* (§4 item 6) |
| 12. validate against published patterns and tracking data | against the wiki's bar, seed sweeps and the owner's eye; not against tracking, by ruling (§2.13) |

The survey's three redesign thresholds: *ants look robotic* — not this
engine's failure; *large colonies tank performance* — the perf line's
standing kill condition is 5% whole-frame, and the creature pass is the
place to look, not the field; *the colony never adapts* — §4 item 7.

---

## 7. The side-view caveat, which cuts across everything

`stigmergy-research.md` §7 stated it before any ant existed and it decides
which half of the survey transfers: **every foraging diagram in this
literature is top-down, and this engine is a vertical section.** The
double-bridge needs terrain to stage two routes; lanes have no axis to form
in; a trail has almost no bifurcations; the side antennae read floor and air;
a turning-angle distribution is a reversal rate. The survey's §3–§5 and §12
are the top-down half. Its §7 is the side-view half — every real nest
cross-section and every one of Toffin's arenas is a vertical section — and it
transfers, once the ground is deep enough to hold a nest.
`nest-entrance-dimensions-2026-09-19.md` makes the rule quantitative: sort
every quantity by the plane it was measured in before converting it.

---

## 8. Performance — measured after the owner's challenge, later the same day

*Added 2026-09-19, later. The owner read §2.13 and §4 item 9 and pushed
back: "I think the field is still a huge part of our cost. Creatures and
the field are all interlinked. It's more complicated than you think. I worry
you're over trusting our documentation." He was right on the substance,
and this section is what a bounded set of easy tests found. All runs on one
four-core container, `RAYON_NUM_THREADS=4`, release build of the same
commit; read the counters and the ratios, never the milliseconds against
another machine.*

### 8.1 What §2.13 got wrong, and why

It quoted the pheromone planes at "under 0.1% settled" and the field at "six
percent of a lab tick". Both are true of the state they were measured in —
a settled world, and a bed with one colony of fifty — and neither is the
state the owner plays in. `CLAUDE.md`: *measure a cost against the state
the optimisation exists for.* The creature-cost report's "86% of the frame
is the creatures at 2,709 ants" is weaker than §2.13 presented it too: the
owner's log has no per-phase split, so that share is *whatever grows with
ant count*, and the ground an ant wakes grows with ant count.

### 8.2 The mechanism of the interlink

An ant's step is a cell write. A cell write marks its chunk dirty
(`World::set` → `Chunk::mark_dirty`), and `field::step` skips its solve only
when no chunk is active — so every field tile under a walking colony is
re-solved on all five channels and re-scanned for blocking
(`rebuild_blocked`) every tick, for as long as the ants keep walking, though
a creature cell itself neither blocks a field block nor sources moisture. A
per-ant instruction profile (`creature-cost` §3, 57,314 Ir per decision)
cannot see any of that: the sweep and the field bill it as background. The
owner's own session log shows the consequence — awake chunks 12 → 52 on a
64-chunk bed as ants went 39 → 2,473 (`evolution-lab-playtest-2026-09-13.md`
§1) — and the report that refuted "it is the sweep" did so on *sites*, not
on awake chunks.

### 8.3 What was measured

**Held population, `antcost`, whole tick against standing ants.** The
stocking loop tops out at what four founding columns can seat — 125 long
ants on a 512 bed, 224 short ants on a 1024 bed — so no harness here reaches
the owner's population.

| bed | ants | awake chunks / tick | whole tick | slope |
|---|---|---|---|---|
| 512 wide, `longant` | 0 → 125 | 19.4 → 19.2 | 1.92 → 2.35 ms | +3.3 µs per ant |
| 1024 wide, `ant` | 0 → 200–214 | 21.4 → 27.9–31.1 | 2.38 → 3.25 ms | +4.3 µs per ant |

Two hundred short ants add seven to ten awake chunks; the per-ant slope
includes whatever they wake, and it is 1.6–2x the owner's own above-knee
figure of ~2.6 µs on a faster machine.

**Phase split, `lab_cost phases=1`, frames 1,000–1,500 of a bed founded at
each colony count** (ms per tick; `plants` is standing plant cells, which
the ants eat — that is the confound above one colony):

| colonies | live ants | plants | `ca_sweep` | **`field`** | `active_sites` | `pheromones` | awake / tick | field solves / tick |
|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 522 | 0.379 | **0.090** | 0.018 | 0.000 | 5.1 | 32.1 |
| 1 | 52 | 509 | 0.345 | **0.188** | 0.062 | 0.067 | 4.2 | 36.2 |
| 4 | 124 | 270 | 0.202 | 0.074 | 0.111 | 0.116 | 3.1 | 23.6 |
| 8 | 129 | 179 | 0.119 | 0.059 | 0.100 | 0.127 | 1.6 | 22.3 |
| 16 | 89 | 170 | 0.112 | 0.055 | 0.073 | 0.144 | 1.4 | 19.5 |

**The one clean comparison is the first two rows**, where the plants are
equal. Fifty-two ants **double the field** (0.090 → 0.188 ms, solves 32 →
36) while their own decisions cost 0.062 ms: **the field's response to the
ants costs more than the ants.** That is the owner's claim, measured. The
pheromone planes track ants rather than plants too, 0 → 0.144 ms — at 129
ants they cost **twice the field**, which is the part of §2.13 that was
most wrong. Above one colony the scene cannot pose the question: the
colonies eat the bed, the sweep and the field fall with the plants, and the
population caps near 125 whatever is founded.

### 8.4 What this does and does not establish

- **Established:** the field and the planes scale with ants, by the path in
  §8.2, and at low count the field's increase exceeds the creature pass.
- **Not established:** the field's *share* at the owner's population. The
  estimate from the per-awake-chunk costs above, scaled to his 52 awake
  chunks and his faster floor, puts field plus sweep somewhere around a fifth
  to a quarter of his tick. It is an estimate across two machines and not a
  number to build on.
- **Not run, deliberately:** anything deeper. The owner's instruction was
  easy tests and no performance rabbit hole.

### 8.5 What to do, in order

1. **A per-phase stopwatch in the live app behind a switch**, off by
   default and free when off. The playtest report says outright that the
   owner's log could not answer the share question and names this as the
   fix; #374 refused stopwatches in the live loop, and a gated one answers
   the objection. One session on his bed then settles it on his clock.
2. **Stop a walking colony waking the field.** A chunk dirtied only by
   creature steps has nothing for the field to re-solve. Falsifier: the
   1024-bed row above — awake chunks at two hundred ants should fall back
   toward the no-ant figure, and the field hash under a walking colony must
   not change. Cheapest lever, largest ceiling, and it is the engine's own
   ethos of stopping work early.
3. **The creature parallel switch on the owner's machine.** It ships off
   because four cores cannot pay for it (`evolution-lab-creature-parallelism`);
   more cores lower the bar.
4. **The GPU field, conditionally.** If step 1 shows the field above about
   a third of the tick at play population *after* step 2, the tile solve is
   the one pass with a compute shader's shape and the survey's
   recommendation is worth taking seriously — priced against same-build
   determinism across drivers and a readback every tick, since the ants read
   the field on the CPU. Below that share, waking less beats solving faster.

Commands, so the rows are re-derivable:

```
RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=0,150,400,800,1400 par=off rounds=10 frames=200
RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=0,300,600,1000 width=1024 colony_species=ant par=off rounds=60 frames=200
for c in 0 1 4 8 16; do RAYON_NUM_THREADS=4 ./target/release/examples/lab_cost colonies=$c frames=1500 every=500 phases=1; done
RAYON_NUM_THREADS=4 ./target/release/examples/labperf     # the attribution by phase, shipped bed
```

## 9. What this review rests on

Repo documents cited: `stigmergy-research.md`; `pheromone-master-2026-09-17.md`
and the three reports under it; `nest-design-2026-09-14.md`;
`nest-biology-2026-09-19.md`; `nest-digging-plan-2026-09-19.md`;
`nest-shape-three-negatives-2026-09-19.md`; `nest-entrance-dimensions-2026-09-19.md`;
`colony-economy-design-2026-09-09.md`; `creature-reproduction-economics.md`;
`creature-signature-and-castes-2026-09-06.md`;
`creature-stacking-design-2026-09-17.md`; `why-colonies-do-not-fight-2026-09-14.md`;
`population-dynamics-research.md`; `ecological-lod-design.md`;
`decaying-gradient-quantization-2026-09-15.md`; `larder-reachability-2026-08-30.md`;
`Reports/dead-ends.md` (entries at the lines cited); `Reports/open-bugs-handoff.md`
(§R4, §T2, §Z6, §Z7); `wiki/ants.md`; `lanes/evolution-lab-coordinator.md`
(owner rulings); `lanes/evolution-lab-pheromones.md`;
`lanes/nest-biology-research.md`; `lanes/nest-digging-handoff-2026-09-19.md`;
`Reports/instruments.md`. Unlanded: `nest-biology-digging-signals-2026-09-19.md`
and `nest-build-plan-2026-09-19.md` on `claude/nest-biology-research`
(PR #472); the trail-lifetime split on `claude/upbeat-shannon-cez0w4`.

Source cited: `src/sim/pheromone.rs`, `src/sim/brain.rs`, `src/sim/creature.rs`
(`tumble`, `home_weighted_pick`, the `Carrying`/`CarryingFood` senses, the
queen regime), `src/sim/organism.rs`, `assets/species/ant.ron`, at the lines
given.
