# Where the evolution lab's frame goes — 2026-09-01

*Owner, 2026-09-01: "we can set a speed modifier to increase how fast time
progresses, but that gets limited based on performance... I may select x1024
and it will run at that speed, but then I add plants and creatures and it
slows down and only runs at somewhere between 4x-50x. 1) Are we still
requiring 60hz at these speed-ups and is that our main limitation? 2) How can
we improve our performance to more consistently run at these faster speeds?
We have done multiple performance reviews in the past... but these were all
for the outdoor game, we have never done one specifically for the evolution
lab."*

Correct on the last point: `frame-cost-audit-2026-08.md`,
`frame-cost-the-render-half-2026-08-29.md` and `scale_probe`'s whole record
are outdoor-world documents, and their headline — *the field is 69–86% of the
frame* — **does not hold in the lab**. This is the first review of the lab's
own frame.

**Read the counters, not the clock.** Every timing here was taken on a shared
four-core container where two byte-identical runs have disagreed 2.42x, so
each one is paired and alternated and its arms' ranges are given. The cell
counts, chunk counts and censuses are deterministic and carry no such caveat;
where the two disagree, believe the counters.

---

## 1. The 60 Hz question: no, and it is not the limitation

**We were not requiring 60 Hz.** The display rate has been decoupled from the
tick loop since Gate 3 (`lab/time.rs`), and `F` cycled it through 60 / 30 / 20
/ 10. What was true is that it **defaulted to 60 and never moved on its own**,
so unless the player found that key the box paid for sixty drawn frames a
second at every stop on the dial.

**And moving it is worth about 1.2x, not the 3x Gate 3 predicted.** Measured
end to end in the real `Lab` frame loop (`labdial mode=rate`, 8 founders and
one colony, 2,500 frames of warm-up, two seconds an arm):

| asked | 60 Hz | 30 Hz | 20 Hz | 10 Hz |
|---|---|---|---|---|
| 64x | 2.1x | 2.4x | 2.4x | 2.7x |
| 256x | 2.0x | 2.3x | 2.6x | 2.3x |
| 1024x | 2.0x | 2.3x | 2.4x | 2.4x |
| **mean** | **2.03x** | **2.33x** | **2.47x** | **2.47x** |

The arithmetic says why, and it is one line: **a tick costs 7.3 ms and a drawn
frame costs 4.7 ms.** At 60 Hz the draw is 28% of the wall clock and at 10 Hz
it is 4.5%, so the whole ladder can only ever return that 28%. Gate 3's
"roughly triples" was written before either number was measured in this bed.

**What shipped anyway**, because it is free and it is what the owner asked
for: the display rate now **follows the dial** (`time::AUTO_DISPLAY`) — 60 Hz
to 4x, 30 Hz to 16x, 20 Hz to 64x, 10 Hz above — and `F` now sets the
**minimum framerate** floor under that ladder rather than the rate itself. A
floor of 60 reproduces the pre-2026-09-01 lab exactly, at every stop.

One thing the ladder's first rung leaves on the table, recorded so it is not
re-derived: `MOTION_TICKS_PER_FRAME` is 12, so motion still reads at **12x**
on a 60 Hz display, not 4x. The `(4, 60)` rung is the owner's own number and
moving it up costs nothing visible.

---

## 2. Where the tick actually goes

`lab_cost phases=1`, the shipped bed (512x320, 8 founders, one colony), the
mean over the last 8,000 frames:

| phase | ms | share |
|---|---|---|
| `ca_sweep` | 3.59 | 56% |
| `field` | 1.78 | 28% |
| `active_sites` | 0.63 | 10% |
| `pheromones` | 0.39 | 6% |
| everything else | 0.002 | 0% |
| **tick** | **6.39** | |
| the draw, dirty-rect | 4.70 | |

`liquid_bodies`, `chunk_bodies`, `player` and `particles` together are **two
microseconds** — the design guide's *"the lab's speed comes from what is not
in the box"* is confirmed and is not where anything is left to win.

**And none of the cost is the plants' own code.** Split by what is living in
the bed, same harness, 8,000 frames:

| bed | tick | awake chunks | field solves/f | `ca_sweep` | `field` | `active_sites` |
|---|---|---|---|---|---|---|
| empty box | **0.006 ms** | 0.0 | 0.2 | 0.003 | 0.003 | 0.000 |
| one ant colony | 0.73 ms | 0.8 | 4.9 | 0.07 | 0.34 | 0.06 |
| eight plants | **7.03 ms** | 25.0 | 39.3 | 4.67 | 2.08 | **0.28** |
| both | 7.65 ms | 20.4 | 39.4 | 4.36 | 2.14 | 0.71 |

