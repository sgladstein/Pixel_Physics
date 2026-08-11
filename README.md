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
  world.rs     the sparse chunk map and the get/set seam
  update.rs    the cellular automaton step
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

## Performance

Measured by `cargo run --release --example ascii`, which reports the worst
single frame of each scenario.

Ordinary scenes — a pile, a pour, a pool — cost well under 1 ms per frame, and
a settled world costs nothing at all. Filling the sandbox's full 512×320 world
with sand and water, all moving at once, costs a **worst frame of about 23 ms**
against a 16.6 ms budget at 60 Hz for the CA sweep (which now includes M14's
fire/heat pass on every visited cell) alone; adding the field step every frame
on top of that (which the live app does) brings the same worst case to
**about 28 ms**.

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
pressure/velocity/temperature/light field grid, and — new in M14 — heat
diffusion, neighbour- and temperature-driven ignition, burnout into a
material-defined byproduct, temperature-triggered melting/boiling, pairwise
reactions, and a fire tint in the renderer. Oil is the one shipped material
with real combustion numbers, burning into ash; `F` force-ignites the brush
area as a debug tool.

Known limitations:

- **Repose angles come out a few degrees shallower than requested** — roughly
  39/30/18 against 45/34/22 — because reach is a whole number of cells. Fine as
  a tuning knob, not a physical measurement.
- **Only oil and ash have real thermal numbers.** Every other shipped material
  defaults to non-flammable, non-meltable, `heat_conductivity: 0.0` — correct
  for sand/water/stone/gravel/smoke, which have no business catching fire, but
  it means there is exactly one flammable material to watch burn right now.
  Lava, steam and richer reactions are natural M15+ additions once there is a
  reason (explosions) to want them.
- **A saturated screen plus an active field disturbance is over the 60 Hz
  budget, more so after M14** — see Performance above. Multithreading
  (planned, not built) is the answer once this becomes the normal case rather
  than a stress test.
- **Windows screen capture cannot see this app's rendered canvas on this
  machine** — see M14 status above for the workaround. Worth re-checking
  whether this is machine-specific before assuming every future visual
  milestone needs the in-app framebuffer dump instead of the normal
  screenshot script.

Not yet built: Bak–Tang–Wiesenfeld toppling for avalanches, hole-propagation
granular flow, explosions, multithreading, free particles, rigid bodies,
character physics, the streaming world, and Lua scripting.
