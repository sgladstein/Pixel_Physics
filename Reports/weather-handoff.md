# Weather, the bigger world, and what is still open

*Written to be picked up cold. Everything below is landed on `master` unless
it says otherwise.*

## What this session did

A twelve-commit push that made the world bigger and gave it weather, plus
three follow-ups that fixed bugs the push itself introduced.

| | |
|---|---|
| World | 2048x640 — four screens wide, twice a screen deep |
| Camera | `Renderer::follow`, called once per rendered frame in `App::draw` |
| Build hitch | 1010 ms → 385 ms |
| Weather | clock, rain, snow, storms, lightning |
| Suite | 537 pass, 0 fail; clippy clean at `-D warnings` |

The camera lives in `App::draw` and not `App::update` deliberately: it is view
state, so it runs once per rendered frame rather than N times inside the
fixed-timestep catch-up loop, and therefore cannot touch determinism.

`src/sim/weather.rs` is a **pure function of `(seed, frame)`** — no stored
state, no RNG stream, nothing to desynchronise. Every draw goes through
`rng::stream`, deliberately not `ParticleSystem::rng`, which is documented as
not reproducible. A forecast is `at(seed, frame + n)`. Keep it that way; the
determinism guarantee is free only as long as nothing accumulates.

`examples/viewshot.rs` is the harness for all of it. It renders viewport-sized
frames through the real `Renderer` and reuses one frame buffer with only the
*first* draw forced full, so every later shot must be repainted by the camera
move alone — without `last_camera` the sheet shows the same view four times,
which no single image can reveal. `rain=wet|dry|snow|bolt` picks a frame that
is both the weather asked for *and* the same time of day; `mine=1` cuts shafts
between two draws.

## Open work, in the order I would take it

### 1. Puddle evaporation — landed

`src/sim/evaporation.rs`, on the active-site scheduler as this section
previously called for. The design was kept: a puddle dries and a lake does
not, and nothing measures the size of a body of water.

Measured, four rows deep in a walled basin, one body per world, same seed so
the same weather:

| | 2,500 frames, calm | 6,000 frames spanning a gale |
|---|---|---|
| puddle, 6 cells wide | **gone** | gone |
| lake, 176 cells wide | **2.2%** | 9.1% and then it stops |

What the lake loses is its shoreline — the band within `SHELTER_REACH` of
open ground — and at two rows deep 9.1% is under a fifth of a row. A lake of
a realistic depth loses the same *absolute* amount, so a full gale takes a
sliver off it and the next rain puts it back.

(The frame counts are small because the guards run on a 256x128 world with
two-row bodies, sized for test cost. The same comparison on the 640x200
four-row scene they were first written against read 95% against 0.9% at
11,000 frames — the same behaviour over more water, not a different one.)

**What the sweep still does** is schedule, and only that: a liquid cell that
fails to move with air above it enqueues an `ActiveKind::Evaporate` site and
forgets it. Everything after is the scheduler, so chunk sleep is irrelevant.
Measured on a settled world, mean awake chunks over 2,000 frames: 0.02 for
the puddle scene, 0.22 for the lake. It does not keep water chunks awake.

**Frame cost: none measurable**, and getting to that answer cost more than
the feature did. `ascii.rs` worst frames with evaporation on, against the
same harness with `water.evaporates` off and re-measured in the same session:
every scene within noise. One scene looked like a 4x regression — parallel
field-stress, 52.7 ms against an 11.9 ms baseline. It was noise. The same
scene reads 8.5 to 20.9 ms with the feature and 11.2 to 11.8 without, and a
standalone probe of that exact scene puts the worst frame at 11.5 ms with 70
sites scheduled across the whole 400-frame run.

**The fix that reading prompted turned the entire mechanic off, silently.**
Gating the sweep hook on `(x + y) % INTERVAL == frame % INTERVAL` skips the
reads on 59 frames in 60 and looks free. Nothing evaporated anywhere, in any
test — because a chunk is swept for a handful of frames after its water stops
moving and then sleeps, so a gate that opens one frame in sixty is a gate
that mostly never opens while the sweep is still visiting. Same shape as the
mistake this whole file exists to correct, one level down: **the sweep's
visits are the scarce resource, and anything that discards one is discarding
the only chance there was.** The stagger it was also providing is worth
having, so it survives as an offset on the *first* `next_frame` rather than a
gate on the hook — the checks spread across the interval, and nothing is
skipped.

