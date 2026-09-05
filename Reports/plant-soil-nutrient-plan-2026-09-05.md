# Plan: make roots pay — the cheap lever first, then an immobile nutrient

**Status: plan, 2026-09-05, revised the same day after three reviews.
Nothing built.** Successor to
[`plant-roots-and-transport-2026-09-05.md`](plant-roots-and-transport-2026-09-05.md),
whose §7 ranking this revises and whose §3 evidence it corrects (that
report's §3z).

**This is the second draft. The first proposed an immobile soil nutrient as
step one; three reviews said build it second, and one found the actual
cause.** §9 records what changed and why, because the first draft's errors
are more instructive than its conclusion.

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

### 0a. The root's depletion zone is 6.3x shallower than it was designed to be — and that is a dated regression

**Filed as `open-bugs-handoff.md` §W2.** `SOIL_UPTAKE_PER_TICK`'s own doc
states this plan's goal as its design intent — *"the interesting behaviour is
a root system **competing with itself and its neighbours for a finite local
store**"* — and calls itself untuned. The maximum standing depletion a root
can hold is the capillary rest gap, and that gap fell **380 → 60** in
`3c46ddad` (2026-08-31), one day after `SOIL_UPTAKE_PER_TICK` landed in
`aef0a53d` (2026-08-30). Per-cell uptake loss went **86% → 14%**.

So a crowded root and an isolated root now drink essentially the same water.
**Root density buys nothing because the depletion zone was flattened, not
because water is the wrong resource.**

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

## 1. The revised ranking

| | first draft | now | why |
|---|---|---|---|
| **1** | immobile nutrient | **raise `SOIL_UPTAKE_PER_TICK` 6–10x** | one constant, no income re-derivation, restores the designed depletion zone (§0a) |
| **2** | — | **open the root-tip gate** | §2b — without it the plant cannot grow the roots anything rewards |
| **3** | — | **root turnover** | §2c — without it there is no interior optimum for any of this to find |
| **4** | *lower the water rates* | **withdrawn, for a corrected reason** | §1a |
| **5** | — | immobile nutrient | §3 — its unique job is smaller than the first draft claimed |
| — | economy reads attachment | unchanged, cheap, independent | predecessor §7a |
| — | price the pool by path | unchanged; girdling is its acceptance test | §8 |

### 1a. Why "lower the water rates" stays withdrawn, but not for the reason I gave

The first draft withdrew it as *"making water behave like an immobile
resource."* **That reasoning is backwards.** The equilibrium depletion a root
sustains is `g* = extraction / (faces x rate x wetness)`; *lowering*
`Absorb.rate` reduces extraction and makes water **more** mobile relative to
the root. Lowering `WATER_SCALE` shrinks the tank and touches depletion not
at all. Neither creates a depletion zone — they only move the knee, which is
the same arithmetic under a different name, and both move income and so drag
the five-constant re-derivation with them.

**`SOIL_UPTAKE_PER_TICK` is a different lever and the first draft never
named it.** It changes how fast the *soil* goes down and leaves income per
drink untouched — `absorb_water`'s own comment says so in as many words:
*"**Income is unchanged**… What changes is how fast the pond goes down…
this is a conservation fix, not an economy change."* `drawn` does not
reference it; only `taken` does.

And `dead-ends.md:892` names *raising the uptake price* and *a per-cell
depletion zone* as the **same** reopening condition. The first draft took the
second and discarded the first.

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

### 2b. The root-tip gate is shut wherever the water term is at its ceiling

`break_root_tips` (`plant.rs:7329`) returns early when
`water_status >= ROOT_REINITIATION_STATUS` (0.95, `plant.rs:5992`). On the
seeds where water status sits at 1.000 — the majority, 28 of 42 readings —
**the plant cannot re-initiate root tips at all.** A mechanism that rewards
root proliferation through a channel that cannot open is
`plant-appearance-design.md`'s failure repeated: a lever that fires, is
counted, and moves nothing.

Anything here must also wire its stress term into this gate and into
`root_weight` (`plant.rs:7892`), not only into income. Note `root_weight`'s
own comment prescribes the composition — *"The two stresses **add** rather
than multiply… either alone still moves it, which a product would not"* —
so a third additive stress is the house pattern.

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

## 9. What the first draft got wrong

Kept because how a plan went wrong is worth more than its conclusion.

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
