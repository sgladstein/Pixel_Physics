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

### 3 — Region-keyed palettes: the spec held, but **it needed two files outside this track**

The work landed as specified — `Character` in, tones out, no material
placement changed, zero frame cost. One part of it did not survive contact
with the code, and it is the part the ground rules say to report rather
than improvise around, so it is written up here in full.

**What the spec said was in scope**: "if the 4-entry palettes clamp too
hard to show a family shift, widening a palette in
`assets/materials/*.ron` is in-scope (data-only)". They do clamp
completely — with four entries there is no way to express a family shift at
all, only a reordering of the same four colours — so the palettes were
widened: stone to four families of four (neutral / cool damp / warm
sandstone / pale cap-rock), soil and sand to three each.

**Why that could not stay data-only.** `palette.len()` was doing two jobs
at once, and widening the list split them apart — CLAUDE.md's *when a fix
changes what a number means, re-deriving the constants that read it is part
of the fix*. Two live consumers pick a shade **at random** from
`palette.len()`:

- `src/sim/world.rs`s brush (`paint_stroke`). Its own comment records the
  scheme: `shade = below(shades) + shades * below(256/shades)`, low bits
  choosing the palette entry and high bits carrying grain entropy. Against
  a 16-entry stone palette that draws uniformly across **all four
  families**, so a painted wall comes out as confetti of grey, sandstone
  and bleached cap-rock. Building is a core verb; this is not a subtle
  regression.
- `src/sim/decay.rs`s ash → soil. Same shape: decayed ash would land in a
  random soil family, speckling a wet bank with desert-pale soil.

So the change is:

- `src/sim/material.rs` — a new `MaterialDef::base_colors` (defaulting to
  `0` = "all of them", so every other material is untouched) and a derived
  `Material::base_shades`.
- `src/sim/world.rs`, `src/sim/decay.rs` — draw the *entry* from
  `base_shades` and keep `palette.len()` as the modulus. For every
  single-family material the two are equal and the arithmetic is
  byte-identical to what it was; the brush's high-bit grain trick is
  preserved rather than removed.

None of those three is on the must-not-touch list, but none is on the owned
list either, so: **flagged for the reviewer, and kept as small as it can
be.** Three files, one new field, two call sites.

**One consumer deliberately left alone**: `src/app.rs::spawn_burst` (the
debug particle burst) still picks from `palette.len()`, so a burst of stone
particles will show mixed families. `app.rs` is contested and forbidden to
this track, and the cost of leaving it is that a debug-only tool is
cosmetically off. If the reviewer wants it, it is the same one-word change.

**Nothing else reaches the widened palettes**, checked rather than assumed:
`plant.rs`s six random-shade sites all read an organism cell's own material
(wood, leaf, moss, rootwood, seed); `fire.rs`, `rigid.rs`, `structural.rs`
and `creature.rs` read `burns_into`/`breaks_into`/corpse materials, none of
which is stone, soil or sand; `liquid.rs` is water.

**The task-2 sweep is clean, and it must be** — this task writes no
different cells, only different shade bytes, so the census cannot see it at
all. The images are the evidence, and the counters that say the mechanism
fired are in `a_varied_world_uses_more_than_one_rock_family`, which prints
the per-family rock census beside them (rolling s1: 41,892 neutral / 29,469
wet / 14,098 cap-rock; canyon s1: 58,863 / 9,327 / 54 dry / 15,907; arid
s1: 9,531 / 84,558 dry / 2,788; wetland s1: 80,183 wet / 1,004 cap-rock).
`flat` is asserted to stay in family 0 — it asked for no regional variation
and the structural workstream compares against its renders.

**Two design notes the reviewer may want to overturn:**

1. The family is chosen by a **cumulative selection over one per-cell noise
   draw**, so a region boundary is a dither rather than a ruled vertical
   colour seam through solid rock. Checked at 4x zoom: it reads as mottled
   sandstone grain, and at 1:1 the sedimentary banding is as legible after
   as before (the bands still come from the band index; only the four
   colours they name change). But it is per-cell white noise, and a
   lower-frequency mottle would read more like real facies change if the
   reviewer thinks the grain is too fine.
2. `region_variation <= 0.0` opts a preset out of families entirely, which
   is what keeps `flat` byte-identical.

### 4 — Pockets: the spec held, and the gravel palette did **not** have to serve two masters badly

Lenses are rotated onto the local bedding (`strata_offset`s own gradient,
so the third consumer of that field agrees with the two that already
existed), stretched 2-4x along it, and their count and size keyed to
`Character.sediment` damped by `resistance`, thinning quadratically toward
bedrock. Both factors are exactly `1.0` at a neutral character and zero
depth, so a preset with no regional variation generates what it always did.
The collect-then-verify-seal skeleton and the one-cell rind are untouched;
only the shape function they evaluate changed, plus the scan bounds, which
had to become the rotated ellipse's bounding box.

**Paired measurement, canyon seed 1 at 512x320** (same world, same seed,
pass toggled):

| | lenses | mean bounding box | aspect | cells |
|---|---|---|---|---|
| before | 15 | 14.7 x 6.3 | 2.3:1 | 876 |
| after | 4 | 38.0 x 8.5 | 4.5:1 | 633 |

Fewer, much longer lenses — which is the direction the review asked for
(the complaint was polka dots), but worth stating plainly rather than
burying: **this seed loses two thirds of its lens count**. Across the
16-seed sweep the cell totals go the other way, p90 `pockets` +42% to +70%
(canyon 1193 → 2005, arid 1507 → 2557, rolling 1075 → 1676, terraced 1075
→ 1523, wetland 921 → 1084). Outcomes are chaotic in the seed exactly as
CLAUDE.md says, and seed 1 is not the sweep. Nothing else moved with them.

The 8.5-cell mean height is how the *rotation* is shown to have fired at
all rather than being a silent no-op: an unrotated lens is `2b` tall, 4-8
cells. At canyon's dip a half-length of 19 contributes `2 x 19 x 0.09` =
3.4 cells of extra height, which is what the measurement shows.

**Gravel: the palette did not have to be forced.** The task offered a
finding proposing a shade-range split as the fallback if one palette could
not serve both scree and buried lens. It can — because task 3 built the
families mechanism, so the split is now a four-line data change rather than
a proposal. `assets/gravel.ron` gains a second family: family 0 is exactly
what shipped and is what the brush, `talus`'s aprons and the soil profile's
stony contact all still get; family 1 is warmer and darker and is used only
by a lens sealed in rock. So scree still reads as broken rock against sky
and soil, and a lens reads as a conglomerate bed against stone. Recolouring
the material outright was the alternative and is recorded in the file as
rejected rather than untried.

Deliberately kept dull: the review found these reading as *ore*, a promise
the game does not keep, so the buried family is warm and dark rather than
bright or saturated.

