# Evolution lab — ecology-measure, the fruit → animal → nest → seedling loop

Lane brief: measure what the ecology loop actually does on the bed the owner
plays (`assets/lab_scenarios/played_bed.ron`), not on the frame-0 harness bed
every earlier windfall figure was taken on. This is the finding; the fixes to
the two binaries that made the measurement possible are the small half of the
work.

---

## 2026-09-10 — the loop barely closes, and the first-order reason is not reach

**Two binary fixes first, because they are why anything below is trustworthy
at all.** `windfall_probe` and `chronicle` both silently ignored
`scenario=`, `CLAUDE.md`'s own named gotcha — an unrecognised `arg()` key
parses to nothing, so `chronicle scenario=played_bed` ran the eight-founder
harness bed under the played bed's name with no warning, which is exactly
what happened to the coordinator. Both binaries now load the scenario the
way `labforage.rs` already does, refuse a bad name at load rather than
silently substituting the default bed, and name the scenario on their first
printed line together with every other parameter taken:

```
$ chronicle scenario=played_bed frames=2000
chronicle: founders=0 of grass colonies=0 seed=1 frames=2000 showing=LINES scenario=played_bed (...)
  scenario played_bed: 0 cells, 13 plants, 0 animals, 0 settings applied

$ windfall_probe scenario=played_bed frames=2000
windfall probe: 2000 frames, ... founders=0 colonies=0 seed=1 handout=0 scenario=played_bed (...)
  scenario played_bed: 0 cells, 13 plants, 0 animals, 0 settings applied
  founders 13/13 ants 0
```

