---
name: trail-teacher
description: Trail-following teacher session on hA1 (maze thread, 2026-10-22 evening): rule 1 (climb B) bad, rule 2 (stay on trail, outward, damped laying) works acting but two DAgger copies failed
metadata:
  type: project
---

Agreed with [[teacher-lane]] after the owner asked (15:33Z) whether to fix trail following by targeted training or a teacher session. Maze training rejected (its slope teaches climb-the-gradient). Table and files: /mnt/project-files/maze/colony/trail-teacher-2026-10-22.md (+ patch vs compute ee38ae5b, copier_t.py with target_only).

32 seeds, sel-off, pack on, vs hA1-ladenB (B laid only while carrying):
- hA1-noB: home 374 vs 271 (28/4). Teacher lane is judging it at 64 seeds as founder candidate.
- Rule 1 (empty, off nest, step up B) acting: home 148 (4/28); copy tA1: 19/24 died. No-lesson control copy fine, so the pipeline is OK. Cause: climbers pile on a runaway B peak 10-20 cols out, short of food.
- Rule 2 (B emit -8*B-here; empty ant stays on trail, steps outward) acting: home 391, field up 30/2 vs noB, fewer deaths, far roamers 40%->21%.
- Rule 2 copies oA1/oA2 (anchor weight 3, 12): worse than baseline, fit 55-64%, dithering. Two failures the same way: stop copying into today's brain. Likely missing a far-from-home direction sense (f::HOME = cos*squash(dist/32)), same as the Teacher lane's come-home copies.

**Why:** don't rerun these arms or retry climb-B. **How to apply:** rule 2 is ready as a second lesson if a sharper home sense lands; check rule acting before any copy.
