# The nest's mouth: a dug entrance, what it does to the nest, and what making it home costs

*2026-09-26. `lab`/`engine`. Lane note: [`lanes/nest-mouth.md`](lanes/nest-mouth.md).
Builds on [`ant-scenes-2026-09-23.md`](ant-scenes-2026-09-23.md) §19 (the
painted door) and [`lanes/nest-entrance-handoff-2026-09-20.md`](lanes/nest-entrance-handoff-2026-09-20.md)
(the dug shaft, never censused). Everything here is behind switches that are
off; nothing in either game changed.*

## 0. The answer so far

- **The founding shaft was falling in by frame 5, and nobody could see it.**
  `PIXEL_PHYSICS_NEST_SHAFT` cut its hole without lining it; the chamber roof
  dropped on frame 1 and the shaft was a dip in the surface by frame 5. The
  census that should have caught it held steady throughout, because the void
  moved up into the dip. Lined the way an ant lines a tunnel, it stands: 82 of
  82 cells open at frame 300.
- **A founding hole concentrates the whole nest, on every seed.** In the bare
  digging box, with the hole masked out of the census so only the colony's own
  digging counts, the room's middle half is narrower on 12 of 12 seeds (45 →
  27–30 columns) and deeper on 11–12 of 12, in every shaft arm. When the hole
  also counts as home, rooms that meet the owner's chamber measure appear on
  8–10 of 12 seeds where the default has almost none.
- **Making the hole home costs the foraging loop.** On the colony bed, a
  home shaft swallows ants: they go down, dig and haul at the home dig rate,
  and starve underground. A hole that is *not* home is neutral on open
  ground, beside the strip and under §19's door alike.
- **The floating dirt over the nest is tunnel lining**, and the arch it forms
  across the sky is built by the spoil lift. That is goal 2's lead, carried by
  review cards, not by this report.

## 1. The founding shaft fell in by frame 5, and the census could not see it

`digbox`, `PIXEL_PHYSICS_NEST_SHAFT=20`, no ants, the `main` binary:

| frame | roofed | open | the cut's own cells still open |
|---|---:|---:|---:|
| 0 | 38 | 44 | 82 of 82 |
| 1 | 33 | 49 | 42 |
| 5 | 24 | 58 | 6 |
| 300 | 24 | 58 | 6 |

Frame 0 is the positive control exactly: open = 2·20 + 4 = 44 and roofed =
4·10 − 2 = 38. The cut was cells set empty and nothing else, a hand-carved
void, and `line_burrow`'s own doc records what the powder sweep does to one.
**`roofed + open` reads 82 at every stop**: the chamber roof falls in, the soil
column over it drops, and the same volume reappears as a dip in the surface,
which the census counts as open room. So a collapse read as nothing happening.
The right-hand column is `digbox`'s new `cut:` line, which counts the cut's own
cells from the footprint the cut now records on its site.

**The repair**, all inside the `NEST_SHAFT` path (unset returns before reading
a cell):

- The cut runs the ant's own lining over every cell it removes
  (`pack_neighbours`, split out of `line_burrow` so `CreatureStats::packed`
  stays ant work). **82 of 82 open at frames 0, 1, 5, 30 and 300**; unlined
  (`BURROW_LINING=off`), 42 and then 6.
- It freezes the world's genesis (sky surface, underground map, ground datum,
  room datum) **before** it cuts. Harnesses found before their first step, so
  the shaft was frozen in as terrain: it drew as bright sky blue, and the room
  census walked a shaft column down to the chamber floor. It now draws as a
  dark hole with daylight fading down it, as a founding in the running game,
  after frame 1, always did.
