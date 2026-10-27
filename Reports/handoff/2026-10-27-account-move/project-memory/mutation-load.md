---
name: mutation-load
description: In-run mutation at the default kpm 0.013 outruns colony selection -- every trait decays; kpm 0.003 keeps and improves them (drift1, 2026-10-27)
metadata:
  type: project
---

Teacher lane, 2026-10-27 (owner asked 02:40Z why selection drops a good habit). Batch drift1 (a88231d9), all from the same latch_pool with lexicase over nest11's cases, 48 rounds. Garden starvation from r7 to r47:
- default kpm 0.013: 46 -> 311, nest 672 -> 143
- kpm 0.003: 51 -> 28, nest ~700 kept (p=6e-7 starvation and p=9e-6 nest, both vs default; improved on its own r7, p=0.003)
- kpm=0 kevents=0: flat at 37
- plain (no colony selection): extinct by r15

Selection DOES favour low-starvation colonies (kept half 144 vs dropped 179), and colony starvation repeats across worlds (+0.89). Mutation load swamps it: about 26 changed weights per child (pm 0.013 over ~2,000 live links, sigma 1) plus 4 structural events, at every birth.

**How to apply:** Teacher-lane batches use `kpm=0.003` from now on. Whether the inrun default itself should change is still open (it affects every lane's training). The owner was told 04:40Z. Logged in PR #937 (head 92028163, merge desk).

Related: [[gate-step2-result]], [[batch-design-rules]] (the in-lab evolution low-rate note), [[report-significance]].

**self1 (2026-10-27 08:30Z, std_pool, 32x96 rounds, seed 31, garden r7->r95, Mann-Whitney over colonies):** kpm 0.003 improved nothing (starved 292->281 p=.8; food home 4,168->2,720 and nest 663->428, both p~5e-6). Default kpm: food home 4,106->7,852 (p=1e-5) bought with nest 596->129 (p=9e-9). Instinct pool at 0.003: starved 46->112 (p=3e-5). Lower rate alone is not the fix.
**elite1 (launched 08:34Z, ref 07c50df9, scratchpad elite1/):** colsel `elite=1` = kept colonies run kpm=0 kevents=0, only refounded children mutate. Arms e3/e13/ehl3 = self1's arms + elite. Compare against self1 at r95.
**elite1 RESULT (12:20Z):** elitism fixes drift. std_pool kpm .003 + elite r7->r95: starved 330->281 (p=2e-4), food home 4,168->5,714, crater 76->8, nest held; default kpm + elite: starved 296->219, nest 626->746. Instinct arm + elite still trades starved 38->86 for food +70% (selection over cases, not drift). Come-home not found in 96 rounds. Logged PR #938 (head 8916708b, merge desk). **Use elite=1 in every colsel batch from now.** elite2 (resume elite1-r1 -> 192 rounds) launched ~12:35Z, scratchpad elite2/.
**Selection review (PR #939, 14:07Z):** in-round ant score fights colony cases; one round's colony rank ~noise (0.05-0.09); lineage collapses to one founder colony by r15, one ant line by r95; std_pool near-clonal (~35/2,035 weights differ). Built colsel refound_mut=/carry=random/sel= passthrough + seed_of() (23d56a4c) and scratch/inrun/exam.py (held-out worlds 500001+, shipped lab boxes). clean1 jobs (k3 ctl, kdec, kgoal, kboth; 32x48) queued for the VM after elite2 -- launch at 23d56a4c.
**Exam (14:40Z, scratch/inrun/exam.py, data /mnt/project-files/selection-review/exam/):** shipped lab boxes, no grant: trained brains (std, hl, elite1 winners) die out in two_larders/scattered/feast_and_famine (lived 4-5 vs old ant 34-147, 0 deliveries in two_larders); handed to env review thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB1X3bY6CcHJAV4BDfsHUTpC. Held-out gardens: deliveries +28-44%; hl starved 43.5 vs std 262; ehl3 winner deliveries x1.9 but starved 75 vs 43, open nest/rooms down (p=5e-4).
**16:30Z:** elite2 r95->r191: no further progress (e13 starved 219->340 p=4e-5). clean1 launched 16:23Z at bc8485fb (k3,kdec,kgoal,kboth,ktone,kev1), waiter -> scratchpad clean1/. Next: env1 (food=all, windfall labscen, donor600, lift harness rules one at a time) + write1 (PP_WRITE=on vs off from std_pool; layout 8 = PR #942 eeb7611; members 4625 slots). Scratch disk fills on big pulls: delete colony dirs right after pulling.
**Windfall/corpse (18:25Z):** trained ants recognise windfall (FOOD kind) but windfall rots (~1,300 frames dry) before their slow wall-hugging search finds it (PickUp option in 104 vs 1,120 decisions, two_larders variants, PP_MIND_LOG). Corpse: mind PickUp needs diet_yield > threshold at the ant's gut (~-1 vs class +1 -> ~0), so trained ants are never offered corpse; old ant uses separate code. food=all includes corpse -> mind can't use it unless gut evolves (cliff).
