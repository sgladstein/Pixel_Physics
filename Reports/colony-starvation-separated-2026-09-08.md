# The colony starves with the larder full — §Z6, separated

**Diagnosis, measured, 2026-09-08. Nothing tuned; the only thing built is the
instrument that separates the two readings. `open-bugs-handoff.md` §Z6 stays
OPEN and its bar is unchanged — what this closes is the question §Z6 left
open, not §Z6.**

§Z6 censused nine 300,000-frame runs, found every shipped bed's colony dead
of starvation, and said in as many words that its own census could not tell
two very different bugs apart:

> Whether the colony *overgrazes* (eats the stand faster than it regrows
> while it lives) or *cannot reach* what is there ... are different bugs with
> different fixes, and this census cannot tell them apart.

**Both are true, and they are not alternatives — they are two stages of one
run.** A colony dies twice, of different things:

- **Frames 0–4,500: it cannot reach.** Forty-one to forty-six of the
  fifty-two founders starve while the bed holds **four to six times the
  colony's entire endowment** in food their own gut would digest. At the
  moment they die, 62–88% of that food is standing in a column no ant has
  entered.
- **Frames 4,500–300,000: it overgrazes.** A colony that survives the first
  stage does eventually walk the whole bed — and eats it to **4–14% of the
  same bed with the ants removed**, with the seed bank at zero, and starves
  with it. On seed 3 the stand is frozen at 445 cells from frame 149,820 and
  never recovers, with no ant alive from 199,740.

So the one line of §Z6 this overturns is *"the plants are not the
casualty"*. Measured against the same bed and the same seed at
`colonies=0` — the only difference being the ants — the stand is at **61–68%
of the unfed control before a single ant has died**.

**And the single mechanism under both stages is that the shipped ant has no
way to be aimed at food.** There is no food sense past the eight cells it is
touching, and not one of the twenty authored weights reads a pheromone plane,
so the trail `(Carrying, EmitB, 2.5)` lays on every laden step is written and
never read. A forager can only eat what it stumbles into, and the first thing
it stumbles into is its own doorstep.

## The reproduction, on the current head

`RAYON_NUM_THREADS=1`, release, `main` at `6d4728a4`. Ants alive at each
stop, then the whole run's births, deaths and cause of death:

| bed | seed | 50k | 100k | 150k | 200k | 250k | 300k | born | died | cause |
|---|---|---|---|---|---|---|---|---|---|---|
| default (8 herb, 1 colony of 52) | 1 | 0 | 0 | 0 | 0 | 0 | **0** | 5 | 57 | 57 starved |
| default | 2 | 19 | 39 | 23 | 8 | 13 | **3** | 215 | 264 | 263 starved, 1 aloft |
| default | 3 | 17 | 103 | 3 | 0 | 0 | **0** | 469 | 521 | 520 starved, 1 aloft |
| full (256 asked / 81 planted, 3 colonies of 52) | 1 | 50 | 58 | 7 | 2 | 0 | **0** | 367 | 523 | 521 starved, 2 aloft |
| full | 2 | 27 | 1 | 0 | 0 | 0 | **0** | 98 | 254 | 253 starved, 1 aloft |
| full | 3 | 30 | 18 | 7 | 24 | 10 | **5** | 966 | 1117 | 1100 starved, 17 aloft |

**§Z6 reproduces.** Two of six runs have any ant alive at 300,000 frames (3
and 5 animals), none has a colony, every death in every run is starvation,
and the bar — a colony alive at 300,000 frames on p90 of seeds, on both
boxes — fails on both boxes.

**It is not round twenty's doing, and that was checked rather than argued.**
The only change to the ant between §Z6's filing (`389192a9`) and this head is
`8d26d46a`'s two authored weights. Built from a worktree at **its own parent
`68515a81`**, so the two binaries differ by that commit and nothing else:

| ants alive | 4,500 | 9,000 | 50k | 100k | 150k | 200k | born over the run |
|---|---|---|---|---|---|---|---|
| seed 1, head | **6** | 4 | 0 | 0 | 0 | 0 | 5 |
| seed 1, parent | **11** | 9 | 12 | 9 | 7 | 0 | 59 |
| seed 3, head | **11** | 6 | 17 | 103 | 3 | 0 | 469 |
| seed 3, parent | **15** | 11 | 36 | 9 | 0 | 0 | 191 |

