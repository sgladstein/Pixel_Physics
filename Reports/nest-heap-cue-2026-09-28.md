# One entrance: a heap of spoil decides where the ground is opened

*Measurement and two switches, 2026-09-28. `engine`. Lane note:
[`lanes/nest-mouth.md`](lanes/nest-mouth.md). Follows
[`nest-dig-wiring-2026-09-28.md`](nest-dig-wiring-2026-09-28.md), which left
the lane on "one mouth: ten remain along the nest strip".*

## 0. The answer

**The colony now digs one nest with one entrance area, not ten holes along
the nest strip.** Two switches, both off by default:
- the founding shaft (`PIXEL_PHYSICS_NEST_SHAFT=6`, built 2026-09-26);
- a new heap cue (`PIXEL_PHYSICS_SPOIL_CUE=5,0`): a dig that would open the
  ground to the sky needs a heap of spoil beside it.

The cue governs such a dig whether the ant stands on the surface or a tunnel
is breaking out from below.

In `digbox` (40 ants, energy 1,000, no food, 12 seeds, frame 12,000), on the
committed code, against the shipped ant:

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
- **Spellings.** `on` is `K` 1.5, floor 0.1; `K,floor` sets both.

It is a data test (`needs_footing`, which only `spoil` carries), not a name
lookup. Unset, it reads nothing and takes no draw.

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

**Lab** (`labforage`, 12 seeds, 120,000 frames): running when this was written; the result goes here.

## 7. What is next

1. **Defaults.** The owner prefers options on unless there is a good reason.
   Shaft, cue and dig down together are the candidate. The lab has to be
   read first, above all for the 2026-09-26 finding that the dug mouth gets
   buried by the colony's own food.
2. **The creep.** A heap of any age licenses an opening, so once spoil covers
   the strip the cue tells nothing apart. A fresh heap is the biology's cue.
   Pellets carry no age; giving them one is the next lever.
3. **The brain input**, per the owner's ruling. The sense is the pellet count
   beside the target, and the gate is "this cut opens the sky". Each new
   input costs 24 genome slots and moves every breeding scene, so it is
   planned with the lab lines, not slipped in.
4. **Chambers.** Dig down makes one compact body of galleries, but a sponge
   of narrow passages, not rooms. The research's rank-1 cue is contents,
   which the dig box lacks.

## 8. Predictions, written before each batch

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
| 79 | lab | births no worse than 4 / 8; ≤ 1 extra colony lost | running | |
| 80 | dig down 1.0, frame 24,000 | ≤ 4 openings, fewer than shaft + cue on ≥ 8; depth ≥ 15 | 5, fewer on 6 of 12; depth 9 | wrong |
| 81 | dig down 0.5, frame 24,000 | between the two | 6 openings, depth 11 | mostly wrong |