The rule about re-measuring a baseline in the same session applies to a
regression you are about to *fix*, not only to one you are about to report.

**The dedup in `World::pending_evaporation` is load-bearing for the rate,
not the frame cost.** Without it the number of duplicate sites a body
accumulates is proportional to how long it stays awake settling — which is
exactly the quantity that made the reverted version dry a lake faster than a
puddle. It would have come back by a different route.

#### Two things found on the way, both of which cost real time

**The moisture channel could not get out of a wet blocked block, and the
design does not work at all without that fixed.** `blocked` goes true for a
whole 8x8 field block if one cell in it is `Solid`, and `step_diffusion`'s
wall rule made a blocked neighbour contribute nothing. `rebuild_blocked`
already scans such a block in full *specifically* so a shallow puddle on a
thin floor registers as a moisture source — and then the solve threw that
source away. Measured: standing water four rows deep with its block clear of
stone put 2.310 in the air a block above; the identical body shifted three
rows so its block also caught the floor put **0.000** there, for a body 240
cells wide. The signal was not weak, it was absent, and absent as a function
of where an 8-row grid boundary fell. `step_diffusion` now lets moisture —
and only moisture — read through a blocked neighbour that is itself a source.
Heat and light keep the strict wall rule; the sealed-room guards depend on it.

**A gale erases every lake in the world, and no reading of humidity can stop
it.** `weather::gust` fires every 26 frames for as long as the wind channel
is over `GUST_THRESHOLD`, which is most of a windy epoch. Traced on seed
12345: the channel crosses at frame 11,460, and within ten frames the air
over a lake goes 2.31 -> 0.23 and stays there, because advection back-traces
two field blocks up into dry air faster than diffusion rebuilds the layer. On
the full 2048x640 world it is slower and no better in the end — 2.31 -> 0.42
by frame 14,000, lake down to 39%, and an 800-cell lake behaves exactly like
a 240-cell one, so it is not a small-world artifact.

The reason it is unfixable from humidity is worth stating plainly: **a gale
mixes the atmosphere, so the air over a puddle and the air over a lake become
equally dry.** They read *identically*. Any function of that reading gives
them the same answer.

So `evaporation::shelter` is a second factor, multiplied in: how much of the
`SHELTER_REACH` field blocks either side of this cell is standing water,
read off `field::moisture_source_at`, which is rebuilt from the CA grid every
frame and never advected. It is a fixed-radius stencil with no notion of a
body — no flood fill, no connectivity, no total — so it cannot tell a
fifty-cell pond from an ocean and does not need to, and two puddles a few
cells apart shelter each other, which is right. **If the gust field is ever
retuned so a windy epoch stops scouring the humid layer, this term can go and
the humidity deficit alone does the whole job again.** It is a separate factor
precisely so that it can.

#### What this changed elsewhere

`plant.rs`'s `moss_spreads_over_damp_stone_and_not_over_dry` failed, correctly:
its scene was a one-row film of water on a stone platform, and a film is the
first thing to go. Confirmed by control — with `water.evaporates` off, the old
scene passes unchanged. The water now sits in a sealed pocket one row *under*
the platform, which never evaporates and shares a field block with the moss.
That comment records the constraint any future edit there has to respect: a
blocked block is damp only if water is *inside* it, so the scene needs water
along the whole platform rather than pooled at one end (measured 4.000 over a
pool and 0.000 two blocks away).

`examples/viewshot.rs`'s settle loop did not call `step_fields`. Harmless
while everything it judged was *drawn*; not harmless once something *reads* a
field channel, because with no field step the humidity is zero everywhere and
a long settle dries every lake in the world.

#### One behaviour to put in front of the owner

**Water spread thin over a flat floor never dries.** Poured onto
`filmstrip scene=tree`'s shelf — which spans the whole world with no lip —
121,000 fill runs out into a film one cell deep across all 512 columns, and
stays there: humidity above it reads 2.34 against `HUMID_STOP`'s 2.0, so the
rate is exactly zero, and `shelter` agrees. It is the design working as
specified (a sheet of water that wide really does saturate its own air) and
it is not a regression, since the film was permanent before this too. But a
player would call that a puddle, and the `flat` preset behind F7 is exactly
the geometry that produces it.

