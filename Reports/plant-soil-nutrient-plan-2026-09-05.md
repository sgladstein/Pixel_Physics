# Plan: an immobile soil nutrient, and what it is meant to fix

**Status: plan, 2026-09-05. Nothing built.** Successor to
[`plant-roots-and-transport-2026-09-05.md`](plant-roots-and-transport-2026-09-05.md),
whose §7 ranking this plan revises — see §1.

## 0. The one-sentence case

Roots in this engine are **nearly free and nearly useless**, so their
quantity is unpriced in both directions and nothing can select on it: they
cost `MAINTENANCE_PER_CELL` and no girth term (332 root cells = **0.65% of
income**), and the only thing they buy saturates at about **20** contact
cells while a tree grows **250–320**.

The comparison with real roots says the fix is not to make water scarcer.
**Water moves through soil toward a drinking root — in this engine
(`update.rs`'s capillary exchange, rest gap 60 of a 440-wide plant-usable
band) and in nature. If water were the only thing roots were for, real root
systems would be far smaller than they are.** What makes them large and
dense is the resource that *does not move*: phosphate's depletion zone is a
millimetre or two, so uptake tracks root length and density in unexploited
soil and never saturates.

This engine has **no nutrient dimension at all** — zero hits for
nutrient/nitrogen/phosph across `src/sim/` and `assets/species/`.

## 1. What this changes in the merged report's ranking

| | old §7 | revised |
|---|---|---|
| **1** | economy reads attachment | **an immobile soil nutrient** — this plan |
| **2** | *lower the per-cell water rates* | **withdrawn** — see §1a |
| **3** | `water_status` can't bind in a deep bed | folded into §1a: it is a symptom, not a cause |
| **4** | price the pool by path | unchanged, and **girdling is its acceptance test** (§6) |
| — | — | economy reads attachment — unchanged, still cheap, still right, and independent of all of the above |

### 1a. Why lowering the water rates is withdrawn

It works as a number and is wrong as a mechanism. Dropping `WATER_SCALE`
~30x and `Absorb`'s rate ~10x would move the saturation knee into the
250–320 range, but only by making water behave like an immobile resource —
using water as a stand-in for the thing that is actually missing. The
measured saturation is not a defect in the water model; it is the correct
consequence of water being the only soil resource in the world.

Two prior documents reach the same place from the design side and neither
was ever built against:

- `plant-simulation-research.md` §4: *"every FSPM needs at minimum carbon
  **and** water/nitrogen — they have different sources and different sinks,
  and collapsing them to one scalar removes the trade-off that makes
  allocation interesting."*
- `PLAN.md`: *"conserved currencies (water, biomass, **one nutrient**)"*, and
  — the sentence this plan is really built on — *"field channels are niche
  axes, and the axis count is what decides whether the world supports several
  plant strategies or exactly one winner."*

Neither appears in `dead-ends.md`. This is untried, not rejected.

## 2. What success looks like, stated before anything is built

Three claims, each with the instrument that would falsify it. **If the
mechanism cannot move all three, it has not earned its frame cost.**

| # | claim | measured by | falsified if |
|---|---|---|---|
| **S1** | Marginal roots pay. A plant with 2x the contact roots earns materially more. | `plant_severance`-style arm sweeping root mass against income | income flat in root count, as it is today |
| **S2** | Root **density** and **exploration** both pay, separately. | a new probe: uptake per root cell against local depletion | uptake per cell independent of how crowded that root's neighbourhood is |
| **S3** | Two root morphologies are both viable, in different soils. | `selection_arena` with a shallow-nutrient / deep-water bed | one arm wins in every bed |

S3 is the one that matters, and it is the owner's own stated goal for the
root line — *"create a system where these types of morphologies can develop
or evolve naturally… and should have different effects on the plant or
develop to fill an ecological niche"* (`root-morphology-findings.md`). With
one freely-flowing soil resource there is exactly **one** optimal root system
and it is small, which is why that line's review verdicts kept reading *"more
vs less roots instead of fully different morphology"*.

## 3. Storage: the decision this plan turns on

The nutrient must be **per soil cell** and must **not flow**. Both halves are
load-bearing.

**A coarse field channel is disqualified, and it is worth stating why rather
than leaving it as an option.** `FIELD_SCALE` is 16, so one field cell per
16x16 block, while the measured rooting zone is **13 rows deep**
(`labsoil`: roots max out at 13 rows over all 48 runs) — the entire root
system of a mature tree fits inside about one block vertically. A field
channel could not distinguish a 20-cell root system from a 320-cell one, so
it reproduces exactly the saturation being fixed. `CLAUDE.md`'s standing
warning applies on top: a coarse-field read is block-nearest, four sensors a
cell apart land in the same block ~7 times in 8, and building a per-cell
decision on the difference has already cost this project four separate bugs.

`Cell::aux` is unavailable: on a `Powder` with `water_capacity > 0` it is
held water on the `SOIL_SATURATED` scale, and `Cell` has already declined to
widen a third time.

So three candidates, for the review to adjudicate:

| | shape | cost | risk |
|---|---|---|---|
| **A** | parallel `Box<[u16]>` on `Chunk`, allocated lazily for soil-bearing chunks | 4,096 x 2 B = **8 KB/chunk**; ~20 soil chunks in the 512x320 world = ~160 KB, and it scales with *loaded* chunks under M10 exactly as `cells` does | dense allocation for a quantity most cells never use; a recharge pass over it is a full-world sweep |
| **B** | sparse `HashMap<(i32,i32), u16>` of **deficit**, keyed only on cells a root has actually drawn from | bounded by root contact cells x 4; 20 mature trees ~= 25k entries ~= 1.2 MB | hashing in `absorb_water`'s inner loop; unbounded growth if entries never retire |
| **C** | as B, but the entry stores `(deficit, frame_last_touched)` and **recovery is computed on read** | same as B | none new; removes the recharge pass entirely |

