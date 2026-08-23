# Dark bands under overhangs, objects and digs

*Written from a play report: "dark bands under any overhangs or objects or
when I'm mining. I'm not sure if this is just the background issue that we set
the background to be the baseline at world generation or if it's a lighting
shadow issue."*

**Status: the overhang and object cases are fixed and shipped; the open-cast
dig is not, deliberately, and its prior art is in
[`prior-art-underground-lighting.md`](prior-art-underground-lighting.md).**
The measurements below are the ones the fix was built from and are left as
taken — pre-fix — because they are what the guards have to reproduce.

**Short answer: the first one.** It is the background baseline set at world
generation. Nothing that shades the terrain on screen knows what is above it.

## The two candidates, and why one is eliminated outright

`sky::apply_light(rgb, level, ambient)` takes a **scalar** light level, held
once per frame in `Renderer::daylight`, and applies it to every cell on the
screen identically. It has no position argument, no occluder, no direction.
The one renderer mechanism that ever looked at a cell's surroundings — the
fake AO experiment — was measured at ~10 ms/frame on the 512x320 stress scene
and cut before shipping; the note recording that is still in `cell_colour`.

**The engine does own a real occlusion model, and it is worth being precise
that it is not this one.** `field.rs`'s light channel does Beer-Lambert
attenuation through occluders — "clear air does not attenuate sunlight,
occluders do" — and it is what plants read for shade death and phototropism.
The renderer touches it in exactly one place: `glow_at`, gated to
`glow_tiles`, the chunks that contain a glowing material. It never shades
ordinary terrain. So a real shadow exists in the simulation, at `FIELD_SCALE`
= 8 (one value per 8x8 cells), and none of it reaches the screen — which is
relevant later, because it is the raw material a genuine fix could use.

What is drawing the bands is `World::sky_surface` and the two renderer paths
that read it.

## The mechanism

`World::freeze_sky_surface` records, once on the world's first `begin_step`,
the topmost `Solid` or `Powder` row in each column. Everything at or below
that row **in that column** is "inside the ground" for the life of the world,
and nothing revises it. `Reports/underground-definition.md` has why it is
stored rather than inferred, and that reasoning stands — this report does not
reopen it.

Two consumers read it, and both are involved:

| consumer | applies to | effect |
|---|---|---|
| `Renderer::background_at` | materially empty cells | fades toward `UNDERGROUND` (31,29,33) over `CAVE_FADE_DEPTH` = 24 rows on a `sqrt` ramp, **saturating there** — no sky colour left at all |
| `TerrainLight::Depth` | everything non-empty — rock, soil, **and water** | multiplies down toward `DEPTH_LIGHT_FLOOR` = 0.62 over 64 rows, off `light_datum` (the notch-clipped skyline) |

The defect is in the *shape* of the question, not in either ramp. "Is there
anything solid above me in this column, as of frame one" cannot distinguish:

- a cave roof from **a cliff brow** — so the open air outside an overhanging
  lip is drawn as the inside of a cave;
- a hillside from **a rock suspended in mid-air at genesis** — so a floating
  slab casts a hard-edged column of cave to the bottom of the world;
- rock you removed from **rock that was never there** — so an open-cast pit
  stays black however wide it is and however much sky is over it.

This is the same failure the "black rectangle under every tree" report was
about, and the `sky_surface` fix genuinely closed *that* one: `Plant` and
`Creature` are excluded from the freeze, and anything placed after frame one
cannot move the surface. What it did not close is **`Solid` and `Powder`
standing over open air at genesis**, which is terrain's own business and
which worldgen produces deliberately — `brows` exists to make overhanging
lips, with `MAX_BROW_REACH` = 20 columns.

## Measurements

New probe: `examples/underground_probe.rs`. It flood-fills from the top of
the world through everything that is not `Solid` or `Powder` — the exact
complement of `freeze_sky_surface`'s own predicate, so it is not a second
opinion about what ground is — and counts cells that are reachable from the
sky yet answer `!World::is_outdoors`. Those are **false caves**: open air
drawn as unlit rock. The genuinely enclosed count is reported beside it as
the control, because that is what the mechanism exists to darken and it must
stay large.

Generated worlds, 2048x640, default preset, 60 frames of settle:

| seed | false-cave cells | % of open air | at full `UNDERGROUND` | genuinely enclosed | largest patch |
|---|---|---|---|---|---|
| 1 | 156 | 0.064% | 0 | 5,178 | 54 |
| 2 | 408 | 0.147% | 16 | 7,894 | 47 |
| 3 | 197 | 0.083% | 16 | 491 | 45 |
| 4 | 339 | 0.130% | 0 | 9,243 | 43 |
| 5 | 330 | 0.129% | 22 | 2,068 | 52 |
| 6 | 168 | 0.060% | 0 | 7,622 | 53 |

p90 and max over the six are both seed 2's 408. The patches are small —
twenty to fifty cells — and they all sit on the skyline, at y 88–150, which
is where the cliff edges are. Small is not the same as invisible: rendered,
each one is a hard-edged patch of noticeably darker sky hanging off a lip,
and `F11`'s void X-ray puts magenta squarely in the open air beside the
cliff.

**Ruled out: the skyline going stale as the world settles.** The surface is
frozen on frame one, before anything has moved, so a world that slumped after
generation would strand the baseline above its own new surface — a thin false
band hugging the whole terrain, which is a different bug with a much easier
fix (freeze later, or re-freeze once). It does not happen. Seed 1 reports
**156 false-cave cells at 1, 60, 600 and 3,000 frames of settle**, unchanged.
The null is not vacuous: the *denominator* moved over the same runs (0.064% →
0.063% of open air, so air volume did change), which is what says the count
held still because the terrain did, not because nothing was being measured.
Generated terrain really is at rest by construction, and that is worth
knowing for the fix — everything here is baked at generation, so a fix at
generation time is sufficient and nothing has to be maintained per frame.

**Digging is an order of magnitude worse**, and it is the case the play
report leads with. Cutting the top 40 rows off a 64-wide patch of the highest
hill — an open pit, nothing left overhead — through the ordinary eraser brush
(`viewshot quarry=64`, added for this):

| | false-cave cells | at full `UNDERGROUND` | largest region |
|---|---|---|---|
| seed 1, untouched | 156 | 0 | 54 cells |
| seed 1, after the pit | 1,363 | 436 | **1,207 cells**, x 971..1011, y 61..100 |

On screen that is a black rectangle with hard vertical sides sitting in a
sunlit hillside, and the pit *floor* draws brighter than the air above it —
which is the tell that this is not light of any kind.

## Apportionment between the two consumers

Measured, not assumed. Rendering the pit with `light=flat` — `TerrainLight::
Off`, the `F10` A/B — leaves it **exactly as black**. All of the black is the
empty-cell cave fade in `background_at`. The depth grade contributes nothing
to it.

The depth grade is not innocent, though: it is what darkens *water*, and that
is the artifact the `water` board's card
`20260822T225340455Z-ad69f8` reported as "a dark vertical band through the
pond". `scene=rockdrop` reproduces it at **frame 0, with zero bodies in
flight**, so it is not the drop: the slab is present when the surface freezes,
so its columns' skyline is the slab's top, and the band it casts runs through
the air, through the pond, and onto the pool floor at exactly the slab's
width. That card's diagnosis named `TerrainLight::Depth` as the cause. It is
the mechanism for the water specifically; the cause of all of it is the
column baseline both consumers read.

## What a fix has to respect

`Reports/dead-ends.md` §977 is unconditional about the direction:

> Unconditional for inference; the fix stores history (`World::sky_surface`
> recorded once at first frame). **Revisit only by storing more history,
> never by inferring.**

Four rules that inferred "underground" from the standing world all failed,
and the last of them opened 35 rows of daylight into a mountain at a 13-cell
shaft while a 12-cell shaft stayed a tunnel. Widening a shaft is what mining
*is*, so any rule with a width threshold breaks while the player is using it.
None of that is reopened here. §985 adds the other constraint: the state must
live in the `World`, not the `Renderer`, or
`dirty_rect_skip_is_pixel_identical_to_a_full_redraw` fails.

## What landed

**The genesis void, stored per *cell* instead of per column.** One bit: "was
this position enclosed when the world was made". Seeded by exactly the flood
fill the probe already does — from the top of the world, through everything
that is not `Solid` or `Powder` — with every solid cell marked too, so a cell
is underground if it is solid, or if it was air that the sky could not reach
at genesis.

What it does to the three cases:

