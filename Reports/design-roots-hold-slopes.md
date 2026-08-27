# Roots hold slopes — and the timing is the whole feature

**Status: design, nothing built.** One finding here changes the shape of the
idea before it starts: **the structural support field cannot hold a soil bank,
because soil is a `Powder` and the field only governs `Solid | Plant`.** The
mechanism has to be the powder rules instead, and that turns out to be both
more correct and cheaper.

## The correction, first

The obvious reading of "roots hold slopes" is that root cells anchor soil into
the support DAG. They cannot:

- `structural::is_body_material` is `Solid | Plant` only. `soil.ron` is
  `kind: Powder`, so it is not in the support field at all and has no `aux`
  distance — its `aux` is moisture (`material::SOIL_SATURATED`).
- `grassroot.ron` and `rootwood.ron` *are* `kind: Plant`, so roots themselves
  are body material and are held up by `plant::anchor_support`. But a root
  being supported says nothing about the dirt around it.

What actually governs a bank is `Material::friction_angle` — `soil` sets
**33.0°** — and the `max_stability_angle` derived from it
(`friction_angle + DEFAULT_STABILITY_ANGLE_GAP_DEGREES`), which is what the CA
sweep consults when it decides whether a grain slumps.

**So the design is: roots raise the local stability angle of the soil they run
through.** That is not a workaround — it is what root reinforcement actually
is in soil mechanics (roots add apparent cohesion; the slope stands steeper
than its bare angle of repose). The engine's knob and the real phenomenon are
the same knob.

## Why the timing is the feature

Deforest a hillside in the real world and it does not slide that afternoon. It
slides *years later*, when the root network has rotted and the apparent
cohesion is gone. Landslide risk after logging peaks well after the logging.

**This engine already has that clock.** `CLAUDE.md` records it as an owner
ruling on the plant line:

> a tree that cannot pay its maintenance is marked senescent and carried out
> by `rot_remains` at the species half-life, **so the death is graded rather
> than a disappearance**

So the sequence falls out of machinery that exists:

1. Player clears the trees on a bank. **Nothing happens.** The roots are still
   in the ground.
2. `rot_remains` carries them off at the species half-life. The reinforcement
   decays with them — graded, not switched.
3. The bank passes its bare stability angle and goes.

A consequence the player caused, that arrives late enough to be surprising and
early enough to be attributable. That is a rare thing to get for free, and it
is the reason this idea is worth more than "trees make dirt sticky".

And it runs the other way: replant, and the slope recovers on the same clock.
**A verb with a slow, visible, reversible consequence** — which is the second
law satisfied on a subsystem where the player is currently a spectator.

## The design

- Root cells contribute a **reinforcement scalar** to nearby soil.
- The soil's effective stability angle is its material `friction_angle` plus a
  bounded bonus from that scalar.
- The scalar decays as roots rot and grows as they spread. It is never
  negative and never unbounded: a bank held by roots is *steeper*, not
  immortal.

### Where the scalar lives, which is the real cost

This is the part to get right, because the naive version is a per-cell
neighbourhood scan in the hottest loop in the engine.

- **Not `aux`**: on a `Powder` that is moisture, and `CLAUDE.md`'s
  two-conventions gotcha is emphatic about what happens when a slot is
  reinterpreted.
- **Not a per-frame "is a root adjacent" probe**: that is exactly the cost
  `CLAUDE.md` names — *"Gating inside the function still pays a `World::get`
  and a lookup per cell per frame."* Guard hot-path work at the call site that
  already has the data.
- **The coarse field layer (`World::fields`) is the natural home** — a root
  density channel, written when roots grow and rot, read by the sweep.

**And there is a trap on that route which has been hit four times.** A
coarse-field read is block-nearest, so neighbouring cells sample the same
value, and *"never build a per-cell decision on the difference between two of
them"*. Here the read is a **threshold/scalar**, not a gradient — the sweep
asks "how reinforced is this cell", never "which way does reinforcement
increase" — so it is a legitimate use. That distinction has to be written down
at the read site or the next reader will assume it was missed.

**A channel needs a writer and a reader, and the compiler checks neither.**
`dead-ends.md` calls this the failure this project has hit three times: light
with no writer, canopy density with an always-zero reader, pressure with no
consumer. Name both ends before building: the writer is plant growth and
`rot_remains`; the reader is the powder slump rule in `update.rs`. If either
is missing, say so rather than assuming the other end exists.

## What it connects

- **Weather and water.** Wet soil is heavier and weaker; `soil` already
  carries saturation in `aux`. Rain on a deforested bank is the classic
  failure and both halves already exist.
- **The gnome.** §C1 in the bug register is *"A forest-floor bank is a wall
  the gnome has no way over"*. Banks are already a terrain feature the player
  bumps into; giving them a cause and a cure makes them a system instead of an
  obstacle.
- **Fire.** A burnt slope is a deforested slope on a much faster clock.

## The falsifying experiments, cheapest first

| question | how | what falsifies it |
|---|---|---|
| **does a soil bank slump at *runtime* at all?** | build banks at a spread of angles, run to rest, census the profile | if soil only ever settles during worldgen's erosion pass and is inert afterwards, there is nothing for roots to hold and this design is dead as written |
| **does the stability angle actually move the outcome?** | the same banks, sweeping `friction_angle` directly in `soil.ron` | a bank whose resting profile does not move with the angle means the knob is not connected — and remember `.ron` edits need a rebuild, or the sweep produces bit-identical "runs" |
| **is the effect visible at play scale?** | the reinforced-vs-bare profile, rendered | a couple of degrees. A slope that stands 1° steeper is correct and invisible, which by this project's standards is not finished |
| **does it cost the sweep?** | `ascii`'s worst-frame timing, and `scale_probe phases=1` for the whole frame | any measurable cost on a world with no plants in it — the reinforced path must be free where there are no roots |

**Run the first one before anything else.** It is a scene and a census, it
needs no new code, and it can come back "powders do not slump at runtime", in
which case the honest answer is that this feature needs a slump rule before it
needs roots. `CLAUDE.md`'s *"a scene that contradicts the code will look like a
bug in the code"* cuts the other way here too: check the mechanism exists
before designing on top of it.

## The judge-by-eye question

**Does the delayed collapse read as consequence or as a bug?** A slope that
falls ten minutes after the player cut the trees is either the best moment in
the game or an inexplicable disaster, and which one it is depends entirely on
whether anything connects the two events. Options to put in front of the owner
as a blind A/B through `scripts/review.py`:

- no signal at all — pure emergence;
- the bank visibly creeping first (the graded middle, and the same telegraph
  `design-load-telegraph.md` proposes for rock);
- dead roots visible in the soil face as they rot.

My guess is the middle one and I would not build on that guess. The card
should show the same slope failing three ways and ask which one the player
would blame themselves for.

## Scope note

This is the largest of the four designs and the only one that touches a
subsystem outside structure. It is also the one with the highest chance of
being blocked by its first experiment. **Sequence it last** — behind the
telegraph, the prop, and sounding — unless the runtime-slump question comes
back positive and cheap.
