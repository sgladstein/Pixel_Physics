# Selection review: how ant-level and colony-level selection balance (2026-10-27)

*Selection review thread, asked by the owner on 2026-10-27 ("review our scoring/selection mechanism, both in-colony ant
selection and between run colony selection. How they balance between each other. If our scoring functions are ideal
and research to see if there are improvements"). Read-only analysis of the Teacher lane's `drift1`, `self1` and
`elite1` batches plus the code at main `76e7564`. Nothing was trained. Pictures:
<https://claude.ai/artifact/S4yqktKEjHwFW8YJgyLyWG>. Every table below is printed by
[`scratch/inrun/selaudit.py`](../../../scratch/inrun/selaudit.py); its full output for all eight arms is
[`selection-review-2026-10-27/audit.txt`](selection-review-2026-10-27/audit.txt).*

Earlier reviews this builds on, not repeated here: the
[in-colony scoring review](in-colony-scoring-review-2026-10-22.md) (the ant score is a food score; adaptive prices are
inert; a summed colony score paid not digging), the [in-run selection method](in-run-selection-method-2026-10-21.md)
report (Waibel 2009, clone colonies, shrunk means), [noisy fitness](q1-noisy-fitness-2026-10-21.md), and the
[colony selection log](../teacher-training/colony-selection-log-2026-10-24.md) through `elite1`.

## In plain words

- **Two kinds of selection train the brain.** Inside a round, ants that earn the most pay get the young (about 500
  hatchings and 330 culls per colony per round). Between rounds, the better half of 32 colonies is kept and the worse
  half is replaced by copies of a few winners.
- **Today the in-colony level steers and the colony level mostly rolls dice.** Among the colonies it is choosing
  between, the order that decides survival repeats from one world to the next at only 0.05-0.09, no better than a
  control in which all colonies carry the same ants. Fresh children take 46-49% of the kept half each round; 50% would
  mean survival cannot tell a child from a proven colony at all.
- **The two levels pull opposite ways.** One round of mutation plus pay-ranked breeding leaves a child colony worse
  than its own donor (same ants, same round, no mutation) on the nest, late cuts, nest scale and above all lab
  survival, and a little better on food home. The colony vote leans the other way: the six nest cases carry 60-70% of
  it. Across colonies, food home and nest size trade off (-0.40).
- **The owner's weighting question has a concrete answer.** Lexicase only chooses which of about four donors each
  child copies. Survival is a plain sum of ten ranks, so each case is a tenth of the vote and cases that move together
  stack up. The two median gates (lab survival, then nest scale) are the strongest weights in the system.
- **Several cases are noise.** Room shape, spoil and the herb box do not repeat from world to world; the herb box's
  ranking is 62% set by its own world seed. Starvation repeats in the std pool arm (0.21) but not in the instinct arm
  (0.02), which is why that arm's starvation could rise under elitism: selection cannot see it there.
- **The pool is one family.** Pay-ranked breeding narrows ~24 carried lines per colony to 2-3 within each round, and
  at round 95 every ant in all 32 colonies descends from one ant carried 7 rounds earlier (22 in the instinct arm).
- **What to change first:** make the child round do less damage (measured by the child-versus-donor table), give
  each goal one well-measured case with an explicit weight, and keep more than one family alive. More worlds per
  colony help only at proportional compute, and only after those.

## 1. The two levels as they run today

**In-run (inside one 200k-frame round, `examples/inrun.rs`, `STANDARD`).** Every 500 frames adults are ranked by z,
(credit rate - colony median) / (own credit noise), where credit is the ledger: deliveries (`w_deliver=7`, times up to
3 for depth), progress pay, paid home meals (`w_home_eat=2`), hungry departures (`w_hungry=10`), cuts and spoil, minus
`w_field` per field bite. With `brood=lay` each hatchling's mind is a copy of a sire picked down that ranking (best get
up to three); quota culls remove ants below the median; `kid_cap=4`, `retire=1`. At round 95 of `elite1` a colony had a
median **552 hatchings and 331 culls** (std pool) against 136 starved; that is roughly eight ant generations a round.
Births mutate at `kpm` with `kevents=4` structural edits each (none in kept colonies under `elite=1`). The round's
**carry** is the top 30 living ants by plain credit rate (`colsel.py carried()`), not by z.

