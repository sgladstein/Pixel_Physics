# Brood and the dig: what a larva does to an ant's digging, and whether it matters to the one-room nest (2026-10-09)

**Status: answered; one thing built (a switch that ships off, counters that ship on).** The answer, from the code
(section 1) and four paired seeds (eight runs, section 5): a larva cannot be cut but the ground under it can, a nest
worker's face turn does cut it, and the larva drops into the pit unhurt. The dig reads a larva as *ground* in two
places, the two that disagree with every neighbouring test. The
heap cue's is inert: it changes the answer on one dig roll in 6,000. The curvature sense's is large (a larva is in reach
on 56-61% of dig rolls and decides "enclosed" on 7-9.5%), and hiding it halves the dig-down turns (7.3-8.8% of rolls to
3.8%) **and leaves the nest one room**: 8 of 8 runs at 100k and 200k frames, 7 of 8 at 300k, the exception being a run
where brood was *not* hidden. **So brood is not what holds the nest to one room**, by this route or by the lane's
earlier `BROOD=off` and `BROOD_DIG` controls, and digging is not drawn to brood (0.31x what the walls offer). What the
diggers do instead (section 5.5): they stay put, a digger's next cut being within 2 cells of its last 77% of the time,
but in a 9x9 patch, advancing a third of a cell a cut, all round the rim, trimming bumps; a tunnel would advance a cell
a cut. Four seeds a side; seeds 5-12 were started afterwards. Read section 6 before building on any of it.

The owner's question, 2026-10-09: *"see if brood or larvae on the ground will affect the dig behaviour. Can an
ant dig under a larva?"* — and, on seeing the answer, whether the way the dig's senses read a larva could be
part of why the lab's nest is **always dug as one huge room, never tunnels or several chambers**.

## 1. What the code does with a larva (read, then run)

A brood item (egg, larva, pupa) is one `Powder` cell of the `brood` material, owned by its own brood organism,
with `CellType::Seed` in `aux` (`brood::lay_egg`; `set_stage` carries `aux` across stages). It does not roll and
falls through bodies (`assets/materials/brood.ron`). Nothing about the dig was written with it in mind, so every
test the dig makes classifies it by whatever is general about it:

| test | where | a brood item is | what follows |
|---|---|---|---|
| the jaw | `jaw_can_cut` | **not cuttable**: `is_live_seed` | a roll aimed at one is spent, counted in `World::dig_diverted_seed` |
| the cut | `act` | (the cut looks at nothing resting on its target) | the floor cell under a larva is cut like any other; the larva, a powder, drops into the pit (next sweep, one cell) |
| the face turn, `workers` (the shipped default) | `dig_face_turn` | skipped (the jaw cannot take it) | a nest worker facing a larva turns to the nearest cell it can cut; **on a chamber floor, with the larva lying on it, that is the cell under the larva** |
| "is there a way down" | `way_down` | not empty, not `Creature`, not cuttable: no way down | three larvae under the head refuse the dig-down turn |
| curvature, the brain's `SurfaceCurvature` input and every enclosed test | `surface_curvature` | **solid** (only `Creature`-kind cells are skipped) | -2/24 = -0.083 a larva in the 5x5; four take a flat reading to the "enclosed" line, -0.3 |
| the heap cue's sky test | `open_to_the_sky` | **a roof** (any `Powder`/`Solid`; no owner test) | one larva in a column open to the sky lets an enclosed digger cut under it where the cue would veto |
| the nest room census (-> `Crowding` -> the dig urge) | `World::roofed_in_column` | **not room** (only empty cells count) and not a roof | each larva in a chamber is a cell of room lost |
| cover | `under_cover`, so `inside_nest` | not cover (owner test) | none |
| footing | `is_footing` | not footing (owner test); *filled* to the default drop-site test | (read, not run) a pellet may be set down on brood; the footing rule turns it to loose soil |

