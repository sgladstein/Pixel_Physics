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

Below the top ~5% the world is the same banded stone with scattered ore for
2,400 rows. `sky_rows` is 80-110 across the presets and `relief_amplitude`
24-70, so the surface sits around row 95-180 out of 2560: **93% of the
world's height is underground, and its character does not change with
depth.**

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

1. **The terrain** (`assets/worldgen.ron`'s 6 presets plus
   `params.rs`'s 46 fields). This is where the owner's original complaint
   lives — *"you cannot create good looking crystals or stalagmites and
   stalactites that are only 1-2 pixels wide"* — so it is where the visible
   payoff is, and it is data rather than code.
2. **World dimensions and `FIELD_SCALE`.** `FIELD_SCALE` 8 -> 16 keeps a
   field block covering the same *physical* area it does today, so light and
   shade look identical and the field's cost falls ~4x — which would close
   the ≤4 ms amortised target `world-scale-handoff.md` records as the one it
   missed. Do not do this without the content scaling: at unchanged content
   it coarsens the shade.
3. **The gnome**, because he is the ruler everything else is judged against.
4. **Plants and creatures.** *This one is not a rescale.*
   `Reports/why-changes-cost-so-much-2026-08-27.md` is about exactly this
   failure: doubling internode lengths changes what the growth economy's
   constants mean, and the economy was calibrated against the current cell
   counts. Budget re-deriving them as part of the work, or the change is not
   scoped, only started.
5. **`app.rs`'s `WIDTH`/`HEIGHT` to 1024x640**, and `main.rs`'s
   `with_inner_size(WIDTH * 2, HEIGHT * 2)` to `(WIDTH, HEIGHT)` so the
   window stays the size it is. **Last, not first**: on its own this
   produces the right-hand pane the owner rejected.

**Do not start at (1) without re-listing the branches.** As of 2026-08-29
`claude/worldgen-sky-soil-mw9jhb` is live in exactly those files and had
cards answered that night — the owner told it to *"increase the soil another
2-3x"*. `CLAUDE.md`'s file-ownership rule applies with its full force here.

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
