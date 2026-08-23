# Phase 2: the world is four times bigger, and so are the caves in it

*The record of `Reports/world-scale-handoff.md`'s Phase 2. Written to be
picked up cold. Branch: `claude/world-scale-handoff-phase-2-scsenh`.*

## What Phase 2 was for

Round 6's renders were rejected in these words: *"everything needs to be
bigger, the whole world, the caves. You cannot create good looking crystals
or stalagmites and stalactites that are only 1-2 pixels wide."* Round 7
bought the performance that made a bigger world affordable. Phase 2 spends
it: `WORLD_WIDTH`/`WORLD_HEIGHT` go to **8192 x 2560**, and the features that
had no room to have a shape grow into the room that makes.

The handoff predicts the outcome and it held: **Phase 2 alone looks worse in
one specific way, and Phase 3 is where that is fixed.** The numbers for it
are in §4, so Phase 3 starts from a measurement rather than an impression.

## 1. What actually changed, and what deliberately did not

The handoff asks to "re-derive every worldgen dimension" and then names a
list: cave envelope bounds, formation widths and heights, boulder and
residual sizes, strata thickness, pocket lens sizes, talus and brow reaches.
Taken literally that list is not self-consistent, and finding out *why* is
most of what this phase learned.

**The surface composition cannot scale, and the handoff's own arithmetic
says so.** It records that `MAX_TOTAL_REGIONS = 64` "clamps a 4x world that
wants up to 80" — 80 is `8192 / COMPOSITION_WINDOW * MAX_REGIONS` with
`COMPOSITION_WINDOW` still 512, i.e. regions stay *one screen* wide. That is
`region.rs`'s central promise ("regions per window, not per world") and it is
what stops a wider world getting duller. But a screen-wide region with 4x the
elevation spread between neighbours is a 4x steeper escarpment across the
same 215 columns of transition — from 0.31 rows per column to 1.25 — and
`TRANSITION`'s own doc records that a sustained moderate slope is the worst
case for loose cover. Scaling relief while regions stay window-sized takes
the soil off the world at every region boundary.

So relief, hills, sky, terracing, dunes and soil depth are **unchanged**, and
the composition a player sees per screen is the composition round 7 shipped.

**Residuals, boulders, talus and brows follow the surface, not the world.**
A residual stands *up* from the ground into `sky_rows + relief_amplitude` of air — 141
rows for `rolling`, and `MAX_HEIGHT = 120` already nearly fills it. A 4x
residual would be built off the top of the world. Talus is already bounded by
the cliff it sits under (`peak = ...min(fall / 2)`), and a cliff is a
surface feature. None of them can move until the surface does. Residuals are also
the pass Phase 4 exists to delete (`residual_density: 0.0` is its stated
first step), so growing them now would be tuning something on its way out.

Boulders are the same case one step further down: `boulders` seats a cluster
on an erosion *shed marker*, which is a plan-space record of hard rock that
retreated off a surface slope. Its size draw (3-13 cells of base width,
height capped at 3x that) is a claim about a block that fell off a cliff, and
the cliff did not get bigger. Scaling it would put thirteen-metre boulders on
a hillside whose whole relief is 76 rows.

**What is left is what lives underground, where the 4x world genuinely
made room** — the massif went from about 470 rows to about 2400 — and it is
also exactly what the owner's sentence names: caves, crystals, stalagmites,
stalactites.

