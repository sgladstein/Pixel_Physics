# FEED_FIRST: no egg while a starving larva is in reach (2026-10-08)

**Status: built, shipped off, and inert: the larvae that starve are 30+ cells
from where eggs are laid** (the result is [below](#measured-it-almost-never-fires-because-starving-larvae-are-far-below-the-layers)).
The original proposal follows unchanged. The owner's second birth brake
(2026-10-08: *"feed before you lay"*), after the boom-and-bust trace
([`../boom-and-bust-2026-10-08/README.md`](../boom-and-bust-2026-10-08/README.md))
and the appetite dial's first results
([`../birth-brakes-proposal-2026-10-08.md`](../birth-brakes-proposal-2026-10-08.md)
once PR 661 lands). Measured unless marked *inferred*.

## The first check: do layers ever stand within reach of a larva?

The 2026-10-07 egg-cap trace found eggs landing at the top of the door shaft
and larvae at its bottom, 10+ rows apart, and the brakes proposal named that as
the risk that would make a reach-limited rule inert. Measured on the owner's
playtest switches (`boom_bust`, the four `play` runs and two `edible` runs of
2026-10-08), with [`laydistance.py`](laydistance.py): for every egg (its first
`laid` row only; later ones are re-appearances), the parent's position at the
lay against the larvae in the nearest earlier `brood.csv` snapshot.

| run | eggs (stats `eggs_laid`) | median distance | larva within 2 | 4 | 6 | 10 | 15 |
|---|---|---|---|---|---|---|---|
| play s1 | 643 (649) | 6 | 11% | 31% | 51% | 81% | 97% |
| play s2 | 713 (719) | 4 | 19% | 50% | 77% | 94% | 100% |
| play s3 | 717 (724) | 4 | 18% | 61% | 85% | 98% | 100% |
| play s4 | 697 (704) | 4 | 20% | 62% | 86% | 97% | 98% |
| edible s1 | 656 | 5 | 12% | 40% | 66% | 92% | 98% |
| edible s2 | 736 | 5 | 17% | 45% | 69% | 94% | 100% |

**The risk does not hold on this stack.** Layers walk home to lay (`LAY_HOME`,
on), and at home a larva is a median 4-6 cells away; 51-86% of eggs are laid
with one inside `NURSE_SCENT_REACH` (6). Positive control: the eggs found match
`stats.csv`'s count to within 1%.

## Why "hungry" has to be "starving"

`brood::nearest_hungry_larva` and `larva_scent` call a larva hungry while it is
short of its pupation target, and every larva is (it pupates on reaching it).
So "no egg while a hungry larva is in reach" at reach 6 would hold 51-86% of all
eggs whatever the food: a birth *stop*, the same failure as the appetite dial
at 1.0. The cue has to say *food is short*, not *brood exists*.

**A starving larva** is one whose bank has fallen below a fraction of the egg it
hatched from (`egg_cost`, 120 J). Unfed, a larva burns `larva_upkeep`
(0.0083 J/frame), so 120 J lasts about 14,400 frames; one below half of it has
gone about 7,000 frames without being fed enough to climb. A larva being fed
climbs toward its ~1,180 J target and is never in that band. That is a local
signal (the larva next to the layer, not a colony count) and it reads the one
thing the boom-and-bust trace says arrives too late: brood not being fed.

## The rule

In `creature::try_bud`, after the affordability check and the brain's `Lay`
output (so it counts only eggs that would otherwise be laid), and before the
egg site is sought:

- If a larva of the layer's colony within reach `R` of its head has a bank
  below `frac x egg_cost`, the ant does not lay this tick. Counted as
  `feed_first_held` (stats and `deeptrace`).
- **Switch:** `PIXEL_PHYSICS_FEED_FIRST=off|on|hold[,reach=R][,frac=P]`, off by
  default, `on` = `hold` at `R = NURSE_SCENT_REACH` (6), `P = 50`.
- **No new feeding code.** A held layer standing beside the starving larva
  already feeds it: `brood::nurse` makes the richest kin adult on the eight
  neighbouring cells give 0.25 of its surplus over 200 J each larva tick. A held
  layer two to six cells away does not (*inferred*); a `seek` part (turn toward
  the starving larva through `NurseSeek`) is the follow-up only if `hold` alone
  shows held layers that never feed.
- **Hunger safety:** it vetoes laying only. It never stops eating, walking to
  food, or anything a hungry ant does (rule audit's "no veto without a floor"
  does not apply: laying is not a need).
- **Biology** (to check before citing in code): workers and queens in
  food-short colonies feed brood before more eggs are produced (Cassill and
  Tschinkel 1995; Orlova et al. 2020, both cited in the 2026-10-07 plan §6).

## How it is tested

- Off arm byte-identical to the base (`scripts/deeptrace_tools/identity.py`).
- `steady_income` at 40 cells per 1,000 frames (a colony of ~200-300), the
  owner's playtest switches, 4 seeds, 200k: off / `hold` / `hold` with the
  appetite dial at its chosen weight. Read first on its own metric:
  `feed_first_held`, larvae starved per egg, eggs per J; then the colony's swing
  (min/mean/max after 100k) and near-collapses.
- Also on main's shipped defaults, since a default changes both.
- Counter sanity: `feed_first_held` must be 0 when no larva is starving (a box
  with food beside the brood) and non-zero when food stops (the `boom_bust`
  end), the positive control for the cue.

## Questions for the review

1. Is `frac x egg_cost` the right starvation line, or does a larva's bank
   fall below it for reasons other than neglect (e.g. right after hatching, or
   a larva that ate part of its egg's worth)?
2. Does holding the egg strand the rich layer's surplus (it neither lays nor
   feeds if the larva is 3+ cells away), and does that matter given the
   appetite dial keeps surplus in crops anyway?
3. Is there an existing rule that already reads larval starvation and would
   double up with this (`FOOD_BRAKE`, `NurseStay`, `CropNurse`)?

## Second review (2026-10-08, a review subagent standing in for Deep trace): build with changes

Its findings, checked against the code it cited:

- **The cue is sound but late and narrow.** A larva starts at exactly
  `egg_cost` (`lay_egg`), only `larva_upkeep` drains it, and every feed adds,
  so a fed larva never sits below the line. Unfed, it crosses 50% about 7,450
  frames after laying. A larva fed once and then abandoned starts from higher
  and crosses much later. So the cue reads "never fed", not "food is short
  now". frac must stay well under 100, or every new larva trips on its first
  tick: clamped under 90.
- **Placement:** before the egg site it would count ticks the breeding regime,
  the food brake or the nest gate would have held anyway, and would change
  those counters between arms. **Moved to after the food brake's return.**
- **Stranding at reach 6:** a held layer 3-6 cells from the larva does not
  feed it (`nurse` is the eight neighbours), so every layer near it holds until
  the larva dies, a local birth stop. At reach 2 the hold mostly resolves
  itself through `nurse` within one larva tick. **So reach 2 and 6 are both
  tested**, and `seek` comes forward if reach-6 holds commonly end in a
  starved larva.
- No existing rule reads larval starvation; `FOOD_BRAKE` reads income
  against burn, and `nurse_seek`, `larva_scent`, `NurseStay` and `CropNurse`
  read shortfall against target and only move ants.
- Off is byte-identical if the switch is read once and the counter only moves
  when on: built that way (`OnceLock`, no draw, no scan when off).

**Changes taken:** placement after the food brake; `egg_cost` read from
`brood_of` (env applied); frac clamped under 0.9; reach as a part (`reach=2`
and the default 6 both tested). **Not taken yet:** per-hold length and
fed-or-died counters. The first test reads them from the logs instead
(`feeds.csv` donors against `feed_first_held` and larvae starved), and they
are built only if that is ambiguous.

## Measured: it almost never fires, because starving larvae are far below the layers

`steady_income` (40 cells per 1,000 frames), the owner's playtest switches
plus `edible`, 200k frames, seeds 1-4, built on main `6c4bbabc`
(`2c7dd4d2`). **Identity:** the off arm's `stats.csv` matches the earlier
`playedible` runs column for column on all four seeds (the new
`feed_first_held` column aside), and a `dig=1` rerun of seeds 1 and 4
matches its no-dig run row for row.

| reach | seed | eggs held (ticks) | colony min / mean / max after 100k | eggs | larvae starved | adults starved (surface) |
|---|---|---|---|---|---|---|
| off | 1 | 0 | 221 / 269 / 350 | 2,038 | 142 (7.0%) | 424 (380) |
| off | 2 | 0 | 225 / 268 / 304 | 1,948 | 121 (6.2%) | 387 (341) |
| off | 3 | 0 | 244 / 272 / 293 | 1,677 | 6 (0.4%) | 191 (171) |
| off | 4 | 0 | 243 / 275 / 304 | 1,880 | 131 (7.0%) | 198 (195) |
| 2 | 1-4 | **0 on every seed** | identical to off | | | |
| 6 | 1-3 | **0** | identical to off | | | |
| 6 | 4 | 188 | 222 / 266 / 309 | 1,799 | 37 (2.1%) | 273 (217) |

Three of four seeds never hold an egg at reach 6, and none at reach 2,
while 6-142 larvae starve per seed. **Why, traced per larva**
([`starvers.py`](starvers.py) over `broodlog.csv` from `dig=1` reruns of the off arm): a
larva on upkeep alone spends its last ~7,200 frames below the line (60 J),
so for each larva that starved, every egg laid in that window and how near
its layer stood.

| seed | larvae starved | where they starved (y; ground 160) | where eggs are laid (layer's y; zones) | nearest egg in the starving window | eggs within 2 / 6 | rows from laying to starving | moves that were one-row drops |
|---|---|---|---|---|---|---|---|
| 1 | 141 (stats 142) | median 199 (179-222) | median 157; 65% mound top, 29% inside the mound, 6% nest | median **31** cells, p10 20 | **0 / 0** | median **38** (p10 29, p90 48) | 3,328 of 3,839 |
| 4 | 128 (stats 131) | median 202 (159-230) | median 157; 63% / 31% / 6% | median **31**, p10 18 | 1 / 61 (2 larvae) | median **40** (29-56) | 3,237 of 3,794 |

- **The larvae fall.** An egg is laid at the top of the brood column under
  the door, about the old ground line; brood is a powder, and 85-87% of a
  starved larva's moves were one-row drops. It starves about 40 rows down,
  in the deep nest, where layers never stand: eggs are laid 94% on or in the
  mound. A larva in its last 7,000 frames never had an egg laid within 6
  cells of it on seed 1, and on seed 4 only two did (both near the surface).
- **Positive control:** seed 4's two reachable starvers are the 188 held
  ticks reach 6 recorded there, and the one egg within 2 cells matches reach
  2's zero (the window is approximate; the hold needs the layer there at the
  tick). Seeds 1-3, where the reader finds none, are the seeds that never
  held. The starvers found match `stats.csv`'s `larvae_starved` to within 3.
  (A starved larva leaves an empty cell, not a corpse, in this log: 145
  larvae `gone` to empty against stats' 142 on seed 1.)
- So the census above (a larva a median 4-6 cells from each lay) was true and
  beside the point: the larvae near a layer are the new ones, being fed; the
  starving ones have fallen out of anyone's reach.

**Verdict: stays off.** A local cue cannot reach the larvae that starve, and
a reach of 30+ cells is no longer a local cue (it would read the whole brood
column, a colony count in disguise). The finding itself is the useful part:
**on this stack larvae starve because they fall away from every rich ant**,
the same disconnection the 2026-10-07 egg-cap trace found from the layers'
side (`handoff/2026-10-07/egg-cap/handoff.md`: 69-90% of eggs at the top of
the door shaft, larvae at its bottom). What would make a feed-first rule
able to act is brood that stays where it is laid, or layers (or feeders) that
go down to it: the "ants live in the nest" problem (plan step 3), not a
birth brake. *Re-test when* layers lay within a few cells of where larvae end
up (brood that does not fall, or a resting place deep in the nest).

Not taken further: `seek` (turning a held layer toward the larva) would
have nothing to turn toward at reach 6 on three seeds of four.
