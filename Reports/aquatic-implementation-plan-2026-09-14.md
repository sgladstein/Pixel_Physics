# The pond is a data file, the plant is one predicate, and the swimmer has to come last

*Implementation plan, 2026-09-14. Turns
[`aquatic-life-research-2026-09-14.md`](aquatic-life-research-2026-09-14.md)
into a build order with prices, guards and briefs. **Phase 0 is built and
measured in this branch**; Phases 1–4 are specified and not built. Numbers
marked **(measured)** were taken here on 2026-09-14 at `RAYON_NUM_THREADS=1`,
release; **OWED** names the run that would take them.*

**Status: plan of record for aquatic work. Phase 0 landed; nothing else
built.** Two review cards carry decisions that are not a lane's to make:
`20260914T030227009Z-0c5076` (does a deep pool go dark — Phase 4) and
`20260914T040932865Z-14b62b` (is this the pond bed you want — Phase 0).

---

## 0a. The owner's verdicts on the three cards, 2026-09-14

All three came back while the measurements above were running, and each moves
something. Recorded here rather than folded away silently, because two of them
change what a phase is for.

**Card `…0c5076` — does a deep pool go dark? (gated Phase 3.)**
> *"prototype and show me what it looks like. Be realistic about the depth"*

**Phase 3 is ungated, and the deliverable is a picture rather than a
proposal.** But *"be realistic about the depth"* collides head-on with §4.1's
measurement and that has to be said before anything is built: real water is
nearly clear over a pond's depth — a lake's euphotic zone runs to tens of
metres — and the lab world is 320 cells tall with a 28-row pond that is
**1.75 field cells**. A physically honest extinction coefficient over 28 rows
is *no visible gradient at all*. So realism and a visible dark bottom are in
tension here, and the prototype's job is to show the owner that tension rather
than to quietly pick one: render the same pool at a realistic coefficient and
at a legible one, side by side, and let the choice be made on the picture.
**Depth is substantially an outdoor feature** for the same reason — ~80 rows
buys a five-step gradient, a quarter of the lab world and nothing at all to a
world that streams.

**Card `…14b62b` — is this the pond bed you want?**
> *"This is fine to start, but in nature it would need a sloped enterance or
> creatures will get stuck. That might happen here"*

**It does happen here, and it was measured before the verdict arrived without
either knowing about the other.** §1.9's `cols` column says the vertical-walled
pit costs the colony **29% of its range on 6 of 6 seeds with the distributions
completely separated**. The owner read it off a still picture; the harness read
it off a six-seed sweep; they are the same finding. **The sloped bank therefore
stops being a nicety and becomes a testable claim**: if the slope is what the
pit is costing, a stepped bank should recover that range toward the flat bed's
279.5, and if it does not, the barrier is something else and the plan should
say so. `shore_slope_dry` is that arm — the identical footprint and depth,
walls stepped 2 rows per 4 columns, so **step height** (what an ant actually
climbs) is what changes rather than the average gradient.

**Card `…bdca4a` — ants standing on water; drown, float or swim at generation
zero?**
> *"most drown? what is realistic?"*

**A question back, and it deserves a real answer rather than a default.** The
honest one is that *float* is realistic for an animal this size and *drown* is
realistic for the engine's physics, and they disagree — which is worth the
owner knowing before E9 is priced. At ant scale surface tension dominates
gravity: real ants are hard to drown, float readily, and some species raft for
days. But this engine has no surface tension, and its creature material is
**exactly water density**, so a body that enters water is neutrally buoyant and
simply hangs there — which is what `land_afloat` already models and what the
card's own picture shows. So "mostly drown" is not what physics at this scale
says; it is a *game* choice, and a defensible one, because drowning is the only
mechanism yet proposed that puts carrion in the world (§5). **The plan's
recommendation is therefore to make the answer heritable and start the
distribution wide rather than pick a default** — E9's own ruling was that all
three be reachable — and to price drowning so it is survivable-but-costly at
generation zero, with the carrion it produces as the payoff rather than the
punishment.

---

## 0. The answer

**Three things measured tonight set the order, and two of them overturn what
the research assumed.**

1. **A pond in the lab is a scenario file, not an engine change.** The research
   said a basin would have to be a `LabBox` field. It does not:
   `Placement::Fill` already accepts `"water"`. `assets/lab_scenarios/the_pond.ron`
   ships with this plan and holds **4,256 water cells, exactly constant from
   frame 0 to 40,000 (measured)**.
2. **But only if its floor is impermeable.** The same basin with a soil floor
   drains **3,648,000 fill units to 3,000 by frame 2,000 and to zero by frame
   4,000 (measured)** — the bed's own 96-row soil column has almost exactly
   enough unsaturated capacity to swallow the pond. Stone holds; soil drinks.
   **This is the constraint every later phase inherits**, because a plant needs
   sediment and sediment is what empties the pond.
3. **A pond as a bare barrier buys nothing.** Three arms, **six seeds each**,
   colony landing on a grown bed at frame 6,000, ants alive at frame 90,000:
   flat bed median **1.0**, water-filled pit **2.0**, dry pit **2.5**
   (measured). Water is not distinguishable from the same hole left dry, and
   what little a pit buys, it buys dry. **So the plan must not be sold on the
   pond as spatial structure.** It has to be sold on calories, which is the
   research's own conclusion arriving from a different direction. §1.4 also
   carries the warning that matters more: every arm collapses from 52 founders
   to one or two ants, so **this bed cannot discriminate a pond effect at all**
   and Phase 2 must not be measured by colony size in it.

**Therefore the order is: bed → plant → animal → gradient.** Not
animal-first. A swimmer authored before there is anything in the water is the
hop again — shipped 2026-08-29, correct, guarded, and used **zero times in
every real scene** because nothing rewarded it. Survival tracks eating at
+0.895 and nothing else.

