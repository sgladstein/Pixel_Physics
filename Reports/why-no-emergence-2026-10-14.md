# Why this engine has not produced emergent behaviour: a diagnosis (2026-10-14)

*Landed 2026-10-27 from PR #701, unchanged. Written about the hand-built ant at `bb8a2151`, before the trained-brain and colony-selection line; read its creature-layer findings as history, its method and literature checks as current.*

**Status: diagnosis; nothing built.** Read from the source at `bb8a2151` and
from the record (`Reports/`, `wiki/`, git history). **§5 is measured**: 16
runs of the shipped game, scripts in
[`why-no-emergence-2026-10-14/`](why-no-emergence-2026-10-14/). **§6 is checked
against the literature** on PubMed and SciSpace, mostly from abstracts. Other new numbers
are counts from git and grep, and each one says what it counts. Every other
figure is quoted from the report that measured it, which is named beside it.
Section 11 says what this report did not do.

**The question** (owner's brief, 2026-10-14): the goal is *"a world that feels
alive on its own with unplanned emergent behavior that I can explore and
interact with. Plants growing, ants building nests. Complex behavior from
simple rules."* Instead, foraging took a great deal of work, ants still do not
dig a realistic nest, and the owner has seen nothing unplanned from creatures
or plants. Why? Is the goal flawed, the implementation, the process, or
something else?

**The owner's correction, mid-investigation (2026-10-14), which this report
is built on:** the switch to designing rules for named behaviours was
deliberate. It came *"after much work without directly designing behavior to
continued failure. The new approach was not to hardcode behaviors, but always
keep the signals local and design simple rules with the goal of specific
behaviors."* So there are two approaches to explain, each of which failed on
its own terms, plus whatever the two have in common.

**A second correction, the same day, which reshapes §0 and §3.3:** the
engine-computed nest structures (the BFS "way in", the flood-filled home and
the rest) are *"not an explanation of why we are failing. It is evidence of
the failure. We made some (hopefully temporary) concessions to global signals
after continued failure using local signals."* That is right, and this
report now treats each concession as a marker of where a local signal ran
out. The question becomes **why local signals keep failing here**. §3.2 and
§4 answer it, and §5 measures what the colony's energy and deaths say.

**A third correction, from measurement, later the same day.** The first draft
gave the second cause as *"no energy margin"*. That was arithmetic on a
figure taken at famine, and measuring it refuted it. A forager leaves with a
median of 8–10 round trips of reserve. §0, §5, §9 and §10 are rewritten to
match.

**A fourth correction, from an adversarial review, the same evening.** The
third draft blamed the one placement rule that still kills (`NURSE_STAY`) on
a jam in the door shaft. The review showed the data fit a population boom,
and the record's own explanation (food diverted to the brood), at least as
well. Re-checked here, the shaft is fuller mainly because the colony is
bigger. That cause is withdrawn from §0. §5.3 now reports the three
candidates and the experiment that would separate them. The review's other
findings are folded in throughout. Its full list is summarised in §11.

---

## 0. The answer

**The vision is sound, and the engine already proves it can produce
emergence.** Fire, avalanches and structural collapse are emergent here, and
they feel alive. That is because they are built the way emergence needs. The
state lives in the world. A few rules apply the same way to every cell.
Nobody chose the outcome. And positive feedback is allowed to run. **The
creature layer was built the opposite way on all four counts**, by two
approaches in turn:

- **Approach 1, roughly 2026-08-11 to 09-18: no designed behaviour.** The bet
  was mechanisms plus an economy, with evolution finding the behaviour. It
  failed because the search had nothing to climb. There was one way to earn
  energy (eating), and digging, trail-laying and hauling were free, so
  selection could not see them. A generation took 8,600 to 27,000 frames. And
  the behaviour that did show up, the "giant hole", was an authored instinct
  that selection could not remove (§2).
- **Approach 2, 2026-09-19 to now: local signals, with simple rules aimed at
  named behaviours.** It produced foraging and a stable colony, but not a
  nest. It did produce unplanned outcomes, and each was treated as a defect
  (§7.1). **A rule written to produce behaviour X, kept because X appeared,
  is a controller for X.** A set of such rules grows by one rule per
  observed failure. It now has about 160 switch names, and `creature.rs` has
  gone from 2.7k lines to 55k, about 13k of them code (§3).

**Why local signals keep failing.** The global concessions (§3.3) mark the
places where a local rule ran out, and they cluster in one place: the nest.
Two causes, in order of weight, then what was measured:

1. **Locally, the world carries almost no information.** A local rule can only
   tell two places apart if the world makes them differ where an ant stands.
   This world mostly does not.
   - Soil is falling sand, so a cut collapses in five frames. Ground holds
     only where a rule stamps it as `packedsoil`, and that stamp erased the
     pellet marker the literature's construction rule reads.
   - The trail planes ignore terrain, so scent leaks through rock from one
     tunnel to the next.
   - The coarse field holds one value per 16 × 16 cells, so a whole chamber
     reads as one temperature, one humidity and one light.
   - Brood is a powder that falls to the lowest point.

   So the inside of a nest is nearly featureless at ant scale, and a rule
   that needs "the brood place" or "the way out" has nothing local to read.
   **Every global concession is the engine supplying a difference the world
   failed to carry** (§4).
2. **A rule aimed at a named behaviour comes out as "go to X", and "go to X"
   needs a map.** The ant's code has 24 functions named `*_pull` or
   `*_target` (a count of function names, including a few helpers): to the
   store, the door, the brood, the way out, the ring, the crest. A
   stigmergic rule reacts to what is under the ant ("drop food where food
   lies"). A goal-directed rule has to know where its goal is. On a
   world that carries no local differences (cause 1), knowing where means a
   global map. That is the chain that produced the concessions. It also makes
   the rule set a controller for its targets, which grows one rule per
   failure and treats anything unplanned as a defect (§3).

**What was measured (§5).**
- **The energy reserve is not the problem.** With food 90 columns away, a
  forager leaves with a median of 8–10 round trips of reserve. Starvation is
  under 1% of deaths in a settled colony, and those starvers are lost or
  held by a rule.
- **The one placement rule that still kills** (nurses staying with the
  brood) does it through a population cycle. Births double, the nest fills
  with young ants that never forage, and they starve in waves about 40,000
  frames apart. Whether a jammed door shaft, the boom itself, or food
  diverted to the brood is the cause is not separated. The experiment that
  would separate them is in §5.3.

Two things make both causes worse:

- **The benchmark is a specification past the research frontier** (§6,
  checked). The current goals are a stable population, a colony living in
  its nest, separate brood, rest and food chambers, and foraging from far
  off, all from local rules. In a limited search (§6), no published model
  couples construction to a self-feeding colony. Each builds one family of
  structures from one building mechanism, in isolation. Biology does not
  know the local cue that stacks chambers by depth: the one proposed (a CO₂ gradient) was
  vented away, and reversed, in the field, and nothing changed. An
  independent 2026 attempt at the same goal hit the same failures and made
  the same concessions. **The one functional chamber the literature does
  explain** forms around brood that workers have moved to suitable
  temperature and humidity: the chamber is dug around the brood, not in
  advance. That needs exactly the local differences cause 1 says this world
  lacks.
- **The process filters out the unplanned and makes exploration expensive**
  (§7). Unplanned outcomes become switches that suppress them, mutation is
  frozen in testing, and every experiment is 12 seeds × 300,000 frames with
  the whole economy on. So no local rule has ever been seen on its own, on a
  substrate that could carry it, without the colony's demography moving at
  the same time.

**Verdict:** the *goal* is right, and the *benchmark* chosen to prove it is
wrong. On *design*, the physics layer is right. The creature layer runs on a
medium that suits destruction and cannot carry the local differences that
construction and placement need. Its rules come out as destinations that
need a map. The *process* is well tuned for verification and
badly tuned for exploration (§9). Section 10 sets out what that implies. Every
item in it is the owner's decision.

---

## 1. What emergence needs, and where the engine already has it

The project's own doctrine is right and does not need replacing.
`emergent-world-architecture.md` §0 names the primitive,
**deposit → diffuse → decay → follow**, and the law that *"behavior count
scales with loops, not with systems written"*. §1 states the operational rule:
*"Agents interact with each other only through the world, never directly"*.
`design-philosophy.md` §0 extends that rule backwards to existing private
state.

Where the engine honours that doctrine, it works. The architecture report says
fire *"already feels alive and produces things that weren't planned"*. Powder
with two repose angles avalanches on its own. Structural failure produces a
size distribution nobody authored. All three have the same four properties:

| | Fire, powder, collapse | The ant |
|---|---|---|
| **State lives in the world** | Cells and temperature; nothing private | Private state: `forage_anchor` (an exact world coordinate), `home_patience`, `home_best`, `scout_patience`, brain state. Engine-held state, conceded where local signals failed (§3.3): `NestSite`, `NestWay` (BFS fields), `nest_dug` (flood-fill), the founding shaft |
| **Few rules, applied the same everywhere** | A handful per material kind | About 160 `PIXEL_PHYSICS_*` switch names in the ant's modules, about 116 of them listed in `how-the-ant-works.md` §12 as changing behaviour; about 13,000 lines of production code in `creature.rs` alone (§3.1) |
| **Nobody chose the outcome** | The rule says how heat moves, not where the fire goes | Each rule is written to produce a named behaviour and kept if it does (§3.2) |
| **Positive feedback allowed to run** | Fire spreads until fuel runs out | Overshoot and collapse are braked (`LAY_BRAKE`, `STORE_BRAKE`); a slower-fading food trail was rejected because it *"cost the goal box a fifth of its food"* (`how-the-ant-works.md` §12, `B_RHO`) |

The first column is a short list of conditions for emergence, and the second
shows the creature layer failing each one. The rest of this report explains
how it came to.

---

## 2. Approach 1: no designed behaviour (to roughly 2026-09-18)

**What it was:** brains whose weights are the genome. The founder's weights
were authored as sparse "instincts", the verbs were left in the economy, and
evolution was meant to find the rest. The owner's framing on 2026-08-31 set
the line: *"This sounds like we're forcing a system into creating behaviors
that we want instead of creating the most correct system and allowing
behaviors to develop"*. That became *"Add senses and economies, never
behaviours"* (`mechanism-vs-behaviour-audit-2026-08-31.md` §0).

**Why it failed: measured, in `creature-behaviour-ceiling-2026-09-05.md`.**

- **One axis to climb.** Over 24 random genomes, survival correlates **+0.895
  with how much an ant eats** and with nothing else: `travelled` +0.151,
  `commute` +0.236. Eating is the only way to earn energy, and `Dig`, `EmitA`,
  `EmitB`, `Drop` and `DropSpoil` were free. *"A behaviour that cannot be paid
  for cannot be evolved."* Prices were added the same day (dig 6 steps, emit
  ½ step), but a price only makes a verb a cost. It still earns nothing.
- **Most of the space is inert.** 58% of random genomes never moved and never
  ate.
- **Too few generations.** That report measured ~8,600 frames per generation
  on the default bed. On the bed the owner played, generation 1 arrived at
  **27,000 frames**, after half the colony had died. *"There is nothing wrong
  with the search, there is barely a search."*
- **The visible behaviour was authored.** The "giant hole" came from
  `(Bias, Dig, 0.4)`, an unconditional instinct. Digging was free, so the
  weight was invisible to selection: *"no number of generations can remove
  it."*

Two points that report does not make, both inferred and not measured:

- **The search started from a hand-set point.** Founders start from
  authored weights (`ant.ron`'s instinct rows; today's hidden units 0–6 are
  each a hand-built circuit, `how-the-ant-works.md` §4), and evolution drifts
  around them. The owner describes approach 1 as working *"without directly
  designing behavior"*. That is right about intent. Offered for the owner to
  weigh: in practice the behaviour selection could reach was bounded by those
  authored instincts, with a search too weak to move far from them.
- **A nest is a colony-level good, and selection here acts on individuals.**
  In real ants the colony is the unit that reproduces: workers are sterile
  and the nest pays off through the queen. Here every adult can breed until
  a brake holds it (`BREEDING=graded`), so selection rewards the ant that
  breeds, not the ant that digs for everyone. Even with the economy fixed,
  individual selection may reward free-riders over builders. This is untested
  here, and it is worth a measurement before any return to evolving nest
  behaviour.

**So approach 1 failed for search reasons, not because emergence is the wrong
idea.** The search had one dimension and few generations, and its starting
point was hand-designed.

---

## 3. Approach 2: local signals, with simple rules aimed at named behaviours (2026-09-19 to now)

### 3.1 What it produced, and how big it got

**Results** (`handoff/PLAN-2026-10-07.md` §1, §4.3, and
`follow-food-home-2026-10-10/README.md` §7, all measured there):

- Foraging from a heap 90 columns away gives 700–800 ants at 300,000 frames,
  with no die-offs, on 12 of 12 seeds.
- About 21% of adults are in the dug nest on steady food, against 1–6% before
  the stack.
- The nest is still **one room**, about 70 × 90 cells by 300,000 frames. In
  the owner's own reading of the pictures, it is *"one ever-bigger room"*.
- The brood room exists because of `SHAFT_ROOMS`, which its own doc calls *"a
  labelled stopgap, against the design rule that structure comes from local
  signals: no ant senses anything here, the rule only refuses cuts, and the
  brood room is a fixed box at the door"* (`creature.rs`, `SHAFT_ROOMS` doc).

**Size: counted for this report.**

| date | `PIXEL_PHYSICS_*` switch names in the ant's modules (cumulative, by first appearance in history) | `creature.rs` lines | `brain.rs` lines |
|---|---|---|---|
| 2026-08-20 | — | 2,700 | 509 |
| 2026-09-09 | 4 | 16,584 (09-10) | 2,213 |
| 2026-09-18 | 37 (about 12 probes, parallelism or breeding; the rest include nest shape and painting, room targets and spoil handling) | 29,638 (09-20) | 2,946 |
| 2026-09-30 | 82 | 40,896 (10-01) | 2,946 |
| 2026-10-14 | **158** | **54,938** | **3,291** |

*Method:* switch names are every `PIXEL_PHYSICS_*` string in `creature.rs`,
`creature/needs.rs`, `brood.rs`, `pheromone.rs`, `organism.rs` and
`brain.rs`. Each is dated by the first commit on `main` that introduced it
(`git log -S`). Line counts are taken at the last commit on `main` before each
date. The 161 strings include three plant switches in `organism.rs`
(`ANNUAL`, `GRAZE_REGROW`, `PLANT_DEFENCE`) and one fragment
(`PIXEL_PHYSICS_SPOIL_`). `creature.rs` today splits into about **13,000
lines of code, 17,500 of comments and 23,300 of tests**. 561 of the 3,407
non-merge commits on `main` touch `creature.rs`, `brood.rs`, `brain.rs`,
`pheromone.rs` or `ant.ron`.

**The brain stayed flat while the hand-written walk grew twenty-fold.** Under
the shipped `trailaway` chooser, the brain's trail inputs are *"handed … as 0
for every creature walking the chooser"* (`how-the-ant-works.md` §6d). Walking
is a weighted sum of five things: a homing pull toward the exact coordinate
`forage_anchor`, a patience that decays, trail presence as a multiplier, an
away-from-home term, and a scouting term with its own patience. The brain
mostly sets the probability of moving at all.

### 3.2 Why rules aimed at behaviours do not produce unplanned behaviour

This is the structural point under the owner's correction. Every rule in
approach 2 is local in the sense meant. Each is also:

1. **Written to produce one named behaviour.** The names say so: `HUNGRY_OUT`,
   `LAY_IN`, `MOUND_IN`, `NURSE_STAY`, `CROP_DOWN`, `STORE_READ`,
   `NEST_STORE`, `SPOIL_HAUL`, `DOOR_COLUMN`, `SHAFT_ROOMS`, `FACE_TRIP`,
   `SOIL_WAY`, `DIG_ROOF`. A rule named for an outcome is a description of
   that outcome.
2. **Accepted when its target metric moves** (`handoff/PLAN-2026-10-07.md`
   §2.1: *"merge it on if it fixes its target with no big measured downside"*).
3. **Added because the previous rules produced something unwanted.** The record
   shows the sequence. The carriers that walked off west got
   `HOME_SEARCH`. Spoil on the door got `DOOR_COLUMN`. A colony living in the
   mound got the inward cues. One big room got `SHAFT_ROOMS`.

A rule set built this way is a **controller**. Its emergent interactions are,
by construction, either harmless (and so invisible) or harmful (and so fixed
by the next rule). It does produce unplanned outcomes (§7.1 lists them; §5.3
adds a population cycle). But its acceptance step reads each one as a
defect, so none survives long enough to be watched.
The growth curve above is that loop running.

**Rules aimed at outcomes come out as destinations.** "Food stored apart"
becomes *carry food to the store*. "Living in the nest" becomes *walk in along
the way*. "Brood in one place" becomes *walk to the larvae*. The ant's code
has **24 functions named `*_pull` or `*_target`**. That is a count of
function names: it includes pace helpers and route variants, so the number
of distinct rules is lower. Among them are `home_pull`,
`hungry_out_pull`, `nest_store_pull`, `store_target`, `lay_in_target`,
`nurse_in_target`, `mound_in_target`, `spoil_haul_target`, `ring_target`,
`sky_target` and `rest_pull`. The plan's own reading of them (PLAN §6 step 3,
citing the rule audit): *"Pulls to places (door, store, anchor) work but are
fragile: each needs its own exceptions for hunger, crowding and the rest, and
most recent bugs were a pull misfiring … The walk has one pull slot, filled
first-match, so any pull can veto hunger."* A stigmergic rule is reactive: it
acts on what is under the ant, such as *pick up where food is sparse, drop
where it is dense* (the shipped `STOREROOM` `pile` part is that form). A
destination rule must know where its destination is. That is the bridge from
rule shape to the global concessions in §3.3.

