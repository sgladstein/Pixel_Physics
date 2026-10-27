# In-colony scoring review: hand-off for the Teacher lane (2026-10-22)

Full review (doc): https://claude.ai/artifact/S2h242Q6ebWYKjywSEBsUt
Read from examples/inrun.rs at compute head ee38ae5b, standard hA1 job args (ha1next.txt hA1c).

## Headline
The ledger pays single ants for acts; the owner's goals (big multi-chamber nest dug all game, no starvation) are colony
results no ant owns. Recommendation: freeze the per-ant ledger and add colony-level selection between the 64 colonies of a
batch, plus births paid from the store.

## Code-level findings (line refs are examples/inrun.rs at ee38ae5b)
1. dig_price (l.800-812) counts every open cell within +-100 cols below row max(datum,40), so the crater counts as nest; with
   a fixed 24-36 headcount it floors at 1 once ~25 new open cells per ant exist.
2. clear price (l.684) = 1 + k * loose_n/(2*alive): rises with loose spoil lying in the nest, i.e. pays MORE per bailed pellet
   as the crater fills. Distance ramp rim=15 -> full at 25 cols = middle of the food scatter (dist 25 spread 10).
3. inrun::birth / birth_in sets s.energy = kid_grants (3) full grants from nowhere; ~220 births per colony-round.
4. Parent rank is z with own-g2 noise (l.844-847); rank=shrunk built but off in the standard job.
5. Nest score (nest.csv) is printed only; store food, alive, starved are not in the ledger.

## Proposed (each behind a switch, off = byte-identical)
- colony selection: a round driver over N colonies; after each round rank colonies on (nest score, store food at end,
  alive, starved) - lexicase or rank-sum, no weights - and refound the bottom half from the top half's final pools.
- births=store: a child needs its starting tanks from the nest store (or parent bank).
- prices: dig price on nest-score chamber+tunnel cells, clear price without the loose multiplier.
- rank=shrunk on.

Local tests in progress (this thread, 4 cores): which terms pick parents; births=store; colony-selection prototype at
8 colonies x 6 rounds. Results appended here.

## Measured, 14:52Z (12 colonies, one standard 200k round of hA1, read-only pick.csv log added to inrun.rs)
- Selection differential (parent minus adult mean, points per 1k frames) over 1,302 quota births: food 87%, home meal 10%,
  everything digging (cut + cutter's exit half + clear) 3%, hungry 1%. Digging pays ~0.15/1k vs food ~1.6/1k.
- Prices across the round: dig 2.91 -> 2.13 (does NOT floor; my code-read guess was wrong), clear at its cap 3.00 from
  75k on, tank food price 1.00 until the last 25k. All three adaptive prices are effectively inert.
- z-rank parents agree with plain score on 271 of 1,022 picks (27%).
- Owner asked (14:51Z) whether ant scores should weight a job up when the colony is bad at it. Answered: the existing
  need-prices already try and are inert; weight can't buy a job ants barely differ in; put need in the senses and at
  colony level. Recommend NOT building the adaptive-weight step.
Patch for the part log: scratch worktree /home/user/sr (this lane); will copy the diff here.
Colony-selection prototype (colsel.py) running: 12 colonies x 6 rounds, plain vs colony; common garden after.

## Colony-selection prototype, 15:15Z (hand-off: this lane is closing; the Teacher lane owns the follow-up)
colsel.py: 12 colonies (seeds 1-12) x 6 rounds x 200k, hA1, standard job args, P=2 per arm on 4 cores (~3 min a round).
Colony arm: after each round rank colonies by rank-sum of (home8, new-ground cuts <=20 cols in the round's 2nd half,
starved (fewer better), final nest score); bottom 6 refounded from top 6's carried 30 (best donates to worst); extinct =
bottom. Common garden (cg.py): each colony's r5 carry in world c*1000+99, sel=off, 200k; base = hA1 mutants, old = nomind.

| arm | home8 med | cuts20 med | colonies >=20 cuts | starved mean | nest score med | alive after r5 |
| colony | 12,324 | 0 | 0/12 | 1.7 | 1 | 12 |
| plain | 4,131 | 132 | 9/10 | 5.5 | -90 | 10 |
| base hA1 | 270 | 201 | 12/12 | 3.9 | -119 | - |
| old ant | 652 | 298 | 12/12 | 0.0 | 143 | - |
Paired colony vs plain: home up 9/10, starved fewer 7 (more 3), cuts down 9/9. Plain matches your 64-seed CG (~3,900 /
~5 / ~130), so the setup is sane. Files: colsel-commongarden-12.csv, colsel-train-{plain,colony}.csv.

Reading: colony selection is the strongest lever measured (3x food, 1/3 starvation in 6 rounds) but it optimises what it
is scored on. The nest score is negative for every crater-digging colony and ~+1 for a colony that never digs, so it
REWARDS not digging; with food and starved also favouring all-forager colonies, the cuts rank was outvoted.
Next test (yours): colony selection with digging as a gate (no colony without >=N 2nd-half new-ground cuts or a chamber
can be top half) or as a lexicase case over colonies; drop the raw nest score until a no-dig colony scores below a
digging one. Same driver, one-line change in the ranking block. 64 colonies on the VM.
Not run: births from the store (free births are only ~4% of food flow; low priority).
