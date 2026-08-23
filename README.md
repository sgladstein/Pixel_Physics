# Pixel Physics

A falling-sand pixel physics engine in Rust. Every cell in the world is one
simulated pixel. Built as a foundation for games, not as a one-off demo.

## Running

```sh
cargo run              # the sandbox
cargo run --example ascii   # headless terminal view, no GPU needed
cargo test
```

## Finding things

What each mechanic *does* in play — materials, fire, collapse, weather, the
gnome, the ant colony — is written up in plain language in
[`wiki/`](wiki/README.md). *Why* it is built that way lives in
[`Reports/`](Reports/README.md), which is indexed. How to *work* here —
method, conventions, gotchas — is `CLAUDE.md`. This file holds the
architecture and the per-milestone build status. Milestone sections are
titled `M<n> status` and sit in the order they were written, not numeric
order — find them by search: M5, M6, M7, M8, M9, M10 (worldgen), M12/M13,
M14, M15, M16, M17, M18, M19, plus sections for weather and the ant colony
and three overnight-run sections (§9 UI, §10 tunables, §11 rendering).

## Controls

| Input | Action |
|---|---|
| Left mouse | Paint the selected material |
| Right mouse | Erase |
| `1`–`9` | Select material by number |
| `Q` / `E` | Cycle material |
| `[` / `]` or scroll | Brush size |
| `=` / `-` | Zoom in / out (§9: one continuous scale — zooming out past 1x uses a sample stride instead of shrinking pixels) |
| `Space` | Pause |
| `.` | Step one frame while paused |
| `F` | Force-ignite whatever's under the brush (debug tool) |
| `P` | Throw a burst of the selected material as free particles (debug tool) |
| `X` | Trigger an explosion (its own radius/strength, tunable under `O` -> EXPLOSION, **not** the brush radius) |
| `C` | **Strike** the rock under the cursor — pulverizes the centre, loosens the rock around it and throws the pieces. Force scales with brush size. The destruction *verb*: erasing removes support but delivers no load, so before this nothing could fail from being hit |
| `T` | Plant a tree seed under the brush (M16 debug tool) |
| `M` | Plant a moss seed under the brush (M16 debug tool) |
| `J` | Plant a worm under the brush (M18 debug tool; was `W` before the gnome claimed WASD) |
| `U` | Summon the gnome at the cursor, or dismiss him (M9). Arrives in `Tool::Dig`, where left-click cuts the near rock face along the aim (the yellow ring shows where the bite lands and how big it is) and right-click still erases; `Z` cycles back to the brush. `A`/`D` run, `W` jump (tap for a hop, hold for full height), and a lip a little above a jump's apex is caught and mantled — the same four keys scroll the map while nobody is summoned, see the row below. He wades knee-deep in powder — slowed in proportion to how deep — swims in liquid (`W` strokes up, `S` down, and holding `W` through the surface hops him onto the bank), and rides a falling chunk body rather than being left behind by it. **A living plant is scenery he walks through, not a wall**: hold `Shift` to take hold of one, then `W`/`S` climb it and no vertical input hangs him there; releasing `Shift` lets go. Climbing has its own key because riding on `W` meant jump-walking through a wood grabbed every trunk it touched and let you hover — and falling through a crown is broken by the foliage. Left-clicking **a plant you are pointing at** shakes it rather than cutting it, and the pick sees straight through living tissue to the rock behind, so a tree merely in the way never swallows a cut: loose material comes off the branches, the shaded leaves that were already dying come down as litter, and a grown tree yields seed |
| `A` `D` `W` `S` (with no gnome) | **Scroll the map.** With nobody summoned the view is yours: the same keys that run him pan the camera instead, and the moment one is summoned it goes back to being his. No mode to toggle — the two readings can never both be live, because `App::draw` re-centres on a gnome every frame, so a camera the player set would simply be pulled back on the next one. The rate is *screens per second*, not cells, so the picture slides at one speed however far in or out you are zoomed; the step is quantised to the zoom-out sample stride, without which a zoomed-out view re-samples rather than translating. It **opens at about the gnome's own running pace and accelerates over ~0.8 s** to 0.5 screens/s, crossing the world in about six seconds — a tap nudges, a hold travels, and reversing restarts the ramp so correcting an overshoot does not fling the view back. It shipped as a flat 1.5 screens/s and was rejected by playtest as "way too fast"; see `render::PAN_SCREENS_PER_SECOND`. The world is four screens wide and two deep, so there is a good deal to see — the bottom-left readout shows where the view is |
| `F10` | Cycle **tree depth** — whether the gnome draws over a stand of trees, weaves through it (the default: half of them draw over him, chosen per tree and stable for its life), or passes behind all of it. Purely graphical; a living plant is walk-through in every mode |
| `F3` `F4` `F2` | Cycle the gnome's **movement feel**, **water feel** and **spoil mode**, in that order — named runtime selectors for the three things only play can settle. (An earlier version of this row had the keys scrambled; the binding is F3 = movement, F4 = water, F2 = spoil.) The active one is shown in the title bar once it differs from the default. Every underlying number is also sweepable under `O` -> PLAYER |
| `Y` | Found an **ant colony** at the cursor — the whole colony feature hangs off this key; see [`wiki/ants.md`](wiki/ants.md) |
| `F6` / `F8` | New world from a fresh seed / back to the previous seed |
| `F7` | Cycle the worldgen preset, keeping the seed — rolling → terraced → canyon → wetland → arid → legacy → flat (the structural test bed) |
| `F9` | Cycle how far structural damage may travel from a blow: SPREAD → LOCAL → TIGHT → NONE; named in the title bar off the default. See [`wiki/structural-collapse.md`](wiki/structural-collapse.md) |
| `L` | Cycle the organism overlay — per-cell organism channels on a fixed dark→bright ramp |
| `I` | Toggle the hover inspector — material, temperature, every field channel at the cursor |
| `N` | Toggle the structural stress view — every load-bearing cell tinted green at rest through red at its limit |
| `H` | Dig — a precise cut that loosens and cracks the rock around it, unlike the eraser (was `D`, which now runs the gnome right) |
| `Z` | Cycle the build tool — freehand brush, solid rectangle, hollow room, line |
| `B` | Stamp a **reference room** — 200x160, standing on whatever ground is under the cursor, walls as thick as the brush. Deliberately sized at the measured edge of what the structural model holds (`Reports/next-session-handoff.md` §2b), so the open question "is a room this big a reasonable thing to want to build?" can be judged in the hand instead of argued from a contact sheet |
| `V` | Cycle the field overlay: off → pressure → temperature → light → moisture → off |
| `G` | Cycle how a liquid's brightness grain is generated: position (default) → cell → muted → animated → motion → animated-muted → animated-smooth. Kept as a live selector so the look can keep being iterated on. The animated variants have to redraw liquid chunks the sweep never touched, so they are the only ones that cost anything: measured on a fully settled world with water across 92% of the width, animated-muted costs 1.45 ms and animated-smooth 7.5 ms, against 0.000 ms for every other mode and for any of them with no water on screen. Exists so the variants can be judged on real moving water rather than argued about; the active one is shown in the title bar. See `render::GrainMode`, and expect this key and the enum to disappear together once one is chosen. |
| `K` | **A/B key.** Flips whatever is being evaluated right now between its baseline and candidate value, so a comparison is one keypress rather than a scroll through a panel. Deliberately reassigned whenever the question changes, with the previous experiment deleted rather than accumulated — see `App::toggle_experiment` for what it currently does. |
| `Tab` | Toggle the material palette (swatch row, current selection outlined) |
| `/` (shown as `?`) | Toggle the keybind help overlay |
| `O` | Toggle the live tunables panel (§10 — browse/adjust/save material fields at runtime) |
| `PageUp` / `PageDown` | Switch which tunables menu is shown (PHYSICS / VISUAL / EXPLOSION / PLAYER), while the panel is open. Split because a dozen materials times ten fields is one scroll of well over a hundred rows. |
| `↑` / `↓` | Move the tunables selection (only while the panel is open) |
| `←` / `→` | Adjust the selected tunable's live value by its own step while the panel is open — and with it closed, adjust the **pinned** tunable, so a value under evaluation is one keypress away mid-play |
| `Enter` | **Pin** the selected tunable for those panel-closed arrow keys (only while the panel is open) |
| `S` | Save the selected tunable back to its `.ron` file, preserving comments (only while the panel is open; this moved off `Enter` when pinning claimed it) |
| `F1` | Chunk overlay — green borders are awake, grey are asleep |
| `F5` | Reload materials and species by hand |
| `R` | Reset |
| `Esc` | Close the tunables panel if open (without saving); quit otherwise |

The window title shows frame rate, selected material, how many chunks are
awake, and the result of the last material/species reload. `0/40 awake` on
a still world means chunk sleeping is working. A persistent HUD line
(bottom-left) shows the same material/brush-size reading always, not just
in the title bar — §9's `hud.rs` bitmap-text primitive, the engine's first
on-screen text.

## Materials

Materials live in [`assets/materials`](assets/materials) as one `.ron` file
each, and are **watched while the app runs** — save a file and the change
applies immediately, with any parse error shown in the window title. The files
are also compiled into the binary, so the engine still works without them.

```ron
(
    name: "sand",
    kind: Powder,          // Solid | Powder | Liquid | Gas | Plant | Creature
    density: 1.6,          // heavier sinks through lighter
    friction_angle: 34.0,  // powders: angle of repose, 45 is steepest
    dispersion: 5,         // liquids and gases: sideways travel per step
    colors: [(222, 196, 128), (212, 184, 116)],
)
```

Ids are keyed by name and never reassigned, so editing a file changes material
already in the world rather than replacing it. Renaming adds a new material.

`Plant` and `Creature` are the two kinds the CA sweep never moves — an
organism cell is relocated by its own tick through `World::get`/`set`, never
by falling or flowing. That single fact drives most of how organisms are
built; [`wood.ron`](assets/materials/wood.ron) and
[`worm.ron`](assets/materials/worm.ron) carry the load-bearing comments on
why each is its kind and not `Solid`.

**`friction_angle` is the parameter worth playing with.** A pile rests where a
grain can no longer see anywhere to fall within its reach, which makes the
resting slope `1 / reach`, so `reach = 1 / tan(angle)`. Because reach is a whole
number of cells, the fractional part is spent by giving *some positions* the
longer reach — which averages to the right angle and leaves the surface
irregular instead of a perfectly straight wedge. Gravel at 45° holds a sharp
peak, sand at 34° a moderate one, ash at 22° slumps almost flat.

That draw is keyed on **position, not the random generator**, and it has to
stay that way. Drawing fresh each call lets a chunk fall asleep on a frame the
dice said no, freezing grains that should have kept rolling.

**The M14 schema** (combustion, phase change, reactions) — see
[`oil.ron`](assets/materials/oil.ron) for the first material that used it —
has been read by `fire.rs` since M14 landed (see the M14 status section
below). This paragraph used to say "nothing reads it yet" and kept saying it
for three milestones after that stopped being true, which is its own small
lesson in where status claims belong. All
of its temperatures are Celsius, the same unit `Cell` and the field grid both
already use:

```ron
flammability: 0.5,       // chance to ignite per burning neighbour per step
burn_temperature: 900.0, // what it radiates while on fire
burn_duration: 180,      // frames
burns_into: "ash",       // plain string, not Option<String> — see field.rs's
                          // notes on RON's Some(...) requirement for why.
                          // burns_into is combustion residue, separate from
                          // melts_into (temperature-triggered, unrelated to fire)
```

**The phase-change half of that schema is what the water cycle is made of**, and
all of it is content — no rule in `fire.rs` names any of these materials, it
reads these fields and does what they say:

| material | kind | what it does |
|---|---|---|
| [`water.ron`](assets/materials/water.ron) | `Liquid` | boils into `steam` at 100°C, freezes into `ice` at 0°C. `freeze_min_fill` refuses to freeze anything but a near-full cell, which is what lets the round trip close. |
| [`steam.ron`](assets/materials/steam.ron) | `Gas` | condenses back to `water` at 45°C, carrying its source cell's fill in `aux` across both transitions — per-cell exact through the loop. |
| [`ice.ron`](assets/materials/ice.ron) | `Solid` | melts back at 1°C, floats, and is a **true structural solid**: a sheet spans to the shore or it comes apart. `heat_conductivity` is not optional on it — see below. |
| [`snow.ron`](assets/materials/snow.ron) | `Powder` | made only by weather, never by freezing. Melts at 2°C into 0.3 of a cell of water — its own density, not a free full one. |
| [`lava.ron`](assets/materials/lava.ron) | `Liquid` | born hot **once** (`intrinsic_temperature`), then cools like anything else and crusts to `stone` at 700°C. Reacts with water into stone and steam, the reaction taking the hotter side's heat into the steam. |

A boiling look (`,`) and a gas translucency (`;`) are runtime selectors in
`render.rs`, both defaulting to the behaviour that shipped before them.

## Architecture

