# Lane note — the nest's mouth

*Kept current, edited in place. The live question is at the top; results and
predictions below it; head SHAs at the bottom.*

- **Session:** `session_01WF4wABj2ewSmzWVJTk6DsC` (the nest-mouth lane).
- **Branch:** `claude/ant-nest-mouth-4f6s79`, off `main` at `636612c6`.
- **Peer:** the foraging-loop session, `session_01AFH5xR442VuoZsXm7VzJmx`.
  It owns the walk, feeding, the crop, dropping food, `trailfollow.rs`,
  `scripts/antloop.py` and `Reports/ant-scenes-2026-09-23.md`. This lane owns
  nest founding and shape (`paint_nest_patch`, `nest_mask`,
  `dig_founding_shaft`, `colony_stations`, nest sites, `adjacent_nest` and
  their switches), digging and spoil, the nest material and the nest/dig
  examples. **Exception:** the loop session's §19 builds the narrow door and
  founder placement; this lane writes no door or founder-placement code until
  §19 lands, then builds on it.

## What this lane is for, in the world's words

1. **First: one mouth every ant goes in and out of.** Today founding paints a
   strip of nest ground ~53 cells wide along the surface and digs nothing, and
   an ant is home anywhere beside it. The road to food starts at the strip's
   food-side end, so ants born on the far half rarely meet it (ant-scenes
   §17b): born far side, 37.5% reach food and 85% starve; food side, 79% and
   59%. About 200 of the 277 dead on the colony bed never reach the food.
2. **Second: the nest reads as a nest on screen.** Judged by eye, and it has
   failed several times ("looks like nothing. a hole floating spoil"). It must
   not block the first.

## Live question

**Does a colony keep the hole when it works inside it?** (A5: `digbox`, 12
seeds, default / lined shaft / lined shaft with home reaching down it, widths
2 and 4.) Answered already: the shaft exists at frame 0 and, lined, stands.

## Predictions (written before each run; marked after)

