# Fed brood vs budding (main 5244c858 + branch claude/ant-breeding-plan-v9kpl5 @ dca0fb46, 2026-10-02)

Brood = eggs laid at 120 J, larvae fed by nestmates touching them and by food beside them, egg and pupa stages 500 frames each.
Budding = today's shipped births. Kin footing (PR 541) on in both.

## Long runs: colony bed, 20 founders, 192,000 frames, one run per seed
Each cell: ants alive at the end/fewest ever / fruit cells taken / born / starved.

budding g90: 124/20 t12625 b1849 s517 | 0/0 t3428 b447 s102 | 0/0 t3282 b469 s68 | 0/0 t5705 b911 s454 | 229/19 t13166 b2194 s892 | 0/0 t6971 b850 s172 | 
budding g200: 429/17 t8955 b1170 s5 | 50/18 t5695 b755 s55 | 193/13 t6723 b857 s15 | 0/0 t636 b19 s13 | 0/0 t289 b7 s5 | 120/14 t6055 b1329 s33 | 
brood   g90: 117/20 t5609 b864 s229 | 104/20 t9134 b958 s315 | 130/19 t7755 b935 s187 | 45/20 t10122 b1213 s328 | 133/19 t9664 b1162 s399 | 121/20 t8960 b979 s335 | 
brood   g200: 205/17 t10047 b838 s232 | 0/0 t3385 b361 s76 | 194/13 t8567 b813 s195 | 72/14 t4209 b522 s109 | 138/10 t8026 b753 s216 | 110/14 t4458 b511 s120 | 

Budding: 6 of 12 colonies alive at the end (0-429 ants). Brood: 11 of 12 (45-205 ants).
Brood took more fruit at both distances (mean ~8.5k vs ~7.5k cells at 90; ~6.4k vs ~4.7k at 200).

## Food box (digbox hungry gap=90), 4 seeds, at 96,000 frames
budding: live 189-675, born 412-1,218, fruit taken 3,770-9,345
brood:   live 230-376, born 489-718,   fruit taken 4,733-5,487

