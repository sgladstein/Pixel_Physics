# The mould game: an independent review for fun and looks (2026-11-10)

A review, not a build. Scott, 2026-11-10 01:41Z: *"review our slime mold game
and give suggestions on how to make it fun to play and look good"*, then
*"Review game design, philosophy and existing games that may have similar
mechanisms or themes"*. Nothing in `src/mould/` was edited.

Pictures, counts, the ranked lists with pros and cons, and the table of
comparable games: https://claude.ai/artifact/W4EqSvxDWK1RC9YgpSQPTR (source
[`mould-review-2026-11-10/page.html`](mould-review-2026-11-10/page.html); the
pictures are on the page, not in the repo).

**What was reviewed.** The mould thread's branch `claude/project-thread-cek3ly`
at `69df5a1` (real food flow, the T panel, 22.5 degrees and the bait patches as
switches), which was not on `main` when this was written. The build and its
measurements are in [`mould-probe-2026-11-07.md`](mould-probe-2026-11-07.md).

**How it was played.** Through a scratch harness that plays a dish from a
written hand (one action a line: time, tool, place) and writes the window's own
frames (`render::draw`), looked at every 5 s before choosing the next move. It
can judge decisions, pacing and looks. It cannot judge mouse feel or sound.
The harness is [`mould-review-2026-11-10/mouldhand.rs.txt`](mould-review-2026-11-10/mouldhand.rs.txt)
(it needs the branch above to compile, so it is kept as text, not in
`examples/`), with the hands played beside it. The Wanderer and the Organ were
not replayed.

Each claim is marked **played** (done once, one dish), **measured** (8 seeds),
**earlier** (the mould thread's own tests) or **memory** (known about another
game, not looked up).

## Summary

1. **The mould is a good toy and a thin game, and its two settings each hold
   half of what is wanted.** At 45 degrees it obeys every oat and then stops
   moving. At 22.5 degrees it never stops moving and does not need the player,
   and what the player does is lost in the mould's own rewiring.
2. **Recommended, in order:** (a) make the goal "keep it alive" on a dish that
   does not end; (b) give the player marks that stay on the dish (lights set
   down and lifted, salt drawn and rubbed out); (c) draw the real walkers and
   thin the tubes.
3. **The owner's pick is pending** (decision card in the review thread,
   2026-11-10).

## What playing showed

**Free dish seed 1, shipped defaults (45 degrees). Played.**

| | Oats | Lamp | Salt, snip | Joined before storm | After storm | Network / shortest | Stars |
|---|---|---|---|---|---|---|---|
| Do nothing | 0 | 0 | 0 | 4 of 6 | 2 | 1.13 | 0 |
| Game 1 | 8 | 4 s (a test) | 0 | 6 of 6 | 3 | 1.06 | 2 (join, short) |
| Game 2 | 9 | 0 | 0 | 6 of 6 | 6 | 1.28 | 2 (join, storm) |

- Three oats laid in a row at 6, 8 and 10 s joined the far pile by 15 s. There
  was then nothing to decide until the next pile landed at 30 s. Every move in
  both games was the same move: the next oat about 20 cells further along a
  line.
- Game 1's loop never closed: oats go off after 35 s and return one per 15 s,
  so the first stepping stones had gone before the last arrived, and the
  half-built arms pulled back. Game 2 timed it and the closed ring kept all six
  piles through the storm, at the cost of the "short" star. So there is a small
  skill, and a real trade between short and tough. **A closed loop between
  piles does hold at 45 degrees**: after game 1 it looked as if the mould
  deletes loops, and game 2 showed that was the oat timing.
- The lamp held on a tube for 4 s made a dent that was straight again about
  1 s after letting go. Salt and snip were never needed.

**Free dish seed 1 at 22.5 degrees. Played.** Untouched, it joined 6 of 6 by
56 s and scored 1 star (the mould thread measured the same on 6 seeds and 7
hands: earlier). A lamp swept through a tube for 6 s bent it into a clear V;
6 s after, the network had rewired into a shape the sweep did not choose, as it
does untouched. A light held in one place for 60 s left the network built
round it (one seed; the difference from the untouched dish is modest).

**Running Dry at both settings. Measured, 8 seeds** (`mouldprobe level=3
sensor_angle=A seeds=1,2,3,4,5,6,7,8 players=idle,random,intended`).

| | 45 | 22.5 |
|---|---|---|
| Do nothing, alive at the bell | 0 of 8 | 2 of 8, with 1 and 2 walkers |
| Do nothing, died at | 1:05 1:17 1:09 1:04 1:13 1:07 1:02 1:24 | 1:50 1:47 1:50 (bell) (bell) 1:45 1:45 1:44 |
| Random hand, alive at the bell | 1 of 8 (seed 7) | 6 of 8 (not seeds 3, 5) |
| Scripted plan, alive at the bell | 8 of 8 | 8 of 8 |
| Scripted plan, mould alive | 400 411 373 374 354 358 374 374 | 397 405 394 404 405 409 389 435 |

Paired by seed: the untouched mould lasted longer at 22.5 on 8 of 8 (sign test
p = 0.008); the random hand lived at 22.5 and died at 45 on 5 seeds, the
reverse on 0, level on 3 (p = 0.06). Two comparisons; only the first survives a
Bonferroni correction. **Doing nothing loses at both settings.** The weak spot
is the random hand at 22.5, which lives on 6 of 8 with up to 16 oats and ends
with more mould than the scripted plan (331 to 723), so aim does not yet matter
enough there.

