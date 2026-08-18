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

### 1. Puddle evaporation — built, measured, reverted, needs the scheduler

**Not on `master`.** The design is right and the plumbing was wrong.

The idea: a puddle dries up and a lake does not, *without anything measuring
size*. Standing water is a moisture source (`field::apply_moisture_sources`),
so it humidifies the air above itself — over a lake that air is saturated and
the dryness term goes to zero, while a thin puddle on bare rock sits in
whatever is around it and goes. One body shelters itself and a small one
cannot. This avoids the "size cap that gates whether something happens"
mistake `CLAUDE.md` names outright, and it should be kept.

What was built: `Material::evaporation` as an opt-in field tested at the
dispatch site (the `MaterialKind::Liquid` arm in `update.rs`, which already
holds the `Cell`), firing only when `update_liquid` returned `false` — still
water.

**Why it cannot work as written:** a settled chunk is not swept, so nothing
visits the cells. "Still water" is exactly the state that stops being
visited. Forced to a rate of 0.9 to prove it ran at all:

    puddle   6000 -> 5900          (-1.7%)
    lake     2400000 -> 2222000    (-7%)

Backwards — the lake loses more, because it stays awake longer while it
settles. Shipping that would have been worse than shipping nothing.

**What it needs:** the active-site scheduler, which is the engine's existing
answer to "do something to this cell later without keeping its chunk awake".
A new `ActiveKind`, scheduled for exposed liquid surface cells, rescheduling
itself with a stale limit so a sealed-in body stops checking forever —
roughly the shape `organism` already uses. Do not solve it by keeping water
chunks awake; that gives back the whole per-tile field-sleeping win.

**Two metric traps on the way, both of which will recur:**

- Counting water **cells** read `6 -> 228` and looked like evaporation
  manufacturing water. It was six full cells spreading into a thin film at
  constant volume. Measure liquid **volume** (`liquid_fill`), never cell
  count — `CLAUDE.md` says so and it still caught me.
- Assuming rain was contaminating the test produced **byte-identical**
  numbers after the fix, which means the condition was degenerate. Rain was
  never involved. An exactly-zero delta is different evidence from a small
  one.

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
