# The nest's mouth: a dug entrance, what it does to the nest, and what making it home costs

*2026-09-26. `lab`/`engine`. Lane note: [`lanes/nest-mouth.md`](lanes/nest-mouth.md).
Builds on [`ant-scenes-2026-09-23.md`](ant-scenes-2026-09-23.md) §19 (the
painted door) and [`lanes/nest-entrance-handoff-2026-09-20.md`](lanes/nest-entrance-handoff-2026-09-20.md)
(the dug shaft, never censused). Everything here is behind switches that are
off; nothing in either game changed.*

## 0. The answer

- **The founding shaft was falling in by frame 5, and nobody could see it.**
  `PIXEL_PHYSICS_NEST_SHAFT` cut its hole without lining it; the chamber roof
  dropped on frame 1 and the shaft was a dip in the surface by frame 5. The
  census that should have caught it held steady throughout, because the void
  moved up into the dip. Lined the way an ant lines a tunnel, it stands: 82 of
  82 cells open at frame 300.
- **A founding hole concentrates the nest, when home reaches down it.** In
  the bare digging box at 40 ants, with the hole masked out of the census so
  only the colony's own digging counts, a 20-row hole the colony treats as
  home gives a dug nest narrower in its middle half on 11 of 12 seeds
  (39.5 → 17 columns), deeper on 10 and bigger on 11. A home only at the
  mouth does not: the dig gate fires at home, so the colony digs at the
  surface.
- **On open ground one home point pays, and §19's painted door is the best
  of them.** On the colony bed (24 seeds) the door starves 209 against 277.
  The door with a dug mouth that is home is as good (221; 10 seeds better, 9
  worse against the door). **The deeper home reaches into the hole, the more
  of the gain it gives back** (the whole cut as home, 254), because ants that
  go down dig at the home rate and starve underground. With no paint at all
  more ants make a full loop (456 against 371, p 0.017), but the food they
  bring falls into the hole and is buried, and starvation barely moves (256).
- **In the lab box the dug mouth does not stay open.** It is buried by
  frame 30,600 on 11 or 12 of 12 seeds in every dug arm, under the colony's
  own delivered food and the roots that grow in it. Every narrow home records
  fewer deliveries than today's strip (lower on 11 or 12 of 12 seeds). But a
  delivery is a drop made at home, so **a smaller home counts fewer of them
  by definition**, and a crumb picked up at home and put back counts twice.
  A new counter measured that: 86% of the strip's deliveries are food picked
  up at home first (median of 12 seeds). Net of it, **the dug mouth brings
  home as much as the strip** (604 cells against 712, 6 seeds up and 6 down),
  and the painted door about half (384, lower on 10 of 12). On the outcomes
  that do not depend on where home is (births, food eaten, colony-frames and
  starvation), no arm separates from today at 12 seeds.
- **A home that follows the pile over the mouth builds a tower.** This was
  tried once after the stop rule, aimed at the burial. The colony stacks its
  food over its own door: a median of 34.5 rows high at frame 30,600, and over
  15 rows on 12 of 12 seeds by 60,300. The fixed mouth's pile stays at a
  median of 3 rows, with none over 15. Its lab deliveries rose 3,204 → 8,015
  and were first read as a success. They included every drop on the tower,
  and over 24 seeds its colony is smaller than today's (colony-frames lower
  on 17 of 24). Reverted (§6).
- **So no mouth tried is better than today on both beds, and the look and
  the loop want opposite homes.** The brief's stop rule applies: the variants
  end here. The remaining switches stay, off and bit-exact on all three
  harnesses, and whether any becomes the default is the owner's ruling.
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

`digbox` at the settings the owner judged (300 ants, 6,000 frames, seed 0;
a crowd that breeds past 800, see §3), drawn plain and painted by class from
one run (`tintout=`). **At 40 ants the arch does not form** — 1–28 stranded
cells across four seeds, 0–2 with `UNPACK` on — so everything in this section
is a big-colony effect, which is the regime the owner's own complaint came
from:

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

