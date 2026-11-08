# Fault probe: is a running crack satisfying to cause, and can you read where it goes? (2026-11-07)

**Status: round 2 (2026-11-08) made the playtest review's five fixes; a
plan now beats random play (28 stars to 11 over 10 walls, p = 0.004). The
owner's verdict is pending, and so is whether one recipe -- a row of holes
with taps between -- makes the wall too easy.** Round 1 (below the round 2
section) is kept as it was measured.
Probe 6 of the prototype lanes from the 2026-11-07 new-game brainstorm
(brief: `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/6-fault.md`).
Play it with `cargo run --release --bin fault`. Harness: `examples/faultprobe.rs`.

## The game

An ice wall holds back a lake that rises on a clock (the thaw, 120 s). If
the lake reaches the crest, the wall bursts. The player cuts a spillway
instead, with four tools: **Tap** (start a crack; 10 a wall), **Drill**
(bore a hole that stops a crack; 5 a wall), **Meltwater** (pour water that
wets the cracks it reaches) and **Frost** (freeze wet cracks, which wakes
the dead crack tips that touch them and runs them on). Cut a piece free and
the lake shoves it off the crest when the water reaches its back; the lake
drains through the gap. A mill downstream must not be hit by ice or flooded.
Stars (round 2): one for no burst, one for holding the lake between the
safe line and the mill race, one for the mill dry and untouched. (Round 1:
the middle star was for draining at least half the lake.)

## Round 2 (2026-11-08): the playtest review's fixes

