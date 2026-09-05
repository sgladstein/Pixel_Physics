# Roots, veins, and what actually moves through a plant

**Status: measured diagnosis, 2026-09-05. No mechanism changed.** Two new
instruments (`examples/plant_reach`, `examples/plant_severance`) and the
numbers they produced. The repairs in §7 are ranked and costed, not built.

Written to answer the owner's report:

> Many plants can grow just fine with tiny or without any roots at all. In
> fact I have seen plants continue to grow after the trunk has been fully
> severed near the base. What exactly are the roots doing, what is the vein
> conductance, how are things being transported through the plant because
> this doesn't seem correct.

Both observations reproduce, and one of them needs a correction the owner
will want: **tiny roots are fine, no roots is not.** The threshold is around
**twenty** contact root cells, and a mature tree here carries 250-320 — so
everything past the first couple of dozen is inert, which is what makes them
look free. §3a.

The severance half reproduces exactly, and only in one configuration: with
COLLAPSE UNDER LOAD off. §6.

## 1. The short answer

A plant has **two transport systems, and the one that funds growth is not
the one that models plumbing.**

| | what it models | scope | what reads it |
|---|---|---|---|
| `organism::transport` (`organism.rs:6384`) | Sachs canalization — per-face efflux conductance, flux reinforcing flux | strictly local: one shared face per substep | `supply_direction` — a **direction**, for `Grow`'s `away_from_supply` and `thicken`'s axis |
| `plant::allocate_to_frontier` (`plant.rs:7655`) | a whole-organism budget | **global: any cell to any tip, in one tick, with no distance term at all** | every growing tip's actual carbon |

Water has no transport system whatsoever: `OrganismState::water` is one
`f32` for the entire plant (`organism.rs:4057`).

So *"how are things being transported through the plant"* has an answer, and
it is the source of the complaint: **for the purpose of growth, they are not
transported. They are pooled and assigned.** Topology enters the carbon
economy only as a tie-break on direction, and enters the water economy not
at all.

## 2. What the roots are doing

Four channels. Only one of them is mechanically root-specific, and the one
the design leans on hardest is saturated in the shipped world.

### 2a. Water income — and it does not need roots

`absorb_water` (`plant.rs:467`) credits `OrganismState::water`. It is
dispatched from `Behavior::Absorb`, and `tree.ron` declares
`Absorb(rate: 1.5)` on **`MatureBody`** (`assets/species/tree.ron:510`) as
well as on `RootTip`. The function's own doc is explicit that this is
deliberate:

> It runs on every `MatureBody` cell, not only root ones… A collar cell
> half-buried in damp ground drinking a little is right, not a bug.

It was widened for a good reason — `tree.ron` caps root tips at 10, so a
`RootTip`-only income is bounded by a constant while demand scales with a
canopy of a thousand leaves. But the consequence is that **any mature cell
with a water-bearing neighbour is a water intake for the whole plant.** A
shoot standing in damp soil with no root system at all still drinks.

### 2b. Water storage capacity — root-only, with a floor

`water_capacity_of(contact_root_cells) = WATER_SCALE × contact.max(1)`
(`plant.rs:750`). This *is* root-specific, and it is the coupling the design
intends: a deep, wide root system buys a drought buffer.

The `.max(1)` floor exists so a fresh seedling has somewhere to put its
first drink. It also means a plant with **zero** contact roots still has a
tank.

### 2c. An allocation bias, which is self-referential

`plant.rs:7891`:

```rust
let anchor_stress = 1.0 - state.anchor_status;
let root_weight = (ROOT_BIAS_AT_FULL_WATER + (1.0 - status) + anchor_stress) * genotype(..., 6, ...);
```

Roots buy a larger share of the growth pool for more roots. Real, but a
closed loop — it changes root:shoot, not what the plant can do.

**`(1.0 - status)` is identically zero in the shipped bed** (§3), so the
plastic half of functional balance — the thing `wiki/plants.md` describes as
*"a plant that is short of water spends its growth on roots instead of
canopy"* — never fires there.

