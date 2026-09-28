# One entrance: a heap of spoil decides where the ground is opened

*Measurement and two switches, 2026-09-28. `engine`. Lane note:
[`lanes/nest-mouth.md`](lanes/nest-mouth.md). Follows
[`nest-dig-wiring-2026-09-28.md`](nest-dig-wiring-2026-09-28.md), which left
the lane on "one mouth: ten remain along the nest strip".*

## 0. The answer

**The colony now digs one nest with one entrance area, not ten holes along
the nest strip.** Two switches, **both on by default since the second PR of
this report** (§9):
- the founding shaft (`PIXEL_PHYSICS_NEST_SHAFT`, 6 rows, built 2026-09-26);
- a new heap cue (`PIXEL_PHYSICS_SPOIL_CUE`, `K` 5, floor 0): a dig that
  would open the ground to the sky needs a heap of spoil beside it.

`=off` on either is the ant before it, bit for bit. A third, dig down, makes
the nest better still and stays off, because it takes the foragers
underground (§8).

The cue governs such a dig whether the ant stands on the surface or a tunnel
is breaking out from below.

In `digbox` (40 ants, energy 1,000, no food, 12 seeds, frame 12,000), on the
committed code, against the shipped ant. **"Shipped" in this report's tables
is the ant before the shaft and the cue came on**, now spelled
`PIXEL_PHYSICS_NEST_SHAFT=off PIXEL_PHYSICS_SPOIL_CUE=off`:

| | shipped | shaft + cue | + dig down | seeds better (shaft + cue / + dig down) |
|---|---:|---:|---:|---:|
| openings to the surface | 10 | **4** (1–8) | **2.5** (1–6) | 12 / 12 of 12 |
| roofed share of the dug room | 0.76 | **0.94** | **0.94** | 12 / 11 |
| middle-half width, columns | 37 | **16** | **14** | 12 / 12 |
| largest connected piece | 0.27 | 0.50 | 0.54 | 11 / 11 |
| 90th-percentile depth, rows | 10 | 7 | 9 | 3 / 4 |
| cells dug | 115 | 54 | 70 | |
| panel vs random walkers, median / worst | 0.24 / 0.07 | 0.98 / 0.74 | **0.96 / 0.92** | |
| seeds at or above 0.9 | 0 of 12 | 10 | **12** | |

"+ dig down" adds `PIXEL_PHYSICS_DIG_DOWN=1.0`: the digger turns downward
before it cuts. With it, the colony is more nest-like than random digging on
every seed.

- **In pictures,** one compact entrance sits under the middle of the nest
  strip. Spoil is heaped round it, and galleries lie below. Where the count
  reads 4 or more, the holes are within about 15 columns of each other: one
  crater, or the porous top of one body of galleries.
- **The cost is about half the digging by frame 12,000** (54-70 cells
  against 115), because the colony starts in one place instead of forty.
- **Given twice as long** (frame 24,000), the openings creep back up: 6 with
  shaft and cue, 5 with dig down, against 11.5 shipped (§4b). The pictures
  show why. Without dig down the colony runs shallow galleries sideways under
  the whole strip and spreads spoil over it, so the heap is everywhere. With
  dig down it stays one compact body, whose top breaks through in places.
- **On the colony bed** (for the foraging lane, not a veto), shaft and cue
  cut starvation from 201 to 83 (fewer on 22 of 24 seeds) and take a
  quarter more food off the pile (§6). The lab is in §6.

## 1. Where this sits

The lane is after one mouth that every ant uses, and a nest that reads as a
nest. The dig wiring put the digging in one place; ten openings remained, and
each read as concave ground, so the curvature term fed every one alike.

**The owner's rulings of 2026-09-28** set the route:
- the footing switch stays a switch, decided together with the marker;
- the marker becomes a brain input once a switch version shows it works;
- the marker could be reopened if it was a good idea.

It was reopened as the heap, not a scent at the digging face:
- a dig-face pheromone tested negative in ants (Bruce 2015), and was never
  built in this engine;
- fresh pellets do draw where ants start digging (Pielström & Roces 2013);
- `nest-biology-digging-signals-2026-09-19.md` §3 and §8 have both.

