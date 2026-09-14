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

