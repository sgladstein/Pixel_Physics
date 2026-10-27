---
name: dig-loss
description: Why in-run selection breeds digging out (2026-10-22): late-round generations never cut; short rounds tested and REFUTED at 64 seeds
metadata:
  type: project
  modified: 2026-10-22T11:41:21.130Z
---

Dig-scoring thread, 2026-10-22. Write-up and scripts: /mnt/project-files/nest-score/digloss/why-digging-dies-2026-10-22.md

- L60c1 (60 rounds x 200k, c1, score selection) lost digging by r4-r6. Fixed dig weights x2/x4 (digw-1022) only delay it.
- Individual trace: parent choice FAVOURS cutters (1.25x same-time peers, 34/41 rounds), cut rate inherited (r 0.35),
  hand-over to the next round neutral once time-matched (the unmatched "survivors cut 40% less" is an artefact: cutting
  falls through the round). Common garden (sel=off, same 3 worlds, 60k): round-0 ants ~170 cuts, s5 r2 19, r4 0.
- Cause: half of cuts in a round's first quarter, births even -> most of ~10 generations/round bred on food alone;
  food trades off with digging (correlated response).
- Local test (c1, 8 seeds, 800k frames): 16x50k rounds vs 4x200k -> common-garden cuts median 154 vs 0 (5 of 8 higher, 2 tied),
  births equal, common-garden deliveries ~25% lower. In-run food/starvation not comparable (fresh world resets).
- Caveat: random-parent arm also lost digging -> likely mutational erosion in unselected late generations, not only food drag; same fix.
- **REFUTED 13:15Z:** shortr-1022 (hA1, 64 seeds, 6x200k/12x100k/24x50k) common garden: cuts 131/190/116 vs base 201, paired 50k vs 200k 6 up 6 down. Round length is NOT the lever; the 8-seed c1 test was luck. hA1 erodes slowly (131 after 6x200k). Owner told.
- (old) Proposed to Teacher lane: shorter rounds (or a mix) as the "keeps digging" floor; judge at 64 seeds on hA1 with
  common-garden cuts. Adaptive weight alone can't act on ants that don't cut.

- **13:40Z real mechanism:** dig price barely falls (3.0->2.5), not the cause. Late in a round ants re-dig loose spoil in the crater: hA1L20 r0 new cuts 146->49 per 50k, recuts 254->461, spoil-in 43->206, pit 43->109 (same in r3, r6, and c1). Crater = treadmill; late generations bred on bailing. Fix = stop crater (refusal teacher) + pay new rooms (cell_values). hA1 x20 common garden: digging gone by r10 in 7/13, r19 10/12. Owner (13:15Z): ants must dig AND forage the whole game; a large multi-chamber nest is THE goal of this lane, never call it a 'design change'.

- **14:20Z refusals:** PP_SPOILVETO=R (mind.rs scratch, patch digloss/spoilveto-vs-98ad9dc4.patch) refuses spoil drop below datum or within R cols of door. With PP_RIMVETO=3, 1 round hA1 8 seeds: recuts 437->177, spoil-in 221->1, best home food, starved = plain; spoil veto ALONE starves (162 vs 63). 6x200k common garden (refusals off): median cuts 165 vs 134 but 3/7 lost digging; inconclusive, 64-seed batch suggested to Teacher lane. Room credit (w_room, patch roomcredit-vs-98ad9dc4) did nothing: rooms ~0.07/cell, cutters dead before paid. GIFs: digloss/gif/. My shortr worktree /home/user/shortr branch roomcredit reproduces hA1L20 r5 common garden exactly.

**Why:** cost an evening to find; next thread should not re-derive. **How to apply:** measure digging in a common garden
(sel=off, fixed worlds), never by in-run counts alone. See [[nest-score]], [[inrun-batches]], [[teacher-lane]].