## 2. The census that said it could work

`digbox`'s funnel now sorts every cut by where it opened and counts the spoil
within 2 cells of it (the cut-kind line; selftest 4e, watched red). The
shipped ant, 12 seeds:

| where the cut opened | share of cuts | spoil near | fresh spoil near | mean spoil cells |
|---|---:|---:|---:|---:|
| a new mouth, from the surface | 1% | 27% | 18% | 0.53 |
| a new mouth, from below | under 1% | (11 seeds) 100% | 67% | 3.3 |
| a mouth already open | 16% | 77% | 66% | 1.86 |
| below the old surface | 54% | 37% | 30% | 0.58 |
| in the heaps | 29% | 83% | 77% | 2.62 |

- **The cue has something to tell apart.** Spoil lies beside 27% of the cuts
  that open a new mouth, against 77% of those at a mouth already open.
- **It must not reach the tunnels.** Cuts below the surface are over half of
  all cuts, and only 37% have spoil beside them.
- **With the footing switch on, spoil lies beside nearly everything** (49%
  against 99%). A flag cannot separate the two there, so the cue counts.
- **About 17 new mouths open per run, and about 15 of them before frame
  3,000.** The mouths are set in the founding burst, when no heap exists yet.

## 3. The switch

`PIXEL_PHYSICS_SPOIL_CUE` (`creature::spoil_cue`, `spoil_cue_factor`):
- **When it acts.** On a won dig roll, on the cell actually cut (after any
  `DIG_DOWN` turn), when that cell is ground and not a pellet, and the cut
  would open the ground to the sky: the ant stands at the surface (curvature
  above -0.3), or the cell has no ground above it (`open_to_the_sky`).
- **What it does.** The cut goes ahead with probability `f = floor + (1 -
  floor) s²/(s² + K²)`, with `s` the pellets within 2 cells of the target.
  Overall the chance of the cut is the urge times `f`. A second draw is taken
  only while `f < 1`.
- **What it leaves alone.** A pellet target (digging a heap out is refill
  churn), and a cut under a roof.
- **Spellings.** `off` removes it; `K,floor` sets both dials. Unset and
  `on` are the shipped `K` 5, floor 0 (`SPOIL_CUE_SHIPPED`) since §9. Until
  then unset was off and `on` meant `K` 1.5, floor 0.1, the first hook's
  arm; the change keeps any spelling from naming a setting nothing ships.

It is a data test (`needs_footing`, which only `spoil` carries), not a name
lookup. Off, it reads nothing and takes no draw.

**Controls and guards.**
- **Floor 1.** With the floor at 1, every factor is 1 and the runs are
  identical to the same arm with the cue unset, on 12 of 12 seeds (checked
  on both hooks). The hook changes nothing but the factor.
- **Unit tests.** `the_heap_cue_scales_a_surface_dig_and_stands_aside_underground`
  has five arms: bare ground, a two-pellet heap, a pellet ahead, a buried
  digger, and a breakout. It was watched red twice: with the enclosure test
  removed, and again with the sky test removed.
  `the_heap_cue_parses_its_spellings_and_refuses_the_rest` covers the
  spellings.
- **Counters.** `CreatureStats::spoil_cue_applied` and `spoil_cue_kept_milli`
  are the "it fired" pair. `digbox` prints them, and echoes every spoil switch
  in its header.

## 4. The arms

`digbox`, 40 ants, 12 seeds, frame 12,000, medians. The panel is the
colony's percentile among random walkers from the door: median over seeds,
then the worst seed, then how many seeds clear 0.9.

**The rows in this first table are the order the cue was built in, and they
ran on its first hook.** That hook scaled the dig roll by the cell ahead
before the roll. Without `DIG_DOWN` that is the same probability as the
committed veto on the cell actually cut, but a different random stream, so
no row here is reproduced digit for digit by the committed code. The rows
above "breakouts governed" also predate the breakout rule. The committed
code's own figures are the second table.

