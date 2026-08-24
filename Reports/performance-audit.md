# Where the frame goes — a measured audit

Written 2026-08-19 against `bb20167`, on the shipped 2048x640 world at the
512x320 viewport. Every number here came from `examples/frame_profile.rs`,
`examples/perf_counters.rs`, `examples/render_cost.rs` and
`examples/weather_duty.rs`, all added by this audit.

## Read this first: two measurement traps caught during this audit

Both are the traps `CLAUDE.md` already warns about, and both produced
confident wrong numbers before being caught.

**1. The machine was not idle.** The first full profile read **45.6 ms mean
on a settled world**. Another session's `creature_space.exe` plus two cargo
builds had all four cores (this machine has *four*, `nproc`). Re-measured
idle, the same scene read 35.5 ms with a completely different shape. Nothing
from a loaded machine is quoted here.

**2. A binary in this worktree was built from another worktree's source.**
`target/release/examples/ascii.exe` in `.claude/worktrees/perf-audit` was
found to contain a banner string (`"quiet reference"`) that exists only in
`.claude/worktrees/perf-lock/examples/ascii.rs`. There is no
`CARGO_TARGET_DIR`, no `.cargo/config.toml` and no junction on this
worktree's `target` — **the mechanism was another session building inside
this worktree**, which that session confirmed once contacted. Which is
exactly the failure `CLAUDE.md`'s "work in your own worktree" rule exists to
prevent, arriving from the opposite direction than expected: not two sessions
in the shared checkout, but a second session inside somebody else's
worktree. **Every
`ascii`-derived number was therefore discarded.** The three probes this
audit relies on are uniquely named, are built and run in the same command,
and contain counters that exist only in this worktree's source (`field::
SOLVED_TILES`, `Renderer::last_full_reason`) — which is what proves they are
this tree's code.

A corollary worth acting on: **`strings` does not exist in this Git Bash**,
so `strings x.exe | grep -c foo` returns 0 for every input. That false
negative nearly closed this investigation with the wrong answer. `grep -c`
straight at the binary works.

## The headline

There are **two independent problems**, and which one you see depends
entirely on whether anything is forcing a full-screen redraw.

| scenario | FRAME | draw | fields | ca_sweep | over budget |
|---|---|---|---|---|---|
| default seed, gnome walking | **22.3 ms** | 12.07 ms (54%) | 9.45 ms (42%) | 0.69 ms (3%) | 300/300 |
| default seed, gnome standing | 22.6 ms | 12.09 ms (53%) | 9.44 ms (42%) | 1.00 ms (4%) | 300/300 |
| default seed, blast every 60f | 30.8 ms | 12.13 ms (39%) | 9.94 ms (32%) | 1.39 ms (4%) | 300/300 |
| **dry seed, nothing forcing full** | **7.2 ms** | **0.07 ms (1%)** | **6.99 ms (97%)** | 0.10 ms (1%) | 2/300 |

The bottom row is the control, and it is the most informative row in the
table. Remove the full-redraw triggers and the draw collapses **170x**, from
12.07 ms to 0.07 ms — the dirty-rect skip works extremely well when it is
allowed to run. What is left is the field, and the field alone is then 97%
of the frame.

**The CA sweep is 0.1–1.4 ms — between 1% and 4% of a frame.** `parallel.rs`,
its checkerboard write-disjointness proof, and the whole chunk decomposition
are guarding a phase that is not the bottleneck in any scenario measured.

## Problem 1: the renderer is forced to repaint the whole screen, and rain is why

`perf_counters` records *why* `Renderer::draw` took its full branch (first
trigger in the `||` chain, via `Renderer::last_full_reason`, added for this).

```
default seed,  600 frames:  precipitating x595, sky changed x4, zoom changed x1
                            => 163,840 px repainted on 100% of frames
default seed, 3000 frames:  precipitating x1561 (52%), dirty-rect path x1111 (37%),
                            sky changed x327 (11%)
