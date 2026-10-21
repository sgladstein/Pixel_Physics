# A2, node keying: built, measured, parked off (2026-10-21)

`World::node_keyed_growth` (`plant::node_stream`; `PIXEL_PHYSICS_NODE_KEY=1`
or `clone_identity node=1`): under the plant key, a growing or root tip draws
on `(dev_seed, cell relative to the collar, node count, stale visits)`
instead of its age in frames, so a tip that arrives late rolls the same
numbers. Every other cell keeps the clock.

## Measured (`clone_variance dev=0`, 8 founders, 20,000 frames, outdoor bed)

Size CV (cells), age key -> node key:

| | spot to spot (one seed) | seed to seed (one spot) |
|---|---|---|
| herb | 0.037 -> 0.156 | 0.309 -> 0.472 |
| shrub | 0.020 -> 0.024 | 0.290 -> **1.337** |
| grass | 0.230 -> 0.126 | 0.634 -> 0.387 |

`clone_identity dev=0 plants=4 frames=20000 gap=120`: herb clones end 163-294
cells (Jaccard 0.36-0.56) under the age key, 263-306 (0.33-0.99) under the
node key; the four shrub clones all stalled at 57 cells under the age key and
all grew (1,855-2,183) under the node key -- a different draw of the same
lottery, not evidence either way.

## Reading

Not a win: grass tightens, herb and shrub widen. The herb spot-to-spot rise is
one founder of eight (173 cells against 270-305, root share 0.86: its shoot
stalled), and shrub's seed-to-seed 1.34 is the stall lottery going bimodal.
**The spread is dominated by whether a seedling's first tips survive** (M0 §3),
which keying cannot touch; it decides which plants lose, not how many. So A2
is parked off and the next step is the rest of A3 (tip survival, the taper as
a carried fraction), which shrinks the dice; A2 is re-measured after it, when
keying is what decides the remaining spread (the plan's own A4 argument).
