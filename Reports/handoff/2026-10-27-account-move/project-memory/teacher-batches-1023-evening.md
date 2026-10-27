---
name: teacher-batches-1023-evening
description: Teacher lane batches of 2026-10-23 evening: brood1 (breeding stable), plantwean2, colsel23 inheritance retry
metadata:
  type: project
---

- **brood1 result (21:11Z, brood=lay lay_cap=48:8, wean=4:20, from hcs):** stable: 70-90 ants, 61-64/64 colonies alive, starved ~2/round by r15. Crutch NOT needed (ctl better on crater 4 vs 18-300). r31 nest_food 260 ctl / 320 crutch vs 514 no-breeding; gut 28-39 vs 107 grants. Nests still food-packed; shape 0 after wean. nests-brood1.png. plantwean2 started 21:14Z, colsel23 queued (bc0yx8xmv).

- Lone predator thread (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBPYNpvazZ94GT2uVUdjhkJz, PR #844 spider + MIND_ATTACK): asked it to build co-evolution harness on inrun/colsel with my founder as prey: spiders=POOL:N, Attack+alarm for mind ants, +guard colony case; fixed pool first.

- **Owner 00:47Z 2026-10-24: breeding ON by default.** PR #845: brood=lay lay_cap=48:8 inrun defaults (brood=off / lay_cap=off opt out). Future jobs: add brood=off for old-bed controls; use kpm=0.013 kevents=4 with breeding (brood1's load).

- Plant bed after #841/#842: cap 30 crashes 3/4 seeds; rerun plantwean as plantwean3.txt on compute branch with pilecap=45:5:10:10 (+brood=off if #845 landed). bp1 (running) uses cap 30: may pin.

- **Resume (02:15Z 2026-10-24, PR #848 merged to main a373035):** colsel.py writes state.json per round and resumes; inrun_cloud.py uploads partial/<job>.tgz every 2 min and `start ... --resume OLDNAME` restores. After a preemption relaunch with --resume runvm.sh now relaunches with --resume (copy it per batch: bash reads scripts while running, never edit a running one). broodplant1c-1024 from compute branch 02:14Z (runvm-bp1.sh, log runvm-bp1.log), ETA ~04:30Z.

- pw3mini (plant lane local, 02:37Z): cap 45 held then fell to 20 by r7; CG alive 20 vs 4.5 founder. plantwean3-1024 queued after broodplant1 (chain-pw3.sh detached, log runvm-pw3.log) at compute 3cd17cde; copy results to /mnt/project-files/plant-food/plantwean3/.

**Why:** brood=lay is the likely new standard bed. **How to apply:** next batches stack on brood=lay lay_cap=48:8, no crutch. See [[teacher-children-1023]], [[colsel-batches-1023c]].
- 21:26Z: plantwean2 build failed (plant branch deleted after #836 merged); requeued as plantwean2b-1023 --ref main after colsel23 (started 21:25Z, compute 49a1109c+ecd0de23). Chain task: csvm23.sh then startP2.sh; after plantwean2b starts, run csvmP2.sh. Copy results to /mnt/project-files/plant-food/plantwean2/.
- **colsel23 (22:53Z):** inheritance (lined carve, link=20) works mechanically; CG r47 nest inh 102 / ctl 208 (20/38 paired), shape 4.5 vs 0, crater 10 vs 4; moving inh nest 43, crater 35. Carried nests = wide shallow food-packed layer under the surface (nests-colsel23.png). ctl reproduces colsel21 exactly (main merge changed nothing). plantwean2b started 22:55Z (waiter bkrsiitrf). Asked brood lane: PR brood to main + broodplant1.txt (bp1 brood+plants, bp1pile ctl); launch ~00:30Z.
- broodplant1 queued (startBP.sh after plantwean2/DONE; then run csvmBP.sh) from claude/project-thread-6einf2 4da1a7f2 (PR #839 brood to main). Copy results to /mnt/project-files/brood/broodplant1/.
- PR #841 (roots_required ON by default, 23:45Z): plant beds and baselines move after it lands; PIXEL_PHYSICS_ROOTS_REQUIRED=0 for paired control. Rerun a control when merging main after it.
- 00:20Z: plantwean2b preempted ~23:28Z (old waiter spun). New detached runner scratchpad/runvm.sh NAME JOBS REF NJOBS DEST (relaunches on preemption, HOURS env; nohup setsid, log runvm-*.log). broodplant1-1024 launched first (HOURS=5); plantwean2 to rerun after.
- 00:25Z: brood branch deleted after #839 merged (broodplant1.txt on main); broodplant1b-1024 launched from main (now with roots_required #841 + plant old age #842 on by default: plant baselines moved). Always launch from main once a lane's PR merges.
