# Lane: Plant community plan (thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB5iFuekoJWbiCr1frmZMiqp)

State as of 2026-10-27 18:40Z. Goal of the line: an understory and four plant
roles (tree, shrub, herb, creeper) that coexist in the lab bed. Design of
record: `Reports/plant-community-plan-2026-10-20.md` (§8 "Next, in order" is
the live status); data under `Reports/plant-community-2026-10-20/` (m0, m1, m2).

## Unmerged work
- None. Branch `claude/project-thread-uodokv` == origin/main (last PR #901,
  merged 0063b2f). No open PRs from this lane.

## Jobs running
- None (no VM or cloud jobs).

## Open to-dos
- Resume the understory plan: M2 rungs 5-6 (recruitment and long-run
  coexistence; the seed-trained tree floods the bed with seedlings, which is
  rungs 5-6's job). See plan §6.2 and the §8 item 5 paragraph.

## Owner decisions still pending
1. **Keep pushing on plant food for the ant colony, or return to the understory
   plan?** (asked 2026-10-25 08:27Z, no answer). Recommendation: return to the
   understory. The bred plant pool gave the colony no food back (8 rounds flat;
   12-seed judge: edible -13% vs species files, starvation roughly doubled,
   10 vs 12 colonies alive), recorded as a dead end (`Reports/dead-ends.md`
   other:164; retry only with much longer rounds or food scored from what ants
   actually ate). The pool code ships OFF (`PIXEL_PHYSICS_PLANT_POOL=1` turns it
   on; `src/sim/plant_pool.rs`).
2. **Ship the seed-trained shrub and herb?** (plan §8 item 5: in the 450k
   bed they keep 3.2 roles alive per seed vs shipped 2.2). Recommendation: yes,
   after one paired lab check with the colony, since lab-food effects reversed
   the pool's plant-only gain.
3. Older items listed in the plan's "Still open with the owner": patchy ground
   in the played lab (+7% frame), the creeper's lifespan / rooting ramets,
   turning lab-only plant rules on outdoors.

## Files outside the repo
- `/mnt/project-files/plant-community/` (20 MB): pictures, runner logs and
  runner scripts/patches. Checked: the data and READMEs are already in
  `Reports/plant-community-2026-10-20/m2/` (refit, hold, community, bedsel,
  rungs; m2-notes/README.md is identical to m2/rungs/README.md), and the
  patches were merged as code. Nothing further needs landing.
- Scratchpad: two throwaway plotting scripts and the HTML sources of the
  artifact pages "Lab Plant Pool" and bed-selection; their numbers are in
  `Reports/plant-community-2026-10-20/m2/bedsel/README.md`. Not needed.

## Working practices learned (not obvious from the repo)
- Paired arms must be run on the same build; an "off" arm reused from an older
  main drifted at seed 1.
- latecensus does not run the lab mister unless `mister=1`; lab-food judgements
  need it on.
- Plant pool founding happens at germination under the world seed, not at
  planting, because lab racks clone a planted box and reseed afterwards.
