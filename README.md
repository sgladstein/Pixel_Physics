# Pixel Physics

A falling-sand pixel physics engine in Rust. Every cell in the world is one
simulated pixel. Built as a foundation for games, not as a one-off demo.

## Running

```sh
cargo run              # the sandbox
cargo run --example ascii   # headless terminal view, no GPU needed
cargo test
```

## Controls

| Input | Action |
|---|---|
| Left mouse | Paint the selected material |
| Right mouse | Erase |
| `1`–`9` | Select material by number |
| `Q` / `E` | Cycle material |
| `[` / `]` or scroll | Brush size |
| `Space` | Pause |
| `.` | Step one frame while paused |
| `F` | Ignite whatever's under the brush (debug tool — M15 will add real ignition sources) |
| `P` | Throw a burst of the selected material as free particles (debug tool for M7) |
| `X` | Trigger an explosion at the brush radius (M15) |
| `F1` | Chunk overlay — green borders are awake, grey are asleep |
| `F5` | Reload materials by hand |
| `R` | Reset |
| `Esc` | Quit |

The window title shows frame rate, selected material, how many chunks are
awake, and the result of the last material reload. `0/40 awake` on a still
world means chunk sleeping is working.

## Materials

Materials live in [`assets/materials`](assets/materials) as one `.ron` file
each, and are **watched while the app runs** — save a file and the change
applies immediately, with any parse error shown in the window title. The files
are also compiled into the binary, so the engine still works without them.

```ron
(
    name: "sand",
    kind: Powder,          // Solid | Powder | Liquid | Gas
    density: 1.6,          // heavier sinks through lighter
    friction_angle: 34.0,  // powders: angle of repose, 45 is steepest
    dispersion: 5,         // liquids and gases: sideways travel per step
    colors: [(222, 196, 128), (212, 184, 116)],
)
```

Ids are keyed by name and never reassigned, so editing a file changes material
already in the world rather than replacing it. Renaming adds a new material.

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

**The M14 schema** (combustion, phase change, reactions) is defined and
loadable — see [`oil.ron`](assets/materials/oil.ron) for the first material
using it — but nothing reads it yet; that is the update logic M14 adds. All
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

## Architecture

```
src/sim/     the simulation — knows nothing about windows or GPUs
  cell.rs      one pixel, packed into 8 bytes: material, shade, flags,
               temperature, and a kind-specific aux slot
  material.rs  materials as data, not code
  chunk.rs     64x64 tiles, coordinate maths, dirty rectangles
  field.rs     the coarse pressure/velocity/temperature/light grid,
               one tile per chunk, its own frame phase
  fire.rs      heat, ignition, burnout, phase change, reactions
  particle.rs  free (off-grid) particles for explosions and splashes
  explosion.rs pressure impulse + heat spike + debris, built from the above
  world.rs     the sparse chunk map and the get/set seam
  update.rs    the cellular automaton step's rules, generic over CellSurface
  surface.rs   the CellSurface trait update.rs/fire.rs run against --
               World (serial) or parallel::ChunkView (multithreaded)
  parallel.rs  M5: the multithreaded checkerboard sweep -- an alternative
               driver for update.rs's rules, not a second copy of them
src/render.rs  cells to pixels
src/app.rs     sandbox state: brush, picker, terrain
src/main.rs    window, input, fixed 60 Hz timestep
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

**No rule may look further than `MAX_REACH`.** Sweep regions are widened by
that much horizontally (and one cell vertically, which is as far as anything
looks up or down), because a cell has to be reconsidered whenever anything it
can *see* changes — not just its immediate neighbours. A rule that reads
further than the region is widened acts on cells that no longer wake it, and
material goes stale mid-flow. Powder roll, liquid dispersion and the liquid
surface search are all capped at it. **This limit applies to the CA sweep
only.** The field grid (below) is a whole-grid pass that reads everything
every step regardless of what changed, so it has no equivalent staleness risk
and is not bound by it — that is precisely why long-range effects like a
shockwave crossing the whole screen live there and not in a CA rule.

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
cheap early exit. Sand and water, never near fire in ordinary play, were
paying for four neighbour reads apiece on every visit for no reason; that
was a further, separate contributor to the same regression above. Only oil
(and ash, its `burns_into` target — see the comment in `ash.ron` for why a
combustion *byproduct* specifically must not default to zero, or the heat it
inherits has nowhere to go and its chunk never sleeps) opt in explicitly.
**Any future material that can be the target of `burns_into`, `melts_into`,
`boils_into`, or a reaction needs a real `heat_conductivity` for the same
reason.**

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

**Known simplification, left as such rather than fixed tonight**: the
fireball reuses `World::ignite_circle`, the M14 debug force-ignite tool,
which sets *any* material burning regardless of its `flammability` — a stone
wall next to a blast currently gets the same fire tint oil would, rather than
being immune the way `flammability: 0.0` says it should be. Visually this
reads as "the blast leaves the surroundings glowing hot," which is not
unreasonable for a first cut; a version that actually checks flammability
(closer to `fire::try_ignite`'s temperature-driven path) would be the more
correct fix.

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
`unsafe` anywhere** (`grep -rn unsafe src/` returns nothing but the doc
comments describing why there's none), at the cost of one extra serial
merge step per pass.

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
about 28 ms serial, **about 11.5 ms parallel**.

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
disturbing the field at the same time, is over budget with no headroom.
Ordinary use — the field sitting quiet, or only a modest area of CA material
moving — stays far under it; a quiet field costs almost nothing since nothing
in it is changing. Multithreading is the intended answer for the saturated
case, and this is the measurement that says when it stops being optional. Run
the example while nothing else is compiling — concurrent cargo processes skew
the figure badly.

## Status

Working: the cellular automaton core, chunked world with dirty-rectangle
sleeping, seven materials loaded from data with hot reload, angle of repose
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
multithreaded — see M5 status above.

Known limitations:

- **Repose angles come out a few degrees shallower than requested** — roughly
  39/30/18 against 45/34/22 — because reach is a whole number of cells. Fine as
  a tuning knob, not a physical measurement.
- **Only oil and ash have real thermal numbers.** Every other shipped material
  defaults to non-flammable, non-meltable, `heat_conductivity: 0.0` — correct
  for sand/water/stone/gravel/smoke, which have no business catching fire, but
  it means there is exactly one flammable material to watch burn right now.
  Lava, steam and richer reactions are natural additions once there is a
  design reason to want them.
- **Explosions ignite anything nearby regardless of flammability** — see M15
  status above. Reuses the debug force-ignite tool rather than a
  flammability-respecting path; stone glows the same as oil would.
- **The field grid's own solve is still single-threaded** — see M5 status
  above. It no longer needs to be threaded to fit the 60 Hz budget (the
  combined worst case is now ~11.5 ms), but it's the next thing that would
  need it if that stops being true.
- **Windows screen capture cannot see this app's rendered canvas on this
  machine** — see M14 status above for the workaround. Worth re-checking
  whether this is machine-specific before assuming every future visual
  milestone needs the in-app framebuffer dump instead of the normal
  screenshot script.

Not yet built: Bak–Tang–Wiesenfeld toppling for avalanches, hole-propagation
granular flow, rigid bodies, character physics, the streaming world, plants,
structural integrity, creatures, and Lua scripting.