| Constant | Round 7 | Phase 2 | Why |
|---|---|---|---|
| `MIN_CAVE_HALF_W` / `_H` | 55 / 22 | **220 / 88** | 4x |
| `MAX_CAVE_HALF_W` / `_H` | 200 / 80 | **800 / 320** | 4x |
| `ROUND_3_HALF_W`, `CAVE_CELL` | 90, 22.0 | **unchanged** | the *denominator*; see below |
| `SPELEO_WIDTH_MIN` / `_MAX` | 3 / 8 | **12 / 32** | the complaint, literally |
| `SPELEO_SPACING_MIN` / `_MAX` | 9 / 28 | **36 / 112** | count, spacing and width are one budget |
| `DRIP_SCALE` | 40.0 | **160.0** | the clustering field the spacing reads |
| `MAX_VAULT_EXTENT` (vug) | 30 | **120** | with its semi-axes and lining |
| cave floor / breakdown mounds | 2-4 rows, ≥20 wide | **8-19 rows, ≥80 wide** | a cavity is 4x wider |
| the span gates on formations | 5, (3,5), 7, 2 | **20, (12,20), 28, 8** | see §3 |
| `MAX_TOTAL_REGIONS` | 64 | **320** | it was clipping the draw |
| `MAX_CEILING_SPAN` | 36 | **unchanged** | a roof-*structure* bound, not a cave-size one |
| `strata_thickness`, pocket lenses | 9, 8-60 x 1-6 | **unchanged** | see §5 — a question for the owner |

**Two constants deliberately did not move, and that is the mechanism rather
than an omission.** `CaveEnv::cell()` is `CAVE_CELL * half_w /
ROUND_3_HALF_W`: the reference is the denominator every cave-space length is
expressed against, so scaling it alongside the envelope would leave every
ratio unchanged and produce a bigger box with the same furniture in it.
That is precisely what round 6's A2 measured and rejected — with the lattice
cell held fixed, span across reached its target while largest-walkable fell
38% to 23%, because the extra area went into finer structure the player
cannot occupy. Leaving the reference alone is what makes a 4x envelope a 4x
*cave*, and it carries the edge fades, `min_system_cells` and the monumental
chamber's `chamber_scale` for free.

## 2. Three things that were silently broken at the new size

Each passed every test it had.

- **`arid` and `flat` were no longer dry.** Their `table_offset: 400` is a
  "past the world floor" sentinel and 400 stopped being past the floor two
  size changes ago; at 2560 rows the table sat at row ~570 with two thousand
  rows of world under it. `ponds` still found no basin, which is why nothing
  caught it, but `moisture_init` writes a moisture floor for every row at or
  below `table_y`. The preset whose whole job is *no water anywhere* was damp
  from a third of the way down, and the structural test bed was answering a
  wet-rock question. The bar that should have caught it was
  `assert!(table_offset > 320.0)` — **a literal**, inside `worldgen`, which
  cannot see `app`. Now 4000, asserted against `WORLD_HEIGHT` from an
  integration test.

- **`MAX_TOTAL_REGIONS = 64` clipped the region draw.** It bites above
  `w = 6553` and `regions_stay_window_sized_as_the_world_grows` only checked
  to 4096. The guarantee `region.rs` exists to hold stopped holding, in
  silence, at the first size that reached it.

- **`cargo test --lib` went from about a minute to past ten.** 25 test bodies
  call `App::new()`, now 9 s and 359 MiB each, several at once across cargo's
  threads. `App::build` takes a size and the tests build at 2048x640 — the
  size they all passed against the day before, so it changes what they
  measure by nothing.

**One shipped-size test remains and it earned itself on the first run.**
`a_shipped_size_world_is_generated_and_at_rest` first asserted zero awake
chunks within 20 frames, and failed. The world was at rest the whole time:
5120 chunks take longer to walk down to sleeping than 320 did — 4936 awake on
frame 1, 35 by frame 60, and **108 mineral cells of 19,834,655 moved** in
between. An awake chunk has been *scheduled for a sweep that will confirm it
is still*, which is not a cell having moved; the same
failure-count-is-not-a-damage-count distinction `CLAUDE.md` records. It
censuses minerals now.

## 3. The cave tests need a world that can hold a cave

