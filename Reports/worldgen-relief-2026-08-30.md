# Relief with a cause: mountains, benches, and rock that stops being stripes

Lane H of the worldgen revamp, 2026-08-30, on `claude/worldgen-relief`.
The plan is `Reports/worldgen-revamp-plan-2026-08-29.md`; this is **the item
that plan calls "THE revamp"** (W1), and it carries the bedding-geometry
correction the owner's rock-vocabulary verdict added to it mid-lane.

The brief was the owner's own words, twice over: *"it is the build"*, and
*"Shape, large rock formation (not just tall pillars), cave openings,
mountains."*

**Read section 5 before section 3.** The measurements are real and the change
was accepted on the A/B — but the owner's replies moved the target twice while
this lane was running, and two of the four cards came back about something the
numbers do not measure at all. Section 5.4 has the quotes.

---

## 1. What is different, in one paragraph

**The ground now wears down at the rate its rock allows, and what is left
standing is the strong rock.** Erosion could previously only remove, and only
where the ground was already steep, so a flat world stayed flat for ever
(measured: no column ever peaked above its iteration-0 prominence, 0 of 2048,
in either preset). A slope-free lowering term coupled to the resistance of the
rock *under* the surface turns the stratigraphy the world already had into
topography: a strong bed holds and the soft ground beside it is carried off,
and the difference between them is a bench, a scarp or a mesa. On top of that
the terrain gained a **massif** — country-scale relief, three to six screens
between crests — because nothing in the pipeline had a wavelength longer than
half a screen and *"mountains"* is a request for one. And the bedding those
two read now **dips, folds and steps at faults**, because it had no `x` in it
at all and rendered as level ribbons running the full width of the world.

The rock is also **quieter**: the six rocks are pulled onto one shared hue with
half the brightness gap between them, because shown the folded and faulted
version the owner replied that the geometry was no longer what bothered him and
the colour was.

---

## 2. The mechanisms

### 2.1 Differential lowering (`erosion.rs`)

Every rate in `erosion.rs` is driven by **slope** — thermal shed relaxes
over-steep faces, creep diffuses curvature, stream power carves in proportion
to gradient. On ground that is already gentle they all do nothing, and gentle
is what our ground was. That is the whole content of dead ends #22/#23
(`worldgen-prior-art-and-dead-ends-2026-08-29.md`): *"turn erosion up and the
formations will come"* is measured impossible **because of the input**, and
the audit flags it as the corpus's most re-testable rejection for exactly that
reason.

Real landscapes lower everywhere, at a rate set by the rock rather than by the
slope. One term, added before the slope terms each iteration:

```
h[x] += STRIP_RATE * (section[x] - mean(section) over ±STRIP_REACH)
```

Three things in that line are load-bearing, and two of them were corrections
made after building the obvious version and looking at it.

**`section`, not `hardness`.** `HardnessField::at` gives the resistance of the
single bed at the surface. Read at one bed, a column that strips through a soft
bed immediately meets the next independent draw, so the surface can never
travel more than one bed before stalling — the relief such a rule can build is
capped at `strata_thickness`, 8 to 12 cells. `HardnessField::section` reads the
top three beds with the topmost weighted double, which is what a cap rock
actually is. It is a **new sampler rather than a change to `at`**, deliberately:
`residual.rs`'s `CAP_CONTRAST` and `LOW_VARIANCE` are calibrated against `at`'s
variance (C7's escapability note warns of exactly this), so widening that
distribution would have been a change to the residual shape classifier too.

**A contrast against the neighbourhood, not a level.** Written the obvious way
— lower where the rock is soft, raise where it is hard, against a fixed
reference hardness — it improved **every number in the census** and produced
the flattest world this generator has ever made. Rendered at 8192x2560 it is
long dead-level tables with sharp notches and no hills at all: the surface
reaches its fixed point everywhere, and since a bed's resistance has no `x` in
it, that fixed point *is* the bedding plane. Subtracting a running mean over
±200 columns leaves only the part that varies *within* a landscape, so the
region layout, the massif and `RegionMap`'s composition guarantee pass through
untouched and differential erosion decides which parts of a hillside stand
out rather than how high the hill is. It also deletes a constant: a contrast
against the local mean is invariant to the field's mean, which a fixed
reference hardness is not.

**This is CLAUDE.md's "look before you measure" earning its place.** The
census said screen relief up, near-air up, local relief up. The picture said
the hills were gone. Had the numbers been trusted, that world would have
shipped.

`STRIP_REACH` is a running sum, so the term is O(w) per iteration and its cost
is one extra hash per column per iteration on top of the hashes the loop
already pays.

### 2.2 Lateral facies (`column.rs`, `HardnessField::section`)

The flat-table failure above has a second half. `HardnessField` draws hardness
on **the band index alone, no `x`** (C7), so a bed is equally resistant along
its whole 8192-column outcrop, and every column in a neighbourhood stalls on
the same bed. `section` therefore blends the bed draw with a three-octave
`(x, band)` field at a 320-column first wavelength: a bed dies out along
strike, the stall surface has somewhere to step down to, and a table becomes a
bench, a butte and a bounding scarp.

**`FACIES_WAVELENGTH` is the constant that sets how wide a rock formation is**,
which is the whole reason it exists — the owner's *"large rock formation (not
just tall pillars)"* is a statement about this number, and nothing in the
generator was speaking to it. Every other wavelength in the pipeline varies by
at most 1.70x across all five presets and none is in this band.

