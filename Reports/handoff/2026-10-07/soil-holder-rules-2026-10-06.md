# Every rule that changes for an ant holding soil

Nest building (lane 3), 2026-10-06 ~13:40. Step 1 of the caution plan Scott agreed at 13:05 ("soil as the lowest job"). Read from the code at main 61a343a1 (`src/sim/creature.rs`, `src/sim/brood.rs`), every read of `spoil` on a live ant: 36 places in 33 rules, 10 of them traced. **Traced** means read decision by decision in a run, with the run named; **inferred** means read from the code only. "Live" means the rule runs in the shipped game (main with PR 645's `dig` on); "switch only" means it runs only under a switch that is off on main.

## Scott's question: right or wrong?

**Right where it kills, wrong as a general description.** A holder does not stop following rules everywhere: it eats and shares food normally. Traced: 73-85% of pellet holders at the heap carry crop food (1k colony samples, seeds 1-4), and 97.5% of hunger spells that began with a pellet ended fed (seed 4, 190-272k). What breaks is narrower. A pellet turns off **every pull that would take a hungry ant to food or out of a pocket** (rows 1-3), puts the soil's own pull **above** the hungry walk home (row 20), and the pellet can only come down where there is open space with headroom (rows 26-29). In a packed mound none of those has an answer, so the ant walks the soil's way until it starves. 249 of 255 starvers held a pellet in their last 30k frames (seed 4, rest pull, main 33389072).

## A. Needs a holder is skipped for

| # | Rule (file:line) | What a holder loses | Live? | Evidence |
|---|---|---|---|---|
| 1 | `hungry_out_gain` (creature.rs:12774) | The hungry way out of the nest (`HUNGRY_OUT`) | Live | **Traced**: 0 pulls in 92k-174k hungry holder decisions per seed (seeds 1-4, 20-200k, main 33389072) |
| 2 | `update_hungry_home` (:21358), `home_pull` (:21546), `home_pull_why` (:21612) | The hungry walk home: an ant counts as empty only with no pellet | Switch only (`HUNGRY_HOME`, off) | Inferred |
| 3 | `rest_pull` (:12662) | The rest pull into the nest | Switch only (`NEST_REST`, off) | Inferred for the skip; the starving route it fed is traced (`pellet-starvation-2026-10-06.md`) |
| 4 | Act's spoil branch ends in `return did` (:17430) | Digging: a laden ant never cuts, including the lean dig out of the mound (`MOUND_OUT` dig) | Live | **Traced** (digging record, old game, seeds 2-3 to 60k): no holder decision was a cut. Holders whose spell ended starved faced cuttable soil on 73% of decisions (seed 2) |
| 5 | Act's food branch returns first (:17130) | The pellet drop roll, whenever the crop also holds food the ant chose to drop | Live | **Traced**: 29-45% of nest workers' pellet decisions in the mound's tunnels (seeds 1-4) |
| 6 | `haul_bite_blocks` (:20705) | Biting food, but only when fed (`HAUL_BITE` on) | Live | **Traced**: hungry holders' bites not refused |
| 7 | `fills_before_walking` (:19819) | Filling up before a trip | Live | Inferred |
| 8 | `carries_lunch` / crop feeding (:20639, :20648) | Feeding larvae from the crop, and the packed lunch | Live (crop feeding on) | Inferred |
| 9 | `relay_crop` (:13675) | Being given food by a nestmate | Live | Inferred |
| 10 | `store_pickup_ok` (:4297) | The store pick-up at home food | Live (`Storeroom` ships `carry` and `pile` on) | **Traced** by Nest race under its `NEST_STORE`: 77% of fed nest workers at door food held a pellet |
| 11 | `store_return_target` (:4151), `harvest_load` (:4040), chooser (:21801) | The storeroom trips | Storeroom carry is on; harvest off | Inferred |
| 12 | `dig_return_target` (:14042), chooser (:21814) | The walk back to its dig face (comes after the drop) | Live (`SPOIL_OUT` back) | Inferred |
| 13 | `ready_to_lay` (:3272) | A queen's walk home to lay | Live | Inferred |
| 14 | creature_tick forage drive (:7893) | Feeling the drive to go out foraging | Live | Inferred |
| 15 | `door_read` (:21710) | The scout's door read | Live | Inferred |
| 16 | chooser scouting away from home (:21989) | Trail-away from home when nothing pulls | Live | Inferred |
| 17 | `home_pull` stray pull (:21504) | Being pulled back toward the nest when fed and astray | Live | Inferred |
| 18 | `nest_leash_holds` (:21470) | The nest leash | Live | Inferred |
| 19 | `is_leaver` (:20860) | Being counted by the forage throttle | Live | Inferred |
| 20 | chooser nurse seek (:21955) | Being drawn as a nurse | Switch only (`NURSE_STAY`, off) | Inferred |
| 21 | brood carry and spread (brood.rs:449, :635) | Being picked to carry brood | Switch only | Inferred |

## B. Pulls the pellet adds, and what they outrank

| # | Rule | What it does | Live? | Evidence |
|---|---|---|---|---|
| 22 | `home_pull` (:21523), `home_pull_why` (:21599), `home_weighted_pick_why` (:23919) | The soil's haul to the door, or the soil's way out along the passages, **outranks** the hungry walk home, crop-down, nurse-in and home | Live | **Traced**: the soil's way 31-51% and the haul 22-42% of hungry holder decisions (seeds 1-4) |
| 23 | `soil_way_pull` (:13815), chooser (:21838) | The way out for soil; below the founding ground `SoilWay`'s lean keeps the pellet until the ant is out | Live | **Traced** (Laying, nurse switch, seed 1): 98 of 161 starvers holding a pellet died below the founding ground walking it |
| 24 | `spoil_haul_pace` / `spoil_pace_target` (:14005, :14016) | Paces the walk toward the haul's target | Live | Inferred |

## C. What a holder senses differently

| # | Rule | Change | Live? | Evidence |
|---|---|---|---|---|
| 25 | `sense` Carrying (:9080) | Reads 1, as if the crop were full (`SPOIL_IS_CARGO`, on) | Live | Inferred |
| 26 | `sense` HomeAligned (:9155) | On, through `spoil_haul_pace` | Live | Inferred |
| 27 | `carried_cells` (:3168) | The pellet adds its weight to every step's cost | Live | Inferred |

## D. Whether the pellet comes down (act's spoil branch, :17154-17430)

| # | Rule | Condition | Live? | Evidence |
|---|---|---|---|---|
| 28 | Store load (:17158) | A store load goes down only in the storeroom | Live | Inferred |
| 29 | Lean drop (:17165-17181, `lean_drop_site` :21086) | A lean holder drops on an empty cell with ground under it | Live | **Traced**: 425 drops in 19,930 lean decisions in the packed mound (seed 4) |
| 30 | Ring hold (:17208) | Held while latched to the ring | Live | Inferred |
| 31 | **Keep rule** (:17238-17284) | Kept while inside or under the soil's way and patient, near the door, or below | Live | **Traced**: holds the pellet on 96-98% of stuck carriers' decisions (seeds 2-3, digging record, to 60k) |
| 32 | Drop roll and site (`spoil_site_open` :14561) | An empty cell, footing, and 3 empty above | Live | **Traced**: nest workers in the mound's tunnels put it down 0% of the time; no site 2-5%; roll lost 13-29% (seeds 1-4) |
| 33 | No lift when kept (:17375-17393) | A kept pellet is not lifted | Live | Inferred |

## What this means for the build

- **Rows 1, 4, 5 and 22 are the live route to starving**, and all four are traced. Row 10 is live too and is the store's blocker. Rows 2, 3, 20 and 21 only matter under switches that are off, which is why each of those switches cost colonies when turned on (findings file).
- **The general rule replaces the skips with one drop**: when a stronger need fires, put the pellet down here, then let the ordinary rules run as for an empty ant. So the parts are the needs, not the rows: `hungry` (rows 1-3, 4 via the lean dig, 22-23), `laden` (crop food: rows 5, 8, 9), `job` (rows 10, 11, 20, 21), plus `pack` for the encased ant with no open cell.
- **Known risk to trace first**: a dropped pellet in the shaft or at the door re-seals it (the keep rule exists to stop exactly that; rows 31-32). Measured with `doorseal.py` per seed.
- Rows 12-19 and 24-27 stay as they are: they steer or cost a holder but do not hold a need away from it.

## E. The four parts added at 13:32 (Scott: "same switch")

What each part changes, read from the code at main 3ba1e7bd, before building. The evidence column is the rule audit's (`/mnt/project-files/rule-audit/rule-audit-2026-10-06.md` §2) unless marked. All four key on one new test, **enclosed**: under cover, inside the nest's box (mound and nest), and with no path of standable cells (`way_cell`, brood and crumbs passable) to open air. It is a breadth-first fill from the open air, rebuilt every 30 frames as the mound's way is, and built only while one of these parts is on. Unlike `MOUND_OUT`'s `shut_in_mound` it reaches below the founding ground, where seed 1's 22 died.

| Part | Rule it changes (creature.rs:line) | Change | Evidence for the rule | Risk to watch |
|---|---|---|---|---|
| `throttle` | `outward_want` (:21376), inside the door's throttle zone | The ant's own hunger counts: want = max(colony's want, hunger), instead of hunger counting only below half the grant | Code-read. Traced effects: newborns go out by the hungry way out; pulled spells mostly refill without eating (findings 22, 34) | More ants leave by the door; the throttle's job (sending out only what the colony needs) weakens |
| `weak` | Act's lean gate (:16093) | An enclosed lean ant keeps its dig roll, anywhere (today only `shut_in_mound`, mound only) | Traced: `MOUND_OUT` dig cut shut-in spells ending starved 1.30% -> 0.50% (12 seeds); seed 1's shaft deaths are outside it | A lean ant spends energy digging; aim is the dig's own (no aim out) |
| `breakthrough` | The heap cue (`spoil_cue_factor`, :14565; applied at :17815) | Stands aside for an enclosed digger: a cut that opens to the sky no longer needs pellets within 2 cells | Traced: 63 of 63 cuts at a plug refused, colony died (Laying 10-04); 219 of 598 rolls at a plug's inner face refused (seed 4) | More mouths. Leaving the cue aside for every roofed digger was measured worse (dead-ends: more mouths on 17 of 24 `digbox` seeds); this is narrower (enclosed only) |
| `door` | `FaceTrip`'s `only` (`face_trip_refuses`, :13982; applied at :17839) | An enclosed digger walking back to its face may cut more than 3 cells from it | Traced: 737 refusals of a plug against 99 cuts (seed 1, rest pull); 11 of 17 reopening rolls (seed 4) | Diggers behind a sealed door cut walls on the way, not only the plug |

Not in any part: the roof rule (`DIG_ROOF`, refuses cuts under a thin roof) also applies to escape cuts. It will be counted for enclosed diggers, not changed.

Scoring plan: each part alone, all eight on, and all eight with each left out, seeds 1-4 to 300k, against Deep trace's dig-on baseline. Metrics per seed: time in the dug nest (headline), door seals (`doorseal.py`), shut-in spells ending starved (`shutep.py`), starved, and new mouths for `breakthrough`.
