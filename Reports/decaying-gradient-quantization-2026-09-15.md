# Does the pheromone line's root cause generalise?

**Survey, 2026-09-15. `engine`.** Asks one question of the whole engine:
the defect found on the pheromone plane — a scalar that both **decays** and
is read as a **gradient**, in storage too narrow for both — how many more
of these are there?

**Answer: one more, and it is already half-defended.** `Cell::temperature`
is the third instance. Every other per-cell scalar in the engine is either
already `f32`, or not a gradient, or not decaying, or its flatness is set
by a **designed threshold one to two orders of magnitude above its storage
quantum**, where widening cannot move a single decision.

So the rule stands at **three instances, with two different remedies** —
and the survey turned up a sharper thing than a third bug: **the two
remedies this engine already uses both defend the decay half and are
structurally incapable of defending the gradient half.** A channel carrying
one of them looks protected and is not. That is §4, and it is the part
worth keeping.

---

## The shape, stated so it is greppable

A scalar that **decays** and is read as a **gradient** needs range for both
and usually gets it for one.

- **Decay** needs headroom *underneath* it. Without it the value hits a
  fixed point: `0.267 * 0.5` rounds back to `0.267` and stays there for
  ever.
- **A gradient** needs resolution *between neighbours*. Without it the
  difference two readers are separated by rounds to zero and the channel
  reads flat.

Narrow storage takes the **gradient** away first, and silently — the code
stays correct, the stored values stay healthy, and every gate stays green.
The two known instances before this survey:

| | instance | storage | how it presented |
|---|---|---|---|
| 1 | canopy density | 4 bits of `Cell::aux` | decay could not release space; `CANOPY_DENSITY_DECAY_PER_TICK` had to be raised to 0.5 to work at all — the tail wagging the dog (`dead-ends.md`:657, :727) |
| 2 | pheromone `Scent` | `u8` | trail flat past its own midpoint: the ant's own along-reading was **+0.000** at 0.7 and 0.9 of the way out while the plane peaked at a healthy 39–98 of 255 (`pheromone-lifetime-and-wiring-2026-09-14.md` §3c) |

## The method, which is the part that finds it

**Do not measure the stored value; measure the number its consumer
computes.** The pheromone plane's own numbers looked healthy the whole way
through — peak 39–98 of 255, hundreds of cells standing — while the value
the ant computed from them was `+0.000` past the midpoint. Same data, two
readers, one of them silent.

**Exactly zero is the signature.** A weak-but-working mechanism reads
0.003; an exhausted representation reads 0.000, at several sample points,
and keeps reading it.

**But there are two causes of an exact zero, and only one is this defect** —
a *degenerate condition* (the rule keyed on something that cannot vary) and
an *exhausted representation*. Rule out the degenerate one first; it is
cheaper.

### The cheap discriminator this survey adds

Print the **consumer's own decision threshold next to the storage quantum**.

> **Where a designed threshold sits far above the quantum, the
> representation cannot be the binding constraint, and there is nothing to
> fix however flat the channel reads.**

It is a ratio, it costs one grep, and it killed two of the four live
candidates outright before any world was built. It is also what makes the
pheromone case legible in hindsight: **the ant's reader had no threshold at
all.** It read the raw difference, so the quantum *was* the decision
boundary. That is the condition to look for — a consumer with no threshold,
or one at the quantum.

---

## 1. The survey

Re-derived from `Cell`'s own definition and `field.rs`, not taken from a
list. `Cell` is a 12-byte tagged union: `material`, `shade`, `flags`,
`temperature: i16`, `burn_timer: u16`, `aux: u16`, `organism_id`.

| scalar | storage | decays? | read as a gradient? | consumer-side number | verdict |
|---|---|---|---|---|---|
| **`Cell::temperature`** | `i16`, quantum **1 °C** | yes, toward the neighbour average | **yes** — `fire::diffuse_heat`, and it has **no threshold**: it reads the raw difference | **0 of 24 cells warmed** over 500 frames beside a +4 °C block | **THIRD INSTANCE — half-defended.** §2 |
| soil water (`Cell::aux`) | `u16`, 0..1000 | yes (evaporation, drainage, uptake) | yes — `organism::moisture_pull` | median **0.6250**, p90 **1.0000** over 281 live root tips | **null** — threshold is **50x** the quantum. §3 |
| liquid fill (`Cell::aux`) | `u16`, 0..1000 | yes (evaporation) | yes — levelling reads the neighbour fill | — | **null** — `min_transfer` is **16x** the quantum. §3 |
| anchor distance (`Cell::aux`) | `u16` | **no** — recomputed exactly, never decayed | yes (`n.aux() < own`) | — | **shape does not apply**: no decay, and neighbours differ by exactly `step` |
| `Cell::burn_timer` | `u16` | counts down | **no** — never differenced | — | not a gradient |
| `field.*` — light, moisture, temperature, pressure, `vx`, `vy` | **`f32`** | yes | yes | — | **safe by width. Confirmed rather than assumed** |
| `carbon_conductance` | `[f32; 4]` | yes (`VEIN_DECAY`) | transport | — | safe by width |
| brain genome, weights, activations | `f32` throughout | — | — | — | safe by width |
| `organism_id`, `heading`, `shade`, `flags`, `generation` | `u8`/`u16` | no | no | — | ids, flags and counters — the shape does not apply |

