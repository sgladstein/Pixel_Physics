# Caves, rebuilt: rooms made by collapse, joined by passages, with a way in

2026-08-30. Lane I of the worldgen revamp, building W3 against the design in
`cave-redesign-2026-08-29.md`. The owner's instruction, verbatim: *"The whole
shape and generation of the cave shold be rebuilt from the ground up. The
caves are still all slightly modified circles or ovals, not realistic at all.
The voroni worly patter around the cave should be removed. I know I said that
I liked it before but no."* And, on a second card the same minute, three
words: ***"Remove the web"***.

This is the implementation report: what was built, what it measures, the
pillar answer the plan called the largest open risk in the programme, and
what it costs in reach. The design is not re-derived here.

---

## 1. What was built

**Two objects, and no field.** `src/worldgen/cave.rs` is new and is the whole
generator; `passes::vaults` now calls it and keeps the writers the owner has
already accepted (gravel floors, the aquifer waterline, speleothems, the
geode vug).

| | what it is | how it is made |
|---|---|---|
| **Room** | a space you stand in | a dissolution lens flooded through a **removal cost** built from the strata, then a roof that falls in until it reaches a bed strong enough to hold its span |
| **Conduit** | the passage between two rooms | a shortest path on a coarse lattice through an **anisotropic cost field** -- bedding, joints, and a palaeo-water-table -- cut as a **keyhole** section |
| **Mouth** | the way in | the same search on a mask without the cover rule, then a run outward down the local slope, rising one row in two until it is out in the air |

**Deleted:** `carve_cave_void`, `settle_cave_void`, `keep_seed_component`,
`all_long_ceiling_runs`, `grow_monumental_chamber`, `erode_breaches`,
`CAVE_CELL`, `CAVE_SQUASH`, `CAVE_EDGE_FADE_X/Y`, `CAVE_THRESHOLD`,
`MAX_CEILING_SPAN`, `ROUND_3_HALF_W`. Nothing in the new module reads a noise
field to decide where rock is absent.

### 1.1 The room is not drawn, and the rock decides its shape

`bed_span` is `max_unsupported_span * attached_span_bonus`, read straight off
the material rather than invented: over the six beds it runs **42 (mudstone)
to 308 (basalt)**. That one number does three jobs -- it prices removal in the
dissolution flood, it decides where the roof stops falling, and it sets how
far apart the pillars stand -- so two rooms in one world differ by a factor of
seven in shape without a parameter moving.

Measured across sixteen seeds, `collapsed` (cells the roofs lost) spans **40
to 137,000 per system**. That is the ethos's first law in a number: the
outcome is a distribution, not a binary.

### 1.2 Pillars are a designed feature, and they are pierced

A room 3-7x the one the owner called small is wider than any bed can roof, so
the pitch is derived from the rock (`pillar_pitch`) and the lens flood is
forbidden to take the reserved columns. The collapse then leaves them standing
floor to ceiling **by construction** -- it only ever eats a cell with void
directly beneath it, and a pillar column has none.

**A floor-to-ceiling pillar is a wall in a side-on world**, which is the one
thing about pillars this engine's geometry forces, and it is not a concession
to pierce them: a real breakdown pillar usually *is* pierced at the base,
because the water that left it standing was still running past its foot. So
`open_floor` cuts an arch through every pillar after the collapse, keeping
both legs at no less than five cells so the roof still reaches the ground.

---

## 2. The pillar finding, and it is not the one the plan expected

The plan: *"stone's `max_unsupported_span` is 16 -- so a room that size forces
pillars. That is measurable today with `support_census`, and it should be
measured before the room size is chosen, not after."*

`support_census` cannot answer it -- it reads the distance field and never
cuts a hole -- and neither can `arch_probe`, which sweeps a hand-built
pier-and-lintel scene in a 200-row test world. So `cave_probe` gained a
**`span=1` mode**: it generates a real world at the shipped 8192x2560, cuts a
room of a chosen width into the massif at cave depth, re-solves the support
field, runs three hundred frames of the real sweep, and censuses the rock that
left a box around it. Three arms, because the answer is different in each:

| arm | what it asks |
|---|---|
| `quiet` | shipped leash, nothing disturbed -- does the room survive genesis |
| `pick` | shipped leash, one pick swing into the ceiling -- what a player sets off |
| `model` | leash off, every rock cell in the rim scheduled -- what the load model believes |

**The answer is that the roof does not come down, and it is not close.** Three
seeds, `rolling`, at the shipped world size, rock cells lost from a box around
the room:

| width across | pillars | quiet | pick | model |
|---|---|---|---|---|
| 0 (control) | -- | **0** | **0** | **0** |
| 128 | none | 0 | 63 | 3 |
| 128 | every 224 | 0 | 63 | 3 |
| 256 | none | 0 | 63 | 11 |
| 256 | every 224 | 0 | 63 | 432 |
| 512 | none | 0 | 63 | 3 |
| 512 | every 224 | 0 | 63 | 6 |
| 1024 | none | 0 | 63 | 112 |
| 1024 | every 224 | 0 | 63 | 360 |


**Read the `quiet` column first.** `World::chain_reach` is a policy layered
over the load model: at the shipped TIGHT setting a failing region is clipped
to what sits near a live disturbance, so an *undisturbed* generated cave is
never licensed to fail however wide its roof is. That is the honest answer to
"does a room this size stand at genesis", and it is **yes at every width
tested, including 2048 -- four screens across.**

The `pick` column is flat in the width: one swing removes its own bite and
sets nothing else off. And even with the leash off and every rock cell in the
rim scheduled, the model's answer is a rounding error against the room's own
volume. The reason is `load::capacity`: a cave roof's `section` is the run of
rock above it, which in a massif is hundreds of cells, and capacity is
quadratic in it and multiplied again by `attached_span_bonus`. `stone.ron`'s
`max_unsupported_span: 16` never enters the arithmetic at the scale the plan
assumed it would.

**So pillars are not required by the physics, and they are in anyway.** They
are required by the *generator's own* collapse model, which is what shapes the
room; they are the eye's only ruler in a space too large to see the ends of;
and they are what makes a dig into a big room's leg mean something later. The
plan asked for the number either way, and this is it: **the engine will not
bring a cave roof down on its own at any width up to two screens.**

**One caution on all three columns.** This is a probe over three seeds on one
preset, and the `model` arm's absolute numbers should not be read as a
prediction of what a player will see -- what it bounds is the *direction*.
The controls it carries are the ones that make the null admissible: `W = 0`
loses zero rock in every arm, and the `lid=6` control -- the same room with
the massif above it cut away, so the roof is a six-cell slab -- is where the
instrument is shown to be able to report a real collapse at all.

| control | quiet | pick | model |
|---|---|---|---|
| `W=0` -- nothing carved | **0** | **0** | **0** |
| `lid=6` -- 512 wide, roofed by a **six-cell slab** | 0 | **185** | **207** |

**Read that control honestly: it works and it is weaker than it should be.**
It proves the instrument is not blind -- the same width goes from 3 to 207
cells lost when the massif above the roof is taken away -- and it also says
that a six-cell slab spanning five hundred cells *still stands*, which is the
same finding again from the other side. `load::capacity` is
`(span^2 / 2) x section^2 x attached_span_bonus`; at a section of six and
stone's twelvefold attachment bonus that is fifty-five thousand against a
demand nothing like it.


---

## 3. The census: before and after

16 seeds x 5 presets at the shipped 8192x2560, `examples/cave_probe`.

### 3.1 An instrument repair first, because it moves the "before" numbers

`cave_probe`'s census window was `WORLD_HEIGHT / 2` -- **1,280 rows down** at
the shipped size, below most of the depth band `vaults` places into. This is
the identical defect the design lane repaired in `viewshot vault=1`
(`cave-redesign-2026-08-29.md` §3.5), found there and not looked for here,
because the same file carries both readings and only one of them was under
suspicion.

