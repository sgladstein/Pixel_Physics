# What a nest should be — a research brief, handed from the druid program

*Written 2026-09-14 by round 36's coordinator, on a handoff from the druid
program coordinator. **This is research for a future round, not round-36 work**
— it is filed so it is not re-derived, and so a druid lane can see the decision
when it is made. Nothing here is started.*

## The owner's question, verbatim

> *"think about how nest is attached to an area. if the actual material or cell
> moves, if the ground is dug up. My thought is it should be attached to a
> world location, not a material and it should be a circle or blob so if the
> ground gets dug, it can still be reached but do more research first (how do
> real ants identify home (i think it is proximity to the queen so doesn't work
> for us) but think if there is a more elegant solution. Ants in this game
> mostly move to where food is so nest doesn't really do much (although that
> could also be related to our issues with pheromone trails that we are
> fixing."*

And earlier, on a founding card he rated 1 of 5:

> *"Why do we need a physical nest at all? I think it also stops ants from
> digging. It should just be soil where you place the ants, cannot we just have
> a separate marker that this is the nest spot?"*

## What is already true — established by the druid coordinator, do not re-derive

- **The nest is already a sensed marker, not a structure.** `nest.ron`'s own
  doc: *"the nest in v1 is a sensed material, not a structure with behaviour.
  `AtNest` is a contact scan for this material and nothing else"*, and *"no ant
  ever asks where the nest is"* — homing is the channel-A gradient scaled by
  nest-touch recency.
- **`AtNest` is literally `adjacent_nest`** (`creature.rs` ~6629): an
  8-neighbour test that the cell's material id equals `def.nest`. That is the
  entire coupling, and it is the whole surface a redesign has to replace.
- **A logical site already exists beside it, unread.** `paint_nest_patch` calls
  `register_nest_site(x, y, half_width)` **before** painting any ground, and
  `world.rs`'s `NestSite` carries `x`, `y`, `scent: [f32; 3]`, `seeded`,
  `drift_epoch`. **The owner's "separate marker" is half-built** — what is
  missing is that nothing reads it for *location*.
- **The patch is a one-cell skin, not a roof** — one surface cell per column
  across ~53 columns, skipping every `nest_drain_period()`-th. It roofs nothing.
- **He is right that it blocks digging, and the margin is large.** The dig gate
  is `penetration_resistance <= dig_force_of(...)`. `nest` is **6.0**; every
  shipped species authors `dig_force: 1.0` (beetle 0.3), against soil 0.8,
  packedsoil 0.95, sand 1.4. **The crust is 6x the strongest jaw that ships**,
  and `TRAIT_DIG_FORCE` is in `ARMS_RACE_SLOTS` so `1.0 + allele_bound` is the
  ceiling. **A colony cannot excavate its own doorstep.** That is a
  lab-visible outcome and is arguably worth filing on its own, separately from
  any redesign.
- **The drainage comb is a workaround for the same fact.** `nest` is `Solid`
  and impermeable while soil is a `Powder` water percolates through, so an
  unbroken patch is a waterproof membrane with no slope: **52–68 cells of
  standing water on it against 0–4 over the ground beside it**, and the colony
  stops delivering. `PIXEL_PHYSICS_NEST_DRAINS=off` is the paired arm.

## How real ants actually find home — the part he asked for

**It is not proximity to the queen**, so his own doubt was right, and the
mechanism it rules out is not the one to reach for anyway. Four real ones, in
roughly the order they carry a returning forager:

1. **Path integration (dead reckoning) — the elegant one, and the one that fits
   this engine.** A forager continuously accumulates a *home vector* from its
   own movement — direction and distance travelled — so at any moment it knows
   the straight line back, having never needed a landmark, a trail, or a
   marker in the world. Desert ants (*Cataglyphis*) run almost entirely on it
   and will march the accumulated vector even when displaced. **In engine terms
   it is per-ant state — a 2D accumulator reset on nest contact — and it is
   attached to no cell, so digging the ground cannot break it.** That is
   precisely the property he asked for, reached without a world marker at all.
2. **The nest entrance has its own odour**, colony-specific, recognised on
   arrival at close range. This is the real analogue of `AtNest`: a *local*
   confirmation once you are already nearly there, not a way of finding the
   place from afar.
3. **Trail pheromone**, for species that lay one — a corridor, not a beacon.
4. **Visual panorama and landmarks**, which we have no analogue for and should
   not invent.

**The shape that follows is a two-part homing rule, and it is the standard one
in the literature: a long-range vector that accumulates error, corrected at
short range by something local.** Path integration gets an ant to the
neighbourhood; the nest's own scent (or the existing channel-A gradient)
closes the last few cells. That also explains why a marker attached to a
*location* rather than a *material* is the right instinct — and why the marker
only has to work at short range, which is what makes a blob affordable.

## The sequencing decision, which is mine and is the point of this brief

**Settle the trail question before redesigning anything, because it decides
whether this is a nest question at all.** His own hypothesis says so: *"nest
doesn't really do much (although that could also be related to our issues with
pheromone trails)"*. Two measurements now sit under that:

- Round 36 Lane C (PR #432): **`DECAY_RHO` is inert and `DIFFUSE` is the real
  lever** — a trail is a live map rather than a memory.
- A druid lane, independently: a one-cell mark is **off the ground in ~3.5 s
  against a ~37 s round trip**.

**If a trail cannot survive a round trip, a nest gradient cannot either** — and
"the nest does nothing" would be a symptom of the trail, not a fact about
nests. Redesigning the nest first would be fixing the wrong object, and this
repo has a rule for that shape: *a scene that contradicts the code will look
like a bug in the code*. So the order is: land the trail work, re-measure
whether the nest still "does nothing", and only then decide.

## The constraint any redesign must not break

`earth_toned_nest`'s doc records it: the patch is **deliberately narrower than
the band of ants** — *"home has to be a place, not everywhere, or there is no
gradient to walk up"* — and the foraging scene measured **414 deliveries** at
that ratio. **A site- or blob-based `AtNest` that widens the footprint flattens
the gradient**, and 414 is the number to hold it against. That scene is the
lab's.

## What the held world needs from whatever lands

1. **Founding is a player verb at an arbitrary position.** `Druid::found_colony`
   places a colony at the gnome's feet on a keypress, anywhere, possibly on
   forest floor. A nest must be creatable on demand at a cursor, not only by
   worldgen or a scenario file.
2. **The shrunk gnome walks colony galleries.** He is 2x3 and the nest stops him
   today (`a_nest_still_stops_him` asserts it), while `rigid::is_tool_target` is
   true for any non-bedrock `Solid`, so his pick *can* cut nest. **If the
   material stops being `Solid`, both change**, and the walk-the-nests feature
   depends on which way.
3. **Tell the druid coordinator before it lands, not after** — a druid lane
   follows it.

**Appearance is out of scope**: `earth_toned_nest` already answers the
visible-stripe half and a druid lane has ported it.
