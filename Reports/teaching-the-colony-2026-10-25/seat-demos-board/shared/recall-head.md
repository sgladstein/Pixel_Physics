# Shared piece: a trained ant that acts on a remembered place (the recall head)

Owner: **keeper** seat (thread "Build the keeper seat demo", branch claude/project-thread-20o5zg).
State: **PR #907 up** (https://github.com/sgladstein/Pixel_Physics/pull/907), head 9abb556b, branch claude/project-thread-20o5zg, main + #904 merged in; with the merge desk. Off is byte-identical to main (keeperlook world hash, seeds 1-3). Stack on the branch now, or on main once #907 lands. API: recall::plant / plant_with / knows / tilt_for / share_on_feed, creature::set_down / treat / is_home_for; switch PIXEL_PHYSICS_RECALL_GAIN=g[,copy]; World::recall.
(when its scripted "knows the patch" becomes real), colony mind (habits). Comments: message the keeper thread
or write in your own status file.

## Why it is needed (engine facts, keeper review 2026-10-25)
- Mind ants have a last-food memory (`f::FOOD_WAY`), but the shipped pool's weights on it are zeroed on load
  (`mind.rs:539-552`, layout 4), so writing it changes nothing on today's ants.
- Hand-seeding a scorer weight has no free unit (all 16 units live in all 30 members).
- Body-level `chooser_step` pulls never reach a mind ant (returns at `creature.rs:8022`).
- The precedent that does work: the dig side head (`mind.rs:5833-5856`), a term added to an option's raw
  score before the sharpness squash, with its gain riding past `GENOME_LEN` like the lay gene.

## Interface (proposal; firmed up after the keeper plan is reviewed)
- `World::recall: Option<Recall>`; **`None` = off, byte-identical**. Set only by a demo (`demo=keeper`) or
  `PIXEL_PHYSICS_RECALL_GAIN=<g>`.
- Term (AS BUILT, 10:00Z): **after the verb is drawn**, if it is Walk, each Walk target's pick weight is multiplied
  by `exp(gain * strength * FOOD_WAY)`. It changes WHICH way an ant walks, never WHETHER it walks (job stage untouched),
  never excludes a heading. Adding before the squash (my first text) would also have changed whether ants walk and their
  recurrent memory; the dig head's note says that flattens saturated scores. Applies only to ants holding a TAUGHT or
  COPIED memory (provenance gate: planted by `plant` or received by a meal), not to every ant that found food itself,
  and only when unladen (no crop/spoil). Gain is a per-world scalar for the demo (scripted, labelled, honest arm = 0);
  later a heritable rider so selection can tune "how much to listen".
- Counters: `tilted`, `mass_moved` (total variation of the Walk pick, exactly 0 at gain 0), `picks_flipped` (same draw
  with and without the tilt).
- `recall::plant(world, ant, cell)`: write an ant's `last_food` to a cell (fresh), for treat bead / carry.
- `recall::share_on_feed`: when a mind ant feeds a nestmate, the receiver's `last_food` takes the giver's if
  the giver's is fresher (so a place spreads by meals as a graded epidemic). Switchable separately.
- Read-outs: per-ant "knows place P" (last_food within r of P and fresh), decisions the term changed (pick
  recomputed with the term at 0), copies made by feeding.

## 2026-10-25 ~12:45Z change (in the keeper demo PR)
- recall::prune drops only the dead: a lesson whose last_food a stray bite moved elsewhere SLEEPS (lesson() None, no tilt) and wakes at the next bite within SAME_PLACE of the taught place.
- SAME_PLACE 6 -> 12 (a station heap spans ~9 columns either side).
- RecallCounts::offered: decisions by a taught, unladen ant (tilted/offered = the Walk share; measured 97-98%).
- creature::can_carry(world, id): set_down refuses stacked ants; pick liftable ones.
