# The plant economy, re-derived: crown recession, the root blob, anchorage, and what kills an adult tree

**Status: shipped, package P2 of `plant-implementation-split-2026-08-23.md`.**
One re-derivation, not six changes. The crown and the roots are one carbon
economy; split, they produce two half-calibrated models that each compensate
for the other's error, which is the failure that report's §1.2 exists to
prevent.

Everything below was measured on this branch in one session. Where a number
is quoted against `main`, both arms were run on the same machine in the same
session — `CLAUDE.md`'s rule, after a change once read as a 25–50% frame
regression that turned out to be the machine.

**Six mechanisms in this package were built, measured and withdrawn.** They
are in `dead-ends.md` with their numbers, and §§2.1, 5, 6 and 8 below are
the accounts. That is most of the work and all of the interesting part.

---

## 0. What changed

| | before | after |
|---|---|---|
| **standing tissue** | free | pays maintenance respiration every organism tick |
| the girth term | — | `MAINTENANCE_PER_NODE × (q_peak / L_node)^1.5` on shoot tissue — Takenaka's exponent, on the monotone girth memory Phase 3 already computes |
| the mass term | — | `MAINTENANCE_PER_CELL` on every living cell, root and shoot |
| **the growth pool** | gross income | income **minus** the bill — NPP against GPP |
| `supportable` (bud break) | gross income / step cost | surplus / step cost |
| **income at night** | full, at every hour | `0.25 + 0.75 × daylight_fraction`; every *decision* stays phase-free |
| **water capacity** | `WATER_SCALE × root_cells` | `WATER_SCALE × contact_root_cells` — root cells sharing a face with soil |
| **a plant whose bill exceeds its income** | kept building | sheds its most distal abandoned tissue until the book balances |
| **anchorage** | not a quantity | `anchor_cells`, `anchor_moment`, `crown_moment`, `anchor_status`, `slenderness` — all free from walks that already ran |
| root allocation | `ROOT_BIAS_AT_FULL_WATER + (1 − water_status)` | `… + (1 − anchor_status)` |

Seven constants, every one derived from a measured distribution rather than
chosen: `MAINTENANCE_PER_NODE`, `MAINTENANCE_PER_CELL`,
`MAINTENANCE_EXPONENT`, `NIGHT_INCOME_FLOOR`, `MEAN_NIGHT_INCOME_FACTOR`,
`ANCHOR_DEMAND`, `MAX_DIEBACK_FRACTION`.

---

## 1. The headline: eight seeds, paired, before and after

`scripts/plantsweep.sh` is new and is the instrument. `seedsweep.sh` sweeps
a destructive verb over `filmstrip`; nothing did the same job for the plant
economy, and the plant economy needs it more — `CLAUDE.md` records twelve
identical trees from one genome spanning 31 to 153 cells, so one
`plant_probe` run is a draw from a very wide distribution. Genotypes are
drawn from `(world seed, germination coordinate)`, so varying `worldseed=`
re-rolls the whole population.

Eight world seeds, 8 founders each, ensemble medians:

| | 28,800 frames | | 45,000 frames | |
|---|---|---|---|---|
| | **before** | **after** | **before** | **after** |
| plant cells | 3,786 | **2,778** | 3,616 | **2,608** |
| wood cells | 2,600 | **1,953** | 2,867 | **2,021** |
| **foliage share of the plant** | 30% | **32%** | 19% | **23%** |
| stem thickness above the base | 15 | **13** | 17 | **13** |
| root cells | 401 | **266** | 574 | **367** |
| root share | 12% | 10% | 20% | 13% |
| **founders established** | 8 of 8, every seed | **8 of 8, every seed** | 8 of 8 | **8 of 8** |
| organisms born (total over 8 seeds) | 557 | 385 | 1,067 | 696 |
| organisms died | 230 | 166 | 617 | 414 |
| **inherited-genome establishments** | 1 | **0** | 2 | **0** |
| organisms senescent | 0 | **0** | 0 | **0** |
| canopy top (min over seeds) | 68 | 62 | 68 | 62 |

**Read the foliage share, not a wood-to-leaf ratio built out of two
medians.** `leaf %` is a median of per-plant shares and is the quantity that
means what it says; dividing the median wood count by the median cell count
divides two different plants and moved the *other way* on the same data. A
metric trap worth naming, because it very nearly went into this report.