**Colony selection (`scratch/inrun/colsel.py`, mode `lex+spoil+crater+shape+scale+lab`).** After each round:
1. the dig gate (kept late cuts >= 5) is passed by 30-31 of 32 colonies, so it no longer binds;
2. gate-passers are ordered by the **sum of their ranks on ten cases** (food home, fewer starved, kept late cuts, nest,
   less spoil, less crater, room shape, nest scale, lab, herb box). The top 16 are kept;
3. donors are the kept colonies at or above the median lab case (`labgate=50`), then at or above the median nest scale
   (`scalegate=50`): **a median of 4 distinct donors a round** in every arm (range 2-8);
4. each of the 16 replaced colonies copies a donor's carry, the donor picked by epsilon-lexicase over the ten cases
   among those ~4.

Each colony plays a different world each round (seed c*1000+r), one world per colony per round.

## 2. Findings

### 2.1 Colony survival is mostly luck among the colonies it chooses between

A colony kept after round r-1 and after round r runs its unmutated carry (`elite=1`) in two fresh worlds, r and r+1.
Spearman of each case across those colonies, averaged over 86-90 round pairs, 90% bootstrap:

| case | std pool, kpm .003 (e3) | std pool, default kpm (e13) | instinct, kpm .003 (ehl3) | control: no mutation (lnomut) |
|---|---|---|---|---|
| food home | **0.48** [0.42, 0.53] | 0.30 | 0.29 | 0.07 |
| fewer starved | 0.21 [0.14, 0.27] | 0.10 | **0.02** [-0.05, 0.09] | 0.04 |
| kept late cuts | 0.25 | 0.08 | 0.15 | -0.05 |
| nest size | 0.41 | 0.21 | 0.16 | 0.03 |
| less spoil | 0.12 | 0.03 | 0.02 | -0.05 |
| less crater | 0.22 | 0.19 | 0.06 | 0.06 |
| room shape | 0.01 | 0.09 | 0.03 | 0.03 |
| nest scale | 0.43 | 0.34 | 0.23 | 0.09 |
| lab (summed) | 0.21 | 0.42 | 0.24 | -0.05 |
| herb box | 0.07 | 0.00 | -0.05 | -0.05 |
| **survival order (rank-sum)** | **0.05** [-0.02, 0.11] | 0.06 | 0.09 [0.01, 0.15] | 0.09 [-0.00, 0.18] |

Controls: `lnomut` (drift1, kpm 0, colonies converged) is the negative control, ~0 on every case. The positive control
pools kept colonies of e3 and ehl3 (instinct versus no instinct, a real difference in starvation): starvation retest
**0.66**. So the instrument sees real differences when they exist.

Why the sum repeats less than its parts: the parts trade off between colonies (2.3), so their real differences cancel
in the sum while their noise adds.

What that does to survival: **children (colonies that mutated last round) take 49%, 46% and 48% of the kept half**
(e3, e13, ehl3) although they are measurably worse than their donors (2.3). A kept colony's median stay is **one
round** (mean 2, max 12-17).

**The world itself is a small part of the noise.** Colony c in round r plays the same world in every arm, so two arms'
within-round colony rankings agree only as far as the world sets them. They agree 0.0-0.2 on every bed case
(e3~ehl3 rank-sum 0.04, e3~s3 0.12), but **0.62-0.70 on the herb box**: that case mostly ranks lab seeds. Replaying a
child on its parent's world therefore buys little over a fresh world for bed cases; the noise is the colony's own
chaotic history, not the map.

The training round also predicts the common garden only weakly (same carry, fresh world, `sel=off`): food home 0.3-0.5,
nest 0.2-0.4, starvation 0.04-0.22 across arms. Training rounds cull ~330 low-ranked ants each, against ~40-140 starved,
so part of what would starve in the garden is culled first in training (a hypothesis, not traced).

### 2.2 How the measures are weighted

Each case's real share of the survival vote, cov(rank_k, rank-sum)/var(rank-sum), averaged over rounds, and the
standardised selection pressure (kept half and donors against all colonies, in sd, + = better):

