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

## The live round — none

Round 37 (the twelve playtest items from the 2026-09-14 playtest) **closed
2026-09-15**: all twelve items landed across PRs #430, #435, #437, #438, #449,
plus #439 and #402. What it overturned is in
[`../druid-rounds-archive.md`](../druid-rounds-archive.md).

**Next is the enemy**, and it is gated on one measurement before any of it is
built: `creature_arena` at **>= 24,000 frames**, read at an order statistic
over seeds. The last predator numbers predate both the graded bite and the
trait reach going to 8, so they would null. And the concept report's own top
risk is a design risk rather than a technical one — *"an enemy that attacks
your economy is often the least fun kind. This is the first thing to
playtest"* — so a crude playable version and a review card come before a
finished mechanic.

## Standing: what this program does NOT own

- **The nest mechanic is the evolution lab's**, ruled 2026-09-14 when the
  owner asked directly whether it should go to that coordinator instead. It
  is ant behaviour in shared `creature.rs`, entangled with the ant-laid
  pheromone work the lab is already running, and the lab holds the foraging
  baseline (414 deliveries) any change is measured against. **Handed over in
  full** — the owner's brief verbatim (attach the nest to a world location
  rather than a material; a circle or blob so a dug-up nest stays reachable;
  research how real ants find home; his hunch that the nest does little
  because ants go to food), plus everything established here, plus the three
  druid-side requirements a redesign must keep. What this program kept is the
  *look*: porting the lab's own `earth_toned_nest`.
  The evidence gathered before handing it over went *with* it rather than
  staying here, which is the point of a handoff: read the brief in that
  session, or re-derive it from `nest.ron`'s own doc, which answers most of it
  in its first paragraph. The one number worth carrying: `nest` is
  `penetration_resistance` **6.0** against a `dig_force` of **1.0** on every
  shipped ant, so a colony cannot excavate its own doorstep.
- **The zoom-out rungs are not this program's either.** *"Get rid of stop
  3"* (card `20260914T084458895Z-ae1b01`) is `render::MAX_ZOOM_OUT_STRIDE` and
  `src/app.rs`, shared with both other games, and was already in flight as
  PR #431 — found by reading the open PR list before routing it. **A verdict
  on the druid board is not automatically druid-program work**, and the check
  is the same one this note already prescribes before dispatching a round.
- **...but the zoom-out *pixel budget* was, and it was missed for two days.**
  Reported from play 2026-09-15 as *"it works well in the lab and looks really
  crisp when we zoom out, but it doesn't look as good in the druid game"*. The
  cause was not in the renderer at all: `bin/druid.rs` built its `Pixels`
  buffer at `(WIDTH, HEIGHT)` and never resized it, so this game alone kept
  taking 2048x1280 cells through 512x320 pixels while the other two grew the
  buffer. **The lesson for routing is the inverse of the bullet above**: a
  shared-renderer feature landing in `src/app.rs` and `src/lab/` is not
  automatically in this game, because this game has its own binary and its own
  HUD, and nothing fails when it is left out — it just looks worse. When a
  render feature lands for "both games", check which two.
  `Druid::pixel_budget` ships from 2026-09-15; the headless hook is
  `PIXEL_PHYSICS_DRUID_ZOOM_OUT=<rung>,<budget>`.

## Environment facts that have cost time here

- **The druid writes its own screenshot filename** — `pixel_physics_druid_screenshot.png`,
  not the sandbox's. Waiting on the sandbox's name looks exactly like a
  capture that never fired.
- **It does not exit after the shutter**, and `pkill -f target/release/druid`
  matches the *wrapping shell's* command line and kills the script instead.
  `for p in $(pgrep -x druid); do kill $p; done` is the one that works. Both
  cost twenty minutes, one after the other, on 2026-09-14.
- **The headless hooks are the only way to reach a key**, and each one is its
  own env var: `_SMALL=<tick>`, `_ZOOM=<rung>`, `_ZOOM_OUT=<rung>,<budget>`, `_CIRCLES=x,y,r,rate;...`,
  `_OFFER`, `_MENU`, `_FOUND`, `_FOUND_AT`, `_ABSORB_AT`, `_WALK=<a>,<b>`,
  `_GIF`, `_CATCH`, `_KEYS=0`, `_SIZE`, `_GROW`, `_START`, `_CIRCLE=off`,
  `_UNLIMITED`, `_LOOK`, `_PRESET`, `_CENSUS`, `_LAY`, `_MARK`,
  `_OVERLAY=<n>` (presses `O` n times).
