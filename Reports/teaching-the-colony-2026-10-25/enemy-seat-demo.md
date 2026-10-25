# The enemy seat: a playable raider demo (2026-10-25)

**What it is.** `cargo run --release --bin lab -- demo=enemy` (add `oldant` for the hand-built colony). You play the
shipped nest raider, a five-cell centipede, against a living ant colony that has dug in for 20,000 frames. The colony
fights back with the engine's own bite rules. It does not learn you: alarm fades in about three seconds and an ant
born in the lab carries its parent's trained mind unchanged.

**Where it sits.** The enemy seat is one of six seat designs for teaching the colony
([`seat-enemy.md`](seat-enemy.md), critique [`seat-enemy-critique.md`](seat-enemy-critique.md)). The owner asked for
each to be adversarially reviewed for feasibility, planned, the plan independently reviewed and revised, then built
to a playable demo. This report closes that loop for the enemy seat. The pieces:

| Step | File |
|---|---|
| Feasibility review (adversarial) | [`enemy/review-feasibility.md`](enemy/review-feasibility.md) |
| Implementation plan v1 | [`enemy/plan-v1.md`](enemy/plan-v1.md) |
| Independent review of the plan | [`enemy/plan-review.md`](enemy/plan-review.md) |
| Plan v2 (what was built) | [`enemy/plan-v2.md`](enemy/plan-v2.md) |
| Shared steering for any mind body | PR #903 (`sim::steer::take_forced` / `choose`, `World::mind_force`) |
| The demo | `src/lab/demo/enemy.rs`, `assets/lab_scenarios/demo_enemy.ron`, on the queen seat's launcher (PR #904) |
| The autopilot | `examples/enemyhunt.rs` |
| Pictures for the owner | Artifact page "The Raider Demo" (blind A/B of the two brains, the door charge, the run table) |

## Feasibility verdict, in one paragraph