**One test method note, because it cost two wrong versions.** Both cheap
classifiers for "is this gravel cell a lens or scree" miscounted, and for
the same reason — they inferred which pass wrote a cell from where the cell
is. "More than ten cells below the ground line" called 37 soil-contact
cells buried, because a blanket is up to 34 cells deep and its stony base is
family 0 by design. "Fully surrounded by rock" called 78, because a contact
cell at the bottom of a blanket usually is. The version that works is a
**paired comparison** against the same world built with `pocket_density:
0.0` — the repo's own preferred shape, and here it is not merely better but
exact, since `pockets` writes only into solid stone and nothing downstream
reads a buried lens.

Strips: `target/filmstrips/task4-after-{rolling,canyon}-s{1,7}.png`, against
`task3-after-*` as the before. Canyon s1 at 1:1 is the clearest: round pale
blobs become elongated lenses lying in the bedding, and brown gravel lenses
appear where there was nothing legible at all.

### 5 — Dunes and risers: both knobs work, and **both needed the spec's mechanism to be re-aimed**

Both changes are in `column.rs`, both behind a preset param defaulting to
the new behaviour with `0.0` reaching the old one exactly, so the owner can
A/B them by eye. Because `assets/worldgen.ron` is read at runtime, flipping
either is a file edit and a re-run — no rebuild — which is how every before
strip below was made.

#### 5a. Dunes — the premise was wrong, and the first implementation was inert

**The review's diagnosis does not survive measurement.** "The phase term
`x/wavelength + 0.6*fbm` is dominated by the linear term, giving a
constant-pitch sawtooth comb" — at `dune_variation: 0.0`, i.e. today, crest
spacing on `arid` already has a coefficient of variation of **0.42 (seed 1)
and 0.47 (seed 7)**. That is not a constant pitch. The fbm phase term is
doing its job.

What *is* uniform is crest **height**, and the cause is a constant that was
compensating for nothing anybody had looked at: `arid` asks for
`dune_amplitude: 18` against a repose cap of `max_slope * FALL * wavelength`
= **13.2**. Three dunes in four were already pinned at the cap.

So the obvious implementation of the spec — per-dune amplitude as
`dune_amplitude * (1 ± v)` — is **inert**, and measurably so: crest-height
spread moved 0.273 → 0.281 across the entire knob range. It reads exactly
like a dead lever and is not one; the clamp was not limiting the variation,
it was *absorbing* it. Varying **downward from the cap** instead is what
makes the knob work, and it is also the physically honest direction, since
a dune cannot be taller than repose allows and a real dune field is not all
fully-developed dunes.

Measured, `arid` at 2048 wide, crest height above its own troughs:

| `dune_variation` | seed | crests | mean height | cv height | mean gap | cv gap |
|---|---|---|---|---|---|---|
| 0.00 | 1 | 29 | 12.07 | 0.273 | 62.3 | 0.419 |
| 0.85 | 1 | 24 | 10.63 | 0.294 | 73.4 | 0.638 |
| 0.00 | 7 | 28 | 14.10 | 0.292 | 72.4 | 0.465 |
| 0.85 | 7 | 23 | 13.72 | 0.322 | 89.2 | 0.521 |

Spacing spread is where most of the gain is (+52% on seed 1). Height spread
moves less than the amplitude distribution suggests it should, and the
reason is a **censored metric, stated rather than hidden**: as variety
rises, the shortest dunes fall below the crest detector's prominence bar and
stop being counted — which is why the crest count falls 29 → 24 alongside.
The visible effect is in the strips.

Two implementation notes worth keeping: the trough datum had to change from
`(profile - 0.5) * amplitude` to `profile * amplitude - 0.5 * base`, because
`profile` is 0 at both ends of a dune's cell and the old form puts the
trough at `-0.5 * amplitude` — two neighbouring dunes of different height
would meet at a step of half their difference. And the repose clamp is
re-evaluated against each dune's **own** fall fraction, not the preset's,
per the task's instruction; the at-rest suite is green and every arid seed
loses zero cells.

**The metric needed two rewrites, both the same mistake.** A crest detector
using a 4-column window at 3 cells of prominence reported **zero crests** in
a world that must contain about 35 — it was asking for a 3-cell drop within
4 columns on a dune whose flank falls 13 cells over 26. Then the height
measure used the drop within the detection window, which on a half-wavelength
of 29 columns never reaches a trough and was reporting the underlying hill
slope. Both are the same failure: a metric written before its subject was
looked at.

#### 5b. Risers — a smooth term cannot break a single-column jump

The spec asks for "a second, larger-amplitude detail term". Implemented at a
14-column wavelength that is exactly what it sounds like, and it does not
work, for a reason that is structural rather than a tuning problem: **a
riser is a single-column jump in a heightfield**, and a term whose
per-column change is small can only move the whole bench up or down. Built
that way it shifted elevations by up to 6 rows near risers and left canyon
seed 7's worst riser at 34 cells, exactly where it started.

The term that works has a wavelength **near the grid** (2.5 columns, one
octave) — deliberately the opposite of every other wavelength in the file —
so it differs sharply between `x` and `x + 1` and turns one tall jump into a
short flight of smaller ones. The gate is the snap residual `|bands -
round(bands)|`, which separates riser from bench for free and with no second
elevation evaluation: on a steep escarpment `bands` sweeps its whole range
every few columns, while on a gentle bench it changes slowly and stays low.

Sweep of single-column steps >= 6 rows over 2048 columns:

| `riser_roughness` | canyon s7 steps / worst / mean | rolling s1 steps / worst / mean |
|---|---|---|
| 0.00 | 8 / 34 / 22.1 | 4 / 25 / 18.5 |
| 0.35 | 12 / 32 / 15.4 | 9 / 22 / 11.8 |
| 0.50 | 14 / 31 / 14.1 | 22 / 21 / 9.3 |
| 0.70 | 22 / 30 / 11.6 | 38 / 21 / 9.4 |

Read the **mean and the count together**, which is why the probe prints
both: the worst step barely falls, because at the steepest escarpment the
underlying relief supplies most of it, but the mean halves while the count
triples. That is one tall jump becoming a flight of shorter ones, which is
the shape asked for. A worst that fell while the count *also* fell would
mean the relief had simply been flattened — a different and worse outcome.

Defaults tuned by eye at 5x zoom on canyon seed 7: 0.45 rolling/terraced,
0.5 canyon, 0.4 arid, 0.35 wetland, 0.0 flat.

**Task 1b's keyhole columns, reported not fixed** as instructed: they do
look different. At `0.5` the plumb faces at canyon s7 x 610/616 become
stepped, and the brow lips over them break into two levels rather than one
straight lintel. The count of single-column steps goes *up*, not down —
this makes each riser shorter, it does not make risers rarer, exactly as
predicted in finding 1b.

#### Sweep consequences, all nine flags read

`pockets` +42..68% is task 4 (the baseline predates it). The rest are task
5, and none is a surprise once the mechanisms are stated:

- **`arid` brows −34%, talus −44%.** Dunes are shorter on average now
  (mean crest height 12.07 → 10.63), so fewer of them clear
  `CLIFF_DROP = 6` and cliff detection finds fewer edges. A real
  consequence, recorded rather than tuned away — and task 6 is about
  exactly these two counters, so it lands on top of this.
- **`wetland` brows +46%.** The opposite direction, and the same cause
  read backwards: riser roughening creates more single-column steps, so
  more of them qualify as cliff edges.
- **`rolling` soil_moisture −32%,** with max unchanged at 4337. Steeper
  ground near roughened risers carries less soil, so there is less soil to
  saturate. Only mid-distribution seeds moved; the worst seed is identical.
- **`arid` awake_chunks 3 → 6.** Checked against the hard gate rather than
  assumed: the at-rest suite is green across every preset x 5 seeds, and
  `cells lost since the cut` is **0 on all of arid seeds 1-8**. Nothing is
  moving; this is the pre-existing active-site churn recorded in finding 2,
  which is why that row is tracked rather than gated at zero.

#### One pre-existing test corrected, not weakened

`column.rs::steep_ground_carries_no_soil` failed after the riser change, at
seed 0 x 77: slope 0.5305 against a cutoff of 0.5195. The generator was
right and the test was reading the wrong material's angle — it used
`soil_tan` for every column, and a column dry enough to carry **sand**
stands at 34 degrees against soil's 33, so the bar was 2% too strict there.
The generator's own gate has always used `cover_tan(x)`. Nothing had ever
put a sandy column that close to its own limit before; riser roughening
did. Fixed by asking about the cover the column actually carries, and the
message now names it. The empirical half of the guarantee — every preset x
5 seeds stepped 120 frames with zero cells moving — was green throughout,
and `cells lost since the cut` is 0 on arid seeds 1-8.

#### Cost

Build, `ascii`, same machine: 2048x640 place 266.3 → 275.9 ms (+3.6%), whole
build 456.5 → 454.8 ms, paid once at generation. Frame timings unchanged
(stress 82.3 ms, +field 88.0 ms, render skip 0.001 ms, 0 chunks awake). The
+3.6% is one extra fbm evaluation per column, and only where the snap gate
opens.

### 6 — Brows/talus rescue: the far scale works, and it needed the pass margins re-derived

`cliff_edges` now measures at two scales and a face qualifying at either
qualifies, sized by the deeper of the two so a face that is part of an
escarpment is sized by the escarpment rather than by its first four columns.

**The far scale is not a proportional bar, and that matters.** The task
suggested "a RUN of ~16-24 with a proportionally larger `CLIFF_DROP`".
Proportional means the same slope — `6 * 20 / 4` = 30 cells over 20 columns
is a slope of 1.5, exactly what the near scale already asks for, and it
would have found *nothing extra for exactly the reason the near scale
misses escarpments*. A regional escarpment is not steeper than a terrace
riser, it is **taller and gentler**: tens of columns at a slope near 1. So
`RUN_FAR = 20` with `CLIFF_DROP_FAR = 20` — a slope of 1.0.

Brow reach and thickness and talus peak all scale with the measured drop
(reach already half-did, and is extended rather than replaced), capped by
`MAX_BROW_REACH` and `MAX_TALUS_PEAK` — caps that bound the work without
gating whether it happens, per the twice-written landmine. Talus still
routes through the existing two-sweep repose taper, untouched.

**Per-seed, 512x320, task 6 alone** (i.e. on top of tasks 3-5):

| | brows before → after | talus before → after |
|---|---|---|
| canyon s3 | 9 → **391** | 20 → **617** |
| canyon s7 | 459 → 1412 | 221 → 678 |
| canyon s2 | 620 → 1655 | 415 → 1048 |
| rolling s2 | 234 → 675 | 111 → 610 |
| rolling s7 | 193 → 431 | 116 → 390 |
| wetland s7 | 4 → **97** | 45 → 123 |
| terraced s2 | 99 → 415 | 10 → 141 |
| rolling s1 | 172 → 216 | 164 → 164 |
| arid s1, s7 | unchanged | unchanged |