**The plan's own preference is C, and the reason is the recharge pass rather
than the storage.** A dense array invites a per-frame sweep over every soil
cell to recover nutrient, which is precisely the shape `CLAUDE.md` warns
about — new per-cell work in the sweep, on a quantity that is zero almost
everywhere. Lazy recovery on read makes recharge cost nothing when nobody is
looking.

**And there may be no recharge pass needed at all.** Litter already rots into
soil as discrete scheduled events (`decay.rs`, `open-bugs-handoff.md` §0e).
Crediting nutrient at exactly those events makes recovery event-driven,
bounded by the litter already falling, and closes the loop `PLAN.md` asks for
in as many words: *"plant → dies → decays → nutrient → plant"*. This is the
option the perf review should weigh hardest.

## 4. The mechanism, minimally

Deliberately small. No new `CellType`, no new growth rule.

1. **A second Liebig term.** `OrganismState::nutrient_status`, computed in
   `organism_upkeep`'s existing whole-plant walk beside `water_status`, by
   the same `settle_*` shape. Income becomes
   `intercepted x min(water_status, nutrient_status)` — **min, not product**:
   Liebig's law of the minimum is what the existing water term already
   implements, and a product would double-penalise a plant short of both.
   *(Open: the report's water term multiplies. Making them `min` changes the
   water term's meaning too. Flagged for review.)*
2. **Uptake.** `absorb_water`'s existing four-neighbour loop already visits
   every wet neighbour of every `MatureBody` cell and already writes the soil
   cell. Nutrient draw rides that visit — no new traversal, no new
   `World::get`.
3. **Depletion that does not flow.** The draw increases that cell's deficit
   and nothing equalises it. This is the whole mechanism: a root that sits
   still earns less over time, and the only way to earn more is to reach
   cells no root has touched.
4. **Recovery.** Event-driven at decay sites (§3), or lazy-on-read.

## 5. Shipping it inert, then turning it on by measurement

**This is a change to a weighted budget, and `CLAUDE.md` is explicit about
what that costs.** Adding a second multiplier on income lowers income, and
every constant calibrated against the current income —
`INCOME_PER_NODE`, `Grow.cost`, `MAINTENANCE_PER_CELL`, `pipe_ratio`,
`ROOT_BIAS_AT_FULL_WATER` — is calibrated against a number that just moved.
A correct mechanism at inherited constants is a regression.

So, following the `VEIN_GAIN = 0` discipline this repo already uses:

- **Phase 2 ships with the initial soil nutrient stock set high enough that
  `nutrient_status` is 1.0 everywhere**, i.e. the mechanism is present,
  exercised and inert. The gate is that the stand is unchanged — ideally
  bit-identical; if not bit-identical, the difference must be explained
  before proceeding.
- **Phase 3 lowers the stock as a measured sweep**, read at an order
  statistic over seeds (`seedsweep.sh`'s discipline; six seeds is not a
  sweep — §S2's 1.64x over six became a per-seed median of zero over
  eighteen).
- Re-derived constants land **in the same change** as the setting that
  requires them.

## 6. Girdling: the acceptance test for the path-priced pool (old §7d)

Worth recording here because it is the sharpest real-world case and this
engine cannot express it. Cut a ring of phloem and leave the xylem: the
crown stays green for weeks while the **roots** starve on their reserves,
and only then does the whole tree die. A graded, delayed, root-first death
from one cut.

Today the crown and the roots share one carbon pool with no direction, so
girdling is not merely unimplemented — it is **inexpressible**. It is also a
good verb by the ethos's own second law: a cut that does nothing visible for
a season and then kills the tree from the bottom up.

Not in this plan's scope. Named so the next session does not have to
rediscover it.

## 7. Explicitly out of scope

- Mycorrhizae, N-fixation, nutrient *species* (N vs P). One nutrient.
- Nutrient transport within the plant. It rides the same whole-organism pool
  carbon does; making that path-dependent is old §7d and is separate.
- Root turnover / fine-root death. Real fine-root production is often a
  third or more of NPP, and this engine's roots are built once and pay
  0.65% of income — that is the *cost* half of "unpriced in both
  directions" and it is a second, independent change. Naming it because
  fixing only the benefit half may simply move the optimum to "even more
  roots".
- The water tank size. Real storage is well under a day's transpiration and
  this engine's is 35x per-tick demand; shrinking it is right, cheap, and
  independent. Keep it out of this change so the sweep has one variable.
- The collar-drinks-for-free fix (`Absorb` on `MatureBody`). Straightforwardly
  unphysical — bark does not absorb water — and a one-line species edit, but
  it moves income and so belongs with its own measurement.

## 8. Frame-cost budget, stated up front

`ascii`'s worst-frame timing is the number to quote (`CLAUDE.md`), and the
bar is that this change must not cost the dirty-rect render skip or keep
chunks awake.

- Uptake rides an existing four-neighbour visit: **no new traversal.**
- Recovery must not become a full-world per-frame sweep. §3's whole argument.
- The nutrient must not wake a chunk. It is written by `absorb_water`, which
  today writes the soil cell through `World::set` and therefore already
  dirties — but if nutrient lives outside `Cell`, its write must **not**
  introduce a new dirty path on a settled bed.

**That last bullet is the one most likely to be wrong, and it is the perf
review's first question.**