**Owner, 2026-09-26: "Make sure you are not using too many ants in your tests
and always take snapshots at multiple times."** The 300-ant settings below
breed past 800 ants by frame 6,000 (`digbox`'s endowment of 20,000 lets every
ant bud), so the primary result is the modest one: **40 ants at `energy=1000`**
(under the 1,100 budding threshold, in a box with no food, so the count stays
40), 12,000 frames, census at frames 0/3,000/6,000/12,000, seeds 1–12, one
binary, scored with the founding cut masked out. Against the default:

| arm | own room bigger | deeper | middle-half narrower | median middle half |
|---|---|---|---|---|
| default | | | | 39.5 columns |
| lined shaft, 10 rows | 7 / 5 | 3 / 7 | 5 / 7 | 31 |
| shaft is home, 10 rows | 8 / 4 | 4 / 7 | **12 / 0** | 18 |
| lined shaft, 20 rows | 5 / 7 | 6 / 6 | **11 / 1** | 21.5 |
| shaft is home, 20 rows | **11 / 1** | **10 / 2** | **11 / 1** | 17 |

At this size the hole silts up: by frame 12,000 a median 35 of the 20-row
cut's 82 cells are still open (11–18 of 42 at 10 rows), filled mostly by
packed tunnel wall and loose soil; home against not home barely changes that
(more open on 6–7 of 12). No chamber reaches the owner's measure in any arm.
Almost nothing floats: 1–28 cells of ground with no path to the floor across
four seeds.