```
src/sim/     the simulation — knows nothing about windows or GPUs
  cell.rs      one pixel, packed into 12 bytes: material, shade, flags,
               temperature, a burn timer, a kind-specific aux slot, and
               an organism-ownership id
  material.rs  materials as data, not code
  chunk.rs     64x64 tiles, coordinate maths, dirty rectangles
  field.rs     the coarse pressure/velocity/temperature/light/moisture
               grid, one tile per chunk, its own frame phase
  fire.rs      heat, ignition, burnout, phase change, reactions
  particle.rs  free (off-grid) particles for explosions and splashes
  explosion.rs pressure impulse + heat spike + debris, built from the above
  world.rs     the sparse chunk map and the get/set seam
  update.rs    the cellular automaton step's rules, generic over CellSurface
  surface.rs   the CellSurface trait update.rs/fire.rs run against --
               World (serial) or parallel::ChunkView (multithreaded)
  parallel.rs  M5: the multithreaded checkerboard sweep -- an alternative
               driver for update.rs's rules, not a second copy of them
  scheduler.rs M16: the active-site list -- everything that must happen to
               a world the sweep has stopped visiting, checked in its own
               phase at cost proportional to how much is happening
  plant.rs     M16: plant growth -- germination, moss, tree and root
               growth, dispatched from scheduler.rs
  organism.rs  the shared cell-typed organism substrate: species as data
               (assets/species/*.ron), one organism state per individual
               -- what retired TreeState and CreatureState
  creature.rs  creatures on that substrate: the worm, the ants, the beetle
  brain.rs     the creature brain, behind its deliberate sense/act cage
  pheromone.rs the ant colony's two trail channels: deposit, diffuse,
               decay, follow
  evaporation.rs standing water drying up, dispatched from scheduler.rs --
               a puddle goes and a lake does not, with nothing measuring
               the size of a body of water (tests in evaporation_tests.rs)
  decay.rs     ash weathering into soil, moisture-gated -- the regrow half
               of "a forest burns and regrows"
  weather.rs   fronts, rain, snow, wind and gusts, lightning -- weather as
               a deterministic property of the world, not an event roll
  structural.rs M17: anchor distance, confinement, the structural check --
               what decides a cell is no longer held up
  load.rs      the load/torque failure criterion on top of it: who carries
               what, and where it breaks -- fissures, strike damage
  rigid.rs     M8: chunk bodies -- component labeling, contour tracing,
               and detached pieces falling as one coherent thing
  fracture_field.rs
               the joint fabric: which Worley domain a cell of rock belongs
               to, so a blast reveals the grain the rock already has instead
               of drawing a star across it. Position-keyed and stateless, so
               a second charge retraces the first one's breaks
  liquid.rs    heightfield liquid bodies -- test-only today: promotion was
               implemented and reverted, so its bugs are latent until it
               lands (Reports/liquid-heightfield-design.md)
  player.rs    M9: the gnome -- running, jumping, digging, burial, swimming
  rng.rs       position-keyed jitter and per-chunk streams -- deterministic
               randomness that survives chunk sleep
src/worldgen/  M10's worldgen half: a playable 2D slice cut from coarse 3D
               worldgen -- params.rs (the knobs, from assets/worldgen.ron),
               noise.rs, region.rs (the 2-5 regions across a world's
               width), column.rs (per-column shaping), passes.rs (the pass
               pipeline), erosion.rs (plan-space erosion, which is what
               makes the mesas and benches), residual.rs (tors and stacks),
               spring.rs (spring placement), legacy.rs (the old hand-built
               practice terrain)
src/render.rs  cells to pixels; dirty-region skipping, overlays, grain
src/sky.rs     the sky: day/night gradient, dawn and dusk, stars, the moon,
               storm dimming -- and the ground lit by time of day
src/hud.rs     the 5x7 bitmap-text primitive every on-screen readout uses
src/tunables.rs the live tunables registry behind the O panel
src/app.rs     sandbox state: brush, picker, tools, terrain, experiments
src/main.rs    window, input, fixed 60 Hz timestep
src/lib.rs     the crate root that wires the above together
```

### Invariants

These are cheap to hold and very expensive to retrofit, so nothing may break
them:

1. The world is a `HashMap<ChunkCoord, Chunk>`, **never** a flat `Vec<Cell>`.
   A flat array indexed `y * width + x` is the one decision that would force a
   rewrite when the streaming world arrives.
2. Every coordinate crossing a public API is a **global signed world
   coordinate**. Screen space exists only in the renderer.
3. All cell access goes through `World::get` / `World::set`. That is the seam
   where chunk loading, generation and eviction get added later.
4. The simulation never writes colours. It stores a material id and a shade
   index; the renderer resolves those to RGBA.

### Two subtleties worth knowing before editing `update.rs`

**Sweep order.** Rows are swept bottom to top, or a falling cell gets
re-examined at its new position on the same sweep and falls again, dropping a
column of sand to the floor in one frame. The horizontal direction alternates
each frame, or every symmetric decision is biased the same way and piles
visibly drift.

**The moved flag, and why it is not a parity bit.** A cell that moves must not
be processed twice in one sweep, so `Cell` carries a `FLAG_MOVED` bit that the
sweep clears when it skips the cell.

It is tempting to use a frame-parity bit instead — compare the cell's bit to a
parity the world flips each frame, and nothing needs clearing. That is broken,
and was the cause of sand freezing in mid-air. Parity only stays in step if
every cell is visited every frame, which is precisely what dirty rectangles stop
doing. A cell skipped for a single frame ends up with a parity that aliases with
the current one, reads as already-handled, is skipped again, and never gets
stamped — so it is skipped on every alternate frame forever and freezes. A flag
cleared when consumed cannot go stale.

The flag is set **only when the sweep will reach the destination again** — see
`World::move_cell`. Downward moves land in rows already passed and must not be
flagged, or everything falls at half speed.

**No rule may look further than `MAX_REACH` (32).** Every movement rule caps
itself at it independently — powder roll (via its friction angle), a liquid's
horizontal levelling search (`HORIZONTAL_TRANSFER_REACH`, 8), a gas's
dispersion search — so it is a hard outer bound on all of them, not a value
any single rule normally reaches. A rule that reads further than its chunk's
sweep region is widened acts on cells that no longer wake it, and material
goes stale mid-flow.

