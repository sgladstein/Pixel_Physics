# Giving the creature pass parallelism: it can be done exactly, and it does not pay

*2026-09-13, round 33. The task was the owner's standing first priority —
`Reports/evolution-lab-creature-cost-2026-09-13.md` §5 item 1 — and round 32's
instruction was "you are the build". It is built, it reproduces the serial
world bit for bit, and **it ships off**, because on a four-core box it costs
3-4% of the frame rather than saving any.*

**The three findings, before the evidence.**

1. **The invariant holds and is cheap to state.** `sense` and
   `brain::eval_brain` are pure reads of an immutable `&World` returning a
   value — 44.5% of one ant's decision, by round 32's profile — so they can be
   computed for many animals at once, ahead of their turn, while every *write*
   stays exactly where it was, one site at a time in `ActiveSite`'s `Ord`
   order. **The engine's one determinism surface is untouched.** A speculation
   is consumed only over ground nothing has written since it was taken; the
   world and field hashes are unchanged at 6, 150, 300 and 450 ants, and move
   under the control that skips the check.
2. **The economics do not.** A speculation costs `c / S` of wall clock and
   saves `c` when it is used, so the mechanism pays **iff hit rate x parallel
   speedup > 1**. Measured here: `S = 2.1` from four cores, so the hit rate has
   to clear **48%**. It is **61% at 150 ants and 32% at 450**, and it falls
   with density because the colony is the thing invalidating it — ants stand
   touching, and `BrainInput::KinNeed` reads a nestmate's energy, which every
   one of them rewrites every tick. The owner plays at a thousand and more.
3. **The hole this would have shipped with was found by a differential, not by
   a hash.** `PIXEL_PHYSICS_CREATURE_PAR=verify` recomputes every accepted
   speculation and names the brain input that differs. It pointed straight at
   `evaporation::tick`, which damps the air over a surface it has just dried
   from inside the same site loop, two field blocks wide through a bilinear
   read. **It showed at 450 ants and not at 150 or 300**, so the obvious gate
   would have called it clean.

Everything below was measured on one four-core cloud container with
`RAYON_NUM_THREADS` pinned, both arms in one process, round-robin by rep, read
at the minimum over reps. The bed is round 32's: 512x512, `herb` founders,
`longant` colonies, seed 1. The harness is
[`examples/antcost.rs`](../examples/antcost.rs), which round 33 gives a `par=`
arm and a world-hash column.

## 1. What was built

Three phases where there were two:

1. **Speculate** (parallel, `&World`): for every creature site due this frame,
   compute `sense` and `eval_brain` and record the rectangles the reads went
   through. `creature::speculate_window`.
2. **Dispatch** (serial, `&mut World`): unchanged. Each creature, in `Ord`
   order, takes its cached read phase **if nothing inside those rectangles has
   been written since** and recomputes it otherwise.

The validity test is `sim::writewatch::WriteWatch`, and the marks are made at
the seams every writer in the engine already goes through — `World::set`,
`World::organism_mut`, `World::deposit_pheromone` and the field painters —
rather than at a list of callers. That is the same argument `World::set`'s own
doc makes for its two existing seam checks: *an enumeration that has to stay
complete is the failure mode this project keeps rediscovering.*

**Three maps, because the reads have three different reaches**, and collapsing
them was measurably wrong twice:

| map | resolution | marked by | read reach |
|---|---|---|---|
| cells | 4x4 cells | `World::set`, `deposit_pheromone` | body ring, crowding disc, curvature disc, sight cast, three pheromone sensors |
| organism state | 4x4 cells | `World::organism_mut` | one cell off the body — where `is_living_kin` and `kin_deficit` resolve an owner |
| field | 16x16 cells (`FIELD_SCALE`) | `paint_field`, `add_vapour` | the moisture, light and temperature samples, widened one field cell for the bilinear |

Pooling the first two made every ant that merely updated its own energy
invalidate every neighbour out to the *cell* reach: **the hit rate went from
42% to 58% at 150 ants when they were split.** Expressing the field's reach in
cells rather than at the field's own resolution meant one evaporating surface
dirtying a 67x67 cell square, and cost **34.8% against 61%**.

## 2. The gate, and the proof that it is sensitive

