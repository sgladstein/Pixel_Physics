# The lamps light the bed

*2026-08-30. Branch `claude/lab-lamps-light-the-bed`. Built and measured, not
merely proposed — everything below is a reading off the shipped scene.*

Owner, 2026-08-30: *"lamps should be what lights the plants, or is there a
reason that they should not. **It would be fun to adjust plant growth by moving
lights.**"*

There was no reason they should not, and making it true also closes a fiction
the design of record already asked for.
[`evolution-lab-design-guide-2026-08-30.md`](evolution-lab-design-guide-2026-08-30.md)
§2: **"The lab has a ceiling, not a sky."** It had both, and the sky was
winning.

## 1. What was actually lighting the crop

**The fixtures contributed nothing, and this is measured twice by two lanes.**
`labshot lamps=0` replaces every fixture with plain stone; the stand came back
**byte-identical at every stop**, bench light included.

What lit the bed was the roof leaking. `field::apply_sky_to` casts daylight
down each CA column through `SKY_TRANSMISSION^(depth / FIELD_SCALE)`; the lab's
ceiling is 4 rows, so `depth/FIELD_SCALE = 0.5` and `COLUMN_TRANSMISSION[4]` is
**0.447214** — which is exactly the 0.447 the bench measured in all 48 runs of
the bed lane's sweep. The arithmetic closes: that was the whole light budget.

The fixtures could not reach because they were `crystal`, and `Material::glow`
seeds the light channel of the emitter's **own field block** and then relies on
`LIGHT_DECAY` (0.95) to spread it. At `LIGHT_DIFFUSION_RATE` 0.3 that is a
decay length of about 2.4 blocks — `field.rs` calls it "a handful", which is
right for a geode read across a cavern. The bench is **nineteen blocks** below
the ceiling.

So the picture said grow lights and the physics said sunshine through the roof,
and the shell's thickness was the crop's light knob: 4 rows to 7 took the bench
from 0.40 to **0.22** of `MAX_LIGHT` and the stand from 474 plant cells to
**286**, seed set 12 to **0**, with nothing failing and no test going red.

## 2. What was built

### The shell stops being the light source: `World::set_sky_lighting(false)`

Three ways to make a ceiling opaque were available — thicken it, change its
material, or say the box has no sky. **The flag is the cheapest and the only
one that is not the same fiction dimmer.** Thickening is what the 4→7
measurement above already priced. An opaque material still pays the whole
descent to arrive at zero. The flag makes the sun's amplitude zero at the top
of the world, so the descent starts dark, every `*c <= 0.0` early-out fires on
the first block, and the only thing left writing light is a lamp.

It is deliberately **not** folded into `World::enclosure`, which is documented
as read by nothing in the simulation and is set by three render tests that want
a room drawn without their world going dark.

It is scoped to **light**. Sky *temperature* still reaches the box through the
shell, attenuated by the same per-column data; a sealed room under a rock roof
still feels the day, it just does not see it.

### The fixtures become the light source: `Material::beam`

`beam` rides **the sun's own column descent** rather than diffusion. A block
that beams re-seeds the amplitude falling down its column, so clear air passes
it undimmed and only occluders stop it — a canopy shades what is under it
exactly as it shades sunlight. This is the whole mechanism; there is no second
pass and no new walk.

Two decisions inside it are load-bearing:

- **Attenuate first, re-seed second.** The fixture emits from its face, not
  through its housing, so a lamp recessed into a ceiling throws what it throws
  however thick the ceiling is. The other order re-couples the crop's light to
  the shell — the knob this change exists to remove.
  `a_fixture_does_not_shade_its_own_light` is the guard, and injecting the
  reversed order takes its encased arm to roughly a fifth of its bare one.
- **A block's beam is the mean over its CA columns, not the max over its
  cells.** §4 is entirely about why.

