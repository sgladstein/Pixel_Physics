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

`=off` on either is the ant before it, bit for bit. **A third, dig down,
ships on in an enclosed form** (§12, §13): an ant already underground turns
down before it cuts, one on the surface does not, and none turns where there
is no way down. It stops the openings creeping back (2 against 6 at frame
24,000 over 24 seeds, more nest-like than random digging on 22 of 24 against
12) and costs the colony bed food (starved 83 -> 152, born 59 -> 30);
everywhere, it took the foragers underground (§8). Refusing the turn more
widely was measured six ways and every one lost the nest (§13). Traced
on the ant with packed lunch (§16), most of the drop in going out first
comes from where the fed foragers' episodes start when food is wanted:
inside the nest, where delivered food lies and pins them, not on the
doorstep.

The cue governs such a dig whether the ant stands on the surface or a tunnel
is breaking out from below. **It leaves alone a cut into the floor under a
roof** (§17, a correction after the foraging lane's review): a room's floor
had read as the surface. The wider fix, leaving every roofed digger alone,
opened more mouths on 17 of 24 seeds, because the misread was what kept the
ground over a gallery just under the surface whole. **A digger facing
straight up turns down through either side** (§18, the same review): it
always took the west, and the nest leaned west of the door on 24 of 24 seeds;
with a coin its galleries fan down both sides, and what lean is left is the
storeroom, which founding cuts on the west.

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
  **Superseded by §12, and the reasoning here was wrong for this lane:** its
  standing ruling (2026-09-27) is that colony numbers do not block a nest
  step -- a step is judged on the nest, and the bed and the lab are run to
  tell the foraging lane what moved. The enclosed form is the better nest
  and the smaller cost of the two, and it ships.
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

The next-steps list proposed a fresh heap as the next lever: pellets carry no age, so a heap
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

## 11. Dig down with a hunger gate: a partial rescue, and a dead end

§8's harm is ants at home turning down to dig instead of going out, so the
first candidate for keeping dig down's nest was the one the owner's route
prefers, a genome weight: a hungry ant digs less. `(Energy, Dig) = w` with
`(Bias, Dig)` moved to `-0.3 - w`, so a fed ant (`Energy` 1.0) digs exactly
as before and a hungry one `w (1 - Energy)` less. Set at runtime through
each harness's `wire=`, one binary on the final code.

**Colony bed** (24 seeds, gap 90):

| | starved | food taken from the pile | born | reached the food |
|---|---:|---:|---:|---:|
| shipped (shaft + cue) | 83 | 3,744 | 59 | 369 |
| + dig down | 295 | 1,870 | 25 | 216 |
| + dig down, gate 1 | 250 | 2,073 | 26 | 233 |
| + dig down, gate 2 | 220 | 2,171 | 22 | 266 |
| + dig down, gate 4 | 140 | 2,750 | 20 | 340 |
| gate 2, no dig down | 69 | 3,914 | 70 | 386 |

- **The gate rescues dig down in part and never in full.** At 4 it takes
  starvation 295 -> 140 (lower on 22 of 24) and food taken 1,870 -> 2,750
  (higher on 23), still short of the shipped ant on food (lower on 21 of
  24) and births (lower on 17). So hunger is only part of why the diggers
  stay home: **fed ants at home turn down and dig too.**
- **Without dig down the gate leans the right way and does not
  resolve:** starved 69 against 83 (lower on 14, higher on 9), food taken
  13 / 11.

**The dig box** (40 ants, 12 seeds, no food) prices it, and the price is the
nest. With nothing to eat an ant's bank is about a quarter of its start by
frame 12,000, so the gate stops nearly all digging: dig events at frame
12,000, median over every seed, 133 shipped, 227 with dig down, and with dig
down gated at 1, 2 and 4, 79, 23 and 9.5; the gate alone, 17.5. A colony
that is hungry stops building, which is the opposite of what a colony short
of food and shelter needs.

**Recorded as a dead end** (`dead-ends.md`, the `(Energy, Dig)` entry). The
candidate left is the one that goes at the mechanism the bed shows: the
turn down only for an ant the ground already encloses, so an ant on the
surface at home never starts a new hole downward and stays a forager.

## 12. Dig down only for an enclosed ant: the nest the lane was after, at a price

`PIXEL_PHYSICS_DIG_DOWN=1.0,enclosed` takes the turn down only when the
digger's curvature is at or below -0.3, the heap cue's own enclosure test,
so an ant on the surface at home never starts a new hole downward. Built
because the hunger gate (§11) showed fed ants at home turning down too. One
binary on the final code; the plain `1.0` spelling on it reproduces the
earlier dig-down runs exactly, on 24 of 24 bed seeds and 12 of 12 dig box
seeds.

**The dig box** (40 ants, 12 seeds):

| | shipped | + dig down | + dig down, enclosed only |
|---|---:|---:|---:|
| openings, frame 12,000 | 4 | 2.5 | **2** |
| openings, frame 24,000 | 6 | 5 | **3** |
| roofed share, frame 24,000 | 0.91 | 0.85 | **0.95** |
| middle-half width, frame 24,000 | 21.5 | 17 | **8.5** |
| largest connected piece, frame 24,000 | 0.34 | 0.42 | **0.63** |
| more nest-like than random walkers (≥ 0.9), frame 24,000 | 6 of 12 | 6 of 12 | **12 of 12** |
| cells dug, frame 24,000 | 73 | 88 | 56 |

**It stops the creep.** The pictures (seeds 3 and 8, three stops, sent to
the owner) show why. Shipped, seed 8 runs a shallow gallery sideways under
the surface for about 40 columns, spoil heaped along the top, 12 holes by
frame 24,000. With the enclosed turn the same seed keeps one entrance over
one compact shaft-and-chamber for the whole run. It digs less there -- 24
cells against 122 -- because only ants already in the shaft dig; over all
12 seeds, 56 against 73.

**The colony bed pays for it** (24 seeds, gap 90):

| | starved | food taken from the pile | born | reached the food |
|---|---:|---:|---:|---:|
| shipped | 83 | 3,744 | 59 | 369 |
| + dig down | 295 | 1,870 | 25 | 216 |
| + dig down, enclosed only | 134 | 2,953 | 22 | 360 |

Against the shipped ant: starved higher on 17 of 24, food taken lower on 22,
births lower on 19. Against dig down everywhere: starved lower on 21, food
higher on 22. With the founding shaft, home is partly inside the ground, so
an ant at home in the shaft is enclosed and still turns down.

**The lab** (`labforage`, 12 seeds, final code) ties on every pair against
the shipped ant: deliveries 5 / 7, food eaten 7 / 5, births 8 / 4, alive at
the end 7 / 3, one colony lost in each. As with dig down everywhere (§8),
the harm shows only where the food is far.

**It ships on** (`DIG_DOWN_SHIPPED`, `PIXEL_PHYSICS_DIG_DOWN=off` the ant
before), by the lane's ruling that colony numbers do not block a nest step:
on the nest it is the largest gain the lane has measured, it stops the
creep, and the lab does not see its cost. **The cost is real and is
stated, not waved off**: on the colony bed half again as many ants starve,
a fifth less food comes off the pile, and about a third as many young are
born. It is in `dig_down_bias`'s doc, the foraging lane was told before
landing, and lowering it is in §14. **§13 changes what ships**: the turn as
measured here failed three tests, and the fix is measured there.

## 13. The turn refused only where there is no way down

The suite on §12's default failed three tests, and one of them was the turn's
fault:

- **A beetle sealed in a stone pocket dug nothing** (the jaw allele's test).
  Its one cell of soil is east of its head, and the turn brought it round to
  the stone below on every roll. A turn that faces what the jaw cannot take
  costs the dig the roll would have made.
- **The decision trace saw a heading change on a tick where nothing
  happened.** The move is decided from the heading the animal had before
  `act` (`step_chain` and `chooser_step` take the tick's `heading`, not the
  state), so a turn stands only when the move roll is lost, and the trace's
  idle rows assumed nothing but the walk turns an animal. Not a defect: the
  row now records the turn (`DecisionRow::dig_turned`), the check allows
  exactly that octant, and a vacuity check asks for such a row while the
  turn is on. Recording the heading after `act` instead was tried and is
  wrong: the step's pick is relative to the heading before.
- **The store test's hungry ant did not eat** (`,keep`). Traced per tick,
  it takes one pickup, at frame 173 of 600, and a draw spent by the turn at
  frame 11 moved it out of the window; the ant rolls a dig three times in
  2,400 frames. At the shipped chance of 1.0 the turn now takes no draw.

**What to refuse was measured, not chosen.** The obvious fix, turning only
toward a cell the jaw can cut, lost most of the nest the turn was shipped
for, and so did every narrower version of it but one. The dig box, 40 ants,
paired by seed against the unrefused
turn at frame 24,000, and the colony bed, 24 seeds; the variants ran on one
scratch binary with a switch for the rule (the `""` and `open` settings
reproduced the two built arms exactly, 12 of 12 each):

| the turn is refused when it would face | seeds | openings | more nest-like than random walkers | fewer openings / more | narrower / wider | more nest-like / less | bed: starved | turns refused |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| nothing (§12) | 24 | 2 | 24 of 24 | | | | 134 | 0 |
| any cell the jaw cannot take | 24 | 3 | 19 of 24 | 4 / 15 | 4 / 18 | 4 / 14 | 98 | 48% |
| ... but open ground allowed | 12 | 3.5 | 10 of 12 | 2 / 7 | 3 / 8 | 3 / 8 | 121 | 14% |
| ... open ground and nestmates allowed | 24 | 4 | 20 of 24 | 6 / 16 | 6 / 15 | 7 / 16 | 116 | 12% |
| ... nestmates allowed | 12 | 3 | 9 of 12 | 3 / 5 | 3 / 9 | 3 / 7 | 135 | 44% |
| a cell it cannot take, when the cell ahead is one it can | 24 | 4 | 19 of 24 | 3 / 16 | 6 / 18 | 6 / 15 | 120 | 17% |
| **a floor with no way down** | 24 | 2 | 22 of 24 | 6 / 8 | 10 / 14 | 11 / 8 | 143 | 1.6% |
| the ant before the turn | 24 | 6 | 12 of 24 | | | | 83 | |

- **The unrefused turn's lead is not a lucky twelve.** It was picked because
  it measured best, so it was re-run on fresh seeds 13-24 against the refused
  turn: fewer openings on 8 of 12 again (3 more), narrower on 9 (2 wider);
  on the panel against random walkers the fresh seeds tie, 4 / 4.
- **The likely reading, not traced per ant:** a turn is one octant, so an
  ant pointing up the shaft is brought round to face down over several
  rolls, and the octants on the way face the paint at the shaft's top, a
  nestmate below or open shaft (a census of what the third rule refused,
  4 seeds: other ants 11-36 a run, nest paint 7-24, bedrock 5-21). Refused
  at any of them, the rotation stops where it is, and the digging goes
  sideways again. Keeping a cut ahead refuses exactly the ant facing a side
  wall with open ground below, which then digs the wall.
- **Every refusal that loses the nest costs the colony bed less**, which is
  the same trade §12 made: the turn's price and its nest are one mechanism.
- **What ships is the last rule** (`way_down`): refused only when all three
  cells under the animal are ground it cannot cut -- stone, bedrock, nest
  paint. It ties the unrefused turn on every nest measure over 24 seeds,
  refuses 1.6% of turns, and fixes the beetle. The bed pays what the
  unrefused turn paid: starved 143 against 134 (higher on 12 of 24, lower on
  11), born 25 against 22.

**On its final binary** (no draw at 1.0, so a new sample of the same rule):

| | the ant before the turn | the turn as it ships | paired |
|---|---:|---:|---:|
| dig box, openings at frame 24,000 (24 seeds) | 6 | **2** | fewer on 18, more on 3 |
| ... at frame 12,000 | 4 | **1** | fewer on 19, more on 2 |
| ... middle-half width | 19 | **12.5** | narrower on 21, wider on 3 |
| ... roofed share | 0.91 | **0.93** | 14 / 10 |
| ... more nest-like than random walkers | 12 of 24 | **22 of 24** | 19 / 5 |
| ... cells dug | 70 | 60 | |
| colony bed, starved (24 seeds) | 83 | **152** | higher on 18, lower on 3 |
| ... food taken from the pile | 3,744 | 3,057 | lower on 19 |
| ... young born | 59 | 30 | lower on 17 |
| lab, 12 seeds | | ties | deliveries 9 / 3, food eaten 7 / 5, births 7 / 5, alive 6 / 6; two colonies lost against one |

Against the unrefused turn on the same 24 dig-box seeds it ties (openings
8 / 11, nest-like 10 / 10; a little wider, 6 / 15), and on the bed it ties
(starved 152 against 134, 12 / 12). It refuses 1% of turns there.

**Proofs.** `PIXEL_PHYSICS_DIG_DOWN=off` on the final binary is #507's ant,
bit for bit: `digbox` 12 of 12 seeds and the colony bed 24 of 24, every log
line but those naming the switches (and the new `down_refused=` field); the
same comparison against the turned-on arm differs on 12 of 12 and 24 of 24.
The variant switch's two controls reproduced the built arms 12 of 12 each.

## 14. What is next

1. **The mouth in the lab** (§7). It is buried by what the colony brings
   home and by the planting growing over the nest ground, not by digging.
   Two readings, to put to the owner before building either:
   - the colony keeps its door clear, as real ants clear their entrances;
   - the food goes inside, into a chamber, so it stops landing on the door.
     That is chambers with contents (item 4), and the food drop belongs to
     the foraging lane, so it would be a joint step.
2. **The enclosed dig down's bed cost** (§12, §13). It ships; what is left
   is that an ant at home in the founding shaft is enclosed, turns down and
   digs when it would have gone out. The hunger gate was tried (§11) and
   fails for its own reason, and §13 shows the lever is not in *which*
   turns are taken: every refusal that lowered the cost lost the nest. So
   the lever is *which ants* are digging at home when food is wanted. The
   trace to take next: per ant, on the bed, which ants dig at home and what
   their crop, energy and the colony's need read when they do, before
   choosing a lever.
   **Traced in §16**, on the ant with packed lunch. When food is wanted,
   47% of the episodes of fed foragers at home start inside the nest with
   dig down, against 35% (episodes, not ants). From inside few go out first:
   they handle food or dig.
3. **The brain input**, per the owner's ruling. The sense is the pellet count
   beside the target, and the gate is "this cut opens the sky". Each new
   input costs 24 genome slots and moves every breeding scene, so it is
   planned with the lab lines, not slipped in.
4. **Chambers.** Dig down makes one compact body of galleries, but a sponge
   of narrow passages, not rooms. The research's rank-1 cue is contents,
   which the dig box lacks.

The creep is closed for now: a fresh heap would refuse at most a third to
under a half of it (§10).

## 15. Predictions, written before each batch

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
| 89 | bed, dig down on the final code | starved 295 exactly | 295 | right |
| 90 | bed, + gate 1 | starved between 83 and 295; food higher than dig down alone on ≥ 18 of 24 | 250; 14 of 24 | half |
| 91 | bed, + gate 2 | starved ≤ 133 | 220 | wrong |
| 92 | bed, + gate 4 | starved within ±25 of 83; food within 10% of 3,744 | 140; -27% | wrong |
| 93 | bed, gate 2 without dig down | starved within ±25 of 83 | 69 | right |
| 94 | bed, dig down enclosed only | starved ≤ 113; food within 10% of 3,744 | 134; -21% | wrong |
| 95 | dig box, enclosed, frame 12,000 | openings ≤ 3; ≥ 11 of 12 seeds nest-like | 2; 12 of 12 | right |
| 96 | dig box, enclosed, frame 24,000 | openings ≤ 5 | 3 | right |
| 97 | dig box, turn only toward a cuttable cell | 20-70% of turn chances refused | 48% | right |
| 98 | same, frame 24,000 | openings between 3 and 6, about 4 | 3 | half |
| 99 | same | nest-like on ≥ 9 of 12 | 9 | right |
| 100 | colony bed, same | starved between 83 and 134, about 100-120 | 98 | half |
| 101 | lab, same | ties | not run: superseded by §13 | -- |
| 102 | dig box, open ground allowed | within a seed of the unrefused turn; nest-like 11-12 of 12 | 10 of 12; openings 2 / 7 against it | wrong |
| 103 | colony bed, open ground allowed | starved within ±15 of 134 | 121 | right |
| 104 | the rule as a switch | `""` and `open` reproduce the built arms | 12 of 12 each | right |
| 105 | dig box, open ground and nestmates allowed | nest-like ≥ 11 of 12, no pair worse than 5 / 7 | 10 of 12; openings 3 / 7 | wrong |
| 106 | dig box, nestmates allowed | between the refused turn and the unrefused | ties the refused turn | wrong |
| 107 | colony bed, open ground and nestmates allowed | starved within ±15 of 134 | 116 | wrong |
| 108 | fresh seeds 13-24 | the unrefused turn beats the refused on the panel on ≥ 8 of 12 | 4 / 4 (openings 8 / 3, width 9 / 2) | wrong |
| 109 | dig box, keep a cut ahead | ties the unrefused turn, no pair worse than 9 / 15 | openings 3 / 16, roofed 2 / 22 | wrong |
| 110 | colony bed, keep a cut ahead | starved within ±15 of 134 | 120 | right |
| 111 | keep a cut ahead | under 10% of turn chances refused | 17% | wrong |
| 112 | dig box, no way down | ties the unrefused turn, no pair worse than 9 / 15; nest-like ≥ 22 of 24 | worst pair 10 / 14; 22 of 24 | right |
| 113 | no way down | under 5% refused | 1.6% | right |
| 114 | colony bed, no way down | starved within ±15 of 134 | 143 | right |
| 125 | colony bed, packed lunch on, dig down on / off | reproduce the foraging lane's factorial exactly | 4,574 / 93 / 151 and 5,377 / 64 / 185 | right |
| 126 | same, the trace (§16) | dug first rises by at least 4 points | 12.0% -> 16.4% | right |
| 127 | same | "picked up at home, left with it" the largest outcome, at least 35% | 24% / 18%; the largest is picked up and emptied at home, 46-49% | wrong |
| 128 | the ant before packed lunch | "picked up at home, left with it" under 15% | 1.6% / 1.2% | right |
| 129 | packed lunch on | went out first falls by at least 4 points | 42.2% -> 34.3% | right |
| 130 | packed lunch on | more of the colony's decisions inside the nest | 27% -> 36% | right |
| 152 | dig box, the first fix (§17) | cells dug higher on ≥ 16 of 24; openings within ±1 (median); nest-like on ≥ 20 of 24 | higher on 11; 2 -> 3, more on 17; 16 of 24 | wrong |
| 153 | colony bed, the first fix | food taken within ±5% of 4,574; starved within ±15 of 93 | +1.9%; 89 | right |
| -- | floor cuts only, both beds | none written: built after the first fix measured worse | ties on both | -- |

## 16. Which fed ants dig when food is wanted

*Traced after #509 put packed lunch on (2026-09-28): the colony bed, 24
seeds, gap 90, one binary on `main`, `PIXEL_PHYSICS_DIG_DOWN=off` against the
default.* §14 item 2 asked this, and the foraging lane waits on it.

The two arms reproduce the foraging lane's factorial exactly. Without dig
down: 5,377 cells taken from the pile, 64 starved, 185 born, 8,591 J standing
at the nest. With it: 4,574, 93, 151, 7,743 J. Food taken is lower on 21 of
24 seeds.

**The trace** reads the decision CSV (scratch `digcost3.py`). An episode
starts when an ant at home is fed (energy at least 0.999), empty, and feels
the forage drive. At home means on the doorstep, or underground below the
nest's 45 columns ("inside the nest"). The episode is scored by the first
thing the ant does next. **An episode is not an ant**: the "picked up food
and emptied it at home" outcome starts the next episode within frames, so an
ant that churns food at home is counted many times, and every share below is
a share of episodes, weighted toward churners (the foraging lane's review,
2026-09-28). Not re-run per ant.

| | dig down off | on |
|---|---:|---:|
| episodes | 27,449 | 26,696 |
| ... starting inside the nest | 35% | 47% |
| went out first, empty or with a packed lunch | 42.2% | 34.3% |
| dug first | 12.0% | 16.4% |
| picked up food at home and emptied it at home | 45.6% | 49.1% |

Paired by seed: dug first is higher with dig down on 18 of 24 seeds, and went
out first is lower on 18.

Split by where the episode starts:

| starts | went out first, off / on | dug first, off / on |
|---|---:|---:|
| on the doorstep | 55.6% / 53.2% | 4.6% / 5.8% |
| inside the nest | 17.4% / 13.1% | 25.7% / 28.3% |

- **Most of the drop in going out first comes from where the episodes
  start.** With dig down, 47% of the episodes start inside the nest, against
  35%. From the doorstep over half go out first; from inside, one in six to
  eight. Held at the old mix, dig down's own rates account for 1.7 of the 4.4
  points of extra digging, and 3.1 of the 7.9 points of going out less. This
  decomposes a first action, not the cost in food, which was not carried
  through.
- **Inside, a fed forager mostly handles food.** It picks up food at home and
  empties it there (57-58%), or it digs (26-28%). The food is underground:
  86-90% of those pickups happen inside the nest, mostly one to three rows
  below the surface, so delivered food lies in the top of the nest. Those
  that go out take a median 18-36 frames (p90 54-96); ants that never leave
  within the episode are not in it (censored), so this is not a time to
  leave. **Why they stay** (the foraging lane, 2026-09-28, measured on
  `main` after #510): food beside an ant pins it -- `Move`'s `FoodAdjacent`
  -1.16 -- and it steps on 3-4% of decisions beside food against 17-18%
  without, the same inside and on the doorstep; inside is where the
  delivered food lies.
- **The colony's time.** Decisions inside the nest go from 27% to 36%.
  Spoil carried inside goes from 11.8% to 16.6%, food held inside from 12.7%
  to 15.4%, and food held on the doorstep falls from 18.3% to 13.8%.
- **The ant before packed lunch has the same shape** (#507's ant against
  #508's, §13). Dug first
  goes 24.0% -> 30.1%, went out first 31.6% -> 25.7%, and episodes starting
  inside 43% -> 59%.

**What it says about a lever.** A gate on digging for a forager the colony
needs would reach the 28% who dig from inside. It would not reach the 58% who
stay to handle food, so it is not expected to send many out. The larger
effect is that a deeper nest has more of the colony inside when food is
wanted, and the food lying there pins them (above). It bears on the granary (`nest-granary-2026-09-28.md` §6): food kept
inside the nest would hold the ants who handle it, as it does here.

## 17. A cut into a room's floor, and the crust the misread was holding

*The foraging lane's review of #506, 2026-09-28: the cue judged a cut on the
floor of a wide underground room as a new mouth. Verified, fixed in a narrow
form, and measured on `digbox` (40 ants, energy 1,000, 24 seeds, frame
24,000) and on the colony bed (24 seeds, gap 90, packed lunch on).*

**Verified.** The curvature disc reads a room's floor as open ground: 0 on
the floor of a room 3 rows tall and 9 wide, 8 rows down, and -0.25 one cell
in from its wall, both above the enclosed line of -0.3. So a cut into that
floor was judged a surface dig, and with floor 0 and no heap it was refused
outright: a room 3 rows tall could not deepen.
`the_heap_cue_leaves_a_dig_inside_a_wide_room_alone` builds that room, and
it goes red on the shipped rule.

**The first fix was worse.** It counted a digger in open ground as at the
surface only with no ground over its head, so every roofed digger was left
alone unless the cut itself opened the sky:

| `digbox`, frame 24,000 (median; seeds better / worse than the shipped cue) | shipped cue | first fix | floor cuts only |
|---|---:|---:|---:|
| openings to the surface | 2 | 3 (5 / 17) | 2 (9 / 8) |
| roofed share of the dug room | 0.95 | 0.91 (5 / 18) | 0.96 (11 / 10) |
| 90th-percentile depth, rows | 10 | 8 (6 / 16) | 9 (9 / 10) |
| cells dug | 54 | 47 | 57 |
| nest-like against random walkers (≥ 0.9) | 22 of 24 | 16 | 21 |
| cuts that opened a new mouth from below | 1 | 2 (more on 16, fewer on 3) | 1 (7 / 7) |
| cells cut again (dug or filled before) | 72.5 | 102 (more on 17) | 84.5 (13 / 10) |
| dug cells refilled by a fall | 458 | 556.5 (more on 15) | 518.5 (12 / 11) |

Paired, the first fix dug no less (more on 11 seeds, less on 12); it dug in
different places.

**Traced**, with the decision logged wherever the first fix left the cue
aside and the shipped rule had applied it (six seeds: five where it opened
more mouths, one where it opened fewer; the traced runs are the measured
ones, line for line). Of 529 such decisions, **503 were by ants one or two
rows under the old surface** (25 in the heaps above it, one deeper), **471
with exactly one cell of ground over the ant's head**, and 432 cut level or
up (97 down). 226 had no pellet within reach of the target, so the shipped
cue had refused them outright; the rest it had scaled down.

**So the first fix's cuts were not in rooms.** They were ants in a gallery
just under the surface, where the digging round the mouth has hollowed the
ground enough to read as open, cutting along or up through the one row of
ground over them. The shipped cue refused those cuts away from a heap, and
that is what kept that crust whole. What each extra cut went on to do was not
traced cut by cut; the counts that moved are the ones a thinned crust moves:
breakouts from below, re-cuts, and refill by falling ground (table).

**So only the floor is exempt.** A cut below a digger with ground over its
head neither opens the sky nor thins the ground over it, and the cue now
stands aside for it as it does for a tunnel's. A roofed digger's level and
upward cuts still meet the cue. The form fires on every seed (all 24 runs
leave the shipped one by frame 6,000) and ties the shipped cue on the nest
(table). In the dig box the case the review named barely arises -- one of
the 529 traced decisions was deeper than two rows -- so its gain is for rooms
this box seldom digs, and it is shipped as a correction, not as a
measured improvement.
`the_heap_cue_still_guards_the_crust_over_a_roofed_digger` holds the other
half: an ant in a notch under the lip of an open shaft, whose cuts along and
up through the ground over it meet the cue, and whose cut into its own floor
does not. It goes red with the first fix put back; the room test goes red
with the shipped rule put back.

**The colony bed** does not move with either. The shipped arm reproduces
§16's figures exactly:

| colony bed, 24 seeds | shipped cue | first fix | floor cuts only |
|---|---:|---:|---:|
| food taken from the pile | 4,574 | 4,663 (higher on 14, lower on 10) | 4,566 (11 / 12) |
| starved | 93 | 89 | 83 |
| born | 151 | 145 | 144 |

**Found on the way: a conservation guard counting two different sets.**
The new trajectory sent `digging_moves_the_ground_rather_than_eating_it`
red at 259 -> 260 cells of ground. Traced, the two pellets lost with dying
carriers were one of spoil and one of corpse, and the identity subtracted both
as ground. Counting only the lost pellets that were ground
(`CreatureStats::spoil_lost_ground`: a material digging turns into spoil)
closes it exactly, 260 + 1 = 259 + 2. `ascii`'s dig scene counted loads and
losses the same way and is fixed the same way. The engine conserved ground
throughout. Not fixed: a lost carcass pellet takes its meat with it unbooked,
which the meat guard, an upper bound, cannot see.

Predictions 152 and 153 (§15) were written before the first fix was
measured. The floor-only form was built after it measured worse, and was run
without one. Dead end: the first fix, in `dead-ends.md`.

## 18. The review's findings 2-5: the half turn's side, the gate's test, and what the founding cut takes

*The foraging lane's review, 2026-09-28, findings 2 to 5. `main` at
`7ee0e338` against the branch; `digbox` at 40 and 200 ants, energy 1,000, 24
seeds, frame 24,000; the colony bed at 20 founders, gap 90, 24 seeds.*

**Finding 2: every enclosed digger facing straight up came down through the
west.** `turn_toward` settled a half turn one octant up `DIRS` every time,
the rule written down so the tie was not left to the reader, and still a
bias: the dig-down turn's only half-turn case is an ant facing north, so a
digger climbing a shaft always swung round through north-west, west and
south-west. The side is a coin now (`half_turn_left`), drawn from a stream
of its own (`RNG_SLOT_HALF_TURN`, keyed on the ant and the frame) so it
moves no other draw. `a_digger_facing_up_turns_down_through_either_side`
takes one digger over 32 frames and asks for both sides, each at least 8
times; it was red on the old rule and is green now.

**What it did to the nest**, paired by seed against `main` on the same
harness at frame 24,000:

| `digbox`, 24 seeds (medians; seeds higher / lower than `main`) | `main` | the coin |
|---|---:|---:|
| 40 ants: nest centre west of the door (`p50x` below 0) | **24 of 24** | 22 of 24 |
| 40 ants: nest centre, columns from the door | -11 | -8 |
| 40 ants: dug cells standing | 207.5 | 260 (20 / 4) |
| 40 ants: entrances | 6.5 | 6 |
| 40 ants: turns down booked (sum) | 16,188 | 16,829 |
| 200 ants: centre west of the door | 21 of 24 | 15 of 24 (east 7) |
| 200 ants: nest centre, columns | -7 | -3 |
| 200 ants: dug cells standing | 603.5 | 583.5 (12 / 12) |
| 200 ants: entrances | 10 | 9.5 |

The turn fires as often as before (1.04 and 0.97 of `main`'s count); only its
side changed. In pictures (card `…ce5b22`, seeds 1 and 20 at four stops)
`main`'s galleries run down and west of the door, or stay a shallow lump
under it; the coin's fan down both sides.

**The lean that is left is the storeroom.** The founding cuts the side room
on the west (away from the door, `Storeroom::SHIPPED`'s `side`), and the
founding cut alone reads `p50x` -5 at frame 6. With the storeroom off, so
the founding cut is the shaft and a centred chamber:

| `digbox`, 40 ants, `STOREROOM=off`, 24 seeds | `main` | the coin |
|---|---:|---:|
| centre west / at the door / east | 19 / 2 / 3 | 10 / 2 / 12 |
| median centre, columns | -6 | +0.5 |
| dug cells standing | 199 | 223.5 (15 / 9) |

**The colony bed** (20 founders, 24 seeds): starved 82 against 57 (fewer on
16 seeds, more on 5), born 179 against 188 (more on 10, fewer on 11). Not
traced; recorded for the foraging lane, whose baseline it moves.

**Finding 3: the enclosed-only gate had no test of its own.**
`the_dig_down_turn_is_an_enclosed_diggers_alone` gives the same forced dig
roll and the same north heading to an ant in a tunnel one row tall, 13 rows
down, and to one on the open surface: the first turns, the second does not,
and with the gate off the second turns too, which is what says the gate is
the thing tested. It passes on the shipped code, as a guard of working
behaviour should, and goes red with the gate taken out (`true ||` in front of
it): *"a digger on open ground turned down under the enclosed-only turn"*.
The first scene was a pocket 9 wide and 4 tall, and it read curvature 0 at
its floor: the ant's disc (radius 2) sees no wall it does not reach, so a
pocket that size is flat ground to it (the limit `surface_curvature`'s own
doc records).

**Finding 4: the founding cut emptied corpses.** A corpse is a `Powder` at
0.1, so the cut took it for soil and set it empty, and the worth stamped in
it left the world with no `meat_lost` booking: the meat identity stopped
closing. `is_diggable_ground` now refuses a material that carries its worth
in `aux` (`worth_in_aux`; the corpse is the only one), so the cut leaves a
corpse where it lies; it falls into the hole like any powder, where the
colony can carry it out or eat it as it would any corpse.
`the_founding_cut_leaves_a_corpse_where_it_lies` (a corpse stamped 300)
was red on the old code.

**Finding 5: a founding dug with the strongest nesting jaw.**
`founding_dig_force` took the largest authored `dig_force` over every
species that nests in `nest`, so a beetle colony (0.3, which cannot break
soil at 0.8) founded where ants are loaded dug the ants' shaft. A founding
through `found_colony_of` now cuts with its own species' jaw; a bare
`paint_nest_patch` (the lab, the druid game, `digbox` and `founding_shot`
lay a home before placing their own animals) keeps the maximum.
`a_founding_digs_with_the_founders_own_jaw` sets the ant's jaw to 0.5 with a
2.0 clone registered and never placed: red on the old code; its control, the
ant at its own 1.0, cuts on both.

**Findings 4 and 5 move nothing where one species founds on soil.** With the
turn off (`PIXEL_PHYSICS_DIG_DOWN=off`, so the coin is never asked), the
branch reproduces `main` line for line on `digbox` at 40 ants (24 of 24
seeds; only the picture's path differs) and byte for byte on the colony bed
(all three 8-seed logs). Both harnesses found only the ant, whose own jaw is
the old maximum, and no corpse lies in a founding.

**Found on the way: two streams share slot 8.** `creature.rs`'s
`RNG_SLOT_OLD_AGE` and `world.rs`'s `RNG_SLOT_NEST_SCENT` are both 8, and
`world.rs`'s doc says it took 8 because `creature.rs` had claimed 0 to 7.
The old-age roll is keyed `(seed, organism, frame, 8)` and the nest odour's
wander `(seed, site, epoch, 8)`, one epoch per 1,000 frames, so the two keys
meet only where an animal is old enough to roll at a frame no larger than the
number of epochs so far. No run this engine makes reaches that, so nothing is
shared today; it is a trap for the next change to either key, and it is
reported rather than moved here. The half turn took 9.

**Predictions**, written before their runs:

| # | arm | prediction | result | right? |
|---|---|---|---|---|
| 207 | the turn off, both binaries, `digbox` 40 ants and the bed | line for line | 24 of 24 and 3 of 3 logs | right |
| 208 | shipped, `digbox` 40 and 200 ants | turns booked within 15% of `main`'s | 1.04 and 0.97 | right |
| 209 | the same | `main` leans west on more seeds than east at 200 ants; the coin within 6 | `main` 21 / 2; the coin 15 / 7 | split |
| 210 | the same | entrances and dug space within 0.85-1.15 of `main` | entrances within; dug 0.99 at 200 ants, **1.24** at 40 | split |
| 211 | shipped, the bed | starved and born within 20% of `main` | born +5%; starved 82 -> 57 | split |
| 212 | storeroom off, `main` | west on at least 18 of 24 | 19 | right |
| 213 | storeroom off, the coin | west and east within 6 | 10 / 12 | right |
| 214 | the shipped founding cut at frame 6 | `p50x` -3 or less | -5 | right |

209 and 210 missed in the same direction: the old side did more than the
predictions allowed, and the storeroom's own position (212-214) is what 209
had not counted.
