# Behaviours worth seeing, and the beds that would select for them — design, 2026-09-06

*Owner's brief, verbatim: "We are trying to get creatures to evolve to
exhibit interesting behaviors. But not only do we need the behaviors possible
in the mechanics/game engine, but the environment needs to be set up
specifically to get those behaviors rewarded. I want you to explore different
interesting behaviors that we would like to see and then design specific
environment scenarios to make it happen. What do you think about this
approach?"*

**Status: design, with §4's first item built the same day** — the scenario
file is `src/lab/scenario.rs`, and §7 below records what building it found.
Nothing else in the catalogue is built, and no scenario has been run to its
horizon yet. Read off `main` at `a9744f9d`. Every number in it is quoted
from a report that measured it, and each one is cited; a claim with no
citation is a prediction and says so.

The two documents this sits on top of, and should be read with:

- [`selective-environments-2026-09-05.md`](selective-environments-2026-09-05.md)
  — the owner's earlier version of the same correction (*"not just population
  size and run length but actually setting up the appropriate environment to
  select for them"*), answered as a method. Its two currencies, its four
  conditions and its rule *never argue an environment, ablate it* — its
  first three sections — are what every bed below is written against.
- [`creature-programme-plan-2026-09-05.md`](creature-programme-plan-2026-09-05.md)
  — the plan that found two of three "environment" problems were the animal,
  and wrote the standing rule **audit the animal before building the
  environment**.

---

## 0. The answer, stated once

**The approach is right, it is the only legitimate way to want a behaviour
out of this system, and the repo has already half-adopted it. It needs one
correction, one addition, and one piece of engine work.**

**Why it is right.** The owner's own standing objection is that we should not
be *"forcing a system into creating behaviors that we want instead of creating
the most correct system and allowing behaviors to develop"*, and the line that
answered it — *the mechanism is code, the policy is genome* — forbids writing
behaviours into the animal. It does not forbid writing them into the *world*.
An environment that pays for a behaviour is how every real evolution
experiment is run: nobody wrote "digest citrate" into Lenski's *E. coli*, they
put citrate in the flask. A bed that makes trails pay is not a hardcoded ant.
So the approach and the objection are the same principle, and the approach is
the half of it that had no method until last week.

**The correction: a behaviour is not the first thing to design, it is the
third.** The programme plan measured this twice in one day. "The bed does not
punish an ant that ignores trails" was true and irrelevant, because the trail
pathway is saturated off except in a narrow carry band; "shelter needs a
hazard outside" was true and irrelevant, because the ant is blind and cannot
tell a tunnel from a pit. **A bed cannot select for what the animal cannot
perceive, and it cannot select for what the genome cannot express on a
single-weight gradient.** So the order for every scenario below is:

1. **the behaviour** — what a player would see and want to keep;
2. **the audit** — which sense fires on the trigger, which verb or trait
   carries the response, and which of the two currencies the outcome moves
   (an animal dies of energy at zero or of its cells being destroyed, and of
   nothing else — §1 of the selective-environments report);
3. **the bed** — the world that makes that currency flow through that
   pathway, built from knobs the box already has;
4. **the ablation** — `creature_arena -- arm=ablate input=<sense>` on that bed
   against the intact animal. If the bed does not punish the loss, it cannot
   select for the behaviour at any population size over any number of
   generations, and the scenario goes back to step 2.

Three of the beds below fail step 2 today, and the report says so for each
rather than designing round it. That is the point of the audit: it turns "the
ants never evolved X" from a mystery into a named missing sense.

**The addition: a scenario changes the world, never the score.** The design
guide's §5 names the trap — *"reward tunnelling, reward farming"* makes
evolution a lookup table for a checklist. The test for every bed here is
whether energy or destruction moves through the world's own physics (food is
far, a roof stops a beetle) or through a rule that names the behaviour (a
bonus for digging). Only the first kind is on the list. And it cuts the other
way, usefully: **pairs of scenarios are Gate 5's score.** The design guide
scores *specialisation* as reciprocal-transplant asymmetry — how much worse
each lineage does in the other's conditions — and a pair of beds that select
in opposite directions, with the specimen shelf to move a jar between them,
is that measurement with no new instrument. Scenarios are not a detour from
the score; they are how it gets its content.