The narrow-storage surface the brief pointed at is real but is mostly
**ids, flags and counters**. `Cell::aux`'s many tenants are a tagged union,
and only two of them are decaying gradients; both are on a 0..1000 scale
with a consumer threshold far above the quantum.

---

## 2. `Cell::temperature` — the third instance, and what defends it

`diffuse_heat` pulls a cell toward its four-neighbour average by
`material.heat_conductivity`, and stores the result back into an `i16` of
whole degrees. A typical conductivity is **0.1**, so a 4 °C difference is a
0.4 °C pull — which rounds straight back to where it started.

Measured by `examples/quantgrad`, **the naive arm against the shipped one,
differing by exactly one clause**:

```
  the dead zone: the largest neighbour difference that still moves NOTHING
  conductivity  naive dead zone  shipped dead zone
          0.02           24.5C              0.5C
          0.10            4.5C              0.0C
          0.25            1.5C              0.0C

  a 4 C gradient at conductivity 0.10, 200 steps:
    naive (no nudge)    24C -> 24C   (moved +0C)
    shipped (nudge)     24C -> 21C   (moved -3C)
```

**The engine already found this and already fixed it** — in
`diffuse_heat`'s own source comment, which names it *"a genuine numerical
fixed point, not just slow convergence"* and forces one degree of movement
whenever the raw pull is real but rounds away. That is a **second remedy**,
distinct from widening the type, and it works: the dead zone collapses from
4.5 °C to 0.

### 2a. What the nudge cannot reach, and why that is not a restatement

The nudge is gated on `!already_settled`, and `already_settled` is measured
against **`AMBIENT_TEMPERATURE`** — not against the cell's own local
equilibrium. So it is switched off exactly where a cell sits at ambient,
**which is the receiving end of every shallow gradient.**

It rescues a hot cell cooling *down* to ambient. It is structurally
incapable of rescuing an at-ambient cell warming *up*:

```
  a cell AT ambient, warm neighbour, 500 steps
   neighbour    cond           warmed by
         21C    0.02                  0C
         24C    0.02                  0C
         24C    0.10                  0C
         30C    0.10                 10C

  real engine, 24 ash cells at EXACT ambient beside a +4C block, 500 frames
  of parallel::step:
    cells that warmed at all: 0 of 24
```

**Exactly zero, at several sample points, and it keeps reading it** — the
signature, reproduced in the sweep and not only in the arithmetic.

### 2b. The second arm, which is what makes this a finding and not a hypothesis

`CLAUDE.md`'s positive control: the same rule and the same gradient at a
finer quantum, changing **nothing else**.

```
A cell at exact ambient with a warm neighbour; degrees it warms by, 500 steps.

 neighbour   cond | 1/degree (shipped)   10/degree   100/degree
       21C   0.10 |              0.00C       0.60C        0.96C
       22C   0.10 |              0.00C       2.00C        2.00C
       24C   0.02 |              0.00C       4.00C        4.00C
```

**The flatness is representational, not physical.** At 24 °C and
conductivity 0.02 the shipped width reads `0.00` and a tenth-degree width
reads the *entire* 4.00 °C — not a partial improvement, the whole signal
was being discarded.

### 2c. …and it is currently unreachable, which is why it is registered and not fixed

The gap binds only on a **shallow** cell-level heat source — under about
5 °C above ambient at conductivity 0.1. There is not one in the engine:

- every `burn_temperature` in `assets/materials/` is **320–900 °C**;
- the only `intrinsic_temperature` is **lava, at 1000 °C**;
- a corpse inherits its creature's temperature, which is ambient exactly;
- beside a 500 °C fire the neighbour average is ~140 °C, a 120 °C
  difference — rounding never kills it and the gate never binds.

