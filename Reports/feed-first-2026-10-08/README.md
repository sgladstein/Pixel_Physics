# FEED_FIRST: no egg while a starving larva is in reach (proposal, 2026-10-08)

**Status: proposal for review; nothing built.** The owner's second birth brake
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