The alarm weights cost the colony roughly a third of its survivors at the
crash on both seeds tried — real, and the direction round twenty measured for
foraging (15–22%). **Both arms die by frame 200,000 on both seeds**, and the
births run 5/469 at the head against 59/191 at the parent, i.e. opposite
directions on the two seeds. Two seeds is not a sweep and the *sign* of the
birth difference is the bed's own spread; what the arm establishes is only
that §Z6 is older than round twenty and is not a regression from it.

## Stage one: forty-five founders starve in a bed holding six times their endowment

`examples/labforage` (new, this session — §"the instrument", below) censuses
what is standing that the shipped ant's own mouth would eat, priced through
`creature::diet_yield` and gated on `EAT_YIELD_THRESHOLD` — the mouth's own
predicate, called rather than re-derived — split by height above the soil, by
distance from the nest, and by whether the cell stands in a column **any ant
has ever occupied**. Default box, at the founder crash:

| seed | frame | ants | edible cells | worth at this gut | within 48 cols of the nest | never-visited column |
|---|---|---|---|---|---|---|
| 1 | 3,600 | 46 | 271 | 36,600 J | 0 (0%) | 127 (47%) |
| 1 | 4,500 | **6** | 341 | **44,640 J** | 7 (2%) | 301 (88%) |
| 2 | 4,500 | **8** | 478 | **63,120 J** | 31 (6%) | 405 (85%) |
| 3 | 4,500 | **11** | 367 | **48,821 J** | 46 (13%) | 228 (62%) |

The ledger says the same thing in four lines. Seed 1, the same bed run to
frame 4,500 and read there:

| | |
|---|---|
| endowment — 52 founders x `start_energy` 200 | **10,400 J** |
| burned — `metabolized` 4,538 + `moved` 5,520 + `synapse` 584 | **10,642 J** |
| harvested over the same 4,500 frames | **1,482 J** |
| standing in the bed, priced at their own gut | **44,640 J** |

They spend their whole endowment and replace **14%** of it, in a bed holding
thirty times what they managed to find. The burn is **0.316 J per creature
tick** over 33,692 ticks, so the founders' clock is
`200 / 0.316 x 6 = 3,800` frames — and the crash is at 3,600–4,500 on all
three seeds. Nothing is dying early or late; they are running a
four-thousand-frame clock down and not finding enough to reset it.

**Where the food is says why**, and `labshot`'s per-founder cell count says it
plant by plant. The eight founder seeds sit at columns
60/116/172/228/284/340/396/452 with the nest at 256. At **frame 900**, seed 3,
the same bed with and without the colony:

| founder | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| column | 60 | 116 | 172 | **228** | **284** | 340 | 396 | 452 |
| distance from the nest | 196 | 140 | 84 | **28** | **28** | 84 | 140 | 196 |
| cells, ants removed | 50 | 35 | 25 | 41 | 35 | 43 | 49 | 39 |
| cells, with the colony | 50 | 35 | **14** | **dead** | **19** | **dead** | 49 | 39 |

**The four founders within 84 columns of the nest are eaten or halved. The
four beyond 140 columns are digit for digit identical to the control.** Two
of eight plants are killed outright before frame 900 — and they are still
`dead` at frame 50,000. The census reads the same thing from the other side:
`16<d<=48` goes 2 → **0** at frame 900 and the near bands do not refill for
tens of thousands of frames. Over the whole run the standing larder is 2–13%
within 48 columns of the nest, against 19% of the bed's width.

### The full box is the control for this, and it was already in the matrix

The two beds differ in exactly the thing stage one is about: the default box
sows eight founders over 504 columns with one nest between two of them; the
full box sows eighty-one over the same width with three nests. Same species,
same engine, same 200 J per ant. `labforage`, seed 1:

| frame | default: ants / edible / **within 48 cols of a nest** | full: ants / edible / **within 48 cols** |
|---|---|---|
| 0 | 52 / 8 / **2** | 156 / 175 / **67** |
| 900 | 52 / 58 / **0** | 156 / 596 / **220** |
| 2,700 | 52 / 176 / **0** | 156 / 1,826 / **695** |
| 3,600 | 46 / 271 / **0** | 158 / 2,127 / **825** |
| 4,500 | **6** / 341 / 7 | **135** / 2,031 / 848 |

