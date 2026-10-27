---
name: teacher-children-1023
description: Teacher lane's child lanes of 2026-10-23 afternoon (soil loss, onto main, brood, plant food): what each delivered and pending hand-offs
metadata:
  type: project
---

- **16:38Z new child lanes (owner OK):** Inherited nest soil loss cmsg_01Mei3dwkcdzPxa5Jmuqi2bBPnRuXpgYVfQJu67Z4iVPav; Teacher work onto main cmsg_01Mei3dwkcdzPxa5Jmuqi2bBSNK1UsH7nK55dfNqUZHk6F (snapshot 89453ec9); Ants feeding on plants cmsg_01Mei3dwkcdzPxa5Jmuqi2bBHNgTg91PZKDrDAAo8cErcs. All briefed. Tools copied to /mnt/project-files/teacher-lane/tools/.
- **Inherit fix (soil-loss lane, PR #834 on 89453ec9, held by merge desk until compute branch PR to main):** carve went into LOOSE soil, refilled in 10 frames -> INHERIT_LINE (tamp kept cells, default on in PR when inherit_nest=1) + plug carve; inherit_link=1 (off) keeps link runs to the door: joined/carved 0.31 -> 1.00. Side finding: nestcount flood seeds from every open cell in the door column (buried pocket crossing it counts as joined).
- **PR #835 (onto-main lane cmsg_01Mei3dwkcdzPxa5Jmuqi2bBSNK1UsH7nK55dfNqUZHk6F, 17:10Z):** compute work through 89453ec9 as off-by-default switches, bit-identical off, head 0814a931, with merge desk; #834 retargets onto it. Post-89453ec9 commits need a follow-up PR (so far only jobs files).
- After #835+#834 land on main: MERGE main into compute branch (not rebase), keep launching from it; follow-up PRs only for code since last landing.
- inherit_link=MIN (b9497657, PR #834): use inherit_link=20 (0 made-up-tunnel maps, 191/192 joined). Soil-loss lane done; resolve-able once #834 lands.
- Brood lane (17:24Z): owner ruled brood as full game, genome chosen at hatching; brood=lay on claude/project-thread-6einf2 a1c272e (off = identical); pilots unstable (boom/bust), needs lower mutation per child. Carries for it at /mnt/project-files/brood/carry/. Flags shipped Storeroom caste=4 reaching the bed.
- Plant-food lane 17:27Z: owner chose weaning bed; branch claude/project-thread-o2tizu 7eb4f68a (plants=, weed=30, fjoule=1, cap_to=, PP_MIND_GRAZE=1; constant cap 30 keeps 18/24 alive); asked it to add colsel pilecap=START:STEP:ALIVE and PR to main after #835; can run its jobs via --ref after colsel22. I can't cherry-pick/merge other branches into compute (auto-mode denied).
- plantwean1-1023 queued (startP.sh && csvmP.sh, --ref claude/project-thread-o2tizu, jobs plantwean1.txt, 1 job P=46 64 col 32 rounds pilecap=30:5:20) -> scratchpad/plantwean1; copy cg.csv/colsel.csv to /mnt/project-files/plant-food/plantwean1/ for the plant lane.
- #835 merged to main e254a60 (17:35Z); waiting on #834, then merge main into compute.
- Brood lane 17:50Z: with brood=lay cs20light carries all go extinct ~100k beside a full nest (store = crumbs; minds never pick up at home when hungry; banks ~87 grants mean they never learned). Recommended weaned crutch (hungry at home + food adjacent -> PickUp only, p weaned) + hungry-at-store CG case; branch 6einf2 57137d1; jobs slot after plantwean1.


**Why:** these deliveries stack on the Teacher lane's founder. **How to apply:** check main for #834 before merging main into the compute branch; see [[colsel-batches-1023c]].
- brood1-1023 queued after plantwean1 (b218i2c9e waits for scratchpad/plantwean1/DONE, then startB.sh && csvmB.sh; --ref claude/project-thread-6einf2 d8f8f890, jobs brood1.txt: brood1crutch PP_HOMEEAT=1.0 / brood1ctl, brood=lay lay_cap=48:8, wean=4:20 drives PP_HOMEEAT_P too, kpm=0.013 kevents=4). Copy cg/colsel csv to /mnt/project-files/brood/brood1/.
- plantwean1 result (18:55Z): cap fell 30->0 by round 6 because end-alive stays 24 (births refill) while starved/round 47..195; cap 0 -> extinct by r23. Nest 247->308 at cap 15. Asked plant lane: gate on starved, floor cap. Files /mnt/project-files/plant-food/plantwean1/.
- brood1 preempted ~19:05Z, relaunched as brood1b-1023 (startBb.sh/csvmBb.sh, chain task then plantwean2 via startP2.sh/csvmP2.sh, plant lane head cbc4125d pilecap=30:5:10:10). If chain task times out (2h), re-run waiter and start plantwean2 by hand.
