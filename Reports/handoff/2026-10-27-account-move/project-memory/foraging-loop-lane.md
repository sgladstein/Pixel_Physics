---
name: foraging-loop-lane
description: "Foraging food loop" thread (2026-10-22): old ant vs new ant, delivery-count trap, the jam that kills copies, shadow teacher, pack move PR #794; copies s4/d1 failed; hA1 (hungry-only DAgger copy) cuts collapse 26->2 of 32
metadata:
  type: project
  modified: 2026-10-22T13:13:22.029Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB5tpJHv4VtX1vkEVLss9JNC, partner to [[teacher-lane]]. Owner: "the old ant is the eventual bar". **Owner rule 2026-10-22 05:34Z: the Teacher lane owns the one ant; this lane delivers lessons/teachers for its founder, short test copies only.** Founder since 05:40Z: c1 (this lane's first copy; Teacher lane 64x6: home food 2,319 vs h1 1,431/colony-round, extinct 9 vs 17).

Report with every table: /mnt/project-files/foraging-loop/report-2026-10-22.md; repo copy with founders, patches, scripts in Reports/brain-training/foraging-loop/ (PR #825, docs only, merge desk). Patch (all env-gated): shadow-rim-vs-9d19121c.patch vs compute branch 9d19121c, worktree /home/user/cloud.

**Delivery-count trap:** the bed books any trip-load put-down beside a dug cell as "delivered" (NestHome::Dug). Count |x-128|<=8 only. 8 seeds x 24 ants, 200k, sel=off: old ant 5,865 home / 0 died; h1 503 / 98; c1 2,469 / 96. `forage_anchor` drifts with every nest contact: never use it as home.

**Old ant's economy:** never chooses Eat; digests ~70% of bites from its carried crop; every old ant ends 25+ tanks.

**What kills copies (traced c1 seed 3):** the colony jams in its pit holding pellets; Drop offered 6%, PickUp 0%; Feed passes banks flat, so all die within ~100 frames beside a stocked store. Deaths come in bursts of 15-20, so 8-seed death sums are ±30 noise.

**Failed (do not retry without a new input):** hand-rule teacher patches HOME_R/ANYDROP/WAY/STORE/NOEAT (90-131 died); copies s4 (old-ant shadow + rim refusal, 133 died) and d1 (DAgger of px2.5 on c1 states, 130 died); body levers PP_NEEDDROP, PP_NOADULTFEED.

**Tools:** PP_SHADOW=1 old ant as teacher; copier_c.py (hold_logs); runl.sh, hs32.py, dlogA.sh in project files. Copier needs ~11 GB: never compile alongside it (s2 died that way).

**Pack move:** PR #794 (owner approved), with the Merge desk.

**32-seed truth (2026-10-22 ~10:30Z; 8 seeds is noise):** every carry-home copy raises collapse (c1 26/32 vs h1 14/32); a no-label control copy collapses like h1, so the carry lesson itself (food leaves bodies, store unreachable) is the cause. **hA1 works:** DAgger copy of the hungry-only teacher (PP_CARRY_HUNGRYONLY + HUNGRY 2.5 + ANYCUT, labels on c1's own states, dlogA.sh, hA1.json) into c1: home 269/colony, died 4.3/24, collapsed 2/32 (c1 253, 14.6, 26). Teacher lane 64x6 (11:14Z): **hA1 is the founder** (pack on). Extinct 8 vs 17. Use hs32.py, 32 seeds, for any arm call.

**State 2026-10-22 13:15Z:** under selection, hA1 lines bring home more food than the old ant (common garden ~3,900 vs 729, see [[inrun-batches-day2]]). Remaining gaps are starving (~5/24 vs 0) and digging fading. The Teacher lane owns both, so this lane has nothing open. PP_NEEDDROP tested on hA1 (13:45Z, 32 seeds): no effect (4.0 vs 4.3 died), since hA1 almost never holds a pellet while hungry; dropped. Starvation trace moved to thread "Why selected ants starve".

**Why / how to apply:** start from this before any new foraging lesson. Don't build a separate carry lesson: the Teacher lane's appetite gate already covers food.