| arm | openings | roofed | depth90 | width | largest | dug | panel vs walkers |
|---|---:|---:|---:|---:|---:|---:|---|
| shipped | 10 | 0.76 | 10 | 37 | 0.27 | 115 | 0.24 / 0.07, 0 of 12 |
| cue, floor 0.1 | 8 | 0.80 | 12.5 | 35.5 | 0.25 | 113 | 0.31 / 0.07, 1 |
| cue, floor 0.03 | 9 | 0.77 | 12 | 33 | 0.27 | 98 | 0.39 / 0.10, 0 |
| cue, floor 0.01 | 6 | 0.80 | 10 | 23.5 | 0.43 | 60 | 0.94 / 0.45, 7 |
| cue, floor 0.003 | 5 (6 seeds dug) | 0.69 | 10 | 19.5 | 0.45 | 42 | half the seeds never started |
| footing | 15.5 | 0.54 | 7 | 36.5 | 0.30 | 108 | 0.09 / 0.01, 0 |
| cue 0.1 + footing | 15.5 | 0.56 | 6 | 38 | 0.30 | 105 | 0.12 / 0.00, 0 |
| shaft | 12 | 0.80 | 14.5 | 35 | 0.26 | 142 | 0.12 / 0.01, 0 |
| shaft + cue, floor 0.01 | 7.5 | 0.82 | 9.5 | 23 | 0.29 | 84 | 0.59 / 0.03, 1 |
| shaft + cue, floor 0 | 4 | 0.90 | 6.5 | 13.5 | 0.44 | 54 | 0.93 / 0.35, 7 |
| shaft + cue, K 5, floor 0 | 4 | 0.88 | 6.5 | 14 | 0.52 | 50 | 0.92 / 0.61, 9 |
| **+ breakouts governed, K 5** | **3** | **0.94** | **9.5** | **14** | **0.50** | **61** | **0.98 / 0.94, 12** |
| + breakouts governed, K 1.5 | 5 | 0.89 | 7.5 | 17 | 0.36 | 52 | 0.84 / 0.54, 5 |

**What each part does:**
- **The floor is the lever on starts.** New surface mouths per run fall
  16.9, 13.6, 9.9 and 3.7 as the floor goes 0.1, 0.03, 0.01 and 0.003. At
  0.003 half the seeds never open the ground at all.
- **The shaft gives the colony a start that needs no floor.** Alone it adds a
  hole and stops none (12 openings). With the floor at 0 it is the one place
  digging begins; its spoil heaps at the lip and licenses digging there.
- **Governing breakouts halves what is left.** With the shaft and the floor
  at 0, 32 of the 66 new openings over 12 seeds were tunnels cutting the top
  of a column from below; with the rule, 16, and new openings total 56.
- **`K` barely moves the openings** (4, 5 and 4 at 1.5, 3 and 5). At 5 the
  worst seed is the most nest-like.
- **The footing switch undoes the cue.** Spoil everywhere means a heap
  everywhere. Both stay off; the owner's ruling to decide footing together
  with the marker is answered, for now, as "not with this cue".

**On the committed code** (the cue a veto on the cell actually cut, after
any `DIG_DOWN` turn), 12 seeds:

| arm | frame | openings | roofed | depth90 | width | largest | dug | panel vs walkers |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| shipped | 12,000 | 10 | 0.76 | 10 | 37 | 0.27 | 115 | 0.24 / 0.07, 0 of 12 |
| shaft | 12,000 | 12 | 0.80 | 14.5 | 35 | 0.26 | 142 | 0.12 / 0.01, 0 |
| shaft + cue 5,0 | 12,000 | 4 | 0.94 | 7 | 16 | 0.50 | 54 | 0.98 / 0.74, 10 |
| + dig down 0.5 | 12,000 | 3 | 0.94 | 10.5 | 14.5 | 0.62 | 81 | 0.99 / 0.73, 9 |
| **+ dig down 1.0** | 12,000 | **2.5** | **0.94** | 9 | **14** | 0.54 | 70 | **0.96 / 0.92, 12** |
| shipped | 24,000 | 11.5 | 0.76 | 11.5 | 39 | 0.28 | 123 | 0.17 / 0.01, 0 |
| shaft | 24,000 | 13 | 0.77 | 16 | 39 | 0.27 | 141 | 0.04 / 0.00, 0 |
| shaft + cue 5,0 | 24,000 | 6 | 0.91 | 9 | 21.5 | 0.34 | 73 | 0.87 / 0.19, 6 |
| + dig down 0.5 | 24,000 | 6 | 0.85 | 11 | 16 | 0.39 | 93 | 0.87 / 0.27, 2 |
| + dig down 1.0 | 24,000 | 5 | 0.85 | 9 | 17 | 0.42 | 88 | 0.89 / 0.56, 6 |

