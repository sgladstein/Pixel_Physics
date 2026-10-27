---
name: parallel-merge-lane
description: "Parallel training with weight merging" thread (2026-10-23): merging only works for lessons on disjoint weights; dig/forage islands merged fail; PR #840
metadata:
  type: project
  modified: 2026-10-23T23:18:07.410Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBF1LkapLdSZr3ThCiY9f3KG, asked by Scotttt 2026-10-23 22:32Z. Report Reports/brain-training/research/parallel-training-and-merging-2026-10-23.md, PR #840 MERGED 2026-10-23 23:44Z (head 540213f5). Data/scripts/genomes: /mnt/project-files/parallel-merge/.

- Slot-union merge of parallel lessons works: hA1+r4+hcs (disjoint slots) food 431 vs hcs 309 (22/24), starved same. Averaging halves each lesson (379).
- Selection lines merge to a compromise: colsel20 c3 digger x c27 forager, mix/crossover/average all between parents (average slightly above the line).
- Dig-only island dies (6/8 extinct, no extra digging); forage-only 4/8; task arithmetic of islands collapsed 12/12; average starved 8.6 vs 1.8.
- Recommendation: no dig/forage lanes merged later; full-job islands swapping whole ants (maybe on GitHub Actions); lessons train own weights (only_allow) + merge by slot union + a few joint rounds.

**Why:** later threads should not rerun this. **How to apply:** see [[teacher-lane]], [[far-home-sense-lane]], [[colony-training-review-lane]].
- 23:25Z owner: dig/forage was only the brief's example; wanted a general framework. Living guide Reports/brain-training/parallel-training-guide.md (PR #840, edit in place): methods A full-job islands swap ants, B world-variant islands, C lessons on own weights + slot union, D specialist weight merge (never), E co-evolving opponents, F weaned body scaffold (Teacher lane, fixed crater); 6 questions; per-task method (dig A+F, forage A+C, brood A+brood=lay, plants C then B, predators C then B/E, battles E).
