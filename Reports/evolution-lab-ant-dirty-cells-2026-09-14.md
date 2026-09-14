# What the 29.4 swept cells an ant dirties are made of — round 36, lane B

*Census, 2026-09-14. Harness: `examples/antdirt.rs` (new). Answers the
question round 36's brief
([`evolution-lab-round-36-brief-2026-09-14.md`](evolution-lab-round-36-brief-2026-09-14.md))
asked before anything is proposed, and
[`evolution-lab-knee-2026-09-14.md`](evolution-lab-knee-2026-09-14.md) §4
handed forward. **No timings in this report.** Every number is a count off
`World::chunks_to_sweep` and `World::sweep_region`, the two calls
`parallel::step` uses to decide what to walk.*

---

## The headline, and it is not about the ant

**An ant changes 0.8 cells per frame. The sweep is asked for 12 to 22 cells
on its behalf. Of that, 3 are the ant and the rest is one rule: a dirty mark
is expanded sideways by the *largest* `Material::sweep_reach` of any cell in
its whole 4,096-cell chunk.**

**In this bed that maximum is set by water, and the chunks it sets it for hold
on average 1.4 water cells.** Water's `sweep_reach` is 24
(`LIQUID_LATERAL_REACH`); soil's is 2. So an ant walking in dry soil sixty
cells from a stray droplet has every one of its body writes re-examined across
**49 columns** rather than 5, because one cell somewhere else in the chunk
could in principle travel that far.

Measured over three seeds, the region the sweep is asked for shrinks **4.8x to
5.7x** if each mark is expanded by what can actually reach *it* instead of by
its chunk's maximum — **with no loss of conservatism at all** (the derivation
is below, and it is the contract `Chunk::sweep_region`'s own doc already
states). That is the largest single lever this line has found, it is not an ant
lever, and it is **blocked by an already-open bug rather than by economics**
(§6).

---

## 1. The instrument, and the two things it had to fix first

`examples/antdirt.rs` reads the awake set and each awake chunk's
`sweep_region` before every step, snapshots the grid, steps, and diffs — then
classifies the *next* frame's awake set against the cells this frame actually
changed, which is the causal pairing. It publishes no clock, so there are no
reps and nothing for a contended box to move; `hash` per arm is what says the
counts did not move with the thread count.

**Two defects had to be fixed before any of it meant anything, and both are
reusable.**

**A `founders=0` bed has no food in it, so every frame past stocking is a
frame the population starves.** The first run used a fixed `age=14000` pin, in
good faith, because round 34's rule is that every arm must be age-matched.
Measured: arms stocked to **0 / 80 / 241 / 420** ants stood at **0 / 0 / 5 /
15** by frame 14,000. Four arms wearing four labels and holding one
population — a harness that has silently deleted its own x-axis, and it would
have published a table. `antdirt` now *derives* the pin: it stocks every arm,
then ages them all to the latest frame any of them reached, so the oldest arm
is not aged at all and prints both counts. **The general rule: an age pin over
a bed with no income is a starvation budget, and it has to be derived from the
arms rather than chosen.**

**Most of what changes in this bed does not wake the sweep, and the
reconstruction has to know that.** `labperf`'s `est_bbox` / `est_rows` columns
are built from a grid diff, and `chunk.rs`'s own `row_spans_enabled` already
records that they "are ~90% soil moisture and — since moisture got its own
dirty channel — do not wake the sweep at all, so they fail that instrument's
own stated control." Reproduced here: with every changed cell counted,
`bbox/swept` read **2.33–4.39**, a reconstruction two to four times larger
than the thing it reconstructs. Soil-moisture writes reach the grid through
`Chunk::set_world_quiet` and mark only the moisture channel, and every caller
in `update.rs` passes `cell.with_aux(..)` — so they are identifiable: material
and organism id unchanged, only `aux` moved, on a material with
`water_capacity > 0`. Filtering them (and adding the neighbour marks, §3) puts
the control at **0.98–1.06 over every arm of every run in this report**.

`chg/f` and `chgmoi/f` are printed side by side rather than the difference
alone, because a reader who does not know that 60–93% of the diff is invisible
to the sweep will read `chg/f` as the sweep's input.

## 2. The ladder, and why it is a ladder

Five rules over one set of marks, each a strict narrowing of the one above,
every one using the chunk's own `reach` except where stated:

| rung | rule |
|---|---|
| `bbox` | **today's**: one rect per chunk, `expanded_xy(reach, 1)`, clipped to the chunk. **The control** — it must land near `swept` or nothing below it means anything |
| `bbox_own` | the same with the `touch_neighbours` marks left out, so `bbox − bbox_own` is the adjacency tax in cells |
| `rows` | the same neighbourhoods as one x-span per row instead of one rect per chunk — this is `PIXEL_PHYSICS_SWEEP=rows`, which exists and ships off |
| `cellreach` | per-cell marks, each keeping its **full** `reach`, unions merged. Tighter than `rows` and exactly as conservative |
| `cellloc` | `cellreach` with each mark's reach derived from **what can reach that mark** rather than from its chunk's maximum |
| `cells` | the union of every mark's own 3x3 — **the floor**, and not a legal rule: it drops the reach entirely |

**The ladder's order is itself a control.** `cells <= cellloc <= cellreach <=
rows <= bbox` holds by construction, so it is asserted per frame and printed;
each rung on its own is a plausible number and nothing else would notice a
rung that crossed. **0 crossings** across every run here.

## 3. The decomposition

Three seeds, 512x512, `founders=0`, `grow=6000`, arms 0/12/40/120/300/440 ants
aged to a common frame, 120 frames each, `RAYON_NUM_THREADS=1`. Levels at
~440 standing ants, cells per frame:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| `swept` — what the sweep is asked for | 18,598 | 15,463 | 14,384 |
| `bbox` — today's rule reconstructed (**control**) | 19,116 (1.03) | 16,050 (1.04) | 14,798 (1.03) |
| `bbox_own` — without the neighbour marks | 11,440 | 9,722 | 11,704 |
| `rows` — per-row spans | 9,583 | 9,139 | 8,829 |
| `cellreach` — per-cell marks, full reach | 8,959 | 8,499 | 8,136 |
| **`cellloc` — per-cell marks, mark-local reach** | **3,261** | **2,933** | **2,970** |
| `cells` — the 3x3 floor (not legal) | 1,504 | 1,426 | 1,421 |

Per ant per frame, least squares over all six arms:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| `swept` | 21.73 | 12.38 | 16.89 |
| `bbox` (control) | 22.40 | 13.14 | 17.26 |
| `rows` | 17.04 | 16.07 | 15.54 |
| `cellreach` | 15.37 | 14.49 | 14.83 |
| **`cellloc`** | **4.00** | **3.16** | **3.68** |
| `cells` | 3.15 | 2.98 | 3.00 |
| `chg_ant` — cells an ant's body changed | 0.82 | 0.76 | 0.78 |

**Read the spreads before the values.** Today's rule varies **1.75x across
three seeds of the same bed** (12.38 to 21.73 per ant); the ant's own floor
varies **6%** (2.98 to 3.15). That is not noise, it is the mechanism: today's
rule couples an ant's cost to whatever else happens to share its chunk, so
what an ant costs is a fact about the bed. **The tighter the rule, the more
the number belongs to the ant.**

**Which is the first thing to say about the 29.4 itself.** It is not wrong and
it is not a property of an ant. Reproduced here at 21.7 on one seed with
`grow=6000`, and at **13.3 on the same seed with `grow=2000`** — nothing
changed but how long the bed settled before stocking, because a quieter bed
has fewer chunks already awake for an arriving ant to hide in. Quote it as a
per-bed figure, never as an ant's.

## 4. What the terms are, and why no *single* narrowing pays

The two inflations **overlap almost entirely**, and this is the load-bearing
structural result:

- Going per-row (`bbox` → `rows`) saves **6%** of the marginal per-ant cost.
- Going per-cell while keeping full reach (`rows` → `cellreach`) saves another
  **7%**.
- Cutting the reach *as well* (`cellreach` → `cellloc`) saves **75%**.

A mark at reach 24 covers 49 columns by itself, which is most of a 64-wide
chunk row — so the per-row hull has almost nothing left to add, and removing
the hull while keeping the reach removes almost nothing. **You have to remove
both, and either alone buys 6–13%.**

**This explains a historical result that has been read as disappointing.**
`PIXEL_PHYSICS_SWEEP=rows` measured 1.19x on the CA phase and **1.05x on the
tick** (`chunk.rs`, re-measured 2026-09-05). That is the `bbox → rows` step
above, and 6% of the region is what it had to work with. It was not badly
built and it was not badly tuned; it removed one of two mechanisms that
independently cover the same cells.

**And the adjacency term is large in the level and small at the margin.**
`World::write_cell` calls `touch_neighbours` on *every* write — its
interior-skip guard is a documented no-op, since `MAX_REACH` (32) is exactly
`CHUNK_SIZE / 2`, making the range it tests `32..32` — so every write marks its
own coordinate in up to five neighbouring chunks. Those marks are **26–67% of
`swept`** at the level (`nbr x` in the table) and **−0.3 to +3.2 cells per ant**
at the margin, because an arriving ant's neighbour marks land in chunks that
are already awake and already have marks further out. **So it is not a lever
for the ant term**, and a fix aimed at it would be aimed at the background.

## 5. What sets the reach, which is the finding

Sampled every 20 frames, over every awake chunk, the material holding the
maximum `sweep_reach` — and how many cells of the chunk's 4,096 actually carry
it:

| bed | material | awake chunks it set | reach it set | cells of it in that chunk |
|---|---|---|---|---|
| 440 ants, seed 1 | water | 59 | 24 | **1.5** |
| | soil | 28 | 2 | 1,968 |
| | packedsoil / spoil | 4 | 2 | ~1,918 |
| 440 ants, seed 2 | water | 55 | 24 | **1.3** |
| | soil | 28 | 2 | 1,964 |
| 440 ants, seed 3 | water | 51 | 24 | **1.4** |
| | soil | 22 | 2 | 1,978 |

**A majority of awake chunks are being swept twelve times wider than their
contents warrant, on the evidence of between one and two cells.** Same shape
at every population including zero ants, which is the point: **this is not an
ant finding, it is an engine finding that the ant census surfaced.** The
`ants=0` arms show `cellloc` at **5.0–7.3x** below `swept` too, so the prize
is the whole CA sweep's asked-for region — plants, weather, rain, everything —
not the animals.

Nothing here is a bug in `recompute_reach`. It takes one maximum per chunk
because it has one number per chunk to put it in, and its `.max(1)` floor and
`MAX_REACH` cap are both deliberate and documented. The mismatch is that the
number is *used* as though it belonged to the mark.

## 6. The repair, priced — and the wall in front of it

**The rule.** A cell `q` can move into a mark `p` in one tick only if
`|q.x − p.x| <= sweep_reach(q)`. So `max { sweep_reach(q) : |q.x − p.x| <=
sweep_reach(q) }` is a superset of everything that can reach `p` — which is
exactly the contract `Chunk::sweep_region`'s doc states ("a cell must be
reconsidered whenever anything it can see has moved"). The chunk maximum is a
superset of *that*. **So this is not a relaxation of conservatism; it is the
same guarantee computed per mark instead of per chunk.** And
`parallel.rs`'s cross-chunk write-safety proof is stated against the bounding
*box* and `touch_neighbours`' flat `MAX_REACH`, neither of which this touches.

**The prize.** `cellloc` is **17.5–20.6% of `swept`** at 440 ants and
**3.2–4.0 cells per ant per frame** against `swept`'s 12.4–21.7 — a 4.8–5.7x
cut in the region the sweep is *asked* for, landing within 6–23% of the
absolute floor. It is also the most stable column in the table after the floor
itself.

**Three things that must be said with it.**

**It is a region, not a frame.** *Removing work is not removing cost* — the
field's momentum gate removed 91% of its work and made the frame slower in 7
of 8 paired runs, because the arithmetic went and the memory traffic only
moved. The `rows` precedent is the closer one: 6% of the region there bought
1.05x on the tick. **Nothing in this report is a timing and nothing in it
predicts one.** Whoever builds this owes a paired, alternating whole-frame
measurement, and the honest prior is that a 5x cut in the region buys much
less than 5x.

**`antdirt`'s own implementation of it is not the implementation.** It scans
`2·MAX_REACH + 1` columns over three rows per mark, which costs far more than
it saves — fine for a census, useless in the sweep. The shape that would work
falls straight out of §5: the far-reaching cells are **one or two per chunk**,
so a chunk can keep their x-positions in a tiny list and a mark's reach
becomes "the chunk's maximum if one of them is within it, else the ordinary
floor". That is a few comparisons per mark against a list that is almost
always empty or a singleton. Designing and costing it is the next job, not
this one.

**§E2 is the wall, and it is the round's real target.** `PIXEL_PHYSICS_SWEEP=
rows` is also a strictly-conservative narrowing, and it **still diverges** —
identical through frame 4,329 and first differing at 4,330 on the standard lab
bed, bisected to the frame, with the RNG-stream coupling already ruled out as
the only cause and a second coupling unidentified
(`Reports/open-bugs-handoff.md` §E2; the leading untested hypothesis is chunk
wakefulness feeding `field::step`'s `active_chunk_count()` gate). A per-mark
reach is the same class of change and would hit the same wall on the same day.
**So the sequence is: understand §E2, then narrow the region.** Narrowing it
first produces a 5x-on-paper change that cannot be shipped, and this report is
not a licence to try.

## 7. Struck off, by measurement rather than by argument

- **The ant's own body is not the cost.** `chg_ant` is **0.76–0.82 cells per
  ant per frame** and the 3x3 floor around it is **2.8–3.0** — 14–24% of
  `swept`. A seven-cell `longant` on a `tick_interval` that gives it ~0.195
  ticks a frame writes about what it should. Making an ant move less, or
  carrying fewer body cells, is bounded above by that 14–24% and most of it is
  irreducible.
- **The pheromone it writes marks nothing.** `World::deposit_pheromone` marks
  the `write_watch` and calls `Pheromones::deposit`; it never touches a
  chunk's dirty channel, and `World::step_pheromones` runs on the pheromone
  plane. So the trail cannot wake the CA sweep at all — struck off by reading
  the seam rather than by measuring, which is the cheaper half of the brief's
  four candidates.
- **Adjacency is not a marginal term.** §4: 26–67% of the level, −0.3 to +3.2
  cells per ant at the margin.
- **Chunks awake for no reason are negligible.** `ch_stal` — awake with
  nothing changed in them and nothing within reach that changed — runs
  **0.25–1.10 chunks per frame** of 10–17 awake, and its swept area is 1% of
  the total at every population. There is no stale-wakefulness leak to find.
- **The awake *count* is not what grows.** `awake` is flat at **0.01 chunks
  per ant** — 11–17 chunks over the whole 0-to-440 range. Ants do not wake new
  chunks; they widen the rects of chunks that are already awake. Any proposal
  premised on keeping chunks asleep is aimed at the wrong quantity.

## 8. What `antdirt` can and cannot answer, so nobody re-derives it

- It answers **which rule would ask for how many cells**. It cannot answer
  what that is worth in milliseconds; that is `antcost` and a paired
  whole-frame run.
- Its arms are **`founders=0` by default**, so the plant bill is pinned and
  the population is dying of starvation throughout — which is why the age pin
  is derived rather than set, and why an arm's standing count is printed twice
  (after stocking and after ageing).
- `reachevery=0` turns off the reach census, which is a full scan of every
  awake chunk and the only per-cell work in the harness besides `cellloc`.
- **Its counts are deterministic, so it takes no reps.** Reps exist to beat a
  contended clock and it has no clock. `hash` per arm is what makes that
  checkable rather than assumed.
- The moisture filter is a **classification, not a measurement**: it names a
  change moisture-only when material and organism id hold still, only `aux`
  moves, and the material holds water. A future write of that shape that
  *does* mark the sweep would be misclassified, and `bbox/swept` drifting off
  1.0 is the tell.

---

## 9. The fix, attempted — 2026-09-14, after the owner read §6 and said "fix it"

**The single-rect version of this fix cannot work, and the reason is
arithmetic rather than tuning.** Built and measured first because it keeps the
region's *shape* untouched and therefore stays clear of §E2:

| rule | cells/frame at 440 ants | per ant per frame |
|---|---|---|
| `bbox` — today | 19,261 | 20.71 |
| `bbox_rowband` — reach taken over the rows the region occupies, not all 64 | 18,904 | 20.30 |

**1.9%.** And the mechanism is plain once looked at: `swept` is 19,261 over
~17 awake chunks, about **64 wide by 18 rows** — at a reach of 24 on a 64-wide
chunk the rect is *already clipped to the full chunk width*. Every
single-rect rule is full-width, so **no reach narrowing can pay while the
shape stays one rect**, whatever the reach is narrowed to. `cellloc`'s 4.8–5.7x
is entirely a *shape* prize wearing a reach label, and §6 should be read that
way.

So the fix needs the narrower shape, and §E2 is the whole of what stands in
front of it.

### §E2, bisected to a cell

**It had been bisected to a frame and never to a cell, and the reason was
mechanical: the switch was a process-wide `OnceLock`.** Two settings meant two
processes; two processes of a chaotic simulation are two different worlds; so
the only available comparison was a world hash, which says *that* two runs
differ and never *which cell*. `Chunk::sweep_rows` is now a per-chunk field
with `World::set_sweep_rows` beside it — shipped default unchanged, read from
the same environment variable at construction — which puts both arms in one
process, as `CLAUDE.md` asks for.

`examples/sweepgap.rs` then steps two `Lab`s on one spec and seed in lockstep,
one per rule, and diffs them every frame. On the shipped lab bed at seed 1:

```
FIRST DIVERGENCE at frame 237 -- 4 differing cell(s)
     x      y  arm A (box)   arm B    auxA   auxB   chunk  reach  distA  distB  in B?
   371    160         soil    soil     795    875   5,2       24      0      0    yes
   373    160         soil    soil     701    620   5,2       24      1      1    yes
   371    161         soil    soil     736    763   5,2       24      -      -     NO
   373    161         soil    soil     646    620   5,2       24      -      -     NO
