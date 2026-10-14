# Why this engine has not produced emergent behaviour: a diagnosis (2026-10-14)

**Status: diagnosis; nothing built.** Read from the source at `bb8a2151` and
from the record (`Reports/`, `wiki/`, git history). **§5 is measured**: 16
runs of the shipped game, scripts in
[`why-no-emergence-2026-10-14/`](why-no-emergence-2026-10-14/). **§6 is checked
against the literature**, from search results and abstracts. Other new numbers
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
out. The question becomes **why local signals keep failing here**, and §3.2,
§4 and §5 answer it.

**A third correction, from measurement, later the same day.** The first draft
gave the second cause as *"no energy margin"*. That was arithmetic on a
figure taken at famine, and measuring it refuted it. A forager leaves with a
median of 8–10 round trips of reserve. The placement rule that still kills
does so by jamming the nest's one narrow exit (§5). §0, §5, §9 and §10 are
rewritten to match.

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
  nest, and nothing unplanned. **A rule written to produce behaviour X, kept
  because X appeared, is a controller for X.** A set of such rules grows by
  one rule per observed failure. It now has about 160 switches, and
  `creature.rs` has gone from 2.7k lines to 55k (§3).

**Why local signals keep failing.** The global concessions (§3.3) mark the
places where a local rule ran out, and they cluster in one place: the nest.
Three causes, in order of weight:

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
2. **The nest has one narrow way out, and a rule that gathers ants by the
   brood jams it** (measured, §5).
   - **The energy reserve is not the problem.** With food 90 columns away,
     a forager leaves with a median of 8–10 round trips of reserve, and
     starvation is under 1% of deaths in a settled colony.
   - **The jam is.** Turn on the rule that keeps nurses with the brood, and
     the door shaft holds nearly twice its usual crowd, because the brood
     lies in the shaft under the door. 1,349–3,482 adults per run then
     starve, mostly inside an open nest. They are pulled toward the door
     and keep moving, and 97–99% never get within 5 cells of it.
   - **Real ants keep narrow tunnels flowing by staying out of them when
     idle** (Aguilar et al. 2018). Here, the place a placement rule
     gathers ants is the only way out.
3. **A rule aimed at a named behaviour comes out as "go to X", and "go to X"
   needs a map.** The ant's code has 24 destination functions (`*_pull`,
   `*_target`): to the store, the door, the brood, the way out, the ring, the
   crest. A stigmergic rule reacts to what is under the ant ("drop food where
   food lies"). A goal-directed rule has to know where its goal is. On a
   world that carries no local differences (cause 1), knowing where means a
   global map. That is the chain that produced the concessions. It also makes
   the rule set a controller for its targets, which grows one rule per
   failure and treats anything unplanned as a defect (§3).

Two things make all three worse:

- **The benchmark is a specification past the research frontier** (§6,
  checked). The current goals are a stable population, a colony living in
  its nest, separate brood, rest and food chambers, and foraging from far
  off, all from local rules. Each published nest model produces one feature,
  in isolation, mostly with no economy. Biology does not know the local cue
  that stacks chambers by depth: the one proposed (a CO₂ gradient) was
  vented away in the field and nothing changed. An independent 2026 attempt
  at the same goal hit the same failures and made the same concessions.
- **The process filters out the unplanned and makes exploration expensive**
  (§7). Unplanned outcomes become switches that suppress them, mutation is
  frozen in testing, and every experiment is 12 seeds × 300,000 frames with
  the whole economy on. So no local rule has ever been seen on its own, on a
  substrate that could carry it, in a nest that does not jam.

**Verdict:** the *goal* is right, and the *benchmark* chosen to prove it is
wrong. On *design*, the physics layer is right. The creature layer runs on a
medium that suits destruction and cannot carry the local differences that
construction and placement need. And its nest has one narrow exit, with the
brood inside it. The *process* is well tuned for verification and
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
| **Few rules, applied the same everywhere** | A handful per material kind | About 160 `PIXEL_PHYSICS_*` switches in the ant's modules, 116 of them listed in `how-the-ant-works.md` §12 as changing behaviour; about 13,000 lines of production code in `creature.rs` alone (§3.1) |
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

- **The founder genome *was* the design.** Founders start from authored
  weights (`ant.ron`'s instinct rows; today's hidden units 0–6 are each a
  hand-built circuit, `how-the-ant-works.md` §4). Evolution then drifts around
  that design. So approach 1 was also designed behaviour, written as weights,
  with a search too weak to move far from it.
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

| date | behaviour switches in the ant's modules (cumulative, by first appearance in history) | `creature.rs` lines | `brain.rs` lines |
|---|---|---|---|
| 2026-08-20 | — | 2,700 | 509 |
| 2026-09-09 | 4 | 16,584 (09-10) | 2,213 |
| 2026-09-18 | 37 (mostly probes, parallelism, breeding) | 29,638 (09-20) | 2,946 |
| 2026-09-30 | 82 | 40,896 (10-01) | 2,946 |
| 2026-10-14 | **158** | **54,938** | **3,291** |

*Method:* switch names are every `PIXEL_PHYSICS_*` string in `creature.rs`,
`creature/needs.rs`, `brood.rs`, `pheromone.rs`, `organism.rs` and
`brain.rs`. Each is dated by the first commit on `main` that introduced it
(`git log -S`). Line counts are taken at the last commit on `main` before each
date. `creature.rs` today splits into about **13,000 lines of code, 17,500 of
comments and 23,300 of tests**. 561 of the 3,407 non-merge commits on `main`
touch the ant's files.

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
by the next rule). It cannot give the owner an unplanned behaviour, because
an unplanned behaviour is exactly what the acceptance step treats as a defect.
The growth curve above is that loop running.