`MIN_CAVE_HALF_W`/`_H` at 220/88 means no cave can be placed in a world
narrower than about 450 columns or shallower than about 400 rows — the
placement is rejected outright, by the same rule that has always rejected a
world too shallow to hold one. `tests/worldgen.rs` builds at 512x320 for
speed, so all five forced-cave tests went from passing to *"only 0
forced-vault worlds actually placed a chamber"* — which is the counter beside
the claim doing exactly its job. They build at 2048x640 now.

This is not a test-only fact. **A 4x cave needs a 4x world**, and any future
harness that builds a small world to look at caves will find none.

## 4. What the census says, and the two numbers that got worse

`examples/cave_probe.rs`. Three builds, **paired**, because a single column
against a remembered number is the comparison this repo keeps getting wrong:

- **round 7** — the shipped 2048x640 world, 16 seeds.
- **the control** — 8192x2560 with round 7's cave constants, 8 seeds. Built
  from a worktree at the commit before the feature work, which is the only
  way to separate "the world grew" from "the caves grew".
- **Phase 2** — 8192x2560 with the table in §1, 8 seeds.

`canyon`, because that is the preset the control was run on. Medians over
systems unless the column says otherwise.

| | round 7 | control: 4x world, round-7 caves | Phase 2 |
|---|---|---|---|
| **void as % of the deep massif** | **0.591%** | **0.041%** | **0.605%** |
| systems per world | 1.3 (5 of 16 with none) | 0.8 (5 of 8 with none) | 2.6 (3 of 8 with none) |
| **formation base width** | med **3**, p90 6, max 7 | med **3**, p90 6, max 7 | med **11**, p90 17, max 31 |
| formation height | med 4, p90 18, max 65 | med 4, p90 51, max 80 | med 16, p90 72, max 357 |
| span across | med 73, p90 282, max 361 | med 84, p90 291, max 376 | med 148, p90 811, max 1544 |
| reachable by player % | med 34, p90 63 | med 45, p90 78 | med 55, p90 90 |
| largest walkable % | med 33, p90 63 | med 41, p90 78 | med 37, p90 83 |
| **walkable regions** | med 1, p90 1, max 3 | med 1, p90 2, **max 5** | med 1, p90 35, **max 92** |
| contrast p95/med | med 483 | med 510 | med 327 |
| near-pairs (per preset) | 10 | 2 | **0** |
| `vaults` pass wall time | 10-22 ms | — | 42-870 ms |

### 4a. Correction, 2026-08-23: the ruler was too deep, and two of the readings above move

**Every number in the table was measured through a census window that starts
below where most caves are.** `cave_probe`'s `deep_y()` returned
`WORLD_HEIGHT / 2`, but `vaults` places from `plan.surface_y +
vault_min_depth` — about row 341, because `sky_rows` is 95 and does not scale
with world height, so the band's top barely moves when the world does. At the
2048x640 the probe was written against, `WORLD_HEIGHT / 2` was 320 against a
true ~341 and the window was **right by arithmetic accident**. At 8192x2560
it is 1280, and the census read the bottom half of a band that starts a
quarter of the way down. This is the same shape as the two defects §2 records
— a literal that stopped matching when the world went 4x — and it was inside
the *instrument* rather than the generator, which is why nothing caught it.

Re-measured paired, one build, one session, canyon, 6 seeds, the shipped
window against the pass's own:

| | window 1280 | window 341 (true) |
|---|---|---|
| systems (6 seeds) | 20 | **26** |
| worlds with **no** system | 1 | **0** |
| formations | 166 | **244** |
| formation base width, med | 11 | **11** |
| contrast p95/med | 2.68x | **3.68x** |
| walkable regions, p90 | 36 | **29** |
| walkable regions, max | 92 | **92** |
| void % of the deep massif | 0.806 | 0.562 |
| tallest open column, med | 35 | **53** |

**Two conclusions above move.**