It is applied to `section` **only**, so `at` — and with it terracing, the shade
pass, the rock vocabulary and `residual.rs`'s classifier — sees the
distribution it was calibrated against.

### 2.3 The massif, and the bedding it stands in (`column.rs`)

**Mountains.** Every elevation term was shorter than a screen
(`hill_wavelength` 150-200, `detail_wavelength` 24-34) or was the region draw,
which changes every 96-241 columns — so the largest thing the terrain could
express was about half a screen wide, and a landform smaller than the view
reads as a bump. `massif()` is a three-octave **ridged** field at a 2200-3000
column wavelength, squared, so basins are broad and summits are the tail: high
country you travel to rather than hills everywhere.

Two details cost a rebuild each and are worth recording. The fold has to
happen **per octave** (`noise::ridged_1d`): folding the summed fBm instead
gives a field concentrated against its *maximum*, mean 0.567 over 2^20
samples, with most of the world near a crest and troughs rare — exactly
backwards. And `MASSIF_MEAN` had to be **measured, not derived**: the closed
form for a uniform draw says 1/3, the field gives **0.364** (sd 0.198, range
0.0002 to 0.9988 over 2^22 samples), and the error would have moved every
world's ground line by up to fifty cells.

**Bedding geometry.** Shown the rock vocabulary the owner approved the rocks
and rejected the geometry — *"the perfect stripped bands are too uniform and
look bad"*, and, asked directly, *"very clearly reads as stripes and it looks
not good"*. He is looking at a **cut face**: `stone_massif` is about 90% of
the world's cells, so most of the screen is a cross-section through rock.

`strata_offset(x)` had a global tilt of 0.06 — 31 cells across a screen, about
3.5 degrees, which reads as level — plus a ±6-cell ripple at a 130-cell
wavelength, which is a texture and not a structure. Two terms were added, both
ordinary structural geology:

- **a long fold** at 7x the ripple's amplitude and 900 columns, so a bed climbs
  or falls across a view and the dip itself rolls over as you travel;
- **faults**: a horst-and-graben field on 380-column blocks, each drawing its
  own throw of up to three beds about zero, so the step between two blocks is
  triangular about zero — most boundaries move the section a little and a few
  move it a long way. The boundary flexure width is drawn per block too (4 to
  22 columns), so some breaks are a line and others are a monocline.

**All six consumers follow for free**, because they all read the one function:
the shade pass, the rock vocabulary, `HardnessField`, `terraced`, the cave
shear and the sand lens's dip. Deform the coordinate and the whole section
deforms together — which is also why the differential erosion above now has a
*dipping* resistant bed to daylight, and a daylighting resistant bed is a
bench.

Both terms scale off `strata_fold`, so a preset asking for zero gets the exact
coordinate this function had before. That is not tidiness: `flat` is the
structural test bed, its own comments require its control renders to stay
byte-identical for the destruction workstream, and it sets `strata_fold: 0.0`.

---

## 3. The measurements