**What the table says.** A tree is a quarter to a third smaller, its wood
falls faster than its foliage (share up 2 points at 28,800 and 4 at 45,000),
its trunk is thinner above the base, and not one founder failed to
establish on any seed at either horizon. No run hit the ceiling.

**And what it says that is not good news** is §7: turnover fell by about a
third, inherited-genome establishment went from a rounding error to zero,
and no adult tree died.

Die-back is not cosmetic at stand scale. Cells shed to starvation, per
stand, over 45,000 frames: **5,300 / 5,300 / 8,839 / 9,950 / 9,851 / 8,957 /
7,603 / 8,787**. Median bill-to-income across the eight stands runs 1.15 to
4.24, with individual plants at 1.99 to 24.07 — most trees are trimming,
some are receding hard, and which is which is not something any rule
assigned.

---

## 2. How the constants were derived

A price cannot be set by looking at what the price does — that fits the
number to its own feedback. So the charge was switched to exactly zero,
while `OrganismState::maintenance_basis` went on accumulating the quantity
it scales (`Σ (q_peak / L_node)^1.5`, the bill at unit price), and the
population it would be charged to was measured first. Four world seeds,
28,800 frames, night income and the contact-water change already in:

| | income / tick | bill at unit price | anchor reach ratio |
|---|---|---|---|
| median plant | 1.80 – 2.88 | 41,000 – 69,000 | 0.0148 – 0.0279 |
| range across plants | 0.03 – 5.52 | 222 – 197,000 | 0.0007 – 0.0721 |

Two things fall straight out. The bill's five-orders-of-magnitude spread
across plants in one stand is the exponent doing its work — it is a size
penalty, not a tax. And `ANCHOR_DEMAND` has to sit near 0.04 for
`anchor_status` to land mid-range on a typical plant: a term reading 1.00 on
everything is a term nothing can select on, which is the failure `CLAUDE.md`
records for three architectural levers that all fired and moved nothing.

The pressure itself — both maintenance constants moved together as one
multiplier, rebuilt at each point, every point printing the full diff of the
file it was about to build — was swept over ×0.5 / ×1.0 / ×2.0 with
establishment holding at 8 of 8 throughout and the median bill-to-income
running 0.3 / 0.6 / 1.3 at the noon-phase horizon the sweep used. ×1.0 was
taken, with headroom: at ×2.0 the *median* plant is past its sustainable
size, and a stand whose typical tree is on that side of the line is a stand
on its way out.

*(That sweep predates §5's phase fix, so its bill-to-income figures are
noon-phase readings and are about half the true values; the establishment
column and the ordering are unaffected, and §1's table is the shipped
configuration measured after the fix.)*

---

## 3. Crown recession, and the two designs it took

### 3.1 The first design was wrong, and the way it was wrong is the point

The obvious rule: charge each cell its bill, and let a cell that cannot pay
die if it carries no living foliage — `q_now == 0`, the live basipetal sum
beside the monotone `q_peak` the bill is charged on. It reads correctly. It
took the stand apart.

Four seeds, 28,800 frames:

| | charge off | first design |
|---|---|---|
| founders established | 8 of 8, every seed | **4 – 6 of 8** |
| median plant | 3,437 cells | **704 – 2,569** |
| worst seed | — | 97 cells, **94% root mass** — the shoots had gone |

**`accumulate_support` walks a spanning tree, and a thickened trunk is a
blob of cells rather than a tree graph** — its own doc says so. So
`q_now == 0` does not mean "carries no foliage"; it means "is not on the
arbitrary path this tick's walk happened to take", which is true of most of
a trunk's girth and shifts as the plant grows. The rule was reading a
traversal artifact as a biological fact and eating trunks with it. That is
`CLAUDE.md`'s *which object does this rule evaluate — a cell, a section, or
a whole piece?*, missed again by a session that had the paragraph in front
of it.

`q_now` is kept — P5's resprout wants it and it costs one `f32` — with the
caveat written onto the field.

### 3.2 The design that works: the plant decides how much, geometry decides where

- **How much**: `maintenance > income` is a property of the *plant*. Keyed
  instead on the sum of per-cell shortfalls, the rule chewed a healthy
  growing tree continuously — **90 of 1,124 cells still reachable from the
  base** — because `organism::transport` is deliberately slow
  (undifferentiated parenchyma conducts at 0.008 against the flat 0.2 it
  replaced) and distal cells in a plant with plenty of stock run momentarily
  short all the time.
