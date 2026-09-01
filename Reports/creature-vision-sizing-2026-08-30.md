# Sizing a sight sense before it is built — E15

**Status: measured pre-flight, 2026-08-30, re-taken in full on every tree
`main` landed underneath it — nine in all, the last a wholesale worldgen
rewrite (2026-08-31).** `examples/vision_probe.rs` is the instrument; nothing
here changes behaviour.

**The recommendation has survived all nine. Two versions of the *argument*
for it did not, and one headline finding no longer generalises.**

- **Radius 64 over 32**: held 9/9, median and p10, every preset — including
  across the worldgen rewrite that moved every absolute number in §3. Stated
  as an **ordering**, after two superlatives died (§3).
- **Eye one cell up**: held 9/9, still reaches the transparent-world ceiling.
  But **where it pays has moved** — `wetland`'s occlusion now costs almost
  nothing, so the value is on `rolling` (§4).
- **"Floor clutter, not landscape" is now preset-dependent** (§4a). The
  rewrite exposes far more stone: **bare rock is 50% of what blocks a sight
  line on `rolling` and 44% on `arid`**, against 12–34% before. On `wetland`
  the original reading holds. **Do not carry it forward as a general claim.**
- **Cost**: free at either end of a per-read price now measured 13.8–38.9 ns
  — 0.14% of a frame at best, **0.38% at the worst reading on the loudest
  box** (§5); the built sense costs twice that (§0b).

Supersedes nothing. Reads forward from `predation_probe`'s pre-flight
(`creature-evolution-plan.md` §5 / `creature-review-2026-08.md` §T6), whose
numbers are quoted rather than re-derived.

---

## 0. The recommendation, first

**Build it at radius 64, all-round, seeing over the litter.** Concretely,
four decisions:

| decision | build it as | because |
|---|---|---|
| **reach** | **64 cells** | **64 delivers more than 32 at every preset, on median and p10 alike, on all nine trees** — currently +0.100 to +0.156 of median and +0.053 to +0.200 of p10. An ordering, not a superlative: see §3 for the two that died |
| **shape** | **all-round**, not a forward cone | a ±60° cone throws away **a third of every sighting** and saves nothing measurable |
| **what stops it** | rock and soil, **never floor litter** | blocking runs **13–20%** of pairs; one cell of eye height reaches the transparent-world ceiling, on all nine trees. **What blocks it is now rock on two of three presets** — §4a |
| **foliage** | **not a binary blocker** | making plant matter opaque costs **most of the sense** (r64 median 0.667 → 0.367) and no eye height buys it back |

**And it is free at this scale.** A radius-64 fan of 16 rays, cast at the
beetle's own `tick_interval`, reads **502 cells per beetle per cast** and
costs **0.005–0.015 ms of a frame** — **0.14–0.38%** of the 3.89 ms mean
`ascii` reports on this tree, and below what a wall clock can resolve. It
runs 1–2% of a frame at a few dozen predators and reaches 10% only in the low
hundreds. §5 says why those are ranges, and §0b why the built sense costs
twice them.

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

Re-measured **nine times** while `main` moved under it, the last after a
wholesale worldgen rewrite. That is enough runs to separate the load-bearing
findings from this week's weather.

| finding | stability |
|---|---|
| **eye=1 reaches the transparent-world ceiling** | **9/9 — never failed**, through a worldgen rewrite |
| **64 beats 32 — median and p10, every preset** | **9/9 — never failed**, through a worldgen rewrite |
| the cost conclusion (free at this scale) | **never failed**, at either end of a per-read price spanning 13.8–38.9 ns |
| `dense` costs most of the sense | never failed |
| a ±60° cone costs a quarter to a third | never failed |
| absolute `los` medians | **drift.** `wetland` r8 has read 0.383, 0.283, 0.360, 0.311 |
| absolute blocking percentages | **drift.** `wetland` has read 28.1%, 24.3%, 19.6% |
| **which materials block** | **changed character.** Rock was 12–34% of blockers, is now 41–44% on two presets — §4a |
| **where eye height pays** | **moved.** It was worth most on `wetland`; that preset now has almost no occlusion left to recover |
| *"32 → 64 is the largest single step"* | **was true, then false** (tree 5) |
| *"...largest single step in the p10"* | **was true, then false** (tree 8) |

**The two dead superlatives are the lesson.** Both described the *shape* of a
curve, and shape moves with the world; both survived one landing and died on
the next. The ordering underneath has survived all nine. **When a finding has
to outlive a moving world, state it as an ordering.**