The cheap candidate fix, not taken, is to read *depth* at the cell — a
surface cell with solid directly beneath it is a film, one with water beneath
it is a body — which is a single local `get` and is about thermal mass rather
than size. It is a third discriminator layered onto a design that already has
two, and it would make a genuinely shallow lake behave like a puddle, so it
wants an opinion before it wants code.

#### Still open here

- **The gust field is a steady wind on a 26-frame timer.** Its own doc says a
  gust every frame would be "a steady wind wearing a different hat"; every 26
  frames with a radius-26 impulse is closer to that than the doc implies.
  `unsettled_field_tiles` sits at 30 and never returns to zero for the whole
  windy epoch, and the velocities are large enough to replace the air over a
  lake several times a second. That is a weather question, not an evaporation
  one, and it is left alone deliberately — but it is what `shelter` exists to
  work around, and fixing it would simplify this file.
- ~~**Evaporated water is gone, not banked.**~~ **Closed** by the water
  bank: one `f64` on `World` in liquid-water cell-equivalents, credited by
  `evaporation::tick` and spent by `weather::step`, with `render.rs` thinning
  the drawn rain by the same supply factor the storm is throttled by.
  Measured flat to -0.0000% across 6,000 frames spanning a storm and a
  drought, both drivers. What is *not* closed is infiltration: a landing
  water cell's fill becomes soil wetness and nothing credits it back, so an
  all-soil world's supply settles at about half — see
  `weather::STORM_RESERVE`'s own doc for the sizing and the trajectory.
- **The rate constants are set from feel, not from anything physical.** A
  six-cell puddle four rows deep takes ~11,600 frames, a little over three
  in-game days; two rows deep, about half that. `probe_drying_curve` is the thing to re-run after touching
  `FILL_PER_CHECK`, `HUMID_STOP` or `CHECK_INTERVAL`.
- ~~**Nothing reads the day/night temperature.**~~ **Closed** by
  `evaporation::warmth`: the rate gains a third factor, `1 + 0.15 * (T -
  ambient)` clamped to 0.25..3.0, off the **raw** field temperature at the
  water's own block. The single sanctioned exception to `CLAUDE.md`'s
  divide-the-oscillator-out rule, and it is one site — `field.rs`'s moisture
  decay stays noon-equivalent, because `dryness` reads the humidity that
  decay produces and making both diurnal multiplies the day into the rate
  twice. Guards: `days_evaporate_more_than_nights` (2.47:1, confirmed to fail
  at 1.00 if the read is swapped to the noon-equivalent) and
  `humidity_does_not_go_diurnal`, which is the same assertion pointed the
  other way.

  **Both re-derivable constants were re-derived and neither moved.** The
  humidity-against-width table reads identically with the coupling off and at
  0.15, digit for digit — expected, since decay is still noon-equivalent, and
  measured rather than assumed. `FILL_PER_CHECK` holds because the factor is
  *linear* in a zero-mean oscillation and so has a day-mean of exactly 1.0:
  whole-day totals move +3.6% over one day and −1.3% over four (32-cell
  basin), −0.1% over four on the lake. An Arrhenius shape would not have this
  property and was rejected for it.

#### Dry cold air suppressing evaporation: designed, deliberately not built

`weather.rs`'s `chill` channel is real when nothing is falling and nothing
reads it (`weather.rs`, the `Weather::chill` doc says so outright). The
obvious next move after the coupling above is for a clear cold snap to slow
drying the way a snowstorm now does. It was weighed and skipped, and the
reason is worth having in writing because the cheap version is the wrong
one.

**The cheap version**: `evaporation::warmth` also reads `weather::at(seed,
frame).chill` and subtracts. Rejected. `weather::at` is a *global* pure
function with no position in it, so a puddle at the bottom of a sealed cave
would slow down because the sky two hundred rows above it is cold — and the
whole design of this file is that both existing factors are local readings
(`dryness` from the block above, `shelter` from a fixed stencil either side).
It would also be a *second* oscillating-ish global read at a site whose one
selling point is that it has exactly one.