Cost is a `bool` per tile. `FieldTile::has_beam` gates the beam-aware loop, so
every tile in the outdoor game takes the original descent verbatim.

### A new material rather than a field on `crystal`

Giving `crystal` a beam would light a shaft under every geode in the massif and
put a germination gate in every cave — a change to the outdoor game made in
passing. `growlamp` also has **no `glow`**, deliberately: `render.rs` splats a
radius-14 disc per glowing *cell* into `near_glow`, a bar is 60 cells against a
geode's static lining, and `flame.ron` records that same cost argument sinking
fire as a light source. The interior renderer already paints the pool from
`Enclosure::with_lamps`.

### One fixture per plant station

`lamp_spacing` moves 128 → 64, and `lamp_columns` now uses the **same `spread`
the founders use**. Two independent even spacings across one bed interleave
rather than coincide: at eight of each the old midpoint formula put every
fixture 3 to 25 columns off a founder. That did not matter while the fixtures
lit nothing and decides the stand now that they do. A working grow room is the
default; making it *not* work by dragging a fixture off its bed is the mechanic.

## 3. What it does to the box

`labshot`, 512x320, 8 herb founders, one colony, same seed:

| | bench light (mean) | dimmest station | plant cells | orgs | seeds |
|---|---|---|---|---|---|
| leaky roof, inert fixtures (before) | 0.372 | 0.219 | 474 | 65 | 12 |
| **sunless, lamps (after)** | **0.421** | **0.219** | **595** | **74** | **23** |

Frame 3,600, **both arms measured on this branch's head**, off one binary. That
matters more than it sounds: `main` moved the soil and then the ants' satiety
gate inside the window this branch was open, and each time it moved the *stand*
in both arms. A before/after where only the treatment side is current is not a
before/after, so the pair has been re-taken on every merge. The granularity
sweep in §4 has come back byte-identical each time, which is the expected
answer — it is a property of the emitter and the field, and neither the soil
nor the ants is in either.

The stand is **26% larger and sets 1.9x the seed**, which is the
light-as-a-lever payoff the design guide already priced from the other side
(1,037 seeds against 435 at full amplitude, for 12% more cost). `beam: 2.4` was
chosen to land the bench a little above the 0.42 the leak used to give it, so
that switching the fiction did not quietly shrink the crop.

The mean above is the founders' mean, and it hides the half that is the
mechanic. **Drag one fixture 84 columns into the next bay and its founder
dies** — that column's bench light goes 0.219 → **0.002**, the bed goes 595
cells to 530, and nothing else in the scene is touched. Light is a place now,
not a level.

## 4. Granularity — the constraint, measured at both `FIELD_SCALE` 8 and 16

Light lives on the coarse field, so a lamp's influence is quantised to a block.
A mechanic that only responds in block-sized jumps is one a player calls
broken: you drag it, nothing happens, you drag it further, it lurches.

`examples/lamp_probe.rs` sweeps one fixture's column **one cell at a time** and
reports the bench light's **centroid** — the quantity that answers the
question, because a block-quantised emitter's centroid sits still and then
steps while a continuous one tracks the fixture.

### At `FIELD_SCALE` 8, as shipped: no dead cell

| offset | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| centroid | 255.93 | 256.42 | 256.99 | 259.26 | 259.74 | 262.01 | 262.58 | 263.07 | 263.93 |
| step | — | 0.49 | 0.57 | 2.27 | 0.47 | 2.27 | 0.57 | 0.49 | 0.86 |

**Every one of 32 columns moves the pool**, minimum step 0.472, and the
centroid advances exactly 8.000 over 8 cells — it tracks the fixture 1:1 on
average, with a period-8 wobble in the *rate*. Peak brightness is flat at 0.600
of `MAX_LIGHT` throughout.

### The positive control, which is what makes that reading mean anything

`LAMP_BLOCK_QUANTISED=1` puts the `glow` convention back — a block's beam is
the max over its cells rather than the mean over its columns — and changes
nothing else:

