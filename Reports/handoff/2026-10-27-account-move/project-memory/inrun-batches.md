---
name: inrun-batches
description: Results of every in-run selection cloud batch (teacher lane, 2026-10-21/22): arms, food, extinctions, verdicts
metadata:
  type: project
---

Rule since the adversarial review (2026-10-22): compare arms paired by seed, food per colony-round over rounds both reached, extinction by exact McNemar. Most single-arm extinction gaps of 5-10 of 64 are noise (p 0.1-0.4).

**Batch 1** h1turn-1021 (h1, 64 seeds, 6 rounds, ~20 min): food/round r0→r5 random 80→136 (32/64 extinct); z30 137→3,047 (21); z15 122→5,607 (35); z60 118→3,047 (29). Turnover 30k chosen. Every nest.csv pit=0 (asked dig thread). Samples: /mnt/project-files/teacher-lane/cloud/h1turn-1021/.
**Batch 2** rimh6-1021: h1/h6 x rim_rows 0/3/3+debit0.5, z30.
**Batches 3-4** (rimshape-1022, ctl-1022; nest score with PR #780 pit fix): control 20/64 extinct, nest -53, pit 45. Every rim shape (pay 0 / half / deep x2 / both) costs 10-19 colonies and lifts paired nest score ~20, pit -1/3. v2 founder 38 extinct. Decision: dig pay unchanged, feeding first. xargs -L joins lines ending in a blank (fixed: VM strips them). Batches deterministic across runs (rimshape share0 = rimh6 h1r3 exactly). Speed flags: ~10% per job.

**Batch 5** bankcap (PP_BANK_CAP, scratch species change): food 2.4-2.7x, extinct 25-33 vs 20; not proposed.

**Food trace (2026-10-22):** only 1-3% of h1's field bites are delivered; the old ant 25%, 0 starved. Details in [[foraging-loop-lane]].

**Batch 6** gate2-1022 (hungergate=1 digest_bar=3/5.5; the bed's 1e30 breeding bar otherwise leaves the gate inert; ant.ron digest_hunger_weight is 0): see recount below. Trails: A saturated (no gradient), B a path signal.

**Batch 7** oldref-1022: old ant (`nomind=1`, sel=off, round 0) 747 delivered/colony, 0 starved, nest +147 = the bar; h1 sel=off 80, nest -124. **kid_grants=3 is now the control** (food 11,076 vs 6,923/seed, extinct 17 vs 20 = noise, paired 15 vs 12). Extinction per seed is near coin-flip: judge arms on food and paired counts. Buried larder: 60% (ctl) / 25% (gate) of living colonies' store food walled off at round end; seed 12's was still open at its die-off (earlier claim corrected). Replay page has a new/old ant switch.

- Batch 8 strict-1022 (2026-10-22, build 20275910): st0 = old score (reproduces kid3 exactly: 11,076 food/seed, 17 extinct), st1 = strict=1. Paired: home food per colony-round 925 vs 1,362 (st1 higher 29/64, n.s.), extinct 22 vs 17 (p=0.44), starved/round same, nest score -69 vs -44. Strict stays OFF. Field drops were only 2-5% of deliveries after round 0.


**2026-10-22 night (home food = deliveries within 8 cols, paired per colony-round):** c1-1022: c1 founder 2,319 vs h1 1,431 (46/64, p=0.001), extinct 9 vs 17 -> c1 is the founder; c1 sel-off starves 16.6/24 vs h1 10.6 (old ant 0). nestzone-1022: zone/ztop/c1z cut food 1/3-1/2 (p<=0.06), near-nest crater unchanged (16-19 vs 15-16 cells, nearpit.py) -> NOT adopted. long60b-1022: maze graft c1g15 on c1 -> 49 vs 9 extinct, home 630 vs 1,166: NOT adopted (ants follow trails, stop foraging); + 60-round c1 score vs random. Recount (same-batch controls): appetite gate hg3 home 1,812 vs 598 (p<0.001), extinct 30 vs 20 (p=0.10) -> reopened, c1hg3 queued (jobs next1.txt with warmoff). Design doc rev 46 has the paired table ('Where it stands, 22 October (night)').

**night2-1022:** c1hg3 (appetite gate on c1) home 5,009 vs 2,475 (47/64), extinct 18 vs 9 (p=0.08) -> decision card to owner 09:10Z (recommended: check lab game first). warmoff: no food effect, extinct 17 vs 9 (p=0.15). c1gl (gated graft): food 1,914 vs 2,373 (p=0.03), not adopted. L60c1 60 rounds: food 987 -> ~7,000/round by r20 plateau, 50/64 survive, but digging bred out (dug = 26-cell stamp from r6, pit 0, both score and random arms). Later batches: [[inrun-batches-day2]].

**Why:** later threads should not rerun these arms. **How to apply:** see [[inrun-cloud]] for how to run, [[inrun-selection-traps]].