**12% of the default box's founders are alive at frame 4,500 against 87% of
the full box's**, and the full box has no founder crash at all — it stands at
158 animals at frame 3,600, *above* where it started. The difference is not
the animal and not the budget; it is whether there was anything to eat inside
the radius an undirected walk covers in 3,800 frames. That is stage one
stated as a control rather than argued.

The full box then dies of stage two instead, on schedule: 156 animals down to
27–65 by 50,000 frames, its larder falling from 2,127 edible cells to 1,553
and on down. **One box dies of both stages and the other of the second
alone** — which is why §Z6's two beds look like one failure and are two.

### `forage_probe` at play length, and the length control that makes it readable

§Z6 asked for `forage_probe` at 300,000 frames rather than 24,000. Run at
both, 3 seeds, 55 ants and a food pile at a known 87-cell gap:

| frames | deliveries (per seed) | deepest excursion | pooled excursion profile |
|---|---|---|---|
| 6,000 | 12 / 15 / 11 | 42 / 59 / 42 | `>=1` 4,764 `>=8` 180 `>=32` 10 `>=64` **0** |
| 24,000 | 45 / 15 / 32 | 144 / 70 / 80 | `>=1` 6,991 `>=8` 227 `>=32` 24 `>=64` **9** |
| 300,000 | 45 / 15 / 32 | 144 / 70 / 80 | `>=1` 6,991 `>=8` 227 `>=32` 24 `>=64` **9** |

**24,000 and 300,000 are identical in every column but `moves`** (29,329
against 29,452). Between frame 24,000 and frame 300,000 — 92% of a play
session — this colony makes 123 more steps, **zero** further deliveries,
trips or deeper excursions. The 6,000-frame arm is the control that says the
knob is connected and this is a finding rather than a stale binary: it gives
a different profile at every row.

So the answer to *"run it at play length"* is that **there is no play length
to run it at**: the colony is functionally dead by 24,000 frames in this
scene too, on the same endowment clock. And the shape of the profile is the
reach answer — of 6,991 excursions pooled over three seeds, **nine ever get
64 cells from the nest and one gets 128**, against food at 87.

### The obvious reach reading is wrong, and two controls say so

The natural conclusion from the height bands is that the surviving food is
out of reach because it is up a stem: at 300,000 frames **68–84% of the
standing larder is more than sixteen rows above the soil**, and the floor
band is down to 10–69 cells. It is wrong twice over.

**Ants climb.** The same census reports the highest ant head each sample, and
over a run they reach **40 / 156 / 144 rows above the soil** on the three
seeds — median of the per-sample maximum 6 / 33 / 36. Sixteen rows is not a
wall.

**And the height profile is a property of the plant, not of the colony.** The
same bed at `colonies=0` — nothing removed but the ants — reads **86 / 85 /
80% aloft and a floor band of 32 / 65 / 40 cells**, against the fed bed's 74
/ 68 / 84% and 69 / 34 / 10. On seed 1 the *fed* bed has more food on the
floor than the unfed one. A herb puts its leaves up; both beds look the same
way up, and the height bands say nothing whatever about grazing or reach.

This is the session's own worked example of `CLAUDE.md`'s worst-recurring
failure. The height split is arithmetically correct, plausible, and about the
wrong thing — and it took the *specificity* control (a bed with the mechanism
absent) to see it, not a second metric. It is kept in the report because the
refutation is the finding: **anyone reading §Z6's "cannot reach what is
there" as a height problem is reading a picture of a plant.**

## Stage two: a colony that survives eats the bed to a floor it cannot leave

The paired control is the same bed, the same seed, `colonies=0` — the only
thing removed is the ants. Living plant cells:

| frame | seed 1 ants / none | seed 2 ants / none | seed 3 ants / none |
|---|---|---|---|
| 900 | 215 / 318 — **68%** | 210 / 342 — **61%** | 206 / 317 — **65%** |
| 2,700 | 611 / 997 — 61% | 610 / 1,001 — 61% | 543 / 826 — 66% |
| 4,500 | 1,111 / 1,979 — 56% | 1,527 / 2,501 — 61% | 1,293 / 1,947 — 66% |
| 25,020 | 3,494 / 8,792 — 40% | 4,171 / 9,855 — 42% | 5,094 / 7,943 — 64% |
| 99,900 | 4,282 / 12,678 — 34% | 3,669 / 11,662 — 31% | 3,359 / 10,448 — 32% |
| 299,580 | 7,089 / 13,782 — 51% | 1,496 / 10,571 — **14%** | 437 / 11,281 — **4%** |

**Grazing bites at frame 900, before any ant has died**, and it never lets
go. Read as a sequence on seed 3, from `labforage`'s edible-cell column, the
whole 300,000 frames are one boom and one bust:

| frames | ants | edible cells | what is happening |
|---|---|---|---|
| 0–4,500 | 52 → 11 | 8 → 367 | stage one: the founders starve, the near larder is stripped |
| 4,500–45,000 | 11 → 17 | 367 → 1,708 | the survivors are too few to matter and the bed recovers |
| 45,000–108,000 | 17 → **127** | 1,708 → **178** | stage two: the colony grows into the larder and eats it |
| 108,000–150,000 | 127 → 3 | 178 → 90 | it crashes with the food; `unvisited` reaches 0 |
| 150,000–300,000 | 0 | 90 | frozen — 445 plant cells, seed bank 0, nothing alive |

There is no brake anywhere on that curve, which is the ethos' first law read
from the other side: the outcome *is* graded, and every grade of it is on the
way down.

Seed 3 is also the case §Z6's *"the stand recovers after them"* predicts and
does not get: the colony booms to 103 animals at 100k, takes the stand from
78 plants / 7,277 cells to **5 plants / 445 cells** by 150k, and dies. From
frame 199,740 there is not one ant alive — and the stand sits at **445 cells
with a seed bank of zero for the remaining 100,000 frames.** It does not
recover because there is nothing left to recover from. The unfed control on
the same seed is at 11,281 cells.

Measured in **food** rather than in biomass — `labforage`'s edible-cell count
against the same bed with the ants removed, at 300,000 frames — the same
result is sharper, because it drops the wood a colony was never going to eat:

| seed | fed | unfed | fed as a share of unfed |
|---|---|---|---|
| 1 (colony dead from ~54k) | 1,423 cells / 173,760 J | 2,525 | **56%** |
| 2 | 307 | 1,990 | **15%** |
| 3 | 90 | 1,934 | **4%** |

**Reach is not what limits a colony that gets going.** `labforage`'s column
mask over the whole run reads **504 of 504 usable columns walked** on seeds 2
and 3 — the entire bed — with `unvisited` at **0**. Only seed 1, whose colony
dies at ~54,000 frames, ends with 415 columns walked and its larder
accumulating over an empty bed. Given time, the walk does cover the bed. That
is the problem: it covers it, eats it, and the stand cannot outgrow it.

The `colonies=0` arm is also this instrument's specificity control and it
reads clean: `eats 0`, `pickups 0`, `burn 0`, `cols 0`, `unvisited` exactly
equal to `edible`. Every counter that must be zero is zero while the census
still reports the food, which is what says a null from the fed arm is about
the colony.

## The mechanism, and it is the species file

The shipped ant's genome is twenty authored weights. **Not one reads a
pheromone plane**, and there is no non-brain trail-following anywhere in
`creature.rs` — `world.pheromone_at` reaches an animal only as
`BrainInput::PheroA*` / `PheroB*` / `Alarm`, and only `Alarm` is wired.
`(Carrying, EmitB, 2.5)` lays a trail on every laden step and **no ant
follows one**.

There is also **no food sense at any distance**. `brain.rs` carries
`PreyNear`/`PreyBearing`, `KinNear`/`KinBearing` and
`ThreatNear`/`ThreatBearing` — and no `FoodNear`/`FoodBearing` at all. The
only food input in the engine is `FoodAdjacent`, the head's own eight
neighbours, driving `(FoodAdjacent, Move, -1.5)` (stop when touching food),
`(FoodAdjacent, Feed, 0.8)` and `(FoodAdjacent, Dig, 0.8)`.