- **Cave rarity was partly the ruler.** "3 of 8 with none" in the Phase 2
  column, and "5 of 8" in the control, count some worlds whose systems sat
  *above* the window rather than worlds without one. How much is a **sample
  size question, and six seeds is not enough to answer it**: on the corrected
  window canyon has a system in all six seeds above, but over **16** seeds it
  is 4 worlds with none (2.5 systems/world). Take the 16-seed figure — the
  six-seed one is exactly the single-sample claim this repo keeps having to
  retract.
- **The contrast fall is a quarter, not a third** (483 -> 368, not -> 327).
  Real, and smaller than stated.

**Two do not, and that matters more.**

- **Formation base width med 3 -> 11 is identical in both windows.** The
  complaint that started the round is answered, and the correction does not
  touch it.
- **`walk_regions` max is 92 either way.** The fragmentation §8 hands to
  Phase 3 is real at any window.

The void fraction is now 0.562% against round 7's 0.591% — and round 7's was
measured at 2048x640, where the old window was accidentally correct, so the
two are comparable and the "this is a *scaled* world" check in the next
paragraph still holds. It is if anything better supported.

`deep_y()` is now a per-column profile (`surface(x) + vault_min_depth`),
derived rather than written down: a single number here has been wrong once
and right by luck once, so a third literal would be the same bug with a fresh
number. `deep=N` overrides it flat, which is how the pair above was measured
and how the next size change can re-check this in one build instead of two.

**The corrected window, swept properly: 16 seeds x every preset.** This is
the baseline Phase 3 should be read against, and it says the two Phase 2
findings are systematic rather than a canyon accident:

| preset | systems (16 seeds) | walk_regions med / p90 / max | contrast p95/med | formation base width, med | void % |
|---|---|---|---|---|---|
| `arid` | 30 (4 with none) | 4 / 44 / **98** | 4.28x | 10 | 0.284 |
| `canyon` | 40 (4 with none) | 2 / 39 / **92** | 3.71x | 11 | 0.318 |
| `rolling` | 37 (3 with none) | 3 / 44 / **89** | 4.00x | 11 | 0.316 |
| `terraced` | 35 (4 with none) | 3 / 36 / **87** | 3.94x | 11 | 0.316 |
| `wetland` | 39 (4 with none) | 3 / 25 / **92** | 4.14x | 12 | 0.323 |
| `flat` | none in any seed | — | — | — | — |

Formation base width is **10-12 in every preset**, so the answer to the
owner's complaint holds everywhere, not just where it was measured. And
`walk_regions` reaches **87-98 in all five**, with the *median* at 2-4 rather
than 1 — the fragmentation is a property of the mechanism, not of a seed.
`flat` has no cave in any of 16 seeds, which is its `table_offset` sentinel
and relief doing what §2 fixed them to do.

**Not re-derived: the round-7 and control columns.** Both would need a
worktree build at a pre-Phase-2 commit, and neither changes the direction of
anything — the gap is recorded here rather than relabelled away.

**The control is the justification for the whole phase, and it is starker
than the handoff's sentence.** *"A feature that stays the same size in a 4x
world has become 4x less significant"* — measured, it is **fourteen times**
less significant, because significance here is area and the world grew
sixteenfold. Cave void fell from 0.591% of the deep massif to **0.041%**,
and five of eight worlds had no system at all. A 4x world with round-7 caves
is a world with essentially no caves in it. Phase 2 puts it back to 0.605%,
which is the check that this is a *scaled* world and not a world with more
cave in it: the same rare systems, each sixteen times the area, in a world
sixteen times the area.

**The complaint is answered.** Median formation base width went from 3 cells
to 11 and median height from 4 to 16 — and the control column shows both were
untouched by the world growing, so the whole of that is the speleothem work.
Three cells has no silhouette, no taper and no interior at any zoom; eleven
has all three.

