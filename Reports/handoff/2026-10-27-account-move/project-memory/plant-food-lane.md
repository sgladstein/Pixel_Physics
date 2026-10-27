---
name: plant-food-lane
description: "Ants feeding on plants" lane (2026-10-23/24): planted bed, grazing, weaning bed runs plantwean1-5, where it stands
metadata:
  type: project
  modified: 2026-10-24T15:14:57.363Z
---

Thread "Ants feeding on plants" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBHNgTg91PZKDrDAAo8cErcs), child of the Teacher lane. Branch `claude/project-thread-o2tizu` (recreated from main per PR). **Full record: Reports/brain-training/plant-food/plant-food-weaning-2026-10-24.md (PR #863)**; working files /mnt/project-files/plant-food/.

- Landed: #836 (planted bed `plants=MIX[:SPACING[:BAND]] grow= rain=`, `weed=B`, `fjoule=1`, `cap_to=`, `%` plant cells in nestmap, PP_MIND_GRAZE), #855 (colsel `pilegate=extinct:LOW:HIGH`, jobs/plantwean5.txt), #863 (report + data + notes + 3 dead-ends; MERGED 06406cf 2026-10-24 16:07Z). Thread resolved after landing; plantwean5 handed to the Teacher lane.
- KEY FINDING: the mind's Pick up took loose food only, so trained ants could not eat living plants or seeds. PP_MIND_GRAZE fixed it; default-on since 2026-10-24 (owner "Yes to grazing").
- 2026-10-23 17:11Z OWNER CHOSE (card): **Weaning bed** = garden + pile shrinking round by round, pay by energy, seedlings pulled near the door. Weaning must be ACROSS rounds (colsel pilecap), not within one (cap_to: colonies die when the pile is gone).
- weed=30 is needed: roots under the door stop digging (jaws can't cut plant tissue).
- Planted bed got harsher after #841/#842 (cap 30 alive 18 -> 2, seeds 1-4); re-baseline beds after plant changes.
- plantwean3 (hcs, no breeding, pilecap=45:5:10:10): CG r31 at cap 10 alive 21/24 vs founder 0, nest 184.5 vs 33 (53/64 pairs better). Cap 5: median 4; cap 0: all die. Ants eat plants but carry ~none home: next lever = plant loads home.
- plantwean4 (std_pool + breeding, pilecap=45:5:10:5): bounced 25<->20, never weaned; worse than untrained std_pool at cap 20 (alive 71.5 vs 80.5, nest 116 vs 310). std_pool cliff is between cap 20 and 10.
- plantwean5 (std_pool, pilecap=30:5:10:5 pilegate=extinct:10:25) queued by the Teacher lane after farfood2 (~15:00Z 2026-10-24); the Teacher judges it against std_pool in the same worlds.
- DEFAULT-ON (owner rule 2026-10-24 17:35Z): with plants=, weed=30 and fjoule=1 now default (=0 opts out); colsel pilecap= on a breeding bed defaults to pilegate=extinct:10:25 (pilegate=off or brood=off for the starved rule). plants= and cap_to stay off (bed choice 2.3x cost; dead end). PR #868 MERGED 72889eb 2026-10-24 19:03Z.
- TREES (owner asked 2026-10-25): trained mind has NO trunk crossing (Walk excludes woody landings); old ant crosses. examples/treewalk (PR #877): trained stuck beside wood 5-8% of ant-time vs old 0.3-2.3%, but reaches 437-500 of 512 columns: slowed, not trapped. Owner said "Yes. Do it" (02:44Z): PR #883 MERGED 84988ab gives minds the trunk crossing, PIXEL_PHYSICS_MIND_CROSS default on; stuck 4-7% -> 0%, labsurvive alive 33-66 -> 77-214; pile bed byte-identical, lab numbers shift (Teacher told). Pictures /mnt/project-files/plant-food/trees/.
- Judging rules for any colsel run: [[colsel-judging]].

**Why:** a later session needs the switch names, what has been tried and the open blocker. **How to apply:** start from the report; see [[teacher-lane]].
