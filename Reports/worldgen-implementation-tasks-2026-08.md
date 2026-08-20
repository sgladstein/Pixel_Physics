# Worldgen data-track tasks, August 2026 — for the implementation session

You are the implementation session for the worldgen data track. Your task
queue comes from the world review (`Reports/world-review-2026-08.md` — read
its §1, §5 and **all of §7** before touching anything; §7's landmines are
quoted per-task below because every one of them has already cost this
project real time). The review was run by a planning session that remains
the reviewer of this work: **you land small, image-backed commits; you do
not judge your own visuals.** When a spec below does not survive contact
with the code, stop and write a finding into this file's Findings section
instead of improvising.

## Ground rules (non-negotiable)

- **Branch**: work on `claude/worldgen-data-track`, branched from
  `claude/game-world-gen-planning-h12713`. Push with
  `git push -u origin claude/worldgen-data-track`. Land task-by-task —
  one task, one commit (two if a task splits naturally). Commit messages
  carry the numbers (counts before/after, sweep results), per repo
  convention.
- **Files you own**: `src/worldgen/*`, `assets/worldgen.ron`,
  `assets/materials/*.ron`, `tests/worldgen.rs`, plus one new script in
  `scripts/`. **Files you must not touch**: `src/render.rs`,
  `src/sim/sky.rs`, `src/sim/field.rs`, `src/sim/parallel.rs`,
  `src/sim/load.rs`, `src/sim/structural.rs`, `src/sim/rigid.rs`,
  `examples/ascii.rs`, and the contested files (`src/app.rs`, `PLAN.md`,
  `README.md`, `CLAUDE.md`, `wiki/*` — wiki updates are folded in by the
  reviewing session at merge). If a task seems to need one of those
  files, that is a finding, not an edit.
