# The shut-in probe, and what it shows on seed 1 (Deep trace, 2026-10-07 07:00 UTC)

Asked by Nest race: are ants read as "shut in" while the door is open? Under NEEDS_FIRST, an ant under its grant that
reads shut in counts as hungry. It then puts its pellet down where it stands and quits its trip.

- **Run:** b30smell seed 1 (heap 30), to 75k.
- **Labels:** measured unless a line says inferred.
- **No fixes:** I propose none.

## In one paragraph

On seed 1 the reading is stale, not the door. The nest's door was reached on every one of the 2,300 rebuilds of the
way out. Yet 97% of the hungry soil drops by ants above half a grant (1,739 of 1,789) were by ants that a way built at
that moment would **not** have read as shut in. The cause is how the reading is kept:

- the way out is rebuilt every 30 frames;
- it is built only over cells an ant could stand in at that moment;
- the crowd in the room moves faster than that.

So an ant standing on the crowd, or in a cell just dug, reads shut in until the next rebuild. The same misread is
behind at least 2,084 of NEEDS_FIRST's 2,272 trip quits. The real cases were 2 ants sealed in a small pocket by the
deep food at 62-75k. Both starved there.

## The probe (measuring only)

**Patch:** `tools/shut-in-probe-95d65cd66.patch`.

- It is a `git format-patch` against 95d65cd66.
- It also applies cleanly on `claude/nest-race-store-stack` (26752d47d).

**In creature.rs:**

- `under_cover`, `way_cell` and `shut_in` are made `pub`. That is visibility only.
- `way_open` is added: way_cell's open half, with no footing asked. Nothing in the engine calls it.

**In deeptrace.rs, with `dig=1`.** `digrows.csv.gz` gains five columns at the end:

| Column | What it says |
|---|---|
| `shut_in` | The engine's own `shut_in` at the head the ant decided from. Read after the step, on the same cached way the decision read. Blank when no way out is built. |
| `cover` | `under_cover` at the head. |
| `way_then` | Could an ant stand in the head's cell, in the world the cached way was built from? |
| `air_then` | Was that cell joined to the open air at all, through open cells (empty, ant, or brood), with footing ignored? 0 means a pocket sealed off from the sky. |
| `shut_now` | Would a way built now, after the step, read the ant shut in where it now stands? Only filled on rows that read shut in. |

**Other outputs:**

- `hungry.csv.gz` gains `shut_in` and `cover`.
- New `outway.csv` has one line per nest per rebuild, with these columns:
  - `match`: cells where the snapshot and the engine's cache disagree;
  - under cover, the cells that are standable, reached, open, and joined to the air;
  - `shaft_way` and `shaft_reached`: the founding shaft's standable cells, and how many the way reached.

**Timing.** The engine rebuilds at world frame `f + 1` when that is a multiple of 30, where `f` is digrows' frame. So
a row at `f` was read against the world as it stood at the end of digrows frame `30 * floor((f + 1) / 30) - 1`.

## Checks (all measured)

**It changes nothing.** The following were byte-identical to the same build without the probe:

- `stats.csv`, `events.txt`, `ledger.csv`, `cells.csv.gz`, `cuts.csv`, `broodlog.csv`, `colony.csv` and the maps;
- every old `digrows` and `hungry` column.

Wall time at 70k went from 258 s to 296 s.

**The snapshot is the world the engine read.** `match` was 0 on all 2,300 rebuilds.

**Positive control.** At 0.5 to 1 of a grant, only `shut_in` can fire a need drop: `job` and `laden` need a full grant
or food in the crop. Of 1,814 such drops:

- 1,789 read `shut_in` = 1;
- 22 of the other 25 sit on the lean line (row energy 0.500, printed to 3 places), so they are lean-line drops;
- 3 are unexplained (0.2%). Inferred: the roof changed during the step, since `shut_in` is read after it.

## What it shows on seed 1 (measured)

