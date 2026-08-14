# Explosion mechanics — diagnosis

Owner report: *"the explosion mechanics are very unsatisfying."* This is the
look-first pass on that, following `CLAUDE.md`'s method: render it and look
before writing any metric, then measure the specific quantities the images
turned up. **No fix is implemented here** — this is the what-and-where.

M15 was signed off against "an explosion throws debris that re-integrates
into terrain on impact," and it does exactly that. The problem is that it
does *only* that, in one frame, and two of the three things the design says
an explosion writes turn out to have no consumer at all.

## Harness note

`examples/filmstrip.rs` could not show an explosion before this pass: it ran
the CA sweep and nothing else, so `ParticleSystem::step` never ran (debris
never moved) and `step_fields` never ran (the impulse never propagated). It
now runs `App::update`'s full phase order, and takes
`explode=x,y,radius,strength,frame` plus two scenes, `boom` (sand pile with a
buried stone slab) and `boom_stone` (solid stone). The added phases are
no-ops for the four pre-existing scenes — nothing promotes a liquid body,
nothing schedules an active site, and no liquid rule reads the field — so
recorded measurements against `pour`/`fall`/`blob`/`sand` are unaffected.

```
cargo run --release --example filmstrip -- scene=boom explode=256,190,20,180,60 \
    start=60 every=2 count=6 cols=3 crop=136,120,240,160 zoom=2
cargo run --release --example filmstrip -- scene=boom_stone explode=256,220,20,180,60 \
    start=61 every=30 count=6 cols=3 crop=176,160,160,120 zoom=3
```