| offset | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| centroid | 255.5 | 259.5 | 259.5 | 259.5 | 259.5 | 259.5 | 259.5 | 263.5 | 263.5 | 267.5 |

Six consecutive dead columns, then a 4.0 jump. So the instrument can see
quantisation, and the averaged emitter is what removes it — this is
`CLAUDE.md`'s *put the fault back and watch it go red*, applied to a
measurement rather than to a guard.

### At `FIELD_SCALE` 16, with the fixture unchanged: it steps

| offset | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| centroid | 263.03 | 263.50 | 263.50 | 263.50 | 263.50 | 263.50 | 263.50 | 263.50 | 263.50 | 263.50 | 263.50 | 263.97 | 268.92 |
| peak | 0.375 | 0.412 | 0.450 | 0.487 | 0.525 | 0.562 | 0.562 | 0.525 | 0.487 | 0.450 | 0.412 | 0.375 | 0.337 |

**Ten dead columns, and then a 4.9-cell jump** — and it *dims* on the way, 0.562
down to 0.412, which is worse than doing nothing. **The cause is not the
emitter model.** It is that a 15-cell bar fits *inside* a 16-cell block: the
whole fixture lives in one block-column, so sliding it changes how much of that
column it covers and nothing else.

### The fix, measured, and it is one line

**A fixture must never be narrower than a light block.** `LAMP_HALF` becomes
`max(7, FIELD_SCALE - 1)` — 7 and therefore bit-identical at today's
`FIELD_SCALE` of 8, and 15 at 16. The same sweep at 16 with the wider bar:

| offset | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| step | — | 0.444 | 0.475 | 0.509 | 0.546 | 0.588 | 4.065 | 0.484 |

**Every one of 32 columns moves the pool again**, minimum step 0.444, peak
pinned flat at 0.600, centroid advancing exactly 16.000 over 16 cells. The
dead zone closes outright.

**So the recommendation to the `FIELD_SCALE` 16 lane is: take it, and widen the
fixture in the same change.** Nothing else in this work needs to move. The
interpolate-the-emitter-across-blocks candidate — the obvious one, mirroring
`field_at_bilinear` — was not needed and was not built: it requires
cross-chunk writes inside `rebuild_blocked`, which is per-chunk parallel, and
the one-line geometric fix measures identically.

### What still ripples, honestly

The pool's **total** flux varies about ±11% with sub-block phase (51.7 to 64.6
in the sweep's own units, period 8) while the peak and the centroid are clean.
It comes from the `max` in the descent rather than from the averaging: a source
spread across three block-columns and one packed into two emit the same total
but combine differently with the diffusive tail. It is a brightness ripple on a
drag, not a position error, and it is left standing.

## 5. Frame cost, with the census beside it

`lamp_probe mode=cost`, two arms off one binary, **alternating** within each
rep, 3,600 frames each. `roof` is the lab as it was (sunlit, fixtures replaced
by stone); `lamps` is the lab as it ships.

| box | roof | lamps | delta | stand, roof → lamps |
|---|---|---|---|---|
| planted (8 founders, 1 colony) | 2.570 ms | 3.387 ms | **+0.817 ms (+31.8%)**, dearer in 3 of 3 | 474 → 595 cells, 12 → 23 seeds |
| **empty (the machinery alone)** | 0.016 ms | **0.017 ms** | **+0.001 ms** | 0 → 0 |

Every row carries the bench light too (`roof` 0.372, `lamps` 0.421), for the
reason `labbox_cost`'s `floor` arm exists: a cost table with no light in it
cannot say whether the cheaper arm was simply darker.

**The light model is free; what costs is the biosphere it grew.** The
empty-box arm is the control that separates them, and it is necessary rather
than tidy — without it the honest headline is "+30% of the lab's frame", which
is true and about the wrong thing.