`lab_cost frames=6000 every=6000 colonies=1`, the bed round 32 used:

| | world hash | field hash |
|---|---|---|
| before this change | `0x6b802e70596b6dfe` | `0x40a395c839c40b42` |
| speculation on, every window forced | `0x6b802e70596b6dfe` | `0x40a395c839c40b42` |
| **`PIXEL_PHYSICS_CREATURE_PAR=unchecked`** | `0xb80855aa15c7bbc2` | `0x40189bd5c00e4c82` |

**That bed stands six ants up, which is not the question.** The gate that
matters is `antcost`'s own, new in this round: two `par` arms of one ant count
are the same bed stocked by the same deterministic loop, so their world hashes
must agree. At 450 ants:

| arm | world hash | creature ticks/f | moves/f | blocked % |
|---|---|---|---|---|
| `par=off` | `0x4faca71b907ec77c` | 87.6 | 40.3 | 11.1 |
| `par=on` | `0x4faca71b907ec77c` | 87.6 | 40.3 | 11.1 |
| `par=unchecked` | `0xd045d613da14acd6` | 87.9 | 39.6 | 10.3 |

The counter columns are the other half of the pair `CLAUDE.md` asks for —
**a cost that vanishes may be work that vanished**, and these say the animals
did the same things.

**And the hash is not the instrument that found the bug.** At 450 ants the two
arms first read `0xbd3d6ddb02a0cb5c` against `0x4faca71b907ec77c`, and a hash
says only *no*. Three things narrowed it, in order:

- **`PIXEL_PHYSICS_CREATURE_PAR_WINDOW=1`** — one creature per window — came
  back clean, which rules out everything except a neighbour's action.
- **Widening every rectangle to a margin of 12** still diverged, which rules
  out a rectangle that is merely too small and says the mark itself is
  missing.
- **`PIXEL_PHYSICS_CREATURE_PAR=verify`**, which uses the cached sense's
  *validation* and then recomputes the sense anyway and compares, named the
  slot: 5, 6 and 20 — `MoistureFront`, `MoistureLateral`, `MoistureGrad`, and
  nothing else. Every one of them a field read. `World::add_vapour`, called by
  `evaporation::tick` from inside this very site loop, was the writer.

`verify` is kept for the next person who touches `sense`. It is the only
instrument here that answers *which read* rather than *whether*.

## 3. Why it does not pay

**The arithmetic is one line.** Speculating for one animal costs `c / S` of
wall clock, where `c` is what its read phase costs serially and `S` is what the
parallel phase actually achieves; it saves `c` if the cache is used and saves
nothing if it is not. So the mechanism pays iff **`p x S > 1`**.

### 3a. `S` is 2.1 on four cores, and the dispatch count is what makes it

`speculate_window` carries its own clock, because a parallel phase that is not
parallel is invisible in a frame total — it looks exactly like an expensive
one. At 450 ants, one dispatch per frame:

| `RAYON_NUM_THREADS` | speculation phase, µs/frame |
|---|---|
| 1 | 501 |
| 2 | 332 |
| 4 | 236 |

**2.12x from four cores.** The box itself is not the limit — four independent
processes on it scale 3.4x — so what is left is memory: one decision is ~200
`World::get` calls chasing a chunk map, and four threads doing that contend.

**Dispatch count matters more than anything else about the window.** The same
work measured **474 µs/frame over three dispatches of 32 and 236 over one**, so
~80 µs a dispatch is rayon's fixed cost at this size. The window therefore
defaults to one per frame. It is **not** a hit-rate knob: the hit rate reads
57.8% at windows of 16, 32 and 64, and the same at 32 against 1,000 — what
spoils a speculation is a *neighbour* acting, not the length of the run.

### 3b. `p` falls with density, and the colony is why

| ants | hit rate | of the misses, on the organism-state map |
|---|---|---|
| 150 | 61% | 83% |
| 450 | 32% | 83% |

The misses are not slop. `adjacent_food_counted` walks the ring of the
animal's own body and resolves the owner of anything living it touches —
`BrainInput::KinNeed` is that nestmate's energy deficit — and metabolism
rewrites every animal's energy every tick. **Two ants standing next to each
other genuinely invalidate one another, every time either of them acts.** A
colony is defined by animals standing next to each other, and the owner plays
at a thousand to three thousand of them in this bed.