**The engine work: a scenario has nowhere to live.** Today a bed is
`LabBox` — fourteen numbers and a wall list, saved and swept — plus whatever
the player paints by hand, which is *not* saved and cannot be put in a rack.
Two of the five beds recommended first cannot be expressed as a `LabBox` at
all (a terrain shape, a food heap on a schedule), so they can be played once
and never replicated. §4 sizes what is needed: a scenario file the box loads
and `labbatch` replicates, with placements and a timeline beside the spec.
Everything else the approach needs already exists.

**And a reframing worth taking.** The owner's stated game is *"if I have
access to food, water, can cull, can create plants, and creatures, I can
figure it out."* A scenario is a saved starting box with a question written
on it — a bed the player opens, the rack runs a hundred copies of, and the
shelf carries jars between. That makes scenarios the lab's *levels*, which
fits *stop balancing, start exposing* exactly: none of them tunes the default,
each is a preset the player picks up.

---

## 1. What a bed can be made of today

The inventory the audit is run against. All of it is on `main`.

**The animal can sense 22 things** (`brain::INPUTS`): a constant bias; each
of two anonymous pheromone planes ahead, to the side, and along the heading;
moisture ahead, to the side, and as a gradient; light here; temperature above
ambient; food adjacent; at nest; own energy; carrying; crowding; prey near
and prey bearing (both **gated on a sight range the shipped ant does not
author**, so the ant is blind and the beetle sees 64 cells); kin near and kin
bearing; surface curvature. Eight hidden units with self-recurrence give it a
memory. What a pheromone plane *means* is decided by which weights emit onto
it, not by the engine — the shipped ant lays A everywhere and B only when
laden, so A is home scent and B is the food route, and the ancestor could wire
them the other way round.

**It can do 12 things** (`brain::OUTPUTS`): turn, move, emit A, emit B, dig,
drop, persist, tumble, caution, feed, jump, drop spoil. Attack is the feed
verb on flesh; there is no strike. Nothing shipped jumps.

**Nine numbers are heritable** (`organism::CREATURE_TRAITS`): gut bias, birth
grant, reproduce-at, sight range, pace, curvature radius, dig force, digest
rate, crop capacity. All are priced (`creature-locked-fields-2026-09-05.md`),
which is what makes them safe to select on: none can ratchet.

