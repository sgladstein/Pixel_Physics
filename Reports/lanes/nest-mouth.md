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

**Does the founding shaft exist at frame 0, and does it stand?**
`PIXEL_PHYSICS_NEST_SHAFT=<rows>` has never been censused at frame 0, and it
cuts without lining — `line_burrow`'s own doc says an unlined cut in soil is
gone by frame 5.

## Predictions (written before each run; marked after)

| # | run | prediction | right? |
|---|---|---|---|
| 1 | `digbox NEST_SHAFT=20`, census at frames 0/1/5/30/300 | frame 0 reads open = 2·20+4 = 44 and roofed = 4·10−2 = 38 (the positive control); by frame 5 most of the cut has refilled, because the cut is not lined | |

## Results

(none yet)

## Head SHAs

- `636612c6` — branch cut from `main`.