So: real, reproducible, correctly diagnosed, **and nothing in the shipped
engine can reach it today.** Filed as **§Z27** with the condition that
reopens it.

**Fixing it would also be a bad trade right now, in two ways this survey is
obliged to state rather than skip.** The gate is not an oversight — its own
comment records the failure it exists to prevent: without it *"a connected
mass of many cooling cells"* nudges itself awake for ever and **no chunk
ever sleeps**, which is `CLAUDE.md`'s hard frame-cost constraint, caught
originally by a test with 40 connected ash cells. And re-scaling the channel
to tenth-degrees would touch **every temperature constant in the engine** —
`AMBIENT_TEMPERATURE`, every material's `burn_temperature` and phase-change
points, `HEAT_GLOW_RANGE`, weather's cold values, explosion glow — which is
exactly the *"name the constants calibrated against the current behaviour,
and if re-deriving them is unaffordable the change is not scoped, merely
started"* rule. (Range is not the obstacle: lava at 1000 °C is 10,000
tenth-degree quanta, well inside `i16`.)

---

## 3. The two nulls, and why they are nulls rather than unmeasured

Both are decaying gradients in `u16`. Both are dead, and the discriminator
says so before any world is built.

**Soil water.** `moisture_pull` differences `aux / 1000.0` across ±4 cells
and a `RootTip` steers on it when it clears `MIZ_THRESHOLD = 0.05` — **50
quanta.** For the storage to bind, a gradient would have to be under one
quantum while its true value is over fifty. Measured on 281 live root tips
(`plant_probe frames=6000 trees=16`):

```
  gradient magnitude  median 0.6250  p75 0.8740  p90 1.0000  max 1.4142
  clearing MIZ_THRESHOLD (0.05): 261 of 281 tips (92%)
```

Median **0.6250** is **625 quanta**. The channel is using most of its range,
not running out of it. Arm (a) fails outright, so there is nothing to
confirm.

Note also that where this channel *does* go flat, the cause is named and
deliberate: capillary redistribution stops at a **rest threshold of 60
units**, sixty times the quantum, to stop a two-cell pump churning at every
water-table boundary. That is `CLAUDE.md`'s degenerate condition, ruled out
first exactly as the method says.

**Liquid fill.** Levelling refuses any transfer below `Material::
min_transfer`, **16 units** — sixteen times the quantum, and `dead-ends.md`
records it being tuned from 150 down to 16 on levelling grounds with no
mention of resolution. The threshold binds 16x before the quantum can.

---

## 4. What this survey actually changes

**Both remedies this engine uses defend the decay half and neither defends
the gradient half**, and that is a general fact about the two, not an
accident of either:

| remedy | where | rescues | cannot rescue |
|---|---|---|---|
| forced strict-decrease decay LUT | pheromone plane (`dead-ends.md`:1196) | decay reaching zero instead of fix-pointing above it | any **difference between two neighbours** — it constrains each cell against *itself*, one pass to the next |
| minimum-progress nudge | `fire::diffuse_heat` | a cell converging to its local equilibrium | the same, and here it is **gated off at ambient**, i.e. at the receiving end of every gradient |

Both are per-cell monotonicity guarantees. **A gradient is not a property of
a cell**, so neither can see one. A channel carrying either looks defended
and is only half defended — which is precisely how the pheromone plane
passed for as long as it did: it *had* the LUT, the LUT worked, and the
trail was flat past its midpoint anyway.

**The check that does generalise** is the discriminator in the method
section: put the consumer's decision threshold next to the storage quantum,
and treat *a consumer with no threshold* as the risk condition. Both
confirmed instances have one; both nulls are thresholds 16x and 50x the
quantum.

## 5. What would overturn this

- **§2c's unreachability is a census of today's heat sources, not a
  theorem.** Any cell-level source within a few degrees of ambient — body
  heat, solar warming of surfaces, geothermal, a warm spring — makes §Z27
  live the day it lands, with no other change. The creature and weather
  lines are both plausible authors of exactly that.
- **§3's soil-water null is 281 root tips in one bed.** The margin is three
  orders of magnitude (625 quanta against a 1-quantum floor), so it is not
  close, but a bed whose gradients are genuinely near the threshold would
  need re-measuring rather than re-reasoning.
- **The survey is over `Cell` and `field.rs` as they stand.** `dead-ends.md`
  :1056 records the standing rule that the next per-cell scalar goes in the
  **sidecar** rather than into repacked `aux` bits — every flag bit is taken
  and `Cell` is asserted 12 bytes. A future channel that ignores that, and
  packs a decaying gradient into a few bits, is instance four; the table in
  §1 is the form to re-run over it.
