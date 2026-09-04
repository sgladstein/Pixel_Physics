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

**1. Take soil moisture off the CA sweep's wakefulness path. — BUILT, 2026-09-02.
See §8.** Predicted 2.7x from the ablation; delivered **1.69x**, and the
difference between those two numbers is itself worth reading, because it is the
gap between removing work and relocating it.
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


---

## 8. Built: moisture as its own pass — 2026-09-02

Owner: *"let's test it."* So it is built, measured and behind a switch.

`update_soil_water`'s three aux-only writes — the cell's own moisture, a
capillary neighbour's, and the cell below on drainage — now go through
`World::set_soil_moisture`, which writes quietly, marks the chunk for the
**renderer** (wet soil is a different colour, and `touched_chunks` is filled
from settledness transitions that a quiet write does not produce), and marks it
on a **second dirty channel** that only `World::step_soil_water` reads. Its
infiltration writes deliberately still go through the ordinary `set`: those
consume a `Liquid` cell outright, which is a material change and must wake the
movement sweep like any other.

`PIXEL_PHYSICS_MOISTURE=sweep` restores the old placement. Everything below is
paired against it, alternating, three runs a side.

| | tick | awake chunks | field solves/f | sweep+moisture | field | plant cells | achieved |
|---|---|---|---|---|---|---|---|
| on the sweep | 6.42 ms | 20.4 | 39.4 | 3.55 | 1.85 | 1,207 | 2.6x |
| its own pass | **3.81 ms** | **8.3** | 33.1 | 1.55 | 1.25 | **1,322** | **4.4x** |

**1.69x on the tick, and the stand came out 9.5% larger** — which is the
number that says this is a speed-up rather than a subtraction. `CLAUDE.md`'s
*a cost that vanishes may be work that vanished* is the failure to rule out
here, and `soil_water_stats` is what rules it out: the pass reports **4,216
cells visited, 3,688 of them soil, 626 changed** per tick, against the 45,442
the sweep used to walk for them.

### Why 1.69x and not the ablation's 2.7x

The ablation *removed* the work; this *relocates* it. Moisture's own cost, when
it was timed as a separate phase, was **1.22 ms** — about 300 ns per soil cell,
and essentially all of it `HashMap` probing: `update_soil_water` makes ~10
`World::get`/`set` calls per cell and `World::get` resolves a `ChunkCoord`
through the map every time. The sweep avoids this by handing each worker a
`ChunkView` with the chunk's array in hand. **That is the next step for this
phase, and it is worth roughly the same again**: a `ChunkView` over the
moisture channel, which reach-1 writes make safe under the existing
checkerboard.

**A chunk-local prefilter was tried first and made it slower** — 1.37 ms
against 1.23 — and the counter beside it says why: **3,688 of the 4,216 cells
marked are soil**, 88%, because the marked region is a patch of the soil bed.
There was nothing to reject. In `dead-ends.md`.

### Where the phase lives, and the failure that decided it

It was written as a `frame::step` phase — visible, separately timable, and the
natural home for a thing the frame orchestrates. **It is inside
`parallel::step` and `update::step` instead**, at their ends, because as a
frame phase it was invisible to every test and probe that drives the world by
calling a CA driver directly. There are **155 such call sites** in this tree;
three of them went red with nothing wrong in the code, and the rest would have
diverged silently from the game. Weather and spring sit at the top of those
same two functions for the same reason, and say so in the same words: *both
drivers, deliberately.*

The cost of that placement is that the harnesses' `ca_sweep` column now carries
moisture, so the two cannot be separated in a phase table — only across the
switch. Recorded because the 1.22 ms figure above came from the brief window
when it *was* separable.

### What the gates said

`cargo test --release --lib` **1,316 passed / 0 failed**, plus `ascii`,
`acceptance`, `worldgencheck`, `docscheck` and clippy, all clean.

Seven tests went red on the first build and **six were the placement**, fixed
by the move above. The seventh is worth recording because it is a guard
problem rather than a code one:
`lab::tests::copies_carry_what_was_planted_and_still_diverge` clones a bed into
three copies and asserts every copy still has a plant 600 frames later. Its bed
held **one organism of twelve cells** against **52 ants**, and plants do not
read `world.seed` after germination — so the copies' stands grow identically
and the only thing that differs between them is where the ants walk. The
assertion was three reseeded coin flips on whether one seedling gets eaten. It
failed on seed 1 only; seeds 2 and 3 passed on **both** sides of the change,
and the control arm passed on all three. The bed now starts from 12 founders
(8 organisms, 61 cells at the moment of cloning); not one assertion moved.

### What is left, in order

1. **A `ChunkView` for the moisture pass** — worth about another 1.0 ms of a
   3.8 ms tick, and the reasoning is above.
