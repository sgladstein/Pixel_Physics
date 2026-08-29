# The renderer had no owner, and 94% of a redraw was one cache miss

*Measured 2026-08-29, on the shipped 8192x2560 world. Prompted by a
cross-session handoff from the perf lane, which measured the cost and
attributed it to the sky. The cost is real and the attribution was wrong; the
correction is §3.*

**Headline: a full redraw went 39.6 ms to 2.5 ms, and the output is
byte-identical.** The whole of it was `rebuild_near_glow` — the crystal-glow
splat — running on essentially every frame of play because its rebuild
condition asked a question about the *screen* instead of about the world.

## 1. Nothing in the repo could say where a redraw's time went

Every frame-cost instrument here times `App::update`. `render_cost` says what
a redraw *costs* in total; `frame_profile` says which **simulation** phase a
frame went to. Between them the largest single cost in the frame had no
owner, so every whole-frame figure on record — this report's own predecessor
`frame-cost-audit-2026-08.md` included — was half a number.

So the first thing built was the missing split:
`PIXEL_PHYSICS_DRAW_TIMING=1` breaks `Renderer::draw` into its phases. It
found this in one run:

```
draw preamble:                 0.00 ms
draw rebuild_horizon:          0.71 ms
draw sky_light:                1.44 ms
draw   ...field-tile scan:     0.04 ms
draw glow scan + near_glow:   37.33 ms
draw pixels:                   1.91 ms
draw overlays + particles:     0.27 ms
```

**The pixel loop is 1.9 ms.** Everything anyone had reasoned about — the sky
gradient, the star hash, the per-pixel chunk lookup, the sky-view ray fan
added the same day — lives in that 1.9 ms. The other 37 ms was one function.

`render_cost`'s own viewport table had been saying so for months and nobody
read it that way: **4x the pixels cost 13% more time** (39.9 → 45.2 ms). A
per-pixel cost cannot do that. A fixed cost can.

## 2. Two independent defects, both in `rebuild_near_glow`

### It ran on nearly every frame, and the trigger is the bug

```rust
if emitters_touched || force_full || self.near_glow_key... != ... {
```

`force_full` means *"an overlay is on screen with no tracked footprint, so
repaint every pixel"*. In the app it is true whenever **the cursor is over the
window**, because the brush outline follows it (`App::draw`). So the splat was
rebuilt on essentially every frame of play.

The splat has nothing to do with what is on screen. Its own doc comment said
so already — *"The splat reads `Material::glow` off cells and nothing else, so
what can invalidate it is a cell changing"* — and the two other terms ask
exactly that. `force_full` was doing one thing by accident, badly: guarding
`App::reset`, which keeps the `Renderer` while building a new `World`. It
guarded it only when the cursor happened to be over the window. That is now
`Renderer::forget_world`, called from `reset`, which is a small **correctness**
gain rather than a performance one.

### Each rebuild hashed twice per written pixel

13 emitter tiles, 3,339 glowing cells, **2,046,807 splat writes**. Every one
of those writes did a `ChunkCoord::containing`, a `HashSet::contains` and a
`HashMap::entry` — two SipHashes — plus a `sqrt` for a falloff that is a
function of the offset alone. 18 ns a write, which is exactly what two hash
probes cost.

Two changes, no behaviour:

- **The falloff is a table.** 841 entries computed once, not two million
  `sqrt`s.
- **The disc is walked per destination chunk.** A radius-14 disc spans at most
  a 2x2 block of 64-cell chunks, so resolving those up front turns four
  million hash probes into eight per glowing cell and leaves plain array
  writes inside. The source cells get the same treatment — one chunk lookup
  per emitter tile instead of 4,096 `World::get` calls, which is the lesson
  `rebuild_sky_light`'s block scan already records, arriving here by the same
  route.

Rebuild cost: **37.2 ms → 5.4 ms**, and it now happens when a crystal is
exposed or mined rather than when the mouse is over the window.

## 3. The correction: it was never the sky

The handoff attributed the cost to PR #94 doubling `sky_rows` (95 → 190), on a
sweep showing a step between 115 and 120 with the sky share climbing smoothly
across it. The step is real. The cause is not the sky.

`assets/worldgen.ron` is runtime-loaded, so this is one binary and four data
files — no rebuild between arms, and no stale-binary hazard at all:

| `sky_rows` | glowing cells | redraw, before | redraw, after |
|---|---|---|---|
| 95 | 726 | 10.29 ms | 2.50 ms |
| 115 | 832 | 13.98 ms | 2.47 ms |
| **120** | **3,413** | **45.91 ms** | 2.59 ms |
| 190 | 3,339 | 39.80 ms | 2.28 ms |

**The step is the glowing-cell count, not the sky.** It jumps 832 → 3,413
between 115 and 120: sinking the terrain moves a geode into the generated
depth band. Cost tracks glowing cells at a near-constant 0.012–0.017 ms each
across all four arms; sky share was a correlate riding along with it.

This matters beyond the attribution. Read as a sky cost it says *"the taller
sky was a mistake, consider reverting #94"* — a worldgen decision, and the
wrong lever. Read as a glow cost it says *"one geode in view costs 40 ms a
frame"*, which was true before #94 for anyone who dug down to one, and would
have been true after any change that put a geode on screen.

It is the same shape as `CLAUDE.md`'s standing rule about sizing a problem
from the wrong measurement: the sweep was correct, reproducible and about a
different quantity than the one it named.

## 4. What the guard was missing

`a_settled_glow_does_not_rebuild_its_halo_every_frame` existed and passed
throughout. It drew eight frames with `force_full: false` — a state the game
is almost never in. It has been extended to run the same loop at `true`, and
to check `forget_world` actually invalidates; both were watched going red
against the old code before being trusted.

## 5. What is left

- The redraw is now 2.5 ms and **73% of it is the per-pixel chunk lookup**
  (1.4 ms), which `render_cost` has always measured and which was 3% of the
  old number. That is the next thing in this function worth touching, and it
  has a known shape: hoist the chunk across a run of pixels, as the draw loop
  already does for `cell_colour`.
- `rebuild_horizon` copies the whole per-cell genesis map every frame (0.4–0.7
  ms at this world size) for the same `App::reset` reason `forget_world` now
  handles properly. Not changed here — it is a tenth of what was just removed
  — but it is the same fix if it ever matters.

*Freshness: 2026-08-29.*
