# What one ant costs, and why halving it is not a tuning problem

*2026-09-13, round 32 lane A. The owner's standing first priority, in his own
words: "The biggest issue is the performance after the creature numbers get
high and that is our #1 priority by far."*

`evolution-lab-playtest-2026-09-13.md` §1 sized that from a 560,000-frame played session and said outright what it
could not do — **localise the cost inside the creature pass**, because its
clock was the owner's own and adjacent samples at equal ant count differ
12–14x. **That report is in flight as PR #382 and is not in `Reports/` yet** —
read it from `claude/evolution-lab-round-31-orudq8`, which is why it is named
here rather than linked. This report is the headless replay it asked for, on a quiet box with
`RAYON_NUM_THREADS` pinned and arms compared inside one run.

**The three findings, before the evidence.**

1. **The played session's cost model reproduces**, and the quantity that
   transfers between machines is the *share*, not the microseconds: at 3,000
   ants the creatures are **86% of the frame**, which §1 measured
   independently from the other side.
2. **The per-ant cost is real, it is inside `creature::tick`, and it is
   diffuse.** One ant decision costs **57,314 instructions**, split five ways
   with no term over 31%. Two hypotheses that would have given a single big
   lever were tested and refuted by measurement rather than by argument.
3. **The frame's dominant term has no parallelism.** The CA sweep runs on
   every core; the creature pass is serial. At the population the owner plays
   at, that means ~86% of the frame is single-threaded, and it is worth more
   than every micro-optimisation in §4 put together.

Everything below was measured on one cloud container, four cores,
`RAYON_NUM_THREADS` pinned, arms round-robin inside one process and read at
the minimum over three reps. The bed is the played one: 512x512, `herb`
founders, `longant` colonies, seed 1, `PLANT_LOAD_FAILURE false`, and 20,000
frames of growth before any ant arrives so the intercept is measured over a
bed with 454 plants in it rather than over eight seedlings. The harness is
[`examples/antcost.rs`](../examples/antcost.rs), new in this round and listed
in [`instruments.md`](instruments.md).

## 1. The played session's model reproduces, up to one scalar

| ants | µs/tick | creature ticks/frame | moves/frame | blocked % |
|---|---|---|---|---|
| 0 | 2,550 | 0.0 | 0.0 | — |
| 100 | 2,982 | 19.8 | 7.3 | 3.8 |
| 196 | 3,515 | 38.9 | 16.5 | 5.9 |
| 254 | 3,820 | 50.2 | 23.3 | 7.9 |

**cost ≈ 2,518 µs/tick + 5.14 µs per ant per tick**, residuals **+32 −48 −14
−7** on a ~3,000 µs frame. Linear, and messy enough at the ends to be a fit
rather than an artifact.

Against §1's **1,000 µs + 2.10 µs**: the intercept is **2.52x** and the slope
is **2.45x**. **The two ratios agree, and that is the whole reading.** What
differs between this container and the owner's machine is one scalar — it is
uniformly slower per unit of work, background and creatures alike — rather
than the creature pass doing something different here. So the microseconds do
not transfer and **the share does**:

> at 3,000 ants, `3000 x 5.14 / (2518 + 3000 x 5.14)` = **86.0% of the frame**,
> against the **86%** §1 computed from the owner's own clock at the far end of
> a range this harness never reached.

Two independent measurements, on two machines, from opposite ends of the
population range, agreeing on the number that matters. **That is the
reproduction, and the µs/ant figure is not.**

### 1a. What the number's own noise is, stated before anything is built on it

Four runs of this harness on this box, same bed, same binary, report slopes of
**4.31, 5.05, 5.14 and 5.98 µs/ant/tick** — a **1.39x** spread on the headline
quantity, and **1.18x** between two runs of byte-identical parameters. The
intercept is steadier (2,452–2,744 across the same runs, 1.12x).

This is not a complaint about the box, it is the **budget for §4**: an
optimisation worth less than about 20% of the per-ant term **cannot be
measured on this wall clock**, however many reps it is given, and claiming one
would be inventing a result. Where §4 lands a change, the gate is the
instruction count, which is exactly reproducible.

## 2. Two explanations that would each have given one big lever, and neither survives

**The ants are not jammed.** A refused step costs *more* than a taken one —
`step_chain` prices its alternatives before it gives up, and `tumble` then
evaluates all eight headings — so a bed of gridlocked ants would explain a high
per-ant cost outright. `moves/f` and `blk%` above are the effect counter from
the far side of the call, and the blocked fraction runs **3.8–7.9%** against
`creature_scale mode=walk`'s standing `Chain(2)` control at **5.2%**. The ants
are walking.