### 2d. Structural anchorage — the one channel that is genuinely root work

`is_structural_anchor` (`plant.rs:6046`) seeds `anchor_support`'s Dijkstra
from any organism cell touching a `Solid` that `anchors_organisms`, **or**
root tissue touching water-bearing `Powder`. So roots are what hold a plant
into soil, and that is real.

Note what `anchor_status` — the measure of whether the anchor plate matches
the crown — is actually used for. Its **only** consumer is the allocation
bias in 2c (`plant.rs:7891`). It does not reach the structural model at all:
a plant carrying a huge crown on a hopeless plate does not fall over, it
budgets more to roots.

## 3. Why roots read as optional: the one channel that pays saturates in the first two dozen cells

`water_status` is the entire economic coupling between having roots and
being able to grow — `plant.rs` says so in as many words at the settle
(*"this single number is the entire coupling"*), and it multiplies every
leaf's contribution to income:

```rust
intercepted += ambient_light_above(world, x, y) * water_status(world, x, y);   // plant.rs:7711
```

Measured here, `plant_severance arms=control seed=1`, four mature trees
tracked from frame 24,000 to 48,000:

| frame | cells | shoot | root | contact | water / capacity | **water_status** | demand | income |
|---|---|---|---|---|---|---|---|---|
| 24,000 | 4,778 | 4,560 | 332 | 252 | 776.6 / 1,008 | **1.000** | 28.95 | 7.613 |
| 32,000 | 5,269 | 4,996 | 397 | 272 | 202.6 / 1,088 | **1.000** | 28.84 | 7.668 |
| 40,000 | 5,726 | 5,310 | 402 | 273 | 660.3 / 1,092 | **1.000** | 28.86 | 7.281 |
| 48,000 | 4,827 | 4,333 | 499 | 317 | 180.9 / 1,268 | **1.000** | 25.88 | 5.731 |

Pinned at 1.000 at every stop, through a stock that swings 180 → 777. This
is not new and it is not this bed: `plant-water-scarcity-2026-08-30.md` §2c
measured 0.966–1.000 across a **6.3×** range of plant-available soil water,
and §2d found the one thing that does move it is **rooting volume** — a
four-row skin of soil over stone takes it to 0.678. In a deep bed a stand
cannot exhaust the water, so the root channel cannot bind.

### 3a. The number that says it: a tree grows ~15x the root system its water demand needs

Two root-dependent terms stand between a plant and its water, and both are
linear in contact roots, so both have a threshold that can be read off
against the **~29** per-tick demand the control arm measures:

- **the tank** — `water_capacity_of` is `WATER_SCALE x contact_root_cells`,
  i.e. **4.0 per contact cell**, so it covers the bill at about **7** cells;
- **the refill** — `absorb_water` credits at most `rate x available` per
  water-bearing neighbour, and `tree.ron` sets `rate: 1.5`, so at roughly one
  wet neighbour per root cell it covers the bill at about **20**.

The trees in that table carry **252 to 317** contact root cells, and their
`uptake` column sits at 28.9 against a demand of 29.0 — demand-limited, not
supply-limited, with an order of magnitude of slack on whichever term is
tighter.

That is the whole of "plants grow fine with tiny roots": the smallest root
system that fully meets a mature tree's water demand is somewhere around
twenty cells, and the tree grows fifteen times that. Every contact root past
that point buys nothing the economy can see, so nothing can select on it.

**But "tiny" and "none" are not the same, and the difference is measured.**
The `deroot_noload` arm (all 1,519 root cells removed, structural response
switched off so the economy is what is being watched) drops `contact` to 0,
capacity to the `.max(1)` floor of 4.0 against a demand of 28.4, and
`water_status` to **0.021** in one organism tick. Income collapses 7.613 ->
0.199 and the plant dies out by frame 36,000-40,000. So the coupling is
real; it is just that it is spent within the first couple of dozen cells.