- **A still cannot show her scent trail, and the two clocks are why.**
  `_OVERLAY=5` selects the pheromone plane, but the shutter counts *drawn
  frames* while `_WALK`/`_LAY` count *player ticks*, so a shutter early
  enough to be affordable fires after she has walked about four cells and
  the overlay draws an empty plane. That is a vacuous arm, not a broken
  feature — the same trap `render.rs`'s own overlay guard records under
  `FieldOverlay::Light`. Use `_GIF`, whose stride **is** in ticks.
- **A hook that fires at construction fires too early.** `spawn_point` returns
  a *surface* cell and `Player::at_scaled` **centres** the body on it, so at
  construction her feet are seven rows inside the ground. Anything that tests
  the ground under her must run at a tick, not in `Druid::new`.
- **The two clocks do not line up.** `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`
  counts *drawn frames*; the game's hooks count *player ticks*; and on a
  software rasteriser one drawn frame is worth several ticks. A verb whose
  effect you want in the frame must **schedule its own shutter** — the absorb
  path and the shrink both do.
- **…and it bites the GIF hook from the other side.** `_GIF=start,every,count`
  tested its stride at *draw* time, so a run that draws several times inside
  one player tick captured that tick over and over: measured 2026-09-14, 60
  frames spanning **38 ticks**, and the walking speed read off that clip was a
  third of the real one. Fixed — the capture now takes one frame per tick —
  but the class stands for any hook whose condition is read in the draw path.
- **A GIF of a walk cannot be read for speed**, because the camera follows
  her and two speeds differ only in how fast the ground goes past. The clip is
  still the right artifact for whether it *reads*; the number has to ride
  beside it. `_GIF` now prints `gif walk: N cells in M ticks`, in plain cells
  and in her own body-lengths, for exactly that.
- **At zoom 1 a 2x3 gnome is six pixels and the legend covers the ground.**
  Any shrink render wants `PIXEL_PHYSICS_DRUID_ZOOM=4` and
  `PIXEL_PHYSICS_DRUID_KEYS=0` together; without both, the first contact sheet
  of the feature is a picture of a hillside and reads as "it does nothing".

## Findings that outlived their round

- **`review.py inbox` is not a filter in a cloud container — use
  `list --board druid` and `get <id>` instead.** `_is_mine` is an `or` over
  branch / worktree / agent, and every cloud session clones to
  `/home/user/Pixel_Physics`, so the worktree clause is true for **every card
  in the queue**. It also marks what it returns as seen. Verified by reading
  the predicate, not taken from the report. What that cost here on
  2026-09-14: the first `inbox` call returned other lanes' cards, and the
  second returned **zero** — which reads as *"no new verdicts"* and actually
  meant *"the first call consumed them"*. No verdict was lost, because the
  board listing was used from then on, but by luck rather than method. Filed
  as **§Z26** by the thicket-founding lane (PR #427), which carries an
  explicit *do not fix this by syncing `seen/`* until the filter is tightened
  — syncing the markers is what turns a harmless bug into a lane silently
  eating another lane's answers.
- **The grow phase is not an independent source of plants — it grows the
  scatter's seeds**, and this note had that backwards. It recorded the
  density change as *"relieves §Z21 where the owner meets it while leaving
  grown and dead untouched"*. Grown and dead were not untouched; they were
  relieved **hardest**, because their entire organism count came from growing
  the scatter, so zeroing the scatter left the grow phase nothing to work on.
  Measured by the thicket-founding lane in PR #427: 0 organisms after the
  grow phase on all three starts. Two sessions had it backwards in the same
  direction at the same time.

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
- **When a size changes, measure the gait in the body's *own* lengths — that
  is the number that says whether anything happened.** The first shrunk gnome
  covered **0.043 of her own body-lengths a tick, the identical figure as the
  14-tall one**, which is what a uniformly scaled *picture* does and not what
  a small animal does. In plain cells per tick the same arm reads 0.129
  against 0.600 and looks like an ordinary consequence of being small. The
  generalisation past this game: when a transform scales a body against a
  world that did not scale, the quantities that must stay constant are the
  dimensionless ones, and a raw rate is not one of them.