**Proof the fix moved something, not just that it runs**: 13 founders
(the played bed's plant count) against the default bed's 8, and — the part
that actually matters for this round — **zero ants standing at frame 2,000**,
because the played bed founds its colony on its timeline at frame 6,000, not
at frame 0. Every number below comes from the fixed binary; every number any
earlier report quoted on `scenario=played_bed` before this fix was quietly
measuring the harness bed instead.

### The measurement

`scenario=played_bed`, seeds 1/2/3, 120,000 frames (the owner's own session
length — `windfall_probe`'s `milestones=` argument snapshots at 30k/60k/90k/
120k), `RAYON_NUM_THREADS=4` pinned, a private `TMPDIR` under this worktree.
Full logs: `.ecology_logs/seed{1,2,3}.log` in this worktree (not committed —
they are measurement output, not source).

| | seed 1 | seed 2 | seed 3 | pooled |
|---|---|---|---|---|
| flower+fruit organs built (cumulative) | 39 | 86 | 98 | 223 |
| windfall **produced** (`fruit_dropped`, ripe fruit let go) | 10 | 11 | 11 | 32 |
| windfall produced (`organ_shattered_to_windfall`, organ lost support) | 0 | 0 | 0 | 0 |
| mean standing windfall (of ~1,201 samples) | 0.040 | 0.032 | 0.047 | 0.040 |
| mean standing windfall **on the floor** | 0.022 | 0.010 | 0.027 | 0.020 |
| peak standing windfall ever | 7 | 3 | 2 | — |
| mean time a windfall stands (Little's law) | 480 f | 354 f | 518 f | 451 f |
| … on the floor only | 260 f | 109 f | 291 f | 220 f |
| germinations from windfall | 1 | 0 | 1 | **2** |
| germinations from a loose seed | 811 | 911 | 1,365 | 3,087 |
| total germinations | 812 | 911 | 1,366 | 3,089 |
| final live ants | 111 | 72 | 200 | 383 |
| births / deaths | 224 / 146 | 188 / 129 | 286 / 104 | 698 / 379 |

**Windfall standing at every one of the twelve 30k-frame checkpoints (three
seeds × four milestones): zero.** Not "small" — the milestone table's own
`windfall=0` column reads zero at 30k/60k/90k/120k on all three seeds. The
peak counts above (7/3/2) are transient blips that come and go inside a few
hundred frames — seed 1's peak of 7 happened around frame 8,000 and had
cleared before the 30,000-frame checkpoint. A reader who only ever samples at
round numbers — which is exactly what a played session looks like from the
outside — would report "the box has never once had windfall on the ground,"
which is true of the *sample* and false of the *process*: it is made, it
just never accumulates.

### The three-way fate verdict

**Two-way, not three, and that is a finding about the instrument, not a
shortcoming of this pass.** `pickups` and `eats` are one event in
`src/sim/creature.rs` (`:5039` pickup, `:5076` eat — "the two verbs merged
when the decision between them went away"), so from the world side an ant
taking a windfall cell off the ground is indistinguishable, at the instant
it happens, from an ant that will carry it somewhere and drop it later
(`:5147`/`:5149`). Both edits are off-limits this round (owned by another
lane), so `windfall_probe` now runs a per-4-frame diff over the floor band
(`windfall_floor_positions`/`ant_positions`) and reports what the world side
can actually tell apart:

| | seed 1 | seed 2 | seed 3 | pooled |
|---|---|---|---|---|
| **eaten-or-carried** (an ant was adjacent when it vanished) | 2 | 0 | 17 | 19 |
| **rotted** (became soil or nothing, no ant near) | 10 | 1 | 3 | 14 |
| unclear (neither test fired) | 5 | 6 | 6 | 17 |
| departures observed | 17 | 7 | 26 | 50 |

**The hook a later lane should add, precisely**: a material-keyed counter
beside `creature.rs:5039` (pickup) that increments once more when
`food == windfall`, and a second beside `:5147`/`:5149` (drop/delivery) keyed
the same way. That splits "eaten where it lay" from "picked up and carried"
cleanly, with no crop trace needed — the two events already exist, they are
just not counted by material. Nothing here builds toward that hook beyond
naming it; per the coordinator's cost-fork instruction, a half-built version
of it is worse than stopping at the world-side census.

**Positive control, because a zero here would otherwise be unreadable.**
`handout=1000` on seed 1, same scenario, 30,000 frames, drops fresh windfall
at the colony's own doorstep every 1,000 frames — the reach problem removed
by construction. Against the frame-30,000 checkpoint from the real run
(`eaten-or-carried=2`), the control reads **81**, mean floor stock rose from
0.02 to 0.50, and windfall-sourced germinations rose from 1 to 3. The
instrument is sensitive; the low numbers above are the world, not a probe
that never fired.

### The reach question — not the bottleneck it was assumed to be

Of standing flower + fruit, the share within an animal's reach of the ground
(within `FLOOR_BAND` = 3 rows, the same sense `windfall_floor` uses) swings
by seed far more than it swings by anything this round changed:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| mean standing flower+fruit within ground reach | 62.3 of 69.6 (**89%**) | 1.4 of 20.7 (**7%**) | 9.0 of 51.4 (**17%**) |
| highest an organ ever stood (rows above soil) | 48 | 61 | 57 |

**Seed 1 says reach was never the played bed's problem — 89% of its standing
crop sat within three rows of the ground the whole run**, because grass and
herb are short and the bed was deliberately built with no tree (the scenario
file's own reasoning: a tree shaded the bench to 0.008 of lamp light).
**Seeds 2 and 3 say the opposite just as loudly** — under a fifth of the
standing crop was ever reachable, because in those two draws the fruiting
herb and the shrub grew tall enough (highest organ 53–61 rows) to put most of
what they made out of an ant's way anyway. Three seeds is not a sweep, but
the spread itself is the finding: **whether reach gates this loop is a
property of the draw, not of the mechanism**, and a report taken from one
seed in either direction would have been confidently wrong about the other
two.

### The germination-source counter, and where it's going

`World::windfall_germination_x` is new this round (`src/sim/world.rs`,
incremented in `src/sim/plant.rs`'s `germinate()`, keyed on the seed cell's
*material* before it is relabelled into a shoot — `cell.material == windfall`
distinguishes a parcel from a scatter, and both share the same `CellType::
Seed` and the same `germinate()` call, so nothing else can tell them apart
after the fact). Pooled: **2 of 3,089 germinations — 0.065%** — came from a
windfall. The bin closest to the nest column (256) that ever received one is
64–96 columns away (seed 1); seed 3's lands at 160–192. Every other bin,
including the ones nearest the nest, is empty. `handout=`'s control run
(above) put two more windfall germinations at 96–128 and 128–160 — still not
near the nest, because a handed-out cell falls where it's dropped and then
follows the same rules as any other windfall from there.

**So the loop the round is named for — fruit → animal → nest → seedling —
is not closing through windfall at all, in any of the three draws**, and the
loose-seed path (`plant::set_seed`, hardcoded to `seed` material, never
`windfall`) is carrying essentially the entire germination load regardless
of whether a colony exists on the bed. Two germinations in 360,000 combined
frames is not "rare", it's "hasn't been observed to matter yet."

### What surprised me: windfall standing that neither production counter can see

**`World::fruit_dropped` (deliberate drop) plus the new
`World::organ_shattered_to_windfall` (structural failure converting a
standing fruit/flower via `fruit.ron`'s `breaks_into: "windfall"`) do not
account for every windfall cell this run.** `windfall_probe`'s own fate diff
makes the gap visible without any extra instrumentation: seed 1 counted
**17 departures against only 10 windfalls ever credited to either producer**
— seven more cells left the floor band than either counter says were ever
made. Seed 3 shows the same shape (26 departures, 11 credited).

Ruled out, each checked directly rather than assumed:

- **Not a material-id collision.** `windfall`, `seed`, `litter` and `soil`
  resolve to four distinct `MaterialId`s (37/18/27/13), confirmed by a debug
  print (`WF_DEBUG=1`) before trusting anything downstream of it.
- **Not `structural::break_free`.** Instrumented directly
  (`organ_shattered_to_windfall`) and it reads zero on every seed.
- **Not decay.** Nothing in `assets/materials/*.ron` decays into `windfall`;
  `windfall` itself decays into `soil`.
- **Not the player's `shake`.** That path needs a live player and only ever
  converts `Leaf` cells; this box has no player and the cells in question
  are `Seed`.
- **Not `plant::set_seed`** (the loose-seed path) — it is hardcoded to
  `"seed"` material and can never produce `"windfall"`.
- **Not seed decay's `shed_to_litter`** — that path writes `litter` material
  at the landing cell, never `windfall`.
- **Not the scenario file or worldgen** — `played_bed.ron` places no `Heap`
  and no pre-existing windfall; grepped and confirmed absent.

What is left, from a `WF_DEBUG=1` trace (`examples/windfall_probe.rs`, kept
in — the same reasoning `SNAP_PROBE` in `structural.rs` gives for keeping a
single-cell camera rather than only a counter): the cell is *already*
`organism_id=0` the first time it is seen inside the floor band, wearing
`CellType::Seed`, `windfall` material, tens of columns from the nearest
fruiting herb — a distance consistent with the rolling `windfall.ron`
describes (`friction_angle: 26`, against `seed`'s 55) but not with a straight
drop. **This means whatever is happening detaches the seed's organism
identity before the cell is observed, without deleting it, converting its
material, or crediting either production counter** — and if that is
happening in general, not only on this bed, it would also explain why
windfall-sourced germination is so rare above: `germinate()` has exactly one
call site and it is reached only through organism-scheduled dispatch, so a
windfall that has already lost its organism cannot be scheduled to germinate
at all, ever, regardless of where it lands. I did not find the line that
does this in the time this round had — the generic movers (`try_move`,
`roll_along_slope`, `fall_through_organism`) all route through
`CellSurface::move_cell`, which is far too well-trodden (every falling
powder in the game, plant and mineral alike) to selectively drop one
organism's identity without failing constantly and visibly. **Reproduction**:
`WF_DEBUG=1 windfall_probe scenario=played_bed seed=1 frames=1300 sample=10
fate=1` — the trace prints at frame 1,184, cell (456,158). Whoever owns the
powder-movement code next should start there rather than re-deriving the
ruled-out list above.

### What I could not measure, and why

- **Eaten vs. carried, split.** Covered above — needs a `creature.rs` edit
  this round does not own. World-side census stops at "eaten-or-carried."
- **The organism-identity leak's exact call site.** Sized the residual (7
  and 15 cells respectively on seeds 1 and 3, against 0 on seed 2 where
  production and departures roughly balance) and ruled out every mechanism
  this file's own greppable inventory names, but did not find the line.
  Flagged above with a reproduction rather than left as a bare number.
- **A true "flowers set" / "fruit set" split as two cumulative counters.**
  `World::organs_built` counts flower construction and the flower→fruit
  relabel together, by design (its own doc says so), and no counter splits
  them. I report standing stock by organ type (`flower`/`fruit` columns
  above), which is a different question — "how much is there now," not
  "how many were ever made" — and said so rather than inventing a split
  that does not exist.

## Files touched

- `examples/windfall_probe.rs` — `scenario=`, parameter echo, the fate diff,
  the organ-floor-reach census, the germination-source histogram, `WF_DEBUG`.
- `examples/chronicle.rs` — `scenario=`, parameter echo.
- `src/sim/world.rs` — `windfall_germination_x` (positions, not a count
  alone), `organ_shattered_to_windfall`.
- `src/sim/plant.rs` — the one line in `germinate()` that tells a parcel from
  a scatter.
- `src/sim/structural.rs` — the far-side counter for a fruit/flower organ
  converting to windfall through `break_free`'s existing `breaks_into` path.

None of `src/sim/creature.rs` or `src/sim/organism.rs` were touched, per this
round's file-ownership split.