**The distribution split in two, and that is the Phase 2 regression.** The
good systems got much better (p90 largest-walkable 63% -> 83%, reaching 97%
on the best seeds) while the count of *disjoint* walkable regions went from
at most 5 to as many as 92, and chamber-to-passage contrast fell by a third.
A system of ninety-two separate walkable pockets is ninety-two caves to the
player walking it, which is the exact question `walk_regions` was added to
ask. **The control pins this on the cave growth, not on the world growth**:
at 4x world with round-7 caves the figure is max 5, indistinguishable from
round 7's max 3.

This is the handoff's prediction arriving on schedule — *"it makes the cave
honeycomb larger, not better ... Phase 2 and Phase 3 are judged together, or
the first strip after Phase 2 reads as a regression"* — and Phase 3's two
candidates (warp the lattice; carve by process) are aimed at exactly this.
**Do not tune the constants in §1 to chase these two numbers.** They are the
honeycomb, and the honeycomb is what Phase 3 replaces.

**`near-pairs` fell 10 -> 2 -> 0, so both changes took some of it.** A
near-pair is a stalactite and stalagmite whose tips end within 3 cells of
each other. Most of the loss is the world growing (10 to 2, with the cave
constants untouched); the last of it is plausibly Phase 2's raising of the
floor below which a formation half is dropped, from 2 rows to 8 — a two-row
stub under a 12-32 cell base is a wide flat lump, not a formation, but if
one half of a pair shrinks under 8 the pair is now discarded where it used to
be kept. At n = 2 the control is a weak signal and this is not settled.
**Attribute it before Phase 3 tunes anything near it**: the cheap experiment
is `SPELEO_WIDTH_*` at Phase 2 values with that floor back at 2, one preset,
`cave_probe seeds=8`.

## 5. Left alone on purpose, and needing the owner's eye

**`strata_thickness` and the pocket lenses move together or not at all.** A
lens is deposited *within* a bed — `pockets` rotates every ellipse onto the
local strata dip precisely so it sits in a visible band rather than cutting
across one — so a 4x lens in a 9-row bed breaks the thing the pass was built
to say. And `strata_thickness` is not only an underground quantity: it feeds
`HardnessField`, which drives plan-space erosion, which is what makes the
mesas and benches the handoff singles out as *working*. Changing it is a
change to the surface.

It is also a `.ron` parameter the owner can turn with F5 in seconds and judge
by eye, which is what this repo does with by-eye questions instead of arguing
them.

**Answered, and against the session's own prediction.** The blind A/B (card
`20260822T043758278Z-018304`) came back for the *shipped* 12: *"Option B has
the better pattern"*, and `blind_was: [1, 0]` resolves B to
`strata_thickness` 12. So `strata_thickness` stays as shipped in every preset,
the lenses stay with it, and the scaling argument above is recorded in
`Reports/dead-ends.md` with the condition that would reopen it. This is the
whole reason for blinding a card you have a stake in: the session predicted
48 and would have shipped it.

Note what the verdict does *not* cover. The card showed a **surface** strip,
so the owner judged the surface pattern; the underground half of the argument
-- that a 4x cave shears along bedding four times finer than the bedding was
tuned for -- was never put in front of anyone. Phase 3 may want to ask it
again, with a picture taken *inside* a cave.

## 6. Costs, measured on one machine in one session

| | 2048x640 | 8192x2560 |
|---|---|---|
| generation | 459 ms | **9010 ms** (place 4999 + structural 4012) |
| peak RSS | — | **359 MiB** |
| settled worst frame, field | — | **51.77 ms** |
| settled worst frame, sweep | — | **0.17 ms** |
| worst frame during the post-load settle | — | **648 ms** |
| `river-cost` scene, spring off | worst 16.1 ms, mean 6.7 | worst **73.4 ms**, mean **10.9** |
| `river-cost` scene, spring on | worst 12.8 ms, mean 7.0 | worst **80.3 ms**, mean **16.3** |
| `cargo test` (whole suite) | ~1 min | **4 m 56 s**, 713 pass |