**With the 6-row shaft the bed arms use** (the final binary, same settings;
its default reproduces the table's default on all 12 seeds), **only a home
that reaches down the hole shapes the nest**:

| arm, 6-row shaft | own room bigger | deeper | middle-half narrower | median middle half |
|---|---|---|---|---|
| no paint, the whole cut is home | **10 / 2** | 6 / 6 | 8 / 3 | 23.5 columns |
| no paint, the mouth is home | 5 / 7 | 4 / 6 | 6 / 6 | 38.5 |
| door, the mouth is home | 5 / 7 | 2 / **10** (shallower) | 8 / 4 | 30 |

The dig gate fires at home, so a home at the surface digs at the surface:
without paint the mouth leaves the nest as wide as it was, and with the door
it comes out narrower on 8 of 12 but shallower on 10 of 12 (20 → 16.5 rows),
a scrape rather than a shaft. The mouth arms also silt their cut harder (5–6
of 26 cells open at frame 12,000, against 11.5 with the whole cut home).
**So the look and the loop want opposite homes**: the shape wants home to
reach down the hole, and the colony bed pays for every row it does (§4).

**The crowded box, for the record** — 300 ants, 6,000 frames, same scoring.
Here the effect is stronger and the cut is full of ants rather than dirt:

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

## 4. On the colony bed one home point pays, and a home down the hole gives it back

The gap-90 bed (the brief's command), 24 seeds paired against the default,
which starves 277 (279 by the harness's own count). The default arm on this
branch reproduces the reference binary's three logs line for line.

**Beside today's strip**, before §19 was in:

| shaft rows | not home: starved (fewer / more) | home, 2 wide: starved (fewer / more) |
|---|---|---|
| 6 | 270 (13 / 7) | 303 (10 / 12) |
| 10 | 265 (14 / 7) | 315 (8 / 14) |
| 20 | 293 (11 / 12) | 309 (8 / 15); 4 wide **376 (4 / 18, p 0.004)** |

**On §19's door**, with the mouth modes. The last two rows are from this
branch after `main` was merged in; its default reproduces the reference logs
line for line, and its door reproduces the scratch merge's door line for line
(so the anchor fix moves nothing without a shaft). The rows above them come
from a scratch merge of PR #491 and this branch, whose default and door were
bit-exact the same way.

| arm | starved | against the default | against the door | loops |
|---|---:|---|---|---:|
| default | 277 | | | 366 |
| door (`NEST_DOOR=2`) | 209 | 18 fewer / 4 more (p 0.004) | | 465 |
| door + 6-row shaft | 195 | 19 / 4 (p 0.003) | 13 / 10 | 423 |
| door + 6-row shaft, home = the mouth | 221 | 18 / 4 (p 0.004) | 10 / 9 | 395 |
| door + 6-row shaft, home = the whole cut | 254 | 13 / 10 | 8 / 14 | 377 |
| **no paint** + 6-row shaft, home = the mouth | 256 | 15 / 9 (p 0.31) | 7 / 16 (p 0.09) | 445 |

**One home point is what pays, painted or dug; the deeper home reaches into
the hole, the more of that it gives back** (the door 209, home to the mouth
221, home to the whole cut 254). Every arm clears the bar against the
default: no more starved, no fewer loops. None beats the painted door.

Without paint, more ants make a loop (295 against 229) and fewer never reach
the food (145 dead against 200), but ants that made exactly one loop starve
at 44% against 32%. **Traced, every one-loop ant that starved** (77): 65 of
them made their delivery inside the mouth rather than on the surface (26 of
47 with the door and the shaft, 10 of 54 with the door alone), and one ant's
life (seed 1, ant 4) shows the rest. After its loop at frame 5,436 it stayed
in and around the mouth for about 13,000 frames at full energy, eating and
re-dropping the food others brought, standing still most of the time. Then it
walked west and up the box wall, and starved at frame 23,292 without going
back to the food. That is the old never-reached failure arriving later rather
than a new one. Food carried into the hole is also
buried more often: 286 crumbs over the 24 runs have no open neighbour,
against 96 on the default.

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

## 5. In the lab box a narrow home records fewer deliveries, and the mouth is buried

§19's setup (`labforage scenario=played_bed frames=120000`, 12 seeds), every
arm from one binary. That binary's default reproduces the `main` binary's
seed-1 log in full, and its default and door arms reproduce §19's stored
medians exactly. Paired against the default:

| lab, 12 seeds, median | deliveries | nest visits | food eaten | births | colony-frames | colonies lost |
|---|---:|---:|---:|---:|---:|---:|
| default | 5,396 | 8,948 | 1,158k | 590 | 10.7M | 1 |
| door (`NEST_DOOR=2`) | 1,279 (0 up / 12 down) | 1,105 (0 / 12) | 1,151k (3 / 9) | 523 (4 / 8) | 10.6M (5 / 7) | 4 |
| door + 6-row shaft | 2,072 (1 / 11) | 1,626 (0 / 12) | 1,039k (5 / 7) | 552 (6 / 6) | 8.7M (6 / 6) | 5 |
| door + 6-row shaft, home = the whole cut | 3,696 (1 / 11) | 3,884 (0 / 12) | 826k (4 / 8) | 310 (5 / 7) | 7.1M (3 / 9) | 3 |
| door + 6-row shaft, home = the mouth | 3,204 (0 / 12) | 1,556 (0 / 12) | 1,060k (4 / 8) | 486 (4 / 8) | 7.5M (4 / 8) | 2 |
| **no paint** + 6-row shaft, home = the mouth | 2,257 (1 / 11) | 1,272 (0 / 12) | 1,062k (6 / 6) | 510 (6 / 6) | 9.3M (6 / 6) | 2 |

*Colony-frames* is the number of ants alive summed over the run: `labforage`'s
ants column, sampled every 900 frames. It is how much colony there was, and
unlike deliveries it does not depend on where home is.

*Colonies lost* counts a seed whose colony died young (fewer than 50 births
in 120,000 frames) or was extinct at the end, once. The lab is far more
chaotic per seed than the bed: on seed 4 the default colony has 60 births and
the door's 1,615, and on seed 3 the reverse, 1,245 against 5.

**Every narrow home records fewer deliveries than the strip**, lower on 11 or
12 of 12 seeds, and against the door the dug arms record more (the home shaft
12 of 12, the plain shaft 10 of 12, p 0.039, the mouth 9 of 12). **But a
delivery is a drop made at home**, so a smaller home counts fewer of them by
definition, and a crumb picked up at home and put back counts twice. The new
counter (§7) measures that on the same 12 seeds; the 48 re-runs reproduce
every other number in the stored logs:

| lab, 12 seeds, median | deliveries | picked up at home | share | net into home |
|---|---:|---:|---:|---:|
| default | 5,396 | 4,578 | 86% | 712 |
| door (`NEST_DOOR=2`) | 1,279 | 956 | 70% | 384 (2 up / 10 down, p 0.039) |
| door + 6-row shaft, home = the mouth | 3,204 | 2,349 | 80% | 604 (6 / 6) |

**Most of every arm's deliveries are the same food going round**, and the
strip churns the most. Net of it, the dug mouth brings home as much as the
strip, so its whole deficit in deliveries was the strip's churn. The painted
door's loss is real, and about half the size deliveries said. **And deliveries
is not what decides the colony here.** Food eaten, births, colony-frames and
survivors are what do, and on those no arm separates from the default: 3 to 6
of 12 up on every column (the whole cut's colony-frames, 3 / 9, p 0.15).
Colonies lost split by whether the hole is home, not by whether there is
paint: the painted door loses 4 colonies of 12 and the door over a plain shaft
5, while every arm whose hole is home loses 2 or 3, against the default's 1.
In this box ants bud anywhere and eat where they find food, so a small home
costs the loop count far more than it costs the colony. **At 12 seeds, 1
against 2 against 4 lost colonies cannot be told apart by any test** (Fisher's
exact, 4 of 12 against 2 of 12, p 0.64).

**Does the mouth stay open to the surface? No, and the colony's own food is
what closes it.** `labshot`'s new census, run on the same 12 seeds (its ant
count equals `labforage`'s at all 180 stops checked), walks each shaft column
up from the mouth until it meets open air. The colony is founded at frame
6,000, so the first stop is 300 frames later:

| frame | mouth reachable from the surface, no paint | door + mouth | door + whole cut as home | cells over the mouth (median, no paint) |
|---:|---:|---:|---:|---:|
| 6,300 | 7 of 12 | 5 of 12 | 6 of 12 | 0 |
| 12,600 | 2 of 12 | 2 of 12 | 2 of 12 | 2 |
| 30,600 | 1 of 12 | 0 of 12 | 0 of 12 | 5 |
| 60,300 | 1 of 12 | 0 of 12 | 2 of 12 | 5.5 |
| 119,700 | 9 of 12 | 6 of 12 | 8 of 12 | 0 |

What fills the cut and covers the mouth, summed over the stops, is mostly
**food the colony delivered** (crumbs: 715, 648 and 877 cell-observations in
the three arms) and **roots**: the lab founds into ground the plants have
already threaded, and the cut refuses living cells (`is_diggable_ground`), so
roots stand in the shaft from the first frame (89-108 cells at frame 6,300).
Seeds and fruit pips turn up in it too (58, 56 and 55). So a home fixed at
the founding footprint ends up under a heap of the colony's own deliveries
and the plants that root in them, and a laden ant on top of that heap is
above home, not at it. That is §19's buried door again, with the colony
doing the burying. The mouths reopen by the end (6-9 of 12) as the stand
dies back. Whether the burial is what cuts deliveries was not measured
directly: this harness counts no refused drops. §6 tested it the other way,
by moving home up onto the pile, and the colony built a tower.

## 6. A home that follows the pile builds a tower

§5 named the lab's condition: a home fixed at the founding footprint ends up
under the colony's own deliveries. So one more variant, aimed at that
mechanism, with its predictions and a stop condition written before any code
(lane note, predictions 19–22). `PIXEL_PHYSICS_NEST_HOME=mound` made home the
mouth **and** any unbroken pile over the mouth's columns, of anything but open
air, gas or spoil, up to 32 rows. Home would rise with whatever the colony
piles on its doorstep. Spoil was left out because `world.rs`'s
`register_nest_site` already warns that a re-measured surface "would climb with
the heap and call the top of a tailings pile home". Food was not, and food is
what climbed.

**It builds a tower.** Ants put food down only at home, home is now the top
of the pile, and crumbs do not slide, so every delivery can raise home by a
cell. The height of the pile over the mouth's columns, above the ground either
side, was read off `labshot`'s frames. Only brown pile cells count; plants and
sky do not. Checked against a hand count of seed 7 at frame 30,600: 47 rows by
hand, 49 by the script. The fixed mouth is the control: the same colony and
food, with home held at the footprint.

| frame | fixed mouth: median rows (max) | mound: median rows (max) | mound seeds over 15 rows |
|---:|---:|---:|---:|
| 12,600 | 5 (10) | 18 (53) | 7 of 12 |
| 30,600 | 3 (15) | 34.5 (52) | 9 of 12 |
| 60,300 | 1 (14) | 27.5 (54) | 12 of 12 |
| 119,700 | 1.5 (8) | 8.5 (50) | 4 of 12 |

No fixed-mouth seed passes 15 rows at any stop. The towers overshoot the
switch's own reach (32 rows over the rim, so row 127): the highest top is row
99, 28 rows past it, and 21 of the 32 sightings over 15 rows stand above row
127. So something other than the home drop raises them further. That was not
traced. A candidate: crumbs are diggable; a dug crumb is put down by the spoil
rule, which is not gated on home; and the spoil lift carries pellets up a
column.

**The number that said it worked.** Prediction 20's stop condition was lab
deliveries above the fixed mouth's on at least 9 of 12 seeds. They were higher
on 12 of 12 (median 3,204 → 8,015), and that was reported as the variant
working before anyone had looked at a frame. But a delivery is a drop made at
home, and here home was the top of the tower, so the count included the tower
being built. Most of it was churn, as in every arm: 77% of its deliveries are
matched by a pickup made at home, against 80% for the fixed mouth. What is
left, net, is higher than the fixed mouth's on 12 of 12 (2,063 cells against
604), and that is the tower. Food stacked on it stays, or is taken off its
flanks by ants that are not at home, which the counter cannot see; for a home
whose shape separates where food lies from where a picker stands, even the net
is not a measure of transport.

On the lab's outcomes that do not depend on where home is, the mound and the
fixed mouth tie on every one: births, food eaten, colony-frames and starvation
each split 6 up, 6 down. Against the default the mound's colony is smaller:
colony-frames lower on 9 of 12 seeds, and on 17 of 24 with seeds 13-24 added
(7.7M against 10.7M, p 0.064), while its deliveries are higher on 19 of 24. On
the colony bed it starved the fewest of any arm, 188 against the default's 279
(22 fewer / 2 more). The bed's food is 90 cells out, and by the last stop only
a few cells stood over the door.

Predictions:
- **19, wrong:** the bed did better than predicted.
- **20, right on its letter and wrong on what it meant.**
- **21, right:** the census's cover over the mouth, a median of 21 cells at
  30,600 against 4. The census stops at the first gap, so it understates a
  tower that has ants in it.
- **22, right:** the digging box's dug nest stays within the mouth arm's
  spread (a median of 126 cells dug against 141).

**Reverted.** The switch lives in commit `6281f729` and is removed by
`e6b3537b`; to run it, check out the first and use §8's command. What it leaves
behind is `CreatureStats::pickups_at_nest` (§7).

## 7. Instruments this found or fixed

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
- **The recorded footprint is the rectangle the cut aimed at, not the cells
  it removed.** Living roots, bodies, water and, on uneven ground, a painted
  door cell above the centre column's surface stay inside it, so "N of M
  open" cannot reach M where the cut was refused. In the lab the roots are
  most of the "plants" counted in the cut at frame 6,300.
- **`labshot` censuses the founding cut in the lab box**: per stop, what
  fills the cut (ants, litter, plants, loose soil, lining, spoil, and anything
  else by name) and how many cells lie over the mouth, `OPEN` when none do.
  Silent without a cut. It exists because §19 named the lab's condition, *the
  mouth stays open to the surface*, from pictures alone.
- **Deliveries and colonies disagree in the lab.** A narrow home cuts
  deliveries on 11 or 12 of 12 seeds while food eaten and births tie the
  default, because lab ants bud anywhere and eat where they find food. Read
  the colony columns before concluding a home failed there.
- **A delivery is a drop made at home, so it moves with the size of home.**
  It also counts a crumb picked up at home and put back again. New:
  `CreatureStats::pickups_at_nest`, the pickups made on the same test and
  read before the food leaves. `deliveries - pickups_at_nest` is the food
  that came home. `labforage` prints both. Read the difference, and
  colony-frames, whenever two arms define home differently. §6 is the case
  where reading deliveries alone called a food tower a success.
- **The height of a pile over the mouth, read off the frames.** `labshot`'s
  census walks each column up from the mouth and stops at the first gap. A
  tower with ants in it has gaps, so the census reads a lower bound: 21 cells
  at 30,600 on a pile the frames put at 34.5 rows. §6's heights come from the
  saved frames instead. The surface is the median top of the ground either
  side of the mouth, and the pile is the brown cells over the mouth's columns
  above it, allowing two-row gaps.

## 8. Commands

```
# the shaft at frame 0 and after (A1)
PIXEL_PHYSICS_NEST_SHAFT=20 digbox ants=0 frames=300 stops=0,1,5,30,300 out=… crop=70,4,60,50 scale=6
# what floats, plain and painted by class (A2)
digbox ants=300 rate=8 w=200 soil=60 frames=6000 stops=6000 out=plain.png tintout=tint.png crop=20,0,160,70 scale=4
# the shape sweep (A5): seeds 1-12, arms default / NEST_SHAFT=20 [_WIDTH=4] [NEST_HOME=shaft]
digbox ants=300 rate=8 w=200 soil=60 frames=6000 stops=0,6000 seed=N
# the shape sweep with the bed's shaft: 40 ants, four stops, seeds 1-12
digbox ants=40 energy=1000 w=200 soil=60 frames=12000 stops=0,3000,6000,12000 seed=N
#   arms: default / NEST_DOOR=0|2 NEST_SHAFT=6 NEST_HOME=mouth / NEST_DOOR=0 NEST_SHAFT=6 NEST_HOME=shaft
# the colony bed, 24 seeds: the brief's command in three batches (seed0=1,9,17), then scripts/antloop.py
RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest \
  [PIXEL_PHYSICS_NEST_DOOR=2|0] [PIXEL_PHYSICS_NEST_SHAFT=6] [PIXEL_PHYSICS_NEST_HOME=mouth|shaft] \
  trailfollow mode=gap gate=shipped frames=24000 ants=20 relay=60 near=10 food=400 refill=400 stop=6000 \
  layfrom=founders arms=self gaps=90 seeds=8 seed0=S decisioncsv decisiondir=DIR
# the lab box, 12 seeds, from the repo root
RAYON_NUM_THREADS=1 [switches] labforage scenario=played_bed frames=120000 seed=N
# the lab mouth census and pictures
[switches] labshot scenario=played_bed seed=N frames=6300,12600,30600,60300,119700 out=…
# the mound (§6): at the commit that carries it, the lab box and labshot as above with
PIXEL_PHYSICS_NEST_DOOR=2 PIXEL_PHYSICS_NEST_SHAFT=6 PIXEL_PHYSICS_NEST_HOME=mound
# its pile height, off labshot's frames (512x320 a stop, the mouth at x 256..257, row 160):
#   sky = blue above red by 6+; brown = red > 90, red > green + 15, green > blue;
#   surface = median top of the ground at x 206..235 and 278..305;
#   pile = walk up x 254..261 from the surface while a row holds a brown cell, gaps of 2 allowed
```
