---
name: crater-refusal-copy
description: Crater refusal (PP_RIMVETO) copied into hA1, 2026-10-22: candidate r4, why plain recipe failed, spoil refusal copy failed
metadata:
  type: project
  modified: 2026-10-22T14:43:26.863Z
---

Thread "Crater refusal lesson" (2026-10-22 13:30-14:45Z). Files: /mnt/project-files/crater-refusal/ (README.md table, genomes, patch vs compute 98ad9dc4, copier_r.py, scripts). Scratch branch claude/project-thread-ng2n8a (never for main).

- **Candidate r4** (genomes/r4.raw.txt): bed 32 seeds sel=off refusal off vs hA1: crater near nest 39 -> 23, food home 269 -> 333, died 4.3 -> 5.9 of 24, collapsed 4 vs 2. Handed to Teacher lane for 64x6 + common garden. Acting ceiling: crater 0, food 491.
- **Why the plain copier_c recipe did nothing (r1, best step 0):** hA1 already avoids refused cuts in 99.5% of labelled states, and mempath left a 0.055 KL floor on every bed group. Fix: `rim_min_mass: 0.02` (keep only states where the ref puts 2%+ on a refused option) + `mempath: false` + rim weight 20 (5 was too weak).
- **Spoil refusal copy (r3, PP_SPOILVETO=5 in the same refused set) FAILED:** ants stop letting go of spoil everywhere, crater 54, 8/32 collapsed. Acting both refusals is the best arm (crater 4, food 525, spoil inside 2), so it needs another copy method.
- `PP_RIMVETO_LABEL=1` = label only; positive control byte-identical events. Copier with 6 bed logs of ~85k labels each OOMs on 15 GB; 4 logs fit.

- **r5** (15:40Z, all three dig-lane teachers incl. PP_SPOILDROP=15 positive far-drop label as 16th field, 3 logs): new cuts 320->388 (29/32), died 4.6, collapsed 1, food/crater ~same, BUT re-cuts 860->2,055 and far drops 937->2,097 (32/32): a field treadmill; spoil inside rose 149->267 (spoil refusal did not copy). Both r4 and r5 with Teacher lane for 64x6. Thread closed (owner consolidating); follow-up belongs to Teacher lane / Dig and nest scoring.

**Why:** next thread should not redo r1/r3. **How to apply:** see [[teacher-lane]], [[dig-loss]], [[foraging-loop-lane]].