| # | run | prediction | right? |
|---|---|---|---|
| 1 | `digbox NEST_SHAFT=20`, census at frames 0/1/5/30/300 | frame 0 reads open = 2·20+4 = 44 and roofed = 4·10−2 = 38 (the positive control); by frame 5 most of the cut has refilled, because the cut is not lined | **Right on both, but the census hid the second half** (below) |
| 2 | lined cut, same run | ≥ 90% of the cut open through frame 300 | **Right**: 82 of 82 |
| 3 | `digbox` 300 ants, what floats (made while planning, before the code survey quoted `update.rs`'s 76-of-76-lining note) | mostly spoil pellets resting on ants | **Wrong**: the floating is lining; the spoil is up in the arch, standing on lining |
| 4 | A5, `digbox` 12 seeds: lined shaft (w2, not home) vs default | the room is taller-for-its-width (`vert`) and its middle half narrower (`iqr`) on ≥ 8 of 12 seeds | **Right**: 11/0 and 12/0, and it survives masking the cut out |
| 5 | A5: shaft home (`NEST_HOME=shaft`, w2) vs lined shaft not home | deeper again (`vert` up on ≥ 8 of 12); and the cut holds **more ground** (spoil + lining + soil) at 6,000 frames on ≥ 8 of 12, because ants at home dig inside it and the foot of an open shaft is a legal spoil drop | **Wrong on both**: `vert` 6/2; the cut holds **less** ground, 1 up / 11 down |
| 6 | A5: width 4 vs width 2, both home | more of the cut still open at 6,000 frames on ≥ 8 of 12 | **Right**, 12/0 (partly by construction: 122 cells against 82) |

## §19 (the loop session's door)

On its branch, not landed (`01b4dba1`, 2026-09-26): `PIXEL_PHYSICS_NEST_DOOR=<d>`
paints a door of 2d+1 columns and homes every founder there; `…_FOUNDERS=pile`
heaps them on it. Colony bed: starved 277 → 209 (18 better / 4 worse). This lane
builds the "shaft is home" arm on it once it lands. Note for then: its founder
anchor is taken from `colony_surface` *after* the cut, which in a shaft column is
the chamber floor.

## Results

**A1, the unlined shaft (2026-09-26, `main` binary, `digbox ants=0`).**
Frame 0 reads open 44, roofed 38: the cut exists, exactly as drawn. By
frame 1 the chamber roof has dropped; by frame 5 the shaft and the chamber
are gone and a 1–2 row dip stands in the surface over where the chamber was.
**`digbox`'s `roofed + open` read 82 at every stop through it**: the void
does not vanish, it moves up into the dip, which the census counts as open
room. So a collapse reads as nothing happening. `digbox` now carries a
`cut:` line counting the cut's own cells from the footprint the cut records
on its site. The frame-0 render also shows the harness-founded shaft drawn as
**sky blue**: the underground map is frozen on frame 1, after the cut.

**A1, the repair.** The cut now lines its walls with the ant's own lining
(`pack_neighbours`, split out of `line_burrow` so the ants' `packed` counter
stays theirs), freezes the world's genesis before it cuts, takes a width dial
(`PIXEL_PHYSICS_NEST_SHAFT_WIDTH`, default 2) and records its footprint on
the nest site (`NestSite::shaft`). Same run: **82 of 82 cells open at frames
0/1/5/30/300**; the unlined control (`BURROW_LINING=off`) reads 42 at frame 1
and 6 at frame 5. It now draws as a dark hole with daylight fading down it.
Guard `the_founding_shaft_stands` (both drivers, unlined control inside it)
watched red with the lining disabled: *"only 6 of 66 cells are open"*.

**A2, what the floating is** (300 ants, 6,000 frames, seed 0, the settings
the owner judged). Tinted from one run: the grey grit is **tunnel lining**
(1,396 cells on screen, 421 above the old ground line) and **ants** (1,234
cells); the heap's arch hanging from the top of the sky is lining and spoil
(266 of 269 spoil cells are above ground). **82% of spoil drops (5,292 of
6,434) went up the digger's own column** rather than beside it; 362 cells of
ground have no path to the floor, and all 102 cells standing on nothing are
lining. So the lever for the look is what lifts pellets and what makes a heap
self-supporting, not where an ant puts one down beside itself.

**First look at a shaft with ants** (same settings, one seed, not a result).
At 6,000 frames the cut is 58 cells of ants, 16 of ground, 8 open: it stays a
space and fills with animals, not dirt. The room ends 28 rows deep against 21,
and its middle half is 29 columns wide against 56.

**Bed baseline, reference binary:** 24 seeds reproduce the stored §18
default funnel line for line (starved 277, loops 366).

**A5, does a colony keep the hole** (`digbox`, 300 ants, 6,000 frames, seeds
1–12, one binary; scored on the colony's own digging, i.e. with the founding
cut masked out of the census, because a shaft arm with no ants at all reads a
13-column room by construction). Seed by seed against the default:

| arm | own room | depth | height/width | middle-half width | chambers (owner's metric) |
|---|---|---|---|---|---|
| lined shaft, w2 | 10 up / 2 | 11 / 0 | 11 / 0 | **narrower 12 / 0** | 3 / 1 |
| lined shaft, w4 | 9 / 3 | 12 / 0 | 12 / 0 | **narrower 12 / 0** | 5 / 1 |
| shaft is home, w2 | 10 / 2 | 12 / 0 | 12 / 0 | **narrower 12 / 0** | 8 / 1 |
| shaft is home, w4 | 10 / 2 | 12 / 0 | 12 / 0 | **narrower 12 / 0** | **10 / 0** |

Medians: middle half 45 → 27–30 columns, depth 23 → 28–32 rows, chambers
0 → 1.5 in the home arms (rooms ~8–10 tall, 8–12 wide, passages still 2). **A
founding hole concentrates the whole nest, on every seed.** Home against not
home changes little in shape and keeps the hole clearer: ground in the cut
lower on 11 of 12 (w2) and 9 of 12 (w4). What fills the cut is ants (median 52–70
cells), in a box that grows past 800 of them.

## Head SHAs

- `636612c6` — branch cut from `main`.