| phase | what the player gets | engine cost | status |
|---|---|---|---|
| **0 — the bed** | a pond that holds, and the harnesses to read it | **none** | **built here** |
| **1 — the margin** | plants that live at and under the waterline | one predicate, one species, one bed rule | specified |
| **2 — the body** | drown, float and swim as a heritable trait (E9) | one trait slot, one verb, one clock | specified |
| **3 — the gradient** | a pool that goes dark with depth | one predicate + per-material extinction | **gated on a card** |
| **4 — the loop** | detritus, drowning, carrion that sinks | small, and mostly falls out | sketch only |

---

## 1. Phase 0 — built and measured in this branch

### 1.1 `the_pond.ron` — the bed

A stone-lined basin, 152 wide and 28 deep, in the standard 512×320 lab bed. Both
numbers are chosen rather than picked: **wide**, because `evaporation.rs`'s
humidity shelter asymptotes by 128 cells so a body this wide saturates the air
above itself and holds; **deep**, because `pond_min_depth` is 2.0 and a one-cell
film "renders as a black line rather than as water and reads as an artifact" —
28 rows is a water column with an inside.

| | frame 0 | frame 20,000 | frame 40,000 |
|---|---|---|---|
| water cells | 4,256 | 4,256 | 4,256 |
| total fill | 4,256,000 | 4,256,952 | 4,256,952 |

Exactly stable. The +952 units between 0 and 20,000 is the sealed box's own lid
condensation (`weather::condense_under_a_lid`), and it then stops.

### 1.2 A still pond is free, and §1f does not bite here

Three arms, one binary, `RAYON_NUM_THREADS=1`, 40,000 frames **(measured)**:

| arm | median ms/tick | mean | p90 |
|---|---|---|---|
| dry basin, no water | 0.012 | 0.013 | 0.013 |
| the pond | 0.012 | 0.015 | 0.018 |
| the pond with a stone block in it | 0.013 | 0.016 | 0.019 |

`open-bugs-handoff.md` §1f says *a pond with rock in it never stops shuffling
fill*. **This is not a refutation of it and must not be quoted as one.** §1f is
a chunk-wake claim measured on the outdoor world with natural pond geometry;
this is a stone-lined rectangle, the most favourable possible shape, and
ms/tick over a near-empty box is far too coarse to see four awake chunks.
What it does establish is narrower and still useful: **at the lab bed's scale,
a pond — with or without an object in it — costs about one microsecond a tick
and its cell counts are bit-stable.** The lab is not where §1f bites.
**OWED**: an awake-chunk census, which no harness currently prints for a lab
scenario.

### 1.3 A plant cannot live in or over water — measured, five positions

`pond_plants`, one herb at each of five positions, founder cell counts by id:

| position | frame 0 | frame 6,000 | frame 30,000 |
|---|---|---|---|
| x=200, over open water | 1 | **dead** | dead |
| x=256, over open water | 1 | 1 | **dead** |
| x=178, on the stone lip | 1 | 1 | **dead** |
| x=170, soil bank, one cell from the lip | 1 | 190 | **249** |
| x=340, soil bank | 1 | 188 | **469** |

Three of three die in or over the water; both on soil one cell away thrive.
The two that sat at **1 cell** for 6,000 frames before dying are the
germination gate refusing, which is `plant.rs:6198`'s deliberate guard against
a floating seed reading as wet ground. This is `growable` and
`cell_carries_nutrient` refusing, exactly as the research predicted, and it is
now measured rather than read off the source.

### 1.4 The barrier alone buys nothing — and the bed cannot tell, which matters more

Three arms, colony arriving on a grown bed at frame 6,000 via the scenario
timeline (**not** dropped at frame 0 — a colony dropped on seedlings collapses
regardless, which would confound everything). **Six seeds per arm, 90,000
frames, `RAYON_NUM_THREADS=1` (measured)**, ants alive at the end:

| arm | the six seeds, sorted | median | mean | births (median) | plants (median) |
|---|---|---|---|---|---|
| flat bed, no pit | 0, 0, 1, 1, 2, 3 | **1.0** | 1.17 | 6.0 | 445 |
| pit filled with water | 0, 0, 1, 3, 4, 10 | **2.0** | 3.00 | 10.5 | 295 |
| pit left dry | 0, 1, 1, 4, 8, 28 | **2.5** | 7.00 | 8.5 | 293 |

**The single-seed reading this section first carried — flat 3, water 4, dry pit
28 — was seed 1, and seed 1's 28 is the outlier of the whole sweep.** Drop it
and the dry pit is 0, 1, 1, 4, 8: a median of 1. The arm's mean of 7.00 is that
one run and nothing else. This is `CLAUDE.md`'s *compare two runs, not one run
against a remembered number*, and its *order statistic, never a single seed* —
and it is worth leaving the wrong first reading visible, because the wrong
reading is what a one-seed pond arm looks like.

**What survives.** Water is not distinguishable from the same hole left dry:
medians 2.0 against 2.5, distributions overlapping almost completely. Both pits
are *marginally* above a flat bed (medians 2.0 / 2.5 against 1.0, births 10.5 /
8.5 against 6.0), and a pit costs plants (median 295 / 293 against 445) because
it removes growing area. **So the conclusion the plan is built on holds and is
now properly supported: a pond as a bare barrier buys nothing — and what little
a pit does buy, it buys dry.**

**And the more useful finding is about the instrument.** Every arm ends with a
median of one to three ants out of **52 founders**. That is a near-total
collapse in all three, so this bed at this horizon has **almost no power to
discriminate anything** — a real pond effect of the size anyone would care
about could hide inside it. Two consequences for the briefs:

- **Do not measure Phase 2 by colony size in this bed.** A swim verb that saved
  a colony would have to lift it out of a collapse, and the collapse is not
  about water. Measure the verb directly: submerged ticks, drownings, and
  **food eaten while submerged**.
- **The collapse itself is worth a separate look and is not this plan's.** It
  reproduces at 6 seeds across three quite different beds, which is a cleaner
  statement of the long-horizon problem than any aquatic question.