- **Where**: most distal first (`path_len`, stamped at creation and never
  recomputed), then most cantilevered (`support`, which `anchor_support`
  already writes), then row-major.

Three exclusions, each a measured failure rather than caution:

1. **Never foliage.** Shedding foliage is abscission's job and it already
   has two graded rules. `is_foliage` asks the *species*, never
   `CellType::Leaf` — the §F4-shaped trap P3 recorded. Grass photosynthesises
   from `MatureBody`, so a cell-type test read every blade of a sward as
   inert structure: **0 of 12 blades standing** where the guard expects 12,
   and the sod that holds a river bank went with it (+5% crest retention
   against a recorded +27%).
2. **Never a cell with something hanging further out than it** (a neighbour
   of strictly greater `path_len`). Without it, a bare branch carrying a
   leafy twig was all candidates *behind* the twig, so the plant shed the
   branch and left the twig floating: connectivity fell to **52% of a
   1,601-cell tree** and stayed there for four thousand frames.
3. **Never a cell whose removal would locally disconnect its neighbours** —
   the standard *simple point* test from topology-preserving thinning. Even
   with (2), seven cells of 1,321 came adrift, because a thickened cell
   *inherits* its neighbour's `path_len` rather than incrementing it, so
   girth beside a shed cell was unprotected. Die-back is now an erosion that
   cannot change the plant's topology.

**A receding crown and a tree coming apart are the same cell count and the
same picture.** `print_crown_recession_trajectory` prints the connected
fraction beside the book, which is the only thing that separates them:

| frame | cells | leaves | income | bill | deficit | shed, cumulative | connected |
|---|---|---|---|---|---|---|---|
| 1,000 | 178 | 109 | 0.069 | 0.024 | 0 | 0 | 100% |
| 3,000 | 1,439 | 918 | 1.040 | 0.513 | 0 | 9 | 100% |
| 4,000 | 2,039 | 1,275 | 1.731 | 0.716 | 0 | 9 | 100% |
| 5,000 | 2,141 | 1,308 | 0.002 | 0.731 | 0.729 | 74 | 100% |
| 8,000 | 1,892 | 976 | 0.003 | 0.795 | 0.791 | 359 | 100% |
| 12,000 | 1,576 | 646 | 0.002 | 0.748 | 0.746 | **994** | **100%** |

That individual is in a 17×8 walled bed and is dying of thirst, which is the
scene rather than the mechanism — the point of the table is the last column
beside the one before it. Nine hundred and ninety-four cells shed, and not
one fragment.

### 3.3 A living plant trims; a dead one rots

Die-back is skipped once `senescent` is set. Without that split the two
raced, die-back was much the faster, and it took apart four `structural.rs`
fixtures whose subject is the support model — a hand-built beam of six wood
cells has no foliage by construction, so it reads as a plant in total
deficit. The sequence a dying tree follows is unchanged: income falls,
die-back trims it toward what it can carry, the last foliage goes, and only
then does it stop being a plant that is shrinking and start being remains.

---

## 4. The root blob: what pricing contact does, and what it does not

The owner's directive, card `20260823T163504317Z-3cef7b`: *"There should be
a disadvantage for growing a big blob of roots that fully fills in all
space. If the root cell isn't touching soil it cannot benefit the plant and
has a cost… As usual we don't want to force the roots to grow a certain way
but set up a system that leads to interesting and heterogenous results."*

Both halves are now real. **Earns nothing**: `water_capacity_of` read root
*mass*. `absorb_water` already credited a walled-in cell nothing — it finds
no wet neighbour — but the plant's whole water store was still sized off
every root cell it owned, so interior root tissue was buying storage for
free. It reads `contact_root_cells` now, counted four-neighbour in the walk
that was already running, because an exchange crosses a face. **Costs
something**: `MAINTENANCE_PER_CELL`, the price every other living cell pays.
Roots need no bespoke constant.