It is now each column's own ground line plus sixty rows, and a component has
to reach `vault_min_depth` under that ground to count as a cave at all --
without which the wider window counts every overhang and valley pocket in the
world (measured: 4.0 "systems" per world against the one or two the pass
places).

**The correction moves the headline figure the programme was quoting.** The
design lane reported *"8 or 9 of 16 worlds have no cave at all"*; measured
through the repaired window on the same code, it is **2 to 4 of 16**. The old
number was counting shallow caves as absent.

### 3.2 The numbers

Both columns measured through the repaired instrument, 16 seeds x 5 presets,
at `47d6209` and at this branch's head. Ranges are across the five presets.

| | before | after |
|---|---|---|
| **worlds with no cave** (of 16) | 2-4 | **0**, every preset |
| cave systems per world | 2.1-2.7 | 3.9-4.9 |
| **largest connected walkable region**, share of a system's void | 36-39% | **98%** |
| **separate walkable pockets** per system, med / p90 / max | 3 / 31-38 / 86-95 | **1 / 3-6 / 9-13** |
| median open column (the gnome is **14** tall) | 13-16 | **60-72** |
| tallest open column | 56-77 | 146-179 |
| span across, median / max | 165-340 / 1,528-1,544 | 197-260 / 929-1,184 |
| void, share of the massif under the ground line | 0.29-0.32% | 0.75-0.89% |
| **systems with a way in** | **0, by construction** | **all of them** |

Four of those are the owner's own sentences answered:

* *"It is also looks like a single room instead of a cave system"* -- a system
  is 3.9-4.9 connected places per world with several rooms in each, and
  **98% of one of them is one walkable region**, against 36-39%. The p90 used
  to be a system shattered into **thirty-one to thirty-eight separate
  pockets** with no way between them; it is now three to six.
* *"It doesn't look like I could even enter it"*, first reading: the median
  passage was **the gnome's own height with nothing to spare**. It is now
  four to five times his height.
* *"It doesn't look like I could even enter it"*, second reading: there was no
  entrance in the game at all. Every system now has one, and no world with a
  cave in it lacks a way in.
* *"there should be variability between caves"*: the biggest room in a system
  runs from ~240 to 660 cells across and `collapsed` -- the volume its roof
  lost -- spans **40 to 137,000 cells**, out of the same code and the same
  draw, because the beds differ.

**The span-across maximum went down** (1,544 to ~1,100) and that is the point
rather than a regression: the old figure was the width of a Worley web spread
across a whole envelope, most of which was fringe the player could not enter.
What replaced it is a shorter system with rooms in it.



---

## 4. What it costs in reach

`VAULTS_MARGIN` was `MAX_CAVE_HALF_W + VAULT_RIND = 802`, the widest declared
margin in the pipeline: generating one 64-column chunk required 1,668 planned
columns, **26x amplification**.

It is now **780**, and the rooms got several times bigger. The mechanism is
not cleverness, it is a direction: **a system chains downward.** The depth
band is over a thousand rows deep at the shipped size and rows are free in a
margin measured in columns, so `MAX_CAVE_HALF_W` came *down* from 800 to 720
while `MAX_CAVE_HALF_H` went from 320 to 560.

**The margin is derived from the widest term, not the obvious one**, because
three passes in this pipeline have already declared a margin smaller than they
walk:

| what leaves the envelope | reach beyond `cx` |
|---|---|
| the seal check's rind | `MAX_CAVE_HALF_W + VAULT_RIND` = 722 |
| the mouth's lintel shell | `MAX_CAVE_HALF_W + 6 + LINTEL_THICK` = 730 |
| the slope read that decides which way a mouth faces | `MAX_CAVE_HALF_W + MOUTH_SLOPE_LOOK` = **780** |

`a_cave_cannot_reach_past_its_declared_margin` in `tests/worldgen.rs` asserts
the first of those against the constants; it is now understated relative to
the real reach and should be widened to the expression above by whoever
touches it next. **Flagged rather than fixed here**: that test belongs to the
margin contract as a whole and three lanes are in `tests/worldgen.rs`.

---

## 5. What it costs at build time