**The three arms, so this is reproducible without shipping two more beds.**
The water arm is `the_pond_stocked.ron` as it ships. The **dry pit** arm is
that file with the one `Fill(material: "water", …)` line deleted. The **flat
bed** arm is that file with the `Clear`, both stone walls, the stone floor and
the water all deleted — the same plants and the same frame-6,000 colony on an
untouched bed. Drop the three into a directory and point
`PIXEL_PHYSICS_LAB_SCENARIOS` at it; `labshot scenario=<arm> seed=<n>
frames=90000` is the run, and `seed=` overrides the scenario's own bed seed.

**Six seeds is not a sweep**, and this repo has measured that specifically:
§S2's anchor-rule census read 1.64x over its first six seeds and 1.08x over the
next twelve. Treat the table above as ruling out a *large* water effect, not as
a measurement of a small one.

**Confirmed a third time, on a different instrument, in §1.8.** The same water
and dry beds raced through `creature_arena arm=lethal` — the sharpest
discrimination test the lab has, rather than a colony count — read median
**33.3% against 35.3%**, 5 of 6 seeds against 6 of 6. Three instruments have
now failed to tell a pond from the same hole left dry. That is not yet proof
there is nothing to find, and §1.8 says why it could not be: at one to
seventeen survivors these beds have very little power. But it does mean **no
brief should be written on the premise that the water alone is spatial
structure**, and the plan does not write one.

### 1.5 A method note worth keeping: the instrument said the opposite of the picture

Building §1.3 I first tried a **soil-floored** basin so a plant could root in
sediment. The founder counts came back 227 and 320 cells — *plants thriving
underwater*, which would have been the finding of the session. The contact
sheet showed **an empty pit**: the pond had drained and they were growing in
dry ground. `CLAUDE.md`'s *a scene that contradicts the code will look like a
bug in the code*, and its inverse — a number that is arithmetically correct and
about a world that no longer exists. **Look at the picture before believing the
counter**, every time, including when the counter is telling you what you hoped.

### 1.6 A rootable pond needs no engine change either — and the plant still dies in it

§1.5's basin drained because its soil floor was continuous with the bed's own
96-row soil column. **That column's unsaturated room is 160 x 60 x (1000-620)
= 3,648,000 units, which is exactly the pond's fill** — hence the drain to
precisely zero.

Seal it instead: keep the stone floor and lay a **four-row sediment layer on
top of it**, inside the basin. Its room is 152 x 4 x 380 = **231,040 units,
6.3% of the pond**. Predicted before running; measured after **(measured)**:

| frame | 0 | 5,000 | 10,000 | 15,000 | 20,000 |
|---|---|---|---|---|---|
| total fill | 3,648,000 | 3,416,949 | 3,416,949 | 3,416,949 | 3,416,949 |

A loss of **231,051** against a predicted 231,040 — eleven units, 0.005%. The
sediment drinks exactly its capacity and stops. **So Brief A0's code change is
not needed**: a pond with ground a root can reach is a scenario file, like the
pond itself. A0 becomes a bed rather than an engine change, which is the
cheapest correction in this document.

**And the plant still dies, which is the point.** Two herbs placed in that
submerged sediment, in a pond that now holds:

| | frame 0 | 6,000 | 20,000 | 40,000 |
|---|---|---|---|---|
| founder cells | 1, 1 | dead, 1 | dead, dead | dead, dead |

Neither ever reached a second cell. **This is the controlled form of §1.3**, and
it is the cleanest demonstration in either document: the *same bed*, the *same
two plant positions*, differing only in whether the water stayed —

- water drained away (§1.5): **227 and 320 cells**;
- water held (here): **dead, without growing once**.

Soil, light, nutrient and placement are all held fixed. Water is the whole
difference, and Phase 1 is therefore exactly as scoped: one predicate.

### 1.7 The bed that proves Phase 1 is not the bed that proves Phase 2

`the_pond.ron` sinks its water four rows below a stone lip. That is right for
the hydraulic question and **useless for the locomotion one**: ants walking
east along that bed stop at x=176, the lip, and the animal bounding box never
enters the pond's footprint. They never touch the water at all. A swim brief
measured in that bed would measure nothing and could not tell that from a
broken verb.

Flush the waterline with the bed surface (`the_pond_shore.ron`, shipped here)
and an ant meets it head-on. Measured at 6x with the marker overlay and
**rain off**, frames 7,400–8,000: **ants walk out of the bank and stand on the
water surface**, five or more cells from the shore. That is §R2's locomotion
half, live, in a bed Phase 2 can be measured in.

Two cautions that cost time here:

- **`animals span x A..B` is a bounding box over every animal**, so a high max-x
  and a high max-y can come from two different ants — one east of the shore and
  one dug into the bank read identically to one ant out on the water. The span
  is what first suggested this and it could not establish it; **the marker
  overlay at 6x is what did.** `CLAUDE.md`'s *ask what your number counts*,
  in its bounding-box costume.
- **`labgif` defaults to `rain=steady`**, which puts water on the bank and into
  the pond. Pass `rain=off` for any question about where the waterline is.

**So Phase 2's briefs measure in `the_pond_shore.ron` and Phase 1's in
`the_pond_sediment.ron`**, and neither is a drop-in for the other. This is the
`CLAUDE.md` rule about a scene failing to contain the defect you are removing,
met in advance for once.

### 1.8 Gate 2 on the pond bed: it passes, and it will still not see a swimmer

**The question this answers is whether Phase 2 can be judged at all**, and it
was taken before any of Phase 2 was written, per `CLAUDE.md`'s *check that a
planned step can demonstrate itself before promising it will*. `creature_arena
arm=lethal` races the shipped ant against a **zeroed** brain: if a bed cannot
put that arm behind, nothing measured in it is interpretable. It could not be
pointed at a pond until §6's harness change; this is its first run there.

Three arms, one binary (md5 recorded before and after the sweep and identical),
`RAYON_NUM_THREADS=4`, six seeds, 24,000 frames — which is the **18,000 the
grant needs after a 6,000-frame founding**, not the 12,000 the grant alone
suggests.