- **Stage explicit paths only. Never `git add -A`** (§7.11 — it once swept
  1,200 lines of someone else's work into an unrelated commit).
- **Before every commit**: `cargo test`, `cargo clippy --all-targets --
  -D warnings`, and confirm the at-rest suite (`cargo test --test
  worldgen`) is green. CI also runs `cargo run --release --example ascii`
  — do not land anything that regresses its worst-frame lines.
- **Every visual change ships with images**: render before/after strips
  with `cargo run --release --example viewshot -- seed=N preset=P
  shots=4 out=target/filmstrips/task<K>-<label>.png` and say in the
  commit message which strips to look at. A green suite is not evidence
  the screen changed (§7.15).
- **`.ron` edits do nothing until rebuild** (`include_str!`, §7.2).
  Identical output across sweep settings means the knob was never
  connected — rebuild between sweep points.
- **Registries are append-only** (§7.5): material ids in
  `material.rs::EMBEDDED` order and `noise::Purpose` discriminants must
  never be renumbered. New noise streams append a new `Purpose`.
- **Determinism** (§7.6): no iteration-order-dependent behavior
  (`BTreeMap`/sorted `Vec`, never an iterated `HashMap`), no wall-clock,
  all randomness through `worldgen::noise` with a `Purpose`.
- **Generated terrain must arrive at rest and sleep** (§7.3):
  `tests/worldgen.rs` enforces zero cells moving in 120 frames and
  `active_chunk_count() == 0` within 45 frames, every preset × 5 seeds.
  Nothing you place may violate either.
- **Only three GLOBAL passes are allowed** and a test pins the list
  (§7.4). Any new pass declares a finite column margin.
- **When a fix changes what a number means, re-derive the constants that
  read it** (§7.14) — re-running the task-2 sweep after each landed task
  is how.

## Task 1 — Step-0 measurements (findings only, no behavior changes)

Three open sightings from the review need one-line answers before other
work builds on the area. Deliverable: entries in the Findings section of
this file (commit them), plus any throwaway probes as `#[test]`s or
`filmstrip` invocations documented there — no shipped behavior change.

1a. **The blue slivers.** `canyon` seed 1 generated **zero** pond cells
    (per-pass counter), yet the rendered strip shows a 1–2-column blue
    sliver at world x≈920 in the canyon notch (and another at arid-s1
    x≈1215 on a dune). Census the actual cells there: generate
    `canyon`/`arid` seed 1 at 2048×640 (`WorldgenPresets` + `Spec::
    Generated`, as `examples/viewshot.rs` does), and print
    material/aux for the columns in question (a small `#[test]` with
    `println!` run via `cargo test -- --nocapture`, or `filmstrip`'s
    `dump=x,y,w,h`). Answer: is it a `Liquid` cell (then ponds leaked
    below `pond_min_width` — where?), moisture shading, or the gnome
    sprite? If it is real water, find the writer (only `ponds` writes
    water at genesis) — finding only, no fix.

1b. **The keyhole slots.** 1–2-column vertical slots cut the full height
    of cliffs: canyon seed 7 x≈600–620, canyon seed 13 x≈205, rolling
    seed 1 x≈1295–1335, rolling seed 2 x≈1465–1520. The review's suspect
    is `column.rs::terraced()`: the terrace snap is full-strength where
    the mask `smoothstep(0.62, 0.82, …)` saturates, and the mask *edge*
    can flip a single column between snapped and unsnapped ground,
    cutting a one-column cliff. Confirm or refute by printing the
    elevation chain terms (base/hill/terraced/detail) for the columns
    around one slot (pure functions — a unit test can call them
    directly, see the purity tests already in `column.rs`). Record which
    term steps and by how much. Finding only — the fix design belongs to
    the reviewing session because it interacts with the erosion design.

1c. **Wetland "white dashes".** The review concluded the pale dashes on
    pond surfaces are shoreline sand + `fill_dimming: 0.0`, not a water
    artifact. Confirm the material census on one pond rim (rolling seed
    1, the pond at x≈780–960): print the top-row materials across the
    waterline. One line in Findings settles it forever.

## Task 2 — 16-seed worldgen census sweep (`scripts/worldgen_sweep.sh`)

The repo rule (§7.9, and CLAUDE.md "build the sweep *before* changing a
model that governs procedural content"): worldgen is procedural content,
so a sweep with order-statistic gates must exist before tasks 3–6 land.

Build `scripts/worldgen_sweep.sh` modeled on `scripts/seedsweep.sh`
(read it first): all six presets × seeds `1..=16` through
`cargo run --release --example filmstrip -- scene=worldgen preset=$p
seed=$s count=1 out=$TMP/...` — **one build, then binary runs** — parsing
the per-pass cell-count table each run already prints. Output: a
tab-separated baseline file (`scripts/worldgen_sweep_baseline.tsv`,
committed) with per-preset **p90 and max** for every pass counter, plus
the awake-chunk line. Print a comparison mode
(`worldgen_sweep.sh compare`) that re-runs and diffs against the
baseline, flagging any per-preset p90 that moves more than ±30% —
flagged means *look*, not *fail*; the point is that no future change to
worldgen can silently zero a pass (the brows/talus blindness) or explode
one. Keep total runtime sane: 96 runs of a 512×320 generation is the
cheap size; do not sweep at 2048×640.

Verify: run it twice back-to-back — identical output (determinism);
deliberately set `tree_density: 0.0` locally, run compare, confirm
`life_scatter` flags, revert.

## Task 3 — Region-keyed palettes (regions become biomes, step 1)

Today every region of every preset is one gray stone, one soil tone, one
strata style (`passes.rs::strata_shade()` takes no `Character` input;
`Ctx` holds one stone id). Route region `Character` into the *shades*
worldgen bakes at genesis — zero frame cost, since `render.rs` colors
from `Cell::shade` and never recomputes (§7.21: palettes bake at genesis;
never key a color to a live field read).

- In `region.rs`, expose the blended `Character` at x (a method already
  interpolates for elevation — reuse that path, do not re-derive
  blending).
- In `passes.rs`, make `strata_shade` and the soil/sand shade profiles
  take the local `Character` and shift their *base tone* along two axes:
  `aridity` (dry = warmer/paler rock and sand, wet = darker/richer soil)
  and `resistance` (resistant = paler cap-rock bands). Keep the existing
  per-cell jitter and band structure — this shifts family, not texture.
  The mapping stays inside the shade ranges the material palettes
  support; if the 4-entry palettes clamp too hard to show a family shift,
  widening a palette in `assets/materials/*.ron` is in-scope (data-only,
  rebuild after, §7.2) — but do NOT touch how `render.rs` consumes shade.
- Do not change which *materials* are placed in this task (that is the
  erosion/formation round) — tones only.

Verify: before/after strips of all six presets at seeds 1 and 7
(12 images) — the reviewer judges whether crossing an escarpment now
reads as entering different country; `cargo test` (determinism tests
hash shade — they will change; the same-seed-same-world test must still
pass *within* a build); at-rest suite green; task-2 compare clean.

## Task 4 — Pockets follow bedding + gravel legibility

`passes.rs::pockets()` currently draws uniform ellipses at uniform
density, indifferent to depth, bedding and region — the strips read them
as polka dots, and the journey lens flagged that they read as ore (a
false promise). Rework placement, keep the seal machinery **exactly as
is** (the collect-then-verify-seal skeleton with the one-cell rind check
is load-bearing and verified — do not restructure it):

- Elongate lenses *along the local strata band*: `column.rs` computes
  `strata_offset` (tilt + fold); orient each ellipse's long axis along
  the band through its center and stretch it (length 2–4× today's,
  thickness similar).
- Key density and size to the blended `Character.sediment` (sedimentary
  regions richer, resistant regions sparse) and vary with depth (denser
  in the upper massif, rare near bedrock).
- **Gravel legibility**: gravel pockets are currently gray-on-gray
  invisible. Separate gravel's buried read from stone via its palette in
  `assets/gravel.ron` — but check first where else gravel appears (talus,
  rubble-adjacent surfaces) and keep the change subtle enough that scree
  still reads as broken rock; if the palette serves two masters badly,
  record a finding proposing a shade-range split instead of forcing it.

Verify: strips (rolling + canyon, seeds 1 and 7) show lenses tracking
the banding; task-2 compare (pocket counts move — record the new p90 in
the commit); at-rest suite green (pockets are sealed; nothing may move).

## Task 5 — Dune-comb and plumb-riser legibility fixes

Two mechanical-looking artifacts, both in `column.rs`, both behind
A/B-able params so the owner can judge (the repo's runtime-selector
convention — here, a preset param defaulting to the new behavior with
the old behavior reachable by setting it to 0):

- **Dunes** (`dunes()`): the phase term `x/wavelength + 0.6*fbm` is
  dominated by the linear term, giving a constant-pitch sawtooth comb.
  Give each dune individual amplitude and wavelength drawn from noise
  keyed on the dune *index* (a new `Purpose` stream, appended), keeping
  the asymmetric windward/lee profile and the existing repose-safe
  amplitude clamp (that clamp is load-bearing for at-rest — re-check it
  against each dune's *own* amplitude).
- **Terrace risers** (`terraced()`): risers are dead-plumb 40+-cell
  one-column faces because `detail_amplitude` (2.5–3.0) is far too small
  to break them. Add riser-scale roughening: a second, larger-amplitude
  detail term applied *only where the terrace snap is active and the
  local slope is riser-steep*, so benches stay flat and faces get ragged
  column-scale variation. New param (e.g. `riser_roughness`), default
  tuned by eye against canyon seed 7, `0.0` = today's behavior.

Verify: arid strip (seeds 1, 7) for dunes; canyon strips (seeds 7, 13)
for risers — including whether the task-1b keyhole columns look
different (report, don't fix); at-rest suite across all presets × 5
seeds is the hard gate (both changes move surface geometry, which is
exactly what the repose guarantees protect); task-2 compare.

## Task 6 — Brows/talus rescue at region scale

The formation vocabulary is invisible: `brows` wrote 34/45/0 cells and
`talus` 148/34/0 (rolling/canyon/wetland seed 1) in ~1.3M-cell worlds.
The cause is recorded in `passes.rs::cliff_edges()`'s own comment: drop
detection uses `RUN = 4` columns and `CLIFF_DROP = 6`, which is nearly
blind to region-scale escarpments that spread tens of columns. Rescue:

- Detect cliffs at *escarpment scale*: measure drop over a window that
  scales with the drop already found (start: also test a RUN of ~16–24
  with a proportionally larger CLIFF_DROP; a face qualifying at either
  scale qualifies). Scale brow reach and talus apron volume with the
  *measured* drop (brows already half-does this — extend, don't
  replace).
- Talus heaps must stay repose-clamped by the existing two-sweep taper
  (that machinery exists and is what keeps at-rest true — route through
  it).
- **Do not add new formation types in this task** (no boulders, no
  hoodoos — those wait on the erosion design).

Verify: pass counts move from 34–148 into the visibly-present range —
record before/after counts per preset in the commit message; strips
(canyon + rolling, seeds 1, 7) show gravel aprons at cliff feet and
lips over drops; at-rest suite green ×5 seeds ×6 presets; task-2 compare
(this is exactly the change class the sweep exists for — the p90s WILL
move; the gate is that nothing else moves with them).

## After task 6

Stop. The next round (erosion-driven formations, boulders, vault pass)
depends on design work in flight on the planning session's side. Push
everything, make sure this file's Findings section holds anything you
learned, and end with a summary comment in the final commit message.

## Findings

*(Append findings here as tasks produce them — task 1 writes 1a/1b/1c;
later tasks add entries when a spec did not survive contact with the
code.)*

Reproductions for everything below are `#[ignore]`d probes, kept rather
than thrown away so a reader can re-check a claim rather than take it:

```
cargo test --release --test worldgen -- --ignored --nocapture   # 1a, 1c
cargo test --release --lib worldgen::column::tests::probe -- --ignored --nocapture   # 1b
```

They build at the shipped 2048x640, not the suite's 512x320, because the
review's x coordinates are shipped-size coordinates and the regional layout
scales with width — at 512 the same x is a different world.

### 1a — The blue slivers are **sky**, not water

**No water anywhere.** A direct material census over every cell of
`canyon` seed 1 and `arid` seed 1 at 2048x640 finds **zero** water cells in
either world. The per-pass counter was telling the truth and `ponds` did
not leak; no other writer put water there either.

**The blue is the sky, seen through a gap in the terrain.** The deep sky
renders at `(56, 104, 174)` and water's palette starts at `(64, 116, 208)`
— close enough that a one-to-two-column gap of sky reads as a sliver of
water at strip zoom. Both cited sightings are gaps, and they are two
*different* objects:

- **canyon s1, x 950–955**: an open notch in the skyline, 7 cells below the
  ground either side of it, floored by talus gravel. World-wide there are
  12 such notches at >= 5 cells deep and none at >= 8.
- **arid s1, x 1234–1244**: not a notch at all but a **`brows()` overhang**
  — attached stone at y 104–106 with open air at y 107–110 beneath it and
  talus at y 111. The blue is sky *under a lip*, which is the pass working
  as designed and rendering at full sky brightness. `arid` s1 has 48
  notches at >= 5 cells and 4 at >= 8.

So the two producers probably want different answers and neither is a
water bug: the skyline notch is 1b's subject below, and full-brightness sky
under an overhang is the cave/overhang-lighting question (roadmap 3 and 8),
not worldgen's.

**Two metric traps hit on the way, both worth recording.** The first metric
I wrote counted "water-coloured pixels" in the rendered strip and returned
all 2048 columns — it was matching the sky, which is CLAUDE.md's *ask what
a metric counts when nothing is wrong* on the first attempt. The second
looked for a column whose first solid cell sits >= 8 rows below the columns
three either side, and reported **zero notches** in a world that visibly
has them: the canyon slot is seven columns wide, so both comparison points
land inside it. The metric that works compares against the **shoulder** —
the highest ground within 12 columns either side — and reports a
distribution rather than a bar chosen before anyone had looked.

### 1b — The keyhole is `round()` in the terrace snap. The mask-edge suspect is **refuted**

The review's suspect was the mask edge: `smoothstep(0.62, 0.82, …)`
flipping a single column between snapped and unsnapped. Printing the
elevation chain term by term around all four cited slots refutes it. The
mask strength `m` does not flip anywhere — at canyon seed 7 it moves
0.753 → 0.784 across the x 609 → 610 step and is a smooth ramp across the
whole 41-column window.

**What steps is `snap_delta`**, at that same column: −10.95 → +11.86. The
cause is the rounding in `terraced()`:

```rust
let snapped = (band_coord / p.terrace_step).round() * p.terrace_step;
```

`band_coord` drifts smoothly, `round()` does not: when it crosses a half-
band it jumps by one whole `terrace_step`, so the surface moves by
**`terrace_step × m` rows in a single column**. Predicted against measured,
all four sites:

| site | `terrace_step` | `m` | predicted | measured |
|---|---|---|---|---|
| canyon s7 x 610 | 34 | 0.784 | 27 | 27 |
| canyon s7 x 616 | 34 | 0.990 | 34 | 34 |
| canyon s7 x 622 | 34 | 0.992 | 34 | 33 |
| rolling s1 x 1313 | 26 | 0.798 | 21 | 21 |

Census of single-column surface steps >= 6 rows over 2048 columns, five
presets x four seeds: 0–16 per world, worst step 34 rows. Re-running each
with `terrace_strength: 0.0` and everything else held equal, **almost every
one of them disappears** — rolling s2 4 of 4 snap-caused, terraced s2 7 of
7, canyon s7 7 of 8. The one real exception is canyon seed 2 (2 of 16),
where the regional escarpment is that steep on its own.

**For whoever designs the fix** (the task file assigns it to the reviewing
session): these steps *are* terrace risers, and a riser is meant to be a
vertical face. What is wrong is the aspect ratio, not their existence.
canyon s7 puts three risers of 27, 34 and 33 rows at x 610, 616 and 622 —
six-column treads between 30-cell faces, which is a ladder rather than a
staircase. Task 5's `riser_roughness` will change how a riser *reads*; it
will not change how many there are or how tall they get, so if the count is
the complaint the lever is `terrace_step` against the local escarpment
slope, not roughening.

### 1c — Confirmed sand, but it is the pond **bed**, and `fill_dimming` is not why it is pale

The review's reading is confirmed in substance and corrected in two
details.

**Confirmed**: the pale dashes are sand. On rolling seed 1 the dashes sit
at rendered rows 162–167, and censusing that band finds a run of sand at
x 816–824, y 162–163 with soil below it and water above.

**Correction 1 — it is not *shoreline* sand.** The top-of-column census
across the whole rim (x 760..980) is water 212, stone 9, **sand 0**: there
is no sand above the waterline anywhere on this pond. The dashes are the
top row of the pond *bed* where the shelf is shallow, which is why they
appear as a broken horizontal line exactly at the waterline.

**Correction 2 — `fill_dimming: 0.0` is not the cause.** It is confirmed
zero in `water.ron`, and that is a real finding on its own (it does disable
water darkening globally, and the owner question in review §5.0a still
stands). But it cannot be what makes these dashes pale, because they are
**sand cells, not water cells** — no liquid dimming applies to them at all.
What makes them pale is sand's own palette, which tops out at
`(232, 208, 142)`, plus `render.rs`'s grain jitter carrying the brightest
entry to the `(255, 229, 150)` measured in the strip.

The whisker/monolayer reading stays refuted, now for a second and
independent reason: the cells are not liquid.

**One incidental**, noticed in the same census and not chased: `ponds`
writes water shades 0..3 while `water.ron` ships a **three**-entry palette,
so `palette[shade % 3]` aliases shade 3 onto entry 0 and that entry draws
twice as often as the other two. Cosmetic, and a one-line data fix
(a fourth colour) if anyone wants the three tones evenly weighted.

### 2 — Sweep notes, and one thing the sweep found on its own

`scripts/worldgen_sweep.sh` (+ `scripts/worldgen_sweep_baseline.tsv`), six
presets x seeds 1..16 = 96 runs at 512x320, ~23 s. `run` prints the table,
`baseline` rewrites the committed TSV, `compare` re-runs and diffs it.

Verified as the task asked: two back-to-back `run`s produce **byte-identical
output**; setting every preset's `tree_density` to `0.0` and running
`compare` flags `life_scatter` in exactly the four presets that had trees
(canyon −40%, rolling −47%, terraced −52%, wetland −41%) and **nothing
else**; reverting returns a clean compare (0 counters moved).

**`assets/worldgen.ron` is not an `include_str!` asset.** Landmine §7.2 is
about `assets/materials/*.ron`, which are compiled in; `WorldgenPresets::
load()` reads `assets/worldgen.ron` from disk at runtime, so a preset edit
takes effect on the next *run* rather than the next build. That is why the
`tree_density` check above worked without rebuilding — and it is worth
knowing before someone spends a sweep wondering why their preset knob
appears connected when material knobs in the same session do not.

**What the sweep found while being calibrated: generated worlds do not
*stay* asleep.** `awake_chunks` is sampled at frame 100, and its floor is
not zero on any preset — p90/max of 3/6 (arid), 6/8 (rolling, flat), 7/7
(canyon, terraced), 6/10 (wetland), out of 40 chunks. **`flat` is the case
that makes this a finding**: dead-level bare rock, no water, no life, no
pockets, nothing that can move, and it still reads 8 of 40 awake, with
`active_site_count` climbing 200 → 482 → 512 over frames 20 → 60 → 200 —
which on a 512-wide world is one site per column.

This is pre-existing on this branch base (nothing in task 1 or 2 changes
engine behaviour), and the at-rest suite is green because it asks a
different question: `generated_terrain_stops_sweeping_almost_immediately`
measures the **first** frame at which `active_chunk_count()` reaches zero
and stops there. It never asks whether the world stays quiet, so a world
that goes quiet at frame 30 and re-wakes at frame 60 passes it.

On `flat` the only `ActiveKind`s that can be responsible are
`StructuralCheck` and `Evaporate`, and `flat` has no water. That points at
`src/sim/structural.rs`, which this track is explicitly read-only on
(§ground rules, and landmine §7.18) — so this is filed as a finding, not
chased. It matters because of landmine §7.20: a permanently-awake chunk
anywhere defeats `field::step`s early-out at ~7 ms/frame for the whole
world, and 3–8 of 40 is not "anywhere", it is a fifth of the world.

Consequence for this sweep: the `awake_chunks` row is a **tracked** number,
not a gate at zero, and `compare` flags it only on a move of at least 3
chunks *and* 30% — a floor a small integer counter needs, since 6 → 8 is
+33% and is noise at this scale.