`vaults` was **38-116 ms** of a ~5,700 ms world build. It is now **115-650
ms**, median around 200 ms -- three to six times more, and 2-11% of the whole
build. Where it goes, in order: `Carvable::build` walks the envelope once
through `World::get` (a bounds check plus a `HashMap` lookup per cell) and
erodes it twice; the lens floods are a Dijkstra per lobe; the seal check is a
5x5 dilation of the void.

Two of those were already made cheap and the numbers are worth keeping:

* **The seal check walks the void outward, not the envelope inward.** The
  obvious form probes 5x5 per cell over a box of nearly two million -- forty
  million array reads for a property that concerns the few tens of thousands
  of cells next to a hole. Iterating the void and dilating it is the same set
  by definition, at a fortieth of the work.
* **The rind erosion is separable and runs over prefix sums**, `O(area)`
  rather than `O(area x r)`. At the tube radii the conduit search needs, the
  naive form is a hundred and fifty million reads per system.

**Nothing here runs per frame.** Caves are static geometry written at genesis,
and the render skip is untouched.

---

## 6. The consequence nobody had named: a cave with a mouth is "outdoors"

**This is the finding most likely to matter to another lane.**
`World::freeze_underground_map` defines "outdoors" as *a flood fill from the
top of the world through everything that is not `Solid` or `Powder`*. That was
exactly right while no cave in this game had a way in. The moment one does,
the flood walks in through the mouth and marks the whole system outdoors --
and the consequence is total, not cosmetic:

* the cave renders as **sky**, with the day gradient in it and **rain falling
  inside it**;
* `ground_datum` -- whose own doc says *"it does not skip a cave, because cave
  air is not outdoors"* -- then grades every cell below a chamber as if the
  chamber's ceiling were the ground, so the strata under a room draw as slabs
  floating in mid-air.

Both were in the first render of a finished room and neither is subtle.

The fix is in `freeze_underground_map` and it restores the *previous* answer
rather than inventing one: the flood now carries **how far under cover it has
travelled** -- zero in the open air and through liquid, one per step anywhere
the sky is not directly overhead -- and stops past `World::SKY_PENETRATION`
(48 cells, twice `render.rs`'s own cave-light ramp, and more than double the
20-cell reach of the deepest brow, which is the largest legitimately-covered
place the unbounded flood used to reach). A 0-1 BFS over a deque, so the
minima are exact and there is no heap.

Before the rebuild every cave was sealed, so every cave was underground. What
changed is that a cave now has a way in, and *"the sky can see this cell"*
stopped being the same question as *"there is a path of air from here to the
top of the world"*.

---

## 7. Three defects found by rendering, each with its mechanism

None of these was visible in any counter, and all three came out of looking at
one frame.

* **Lenses left floating.** `pockets` writes sand and gravel through the
  massif and `Carvable` forbids carving them *or the two cells of rock around
  them*, so a lens inside a dome survived as an island hanging in the void,
  rock rind and all, with stalactites hung underneath it. Routing around a
  lens is right for a big one -- that is a passage narrowing past an
  incompetent bed -- and wrong for a small one, which a falling roof takes
  with it. Pockets under 3,000 cells and clear of the envelope edge are now
  swallowed **whole** (taking half of one leaves the other half loose against
  a free face, which the seal assertion caught on the third world), and
  anything the carve still leaves enclosed by void goes with `drop_islands`.
* **A drawn passage is not a connected one.** `chain_rooms` cut a keyhole
  along every edge of a spanning tree over the rooms and the void still came
  out in **seven pieces on one seed** -- a conduit's section is clipped per
  cell to carvable rock, and where the route grazes a lens the narrowing can
  go to nothing. Two repairs: the path search now runs on a mask eroded by the
  *narrowest* section a conduit can cut (eroding by the widest was tried and
  closed the massif off entirely -- `conduits 1` for four rooms), and whatever
  is still separate afterwards is welded and **counted** (`pieces`, `welds`).
