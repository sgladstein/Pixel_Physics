# Brood carrying without flat rooms (BROOD_CARRY=on + BROOD_DEEP=on), heap 90, seeds 1-4, 300k (Nest race, 2026-10-07)

Build `claude/nest-race-two-crowds` 516a223c (dt-tc), FLAT_ROOM unset, on the WAY_FOOT=on stack (base = t90on runs, the
12-seed bar's on arm; identical with the new switches unset, seed 1 to 40k). Pictures: `pictures/`.

## Measured, base -> brood carry
    t90on-s1	ants 796	starved 24	fall 10.4%	deep-starvers 10	bites 52211	digs 73765
    bc90bc-s1	ants 483	starved 9	fall 2.4%	deep-starvers 0	bites 9015	digs 20566
    t90on-s2	ants 771	starved 9	fall 12.4%	deep-starvers 3	bites 51618	digs 65659
    bc90bc-s2	ants 478	starved 9	fall 2.2%	deep-starvers 0	bites 5024	digs 16550
    t90on-s3	ants 748	starved 148	fall 10.8%	deep-starvers 77	bites 49902	digs 65230
    bc90bc-s3	ants 598	starved 91	fall 1.9%	deep-starvers 57	bites 4762	digs 14277
    t90on-s4	ants 736	starved 269	fall 11.8%	deep-starvers 179	bites 42988	digs 58019
    bc90bc-s4	ants 579	starved 5	fall 0.5%	deep-starvers 0	bites 3154	digs 15166

- Brood in the nest at 300k 485-582 -> 1,814-2,159; eggs 6.2-6.5k -> 9.9-13.1k; larvae starved 0.8-1.1k -> 4.5-7.2k;
  births 4.8-4.9k -> 3.1-3.8k; colony at 300k 736-796 -> 478-598. Adult starving is LOW (5-91). Digs a quarter.
- Pictures: the brood is carried down off the door column into one heap that covers the floor of the room.

## The egg rate (the coordinator's ask: food is supposed to cap births)

The laying gate (`creature.rs` ~5440-5560, read): under LAY_BAR=body an ant may lay once its own body clears the egg
bar; then the brain's Lay output; then, at a nest, `brood::pile_site` must find an empty home cell within the pile reach
(4) of the layer, else the tick is counted in `buds_held_for_nest` and nothing is laid. That counter counts only ticks
that already passed the food/body check.

Eggs laid against held-for-no-site ticks, per window (stats.csv, measured):

| run | 20-50k | 50-100k | 100-200k | 200-300k |
|---|---|---|---|---|
| base s1 | 836 laid / 588k held (0.14%) | 908 / 1.74M (0.05%) | 1,983 / 3.11M (0.06%) | 2,257 / 2.70M (0.08%) |
| carry s1 | 1,073 / 156k (0.68%) | 2,370 / 1.62M (0.15%) | 4,442 / 3.48M (0.13%) | 5,005 / 2.95M (0.17%) |
| base s4 | 618 / 283k (0.22%) | 1,048 / 1.19M (0.09%) | 2,093 / 3.11M (0.07%) | 2,584 / 3.55M (0.07%) |
| carry s4 | 1,069 / 250k (0.43%) | 1,339 / 1.38M (0.10%) | 3,316 / 3.02M (0.11%) | 3,907 / 3.77M (0.10%) |

- **On this stack, food does not cap births; egg space does.** In both arms an ant that can afford an egg is held for
  want of an empty cell near it on more than 99.8% of those ticks. Carrying brood away from the door frees cells near
  where layers stand, so 1.5-2x as many affordable ticks find a site, and eggs go up 1.5-2x. Measured at the counter;
  per layer it is inferred (no broodlog in these runs: it was sent to /dev/null for disk).
- The larvae then starve (4.5-7.2k) because the extra brood is not fed, so births go DOWN while eggs go up.
- So brood carrying can't be judged until births are capped by food. That gate is the thing to fix first: the brood
  column has been the cap on laying.