Eight plants take the box from 0.006 ms to 7.03 ms — a factor of **1,170** —
and the phase that *is* the plants costs 0.28 ms of it. The other 6.7 ms is
the world reacting to them.

---

## 3. What the plants are doing to the sweep

`examples/labperf`, built for this review, reads three things off the world
before each tick — what the sweep is *asked* for, what it *finds*, and which
phase wrote the cells that asked for it. At frame 8,000, over 150 ticks:

| arm | awake | reach | cells swept | cells changed | waste |
|---|---|---|---|---|---|
| plants | 27.1 of 40 | 14.1 | **45,442** | **447.2** | **102x** |
| both | 24.0 of 40 | 13.5 | 38,281 | 318.6 | 120x |

**The sweep walks a hundred cells for every one that moves.** And the churn
census says what is moving:

```
cells/tick by material: soil 410.5, empty 21.7, water 10.1, log 2.7, seed 1.5, wood 0.3
4,570 distinct cells changed at all, 59 on half the ticks or more, 5 on every tick
```

**92% of everything that changes in a box the player would call settled is
soil moisture.** It is not an oscillation — 4,570 different cells over 150
ticks, only 5 of them moving every tick — it is a wetting field being
continuously redistributed. Roots drink, the gradient opens,
`update::update_soil_water` closes it, and every close is a `World::set`.

The attribution table names the writer outright: `ca_sweep` writes 440.3
cells a tick across **20.8 distinct chunks**, `active_sites` writes 7.2
across 3.5, and the field, the pheromones, the particles and the bodies write
**zero**. The sweep is keeping itself awake.

**A dirty chunk buys two phases, not one**, and this is the part that turns a
56% phase into an 84% one. `field::step` skips its whole five-pass solve only
when the field has converged **and `active_chunk_count()` is zero** — so
every soil-moisture write pays for the field as well as for the sweep.

### The ablation that sizes it

`PIXEL_PHYSICS_SOIL_WATER=off` stops infiltration, capillary exchange and
drainage. **It is a control, not a proposal**: with it off the roots deplete
soil nothing refills, so the stand is a different stand and its census is
printed for exactly that reason. Two runs a side, alternated:

| | awake | solves/f | `ca_sweep` | `field` | tick | achieved | plant cells |
|---|---|---|---|---|---|---|---|
| shipped | 20.4 | 39.4 | 3.59 | 1.78 | **6.39 ms** | 2.6x | 1,207 |
| moisture off | **4.8** | 28.4 | **0.31** | **1.07** | **2.39 ms** | **6.9x** | 1,513 |

**Soil moisture transport is 63% of the lab's tick**, and it is not cheap
because the stand died — the ablated arm's stand is 25% *larger*.

---

## 4. What to do about it, in the order the evidence supports

**1. Take soil moisture off the CA sweep's wakefulness path.** Worth 2.7x by
the ablation above, and it is the only item on this list in that class.
Moisture transport is not a *movement*: a wetness change needs its own cell
and the four it exchanges with reconsidered, and nothing else — not the
`reach`-wide movement neighbourhood, and not a chunk kept awake for the CA
sweep and the field to walk. **The engine already has the right home for it**:
`evaporation::schedule_damp_soil` is called from inside `update_soil_water`
and puts damp soil on the active-site schedule.

Two things such a change needs, both found while measuring and neither
obvious from outside:

- **A quiet write that the renderer still sees.** `Chunk::set_world_quiet`
  writes without dirtying, but `World::touched_chunks` — the renderer's set —
  is filled from settledness transitions around `end_sweep`, so a quiet write
  is invisible to the draw as well as to the sweep. Wet soil is a different
  colour; the render touch has to be re-added explicitly.
- **The moisture work itself is small.** 410 cell updates a tick is nothing;
  what costs 3.3 ms is the sweep *around* them. Expect the fix to move the
  work rather than remove it, and to land near the ablation's 2.4 ms rather
  than at it.

**2. The field's early-out is all-or-nothing.** After (1) the field is the
biggest item left, at 45% of a 2.4 ms tick, and it still solves 28 tiles a
frame with only 4.8 chunks awake — because *any* awake chunk anywhere costs
the entire five-pass solve over the whole box. A lab bed always has something
growing somewhere, so the field runs every frame for ever. Gating the solve
on locality rather than on a global flag is the second lever; it is a
behaviour change and wants its own measurement.

**3. Narrower dirty regions — measured, and parked. See §5.**