**Trust the orderings. Re-take every percentage, and re-read §4a before
repeating anything about what blocks a sight line** — that one did not merely
drift, it changed character. One command, three minutes:

```
cargo build --release --example vision_probe   # NOT --release alone
./target/release/examples/vision_probe mode=survey seeds=18 preset=wetland
```

**A re-take is worth it when `main` has touched `src/sim/creature.rs`,
`organism.rs`, `plant.rs`, `world.rs`, `assets/species/*.ron` or
`src/worldgen/`** — the population, the floor, or the terrain. Nine trees say
a landing touching none of those moves nothing at all. **Four of the nine
moved the numbers: two plant-side, one creature-side, and the worldgen
rewrite — which moved them furthest and changed §4a's finding outright.** A
hit on the rule is not evidence of movement: five landings tripped it and
came back byte-identical.

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
eye=0 cone=±60°`, on the **rewritten worldgen**. Median `los`:

| preset | r8 | r16 | r32 | **r64** | p10 r32 | **p10 r64** | pairs blocked |
|---|---|---|---|---|---|---|---|
| `wetland` | 0.311 | 0.483 | 0.500 | **0.656** | 0.033 | **0.233** | 19.6% |
| `rolling` | 0.300 | 0.353 | 0.400 | **0.500** | 0.067 | **0.233** | 14.3% |
| `arid` | 0.247 | 0.300 | 0.353 | **0.456** | 0.260 | **0.313** | 12.6% |

`wetland` in full:

| | min | p10 | **median** | p90 | max |
|---|---|---|---|---|---|
| `range` r64 | 0.000 | 0.344 | 0.667 | 0.800 | 0.967 |
| **`los` r8** | 0.000 | 0.022 | **0.311** | 0.500 | 0.567 |
| **`los` r16** | 0.000 | 0.022 | **0.483** | 0.561 | 0.700 |
| **`los` r32** | 0.000 | 0.033 | **0.500** | 0.656 | 0.742 |
| **`los` r64** | 0.000 | 0.233 | **0.656** | 0.783 | 0.889 |
| `cone` r64 | 0.000 | 0.117 | 0.483 | 0.583 | 0.594 |
| beetle → nearest ant | 19.2 | 32.2 | 48.7 | 75.4 | 128.0 |
| ...nearest ant it can *see* | 3.9 | 5.6 | 15.2 | 23.7 | 60.4 |

### The radius argument, stated on the one footing that has held

**Build at 64. The claim is an ordering, not a superlative.**

> **At every preset, on every one of nine trees, radius 64 delivers more than
> radius 32 — median and p10 alike.** Here the median gains +0.156 / +0.100 /
> +0.103 and the p10 gains +0.200 / +0.166 / +0.053. The sign has never gone
> the other way, **including across a worldgen rewrite that moved every other
> number on this page.** §5 prices the extra reach at 0.14–0.38% of a frame.

**Two earlier versions of this argument used a superlative, and both were
falsified by a later tree.** Recorded rather than quietly replaced:

| version | claim | how it died |
|---|---|---|
| first | *"32 → 64 is the largest single step at every preset"* | tree 5: on `wetland` the **median**'s largest step became 8 → 16 |
| second | *"...the largest single step **in the p10**"* | tree 8: on `wetland` and `rolling` the p10's largest step became 16 → 32 |

**The lesson, now tested three times: the curve's *shape* moves with the
world; its *ordering* does not.** A superlative is a claim about shape, and
neither survived two landings. "More at 64 than at 32, everywhere, always" is
a claim about ordering, and nine trees — the last a worldgen rewrite — have
not dented it.

**The margin is not stable either, and that is worth saying.** On tree 8 the
p10 at r32 ran 0.22–0.27 and the case for 64 was merely sufficient; the
rewrite collapsed it to **0.033–0.260** and the case is dramatic again — the
stranded beetle on `wetland` now sees prey a thirtieth of the time at 32 and
a quarter of the time at 64. **The direction is what to build on; the size of
the gap is a fact about this week's world.**

**Two further readings:**

- **A short sense is a contact sense with extra steps.** At r8 the median runs
  0.25–0.31; the median *visible* ant is 15 cells away, so a radius under that
  discards the sightings that actually happen.
- **The curve is still climbing at 64.** The median beetle sits 49–64 cells
  from the nearest ant. Where 128 lands is **not measured**.

**A ±60° cone costs a quarter to a third** (r64 median 0.656 → 0.483, 0.500 →
0.342, 0.456 → 0.300) for a saving §5 shows is not worth having.

---

## 4. What terrain costs, and what is actually doing the blocking

`mode=occlusion`, median `los`. `none` is the **transparent-world ceiling**
rather than a setting, and every other row is asserted at or below it.

`wetland`, 18 seeds:

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.325 | 0.483 | 0.500 | **0.667** | 0.0% |
| `opaque` | 0 | 0.311 | 0.483 | 0.500 | **0.656** | 19.6% |
| **`opaque`** | **1** | 0.311 | 0.389 | 0.500 | **0.667** | **8.2%** |
| `opaque` | 3 | 0.325 | 0.483 | 0.500 | 0.667 | 6.5% |
| `dense` | 0 | 0.311 | 0.344 | 0.367 | **0.367** | 74.8% |
| `dense` | 3 | 0.311 | 0.344 | 0.389 | 0.411 | 64.9% |
| `all` | 0 | 0.311 | 0.344 | 0.367 | 0.367 | 75.0% |

`rolling`, 12 seeds:

|  occl | eye | r8 | r16 | r32 | **r64** | pairs blocked |
|---|---|---|---|---|---|---|
| `none` (ceiling) | 0 | 0.313 | 0.353 | 0.400 | **0.507** | 0.0% |
| `opaque` | 0 | 0.300 | 0.353 | 0.400 | **0.400** | 16.3% |
| **`opaque`** | **1** | 0.307 | 0.353 | 0.400 | **0.500** | **10.8%** |
| `opaque` | 3 | 0.307 | 0.353 | 0.400 | 0.500 | 10.1% |
| `dense` | 0 | 0.300 | 0.353 | 0.393 | **0.400** | 41.5% |
| `dense` | 3 | 0.293 | 0.333 | 0.380 | 0.393 | 34.5% |
| `all` | 0 | 0.300 | 0.353 | 0.393 | 0.400 | 41.5% |

**Eye height is still the setting to build, and it is now 9/9.** `opaque
eye=1` reaches the ceiling exactly on `wetland` (0.667) and 0.500 of a 0.507
ceiling on `rolling`. Nine trees, one of them a worldgen rewrite, and this
has never failed.

**But its *value* has moved, and it now pays on `rolling`, not `wetland`.**
On the rewritten worldgen `wetland`'s occlusion barely costs anything at r64
— 0.656 against a 0.667 ceiling, a loss of 0.011 — so there is almost nothing
for the lift to recover. On `rolling` it turns 0.400 into 0.500, **a quarter
more sightings**. A reader who took "eye=1 recovers the ceiling" to mean
"eye=1 is worth a lot everywhere" would now be wrong: it is worth a lot where
occlusion is, and occlusion has moved.

### 4a. "Floor clutter, not landscape" is now preset-dependent — a headline correction

**This report's second-most-quoted finding no longer generalises, and the
worldgen rewrite is why.** What stops the rays, pooled over 18 seeds:

| preset | what stopped them | rock share |
|---|---|---|
| `wetland` | corpse 22%, basalt 22%, soil 21%, seed 18%, litter 10%, packedsoil 5% | **27%** |
| `rolling` | **basalt 41%**, seed 19%, soil 12%, packedsoil 9%, corpse 6%, litter 6% | **50%** |
| `arid` | **basalt 44%**, corpse 41%, seed 15% | **44%** |

Against the old worldgen, where litter and seed dominated and bare rock ran
12–34%, **rock is now half of what blocks a sight line on `rolling` and 44%
on `arid`**. The rewrite exposes far more stone. So:

- on `wetland` the original reading holds — clutter and bodies, rock a quarter;
- on `rolling` and `arid`, **landscape is now the larger half**, and an eye
  lift cannot see over a boulder.

This is consistent with the eye-height numbers rather than in tension with
them: total blocking *fell* (24.3% → 19.6%, 22.1% → 16.3%) while its
*composition* shifted to rock. Less is blocked, and more of what is blocked
is terrain. **Do not carry "it is floor clutter" forward as a general claim.**

**`eye=3` is not better than `eye=1`** — on `wetland` its pooled blocking is
lower (6.5% against 8.2%) at equal or better medians. But note `eye=1` reads
**0.389 at r16** against `eye=0`'s 0.483: **the lift makes that one radius
worse**, which it did not on any earlier tree. Lifting an eye can move a
sight line *into* a blocker as easily as over one. Nothing here explains it;
it is one radius on one preset, recorded rather than smoothed.

**The owner was asked about eye height and declined to pick.** Card
`20260830T021057007Z-18900e`: *"I don't think there is a clear good answer.
Just pick one that makes sense to you."* So eye=1 rests on the numbers.

**Foliage remains the biggest single lever.** `dense` takes r64 from 0.667 to
**0.367** on `wetland` and 0.507 to **0.400** on `rolling`, and eye height
recovers only a fraction. **`dense` is not shippable as a binary rule**, and
this is the ethos law rather than a tuning note: *an outcome is a
distribution, not a binary*. A bush should *attenuate* — shorten the
effective radius through it — which this study did not price and the next
lane should.

Water is a non-question: `all` differs from `dense` by 0.2 points.

---

## 5. What it costs per frame

`mode=cost`. A **fan of 16 rays**, each marched to the first blocker or the
radius, cast once every 8 frames. **Three arms, not two**: `cast_fan` must
find the beetles first, and this harness does that by scanning all 81,920
cells, which the engine never does. `locate` is that scan alone; **the sense
costs `rN` minus `locate`.**

```
       arm     ms/frame     vs blind    vs locate     cells read  per beetle/cast
     blind       3.4858            -            -              0                -
    locate       3.7876            -            -       30720000              6.0
    locate       3.6286            -            -       30720000              6.0
    locate       4.0248            -            -       30720000              6.0
    locate       3.8781            -            -       30720000              6.0
    locate       3.7438            -            -       30720000              6.0
     blind       3.3418            -            -              0                -
        r8       3.6065       0.1928      -0.2060         216771               96
       r16       3.8503       0.4365       0.0377         373215              166
       r32       4.0383       0.6246       0.2258         642000              285
       r64       3.8392       0.4255       0.0267        1128602              502