So the dig sees a larva as **ground, air and an obstacle in different places**. The two that read it as ground
(`surface_curvature`, `open_to_the_sky`) are the two that disagree with their neighbours: the flesh and nestmate
exclusion in `surface_curvature`'s own doc ("a crowd is not a hollow") holds for a pile of larvae as well, and
every other roof test skips an organism-owned cell.

### Run, 2026-10-09 (guards in `creature.rs`, section 7)

- **A nest worker facing a larva on the floor cuts the floor under it** (`digs_faced` 1, cut `(61, 63)` under a
  larva at `(61, 62)`); a forager facing the same larva cuts nothing and `dig_diverted_seed` moves.
- **The larva drops into the pit within one frame** of the cut under it, with the real frame loop
  (`parallel::step` then `step_active_sites`), and 406 frames later it is alive, at the pit's cell, with
  `brood_lost` 0 and its upkeep ticks having run there (bank 520.0 -> 516.9 J, identical to the no-cut control).
  The control (no cut) leaves it where it was.
- Three larvae side by side under the head: `way_down` false. One in the row: true.
- A larva lying on a bare surface cell: `open_to_the_sky` of the cell under it false (true bare).
- A 3x3 chamber: roofed void 9 -> 8 with one larva in it -> 9 with it removed.
- In an open pit with an enclosed digger and no heap, the heap cue's factor is `Some(0.0)` (the cut is vetoed);
  with one larva lying above the cut it is `None` (the cut is allowed). Curvature at the head -0.75 -> -0.83.

**A trap for the next probe.** This module's `run` steps the scheduler only, not the powder sweep. A first
version of the "does the larva fall" probe used it and read "stays put" in both arms, with the cut and without
it: identical arms across a change that must have moved something. The frame the app runs is `parallel::step`
then `World::step_active_sites`; the tests below use `real_frames`. A nest cut from loose `soil` also caves its
roof in the moment the sweep runs; the probes cut their chamber from `packedsoil`, as a lined nest is.

## 2. The switch and the counters

**`PIXEL_PHYSICS_BROOD_BLIND=off|curv|sky|on`** (`curv,sky` is `on`), **off** unless set; `World::brood_blind` for
one world (`creature::BroodBlind`). `curv`: `surface_curvature` leaves brood out of its solid count, which is the
brain's `SurfaceCurvature` input and every enclosed test (the dig-down turn, the heap cue's stand-aside, the dig-face
turn's cue check). `sky`: `open_to_the_sky` walks on past a brood item, so a larva is no roof. A brood item is
`is_brood_cell`: organism-owned `Powder` whose owner carries a brood record, the test `is_partable`'s brood arm makes;
a plant's seed (also an owned powder) still counts as ground. Off, both senses are what they always were, bit for bit.

It ships off because it is an ablation: built to find out whether the unplanned pull in section 1 has anything to do
with the one-room nest, not as a fix.

**Counters, always on** (pure reads: no draw, no write to a cell; the same whichever way the switch is set, so they
say what the switch *touches*, not what it did). In `CreatureStats` and the last columns of `deeptrace`'s
`stats.csv`:

| column | counts |
|---|---|
| `dig_rolls_near_brood` | dig rolls won with a brood item in the digger's head disc (the cells `surface_curvature` counts) |
| `dig_enclosed_flips` | ...of those, rolls where the brood moved the "enclosed" test (-0.3) one way or the other |
| `dig_sky_flips` | rolls that could cut where `open_to_the_sky` says something else about the target's column or the digger's own once a larva stops being a roof |
| `cuts_under_brood` | cuts with a brood item directly above the cut cell (the larva drops into the hole) |
| `cuts_near_brood` | cuts with a brood item anywhere in the 5x5 round the cut cell (those under included) |

and, so they can be read as shares, `dig_rolls`, `digs_aimed_down`, `digs_faced`, `digs_down_refused`,
`spoil_cue_applied`, `spoil_cue_kept_milli`, `digs_refused_roof`, which already existed. `cuts.csv` already carried
`brood_d` (distance to the nearest brood, Chebyshev, up to 15) and `brood5` (brood within 5) for every cut; the two
instruments agree on how many cuts have a larva within two cells (section 4), which is the check that neither is
counting something else.

