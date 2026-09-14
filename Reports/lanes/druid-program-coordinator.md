# Coordinator — the held world (`--bin druid`)

**The druid program's standing note.** Its sibling for the second game is
[`evolution-lab-coordinator.md`](evolution-lab-coordinator.md), and this is
deliberately the same shape: **kept small on purpose**, carrying only the
standing owner rulings, the live round, and the environment facts that have
cost time. Finished rounds — *what each overturned*, which is the part a later
session cannot reconstruct — are in
[`../druid-rounds-archive.md`](../druid-rounds-archive.md). Read the one
round, not the file.

**The machinery of running lanes is not here.** Spawning, model choice,
reaching a lane, closing a round: the **`lab-coordinator` skill**, whose name
says lab for historical reasons and whose contents are not lab-specific —
invoke it *before* you spawn anything. The protocol behind it is
[`../session-programs.md`](../session-programs.md). What the *game* is:
README's `Held world status` and
[`../held-world-game-concept-2026-09-13.md`](../held-world-game-concept-2026-09-13.md).

## Standing owner rulings

These are decisions, not findings. They do not expire when a round closes.

- **Pheromone trails: the gnome-laid one is the druid's, the ant-laid ones are
  the evolution lab's.** 2026-09-14, asked and answered directly: *"For
  pheromone, you make the gnome laid trail last longer. The evolution lab will
  explore the ant laid ones."* Earlier the same rule, wider: *"I dont want you
  working on pheramone trial mechanics. that is the evolution lab property,
  but the bug you found should be fixed."* So `pheromone::DECAY_RHO` and
  anything ant-laid depends on is **not** this program's to change.
- **The world starts bare and she plants it.** *"The world should not start
  with any seeds. The druid has her own seeds to plant and that populates the
  world."*
- **One hue is the default look**, not a key away.
- **No readout of how much life energy an animal holds.** *"You just get less
  or none if you try to absorb too soon. You should just see how much energy
  you get by how many particles come."*
- **Order of the big pieces, given 2026-09-13**: *"1) I dont want you working
  on pheramone trial mechanics... 2) Shink and walk the nests. 3) The enemy.
  In that order."* Steps 1 and 2 are done; **the enemy is next**, and it is
  gated on one `creature_arena` sweep at >= 24,000 frames read at an order
  statistic over seeds — the last predator numbers predate the graded bite and
  would null.
- **It is a separate game.** *"I don't want to build this into the existing
  gnome game. This is a fully new and separate game."*

## The live round — round 2, the twelve playtest items

Owner playtest 2026-09-14. Items in their words are in the round-2 dispatch;
state:

| # | item | where |
|---|---|---|
| 8 | her own circle vanished above 1x | **done**, PR #430 |
| 10 | simplify the top-left readout | **done**, PR #430 |
| 1, 6, 7 | bubbles merge + speed as colour, creature colours, lab overlays | Lane A, `claude/druid-look` |
| 2, 5, 9 | the yellow bars, founding far away, the founding screen | Lane B, `claude/druid-founding` |
| 3, 4, 11a | the stress setting, restart, one size dial | Lane C, `claude/druid-shell` |
| 11b | longer gnome-laid trail, diffuse look | Lane D, `claude/druid-trail` |

**Item 3 is a suspicion, not a confirmed bug**, and Lane C's brief says so: it
may be working and merely invisible, which wants a different answer than a
wiring fix.

## Environment facts that have cost time here

- **The druid writes its own screenshot filename** — `pixel_physics_druid_screenshot.png`,
  not the sandbox's. Waiting on the sandbox's name looks exactly like a
  capture that never fired.
- **It does not exit after the shutter**, and `pkill -f target/release/druid`
  matches the *wrapping shell's* command line and kills the script instead.
  `for p in $(pgrep -x druid); do kill $p; done` is the one that works. Both
  cost twenty minutes, one after the other, on 2026-09-14.
- **The headless hooks are the only way to reach a key**, and each one is its
  own env var: `_SMALL=<tick>`, `_ZOOM=<rung>`, `_CIRCLES=x,y,r,rate;...`,
  `_OFFER`, `_MENU`, `_FOUND`, `_FOUND_AT`, `_ABSORB_AT`, `_WALK=<a>,<b>`,
  `_GIF`, `_CATCH`, `_KEYS=0`, `_SIZE`, `_GROW`, `_START`, `_CIRCLE=off`,
  `_UNLIMITED`, `_LOOK`, `_PRESET`, `_CENSUS`, `_LAY`, `_MARK`.
- **A hook that fires at construction fires too early.** `spawn_point` returns
  a *surface* cell and `Player::at_scaled` **centres** the body on it, so at
  construction her feet are seven rows inside the ground. Anything that tests
  the ground under her must run at a tick, not in `Druid::new`.
- **The two clocks do not line up.** `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`
  counts *drawn frames*; the game's hooks count *player ticks*; and on a
  software rasteriser one drawn frame is worth several ticks. A verb whose
  effect you want in the frame must **schedule its own shutter** — the absorb
  path and the shrink both do.

## Findings that outlived their round

- **Before sizing an item from a doc comment, check the code does what the
  comment says.** Four of round 1's seven items were sized off docs and two of
  those four were wrong — `Draw::amount` claimed to set how heavy the flow
  looked and was never read by the renderer; `carried_cost` already priced the
  base circle at zero, so item 5 had no power to save and wanted an *off*
  state instead.
- **A card's verdict and a playtest item can be the same defect wearing two
  descriptions.** Three 2026-09-13 review verdicts were answered by round 1
  without anyone routing them there. Read `review.py inbox` *before*
  dispatching a round, not after.
- **A push while CI is running is a decision to start CI again**, and on this
  repo that is a twenty-minute round trip. Three restarts on #408 were the
  coordinator's own successive pushes.
- **`src/sim/creature.rs` is shared with the lab**, so any founding or
  placement change reaches it silently and wants a paired before/after on a
  lab bed as well as in the druid.