**It is not what the ants leave dirty behind them, either.** A callgrind
profile put `world::visit_soil_water` at **37.8% of every instruction in the
run** — more than any other single function by a factor of two and a half —
which reads as *ants dirty soil chunks, the moisture pass walks the row hull of
every mark, the hull is the whole chunk*. The mechanism is real and is written
up in `chunk::moisture_marks_cells`' own doc. It is not what is on the clock
here, and one switch settles it: `PIXEL_PHYSICS_MOISTURE=sweep`
(`update::moisture_phase_enabled`, a control and never a setting) puts the pass
back in its pre-2026-09-02 home inside the CA sweep.

| | intercept µs/tick | slope µs/ant/tick |
|---|---|---|
| moisture as its own pass (shipped) | 2,709 | **5.045** |
| moisture back inside the sweep | 4,985 | **5.060** |

**The switch moves the intercept 1.84x and the slope by 0.3%** — inside a
run-to-run spread of 18%. Two more columns say the same thing from the other
direction: soil-water visits per frame are **highest at zero ants** (13,591 at
0, 7,804 at 148, 10,315 at 254 — the ant arms have eaten some of the bed), and
awake chunks are flat in ant count (**18.1 → 20.9 → 19.3**). The moisture pass,
the field and the sweep are the **intercept**. The playtest report's refutation
of the cell sweep stands, and this is a second, independent route to it.

The callgrind figure was not wrong; it was answering a different question,
because that profile's bed had **8 plants in it** and the ants were the only
things marking soil at all. `CLAUDE.md`'s worst-recurring failure, met head on:
a number that is arithmetically correct and about the wrong thing looks exactly
like a result.

## 3. Where the 57,314 instructions of one ant's decision actually go

Callgrind, release build, **49,568 creature ticks** of the played bed's own
species. Instruction counts rather than a clock, deliberately: this box's
wall-clock spread is 1.39x on the quantity under test and Ir is bit-exact.

**`creature::tick` costs 57,314 Ir per ant decision**, and at the long ant's
`tick_interval` of 6 that is one decision per ant per six frames.

| inside `creature::tick` | Ir/tick | share | calls/tick |
|---|---|---|---|
| `sense` | 17,423 | 30.4% | 1.00 |
| `step_chain` | 9,486 | 16.6% | 0.38 |
| `brain::eval_brain` | 8,077 | 14.1% | 1.00 |
| `tumble` | 7,358 | 12.8% | 0.31 |
| `act` | 6,540 | 11.4% | 1.00 |
| `reconcile_chain` | 1,662 | 2.9% | 1.00 |
| `try_bud` | 1,272 | 2.2% | 1.00 |
| `CreatureDef::clone` + its drop | 1,179 | 2.1% | 1.00 |
| `apply_creature_energy` | 851 | 1.5% | 1.00 |
| `organism_tick_interval` | 726 | 1.3% | 1.00 |
| `organism_sight_range` | 703 | 1.2% | 1.00 |
| everything else, and self | ~1,900 | 3.3% | |

**No term is over 31%, and that is the finding.** There is no single lever that
halves this. Splitting the largest one further does not change that:

| inside `sense` | Ir/tick |
|---|---|
| `adjacent_food_counted` | 5,377 |
| `moisture_gradient` (4 bilinear field samples) | 3,141 |
| self — mostly the 5x5 crowding scan | 2,496 |
| `field::sample_bilinear`, 4 more | 2,623 |
| `surface_curvature` | 1,582 |
| `adjacent_nest` | 756 |
| `organism_sight_range` | 703 |

**One duplicate is worth naming on its own.** `adjacent_food_counted` runs
**twice** per tick — once in `sense`, for `BrainInput::FoodAdjacent` and
`KinNeed`, and again in `act` when the feed verb fires — walking the same
deduplicated body ring both times, at **51 `World::get` calls and 22
`is_living_kin` calls** an invocation. Together that is **10,739 Ir, 18.7% of
the tick**, and about half of it is the same answer computed twice. It is
*not* safely cacheable as things stand: `act` runs its dig and attack verbs
before the feed verb, so the grid and the organisms' energies can both have
moved between the two calls. Making it safe wants a world-write generation
counter to validate the cached scan against, which is a change to the write
path and is priced in §5 rather than taken here.

