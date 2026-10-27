---
name: colony-war-lane
description: Colony-vs-colony combat thread (2026-10-24/25): rivalbed two-colony bed, tournaments, findings, PR #886, traps for any two-colony harness
metadata:
  type: project
  modified: 2026-10-25T04:33:36.128Z
---

Thread "Explore colony vs colony combat" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBUup3BEXYmC7DALPtj55Y57), asked by Scotttt 2026-10-24 22:28Z. Branch claude/project-thread-ullbc9, PR #886 merged 2026-10-25 as 778de14. Report: Reports/colony-vs-colony-2026-10-24.md. Data: /mnt/project-files/colony-war/.

- Tools: `examples/rivalbed.rs` (two mind colonies on colony_bed, pool_a/pool_b, bite=, solo=, food=split, mutate=, save=, kills.csv, nest_deep/crater). `scratch/inrun/rivaltour.py` (tournament, score=). `scratch/inrun/rivaljudge.py` (head to head, score=).
- std_pool holds the bite ON (inrun enter_bite). With both able to bite: cold peace (~17 kills/100k frames). One-sided: the victim dies out 10/24, the biter gains nothing. Ant-on-ant bites kill in one bite, so a lone raider in a nest kills 20-60. PP_MIND_BITE_SCALE (scratch, off) makes the bite graded.
- Tournaments scored against the rival breed fighters (24/24 vs std) and do NOT settle into peace. But food home collapses (~4,500 -> ~500) whatever the score, and also in the no-bite control. Causes: a relative share can't hold a habit both colonies lose, and inside a match the layers and survivors are just the fed ants.
- Proposal sent to [[teacher-lane]]: a paired-colony colsel case. Score the contest against the rival; score food and nest against the founder; keep births going to carriers; use the graded bite.

**Traps (Why: each cost a batch):** the brood lay_at is per species, so with two colonies the bigger one stops its neighbour laying. Use World::colony_lay_scale. Never `pgrep -f`/`pkill -f` (it killed my own shell). The deadendindex regen needs `git fetch --unshallow`.
**How to apply:** any second-colony harness or combat training goes through [[teacher-lane]]. See [[predator-lane]].
