# Is there a real gradient? Nest-air and larva-scent fields on real nest maps

Rule audit thread, 2026-10-06. For the design doc "A needs-and-jobs ant: design for a second walk",
section *Scents and fields in depth*. Everything here is computed from the shared baseline's maps
(`deep-trace/baseline/3ba1e7bd5`, main 3ba1e7bd5, evolved founder, mutation off): seeds 1-4 at
frames 100k, 200k and 295k (12 maps; seed 4 at 200k had its door shut, so 11 have a way out).

## What was solved

Steady state of a diffusing gas on the map's open cells (`.`, ant, brood, crumbs), 4-neighbour
exchange, sources averaged over the 11 maps at f-5k..f+5k (ant positions over ~10k frames):

| Field | Sources | Walls | Sink |
|---|---|---|---|
| air, sealed walls | ant 1, brood 0.25 | soil passes nothing | open sky = 0 (open cell above ground with nothing over it) |
| air, walls leak 2% | same | soil passes 2% of air | same |
| air, walls ignored | same | soil passes everything: what `pheromone.rs` does today | same |
| larva scent, 3 / 10-cell reach | brood 1 | sealed | decay, reach sqrt(D/k) = 3 or 10 cells |

## What was measured (over dug-nest cells: open, below the ground line, reachable from sky)

- **readable**: an ant's 6-cell sensor (`sensor_offset` 6; 4,4 on a diagonal), front on an open cell,
  reads |front-here|/(front+here) of at least 1% on its best heading.
- **out**: steepest walkable descent ends in open sky.
- **agree**: the steepest-descent step also shortens the walk out (8-neighbour BFS from open sky).
- **into soil**: the steepest descent over all 8 neighbours is a soil cell.
- **settle**: frames for today's plane rate (D = 0.25/36 cells^2/frame, one pass per 12 frames) to bring
  the nest within 10% of steady state, from empty.

## Results (median over 11 maps, range in brackets)

| Field | readable | out | agree | into soil | room spans |
|---|---|---|---|---|---|
| air, sealed walls (7 maps with a way out) | 9% [3-16%] of cells; room cells 9% [2-16%] | 99.6% [99.0-99.8] | 96% [95-98] | 0 | 6% [3-9%] of its top |
| air, walls leak 2% | 27% [22-41%] | 8% [0-16%] | 61% [58-69%] | 22% [19-26%] | 12% [8-26%] |
| air, walls ignored | 100% | 0% [0-3%] | 19% [8-30%] | 17% [15-20%] | 58% [47-64%] |
| larva scent, 10-cell reach | 100% | n/a | n/a | 0 | ascent ends at the brood pile for 85-100% of cells |

Sealed air reads as sealed (no value) on 4 of the 11 maps: the nest's only way out passes a corner
gap two soil cells touch diagonally, which a face-to-face gas exchange cannot cross and an
8-heading walk can (`dbg3`-style check in `gradcheck.py`'s notes; seeds 1@100k, 1@295k, 2@200k, 3@295k).
The medians above for sealed air are over the 7 maps where it has a way out. On those 7, the median
contrast at the 6-cell sensor is 0.2%, 95% of 1-cell steps are under 1/256 of the nest's top (flat in
an 8-bit plane) and 4% under 1/65,536 (flat in 16 bits even at perfect scaling). Steepest ascent ends
deep in the room, 26-48 rows under the ground, for 73-92% of cells.

**Settling at today's plane rate** (seed 1, 200k): walls ignored 492,000 frames; sealed walls did not
settle within 1,500,000 frames. Positive control (`control.py`): a 5-wide, 40-deep shaft settles in
222,000 frames against 218,000 predicted, and the steady state matches the analytic profile.

**Keeping a sealed field current** (`warm.py`, 4 maps): red-black SOR to 1% of the direct solve took
1,710-5,375 sweeps warm-started from the field 1,000 frames earlier (3,455-6,760 cold), over
900-1,370 open cells. 33-68 cells opened or closed per 1,000 frames, mostly in the mound.

## Reading

The sealed field has the right shape (down leads out, up leads deep) but the room is a plateau: the
whole room varies 3-10%, so an ant's sensor reads under 1% almost everywhere in it. The gradient lives
in the shaft and the mound's tunnels. Cox & Blanchard (2000, J Theor Biol 204:223,
doi 10.1006/jtbi.2000.2010, via PubMed) predicted exactly this for real nests: "a plateau of high
concentration in the back half of the nest; an intermediate region of increasingly steep gradient
towards the entrance; and a steep linear gradient in the entrance tunnel". Any leak through soil turns
the slope toward the roof. A terrain-blind plane makes a readable blob centred on the brood that
points at the walls and away from the door.

## Pictures

- `inside-nest_s1_f200000.png`, `inside-nest_s3_f295000.png`: each field stretched over the nest's own
  range (top row) and where a 6-cell sensor can read it (bottom row; green readable, grey flat, red slopes
  into soil). Seed 3 at 295k is one of the corner-gap maps.
- `fields_s*_f*.png`: whole-map views on a log scale (the nest saturates; use the inside-nest pictures).

## Files

`gradcheck.py` (solver and metrics), `render2.py` (pictures), `warm.py` (solver cost), `control.py`
(positive control), `agg.py` -> `aggregate.txt`, `results.jsonl` (one line per map), `settle/` (the
run with settling times).
