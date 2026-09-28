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

In `digbox` (40 ants, energy 1,000, no food, 12 seeds, frame 12,000), against
the shipped ant:

| | shipped | shaft + cue | seeds better |
|---|---:|---:|---:|
| openings to the surface | 10 | **3** (2–5) | 12 of 12 |
| roofed share of the dug room | 0.76 | **0.94** | 12 of 12 |
| middle-half width, columns | 37 | **14** | 12 of 12 |
| largest connected piece | 0.27 | **0.50** | 10 of 12 |
| 90th-percentile depth, rows | 10 | 9.5 | 5 of 12 |
| cells dug | 115 | 61 | |

**Against random digging, it is the first arm to beat the walkers on every
seed.** The combined panel reads a median 0.98 and a worst seed 0.94, where
the shipped ant reads 0.24 and 0.07.

In pictures, one compact entrance sits under the middle of the nest strip,
with spoil heaped round it and galleries and sloping tunnels below. The
seeds counted at 4 or 5 openings are one crater, all its holes within about
15 columns.

**The cost is half the digging by frame 12,000** (61 cells against 115). The
colony starts in one place instead of forty.

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
- **When it acts.** On a dig roll aimed at ground that is not a pellet, when
  the ant stands at the surface (curvature above -0.3), or the cell it cuts
  has no ground above it (`open_to_the_sky`).
- **What it does.** The urge is multiplied by `floor + (1 - floor) s²/(s² +
  K²)`, with `s` the pellets within 2 cells of the target.
- **What it leaves alone.** A pellet target (digging a heap out is refill
  churn), and a cut under a roof.
- **Spellings.** `on` is `K` 1.5, floor 0.1; `K,floor` sets both.

It is a data test (`needs_footing`, which only `spoil` carries), not a name
lookup. Unset, it reads nothing and takes no draw.

**Controls and guards.**
- **Floor 1.** With the floor at 1, every factor is 1 and the runs are
  identical to the shipped ant on 12 of 12 seeds. The hook changes nothing but
  the factor.
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

## 5. What it looks like

Tinted sheets, frames 3,000 / 6,000 / 9,000 / 12,000, were sent to the owner
on 2026-09-28:
- the shipped ant against the shaft and cue at floor 0, on seeds 3 and 11;
- the final arm on seeds 2, 5 and 12.

Tunnel wall is cyan, spoil orange, ants magenta.
- **Shipped:** a lined crust scraped along the whole nest strip, with
  openings all along it.
- **Shaft and cue:** one entrance under the middle of the strip, a crown of
  spoil round it, a small cluster of rooms, and one or two tunnels sloping
  away below. The ground either side is untouched.

## 6. What is next

1. **The colony bed and the lab, for the foraging lane.** The shaft changes
   founding in every game, and the cue halves early digging. Both are run to
   tell the foraging lane what moved, not to veto (the owner's ruling of
   2026-09-27). The 2026-09-26 lab runs found the dug mouth buried by the
   colony's own food, which has to be re-read on this dig.
2. **Defaults.** The owner prefers options on unless there is a good reason.
   The combination is the candidate once the beds are read.
3. **The brain input.** Per the owner's ruling, the switch has shown it works
   in the dig box. The sense is the pellet count beside the target; the
   gating is the hard part, since it must act only where a cut opens the
   ground to the sky.
4. **Deeper and bigger.** Half the digging by frame 12,000 means a smaller
   nest. A longer run says whether it catches up.

## 7. Predictions, written before each batch

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
