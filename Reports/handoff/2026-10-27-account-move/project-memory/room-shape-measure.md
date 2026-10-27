---
name: room-shape-measure
description: roomshape.py (2026-10-22) graded "on the way to multiple rooms" score from a nestmap, for Teacher lane lexicase
metadata:
  type: project
  modified: 2026-10-22T19:04:18.016Z
---

Dig-scoring thread built /mnt/project-files/teacher-lane/nestmaps-1022/roomshape.py + ROOMSHAPE.md (owner 18:25Z: get multiple rooms, credit first steps).
- Rooms = 4-connected pieces of the 5x5 morphological opening; main room = biggest; extra rooms min(cells,50)/2; tunnels = reach-2 cap 15, best 4. One room of any size = 0.
- Crater (sky-open, joined to door, below datum) MUST be in the space and count as body: leaving it out made deep slot pits' overhangs read as rooms (c39 76); rooms mostly sky-open are dropped (c61 bowl 73).
- Old ant common garden 12 worlds (old-ant-cg/): median 20 vs selected medians 3-6; old ant not above every colony (worst world 8).
- Soft spot: a 4-tall neck splits a chamber into 2 rooms.
See [[dig-loss]], [[teacher-lane]], [[nest-score]].
- 19:50Z per-ant w_shape (pay a cut its marginal roomshape gain, cap 4; patch nest-score/digloss/wshape-vs-529ff42a.patch, commit 47f509e2 in /home/user/colsel branch shapecredit): small test hA1 8 seeds x 3 rounds w=3 -> no selection response (paid cuts flat ~25-30), r2 roomshape 1 vs 8, no rooms either arm. Local 5x5 "narrow cut" rule useless (rooms start as tunnels). Only worth trying inside colsel's +shape arm.