```

**This run was taken on a loaded box, and the report says so rather than
quietly dropping it.** The `locate` spread is **0.396 ms** — three to eight
times any earlier run's — and `ascii` on the same tree reads mean 3.891 ms
against ~3.0 on the quiet ones, with a 79 ms worst frame. Every `vs locate`
is still inside that spread.

**The per-read price is the loosest number here, and this run stretches it.**
Seven readings across nine trees: **15.6, 13.8, 14.9, 22.1, 16.4, 15.6 and
38.9 ns**, the last from the run above. The spread tracks the *control*
spread, which is what a machine-load explanation predicts. **So take the
whole range**, and note the conclusion survives at the far end:

| radius | cells read per beetle per cast | ms/frame at 6 beetles | µs per beetle per frame |
|---|---|---|---|
| 8 | 96 | 0.0010–0.0028 | 0.17–0.47 |
| 16 | 166 | 0.0017–0.0048 | 0.29–0.81 |
| 32 | 285 | 0.0030–0.0083 | 0.50–1.39 |
| **64** | **502** | **0.0053–0.0147** | **0.88–2.44** |

**At the optimistic end a radius-64 sense is 0.14% of a frame; at the
pessimistic end — worst per-read reading, loudest box — 0.38%.** Both are
free, which is the point of the range: the recommendation does not depend on
which measurement you trust.

**Against the whole frame.** `ascii` reads **mean 3.891 ms** over 12,000
frames with 179 live organisms (worst 79.097 ms). The worst is **not** pinned
by an aggregate — mean × frames is 46,692 ms against a 79 ms worst — so it is
noise wearing a number, and here it corroborates that the box was loud.

**At scale**, at 0.9–2.4 µs per beetle per frame against a ~3.9 ms frame:

| predators in the world | cost of the sense | share of a frame |
|---|---|---|
| 6 (measured) | 0.005–0.015 ms | 0.14–0.38% |
| a few dozen | ~0.03–0.09 ms | ~1–2% |
| **a few hundred** | **~0.3–0.8 ms** | **10–20%** |

**And the built sense costs about twice this** — §0b: 1,020–1,100 cells per
cast against the 502 priced here, because it must test prey in the un-lifted
frame and blockers in the lifted one. **Double this table before sizing a
different sense off it.**

**Radius buys itself cheaply.** 8 → 64 is an eightfold radius for a
**five-fold** read count (96 → 502), because rays die on the first blocker.
**Occlusion makes the sense cheaper as well as weaker** — the converse showed
on tree 5, where a world with less litter cost 9% more to look at.

**Two guards**, because a cost that vanishes may be work that vanished:
`cells read` is asserted nonzero on every sighted arm and comes back from the
far side of the casting call, and `locate` is asserted to have found a beetle.

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