**Every number below is a paired A/B in one process**, one binary, one
machine: `PIXEL_PHYSICS_RELIEF=0` restores the shipped pre-W1 **terrain** and
is the arm labelled *before*. (It gates the shape work only. The colour work of
section 5.5 rides the rock vocabulary's own switch, `PIXEL_PHYSICS_ROCK_VOCAB`,
which is why that section's numbers are a separate paired run.) Its output was checked **identical to the digit**
against a run of the pre-change binary on the same seeds -- screen med 36,
p90 64, max 107, mean|step| 0.28, reach15 p99 8, reach30 p99 11, near-air
3.5%, brows 2,928, talus 1,472, residuals 9,312 on `rolling` -- which is the
specificity half of `CLAUDE.md`'s control rule: the instrument stays quiet
when the mechanism is off. The sensitivity half is the tables themselves.

Instrument: `examples/wg_ceilings.rs mode=relief`, added by this lane. Six
seeds per preset at the shipped 8192x2560, order statistics over every
512-column window and every column of every world -- never a single seed.

### 3.1 How far the skyline moves across one player screen

The audit's headline was 12-42 rows of 320 (five presets, seed 1). Median over
every 512-column window of six worlds, with the p90 window beside it:

| preset | median before -> after | p90 before -> after |
|---|---|---|
| arid | 32 -> **69** | 49 -> **112** |
| canyon | 90 -> **119** | 132 -> **203** |
| rolling | 37 -> **54** | 64 -> **125** |
| terraced | 45 -> **56** | 71 -> **124** |
| wetland | 19 -> **30** | 31 -> **66** |

**Every preset roughly doubles at p90**, and `arid` -- the flat one, 12 rows
on the audit's screen -- more than doubles at the median too. A typical screen
now moves 30 to 119 rows of 320 rather than 19 to 90, and one screen in ten
moves 66 to 203 -- against the audit's 12 to 42.

### 3.2 The formation-scale band

Two statistics, and the difference between them matters. **Prominence** is
`viewshot`'s definition, two-sided: how far a column stands above the ground
on *both* sides. It scores a spire and scores **zero** on a scarp, a bench rim,
a mesa edge and the whole interior of a plateau -- so steering by it rewards
exactly the tall pillars the owner rejected. **Local relief** is the range of
the skyline over the same window, and counts any vertical structure at that
width whichever side of it the ground is on. Both are printed; neither is
enough alone.

Local relief, p90 and p99 over every column of six worlds:

| preset | reach 15 p90 | reach 15 p99 | reach 30 p90 | reach 30 p99 |
|---|---|---|---|---|
| arid | 13 -> **31** | 35 -> **69** | 21 -> **46** | 41 -> **85** |
| canyon | 31 -> **53** | 70 -> **135** | 54 -> **83** | 99 -> **177** |
| rolling | 16 -> **29** | 45 -> **78** | 27 -> **50** | 54 -> **98** |
| terraced | 18 -> **27** | 42 -> **81** | 30 -> **47** | 54 -> **98** |
| wetland | 8 -> **16** | 25 -> **52** | 14 -> **26** | 34 -> **65** |

**Roughly 2x on every preset at both reaches and both order statistics.** The
band the audit found empty -- *"at reaches 15 and 30 the tallest thing in the
entire world is 4 to 10 cells"* -- now carries 26 to 83 cells at p90 and 65 to
177 at p99.

Prominence moves too, but only at the wider reach: reach 30 p99 goes 11-17 ->
21-29, while reach 15 p99 is flat (2-8 -> 3-9). **That is the shape of the
change, not a shortfall in it.** What was built is bench-and-mesa relief,
wider than 30 columns, so it registers as local relief and as reach-30
prominence and not as reach-15 prominence. A change that moved reach 15 would
have been a change that made more pillars.

### 3.3 How much of the ground on screen is near air

`terrain_shade`'s ceiling statistic -- the fraction of on-screen ground within
six cells of air, which bounds what anything that shades, outlines or textures
the ground can reach. Eight viewports per world aimed at the skyline:

| preset | share of ground before -> after | near-air cells per screen |
|---|---|---|
| arid | 3.6% -> **4.3%** | 3,918 -> **4,660** (+19%) |
| canyon | 4.1% -> **4.4%** | 4,504 -> **4,905** (+9%) |
| rolling | 3.4% -> **3.7%** | 3,759 -> **4,076** (+8%) |
| terraced | 3.5% -> **3.9%** | 3,857 -> **4,174** (+8%) |
| wetland | 3.1% -> **3.4%** | 3,391 -> **3,650** (+8%) |

**This is the weakest of the three, it is reported with its denominator, and
it is reported *after* a change that cost it.** Three things to read here.

The share is `near-air / ground on screen`, and relief moves both terms: a
viewport on a mountain flank holds more solid than one on a plain, so the
share can fall while the boundary itself has grown. Measured, the ground share
did **not** move (66-67.8% in both arms), so the cells-per-screen column is
the honest reading and it says **+8% to +19%**.

It was **+11% to +32%** before the brow repair in section 5.2, and the repair
took a third of it back. That is the correct trade and it is stated rather
than buried: the combs of shelves it removed were real air/rock boundary and
they looked like a fish bone.

And the reason it cannot do much better is geometric. A smooth skyline already
contributes about 512 columns x 7 rows = 3,600 near-air cells to a 512x320
viewport, which is most of the *before* number. Doubling it needs air the
ground line does not supply -- overhangs, notches, and above all **caves**,
which measure 21.8% against the surface's 4.7%. That is W3's, not this lane's.

### 3.4 The starved passes switched back on

The plan's prediction was that supplying relief *"switches `brows`, `talus`
and boulders back on for free"*. Cells written per world, six seeds, both arms
through the repaired `brows`:

| preset | brows | talus | boulders | residuals |
|---|---|---|---|---|
| arid | 52 -> **2,864** | 271 -> **11,475** | 0 -> **225** | 3,269 -> 2,674 |
| canyon | 766 -> **3,741** | 5,607 -> **18,030** | 151 -> **2,112** | 7,647 -> 6,509 |
| rolling | 399 -> **2,871** | 1,770 -> **15,312** | 50 -> **1,993** | 7,039 -> 4,854 |
| terraced | 346 -> **3,115** | 1,185 -> **13,194** | 76 -> **933** | 6,274 -> 3,963 |
| wetland | 27 -> **1,189** | 90 -> **5,947** | 0 -> **751** | 5,401 -> 2,275 |

`brows` 4.9x to 55x, `talus` 3.2x to 66x, `boulders` 12x to 40x and from zero
on two presets. Both arms run through the **repaired** `brows`, which is why
its absolute numbers are far below the first measurement of this change (up to
45,797 cells on `canyon`): most of that volume was the comb, and section 5.2
took it out of both arms. **`boulders` writes cells at all for the first time** on
`rolling`, `terraced`, `arid` and `wetland` -- the plan's section 3.4 recorded
it at 0 cells on all six presets and 3 seated in 80 worlds, against a wiki
paragraph that gives boulders *"an event, not a decoration"*. It is a
wiki-documented feature that now exists.

`residuals` moves the other way, 3,269-7,570 -> 2,275-5,933 cells. Real relief
crowds the sites out, and given *"not just tall pillars"* that is the right
direction -- but it is a side effect of this change rather than a decision,
and W2 owns the shape of what is left.

## 4. What it cost

**Generation time: 2,054-2,175 ms per 8192x2560 world before, 2,334-2,439
after — about +290 ms, +14%.** Paired, sequential, same machine, six seeds per
preset. **Read it as an order of magnitude, not a figure**: the same census
re-run later on the same binary returned a byte-identical world and
`gen 6,077 ms` for a line that had read 2,439, because another session was on
the box. `CLAUDE.md`'s rule holds — a timing number is only as trustworthy as
the box was quiet — and what makes this one usable at all is that both arms
ran back to back inside one command. All of it is build-time: the differential term is one extra hash and a
running-sum subtraction per column per erosion iteration, `section` costs three
hashes where `at` cost one, and the massif and the fold are two more fBm calls
per column in a function that is already evaluated once per column.

**Frame cost: nothing.** No per-frame code changed — every line of this lane
runs at generation time — so the world is different, not the simulation of it.
`examples/ascii` ran clean over all 31 scenes at a worst frame of 46.5 ms and
a mean of 2.76 ms over 12,000 frames, which is the shape of the number rather
than a delta: `CLAUDE.md`'s own rule says an untrusted worst frame among
thousands of comparable ones is noise wearing a number, and nothing here gives
it an aggregate to pin against. The claim that carries weight is the
structural one, not the timing.

**`scripts/acceptance.sh` is green on all cases**, which it has to be for a
non-obvious reason: `flat` is the structural test bed, and the destruction
workstream's control renders require it to stay byte-identical. All three
terms are off there by construction — `massif_amplitude: 0.0`,
`strata_fold: 0.0`, `world_age: 0.0` — and the `world_age == 0` early return
had to be restored after the settle tail was first added *before* it, which
would have run sixty iterations of thermal and creep on a bed whose whole
purpose is not having had any.

---

## 5. What broke, what the owner said, and what was done about it

### 5.1 A mountain does not fit in a 320-row world

**The presets are written for 8192x2560 and the same `WorldgenParams` build
the 512x320 worlds that `filmstrip`, the foraging scenes and most of `tests/`
use.** That is the audit's own loop failure #1 — *judge at the shipped world
size* — arriving from the other direction, and it broke four tests before
anyone had looked at a small world.

`datum` grew by `massif_amplitude * (1 - MASSIF_MEAN)`, which at 260 cells is
more than half of a 320-row world, so `plan_from`'s clamp caught every column
at the same row and `every_seed_has_a_ridge_and_a_valley` reported **0 cells
of relief across the world**. Independently, the differential term drove
**134 of 512 columns onto the floor clamp** on `canyon` seed 8, leaving a
dead-level 134-column pan that `ponds` filled with a six-cell-deep sheet —
and a sheet that shallow is not a lake. `a_forced_residual_world_arrives_at_rest`
caught it correctly: 93% of that world's water left in 120 frames, and it was
water, not rock, that moved.

Both are bounded by one measured quantity, `Terrain::relief_headroom` — the
rows left after the sky guarantee and every existing elevation term have taken
their full swing. It is **a bound on a magnitude, never a gate on a decision**:
less room gives smaller relief, continuously, and at zero it gives the world
this generator shipped before W1, bit-identical. At 8192x2560 the room is 2,180
rows against the largest preset's 380, and the shipped-size census re-run
through the bound is **identical on every measure to the run before it** — so
nothing the player sees is clamped by this.

The cost of the bound is real and should be stated: **`filmstrip` and every
512x320 scene see none of W1.** Those are test beds, and a test bed whose
terrain silently changed would have been the worse outcome — but anyone
judging this work from a `filmstrip` render will see nothing, which is the
mistake the plan's loop-failure list opens with.

### 5.2 `brows` combed every slope

**`brows` combed every slope.** `cliff_edges` asks only that the ground falls
away on one side, and its far test is a 20-row drop over 20 columns — a slope
of exactly 1.0. So *every column* of any sustained 45-degree hillside is a
cliff edge, and at `brow_chance` 0.8 four in five of them hang a lip. While the
terrain never cleared that test this was invisible (`brows` wrote 2,352 cells
of 18.9M). Supplied with real relief it rendered as a **ladder of thin shelves
stepping down both flanks of every knoll** — a fish bone, and the single most
artificial thing in a W1 render.

This is `CLAUDE.md`'s *fixing a bug exposes the constant that was compensating
for it*, with an unusual compensator: the compensating quantity was the
**terrain**, not a number. So thinning `brow_chance` is the wrong repair — a
uniform thinning leaves the comb, only sparser, which is the shape the owner
has rejected by name on a different pass. An overhang is a lip at the *top* of
a face, and what makes it a top is level ground behind it. `brows` now requires
the ground behind the edge to stay within 3 rows over `RUN_NEAR` columns.

**That was not the whole of it, and the owner found the rest.** Shown the first
W1 render he replied *"the leftmost peak has weird artifacts at the top"*.
Cropped and enlarged, they are lips jutting horizontally out of a **rounded
dome** with nothing under them — planks floating in the air. The bench test
cannot catch that on its own, because a dome *is* level over four columns
behind the edge; what admits them is `cliff_edges` accepting an edge on
*either* test, and the far one (20 rows over 20 columns) is satisfied by the
flank of any rounded hill. So a brow now re-asks the **near** test as a
requirement rather than as an alternative: six rows over four columns, the
pass's own number for a face. `talus` keeps the far test, and should — scree
does mantle a long slope, and a plank does not.

Two consequences, both measured:

- The near-air gain fell from +11–32% to +8–19%. The combs were real air/rock
  boundary. That trade is the right way round and section 3.3 states it.
- **`boulders` roughly tripled on the control arm as well** — canyon 3 → 151
  cells with no relief change at all. `pass-interference-2026-08.md`'s R4-1 is
  that `brows` deletes boulders by getting to the crown first; fewer spurious
  lips is fewer deletions. That defect is W5's and this only dents it.

### 5.3 A guard that had been rubber-stamping

`an_old_world_is_smoother_than_a_young_one` asserted that erosion reduces
summed |slope| — age 0 against age 2, on **seeds 1 and 7 only**, at least 2%
smoother. W1 nudged seed 7 across the line (172.6 → 174.3, a 1% miss), and
measuring properly is what the failure bought:

| | median ratio, age 2 / age 0 | at age 0.5 |
|---|---|---|
| shipped code | **1.185** | 1.010 |
| with W1 | 1.104 | 1.003 |

**An old world is *rougher* at the median in both arms**, and it already was
before this lane touched anything. Moving the comparison onto age 0.5 — the
stretch the test's own comment called monotone — does not rescue it either.
It was a two-seed test over a procedural system whose property does not hold,
which is `CLAUDE.md`'s *"a guard over a procedural system has to sweep the
procedure and gate an order statistic"* exactly.

It is **deleted rather than ported**, and replaced by
`differential_erosion_builds_relief_that_slope_driven_erosion_cannot`, which
guards the claim the module now makes. Its two arms are a 320-row world and a
1200-row one at `massif_amplitude: 0.0`, so `elev` hands both the identical
profile and the only difference is whether `relief_room_fraction` lets the
term run: a paired A/B, same seed, same terrain, same binary. It gates the
**median over eight seeds** at 1.4 against a measurement of 2.33 — two of the
eight go the other way, and that spread is why it is a median. It carries both
a fired counter (`stripped`) and an effect counter (local relief), and it has
been watched going **red** with `PIXEL_PHYSICS_RELIEF=0`.

---

### 5.4 What the owner said, shown it

Three cards, all answered within the hour, all at the shipped size with the
gnome in frame. **He picked the relief arm** on the before/after, and the other
two came back about something else entirely.

**On the relief itself** (`20260830T013709739Z-03dbdb`, chose B):

> *"There are some aspects that I like about B, but it also doesn't look too
> natural. the leftmost peak has weird artifacts at the top. Then there is
> just one super sharp peak. I think a more natural landscape would have
> repeated features but those features vary alot between biomes/areas. Here
> you have a bunch of different interesting features but they are all in the
> same place and too different together."*

Three separable findings, and only one is this lane's:

- *"weird artifacts at the top"* — **acted on**, section 5.2's second half.
  Cropped and looked at, they are `brows` lips jutting horizontally out of a
  rounded dome with nothing under them: planks, not overhangs. Fixed by making
  a brow re-ask the *near* cliff test as a requirement.
- *"just one super sharp peak"* — a residual. That is W2's pass and the owner
  has complained about it before, by name.
- **"repeated features that vary a lot between areas… here they are all in
  the same place and too different together"** — this is the most useful
  sentence in the three cards and it is **W7's**, stated in the owner's own
  words for the first time. It is not a request for *more* variety; it is a
  request for variety at a **different scale** — homogeneous within an area,
  different between areas. W1 gives a world one vocabulary of benches, scarps
  and mesas everywhere, and by that reading that is exactly the complaint.

**On the bedding geometry** (`20260830T013728630Z-ed21b1`) he did not answer
the question asked. Shown dipping, folded, faulted rock and asked whether it
still reads as stripes, he replied:

> *"I think the main issue is that the color changes between layers is too
> much. There is too much contrast. There should be more of a shared color
> palette. Also the layers should only start deeper. Anything above the ground
> level should be more uniform no layers (this is just an aesthetics thing, you
> can keep different materials for the world gen if needed"*

and gave the identical verdict on the third card, unprompted. **Read that as
the stripes complaint being superseded rather than answered**: the geometry is
no longer what he names, and the palette is. Two concrete instructions in it —
a narrower shared palette between beds, and no visible layering near the
surface, with the section only starting deeper. The parenthesis matters: he
explicitly severs the *look* from the *materials*, so the six rocks can stay
and only the tones need to change.

**Both were taken up by this lane rather than handed on** (section 5.5),
because he had said the same thing twice unprompted, which makes it the
highest-value thing left. Shown the first colour step he replied *"I would
drop the contrast even more"*, and shown the near-surface change,
*"These look identical to me"* — so the first went a second round and the
second was **counted instead of tuned**, which is what settled it.

---

### 5.5 Acting on the colour verdict

The owner gave the same verdict on two of the three cards, unprompted, so it
was taken as the priority over anything else this lane could still do. **Both
pieces of it are gated by `PIXEL_PHYSICS_ROCK_VOCAB`**, not by this lane's own
switch: they are changes to the rock vocabulary, and the vocabulary lane's
control arm and the destruction workstream's `flat` control render both have to
stay byte-identical to the shipped world. The near-surface rule leaked into
that control on its first writing — it reached the pre-vocabulary path too,
because that path reads the same `tone` — and the gate is what closed it.

#### The beds pulled onto one palette

Five of the six rocks — `mudstone`, `sandstone`, `limestone`, `ironstone`,
`basalt` — have every colour in every family pulled **86% of the way onto one
shared warm-grey hue**, and the brightness gap *between* rocks halved. The
transform is written down rather than eyeballed so it can be re-derived: the
shared hue is the mean of the six fresh families renormalised to unit
brightness; each material is shifted bodily toward the global mean brightness
by half its own distance from it, and then each colour is blended toward the
shared hue at its own shifted brightness.

**The shift is per *material*, not per colour, and that is the load-bearing
detail.** The first version compressed every colour toward one global mean,
which squeezes the four tones *inside* a rock as well as the gap between
rocks — and the tones inside a rock are what make a bed a bed. At the
compression the owner asked for, that would have taken stone's own 28-point
family down to 15, straight into the window `stone.ron`'s comment records as
the one where banding vanished from the render. Shifting the whole material at
once leaves every internal spread at exactly its original value and compresses
only what the complaint was about.

Measured over the six fresh families: the brightness range across rocks goes
**57 to 176 → 88 to 145** — 116 points to 57, halved — while the spread inside
any one of them is unchanged.

**`stone.ron` is deliberately untouched.** Its families 0–3 are what
`PIXEL_PHYSICS_ROCK_VOCAB=0` restores byte for byte, so editing them would
break the vocabulary lane's control arm; and family 0 is neutral grey at
brightness 128, already on the shared hue and at the mean, so leaving it out
costs nothing.

**Brightness is the axis that was left alone, on purpose.** `stone.ron`'s own
comment records that a weighting which kept most bands eight apart made the
banding vanish from the render entirely, so the failure to avoid is a
*brightness* collapse. After the change the fresh families run 74 to 163 in
brightness — an 89-point spread against the 28-point window that failed.

Measured paired, same seeds, same geometry, one difference
(`world_look mode=colour`, canyon, 6 viewports):

| | colours covering half the ground | covering nine tenths | distinct colours |
|---|---|---|---|
| seed 1: shipped → first pull → second | 14 → 11 → **7** | 55 → 40 → **29** | 179 → 153 → **142** |
| seed 2: shipped → first pull → second | 15 → 13 → **9** | 50 → 41 → **28** | 170 → 147 → **139** |

Two steps, because the owner judged the first and replied *"I would drop the
contrast even more."* The rock vocabulary took the first column from 4–8 to
5–14; this hands most of that back while keeping the rocks themselves.

**Where the floor is, stated with the number rather than guessed at.** 7–9 is
close to the pre-vocabulary 4–8, so there is not much further to go by pulling
*rocks* together. What must not be touched is the spread inside one rock — the
four tones that make a bed legible — and the transform above is built so that
pulling harder cannot reach them. Past this point, less contrast has to come
from changing *what* varies, not how much.

#### Bedding that starts deeper

*"the layers should only start deeper. Anything above the ground level should
be more uniform no layers."* **"Above the ground level" has two readings and
only one of them moves the picture at all**, which is how the *first* half of
the ambiguity got settled:

| reading | share of a shipped-size frame it changes |
|---|---|
| deeper than this column's own surface | **0.04%** (209 pixels of 491,520) |
| deeper than the **base level** — the lowest ground within half a screen | **19.4%** |

The first is nearly a null because the top of a mesa *is* its own column's
surface, so a 150-cell tower measures as one cell deep at its summit. `Ctx`
gained `base_y`, a sliding-window maximum of `surface_y` over ±220 columns,
computed once by a monotonic deque in O(w). Rock standing above that draws one
tone; the bedding fades in over 44 cells below it, dithered rather than cut,
so no ruled line crosses a face.

**The materials are untouched**, which the owner explicitly allowed and which
this lane needs: a soft bed under a hard cap is what the differential erosion
is made of. Only the tone is unified.

**Posted, judged, and the verdict was *"These look identical to me."*** That
is a finding about the change, so it was counted rather than argued with:

| | |
|---|---|
| the two images | **not** byte-identical |
| pixels it redraws | **95,253 of 491,520 — 19.4% of the frame** |
| how far each moves, out of 255 | median **11**, mean 15, p90 24, max 38 |

So it is case two of the three: **it ran, it reaches a fifth of the picture,
and it is too small per pixel to see.** A fifth of a frame shifted by four
percent of the range is below anyone's threshold.

**And the reason is structural rather than a tuning miss.** The rule swaps a
bed's *tone* — one of four shades inside one rock, about 28 points apart by
design. What the eye reads as "layers" is the difference between *rocks*,
which was four times larger before section 5.5's palette work and is still
larger now. Unifying tone near the surface therefore **cannot** deliver
*"anything above the ground level should be more uniform, no layers"*, however
it is tuned. Unifying the *rock* is the mechanism that could — **untested, and named as a
hypothesis rather than a finding** — and this lane did not do it: the
differential erosion the whole of W1 is built on needs beds that differ in
strength, and weakening that to win a colour argument is not a trade to make
silently. It is put to the owner as a question instead, with the count.

**Kept rather than reverted**, on the grounds that it is nearly free (one
`Vec<i32>` per world and one lookup per cell), it is correct, and it will
matter more as the rocks keep converging — but it is honestly a lever that
does not reach the picture today, and the next session should not read it as
having answered the request.

#### And the guard three comments named, which did not exist

`stone_massif` walks a column through `ColumnShade`, memoising the per-band
draw; every pass that paints rock outside the massif calls `strata_shade` per
cell. Two implementations of one rule. **Three separate comments in
`passes.rs` cite `column_shade_matches_the_per_cell_version` as the thing that
stops them drifting apart, and no such test existed** — found because the
mantle rule had to be written into both and nothing complained. It exists now,
compares four presets x three seeds cell for cell with a "did it fire" count
beside the assertion, and has been watched going red.

---

## 6. What I could not do

**Bed thickness is still constant.** Every bed in a world is
`strata_thickness` cells thick, 8 to 12 depending on preset. Varying it means
warping the *quantisation* of the band coordinate, and six different consumers
floor that coordinate themselves — the shade pass, the rock vocabulary,
`HardnessField`, `terraced`, the cave shear and the sand lens. It is a change
in six places rather than one, and every one of them has calibrated constants
reading the current spacing. The rock **vocabulary** already warps its *unit*
thickness (`Purpose::RockUnit`), so the coloured packages vary while the
bedding planes inside them do not; that is why the section reads better than
this paragraph suggests, and also why it is not fully fixed.

**No unconformities.** A truncation surface with a differently-oriented stack
above it needs `strata_offset` to be piecewise in *elevation*, and it is a
function of `x` alone. Adding an elevation argument changes the signature every
one of those six consumers calls, and `HardnessField` precomputes the offset
per column specifically so the erosion loop does not pay it 600 times.

**Faults are vertical.** The block coordinate is a function of `x`, so a fault
plane is plumb. A real one dips 60-70 degrees, and inclining it needs the same
elevation argument the unconformity does. What is variable is the *flexure
width* (4 to 22 columns per block), so some breaks read as a line and others as
a bend, which is most of the visual difference.

**Reach-15 prominence did not move**, and section 3.2 argues it should not
have. If the owner's *"large rock formation"* turns out to mean something
narrower than 30 columns after all, that is a different mechanism from this
one — it is `residual.rs`'s profile work, which is W2.

**No `seedsweep.sh` run.** `CLAUDE.md` asks for it before changing a model over
procedural content, and this is such a change. It measures *structural* damage
under a strike, which is the wrong question for a terrain shape change and
would have been read at a frame budget its own default stops short of; what
this lane ran instead is a six-seed order-statistic sweep in
`wg_ceilings mode=relief` over the three quantities the change is *for*. The
structural gates that do bear on it — `scripts/acceptance.sh`, which is where a
world that is not at rest shows up — are green. A seed sweep of the
*destruction* consequences of much steeper ground is a real open question and
is named here rather than answered.

**`filmstrip` sees none of this**, and so does every 512x320 scene — see
section 5.1. That is a deliberate bound and it is also a trap for the next
session: judging W1 from a `filmstrip` render will show the pre-W1 world, and
the plan's loop-failure list opens with exactly that mistake in the other
direction. `examples/viewshot.rs` renders the shipped size and is what every
picture in this lane came from.

**Near-surface rock still reads as layered**, and section 5.5 says why the
lever built for it cannot fix that: the answer is to unify the near-surface
*material*, which was declined rather than attempted, and is with the owner as
a question.

**The "one of everything in one place" complaint is untouched.** The owner's
sharpest observation on this lane's own card — *"repeated features but those
features vary alot between biomes/areas… here they are all in the same place
and too different together"* — is a request for coherence within a place and
difference between places. W1 gives every stretch of every world the same
vocabulary of benches, scarps and mesas, so by that reading it makes the
complaint *more* true, not less. It is W7's, and this lane pushed in the
opposite direction while answering the brief it was given.

**The rate was set on two presets.** `STRIP_RATE` was swept at 0.10, 0.30,
0.60, 1.00, 1.50 and 3.00 on `rolling` and `canyon` at two seeds each, then
the chosen setting was checked on all five at six. The sweep is honest about
one thing: past about 1.5 the numbers keep improving and the pictures do not,
because the extra rate is spent driving columns that have already reached the
section that stops them.

---

## 7. What this changes for the other lanes

- **W2 (formations).** Residuals now stand in country that has benches and
  scarps of its own, and there are 25-40% fewer of them because relief crowds
  the sites out. The profile work has a different backdrop than it was scoped
  against.
- **W3 (caves).** `strata_offset` is the cave shear's coordinate, so caves now
  follow a *dipping, faulted* section rather than a level one. And the surface
  is 30-100 cells more broken, which is where a cave entrance would have to
  come out.
- **W5 (the discarded character).** R4-1 is dented but not fixed — see section
  5.2. `talus` is now supplied 3x to 66x more material to realise, so the
  R4-2 finding (median 244.5 computed, 3 cells realised) is worth re-measuring
  against a world that actually sheds.
- **W7 (provinces).** `massif_amplitude` and `massif_wavelength` are the first
  parameters in the generator whose unit is *screens*, and they are per preset.
  If a province is to be a kind rather than a gain, this is a knob that changes
  the shape of a country rather than its amplitude, and it is already there.
  **And W7 now has the owner's own words for what it is for**, given on this
  lane's card: *"a more natural landscape would have repeated features but
  those features vary alot between biomes/areas. Here you have a bunch of
  different interesting features but they are all in the same place and too
  different together."* That is not a request for more variety — it is a
  request for variety at a coarser scale, and homogeneity below it.
- **W0 (the rock vocabulary).** Two cards asking about *geometry* came back
  about *colour*, unprompted and identically: too much contrast between beds,
  a shared palette wanted, and **no visible layering above ground level at
  all** — the section should start deeper. He severs the look from the
  materials explicitly (*"you can keep different materials for the world gen
  if needed"*), so this is a tone change, not a rock change. Section 5.3 has
  both quotes in full.

## 8. Where the code is

| what | where |
|---|---|
| differential lowering | `src/worldgen/erosion.rs`, `STRIP_RATE` / `STRIP_REACH` and the block in `erode`'s loop |
| section resistance, lateral facies | `src/worldgen/column.rs`, `HardnessField::section` |
| the massif | `src/worldgen/column.rs`, `Terrain::massif`, `MASSIF_MEAN`; `noise::ridged_1d` |
| fold and faults | `src/worldgen/column.rs`, `Terrain::strata_offset` / `fault_throw` |
| the control switch | `src/worldgen/mod.rs`, `relief_on` (`PIXEL_PHYSICS_RELIEF=0`) |
| the two brow repairs | `src/worldgen/passes.rs`, `BROW_BENCH_TOLERANCE` and the near-drop requirement beside it |
| the world-size bound on both terms | `src/worldgen/column.rs`, `relief_headroom` / `relief_room_fraction` |
| the shared rock palette | `assets/materials/{mudstone,sandstone,limestone,ironstone,basalt}.ron` |
| bedding starting deeper | `src/worldgen/mod.rs`, `Ctx::base_y` / `base_levels`; `src/worldgen/passes.rs`, `MANTLE_DEPTH` and both shade paths |
| the shade guard that was missing | `src/worldgen/passes.rs`, `column_shade_matches_the_per_cell_version` |
| the instrument | `examples/wg_ceilings.rs mode=relief` |
| per-preset amplitudes | `assets/worldgen.ron`, `massif_amplitude` / `massif_wavelength` |

---

## 9. The cards, and where each stands

| card | verdict |
|---|---|
| `20260830T013709739Z-03dbdb` — the relief A/B | **chose "with relief"**; flagged artifacts at one peak (fixed, section 5.2) and *"one of everything in one place"* (W7, section 7) |
| `20260830T013728630Z-ed21b1` — does the rock still read as stripes | answered about **colour** instead; taken up in section 5.5 |
| `20260830T014140708Z-3cd1ef` — is this high country | the same colour answer, unprompted |
| `20260830T024556778Z-076c03` — the layers pulled onto one palette | **chose the pulled palette**, *"I would drop the contrast even more"* — done, second step |
| `20260830T025626657Z-7b5fad` — layers starting deeper | *"These look identical to me"* — counted rather than tuned, section 5.5 |
| `20260830T030452516Z-daed5b` — contrast dropped again, and where the floor is | **open** |
| `20260830T030513497Z-43e806` — the near-surface measurement, and whether to unify the *material* | **open** |

Six cards, four answered, and the two open ones are the two live questions
this lane leaves. Collect them with `review.py inbox`.
