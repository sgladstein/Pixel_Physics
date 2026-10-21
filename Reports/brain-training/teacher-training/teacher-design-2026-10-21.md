# The teacher: design doc for the next steps (rules, planners, search, and the colony)

*2026-10-21, by the teacher-approach session. Status: **design, for adversarial and improvement review. Nothing in it
is built** beyond what step 1 already built (the three-rule teacher, the copier, the probe). It answers the owner's
question of 2026-10-21: keep the current teacher or improve it, and how a teacher can do more than simple rules
(pathfinding for soil, food and digging; the nest analysis tools choosing where to dig). Companion documents, new
today: [`search-teacher-design-2026-10-21.md`](search-teacher-design-2026-10-21.md) (the search teacher in full, from
a research agent; literature in [`notes/search-teacher-literature.md`](notes/search-teacher-literature.md)) and
[`notes/nest-architecture-2026-10-21.md`](notes/nest-architecture-2026-10-21.md) (what real nests look like in
vertical section, with sources). Background: [`guidelines-2026-10-20.md`](guidelines-2026-10-20.md) (T1-T9, C1-C6),
[`step1-results-2026-10-21.md`](step1-results-2026-10-21.md).*

## The answer, first

**Keep the current teacher for the 8-row step, and grow it into a framework of planners, with search on top as the
judge.** At 8 rows the hand teacher is not what limits us: driving s161 it already scores 20.66 full loads a world
against 6.8, three times the brain. What limits us is *copying*: one rule copies well (go down, 6.42 -> 10.57), two do
not (dig collapses the ant alone; walk home spreads to foragers). A smarter teacher at 8 rows would add headroom
nobody can copy yet.

**Past 8 rows there are no rules to write, so the teacher must compute.** The owner's three examples are the right
three, and each is a **planner**: a goal, a search on the true world for the cheapest way to it, and a rule for turning
the plan into a lesson on the options the ant actually has.

