## What this does

Three of the four items from the owner's 2026-09-14 held-world playtest —
Lane C's slice ("the shell"):

- **Plants bend/break under stress, made to catch up when you turn it back
  on.** The owner's suspicion — *"when I change plants bend or break under
  stress, I am not sure it is actually changing it in the game right
  away"* — turned out to name a real gap rather than a wiring bug: bending
  is checked live every organism tick and reaches you within a few seconds
  either way, but the load-failure switch's other two consumers
  (`structural.rs`'s cantilever span check and its detached-living-plant
  clause) only re-judge a cell when a structural check is scheduled for it,
  and turning the switch back on schedules nothing. An already-settled,
  over-span beam could stand forever after you re-enabled the rule that
  should now bring it down. Fixed by re-queuing a check on every living
  plant cell the moment the switch flips back on.
- **A restart key (`N`).** *"There should be a restart options."* Generating
  the held world is about a minute of wall clock on one thread with nothing
  else able to run meanwhile, so the key does not restart on the frame it is
  pressed — it shows "regenerating" first, lets that frame reach the screen,
  and only then blocks, so the freeze reads as a wait rather than a hang.
- **One bubble-size dial.** *"There should just be one bubble control size
  for the druid and placeable bubbles."* `Q`/`E` now drive both the circle
  you place and the circle you carry from a single number; `[`/`]` are gone.
  The two circles still end up different sizes for reasons that were always
  load-bearing — the carried one is capped low and free at its base, the
  placed one is capped high and priced by count rather than size — so the
  shared control is an *offset* from each circle's own base rather than one
  shared absolute radius, which is what keeps "the circle you already are
  must stay free" true at the dial's own default.

## Where it sits

Three independent fixes out of four playtest items in the held world's
"shell" brief; item 4 (bubble look/aura) is a colleague lane's slice. All
three here are the kind of gap that reads as "the game ignored my input" —
closing them is about the controls telling the truth about what they just
did, not about new mechanics.

## Mechanism

**Plant bend/break.** `World::schedule_structural_recheck_of_all_living_
plants` (`src/sim/structural.rs`, beside the existing `schedule_structural_
check_around`) walks every organism's cell map and re-queues a structural
check on each; `druid::menu::Setting::PlantBreak::advance` calls it right
after flipping `World::plant_load_failure`. Reproduced first
(`flipping_the_load_failure_switch_the_way_the_menu_does_does_not_
retroactively_recheck_a_settled_beam` — passes today, i.e. is the bug),
then a second test proves the fix
(`schedule_structural_recheck_of_all_living_plants_catches_up_a_beam_the_
switch_had_exempted`). `World::plant_bending` needed no equivalent fix — it
is read live every organism tick, never queue-gated.

**Restart.** `Handler::request_restart` (`src/bin/druid.rs`) arms a
one-frame-deferred `restart_countdown`: the keypress sets a `Druid::note`
message and the countdown, the following drawn frame presents that message
against the still-current world, and only the frame after that calls
`Handler::perform_restart`, which is where `Druid::new()` — and its ~minute
of generation — actually blocks. `perform_restart` also resets what belongs
to the run (tick accumulator, held movement keys, the biosphere page's
history) rather than the window, while preserving whether that page was
open.

**One bubble dial.** `carried_radius_for(place_radius)` (`src/druid/mod.rs`)
returns `(CARRIED_RADIUS + (place_radius - PLACE_RADIUS_START)).clamp
(CARRIED_RADIUS, CARRIED_RADIUS_MAX)`; `Druid::set_bubble_radius` is the one
place that sets both `place_radius` and `world.carried_radius` now, and
`Q`/`E` are its only callers. Guarded by
`the_one_dial_leaves_the_carried_circle_free_at_its_own_default_and_clamps_
each_side_on_its_own_range`, which checks the free-at-default property the
merge could have quietly broken, and that each side pins at its own range
boundary rather than the narrower of the two.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings`: clean.
`cargo test --release`: PASSED_OR_FAILED_PLACEHOLDER.
`bash scripts/docscheck.sh`: clean (pre-existing lane-note size warnings
from unrelated lanes). `python3 scripts/deadendindex.py --touching`:
DEADEND_PLACEHOLDER.

## Rendered

RENDER_PLACEHOLDER