Two controls, both on the committed code:
- With the floor at 1, shaft and cue are identical to the shaft alone on 12
  of 12 seeds, so the veto takes no draw when it lets everything through.
- The shipped ant is identical to its runs before the fix on 12 of 12.

### 4b. Given longer, the openings creep

By frame 24,000 shaft and cue have 6 openings, not 4, and 2 to 12 by seed.
The pictures say why, and it is the footing failure again.
- **Without dig down,** the colony runs shallow galleries sideways under the
  whole nest strip and spreads its spoil along the top.
- **Once the strip is covered,** a heap lies beside every cut, and the cue
  lets the thin roof be opened anywhere.

The nest still grows, from 54 to 73 cells, and still beats the shipped ant
on openings on 10 of 12 seeds.

### 4c. Dig down, and a bug it found

`PIXEL_PHYSICS_DIG_DOWN` (built 2026-09-19) turns a digger downward before
it cuts. It is gravity in the dig, rank 2 in the biology's list.

**The first run with it was a disaster, and the disaster was mine.**
- **The numbers:** 14 openings at frame 24,000, roofed 0.75, 190 cells
  dug, and no seed more nest-like than random digging.
- **The cause:** the cue had judged the cell ahead *before* the dig-down
  turn. An ant at the surface facing along it read no ground ahead, took
  the whole urge, turned down and cut.
- **The census showed it:** new openings from the surface went 77 -> 212
  over 12 seeds, three in four with no spoil beside them, which the cue
  forbids.
- **The picture showed it too:** the crust along the whole strip, back
  again.

The cue is now a veto on the cell actually cut, after the turn.

**Judged correctly, dig down makes the nest one compact body.** The pictures
at frame 24,000 show a block of galleries about 20 columns wide and 10 to 15
rows deep under the middle of the strip, with no sideways spread. Where the
count reads 5 to 7, it is the porous top of that one body. It is a sponge of
narrow passages, not chambers.

## 5. What it looks like

Tinted sheets were sent to the owner on 2026-09-28. Tunnel wall is cyan,
spoil orange, ants magenta. The owner asked for the pictures to be checked
against the metrics as the work went, and each finding here was.
- **Frames 3,000 to 12,000, seeds 3 and 11, shipped against shaft and cue:**
  - shipped, a lined crust scraped along the whole nest strip, with
    openings all along it;
  - shaft and cue, one entrance under the middle of the strip, a crown of
    spoil round it, a small cluster of rooms, and one or two tunnels sloping
    away below, the ground either side untouched.
- **The final arm on seeds 2, 5 and 12:** the "5 openings" of seed 2 are
  one crater, within about 15 columns.
- **Frames 6,000 to 24,000, the long run (§4b):** seed 3 stays one
  entrance, while seeds 8 and 12 spread shallow galleries under the whole
  strip. That is the creep the count reported.
- **The same long run with dig down (§4c):** one compact body of galleries.
  The first dig-down pictures, before the fix, showed the crust back along
  the whole strip. That is how the bug was confirmed as a bug and not a
  verdict on digging down.

## 6. The colony bed and the lab

Run to tell the foraging lane what moved, not to veto (the owner's ruling
of 2026-09-27). Shaft and cue (`NEST_SHAFT=6`, `SPOIL_CUE=5,0`) against the
shipped ant, on the committed code, one binary.

**Colony bed** (`trailfollow`, 24 seeds, 20 founders, gap 90), paired by seed:

| | shipped | shaft + cue | shaft + cue higher / lower |
|---|---:|---:|---:|
| starved | 201 | **83** | 2 / 22 |
| food taken from the pile, cells | 3,043 | **3,744** | 19 / 5 |
| food standing at the nest, J (median, mean from frame 6,000) | 7,000 | 7,160 | 15 / 9 |
| born | 59 | 59 | 10 / 11 |

Ants that reached the food rose from 305 to 369. The pre-fix binary gave the
same picture (starved 82, taken 3,914). The founding shaft is the likely
cause: on 2026-09-27 door and shaft also halved the bed's starvation, with a
mouth that ants find (`nest-work-2026-09-27.md` §6).

**Lab** (`labforage`, 12 seeds, 120,000 frames): ties on the pairs, with the
large swings in the medians the lab always shows.

| | shipped | shaft + cue | shaft + cue higher / lower |
|---|---:|---:|---:|
| births (median) | 724 | 587 | 5 / 7 |
| food eaten, J (median) | 1.53 M | 1.23 M | 4 / 8 |
| alive at the end (median) | 66 | 19 | 7 / 5 |
| colonies extinct | 1 of 12 | 1 of 12 | |
| nest visits | | | 2 / 10 (p 0.04) |

**Looked at, on seed 11 at frame 120,000:**
- **The shipped colony** has a shallow gallery just under the surface and a
  couple of short tunnels.
- **The shaft-and-cue colony** has a network of long sloping tunnels 20-30
  rows deep. That is its roofed room, 2.5 times the shipped one: 748 cells
  against 298, with 244 ants against 116.
- **Both** run a gallery just under the surface along the width of the view,
  the same shallow spread as the dig box's long run (§4b).

Whether the dug mouth is buried by the colony's own food, the 2026-09-26
finding, needs `labshot`'s cut census and was not run.

## 7. The lab: the founding mouth is buried in every arm

`labshot` on the lab's `played_bed` (the owner's planting, grown for 6,000
frames before a colony lands), 12 seeds, 120,000 frames, one binary on the
committed code. At each stop its census reads the founding cut: how much of
it is open, and how many cells of cover lie over its mouth.

| mouth open, of 12 seeds | 6,300 | 12,600 | 30,600 | 60,300 | 119,700 |
|---|---:|---:|---:|---:|---:|
| shaft alone | 6 | 2 | **0** | 1 | 5 |
| shaft + cue | 6 | 3 | **0** | 1 | 4 |
| + dig down | 6 | 4 | **0** | 1 | 4 |

- **Buried on 12 of 12 seeds by frame 30,600, in every arm.** The
  2026-09-26 census found the same under the door and home switches (11-12 of
  12). The shaft alone buries the same way, so neither those switches nor
  these cause it.
- **Buried from above, by what the colony brings home and what grows.**
  Over every seed and stop the cover is delivered food (crumbs) and plants,
  and late on loose soil and lining. Spoil is 0 to 2 cells of it in each
  arm: the colony does not fill its mouth with its own digging.
- **The picture** (seed 7, three arms, five stops, sent to the owner): the
  grass of the lab's planting grows across the whole nest ground and over the
  mouth by frame 12,600. By 60,300 the plants have died back to stalks among
  the crumbs. Underneath, all three colonies have long sloping tunnels.
- **The colony's size at the end is a boom and a bust, not a verdict.** At
  60,300 the cue's colonies are larger than the shaft alone's on 8 of 12. At
  119,700 they are smaller on 9 (3 larger, p 0.15), five of them at 3 ants or
  fewer against none with the shaft alone. Dig down turns it round: larger
  than the cue alone on 9 of 12 (p 0.07), one at 3 or fewer. Against the
  shipped ant (`labforage`, §6) the cue's colonies ended larger on 7 of 12.

## 8. Dig down on the colony bed and the lab

Run because the owner's rule asks whether a switch that helps the nest should
ship on. `trailfollow`, 24 seeds, 20 founders, gap 90, one binary, paired by
seed:

| | shipped | shaft + cue | + dig down | dig down higher / lower than shaft + cue |
|---|---:|---:|---:|---:|
| starved | 201 | 83 | **295** | 22 / 0 |
| food taken from the pile, cells | 3,043 | 3,744 | **1,870** | 0 / 24 |
| food standing at the nest, J (median) | 7,000 | 7,160 | 5,784 | 7 / 17 |
| born | 59 | 59 | 25 | 4 / 15 |
| ants that ever reached the food | 305 | 369 | 216 | |

It is the 2026-09-27 mechanism unchanged (`dead-ends.md`'s `DIG_DOWN`
entry): an ant at home, where `Dig` runs high, turns down and digs instead of
going out. The dig wiring lowered `Dig` away from home and left it high at
home, so that entry's re-test condition is not met.

**The lab** (`labforage`, 12 seeds, 120,000 frames) does not see the harm:

| | shipped | shaft + cue | + dig down | dig down higher / lower than shipped | than shaft + cue |
|---|---:|---:|---:|---:|---:|
| food eaten, J (median) | 1.53 M | 1.23 M | 1.47 M | 6 / 6 | 9 / 3 |
| births (median) | 724 | 587 | 759 | 6 / 6 | 7 / 5 |
| alive at the end (median) | 66 | 19 | 137 | 8 / 4 | 9 / 2 (p 0.07) |
| colonies extinct | 1 of 12 | 1 of 12 | 0 of 12 | | |

In the lab box food grows all round the nest, so an ant that digs at home
is never far from a meal; on the colony bed the food is 90 cells off, and
every ant that goes down instead of out is a forager lost. The harm shows
where the food is far.

## 9. The default: the shaft and the cue on, dig down a switch

- **The rule** (owner, 2026-09-27, in `CLAUDE.md`): a switch that measures as
  a gain or as neutral ships on, and a good reason to keep one off is a
  measured harm, stated in its doc and the report.
- **The shaft and the cue: on.** They gain on the nest (every dig box
  measure; openings fewer on 12 of 12 seeds) and on the colony bed
  (starvation 201 -> 83, food taken +23%), and are neutral in the lab (§6's
  ties; the mouth buried exactly as with the shaft alone, §7).
- **Dig down: off, for the harm in §8.** Its doc (`dig_down_bias`) says so,
  and names what brings it on: the turn has to stop recruiting the foragers.
- **In the code.** `NEST_SHAFT_ROWS` (6) and `SPOIL_CUE_SHIPPED` (`K` 5,
  floor 0) are the unset values, and `off` on either is the ant before it.
  `World::spoil_cue` joins `World::nest_shaft` as a per-world override. The
  `ascii` construction scene places its ants by hand on a lattice with no
  heap, and turns the cue off beside the supply it already pins.
- **Floor 0 leans on the founding shaft.** A colony opens bare ground only
  beside a heap, so one placed without a founding cut never opens it. Every
  game founds through `found_colony_of`, which cuts one. A hand-built scene
  that wants the ant before the cue sets `World::spoil_cue`.
- **The shaft now digs only what the founders could.** Its cut took any
  solid or powder cell: stone (penetration resistance 100), gravel (3.5) and
  sand (1.4) as readily as soil, against the ant's `dig_force` of 1.0. Found
  when the flip turned `the_books_close_for_every_colony` red: on that
  test's one-row stone floor the shaft opened a hole into the void below. A
  column now opens only where the cell under the nest paint is neither empty
  nor too hard for the ant, and stops at the first ground that is
  (`founding_dig_force`); the chamber is cut only if a column reached it;
  and a founding on rock is a painted nest with no hole and no footprint. On
  soil nothing changes, which the proofs below show digit for digit.
- **What the hole had exposed was a books leak in founding itself**, older
  than the shaft. Founding books each founder's flat grant, then deals the
  colony's reserve out unevenly in pairs that cancel; it dealt over every
  planned station, so a station that could not place its founder took its
  share with it, unbooked. With the shaft, the hole moved the test bed's
  first colony one column (founders at 81-101 instead of 82-102), and the
  second colony's first station, at 102, was left without room for a
  two-cell body beside it: 82.6 J out before the first frame. Two colonies founded 8 cells apart, no shaft at all: 85.1 J out. The
  reserve is now dealt over the founders placed, and a guard written first
  was watched red on the old code (-85.09 J).

