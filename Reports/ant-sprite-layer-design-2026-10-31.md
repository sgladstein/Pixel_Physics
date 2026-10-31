# Drawing an ant over the two cells: a render-only sprite layer for the lab

*Design, 2026-10-31, thread "Make the ants look good". Owner's ask:
"It really does suck, it is just two pixels. Is there an animation or
something else we can put on top of it — we know all the forms and shapes
those pixels can take — that'll make it look really pretty and cool,
especially when they're swarming around." Nothing in `src/` changes with this
report. The picture page is
[Ant Sprite Styles](https://claude.ai/artifact/HYZVWvNjtMKjTsEt7o2UcH).*

## 1. The short version

The simulation keeps its two-cell ant. The renderer draws an ant **over** it,
on a layer of its own at the interface layer's resolution, from data every ant
already carries: the two cells (body axis, one of eight), `heading`,
`crop` (how full), `spoil` (a pellet or a store load in its jaws), `flight`
(rarely set: only a launch, §9 item 13). Nothing is written back to the world, so training, scores and
determinism are untouched by construction (§6).

Three things make it look alive, and all three are in every style on the page:

1. **Glide.** An ant steps one whole cell at most once per its own tick
   interval — the species' `tick_interval: 6` scaled by its pace allele, 3 to
   12 frames (`creature::tick_interval_of`). The sprite slides from the old
   cell to the new one over that interval instead of jumping.
2. **Legs that walk.** Six legs in the alternating-tripod gait, phased by
   distance actually travelled, so a standing ant's legs are still and a
   running ant's legs blur.
3. **Load you can see.** A pellet in the jaws (`spoil`), and an abdomen that
   swells and pales with `crop` fill — graded with the fill, not on/off
   (`CLAUDE.md` law 1).

The style question — how those are drawn — is the owner's call, and the page
shows three. **Recommendation: B, with A and C kept behind a key** (§5).

## 2. Why this is not the rejected "sprite body plan"

`Reports/dead-ends.md` records (creature-direction.md §0, D1) that
*Noita-style sprite entities atop the grid were rejected as the creature
body plan*: sprites stop being made of the world (no genome-as-anatomy, no
partial damage as cell loss). That rejection is about **what the animal is**.
Here the animal is still its cells; collision, digging, eating, being eaten
and every sense still act on the two cells. The sprite is paint. If the
sprite is turned off, the game is bit-identical.

`Reports/creature-appearance-design.md` is the input this builds on. It found
that **extent is the only lever** for finding an ant in a picture (decoys at
the shipped two cells: 127; at nine: 15; at sixteen: 0) and that extent
costs mobility when it is real body. A sprite gets extent for free — the
drawn ant is about three cells long with legs — without touching the body.
That report's §7 also says appearance cannot be in the genome; nothing here
changes that.

## 3. What was looked at

`examples/antlook_dump.rs` (scratch harness, this branch) runs `played_bed`
seed 1 to frame 45,000 (250 ants alive, all at or near the nest) and dumps
150 frames of the real renderer's output plus every animal's chain, heading,
crop and spoil. The mock-ups were drawn offline over that recording, with the
ant cells replaced by what was last seen under them. Counts over the window:
250 ants, 27 holding a pellet in the jaws, ~40 with food in the crop.

Every style was rendered at three scales, in layer pixels per cell:

| scale | where the player sees it |
|---|---|
| 2 px/cell | the whole box at zoom 1 (the layer is 2x the box) |
| 4 px/cell | zoom 2 |
| 10 px/cell | zoom 5, a close-up |

**Stand-out** beside each picture is `examples/creature_look.rs`'s *ink*
quantity, per ant: summed absolute luminance difference between the frame and
the same frame without ants, divided by ants drawn and by pixels per cell.
Specificity check: today's two cells score 134 to 169 across the three
scales, the same order everywhere (one quantity, three samplings). It is a
*visibility* number, not a *reads-as-an-ant* number: a dark leg on dark soil
adds shape and almost no ink.

| style | close-up | play zoom | whole box |
|---|---|---|---|
| today (cells) | 134 | 163 | 169 |
| A | 117 | 208 | 529 |
| B | 126 | 226 | 348 |
| C | 355 | 697 | 1,085 |

**Blind key — kept off the page:** A = smooth (anti-aliased, continuous
rotation, soft shadow, thin dark rim); B = crisp pixel (no anti-aliasing,
sixteen drawn headings, one-pixel dark ring, snapped to layer pixels);
C = bold sticker (30% larger, pale ring). All three share the glide, the gait,
the pellet and the swelling abdomen; only the drawing differs.

What the pictures say, by eye (mine, to be overruled by the owner's):

- At **2 px/cell** nothing has legs that read; every style is a bean with a
  rim. A and C read as a smear in the dense pile; B's dark ring separates
  individuals best. LOD is therefore not optional (§4.4).
- At **4 px/cell** B reads as ants with legs; A's thin anti-aliased legs fade
  into the soil (hence the lower stand-out); C reads as a mass of white
  outlines.
- At **10 px/cell** A is the prettiest single ant; B is close and matches the
  world's pixel look; C is cartoon.

## 4. How it would be built

### 4.1 Where the drawing goes

The lab already composites two textures on the GPU (`src/bin/lab/layer.rs`):
the box (`WIDTH x HEIGHT` times `pixel_scale`) and the interface layer at
`LAYER_SIZE` = 1024x640, two layer pixels per logical pixel. The interface
layer is repainted only when chrome is due (`Lab::chrome_due`, at most every
`CHROME_LIVE` = 30 ms while running), so ants cannot ride it — they move every
frame and the chrome must not be forced to repaint at 60 Hz
(`Reports/lab-ui-redesign-2026-10-22.md`'s cost rule).

So: **a third texture, the creature layer**, also `LAYER_SIZE`, premultiplied
RGBA, composited between the two: `out = ui + (creature + box * (1 - ca)) *
(1 - ua)`. `render::composite_layer` gains the same middle term so the
screenshot hook stays exact. CPU side, it is a `Vec<u8>` owned by the lab;
each frame clears the union of last frame's sprite boxes, draws this frame's
sprites, and uploads only the row band that changed.

**The single-buffer path** (`PIXEL_PHYSICS_LAB_CRISP=0`, and every harness
that calls `Lab::draw`) keeps today's cells. Drawing sprites into the box
buffer would defeat the dirty-rect skip (a sprite overhangs cells no chunk
dirtied) and at 1 px/cell there is nothing to draw. Harnesses that need
pictures of the sprite get it through `draw_layered` + `composite_layer`
(`labgif sprites=1`, to be added).

### 4.2 Hiding the cells under the sprite

The glide makes the sprite lag its cells by up to six frames, so the cells
would peek out ahead of it. With sprites on, the renderer draws a sprite-drawn
species' body cells **as an empty cell at that position** — the tunnel or sky
colour the cell would show if the ant were not there. This is a change to
`Renderer::cell_colour`'s creature block (`src/render.rs`, the block gated on
`CellType::Head | Segment | ...`), behind the same switch, and it rides the
dirty rectangles the moving ant already causes.

**Turned off for every debug overlay.** When `organism_overlay` is anything
but `Off`, sprites are not drawn and cells draw as themselves: a debug
readout must not be a function of the thing it debugs (`CLAUDE.md`, Method).

### 4.3 The per-ant visual state (renderer-owned)

A map `OrganismId -> AntVis { from, to, moved_at, angle, phase }`, kept per
world the way `pale_drawn` is (organism handles are per world; the lab draws
other chambers through the same renderer). Updated once per draw from
`World::live_organism_ids`; entries for gone ids are dropped.

- **Position**: body midpoint of the two cells. On a change, `from = to`,
  `to = new`, `moved_at = world.frame`; drawn at
  `lerp(from, to, min(1, (world.frame - moved_at) / tick_interval))`. A jump of
  more than two cells (a hop, a crossing, a reused handle) snaps.
  **Keyed on `world.frame`, not wall time**, so a paused box is still and a
  screenshot is reproducible; at speeds above one tick per draw the glide
  simply completes inside the draw.
- **Angle**: target from the head-minus-tail vector (one of eight); falls back
  to `heading` when the two cells are not adjacent. Eased toward the target
  (35% per draw in the mock-up), so a 45-degree turn is a turn, not a snap.
- **Gait phase**: advanced by distance travelled this draw. Standing still is
  phase-frozen.
- **Pose from state**, all read-only: `spoil` → pellet in the jaws, coloured
  from the spoil cell's material (a store load shows as food); `crop` →
  abdomen radius and paleness by `worth() / crop_capacity`, graded;
  `flight` → legs splayed.

### 4.4 Sprites, sizes and styles

Each style is a **set of pre-drawn masks**, built once per pixels-per-cell
size the first time that zoom is used: sixteen headings x four gait frames x
three abdomen sizes, as greyscale shade + alpha. A frame blits one mask per ant
and multiplies by the ant's colour. That makes the three styles the same code
with different masks, so keeping all three behind a key costs nothing at run
time.

Level of detail by layer pixels per cell (`zoom * LAYER_SCALE`):

| px/cell | lab zoom | what is drawn |
|---|---|---|
| < 2 | zoomed out (`zoom_out_stride > 1`) | nothing; cells as today |
| 2-3 | 1 | a three-blob bean with a rim, no legs; pellet as one pixel |
| 4-5 | 2 | legs and antennae, one pixel wide |
| 6+ | 3-8 | full: jointed legs, antennae, abdomen highlight |

**The colour** is the colour the renderer gives the ant's body today — the
colony/job/state/energy modes, the demo line tints and the carry cue — so
factor the creature block of `cell_colour` into one function both callers
use. A colour mode that works on cells keeps working on sprites.

Drawn back to front by screen row, so in a pile the lower ant is in front and
the rim separates bodies.

### 4.5 Scope

Version one: the two-cell walking ants, by species name — `ant` and
`ancestor` (§9 item 1; every ant on the lab's picker is the `ant` species
with a different brain). The beetle, spider, centipede, flitter, rove,
worm, `ant_block` and `longant` keep their cells; each is a later sprite set on the same
layer. The outdoor game and the held world have no creature layer yet; the
same module draws into one when they get one.

### 4.6 What comes after, if version one is liked

Each is an event that today leaves no mark, which `CLAUDE.md` says is
unfinished:

- **Two ants sharing food** (`last_share_frame`): antennae touch for a
  moment.
- **A dug cell**: the forelegs pump and two or three grains fly.
- **A death**: the sprite curls, legs up, and fades into the corpse cells
  over a second, instead of vanishing.
- **Climbing**: legs drawn toward the surface the ant is on.

## 5. Recommendation

*(Revised after review, §9.)* **B as the default, with a key that cycles B / A / C / cells, named on screen
(`CLAUDE.md`: "for does-this-look-right, ship a runtime selector").** B keeps
the world's pixel look, separates individuals best in a crowd at the two
zooms the lab is mostly watched at, and is the cheapest to draw. A is the
prettiest close up and can be the default at zoom 4 and above if the owner
likes it there. C is the most visible by a wide margin and the least like the
rest of the world.

Default **on** once built, by the project's defaults-on rule, unless the
measured frame cost (§7) is a harm.

## 6. Why the simulation cannot notice

- **The type system is the guarantee**: `Renderer::draw` takes `&World`
  (`src/render.rs:4904`), and the sprite pass will too. It writes only
  renderer state and the creature layer.
- Training mostly never draws; `train::picture` (`src/train/mod.rs:2120`)
  does, on a clone of the world, so it cannot feed back either.
- **Guard:** `tests/determinism.rs`'s `lab_hash` over N frames, drawing with
  sprites on and off, must match; then put a world write in the sprite path
  and watch the guard go red.

## 7. Cost, and how it will be measured

Estimated, not measured: per ant one mask blit of about (3 x px/cell)^2
pixels — ~150 px at zoom 2, ~600 at zoom 4 — so ~0.1 ms for 250 ants plus the
map update. The clear and upload are bounded by the union of sprite boxes, not
the screen. **Measured before shipping** with `examples/labdraw.rs` (it
already times the layered path, paused and running, with and without pages)
with a `sprites` arm on and off in the same process — **on a 250-ant bed**
(`frames=45000` on the played bed; its default warms a young colony), and
reporting **bytes uploaded per frame** alongside CPU time, since labdraw does
not time the GPU upload. `ascii` never builds a `Lab`, so CI's worst-frame
line cannot see this cost and is not the gate. A paused box must cost what it costs today: with the
world frame not advancing, nothing in the creature layer changes and nothing
is uploaded.

## 8. Build order

1. Factor the creature colour block into one function; no visible change.
2. Renderer-side `AntVis` map, glide and gait; masks for style B; the creature
   layer texture and shader term; `composite_layer`'s middle term; hiding the
   cells; the overlay rule. Guard from §6.
3. Styles A and C as further mask sets, the key, the on-screen name.
4. `labgif sprites=1`, and a before/after page from the real game, with the
   counts beside each picture.
5. `labdraw` arms, numbers in the PR.

## 9. Independent review (2026-10-31) and what it changed

A code-checking reviewer read this document against the source at
`2de3e9f`. Four facts were wrong and eight holes were found. Each is fixed
above or here; the build follows this section where the two differ.

**Facts corrected**

1. **Scope is by species, not by body plan.** `ant_block` is an 8-cell
   `Rigid` body (`assets/species/ant_block.ron:31`), not `Chain(2)`. The
   `Chain(2)` species are `ant`, `ancestor` and `flitter`; the lab's ant
   picker swaps brains, not species. Version one draws `ant` and `ancestor`;
   the flitter (which flies) is a later set.
2. **The step is 3 to 12 frames, per ant** (`tick_interval_of`,
   `src/sim/creature.rs:5426`). The glide divides by that ant's interval.
3. **`ascii` cannot see this cost** (it builds no `Lab`); `labdraw` is the
   gate (§7).
4. **"Training never draws" was too strong**; the real guarantee is `&World`
   (§6).

**Design holes closed**

5. **Hiding ant cells is a per-call request, off by default.** The same
   renderer draws chamber thumbnails, the single-buffer path, every harness
   and `train::picture`; a renderer-wide switch would leave holes where ants
   are in all of them. Only the layered paint asks for it, on that call.
6. **Hide at the top of `cell_colour` by substituting `Cell::EMPTY`** and
   letting the normal path colour it, not inside the creature block: the
   nest cutaway runs before that block (`src/render.rs:7513`), and an empty
   cell's colour depends on `background_at`, the cutaway's roofed test, the
   field overlay and others that only the normal path knows.
7. **Player marks must stay on top.** The pin marker and trail, life marks,
   battle view and demo marks are drawn into the box after `renderer.draw`
   (`src/lab/ui.rs:11156`, `src/lab/mod.rs:2456`), so a creature layer above
   the box would cover them. They move onto the creature layer, drawn after
   the sprites.
8. **A style switch forces one full repaint**: the sprite mode joins the
   `look` tuple in `Renderer::draw` (`src/render.rs:5181`), because it changes
   which cells are hidden without dirtying any chunk.
9. **Turning eases per world frame, not per draw**, so a paused box is truly
   still, uploads nothing, and a screenshot does not depend on draw rate.
10. **The `AntVis` map is cleared on a chamber switch or rebuild**
    (`view_dirty`). `pale_drawn`'s per-world key is the world's address, and a
    swapped-in world reuses it; a stale entry would glide a new ant from an
    old chamber's position. `live_organism_ids` already carries a generation,
    so a reused handle is a different id.
11. **The abdomen swell reuses the carry cue's rescaling** (fill over
    `CARRY_FILL_FLOOR`, `src/render.rs:1800`), not raw `worth / capacity`:
    capacity is 5,760 J and raw fill clusters low, so a raw swell would barely
    show. The fill distribution on the 250-ant bed is measured first, and the
    body colour no longer double-counts it (the brightness cue is dropped
    where the swell carries it).
12. **The cost is measured on the swarm, with the upload counted** (§7), and
    the upload is banded: `Layer::upload` sends the whole 2.6 MB today
    (`src/bin/lab/layer.rs:197`).

**Smaller points**

13. `flight` is set only by a launch; an ordinary fall is a walk step with no
    flight. "Legs splayed" is therefore keyed on the ant having moved down
    with nothing under it, or dropped.
14. A bite can shrink a chain to one cell (`reconcile_chain`); a one-cell ant
    takes its angle from `heading`.
15. **The sprite follows the body axis (head minus tail), not `heading`.** An
    ant that turns while blocked changes `heading` only; the sprite's
    antennae can sweep toward `heading` as a cheap "looking" cue.
16. `draw_layered` returns early when the interface is not due
    (`src/lab/mod.rs:2466`); the creature layer is painted before that
    return and carries its own repainted flag for the upload.
