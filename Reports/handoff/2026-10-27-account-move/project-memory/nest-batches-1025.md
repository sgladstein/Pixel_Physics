---
name: nest-batches-1025
description: Teacher lane nest batches 2026-10-25 (nest1-nest5): what each found; full record in the repo's colony-selection log
metadata:
  type: project
  modified: 2026-10-25T21:02:11.579Z
---

Full record: Reports/brain-training/teacher-training/colony-selection-log-2026-10-24.md (PRs #918, #921).

- **Pictures 15:20Z** (https://claude.ai/artifact/7CykXk4N8PiKzo1BEomvVz): c8/c10 open nest 70-150 vs hand-built 980-1,250; ~80% of the trained dug space is stored food.
- **Owner 15:15Z** asked to redo the old-rules scaffold with a slower phase-out; per-ant dig weight was already tried (no hold).
- **nest1** (heaps=20:120): random distance alone erodes the nest (116); scalegate gets it back (424) but grows food; rules arm level.
- **Rooms score fooled** (replay https://claude.ai/artifact/1MgsAwA6bprQ1o73HaCs6B): heapsgate c9 scored 85 on 17 open cells vs hand-built 36 on 732. OPEN_ONLY fix: trained 0-1, hand-built 19-100.
- **nest2** (scaleopen=1, https://claude.ai/artifact/7PtyjnZKCrGeCNpUAJusqt): open-room gate alone failed. With rules weaned r6-r20, open median 42/168/50 at r7/15/23 (control 38/28/26), so open room peaks while the rules are on and halves after.
- **nest3** was stopped 19:20Z: garden food too close (owner).
- **nest4** (patches=20000:40:120, 48 rounds, https://claude.ai/artifact/8SCJ54UxXuWroAWGgtShy6): no arm improved. Garden starved 110-276 vs hand-built 11; rules arms die out (9 and 15 of 32). Survivors hoard (rooms c21: 83 alive, open 87, store 655).
- **ROOT:** c10 on patches 20-40 dies out in 3 of 4 runs beside 175-315 stored food cells it never eats. The brood lane saw the same on 10-23 (PP_HOMEEAT crutch; that crutch arm cratered 200-371 on fixed heaps).
- The trained colony delivers 8x the hand-built ant's food but burns most of it; its store only grows (18 -> 472) while the hand-built ant's ends at 6.
- **Replay recipe:** colony carry from cN/rR/final/index.csv; garden seed c*1000+99; reproduces exactly. Hand-built: founders=scratch/inrun/genomes/hA1.raw.txt:1 mutants=30:0.03:1.0:7 nomind=1.

See [[nest-priorities]].