| case | today | with a per-cell bit |
|---|---|---|
| air under a cliff brow | cave | **sky** — the fill reaches it round the side |
| air under a slab suspended at genesis | cave | **sky** — same |
| a worldgen cave or vault | cave | cave — the fill never reaches it |
| a dug shaft, any width | cave | cave — those cells were rock |
| air under a roof built after genesis | sky | sky — unchanged, still the deliberate trade |
| a 64-wide open pit | cave | **cave** — those cells were rock too |

It is "storing more history" in the sense §977 requires, it has no width
threshold anywhere, and it is strictly a *subset* of option C in
`underground-definition.md` (the Terraria-style wall layer) in the same way
the current per-column rule is — nothing built for it has to be unbuilt to
get to C.

Cost, measured: the flood fill is **5.8 ms once** over 1,310,720 cells,
against worldgen's own 325 ms for the same world. Storage is one bit per
cell — 164 KB at 2048x640, against `sky_surface`'s 8 KB. Per-pixel it is a
bitmap index where today it is a `Vec<i32>` index and a compare, so the draw
cost is a wash and the dirty-rect skip is untouched (it stays a pure function
of position, stored in the `World`).

**The last row of that table was put to the owner** on card
`20260822T235306955Z-ac8c28`. The answer was *"we can start with the fix for
1 and 3... but I do want a better solution"* for 2, so the boolean shipped and
the pit is still black on purpose.

### Measured after

`examples/underground_probe.rs` now asks the **column rule directly** rather
than through `World::is_outdoors`, because that predicate reads the map the
probe's own flood fill seeds — asking it would have made the instrument a
function of the thing it measures and reported a flat zero whether the fix
worked or not. So the false-cave column below is unchanged on purpose; what
moved is the scoreboard beside it.

| seed | false cave (column rule) | rescued by the map | left dark |
|---|---|---|---|
| 1 | 156 | **149** | 7 |
| 2 | 408 | **406** | 2 |
| 3 | 197 | **192** | 5 |

The remainder is not a miss. Those cells were `Solid` or `Powder` at genesis
and are air now — the world moved them in its first sweep — so the map calls
them underground for the same reason it does a dug shaft. A zero in the
"rescued" column is the tell for a fix that never reached the predicate at
all, and the probe says so in words when it sees one.

### Frame cost

Measured in one session against a worktree built at the parent commit, and
**interleaved** (after, before, after, before, …) because the machine drifted
upward across the run and a block-ordered comparison would have charged that
drift to whichever went last.

| | draw, mean of 10 full redraws |
|---|---|
| round 1 | before 10.25 ms, after 11.86 ms |
| round 2 | before 11.39 ms, after 11.60 ms |
| round 3 | before 12.21 ms, after 12.50 ms |

**+0.3 to +0.7 ms on a ~11.5 ms full-screen redraw, call it 3–6%**, and the
sign is consistent across all three rounds even though the interleaving
biases *against* finding it. It is the per-pixel map read: the depth grade now
consults `under_sky` for every non-empty pixel it grades, where before it read
only the column datum. The 164 KB per-frame copy is not the cost — that is
~16 µs, which matters because a settled screen otherwise draws for almost
nothing (`CLAUDE.md`'s animated-grain lesson: measure a cost against the state
the optimisation exists for). The dirty-rect skip is untouched; the map is a
pure function of position held in the `World`, so a renderer with history and
a fresh one still agree.

`ascii`'s sim-side figures moved from a mean of 3.38 ms to 3.24 ms over 12,000
frames, which is noise and is recorded only so nobody reads it as a win.

### What it does not fix, and what is still visibly wrong