The demo playtest (`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/playtest-2026-11-08/fault-review.md`,
page https://claude.ai/artifact/E8re9eqAAKbggnRJM8oEK9) found that a plan
did not beat random, a careless deep tap under the waterline was the best
strategy, a fresh player burst 9 walls of 10, and nothing showed where a
crack would run. It ranked five fixes; all five are in.

1. **Score a band, not a drain.** A dashed **safe line** 15 rows under the
   crest (keep the lake below it) and a **mill race** at row 80 (keep it
   above, or the wheel runs dry). Stars: no burst; the lake held between
   the two; the mill dry and untouched. The gorge is wider (140..176), the
   mill stands on a bedrock terrace, and the river below the wall carries
   at most 200 cells a second (`RIVER_PER_SEC`). **The safe line alone was
   gamed** (random 14 stars, deep taps 24): once the mill was out of reach,
   emptying the lake was the safest play. The race line is what makes a
   deep cut cost a star.
   A crack started more than `DEEP_ROWS` under the surface does not tire
   while it is under water, so a careless deep tap runs on and drains far.
2. **Preview** (`fault::preview_tap`): hovering with Tap runs the tap in a
   trial copy (`trial` + `settle_cracks`) and draws the new edges as an
   amber line. It is exact (the same function the game then runs), refreshed
   when the cursor's corner changes or every 30 frames.
3. **Cut-free outline**: a severed piece pulses yellow, with a tick in the
   lake at the row the water must reach to push it (`tips::push_row`), and
   the hint line says so.
4. **Holes draw cracks in.** A tap within 10 cells of a hole aims a half at
   each of the two nearest (`tip.aim`), and any tip within `HOLE_PULL` (7)
   is drawn in, straight, with no fork and no tiring. Without the aim a half
   started eight cells off wandered back onto the grain first.
5. **A burst sheds the face as slabs**: the top 28 rows not already moving
   go as one slide, broken at the grain into blocks.

**How to play on screen** (after the owner's Storm and Mould playtests,
where the goal and the tools were the complaint): the game opens on a panel
with the goal, one line per tool and key, and one per star; the thaw does
not start until the first click or key, and `H` brings the panel back. A
test checks every character is in the HUD font and every line fits (red for
`&` and for an over-long line).

**Two bugs found on the way.** The engine's `paint_circle` is rounder than
the hole the cracks see (`r^2 + 1`), so a drilled hole kept four cells of
ice that cracks took for hole: a tip stopped one cell short, and a
hole-to-hole cut across wall 2 held its whole top on through that corner.
`drill` now clears exactly the marked cells. And `MAX_PIECE` = 1600 was a
gate, not a bound (a deep cut-off piece was "anchored" for being big); it is
5000 now. The planner's notch jammed on walls 2 and 4; the slide's grind and
lift now grow with each frame it is stuck.

### Results (10 walls, paired; same harness built against both trees)

The old build is scored by the new rules from the harness's own record
(burst, highest lake, final surface), so the rows compare like with like.
New better / worse / same is per wall; the sign test is two-sided.

| strategy | stars old -> new | burst | lake held | mill dry | new vs old |
|---|---|---|---|---|---|
| do nothing | 0 -> 0 | 10 -> 10 | 0 -> 0 | 0 -> 0 | 0/0/10 |
| random (a tool every 2 s) | 5 -> 11 | 5 -> 0 | 0 -> 0 | 0 -> 1 | 6/0/4, p 0.03 |
| planned (look-ahead) | 9 -> 28 | 7 -> 0 | 3 -> 9 | 3 -> 9 | 7/1/2, p 0.07 |
| fresh (scripted first game) | 2 -> 19 | 9 -> 2 | 0 -> 4 | 1 -> 7 | 7/0/3, p 0.02 |
| deep taps under water | 4 -> 14 | 6 -> 0 | 0 -> 0 | 0 -> 4 | 7/0/3, p 0.02 |
| drill only | 0 -> 0 | 10 -> 10 | 0 -> 0 | 0 -> 0 | 0/0/10 |

- **The deciding comparison, named before the run: planned against random
  on the new build: 9 better, 0 worse, 1 tied, p = 0.004** (old build 3/3/4,
  p = 1.0). Six before/after rows were tested, so the p of 0.02-0.07 on each
  is not individually convincing; 0.004 survives a correction for six.
- Deep taps never hold the lake (all 10 end below the race, and the lake never
  rose above row 92) and hit or flood the mill on 6 of 10.
- **One recipe does it.** The planner's winning move on every wall is a
  `stitch`: holes 16 or fewer cells apart from face to face along one row,
  a tap between each pair (3-5 holes, 2-7 taps). Whether that is a plan a
  player enjoys finding or a formula that empties the puzzle is the open
  question for the owner.
- Frame cost, single runs of wall 6 alone: doing nothing mean 1.17 ms on
  both builds, worst 3.5 -> 4.3 ms; planned mean 1.36 -> 1.14 ms, worst 5.6
  -> 7.1 ms.

Harness: `faultprobe plans=nothing,random,planned,fresh,deeptap,drillonly`
(the default table runs all six); `look=1` writes stills of the preview, the
two-hole stitch and a cut-free row (`row=`).

## Round 1 (2026-11-07)

## What is new and what is shipped

- **New, game-only** (`src/fault/`): a tension field (a screened relaxation
  fed by the lake's depth and the weight of ice above, which does not cross a
  crack), crack tips that walk the **ice's own grain** (joints between
  `fracture_field::domain` cells at scale 10) towards the most strained
  neighbour, fork when strained, and stop at a hole, a face or another crack;
  severance of a closed ring; the lake's shove (a cut piece slides along its
  crack, grinding the step corners to snow, and peels off the crest a grain
  block at a time); a support sweep that lets fall any ice nothing holds up.
- **Shipped**: the crack bits on `Cell`, the body code (`rigid.rs`) the
  blocks fall through and break on, water, the renderer and the HUD.
- **One shared edit**, additive: `rigid::throw_block`, a public way to hand a
  block of cells to the body code with a push, for a caller that decided
  severance itself. Nothing else calls it.
