---
name: replays-and-far-food
description: Owner 2026-10-25 19:09Z: make colony replays after most runs; default test food must be far from the nest and moving (dist=25 garden too close)
metadata:
  type: feedback
---

Owner (Scotttt, 2026-10-25 19:09Z, Teacher lane): "please generate colony replays after most runs. Also I think our default food location should be farther from the nest (ideally still always moving), but it is way too close in the rooms colony replay."

**Why:** the owner judges by watching colonies; colsel's common garden ran food at dist=25, which hid how trained colonies fail when food is far.
**How to apply:**
- After each batch, replay the best colony (and the founder or hand-built ant on the same seed) with inrun `film=50 film_hw=128`, encode with ffmpeg (glob f*.png, scale x3 neighbor), publish on an Artifact page with counts (alive, starved, nest/open, crater, store). Rebuild a colony's carry from cN/rR/final/index.csv (colsel.carried); the garden seed is c*1000+99; the run reproduces exactly.
- New default test food: inrun `patches=20000:40:120` (one heap, random side, 40-120 cols, jumps every 20k frames; seeded, so arms pair). Putting `patches=` in a colsel job line moves both training and garden (CGC inherits EXTRA); drop heaps=/dist=.
- First look, world 21099: hand-built ant 80 alive, 11 starved, 1,139 open; nest2 c21 48 alive, 300 starved, 210 open, crater 180; std c10 23 alive, 217 starved, store 307. Page https://claude.ai/artifact/HcMfwxsyTjY43HPmbNkKbe
See [[nest-priorities]], [[teacher-pictures]].