Generation is 19.6x for 16x the area, so it is slightly worse than linear;
the loading screen covers it. RSS matches the handoff's 358 MiB. The settled
field worst frame is **better** than the 72 ms the handoff measured, and the
handoff's unmet 4 ms amortised target is unchanged and still unmet.

**`ascii`'s river-cost scene is the one standing frame cost worth flagging.**
It builds a spring, a fall and a pool at the shipped size and holds them at
steady state: the standing bill went from 1.37 ms/frame at 2048x640 to
**5.43 ms/frame**, past both the ~3.5 ms wind-revert class and the
pre-registered 2.0 ms bar the harness prints. Not investigated here -- it is
a world-size cost in the field and liquid layers, not a worldgen one -- but
it is the number a session working on frame cost should start from, and the
scene now runs at the shipped size automatically because it takes
`app::WORLD_WIDTH`.

**The load and fracture model is untouched, and that is measured rather
than asserted.** `scripts/seedsweep.sh` -- the order-statistic sweep over
`worldcrack` at 512x320, six presets x four seeds -- came back
**bit-identical** across the Phase 2 worldgen work: cells lost max 131, p90
85, median 28, min 0; rock destroyed max 165, p90 5. Run before *and* after,
in the same session on the same machine, against a worktree build of the
previous commit, because a remembered number is the comparison this repo
keeps getting wrong.

**It moved on the review round, and the reason is the world, not the
model.** After `residual_density` went to 0.45 (§7a): cells lost max 131
(unchanged), p90 85 -> 94, median 28 -> 20 -- noise either side -- and rock
destroyed **max 165 -> 4, p90 5 -> 0**. That last one is large and it is
expected: `worldcrack` digs into *generated terrain*, and a world with a
third of the standing tors has far less loose rock for a dig to bring down.
The sweep exists to catch a change that eats **more** world; this eats less,
for a reason that is upstream of the load model entirely. Worth stating
rather than waving through, because a 40x move in a guard number is exactly
the shape of thing that should stop a session.

**The 648 ms frame is a load transient, not a standing cost**, and it is
already visible in `scale_probe`'s own worst-frame columns (sweep 336 ms +
field 307 ms during settling, against 0.17 / 51.77 once quiet). The loading
screen covers generation but not the settle behind it.

## 7. Findings a future session should not have to re-derive

- **The world is not wetter, and it took a census to know.** The real app,
  screenshotted headlessly at 8192x2560 (`CLAUDE.md`'s recipe; it works, and
  the HUD, rain and lighting all come up correctly at the new size), showed a
  sheet of standing water wider than any 2048x640 render had. Ponds are not
  something Phase 2 touched, so either the world had got wetter or a wider
  world simply has wider hollows. Only a census separates those, and nothing
  measured it — `probe_p2_how_much_of_the_world_is_water` does now:

  | preset | 512x320 | 2048x640 | 8192x2560 |
  |---|---|---|---|
  | `rolling` | 2.08% | 0.27% | **0.30%** |
  | `terraced` | 0.38% | 0.11% | **0.06%** |
  | `arid` | 0.00% | 0.00% | **0.00%** |
  | `flat` | 0.00% | 0.00% | **0.00%** |

  (water as a percentage of the cells that are not empty, seed 1). The
  fraction is flat from 2048x640 to 8192x2560 and `terraced` actually dries
  out. What is on screen is one wide lake in one wide hollow, which is what a
  lake looks like when the relief is unchanged and the world is four times
  wider. `arid` and `flat` at exactly zero is the `table_offset` sentinel fix
  in §2 doing its job — the handful of cells the probe attributes to `arid`
  at the larger sizes are the aquifer waterline *inside cave chambers*, which
  is deliberate.