### 1. The door was never cut off

The founding shaft was reached by the way out on all 2,300 rebuilds. Under cover, the way reached 444-701 of 458-714
standable cells (means per window).

### 2. The need drops above half a grant were nearly all misreads

| Need drops at 0.5-1 that read shut in | Count | Share |
|---|---|---|
| Nothing to stand on at the last rebuild, but joined to the open air; a way built now does not read it shut in | 1,357 | 76% |
| Not open at the last rebuild; a way built now does not read it shut in | 381 | 21% |
| Really sealed: standable, no way to the air, and a way built now agrees | 50 | 3% |
| Other | 1 | 0% |

**The 1,357 on a cell with nothing to stand on:**

- 1,144 had no ground in the 8 cells round the head. They were standing on other ants.
- All but 2 were in the nest.
- 900 were nest workers.

**The 381 on a cell that was not open:**

- For 185, `cells.csv` shows the cell dug or slid open since the rebuild. 112 of those were soil to empty, unattributed:
  slid or fell.
- The other 196 are not in `cells.csv`, which records only ground. Inferred: food, crumbs or a corpse stood there then.

**The misreads by window:**

| Window | Misread drops |
|---|---|
| 0-40k | 730 |
| 40-55k | 502 |
| 55-60k | 180 |
| 60-65k | 270 |
| 65-70k | 8 |
| 70-75k | 49 |

### 3. Trip quits

Rows at 0.5-1 that read shut in while the ant had a face to walk back to (`dig_return` set) each clear it, which counts
as one `needs_quit`.

- 2,084 such rows were misreads and 49 were real.
- The game's own counter reads 2,272 by 75k. So at least 92% of NEEDS_FIRST's trip quits on seed 1 were the misread.
- It is a floor, since store trips are not counted.

### 4. The real ones: 2 ants sealed by the deep food

- All 50 real drops, and every real sealed row in the nest after 62k, were 2 ants. They were in a pocket at x 239-245,
  rows 198-204.
- They cut and dropped in place from 62.5k to 72k. Both starved in the nest.
- At 70-75k they also made 29 of the 50 cuts by ants under half a grant. Those cuts were let through because the ants
  read shut in, which is NEEDS_FIRST's `weak` part.

**Correction to my lobedrops note.** The 65-70k "churn by the deep food" on seed 1 (41 of 44 drops) is these 2 sealed
ants. My earlier inference that those were misreads was wrong for that window. The misreads are the bulk of the other
windows.

### 5. A counter to read with care

`needs_weak_digs` counts lean ants let past the no-dig gate because they read shut in. It does not count cuts.

- On seed 1 it reads 1,244 by 75k (0 to 60k, then 199, 566 and 479 per 5k).
- Only 50 of those became cuts. The rest lost the roll, had no ground ahead, or were not asked.

The other two escape parts also read `shut_in`, at any energy:

- `needs_cue_waived` counts the heap cue waived on a cut that would open to the sky: 4,360 by 75k.
- `needs_face_waived` counts the walk-back's reach waived: 174 by 75k.
- I have not split either by the probe.

### 6. Fed ants in sealed pockets are common and harmless here

68% of all rows reading shut in were ants at a full grant in real pockets:

- inside the spoil mound;
- under the food heap. The heap is in the way's box, and food overhead counts as cover.

NEEDS_FIRST does nothing to a fed ant for that.

## What it means (inferred, not traced)

The way out is a 30-frame-old picture of where ants could stand, and the room's crowd moves within it. An ant on the
crowd, or in a cell just dug, reads shut in with the door open. NEEDS_FIRST then counts it as hungry at any energy under
its grant:

- it puts its soil down where it stands, in the room;
- it quits its trip.

On seed 1 that is most of NEEDS_FIRST's drops and quits above half a grant.

**Not shown:** whether this costs the colony food or lives. That needs an arm with the reading changed, and it is not a
proposal from me.