## 3. With the switch off this is `main`, and it costs nothing

The counters read the world and write only into their own fields; the switch is one `Option` on the world and an
environment variable read once. That is a claim about the code, and this is the check of it, the way the lane's
baseline protocol asks (`scripts/deeptrace_tools/identity.py`).

**Identity.** The unmodified tree (`main` at `43522586`) built into one `deeptrace` binary and this tree into
another; the same `nest_goal` run on each, seed 1, 300,000 frames, the switch unset. `stats.csv` is the same in
every one of its 295 rows and 83 shared columns (`identity.py`: 0 cells differ; the 12 columns the counters add exist
only in the new file), and the 137 other files the run writes (the maps, `cuts.csv`, `colony.csv`, the event log)
are byte for byte the same.

**Cost.** One 60,000-frame run each, seed 1: 111.3 s of user CPU on the unmodified binary and 111.2 s on this one
(wall 123 s and 125 s). The census is one 5x5 read on a won dig roll and another on a cut, and the curvature and the
sky walk each work out the sighted answer and the blind one in the pass they already made.

## 4. What the senses see: the census

With the switch off, the counters say how often a larva is the deciding thing. Shares of won dig rolls and of cuts,
300,000 frames, four seeds, the `base` arm:

**Brood in the dig's senses, `base` (shares of won dig rolls and of cuts)**

| | seed 1 | seed 2 | seed 3 | seed 4 | median |
|---|---|---|---|---|---|
| won dig rolls | 2429706 | 2454795 | 2622545 | 2439648 | 2447222 |
| % with a larva in the head's 5x5 | 61.4 | 57.4 | 56.3 | 60.3 | 58.9 |
| % where brood decides 'enclosed' | 8.4 | 7.2 | 7.2 | 9.5 | 7.8 |
| % where brood changes the sky test | 0.013 | 0.015 | 0.014 | 0.019 | 0.014 |
| % that turn the jaw down (dig-down) | 7.3 | 7.4 | 7.3 | 8.8 | 7.3 |
| cuts | 69072 | 83250 | 87774 | 90621 | 85512 |
| % of cuts directly under a larva | 0.165 | 0.168 | 0.187 | 0.148 | 0.167 |
| % of cuts within 2 cells of brood | 4.5 | 3.8 | 3.8 | 3.1 | 3.8 |

**The same shares with `blind`**

| | seed 1 | seed 2 | seed 3 | seed 4 | median |
|---|---|---|---|---|---|
| % of won rolls that turn down | 3.8 | 3.8 | 3.8 | 3.8 | 3.8 |
| % where brood would decide 'enclosed' | 6.4 | 8.4 | 8.6 | 8.8 | 8.5 |

A larva is inside the head's 5x5 on 56-61% of won dig rolls: the colony's brood lies in a column down the middle of
the nest and the diggers work all round it (the picture in section 5.2). It tips the "enclosed" test (curvature at or
under -0.3) on 7-9.5% of rolls, which is nearly the whole of the dig-down turn's share (7.3-8.8%). It changes the heap
cue's sky answer on 0.013-0.019% of rolls, one in about 6,000: 324-456 rolls in 2.4-2.6 million. And a cut is
directly under a larva 0.15-0.19% of the time.

The two instruments for "a cut with brood close by", `stats.csv`'s counter (read in `act`) and `cuts.csv`'s
`brood_d` (read by `deeptrace`), agree to within 12 cuts in about 3,000 on every run (3,088 and 3,084; 3,125 and
3,124; 3,331 and 3,336; 2,808 and 2,813 on `base`). I did not chase the remainder.

## 5. The paired runs

### 5.1 Setup

