# Sizing a sight sense before it is built — E15

**Status: measured pre-flight, 2026-08-30, re-taken in full on every tree
`main` landed underneath it — eight in all.** `examples/vision_probe.rs` is
the instrument; nothing here changes behaviour, and the probe is read-only
geometry over `World::get`.

**Why eight.** `main` moved under this study eight times while it was being
written, and each landing that touched the population, the floor or the
terrain was re-measured rather than waved through. Three of the eight moved
the numbers; five did not.

**The recommendation has survived all eight. Two versions of the *argument*
for it did not**, and §3 records both deaths rather than quietly replacing
them, because the failure generalises: a claim of the form *"X is the largest
step"* is a claim about the **shape** of a curve, and the shape moves with
the population. A claim of the form *"64 beats 32, everywhere"* is about
**ordering**, and the ordering has not moved once. Prefer the ordering.

**What is stable and what drifts is now a table in §0a**, so the next reader
can tell which figures to trust without re-deriving any of it.

Supersedes nothing. Reads forward from `predation_probe`'s pre-flight
(`creature-evolution-plan.md` §5 / `creature-review-2026-08.md` §T6), whose
numbers are quoted rather than re-derived.

---

## 0. The recommendation, first

**Build it at radius 64, all-round, seeing over the litter.** Concretely,
four decisions:

| decision | build it as | because |
|---|---|---|
| **reach** | **64 cells** | **64 delivers more than 32 at every preset, on median and p10 alike, on all eight trees** — currently +0.056 to +0.100 of median and +0.067 to +0.200 of p10. An ordering, not a superlative: see §3 for the two superlatives that died |
| **shape** | **all-round**, not a forward cone | a ±60° cone throws away **a third of every sighting** and saves nothing measurable |
| **what stops it** | rock and soil, **never floor litter** | at head height, floor clutter blocks **24%** of sight lines; one cell higher, **6.3%** — the whole transparent-world ceiling recovered, on all eight trees |
| **foliage** | **not a binary blocker** | making plant matter opaque costs **most of the sense** (r64 median 0.667 → 0.387) and no eye height buys it back |

**And it is free at this scale.** A radius-64 fan of 16 rays, cast at the
beetle's own `tick_interval`, reads **525 cells per beetle per cast** and
costs **0.005–0.007 ms of a frame** — **0.14–0.23%** of the 3.16 ms mean
`ascii` reports, and below what a wall clock can resolve. It stays under 1%
of a frame to a few dozen predators and under 10% to **two or three
hundred**, which is the number to carry into a streamed world. §5 says why
that is a range rather than a figure.

### 0b. It was built, and here is how the prediction landed

**`creature-sight-sense-2026-08-30.md` is the build of this report**, shipped
the same day: `PreyNear` and `PreyBearing`, a 16-ray fan at radius 64 from one
cell above the head, `sight_range: 64` authored on the beetle. Every
recommendation in §0 was taken.

**The perception prediction transferred.** This report predicted 0.572 of
samples with prey in sight; the built sense reads **0.50** over 8 generated
seeds — the right side of a coin flip, from geometry measured before anything
existed. Pursuit then moved two independent far-side counters together: mean
sighted range **15.2 → 12.5 cells** and prey caught **302 → 323**.

**The cost prediction was wrong by a factor of two, and §5 is corrected by
the build rather than by me.** The shipped sense reads **1,020–1,100 cells
per cast against the 525 priced here** — because a real implementation must
test prey in the *un-lifted* frame and blockers in the *lifted* one, so it
walks the fan twice where this probe walked it once. That is a modelling gap
in `cast_fan`, not a measurement error: the harness priced the geometry it
traced, and the engine needs a slightly different geometry. **It is still
0.3% of a frame**, so the conclusion survives — but a reader sizing a
*different* sense off §5's table should double it for the same reason.

Two findings from the build worth knowing before trusting anything here about
what a beetle *does* with a sighting, as opposed to what it can see:
`BrainOutput::Turn` is nearly inert for a surface walker on level ground
(filed as `open-bugs-handoff.md` §R4 — a movement finding, not a perception
one), and the obvious "did it move closer this tick" effect counter **cannot
fire on the ticks the sense exists for** and falls where catches rise.