| Planner | Goal | What it computes that no ant could | Order |
|---|---|---|---|
| **Soil placement** | A ring mound round the entrance whose inner toe stays clear of the hole | Exactly where a pellet will slide once put down (it can try the drop in a copy of the world); the shortest carry to an unfilled spot on the ring | **first** |
| **Nest excavation** | A blueprint drawn from real nests, chosen by the owner by eye | Which cells of the blueprint are diggable *now* without dropping a roof or the rim; the order that keeps the nest connected; which ant should take which face | second |
| **Foraging** | Straight to known food, straight back | The shortest route on the true ground, around stone and over the mound | last (owner: no strong opinion; exploration stays the brain's) |

**The three rules we have are the simplest planners**: "go down" is a shortest path to the shaft's foot, "walk home"
a shortest path to the mouth, "dig at the foot" a one-cell plan. So keeping them and building planners is one design,
not two: the rules become the planners' degenerate case and the examiner of every later teacher.

**The answer to "will simple rules make a simple student"**: the student never copies a rule; it copies the rule's
*choices*, on its own states. A teacher that computes more makes better choices, and the student can learn every one of
them that it can tell apart from its senses. **So the teacher should compute as much as it likes; what it must not do
is ask for distinctions the ant cannot make** -- and that is measured, decision type by decision type, by the
copyability probe we already have. Where the probe fails, there are five ways out, cheapest first (section 3), and one
of them is the owner's: a new local, biologically realistic sense.

**Three ideas from the colony discussion stay in, unchanged** (section 6): put the teacher in a real crowd and let a
central planner assign the work; teach *outcomes*, not actions, for the colony-level parts (how big, when to stop,
where to bud); and crowding needs at least two ants, so nothing that reads it can be taught solo.

**The one result here that changes a plan on `main`**: real chambers are flat lenses one to two body lengths high
(2-4 cells in this game), three to twenty times wider than tall. The dig ladder's planned 8-16-cell-tall chambers
(`dig-rung-design.md`, steps 10-11) are three to five times taller than any real chamber. The owner judges which by
eye; renders are coming with this doc.

## 1. What a teacher is, in this framework

Every teacher has four parts. Writing them out is what lets a rule, a planner and a search share one copier.

| Part | The three rules (step 1) | A planner | The search teacher |
|---|---|---|---|
| **Goal** | Implicit: full loads | Explicit: a target shape, a target place for soil, food home | An outcome score (full loads, plus a value of what is in flight) |
| **Computation** | One step of geometry | A search on the true world (shortest paths, a trial drop in a copy, a dig-order over the blueprint) | Roll each option out in a copy of the world, with the brain carrying on |
| **Values per option** | Allowed or not | A cost-to-go per option: path length, how much of the plan an option advances, whether a drop stays put | Q(a) with a standard error |
| **Lesson** | The ceiling policy: the brain's own odds renormalised over the allowed options | **The same as the search's**: pi\*(a) ∝ p_brain(a) · exp(A(a)/tau), with A the option's advantage over the brain's average | pi\*(a) ∝ p_brain(a) · exp(A~(a)/tau), A~ shrunk by its noise |

**One lesson form for every teacher.** Turning a planner's costs into an advantage and a regularised target
(section 1.5 of the search design; Grill et al. 2020) does three things the step-1 ceiling policy did not:

- **It is graded** (T3): an option almost as good as the best gets almost as much of the lesson.
- **It stays near the brain** (tau sets how near). Step 1's two failures were both the lesson moving the brain too
  far where it was not taught: walk home spread from empty diggers to loaded ants and to foragers; dig, copied alone,
  removed the wall cuts the ant's descent depended on. Coaching (He et al. 2012) and the regularised target are the
  literature's answer to exactly this.
- **It can abstain**: where no option clearly beats the brain's average, the lesson is the brain itself, which is
  step 1's "no rule applies" row.

**The planner's cost becomes an advantage in full-load units** by one exchange rate per planner, measured, not chosen:
for the soil planner, the full loads s161 completes per frame of carrying (a cell of extra path costs that many full
loads); for the nest planner, full loads per frame of empty walking plus a blueprint term (section 5.4). tau is then
chosen on the selection worlds from {0.03, 0.1, 0.3}, as for search.

## 2. Decision: keep the current teacher, and how it changes

| Part of today's teacher | Decision | Why |
|---|---|---|
| **Go down** | **Keep, as is.** It is the one lesson that copied cleanly (+4.16 ± 0.48, kept skills held) | -- |
| **Dig only at the foot** | **Keep, but never copy it alone.** It is coupled to descent (alone it collapsed the copy to 3.35, 15 seeds under 3). Copy it after go-down is in, from the go-down copy, with the regularised target | The ceiling test showed the coupling first; the copy showed it again |
| **Walk home** | **Replace with the soil planner's carry path**, which is a walk home *only while loaded*, ending at a ring site rather than the mouth | Walk home was taught on empty diggers too, and the empty-forager look-alike is what broke fetch. The planner never labels an empty ant with "go home" |
| **The put-down rule** | Stays dropped (owner agreed). The soil planner supersedes it | "Put down sooner" hurt alone because the brain's carry and its turn home are coupled; a planner gives the whole carry, not a veto |
| **Its role** | **The examiner.** Every new teacher must agree with the rules where a rule fires, or explain why (search design, B2a) | A teacher that cannot find a +4.7-load lesson someone already wrote is broken |

**What "improve it" means, then**: not a better rule at 8 rows, but making the rules the first instances of a planner
interface, so that what follows (soil, nest, food, colony) needs no new copier, no new probe and no new judge.

## 3. The limit on any teacher: what the ant can tell apart

The owner's challenge (2026-10-21) was right: "it may only base its calls on things the ant can also sense" is too
narrow. The teacher may use anything. What is limited is **what the student can learn from it**: a student copying a
label it cannot predict from its own inputs learns the label's average over everything that looks the same to it
(Weihs et al. 2021, the imitation gap; T1). The limit is checked, not assumed:

**The gate, per decision type, per teacher, before any copying**: the copyability probe (`probe.py`) fits the
teacher's choice from the brain's own inputs on held-out worlds, with a positive control (a pure function of the
senses) and the within-decision shuffle as the negative control. Step 1 passed it for all three rules (94-100% held
out) and still failed to copy two of them, so **the probe is necessary, not sufficient**: it says the information is
there, not that the brain's one shared scorer can hold the lesson beside its other skills (section 7).

**Where the probe fails, five ways out, cheapest first:**

| Way out | What it is | Example here |
|---|---|---|
| 1. **An existing sense** | Often the probe fails on a feature set too small; widen it to all 22 senses, 12 memories and 45 option features | The ant already senses placed spoil (`SPOIL`) apart from native soil and gallery lining (`LINED`), the hole (`CUT`), a cut that would drop a roof (`ROOF_DROP`, `ROOF_CUT`), the headroom over a cell in body heights (`HEADROOM`), depth (`DEPTH`), the home vector (`HOME`, `HOME_DIST`), crowding (`CROWD`, `ROOM`) |
| 2. **Memory** | The probe on senses plus the 12 memory units, and on a short history of senses | How long since the last put-down; which way the ant came |
| 3. **Change the goal to one that local rules generate** | **The new idea in this doc.** A goal the ant cannot locate itself in is uncopyable however well it is planned. A goal built from the rules real ants follow is copyable by construction | Real nests are generated by local rules (section 5.1): dig down from a chamber's edge, widen at the bottom; stop the roof at about a body height (Khuong et al. 2016); bud when crowding at the wall falls (Toffin et al. 2009). A blueprint of that family is one the ant can follow from its senses |
| 4. **A new local, biologically realistic sense** (owner: "on the table if it is a local biologically realistic sense") | Added only when the probe names the missing fact and a real ant demonstrably senses it | **Slope under a put-down cell** (Pheidole oxyops read the pile's slope and drop where it turns from up to down; Tofilski & Ratnieks 2008). **The way back to the last food** (`LAST_FOOD`, already reserved in the option features as "missing"; desert ants hold a vector to a food site, Wehner). **A building scent on handled soil** (`BUILDING`, reserved; Khuong et al. 2016) |
| 5. **Teach the outcome instead** | Take the decision type out of the copy loss and leave it to selection on a colony score | Room size, when to stop digging, how many dig against forage (section 6) |

**A sense is a brain change**, so a new one goes through the brain lane, defaults on per the house rule only if it
measures as a gain or neutral, and is added to a genome with its new links at zero (so every existing brain is
unchanged until trained).

## 4. The soil-placement planner (first)

### 4.1 Why first

- **Every dig rung is scored on where the soil ends up**: a full load is a pellet carried up and put down at least
  three cells (Chebyshev) from every hole cell at the bell, after a 2,000-frame settle (`dig-rung-design.md` S3).
- **It is the owner's stated ideal** ("a mound round the entrance is ideal"), and the place the previous game
  struggled (section 4.4).
- **It is solo-teachable.** Nothing in it reads crowding.
- **It replaces the one rule that hurt** (walk home spreading to empty ants) with one that only ever labels loaded
  ants.

### 4.2 The goal: a ring with its crest held out from the lip

From the nest research (section 3 of the notes):

- **Real ants dump just beyond the ridge**, so loads roll down the *outer* slope: Dorymyrmex keep 73-82% of loads where
  they land, the rest roll outward, and "almost never" back to the entrance (Tofilski & Ratnieks 2005). Pheidole read
  the slope and drop where it turns from up to down. Several species make a one-sided crescent instead of a ring and
  move it downslope.
- **The geometry, for this world's grains**: a cell automaton with a 1:1 diagonal slide rests at about 45°. A mound of
  height h with its crest at radius r_c from the mouth's centre has its inner toe at about r_c − h. Hold the inner
  toe at least one body length (2 cells) outside the lip: **r_c ≥ half the mouth's width + 2 + h.** Every put-down
  goes at r ≥ r_c, on the outer flank, and the crest moves outward as the mound grows.
- **The planner's plan**: a target ring profile (height by column), seeded at a crest radius of 3 cells past the
  scoring zone's edge and grown outward as it fills, both sides of the mouth evenly (or one side, if the owner picks
  the crescent).

### 4.3 What it computes, and the lesson

For a **loaded** ant at each decision:

1. **Sites**: the cells on the plan that are open and under-filled, on the outer flank.
2. **Trial drop**: for each candidate put-down in reach, the planner puts the pellet down in a *copy* of the world and
   runs the grain physics for the settle (a few hundred frames, cells only, no minds). A drop that slides inside the
   scoring zone or into the hole is worth nothing; one that comes to rest on the plan is worth one full load. **This is
   privileged computation at its plainest**: no ant knows how a pile will slide; the planner finds out by trying.
3. **Carry path**: for each step option, the shortest walkable path (`STAND`/`CLIMB` cells) from the option's cell to
   the nearest site, on the true ground.
4. **Values**: drop options by their trial value; step options by the path length saved, at the measured exchange rate
   (section 1). Dig and other verbs while loaded: abstain (the brain's own odds).
5. **Lesson**: the regularised target. **Empty ants get no label from this planner.**

**Cost estimate** (to measure first): loaded decisions are about a third of a run's ~2,700; a trial drop is one copy
of a 256-wide bed plus ~300 frames of cells, which at ~58 µs a frame for the whole world is an upper bound of ~20 ms;
with ~3 drop options a decision, about a minute of CPU a world. Cache the trial by the local ground shape.

### 4.4 What went wrong last time, and whether this repeats it

The owner: "in the previous game attempts ... ants often dug into the mound and soil often spilled from the mound back
into the nest." The record (`Reports/dead-ends.md`) is specific, and it changes the plan:

| Earlier attempt (hand-written creature, not the brain) | What happened | What it means here |
|---|---|---|
| The spoil ring carried further out, the crest drop and the door collar (`SPOIL_RING`, `SPOIL_CREST`, `DOOR_COLLAR`, 2026-10-01/02) | **None reached what refilled the tunnels.** The largest traced source was **the loose rim of a crater falling in**, not the heap: the heap was mostly 15+ columns out already. The crest drop alone *raised* fill | **A good mound does not by itself stop spill-back.** The rim must not be cut. This is the nest planner's job (5.3), and the brain already senses it (`ROOF_DROP`, `ROOF_CUT`, both added for exactly this, PR 2). The dig ladder's own trace agrees: the falling grains came from rim cuts dropping the shaft's shoulder |
| The mound refused as a dig face (`MOUND_DIG`, `SOLID_MOUND`), and called foragers not digging (`FORAGER_NODIG`) | **Every version cost the colony its way to food**: the cuts into the mound were the passages through it; refusing them shut ants in to starve (colony mean 268-275 -> 134-198) | **Never forbid mound cuts by position.** The re-test condition the record itself names is "a rule that can tell a passage from a scrape (a cut that joins open space toward food or door, against one into a dead-end hollow)". **A planner can tell them apart, because it computes paths.** In the solo dig arena there is no food to reach, so the planner labels mound cuts "no"; in a colony arena it labels a mound cut "yes" only if it shortens a path between door and food |
| Home counted on the mound (`NEST_HOME=mound`) | Food stacked into towers over the door | The planner's goal is a shape, not a home; home stays the door's seed cells |

**So the honest prediction is mixed**: the planner should fix *where pellets go* (that is what it controls, and real
ants show the rule), and should not be expected to fix spill-back on its own. The test separates them: the trial-drop
value counts slides; the census at the bell counts fill in the shaft by cause (pellet slid, rim fell, put back), as the
dig ladder already prints.

### 4.5 Can the ant learn it? (prediction, to be probed)

The ring is close to sensable already: the step options carry `UP` (is that step up or down from the head) and
`HOME` (does it point home), and the context carries `HOME_DIST`. "Walk away from home while the ground rises; put down
where it starts to fall" is a function of those. **Prediction: the probe passes for step and put-down choices at
≥90% held out without a new sense.** If put-down choices fail, the missing fact is likely the slope at the target
cell, and the candidate sense is Pheidole's: the height difference across the put-down cell (a local reading of the
ground under the jaws).

## 5. The nest-excavation planner (second)

### 5.1 What real nests look like, and why that matters for copying

From the notes (Tschinkel's casts of about 20 species; the 2D lab nests of Toffin, Buhl, Gautrais):

- **The basic unit is a shaft with flat chambers hung on it** (Tschinkel's "shish-kebab"). Big nests add more
  shaft-chamber units; the shape does not change as the nest grows.
- **Chambers are flat lenses**: about 1-2 body lengths high at every area, floor horizontal, roof flat or slightly
  domed, 3-20 times wider than tall. The game ant is two cells long, so a true-scale chamber is **2-4 cells tall**;
  drawn twice as tall for legibility, 4-8.
- **Tunnels are about one body length wide** (fire ants 1.06 ± 0.23 body lengths; harvester shafts flatten to ~2.5 near
  the surface). The owner's 4-cell passage is 2 body lengths, inside the real range for upper shafts.
- **Most nests are top-heavy** (harvesters: half the chamber area in the top quarter; spacing grows with depth); one
  genus is bottom-heavy (Aphaenogaster floridana).
- **Size tracks the colony**: harvester area ∝ workers^0.87, depth ∝ workers^0.37, the same shape at every size; in 2D
  lab nests, area nearly proportional to workers.
- **A nest grows in a graded way**: one round cavity, then buds along its wall, then galleries, then a meshed network,
  and more ants make the switch sooner (Toffin et al. 2009, 2010; Gautrais et al. 2014). Not a binary.
- **The closest thing to an ant farm** with real ants (P. occidentalis behind glass; Halfen & Hasiotis 2010): galleries
  about 1.2 body lengths across, chambers 4-14 long by at most 2 high.

**Why this matters beyond realism**: every one of these shapes is generated by *local* rules (dig down from a chamber's
edge, widen at the bottom; roof at body height; spacing set by depth; bud when the wall is no longer crowded). **A
blueprint of this family is one the ant can find its place in from `DEPTH`, `HEADROOM`, `ROOFED`, `CROWD` and memory**
-- way out 3 in section 3. An arbitrary "ideal nest" drawn by hand would not be.

**The owner judges the blueprint by eye.** Five candidate blueprints from five real nest families, plus the ladder's
current plan for comparison, are drawn as ant-farm cross-sections at game scale (a 256 x 128 box, ground at row 64,
passages 4 cells, chambers 5 cells tall) in [`measurements/blueprints/blueprints-sheet.png`](measurements/blueprints/blueprints-sheet.png)
(specs: `nest-blueprint-specs.json` beside it; renderer: `render_blueprints.py`), and posted for a pick (review card
`20261021T051824537Z-683876`). The planner is not built until one is picked.

**Drawing them turned up a constraint on the mound.** In a 2D slice nothing spreads into a third dimension, so the
mound's area *is* the nest's: 440-990 open cells for these blueprints. Held to the spec's inner toe and a 45° slope,
every blueprint's mound comes out a flat-topped ring about 8 cells tall reaching 34-86 cells from the mouth, wider
than any real disc in body lengths. Real ants have three ways out, all usable here: a taller ring set back further, a
wide thin disc, or backfilling dead ends (about half the spoil in a glass-sided harvester nest, Halfen & Hasiotis
2010). The soil planner's ring must budget for this from the start.

### 5.2 The plan: a blueprint, a dig order, and a progress score

- **Blueprint**: a set of target open cells, fitted to the box (real nests are 10-100x deeper in body lengths than
  the box's ~30, so depths are relative; the Aphaenogaster family is the one real family that fits at true scale).
- **Frontier**: blueprint cells adjacent to the current void and reachable by a walking ant.
- **Dig order**, by priority: (1) **keep the nest connected** -- the shaft before the chamber it serves, top down;
  (2) **never a cut that drops a roof or the rim** -- reject any frontier cell whose cut fires `ROOF_DROP`, or whose
  trial cut in a copy drops a cell into the shaft (the same trial as the soil planner's, on the dig side); (3) nearest
  first.
- **Progress, by the nest tools we have**: `nestcensus::rooms_of` (a room cell is one with 7 of its 9 cells open; rooms
  are 4-connected; it counts rooms, their sizes, and the steps between them), `World::in_room`, `in_chamber`,
  `room_per_ant`; plus a **blueprint match**: open cells inside the blueprint minus open cells outside it, each
  normalised by the blueprint's area. Roofed void only: a hole open to the sky is not a room
  (`measuring-the-world.md`).

### 5.3 The lesson

For an **empty** ant:

- **Dig options**: a frontier cell on the plan and stable is worth its share of the blueprint; any cell outside the
  plan is worth a penalty (the "must-not-dig" of step 1, generalised); a rim or roof-dropping cut a larger penalty.
- **Step options**: path length saved to the ant's assigned frontier cell, on the true ground.
- **Assignment** (solo: the nearest frontier cell; in a crowd, section 6): one ant, one face.

For a **loaded** ant: the soil planner's lesson. The two planners share the hole and the plan, so a pellet is never
put down on a cell the nest plan will cut.

### 5.4 What the ant would need to follow it

| The plan says | The ant would need to sense | Have it? |
|---|---|---|
| Dig down here (the shaft) | Depth, the foot, the hole | Yes (`DEPTH`, `CUT`, the step 1 probe 94-100%) |
| Start a chamber at this depth | Depth, and how far since the last chamber | Depth yes; spacing from memory (to probe) |
| Widen sideways, not up | Headroom in body heights at the cut | Yes (`HEADROOM`, `ROOFED`): the body-size template real ants use (Khuong et al. 2016) |
| Stop widening this chamber | How crowded the room is | Only in a crowd (`CROWD`, `ROOM`): section 6 |
| Don't cut the rim or a roof | Whether a cut drops a cell | Yes (`ROOF_DROP`, `ROOF_CUT`) |
| Leave the mound alone (solo) / cut a passage (colony) | Placed spoil; whether the cut opens toward food or door | Spoil yes (`SPOIL`); "toward food" partly (`HOME`, scent); to probe |

**Prediction**: the shaft and the chamber's floor and roof copy (they are depth and headroom); the chamber's *width*
does not copy solo, because what sets it in real ants is crowding. That is the boundary between section 5 and
section 6.

## 6. The colony tier

### 6.1 Put the teacher in the crowd, and run a central planner

**Kept from the discussion.** At rung 9 (twelve ants) the nest planner runs in the real colony:

- **One plan, many ants**: the planner assigns frontier faces to the empty ants (a minimum-cost assignment over path
  lengths, re-solved every few hundred frames), and the soil plan's sites to the loaded ones. This is the central
  planner: it sees the whole colony.
- **Each ant's lesson is its own row** of that plan: dig your face, walk to it, carry to your site.
- **The imitation gap is at its worst here** (C5): "you take the left face because three ants are on the right" is
  information no ant has. So the probe runs on crowd decisions, with `CROWD` and `ROOM` and scent in the feature set,
  **before** any copying, and only the decision types that pass are copied.
- **Search, one ant at a time** (search design section 5): copy the colony, force one ant's option, let the others
  run the shared brain, score the colony; the planner proposes the options and the search judges them. Cost grows
  linearly with the ants (Bertsekas's one-agent-at-a-time rollout; SPARTA in Hanabi).

### 6.2 Teach outcomes, not actions, for the colony-level parts

**Kept from the discussion.** Some choices are only good or bad for the colony, and no ant can tell which from where
it stands:

| Colony-level choice | Taught as | Scored by |
|---|---|---|
| How big the nest gets; when to stop digging | **Outcome** | Blueprint match scaled to the colony's size (area ∝ workers^0.87; the 2D lab result is nearly proportional), so the same brain in colonies of N and 2N should dig clearly more for 2N (C4's acceptance test) |
| When a chamber buds a new one | Outcome | The graded shape sequence (round, budded, galleried) against crowding, from the 2D lab nests |
| How many ants dig, carry, forage | Outcome | The decomposed colony score (C1: cells dug, food home, brood raised), never a target split of jobs (Duarte et al. 2012) |
| What each chamber is for (brood deep, food stored, foragers near the top) | Outcome, later | Needs brood and stored food in the arena (below) |

"Taught as outcome" means: **not in the copy loss; selection on the colony score decides it**, with the planner's
blueprint as part of the score. The planner still matters there: it supplies the target the colony is scored against.

### 6.3 What the colony tier needs from the world

The owner's point, kept: "the correct signal might require us having food storage, brood, other ants in the nest".

- **Crowding needs two or more ants** (C4). In a solo arena `CROWD` reads 0, and anything that never matters is pruned:
  the links from `CROWD`, `SELF` and `AGE` should be protected through the solo ladder (C3).
- **Real nests put brood in the bottom third and food deep** (harvesters store seed 40-100 cm down; foragers stay in
  the top 15 cm). A blueprint with purposes needs brood and food in the arena. The lab has both already; the training
  arenas do not. This is a later rung, after sizing by crowding works.
- **No division of labour appears unless the world charges for switching** (C2, Ferrante et al. 2015). The first
  colony experiment varies the world, not the brain.

## 7. The bottleneck the teacher cannot fix: copying into one shared scorer

Step 1's clearest finding is that **the student, not the teacher, limited the result**. All three rules passed the
probe; two did not copy. Better teachers will hit the same wall, so the design has to say what it does about it:

| Lever | What it does | Status |
|---|---|---|
| **The regularised target** (section 1) | Keeps every lesson near what the brain already does, so it spreads less to look-alike states | New in this design; the search design's tau |
| **One lesson at a time, then selection** | Copy a lesson alone, then run selection from the copy with the kept-skill floor and the copy's revived links protected (slot 1498 and the `DEPTH` links 654, 699, 789, 834, 924, 1059, 1149, 1239, sent to the brain lane) | The go-down copy is the first candidate; s171 is the second (same ceiling, better kept skills) |
| **Kept-skill anchors** | Fetch and trail decisions in the copy pool with target = the brain itself | Built; held only partly at 10x |
| **A context probe** | Before copying a lesson, probe whether the ant can tell the taught states from the kept-skill states that look like them (e.g. empty digger against empty forager). If it cannot, the lesson will spread, and the answer is a context sense or a different lesson, not more weight | New; cheap (the probe with a different label) |
| **Capacity** | 16 scorer units and 8 hidden units serve every verb in every arena. If lessons keep interfering after the above, more capacity is a brain change for the brain lane | Not proposed yet; the evidence would be lessons that pass the context probe and still interfere |

## 8. The search teacher's place

In full in [`search-teacher-design-2026-10-21.md`](search-teacher-design-2026-10-21.md). In this framework:

- **It is the judge of planners**, and the teacher where no planner exists. A planner proposes options (dig sites,
  put-down sites) and supplies a value at the horizon; the search plays the options out with the brain carrying on and
  keeps what actually scores. "The planner proposes, the search disposes" catches a bad heuristic instead of teaching
  it.
- **Its key prediction is about cost**: big single calls (dig here or walk on: ~0.3-0.5 full loads at stake) need
  only a few rollouts; a single step (~0.02 at stake) needs thousands. So **search labels the big calls, and planners'
  own cost-to-go labels the steps.**
- **Its pilot is dig-or-not on the 8-row step**, checked against the hand rule (B2a: must find "dig at the foot"), with
  a positive control of known sign (B2c: put down clear of the zone).

**Decisions on the search design's three open questions** (for the review to challenge):

1. **Main arm R1** (the brain inside the rollouts; no human rules), **R2** (hand teacher inside) as its control. R1 is
   the one not limited by what a person writes, and R2 tells us whether a failure is the rollout policy or the search.
2. **Charge energy in the outcome**, at s161's measured exchange rate of full loads per joule (the soil arena runs on a
   fixed 2,400 J budget), and report the uncharged score beside it.
3. **Persistent deviations are allowed** ("take the deepest step for the next ten steps"). They name a kind of
   behaviour, not when to use it; the search still decides when.

## 9. How each planner is tested: the gauntlet

Every planner passes the same stages. Each can stop the work, and each has controls that can fail.

| Stage | What | Pass | Controls |
|---|---|---|---|
| 0. **Look at the plan** | Render the planner's target (the ring, the blueprint, the dig order) on a real bed, for the owner | The owner's eye | -- |
| 1. **The planner acting** (T9) | The planner drives s161 on the decisions it labels; the brain does the rest. Count every force and veto. GIF of sealed world 2 for the owner, counts in the card's `meta` | Full loads (or blueprint match) above the brain alone by more than two seed-clustered standard errors; kept skills not run | The step-1 ceiling as a reference (20.66); a planner with its plan shuffled (expect no gain) |
| 2. **Copyability** | The probe, per decision type, held out by world; the context probe (section 7) | Well above the floor; the context probe separates taught from kept-skill states | Positive: a pure function of the senses; negative: the within-decision shuffle |
| 3. **Copy alone** | Into quieted s161 (and s171), regularised target, anchors | -- | The copier's controls from step 1 (round trip, reachable target, shuffle) |
| 4. **Closed loop** | 96 evaluation worlds, standard errors clustered by seed, fetch and trail on their fixed worlds | Beats the start by more than two s.e. with kept skills at floor | -- |
| 5. **Combine** | Copy onto the previous lesson's copy | Above both alone | -- |
| 6. **Select from the copy** | A selection run from the copy with the kept-skill floor and protected links | Holds or improves the copy's gain | Selection from the uncopied start, same budget |
| 7. **Look again** | GIF of the copy against the start, for the owner, before anything is called done | The owner's eye | -- |

## 10. Order of work, and cost

All local unless the owner says otherwise (no VM without the owner's go).

| # | Step | Cost (estimate) | Gives |
|---|---|---|---|
| 0 | **Finish step 1**: the go-down copy's seed-to-seed spread; go-down into s171; the step-1 write-up with a GIF | hours | Whether 10.57 is a copy or a lucky copy; the first selection-from-a-copy candidate |
| 1 | **The owner picks a blueprint** from the renders (and a ring or a crescent) | owner's time | The nest planner's goal; the chamber height |
| 2 | **Soil planner**: build; render the ring; stage 1 on the 8-row arena (it should beat the brain's put-downs, and its trial drops should show how many of s161's pellets slide back) | ~1 day to build; ~1 CPU-hour to act | The first planner, and the size of the gain where the soil goes |
| 3 | **Search instrument** in parallel: force-option hook, determinism, clone cost, the pairing measurement S1 (~4 min) | ~1 day | Whether search is affordable at all; it gates step 6 |
| 4 | Soil planner stages 2-7 | ~1 day | A second copied lesson, or a named missing sense |
| 5 | **Nest planner** solo, on the roof and chamber rungs (8, 10-11): the shaft and the chamber's floor and roof | ~2 days | Chambers shaped like real ones, copyable or not |
| 6 | **Search pilot**, dig-or-not (B1-B3, then C) | ~1-2 CPU-hours | Whether search finds what the hand rule found, without being told |
| 7 | **Colony tier** at rung 9 (twelve ants): the nest planner in the crowd, assignment, the probe on crowd decisions; outcome scoring for size | days | Room sizing by crowding; the first colony lessons |
| 8 | **Foraging planner** (return leg and known-food leg; needs `LAST_FOOD` if the probe asks for it) | ~1 day | Straighter trips; measure the fetch ceiling first, against s91 |

**Compute is not the constraint for planners**: a path search or a short trial drop per decision is milliseconds,
against ~58 µs a frame for the whole world. Search is the expensive tier, and its own design prices it at 2-4% of a
selection run for a sampled round.

## 11. Risks, and what would make us stop

| Risk | Signal | Response |
|---|---|---|
| **A planner improves its own goal, not full loads** | Stage 1: blueprint match up, full loads flat or down | The goal is wrong for the score; ask the owner which the arena should pay |
| **The planner exploits the simulation** | A GIF that looks wrong while the score rises (posts by the mouth; chambers that no ant would dig) | The owner's eye is the bar; change the goal, not the brain |
| **Lessons copy alone and interfere together** (step 1 again) | Stage 5 below either alone | Section 7, in order; capacity last |
| **The blueprint's place cannot be sensed** | Stage 2 fails for chamber placement | Way out 3 (a goal generated by local rules) before way out 4 (a sense) |
| **The soil planner fixes placement and not spill-back** | Fill in the shaft unchanged, by cause: rim | Expected (section 4.4); it is the nest planner's rim rule |
| **A crowd decision depends on other ants' futures** | Stage 2 fails on crowd decisions | Outcome scoring for that decision type (6.2) |
| **Planner labels for steps are too weak to matter** | Copying step lessons moves nothing | Persistent lessons (a whole carry path as one lesson), as the search design's persistent deviations |

## Questions for the review

1. Is "one lesson form for every teacher" (the regularised target from a planner's cost) sound, or does converting a
   path length into full-load units at one exchange rate hide a bias (e.g. against long carries that pay at the
   settle)?
2. Is the soil planner's trial drop in a copy of the world the right way to value a put-down, given the 2,000-frame
   settle and that later pellets land on earlier ones?
3. Does "change the goal to one local rules generate" (way out 3) quietly make the teacher a hand-written rule again,
   which is what the owner wanted to move past?
4. Is the order right: soil before nest before food, search instrument in parallel?
5. What should the first colony experiment be, given C2 (no division of labour without a switching cost)?
6. Are there planning methods for agents that learn from a planner (beyond DAgger, AggreVaTe, expert iteration and
   coaching) that would change any of this?