| arm | per-seed B share | median | seeds with B behind | survivors of 52 |
|---|---|---|---|---|
| **water** (`the_pond_shore`) | 26.7, 28.6, 30.0, 50.0, 43.8, 33.3 | **33.3%** | 5 of 6 | 15, 7, 10, 2, 16, 6 |
| **dry** (same file, water `Fill` deleted) | 42.9, 36.4, 28.6, 0.0, 35.3, 0.0 | **35.3%** | 6 of 6 | 14, 11, 7, 6, 17, 1 |
| **flat** (the default flag bed) | 25.0, 25.0, 0.0, 0.0, 0.0, 0.0 | **0.0%** | 6 of 6 | 4, 4, 1, 0, 2, 2 |

**All three beds have teeth, so Phase 2 has a bed it can be judged in.** That
is the finding the phase needed and it is a green light.

**Water against dry is the only clean comparison here, and it shows nothing.**
The two files differ by one line — the placement counter reads 11,552 cells
against 6,688, and the difference of **4,864 is exactly 152 x 32**, the water
fill and nothing else. Median 33.3% against 35.3%, 5 of 6 against 6 of 6: the
water is not distinguishable from the same hole left dry. That is the **third**
independent arrival at §1.4's finding, now on the sharpest instrument the lab
has rather than on colony size.

**Do not read the flat bed's 0.0% as "the pond discriminates worse."** It is
the comparison that is broken, not the beds: the flat arm founds at frame 0 and
gets 24,000 frames of mortality where the pit arms found at 6,000 and get
18,000, so it differs in exposure, in larder and in bed all at once. Its
survivor counts say the same thing from the other side — **4, 4, 1, 0, 2, 2 of
52 founders**, a near-total wipeout in which "B share 0%" is a statement about
one to four animals. A bed that kills almost everyone eliminates the weaker arm
by attrition, which looks like sharper discrimination and is not the same
claim.

**And the result that governs Phase 2 is in the survivor column, not the share
column.** `arm=lethal` is a **maximal-effect** test — a brainless ant against a
whole brain is the largest fitness difference this bed can be shown, and
`Reports/instruments.md` already records that this bounds what passing
licenses: small-effect races null in this harness on a power problem, which is
why flight nulled. A swim verb is a small effect, and these beds end with
**one to seventeen animals**. So Gate 2 passing licenses the bed and says
nothing about whether a swimmer would be visible in it.

**This is the plan's own §3 instruction, now measured rather than argued: judge
Phase 2 by the verb's own counters — submerged ticks, drownings, swim moves,
and above all cells of food eaten while submerged — never by colony size.** A
swim arm raced on population in this bed would null, and a null there would be
a statement about the bed's power, not about the verb.

