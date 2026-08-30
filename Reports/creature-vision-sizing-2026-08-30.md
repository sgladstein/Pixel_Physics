# Sizing a sight sense before it is built — E15

**Status: measured pre-flight, 2026-08-30, on `e7b72e7` + `examples/vision_probe.rs`,
and **re-taken in full after merging `main`** — the worldgen revamp landed
underneath this work (716 lines of `passes.rs`, five new rock materials), so
every number here was measured twice, on two different trees. **Every order
statistic in §3 and §4 came back identical**; the only change anywhere is
that the base rock is now called `basalt` rather than `stone` in the blocker
census, and the pair counts move by a handful out of ~20,000.**
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
| **what stops it** | rock and soil, **never floor litter** | at head height, floor clutter blocks **28%** of sight lines; one cell higher, **8.5%**, which is the whole of the transparent-world ceiling recovered |
| **foliage** | **not a binary blocker** | making plant matter opaque costs **half the sense** (r64 median 0.667 → 0.350) and no eye height buys it back |

**And it is free at this scale.** A radius-64 fan of 16 rays, cast at the
beetle's own `tick_interval`, reads **485 cells per beetle per cast** and
costs **0.004 ms of a frame** — **0.14%** of the 2.98 ms mean `ascii`
reports, and below what a wall clock can resolve. It stays under 1% of a
frame to about **36** predators and under 10% to about **358**, which is the
number to carry into a streamed world.

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
| `range` r64 | 0.000 | 0.467 | 0.667 | 0.922 | 0.989 |
| **`los` r8** | 0.000 | 0.022 | **0.383** | 0.650 | 0.667 |
| **`los` r16** | 0.000 | 0.089 | **0.450** | 0.667 | 0.708 |
| **`los` r32** | 0.000 | 0.156 | **0.467** | 0.667 | 0.742 |
| **`los` r64** | 0.000 | 0.389 | **0.572** | 0.822 | 0.911 |
| `cone` r64 | 0.000 | 0.156 | 0.400 | 0.589 | 0.650 |
| beetle → nearest ant | 19.2 | 25.1 | 55.2 | 68.2 | 122.2 |
| ...nearest ant it can *see* | 2.3 | 3.6 | 16.4 | 32.0 | 54.4 |

**And it is not one scene.** `preset=` was added for exactly the reason
"six seeds is not a sweep" exists — measured on one preset, this would be a
statement about `wetland`. Median `los`, 18 seeds each:

| preset | r8 | r16 | r32 | **r64** | p10 r32 | **p10 r64** | pairs blocked |
|---|---|---|---|---|---|---|---|
| `wetland` | 0.383 | 0.450 | 0.467 | **0.572** | 0.156 | **0.389** | 28.1% |
| `rolling` | 0.327 | 0.380 | 0.393 | **0.500** | 0.108 | **0.240** | 23.3% |
| `arid` | 0.240 | 0.293 | 0.347 | **0.440** | 0.260 | **0.347** | 8.4% |

**Three readings, and the third is the one that sets the radius.**

1. **A short sense is a contact sense with extra steps.** At r8 the median
   runs 0.24–0.38 and the beetle is already almost on top of the ant; the
   median *visible* ant is 16 cells away, so a radius under that is throwing
   away the sightings that actually happen.
2. **The curve does not flatten.** 32 → 64 is the largest single step at
   every preset (+0.105, +0.107, +0.093) and it is still climbing at 64 —
   unsurprising, since the median beetle is 55–65 cells from the nearest ant.
   Where 128 lands is **not measured**, and is the honest gap in this table.
3. **The p10 seed is where the decision is.** The house rule is to gate an
   order statistic, not a median, and the p10 beetle — the one stranded away
   from the colony, which is precisely the animal a distal sense exists for —
   goes from **0.156 / 0.108 / 0.260** at r32 to **0.389 / 0.240 / 0.347** at
   r64. At 32 that beetle is blind five times in six on two of the three
   presets. **That is the case for 64 and it is not a median argument.**

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
**The placement is not driving the result.** Occlusion does rise with time —
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
| `opaque` | 0 | 0.383 | 0.450 | 0.467 | **0.572** | 28.1% |
| **`opaque`** | **1** | 0.389 | 0.450 | 0.500 | **0.667** | **8.5%** |
| `opaque` | 3 | 0.389 | 0.450 | 0.500 | 0.613 | 4.8% |
| `dense` | 0 | 0.272 | 0.322 | 0.328 | **0.350** | 78.7% |
| `dense` | 3 | 0.272 | 0.350 | 0.367 | 0.422 | 63.8% |
| `all` | 0 | 0.272 | 0.322 | 0.328 | 0.350 | 79.4% |

**Terrain relief is not the problem. Floor clutter is.** What stops the rays,
pooled over 18 seeds:

| preset | what stopped them |
|---|---|
| `wetland` | seed 25%, litter 21%, soil 18%, corpse 17%, basalt 13%, deadwood 4% |
| `rolling` | seed 25%, soil 20%, basalt 19%, litter 15%, corpse 13%, deadwood 3% |
| `arid` | corpse 45%, basalt 34%, seed 22% |

