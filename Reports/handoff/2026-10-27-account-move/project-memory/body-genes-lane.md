---
name: body-genes-lane
description: Body genes in in-run training (thread started 2026-10-22 13:35Z): appetite gate as gene slot 28, body= switch in the bed, which genes a mind ant actually reads
metadata:
  type: project
  modified: 2026-10-22T13:52:08.159Z
---

Owner 13:34Z 2026-10-22: "The body should be able to mutate too." Opus thread "Body genes in training" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBJszhyE7kpUzR8QoiHEK7L1).

- **Branch** claude/project-thread-fnf43q = compute branch 98ad9dc4 + db1cc976 (engine: TRAIT_APPETITE slot 27, CREATURE_TRAITS 28, creature::mutate_traits split out of mutate_newborn, creature::inherit_body; all species files 28-tuples, width 0) + a4a59972 (examples/inrun.rs `body=LIST`, jobs body1022). Off = byte-identical to 98ad9dc4 (checked seeds 1,2 at 200k, gate off and hungergate=1).
- **Engine gene MERGED to main as PR #827 (2a67f6d, 2026-10-22 ~17:05Z)**; the colony panel leaves the appetite slot off, as walk/door slots; branch claude/project-thread-fnf43q now = main + engine gene. The bed switch (scratch, compute branch only) is in /mnt/project-files/body-genes/patches/ 0001-0004 against 98ad9dc4 (apply all four).
- **What a mind ant reads (measured):** 17 of the 27 old genes are inert for a mind ant: birth_grant, reproduce_at (bed breeds by the harness), sight_range, curvature_radius (a mind casts no rays, unbilled), all 11 walk/trail-plane/return-memory genes and both door genes (read only by the game ant's chooser/act). Opened at width 0.5 they changed nothing, byte-identical. So "behaviour genes overlapping the brain" do not exist for the mind ant: they are dead.
- Live: gut_bias, pace, dig_force, digest_rate, crop_capacity, armour, + appetite (needs digest_bar=3 in the bed). body=default = appetite, digest_rate, crop_capacity, gut_bias, pace. Left out: armour (billed, never bitten in bed: one-way), dig_force (ant 1.0 vs packed soil/spoil 0.95: one step down = can't cut spoil).
- The bed's ant body is the lab's (gut -0.8, digest 0.2, pace 0.21...), not ant.ron's.
- Bodies carry between rounds as final/ID.body.txt; body.csv per round.
- Pilot 8x6 (results /mnt/project-files/body-genes/results.md): fixed gate ~6x food home (8/8 seeds); gate gene from off creeps only to ~0.15 in 6 rounds, no food gain; gate on + genes ~= fixed gate. Pace climbs fast (+0.2 -> +0.6..0.8) and costs ~1.5x frame time -> dropped from body=default (f8b00b7a); default now appetite, digest_rate, crop_capacity, gut_bias.
- Batch body-1022 (my VM pixel-train-body-1022, c4d-highcpu-64, started 14:36Z, ref f8b00b7a): hA1 64x6, arms hA1c, bodyD, bodyG, hg (hungergate=1), hgbodyD. Spot quota is 64 CPUs for the whole project: one VM at a time.

See [[inrun-cloud]], [[inrun-batches-day2]], [[teacher-lane]].

- **body-1022 result (64x6, 2026-10-22 ~15:30Z):** fixed gate (hg) 8,269 vs 1,700 home food (62/64), die-outs 16 vs 8. Gate gene alone from off (bodyG): climbs 0 -> 0.21 in 6 rounds, food 2,177 vs 1,651 n.s., die-outs 7 vs 8. Opening gut/crop/diet on top of the gate (hgbodyD vs hg): food 5,434 vs 8,342, lower in 45/64 (p=0.002), starved 66 vs 47; selection grows crop ~1.5x (score rewards something food home doesn't). Verdict: keep those genes closed for training. VM deleted. Thread wound down after owner's consolidation (15:02Z); switch and results handed to the Teacher lane.
- Digging (paired): fixed gate digs a third less (63 vs 93 cells, rooms 0.2 vs 0.6); body genes on top of the gate win some back (83 vs 65, rooms 0.4 vs 0.2) at a third of food. Owner: food is not the only goal, so this is a trade for the Teacher lane's score.
