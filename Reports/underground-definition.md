# What "underground" means, and why it is stored rather than inferred

*Written because mining is next, and mining is the activity that breaks every
version of this that guesses.*

## The question

Empty space in this world is drawn one of two ways: as **sky**, which follows
the time of day, or as **`UNDERGROUND`**, unlit rock behind an opening. Every
cell of air has to be one or the other, so something has to answer "is this
position inside the ground". That answer is what this report is about.

It sounds like a rendering detail and is not. It decides what a mine looks
like, whether a tree casts a black rectangle, and whether widening a tunnel
floods a mountain with daylight.

## What was wrong with inferring it

Four successive rules tried to read the answer off the world as it stands.
The last of them — the one this replaced — took the topmost non-empty cell in
a column as the skyline, then repaired any column standing lower than the
ground within six columns either side, on the reasoning that such a column is
a hole and the sky should not follow a pick down it.

Measured (`render.rs`'s `probe_what_counts_as_underground`, ground at y=80,
reading at y=60 above and y=100 below):

| shape | verdict |
|---|---|
| bare ground | sky |
| one floating cell at y=40 | **20 rows of cave beneath it** |
| a 1-cell spire | same |
| a plank 1 cell wide | same |
| a plank 51 cells wide | same |
| shaft 1, 3, 7 wide | underground (correct) |
| shaft 13, 25, 51 wide | **open daylight, 35 rows into the mountain** |

Two failures, pointing opposite ways:

- **Nothing overhead was ever discriminated.** One pixel and a fifty-cell
  slab produced identical results, because the repair rule only ever filled
  *holes* and had no mirror for spikes. This is what the "shade under a tree
  is way too intense" report was actually about — a canopy is a plank.
- **The repair had a hard width threshold**, at twice its reach. A 12-cell
  shaft was a tunnel and a 13-cell shaft was a skylight. Widening a shaft is
  what mining is, so the rule broke precisely while being used, and the
  failure was a 35-row shaft of daylight rather than something subtle.

Raising the reach moves the threshold, it does not remove it — and it makes
the wrong answer wider when it comes. There is no setting that does not exist.

**The deeper reason no reach works.** Four shapes have to be told apart: a
hill, a shaft someone dug, a roof someone built, and a grain in mid-air. From
the cells alone, they are the same arrangement. The difference between "I dug
this" and "this is a hill" is *history*, and `CLAUDE.md` already has the rule
this falls under: when a rule must tell apart two things that can look
identical, state the difference as data. Support models learned it four times;
this is the same lesson in a different costume.

## The options weighed

**A — keep inferring, add the missing mirror.** Ignore a column standing much
higher than both neighbours, so a spike stops casting. A few lines, no
storage. Fixes the tree and the floating pixel; leaves the shaft threshold
exactly where it is and adds a second one for plank width. Rejected: it
buys the cheap half and leaves the half that mining will hit.

**B — freeze the ground surface, store it in the `World`.** *Chosen.* One
height per column, recorded on the world's first frame and never revised.
Digging cannot lower it, building cannot raise it, and there is no threshold
anywhere in it.

**C — a wall/background layer, Terraria-style.** A per-cell background that
worldgen fills below the surface and mining leaves behind, which the player
can also place. Strictly more expressive than B: it can tell a tunnel through
rock from a room someone built, which B cannot. It is a real feature — a
second layer in `Chunk`, plus streaming, saving, a brush and rendering — and
**B is a strict subset of it**: B is exactly "walls, auto-filled, contiguous
from the surface down, not editable". Nothing built for B has to be unbuilt to
get to C, so C is the destination and B is on the way.

## Why storing it does not repeat the reverted attempt

A stateful skyline was built here once and reverted, because it broke
`dirty_rect_skip_is_pixel_identical_to_a_full_redraw` — that test draws with
one renderer that has history and one fresh one, and compares the pixels.

**That objection is about where the state lived, not that it existed.** The
reverted version remembered the skyline *in the `Renderer`*, so a fresh
renderer could not agree with an old one. `World::sky_surface` is in the
world, which both renderers read, so they agree by construction. The test
passes unchanged.

## What it does

`World::sky_surface` is a per-column `i32`: the topmost row of ground, or
`i32::MAX` for a column with none. Frozen by `World::freeze_sky_surface` on
the first `begin_step` — the world is fully built by then, and nothing has had
a chance to dig into it or build on it, since both are things that happen
while it runs. `App::reset` builds a whole new `World`, so a regenerate cannot
leave it stale.

**Ground means `Solid` or `Powder`**, and both exclusions are load-bearing:

- `Plant` and `Creature` are things standing *in* the world. Worldgen plants
  trees before the first frame, so counting a canopy would bake the original
  bug into the one place nothing can later correct.
- `Liquid` and `Gas`, because a waterline is not a ground line and water
  levels move. Counting the top of a lake fixes the sky at frame one's
  waterline, so draining it — into a shaft, or by evaporation — leaves a band
  of false cave hanging above the new level and creeping down as it falls.
  Caught in a `viewshot mine=1` render, where mining into a lake drained it.
  Taking the rock beneath means the whole water column reads as outdoors,
  which it is, and costs nothing: a liquid cell is not empty and draws as
  itself either way.

Storage is a `Vec<i32>` over the world's width — 8 KB at 2048 wide. M10's
streaming will want it per chunk column, along with everything else currently
sized to a resident world.

## What this gives up, deliberately

**Building up no longer takes the sky down with it.** Lay a roof across a gap
and the space under it reads as outdoors rather than as a room. That was real
behaviour with a test asserting it, and it is the same mechanism that put a
black rectangle under every tree — there is no version that keeps one and
drops the other.

It is the right trade for now: what a player builds cannot accidentally
blacken the world under it, and "outdoors under a roof" is a mild wrongness
where "a cave under a floating pixel" was a loud one. Making a building read
as indoors is option C, and it should be a thing the player *places* rather
than something inferred from having put a block overhead.

## The other half: it fades

Orthogonal to the definition, and worth keeping separate in your head. Once a
cell is underground, how dark it draws ramps over `CAVE_FADE_DEPTH` (24 rows,
square-rooted so about a fifth of the drop lands in the first row) instead of
switching at the boundary. That is what makes a cave mouth read as an opening
rather than a hole cut out of the picture. It was the other half of the same
"way too intense" report and has nothing to do with where the boundary is.

## Guards

- `digging_a_shaft_does_not_bring_the_sky_down_with_it` — now swept over
  shaft widths 1, 12, 13 and 40, because the widths are the substance: 12 and
  13 straddled the old threshold.
- `building_a_roof_does_not_turn_the_air_under_it_into_a_cave` — the
  deliberate reversal, asserted so it cannot be undone by accident.
- `a_tree_does_not_turn_the_sky_behind_it_into_a_cave` — the original report.
- `draining_a_lake_does_not_leave_a_dark_band_above_it` — the waterline case.
- `the_dark_under_a_roof_fades_in_with_depth_rather_than_cutting` — written
  to fail for the *replacement* artifact too: it fails if the fade never
  reaches full dark, so lightening `UNDERGROUND` instead cannot pass it.
- `probe_what_counts_as_underground` (`#[ignore]`) — the table above, to
  re-run against any future change here.