- **The open-cast pit**, as designed and as agreed.
- **Rock under an overhang is still over-darkened**, and *measured* it is far
  larger than the "suspended object" framing this section first gave it. The
  *decision* is per cell now; the *depth* still comes from `light_datum`, the
  per-column skyline with narrow dips clipped — and an opening clips dips, not
  spikes, so anything solid standing over open air hands the ground beneath it
  a large depth. Every cliff brow does this, not just a slab dropped in a test
  scene.

  Counted by `underground_probe`'s `overdark_report`, which compares the depth
  the grade reads (`y - sky_surface[x]`) against how far up you actually walk
  before reaching a cell the sky can reach — so a sealed cave does not count,
  because cave air is not outdoors and the walk keeps going:

  | seed | over by >=8 rows | by >=24 | by >=64 | worst |
  |---|---|---|---|---|
  | 1 | 7,569 | 0 | 0 | 19 rows |
  | 2 | 48,391 | 2,966 | 0 | 37 rows |
  | 3 | 25,588 | 5,415 | 0 | 29 rows |
  | 4 | 34,114 | 0 | 0 | 23 rows |

  **Cell count is the wrong scale to judge it by, though.** The grade is a
  smoothstep from 1.0 to `DEPTH_LIGHT_FLOOR` over 64 rows and is flat at both
  ends, so 8 rows of excess costs about **1.6%** of brightness — invisible.
  24 rows costs ~12% and the worst case ~23%, which is the part that can be
  seen, and it is zero on half the seeds measured.

  **It is not a shadow, and the argument that it might be was wrong.** The
  reasoning was that it darkens rock under an overhang, which is what ambient
  occlusion does, so removing it might flatten the cliffs. Clustering the
  visible cells kills that: they do not hug anything. They are **narrow
  vertical stripes, 1 to 10 columns wide, running from the surface to
  bedrock** — seed 5's is 2,990 cells at x 332..337 spanning y 139..639, and
  seed 7's is a *single column*, 494 cells tall. Nothing physical makes a
  six-pixel-wide column of rock 12% darker for five hundred rows. Rendered,
  it is a straight vertical tone seam with a hard edge, which is the same
  family as the artifact this whole report is about.

  A count could not have said that and did not: the totals looked like
  "rock under overhangs" until the cells were clustered and given
  coordinates. `CLAUDE.md`'s rule again — a metric says how much, only a
  position says *where*, and the shape was the whole question.

  Put to the owner as card `20260823T042729625Z-b3377d`, with the size
  stated honestly (zero visible cells on about half the seeds) rather than
  as a defect to be fixed on principle.

  Two obvious fixes are worse than they look. A per-column run-length datum
  (depth below the nearest outdoors cell *above*) reintroduces bright rock at
  the floor of every narrow notch — the exact artifact `light_datum`'s opening
  exists to prevent, because the two cases are the same shape to any column
  rule. Driving the solid grade off the sky-light field discriminates them
  with no threshold and collapses the lit surface band from 64 rows to about
  **4**, since light decays 0.56 a cell through solid: a whole-world relight,
  not a bug fix.

  **The narrow one, if it is wanted.** Store a second per-column datum at
  genesis — the top of the *lowest* run of cells the sky cannot reach, found
  by walking each column up from the bottom until an outdoors cell stops the
  walk — and feed the existing opening from that instead of the raw skyline.
  A brow is skipped because outdoors air sits below it; a cave is not,
  because cave air is not outdoors; a notch still gets the opening it needs.
  Stored history rather than inference, no width threshold anywhere, ~8 KB
  and one scan at genesis, and the existing notch guard already covers the
  regression it could cause.

## Guards

- `an_overhanging_lip_does_not_put_a_cave_in_the_sky_beneath_it` — the
  reported case, with a sealed cavity as the paired control, so "call
  everything sky" cannot pass it.
- `a_slab_hanging_in_the_air_at_genesis_casts_no_cave_beneath_it` — the
  object case, and distinct from the tree one: `Plant` is excluded from the
  freeze and a slab of stone is `Solid`, which no exclusion list reaches.
- `the_per_cell_map_never_turns_open_sky_into_cave` — the property, over
  every cell of a world carrying a lip, a slab, a sealed cavity and a dug
  shaft at once, plus a `rescued > 0` liveness assertion so it cannot pass
  against a map that is not wired up.
- All four guards from `underground-definition.md` still pass unchanged, and
  they are not vacuous under the replacement: the shaft one now exercises
  "those cells were rock", which is what keeps a tunnel a tunnel at any width.
- **Checked by sabotage**, per `CLAUDE.md`: reverting `under_sky` to the
  column form turns all three new guards red and leaves the other 66 render
  tests green.

## Instruments left behind

- `examples/underground_probe.rs` — the false-cave census above.
  `seeds=N` sweeps and reports p90 and max, per `CLAUDE.md`'s rule that a
  guard over procedural content gates an order statistic; `quarry=W` cuts the
  open pit; every run echoes its own parameters on the first line.
- `examples/viewshot.rs` gains `quarry=W`, beside the existing `mine=1`. The
  two answer different questions: a shaft can be argued to be a tunnel, and a
  pit with nothing over it cannot.

*Freshness: 2026-08-22.*
