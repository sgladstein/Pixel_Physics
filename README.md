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

## Architecture

```
src/sim/     the simulation — knows nothing about windows or GPUs
  cell.rs      one pixel, packed into 4 bytes
  material.rs  materials as data, not code
  chunk.rs     64x64 tiles, coordinate maths, dirty rectangles
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
surface search are all capped at it.

## Performance

Measured by `cargo run --release --example ascii`, which reports the worst
single frame of each scenario.

Ordinary scenes — a pile, a pour, a pool — cost well under 1 ms per frame, and
a settled world costs nothing at all. Filling the sandbox's full 512×320 world
with sand and water, all moving at once, costs a **worst frame of about 16 ms**
against a 16.6 ms budget at 60 Hz.

So a completely full screen of moving material sits right on the limit, with no
headroom. Multithreading is the intended answer, and this is the measurement
that says when it stops being optional. Run the example while nothing else is
compiling — concurrent cargo processes skew the figure badly.

## Status

Working: the cellular automaton core, chunked world with dirty-rectangle
sleeping, seven materials loaded from data with hot reload, angle of repose from
a friction angle, density-driven displacement and layering, a capsule-swept
brush that emits loose material as a stream, and the sandbox.

Known limitations:

- **Repose angles come out a few degrees shallower than requested** — roughly
  39/30/18 against 45/34/22 — because reach is a whole number of cells. Fine as
  a tuning knob, not a physical measurement.
- **Nothing burns, melts or reacts.** `Cell` is exactly 4 bytes with no room for
  temperature, so heat needs a deliberate decision to widen it to 8.

Not yet built: Bak–Tang–Wiesenfeld toppling for avalanches, hole-propagation
granular flow, heat and reactions, multithreading, free particles, rigid bodies,
character physics, the streaming world, and Lua scripting.
