---
name: hungry-lane
description: "Go out when hungry" teacher lesson thread (2026-10-21): what kills in-run colonies, the ceiling result, where the scratch code is
metadata:
  type: project
  modified: 2026-10-21T21:19:27.419Z
---

Thread "Go out when hungry lesson" (split from [[teacher-lane]] / [[inrun-selection]] by owner, 2026-10-21). Teacher lane keeps scoring/selection; this thread builds the lesson and delivers founder genomes (.raw.txt) to /mnt/project-files/teacher-lane/hungry-lesson/, then messages the Teacher lane thread (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFFqqpwTYL4i4DtFihYcesb). Teacher lane offers judge.sh in /mnt/project-files/teacher-lane/inrun/scratch-patches/latest/.

**Die-off cause (traced 2026-10-21):** copy v2's job head ignores energy (walk ~60% / feed ~37% at every energy). In the last 4k frames before a die-off 94% of hungry decisions are ants holding a soil pellet at the bottom of the nest chamber; jaws full, so no PickUp; an up step is offered nearly always, Drop almost never. Feed is billed like a cut, so constant feeding drains tanks in lockstep.

**Ceiling (acting scratch teacher `PP_HUNGRY=lo:hi PP_HUNGRY_AWAY=1`, radius `PP_HUNGRY_R`=40):** below 2 grants, empty crop: drop pellet off-hole if offered else carry up/away; climb out; step away from home to 40 cols; force PickUp. Clone colonies survivors 18/19/22/23 of 24 (v2 ~1). 3 grants worse (founders start at 3 and run out before heaps fill); seeking teacher-visible food worse (chased buried food, ant towers); no radius ran founders to the walls.

**Arena energy trap:** training arenas read energy against their own budget (fetch/trail exactly 1 grant, soil 1 -> 0.25), so the lesson fires only in [1.05, 2) grants. Real fix (owner/Brain lane call): arenas show real grants.

**Founders delivered** (hungry-lesson/README.md has the table): h1 (881 links, clone survivors 8.8/24 vs v2 1.5, dig -0.22, fetch ok) and h6 (1,135 links, 10.6/24, dig -1.11 = behaviour leak, NOT link bill; fetch ok). L1-priced h8 lost the lesson. Copier needs ~9 GB: one run at a time on this 15 GB box (a second one dies silently).

**Scratch:** worktree /home/user/tp branch scratch/hungry; patch + copier_h.py + thin.py + configs saved in /mnt/project-files/teacher-lane/hungry-lesson/scratch/.

**Routing (owner, 2026-10-22):** send this lane's results and requests to the Teacher lane thread, not to the Brain lane or the owner; the Teacher lane does the main work now. Owner approved making the arenas show real energy (2026-10-22); the request sits with the Teacher lane. Copy 9 (plain "below 2 tanks" on honest readings) failed: clone survivors 3.4/24, lesson overlap 0.24->0.30; not delivered. **Teacher lane's in-colony batch (64 seeds x 6 rounds) ranks h1 above h6** (lost 20 vs 28 colonies, food 6,906 vs 5,291/seed): aim any further copy at in-colony survival, not clone survival; Teacher lane runs a 64-seed cloud batch in ~20 min. Energy PR (Teacher lane chose option A, 2026-10-22): Brain lane patch `CreatureDef::sensed_tank`; fed arenas (1e9 J) now feel ~1.0, so the forager preset's egg bar fires there and the fetch test's forager has it off (`mind::below_the_egg_bar`). Worktree /home/user/pr-energy, branch claude/project-thread-o140z6.

**In the repo (2026-10-22):** energy fix merged (PR #781); report Reports/brain-training/teacher-training/hungry-lesson-2026-10-22.md + measurements/hungry-lesson/ (genomes, copier, configs, whole scratch tree as patch from b0dc264a) in PR #826, with the merge desk.
