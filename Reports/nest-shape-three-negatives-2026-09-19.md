# Four levers that do not make a shaft, and the one thing that would

*2026-09-19. Written from measurements taken in `examples/digbox` and
`examples/burrow_probe` on the night of the entrance research landing. **Every
number here is from a run in this repo**; where something is an inference it
says so. The build plan this tested is
[`nest-digging-plan-2026-09-19.md`](nest-digging-plan-2026-09-19.md), and the
dimensional audit that reordered it is
[`nest-entrance-dimensions-2026-09-19.md`](nest-entrance-dimensions-2026-09-19.md).*

---

## 0. The finding

**A colony in this engine does not dig a nest. It scratches the whole floor,
and "the nest" is the densest part of the scratching.** Four separate levers
were built or refuted against that, and none of them changes it, because none
of them is the missing thing. The missing thing is *concentration*: nothing in
the engine makes a dug cell attract the next dig.

**That mechanism has a published form, added 2026-09-20.**
[`nest-excavation-mechanics-2026-09-19.md`](nest-excavation-mechanics-2026-09-19.md)
§2 is Toffin's dig-marker rule with constants: a decaying scalar on dug cells,
a cell scored `x²/(x²+K²) + ξ` on the marker summed over its eight neighbours,
K = 50, ξ = 0.001, 100 units laid by a dig, ν = 0.05 evaporating per step. The
x² is the part that matters to §0's finding — it is what makes a slightly-dug
site beat a fresh one hard enough that **one shaft wins over many shallow
pits**, which is this report's whole complaint stated as a functional form.
Nothing here has been measured in this engine.

The evidence is one column that nobody had: the **bounding box of the room**.

| arm | room bbox | `vert` | room total | digs |
|---|---|---|---|---|
| baseline | 129w × 14h | **0.11** | 641 | 2,836 |
| narrowest door (`cols=3, rows=16`) | **208w × 12h** | 0.06 | 506 | 2,223 |
| `(Bias, Dig)` 0.15 → 0.0 | 62w × 9h | 0.15 | 253 | 725 |
| depth taxis, best arm | 113w × 18h | 0.16 | 733 | 2,746 |

`vert` is height over width; a shaft is well above 1. **Nothing here leaves
0.19**, and the room is 62–208 cells wide in a 400-wide box against a
53-column door — so most of what every census in this line has called "the
nest" is not at the nest at all.

---

## 1. The four negatives

### 1a. The door's reach is not the nest's shape

`PIXEL_PHYSICS_NEST_SITE_COLS` was added beside `_NEST_SITE_ROWS` so the
site-based `AtNest` rectangle could be narrowed to what the biology calls a
door — a shaft about one ant wide (Gravish et al., *PNAS* 2013: tunnel
diameter close to one body length), which at `body: Chain(2)` is 1–2 cells
against this engine's 46. Unset is bit-exact: `cols=unset` and `cols=26` are
identical to the cell.

**It narrows where ants stand and not where they dig.** The narrowest door
produced the *widest* room (208w × 12h). The reason is visible in the same
table: the room is far wider than the door under every setting, so the door
was never what bounded it.

### 1b. The dig target was refuted before it was built

The plan's original Step 2 was a downward bias on the dig target. It was
withdrawn on a reading of the code rather than a measurement, and the reading
is worth keeping: **the dig target and the step target are the same cell by
construction.** The dig reads `DIRS[heading]` (`creature.rs:8716`) and
`step_chain` chooses among `[(heading+AHEAD_LEFT)%8, heading,
(heading+AHEAD_RIGHT)%8]` (`creature.rs:9606`) — the middle candidate **is**
the cell just dug, and `passable` requires it to be empty. The dig is what
licenses the next step, so the excavation is the trace of where the animal
walked. An override severs that: the ant digs a cell it will never enter, the
hole self-limits at one cell per standing position, and `dig_rolls` increments
at `:8715` before the emptiness gate at `:8771`, so the roll is spent on
nothing.

This also matters for anyone re-reading `dead-ends.md`'s two dig-target
entries: they modulated a *scalar* on a fixed geometry and could never have
changed a shape.

### 1c. Steering the walk by moisture does not make a shaft

The replacement was to steer the *heading* instead, using `MoistureFront` /
`MoistureLateral` — which sample at `at(heading)`, `at(heading+AHEAD_LEFT)`,
`at(heading+AHEAD_RIGHT)`, **exactly the three directions `step_chain` chooses
among**, and which `ant.ron` weights zero times. Steering the heading drags
the dig target with it and keeps the coupling intact.

Run on a bed that can pose the question (§2), over five arms:

| arm | room bbox | `vert` |
|---|---|---|
| control | 80w × 15h | **0.19** |
| `(MoistureLateral, Turn)` +1.5 | 154w × 14h | 0.09 |
| `(MoistureLateral, Turn)` −1.5 | 113w × 18h | 0.16 |
| `(MoistureFront, Move)` +1.5 | 129w × 14h | 0.11 |
| both | 125w × 14h | 0.11 |

The control is the best of the five, and both signs of the turn weight widen
the room.

### 1d. …and the spoil teleport is holding the mechanic up, not hiding one

A fourth lever, added after the first three: all three existing
`PIXEL_PHYSICS_SPOIL_LIFT` modes abstract the **return trip** away. Inside a
shaft there is always a wall beside you, so even the default `climb` lifts the
full `SPOIL_LIFT` (160 rows) precisely where a real nest would have a haulage
corridor. The rendered box makes the question urgent — **638 cells standing
above the original ground line against 137 of void below it**, so the material
is leaving the ground rather than lining a gallery.

`=none` was added as the missing ablation, and the answer runs the other way:

| lift mode | room bbox | `vert` | roofed | hauled up | digs |
|---|---|---|---|---|---|
| `climb` (default) | 175w × 14h | 0.08 | 62 | 638 | 2,749 |
| `dig` | 138w × 16h | 0.12 | 82 | 618 | 3,209 |
| `unbounded` | 109w × 23h | **0.21** | 103 | 741 | 2,697 |
| `none` | 158w × 3h | **0.02** | 6 | 57 | 903 |

**More lifting is better.** The maximum teleport gives the best shape of the
four and the most roofed room; removing it collapses the colony's ability to
work at all — an ant that cannot put a pellet down does not dig a tunnel to
carry it along, it stops digging. The arm stays as a control, not a candidate.

---

## 2. Two instrument faults found on the way, both of which had produced findings

### 2a. A metric that follows its own dial

`digbox`'s `trace` prints *"spread over N columns × M rows"*. Swept against
`NEST_SITE_COLS` it tracked the dial beautifully — 52 → 24 → 12 → 6 → 0 — and
at `rows=16, cols=3` read **"6 columns × 21 rows"**, which reads exactly like a
shaft and was nearly reported as one.

**It is the spread of *at-nest ants*, so it follows the reach by
construction.** Set the reach to 3 columns and it reports 6: that is the
definition of the dial, not a result about digging. What caught it was
rendering the two arms and looking — the pictures were indistinguishable, a
surface pile of ants and spoil with nothing shaft-like underground, while the
numbers claimed a 90× change in aspect ratio.

`census` now returns the room's bounding box and `SUMMARY` prints
`room WxH vert`. **Read that, never the trace line, for anything about shape.**

### 2b. Every shape column in `burrow_probe` was rotation-invariant

`cells`, `perimeter`, `circularity`, `inradius` and `buds` read **identical**
for a 46×2 lens and a 2×46 shaft. Its own selftest had been proving it by
accident since it was written, by drawing `bar 64x3 (no chamber)`. The
selftest now draws the same bar rotated and asserts both halves — the five old
columns must agree, and `vert` must not:

```
                       shape    cells  perimeter    circ    inradius   buds  bboxw  bboxh    vert
       bar 64x3 (no chamber)      192        134   0.134        2.00      2     64      3    0.05
shaft 3x64 (the bar rotated)      192        134   0.134        2.00      2      3     64   21.33
```

### 2c. …and a third, which was mine

A `wet=` argument added to `burrow_probe` was inserted into the wrong arm's
branch and never ran, producing a "the moisture senses are dead at every
wetness" finding that had to be retracted within the hour. It is the exact
failure the `wire=` echo in the same commit exists to prevent. **Every
argument added that night now prints its own value**, and the harness prints
the bank's soil water beside the field's moisture so the two can never be
assumed to agree.

---

## 3. What the moisture channel actually does, since the record was wrong

`MoistureGrad` is on record as *"inert, not inverted — 0.000 at wilting point,
400, field capacity and saturation"*. Measured over 55 ants, reading what the
consumer computes:

| bed | `MoistureLateral` | `MoistureFront` |
|---|---|---|
| dry | 1 distinct, 0.0000 | 1 distinct, 0.0000 |
| uniform, field capacity | 26 distinct, ±0.62 | — |
| uniform, saturated | **3 distinct** (it clips) | — |
| graded 120 → 980 | **44 distinct**, ±1.00 | 32 distinct, 0.00–1.00 |

