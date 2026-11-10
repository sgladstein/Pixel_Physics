# The slime mould out of the dish: what the game looks like in a real world (2026-11-09)

Design exploration, not a build. Scott, 2026-11-09 03:48Z: *"explore how to
bring the slime mold into a real world out of just a dish. How would that game
look?"* Thread "Imagine the slime mould in a real world". The mould game
itself is the thread "Prototype a slime mould herded by light"; nothing in
`src/mould/` was edited.

Pictures and counts: https://claude.ai/artifact/Uda48Hzdse6pSLWWZEBGb1 (source
`Reports/mould-real-world-2026-11-09/page.html`). Instrument:
`examples/mould_world_sketch.rs`.

**Scott's standing rulings this works under.** Everything on screen must be
real simulation, no drawn-only effects (11-09); fun over fidelity; a switch
or slider for each system; the two laws (an outcome is a spread; every
system needs a verb that delivers something). **One ruling this would
reverse:** on 11-08 06:44Z he said the mould game should *not* bring in the
Pixel Physics world, plants or ants. This report is the exploration he has
now asked for; going ahead with an engine option is his call.

## Summary

1. **The engine is side-on, and side-on the mould becomes a runner along the ground.**
   Dropped unchanged into a slice of the lab box (the main game's played bed,
   aged 20,000 frames), it lives in the top of the soil and in the litter, and
   it grows **one long runner along the ground**, because nearly all the
   world's dead matter (fallen fruit, leaves, crumbs, dead ants) lies on the
   surface. It reached 4 of the 5 piles beyond home within 10 s with nobody
   touching it. That is a line, not a network, and too easy.
2. **Put food under the ground and the network comes back.** Five piles
   buried 12+ rows down (a stand-in for a dead ant, a root, an ants'
   larder) and it branches down into the soil in forks, like roots,
   reaching 9 of the 10 piles beyond home. The control (the same 60-row
   ground with no buried food) stays a runner with one loop. So in a
   side-on world the network is made by *where the world buries food*. A
   game would add the real soil's damp-below, dry-on-top profile (the
   sketch ignores moisture and light): a tension the dish does not have,
   **food is up top, water is down below, and light pushes it down**.
3. **Seen from above, a forest floor keeps the dish's network exactly** —
   logs, stones and a puddle around it, two of the seven leaf piles beyond
   home reached alone, the rest left for the player. But it is not the engine: wet,
   shade, rain and wind would all have to be simulated by the mould game
   itself (the 11-09 depth report already designed the wet field).
4. **Recommendation: build the side-on version first, as its own small
   binary on a lab-built world** ("mould under the garden"), with the engine
   doing water, light, gravity and decay, and keep the top-down forest floor
   as the fallback if side-on loses what Scott likes about the dish. Reasons
   and costs are in §4.

## 1. What was run

`examples/mould_world_sketch.rs` builds the shipped mould (`Dish::from_level_with`
at `Level::params()`, no edits) and replaces its ground with a mask:

- **`view=side`**: the lab's `played_bed` scenario, aged for `age=` frames
  (20,000 here) with `frame::step`, the scenario timeline and the mister
  (`lab::rain::tick`). That is an approximation of the played bed: the
  window's own tick (`lab::tick_world` plus `Lab::tick`) also runs tandem,
  visitors and regrouping, which the sketch skips. Candidate windows
  (every 16 columns, top edge 70 rows above the surface at the window's
  middle) are scored by food cells over the whole window; the best was
  x=240, y=90 on bed seed 1. The mould may live on
  litter, dead leaves, moss, logs, dead wood, fungus, fruit, windfall,
  corpses, crumbs, seeds and pips, on the top `skin=` rows of soil-like
  ground (soil, packed soil, spoil, grass roots, root wood, nest), and,
  with `film=N`, on the N cells of air just above the ground. Everything
  else is rim. Piles are put on the world's own food cells (dead leaf,
  fruit, windfall, fungus, corpse, crumbs, litter), at most 9, at least 24
  apart and 6 from the edge, only where 20 of the 49 cells round them are
  livable, then dropped down their column onto livable ground (+2 rows);
  `buried=N` adds N piles at livable cells at least 12 rows under the
  surface and 30 from every other pile. The window's outer ring is always
  rim (see the review note below).
- **`view=top`**: a seeded forest floor: two logs (livable bark), seven
  stones and a puddle (rim), eight leaf piles.

The picture is the engine's own renderer for the side view (noon pinned)
and a flat painted floor for the top view, with the mould's own
`render::cell_colour` written over any cell whose trail is above half the
tube threshold. It is a full replace, never a blend.

Results at 60 s of mould time (7,200 steps), one seed each. One seed is a
sample, not a distribution: these say what the shapes look like, not how
often. A rerun of the same arm gives identical counts.

"Piles reached" counts piles beyond home that had mould standing on them
at the last census (`Pile::reached`), which is not the same as joined by
fed tube.