**The root system the economy settles on is correspondingly small**: 332
root cells against 4,560 shoot at frame 24,000 is a **6.8%** root share,
against `MAX_ROOT_FRACTION`'s ceiling of 50% and a real tree's 20-50%. The
ceiling is not what is binding; the allocation weight is, and its plastic
term `(1 - water_status)` is identically zero everywhere above that
threshold.

## 4. What the vein conductance is, and how far it reaches

The model (`organism.rs`, "Polarity: canalization of the carbon channel") is
Sachs canalization as formalized by Mitchison and Prusinkiewicz et al.
(2009). Each cell stores four **efflux** conductances, one per face; flux
through a face reinforces that face:

```
c ← c + VEIN_BASAL + VEIN_GAIN·Φ(J) − VEIN_DECAY·c,   Φ(J) = J² / (J_ref² + J²)
```

with `VEIN_BASAL = 0.1`, `VEIN_GAIN = 0.9`, `VEIN_DECAY = 0.1`. So
conductance runs from `CONDUCTANCE_MIN = 1.0` (undifferentiated parenchyma)
to `CONDUCTANCE_MAX = 10.0` (a fully canalized strand) — a 10:1 contrast.
Carbon then moves by `TRANSPORT_RATE = 0.024` per substep times conductance,
over `CARBON_SUBSTEPS = 16` per organism tick.

The design is sound and the asymmetry is the good part: efflux is stored per
ordered cell pair, which is what lets a branch point carry flux both ways
without the data structure deciding apical dominance in advance.

**But its reach is nowhere near a tree.** `examples/plant_reach` charges the
bottom four rows of a grown tree's root system to `RESOURCE_SCALE`, zeroes
every other cell, and then runs `organism::transport` **and nothing else** —
so anything that appears higher up arrived through the conductance model,
because there is nothing else in the world to put it there.

Seed 1, plant of 6,257 cells spanning rows 88…233 (145 rows), 76 cells
charged, finite source:

| ticks | frames | usable front | rows advanced | total carbon |
|---|---|---|---|---|
| 0 | 0 | 230 | — | 304.0 |
| 10 | 450 | 222 | 7 | 270.7 |
| 20 | 900 | 218 | 11 | 256.1 |
| 30 | 1,350 | 213 | 16 | 249.1 |
| **40** | **1,800** | **209** | **20** | **245.9** |

The crown top is **141 rows above the charge** (the harness prints that
distance itself, and the `advanced` column is measured on the same basis). In
1,800 frames the usable front — carbon at a tenth of `RESOURCE_SCALE`, itself
half of `tree.ron`'s `cost: 0.2` for one growth step — advances **20 of those
141 rows, and then stops**: front and total are both flat across the last four stops, which is
the settling check, not a truncated run.

The falling total is the instrument's own control behaving: `transport`
clamps each cell at `RESOURCE_SCALE`, so inflow into an already-full charged
cell is clipped. A total that *rose* would have been a probe bug.

### 4a. The held-source control, which is the arm that settles it

A finite charge necessarily stalls once it has spread, so the table above is
arguable on its own. `hold` refills the seeded band to `RESOURCE_SCALE` after
every tick — a root system that keeps drinking rather than one tankful — and
the total carbon column shows it working: **304 → 971**, climbing at every
stop, so the plant is being fed throughout.

| ticks | frames | usable front | rows advanced | total carbon |
|---|---|---|---|---|
| 10 | 450 | 220 | 9 | 608.6 |
| 20 | 900 | 214 | 15 | 781.5 |
| 30 | 1,350 | 209 | 20 | 877.6 |
| **31–40** | 1,395–1,800 | **208** | **21** | 888 → 971 |

**Twenty-one of 141 rows, and then flat for ten consecutive stops while the
source keeps pouring carbon in.** That is not a source running dry; it is the
steady state. The gradient establishes, and past ~21 rows the vein model
simply does not deliver an amount a tip could spend.

