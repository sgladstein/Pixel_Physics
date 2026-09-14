# Lane A, round 36 — the food economy, made readable

*Branch `claude/evolution-lab-food-readable`. Brief:
[`../evolution-lab-round-36-brief-2026-09-14.md`](../evolution-lab-round-36-brief-2026-09-14.md)
§"Lane A". What round 35 shipped is
[§4 and §5 of its record](../evolution-lab-round-35-2026-09-14.md).*

**The three verdicts this lane exists to answer**, in his words:

| card | his words |
|---|---|
| `…063034460Z-989679` road + harvest, planted bed | *"No i cannot really tell. the amber hatch it bad. is it too much to track actual paths and make trail?"* |
| `…063101470Z-06554c` road + harvest, far larder | *"Why is the amber hatch drop like a huge box?"* |
| `…063026275Z-e7b8f3` the FOOD page | *"Nope. I don't understand what these visuals are trying to tell"* |
| `…063118804Z-711e5a` empty-handed walking | *"both is interesting"* — keep both arms, settled |

---

## 1. Reproducing his reading, before changing anything

The brief's instruction, and it paid immediately: **render the two channels
separately.** Four arms, one bed, one seed, one frame — `played_bed` seed 1,
warmed 12,000 frames and captured at 17,600, mode swapped between runs and
nothing else:

```
./target/release/examples/foodroad scenario=played_bed seed=1 \
    start=12000 frames=5600 every=5600 mode={off|road|harvest|both} \
    png_dir=… up=4 crop=120,130,256,60
```

Identical world in all four (the overlay only reads), so the pictures differ
by exactly the thing under test. Counters, identical across the three live
arms and the number that says it fired at all:

```
laden steps 622 | unladen steps 657 | harvest bites 980 | face value taken 141,720
365 road cells, 36 harvest tiles held at the captured frame
```

**Finding 1 — the road works, and the hatch was burying it.** On the `road`
arm alone the surface carries a legible red-orange line with blue stretches
running along it: *carried this way, walked back that way*. On the `both` arm
that same line is underneath an amber checkerboard **in an adjacent hue**, and
it is gone. Nothing had to be built to see this; it is what the brief
suspected, and it means a large part of the repair is **what to draw**.

**Finding 2 — the hatch paints empty sky, and that is the "huge box".** Every
one of the 36 live harvest tiles sits at tile row `y=156` or `y=164` — the
surface band. A harvest tile is 8 cells square; at the surface that is about
one row of ground and **seven rows of air above it**. The wash covers the
whole tile, so what is drawn is a rectangle whose top edge is the *tile grid*
and not the ground: a hard-edged 8x8 block hanging over the terrain, brightest
tiles nearly solid at `HARVEST_MAX_COVER = 0.8`. Dropped like a huge box is a
literal description of it.

*(Kept here as a measurement rather than a guess: the tile rows come from
`foodroad`'s own `SOURCES` line, which prints world coordinates.)*

## 2. The repair: the wash marks the skin of the ground, and nothing else

Two clips, and they are one change — **either alone leaves a box.**

- `FoodRoad::harvest_on_ground` refuses **open air**. That removes the straight
  top edge hanging over the terrain.
- `food_road::near_open_air` refuses ground more than `HARVEST_SKIN` = 3 cells
  under the exposed face. Without it the block is simply anchored to the
  *bottom* of the tile instead of the top — the same box upside down, which is
  what the air-only clip actually drew on `far_larder`, where the surface is
  bare soil and the tile's lower half is solid.

`HARVEST_MAX_COVER` went 0.8 → 1.0 in the same change and for the same reason:
the fifth held back existed so a tile the colony lives off still showed the
ground *underneath* it, and clipped to the skin the wash no longer covers what
it points at. Holding it back only made the strongest patch in the bed read as
speckle. `ramp`'s floor keeps the middle — the quietest live patch is still 18%
dots.

**What was ruled out by measurement, not by argument:** *shrinking the tile*.
At `tile=4` the sheet still draws boxes in the sky, only smaller; at `tile=1`
the channel vanishes into the road entirely and says nothing. **The box is the
air, not the size** — which is why the fix is a clip and `tile` stays at 8,
where the region reading lives.

Guard:
`render::tests::the_harvest_wash_marks_the_skin_of_the_ground_and_not_the_sky_over_it`
— sky unchanged, skin changed, buried ground unchanged, and a fourth arm that
**puts the fault back** (`harvest_on_ground = false`) and requires the sky pixel
to move, so the first three cannot be green because the wash never fired.

**Card `20260914T193144118Z-d8e147`** — *"The amber hatch, no longer a box"*,
posted 2026-09-14 19:31Z, before/after frame sequences on the planted bed.
Verdict: *pending*.

