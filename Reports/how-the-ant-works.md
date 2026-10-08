# How the ant works

**A living reference: what the shipped ant (`assets/species/ant.ron`) does on
every tick, how each mechanism is implemented, and what it reads.** It is
written from the source and describes the code as it is now, not as it was or
will be.

- **Verified against:** `main` at `bb65d507`, 2026-09-22; §5 step 6's forager-dig sentence, §9's walk-in and nurse-workers sentences and §12's `FORAGER_NODIG` and `LAY_IN` rows 2026-10-08 against `forager_nodig`, `act`'s dig gate, `lay_in_target`, `lay_in_pull`, `deeper_along_way`, `home_pull`, `home_pull_why` and `brood::nurse_mode`; §6d item 4's fill paragraph, step 5's search sentence, stage 2's turn sentence and §12's `CARRY_HOME` row 2026-10-06 against `CarryHome`, `carry_home_of`, `fills_before_walking`, `carry_route_floor`, `chooser_step` and the tick's fall-then-roll order; §6d's `NEST_REST` paragraph and §12's `NEST_REST` row 2026-10-06 against `nest_rest_of`, `parse_nest_rest` and `NestRest::SHIPPED` (off again); §15's scout-want and door-read bullet 2026-10-05 against `chooser_step`, `outward_want` and `door_read`; §5 step 5's soil-way, lean-keep and face-trip sentences, §6d's way-gaps paragraph and §12's `WAY_GAPS`, `FACE_TRIP` and `SOIL_WAY` defaults 2026-10-05 against `way_gaps_of`, `soil_way_of`, `face_trip_of`, `gaps_hold` and `act`'s lean drop (all three on); §6d's nurse-stay paragraph, §9's nurse-stay sentence and §12's `NURSE_STAY` row 2026-10-05 against `NurseStay`, `relay_crop`, `is_crop_nurse`, `nurse_in_target`, `nurse_unpulled`, `nurse_stays`, `home_pull`, `chooser_step` and `act`'s drop; §15's pull bullet and its guard 2026-10-05 against `home_pull_why`, `home_pull`, `hungry_out_pull`, `rest_pull` and `chooser_step`; §15's nurse-term, walk-back and larva-meal paragraphs 2026-10-05 against `chooser_step`, `dig_trip_over`, `dig_trip_end_why`, `note_feed`, `brood::crop_feed`, `brood::nurse`, `brood_tick` and `act`'s share; §5 step 6's doorway sentence and §12's `DOOR_LOOSE` row 2026-10-04 against `door_loose_of`, `in_a_passage` and `pack_neighbours_with`; §6d's way-out paragraph and §12's `HUNGRY_OUT` row 2026-10-04 against `hungry_out_of`, `hungry_out_pull`, `step_nest_rest` and `chooser_step`; §12's `NEEDS_FIRST` row's `backfill` sentence 2026-10-07 against `pack_behind` and `below_founding_ground`; §12's `NEEDS_FIRST` row 2026-10-06 against `NeedsFirst`, `needs_first_act`, `needs_hungry`, `hungry_larva_beside`, `need_drop_site`, `closes_a_way`, `in_a_doorway`, `walled_in`, `store_job_waits`, `pack_behind`, `build_out_way`, `shut_in`, `throttle_lifted`, `outward_want`, `step_nest_rest` and `act`'s lean gate, heap cue and face trip; §5 step 6's mound-dig sentence, §6d's mound-way sentences and §12's `MOUND_OUT` row 2026-10-06 against `MoundOut`, `build_mound_way`, `mound_out_pull`, `shut_in_mound`, `hungry_out_gain`, `step_nest_rest`, `chooser_step` and `act`'s lean gate, and its default and step 6's sentence against `MoundOut::SHIPPED` (`dig` alone); §6d's way-gaps paragraph, §5 step 5's soil-way sentence and §12's `WAY_GAPS` and `SOIL_WAY` rows 2026-10-05 against `WayGaps`, `way_cell`, `build_nest_way`, `SoilWay`, `soil_way_pull`, `way_out_from` and `act`'s keep; §5 step 5's face-trip sentence, §15's dig verdicts and §12's `FACE_TRIP` row 2026-10-05 against `FaceTrip`, `face_for_cut`, `face_trip_refuses`, `dig_trip_over`, `dig_return_target` and `act`'s dig; §6d's crop-down paragraph and §12's `CROP_DOWN` row 2026-10-05 against `CropDown`, `crop_down_holds`, `crop_down_unpulled`, `carriers_seek_larvae`, `home_pull` and `act`'s drop; §6d step 5's search sentence and §12's `HOME_SEARCH` row 2026-10-04 against `home_search_of`, `home_search_reach` and `chooser_step`; §2's support and foothold bullets and §12's `WATER_FOOTING` row 2026-10-04 against `water_footing_of`, `stands_on_water`, `head_has_foothold`, `fall_if_unsupported` and `commit_step`; §5 step 6, the walked cycle's lean carrier, §6d's lean exception to the throttle and §12's `LEAN_FORAGE` row 2026-10-03 against `LeanForage`, `lean_drop_site` and `outward_want`; §9's spread and nurse sentences, §6d's nurse paragraph and §12's `BROOD_SPREAD` and `NURSE_SEEK` rows 2026-10-04 against `brood::spread`, `brood::larva_scent`, `brood::nurse` and `chooser_step`; §9's crop sentence, §6d's carrier paragraph and §12's `CROP_NURSE` row 2026-10-04 against `brood::crop_feed`, `creature::crop_to_feed`, `yield_of` and `chooser_step`; §9's egg-rule sentence and §12's `EGG_DOOR` and `BROOD_CARRY` rows 2026-10-03 against `brood::EggBar`, `pile_site` and `carry`; §5 step 3's defended-plant sentence 2026-10-03 against `deterred_by_defence` and `food_value`; §2's support
- **Verified against:** `main` at `bb65d507`, 2026-09-22; §6d's nest-store paragraph and §12's `NEST_STORE` row 2026-10-06 against `NestStore`, `fill_nest_store`, `nest_store_pull`, `pull_pace_target`, `store_inward`, `fetch_target`, `store_pickup_ok`, `store_drop` and `hungry_out_pull`; §6d item 4's fill paragraph, step 5's search sentence, stage 2's turn sentence and §12's `CARRY_HOME` row 2026-10-06 against `CarryHome`, `carry_home_of`, `fills_before_walking`, `carry_route_floor`, `chooser_step` and the tick's fall-then-roll order; §6d's `NEST_REST` paragraph and §12's `NEST_REST` row 2026-10-06 against `nest_rest_of`, `parse_nest_rest` and `NestRest::SHIPPED` (off again); §15's scout-want and door-read bullet 2026-10-05 against `chooser_step`, `outward_want` and `door_read`; §5 step 5's soil-way, lean-keep and face-trip sentences, §6d's way-gaps paragraph and §12's `WAY_GAPS`, `FACE_TRIP` and `SOIL_WAY` defaults 2026-10-05 against `way_gaps_of`, `soil_way_of`, `face_trip_of`, `gaps_hold` and `act`'s lean drop (all three on); §6d's nurse-stay paragraph, §9's nurse-stay sentence and §12's `NURSE_STAY` row 2026-10-05 against `NurseStay`, `relay_crop`, `is_crop_nurse`, `nurse_in_target`, `nurse_unpulled`, `nurse_stays`, `home_pull`, `chooser_step` and `act`'s drop; §15's pull bullet and its guard 2026-10-05 against `home_pull_why`, `home_pull`, `hungry_out_pull`, `rest_pull` and `chooser_step`; §15's nurse-term, walk-back and larva-meal paragraphs 2026-10-05 against `chooser_step`, `dig_trip_over`, `dig_trip_end_why`, `note_feed`, `brood::crop_feed`, `brood::nurse`, `brood_tick` and `act`'s share; §5 step 6's doorway sentence and §12's `DOOR_LOOSE` row 2026-10-04 against `door_loose_of`, `in_a_passage` and `pack_neighbours_with`; §6d's way-out paragraph and §12's `HUNGRY_OUT` row 2026-10-04 against `hungry_out_of`, `hungry_out_pull`, `step_nest_rest` and `chooser_step`; §6d's way-gaps paragraph, §5 step 5's soil-way sentence and §12's `WAY_GAPS` and `SOIL_WAY` rows 2026-10-05 against `WayGaps`, `way_cell`, `build_nest_way`, `SoilWay`, `soil_way_pull`, `way_out_from` and `act`'s keep; §5 step 5's face-trip sentence, §15's dig verdicts and §12's `FACE_TRIP` row 2026-10-05 against `FaceTrip`, `face_for_cut`, `face_trip_refuses`, `dig_trip_over`, `dig_return_target` and `act`'s dig; §6d's crop-down paragraph and §12's `CROP_DOWN` row 2026-10-05 against `CropDown`, `crop_down_holds`, `crop_down_unpulled`, `carriers_seek_larvae`, `home_pull` and `act`'s drop; §6d step 5's search sentence and §12's `HOME_SEARCH` row 2026-10-04 against `home_search_of`, `home_search_reach` and `chooser_step`; §2's support and foothold bullets and §12's `WATER_FOOTING` row 2026-10-04 against `water_footing_of`, `stands_on_water`, `head_has_foothold`, `fall_if_unsupported` and `commit_step`; §5 step 6, the walked cycle's lean carrier, §6d's lean exception to the throttle and §12's `LEAN_FORAGE` row 2026-10-03 against `LeanForage`, `lean_drop_site` and `outward_want`; §9's spread and nurse sentences, §6d's nurse paragraph and §12's `BROOD_SPREAD` and `NURSE_SEEK` rows 2026-10-04 against `brood::spread`, `brood::larva_scent`, `brood::nurse` and `chooser_step`; §9's crop sentence, §6d's carrier paragraph and §12's `CROP_NURSE` row 2026-10-04 against `brood::crop_feed`, `creature::crop_to_feed`, `yield_of` and `chooser_step`; §9's egg-rule sentence and §12's `EGG_DOOR` and `BROOD_CARRY` rows 2026-10-03 against `brood::EggBar`, `pile_site` and `carry`; §5 step 3's defended-plant sentence 2026-10-03 against `deterred_by_defence` and `food_value`; §2's support
  bullet and §12's `KIN_FOOTING` row 2026-10-02 against `fall_if_unsupported`,
  `touches_ground` and `held_by_kin`; §5 step 2's top-up,
  §6d's throttle paragraph and §12's two rows 2026-10-02 against
  `outward_want`, `door_scent` and the share block; §5 step 5 and the
  `SPOIL_HOLD` row 2026-10-01 against `spoil_hold_of` and the drop block;
  §6d's `NEST_REST` paragraph and §12 row 2026-10-01 against `rest_pull`. §2, §6c, §13 and
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
  `spoil_ring_let_go`). §9 and §12 on 2026-10-03 for the brood pile
  (`brood::pile_site`, `egg_pile_reach`, `creature::home_at`), and §12
  for the anchor re-aim switch (`home_reaim`). §5 steps 4 and 6 and §12 on
  2026-10-03 for the door rules (`food_door_of`, `door_clear_of`,
  `in_doorway`, `food_drop_site`, `door_reopen_of`, `door_cue_weight`, the
  heap cue's call in `act`), and §15's `free8` against
  `note_drop_surroundings`. §1, §4 and §9 on 2026-10-03 for the `Lay`
  output (`BrainOutput::Lay`, `LAY_HOLD_BELOW`, `try_bud`, `lays_declined`)
  and the header's `mutation_rate`. §6d, §8 and §12 on 2026-10-03 for the
  `met` forage drive shipped on (`ForageNeed::Met`, `return_met`,
  `meet_returning_forager`, `TRAIT_RETURN_MEMORY`, `door_read`).
  Update this line whenever a section is re-checked against the code.
- **Verified against:** `claude/nest-race-way-foot` at `58afbd26`, 2026-10-07: §12's `WAY_FOOT`, `DOOR_COLUMN` and `LAY_BAR` rows, the `NEST_STORE` row's 2026-10-06/07 parts and the `HUNGRY_OUT` row's `lean` against `WayFoot`, `DoorColumn`, `lay_bar_body`, `NestStore` and `hungry_out_lean`.
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
  (`mutation_rate: 0.0032350` per slot per birth). An evolved colony can
  differ from this page, and a measurement that matters should read the
  genome it ran. §2, §5, §9 and §12 on 2026-09-30 for the default flip: stacking at 4 (`SHIPPED_STACK_CAP`, `parse_stack_depth`), the walked cycle (`parse_spoil_out`, `ring_gate`) and births on nestmates (`parse_bud_stack`).
  §1 step 2 and §12 on 2026-10-03 for the nest-odour kin gate (`blend_with_nest`, `nest_kin_gate`, `CreatureStats::nest_blends_refused`).
  §12 on 2026-10-03 for tolerance drifting at a third of the scent rate (`PIXEL_PHYSICS_TOLERANCE_DRIFT`).
  §15 on 2026-10-05 for the dig's funnel in the trace (`DigWhy`, `DIG_FLAG_DOWN` and friends, `DIG_NO_TARGET`, `act`'s dig roll drawn into a local).

---

## 1. The shape of one tick

`creature::creature_tick`, once per scheduled tick, in this order. Every step
can end the tick early.

1. **Housekeeping.** `reconcile_chain`. A burning ant only reschedules.
   **Old age:** `life_half_life: 40000` frames, rolled every tick. An ant
   mid-`Crossing` (inside a trunk) or mid-`flight` runs that branch instead
   of everything below.
2. **Sense.** `sense` fills the input vector (§3). If `AtNest`, the colony
   scent blends (`blend_with_nest`) with the spatially nearest nest site:
   the ant steps `nest_blend` (0.1) toward the site's odour and the site
   `nest_uptake` (0.02) toward the ant's. **Its own colony's nest always**
   (the site its colony, or one it split from, seeded: `NestSite::colony`,
   `World::descends_from`); **any other nest only if it smells like kin**:
   when the ant's scent is farther from that site's odour than its own
   tolerance radius (`tolerance_radius`, `TRAIT_TOLERANCE + 1`), nothing is
   exchanged either way (`CreatureStats::nest_blends_refused`). Before the
   gate, ten ticks on a rival's nest made an ant a stranger to its own colony
   under its old name.
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
   bud if eligible and the brain's `Lay` does not hold it (`try_bud`).