**This is the same wall the water channel hit and was withdrawn for.**
`OrganismState::water`'s own doc records per-cell water with symmetric
transport being built and measured: *"Water entered at the roots and never
arrived… foliage median 0.00… a mature tree is ~130 rows from root to crown,
needing thousands [of substeps]."* Carbon's configuration is **slower** than
the water one that failed — 16 substeps at 0.024–0.24 against 45 at 0.2 —
and it only appears to work because §5 bypasses it.

## 5. How growth is actually funded

`allocate_to_frontier`, once per organism tick, before anything spends:

1. **Donors** are every non-frontier cell of the organism — no position test
   (`plant.rs:7712`, `:7714`).
2. **Income** is `Σ over leaves of (light × water_status) ÷ L_node ×
   INCOME_PER_NODE`; **surplus** is `(income − maintenance).max(0).min(stock)`;
   **pool** is surplus less the reproductive share.
3. Each tip's **share** is `pool × weight / total_weight`.
4. The share is drawn **from the richest donors in the whole plant, sorted
   globally by carbon** (`plant.rs:7910`), and then the tip is written:

```rust
write_carbon(world, fx, fy, share.min(organism::RESOURCE_SCALE).max(held));   // plant.rs:7924
```

There is no distance, no path, and no conductance anywhere in that. And
because step 4 is an **assignment**, it supersedes whatever `transport`
delivered unless transport already delivered more.

This is not gratuitous — it is what makes root tips fundable at all. A root
tip has no local carbon source, sits a hundred cells from the nearest leaf,
and `PLAN.md` records the pre-pooling behaviour: *"a root with no adjacent
water lives entirely off resource slowly diffusing over from the trunk, and
can permanently go dormant."* The pool fixed a real defect. It fixed it by
removing the plumbing from the answer.

## 6. The severance measurement, and the switch that decides it

`examples/plant_severance` grows a bed to frame 24,000, then applies one
treatment to every established plant and runs to 48,000:

- **`control`** — nothing.
- **`sever`** — the plant's own cells in the three rows immediately above the
  soil surface are emptied, cutting the shoot free of its root system. Its
  own cells only, not a radius of world: an axe bite also throws soil and
  rock, and the question here is what the **economy** does, not what falls.
- **`deroot`** — every root cell removed, the shoot left standing.
- **`sever_noload` / `deroot_noload`** — the same two, with
  `World::plant_load_failure` set false at the cut. See §6b.

`unreached` is the positive control on the cut: `anchor_support` writes
`u16::MAX` into any cell with no path to an anchor, so a sever that leaves it
at zero did not sever anything and every number beside it describes an intact
plant.

### 6a. At the shipped default, severance is fatal, and correctly so

**This refutes the hypothesis this report was started on.** The prediction
from reading the code was that a severed crown would go on being funded
indefinitely; measured, it does not, because the *structural* half removes it
first. Seed 1, four tracked plants, `plant_load_failure` at its default
`true`:

| frame | plants | cells | shoot | root | demand | income | | |
|---|---|---|---|---|---|---|---|---|
| **sever** — 483 cells cut | | | | | | | | |
| 24,000 (at the cut) | 4 | 4,672 | 4,560 | 332 | 28.95 | 7.613 | | |
| 28,000 | 4 | 220 | **0** | 221 | 0.00 | 0.000 | | |
| 36,000 | 4 | 86 | 0 | 87 | 0.00 | 0.000 | | |
| 48,000 | 4 | 21 | 0 | 21 | 0.00 | 0.000 | | |
| **deroot** — 1,519 cells cut | | | | | | | | |
| 24,000 (at the cut) | 4 | 4,562 | 4,560 | 332 | 28.95 | 7.613 | | |
| 28,000 | **0** | — | — | — | — | — | | |

The whole shoot leaves the organism inside one stop:
`structural::organism_structural_tick` finds the crown detached and
`rigid::fell_severed_tissue` converts the region, so the crown stops being a
plant at all. What is left is a rootstock with no shoot, which earns nothing,
trips the `vital_cells == 0` senescence rule, and is carried out by
`rot_remains` at the species half-life — 221 → 21 cells over 24,000 frames.
That is a **graded** death rather than a disappearance, which is what the
ethos asks for.

