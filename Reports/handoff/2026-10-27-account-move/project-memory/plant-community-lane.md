---
name: plant-community-lane
description: Plant-community plan thread state (to 2026-10-25) — owner rulings, what shipped, what is parked, tree-age fix, bed selection, lab plant pool
metadata:
  type: project
  modified: 2026-10-23T21:13:23.537Z
---

Plant-community thread (plan: Reports/plant-community-plan-2026-10-20.md, branch `claude/project-thread-uodokv`, worktree /home/claude/shade).

- 2026-10-21 owner: accept the clone size spread, start M2. Parked off (dead-ends.md): A2 node keying, A3 laterals-by-node, collar sprout.
- 2026-10-21 owner: lab mister rains per column everywhere (#773). Role re-fits on rung 2 all grew 1.6-5x and erased trade-offs; not shipped.
- 2026-10-22 owner: SHIP the seed-trained shrub only (#814). Shipped creeper has NO lifespan; it dies of canopy shade, not age.
- 2026-10-22 owner chose "Fix the shade seedling": switches `World::seedling_reserves`, `World::seed_settles` (off, #830 merged). Stand floor at ground = 0.4-1.7% open light, below any shade gene. Herb stand rung NOT repeatable at 12 seeds.
- Owner chose thinner crowns: leaf_depth dial (#831, off) -> trees grow +21% leaf, floor dark again by 450k; dead end.
- 2026-10-23 owner: "either suggestion or both" -> chose gaps from tree death. FOUND: old age read `age_ticks*45` rolled over 45 frames; real tick span is 45*size_cadence and lab is half pace, so a lab tree lived ~7x its authored median (~415k frames). `World::old_age_plant_time` (off), `plant_m0 gaps=N oldage=1`, PR #838 to merge desk (head fc89e4a8). 8 seeds x 450k: today roles/bed 2.62, dark floor 32%; +clock+shrub 135k: 3.00, 5%, tree leaf 27%; +tree 120k: 2.88, 18%, tree leaf 70%. Creeper absent in all. Every life_half_life was tuned on the broken clock.
- Owner chose ON (trees 2x): PR #842 merged 9c32fcd -- `old_age_plant_time` default on, tree/conifer 120k, shrub 135k, env `PIXEL_PHYSICS_OLD_AGE_CLOCK=ticks` = old clock. Lab (latecensus played_bed, 12 seeds): ant food on bed -18%, colony unchanged. With roots_required (#841, on) too: dark floor 37->22%, shrub gain came from roots, roles 2.88->2.62; owner chose keep on (2026-10-24 00:33Z). Report PR #843 (docs).
- 2026-10-24 bed selection (lessons from ant colony training): `plant_m0 mode=bedsel` (save=/load= pool ron, pick=random control, common garden). Judge 16x450k: roles/bed shipped 2.56 -> pool 3.44, creeper alive 16/16. Pool Reports/plant-community-2026-10-20/m2/bedsel/pool-r8.ron. Owner default-on rule (2026-10-24): seed_settles + shade_scald ON; seedling_waits/reserves OFF (darken floor 17->25%). PR #876 merged.
- 2026-10-25 lab plant pool: #878 merged (LabBox::plant_pool, assets/plant_pools/roles4.ron, founding at GERMINATION under world seed -- planting-time founding made rack copies identical). Owner chose '12 more seeds': 24 paired seeds one build -> edible -24% (22/24), starved 139->218, extinct 3->6. PR #888 flips default OFF (PIXEL_PHYSICS_PLANT_POOL=1 on). Then bred on played_bed WITH colony (plant_m0 bedsel bed=played, kinds then food, 8x8x150k): food flat, judge (latecensus mister=1, 12 seeds) new pool = old pool; dead end other:164, PR #901. NOTE latecensus runs WITHOUT the lab mister unless mister=1. NEXT: back to understory plan unless owner says otherwise. Lesson: re-run BOTH arms when main moves (off arm drifted). Machine 4 cores; build only needed examples.
- Rootless surviving plants: separate thread (branch claude/project-thread-wefr10), `World::roots_required` may shift tree/understory numbers.
- Runner kit: /mnt/project-files/plant-community/runners/. Pictures under /mnt/project-files/plant-community/.

**Why:** a later thread needs to know what the owner already decided and what's parked.
**How to apply:** check whether the owner took up in-bed selection; continue M2 (rungs 5-6, understory). Related: [[merge-desk]], [[review-queue-retired]], [[rootless-plants-lane]].