## What got it here (food box, main 44f14af)
- Pupae stuck without room to hatch: hatch up to 3 cells away, and on a nestmate as budded births do.
- Larvae stood unfed for tens of thousands of frames (38-50 at a time, ~40 ants' worth of energy). Fix: a nestmate touching a larva feeds it.
- Stages of 1,500 frames halved growth (born 216-338 vs 322-580 at 500).
- Lowering the laying bar made it worse (lay_at 600: 11-45 alive; 350: 0-5).

## More beds on 5244c858 (coordinator's ask): 140 cells, and the refilling pile (30 cells every 6,000 frames at 90)
budding u140: 663/19 t8659 b1425 s8 | 398/20 t10941 b1776 s93 | 0/0 t3203 b524 s154 | 0/0 t6274 b754 s85 | 802/19 t13675 b1886 s113 | 286/20 t9871 b1341 s25 | 
budding p90: 0/0 t735 b128 s123 | 14/1 t933 b154 s120 | 28/3 t959 b251 s204 | 0/0 t924 b94 s70 | 50/4 t946 b180 s89 | 4/2 t485 b24 s16 | 
brood   u140: 39/18 t4247 b358 s84 | 73/20 t3560 b308 s80 | 33/18 t4283 b376 s152 | 48/20 t5014 b403 s115 | 59/19 t4969 b420 s114 | 44/20 t3509 b373 s158 | 
brood   p90: 20/2 t933 b115 s86 | 38/6 t953 b113 s40 | 20/7 t954 b86 s46 | 22/3 t918 b121 s79 | 22/4 t920 b98 s49 | 8/4 t846 b41 s25 | 

Across all four beds: budding alive at the end in 14 of 24 runs, brood in 23 of 24. At 140 cells brood colonies are small (33-73 ants) and take about half the fruit; on the refilling pile both take all the fruit there is and brood starves about half as many.

## Nest (digbox fed, 8 seeds, 24k): about the same. 40 ants: dug 114 -> 108, alive 33 -> 30.5. 200 ants: dug 141 -> 155, alive 169 -> 164.

## Lab box (labforage played_bed, 120k, 24 paired seeds): brood is a regression
births 566 -> 266 (worse on 20/24), food eaten 1.31M -> 0.82M J (19/24), peak ants 309 -> 136, alive at end 213 -> 94, starved per million ant-frames 7.3 -> 10.8 (12/24, not significant), died out 0 -> 2 boxes.

## Lab box gap, split (labforage played_bed, 12 seeds, main 34b46b64 binary, brood on vs budding)
births / peak ants (budding 672 / 374):
- shipped brood (egg 120, stages 500/500): 270 / 125
- egg 900, stages 500/500: 338 / 183
- egg 1040, stages 500/500: 312 / 188
- egg 120, no stage delay: 565 / 361
- egg 1040, no stage delay: 764 / 472 (matches budding: the plumbing is sound)
The egg and pupa stages are most of the lab gap; the egg price is the smaller part.

## B2: graded fertility re-swept with brood on (main 8d7cfc9c tree, stages 250)
Food box, 96k, 4 seeds: fruit taken / live / born | food standing in the storeroom, elsewhere underground
- none (individual): 4.3-9.3k / 249-618 / 526-1,267 | store 0, under 1-5
- graded 1.25x: 4.6-8.5k / 261-538 / 366-917 | store 0-1, under 1-11
- graded 2x: 3.2-5.3k / 89-327 / 146-436 | store 0-16, under 5-31
- graded 3x: 2.1-2.5k / 36-71 / 61-111 | store 6-13, under 31-43
Long runs, 192k, 6 seeds (alive at end; mean fruit; starved per run):
- none: 90 cells 5/6, 9.4k, 70-361 | 200 cells 6/6, 6.6k, 73-265
- 1.25x: 90 6/6, 11.5k, 93-384 | 200 5/6, 6.7k, 8-174
- 2x: 90 5/6, 8.1k, 9-148 | 200 2/6, 1.9k
- 3x: 90 6/6, 8.0k, 1-137 (mostly under 10) | 200 1/6, 1.4k
Food only stands in the nest once suppression is strong enough to shrink the colony (2-3x), and at 200 cells those colonies die out.

## Re-test of FORAGE_DRIVE=returns,keep with brood on (dead-ends entry's condition met: brood now uses stored food)
Food box 96k, 4 seeds, main 8d7cfc9c + graded 1.25 default (PR 545 tree):
- returns: fruit 4.6-8.5k, live 261-538, born 366-917 | store 0-1, under 1-11
- returns,keep: fruit 1.8-5.2k, live 82-281, born 102-750 | store 0-1, under 2-8
Still no store, and the colony is smaller. Not adopted.

## Is it steadier? Colony size every 3,000 frames, frames 60k-192k (same B2 long runs, 8d7cfc9c tree)
Per seed: min-max ants, coefficient of variation, halvings from a running peak (peak >= 20).
- 90 cells: individual cv median 0.33, 9 halvings, 1 died | 1.25x cv 0.20, 3 halvings, none died | 3x 0 halvings but ~70k frames before growth starts.
- 200 cells: individual cv 0.38, 7 halvings | 1.25x cv 0.75, 5 halvings, 1 never grew, s5 227 -> 1 | 2x/3x: 9 of 12 founder groups never grew and died of old age.
Cause: the graded brake is distance to the nearest breeder (creature.rs graded_suppression_factor), blind to food and colony size, so a huddled founding group is braked as hard as a crowded nest.
Script: scratchpad stab.py (parses the FOOD STORE series' `ants N`).

## Energy banks (main 6a8dacd4 + bankdump, long runs 6 seeds, adults, frames 60k-192k)
Shares by bank band (<100, 100-200, 200-400, 400-700, 700-1.1k, 1.1-2k, 2-4k, 4k+), %:
- food 90, old rule: 2.3 10.2 38.1 14.9 9.9 11.1 8.5 4.7 | 1.25x: 1.6 10.1 42.5 15.5 9.8 9.6 7.5 3.4
- food 200, old rule: 2.1 9.3 29.7 15.5 12.0 14.4 10.8 6.0 | 1.25x: 0.9 7.5 36.5 15.8 11.3 13.0 10.8 4.3
Top tenth hold 41-48% of banked J; p99 6-8k J; max 19-41k J. Under 100 J: 1-2%. p10 ~197 (sharing stops at start_energy).
Where (1.25x, position found for ~80%): 2,000+ J ants median 34 cols from the nest (food 90) / 78 (food 200), 10-18% within 10 cols; ants 200-1,100 J ~32-39% within 10 cols.
Food standing at home (FOOD STORE 'nest food', median): 750 J (food 90), 1,360 J (food 200) at 1.25x.
Cause: BUD_SITE=nest holds laying away from home, so foragers living at the pile bank instead. The shipped game leaves BUD_SITE off.

## Where ants eat (main 6a8dacd4 + EATING counter, long runs, 6 seeds a gap, BUD_SITE=nest)
digested with the AtNest sense on / away / shared mouth to mouth (J):
- food 90: at nest 1.6-3.7% of digestion; shared = 23-45% of digestion; ~20% of shared goes to brood.
- food 200: at nest 1.4-7% (s2 nearly died); shared 4-36%.
The home store is not a source: foragers eat at the pile or on the way, and trophallaxis moves food home. Counter: CreatureStats::digested_at_nest_face (creature.rs digest block).

## Nest workers (id % 4 == 0, Storeroom::SHIPPED caste 4) vs others, bankdump positions, 1.25x, frames 60k+
- food 90: workers 27% of adults, 58% within 10 cols of the door (median 7 out), bank p50 313 / p90 1,019, 9% >1,100 J; others 26% within 10 cols, p50 438 / p90 2,779, 27% >1,100.
- food 200: workers 54% within 10 (median 8), p50 306, 7% >1,100; others 19%, p50 688, 37% >1,100.

## Appetite gate re-test (hungergate=1, digest_hunger_weight 1; long runs, BUD_SITE nest, main 6a8dacd4 tree): REJECTED again
alive/fewest, fruit taken, births, starved | share of digestion at the nest
- food 90 base: 630,334,444,462,61,259 alive; taken 6.1-14.8k | home 1.7-4.0%
- food 90 gate: 71,545,173,176,142,260 alive; taken 5.2-12.0k (about -40%) | home 3.9-5.3%
- food 200 base: 2 of 6 dead or at 1; gate: 4 of 6 dead (0,0,1,10,0,124)
Delivered food still doesn't stay down for others; re-test on the nest lane's dug home (NEST_HOME=dug).

## BUD_SITE=nest by default: lab box (labforage played_bed 120k, 12 seeds, bedenv, 6a8dacd4 + flip)
births 329 -> 4 (12/12 worse), intake 1.02M -> 0.16M J, alive at end 178 -> 0, died out 0 -> 10 of 12.
Trace (seed 1, 30k, budtrace): affordable ants median 9 cells from a nest column (p10 4), at_nest on 3% of affordable samples; eggs laid 66 -> 5.
Held: flip on branch claude/ant-breeding-plan-v9kpl5 (8c082cd9), no PR; re-measure with the nest lane's NEST_HOME=dug.

## Walk home to lay: does not rescue the lab box (0738a8ca + f6accdf9, head 231ab3d1)
Lab box, labforage played_bed 120k, 12 paired seeds, LAY_HOME off -> on: births median 4 -> 7 (higher on 7/12, p 0.34), died out 10 -> 10 of 12, alive at end 0 -> 0.
Trace (budtrace, seed 1, 30k): ~15 ants ever clear the bar; 90% of those samples are laden; they circle 4-10 cells from the painted nest and touch it on ~2%.
Rejected on seed 1 (30k): LAY_HOME=laden laid 5 vs 8; LAY_REACH=3 laid 3 vs 8. NEST_REACH=r6 oracle, seeds 1-4 at 120k: born 2-9, all died. Control BUD_SITE=anywhere: laid 66 in 30k.
Reading: in the lab box, births were paid by ants laying out at the food; the nest is a small painted patch that ready ants (mostly laden) don't reach.

## Food brake with small colonies exempt (c2214bf4): mixed, stays off
Long runs, 6 seeds, gap 90: s2, s4, s6 steadier, s3 crashed (0-368 ants, 4 halvings); births held 651-4,098 per run. Gap 200: near neutral, s5 improved (78-317, 0 halvings vs 1-234, 4). Food box: live 333-393 vs off 261-538. No food store forms in any arm.

## Lab box lays anywhere (a8a8ad1b, base 231ab3d1 = main 0738a8ca + branch)
labforage played_bed 120k, 12 paired seeds, nest-only (LAY_HOME=off logs) -> lab lays anywhere: births median 4 -> 329 (12/12 higher), died out 10 -> 0, alive at end 0 -> 178.5, peak 42 -> 188, starved per million ant-frames 3.8 -> 4.7 (9/12 higher, p 0.15). The lab is back to its pre-546 births. Main game unchanged.

## Re-check on main e8adc960 (dug home + piling) + branch b16787b4
Food box (digbox hungry gap=90, 96k, 6 paired seeds), LAY_HOME on vs off, live/born at 96k: on 189/282 198/414 119/425 243/424 276/552 274/420; off 227/479 311/426 49/228 188/375 338/647 252/426. Median live 220 vs 240, born 422 vs 426; higher on 3/6. Neutral, no colony lost: LAY_HOME stays on.
Lab box with lab lays anywhere, 6 seeds, 120k: born 84-796, alive at end 42-341, 0/6 died out.
