---
name: brain-lane
description: Brain lane (arena selection training of the ant's general brain, dig step 5) CLOSED 2026-10-22 by owner choice; final cvs result, open items, where data lives
metadata:
  type: project
  modified: 2026-10-22T01:52:03.702Z
---

**Lane closed 2026-10-22 ~02:00Z** (owner chose "Wrap up the lane"; training moved to the teacher lane's in-colony selection, see [[teacher-lane]], [[inrun-selection]]). Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB9QX2Rp95c3vjhPzuDYRVRm. Final PR #783 (docs/data only) handed to [[merge-desk]]. No VMs running.

- **cvs result** (96 test worlds, full loads): copy 12.51/11.94 vs selection 10.66/10.44 (paired +1.85±0.50, +1.50±0.46). Seed 201 unequal CPU (150 vs 68 gens). Lead is mostly head start (copy v2 1.23 ahead of s171). Data: Reports/brain-training/stage2/dig/8/cvs/ and /mnt/project-files/brain-lane/.
- **Fetch erosion = shrinking trips** (trips end 20-27 cols out; s91 33+); loss only with food at 30 cols. Far-only share of s91 after cvs: 0.37-0.58. Any foraging score should test far food.
- **Open items** (hand-off last log entry): far-only fetch gate dist=30,30 calib=96 (design doc far-fetch-and-centroid-design-2026-10-21.md); energy-sense patch /mnt/project-files/brain-lane/energy-sense-real-tank.patch (owner approved, passed to teacher lane); parents'-average child deferred; dig-rung-design.md steps 10-11 should say chambers 7 tall; speed PRs #775/#772.

**Why:** a later thread asking about arena training starts here. **How to apply:** don't restart the arena ladder without the owner; route ant training to the teacher lane.