So a forager's whole search is `(Bias, Move, 2.0)` against a persisting
heading: an undirected walk with a one-cell mouth. It cannot be aimed at
food, it cannot be recruited to food another ant found, and it has no
gradient to climb. This is `CLAUDE.md`'s second law with the verb intact and
nothing delivered — the ant *can* forage, and nothing in the world gives it
anything to forage *at*.

**`stamp_probe` at 300,000 frames prices what that costs**, and the margin is
the finding rather than the mean. On world terrain, seed 21 (seed 1 seats 3
of 55 founders and is not a bed):

> `bar: birth costs 1040 | upkeep 0.100/tick | best mouthful on the whole
> table 360, standing in this world 120 | best net +0.725/tick — a child
> every 1434 ticks of uninterrupted feeding`

1,434 ticks is **8,604 frames of uninterrupted feeding** for one child — and
that prices upkeep at the *idle* 0.100/tick. At the measured burn of
0.316/tick the same arithmetic gives 2,039 ticks, **12,235 frames**, against
a founder's 3,800-frame budget. The economy is solvent only for an animal
that eats nearly all the time, and an undirected walk is the mechanism that
decides how often it does.

**Its own run also says the failure is not the sealed box.** Seed 21, 28 of
55 founders seated, 300,000 frames on world terrain: **92 born, 120 died,
none alive, richest bank 0 against a bar of 1,040**. A colony on outdoor
ground dies inside a session too. (Its 12,276 `denied-no-space` births
against 92 born are a separate and already-filed limiter —
`creature-behaviour-ceiling-2026-09-05.md` found the same shape — and are not
what kills it: every death here is the larder.)

## What §Z6's three named instruments could and could not say

All three were run at 300,000 frames. **Two of them cannot answer, and the
reason is the scene rather than the counters** — worth recording, because
§Z6 named them as the instruments that would settle it:

- **`forage_probe` builds a hand-made bank** with 55 ants and a pile at a
  fixed 87-cell gap. It answers *can a colony range* — and does, well, once
  the length control above is run beside it. It cannot answer *is the shipped
  bed's food reachable*, because its bed is not the shipped bed.
- **`stamp_probe` runs outdoor terrain**, not `LabBox`. Its known limit is
  already in `instruments.md` — *"standing in the world is still not
  reachable"* — and that limit is this bug. What it does give is the price
  above, which is scene-independent.
- **`labnest` is the one that runs the shipped bed**, and its columns are
  about the *nest*: `roofed`, `packed`, `overcap`, `wettest`, `buried`,
  `blocked%`. It has no food census at all. It rules the structural reading
  out and cannot rule the larder reading in.

Nothing in `instruments.md`'s twenty-six binaries censuses the shipped bed's
larder. `windfall_probe` is closest — right scene, height bands, a
`colonies=0` sink control, a `handout=` positive control — and counts only
flower, fruit and windfall, so it cannot see **leaf**, which is the staple.
`larder_probe` bands food by distance to the nest on `predation_probe`'s
scene; `crown_census` is a material-by-height histogram on `PlantScene`. Each
is the right shape somewhere else.

## The instrument: `examples/labforage`

A standing count of food cannot separate these two findings, because food is
standing in both worlds — an overgrazed bed still holds wood, and an
unreachable larder is by definition untouched. What differs is whether the
colony was ever *there*, so the deciding column is `unvisited`: edible cells
standing in a column no ant has occupied at any point in the run, read beside
the same bed at `colonies=0`.

The mask is over **columns, not cells**, deliberately. An ant reaches from
the surface it walks on and that surface drifts upward as litter piles, so a
cell-exact mask would score a leaf one row above a track as unreached, which
is a statement about the bookkeeping rather than about the animal. The column
mask is the reading most likely to erase the finding, and stage one's finding
survives it.

`control=selftest` is the sensitivity half and this file would not be worth
citing without it — a band that is always zero and a band reporting a real
zero print the same line, and `aloft` and `unvisited` are exactly the two
that would. It plants known food at a known height and a known distance and
asserts the total, both height bands, the distance bands, the visited mask
**in both directions**, and that a pure-flesh gut stops seeing plants. It
runs in under a second.

**One reading trap in its output**: the four distance columns are *exclusive*
bands (`d<=16`, `16<d<=48`, `48<d<=128`, `>128`), not cumulative — "within 48
columns" is the sum of the first two.

