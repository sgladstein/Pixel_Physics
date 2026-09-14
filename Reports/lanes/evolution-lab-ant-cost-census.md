# Lane B — where an ant's cost actually goes

*Round 36, 2026-09-14. Branch `claude/evolution-lab-ant-cost-census`. Census
first, fix second — and the census is the deliverable.*

**Full record:
[`../evolution-lab-ant-dirty-cells-2026-09-14.md`](../evolution-lab-ant-dirty-cells-2026-09-14.md).
Harness: `examples/antdirt.rs` (new, mine). Nothing under `src/` touched.**

## What the round asked, and the answer

*What are the 29.4 swept cells an ant dirties per frame made of?*

**Three of them are the ant. The rest is one rule, and it is not about
animals.** A dirty mark is expanded sideways by the largest
`Material::sweep_reach` of any cell in its whole 4,096-cell chunk. In this bed
that maximum is **24, set by water, in chunks holding an average of 1.4 water
cells** — soil's own reach is 2. So an ant's body write in dry soil is
re-examined across 49 columns instead of 5 because of one droplet somewhere
else in the chunk.

Per ant per frame, three seeds:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| cells an ant's body changed | 0.82 | 0.76 | 0.78 |
| `swept` — today's rule | 21.73 | 12.38 | 16.89 |
| `cellloc` — same guarantee, computed per mark | **4.00** | **3.16** | **3.68** |
| the 3x3 floor (not a legal rule) | 3.15 | 2.98 | 3.00 |

**A mark-local reach cuts the region the sweep is asked for 4.8–5.7x with no
loss of conservatism** — the derivation is §6 of the report, and
`parallel.rs`'s write-safety proof is untouched because it is stated against
the bounding box and `touch_neighbours`' flat `MAX_REACH`.

Read the spreads too: today's rule varies **1.75x across three seeds of one
bed**; the ant's floor varies 6%. **What an ant costs today is a fact about
what shares its chunk.** The same seed reads 21.7 at `grow=6000` and 13.3 at
`grow=2000` — so **29.4 is a per-bed number, not an ant's**, and should stop
being quoted as one.

## Four candidates in, four answers out

| the brief's candidate | verdict |
|---|---|
| its own moves | **0.76–0.82 cells/frame**, floor 2.8–3.0 — 14–24% of swept and mostly irreducible |
| the cells it wakes by being adjacent | 26–67% of the *level*, **−0.3 to +3.2 cells per ant at the margin** — not an ant lever |
| the chunk it keeps awake | **struck off**: `awake` is flat at 0.01 chunks/ant. Ants widen rects, they do not wake chunks |
| the pheromone it writes | **struck off by reading the seam**: `deposit_pheromone` marks `write_watch` and the pheromone plane only, never a chunk's dirty channel |

And the structural result that matters most: **the two inflations overlap
almost entirely.** Per-row spans save 6% of the marginal cost, per-cell marks
another 7%, cutting the reach as well saves 75%. **Which explains
`PIXEL_PHYSICS_SWEEP=rows` measuring 1.05x on the tick** — it removed one of
two mechanisms covering the same cells, so 6% of the region was all it had.

## For the coordinator — the one thing to route

**The fix belongs in `src/sim/chunk.rs` + `src/sim/world.rs`, which I do not
own this round, so it is written up rather than written.** Shape, from the
census: the far-reaching cells are one or two per chunk, so a chunk keeps
their x-positions in a tiny list and a mark's reach becomes "the chunk maximum
if one is within it, else the ordinary floor" — a few comparisons against a
list that is almost always empty.

**Do not route it yet.** §E2 is in front of it: `PIXEL_PHYSICS_SWEEP=rows` is
*also* a strictly-conservative narrowing and still diverges at frame 4,330 on
the standard lab bed, with the RNG coupling ruled out and a second coupling
unidentified. A per-mark reach is the same class of change and hits the same
wall. **The routable job is §E2** — understand the coupling, then the 5x is
available. Its leading untested hypothesis is already written down: chunk
wakefulness feeding `field::step`'s `active_chunk_count()` gate.

**And nothing here is a timing.** `antdirt` publishes no clock on purpose.
*Removing work is not removing cost*; the region is an upper bound and the
`rows` precedent says the realised frame gain is much smaller. Whoever builds
it owes a paired alternating whole-frame run.

## Two harness findings worth keeping

**An age pin over a bed with no income is a starvation budget.** My first run
used a fixed `age=14000`, in good faith, because round 34 requires age
matching. Arms stocked to **0 / 80 / 241 / 420** ants stood at **0 / 0 / 5 /
15** by frame 14,000 — four labels, one population, and it would have
published a table. `antdirt` now stocks every arm first and ages them all to
the latest frame *any of them reached*, and prints the standing count twice.
Anything with `founders=0` and a fixed `age=` has this defect.

**`labperf`'s `est_` columns need a moisture filter, and `antdirt` has one.**
`chunk.rs` already records that they are ~90% soil moisture and fail their own
control; reproduced here at `bbox/swept` **2.33–4.39**. Moisture writes are
identifiable (material and organism id unchanged, only `aux` moved, material
holds water) because they all reach the grid through `set_world_quiet`.
Filtering them *and* adding the `touch_neighbours` marks — which the `est_`
columns also omit — puts the control at **0.98–1.06 on every arm of every run
in this report**. Porting both to `labperf` would make its `est_` columns
usable; I did not, because `labperf` is contested and not mine.

## Controls, both green

- **Specificity.** `ants=0` must attribute nothing to an animal: `sw_ant 0
  ch_ant 0 chg_ant 0` — PASS on every run.
- **Sensitivity of the reconstruction.** `bbox/swept` **0.98–1.06**, so the
  ladder ranks the rule that is actually running.
- **Ladder order.** `cells <= cellloc <= cellreach <= rows <= bbox` asserted
  per frame — **0 crossings**. Each rung alone is a plausible number and
  nothing else would notice a crossing.

## Review queue

**Nothing posted.** The queue is for visual evaluations only (owner ruling)
and this lane produced no artifact anyone judges by eye. Checked open at the
start: empty.
