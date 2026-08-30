# Sizing a sight sense before it is built — E15

**Status: measured pre-flight, 2026-08-30, on `e7b72e7` + `examples/vision_probe.rs`,
and re-taken in full three times as `main` landed underneath it — the
worldgen revamp (716 lines of `passes.rs`, five new rock materials),
tree-breaking (355 lines of `plant.rs`, which changes what lies on the floor),
and the creature-economy rework (`ant.ron`, `beetle.ron`, `creature.rs`,
`organism.rs` — which changes where the animals stand). Each could plausibly
have moved the result, so every number here was measured on **four different
trees**.

**The first three were byte-identical. The fourth moved, and only in the third
decimal.** The creature rework perturbs individual seeds — no surprise, it
changes the population — but **every median and every p10 the recommendation
rests on is unchanged**, on all three presets, as is every median in §4. What
moved: `arid`'s median at r64 (0.440 → 0.420) and its p10 at r32 (0.260 →
0.280), a handful of p90s and maxima, blocking percentages by a tenth of a
point, and the frame timing, which §5 now handles as a range rather than a
number. **That the first three runs were identical is what makes the fourth
informative**: the instrument does move when the world does.**
Answers the question the owner's E15 authorisation left open. Nothing here
changes behaviour: the instrument is read-only geometry over `World::get`,
and no vision is implemented on this branch.

Supersedes nothing. Reads forward from `predation_probe`'s pre-flight
(`creature-evolution-plan.md` §5 / `creature-review-2026-08.md` §T6), whose
numbers are quoted rather than re-derived.

---

## 0. The recommendation, first

**Build it at radius 64, all-round, seeing over the litter.** Concretely,
four decisions:

| decision | build it as | because |
|---|---|---|
| **reach** | **64 cells** | the bad-seed beetle sees prey a sixth of the time at 32 and **two fifths** at 64; 32 → 64 is the largest single step at every preset measured |
| **shape** | **all-round**, not a forward cone | a ±60° cone throws away **a third of every sighting** and saves nothing measurable |
| **what stops it** | rock and soil, **never floor litter** | at head height, floor clutter blocks **28%** of sight lines; one cell higher, **8.4%**, which is the whole of the transparent-world ceiling recovered |
| **foliage** | **not a binary blocker** | making plant matter opaque costs **half the sense** (r64 median 0.667 → 0.350) and no eye height buys it back |

**And it is free at this scale.** A radius-64 fan of 16 rays, cast at the
beetle's own `tick_interval`, reads **479 cells per beetle per cast** and
costs **~0.005 ms of a frame** — **0.15–0.22%** of the 2.94 ms mean `ascii`
reports, and below what a wall clock can resolve. It stays under 1% of a
frame to a few dozen predators and under 10% to **two or three hundred**,
which is the number to carry into a streamed world. §5 says why that is a
range rather than a figure.

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
sampled frame. **Order statistics over seeds, never a mean**: six seeds is
not a sweep and a mean over a chaotic population is a number no design can
be sized from.

`wetland` — the scene the null was measured in:

| | min | p10 | **median** | p90 | max |
|---|---|---|---|---|---|
| `range` r8 | 0.000 | 0.022 | 0.389 | 0.667 | 0.675 |
| `range` r64 | 0.000 | 0.467 | 0.667 | 0.917 | 0.989 |
| **`los` r8** | 0.000 | 0.022 | **0.383** | 0.650 | 0.667 |
| **`los` r16** | 0.000 | 0.089 | **0.450** | 0.667 | 0.708 |
| **`los` r32** | 0.000 | 0.156 | **0.467** | 0.667 | 0.742 |
| **`los` r64** | 0.000 | 0.389 | **0.572** | 0.817 | 0.911 |
| `cone` r64 | 0.000 | 0.156 | 0.400 | 0.589 | 0.617 |
| beetle → nearest ant | 19.2 | 25.1 | 55.2 | 68.2 | 122.2 |
| ...nearest ant it can *see* | 2.3 | 3.6 | 16.5 | 32.0 | 54.4 |

**And it is not one scene.** `preset=` was added for exactly the reason
"six seeds is not a sweep" exists — measured on one preset, this would be a
statement about `wetland`. Median `los`, 18 seeds each:

| preset | r8 | r16 | r32 | **r64** | p10 r32 | **p10 r64** | pairs blocked |
|---|---|---|---|---|---|---|---|
| `wetland` | 0.383 | 0.450 | 0.467 | **0.572** | 0.156 | **0.389** | 28.0% |
| `rolling` | 0.327 | 0.380 | 0.393 | **0.500** | 0.108 | **0.240** | 23.5% |
| `arid` | 0.240 | 0.293 | 0.347 | **0.420** | 0.280 | **0.347** | 8.6% |

**Three readings, and the third is the one that sets the radius.**

1. **A short sense is a contact sense with extra steps.** At r8 the median
   runs 0.24–0.38 and the beetle is already almost on top of the ant; the
   median *visible* ant is 16 cells away, so a radius under that is throwing
   away the sightings that actually happen.
2. **The curve does not flatten.** 32 → 64 is the largest single step at
   every preset (+0.105, +0.107, +0.073) and it is still climbing at 64 —
   unsurprising, since the median beetle is 55–65 cells from the nearest ant.
   Where 128 lands is **not measured**, and is the honest gap in this table.
3. **The p10 seed is where the decision is.** The house rule is to gate an
   order statistic, not a median, and the p10 beetle — the one stranded away
   from the colony, which is precisely the animal a distal sense exists for —
   goes from **0.156 / 0.108 / 0.280** at r32 to **0.389 / 0.240 / 0.347** at
   r64. At 32 that beetle is blind five times in six on two of the three
   presets. **That is the case for 64 and it is not a median argument.**
   `arid` is the weak leg and is worth naming: its p10 gains only 0.280 →
   0.347 and its median 0.347 → 0.420. It is also the preset with almost no
   occlusion (8.6% of pairs blocked), so what limits it is distance alone —
   its beetles sit 47–81 cells from the nearest ant, a much tighter spread
   than the other two presets produce.

**A ±60° cone costs a third of everything**, at every preset (r64 median
0.572 → 0.400, 0.500 → 0.360, 0.440 → 0.307), for a saving that §5 shows is
not worth having. Build it all-round.

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
**The placement is not driving the result.** (This control was taken on the
second of the four trees and not re-run since; it is a qualitative check that
two ways of sampling agree, and the figures either side of it moved by
thousandths on the trees that followed.) Occlusion does rise with time —
28.1% → 38.2% of pairs blocked, with litter climbing from 21% to 36% of
blockers — because litter accumulates on the floor, which is §4's finding
arriving by a second route.

---

## 4. What terrain costs, and what is actually doing the blocking

`mode=occlusion`, median `los` over 18 seeds on `wetland`. `none` is the
**transparent-world ceiling** rather than a setting, and every other row is
asserted at or below it in the harness — an arm above its own ceiling is an
arithmetic bug, so the check is free and runs every time.

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.389 | 0.450 | 0.500 | **0.667** | 0.0% |
| `opaque` | 0 | 0.383 | 0.450 | 0.467 | **0.572** | 28.0% |
| **`opaque`** | **1** | 0.389 | 0.450 | 0.500 | **0.667** | **8.4%** |
| `opaque` | 3 | 0.389 | 0.450 | 0.500 | 0.613 | 4.9% |
| `dense` | 0 | 0.272 | 0.322 | 0.328 | **0.350** | 78.8% |
| `dense` | 3 | 0.272 | 0.350 | 0.367 | 0.422 | 63.8% |
| `all` | 0 | 0.272 | 0.322 | 0.328 | 0.350 | 79.4% |

**Terrain relief is not the problem. Floor clutter is.** What stops the rays,
pooled over 18 seeds:

| preset | what stopped them |
|---|---|
| `wetland` | seed 25%, litter 21%, soil 18%, corpse 17%, basalt 13%, deadwood 4% |
| `rolling` | seed 25%, soil 20%, basalt 19%, litter 15%, corpse 14%, deadwood 4% |
| `arid` | corpse 45%, basalt 34%, seed 21% |

Both animals are ground-hugging, so a sight line between two heads grazes the
floor for its whole length and a two-cell seed pile stops a forty-cell line.
**One cell of eye height removes most of it**, and on `wetland` it removes
all of it: `opaque eye=1` reads 0.667 at r64, identical to the transparent
world, at 8.4% blocking against 28.0%.