dry seed,      600 frames:  (dirty-rect path) x599, zoom changed x1
                            => 1,700 px repainted per frame, 1% of a full frame
```

The 600-frame window sat entirely inside one wet spell; over 3,000 frames the
same seed takes the cheap path 37% of the time. Both numbers are real and the
longer one is the fairer summary of *this* seed — which is wetter than
average (see the cross-seed duty cycle below).

**Rain forces a full repaint on its own**, and while it is falling nothing
else about the draw can even be attributed — which is why the 600-frame
window, sitting inside one wet spell, could not separate the camera from the
weather and the dry-seed control was needed.

Rain is not rare and it is not brief. `weather_duty` over 12 seeds x 200,000
frames:

```
raining 13.8% of the time overall
longest unbroken spell per seed: 2,744 to 33,007 frames  (45 s to 9 min at 60 Hz)
```

So roughly one minute in seven, and for stretches of up to nine minutes
without a break, the renderer pays 12 ms/frame instead of 0.07 ms. That is
the difference between comfortably inside the budget and 35% over it.

`sky changed` is the same shape of problem at smaller scale — a sunrise
repaints every pixel for its duration.

### Why a full redraw costs 12 ms

`render_cost` breaks the 163,840-pixel repaint down. Best-of-20, same world,
same machine, so the rows are directly comparable:

```
      full redraw (Renderer::draw)   11.79 ms   72.0 ns/pixel
   just the cell reads, World::get    1.80 ms   11.0 ns/pixel
just the cell reads, chunk-hoisted    0.45 ms    2.8 ns/pixel
```

The obvious suspect — `cell_colour` doing a `HashMap<ChunkCoord, Chunk>`
lookup, and therefore a SipHash, **per pixel** — is real but is only **12%**
of the redraw. Hoisting the chunk out of the inner loop (64 consecutive
pixels share one at 1:1) would save ~1.35 ms. Worth having, not the answer.

**84% is the colour computation itself.** Isolating the branches by drawing a
world that is entirely one thing:

```
all empty sky   76.3 ns/pixel
all stone       62.5 ns/pixel
all water      108.4 ns/pixel
```

There is no single hot branch to remove — the whole per-pixel path is heavy.
62 ns for *stone*, the cheapest case, is ~250 cycles for what is nominally a
palette lookup. Visible in `cell_colour` on that path: `palette[shade as
usize % palette.len()]` is a **runtime integer division** per pixel;
`materials.get(cell.material)` and `materials.kind(...)` are re-fetched three
or four times per pixel; `rng::jitter` hashes per pixel; `sky::apply_light`
does per-pixel float work; and 45% of a real screen is empty sky, which
additionally runs a gradient, a moon-distance test and a star hash per pixel
even though the gradient varies only with `y`.

### What to do, in order of payoff

1. **Stop precipitation forcing a full redraw.** It is the largest single
   trigger — 52% of frames over 3,000 on this seed, and it fires for minutes
   at a time without a break. Drawn precipitation is already
   position-hashed against *world* coordinates in `draw_precipitation`, so
   its footprint is computable — union the drop rectangles into the dirty
   region the way `chunk_bodies` was already converted from "force full" to
   "union its rects" (that change is documented in `draw` and was made for
   exactly this reason, after a play report). This is the single largest
   frame-cost item in the audit and the fix has a precedent in the same
   function.
2. **Same treatment for `sky changed`.** A sky change repaints every pixel;
   the sky is a function of `y` (gradient) plus the moon and stars, so the
   ground half of the screen does not need repainting for it at all.
3. **Make the per-pixel path cheaper** — a per-material 256-entry shade table
   to kill the modulo, one `materials.get` hoisted per pixel, a per-row sky
   gradient. These cut the cost of the redraws that remain.
4. **Hoist the chunk lookup** (~12%, and it also unblocks the fake-AO work
   that was cut for cost — `cell_colour`'s own comment names this as the
   prerequisite).

### The camera does *not* drive this in the shipped build — but the camera change in flight will

The intuition "the camera follows the gnome, so walking repaints everything"
is wrong for the code that ships today, and **right for the code currently
being written**. Both halves matter.

Measured at `bb20167`, gnome holding right for 600 frames, reproduced through
`App::update`/`App::draw` exactly as `main.rs` drives them
(`examples/camera_snap.rs`):

```
player moved on 126 of 600 frames (world x 256 -> 382)
camera moved on   1 of 600 frames
  f174: camera 0 -> 86 (+86), player at (342, 176)