The design's three modes were a Raider (play the predator), a Range (a rival colony) and a Pit (an antlion trap). Only
the Raider is buildable on today's engine without new physics: mind ants bite any other colony, so a Range is a war
rather than a seat, and the sand does not trap ants and there is no antlion species, so a Pit needs both. The
review's findings that shaped the build: the colony cannot learn within a session (no mutation at a lab birth, the
std pool's 30 members are near-clones), so the demo says so on the season card rather than promising it; the steered
body needed a way to drive a mind body (PR #903); and a raider that buds must be stopped (two raiders became 72).

## What the plan review changed

The review's must-fix list, all built: damage is one `gnawed` meter per body and the cell that takes the filling bite
comes off (head means death, a neck bite takes everything behind it), so the HUD says exactly that and the help line
says "back out tail first"; Q spits a crop cell so biting can resume (there is no satiation ceiling to promise); a
refused bite is decided inside `act`, so `Steered::last_act` records it for a "why did nothing happen" line;
`Steered::colony = u32::MAX` would have crashed `World::book`, so the colony is left as `mark_with` sets it;
`KillRecord` has no position, so a kill ring is drawn at the raider's head on the frame the record appears; loads
home read `creature_stats.trip_deliveries`, never `deliveries` (2.4-5.8x inflated, from the colony mind seat).
Season 2 (a colony that adapts across seasons) was redesigned as a separate, owner-gated project in plan v2 Part B.

## What was built

- **Engine hooks, inert for every body nobody marked.** `Steered::may_breed` (read at the top of `try_bud`): the
  player's raider does not bud. `Steered::last_act`: the steered mind body's asked verb, done verb and success,
  written after `act` in `mind::tick` only when forced.
- **The box.** `demo_enemy.ron`: the Teacher lane's `lab_mid_fruit` steady-income bed with the fruit heaps 40 columns
  from the door. The trained pool lived on it to 66,000 frames on 4 of 4 seeds with no raider.
- **The run of play.** A visible fast-forward (dial preset 5) while the colony lands at frame 6,000 and digs in; at
  26,000 the raider is set down 70 columns from the nearest door, the dial drops to 1x, the camera closes in to three
  screen pixels a cell and follows it, and the BATTLE panel is kept off the HUD.
- **Legibility.** The raider in crimson with a white ring on the head (the shipped centipede is amber, the ants'
  colour within a shade, and at hunt zoom the first frames could not be read). A pulse where each bite lands (yellow
  yours, red theirs), a ring on a kill, the doors marked. HUD: cells, wound meter, gut, tank, kills and bites both
  ways, colony size and loads home since you came, and a line saying why a key did nothing. A season card (Y, or on
  death) with time hunted, kills, cells lost by place in the chain, ants then and now, loads home, and one plain
  sentence.
- **Counts from the world.** Kills from `kills_log`, bites given from `encounter_log`, **bites taken from the body's
  wound meter**: the hand-built ant's gnaw (the Eat path at an animal cell) wounds without writing an
  `ENCOUNTER_BITE` row, and the log alone showed 1 bite on a raider whose head came off.
- **Keys:** arrows walk, B bites, E eats, Q spits or puts a crumb down, G cuts stored crumbs, Y the card, R a new
  raider after a death. `land=` and `warm=` move the landing (the raider never lands before the colony has a door).

## Measurements

All with `examples/enemyhunt.rs`, an autopilot that presses the player's keys through `Lab::demo_key`, on main at
648d705, 6 seeds a row, 12,000 frames after landing. `policy=door` walks straight at the nearest door and holds down;
`policy=skulk` waits on the surface for an ant walking alone (at most one other ant within 6 cells, 12+ columns from
the door), bites it, and backs off when wounded or a cell is lost.

| Play | Colony | Raider died | Fastest death (frames) | Kills per run | Kills in all |
|---|---|---|---|---|---|
| door, landing 70 | trained | 6/6 | 516 | 0 | 0 |
| door, landing 70 | hand-built | 6/6 | 439 | 0 | 0 |
| skulk, landing 30 | trained | 4/6 | 326 | 0-4 | 8 |
| skulk, landing 30 | hand-built | 1/6 | 981 | 0-4 | 9 |
| **skulk, landing 70 (shipped)** | trained | 4/6 | 2,050 | 2-6 | 18 |
| **skulk, landing 70 (shipped)** | hand-built | 0/6 | never | 2-10 | 38 |

- **A0.2, "is there a fight", passes for both brains, and the trained colony is the harder opponent.** The plan's
  gate was under ~1 bite a minute on a raider in the nest. No scripted raid ever got into the nest: the colony bit it
  apart on the mound, 3-51 bites per run, losing cells tail first before the head (the door clip on the page: cells
  5, 4, 3, 2, then the head, five seconds from landing). The demo opens on the trained colony, the lab's default.
- **The landing moved from 30 to 70 columns.** At 30 the raider was set down on the foragers' road between the door
  and the fruit, and two trained seeds killed it inside 430 frames, before a player has found the keys.
- **A0.3, the digest clock:** the crop falls about 0.65 a frame (1,420 to 1,407 over 20 frames on a hand-built run),
  so one ant cell of 480 takes about 740 frames, 12 seconds at 1x.
- **A0.4, the alarm trace, was not run.** It decided one HUD word ("they come to the bite"), which the demo does not
  print.
- **A6, frame cost:** the demo's view costs `Lab::draw` 2.45 ms against 2.16 ms for the plain lab at the same camera
  (mean of 3,600 frames, `enemyhunt drawcost`), p99 4.0 against 3.7 ms. With the camera following, the dirty-rect skip
  does not apply while you hunt; the demo is off unless launched with `demo=enemy`, and `ascii` never runs it.
- **The morning's numbers were on an older main.** The first 24 runs (before the brain link price went to 0.25 by
  default, e7763514) had a trained colony of ~105 ants at landing; on 648d705 it is ~230-250, close to the hand-built
  colony's. The conclusions held (door 12/12 dead; 70 columns better than 30), the counts above are the re-run.

## What the autopilot is not

It is a dumb, fixed policy, the same for both brains. It does not back out tail first, does not use Q, and never
retries the door after its first approach. A human will do better and worse than it. Its job is a paired comparison
and the pictures, not a difficulty rating.

## Mutation at birth (owner's pick, 2026-10-25 14:40Z)