Sweep p90 over 16 seeds, against the task-2 baseline (which predates tasks
3-6, so these are the whole track's movement): brows rolling 376 → 1081
(+188%), canyon 918 → 2749 (+199%), terraced 290 → 746 (+157%), wetland 24
→ 109 (+354%), arid 58 → 153 (+164%); talus rolling 246 → 840 (+241%),
canyon 676 → 1621 (+140%), terraced 282 → 719 (+155%), wetland 46 → 141
(+207%), arid 137 → 182 (+33%).

**Two rows in that table are the ones worth reading, and neither is a
gain.** `rolling s1` talus and both `arid` seeds come out *bit-identical*.
That is the shape CLAUDE.md flags as "identical outputs mean the knob was
never connected" — checked rather than assumed, and here it is the correct
answer rather than a dead knob: those worlds have no face that clears the
far scale, so only the near scale fires and nothing about it changed, and
their talus peaks are all limited by `fall / 2` rather than by
`talus_max_height`, which is the term that grew. The sweep is what shows the
knob is live; seed 1 is not the sweep. This is also why the review's headline
"brows 34, talus 148" needs a footnote: those were seed-1 numbers, and the
p90 across sixteen seeds was already 376 and 246.

**A finding this cannot fix**: `wetland` seeds 1, 2 and 3 generate **zero**
cliffs, before and after. Low relief plus `terrace_strength: 0.3` means no
face anywhere clears even `CLIFF_DROP = 6`, so the formation vocabulary is
simply absent from most wetland worlds. Lowering the bar would hang lips off
gentle slopes, which is the failure the constant's own comment warns about.
If wetland should have scree at all, the lever is its relief or its terrace
strength, not cliff detection — an owner/reviewer call, not one for this
track.

**The pass margins had to be re-derived, and one was already wrong.** A
margin is the contract a per-chunk generator plans against, so an understated
one is a promise to produce different cells at a chunk edge. `brows` goes
4 → 40 (`RUN_FAR` of detection + `MAX_BROW_REACH` of writing). `talus` goes
3 → 200 — and **3 was already wrong before this change**, because the pass
walks up to `MAX_FALL` = 120 columns looking for the foot of a fall and has
done since it was written. Both are large and both are honest; shrinking
them is the coarse map's job, not optimism here. `only_the_water_passes_
read_the_whole_world` still passes: these are finite, not `GLOBAL`.

**Placement, not just count.** `cliff_formations_land_at_cliffs_and_are_
visibly_present` is the guard, because a detector loosened until it fires
everywhere would move the counts just as well and be strictly worse. It
attributes cells by building the same world with each pass switched off,
then checks each cell has a real drop within reach: **100% of talus and 100%
of brow cells, on rolling, canyon and terraced.** Bars set from the
measurement with headroom (weakest case is terraced at 47 talus and 102 brow
cells; bar is 30).

Its relief metric needed one correction, the same shape as the two in task
4 and one in task 5: measuring only "does the ground fall away from here"
scored 91 of 216 brow cells and 72 of 164 talus cells as misplaced, when
every one was where it belonged — a brow hangs over ground that has
*already* fallen and an apron sits at the foot of a face that *rises* beside
it. Absolute local relief is the right question. Then the drop threshold had
to come down from 20 to 6, because 6 is the pass's own near-scale bar and an
apron under a modest riser is a legitimate apron; asking for an escarpment
was a test failing for wanting something the code never claimed.

**Cost**: 2048x640 place 275.9 → 275.4 ms — the second scale is 40 more
array reads per column and does not register. Whole build 454.8 → 469.9 ms,
the difference being the structural pass over more formation cells
(178.9 → 194.5 ms), paid once at generation. Frame timings unchanged, 0
chunks awake in every settled `ascii` scene, render skip 0.002 ms.

At-rest green across every preset x 5 seeds — the aprons are much larger and
still route through the two-sweep repose taper, which is what makes that
true rather than lucky.

**The sweep baseline was refreshed in the task-6 commit**, deliberately and
as the last act of the track. Up to that point every `compare` was against
the pre-task-3 numbers, which is what made the whole track's movement
visible in one diff (and is why the tables above quote it). Left un-refreshed
it would show tasks 3-6's movement forever and the next session would learn
to ignore it, which is exactly how a rubber-stamped baseline happens. The
before numbers are preserved here and in the commit messages; the file now
carries the post-track state, so the next change compares against what is
actually shipped.

---

# Round-2 findings

Round 2's queue is `Reports/worldgen-implementation-tasks-round2-2026-08.md`;
its findings are appended here, as that file instructs, so one file holds the
whole track's record.

Reproductions, all `#[ignore]`d probes kept rather than thrown away:

```
cargo test --release --lib worldgen::column::tests::probe_r2t1 -- --ignored --nocapture
```

### R2-1 — Slope attenuation works on the escarpment risers, and **cannot reach `rolling`'s two tallest**

The mechanism landed exactly as specified. `terraced()` now scales its mask
by `terrace_yield(x)`, which is `1 - smoothstep(terrace_slope_lo,
terrace_slope_hi, slope)` over a +-8 column central difference of the
**pre-terrace** elevation (`base_wave + hills`, factored out as
`pre_terrace_elev` — `slope()` differences `elev()` and `elev()` calls
`terraced()`, so a terrace rule asking `slope()` would recurse forever).
Shipped window `0.6..2.0` on every preset.

**Against the pre-registered bar — no single-column surface step > 18 rows on
`probe_1b_how_often_the_surface_steps` — 18 of 20 worlds pass and two do
not.** Both misses are `rolling`, and both are unchanged by the mechanism
rather than merely under-tuned by it:

| world | worst before | worst after | count before -> after |
|---|---|---|---|
| canyon s7 | **31** | **15** | 14 -> 10 |
| terraced s2 | 20 | 12 | 6 -> 5 |
| terraced s13 | 18 | 17 | 7 -> 4 |
| terraced s7 | 16 | 9 | 8 -> 7 |
| canyon s13 | 16 | 9 | 4 -> 1 |
| rolling s7 | 18 | 15 | 4 -> 2 |
| **rolling s1** | **21** | **21** | 21 -> 21 |
| **rolling s2** | **25** | **25** | 15 -> 13 |

canyon s2 is the escarpment exemption finding 1b already carved out; its
worst is 8 either way, so it never needed one.

**Why the two misses are structural, not a tuning gap.** The design premise
is that a tall riser stacks on top of ground the relief has already made
steep. That premise is *measured true* for the population it was written
for and *measured false* for these two. Pre-terrace regional slope at each
world's worst step, beside the world's own median slope:

| world | worst step | slope at it | world p50 slope |
|---|---|---|---|
| canyon s7 | 31 | **5.03** | 0.140 |
| terraced s2 | 20 | 1.44 | 0.135 |
| rolling s1 | 21 | **0.204** | 0.156 |
| rolling s2 | 25 | **0.272** | 0.140 |

`rolling` s1 and s2 put their tallest risers on ground at essentially the
world median slope — the gentle country the spec explicitly instructs the
snap to keep at full strength ("benches keep their full snap on gentle
ground"). The bar and the mechanism are asking for opposite things there.

The sweep of the attenuation window says the same thing from the other side,
and says what forcing it would cost. Worst step / count of steps >= 6:

| window | rolling s1 | rolling s2 | rolling s7 | rolling s13 | terraced s1 | canyon s13 |
|---|---|---|---|---|---|---|
| off | 21/21 | 25/15 | 18/4 | 8/3 | 18/4 | 16/4 |
| 0.6-2.0 *(shipped)* | 21/21 | 25/13 | 15/2 | 8/3 | 18/4 | 9/1 |
| 0.25-0.9 | 21/21 | 25/10 | **0/0** | 8/2 | 18/3 | **0/0** |
| 0.10-0.40 | 14/13 | 13/2 | **0/0** | **0/0** | **0/0** | **0/0** |

The only settings that reach `rolling` s1 and s2 are the ones that delete
terracing outright from five of the twenty worlds. **Read the count beside
the worst**, which is why the probe prints both: a worst that falls while
the count falls to zero is not a tamed riser, it is a flattened world — the
same trap round 1's finding 5b called out for `riser_roughness`.

**The lever that does reach them, measured rather than proposed.** Finding
1b named `terrace_step` as the alternative, and on `rolling` it works. With
the shipped `0.6..2.0` window and `terrace_step` reduced from 26:

| terrace_step | s1 | s2 | s7 | s13 |
|---|---|---|---|---|
| 26 *(shipped)* | 21/21 | 25/13 | 15/2 | 8/3 |
| 22 | 18/8 | 24/12 | 12/2 | 15/5 |
| **18** | **16/4** | **18/5** | **8/2** | **8/1** |
| 15 | 18/8 | 17/5 | 8/3 | 12/2 |

`terrace_step: 18` clears the bar on all four `rolling` seeds with no count
zeroed. **It is not shipped**, and deliberately: `terrace_step` is the
height of every bench on the preset, so cutting it 30% re-spaces `rolling`'s
entire benched vocabulary — a landform decision of the kind the ground rules
reserve to the reviewing session, not a tuning of the mechanism this task
specified. The numbers are here so the call can be made without re-deriving
them. It is a one-line `assets/worldgen.ron` edit, runtime-loaded, no
rebuild.

**One framing correction, cheap and worth having.** The round-1 name for
these is "keyhole slots", and a slot implies a notch that drops and comes
straight back. Measured, they do not: of the three worst steps in each of
twelve worlds, all but four are one-way drops that stay down (recovery
within four columns < 0.6). They are bluff faces, which is what finding 1b
concluded from the elevation chain and what this confirms from the surface.
The first version of that recovery metric reported "SLOT" for every step in
every world, because it included `k = 0` and so compared each step against
itself — the repo's *ask what a metric counts when nothing is wrong* trap,
hit again, caught by the answer being unanimous.

**Gates.** `cargo test` green (the one failure on the way was real and
correct: `the_default_preset_matches_the_compiled_in_fallback` caught
`WorldgenParams::default()` drifting from `rolling` the moment the field was
added, so the default carries `0.6/2.0` too). Clippy clean. At-rest suite
green. Sweep `compare`: **0 counters moved past +/-30%**, the largest move
being `terraced` talus p90 719 -> 707 (-1.7%) — this changes surface
geometry slightly, so the formation passes see slightly different faces.
`flat` is untouched by construction: `terrace_strength: 0.0` returns before
`terrace_yield` is ever called.

**Cost: below this machine's noise floor, and the noise floor is the number
worth recording.** One extra `pre_terrace_elev` pair per column, only where
the mask gate is already open. `ascii`'s 2048x640 place time, both directions
re-measured back to back in this session rather than against a remembered
number:

```
attenuation off : 215.4  211.9  217.6  235.3      mean 220.0   (also one 170.2, see below)
attenuation on  : 218.2  219.2  222.2  217.2      mean 219.2
```

The means differ by 0.4%, which is nothing, and the *spread within a single
setting* is 212-235 ms — larger than any effect being looked for. An early
sample of the off case returned 170.2 ms on the same binary and the same
settings, a 28% swing, which is what a shared container does to a wall-clock
measurement. So the honest statement is not "+0.4%" but **"the added work
does not register against a +-10% run-to-run spread"**, and anyone re-testing
this should expect to need many runs to see an effect this small at all.
Recorded this way deliberately: quoting 219.2 against a single 170.2 baseline
would have manufactured a 29% "regression" out of container noise, which is
exactly the trap CLAUDE.md's *re-measure the baseline in the same session*
rule exists for — and re-measuring is what caught it here.

Frame timings unchanged: `ascii` reports 0/40 chunks awake in every settled
scene and a 0.002 ms render skip.

**Images**: `target/filmstrips/task1-{before,after}-canyon-s{7,13}.png`. The
mesa constraint holds — all four of canyon s7's buttes keep their
silhouettes and their caps. What changes is the second butte's left face,
which is a single dark full-height slot in the before strip and a stepped
flight in the after.

### R2-2 — The piers were real, canyon-only, and the fix needed a metric that was not the dither

Two changes in `palette_family`, together: the family probability is now
displaced by a slow 2-D field (`Purpose::PaletteField`, appended at **18** --
17 was already `Riser`, and the collision was a compile error rather than a
silent renumber, which is the registry rule working), and the aridity ramps
are widened from `0.50..0.78` / `0.10..0.34` to `0.42..0.86` / `0.06..0.42`.
Behind a per-preset `palette_field`, `0.0` reaching the per-column threshold
round 1 shipped.

**The metric took two attempts and the first one was worthless in an
instructive way.** Version one measured per-cell run length of the family
along y against along x. It reported **y/x = 0.99 for every preset at every
setting, before and after** -- which reads as "the mechanism does nothing"
and is actually "the question is wrong": the per-cell dither is white noise
from `unit(seed, Palette, x, y)`, so its run lengths are isotropic by
construction no matter what the probability behind them does. It was
measuring the stipple and could never have seen the band. The tell was the
answer being identical in cases already known to differ.

Version two measures at the scale the artifact has: family *mix* per 8x8
block, then the mean L1 change between vertically adjacent blocks against
horizontally adjacent ones. A pier is a column of blocks that agree beside
blocks that do not, so `v/h` is the shape number.

**It localises the complaint, which is worth as much as fixing it.** At
`palette_field: 0.0`, canyon is the *only* preset that is columnar:

| preset (seed) | v/h before | v/h after | top-family share before -> after |
|---|---|---|---|
| canyon s7 | **0.58** | **0.75** | 0.47 -> 0.44 |
| canyon s13 | **0.56** | **0.74** | 0.50 -> 0.45 |
| rolling s1 | 0.91 | 0.94 | 0.54 -> 0.51 |
| wetland s1 | 0.96 | 0.98 | 0.88 -> 0.84 |
| arid s1 | 0.98 | 0.99 | 0.91 -> 0.88 |

The merge review saw this on canyon and only on canyon, and the number agrees:
rolling, wetland and arid were already near-isotropic and had no artifact to
fix. That is why the knob is **per-preset** rather than one global setting --
canyon 0.45, rolling/terraced 0.30, wetland/arid 0.15, flat 0.0 -- which is
the branch the task's own text sanctions ("sweep both knobs behind preset
params if a single setting doesn't hold across presets"). It does not hold:
the sweep is monotone in both directions at once.

| `palette_field` | canyon s7 v/h | wetland s1 top family |
|---|---|---|
| 0.00 | 0.58 | 0.88 |
| 0.15 | 0.66 | 0.84 |
| 0.30 | 0.71 | 0.80 |
| 0.45 | 0.75 | 0.76 |
| 0.60 | 0.79 | 0.71 |

**`v/h = 1.0` is not the target, and chasing it would undo round 1's task 3.**
Some horizontal anisotropy is *signal*: aridity genuinely varies with x, and
a family boundary that has no preference for x at all means regions have
stopped being different country. The cost column is what says where to stop
-- pushing wetland to 0.60 takes its dominant family from 88% to 71%, which
is a wet country that is no longer notably wet. So the artifact removed is
the part pinned to a *column*; the part that tracks the region stays.

**The ramp widening earns its place, measured separately rather than assumed.**
Held at `palette_field: 0.0` with only the ramps reverted, canyon s7 reads
v/h 0.50 and canyon s13 0.46, against 0.58 and 0.56 widened. On s13 the
widening alone is the larger of the two contributions (+0.10 against the
field's +0.13 on top of it). It costs canyon s13 four points of dominant
family and arid two, and wetland nothing.

**Sweep compare is the proof this is shade-only, and it is exact**: the
`compare` output after this task is **byte-identical** to the output after
task 1 -- same eight sub-threshold rows, same numbers, 0 counters past
+/-30%. This task writes no different cell anywhere; only shade bytes change,
which the census cannot see by construction.

`flat` and any `region_variation <= 0.0` preset are untouched:
`palette_family` returns `FAMILY_NEUTRAL` before the field is evaluated, and
`a_varied_world_uses_more_than_one_rock_family` still asserts flat is family
0 only.

Per-family rock census at 512x320 seed 1, beside round 1's, as the counter
that says the mechanism fired (0 neutral / 1 wet / 2 dry / 3 cap-rock):

```
rolling  round 1: 41892 / 29469 /     - / 14098      now: 37443 / 35201 /  1200 / 13968
canyon   round 1: 58863 /  9327 /    54 / 15907      now: 55197 / 10835 /  3334 / 15719
arid     round 1:  9531 /     - / 84558 /  2788      now: 16291 /  1048 / 74042 /  4011
wetland  round 1:     - / 80183 /     - /  1004      now:  4717 / 75037 /  1615 /  2349
```

`-` is a family round 1's finding did not list, which for that census means
absent; the widened ramps are why every preset now draws all four. The
direction to read is that the minority families gained -- canyon's dry went
54 -> 3334, which is the "warm country" the review wanted the grey to be
intergrown *with*.

Images: `target/filmstrips/task2-after-canyon-s{7,13}.png` against
`task1-after-canyon-s{7,13}.png` as the before, plus
`task2-after-{rolling,wetland,arid}-s1.png` for collateral, which is clean --
no confetti, no family appearing where its country is not.

Reproduction:

```
cargo test --release --test worldgen probe_r2t2 -- --ignored --nocapture
```

### R2-3 — Vaults: the pass works, and three things about it did not survive contact with the code

A `vaults` pass (margin **32**, finite and derived: `MAX_VAULT_EXTENT` 30 +
`VAULT_RIND` 2 + the shape test's 1 cell of scan margin, rounded up),
`crystal.ron` and `shard.ron` appended to `EMBEDDED`, `Purpose::Vault` = 19,
and three preset params (`vault_density`, `vault_min_depth`,
`vault_bedrock_margin`). The collect-then-verify-seal skeleton is `pockets`'
own, kept whole.

**It fires.** At the shipped 2048x640, chambers place in 5 of 8 seeds on
every preset; `rolling` seed 4 writes 267 cells, seed 7 213, seed 1 161. The
cross-sections are in `probe_r2t3_dump_a_chamber` and are the acceptance
artifact for shape: a vug comes out as a crystal ring around an air dome
over a pool over a flat gravel floor, a grotto as a lumpy multi-lobe cavern
with the same interior.

#### 1. The size cap was capping the wrong thing, and it broke the seal

First version capped the *scan box* at `MAX_VAULT_EXTENT`. A lobe reaching
past the cap then had its far end never visited -- so those cells were
neither written **nor seal-checked**. The chamber came out with a flat
sawn-off face and the guarantee that the whole envelope is stone quietly
stopped covering all of it. The cap now applies to the lobe radii, so the
shape is always inside the box and the scan always covers the whole
envelope. This is CLAUDE.md's twice-written landmine arriving in a third
costume: *a size cap must bound work, never gate whether something happens*
-- here it was silently deforming the thing it bounded.

#### 2. "Floor filled flat" read literally is not a floor

`floor_y` is the lowest row the hollow reaches, and filling *that row* is
what the phrase says. It leaves the chamber's curved bottom as bare stone
with a two-cell strip of gravel at the very bottom of the bowl. The floor is
now every hollow cell from a chosen row downward, which makes the gravel's
top surface a horizontal line **by construction** -- and that is structural,
not cosmetic: gravel following the curve is loose powder on a slope at every
cell and runs on frame one.

#### 3. The water level is set by the chamber's *shape*, not by the table alone

The spec's rule -- standing water when the floor sits below the local water
table -- is implemented, and at `vault_min_depth: 200` it is **unconditional**:
the table is always hundreds of rows above the depth band, so every chamber
in every world qualifies. Filling to the table therefore means filling to the
ceiling, and that does not hold still.

Measured rather than reasoned about. `rolling` seed 1 lost **exactly one
cell**, at (70, 257): the single hollow cell on row 257 of a chamber whose
next row down is fourteen wide. A one-cell column of water standing on a wide
body is a head difference, and the liquid solver drains it, which is correct
behaviour by the solver and a bad chamber by me.

Two fixes were tried and the first is recorded as wrong rather than untried:

| rule | result |
|---|---|
| surface row must be >= 3 cells wide | `rolling` s1 fixed, `rolling` s4 lost 6 cells |
| surface row must be >= 5 cells wide | `rolling` s4 still lost 6 cells off a 6-wide row |
| **surface is the chamber's widest row** | **green, every preset x 5 seeds** |

The quantity was never absolute width. A chamber is an ellipse (or a union of
them), so filling to any row *above the equator* makes a **flask** -- a narrow
neck standing on a wide body -- and the neck drains. Filling to the equator
makes a **bowl**, where every row below the surface is narrower than it, which
is the shape a pond has and the shape that holds.

The side effect is the better picture, which is worth saying plainly because
it was not the goal: the upper half of a flooded chamber is now a pocket of
trapped air under the roof rather than solid water. That is what a sealed
flooded void actually contains, and it is a far better thing to break into.

#### The depth band does not exist at the sweep's world size

`vault_min_depth: 200` plus `vault_bedrock_margin: 16` needs about 250 rows of
massif. The shipped world is 2048x640 and has it; **the sweep, `filmstrip` and
the whole `tests/worldgen.rs` suite run at 512x320, where the surface sits
around y 100-200 and bedrock around y 300, so the band is empty and no chamber
can ever be placed.** Consequences, all handled rather than left implicit:

- The sweep's new `vaults` row is `0 0` for all six presets, and will stay
  that way. **The sweep cannot guard this pass at its current size** -- which
  is exactly the brows/talus blindness the sweep exists to prevent, arriving
  from the other direction. Recorded here because a future reader seeing a
  row of zeros should know it is by construction and not a regression.
- `every_pass_writes_something` would have failed, and correctly. Rather than
  excusing `vaults` from the guard, it now asserts **zero at 512x320 and
  non-zero at 2048x640** -- so the guard still has teeth, and it documents the
  size constraint where someone will hit it.
- The at-rest and seal tests build with `vault_min_depth: 40`, which is stated
  in `vault_test_params` as a fact about the world size rather than a
  convenience.

**For the reviewer**: if the sweep should guard this pass, the lever is either
sweeping worldgen at the shipped size (96 runs of a 4x larger world) or
expressing the depth as a *fraction of the massif's thickness* the way
`pockets` already does -- its own comment says why ("a canyon massif is five
times the depth of a wetland one and 'near bedrock' has to mean the same thing
in both"), and that argument applies here word for word. Not done unilaterally
because it changes what the shipped `200` means.

#### Gates and the seal contract

`a_forced_vault_world_is_sealed_and_arrives_at_rest` is a **paired build**
against the same world with `vault_density: 0.0` -- exact here, because the
pass writes nothing unless it writes a whole chamber and nothing downstream
reads a vault, so every difference is a vault cell and no difference is
anything else. Inferring vault cells from *where they are* is the mistake
that miscounted twice in round 1's task 4. It asserts every written cell was
stone beforehand, every 8-neighbour of a written cell was stone or is itself
part of the chamber, and zero cells move in 120 frames -- across three presets
x five seeds, with a counter (>= 8 worlds must actually have placed one) so
the test cannot pass by never running.

`vault_water_cannot_wet_the_massif_around_it` states the moisture-inert claim
as the task asks. The reason is structural, not lucky: `soil_moisture` writes
only to cells with non-zero `water_capacity`, and **soil is the only material
in the registry that has one**, so a chamber sealed in rock has nothing in
reach it could wet even though its water does seed the distance transform.

`a_world_with_no_vaults_is_byte_identical` pins the opt-out by world hash.
`flat` ships `vault_density: 0.0` for the same reason it opts out of palette
families -- the destruction workstream compares against its renders.

Sweep re-baselined, as the task instructs, to add the `vaults` row. That
refresh also absorbs tasks 1 and 2's sub-threshold movement; those numbers are
preserved in the two commit messages and in findings R2-1 and R2-2.

#### One file outside the owned set

`examples/viewshot.rs` gained a `vault=1` mode: it locates a chamber, aims the
camera at it, and sinks a shaft from the surface into it on the second shot.
Flagged rather than slipped in, the way round 1's finding 3 flagged
`material.rs`/`world.rs`/`decay.rs`. It was necessary: the task asks for a
mined strip showing a breach, and **every other rendering path in the repo is
incapable of showing a vault** -- `filmstrip` builds at 512x320 where no
chamber exists, and `viewshot`'s camera aims at the skyline, which is 200+
rows above the subject. The change is additive and default-preserving; no
existing invocation renders differently.

**On reading the strips**: at 1:1 a chamber is 40-60 cells across in a
512-wide viewport, so it renders as a small dark hole with a pale rim -- the
`task3-vault-*` strips show it *in context* and show the shaft reaching it,
but they are not where the structure can be judged. The ASCII cross-sections
are, which is why the probe prints them: a render at the zoom a contact sheet
is read at cannot distinguish a lined vug from an unlined grotto, and those
are the two shapes this task delivers.

Images: `target/filmstrips/task3-vault-{rolling-s4,canyon-s7}.png` -- three
shots each, the third with a shaft sunk from the surface into the chamber.

Reproductions:

```
cargo test --release --test worldgen probe_r2t3 -- --ignored --nocapture
cargo run --release --example viewshot -- seed=4 preset=rolling vault=1 shots=3
```

### R2-4 — Water's fourth tone: the one-line fix, and the test that has to fail for the *right* reason

`assets/materials/water.ron` gains a fourth colour, exactly the midpoint of
the two lightest it already had: `(68, 121, 213)` between `(64, 116, 208)` and
`(72, 126, 218)`. No new hue, and the set still spans the range it always did
(58 / 64 / 68 / 72 in red) with the gap between the two lightest filled in.

**Measured before and after, which the one-line description does not need but
the claim does.** `ponds` draws shades 0..3 and `render.rs` colours a cell
`palette[shade % palette.len()]`, so against three entries shade 3 folded onto
entry 0. Censusing rendered tone across `wetland` x 5 seeds, 8,616 water cells:

| | entry 0 | fair share |
|---|---|---|
| three colours | 4,293 (49.8%) | 33.3% -- **1.49x** |
| four colours | even, every entry within 25% of 1/4 | 25% |

**The test asserts the distribution, not the palette length**, and that
distinction is the whole of it. A length check would pass just as well for a
future change that adds a fifth colour without touching `TONES = 4`, which
reintroduces exactly this bug pointing the other way -- entry 4 would never
draw at all. Asking the question the bug is actually about survives that.

Deliberately broken to check it fails for the right reason, per the repo's
rule about a guard that must be able to fail: with the fourth colour removed
it reports `water tone 0 drew 4293 of 8616 cells, 1.49x its fair share of
1/3`, which is the bug named in its own terms rather than a length mismatch.

**One consequence worth stating**: the brush now paints water in four tones
rather than three, because `world.rs`'s `paint_stroke` draws its entry from
`base_shades`, which is the whole list for any material without a
`base_colors` cap. That is the same evenly-weighted set generated water uses,
so painted and generated water match, which they did not quite do before.

`fill_dimming` is untouched, as instructed -- it is `0.0`, that is round 1's
finding 1c, and it is an open owner question rather than this task's.

Images: `target/filmstrips/task4-{before,after}-wetland-s1.png`. They look the
same, and that is the correct outcome: an interpolated tone drawn a quarter of
the time instead of a third changes the weighting of a three-point-wide
brightness spread. The census is the evidence here; the strip only confirms
nothing else moved.

---

# Round-3 findings

Round 3's queue is `Reports/worldgen-implementation-tasks-round3-2026-08.md`;
its findings are appended here, as that file instructs.

### R3-1a — The two-sub-threshold sketch does not survive the geometry; the shipped rule is the research's own single threshold

The task sketches `t_chamber` carving discs around Worley cell *centres* and
`t_passage` carving the boundary web, with the caveat "measure and look
rather than trusting this sketch". Derived before building, because the
failure is geometric rather than a tuning matter: a disc of radius
`t_chamber` around a feature point and the strip `F2 - F1 < t_passage`
around the cell boundary **never touch**. For two feature points a unit
apart, the strip reaches inward only to `(1 - t_passage) / 2` from either
point -- at the sketch's own scales (`t_chamber` ~ 0.3, `t_passage` ~ 0.12)
that leaves ~0.14 of solid stone between every chamber and every passage, so
each chamber is a sealed satellite that the mandatory component-keep then
deletes, and the "system" collapses to whichever single blob contains the
seed. Closing the gap needs `t_passage >= 1 - 2 * t_chamber`, at which point
the union carves most of the envelope and there is no anatomy left.

What ships is the rule the research itself states
(`Reports/worldgen-design.md` §7, quoted at the top of the round-3 task
file): **one threshold on `F2 - F1`**. The passages are the Voronoi edge
network; the chambers are the junction bulges where three edges meet, which
open out because the low-`F2 - F1` bands of three boundaries overlap there.
One field, one threshold, and the linked anatomy is a property of the
geometry rather than of two constants that have to be kept in agreement.

### R3-1b — "Bounding box + rind, all stone" is read as the r2 skeleton's dilated envelope, not a literal box

The task's seal line says "bounding box of the kept component + 2-cell
rind, all stone". Read literally that is a *stronger* check than round 2
shipped -- the r2 skeleton never checked a box; it checked the **shape
grown by the rind** (`inside(fx, fy, VAULT_RIND)`), which is what
"envelope" has meant in this pass since `pockets`. For a ~180x70 system
the literal box covers ~13,000 cells and overlaps roughly three of
`pockets`' 64-cell regions, so a sand lens tens of cells from the nearest
void -- sealed in its own right, incapable of touching the cave -- would
reject the whole system, and the pass would fire noticeably less often for
no at-rest gain. The seal shipped is the raster equivalent of the r2
check: every cell within a 2-cell Chebyshev dilation of the kept component
must be solid stone, verified in full before a single write, else the
system is rejected wholesale. The contract the round-3 task marks
must-not-change -- whole envelope + rind verified before any write --
is exactly what this is.

---

## Track summary — what changed, and what the next session should know

Six tasks, six commits, all gates green at each one. What the world looks
like now, against the review's own list of what it cost the picture:

- **Problem 2, "presets don't differentiate at play scale"** — a region's
  `Character` now picks the rock, soil and sand *palette family* at genesis
  (task 3), so crossing an escarpment crosses into different country at zero
  frame cost. Canyon s7 and arid s1 are where it is unmissable.
- **Problem 4, "arid dunes read as a mechanical sawtooth"** — partly. The
  premise was measurably wrong (spacing already varied, cv 0.42); the real
  uniformity was in height and was caused by the preset asking for more
  amplitude than repose allows. Fixed by varying downward from the cap
  (task 5).
- **Problem 5, "the keyhole artifact"** — traced, not fixed, as instructed.
  It is `round()` in the terrace snap, not the mask edge, and the step is
  exactly `terrace_step * mask` rows (task 1). Riser roughening (task 5)
  makes each one shorter and more numerous rather than rarer.
- **Problem 6, "pale dashes on pond surfaces"** — settled: submerged
  shoreline sand at the pond bed, not a water artifact, and not caused by
  `fill_dimming: 0.0` either (task 1).
- **The blue slivers** — sky through a gap, with zero water cells anywhere
  in either world (task 1).
- **"brows 34 / talus 148, too rare to register"** — rescued at region
  scale, p90 up 140-350% across every preset, 100% of the cells landing at
  real cliffs (task 6). Pockets stopped reading as polka dots and buried
  gravel stopped being invisible (task 4).

**Left for the reviewer, in rough order of how much they matter:**

1. **Three files outside this track's owned set were touched** —
   `src/sim/material.rs`, `src/sim/world.rs`, `src/sim/decay.rs` — because
   widening a palette split what `palette.len()` meant. Full reasoning in
   finding 3; `src/app.rs::spawn_burst` was deliberately left alone.
2. **Generated worlds do not stay asleep** (finding 2): 3-8 of 40 chunks
   awake at frame 100 on every preset including `flat`, with active sites
   climbing to one per column. Pre-existing, points at
   `src/sim/structural.rs`, and it matters for landmine 7.20.
3. **`talus`'s declared margin was wrong by 40x** before this track and is
   now 200 (finding 6). Any streaming work inherits that.
4. **`wetland` seeds 1-3 have no cliffs at all** (finding 6) — a relief/
   terrace-strength question, not a detection one.
5. Two design choices in task 3 that are cheap to overturn: per-cell white
   noise for the palette dither (a lower-frequency mottle would read more
   like facies change), and `region_variation <= 0` opting a preset out of
   families entirely.

**Method notes worth carrying forward.** Six metrics were written wrong in
this track and every one failed the same way — it measured where a thing is
instead of asking what produced it, or it was written before anyone had
looked at the subject. A "water-coloured pixel" count matched the entire
sky; a notch detector comparing 3 columns out found none in a world with a
7-column notch; a crest detector using a 4-column window found zero crests
in a world with 35; a crest *height* measured the hill it sat on; two gravel
classifiers inferred the writing pass from the cell's position; a relief
metric looked only downhill and called 91 of 216 correctly-placed brow cells
misplaced. The fix in four of the six was a **paired comparison** — build
the same world with the mechanism switched off and diff — which is exact
rather than merely better, and is now the shape every counter in
`tests/worldgen.rs` uses.

---

## Round-2 summary — four tasks, four commits, and what the reviewer owns

All gates green at each commit: `cargo test`, `cargo clippy --all-targets --
-D warnings`, the at-rest suite, and the task-2 sweep.

- **Keyhole risers (R2-1)** — terracing now yields on steep ground. canyon s7
  31 rows → 15, terraced s2 20 → 12, canyon s13 16 → 9; mesas intact.
  **18 of 20 worlds clear the pre-registered ≤ 18 bar; `rolling` s1 and s2 do
  not**, and cannot be reached by this mechanism because their tall risers
  stand on *gentle* ground — the country the spec keeps at full snap.
- **Palette piers (R2-2)** — a slow 2-D field plus wider aridity ramps.
  canyon v/h 0.58 → 0.75 and 0.56 → 0.74; the metric also showed the artifact
  is **canyon-only**, so the knob is per-preset. Shade-only, proved by a
  sweep `compare` byte-identical to task 1's.
- **Vaults (R2-3)** — a sealed `vaults` pass with two shapes, two new
  materials, a 2-cell rind and a paired-build seal test. Chambers place in 5
  of 8 seeds at the shipped size and arrive at rest.
- **Water tones (R2-4)** — a fourth interpolated colour. Entry 0 went from
  1.49× its fair share to even.

**Decisions left to the reviewing session, in the order they matter:**

1. **`rolling`'s two tall risers (R2-1).** Slope attenuation cannot reach
   them. `terrace_step: 18` (from 26) clears the bar on all four rolling
   seeds with no count zeroed — measured, not proposed — but it re-spaces
   every bench on the preset, which is a landform call. One runtime-loaded
   `.ron` line.
2. **The sweep cannot see the vault pass (R2-3).** `vault_min_depth: 200`
   needs ~250 rows of massif; the sweep, `filmstrip` and the whole test suite
   run at 512x320, where the band is empty. The `vaults` row is `0 0` for
   every preset and will stay so. `every_pass_writes_something` covers the
   pass at the shipped size instead. Lever: sweep at the shipped size, or
   express depth as a fraction of massif thickness the way `pockets` already
   argues for.
3. **Every chamber floods (R2-3).** The spec's rule — water when the floor is
   below the table — is unconditional at a 200-row depth. Air now occupies
   the dome above the waterline because the water level had to become the
   chamber's widest row for at-rest reasons, which improved the picture by
   accident. Whether some chambers should be dry is an owner call.
4. **`examples/viewshot.rs` was touched** (a `vault=1` mode), outside the
   owned set, because no other rendering path in the repo can show a vault.
   Additive and default-preserving.
5. Round 1's open items still stand, in particular that **generated worlds do
   not stay asleep** (finding 2) and **`talus`'s margin is 200** (finding 6).

**Method notes from this round**, all the same shape as round 1's:

- **A metric can be isotropic by construction.** The first columnarity metric
  measured per-cell run lengths and read 0.99 everywhere, before and after,
  because the per-cell dither is white noise — it was measuring the stipple
  and could never see the band. Caught by the answer being *identical in
  cases known to differ*.
- **A metric can compare a thing to itself.** The first slot/bluff metric
  included `k = 0` and so reported "fully recovered" for every step in every
  world. Caught by unanimity.
- **A size cap can deform what it bounds.** The vault cap was applied to the
  scan box, so an oversized lobe was neither written nor *seal-checked*.
  CLAUDE.md's landmine in a third costume.
- **The quantity that fixes a bug is often not the one the bug is stated in.**
  Draining chamber water looked like a minimum-width problem and was a
  *shape* problem: fill above the equator makes a flask, fill to it makes a
  bowl. Two width bars failed before the shape rule worked.
- **A guard must fail for the right reason.** The water-tone test asserts the
  rendered distribution, not `palette.len()`, so it also catches the inverse
  bug; deliberately broken, it names the 1.49x fold rather than a length.