### 0a. How stale is this, and what to re-take

`main` moves fast enough that this report was re-measured **eight times**
while it was being written. Rather than leave the next reader guessing which
figures survived, here is what eight trees actually showed.

| finding | stability |
|---|---|
| **eye=1 recovers the whole transparent-world ceiling on `wetland`** | **never moved, 8/8.** The most robust thing here by a wide margin |
| **radius 64 beats 32 — median and p10, every preset** | **never moved, 8/8.** The ordering is stable where the values are not |
| the cost conclusion (free at this scale) | **never moved**, at either end of a per-read cost that itself spans 13.8–22.1 ns |
| `dense` costs roughly half the sense | never moved |
| a ±60° cone costs a fifth to a third | never moved |
| absolute `los` medians | **drift.** `wetland` r8 has read 0.383, 0.283, 0.360 |
| absolute blocking percentages | **drift.** `wetland` has read 28.1%, 24.3%, 24.3% |
| the blocker census composition | **drifts with the floor.** Litter has run 21% → 10% of blockers as plant work landed |
| *"32 → 64 is the largest single step"* — median form | **was true, then false** (tree 5) |
| *"...largest single step in the p10"* — corrected form | **was true, then false** (tree 8) |

**The two dead claims are the lesson, not an embarrassment.** Both were
superlatives, and a superlative describes the *shape* of a curve, which moves
with the population. Both survived one landing and died on the next. The
ordering underneath them — more at 64 than at 32, everywhere — has survived
all eight. **When a finding has to outlive a moving world, state it as an
ordering.**

**So: trust the orderings and the recommendations; re-take any absolute
percentage you intend to quote.** One command, three minutes:

```
cargo build --release --example vision_probe   # NOT --release alone
./target/release/examples/vision_probe mode=survey seeds=18 preset=wetland
```

**A re-take is worth it when `main` has touched `src/sim/creature.rs`,
`organism.rs`, `plant.rs`, `world.rs`, `assets/species/*.ron` or
`src/worldgen/`** — the population, the floor, or the terrain. It is not worth
it for anything else; eight trees say a landing touching none of those moves
nothing at all. **Three of the eight moved the numbers materially: two
plant-side and one creature-side** — the floor and the population, which are
exactly the two things a ground-level sight line runs into. A hit on the rule
is not evidence of movement, though: five landings tripped it and came back
byte-identical.

**What this does not say.** Whether a beetle that can see an ant will catch
one. That is movement and brain work; `predation_probe`'s control already
settled that the kill itself works at contact range. This sizes the sense
and nothing downstream of it.

---

## 1. What was already known, and is not re-derived here

From `predation_probe`, re-run this week and quoted as-is:

- channel-B pheromone mass **294** over **33** nonzero cells in an
  81,920-cell world;
- **77%** of ants stand within a sensor offset of a nonzero cell, against
  **32%** of beetles;
- mean beetle → nearest nonzero cell **46 cells**, against a **6**-cell
  sensor span;
- the beetle's two sensor reads differ **1.3%** of the time, `|along|`
  **0.0067** — there is no gradient anywhere a beetle stands;
- **the kill works.** Under a saturated control a hungry beetle beside an ant
  feeds, ant cells 24 → 22 → 21. **The search is what fails.**

And the structural fact that makes E15 a direction rather than a preference:
**no sensor in `brain::BrainInput` reports another organism at a distance at
all.** `FoodAdjacent` and `AtNest` are contact-range; the two pheromone
planes are the only distal sense, and both are measured failing above.

**One quantity in that list is easy to misread into this report and must not
be.** The 46 cells is the distance from a beetle to the nearest *pheromone
cell*. The distance from a beetle to the nearest *ant* is a different
quantity over a different population, measured here for the first time:
median **55.2** cells on `wetland`, with a per-seed range of 19.2 to 122.2.

---

## 2. The instrument, and the two ways it is shown not to be lying

`examples/vision_probe.rs`. It rebuilds `predation_probe`'s scene — which is
`creature_space`'s — because a pre-flight measured on a different world than
the null it explains explains nothing. For each beetle head at each sampled
frame it asks, of every live ant head:

- **`range`** — is it within radius? *"There was something to see."*
- **`los`** — and is the Bresenham line between the two heads unobstructed?
  *"It could actually be seen."*
