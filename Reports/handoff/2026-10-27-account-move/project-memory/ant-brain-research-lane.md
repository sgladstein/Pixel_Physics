---
name: ant-brain-research-lane
description: Research lane for the ant brain (2026-10-21): its reports (PR 755 merged, PR 768 on in-run selection), how it messages other lanes, what was sent
metadata:
  type: project
  modified: 2026-10-21T19:52:46.194Z
---

Started 2026-10-21 at Scotttt's request. Literature research only: no trainer, engine or VM work. Q1 noisy fitness, Q2 a ladder that doesn't forget, Q3 teaching plus selection, Q4 colony tasks, Q5 real ant sensing, then a field map with 21 ranked tests.

- **Reports:** `Reports/brain-training/research/`. Q1-Q5 and the field map are on `main` (**PR #755, merged 2026-10-21 13:57Z**). Copies are in `/mnt/project-files/brain-research/`.
- **In-run selection report** (`in-run-selection-method-2026-10-21.md`): **PR #768** on branch `claude/project-thread-hhopdh`, restarted from main after #755. Handed to the merge desk at 19:50Z. Findings:
  - Food delivery can't rise. There is about one delivery per ant per 30k tenure, and z-ranking (edge / own-credit SE) marks big, rare credits down.
  - Proposed arms, which await Scotttt's ruling: breed on the shrunken mean; pay food half as it moves, half on arrival. The fixed-brood arm was dropped.
  - Teacher lane measured (19:50Z): the food price is near its cap (mean 2.83-2.91 of 3), because pile bites count as eaten. The harness mutates with prune 0 and grow 1, and the body bills each live link. The report was corrected at head 50edff5a.
  - Other points: the turnover rule is rtNEAT's law of eligibility; calibrate the z<-3 cull on clones; grow=1 with no wire cost bloats the brain.
- **Scotttt's emphasis (2026-10-21):** research the modelling and training approach first. Keep ant biology light, because the repo already has a lot of it (teacher notes `Reports/brain-training/teacher-training/notes/`).
- **Messaging lanes that are project threads:** use `message_thread`.
  - Teacher lane thread: cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFFqqpwTYL4i4DtFihYcesb.
  - Merge desk thread: cmsg_01Mei3dwkcdzPxa5Jmuqi2bBBjZgzija4YCh6ciFtqnPbN.
  - Brain lane thread: cmsg_01Mei3dwkcdzPxa5Jmuqi2bB9QX2Rp95c3vjhPzuDYRVRm.
  - For outside sessions, use `create_trigger(persistent_session_id=...)` then `fire_trigger`, with no `connectors` field.
- **Sent so far, all confirmed delivered:**
  - Q3a-e to the teacher lane: the walk-home lesson spreads through routing via slot 1252.
  - Q1 and Q2 to the brain lane: prune only small links (`noprune=@file`).
  - Q4's C7-C10 to both lanes.
  - Q5 to both lanes: #751 touch bit, ablate DEPTH, the home vector is too exact, a height sense needs a reach of at least 17.
  - The in-run selection findings to the teacher thread (19:50Z).
- **docscheck:** run it with `PYTHONWARNINGS=ignore::DeprecationWarning` (a Python 3.13 warning in deadendindex.py).
- **Scale:** 1 BL = 2 cells (`Chain(2)`).

**Why:** later brain-training threads should know what was already sent and ruled on, so they don't repeat it.
**How to apply:** check the lanes' results on these tests before proposing them again. See [[soil-physics-and-roof-sense]] and [[brain-lane]].