**How often.** `organism_tick_interval` is `tick_interval: 6`, scaled by the
ant's expressed `TRAIT_PACE` and by its body's leg fraction. A founder decides
**once every 6 frames**, so the ceiling on any per-frame movement rate is 1 in
6. Pace mutates (width 0.15 per birth), so descendants differ.

## 2. The body and the world it can stand in

- **Body:** `Chain(2)`, a head and one segment. Headings are the 8 `DIRS`.
- **Support** (`step_chain`): the body is held up if **any** of its cells has
  a `Solid`, `Powder` or `Plant` cell among its 8 neighbours (`touches_ground`),
  **or, since 2026-10-02, a cell of a nestmate (same species) whose own body
  touches ground** (`held_by_kin`, `PIXEL_PHYSICS_KIN_FOOTING`, on): one ant
  holds the next and no further, and only while the held ant has stood still
  for fewer than `KIN_GRIP_TICKS` (60) ticks, so resting ants do not perch on
  each other. Water never holds a body up here (see Foothold): a body
  standing on a puddle stays because its fall would land in liquid, which
  is not enterable. Otherwise the ant falls one cell. **The fall happens only inside `step_chain`, so only
  after a successful move roll**: an unsupported ant with `P(move) = 0`
  hangs in the air. A fall counts as a move and lays trail.
- **Enterable cell** (`cell_is_enterable`): empty, its own body, or living
  plant tissue (`is_partable`: leaf, wood, grass, reed, moss and fruit are
  walk-through while alive; `TISSUE_PARTING` on), and, under
  `PIXEL_PHYSICS_PUSH_PAST`, loose `crumbs` and brood cells, parted the same
  way (`PushPast`; both on by default since 2026-10-03). A brood cell an ant stands on is
  out of the grid, so `brood_tick` asks `held_brood_at` before booking it lost
  and waits a larva tick instead, changing nothing. A nestmate is
  enterable up to the stack cap, 4 by default since 2026-09-30
  (`PIXEL_PHYSICS_STACK_DEPTH`; at 1 it is not). Parted tissue is
  held by the ant standing in it and closes when the cell is left empty; when
  an ant steps off or dies in a cell a nestmate still stands in (stack depth
  above 1), the nestmate holds the tissue instead (`close_or_hand_over`).
