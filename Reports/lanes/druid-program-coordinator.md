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
| 8 | her own circle vanished above 1x | **landed**, PR #430 |
| 10 | simplify the top-left readout | **landed**, PR #430 |
| 3, 4, 11a | the stress setting, restart, one size dial | **landed**, Lane C, PR #435 |
| 1, 6, 7 | bubbles merge + speed as colour, creature colours, lab overlays | Lane A, **PR #437** open |
| 11b | longer gnome-laid trail, diffuse look | Lane D, **PR #438** open |
| 2, 5, 9 | the yellow bars, founding far away, the founding screen | Lane B, `claude/druid-founding` — pushed, amending |
| — | the shrunk gnome moves too slowly (card verdict, not a playtest item) | **landed**, PR #439 |
| — | the regional-time scope report, stuck un-mergeable since 2026-09-13 | **landed**, PR #402 |

**Round 37's verdicts, all four answered 2026-09-14 evening.** Speed: *"Looks
good"*, 5 of 5 — closed. Founding menu: approved as built. The other two sent
work back: the trail wants **2x more lifetime again** and its anchor raised,
and the founding patch's colour **dies entirely** rather than softening (1 of
5). The trail anchor is the one worth keeping — the lane set it to
`Player::feet`, which is about four cells *under* any soil surface, because
`wade_rows` sinks a standing gnome knee-deep by design. Same fact that made
the shrink verb refuse on every soil surface in the world.

**Lane A and Lane D finished without the GitHub tools**, in the shape
`CLAUDE.md` describes: branch pushed, `PR_BODY_LANE_*.md` written on it, head
SHA reported. The coordinator opened both. **The tell that a lane is done is
a last commit reading "lane note, PR body"** — there is no other signal, and a
finished branch with no PR is invisible to everyone.

**A merged lane's branch disappears from the remote.** Lane C's
`claude/druid-shell` was gone from `git ls-remote` while the local
remote-tracking ref still named it, which reads exactly like a branch deleted
out from under finished work. It had merged as PR #435; `git fetch --prune`
then `git branch -r --contains <sha>` settles it in one command. Pin the sha
to a local branch *before* pruning, because the prune is what removes the only
handle you have if the answer turns out to be the bad one.

**Item 3 is a suspicion, not a confirmed bug**, and Lane C's brief says so: it
may be working and merely invisible, which wants a different answer than a
wiring fix.

**Lane D's brief named the wrong dial and was corrected in flight.** PR #432
(the lab's own pheromone lane) measured that **`pheromone::DECAY_RHO` is
inert** — set to zero, trail life does not move — and that the lever is
`DIFFUSE`, at 16.7% a pass against decay's 2.9%. The correction carries a
constraint the original brief did not: **the dial is per *channel*, not per
layer**, and the gnome lays on channel A, which is also the ants' road home.
So turning channel A's diffusion down to lengthen her trail reaches the ants
too, and the owner's split — her trail is ours, theirs is the lab's — is not
automatically satisfied by staying out of `DECAY_RHO`.

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
