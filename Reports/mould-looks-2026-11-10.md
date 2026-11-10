# The mould game's looks pass (2026-11-10)

Before/after page: https://claude.ai/artifact/8qWbqcMcrdHKWEz9yjgvVR

Owner, 2026-11-10: "Looks too" (after the independent review,
[`mould-review-2026-11-10.md`](mould-review-2026-11-10.md)), then "Not sure
look alive but look good... improve where the graphics look pixelated. I know
this is a pixel simulation but we can still make things look better on top of
that", and "Those suggestions are in addition to everything else that was sent
to you. Not instead of".

Drawing only. Nothing in the simulation changed: every picture below is the
same dish, at the same step, drawn two ways. Everything drawn is something the
dish holds (owner, 2026-11-09: "Everything we're seeing should be based on
real pixel simulations"). Finer tubes (sensor reach) change how the mould
behaves, so they stay with the mould thread.

## What changed, all on by default, each with a key

| Change | What it draws from | Key / switch |
|---|---|---|
| **A speck per walker** | `Dish::walkers`, every walker where it is, every frame. Grains (darker fleck) by default; with `flow`, a laden walker is lighter than an empty one (`Walker::food`) | F4: grains, glints, off; `look::WALKER_LOOK` |
| **One colour ramp for food** | With `flow`: the walkers' average load and the sap in the tube (`look::food_level`), on a log scale; without: the census's graded hunger (`fed_at`). White-gold rich, gold, amber, dull brown empty; thickness moves the tube up the ramp and brightens it | F3 now starts on it (then grey, embers, dried); `look::STARVE_LOOK` = 3 |
| **Smoothing** | The rim drawn per pixel from its ellipse, anti-aliased (4 samples a pixel); tube insides blended between cell centres instead of 4x4 blocks; the food amount read between centres; salt walls and snip cuts drawn from a softened mark field with pixel-sized salt grains | F5; `look::SMOOTH` |
| **Stain where mould has been** | Cells that were tube keep a faint film, half-life 20 s of dish time, covered by salt. Drawn from the trail each frame; the walkers do not read it | F6; `look::STAIN` |
| **The lamp as light** | The same profile the simulation shines (`Dish::update_light`: flat core to half the reach, then a slope), smoothstepped, with a faint scatter; the hard bright ring is now a faint thin one | none (look B / the game's look only) |
| **Narrow bar** | Below 950 px wide the four tool tiles are 92 px, not 142, so the storm line and the mould count no longer print over each other (scale 3) or over the SALT tile (scale 2) | automatic |

## Numbers

Seed 1, the scripted plan hand (`play::Plan`), `examples/mouldlooks.rs`
(`tools=1` adds a salt wall at 25 s, a snip at 35 s and the lamp from 44 s).

**Why the food ramp needed rescaling.** With `flow` on, the old reading
(`Dish::carried_look`, clamped at 1) was full on over more than half the tube
at every sample, 10-60 s: the "flat bright yellow" the review saw. The sap
alone spans 0.4 to 9 `SAP_FULL`s over tube cells (p10 to p99), the walkers'
average load 0.0 to 1.0. Read on a log scale topping out at 6, the food level
on tube cells is p10 0.22-0.31, p50 0.44-0.58, p90 0.76-0.93: the whole ramp
is used.

**Walkers.** 850-1,800 walkers on 3,000-8,400 tube cells: 0.21-0.33 a cell,
crowded at the piles. None off the tube at any sample.

**Stain.** At 22.5 degrees by 60 s the stain covers 15,875 cells, 39% of the
dish, so it fades. Blending it pixel by pixel cost 2 ms a frame there; it is
softened (3x3 mean) and drawn a cell at a time instead.

**Frame cost** (draw only, scale 4, same dish states, both arms in one run;
`before` is the switches off, which drew within 0.1 ms of the unchanged code):

| Dish | Before | After | Extra |
|---|---|---|---|
| 45 degrees, 40-43 s, 91 frames | 4.33 ms | 6.68 ms | +2.35 ms |
| 22.5 degrees, 60-63 s, 91 frames | 4.70 ms | 7.36 ms | +2.66 ms |
| flow on, 40 s, 1 frame | 4.33 ms | 5.95 ms | +1.62 ms |

About half is the food ramp and grains (the `rough` arm: +0.9 to +1.1 ms),
half the smoothing and stain. The window's frame is 16.7 ms; the simulation
takes 1.4-3.3 ms of it (`mouldprobe`). The first build of the smoothing drew every
tube pixel in full and doubled the look's draw (about 4 to 9 ms); tube
insides are now a blend of the four nearest cell-centre colours, with
integer weights.

## Tried and changed

- Food ramp position from the amount alone: every tube one flat tone; thickness
  now moves the colour as well as the light.
- `FOOD_TOP` 8 with thickness scaling: flow-on tubes went brown all over;
  6, with the ramp position `full * (0.6 + 0.4 v)`.
- Salt and cuts rounded inside their own cells only: the outline barely moved
  (only corners trimmed); now a softened 3x3 mark field read between centres,
  edge at 0.3, so a one-cell line still shows.
- Stain blended per pixel: 2 ms a frame at 22.5 degrees (above).

## What this does not show

- One seed. The looks are judged by eye on the page; no count can say whether
  they look better.
- Glints against grains is put to the owner blind on the page. Key: A is
  grains (the default), B is glints.
- Not judged on a real screen at the owner's window size, or for motion
  sickness from the shimmer of 1,800 specks.