**The rules also interfere more as they pile up.** Each of these is measured
in the record:

- The new ant: *"every single-fix arm also failed (5 of 16 colonies died out
  or fell under 20)"* (PLAN §7).
- Birth brakes: *"every birth brake tried made the long run worse"* (PLAN §0).
- `FRESH_CUT`: a bigger nest on 12 of 12 seeds, *"but live ants were 26
  against 37"* (`dead-ends.md`).
- `HOME_SEARCH`: shipped off *"because it shrinks the colony"*.
- `NEST_REST`: *"two colonies of four collapse late under it"*.
  (`how-the-ant-works.md` §6d, §12.)

That is what a dense, hand-coupled controller does as it grows, and it is the
opposite of "simple rules".

### 3.3 The concessions: a map of where local signals ran out

The owner's design principle (PLAN §2.4) is that *"rules should read what an
ant could sense where it stands. A global signal … is allowed only as a
labelled stopgap."* The owner's correction (top of this report) puts these
structures in their place: **they are evidence of the failure, not its
explanation.** Each was conceded after local signals failed. Read as a set,
they show *where* local signals fail, and the answer is consistent. Every
one supplies a difference between places inside or around the nest: where
home is, which way is out, where the roof is, who stays inside. `DOOR_COLUMN`
and `SHAFT_ROOMS` carry a stopgap label. The rest are computed by the engine
with knowledge no ant has:

| structure | what the engine does | read by | source |
|---|---|---|---|
| Founding shaft and chamber | **The engine cuts** a shaft 6 rows deep and 2 wide, with a lined chamber at its foot, and paints a door 5 columns wide | everything: "founding" appears 435 times in `creature.rs`, "shaft" 456 times | `how-the-ant-works.md` §8, `cut_founding_shaft` |
| Home (`nest_dug`) | A **flood-fill from the door** over every open cell under the old ground line, within 60×60, rebuilt every `ROOM_INTERVAL` | `AtNest`, and so every nest rule | §8, `World::step_nest_dug` |
| The way in (`NestWay`) | A **breadth-first search from the door**, rebuilt and stored as `dist`, `store`, `foot` and `store_foot` | `hungry_out_pull`, `home_pull`, `nest_store_pull`, `lay_in_target`, `SOIL_WAY`, `MOUND_IN` | `build_nest_way`, `creature.rs:12966` |
| Home position | `forage_anchor`, an **exact world coordinate** that every carrier homes on | `HomeAligned`, the chooser's home term | §8 |
| Nest-worker caste | **One ant in four**, founders and newborns alike, nest-bound for life (`STOREROOM`'s `caste=4`, shipped) | the store, the nurse rules | `Storeroom::SHIPPED`, `creature.rs:3984` |
| The roof | **A cut is refused** if it lies within 6 rows under the founding surface and outside the door's columns (`DIG_ROOF`, shipped) | the dig | §12 |

`hungry_out_pull`'s doc defends the BFS: *"The cue is local. The way in is the
breadth-first step count from the door … and the pull reads it only round the
ant's own head. It stands in for the nest's air."* A real diffusing air
channel would be slow, blocked by crowds, flattened at the back of a chamber
and disturbed by every new cut. Those imperfections are where the unplanned
behaviour would come from. An exact BFS is the version with all of them taken
out.

**None of these was a bad decision on its own.** Several were the only way to
keep a colony alive on this substrate. What matters is what they have in
common. Each supplies the engine's knowledge of *which place is which*,
because the world did not carry that difference where an ant stands (§4). And
each serves a destination rule (§3.2), which cannot work without it. While
they stand, the nest's skeleton and map are the engine's, so the nest cannot
be one nobody designed. But removing them without fixing §4 and §5 would only
repeat the failures that put them there.

---

## 4. Cause 1: locally, the world carries almost no information

A local rule works only if the world makes two places differ where an ant
stands. In stigmergy, the medium is the memory. A termite builds a pillar because the
pellet already there is marked. An ant digs where the face is fresh. A trail
is followed because it persists. **Construction stigmergy therefore needs
ground that holds its shape and carries marks.** This engine's ground does
neither:

- **Soil is a falling powder.** *"A shaft, a gallery and a chamber cut into a
  bed of `soil` are 63%/67%/82% open one frame later and the gallery is gone
  in five frames, because `update_powder`'s straight-down rule is
  unconditional on support"* (`assets/materials/packedsoil.ron`). Raising the
  friction angle cannot reach this: *"repose only ever makes a pile
  flatter"*.
- **So structure exists only where a rule stamps it.** `line_burrow` packs all
  eight neighbours of every dug cell into `packedsoil`. A tunnel's walls are
  where the rule put them, and the rule's stamp is the shape.
- **The stamp erased the construction cue.** Khuong et al. 2016's rule
  (deposit where pellets are marked) was *"declined before it was built"*. The
  lining had relabelled about two-thirds of the mound as gallery wall, and
  **77% of pellet drops (2,166 of 2,807) went up the column**, *"where
  `creature.rs`'s `lifted` branch abstracts the haulage trip and there is no
  neighbour to prefer at all"*. Once that was fixed by switches, the marker
  was everywhere instead, and the rule still had about one decision in a
  hundred to make (`dead-ends.md`, the Khuong entry).
- **The trail planes ignore terrain.** *"Scent spreads into rock and sky
  exactly as into open air, and leaks between parallel tunnels"*
  (`how-the-ant-works.md` §7). A scent that cannot tell one tunnel from the
  next cannot encode a nest's layout. The chemistry could not carry the map,
  so the engine computed one (the BFS "way", §3.3).
- **The coarse field is flat at ant scale.** It holds one tile per 4 × 4
  block of a 64-cell chunk, so one value per 16 × 16 cells. A 70 × 90 nest
  spans about 4 × 6 field values, and a chamber reads as one temperature, one
  humidity and one light. And a field read is block-nearest: four sensors one
  cell apart *"land in the same field block roughly seven times in eight, so
  their differences are zero"* (`.claude/rules/src-sim-cells.md`). Real nests
  have the gradients ants place brood by (depth, temperature, humidity, CO₂).
  Here none of them exists at the scale of a chamber. `MoistureGrad` is
  recorded as inert on uniform beds (`dead-ends.md`).
- **Brood is a powder that falls.** A brood cell is *"a powder that does not
  roll and drops through bodies"* (`assets/materials/brood.ron`). Without `SHAFT_ROOMS`'s floor, *"the box alone let the
  brood fall through the door shaft (seed 1: 76% of brood below the box, 115
  larvae starved)"* (`creature.rs`, `SHAFT_ROOMS` doc). In a vertical slice,
  gravity sorts brood to the lowest open cell, whatever a local rule says.
- **Local crowding was tried and moved nothing.** The dig urge reads one
  nest-wide crowding number, *"so every ant digs alike everywhere and nothing
  can branch the nest"* (PLAN §6 step 4). The local reading
  (`CROWDING_LOCAL`, 2026-09-19) left the nest unmoved too: *"the local
  reading desaturates the input and the nest still does not move"*
  (`ant-survey-round-2026-09-19.md` §0). On featureless ground, a local
  density signal has nothing to amplify.
- **The two literature digging rules that were tried both ran on this
  medium.** Toffin et al. 2009's density-dependent digging was *"a coin flip,
  with the sign reversing between colony sizes"*, and its one surviving effect
  was *"the opposite of the goal"*. Buhl et al. 2004's positive feedback at
  the face (`FRESH_CUT`) dug bigger but shrank the colony
  (`dead-ends.md`). Neither was ever run on a medium that holds a cut by its
  own physics.

**Put together:** inside the nest, an ant's eight neighbours are packed soil,
other ants or brood almost everywhere. The scent is smeared through the walls
and the field is one value. Two places that should differ (the way out and a
dead end, the brood room and the store) read the same where an ant stands.
That is why every concession in §3.3 is a *which-place-is-which* signal.

**This is mostly not about the world being 2D.** A real ant farm is 2D, and
ants dig good nests in it, because moist sand has cohesion. It holds a roof,
fails gradually and records where the last cut was. A falling-sand cellular
automaton is defined by *not* doing that. The engine's chosen substrate is
excellent for destruction and poor for construction, and nests are
construction.

**One part of it is about the side view, though.** The surface is a single
line, and the nest meets the world through it. That limits alternative
routes and makes one door the natural outcome. The record already lists the
side view's cost: *"foraging is natively top-down, so double-bridge
scenarios are harder to stage"* (`dead-ends.md`). Harder is not impossible:
alternative routes have to be built from terrain (§10, recommendation 1).

**Foraging was hard for related reasons** (inferred from the record, not
re-measured). The world is a vertical slice with gravity. Ants climb walls and
ceilings, a sensor 6 cells ahead on a diagonal lands in rock or sky (hence the
"honesty gate" and the row projection, `how-the-ant-works.md` §3), and trail scent pools in solid
ground. What is a short forage-and-follow loop in a top-down simulation became a
layered chooser, because the trail alone could not carry the ant home here
(round 36, 2026-09-15, before the trail reader was fixed: *"cutting the homing circuit out of the genome leaves deliveries
inside the noise"*).

---

## 5. Measured: the energy margin is wide, and the placement rule that still kills is unexplained

This section has been corrected twice.
- **The first draft** said the ants had no energy margin. It took the median
  ant's reserve from the boom-and-bust trace, which was measured at famine,
  and estimated the trip by arithmetic. **Measured, that claim was wrong**
  (§5.1).
- **The second draft** said the one placement rule that still kills does so
  by jamming the nest's door shaft. **An adversarial review (2026-10-14)
  showed the data do not separate that from a population boom or from food
  diverted to the brood** (§5.3).

What follows is what was measured, and what was not.

**Setup.** `deeptrace` on the shipped game, built from `bb8a2151`: the dry
goal box (`nest_goal`), the evolved founder, mutation off, seeds 1–4,
300,000 frames, `ants=60 hungry=1 mapevery=1000`. That records:
- every decision of 60 focal ants, refilled as they die;
- every decision of every hungry ant;
- every ant's head every 1,000 frames;
- every death, with its cause and place.

There were four arms:
- food 30 columns from the nest (the box's own);
- food 90 (`foodgap=90`);
- food 30 with `PIXEL_PHYSICS_NURSE_STAY=on`;
- food 30 with `PIXEL_PHYSICS_NEST_REST=workers`.

The last two are rules the record lists as failing through starvation (PLAN
§5). The run line and four readers (`margin.py`, `starvers.py`,
`stuckway.py`, `shaftcensus.py`) are in
[`why-no-emergence-2026-10-14/`](why-no-emergence-2026-10-14/). The run data
regenerates deterministically and is not in the repo. The reserve is read
over frames 100k–300k (the settled colony) and deaths over 20k–300k.

### 5.1 The reserve is several round trips deep

The cross-check on the burn rate: hungry ants starting near 200 J lasted
2,300–3,400 frames, as 0.077 J a frame predicts. Ranges are over the four
seeds:

| | food 30 | food 90 |
|---|---|---|
| burn, J a frame (empty crop, nothing digesting) | 0.077 | 0.077 |
| reserve over all focal decisions, median (frames of living) | 16,900–20,400 | 5,200–9,200 |
| round trip, median (p90), frames | 80–110 (265–420) | 885–940 (1,290–1,610) |
| reserve at departure ÷ that trip, median | 450–810 | 8.3–10.2 |
| … 10th percentile | 46–95 | 3.4–3.8 |
| trips begun with under one round trip of reserve | 0% | 0.2–2.0% |
| adults starved after frame 20,000 (of all deaths) | 0–6 (of 3,074–3,496) | 2–28 (of 3,276–3,613) |

**The trip denominator** is focal foraging trips that reached the heap and
came home with food. Excursions that came home without reaching food are not
counted: 2,703 and 4,035 of them on food-90 seeds 1 and 2 (median 105–115
frames), against 644–684 counted trips.

**The ratio survives two variants** (the review's check):
- counting energy without crop food: 10.0 and 8.2 on seeds 1 and 2;
- burn measured on moving ants only: still 0.077 J a frame.

At food 30 the heap's zone touches the mound, so those trips are short. Food
90 is the real test. There the median forager leaves with about **nine
round trips** of reserve. The earlier arithmetic was wrong twice: it put the
trip at 1,700+ frames against a measured median of about 930, and it took
the reserve from a colony at famine.

**The reserve is thin in only two places.** One is founding: 14–35 of the 52
founders starve before frame 20,000, starting on 200 J. The other is famine:
the boom-and-bust figure.

### 5.2 Who starves in a settled colony

Few ants starve, and not for want of reserve. A starver's last hunger spell
lasted 2,300–3,300 frames, two to three round trips at food 90, without
reaching the heap. (Each spell starts just under `start_energy` by
construction, because `hungry.csv` logs only ants below it.) There are two
kinds:

- **Lost.** On food-90 seed 2, 25 ants died within 3,000 frames (frames
  239,374–242,194). All were at the far west of the box (x 4–123), on the
  opposite side of the nest from the heap (x 346). Others died on the box's
  east wall or its ceiling (y 4), both of which ants can walk. They had the
  time but not a way back. That is cause 1 again, out on the open ground.
- **Held.** The rest spent most of their last spell under a pull that does
  not lead to food: `spoil haul`, `soil way out`, `laden home`. That is
  §3.2's destination rules.

### 5.3 The two placement rules

**`NEST_REST=workers` no longer kills.** Adults starved 20–24 a run against
26–36 on the baseline, and colonies ended at 584–925 ants against 558–612.
Its recorded failure (*"deaths ran through the pellet trap"*) belonged to a
`main` from before the hunger-first rules, as PLAN §5 already notes.

**`NURSE_STAY=on` still kills, in boom-and-die-off cycles, by a mechanism not
yet separated.**

*It fired.* Crop food to brood was 3.7–4.8 MJ a run against 35–40 kJ, and
larvae starved 0–20 against 39–44.

*The colony boomed:*
- births 6,262–8,972 against 3,674–4,076;
- mean colony 721–956 against 537–593 (100k–300k);
- ants below ground 194–392 against 88–102.

*Adults starved 1,403–3,563 a run against 26–36. Who starved:*
- young ants: median age at death 7,100–15,400 frames, against 34,000–36,000
  for the same arm's old-age deaths;
- ants that never foraged: only 18–28% ever had, against 72–82% of old-age
  deaths;
- only 12–17% ever reached the heap.

*When they starved:* in pulses, at the peaks of the population below ground.
- On seed 2, 375, 504 and 118 ants starved in the nest at 40–60k, 100–120k
  and 240–260k frames. The count below ground then was 358, 587 and 345,
  against 53–93 between pulses.
- Seed 4 repeats the cycle roughly every 40,000 frames.
- The baseline holds 75–103 below ground, with no pulses.

*Where they starved.* Of the 1,349–3,482 starvers a run after frame 20,000:
- 99–100% had an open path to the door when last sampled (`starvewhere.py`);
- most died in the nest: all of them on three seeds, 73% on seed 2;
- 52–120 a run spent their last spell under the `soil way out` pull;
- the rest were unpulled, or pulled toward the door by `hungry out`;
- nest starvers began their last spell about 14 rows down, rose to about 6,
  and died 8–9 down;
- only 1–3% came within 5 cells of the door.

*The door shaft* (`shaftcensus.py`: 7 columns × rows 160–175, the ground row
included, mean over 100k–300k):

| | nurse rule (3 seeds) | baseline |
|---|---|---|
| ant heads in the shaft | 129–138 | 68–71 |
| occupied cells, of 112 | 68–72 | 38–40 |
| occupied cells at the four-ant stacking cap | 4.3–4.6% | 3.4–4.1% |
| shaft heads as a share of the colony | 13.7–15.1% | 11.9–12.7% |
| shaft heads carrying food | 9–11% | 1% |
| below-ground ants that are in the shaft | 44–49% | 71–78% |

**So the shaft is fuller mainly because the colony is bigger.** Relative to
the colony it changes little. It is not packed to the stacking cap, and most
of the extra crowd is hungry ants rather than food-carrying nurses. The
nest *outside* the shaft grows most: it is where most below-ground ants now
are.

Seed 2's mean of 69 is not an unjammed control. In its three die-off windows
the shaft held 134, 150 and 106.

**Blocking was not measured.** The starvers' recorded outcomes are steps,
idle rolls, falls and `filling`. No swap, reversal or blocked outcome
appears. Either no ant was ever refused a step, or the trace cannot record
a refusal. There is no positive control either way.

**Three mechanisms fit these data, and they are not separated:**
1. **A jam.** The way out is crowded at the die-off peaks.
2. **A boom.** Births double, and the nest fills with young ants that never
   go out to forage. It overshoots, and they starve in waves.
3. **Food diverted to the brood.** This is the record's own account
   (`creature.rs`, `NurseStay`'s doc): the hand-off *"takes 36-42% of the
   foragers' loads from the ants that ate them at the door"*, and with
   `pace` *"the larvae get more of the nurses' crop food … so less is left
   to share with hungry adults"*.

**The experiment that would separate them** is 4 seeds × 300k frames:
- (a) `NURSE_STAY=on` with births held to the baseline's rate. If deaths
  persist with no boom, the boom is not the cause.
- (b) the switch's parts alone (`stay`; `relay,down`; `nurse,pace`). This
  isolates the food diversion.
- (c) `NURSE_STAY=on` with `SHAFT_ROOMS`' brood box moved off the door's
  columns. This tests the jam.

### 5.4 What this changes

- **The energy margin is not the problem.** That claim is withdrawn.
- **The economy's guards are against rule vetoes.** Most starvers in a
  settled colony are lost or held by a rule. So PLAN §2.4's hunger-first
  principles stand, as guards against rule vetoes, not against a thin
  margin.
- **The placement rule that still kills does it through a population cycle
  the rule creates.** The nest fills with young ants and empties by
  starvation, about every 40,000 frames on seed 4. Its mechanism is open
  (§5.3).
  - Whatever that mechanism is, **it is a collective dynamic nobody
    designed.** It arrived the way §7.1 says unplanned outcomes arrive: as a
    defect.
- **The jam stays a hypothesis.** It is supported by the record's other
  sightings of a crowded door: *"a direct way that is narrow and packed with
  ants"* (`moundway.py`), *"the solid column of ants in the door standing on
  brood"* (`fedboundary.py`), and layers *"blocked by a crowd at the door"*
  (PLAN §5). It is also supported by the literature on how real ants manage
  narrow passages (§6 item 6).
  - It is not established as the cause of these deaths. The earlier drafts'
    "cause 2: one narrow exit" is withdrawn from §0.

---

## 6. The target is past the research frontier (checked against PubMed and SciSpace)

The owner's goals, held since 2026-10-03 (PLAN §1):
- **a stable population**;
- **the colony lives in its nest**;
- **separate chambers**: *"brood in one place, resting workers in another,
  food stored apart from the brood"*;
- **foraging from a far food spot**;
- **"All of it emerges from the ants' own rules. No drawn rooms, no scripted
  building, no fixed castes where a rule could do it."**

Each rule must cite the biology.

**How this was checked (2026-10-14).**
- **PubMed:** a structured search for ant or termite, nest, model or
  simulation, and excavation, construction or architecture, in title or
  abstract. It returned 23 papers, all screened from their abstracts.
- **SciSpace:** semantic searches, one per claim below.
- **Full text:** Tschinkel 2013, read from PubMed Central.
- **Not read in full:** the other papers. Each item says which source it
  rests on.

An earlier pass from web search, and the Consensus connector (its allowance
was spent), are superseded by this one. References are numbered in the
Sources list at the end.

1. **No counter-example found, in a limited search.** None of the 23 PubMed
   papers and none of the SciSpace results models functionally different
   chambers (brood, food, rest) emerging from local rules in a colony that
   must also feed and breed itself. The search had limits:
   - the PubMed query required "nest", and missed at least two relevant
     models that are in PubMed [30][31];
   - PubMed does not index the artificial-life venues (ALife, ECAL, JASSS)
     where integrated colony models tend to appear, and two of this list's
     own sources come from there [1][6].

   So read this as "none found", not "none exists". The nearest are:
   - a whole-life-cycle colony that forages, recruits, stores and
     distributes energy, breeds and defends territory, with no nest
     construction at all [1];
   - a quantitative model in which colony demography sets *how much* is dug
     and the tunnels' angle [2];
   - a wall-and-entrance model that predicts the nest lags behind the
     colony's growth [3].
2. **Each published construction model builds one family of structures from
   one building mechanism,** in a controlled arena and mostly at a fixed
   population. None couples it to a colony that must feed itself:
   - pillars, walls and a royal chamber from a cement pheromone plus a queen
     template [4];
   - pillars, caps and roofs from a building pheromone whose lifetime *"is a
     highly influential parameter"*, plus the body as a ruler [5][6];
   - the round-to-branching transition, from density [7];
   - tunnel-network topology from probabilistic laws of initiation,
     propagation and termination [8];
   - termite pillars, which stay ordered only if builders retreat from
     crowding [9];
   - termite floors and ramps from fields of mud, termites and pheromone
     [10];
   - arboreal termite walls from surface curvature [11];
   - excavation rates regulated by collisions [12];
   - **pheromone whose diffusion is blocked by the built structure**, giving
     pillars, walls, royal chambers, tunnels and kept-open entrances in 3D
     [30]. That is prior art for §4's "scent leaks through rock" point and
     for §10's recommendation 2;
   - cooperative excavation that shifts from individual to collective
     digging [31].

   One termite tunnelling model does give the diggers an energy budget, but
   its output is tunnel shape, not chambers [13].
3. **In experiments, chambers form around their contents.** In leaf-cutting
   ants:
   - workers *"preferentially relocate brood to suitable locations along a
     tunnel"*;
   - when brood and fungus are *"exposed to unfavorable environmental
     conditions, either low temperatures or low humidity, both were
     relocated, … ants clearly preferred to relocate the brood first"*;
   - so *"nest chambers are not excavated in advance, but emerge through a
     self-organized process resulting from the aggregation of workers and
     their density-dependent digging behavior around the relocated brood and
     fungus"* [14];
   - in-nest stores change what is dug: larger chambers, fewer tunnels [15];
   - with no fungus, colonies dug only tunnels and no chambers [16].

   One study of a related species found no such effect [17]. A queen-scent
   template makes the royal chamber in [4].

   **The literature's route to a brood chamber therefore needs three
   things**: brood that stays where it is put, a local gradient (temperature,
   humidity) to choose where to put it, and digging that rises with crowding.
   This engine lacks the first two (§4: brood is a powder that falls, and the
   field is one value per 16 × 16 cells).
4. **Biology does not know the cue for stacking chambers by depth**
   (confirmed in full text [18]):
   - venting the soil CO₂ gradient and *reversing* it both left nest
     architecture and the vertical sorting of ants unchanged: *"Sadly, a fine
     and appealing hypothesis has gone west."*
   - *"The nature of the depth information is thus still unknown … will
     undoubtedly remain a challenge for years to come."*
   - The same paper notes that the gas-template model of Cox & Blanchard [19]
     *"applies only to ants nesting above ground between stones. In soil, any
     ant-generated CO₂ would be swamped by that generated by microbes and
     roots."*

   That is the model `hungry_out_pull` cites for the "nest air" its BFS
   stands in for (§3.3). The real version of that gradient may not be
   available to real ants in soil.
5. **Tunnels stand because of the medium.**
   - Fire ants built nests in every substrate tested *"other than completely
     dry or fully saturated"*. Tunnels were deepest at intermediate moisture,
     and maximum depth tracked cohesive strength [20].
   - Tunnel stability depends on grain size, tunnel diameter and water
     content [21].
   - Arches form in the soil around ant tunnels and shield them [22].
   - Tunnelling patterns follow grain size, moisture and packing [23].

   This is §4's falling-sand point, measured on real ants: in a dry,
   cohesionless medium they do not build at all.
6. **Narrow passages jam, and ants act to prevent it.**
   - Fire ants mitigate clogs in narrow tunnels with unequal workloads,
     direction reversals and short interactions [24][25].
   - Confined fire-ant traffic shows obstruction *"arising from the
     inability of individual ants to interpenetrate"* [26].
   - Argentine ants keep traffic flowing at up to 80% occupancy partly by
     avoiding *"entering overcrowded trails"* [27].
   - Termites retreat from overcrowded building sites, and without that
     response the modelled nest is *"disordered, unrealistic"* [9].

   *Inferred, untested:* this engine lets nestmates stack up to four deep
   (`STACK_DEPTH`), so a shaft can hold more than one ant per cell where real
   ants would be refused entry. In §5.3's census only 3–5% of occupied shaft
   cells sat at that cap in either arm, so stacking is not obviously what
   crowds the shaft.
7. **Brood and workers are stratified, and age drives the movement.** In
   *Pogonomyrmex badius*, brood and young workers are deepest, older workers
   higher, and seeds sit at a preferred absolute depth of 40–100 cm. Workers
   born deep move upward as they age, from brood care to excavation to
   foraging [28].
8. **The closest whole-colony effort hit this repo's failures.** ANThill-Sim
   [29] (2026, an open hobby model of *P. badius*, not peer-reviewed; read
   from its repository) digs, forages, stores seed and feeds brood. It
   reports caverns where stacked chambers should be, and colonies that grow,
   then starve their larvae. It made the concessions this repo made:
   - the founding nest is supplied;
   - founding spoil is *"counted onto the surface without her walking it
     up"*;
   - the share of diggers who go to the face is *"invented and fitted"*;
   - each ant is handed its own depth, because, as item 4 says, nobody knows
     the real cue.

**So the claim stands, with a sharper edge and one constructive turn.**
- **Beyond published models:** the combination the goals ask for.
- **Beyond published biology:** one piece of it, chambers stratified by depth.
- **Same concessions elsewhere:** an independent attempt made them too, which
  suggests they belong to the problem, not to this team.
- **The constructive turn:** the one functional chamber the literature *does*
  explain, a brood or fungus chamber, emerges from a reactive mechanism (item
  3) that needs exactly the local differences §4 says this world lacks.

**There is also a tension inside the brief.** *Realistic* means matching what
real ants do. *Unplanned* means matching nothing anyone wrote down. With the
goal stated as fidelity to a known biology (*"cite the biology"*), every
departure from real ants is a defect by definition. So the most likely source
of something unplanned, a colony that behaves like no real one, is filtered
out by the definition of success.

---

## 7. The process removes the unplanned

### 7.1 Unplanned outcomes come in as complaints and leave as switches

These have all happened. Several were genuine bugs, and fixing them was right.
The point is that **the pipeline has only one category for an unplanned
outcome: a defect.**

| unplanned outcome | what happened | record |
|---|---|---|
| Ants grazing the forest down (an alarm raised by grazing led to attack) | Removed by owner ruling; `PLANT_FOE=on` restores it | coordinator note, round 36; PR #440 |
| The "pile-loafer": one mutation parked laden ants at the food | `HOME_GATE`, `MUT_CLAMP` | `how-the-ant-works.md` §4, §12 |
| Boom and bust: the colony grows to 400–500 on a finite store, then every adult starves within 3,000–20,000 frames of the last bite | Birth brakes proposed and tried | `boom-and-bust-2026-10-08/README.md`, `birth-brakes-proposal-2026-10-08.md` |
| Carriers walking off west laying trail | `HOME_SEARCH` (off, because it shrank the colony) | §6d |
| Spoil over the door | `DOOR_COLUMN` (a labelled stopgap) | `creature.rs` |
| The colony living in the mound | Local inward cues, next on the plan | PLAN §6 step 3 |
| The nest as one big room | `SHAFT_ROOMS` (a labelled stopgap) | §3.1 above |

Boom and bust is worth pausing on. **Overshoot and collapse is the classic
emergent population dynamic**, and it is the clearest unplanned collective
behaviour this colony has shown. It arrived as the complaint *"we cannot
build up food because our population grows quickly as we get more food"*. The
owner's reasoning was sound: a game needs a colony that lasts, and the
owner's stated goal is *"a lab colony with a stable population … booms
followed by famine are the failure"* (PLAN §1). But it means the clearest
emergent dynamic the creature layer has produced was being engineered
against within days of being traced. §5.3 finds another: under `NURSE_STAY`
the nest itself fills and empties in cycles.

**No instrument here counts novelty.** Every census counts a planned quantity:
deliveries, ants in the nest, room cells, larvae starved. There is
`dead-ends.md` with 932 entries of what failed, and no register of what
emerged that nobody intended.

### 7.2 The one generator of the unplanned is switched off in testing

*"Mutation stays frozen in testing"* (owner, 2026-10-13; PLAN §2.8;
`how-we-test.md` §1). `PIXEL_PHYSICS_MUTATION` is off in every measuring
example. That is the right call for a paired A/B. It also means no
measurement in the current method can see behaviour that nobody wrote.

### 7.3 Each experiment is built for verification, not exploration

The standard setup is *"seeds 1-4 to iterate, 12 to settle, 300k frames
… 5-8 minutes per seed"*, with the whole economy on (energy, brood, starvation,
breeding, store and plants). Each new rule needs a trace, a written proposal,
an adversarial review by a second session, a switch, a 12-seed test, and an
update to the reference (PLAN §2.1, §2.5). That is rigorous, and right for
deciding whether to ship. **Exploration needs the opposite: hundreds of
cheap tries in a minimal scene, judged by eye.** With the economy always on, a
digging rule is scored on whether the colony survives, so every construction
experiment is confounded with demography. Live counts, door closures, food
sealed in soil and late collapses decided `FRESH_CUT`, `NEST_REST`,
`HOME_SEARCH` and the new ant. The construction question was never asked on
its own.

### 7.4 The record taxes every attempt

Counted for this report:

- 4,960 commits on `main` in 65 days.
- About 2.43 million words across the `.md` files under `Reports/`.
- `README.md` at 114,529 words. `CLAUDE.md`'s "~72k tokens" is stale by about
  2x.
- `dead-ends.md` at 932 top-level entries and about 1 MB.
- `how-the-ant-works.md`'s **"Verified against" header alone is 2,342
  words**, longer than most reports.

The method rules in `CLAUDE.md` were each paid for by a real failure, and
each is right on its own terms. Together they make every new idea pay for a
very large amount of reading and measurement before it touches the world.
That cost falls hardest on exactly the cheap, speculative tries that emergence
work is made of.

### 7.5 The engine was built breadth-first for another purpose

The tree holds 288k lines of `src` and 107k of `examples`, across three games,
destruction, explosions, weather, worldgen, a player character and the druid
game. The creature layer sits on a medium chosen for falling sand and
destruction (§4). It inherited that medium's properties, and **few loops tie
the creature layer back into the rest of the world**. The ant reads moisture
(`MoistureGrad`), and the record calls that inert on uniform beds
(`dead-ends.md`). It reads temperature only to turn. Its behaviour is driven
by its own machinery.

---

## 8. Plants, briefly

Investigated less than the ant, so held more loosely. The same shape shows:

- **The species file is the design.** Per `plant-engine-rethink-brief-2026-09-03.md`,
  *"how much specific behaviour is currently designed … the answer is 'nearly
  all of it, and it lives in `assets/species/*.ron`'"*. The genome is ten
  multipliers around authored constants, the cell types and behaviours are
  closed enums, and a lineage *"cannot find a value nobody wrote down"*.
- **The architecture's own headline example of emergence was never
  measured.** `emergent-world-architecture.md` §0 predicts *"spontaneous
  spatial organisation of vegetation, with zero agent code"* (tiger bush, from
  local shading plus wider water depletion). A grep finds the phrase nowhere
  else in `Reports/`.
- `plant.rs` is 27,678 lines.

The plant line has the economy and the graded death that the ant line lacked
early on. But it was never pointed at the one pattern-formation result the
doctrine promised for free.

---

## 9. So: goal, design, or process?

| | verdict | why |
|---|---|---|
| **The goal** (a living world, unplanned behaviour, simple rules) | **Sound** | The physics layer already delivers it. Precedent outside this repo is broad: termite chip-piling in three rules, trail networks, slime-mould networks, falling-sand chemistry |
| **The benchmark** (a realistic nest with separate chambers in a self-financing colony, all emergent) | **Wrong as a first proof** | It is a specification past the published models, and one piece of it (stratified chambers) past the published biology. Its "realistic" half defines departures as defects (§6) |
| **Engine design: physics layer** | **Right** | It follows the doctrine and it emerges (§1) |
| **Engine design: creature layer** | **Cannot support local rules for construction or placement** | Locally, the nest is featureless: soil that cannot hold or mark a cut, scent through rock, a field flat at ant scale, brood that falls (§4). The engine-held nest structure is the evidence of that, not its cause (§3.3) |
| **Engine design: the colony economy** | **Not the bottleneck** | A forager carries about nine round trips of reserve. The placement rule that still kills does it through a boom-and-die-off cycle whose mechanism (jam, boom or food diversion) is not separated (§5) |
| **Approach 1** | **Failed on search economics** | One fitness axis, unpaid verbs, slow generations, a designed starting point (§2) |
| **Approach 2** | **Succeeded as a controller, so it suppresses the unplanned** | Rules aimed at outcomes come out as destinations that need maps; one rule per failure, accepted on its target; about 160 switches; interference grows with the count (§3) |
| **Process** | **Built for verification, not exploration** | Unplanned outcomes become defects; mutation is frozen; every test runs the full economy; reading costs dominate (§7) |

---

## 10. What the evidence implies

These are the owner's calls. They are listed in the order the evidence
supports, and none of them is built here.

1. **Prove the substrate on a textbook result before building on it.** Use a
   minimal scene: a fixed number of ants, no energy, brood, breeding or
   plants, and short runs judged by eye. Reproduce three classics with
   *reactive* rules and no destinations:
   - termite chip-piling (pick up a chip when you bump one while empty, drop
     it when you bump one while carrying);
   - Khuong-style pillars from marked pellets;
   - **the double bridge, redesigned** (below).

   The Khuong rule was *"declined before it was built"* because the pellet
   marker did not survive the lining (`dead-ends.md`). That is the point of
   running it on a medium that keeps the mark.

   **The double bridge was tried, and it failed for fixable reasons.**
   - **It was confounded.** In a side view the two routes have to be built
     from terrain. `examples/ascii.rs`'s `double_bridge_scene` uses a wall
     with a 6-cell tunnel through it against a 40-cell climb over it. An
     August build "passed" (channel B 21.00 on the short route against 6.22
     on the long one). But it *"measured the terrain, not the colony"*: the
     ants' lateral steering was dead, so they used the one route a
     ground-dweller could take. There was no control to tell the two apart
     (`creature-direction.md` §13c). The assertion was demoted to a printed
     note.
   - **It was nearly empty.** After the one-colony fix of 2026-09-14, the
     scene's honest count is 13 round trips a run (round 35's record), too
     few for trails to compete.

   **A version that can answer the question:**
   - **A control:** the same scene with the food trail muted.
     `trailfollow`'s `mute` arm already silences `EmitB` across births. If
     the short route wins only with the trail on, that is path selection by
     pheromone. If it wins either way, it is the terrain.
   - **Mirrored layouts,** so the short route is on the left in half the runs
     and on the right in the other half, and a direction bias cannot pass
     for a choice.
   - **Enough traffic first:** more ants or a shorter span, until round trips
     are in the hundreds.
   - **Above all, the equal-branch version:** two identical routes, such as
     two equal tunnels. In Deneubourg's experiment a colony still settles on
     one branch, by positive feedback on small early differences, and which
     branch differs from run to run. **That is the purest emergence test
     available: a collective decision nobody designed, with no right answer
     built into the scene.** Read it as a census of which branch each seed
     settles on, against the trail-muted control. If colonies do not break
     symmetry, the deposit → diffuse → decay → follow loop is not doing the
     work, which is what §3.1 suspects of the shipped walk.

   If the engine cannot produce these, the fault is in the medium or in the
   trail loop, and no amount of rule-writing upstream will fix it. Each is
   days of work, not weeks, and it tests cause 1 directly, with the colony's
   demography taken out. **This sits against PLAN §2.3** (*"spend tokens only on the top
   priority"*, which is ants living in the nest). Whether it displaces that
   priority is the owner's call.
2. **Make places differ locally.** This is what the concessions stand in for,
   so it is what lets them go.
   - Soil that holds a cut by its own physics, with graded,
     moisture-dependent strength and slow failure, instead of a lining stamp.
   - Pellets that carry a decaying mark on the cell.
   - Scent that diffuses only through open cells.
   - A per-cell channel the nest writes: odour from brood, food and bodies,
     which are the "air" `hungry_out_pull` says its BFS stands in for.

   Then retire each concession in §3.3 one at a time against the local signal
   that replaces it, the BFS "way" first. The first target with a known
   mechanism is the brood chamber (§6 item 3). Brood that stays where it is
   put, moved by workers toward the warmest or dampest spot within reach,
   with digging that rises with crowding around it.
3. **Trace before fixing the placement-rule failure.** The `NURSE_STAY`
   die-offs have three candidate causes (§5.3). Run the separating
   experiment there, 4 seeds × 300k, before building any fix, as PLAN §2.1
   asks: births held to baseline; the switch's parts alone; the brood box
   moved off the door. `shaftcensus.py` and `starvers.py` already read the
   outcome. If it is the jam, keep gathering points out of the exit. If it
   is the boom or the diversion, the fix is in the economy, not the
   geometry.
4. **Prefer reactive rules to destinations.** Write a rule as "act on what is
   here" rather than "go to X". A rule that needs a `*_target` is a sign that
   the world does not carry the difference it depends on (item 2).
5. **Change the first benchmark.** Move from "the nest matches real ants" to
   **"something happened that nobody designed, and I can watch it."** Keep
   the realistic nest as a long-run aim rather than the gate.
6. **Give the unplanned somewhere to go.** When a playtest shows something
   unexpected, characterise it before fixing it, and keep a register of
   emergent outcomes beside `dead-ends.md`. Play with mutation on, even while
   testing with it off.
7. **Separate exploration from verification.** Explore in small, fast scenes
   with a few seeds and pictures. Keep the 12 × 300,000-frame regime for what
   is about to ship. Count the rules, and prefer deleting one to adding one.

---

## 11. What this report did not do

- **Runs: §5 only.** Sixteen 300,000-frame runs of the dry goal box, four
  arms × seeds 1–4, on one machine. Four seeds is enough to refute "the
  median reserve is about one trip": the refuting margin is ninefold on every
  seed. It is not enough to settle a mechanism.
- **Adversarial review (2026-10-14).** An independent reviewer reproduced
  the headline numbers and raised 22 findings: 2 blocking (the jam claim),
  12 major and 8 minor. All are addressed in this version.
  - **The jam claim:** withdrawn.
  - **Shaft census:** normalised, with its script added.
  - **Literature:** the search scope is stated, and [30][31] are added.
  - **Recommendations:** the double bridge is kept, but redesigned with a
    trail-muted control, mirrored layouts and an equal-branch version,
    because the earlier attempt failed for fixable reasons. PLAN §2.1 and
    §2.3 are acknowledged.
  - **Readers:** `stuckway.py` now uses an exact cause and a window;
    `starvers.py` no longer counts unscored decisions as food-seeking.
  - **Wording and counts:** fixed throughout.
  `NURSE_STAY` was run at food 30 only, and the food-90 arm without the
  placement rules. Every other behavioural number is quoted from the report
  named beside it. Switch dates, line counts, commit counts and word counts
  are counts from git and grep at `bb8a2151`; their method is stated where
  they appear.
- **The `NURSE_STAY` mechanism is unresolved** (§5.3). The jam, the boom and
  the food diversion all fit the data, and no run separates them. Blocking
  itself was never measured.
- **The approach boundary (about 2026-09-19) is inferred** from the switch
  dates and the owner's correction, and it is soft. Of the 37 switch names
  dated before 09-19, about 12 are probes, parallelism or breeding. The rest
  include engine-painted nest structure (`NEST_SHAPE`, `NEST_CORE`,
  `NEST_DRAINS`, 2026-09-12), a room-occupancy target (`LAB_ROOM_TARGET`),
  spoil handling (`SPOIL_FOOTING`, `SPOIL_LIFT`, `BURROW_LINING`,
  `DIG_SPOIL`), `TRAFFIC_DEFER`, `TROPHALLAXIS`, and flight
  (`CRUISE*`, `HOVER`). So engine-held nest structure predates the
  boundary.
- **The individual-selection point in §2 is an inference**, not measured.
- **The literature in §6 rests mostly on abstracts.** It was read through the
  PubMed and SciSpace connectors (the container's own network policy refuses
  the publishers). Read in full: Tschinkel 2013 (from PubMed Central) and the
  ANThill-Sim repository. The PubMed screen is one structured query, 23
  papers; a model described only in a book or a conference paper outside both
  indexes could be missed.
- **Consensus, a third index, was not used.** Its monthly allowance was
  spent. The brief is kept in
  [`why-no-emergence-2026-10-14/consensus-brief.md`](why-no-emergence-2026-10-14/consensus-brief.md)
  if a third check is wanted.
- **Plants (§8) were read, not traced.**
- **Not read whole:** `README.md`, `PLAN.md`, `dead-ends.md`,
  `open-bugs-handoff.md` and most dated ant reports. They were read by
  section and grep, per `CLAUDE.md`.

---

## Sources (§6)

Retrieved through PubMed and SciSpace on 2026-10-14; [18] read in full from PubMed Central.

1. Klügl, Puppe, Raub & Tautz (1998). *Simulating multiple emergent phenomena — exemplified in an ant colony.* Artificial Life VI (poster). [ACM](https://dl.acm.org/citation.cfm?id=286197)
2. Rajendran, Weinberger, Fonio & Feinerman (2026). *Colony demographics shape nest construction in ants.* eLife 13. [DOI](https://doi.org/10.7554/eLife.100706)
3. Franks & Deneubourg (1997). *Self-organizing nest construction in ants: individual worker behaviour and the nest's dynamics.* Animal Behaviour 54:779–796. [DOI](https://doi.org/10.1006/anbe.1996.0496)
4. Bonabeau, Théraulaz, Deneubourg, Franks, Rafelsberger, Joly & Blanco (1998). *A model for the emergence of pillars, walls and royal chambers in termite nests.* Phil. Trans. R. Soc. B 353:1561–1576. [Bristol record](https://research-information.bris.ac.uk/en/publications/a-model-for-the-emergence-of-pillars-walls-and-royal-chambers-in-/)
5. Khuong et al. (2016). *Stigmergic construction and topochemical information shape ant nest architecture.* PNAS 113:1303–1308. [DOI](https://doi.org/10.1073/pnas.1509829113)
6. Khuong, Theraulaz, Jost, Perna & Gautrais (2011). *A computational model of ant nest morphogenesis.* ECAL 2011. [DOI](https://doi.org/10.7551/978-0-262-29714-1-ch061)
7. Toffin et al. (2009). *Shape transition during nest digging in ants.* PNAS 106:18616–18620. [DOI](https://doi.org/10.1073/pnas.0902685106)
8. Buhl, Gautrais, Deneubourg, Kuntz & Theraulaz (2006). *The growth and form of tunnelling networks in ants.* J. Theor. Biol. 243:287–298. [DOI](https://doi.org/10.1016/j.jtbi.2006.06.018)
9. O'Toole, Robinson & Myerscough (1999). *Self-organized criticality in termite architecture: a role for crowding in ensuring ordered nest expansion.* J. Theor. Biol. 198:305–327. [DOI](https://doi.org/10.1006/jtbi.1999.0917); extended in J. Theor. Biol. 221:15–27 (2003). [DOI](https://doi.org/10.1006/jtbi.2003.3047)
10. Heyde, Guo, Jost, Theraulaz & Mahadevan (2021). *Self-organized biotectonics of termite nests.* PNAS 118:e2006985118. [DOI](https://doi.org/10.1073/pnas.2006985118)
11. Facchini, Lazarescu, Perna & Douady (2020). *A growth model driven by curvature reproduces geometric features of arboreal termite nests.* J. R. Soc. Interface 17:20200093. [DOI](https://doi.org/10.1098/rsif.2020.0093)
12. Avinery et al. (2023). *Agitated ants: regulation and self-organization of incipient nest excavation via collisional cues.* J. R. Soc. Interface 20:20220597. [DOI](https://doi.org/10.1098/rsif.2022.0597)
13. Lee & Park (2024). *Ecological drivers for the absence of task shifting in termite-tunneling activity: a simulation study.* BioSystems 247:105380. [DOI](https://doi.org/10.1016/j.biosystems.2024.105380)
14. Römer & Roces (2014). *Nest enlargement in leaf-cutting ants: relocated brood and fungus trigger the excavation of new chambers.* PLOS ONE 9:e97872. [DOI](https://doi.org/10.1371/journal.pone.0097872)
15. Römer & Roces (2015). *Available space, symbiotic fungus and colony brood influence excavation and lead to the adjustment of nest enlargement in leaf-cutting ants.* Insectes Sociaux. [DOI](https://doi.org/10.1007/s00040-015-0419-1)
16. Camargo, Lopes & Forti (2013). *O jardim de fungo atua como um molde para a construção das câmaras em formigas cortadeiras.* Ciência Rural. [DOI](https://doi.org/10.1590/S0103-84782013005000017)
17. dos Santos et al. (2017). *Effect of the presence of brood and fungus on the nest architecture and digging activity of* Acromyrmex subterraneus. Rev. Bras. Entomol. [DOI](https://doi.org/10.1016/j.rbe.2016.12.002)
18. Tschinkel (2013). *Florida harvester ant nest architecture, nest relocation and soil carbon dioxide gradients.* PLOS ONE 8:e59911. [DOI](https://doi.org/10.1371/journal.pone.0059911)
19. Cox & Blanchard (2000). *Gaseous templates in ant nests.* J. Theor. Biol. 204:223–238. [DOI](https://doi.org/10.1006/jtbi.2000.2010)
20. Monaenkova et al. (2015). *Behavioral and mechanical determinants of collective subsurface nest excavation.* J. Exp. Biol. 218:1295–1305. [DOI](https://doi.org/10.1242/jeb.113795)
21. Yuliza et al. (2018). *Stability of granular tunnel.* Granular Matter. [DOI](https://doi.org/10.1007/s10035-018-0847-5)
22. Buarque de Macedo et al. (2021). *Unearthing real-time 3D ant tunneling mechanics.* PNAS 118:e2102267118. [DOI](https://doi.org/10.1073/pnas.2102267118)
23. Espinoza & Santamarina (2010). *Ant tunneling — a granular media perspective.* Granular Matter 12:607–616. [DOI](https://doi.org/10.1007/s10035-010-0202-y)
24. Aguilar et al. (2018). *Collective clog control: optimizing traffic flow in confined biological and robophysical excavation.* Science 361:672. [DOI](https://doi.org/10.1126/science.aan3891)
25. Aina et al. (2022). *Toward task capable active matter: learning to avoid clogging in confined collectives via collisions.* Frontiers in Physics. [DOI](https://doi.org/10.3389/fphy.2022.735667)
26. Gravish et al. (2015). *Glass-like dynamics in confined and congested ant traffic.* Soft Matter. [DOI](https://doi.org/10.1039/C5SM00693G)
27. Poissonnier, Motsch, Gautrais, Buhl & Dussutour (2019). *Experimental investigation of ant traffic under crowded conditions.* eLife 8:e48945. [DOI](https://doi.org/10.7554/eLife.48945)
28. Tschinkel (1999). *Sociometry and sociogenesis of colonies of the harvester ant,* Pogonomyrmex badius. Ecological Entomology. [DOI](https://doi.org/10.1046/j.1365-2311.1999.00184.x)
29. ANThill-Sim (2026), an open hobby model of *Pogonomyrmex badius*; not peer-reviewed; read from its repository (README and `docs/SCIENCE.md`, commit of 2026-09-25). [GitHub](https://github.com/Adiezc/ANThill-Sim)
30. Ladley & Bullock (2005). *The role of logistic constraints in termite construction of chambers and tunnels.* J. Theor. Biol. 234:551–564. [DOI](https://doi.org/10.1016/j.jtbi.2004.12.012)
31. Prasath, Mandal, Giardina, Kennedy, Murthy & Mahadevan (2022). *Dynamics of cooperative excavation in ant and robot collectives.* eLife 11:e79638. [DOI](https://doi.org/10.7554/eLife.79638)