All numbers below are from a throwaway `examples/debug_explosion.rs` (deleted
after use, same precedent as PLAN.md §6), at the app's own live parameters:
`radius = 20` (brush), `strength = 180.0` (`App::explode`'s fixed constant).

## What the player actually sees

**In stone, six samples spanning frames 61→211 are essentially identical: a
perfect orange donut, motionless, for two and a half seconds.** The crater
inside it is a small irregular blob far smaller than the 20-cell radius that
was actually cleared, because the blast's own debris landed back in its own
hole within three frames. That donut is the entire effect.

In sand, a thin flat sheet of grains lifts off and drifts; the crater slumps
shut. The detonation frame itself shows a thin orange ring arc and otherwise
looks unchanged.

## Findings, in order of how much they cost the effect

### 1. The detonation frame is very nearly a no-op

`trigger` spawns each debris particle at exactly the cell it replaces,
carrying that cell's own `material` and `shade`, and `draw_particles` paints
it from the same palette entry. The only pixel difference is grain:
`cell_colour` applies position-keyed jitter, `draw_particles` does not.

Measured as **mean absolute RGB delta** between the frame before and the
detonation frame (a *count* of changed pixels says 104% of the footprint
changed and is useless here — it counts the grain difference, which is
invisible; this is the metric trap `CLAUDE.md` warns about, hit live):

| region | buried blast | surface blast |
|---|---|---|
| crater interior | **12.9 / 255 (5%)** | 11.0 / 255 (4%) |
| fireball ring | 25.0 / 255 (10%) | 16.2 / 255 (6%) |

The single most dramatic frame of the effect changes the crater's appearance
by 5%. There is no flash, no white-hot core, no expanding front — nothing
that reads as *detonation* rather than *deletion*.

### 2. Debris lands on frame 1

| | buried (70 cells of cover) | just under the surface |
|---|---|---|
| debris spawned | 2513 | 1804 |
| still in flight after 1 frame | 364 (**86% already landed**) | 551 (69% landed) |
| after 3 frames | **0** | 208 (88% landed) |
| furthest any debris got | **30 cells** (blast radius 20) | 317 cells |

`advance_and_check_landing` converts a particle back to a CA cell the moment
its next substep is non-empty. Debris is launched from inside a solid volume
toward a crater rim only `radius` away; at `strength 180` the launch speed is
`180 * 0.05 = 9`, clamped to `MAX_SPEED_PER_AXIS = 8`, so it crosses 20 cells
in about three frames and buries itself in the wall.

The behaviour is **bimodal**: debris either dies immediately (the
overwhelming majority) or finds open sky and flies clean across the world.
There is no middle case — which is why the surface blast reads as a flat
horizontal sheet of grains rather than a plume. The survivors are exactly the
subset that launched upward, so they all share one velocity profile.

### 3. The crater refills itself within half a second

1257 cells dug. Still empty:

| | f+1 | f+2 | f+5 | f+30 |
|---|---|---|---|---|
| buried | 717 | 562 | 416 | **0** |
| surface | 1010 | 913 | 892 | 812 |

Buried, **43% of the crater refills on the first frame** and it is completely
gone by frame 30 — the blast leaves no lasting mark at all. The surface case
holds only because sand above the rim has an angle of repose to rest at.

This is the same fact as (2) seen from the other side: the debris that "dies
on frame 1" is landing back inside the hole it came from.

### 4. The fireball is a stamped ring with a step-function lifetime

`ignite_circle(cx, cy, radius * 1.5)` force-ignites a *filled* circle, but
step 2 just cleared the inner `radius`, so what is left is a geometrically
perfect annulus of uniform thickness. Every cell in it receives the same burn
duration on the same frame, so it burns out in lockstep:

```
cells burning:  f+1..f+180: 520 (constant, unvarying)   f+210: 0
```

It appears instantly at full strength, does not move, expand, shrink, or
fade, and then vanishes all at once. That is what makes the stone filmstrip
read as a frozen decal.

Compounding it: `ignite_circle` is the M14 *debug force-ignite tool* and
ignores `flammability` entirely — already a known bug (README M15 caveats,
and `explosion.rs`'s own comment). On stone, which has `flammability: 0.0`,
the burning ring **is** the explosion; it is the dominant visual of the whole
effect and it should not exist.

### 5. Two of the three things an explosion writes are inert

The design is "a pressure impulse, a temperature spike, and a radius
converted to debris." Verified by grep across the crate:

**Pressure has one gameplay consumer: `explosion::debris_velocity`, at
trigger time, before the field has taken a single step.** Everything after
that — the shockwave propagating and reflecting across the world, which
`explosion.rs`'s own module doc calls "the whole reason for building it" — is
read by nothing. `particle.rs` deliberately does not couple to the field, and
no CA rule reads pressure. Its only other readers are the F3 debug overlay
and the HUD inspector.

**The temperature spike heats no cell.** `add_heat` writes the *field*.
`fire.rs::diffuse_heat` deliberately does not read the field (removed for a
measured 16ms→64ms regression), `try_ignite` gates on `cell.temperature()`,
and `render.rs`'s heat glow reads `cell.temperature()` too. So the spike
ignites nothing and glows nowhere; its only readers are worm flee-behaviour,
moisture evaporation, and the overlay. Every flame you see comes from
`ignite_circle` in (4), not from the heat.

Debris is the only one of the three that does anything, and per (2) it lands
on frame 1.

### 6. No smoke, no dust, no residue

`SMOKE` exists (`kind: Gas`, rises, `dispersion: 3`), and `field.rs`'s
advection doc specifically describes itself as "what actually carries smoke
sideways on wind." **Nothing in the simulation ever creates a smoke cell** —
not fire, not explosions; grep finds only tests and the player brush. Once
debris lands and the ring burns out there is no trace the explosion happened.

### 7. Every explosion is identical

`App::explode` uses a fixed `STRENGTH = 180.0` and the brush radius, with no
variance of any kind and no per-material response.

## Where this leaves the fix

Ranked by expected payoff against what `CLAUDE.md` requires me to state —
frame cost. **None of these costs are measured yet**; they are the estimates
a proposal has to open with, not results.

1. **Give the blast a duration.** It currently happens entirely on one frame.
   Nothing else on this list matters as much: an explosion is a *sequence*
   (flash → expanding front → debris → settling residue) and this one has no
   time axis at all. Cost: keeps a small region awake for ~10-20 frames
   instead of 1 — real but bounded, and the region is awake anyway while
   debris flies.
2. **Write CA cell temperature, not just field temperature.** Makes the
   detonation frame actually flash (the renderer's heat glow reads exactly
   this), and routes ignition through `try_ignite`'s deterministic
   temperature path, which respects `ignition_temperature` — fixing (4)'s
   flammability bug in the same change. This is the fix `explosion.rs`'s own
   comment already proposes. Cost: a bounded per-blast write, nothing
   per-frame; strictly cheaper than the `ignite_circle` call it replaces.
3. **Stop debris from dying on frame 1.** 2513 particles for a 20-cell blast
   is far more than can be read on screen, and 86% of them exist for a single
   frame. Fewer, longer-lived, faster debris is both better-looking and
   *cheaper*. Cost: negative, if the count comes down with it.
4. **Emit smoke in the crater.** Gives the blast an aftermath and a plume,
   and gives `SMOKE` its first producer. Cost: gas cells keep their chunk
   awake while they rise — the one item here with a genuine ongoing frame
   cost, and worth measuring before committing.
5. **Jitter the fireball's burn durations** so it fades raggedly instead of
   switching off. Cost: none.
6. **Couple field pressure to something** — gas, or particles in flight. The
   largest change, and the only one that makes the shockwave the field
   already computes do any work. Explicitly out of scope of a first pass;
   `particle.rs`'s no-coupling decision is deliberate and documented.

Items 1, 2, 3 and 5 together are the ones that address the images directly,
and none of them adds a per-frame cost to a settled world.

---

# Round 2 — the depth cliff

Owner's sharpening of the complaint, which turned out to name the single
biggest cause: *"if you're not right on the edge, most of the sand or water
around the explosion doesn't move so it just kind of looks like a small
vaporizing of particles and then a little collapse and you have to be so
close to the edge to actually get material to blast around. it just doesn't
happen if you're not really close."*

Reproduced on a flat 512-wide bed of one material, charge buried at a
measured depth below the free surface, everything allowed to settle for five
seconds. New `filmstrip` scenes `sandbed` / `waterbed` exist for this.

## It is a cliff, not a falloff

Cells of material thrown clear of the blast **and its shockwave annulus**,
`radius = 20`, `strength = 180`:

| cover above the charge | sand | water |
|---|---|---|
| 2 | 2 | 0 |
| 6 | 7 | 0 |
| 15 | 33 | 0 |
| 30 | **0** | 0 |
| 60 | **0** | 0 |

Past roughly 15 cells of cover it is not "less" — it is **exactly zero**, and
for water it is zero at every depth including two cells down. Meanwhile total
material *disturbed* rises with depth (69 → 686 cells), because that is
collapse into the hole, not ejecta. "A small vaporizing and then a little
collapse" is a literal description of the measurement.

## One structural cause: nothing in this engine can move through material

A CA cell only ever moves into empty space, by design. A free particle lands
the instant its next substep is occupied. Every mechanism that could throw
material converts a cell into a free particle — and a buried blast is
surrounded by occupied cells in every direction, so:

- crater debris flies as far as the crater rim and re-embeds (measured in
  round 1: furthest travel 30 cells from a 20-cell blast),
- the shockwave annulus (step 2.5) converts loose cells that are *already*
  enclosed, so they land on frame 1,
- and nothing else can move a settled CA cell except gravity.

Material therefore only blasts around where the launch vector happens to
point at open air — i.e. right at the surface. Exactly as reported.

## Prototype: a pierce budget on free particles

`Particle::pierce` — a count of *loose* (`Powder`/`Liquid`) cells a particle
may punch through before it has to come to rest, spent one per cell and
costing `PIERCE_SPEED_RETENTION` (0.82) of its speed each time, so debris
decelerates inside cover instead of crossing any thickness of it for free.
`explosion::pierce_budget` scales it from `strength`, not `radius`. Solids
and plants are never pierceable — the same line the shockwave step already
draws.

**An exchange version was tried first and rejected by measurement**: deposit
the carried grain in the cell being left, pick up the one being entered. It
conserves mass locally and drags a visible channel outward, but it deposits
at every step, riddling the pile with displaced grains and holes that read as
static rather than motion, and it transports material only one cell per
exchange however far the particle flies. It tripled "material disturbed"
while leaving "material thrown clear" at zero — the wrong one of the two to
move. Carrying the grain through is also cheaper: no CA writes per pierce.
`land` gained a small bounded nearest-empty ring search, because a particle
that runs out of pierce mid-pile is embedded by construction and the old
"neither position available, drop it" branch would silently delete a grain on
every deeply buried blast.

### Result, sand, radius 20

| cover | crater still open after 5s | material evacuated from 3× radius |
|---|---|---|
| | before → after | before → after |
| 2 | 81% → **100%** | 564 → **1332** |
| 6 | 74% → **100%** | 671 → **1502** |
| 15 | 47% → **92%** | 771 → **1861** |
| 30 | 0% → **29%** | 703 → **1833** |
| 60 | 0% → 0% | 170 → **566** |

Material evacuated roughly doubles-to-triples at every depth, and craters
stop closing up. Water improves much less (55%→61%, 18%→25% held) because a
liquid flows back regardless — correctly. A charge under 60 cells of cover
still just collapses, in both: the depth falloff is preserved rather than
erased, which is the intent.

**Frame cost: no meaningful regression.** Worst frame across the whole sweep
sits at 3.2–6.2 ms both before and after, typically +0.2–0.5 ms. (Both runs
show one ~35 ms first-iteration outlier at different rows — warm-up, not the
mechanic.) `examples/ascii.rs`'s standing stress numbers are untouched, since
no piercing particle exists in those scenes. Particle counts do rise sharply
during a blast (1346 → 7213 in flight one frame after four simultaneous
radius-20 charges), which is the thing to watch if debris counts ever grow.

## The other half: the default explosion is far too small

`App::explode` uses `brush_radius`, which defaults to **6** — a 113-cell
crater. That is a sensible painting default and a terrible explosion default.
At radius 6 the effect is four faint orange smudges on an undisturbed
surface, with **zero particles in flight two frames after detonation**, and
the pierce fix barely changes that (it does raise crater retention 58%→99% at
2 cells of cover and roughly triples evacuation, but 149 moved cells is
nothing on a 512×320 screen).

No amount of correct physics on 113 cells will read as an explosion. **The
blast radius should be decoupled from the brush radius** — or the default
raised a long way. This is probably worth doing before anything else on the
round-1 list, because it is one line and it gates whether any of the rest is
even visible.