Note what the planted delta is *not*, because the obvious account does not
close: 121 more plant cells at the ~0.7 µs per cell per tick this box charges
is 0.08 ms, against 0.82 ms measured. That is expected rather than a
discrepancy —
[`evolution-lab-gate-1-2026-08-30.md`](evolution-lab-gate-1-2026-08-30.md)'s
own finding is that **frame cost in the lab tracks tiles solved (r = +0.90)
and not plant cells (r = −0.02)**, so what a bigger, busier stand buys is a
larger awake set, which is exactly the term that moves. Reported this way
because `CLAUDE.md`'s rule cuts both directions: a cost that vanishes may be
work that vanished, and a cost that *grows* may be work that was bought.

Worst-frame figures are not quoted. The ratio test says they should not be:
mean × frames is three orders above the worst here, so the worst is an order
statistic over thousands of comparable frames rather than an aggregate-pinned
rare event.

**The outdoor game is untouched.** `has_beam` is false on every tile in it, so
the descent takes its original loop verbatim; `examples/ascii` runs 31 scenes,
0 skipped, and the field suite is green. The one thing that did move for every
world is `field::field_hash`, which now covers the beam array — hashing a
constant still changes a digest, so a hash from before this change does not
compare with one from after. That is recorded at the function.

## 6. The API the parameters-panel lane should call

Placing and dragging is UI and belongs to that lane. This is what a lamp *at a
position* needs, all on `LabBox`, all in `src/lab/scene.rs`:

```rust
spec.lamps_in(&world)             -> Vec<i32>   // where the fixtures are now
spec.lamp_near(&world, x)         -> Option<i32> // the one a click at x picks up
spec.move_lamp(&mut world, from, to) -> bool     // drag; false = refused at the wall
spec.remove_lamp(&mut world, cx)  -> bool        // uninstall
spec.lamp_rows()                  -> Range<i32>  // the rows a fixture occupies
```

Four things worth knowing at the call site:

- **`lamps_in` reads the world, not the spec.** The spec says where the builder
  put them; only the world knows where they are now.
- **`move_lamp` is the whole mechanic in one call.** The light follows on the
  next field step: writing cells wakes the chunk, and `rebuild_blocked` gathers
  `beam` in the scan it already runs.
- **It refuses rather than clamps** when the bar would leave the shell, so a
  drag can be attempted every frame.
- **It re-points `Enclosure::lamps` at where the fixtures actually are.** That
  is not housekeeping: the builder sets that list once, and without the resync
  a moved fixture beams from its new column while the room stays *drawn* lit
  under its old one — this document's own opening defect, reintroduced by the
  fix for it.

Sub-`FIELD_SCALE` drags are real (§4), so the panel does not need to snap.

## 7. What is not done

**The pool is on the back wall, not on the ground.** The bench light is real
and the plants respond to it, but `render.rs`'s field-light read (`glow_at`) is
gated on `glow_tiles`, which is a glowing tile plus its 3x3 neighbours — so
nothing samples the field nineteen blocks under a fixture and the bright patch
on screen is `sky::Interior`'s backdrop bloom. Ungating it for a beam-lit
column is a small change in `render.rs`, which the sky-light lane is rewriting
this week (722 lines), so it is left to whoever lands second. Posted to the
owner as a question rather than decided here.

## 8. Relation to the other two lanes

- **`claude/lab-skylight-cost`** is entirely in `render.rs`; there is no file
  overlap. Its finding — 2.8 ms of a 4.78 ms draw spent lighting a room with a
  ceiling — points the same way from the other end, and `set_sky_lighting(false)`
  is now a *world-level* statement that the room has no sun, which its
  render-side scan may be able to read as a fast path rather than deriving.
  Complementary, not overlapping.
- **The `FIELD_SCALE` 16 lane** has §4's answer and the one-line fix; it should
  take `LAMP_HALF = max(7, FIELD_SCALE - 1)` along with the constant.