**Rules aimed at outcomes come out as destinations.** "Food stored apart"
becomes *carry food to the store*. "Living in the nest" becomes *walk in along
the way*. "Brood in one place" becomes *walk to the larvae*. The ant's code
has **24 functions named `*_pull` or `*_target`**, among them `home_pull`,
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
- **Brood is a powder that falls.** A brood cell *"does not roll and falls
  through bodies"*. Without `SHAFT_ROOMS`'s floor, *"the box alone let the
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

**This is not about the world being 2D.** A real ant farm is 2D, and ants dig
good nests in it, because moist sand has cohesion. It holds a roof, fails
gradually and records where the last cut was. A falling-sand cellular
automaton is defined by *not* doing that. The engine's chosen substrate is
excellent for destruction and poor for construction, and nests are
construction.

**Foraging was hard for related reasons** (inferred from the record, not
re-measured). The world is a vertical slice with gravity. Ants climb walls and
ceilings, a sensor 6 cells ahead on a diagonal lands in rock or sky (hence the
"honesty gate" and the row projection, `how-the-ant-works.md` §3), and trail scent pools in solid
ground. What is a short forage-and-follow loop in a top-down simulation became a
layered chooser, because the trail alone could not carry the ant home here
(round 36, 2026-09-15, before the trail reader was fixed: *"cutting the homing circuit out of the genome leaves deliveries
inside the noise"*).

---

## 5. Cause 2: one narrow way out, and rules that gather ants by the brood jam it (measured)

The first draft of this section said the ants had no energy margin. It took
the median ant's reserve from the boom-and-bust trace, which was measured at
famine, and estimated the trip by arithmetic. The owner asked for both to be
measured. **Measured, the claim was wrong**, and what the measurement found
in its place is below.

**Setup.** `deeptrace` on the shipped game, built from `bb8a2151`. The dry
goal box (`nest_goal`), the evolved founder, mutation off, seeds 1–4, 300,000
frames, `ants=60 hungry=1 mapevery=1000`. That records:
- every decision of 60 focal ants, refilled as they die;
- every decision of every hungry ant;
- every death, with its cause and place.

There were four arms:
- food 30 columns from the nest (the box's own);
- food 90 (`foodgap=90`);
- food 30 with `PIXEL_PHYSICS_NURSE_STAY=on`;
- food 30 with `PIXEL_PHYSICS_NEST_REST=workers`.

The last two are rules the record lists as failing through starvation (PLAN
§5). The run line and the three readers (`margin.py`, `starvers.py`,
`stuckway.py`) are in
[`why-no-emergence-2026-10-14/`](why-no-emergence-2026-10-14/). The run data
regenerates deterministically and is not in the repo. The reserve is read over
frames 100k–300k (the settled colony) and deaths over 20k–300k.

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

At food 30 the heap's zone touches the mound, so those trips are short. Food
90 is the real test. There the median forager leaves with about **nine round
trips** of reserve. The earlier arithmetic was wrong twice: it put the trip at
1,700+ frames against a measured median of about 930, and it took the reserve
from a colony at famine.

**The reserve is thin in only two places.** One is founding: 14–35 of the 52
founders starve before frame 20,000, starting on 200 J. The other is famine:
the boom-and-bust figure.

### 5.2 Who starves in a settled colony

Few ants starve, and not for want of reserve. Each starver's last hunger spell
began near its full grant (median 0.99–1.00 of `start_energy`) and lasted
2,300–3,300 frames, two to three round trips at food 90, without reaching the
heap. There are two kinds:

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
26–36 on the baseline, and colonies ended at 584–925 ants against 558–612. Its
recorded failure (*"deaths ran through the pellet trap"*) belonged to a `main`
from before the hunger-first rules, as PLAN §5 already notes.

**`NURSE_STAY=on` kills, through a jam.** It fired: crop food to brood rose
about 100-fold, with 248k–317k nurse seeks a run, and larvae starved 0–20
against 39–44. Adults starved **1,403–3,563 a run against 26–36**. Of the
1,349–3,482 starvers a run after frame 20,000:

- **99–100% had an open path to the door** when last seen (`starvewhere.py`:
  0–3 a run sealed in, pocketed or encased);
- they died in the nest (all of them on three seeds; 73% on seed 2), mostly
  under the `hungry out` pull toward the door, and moved on 52–56% of their
  decisions;
- each nest starver began its last hunger spell about 14 rows down, climbed
  to about 6, and died 8–9 down. **Only 1–3% came within 5 cells of the
  door** (median closest 10–12), after a median 4,200–4,500 frames hungry,
  about 40 round trips at this food distance;
- on three seeds the door shaft (7 columns × 16 rows under the door) held
  129–138 ants, 115–123% of its cells because ants stack, against 68–71 on
  the baseline. The fourth seed had no jam (69) and the fewest starvers.

So these ants had reserve and an open way out, and could not get through the
crowd standing in the shaft. Nurses gather at the brood, and the brood lies
in the shaft under the door. `SHAFT_ROOMS`' brood box sits at the door, and
*"most deep ants sit on the brood pile in the shaft under the door"*
(`how-we-test.md` §4).

### 5.4 What this changes

**Cause 2 is not the energy economy. It is the nest's geometry**: one narrow
way out, with the brood in it, and with it every rule that gathers ants by the
brood. The record had found the same jam from other directions:
- *"a direct way that is narrow and packed with ants"* (`moundway.py`);
- *"the solid column of ants in the door standing on brood"*
  (`fedboundary.py`);
- layers *"blocked by a crowd at the door"* (PLAN §5).

Real ants face exactly this problem in narrow tunnels. They solve it by
staying out of the tunnel when idle and backing out of clogs (Aguilar et al.
2018, §6). Here, the place an idle nurse is drawn to is inside the tunnel.

It also ties back to cause 1. A nest that carried local differences, such as
a brood place off the shaft, would not put its gathering point in its exit.
The brood is in the shaft because the ground carries no other place for it
(§4: brood is a powder that falls, and the box exists to catch it).

The hunger-first principles in PLAN §2.4 stand, because held starvers are
real. But they are guards against rule vetoes, not against a thin margin.

---

## 6. The target is past the research frontier (checked against the literature)

The owner's goals, held since 2026-10-03 (PLAN §1):
- **a stable population**;
- **the colony lives in its nest**;
- **separate chambers**: *"brood in one place, resting workers in another,
  food stored apart from the brood"*;
- **foraging from a far food spot**;
- **"All of it emerges from the ants' own rules. No drawn rooms, no scripted
  building, no fixed castes where a rule could do it."**

Each rule must cite the biology.

**Checked on 2026-10-14 by literature search.** This container's network
policy refused the papers themselves, so what follows rests on abstracts and
search summaries. One exception is the hobby model in item 4, which was read
from its repository. Sources are listed at the end of this report.

1. **No model found does what the goals ask.** Nothing, peer-reviewed or not,
   grows functionally different chambers (brood, food, rest) from local rules
   in a colony that must also feed and breed itself.
2. **Each published construction model produces one feature,** in a
   controlled arena and mostly at a fixed population:
   - **Bonabeau et al. 1998.** Pillars, walls and a *royal chamber*, from a
     cement pheromone plus a template the queen emits; air flow and traffic
     change which forms appear. This is the one published route to a
     *functional* chamber, and it is a template: a diffusing signal from the
     thing the chamber is for.
   - **Khuong et al. 2016.** Pillars and roofs from a cement pheromone, whose
     lifetime is the most influential parameter, plus the ant's own body as a
     ruler for roof height.
   - **Pitonakova & Bullock 2013.** Nest walls from a pheromone template and
     the ants' bodies.
   - **Toffin et al. 2009.** The round-to-branching shape, from digger
     density.
   - **Buhl et al. 2004 and 2005.** Excavation that stops on its own, with
     nest volume proportional to colony size.
   - ***Camponotus fellah*, eLife 2024/2025.** Each ant digs toward its own
     age-dependent space target, so colony demography shapes the nest. This
     is the nearest any model comes to coupling building to colony life.
   - **Ganga Prasath et al. 2022.** How diggers shift from random to
     coordinated digging.
3. **Biology does not know the local cue for stacking chambers by depth.**
   Tschinkel proposed the soil CO₂ gradient as the template. Venting the
   gradient away left nest architecture and ant distribution unchanged
   (Tschinkel 2013, *PLOS ONE*; the abstract says so). A whole-colony model's
   own sources page puts it plainly: *"The hypothesis is dead and no
   replacement exists."* That model hands each ant its depth as *"the largest
   invented element in the model."*
4. **The closest whole-colony effort hit this repo's failures.** ANThill-Sim
   (2026) is an open, cited hobby model of *Pogonomyrmex badius*, not
   peer-reviewed. It digs, forages, stores seed and feeds brood. Its README
   reports caverns where stacked chambers should be, and colonies that grow,
   then starve their larvae. It also made the concessions this repo made:
   - the founding nest is supplied;
   - founding spoil is *"counted onto the surface without her walking it
     up"*;
   - the share of diggers who go to the face is *"invented and fitted"*;
   - each ant is handed its own depth.
5. **Tunnels stand because of the medium.** Tunnel stability in granular soil
   depends on grain size, tunnel diameter and water content (*Granular
   Matter*, 2018). The soil around ant tunnels forms load-bearing arches
   (Buarque de Macedo et al. 2021, *PNAS*). Laboratory excavation is run in
   *wetted* glass beads (Aguilar et al. 2018). That is §4's falling-sand
   point, from the physics side.
6. **Narrow tunnels jam, and ants manage it.** Fire ants keep excavation
   flowing in narrow tunnels by individual idleness and by retreating from
   clogs (Aguilar et al. 2018, *Science*). That is §5's jam, solved by ants
   staying out of the way.

**So the claim stands, with a sharper edge.** The combination the goals ask
for is beyond published models. One piece of it, chambers stratified by
depth, is beyond published biology, so "cite the biology" cannot be met for
it. An independent attempt with the same aim made the same concessions this
repo did, which suggests the concessions are the problem's shape, not this
team's lapses.

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
owner's reasoning was sound: a game needs a colony that lasts. But it means
the clearest emergent dynamic the creature layer has produced was being
engineered against within days of being traced.

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
| **Engine design: the nest's way out** | **One narrow exit with the brood in it** | Not the economy: a forager carries about nine round trips of reserve. A rule that gathers ants by the brood packs the shaft, and the ants behind the jam starve with an open path (§5) |
| **Approach 1** | **Failed on search economics** | One fitness axis, unpaid verbs, slow generations, a designed starting point (§2) |
| **Approach 2** | **Succeeded as a controller, so it cannot give the unplanned** | Rules aimed at outcomes come out as destinations that need maps; one rule per failure, accepted on its target; about 160 switches; interference grows with the count (§3) |
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
   - the double-bridge shortest-path choice;
   - Khuong-style pillars from marked pellets.

   If the engine cannot produce these, the fault is in the medium, and no
   amount of rule-writing upstream will fix it. Each is days of work, not
   weeks. This also tests cause 1 directly, with no exit to jam.
2. **Make places differ locally.** This is what the concessions stand in for,
   so it is what lets them go.
   - Soil that holds a cut by its own physics, with graded,
     moisture-dependent strength and slow failure, instead of a lining stamp.
   - Pellets that carry a decaying mark on the cell.
   - Scent that diffuses only through open cells.
   - A per-cell channel the nest writes: odour from brood, food and bodies,
     which are the "air" `hungry_out_pull` says its BFS stands in for.

   Then retire each concession in §3.3 one at a time against the local signal
   that replaces it, the BFS "way" first.
3. **Keep the gathering points out of the exit, and score placement rules
   on the way out.** The brood sits in the door shaft because nothing else
   holds it (§4, `SHAFT_ROOMS`). Any rule that draws ants to the brood
   therefore draws them into the only way out.
   - Score every placement rule on the door shaft's occupancy as well as its
     own target. This report's `stuckway.py` and the repo's `moundway.py`
     and `fedboundary.py` already measure it.
   - Treat the brood's place as the thing to fix: a place the world itself
     marks (item 2), off the shaft, as real nests have.
   - The margin does not need raising. The measured reserve is about nine
     round trips (§5.1).
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
  seed. It is enough for a picture of the jam, not to settle its size.
  `NURSE_STAY` was run at food 30 only, and the food-90 arm without the
  placement rules. Every other behavioural number is quoted from the report
  named beside it. Switch dates, line counts, commit counts and word counts
  are counts from git and grep at `bb8a2151`; their method is stated where
  they appear.
- **The jam is traced, not intervened on.** The door shaft's crowd and the
  starvers' failure to reach the door go together on 3 of 4 seeds, and the
  unjammed seed had the fewest starvers. No run cleared the shaft to show the
  deaths stop.
- **The approach boundary (about 2026-09-19) is inferred** from the switch
  dates and the owner's correction. Switches dated before 09-19 are mostly
  probes, parallelism and breeding. After it they are behavioural
  (`DIG_DOWN`, `NEST_SHAFT`, `SPOIL_HAUL`, `HOME_TARGET`, …).
- **The individual-selection point in §2 is an inference**, not measured.
- **The literature in §6 rests on search results and abstracts.** This
  container's network policy refused the papers, so nothing in §6 was read in
  full except the ANThill-Sim repository. The reversed-CO₂ arm of Tschinkel
  2013 could not be read; only the vented arm is cited.
- **Plants (§8) were read, not traced.**
- **Not read whole:** `README.md`, `PLAN.md`, `dead-ends.md`,
  `open-bugs-handoff.md` and most dated ant reports. They were read by
  section and grep, per `CLAUDE.md`.

---

## Sources (§6)

- Bonabeau, Théraulaz, Deneubourg, Franks, Rafelsberger, Joly & Blanco (1998). *A model for the emergence of pillars, walls and royal chambers in termite nests.* Phil. Trans. R. Soc. B 353:1561–1576. [Bristol record](https://research-information.bris.ac.uk/en/publications/a-model-for-the-emergence-of-pillars-walls-and-royal-chambers-in-/); [SFI working paper abstract](https://ideas.repec.org/p/wop/safiwp/97-04-033.html)
- Ladley & Bullock (2005), the 3D extension. [Southampton eprint](https://eprints.soton.ac.uk/261442/1/ladley_bullock_jtb.pdf)
- Khuong et al. (2016). *Stigmergic construction and topochemical information shape ant nest architecture.* PNAS 113(5):1303–1308. [HAL](https://hal-univ-tlse3.archives-ouvertes.fr/hal-02324165); [PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC4747701)
- Pitonakova & Bullock (2013). *Controlling ant-based construction.* ECAL 2013. [Southampton eprint](https://eprints.soton.ac.uk/361147/1/controllingAntBasedConstruction.pdf)
- Buhl et al. (2005), excavation dynamics and nest volume against colony size. [HAL](https://hal-univ-tlse3.archives-ouvertes.fr/hal-00093178v1)
- *Colony demographics shape nest construction in* Camponotus fellah *ants*, eLife (2024 reviewed preprint; 2025 article). [eLife](https://elifesciences.org/articles/100706); [PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC13102390/); [bioRxiv](https://www.biorxiv.org/content/10.1101/2024.07.09.602713.full.pdf)
- Ganga Prasath et al. (2022), cooperative excavation, eLife. [eLife PDF](https://elifesciences.org/articles/79638.pdf)
- Tschinkel (2013). *Florida harvester ant nest architecture, nest relocation and soil carbon dioxide gradients.* PLOS ONE 8:e59911. [PMC](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3610692/)
- Tschinkel (2015). *The architecture of subterranean ant nests: beauty and mystery underfoot.* [AntWiki PDF](https://www.antwiki.org/w/images/8/83/Tschinkel,_W.R._2015._The_architecture_of_suberranean_ant_nests_-_buauty_and_mystery_underfoot.pdf)
- Aguilar et al. (2018). *Collective clog control: optimizing traffic flow in confined biological and robophysical excavation.* Science 361:672. [PDF](https://crablab.gatech.edu/pages/publications/pdf/science-2018.pdf)
- Espinoza & Santamarina (2010). *Ant tunneling — a granular media perspective.* Granular Matter 12:607–616 (abstract only). [mirror](https://community.altair.com/discussion/40523/ant-tunneling-a-granular-media-perspective)
- *Stability of granular tunnel* (Granular Matter, 2018). [arXiv](https://arxiv.org/pdf/1802.07906)
- Buarque de Macedo et al. (2021). *Unearthing real-time 3D ant tunneling mechanics.* PNAS. [Caltech](https://resolver.caltech.edu/CaltechAUTHORS:20210824-153629422)
- ANThill-Sim (2026), an open hobby model of *Pogonomyrmex badius*, not peer-reviewed; read from its repository (README and `docs/SCIENCE.md`, commit of 2026-09-25). [GitHub](https://github.com/Adiezc/ANThill-Sim)
