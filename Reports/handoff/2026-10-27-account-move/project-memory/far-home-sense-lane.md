---
name: far-home-sense-lane
description: Far-from-home sense thread (2026-10-22 evening): PP_HOMEVEC made come-home copyable (hc1, hcs founder candidate); trail rule 2 still does not copy
metadata:
  type: project
  modified: 2026-10-22T21:14:39.442Z
---

Thread "Far-from-home sense" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBAGWiYCU5kfaYBKZMeh4ksH), asked for by the Teacher lane after the owner offered spare capacity (19:57Z). Report: Reports/brain-training/teacher-training/far-home-sense-2026-10-22.md, PR #832 (reports only), handed to the Merge desk. Files also in /mnt/project-files/far-home-sense/.

- **Switch `PP_HOMEVEC=R:W`** (used 40:30): far = clamp((d-R)/W,0,1) on ctx slot 9, and cos-to-home*far on option feature 40 (f::LAST_FOOD). Exactly 0 within R. On the compute branch since 75593154 (Teacher lane merged commit f45d75ce, which also ports PP_CARRY/PP_HOMEFAR and PP_TRAIL_*). Genomes need slot 9 / f40 links zeroed first (zero.py).
- **Recipe that worked:** train ONLY the new sense's weights (copier `only_allow`), so nothing near home can change. That is what fixed the hm1-hm3 leak.
- 32 seeds, sel off, starved per 24 (past 60 cols): hA1 4.31 (2.69); teacher driving 2.28 (0.75); hc1 copy 2.50 (0.75), food and digging unchanged; **hcs** = hc1 + hA1-sup emit edit 1.22 (0.44), food 305. hcs handed to the Teacher lane as founder candidate (needs PP_HOMEVEC=40:30).
- Trail rule 2: acting on hcs 0.69 starved and food 383 (best arm), but the copy tb fails (food 309 -> 218). PP_HOMEDIR (cos at every distance) fit nothing and costs a little. The blocker is "on the trail" (relative across options); next sense to try: per-option "B there / best B in reach", trained alone.

**Why:** later lanes shouldn't redo this. **How to apply:** for any lesson that leaks, build the input the rule reads and train only its weights. See [[starve-trace-lane]], [[trail-teacher]], [[teacher-inbox-1022]].