**And a cost that is under everything above**: `World::get` costs **58
instructions a call** — `in_bounds`, `ChunkCoord::containing`'s two
`div_euclid`s, the grid's own bounds test and index, then `local_index`'s two
`rem_euclid`s — and the creature pass makes roughly 150–200 of them per
decision. That is ~10,000 Ir, on the order of 17% of the tick, spread across
every scan above rather than belonging to any of them.

## 4. What was cut, and the two things it is honest to say about it

Two changes landed, both **bit-identical by construction** rather than by
argument, and both gated on `lab_cost`'s world **and** field hash against the
binary built from the commit before them: `0x6b802e70596b6dfe` /
`0x40a395c839c40b42` on both sides. **The gate's own sensitivity was checked
rather than assumed** — `PIXEL_PHYSICS_MOISTURE=sweep` on the same binary moves
it to `0x8cddb2223f087648` / `0x52f6198e46dd7ee5`, so a green here is evidence
about the change and not the default state.

- **`creature::organism_sight_range` returns early when the reach is zero.**
  It computed the trait-shifted reach, then walked the animal's whole body for
  a head-composition mix, then multiplied the two — and every shipped species
  but the beetle authors `sight_range: 0`, so the body walk existed to turn a
  zero into a zero. The guard is on `base`, the value the line above has just
  produced, so a lineage that evolves `TRAIT_SIGHT_RANGE` up off a blind
  species still falls through to the mix exactly as before. `0.0 * mix` is
  `0.0` for every finite mix, and `f32::max(NaN, 0.0)` is `0.0`, so the
  infinite and NaN cases land on 0 here too. It is called **twice per tick**,
  by `creature_tick`'s cast gate and again inside `sense`.
- **`creature_tick`'s head no longer clones the species name into a `String`
  to look the material up.** `materials` and `species` are separate fields of
  `World` and can both be borrowed immutably at once; the clone bought a
  `malloc`/`free` pair per animal per tick and nothing else. The `id_of` hash
  stays — removing *that* wants a material id cached on the species, which is
  a registry change rather than a line.

**Measured, not predicted: the same callgrind run either side of the change.**
`creature::tick` goes **2,840,928,131 → 2,772,587,023 Ir** over an identical
49,568 creature ticks — **−1,379 Ir per ant decision, −2.41%** — of which
`body_mix` is **−40%** (141,069,746 → 84,771,102) and `sense` **−3.5%**. The
prediction from the before-profile was 1,556 Ir; it came in at 1,379. The run
also stocks to the identical `151 ants, 8 plants, frame 1320`, which is a
second determinism signal beside the hashes.

**The first honest thing.** That is a real saving and it is **not a
measurable speed-up on this box**. §1a put the run-to-run spread of the per-ant
slope at 1.39x across four runs and 1.18x between two identical ones; 2.41% of
the creature tick is perhaps 1.8% of the per-ant term, which is an order of
magnitude inside the noise. So no whole-frame before/after is quoted for these
two, because any number that came back would be this container's mood. The
instruction count is the gate, and it is what the claim rests on.

**The second honest thing.** 2.41% is not the answer to the owner's question,
and nothing in §3 adds up to one either. The cost of an ant is five roughly
equal things, every one of which the long ant's genome actually reads —
checked, not assumed: a per-species mask of which `BrainInput` slots any wire
touches would let `sense` skip whole scans for free, and the long ant's 101
instincts and 33 hidden wires read `FoodAdjacent`, `KinNeed`, `MoistureGrad`,
`SurfaceCurvature`, `Crowding`, `AtNest`, `Carrying`, `Energy`, `Alarm` and
the trail planes. The only inputs it never reads are the sight ones, and those
are already gated.

## 5. What a 2x would actually cost, priced rather than proposed

Ranked by what each is worth against the per-ant term, with the reason none of
them was taken in this round.

**1. Give the creature pass some parallelism — worth more than everything
else here put together.** `parallel::step` runs the CA sweep on every core;
`scheduler::step` dispatches every creature site on one, in `ActiveSite`'s
`Ord` order, which the engine documents as its one determinism surface. At
3,000 ants that leaves ~86% of the frame single-threaded. Measured on this
bed, one thread against four:

| `RAYON_NUM_THREADS` | intercept µs/tick | slope µs/ant/tick |
|---|---|---|
| 1 | 2,675 | 7.198 |
| 4 | 2,452 | 5.975 |

Three extra cores are worth **1.09x on the background and 1.20x on the ants**,
and the second of those is inside §1a's spread. **This bed is essentially a
single-threaded program**, and the ceiling on fixing that is far above any
number in §3. It is also the hardest change in this list — dispatch order is a
behaviour input, two ants due on the same frame racing for a neighbour resolve
by it — so it is a design report, not a patch.