| Arm | Livable cells | Piles beyond home reached, 10 s / 60 s | Walkers at 60 s | Tube cells at 60 s | ms per mould step* |
|---|---|---|---|---|---|
| top: forest floor | most of 40,960 | 2/7, 2/7 | 900 | 1,918 | 0.89-1.04 |
| side: 14-row soil skin | 2,840 | 1/4, 1/4 | 536 | 1,153 | 0.17-0.18 |
| side: 40-row skin + 3-cell air film | 9,704 | 4/5, 4/5 | 1,465 | 2,711 | 0.39-0.43 |
| side: 60-row skin + film (control, no buried food) | 14,824 | 4/5, 4/5 | 1,472 | 3,449 | 0.43-0.49 |
| side: 60-row skin + film + 5 buried piles | 14,824 | 9/10, 9/10 | 2,634 | 4,902 | 0.44-0.53 |
| same, sensor reach 4 (shipped 9) | 14,824 | 5/10, 9/10 | 3,144 | 5,639 | 0.40-0.56 |

Livable cells are counted before the outer ring is made rim.

\*Six runs in parallel on a cloud container, so the step times are rough.
The game runs 2 steps a drawn frame.

**Review note (2026-11-09).** The code review found that `Dish::spread`
only updates cells 1..W-2 and 1..H-2, so on the first runs, whose mask made
the window's edge columns livable, trail on the edge never faded and stood
as bright tube at x=0 and x=255. The sketch now makes the outer ring rim
and every number and picture here is from the rerun. It changed two
readings: the 40-row runner's "short drops" at the edges were that artifact,
and the reach-4 arm's "sprawl" was mostly that artifact too. The shipped
dish is an ellipse that never reaches its edge, so the game itself is not
affected.

What the pictures show (traced, by looking at each still):

- **14-row skin**: a short yellow slab at the left that stops at the
  nest's stone and never crosses it. Dull.
- **40-row skin + film**: one tube along the ground under the plants and
  nothing else. Nearly every pile on it, by itself.
- **60-row control**: the same runner, plus one loop that dips into the
  soil at the left and starves (grey). Deeper ground alone does not make a
  network.
- **Buried food**: the runner along the ground plus four thick branches
  going down to the buried piles. It reads as a root system or a mycelium
  under a garden. This is the only side-on arm that looks like a network.
- **Sensor reach 4**: the branches are visibly thinner and wander more,
  with one loop; it reaches the same 9 of 10, slower (5 at 10 s). One seed,
  so a lead for tube width at world scale, not a finding.
- **Forest floor**: identical in character to the dish: a fat bent tube
  over three piles; the other five are, I infer, beyond its 70-cell
  feeding reach (`Params::feed_reach`).

Tube width is judged by eye from the stills (about 10-14 cells at the
shipped reach); no instrument prints it.

**One more observation, not kept as an instrument:** an early draft of the
sketch ran on a freshly generated outdoor world (8192x2560, default worldgen
params, seed 7) and found almost nothing for a mould to eat near the
surface (its best window: 6 moss cells, 0 food cells). That draft's
outdoor mode was replaced by the lab box, so treat this as unverified.
The aged lab window above held 135 food cells after 20,000 frames (fruit
53, litter 50, crumbs 22 and others).

## 2. Engine facts the options hang on

All checked in code or the wiki on 2026-11-09.

- **The mould is private.** `src/mould/` runs on its own 256x160 float
  fields and touches no shared engine file (`src/mould/mod.rs` module doc).
  Any engine option is new coupling.
- **Side-on Jones sensors fail on a surface.** A walker on top of the
  ground has both side sensors in open air (`Reports/dead-ends.md`,
  *creature-direction §13b*, measured 0.000). **Inside** a solid it is a
  flat 2D region again, which is why the sketch lets it live in the soil
  and litter and only a thin film of air. The pictures confirm it works there.
- **Soil holds water per cell, seeps it and dries from the top**, settling
  to "dry at the top, damper as you go down" (`wiki/world-cycles.md`, *Wet
  ground and dry ground*). This is the vertical tension in §1.
- **Light is a coarse field**, 16x16-cell blocks (`field::FIELD_SCALE`).
  Good enough to push the mould away from a lamp, too coarse to cast a
  sharp leaf shadow.
- **The lab already has the verbs**: fixtures you place, remove and drag
  (`U`, *Creature tools status* in README), a mister that rains
  (`lab::rain::tick`), dig and drop.
- **Rich soil exists** (`Cell::richness`, `rich_earth`): ground beside the
  ants' finished fungus is enriched. An existing per-cell value a mould
  could also leave behind where it eats.
- **The outdoor world is 8192x2560**, 512 dishes. Whole-world float fields
  would be about 21 million cells each: the mould would have to live in a
  window or in chunk-bound fields like `sim::pheromone`.

## 3. Options

### A. Mould under the garden (side-on, on the real engine)

**What you see:** the lab box cut open from the side: plants and sky above,
brown soil below, and a yellow mould living in the top of the soil and the
litter, branching down to buried food like roots. At night (lamps off) it
comes up to the surface to eat what has fallen; under a lamp it is pushed
down. After the mister rains it surges; in a dry spell it pulls down to
where the soil stays damp.