`deroot` is fatal outright: with its root tissue gone the shoot has no
anchor (`is_structural_anchor` gives root tissue in soil, or *any* organism
cell touching a `Solid` that `anchors_organisms` — soil is `Powder`, so a
bare stem standing in it anchors nothing), so the entire plant reads
detached and is felled whole.

### 6b. The configuration where it does keep growing

`structural.rs:1323`, the detached branch:

```rust
if detached && !world.plant_load_failure && world.organism(organism_id).is_some_and(|st| !st.senescent) {
    return Vec::new();
}
```

**With COLLAPSE UNDER LOAD off, a living plant that has lost its anchorage
is never taken apart.** And nothing in the economy reads attachment:
`allocate_to_frontier`, `organism_upkeep`, the senescence rule
(`vital_cells == 0`) and the starvation rule all walk `state.cells` with no
attachment test. A cut does not split the organism either — the root cells
below the cut keep their organism id, stay in `state.cells`, keep running
`Absorb`, and keep crediting the same whole-organism `state.water` for a
crown they are no longer connected to. The carbon draw reaches across the cut
for the same reason.

So with that switch off a fully severed plant stands, drinks through roots it
is not joined to, and goes on building. That is the owner's report, and the
switch is one they use — `World::plant_load_failure`'s own doc quotes them:
*"I turned COLLAPSE UNDER LOAD off, but trees are still falling over."* The
field's doc also already calls the switch what it is: *"It is a control, not
a repair. What it masks in the lab is a real defect."*

**The defect it masks is not the one that doc names.** That doc reads the
masked defect as structural — a living plant felled whole. The measurement
here says there is a second one underneath it, and it is economic:
**severance is a structural event and not an economic one.** The plumbing is
cut; the books are not. Turn the structural response off and nothing else in
the plant notices the cut at all.

### 6c. Measured: a fully severed plant grows as if nothing happened

`arms=sever_noload`, seed 1, the same bed and the same cut as 6a, with
`plant_load_failure` set false at the moment of the cut. `fine=900` because
the coarse schedule was wider than the whole window — in 6a the crown was
gone between one 4,000-frame stop and the next.

| frame | cells | Δ | **unreached** | shoot | root | water / cap | status | uptake | income |
|---|---|---|---|---|---|---|---|---|---|
| 24,000 (at the cut) | 4,672 | −106 | 1 | 4,560 | 332 | 776.6 / 1,008 | 1.000 | 28.92 | 7.613 |
| 24,900 | 4,724 | **+52** | **4,469** | 4,470 | 336 | 777.0 / 1,016 | 1.000 | 29.25 | 7.497 |
| 25,800 | 4,876 | **+152** | 4,582 | 4,582 | 350 | 776.9 / 1,048 | 1.000 | 29.91 | 7.542 |
| 27,600 | 5,061 | +106 | 4,729 | 4,735 | 356 | 738.8 / 1,068 | 1.000 | 29.01 | 7.493 |
| 32,000 | 5,252 | +161 | 4,917 | 4,922 | 380 | 262.8 / 1,104 | 1.000 | 15.13 | 6.530 |
| 36,000 | 5,444 | +192 | 5,116 | 5,117 | 406 | 625.8 / 1,088 | 1.000 | 26.13 | 7.402 |
| 40,000 | 5,691 | +247 | 5,258 | 5,262 | 415 | 1,053.1 / 1,116 | 1.000 | 22.52 | 7.333 |
| 44,000 | 5,911 | +220 | 5,430 | 5,436 | 453 | 1,036.1 / 1,196 | 1.000 | 18.76 | 7.295 |
| 48,000 | **6,064** | +153 | **5,549** | 5,554 | 477 | 366.6 / 1,132 | 1.000 | 13.33 | 5.635 |