**On a second preset it removes most but not all, and that correction is why
the second preset was run.** `rolling`, 12 seeds:

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.424 | 0.429 | 0.500 | **0.613** | 0.0% |
| `opaque` | 0 | 0.419 | 0.429 | 0.492 | **0.567** | 25.1% |
| `opaque` | 1 | 0.424 | 0.429 | 0.500 | **0.581** | 9.5% |
| `opaque` | 3 | 0.424 | 0.429 | 0.500 | 0.581 | 6.5% |
| `dense` | 0 | 0.373 | 0.380 | 0.400 | **0.429** | 51.4% |
| `dense` | 3 | 0.380 | 0.387 | 0.408 | 0.452 | 38.1% |
| `all` | 0 | 0.373 | 0.380 | 0.400 | 0.429 | 51.4% |

Blocking falls the same way (25.1% → 9.5%) but r64 recovers to 0.581 of a
0.613 ceiling — **about 70% of the gap, not the whole of it**. `rolling` has
real relief, so some of what stops a line there is landscape rather than
clutter, which the blocker census agrees with: stone is 19% of `rolling`'s
blockers against 13% of `wetland`'s. **Eye height is still the setting to
build; it is not a complete fix on hilly ground.**

**The owner was asked and declined to pick, so this one is settled on the
measurement.** Card `20260830T021057007Z-18900e` put the two eye heights side
by side as a labelled A/B — the same world and instant, sight lines drawn by
the tracer under test — and asked which reads right for an insect on a forest
floor. The verdict, 2026-08-30: *"I don't think there is a clear good answer.
Just pick one that makes sense to you."* So **eye=1 is recommended on the
numbers above and nothing else**: it is the setting that recovers the ceiling
on `wetland` and ~70% of the gap on `rolling`, at a third of the blocking. A
later playtest may overturn it, and if it does, the thing to change is one
parameter rather than the model — which is part of why the eye is a knob here
and not baked in.

**`eye=3` is not better than `eye=1`, and this is the row not to smooth
over.** Its pooled blocking is lower (4.9% against 8.4%) while its median
`los` at r64 is *worse* (0.613 against 0.667). The two columns are different
statistics — pooled pairs are dominated by the seeds carrying the most pairs,
the median is per-seed — and they genuinely disagree here. Nothing in this
study explains it; **do not read the pooled column as ranking eye heights.**
It is also `wetland`-only: on `rolling`, eye=1 and eye=3 give the identical
0.581 and only the pooled blocking moves.

**Foliage is the biggest lever in the table, and bigger than radius.**
Making plant matter opaque takes r64 from 0.667 to **0.350** on `wetland` and
from 0.613 to **0.429** on `rolling` — on `wetland`, worse than halving the
radius — and eye height recovers only a third of it (0.422 / 0.452 at eye=3). Two things follow. First, if E15 wants the sense to work at all,
**`dense` is not a shippable setting as a binary rule.** Second, this is the
ethos law rather than a tuning note: *an outcome is a distribution, not a
binary*. A canopy that either passes sight perfectly or blocks it perfectly
has the same defect the old rubble had; what a bush should do is *attenuate*
— shorten the effective radius through it — which is a mechanism this study
did not price and the next lane should.

Water is a non-question: `all` differs from `dense` by 0.7 points of
blocking and nothing at all in median `los`.

---

## 5. What it costs per frame

`mode=cost`. The implementation priced is the one a sensor would actually
use: a **fan of 16 rays** swept over the circle, each marched outward until
it hits something or reaches the radius, cast once every 8 frames — the
beetle's own `tick_interval`. Its cost is a function of radius and ray count
and **not** of how many prey exist, which is what makes it the shippable
shape; the alternative, testing every prey pairwise, needs a prey index the
engine does not have and scales with the colony.

**Three arms, not two, and the middle one is the whole point.** `cast_fan`
has to find the beetles before it can cast from them, and this harness finds
them by scanning all 81,920 cells — which an engine implementation never
does, because the active-site scheduler dispatches a creature at its own
position. Timing scan-plus-rays against nothing prices the harness, not the
design. `locate` is the scan alone.

```
       arm     ms/frame     vs blind    vs locate     cells read  per beetle/cast
     blind       2.8043            -            -              0                -
    locate       2.9846            -            -       30720000              5.0
    locate       3.0825            -            -       30720000              5.0
    locate       3.0167            -            -       30720000              5.0
    locate       3.0155            -            -       30720000              5.0
    locate       2.9791            -            -       30720000              5.0
     blind       2.7743            -            -              0                -
        r8       2.9858       0.1965      -0.0299         147501               79
       r16       3.0376       0.2483       0.0219         259794              139
       r32       3.0129       0.2236      -0.0028         480936              256
       r64       3.0124       0.2231      -0.0033         898619              479
```