`deeptrace scenario=nest_goal seed=S foodgap=90 frames=300000 founder=evolved ants=0 dig=1` on the nest-race
lane's standard stack (`Reports/handoff/nest-race/README.md`: `NEEDS_FIRST=on,backfill`, `CARRY_HOME=on`,
`DOOR_COLUMN=on`, `LAY_BAR=body`, `NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible`, `WAY_FOOT=on`), seeds 1-4.
Two arms from one pinned binary, paired by seed: `base` (switch unset) and `blind`
(`PIXEL_PHYSICS_BROOD_BLIND=on`, both senses). The numbers are read by `scripts/deeptrace_pairs.py`
(`tables`, `enrich`, `diggers`; `--selftest` builds a tunnel, a patch and a bank of soil and checks it can tell them
apart). **Four seeds a side.** Seeds 5-12 were started afterwards, to the lane's twelve, and are not in these tables.

### 5.2 The nest: one room, with or without brood in the senses

**Nest shape and colony at 300,000 frames, `base` -> `blind`**

| | seed 1 | seed 2 | seed 3 | seed 4 | median change | blind higher / lower |
|---|---|---|---|---|---|---|
| rooms of 30+ cells | 1 -> 1 | 1 -> 1 | 1 -> 1 | 2 -> 1 | 0 | 0 / 1 |
| ...of them separate (a passage away) | 0 -> 0 | 0 -> 0 | 0 -> 0 | 1 -> 0 | 0 | 0 / 1 |
| biggest room, cells | 4170 -> 4338 | 4137 -> 3374 | 4203 -> 3875 | 3788 -> 3679 | -218 | 1 / 3 |
| open cells under the ground line | 4504 -> 4669 | 4546 -> 3814 | 4609 -> 4244 | 4310 -> 4096 | -290 | 1 / 3 |
| deepest open row | 71 -> 70 | 76 -> 76 | 75 -> 74 | 76 -> 70 | -1 | 0 / 3 |
| width of the open region | 105 -> 102 | 105 -> 107 | 107 -> 107 | 105 -> 100 | -1.5 | 1 / 2 |
| ants alive | 729 -> 726 | 782 -> 771 | 778 -> 695 | 727 -> 738 | -7 | 1 / 3 |
| births | 4856 -> 4925 | 4727 -> 4927 | 4963 -> 4109 | 4818 -> 4767 | 9 | 2 / 2 |
| larvae starved | 1097 -> 391 | 1011 -> 884 | 880 -> 839 | 1095 -> 797 | -212 | 0 / 4 |
| adults starved | 26 -> 25 | 68 -> 31 | 21 -> 51 | 35 -> 25 | -5.5 | 1 / 3 |
| cuts (engine digs) | 69072 -> 57198 | 83250 -> 81474 | 87774 -> 64607 | 90621 -> 76934 | -12780 | 0 / 4 |

![The nest at 300,000 frames, seeds 1-4 left to right; top row `base`, bottom row `blind`](img/brood-and-the-dig/nest-300k-seeds1-4.png)

*Frame 300,000. The pale column down the middle of every nest is the brood: eggs are laid at the door and fall.
Yellow is stored food, red are adults, and the paler brown rim round each room is the packed wall the diggers tamp.*

One room of 30+ cells on every run at 100k and at 200k (8 of 8 each time), and on 7 of 8 at 300k; the eighth, `base`
seed 4, has a second room a passage away, and the `blind` run of that seed has not. The owner's chamber rule
(`scripts/deeptrace_tools/chambers.py`: radius-4 cores, spec-shaped when 8-16 tall and wider than tall) reads one
chamber on 23 of the 24 maps and no spec-shaped chamber on any; the one second core is a 7x8 in `blind` seed 4 at
200k, gone by 300k. Where the arms differ the difference is small and not one-sided: the biggest room is 218 cells
smaller at the median (3 of 4 lower, 1 higher, against a spread inside `base` alone of 3,788-4,203), the width 1.5
columns less, the depth one row.

