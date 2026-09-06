# Plan: make roots pay — root reach is the constraint, not root income

**Status: LANDED, 2026-09-05, third draft. §2b-iii and §2b-v are built,
measured and ON by default** (`PIXEL_PHYSICS_ROOT_GATE=whole` and
`PIXEL_PHYSICS_WATER_TANK_CAP` ablate them); §1's immobile soil nutrient is
**not** built and is not proposed any more -- §9 says why. Successor to
[`plant-roots-and-transport-2026-09-05.md`](plant-roots-and-transport-2026-09-05.md),
whose §7 ranking this revises and whose §3 evidence it corrects (that
report's §3z).

**Third draft. The first proposed an immobile soil nutrient as step one;
three reviews said build it second and one found what looked like the actual
cause; then the experiment that cause implied was run over 12 seeds and
refuted it.** §9 records the whole chain, because the errors are more
instructive than the conclusion.

> **The headline, after measurement: neither the cheap lever nor the
> depletion story survives.** Soil touching roots is at **0.016**
> plant-available against **1.000** away from roots, at the *shipped*
> constant — the depletion zone is already at the wilting floor, so there is
> nothing to restore. Raising `SOIL_UPTAKE_PER_TICK` drives roots up (12/12
> seeds at 8x) and income down harder (0.11x) at every setting, with no
> usable band. **The binding constraint is root *reach*, not root income.**
> §0a and `open-bugs-handoff.md` §W2a. **And §2b now measures what does
> bind**: the root-tip gate refuses 95-99.6% of initiations while carbon
> refuses none, because it reads a saturated signal — the plant reads "not
> thirsty" while its roots sit in soil at 0.016. §2b-ii is the change that
> follows, and it is small.

## 0. The problem, restated with the corrected numbers

Roots are **cheap to keep and their benefit saturates early**, so root
quantity is unpriced at the margin and nothing selects on it.

- **Standing cost.** 332 root cells x `MAINTENANCE_PER_CELL` (1.5e-4) =
  0.0498, against **collected** income of 7.613 x `MEAN_NIGHT_INCOME_FACTOR`
  (0.49) = **1.34%**. (The predecessor said 0.65%; it compared against
  noon-equivalent income. Still tiny, still the point.) Roots pay no girth
  term — `maintenance_cost`, `plant.rs:5921`.
- **Construction cost is *not* unpriced**, and the predecessor's "unpriced in
  both directions" overstated it: a root cell costs `0.25` to build
  (`tree.ron:378`) against a shoot cell's `0.2`, times
  `penetration_cost_mult` (1.0 soil, 1.75 sand, 4.4 gravel). Roots are the
  *more* expensive tissue to build. What is unpriced is **standing cost
  against marginal benefit**.
- **Benefit saturates at ~7 (tank) and ~20 (refill) contact cells** against a
  demand of ~29, while trees grow **252–317**. Re-derived and confirmed
  independently; the measured oversizing factor across six seeds is
  **7.8x–13.1x**.

Two things make the margin worthless, and the first draft only saw the
second.

### 0a. The depletion zone is already at the floor — measured, and this replaces the second draft's story

The second draft blamed a dated constant regression: the capillary rest gap
fell 380 -> 60 one day after `SOIL_UPTAKE_PER_TICK` landed, so (the argument
went) the depletion zone a root can hold was flattened 6.3x. **Measured over
12 seeds, that is false**, and the reasoning behind it was wrong in a way
worth keeping:

**The rest gap bounds the difference between two *adjacent* soil cells, so
it caps how *steep* the depletion boundary is, not how *deep* the depletion
goes.** A root system draws its whole neighbourhood to the wilting point in
a staircase of 60-unit steps. The measured profile is that staircase almost
exactly: `near` 0.016 to `far` 1.000 is moisture 187 to 620, a 433-unit rise
across the 7 cells from BFS distance 1 to 8 — **61.9 units per cell against
a 60-unit rest gap.**

So the 2026-08-31 change made the depletion zone **wider**, not shallower —
a 7-cell ramp instead of a 1-2 cell cliff — and a wider zone means *more*
overlap between neighbouring root systems, which is *more* competition for a
local store, not less.

**And the lever it implied does not work.** `PIXEL_PHYSICS_SOIL_UPTAKE` at
1x/2x/4x/8x, 12 seeds, paired, frame 24,000:

| arm | root | contact | income | status | `near` |
|---|---|---|---|---|---|
| 60 (1x) | 224 | 175 | **4.785** | 1.000 | 0.016 |
| 120 (2x) | 305 | 241 | 3.744 | 0.944 | 0.007 |
| 240 (4x) | 362 | 289 | 2.191 | 0.571 | 0.002 |
| 480 (8x) | 467 | 313 | **0.506** | 0.346 | 0.000 |

Roots rise monotonically and reliably — **12 of 12 seeds at 8x** — and
income falls faster at every step (0.78x / 0.46x / **0.11x**; income up in
1 of 12 seeds at both 4x and 8x). Roots double, income falls to a ninth.
**There is no usable band**, because with `near` already at 0.016 a bigger
draw takes the same nearly-zero water faster and only pushes status down.

**What survives:** `SOIL_UPTAKE_PER_TICK`'s doc does state this plan's goal
and does call itself untuned. That is an observation about the record, not a
regression, and the constant is not the lever.

### 0b. Roots are geometrically redundant, so "more root" is the wrong payoff variable

Even with a depletion zone restored, income keyed on **contact root cells**
would saturate, because contact root cells are not distinct soil cells.
Already measured, in `dead-ends.md:892`: a root handicap *"removed 23% of
root cells and **3%** of uptake surface."* An ~8x redundancy. `plant_probe`
on the outdoor bed agrees — 57% of root cells have three or more root
neighbours.

**The earning surface is distinct exploited soil cells, and the marginal root
cell mostly touches soil another root already drank from.** Any mechanism
here has to be keyed on the soil cell and split among the roots touching it,
or it measures root mass and calls it uptake.

## 1. The ranking, after the experiment

| | second draft | now | why |
|---|---|---|---|
| **1** | raise `SOIL_UPTAKE_PER_TICK` | **refuted — do not build** | §0a: monotone tax, no usable band, 12 seeds |
| **1=** | — | **shrink the water tank** — it is now a prerequisite, not a nicety | §2b-vi: with the gate on, roots inflate the tank and a droughted tree cannot die |
| **2** | open the root-tip gate | **BUILT, default-off, blocked on the tank** | §2b-iii/vi: roots 3.93x in 12/12 seeds, income flat, +34% at 20 rows — and a droughted tree survives |
| **3** | root turnover | **built, measured, does not do the job** | §2b-v: every rate is worse than none on the failing bed — a shed/rebuild treadmill, not a counterweight |
| **4** | *lower the water rates* | still withdrawn, and for a third reason | §1a |
| **5** | immobile nutrient | unchanged, still second-order | §3 |
| — | economy reads attachment | unchanged, cheap, independent | predecessor §7a |

### 1a. Why every water lever is now withdrawn

Three have been tried in reasoning and one in measurement, and they fail for
three different reasons. `WATER_SCALE` moves the tank and not the depletion.
`Absorb.rate` *reduces* extraction, so it makes water more mobile relative
to the root, not less. `SOIL_UPTAKE_PER_TICK` moves the depletion and is
measured above as a monotone tax. **The common cause is that soil water near
roots is already spent**, so no lever on how it is drawn changes what is
there to draw.

## 2. What has to be true for any of this to work

Three preconditions. **Two of them are blockers today, and neither is about
resources at all** — which is why the first draft's mechanism would have
fired and moved nothing.

### 2a. The payoff variable must be soil cells, not root cells

§0b. Per-tick supply belongs to the **soil cell** and is split among the
roots touching it (`min(draw, cell_available)` per cell), never `draw` per
contact independently. The two read identically in prose and only one
produces a density response.

**Precondition to measure first, one census on the existing
`plant_severance` bed:** distinct soil cells adjacent to root tissue, against
`contact_root_cells`. If that ratio is near 1, root **reach** is the
prerequisite and everything here is step two.

### 2b. MEASURED: the gate refuses 95-99.6% of root-tip initiations, and carbon never refuses one

The second draft inferred this and left it open, noting that the
uptake-sweep split could not separate the gate from depletion because the
arm drove both. **`break_root_tips` already carries a six-bucket exit
census** (`ROOT_TIP_EXITS`, `plant.rs:7279`) whose whole purpose is this
question — *"did this fire, and if not, which line turned it back"* — and it
is a **within-run decomposition**, so no cross-arm confounding applies.

Two existing print tests, four beds and two genotype draws:

| bed / draw | gated | at_cap | no_candidate | **poor** | FIRED |
|---|---|---|---|---|---|
| 17x8, moisture 310 | 94.5% | 0.0% | 0.5% | **0.0%** | 5.0% |
| 17x8, moisture 620 | 95.6% | 0.0% | 0.1% | **0.0%** | 4.3% |
| 61x30, moisture 310 | 98.8% | 0.0% | 0.0% | **0.0%** | 1.2% |
| 61x30, moisture 620 | 98.9% | 0.0% | 0.0% | **0.0%** | 1.0% |
| slot draw -1.0 | 98.9% | 0.0% | 0.1% | **0.0%** | 1.0% |
| slot draw +1.0 | 99.6% | 0.0% | 0.1% | **0.0%** | 0.3% |

**The `status >= 0.95` gate swallows nearly everything, and `poor` is zero
in all six runs.** Root extension is not bounded by the tip cap
(`at_cap` 0.0%), not by a lack of sites to grow from (`no_candidate`
<= 0.5%), and **not by carbon** — which kills outright the "a thirsty plant
cannot afford tissue" reading the counter's own doc says §U predicted would
dominate this column. The bigger bed gates *more* (98.9% against 95.6%), so
the outdoor bed is on the wrong side of that trend.

### 2b-i. Why that is the whole answer, and it joins every other finding

The gate reads `water_status`. Three facts, each measured separately in this
session, compose:

1. `water_status` is **ceiling-clipped** and flat across roughly 80% of the
   tank's range (predecessor §3z) — it cannot tell a fifth-full tank from a
   brim-full one.
2. The soil the plant's roots actually touch is at **0.016** plant-available
   (§0a) — scraped to the wilting floor.
3. So the plant reads **"demand met, not thirsty"** while standing in
   exhausted soil, and refuses to build the roots that would reach soil that
   is not exhausted.

**That is why every water lever failed.** They all move `water_status`, and
raising `SOIL_UPTAKE_PER_TICK` did open the gate — which is exactly why
roots rose in 12 of 12 seeds at 8x. But the only way it opens the gate is by
starving the **whole plant**, which is why income collapsed to 0.11x in the
same runs. The gate's threshold was never the problem.

### 2b-ii. The change this implies: swap the gate's input, do not move its threshold

`break_root_tips` should ask *"is the soil my roots are in exhausted?"*
rather than *"is my tank low?"*. The quantity is one the engine already
computes cell by cell — `update::plant_available_fraction` over the soil
cells this organism's root tissue touches, which is the plant-side twin of
the `near` column `plant_severance` now prints. It reads **0.016** where
`water_status` reads 1.000.

Why this is the right shape rather than a retune:

- **It is not a threshold move.** Lowering 0.95 makes the plant build roots
  when its *tank* dips, which is a different and noisier signal; the
  measured 0.016/1.000 contrast is enormous and unambiguous.
- **It opens the gate without starving the plant** — the failure mode that
  sank the uptake lever.
- **It makes reach pay for the right reason**: a root in spent soil reads
  scarcity and extends; a root in fresh soil does not need to. That is the
  exploration response §0b says the payoff variable demands, keyed on the
  soil cell exactly as §2a requires.
- It costs one four-neighbour look per root cell in a walk
  (`organism_upkeep`) that already makes it.

**What must be measured before believing it**, because this is a rule over
emergent behaviour and green is its default state: the exit census is the
guard, and the prediction is specific — `gated` must fall and `FIRED` rise,
with `poor` **staying at zero** (if `poor` becomes large the constraint has
merely moved to carbon), and income must not fall the way it did under every
uptake arm. Run it against the same four beds so the before/after is paired.

### 2b-iii. BUILT and MEASURED — it works, and it is income-neutral

Shipped **on by default** (`plant.rs`'s `root_gate_is_local`;
`PIXEL_PHYSICS_ROOT_GATE=whole` is the ablation). It landed inert for one
commit so it could be measured apart from the tank cap it needs -- see
§2b-v -- and was switched on once the two guards standing in the way had
been rebuilt from their own sweeps rather than widened. Reads
`OrganismState::root_zone_water` — the mean `plant_available_fraction` over
the drinkable faces `contact_root_cells` already counts, accumulated in that
same four-neighbour look. **The threshold is not retuned**: 0.95 means the
same thing on both fraction-of-full scales.

**The switch was inert at landing, verified rather than asserted**: with it
off the root-tip exit census read byte-identical to the pre-change run on all
four beds. That is what made the paired measurement below trustworthy, and
`PIXEL_PHYSICS_ROOT_GATE=whole` still buys it back.

**12 paired seeds, outdoor bed, frame 24,000:**

| | gate off | gate local | ratio | seeds up |
|---|---|---|---|---|
| root cells | 224 | **879** | **3.93x** | **12/12** |
| contact roots | 175 | 576 | 3.29x | **12/12** |
| **income** | 4.785 | 4.765 | **1.00x** | 7/12 |
| water in the root zone (`near`) | 0.016 | **0.570** | **35.6x** | **12/12** |
| uptake | 16.5 | 21.8 | 1.32x | 10/12 |
| worst plant's water status | 0.723 | **1.000** | 1.38x | 8/12 |
| shoot cells | 3,724 | 3,515 | 0.94x | 4/12 |
| total cells | 3,941 | 4,159 | 1.06x | 6/12 |
| **root:shoot** | **7.3%** | **23.3%** | | real trees ~20-25% |

**The predictions held and the shape is the opposite of the uptake lever's.**
Roots nearly quadruple, unanimously, and **income does not move** — where
raising `SOIL_UPTAKE_PER_TICK` bought the same root response at a cost of 89%
of income. It buys roots with *reach* rather than with starvation: the root
zone goes from scraped-to-the-floor (0.016) to half-full (0.570) because 3.9x
the roots are spread over enough soil that no cell is exhausted, uptake rises
32%, and the worst-off plant in each bed goes from 0.723 to fully watered.
`poor` stayed at zero, so the constraint did not merely move to carbon;
`at_cap` rose, so the tip cap is the new bound — a bound, not a runaway — and
the gate still refuses 86-98%, so it has not become always-open.

**And root:shoot lands where the biology says it should**, 23.3% against a
real tree's 20-25%, from 7.3%. That is the number the owner's original
observation was really about.

**Repo gates, both with the switch on:** `cargo run --release --example
ascii` 31 scenes / 0 skipped; `scripts/acceptance.sh` all cases met their
expectations. `cargo test --lib --release` 1,358 passed / 0 failed.

### 2b-iv. The boundary: it works only where there is soil left to reach

The shallow arm, 12 paired seeds at `soil=6`, against the deep default:

| | shallow (6 rows) | | deep (34 rows) | |
|---|---|---|---|---|
| | off -> local | seeds up | off -> local | seeds up |
| root cells | 364 -> 431 (**1.18x**) | 8/12 | 224 -> 879 (**3.93x**) | 12/12 |
| **income** | 1.588 -> 0.189 (**0.12x**) | **2/12** | 4.785 -> 4.765 (**1.00x**) | 7/12 |
| shoot | 3,127 -> 2,076 (0.66x) | 2/12 | 3,724 -> 3,515 (0.94x) | 4/12 |
| root-zone water | 0.016 -> **0.000** | 0/12 | 0.016 -> **0.570** | 12/12 |
| root:shoot | 11.8% -> 27.4% | | 7.3% -> 23.3% | |

**In a shallow bed the change is harmful, and by the same margin the uptake
lever was: income falls to 0.12x.** The mechanism is legible in the
root-zone column. Deep, roots spread into soil no root has drunk and the
zone recovers to half-full. Shallow, there is nowhere to spread: the gate
correctly reads "spent", opens, the plant buys roots — and the zone goes to
**0.000**, fully exhausted, because the new roots land in soil that was
already dead. It pays carbon for tissue that returns nothing.

**So the rule is right and it is missing its counterweight.** "Extend when
the local soil is spent" is only correct while extension can reach something
better; nothing currently stops a plant buying roots that earn zero. That
missing bound is **§2c, root turnover** — a root in exhausted soil should
die and stop costing — which this plan already lists as necessary and which
this measurement upgrades from *necessary in principle* to *the specific
thing that makes 2b-ii safe*. Without it the gate is conditional on bed
geometry, which is not a property a rule should have.

**Two consequences for the roadmap.** The gate stays default-off until
turnover exists; and **shallowing the lab bed and enabling this gate are the
worst pairing available** — separately measured, the shallow bed is already
the hostile one (income 1.588 against 4.785 deep, worst-plant water status
0.063 against 0.723), and the gate makes it worse rather than more
interesting. The lab's 96 rows are not the problem to solve first.

### 2b-vi. The depth curve, and the regression that blocks shipping it

**Where the gate's sign flips**, 12 paired seeds at each depth:

| soil rows | income off -> on | seeds up | root cells | root zone recovers to |
|---|---|---|---|---|
| 6 | 1.588 -> 0.189 (**0.12x**) | 2/12 | 1.18x | 0.000 |
| 12 | 3.758 -> 3.469 (0.92x) | 5/12 | 2.26x | 0.016 |
| **20** | 4.168 -> **5.585 (1.34x)** | **10/12** | 3.46x | 0.280 |
| 34 | 4.785 -> 4.765 (1.00x) | 7/12 | 3.93x | 0.570 |

The last column orders it: the rule pays exactly where roots can reach soil
no root has drunk. At 20 rows it is not merely free but **+34% income in 10
of 12 seeds**. The one harmful bed is 6 rows, which is already marginal
before any change and which neither game ships — `worldgen.ron` gives 48-135
rows outdoors and the lab's `DEFAULT_SOIL_DEPTH` is 96.

**This also corrects §2b-iv's advice about the lab bed.** That section said
shallowing the bed and enabling the gate were the worst available pairing;
that was drawn from `soil=6` alone and is wrong. Shallowing toward **~20**
is the *best* pairing. Only going below about 10 is bad.

**On that evidence the default was flipped, and the flip was reverted the
same hour, because a guard caught a real regression.** With the gate on,
`a_tree_denied_water_dies_and_a_watered_one_does_not` fails: a tree denied
water for twenty thousand frames survives at **221 cells with zero
consecutive starving ticks** — comfortable, not marginal. That test asserts
the owner's 2026-08-24 ruling, so it is not a number the improved behaviour
gets to move.

**The loop, and it is this rule meeting an older defect rather than a fault
of its own.** `water_capacity_of` is `WATER_SCALE x contact_root_cells`, so
roots buy *storage*. Open the gate and a thirsty plant answers by rooting —
measured, contact roots 175 -> 576, a **3.29x tank** — and `water_status` is
`min(stock/demand, openness)` against that inflated tank, so it **rises**:
measured, the worst plant in each bed went **0.723 -> 1.000**. Income
carries `water_status` as a multiplier, so income stays above the starvation
line and nothing fires. **A droughted plant roots its way to a bigger bucket
and reads as well-watered.**

**So the prerequisite is the one this line has deferred three times**:
capacity must stop scaling without bound with contact roots
(`plant-roots-and-transport-2026-09-05.md` §7b, and §3a's arithmetic that
the tank covers the bill at ~7 contact cells while trees grow 250-320).
Until then the gate is *correct and unshippable*, and that is a statement
about the tank rather than about the gate.

### 2b-v. Turnover was built and does NOT fix it — a null, and the reason is instructive

§2b-iv predicted fine-root turnover was the missing counterweight. **Built,
swept at three rates on the failing bed, and the prediction is refuted:**

| arm (shallow, soil=6, 12 paired seeds) | income | root | shoot | root-zone |
|---|---|---|---|---|
| gate off (shipped) | **1.588** | 364 | 3,126 | 0.016 |
| gate on, turnover 0 | 0.189 | 430 | 2,076 | 0.000 |
| gate on, turnover 0.002 | **0.104** | 402 | 1,766 | 0.000 |
| gate on, turnover 0.01 | **0.070** | 328 | 1,271 | 0.000 |
| gate on, turnover 0.05 | **0.123** | 300 | 2,290 | 0.000 |

Every rate is **worse** than turnover-0, and per-seed the direction is
noise (income up in 7/12, 3/12, 5/12 against that arm). Turnover does not
rescue the shallow bed; it deepens the hole.

**Why, and it is a failure this repo has already recorded.** In a 6-row bed
*every* cell is exhausted — the root-zone column reads **0.000 in all four
arms**. So turnover does not shed the roots that stopped earning in favour
of ones that are; it sheds roots indiscriminately, the gate immediately
rebuilds them because local soil is still spent, and the plant pays
construction twice. That is the treadmill `plant.rs`'s own die-back comment
names — *"a starving plant re-lays almost exactly what die-back removes"* —
which I had read while building this and did not connect.

**What it actually says, which is bigger than turnover.** Both rules — the
gate and turnover — assume a **gradient**: somewhere better to put a root.
The gate's whole logic is *this soil is spent, go elsewhere*, and turnover's
is *this root stopped earning, spend the carbon elsewhere*. **A bed with no
unexhausted soil has no "elsewhere", so both rules become pure cost.** The
deep bed has a gradient (root zone recovers 0.016 -> 0.570) and both the
gate's gains and its neutrality on income follow from that.

**And the shallow bed may be the wrong test.** At `soil=6` the bed is
already marginal *before any change*: income 1.588 against the deep bed's
4.785, and worst-plant water status **0.063**. Asking a root rule to rescue
an environment that is nearly dead is not the same as asking whether the
rule is sound. The measurement that decides it is **where the sign flips
across depth**, which is running: `soil=12` and `soil=20` against the
`soil=6` and `soil=34` already measured. If the gate is negative only in
beds that are already marginal, it is shippable with a stated range; if it
inverts somewhere in the middle of the useful range, it is not.

### 2c. Without root turnover there is no interior optimum

A root that has mined its cell out earns nothing for ever and costs
`MAINTENANCE_PER_CELL` for ever, so the optimal strategy becomes *grow root,
abandon it, grow more* and root mass rises monotonically — which
`dead-ends.md:781` records as the shipped state once soil gave roots income
everywhere (*"they proliferated until they had converted an entire soil bed
to root tissue"*), bounded now only by `MAX_ROOT_FRACTION` (0.5), a **third**
saturation. Real fine roots die on a timescale of weeks to months precisely
because the soil around them is exhausted.

The first draft scoped turnover out while stating the reason it cannot be
scoped out. Cheapest form: a per-root-cell death chance scaled by *that
cell's own local depletion* — free to read, and returning that cell's
nutrient to the soil is the conservation §3 needs.

## 3. The nutrient, and its now-smaller job

> **BUILT 2026-09-06, and shipped INERT.** §3a's storage, §3b's time
> recovery, §3c's construction pricing and §3d's placement are all in the
> tree behind `PIXEL_PHYSICS_NUTRIENT=<initial>`, default `0` = off. What is
> **not** done is the calibration sweep, and that is deliberate rather than
> unfinished: `CLAUDE.md` says a correct mechanism at inherited constants is
> a regression, and every number below (`initial`, the per-tick draw,
> recovery, `NUTRIENT_STARVED_COST`) is a first guess.
>
> **Why it was built after all**, given §1 ranked it second and the roots
> work then refuted the premise it was ranked on: the refutation was of an
> immobile nutrient *as the cheapest lever to make roots pay*. The gate was
> that lever and it shipped in #246. This is a different claim, reported
> from play and never tested here — **soil must supply something water
> cannot**, or a lit plant with a drip on it has everything the engine
> prices and cannot die. That was the evolution lab's floating plant.
>
> **One exploratory run, n=1, unswept**, lab box at 30,000 frames,
> `initial=200 recovery=1 draw=1`, paired against off with
> `RAYON_NUM_THREADS` pinned: cells **9,393 → 7,509** (0.80x), plants 481 →
> 456, biggest 497 → 480, and **roots reach 34 → 22 rows**.
>
> **That last column was the finding, and it pointed at §3c.** Pricing
> *construction* taxed root growth along with everything else, so a nutrient
> shortage made a plant build **fewer** roots — the opposite of foraging,
> and against the root gate #246 shipped.
>
> **§3c amended 2026-09-06: root tissue is exempt from the construction
> penalty.** Charging scarcity prices to build a root is a deadlock — roots
> are the only way to reach more nutrient, so a plant short of it cannot
> afford the one thing that fixes the shortage. Preferred over adding a
> nutrient term to `allocate_to_frontier`'s `root_weight` (the prior art
> beside `ROOT_BIAS_AT_FULL_WATER`) because that weight sits in a sum whose
> terms are calibrated against each other, and `CLAUDE.md` is explicit that
> changing what one term can express reallocates the whole sum. Changing a
> *price* lets the existing economy do the shifting: under scarcity shoots
> get dear and roots do not.
>
> **Measured, four paired founder counts, lab box at 30,000 frames,
> `RAYON_NUM_THREADS` pinned:**
>
> | founders | cells off → on | roots reach off → on |
> |---|---|---|
> | 4 | 0.84x | **1.17x** |
> | 6 | 0.92x | **1.38x** |
> | 8 | 1.07x | **1.55x** |
> | 12 | 0.84x | 0.76x |
>
> **Root depth up on 3 of 4, median ≈1.28x** — against **0.65x** on the one
> arm measured before the exemption, so the sign is reversed. It is *not*
> unanimous, and the stand shrinks on 3 of 4 (median ≈0.88x), which is what
> a tax should do. Still inert; the constants are still first guesses, and
> founders=12 going the other way is the first thing a real sweep should
> explain.


Restoring the depletion zone (§1 item 1) plausibly delivers **S1 and S2**
below. What it cannot deliver is **S3**, because two viable root
morphologies need two resource axes with *different spatial distributions* —
and it does not decouple root income from rainfall.

So the nutrient is worth building, second, for those two things. If §1 item 1
produces the null instead, the nutrient inherits a positive control this plan
would otherwise lack.

### 3a. Storage: option D, and B/C are a documented dead end

**The first draft preferred a sparse `HashMap<(i32,i32), _>` sidecar.
`dead-ends.md:685` rejects exactly that** — *"a hash lookup on the diffusion
path, positions unstable under a world that streams chunks, and an
overwritten cell leaves an entry nobody owns or reclaims."* Two of the three
apply verbatim; the plan's risk column claimed "none new". Its re-test
condition (*per-cell data outliving its organism*) **is** met here, so it is
a genuine reopening — but not on the terms the first draft offered.

**Option D:** a parallel `Box<[u8]>` per soil-bearing chunk, lazily
allocated, plus one `nutrient_recovered_at: u64` stamp on the chunk, so
recovery is **closed-form on read** — `deficit − rate × (frame − stamp)`.

- 4,096 x 1 B = **4 KB/chunk**; ~600 soil-bearing chunks in the shipped
  8192x2560 world ≈ **2.4 MiB against a 240 MiB grid**. (The first draft
  sized this against the lab bed and used a stale `chunk.rs` comment
  implying a 4-byte `Cell`; `Cell` is **12** bytes, guarded at
  `cell.rs:525`.)
- No hashing on the uptake path — `World::get` already resolves the chunk.
- No leak, no streaming instability: allocated with the chunk, dropped with
  it. The stamp **is** the M10 catch-up primitive `PLAN.md:1202` already
  requires for a nutrient currency.
- Availability reads as `initial(x, y) − deficit`, with `initial` a cheap
  function of depth — which makes the **shallow-nutrient / deep-water bed
  S3 needs** free, and matches the biology (nutrients concentrate in
  topsoil).

**A coarse field channel stays disqualified, but on the general argument
only.** The first draft's arithmetic was wrong: it quoted "13 rows deep" as a
max when its source explicitly warns *"'Roots stop at 13 rows' is a statement
about the median, not the max… Quoting a max here will read as a
contradiction"*, and that figure is a lab number since superseded. Measured
outdoors: **median 24 rows, max 34, lateral spread median 37.** The
disqualification rests instead on `CLAUDE.md`'s block-nearest gradient trap —
four recorded bugs — which is sufficient alone.

### 3b. Recovery: not at decay sites

The first draft preferred crediting nutrient at decay events. **Both the
performance and the biology review independently rejected it**, and
`dead-ends.md:583` and `:585` record the same mechanism built twice and
reverted twice, once with a measured runaway — *"the standing-biomass probe
went from **1,718 cells to 2,652 and still climbing**"* — because it creates
a pump: tree sheds litter, litter rots into resource, tree drinks it, grows,
sheds more. A freely-credited nutrient is worse than water, which at least
has evaporation as a sink.

It also lands in the wrong place: decay fires where litter comes to rest, at
the surface, while depletion is metres down, and a nutrient that by design
does not flow would pile credit in row 0 above a deficit that never clears.
And it would tie the new currency's supply to `open-bugs-handoff.md` §O, an
open litterfall bug the owner reported as *"leaves are just falling too fast
which creates too much food."*

**Use closed-form time recovery (§3a).** Decay events may add a *bonded*
deposit later — the tissue's own stored nutrient, spread a few rows down —
which is `PLAN.md`'s conserved loop done properly. Conservation is the
condition that separates it from the reverted pump.

### 3c. Where the term enters income: not `min`

The first draft proposed `min(water_status, nutrient_status)`. **Do not.**
Under `min` the non-binding resource has exactly zero marginal value —
promoting this plan's own complaint from an accident to a law — and income
collapses to one axis, contradicting the diversity argument the plan rests
on. It also erases the stomatal locus's fitness signal: wherever nutrient
binds, a prudent individual and a spendthrift earn the same while the
prudent one keeps its stock, so prudence becomes free
(`plant-genome-design.md` §4.3 is about avoiding exactly this shape).
`water_status` is a **stomatal conductance**, not a Liebig term, and
multiplying is correct for it.

**Preferred: put the nutrient on the price of new tissue** (`Grow.cost`
scaled by nutrient scarcity) rather than on the light multiplier. That leaves
water's calibration literally untouched, matches the physiology better — N
and P limit sink activity and leaf construction more than instantaneous
photosynthesis per unit leaf — and adds a real second axis instead of
competing for the first.

### 3d. An implementation trap that would silently zero the mechanism

`absorb_water`'s Powder arm gates its body on `available > 0.0` **and** on
`capacity - water` (the water tank's headroom, `plant.rs:565-570`). A
nutrient draw placed inside either gate saturates on the **water tank** —
the ~7-cell knee — and would look exactly like "the nutrient does not
matter." It must be a sibling of the water draw inside the neighbour loop,
sharing the visit and none of the gates. `transpire` (`plant.rs:9218`) is
the existing prior art for that shape: per-root-cell, four-neighbour, no tank
gate.

## 4. Success claims, restated so they can fail

The first draft's three could not: S1's instrument measured S2, and S2 was
decided by a line of specification rather than by a run.

| # | claim | falsifier | notes |
|---|---|---|---|
| **S1** | **volume-matched**: two plants with equal soil volume reached, one with 2x the root cells in it, differ materially in income | income flat in root count at matched volume | the volume control is what the first draft lacked |
| **S2** | an individual whose roots **spread** out-earns one of equal root mass that **clumps** | spread and clumped earn alike | a paired single-plant comparison, not a per-cell statistic |
| **S3** | shallow-fibrous and deep-fibrous are **both** viable, each winning in its own bed | one arm wins in every bed | needs the nutrient; §1 item 1 alone cannot do it |

**S3 no longer claims taproot-vs-fibrous, and the first draft was wrong to.**
`root-morphology-findings.md` names three blockers this plan removes none of,
and one is decisive: `can_widen` (`plant.rs:9491`) requires an `EMPTY` or
own-`Leaf` neighbour, so **a root buried in soil can never thicken** — the
conical / fusiform / napiform family is a *thickness* shape and is
inexpressible at any resource setting. There is also no root apical
dominance: `allocate_to_frontier` gives every root tip one shared
`root_weight`, which produces fibrous by construction.

## 5. Order of work

1. **Census the precondition** (§2a). One run, existing harness.
2. **`SOIL_UPTAKE_PER_TICK` at 6–10x**, `plant_severance` + `selection_arena`,
   order statistic over **>=12 seeds** — six is not a sweep, and this line has
   already had a 1.64x over six become a per-seed median of zero over
   eighteen. Watch `a_tree_denied_water_dies_and_a_watered_one_does_not`;
   `dead-ends.md:1122` records it as the test that moves whenever soil water
   is touched.
3. **Open the root-tip gate** (§2b) — necessary for 2 to show anything.
4. **Root turnover** (§2c) — necessary for 2 to have an interior optimum.
5. **Then** the nutrient, for S3 and rainfall-decoupling, shipped inert
   behind `const NUTRIENT_DEMAND: f32 = 0.0` with an early `return 1.0`,
   exactly parallel to `anchor_status_of` — a **code** switch, not a data
   setting. (The first draft proposed pinning the initial *stock* high, which
   is not inert: a seedling with two contact roots reads status < 1 on a bed
   of infinite stock, so phase 2 would have been a handicap on establishment
   precisely where this line's arms have gone vacuous before.)

Nothing in step 5 may add a `world.rng` draw or a `HashMap` iteration to any
pass: one RNG call diverges the world outright.

## 6. Frame cost

- Uptake rides an existing four-neighbour visit — **verified**, ~0.07 ms/frame
  for twenty mature trees, and four extra probes per call is +0.03 ms.
- Recovery must never become a full-world per-frame sweep. §3a's whole point.
- **A nutrient write introduces no new dirty-chunk path**: `absorb_water`
  already writes the soil cell through the ordinary `world.set`
  (`plant.rs:614`), so every root drink dirties its chunk every organism tick
  already. The first draft called this its most likely error; it is a clean
  no.
- **A debug overlay must default off.** `render.rs:1941-1971` records that a
  scalar living outside `Cell` defeats the dirty-rect skip because a changed
  reading dirties nothing — *"a full redraw every frame is ~10 ms mean on a
  settled world."*
- **Free win found next door:** `plant.rs:614` is an aux-only write on an
  unchanged material and still uses the loud channel, while
  `World::set_soil_moisture`'s quiet path exists for exactly that case. A
  dirty chunk costs the field solve as well as the sweep. One line, its own
  measurement, not part of this plan.

Instruments: `lab_cost phases=1` (read `active_sites`, `field`, `awake/f`),
`ascii` for the outdoor worst frame, `RAYON_NUM_THREADS` pinned for any
compared counter, and a `NUTRIENT=off` env arm inside one binary so both
arms see the same parallelism.

## 7. Out of scope

Mycorrhizae (and note they make S2's *density* half a sub-cell claim this
grid cannot resolve — state it as contact **redundancy**); N-fixation;
nutrient species; nutrient transport within the plant (that is the
path-priced pool, §8); the water tank's size.

## 8. Girdling, still the acceptance test for path-priced carbon

Cut a ring of phloem, leave the xylem: the crown stays green for weeks while
the **roots** starve, and only then does the tree die. Root-first, graded,
delayed, from one cut. Inexpressible today — crown and roots share one
undirected pool. Note `dead-ends.md:637` rejects modelling *"distinct
xylem/phloem systems"* at the design-philosophy level (*"model the signals,
not the biochemistry carrying them"*), so this has to argue past that, not
only past scope.

## 9. What each draft got wrong

Kept because how a plan went wrong is worth more than its conclusion — and
this one went wrong three times, each time in a way the next step caught.

**Draft two's error, found by running its own experiment.** It blamed a
dated constant regression and proposed raising `SOIL_UPTAKE_PER_TICK` as the
cheap fix. Both halves failed: the rest gap caps the depletion's *steepness*
and not its *depth*, so nothing was flattened (§0a), and the lever is a
monotone tax across 12 seeds with no usable band. **The error was reasoning
about a depletion zone without ever measuring one** — three documents and
two reviewers argued about how deep it was, and the answer took one census
and thirty seconds of runtime. `CLAUDE.md`'s first method rule is *look
before you measure*; all of us went straight to arithmetic.

**And a null this design cannot deliver, stated so nobody re-reads it as
one.** Splitting the 48 runs on `break_root_tips`' 0.95 gate gives median
root 305 shut against 364 open — but the groups also differ 8x in `near`,
because the arm drives both. Arm is a confounder; the split cannot separate
the gate from the depletion. §2b is still open and needs an arm that moves
the gate alone.

### 9a. Draft one's errors

1. **Misattributed the cause.** Blamed "water flows, in this engine and in
   nature." The real cause is at least partly two constants that stopped
   composing on 2026-08-31 (§0a, §W2). The prosecution found this by reading
   a constant's doc and running `git log -S`.
2. **Never named the cheapest lever.** `SOIL_UPTAKE_PER_TICK` moves the
   depletion zone without moving income; the draft withdrew a whole class of
   "water levers" on reasoning that is mechanically backwards (§1a).
3. **Preferred a documented dead end for storage** (`dead-ends.md:685`) and
   a twice-reverted pump for recovery (`:583`, `:585`). I grepped
   `dead-ends.md` for my *subject* and not for my *mechanism*, which is the
   exact failure `CLAUDE.md` warns about.
4. **Keyed the payoff on the wrong variable** — contact root cells, which
   `dead-ends.md:892` had already measured as ~8x redundant against uptake
   surface.
5. **Oversold S3** — three morphology blockers survive untouched, one of
   which makes the taproot family inexpressible at any setting.
6. **Wrote success claims that could not fail**, and a ship-inert phase that
   was not inert.
7. **Cited `PLAN.md`'s "axis count" line as its foundation** while §3
   disqualified the field channels that line is about. The principle
   transfers; the citation does not endorse the storage decision.
