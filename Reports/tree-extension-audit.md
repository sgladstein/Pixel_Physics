# Why tree growth stops — measured

Instrumented audit against `Reports/tree-shape-problem-statement.md`.
8 trees / 14,000 frames / `ground=96`, 16-tree figures in brackets.

## The finding

**It is not `max_active_tips`, and it is not the scoring function. Growth
stops because the tip population is a sub-critical branching process, and
tips die of local carbon starvation while the tree around them is
carbon-rich.**

```
tips ever created   1419   (8 seeds + 1335 primary children + 76 branch children)
tips that grew      1335
tips that aged out    84            1335 + 84 = 1419  ✓
mean offspring per tip = 1411/1419 = 0.9944
```

16-tree ensemble: `(3135+157)/3308 = 0.9952`.

**Mean offspring < 1 ⇒ extinction with probability 1.** Both ensembles sit
a fraction of a percent below the critical value. Branching creates new
lineages at 76/1335 = **5.7%** per growth; lineages die at 84/1419 =
**5.9%** per tip. Survival depends on those two rates, and the death rate
is marginally higher. Per-tree counts confirm a random walk hitting an
absorbing zero — trees go extinct one at a time at frames ~2000, ~2000,
~5500, ~8500, ~8500, ~9500, ~10000, ~11000, and the world reaches 0 active
sites at frame 11,000.

`branch_chance: 0.1` is nominal only: the roll is gated on affording a
*second* cell, and **581 of 1335 grows (43.5%) could not**, cutting the
effective branch rate from 0.10 to 0.057. **Carbon scarcity attacks growth
twice** — once by killing tips, once by suppressing the only mechanism that
makes new ones.

## A tip is not a persistent object

`plant.rs:917` retires a tip to `MatureBody` **in the same tick it
successfully grows**; the child carries the frontier. So a tip gets at most
4 evaluations (`ORGANISM_STALE_LIMIT`), 45 frames apart, and either extends
once or dies.

| line | gate | sets `found_candidate`? | → staleness |
|---|---|---|---|
| `plant.rs:706` | `resource < cost` | **no** | +1 |
| `plant.rs:715` | `RootTip` allometry ≥ `MAX_ROOT_FRACTION` | **no** | +1 |
| `plant.rs:720` | `active_tip_count >= max_active_tips` | **no** | +1 |
| `plant.rs:817` | `candidates.is_empty()` | **no** | +1 |
| `plant.rs:820` | reached scoring with ≥1 candidate | yes | reset to 0 |

All four refusals are indistinguishable to the counter.

## Starvation vs no viable candidate

Of 84 `GrowingTip` retirements:

| last gate before death | count | share |
|---|---|---|
| `resource < cost` | 66 [141] | **78.6%** [81.5%] |
| `candidates.is_empty()` | 18 [25] | 21.4% [14.5%] |
| `max_active_tips` | 0 [7] | 0.0% [4.0%] |

Across all 2,700 `Grow` evaluations: 49.4% grew, **47.5% refused on
resource**, 3.1% on empty candidates, 0.0% on the tip cap.

## It is allocation, not economy — the decisive measurement

```
at any starvation refusal:   tip's own carbon  0.051   (cost = 0.2)
                             best neighbour    0.971   (4.9x cost)
                             79.2% starved beside a cell holding >= cost

at the moment of death:      dying tip's carbon 0.513  (2.5x cost!)
                             best neighbour     1.835  (9.2x cost)
                             88.1% [80.3%] died beside a cell holding >= cost
```

The probe reports `max resource 4.000 / 4.0` — cells pinned at the cap.
**The tree is not out of carbon. The tip is, and only the tip.** The carbon
sits one cell away at up to 9x the required cost.

Two compounding causes, both structural:

**1. A child is born with zero carbon.** `world.set` at `plant.rs:945`
registers a zeroed `OrganismCell`; nothing seeds it. **52.8% of all
starvation refusals are at exactly 0.0** — a tip's first evaluation,
burning one of its four lives before it has ever had income.

**2. Transport actively drains the tip.** `organism.rs:1118-1120` settles
at `carbon[j] = carbon[i] · c_ij/c_ji`. A fresh tip has basal conductance
on every face; its mature neighbour has a canalized face (up to
`CANALIZATION_CONTRAST` = 10x) pointing back into the trunk. The pairwise
carrier rule therefore parks the *newest, lowest-conductance* cell at the
*poorest* end of the gradient — measured 0.051 vs 0.971, a ~1:19 ratio.
**The tip is the poorest cell in the plant by construction of the transport
rule.** This is a direct consequence of the polarity work.

**3. And a plain bug on top.** `Grow` runs before `Photosynthesize` in
`tree.ron`, so a tip that refuses on carbon then earns income in the same
tick and retires holding **2.5x what it needed**. The staleness counter
counts `Grow` failures and is never reset by income arriving afterwards.
**These tips are killed by a counter their own economy had already
satisfied.**

This maps exactly onto the prior-art finding: the engine gives each bud a
**local reserve it owns**, and the reserve is drained by a gradient the bud
cannot win. Palubicki's `n = ⌊v⌋` — an allocation of 0.9 yields no metamer
and the bud neither grows nor dies — removes both failure modes at once,
because it removes the reserve *and* the death-by-idleness.

## Why candidate sets come back empty — direction, not crowding

Per-candidate, over 8,104 growable neighbours evaluated [19,273 at 16]:

| outcome | share | 16-tree |
|---|---|---|
| scored > 0 | 53.6% | 53.4% |
| rejected — **direction terms alone ≤ 0** | **42.8%** | 43.3% |
| rejected — crowding flipped an otherwise-positive score | 3.5% | 3.3% |

**Direction outweighs crowding 12:1.** Crowding is nearly inert, and the
probe's `max canopy 0.000 / 4.0` explains why: `CANOPY_DENSITY_DECAY_PER_
TICK` has erased the channel by the time it is read. **`crowding_weight:
0.5` is not what is refusing candidates** — and by extension, the crown
shyness change lands on a dead channel.

The direction rejections are geometric, not tunable: the ~4 directions
pointing back down the supply vector are negative by construction.

Of the 83 empty-set events: 53.0% had **all 8 neighbours occupied** — mean
growable neighbours per evaluation is only **3.00 of 8**, the tip already
walled in by its own thickened tissue — 41.0% had open cells but every
direction ≤ 0, 6.0% crowding.

## `max_active_tips: 14` — not binding

- **0 of 2,700** evaluations rejected by the cap at 8 trees; 25 of 6,602
  (0.4%) at 16.
- Peak simultaneous tips per tree: **10** at 8 trees.
- **Mean tips at `Grow` time: 1.45** [2.03].

**Trees spend most of their life at one or two tips, not fourteen.** A tree
at 1 tip is one unlucky 180-frame window from permanent death.

## Corrections to the problem statement

- §3's "`max_active_tips` caps concurrent tips at 14" is true but **inert**
  — never the binding constraint (0.0–0.4% of refusals).
- §3's "roughly 14 tips, each of which extends a short distance and dies"
  **understates it badly**: 1,419 tips were created and 1,335 extended
  successfully. The tree is not tip-budget-limited; it is **birth-rate
  limited by a hair**, at 0.994 offspring per tip.
- §4's premise "carbon fills every cell to `RESOURCE_SCALE`" is true of
  *mature* tissue and **precisely false of tips**. That asymmetry (tip
  0.05, neighbour 0.97, trunk 4.0) is the actual defect — and it means a
  bud-break rule keyed on local surplus would still fire on the wrong
  cells, because **the frontier is the one place surplus never appears.**