- **`vault_density` is the only density in the generator whose denominator is
  the whole world.** `pockets` is per 64x64, `residuals` per 256 columns,
  regions per 512-column window — all three scale for free. Vaults do not, and
  that is *correct* under a 4x-zoom reading (same count, each 16x the area,
  void fraction preserved — measured, §4) but it is correct by luck rather
  than by design, and it is the first thing that breaks if cave size and world
  size ever stop moving together.
- **Cave depth is now drawn uniformly over a ~2200-row band** rather than a
  ~280-row one, because `vault_min_depth` is a fixed 200 below the surface and
  the massif got five times deeper. Most systems are now far deeper than any
  round-7 system was. Nothing is broken; the *distribution* is new and nobody
  chose it.
- **Generation is *not* super-linear, and the metric that said it was is
  counting the wrong thing.** `examples/ascii.rs` reported "202.94x the
  512x320 build for 128.0x the area -- WORSE THAN LINEARLY", which is true
  and means nothing: `sky_rows` does not scale with world height, so a taller
  world is a proportionally *more solid* one. 59% of cells filled at 512x320
  against 94% at the shipped size, so solid cells went up **206x** for 128x
  of area. Against the cells it actually writes the build is linear or
  slightly better -- `PASS_TIMING=1` puts `stone_massif` at 3946 of 5188 ms
  and 201 ns per placed cell, against ~300 ns/cell at 512x320. The message
  now ratios against solid cells and prints both. This cost a wrong
  hypothesis (that `RegionMap::sample`'s O(regions) linear scan was the
  super-linear term) and a change that was implemented, verified
  bit-identical by world hash, measured at **zero** (5031 -> 5082 ms) and
  reverted; it is in `Reports/dead-ends.md` with the condition to re-test it.
  `CLAUDE.md`'s "ask what a metric counts when nothing is wrong", one more
  time.
- **Erosion incises about twice as deep in a 4x-wide world with no constant
  touched.** `RAIN_SUPPLY` is per column and flow accumulates downhill across
  the whole world with no reset, while carve is proportional to `sqrt(flow)`.
  This is the one place growth changed the picture without anyone asking for
  it. Not measured against a paired baseline here.
- **The margins are expressions now.** Every one of the four local passes'
  margins has been silently wrong at least once (`talus` declared 3 while
  walking 120; `vaults` declared 96 after A2 took the envelope to 202;
  `residuals`' 80 was justified by an aspect floor that had since been
  withdrawn). They read `passes::BROWS_MARGIN`, `passes::TALUS_MARGIN`,
  `passes::VAULTS_MARGIN` and `residual::RESIDUALS_MARGIN`, and
  `every_local_pass_declares_the_margin_it_reaches` generalises the cave-only
  guard to all four.
- **`Pheromones::new` eagerly allocates ~84 MiB** at the new size (2 channels
  x 2 buffers x width x height), whether or not a creature ever exists. Inside
  the measured 359 MiB, so not urgent; it is a quarter of it.
