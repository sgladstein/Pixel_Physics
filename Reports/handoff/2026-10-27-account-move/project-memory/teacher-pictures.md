---
name: teacher-pictures
description: Owner wants images or colony replays with every Teacher-lane result (2026-10-22 20:06Z); the one-command tools that make them
metadata:
  type: feedback
---

Owner (2026-10-22 20:06Z): "I need images or colony replays from these." Numbers alone are not enough; every batch result goes out with a picture.

Tools (copies in /mnt/project-files/teacher-lane/):
- `replay.sh POOLDIR SEED NAME TITLE` (scratchpad): films a colony's final/ pool (or `old`) in a common-garden world with film=50, makes NAME.mp4 + NAME-data.json + last.png and prints food/starved/nest/crater/spoil/rooms. ~3 min. Then add a RUNS entry to scratchpad film/page/index.html and republish the replay artifact https://claude.ai/artifact/HFEskdEuRrzvSsSEYU7LeK with `files`.
- `nestsheet.py OUT.png "Label|sub=GLOB" ...`: contact sheet of end maps (nestmap.txt), 8 colonies per row at nest-size quantiles, with nest/crater/spoil/rooms captions and a legend. Selection-round maps (c*/rN) exist for every colony; garden run dirs are deleted.
- Each film run makes ~450 MB of frames: replay.sh deletes them. The box filled up once (2026-10-22 20:10Z).

**Why:** the owner judges by eye; the 32-round sheet showed every selected arm opens the top layer into a sky-open bowl with spoil towers, which the counts hid. **How to apply:** attach a sheet or replay link to each result message. See [[nest-cuts-rule]].
