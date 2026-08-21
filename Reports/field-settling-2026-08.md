# The field never settles, and it costs a third of the world every frame

*Found while measuring what a 4x world costs, round 7. Not a scale problem —
a standing defect at the shipped size that scale merely makes fatal.*

## The measurement

`examples/scale_probe.rs`, built for this. Generated world, no player, no
input, timed after the CA has gone completely quiet — `active_chunk_count()`
is **0** in every row below.

| size | cells | place | structural | gen | peak RSS | settled sweep | settled field |
|---|---|---|---|---|---|---|---|
| 2048×640 | 1.31 M | 355 ms | 196 ms | **551 ms** | 37 MiB | 0.00 ms | **8.26 ms** |
| 4096×1280 | 5.24 M | 1503 | 903 | 2405 | 138 MiB | — | — |
| 6144×1920 | 11.8 M | 3415 | 2244 | 5658 | 305 MiB | — | — |
| 8192×2560 | 21.0 M | 6270 | 4433 | **10 542 ms** | **539 MiB** | 0.09 ms | **42.37 ms** |

Two things to read off it.

**Chunk sleeping works, completely.** The CA sweep on a settled 4x world —
sixteen times the cells — is **0.09 ms**. Whatever else scale costs, it is
not the falling-sand simulation.

**The field is the whole cost, and it is already the whole cost today.**
8.26 ms per frame at the shipped size, on a world where nothing is
happening, is half a 60 Hz budget spent on nothing.

## Why

`field::step`'s early-out needs `active_chunk_count() == 0 &&
fields_settled() && !amplitude_changed`. Instrumented over 3000 frames on a
quiet world, `fields_settled()` is **false on every single frame**, and the
solve set plateaus at **~118 of 320 tiles — 37% of the world — and never
reaches zero**:

```text
frame=50   tiles=320 solve=118 active=0
frame=650  tiles=320 solve=130 active=0
frame=1850 tiles=320 solve=118 active=0
frame=3050 tiles=320 solve=188 active=0     <- a sky step, expected
```

Attributing the non-convergence by channel over the last 2000 frames:

```text
1986  pressure
  14  velocity
```

**Pressure alone.** Sample cells sit around `-6.1047 -> -6.0742` frame after
frame — a drift of ~0.03 against `SETTLE_EPSILON_PRESSURE = 0.01`, forever.
Not a slow approach to equilibrium: the count is flat from frame ~650 to
frame 3000.

Every unsettled tile also drags its 8 neighbours in through the halo ring,
which is how 30-odd genuinely-drifting tiles become 118 solved ones.

## What was tried, and what it measured

**1. Quantise `sky_light_amplitude`. Kept.** Bought nothing on its own —
8.26 → 8.55 ms, inside noise — and it is kept anyway because it is a
prerequisite, not an optimisation.

The old code relied on an accident. The comment above `apply_sky_to` states
it plainly: the sun's amplitude moves less than `SETTLE_EPSILON_LIGHT` per
frame, so the `amplitude_changed` flag *written to drive the clock* "is
essentially never true", and the sky advanced instead because the pass wrote
a slightly different value every frame and its tiles therefore never
converged. So the day/night cycle was being driven by tiles failing to
sleep. Rounding the amplitude to `SKY_LIGHT_STEP = 0.01` — 0.26% of the
0.2..4.0 range, well under one step of the 8-bit colour it is drawn through,
and above the 0.005 epsilon so a step registers — makes it piecewise
constant, so the flag fires on the ~760 frames per 3600 where the sky
actually moves and the clock runs on the mechanism intended for it.

Quantised on the 0..1 daylight *fraction*, not on the amplitude, so
`daylight == 0` and `daylight == 1` stay bit-exact and
`sky_light_amplitude_cycles_between_the_night_floor_and_max_light` keeps its
`assert_eq!`s. The first attempt quantised the amplitude and moved the night
floor to 0.19999999; weakening that test to fit would have been the wrong
way round.

**2. Subset `apply_sky_to` by column. Reverted — do not retry without
re-reading the solve-set number above.** Gating each column on `sky_drifted`
measured 8.26 → 6.55 ms and was measuring a bug. A tile in the solve set is
rebuilt from a fresh `FieldTile`, so a skipped column does not keep stale
light — it **loses** it. Measured: mid-air cells going `2.43 -> 0.0` and
staying there, because the fresh tile also takes `sky_amplitude = amplitude`
and so reports no drift to ask for a repair. This is the same trap the
existing comment records ("a quiet world had no awake tiles, so no column
was written, so the light froze") wearing a different coat.

Corrected to "solved OR drifted", the subset covers the whole world anyway
and costs **8.26 → 9.06 ms**. There is no small set to subset to while
pressure keeps 37% of the world in the solve set.

## What to do next, in order

1. **Make pressure converge.** This is the fix; everything else here is
   downstream of it. Until it lands, the early-out cannot fire and no
   subsetting of the passes can pay.
2. Then re-try the sky subset (2 above), which becomes worthwhile the moment
   the solve set is small — and re-measure rather than assuming.
3. Generation cost and peak RSS are the *other* 4x blockers and are
   independent of this: 10.5 s and 539 MiB, with `structural::
   compute_world_distances` at 41% of generation and holding a second
   full-grid mirror while it runs.

## The trap this file exists to record

Both wrong turns above came from the same place: **a settled-looking world
that is not settled.** The first version of `scale_probe` reported "11 ms
settled" at the shipped size against a recorded 0.008 ms, and the disagreement
was the probe's, not the engine's — it never checked whether the world had
actually gone quiet. Reporting `active_chunk_count()` next to the timing is
what turned a confusing number into a diagnosis. Print the state a timing was
taken in, next to the timing.
