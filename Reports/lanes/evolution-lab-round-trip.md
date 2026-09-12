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

`assets/materials/nest.ron` gains `water_capacity: 1000` — the door is worked
soil and drinks rain like the ground it was made from. `weather.rs`'s
held-water arm widens from `Powder` to `Solid | Powder`, because `nest` is a
`Solid` and the conservation ledger would otherwise read every absorbed drop
as a leak. Ablate with **`PIXEL_PHYSICS_NEST_DRAINS=off`**, which reproduces
the pre-fix run **row for row** — one binary, two arms.

Guards: `update.rs::the_nest_patch_drinks_the_water_standing_on_it` (four
separate soakings, so a door that fills once and then saturates fails) and
`water_held_in_the_nest_patch_stays_on_the_conservation_books`. Both watched
going red under the ablation.

Before / after at 120,000 frames, `nestdoor`:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| water on the patch ÷ beside it | **5.2x → 1.0x** | 2.2x → 0.8x | 3x → ~1x |
| nest cells with air beside them | **5–6 → 15–17** of 53 | 8–26 → 1–15 of 44 | 21–27 → 9–16 |
| `nest_visits` | **731 → 2,589** | 5,810 → 5,853 | 4,320 → 3,378 |
| deliveries | 232 → 294 | 7,967 → 4,762 | 2,355 → 3,707 |
| born / alive at 120k | 139/36 → **175/78** | 210/129 → 599/397 | 712/501 → 319/146 |
| deepest generation | **7 → 11** | 7 → 21 | 20 → 16 |

Read the **top two rows** and not the bottom three: the door census is the
direct measurement of the mechanism and moves the same way on every seed,
while the colony outcomes are three samples of a chaotic process — two runs
that diverge on one frame are different worlds by the next.

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
- **`rain=0` clears the door on its own** (seed 1: water 51 → 0 and `airy`
  2 → 53 by frame 20,000) and is the control, never a fix — the bed's plants
  need the mister and that arm crashes the colony for its own reasons.

## Where to look next

The residual is in the walk, which another lane owns: an ant has no idea
where home is, and channel A's home trail is laid only by animals that have
touched the nest recently. Once the whole colony loses contact the trail
evaporates and contact is re-established only by a random walk stumbling onto
the patch — which is what the bursts in every column look like. `nestdoor`
prints *closest anyone came* per window for exactly this question.