- **`cone`** — and is that line inside ±60° of the beetle's heading?

**`range` and `los` are the same pair a counter and its effect counter
always are**, and the gap between them is the only thing that can tell a
*reach* failure from an *occlusion* failure. A probe printing `los` alone
could report 0.02 and could not say which of two entirely different designs
that argues for.

### 2a. It can report both answers, on scenes whose answers are known

**Sensitivity and specificity are different checks**, and this repo has paid
six times for conflating them: a number that stays quiet when nothing is
wrong has not been shown to move when something is. `mode=control`, three arms from one builder so nothing but the thing under
test differs between them:

```
           arm |   rng8  rng16  rng32  rng64 |   los8  los16  los32  los64 |       d  blocked
         clear |  0.100  0.200  0.800  1.000 |  0.100  0.200  0.800  1.000 |    24.7        0
        walled |  0.100  0.200  0.700  1.000 |  0.000  0.000  0.000  0.000 |    26.3       50
  out of range |  0.000  0.000  0.000  0.000 |  0.000  0.000  0.000  0.000 |   123.7        0
```

- **`clear`** — a bare stone floor. `los` equals `range` at every radius and
  reaches **1.000** at r64. The sight test is not stuck at zero.
- **`walled`** — the same ants at the same distances behind a full-height
  stone slab. **`range` is unchanged at 1.000 and `los` is 0.000.** The
  occlusion test is not stuck on, and it is not quietly deleting the ants.
- **`out of range`** — the same ants at 100 cells. Both zero at r64, nearest
  distance 123.7. The radius test is a radius test.

Plus three assertions on the tracer itself before any scene exists: a line
through empty world is clear, a line through one stone cell is blocked *and
names stone*, and a line whose **endpoints** are both stone is clear — or
every sighting would fail for the trivial reason that prey is made of prey.

**The first draft of `clear` asserted the wrong thing and the control caught
it**, which is the only reason it is worth writing down: it asserted
`los == 1.000` at every radius, and `rng8` reads **0.100** because the
animals *walk* — the ant placed 4 cells away is inside r8 in a tenth of the
samples. The claim the arm actually supports is `los == range`, which is
tighter, not looser.

### 2c. The probe drives the same frame order the game does

Checked 2026-08-30, when `src/sim/frame.rs` landed as the single canonical
copy of the tick sequence. This probe does not call `frame::step` — it drives
the four phases it needs directly, as `predation_probe` does:

```
parallel::step  →  step_active_sites  →  step_fields  →  step_pheromones
```

which is `frame::step`'s relative order for those four. What it omits — rigid
bodies, the player, particles — cannot act in a scene with no gnome, no
detached body and no particle. **Worth re-checking whenever `frame.rs`
changes**, because a probe running a different phase order from the game is
measuring a world nobody plays, and now that the order has one canonical home
that check is a two-minute read rather than an archaeology exercise.

### 2b. The trap that does not apply here, said out loud

`CLAUDE.md` records the coarse-field degeneracy hit **four times on three
different lines**: a `FIELD_SCALE` read is block-nearest, so two sensors a
cell apart land in the same block roughly seven times in eight and their
difference is a constant zero. **Nothing in this file reads a field.** Every
read is `World::get` at CA resolution, and the quantity is not a difference
of two samples at all — it is a boolean over a traced line. There is no pair
of reads to be degenerate. This is stated rather than assumed because
"should not apply" is exactly what the previous four believed.

---

## 3. How far a sight line has to reach

`mode=survey`, 18 world seeds, 3,000 frames, sampled every 100, `occl=opaque
eye=0 cone=±60°`. Fractions are over **beetle samples** — one beetle at one
sampled frame. **Order statistics over seeds, never a mean.**

`wetland` — the scene the null was measured in:

| | min | p10 | **median** | p90 | max |
|---|---|---|---|---|---|
| `range` r8 | 0.000 | 0.000 | 0.360 | 0.667 | 0.683 |
| `range` r64 | 0.000 | 0.500 | 0.667 | 0.800 | 1.000 |
| **`los` r8** | 0.000 | 0.000 | **0.360** | 0.633 | 0.667 |
| **`los` r16** | 0.000 | 0.000 | **0.492** | 0.667 | 0.717 |
| **`los` r32** | 0.000 | 0.267 | **0.500** | 0.667 | 0.742 |
| **`los` r64** | 0.000 | 0.467 | **0.593** | 0.742 | 0.889 |
| `cone` r64 | 0.000 | 0.156 | 0.467 | 0.567 | 0.644 |
| beetle → nearest ant | 18.1 | 26.2 | 55.4 | 71.2 | 109.5 |
| ...nearest ant it can *see* | 2.0 | 4.0 | 18.8 | 31.2 | 54.5 |

Median `los`, three presets, 18 seeds each:

| preset | r8 | r16 | r32 | **r64** | p10 r32 | **p10 r64** | pairs blocked |
|---|---|---|---|---|---|---|---|
| `wetland` | 0.360 | 0.492 | 0.500 | **0.593** | 0.267 | **0.467** | 24.3% |
| `rolling` | 0.356 | 0.400 | 0.444 | **0.500** | 0.222 | **0.322** | 21.1% |
| `arid` | 0.247 | 0.300 | 0.353 | **0.453** | 0.260 | **0.327** | 8.8% |

### The radius argument, stated on the one footing that has held

**Build at 64. The claim is an ordering, not a superlative, and that
distinction is the hard-won part of this section.**

> **At every preset, on every tree measured, radius 64 delivers more than
> radius 32 — on the median and on the p10 alike.** On this tree the median
> gains +0.093 / +0.056 / +0.100 and the p10 gains +0.200 / +0.100 / +0.067.
> The sign has never once gone the other way. And §5 measures the extra reach
> at **0.14–0.23% of a frame**, so nothing trades against it.

**Two earlier versions of this argument used a superlative, and both were
falsified by a later tree.** They are recorded rather than quietly replaced,
because the failure has a lesson in it:

| version | claim | how it died |
|---|---|---|
| first | *"32 → 64 is the largest single step at every preset"* | tree 5: on `wetland` the **median**'s largest step became 8 → 16 |
| second | *"...the largest single step **in the p10** at every preset"* | tree 8: on `wetland` and `rolling` the p10's largest step became 16 → 32 |

**The lesson: the curve's *shape* moves with the population; its *ordering*
does not.** A superlative is a claim about shape and it has a one-in-three
chance of surviving the next landing. "More at 64 than at 32, everywhere,
always" is a claim about ordering, and eight trees have not dented it. Prefer
the ordering — it is what the recommendation actually needs, and it is the
half that is true.

**And be honest that the case has weakened as the world changed.** On tree 5
the p10 at r32 ran 0.08–0.28 and the stranded beetle was blind five times in
six on two presets; on tree 8 it runs 0.22–0.27 and that framing is no longer
available. What survives is the plainer statement: **64 is strictly better
everywhere and costs nothing measurable**, which is sufficient but is not the
dramatic gap it once was. If a future tree ever puts r32's p10 above r64's,
this recommendation should be re-opened — nothing else here would need to
change.

**Two further readings, both stable across trees:**

- **A short sense is a contact sense with extra steps.** At r8 the median runs
  0.25–0.36 and the beetle is nearly on top of the ant; the median *visible*
  ant is 19 cells away, so a radius under that discards the sightings that
  actually happen.
- **The curve is still climbing at 64.** The median beetle sits 55–64 cells
  from the nearest ant. Where 128 lands is **not measured** and is the honest
  gap in this table.

**A ±60° cone costs a fifth to a third of everything** (r64 median 0.593 →
0.467, 0.500 → 0.387, 0.453 → 0.300) for a saving §5 shows is not worth
having. Build it all-round.

### 3a. The placement confound, ruled out rather than argued away

Beetles are stood up at x = 40 + 45i and ants from x = 24 upward, so at frame
0 **every beetle is standing inside the colony** and a short-radius sighting
would be a statement about the placement loop. `settle=` re-asks the question
of a dispersed population by skipping the first 3,000 frames and sampling the
next 3,000. Median `los` on `wetland`:

| | r8 | r16 | r32 | r64 |
|---|---|---|---|---|
| from frame 0 | 0.383 | 0.450 | 0.467 | 0.572 |
| from frame 3,000 | 0.358 | 0.483 | 0.500 | 0.622 |

