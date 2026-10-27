# In-run selection: where it stands (teacher lane, 2026-10-21 19:30Z)

Design doc (owner-reviewed): https://claude.ai/artifact/DT1eU7tWsJTyaUCLk67pSV

## What it is
One colony of 24 mind ants (one brain for every ant, owner rule) on the lab training bed. Food is plentiful, in heaps
25 columns either side of the nest. Dig is on. Old age is off, egg-laying is off, and ants can starve. Selection runs
*inside* the colony: each ant is scored, the weakest are culled and the strongest breed mutated children. A
random-parent arm with the same counts is the control. Every arm is judged by putting the evolved brains into fresh
colonies (24 ants, selection off, 200k frames, 4 fresh worlds) and counting survivors and work done.

## The score (changes often; the owner expects it to keep changing)
- Per-ant ledger. Credit per event is multiplied by a *need price*: p = 1 + 2 * shortfall of that stock.
  - Delivery: worth 7, and up to 3x more by depth below the ground line.
  - Spoil carried out: worth 1, and nothing if dropped within 15 columns of the nest.
  - Dig: half on the cut, half when that cell leaves the nest (so a digger + carrier pair both earn).
  - Meal at home after work: worth 2, paid once per stretch of work (a trip, a dig or a carry).
  - Each bite eaten at the pile: -0.05.
- Score = credit per 1,000 frames alive.
- Judged as z = (score - adults' median) / standard error, the error taken from the ant's own sum of squared credits
  and floored at the colony's average rate.
- Minimum age is 8k frames.

## Selection rules (owner-agreed)
- **Target turnover:** each adult is replaced about once per 30k frames. The bar is a rank, not a fixed z.
- **Clear failures go at once:** an ant with z < -3 is culled regardless of the quota.
- **Breeding:** no ant below the median breeds. The best get up to 3 kids.
- **Parents stay**, but each child is earned on the record since the last one. A lifetime cap of 4 kids.
- **Headcount:** a band of 24 to 36. A family cap of 1/4 of the colony.
- **Rounds:** 200k frames each, on a fresh world each round, carrying the living ants forward.

## Results so far
- **The score sees the brain.** 24 mutants x 8 colonies: the split-half rank correlation of a genome's score is
  0.76-0.84, and a single ant's score is about half brain.
- **Score beats random parents.** Fresh-colony survivors of 24 (latest, nest-purpose score):
  - Survivors: score 22 vs random 16. Worst population 20.5 vs 10.5.
  - Copy v2 clones: 3. The starting mutant mix: 11.5.
- **The nest now has a use.** Time at home rose to 34% (vs 29%), meals at home after work rose 16%, and spoil carried
  out rose 40%.
- **Unsolved:**
  - Food brought home does not rise under selection (152 vs 196 per colony per 200k).
  - Food carried *into* the nest (3+ rows down) almost never happens (about 8 per colony).
  - Ants live at the food pile 50-65% of the time.
- **Slow evolution:** only 2-4 generations per 600k frames before target turnover. Turnover was added because
  selection froze once a colony was healthy.
- **A bug was fixed:** the cull killed by position and ants overlap, so up to 16 of 40 ants went unjudged. Results
  before 18:45Z are void.

## Open questions we would like literature on
1. Steady-state / in-run (rtNEAT-style) selection with **noisy, social, credit-shared fitness**:
   - how to set the replacement rate;
   - evidence thresholds before culling;
   - elitism (parents stay) vs replacement.
2. **Credit assignment in a colony.** Paying chains (digger + carrier) and need-priced job weights, versus group or
   kin selection, or colony-level fitness. When does individual selection on a shared-store colony evolve
   provisioning, and when does it evolve freeloading?
3. **Why provisioning (bringing food home) does not evolve when an individual can eat at the source.** Real-ant /
   social-insect theory (trophallaxis, crop sharing, central-place foraging), and evolutionary-robotics colony
   foraging results.
4. **Mutation rates for in-run selection versus generational selection.** The owner wants these tested; the trainer
   uses pm .03, sigma 1.
5. **Islands / colony competition** (replacing a weak colony with a copy of a strong one): how often, and whether it
   beats individual selection at equal compute.
6. **Whether a missing skill should be taught before selection rather than evolved.** Specifically the walk home while
   carrying food, which copy v2 was never taught. This builds on the earlier Q3.