**The version worth building**: `weather::step` writes dry chill into the
*field's temperature channel*, as a boundary condition on open-sky columns,
exactly as `hold_column_cold` already does under falling snow — and then
evaporation reads nothing new at all. The coupling above picks it up for
free, attenuated with depth by the machinery that already attenuates the
sky's own forcing, so the sealed cave stays warm and the open shore does not.
It also makes a clear cold night visible in the temperature overlay and to
anything that later reads field temperature, rather than being a term hidden
inside one consumer.

Two things to settle before building it: how much of `SNOW_CHILL`'s 26
degrees a *dry* snap should be worth (a clear cold night is not a blizzard,
and `WARMTH_FLOOR` is reached at 13.7 below ambient, so anything much over
that is indistinguishable from anything else over it); and what it costs the
field's sleep gate, which currently wakes on `sky_temperature_offset`
changing and would need the same quantised-staircase treatment
(`SKY_TEMPERATURE_QUANTUM`'s doc is the whole argument) or it will hold the
world awake through every cold epoch.

### 1e. The outer cycle now has a *hot* leg, and it exposed a magnitude

**Landed.** Steam that condenses with open sky above it credits
`World::atmospheric_bank` instead of leaving a water cell where it stood
(`MaterialDef::condenses_into_sky`, set only on `steam.ron`). Reported from
live play as a plume that "rises about 5 ft in the air and then drops back
into rain... it almost looks like bouncing".

The complaint was not about rate or height. On the new
`filmstrip scene=lavadrop` the plume held **421 water cells standing in the
air** at peak, through its whole 40-row height — condensate falling back
*through* the steam still rising, and since a falling `Liquid` displaces a
`Gas`, each droplet shoved steam cells downward on the way. Airborne water
421 → 0; `water_equivalents + bank` unchanged to a tenth of a cell.

"Open sky" is the frozen `World::sky_surface`, so a sealed pocket, a cave,
or `scene=boil`'s roofed chamber all still condense into water in place —
which is what keeps the sealed-pocket guards meaningful rather than
vacuous.

**What it exposed.** The boil-off was always this large; recycling hid it.
With the return leg gone, an 800-cell lava blob visibly drops a
12,000-cell pond. The cause turned out not to be the boil rate but
`diffuse_heat` being non-conservative — see
`Reports/open-bugs-handoff.md` 1b, which is the real defect and is left
open deliberately.

`fire::LATENT_HEAT_DEGREES` (540, water's latent heat as a multiple of its
specific heat) charges each boil to the stored heat of its neighbours and
refuses the boil if they cannot pay. It cannot go on the *product*: water
boils at 100 and steam condenses at 45, so a birth temperature more than
~55 degrees down flashes straight back. Measured where boiling is the
mechanism:

| | boiled | standing water |
|---|---|---|
| `scene=boil`, no charge | 42,740 | 4,369 |
| `scene=boil`, 540 | **17,019** | **7,379** |
| `scene=simmer`, no charge | 1,941 | pan dry |
| `scene=simmer`, 540 | **191** | 1,334 |

On `scene=lavadrop` the charge is nearly flat, because there the water goes
through the **quench reaction** rather than through boiling — and that path
is already about one water per lava (944 reactions for 800 cells of lava,
banking 924 cell-equivalents). A `min_fill` gate on the reaction was built
to cut it further, swept, and reverted: it moves conversions between paths
without changing the total. The table is in `lava.ron`.

**Still open here:** nothing bounds how much water a *reaction* may
convert, the way the boil is now bounded. The quench happens to be roughly
1:1 by construction; a future reaction need not be.

### 2. Thunder

Lightning flashes and forks; the world is silent. The engine has no audio at
all, so this is a larger question than it sounds and should probably wait for
a decision about sound generally.

### 3. Tuning calls that want the owner's eyes, not more of mine

None of these are bugs. All four are "does this feel right", which is the one
category where playtest reports have repeatedly overturned things that
measured fine:

- **Snow depth.** Drifts build fairly thin before a front moves on. Unknown
  whether they read as snow or as frost.
- **Rain density in motion.** Set by eye against stills, twice, but never
  watched moving — and motion is where it either reads as rain or as
  hatching.
- **Gusts.** Whether they are noticeable at all in play.
- **The lightning flash.** Whether it is too strong, or too rare.

## Bugs this push introduced and then fixed — read before touching these

Three, all of the same shape, and the shape is the lesson.

**The advection clamp** (`ADVECTION_MAX_TILES`, `field.rs`). The field-sleeping
commit also clamped advection back-trace to one field *cell*, with a comment
claiming it cost "only that very fast flow transports at one cell per step".
Never measured, and wrong: open-ground pressure went from dispersing to 2.9 to
freezing at 2177.8, 750x too concentrated, forever. Sized from measurement at
four tiles, which is byte-identical to the pre-sleeping baseline. Going past
the halo is safe because advection only ever *reads* — it writes nothing
outside its own tile — so an overlong back-trace costs accuracy, never
correctness.

**The gust monopole** (`weather::gust`). A lone positive pressure impulse
injects net pressure into a closed world with nowhere to go, so the tiles
around it never reconverge — the reverted steady-wind failure by another
route. A gust is now a dipole: high pressure behind, low ahead, summing to
zero. **Do not make it a monopole again**, and do not reintroduce a steady
forcing term (measured once at a permanent 3.55 ms/frame on every scene).

**The skyline** (`rebuild_sky_floor`, `render.rs`). Mining dropped a column's
horizon so the sky came down the hole with the pick. Fixed by asking "is this
inside the ground" rather than "can the sky reach this cell". A **stateful**
version — remember the highest the ground has ever been, i.e. how Terraria
walls behave — fixed the shaft and broke
`dirty_rect_skip_is_pixel_identical_to_a_full_redraw`, because a skyline that
depends on history cannot agree between a renderer that has one and a fresh
one that does not. The shipped rule is stateless: a column is inside the
ground where the terrain to **both** sides stands higher. It is an `and`
rather than a `min` because a cliff edge has higher ground on one side only,
and a `min` paints a dark band along the foot of every cliff.

### The single lesson worth carrying forward

All three, plus the evaporation attempt, are the same failure: **every guard
tested that a mechanism fires, and none tested that it stops.** Pressure
propagated and never dispersed. Gusts disturbed and never settled. Rain
landed and was never seen. Evaporation ran and then silently stopped running.

`field.rs` now has
`a_disturbance_in_open_ground_disperses_rather_than_freezing`, and
`weather.rs` has `a_gust_disperses`, both written as ratios or against a calm
control so they survive retuning. **When adding anything to these subsystems,
write the "and then it stops" test first.**

Two related process notes that each cost real time:

- The sealed-room field failure was called "pre-existing" twice, having been
  checked against a `master` that already contained the change under
  suspicion. A moving baseline is not a baseline. The rule in `CLAUDE.md`
  about re-measuring timings in the same session applies just as hard to
  correctness.
- The reproduction for the skyline bug has to contain the **order of events**,
  not just the final state. Mining before the renderer's first draw does not
  reproduce it — the opening scan records the shaft floor as true ground, the
  sky fills the hole, and a working fix looks broken.

## Things that are true and easy to get wrong

- `field::step` and everything under it iterates an **awake subset**. A probe
  on `solve.len()` is the first diagnostic for anything field-shaped; it said
  16 of 16 chunks in the case above, which is what ruled sleeping out.
- `World::unsettled_field_tiles()` is `#[cfg(test)]` and is the honest way to
  ask "has this disturbance gone away". Summed pressure cannot answer it — the
  field's own background relaxation is an order of magnitude larger than a
  gust.
- Weather runs inside **both** drivers (`parallel::step` and `update::step`).
  Any test that steps the world is therefore subject to rain unless it picks a
  dry window.
- Precipitation is simulated **where it lands, not where it falls**. No write
  ever goes into a sky column; that would wake every field tile between cloud
  and ground and undo per-tile sleeping.
- Snow's melting point is *below* ambient on purpose. It survives only while
  the front's cold band is overhead, and the cold must be written to the
  **cells** as well as the field — `fire::update` compares `cell.temperature`,
  which the field channel does not feed.
