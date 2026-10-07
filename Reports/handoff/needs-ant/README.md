# The new ant (the needs-and-jobs walk): paused

**Paused by Scott on 2026-10-07**, after slice 1 and its one fix round both failed the
gate and met the kill rule (a late collapse, and less food carried in from outside than
the shipped ant). Nothing here is on `main`.

- **Code:** branch `claude/project-thread-ns0j6p`; the fix round is commit `067dad72`.
  `src/sim/creature/needs.rs`, hung off `World::needs`, off by default
  (`PIXEL_PHYSICS_NEEDS=walk` in the game, `needs=walk` in `deeptrace`). No PR.
- **Design:** the live design doc is
  https://claude.ai/code/artifact/6f9b6730-a887-4697-8689-906afc9afb0a (its *Build order*
  item 1 and *Kill criteria* carry these results). `design-doc-rev132-2026-10-07.txt` is a
  text copy from just before the fix round's results were written into it;
  `design-doc-snapshot-2026-10-06.md` is the copy the reviews read.
- **This folder** is the project's shared `needs-ant/` folder, copied whole to
  `Reports/handoff/needs-ant/` on the branch. The raw runs (several GB) were left in the
  lane's container and go with it; they regenerate exactly (below).

## Read first

1. `slice1/fix-round-results-2026-10-07.md`: the last result, all four fixes together and
   each alone, against slice 1, the stack and shipped, and the seed-1 trace of the door.
   Chart: https://claude.ai/artifact/PnGwDdHmeh6hV86XJNKftV (sources in `slice1/chart/`).
2. `slice1/results.md`: slice 1.
3. `slice1/fix-proposal-2026-10-07.md`, and the other lane's review beside it.

Before slice 1 (2026-10-06): `review/` (four adversarial reviews of the design; the
physics review's scripts are in `review/physics-scripts/`), `gradient-check/`,
`porosity/`, `depth-test/`, `slice0/`, `trail-lessons-2026-10-06.md`, `tools/`.

## Before building any more

Trace first (Scott's standing rule): why the dig job cut 568 cells below ground against
shipped's 143 (inferred to keep loose soil over the door, so it stays shut), and why births
fell under Fixes 2 and 3 (untraced). Then a proposal, reviewed by another lane, then build.

## Reproducing the runs

Deterministic: same build, same seed, same game. From the branch, with no
`PIXEL_PHYSICS_*` variables set (the wrappers in `slice1/runs/` unset them):

    cargo build --release --example deeptrace
    RAYON_NUM_THREADS=1 target/release/examples/deeptrace scenario=nest_goal seed=S \
      frames=300000 ants=0 mapevery=1000 hungry=1 out=DIR ARM

| Arm | ARM |
|---|---|
| shipped | nothing |
| the stack | set `PIXEL_PHYSICS_NEEDS_FIRST=on PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_NEST_REST=on` |
| slice 1 | `needs=walk needsat=50000 walktrace=20` |
| all four fixes | `needs=walk needsat=50000 needsparts=all walktrace=5 dig=1` |
| one fix alone | `needs=walk needsat=50000 walktrace=20 dig=0 needsparts=` Fix 1 `only_diggers,dig_job,clear`, Fix 2 `pace,give_up,lay_home`, Fix 3 `meal`, Fix 4 `won_stall` |

Seeds 1-4. Mutation off and the evolved founder are the defaults here. `dig=` and
`walktrace=` only record; they do not change the game. With every part off, the fix-round
code is slice 1 exactly (checked on seed 1 to 55k).

The scripts in `slice1/tools/` import `doorseal.py`, which is in the repo at
`scripts/deeptrace_tools/doorseal.py`: run them with `PYTHONPATH=scripts/deeptrace_tools`.
`slice1/tables/` holds tables they and the trace tests printed during this work.

**Trap:** `cells.csv.gz` books a cut only when the ant's act made it. The walk's escape,
pack and door cuts get a blank cause, so `sealcell.py` reads them as soil that fell; match
`walk.csv`'s `cut` column instead.
