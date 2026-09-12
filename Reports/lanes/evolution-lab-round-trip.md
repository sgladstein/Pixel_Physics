# The round trip — why nothing came home, round 29

*2026-09-12. Closes the first half of `open-bugs-handoff.md` §T2 and opens a
second, narrower bug under it. Instrument: `examples/nestdoor.rs`.*

## The diagnosis, in one line

**The colony's doorstep was the only impermeable ground on the bed, so it
stood under a permanent puddle, and an ant cannot step into water.**

`creature::paint_nest_patch` converts ~53 surface columns into `nest`.
`nest.ron` authored **no `water_capacity`** — the default 0, "holds none at
all, and never absorbs an adjacent `Liquid`" — while `soil` and `packedsoil`
both hold 1,000. The mist soaked in everywhere else and stood on the door.
`creature::landing_is_placeable_through_tissue` wants `world.is_empty`, and a
liquid cell is not empty, so the film is a **wall**: `adjacent_nest` goes
false for every animal in the colony, and `AtNest`, `nest_visits` and
`deliveries` freeze on the same frame. That is the exact shape §T2 reported —
two counters identical at 9,000, 30,000 and 120,000 frames while `pickups`
kept climbing.

## What the counters ruled in and out

`nestdoor`, played bed, `RAYON_NUM_THREADS=1`, 120,000 frames, sampled every
10,000. Each hypothesis got a counter that can move only under it.

| hypothesis | counter | verdict |
|---|---|---|
| the door is **buried by spoil** | `covered by` histogram over the patch | **no** — the cover is `water` 47–49 of 53 on seed 1; `packedsoil` is 1 |
| the door is **dug away** | original patch cells no longer `nest` | **no** — `lost 0` on seed 1 for the whole run (seeds 2–3 lose 6–10 to seedlings and digging, and still deliver) |
| the door is **drowned** | free liquid over the patch vs the same width of ordinary ground beside it | **yes** — seed 1 **89–91 against 17–19**, a 5.2x excess that is the patch's alone |
| **nobody paths home** | distinct ants 8-adjacent to a nest cell per window | fires *as well*, and separately — see below |
| the laden ant **unloads en route** (§T2's own leading hypothesis) | pickups / drops / deliveries per window | **stale** — the `drop_urge * moisture_gradient` product §T2 names has not existed since 2026-09-02; the drop is one probability wherever you stand |
| **only founders delivered** | `life.deliveries` split gen 0 / later | **no** — later generations deliver once the door is open (seed 1: 0 → 132 after the fix) |

The seed split §T2 could not explain falls straight out of the water pair:
**seed 1 floods (89 vs 17) and never booms; seed 3 does not (2–3 vs 0–1) and
booms.** Same code, same bed.

## The fix

**Drains in the threshold.** `paint_nest_patch` leaves every third column as
the ordinary ground it was, so a one-cell film has **one cell** to travel
before the earth drinks it, instead of 26. Nothing else changes: not the
material, not the ledger, not the moisture field, not what germinates where.
Ablate with **`PIXEL_PHYSICS_NEST_DRAINS=off`** (`=<n>` sweeps the period),
which lays the unbroken patch that shipped until today — one binary, two arms.

**Two more principled fixes were built first and are worse.** Both are in
`dead-ends.md` with their conditions, and both are about `nest.ron` rather
than this loop:

- giving `nest` a `water_capacity` while it is a `Solid` **aliases
  `Cell::aux`**, which on a `Solid` is the structural anchor distance that
  `structural::tick` rewrites — and roots at 0 the moment powder touches the
  underside, which a surface patch always has. The door becomes a water
  *sink*, deleting liquid every frame, and every counter that was looking
  read it as a clean win;
- making `nest` a `Powder` so that combination is legal turns
  `player::footing` from `Hard` to `Soft`, and the gnome wades through the
  bottom of a nest wall (`a_nest_still_stops_him`).

Guards in `creature.rs`: `rain_does_not_stand_on_the_nest_patch` (two
soakings, because a door that drains the first and not the second is the same
bug on the second day) and
`the_nest_patch_is_still_continuous_enough_to_walk_home_to`. Both watched
going red with the drains removed.

## What is NOT fixed, and do not re-derive it

**Deliveries still fall to zero for long stretches with the door standing
open.** Seed 1 after the fix: windows 30,000–90,000 read `stnd` 16–34 of 53
nest cells with somewhere to stand beside them, `airy` 24–34 — and **not one
ant within eight cells of the door for 60,000 frames**. Seed 3 does the same
from 80,000. So door access and home-finding are two bugs, and only the first
was §T2's.

Three things this lane measured that a later session should not repeat:

- **Burial by spoil is not the binding constraint.** Across the after arm the
  correlation between `stnd` and ants-at-the-door runs the *wrong* way (seed
  1: `stnd` 34 → 0 ants at 30,000, `stnd` 15 → 17 ants at 100,000). A rule
  refusing a pellet over a nest cell was therefore **not** built: it would
  remove ~13 of ~46 covered cells and buy nothing measurable.
- **The nest patch is now a germination bed.** It held no water, so nothing
  could sprout on it; it does now, and seed 1's door reads `grassblade`
  14–17 and `grassroot` 5–9 by mid-run. Access is unharmed (`airy` 24–34),
  and the garden-midden and late-game lines both *want* plants on the mound —
  but it is a real change to what the anthill looks like and it was caused
  here.
- **`rain=0` clears the door on its own** (seed 1: water 51 → 0, `airy`
  2 → 53 by frame 20,000). A control, never a fix — the bed needs the mister.
- **The patch now feeds the coarse moisture field** (`field.rs` builds
  `moisture_source` from `aux / water_capacity`, kind-agnostic), so the ground
  at the door reads *wet* to `MoistureGrad` where it used to read like bare
  rock. That channel is a free weight on `Drop` and `Dig` in the genome, so
  part of the behaviour change is this and not the access — do not attribute
  all of it to the door.

## Where to look next

The residual is in the walk, which another lane owns: an ant has no idea
where home is, and channel A's home trail is laid only by animals that have
touched the nest recently. Once the whole colony loses contact the trail
evaporates and contact is re-established only by a random walk stumbling onto
the patch — which is what the bursts in every column look like. `nestdoor`
prints *closest anyone came* per window for exactly this question.