**2. The duplicate food scan: 5,370 Ir/tick, 9.4% of the tick.** §3 has the
mechanism. What it needs is a cheap validity token — a monotonic world-write
generation, bumped on the cell-write path and on organism energy — so `act`
can prove the grid and the ledger have not moved since `sense` looked, and
re-scan when they have. The cost is one increment on the hottest write path in
the engine, which is exactly the kind of trade `CLAUDE.md` records going the
wrong way (a gate that removed 91% of a phase's work and made the frame
slower), so it wants measuring rather than assuming.

**3. Dead hidden units in `eval_brain`: ~4,300 Ir/tick, 7.5%.**
`BRAIN_HIDDEN` is 64 and the evaluation is dense — every hidden unit is
scored against all 29 inputs every tick — while the long ant wires 33 hidden
connections, so most units sum to exactly `0.0` and are then squashed. A unit
is skippable when `hh[h]` is exactly zero *and* no incoming weight clears
`W_EPS`; it then contributes `squash(0.0)` and increments `active` by nothing,
so skipping it is bit-identical including the synapse counter that prices
metabolism. The catch is that the test costs the same scan it saves, so the
mask has to be **cached per genome** and invalidated on every genome write —
a stale mask would silently change what an animal does, which is the worst
failure shape in this engine. Worth building, worth building carefully.

**4. `World::get`, 58 instructions a call, ~150–200 calls per decision.**
A one-entry chunk cache would save perhaps 13 of the 58 safely (cache the slot
index) or ~25 unsafely (cache a pointer, which `take_chunk` invalidates). That
is ~4% of the tick in exchange for a new invalidation invariant on the single
hottest shared function in the engine. **Declined on that trade**, and
recorded here so the next session does not re-derive it.

**5. The one lever that is already built and is not this lane's to pull.**
`PIXEL_PHYSICS_MOISTURE_MARKS=cells` replaces the moisture pass's row hull
with a per-cell mark set and was measured at **1.21–1.40x whole-frame** in
`evolution-lab-frame-cost-2026-09-01.md` §17.3. It is an **intercept** change,
not a per-ant one — §2 shows the moisture pass is flat in ant count — so it
does not move the 86%; it moves the millisecond underneath it. It is off by
default because it is a behaviour change (water diagonally adjacent to a mark
arrives a tick later), and §17.3 names exactly what is owed before the default
flips: a seed sweep on standing biomass, because the +6.5% figure it has is one
seed of one bed. That sweep is a well-scoped round's work and it is the
cheapest real millisecond on the table.

**6. And the design lever, which is the owner's and nobody else's.** The long
ant's `tick_interval` is 6. The per-ant *per-frame* cost is one decision
divided by that interval, so 12 halves it exactly and no code changes. That is
not a speed-up, it is an ant that thinks half as often, and whether the colony
still reads as alive is a judgement by eye rather than a number.

## 6. Reproducing all of it

```
cargo build --release --examples                      # --examples, or you measure a stale binary
RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=0,100,200,300,450 \
    frames=400 reps=3 grow=20000 rounds=300 settle=40         # §1
RAYON_NUM_THREADS=4 PIXEL_PHYSICS_MOISTURE=sweep ./target/release/examples/antcost \
    ants=0,150,300 frames=400 reps=3 grow=20000 rounds=300 settle=40   # §2, the control arm
RAYON_NUM_THREADS=1 ./target/release/examples/antcost ants=0,150,300 \
    frames=400 reps=3 grow=20000 rounds=300 settle=40         # §5.1
valgrind --tool=callgrind --callgrind-out-file=cg.out --cache-sim=no --branch-sim=no \
    ./target/release/examples/antcost ants=150 frames=1200 reps=1 grow=600 rounds=60 settle=20
callgrind_annotate --inclusive=yes cg.out             # §3; divide by the run's own creature-tick count
RAYON_NUM_THREADS=4 ./target/release/examples/lab_cost frames=6000 every=6000 colonies=1   # §4's hash gate
```

**Read `stocked` beside `want` in every `antcost` row.** The stocking loop
saturates around 460 ants on a 512-wide bed — founders starve about as fast as
they are added — and two arms of this round's first sweep were saturated
duplicates wearing fresh labels. They are excluded from every fit above. The
consequence for anyone extending this: **the fit is measured over 0–254 ants
and the 86% is an extrapolation to 3,000**, which is honest only because §1's
independent measurement at 2,473 agrees with it. Reaching three thousand ants
headlessly wants a bed with real standing food in it, not more founding rounds.