Within the seed spread everywhere, and slightly *higher* at long radius.
**The placement is not driving the result.** (Taken on the second of the eight
trees §0 names and not re-run since; it is a qualitative check that two ways
of sampling agree, and that conclusion is insensitive to the drift the later
trees produced.)

---

## 4. What terrain costs, and what is actually doing the blocking

`mode=occlusion`, median `los` over 18 seeds on `wetland`. `none` is the
**transparent-world ceiling** rather than a setting, and every other row is
asserted at or below it in the harness — an arm above its own ceiling is an
arithmetic bug, so the check is free and runs every time.

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.360 | 0.500 | 0.500 | **0.667** | 0.0% |
| `opaque` | 0 | 0.360 | 0.492 | 0.500 | **0.593** | 24.3% |
| **`opaque`** | **1** | 0.360 | 0.500 | 0.500 | **0.667** | **6.3%** |
| `opaque` | 3 | 0.360 | 0.500 | 0.500 | 0.622 | 4.1% |
| `dense` | 0 | 0.320 | 0.373 | 0.387 | **0.387** | 78.3% |
| `dense` | 3 | 0.360 | 0.400 | 0.400 | 0.420 | 62.3% |
| `all` | 0 | 0.320 | 0.373 | 0.387 | 0.387 | 78.8% |

**Terrain relief is not the problem. Floor clutter is.** What stops the rays,
pooled over 18 seeds:

| preset | what stopped them |
|---|---|
| `wetland` | seed 31%, soil 24%, corpse 19%, basalt 12%, litter 10%, gravel 3% |
| `rolling` | soil 28%, seed 23%, basalt 20%, corpse 16%, litter 7%, gravel 3% |
| `arid` | corpse 48%, basalt 30%, seed 22% |

Both animals are ground-hugging, so a sight line between two heads grazes the
floor for its whole length and a two-cell seed pile stops a forty-cell line.

**One cell of eye height recovers the entire transparent-world ceiling on
`wetland`, and it has done so on all eight trees.** `opaque eye=1` reads 0.667
at r64 — identical to a world with nothing in it — at 6.3% blocking against
24.3%. **This is the most stable finding in the report by a wide margin**: the
absolute percentages have drifted with every landing on `main`, and this
recovery has not moved once.

**On a second preset it removes most but not all.** `rolling`, 12 seeds:

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.373 | 0.433 | 0.500 | **0.600** | 0.0% |
| `opaque` | 0 | 0.373 | 0.433 | 0.444 | **0.500** | 22.1% |
| `opaque` | 1 | 0.373 | 0.433 | 0.500 | **0.581** | 8.7% |
| `opaque` | 3 | 0.373 | 0.433 | 0.500 | 0.581 | 6.5% |
| `dense` | 0 | 0.360 | 0.429 | 0.429 | **0.444** | 50.9% |
| `dense` | 3 | 0.360 | 0.425 | 0.438 | 0.452 | 38.2% |
| `all` | 0 | 0.360 | 0.429 | 0.429 | 0.444 | 50.9% |

Blocking falls the same way (22.1% → 8.7%) but r64 recovers to 0.581 of a
0.600 ceiling — **about 80% of the gap, not the whole of it**. `rolling` has
real relief, so some of what stops a line there is landscape rather than
clutter, which the blocker census agrees with: bare rock is 20% of
`rolling`'s blockers against 12% of `wetland`'s. **Eye height is still the
setting to build; it is not a complete fix on hilly ground.**

**The owner was asked and declined to pick, so this is settled on the
measurement.** Card `20260830T021057007Z-18900e` put the two eye heights side
by side as a labelled A/B and asked which reads right for an insect on a
forest floor. The verdict, 2026-08-30: *"I don't think there is a clear good
answer. Just pick one that makes sense to you."* So **eye=1 is recommended on
the numbers above and nothing else**. A later playtest may overturn it, and
if it does the thing to change is one parameter rather than the model — part
of why the eye is a knob here and not baked in.