| case | e3 share | ehl3 share | e3 pressure, kept / donors | ehl3 pressure, kept / donors |
|---|---|---|---|---|
| food home | 10% | 6% | +0.15 / -0.20 | +0.11 / -0.03 |
| fewer starved | 13% | 10% | +0.40 / +0.32 | +0.32 / +0.40 |
| kept late cuts | 15% | 17% | +0.46 / +0.67 | +0.56 / +0.81 |
| nest size | 18% | 19% | +0.47 / +0.78 | +0.60 / +0.83 |
| less spoil | 12% | 11% | +0.35 / +0.46 | +0.32 / +0.48 |
| less crater | 10% | 8% | +0.28 / -0.04 | +0.21 / -0.06 |
| room shape | 3% | 4% | +0.00 / -0.36 | +0.07 / -0.20 |
| nest scale | 5% | 12% | +0.18 / +0.83 | +0.41 / +1.04 |
| lab | 7% | 9% | +0.24 / +0.96 | +0.26 / +0.86 |
| herb box | 6% | 6% | +0.18 / +0.03 | +0.17 / +0.10 |

So:
- **Survival** gives the six nest cases 63-71% of the vote, food home 6-13%, starvation 10-13%, lab 13-15%. "No
  weights" holds only in name: the weight of a goal is the number of correlated cases written for it.
- **Donor choice** is set by the two median gates: lab (+0.86 to +0.96 sd) and nest scale (+0.83 to +1.04 sd). Food
  home is not selected among donors at all (-0.20 to -0.03), and room shape is selected *against*.