**There are two currencies and one income channel.** Energy comes in through
`diet_yield` on an adjacent cell and nothing else; it goes out through idle
per cell, synapse, sight, curvature, force and exposure taxes, movement, and
since 2026-09-05 through digging, emitting and hauling spoil. Destruction is a
bite, fire, a blast or the cull. Heat, light and water are brain inputs and
**do nothing to an animal** — an ant on water stands on it for ever
(`creature.rs`'s `colony_ant_site` doc), and heat is walked away from because
the ant was authored to. A bed that varies only those varies nothing a
creature can be selected on.

**Food comes in two classes at ±1.0 and nothing between.** Flesh at +1.0:
ant and corpse 480, beetle 200, worm 480. Plant at −1.0: leaf, litter, moss,
seed, deadleaf 480; fruit and windfall 960; flower 1,440. Diet quality is
`(1 − |gut − class| / 2)²`, so a generalist at 0.0 gets a quarter of what a
specialist gets from either — a 4x gradient on one heritable scalar, which
matters for scenario 1.

**The box's own knobs** (`lab::scene::LabBox`): width, height, soil depth,
ground row, compartments, plant founders and their species, colonies, ants
per colony, colony species, predators, lamp spacing, seed, extra walls. Hand
tools on the bar: plant a seed, release a colony, cull, paint soil, paint
water, drop a wall, put food on the ground (`windfall`, which falls, piles and
rots), and place a jar. The parameters page carries the verb prices
(`dig_cost_in_moves` 6.0, `emit_cost_in_moves` 0.5, `spoil_weight_cells` 1.0),
the exposure tax (ships at 0.0), the mutation rate and species drift, the
damage switches and the heredity dials.

**Four animals**: `ant` (nest, no eye), `ancestor` (no nest, kin odometer, an
eye, otherwise the ant), `beetle` (carnivore, sight 64, breeds at 2,550,
can now buy its way into the ground), `worm`.

**Instruments that already answer scenario questions**: `creature_arena`
(two genomes race in one `LabBox`; `arm=ablate input= output=`, and the
economy varied in the same run); `labbatch` (a rack of one bed, replicates
seeded `seed0 + j`, one `f32` field swept); `creature_space` (behaviour-space
coverage over travelled / commute / feeding / depth); `predation_probe`
(`mode=range` for shelter, `mode=preflight` for a stigmergy census);
`burrow_probe` (roofed-void census); `labshot`, `labstats`, the roster with
five inherited numbers beside their ancestral values, the graveyard with a
`DeathCause`, and since PR #252 the lineage overlay.

**Two facts that bound every readout.** An ant's founding grant is 12,000
frames of doing nothing, so nothing read before 24,000 frames means anything
(a random genome *beats* the authored ant at 9,000). And the world seed alone
spans 2.42x–3.12x across the lab census with no effect present, so a result is
a direction statistic over 8–12 paired seeds, never a mean.

---

## 2. The card every scenario carries

Each bed below is written on the same rows, so that a missing row is visible:

| row | what it holds |
|---|---|
| **you would see** | the behaviour in the world's words, at play zoom |
| **audit** | sense · verb or trait · currency — and whether each exists today |
| **bed** | the box, in `LabBox` fields plus hand placements |
| **control** | the paired bed that differs only in the pressure |
| **ablation** | the one command that says the bed has teeth |
| **read** | the counter beside the picture, and at what horizon |
| **blocked on** | nothing, or the named thing |

The catalogue is ordered by readiness: tier 1 runs today, tier 2 needs one
small animal-side change, tier 3 is blocked and is listed so the block is
visible rather than rediscovered.

---

## 3. The catalogue

### Tier 1 — runs today, on genes that already vary

#### S1 · Two larders — diet specialisation

**You would see:** two compartments founded from the same jar; over a session
the left colony turns into grazers that strip plants, the right into scavengers
that clear corpses, and a jar carried across the wall fails in the other
room. The roster's gut column drifts apart between the two.

**Audit:** trait `gut_bias`, heritable and priced · verb `Feed` · currency
energy. Passes all four conditions: a scalar is a single-weight gradient, one
trait is trivially separable, the 4x quality curve pays, and neither room
starves at generation 0 (an ant at gut 0.0 clears the eat threshold on both
classes: yield 50 on flesh against a threshold of 12, per the programme plan
§2a).

**Bed:** `compartments: 2`, `colonies: 2`, same `colony_species`. Left room:
plant founders under the fixtures plus a windfall heap. Right room: no plants
(cull the founders spread into it) and a flesh source — the cheapest is a
worm release; whether the bar can place a worm today is the first thing to
check, since round five shipped three animals of which one could be placed.

**Control:** the same bed with the heredity mutation dial at zero — the
clonal null distribution the trait means are read against.

**Ablation:** none needed; a trait is not a pathway. The paired control is the
test.

**Read:** mean `gut_bias` per compartment against generation, 8 seeds, 60,000
frames (generation reaches ~7 per 60,000 in a stable bed). Then the transplant:
`KEEP` the best of each room, `FREE` it in the other, and read the lineage
overlay. That asymmetry **is** the design guide's specialisation score.

**Blocked on:** nothing. This is the cleanest scenario in the catalogue and
the one to run first. Intermediate food classes (§4) would turn the two
points into a range and are not required for it to work.

#### S2 · Feast and famine — life-history strategy

**You would see:** one bed where the population climbs and crashes with each
food delivery and one where it holds flat; on the ants page, the two beds'
inherited numbers walk away from each other — how big a store an ant holds,
when it thinks it is rich enough to breed, how much it gives its young, how
fast it lives.

**Audit:** traits `crop_capacity`, `reproduce_at`, `birth_grant`,
`digest_rate`, `pace` — all five heritable and priced · verb `Feed` ·
currency energy. Separable, incremental, paid. The prediction is only that
the two beds move them *differently*; which direction each goes is the
experiment.

**Bed:** two boxes, same total food over the run. *Steady*: plant founders
under the fixtures, nothing else — the crop is the trickle. *Pulsed*: no
plants; a windfall heap of fixed size beside the colony every N frames and
nothing between. N is the knob, and the interesting band is around the
founding grant (12,000 frames) where a lineage that cannot hold enough to
bridge the gap dies in it.

**Control:** the two beds with mutation at zero, which separates *selection
moved the trait* from *the trait moved because survivors happened to carry
it*.

**Ablation:** not applicable to a trait; the paired beds are the test.

**Read:** the five trait means per bed at 60,000 frames, 8 paired seeds, and
the population strip, which is the picture that makes this legible.

**Blocked on:** the pulse. Nothing in the engine delivers food on a schedule —
the player does it by hand, which is fine for play and impossible for a rack.
This is the first bed that needs the scenario file's timeline (§4).

#### S3 · The far larder — trails and routes

**You would see:** a colony at one wall and one heap at the other, and on the
pheromone overlay a route forming between them that the scattered-food control
never draws.

**Audit:** senses `PheroBAlong`, `PheroBFront`, `PheroBLateral`, `Carrying` ·
verbs `EmitB`, `Turn`, `Move` · currency energy, through trip time. Separable
— the selective-environments report's §4 measured it (3 seeds of 4, median
37.8% for the trail-less arm in the ordinary bed). **The caveat is the
animal's**: the trail pathway is worth 18% of the move decision at ~60% laden
and under 1% empty or full. A far larder sends ants out empty and home full,
which is exactly where the pathway is worth least. So this bed may come back
null for a reason that is in the ant, and that would be a finding, not a
failed scenario.

