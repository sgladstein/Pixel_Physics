# The lab was lighting a room that has a ceiling

*2026-08-30. Where the evolution lab's draw actually goes, and what taking
half of it out cost. Follows
[evolution-lab-gate-1-2026-08-30.md](evolution-lab-gate-1-2026-08-30.md) §5.3,
which found the number and named it "next optimisation, and it is not where
anyone was looking". The pass itself is
[sky-light-design.md](sky-light-design.md).*

**Read §1 if you read nothing else.** §2 is where the cost lives, §3 is what
changed, §4 is the evidence that the picture did not, §5 is what is left.

---

## 1. What it says

1. **The 2.8 ms reproduces, and it is in the draw, not the field.** It is
   `Renderer::rebuild_sky_light`, called from `Renderer::draw` — not
   `field.rs`'s solve, which is the other thing in this engine with "light" in
   its name. §2.
2. **It ran every frame because a live box always has a touched chunk.** The
   rebuild's gate is `grid empty || camera moved || scale changed ||
   !touched.is_empty()`. The lab's camera never moves and its sky is held at
   noon, so the third clause is the only one that ever fires — and in a bed
   with fifty walking ants it fires on essentially every frame. **The sky
   being held was never the thing keeping it awake; the world changing was.**
   §2.
3. **The cost inside it is a per-cell scan, not the propagation.** Split at
   block 4 over the lab's 640x448 region: **build 1.55 ms, view fan 0.90 ms,
   four sweeps 0.72 ms.** The scan reads 286,720 cells to fill 18,193 blocks,
   and it re-read all of them whatever moved. §2.
4. **So the scan is cached per block and rerun only where a chunk was
   touched.** Everything downstream is a pure function of two per-block counts
   — solid cells and open-sky cells — so when neither moved, the grid cannot
   have moved and the rebuild returns without touching it. §3.
5. **The lab's draw falls by about half, paired and alternating.** Fresh box
   **4.80 -> 2.56 ms** (4 of 4 pairs); settled box **2.97 -> 1.34 ms** (3 of
   3). Whole frame, tick included: **7.30 -> 4.94 ms** fresh and **4.55 ->
   2.80 ms** settled. §3.2.
6. **The picture does not change.** `labshot`'s contact sheet and a
   `filmstrip` of the generated outdoor world are both **byte-identical**, and
   over 3,000 per-frame comparisons against a from-scratch renderer in the lab
   bed — fresh and settled — **not one pixel differed**. §4.
7. **The frame hash is the weaker of the two checks and that is not
   obvious.** With the cache's change detection deliberately broken,
   `labshot`'s sheet still came back byte-identical: inside a sealed box the
   light grid moves in places that quantise to the same colour. The outdoor
   `filmstrip` did move. **A picture is a positive control only where the
   thing under test can reach a pixel.** §4.2.
8. **Half the remaining pass is still spent solving for an answer that does
   not change.** Over 1,500 settled frames: 378 solves ran the fan and the
   sweeps, and 367 moved a byte of the grid — but *how much* they moved it is
   at most **20 bytes of 18,193**. That is the next thing here, and it is a
   harder change than this one. §5.

---

## 2. Where the 2.8 ms lives

Reproduced on this box before anything was touched, with `lab_cost
render_every=1` on `LabBox::default()` and `PIXEL_PHYSICS_DRAW_TIMING=1`:

| draw phase | ms |
|---|---|
| **sky_light** | **3.30–3.64** |
| pixels | 3.5–3.8 |
| glow scan + near_glow | 0.95 on the first rebuild, ~0 after |
| preamble, horizon, overlays | 0.04 |

Those are inflated by the seven `eprintln`s a split adds per frame; the
un-split draw on the same bed measures **4.57–4.80 ms**, which is where §5.3's
4.78 came from. Either way the sky-light pass is the largest single term and
it is **in `render.rs`, not in `field.rs`** — a point worth stating because
this engine has two things called light and the coordinator note's phrasing
("the draw") is right.

`PIXEL_PHYSICS_SKY_LIGHT_TIMING=1` splits it further, and this is the number
the change is built on:

```
sky_light block=4 region=640x448 cells=286720 blocks=18193
          build=1.55 ms view=0.91 ms sweep=0.70 ms
