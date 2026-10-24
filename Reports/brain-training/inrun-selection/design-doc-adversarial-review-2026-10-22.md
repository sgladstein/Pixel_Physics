# Adversarial review: in-run selection design doc

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `teacher-lane/`. It reviews [`design-doc-2026-10-22.md`](design-doc-2026-10-22.md) in this folder.

p-values are paired exact McNemar tests over 64 seeds against `cloud-ctl/ctl`, computed by `review/tools/pair.py`.

1. **"Colonies survive" measures the harness, not the brain. CONFIRMED.** Over 302 colony-rounds, ctl's births.csv logs 31,326 births, 25,353 refills and 9,883 hall refills. That is about 220 newborns per colony-round, each with free tanks, and about 100 starve each round. "Extinct" only means the hall ran dry. The old ant ran with no births, so it is not comparable. The gate "most of 8 seeds get through 3 rounds" can pass while half of every brood starves. kid3's food gain (6,923 → 11,076) comes from extra free energy at birth, not from a better brain.
   *Fix:* score starvations per 1,000 ant-frames in a fresh colony with births and refills off, and run the old ant under the same rules.

2. **Rankings that rest on noise. CONFIRMED.**
   - "h1 is the better founder" (h6 28 vs 20 extinct): p = 0.22, and the probe favours h6.
   - Bank cap and appetite gate rejected for "30 to 33 extinct": hg3 p = 0.10, cap3 p = 0.09. cap8 (25, p = 0.42) is left out of the quoted range.
   - "Every dig change costs 10 to 19 colonies": share5 p = 0.10.
   - "Rim cutters delivered more in 9 of 12 seeds": p = 0.15.
   - Random vs score on extinction: p = 0.058.

   Only v2, deep1, z15, share0, h1r3 and deep1s5 have p < 0.05, with no correction for testing many arms. kid3's −3 extinctions is called noise, but cap8's +5 is called harm.
   *Fix:* put discordant pair counts and p-values in every table, and apply one noise bar to gains and losses alike.

3. **Food per seed is pooled over runs of unequal length. CONFIRMED** (`armsum.py` sums every round that exists, so extinct seeds contribute fewer). Among colonies that survived, the dig arms deliver *more* per round in round 6 (share0 4,230, deep1s5 4,903, against ctl 3,037). That contradicts "the shift toward digging pulls selection away from feeding". The cap and gate arms delivered 2.4–2.7x more food and had better nest scores (−14 to −30 against −53), yet were rejected on extinction gaps mostly inside noise.
   *Fix:* report food per colony-round, paired by seed over the rounds both arms survived, and report survival separately.

4. **The score as built is not the one documented, and it pays draining the store. CONFIRMED** (`inrun.rs` 427–613).
   - There is no debit for eating at home, so an idle ant scores 0, not "below zero". The check "an idle ant ranks lowest" fails.
   - A home meal pays +2 after any time spent 15 or more columns out (624–630).
   - `w_hungry` pays 10 for crossing from 12 to 15 columns below 2 tanks, with no bite required.
   - So "step out, come back, eat from the store, stay lean" is paid. That is the die-off pattern.

   *Fix:* require a field bite before either payment, and rerun the six score checks on the score as built.

5. **"Delivered" counts more than the game's deliveries. CONFIRMED.**
   - `FOOD_DROP` is flagged on any put-down of a trip cell, before the `at_nest` test (creature.rs:20977). `inrun.rs:487` pays all of them.
   - In local logs, 22 of 133 (bury/s52) and 13 of 121 (gatetrace/s12) "deliveries" sit 13–16 columns out, beyond home.
   - Progress pay is taken back only if no cell of the trip arrives (583–586), so delivering one cell and eating the rest keeps the progress pay for all of them.

   *Fix:* pay only when the game's own `trip_deliveries` count rises, and take progress back per cell.

6. **More geometry holes. CONFIRMED in code; the behaviour is SUSPECTED.**
   - Clear pay reaches full at 25 columns (line 536). That is the centre of the food scatter (`dist=25 spread=10`), so ants are paid fully for dumping spoil on the food.
   - `w_depth` pays up to 3x for food dropped at the crater floor, which may be why stored food ends up buried.
   - Cloud batches keep no `events.csv`, so none of this can be audited.

   *Fix:* keep the event logs, and census where credit is earned in every round.

7. **Food delivered has stopped tracking the goal. CONFIRMED.** In round 6, selected colonies deliver 3,037–9,813 cells per round, 4–13x the old ant's 747, yet starve about 100 ants a round against 0. Food stored in the nest is no higher (17–25 cells against 25). "About 20x more food" reports the score, not the goal. None of the doc's judges (fresh colony, 96 evaluation worlds, one-brain clones) was run on any cloud batch.

8. **Baselines mix builds. CONFIRMED.** Three identical configurations give different results: ctl, z30 and h1r0 have 20/21/20 extinct and 6,923/6,857/6,906 food. The doc uses both 6,906 and 6,923 as the baseline, and the "Softer rim" control comes from a different batch.
   *Fix:* run a control in every batch and record the build hash.

9. **"Before long runs" has items missing and out of order. SUSPECTED.**
   - The score checks and calibration, and the judge, must come before "clean comparison reads".
   - The arena-selection arm at equal CPU, which was the pilot's question, is gone.
   - The positive control (v2 spreading into s171) is never reported.
   - Eight seeds cannot tell 31% extinction from 53% (SE ≈ 0.17).
   - No audit for score hacks (item 6) and no per-batch control (item 8).