## For heap 90, seed 4 (Nest race's rerun)

1. Apply the patch to 95d65cd66, then build with
   `set -o pipefail; cargo build --release --example deeptrace`. Check the binary changed.
2. Use the same args, with `dig=1 hungry=1`.
3. If you cut the decision log to a box, also keep every row with `spoil_why` = need or `shut_in` = 1, wherever the head
   is. Keep `outway.csv` whole; it is small.
4. Read it with:
   - `tools/shutprobe.py RUN --windows 118000-123000,123000-128000,128000-133000,133000-138000`;
   - `tools/shutwhy.py RUN --windows ...`.
   First check that `match` is all 0 and that the positive control passes. Each takes about 40 s on a 7-million-row log.

**What would settle the s4 question:**

- the drops in the store lobe at 0.5-1, split by `shut_now`;
- the lobe cutters at 0.52-0.66, split by `shut_in`. Above half a grant an ant digs whatever it reads. A shut-in
  reading only waives two things: the heap cue on a cut that would open to the sky (`breakthrough`), and the walk-back's
  reach (`door`). So the lobe cutting is ordinary digging unless those waivers show up there;
- whether any of the store lobe is really sealed (`air_then` = 0 and `shut_now` = 1).

## Caveats

- `shut_in` and `cover` are read after the step on the same cached way. The roof can change by a cell during the step.
- `shut_now` is read where the ant stands after the step, on a way built from the world after the step.
- The air flood is bounded by the engine's own box, as the engine's way is.
- The patch was laid out by rustfmt after the test build. The changes are layout only, and the laid-out version builds
  clean under `cargo clippy --release --locked --example deeptrace -- -D warnings`.

## Files

**Tools, in `store-arms/sky-meal/tools/`:**

- `shut-in-probe-95d65cd66.patch`;
- `shutprobe.py`: the timing check, the positive control, and the probe's split by window;
- `shutwhy.py`: the control misses, the cells changed since the rebuild, misread against real, trip quits, and where
  the real pockets are.

**Tables:** `store-arms/sky-meal/shut-in-probe-s1-tables.txt`, the full output of both tools on seed 1.

---

# v2: does the way out run over the crowd? (Deep trace, 07:20 UTC)

Nest race found the heap-90 seed 4 starvers falling while they crossed the open middle of a big room, mostly on the
walk-out pull. They asked what the fallers stand on, and why the route sends them across the middle rather than along a
wall.

## The code (read at 95d65cd66)

**How the walk-out pull aims:** it aims 3 steps down the nest's way, via `hungry_out_pull`, then `way_out_from`, then
`step_down_way`. Every way a pull walks is built the same way:

- the nest's way;
- the store field;
- the soil's way out.

**How those ways are built:**

- They are built every 30 frames, breadth first, over `creature::way_cell`.
- A way cell is open, with ground, a plant **or an animal** in its 8 neighbours ("in a crowded room ants stand on ants").
- So the shortest way to the door can run up over the crowd. Being shortest is all it is chosen for.

## The probe's three new columns

| Column | What it says |
|---|---|
| `ants8` | Other animals' cells in the 8 round the head, before the step. |
| `pull_then` | What the target of the pull the ant was scored with offered to stand on, in the world the ways were built from. |
| `pull_now` | The same, after the step. |

**The footing codes:**

- `g`: ground or a plant beside it;
- `a`: only an animal beside it;
- `o`: open with nothing beside it;
- `x`: filled.

After the step the ant may stand in its own target. Its own body then counts as the animal beside it, so read
`pull_now` with care.

## Checks (measured)

- **It changes nothing.** Every output and every v1 column is byte-identical to v1 on b30smell seed 1 to 75k.
- **Positive control.**
  - Walk-out, soil-way and store targets are all `g` or `a` when the ways were built. By construction they are way cells.
  - Only the walk back to the face shows `o` and `x`. It aims at a face, not along a way.