**The proofs.** One binary on the final code against the arms measured
before the flip:
- **`digbox`** (40 ants, 12 seeds, 24,000 frames): unset is the old
  `NEST_SHAFT=6 SPOIL_CUE=5,0` arm, both `=off` is the old unset ant, and
  `SPOIL_CUE=off` is the old shaft alone -- 12 of 12 each, every line of the
  log but the two that name the switches. The same diff against the wrong arm
  differs on 12 of 12, so it can fail.
- **The colony bed** (`trailfollow`, 24 seeds): unset is the old shaft and
  cue arm on all 24; against the old shipped ant it differs, as it must.
- **The lab is not bit for bit, and the founding fix is why.** The lab's
  colony plans 52 founders and places 43, plants standing on the other nine
  stations, so the reserve was dealt over 52 with nine gaps and is now dealt
  over the 43. With the shaft and the cue both off the old and new binaries
  already differ by frame 6,300. On the final code the
  flip is neutral there (`labforage`, 12 seeds, new default against `=off`):
  deliveries 6 / 6, food eaten 7 / 5, births 8 / 4, alive 5 / 6, one colony
  lost in each. The founding fix alone moves every lab trajectory and leans
  nowhere: against the old shipped runs, food eaten 4 / 8, births 4 / 8,
  alive 6 / 6.
- **`ascii`**: 31 of 31 scenes. The excavation scene, whose ants are set
  down by hand beside a bank with no founding shaft, digs 326 -> 151 and
  roofs 83 -> 51 cells of void with the cue on, and still passes its guards:
  the cue's early cost, where nothing has made a heap yet.
- **The suite**: 1,978 passed, 0 failed (lib 1,918 with 88 ignored, druid 2,
  the bin 10, `tests/determinism.rs` 4, `tests/worldgen.rs` 44 with 18
  ignored). `clippy --all-targets --release --locked -D warnings` and
  `docscheck` clean.

## 10. The creep, sized before building a fresh heap

§11 proposed a fresh heap as the next lever: pellets carry no age, so a heap
of any age licenses an opening. `digbox`'s funnel already dates every pellet
it sees put down, so the lever's reach could be read before building it. In
the window where the creep happens, frames 12,000 to 24,000, over 12 seeds:

| | new openings | spoil beside | fresh spoil beside (≤ 1,000 frames) | old spoil only |
|---|---:|---:|---:|---:|
| shaft + cue | 54 | 53 | 34 | **19 (35%)** |
| + dig down | 34 | 32 | 17 | **15 (44%)** |

- A cue that counted only fresh pellets could refuse at most the last column,
  and less if the ants then opened somewhere else: a third to under a half of
  the creep.
- The rest open beside fresh spoil, next to digging that is going on now:
  the porous top of the one body, not a new start. So pellet age is a partial
  lever at best, and it is not built.

## 11. What is next

1. **The mouth in the lab** (§7). It is buried by what the colony brings
   home and by the planting growing over the nest ground, not by digging.
   Two readings, to put to the owner before building either:
   - the colony keeps its door clear, as real ants clear their entrances;
   - the food goes inside, into a chamber, so it stops landing on the door.
     That is chambers with contents (item 4), and the food drop belongs to
     the foraging lane, so it would be a joint step.
2. **Dig down without the foraging harm** (§8). The turn has to stop
   recruiting the foragers. Two candidates, each measured first on the colony
   bed:
   - a genome weight, the owner's preferred route: a hungry ant's `Dig`
     lower, so it goes out rather than down;
   - the turn only for an ant the ground already encloses, so none starts a
     new hole from the surface at home.
3. **The brain input**, per the owner's ruling. The sense is the pellet count
   beside the target, and the gate is "this cut opens the sky". Each new
   input costs 24 genome slots and moves every breeding scene, so it is
   planned with the lab lines, not slipped in.
4. **Chambers.** Dig down makes one compact body of galleries, but a sponge
   of narrow passages, not rooms. The research's rank-1 cue is contents,
   which the dig box lacks.

The creep is closed for now: a fresh heap would refuse at most a third to
under a half of it (§10).

