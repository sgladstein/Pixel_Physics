# Colony size: the nest and the colony at 40, 80 and 200 ants

*Measurement, 2026-09-28. `engine`. Lane note:
[`lanes/nest-mouth.md`](lanes/nest-mouth.md). The owner: "Also curious how
things change with 40 or 80 or 200 ants." Everything the nest lane has built
was tuned at 40 ants in `digbox` and 20 founders on the colony bed. No
prediction was written before these runs; they are descriptive.*

## 0. The answer

**A bigger colony digs more, and its nest stops reading as one nest.** At 40
ants the colony digs one compact nest with one or two entrances. At 200 it
digs a wide, riddled crust with about fifteen. The rules that hold the colony
to one entrance -- the founding shaft, the heap cue and the dig-down turn --
were tuned at 40 and do not hold it at 80 or 200.

**And on the colony bed a bigger colony starves more and breeds less per
ant**, with one home point or a strip of them alike. The food supply is not
what limits it; why each ant fetches less was not traced.

## 1. The nest (`digbox`)

The same 200-column box of soil at each size, ants trickled in at 4 a frame,
energy 1,000, no food, 24 seeds, today's ant (after #512). Medians:

| | 40 ants | 80 ants | 200 ants |
|---|---:|---:|---:|
| cells dug, frame 24,000 | 57 | 98 | 337 |
| entrances to the surface | 2 | 5 | 15.5 |
| seeds with one entrance only | 8 of 24 | 2 | 0 |
| roofed share of the dug room | 0.96 | 0.90 | 0.80 |
| width, middle half, columns | 12.5 | 17 | 39 |
| 90th-percentile depth, rows | 9 | 10 | 13 |
| nest-like against as many random walkers (score >= 0.9) | 21 of 24 | 13 | 0 |
| entrances at frame 12,000 | 1 | 3 | 10.5 |

The random-walker panel draws as many walkers as the colony has ants, so its
column compares like with like.

- **In pictures** (the median seed at each size, five stops; widths read
  off the picture): at 40 a tunnel slants down from the founding shaft
  beside one small heap. At 80 a band of shallow galleries roughly 45
  columns wide runs under the surface, with several holes through it. At
  200 the colony has hollowed a crust roughly 70 columns wide, with spires
  of spoil along it and galleries below.
- **Digging per ant hardly changes** (1.4, 1.2 and 1.7 cells an ant by frame
  24,000); where it goes does. The extra digging spreads sideways just under
  the surface, which is where
  [`nest-heap-cue-2026-09-28.md`](nest-heap-cue-2026-09-28.md) §17 found the
  heap cue holding the crust at 40 ants.

## 2. The colony (`trailfollow`, the colony bed)

The bed founds its colony along the ground, about two cells a founder, so
200 founders stand in a line about 400 cells long. With the food left 90
cells from the nest's centre the colony would stand on it (the harness
refuses). So **the food was moved out with the colony**: 90, 105, 135 and
220 cells for 20, 40, 80 and 200 founders, which leaves it about 75 cells
beyond the colony's near edge at every size. Packed lunch on, 24 seeds.

| | 20 | 40 | 80 | 200 founders |
|---|---:|---:|---:|---:|
| food taken per founder (median) | 9.2 | 6.1 | 4.7 | 4.2 |
| starved, share of every ant that lived | 13% | 28% | 42% | 52% |
| born per founder | 0.30 | 0.11 | 0.06 | 0.01 |
| alive at the end (median) | 18 | 26 | 38 | 77 |

With the door on (`PIXEL_PHYSICS_NEST_DOOR=2`), where every founder's home
is one point, the 20-founder colony does much better (3.7% starved) and the
larger ones the same as without it: 27%, 43% and 50% starved, born 0.06,
0.02 and 0.00 per founder. So the line of founders is not the whole of it.

- **The food supply is not the limit.** The pile is refilled to 400 cells
  every 400 frames, about 24,000 cells a run; the 200-founder colony took
  about 850 a seed.
- **The founding energy is not it either.** Each founder's starting bank is
  its species' `start_energy` times a paired spread averaging 1, whatever
  the colony's size (`founder_reserve`).
- Why each ant fetches less in a bigger colony was not traced. It belongs to
  the foraging lane's region, and is passed to it.

## 3. What it says

- The nest's one-entrance rules hold at 40 ants and not beyond. A bigger
  colony needs its extra digging kept below the crust, not only its
  entrances limited.
- Every colony number this lane and the foraging lane have quoted is a
  20-founder number. The storeroom (`nest-granary-2026-09-28.md` §8j) was
  measured at that size too.