**Bed:** `founders: 0` so nothing grows near the nest; `colonies: 1` at one
wall; one windfall heap at the far wall, 400+ columns off on the 512 bed or
the full width of the 1,024 one. Deep soil is irrelevant here; keep it
shallow so digging is not a confound.

**Control:** the same mass of windfall scattered evenly along the ground.

**Ablation:** `creature_arena -- arm=ablate input=PheroBAlong seeds=8
frames=24000` on both beds. Teeth means the far bed punishes the loss more
than the scattered one.

**Read:** pickups and deliveries per lineage, channel-B mass from
`predation_probe mode=preflight` (the stigmergy census), lineage share.

**Blocked on:** nothing for a single play; the rack needs the scenario file
to place the heap.

#### S4 · The bank — shelter, from the environment side

**You would see:** a colony founded at the foot of a soil bank, digging *into*
it rather than down, and a gallery with ants inside it while beetles walk the
top.

**Audit:** this is the scenario the 2026-09-05 work says is blocked on the
animal, and half of that block is environmental. The finding: from a flat
surface `(Bias, Dig)` digs along the heading, which is a **pit, open to the
sky**, and a pit is not shelter — ants were in the open on 66.2% of ticks
with exposure at 1x the cost of living and still the non-digger won. A pit is
what "dig along the heading" produces *from a flat floor*. From the foot of a
vertical face the same verb produces a horizontal gallery on the first bite.
So the bed's job is to make the ant's existing verb produce the right shape.
Sense: what the ant reads indoors. `SurfaceCurvature` is live (radius 2, wired
to the drops) and `LightHere` falls as `0.2^(depth/8)` through opaque columns
(the coordinator note's shell finding), so under a few rows of soil it reads
near zero — a coarse roof sense that already exists and that nobody has
checked as one. **Run `creature_probe` on an ant in a gallery before believing
any null here.** Currency: energy through `exposure_cost_per_cell`, and
destruction through beetles, which cannot enter a one-wide gallery (the
survival advantage of a cell no beetle fits into measured 2.8x with no
predator present, `predation_probe mode=range`, twelve seeds).

**Bed:** a stepped floor — the left third at `ground_y`, the right two-thirds
raised 12–16 rows as soil, the colony founded against the face. Exposure on at
the 1x setting the null was measured at; `predators: 4`, breeding on;
`compartments: 1`.

**Control:** the flat bed at the same settings, whose null is on record (the
non-digger wins 4 of 4).

**Ablation:** `creature_arena -- arm=ablate input=Bias output=Dig
exposure=0.05` on the bank. Teeth means the direction flips against the flat
control.

**Read:** roofed void from `burrow_probe`, exposed ticks as a fraction of
creature ticks, deaths by `Killed` per arm. Plenty of roof plus ants outside
means the block is perception, not shape — the programme plan's own
separator.

**Blocked on:** a terrain shape, which `LabBox` cannot express; the player can
paint it. If the null survives the bank, the block is the roof sense and it
moves to tier 2.

#### S5 · Gause's jar — predator–prey persistence

**You would see:** in a single open box, beetles eat the colony down and then
starve out, or the colony outgrows them and they fade to one; in a box cut
into four, both sides persist longer, because a room the beetles have not
found is a refuge. Two population lines on the strip, and the graveyard's
`Killed` count.

**Audit:** beetle side — sense `PreyBearing` (sight 64), verb `Feed`, breeds
at 2,550 (landed 2026-09-05; the verb fires, 6 → 7). Ant side — nothing: the
ant cannot see a beetle. So this bed selects **beetles** for hunting and ants
only for whatever incidentally keeps them alive, which is fine: the question
here is the ecology, not the genome. The two seeds run so far disagreed in
direction (ants 15 against 53 on one seed, 34 against 21 on the other), so
the sweep is owed regardless.

**Bed:** `predators: 4`, breeding on, `compartments` swept over 1, 4, 16 —
`labbatch` can sweep it today, since `params::write_bed` names it as a
sweepable field.

**Control:** the exogenous beetle, which `labstats beetlebreed=0` already
switches on.

**Ablation:** not a genome question; the compartment sweep is the experiment.

**Read:** time to the first extinction of either species, order statistic
over 12 seeds at 60,000 frames; the population strip is the picture.

**Blocked on:** nothing. This is the one scenario that is a pure `labbatch`
today, and the textbook prediction (spatial structure extends coexistence —
Huffaker's oranges) gives it a known answer to check the instrument against.

### Tier 2 — one small change in the animal first

#### S6 · The hunting ground — flight from a predator

**You would see:** ants scattering when a beetle walks in. At play zoom an ant
is picked out of the soil by *moving*, so a colony that breaks and runs is the
most legible creature behaviour this engine can produce.

**Audit:** sense `PreyNear` / `PreyBearing` — **exists, priced, and switched
off on the ant**, which authors no `sight_range`. The programme plan §2a has
the whole case: a beetle is already visible prey to an ant with an eye (yield
50 against 12), `PreyBearing`'s sign was left free so one weight can mean
approach or retreat, and the price is already on the ant (`sight_fraction`,
which it has been paying for a sense it never used). Verb `Turn`, `Move`,
possibly `Impulse`. Currency destruction.

**Bed:** open and flat, `predators: 6–10` breeding, `compartments: 2` so the
prey cannot be run to extinction before anything is read, shallow soil so
shelter is not the confound.

**Control:** the same bed with the sighted ant's `PreyBearing` weights zeroed.

**Ablation:** `creature_arena -- arm=ablate input=PreyBearing` on the sighted
ant. Teeth means the blind arm dies more.

**Read:** `Killed` per lineage, lineage share, 12 seeds, 24,000+ frames.

**Blocked on:** one asset line — `sight_range` on `ant.ron` — and the
`priced_but_blind` control in `creature.rs` that will go red the moment it
lands (named in the plan). A cast is 328–1,186 `World::get` per ant per tick;
the frame cost is stated there and should be re-measured on the played bed
(`founders=128 colonies=1`).

**And a second scenario for free:** the same weight with the opposite sign is
**the pack** — many ants against few beetles, whose flesh is worth 200 a cell.
Whether a colony hunts or runs is then the bed's choice, not the author's,
which is the whole idea.

#### S7 · No home — aggregation without a nest

**You would see:** `ancestor` founders scattered thinly across a wide bed,
with no nest material anywhere, drawing together over a session — or not,
which the owner has accepted in advance as a real finding.

**Audit:** senses `KinNear`, `KinBearing` (live on the ancestor, whose odometer
counts from kin instead of a material) · verb `Turn` · currency energy. **The
tension to state up front:** a birth needs a free cell beside the parent, and
the stable bed denies 1,171 births for want of one against 157 born
(`creature-behaviour-ceiling-2026-09-05.md`). Clumping *costs* births, so
aggregation has to pay for itself through finding food together, and this bed
must make food findable by kin and not by a lone walker.

**Bed:** wide (the 1,024 bed), `colony_species: ancestor`, eight colonies of
six rather than one of fifty-two so the founders start spread; one windfall
heap; `founders: 0`.

**Control:** the same bed with the ancestor's `(KinBearing, Turn, 1.2)`
zeroed.

**Ablation:** `creature_arena -- arm=ablate input=KinBearing output=Turn` on
the ancestor.

**Read:** mean kin-near per lineage, births denied against births, lineage
share; the lineage overlay is the picture.

**Blocked on:** the multi-colony spread is expressible today; the single heap
needs the scenario file for a rack.

#### S8 · Lamp and shade — light seeking

**You would see:** half a bed lit and half dark; food grows only in the light
(a herb in a fixture gap died without a seed on 4 of 4 seeds where 4 of 4 bred
under a fixture, `plant-reseeding-2026-09-03.md`), and the colony learns to
stay where the light is.

**Audit:** sense `LightHere`, live · verb `Turn` · currency energy, but
**indirectly**: light does nothing to the animal, food is merely where light
is. That is a weak, second-hand gradient and the honest expectation is a
small effect.

**Bed:** fixtures over one half only. `lamp_spacing` tiles the whole ceiling
today, so this needs a lamp mask in the scenario file or a hand-placed wall
under the dark half's fixtures.

**Ablation:** `creature_arena -- arm=ablate input=LightHere`.

**Blocked on:** the lamp mask; low priority.

### Tier 3 — blocked, and listed so the block is visible

#### S9 · The crossing — water

A moat between the colony and the larder would select for whatever gets an
animal across water. **Nothing can, and nothing needs to**: water is not a
currency. An ant that walks onto a pond stands on it for ever (`creature.rs`,
`colony_ant_site`), and what it *should* do — drown, float, swim — is an open
design decision recorded as the owner's. Until water does something to an
animal, no water bed selects for anything. The decision is the block, not the
engine.

#### S10 · The gap — jumping

`Impulse` is a priced verb nothing uses; the wiki says which creatures jump
*"is meant to be settled by which ones do better for it."* Ants walk up walls
and along ceilings and refuse open air, so the only obstacle that stops a
walker is a **gap**: a trench of air between colony and food. **Measure first
whether a walker crosses a two-cell air gap diagonally** — if it does, widen
until it does not, and that width is the bed. Cheap to run once the scenario
file can cut a trench; a hand-painted one works for a single play.

#### S11 · Two tribes — interspecific competition

Two species in one room with one larder — `ant` against `ancestor`, or two
jarred lineages promoted to species. Ants can bite non-kin flesh (`eats_kin`
is species identity, and a foreign ant's cell clears the yield threshold), so
this is competition and predation at once, and the lineage overlay was built
for exactly this picture. **Blocked on S6's eye** — without sight neither side
can find the other except by walking into it.

#### S12 · Division of labour

The behaviour everyone wants first and the one furthest away. Needs two tasks
with different optimal traits *and* a way for one ant's income to reach
another's bank. **There is no food sharing between adults**, so an ant that
specialises away from feeding starves (selective-environments §5). No bed can
be built for it; the programme plan lists it as not-until-Phases-1-to-3-pay.

#### S13 · The lawn — grazing a regrowing crop

Moss regrows under a grazer; a bed of moss and nothing else asks whether a
slow, sessile lineage beats a ranging one. This is S2's steady arm with the
trickle made spatial, and it folds into S2 unless someone wants the picture —
bare patches where colonies sit — which is legible on its own.

---

## 4. What the catalogue exposes, in the order it blocks things

1. **A scenario file.** `LabBox` persists (`scene::LabBox::save` /
   `load_saved`, gitignored `assets/lab_bed.ron`), the dials persist
   (`params::Dials`), and **nothing the player paints does** — a soil bank, a
   food heap, a trench, a lamp mask, a species release. Five of the eight
   designed beds need one of those, and a bed that cannot be saved cannot be
   put in `labbatch`, so it can be played once and replicated never. The
   shape is `LabBox` plus a list of placements (rectangles of soil, water or
   wall; heaps of a material; releases of a species from a jar or a name)
   plus a timeline (the same placements at frames), read by the same builder
   `bin/lab.rs` already runs twice at startup. It costs the frame nothing:
   everything in it happens at build or at a scheduled frame. It is a data
   format and a builder, and it is the one piece of engine work the approach
   actually needs. **Built — §7.**
2. **The ant's eye** — `sight_range` on `ant.ron`, one asset line, priced
   already, with the plan's own guard to watch go red. Unblocks S6 and S11
   and is the cheapest route to the most visible behaviour on the list.
3. **A per-animal behaviour descriptor** (programme plan §3, 1b).
   `creature_space`'s descriptors are population statistics, so a scenario's
   readout today is per bed, not per lineage. Every S-card's *read* row gets
   sharper with this and none is blocked on it.
4. **Water as a currency** — the owner's decision, recorded as open. S9.
5. **Intermediate food classes** — turns S1's two points into a range. Not
   required for S1 to work.
6. **Food sharing between adults** — S12. Large, and not next.

---

## 5. Traps, mapped onto this work

All from `CLAUDE.md` or the two parent reports; listed because each has
already fired at least once on the creature line.

- **The horizon.** Nothing under 24,000 frames means anything; a random
  genome *wins* at 9,000 because starving is not yet possible inside the
  grant. Every S-card's read row says 24,000 or 60,000 for this reason.
- **One seed.** The seed alone spans 2.42x–3.12x. Eight to twelve paired
  seeds, read as how many moved the same way.
- **A dominated bed.** If everything starves nothing else is selected
  (selective-environments §2d). Run `arm=same mirror=off` on a new bed
  first and confirm the colony persists to the horizon before ablating
  anything in it.
- **Space-limited demography.** 88% of births in the stable bed fail on
  adjacency. A bed that packs the colony tighter selects for nothing but
  room, whatever else it was designed for. Watch births denied beside births.
- **The scoring trap.** If a bed's pressure is a rule that names the
  behaviour, it is not a scenario. §0.
- **The stale binary.** Every readout here comes out of an example.
  `cargo build --release --examples` with `set -o pipefail` before any run,
  and treat identical output across a change that must have moved something
  as the tell.
- **The audit first.** Two of the three named blocks on 2026-09-05 were the
  animal. Before building any bed, name the sense that fires on its trigger.

---

## 6. Recommended order

1. **S1, Two larders.** No engine work, one heritable scalar with a 4x
   gradient, and it produces Gate 5's specialisation score as a side effect.
   If this does not separate, the bed is not selecting and nothing below it
   will.
2. **S5, Gause's jar.** A pure `labbatch` today, with a textbook answer to
   check the instrument against, and it closes the beetle seed sweep the plan
   already owes.
3. **S3, The far larder.** Runs by hand today; the ablation is one command;
   and a null here is a finding about the carry-band gate rather than a
   failure.
4. **S4, The bank.** The environment-side half of the shelter question, which
   the 2026-09-05 work concluded was all animal. One hand-painted step tests
   that conclusion.
5. **S2, Feast and famine** — once the scenario file can pulse food; by hand
   before that.
6. **S6, The hunting ground** — after the one asset line.

The scenario file (§4 item 1) is worth starting alongside S1, because S2, S3,
S4 and S7 all wait on it to leave the single-play stage.

---

## 7. Built the same day: the scenario file, and what building it found

`src/lab/scenario.rs`, on this report's branch. README's *"Lab scenarios
status"* is the shipped behaviour; this section keeps only what a later
session cannot reconstruct from the code.

**What a scenario is, as shipped.** A `LabBox` plus three lists: settings
(parameters-page knobs by the page's own labels, so a scenario can switch
the ant's eye on without touching `ant.ron`), placements applied once after
the build (a filled rectangle, a heap or a scatter on the surface, a plant,
a colony or one animal at a column, and the bed's own colony and predator
spread so a `compartments` sweep still lands one per room), and a timeline
of the same placements at frames. Loaded from the BOX page, from
`scenario=` at startup, and from `labbatch`/`labshot`. Eight files under
`assets/lab_scenarios/`, one per S1–S6 plus the two paired controls. The
per-tick cost of the timeline check is unmeasurable by a paired control
over one cloned world.

**Three things the pictures found that the counters could not**, each now
written into the scenario file it applies to:

- **Plant founders were eaten before they established.** A colony placed on
  frame 0 lands beside seeds, and `gauses_jar` read *plants 0* from frame 100
  on. So every scenario that has both founds its colonies from the timeline
  after a 3,000-frame head start, and a guard asserts no creature exists
  before that arrival and a plant is still standing when it lands. Measured
  after: plants 8 → 48 → 98 → 210 at frames 0 / 3,000 / 6,000 / 12,000 in
  Gause's jar. **Any future scenario with plants and animals wants the same
  shape**, and it is the first thing to suspect when a scenario's plant
  count reads zero.
- **Soil is a powder, so an authored cliff is a ramp.** `the_bank`'s
  vertical face slumps into a short slope inside the first 3,000 frames and
  then holds; the bank top stays at full height. S4 is therefore "dig into a
  slope", not "dig into a wall", until a scenario can lay a stone core under
  a soil skin. Recorded rather than fought.
- **A larder has to be sized from the colony's burn, not chosen.** Fifty-two
  ants idle at 31,200 energy per 6,000 frames, and this report's first heap
  sizes fed a fifth of that: both rooms of `two_larders` had eaten their
  starting heaps by frame 600. The shipped heaps cover about 92% of idle burn
  per window (30 windfall / 60 corpse) so births, not starvation, decide the
  count. The arithmetic is in each file's header so the owner can move it.

And one placement fact worth carrying: a 52-ant colony's band is ~204 cells
wide and centred on its column, so the smallest column at which all 52 place
on the shipped bed is **x = 107** (35 of 52 at x = 40; 64, 80, 96, 102 and
106 all lose at least one).