## What this rules out

- **Not the birth rate**, and §Z6 was right about that: 5–966 children a run.
  Seed 1's five births are the bed's own spread rather than a floor — the
  same bed on seed 3 makes 469, and the parent commit reverses the ordering.
- **Not predation and not fighting.** `predators=0` on every run; the fight
  counters fire on the lab bed (75 attacks in the first 9,000 frames) and
  kill nothing (`kills 0` throughout, `ants eaten` 0). The only killer is the
  larder.
- **Not height.** Refuted by its own control above: ants reach 40–156 rows,
  and an unfed bed reads 80–86% aloft with nothing living in it.
- **Not the nest coming down, and not ants stuck underground.** `labnest` at
  300,000 frames on the shipped bed with `founders=8`: `roofed` holds at
  30–39 and `packed` *rises* 449 → 649 over the run, so the gallery stands
  and its lining stands; `buried` is **0 at every stop but one** (a single
  ant at frame 99,999), and `blocked%` runs 5.6–9.8%. The colony on that same
  bed goes 52 → 4 → 0. They are dead, not stuck — which settles the other
  half of the 2026-08-30 playtest report at play length.
- **Not the two boxes disagreeing.** Both die the same way and for the same
  reason; the full box's three colonies start with more standing crop and
  take longer.

## What a fix has to move, in the order the evidence puts it

Stated as what is missing rather than as a design — §Z6 says the choice of
which economy to move is the owner's, and it still is.

1. **A forager cannot be aimed at food.** No food sense past one cell, and
   nothing reads the trail already being laid on every laden step. Either
   half would give the search a gradient; today it has none, and this is the
   term with no counterweight anywhere in the ledger. It is also the only one
   of the three that is a *behaviour* rather than scene arithmetic.
2. **The bed sows its food where the colony is not, and the colony eats the
   exception first.** Eight founders over 504 columns with the nest between
   two of them guarantees the near larder is stripped inside 900 frames and
   the rest is beyond an undirected walk's reach.
3. **The founders' budget is spent on the bed's own start-up transient.** At
   frame 0 the bed holds **eight edible cells against fifty-two ants** — 960 J
   against a 10,400 J endowment — and does not carry a founder's-worth of food
   per ant until about the frame they start dying. `ant.ron` sizes the
   endowment as `200 / 0.10 x 6 = 12,000` idle frames; the measured burn is
   0.316 J a tick rather than 0.10, so the real clock is **3,800 frames**.
   Both halves of that are movable and they are different fixes.

**And whatever moves, stage two is the one that decides whether it holds.**
A colony that survives stage one walks the whole bed and eats it to 4–14% of
the unfed control with the seed bank at zero. Making the founders survive
without giving the stand something the grazing cannot outrun buys a later
extinction, not a colony — which is the shape of seed 3 already.

The bar is §Z6's and is unchanged: **a colony alive at 300,000 frames on p90
of seeds, on the default box and the full box, at `RAYON_NUM_THREADS=1`.** A
default-box run costs about four minutes on one core and a full box about
twenty, so it is affordable in a lane's own gate.

## Reproducing every figure here

```text
cargo build --release --examples
RAYON_NUM_THREADS=1 ./target/release/examples/labstats   founders=8   colonies=1 frames=300000 seed=N
RAYON_NUM_THREADS=1 ./target/release/examples/labstats   founders=8   colonies=0 frames=300000 seed=N  # the unfed control
RAYON_NUM_THREADS=1 ./target/release/examples/labstats   founders=256 colonies=3 frames=300000 seed=N
RAYON_NUM_THREADS=1 ./target/release/examples/labforage  founders=8   colonies=1 frames=300000 seed=N
RAYON_NUM_THREADS=1 ./target/release/examples/labforage  founders=8   colonies=0 frames=300000 seed=N
RAYON_NUM_THREADS=1 ./target/release/examples/labforage  control=selftest
RAYON_NUM_THREADS=1 ./target/release/examples/forage_probe frames=300000 seeds=3   # and 24000, and 6000
RAYON_NUM_THREADS=1 ./target/release/examples/stamp_probe  frames=300000 seed=21
RAYON_NUM_THREADS=1 ./target/release/examples/labnest      frames=300000 seeds=3 founders=8
```
