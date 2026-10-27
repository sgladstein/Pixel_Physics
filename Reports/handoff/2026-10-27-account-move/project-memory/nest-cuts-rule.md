---
name: nest-cuts-rule
description: How digging is credited (owner-agreed 2026-10-22 17:10Z): ants paid at cut time; colonies judged on cuts still nest at round end (nestcount.py)
metadata:
  type: project
---

- **Colony level (colony selection, judge):** scratch/inrun/nestcount.py reads the end map (nestmap.txt) and counts cuts whose cell is still nest at the end (open, joined to the door, below the datum, roofed or in the shaft; crater and refilled cells count for nothing), plus nest size, crater, spoil left within 20 cols, spoil let go below the datum. colsel.py DIG=nest (default, commit 0d98f795): gate = kept cuts made in the second half of the round >= 5; lexicase cases = home8, -starved, kept late cuts, nest size. DIG=cuts = old raw-cuts rule.
- **Ant level (in-run pay):** unchanged, the ant is paid when it cuts, never clawed back. Owner agreed (👍): a refill is usually not the cutter's fault, and parents are picked mid-round, so end-of-round credit is too late. Blame for refills goes to the spoil dropper (rim=15). nest=zone per-ant pay exists but was not adopted (nestzone-1022: food -1/3 to -1/2, crater unchanged).
- World 3099, 200k: old ant nest 278, kept cuts 232 (48 late), spoil 0; the replay's "best" selected colony hA1veto_3: nest 19, kept cuts 7, crater 127. Its cuts were crater.

**Why:** the owner asked twice how nest-expanding cuts are counted; don't re-litigate. **How to apply:** judge digging with nestcount, not raw cuts near the door. See [[teacher-plan-1022]], [[nest-score]], [[dig-loss]].

Batch results: [[colsel-batches-1022]].
- **cut_join=N (743157ab, owner 21:26Z: swimming under the nest must not count as nest digging or be selected for):** per-ant pay only for cuts touching open space joined to the door (flood every N frames through non-ground below the datum). Off by default; old ant keeps 200/232 cuts paid; selected lose surface-row rim cuts and swims. To test in colsel9 (hcs with/without). Picture /mnt/project-files/teacher-lane/cuts-paid-vs-not.png.
- Compute branch merged main via harness lane's claude/project-thread-39xi6a (PR #833) at 0bbab07f; still push to claude/project-thread-8y5ak0.