Declining to speculate for an animal with an earlier due neighbour within 18
cells removes the waste and the benefit together: **6.5% cached at 450 ants**,
because at that density almost every animal has one. That filter
(`PIXEL_PHYSICS_CREATURE_PAR_NEAR`) therefore **ships at 0**, i.e. off; the hit
rates above are measured with it off, which is the shipped configuration.

### 3c. So, whole-frame, paired and alternating

| bed | `par=off` | `par=on` | |
|---|---|---|---|
| 450 ants | 4,050 µs/tick | 4,159 | **+2.7%** (shipped default) |
| 450 ants | 3,950 | 4,098 | **+3.7%** (with the neighbour filter on) |
| 0/150/300 ants, fitted | 2,077 + 3.670 µs/ant | 2,104 + 3.739 | **+1.9% on the per-ant term** |

Every one of those is a paired comparison inside one process, `RAYON_NUM_THREADS`
pinned at 4, minimum over reps, with the two arms' counter columns identical.

**With the mechanism off the hooks cost nothing measurable**: at zero ants,
three alternating runs of the pre-change binary against `par=off`, minima 1,893
against 1,828 µs/tick — the new one nominally faster, i.e. inside a ~5%
spread.

## 4. What would have to change

Stated as the two terms, because that is what a later round can act on.

**Raise `S`.** Four cores gave 2.12x on a memory-bound scan. More cores are
the obvious lever and this container has four; the owner's machine is not this
container, and `antcost par=on,off` re-runs the whole comparison in one
command wherever it is run. **This is not a claim that it pays there** —
`CLAUDE.md` is explicit that an advantage which only appears at larger width
has to be measured rather than argued — it is a statement of what the
measurement would be.

**Raise `p`.** The ceiling is set by how much of `sense` genuinely depends on a
neighbour, and one number bounds it: the organism-state map is 83% of all
misses. The obvious next build is a **partial** recompute — on a miss that is
*only* on the state map, keep the cached pheromone, field, crowding and
curvature inputs and recompute `adjacent_food_counted` alone, then re-run
`eval_brain`. That turns a 25,500-instruction miss into a 13,500-instruction
one. Priced against the numbers above it roughly doubles the parallel fraction
at 450 ants and still lands at about break-even on four cores, which is why it
was not taken here.

**Or abandon the read-phase split.** It is bounded at `44.5% x (1 - 1/S)` of
the creature pass by construction. The unbounded version is to run *whole
ticks* in parallel over animals whose read **and** write footprints are
disjoint — the same independent-set argument `parallel.rs` makes for chunks,
which is exact rather than speculative and needs no validity test at all. It
needs `&mut World` split across workers for objects that do not align with
chunks, which is a real piece of work and is why round 33 did not attempt it.

## 5. Reproducing all of it

```
cargo build --release --examples                      # --examples, or you measure a stale binary
RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=450 par=on,off,unchecked \
    frames=200 reps=2 grow=6000 rounds=200 settle=40          # §2's gate and §3c
RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=450 par=verify \
    frames=25 reps=1 grow=6000 rounds=200 settle=40           # §2's differential
RAYON_NUM_THREADS=1 PIXEL_PHYSICS_CREATURE_PAR_WINDOW=1000 ./target/release/examples/antcost \
    ants=450 par=on frames=150 reps=2 grow=6000 rounds=200 settle=40   # §3a, then at 2 and 4
RAYON_NUM_THREADS=4 PIXEL_PHYSICS_CREATURE_PAR_MIN=0 ./target/release/examples/lab_cost \
    frames=6000 every=6000 colonies=1                         # §2's hash table
```

Switches, all controls rather than settings: `PIXEL_PHYSICS_CREATURE_PAR`
(`off` default, `on`, `unchecked`, `verify`), `..._WINDOW`, `..._MIN`,
`..._NEAR` (0), `..._TILE` (4), `..._MARGIN` (1). Every one of them is also a field of
`World::creature_par`, which is how `antcost` puts two arms in one process.

**Read `cached%` and `spec µs/f` beside every timing.** A parallel read phase
that is always invalidated is the same wall clock as no parallel read phase at
all, and the frame total cannot tell them apart.