**And it is not the brake, which is the part to keep saying out loud.**
`root-blob-and-uptake-surface-2026-08-23.md` measured the walled-in interior
at 33.1% / 36.1% / 33.3% at 10,800 / 25,200 / 43,200 frames — root cells
nearly quadruple across that range while the interior share holds at about
one third. A soil-contact price is therefore a **flat ~33% tax on root mass,
not a bound on it**, and nothing here changes that. Anything that stops the
mass growing has to be scale-dependent; §8 is what this package found about
that.

**What it does buy is the thing that was actually asked for.** That report's
strongest finding is that per-plant contact already spanned **51%–79% at
comparable mass, same genome, same scene**, and nothing priced it, so
nothing could select on it. After the change, eight seeds at 45,000 frames,
per-plant soil contact:

| seed | min | median | max |
|---|---|---|---|
| 1 | 51.0% | 65.5% | 73.6% |
| 2 | 50.8% | 72.2% | 79.6% |
| 3 | 51.4% | 69.5% | 74.3% |
| 4 | 50.8% | 66.5% | 88.4% |
| 5 | 47.6% | 64.7% | 72.9% |
| 6 | 56.6% | 64.3% | 94.0% |
| 7 | 56.9% | 70.6% | 87.2% |
| 8 | 48.6% | 71.6% | 83.2% |

**The spread survives.** Pricing the difference did not collapse the
population onto one efficient morphology — 47.6% to 94.0% is at least as
wide as the 51%–79% measured before anything was priced. That is the "system
that leads to interesting and heterogeneous results, not every plant root
eventually grows into the same blob" the directive asks for: an existing
spread converted into a fitness difference, with no rule anywhere saying
what shape a root should be.

---

## 5. Night income, and the oscillator bug this package shipped and caught

The 2026-08-17 directive: income runs at `0.25 + 0.75 × daylight_fraction`,
decisions stay noon-normalised.

It reaches exactly two places — the photosynthetic credit in
`organism_upkeep`, and the growth pool in `allocate_to_frontier` — and
deliberately not `break_buds`' `supportable`, which is a policy. A
`supportable` that fell at dusk would retire the frontier every night, which
is the nightly extinction event `field::noon_equivalent_light` exists to
end; the measured form of that failure is a live tip count of 71 at noon
against 28 at night on one stand.

A day's mean factor is **0.49** — the sun is a clipped hump, so over half
the cycle sits at the floor — so this is close to halving gross income. That
is why it lands inside the single re-derivation rather than after it. The
charge-off control in §2 separates its effect from the bill's: night income
and the contact-water change together cost **9%** of median plant mass
(3,786 → 3,437) with establishment untouched.

**And then the same rule was broken one function away.** Die-back compares a
plant's standing bill against its income. The bill is charged every tick and
does not care what time it is; income was night-scaled. So the comparison
read four times worse at midnight than at noon, and a stand shed on a
nightly cycle.

**The tell was in the numbers before it was in the code.** One build
reported a median bill-to-income of **0.6** at 28,800 frames and **2.6 to
5.6** at 45,000. Two figures four-fold apart from one binary — because
28,800 is exactly eight day/night periods and 45,000 is twelve and a half.
`OrganismState::income` is stored noon-equivalent now, `MEAN_NIGHT_INCOME_
FACTOR` turns it into what a plant collects over a cycle, and the hour
enters the model in exactly one place: the pool, where carbon actually
moves.

---

## 6. §U: still open, and now with a measured account of why

P1 instrumented the exit §U hangs on — `ROOT_TIP_POOR`: thirsty, under the
tip cap, sites available, and no cell holding the price — and measured it at
**0 in every arm**. **Maintenance respiration does not close it.**
Re-measured with the whole economy in
(`print_drought_grows_bigger_with_root_tip_counter`, 12,000 frames):

| bed | soil moisture | cells | wood | root | calls | gated | at cap | no candidate | **poor** | fired |
|---|---|---|---|---|---|---|---|---|---|---|
| 17×8 | 310 (dry) | 1,871 | 1,159 | 142 | 266 | 101 | 0 | 1 | **0** | 164 |
| 17×8 | 620 | 3,013 | 1,824 | 70 | 266 | 71 | 0 | 1 | **0** | 194 |
| 61×30 | 310 (dry) | 2,681 | 1,483 | 217 | 293 | 183 | 2 | 1 | **0** | 107 |
| 61×30 | 620 | 3,916 | 1,942 | 154 | 478 | 297 | 0 | 1 | **0** | 180 |