## Seed 1, nest zone (measured)

### What the fallers stand on

| | Ground round the head | Only ants | Nothing |
|---|---|---|---|
| Rows that fell (124k) | 0.2% | 93.6% | 6.2% |
| Other rows (1.12M) | 36.4% | 58.1% | 5.5% |

### Where the routes lead

**How often the target stood on ants alone, whole run:**

- walk-out: 38% of targets;
- soil way out: 36%;
- store field: 18%.

At 60-70k the walk-out's share was 54-57%, against 14-16% at 40-60k.

### Stepping toward an ant-held target is followed by a fall

The table gives the share of steps toward the target whose next decision was a fall.

| Pull | Target on ants (`a`) | Target by ground (`g`) |
|---|---|---|
| Walk-out | 22.0% (n 64,703) | 1.2% (n 106,000) |
| Soil way out | 23.6% (n 104,678) | 1.5% (n 189,548) |
| Store field | 12.3% (n 7,530) | 3.1% (n 34,606) |

**It holds within the ant's own footing.** For the walk-out:

- ant on the crowd: 25.4% against 2.0%;
- ant at a wall: 3.1% against 0.1%.

The soil way out is the same: 27.6% against 2.6%, and 4.4% against 0.2%.

### The 53 ants that starved at 60-75k, in their last 5,000 frames

- 87% of their walk-out targets (17,505 of 20,141) were held up only by ants.
- 30.0% of their steps toward one were followed by a fall, against 8.5% toward ground.
- The last scored pull before 92% of their 13,455 falls (12,360) was the walk-out toward an ant-held target.

## What it means

**Measured:** on seed 1 the walk-out route took the hungry ants over the crowd, and they fell back.

**Inferred, not traced:** the route runs over the crowd because the way counts an ant as footing, and breadth first
takes the shortest way.

**Untested:** whether a way built over ground alone would have a route along the walls in these rooms, and how much
longer it would be. A rule change built on this needs that answered first, or ants with no wall route would read as
having no way out. I can add that as a probe column if wanted.

## Files

**Patches, in `tools/`:**

- `shut-in-probe-v2-95d65cd66.patch`: v1 and v2 as a `git am` series on 95d65cd66. It also applies on 26752d47d.
- `shut-in-probe-v1-to-v2.patch`: v2 alone, on top of v1.

**Reader:** `tools/routefoot.py RUN --windows ...`.

**Tables:** `shut-in-probe-s1-v2-tables.txt`.

---

# v3: would a way over ground alone still lead out? (Deep trace, 07:30 UTC)

Nest race asked this before proposing that the ways count only ground or a plant as footing. Two questions:

- from where the ants are, does such a way reach the door?
- how much longer is it?

## What the probe adds

**The rebuilt way.** The probe rebuilds the nest's way in (the way the walk-out pull follows), the way `build_nest_way`
does:

- from the door;
- over the cells inside the nest;
- in the engine's own box;
- from the same snapshot.

It does this twice:

- with the engine's footing (`rep`), which must equal the engine's cache;
- over cells with ground or a plant beside them only (`ground`).

**New columns in `digrows`:**

| Column | What it says |
|---|---|
| `way_d` | The head's steps from the door on the engine's nest way. |
| `wall_d` | Steps on the ground-only way from the best cell within 3 of the head, plus the steps to that cell. Blank: no ground-only route within 3 cells. |

**New file `wallway.csv`,** one line per nest per rebuild:

- `match`;
- the cells on each way;
- the deepest step on each;
- the median and p90 of ground steps over way steps.

## Checks (measured)

- **It changes nothing.** Every output and every v2 column is byte-identical.
- **The rebuild with the engine's footing is the engine's cache.** `match` = 0 on all 2,300 rebuilds, cell for cell. So
  the ground-only way is built on the same frame, box, door and inside rule as the real one.

