# How Terraria and Noita answer "is this dark"

*Research, commissioned directly: "explore how noita or terraria deal with
this type of issue". Written for the open half of
`Reports/dark-bands-diagnosis.md` — a pit you dig in daylight still draws as
cave, and the fix that landed cannot help it, because those cells really were
rock.*

The question both games have to answer is the one this engine has:
**empty space needs a colour, and "empty" and "open to the sky" are not the
same question.** They answer it in opposite ways, and the pair brackets the
options usefully.

## Terraria: store it per tile, then propagate light through it

Two mechanisms, and the split is the interesting part.

**1. The background wall decides whether sky light can enter at all.** From
the decompiled `Lighting.cs`, the sunlight seeding condition is:

```cs
if ((!tile.active() || !Main.tileNoSunLight[tile.type])
    && color[..] < Lighting.skyColor
    && (double)y < Main.worldSurface
    && tile.liquid < 200
    && (Main.wallLight[tile.wall] || tile.wall == 73))
```

Read the last clause: a tile receives sky light only if **its wall admits
it**. Walls are a second, per-tile layer that worldgen fills in behind the
terrain, and mining the block in front does *not* remove the wall — that
needs a hammer, as a separate action. So a tunnel you dig keeps its dirt or
stone wall, and stays dark; a room you build gets walls you placed, and reads
as a room; and space with no wall behind it is sky.

Note also `y < Main.worldSurface`: a single **scalar** depth for the whole
world, not a per-column skyline. Terraria does not try to make the boundary
follow the terrain at all.

**2. Light then propagates, so nothing is a step function.** `negLight =
0.91` and `negLight2 = 0.56` are the per-tile multipliers — through air and
through a light-stopping tile — applied over a screen-sized light map in four
directional sweeps. 0.91ⁿ falls to about a tenth in **24 tiles**, which is
worth noting: this engine's `CAVE_FADE_DEPTH` is 24, set by eye and
independently.

What that buys is exactly the case still open here. A wide pit dug into a
hillside has walls behind it, so it is not "outdoors" — but sunlight floods in
from the open top and attenuates with distance, so the pit is *bright near
the surface and dark at the bottom*, with no threshold anywhere and no
classification of the pit as a whole. The shape of the hole does the work.

## Noita: do not classify at all — cover the world and let light reveal it

Noita has no wall layer. Its world is per-pixel material over a biome
background image, and darkness is a **fog of war**: a 4096×4096 byte array,
**one byte per 32×32 pixel block**, over a 131072×131072-pixel world, centred
on the origin and never resized. It is composited by the final full-screen
shader (`post_final.frag`), and lights are emissive discs that clear it.

Two details are directly relevant here:

- **The fog is coarse and deliberately blurred.** It was changed to draw as
  overlapping tiles blending into continuous soft shadow, because as plain
  tiles it read as a visible grid with seams. That is
  `Reports/open-bugs-handoff.md` §0c almost word for word — this engine's
  cave light was quantised to 8-cell squares and read as rectangles of rock —
  and it is the answer: a coarse field is fine if you blur it, and reads as
  blocks if you do not.
- **Nothing decides whether a place is "outdoors".** Everything is dark
  until something lights it. The question this report is about does not
  arise, because Noita never asks it.

Noita's model is not available here for free: the dark-until-lit answer only
works because the player carries a light and the whole game is underground.
This world has a day, a sky and a surface to stand on.

## What this says about the case still open

The landed fix (per-cell "was this enclosed at genesis") is the **wall layer's
first bit**: a wall that worldgen places, that mining does not remove, that
the player cannot yet place themselves. That is precisely the option C
`Reports/underground-definition.md` already named as the destination, and
Terraria confirms the shape is right — including the part that says a dug
tunnel *should* stay dark, which is the behaviour the owner explicitly asked
to keep ("if you dig out a tunnel underground it looked like sky behind it").

What the pit case needs is Terraria's **second** mechanism, not more of the
first. No per-cell classification can make a 64-wide pit bright and a 3-wide
shaft dark, because the difference between them is not what the cells *were*
— it is how much sky they can see now. That is a propagation question, and
propagation has no threshold in it: the pit is bright because sky reaches it
from above, and the shaft is dark because 24 rows of 0.91 is a tenth.

**This engine already owns the mechanism.** `field.rs`'s light channel does
Beer-Lambert attenuation through occluders — *"clear air does not attenuate
sunlight, occluders do"* — and plants have read it since M16 for shade death
and phototropism. Nothing draws it. What stands between it and the screen:

- it is at `FIELD_SCALE` = 8, one value per 8×8 cells, and §0c already
  recorded what that looks like rendered directly — which is the same wall
  Noita hit and solved by blurring rather than by refining;
- it oscillates 20:1 over the day/night cycle by design, so any *decision*
  taken on it has to go through `field::noon_equivalent_light` (`CLAUDE.md`);
- the per-pixel read has to stay cheap enough not to cost the dirty-rect
  skip — the constraint that killed fake AO at ~10 ms/frame.

None of those is a reason it cannot be done; they are the three things a
proposal has to answer. The honest summary is that the pit case is a
*rendering* feature — draw the sky-visibility field this engine already
computes — and not another classification rule, and that trying to solve it
with a better boolean is the shape of mistake `dead-ends.md` §977 records four
times.

## Sources

- Terraria `Lighting.cs`, decompiled:
  [TheVamp/Terraria-Source-Code](https://github.com/TheVamp/Terraria-Source-Code/blob/master/Terraria/Lighting.cs)
  — the sunlight seeding condition and `negLight`/`negLight2`.
- [Background walls — Terraria Wiki](https://terraria.wiki.gg/wiki/Background_walls)
  and [Placement Layers](https://terraria-archive.fandom.com/wiki/Placement_Layers)
  — the wall layer, natural wall generation, hammer removal.
- [Modding: Fog of War — Noita Wiki](https://noita.wiki.gg/wiki/Modding:_Fog_of_War)
  — the 4096×4096 byte array, 32×32 blocks, and the shader compositing.
- [Exploring the Tech and Design of 'Noita'](https://www.gdcvault.com/play/1025695/Exploring-the-Tech-and-Design),
  GDC 2019 — the Falling Everything engine; background for the per-pixel
  world, though it does not cover lighting.

*Freshness: 2026-08-23.*