The thread's decision card asked how the demo should show the colony learning. The owner picked "Turn on brain
mutation in demo". Built (PR after #909):

- **Every egg of the trained colony takes a mutated copy of its parent's mind**, in `demo=enemy` only. It goes
  through the queen seat's fathers store (`sim::fathers`), which gains a standing level `Store::wild_all`, spent by
  no egg (0, the default, is today's frozen inheritance). The demo draws every founder's mind from minds the store
  built, so any child can be rebuilt mutated, and drains the store's log each tick, pruning genomes no ant carries.
  The operator is the TRAINING page's own (`lab::training::mutate_genome`).
- **Seasons.** Each raider life is a season. The card lists the newest four: time lived, kills, bites taken, and
  **bites per minute in reach** (an ant cell within 3 cells of the head). It shows the rate, not the count, because a
  raider that hides is bitten less without the colony having changed.
- **The rate, measured on the colony alone** (`enemyhunt warm=1000000000 frames=176000`, no raider), ants at frame
  175,000, 6 seeds a level:

  | Level | Ants left |
  |---|---|
  | off | 16, 183, 274, 324, 326, 261 |
  | training rate (`kpm` 0.03, `kevents` 10; `mutate=3`) | 182, 2, 0, 0, 4, 72 |
  | a third (`mutate=2`) | 319, 361, 147, 293, 128, 109 |
  | **a tenth (`mutate=1`, the default)** | 253, 368, 307, 276, 336, 238 |

  At the training page's rate the colony melts down: three went extinct by frame 150,000 with no raider. The
  training bed culls a bad mind before it breeds; the lab culls nothing. The first A/B with the raider at that rate
  showed the same (4 of 6 below 30 ants; the raider took 5-31 ants a run, far too few to cause it). At a tenth of the
  rate every egg still takes a changed mind (about 3,000 in a run this long) and the colony lives in the off arm's
  spread, so that is the default. `mutate=0` turns it off.
- **Not shown: that the colony gets better at fighting the raider within a session.** Over 150,000 frames the
  scripted raider died 1-7 times a run, and the per-season bite rate swung from 4 to 222 with how it played. That is
  far more spread than any trend a handful of ant generations could show. The card makes the claim checkable, not
  true.

## The owner's first playtest: no spit after a bite (2026-10-25)

The owner played it and found the spit "really annoying": after the first kill every bite was refused until Q spat a
cell. The cause was the centipede's sated rule (`CreatureDef::handling`, "a sated predator does not kill"): its crop
holds 1,440, three ant cells, and digests one in about 740 frames, so the crop filled on the first ant. The rule is a
decision, not physics, so the player's body now bites past it (`Steered::bites_when_full`, set by the seat): the
bitten cell still comes off and is not swallowed. `fullbite=0` puts the rule back. Guard:
`mind::tests::a_full_player_raider_bites_on_and_swallows_nothing` (red with the check removed).

Measured with `enemyhunt policy=skulk`, trained colony, mutation on (the default), 12,000 frames after landing, the
same 6 seeds both ways:

| | Kills per run | Kills in all | Bites taken | Raider died |
|---|---|---|---|---|
| sated rule (`fullbite=0`) | 1-6 | 26 | 6-17 | 1/6 |
| **bites when full (shipped)** | 36-72 | 345 | 1-17 | 2/6 |

A stalker now kills about an ant every 3 seconds at 1x and lives. The colony ends 186-255 ants against 201-303. The
mound is still death for a raider that walks into it.

## Open

- **Season 2 as designed** (a line bred off-screen against raiders, judged against a no-raider control) was not
  picked. The owner chose mutation in the demo (above).
- **Nobody has played it by hand yet.** The page asks which colony is the better opponent.

## Tests

`lab::demo::enemy::tests::the_box_is_untouched_until_the_raider_lands` (cell-identical to no demo over 3,000
frames), `a_raider_lands_after_the_warm_up_and_obeys_the_arrows` (lands, is the player's, may not bud, the camera
closes in, walks the held way), `sim::steer::tests::a_body_marked_not_to_breed_does_not_bud` (against a control that
buds; red with the `try_bud` check removed).