```

**`build` is a cell scan, and it is the largest of the three.** Per block it
counts how many of the block's cells are solid and how many stand under open
sky; `view` traces the eight-ray aperture fan; `sweep` runs four directional
passes twice. The scan is 286,720 `under_sky` reads and material lookups, and
it ran in full on every frame anything anywhere in the box moved.

**Why every frame.** `Renderer::draw` rebuilds when the grid is empty, the
camera moved, the scale changed, **or `touched` is non-empty**. The lab holds
its sky (`set_sky_hold`) and never moves its camera, so the first three never
fire after frame 1 — and the fourth fires constantly, because plants grow and
a colony walks. Measured over 400 lab frames on the unchanged build: **the
pass ran on every single one.**

**43% of the scanned region is not even in the world.** The region is the
viewport plus a 64-cell margin, so at 512x320 it is 640x448 = 286,720 cells
against a world of 163,840. The margin is deliberate and load-bearing —
everything outside the world counts as *wall*, which is what stops daylight
running down the columns beside the world and lighting a sealed room through
the edge — but it does not need a per-cell loop to say so.

---

## 3. The change

### 3.1 What it does

`Renderer` keeps the per-block occupancy it derived the grid from —
`sky_light_solid` and `sky_light_outdoors`, one byte each per block. A rebuild
now:

- **reuses them** when the region, block size and dimensions are unchanged and
  the grid is not empty, and rescans only the blocks the touched chunks cover.
  `CHUNK_SIZE` is 64 and blocks are 1, 2 or 4 aligned to world multiples of
  their own size, so a block never straddles a chunk: the block range of a
  chunk is exact, and one chunk lookup covers every block in it;
- **returns without doing anything else** when no count changed. This is the
  case the lab is in for most of its frames — a root thickening inside soil
  that was already solid, an ant stepping between two cells of the same block;
- **answers a block wholly outside the world without scanning it** — the two
  rules the per-cell loop applies there are constant over such a block;
- **tabulates the Beer-Lambert transmission by solid count.** A block holds
  `area + 1` distinct counts, seventeen at block 4, so the two `powf`s per
  block were 36,386 calls computing 17 answers.

Three things invalidate the cache, and between them they are the whole
correctness argument: `forget_world` (a new world under the same `Renderer`,
which is what `App::reset` does), a change in the copied `underground` map
that `under_sky` reads per cell, and the region/block/dimension check itself.
Cell changes arrive through `touched`, **on exactly the contract the
dirty-rect pixel skip already runs on** — a cell that changed without dirtying
its chunk would leave a stale pixel too, so this adds no assumption the draw
was not already making.

`forget_world` also drops the grid now, which is a pre-existing gap rather
than one this cache opened: the rebuild is called only when the grid is empty,
the camera or scale moved, or something was touched, so a reset that happened
to dirty nothing would have kept the previous world's lighting on screen. It
never bit because building a world touches every chunk it writes — a
coincidence of `App::reset`'s order, not a contract.

### 3.2 What it costs, measured against the whole frame

`CLAUDE.md`: *removing work is not the same as removing cost* — a gate that
took 91% of the field's momentum work out once made the frame **slower**,
because the passes it skipped had been touching every tile and the pass after
them paid the cold misses instead. So the number below is the **whole draw**,
not the phase, taken **paired and alternating** between two binaries built
from one tree, differing only by `src/render.rs`.

`lab_cost render_every=1`, `LabBox::default()`, the window named in the header:

| bed | draw, before | draw, after | delta | pairs |
|---|---|---|---|---|
| fresh (frames 1–2,000) | 4.785 / 4.794 / 4.845 / 4.775 | 2.531 / 2.570 / 2.563 / 2.577 | **−2.24 ms, −47%** | 4 of 4 |
| settled (frames 10,001–12,000) | 2.914 / 3.145 / 2.863 | 1.364 / 1.330 / 1.311 | **−1.64 ms, −55%** | 3 of 3 |

**Settled as well as busy, deliberately.** `CLAUDE.md` records an animated
grain that measured free in every moving scene and cost ~10 ms/frame on a
settled one, because a settled world is exactly where the dirty-rect skip does
its work. A settled box is where this change has the *least* left to remove —
the pixel loop is already nearly free there, so the sky-light pass is most of
what remains — and it still takes more than half of it.

The whole frame, tick and draw, at one draw per tick:

| bed | tick | before | after |
|---|---|---|---|
| fresh | ~2.4 ms | 7.30 ms | **4.94 ms (−32%)** |
| settled | ~1.5 ms | 4.55 ms | **2.80 ms (−38%)** |

**In the units the speed dial is quoted in.** At a settled 1.5 ms tick and a
60 Hz display, `(1000/hz − draw) / tick` gives **9.1x real time before and
10.2x after**, against a ceiling of 11.1x if drawing were free. The renderer's
tax on the dial falls from **18% to 8%** — more than half of it gone. The
absolute gain looks modest because once the draw is cheap the *tick* is the
budget, which is the same shape §5.4 of the Gate 1 report found when it
measured the display rate as worth 22% rather than the expected tripling.

**Outdoors, where the cache can never help, it costs nothing.** `render_cost`
on the shipped 2048x640 world does a *full* redraw, which is precisely the
path with no reuse in it: three alternating pairs read 4.061 / 3.732 / 4.130
ms before and 3.834 / 4.046 / 3.557 after — medians 4.06 against 3.83, inside
the spread and not a regression in either direction.

---

## 4. The picture

### 4.1 What was compared

- **`labshot frames=0,600,3000,9000`** — the lab contact sheet, one
  `Renderer` reused across the stops with real touched sets, so the
  incremental path is exercised. **sha256 identical**, before and after.
- **`filmstrip scene=worldgen start=2 every=200 count=6`** — the generated
  outdoor world, where sky light is what draws the dark under a brow.
  **sha256 identical**, before and after.
- **`skylight_cost check=1 checkevery=1`** — the lab bed, drawn every frame
  through the running `Renderer` *and* through one made a moment ago, which
  can only do the full scan. 1,500 comparisons on a fresh box and 1,500 on a
  settled one: **0 differing pixels in all 3,000.**

### 4.2 The trap in that, said plainly

**A frame hash is a positive control only where the thing under test can reach
a pixel, and in the lab it cannot.** With the incremental rebuild's change
detection deliberately broken — the cache never notices anything, which is the
exact fault it exists to avoid — `labshot`'s sheet came back **byte-identical
to the correct one**. The outdoor `filmstrip` sheet did move, which is what
makes it the control that works.

So the discriminating comparison is the **grid**, not the picture. The same
per-frame check that reports 0 differing pixels reports the grid differing on
234 of 1,500 frames — by at most 98 bytes of 18,193, never for more than 2
consecutive frames.

**Those differences are the touched-set's own one-frame lag, shared with the
pixel path.** Organism growth and creature movement run in
`step_active_sites`, which is *after* `World::end_step` in `frame::step`, so a
write there reaches `touched_chunks` only on the next tick. The dirty-rect
pixel path is already one frame behind on exactly those cells; this cache is
exactly as behind and never more, and it self-heals the moment the chunk is
reported. `skylight_cost`'s bar is set on that: nothing may *persist*, because
a lasting difference is a missing invalidation rather than a frame shared with
the renderer around it.

### 4.3 The guards, and watching them go red

`CLAUDE.md`: a guard's green is worth nothing until the fault it is named for
has been put back.

| guard | fault put back | result |
|---|---|---|
| `an_incremental_sky_light_rebuild_agrees_with_a_full_one` | the incremental branch never sets `occupancy_changed` | **red at step 0** |
| `forget_world_drops_the_sky_light_block_cache` | `forget_world` keeps the occupancy and drops only the grid | **red on the occupancy assertion** |
| the two above, on the *pixel* comparison alone | either of the above | **green** — which is why both assert the grid first |

**One guard was written, found blind, and deleted rather than kept.** A test
for the `underground`-map invalidation stayed green with that invalidation
removed, because the map can change exactly once in a world's life — the
first step, which also wakes every chunk, so the incremental branch rescans
everything on that draw anyway and the stale cache is overwritten by
coincidence. The invalidation stays (four lines, and its comparison replaces a
copy rather than adding one) and the code says in place that nothing tests it.

---

## 5. What is left, and why it was not taken

**The scan is gone; the solve is not.** Over 1,500 settled lab frames the pass
now takes three routes: **1 full**, **378 incremental** (scan, then fan and
sweeps), **620 held** (scan, nothing moved, grid kept), and 501 frames where
`touched` was empty and it was not called at all.

So roughly a quarter of frames still pay the fan and the four sweeps — about
1.6 ms when they fire. And **367 of those 378 solves did move the grid**, so
they are not wasted in the "did nothing" sense. What they are is *tiny*: the
largest per-frame movement measured across the settled window is **20 bytes of
18,193**, and the largest across the fresh window is 98.

The obvious next step is to predict that before running the fan — bound the
propagation to the part of the grid that can carry light above the
quantisation floor, which in a sealed box is the handful of block rows under
the ceiling. It is deliberately not in this change:

- it is a **cap that decides an answer**, which is the shape `CLAUDE.md`
  names as having been got wrong three times, twice by reports quoting the
  rule while failing it. A bound on where light can reach is exact only while
  every decay factor is strictly below 1 and every source is a seeded block;
  both are true today and neither is written down as a contract;
- the scan cache is exact by construction — it computes the same two counts
  the same way — and a bound is exact only by argument. Those are different
  kinds of change and they should not land together.

**And one thing worth knowing before anyone reaches for it.** The lab's
picture does not read the sky-light grid at all in any frame sampled here: the
broken-cache build drew the same contact sheet. If that survives a closer
look, the cheapest remaining move in the lab is not a faster solve but a
narrower region — and that is a question about `sky::Interior` and the lab's
own render, which this lane does not own.

---

## 6. Provenance

Every timing on one machine, in one session, four cores, base and change built
from the same tree and alternated run by run. Ticks are unchanged by
construction (this is a render change) and measured unchanged: 2.4–2.6 ms
fresh and 1.4–1.8 ms settled in both arms. Worst frames are reported by the
harness with the `mean x frames ≈ worst` ratio beside them and **are not
quoted here** — nothing pins them, so they are noise wearing a number.

`cargo clippy --all-targets --release --locked -- -D warnings` clean;
`cargo test --release --lib render::` 83 passed, 0 failed. The full library
suite does not fit in one foreground call on this container and is left to CI,
which is the route the coordinator note names for exactly that reason.
