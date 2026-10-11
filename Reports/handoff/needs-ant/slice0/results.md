# Needs walk, slice 0: hooks and instruments (2026-10-06)

Branch `claude/project-thread-ns0j6p`: 1b52c78b (switch + hooks), 7041594d (guard), then the cost harness.
Design doc: https://claude.ai/code/artifact/6f9b6730-a887-4697-8689-906afc9afb0a (rev 129).

## 1. The switch changes nothing (measured)
`World::needs` (None = today's ant) and four hooks in `creature_tick`; `deeptrace needs=passthrough needsat=20000`.
Seeds 1-2 to 60k, plus seed 1 with NEEDS_FIRST+CARRY_HOME+NEST_REST on: every output file byte-identical
to the parent commit (colony.csv, stats.csv, ledger.csv, every map, hungry.csv.gz decompressed, events.txt
less the NEEDS line). Positive control: nudging any one of the three live hooks differs at the first map
after the switch (f21000) and at none before.

## 2. The no-veto guard (measured, `cargo test --lib needs -- --ignored --nocapture`)
36 rows: 6 places (door, chamber, deep room, mound tunnel, mound pocket, buried pocket) x 3 loads
(nothing, soil pellet, food in the jaws) x 2 roles (forager, nest worker); each ant at 30% of its 200 J,
bound 3,000 frames. Green = ate, or reached open air (not with food in the jaws), or (sealed) cut 3+ times,
before dying; and never let go of a load in a way that cut it off from the air.
- Today's ant: 20 of 36 green. Red: food in the jaws put down then starved outside (6); mound tunnel, no
  pull out (4); buried and lean, never cuts (4); forager in the mound pocket stops after 1 cut (2).
- With NEEDS_FIRST on: 26 of 36.
The bed itself has a test (`the_guard_bed_stands`), watched red with the tunnel lining removed.

## 3. Switch frame: 50k for every seed (measured on the 3ba1e7bd5 baseline maps, seeds 1-12)
Open dug cells more than 3 columns off the door column first reach 100 at 18k-34k (seed 3 last);
at 50k every seed has 162-251 such cells, 22-28 rows deep. ("Seed 4 has none at 50k" was the old game.)
Script: `dugnest.py` (scratch); table below.
seed: >=100 cells / at 50k cells, deepest row
s1 22k 215/25 | s2 18k 200/26 | s3 34k 181/22 | s4 20k 213/28 | s5 18k 251/24 | s6 26k 220/24
s7 24k 216/24 | s8 18k 232/26 | s9 30k 162/22 | s10 24k 174/23 | s11 18k 197/23 | s12 20k 190/24

## 4. Frame cost (measured; instructions, not time)
`examples/needscost.rs`: goal box built exactly as deeptrace builds it (538 / 548 ants at 100k, seeds 1 / 2,
the baseline's counts), 1,200-frame window profiled with callgrind; RAYON_NUM_THREADS=1.
Native: 2.28 ms a frame (seed 1, min of 3 windows).
                                   seed 1        seed 2
  whole frame (M instructions)     24.24         24.26
  regroup_by_scent (colony labels) 41.7%         42.5%   <- every tick, pairwise over ~540 ants
  frame::step (the world)          55.7%         54.8%
    creature pass                  37.3%         36.6%
      brood::pile_site (egg site)  13.8%         13.0%   <- 57% of ticks search, most fail
      sense (brain inputs)          8.4%          8.5%
      chooser_step (heading)        4.3%          4.3%
      eval_brain                    4.2%          4.2%
      act                           2.2%          2.3%
    pheromone planes                8.3%          8.3%
    CA sweep                        6.2%          6.0%
    field                           2.8%          2.7%
So today's choosing (chooser + brain) is ~8.5% of the frame; the two biggest costs are not the walk.
Control: arms off vs passthrough time the same natively (0.560 vs 0.562 ms a frame at 20k).
Reproduce: `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --example needscost
--target-dir T; RAYON_NUM_THREADS=1 valgrind --tool=callgrind --instr-atstart=no T/release/examples/needscost
seed=1 at=100000 window=1200 reps=1 cg=1` (~20 min; the pre-run is uninstrumented).