**Read the `unreached` column first.** It goes from 1 to 4,469 in one
organism tick and climbs from there: essentially the entire plant has no
path to an anchor. The cut is a cut, on the engine's own measure, and it
stays cut for the whole run.

Now read the rest of the row. The plant **gains 52 cells in that same
tick**, and never stops: 4,672 → 6,064 over 24,000 frames. Its water stock
holds near capacity, `uptake` runs 29.9 at its highest, and `water_status`
never leaves 1.000 — it is drinking, through root cells it is no longer
joined to, into a pool that is one number for the whole organism. Income
holds at ~7.3–7.5 against 7.6 before the cut. And its **root system keeps
growing too** (332 → 453), funded across the severance by carbon drawn from
donors on the other side of it.

For scale, the intact `control` arm on the same bed and the same seed:

| frame | control | **severed** |
|---|---|---|
| 24,000 | 4,778 | 4,672 |
| 44,000 | 5,099 | **5,911** |
| 48,000 | 4,827 | **6,064** |

One seed and four plants is not a basis for claiming the cut *helps* — the
control's late decline is ordinary crown recession and the spread between
individuals here is wide. It is more than enough to say the cut **costs
nothing measurable**.

### 6d. The control that says this is about attachment, not about a toothless economy

The obvious objection to 6c is that the economy simply never kills anything,
in which case the severance result says nothing about attachment.
`deroot_noload` is the control, and it refutes that: same bed, same seed, the
same structural switch off, but the treatment removes every root cell instead
of cutting above them.

| frame | cells | unreached | contact | water / cap | status | income |
|---|---|---|---|---|---|---|
| 24,000 (at the cut) | 4,562 | 1 | 252 | 776.6 / 1,008 | 1.000 | 7.613 |
| 24,900 | 4,457 | 4,442 | **0** | 0.0 / 4 | **0.021** | 0.199 |
| 27,600 | 4,317 | 4,315 | 0 | 0.0 / 4 | 0.007 | 0.050 |
| 32,000 | 4,148 | 4,146 | 0 | 0.0 / 4 | 0.006 | 0.037 |
| 36,000 | 2 | 2 | 0 | 0.0 / 4 | 0.000 | 0.000 |
| 40,000 | — | — | — | — | — | dead |

**The economy has teeth.** Take the water source away and the plant is
starved out in twelve thousand frames, structural switch or no. What it
cannot do is notice that the source it is drinking from is no longer
attached to the mouth.



## 7. What is actually wrong, ranked

Not built. Each names what it costs.

### 7a. The economy must read attachment — and one switch is currently making two promises

`anchor_support` runs earlier in the same organism tick and has already
written `support` on every cell. Excluding `support == u16::MAX` cells from
`donors`, from `intercepted`, and from the water credit would make a severed
crown live on its own reserves and then die — **graded**, over the carbon it
was holding, which is the ethos's first law rather than a disappearance.