- **Lexicase** picks among ~4 donors and so matters little. The literature agrees it is the wrong tool for a handful of
  contradictory goals when a compromise is wanted: it selects only colonies at the corners of the trade-off (best on
  at least one case), never the middle (La Cava et al. 2018, theorem 3.4; Shahbandegan & Dolson 2024: "lexicase
  selection is obviously a bad choice for most classic two and three objective problems").
- About a quarter of the survival vote (spoil, shape, herb box, and starvation in the instinct arm) goes to cases that
  do not repeat, i.e. to luck.

### 2.3 The two levels pull opposite ways

A child and its donor in the **same round** start from the same carried ants; the donor (kept) runs with no mutation,
the child mutates and breeds by pay rank. Counting pairs where the child is better / worse (1,520 pairs per arm; pairs
share donors and rounds, so read the p as optimistic):

| case | e3 child worse | e13 | ehl3 | control lnomut (no mutation) |
|---|---|---|---|---|
| food home | 47% (median **+90**) | 52% (-79) | 47% (+75) | 49% |
| fewer starved | 54% | 53% | 51% | 53% |
| kept late cuts | 56% | 54% | 58% | 52% |
| nest size | 57% (median **-38**) | 59% (-50) | 60% (-52) | 54% (-20) |
| less spoil | 57% | 55% | 57% | 47% |
| nest scale | 56% | 53% | 59% | 48% |
| lab | **63%** (-58) | **69%** (-93) | **65%** (-65) | 54% (-11) |
| herb box | 49% | 49% | 46% | 52% |

Sign tests (e3): nest p = 2e-7, lab p = 4e-24, food home p = 0.02 (better). The control stays within 47-54% (its nest
p = 0.02 and crater p = 0.007 are the residual the instrument shows with no mutation; the mutating arms are 10-50
orders of magnitude past it on lab). Between colonies (case at r against another case at r+1, kept-twice colonies, so
no shared world), e3: food home with nest size **-0.40**, with nest scale -0.39, with late cuts -0.32; fewer starved
with food home +0.25.

Reading: **the in-colony round is a variation operator with a direction.** Its children carry the ant score's taste
(food) and lose what the score does not pay for (the nest kept at the end, surviving in a lab box). The colony vote
leans on the nest. The population sits where the two forces balance, which is why progress is slow and why
`self1` at the default rate traded the nest for food (596 -> 129) while elitism held it.

### 2.4 The pool is one family

From `elite1`'s round-95 per-ant files: colonies start the round with ~24 carried lines; the lines still hatching young
fall to a median **7, 5, 3.5, 3, 2** over the five 40k-frame fifths (std pool; 7, 4, 3, 3, 3 instinct). At the end a
colony holds 2-3 founder lines. Across all 32 colonies, every living ant descends from **one ant carried 7 rounds
earlier** (e3, e13) or 22 (ehl3), and one of the 24 starting pool members has descendants. The owner's rule is that
the colony's mixture of genomes is the product; what the training produces is close to one brain lineage. This is
the "mixed colony, colony-level selection" setup that Waibel, Keller & Floreano (2009) found worst, sliding toward the
near-clonal colonies they found best, but by in-run collapse rather than by design.

### 2.5 The instinct arm's starvation rise

The Teacher lane read `ehl3`'s garden starvation rise (38 -> 86 under elitism, with food home +70%) as the rank-sum
trading starvation for food. The numbers point to something simpler: in that arm starvation **does not repeat**
(0.02), so selection cannot see which colonies starve less, while food home repeats (0.29) and can be selected. A trait
selection cannot see is carried by whatever children happen to be accepted, and children are accepted at random
(48%). Children in that arm are not worse on training starvation on average (51%, p = 0.57), but training starvation
predicts garden starvation at only 0.04.

### 2.6 Seed overlaps

- Training seed c*1000+r equals the garden seed c*1000+99 at round 99 (the Teacher lane's catch): runs past 99 rounds
  train once on their own garden world.
- The lab case seeds c*1000 + r*10 + k reach colony c+1's seeds from round 100 (r*10 >= 1000).
- `labcase=` seeds use r+100(k+1), i.e. c*1000 + 10r + 1000(k+1): colony c's herb box seed equals colony c+1's first
  lab seed in the same round (different scenario, same seed). Harmless today, but the scheme should hash
  (purpose, colony, round, k) rather than add.

## 3. What the literature says

- **Level of selection.** When the goal is a group outcome, selecting groups works when competition inside the group
  does not oppose it. Muir (1996) kept or culled whole hen families by cage results: annual mortality in cages fell
  from 68% to 8.8% in four generations, where selecting single best layers had bred aggression. Waibel, Keller &
  Floreano (2009) crossed team composition with level of selection in simulated foraging: genetically mixed teams under
  team selection did worst, same-genome teams under team selection did best wherever cooperation was needed. Our in-run
  breeding is the "individual" level running inside the "team" level, with ~8 generations a round against one colony
  decision.
- **Lexicase.** Built for many shared test cases, where it keeps specialists that solve different subsets (La Cava et
  al. 2018). With few, contradictory objectives it selects only the corners of the trade-off and loses diversity
  ("hyperselection"), and epsilon makes it worse on contradictory objectives (Shahbandegan & Dolson 2024). Our cases
  are also measured on a different world per colony, so no colony is "best on a case" in the test-case sense.
- **Noisy fitness.** Jin & Branke (2005) survey the two answers, explicit averaging (more worlds) and implicit
  averaging (larger populations); which wins depends on how reliable one evaluation is. The project's own
  `q1-noisy-fitness` report covers racing and resampling. colsel already solved this once for spiders
  (`spiderevo` ELITE: keep every life, rank on a mean shrunk toward the round median).

## 4. A toy model, for ranking ideas

[`toy_model.py`](selection-review-2026-10-27/toy_model.py): 32 colonies, one world each, truncation at half, 4 donors,
world noise sd 1, between-colony sd 0.25 (repeatability 0.06, as measured), child = donor - 0.28 + N(0, 0.30) (children
worse than their donor in ~58% of pairs, as measured). It reproduces the measured **47% children share** of the kept
half. Gain after 96 rounds, 40 runs:

| change | compute | gain | children in top half |
|---|---|---|---|
| today | 1x | 3.97 | 47% |
| running shrunk mean for kept colonies (prior 2) | 1x | 3.69 | 25% |
| 16 colonies x 2 worlds | 1x | 3.56 | 45% |
| 32 colonies x 2 worlds | 2x | 6.10 | 46% |
| children carry a third of today's damage (load 0.1) | 1x | 12.92 | - |
| donors 1 / 2 / 4 / 8 / 16 | 1x | 3.0 / 3.9 / 4.0 / 3.0 / 2.1 | - |

The running mean only helps when children are mostly worse with little spread (load 0.28, spread 0.15: -3.0 today,
-0.4 with it), because it protects kept colonies and so admits fewer children, which are where gains come from. The
model has one trait, so it says nothing about trade-offs or the value of keeping several families; it is a sketch, not
a forecast.

## 5. Recommendations, ranked

1. **Make the child round do less damage, and measure it every batch.** `selaudit.py`'s child table is the
   instrument: today a child is worse than its donor on lab in 63-69% of pairs and on the nest in 57-60%. Arms for the
   Teacher lane, each paired against `elite1`'s control at 32 x 48:
   - `kevents=1` (or 0) with `kpm=0.003`: today every birth takes 4 structural edits.
   - The decisive multilevel test (needs code): mutate the carry **once** when a colony is refounded, then run the
     round with `sel=random` (blind births and culls, which already exists) and the carry drawn at random from the
     living ants. Colony selection then judges variation the ant score has not filtered. If children stop losing the
     nest and lab, the in-colony score was the cause.
2. **One case per goal, with an explicit weight the owner sets.** Today's implicit weights (nest 63-71%) are an
   accident of how many cases were written for each goal. Suggested themes: feeding, nest (one of nest size or
   scale), crater, lab survival. Drop room shape and spoil from the rank (they do not repeat), run the herb box on
   one shared seed per round or average several (its seed sets 62% of its ranking), and give starvation a measure that
   can see it (e.g. starved per hatching, or a short `sel=off` run of the carry, as the lab case already does). Keep
   gates for floors, as `labgate`/`scalegate` do; they work. Drop lexicase for donor choice or widen the donor pool so
   it has something to choose from.
3. **Keep more than one family alive.** Gates at the 25th percentile (~9 donors instead of 4) or a family cap on the
   carry, and report lineage depth (`selaudit.py`) each batch. The toy model says 4 donors is fine for one trait; the
   owner's pool rule and the trade-offs argue for more. An owner call.
4. **More worlds per colony only after 1 and 2.** Two worlds per colony doubled the toy's gain at double the compute,
   and broke even at fixed compute. Same-world replays do not beat fresh worlds here (2.1). A running mean for kept
   colonies is not recommended unless 1 fails.
5. **Fix the seeds** (2.6) before any run longer than 99 rounds is judged; `elite2` (192 rounds) crosses it.

## 6. The Teacher lane's four questions

1. **Rank-sum for the kept half plus lexicase for donors: right way to combine unequal cases?** No. The rank-sum
   weights goals by how many correlated cases each has (nest 63-71%), and lexicase among ~4 donors adds little; with a
   few contradictory goals lexicase picks corners, not compromises. Use one case per goal with explicit weights, keep
   gates for floors. The instinct arm's starvation rise is better explained by starvation being invisible in that arm
   (repeatability 0.02) than by the sum trading it away (2.5).
2. **How noisy is one world, and would more worlds or a parent-vs-child replay beat more rounds?** Very: the survival
   order repeats 0.05-0.09, and children take 46-49% of the kept half. More worlds help only proportionally to compute
   in the toy; a same-world replay is no better than a fresh world for bed cases (the world explains 0.0-0.2 of
   rankings), but the herb box needs a shared seed. The child round's damage is the bigger lever.
3. **Do the ant score weights pull against the colony cases?** Yes, measurably: one round of mutation plus pay-ranked
   breeding makes a child worse than its donor on nest (57-60% of pairs), lab (63-69%) and nest scale, and better on
   food home in two of three arms; food home and nest trade off at -0.40 between colonies; and pay-ranked breeding
   collapses ~24 lines to 2-3 per round.
4. **Seed leak at round 99.** Confirmed, and there are two more overlaps in the lab seeds (2.6).

## 7. Method notes and caveats

- Retest needs `elite=1`: in `self1` the "kept" colonies mutate, so their retest measures a colony after one more
  mutating round (printed in `audit.txt` but not used above).
- Kept-twice colonies are a restricted sample (all were in the top half twice), which lowers correlations; it is
  exactly the set survival chooses within. The training-versus-garden correlations (all colonies) are of similar size.
- Child-donor pairs share donors and rounds; the sign-test p values are optimistic, the effect directions are
  consistent across three arms and absent in the control.
- Lineage counts use the `family` chain the carry builds (one ant id appended per round); with no crossover it is the
  exact genealogy.
- Survivor bias: extinct colonies drop out of every per-round statistic here; 0-2 per round in these arms.

## References

- Jin, Y. & Branke, J. (2005). Evolutionary optimization in uncertain environments: a survey. *IEEE Trans. Evol.
  Comput.* 9(3), 303-317.
- La Cava, W., Helmuth, T., Spector, L. & Moore, J. H. (2018). A probabilistic and multi-objective analysis of lexicase
  selection and epsilon-lexicase selection. *Evolutionary Computation*. arXiv:1709.05394.
- Muir, W. M. (1996). Group selection for adaptation to multiple-hen cages: selection program and direct responses.
  *Poultry Science* 75, 447-458.
- Shahbandegan, S. & Dolson, E. (2024). On the robustness of lexicase selection to contradictory objectives. GECCO
  2024 companion. arXiv:2403.06805.
- Waibel, M., Keller, L. & Floreano, D. (2009). Genetic team composition and level of selection in the evolution of
  cooperation. *IEEE Trans. Evol. Comput.* 13(3), 648-660.