Both animals are ground-hugging, so a sight line between two heads grazes the
floor for its whole length and a two-cell seed pile stops a forty-cell line.
**One cell of eye height removes most of it**, and on `wetland` it removes
all of it: `opaque eye=1` reads 0.667 at r64, identical to the transparent
world, at 8.5% blocking against 28.1%.

**On a second preset it removes most but not all, and that correction is why
the second preset was run.** `rolling`, 12 seeds:

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.424 | 0.429 | 0.500 | **0.613** | 0.0% |
| `opaque` | 0 | 0.419 | 0.429 | 0.492 | **0.567** | 24.9% |
| `opaque` | 1 | 0.424 | 0.429 | 0.500 | **0.581** | 9.5% |
| `opaque` | 3 | 0.424 | 0.429 | 0.500 | 0.581 | 6.5% |
| `dense` | 0 | 0.373 | 0.380 | 0.400 | **0.429** | 51.3% |
| `dense` | 3 | 0.380 | 0.387 | 0.408 | 0.452 | 38.1% |
| `all` | 0 | 0.373 | 0.380 | 0.400 | 0.429 | 51.3% |

Blocking falls the same way (24.9% → 9.5%) but r64 recovers to 0.581 of a
0.613 ceiling — **about 70% of the gap, not the whole of it**. `rolling` has
real relief, so some of what stops a line there is landscape rather than
clutter, which the blocker census agrees with: stone is 19% of `rolling`'s
blockers against 13% of `wetland`'s. **Eye height is still the setting to
build; it is not a complete fix on hilly ground.**

**`eye=3` is not better than `eye=1`, and this is the row not to smooth
over.** Its pooled blocking is lower (4.8% against 8.5%) while its median
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
     blind       2.9249            -            -              0                -
    locate       3.0548            -            -       30720000              5.0
    locate       3.0266            -            -       30720000              5.0
    locate       3.0940            -            -       30720000              5.0
    locate       3.0757            -            -       30720000              5.0
    locate       3.1130            -            -       30720000              5.0
     blind       2.9378            -            -              0                -
        r8       3.0379       0.1065      -0.0349         152319               81
       r16       3.0920       0.1606       0.0191         266713              142
       r32       3.0677       0.1363      -0.0052         487096              260
       r64       3.1017       0.1703       0.0289         909763              485
```

blind spread **0.0129 ms**, locate spread **0.0864 ms** over 3,000 frames an
arm, arms alternating, every arm asserted to have started from a
byte-identical world.

**Read `vs locate`, never `vs blind`.** The `vs blind` column runs 0.11–0.17
ms and is almost entirely this harness's own whole-world scan; a reader
taking it for the sense's cost would be off by a factor of thirty.

**The wall clock cannot resolve the sense at all, and this is now measured
three times rather than argued.** In this run every `vs locate` lands inside
the control spread (−0.035 to +0.029 against 0.086). Across three runs of
this mode — two before the `main` merge, one after — r64 has read **−0.012,
+0.059 and +0.029 ms** against control spreads of 0.046 to 0.086. A quantity
whose sign flips between runs and never leaves its own noise bar is noise.

**The deterministic route says the same thing and transfers.** The
`cells read` column is **bit-identical across the merge** — 909,763 at r64,
5.0 beetles located per cast, on two different trees — which is the staleness
check the wall clock cannot provide. And `locate` reads every cell of the
world once per cast and does nothing else, so it prices one `World::get`
directly: 0.141 ms/frame for 10,240 reads/frame is **13.8 ns a read** on this
box (the pre-merge run gave 15.6 ns, so call it 14–16). Then:

| radius | cells read per beetle per cast | ms/frame at 5 beetles | µs per beetle per frame |
|---|---|---|---|
| 8 | 81 | 0.0007 | 0.14 |
| 16 | 142 | 0.0012 | 0.24 |
| 32 | 260 | 0.0022 | 0.45 |
| **64** | **485** | **0.0042** | **0.84** |

So a radius-64 all-round sense costs **0.004 ms of a frame** at this
population — which is why the clock cannot see it, and why the noise-floor
statement and the derived one agree rather than merely coexisting.

**Against the whole frame.** `cargo run --release --example ascii` on this
tree reports **mean 2.983 ms** over 12,000 frames with 154 live organisms
(worst 27.593 ms). Per `CLAUDE.md`'s own test the worst is **not** pinned by
an aggregate here — mean × frames is 35,796 ms against a 27.6 ms worst, so
the worst is one frame among thousands of comparable ones and is noise
wearing a number. The mean is the figure to quote, and this harness's blind
arm at 2.93 ms agrees with it. **The sense is 0.14% of a mean frame.**

**What it costs at scale, which is the number that matters** — the current
512x320 world is a test environment, not the target. At 0.84 µs per beetle
per frame:

| predators in the world | cost of the sense | share of a 2.98 ms frame |
|---|---|---|
| 5 (measured) | 0.004 ms | 0.14% |
| 36 | 0.030 ms | 1% |
| **358** | **0.30 ms** | **10%** |

**Radius buys itself cheaply, and that is not an assumption.** 8 → 64 is an
eightfold radius for a **sixfold** read count (81 → 485), well short of the
16 × 64 = 1,024 an unobstructed fan would cost, because rays terminate on
the first blocker and a ground-hugging beetle's downward rays die at once.
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