**4. Parallelism is thinner than it looks, and this one is diagnosed rather
than fixed.** Four threads against one, three runs a side: **6.37 / 6.46 /
6.34 ms against 9.01 / 8.98 / 9.33** — 1.42x on four cores.
`parallel::step` cuts the sweep into one rayon dispatch per (chunk row, `cx`
parity), which in a five-chunk-tall box is **9.5 dispatches a tick for at most
four chunks each**. That shape is load-bearing (it is what keeps chunk rows
sweeping bottom to top), so this is not a "just use one pass" finding — but at
lab scale the box is short enough that the joins are a real share, and it does
not appear at outdoor width.

---

## 5. Per-row dirty spans: 1.19x, and why it is off by default

`Chunk::dirty` is a **bounding box**, so two writes at opposite corners of a
64x64 chunk dirty everything between them — which is exactly the shape 410
scattered soil writes produce. Replacing it with **one x-span per row** is
strictly tighter and still a superset of every cell the current rule can act
on. Shipped, behind `PIXEL_PHYSICS_SWEEP=rows`, **off by default**.

Estimated first, from the changed cells themselves, before it was built
(`labperf`'s `est_` columns; `est_bbox` reproduces today's rule from the same
diff and is the control on the estimate):

| rule | cells asked for | against today's |
|---|---|---|
| bounding box per chunk (`est_bbox`) | 28,612 | — |
| **one span per row** (`est_rows`) | 9,629 | **3.0x less** |
| box, horizontal expansion cut to 1 | 22,523 | 1.3x less |
| spans, expansion cut to 1 | 3,683 | 7.8x less |

Built and measured paired, three runs a side, ranges not overlapping:
**`ca_sweep` 3.67 → 2.67 ms (−27%), whole tick 6.51 → 5.49 ms (−16%,
1.19x).** Well short of the 3.0x the cell count predicted, which is
`CLAUDE.md`'s *removing work is not the same as removing cost* again: what is
left is per-chunk fixed cost and the cells that genuinely act.

**Why it is not the default, and this is the transferable half.** **The CA
sweep's random draws are consumed per *visited* cell, not per cell that
acts** — `update_liquid` and `update_powder` each open with
`surface.rng().flip()` on every visit, whether or not anything moves. So
*any* narrowing of the swept region, however provably it only drops cells no
rule could have acted on, shifts the per-chunk RNG stream and with it every
pile, front and stand downstream. Two guards go red on it, both from the
stream shift rather than from a lost cell:

- `frame_step_matches_the_sequence_app_update_ran_before_extraction`, which
  holds a hash taken on `origin/main`;
- `a_determinate_species_terminates_its_axes_in_organs_and_an_indeterminate_
  one_does_not`, which asserts an organ is *standing* at one instant 30,000
  frames in — `CLAUDE.md`'s *assert the property, not two instants* in the
  wild, and a phase shift moves it.

**1.19x does not buy a change to how every pile in the world lands**, so it
ships as the instrument that measured it. **What would unlock it is not more
tuning of the region: it is seeding the per-cell draw from position and frame
instead of from a per-chunk stream.** After that, sweep-region work becomes
free — worth up to 3x of this phase by the table above — and so does every
other narrowing anyone proposes later. Recorded in `dead-ends.md` with that
condition attached.

---

## 6. What this changes about the received picture

- **"The field is 69–86% of the frame"** is an outdoor result. In the lab the
  field is 28% and the CA sweep is 56%.
- **"Cost follows living biomass"** (design guide, Gate 3) was already
  overturned by round two's +0.03 correlation. This says what it follows
  instead, mechanically rather than correlationally: **soil moisture writes →
  dirty chunks → the sweep and the field, both.**
- **"The draw dominates the tick five to one"** (round two: 4.78 ms against
  0.94 ms) was measured on a young bed. On a grown one the tick is 6.4 ms
  against a 4.7 ms draw and the ratio has reversed. **Which half dominates is
  a function of how full the box is**, so neither figure should be quoted
  without the stand's cell count beside it.
- **Gate 3's "the tick multiplier roughly triples" at 20 Hz** is 1.2x
  measured. The display rate is not where the dial's ceiling is.

## 7. Reproducing all of it

```
cargo run --release --example labperf                                  # the four arms, counters
cargo run --release --example labperf -- settle=8000 probe=150 map=1   # ...with the awake-chunk map
cargo run --release --example lab_cost -- frames=8000 every=8000 phases=1 render_every=0
PIXEL_PHYSICS_SOIL_WATER=off cargo run --release --example lab_cost -- frames=8000 every=8000 phases=1
PIXEL_PHYSICS_SWEEP=rows     cargo run --release --example lab_cost -- frames=8000 every=8000 phases=1
cargo run --release --example labdial -- mode=rate seconds=2 warm=2500
```

Pin `RAYON_NUM_THREADS` for anything whose counter you intend to compare —
`CLAUDE.md`'s note on counters downstream of the checkerboard — and alternate
the arms rather than running one after the other.
