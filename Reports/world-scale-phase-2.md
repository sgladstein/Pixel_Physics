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

**Residuals, talus and brows follow the surface, not the world.** A residual
stands *up* from the ground into `sky_rows + relief_amplitude` of air — 141
rows for `rolling`, and `MAX_HEIGHT = 120` already nearly fills it. A 4x
residual would be built off the top of the world. Talus is already bounded by
the cliff it sits under (`peak = ...min(fall / 2)`), and a cliff is a
surface feature. Neither can move until the surface does. Residuals are also
the pass Phase 4 exists to delete (`residual_density: 0.0` is its stated
first step), so growing them now would be tuning something on its way out.

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
them. **A blind A/B is queued in the review inbox.**

## 6. Costs, measured on one machine in one session

| | 2048x640 | 8192x2560 |
|---|---|---|
| generation | 459 ms | **9010 ms** (place 4999 + structural 4012) |
| peak RSS | — | **359 MiB** |
| settled worst frame, field | — | **51.77 ms** |
| settled worst frame, sweep | — | **0.17 ms** |
| worst frame during the post-load settle | — | **648 ms** |
| `cargo test` (whole suite) | ~1 min | **4 m 56 s**, 713 pass |

Generation is 19.6x for 16x the area, so it is slightly worse than linear;
the loading screen covers it. RSS matches the handoff's 358 MiB. The settled
field worst frame is **better** than the 72 ms the handoff measured, and the
handoff's unmet 4 ms amortised target is unchanged and still unmet.

**The 648 ms frame is a load transient, not a standing cost**, and it is
already visible in `scale_probe`'s own worst-frame columns (sweep 336 ms +
field 307 ms during settling, against 0.17 / 51.77 once quiet). The loading
screen covers generation but not the settle behind it.

## 7. Findings a future session should not have to re-derive

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

## 8. What Phase 3 inherits

The handoff's Phase 3 is unchanged and this does not narrow it: two cave-shape
candidates, built and compared on a blind A/B at play zoom. What it now has
that it did not is a **baseline at the size the shape work will actually run
at** — the table in §4 — and a specific target: the honeycomb's fragmentation
(`walk_regions` p90 35, max 92) and its chamber-to-passage contrast falling
by a third, which are the two numbers that got worse and the two a better
shape rule should move.