- **Foothold** (`head_has_foothold`): the **head's** 8 neighbours include
  `Solid`, `Powder` or `Plant`, or a nestmate (`climbs_over_kin: true`).
  Ants walk on walls and ceilings. Under `WATER_FOOTING` (built
  2026-10-04, **on** by default) liquid counts too, but **only in the three cells under the
  head** (`stands_on_water`): water is a floor and never a wall, so an ant
  crosses a pool's top and cannot climb a stream. Liquid itself is still
  not enterable. It is the step's rule only: counting water in Support as
  well held up bodies that should have fallen. Alone the step killed 3 of
  5 goal-box colonies that live without it, the door sealed with the
  colony outside; it ships on only with the loose doorway
  (`DOOR_LOOSE`), under which none died (`water_footing_of`'s doc).
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
self-recurrence, and 17 outputs. `squash(x) = x / (1 + |x|)`.

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
| `Lay` | none (0), and 0 never holds a birth: the hold is below −0.1 | §9 |

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
   Under `PIXEL_PHYSICS_SHARE_TOPUP` (off) a leaver beside the donor comes
   first (`is_leaver`: a forager below its grant, empty but for a packed
   lunch, no spoil, within the throttle's reach of its nest site) and gets
   `TOPUP_FRACTION` (half) of the difference; with none beside it the share
   is as above (`topup_shares`, `topup_j`).
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
   into the crop**, and `act` returns. **A defended plant is passed over in
   proportion to its defence** (`OrganismState::defence`, 0 to 1, evolved
   by plants since 2026-10-03; `PIXEL_PHYSICS_PLANT_DEFENCE=0` turns it
   off): `adjacent_food_counted`, the one scan both the `FoodAdjacent`
   sense and the bite read, skips a living plant cell with probability
   `defence` (`deterred_by_defence`, a keyed roll per eater, cell and
   450-frame window, so the same ant keeps refusing the same cell for that
   window rather than re-rolling every tick), and a cell it does take is
   worth `1 - defence` of its face value (`food_value`, so the crop, the
   overlay and the ledger all see the discount). **Two storeroom rules come first**
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
   - the first empty cell among the 8 neighbours, scanned from the
     north-west or the north-east, half and half on a hash of the seed, the
     frame and the cell (`food_drop_order`, since 2026-09-30; before, always
     north-west first, which put every delivery on the ant's west side and
     walked the colony out east: `PIXEL_PHYSICS_DROP_SIDE=west`);
   - if there is none, **the nearest empty cell reachable by handing the food
     through bodies**, the ant's own and any other creature's, never through
     ground, nest material or food already put down. So a blocked ant passes
     its food back along its body or through the crowd;
   - if even that finds nothing, nothing happens: the roll is spent.

   `PIXEL_PHYSICS_DROP_REACH=adjacent` turns the second rule off. **Neither
   rule counts a cell in a nest's door as empty** (the food-door rule, on
   since 2026-10-03; `PIXEL_PHYSICS_FOOD_DOOR=off` removes it): a cell within
   the door clearance of any colony's founding shaft -- that many columns
   either side of it, from that many rows above its mouth down to its foot --
   is skipped (`in_doorway`); the chamber under the foot and a side room are
   not door, and take food. A nest worker piling food (`store_drop`) asks
   the same search, with the same test. The clearance is 6 cells at the shipped allele and is a
   gene (`TRAIT_DOOR_CLEAR`, slot 24, on the reciprocal axis: allele +1
   doubles it, -1 halves it; `door_clear_of`). So a fed ant at the door
   walks its food out past the clearance, or hands it through the crowd,
   before putting it down.

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
   3.4-4.6×; food taken from the pile is the honest flow. **That flow is
   `trip_deliveries`** (2026-10-04): a delivery of a cell the same ant bit on
   a trip, on the gate that marks a trip load (step 3's `trip_load`: the ant
   had been `FORAGE_TRIP_MIN` cells from its last nest contact, and the food
   lay beyond the trip reach from every door, or was living tissue). The crop
   keeps `trip_cells`, the count of its cells taken that way; a put-down
   spends one (trip cells first) and a delivery while one is spent books it.
   Each such cell counts once, so a crumb shuffled about the nest afterwards
   does not count again. Per ant it is `life.trip_deliveries`, shown on the
   cell page as `DELIVERED n (m FROM TRIPS)`. Why: the deep trace's top
   "forager" by `deliveries` had 291, of which 16 were food bitten at the
   pile.
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
   **By default since 2026-09-30 the cycle is walked** (`SpoilOut::ON`,
   §12's `SPOIL_OUT`; `off` restores what this step describes above):
   inside the nest (`inside_nest`) the drop is held and nothing is lifted
   while the haul's patience lasts; the carrier is pulled at the laden pace
   to the door, draws its column when it comes out by it (`carry_stage`,
   `SPOIL_RING`'s row), is held until it is that far out, and puts the
   pellet down there by the rule above; then, if not hungry, it walks back
   to the cell it cut (`dig_return_target`). **A lean carrier puts its
   pellet down where it stands** (`LeanForage::drop`), before any of this:
   in the first cell beside it that is empty with two of the three cells
   under it filled, no headroom asked (`lean_drop_site`, `lean_dropped`);
   with no such cell it carries on as above. A carrier whose patience runs
   out inside keeps the pellet if it is within `SPOIL_HOLD`'s 12 cells of the
   haul's target (`spoil_hold_of`); further in it may lay the pellet beside
   itself, and it is never lifted. **Under `PIXEL_PHYSICS_SOIL_WAY` (on since
   2026-10-05) the soil leaves by the passages** (`soil_way_of`): a carrier inside its
   nest (`in_nest_for_soil`: `inside_nest`, or below the founding ground) is
   pulled along the nest's way in towards the door (`soil_way_pull`, the
   way out's own step, `way_out_from`) instead of straight at the shaft's
   top, below the founding ground it never lets the pellet go
   (`spoil_held_below`), and under its `lean` part a lean carrier keeps its
   pellet until it is above the founding ground; in the spoil mound, which
   is cover and so reads as inside, it puts it down where it stands, because
   lean carriers held there starved holding their pellets. Counted in
   `soil_way_pulls`. **Under
   `PIXEL_PHYSICS_FACE_TRIP` (on since 2026-10-05) the digger keeps its
   face through the trip** (`face_trip_of`): `below` makes any cut below the founding ground
   a face wherever the digger stood (otherwise only a cut made standing
   `inside_nest`, which the room under an open door is not, and a cut in
   the spoil mound is; `face_for_cut`); `door` aims a digger not yet below
   the founding ground (nor in the founding cut) at the door rather than
   at its face; `food` lets food in the crop of a digger not hungry pause
   the walk back instead of ending it; `only` refuses, while it walks back,
   a cut more than `FACE_TRIP_REACH` (3) cells from its face
   (`face_trip_refuses`, `DigWhy::Face`, `digs_refused_face`); `stay` means
   arriving does not end the walk back, only the next cut or patience does
   (`dig_trip_over`).
6. **Dig**, only if both crop and spoil are empty. **So a laden ant never
   digs.** **Nor does a lean one** (`PIXEL_PHYSICS_LEAN_FORAGE`, on since
   2026-10-03, `LeanForage::nodig`): below `LEAN_LINE` (half) of its
   `start_energy` its `Dig` urge reads 0, and the roll still spends its draw
   (`lean_digs_skipped`). Under `PIXEL_PHYSICS_MOUND_OUT`'s `dig` (on since 2026-10-06) a lean ant shut in the
   spoil mound, on neither the nest's way nor the mound's, keeps its roll
   (`shut_in_mound`, `mound_digs_let`). **Under `PIXEL_PHYSICS_FORAGER_NODIG` (built 2026-10-08, off) a forager the colony is calling does not dig either**: with its forage drive (§6d, `forage_drive_level`) at or over the line (0.5 for `on`), its `Dig` urge reads 0 the same way, with the same shut-in exceptions (`forager_nodig`, `forager_digs_skipped`); it is for the foragers that hand their load over on the mound and then cut it. The roll is against `Dig`, and the target is **the cell straight
   ahead of the head, along its current heading**, except where the face
   turn below picks another cell for a nest worker inside the nest. Nothing
   chooses a place near other digging. **An enclosed digger first turns down**: on a
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
   replaces the turn and a lost move roll leaves it standing. **A digger
   inside the nest that faces no ground can turn to the nearest face**
   (`PIXEL_PHYSICS_DIG_FACE`, on for nest workers since 2026-10-03,
   `=off` removes it; `dig_face_of`): on a won roll,
   when the cell ahead is not ground its jaw can take and no widening cut was
   chosen, an ant inside the nest (`inside_nest`) turns straight to the
   nearest octant round from its heading whose cell it can cut without the
   roof refusing it or the heap cue scaling it (`dig_face_turn`; the side
   tried first is the half-turn coin), and cuts that (`digs_faced`). The
   shipped `workers` turns only a nest-bound ant, so a forager digs as
   before; `on` turns every such digger. **The heap
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
   the cue when it cuts level or up. **A cut into a nest's own founding cut
   meets only part of the cue** (the door-reopen rule, on since 2026-10-03;
   `PIXEL_PHYSICS_DOOR_REOPEN=off` removes it): the cue's chance `f` becomes
   `1 - w(1 - f)`, `w` the ant's `TRAIT_DOOR_CUE` gene (slot 25) clamped to
   0..1, and 0 at the shipped allele, so a door that loose soil, grass or
   water has plugged is dug open again as if it were a tunnel
   (`door_cue_weight`). At `w` 1 the cue holds there as everywhere else. It must not be empty, a creature or
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
   footing). `PIXEL_PHYSICS_SPOIL_PACKS=on` packs spoil as well. **Nor
   does it pack a nest's doorway** (on since 2026-10-04;
   `PIXEL_PHYSICS_DOOR_LOOSE=off` packs it as before): a cell in a founding
   shaft's own columns, from 8 rows over the mouth down to the mouth's last
   row, is left as it is (`door_loose_of`, `in_a_passage`), so soil that
   slides into the mouth stays loose soil and is cut or falls on down the
   shaft instead of being tamped into a plug. The shaft's side walls and
   everything deeper are lined as before.

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

**Every gain below is the ant's own, inherited** (since 2026-10-03). Six
`CREATURE_TRAITS` slots, `organism::WALK_GAIN_SLOTS` (14–19), each a factor on
one constant through `walk_gain` on the reciprocal axis (+1 twice, −1 half,
0 exactly 1.0): `TRAIT_HOME_PULL` on `HOME_GAIN`, `TRAIT_TRAIL_HOLD` on
`TRAIL_GAIN`, `TRAIT_ROUTE_AWAY` on `AWAY_GAIN`, `TRAIT_SCOUT` on the
scouting gain, `TRAIT_DOOR_READ` on the door reader's `FOOD_TRAIL_GAIN`, and
`TRAIT_PATIENCE` on the leak of both patiences (`1 − PATIENCE_DECAY`; +1
halves it). **They ship at allele 0 and mutation width 0 on every species**,
so the walk is bit-for-bit the constants until a measurement turns the width
on, and they have no developmental weight (`brain::DEV_TRAITS` stays 14,
because a live dev slot is drawn by every birth's mutation). The constants
named below are the ancestral values. **Which trail is a route is
inherited too**: four more slots (`TRAIL_PLANE_SLOTS`, 20–23) weight each
plane's presence for a laden and an empty ant, base 1 on the plane the code
used to pick and 0 on the other (`trail_planes`), so allele 0 is laden-reads-A,
empty-reads-B exactly.

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

   **Under `PIXEL_PHYSICS_CARRY_HOME=fill` (off) a carrier fills up before
   it walks** (`fills_before_walking`): when its crop holds a trip's load
   (`trip_load`) with room for one more cell, it carries no spoil, its
   `Feed` urge is above 0, and the food beside its head is its load's
   material and lies beyond the trip reach of every door, the won roll is
   spent standing (outcome `filling`, `CreatureStats::carry_fills`), and
   `act` takes the next mouthful on the next tick. Food at a door never
   holds a carrier, nor does food `act` would refuse.
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
   **`PIXEL_PHYSICS_HOME_SEARCH` (off) gives a food carrier whose patience
   has run out a search** (`home_search_of`): once it is under
   `SCOUT_GIVE_UP` (0.1) and more than 8 cells (`HOME_SEARCH_REACH`,
   Chebyshev) from where it set its best, the carry starts over from where
   it stands, patience 1 and the best forgotten, and the next loop may go
   twice as far (`home_search_loops`, held at 64 times). Nearer than that it
   wanders, which is the search. Built 2026-10-04 for the carriers that lost
   the pull on the spoil mound and walked off west laying trail B (48-68% of
   food trips on the goal box). **Off, because it shrinks the colony**:
   on the dry goal box with the evolved founder (evolution off, 4 seeds,
   300k) fewer carriers end up west, but mean ants over 100-300k fall on 4
   of 4 (436/461/367/399 -> 257/341/12/388), and on 9 of 12 seeds of the
   game after it. The why is traced: the carriers it brings back are the
   ones that ate their fill at the heap, so they leave the heap with less
   and the colony takes less food off it.

**Stage 2 (`PIXEL_PHYSICS_CHOOSER=trail`) adds two things:**
- **The trail where a step would go.** For each heading, `trail_presence`
  reads the scent in the cell the head would enter and the one beyond it,
  takes the larger, and saturates it as `x / (1 + x)` over `TRAIL_HALF` (a
  tenth of one full deposit). Trail B for an ant carrying no food, trail A
  for one that is, at the ancestral alleles: the reading is each plane's
  presence times the ant's inherited weight for it, summed and floored at 0
  (`trail_planes`), and a zero-weight plane is not read. The heading's turn score is multiplied by
  `1 + TRAIL_GAIN × presence` (`TRAIL_GAIN` 3), so a heading onto a full
  route scores up to 4 times its turn alone, and turning round still scores
  0. Presence has no direction: both ways along a route score the same.
  **Under `PIXEL_PHYSICS_CARRY_HOME=turn` (off) a carrier holding a trip's
  load counts presence above the weakest of its options** (the usable
  headings and the crossing; `carry_route_floor`,
  `CreatureStats::carry_turns`): `1 + TRAIL_GAIN × (presence − floor)`. A
  route still holds it, but scent lying even over every heading, as nest
  scent lies over the spoil mound, holds nothing, so the pull home can turn
  it.
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

**Under `PIXEL_PHYSICS_FOOD_TRAIL=...,giveup` (on by default since 2026-10-01) the trail lets
a given-up scout go, and a walked trail's end bounds the scout.** Once
`scout_home` is set by a step onto a heading with no trail
(`OrganismState::scout_dark`; a give-up on a lit route is left as it was),
the trail's hold on the heading is dropped (the
`1 + TRAIL_GAIN × presence` factor is 1), the away term is 0, and the pull
home is `scout_w × level cos(heading, home)` without `(1 − presence)`. And
once an excursion has stepped onto a heading carrying trail B
(`OrganismState::scout_lit`), a step onto a heading with none is not
progress however far out it lands, so patience decays on the dark ground past
the trail's end. A scout that never met a trail scouts as without it.
**Past its point of no return the bound lets go** (`noreturn`, on by default
since 2026-10-01): a scout that has spent more energy on this excursion than
it has left (`energy < scout_e0 − energy`, `OrganismState::scout_e0` taken
when the excursion starts) counts dark progress as progress again, so it
scouts on instead of turning for a home it cannot reach.

**Under `PIXEL_PHYSICS_FOOD_TRAIL=...,read` (on by default since 2026-10-01) an empty ant at
the door turns toward the food side** (`door_read`, Stage 2,
`ant-scenes-2026-09-23.md` §23e). The ant must be unladen (a packed-lunch
carrier counts as empty), with no home pull and no spoil, not a given-up scout,
and standing in the door box (`door_site`): within the door's half-width plus
one column of a nest site, on the walking row or up to `DOOR_READ_RISE` (4)
rows above it over the spoil mound, and never inside the cut. It reads trail B
at its own reach-6 sensors, east and west on its own row, giving
`g = (bE − bW) / (bE + bW + TRAIL_HALF)`. Every heading on g's side that does
not land below the walking row gets `gain × want × |g|`, where `gain` is
`FOOD_TRAIL_GAIN` (6) and `want` is the larger of hunger and the forage
drive. Nothing is subtracted, and with both sensors dark there is no term.
The term is withheld once the nest's last return (`World::nest_last_return`)
is older than `return_window()`, so a pile that has run out does not keep
leashing the door. Counters: `door_reads`, `door_stale`, `door_pulled`, and
`door_followed` (the picked heading was on g's side).

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

**`PIXEL_PHYSICS_NEST_REST` (off; `workers` was the default for one day,
2026-10-05 to 2026-10-06) gives an ant with nothing to do a pull into its
nest.** Who it may reach is the form (`NestRest`): `workers`, nest
workers only; `on`, nest workers and any ant that has foraged
(`OrganismState::foraged`), so a scout that has never found food never
rests; `all`, every ant. Of those, it reaches an ant `home_pull` gives no target (no food, no
spoil, no store trip, no way back to a face, no hungry-home latch) whose
`max(hunger, forage drive)` is under `REST_BALANCE` (0.5), at gain
`home_bias × (1 − that / 0.5)` (`rest_pull`). Outside the nest the target is
the door over the mouth; on the nest's way in (`NestWay`: a breadth-first
step count from that door over cells an ant can stand in, in the founding cut
or under cover below the door, rebuilt every `REST_REFRESH` (30) frames by
`step_nest_rest`) it is the cell `REST_LOOKAHEAD` (3) steps further in, the
ant's id ordering the neighbours so forks split the colony; where no step
leads further in, the target is the ant's own head. Like any pull it
suppresses scouting and the away term. Counted in `rest_pulls`. On the
dry goal box (evolved founder, evolution off, 4 seeds to 200k, means over
100-200k) `workers` against `off`: colonies larger on 4 of 4 (541-562 ->
551-613 ants), nest workers starved over the run 3-21 -> 1-11, their share
of decisions below the founding ground 8.3-10.3% -> 9.3-12.3%, and cuts
below it 336-392 -> 442-597; in the played lab box (12 seeds) births
290.5 -> 325.5 and animals underground 16.6% -> 19.9%. On the colony bed (B1, 24 seeds) it still
costs what §20 of the nest report found: born 4,362 -> 3,931, starved
47 -> 97 (`NestRest`'s doc). **Off again since 2026-10-06**: run on to 300k
on the goal box, `workers` collapses two colonies of four after about 230k
(seed 1 553 -> 57 ants, seed 4 586 -> 17; seeds 2 and 3 hold) and `off`
none. How is not yet traced.

**`PIXEL_PHYSICS_HUNGRY_OUT` (on since 2026-10-04; `off` is the ant before
it) gives a hungry ant inside its nest the way out.** Inside a dug nest the anchor follows the ant (§8), so the scout's pull
out from home has no direction there. Under the switch, an ant that carries
nothing (no food, no pellet), is under its grant, and stands on its nest's
way in (`NestWay`, the rest pull's field, built while either reads it) is
pulled along the passages towards the door, `REST_LOOKAHEAD` (3) steps at a
time (`hungry_out_pull`), at `home_bias` times what the scout's pull out
would be (`outward_want`: its hunger, or at the door what the throttle lets
out). It is tried after `home_pull` and before the rest pull, and never meets
it: the rest pull wants an ant the pull out does not reach. At the door
(step 0) and outside there is no pull. Counted in `hungry_out_pulls`. On
the dry goal box with the evolved founder (300k, 4 seeds, mean ants over
100-300k) colonies are 1.31-1.56 times the size with evolution off and
1.12-1.50 with it on, higher on 4 of 4 each, and starvation over the same
span falls 77-93%. **The way in starts at the door and runs down, so an ant in the spoil
mound above the door is not pulled.** Under `PIXEL_PHYSICS_MOUND_OUT=way`
(off) each nest also has a way out of its mound (`build_mound_way`: from
every open-air standing cell over the covered ones, at or above the founding
ground), and a hungry ant there that the way out does not pull is pulled
down it at the same gain (`mound_out_pull`, `mound_out_pulls`). Either way
a pellet holder is not pulled (`hungry_out_gain`).

**The way in had two gaps, which `PIXEL_PHYSICS_WAY_GAPS` (on since
2026-10-05) closes** (`WayGaps`). Without it the way counts a cell as inside
when it is in the founding cut or has ground within `COVER_REACH` (20) rows
overhead, and as standing room only when it is open or holds an animal. So
the room dug under the door, more than twenty rows below the open mouth, is
outside (`below` counts every cell under the founding ground as inside), and
the brood pile, which the walk parts (`PushPast`), is a wall (`brood` counts
what `is_partable` passes as open). The brood column stands in the shaft's
foot, so on the goal box the way reached a median 124 cells of the nest (401
with brood open), and the ants that starved inside spent 75-83% of their
last 3,000 frames with no pull at all. With it a hungry empty ant's steps in
the nest with no pull fall to 0.1-0.5% and starvers in the nest to 0-4
(deep trace, seeds 1-4); `brood` alone does all of it. It went on with
`SOIL_WAY` and `FACE_TRIP`, which both need it (§5 step 5).

**`PIXEL_PHYSICS_FORAGE_DRIVE` (`met` since 2026-10-03, `returns` from
2026-09-29, `always` from 2026-09-27; `off` is the ant before it) sends a fed
forager out while food is coming home.** It reaches an animal that has foraged (`OrganismState::foraged`,
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
- `returns` (the default 2026-09-29 to 2026-10-03): 1 while its nest last saw
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
- `met` (the default since 2026-10-03, `ForageDrive::SHIPPED`): `returns`
  sensed rather than told. The same plateau and fade, counted from
  `OrganismState::return_met`, the last frame *this ant* met a forager home
  with food from a trip: the booking above also stamps the deliverer and
  every live ant of its species within `RETURN_MEET` (3) cells of the drop
  (`meet_returning_forager`). An ant that has met none counts from its
  `born_frame`. The window is `W` times the ant's own `TRAIT_RETURN_MEMORY`
  gene (slot 26, reciprocal axis, width 0). Under `met` the door reader's
  stale gate reads the same memory in place of `nest_last_return`. Stamped
  whatever the drive; read only under `met`. Shipped on the owner's ruling
  that it is the more correct rule unless it makes the world clearly worse;
  against `returns` (played bed, 12 paired seeds, 240k frames) colonies lost
  4 -> 3, deepest fall from peak 97% -> 95.5% median, births 603 -> 552.5,
  starved per million ant-frames 20.7 -> 21.8, none significant.
  `OrganismState::trip_src` records where a crop's marking pickups were
  taken, for `CreatureStats::trip_returns_near` and
  `trip_returns_tissue_near` (an upper bound on the tissue exemption: any
  crop holding tissue taken within the reach).
**The forage throttle (`PIXEL_PHYSICS_FORAGE_THROTTLE`, on since
2026-10-02) decides who goes out near the door** (`outward_want`, read by
scouting's pull, the door reader's want and the rest pull's "out"). Within
`THROTTLE_REACH` (16, scaled) of its nest site's centre column and walking
row (`throttle_site`), a forager that is not nest-bound wants the largest of
the forage drive, the food scent at its door (`door_scent`: trail B summed
over the door's box, the door's half-width plus one each side on the walking
row and the one above, read as `b / (b + TRAIL_HALF × cells)`) and, if it has
never foraged, `THROTTLE_PATROL` (1). **Its own hunger is not in it**, so a
hungry forager at a door with no drive and no scent is held -- **unless it
is lean** (`LeanForage::out`, below `LEAN_LINE` of its grant), when its
hunger is its want there too. Each such
scouting decision stores the want in `OrganismState::sent_want`; past the
zone, an ant with one feels `max(drive, sent_want)` where that beats its
hunger, else its hunger (under `,hold`, never its hunger). A nest-bound ant,
and an ant the throttle has never judged, scouts on `max(hunger, drive)` as
above. The forage pace (`forage_pace`) still reads the drive alone. Counted
in `throttle_reads`, `throttle_held` (hunger above the want) and
`throttle_sent` (the want above hunger).

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

**Under `PIXEL_PHYSICS_NURSE_SEEK` (off by default) a nurse walks up the
scent of hungry larvae** (`brood::larva_scent`, the `nurse` closure in
`chooser_step`). The ant must have something to give, as `brood::nurse`
takes it: a bank over its `start_energy`, no food in its crop (a packed
lunch does not count), no pellet, and its
head inside the nest (`inside_nest`); under `on` (or `workers`) it must also
be nest-bound (`is_nest_bound`), under `all` any ant will do. Every larva of
its own colony within 6 cells either way (`NURSE_SCENT_REACH`) adds its
hunger, the share of its pupation target it still lacks, over its distance
squared, along the line to it; the summed vector's direction is the pull and
its length `L` sets the strength `L / (L + 0.25)` (`NURSE_SCENT_HALF`). Each
heading scores `gain × strength × cos(heading, pull)`, `gain` 1
(`NURSE_SEEK_GAIN`, or the number given). A larva a walker holds gives no
scent. Counted in `CreatureStats::nurse_seeks`; `larva_ticks_hungry` and
`larva_ticks_nursed` count the larva ticks that ended hungry and, of those,
the ones with a fed nestmate touching it. `World::nurse_seek` for one world.

**Under `PIXEL_PHYSICS_CROP_NURSE=on` (not the default, which is `touch`) a
carrier walks up the same scent**, because food in its crop is what a
larva can be fed from (§9). The ant must hold crop food it can give
(`creature::crop_to_feed`: not a packed lunch, no seed riding in it, no
pellet in its jaws), a bank over its `start_energy`, and its head inside the
nest; it scores the same `larva_scent` pull at gain 1 (`NURSE_SEEK_GAIN`),
counted in the same `nurse_seeks`. Nest-bound or forager makes no
difference. `touch` leaves the walk alone and only feeds.

**Under `PIXEL_PHYSICS_CROP_DOWN` (off by default) a fed carrier brings its
crop down to the brood** (`creature::CropDown`). It acts on an ant over its
`start_energy` holding crop food it can give (`crop_to_feed`). `hold`: its
drop roll at home is skipped while its crop is down to its last cell (`on`;
`all` keeps every cell) and its head is no more than 5 rows
(`CROP_DOWN_DEPTH`) under the founding ground; the roll is still drawn
(`crop_down_holds`, counted in `CreatureStats::crop_down_holds`). `scent`:
below the founding ground it has no pull home and walks up the larva scent
as under `CROP_NURSE=on` (`crop_down_unpulled`, `carriers_seek_larvae`).
Without it, carriers spend 92% of their walking decisions in the mound and
on the door and 0.6% below the old ground, and put their crop down at the
door. Measured 2026-10-05 (dry goal box, seeds 1-6): larvae starved per egg
lower on 5 of 6 under `on` and under `all`, colony 0-17% smaller under `on`
and 13-37% under `all`; traced under `all`, no crumbs are left at the door
for newborns and carriers make fewer trips.

**Under `PIXEL_PHYSICS_NURSE_STAY` (built 2026-10-05, off by default)
foragers hand their crops to nurses, who take them to the brood**
(`creature::NurseStay`). A nurse is a nest-bound ant holding crop food it
can give (`is_crop_nurse`). `relay`: a forager at its nest that is not
nest-bound and holds crop food (`crop_to_feed`) hands its whole crop, before
its drop roll, to a touching kin nest worker with an empty crop; it is
booked as a trip home (`forage_returns`) but not as a delivery, so
`deliveries` and `trip_deliveries` miss food handed on (`relay_crop`).
`down`: it may hand it instead to any touching kin with an empty crop below
the founding ground and below its own head, the deepest first, and a nurse
still above the founding ground does the same; the receiver becomes a nest
worker for 2000 frames (`NURSE_STAY_FRAMES`). `nurse`: a nurse never puts
its crop down at home (`nurse_holds`); below the founding ground it has no
pull home and walks up the larva scent fed or hungry, eating out of its own
crop; above it, it is pulled to 3 rows under its nest's door
(`NURSE_IN_DEPTH`, `nurse_in_target`) and keeps its patience on the way, as
a leashed ant does. `stay`: a nest worker that feeds a larva, from its crop
or its bank, stays one 2000 frames longer (`nurse_stays`). Counted in
`CreatureStats::nurse_handoffs`, `nurse_passed_down`, `nurse_converted` and
`nurse_holds`; `larva_ticks_fed_away`, `brood_fed_away_j`, `pupae_away` and
`larvae_starved_away` count larvae more than 3 columns off the door
(`brood::DOOR_LANE`). Without `down`, nurses holding food stood on the spoil
mound behind the idle ants in the doorway, 0.3-2 of 5-28 underground.
Measured 2026-10-05, before PR 629's nest switches (dry goal box, evolved
founder, seeds 1-4, 100k-300k): larvae starved per egg laid 19-21% -> 11-15%, and 30-35% -> 13-25% with the
brood spread; larvae ate 1.0-1.4 MJ from crops against almost none; the
colony is 6-33% smaller from 40k to 150k and the same size from 200k; food
taken from the heap is 12-30% lower; grown foragers starving above ground
rise from 27-38 to 66-342 a run, starving underground falls. The brood
column under the door comes back by 200k-300k: nurses feed it, nothing
moves it. **Off because, with `SOIL_WAY`, `WAY_GAPS` and `FACE_TRIP` on, it
kills colonies** (owner's rule: such a fix stays built but off until the
deaths are understood). Same box and seeds on main 043e9104, 100k-300k:
live ants 128-293 against 529-573 with nurses off, grown ants starved
477-1070 a run against 7-12, heap intake 3.6k-11.0k cells against
21.4k-25.2k (bites taken 8.3k-25.0k against 54.4k-64.5k), and seed 2 died
out by 260k; nurses with the three off are
488-583. Why the pair starves the colony is traced meal by meal: the
hand-off takes 36-42% of the foragers' loads from the ants that ate them at
the door. `pace` (2026-10-06, in `on`): a nurse reads its `HomeAligned`
bearing to the larvae (`nurse_pace_target`) -- above the founding ground the
point under the door, inside the larva scent 4 cells up it
(`NURSE_PACE_LEAD`), past it the nearest hungry larva within 16 cells
(`NURSE_FILL_REACH`, `brood::nearest_hungry_larva`) -- not its forage
anchor, which the nest re-sets to where it stands: without it laden ants
inside the nest stood on their own anchor in 97-99% of their decisions and
idled in 74-76%. Under `NEST_STORE`'s `keep` a fed nest worker may fill its
crop at the store while a hungry larva lies within 16 cells
(`store_nurse_may_fill`), so a nurse can be fed from the store rather than
from the door. **`pace` makes the switch lethal** (2026-10-06, `NEST_STORE=on`,
`NEST_REST=off`, seeds 1-4, 300k): `on` killed 4 of 4 colonies, the same
parts without `pace` none, `nurse,stay,pace` one. Traced on seed 1: nurses
put the colony's food into larvae (crop food to larvae over 30-100k 5 kJ with
nurses off, 426 kJ without `pace`, 534 kJ with it, hand-offs no more
frequent), hungry adults are topped up by nestmates about half as often and
spend their hunger on the mound and surface, and die there holding soil
pellets or scouting west: 0, 328 and 562 starved.

**Under `PIXEL_PHYSICS_NEST_STORE` (built 2026-10-06, off by default)
food has a store inside the nest** (`NestStore`). The store is loose food
(not corpses) beside the nest's way at least 20 cells in along it (`depth`);
`fill_nest_store` marks those cells and their walk distance when the way is
rebuilt, every 30 frames. `carry`: a nest worker's won eat roll on loose
food at home that is not store food takes it in its jaws, and its pull home
(`nest_store_pull`, ahead of every other pull) and its `HomeAligned`
bearing (`pull_pace_target`, `store_inward`) lead down the way until it is
in the store or stuck, where it drops. `eat`: a hungry ant inside is pulled
to the store when it holds at least 8 cells (`STORE_EAT_MIN`), and
`HUNGRY_OUT` does not send it out while the store can feed it (it still
does when the store is empty -- the way newborns start foraging). `keep`:
a fed ant does not eat store food. `home`: a fed idle nest worker is
pulled deeper. `larder`: the colony's larder need reads the store. `pick=N` lets
a nest worker take food within N cells of the door from the doorstep (0
by default: at 20 carriers took food off the mound). `jaws`: a full crop
does not stop the pick-up. `fetch`: an idle fed nest worker is pulled to
loose food by the door. Measured 2026-10-06 (seeds 1-4, 150k,
`NEST_REST=off`, scored 50-150k): 190-350 loads carried in per run against
3,500-4,500 food trips home, and every load eaten on arrival -- the store
held 0-1 cells on average in every arm. What stops more loads is the soil
pellet: of 1,156 won eat rolls by fed nest workers at door food, 892 held a
pellet and about 17 only a full crop, so `jaws` gained nothing, and `fetch`
raised loads 15-25% while the share of nest workers inside fell from 8% to
6%. `on` is `carry,eat,keep,home,larder`; `jaws` and `fetch` are named
alone.

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
    (`PIXEL_PHYSICS_FOOD_TRAIL`, on unless set) multiplies it by 1 while
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
  - **B, in the lab box:** `rho = LAB_B_RHO = DECAY_RHO = 0.03`, blend
    `LAB_B_DIFFUSE = 0.05` (2026-10-04): a thin food trail at 10-46 of 255
    along the path. **Elsewhere** (the outdoor and held worlds) B is the
    engine pair, `DECAY_RHO = 0.03`, blend 0.25, at which a lone laden pass
    lays ~29 and is gone in ~144 frames, mostly to the blend, so B never
    builds up beside an endless pile (`examples/trailprofile.rs`).
    `PIXEL_PHYSICS_B_RHO` / `PIXEL_PHYSICS_B_DIFFUSE` override both. **A
    slower fade fails**: at `rho = 0.005` old side paths stay lit, empty ants
    follow any lit B away from home, and the goal box lost a fifth of its
    food (`pheromone::b_rho`'s doc).
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
- **Shipped since 2026-10-02: home is the dug nest** (`NestHome::Dug`). A
  cell counts as next to the nest when it is within one cell of
  `World::nest_dug`: every open cell (empty, an animal, or loose spoil,
  corpse, crumbs or unowned food) at or below a nest site's old ground line
  that a 4-connected walk reaches from the door's surface cells or the
  founding cut, within `DUG_HOME_REACH` (60x60) of the site, rebuilt every
  `ROOM_INTERVAL` frames (`World::step_nest_dug`). So every tunnel and room
  the colony digs from the door is home. **A plant grown into a dug cell does
  not cut it** (`PIXEL_PHYSICS_HOME_PAST_TISSUE`, on since 2026-10-03): a dug
  cell (`World::dug_cells`) holding living tissue a body gets through (a
  blade, root or leaf it parts, a stem it crosses) is open to the fill; the
  same tissue in undug ground is not. `PIXEL_PHYSICS_NEST_HOME=material`
  is the painted door strip, the home before this.
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
  last came home to it with food from a trip, read by the `returns` drive
  (§6d; the shipped `met` drive reads each ant's own `return_met` instead); `World::step_nest_need` stamps a site it has not seen with the
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
- **Diet quality** is `(1 - |gut - food_class| / 2)^2` (`diet_quality`).
  Every plant food is `food_class` -1, flesh +1. **A lab ant is founded at
  gut -0.5** (`scene::LAB_ANT_GUT`, since 2026-10-04), so it gets 0.56 of a
  plant cell -- 540 J of a 960 J provisions cell -- where the shipped
  generalist at 0 gets 0.25. That is the range a lab colony's gut evolved to
  on its own; see `PIXEL_PHYSICS_LAB_ANT_GUT` in §12.
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
  `try_bud` above `reproduce_threshold: 1100`, wherever the ant stands,
  **unless its brain holds the birth**: on a tick the ant could afford a
  child, `BrainOutput::Lay` below −0.1 (`brain::LAY_HOLD_BELOW`) holds it,
  before any rule about where, and `CreatureStats::lays_declined` counts
  those ticks. The founder wires nothing into `Lay`, so it reads exactly 0
  and never holds; a lineage that evolves weights there decides when and
  where it breeds.
  The bar counts the food in the eight cells around the head as well as the
  bank (`reachable_provision`), and a birth the bank cannot cover eats that
  food to make up the difference (`place_creature`'s `Origin::Bud` arm),
  leaving the parent at 1 J or more. A bare seed there counts only what a
  bite that spares it pays (`seed_provision_fraction`, a quarter), because
  a bite spares it 60% of the time (`plant::guaranteed_bite_fraction`,
  `PIXEL_PHYSICS_BIRTH_PRICE`); counted at face, the top-up could fall
  short and the parent died the next tick. `CreatureStats::births_overdrawn`
  counts births that leave a parent under the floor.
  By default (since 2026-10-02; `PIXEL_PHYSICS_BUD_SITE=anywhere` or
  `World::bud_at_nest` turns it off) a species that names a nest material
  lays or buds only while at its nest (the `AtNest` read); `CreatureStats::buds_held_for_nest` counts the ticks it could have
  budded and did not. **An egg is held to the nest by where the egg lands,
  not where the layer stands** (`brood::pile_site`,
  `PIXEL_PHYSICS_EGG_PILE`, on at a reach of 4; `off` is the head read):
  a walk of up to 4 steps from the head, through empty cells and
  nestmates, to an empty cell at home (`creature::home_at`, the live home
  definition); one touching brood already lying there wins (the brood
  pile), then the fewest steps. No such cell, no egg. Cells the egg rule
  refuses (`brood::EggBar`, `PIXEL_PHYSICS_EGG_DOOR`, off by default: `door`
  is the shaft and the ground round its mouth, the cells a food drop keeps
  clear, at the layer's own door gene) are walked through but never chosen,
  here and for an egg laid beside the head. **A crowded pile can be
  spread** (`brood::spread`, `PIXEL_PHYSICS_BROOD_SPREAD`, off by default):
  at a brood item's tick, if 6 or more of the 24 cells round it hold brood
  and a grown nestmate touching it has free jaws, it is carried 5 to 10
  steps through the nest to an empty floored home cell with at most 4 brood
  round it and no loose food within 3 cells (most brood round it wins, then
  the farthest). Budding keeps the
  head read. An ant whose own bank clears the bar
  with an empty crop walks home to lay as a laden ant walks home
  (`ready_to_lay`, `PIXEL_PHYSICS_LAY_HOME`, on; `laden` includes ants
  carrying food). That walk ends at the first home cell, the door; under
  `PIXEL_PHYSICS_LAY_IN=on` (built 2026-10-08, off) a ready ant beside home
  with no egg site in reach is pulled on along the nest's way in, away
  from the door, until one is (`lay_in_target`, the rest pull's walk
  `deeper_along_way`; the trace names it `walk in to lay`). **The lab box lays only at the nest too** (since
  2026-10-03, the owner's ruling, though nest-only laying there took births
  329 -> 4 and none of the fixes tried brought it back). With `PIXEL_PHYSICS_BUD_STORE=on` (or
  `World::bud_store`) a nesting species' births are paid from its store
  (`bud_from_store`): only an animal within a cell of the storeroom
  (`ShaftFootprint::touches_store`) buds, its own bank is read as 0 and the
  bar as the birth's price, and loose food anywhere in the founding cut and
  its storeroom pays all of it, nearest first (`provisions_in_store`,
  `place_creature`'s `Origin::Bud` arm), counted in
  `CreatureStats::store_births`. The child goes on the first of the eight neighbours
  of the parent's head where its whole body fits on empty cells; a parent
  with none is refused and tries again next tick
  (`CreatureStats::births_denied_no_space` counts the tries). By default
  since 2026-09-30 (`PIXEL_PHYSICS_BUD_STACK`, `off` for the old birth, or
  `World::bud_stack`), above a stack cap of 1, a refused birth takes a second pass in which the child may stand on
  nestmates as a rider (`place_creature`'s `kin`;
  `CreatureStats::births_on_kin`); a `Segmented` child still needs empty
  ground.
  **Who in a colony may bud** is `PIXEL_PHYSICS_BREEDING` (§12), read once
  per process and `graded` unless set (since 2026-10-02; `individual` is no
  suppression, every animal breeding on its own account). An animal is a **breeder** once it has budded
  (`OrganismState::children > 0`; nobody is one at founding). Under `queen`
  nobody buds while another living member of its colony is a breeder; under
  `graded` the bar is multiplied by up to 1.25 near one (`suppress_bar`,
  `GRADED_MAX_SUPPRESSION`; `PIXEL_PHYSICS_BREEDING_MAX` overrides it). The
  scaling is applied to the composed bar, after the affordability check, and
  a colony with no other living breeder is not suppressed at all, so a
  colony whose breeder dies resumes.
- **Brood** (on by default since 2026-10-02; `PIXEL_PHYSICS_BROOD=off` or `World::brood` for budding,
  `brood.rs`): with it on, the same `try_bud` lays an egg instead of
  budding an adult. The bar is the brood block's `lay_at` (1,100 J, scaled
  by the same heritable multiplier) and the egg costs `egg_cost` (120 J),
  topped up from food in reach if the bank falls short. The egg is one
  organism-owned `brood` powder cell on the brood pile when laying only at
  the nest (`pile_site`, above), else on an empty cell beside the head
  (`lay_egg`, out to `PIXEL_PHYSICS_LAY_REACH` rings, default 1); it is a
  separate organism, not counted as a live ant (`live_organism_ids` skips
  brood, `live_brood_ids` lists it). Its stages run in `brood_tick`: egg for
  `egg_frames` (250); larva until its bank reaches an adult's birth cost
  (`target`), paying `larva_upkeep` a frame, taking one bite a larva tick
  (60 frames) of food beside it (`provisions_in_reach`), and fed by touch
  (`nurse`: the richest grown nestmate on one of its eight neighbours gives
  a quarter of what it holds above `start_energy`, capped at the need;
  `PIXEL_PHYSICS_NURSE=off` removes it, and `workers` (built 2026-10-08,
  off) lets only nest workers give, so a rich forager at the door is not
  drained; nothing brings a nurse there unless
  `PIXEL_PHYSICS_NURSE_SEEK` is on, §6d). The crop comes before the bank
  (`crop_feed`, `PIXEL_PHYSICS_CROP_NURSE`, `touch` by default): the
  touching kin with the most food in its crop, over its own `start_energy`,
  gives the shortfall out of the cell it is on and no further, the larva
  keeping what its own gut would (`creature::yield_of`: digestion's quality
  and overhead), booked as a harvest of that food; the crop gives up face
  value as its own chewing would. Under `PIXEL_PHYSICS_NURSE_STAY` (off by default) foragers hand
  their crops to nest workers at the door and down the nest, and those
  nurses carry them to hungry larvae (§6d); a nest worker that feeds a
  larva, from its crop or its bank, stays one 2000 frames longer. Pupa for `pupa_frames` (250), then
  it hatches. A hatchling is laid on the pupa's cell or the nearest open
  cell out to 3 rings, then the same rings again standing on a nestmate
  when `bud_stack_of` allows (`place_hatchling`, `Origin::Hatch`); a pupa
  with no room tries again 60 frames later. The birth (counter, Born log,
  generation, line population) is booked at hatching, not laying. A larva
  whose bank reaches 0 dies as a corpse cell holding what it had left
  (`larva_starves`); an egg whose cell is destroyed is lost. A larva reads
  as needy kin against its target (`kin_deficit`), so a brain's `Share` can
  feed it too; eggs and pupae are never fed. `digbox` prints the counts on
  its `BROOD` line.

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
| `PIXEL_PHYSICS_STACK_DEPTH` | 4 (since 2026-09-30) | nestmates are enterable up to the cap; `1`: not at all, the engine before stacking |
| `PIXEL_PHYSICS_DEPOSIT_AT` | head | `vacated` |
| `PIXEL_PHYSICS_TRAIL_READ` | here-vs-front | `fwd`: trail A read as far-minus-near at 6 and 12 cells |
| `PIXEL_PHYSICS_SENSOR_PROJECT` | on | `off`: no row projection; `none`: also no honesty gate |
| `PIXEL_PHYSICS_A_RHO` | 0.0 | trail A's fade rate |
| `PIXEL_PHYSICS_B_RHO` | 0.03 everywhere (`LAB_B_RHO = DECAY_RHO`) | trail B's fade rate per pass; `0.005` is the lasting trail the owner played 2026-10-04, which cost the goal box a fifth of its food (`pheromone::b_rho`) |
| `PIXEL_PHYSICS_B_DIFFUSE` | 0.05 in the lab, 0.25 elsewhere | trail B's blend toward its 3x3 mean per pass |
| `PIXEL_PHYSICS_NEST_REACH` | r1 | `rN`: nest contact within radius N; `body`: any body cell |
| `PIXEL_PHYSICS_NEST_HOME` | `dug`: the dug nest joined to the door (§ on `adjacent_nest`) | `material` (or `off`): nest material only, the door strip; `shaft`: a head within one cell of the founding cut (dug by `PIXEL_PHYSICS_NEST_SHAFT=<rows>`, `_NEST_SHAFT_WIDTH=<cells>`, lined) also reads `AtNest`; `mouth`: only within one cell of its top two rows (§8) |
| `PIXEL_PHYSICS_LAB_ROOM` | on | `off`: at-nest `Crowding` falls back to local density |
| `PIXEL_PHYSICS_TROPHALLAXIS` | on | `off` |
| `PIXEL_PHYSICS_DROP_REACH` | through bodies | `adjacent`: a food drop looks only at the 8 neighbours |
| `PIXEL_PHYSICS_BUD_SITE` | nest | a species with a nest material lays or buds only at its nest (§9), in the lab box too since 2026-10-03; `anywhere` restores the old rule |
| `PIXEL_PHYSICS_EGG_DOOR` | off | where an egg is never put down (§9, `brood::EggBar`): `door` is a nest's way in, the cells a food drop keeps clear; `cut` is the whole founding cut too (no egg when nothing dug beyond it is in reach); `deep` takes the founding cut only when nothing dug is in reach; `off` is anywhere |
| `PIXEL_PHYSICS_BROOD_CARRY` | off | at a brood item's tick a touching nestmate with free jaws moves it, within that many steps, out of a cell the egg rule refuses, onto home, or next to more brood (`brood::carry`, counted in `CreatureStats::brood_carried`); `on` is a reach of 3 |
| `PIXEL_PHYSICS_BROOD_SPREAD` | off | `on` (a crowd of 6) or a crowd of 3 or more: at a brood item's tick, an item with that many brood in the 24 cells round it and a touching nestmate with free jaws is carried 5-10 steps to a home cell with at most crowd - 2 brood round it and no loose food within 3 (§9, `brood::spread`, counted in `CreatureStats::brood_spread`) |
| `PIXEL_PHYSICS_CROP_NURSE` | `touch` | `touch`: a hungry larva is fed first from the crop of a fed kin carrier touching it (§9, `brood::crop_feed`, counted in `CreatureStats::larva_ticks_crop_fed` and `brood_crop_fed_j`); `on`: that, and a carrier with its head inside the nest walks up larva scent (§6d); `off`: neither; `World::crop_nurse` for one world |
| `PIXEL_PHYSICS_NURSE_SEEK` | off | `on` or `workers`: a fed, empty-jawed nest worker inside the nest walks up the hunger-weighted scent of its colony's larvae (§6d, `brood::larva_scent`, counted in `CreatureStats::nurse_seeks`); `all`: any such ant; a number over 0 is the workers' gain (1 for `on`); `World::nurse_seek` for one world |
| `PIXEL_PHYSICS_EGG_PILE` | on, reach 4 (since 2026-10-03) | acting only when laying only at the nest: an egg goes onto an empty home cell up to that many steps from the layer's head, through nestmates, nearest the brood already there (§9, `brood::pile_site`); `off` (or 0) is the egg beside the head and "at the nest" read off the head; an integer sets the reach |
| `PIXEL_PHYSICS_HOME_REAIM` | off | `loose` (or `on`): every 16th tick, an animal whose homing anchor (`forage_anchor`) stands in ground, water or a plant has it moved to the nearest empty home cell within 12 of it (`home_reaim`, counted in `CreatureStats::home_reaims`); `strict` also moves it off an animal or loose food. Off because it moved nothing on the lab nest (`dead-ends.md`) |
| `PIXEL_PHYSICS_BUD_STORE` | off | `on`: a nesting species buds only at its storeroom, and food in the founding cut and its storeroom pays the whole birth (`bud_from_store`, `provisions_in_store`, §9). `bank`: the same place, but the parent's bank counts as at the door and the store tops it up from food over `BUD_RESERVE` (`bud_store_counts_bank`) |
| `PIXEL_PHYSICS_BUD_RESERVE` | 0 | `<J>`: under `BUD_STORE=on`, a birth's bar is its price plus `<J>`, so the store must still hold that much after it; under `BUD_STORE=bank`, the store's food counts only above `<J>` (`bud_store_reserve`) |
| `PIXEL_PHYSICS_BUD_STACK` | on (since 2026-09-30) | above a stack cap of 1, a birth with no free cell beside the parent stands on nestmates (§9); `off`: it is refused and tries again |
| `PIXEL_PHYSICS_BREEDING` | `graded` (since 2026-10-02) | `individual`: no suppression, the ant before. `queen`: while any other living animal of the same colony has budded (`children > 0`), nobody else in it buds; when that breeder dies, the next animal to reach its bar succeeds it. `graded`: the bar is multiplied by `1 + (GRADED_MAX_SUPPRESSION - 1)(1 - d/r)` for `d` the distance to the colony's nearest other breeder, so 1.25 beside one (`PIXEL_PHYSICS_BREEDING_MAX` overrides it), falling linearly to 1 at `r` = `PIXEL_PHYSICS_BREEDING_RADIUS` (24) cells and beyond (`suppress_bar`, `graded_suppression_factor`). Anything else reads as `graded`. `PIXEL_PHYSICS_BREEDER_INDEX=scan` replaces the per-colony breeder index with a scan of every organism, as the control for the lookup (§9) |
| `PIXEL_PHYSICS_CHOOSER` | trailaway | For species with a nest. `off`: the walk of §6a–§6c; `on`: the chooser's first layer only (§6d items 1–5); `nopatience`: the same with patience held at 1; `trail`: the chooser reading the trail where it would step, with the throttle retired, and no away term |
| `SPOIL_IS_CARGO` | on | `0`: spoil no longer counts toward `Carrying` |
| `PIXEL_PHYSICS_DIG_SPOIL` | kept | `destroy`: dug cells vanish |
| `PIXEL_PHYSICS_BURROW_LINING` | on | `off`: no `packedsoil` lining |
| `PIXEL_PHYSICS_SPOIL_PACKS` | off | `on`: the lining packs spoil into wall too, so an undermined heap can hang (§5) |
| `PIXEL_PHYSICS_SPOIL_FOOTING` | filled | `ground`: a pellet is put down only where the cell beneath is ground, never on an animal or over a hole (§5) |
| `PIXEL_PHYSICS_FOOD_DOOR` | `on` | `off`: food may be put down in a nest's door, the ant before 2026-10-03. On: no food drop counts a cell within the door clearance of a founding shaft (6 cells at the shipped allele of `TRAIT_DOOR_CLEAR`, from that many rows over the mouth down to the foot) as room; the chamber and side room still take food (§5 step 4; `World::food_door` for one world) |
| `PIXEL_PHYSICS_DOOR_REOPEN` | `on` | `off`: the heap cue holds at a plugged door as everywhere else, the ant before 2026-10-03. On: a cut into a founding cut meets the cue only to the ant's `TRAIT_DOOR_CUE` weight, 0 at the shipped allele (§5 step 6; `World::door_reopen` for one world) |
| `PIXEL_PHYSICS_DOOR_LOOSE` | `on` | `off`: the lining tamps a nest's doorway as any other neighbour, the ant before 2026-10-04. On: a cell in a founding shaft's own columns from 8 rows over its mouth to the mouth's last row is never packed (§5 step 6; `World::door_loose` for one world) |
| `PIXEL_PHYSICS_SPOIL_CUE` | `on` (K 5, floor 0) | `off`: no heap cue, the ant before 2026-09-28; `K[,floor]` sets the dials. The cue: a dig that would open the ground to the sky, from the surface or from a tunnel breaking out, goes ahead only in proportion to the pellets beside its target (§5 step 6); `World::spoil_cue` for one world |
| `PIXEL_PHYSICS_NEST_SHAFT` | 6 | `off` (or `0`): founding paints only and digs nothing, the ant before 2026-09-28; `<rows>`: a deeper or shallower founding shaft (§8); `_NEST_SHAFT_WIDTH=<cells>` its width (2); `World::nest_shaft` for one world |
| `CROSS_TRUNK`, `TISSUE_PARTING` | on | `0` |
| `PIXEL_PHYSICS_HOME_PAST_TISSUE` | `on` | `off`: a plant grown into the dug nest is a wall to the dug home, as before 2026-10-03. On, a dug cell holding living tissue a body parts or crosses is open to `World::step_nest_dug` (`body_gets_through_tissue`; `World::home_past_tissue` for one world) |
| `PIXEL_PHYSICS_PUSH_PAST` | `crumbs,brood` | `off`: neither crumbs nor brood can be walked through, the ant before 2026-10-03; `crumbs` or `brood` alone. On, bodies part loose crumbs and brood as they part foliage, the cell coming back exactly as it was when the body leaves, and the dug home reads past brood (§2; `World::push_past` for one world) |
| `PIXEL_PHYSICS_DIGEST` | continuous | `lump`: pays out per whole cell |
| `PIXEL_PHYSICS_LOAD_BY` | joules | `cells`: a load weighs the cells in the crop, not its worth ÷ 480 (§9) |
| `PIXEL_PHYSICS_NEST_DOOR` | 2 | `off`: founding paints the strip of up to 53 columns and every founder's home is its spawn cell, the ant before 2026-09-29; `<d>`: a door of `2d + 1` columns, every founder's home the door (§8); a value it cannot read is read as unset; `World::nest_door` for one world |
| `PIXEL_PHYSICS_NEST_DOOR_FOUNDERS` | spread | `pile`: under the door, founders start heaped on it instead of spread along the ground (§8) |
| `PIXEL_PHYSICS_SCOUT` | 2 | `<gain>`: under `trailaway`, a hungry empty ant off a route runs out from home and back (§6d); `0` turns it off; `World::scout` for one world |
| `PIXEL_PHYSICS_HUNGRY_HOME` | off | `on`/`refed` or `tether`: an empty ant too hungry to be out is pulled home to its nest's larder (§6d, §8); `World::hungry_home` for one world |
| `PIXEL_PHYSICS_FORAGE_DRIVE` | `met` | `off`, `hunger`, `larder`, `returns`, `always` or `met`, then optionally `,nopace`, `,keep` and `,fed` (only foragers at or above `start_energy`): a fed forager goes out when its nest needs food (§6d), and with `,keep` leaves the store at home (§5); `World::forage_drive` for one world |
| `PIXEL_PHYSICS_PACKED_LUNCH` | on | `off`: a crop filled only at home counts as a load, so the forage drive does not reach its carrier (§6d); `World::packed_lunch` for one world |
| `PIXEL_PHYSICS_TRIP_REACH` | on (16) | `off`: a pickup away from home marks a trip once the ant has been `FORAGE_TRIP_MIN` cells from its last nest contact, wherever the food lay; on, the food must also be living tissue or loose food more than the reach (authored cells, scaled; an integer sets it) from every nest's door (§6d); `World::trip_reach` for one world |
| `PIXEL_PHYSICS_RETURN_WINDOW` | 1400 | `<frames>`: the `returns` drive's window (§6d) |
| `PIXEL_PHYSICS_LEAN_FORAGE` | `on` (since 2026-10-03) | below `line` (default 50%) of its `start_energy` an ant does not dig (`nodig`, §5 step 6), puts a pellet down beside it (`drop`, §5's walked cycle) and at its door goes out on its own hunger (`out`, §6d); `off` is the ant before it; a comma list of `nodig`, `drop`, `out`, `line=<pct>` takes what it names; `World::lean_forage` for one world |
| `PIXEL_PHYSICS_FORAGER_NODIG` | off (built 2026-10-08) | `on` or a line 0-1: an ant whose forage drive is at or over the line (0.5 for `on`) reads its `Dig` urge as 0, except shut in (§5 step 6, `forager_nodig`, counted in `CreatureStats::forager_digs_skipped`) |
| `PIXEL_PHYSICS_LAY_IN` | off (built 2026-10-08) | `on`: a ready layer beside home with no egg site in reach is pulled along the nest's way in away from the door (§9, `lay_in_target`, trace `walk in to lay`) |
| `PIXEL_PHYSICS_FORAGE_THROTTLE` | `on` (since 2026-10-02) | near its door a forager's outward want is the colony's, not its hunger, and is carried on the excursion (§6d); `off` is the ant before it; parts `patrol=<p>` (default 1), `reach=<cells>` (default 16), `noscent`, `hold`; `World::forage_throttle` for one world |
| `PIXEL_PHYSICS_SHARE_TOPUP` | off | `on`: a share goes first to the neediest leaver beside the donor, at `frac=<f>` of the difference (default 0.5) (§5); `World::share_topup` for one world |
| `PIXEL_PHYSICS_FOOD_TRAIL` | `lay,read,giveup,window=700,noreturn` (since 2026-10-01; `lay` alone 09-30) | the food trail's recipe (`FoodTrail`): `lay` lays trail B only on a trip load (§7), `off` is the ant before 2026-09-30 (every ant with food in its crop lays), `t=<ticks>` adds an odometer; `giveup` lets a given-up scout go and bounds a scout at a walked trail's end (§6d), and `noreturn` lifts that bound past a scout's point of no return; `read` turns an empty ant at the door toward the food side (§6d), with `gain=` its gain (default 6) and `window=<frames>` its stale-pile window (700 when unset, `READ_WINDOW_SHIPPED`; 0 is `return_window()`); `reach=2\|6` and `follow=all` parse and do nothing yet; `on` is all four parts (`window=700` included); `World::food_trail` for one world |
| `PIXEL_PHYSICS_DROP_SIDE` | `even` | `west`: every food drop scans its neighbours north-west first, the ant before 2026-09-30 (`food_drop_order`, §5); `east`: always north-east first, its mirror |
| `PIXEL_PHYSICS_BROOD` | on (since 2026-10-02) | births are eggs that grow through larva and pupa (§9); `off` is budding, the ant before; `World::brood` for one world. `PIXEL_PHYSICS_NURSE=off` stops nestmates feeding larvae by touch, `workers` (2026-10-08, off) leaves it to nest workers; `PIXEL_PHYSICS_LAY_AT`, `PIXEL_PHYSICS_EGG_COST`, `PIXEL_PHYSICS_BROOD_FRAMES=<egg>,<pupa>` and `PIXEL_PHYSICS_LAY_REACH` override the brood block for a sweep |
| `PIXEL_PHYSICS_BIRTH_HEADING` | `outward` | `east`: every founder and every bud is born facing east, the ant before 2026-09-30. `outward`: a founder faces away from its colony's cursor, and a bud faces the way its body was laid (`birth_heading_outward`) |
| `World::mute_emit_b` | `false` | harness-only, set by `trailfollow`'s `mute` arm and by no game: every newborn's `EmitB` wiring is re-zeroed after its birth mutation (`silence_emit_b`), so a colony whose founders were silenced stays silent across births |
| `PIXEL_PHYSICS_STORE_LUNCH` | off | `on`: a cell taken before the ant has been `FORAGE_TRIP_MIN` (8) cells from its last nest contact counts as taken at home for the packed lunch, wherever it stood; off, a crop is a lunch only while every cell in it was taken with the head beside nest material (§6d); `World::store_lunch` for one world |
| `PIXEL_PHYSICS_HAUL_BITE` | on | `off`: an animal holding spoil cannot swallow or load food; `fed`: only one at or above `start_energy` (§9) |
| `PIXEL_PHYSICS_BIRTH_PRICE` | `guaranteed` | `face`: a birth counts a bare seed in reach at its full worth, though a bite that spares it pays a quarter, so the top-up can leave the parent overdrawn (§9); `World::birth_price` for one world |
| `PIXEL_PHYSICS_CARRY_PATIENCE` | `pickup` | `off`: a carry's home patience restarts only at the first pickup, not at every one (§6d step 5, bug Z35); `World::carry_patience` for one world |
| `PIXEL_PHYSICS_LOAD_SCALE` | 1.0 | `<f>`: every food load weighs `f` times as much again, on top of the species' `food_weight` (§9) |
| `PIXEL_PHYSICS_STOREROOM` | `on,caste=4,workerhome,pile` | `pile`: no room is carried to; a nest worker picks loose food up with probability `(0.1/(0.1+f))^2` and drops it with `(f/(0.3+f))^2`, `f` the share of the 24 cells round it holding loose food (Deneubourg 1991; `pile_pick_p`, `pile_drop_p`, `pile_food_share`), or where its patience runs out; under `keep`, a fed ant leaves food with two or more food cells beside it at home (`store_kept`). `off`: no storeroom and no nest workers, the ant before 2026-09-29. Otherwise parts, comma-joined, naming the whole rule (so `on` alone is the carry and nothing else), and a value it cannot read is read as unset: `on`, a fed ant at home with an empty crop and empty mandibles, on a won `Feed` roll beside loose food more than a cell from its nest's founding chamber, takes the cell whole into its mandibles instead of swallowing it (`store_pickup_ok`); the load is pulled home to the chamber (`home_pull`, `home_target`, `HomeAligned` read as laden), goes down on a `DropSpoil` roll only in or beside the chamber (`store_drop`), is let go where it stands after 48 still decisions or when patience runs out, and its carrier is pulled back up to the mouth (`store_return_target`); a pick-up within a cell of the chamber reads as at home. `home`: the chamber is home to `adjacent_nest` (§8). `once`: one load until the next pick-up away from home. `post`: the load is handed down the open shaft from the mouth instead of walked down (`store_post_site`). `nestbound=<frames>[/<k>]` (bare: 8,000): an ant born in the colony, and with `/<k>` one founder in `k`, is nest-bound for its first `<frames>` (`OrganismState::nest_bound_until`, `is_nest_bound`): the forage drive reads 0 for it, fed it takes no away term and is pulled home when it strays (`home_pull`), and only it carries. `caste=<k>`: one ant in `k`, by id, founders and born, is nest-bound for life. `workerhome`: the founding cut is home to a nest-bound ant only (`nest_within_reach`). `side`: the storeroom is a room cut at founding off one side of the entrance shaft (`SideRoom`), on the side away from the door, joined to the shaft by a passage two rows tall halfway down, its floor a row lower; every rule above reads its rectangle in place of the chamber's (`ShaftFootprint::store_rect`), a carrier in the shaft or the chamber is pulled to the passage's far floor cell and then the room's floor (`store_target`), and `post` does not apply. `keep`: a fed animal's won `Feed` roll on a storeroom cell takes nothing and ends its turn, so only a hungry one eats the store (`store_kept`, counted in `CreatureStats::store_kept`). `keep=<pct>`: `keep`, with "fed" read as at or above `<pct>`% of `start_energy` (100 is the bare `keep`). `stock=<pct>`: a nest worker stocks at or above `<pct>`% of `start_energy` (100 is the shipped "fed"). `harvest`: a forager with food in its crop that is not a packed lunch is routed into the storeroom by the store load's path while the room has space (`harvest_target`, in `home_target` and `home_pull`), puts nothing down until beside the room, and there puts a cell on the room's floor at 0.25 a tick unless under 40% of `start_energy` (`harvest_drop`, `HARVEST_DROP_P`, `HARVEST_HUNGRY`); each cell sets `store_return`, so it is pulled back up to the mouth after (`CreatureStats::harvest_stored`, `harvest_held`). `World::storeroom` for one world |
| `PIXEL_PHYSICS_NEST_SHAFT_OFFSET` | 0 | `<cells>`: the founding shaft is cut that many columns from the founding point (negative is west), so a door (`PIXEL_PHYSICS_NEST_DOOR`) has the mouth beside it (§8) |
| `PIXEL_PHYSICS_DIG_DOWN` | `1.0,enclosed` | `off` (or `0`): no turn, the ant before 2026-09-28; `<w>`: the turn with chance `w` for any digger; `<w>,enclosed`: only an enclosed one (§5 step 6) |
| `PIXEL_PHYSICS_SPOIL_HAUL`, `_SPOIL_DROP_COVER`, `_TRAFFIC_DEFER`, `_COLONY_SPACING` | unset | haulage re-roll to the nest door, spoil held under cover, jam deferral length, founder spacing |
| `PIXEL_PHYSICS_SPOIL_OUT` | on (since 2026-09-30) | the excavation cycle walked (`SpoilOut`): parts, comma-joined, `on` for all four, `off` for the lift. `haul`: a pellet carrier is pulled to the door over the mouth (`spoil_haul_target`); `pace`: at the laden pace, `HomeAligned` read against that target (`spoil_pace_target`); `keep`: inside the nest (`inside_nest`: under cover, or in the founding cut) the pellet is not put down while patience lasts, and is never lifted from there (`spoil_kept_inside`, `spoil_kept_no_lift`); `back`: a digger not hungry walks back to the cell it cut once its pellet is down (`OrganismState::dig_return`, `dig_return_target`) |
| `PIXEL_PHYSICS_SPOIL_LIFT` | `climb` | where a pellet with no cell beside its carrier goes: `climb` up the carrier's column as far as it could have walked (`lift_reach`); `out` through the passages to the nearest cell in the open that holds a pellet (`lift_out`, `spoil_lifted_out`); `none`, `dig`, `unbounded` the older reaches |
| `PIXEL_PHYSICS_SPOIL_HOLD` | `12`, **acting only under the walked cycle's `keep`** | `<cells>`, `on` or `off` (`0`): a carrier inside the nest whose haul patience has run out keeps its pellet while its head is within that Chebyshev distance of the haul's target (`spoil_haul_target`), instead of laying it beside itself (`spoil_hold_of`, counted in `spoil_held_near_door`); `World::spoil_hold` for one world |
| `PIXEL_PHYSICS_HOME_SEARCH` | off | `on`: a food carrier whose home patience has run out and that has strayed past its search reach (8 cells from where it last got nearer home, doubling each loop) starts its carry over from where it stands (§6d step 5; `World::home_search` for one world, `CreatureStats::home_searches`). Built 2026-10-04; off because colonies came out smaller on 4 of 4 seeds |
| `PIXEL_PHYSICS_CARRY_HOME` | off | `on`, `off`, or a comma list of `fill` and `turn`, acting only on a carrier whose crop holds a trip's load. `fill`: it stands beside food beyond the trip reach of every door until its crop is full (§6d item 4). `turn`: its trail hold counts presence above the weakest option's, so even scent holds nothing (§6d stage 2). `World::carry_home` for one world; `CreatureStats::carry_fills`, `carry_turns`. Built 2026-10-06 for the carriers that lose the pull home on the mound; off because one colony of four still falls late with it on (seed 2, 650 ants at 200k to 150 at 300k) and the other three end smaller |
| `PIXEL_PHYSICS_HUNGRY_OUT` | on (since 2026-10-04) | an ant under its grant carrying nothing, inside its nest's way in, is pulled towards the door along the passages at the scout's pull out (§6d; `World::hungry_out` for one world); `off`: no such pull; `lean` (built 2026-10-07, off): only an ant below `LEAN_FORAGE`'s line, or shut in, is pulled (`hungry_out_lean`) |
| `PIXEL_PHYSICS_MOUND_OUT` | `dig` (since 2026-10-06; `way` off) | `on`, `off`, or a comma list of `way` and `dig`, acting in the spoil mound above the door. `way`: a hungry ant there that `HUNGRY_OUT` does not pull is pulled along the mound's own way out (`build_mound_way`, from the open air through the covered cells at or above the founding ground) at the same gain (§6d). `dig`: a lean ant shut in the mound, on neither way, keeps its dig roll (§5 step 6). `World::mound_out` for one world; `CreatureStats::mound_out_pulls`, `mound_digs_let`. `dig` ships on: over 12 seeds it starved 374 grown ants -> 143 with the colony the same size, by getting shut-in ants out (1.3% -> 0.5% of shut-in spells ended starved); `way` stays off, as it saves none and costs trip food (-11%) |
| `PIXEL_PHYSICS_NEEDS_FIRST` | off (built 2026-10-06, not yet scored) | `on`, `off`, or a comma list of eight parts: needs outrank the rules that hold an ant to a job. The jaws (soil pellet or store load, the one `spoil` field that 33 rules read): `hungry` (no crop food, and under `LeanForage`'s line, or under its grant and shut in: puts the load down ahead of the keep rule and `SoilWay`'s `lean`, and gives up the walk back to its face or up from the store; the start grant was measured twice and is not where hunger starts in this game, since the founders land at half of it), `laden` (crop food it could feed and a hungry larva beside its head: a soil pellet goes down first), `job` (a fed worker beside home food the storeroom would carry, refused only for its pellet), `pack` (a hungry soil carrier walled in -- every cell beside its head its own body or bare ground, nobody riding in its cells -- cuts the cell ahead, aimed one octant towards up, steps into it and packs its old pellet into the cell its tail left: `pack_behind`, `walled_in`). A ninth part, `backfill`, is not in `on` and must be named (`on,backfill`): below the founding ground (`below_founding_ground`) what `pack` leaves is the pellet's packed form (`packs_into`), so a later cut under the column's foot cannot drain it and the loose ground beside it into a room -- the east opening traced on heap-90 seeds 3 and 7 (2026-10-07); in the spoil mound it stays the pellet. A load goes down on a `need_drop_site` cell: `lean_drop_site`'s footing, outside the doorway (the founding shaft's columns from 8 rows over its mouth to its foot), and never where filling it would part the open cells round it (`closes_a_way`); with no such cell it is kept (`needs_no_site`). The blocks: `throttle` (in the door's throttle zone hunger counts against the colony's want, the larger winning), and for an ant `shut_in` (under cover with no walkable way to open air, `build_out_way`'s fill over mound and nest alike) `weak` (a lean one keeps its dig roll), `breakthrough` (the heap cue stands aside) and `door` (`FaceTrip`'s `only` stands aside). `World::needs_first` for one world; `CreatureStats::needs_down`, `needs_packed`, `needs_quit`, `needs_weak_digs`, `needs_cue_waived`, `needs_face_waived`, `needs_throttle_lifted`, `needs_roof_refused` (the roof rule is left in place and counted), `needs_no_site`. Inventory: `/mnt/project-files/nest-race/lane3/soil-holder-rules-2026-10-06.md` |
| `PIXEL_PHYSICS_WAY_FOOT` | off (built 2026-10-07) | `on` (`way,store` at `k=4`), `off`, or a comma list of `way`, `store` and `k=N`: the nest's way is costed again (`foot_field`) so a step into a cell whose only footing is another ant costs `k`, so the walk out (`way_out_from`: the hungry walk out and the soil's way out) and, under `store`, the walk to the store follow the walls rather than the backs of the crowd. It prefers, never forbids; `dist` is untouched. Heap 90, 12 seeds on the stack: starved lower 12 of 12 (`Reports/handoff/nest-race/way-foot-twelve-seeds-2026-10-07.md`) |
| `PIXEL_PHYSICS_DOOR_COLUMN` | off (built 2026-10-06; **a stopgap**: a fixed column, not something an ant senses) | `on`, `off`, or a comma list of `drop` and `clear`, over the founding shaft's columns and one either side above its mouth: `drop` no pellet is set down there (`in_door_column`); `clear` soil that gets there is removed every frame (`clear_door_column`) |
| `PIXEL_PHYSICS_LAY_BAR` | unset (built 2026-10-07) | `body`: a laying species clears its egg bar from its own energy alone, food in reach not counted (`lay_bar_body`, `try_bud`); unset is the reach rule, bit-identical |
| `PIXEL_PHYSICS_MOUND_DIG` | off (built 2026-10-08; **measured harmful**) | `on`, or a comma list of `down`, `cue` and `roof`: the spoil mound (over the founding surface, within 40 columns of the site) is not a dig face (`MoundDig`, `in_mound`, `mound_roof_refuses`; counter `digs_refused_mound`). Every part that stops the mound digging costs the colony a third to a half, because the hollow mound is its way to the food (`Reports/mound-diggers-2026-10-08/README.md`) |
| `PIXEL_PHYSICS_FEED_FIRST` | off (built 2026-10-08; **inert on the playtest stack**) | `on`/`hold`, `reach=<cells>` (6), `frac=<percent>` (50): an ant that has cleared every other bar to lay does not lay while a larva of its colony within `reach` has a bank under `frac` of `egg_cost` (`FeedFirst`, `try_bud`, `brood::starving_larva_near`; counter `feed_first_held`). Measured on `steady_income` with the playtest switches, 4 seeds: holds on 1 seed of 4 at reach 6, none at reach 2, because the larvae that starve have fallen a median 38-40 rows down the brood column, 31 cells from the nearest egg laid while they starve (`Reports/feed-first-2026-10-08/README.md`) |
| `PIXEL_PHYSICS_WAY_GAPS` | on (since 2026-10-05) | the nest's way in (`NestWay`) closes its two gaps (§6d, `WayGaps`): `below` counts every cell under the founding ground as inside, `brood` counts brood and crumbs the walk parts as open; `on` is both, a comma list takes what it names; `World::way_gaps` for one world; `off` is the ant before it. Built 2026-10-05: starvers in the nest fall to nearly none and alone the colony is about a fifth smaller (fewer eggs: laying waits on brood falling from the pile, `WayGaps`); with `SOIL_WAY` and `FACE_TRIP`, on with it the same day, 4-12% smaller, adults starved 27-67 -> 1-8 (deep trace, seeds 1-4, 100-200k) |
| `PIXEL_PHYSICS_FACE_TRIP` | on (since 2026-10-05) | a digger keeps its face through the trip out and back (§5 step 5, `FaceTrip`): `below`, `door`, `food`, `only`, `stay`, comma-joined, `on` for all five; `World::face_trip` for one world; `off` is the ant before it, and it reads as off while `WAY_GAPS`' `brood` is off (`gaps_hold`). Built 2026-10-05: next cut at the face after a nest cut 0-4% -> 35-59% with `SOIL_WAY` and `WAY_GAPS` on (seeds 1-4, new ground dug +18-33%, colony no smaller), 13-15% -> 30-43% on today's nest, where starved deaths rise on 5 of 6 runs; with `WAY_GAPS` alone 18-27% -> 57-61%, starved 0-5, colony about today's size. On with `SOIL_WAY` and `WAY_GAPS` (it needs the gaps: alone 94-282 starve deep in the nest): next cut within two cells 12-15% -> 36-48%, no walk back ends hungry (26-40%) |
| `PIXEL_PHYSICS_CROP_DOWN` | off | a fed carrier brings crop food down to the brood (§6d, `CropDown`): `hold` keeps its last crop cell (`keepN`: N cells, `keep0` every cell) until more than 5 rows under the founding ground, `scent` takes its pull home away below the founding ground so it walks up the larva scent; `on` is `hold,scent`, `all` is both keeping every cell; `World::crop_down` for one world, `CreatureStats::crop_down_holds`. Built 2026-10-05: larvae starved per egg laid 16.8/12.3/16.0/16.0/13.0/15.8% -> 9.4/8.8/15.0/11.1/15.2/13.0% (`on`, seeds 1-6), larva food from crops 0-4.5k -> 25-81k J; colony 0-17% smaller on 5 of 6 (`all`: 13-37%) |
| `PIXEL_PHYSICS_NURSE_STAY` | off (built 2026-10-05; `on` is `relay,nurse,down,stay,pace`) | foragers hand their crops to nurses, who take them to the brood (§6d, `NurseStay`): `relay`, a forager at its nest hands its whole crop to a touching nest worker with an empty crop; `down`, to the deepest touching ant below the founding ground and its own head, which becomes a nest worker for 2000 frames; `nurse`, a nest worker with crop food keeps it, has no pull home underground and walks the larva scent, and is pulled to 3 rows under the door from above; `stay` (`stayN`), a nest worker that feeds a larva stays one 2000 (N) frames longer; `pace` (2026-10-06), a nurse's `HomeAligned` reads the way to the larvae, not its forage anchor (idling inside 73-80% -> 21-38%, but `on` then killed 4 of 4 colonies, §6d); `off` is the ant before 2026-10-05; `World::nurse_stay` for one world. Built 2026-10-05: larvae starved per egg laid 20.2/20.5/21.0/19.1% -> 10.7/14.8/11.1/13.5% (seeds 1-4), larva food from crops ~0 -> 1.0-1.4 MJ; colony 6-33% smaller 40k-150k, the same 200k-300k; grown foragers starving above ground 27-38 -> 66-342 a run. Off since it ships: with `SOIL_WAY`, `WAY_GAPS` and `FACE_TRIP` on, live ants 100k-300k 128-293 against 529-573 with it off, grown ants starved 477-1070 against 7-12, seed 2 died out (§6d) |
| `PIXEL_PHYSICS_NEST_STORE` | off (built 2026-10-06; `on` is `carry,eat,keep,home,larder`) | a store inside the nest (§6d, `NestStore`): loose food beside the nest's way at least `depth=N` (20) cells in. `carry`, a nest worker takes loose food at home in its jaws and walks it down the way to the store; `eat`, a hungry ant inside goes to the store when it holds 8 cells, and `HUNGRY_OUT` waits while it can feed it; `keep`, a fed ant does not eat there; `home`, a fed idle nest worker is pulled deeper; `larder`, the larder need reads the store; `pick=N`, door food within N cells is taken from the doorstep (0); `jaws`, a full crop does not stop the pick-up; `fetch`, idle fed nest workers are pulled to door food; `World::nest_store` for one world. Measured 2026-10-06 (seeds 1-4, 150k, `NEST_REST=off`): 190-350 loads a run, all eaten on arrival, the store held 0-1 cells; 77% of fed nest workers at door food hold a soil pellet, so neither `jaws` nor `fetch` moves it, and `fetch` takes nest workers out of the nest (8% -> 6% inside) Parts added 2026-10-06/07, none in `on`, all off: `pick=<cols>` the doorstep reach of the carry's pick-up; `jaws` a worker with a crop still carries in its jaws; `smell=<steps>` the eat pull acts only within that many steps of the store's way; `sated` a fed ant leaves home food for the store; `whole` crumbs are not the store; `meal` a hungry ant keeps and digests what it bit at home (`OrganismState::lunch`, written always); `sky` a carrier's soil goes down under open sky (`ring_target`); `edible` the store counts only food the colony's mouth can take (`store_food`, `diet_yield` at `NestSite::gut`, the seeding ant's expressed gut). The tested stack runs `on,pick=20,jaws,sky,meal,smell=10,edible` (`Reports/handoff/nest-race/`) |
| `PIXEL_PHYSICS_SOIL_WAY` | on (since 2026-10-05) | soil cut in the nest leaves by its way out (§5 step 5, `SoilWay`): `way` pulls a pellet carrier inside along the nest's way in and never lets the pellet go below the founding ground, `lean` keeps a lean carrier's pellet until it is above the founding ground (not until it is out of the spoil mound); `on` is both; `World::soil_way` for one world; `off` is the ant before it, and it reads as off while `WAY_GAPS`' `brood` is off (`gaps_hold`). Built 2026-10-05: put back in the nest 67-75% -> 0%; on the way in with its gaps its starvers rise, so it went on with `WAY_GAPS` (and `FACE_TRIP`): put back 59-68% -> 0% on 4 of 4, re-digs 83-88% -> 15-42% of nest cuts |
| `PIXEL_PHYSICS_NEST_REST` | off (`workers` 2026-10-05 to 2026-10-06; two colonies of four collapse late under it) | `workers`, `on` or `all` (`NestRest`: nest workers; nest workers and ants that have foraged; every ant; `off`: no rest pull): such an ant with nothing to do and more fed than hungry, not sent out by the forage drive, is pulled to its door and then along the passages away from it (§6d, `rest_pull`, `NestWay`, counted in `rest_pulls`); `World::nest_rest` for one world |
| `PIXEL_PHYSICS_SPOIL_RING` | `2,2`, **acting only under the walked cycle** (`SPOIL_OUT`, on by default since 2026-09-30; with it `off` the lift is untouched) | `<shape>,<scale>` or `off`; `spoil_ring_of` gates it on the walked cycle, and a world's own `World::spoil_ring` overrides both: when a carrier comes out by the door with its pellet (on or above the door's row, nothing overhead: `carry_stage`) it draws a column on its own side, the door's half-width plus one plus a Gamma(shape, scale) draw from the nest's centre (`spoil_ring_column`, `OrganismState::spoil_ring`, its own stream), is pulled to the top of the ground in that column (`ring_target`, climbing any mound; `spoil_haul_target`), and its drop roll is held until its head is that far out (`spoil_ring_holds`; `spoil_ring_drawn`, `spoil_ring_held`). The column is kept under a mound's overhang and let go only back in a tunnel (more than two rows under the door's row with ground overhead, or in the founding cut: `spoil_ring_let_go`); before it has come out, under `keep`, the pellet is held wherever the carrier stands; `World::spoil_ring` for one world |
| `PIXEL_PHYSICS_DIG_ROOF` | `6` (on since 2026-10-02, `DIG_ROOF_SHIPPED`) | `<rows>` or `off`: a cut within `rows` under the nearest nest site's founding surface and outside the door's columns is refused (`dig_roof_of`, `under_roof`, `CreatureStats::digs_refused_roof`); a heap above the surface is never refused, nor anything with no door; `World::dig_roof` for one world. Nest report §25 |
| `PIXEL_PHYSICS_WATER_FOOTING` | on (built 2026-10-04; ships with `DOOR_LOOSE`) | `off`: liquid is neither footing nor enterable, so a pool wider than a body is a wall. On: liquid in the three cells under the head foots a step, so a pool's top is walked across; it never holds a body up in the fall check (`water_footing_of`, `stands_on_water`, `CreatureStats::water_steps`); every walking animal, not only the ant; `World::water_footing` for one world |
| `PIXEL_PHYSICS_MUTATION` | on in the game and the test suite; **off in the measuring examples** (since 2026-10-04) | `off`: every birth inherits its parent exactly -- an ant's brain genome, its traits (scent and tolerance included) and its body plan, and a plant seed's genotype, loci, growth rules and parameters (`mutation_of`, `mutate_newborn`, `plant::bear_seed_at`). A measuring example (`labforage`, `digbox`, `nestdoor`, `trailprofile` and 21 more) calls `mutation_off_for_measuring` first; `on` puts mutation back in one; `World::mutation` for one world. **Lab numbers from a measuring example before 2026-10-04 and after it do not compare**: before, each arm of an A/B also evolved its own way, which the deep trace measured as roughly doubling the seed-to-seed spread. Off, a newborn also carries its parent's own scent, so the per-birth scent drift inside a colony stops; each nest's odour still wanders |
| `PIXEL_PHYSICS_LAB_ANT_GUT` | unset: the founder's own gut (-0.8 for the lab's evolved ant since 2026-10-05; `scene::LAB_ANT_GUT`'s -0.5 for `half-plant`, the lab default 2026-10-04 to 2026-10-05); the outdoor ant keeps `ant.ron`'s 0 | a number from -1 to 1, or `ancestral` for 0: the `gut_bias` a lab box's ants are founded with (`LabBox::build_counted`, before the colonies are placed). At 0 a founder gets a quarter of a plant-class cell (`diet_quality`), which is everything a lab box feeds it; with mutation on, a colony's gut drifted to -0.43..-0.68 by 80-100k frames on all four goal-box seeds and the colony grew only after it had, and with mutation off it never grew (about 35 ants to 200k, 4 of 4). At -0.5 a standing leaf clears `EAT_YIELD_THRESHOLD` (22.5 J against 12), so the ants graze the living plants from the start, and a corpse pays a sixteenth of its worth. The gut stays heritable; the parameters page's `gut_bias` row and a scenario's `settings` still set it. Laid over whichever founder `PIXEL_PHYSICS_LAB_ANT` names, replacing its gut |
| `PIXEL_PHYSICS_LAB_ANT` | `evolved` in a lab box (since 2026-10-05, the owner's ruling; `half-plant` before) | `evolved` founds a lab box's ants at `scene::LAB_ANT_TRAITS`, the body goal-box colonies evolve to with mutation on: gut -0.8, birth grant -0.27, breeding bar -0.14, pace +0.21 (a decision every 5 frames, not 6), ground feel +0.11 (radius 3, not 2), digestion +0.2; `ancestral` is `ant.ron`'s own. Goal box, mutation off, seeds 1-4: the half-plant colony peaks near 100k and falls to 0-129 ants by 200k, the evolved one holds 278-399 to 300k (11 of 12 over seeds 1-12: seed 9 boomed to 767 and died out by 240k; ancestral brain, frozen; the body alone does it). On a planted bed it overgrazes (played bed, 120k, 12 seeds, peak 94 -> 167 ants, 42% fewer plants, starvation up on 12 of 12, 2 of 12 colonies died out); the owner put it in every box anyway and asked for a garden where ants and plants live together, which is its own task. `half-plant` founds the gut-only ant. At -0.8 a 480 J corpse pays 4.8 J, under the 12 J bar, so dead nestmates are not eaten, and a beetle is not prey (`PreyNear` reads 0; `hunting_ground.ron` pins the gut to 0). Before it was the default a scenario took it with six `settings` rows (the constant's doc lists them); on top of the default they count the two species rows twice |
| `PIXEL_PHYSICS_KIN_FOOTING` | on (since 2026-10-02) | `off`: an ant is held up only by ground. On, a body touching a nestmate whose own body touches ground is held too, one level, while it has stood still under `KIN_GRIP_TICKS` (`held_by_kin`, `kin_footing_of`, `CreatureStats::kin_holds`); `World::kin_footing` for one world. Nest report §26 |
| `PIXEL_PHYSICS_NEST_KIN_GATE` | on (since 2026-10-03) | `off`: an ant at any nest site exchanges odour with it, the ant before 2026-10-03. On, at a site seeded by a colony the ant does not descend from, the exchange is skipped (both ways) when the ant's scent is outside its own tolerance radius of the site's odour; its own colony's site is always joined (`nest_kin_gate`, `blend_with_nest`, `NestSite::colony`, `CreatureStats::nest_blends_refused`). §1 step 2 |
| `PIXEL_PHYSICS_TOLERANCE_DRIFT` | slow (since 2026-10-03) | `full`: `TRAIT_TOLERANCE` drifts per birth at the full `scent_drift`, the ant before 2026-10-03. Slow, at `TOLERANCE_DRIFT_SHARE` (a third) of it; the scent signature keeps the full rate. Lines that narrowed at the full rate bit nestmates on the two-colony bed (`trait_width`, `slow_tolerance_drift`) |
| `PIXEL_PHYSICS_DOOR_COLLAR` | off | `on`: before `act`, an ant whose head is within the door's half-width + `COLLAR_REACH` (2) columns of a nest site and from `COLLAR_UP` (6) rows over its founding surface to one under it packs each neighbour on the rim of the opening (at or over the surface, in a column whose cell one row under the surface is ground, beside one where it is open), pellets included when footed (`collar_tamp`, `CreatureStats::collar_packed`); `World::door_collar` for one world |
| `PIXEL_PHYSICS_SPOIL_CREST` | off | `on`: under the ring, a carrier walks on from its drawn column while the ground ahead still rises (at most `CREST_REACH` 8 columns) and drops on the crest (`crest_column`, used by `spoil_haul_target` and `spoil_ring_holds`); `World::spoil_crest` for one world. Nest report §25: alone it lost 2 of 4 food-box colonies |
| `PIXEL_PHYSICS_DIG_FACE` | `workers` | `off`: no turn, the ant before 2026-10-03; `on`: a digger inside the nest whose won roll faces open air or a nestmate turns straight to the nearest octant whose cell it can cut and keep underground (not refused by the roof, not scaled by the heap cue), and cuts it; `workers` (shipped): only a nest-bound ant (§5 step 6; `digs_faced`; `World::dig_face` for one world) |
| `PIXEL_PHYSICS_DIG_WIDEN` | off | `on`: tunnels one body length (two cells) wide. On a won dig roll, a digger whose way ahead is open and whose head stands where its passage is one cell wide (ground above and below, or either side) cuts one of those walls instead of turning down and cutting ahead (`ahead_is_open`, `dig_widen_site`); a digger at a face cuts a shoulder beside the cell ahead on half its rolls (`dig_shoulder_site`), so a gallery advances two cells across. A passage two wide is left alone. Both cuts are ordinary cuts after that: the heap cue and the jaw judge them (`digs_widened`; `World::dig_widen` for one world) |

## 13. Where the implementation lives

- `creature.rs`: `creature_tick`, `sense`, `act`, `step_chain`, `tumble`,
  `usable_headings`, `home_weighted_pick_why`, `trail_sample_point`,
  `fall_if_unsupported`, `commit_step`, `chooser_step`, `home_pull`,
  `trail_presence`, `brain_inputs`, `adjacent_nest`, `line_burrow`, `food_drop_site`, `choose_weighted`, and the decision trace's
  types (`DecisionRow`, `DecisionOutcome`, `HomewardWhy`, `DropWhy`).
- `brain.rs`: `eval_brain`, the `BrainInput` / `BrainOutput` enums.
- `brood.rs`: eggs, larvae and pupae (`lay_egg`, `brood_tick`, `nurse`,
  `hatch`), behind `PIXEL_PHYSICS_BROOD`.
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
  and the head's eight neighbours at the roll: how many were room for food,
  empty and not in a nest's door (`free8`), their materials, and which were the ant's own body or another
  organism; and how far the food went (`drop_reach`: 1 for a neighbour,
  more when handed on through bodies);
- the dig in `act` that same tick (`DigWhy`): how far it got (not reached,
  lean, roll lost, refused by the heap cue, the face trip or the roof,
  nothing the jaw can take, cut), the brain's urge before the lean gate (`dig_p`), the cell it
  was judged on and that cell's material (`dig_at`, `dig_mat`), what turned
  or moved the target first (`dig_flags`: turned down, faced, widened, the
  turn down refused), and the two senses on the dig's wires the row did not
  otherwise carry (`curvature`, `moisture_grad`), so the urge can be rebuilt
  term by term;
- the cone's three scores after the zeroing and the candidate taken;
- under the chooser (§6d), the patience it scored with and the home cosine
  of the heading it picked, and under stage 2 that heading's trail presence
  (`chosen_route`).
- under the chooser, which pull it scored with (`pull_why`, named by
  `PULL_WHY_NAMES`: one of `home_pull`'s branches -- store trip, walk home to
  lay, nest worker leash, spoil haul, the spoil haul's soil way out
  (`SOIL_WAY`), back to the face, hungry home, laden home -- or the hungry
  ant's way out, the rest pull, or none), that pull's
  target (`pull_at`), the home term's weight as scored (`pull_gain`: the
  home gain × the pull's own gain × patience, 0 with no pull) and the
  persistence (`persist`), so every option's score splits into persistence
  × turn preference × trail hold, the side term and the pull.
  `home_pull_why` names the branch by mirroring `home_pull`'s order, read
  only while the trace is on; a branch whose target is not the one
  `home_pull` returned is booked `mismatch`.
- under the chooser, the larva-scent term it scored with (`nurse_w`,
  `nurse_ux`, `nurse_uy`: the term's weight and unit direction, NaN when it
  added none), so an option's score splits with no remainder for a carrier
  steered by the scent (`CROP_DOWN`'s `scent`, `CROP_NURSE=on`, a
  `NURSE_STAY` nurse).
- why a digger's walk back to its face ended on this decision (`trip_end`,
  named by `TRIP_END_NAMES`: arrived, gave up, hungry, food in the crop,
  other, or `mismatch`), from `dig_trip_end_why`, a mirror of
  `dig_trip_over`'s tests in its order read only while the trace is on. A
  cut that replaces or forgets the face is not booked here; it shows as the
  cut.
- the animal's energy in joules (`energy_j`: `energy` is the clamped input),
  the forage drive it felt (`drive`, NaN when off or carrying), and scouting
  as the chooser scored it (`scout_w`, `scout_patience`, `scout_home`).
- what the scout was sent out with (`want`, `zoned`): the want its pull out
  was scaled by as `outward_want` reads it -- its own hunger, or within the
  door's throttle zone the colony's want -- and whether the head stood in
  that zone (NaN and false when the chooser added no scouting term); and why
  `door_read` did or did not add its term (`door_why`, named by
  `DOOR_WHY_NAMES`: not asked, off door, spoil, stale, dark, read) with the
  term as scored, signed + east (`door_f`, NaN unless read). A given-up
  scout, a laden ant and a pulled one are never asked.
- the trail it laid and read: the raw amounts actually deposited on each
  channel (`emit_a_laid`, `emit_b_laid`), the brain's `EmitB` before the cast
  (`emit_b_brain`), the cell laid on (`deposit_at`), and the cargo's age
  (`since_trip`, `OrganismState::since_trip`: ticks since the pickup that set
  `trip_load`, counted beside `since_nest`); and under the chooser, trail B
  one and two cells along each of the eight headings (`b_near`, `b_far`, the
  cells `trail_presence` reads) and at the six-cell sensor point (`b_six`),
  the option mask, whether a crossing forced the pick, the blend `k`, every
  option's score, the heading chosen and whether the trail was read.

**The larva-meal log** rides beside it: off unless a harness sets
`World::feed_log = Some(Vec::new())`, and while it is on every meal a larva
is given pushes one `FeedRow` (`note_feed`): the larva, the cell it lay in,
how it was fed (`FEED_KIND_NAMES`: food in reach it ate, a carrier's crop, a
nestmate's bank, a brain's `Share`), the donor (0 for food it ate) and the
energy it gained. The four kinds are booked where `brood_ate_j`,
`brood_crop_fed_j`, `brood_nursed_j` and `brood_shared_j` are, so the rows
sum to those counters exactly; the counters say how much, the log says
where.

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
    with it on and off, and checks no scored decision's pull is booked
    `mismatch` and the laden branch is exercised (red with the mirror's
    laden target broken);
  - `the_decision_trace_carries_the_nurse_term_it_scored_with` (red with
    the chooser's nurse line removed) and
    `the_trip_end_trace_names_why_the_walk_back_ended` check the two
    columns above on scenes with known answers;
  - `the_feed_trace_changes_nothing_it_logs` feeds one larva every way a
    bed can (food beside it, its parent's crop and bank) and compares the
    state bit for bit with the log on and off (red with `note_feed` taking a
    joule off the larva it logs); the crop and egg-life tests in `brood.rs`
    check each row's larva, cell and donor and that the rows sum to the
    counters;
  - `the_setting_class_reads_the_ground_the_ant_stands_on` checks the
    classifier on three known terrains;
  - `every_traced_decision_agrees_with_the_counters_and_the_positions`
    checks rows against the census, against every counter above and the
    per-verb counters (`moves`, `drops`, `deliveries`, `tumbles_homeward`,
    and the dig's `dig_rolls` and `digs`), and against where the head went;
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
  the draws and reads the climb and the door departures. `deeptrace` writes
  every row of every ant in the goal box, and with `dig=1` a narrow row per
  decision carrying the dig's funnel (`digrows.csv.gz`) and the larva-meal
  log (`feeds.csv`); `scripts/deeptrace_plan.py` reads its `walk=1` runs
  into one table per nest-plan switch.