## Seed 1 (measured)

### Per rebuild (window means)

| Window | Cells on the way | Over ground alone | Deepest step (way / ground) | Ground over way steps: median, p90 |
|---|---|---|---|---|
| 40-55k | 564 | 446 | 47 / 62 | 1.00, 1.11 |
| 55-60k | 668 | 528 | 51 / 67 | 1.19, 3.05 |
| 60-65k | 841 | 624 | 52 / 72 | 1.44, 3.52 |
| 65-70k | 809 | 650 | 53 / 58 | 1.00, 1.00 |
| 70-75k | 761 | 667 | 54 / 61 | 1.00, 1.03 |

The cells on the way that the ground-only way does not reach (20-26%) are the cells held up by the crowd.

### Per row in the nest, rows on the way

| Where the ant stands | Ground route within 3 cells | Ground over way steps: median, p90 |
|---|---|---|
| Ground beside the head | 99.3% | 1.00, 1.12 |
| Only ants beside | 94.8% | 1.00, 1.55 |
| Nothing beside | 99.4% | 1.00, 3.19 |
| Walk-out pull, on the crowd | 96.2% | 1.00, 1.59 |
| Walk-out pull, on the crowd, 60-65k | 92.4% | 1.20, 2.39 |

### The 53 ants that starved at 60-75k, in their last 5,000 frames

| Where the ant stands | Ground route within 3 cells | Ground over way steps: median, p90 |
|---|---|---|
| Ground beside the head | 97.4% | 1.08, 1.30 |
| Only ants beside | 89.1% | 1.17, 1.47 |
| Walk-out pull, on the crowd | 91.4% | 1.17, 1.51 |

## What it means for a ground-only rule

**Measured on seed 1:**

- A way over ground alone still reaches the door from 89-99% of the positions the ants were in.
- It is about the same length for most rows: a median of 1.0-1.2 times the steps, and p90 1.3-1.6 times. It runs up to
  about 3.5 times longer for the far cells of the room at 55-65k.
- At 60-65k the deepest cell is 72 steps out instead of 52.

**The risk, measured:** 9-11% of the starvers' rows on the crowd had no ground-only route within 3 cells. Those are ants
in the middle of the crowd.

- Off the way, `step_down_way` gives no target, so the walk-out pull would give them no direction at all.
- They would have to fall or walk to the floor or a wall first.

**Not measured:**

- whether such a rule would cost the colony anything else. Every pull that walks a way would change: the store field,
  the soil way out and the rest pull too.
- seed 4 at heap 90, where the rooms are bigger.

## Files

**Patches, in `tools/`:**

- `shut-in-probe-v3-95d65cd66.patch`: v1, v2 and v3 as a `git am` series. It also applies on 26752d47d.
- `shut-in-probe-v2-to-v3.patch`: v3 alone, on top of v2.

**Reader:** `tools/wallroute.py RUN --windows ... [--starved-from F --starved-to F --last N]`. The first line is the
`match` check.

**Tables:** `shut-in-probe-s1-v3-tables.txt`.

# v4: where the walk-out would aim with the way weighted (Deep trace, 08:20 UTC)

For Nest race's WAY_FOOT proposal (`nest-race/way-over-ground-proposal-2026-10-07.md`): a step into a cell held up only
by an animal costs K instead of 1. My review is `nest-race/way-over-ground-review-deep-trace-2026-10-07.md`. This
answers its Q3, which K. Measuring only.

## What the probe adds

Each rebuild, the probe builds the nest's way again from the same world at K = 1, 2, 4 and 8: the cheapest cost from
the door, over the same cells. Then, for every row whose pull walks the nest's way down (the walk-out, and the soil
way out), it walks `step_down_way` on each one from the ant's head, in the ant's own neighbour order.