2. **The field's all-or-nothing early-out.** Still the largest single item at
   1.25 ms of 3.81, and still gated on *any* chunk being awake anywhere.
3. **The positional RNG**, which unlocks §5's region work.


---

## 9. Re-measured on main, 2026-09-03 — the target has moved

*Owner: "there have been lots of changes since this session was last active and
performance has gotten a lot worse." **It has, it reproduces, and it is not a
regression in any of the work above** — every phase this report optimised is
unchanged. What grew is the plant line's per-organism work, because the box got
much more fertile.*

**81 commits** landed between PR #212's merge (`9652ba6b`) and `d150e266`:
leaf clumps with shape (#224), the genome's reach and the species file as a
starting point (#223), seed dispersal (`seed_launch`), lighting the whole bench
and the free-germination fix (#222), plus the creature line's armour, severing
and curvature work.

### The default bed got *faster*, which is why this needs a heavy bed to see

Same 8-founder / 1-colony bed, 8,000 frames, that §8 was measured on:

| | tick | awake | field | plant cells | dial |
|---|---|---|---|---|---|
| `9652ba6b` (§8) | 3.81 ms | 8.3 | 1.25 | 1,322 | 4.4x |
| `d150e266` | **2.89 ms** | 8.7 | 0.93 | **2,039** | **5.8x** |

**A measurement on the default bed would have reported "no regression" and been
useless.** The bed the owner plays is not this one: `bin/lab.rs` opens
*empty* and they paint the population in, so the config that matters is a full
box.

### On a full box it reproduces, paired, two runs a side

`width=512 height=320 soil=96 founders=128 colonies=1 seed=1`, 12,000 frames,
both binaries built from the same harness source, bed echo identical on both
sides:

| | tick | worst frame | `ca_sweep` | **`active_sites`** | `field` | dial@60Hz |
|---|---|---|---|---|---|---|
| `9652ba6b` | 3.97 ms | 42–53 ms | 2.13 | **0.49** | 1.07 | 3.3x |
| `d150e266` | **5.67 ms** | **168–172 ms** | 2.17 | **2.22** | 1.15 | **2.3x** |

**Every phase is unchanged except `active_sites`, which is 4.5x.** The CA
sweep, the field and the moisture channel all hold — the work in §5 and §8 did
not come undone.

The worst frame tripling is real in the sense that it reproduced on both runs
of both arms (171.7 / 168.5 against 52.7 / 42.0), but `mean x frames` does not
pin it (653:1), so by this file's own rule it is an order statistic over many
similar frames and **must not be quoted as a number** — only as a direction to
look, which is what §9.2 does.

### 9.1 Where inside `active_sites`, and it is not the scheduler

`World::step_active_sites` is two calls. `SCHED_PASS=4000` times the first:

| at frame 12,000 | `9652ba6b` | `d150e266` |
|---|---|---|
| `scheduler::step` total | 0.35 ms | 0.66 ms |
| ...of which `organism` | 0.06 / 87 sites | 0.16 / 290 sites |
| ...of which `creature` | 0.23 / 40 | 0.40 / 61 |
| **`plant::step_organisms`** (the remainder) | **~0.14 ms** | **~1.56 ms** |

**The scheduler accounts for 0.3 ms of the 1.7 ms growth. The other 1.4 ms is
`plant::step_organisms`, which runs once per organism**, and that is ~11x.

### 9.2 Why: the box is far more fertile, and each organism costs more

Same bed, frame 12,000:

| | `9652ba6b` | `d150e266` | |
|---|---|---|---|
| live organisms | 441 | **1,174** | 2.7x |
| plant cells | 4,020 | 8,401 | 2.1x |
| standing seeds | 719 | **1,828** | 2.5x |
| leaf cells | 1,112 | **2,359** | 2.1x |
| organism slots used | 482 | **1,239** | of a 4,095 cap |
| **cost per organism** | 1.11 us | **1.89 us** | **1.7x** |
| cells per organism | 9.1 | 7.2 | *smaller* |

Two independent terms multiply: **2.7x more organisms, each 1.7x dearer.** And
the per-organism rise is not "organisms got bigger" — the average organism is
**smaller** now (7.2 cells against 9.1), so the extra cost is new per-organism
work, not more cells to walk.

**This is the plant line doing its job**, not a defect: the germination fix and
seed dispersal are what a more alive box looks like, and the owner asked for a
box that lives. What it means for the dial is that **the optimisation target
has moved.** This report's whole subject — soil moisture waking chunks, the CA
sweep's dirty regions, the field's solve gate — was correct for a box holding
441 organisms and is now the *second* problem.

### 9.3 What the next session should do, in the order the evidence supports

1. **Profile `plant::step_organisms` per organism.** It is 1.56 ms of a 5.67 ms
   tick and nothing has ever been optimised inside it. The 1.7x per-organism
   rise is unattributed — 81 commits landed and this measurement does not say
   which. A bisect over the plant commits with the heavy bed above is the
   direct route; `SCHED_PASS` already separates the scheduler out, so the
   remainder is the signal.
2. **The organism-slot ceiling is now in play.** 1,239 of 4,095 used at 12,000
   frames on a bed the owner would call medium, still climbing. Whatever the
   cap does when it binds is untested at this population.
3. **Then the two items §8 left**: a `ChunkView` over the moisture channel
   (~1.0 ms), and the field's all-or-nothing early-out (~1.15 ms, still gated
   on *any* chunk being awake anywhere).
4. **Measure on a full box, never the default one.** The default bed reports
   this regression as a 1.3x *improvement*. `founders=128 colonies=1` at 12,000
   frames is the config that shows it.

### 9.4 Still open from §8

The blind A/B asking whether the ground still *looks* right after moisture
moved off the sweep (card `20260902T013339718Z-fcfc2c`) was never opened. It is
still queued and still answerable. If the verdict says the shipped arm reads
wrong, `PIXEL_PHYSICS_MOISTURE=sweep` restores the old placement and flipping
`update::moisture_phase_enabled`'s default is a one-line change.


---

## 10. The owner's own case, reproduced — 2026-09-03

*Two details from the owner turned §9 from "the plants got dearer" into
something sharper: **"previously I could get 10-30x (with plant structural
damage turned off); now I basically max out at 4x"**, and **"the toggle doesn't
change anything right away but once a plant grows and collapses, the collapse
destroys performance."** Both are reproducible, and the second names the
regime §9 was measured outside of.*

### 10.1 The toggle used to be free, and now is not

`world.plant_load_failure` is the parameters page's `collapse_under_load`.
`lab_cost plant_load=0` sets it. Four arms, same bed
(`founders=128 colonies=1`, 12,000 frames):

| | tick | `ca_sweep` | `active_sites` | `field` | dial |
|---|---|---|---|---|---|
| `9652ba6b`, collapse **on** | 3.95 ms | 2.14 | 0.49 | 1.06 | 4.2x |
| `9652ba6b`, collapse **off** | 3.95 ms | 2.01 | 0.54 | 1.09 | 4.2x |
| `d150e266`, collapse **on** | 5.68 ms | 2.13 | **2.27** | 1.14 | 2.9x |
| `d150e266`, collapse **off** | 4.29 ms | 2.02 | **1.01** | 1.09 | 3.9x |

**Before, the switch bought nothing at all — 3.95 against 3.95.** Now it buys
**1.32x**, and it takes `active_sites` from 2.27 to 1.01. So *more than half*
of `active_sites`' new cost is load/collapse work on living tissue, and the
owner's habit of turning it off is doing real work it never had to do before.
The residue is the other half: even with collapse off, `active_sites` is 1.01
against the old 0.54.

### 10.2 The real regime: big plants, and a box that never self-limits

§9 measured `herb` at 128 founders. The owner grows **big** plants. Same
harness, `species=tree founders=16 colonies=0`, collapse on, 32,000 frames:

| frame | `9652ba6b` cells / orgs / mean / p50 / dial | `d150e266` cells / orgs / mean / p50 / dial |
|---|---|---|
| 8,000 | 6,100 / 67 / 4.01 / 3.14 / 4.2x | 19,799 / 241 / 7.02 / 3.27 / 2.4x |
| 16,000 | 3,153 / 93 / 6.41 / 5.58 / 2.6x | 25,014 / 608 / 6.47 / 2.68 / 2.6x |
| 24,000 | 3,080 / 47 / 4.65 / 2.97 / 3.6x | 25,968 / 687 / 6.66 / 2.87 / 2.5x |
| **32,000** | **2,961 / 33 / 2.57 / 2.07 / 6.5x** | **27,013 / 646 / 6.78 / 2.25 / 2.5x** |

**The brake engages far later and far higher.** On `9652ba6b` the stand peaks
at 6,100 cells and 93 organisms, then *falls* to 2,961 and 33 — and the dial
climbs back to **6.5x**, better than it started. On `d150e266` it is at 27,013
cells and 646 organisms at 32,000 frames, and the dial sits at **2.5x from
frame 8,000 onward**.

**Corrected 2026-09-03, and the correction matters.** This section first read
*"the new box never self-limits"*, on the strength of it still climbing when the
run stopped. Run to **64,000 frames** it does turn over: 27,013 cells at 32,000,
then 23,281 / 23,683 / 24,348 / **16,891** — peaking around frame 32,000 and
falling to 333 organisms. So the brake is intact; it engages **about 4x later
and at about 4.5x the population**. "Never" was an extrapolation from a run
that had not reached the turn, which is this file's own *a cascade censused
before it settles* rule arriving in a new costume: the census was taken before
the population curve had settled, and read a delay as an absence.

That is still the owner's sentence, measured — *it maxes out at 4x all the
time* — because a peak four times further out is, from the seat of someone
playing, permanent.

### 10.3 And the median frame is fine, which is why this is hard to see

The most useful column in that table is the one it would be easy to skip.

| at frame 32,000 | `9652ba6b` | `d150e266` |
|---|---|---|
| **median** frame | 2.07 ms | **2.25 ms** |
| **mean** frame | 2.57 ms | **6.78 ms** |
| mean / median | 1.24 | **3.01** |

**The typical frame barely moved — 2.07 to 2.25 ms.** What moved is the tail:
on the new build the mean is three times the median, so roughly **two-thirds of
all time spent in that window is in frames above the median**. The speed dial
reports achieved throughput, which the *mean* governs, so a box whose typical
frame is fine reads as a permanently slow box.

**This is why the collapse "destroys performance" rather than costing a hitch**,
and it is the shape the next session should chase: not a phase that got
uniformly dearer, but a heavy tail over a stand that is nine times larger and
still growing. `mean x frames` does not pin the single worst frame (347:1), so
the worst is still not a quotable number — but the **mean-to-median ratio is a
distributional fact and is**.

### 10.4 What this changes about §9

§9's framing — *2.7x more organisms, each 1.7x dearer* — is right for the
`herb` bed it was measured on and **understates the case the owner is in**. On
the big-plant bed, at the moment both boxes are compared at frame 32,000, the
organism count is **20x** (33 -> 646) and the stand **9x** (2,961 -> 27,013).
Read against each box's own peak rather than one frame, it is a **4.5x taller
population that takes 4x longer to come back down** (§10.2's correction).

### 10.5 Revised order for the next session

1. **Why does the big-plant box no longer self-limit?** `species=tree
   founders=16 colonies=0 plant_load=1`, 32,000 frames, both commits. The old
   stand peaks at 6,100 cells and falls to 2,961; the new one climbs past
   27,000. Nothing else on this list matters as much, because every other cost
   is per-organism and this is what sets the organism count.
2. **Find the tail.** Median 2.25 ms against a 6.78 ms mean: two-thirds of the
   time is in frames nobody has looked at. A per-frame histogram, or
   `labperf`'s attribution run at the frames above the median, says what they
   are doing. The owner's report says a collapse is involved.
3. **Halve `active_sites` for free**: over half of it is collapse work on
   living tissue (§10.1), which the owner already turns off by hand. Whether
   that work can be made cheaper rather than switched off is untouched.
4. Then §8's two leftovers — the moisture `ChunkView` and the field's
   all-or-nothing early-out.


---

## 11. The damage-off regime, which is the one the owner plays — 2026-09-03

*Owner: **"Solving performance with plant structural damage on would be great
but I would prefer to solve it with it off first."** Everything in §10 was
measured with it on. This is the same bed with it off, which changes the answer
and simplifies it.*

`species=tree founders=16 colonies=0 plant_load=0`, 32,000 frames, paired:

| at frame 32,000 | `9652ba6b` | `d150e266` |
|---|---|---|
| plant cells | 11,135 | **27,718** (2.5x) |
| **median** frame | 1.98 ms | **2.33 ms** (1.18x) |
| mean frame | 3.63 ms | **6.02 ms** (1.66x) |
| dial | 4.6x | **2.8x** |
| `ca_sweep` | 0.84 | 0.78 |
| **`active_sites`** | **1.77** | **4.19** (2.4x) |
| `field` | 1.02 | 1.03 |

### 11.1 With damage off, nothing got dearer — there is just more of it

Divide `active_sites` by the stand it is working on:

| | `9652ba6b` | `d150e266` |
|---|---|---|
| `active_sites` per plant cell | **0.159 us** | **0.151 us** |

**The same, to 5%.** So in the regime the owner actually plays, the per-cell
cost of the plant work did *not* regress at all. The box simply grows **2.5x
more plant**, and a per-cell cost that never changed is being charged 2.5 times
as often. That is a much better problem to have than §10's, and it is the one
to solve first because the owner asked for it first.

**Which also means §10.1's finding is confined to the damage-on arm**: the
collapse work is real and is over half of `active_sites` *when the switch is
on*. With it off there is no such term to remove, and what is left is ordinary
per-organism plant work at an unchanged unit price.

### 11.2 So the target is `active_sites`, and it was already the target

The thing worth carrying: **`active_sites` was already the largest phase on the
old build in this regime** — 1.77 ms of 3.63, 49% — and it is now 4.19 of 6.02,
**70%**. The CA sweep is 13% and the field 17%. Every optimisation this report
has landed or proposed (§5's dirty regions, §8's moisture channel, §8's
`ChunkView`, the field's early-out) addresses the other 30%.

**Nothing has ever been optimised inside `plant::step_organisms`**, and at
0.15 us per plant cell over 27,718 cells there is no exotic mechanism to find —
it is a per-cell cost paid on a large stand. That makes it the most tractable
item on this whole list: a pure optimisation, no behaviour change, no seed
sweep, no owner verdict needed.

### 11.3 The tail is still there, and it is not the phase table

Median 2.33 ms against a 6.02 ms mean — a ratio of **2.58**, against the old
build's 1.83 on the same bed. Worst frames of 100–157 ms. The phase means above
are averages over that distribution and therefore describe the tail as much as
the typical frame; a per-frame breakdown at the frames above the median is
still unmeasured, and is where the difference between "the box is bigger" and
"something occasionally does something enormous" will be settled.

### 11.4 Order of work, with the owner's preference applied

1. **Optimise `plant::step_organisms`.** 70% of the tick with damage off, at an
   unchanged 0.15 us per plant cell over a 2.5x larger stand. Pure win, no
   behaviour change.
2. **Profile the frames above the median** (§11.3). Two-thirds of the time is
   there and nobody has looked.
3. **Only then the damage-on arm** (§10.1): over half of `active_sites` is
   collapse work on living tissue when the switch is on.
4. Then §8's leftovers — the moisture `ChunkView` and the field's early-out —
   which together address about 30% of the frame.


---

## 12. `step_organisms` optimised — 2026-09-03

*§11.4 item 1, done. **The world hash at frame 32,000 is byte-identical
across every arm below**, on both settings of the collapse switch, so nothing
here is a behaviour change: no seed sweep, no owner verdict, no re-derived
constant. `lab_cost` now prints that hash, which is what makes the claim
checkable rather than asserted.*

### 12.1 The profiler was measuring the stagger, not the pass

`ORGANISM_PASS` has existed since the frame-cost audit and **it was off by
about fifty times**, in the direction that hides the problem. It printed one
sampled frame. Organisms tick on a stagger of `ORGANISM_TICK_INTERVAL`, so any
single frame holds ~1/45th of the population and *which* 1/45th is a lottery:
on the tree bed at frame 32,000 the sampled frame ticked **14 organisms
holding 14 cells between them** — one cell each, every one a seed — and
reported the whole pass at **0.08 ms** while `active_sites` averaged **4.04**.

Read that way the pass looks free and §9.1's attribution looks wrong. Averaged
over the window instead, the same build reports **3.74 ms**, which is 93% of
`active_sites` — §9.1's figure, arrived at by subtraction, was right.

Two things came out of fixing it, and the second is the one worth carrying:

- **`stress_field`, `bend_under_load` and `break_under_load` had no slots at
  all.** The instrument covered seven of the ten calls in the loop, and the
  pass that turned out to dominate was one of the three it could not see. A
  timing harness that does not cover every call in the loop reports a total
  that is not the total.
- **A profiler over a staggered schedule must average over the stagger.** This
  is `CLAUDE.md`'s *ask what your number counts when nothing is wrong* in a new
  costume — the number was arithmetically correct and answered a different
  question. The tell was there to be read: `ticked 14 / cells 14` says one cell
  per organism, which is not what a 27,000-cell stand looks like.

### 12.2 Where the 3.74 ms goes

Per frame, averaged over the 8,000 frames ending at 32,000, `species=tree
founders=16 colonies=0 seed=1`:

| pass | before | after | |
|---|---|---|---|
| **stress** (`stress_field`) | **1.301** | **0.469** | **2.8x** |
| frontier | 0.602 | 0.599 | — |
| upkeep | 0.494 | 0.469 | — |
| transport | 0.408 | 0.403 | — |
| bend | 0.328 | 0.300 | 1.09x |
| anchor | 0.313 | 0.257 | 1.22x |
| support | 0.152 | 0.152 | — |
| buds | 0.104 | 0.104 | — |
| break | 0.013 | 0.013 | — |
| roottips | 0.009 | 0.009 | — |
| **`step_organisms`** | **3.740** | **2.789** | **1.34x** |

The passes nothing touched holding still across four rebuilds is the control:
only the three that were changed moved.

The slots sum to within 0.02 ms of the whole-function clock, which settles a
question the original instrument's doc raised and could not answer — **the
per-organism cadence gate really does cost nothing.** It runs for all 646 live
organisms every frame to find the ~15 that are due, and that is not where the
money is.

### 12.3 What was actually wrong: a constant read through two hash lookups

`World::get` resolves a chunk through a `HashMap<ChunkCoord, Chunk>`, and
`World::organism_cell` adds a second hash lookup in the organism's own cell
map. So `organism_cell` is **two hashes**, and `stress_field`'s `rank` — which
is `(support, Reverse(y))` — called it once per **sort comparison** and nine
times per cell in the flow loop.

`stress_field` takes `&World` and never writes. Every one of those reads was
returning the same answer it had already returned. Hoisting `support` and
`material` into two arrays, one pass, changes no arithmetic — the values are
identical by construction — and is the whole 2.8x.

The same shape, twice more:

- **`anchor_support`** asked `cell_type(world.get(..).aux())` of the cell being
  expanded *and* of all eight neighbours, so a leaf cost nine chunk lookups per
  visit for a property the walk never changes. Hoisted to one pass. Its
  write-back loop also did `organism_cell` and then `organism_cell_mut` on the
  same cell; folded into one, **carefully** — the old `map_or(0, ..)` gave an
  unregistered cell `was = 0` and still ran the schedule test, so the fold must
  not become an `if let` around both.
- **`bend_under_load`** filtered the field by fetching `support` from the world
  per entry, one line after `stress_field` had read it. `CellStress` now
  carries it.

### 12.4 The binary search that looked free and was slower

`stress_field`, `anchor_support` and `accumulate_support` each build a
`HashMap<(i32, i32), usize>` over a list that is **already sorted by `(y, x)`**
and read it eight times per cell. Replacing it with a binary search is the
obvious move: same answer, no allocation, no hashing.

It is a pessimisation, and not a marginal one. One change at a time, same bed,
same frame:

| pass | with the map | with a binary search |
|---|---|---|
| `accumulate_support` | 0.152 | **0.292** |
| `anchor_support` | 0.316 | **0.405** |
| `stress_field` (hoists in both) | **0.472** | 0.659 |

An organism here averages ~42 cells, so the search is 5-6 dependent,
cache-missing comparisons against one hash and a probe. **The container was
never the problem** — it was reading the world through it. Landed with the
maps intact.

This is worth recording because the reasoning for the swap was sound and the
result was backwards, which is only visible if the two halves of the change are
measured separately. Bundled with the hoists it would have shipped as a win —
`stress` still fell from 1.301 to 0.659 — and left two-thirds of the available
gain on the floor.

### 12.5 Whole-frame, in the regime the owner plays

`plant_load=0`, paired and alternating, two runs a side, on a shared 4-core
box. `lab_cost` grew the `plant_load` knob in §11's own branch; the bed echo
now prints it, and the census below matches §11's stand exactly.

| at frame 32,000 | base r1 | opt r1 | base r2 | opt r2 |
|---|---|---|---|---|
| world hash | `0x0102…8f79` | **identical** | identical | identical |
| plant cells | 27,718 | 27,718 | 27,718 | 27,718 |
| **`active_sites`** | 3.602 | **2.910** | 3.574 | **2.866** |
| `ca_sweep` | 0.718 | 0.732 | 0.722 | 0.713 |
| `field` | 1.144 | 1.134 | 1.133 | 1.104 |
| **mean frame** | 5.475 | **4.786** | 5.440 | **4.693** |
| median frame | 2.373 | 2.365 | 2.358 | 2.293 |
| `us/cell` | 0.20 | **0.17** | 0.20 | **0.17** |
| dial @60Hz | 3.0x | **3.5x** | 3.1x | **3.6x** |

**`active_sites` 1.24x, the whole tick 1.15x, the dial 3.0 -> 3.5x.** The
optimised arm wins both pairs on every column that moved, and `ca_sweep` and
`field` — which nothing here touches — stay put, which is what says the
difference is the change rather than the box.

**§11.1's unit price is the number that actually moved**: `active_sites` per
plant cell was **0.159 us on the old build and 0.151 on the new**, the finding
that there was nothing to fix per-cell. It is now **0.104 us**, which is 1.5x
better than the build that was measured before the box got fertile.

Note the median barely moves (2.373 -> 2.365) while the mean falls 0.69 ms.
That is §11.3 restated: this is tail work, so a per-frame improvement shows up
in the mean and the dial and not in the typical frame. It also means **§11.3
is still the next thing to do** — the tail has been made cheaper, not
explained.

### 12.6 What is left, unchanged in order

1. **Profile the frames above the median** (§11.3). Untouched. The mean is
   still 2.0x the median after this work.
2. **The damage-on arm** (§10.1). On this bed `break` is 0.013 ms so there was
   nothing to win here; §10.1's "over half of `active_sites`" was measured on
   the 128-founder `herb` bed and still stands there.
3. **§8's leftovers** — the moisture `ChunkView` and the field's
   all-or-nothing early-out. `field` is now 1.11 ms of a 4.74 ms frame, so
   its share has *risen* to 23% simply because the plants got cheaper.
4. **Inside `step_organisms`, what is now on top**: `frontier` at 0.599 ms and
   `upkeep` at 0.469 are the two largest remaining passes and neither has been
   looked at. `stress_field` still walks `section_across` per cell through the
   world rather than through the arrays now beside it, which is the one hoist
   in this section that was identified and not taken.

### 12.7 The blind A/B from §8 and §9.4 is answered

Card `20260902T013339718Z-fcfc2c`, queued 2026-09-02 and listed as never
opened, **was answered on 2026-09-03**: *"They look the same."* The soil reads
the same with moisture off the movement sweep as on it, so PR #212's placement
stands. `PIXEL_PHYSICS_MOISTURE=sweep` stays as the escape hatch and
`update::moisture_phase_enabled`'s default does not move.


---

## 13. The tail, profiled — 2026-09-03

*§11.4 item 2. Two instruments: `TAIL=1` splits `lab_cost`'s phase table by how
dear the frame was, and `ORGANISM_SIZE=<every>` charges each organism's whole
tick to a bucket by how big the organism is. Between them they answer §11.3 and
**overturn the reason it was on the list**.*

### 13.0 First, the correction — flattening a tail cannot move the dial

§11.3 and §12.5 both name the tail as the next thing to chase, on the strength
of the mean running about twice the median. The implied prize — *make every
frame cost the median and the dial doubles* — **is arithmetically impossible**,
and it took building the instrument to see it.

The dial is `16.67 / mean`, and the mean is total work over frames **however
the work is spread**. Rescheduling a lumpy pass moves cost between frames and
leaves the total exactly where it was. A tail is worth attacking for
**hitching** — a 47 ms frame is a visible stutter — but the owner's complaint
is *"I max out at 4x"*, which is throughput, and throughput only responds to
work that is **removed**.

So the tail profile's value is not a fix. It is **targeting**: it says where
the work is, and the size curve below says whether that work can be taken away.

### 13.1 The tail is entirely `active_sites`

`species=tree founders=16 colonies=0 seed=1 plant_load=0`, the 8,000 frames
ending at 32,000, frames ranked by their own total:

| band | frames | % of all ms | mean frame | `ca_sweep` | **`active_sites`** | `field` |
|---|---|---|---|---|---|---|
| p0-50 | 4,000 | 17.7% | 1.82 | 0.53 | **0.37** | 0.92 |
| p50-90 | 3,200 | 42.7% | 5.51 | 1.00 | **3.40** | 1.09 |
| p90-99 | 720 | 32.9% | 18.84 | 1.20 | **16.46** | 1.18 |
| p99-99.9 | 79 | 6.6% | 34.23 | 0.96 | **32.07** | 1.18 |
| worst | 1 | 0.1% | 46.68 | 1.46 | **43.99** | 1.22 |

**`field` is flat across every band** — 0.92 to 1.22, a per-frame cost with no
tail in it at all. `ca_sweep` is nearly flat. `active_sites` runs **0.37 to
43.99, a 120x swing**, and is the entire tail.

Half the frames hold 17.7% of the time; the dearest 10% hold 39.6%.

### 13.2 Eleven trees are 97% of it

The size curve, same window. `ORGANISM_TICK_INTERVAL` is 45, so 8,000 frames
is 177.8 ticks per organism and **the tick column is a headcount**:

| cells | ticks | = organisms | cells total | ms total | **% of pass** | us/cell |
|---|---|---|---|---|---|---|
| 1-9 | 120,246 | **676** | 122,223 | 737 | 3.2% | 6.03 |
| 10-49 | 458 | 2.6 | 9,310 | 37 | 0.2% | 4.03 |
| 50-199 | 88 | 0.5 | 5,553 | 18 | 0.1% | 3.32 |
| 800-3199 | 1,780 | **10.0** | 3,824,570 | **17,871** | **76.9%** | 4.67 |
| 3200+ | 178 | **1.0** | 880,711 | **4,587** | **19.7%** | 5.21 |

**Eleven organisms consume 96.6% of `step_organisms`. The other 676 consume
3.2%**, and the census agrees exactly — 677 live organisms, of which all but
eleven are 1-9 cell seeds and seedlings.

That is the tail, completely explained: a frame that ticks one of the eleven
costs 10-30 ms, a frame that ticks only seeds costs 0.37, and with eleven big
trees on a 45-frame stagger about a quarter of frames carry one.

**It is not "many organisms". It is eleven trees**, and every framing in §9
and §10 that reasoned from organism *count* — "2.7x more organisms, each 1.7x
dearer" — was counting the 97% that does not matter. The population curve and
the cost curve are different questions and the seeds are only on one of them.

### 13.3 And the cost is linear, so there is no algorithmic prize

The `us/cell` column is the one that decides what to do next, and it is
**flat**: 6.03, 4.03, 3.32, 4.67, 5.21 across four orders of magnitude of
organism size, lowest in the middle. A big plant is **not** punished for being
big — it costs what its cells cost.

**Positive control**: 4.67 us/cell/tick over a 45-frame interval is 0.104
us/cell/frame, which is §12.5's independently-measured `active_sites` unit
price to three figures. Two instruments that share no code agree, so the curve
is measuring what it claims to.

So there is no super-linearity to exploit and no single dominant pass left
(§12.2 after the hoists: `frontier` 0.599, `upkeep` 0.469, `stress` 0.469,
`transport` 0.403, `bend` 0.300, `anchor` 0.257 — spread). **Total organism
work is simply proportional to standing plant cells**, and only three things
change it:

1. **Cheaper per cell.** §12 took 1.34x this way and the remaining passes have
   never been examined. Each is worth ~15% of the pass if it can be halved.
2. **Fewer full walks.** A tick makes **nine** separate whole-organism passes,
   each of which opens by rebuilding the same sorted cell list — for a
   5,000-cell tree, every 45 frames, from scratch. Sharing one list across
   them looks like a pure optimisation. **Measured at 4.9%, against the ~15%
   this paragraph first estimated — see §13.5, which is why the estimate was
   published as an estimate.**
3. **Less often, or less of it.** Ticking a 5,000-cell tree on a longer
   interval than a seedling is the only lever with a large number behind it,
   and it is **a behaviour change, not an optimisation** — the tick *is* the
   plant's economy, so a slower tree is a different plant. That is the owner's
   call, not a lane's.

### 13.4 What this does to the order of work

§11.4's list survives with its second item deleted and its reason rewritten:

1. **The remaining plant passes** — `frontier`, `upkeep`, `transport`, worth
   ~1.5 ms combined and never examined. Shared scratch (item 2 above) looked
   like the cheapest test covering all three at once; **§13.5 measured it and
   it is not worth building**, so these three need looking at individually or
   not at all.
2. **`field` at ~1.1 ms** — §8's two leftovers. **Now known to be pure
   per-frame cost**: it is 0.92-1.22 ms in *every* band, so unlike the plants
   it is not tail work and every millisecond taken off it is taken off every
   frame.
3. **The tail as hitching, if the owner cares about stutter** — worth saying
   out loud that this is a *different complaint* from the dial, and the fix
   for it (spreading a big organism's tick) buys smoothness and no throughput.
4. Damage-on (§10.1) stays last: `break` is 0.013 ms on this bed.

**And the honest ceiling.** At 5.15 ms the dial reads 3.2x. 10x needs 1.67 ms
— 68% of the frame removed — when `active_sites` is 65% of it and is now known
to be linear in a stand the owner deliberately grew. **Optimisation alone does
not reach 10x on this bed**; it reaches perhaps 5-6x, and the rest has to come
from the box holding fewer plant cells or ticking them less often, both of
which are design decisions rather than performance work.


### 13.5 The shared-scratch estimate was 3x too high — measured, not built

§13.3's second lever priced sharing one cell list across the nine
per-organism passes at "~15% of the pass", from arithmetic on sort sizes, and
said in the same sentence not to trust it. `ORGANISM_PROLOGUE` was built to
replace the estimate with a number. Tree bed, 16 founders, `plant_load=0`, the
8,000 frames ending at 32,000:

| | |
|---|---|
| collect+sorts in the window | **896,068** |
| cells sorted | **34,531,983** |
| cost | **1,132.6 ms** |
| against `step_organisms` | ~23,251 ms — **4.9%** |
| per cell, per prologue | 0.0328 us |

A *perfect* sharing therefore buys about **0.16 ms of a 5.15 ms frame, ~3%**
— and it is not a drop-in, because the cell set genuinely changes during a
tick: bending moves cells, breaking removes them, a bud flush adds them. That
is real per-pass invalidation surface for 3%, so **it is not built**, and the
entry in `dead-ends.md` records the ratio it was rejected on rather than the
idea, so a later session with 20,000-cell organisms can re-ask it.

**The general shape is worth more than the number.** A cost that is obviously
*repeated* — nine times, on every tick, over thousands of cells — reads as
obviously *large*. It is 4.9%. **Repetition is not magnitude**, and the only
thing that separates the two is measuring it. This is the same session's
binary-search result from the other direction: there, an obviously-cheaper
container was slower; here, an obviously-wasteful repetition is small. Both
were settled in one run and neither would have been settled by argument.