The reason to do this is sharper than "correctness". Today
`World::plant_load_failure` is documented as one promise — *whether a plant
can be pulled apart* — and because the economy is attachment-blind it is
silently making a second: **whether a severed plant dies at all.** Turning
off a structural control turns off the plant's mortality with it, and there
is no setting of that switch at which a cut plant behaves sensibly: on, the
crown is felled whole (which the field's own doc already calls a defect);
off, it floats and thrives. Making the economy read attachment is what
separates the two promises, and it is the change that makes the switch safe
to expose.

Cost: one `u16` compare per cell, in passes that already walk every cell.

Risk to respect: `anchor_support` runs at `ORGANISM_TICK_INTERVAL`, and a
cell between creation and its first walk reads `support = 0` deliberately —
see `OrganismCell::default`, where the bias is toward the answer that defers
because the alternative is destroying tissue that has simply not been walked
yet. So the rule must exclude on `u16::MAX` exactly, never on a threshold.

The more correct version — a detached piece becomes its own organism, with
its own water pool and its own books — is strictly more work, and is what a
felled crown that goes on living for a while would need.

### 7b. Move the root threshold into the range plants actually occupy (this is the roots one)

§3a is the number to design against: both root-dependent terms are linear in
contact cells against a demand of ~29 — the tank at **4.0** per cell covers
it at ~7, the refill at **1.5** per wet neighbour covers it at ~20 — and a
tree grows **250-320**. The mechanism is not missing; its threshold is an
order of magnitude below where plants live, so the whole selectable range
sits above it.

Two levers, and they are not alternatives:

- **The per-cell rates are too generous.** Each threshold is
  `demand / (per-cell term)`: `demand / WATER_SCALE` for the tank and
  `demand / (Absorb rate x available)` for the refill. So moving the knee up
  into the 250-320 range means **lowering** those two — `WATER_SCALE` by
  roughly 30x, `Absorb`'s rate by roughly 10x — or raising demand
  (`TRANSPIRATION_PER_RATE`) by the same factor. Note which direction that
  is: the rates are what make roots cheap, and the instinct to "make water
  matter more" by raising them is exactly backwards. This moves terms the
  whole economy is calibrated against, so it wants the sweep 7d describes; it
  is not a one-line tune.
- **The collar drinks for free.** `Absorb` sits on `MatureBody`
  (`tree.ron:510`), so any mature cell touching damp soil is an intake.
  Moving it to mature **root** tissue only — the `reinforces_powder` test
  `organism_upkeep` already makes — keeps the mass-scaling it was widened for
  and removes the case where a shoot standing in soil drinks with no roots at
  all. Cheap, one species line, and on its own it does nothing for a plant
  that has roots.

Neither makes roots matter in a deep bed, because of 7c.

### 7c. `water_status` cannot bind in a deep bed, and that is upstream of everything

Measured twice now, independently: the shipped bed holds more water than a
stand can exhaust, so the one channel roots pay through sits at 1.000 and
the plastic root-allocation term is dead. Any repair to 7a or 7b leaves this
untouched. `plant-water-scarcity-2026-08-30.md` §2d says what does move it —
rooting **volume**, not moisture — so this is a question about worlds and
soil depth as much as about the plant model, and it wants its own
measurement before anything is tuned.

### 7d. Price the pool by path (the real fix, and the expensive one)

Do **not** delete the pooled allocation: removing it starves every root tip,
which is the documented failure it was built to fix. Price it instead —
discount a tip's share by `path_len`, or by the conductance actually
established along its supply chain, so distance and vasculature cost
something while the global solve stays.

This is a change to a weighted budget, which `CLAUDE.md` is explicit about:
reshaping what one term can express reallocates the whole sum, and the
constants calibrated against the current behaviour — `INCOME_PER_NODE`,
`cost`, `pipe_ratio`, `ROOT_BIAS_AT_FULL_WATER` — have to be re-derived as
part of the work. Budget the sweep, or the change is started rather than
scoped.

## 8. Instruments added

- **`examples/plant_reach`** — how far `organism::transport` carries carbon
  through a grown tree, with everything else in the world switched off.
  Finite-source and held-source arms off one warm-up.
- **`examples/plant_severance`** — `control` / `sever` / `deroot`, each with a
  `_noload` twin, on one bed, tracking the largest plants through the cut,
  with `unreached` as the positive control that the cut was a cut.

Both are in `Reports/instruments.md`.

## 9. What this report got wrong on the way, since it is worth recording

It was started on the hypothesis that a severed crown would go on being
funded indefinitely, straight from reading that no economy pass tests
`support`. Measured at the engine's default, that is **false** — the
structural half removes the crown inside one stop, and the plant dies
properly (§6a). The reading of the code was right and the conclusion drawn
from it was wrong, because it did not ask what *else* would act on the plant
first.

What made the difference was not a better hypothesis but two controls: the
`_noload` arms, which isolate the economy from the structural response, and
`fine=`, which was added after discovering the coarse schedule was wider than
the entire window the harness existed to observe. Both are the same lesson —
a null (or a wrong positive) that is never checked against a case where the
answer is known stays a wrong answer with a number attached to it.