| Where | Column | What it says |
|---|---|---|
| digrows | `k2_then`, `k4_then`, `k8_then` | What held up the cell the walk would aim at, at that K: `g` ground or a plant beside it, `a` only an animal |
| digrows | `k1_hit` | The control: 1 when the probe's walk at K = 1 aims where the engine's did |
| `wallway.csv` | `val_diff` | Cells whose step count in the probe's rebuild differs from the engine's cache |
| `wallway.csv` | `k1_diff` | Cells where the weighted way at K = 1 differs from that rebuild |
| `wallway.csv` | `reach_diff` | Cells on the way at K = 8 and not on the rebuild, or the reverse |

## Checks (measured, all passed)

- **Same game as v3.** `wallroute.py` on this run prints the v3 tables exactly.
- **The rebuild is the engine's cache, values included:** `val_diff` 0 on all 2,300 rebuilds.
- **The weighted way at K = 1 is that rebuild** (`k1_diff` 0), and at K = 8 it reaches the same cells (`reach_diff`
  0). So weighting changes no ant's having a way, only where it leads.
- **The probe's walk is the engine's.** At K = 1 it aims exactly where the engine did on all 475,571 rows: 172,400
  walk-out and 303,171 soil way.

## Seed 1, nest zone, to 75k (measured)

**Walk-out aims on a cell held up only by ants:**

| The ant stands | Rows | K = 1 (today) | K = 2 | K = 4 | K = 8 |
|---|---|---|---|---|---|
| On the crowd (only ants round its head) | 114,945 | 47.5% | 19.1% | 6.7% | 4.8% |
| At a wall (ground round its head) | 47,186 | 16.4% | 2.8% | 1.2% | 1.0% |
| The 53 ants that starved at 60-75k, last 5k frames, on the crowd | 18,003 | 89.7% | 43.9% | 11.9% | 8.3% |
| The same ants, at a wall | 1,846 | 67.8% | 17.8% | 6.1% | 5.6% |

**The soil way out, on the crowd** (185,272 rows): 46.4% today, then 22.0%, 9.2% and 5.6% at K = 2, 4 and 8.

- **K = 4 moves 87% of today's ant-held walk-out aims onto ground.** 33.0% of all walk-out rows go from an ant-held
  aim to a ground one; 4.9% stay ant-held. Under 0.5% go the other way.
- **The window with the most starving keeps the most.** At 60-65k the walk-out from the crowd goes 66.4% today, then
  32.5%, 14.7% and 10.0%.

## What it would do to falls (inferred, not measured)

Suppose a re-aimed ant fell next as often as an ant does today on the same footing, aiming at the same kind of cell.
Then the next decision after a walk-out step from the crowd would be a fall:

| | K = 1 (today) | K = 2 | K = 4 | K = 8 |
|---|---|---|---|---|
| All rows | 13.1% | 6.4% | 3.5% | 3.1% |
| The starvers' last 5k frames | 23.0% | 12.2% | 4.8% | 3.9% |

That rests on three things the probe cannot show:

- **The re-aimed ant falls like today's ground-aimers.** Those were already near a wall.
- **The crowd stands where it stands today.** Under the arm it would move.
- **The brain follows the aim as often as it does today.**

## What it means for K

- K = 2 does about half of it, K = 4 most of it, and K = 8 little more than 4.
- So the sweep can be K = 2 and K = 4. Run K = 8 only if K = 4 leaves a residue in the arms.
- Not measured: whether the extra traffic jams the walls. A weighted way cannot see that. The arms can.

## Files

**Patches, in `tools/`:**

- `shut-in-probe-v4-95d65cd66.patch`: v1 to v4 as a `git am` series. It also applies on 26752d47d, checked patch by
  patch.
- `shut-in-probe-v3-to-v4.patch`: v4 alone, on top of v3.

**Reader:** `tools/kshift.py RUN --windows ... [--starved-from F --starved-to F --last N]`. Its first five lines are
the checks.

**Tables:** `shut-in-probe-s1-v4-tables.txt`.