**`eye=3` is not better than `eye=1`, and this is the row not to smooth
over.** Its pooled blocking is lower (4.1% against 6.3%) while its median
`los` at r64 is *worse* (0.622 against 0.667). The two columns are different
statistics — pooled pairs are dominated by the seeds carrying the most pairs,
the median is per-seed — and they genuinely disagree. Nothing in this study
explains it; **do not read the pooled column as ranking eye heights.** It is
`wetland`-only: on `rolling`, eye=1 and eye=3 give the identical 0.581.

**Foliage is the biggest lever in the table, and bigger than radius.**
Making plant matter opaque takes r64 from 0.667 to **0.387** on `wetland` and
from 0.600 to **0.444** on `rolling`, and eye height recovers only a fraction
(0.420 / 0.452 at eye=3). Two things follow. First, if E15 wants the sense to
work at all, **`dense` is not a shippable setting as a binary rule.** Second,
this is the ethos law rather than a tuning note: *an outcome is a
distribution, not a binary*. A canopy that either passes sight perfectly or
blocks it perfectly has the same defect the old rubble had; what a bush should
do is *attenuate* — shorten the effective radius through it — which this
study did not price and the next lane should.

Water is a non-question: `all` differs from `dense` by half a point of
blocking and nothing at all in median `los`.

---

## 5. What it costs per frame

`mode=cost`. The implementation priced is the one a sensor would actually
use: a **fan of 16 rays** swept over the circle, each marched outward until
it hits something or reaches the radius, cast once every 8 frames — the
beetle's own `tick_interval`. Its cost is a function of radius and ray count
and **not** of how many prey exist, which is what makes it the shippable
shape.

**Three arms, not two, and the middle one is the whole point.** `cast_fan`
has to find the beetles before it can cast from them, and this harness finds
them by scanning all 81,920 cells — which an engine implementation never
does, because the active-site scheduler dispatches a creature at its own
position. `locate` is that scan alone.

```
       arm     ms/frame     vs blind    vs locate     cells read  per beetle/cast
     blind       3.0407            -            -              0                -
    locate       3.2280            -            -       30720000              5.0
    locate       3.1743            -            -       30720000              5.0
    locate       3.2062            -            -       30720000              5.0
    locate       3.2984            -            -       30720000              5.0
    locate       3.3003            -            -       30720000              5.0
     blind       3.1217            -            -              0                -
        r8       3.2067       0.1255      -0.0347         173876               93
       r16       3.2336       0.1524      -0.0078         304607              162
       r32       3.1902       0.1090      -0.0512         549850              293
       r64       3.2115       0.1303      -0.0299         984427              525
```

blind spread **0.081 ms**, locate spread **0.126 ms**, arms alternating,
every arm asserted to have started from a byte-identical world.

**Read `vs locate`, never `vs blind`.** The `vs blind` column is almost
entirely this harness's own whole-world scan; a reader taking it for the
sense's cost would be off by a factor of thirty.

**The wall clock cannot resolve the sense, and this is now seven runs rather
than an argument.** Every `vs locate` here is inside the control spread, and
all four are *negative* — a sense that made the frame faster. Across seven
runs on six trees, r64 has read **−0.012, +0.059, +0.029, −0.015, −0.003,
−0.040 and −0.030 ms** against control spreads of 0.046 to 0.126. The sign
flips four times and never leaves the noise bar.

**The deterministic route is the one to quote.** `cells read` at r64 has run
909,763 → 898,619 → 977,415 → 984,427 as the world changed; beetles located
per cast is 5.0 on every tree. `locate` prices one `World::get` directly, and
**that price is the loosest number here and is quoted as a range**: six
readings give **15.6, 13.8, 14.9, 22.1, 16.4 and 15.6 ns**. The 22.1 came
from a run whose control spread was twice the others'. Taking the range:

| radius | cells read per beetle per cast | ms/frame at 5 beetles | µs per beetle per frame |
|---|---|---|---|
| 8 | 93 | 0.0008–0.0013 | 0.16–0.26 |
| 16 | 162 | 0.0014–0.0022 | 0.28–0.45 |
| 32 | 293 | 0.0025–0.0040 | 0.51–0.81 |
| **64** | **525** | **0.0045–0.0073** | **0.91–1.45** |

**The conclusion does not depend on which reading you take**: at the
optimistic end a radius-64 sense is 0.14% of a frame, at the pessimistic end
0.23%. Both are free.