## 12. Predictions, written before each batch

| # | arm | prediction | result | right? |
|---|---|---|---|---|
| 55 | census | new mouths ≤ 5% of cuts | ~1.5% | right |
| 56 | census | spoil at new surface mouths ≤ half that at open mouths | 27% vs 77% | right |
| 57 | census | ≥ 50% fresh spoil at open-mouth cuts | 66% | right |
| 58 | census | breakouts ≥ 20% of new mouths | ~13% | wrong |
| 59 | census, footing | spoil near rises everywhere, gap widens | rises; gap does not widen | half |
| 60 | cue 0.1 | openings < 10 on ≥ 9 of 12 | 8 median, 8 of 12 | wrong |
| 61 | cue 0.1 | new surface mouths ≤ 8 a run | ~15 | wrong |
| 62 | cue 0.1 | roofed and depth within reach of shipped | both better | better |
| 63 | cue + footing | openings under footing alone | 15.5, the same | wrong |
| 64 | floor sweep | new mouths fall monotonically, to ≤ 10 / 6 / 4 | 13.6 / 9.9 / 3.7 | monotone, levels wrong |
| 65 | floor 0.01 | openings ≤ 6, fewer on ≥ 9 | 6, 12 of 12 | right |
| 66 | floor 0.003 | dug under 60% of shipped | half the seeds dug nothing | right |
| 67 | K 0.5 | fewer openings than K 1.5 | 10 against 9 | wrong |
| 68 | shaft alone | openings 10 ± 3 | 12 | right |
| 69 | shaft + 0.01 | ≤ 4, under floor 0.01 alone | 7.5 | wrong |
| 70 | shaft + 0.001 | ≤ 2, width ≤ 15, dug ≥ 50% | 6, 16.5, 52% | wrong |
| 71 | shaft + 0 | as 70, and every seed a nest | 4, 13.5, 47%; every seed | partly |
| 72 | shaft + K 3 | ≤ 3 | 5 | wrong |
| 73 | shaft + K 5 | ≤ 2, dug ≤ 45 | 4, 50 | wrong |
| 74 | breakout rule, K 5 | breakouts ≤ 1 a run, openings ≤ 2 | 1.3, 3 | wrong, close |
| 75 | breakout rule, K 5 | dug ≥ 40 | 61 | right |
| 76 | breakout rule, K 1.5 | openings ≤ 3 | 5 | wrong |
| 77 | colony bed | starved within ±30 of shipped | 201 -> 83, fewer on 22 of 24 | wrong: far better |
| 78 | colony bed | food taken from the pile within ±10% | +23%, higher on 19 of 24 | wrong: better |
| 79 | lab | births no worse than 4 / 8; ≤ 1 extra colony lost | 5 / 7; 1 lost either way | right |
| 80 | dig down 1.0, frame 24,000 | ≤ 4 openings, fewer than shaft + cue on ≥ 8; depth ≥ 15 | 5, fewer on 6 of 12; depth 9 | wrong |
| 81 | dig down 0.5, frame 24,000 | between the two | 6 openings, depth 11 | mostly wrong |
| 82 | lab census, shaft alone | mouth buried at 30,600 on 6-9 of 12 | 12 of 12 | wrong: worse |
| 83 | lab census, shaft + cue | open on more seeds than the shaft alone at 30,600 and 60,300 | 0 against 0, 1 against 1 | wrong |
| 84 | lab census, + dig down | open on at least as many as the cue from 30,600 on | 0, 1, 4 against 0, 1, 4 | right, as a tie |
| 85 | lab census | the cover is crumbs and plants, not spoil | spoil 0-2 cells an arm | right |
| 86 | colony bed, + dig down | starved within ±25 of shaft + cue's 83, under shipped on ≥ 20 of 24 | 295; over shipped on 19 | wrong |
| 87 | colony bed, + dig down | food taken within ±10% of shaft + cue | -50%, lower on 24 of 24 | wrong |
| 88 | lab, + dig down | ties with shipped, no split worse than 3 / 9; ≤ 2 colonies lost | 6 / 6 on food and births; none lost | right |