So the channel is **not inert**. It is dead on dry ground, saturates at the
top of the range, and has its full range only on a bed with a vertical
*profile* — which no hand-built harness had, because a uniform fill has no
gradient at any value. `wetgrad=top:bottom` now builds one in both harnesses.

---

## 4. The one measurement that says what to build next

With `(Bias, Dig)` swept, everything scales together and nothing concentrates:

| `(Bias, Dig)` | digs | room bbox | room total |
|---|---|---|---|
| 0.15 (shipped) | 2,836 | 129w × 14h | 641 |
| 0.05 | 1,510 | 158w × 11h | 443 |
| 0.0 | 725 | 62w × 9h | 253 |
| −1.0 | 338 | 47w × 2h | 98 |

**The lens is what scratching-everywhere looks like when it is turned down.**
It never becomes a shaft; it becomes a smaller scratch.

That is the shape of a system with no positive feedback in it, and it is
exactly the term the entrance research names: Toffin's rule makes the
probability of digging a cell rise with how much digging has recently happened
*next to* it, with a sigmoid response so one site wins instead of many shallow
pits spreading across a surface. **Our failure is, in the report's own words,
many shallow pits across a surface.**

In this engine that term does not need a pheromone — which is as well, because
a dig-face pheromone is measured and negative (Bruce 2015). It needs `spoil`,
which already exists as a distinct material, as a *material adjacency* test.
That is the plan's Stage 4 and on this evidence it should be first.

**But check the marker exists before building the rule that reads it.** Two
censuses, added the same night and run before any of Stage 4 was written:

```
spoil standing in the world: 143 cells, against 2662 pellets ever put down
spoil adjacency: 11 of 11157 diggable cells have spoil in reach (0.1%)
```

**About 5% of dug spoil persists as `spoil` at all**, and of the 143 cells
that do, **11 sit next to ground a dig could target.** A weight on *"is there
spoil beside me"* would be off essentially everywhere — not the always-on
failure that was expected, but the same dead end reached from the other side,
and it would have made the build fail mysteriously.

Haulage is **not** the cause, measured rather than reasoned: adjacency is
0.1–0.2% under *every* `PIXEL_PHYSICS_SPOIL_LIFT` mode including `none`, where
only 57 cells are hauled clear against `climb`'s 638. The spoil simply does
not last.

That is a design requirement rather than a refutation, and it lands on a
parameter the entrance research already flagged — Toffin's marker decays with
a lifetime of tens of steps, and Khuong's needs more than ten minutes before
any structure forms. The dimensional audit's rule for porting those is that a
decay rate belongs in **dig cycles**, and ours is unmeasured. **So the first
question for Stage 4 is not the response curve. It is how long a pellet has to
stay a pellet.**

**And one caution from the dimensional audit after that**: at `body: Chain(2)`
a shaft is 1–2 cells while the 8-neighbour kernel Toffin's rule uses is 3
across, so the kernel is wider than the feature it is meant to create.
Toffin's ant is 4 cells and his kernel 3 — narrower than the feature. That
ratio is the second thing to check, and neither is the rule itself.

---

## 5. What else came out of the same runs

- **The at-nest crowding input is saturated.** The histogram reads
  `[0,0,0,0,0,0,0,0,0,48662]` — **top tenth for 100.0% of at-nest ticks**.
  `NestRoom::occupancy` is a hyperbola chosen so that neither end saturates;
  it is pinned at one end. The cause is `room_per_ant = roofed / ants` where
  `roofed` counts materially EMPTY cells and so **excludes the ants
  themselves** — a double squeeze. A do-not-tidy note sits on
  `roofed_in_column` in `src/sim/world.rs`, because changing it moves every
  dig decision in both games and needs a switched arm, not a repair.
- **The nest census undercounts about threefold**, fixed in `lab::census` and
  `burrow_probe` the same night: roofed 157 + open 165 + **bodies 692** =
  1,014 against 941 cells hauled above the surface. Conservation closes to 8%;
  on the empty count alone it fails by 619 cells.

---

## 6. Sources

Measurements: `examples/digbox` (`ants=300 rate=8 w=400 soil=80 frames=6000`,
`RAYON_NUM_THREADS=1`) and `examples/burrow_probe` (`arms=colony`, 12 seeds).
Literature is cited through
[`nest-entrance-dimensions-2026-09-19.md`](nest-entrance-dimensions-2026-09-19.md)
and [`nest-biology-2026-09-19.md`](nest-biology-2026-09-19.md) rather than
repeated here.

**Not established, and stated so nobody upgrades it quietly**: that spoil
adjacency will concentrate digging. It is the term the research names and the
only one of the four candidates left standing, which is an argument for testing
it next and not evidence that it works.