Sweep regions are widened horizontally by each chunk's own **tracked
reach**, not a flat constant (issue #3): every write grows it to at least
that material's own reach (`Material::sweep_reach`), and it only shrinks
back down when the chunk goes fully quiet, the one point a smaller value is
both cheap to recompute and safe to adopt. A chunk holding only sand no
longer pays for a `MAX_REACH`-wide band the way a chunk full of dispersing
gas does. (One cell vertically either way, which is as far as anything
looks up or down, is unaffected — that part of the widening was never the
expensive one; see `chunk.rs`'s own doc on `Chunk::sweep_region`.)
**This limit applies to the CA sweep only.** The field grid (below) is a
whole-grid pass that reads everything every step regardless of what
changed, so it has no equivalent staleness risk and is not bound by it —
that is precisely why long-range effects like a shockwave crossing the
whole screen live there and not in a CA rule.

## The coarse field grid

One [`FieldTile`](src/sim/field.rs) per chunk, at 1/8 the CA grid's
resolution, carrying pressure, velocity, ambient temperature and light.
Modelled on [The Powder Toy's `Air.cpp`](https://github.com/The-Powder-Toy/The-Powder-Toy/blob/master/src/simulation/Air.cpp):
pressure accumulates velocity divergence, velocity accumulates the negative
pressure gradient, walls block flow, and the whole thing needs damping to stay
bounded since it is not an exact energy-conserving scheme.

One deliberate departure from that reference: it updates in place
(Gauss-Seidel — later cells in a sweep see already-updated earlier ones).
This implementation reads a full snapshot of the old state and writes a fresh
new one every pass instead (Jacobi) — slightly more memory, but every pass is
order-independent and parallelizable later, which matters once the CA sweep
threads the same way.

### Wall boundary conditions

**Wall boundary conditions went through three attempts, and the reasoning for
landing on the third is worth knowing before touching that code again.**
Zeroing a cell's whole velocity whenever it touched *any* blocked neighbour,
regardless of direction, was first — and wrong: in a small sealed room almost
every interior cell borders some wall, so that version force-zeroed velocity
there nearly every step no matter which way it was flowing, bleeding energy
out of a sealed room *faster* than open ground (which never triggers the
check at all). Measured backwards: a sealed room retained less pressure than
open air. Reflecting the blocked component (flipping its sign to bounce,
conserving kinetic energy) was tried next and made the same measurement
*worse*, most likely energy pooling at wall-adjacent cells and repeatedly
hitting the pressure/temperature clamps, which are themselves lossy.
Reflection is a billiard-ball model, appropriate for discrete particles; the
textbook boundary condition for a continuum velocity *field* is
no-penetration — only the velocity component actually pointing into a wall is
stopped, and it is clamped to zero, not bounced. That is what is there now.

An independent review (a fresh agent, no context from building this) caught
three more bugs before this was trusted: `sample_bilinear`'s interpolation
weight was computed at one-world-unit granularity instead of across a field
cell's full 8-cell width, `step_pressure` checked wall occupancy against the
*previous* step's map instead of the one just rebuilt for the current step,
and `step_diffusion` had no wall awareness at all, so heat and light diffused
straight through solid stone. All three are fixed, and each has a named
regression test in `field.rs`.

### The light channel

**The light channel had two readers from the start (`plant.rs`'s moss
`shade_factor` and tree phototropism) and no writer until `Reports/emergent-
world-architecture.md` §2** — both mechanisms existed but were permanently
inert, since ambient light always read zero. Two writers now feed it: fire
(`fire.rs`'s `tick_burn` pushes a small `add_light` next to its existing
`add_heat`) and a constant sky boundary condition (`field::apply_sky`, run
last in `field::step`'s pipeline, after `step_advection` — every earlier pass
unconditionally overwrites every field cell it touches, sky row included, so
anything applied earlier would just be clobbered). `apply_sky` forces the
topmost field row *without a chunk resident directly above it* to `MAX_LIGHT`
each step (unless that row's own cell is CA-blocked), which adapts correctly
to an irregular or still-streaming chunk layout rather than assuming a single
global top row. It deliberately does not clear `fields_settled` — unlike
`add_light`/`add_heat`, it is a standing boundary condition, not a
disturbance, and `is_converged`'s existing comparison already notices any
real change on its own (a newly exposed or newly shaded cell shows up as a
jump between the pre-step and post-step value, same mechanism issue #4
already relies on for CA occupancy changes).

`LIGHT_DECAY` retuned from 0.85 to 0.997 (owner request: real outdoor sunlight
depth rather than requiring every plant within a couple of field rows of open
sky, which the original steep decay effectively forced once the tree
rewrite's `Germinate` made light-gated growth real) — a field cell reads
above a `0.1` threshold to roughly 75 world cells below open sky now, versus
roughly 20 before. The real cost: convergence to a static sky amplitude now
takes roughly 100x longer (`field.rs`'s own `LIGHT_DECAY` doc has the
specifics) — the field-sleep optimization (issue #4) stays correct, it just
spends more real frames awake near each day/night peak/trough first. The
regression test (`open_sky_reads_brighter_than_a_directly_blocked_cell`)
still only probes one field row down — that's checking the sky boundary
condition itself, not the full depth range.

### Sampling: block-nearest broke every gradient-follower

**`World::field_at` is block-nearest** — any two positions inside the same
8x8 field block read byte-identical values, which quietly broke every
short-range gradient-follower built against it: a worm's thermotaxis
compares its four ±1-cell neighbours, and a tree tip's phototropism compares
"here" against a 4-pixel-up probe, both almost always landing inside the
same coarse block and degenerating "follow the gradient" into "always pick
whichever candidate was checked first." `World::field_at_bilinear(fx, fy)`
(architecture §6a) exposes the interpolated sampler `step_advection` already
used internally, and both consumers now read through it instead. Left
alone, deliberately: trail *width* (a one-cell-wide pheromone trail smeared
across an 8-cell field block stays smeared no matter how it's sampled) —
that's a future moisture/pheromone channel-resolution question, not this one.

### The moisture channel

**A fifth channel, moisture, closes architecture §4** — `Liquid` CA cells now
push ambient humidity into the field (`apply_moisture_sources`, same shape as
`apply_sky`), which diffuses, evaporates faster near heat, and replaces two
hand-rolled O(r²) grid scans (`is_damp`, `strongest_water_pull` in
`plant.rs`) with one shared field every consumer reads. `rebuild_blocked`'s
own CA scan now also detects `Liquid` presence in the same pass, which cost
it its early exit on finding a solid cell: the first version kept the
original "stop at the first solid cell" short-circuit and broke moss
detecting a directly-adjacent puddle whenever an unrelated solid cell (a
retaining wall, say) happened to sit earlier in scan order than the water —
caught by `moss_spreads_over_damp_stone_and_not_over_dry` regressing from
"spreads over damp stone" to "spreads over almost nothing." Every field
block is now scanned in full instead. Measured against the same full-screen
stress scene the issue #4/#5/#6 numbers above use: **no measurable
regression** — 28.0 ms serial / 8.3 ms parallel, statistically the same as
the ~28 ms/~9 ms already on record. The scan itself was never the bottleneck
in a scene this CA-heavy; if a future scene turns out to actually feel this
cost, it's the first place to look.

### Evaporation rides the scheduler

**Standing water dries up, and the moisture channel is what makes a lake
different from a puddle** (`src/sim/evaporation.rs`). Water already
humidifies the air above itself, so a wide body saturates that air and cannot
evaporate into it while a thin puddle on bare rock sits in whatever is around
and goes -- measured at 1.45 humidity over a six-cell puddle against 2.31
over a 240-cell lake, settled, four rows deep. Nothing counts cells or floods
a region. It runs on the **active-site scheduler**, not the CA sweep, and
that is the whole reason it works: a settled chunk is not swept, and still
water is exactly the state the sweep stops visiting. Built on the sweep
first, it dried a lake faster than a puddle (7% against 1.7%) because a lake
stays awake settling for longer. The sweep now only *schedules*.

**The loop closes, and it runs on the day.** Evaporation used to delete
water and credit nothing while precipitation created it out of nothing; both
halves now go through one `f64` on `World` in cell-equivalents
(`World::atmospheric_bank`), so a storm can only spend what evaporation put
there and `render.rs` thins the *drawn* rain by the same supply factor. On
top of that, `evaporation::warmth` reads the sky's day/night temperature
**raw** — the one place in the engine that does not divide a designed
oscillator back out, because here the oscillation is the effect: a puddle
loses 2.47x as much across a noon-centred window as across a midnight-centred
one under a lid, 3.7x under open sky. The factor is *linear* in the offset,
so its day-mean is exactly 1.0 and the drying timescale is unchanged over
whole days — re-measured at -0.1% over four days, with `FILL_PER_CHECK` and
`HUMID_STOP` both checked and neither moved. `filmstrip scene=watercycle`
runs two clear days, a storm and a clear day after it: the bank climbs
through each afternoon, sits nearly still through each night, drains through
the front, and standing water plus bank reads 3940.0 on all twenty tiles.

One fix in the field was a prerequisite: `step_diffusion`'s wall rule
discarded the moisture source in any block that also held solid, so water in
a rock basin humidified nothing at all -- 2.310 in the air above a body whose
block was clear of stone against **0.000** for the identical body shifted
three rows, for a body 240 cells wide. Moisture, and only moisture, now reads
through a blocked neighbour that is itself a source; heat and light keep the
strict wall rule the sealed-room guards depend on.

### Organism feedback loops

**Both channels plants read are now also channels they write** (architecture
§5g), which is what turns "moss reads shade" and "roots read moisture" from
one-way sensing into an actual feedback loop between organisms. Light
occlusion needed no new code — `rebuild_blocked` has blocked on `Solid |
Plant` since M16, so a tree canopy already shaded whatever grew under it the
moment the light channel itself started carrying real values. Moisture
needed one new method, `World::deplete_moisture` (subtracts and floors at
zero, the mirror image of `add_light`), called at both of a root's
water-drink sites: a root draining a small, contained puddle now leaves it
measurably drier, which a second root's own `moisture_pull` gradient read
can notice and steer away from — resource competition mediated entirely
through the world, with no code anywhere that knows two roots are competing.

### The day/night cycle

**A day/night cycle now drives the sky** (architecture §5h) — the same
`apply_sky` writer from §2, given a time-varying amplitude instead of a flat
`MAX_LIGHT`: a clamped cosine hump that spends half of `DAY_NIGHT_PERIOD_
FRAMES` at a dim `NIGHT_LIGHT_FLOOR` and the other half ramping through a
daylight peak. Every existing light-channel reader (moss shade-seeking, tree
phototropism) gets a real cycle for free. The one thing this couldn't just
be free: `apply_sky`'s value now changes with elapsed time alone, with no
CA write to keep the field-sleeping gate (issue #4) awake for it the way
every other disturbance does — left alone, a field that settled at noon and
then saw the world go quiet would stay frozen at noon forever. `field::step`
now also wakes for a sky-amplitude change bigger than its own settle
epsilon, checked with one cheap pure-function call rather than a field read,
which — since the oscillator's rate of change is genuinely near zero at
noon and midnight — still lets a scene sleep through the steady parts of
day and night exactly as before, only staying awake through the actual
dawn/dusk transition.

### Decay and regrowth

**Ash decays into soil, moisture-gated, and soil sometimes reseeds plant
growth** (architecture §5f/§5e) — closing M16's own verify criterion, "a
forest burns and regrows," whose regrow half didn't exist before this. New
`decay.rs`, a new `soil` material, and a new `ActiveKind::Decay` dispatched
from the M16 scheduler the same way structural checks and creatures already
are. `fire.rs`'s burnout path schedules a decay check the moment a burnout
produces ash specifically; from there it's the same damp/dry duality
`plant.rs`'s moss already uses, and a freshly-decayed soil cell gets one
roll to reseed moss or a tree in the empty cell above it. Reaching this from
inside `fire::tick_burn` — which runs generic over `CellSurface`, both the
serial sweep and the parallel sweep's per-worker `ChunkView` — needed
`CellSurface` extended with `frame()` and `schedule_active_site()`, since
only `World` owns the active-site heap; `ChunkView` queues the site and
replays it after the pass, the same shape as its existing `field_writes`/
`light_writes` queues.

Exposed a real, if quiet, regression along the way: `examples/ascii.rs`'s
`plant_scene` helper never actually called `world.step_fields()`, despite
its own doc comment claiming it did. Harmless before the moisture channel
existed (`is_damp` used to scan the CA grid directly), but once §4 switched
that to a real field read, the "moss spreads on damp stone, stalls on dry"
demo scene silently stopped demonstrating anything — both sides read
uniformly dry, since the field was never actually being solved. Fixed, and
a new `regrowth_scene` demoes the whole ash → soil → (sometimes) regrowth
path end to end.

### Wind lean, and structural cover for plants

**Two of the plan's "lower priority" extras are done too.** A tree's growth
direction now leans downwind of a real pressure-field breeze — the same
additive formula phototropism already uses, given a fixed-magnitude,
direction-only wind term (not scaled by raw velocity — `field.rs` clamps
pressure but never velocity, and an early version that did scale by
magnitude let a nearby explosion's own shockwave dominate the formula
outright; independent review caught it, along with the original test's
one-shot impulse producing a decaying oscillation rather than a real steady
breeze). And structural integrity (M17) now covers `Plant` as well as
`Solid`: `wood.ron`
finally has the span/`breaks_into` numbers the plan named from the start
("stone 3, wood 8, steel 20"), a burnt-away trunk base brings the rest of
the tree down the same way cutting a stone bridge's support does, and a
broken trunk falls as a new `deadwood` material rather than vanishing. The
`Cell::aux` slot this needed was reserved for a per-cell growth stage that
was never actually built — real per-tip state lives in `TreeState` instead
— so extending the slot's meaning was a resolution, not a workaround.

### Playtest-driven changes

**Playtest feedback drove two more changes.** Running the actual GUI (not
just the ascii harness) surfaced that explosions vaporized almost everything
in the blast radius and produced almost no visible force — an old
`chance(1.0 - sqrt(dist2/r2))` debris roll put the odds against debris
almost everywhere a circle's area actually is (its outer band), which a
reproduction test confirmed gave only 28% debris in a dense fill. `explosion
.rs` now vaporizes only a small deterministic core (`VAPORIZE_FRACTION =
0.12`), gives everything else in the primary radius debris unconditionally,
and adds a shockwave annulus out to `radius * 1.8` where loose material
(`Powder`/`Liquid` only — not `Solid`/`Plant`, which stays M17's territory)
gets a linearly-fading pickup chance, so a blast in the middle of a sand
pile now actually flings sand outward instead of just collapsing the
crater inward. Separately, fire looked flat — a cell would flash orange for
a frame and revert, with no real flame look. `render.rs` now flickers
actively-burning cells (`rng::jitter3` keyed on position plus a coarse
time bucket, so it varies frame-to-frame without 60fps noise or per-cell
state) and blends toward a real hue ramp (`FIRE_TINT_LOW` dim ember →
`FIRE_TINT_HIGH` bright yellow-white by `heat_ratio`) instead of one flat
tint, so intensity changes colour, not just blend strength. A third piece
of feedback — tree growth starting mid-air with no germination, uniform
one-pixel trunk thickness, roots failing to grow on bare stone — was
deliberately **not** acted on yet; it needs a longer design conversation
first, with an explicit constraint from the owner to avoid hardcoding the
fix and instead get the emergent behaviour from simple rules.

## M12/M13 status

`Cell` widened from 4 to 8 bytes (32 MB instead of 16 for a 2048² world —
irrelevant) to carry a per-cell temperature and a kind-specific `aux` slot:
burn timer while on fire, anchor distance for a solid, growth stage for a
plant, owner id for a creature. `MaterialDef` gained the M14 schema
(`flammability`, `ignition_temperature`, `burn_temperature`, `burn_duration`,
`heat_conductivity`, `melting_point`/`melts_into`,
`boiling_point`/`boils_into`, `burns_into`, `reactions`) — oil is the first
material with real numbers behind them, burning into ash. Cross-material
references (`melts_into: "ash"`) are plain `String`, not `Option<String>` —
RON requires an explicit `Some(...)` wrapper for a present `Option` value,
which is friction with no payoff for a field that is simply absent (empty
string) most of the time. Resolution from name to `MaterialId` happens in a
dedicated pass after every material in a reload batch is known, since a
material can reference one that has not been parsed yet, or one from an
earlier load this reload never touches.

**Widened again, 8 → 12 bytes**, once the "burning always overwrites
`aux`" rule above turned out to be a real bug rather than a harmless
simplification: a burning `Liquid` cell (oil) needs `aux` to keep meaning
its fill fraction while it burns, and a burning `Plant` cell needs its
`aux`-held cell-type tag to survive the fire rather than resetting to type
0 once the timer clears. `burn_timer: u16` and `organism_id: u16` (the
latter reserved, unused until the organism-substrate rewrite) are now
their own fields; `aux` is genuinely kind-specific with no exceptions.

## Liquid physics: compressible volume, not discrete occupied cells

Replaced the original "search up to `dispersion` cells for an empty
destination" model, which structurally could never level a wide body of
liquid — a cell buried more than `dispersion` (5) cells from an opening had
no destination to find, on any frame, confirmed from a live playtest
screenshot showing a water column eroding only at its edges. Each `Liquid`
cell now holds a continuous fill amount in `aux` (`material::LIQUID_FULL`
= 1000 scale, `LIQUID_MAX_COMPRESS` = 10 allowed overfill), exchanging fill
with neighbours — the standard falling-sand compressible-volume technique
(Tom Forsyth's "Cellular Automata for Physical Modelling"; the
w-shadow.com falling-sand water tutorial). `aux == 0` on a `Liquid` cell
means "untouched, treat as full," not "empty," which is what lets every
pre-existing liquid-creation site in the codebase keep working unmodified —
a drained cell converts to `Cell::EMPTY` outright rather than lingering as
a zero-fill `Liquid` cell.

Horizontal transfer scans up to 8 cells (`HORIZONTAL_TRANSFER_REACH`) for
the emptiest reachable cell rather than only the immediate neighbour —
added after a live capture showed pure nearest-neighbour diffusion needing
on the order of *width²* frames to flatten a wide body (a 100-cell column
was still a visible mound after 3000 frames), which raising `flow_rate`
alone could not fix. New per-material `flow_rate: u16` field replaces
`dispersion`'s role for `Liquid` kind specifically; `dispersion` is
untouched and still governs `Gas`.

## M14 status

Heat, fire, phase change and reactions, in [`src/sim/fire.rs`](src/sim/fire.rs) —
called once per visited CA cell, before movement, so a phase change lands
before the same frame's movement dispatch decides how the cell behaves.

**Neighbour-driven ignition is rolled fresh every frame, not keyed on
position like `roll_reach_at`, and this is safe for a different reason than
it looks.** `roll_reach_at` had to be position-stable because a lone rolling
grain's own dice was the only thing keeping its chunk awake — an unlucky roll
could let the chunk sleep and freeze it permanently. A burning neighbour is
different: it keeps re-dirtying *its own* position every frame for as long as
it burns (independent of what this cell's ignition roll says), which keeps
this cell's chunk awake regardless. An unlucky roll this frame just means
another one next frame, for as long as the fire lasts.

**Residual heat needed the same class of fix `roll_reach_at` needed, applied
to temperature.** A cell warmer than ambient has nothing external re-dirtying
it, so it must keep writing itself while unsettled — `THERMAL_SETTLE_EPSILON`
governs when it stops. Naively rounding the diffused temperature hit a real
numerical fixed point a few degrees short of equilibrium (21.6 rounds to 22,
which pulls toward 20 by only −0.4, which rounds right back to 22) — caught by
a test that ran 5000 frames waiting for a 200° cell to reach ambient and it
never did. Fixed by guaranteeing at least one degree of progress whenever the
raw (unrounded) pull is real but rounds away to nothing.

**The field coupling in `diffuse_heat` was tried, measured, and removed.**
Pulling every visited cell toward the field's ambient temperature made a
sleeping cell's residual heat converge to true ambient — but `diffuse_heat`
only ever runs for a *visited* cell, and a sleeping chunk's cells are by
definition not visited, so the mechanism never fired for the case it was
built for. It also cost a `HashMap` lookup per visited CA cell: on the
sandbox's full-screen stress scenario that took the worst frame from ~16 ms to
~64 ms. Removed; CA-neighbour diffusion alone already converges a visited,
isolated hot cell to exact ambient, since an untouched neighbour reads
`Cell::EMPTY`'s ambient default. The coupling that *is* still needed — so the
field actually reflects nearby fire, for later milestones that read ambient
temperature or light — is pushed from burning cells instead
(`World::add_heat`, called from `tick_burn`), a naturally small, bounded set
at any moment rather than every swept cell.

**`heat_conductivity` defaults to 0.0, not a moderate always-on value.** A
default of 0.15 was tried first, so every material conducted heat plausibly
without a content author having to think about it — but `diffuse_heat` runs
for every visited cell, and nonzero conductivity means it cannot take its
cheap early exit. Sand and water, neither of which had any reason to conduct
at the time, were paying for four neighbour reads apiece on every visit for
nothing; that was a further, separate contributor to the same regression
above. (Water has since opted in deliberately, and what that cost was
measured — see below.) Materials
opt in explicitly, and each one has a reason: oil; ash, its `burns_into`
target — see the comment in `ash.ron` for why a combustion *byproduct*
specifically must not default to zero, or the heat it inherits has nowhere to
go and its chunk never sleeps; the plant and creature materials, which catch
fire or feel it; `stone`, so a hot quench delta drains into the rock it made;
and the water cycle's own four — `water` and `snow` at 0.08, `ice` at 0.1,
`steam` at 0.12, with `lava` deliberately near-insulating at 0.002 so a flow
carries its heat rather than shedding it into the first thing it touches.
Sand and gravel, which are never near fire in ordinary play, still have none.

**Any material that can be the target of `burns_into`, `melts_into`,
`boils_into`, or a reaction needs a real `heat_conductivity` for the same
reason** — and *so does anything the weather can chill*, which is the version
of the rule the ice work had to learn. A cell the storm cools and nothing can
warm sits permanently off ambient, `fire.rs`'s `must_stay_dirty` keeps its
chunk awake forever, and a world that has been snowed on never sleeps again.
Setting either `ice.ron`'s or `snow.ron`'s conductivity back to 0.0 leaves
chunks awake at a 1,200-frame budget on `weather.rs`'s own `thawed_world_sleeps`
test; with both, the world sleeps 42 frames after the front passes.

**Water's opt-in was measured against this paragraph's own warning and found
free.** The concern was exactly the regression above — water is everywhere, so
giving it a conductivity means `diffuse_heat` cannot take its cheap exit on a
large fraction of every sweep. On ascii's `stress: a full screen of sand and
water (serial)` the worst frame measured **102.0 ms as the baseline against
106.9–110.7 ms across paired runs**, which is inside the container's own
run-to-run spread on that scene: unmeasurable, not free-in-principle. The
historical 16 → 64 ms regression this section opens with was **not** the
per-cell diffusion at all; it was the `World::field_at` `HashMap` lookup that
version did per visited cell, and `src/sim/surface.rs`'s `field_wind_at` doc
carries the full measurement of why the surviving field read is a different
and much cheaper operation.

**Screenshotting the live app doesn't work on this machine** — found while
trying to visually confirm the fire tint below. This build's DXGI/wgpu
swapchain is invisible to Windows screen capture: neither
BitBlt/CopyFromScreen nor PrintWindow(PW_RENDERFULLCONTENT) can see the
client area, both returning solid black while the window chrome captures
fine. Worked around by having the app dump its own in-memory framebuffer
directly (`PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=<n>`, see
`save_framebuffer_png` in `src/main.rs`) — no OS capture involved. This is
what actually confirmed burning oil renders with a visible orange tint
(`render.rs`, a flat blend toward fire colour — not real emissive
lighting/bloom, which stays M6's job).

Burning cells render with that flat tint so M14's work is visible at all
before M6 exists; press `F` over painted material in the live app to ignite it
(a debug tool — M15 gives explosions a more physical ignition source).

## M7 status

Free particles, in [`src/sim/particle.rs`](src/sim/particle.rs) — a separate
system from the CA grid entirely (`ParticleSystem`, a plain `Vec<Particle>`
with float position and velocity), for the ballistic arcs a one-cell-per-frame
CA rule cannot express. Gravity, sub-cell substepping so a fast particle
cannot tunnel through a one-cell-thick wall between frames, and conversion
back into a normal CA cell — with the same shade-picking a paint stroke
uses — the instant a step would land on non-empty ground. Runs after the CA
sweep each frame (a landing check needs this frame's fully-settled CA state,
not last frame's) and does not touch the M13 field at all — no wind, no
coupling — since neither of this system's two callers (M15 explosion debris,
splash effects) need that to exist yet; adding a cross-system read only when
something concrete needs it is the same call M14 made about not coupling
every visited cell to the field.

**A real bug, caught by the fact every single test in the module failed the
same way.** The first version's substep function took `&Particle` and updated
local `x`/`y` shadow variables that were never written back — so a particle's
recorded position never advanced on any frame that did not end in a landing.
"Falls under gravity," "lands and becomes a cell," "doesn't tunnel," and
"conserves material count" all failed with the same shape (nothing ever
moved), which is what made it fast to diagnose rather than four separate
mysteries — changed to `&mut Particle` and it mutates for real.

Press `P` over painted material in the live app to throw a burst of it as
particles (debug tool, ahead of M15 giving explosions a real reason to call
`ParticleSystem::spawn`).

## M15 status

**Rebuilt after a diagnosis pass** — see
[`Reports/explosion-mechanics-diagnosis.md`](Reports/explosion-mechanics-diagnosis.md)
for the measurements. A blast now expands over `Tuning::duration` frames
rather than resolving in one; the fireball writes CA cell temperature (so it
glows, fades raggedly, and respects `flammability` — the old path reused the
debug force-ignite tool and set *stone* alight); debris can punch through
loose material (`particle::Particle::pierce`), which is what makes a buried
charge throw anything at all; and the crater is backfilled with smoke, giving
`SMOKE` its first producer anywhere in the simulation. Every number lives on
`explosion::Tuning`, live-adjustable under `O` and persisted to
`assets/explosion.ron`.

Explosions, in [`src/sim/explosion.rs`](src/sim/explosion.rs) —
`explosion::trigger`, built entirely from M13/M14/M7 triggered together, no
new simulation primitive. Per the plan: a pressure impulse and heat spike
into the field, then a radius of cells converted to thrown debris or vacuum
(a chance that falls off toward the edge — a direct hit at the centre
reliably throws debris, the outer rim of the same blast mostly just clears
without launching anything), then a fireball ignites the intact ring just
*beyond* the clearing radius. Press `X` over anything in the live app to
trigger one at the brush radius.

**Debris velocity comes from the local pressure gradient, not a naive radial
burst** — the one piece of physical grounding the plan specifically called
for, so a blast throws material away from the centre and *around corners*,
venting along a corridor rather than through its walls. Read directly from
the field the instant after the impulse is injected, before the field has
taken a single `field::step` of its own — the impulse has not propagated
anywhere yet at that point, so what actually produces the corner-aware shape
is checking `field_is_blocked` at each of the four neighbours and excluding a
blocked one from the gradient, the same exclusion `step_velocity` applies,
just computed directly here rather than waiting a frame for the field to do
it. A regression test walls off a corridor and confirms debris on the near
side never gets a strong push toward/through it.

**A real ordering bug, caught immediately by a test rather than shipped
quietly wrong.** The first version ran the fireball ignition *before*
clearing the blast radius — and the clearing step then unconditionally wiped
every cell in that same, larger radius to vacuum or debris, silently erasing
the fire it had just set. Fixed by moving ignition to target a ring *beyond*
the clearing radius instead of a smaller circle within it — which is also the
more sensible design regardless of the bug, since a fireball inside a hole
has nothing left to burn.

**A simplification this section used to carry is gone**: the first cut's
fireball reused `World::ignite_circle`, the M14 debug force-ignite tool,
which sets *any* material burning regardless of its `flammability` — stone
next to a blast glowed like oil. The rebuild described at the top of this
section replaced that with a real per-cell `flammability` roll
(`explosion.rs`'s ignition step documents the replacement as the point), so
stone is now immune the way `flammability: 0.0` says it should be. This
paragraph described the old behaviour for some time after the rebuild
landed — the two halves of one section disagreeing is exactly the failure
`scripts/docscheck.sh` cannot catch and a read-through can.

**Debris realism, added later (overnight run §6).** The corner-aware
gradient above was right about *direction* but every cell within roughly
one field tile (`world.field_at`'s own coarse-block granularity) read the
same quantized gradient, so a whole tile's debris launched with near-
identical velocity and read as a moving block rather than a scatter —
fixed with position-keyed jitter (`rng::jitter`) added to each cell's
launch velocity, scaled by that cell's own computed speed rather than raw
`strength` (a `* strength` term would have pinned every particle to
`particle::MAX_SPEED_PER_AXIS`'s clamp and made debris *more* uniform, not
less). Separately, every particle shared one flat `GRAVITY` with no
per-particle variation, so identically-launched particles fell in lockstep
forever — `Particle` now carries its own `drag`/`gravity_scale`, drawn once
at spawn from `ParticleSystem`'s own internal RNG stream and held for the
particle's whole flight.

## M6 deferral

Rendering upgrade (dirty-region texture uploads, a custom wgpu pipeline for
emissive light and bloom) stays parked. It needs live visual judgment of a
bloom kernel and light falloff that the framebuffer-dump technique can
confirm *exists* but not confirm *looks right* — that's true regardless of
who's watching, so unlike M5 below it isn't something a return to
supervision unblocks by itself. Picked back up whenever there's a session
built around watching the render output directly.

## M5 status

Built, once the user returned mid-session and picked "attempt it now" over
deferring further — see the [plan's progress log](PLAN.md#progress-log) for
the timeline. The design that shipped is **not** the plan's original sketch
(a single `unsafe` function handing out overlapping mutable 3×3 chunk
neighbourhoods). That version needed a second, harder proof — that reads
within one worker's own sweep still see that worker's own earlier writes,
the same way a direct `World::set` always has — before it could be trusted,
and it does not visibly announce itself as missing; it would have shipped
as a subtle, load-bearing gap. What's in `parallel.rs` instead needs **no
`unsafe` anywhere** in the simulation, at the cost of one extra serial merge
step per pass. (This section used to claim `grep -rn unsafe src/` returned
nothing but doc comments — stale: `main.rs` has since gained test-only
`unsafe { std::env::set_var }` blocks, which Rust 2024 makes unsafe and
`ENV_LOCK` serializes. Nothing in the sweep, the rules, or any simulation
path uses `unsafe`.)

**The proof, worked out by hand and then checked exhaustively by a test.**
Movement only ever moves a cell by `MAX_REACH` (32, exactly half of
`CHUNK_SIZE`) sideways within a row, or by exactly one cell in any of the 8
directions vertically/diagonally; fire's reach is a strict subset of the
latter. Enumerating which pairs of a chunk's 8 neighbours can be
simultaneously active under the mod-2 checkerboard (comparing `(cx%2,
cy%2)` pairwise) shows only four possible configurations — left+right,
top+bottom, and the two diagonal pairs — and each one's write footprint
lands in geometrically opposite, disjoint halves/rows/corners of the shared
chunk. No two workers in the same pass ever target the same cell.
`same_group_chunks_are_never_within_reach_of_each_other` in `parallel.rs`
checks this exhaustively across a wide neighbourhood rather than resting on
the derivation alone.

**That settled cross-chunk safety. It did not settle within-worker
ordering, and this is the part that actually broke once during
implementation.** `flow_sideways` (liquids/gases) can jump a cell directly
by up to `MAX_REACH` cells in one step — not just one — so a cell well
inside a chunk can still land across a boundary. If a queued cross-boundary
write were invisible to that same worker's own later reads, a second cell
processed afterward in the same sweep could scan straight past the
already-claimed destination (seeing the stale pre-pass snapshot instead)
and independently claim it too — two queued writes to the same position,
silently losing one at replay. `ChunkView::remote_writes` fixes this by
making `get` check the worker's own queued writes before falling through to
the shared snapshot, restoring exactly the property a direct write always
had serially. Caught by reasoning through the design, not by a failing
test — worth being honest about, since it means the *next* subtle gap like
this might not announce itself either.

**Design: exclusive ownership plus a deferred queue.** Each pass pulls its
active chunks (and field tiles) out of `World`'s maps into a plain `Vec` —
a `Vec`'s elements don't alias each other the way two `&mut` borrows into
one `HashMap` would, so handing each rayon worker `&mut Chunk` needs no
`unsafe`. A `CellSurface` trait (`surface.rs`) is what makes this possible
without forking the rules: `update.rs`/`fire.rs` are generic over it, so the
exact same movement and fire code runs whether given a plain `&mut World`
(serial) or a `ChunkView` (parallel) — there is only one implementation of
any rule to keep correct, not two to keep in sync. A `ChunkView` gives its
worker direct mutation of its own chunk/field tile, shared read-only access
to everything else, and three queues (cell writes, dirty-wake touches, field
writes) for anything landing outside its own bounds — replayed serially
through the ordinary `World::set`/`mark_dirty_at`/`add_heat_local` once the
pass's workers finish.

**A real, pre-existing M14 bug found along the way, not an M5 regression.**
`fire::diffuse_heat`'s "minimum one degree of progress" fix (added to stop
an isolated hot cell getting numerically stuck a few degrees off ambient)
turned out to force a whole-degree jump on *any* nonzero pull, even a
vanishingly small one — harmless for one isolated cooling cell, since its
only neighbours are `Cell::EMPTY`'s fixed ambient value, but a connected
mass of many cells cooling *together* (40 cells of ash from a burned-out
fire, the scenario M5's own stress test happened to be the first thing to
actually exercise) pull on each other by tiny nonzero amounts forever,
never landing on exactly zero. The fix now only forces progress when the
cell is actually outside `THERMAL_SETTLE_EPSILON` of ambient — tied to the
same "is this settled" question `must_stay_dirty` already asks, rather than
a separate, looser one. `fire.rs`'s
`a_connected_mass_of_cooling_cells_actually_settles` is the regression test.

**Per-chunk RNG, not one shared stream.** Each `Chunk` now owns its own
`Rng`, seeded from its coordinate, rather than every cell drawing from a
single `World`-level generator — a parallel worker can't safely share a
mutable generator across threads, and since this project's determinism
decision (see the plan) never required reproducible randomness, splitting
the stream costs nothing behaviourally that a shared one bought. `World`
keeps its own separate `Rng` for everything outside the sweep (painting,
explosions, particle bursts), unchanged.

**Verification beyond the unit tests.** Miri was the plan's own bar for the
`unsafe` seam it expected — with none shipped, there's nothing for Miri to
check; the equivalent bar here is the conservation/settling stress tests in
`parallel.rs` (a full multi-chunk screen of sand and water, a fire chain
spanning dozens of cells, a chunk touched only via a neighbour's dirty mark)
plus an independent review pass focused specifically on concurrency
correctness before this was committed.

## Plant lines merged: the genome, and the ecology

**Two long-running plant branches landed together** — `plant-substrate-v2`
/ `plant-genome` (the genome and root work) and `plant-ecology-design`
(litter, decay and herbaceous species). Both had been developing against a
trunk that moved 111 commits underneath them, so read this section's last
paragraph before trusting any plant number.

Built, on the genome side: a positional genotype slot map, so an individual
plant differs from its species mean in traits that are drawn once and
inherited; **root branching by primed sites** rather than an in-tick roll
(the old form demanded two steps' carbon in one tick and measurably never
fired); root laterals that carry an order; a stomatal reserve as a standing
throttle; and organism slot reclamation, so a plant that dies gives its
`organism_id` back instead of consuming one of 4,095 forever.

Built, on the ecology side: **shed foliage becomes `litter`** — a falling,
piling, burnable powder that weathers back into soil — rather than being
deleted, which is what makes a forest floor a cycle instead of an
accumulator. Decay became data (`Material::decays_into`) rather than an
ash-only special case, and decay sites are now scheduled when a chunk
*settles*, since a shed leaf falls and a bare coordinate does not follow
it. Two new species: **grass**, whose roots reinforce the soil they thread
so a rooted bank holds where a bare one spills, and **creeper**, a form
probe.

Three materials arrive with them: `litter`, `grassblade`, `grassroot`.

**What is not settled.** One test is red, and it is a calibration problem
rather than a visible one -- the stand was rendered before and after and
judged by eye as "a little different, fatter merging a bit more, not wildly
different", with the root change inside existing plant-to-plant
variability. The genome's root-branching slot
orders root mass in the right direction but by a fraction of the margin it
was calibrated at. Two explanations have been offered and **both have been
measured and falsified** — it is not the `field.rs` diff and it is not
weather (it never rains during that test's frame window). The cause is
genuinely not yet known. A separate suspicion, that grass and creeper were
running a retired root-branching model, was measured and **closed**: both
species' knobs fire, and zeroing grass's would have cost 60% of its root
mat. All of it, with the controls that produced each number, is in
`Reports/open-bugs-handoff.md` §A–§E. Do not re-derive those diagnoses, and
do not trust a plant constant without re-measuring it first.

## M16 status

Built: the active-site scheduler (`scheduler.rs`) and plant growth
(`plant.rs`) — moss and trees-with-roots. Grounded in real research rather
than invented rules; see `research/m16-plant-biology.md` for the full
citations behind every mechanism named below, and `PLAN.md`'s M16 section
for the condensed version. Debug tools: `T` plants a tree seed, `M` plants
moss, both at the brush position.

**The scheduler is a genuinely separate frame phase from the CA sweep, and
that separation is the entire point.** `World::step_active_sites()` runs
after the CA sweep, checking only sites that are actually due this frame —
cost proportional to how much is growing, not to world size. A settled
world with an actively-growing tree in it still reports `0` awake CA
chunks; `active_site_count()` is the separate number that says something is
still happening. `a_settled_world_with_a_growing_tree_still_sleeps_between_
growth_ticks` is the regression test for exactly this.

**That "checking only sites that are actually due" claim above was not
literally true until `pixel-physics-issues.md` issue #7 was fixed.** The
original storage was a `HashMap<ChunkCoord, Vec<ActiveSite>>`, and
`scheduler::step` drained and re-tested *every* pending site against `due`
every frame, rebuilding the whole map regardless of how many were actually
due — real cost proportional to total pending sites, not to how many were
due, at odds with the module's own doc. It also carried the engine's one
documented non-determinism source (`Reports/emergent-world-architecture.md`
§8b): a `HashMap`'s iteration order is randomized per process, so two sites
due on the same frame — two moss tips racing for the same empty neighbour,
say — could resolve differently run to run, which matters now that
same-build deterministic replay is a committed requirement (see the
decisions table near the top of `PLAN.md`). Both are fixed together: the
storage is now a `BinaryHeap<Reverse<ActiveSite>>`, a min-heap on
`next_frame` with `(x, y, kind)` as a fully deterministic tiebreak
(`ActiveSite`'s own `Ord` impl). `scheduler::step` peeks the minimum and
stops the instant it finds a not-yet-due site — true O(due · log n), no
full-structure rebuild, and the same result on every run given the same
sequence of schedule calls.

**Moss** spreads based on real ecology, not a "damp stone" rule invented for
convenience: real moss is poikilohydric (no waterproof cuticle, no internal
water regulation), so growth chance is gated hard on nearby water and
modulated by local shade (read from the M13 light field, since shade slows
evaporation — the actual reason real moss favours shaded surfaces, not any
directional pull).

**A real bug, caught by a test that expected growth and got almost none.**
The first version of moss growth required a candidate cell to have a
*solid* neighbour specifically. That works for the first ring of growth
along bare rock, but once a cell's only non-empty neighbour was
already-grown moss rather than raw stone, it read as having nowhere left to
grow — every growth front dead-ended one step after starting, producing a
single-cell-wide line frozen in place rather than a spreading patch. Fixed
by also counting existing moss as a growable surface, which is what real
moss does — it thickens by growing over its own earlier growth, not just
sideways along the original rock.

**Moss retrofitted onto the organism substrate (overnight run, section
8)**, per `Reports/organism-substrate-design.md`: moss's own damp/dry
spread chances and self-thickening rule above are unchanged in *behaviour*
— `moss_spreads_over_damp_stone_and_not_over_dry` and the new
`moss_thickens_into_a_patch_by_growing_over_its_own_earlier_growth` are
both regression tests against the exact numbers — but they're now data
(`assets/species/moss.ron`, a `Divide` behavior) read by a generic
`organism_tick` dispatch (`plant.rs`) instead of a moss-specific
`moss_tick` function. `Cell::organism_id` (added in §2, unused until now)
and a generational allocator (`World::push_organism`/`organism`, issue
#8's own "generational indices with a free list" direction) replace the
material-name check (`world.get(x,y).material != moss_id`) the old code
used to detect a disturbed tip. Trees and the worm were **not** retrofitted
in that pass, deliberately; see `PLAN.md`'s note on why moss alone was the
right scope for one. Both have since joined — the tree rewrite, and then
the worm (`Reports/creature-direction.md` stage 1), which retired
`CreatureState` entirely.

**Trees** use space colonization (already-committed citation) for canopy
shape, extended with two more mechanisms from the deep-dive research pass
rather than left as a bare attractor-seeking loop:

- **Auxin canalization** (Prusinkiewicz et al. 2009, PNAS) for branching and
  apical dominance — translated honestly, not oversold. Each tip carries a
  `channel` scalar that grows on a successful step and decays at a genuine
  dead end; a strong tip can spawn a branch, whose starting channel is
  *debited from the parent's* rather than created free, which is the one
  place one tip's channel actually depends on another's — real cross-tip
  interaction, not N independent scalars sharing a name with a citation.
  The bulk of the visible "one leader, suppressed siblings" effect, though,
  comes from two plainer mechanisms already needed elsewhere: tips share
  one attractor list that any tip's growth shrinks, and all tips (and now
  roots) draw from one flat energy pool. Real canalization is a richer,
  more actively self-reinforcing mechanism than shared-resource depletion
  plus one debit — see `plant.rs`'s own module doc for the precise
  breakdown, added after an independent review specifically flagged the
  original version's doc comment as claiming more than the code delivered.
- **Gravitropism vs. hydrotropism** (MIZ1 antagonism) for root growth
  direction — a hard switch, not a blend: gravity normally wins, but a
  strong local moisture gradient suppresses it so the root steers toward
  water instead.
- **Oscillator-based lateral root priming**, not a flat per-tick branch
  chance — a root only *considers* branching every few ticks, and only
  follows through if local water actually supports it.

**A real bug in root growth, not just a test-setup issue this time.** The
first version only let a root advance into `Empty` or `Powder` ground,
treating anything else — including the water it was growing toward — as
blocked and killing the tip on the spot. A root approaching any wet ground
died at its edge without ever drinking, which is backwards: water is
exactly what root growth is *for*. Fixed by giving `Liquid` targets their
own case — absorbed on the spot (same as the neighbour-drink check every
tick already does), tip advances into the now-empty space, no wood cell
left behind.

**Simplified deliberately, not by oversight** (documented in `plant.rs`'s
own module doc, not just here): cytokinin is a bonus push-pull signal from
the research, not a separate diffusing field; gravitropic setpoint angle is
a fixed constant per branch rather than a continuously-regulated flux
balance; canopy light competition is a direct light-field read with a
gentle phototropic lean, not Palubicki et al.'s full shadow-voxel
propagation (branches don't cast their own shadows into the field yet).

**Verified visually, not just by assertion.** `cargo run --release --example
ascii` has two permanent M16 scenes: a tree grown near a water source (shows
real branching, not a straight trunk, and a shrinking puddle as roots drink
it) and moss on a damp ledge next to moss on a dry one (the damp side grows
a visible 2D patch; the dry side stalls at its single seed cell — this is
what actually caught the growable-neighbour bug above, since the first
version's tree/moss both looked visually wrong before any test failed).

**Independent review found six more real issues, all fixed.** Beyond the
canalization doc-vs-code gap above:

- **Moss never went dormant.** Every tick unconditionally rescheduled the
  tip itself, even with zero growable candidates — a moss cell that becomes
  fully enclosed stayed on the active-site list checking every tick
  forever, exactly the unbounded cost the scheduler exists to avoid. Fixed
  with a stale-tick counter (`ActiveKind::Moss { stale_ticks }`): a tip
  tolerates a few consecutive empty checks (conditions can be transiently
  unavailable) but goes permanently dormant past a threshold. Nothing
  currently re-wakes a dormant tip if conditions change later — a real,
  accepted limitation, not an oversight.
- **`MaterialKind::Plant` didn't block the M13 field grid.** Only `Solid`
  cells marked a field tile blocked, so light/heat/pressure passed straight
  through a solid wood trunk as if it were open air — the exact bug
  `field.rs` already recounts fixing once for `Solid` alone, reopened by a
  second "static" material kind neither call site knew about. Concretely
  undermined this milestone's own moss mechanic (a canopy contributed zero
  shade). Fixed by blocking on `Solid | Plant` in both `field.rs`'s
  occupancy check and `world.rs`'s paint-brush guard (which had the same
  gap — the brush could erase a grown tree cell by cell, unlike stone).
- **Tree tips tunnelled through wood.** `wood` is one shared `MaterialId`
  across every tree, so comparing a growth target against it meant "is this
  *any* wood," not "my own already-grown wood" — a tip could pass straight
  through another tree's trunk (or curve back into its own), spending
  energy on a step that created nothing. Fixed by blocking on any non-empty
  cell, full stop.
- **Roots grew for free**, contradicting `TreeState::energy`'s own doc that
  every tip *and root* draws from one competitive pool — only tips actually
  checked it. Fixed by giving root growth its own (smaller) cost, debited
  from the same shared pool. This immediately surfaced a second bug of the
  same shape as the moss one: a root waiting on energy that will genuinely
  never arrive (every tip already starved to death, no water nearby) has
  no other way to stop, and became a second kind of immortal active site.
  Fixed the same way moss was — a starvation-tick counter that lets a root
  go dormant rather than wait forever.
- **Test coverage gaps**: no test distinguished "root moved because of
  gravity" from "root moved because of hydrotropism" (the water tank sat
  directly on the gravity path in the original test, so it would have
  passed even with the MIZ1 switch deleted entirely), and nothing checked
  that a tree ever actually produced more than one simultaneous tip.
  `roots_steer_toward_off_axis_water_via_hydrotropism` and
  `a_tree_can_produce_multiple_simultaneous_tips_via_branching` close both
  — the latter directly exercises `TreeState`'s private fields, which
  Rust's privacy model allows from `plant.rs`'s own nested test module.
  Writing the branching test surfaced one more real tuning bug: channel
  decayed on *every* tick spent waiting for energy, not just genuine dead
  ends, and since energy waits happened roughly 2-3x more often than
  successful growth at the original constants, channel could practically
  never climb far enough to cross the branch threshold — trees grew but
  essentially never branched. Fixed by decaying channel only on a true dead
  end (no attractors in reach), not on a temporary resource shortfall.

## M17 status

Built: destructible building with no polygon solver (`structural.rs`). Every
`Solid` cell can store, in `Cell::aux`, its distance in cells to the nearest
anchor — bedrock, or the world edge (the `Cell::OUT_OF_BOUNDS` sentinel
already used everywhere else a rule needs to treat the edge as a wall, so
both cases are one check). A cell whose distance exceeds its material's
`max_unsupported_span` converts to `breaks_into` (stone becomes rubble) and
falls under ordinary gravity like any other loose material — unless the
failing region is a detached piece big enough to read as a chunk, in which
case M8's `rigid::ChunkBody` flies it as one coherent falling body instead
(see the M8 section for which failures qualify and which do not).

**Confinement is what makes bulk material work, and it replaced the
exemption the milestone originally shipped with.** Distance-to-bedrock
alone is the wrong question to ask of bulk rock: a cell buried 500 deep
inside a mountain scores 500, vastly over any sane span, while being the
most supported cell in the world. Read literally that condemns the interior
of every mountain, which is why terrain was originally exempt from checking
altogether — and `Reports/worldgen-design.md` §6b calls that exemption "the
structural-integrity landmine," since the whole world was structurally
invalid while reading as anchored.

The fix follows from what this world *is*: a 2D vertical **slice** through
a 3D world (§0, "the world is 2D; the worldgen is 3D"). A real cave ceiling
is held up largely by rock out of plane, so requiring every cell to trace
an in-plane path to bedrock asks the slice to justify support it cannot
observe. A cell confined on every side the slice *can* see is anchored
outright, with a per-material `confinement_radius` — so the minimum
self-supporting thickness is `2r + 1`, authored rather than hardcoded. That
is §7's open problem ("a noise-defined ceiling has no bounded thickness")
answered rather than worked around, and it is also honest rock mechanics:
confinement is what gives rock its strength, and failure initiates at free
faces.

**A step's cost now depends on which direction the support comes from.**
The relaxation charged a flat 1 regardless, so a 1-cell tower snapped at
exactly the reach a 1-cell cantilever managed. Rock is strong in
compression and weak in bending and tension;
`support_cost_below`/`_beside`/`_above` split that three ways (stone takes
0/1/3), so a wall stands to any height while an overhang still fails at its
span. All three default to 1, the behaviour they replaced.

**Terrain is now genuinely checked, not exempt.** `app::build_terrain`
places real bedrock and calls `structural::compute_world_distances`, a
single converged Dijkstra run at generation — deliberately *not* routed
through the active-site scheduler, which caps at `MAX_SITES_PER_FRAME` and
would spread a world's terrain over many frames, during which the
count-to-infinity dynamic can push a cell past its span before the true
anchor value reaches it. That is §6b's predicted global collapse arriving
as visible crumbling on frame one. Measured at 9.1 ms for the 512×320
terrain's 6,616 solid cells, against 0.4 ms to build the same terrain
without it, reported permanently by `examples/ascii.rs`. The relaxation is
the cheap half; the whole-world seeding scan dominates, which is issue #5's
hashed-`World::get` pattern in a new place and becomes per-chunk under M10.

The three floating ledges stand on confinement alone, with no support
pillars — `generated_terrain_is_structurally_real_and_still_stands` asserts
both that distances were genuinely computed and that nothing crumbles.

**Recomputation reuses the M16 active-site scheduler unchanged** —
`ActiveKind::StructuralCheck` is a third kind alongside moss and tree/root
growth, dispatched to `structural::tick` instead of `plant::tick`. Distance
is a shortest-path relaxation (`d = 1 + min(solid neighbours' d)`, anchors
at `d = 0`), recomputed one cell at a time and only propagated to a cell's
`Solid` neighbours when its own value actually changes — exactly the same
"stop rescheduling once stable" shape a moss tip with nowhere left to grow
already uses, which is what keeps a cascade's cost bounded by the size of
the affected structure rather than the size of the world. Ticks are paced
5 frames apart (`STRUCTURAL_TICK_INTERVAL`) rather than resolving a whole
cascade in one frame, which is what makes a collapse read as progressive —
see `cargo run --release --example ascii`'s bridge scene for what that
actually looks like: a 30-cell bridge anchored at both world edges stands
whole (stone's span reaches every cell from one end or the other), then
erasing the right anchor collapses everything past the span from the
surviving left anchor while the near stub stands.

**That scene's width tracks the span, and has to.** It was 7 cells against
a span of 3. Confinement and direction-weighted steps raised the span well
past that, and at 7 wide the scene silently stopped demonstrating anything
— cutting the anchor removed one cell and the remaining six stood, since
all of them were now comfortably in reach of the left edge. It printed a
healthy-looking bridge twice and proved nothing, with every test green.
Caught by looking at the output. Keep it wider than twice the span.

One genuinely interesting emergent property, not deliberately designed in:
a `Solid` structure with **no** path to any anchor doesn't stay at distance
0 by default (which would misread "never checked" as "already anchored").
Once *any* part of it is disturbed and enters the scheduler, cells with only
each other to reference relax upward every round-trip with no true zero
source to converge toward — the same shape as the "count-to-infinity"
problem well known from distance-vector routing — climbing without bound
until every cell's distance exceeds its span and the whole structure
collapses, exactly the outcome a real floating, unsupported structure
should have. `saturating_add` keeps the arithmetic safe as the value climbs
toward `u16::MAX`.

**Guard against the burn-timer conflict.** `Cell::aux` is a tagged union —
while a cell is burning, `aux` is the burn countdown, not a distance, and
`Cell::set_aux` `debug_assert`s against writing it during a burn. A
structural check on a burning cell defers (reschedules itself) rather than
touching `aux`, and picks the distance question back up once the fire
either goes out or the cell is consumed.

`Plant` was originally out of scope for this milestone, and this section
went on saying so after it stopped being true. Structural integrity now
covers `Plant` as well as `Solid` (`is_structurally_interesting` matches
both): `wood.ron` carries real span/`breaks_into` numbers, and a burnt-away
trunk base brings the rest of the tree down the same way cutting a stone
bridge's support does — see the field-grid section above for how that
landed.

Independent review (the same standing per-milestone practice as M5/M13/M16)
found one real bug before commit: the neighbour-scanning loop that computes
`min_neighbour` read a burning `Solid` neighbour's `aux()` as if it were a
distance — but `aux` is a tagged union, and while `is_burning()` is true it
holds the burn-timer countdown instead. Reachable in real play, not just in
theory: `explosion::trigger`'s fireball step calls `World::ignite_circle`,
which deliberately ignores `flammability` and force-ignites any non-empty
cell in the ring beyond the blast — including stone — so a non-burning
stone cell checking a burning stone neighbour is an ordinary consequence of
setting off an explosion near a wall. Depending on where the timer happened
to be, this could either mask a real break (a burn timer counting down
through a small value reads as "well supported") or shatter a
perfectly-supported cell for no structural reason (a fresh, large timer
value reads as "extremely far from any anchor"). Fixed by excluding burning
neighbours from the relaxation and deferring the check (rescheduling
without writing `aux`) if every `Solid` neighbour that exists happens to be
mid-burn, rather than either reading their timers or treating "temporarily
unusable" the same as "no support at all." Regression test:
`a_burning_solid_neighbours_burn_timer_is_never_read_as_its_distance`.

**Rock has a grain, and a blast wakes it** (`fracture_field.rs`). Where the
planes stone is disposed to part along run is a fixed, position-keyed
property of the place rather than a shape drawn at the moment of the event,
so a second charge on the same ground retraces the first one's breaks rather
than drawing new ones. A blast severs the boundaries of the Worley domains
around it — closed, straight-sided polygons meeting at three-way junctions —
and a *confined* failure reveals the same fabric where it stands. The
severing rule is exact and needs no threshold: an edge is a joint iff its two
cells sit in different domains, which is the domain's own boundary on the
4-connected grid, so a domain whose boundary is fully severed is enclosed by
construction rather than by luck. `Reports/explosion-stone-review.md` §15-17
is the design record, and `Material::joint_spacing` (`0.0` = not jointed) is
what keeps this to brittle rock: sand, soil, gravel and snow are already the
fragments.


## M18 status

Built: M18 Phase 1, a cell-based creature — a burrowing worm (`creature.rs`).
Grounded in real animal behaviour research, not invented rules; see
`research/m18-creature-biology.md` for the full citations behind every
mechanism below, and `PLAN.md`'s M18 section for the condensed version.
Debug tool: `J` plants a worm at the brush (moved off `W` when the gnome
claimed WASD).

**A worm is one `MaterialKind::Creature` cell, dispatched from the M16
scheduler exactly like a plant tip** — an `ActiveKind::Creature { organism }`
variant carrying the same generational handle a plant's site does, checked
every `WORM_TICK_INTERVAL` (6 frames). It was originally a raw index into a
parallel `World::creatures` vector; that scheme is gone
(`Reports/creature-direction.md` stage 1) and a creature is now an organism
like any other — state in `OrganismState`, identity in `Cell::organism_id`,
species in `worm.ron`, and its slot returned on death. `Creature` is excluded from the CA sweep's movement dispatch, the
same as `Solid`/`Plant` — a worm is relocated explicitly, by writing through
the ordinary `World::get`/`set`, never by the sweep itself.

**Fire needed zero creature-specific code.** `fire.rs` already applies
ignition, burning and burnout to every material kind uniformly, purely from
`.ron` data — so giving `worm.ron` real flammability numbers
(`flammability: 0.6`, `burns_into: "corpse"`) gets "a creature that catches
fire and dies" entirely for free. `creature.rs` only owns the part fire.rs
*can't* provide: choosing to move, including choosing to flee before contact.

**Three mechanisms translated directly from the research, not invented:**

- **Burrowing cost from substrate physics, not a material whitelist.**
  Real peristaltic burrowing only works within a narrow substrate-resistance
  band (Kurth et al. 2018), and the Namib golden mole spends ~26x more energy
  burrowing loose sand than moving on the surface. `move_cost` ties a
  `Powder` target's burrow cost to its own already-tracked `density`
  (`WORM_MOVE_COST_OPEN + density * WORM_BURROW_DENSITY_COST`, tuned so sand
  costs ~20x open ground) rather than a flat per-kind multiplier, so a denser
  or looser future granular material costs proportionally more or less
  automatically. `Solid`/`Liquid`/`Plant`/other `Creature` cells are all
  impassable — worms are not modelled as aquatic this milestone.
- ***C. elegans*-style thermotaxis for fire avoidance.** The real mechanism
  — a single thermosensory neuron comparing current temperature against a
  remembered set-point, fleeing down-gradient once above it — maps directly
  onto reading the local M13 ambient-temperature field
  (`WORM_HEAT_THRESHOLD_ABOVE_AMBIENT`) and moving toward whichever reachable
  neighbour reads coolest. Deliberately both the accurate model and the cheap
  one; no invented complexity needed.
- **An energy budget replacing random wandering.** Depletes on every move
  (more on burrowing than open ground), is partially replenished by
  "eating" the powder it burrows through (`WORM_ENERGY_FROM_EATING`), and
  reaching zero is what kills a permanently-trapped worm — no separate
  dormancy counter needed the way `plant.rs`'s `MOSS_STALE_LIMIT` is, since
  starvation is itself the bound. A worm eating sand ahead and leaving the
  same material behind it (`worm_tick`'s move/excrete step) is a literal
  translation of real earthworm ingest-ahead, cast-behind burrowing, not
  just a metaphor.

**Known simplification, not yet built:** the Marginal Value Theorem's actual
patch-leaving rule (leave once local intake drops below the environment's
running average) needs a maintained average-intake estimate this first cut
doesn't keep. What's here instead — prefer burrowing into powder over
drifting through open space whenever both are available — captures "the
worm has a reason to move" without that bookkeeping. Multiple interacting
creature kinds (Wa-Tor-style predator/prey) and the shared
resource-gradient primitive for slime/fungus creatures (research file,
sections 4–5) are explicitly out of scope for this first cut.

A real test-quality bug was caught and fixed while writing this milestone's
own tests, not by an external review: three tests filled their whole
terrain grid with sand *before* calling `plant_worm` at a position already
inside that fill, so `plant_worm_seed`'s `is_empty` guard silently no-op'd
and no worm was ever created — the tests were passing vacuously (a stone
wall simply never disturbed can't have been "entered" by a worm that never
existed). Fixed by explicitly clearing the seed cell before planting in
each affected test. A second, separate bug in the fire/corpse test: a newly
formed `corpse` cell (`kind: Powder`) is free to fall *or roll* under
gravity the instant it's created, in the same CA-sweep frame that created
it (`fire::update` runs before movement dispatch for the same cell) — a
floor placed only directly underneath blocked the straight fall but not a
multi-cell roll onto adjacent open ground followed by a fall from there,
found via a throwaway diagnostic print rather than by inspection alone.

**Independent review** (the same standing per-milestone practice as
M5/M13/M16/M17) found one critical bug before commit, confirmed by the
reviewer's own reproduction and now guarded by a regression test:
`worm_tick`'s moving branch always rebuilt the worm's cell from scratch
(`Cell::new(worm_id, ...)`), which silently cleared `FLAG_BURNING` and the
burn-timer `aux` the instant a burning worm's next scheduled move came due.
Since `WORM_TICK_INTERVAL` (6 frames) is far shorter than `worm.ron`'s
`burn_duration` (60), a burning worm normally gets several movement
decisions during any single burn — this fired in the *ordinary* case, not
an edge case, and a worm effectively survived every fire it caught simply
by moving. The established codebase precedent for exactly this situation —
`structural.rs`'s `if cell.is_burning() { defer, don't touch aux }` guard —
had not been applied here. Fixed the same way: `worm_tick` now defers
(reschedules without acting) whenever its own cell is burning, leaving
`fire.rs` (which runs independently every visited CA frame) to finish
deciding the worm's fate undisturbed.
`a_burning_worm_keeps_burning_even_when_its_movement_tick_comes_due` is the
regression test — deliberately built on open ground rather than sand, since
a burrowing worm starves from movement cost alone within the test's frame
budget regardless of the fire bug, which would have made a sand-field
version of this test pass vacuously either way; verified to actually fail
without the fix before being kept. A related, lower-severity finding from
the same review was fixed alongside it: a worm could burrow directly into
an actively-burning neighbour (candidates were filtered by material kind
only, never by the target's own `is_burning()` state) — physically backwards
for a mechanism sold as fire avoidance, though not a crash or data-corruption
bug. Two smaller, non-blocking observations were also addressed: a
`debug_assert` guarding `push_creature`'s `u16` index against silent
wraparound past 65,535 live creatures (since superseded: `push_creature` is
gone, and the organism allocator's 12-bit index has its own bound and a
generation-wrap counter), and a positive existence assertion added to
`a_worm_burrows_through_sand_but_never_enters_stone`, which previously only
checked the stone wall was undisturbed and could have passed vacuously the
same way the three tests above did, for an unrelated reason, in the future.

### M18 S1–S4: the creature economy, and an edible forest floor

Merged from `creatures-m18` on 2026-08-23. `Reports/creature-evolution-plan.md`
holds the staged plan and the "As built" measurements; **every S4 number in it
predates this merge and is superseded by the numbers here.**

**S1–S2 — the genome.** The heritable genome grew from 248 slots to 584 on a
scheme that can extend on any axis without re-keying what is already there,
with a manifest hash so a stale genome cannot be read as a fresh one.
`BRAIN_OUTPUTS` is 10. `synapse_cost` became `synapse_fraction`, a fraction of
`start_energy` rather than an absolute: as an absolute it was silently a
different tax every time anything changed the energy budget, and one harness
spent 80% of a creature's life on thinking without anyone noticing.

**S3 — food is worth what it is.** Nutrition used to be `eat_energy`, a
constant of the *eater*, so a corpse was worth whatever bit it. Worth now
lives on the material (`food_energy`, `food_class`), except for the one case
that genuinely varies — a corpse is worth what the animal was made of, and
carries that per cell in `Cell::aux` (`worth_in_aux`). `body_energy` is
granted at spawn and can never be spent, so a creature starved to exactly 0
still leaves food behind. The `EnergyLedger` was reworked into two stocks,
live and meat, which closes the pump §13l recorded: the old ledger balanced
while conjuring 300 joules per bite.

**S4 — the canopy feeds the floor.** All three abscission sites write `litter`
instead of erasing the leaf, and litter is on the ants' menu. A shed leaf is
carried down through its own crown to where it would have landed, rather than
written where it hung: writing in place looked equivalent and was not, because
a crown catches its own leaf fall — 3,825 of 4,330 standing litter cells were
resting on plant tissue. Litter rots back to soil on `decay.rs`'s channel at
its own per-material rate, so the floor reaches equilibrium instead of
integrating the canopy's shedding forever.

**What it costs, measured paired in one session on one machine.** The colony
scene at 12,000 frames went **mean 3.121 ms → 2.979 ms** — litter that reaches
the floor and rots is *cheaper* than the bare canopy it replaced. That is the headline
result and it is not a rounding artifact: the same mechanism measured **+45%**
(1.875 → 2.714 ms) on `creatures-m18`, where litter never rotted and simply
accumulated. Worst-frame moved 46.1 → 66.4 ms and is *not* quoted as a
regression — worst-frame spread on identical binaries has been measured at
3.5x here, so only the mean is usable.

**Known limitations.**

- **The floor feeds the colony, and the colony stops ranging.** Same run:
  deliveries 222 → 260 (+17%), but moves 13,980 → 9,595 (−31%), nest-visits
  6,014 → 3,852 (−36%), digs 79 → 43 (−46%). This is the owner's stated
  constraint arriving as a measurement — a complex system whose visible
  result is ants sitting still eating fallen leaves is not wanted. Litter's
  rot rate is therefore a **design** knob, not a performance one, and the
  values here (damp 0.5 / dry 0.1) are a starting point pending a verdict.
- Most litter is **dry**, whatever the weather: the moisture field is sampled
  at the litter's own block, which is air, so only 2–7% of standing litter
  reads above the damp gate. The dry rate is the one that governs.
- `decay_chance_*` is resolved from a serde default at parse time rather than
  a `0.0`-means-shared sentinel, so `decay_chance_dry: 0.0` means what it says.
- **"Against plant" is not "stuck in a tree", and the probe used to imply it
  was.** 39% of standing litter has a live organism cell underneath it, and
  almost all of that is a drift piled against a trunk *at floor level* — 88%
  of all litter sits within four rows of the ground and none of it is more
  than 32 rows up. A litter cell is a grid cell and cannot go behind a tree
  the way the gnome can: he is an entity with his own collision rules, it is
  a material, and two materials cannot share a cell. So resting on a trunk
  base is `litter.ron`'s 42-degree friction angle working, not failing.
  `litter_probe` now names the column `against-plant` and refuses to be read
  without the height bands.

**Two follow-ups the owner judged by eye, after the merge landed.** Litter's
palette is warmer and lighter than the browns it shipped with: the original
set was deliberately close to soil so a layer would read as ground *texture*,
and at play zoom that lost — the floor was there and could not be seen. Chosen
from a blind A/B. And `LITTER_FALL_REACH` went 64 → 512, because a grown crown
tops out ~125 rows above the ground and the walk was running out *inside the
canopy*: the tallest trees, whose leaves have furthest to fall, were the ones
whose litter never reached the floor. Litter against plant 44.4% → 39.3%,
within three rows of the ground 29.5% → 35.4%.

**Foraging range, and why `nest_visits` was never the guard it read as.**
`CreatureStats::nest_visits` guards on `since_nest > 0`, and `since_nest` is
incremented unconditionally every tick — so the guard is false exactly once
per lifetime and the counter scores every move made while nest-adjacent. It
counts loitering. `assert!(nest_visits > 0)` therefore passes trivially for a
colony that never leaves the nest mouth, which is the failure it looked like
it was guarding.

The replacement is a spatial excursion depth re-anchored at every nest contact
(`OrganismState::forage_anchor`), booked as `forage_trips` /
`forage_depth_sum` / `forage_depth_max` and a threshold-free cumulative
profile, `forage_reach`. Measured on the foraging scene at 12,000 frames after
the merge: **98 trips, deepest 18 cells, mean depth 10.3**, profile
`[3858, 475, 185, 98, 1, 0, 0, 0]`. The bars are set from that with headroom
(a seventh of the count, under half the depth) because outcome spread here is
large. `examples/forage_probe.rs` pairs the scene against a sessile control —
one ant, a nest, no food — and neither arm is worth anything alone.

**`Material::insubstantial` bought zero cells on `wood`, and the zero is
recorded.** The gnome runs through leaf litter with no wade drag, on the
owner's direct instruction. It does not move `scripts/acceptance.sh`'s `wood`
case, which reads **98 travelled against a bar of 200 on all three of**
`origin/main`, the merge, and the merge plus the flag. Earlier notes citing 34
predate main's own plant work and no longer reproduce; the residual is tree
architecture, not litter depth (`Reports/open-bugs-handoff.md` bug Y).

## UI improvements — overnight run, section 9

The engine's first on-screen text, and everything built on top of it.
Before this, `render.rs`'s own comment on the window title bar called it
"cheaper than rendering text" — every status readout lived in the OS
window chrome, invisible the moment the window lost focus or a screenshot
cropped it out.

**`hud.rs`**: a fixed-width 5x7 bitmap font, deliberately *not* the plan's
originally-sketched full ASCII 0x20-0x7E range (95 glyphs). Hand-authoring
95 accurate glyphs with no reference font to check against risks silently
shipping wrong bitmap data for characters nothing exercises before they're
ever seen; scoped down instead to what the HUD actually needs — space,
`A`-`Z`, `0`-`9`, and a small punctuation set — with HUD text upper-cased
internally as the direct consequence (no lowercase glyphs exist). A visual
check (rendering sample text to a PNG and reading it, not just trusting
the hand-transcribed bit patterns) caught one real gap before commit:
`[`/`]`, used by the help overlay's own brush-size line, had no glyph at
all and rendered as an invisible gap — fixed, with a regression test that
checks every character the module doc claims to support actually lights a
pixel, confirmed via revert to fail against the original omission.

**Zoom** (`=`/`-`, `Renderer::zoom`/`zoom_out_stride`): one continuous
scale across two fields rather than two independent controls — zooming
out first counts down a sample stride (seeing more world per pixel, up to
4 cells per pixel) before magnification (`zoom`, up to 8 screen pixels per
world cell) ever engages the other way, so the key pair reads as a single
"more/less zoom" control. `screen_to_world`/`world_to_screen` both use
`div_euclid`, not `/` — the same reasoning `ChunkCoord::containing`
already established: a screen position left of the camera must floor
toward negative infinity, not fold onto the same world cell a position to
its right would.

**Brush label** (always on), **brush outline preview** (a midpoint-circle
outline at the cursor, scaled to match the zoom level so the ring is
actually the size a click would paint), **hover inspector** (`I` — cell
material/temperature/burning state plus every M13 field channel at the
cursor), **material palette** (`Tab` — a swatch row, current selection
outlined), and **keybind help** (`/`, shown as `?`) round out the pass.
The field overlay (`V`, cycling pressure/temperature/light/moisture) tints
every pixel by the selected channel, including empty cells — a field
reading exists over vacuum the same as anywhere else, so the overlay
routes through both the empty- and non-empty-cell paths in
`Renderer::cell_colour` rather than only the latter.

## Live tunables panel — overnight run, section 10

A generic `(category, name, value, min, max, step)` registry
(`tunables.rs`), built on section 9's text primitive rather than a bespoke
UI per subsystem. Only `Material`'s finite `f32` fields register this
round — integer fields and anything still at the "never" (`f32::INFINITY`)
sentinel are deliberately skipped. `O` opens it; adjusting a value applies
immediately to the live registry (felt next frame); `Enter` saves back to
the `.ron` file via a targeted text-span edit that never touches anything
but the one value, so hand-written comments (`oil.ron`'s own header, for
one) survive; `Esc` closes without saving, though the live-adjusted value
stays in effect for the session regardless.

Live PNG verification against the real asset files (not just hand-built
test strings) caught two bugs the unit tests had missed: saving failed for
most materials, because most files only write the fields that differ from
`Material`'s own defaults (`stone.ron` never mentions
`heat_conductivity`), and the original save path treated "field not in the
file" as an error rather than the common case it actually is — fixed to
append the field before the file's closing paren instead. Separately, the
panel's last row overlapped the save-confirmation message whenever both
happened to land on the same pixels — fixed by reserving that footer space
unconditionally rather than only when a message happened to be showing.
An independent review then caught two more: the value-span search that
finds `field: value` to replace didn't know about `//` comments, so a
value written with a trailing inline comment would silently lose it on
save; and the same blind spot existed in the "does this file need a
leading comma before the appended field" check. Both fixed by making the
relevant scans comment-aware, each confirmed via a test that failed
against the pre-fix code first.

## Rendering performance — overnight run, section 11

`Renderer::draw` skips recomputing pixel colour for any screen region
whose underlying chunk hasn't changed, instead of redrawing all 512×320
pixels every frame regardless of what's settled. The originally-planned
route (GPU dirty-region texture uploads) turned out to be blocked by the
`pixels` crate's own architecture — its `render`/`render_with` always
re-upload the whole frame internally, with no accessor for the underlying
surface to drive an alternative path — so the actual fix landed CPU-side,
where the real cost already was. Measured on the existing full-screen
sand benchmark: 6.6ms → 0.0ms worst frame once the scene settles. A real
bug surfaced twice while building this (a settled-chunk check done only
at draw time misses a chunk that changes and re-settles *between* two
draws, which the engine's own catch-up ticking can do on a slow frame;
the initial fix for that still had a one-tick lag for a chunk's very
first write) — both found live, both fixed, both covered by regression
tests. The bloom/emissive shader half of M6 remains deferred, unchanged
from before — that one genuinely needs a human watching it render, which
this section's fix doesn't touch.

## M8 status — started, not complete

The plan's own words for this milestone: "the largest single milestone —
treat as its own project" and "the risk concentration; deferred as far as
it sensibly can be." Its full pipeline — connected-component labeling →
marching squares contour → Douglas-Peucker simplification → `earcutr`
triangulation → a `rapier2d` collider → erase-step-re-rasterize every frame
— is real, separate engineering at each stage, not something to rush
unsupervised at the tail of a long session. This milestone is **only
started**, deliberately: `rigid.rs` implements the first stage,
connected-component labeling, and nothing past it.

**Built:** `rigid::label_component` — a 4-connected flood fill over `Solid`
cells from a seed position, returning every reachable `Solid` cell (never
diagonal — two blobs touching only at a corner are not one physical body,
`diagonal_only_contact_does_not_connect_two_components` guards this),
capped by a `max_cells` parameter so labeling a component that happens to
touch the sandbox's own contiguous floor doesn't walk the whole world for
one request.

**A real bug was caught while writing this module's own tests, before any
external review:** `Cell::OUT_OF_BOUNDS` (returned for every out-of-bounds
read) has `material: BEDROCK`, and bedrock's `MaterialKind` is `Solid` — so
a naive flood fill treats the entire world boundary as one giant connected
"wall," and a component touching any edge floods along that wall until it
hits `max_cells` rather than stopping at its own true extent. Caught by
`a_component_smaller_than_the_cap_returns_its_true_size` unexpectedly
reporting 1000 cells for a 5-cell blob. Fixed with an `is_body_material`
helper (`kind == Solid && material != BEDROCK`) applied at both the seed
check and every neighbour considered during the flood fill — the same
single-check trick `structural.rs`'s anchor detection already uses to treat
literal bedrock and the world edge as one case. Two dedicated regression
tests guard it directly:
`touching_the_world_boundary_does_not_flood_along_the_edge` and
`does_not_include_literal_bedrock`.

**Also built:** `rigid::trace_contours` — the pipeline's second stage,
boundary/contour extraction from a labeled component. Not the classic
marching-squares algorithm (that walks a continuous scalar field and needs
a 16-case lookup table plus a saddle-case tie-break specifically for
interpolation ambiguity); this input is already a binary occupancy grid, so
the equivalent, unambiguous approach is directed boundary-edge stitching —
see the module doc for the full reasoning. Handles holes correctly with no
special-casing (a hole boundary falls out already wound the opposite way
from the outer boundary), verified via the shoelace formula across a
filled rectangle, a concave L-shape, and a shape with a hole.

**Independent review found one severe bug before commit**, confirmed by the
reviewer's own reproduction: a "pinch point" (two cells touching only at a
shared corner, connected to each other only through a longer 4-connected
path elsewhere) didn't just produce the documented "wrong-but-closed"
contour — it made the inner boundary-walk loop **forever**, growing memory
without bound. The corner's start-point collision silently drops one
lobe's exit edge, rerouting that lobe's walk into the *other*,
already-closed lobe's cycle, which never revisits the dropped lobe's own
start point — so the loop's `n == start` guard alone never fires. Fixed by
also breaking when the walk revisits *any* already-visited point, not just
its own start, turning the hang into what the module doc always claimed
happened (a bounded, wrong, terminating ring — pinch points are still not
correctly resolved, only safely bounded). Regression test
`a_pinch_point_terminates_rather_than_hanging` runs the repro on a
background thread with a timeout, so a future regression fails the test
rather than hanging the suite.

**Also built: chunk bodies — the milestone's first gameplay wiring.**
`label_component` and `trace_contours` were pure queries with nothing
calling them; a structural failure now promotes to a `rigid::ChunkBody`
that falls as one coherent piece, displaces what it passes through, lands,
and re-rasterizes back into the grid as ordinary terrain. It runs in its
own serial phase beside `step_liquid_bodies` because
`Reports/coupling-research.md` §4 is explicit that a body spanning two
same-parity chunks would write to both and break `parallel.rs`'s
write-disjointness proof.

The trigger uses a *failure* predicate rather than `label_component`:
labelling connected solid from a failing cell would sweep in the whole
mountain the overhang is still attached to. `structural.rs` has already
written every cell's distance by then, so the flood is over "solid,
unowned, and past its own span."

**Only a detached region becomes a chunk; progressive span failure does
not, and cannot** — found by counting, not by looking, and the distinction
is worth keeping. `filmstrip`'s `mine` scene reads exactly like coherent
slabs coming down and reported `bodies 0` throughout: the picture said the
feature worked while the count said it had never fired. A mined roof is
still held at both ends, so its cells sit at different distances and the
failure wavefront walks inward one cell per `STRUCTURAL_TICK_INTERVAL`; at
the instant the first crosses its span the next is still short, and the
region is one cell. A detached shelf has no anchor at all, so its cells
climb in lockstep and cross together — `filmstrip`'s `snap` scene promotes
one 54-cell body that falls, lands, and becomes terrain again. That is the
physically right split rather than a gap: a roof held at both ends is a
span crumbling, not a chunk that fell off. Chunks from *mining* would need
reachability computed forward instead of read from the cached distance,
and are deliberately not attempted.

**Still not built, in pipeline order:** Douglas-Peucker simplification,
`earcutr` triangulation, a `rapier2d` collider, and *continuous* rotation.
Bodies translate and tumble in **quarter turns** — deliberately right-angle
only, since a cell grid cannot hold a slab at 37° without the resampling
leaks the re-rasterization pitfall below describes, while a 90° transform is
exact and can never gain or lose a cell (`rigid::ChunkBody::spin`'s own doc).
No new dependency (`rapier2d`, `earcutr`,
`glam`) has been added to `Cargo.toml`, deliberately: what a falling chunk
needs is gravity, a grid fit test and a settle rule, not a constraint
solver, and everything except the integration step is shared with a rapier
version if one ever lands. Two facts that outlive that choice, both from
`Reports/coupling-research.md`: rapier's `enhanced-determinism` cannot be
combined with its `parallel` feature (§0.2), against a determinism
requirement `PLAN.md` reversed to *required*; and §3's resistive-force
coupling and §5's buoyancy both remain unbuilt, so a body currently sits on
sand as though it were stone.

## M9 status — the gnome

Built: a summonable character (`U`), in `src/sim/player.rs` — a kinematic
body over the cell grid, not a rigid-body import. He runs (`A`/`D`), jumps
with tap-for-hop, hold-for-height (`W`), wades powder slowed in proportion
to how deep he is in it — shouldering past the few loose grains that a
canopy or a dig leaves at chest height, while a bank several cells abreast
still stops him — swims with a surface-exit window, is buried and
digs out, rides a falling chunk body rather than being left behind by it,
and digs an aimed bite along the cursor (`Tool::Dig`, the yellow ring).

The feel is data, not code: `player::Tuning` behind `O` -> PLAYER
(persisted to `assets/player.ron`), with the three whole-feel families that
only play can settle behind named runtime selectors (`F3` movement, `F4`
water, `F2` spoil) — the "ship a runtime selector rather than choosing"
convention applied to a character. What he is like to play is
[`wiki/the-gnome.md`](wiki/the-gnome.md); the build plan he followed is
`Reports/m9-gnome-character-plan.md`.

**Known limitation:** he cannot get over a bank of loose powder. `wade_rows`
lets it reach his knee and no higher, and `step_up` mounts a *ledge* — a
tall powder face fails the same test at every lift — so a forest floor that
piles above knee height across his width is terminal, and he has to dig
through or go round. Measured and left visible rather than tuned away:
acceptance case 8 clears a wood at 357 cells against a bar of 200, while
case 8b at the worst-grown stand gates 40 against a measured 50. The gap
between those two bars is this limitation. Bug C1 in
[`Reports/open-bugs-handoff.md`](Reports/open-bugs-handoff.md) holds the
numbers and three candidate directions, none attempted.

## M10 status — the worldgen half

M10 as planned is the infinite streaming world. What has landed is its
worldgen redesign (`Reports/worldgen-design.md`): `src/worldgen/` builds a
bounded-but-large world as a 2D playable slice through coarse 3D worldgen —
two to five regions across the width, each with its own height, ruggedness
and dryness; escarpments where they meet; layered, gently folded rock; soil
with a real vertical profile that thins with slope; a water table with
standing pools where the land dips below it; buried pockets, scree,
overhangs; seeded plant cover, clustered rather than scattered. Worlds
arrive settled and structurally real — nothing moves until something moves
it. `F6`/`F8` roll seeds, `F7` cycles presets, and the same seed and preset
rebuild the same world within one build. `tests/worldgen.rs` guards it;
[`wiki/the-world.md`](wiki/the-world.md) describes what a player sees.

Streaming itself — chunks loading and unloading past the bounds — has not
started; `ChunkCoord`'s reserved slice-identifier (issue #11) is the one
piece of it already spoken for, and it must land before the save format.

## Weather status

Built (`src/sim/weather.rs`): weather as a seeded property of the world —
the same seed gets the same weather at the same moment, whether or not
anyone watched the hours between. Fronts gather and fade over days rather
than switching on; roughly one part in seven is wet. Rain falls where the
sky can reach, soaks into what can hold it (the moisture the plants read),
and puddles and runs off on what cannot. A cold front's precipitation is
snow, which banks steeper than sand and thaws to meltwater when the front
moves on (`snow.ron` melts at 2 °C — the one shipped material that melts).
Wind slants precipitation, arrives in gusts that shove smoke and lean
trees, and takes a visible slice off exposed water; the heaviest storms
throw lightning, and the storm sky is the clear sky drained and darkened
(`sky.rs`). The play-facing description, including what is deliberately
absent (thunder, erosion, seasons, a closed water cycle), is
[`wiki/weather.md`](wiki/weather.md).

## The ant colony — status

Built (`Reports/creature-direction.md`'s cell-chain direction): `Y` founds
a colony. Ants live on the organism substrate like everything else —
species data in `assets/species/ant.ron`, behaviour behind the deliberate
sense/act cage in `brain.rs` — and coordinate through the world rather than
each other: two pheromone channels (`pheromone.rs`, the stigmergy
deposit → diffuse → decay → follow primitive from
`Reports/stigmergy-research.md`) are what make them forage, dig and build
without ever being told to. Fire kills an ant the ordinary M14 way, into a
corpse. No queens, eggs or new ants yet — the ants you place are the ants
you get. A beetle species landed alongside as data
(`assets/species/beetle.ron`). Play-facing: [`wiki/ants.md`](wiki/ants.md).

## M19 status — started

The visual-polish milestone (`PLAN.md`'s M19 section;
`research/m19-visual-polish.md` is the source material). Landed so far: the
sky (`src/sky.rs`) — a day/night gradient on the same clock the plants
read, dawn and dusk as watchable colour events, stars that come out as dusk
deepens, a moon with a halo that climbs through the night, storm skies as
the clear sky drained of colour; the whole ground tinted by time of day;
underground darkness fixed at worldgen time with a gradual cave-mouth
falloff (`Reports/underground-definition.md`); per-cell liquid grain behind
the `G` selector and the continuous zoom (§9). The remaining tiers stay in
`PLAN.md`. Play-facing: [`wiki/world-cycles.md`](wiki/world-cycles.md).

## Performance

Measured by `cargo run --release --example ascii`, which reports the worst
single frame of each scenario — now run through both drivers back to back
for the stress scenes specifically so the gap is visible directly rather
than compared against an old README number.

Ordinary scenes — a pile, a pour, a pool — cost well under 1 ms per frame, and
a settled world costs nothing at all. Filling the sandbox's full 512×320 world
with sand and water, all moving at once, costs a **worst frame of about 23 ms**
against a 16.6 ms budget at 60 Hz for the CA sweep alone (serial) — **down to
about 6.4 ms parallel**, a **~3.6x speedup on this machine's 4 logical
cores**. Adding the field step every frame on top of that (which the live app
does, and which M5 does not parallelize — see below) brings the worst case to
**about 28 ms serial, about 9 ms parallel** — down from the pre-issues-backlog
~28 ms/~11.5 ms baseline, though not by as much as it looks: issues #5 and #6
(`rebuild_blocked` fetching the owning `Chunk` once per field tile and
indexing directly into it instead of paying a `HashMap<ChunkCoord, Chunk>`
lookup plus a bounds check per CA cell scanned — up to 4096 of those per
tile in the worst case — plus hoisting seven loop-invariant
`next.get(&coord)`/`get_mut` calls out of `field.rs`'s inner loops) measured
at ~24.7 ms/~7.6 ms in isolation, and issue #4 (field sleeping, below) then
added a small amount of that back — `is_converged`'s own comparison pass
costs something real on every frame the solve actually runs, which is every
frame in this specific *worst-case, never-settling* stress scenario. The win
from #4 shows up entirely in the *quiet* case instead — see below — which
this stress scene is deliberately not testing.

Independent review of that change caught a real boundary bug before commit:
`Chunk::get_world` (unlike the `World::get` it replaced) has no concept of
world bounds, so the out-of-world sliver of a chunk whose 64×64 span
extends past a world size that isn't a multiple of `CHUNK_SIZE` (the
sandbox's own 512×320 happens to divide evenly, but `plant.rs`/
`creature.rs`'s 200×200 test worlds don't) silently stopped reading as
blocked. Currently inert in practice — every real consumer of field data
re-checks world bounds itself before ever consulting the stored value — but
exactly the class of bug this file's own history warns about (see M13
status above), worth fixing rather than leaving dependent on that masking
staying intact forever. Fixed with an explicit `world.in_bounds` check
ahead of the `Chunk::get_world` read, with a regression test that reaches
past the masking layer (through `World::fields_ref`, the same
crate-internal seam `rebuild_blocked` itself writes through) to check the
actual stored value rather than the value every real caller would see.

**Issue #4 (field sleeping)**: `field::step` now skips its whole five-pass
solve once `world.active_chunk_count() == 0 && world.fields_settled()` —
both conditions, not the field's own convergence alone, which is what keeps
"a shockwave can cross the whole screen" safe without any separate per-tile
occupancy tracking. Any CA write (including painting a new wall) always
dirties its own chunk, forcing at least one more full pass, and within that
pass a cell that just became blocked resets to ambient (every pass skips
writing to a blocked cell) while the pre-block value is still what
`is_converged` — a new function comparing each channel of the just-solved
state against its pre-step value against a small per-channel epsilon —
compares against, a jump it will not miss. `add_pressure_impulse`/
`add_heat`/`add_light`/`add_heat_local` clear the settled flag directly,
since those bypass the CA grid entirely. Measured via a new permanent
`examples/ascii.rs` scene (`field: sleeping after convergence`): an isolated
pressure impulse's worst frame drops from ~2-4 ms while actively propagating
to ~0.0001-0.01 ms once settled — several hundred times, and the actual
acceptance criterion the issue asked for, a measured number rather than an
assertion.

Independent review of this change found two real, narrow gaps in the
"occupancy changes are caught for free" argument, both fixed:

- `parallel::ChunkView::add_heat`'s same-chunk branch (the common case for
  `fire::tick_burn`'s heat push, since `FIELD_SCALE` divides `CHUNK_SIZE`
  evenly) wrote directly into a worker's own field tile without clearing
  the settled flag — a worker has no `&mut World` to clear it on the spot,
  unlike the serial path. Currently masked only by the coincidence that a
  burning cell's own `tick_burn` also writes its cell every frame it
  burns, independently keeping the chunk awake regardless — not a
  structural guarantee. Fixed with a queued `field_touched` flag, replayed
  in `parallel::run_pass` the same way `field_writes` already is, with a
  regression test (`a_same_chunk_heat_push_during_the_parallel_sweep_
  wakes_the_settled_field`) confirmed to fail without the fix.
- A wall placed by `step_active_sites()` (plant growth — `wood` is
  `kind: Plant`, which blocks per `rebuild_blocked`) or `particle::step()`
  (a landed particle depositing material) is invisible to
  `active_chunk_count()` for the one frame it happens on, if the field was
  already fully converged and quiet: `Chunk::mark_dirty` only ever sets
  `pending_dirty`, and promotion to the `dirty` state `active_chunk_count()`
  reads happens in `World::end_step`, called once per frame from
  `parallel::step` *before* those two subsystems run (see `App::update`'s
  frame order). Self-correcting, not a lasting bug — the very next frame's
  own `end_step()` promotes the pending mark, so the wall is noticed one
  frame late rather than never — and narrower than it sounds, since CA
  writes from the sweep itself are never subject to it (their own
  `mark_dirty` → `end_step` promotion happens entirely within the same
  `parallel::step` call, before `step_fields()` runs). Documented in
  `field::step`'s own doc rather than structurally fixed, since fixing it
  would mean coupling `plant.rs`/`particle.rs` to field-grid internals for
  a one-frame effect that already heals itself.

Both numbers now sit comfortably under the 16.6 ms budget, including the
combined worst case that was the plan's own stated reason M5 could not be
deferred indefinitely. The field grid's own solve (`field.rs`) is not
threaded by this milestone — its per-phase, whole-grid structure is at least
as parallelizable as the CA sweep was, arguably more so since it has no
cross-chunk boundary case to solve at all, but that is additive work for a
future session, not something this one needed to unblock the budget.

That fire/heat pass cost real, measured performance getting to this point —
worth knowing the shape of, since the same mistake is easy to make again in
M16/M17/M18, which all add their own per-cell work to the same sweep. The
worst version (coupling every visited cell to the M13 field) cost the CA-only
figure ~64 ms; removing that and defaulting `heat_conductivity` to zero (see
M14 status above) brought it back down to ~23 ms — still meaningfully above
the pre-M14 baseline of ~16 ms, because even a cheap early-exit check is not
free run 10⁵ times a frame. The lesson for future milestones: a check that is
individually cheap is not free at CA-sweep scale, and the sweep does not have
headroom left to spend carelessly.

So a completely full screen of moving material, with something actively
disturbing the field at the same time, is close to budget with little headroom
serial and comfortable parallel. An earlier "correction" paragraph here said a
quiet field costs the same as a busy one because `field::step` ran all five
whole-world passes unconditionally (issue #4, then unfixed) — that correction
itself went stale when field sleeping landed, and stood in this file for some
time alongside the Issue #4 entry above describing the fix. The current truth
is the one that entry measures: a converged, quiet field skips its whole solve
(~0.0001–0.01 ms against ~2–4 ms while propagating). Multithreading remains
the intended answer for the saturated case, and this is the measurement that
says when it stops being optional. Run the example while nothing else is
compiling — concurrent cargo processes skew the figure badly.

## Status

Working: the cellular automaton core, chunked world with dirty-rectangle
sleeping, twenty-four materials loaded from data with hot reload, angle of repose
from a friction angle, density-driven displacement and layering, a
capsule-swept brush that emits loose material as a stream, the coarse
pressure/velocity/temperature/light field grid, heat diffusion,
neighbour- and temperature-driven ignition, burnout into a material-defined
byproduct, temperature-triggered melting/boiling, pairwise reactions, a fire
tint in the renderer, free particles with gravity and tunnelling-safe
collision for debris that needs a real ballistic arc, and — new in M15 —
explosions that combine all of the above: a pressure impulse and heat spike
into the field, a crater of thrown debris with corner-aware velocity from the
local pressure gradient, and a fireball igniting the intact ring around the
blast. Oil is the one shipped material with real combustion numbers, burning
into ash; `F`/`P`/`X` force-ignite, throw a particle burst, and trigger an
explosion at the brush, all debug tools standing in for gameplay triggers
that don't exist yet. As of M5, the CA sweep the live app runs every frame is
multithreaded — see M5 status above. As of M16, moss and trees grow on their
own schedule, separate from the CA sweep, with root growth tied to real
water uptake and canopy shape driven by auxin canalization — see M16 status
above; `T`/`M` plant a tree/moss seed at the brush. As of M17, painting or
erasing `Solid` material (and explosions) reactively checks structural
support — a stone structure whose span exceeds `max_unsupported_span` from
any anchor breaks free and falls as loose material — see M17 status above.
As of M18, a burrowing worm creature moves on its own schedule, eating
through powder at a cost tied to the target's density, fleeing heat sensed
through the M13 field, and dying (to fire, via M14's existing mechanism
unmodified, or to starvation) into a destructible corpse — see M18 status
above; `J` plants a worm at the brush. Since then: the gnome (M9), the
worldgen redesign (M10's worldgen half), weather, the ant colony, and
M19's sky and lighting have each landed — each has its own status section
above rather than another clause here.

Known limitations:

- **Repose angles come out a few degrees shallower than requested** — roughly
  39/30/18 against 45/34/22 — because reach is a whole number of cells. Fine as
  a tuning knob, not a physical measurement.
- **Real combustion numbers now cover the living world and the water
  cycle** — leaf (at 0.75, the most flammable material in the game), moss,
  wood, rootwood, deadwood, seed, corpse, and the creatures (worm, ant,
  beetle), alongside the original oil and ash; and, from the water-cycle
  work, water, steam, stone and lava. Snow and ice are the two materials
  that melt. Sand, gravel and smoke stay inert deliberately — they have no
  business catching fire. The rule for adding more has not changed and is in
  `stone.ron`'s and `ash.ron`'s headers: **any material that can be the
  target of `burns_into`/`melts_into`/`boils_into`/a reaction needs a real
  `heat_conductivity`**, or the heat it inherits has nowhere to go. Stone
  gained one for exactly that reason when it became lava's quench product,
  measured both ways — see the commit that added lava. (Two earlier versions
  of this bullet went stale in opposite directions: one said only four
  materials had real numbers, lagging the organism work by several
  milestones; the other listed lava and steam as things to add once there
  was a design reason to want them, and outlived their arrival.)
- **Lava that never reaches water never stops being lava.** The
  `intrinsic_temperature` pin has no cooling model behind it, so a flow
  which stalls on a slope stays molten and holds its chunks awake
  indefinitely (measured: 143 cells still molten after 3000 frames of
  `filmstrip scene=lavapour`). Quenching against water is the only exit
  today. Stone has no `melting_point` either, deliberately — see
  `lava.ron`'s header for why the cycle is one-way for now.
- **A burned forest can regrow now** — ash weathers into soil
  (moisture-gated, `decay.rs`) and fresh soil occasionally reseeds moss or
  a tree, which closed the M16 verify criterion's "regrows" half; see the
  field-grid section's decay-and-regrowth heading. (This bullet used to say
  the opposite, and outlived the fix.) What remains true: growth has no
  seasonal or long-term dormancy — a tree either keeps growing until it
  exhausts its attractors/energy, or it doesn't grow at all.
- **Canopy light competition doesn't cast real shadows.** A tree tip leans
  gently toward brighter nearby cells, but branches don't occlude the M13
  light field for each other the way Palubicki et al.'s full model does —
  see M16 status above for what's simplified and why.
- **The field grid's own solve is still single-threaded** — see M5 status
  above. It no longer needs to be threaded to fit the 60 Hz budget (the
  combined worst case is now ~11.5 ms), but it's the next thing that would
  need it if that stops being true.
- **Windows screen capture cannot see this app's rendered canvas on this
  machine** — see M14 status above for the workaround. Worth re-checking
  whether this is machine-specific before assuming every future visual
  milestone needs the in-app framebuffer dump instead of the normal
  screenshot script.

Not yet built: a two-angle (repose vs. maximum-stability) granular model
and the dilatancy/packing-state it would enable — see
`Reports/granular-mechanics-research.md`; **Bak–Tang–Wiesenfeld avalanche
toppling is deliberately not planned at all** (that report found real
sandpile avalanches don't follow BTW's power-law prediction, so the
two-angle model above replaces it rather than sitting alongside it) —
hole-propagation granular flow is likewise on hold pending an actual
hopper/silo use case, per the same report; rigid bodies past chunk bodies
(Douglas-Peucker → triangulation → a real `rapier2d` collider and
continuous rotation — see M8 status above); the streaming world (M10's
unstarted half); multi-creature-kind predator/prey dynamics; and Lua
scripting. Character physics shipped as M9.

## License

MIT — see [`LICENSE`](LICENSE).