**Scale and camera:** the 512x320 box, viewed whole or as a 256x160 window
that pans. At the shipped settings a tube is about 10-14 cells wide against
a 2-cell ant: the mould is a fat creature in this world. Halving the
sensor reach gave thinner branches on one seed (§1), the first knob to try.

**Verbs** (each the engine's own): drop food (it falls and lands for real);
water (soaks the real soil, the mould follows the damp); move a lamp (the
real light field pushes it, the dish's lamp with a real source); dig and
fill (open a gap, bury food so the network has to reach down). The storm
becomes the world's own: rain, an ant tunnel cutting a tube, a dry spell.

**Goals it can carry:** reach the fallen fruit before it rots or the ants
take it; keep a branch alive through a dry spell by getting it deep; feed
the garden (the mould leaves rich soil where it eats, the plants grow from
it); later, the mould and the colony share one box.

**Pros:** every effect is the real engine: Scott's real-simulation ruling
is met by construction. It gains the engine's best mechanics for free (water
moving through soil, gravity, day and night, decay). The vertical tension
(food up, water down, light pushes down) is new and only exists side-on.
It is the project's own goal: a fun sim in the lab box.

**Cons:** side-on, with food only on the surface, it is a line, not a
network, and it nearly wins alone (4 of 5 piles beyond home in 10 s). The network depends on the world
burying food. Tubes are fat at lab scale. The engine's surface trap means
it cannot crawl over things, only through them. It reverses Scott's 11-08
"not the world" ruling.

**Costs:** mould step 0.17-0.56 ms on a 256x160 window here (two steps a
frame). The whole 512x320 box is four times the cells; the cost of the
whole box is inferred, not measured, at roughly 1-2 ms a step. Reading
moisture and light once a second into the mould's own fields is cheap;
writing to the world (eating a leaf, leaving rich soil) wakes chunks and
must be batched. **If it ever enters the lab game itself**, the Teacher
lane must be told first, because anything in the box can move ant-training
results; its own binary on a lab-built world moves nothing.

### B. A forest floor from above (top-down, its own world)

**What you see:** the dish's mould, the same networks Scott liked, on a
mossy floor with fallen logs, stones and a puddle, bigger than one screen,
the camera following the front. Rain falls as drops, a dry spell spreads
from sunny gaps, canopy shade moves across with the day.

**Verbs:** today's lamp, oats, salt and snip, plus water; the depth
report's crust, fruiting and spores (`complexity.md` in the depth report, on the mould thread's branch) fit here unchanged.

**Pros:** keeps everything already tuned and liked; no surface trap; no
engine coupling, so no frame cost to any other game and no risk to the lab.
Cheapest to build.

**Cons:** the "real world" is the mould game's own simple layers, not the
engine; each one (wet, shade, rain, wind) must be built and must be real
simulation to meet Scott's rule. Bigger than one screen costs: the all-open
floor stepped at 0.89-1.04 ms here, so a four-screen floor needs the same
window or chunk trick as the outdoor world.

### C. Across the outdoor world (side-on journey, later)

**What you see:** a mould that lives in one rotting patch and, when it
starves, fruits; its spores ride the real weather wind to another patch
somewhere along the 8192-wide world, and the camera follows the new colony.

**Pros:** the biggest, most "real world" version; spores on real wind are a
lovely second life for the depth report's fruiting idea.

**Cons:** a fresh outdoor world likely has little to eat at the surface
(§1, unverified); the
world is 512 dishes in size, so the mould must live in windows; it needs A
working first. A later layer on A, not a start.

## 4. Recommendation

**A first, as a small probe in its own binary** ("mould under the garden"):
build a lab-style box, run the shipped mould in its soil with the engine
doing water, light, gravity and decay, and give the player four verbs (drop,
water, lamp, dig). No edits to `src/mould/` (the mould thread owns it) and
no shared engine edits for the first round, so nothing in the lab or
training can move.

Why A over B: it is the only option where the world is real by
construction, which is Scott's newest ruling, and the pictures say the
network survives side-on as long as food is in the ground. It also gives the
mould a place in the project's main game later, which B never can.

What would change this: if, played, side-on herding feels like steering a
rope along a line (the 40-row picture) rather than growing roots (the
buried-food picture), go to B. **The deciding measures, named before
building:** Scott's play verdict, plus two counts on the same seeds: piles
joined by a do-nothing hand against a planned hand (the dish's star test),
and the share of tube below 12 rows (a line scores near 0, roots well above
it).

**The open risk to check first:** whether the real moisture profile and the
lamp push are enough to keep it from winning alone. The 40-row arm reached 4 of 5 piles
beyond home in 10 s untouched; if that holds with the engine's water and
light on, the game needs burying and drying to be the player's job.

## Files

- `examples/mould_world_sketch.rs`: the instrument. `view=side|top`,
  `skin=`, `film=`, `buried=`, `so=`, `seeds=1`, `age=`, `world_seed=`,
  `seed=`, `seconds=`, `every=`, `out=`.
- `Reports/mould-real-world-2026-11-09/`: the page source and the
  pictures on it.