- **Rain is a quarter as dense as it was**, and it is a documented trade
  rather than a bug: `weather.rs` caps the storm at `MAX_COLUMNS_PER_FRAME =
  24` columns per frame normalised against a `REFERENCE_WIDTH` of 2048, and
  its own comment prices the trade ("rain that thins out on [larger worlds]
  ... frame cost is a hard constraint and rain density is a feel"). The
  trade was priced when a larger world was hypothetical; it now ships, and
  what the player sees on their 512-column screen is four times less rain.
  Left alone here because weather is not Phase 2 and undoing it costs frame
  time that wants its own measurement — but it is a visible change nobody
  asked for and it should be decided, not inherited.

## 7a. The review round, and what it cost to read a picture wrong

Both Phase 2 cards came back. The cave sizing got *"Looks better. I need to
start playtest to really know"* -- provisional, not settled -- and the strata
A/B went **against this session's own prediction**, which §5 records.

The third thing in that verdict is the one worth writing down as method.
Asked what was wrong with the render, the owner answered: *"The biggest #1
issue here though, is the repeating hard boundary at 1/3 and 2/3 of the
image. The patterns don't flow and there are sharp vertical faces."*

**It was the harness.** `viewshot shots=3` spreads its cameras across the
whole world and butts the tiles together with no separator, so that card was
three places -- world x 1109-1620, 3840-4351, 6570-7081, with 2,219 unshown
columns between each -- presented as one landscape. Measured on that image,
the two joins are the largest and second-largest of all 1,535 adjacent column
pairs (88.8x and 61.7x the median), and the skyline steps **61 rows in one
column**. A null test settles what that means: 400 random *cross-tile* column
pairs -- genuinely unrelated places -- have a median difference of 15.6, and
the joins sit inside that distribution, because that is exactly what they
are.

For scale, `probe_p2_how_sheer_is_the_ground` then measured the real thing:
the largest adjacent step worldgen produces, in any preset, over all 8192
columns, is **5 rows**. Not one column in any preset steps six. The seam was
twelve times larger than anything the generator can make.

So the owner spent a review round reporting a defect in the picture as the
world's worst problem, and would have been right to. `strip=1` and a default
gutter exist so it cannot recur, and the summary line now names the columns
a sheet is *not* showing. **The lesson is not "check the harness" -- it is
that a rendering convention which is obvious to the person who wrote the
renderer is invisible to the person judging the render.** A contact sheet
looks like a photograph.

Two real findings came out of chasing it, neither of which anyone was
looking for:

- **Erosion has already removed every plumb face from the ground.** Pre-
  erosion terrace risers reach 19.9 rows in a single column; `THERMAL_STABLE_
  SOFT + hard * THERMAL_STABLE_HARD_BONUS` caps adjacent pairs at 5.05
  rows/column and every shipped preset carries `world_age > 0`, so none of
  them survive to the screen. `column.rs`'s long justification for the riser-
  roughening term reads as though it still fires. It does not.
- **The palette-family thresholds are gated on x only.** A rock-colour
  boundary can therefore sweep 0 to 1 in **11 columns** and run coherently
  from the skyline to the bottom of the frame -- a genuine near-vertical
  seam, and one lands mid-tile in the picture the owner judged. It may be a
  second, real referent for *"the patterns don't flow"*, and it is untested
  and unasked.

## 8. What Phase 3 inherits

The handoff's Phase 3 is unchanged and this does not narrow it: two cave-shape
candidates, built and compared on a blind A/B at play zoom. What it now has
that it did not is a **baseline at the size the shape work will actually run
at** — the table in §4, **read through §4a's correction** — and a specific
target: the honeycomb's fragmentation (`walk_regions` p90 **29**, max 92) and
its chamber-to-passage contrast falling by a **quarter**, which are the two
numbers that got worse and the two a better shape rule should move.

**A caution about both of them, added 2026-08-23.** They are proxies, and the
owner's own specification for Phase 3 is not in them. He said: *"you could go
bigger or more even better longer or have chains of caves for the bigger. The
stalagmite and stalactites look way better. The voroni patter is too much. I
like a little of it, but there is too much."* Nothing in that sentence is
`walk_regions` or a contrast ratio, and a render answers it where a census
cannot: the chamber is a rasterised ellipse (`grow_monumental_chamber`) and
the passages are Worley perpendicular bisectors — straight segments meeting
at 120° junctions. Move these two numbers if a better rule moves them, but do
not aim at them.

**And check `MAX_CEILING_SPAN` before blaming the honeycomb for the
fragmentation.** `settle_cave_void` drops a 3-row stone tooth into the middle
of every roof run longer than 36 and repeats until all comply. The player is
7x14 with crouch unimplemented, so a tooth blocks him in any passage under
17 rows tall while leaving the void *connected* — which reads as one system
to the census and as several caves to the player. That predicts both the
`walk_regions` count and the divergence between `reachable` and
`largest_walkable`, and it is cheap to test.