**Depth is clipped by the box.** Row 80 under the ground line is hard rock, and the nests stop at row 76-77 (on the
four maps I looked at, `base` 2 and 4 and `blind` 2 and 3, row 80 is rock and there is no open cell below 76); three of
the eight reach 76 by 300k. So "deepest open row" at 300k is partly the box and not the dig. At 100k and 200k, where it
is still rising (64, 67, 71 on `base` seed 1), it does not differ between arms (the median change is -5 rows at 100k and
-2 at 200k, with `blind` lower on 3 of 4 and on 2 of 4).

### 5.3 What hiding brood did to the dig and the colony

The dig-down turn halves: 7.3, 7.4, 7.3 and 8.8% of won rolls with the brood in the sense, **3.8% on every seed** (3.76-3.83)
with it hidden. About half the dig-down turns in the unmodified colony are a larva being counted as ground.
Cuts (the engine's digs) are lower on 4 of 4: 69,072 to 57,198, 83,250 to 81,474, 87,774 to 64,607 and 90,621 to
76,934 (median -12,780, about -15%), and the open space under the ground line is 290 cells smaller at the median
(3 of 4 lower). The colony is a mixed reading: ants alive at 300k lower on 3 of 4 (by 3, 11 and 83) and higher
on one (by 11); births higher on 2 and lower on 2; adults starved lower on 3 of 4;
**larvae starved lower on 4 of 4** (1,097 to 391, 1,011 to 884, 880 to 839, 1,095 to 797). That last one I cannot
explain. Fewer eggs were laid on 3 of 4 (6,493 to 5,743; 6,340 to 6,378; 6,363 to 5,437; 6,450 to 6,025), but per egg
laid the starved share still falls on 3 of 4 (16.9 to 6.8%, 15.9 to 13.9%, 13.8 to 15.4%, 17.0 to 13.2%) so it is not only
that.

### 5.4 Is digging drawn to brood?

That is what would make brood the thing that holds the digging in one place, and it is not. Of the cuttable wall that
touches open space under the old ground line, 6-16% lies within 2 cells of brood at any map; the nest cuts made in the
25,000 frames round that map are within 2 cells of brood 0.9-10.3% of the time. The ratio (`scripts/deeptrace_pairs.py
enrich`) is **0.31 at the median within 2 cells** (0.11-0.78 over 20 seed-and-map points, none above 1.0) and 0.38
within 5. With the switch on it is 0.33 and 0.41, two points above 1.0 (the highest 1.21). Cuts keep away from brood by
about a factor of three. I have not tested why; the brood is a column in the middle of the room and the walls are
at the edge, which would do it.

### 5.5 What the diggers do

Cuts made in the nest after 100,000 frames, range over the four seeds (`scripts/deeptrace_pairs.py diggers ... --md`).
A *bout* is one digger's run of cuts, each within 5 cells and 500 frames of the one before; the table counts bouts of
8 cuts or more.

| | `base`: range over 4 seeds | `blind`: range over 4 seeds |
|---|---|---|
| nest cuts | 48121 to 62400 | 33854 to 53800 |
| diggers that cut | 1380 to 1677 | 1311 to 1990 |
| cuts per digger: median | 4 to 14 | 2 to 9 |
| cuts per digger: mean | 31 to 45 | 17 to 38 |
| cuts per digger: the most | 360 to 522 | 282 to 657 |
| a digger's next cut within 2 cells of its last, % | 77 to 79 | 73 to 77 |
| ...within 5 cells, % | 93 to 94 | 89 to 94 |
| share of nest cuts that sit in a bout, % | 79 to 85 | 68 to 83 |
| a bout: cuts in it, median | 18 to 22 | 16 to 20 |
| a bout: cells from its first cut to its last, median | 6 to 7 | 6 |
| a bout: cells advanced per cut, median | 0.25 to 0.33 | 0.30 to 0.33 |
| a bout: bounding box of its cuts, width, median | 9 | 8 to 9 |
| a bout: bounding box of its cuts, height, median | 8 to 9 | 8 to 9 |
| cut: columns from the door, median | 30 to 33 | 31 to 38 |
| open cell of the nest at the reference map: columns from the door, median | 14 to 15 | 14 to 15 |
| cuts farther out than that median, % | 84 to 92 | 79 to 90 |
| cuts of a cell below the digger's head, % | 92 | 92 to 93 |
| cuts that TRIM a bump (5+ of the cell's 8 neighbours open), % | 72 to 80 | 64 to 80 |
| cuts that BORE (3 or fewer open), % | 11 to 15 | 11 to 21 |

Neither arm changes this. It says three things.

- **A digger stays where it is.** Its next cut is within 2 cells of its last 77-79% of the time (`base`), within 5 cells
  93-94%, and a few diggers do most of the work: a median of 4-14 cuts a digger, a mean of 31-45, the most 522. (The
  lane's drained-bed trace of `DIG_NARROW`, seed 1, 100k, 878 cuts, counted *advancing* cuts only and found 92 of 432
  within 2 cells of the same ant's last, with 282 ants digging a median of 3 cuts each. That is a narrower population on
  another bed and arm, and I have not reproduced its definition; the all-cuts figure over the same kind of early
  stretch here, 50k-100k, is 72-76% within 2 cells and 2-4 cuts a digger at the median.)
- **A bout is a patch, not a line.** 79-85% of nest cuts sit in a bout of 8 or more, and the cuts of a bout fit a 9x9
  box: the digger gets about 6 cells from its first cut to its last in a median of 18-22 cuts, 0.25-0.33 cells of
  advance per cut. A tunnel advances about one cell per cut in a box one cell wide. The constructed tunnel in
  `--selftest` reads 0.97 and 40x1; the constructed patch 0.07 and 3x3. The same picture comes out with other
  thresholds: `--reach 3 --gap 200` gives 0.26-0.33 in boxes 7-8 cells across, `--reach 8 --gap 1000 --min 5` gives
  0.28-0.38 in boxes 8-9 across.
- **The patches are on the rim, and they trim.** The median cut is 30-33 columns from the door (31-38 in `blind`)
  while the median open cell of the nest is 14-15, and 84-92% of cuts are farther out than that median; 92% of cuts take
  a cell below the digger's head; and 72-80% **trim** a cell that already has 5 or more of its 8 neighbours open (a bump or
  a spike sticking into the room) against 11-15% that **bore** (3 or fewer open: a flat wall or a tunnel's end). Cutting
  bumps off makes a room rounder and larger. It does not start a passage.

**Is it the face turn?** The nest workers' dig-face turn (shipped on 2026-10-03) rotates a digger that faces open air to
the nearest cell it can cut. Four `DIG_FACE=off` runs were started and a worker
restart stopped them at 105k-140k frames, so they are compared over the frames all of them reached, 50k-100k, with
`base` and `blind` over the same frames:

| | `base`: frames 50k-100k, range over 4 seeds | `blind`: frames 50k-100k, range over 4 seeds | `dface`: frames 50k-100k, range over 4 seeds |
|---|---|---|---|
| a digger's next cut within 2 cells of its last, % | 72 to 76 | 69 to 78 | 71 to 75 |
| share of nest cuts that sit in a bout, % | 60 to 74 | 59 to 77 | 56 to 64 |
| a bout: cells advanced per cut, median | 0.30 to 0.33 | 0.25 to 0.36 | 0.29 to 0.33 |
| a bout: bounding box of its cuts, width, median | 7 to 8 | 6 to 8 | 6 to 7 |
| a bout: bounding box of its cuts, height, median | 7 to 8 | 6 to 8 | 6 to 7 |
| cuts of a cell below the digger's head, % | 91 to 92 | 92 to 93 | 90 to 91 |
| cuts that TRIM a bump (5+ of the cell's 8 neighbours open), % | 63 to 72 | 58 to 78 | 53 to 61 |
| cuts that BORE (3 or fewer open), % | 16 to 23 | 12 to 25 | 22 to 29 |

The patches do not need the face turn: the same box (6-7 cells against 6-8), the same advance per cut (0.29-0.33
against 0.25-0.36), the same 90-93% of cuts below the head. The one thing that moves is the split of what is cut, a
few more bores and fewer trims without it (22-29% and 53-61% against 12-25% and 58-78%). The nest at 100k is one room in
all four (rooms of 30+ cells 1, none separate). That repeats, shorter, what the nest-race lane measured on its drained
bed (`DIG_FACE=off`: one room on every seed, `Reports/handoff/nest-race/scoreboard.md`, "Lane 3, 01:00"), which is
why I did not run the arm again to 300k.

## 6. Reading

**Can an ant dig under a larva?** Yes, and it does: the jaw never takes a larva, but the ground under it is cut like
any other, the nest worker's face turn goes for exactly that cell when a larva lies in its way on a chamber floor, and
the larva drops into the pit and lives (section 1). It is rare: 0.15-0.19% of cuts.

**Does the way the dig reads a larva matter?** In one of the two places it does, and in neither does it reach the
shape of the nest.

- The heap cue's sky test is inert: it gives a different answer on one dig roll in 6,000.
- The curvature sense counts a larva as ground on a large share of rolls; hiding it halves the dig-down turns on every
  seed. That is a real change in what the diggers do, and it leaves the nest one room (8 of 8 runs at 100k and 200k,
  7 of 8 at 300k, the exception being a run where brood was *not* hidden).

**Is brood why the nest is one room?** Not by these routes, and not by the others the lane has tried. `BROOD=off`
(no brood at all) left one room (Lane 3, 01:00, "the brood column is not what holds the nest to one room"), and
`BROOD_DIG` (a digger beside brood cutting the wall next to it) gave one chamber on both seeds. The hypothesis that
digging gathers round brood is not supported either: cuts are about a third as likely near brood as the walls
offer. **The thought that a brood-anchored dig might be what anchors the one chamber (raised in the discussion before
this was run) is not carried by the data and should not be repeated.**

**What the data do say about the one room** is a description and not a cause. The nest is the sum of many local bays:
persistent diggers (77% of next cuts within 2 cells), each working a 9x9 patch for about twenty cuts, advancing a
third of a cell per cut, on the rim all round, trimming bumps. A tunnel needs a digger whose next cut is *further
along* than its last. Persistence is there, so what the data point at is **advance.** It is not the dig-down turn
(halving it changes nothing here) or the face turn (without it the patches are the same). Two levers from the 10-04
work went at advance. `DIG_NARROW` (advance only from inside a tunnel): no tunnel ran 10+ cells out, a fringe of 3-6-cell
stubs. `DIG_STAY` (keep the walk back to the face and stay there until the next cut): it held diggers at their faces,
60% of next cuts within 2 cells, but "the work went into widening", those faces being round the room's contents. So a
tunnel also needs somewhere to go that is not the room.

**What I would measure next, not done.** Trace the cuts of a long bout one at a time: the digger's heading, the cell it
faced, the cell it cut and why the next one was not further along (`digrows.csv.gz`, which these runs wrote to
/dev/null to save disk; `deeptrace_dig.py face` reads it). That is the `funnel` skill's question for one digger rather
than a population. If the heading does not persist from one cut to the next, the cut chooser is what turns a line into
a patch; if it does and the patch forms anyway, something is closing the line behind the digger.

**What is not established.**

- *Four seeds a side.* The nest-shape result is a result of "nothing moved"; a change of a few per cent in the size of the
  room would not show against a spread of 3,788-4,203 cells inside one arm. Seeds 5-12 are running for the lane's twelve.
- *Depth at 300k* is clipped by the bedrock (section 5.2).
- *The `sky` and `curv` halves were not run apart.* The counter says `sky` touches one roll in 6,000, so `on` is the
  curvature arm in all but name, but that is read from the counter and not from a run of its own.
- *The fewer starved larvae and the fewer cuts on 4 of 4 seeds* are unexplained (section 5.3). If anyone wants the
  switch on for the colony's sake, that is the place to start, with the funnel: every larva that starved, with the inputs.
- *The `DIG_FACE=off` runs are partial* (section 5.5).

**The switch.** It ships off, and the counters ship on. The owner's rule is that a switch that measures as a gain or as
neutral ships on. On the nest this one is neutral; on the colony it is mixed (fewer larvae starved, fewer ants alive,
fewer cuts); four seeds do not settle that, and nothing in it is wanted for its own sake. It stays off until the twelve
seeds say whether it is neutral.

## 7. Guards, red checks and gates

Eight tests in `src/sim/creature.rs`:

| guard | pins |
|---|---|
| `brood_blind_parses_its_spellings_and_ships_off` | the spellings, a typo anywhere reading the whole value as off, and the shipped value being off |
| `brood_is_ground_to_the_curvature_sense_until_the_switch_blinds_it` | four larvae in the 5x5 read -0.333, past the enclosed line; `curv` and `on` stop it; `sky` alone does not; a plant's seed counts under every setting |
| `a_larva_is_a_roof_to_the_sky_test_until_the_switch_blinds_it` | a larva over a cell is a roof unless `sky` hides it; real ground over a larva, and a plant's seed, still are under every setting |
| `the_heap_cue_stands_aside_over_a_larva_in_an_open_pit_until_the_switch_blinds_it` | five cases: the cue's factor is `Some(0.0)` with no larva, `None` with one above the cut, and `Some(0.0)` again once `sky` hides it |
| `a_cut_under_a_larva_is_counted_and_the_larva_drops_into_the_hole` | a cut under, beside and eight cells from a larva: the two counters each time, and the larva dropping into the pit within 3 frames of the real frame loop, or staying when its floor was not cut |
| `a_nest_worker_facing_a_larva_cuts_the_floor_under_it_and_a_forager_cuts_nothing` | the face turn's cut under the larva, and the forager's roll counted as one aimed at a live seed |
| `the_census_counts_brood_in_a_dig_rolls_disc_and_where_it_tips_the_enclosed_test` | `dig_rolls_near_brood` and `dig_enclosed_flips`, with the switch off and on |
| `the_census_counts_a_sky_answer_that_brood_moves` | `dig_sky_flips`: a larva over the target counts, one under real ground does not |

(`brood_item_at`, `flat_bed`, `brood_room`, `dig_once` and `real_frames` are their helpers; `real_frames` is the frame
loop the app runs, the trap in section 1.)

**Put the fault back and watch them go red.** Five test-only faults were put into one test build, each chosen by an
environment variable, the clean file saved first and restored after (`grep -c zz_fault src/sim/creature.rs` is 0):

| fault | guards red of 8 |
|---|---|
| none (the control) | 0 |
| `is_brood_cell` reads false | 7 (all but the spelling test) |
| `surface_curvature` ignores the switch | 1 (the curvature guard) |
| `open_to_the_sky` ignores the switch | 2 (the sky guard, the heap cue guard) |
| the census counts nothing | 4 (both census guards, the cut-under guard, the face-turn guard) |
| the blind sky walk stops at the first larva | 3 (the sky guard, the sky census guard, the heap cue guard) |

Every fault turns at least one guard red and the control none. (The guards' own doc comments name a few more faults
watched red by hand.) The same is true of the instrument: `deeptrace_pairs.py
--selftest` reads a constructed tunnel as 0.97 cells per cut and a constructed patch as 0.07, and a bank of soil with
cuts drawn to its larvae as enriched 13x and cuts kept away from them as 0.

**Gates, on this tree.** `cargo test --release --lib sim::creature`: 493 passed, 12 ignored. `cargo clippy
--all-targets --release --locked -- -D warnings`: clean. `bash scripts/docscheck.sh`: clean (in this container
`scripts/deadendindex.py:214`'s `utcfromtimestamp` DeprecationWarning, which Python 3.13 prints and the check counts as a
finding, needs `PYTHONWARNINGS=ignore`; it is not from this change).