- **Per-world material edits** (the registry is owned by the world): ice
  melts at 400 rather than 4 (no weather here keeps it cold), is recoloured
  so a white crack reads, uses stone's fragment ladder, and water never
  freezes (a cold front skinned seed 3's lake with single ice cells).

## Results (10 walls, paired by seed; `faultprobe` with no plan)

| | burst | stars (total of 30) | runs with 2+ stars |
|---|---|---|---|
| do nothing | 10 of 10 | 0 | 0 |
| random (a tool every 2 s, 70% taps) | 5 of 10 | 7 | 2 |
| planned (look-ahead, staged notches) | 5 of 10 | 5 | 0 |

- **Doing nothing loses**, every wall.
- **Random loses to nothing it shouldn't, and the planner does not beat
  it**: planned against random by stars, 3 better, 3 worse, 4 tied; sign
  test p = 1.0. Each beats doing nothing on 5 walls with the rest tied
  (p = 0.06 each, two comparisons).
- **Before the tap stock**, random tapping (about 45 taps a game) kept the
  lake under the crest on 7 of 9 walls by dicing the wall down; the stock of
  10 halved that.
- No run earns three stars: every run that drains half the lake throws ice
  at the mill.

The planner (`Planner` in the harness) tries every tap in a band in its head
(`Game::trial` + `settle_cracks`) and keeps the one whose cracks best cut
across the wall at the target row. It stands in for a player who can read
where a crack will run, **which is the question the probe asks of a person,
not something this answers.** Its failure says a straight look-ahead on
crack geometry does not plan this level well; it does not say a person
cannot.

## Repeatability (`faultprobe repeat=1`)

One tap at the same spot and moment:

- **The same wall twice: identical**, edge for edge (48 edges).
- **Ten walls**: the crack stays near where you tapped (its rows span 42..76
  for a tap at row 60) but turns a different way on each wall, following
  that wall's grain. Overlap with wall 1's crack is 0-6%. Only one of ten
  crossed the wall. So "roughly where" is readable; "which way" is not, from
  the ice alone. That is what the shimmer (`S` key) is for.

## Frame cost

Single runs of seed 6 with nothing else running (`faultprobe
plan=nothing|planned seed=6`), one game step (tension, tips, slides, support
sweep and the shipped `frame::step`) at 256x160, drawing excluded:

| | mean | worst | worst within a second of a slab breaking |
|---|---|---|---|
| do nothing | 1.42 ms | 6.8 ms | (no slab broke) |
| planned | 1.83 ms | 11.4 ms | 11.4 ms |

The four-wide table runs, contended with each other and a build, saw worst
frames up to 44 ms; treat those as noise. The heavy frames are slab breaks
(many bodies at once) and the support sweep's flood, every 6 frames.

## Tried and rejected along the way

- **Handing a closed ring to the shipped load model**: a ring inside the
  wall reported "confined" every 5 frames (43 failures, 2,924 cells over 240
  frames) and released nothing; one at the crest "crumbled" to snow. Severance
  is decided in `tips::find_severed` instead, and crack cuts schedule no
  structural check.
- **Releasing a piece with no push**: the body code wrote it back where it
  was, crack bits lost (192 cells back in place 30 frames later).
- **Throwing the piece** (a heave out of the wall): 1.6/0.9 fell back and
  dammed the notch; 2.6/1.4 and 2.2/1.3 landed slabs on the mill. Replaced by
  the slide along the crack.
- **Dropping the whole slab once half overhung**: the upstream half fell
  onto the stub and welded into a lump. **Toppling it at its centre of
  mass**: ten blocks fell together onto the crest. Replaced by per-block
  peeling.
- **Dropping a jammed slab's remainder as resting bodies**: they set on a
  neighbouring slab that then slid away, leaving lumps hanging off the crest
  for the rest of the game. Replaced by `tips::support_sweep`, which also
  needed: a floe on water is held (one was re-dropped 177 times); a lump the
  body code puts back is trusted for 2 s; a lump wedged by its sides (the
  body code cannot drop it) is let down by hand.

## Open

- ~~Make a plan beat random~~ (round 2: done by the level -- safe line,
  mill race, terrace -- and holes that pull cracks in).
- Does one recipe (a row of holes, taps between) make it too easy? If so,
  walls where it cannot span: a seam holes cannot bridge, or fewer drills.
- Cracks curl into hooks on some walls (wall 1's repeat crack): the step rule
  prefers joints over tension strongly enough to wander.
- The shimmer is faint; the blind A/B on the page asks whether it helps.
