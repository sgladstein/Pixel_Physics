# The other half of the frame: `Renderer::draw`, and what a taller sky costs it

**Status: measurement of record for the *whole* frame — simulation plus
render. Taken 2026-08-29 on `main` @ `1c375f7`, 4 logical cores, a
deliberately quiet box (no other lane running, load average 0.06–0.11 between
runs).**

Extends `Reports/frame-cost-audit-2026-08.md` rather than superseding it:
that report is still the record for how `App::update` divides, and its method
is reused here unchanged. What it could not see is that `App::update` is
**not the frame**.

## Why this exists

The owner reported the game feeling slow the morning after 2026-08-28. The
standing suspect was `39e6f36` (PR #94, *"worldgen: double the sky, deepen
the soil blanket ~4x"*), merged at 22:49 the night before, on the reasoning
that more sky and deeper soil mean more tiles for the per-tile field solve to
walk.

**The reasoning was right about the commit and wrong about the mechanism, and
the mechanism is the part that decides what to do about it.**

## 1. The simulation is not the problem — it got 28% faster

`scale_probe size=8192x2560 phases=1 warm=1500 frames=7200`, preset
`rolling`, seed 1: the audit's protocol exactly, so the numbers compare.

| phase | mean | p90 | share |
|---|---|---|---|
| field | 11.22 ms | 20.2 ms | **59.4%** |
| active sites: organisms | 5.05 ms | 12.6 ms | **26.7%** |
| sweep (`parallel::step`) | 1.39 ms | 2.10 ms | 7.3% |
| active sites: scheduler | 1.20 ms | 3.71 ms | 6.4% |
| the other seven together | <0.03 ms | | 0.1% |
| **`App::update`** | **18.88 ms** | **31.2 ms** | 100% |

**55.8% of frames exceed the 16.6 ms budget.** Against the audit:

| | audit, before its fixes | audit, after | **today** |
|---|---|---|---|
| `App::update` amortised | 30.10 ms | 26.16 ms | **18.88 ms** |
| frames over budget | 79.0% | 70.9% | **55.8%** |
| field | 20.74 ms | 15.22 ms | **11.22 ms** |

**The creature lane's reading is confirmed in direction and overstated in
size** — it put the field at 63–74% of the frame, taken at load average
15–19 with three agents compiling; on a quiet box it is 59.4%.

### The noise bar for this job, measured on this job

Three repeats of one binary (md5 `df7f7365…`), back to back:

| | run 1 | run 2 | run 3 | spread |
|---|---|---|---|---|
| `App::update` mean | 18.986 | 18.839 | 18.817 | **0.9%** |
| field mean | 11.317 | 11.179 | 11.158 | 1.4% |
| organisms mean | 5.042 | 5.058 | 5.050 | 0.3% |
| frames over budget | 55.5% | 55.8% | 56.0% | 0.5 pp |
| **worst frame** | **493.76** | **61.85** | **90.39** | **8.0x** |

**±1% on a mean, applied to both signs.** And the worst frame is worth
nothing: `mean x frames ~ worst` gives 11.317 x 7,200 = 81.5 s of field time
against a 488 ms worst — a ratio of 0.006, so no aggregate pins it. It is an
order statistic over 7,200 comparable frames and it duly moved 8x across
three runs of one binary doing bit-identical work. The counters did not move
at all: `live organisms: 1363   chunks: 5120   awake chunks: 29`, identical
in all three.

## 2. The hole: nothing has ever counted the renderer

`scale_probe phases=1` times `App::update`. **`Renderer::draw` is not in
`App::update`**, so every whole-frame figure this repo has published —
including the table above and the audit's headline — excludes it.
`Reports/resolution-step-2026-08-29.md` found the same gap the same morning
and put a 512x320 full redraw at **12.1 ms** after parallelising it.

Measured today on the shipped world, quiet box, `examples/render_cost`:

```
      full redraw (Renderer::draw)   39.7 ms      242 ns/pixel
```

**39.7 ms, against 12.1 ms that morning.** A full redraw runs on ~100% of
frames while the gnome is walking, because a camera move invalidates every
pixel. So the honest frame is roughly **18.9 ms of simulation plus ~40 ms of
render — around 59 ms, or 17 fps** — and the render is the larger part by
2:1.

## 3. The bisect: it is PR #94, it is the sky half, and it lands on the render

`39e6f36` moved the `rolling` preset's `sky_rows` 95 -> 190 and `soil_depth`
26 -> 105. **`datum = sky_rows + relief_amplitude`, so doubling the sky moves
the whole terrain 95 rows down** in a world whose height did not change.

### The instrument is better than a commit bisect, and dodges its hazard

`assets/worldgen.ron` is read at runtime (`std::fs::read_to_string`) and is
**not** `include_str!`ed the way `assets/materials/*.ron` and
`assets/species/*.ron` are. So the arms can be **one binary and four data
files**: no rebuild between points, and therefore no stale-example failure
mode at all — the one that makes a bisect read "no regression" when it is
really reading yesterday's binary.

The variants are line-addressed, not pattern-addressed: `sky_rows: 190.0`
appears in three presets, and `CLAUDE.md` records a blind `sed` on a field
name dragging a second, deliberate value through every point of a sweep.

`render_cost.HEAD` (md5 `4d7d1466…`), three alternating passes:

| arm | sky_rows | soil_depth | sky on screen | full redraw, 3 passes | median |
|---|---|---|---|---|---|
| **now** (shipped) | 190 | 105 | 70% | 38.79, 41.42, 39.68 | **39.68 ms** |
| sky95 | **95** | 105 | 40% | 10.50, 10.69, 12.62 | **10.69 ms** |
| soil26 | 190 | **26** | 70% | 39.71, 39.82, 45.04 | 39.82 ms |
| pre94 | **95** | **26** | 40% | 10.64, 10.65, 10.52 | **10.64 ms** |

- **The sky is the whole of it: ~29 ms a redraw, 3.7x.** Nine of nine paired
  comparisons agree, against a gap far larger than any noise bar here.
- **The soil is nothing**: 39.68 against 39.82 ms. The half of PR #94 that
  sounds most expensive per frame costs nothing per frame.
- **The positive control fires.** Sky share moves 70% -> 40% and the counted
  pixels 114,567 -> 66,222, so the asset edit reached the run. Without that
  check this is one world wearing four labels.
- **A second harness reproduces it independently.** `viewshot seed=24301
  shots=1 settle=600` reports its own redraw: **51.56 ms** against
  **14.53 ms**.

### It is a cliff and a fixed cost, which is why it is a defect and not a price

Sweeping `sky_rows` on one binary, everything else held:

| sky_rows | 95 | 96 | 100 | 105 | 110 | 115 | **120** | 140 | 160 | 175 | 190 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| redraw, ms | 12.1 | 11.0 | 10.7 | 12.0 | 13.9 | 13.2 | **46.8** | 40.8 | 41.3 | 49.7 | 44.2 |
| sky on screen | 40% | 41% | 42% | 44% | 45% | 47% | 48% | 55% | 61% | 65% | 70% |

**The sky fraction climbs smoothly and the cost does not follow it.** It
steps between 115 and 120 and is then flat to 190.

The viewport sweep says the same thing from the other side. Fitting
`cost = fixed + k x pixels` over three viewport sizes:

| world | fixed per draw | per pixel |
|---|---|---|
| `sky_rows` 190 | **~40.9 ms** | ~11 ns |
| `sky_rows` 95 | **~8.4 ms** | ~13 ns |

Same per-pixel price; the whole difference is a **fixed ~29 ms per draw**,
independent of viewport size and of how much sky is actually on screen. A
per-pixel price for sky would be a trade to put to the owner. A fixed cost
that steps at a threshold is a defect, and fixing it keeps both the sky and
the frame.

### What has been ruled out, by measurement rather than by argument

- **Not PR #116's sky-view ray fan**, the obvious suspect. `render_cost`
  built at `e2f6667` and at its parent `e47eda5` — md5 `bb0a4068…` and
  `afbaf40f…`, three alternating passes each — reads a median 39.28 against
  39.44 ms, on a world byte-identical across the arms (the `erosion detail`
  census line matches to the digit).
- **Not `rebuild_sky_light`.** `PIXEL_PHYSICS_SKY_LIGHT_TIMING=1` splits it at
  `build 1.19 / view 0.50 / sweep 0.53 ms` in the 190 world and
  `1.20 / 0.52 / 0.53` in the 95 world — identical, and 2.2 ms of a 40 ms
  draw either way.
- **Not the rain, though the rain is its own finding.** Both worlds are in a
  downpour at this seed and settle count — which is why *looking* at them was
  worth the minute it cost. Holding everything else and picking a dry frame:
  49.31 -> 41.31 ms in the shipped world and 16.81 -> 13.75 ms in the pre-#94
  one. **Rain is worth ~8 ms of a shipped redraw**, and it forces a full
  repaint on every frame it falls, and it is in no budget either. It is not
  the gap: dry against dry is still 41.3 against 13.8.
- **Not the settled state.** The gap is fully present at `settle=1`
  (42.19 against 13.99 ms), so it does not depend on anything the simulation
  does after generation.

## 4. What PR #94 costs the simulation: about 2 ms, not cleanly attributable

The same four worlds through `scale_probe.HEAD phases=1 warm=1500
frames=7200`, paired and alternating, two passes:

| arm | pass 1 | pass 2 | mean | field | live organisms |
|---|---|---|---|---|---|
| **now** (shipped) | 18.664 | 18.509 | **18.59 ms** | 11.2 ms | 1363 |
| sky95 | 15.889 | 15.999 | 15.94 ms | 8.84 ms | 1331 |
| soil26 | 15.082 | 14.835 | 14.96 ms | 9.18 ms | 1237 |
| pre94 | 16.404 | 16.686 | **16.55 ms** | 8.33 ms | 1242 |

Passes reproduce to ~0.25 ms, so the arms genuinely differ, and the shipped
world is the most expensive of the four by ~2.0 ms — about 12%.

**But it does not decompose, and saying it does would be inventing a
result.** Dropping the soil at `sky_rows` 190 *saves* 3.6 ms; dropping it at
`sky_rows` 95 *costs* 0.6 ms. These are four different terrains at one seed,
and terrain drives simulation cost through plant count, water and awake
chunks at once — `live organisms` alone runs 1363 / 1331 / 1237 / 1242 across
them. A clean sky-versus-soil split needs a seed sweep, and `CLAUDE.md` is
explicit that six seeds is not one.

What is safe: **on the simulation side PR #94 is worth about 2 ms of an
18.6 ms frame; on the render side it is worth 29 ms.** The simulation is not
where this went wrong.

## 5. What to do

1. **Put `Renderer::draw` in the frame budget, permanently.** It is the
   larger half of the frame and no report, gate or plan counts it.
   `scale_probe phases=1`'s "WHOLE FRAME" row is a whole `App::update`, which
   is a different thing, and it has been quoted as if it were the frame.
2. **Do not revert PR #94.** The cost is a cliff and a fixed per-draw charge,
   not a price per sky pixel, so the sky and the frame are not actually in
   competition. Find the threshold.
3. **Rain is a second, separate ~8 ms.** Worth its own look.
4. **If the cliff turns out to be inherent after all**, the question for the
   owner is a picture, not a table: what the taller sky buys against ~29 ms
   a frame.

## How to retake any of it

```
cargo build --release --examples          # NOT --release alone; examples go stale
cargo run --release --example scale_probe -- size=8192x2560 phases=1 warm=1500 frames=7200
cargo run --release --example render_cost
cargo run --release --example viewshot -- seed=24301 preset=rolling shots=1 aim=256 settle=1
```

`assets/worldgen.ron` is runtime-loaded, so a worldgen A/B is a file swap on
one binary — edit line 34 (`rolling`'s `sky_rows`) and re-run. Nothing else
compiling at the time; a concurrent `cargo` skews all of this badly.

## Two caveats on the numbers here

- **The harnesses run the baseline clock; the app runs `day_minutes: 8`.**
  `World::new` leaves the clock at baseline (a 3,600-frame day) and
  `assets/clock.ron` ships 8 minutes, so the app's sun steps 8x less often
  per real frame than any harness's. Every simulation figure here is on the
  baseline clock, and since the expensive frames are the ones where the sky
  steps, the app's amortised *field* cost is probably lower than §1 reports.
  The render figures do not depend on it.
- **Two seeds are in play and they are not the same world.** `scale_probe`
  runs `seed=1`; `render_cost` and `viewshot` run `App::new`'s own
  `INITIAL_SEED` (0x5EED). Both show the sky effect, so it is not
  seed-specific, but no number from one table should be arithmetically
  combined with one from the other.
