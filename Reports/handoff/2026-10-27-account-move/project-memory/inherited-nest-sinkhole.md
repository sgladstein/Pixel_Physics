---
name: inherited-nest-sinkhole
description: 2026-10-23: why inherit_nest=1 rounds lost a soil band (unlined carve into loose soil), the fix (PR #834), and the open door-join issue
metadata:
  type: project
  modified: 2026-10-23T16:56:19.588Z
---

- **Cause (found 2026-10-23 17:30Z):** inrun.rs read_nestmap carved kept nest cells EMPTY into the fresh world's loose soil (a powder; only packedsoil spans a hole). Holes refill from above within 10 frames and the void surfaces as the "scraped band". Copying the old map's lining can't help: the crater rule keeps only the roofed edges of crater-topped rooms, so their inner wall is fresh loose soil.
- **Fix:** commit fa8fcc8f on claude/project-thread-lvrh2k (on top of compute branch 89453ec9), PR #834 (base = compute branch). It lines every kept cell via the new World::line_dug_cell (pack_neighbours) and carves kept '?' cells (ants/food that used to be left as soil plugs). INHERIT_LINE=0 / INHERIT_PLUG=1 give the old carve. Over 192 nestmaps-1022 maps at 3000 frames: refilled median 74 -> 0; top-6-row band 22->66 today vs 22->22 fixed. Files: /mnt/project-files/inherited-nest/.
- **Door join (done 17:45Z):** most inherited rooms arrived sealed (median 115 cells carved, 30 joined: the link ran through the uninherited crater). The Teacher lane chose to keep link cells: inherit_link=1 (1cbfbba5, off by default, in PR #834) keeps the cheapest 2-wide lined run of old open cells to the door, never using the surface row, top-6 rows cost 4. Joined/carved went 0.31 -> 1.00, 166/192 maps fully joined, top-6 band flat. PR #834 MERGED to main as c08e7f1 (2026-10-23 17:57Z).
- **Min pocket (18:20Z, b9497657):** linking every pocket drew made-up tunnel trees on crater-bowl maps (c20, owner asked "is this ant dug?"). Now inherit_link=MIN (USE 20) links pockets of 20+ cells and drops smaller ones. At MIN 1, 49 of 192 maps had links bigger than the pockets they joined; at 20, none. 86% of pocket cells are kept, and 191/192 maps are fully joined. At 50, real rooms start to be dropped.
- Side finding: the keep flood and nestcount.py seed from every open door-column cell, so a buried pocket crossing the door column counts as joined. Not changed.

**Why:** the Teacher lane trained without inheritance because of this bug. **How to apply:** inherited batches run before fa8fcc8f started every round in a sinkhole. See [[colsel-batches-1023b]].