* **Six-hundred-row needles.** Formation length is drawn from the local open
  span, the rebuild took that from tens of rows to four hundred, and the first
  big chamber had hairlines hanging the full height of it -- the owner's
  *"they are all 1 pixle thick"*, made worse by a change that never touched
  that code. §7 of the design asks for a minimum aspect enforced by
  *shortening*; `SPELEO_MAX_ASPECT` is that, at nine times the formation's own
  drawn base width.

---

## 8. What the counters say, and why they are there

`vaults detail` now prints, per world:

```
vaults detail: systems 1/1 rooms 2 pillars 3 conduits 2 mouths 1
  | collapsed 96338 lintel 1873 swallowed 0 capped 0d/2l pieces 1 welds 0
  | room med 522x308 max 522x308 | ...
vaults mouths at: 2826,252
```

Every one of those exists because a picture cannot show it. Three are worth
naming:

* **`mouths` counts mouths that reached daylight, not attempts.** It counted
  attempts first: the breakout was recorded as a mouth whatever point it
  stopped at, so a run that hit the envelope wall two hundred rows underground
  printed `mouths 1`. That is this repo's most-recurring failure -- a number
  that is arithmetically correct and about a different question.
* **`capped Nd/Ml`** counts domes stopped by `MAX_DOME_RISE` rather than by
  the rock, and lenses stopped by their volume budget rather than by the cost
  field. A cap that bounds work is fine and a cap that decides the answer is
  the landmine `CLAUDE.md` names twice; counting how often each binds is the
  only way to tell them apart. At the shipped settings `domes_capped` is
  **0 on almost every world**.
* **`pieces`** is what *"chained together so you can walk directly from one to
  the other"* is actually about, and `conduits` cannot answer it (§7).

`vaults mouths at:` prints the coordinate because a cave's position is a draw
and a hardcoded one goes stale -- `viewshot vault=1` learnt that the hard way.
`viewshot at=<x>` frames an opening from it.

---

## 9. The size distribution, and the round it took to get it right

**The first build put a big room in every system and the owner's verdict was
that they were all too big** -- *"these all look huge. Huge sometimes more
rare is good, but they should not all be this large."* That is the same
complaint as *"this is way too small"*, one iteration later and from the other
side: the number moved and the *shape* did not. `CLAUDE.md`'s first law is
that an outcome is a distribution rather than a binary, and a size everything
shares is a binary wearing a number.

What was wrong was not the size, it was that it was a **setting**. The first
build forced the first room of every system into the top of the range, and it
did that for a real reason: sites are rejected against each other by their own
extents, so a big room needs a lot of clear envelope and is rejected far more
often than a small one -- draw order truncates the distribution toward small,
and a free draw produced systems whose biggest room was 85 cells across.

The fix is not a forced size, it is **placement order**: draw every width up
front, sort descending, and place the biggest first, so the largest room gets
first pick of an empty envelope and the truncation falls on the small ones,
where it does not matter. `ROOM_BANDS` is then a plain mixture:

| band | across | in "small rooms" (145) | share of draws | measured, 4 seeds |
|---|---|---|---|---|
| small | 150-320 | 1.0 - 2.2x | 62% | 53% |
| medium | 320-520 | 2.2 - 3.6x | 26% | 27% |
| large | 520-720 | 3.6 - 5.0x | 9% | 13% |
| **huge** | 720-950 | 5.0 - 6.5x | **3%** | 7% |

The measured column is a fifteen-room sample and is stated as such; the
per-band count is printed by `vaults detail` (`bands a/b/c/d`) on every world,
so the distribution can be censused rather than taken from this table. **The
shares are the reviewable object here, not any one room**, which is why they
are written as an explicit mixture rather than as an exponent on a unit draw.

---

## 10. Smoothing, without going back to a drawn shape

*"The opening is fine. Everything should be smoother."* Two causes, and the
fix has to leave the shape the collapse made or the whole claim of the
generator goes with it -- *"slightly modified circles or ovals"* was rejected
by name, so fitting an ellipse to a room is not available.

