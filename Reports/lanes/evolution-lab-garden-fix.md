# Round 28, second half — the pip is not spoil, and the reach the scene needs

*Follow-on to `Reports/lanes/evolution-lab-garden-loop.md`. Two ordered
builds; both measured, one shipped, one kept as a rejected reference.*

## Build 1 — the pip at the nest is not spoil

**Diagnosis, before any fix.** The traced pip (round 28, organism 13116)
died five frames after delivery. The bite verb could not have taken it:
`adjacent_food_counted`'s own gate (`gain <= EAT_YIELD_THRESHOLD { continue
}`) reads `diet_yield(pip, neutral) == 10 < 12` and never lets a neutral gut
select one as a target — read at the source, not assumed. The dig verb's
own `ground` test (`!matches!(kind, Creature | Plant)`) does not exclude a
live seed: `pip`/`windfall` are `Powder`-kind with a low
`penetration_resistance` (so a gut that *can* eat one still can — armour is
a bite-side concept, not a scene-wide one), which reads exactly like dirt
to the dig branch. So the dig verb was clearing a standing pip as spoil
with no call to `seed_survives_bite` at all, and no counter anywhere saw it
happen.

**The fix**, at the dig dispatch site (`act`, `src/sim/creature.rs`), which
already holds the `Cell`: skip a cell that is organism-owned `CellType::
Seed` (`live_seed = target.organism_id() != 0 && organism::cell_type(target
.aux()) == Some(CellType::Seed)`), counted at `World::dig_diverted_seed`.
The bite path is untouched — a gut that drifts enough to clear the
threshold can still eat a pip, and that still counts as `pips_eaten`.

**Positive control**: `a_dig_only_ant_does_not_clear_a_live_seed_standing_
in_its_path` (`src/sim/creature.rs`) — a dig-only ant with a live pip as
its only reachable target. Put the fault back (removed `!live_seed` from
the guard) and watched it go red (`digs` 1 against an asserted 0) before
trusting the green.

**Measured, 3 seeds x 120,000 frames, played_bed:**

| seed | dig_diverted_seed | pips_eaten | pips_rotted | seeds_delivered | pip_checks | plants_from_pip |
|---|---|---|---|---|---|---|
| 1 | 6 | 0 | 0 | 0 | 0 | 0 |
| 2 | 22 | 1 | 0 | 1 | 0 | 0 |
| 3 | 26 | 0 | 3 | 3 | 0 | 0 |

**The counter fires on every seed — the mechanism was real and is now
closed.** `pips_eaten` fell from round 28's 0/6/0 to 0/1/0: seed 2's six
delivered-and-eaten pips are gone from that exit, and the one that remains
is a genuine bite (the only path that can reach `pips_eaten` at all).

**`pip_checks` is still 0 on every seed.** No pip has yet survived past
delivery long enough for even one `Behavior::Germinate` evaluation, so
there is no threshold row to report — hypothesis (a) remains untested, not
refuted, exactly as before the fix. The new dominant cause, once dig
stopped eating them: **decay racing the Germinate schedule.** Seed 3
delivered 3 pips and all 3 rotted (`pips_rotted`) rather than reaching a
check; `deliver_seed_passenger` settles the whole carried span's viability
in one closed-form roll *at the moment of delivery*
(`half_life_chance(seed_half_life, frames_carried)`), and seed 3's
transit ran long enough (median 1,020 frames this run, against round 28's
sub-200-frame norm) that the roll had real odds of failing on arrival,
before the cell had stood for a single tick. Per the brief: reporting this
and stopping here rather than adding a second mechanism (delaying decay
relative to the Germinate schedule, or similar, is a design question for
the next round, not a fix this one owns).

**Note on comparing the two runs**: this is not an isolated-variable A/B.
Diverting a dig changes the world from the very first tick the situation
arises, and this is a chaotic system (`CLAUDE.md`: twelve identical trees
from one genome span 31-153 cells) — `windfall_bitten` and `seeds_spilled`
moved a lot between the two runs (e.g. seed 2: 9 spilled -> 1) because the
whole trajectory diverged, not because the fix directly suppressed bites.
The clean, causally-isolated reading is `dig_diverted_seed` itself (proves
the mechanism fired) and the guard test (proves it fires correctly on a
controlled scene).

## Build 2 — windfall in reach of the colony (scene arm)

**Change measured**: `assets/lab_scenarios/played_bed_windfall_reach.ron`,
a copy of `played_bed.ron` with the two inner scramblers moved from
columns 118/395 to 170/340 — ten and fifty-five columns further outside
the colony's nominal 180-330 founding band, on the reasoning that a
scrambler *inside* that band already measured 2-of-52 founders against 31
on the bare gap (`played_bed.ron`'s own comment).

**Measured against main's own played_bed, 3 seeds, frames 6,100 and
30,000, same binary:**

| seed | founders at arrival (main) | founders (variant) | windfall_bitten @30k (main) | windfall_bitten @30k (variant) |
|---|---|---|---|---|
| 1 | 40 | 27 | 1 | 0 |
| 2 | 31 | 23 | 0 | 2 |
| 3 | 23 | 22 | 0 | 0 |

**Both ship conditions failed.** Founders dropped on all three seeds under
the variant, and seed 3's 22 sits one below main's own floor (23) — "holds
within main's range" fails even at the edge, not by a wide margin.
`windfall_bitten` rose on 1 of 3 seeds, not a majority (and at a
30,000-frame budget — a quarter of the round's usual 120,000 — the counts
are small enough that noise is a live concern before trusting either
direction).

**Reading**: the colony's actual founding/foraging footprint is wider than
the rectangle named in `played_bed.ron`'s own comment. Ten columns of
clearance was not enough; the scrambler at 170 still reaches into
whatever `found_colony_of`'s real station placement uses. `played_bed.ron`
is unchanged. The variant scene is kept as a reference the way
`played_bed_scrambler.ron` already is (not deleted) — full account and
re-test condition in `Reports/dead-ends.md`.

## Card

If a plant had come up from a pip, the card would show the ringed
set-down-to-seedling sequence with `plants_from_pip` in `meta`. None did —
`plants_from_pip` is 0 on all three seeds in both builds — so the card
instead shows the pip surviving *longer* than round 28's traced example
(seed 3, this build: delivered and still standing well past frame 5, where
round 28's seed 2 example was gone) with the ring on its own cell,
`pip_checks` (still 0) in `meta`, and one line on what still stops it:
decay racing the Germinate schedule, not predation.
