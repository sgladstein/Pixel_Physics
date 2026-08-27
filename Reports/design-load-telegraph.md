# The load telegraph — making strain a thing you hear and see before it falls

**Status: design, nothing built.** No engine change is proposed here that
computes anything new; every number this needs is already produced every frame
and thrown away.

## The defect, stated as `CLAUDE.md` states it

> **An outcome is a distribution, not a binary.** … Ask of any change: does
> this have a middle?

The structural model *has* a middle. `load::evaluate` returns a public
`Load { mass, moment, torque, capacity, supported, truncated }`, and
`torque / capacity` is a continuous ratio that says how hard a cell is working
— 0.4 is comfortable, 0.95 is about to go, 2.52 was `scene=ligament`'s neck
standing at two and a half times its limit.

**The player sees none of it.** There is no structural overlay (`render.rs`
carries `FieldOverlay` and `OrganismOverlay`; nothing for load), there is **no
audio in the repository at all**, and the only structural readout is a debug
string in `app.rs` — `BULK D{n} BACKGROUND`. So a rich distribution is
delivered through one bit: *it fell, or it didn't.*

That is the ethos violated at the presentation layer while the simulation
underneath already satisfies it.

## The thing being wasted

`CHAIN_WINDOW_FRAMES` is **600 frames — ten seconds** — of deliberate
generosity, and the owner's stated reason for it is *"collapse must be obvious
and delayed, so the player can get supports in first."* The engine is already
paying for a ten-second warning that no other game can offer, and currently
spends it on silence. The first thing the player learns about a failure is the
rock arriving.

## The design

Three channels, in order of value per unit of work. All of them read the same
ratio and none of them is a debug view.

### 1. Dust from the underside (visual, no audio needed)

A cell over some fraction of capacity sheds a little dust from any face
exposed to air. Rate scales with the ratio, so the eye reads *how bad* without
a number:

- the particle system already exists and already carries a cell's `aux`
  correctly (`ParticleSystem::spawn_from_cell`);
- the emitter is one call at the point `evaluate` already computed the ratio;
- dust is a `Powder`/`Gas`, so it does not enter the support field and cannot
  feed back into what it is reporting.

**This is the one to build first** because it needs nothing the repo does not
have, and because it is *diegetic*: a heatmap tells the player about the
model, dust tells them about the rock.

### 2. Creak (audio, and the repo has no audio at all)

Map the ratio to creak density and pitch. This is the single largest missing
channel in the project and the cheapest information a player ever gets: sound
carries off-screen, needs no pixels, and does not compete with the render.

It is also the biggest *new* dependency in this document — there is no audio
subsystem, so this is "add audio to the engine", not "add a sound". Scope it
as its own decision.

### 3. Hairline cracks that mean something

`Cell::crack_right` / `crack_down` already exist, already remove edges from
the relaxation, and are already rendered. A cell approaching its limit could
score a crack **on a face that is not load-bearing** — visible, honest,
and structurally inert until it isn't.

**Careful here, and this is the trap:** a crack removes a support edge, so
scoring one on a loaded face *causes* the failure it was supposed to warn
about. The rule has to be "score the crack where support is not coming from",
which `support_parent` answers directly. Get that backwards and the telegraph
becomes the trigger.

## What it costs, and the two constraints it must respect

- **Frame cost is a hard constraint, not a tiebreaker.** The ratio is free —
  `evaluate` already computed both terms — but *emitting* is not. Gate the
  emitter at the call site that already holds the `Load`, never by re-walking.
- **The dirty-rect render skip is the thing to protect.** `CLAUDE.md` records
  an animated grain that "looked free in every moving scene and cost ~10 ms a
  frame on a *settled* one, because what it defeats is the dirty-rect render
  skip". A telegraph that emits on a settled world does exactly that. So:
  **only cells that are actually near their limit emit**, and a world with
  nothing straining must be bit-identical to today. That is a measurable
  requirement, not a hope — see below.

## The falsifying experiment, cheapest first

| question | how | what falsifies it |
|---|---|---|
| **does it cost the settled frame?** | `scale_probe phases=1` on an idle world, telegraph on and off, paired and alternating | the settled frame moving at all — a quiet world must not emit |
| **does the warning actually precede the failure?** | log the first frame a cell crosses the emit threshold against the frame its region fails, on `acceptance.sh`'s existing collapse cases | a median lead time near zero: a telegraph that fires as the rock leaves is not a telegraph |
| **is the threshold in the right place?** | histogram `torque/capacity` over a live world; pick the emit threshold from the distribution, with headroom | everything or nothing emitting — a threshold that fires on 40% of the world is decoration |

**Run the lead-time one first.** It is the whole point of the feature and it
is the one that can come back "the model does not know early enough" — in
which case the delay is a queue artifact rather than a physical margin, and
the feature needs `structural-support-model.md`'s convergence work landed
first.

## The judge-by-eye question

Everything above is a *tension* claim, and tension is not measurable. Before
tuning anything, put a blind A/B in front of the owner through
`scripts/review.py`: the same collapse with and without dust, as frames rather
than a GIF, with the failure counts in the card's `meta`. The question is not
"is the dust visible" but **"did you know it was coming"**.

## Why this is the first of the four

`arch-vs-lintel-measurement.md` shows the model already rewards a better shape
by 1.6x at equal material. **A player cannot discover that without feedback.**
Right now the only signal that a flat roof was a mistake is the roof; with a
telegraph, the flat roof groans and the arch does not, and the lesson is
available in one build instead of two.