blind spread **0.030 ms**, locate spread **0.103 ms** over 3,000 frames an
arm, arms alternating, every arm asserted to have started from a
byte-identical world.

**Read `vs locate`, never `vs blind`.** The `vs blind` column runs 0.20–0.25
ms and is almost entirely this harness's own whole-world scan; a reader
taking it for the sense's cost would be off by a factor of forty.

**The wall clock cannot resolve the sense at all, and this is now five runs
rather than an argument.** In this run every `vs locate` lands inside the
control spread (−0.030 to +0.022 against 0.103). Across five runs of this
mode, spanning four trees, r64 has read **−0.012, +0.059, +0.029, −0.015 and
−0.003 ms** against control spreads of 0.046 to 0.103. **The sign flips four
times.** A quantity that never leaves its own noise bar and cannot hold a
sign is noise.

**The deterministic route is the one to quote, and it is stable.** `cells
read` was bit-identical across the first three trees (909,763 at r64) and
moved 1.2% on the fourth, to **898,619** — the creature rework changed the
population slightly, which is exactly the kind of thing it should track.
Beetles located per cast is 5.0 on every tree.

`locate` reads every cell of the world once per cast and does nothing else,
so it prices one `World::get` directly. **That price is the loosest number in
this report and it is quoted as a range on purpose**: four readings give
**15.6, 13.8, 14.9 and 22.1 ns**, and the 22.1 came from the run above, whose
own control spread (0.103 ms) is twice any of the others — the same run whose
`ascii` worst frame read 78 ms against a 28 ms usual. So the box was loud, not
the code slow. Taking the whole range rather than picking:

| radius | cells read per beetle per cast | ms/frame at 5 beetles | µs per beetle per frame |
|---|---|---|---|
| 8 | 79 | 0.0007–0.0011 | 0.15–0.22 |
| 16 | 139 | 0.0013–0.0019 | 0.26–0.38 |
| 32 | 256 | 0.0024–0.0035 | 0.48–0.70 |
| **64** | **479** | **0.0045–0.0066** | **0.90–1.32** |

**The conclusion does not depend on which reading you take**, which is the
point of quoting the range: at the optimistic end a radius-64 sense is 0.15%
of a frame and at the pessimistic end 0.22%. Both are free.

**Against the whole frame.** `cargo run --release --example ascii` on this
tree reports **mean 2.937 ms** over 12,000 frames with 159 live organisms
(worst 78.303 ms). Per `CLAUDE.md`'s own test the worst is **not** pinned by
an aggregate — mean × frames is 35,244 ms against a 78 ms worst, 0.2% of it —
so that worst is one frame among thousands of comparable ones and is noise
wearing a number. It is quoted here only as corroboration that this
particular run was taken on a loaded box: the same run's `locate` spread
doubled. The mean is the figure to use, and this harness's blind arm at
2.79 ms agrees with it to 5%.

**What it costs at scale, which is the number that matters** — the current
512x320 world is a test environment, not the target. At 0.9–1.3 µs per beetle
per frame against a ~2.9 ms frame:

| predators in the world | cost of the sense | share of a frame |
|---|---|---|
| 5 (measured) | 0.005–0.007 ms | 0.15–0.22% |
| a few dozen | ~0.03 ms | ~1% |
| **two to three hundred** | **~0.3 ms** | **10%** |

Round numbers deliberately, and rounder than the last revision: the per-read
cost spans 13.8–22.1 ns across four measurements and the frame itself has
moved 2.82–2.98 ms across three, so any three-digit predator count would be
false precision. **The honest claim is "a few hundred", and it is the claim
the design needs** — nothing plausible for this world sits near that bound.

**Radius buys itself cheaply, and that is not an assumption.** 8 → 64 is an
eightfold radius for a **sixfold** read count (79 → 479), well short of the
16 × 64 = 1,024 an unobstructed fan would cost, because rays terminate on the
first blocker and a ground-hugging beetle's downward rays die at once.
**Occlusion makes the sense cheaper as well as weaker**, which is worth
knowing before anyone proposes to relax it for performance.

**Two guards on all of the above**, because a cost that vanishes may be work
that vanished: `cells read` is asserted nonzero on every sighted arm and
comes back from the far side of the call that does the casting, and the
`locate` arm is asserted to have actually found a beetle. An arm that timed
as free while probing nothing fails rather than publishing a bargain.

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