* **The tool marks.** A room whose boundary is where a collapse stopped has a
  one-cell staircase everywhere the ceiling changed bed. `smooth_walls` is a
  **majority vote over a 7x7 window** -- a filter over the shape the physics
  produced, not a replacement for it. It fills a notch and shaves a spur of up
  to three cells and leaves every feature wider than the window exactly where
  it was; the pillars (ten cells and up), the arches through them (thirteen
  across) and the passages (eleven and up) all pass through untouched.
  Additions are clipped to carvable rock like everything else, so it cannot
  smooth its way through the seal.
* **Straight walls, which were not tool marks at all.** The lens flood was
  clipped to the room's nominal box, and with `BEDDING_ANISOTROPY` at 3.4 the
  flood's own iso-cost contour is *wider* than that box is -- so the clip, not
  the rock, set the outline, and the room came out with dead straight sides.
  Giving the box half again the nominal half-width makes the budget the
  binding constraint, and the outline is then a contour of the cost field.
  **A box nothing drew still looks drawn.**

---

## 11. What I could not establish, and what is left

* **Whether the room sizes are the ones the owner meant.** The bar was read as
  linear extent (`cave-redesign` §8.1): 435-1015 across, from a room he called
  small at 145. The generator's *largest room per system* runs median ~520 and
  reaches 660; the median room over all of them is smaller, because a system
  is a cathedral with smaller rooms hung off it rather than a row of equals.
  That is a judgement call and it is on the review cards.
* **The collapse is a single upward sweep, and a real one is iterative.**
  Charging each ceiling cell for its distance to the nearer abutment is the
  better statics and produces **no dome at all** in one sweep -- the middle
  fails, the row above sees a narrower run, and it closes. Recorded at the
  call site so it is not rediscovered. What ships charges the whole run and
  bounds the result by `DOME_ASPECT`, which is a shape rather than a budget
  but is still a rule about the answer rather than about the rock.
* **`welds` reaches its cap of 12 on some seeds**, which means the void was in
  more pieces than the repair could join and the system stayed fragmented.
  Rare, counted, and the number to watch if a card comes back saying a cave
  dead-ends.
* **The lintel writes up to 17,000 cells of hillside as rock** on a long
  entrance run. It is invisible from outside (it is a lining, under the soil)
  but it is a lot of writing, and in a world as flat as this one an entrance
  is a long shallow adit rather than a mouth in a cliff. **The right fix is
  upstream**: W1's relief gives a mouth somewhere to open, and this should be
  re-measured after it lands.
* **The formations are still the weakest thing in the picture**, exactly as
  the owner said (*"not at all the main issue"*). The aspect cap stops the
  needles; it does not give them a profile.

---

## 12. Round three: the passages, the middle of the range, and a bug

Three verdicts on the second set of cards, and the first is the only
unambiguously positive thing said about worldgen in this programme:

> *"This is worlds better than our previous iterations. Thank you. Still needs
> some work. The tunnels and caves are too boxy. They read more planned than
> natural, especially the tunnels."*

> *"small tunnels leading to huge caverns. There should be more smaller
> caverns too."*

> *"In all these test images I am being shown, there are some spots where it
> looks like background (sky) is coming into the cave."*

### 12.1 Why the tunnels read as planned: because they were plans

**A shortest path on a square lattice is a straight line.** The conduits
minimise a sum over a grid, so they have no reason to wander and come out as
straight runs meeting at angles -- which is exactly what a plan looks like,
and the owner read the algorithm off the picture.

The cure has to be in the **field**, not in a filter afterwards: rounding the
corners of a straight tunnel gives a straight tunnel with rounded corners. So
the per-cell traversal cost carries a low-frequency roughness (`WANDER`, at a
90-cell wavelength -- a few passage widths, so a detour is genuinely cheaper
than the straight line over a stretch long enough to be a bend rather than a
wobble). A second term: the bore's radius rides its own slow wobble along the
run, half as wide at its narrowest and half again at its widest, because a
tube of one radius is a pipe however the route bends.

### 12.2 The size range was bimodal, and the bands now say so

*"Small tunnels leading to huge caverns"* is a description of a distribution
with a hole in the middle: at four bands starting at 150 cells there was a
passage or a hall and nothing between. Two bands were added below, and the
smallest is the one that was missing:

| band | across | in gnome-heights | share of draws |
|---|---|---|---|
| **cell** | 60-140 | 4-10 | 30% |
| **chamber** | 140-300 | 10-21 | 34% |
| **hall** | 300-500 | 21-36 | 22% |
| **great** | 500-720 | 36-51 | 11% |
| **cathedral** | 720-950 | 51-68 | **3%** |

Rooms per system went from 3-6 to 4-9 with them, because the small ones cost
almost no envelope and a three-room system cannot show a distribution at all.
Measured over four `rolling` seeds, thirteen rooms: **4 cell / 3 chamber /
2 hall / 3 great / 1 cathedral** -- every band occupied. `vaults detail`
prints the count per band on every world, so this is censused rather than
asserted.

### 12.3 The sky in the cave was the renderer, and the cause was my own fix

**Discriminated before it was touched, as it should be**: the wide-context
frames show rock on every side of the affected chambers, so the void was not
breaching to open air -- the cells were being *drawn* as sky.

The cause is §6's own fix, one level down. `freeze_underground_map` treated a
cell as uncovered when it is at or above **its own column's** topmost ground,
and a shaft cut down from the surface has no ground above it -- so every cell
of the shaft answered "uncovered", the flood spent none of its budget
descending, and it arrived at a chamber hundreds of rows down with the whole
allowance intact.

**Two rules were written before the right one, and they are wrong in opposite
directions.** The obvious repair is to ask about the ground *around* the
column instead -- the shallowest within twenty-four either side -- and that
blackens a **deep valley**, because a valley floor is far below the ridge
beside it and has perfectly good sky over it. `render.rs`'s
`the_per_cell_map_never_turns_open_sky_into_cave` exists for exactly that
trade and goes red for it.

What separates a canyon from a shaft is neither depth nor the neighbours: it
is the **aperture**. At a given row, how wide is the run of columns open at
that row? A canyon is hundreds and is open country however deep it is; a shaft
is a dozen and is a hole. So a cell is uncovered when the opening it sits in
is at least `World::SKY_APERTURE` (20) columns across, and past that the cover
budget starts running. `SKY_PENETRATION` came down from 48 to 32 with it.

**The guard was watched going red before its green was cited**: with
`SKY_APERTURE` set absurdly high -- nothing counts as open -- the test fails,
and at the shipped value all 78 render tests pass. A guard whose green is the
default state is not evidence.

**That is the second time this session a rule keyed on a column has been wrong
about a hole in that column**, and it is worth stating as a shape:
`sky_surface` answers *"is there anything above me"*, and both of these needed
*"is there any sky over this piece of ground"*. `ground_datum`'s own doc
records the same distinction being needed for terrain shading.

---

## 13. Gates

`cargo test --lib`, `cargo +1.98.0 clippy --all-targets -- -D warnings`,
`bash scripts/acceptance.sh` and `cargo run --release --example ascii` are all
green on the head this report describes. `bash scripts/docscheck.sh` reports
one thing: this report is not indexed in `Reports/README.md`, which this lane
was told not to edit -- **the coordinator has to add that line**.

**One acceptance note worth keeping.** Run while the box was carrying a
sixteen-seed census and a debug test build, `ligament`, `rockdrop` and
`lavadrop` all failed their 60 ms worst-frame budgets (68.6, 66.1, 117.8 ms);
re-run on a quiet box, every case passed. That is `CLAUDE.md`'s own rule
arriving on schedule -- *a wall-clock assertion is a flake generator*, and *a
timing number is only as trustworthy as the box was quiet*. Nothing in this
change touches those scenes.

---

*Freshness: written 2026-08-30 on `claude/worldgen-caves`, off
`47d6209`. Every figure is reproducible from `examples/cave_probe` at that
head; the invocations are in the file's own doc comment. Same-build
determinism holds and was checked; note `CLAUDE.md`'s warning that the release
profile re-inlines on any recompile, so no figure here should be A/B'd across
a build.*
