# The World

*Current as of: this build.*

Every world is generated. There is no single fixed map: the world you get is
built from a **seed** — a number — and a **preset**, which is a named style of
landscape. The same seed and preset always rebuild exactly the same world,
down to the last grain, so a world worth keeping can be written down and
returned to.

The seed and preset are shown in the title bar at all times, and there are
three keys for moving between worlds:

- **F6** builds a brand-new world from a fresh seed.
- **F8** goes back to the previous seed, for when you roll past one you liked.
- **F7** switches preset, keeping the same seed — so you can see what the
  *same* underlying land looks like as a canyon rather than as rolling hills.

**R** rebuilds the world you are already in, discarding anything you have
done to it.

## What a world is made of

Looking at a world from the side, it is a cross-section — you are seeing the
ground the way a cut bank or a quarry face shows it, not the way a map does.

**The surface** always has real shape to it. Every world contains at least one
substantial ridge and one valley, rather than a flat line with bumps, so there
is always high ground and low ground within reach. On top of that sit smaller
hills, and finer roughness over those.

**Bluffs and benches** appear in patches: flat steps cut into a hillside with
steeper faces between them. They are not scattered arbitrarily — they follow
the rock layering, so a bench forms where a band of rock reaches the surface,
and its edge lines up with the banding visible in the face below it.

**Overhangs** form at the top of some cliffs: a lip of rock that juts out over
the drop, with open air beneath it. It is holding itself up in the ordinary
way, not by special dispensation — hit it hard enough and it will come down
like anything else.

**Scree** heaps at the foot of cliffs: loose gravel, sloped the way shed rock
naturally piles. It is real loose material, so digging into it makes it run.

**Soil** lies over the rock wherever the ground is gentle enough to hold it,
thinning as the ground steepens and giving out entirely on steep faces, which
is why cliffs read as bare rock. It also thins near a drop, so the lip of a
bench is barer than its middle, and where the cover has worn to almost
nothing the rock breaks through it altogether in patches. Where a valley floor
is flat, the top of the soil turns to sand, as washed sediment would collect
there.

Dig into soil and it has a **profile**: dark at the top where it is richest,
paler further down, and its base is not a clean line but a gradation — earth
with stones in it, getting stonier until it is simply rock. That profile is
only visible in a cross-section, which is the one thing this view has that a
map does not.

**Rock** carries visible layering — beds of slightly different tone, tilted
and gently folded, running through the whole massif. The beds vary in
thickness and there is no repeating pattern to them: a distinctly pale or dark
layer can be followed from one cliff face to another across the world. Every
cut you make, every tunnel and every blast crater exposes it, so the inside of
a hill looks like rock rather than like fill.

**Buried pockets** of sand and gravel sit sealed inside the rock. You will not
see them from outside; you find them by digging, and they pour once opened.

**Bedrock** lines the bottom of the world. It is the thing everything else is
ultimately anchored to.

## Worlds arrive settled

A newly generated world is already at rest. Nothing slumps, avalanches or
collapses on its own when you arrive — if something moves, something moved it.
That is deliberate, and it is why soil stops where slopes get steep and why
scree only heaps where there is level ground beneath it to heap on.

## The presets

- **Rolling** — the reference landscape: a ridge, a valley, benched bluffs in
  patches, and a good depth of soil.
- **Terraced** — benched country, with steps over most of the relief rather
  than in occasional patches.
- **Canyon** — deep relief and big vertical faces, thin soil, tall scree.
- **Wetland** — low, gentle, deep-soiled ground.
- **Arid** — high relief, thin soil, plenty of scree and buried pockets, and
  no water anywhere.
- **Legacy** — the old hand-built practice terrain: a flat floor and three
  ledges. Not a landscape, but a known shape, useful for comparing against.

## What is not here yet

Rivers, lakes and springs, caves, and plant cover that arrives with the world
rather than being planted by hand. Worlds are also currently a fixed size —
one screen wide — rather than continuing as you travel.
