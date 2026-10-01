---
paths:
  - "src/sim/**"
  - "src/worldgen/**"
  - "src/lab/**"
  - "src/druid/**"
  - "examples/**"
  - "scripts/seedsweep.sh"
  - "scripts/acceptance.sh"
  - "scripts/blastsweep.sh"
  - "scripts/plantsweep.sh"
  - "scripts/worldgencheck.sh"
  - "scripts/megastudy.sh"
  - "scripts/worldgen_sweep.sh"
  - "scripts/labpair.py"
  - "scripts/funnelpair.py"
  - "scripts/tracepair.py"
  - "scripts/trailpair.py"
  - "scripts/nestscore.py"
  - "scripts/twopile.py"
---

# Measuring the world: traps that have each cost real time

Moved out of `CLAUDE.md` on 2026-09-30 so that sessions which never touch the
simulation do not pay for it. **The worked cases and every number** are in
`Reports/claude-md-evidence-2026-09-30.md`, under the same headings. If you
measure the world without opening a file that matches the globs above, read
this file by hand.

## Metric traps

- **Liquids: measure column *volume*, not the topmost cell.** Near-empty cells
  fringe every artifact (topmost said 1.7x, volume said 9x).
- **Dark or torn rows: measure *fill*, not occupancy.** A row draws black
  while every cell in it is occupied.
- **Powder faces: measure the face, not the spreading front.**
- **Excavation: standing void is not *dug* void.** A nest is **roofed** void,
  empty cells with ground above them; a hole open to the sky is not a room.
  Censusing empty cells scored the roofless build higher, exactly backwards.
- **Destruction: a failure count is not a damage count.** A failed cell that
  became rubble is still standing. Census materials before and after.
- **Whether something turned to dust:** read `FailureCounts::crumbled`
  (`filmstrip` prints `crumbled to grit`), never the mean failing-region size,
  which divides by events and moves when marginal rock fails more often.
- Prefer a **continuous** quantity (a summed deficit) over a **count** of bad
  cells; counts give knife-edge margins.

## Timing a collapse or a blast

- **Size a problem at the moment it starts, not after it has been running.**
  Take a second census close to the event. If the two differ by orders of
  magnitude, the late one is the engine's *response*, and that is what to fix
  (a charge invalidated 370 cells at 5 frames and 67,100 at 1,300).
- **A cascade censused before it settles reads a *delay* as damage.** Both
  runs must have landed. **`seedsweep.sh`'s default `FRAMES="start=2 every=400
  count=4"` stops mid-collapse**; use `every=900 count=5` and read **`rock`**,
  not `cells lost`, which rides the water cycle at about ±1,700 cells and never
  settles. Settled means the censused quantity stopped moving across two
  consecutive tiles.
- **Two runs that diverge on one frame are different worlds by the next**, so
  one cascade scene cannot compare two models. Compare in `seedsweep.sh`, run to
  rest, at the order statistic.

## Two drivers, and the app runs the parallel one

`update::step` is serial; `parallel::step` is a four-pass checkerboard and is
what `App::update` calls. **Test both.** `update::step_monolithic` (test-only)
sweeps the world as one region: the control for "movement rules, or how the
sweep is cut into chunks?"

## Chunk decomposition is a recurring root cause

Both drivers sweep chunk by chunk, and half of all horizontal seams invert the
bottom-to-top row order. **Artifacts that line up with the F1 chunk grid are
usually this, not the physics.** Suspect it early.

## A channel that decays *and* is read as a gradient needs range for both

Narrow storage takes the gradient away first, and silently. **Measure the
number the consumer computes, never the stored value**; **exactly zero** at
several sample points is the signature. Discriminator: the consumer's decision
threshold against the storage quantum; the risk is a consumer with **no
threshold at all**. Per-cell monotonicity defences (the pheromone decay LUT,
`diffuse_heat`'s nudge) cannot see a gradient. Survey:
`Reports/decaying-gradient-quantization-2026-09-15.md`.

## Guard hot-path work at the call site that already has the data

Per-material behaviour goes on `Material` as a field, tested at the dispatch
site that already holds the `Cell`. Never `World::get` or `id_of("name")` per
cell per frame inside the function.

## Frame cost

- **Gate on counters, never on wall clock** — and a counter is only
  load-independent at **fixed parallelism**: pin `RAYON_NUM_THREADS`, or
  compare both arms inside one run. **Pair every "it fired" counter with an
  effect counter from the far side of the call.**
- **Measure one scene, not the suite.**
- **A worst-frame figure is worthless unless an aggregate pins it**: check
  mean × frames ≈ worst before quoting a worst.
- **An isolated harness overstates what the app will see.** Quote the
  whole-frame figure (`scale_probe phases=1`), measured paired and
  alternating. A sub-phase that falls while the frame does not has usually
  relocated its cost, not removed it.
- Timings are machine-state-sensitive: `Reports/measurement-under-contention.md`
  §7 has the derivations.