**The table survives a merge that should have moved it, and why that is not
luck.** These arms were taken 20 commits behind `main`, which had meanwhile
shipped **rivalry on by default** (`ant.ron` `scent_spread` 0 → 2.0, PR #423) —
a behavioural change to the very animal being raced. Re-run on the merged tree,
all three arms came back **digit-identical on every seed**. That is
`CLAUDE.md`'s own tell for a stale binary, so it was checked rather than
believed: the md5 moved and `scent_spread` is in the binary's strings.

The dial is inert here **by construction**. Its own knob text says the scent
offset is drawn *per founding click* — *"at 0 every click is one family"* — and
`creature_arena` founds exactly one colony, on the flag path by hardcoded
`colonies: 1` and on the scenario path because every bed shipped today founds
from a single `Colony` event. One draw, one family, nobody a stranger.

**Two things follow, and the second is the one to carry into Phase 2.** The
table above is merge-stable, so it stands. And **every race this harness runs is
a race in a world with rivalry switched off** — so a Phase 2 result measured
here is silent about whether a swimmer that crosses into another colony's water
is treated as a stranger, which is exactly the kind of question §5's ecology
phase would want to ask. That needs `conflict_arena` or a bed that founds
twice, and `conflict_arena` cannot take a `scenario=` either. The general form
is worth stating plainly because nothing warns you: **a dial that fires between
two instances of a thing is invisible to any harness that makes only one, and
the harness reports a clean unchanged number rather than an error.**

### 1.9 The hole is the spatial structure. The water is not.

**This is the measurement §1.4 could not make and the research named as the one
that decides whether any of this is worth building.** `labforage`'s stated
question is already *"gone, or never got to it"* — its `unvisited` column is
standing food in columns the colony has **never occupied**, and `cols` is how
many columns it reached at all. Colony size could not separate those; this can.

Three arms, one binary (md5 identical across all twelve runs), six seeds,
24,000 frames, `RAYON_NUM_THREADS` pinned. Each arm is the one above it with
one thing deleted: the water `Fill`, then the pit itself.

| arm | `cols` per seed | `cols` median | `unvisited` median | `eats` median |
|---|---|---|---|---|
| **water** | 199, 200, 205, 199, 218, 180 | **199.5** | 158.5 | 823.0 |
| **dry** | 196, 192, 207, 236, 191, 197 | **196.5** | 154.5 | 826.5 |
| **flat** (no pit at all) | 307, 308, 273, 286, 253, 269 | **279.5** | 138.5 | 761.5 |

**The pit is a barrier, and a large one.** Flat reaches 279.5 columns against
roughly 197–200 for either pit — a **29% loss of range**, on **6 of 6 seeds
with the two distributions completely separated**: the worst flat seed (253) is
still further-ranging than the best pit seed (236). Nothing in this repo's
usual spread survives that cleanly; it is about as unambiguous as a
six-seed result here gets.

**And the water contributes none of it.** Water against dry is 199.5 against
196.5, three seeds up and three down, with `unvisited` (158.5 / 154.5) and
`eats` (823 / 827) equally flat. §1.4 guessed at this — *"what little a pit
buys, it buys dry"* — on an instrument it then showed had almost no power.
This establishes it on one that does.

**The flat arm is also the sensitivity control, and that is why it was run at
all.** A null is worth very little until the instrument has been shown to move
for a case you know differs: `cols` moved 29% with complete separation the
moment the pit was removed, so the water null is a real null rather than a dead
probe. `CLAUDE.md` asks for exactly this pairing and it is cheap — one more arm
of an existing sweep.

**What it changes, and what it does not.** The research's stop condition was
*"if the delta is ≈0, the aquatic niche is decoration and this document's
recommendation changes."* The delta between **pond and no-pond** is emphatically
not zero; the delta attributable to **the water** is. So the recommendation
stands unchanged, because it never rested on water-as-barrier — it rests on
there being calories in the water, which is Phase 1's job and not Phase 2's.
**Four instruments have now failed to distinguish a pond from the same hole
left dry** (colony survival, births, `creature_arena arm=lethal`, and this),
and the honest summary is that **in this engine today the water is scenery and
the hole is the mechanic.** A brief that proposes water as spatial structure is
proposing something four measurements say is not there; a brief that puts food
in the water first is proposing the only thing that would change that.

### 1.10 The owner's sloped entrance, measured: it gives back 92% of the loss

§0a's second verdict — *"in nature it would need a sloped enterance or
creatures will get stuck"* — arrived after §1.9 had independently measured the
sticking. `shore_slope_dry` tests the remedy: **identical footprint and depth**
to the vertical pit, walls stepped **2 rows per 4 columns**. The step height is
deliberately the small number, because what an ant climbs is a step, not an
average gradient.

| arm | `cols` per seed | median | `unvisited` |
|---|---|---|---|
| vertical pit, dry | 196, 192, 207, 236, 191, 197 | 196.5 | 154.5 |
| **sloped pit, dry** | 290, 273, 272, 252, 251, 276 | **272.5** | 137.5 |
| no pit at all | 307, 308, 273, 286, 253, 269 | 279.5 | 138.5 |

**The vertical wall costs 83 columns of range and the slope returns 76 of them
— 92%.** Sloped against vertical is **completely separated on 6 of 6 seeds**
(worst slope 251, best vertical 236), and sloped against no-pit-at-all overlaps
heavily (251–290 against 253–308), which is the shape of a fix rather than an
improvement. `unvisited` says the same from the larder's side: 137.5 against
the flat bed's 138.5, where the vertical pit left 154.5 standing.

**So the bed gets a sloped bank, and the vertical-walled basin should be
understood as an obstacle rather than a habitat.** That reaches back through
this whole document: `the_pond.ron`, `the_pond_sediment.ron`,
`the_pond_stocked.ron` and `the_pond_shore.ron` are all stone-lined boxes, and
every one of them is costing a third of the colony's range for reasons that
have nothing to do with water.

**What this arm does *not* license, stated because the columns are right there
and invite it.** The sloped arm places **8,576 cells against the vertical's
6,688** — the ramp is made of stone, so it replaces soil the vertical wall left
alone. `eats` (621 against 827) and `alive` (4.0 against 8.5) are therefore
**not a clean comparison** and are not read here: the two beds differ in how
much diggable, plantable ground they contain as well as in wall shape. The
range claim survives that confound because `cols` is about where an ant can
walk and the effect is large, separated and mechanically direct — a 32-row
vertical face against a 2-row step. A claim that the sloped pond is *better for
the colony* would not survive it, and is not made.

---

## 2. Phase 1 — the margin

**What the player gets:** the pond stops being a dead blue rectangle. Reeds
stand in the shallows, their stems crossing the waterline, their reflections
the first thing in the box that is *of* the water.

**The one predicate.** `plant::growable` (`plant.rs:349`) refuses `Liquid` on
both the shoot and root paths. It must allow a shoot to extend into a `Liquid`
**for a species that has opted in**, and for no other species.

**The opt-in goes on the species, tested at the call site** — `CLAUDE.md`'s
*guard hot-path work at the call site that already has the data*. A
`SpeciesDef` field (`submerged_shoot: bool`, default false) read where
`growable` already holds the species, never a `id_of("reed")` string compare in
the sweep. This is also what keeps the change from re-deriving every existing
species' constants: a per-species opt-in is a no-op for all twenty shipped
plants by construction, and that is the whole reason it must be per-species
rather than a global relaxation.

**The bed rule, which is §1.1's constraint and is not optional.** A rooted
aquatic plant needs sediment; sediment drains the pond in under 4,000 frames.
The fix is not a new mechanism — it is to place the sediment **already
saturated**. `Placement::Fill` lays soil at `SOIL_FIELD_CAPACITY` (620);
`SOIL_SATURATED` is 1,000, and a saturated cell has no room to drink. So Phase
1 needs either a `Fill` that takes an explicit wetness, or a distinct
`"sediment"` placement — **one line in `scenario.rs`, and it is the cheapest
part of this phase.** Verify by re-running §1.1's constancy table on the
sediment bed: if fill is not flat to the unit at 40,000 frames, the sediment is
still drinking and nothing downstream is worth measuring.

**Appearance is not optional either**, and this is where the phase most likely
fails quietly. `plant-appearance-design.md` §5: three architectural levers
fired, all measured, and the owner saw no change, because every plant in the
world drew from one four-brown palette and one four-green one. **A reed built
from `wood` and `leaf` will read as a twig standing in a puddle.** It needs its
own materials and its own palette band — which is exactly the case that
document says a new material is *warranted* for, since a reed's physics differ
(it does not hold itself up the same way and it lives wet).

**The height rule is deferred to Phase 4, deliberately.** The research's
finding — that the turgor ceiling is Lockhart's equation charging the
gravitational cost of lifting water, so lift should only be charged above the
waterline — is real and is the most elegant thing in the whole area. It is also
a change to the one gate in the plant system built from geometry rather than
resource state, and it *removes a bound without supplying a replacement* (the
first kelp is 300 cells tall). An emergent reed rooted in shallow sediment
does not need it: it is short by construction. **Do not take the elegant change
until there is a plant that needs it.**

---

## 3. Phase 2 — the body (E9)

The owner's ruling, 2026-08-29: drown, float and swim all reachable, heritable,
with the float limit **mechanical rather than genetic**.

**Float is already built** and nobody had written it down: `land_afloat`
(`creature.rs:9967`) — *"Weightless and not flying is standing on water, not
flying over it"* — sets `creature_stats.landed_afloat` and ships with
`LAND_AFLOAT=0` as a runtime A/B. So this phase is **drown and swim**, and it
starts with a control arm already in the binary.

**The mechanical limit already has a dial, and one comment is wrong about it.**
`creature.rs:8943` says *"every cell is density 1.0, which every creature
material currently is"*. `beetle.ron` is **1.2** and `corpse.ron` is 1.2, so the
prey hangs at the surface, the predator sinks and the dead sink — authored by
accident and better than anything a design would have invented. **Fix that
comment in whichever brief lands first**; it is load-bearing for the rest of
this section.

The fork, named rather than resolved:

- **density-as-material** — one authored number per species, not evolvable.
  Satisfies "mechanical not genetic" as written, and already works today.
- **density-as-trait** — slot 14 of `CREATURE_TRAITS` (14 used of a 64-slot
  reserve; appending is lawful and guarded by
  `appending_a_slot_on_any_axis_moves_no_existing_weight`). Evolvable, which is
  the other half of the ruling — but then the mechanical limit has to mean
  something else, most naturally *body size sets a bound the gene cannot
  cross*.

**Drowning must be a clock, not a predicate** — `CLAUDE.md` law 1, an outcome
is a distribution. A consecutive-submerged-frames counter, with `flight.stall`
as the working pattern in the same file. And drowning is worth more than it
looks: it is the only mechanism yet proposed that puts **carrion** in the
world, which is measured as the reason the survival-versus-`gut_bias` curve has
one hump instead of two.

**Two costs that must be stated before the phase starts, not discovered in
it.**

1. **A new `BrainOutput` grows `live_slots`, which re-derives every species'
   `mutation_rate`.** The precedent is exact and recorded: `live_slots` 846 →
   870 meant `main` before `f9dd3295` was not a valid control arm. If swim is a
   new output, **every species file changes in the same commit** and every
   baseline taken before it is void. Consider instead whether swimming is a
   *movement mode* selected by the existing `Move` output when the body is in
   liquid — no new slot, no re-derivation.
2. **The guard that encodes the current design must be made to fail.**
   `a_weightless_body_is_put_down_on_water_unless_it_is_flying` is a guard whose
   name is the decision. A swim verb adds a third state and the guard must go
   red for the replacement, or it is a superseded test that keeps passing while
   testing nothing.

**And the orientation trap.** `dead-ends.md:1106` (decision D1) rejected
rotating a body through the grid — aliasing, self-overlap, cells appearing from
nowhere — and its own re-test clause says arbitrary-orientation creatures
reopen it. **A swimmer keeps the canonical up and mirrors like everything
else.** In a side-view pixel game a fish that stays upright is not a
compromise; it is what the view wants.

---

## 4. Phase 3 — the gradient, gated on a card

**Do not start this before card `20260914T030227009Z-0c5076` is answered.** If
the owner does not want a dark pool, the phase goes and Phases 1–2 are
unaffected.

If it is wanted: `FieldTile::transmission` (`field.rs:308-347`) is already a
**Beer–Lambert per-column depth model**, built for foliage, and Beer–Lambert is
the law that governs light in water. Water is simply absent from the predicate
in `rebuild_blocked` (`field.rs:2999`), which counts only `Solid` and `Plant`.

Two conditions travel with it, and the second is the one that would break the
box:

- **Per-material extinction.** `column_depth` is a plain `u8` count today; a
  cell of water must not attenuate like a cell of wood. `dead-ends.md:224`
  already names the fix site for the parallel buried-plant case: opacity as a
  material property.
- **It must be `transmission`, never `blocked`.** A blocked block is skipped
  entirely by `step_diffusion`, which is the measured cause of the bug at
  `field.rs:815` — **96.8% of grass cells read field moisture exactly 0.000 at
  every wetness level**, because the presence of fuel in a block is what makes
  the block read bone dry. Water is the *moisture source*. Marking a pond
  `blocked` would make the pond read as having no water in it.

### 4.1 The resolution problem, taken before writing the phase

**Checked here rather than discovered later**, per *check that a planned step
can demonstrate itself before promising it will*. Two verified constants
decide how much gradient the field can express at all:

- `FIELD_SCALE = 16` (`field.rs:48`) — **one transmission sample per 16 CA
  rows**.
- `COLUMN_TRANSMISSION` has exactly **17 entries**, 0..=16 opaque cells, and
  its last entry *is* `SKY_TRANSMISSION` (`field.rs:2315`) — so a field block
  whose columns are full of opaque material is already at the table's floor.

Two consequences, and both change the phase:

1. **Water cannot count as a whole opaque cell.** If it did, the table would
   saturate after 16 rows and a pond would go from full light to the floor in
   one step. Water needs a **fractional extinction coefficient**, which means
   `column_depth` — a plain `u8` count today — has to become a fractional
   accumulator or the coefficient has to scale the table index. That is real
   work beyond "add `Liquid` to the predicate", and the research understated
   it.
2. **`the_pond.ron` is too shallow to show a ramp, and so is the lab.** 28 rows
   is **1.75 field cells**. A five-step gradient needs roughly 80 rows of
   water — a quarter of the 320-row lab world. So **the depth gradient is
   substantially an outdoor-world feature**, where a lake can be deep, and the
   lab can only ever show two or three bands of it. That inverts this plan's
   "lab as the proving ground" framing *for this phase only*; Phases 1 and 2
   are unaffected.

Neither of these is a reason not to do it — a two-band pool is still a pool
with a dark bottom, which is what the card asks about. They are reasons the
phase is bigger than one predicate, and they should be in front of the owner
before the card is answered, not after.

**What was tried and did not settle it:** `filmstrip scene=forest
channel=light` shows the mechanism working *laterally* — trunk shadows are
visible as distinct vertical bands — but everything below the surface
saturates to one value, so it does not demonstrate a depth ramp. **OWED**: a
render of a deep column under fractional extinction, which cannot be taken
until the coefficient exists.

**The height rule lands here, not in Phase 1**, because depth and plant height
are the same gradient: charge turgor lift only above the waterline, and how
deep the water is decides how tall a plant can grow. The bound stays finite —
water depth plus whatever the plant can lift in air — so
`every_growing_species_has_a_height_ceiling_to_be_charged_against` is satisfied
rather than weakened.

---

## 5. Phase 4 — the loop, sketch only

Not specified in detail because Phases 1–3 will change what it should be. The
shape: things that fall in sink (`windfall` 1.05, `corpse` 1.2) and accumulate
on the pond floor; an animal that feeds on drift scores **position held**
rather than distance covered, which is the first income channel in this engine
that is not a function of locomotion — and locomotion is **5,599 J of the
colony's 10,796 J burn, 52%**. A bubble is the oxygen model without a scalar:
`is_displaceable` is already `Liquid | Gas` **(measured**, `material.rs:219`**)**,
so a gas cell should rise through a water column with no new mechanism, and
that dodges the `Cell::aux` tagged-union trap entirely.

---

## 6. Build order, ownership and collisions

**Order:** Phase 0 (landed) → **1a** (bed rule) → **1b** (predicate + species)
→ **2** (E9) → **3** (gated) → 4.

| lane | owns | disjoint from |
|---|---|---|
| **1a — sediment** | `assets/lab_scenarios/*.ron` **only** — §1.6 removed the `scenario.rs` change | everything; lands first and alone |
| **1b — the reed** | `src/sim/plant.rs`, `src/sim/organism.rs` (one `SpeciesDef` field), `assets/species/reed.ron`, two new materials, one `include_str!` line | **2** entirely — run them together |
| **2 — E9** | `src/sim/creature.rs`, `src/sim/organism.rs` (trait slot) | **1b** except `organism.rs`; see below |
| **3 — depth** | `src/sim/field.rs`, `assets/materials/*.ron` | both |

**1b and 2 both touch `organism.rs`.** `SpeciesDef` and `CREATURE_TRAITS` are
far apart in the file and the merge is mechanical, but `src/sim/world.rs` /
`README.md` / `Reports/README.md` are the contested rows here (103, 103 and 103
landings) — **land each quickly rather than holding a large diff**.

**One harness was not ready and the briefs must not pretend otherwise.**
Checked 2026-09-14: `labforage`, `labshot`, `waterstand`, `labgif`, `labbatch`
and `soil_drawdown` all accept `scenario=`, so every measurement named above is
executable today. **`creature_arena` and `labstats` did not** —
`creature_arena` built its `LabBox` from flags (`founders`, `ants`,
`predators`, …) and could not be pointed at a pond at all. Since it is the
teeth test, and the teeth test is what Phase 2 ships or does not ship on,
**adding `scenario=` to `creature_arena` is a prerequisite of Phase 2, not a
nicety** — and it was the one harness change this plan asks for.

> **Built, 2026-09-14 — and the flag was the small half.** `creature_arena`
> now takes `scenario=`, with the sweep's `seeds=1..=N` stamped onto the
> scenario's own `bed.seed` per seed rather than onto the `LabBox` it hands
> back, which is where `labshot` records the same knob silently reaching
> nothing.
>
> **The real work was that arms could only be handed out at frame 0.** Every
> bed the flags can build has its colony standing when `build` returns, so
> assignment was a straight line after it — and every bed written since the
> owner's 2026-09-09 correction founds its ants from the **timeline**
> instead, `the_pond_shore.ron` at frame 6,000. A frame-0 scan of one of
> those finds **zero** animals and trips the `>= 8` assert before the run
> starts. Assignment is now *whenever the founders arrive*, and the set to
> label is **every live animal of the species whose lineage is unlabelled** —
> exact rather than heuristic, because `Origin::Bud` carries a lineage
> through unchanged, so a descendant already carries a label and only
> `found_colony_of` mints a new one.
>
> **And the second-order break is the one worth carrying into any brief
> here.** `idle_life` — the founding grant, whose 12,000 frames produced the
> finding that governs every race this harness has ever run — starts at
> *founding*, so the harness's own interpretability check (`frames >=
> idle_life`) silently became wrong by the founding frame the moment a
> colony could arrive late. A bed founding at 6,000 and run for 13,000 gives
> its colony 7,000 against a 12,000-frame grant and would have reported the
> horizon as sufficient. It now measures from founding and prints both
> numbers. **Any check written against "the run" means "the run after the
> thing existed" as soon as anything can start late** — which is the shape of
> every timeline bed in §1.7, not a fact about this harness.
>
> **So a Phase 2 race on the pond bed needs `frames >= 18000`**, not the
> 12,000 the grant alone suggests: 6,000 to found plus the full grant after
> it. Three guards refuse the ways this reads as a result when it is not — a
> run too short to reach the founding, a scenario founding a species the race
> does not read, and a bed that seats nobody — because each of them otherwise
> prints a table of zeros, which is indistinguishable from a bed where
> neither arm survived. With no `scenario=` every column is identical to the
> pre-change binary.

**Every brief carries the same cost fork:** build it, or write the finding and
stop; never a half-built fix. **Every brief's creature card is a moving
sequence** (`labgif`, or `filmstrip gif=1`), never a still — an ant is two dark
cells at play zoom and is picked out of dark soil by *moving*. **Each brief measures in the bed
that contains its defect** (§1.7): Phase 1 in `the_pond_sediment.ron`, Phase 2
in `the_pond_shore.ron`, ecology questions in `the_pond_stocked.ron` — 6+
seeds, 90,000 frames, `RAYON_NUM_THREADS` pinned. **A brief measured in the
wrong pond measures nothing and cannot tell that from a broken verb.**

---

## 7. The briefs

### Brief A0 — the sediment bed (do this first; it is not optional)

**Reduced to a data file by §1.6** — it was specified as a `scenario.rs`
change and does not need one.

*Owns:* `assets/lab_scenarios/`.
*Read first:* §1.5 and §1.6 here.
*Build:* `the_pond_sediment.ron` — `the_pond.ron` with a four-row soil layer
laid **on top of the stone floor**, inside the basin, and the water shortened
to match. The stone is what seals the sediment from the bed's own column; take
it away and §1.5 happens.
*Counters:* total fill at 0 / 5,000 / 20,000.
*Positive control:* the same bed with the stone floor replaced by soil must
drain to zero by frame 4,000, reproducing §1.5.
*Ships if:* fill is flat to the unit from frame 5,000 on, having lost about
6% — and **check the loss against 152 x depth x 380 before believing it**, since
that arithmetic predicted the measured number to 0.005%.
*Guard:* adding a scenario enrols it in
`every_shipped_scenario_loads_builds_and_places_what_it_says`, so run
`cargo test --release --lib lab::scenario` specifically.
*Cost fork:* if a deeper sediment layer is wanted for rooting and the loss
becomes visible, the fork is a wetness-carrying `Fill` after all — that is the
change this brief originally specified, and §1.6 is the reason not to reach for
it first.

### Brief A1 — the reed

*Owns:* `src/sim/plant.rs`, `src/sim/organism.rs` (`SpeciesDef` only),
`assets/species/reed.ron`, `assets/materials/` (two new), one `include_str!`
line in `organism.rs`'s registry.
*Read first:* §2 here; `plant-appearance-design.md` §5 and §3;
`dead-ends.md` line **845** (germination in mid-air was the first thing the
owner ever reported) and line **1843** (the substrate gates were a null on what
he was actually looking at).
*Build:* `SpeciesDef::submerged_shoot: bool` (default false), tested inside
`growable` where the species is already in hand. A `reed.ron` cut from
`grass.ron` — grass is the proof that a non-woody plant photosynthesising from
`MatureBody` works end to end — with its own shoot and leaf materials and a
disjoint palette band.
*Build trap, and it is the asymmetry that made Phase 0 cheap:* **lab scenarios
load from disk at runtime; species and materials do not.** `the_pond.ron` and
its siblings worked the moment they were written, with no rebuild. `reed.ron`
will not — species are `include_str!`'d into the binary, so a new one needs its
registry line *and* a rebuild, and a harness run against a stale binary will
report the reed doing nothing while the file on disk is perfect. Identical
output across a change that must have moved something is the tell
(`.claude/rules/assets.md`).
*Constants to re-derive:* none for existing species, **by construction**, and
that is the reason for the per-species flag. State that in the PR body so the
next session does not re-audit it.
*Counters:* reed cells standing **below the waterline** (it fired) and standing
reed count at 90,000 (there is something in the picture). A null here is
indistinguishable from a probe that never ran, so print both.
*Positive control:* an arm at `submerged_shoot: false` must take
below-waterline cells to exactly zero while the reed still grows on the bank.
*Measurement:* `labshot scenario=the_pond_sediment`, 6 seeds, 90,000 frames.
*Card:* the pond with reeds in it against the pond without, both at 90,000,
standing-reed count in `meta`.
*Cost fork:* if the reed grows but reads as a twig in a puddle, **that is the
appearance finding and it is worth more than the mechanism** — write it, with
the render, and stop. Do not tune the growth rules to fix a palette problem.

### Brief A2 — E9

*Owns:* `src/sim/creature.rs`, `src/sim/organism.rs` (`CREATURE_TRAITS`).
*Read first:* §3 here in full, including both stated costs.
*Build:* in order — (i) fix the stale density comment at `creature.rs:8943`;
(ii) open `move_cost` for `Liquid` at a price; (iii) the drown clock;
(iv) swim, **preferring a movement mode over a new `BrainOutput`** for the
`live_slots` reason.
*Counters:* submerged ticks, drownings, swim moves; and from the far side,
**cells of food eaten while submerged** — a "it fired" counter without an
effect counter is how a clean negative turned out to be 23 swings removing 0
cells.
*Positive control:* `LAND_AFLOAT=0` restores the old defect; the new drown
clock at an infinite threshold must reproduce today's behaviour exactly.
*Guard:* `a_weightless_body_is_put_down_on_water_unless_it_is_flying` **must be
made to fail for the replacement** — verify red before shipping green.
*Cost fork:* if swim needs a new brain output after all, that is a
whole-corpus `mutation_rate` re-derivation and it is **its own PR**, not a
rider on this one.

---

## 8. What would stop each phase

- **Phase 1** stops if saturated sediment still drains the pond (A0's fork), or
  if a reed reads as a twig (A1's fork).
- **Phase 2** does **not** stop on §1.4 — the sweep came back showing water
  costs the colony nothing, and showing the bed cannot discriminate either way.
  What that changes is how Phase 2 is judged: **not by colony size**, which
  collapses to one or two ants in every arm, but by the verb's own counters and
  especially by **food eaten while submerged**. It stops if that comes back
  zero — that is the hop again, caught early.
- **Phase 3** stops on the owner's card, or if the field's 16-cell resolution
  makes a 28-row pond exactly two bands of light — which is an
  **OWED** measurement and should be taken before any of Phase 3 is written.
- **Phase 4** is not specified enough to stop.

---

## 9. What this deliberately does not build

No dissolved oxygen, salinity or water-column nutrient scalar — `Cell::aux` is
a tagged union whose tag is material data, and the `nest.ron` entry is the
worked case where **every counter read as a clean win** and the design was
still wrong. No current or advection. No free-floating algae. No ice lid. No
change to the gnome's swimming, which is the one aquatic thing in the repo that
has actually been played and already carries an owner complaint. No outdoor
worldgen work: ponds already generate there, and the lab is the proving ground
because it is the only game with a control arm.