```

**Every differing cell is `soil`, with the same material and the same organism
id in both arms, and a different `aux`. So §E2's divergence is in the
soil-moisture channel, not in what moved.** Nothing in the world has been
displaced differently; the water held in four soil cells has.

That is the second coupling §E2 records as unidentified, narrowed from "the
spans are not a superset of every cell the rules can act on" to a named
channel, four named cells, and **frame 237 instead of 4,330** — thirty seconds
of runtime instead of a long bisect. Two of the four sat outside arm B's own
narrowed region with no mark within one row of them; two sat inside it, which
is the half that says the narrowing alone does not explain it.

**What is not established**: why. The moisture pass walks
`Chunk::take_moist_plan`'s own region over *every* chunk, not the awake set,
and its seed (`pending_moist_rows`, unioned from the ordinary marks in
`end_sweep`) is identical in both arms — so the obvious route is ruled out and
the actual one is not found. Read this as a located fault, not a diagnosed
one.

### The controls, and one of them found a hole in the other

- **`arm=box`** runs both arms on the shipped rule and must show no
  divergence: **1,500 frames, none** — so a divergence under `arm=rows`
  belongs to the rule and not to the lockstep comparison.
- **Two classes of change are correctly outside every sweep region** and are
  the entire answer if not excluded: soil-moisture writes
  (`Chunk::set_world_quiet`, its own channel) were **82.85%** of the diff, and
  organism writes (a different frame phase) **2,090 `ant` + 977 `empty`** of
  the 2,871 that remained. Both are excluded at the seam rather than by
  material name.
- **The new guard
  (`set_sweep_rows_narrows_the_plan_and_a_chunk_born_afterwards_inherits_it`)
  is sensitive to one fault and measured blind to another.** Setter made a
  no-op → red (`walked 2013, box 2013`). `World::new_chunk`'s override
  dropped → **still green**. Recorded in the test itself as a known hole;
  `new_chunk`'s correctness rests on reading it, not on that green. And the
  guard's first geometry — two marks on one row — could not discriminate at
  all, because a row span is the *hull* of the marks on its row, so both rules
  gave the identical region and it read `walked 183, box 183` for a setter
  that was working perfectly.

### `parallel.rs`'s proof — verified, not asserted

§6 claimed the write-disjointness proof is untouched because it rests on the
bounding box and `touch_neighbours`' flat `MAX_REACH`. Checked:
`concurrent_chunks_are_never_within_reach_of_each_other` (`parallel.rs`) is
exhaustive **over chunk coordinates and the flat `MAX_REACH` alone** — it
reads neither `sweep_region`, nor `sweep_plan`, nor a chunk's tracked `reach`.
`parallel::step` consumes `chunk.sweep_plan()` and the module's own comment
states the box is what the proof is stated against. **The claim holds.** The
new guard asserts the bounding box is identical under both rules, so it stays
held.

### What this leaves

**No frame-cost figure, because there is nothing yet to time.** The change
that would earn one is the shape change, and shipping it means shipping a
known divergence. The honest state: the prize is real and re-confirmed, the
single-rect route to it is dead by arithmetic, and the blocker has moved from
"unidentified coupling, bisected to a frame" to "the soil-moisture channel,
four cells, frame 237, reproducible in thirty seconds". **The next job is
`sweepgap` pointed at why those four cells' moisture differs**, and it is now
a half-hour question rather than an evening.