- `PIXEL_PHYSICS_NEST_SHAFT_WIDTH` (default 2, the biology's one body length;
  4 is the owner's 2026-09-20 "passage ~4").
- The footprint is recorded on the site (`NestSite::shaft`), because after the
  cut `colony_surface` in a shaft column finds the chamber floor.

Guard `the_founding_shaft_stands` runs both arms under both drivers, with the
unlined cut as its control; watched red with the lining disabled.

## 2. What the floating dirt is

`digbox` at the settings the owner judged (300 ants, 6,000 frames, seed 0),
drawn plain and painted by class from one run (`tintout=`):

- The grey grit is **tunnel lining**, 1,396 cells in view (421 above the old
  ground line), and **ants**, 1,234 cells. Lining has its own greyer palette.
- The arch across the top of the sky is lining and spoil: 266 of the 269 spoil
  cells in view are above the ground.
- **82% of spoil drops (5,292 of 6,434) went up the digger's own column**
  instead of beside it; 362 cells of ground have no path down to the floor;
  every cell standing on nothing (102) is lining.

`PIXEL_PHYSICS_UNPACK=on` (built 2026-09-20, never judged) turns stranded
lining back to falling spoil. On this seed the arch goes: stranded ground
362 → 37. It is on a blind card to the owner with that count in its meta.

## 3. A founding hole concentrates the nest

`digbox`, 300 ants, 6,000 frames, seeds 1–12, one binary. **Scored with the
founding cut masked out of the census**: a shaft arm with no ants at all reads
a 13-column room by construction, and the masked census reads 0 there. Seed
by seed against the default:

| arm | own room | depth | height / width | middle-half width | chambers |
|---|---|---|---|---|---|
| lined shaft, 2 wide | bigger 10 / 2 | 11 / 0 | 11 / 0 | **narrower 12 / 0** | 3 / 1 |
| lined shaft, 4 wide | 9 / 3 | 12 / 0 | 12 / 0 | **narrower 12 / 0** | 5 / 1 |
| shaft is home, 2 wide | 10 / 2 | 12 / 0 | 12 / 0 | **narrower 12 / 0** | 8 / 1 |
| shaft is home, 4 wide | 10 / 2 | 12 / 0 | 12 / 0 | **narrower 12 / 0** | **10 / 0** |

Medians: middle half 45 → 27–30 columns, depth 23 → 28–32 rows, chambers
0 → 1.5 in the home arms, rooms about 8–10 tall and 8–12 wide with passages
still 2 cells (the owner's spec is 4). Home against not home changes the shape
little and keeps the hole clearer: ground inside the cut lower on 11 of 12
seeds. What fills the cut at 6,000 frames is ants, in a box that grows past
800 of them.

This answers the handoff's open question (*does a colony given a hole keep
it, deepen it, or fill it in?*): it keeps it, and digs around it rather than
across the floor. It is also the first lever in this line that moved the
middle-half width at all, and it is a founding fact, not a behaviour.

## 4. On the colony bed a home shaft swallows ants

The gap-90 bed (the brief's command), 24 seeds paired against the default,
which starves 277 (279 by the harness's own count). The default arm on this
branch reproduces the reference binary's three logs line for line.

**Beside today's strip**, before §19 was in:

| shaft rows | not home: starved (fewer / more) | home, 2 wide: starved (fewer / more) |
|---|---|---|
| 6 | 270 (13 / 7) | 303 (10 / 12) |
| 10 | 265 (14 / 7) | 315 (8 / 14) |
| 20 | 293 (11 / 12) | 309 (8 / 15); 4 wide **376 (4 / 18, p 0.004)** |

**On §19's door** (a scratch merge of PR #491 and this branch, never pushed;
the default is bit-exact there too, and the door reproduces §19's 209):

| arm | starved | against the door | loops |
|---|---:|---|---:|
| default | 277 | | 366 |
| door (`NEST_DOOR=2`) | 209 | | 465 |
| door + 6-row shaft | 195 | 13 fewer / 10 more | 423 |
| door + 6-row shaft, home | 254 | 8 fewer / 14 more | 377 |

The funnel puts the loss at the first stage: with the 20-row, 4-wide home
shaft beside the strip, ants that ever reach the food fall **271 → 145**,
while those that reach it close a loop at the same rate. **Traced, every
founder that never reached the food**:

| arm | never reached | decisions underground | median depth | died underground |
|---|---:|---:|---:|---:|
| default | 221 | 32% | 1 row | 85 |
| shaft, not home | 247 | 48% | 2 | 136 |
| shaft home, 4 wide | 343 | **67%** | **17** | **277** |

A third of those decisions hold a pellet. One ant's life: born on the far end
of the strip, down the shaft by frame 800, then 2,300 frames 8–22 rows down
with `AtNest` on, alternately digging and hauling, energy 0.7 → 0, never back
up. The mechanism is three things the code already says: underground there
is no food trail; an empty ant off a trail walks without direction
(`how-the-ant-works.md` §6d); and at home the dig gate fires about five times
as often (hidden units 5/6), so the ants in the cavity dig and enlarge it.
**A home is where a colony spends its time, and this engine's colony spends it
digging.**

## 5. The lab

*(pending — the painted door fails there on 12 of 12 seeds because plant
litter buries it; the question is whether a dug mouth stays open.)*

## 6. Instruments this found or fixed

- **`digbox`'s void census counts a collapse as room.** Void conserved into
  a surface dip reads as open room, so `roofed + open` cannot see a hole fall
  in. The `cut:` line counts the cut's own cells.
- **A shaft arm's room census includes the room founding dug.** Scored raw,
  every shaft arm's middle half is narrower by construction (13 columns with
  no ants at all). `census_masked` and the SUMMARY line *dug by the colony,
  outside the founding cut* score what the colony did.
- `digbox` now echoes `NEST_SHAFT`, `_WIDTH` and `NEST_HOME` (nothing did),
  and `tintout=` paints every class of ground flat from the same run.
- Harness order matters for the shaft: a harness that founds before frame 1
  froze the cut in as terrain until the cut froze genesis itself.

## 7. Commands

```
# the shaft at frame 0 and after (A1)
PIXEL_PHYSICS_NEST_SHAFT=20 digbox ants=0 frames=300 stops=0,1,5,30,300 out=… crop=70,4,60,50 scale=6
# what floats, plain and painted by class (A2)
digbox ants=300 rate=8 w=200 soil=60 frames=6000 stops=6000 out=plain.png tintout=tint.png crop=20,0,160,70 scale=4
# the shape sweep (A5): seeds 1-12, arms default / NEST_SHAFT=20 [_WIDTH=4] [NEST_HOME=shaft]
digbox ants=300 rate=8 w=200 soil=60 frames=6000 stops=0,6000 seed=N
# the colony bed, 24 seeds: the brief's command, three batches, then scripts/antloop.py
```
