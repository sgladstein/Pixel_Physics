# Lane B — where an ant's cost actually goes

*Round 36, 2026-09-14. Census landed as #433 from
`claude/evolution-lab-ant-cost-census`; the fix attempt that followed the
owner's "fix it" is on `claude/evolution-lab-mark-local-reach`.*

**Full record:
[`../evolution-lab-ant-dirty-cells-2026-09-14.md`](../evolution-lab-ant-dirty-cells-2026-09-14.md)
— §9 is the fix attempt. Harnesses: `examples/antdirt.rs` and
`examples/sweepgap.rs`, both new.**

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
`parallel.rs`'s write-safety proof is untouched (verified, see below).
**But read the next section before building anything from this line: the
saving is a *shape* prize, not a reach prize**, and calling it a reach prize
is a mistake this note made first.

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

## The fix, after the owner said "fix it" — what happened

*Branch `claude/evolution-lab-mark-local-reach`. Report §9 has the whole of
it.*

**I did not deliver the 4.8–5.7x, and the reason is a measurement, not a
shortfall of effort. Two findings, both load-bearing:**

**1. The single-rect version of this fix is arithmetically incapable of
working.** Built and measured first, because keeping the region's shape
untouched is what stays clear of §E2. Taking the reach over the rows the
region occupies instead of all 64: **19,261 → 18,904 cells a frame, 1.9%.**
The mechanism is plain — `swept` is ~64 wide by 18 rows per awake chunk, so
**at reach 24 on a 64-wide chunk the rect is already clipped to the full chunk
width.** Every single-rect rule is full-width. So **§6's 4.8–5.7x is entirely
a *shape* prize wearing a reach label**, and my own note said "reach" where it
should have said "shape". Correcting that is the most useful thing in this
slice.

**2. §E2 is now bisected to a cell, and it is the soil-moisture channel.** It
had only ever been bisected to a frame, and the reason was mechanical: the
switch was a process-wide `OnceLock`, so two settings meant two processes, two
processes of a chaotic sim are two different worlds, and the only available
comparison was a world hash — which says *that* two runs differ and never
*which cell*. `Chunk::sweep_rows` is now a per-chunk field with
`World::set_sweep_rows` beside it (shipped default unchanged, same env var),
and `examples/sweepgap.rs` steps both arms in lockstep in one process:

```
FIRST DIVERGENCE at frame 237 -- 4 differing cell(s)
   371 160  soil/soil  aux 795 vs 875   chunk 5,2  reach 24   in B? yes
   373 160  soil/soil  aux 701 vs 620   chunk 5,2  reach 24   in B? yes
   371 161  soil/soil  aux 736 vs 763   chunk 5,2  reach 24   in B? NO
   373 161  soil/soil  aux 646 vs 620   chunk 5,2  reach 24   in B? NO
```

**Every differing cell is `soil`, same material, same organism id, different
`aux`. Nothing moved differently; the water held in four soil cells did.**
Frame **237**, not 4,330 — thirty seconds instead of a bisect. `arm=box` (both
arms on the shipped rule) shows no divergence in 1,500 frames, so it belongs
to the rule and not to the harness.

**Not established: why.** The moisture pass walks its own region over every
chunk and its seed is identical in both arms, so the obvious route is ruled
out and the real one is not found. A located fault, not a diagnosed one.

**For the coordinator, plainly: the shape change is what earns the prize, and
shipping it today ships a known divergence.** The next job is `sweepgap`
pointed at why those four cells' moisture differs — now a half-hour question.
I have taken no frame-cost measurement because there is nothing yet to time;
the thing that would earn one is the change I am saying should not ship.

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

## Controls — and one fault-back found a hole

**The new guard is sensitive to one fault and measured blind to another**, and
both were established by putting the fault back rather than argued:
`Chunk::set_sweep_rows` made a no-op → **red** (`walked 2013, box 2013`);
`World::new_chunk`'s override dropped → **still green**. The hole is recorded
in the test itself. Its first geometry — two marks on one row — could not
discriminate at all, because a row span is the *hull* of the marks on its row,
so both rules gave the identical region and it read `walked 183, box 183` for
a setter that worked perfectly.

**`parallel.rs`'s proof: verified, not asserted** (the coordinator was right
to ask). `concurrent_chunks_are_never_within_reach_of_each_other` is
exhaustive over **chunk coordinates and the flat `MAX_REACH` alone** — it
reads neither `sweep_region`, nor `sweep_plan`, nor a chunk's tracked `reach`.
The claim holds, and the new guard asserts the bounding box is identical under
both rules so it stays held.

## Census controls, all green

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