**Against the whole frame.** `ascii` on this tree reports **mean 3.161 ms**
over 12,000 frames with 143 live organisms (worst 51.773 ms). Per
`CLAUDE.md`'s test the worst is **not** pinned by an aggregate — mean ×
frames is 37,932 ms against a 52 ms worst — so it is one frame among
thousands of comparable ones and is noise wearing a number. The mean is the
figure to use; this harness's blind arm at 3.08 ms agrees with it to 3%.

**What it costs at scale** — the current 512x320 world is a test environment,
not the target. At 0.9–1.5 µs per beetle per frame against a ~3.2 ms frame:

| predators in the world | cost of the sense | share of a frame |
|---|---|---|
| 5 (measured) | 0.005–0.007 ms | 0.14–0.23% |
| a few dozen | ~0.03 ms | ~1% |
| **two to three hundred** | **~0.3 ms** | **10%** |

Round numbers deliberately: the per-read cost spans 13.8–22.1 ns across six
measurements and the frame itself has moved 2.82–3.16 ms across five, so a
three-digit predator count would be false precision. **The honest claim is "a
few hundred", and nothing plausible for this world sits near that bound.**

**Radius buys itself cheaply, and that is not an assumption.** 8 → 64 is an
eightfold radius for a **five-and-a-half-fold** read count (93 → 525), well
short of the 16 × 64 = 1,024 an unobstructed fan would cost, because rays
terminate on the first blocker. **Occlusion makes the sense cheaper as well
as weaker** — demonstrated directly on tree 5, where a world with less litter
cost 9% more to look at.

**And the built sense costs about twice this.** §0b has the detail: the
shipped implementation reads 1,020–1,100 cells per cast against the 525
priced here, because it must test prey in the un-lifted frame and blockers in
the lifted one — two walks of the fan where this probe made one. Still 0.3%
of a frame, so nothing in the recommendation changes, but **double this
table before sizing a different sense off it.**

**Two guards on all of the above**, because a cost that vanishes may be work
that vanished: `cells read` is asserted nonzero on every sighted arm and
comes back from the far side of the call that does the casting, and the
`locate` arm is asserted to have actually found a beetle.

---

## 6. What this cannot answer

- **It is geometry.** Whether a beetle *acts* on a sighting is brain and
  movement work. `predation_probe`'s control already says the kill works at
  contact range, so what this sizes is the gap between those two facts.
- **One world size, one colony size.** 512x160, 52 ants placed, 9 beetles
  placed and **3 to 6 alive** by the time the world settles. Ant clumping
  sets the entire shape of §3's curve; a denser or more dispersed colony
  moves every number in it. Three presets agree on the shape, which is
  evidence but not a proof of transfer.
- **Radius 128 is not measured.** The curve is still climbing at 64 and the
  median beetle is 55–65 cells from prey, so 128 would deliver more. Whether
  a sense that is on essentially all the time is still a *search* cue is a
  design question this study cannot referee.
- **It is a duty cycle, not a per-tick figure.** Sampled every 100 frames
  against a beetle that decides every 8, so it observes about one tick in
  twelve. It says how often a sense would have something to report, not what
  it reports on any given tick.
- **The eye is a knob, not a model.** `eye=` lifts the endpoint through
  non-blocking cells only. Where a real animal's eye sits, and whether the
  ant's own head is the right thing to aim at, are design choices.

---

## 7. What the next lane builds against

1. A distal sense with reach **64**, all-round, dispatched from the
   active-site scheduler at the creature's own position — **never** by
   scanning for creature cells, which is what §5's `locate` arm exists to
   keep out of the cost.
2. Occlusion that reads rock and soil and **passes floor litter** — the
   cheapest expression of which is the eye sitting one cell above the head,
   since that recovers the transparent-world ceiling without a new rule.
3. Foliage attenuating rather than blocking, if it is to matter at all.
4. A `BrainInput` pair in the same shape as the existing distal senses, so
   the brain can steer on it — and a re-run of `predation_probe mode=ab`
   afterwards, because *the* test of E15 is whether `beetles=0` and
   `beetles=9` stop running bit-identical.

**The sense is not the mechanism.** The measured null is that a predator
moves no counter; a beetle that can see an ant and does nothing about it
will still move no counter. §5's cost figure is the licence to build this,
not the evidence that it works.
