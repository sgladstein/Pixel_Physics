# How the ant works

**A living reference: what the shipped ant (`assets/species/ant.ron`) does on
every tick, how each mechanism is implemented, and what it reads.** It is
written from the source and describes the code as it is now, not as it was or
will be.

- **Verified against:** `main` at `bb65d507`, 2026-09-22. §2, §6c, §13 and
  §15 re-checked the same day against the decision-trace change
  (`usable_headings`, `home_weighted_pick_why`); §5, §6b, §6c, §13, §14 and
  §15 re-checked 2026-09-23 against the drop, cone and homeward counters;
  §5, §12 and §15 again the same day for the drop through bodies, and §9
  for what a part-eaten cell is worth once put down (re-checked the same
  day against the crumbs fix in `Carried::into_cell`). §6d and §12 written
  2026-09-23 from `chooser_step`, `fall_if_unsupported` and `commit_step`;
  §6d, §12 and §15 again for stage 2 (`trail_presence`, `brain_inputs`),
  and §9 for crumbs not sliding (`update_powder`'s `rolls` gate); §9 and
  §12 for the bud-at-nest switch (`try_bud`, `bud_at_nest`); §6d and §12
  for `trailaway` (`chooser_step`'s `away_home_cos`, `AWAY_GAIN`). §1, §4,
  §5, §6, §10, §11 and §12 rewritten 2026-09-24 when `trailaway` and the
  `Drop` hunger wires became the default (`chooser_from_env`, `chooser_for`,
  `ant.ron`'s `Drop` row). §9 and §12 2026-09-25 for what a load weighs
  (`carried_cells`, `crop_load_cells`, `load_scale`, the digest block), and
  §5 and §9 again when `crop_capacity` 5760 and `food_weight` 0.5 became the
  ant's default (`ant.ron`, `CreatureDef::food_weight`). §8 and §12 on
  2026-09-26 for the nest-door switch (`nest_door`, `paint_nest_patch_with`,
  `found_colony_with`, `colony_stations_with`), and again that day for the
  founding cut as home (`adjacent_nest`, `nest_home`, `NestSite::shaft`) and
  the door's anchor over a shaft (`found_colony_with`). §6d and §12 on
  2026-09-26 for scouting (`chooser_step`'s `scout_w`, `scout_cos`,
  `SCOUT_GIVE_UP`, `scout_of`). §5 on 2026-09-26 for what a delivery counts
  and `pickups_at_nest` (`act`'s feed and drop branches).
  §6d, §8 and §12 on 2026-09-27 for scouting as the default
  (`SCOUT_DEFAULT`), the hungry-home switch and the nest larder
  (`update_hungry_home`, `hungry_target`, `NestSite::larder`, the delivery
  block in `act`).
  §5 steps 5-6 on 2026-09-27 against `act`'s spoil and dig branches,
  `lift_reach` and `line_burrow`: the spoil roll is `DropSpoil`, and lining
  takes spoil as well as soil.
  §4, §5, §6d, §8, §12 and §15 on 2026-09-27 for the forage drive
  (`forage_drive_from_env`, `forage_drive_level`, `forage_pace`,
  `nest_needs`, `World::step_nest_need`, `OrganismState::foraged`, `act`'s
  feed urge under `,keep`) and the trace's `energy_j` and scout columns; §4's
  `P(move)` table corrected the same day for `Crowding` at the nest.
  §5 steps 5-6 and §12 again on 2026-09-27 for the spoil footing and packing
  switches (`spoil_site_open`, `is_footing`, `spoil_footing_drop`,
  `pack_neighbours_with`, `spoil_packs`), and again that evening when the
  lining stopped packing spoil by default.
  §6d and §12 on 2026-09-27 for the forage drive and carry patience shipped
  on (`forage_drive_from_env`, `ForageDrive::SHIPPED`,
  `carry_patience_from_env`, the pickup block in `act`). §3 and §4 on
  2026-09-28 for the dig wiring (`ant.ron`'s `(Bias, Dig, -0.3)` and
  `(SurfaceCurvature, Dig, -1.0)`). §5 step 6 and §12 again that day for
  the heap cue (`spoil_cue`, `spoil_cue_factor`, `open_to_the_sky`).
  §5 step 6, §8 and §12 again that day for the founding shaft and the heap
  cue shipped on (`parse_nest_shaft`, `NEST_SHAFT_ROWS`,
  `dig_founding_shaft`, `cut_founding_shaft`, `founding_dig_force`,
  `parse_spoil_cue`, `SPOIL_CUE_SHIPPED`, `spoil_cue_of`). §5 step 6 and
  §12 again that day for the dig-down turn shipped on for an enclosed
  digger (`dig_down_bias`, `parse_dig_down`, `DIG_DOWN_SHIPPED`), and §5
  step 6 again for the turn refused where there is no way down
  (`way_down`, `jaw_can_cut`, `dig_down_of`, `DecisionScratch::dig_turned`).
  §9 on 2026-09-28 for eating at the nest store (`act`'s feed branch and
  drop roll, the drive gate in `creature_tick`, `DropWhy::NotAsked`). §6d,
  §9 and §12 again that day for packed lunch shipped on (`carries_lunch`,
  `packed_lunch_of`, `OrganismState::lunch` and `eat_lunch_now`, the drive
  gate, `home_pull`, `chooser_step`'s `laden`, the drop block and
  digestion's `progressed`). §9 and §12 again that day for the birth price
  (`try_bud`, `reachable_provision`, `birth_price_of`,
  `plant::guaranteed_bite_fraction`, the `Origin::Bud` arm of
  `place_creature`). §12 on 2026-09-28 for the storeroom switch, off
  (`storeroom_of`, `store_pickup_ok`, `store_drop`, `store_post_site`,
  `store_target`, `store_return_target`, `Spoil::store`), and its nest-bound
  part (`is_nest_bound` in `forage_drive_level`, `home_pull` and
  `chooser_step`'s `away_from`), its caste and worker-home parts
  (`nest_within_reach`), and `nest_shaft_offset`. §5 step 6 again that day
  for the heap cue standing aside for a cut into the floor under a roof
  (`spoil_cue_factor`). §12 again that day for the storeroom's `side` part
  (`SideRoom`, `ShaftFootprint::store_rect`, `store_target`,
  `World::cut_founding_shaft_with`), and its `keep` part (`store_kept`, in
  `act`'s feed branch); §6d and §12 again that day for the store-lunch switch
  (`store_lunch_of`, `act`'s pickup `from_home`) and the pellet switch
  (`haul_bite_blocks` in `act`'s feed branch). §5, §6d, §8 and §12 on
  2026-09-29 for the door and the full storeroom shipped on (`parse_nest_door`,
  `nest_door_of`, `World::nest_door`, `Storeroom::SHIPPED`,
  `parse_storeroom`), and §6d, §8 and §12 that day for the `returns` drive
  shipped on and store lunch held off (`returns_drive`, `RETURN_WINDOW`,
  `trip_load`, `nest_last_return`, `step_nest_need`, `ForageDrive::ALWAYS`,
  `store_lunch_from_env`), and §6d and §12 again that day for the trip reach
  shipped on (`trip_reach_of`, `trip_source`, `door_distance`, `trip_src`,
  `return_window`). §5 step 6 and §8 on 2026-09-29 for the half turn's
  side (`turn_toward`, `half_turn_left`) and the founding cut's jaw and
  corpses (`is_diggable_ground`, `founding_dig_force`,
  `paint_nest_patch_with`). §12 again that day: the walked cycle's and the
  lift's rows, which #517 left out, the carry away from the mouth
  (`SpoilOut`, `spoil_lift_mode`, `spoil_ring`), and tunnel widening
  (`dig_widen_of`, `dig_widen_site`, `dig_shoulder_site`). §7 and §10 on
  2026-09-29 for who lays trail B (`CarryingFood`, `carries_lunch`: a lunch
  carrier lays it), §7, §10 and §12 that day for the food trail's lay
  switch (`food_trail_lay`, `FoodTrail`), and §15 that day for the trail
  columns (`since_trip`, `DecisionRow::emit_b_laid`, `b_near`, `score`).
  §9 and §12 on 2026-09-29 for the breeding regimes, which neither section
  named before (`breeding_regime`, `breeding_radius`, `suppress_bar`,
  `graded_suppression_factor`, `colony_has_other_breeder`,
  `nearest_breeder`, `breeder_index_enabled`, and `OrganismState::children`).
  §7, §10 and §12 on 2026-09-30 for the lay rule shipped on
  (`FOOD_TRAIL_UNSET`), and §12 that day for `World::mute_emit_b`. §9 and
  §12 on 2026-09-30 for births on nestmates (`bud_stack_of`,
  `place_creature`'s `kin`, `births_on_kin`), and §12 that day for the
  carry's latch at the door (`carry_stage`, `ring_target`,
  `spoil_ring_let_go`).
  Update this line whenever a section is re-checked against the code.
- **Edit it in place. Never append history.** When you change a mechanism
  described here, update the section in the same commit. When you find this
  document was wrong, fix the text and say so in the commit message. It
  should not grow as time passes; it grows only if the ant does.
- **What does not belong here:** measurements, experiment results, open
  questions, plans, or "we used to think". Those go in a dated report,
  `open-bugs-handoff.md` or `dead-ends.md`. The one exception is §14, a
  short list of source comments that currently contradict the code, deleted
  line by line as they are fixed.
- **Sections name functions, not line numbers.** Line numbers go stale within
  a day in `creature.rs`.
- **The weights below are the founder genome as authored.** Every ant carries
  its own copy of the genome, and descendants drift from it by mutation
  (`mutation_rate: 0.0033758` per slot per birth). An evolved colony can
  differ from this page, and a measurement that matters should read the
  genome it ran.

---

## 1. The shape of one tick

`creature::creature_tick`, once per scheduled tick, in this order. Every step
can end the tick early.

1. **Housekeeping.** `reconcile_chain`. A burning ant only reschedules.
   **Old age:** `life_half_life: 40000` frames, rolled every tick. An ant
   mid-`Crossing` (inside a trunk) or mid-`flight` runs that branch instead
   of everything below.
2. **Sense.** `sense` fills the input vector (§3). If `AtNest`, the colony
   scent blends (`blend_with_nest`).
3. **Brain.** `brain::eval_brain` turns inputs into outputs (§4).
4. **Standing costs** are charged: idle, synapse, force and armour taxes.
5. **Act.** `act`: attack, share, feed, drop, dig, at most one of each and in
   that order, with early returns (§5).
6. **Move** (§6). The ant walks the chooser (§6d): the support check and
   any fall first, then one draw against `P(move)`: on success, a heading
   picked from every usable one; on failure, a pause. A species with no nest
   walks §6a–§6c instead: on success, step; on failure, maybe tumble.
7. **Lay trail**, only if the step succeeded (§7).
8. **Memory upkeep.** `phero_a_mem`, `since_nest += 1`, `still_ticks` (reset
   by a move, otherwise incremented).
9. **Digest** the crop (§9), apply the energy change, starve if energy ≤ 0,
   bud if eligible (`try_bud`).

**How often.** `organism_tick_interval` is `tick_interval: 6`, scaled by the
ant's expressed `TRAIT_PACE` and by its body's leg fraction. A founder decides
**once every 6 frames**, so the ceiling on any per-frame movement rate is 1 in
6. Pace mutates (width 0.15 per birth), so descendants differ.

## 2. The body and the world it can stand in

- **Body:** `Chain(2)`, a head and one segment. Headings are the 8 `DIRS`.
- **Support** (`step_chain`): the body is held up if **any** of its cells has
  a `Solid`, `Powder` or `Plant` cell among its 8 neighbours. Otherwise the
  ant falls one cell. **The fall happens only inside `step_chain`, so only
  after a successful move roll**: an unsupported ant with `P(move) = 0`
  hangs in the air. A fall counts as a move and lays trail.
- **Enterable cell** (`cell_is_enterable`): empty, its own body, or living
  plant tissue (`is_partable`: leaf, wood, grass, reed, moss and fruit are
  walk-through while alive; `TISSUE_PARTING` on). A nestmate is **not**
  enterable at the default `PIXEL_PHYSICS_STACK_DEPTH=1`. Parted tissue is
  held by the ant standing in it and closes when the cell is left empty; when
  an ant steps off or dies in a cell a nestmate still stands in (stack depth
  above 1), the nestmate holds the tissue instead (`close_or_hand_over`).
- **Foothold** (`head_has_foothold`): the **head's** 8 neighbours include
  `Solid`, `Powder` or `Plant`, or a nestmate (`climbs_over_kin: true`).
  Ants walk on walls and ceilings.
- **Usable heading** (`usable_headings`): enterable **and** footed. This one
  predicate decides what the ant can do in each setting, and both the tumble
  and the decision trace's setting class read it:

| Setting | Usable headings |
|---|---|
| Flat ground, 1-wide tunnel, flat wall face | 2: forward and back. The upper diagonal has no foothold; the lower one is ground |
| Corners, ledges, steps, the base of a stem, tunnel junctions | 3–5 |
| Inside a canopy (tissue is enterable and footed) | up to 8 |
| Burrow pocket, crowd | 0–1 |

## 3. What the ant senses

`sense`, evaluated at the ant's own cell `(x, y)` with its current heading.
Only the inputs `ant.ron` wires are listed. Everything else in
`brain::BrainInput` is computed and read by nothing in the ant.

| Input | How it is computed |
|---|---|
| `Bias` | 1.0 |
| `Energy` | `energy / start_energy`, clamped to 0..1. **`start_energy` is 200 while budding needs 1,100, so any fed ant reads 1.0**; only a hungry one reads less |
| `FoodAdjacent` | 1 if an edible cell is in reach of the mouth (`adjacent_food_counted`), else 0 |
| `KinNeed` | the energy deficit of the neediest adjacent nestmate, from the same scan |
| `AtNest` | 1 if nest material is among the 8 neighbours (`nest_within_reach`; radius 1 by default) |
| `Crowding` | **At the nest:** nest-room occupancy at the nearest nest site, `2 / (2 + roofed empty cells per ant)` (`room_target` 2.0), so 0.5 at two roofed cells per ant and rising as the nest fills. **Elsewhere:** creature cells within radius 2, divided by 8, capped at 1 |
| `Stillness` | `(still_ticks / 192)²`, capped at 1. `still_ticks` counts consecutive ticks without a move |
| `CarryingFood` | 1 if the crop holds any food, else 0 |
| `Carrying` | crop fill (`crop.worth() / crop_capacity`), or 1 while holding a spoil pellet |
| `HomeAligned` | laden only: the cosine between the heading and the vector from the head to `home_target`. 0 when the crop is empty or within 1 cell of the target. `home_target` is `forage_anchor` (§8) |
| `PheroAAlong`, `PheroBAlong` | `(front − here) / (front + here + 256)` on that trail plane. `here` is the scent at the ant's own cell. `front` is **6 cells** along the heading (`sensor_offset: 6`); for a diagonal heading on the ground it is projected onto the ant's own row (`trail_sample_point`). Forced to 0 when both planes read 0 at the front point and that point is inside solid, powder or plant (the "honesty" gate) |
| `Alarm` | the alarm plane at the ant's own cell |
| `TempAboveAmb` | (noon-equivalent temperature − ambient) / scale, clamped to −1..1 |
| `MoistureGrad`, `SurfaceCurvature` | used only by digging and spoil dropping |

**Computed and read by nothing:** `PheroAFront` and `PheroBFront`, the trail
strength 6 ahead, so **no ant responds to how strong a trail is**.
`PheroALateral` and `PheroBLateral`, right minus left at 6 cells along
heading ± 45°. On a surface these points are sky and rock, and the plane
holds scent there because it ignores terrain (§7). Also `PheroARise`
(laden only: trail A under the head now against `phero_a_mem`, a 0.995
running average), light, moisture-ahead, and every sight input. Founders
are blind (`sight_range` 0), but `TRAIT_SIGHT_RANGE` is heritable, so a
lineage can evolve eyes.

## 4. The brain

`brain::eval_brain`: one hidden layer of 8 units, each with optional
self-recurrence, and 16 outputs. `squash(x) = x / (1 + |x|)`.

```
hidden_h = squash(rec_h * hidden_h(previous tick) + Σ w * input)
output_o = squash(Σ w * input + Σ w * hidden_h)
```

**What the founder genome's hidden units do.**

| Unit | Wiring | Job |
|---|---|---|
| 0, 1 | `Bias −45, CarryingFood +45.5, PheroAAlong ±6`; out to `Move ±2.5` | **Laden only:** trail A's forward difference as a throttle on stepping. Shut when empty; the mirror pair cancels, so a shut pair adds ≈0 |
| 2, 3 | `Bias +0.5, CarryingFood −45.5, PheroBAlong ±6`; out to `Move ±2.5` | **Empty only:** trail B's forward difference as a throttle on stepping |
| 4 | `AtNest +0.05`, recurrence `0.99995`; out to `EmitA +32` | **Nest odometer.** Charges slowly while touching the nest and decays after leaving. It sets how much trail A the ant lays; the source's own fit quotes 0.819 falling to 0.010 over 3,000 ticks |
| 5, 6 | `Bias −30, AtNest +30, Crowding ±6`; out to `Dig ±2.5` | **At the nest only:** the less room per ant, the stronger the urge to dig |
| 7 | nothing | free |

The trail throttle's size, from those weights: `2.5 × (squash(0.5 + 6a) −
squash(0.5 − 6a))` for a forward difference `a`. That is ±1.5 at a = ±0.1,
±2.6 at ±0.2, and at most ±4.3.

**Outputs, their direct wires, and what they come to.**

| Output | Direct wires (plus the hidden units above) | Read by |
|---|---|---|
| `Move` | `Bias +2.0, Energy −1.75, HomeAligned +3.0, Stillness +1.5, KinNeed +1.25, FoodAdjacent −1.16, Alarm −1.0, Crowding −0.3` | the step roll (§6) |
| `Turn` | `TempAboveAmb −0.8`, so it lies within ±0.44, is exactly 0 at ambient (no bias), and stays near 0 wherever the temperature does | the forward cone (§6) |
| `Persist`, `Caution`, `Tumble` | **none**: `squash(0) = 0`, which `unit_scale` maps to the midpoint. **Persist 1.0, footing 0.6, tumble chance 0.5** | cone and tumble (§6) |
| `EmitA` | unit 4 only | trail A (§7) |
| `EmitB` | `CarryingFood +2.5`, so **0.714 while laden, 0 while empty** | trail B (§7), laid only on a trip load |
| `Feed` | `Bias +0.4, FoodAdjacent +0.8`: 0.29, or 0.55 with food in reach | §5 |
| `Drop` | `Bias −2.0, Energy +1.8, AtNest +1.0889, Carrying +0.2`: **0 away from the nest; at it, 0.52 when fed and 0 below about 40% of `start_energy`** | §5 |
| `DropSpoil` | `AtNest +0.9, Carrying +0.2, SurfaceCurvature +0.169` | §5 |
| `Dig` | `Bias −0.3, SurfaceCurvature −1.0, FoodAdjacent +0.8, MoistureGrad −0.55`: **0 on open ground away from the nest**, positive where the ground encloses the ant (a tunnel face, a shaft, a pit) or beside food; at the nest about 0.8 from units 5/6 | §5 |
| `Share` | `Energy +2.5, KinNeed +1.9, Bias −2.5` | §5 |
| `Attack` | `Alarm +2.0` | §5 |
| `Impulse`, `Provision`, `Fly` | none (0): no hops, no birth provisioning, no flight | — |

**`P(move)` in common states**, founder genome, no trail reading, not still,
nothing adjacent:

| State | Sum | `P(move)` per decision |
|---|---|---|
| Empty, fed | 2.0 − 1.75 = 0.25 | **0.20** |
| Empty, hungry (`Energy` 0.5) | 1.125 | 0.53 |
| Laden, facing home | 3.25 | 0.76 |
| Laden, 45° off home | 2.37 | 0.70 |
| Laden, perpendicular to home | 0.25 | 0.20 |
| Laden, facing away | −2.75 | **0, exactly** (the clamp) |
| Any, food in reach | 0.25 − 1.16 | 0 |

**Under the chooser, which is how the ant walks, two of these do not
apply:** `HomeAligned` reads 1 whenever the ant carries food off its anchor,
so every laden row is 0.76 whichever way it faces; and the trail throttle
(units 0–3) is fed `PheroAAlong` and `PheroBAlong` as 0, so it adds nothing
(§6d). For a species walking §6a–§6c, the throttle adds to these sums and can
outweigh any of them. Stillness adds up to +1.5 after 192 still ticks, which
is what eventually frees a stuck ant.

**At the nest the table's "empty, fed" row is lower.** There `Crowding` is
the nest's room occupancy (§3), typically about 0.5, which takes 0.15 off the
sum: a fed empty ant at home steps on about 1 decision in 10 (measured
0.10–0.13 on the colony bed), and on 0 beside food.

## 5. Acting: feed, drop, dig, share, attack

`act`, before movement, in this order. **Later steps are skipped by an early
return**, so the order is part of the behaviour. A return ends `act`, not
the tick: the ant still gets its move roll (§6) afterwards.

1. **Attack** (at `Attack` probability): bite the nearest foe. It displays
   instead of committing when `contest` odds say so.
2. **Share (trophallaxis)**, on by default: give a quarter (`SHARE_FRACTION`)
   of the energy difference to the neediest adjacent nestmate that has less.
3. **Feed = pick up.** With food in the crop, `choose_weighted` between
   `Feed` and `Drop` first decides whether to try feeding at all. Feeding
   requires room in the crop and the same material as what is already held.
   **`Feed` reads no hunger**, so a fed ant beside food at the nest takes it
   on about half its decisions. Under `PIXEL_PHYSICS_FORAGE_DRIVE=<need>,keep`
   (`,keep` is off, §6d) a forager the colony needs, at or above its `start_energy`,
   has its feed urge at the nest scaled by `1 - drive`, so it leaves the
   store for the hungry and unloads rather than re-taking; below
   `start_energy` it eats as before.
   A successful feed roll **removes one adjacent food cell from the world
   into the crop**, and `act` returns. **Two storeroom rules come first**
   (since 2026-09-29, §8; `PIXEL_PHYSICS_STOREROOM=off` removes both). A fed
   animal's won roll on a cell of the storeroom takes nothing and ends
   `act`: **the store is kept for the hungry** (`store_kept`, counted in
   `CreatureStats::store_kept`). And a nest worker at home, fed (at or above
   `start_energy`), with an empty crop and empty mandibles, that wins its
   roll beside loose food (not meat) lying more than a cell from the
   storeroom, while the room has space, **takes the cell whole into its
   mandibles instead of swallowing it** (`store_pickup_ok`), walks
   it down the shaft and along the passage to the room's floor
   (`store_target`) and puts it down there on a `DropSpoil` roll
   (`store_drop`); it lets the load go where it stands after 48 still
   decisions or when patience runs out, and is then pulled back up to the
   mouth (`store_return_target`). **The crop is both the cargo and the
   stomach** (§9). `crop_capacity: 5760` worth units is six fruit cells
   (960 each), which is 1,440 J to an ant at the neutral gut.
4. **Drop food**, whenever the crop holds anything, then **return**. The roll
   against `Drop` is taken **first**. Only on a win does it look for a place
   (`food_drop_site`), and put one food cell there:
   - the first empty cell among the 8 neighbours, in fixed order;
   - if there is none, **the nearest empty cell reachable by handing the food
     through bodies**, the ant's own and any other creature's, never through
     ground, nest material or food already put down. So a blocked ant passes
     its food back along its body or through the crowd;
   - if even that finds nothing, nothing happens: the roll is spent.

   `PIXEL_PHYSICS_DROP_REACH=adjacent` turns the second rule off.

   The drop is
   not gated on being at the nest, but `Drop` is 0 elsewhere, and **at the
   nest it rises with `Energy`**: a fed ant puts food down at about 0.25 a
   tick, and one below about 40% of `start_energy` never does, so it keeps
   what it holds and eats it (§4). `deliveries`
   counts any drop made while `AtNest`; `drop_census` counts every roll by
   outcome (`DropWhy`: lost, placed, delivered, no room), and
   `drops_passed_on` the drops that went past the neighbours. **A delivery
   is any drop at home, whatever the food's history**, so a crumb picked up
   at the nest and put back counts again. `pickups_at_nest` counts step 3's
   pickups made on the same test (read before the food leaves). **The
   difference is not the food that came home** (measured 2026-09-27,
   `ant-scenes-2026-09-23.md` §22j): both counters judge the acting ant's
   head, not the food cell, so a crumb at the nest's edge is picked up from
   outside, uncounted, and delivered again. On the colony bed it overcounts
   3.4-4.6×; food taken from the pile is the honest flow.
5. **Drop spoil**, if holding a dig pellet, then **return**, placed or not,
   so an ant holding a pellet never digs. The roll is against `DropSpoil`,
   its own output (§4), not the food drop's. On a win the pellet goes on the
   first of the 8 neighbours, in fixed order, that is empty, sits on at least
   two filled cells of the three below it, and has `SPOIL_HEADROOM` (3) empty
   cells above (`spoil_site_open`). **An animal counts as filled**, so a
   pellet can be set on its carrier's own back, on a nestmate, or over a hole
   whose two diagonals are filled. The footing rule (`update_powder`) turns a
   pellet to loose soil unless the cell straight beneath it is ground, so
   such a pellet falls within a few frames. Under
   `PIXEL_PHYSICS_SPOIL_FOOTING=ground` the drop site asks for ground instead:
   the cell straight beneath and two of the three must be a non-animal
   `Powder` or `Solid` (`is_footing`). **If none qualifies it is posted straight up the ant's own
   column** to the first cell that does, and the ant does not move
   (`lift_reach`). Under the default `SPOIL_LIFT=climb` that scan passes
   ground the ant could cut and empty rows with a wall beside them, and stops
   at open sky with nothing to hold, at material it cannot cut, or after 160
   rows. `spoil_lifted` counts these.
6. **Dig**, only if both crop and spoil are empty. **So a laden ant never
   digs.** The roll is against `Dig`, and the target is **the cell straight
   ahead of the head, along its current heading**: nothing chooses a face or
   a place near other digging. **An enclosed digger first turns down**: on a
   won roll, an ant whose curvature is at or below -0.3 turns one octant
   toward straight down before it cuts (`dig_down_bias`, `dig_down_of`, on
   since 2026-09-28 in this enclosed form; `PIXEL_PHYSICS_DIG_DOWN=off`
   removes it, `=<w>` turns anywhere), so an ant on the open surface never
   starts a hole downward. An ant facing straight up has no shorter way
   round and turns to either side by a coin of its own (`turn_toward`,
   `half_turn_left`, keyed on the ant and the frame). **It does not turn where there is no way down**
   (`way_down`): when all three cells under it are ground it cannot cut
   (`jaw_can_cut`, the test below) -- stone, bedrock, nest paint -- the turn
   is refused (`digs_down_refused`) and the roll digs straight ahead. At the
   shipped chance of 1.0 the turn takes no draw from the move stream. The move after it is still
   decided from the heading the ant had before `act`, so a step or a tumble
   replaces the turn and a lost move roll leaves it standing. **The heap
   cue** (on since
   2026-09-28; `PIXEL_PHYSICS_SPOIL_CUE=off` removes it) lets a cut that
   would open the ground to the sky go ahead only with probability
   `floor + (1 - floor) s²/(s² + K²)`, `s` the pellets within 2 cells of the
   cell actually cut, after any `DIG_DOWN` turn; shipped at `K` 5 and floor
   0, so bare ground is opened only beside a heap, and a colony's first
   opening is its founding shaft (§8). Two cuts are left alone: a pellet
   target, and a cut into a cell with ground above it by an ant that is
   enclosed (curvature at or below -0.3) or that has ground over its own
   head and cuts below itself (`spoil_cue_factor`, `open_to_the_sky`). So a
   tunnel's digger, and one on the floor of a wide room, dig freely until a
   cut would open the sky, while an ant in open ground under a roof meets
   the cue when it cuts level or up. It must not be empty, a creature or
   plant cell, or a live seed, and needs `penetration_resistance ≤
   dig_force` (1.0) (`jaw_can_cut`). Soil, lining and spoil pass, and so do powder foods and
   litter such as crumbs; sand and the nest's own material do not. The cell
   becomes the held pellet in its `spoils_into` form (soil, lining and spoil
   all become `spoil`), else its `packs_into` form, else as itself: a dug
   crumb stays food, carried in the spoil slot. Then it **lines the burrow**
   (`line_burrow`): every one of the 8 neighbours with a `packs_into` form
   becomes `packedsoil`, **except spoil**: a neighbour whose material
   `needs_footing` is left as it is, so a heap undermined by the cut slumps
   into loose soil rather than hanging as wall (`packedsoil` needs no
   footing). `PIXEL_PHYSICS_SPOIL_PACKS=on` packs spoil as well.

## 6. Moving

### 6a. Step or not

```
p_move = clamp(Move, 0, 1)
if draw < p_move:      hop (Impulse; 0 for the ant), else step_chain
else if draw < 0.5:    tumble                      (0.5 = unwired Tumble)
```

**Stepping and tumbling are the two arms of one `if`**, so an ant turns only
on a tick when it did not step. A tick where the roll fails and the tumble
roll fails does nothing.

### 6b. Where a step goes (`step_chain`)

1. The support check and possible fall (§2).
2. **Three candidates:** ahead-left, ahead, ahead-right (heading + 1, heading,
   heading + 7). **An ant can turn at most 45° per step.**
3. Each candidate's landing is the whole body after the step
   (`body_after_step`). It scores 0 if not enterable, otherwise
   `[max(Turn, 0), Persist, max(−Turn, 0)]` for its position, **plus 0.6 if
   the head would have a foothold**. **No pheromone or home term appears
   anywhere in this score.**
4. Any candidate scoring ≤ 0.3 (footing × 0.5) is zeroed. On flat ground that
   removes both diagonals, so the ant can only go straight: open bug §R4,
   *"it can be scattered, not steered"*.
5. The pick is `choose_weighted(scores, k = 0.1)`: probability ∝ (0.1 + s)².
   With three footed candidates at `Turn` 0, that is 12.7% / 74.6% / 12.7%.
   Its randomness is intended; never replace it with an argmax.
   `cone_picks` counts which candidate each step took. `turn_requests`
   counts steps chosen with a nonzero `Turn`, and `turn_discarded` those
   whose requested side had been zeroed, so the turn could not happen.
6. **The blocked path**, when nothing survives: try a trunk crossing (a woody
   cell ahead puts the ant in a `Crossing` for thickness × interval frames).
   Then a **reversal**, if the ant is **boxed**: refused in all eight
   headings, laden or not (`is_boxed`). The default `ReverseRule::Flip`
   reverses the body in place and turns the heading 180°. A laden ant boxed
   only by other creatures waits instead, for a bounded number of ticks
   (`boxed_by_traffic`, `traffic_deferred`). Then a swap with a
   nestmate (only for `passes_through_kin` species; the ant is not one).
   Otherwise **tumble**. A blocked ant always re-rolls, whatever `Tumble`
   says.
7. **After a successful step:** heading = the chosen candidate. If the new
   head cell is next to nest material, `forage_anchor` = that cell and
   `since_nest = 0` (§8).

### 6c. Tumble (`tumble`)

Re-roll the heading **uniformly among the usable headings** (§2's predicate,
over all 8). **The one exception is the homeward re-roll**
(`home_weighted_pick_why`), which replaces the uniform pick with **the usable
heading whose cosine to `forage_anchor` is highest**: a hard argmax, ties to
the last. It fires only if all of these hold:

- `home_bias > 0`: `ant.ron` authors **1.0**;
- the crop holds food, with fill > 0;
- the ant is at least 1 cell from `forage_anchor`;
- a draw < `home_bias × fill`, taken last, so a gate that fails costs no
  draw.

It aims at `forage_anchor` **even when `PIXEL_PHYSICS_HOME_TARGET=nest`**
moves `HomeAligned`'s target, so under that switch the throttle and the
re-roll aim at different places. `creature_stats.homeward_why` counts every
call by the gate that decided it (its two firing slots sum to
`tumbles_homeward`), and `homeward_aim` counts every firing by where the
chosen heading pointed: toward the anchor, across, or away (cosine above
0.01, between, below −0.01). "Away" is possible because the pick is the best
*usable* heading.

**Under this walk, the re-roll is the only thing that aims a walking
animal.** `HomeAligned` only grants or withholds permission to step. Under
the chooser, which the ant walks, the home and away terms of §6d aim it.

### 6d. The chooser (`chooser_step`): how the ant walks

**The ant walks this, not §6a–§6c.** `chooser_for` gives every species that
names a nest (the ant and its variants, the beetle, the hopper) the walk set
by `PIXEL_PHYSICS_CHOOSER` or `World::chooser`, **`trailaway` unless set**. A
species with no nest (the flitter, the worm) walks §6a–§6c whatever the
switch says, and so does the ant under `PIXEL_PHYSICS_CHOOSER=off`. The walk
is built in layers, and the ant walks all of them: items 1–5 below (`on`),
plus the trail terms (`trail`), plus the away term (`trailaway`).

1. **Every decision, before any roll:** the support check and possible fall
   (§2). A fall is not a move: it lays no trail and costs no step.
2. `p_move` as in §6a, except that `HomeAligned` reads **1 whenever the
   ant carries food and is off its anchor**, whichever way it faces: the
   shipped facing-home pace (0.76). The heading is picked after the roll, so
   the roll does not depend on the facing. (A facing-dependent pace let a fed
   ant beside food, facing away, reach `p_move` 0, and with no re-aim on a
   lost roll that is permanent.)
3. **A lost roll is a pause.** No tumble, no re-roll.
4. **A won roll picks one heading from all the usable ones** (§2's
   predicate) with `choose_weighted`, at `k = 0.1 × unit_scale(Tumble, 2)`
   (0.1 unwired). Each scores:
   - `Persist × (1 + cos turn) / 2`: going on 1, turning round 0;
   - `Turn`'s left/right bias, as in §6b;
   - while carrying food: `home_bias × patience × cos(heading, home)`, aimed
     at `home_target` (so it follows `PIXEL_PHYSICS_HOME_TARGET`), not
     scaled by fill.

   If the facing itself is not usable and a trunk crossing is available, the
   crossing is one more option at the straight-ahead score. With no usable
   heading and no crossing, the decision goes to `step_chain` unchanged
   (reversal, kin swap, blocked tumble).
5. **Patience** (`home_patience`, 1 at rest): times 0.9 on every step that
   gets no nearer home than `home_best` (by 0.25 cells), plus 0.25 on every
   step that does. **Back to 1 when a way round fails**: once the head has
   been 6 or more cells (`EXCURSION_CELLS`) from where it set its best, and
   comes back to within a cell of that spot. It resets when the ant has
   nothing to take home or the target moves, **and at every pickup**
   (`carry_patience_of`, `PIXEL_PHYSICS_CARRY_PATIENCE`, on since
   2026-09-27), so a carry is measured from the last cell loaded. Before
   that it restarted only at the first: loading more cells and climbing the
   pile's face ran it down, and a loaded forager with no pull home walked
   off the pile's far side (bug Z35). `PIXEL_PHYSICS_CARRY_PATIENCE=off` is
   the old rule. `PIXEL_PHYSICS_CHOOSER=nopatience` holds it at 1.

**Stage 2 (`PIXEL_PHYSICS_CHOOSER=trail`) adds two things:**
- **The trail where a step would go.** For each heading, `trail_presence`
  reads the scent in the cell the head would enter and the one beyond it,
  takes the larger, and saturates it as `x / (1 + x)` over `TRAIL_HALF` (a
  tenth of one full deposit). Trail B for an ant carrying no food, trail A
  for one that is. The heading's turn score is multiplied by
  `1 + TRAIL_GAIN × presence` (`TRAIL_GAIN` 3), so a heading onto a full
  route scores up to 4 times its turn alone, and turning round still scores
  0. Presence has no direction: both ways along a route score the same.
- **The throttle retires.** `brain_inputs` hands the brain `PheroAAlong`
  and `PheroBAlong` as 0, for every creature walking the chooser. The ant's mirrored
  hidden-unit pairs on those inputs cancel exactly at 0, so the trail no
  longer changes `p_move`. The sensed values are still what the trace
  records.

**`trailaway`, the ant's walk, adds a direction along a route**, for an
empty ant only (one not carrying food or spoil): each heading also scores
`AWAY_GAIN × presence × cos(heading, away from home)`, with home from
`home_target` as the laden ant uses it (`AWAY_GAIN` 1). Scaled by presence,
it acts only on a route; off a trail an empty ant scores exactly as under
`trail`. On a route an ant facing home can turn round (turning round now
scores `AWAY_GAIN × presence`), and one facing away almost never does.

**Scouting (`PIXEL_PHYSICS_SCOUT=<gain>`, 2 unless set, 0 turns it off) gives
the empty ant a direction off a route too, scaled by hunger.** Under `trailaway`, an empty
ant carrying no spoil and not fed (`hunger = 1 − energy / start_energy`, so 0
when fed and the term is never added) scores each heading with
`gain × hunger × (1 − presence) × level cos(heading, away from home)`. Home is
`home_target`, and *level* means the cosine of the heading's sideways part
only, so a step straight up or down scores 0. The pull carries a patience,
the mirror of the laden ant's (`scout_best`, `scout_patience`). A step that
gets the scout no further out, level distance from home, than it has been on
this excursion multiplies patience by `PATIENCE_DECAY`, and the outbound
pull is scaled by patience. Below `SCOUT_GIVE_UP` (0.1) the scout has given up
(`scout_home`) and the same term pulls it home, by the full home cosine, until
a nest contact re-anchors it and starts the next excursion. Only the ant's
state is written, and only while the term is on.

**`PIXEL_PHYSICS_HUNGRY_HOME` (off) gives a hungry empty ant the laden ant's home
pull.** It fires when the ant's energy is under what the walk home costs:
`0.1 × start_energy + distance × (move_cost_per_cell + idle_cost_per_cell) ×
body cells × 2`. The latch is `hungry_home` (`update_hungry_home`). It pulls
the ant to its nest's larder (`hungry_target`, §8) through `home_pull`,
patience and all, and it suppresses scouting, since `away_from` needs no home
pull. Two forms:
- `on` / `refed` holds the latch until the ant has eaten back to half of
  `start_energy`.
- `tether` sets it only off a route (trail B under the head under presence
  0.5), and clears it within 2 cells of the target.

**`PIXEL_PHYSICS_FORAGE_DRIVE` (`returns` since 2026-09-29, `always` from
2026-09-27; `off` is the ant before it) sends a fed forager out while food is
coming home.** It reaches an animal that has foraged (`OrganismState::foraged`,
set by any pickup away from its nest), is empty (sensed empty and still empty
after `act`) **or carries a packed lunch**, carries no spoil and walks
`trailaway`. A packed lunch (`carries_lunch`, `PIXEL_PHYSICS_PACKED_LUNCH`,
on since 2026-09-28) is a crop filled only at home since it was last empty
(`OrganismState::lunch`, set by a pickup at home into an empty crop, cleared
by any pickup away from home; "at home" is the head beside nest material, or,
under `PIXEL_PHYSICS_STORE_LUNCH` (off; `on` is a switch), any pickup before the ant has been
`FORAGE_TRIP_MIN` cells from its last nest contact, `OrganismState::
forage_max`): its carrier also has no pull home
(`home_pull`) and scouts and reads the trail as an empty ant (`chooser_step`'s
`laden`), and eats the lunch on the road. Its `Drop` still reads `AtNest`, so
it does not put the lunch down on the way; beside food its crop cannot
swallow (a crop holds one material), it finishes the lunch that tick
(`eat_lunch_now`: digestion completes the cell in progress) so it can load. That animal feels
`drive` (`forage_drive_level`) in two places and nowhere else:
- scouting's pull uses `gain × max(hunger, drive)` in place of
  `gain × hunger`, so a fed forager runs out and back like a hungry scout;
- unless `,nopace`, the `Move` row reads `Energy` as `1 - drive` where that is
  lower than its own (`forage_pace`: the row's sum is recovered by inverting
  `squash`, and the animal's own `Energy → Move` weight times the change is
  added). `Drop`, `Share` and `Feed` still read its true energy.

The drive is the nest's need (`World::nest_need`, §8), found from
`home_target` as `hungry_target` finds its nest, or 1:
- `hunger`: the mean over the nest's animals of `1 - energy / start_energy`,
  floored at 0 each;
- `larder`: `1 - store / (animals × start_energy × LARDER_GRANTS)`, clamped
  to 0..1, where the store is the loose food near the nest (§8);
- `always`: 1. Built as the control for the two needs; it measured best of
  the three on the bed and shipped from 2026-09-27 to 2026-09-29
  (`ForageDrive::ALWAYS`);
- `returns` (the default, `ForageDrive::SHIPPED`): 1 while its nest last saw
  a forager come home with food from a trip under `RETURN_WINDOW` (1,400)
  frames ago, then `e^-(age - W)/W` (`returns_drive`; `W` is
  `return_window`, `PIXEL_PHYSICS_RETURN_WINDOW`, unset 1,400). A return is
  booked once per trip, at the first put-down at home of a crop marked
  `OrganismState::trip_load`, into `World::nest_last_return`;
  `World::step_nest_need` starts a new nest's clock when it first sees it. A
  pickup marks a trip when it is away from home, the ant has been
  `FORAGE_TRIP_MIN` (8) cells from its last nest contact, and -- under
  `PIXEL_PHYSICS_TRIP_REACH`, on -- the food was living tissue or loose food
  taken more than `TRIP_REACH_SHIPPED` (16, scaled) Chebyshev from the
  centre of every nest's door, measured from `NestSite::surface` at the food
  cell (`trip_source`, `door_distance`). So food moved about beside a door
  books nothing. A mark belongs to its crop: under the reach, a pickup into
  an empty crop clears `trip_load` and `trip_src` first, so a crop digested
  to nothing, or put down away from home, leaves no mark for the next one.
  `OrganismState::trip_src` records where a crop's marking pickups were
  taken, for `CreatureStats::trip_returns_near` and
  `trip_returns_tissue_near` (an upper bound on the tissue exemption: any
  crop holding tissue taken within the reach).
**A nest worker (§8) is never driven**: `forage_drive_level` reads 0 for a
nest-bound animal, and fed it takes no away term and is pulled home when it
strays (`home_pull`); hungry, it scouts for food as any ant does.
`,keep` adds the store rule in §5. `,fed` drives only an animal at or above
its `start_energy`; below it the level reads 0 and the animal goes out on its
own hunger. Without `,fed` the drive reaches hungry foragers too, and early in
a run they are nearly all it reaches: on the bed before frame 6,000, 29,459 of
30,817 driven decisions were ants under the grant. On the nest itself the
anchor follows the ant, so the pull has no direction until the ant steps off
an end.

The decision trace records the patience each choice scored with, the home
cosine of the heading picked (for an empty ant too, under `trailaway`), and
under stage 2 its trail presence (`patience`, `chosen_cos`, `chosen_route`).

## 7. The trail planes

`pheromone.rs`: two world-sized `u16` planes, A and B, plus an alarm plane.
The channels carry no meaning in the engine; the meaning is in the wiring.

- **Laying** (end of `creature_tick`, **only if the ant moved**): add
  `emit × DEPOSIT` to the head cell. `DEPOSIT` is 40 × 256.
  `PIXEL_PHYSICS_DEPOSIT_AT=vacated` uses the cell just left instead.
  Adding saturates. Laying costs energy.
  - **Trail A:** every ant, laden or empty, at the unit-4 odometer's
    strength: strong just after leaving the nest, fading with time away.
    It works as nest scent.
  - **Trail B:** only a forager bringing food back from a trip. The
    genome's `EmitB` is 0.714 for any ant with food in its crop
    (`CarryingFood`, 1 whenever `crop_fill > 0`), and **the lay rule**
    (`PIXEL_PHYSICS_FOOD_TRAIL`, `lay` unless set) multiplies it by 1 while
    the crop holds food marked as a trip load (`trip_load`, §6d) and by 0
    otherwise (`food_trail_lay`). So packed-lunch carriers walking out,
    nest workers and ants eating store food at home lay none. It works as
    the food trail. `t=<ticks>` adds an odometer, `T / (T + since_trip)`.
    It multiplies, so a genome with `EmitB` silenced still lays nothing.
    `PIXEL_PHYSICS_FOOD_TRAIL=off` is the ant before 2026-09-30: every ant
    with food in its crop lays at 0.714.
- **Spreading and fading**, every `PHEROMONE_INTERVAL = 12` frames, every
  awake tile: each cell becomes `here + 0.25 × (mean of its 3×3 − here)`,
  then fades by `× (1 − rho)` with a forced minimum drop of 1 raw unit.
  - **A:** `rho = TRAIL_A_RHO = 0.0`, so it fades only by that minimum.
  - **B:** `rho = DECAY_RHO = 0.03`.
  - **Terrain is ignored.** Scent spreads into rock and sky exactly as into
    open air, and leaks between parallel tunnels.
- **Alarm:** spreads by distance falloff (`Spread::ActiveSpace`),
  `ALARM_RHO = 0.35`. It is laid by being bitten, and by displays.

**Who reads what.** Laden ants read A (units 0 and 1); empty ants read B
(units 2 and 3). **Only the forward difference, and only into `Move`.**
Laden ants lay B, and everyone lays A. No other wire or code path acts on
either plane: the other trail inputs are computed and wired to nothing (§3).

## 8. Home: the nest, the anchor, and "at nest"

- **Nest material** is the species' `nest: "nest"` material. Being next to it
  drives `AtNest`, which is the only way the ant knows it is home. Founding
  paints it on the surface as **a door five columns wide, unbroken, since
  2026-09-29** (`NEST_DOOR_SHIPPED`, a half-width of 2; before it, and under
  `PIXEL_PHYSICS_NEST_DOOR=off`, a strip of up to 53 columns, ±26, a masked
  comb with an unbroken core), and **since 2026-09-28 also cuts a shaft**
  `NEST_SHAFT_ROWS` (6) rows deep and 2 wide under the founding point, with
  an entrance chamber at the bottom, lined, and records it in
  `NestSite::shaft` (`dig_founding_shaft`, `cut_founding_shaft`; one cut per
  site; `PIXEL_PHYSICS_NEST_SHAFT=off` paints only). It cuts only ground the
  founders could dig themselves (the founding species' own `dig_force`, the
  ant's 1.0, plus the nest paint over its mouth; a bare `paint_nest_patch`
  with no founders takes the strongest nesting jaw, `founding_dig_force`): a
  column opens only where the cell under the paint is not empty and not too
  hard, it stops at stone, gravel or sand, the chamber is cut only if a
  column reached it, and on rock the nest is painted and nothing is cut. A
  corpse is not ground (`is_diggable_ground`): the cut leaves it where it
  lies. `PIXEL_PHYSICS_NEST_DOOR=<d>` paints `2d + 1`
  columns (§12). **Since 2026-09-29 the cut also holds a storeroom**
  (`Storeroom::SHIPPED`'s `side`; `SideRoom`, `cut_founding_shaft_with`): a
  passage two rows tall leaves the shaft's wall halfway down, on the side
  away from the door (west when the mouth is under it), and runs past the
  chamber's end to a room as wide as the chamber (7 columns), its floor a
  row below the passage's, so food on it lies under the level ants walk at.
  It is cut only where the shaft was, on the same rules.
- **`forage_anchor`** is a world coordinate. It is set to the spawn cell at
  birth (for a founder, to the cell above the door's centre instead, read
  from the surface the founding started on, so a founding shaft through the
  door leaves it at the mouth; under `NEST_DOOR=off`, its spawn cell), and **reset to the new head cell on every step that lands next to
  nest material** (in `step_chain`, not after falls, swaps or reversals).
  So the anchor is the last cell the ant stood on **beside** the nest, and
  leaving a wide nest anchors it at the edge it left from, not at the
  nest's centre.
- It is the homing target for both `HomeAligned` and the homeward re-roll.
  The re-anchoring makes "home" mean *the last spot beside the nest I
  stood on*.
- `since_nest` counts ticks since the last such step. `forage_max` records
  excursion depth, read at a pickup by `PIXEL_PHYSICS_STORE_LUNCH` and the
  `returns` drive's trip mark (§6d).
- **Under `PIXEL_PHYSICS_NEST_HOME=shaft`** (§12), a cell within one cell of
  the founding cut (the shaft, its chamber, and the rim of its mouth, as
  recorded in `NestSite::shaft` when `PIXEL_PHYSICS_NEST_SHAFT` dug it) also
  counts as next to the nest. **Under `=mouth`**, only a cell within one cell
  of the shaft's top `NEST_MOUTH_ROWS` (2) rows does: the rim and the first
  body length down. `AtNest` and the re-anchoring both ask `adjacent_nest`,
  so both follow it. Unset, nothing here changes.
- **`NestSite::larder`** is where a nest keeps its food: a running mean of
  the cells food is delivered onto (each new delivery weighs `LARDER_EMA`,
  0.05). Only the hungry-home switch reads it (§6d). On the colony bed
  deliveries land on the door (under `NEST_DOOR=off`, at the end of the strip
  facing the food).
- **Nest workers** (since 2026-09-29, `Storeroom::SHIPPED`'s `caste=4` and
  `workerhome`): one ant in four, by id, founders and the born alike, is
  nest-bound for life (`OrganismState::nest_bound_until` at `u64::MAX`,
  `is_nest_bound`). The founding cut (shaft, chamber, side room and the rim
  of the mouth) is home to a nest worker and only to it
  (`nest_within_reach`), so it lives, eats, digs and breeds there while the
  foragers keep the door. The forage drive never reaches it (§6d), and it
  alone carries food into the storeroom (§5 step 3).
- **`World::nest_last_return`**, one per nest site, is the frame a forager
  last came home to it with food from a trip, read by the shipped `returns`
  drive (§6d); `World::step_nest_need` stamps a site it has not seen with the
  current frame.
- **`World::nest_need`**, one per nest site, is the forage drive's need
  (§6d), rebuilt every 256 frames by `World::step_nest_need` and empty unless
  the drive reads it (`hunger`, `larder`). It attributes every live creature of a nesting species
  to its nearest nest site by head, as the room census does (`nest_needs`).
  Under `larder` the store is every loose food cell (not living tissue)
  within 2 cells of nest material and 64 columns / 24 rows of the site,
  priced as the first attributed animal absorbs it (`diet_yield`, above
  `EAT_YIELD_THRESHOLD`).

## 9. The crop, digestion and energy

- **Crop** (`Crop`): one food material, `cells`, a per-cell `unit` worth, and
  `digesting` progress. `worth()` is net of what has already been digested.
  Fill = worth / capacity.
- **Digestion runs every tick the crop holds food, wherever the ant is.** At
  `digest_rate: 3.3` (trait-scaled), times diet quality, minus overhead, it
  pays into energy continuously. **A laden ant eats its cargo while carrying
  it**, so fill falls on the way home and with it the homeward re-roll's
  chance. `digest_hunger_weight: 0.0`: digestion does not wait for hunger.
- **At the nest store the crop is a spoon, not a suitcase.** An empty ant
  beside floor food swallows a cell at `feed_urge` 0.545 a tick whatever its
  hunger (`Feed` reads no hunger, §5), eats from it while it holds it, and
  puts the rest back where it stands: a feed pick that swallows nothing falls
  through to the drop roll, so for one cell at the nest a delivery per tick
  is 0 / 0.001 / 0.19 / 0.33 at E 0.25 / 0.5 / 0.75 / 1 beside food, and
  `drop_urge` itself (0 / 0.003 / 0.31 / 0.47) away from it. Emptied beside
  food it usually swallows again before it steps. A fed ant beside food does
  not step on its own: its `Move` sum is below 0 (`FoodAdjacent` -1.16).
  But a crop of store food is a packed lunch (§6d), so a foraged ant holding
  one feels the forage drive and goes back out, eating as it walks (the drive
  reads the crop after `act`, and without packed lunch a swallow put the ant
  out of its reach that tick). Nothing is lost -- the put-back returns what
  is left -- and this is where most of the colony's eating and budding
  happens (measured in `Reports/ant-scenes-2026-09-23.md` §22n, §22o).
- **A part-eaten piece of plant food goes down as `crumbs`.** The drop hands
  over the worth left (`unit - digesting`). A whole cell goes down as its
  own material. A part-eaten one (plant food, `food_class` below 0) goes
  down as `crumbs`, a powder holding exactly what is left in `aux`
  (`carries_worth`), at least 1. It is priced from that when picked up, so
  putting food down and picking it up again neither creates nor loses food.
  Crumbs do not rot, and **do not slide** (`rolls: false`,
  `Material::rolls`): they drop straight down through open air and
  otherwise stay where they are put, as the fruit they replace would. Two cases still go down at full price, and
  `drop_worth_restored` counts what they restore: flesh bitten off a living
  animal, and a fruit carrying a seed passenger, which goes down whole.
- **Energy costs:**
  - per tick: idle 0.05 × body cells, plus the synapse, sight, curvature,
    force and armour taxes;
  - per step: `move_cost_per_cell` 0.125 × (body + carried cells), **so a
    laden ant pays more per step; it does not step less often**. Carried
    cells are the crop's worth ÷ `body_energy` (480) × `food_weight` (0.5
    for the ant, 1.0 for species that do not author it), so food weighs by
    its joules at half the density of flesh. A fruit cell (960) weighs one
    body cell, and a full crop of fruit (5,760) six, three times the ant.
    A full crop's step costs 1.0 J against 0.25 J empty; an average laden
    step (crop about 40% full) about 0.55 J. Digestion pays at most
    3.3 × 0.25 = 0.825 J a tick, and **it runs while the ant carries**, so a
    forager eats from its load on the way home. Spoil weighs
    `spoil_weight_cells`. `PIXEL_PHYSICS_LOAD_BY=cells` weighs the crop by
    its cells instead, and `PIXEL_PHYSICS_LOAD_SCALE` multiplies every food
    load's weight (§12);
  - per dig: 6 × a step;
  - per laying tick: 0.0625 × a step × (emit A + emit B).
- **Death:** starved at energy ≤ 0; old age by half-life. **Budding:**
  `try_bud` above `reproduce_threshold: 1100`, wherever the ant stands.
  The bar counts the food in the eight cells around the head as well as the
  bank (`reachable_provision`), and a birth the bank cannot cover eats that
  food to make up the difference (`place_creature`'s `Origin::Bud` arm),
  leaving the parent at 1 J or more. A bare seed there counts only what a
  bite that spares it pays (`seed_provision_fraction`, a quarter), because
  a bite spares it 60% of the time (`plant::guaranteed_bite_fraction`,
  `PIXEL_PHYSICS_BIRTH_PRICE`); counted at face, the top-up could fall
  short and the parent died the next tick. `CreatureStats::births_overdrawn`
  counts births that leave a parent under the floor.
  With `PIXEL_PHYSICS_BUD_SITE=nest` (or `World::bud_at_nest`) a species
  that names a nest material buds only while at its nest (the `AtNest`
  read); `CreatureStats::buds_held_for_nest` counts the ticks it could have
  budded and did not. The child goes on the first of the eight neighbours
  of the parent's head where its whole body fits on empty cells; a parent
  with none is refused and tries again next tick
  (`CreatureStats::births_denied_no_space` counts the tries). With
  `PIXEL_PHYSICS_BUD_STACK=on` (or `World::bud_stack`) above a stack cap of
  1, a refused birth takes a second pass in which the child may stand on
  nestmates as a rider (`place_creature`'s `kin`;
  `CreatureStats::births_on_kin`); a `Segmented` child still needs empty
  ground.
  **Who in a colony may bud** is `PIXEL_PHYSICS_BREEDING` (§12), read once
  per process and `individual` unless set: no suppression, every animal buds
  on its own account. An animal is a **breeder** once it has budded
  (`OrganismState::children > 0`; nobody is one at founding). Under `queen`
  nobody buds while another living member of its colony is a breeder; under
  `graded` the bar is multiplied by up to 6 near one (`suppress_bar`). The
  scaling is applied to the composed bar, after the affordability check, and
  a colony with no other living breeder is not suppressed at all, so a
  colony whose breeder dies resumes.

## 10. Laden versus empty, every difference in one place

| | Laden (crop holds food) | Empty |
|---|---|---|
| Throttle on stepping | none under the chooser (§6d); under §6a–§6c, trail A's forward difference (units 0–1) | none under the chooser; under §6a–§6c, trail B's forward difference (units 2–3) |
| `HomeAligned` → `Move +3.0` | 1 whenever off the anchor, whichever way it faces | 0 |
| What picks the heading | every usable heading, scored by going on, trail A where it would step, and home at `patience` | every usable heading, scored by going on, trail B where it would step, and away from home on a route |
| Reversal when boxed in | yes, but a jam of creatures is waited out first | yes, at once |
| Lays trail B | 0.714 on every step, only on a trip load (`FOOD_TRAIL`, §7) | no (under `FOOD_TRAIL=off`, a packed-lunch carrier does: its crop holds food, so `CarryingFood` reads 1) |
| Lays trail A | at the odometer's (by then faded) level | at the odometer's level, strongest just out of the nest |
| Digs | never (`act` returns first) | when the dig roll wins |
| Drops | food at the nest, about 0.25 a tick when fed, never below ~40% energy | spoil, if holding it |
| Cost per step | higher (the load) | base |
| Digests | continuously, the cargo | nothing (the crop is empty) |
| **Anything that aims it** | the home term (the homeward re-roll under §6a–§6c) | the away term, on a trail only; nothing off one |

## 11. Other species sharing this machinery

`ancestor.ron` (the lab ancestor), `hopper`, `longant`, `ant_block`,
`ant_block_shaded`, `ant_long`, `ant_wide` and `chitin_pale` carry the same
hidden-unit layout, with two differences that matter:

- **Their units 2–3 are saturated** (`Bias +45`, `Carrying −75`), so the
  empty ant's trail-B throttle moves `Move` by about 0.003, effectively
  nothing.
- **They lay B on `Carrying`, which includes spoil**, so their diggers lay
  "food trail".

Everything in §2 and §6 is shared engine code. The chooser (§6d) reaches
every species that names a nest: these, the hopper and the beetle. The
flitter and the worm name none and walk §6a–§6c. The `Drop` hunger wires
(§4) are in `ant.ron` only.

## 12. Switches that change ant behaviour

Read once per process from the environment. The default is what ships.

| Variable | Default | Effect when set |
|---|---|---|
| `PIXEL_PHYSICS_HOME_TARGET` | anchor | `nest`: `HomeAligned` aims at the nearest nest site's surface; **the re-roll does not follow** |
| `PIXEL_PHYSICS_REVERSE` | flip | `off` / `back` |
| `PIXEL_PHYSICS_STACK_DEPTH` | 1 | >1: nestmates become enterable, up to the cap |
| `PIXEL_PHYSICS_DEPOSIT_AT` | head | `vacated` |
| `PIXEL_PHYSICS_TRAIL_READ` | here-vs-front | `fwd`: trail A read as far-minus-near at 6 and 12 cells |
| `PIXEL_PHYSICS_SENSOR_PROJECT` | on | `off`: no row projection; `none`: also no honesty gate |
| `PIXEL_PHYSICS_A_RHO` | 0.0 | trail A's fade rate |
| `PIXEL_PHYSICS_NEST_REACH` | r1 | `rN`: nest contact within radius N; `body`: any body cell |
| `PIXEL_PHYSICS_NEST_HOME` | material only | `shaft`: a head within one cell of the founding cut (dug by `PIXEL_PHYSICS_NEST_SHAFT=<rows>`, `_NEST_SHAFT_WIDTH=<cells>`, lined) also reads `AtNest`; `mouth`: only within one cell of its top two rows (§8) |
| `PIXEL_PHYSICS_LAB_ROOM` | on | `off`: at-nest `Crowding` falls back to local density |
| `PIXEL_PHYSICS_TROPHALLAXIS` | on | `off` |
| `PIXEL_PHYSICS_DROP_REACH` | through bodies | `adjacent`: a food drop looks only at the 8 neighbours |
| `PIXEL_PHYSICS_BUD_SITE` | anywhere | `nest`: a species with a nest material buds only at its nest (§9) |
| `PIXEL_PHYSICS_BUD_STACK` | off | `on`: above a stack cap of 1, a birth with no free cell beside the parent stands on nestmates (§9) |
| `PIXEL_PHYSICS_BREEDING` | `individual` | `queen`: while any other living animal of the same colony has budded (`children > 0`), nobody else in it buds; when that breeder dies, the next animal to reach its bar succeeds it. `graded`: the bar is multiplied by `1 + (GRADED_MAX_SUPPRESSION - 1)(1 - d/r)` for `d` the distance to the colony's nearest other breeder, so 6.0 beside one, falling linearly to 1 at `r` = `PIXEL_PHYSICS_BREEDING_RADIUS` (24) cells and beyond (`suppress_bar`, `graded_suppression_factor`). Anything else reads as `individual`. `PIXEL_PHYSICS_BREEDER_INDEX=scan` replaces the per-colony breeder index with a scan of every organism, as the control for the lookup (§9) |
| `PIXEL_PHYSICS_CHOOSER` | trailaway | For species with a nest. `off`: the walk of §6a–§6c; `on`: the chooser's first layer only (§6d items 1–5); `nopatience`: the same with patience held at 1; `trail`: the chooser reading the trail where it would step, with the throttle retired, and no away term |
| `SPOIL_IS_CARGO` | on | `0`: spoil no longer counts toward `Carrying` |
| `PIXEL_PHYSICS_DIG_SPOIL` | kept | `destroy`: dug cells vanish |
| `PIXEL_PHYSICS_BURROW_LINING` | on | `off`: no `packedsoil` lining |
| `PIXEL_PHYSICS_SPOIL_PACKS` | off | `on`: the lining packs spoil into wall too, so an undermined heap can hang (§5) |
| `PIXEL_PHYSICS_SPOIL_FOOTING` | filled | `ground`: a pellet is put down only where the cell beneath is ground, never on an animal or over a hole (§5) |
| `PIXEL_PHYSICS_SPOIL_CUE` | `on` (K 5, floor 0) | `off`: no heap cue, the ant before 2026-09-28; `K[,floor]` sets the dials. The cue: a dig that would open the ground to the sky, from the surface or from a tunnel breaking out, goes ahead only in proportion to the pellets beside its target (§5 step 6); `World::spoil_cue` for one world |
| `PIXEL_PHYSICS_NEST_SHAFT` | 6 | `off` (or `0`): founding paints only and digs nothing, the ant before 2026-09-28; `<rows>`: a deeper or shallower founding shaft (§8); `_NEST_SHAFT_WIDTH=<cells>` its width (2); `World::nest_shaft` for one world |
| `CROSS_TRUNK`, `TISSUE_PARTING` | on | `0` |
| `PIXEL_PHYSICS_DIGEST` | continuous | `lump`: pays out per whole cell |
| `PIXEL_PHYSICS_LOAD_BY` | joules | `cells`: a load weighs the cells in the crop, not its worth ÷ 480 (§9) |
| `PIXEL_PHYSICS_NEST_DOOR` | 2 | `off`: founding paints the strip of up to 53 columns and every founder's home is its spawn cell, the ant before 2026-09-29; `<d>`: a door of `2d + 1` columns, every founder's home the door (§8); a value it cannot read is read as unset; `World::nest_door` for one world |
| `PIXEL_PHYSICS_NEST_DOOR_FOUNDERS` | spread | `pile`: under the door, founders start heaped on it instead of spread along the ground (§8) |
| `PIXEL_PHYSICS_SCOUT` | 2 | `<gain>`: under `trailaway`, a hungry empty ant off a route runs out from home and back (§6d); `0` turns it off; `World::scout` for one world |
| `PIXEL_PHYSICS_HUNGRY_HOME` | off | `on`/`refed` or `tether`: an empty ant too hungry to be out is pulled home to its nest's larder (§6d, §8); `World::hungry_home` for one world |
| `PIXEL_PHYSICS_FORAGE_DRIVE` | `returns` | `off`, `hunger`, `larder`, `returns` or `always`, then optionally `,nopace`, `,keep` and `,fed` (only foragers at or above `start_energy`): a fed forager goes out when its nest needs food (§6d), and with `,keep` leaves the store at home (§5); `World::forage_drive` for one world |
| `PIXEL_PHYSICS_PACKED_LUNCH` | on | `off`: a crop filled only at home counts as a load, so the forage drive does not reach its carrier (§6d); `World::packed_lunch` for one world |
| `PIXEL_PHYSICS_TRIP_REACH` | on (16) | `off`: a pickup away from home marks a trip once the ant has been `FORAGE_TRIP_MIN` cells from its last nest contact, wherever the food lay; on, the food must also be living tissue or loose food more than the reach (authored cells, scaled; an integer sets it) from every nest's door (§6d); `World::trip_reach` for one world |
| `PIXEL_PHYSICS_RETURN_WINDOW` | 1400 | `<frames>`: the `returns` drive's window (§6d) |
| `PIXEL_PHYSICS_FOOD_TRAIL` | `lay` | the food trail's recipe (`FoodTrail`): `lay` lays trail B only on a trip load (§7), `off` is the ant before 2026-09-30 (every ant with food in its crop lays), `t=<ticks>` adds an odometer; `read`, `giveup`, `gain=`, `reach=2\|6` and `follow=all` parse and do nothing yet; `on` is all three parts; `World::food_trail` for one world |
| `World::mute_emit_b` | `false` | harness-only, set by `trailfollow`'s `mute` arm and by no game: every newborn's `EmitB` wiring is re-zeroed after its birth mutation (`silence_emit_b`), so a colony whose founders were silenced stays silent across births |
| `PIXEL_PHYSICS_STORE_LUNCH` | off | `on`: a cell taken before the ant has been `FORAGE_TRIP_MIN` (8) cells from its last nest contact counts as taken at home for the packed lunch, wherever it stood; off, a crop is a lunch only while every cell in it was taken with the head beside nest material (§6d); `World::store_lunch` for one world |
| `PIXEL_PHYSICS_HAUL_BITE` | on | `off`: an animal holding spoil cannot swallow or load food; `fed`: only one at or above `start_energy` (§9) |
| `PIXEL_PHYSICS_BIRTH_PRICE` | `guaranteed` | `face`: a birth counts a bare seed in reach at its full worth, though a bite that spares it pays a quarter, so the top-up can leave the parent overdrawn (§9); `World::birth_price` for one world |
| `PIXEL_PHYSICS_CARRY_PATIENCE` | `pickup` | `off`: a carry's home patience restarts only at the first pickup, not at every one (§6d step 5, bug Z35); `World::carry_patience` for one world |
| `PIXEL_PHYSICS_LOAD_SCALE` | 1.0 | `<f>`: every food load weighs `f` times as much again, on top of the species' `food_weight` (§9) |
| `PIXEL_PHYSICS_STOREROOM` | `on,caste=4,workerhome,side,keep` | `off`: no storeroom and no nest workers, the ant before 2026-09-29. Otherwise parts, comma-joined, naming the whole rule (so `on` alone is the carry and nothing else), and a value it cannot read is read as unset: `on`, a fed ant at home with an empty crop and empty mandibles, on a won `Feed` roll beside loose food more than a cell from its nest's founding chamber, takes the cell whole into its mandibles instead of swallowing it (`store_pickup_ok`); the load is pulled home to the chamber (`home_pull`, `home_target`, `HomeAligned` read as laden), goes down on a `DropSpoil` roll only in or beside the chamber (`store_drop`), is let go where it stands after 48 still decisions or when patience runs out, and its carrier is pulled back up to the mouth (`store_return_target`); a pick-up within a cell of the chamber reads as at home. `home`: the chamber is home to `adjacent_nest` (§8). `once`: one load until the next pick-up away from home. `post`: the load is handed down the open shaft from the mouth instead of walked down (`store_post_site`). `nestbound=<frames>[/<k>]` (bare: 8,000): an ant born in the colony, and with `/<k>` one founder in `k`, is nest-bound for its first `<frames>` (`OrganismState::nest_bound_until`, `is_nest_bound`): the forage drive reads 0 for it, fed it takes no away term and is pulled home when it strays (`home_pull`), and only it carries. `caste=<k>`: one ant in `k`, by id, founders and born, is nest-bound for life. `workerhome`: the founding cut is home to a nest-bound ant only (`nest_within_reach`). `side`: the storeroom is a room cut at founding off one side of the entrance shaft (`SideRoom`), on the side away from the door, joined to the shaft by a passage two rows tall halfway down, its floor a row lower; every rule above reads its rectangle in place of the chamber's (`ShaftFootprint::store_rect`), a carrier in the shaft or the chamber is pulled to the passage's far floor cell and then the room's floor (`store_target`), and `post` does not apply. `keep`: a fed animal's won `Feed` roll on a storeroom cell takes nothing and ends its turn, so only a hungry one eats the store (`store_kept`, counted in `CreatureStats::store_kept`). `World::storeroom` for one world |
| `PIXEL_PHYSICS_NEST_SHAFT_OFFSET` | 0 | `<cells>`: the founding shaft is cut that many columns from the founding point (negative is west), so a door (`PIXEL_PHYSICS_NEST_DOOR`) has the mouth beside it (§8) |
| `PIXEL_PHYSICS_DIG_DOWN` | `1.0,enclosed` | `off` (or `0`): no turn, the ant before 2026-09-28; `<w>`: the turn with chance `w` for any digger; `<w>,enclosed`: only an enclosed one (§5 step 6) |
| `PIXEL_PHYSICS_SPOIL_HAUL`, `_SPOIL_DROP_COVER`, `_TRAFFIC_DEFER`, `_COLONY_SPACING` | unset | haulage re-roll to the nest door, spoil held under cover, jam deferral length, founder spacing |
| `PIXEL_PHYSICS_SPOIL_OUT` | off | the excavation cycle walked (`SpoilOut`): parts, comma-joined, or `on` for all four. `haul`: a pellet carrier is pulled to the door over the mouth (`spoil_haul_target`); `pace`: at the laden pace, `HomeAligned` read against that target (`spoil_pace_target`); `keep`: inside the nest (`inside_nest`: under cover, or in the founding cut) the pellet is not put down while patience lasts, and is never lifted from there (`spoil_kept_inside`, `spoil_kept_no_lift`); `back`: a digger not hungry walks back to the cell it cut once its pellet is down (`OrganismState::dig_return`, `dig_return_target`) |
| `PIXEL_PHYSICS_SPOIL_LIFT` | `climb` | where a pellet with no cell beside its carrier goes: `climb` up the carrier's column as far as it could have walked (`lift_reach`); `out` through the passages to the nearest cell in the open that holds a pellet (`lift_out`, `spoil_lifted_out`); `none`, `dig`, `unbounded` the older reaches |
| `PIXEL_PHYSICS_SPOIL_RING` | `2,2`, **acting only under `PIXEL_PHYSICS_SPOIL_OUT`** (so the shipped lift is untouched; owner 2026-09-29) | `<shape>,<scale>` or `off`; `spoil_ring_of` gates it on the walked cycle, and a world's own `World::spoil_ring` overrides both: when a carrier comes out by the door with its pellet (on or above the door's row, nothing overhead: `carry_stage`) it draws a column on its own side, the door's half-width plus one plus a Gamma(shape, scale) draw from the nest's centre (`spoil_ring_column`, `OrganismState::spoil_ring`, its own stream), is pulled to the top of the ground in that column (`ring_target`, climbing any mound; `spoil_haul_target`), and its drop roll is held until its head is that far out (`spoil_ring_holds`; `spoil_ring_drawn`, `spoil_ring_held`). The column is kept under a mound's overhang and let go only back in a tunnel (more than two rows under the door's row with ground overhead, or in the founding cut: `spoil_ring_let_go`); before it has come out, under `keep`, the pellet is held wherever the carrier stands; `World::spoil_ring` for one world |
| `PIXEL_PHYSICS_DIG_WIDEN` | off | `on`: tunnels one body length (two cells) wide. On a won dig roll, a digger whose way ahead is open and whose head stands where its passage is one cell wide (ground above and below, or either side) cuts one of those walls instead of turning down and cutting ahead (`ahead_is_open`, `dig_widen_site`); a digger at a face cuts a shoulder beside the cell ahead on half its rolls (`dig_shoulder_site`), so a gallery advances two cells across. A passage two wide is left alone. Both cuts are ordinary cuts after that: the heap cue and the jaw judge them (`digs_widened`; `World::dig_widen` for one world) |

## 13. Where the implementation lives

- `creature.rs`: `creature_tick`, `sense`, `act`, `step_chain`, `tumble`,
  `usable_headings`, `home_weighted_pick_why`, `trail_sample_point`,
  `fall_if_unsupported`, `commit_step`, `chooser_step`, `home_pull`,
  `trail_presence`, `brain_inputs`, `adjacent_nest`, `line_burrow`, `food_drop_site`, `choose_weighted`, and the decision trace's
  types (`DecisionRow`, `DecisionOutcome`, `HomewardWhy`, `DropWhy`).
- `brain.rs`: `eval_brain`, the `BrainInput` / `BrainOutput` enums.
- `pheromone.rs`: the planes and the constants in §7.
- `update.rs`: `update_powder`, where crumbs' `rolls: false` stops the slide.
- `organism.rs`: `OrganismState` (`forage_anchor`, `crop`, `still_ticks`,
  `brain_state`, and the chooser's `home_best`, `home_best_for`,
  `home_patience`) and `Crop`.
- `assets/species/ant.ron`: every weight and constant quoted here.

## 14. Source comments that currently contradict the code

Delete each line when the comment is fixed. None are known.

## 15. The decision trace, built into the ant

Off unless a harness sets `World::decision_log = Some(Vec::new())`. While it
is on, every walking decision, the move stage of `creature_tick`, pushes one
`DecisionRow`:
- the head position and heading before and after;
- the usable-heading mask, which gives the setting class (0–1 pocket,
  2 corridor, 3–5 junction, 6–8 open);
- the leg as the brain saw it, the crop fill and the anchor;
- the wired inputs, the raw `Move`, `p_move` and `Turn`;
- both rolls;
- the branch taken (`DecisionOutcome`);
- the homeward re-roll's gate (`HomewardWhy`) and, if it fired, the true
  cosine of the chosen heading;
- the drop in `act` that same tick (`DropWhy`), its roll and probability,
  and the head's eight neighbours at the roll: how many were empty
  (`free8`), their materials, and which were the ant's own body or another
  organism; and how far the food went (`drop_reach`: 1 for a neighbour,
  more when handed on through bodies);
- the cone's three scores after the zeroing and the candidate taken;
- under the chooser (§6d), the patience it scored with and the home cosine
  of the heading it picked, and under stage 2 that heading's trail presence
  (`chosen_route`).
- the animal's energy in joules (`energy_j`: `energy` is the clamped input),
  the forage drive it felt (`drive`, NaN when off or carrying), and scouting
  as the chooser scored it (`scout_w`, `scout_patience`, `scout_home`).
- the trail it laid and read: the raw amounts actually deposited on each
  channel (`emit_a_laid`, `emit_b_laid`), the brain's `EmitB` before the cast
  (`emit_b_brain`), the cell laid on (`deposit_at`), and the cargo's age
  (`since_trip`, `OrganismState::since_trip`: ticks since the pickup that set
  `trip_load`, counted beside `since_nest`); and under the chooser, trail B
  one and two cells along each of the eight headings (`b_near`, `b_far`, the
  cells `trail_presence` reads) and at the six-cell sensor point (`b_six`),
  the option mask, whether a crossing forced the pick, the blend `k`, every
  option's score, the heading chosen and whether the trail was read.

`CreatureStats::decision_census` counts the same decisions by leg × setting
× outcome, and only while the trace is on, because the setting needs all
eight headings tested. The drop, cone and homeward counters (§5, §6b, §6c)
are always on.

- **It takes no RNG draw and changes no branch.** `act`, `step_chain` and
  `tumble` write a scratch value (`World::decision_scratch`) only while it is
  on. The scratch is reset before `act`, so a row's drop and move are the
  same tick.
- **Guards:**
  - `the_decision_trace_changes_nothing_it_watches` compares a whole bed
    with it on and off;
  - `the_setting_class_reads_the_ground_the_ant_stands_on` checks the
    classifier on three known terrains;
  - `every_traced_decision_agrees_with_the_counters_and_the_positions`
    checks rows against the census, against every counter above and the
    per-verb counters (`moves`, `drops`, `deliveries`, `tumbles_homeward`),
    and against where the head went;
  - four scenes with known answers: `the_homeward_re_roll_aims_along_a_known_floor_at_the_rate_its_fill_sets`,
    `a_drop_with_nowhere_to_go_is_counted_and_one_with_room_is_delivered`,
    `a_blocked_drop_passes_the_food_through_bodies_to_the_nearest_empty_cell`,
    and `the_cone_discards_a_turn_with_nowhere_to_go_and_follows_one_with_somewhere`,
    which feeds `Turn` directly because the shipped ant's is near 0 at
    ordinary temperatures;
  - `a_replayed_plane_equals_the_live_one`: a copy of the planes fed only
    the traced deposits equals the world's, cell for cell, so the trace
    carries every trail write;
  - three chooser scenes, each against the shipped walk as its control:
    `the_chooser_walks_a_laden_ant_out_of_a_dead_end_and_patience_is_what_lets_it`,
    `under_the_chooser_an_unsupported_ant_falls_whatever_the_step_roll` and
    `an_empty_ant_keeps_going_under_the_chooser_and_turns_round_under_the_shipped_walk`.
- **In the harness:** `trailfollow decisioncsv` writes the rows and repeats
  those reconciliations at the end of every run; `decisionnorows` keeps only
  the census; `dwide` appends the trail columns. `shadow` replays the logged
  deposits into a copy of the planes and asserts it equals the world every
  100 frames, and `cf=<rules>` lays copies under other rules along the same
  paths. `scripts/decisioncensus.py` reads the rows, including the drop and
  cone columns; `scripts/trailclimb.py` checks the logged scores reproduce
  the draws and reads the climb and the door departures.