**Looks. Played.** The frame draws a softened, thresholded trail as gel; the
walkers are not drawn. Tubes are about 12 cells wide on a 256-cell dish. With
`flow` on, fed tube is a flat bright yellow and hungry stretches are grey
smears; it reads worse than with flow off. Sensor reach 6 (shipped 9) gives
thinner tubes and a denser web on the same seed; reach 4 joined 1 of 6. At
scale 3 the bar's "Storm in" prints over the mould count.

## Fun, ranked

1. **"Keep it alive" on a dish that does not end.** Food runs out and turns up
   further away; storms and dry spells are weather; the score is how long it
   lived and how big it got, beside the best so far. *For:* doing nothing lost
   at both settings; nearly all built (piles that run out, per-piece food,
   graded starving); a creature that can die is a reason to care; always a next
   decision; the outcome is an amount. *Against:* not shown fun past two
   minutes; a random hand survives 6 of 8 at 22.5, so oats must be scarcer or
   food harder; needs a difficulty ramp. *Cheapest test:* Running Dry at 22.5
   with no bell, about 6 oats, each pile further; pass if doing nothing dies,
   random dies on most seeds and a better plan lives longer.
2. **Marks that stay.** A few lights set down and lifted, more salt, both
   rubbed out by a click: the player shapes the dish and the lively mould keeps
   showing it. Tero et al. 2010 steered a real mould with an illumination mask
   for mountains and sea. *For:* keeps 22.5 and restores a visible
   consequence; gives lamp and salt a job. *Against:* one small light is weak
   (one seed); too many marks makes a wall-drawing game; limits need tuning.
3. **Oats as a nudge, not a ruler.** Fewer, earned by finishing a pile, or
   weaker so the mould leans. *For:* the mould surprises again. *Against:* the
   owner liked pulling with oats; too weak and the mould ignores the hand (the
   complaint against Mushroom 11: earlier). Only with 2.
4. **Score an amount and keep the best**, with the do-nothing result beside it.
5. **Keep the three challenge dishes as a side pack at 45 degrees.** They pass
   their bar there and fail it at 22.5 (earlier). One-idea puzzles; Beat the
   Ruler is the best because its answer is a join in empty space.
6. **Stop:** the keep-off bait patches (no reason in the dish, and the smell
   traps the mould: earlier); the storm as a final exam (the result comes when
   the player can no longer act); the "short" star at 22.5 (the oracle pruner
   could not reach it: earlier).
7. **Out of the dish: wait** until the dish game works
   ([`mould-real-world-2026-11-09.md`](mould-real-world-2026-11-09.md) found a
   single runner side-on; the networks are the fun).

## Looks, ranked

All draw what the simulation already holds.

1. **Draw the walkers.** A speck per walker gives tubes a living grain and
   shows crowding at food; it does honestly what the removed drawn pulse tried
   to do. Mocked by painting a dot per walker over the game's frame. Cheap;
   needs a pass on dot size and a phone-sized check.
2. **Finer mould.** Sensor reach about 6. Free in frame time; every number
   tuned to 12-cell tubes (`TUBE_WIDTH`, the lamp radius, the thrift bar) needs
   re-deriving, and reach 4 is below the floor.
3. **One colour ramp for flowing food**, white-gold at the food through amber
   to dull brown, with hunger thinning and dimming the tube rather than turning
   it grey. Settle together with the pending grey / embers / dried-slime pick.
4. **A stain where the mould has been** (the real organism leaves slime it
   tends not to re-cross: memory). Mocked from cells that have ever held tube.
   At 22.5 the mould visits most of the dish by 85 s, so it must fade.
5. Smaller: the lamp reads as a lens, not light; the bar overlaps at scale 3.
   Sound was not judged.

## Design writing and comparable games

In full on the page. The points that drove the ranking:

- **Lens of the toy** (Schell: memory): the mould is fun to play with before it
  is a game; keep goals light.
- **Intention and perceivable consequence** (Church 1999: memory): 45 has both
  and no life; 22.5 has life and loses the second. This is why the fix proposed
  is on the tools (marks that stay), not on the goals.
- **Kinds of fun** (Hunicke, LeBlanc, Zubek: memory): the mould is strong at
  sensation, discovery and expression; the work since the first play added
  challenge (stars, bars, storms, patches).
- **Indirect control** weakens under unpredictable randomness
  (https://math.vu.nl/~eliens/media/pattern-indirectcontrol.html).
- Games: Mini Metro (a network never finished: the shape of 1), From Dust
  (shape the ground, not the creature: the shape of 2), Prune, Mushroom 11,
  Lemmings and Pikmin, World of Goo, Creeper World, Townscaper, Opus Magnum,
  PHYSARUM and Wetware. A search on 2026-11-10 still found no game that puts a
  goal on a real self-growing slime mould simulation.

## What this does not show

Nobody held the mouse. Most play findings are one dish played twice; only the
Running Dry table is 8 seeds. The three mock pictures are dots and a stain
painted over real frames from real simulation data. Nothing was built, so
there is no dead-ends entry.
