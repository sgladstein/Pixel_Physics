# Doubling the resolution: what it costs, and what is left

*Written to be picked up cold. Branch: `claude/world-size-resolution-perf-uftzla`.*

`Reports/world-scale-handoff.md` records the owner's decision as *"bigger
world at the current resolution first (higher resolution later)"*. This is
the first half of **later**, and it answers the question that was asked:

> can we shrink the size of the world (mostly stone depth but you can shrink
> in both directions) and double the resolution without losing performance

**On performance, yes, and it is done.** A doubled viewport now costs less
than the single one did before this branch. What is *not* done, and is much
the larger half, is the content: the owner's verdict says the picture must
keep its present apparent scale, which means every feature has to be made of
four times as many cells. That is 261 places in the source, and it is not a
session's work.

## The one number that decides it

The render was the whole constraint, and nothing was watching it.
`examples/scale_probe.rs phases=1` times `App::update` and `Renderer::draw`
is not in it, so the frame budget everyone quotes had a hole in it the size
of the largest cost in the frame. Measured on a 4-core box at the shipped
8192x2560 world:

| | |
|---|---|
| whole of `App::update`, mean | 6.28 ms |
| one forced full redraw, 512x320 | **21.2 ms** |

The render was three times the simulation. And it is not a worst case: a
camera move invalidates every pixel, so a full redraw runs on ~100% of
frames while the gnome is walking (`Renderer::draw`'s `camera_moved`).

## What was done

`parallel.rs` has had the CA sweep on `rayon` since M5. `Renderer::draw`'s
two pixel loops never followed, and `cell_colour` was re-fetching the same
`Material` six times per pixel and hashing a `ChunkCoord` once per pixel for
a coordinate that changes chunk every 64th of them.

Three changes, all in `src/render.rs`, all bit-identical by construction:

1. **Both pixel loops parallel over scanlines.** A row is `4 * width`
   contiguous bytes, so a worker writes whole cache lines and consecutive
   `cell_colour` calls walk one chunk row.
2. **`ChunkRun`** holds the chunk a run of pixels shares, misses included.
3. **One `Materials::get` per pixel**, not six.

Paired and alternating, four runs of each of two fixed binaries in one
session (the protocol `CLAUDE.md` asks for, and the reason it does is
below):

| viewport | serial | parallel | + hoist |
|---|---|---|---|
| 512x320, generated world | 21.2 ms | 12.6 ms | **12.1 ms** |
| 1024x640, generated world | 52.8 ms | 21.5 ms | **18.6 ms** |
| 512x320, all-stone control | 9.33 ms | 3.19 ms | 2.15 ms |
| 1024x640, all-stone control | 34.4 ms | 8.69 ms | 5.09 ms |

**The headline: a 1024x640 redraw costs 18.6 ms against the old 512x320's
21.2 ms.** Doubling the framebuffer is now free and then some.

### Three ways the measurement lied on the way here

Each cost a wrong conclusion before the control caught it, and each is a
rule this repo already has.

- **The viewport scaling read 2.41x at 4x the pixels**, which looks like
  useful sublinearity and is not. The camera is clamped at the world origin,
  so a taller viewport shows *further down*, and everything it adds is
  underground stone — `cell_colour`'s cheapest branch, 48.6 ns/px against
  sky's 69.9. The uniform-world controls now sit beside it and read 3.68x,
  which is the honest number. *Ask what your number counts when nothing is
  wrong.*
- **The hoist looked like a 13% regression at 512x320.** It was measured
  against a single earlier sample that had landed low, on a box that drifts
  upward within a run (the same arm reads 11.04 then 13.85 four runs later).
  Alternating two fixed binaries puts the hoist ahead in 7 of 8 paired runs.
  *Compare two runs, not one run against a remembered number.*
- **A guard reported `ok. 0 passed`** while verifying that it could fail.
  `cargo test --lib "a|b"` is a substring filter, not a regex, so it matched
  nothing and printed a line that reads exactly like a pass. *A green suite
  does not prove a test ran.*

### The guard

`a_parallel_redraw_is_bit_identical_to_a_serial_one` has two arms because
they fail for different faults: a one-thread `rayon` pool against the
default pool catches the parallelism, and a serial reference over
`cell_colour` catches a wrong row index, which both arms of the first
comparison would get wrong identically. Verified sensitive by putting three
faults back — an off-by-one row index, a row index taken from an atomic
counter, and a `ChunkRun` that never invalidates. The third is also caught
by `dirty_rect_skip_is_pixel_identical_to_a_full_redraw`.

## The owner's verdict: which reading of "resolution"

Card `20260829T030025652Z-b52864`, board `world-scale`. Two panes, the same
window on the same spot of the same world: today's 512x320 upscaled 2x, and
the framebuffer doubled to 1024x640 with the content left alone. Asked
whether the doubled pane was wanted *or whether everything should stay the
size it is on the left*, the owner **chose the left pane**.

So the extra pixels are **not** to show more world. The gnome must stay the
size he is; the world must stay the size it is; the resolution has to come
from **more cells per feature**. That is the reading that deserves the name,
and it is the expensive one — not in frame time, which is now paid for, but
in content.

## The owner's second verdict: the rescale is the right direction

Card `20260829T080824307Z-6f335b`, board `world-scale`, answered 2026-08-29.
The same coastline at both cell sizes, gnome in each for scale. The verdict
was **"Four times the cells"**, with two things named:

> *"4x looks better, but our gnome shouldn't have shrunk. Also, are all my
> plants going to be 1/4 the size (or grow 1/4 the speed visually)."*

Both are the same defect as the tor slabs and both are correct. **The card's
own `meta` claimed the gnome drew at 28 window pixels in both panes and it
did not** — `player.rs`'s `PLAYER_WIDTH`/`PLAYER_HEIGHT` are a hardcoded 7x14
cells, so at half the cell size he came out 7x14 window pixels against 14x28.
He halved. The number on the card was wrong and the owner caught it by eye,
which is the argument for posting pictures rather than tables.

The plants answer is *both* of the guesses at once, and worse than either: a
tree's height is set in cells, so at half the cell size it grows to half the
physical height **and** climbs the screen at half the speed, finishing at a
quarter of the on-screen area.

**The gnome has two separable fixes** and they should not be confused. His
size is a bug: nearest-neighbour doubling `GNOME_SPRITE` and `GNOME_SWING` to
14x28 makes him the right size with no art and no judgement call. A *real*
14x28 sprite — four times the pixels, so a face and hands become possible —
is an opportunity, is judge-by-eye, and is a separate card.

## What that costs, and why the frame budget is not the problem

To hold the same physical world at twice the cell density is four times the
cells: 16384x5120, 84M, against today's 21M. That does not fit, which is
where the shrink in the original question comes in — and the shrink was
measured:

| world | cells | generation | peak RSS | field, settled |
|---|---|---|---|---|
| 8192x2560 (today) | 21.0M | 4824 ms | 361 MiB | 8.48 ms |
| 8192x1536 | 12.6M | 2770 ms | 218 MiB | **8.39 ms** |
| 6144x1536 | 9.4M | 2042 ms | 165 MiB | 6.28 ms |

**Cutting stone depth buys load time and memory, not frame rate.** Removing
1,024 rows — 40% of the world's cells — halves generation and memory and
moves the settled frame cost by 1%. That rock was never in the field's
working set: it is dark, blocked and quiet, so no pass walks it. Only
*narrowing* the world moves the frame, because the sky-lit band is
proportional to width.

That is worth knowing before planning around it, because the question
proposed the depth cut as the thing that pays for resolution. It is not. The
render was. The depth cut is still worth doing on its own merits, and the
picture says so plainly:

```
cargo run --release --example viewshot -- shots=1 aim=4096 view=256x640 stride=4 zoom=2
```

The world is the same banded stone with scattered ore for over two thousand
rows straight down. `sky_rows` is **160-220** across the presets and
`relief_amplitude` 18-70, so the surface sits around row 160-290 of 2560,
with 48-135 rows of soil under it: **at least 83% of the world's height is
stone, and its character does not change with depth.**

*(Those figures are post-#94, which roughly doubled the sky and deepened the
soil blanket ~4x while this branch was in flight. The rendered strip above
was made before it landed, so it shows the thinner sky and soil of that
morning; the thing it is evidence for -- that the deep rock is uniform --
is unaffected, and #94 only moved the boundary further down.)*

## What is left, and in what order

The content scaling. **261 places in the source reason in cells, across 29
files**, and they are in every subsystem, not just worldgen:

| | |
|---|---|
| `structural.rs` 28, `load.rs` 15, `rigid.rs` 15 | spans, reaches, fragment sizes |
| `weather.rs` 25, `explosion.rs` 20, `fire.rs` 7 | radii and spread distances |
| `worldgen/passes.rs` 21, `column.rs` 6, `erosion.rs` 6, `residual.rs` 4 | the terrain itself |
| `plant.rs` 19, `organism.rs` 10, `creature.rs` 7 | internode lengths, body sizes |
| `player.rs` 17 | the gnome, 7x14, and his sprite |

Suggested order, and the reason for it:

1. **The terrain — DONE, and it was two changes rather than 46.** See the
   section below; `WorldgenParams::scaled(k)` plus one line in `region.rs`
   makes the generated surface scale-covariant, measured. What it does *not*
   reach is the features sized by source constants, which is item 1b.
1b. **The features `scaled` cannot reach** — `residual.rs`'s stack and tor
   widths, the cave and speleothem widths, `LENS_LOBE`. These are lengths in
   cells living in the source, so a rescaled world draws them at half their
   proper width and they read as slivers. **This is the owner's round-6
   complaint arriving from the other direction** — *"you cannot create good
   looking crystals or stalagmites and stalactites that are only 1-2 pixels
   wide"* — and it is visible in the rendered pair below as two thin grey
   slabs where there should be tors. `cell_scale` is on the params precisely
   so these can read it; each site needs a decision, not a rewrite.

2. **World dimensions and `FIELD_SCALE`.** `FIELD_SCALE` 8 -> 16 keeps a
   field block covering the same *physical* area it does today, so light and
   shade look identical and the field's cost falls ~4x — which would close
   the ≤4 ms amortised target `world-scale-handoff.md` records as the one it
   missed. Do not do this without the content scaling: at unchanged content
   it coarsens the shade.
3. **The gnome — DONE**, and doing him first was right for a reason that
   was not the stated one. He was picked because he is the ruler everything
   else is judged against; what he actually settled is the *mechanism*. See
   the section below.
4. **Plants and creatures.** **The owner has settled the hard half of this:
   *"growth rate can be slower that is fine"* (2026-08-29).** That is a real
   simplification — a tree twice as tall in cells, built at the same cells
   per tick, takes twice as long to get there, and rate parity was the part
   that would have forced the growth economy to be re-derived
   (`Reports/why-changes-cost-so-much-2026-08-27.md`). It is not needed.

   **What that ruling does *not* cover, and what still has to be checked:**
   slower is not the same as never. The economy is a budget, and a plant
   that cannot pay for twice the tissue does not grow slowly — it dies before
   seeding. This exact failure is already on the record: reshaping
   `phototropism_dir` gave trees a direction they had never had, they spread
   instead of climbing, never reached `seed_maturity`, and **reproduction
   went to zero** with every gate green but one. So the acceptance criterion
   here is not a growth rate, it is *does a stand still reach maturity and
   still reproduce* — which `examples/plant_probe.rs` reports directly, and
   which is a bounded measurement rather than an open re-derivation.
5. **`app.rs`'s `WIDTH`/`HEIGHT` to 1024x640**, and `main.rs`'s
   `with_inner_size(WIDTH * 2, HEIGHT * 2)` to `(WIDTH, HEIGHT)` so the
   window stays the size it is. **Last, not first**: on its own this
   produces the right-hand pane the owner rejected.

**Re-list the branches before starting at (1).** The lane that held those
files while this branch was in flight — `claude/worldgen-sky-soil-mw9jhb` —
**landed as #94** on 2026-08-29, doubling `sky_rows` and deepening
`soil_depth` ~4x on the owner's review verdict. So the file is free as of
that merge, and its numbers are *newly tuned by eye*: a 2x rescale on top of
them has to be judged against what the owner just approved, not against what
this report first measured. `CLAUDE.md`'s file-ownership rule still applies
— that lane's landing is exactly why it does.

## The terrain half, done: two changes, not forty-six

**Worldgen is scale-covariant once one constant stops being hardcoded**, and
that was worth finding out before anyone hand-edited 46 parameters.

`WorldgenParams::scaled(k)` reinterprets the whole struct at `k` times the
cell resolution. The classification is the work; the arithmetic is trivial.
**The 46 fields carry four dimensions, not one:**

| dimension | factor | example |
|---|---|---|
| a length or wavelength in cells | `k` | `sky_rows`, `hill_wavelength`, `soil_depth` |
| dimensionless — ratio, probability, slope | `1` | `strata_tilt` is rise over run; both terms scale |
| a per-column probability | `1/k` | `tree_density`: `k` times the columns cross the same ground |
| a count per fixed *cell* region | `1/k` or `1/k²` | see below |

**The last row is the trap and it is invisible from the field.**
`pocket_density` is drawn once per 64x64 cell region in a *2-D* loop, so it
takes `1/k²`; `residual_density` is drawn per 256-*column* region in a 1-D
loop, so it takes `1/k`. Two fields whose names, types and doc comments all
read the same way, needing different factors — which is why the function is
an exhaustive destructure with no `..`: adding a 47th field stops compiling
until somebody classifies it.

**And one hardcoded constant was the whole difference between "the same
world, finer" and "a different world."** `region::COMPOSITION_WINDOW` is 512
cells because that is *"roughly one screen at 1:1"* — its own comment says
composition is a property of what fits in view. A screen is the right unit;
512 is only its value at today's resolution. Left fixed, a world with twice
the cells gets **twice as many regions** rather than the same regions twice
as wide.

Measured with `examples/scale_covariance.rs`, mean absolute difference in
rows between the small world's elevation profile and the big one's rescaled
back down — **each against a control of the same preset at a different
seed**, because a small residual means nothing on its own:

| preset | window fixed at 512 | window scaled | unrelated seed |
|---|---|---|---|
| rolling | 39.13 | **1.27** | 42.49 |
| terraced | 34.55 | **1.29** | 34.47 |
| canyon | 58.32 | **1.10** | 61.61 |
| wetland | 19.77 | **0.20** | 20.32 |
| arid | 16.01 | **1.94** | 16.96 |

Read the first and last columns together: with the window fixed, **a
rescaled world was no more like the original than a stranger was**. That is
the finding, and no bare threshold would have shown it — which is why
`a_rescaled_world_is_the_same_world_at_a_finer_grain` carries the
different-seed control *inside* the assertion and asserts a ratio rather
than a bar. Verified sensitive by putting the fixed 512 back.

`worldgen` sits below `app` in the crate layering and cannot read
`app::WIDTH`, so the factor arrives as data: `WorldgenParams::cell_scale`,
which `scaled` multiplies and which any source-side length in cells can now
consult. That is the mechanism item 1b needs.

The residual does not reach zero and should not: column `x` maps to column
`round(kx)`, and `column::strata_offset` folds its bands on a hardcoded
130-cell wavelength `scaled` cannot reach. 1-2 rows against 17-62 is the
floor, not a defect.


## The gnome, done: and a tuning struct is the unit of this work

**A subsystem's cell-scale problem is not a list of sites. It is one
classified `scaled()` per tuning struct, plus a handful of true one-offs.**
That is the useful shape, and the gnome is where it became visible.

`World::cell_scale` now carries the factor, set once at generation from
`WorldgenParams::cell_scale`. It is on the world rather than on the worldgen
params because most of what needs it is not worldgen — the gnome's body, a
blast radius, an internode — and those files hold a `&World` and have no
reason to know what a `WorldgenParams` is.

**His body was the half that was expected.** `Player` carries its own `w`/`h`
and the twenty-odd rules that read `PLAYER_WIDTH`/`PLAYER_HEIGHT` go through
it. The sprite walks the cells he occupies and samples the 7x14 table
nearest-neighbour, so he is the right size at any scale with no new art.

**His motion was the half that was not.** `Tuning` holds `gravity`,
`run_max`, `jump_impulse`, `step_up`, `dig_radius`, `wade_rows` — every one a
length or a speed in cells. Scaled in body alone he would have been the right
size moving at half the physical speed and jumping half as high: right in a
screenshot, wrong in the hand, which is the failure this project's ethos
section is about. `Tuning::scaled(k)` sorts its 27 fields into **the same
four classes** as `WorldgenParams::scaled` sorts its 46.

That repetition is the finding. The "261 cell-valued sites" figure counted
*mentions*; the work is structured far better than that number suggests, and
the next subsystems should be approached by finding their tuning struct
rather than by grepping for lengths.

**The arithmetic is checkable, which is what makes the classification more
than taste.** Jump height is `v²/2g`; scaling `jump_impulse` and `gravity`
both by `k` gives `(kv)²/(2kg) = k·v²/2g` — `k` times the cells, the *same
physical height*, reached in `v/g = kv/kg`, the same number of ticks. The
trap of the set is `buoyancy`: it reads like an acceleration and its own doc
calls it one, but it is stated *as a multiple of `gravity`*, so `gravity`
scaling already carries it and scaling it again double-counts. Same shape as
`strata_tilt` in the worldgen struct.

### Two guards were blind before one bit

Worth recording in full, because both blindnesses are ones this repo will
meet again.

- **A drop onto flat stone passed** with `rect_free`'s wade line, the grip
  rows and `Bodies::near`'s window each put back to the unscaled constant. A
  plain fall never asks about any of them.
- **A convergence assertion between 2x and 4x passed** with `step_up`
  unscaled — because the ledge is 6 cells at 2x and 12 at 4x against an
  unchanged 4-cell step, so he is stuck at *both* and the two fine runs agree
  beautifully with each other while diverging from 1x. **A quantity that does
  not scale at all can make the fine arms agree**, which is the specific way
  a convergence test lies.

What ships makes him travel — fall, run, step a ledge, wade two drifts — and
asserts a coarse-vs-fine bound *and* convergence. Twelve faults injected one
at a time, it catches nine; the three it misses (grip rows, the body window,
`wade_rows`) are **missing scene rather than weak assertions**, and the test
says which scene element each would need.

The 1x-2x divergence is 2.80 cells of ground over a 232-unit run and 2x-4x is
0.25. The first number read as a scaling bug until the third resolution was
measured: the disagreement collapses as the grid refines, which is
convergence to a continuum limit and not a quantity left behind.

### The suite caught a real bug, through somebody else's control

`Tuning::scaled`'s `cells()` helper floored counts at 1 — so
`shoulder_grains: 0` became 1 at every scale **including 1.0**. In that struct
a zero means *off*: it is the old hard veto, a different rule rather than a
smaller number. A world nobody had rescaled quietly started letting the gnome
shoulder past a grain that should have stopped him.

`a_stray_grain_at_chest_height_is_not_a_wall` failed — and specifically on
its *control* arm, the one an earlier author added so the test could not pass
for the wrong reason. Nothing else in 1,011 tests sets a knob here to zero.
**`scaled(1.0)` must be the identity**, asserted for `WorldgenParams` and not,
until now, for `Tuning`.


## Instruments

- **`render_cost` gained `viewport_scaling`** — one world drawn at 512x320,
  768x480 and 1024x640, with two uniform-world controls beside it. The
  controls are the number; see above for why.
- **`viewshot` gained `view=WxH`** — the viewport to render through, in
  cells, defaulting to `app::WIDTH`x`app::HEIGHT`. Those are compile-time
  constants, so without this a resolution comparison is two builds and
  cannot be paired. It echoes the viewport it used. `gnome=1` is what makes
  a pair readable: he is 7x14 cells whichever viewport draws him, so he is
  the ruler that says whether a bigger viewport is showing more world or the
  same world larger. `view=256x640 stride=4` renders the world's whole
  2,560-row depth in one frame, which is the picture in the section above.
  It also gained **`world=WxH`** and **`cellscale=K`**, which is what renders
  the resolution pair: the shipped world at `cellscale=2` is 84M cells and
  will not generate twice in a sitting, so the comparison is made on a
  smaller world at both scales.
- **`scale_covariance`** — *is the same seed at `k` times the resolution the
  same landscape?* Reports the rescaled elevation residual **beside an
  unrelated-seed control and a `region_variation=0` arm**, which is what
  turned "the residual is 39 rows" from a number into a diagnosis: the flat
  arm read 1.18 and named the regions as the cause in one run.