The exit is **structurally unreachable**, not merely rare: the bill is paid
out of the richest standing cell, a mature trunk sits at `RESOURCE_SCALE`
whatever the plant's book says, and draining it takes far longer than a
tick.

**The fix §U names was built and withdrawn.** Gating the amplifier on
whole-plant solvency — the same shape as the pool and as `supportable` — has
a fatal interaction with a maintenance economy: a plant at its maximum
sustainable size is insolvent *by construction*, so the gate shut root
re-initiation on essentially every mature plant and produced the exact death
spiral `allocate_to_frontier` documents. Four seeds, 28,800 frames, gate on
against off:

| | gate on | gate off |
|---|---|---|
| founders established | **6 – 8 of 8** | **8 of 8, every seed** |
| median root cells | 156 | 305 |
| median plant | 1,979 cells | 2,729 cells |

Where the penalty lives instead: `water_status` multiplies income, income
nets maintenance, and the growth pool is what is left. A thirsty plant is
poorer at the pool rather than at the till. That is a different place from
the one §U names, and it is the one that does not spiral. **§U is not
closed**, and a replacement needs a gate that can tell "at its ceiling" from
"in trouble" — the same distinction `break_buds`' own defect note needs, and
what `q_peak`'s high-water memory exists to make.

One thing the table settles in passing: drought grows a **smaller** tree on
both beds (1,871 against 3,013; 2,681 against 3,916), agreeing with P1's
8-seed sweep and disagreeing with §U as filed. That half of §U stays
retired.

---

## 7. Adult mortality: the cause is built, it fires, and nothing dies

P3 handed this package the cause and said so: *"Nothing kills a healthy
tree; a mature tree always holds dormant buds, so it is never senescent,
which is correct. The cause arrives with P2's superlinear maintenance
respiration."*

**The cause is here and it fires hard** — 5,300 to 9,950 cells shed per
stand over 45,000 frames. **Nothing dies.** `senescent` reads **0 on every
one of eight seeds at both 28,800 and 45,000 frames**, exactly as it did
before this package.

Three things block it, and only the third is a defect:

1. **A plant at a light- or water-limited ceiling reaches a genuine small
   equilibrium rather than dying, and that is correct.** Traced with a lid
   dropped over a grown tree (`print_a_shaded_tree_against_a_lit_one`,
   `print_crown_recession_trajectory` with `RECESSION_LID=`): a tree with no
   light sheds its way down to a stump — 1,848 cells and 512 leaves lit
   against 836 and 163 dark — and then holds. A tree in a pot stays small;
   it does not die of being in a pot.
2. **Dormant buds keep a plant `is_vital` indefinitely**, which is P3's own
   observation and is right. A stump with 120 buds is a plant waiting for a
   good year, not a corpse.
3. **A compact stump has almost no erodible cells.** Die-back is a
   topology-preserving erosion (§3.2), so a dense blob loses only cells
   whose removal cannot disconnect a neighbour, and a stump is nearly all
   interior. This is the defect: the exclusions that make die-back safe on a
   crown make it nearly inert on a stump.

**What would move it**, in the order the evidence supports:

- **The free-thickening treadmill, §8.** A starving plant re-lays almost
  exactly what die-back removes. Until secondary growth costs something,
  standing tissue is not really bounded.
- **Disturbance rather than economics.** Wind-throw (lane S, T5) and
  grassfire (W2) kill trees where they stand. §4 of the physical-trees
  addendum already calls wind-throw "a selective *death*, which is the
  strongest kind of pressure and the one this model has been short of".
- **Competitive exclusion**, which needs recruitment into the understorey,
  which needs seeds establishing — see §9, where the number went the wrong
  way.

---

## 8. What was built and withdrawn: charging secondary growth

`SecondaryThicken` lays wood for nothing, and that is the reason superlinear
upkeep bounds a plant's *size* without bounding its *tissue*. Measured on a
tree with no income left: **5,914 cells shed over 60,000 frames for a net
loss of ten**, because it re-laid almost exactly what die-back removed. It
stood there for ever, which is the state §7 is about.

Charging it one growth step per cell laid, from the thickening cell's own
carbon and gated on whole-plant solvency, was built and measured. It removes
the treadmill outright (5,914 shed becomes 120). It also:

| | baseline | charge on, ×1.0 | charge on, ×0.5 |
|---|---|---|---|
| founders established | 8 of 8 | **6 – 8 of 8** | 8 of 8 |
| median plant | 3,786 | 2,067 | 3,080 |
| **foliage share** | 30% | 31% | **32%** |
| median root cells | 401 | 130 | 335 |

At full pressure it costs establishment; at the pressure that restores 8 of
8 it is barely distinguishable from the no-charge configuration on the one
metric it was supposed to move, and at neither setting did anything become
senescent. Withdrawn, and in `dead-ends.md` with its re-test condition: the
charge wants to come out of the same allocation pool extension draws from,
not out of the thickening cell's own carbon, which `organism::transport`
refills within a tick or two.

---

## 9. Selection throughput: this package makes it worse

The number that gates every downstream evolution claim
(`plant-evolution-design.md` §5, quoted in `world.rs`'s own doc):
**established plants carrying an inherited genome.**

| | before | after |
|---|---|---|
| 28,800 frames, 8 seeds | 1 | **0** |
| 45,000 frames, 8 seeds | 2 | **0** |
| organisms born, 45,000 frames | 1,067 | 696 |
| organisms died, 45,000 frames | 617 | 414 |

**It was ~0 and it is now 0.** Turnover fell by about a third, for a reason
that is not subtle: `Reproduce` fires per mature cell, so a plant's fecundity
is its canopy size, and this package makes every plant a quarter to a third
smaller. Fewer seeds, and no founder dies to make room for the ones that
germinate.

So: **no selection or adaptation claim can be made for trees on this
branch**, and none is made. Every tree figure in this report is about the
eight individuals somebody planted. P1 and P3 both said this; P2 does not
change it and, on this measure, moves it the wrong way. What moves it is
founder mortality (§7) and disturbance, not economy tuning.

---

## 10. Frame cost

`ascii`'s tree scene, both binaries built and run in the same session on the
same machine, alternating:

| | worst frame, growing | worst frame, settled |
|---|---|---|
| before | 1.352 / 0.667 / 0.646 ms | 0.284 / 0.427 / 0.353 ms |
| after | 0.676 / 0.660 / 0.662 ms | 0.317 / 0.649 / 0.407 ms |

**No detectable change**: the spread of repeated runs of one binary (0.646
to 1.352 ms on the *before* arm alone) is larger than any difference between
the arms. That is the reason `CLAUDE.md` insists on re-measuring the
baseline in the same session — an hour-old figure here would have supported
either conclusion.

The work added is bounded and sits inside walks that already ran: one
`f32` store per cell in `accumulate_support`, a four-neighbour soil test per
*root* cell in the upkeep walk, two running sums in `anchor_support`'s
seeding loop, and one eight-neighbour candidate scan **only over plants
running a deficit** — guarded at the call site on a number the same walk
produced, so a plant in surplus pays one float compare.

---

## 11. What this does not answer

- **Whether the bole reads as clearer on screen.** The numbers say the trunk
  is thinner above the base and the foliage share is up; whether a stand
  *looks* like trees with boles is a card, and cards are the only acceptance
  channel this project has for that.
- **Whether anchorage changes root morphology or only root mass.** The
  contact spread survives (§4) and `anchor_status` is live across the
  population, but nothing here renders two root systems side by side. Lane
  B's re-run was queued behind P2 for exactly this and now has a priced
  economy to run against.
- **Anything about wind.** `anchor_status`, `crown_moment` and `slenderness`
  exist and are read by the allocation rule only. No structural check is
  scheduled from any of them; lane S owns the storm.
- **Whether `MAX_DIEBACK_FRACTION` binds.** It is a pace limit and the
  measured limiter is candidate scarcity, not the cap (§7.3). It has never
  been the binding constraint in any run measured here, which means it is
  currently unfalsified rather than validated.
- **Grass and the other species.** Every figure here is `tree`. The economy
  is species-independent by construction, and `is_foliage` is the one place
  that has already bitten (§3.2), but the sweep has not been run on
  `conifer`, `shrub`, `creeper` or `grass`.

## 12. Freshness

Written 2026-08-23 against `main` at `c0ba0b3` (P3's merge). Every number is
measured on this branch in one session except the three quoted from
`root-blob-and-uptake-surface-2026-08-23.md` §2, which are cited as that
report's.