```

One move, of 86 cells, on an ordinary `+1, +0` walking step. That is not a
bug and not a teleport — it is exactly what `Renderer::follow` is written to
do. `dead_x = span_x / 6 = 85`; crossing it **re-centres** rather than
dragging to the boundary, so `cam_x = target.0 - span_x / 2 = 342 - 256 =
86`. The measured frame and the measured distance both fall straight out of
the arithmetic.

And `follow`'s own comment gives the reason, which is this audit's subject:

> Dragging to the boundary leaves it sitting exactly on the edge, so the very
> next step crosses again and the camera moves every single frame anyone is
> walking — which is a full-screen repaint every frame, the precise cost the
> dead zone exists to avoid.

**So the shipped camera is already optimised for this, deliberately, and
should be left alone.**

The warning is for the uncommitted work. `src/render.rs` in the main tree
currently carries ~251 uncommitted lines introducing an *eased* camera —
`camera_ease = 0.12`, `camera_max_step = 6.0`, a fractional `camera_fx`
shadow, and a dead zone that is a float rather than `span/6`. By
construction that moves the camera a little on **many** frames instead of a
lot on one. On today's renderer every one of those frames is a full-screen
repaint at ~12 ms, so an eased camera converts one repaint per 86 cells of
walking into a repaint on most frames of walking. The comment above predicted
this before the change was written.

That is not an argument against the eased camera — it is much nicer to look
at, and this project ranks feel above frame cost with the standing caveat
that frame cost is a hard constraint. It is an argument about **ordering**:
the eased camera needs "a camera move no longer forces a full repaint" to
land first or alongside it, or it will read as a large performance regression
that arrived with a camera change.

**A trap worth recording, because it nearly went into this report as fact.**
The first pass at this section explained the 86-cell jump using
`camera_max_step`, `camera_ease` and a "re-centre in one frame" branch, and
concluded the jump was unexplained because 86 exceeds a 6.0 step cap. All
three of those exist only in the main tree's *uncommitted* `render.rs` — the
build being measured was `bb20167`, whose `follow` has none of them. The
function was read out of one tree and the numbers measured in another.
`CLAUDE.md` already says to re-read the function rather than the diff after a
stash or merge; the same applies to reading a file out of a working tree
somebody else is mid-change in. Check `git status` on the tree you are
*reading*, not just the one you are building.

### Still open: the gnome stops walking

Separate from the camera, and not chased down here. With `right` held for 600
frames the gnome moved on 126 of them and then stopped dead at frame 253 at
x = 382. Terrain he cannot climb is the likely and boring explanation, but it
is unverified, and it means **no measurement in this audit covers sustained
running** — including the claim directly above about how often an eased
camera would move.

## Problem 2: the field solves most of the world every frame, forever

This is the cost that never goes away — 7.0 ms on a dry, quiet world where
the draw costs 0.07 ms, and 97% of that frame.

`field::step` has per-tile sleeping already (issue #4 work). `perf_counters`
reports what it actually achieves on the shipped world:

```
awake CA chunks:  17.4 of 320   (5%)
field tiles solved: 184–218 of 320   (58–68%)  every frame
frames fully quiet: 0 of 1200   (0%)
```

**5% of the world is moving and 58–68% of it is being solved.** The world is
never fully quiet, so the global early-out
(`active_chunk_count() == 0 && fields_settled()`) essentially never fires.

The cause is stated outright in `field.rs`, and is a deliberate,
well-reasoned decision:

> The consequence is honest and worth stating: sky-lit tiles never sleep,
> because the sun really is always moving. What sleeps is everything the
> light does not reach, which on a large world is most of it.

**The measurement contradicts the last clause.** On the world that actually
ships — 2048x640, ten chunks tall, mostly sky and surface — what the light
does not reach is *not* most of it. It is about a third. The reasoning was
sound for a deep world and does not survive the aspect ratio the game
shipped with.

Two further costs in the same function, both already documented in its own
comments:

- `apply_sky_to` runs over **every** tile, never subsetted, unconditionally
  — the comment explains why (subsetting it froze the day/night cycle and
  `the_sky_keeps_cycling…` caught it), so this is a known trade, not an
  oversight.
- Sleeping tiles are **cloned** into `next` every frame; a `FieldTile` owns
  five boxed slices, so ~124 sleeping tiles is ~620 allocations plus memcpy
  per frame for tiles nothing touched. The comment records that removing the
  clone was tried, made things dramatically worse, and needs `apply_sky` to
  write into the solved subset first.

### What to do

**Divide the oscillator out of the field's own sleep decision.** This is
`CLAUDE.md`'s own rule — "a channel that oscillates by design must be divided
out of decisions" — applied one level lower than it currently is. The engine
already has `field::noon_equivalent_light` so that *consumers* are not fooled
by the day/night swing; the field's storage and its convergence test are not
using the same trick. Store light normalised to noon and multiply by
`sky_light_amplitude(frame)` at read time: the amplitude is a pure function
of the frame, so a sky-lit tile stops drifting, stops re-solving, and can
sleep. `apply_sky_to` still walks every column for attenuation, but it would
be writing a value that no longer changes every frame — which is also the
precondition the clone-removal note says it needs.

Expected payoff, from the numbers above: the dry-quiet frame is 7.2 ms of
which the field is 7.0 ms, and 58–68% of the solved tiles are solved *only*
because the sun moved.

## Smaller findings

- **Worldgen is 367 ms per `F6` reroll** (median of 8, at 2048x640). `F6`,
  `F8`, `R` and `F7` all pay it. `App::next_seed`'s own doc argues the
  generator is judged by rerolling constantly — a third of a second per
  reroll is a real tax on the workflow the design depends on.
- **Active sites cost 7.3 ms mean (24% of frame) while digging**, against
  0.04 ms when not. `MAX_SITES_PER_FRAME`'s doc asks to "revisit with a real
  per-site cost measurement if a scene is ever found where this is either too
  low or too high" — this is that measurement, and 2,000 is too high for the
  16.6 ms budget when the sites are structural checks.
- **`scheduler::step` calls `std::env::var("PROBE_NO_LOAD")` every frame.**
  That allocates and takes the process-wide environment lock, in the frame
  loop, to read a debug flag. A `OnceLock` fixes it. Small, but it is in the
  per-frame path of the shipping build.
- **`parallel::step` filters the whole active-chunk list once per pass**
  (`active.iter().filter(...).collect()` inside the pass loop), so it is
  O(passes x active) with a `Vec` allocation per pass. Irrelevant at 40
  chunks; worth knowing before M10 streaming grows the chunk count.
- **The CA sweep's parallel width is capped at chunks-per-row / 2** by the
  `(chunk row, cx parity)` pass key — 4 at the current width, on a 4-core
  machine. Stated from reading `pass_key`/`run_pass`, **not measured**: the
  sweep is 1–4% of a frame, so this is not currently worth acting on.
- **The dirty region is a single unioned bounding rectangle**, so two touched
  chunks at opposite corners repaint everything between them. Not separately
  measured — on the dry seed the whole dirty path was only 1,700 px/frame, so
  it is not currently hurting.

## The one-line summary

Fix the full-redraw triggers (rain first) and the field's sky-lit sleep
decision, and the two of them account for essentially the entire gap between
22.3 ms and the 16.6 ms budget. Nothing in the CA sweep needs to change.
