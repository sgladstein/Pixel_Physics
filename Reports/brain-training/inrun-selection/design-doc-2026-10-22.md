# In-run selection for the ant brain

Oct 21, 2026 · the owner

## Verdict

**Yes, there is real promise, and it fits the plan we already have.** It is the food-and-dig world we agreed to build, with selection running inside the run instead of between runs. This style of selection has worked before: rtNEAT trained the soldiers in the game NERO this way, and the "embodied evolution" robot-swarm work is the same idea.

A naive version will fail in four predictable ways: luck gets rewarded, free riders win, one job takes over, and the population becomes clones. Each has a known fix, and the design below folds all of them in. I recommend a small local pilot against a random-parent control before any big run.

## Where it stands, 22 October (night)

The founder is now **c1**, the foraging lane's copy of the old ant's carry-home rules into h1. Since the owner's ruling of 22 October, this lane owns the one ant. The other lanes (foraging, dig, maze, playtest) hand over lessons, teachers and score rules to stack onto the current founder, not finished ants of their own.

After the adversarial review, numbers are read like this:

- **Food home** is food put down within 8 columns of the founding hole. That is the foraging lane's count: the old ant puts 5,865 of its 5,899 there. The old count booked food put down beside any dug cell, including cuts out at the food.
- **Every arm is paired with a control from the same batch, seed by seed.** Food is compared per colony-round over the rounds both colonies reached, with a sign test and a bootstrap interval. Colonies lost are compared with McNemar's exact test. Gains and losses face the same noise bar.
- **Survival is also judged with births off**: one round with selection off, where nothing replaces the dead. With births on, about 220 newborns per colony-round hide how many starve.

| Arm against its control (64 seeds, 6 rounds) | Food home per colony-round | Seeds where arm is higher | Colonies lost | Verdict |
| --- | --- | --- | --- | --- |
| Random parents against score selection (h1) | 72 vs 785 | 16 of 63, p < 0.001 | 32 vs 20 (p = 0.06) | Selection works |
| c1 founder against h1 | 2,319 vs 1,431 | 46 of 64, p = 0.001 | 9 vs 17 (p = 0.13) | Adopted |
| Newborns start with 3 tanks, not 1 | 1,078 vs 694 | 37 of 64 (interval +11 to +784) | 17 vs 20 (p = 0.70) | Adopted |
| Appetite gate, bar 3 tanks (h1) | 1,812 vs 598 | 47 of 64, p < 0.001 | 30 vs 20 (p = 0.10) | Reopened: testing on c1 |
| Appetite gate, bar 5.5 tanks (h1) | 2,016 vs 654 | 40 of 63, p = 0.04 | 32 vs 20 (p = 0.03) | Reopened |
| Score holes closed (strict) | 772 vs 1,243 | 26 of 62, p = 0.25 | 22 vs 17 (p = 0.44) | Left off |
| Dig pays only within 10 columns | 1,157 vs 1,213 | 36 of 64 | 16 vs 17 | No effect |
| Nest zone (owner's nest rule) | 840 vs 1,251 | 24 of 64, p = 0.06 | 16 vs 17 | Not adopted: crater unchanged |
| Nest zone, top 3 rows unpaid | 604 vs 1,334 | 17 of 64, p < 0.001 | 14 vs 17 | Not adopted |
| Nest zone with c1 | 1,157 vs 2,416 | 15 of 64, p < 0.001 | 16 vs 9 | Not adopted |
| Maze trail graft on c1 | 630 vs 1,166 | 8 of 64, p < 0.001 | 49 vs 9 (p < 0.001) | Not adopted |
| Deep cuts pay double (h1) | 474 vs 482 | 21 of 62 | 39 vs 20 (p = 0.002) | Not adopted |
| Rim cuts pay half (h1) | 764 vs 635 | 30 of 64 | 31 vs 20 (p = 0.07) | Not adopted |
| Old founder v2 against h1 | 397 vs 520 | 31 of 63 | 38 vs 20 (p = 0.003) | h1 better |

Controls differ from row to row because each pair counts only the rounds both colonies reached.

| Selection off, one round, 64 seeds | Food home per colony | Starved, of 24 |
| --- | --- | --- |
| Old ant (built-in rules) | 743 | 0 |
| c1 | 232 | 16.6 |
| h1 | 66 | 10.6 |
| c1 with the maze trail graft | 41 | 23.4 |

What the review changed:

- **Selection does train the colony, but the old headline measured the score.** Counted honestly, h1 colonies go from 69 to 4,448 food home per colony-round between round 1 and round 6. But even c1 loses 17 of 24 ants in a round once births stop. It learned to carry food home, not to eat when hungry. Births are what keep the selected colonies alive.
- **Several earlier calls sat inside the noise.** h1 against h6 as founder had p = 0.22, and most rim rules had p of 0.07 to 0.1. The appetite gate was rejected on an extinction gap of p = 0.10, while it tripled food home. With the owner's ruling that extra starvation is acceptable when it buys a clear fix, it is reopened.
- **The score's holes are real but small.** After round 1, field drops were 2 to 5% of deliveries, and closing every hole changed nothing measurable.
- **The crater.** The nest-zone rule doesn't move it. Near the nest it measures 16 to 19 cells against the control's 15 to 16. The lead is the dig lane's refusal used as a teacher, whose ceiling reached 0 crater near the nest on 8 of 8 seeds.
- **The full event logs are kept** for every cloud batch, in the bucket under runs/NAME/jobs/. The review thought they were not.

Next: copy the old-ant shadow teacher and the crater refusal into c1, then judge the copy with the refusal off to see whether the crater comes back. Also run c1 with the appetite gate. A 60-round run of c1, score against random parents, is in flight.

### Overnight, 22 October

| c1 arm against c1 (64 seeds, 6 rounds) | Food home per colony-round | Seeds where arm is higher | Colonies lost | Starved per round |
| --- | --- | --- | --- | --- |
| Appetite gate, bar 3 tanks | 5,009 vs 2,475 | 47 of 64, p < 0.001 | 18 vs 9 (p = 0.08) | 94 vs 63 |
| Warmth sense off (creature-perf switch) | 2,303 vs 2,087 | 32 of 64, p = 1.0 | 17 vs 9 (p = 0.15) | 85 vs 68 |
| Maze trail graft, gated on carrying | 1,914 vs 2,373 | 23 of 64, p = 0.03 | 13 vs 9 (p = 0.5) | 81 vs 63 |

**60 rounds of selection on c1** (64 seeds; random parents as the control):

- Food home per round rises from 987 in round 1 to about 7,000 by round 20, then holds there. 50 of 64 colonies live through all 60 rounds.
- With random parents, food falls to 17 a round and 26 colonies die out.
- The crater disappears (median pit 75, then 26 by round 6, 5 by round 11, 0 from round 30). But that is because digging stops: from round 6 on, the median colony has dug nothing beyond the 26-cell starting hole. The random-parent colonies converge to the same 26 cells.
- Under today's score, long selection breeds foragers and drops digging altogether. A dig rung has to pay enough to keep digging alive against feeding.

**Why c1 starves with births off** (foraging lane, traced on seeds 3 and 5): its copy learned "holding a pellet, walk down". The carry teacher's trip loads, food in the crop plus a pellet in the jaws, were walked toward an anchor that drifts into the pit. Fed c1 ants spend 40-55% of their decisions holding a pellet deep in the pit, against 12-13% for h1. They jam there and feed each other flat. A recopy that keeps only upward pellet walks (c6) is in training. Copies of the old-ant shadow teacher and the crater refusal into c1 both did worse (133 and 130 of 192 died, against 96), and the refusal barely copied.

## Why it could beat arena selection here

The main reason is that it judges the brain in the world we actually want, not in a solo arena. Four facts from this repo point the same way.

- **The biggest gap needs a world with both tasks.** When the teacher acts on copy v2, walking home empty is worth +6.0 full loads a world, the largest single piece. We left it to selection because teaching it in a dig-only arena leaks into foraging. A shared world with food and a dig scores one brain on both at once, which is your one-brain rule.
- **It wastes less time waiting.** The lab takes 7 to 13 hours a generation at 540 ants, and nothing is selected until the generation ends. Here every birth is a selection step. Whether that buys more progress per CPU-hour is the pilot's question, not a given.
- **It tests the colony, not a lone ant.** Solo arenas never see crowding, trails left by others, or a shared nest. The crowd sense reads 0 in every solo arena today.
- **We have founders that already work.** Copy v2 scores 10.71 full loads a world. Starting from it means selection polishes a working brain instead of searching from scratch.

It also points at your long-term ideal. If an ant's score is what it contributed, priced by what the colony is short of, then ants that switch to the scarce job score higher. That is roles chosen by signals in the world, rather than castes.

## What breaks a naive version

The naive rule (score above a bar spawns children, score below a bar is removed) fails in six ways. The biggest is noise: one ant's score is mostly luck.

| Failure | What happens | Evidence or precedent | Fix (in the design below) |
| --- | --- | --- | --- |
| Luck is rewarded | An ant near the food or a soft wall scores well whatever its brain | Mirrored twin worlds agree only 0.41 to 0.65; one bed's loads vary 2.4 to 3.1 between runs | Score against same-age ants, and pull each score toward its family's average |
| Children are culled at birth | A newborn has scored nothing yet, so it sits under the bar | rtNEAT protects every newborn for a minimum age | Nobody is scored or removed before a minimum age |
| Free riders win | Digging the nest and clearing spoil help everyone, while a delivery is credited to one ant | The standard public-goods problem in group evolution | Credit shared work, and run several colonies that compete as wholes |
| One job takes over | If one job scores more easily, every ant drifts to it and the other stops | Seen whenever two tasks share one score | Price each job by what the colony is short of |
| The colony becomes clones | One strong family fills the world, so there is nothing left to select from | rtNEAT and NEAT both protect variety for this reason | Cap any one family's share, and pick parents by small tournaments |
| The headcount runs away | Births outpace deaths, or the reverse |  | Keep the headcount in a range: it sets how many births and removals, never which ants |

One rule is not a failure mode but a requirement. Every birth, death and mutation must come from the run's seed in a fixed order, so the same run gives the same result.

## The improved design

The core idea is to judge each ant by how sure we are of it, not by its score alone. An ant whose record is clearly bad is removed, fast if it fails hard. An ant whose record is clearly good earns a child each time it clears the bar again. The headcount decides how many births and removals happen; the evidence decides which ants.

&#91;embedded content: in-run selection · one colony's loop, and the step between colonies\]

Each colony loops on its own; the band below it is the slower step where whole colonies compete.

1. **Founders.** Four colonies of copy v2, each ant lightly mutated.
2. **The ledger.** Each ant earns credit for food delivered, nest cells dug and spoil carried out. A digger is credited again when the spoil it cut leaves the nest, so a digger and a carrier working as a pair both score. A job's price rises while the colony is behind on it, so the scarce job pays more. This is how roles can choose themselves.
3. **Minimum age.** An ant is not scored, removed or used as a parent until it has lived a set stretch.
4. **Judged when clear, not on a timer.** Each ant's record is compared with ants of the same age and pulled toward its family's average. A hard failure becomes clear soon; an ordinary ant may never be clear either way, and simply keeps working.
5. **Removal.** An ant whose record is clearly below the bar is removed, the clearest failures first.
6. **Children.** An ant whose record is clearly above the bar earns a child, and another each time it clears the bar by a further margin. A standout ant has several; a decent one has one. No family may hold more than a quarter of the colony. The child is a mutated copy, born at the nest. Luck rewarded here wastes slots rather than taking over, because a lucky parent's children score poorly in turn and are removed. So whether "clearly above" beats plain score is a pilot arm, not a fixed rule.
7. **Headcount.** A controller keeps each colony between a floor and a ceiling. When the colony shrinks it allows more births, given to the clearest successes first; when it grows it removes more, the clearest failures first. It never lowers the bar for a parent, because a crash is when scores mean least.
8. **Colonies compete.** Every M frames the weakest colony is replaced by a mutated copy of the strongest. This is the step that rewards shared work, which no single ant's score can see.

The minimum age, how clear a record must be, the floor and ceiling, and M are for the pilot to set. Starting guesses: a minimum age of 10,000 frames, 30 to 100 ants a colony (the pilot picks), and M = 25,000. The pilot runs 200,000 frames, which gives about twice the judgements of the 100,000 first proposed. Whether that is enough for selection to bite is something the pilot measures.

## The scoring function

An ant's score is what it added to the colony minus what it ate, with each job priced by how short the colony is of it, per thousand frames of life. Most of the counts come from logs the engine already writes: food moves, larva feeding and the decision trace. Tagging each loose spoil cell with the ant that cut it is new harness code.

| Job | Counted when | Colony stock that sets its price |
| --- | --- | --- |
| Food | a food cell reaches the store, or feeds a larva | store food per ant |
| Digging, first half | the ant cuts a cell below the ground as built, inside the nest | open nest space per ant |
| Digging, second half | a cell this ant cut leaves the nest, whoever carries it | open nest space per ant |
| Clearing spoil | the ant carries a loose soil cell out of the nest | loose soil lying in the tunnels |
| Eating (a debit) | the ant eats from the store | store food per ant |

```latex
\text{score}_i = \frac{1}{T_i}\sum_{e \in i} w_{j}\,p_{j}(t_e)\,s_e \qquad p_j(t) = 1 + k\,\max\left(0,\ 1 - \frac{\text{stock}_j(t)}{\text{target}_j}\right)
```

Here e is one of ant i's events, j its job, s = +1 for work and −1 for eating, and T is the ant's thousands of frames alive past the minimum age.

- **Prices are read when the work is done.** A dig made while the nest is cramped counts for more than the same dig in a roomy nest. With k = 2, a job the colony is wholly short of pays three times its base.
- **Exchange rates are measured, not chosen.** Each w is set so that, in a balanced colony of copy v2, a frame spent on any job earns the same. They come from a baseline run before the pilot.
- **Eating is a debit at the food price.** An idle ant scores below zero and is the first out. An ant that only feeds itself scores near zero.
- **A digger is paid in two halves.** The second half arrives only when its spoil leaves the nest, so a digger and a carrier working as a pair both score. A digger whose spoil clogs the tunnel earns half.
- **Later, not now: pay digging through the nest score.** The pilot pays every cut cell the same. Once the score works, the general nest score could pay each cutter by what its cells became. That score wants chambers about 7 tall and 3 times as wide, and tunnels 3 to 6 wide; cracks and pits count against. A cell that was tunnel and becomes part of a chamber pays more later.
- **The colony is scored between colonies, not inside each ant's score.** Adding the colony's result to every ant in it would not change who ranks above whom inside it. It matters only when colonies compete, so colonies are compared on the same ledger summed over their ants.
- **Luck is damped as in the design above.** Each ant is compared with ants of the same age, and pulled toward its family's average.

### Checks the score must pass before any selection runs

These need no selection and run on the 4 local cores. If the score fails one, it is fixed before the pilot starts.

1. **Noise floor.** A colony of one genome: the spread of the clones' scores is how much is luck.
2. **A better brain ranks higher.** Copy v2 ants mixed with s171 ants: copy v2 should score above s171 by more than the noise floor.
3. **An idle ant ranks lowest.** A brain that never moves must score below zero.
4. **Prices respond.** Teacher-driven diggers should earn more in a cramped nest than in a roomy one.
5. **A pair wins.** A teacher-driven digger and carrier should both score well above an idle ant.
6. **The score repeats.** An ant's score over the first half of its life should predict its second half. The correlation is the number to report.

## Pilot

The pilot asks one question: does in-run selection make better brains than picking parents at random, and than our current arena selection, for the same CPU? It runs locally on the 4 cores, with no Google compute.

**What to build.** A harness on the lab's own engine, so the physics stays the same as every other world. The lab already has a nest store, hungry ants eating from it, brood and starvation. One colony per core, with surface food and a nest to dig, 30 to 100 ants, 200,000 frames. The founders are copy v2 with small mutations.

**Arms, on the same seeds:**

1. **In-run selection**, the design above.
2. **Random control.** The same births and removals at the same times, but parents and victims are chosen at random. If arm 1 cannot beat this, selection has no teeth.
3. **Arena selection** at equal CPU, with the existing trainer.

**Positive control first.** Put a few copy v2 ants (10.71 full loads) into a colony of s171 ants (9.47). Arm 1 should spread the copy v2 family and arm 2 should not. If the better family does not spread, the score cannot see a real difference, and the full run is pointless.

**How to judge.** Take the most common brains at the end of each run and score them the usual way: full loads on the 96 evaluation worlds, then the trainer's fetch floor on the worse side. Paired standard errors, clustered by mirrored pair.

**Cost.** I have not timed a colony for 200,000 frames yet; that is the first measurement. Bigger colonies run slower but judge more ants at once, so the pilot times 30, 60 and 100 ants and picks the size that judges the most ants per core-hour. Nothing goes to Google compute until the pilot says yes.

**What would make us drop it:**

- Arm 1 is not better than arm 2 beyond the noise: the score is mostly luck.
- Arm 1 is not better than arm 3 at equal CPU: keep arena selection.
- One family takes the whole colony early despite the cap: fix variety before anything else.

## Pilot findings, 21 October

Selection on the score beats random parents, but colonies keep dying out mid-run, and that now blocks every comparison. Pilots were local: 24 ants per colony, 200,000 frames per round (about 30 seconds), 8 paired seeds per arm. The judge is a fresh colony, with selection off, built from each arm's evolved brains; it counts survivors of 24 after 200,000 frames.

| Finding | Number | What it changed |
| --- | --- | --- |
| The score sees the brain | Split-half rank correlation of a genome's score 0.76 to 0.84; one ant's score is about half brain | Selection on the score is worth doing |
| Selection beats random parents | Fresh-colony survivors: copy v2 clones 3, start mix 11.5, random parents 16 to 18, score 20 to 22 | Keep the random-parent control in every batch |
| Culling by position hit the wrong ant | Up to 16 of 40 ants never judged; runs before 18:45 void | Culls now remove an ant by id |
| A healthy colony stops selecting | 0 to 47 births per 200,000 frames once fed | Target turnover added (below) |
| Mutation grew brains until ants starved | Links 700 to 2,200 in 13 to 21 generations | Mutation now adds and removes links with equal odds |
| Ants farmed the dig score | Cut surface soil 20 to 40 columns out and dropped it by the hole | Dig and clear pay only within 20 columns of the nest |
| Food home rose under selection | 1,492 deliveries per colony against 139 to 157 | The best result so far, from a run before the farming fix |
| Ranking on z marks deliveries down | z and plain score agreed on 44 of 133 parents | Shrunken-mean ranking is an arm (research lane, PR #768) |
| Colonies die out mid-run | 3 to 7 of 8 colonies in every arm; 47 of 108 rounds had a die-off | The open problem below |

## The rules as built

The pilot harness now runs the owner's rulings below; each is a switch, so any can be turned off for a comparison.

**World.** No old age and no egg-laying, so births come only from the score. Ants can starve. Food is plentiful in two heaps 25 columns either side of the nest. Founders start piled at the door.

**Score**, credit per 1,000 frames alive, each job multiplied by a need price:

- Food delivered: worth 7, up to 3 times more by depth in the nest. Later: pay food left in a chamber, not the deepest point. Half is paid as the food moves home and half on arrival, taken back if the carrier eats it on the way (food home +54%, starvations -20% on 8 seeds).
- Digging: half on the cut, half when that cell's spoil leaves the nest, only for cells within 20 columns of the nest.
- Spoil carried out: worth 1, nothing if dropped within 15 columns of the nest.
- A meal at home after work: worth 2.
- Leaving home while hungry (below 2 full tanks): worth 10, at most once per 3,000 frames.
- Food price: set by how full the colony's tanks are, from 1 when everyone is full to 5 when empty.

**Selection.**

- Target turnover: each adult is replaced about once per 30,000 frames. The bar is a rank, not a fixed score.
- An ant far below the median (z below -3) is culled at once.
- No ant below the median breeds; the best get up to 3 children at a time.
- Parents stay, but each child is earned on the record since the last one; after 4 children a parent retires.
- A colony below 24 ants refills from the best of the living and the recently starved, a dead ant's score halved.
- No family holds more than a quarter of the colony while other families can breed.

**Rounds.** Each 200,000-frame round runs on a fresh world, carrying the top 30 ants forward.

**The score will never be settled.** The gate for long runs is instead: weights read from a file at round boundaries, every change logged, and the quick checks rerun after each change. Later, a metric that stops improving gains weight, capped at 4 times its base, so a lucky rare event cannot dominate.

## The open problem: colonies die together

Foragers were not culled. They filled up, stopped going out, and starved together when the shared food ran down. All 29 die-offs in the latest batch were traced ant by ant; in the early ones every ant that had delivered food was still alive when the colony died.

&#91;embedded content: in-run pilot p12, arm A seed 1, round 0 · ants\_f and events logs, 0 to 32k frames\]

Food is shared at home, so every tank drains in lockstep, and none of the ants leaves for food until it is too late. Because everyone dies at once, survival cannot pick out the ants that would have gone out. Retirement removed some foragers in later die-offs (108 of 295), but the arm without retirement dies at the same times.

**Tried so far**, each on 8 paired colonies:

- Refilling from the best dead ants keeps some colonies going, but it becomes a treadmill: about 230 newborns starved in one colony between 30k and 90k frames.
- Pricing food by tank level and paying hungry departures each helped a little: starvations 1,872 down to 1,578, food delivered 609 up to 971, colonies dead or nearly 5 of 8 down to 4 of 8.

Selection can only amplify a habit some ants already show, and almost none leave when hungry, about 30 hungry departures per colony in 200,000 frames. A separate thread is teaching the habit (leave when your tank drops) so the colony starts with it.

## Cloud batches, 21 to 22 October

*Read with care. This section and the next use the old food count, which booked food put down beside any dug cell. They also pool food over runs of unequal length and call arms on extinction gaps without a test. The paired recount is in "Where it stands, 22 October (night)" near the top.*

The first 64-seed batches ran on a 48-core cloud machine. Each colony starts from the lesson thread's "go out when hungry" founder (h1) plus 30 mutants, and runs 6 rounds of 200,000 frames. Each round is a fresh world founded on the last round's 30 best ants. A batch of 256 to 320 colonies takes about 20 minutes.

**Selection now trains the colony.** With random parents the colony never improves. With score selection the survivors bring in about 20 times more food by round 6.

| Arm (h1 founders, 64 seeds) | Food per round, round 1 → 6 | Extinct of 64 |
| --- | --- | --- |
| Random parents (control) | 80 → 136 | 32 |
| Score, turnover 15k | 122 → 5,607 | 35 |
| Score, turnover 30k | 137 → 3,047 | 21 |
| Score, turnover 60k | 118 → 3,047 | 29 |
| Score, 30k, no pay for rim cuts (`rim_rows=3`) | 121 → 4,261 | 34 |
| Score, 30k, rim cuts charged (`rim_debit=0.5`) | 123 → 3,719 | 35 |
| Score, 30k, h6 founders | 113 → 2,835 | 28 |

Food per round is the mean over colonies still alive in that round, so later rounds count fewer colonies. Compare the arms by total food per seed and by extinctions.

- **Turnover 30k is the default.** Faster turnover trains harder, but most of its extra extinctions come in round 1, before selection has had time to work. This is the owner's worry about ants that never need to eat.
- **No pay for rim cuts costs survival.** The dig thread's crater fix took extinctions from 20 to 34, nearly all starvation die-offs in round 1. Total food per seed was unchanged (6,936 against 6,906). In round 1, ants that cut mostly at the rim delivered more food than ants that cut mostly deep in 9 of 12 seeds. So the likely mechanism is that withdrawing rim pay withdraws pay from the surface-goers. This is not yet traced through to who was culled.
- **h1 is the better founder.** h6 answers hunger better in the probe (7 starved against 43 of about 190) but loses more colonies (28 against 20).
- **Nest scores from these batches are not usable.** The nest score read every crater's pit as 0 in this box, because its sky line started at the lid over the wall columns. The dig thread is fixing it, and the nest numbers will be rerun.
- **Determinism fixed.** 2 of 64 seeds differed between the two batches, because dropped pellets were paid in hash-map order. They are now sorted, and both seeds repeat exactly.

### Softer rim rules, with the fixed nest score

Two more batches (same founders, seeds and settings) tried the dig thread's softer shapes and reran the control with the pit fix. Nest score, pit and door are compared only on seeds where both the arm and the control survived all 6 rounds.

| Arm | Extinct of 64 | Food per seed | Nest score, control → arm (paired) | Open pit cells (paired) |
| --- | --- | --- | --- | --- |
| Control | 20 | 6,923 |  |  |
| Rim cuts pay nothing | 34 | 6,905 | -56 → -35 (21 seeds) | 48 → 32 |
| Rim cuts pay half | 30 | 7,087 | -52 → -33 (24 seeds) | 45 → 30 |
| Deep cuts pay double | 39 | 4,180 | -58 → -46 (17 seeds) | 50 → 36 |
| Deep double, rim half | 33 | 7,343 | -62 → -39 (21 seeds) | 53 → 36 |
| Old founder v2 (no hunger lesson) | 38 | 3,510 |  |  |
| Random parents | 32 | 488 |  |  |

Every change to the dig rule shrinks the crater by about a third and lifts the nest score by about 20 points, but every one also costs 10 to 19 more colonies of 64. Even paying deep cuts double, with rim cuts still at full pay, costs survival. So the cost is not only that rim cutters are surface-goers losing pay: any shift of the score toward digging seems to pull selection away from feeding. All nest scores are still negative, and every surviving colony still has a crater of about 30 cells. The hunger lesson is worth more than any of these: the old founder v2 lost 38 colonies against h1's 20.

A batch-runner bug was found and fixed on the way: a jobs line ending in a blank was joined to the next line, so one arm ran twice and another not at all. That arm's results were relabelled, and it is the "pay nothing" row above.

## Where the food goes, 22 October

The owner asked whether food ever reaches the nest. Traced cell by cell in the in-run bed, it mostly does not: of the food the h1 ant bites in the field, **1 to 3% is delivered** (seed 1: 2,378 bitten, 68 delivered; seed 3: 4,290 bitten, 108 delivered). The colony digests 2 to 2.5 times what it burns, and 89 to 96% of what it digests goes into ants already above 5.5 tanks. So the foragers grow very fat, the store stays near empty, and the ants at home run down together and starve in bursts.

**It is the brain, not the gut.** The foraging-loop thread ran the game's own hand-built ant (the "old ant", no trained mind) in the same bed. The old ant delivers 25% of what it bites, eats none of it by choice, and none starve. The h1 ant swallows 38% of its bites by choice (the Eat verb, mostly at 20+ tanks) and puts 41% back down in the field, no nearer home. A cell takes about 1,500 frames to digest and a walk home is a few hundred, so digestion on the way is not the bottleneck.

| Same bed, 64 seeds, one round | Food delivered per colony | Starved per colony | Nest score |
| --- | --- | --- | --- |
| Old ant (built-in rules) | 747 | 0 | +147 |
| h1, no selection | 80 | 11 | -124 |
| h1 with selection, round 1 | 137 | 49 |  |

**What was tried on the body, and why it is not the fix.** Digest less when full (the engine's appetite gate, scratch-enabled here because the bed's 1e30 breeding bar left it inert) and a hard bank cap each lifted food delivered about 2.5x and the nest score by about 25, but more colonies died out (30 to 33 of 64 against 20). Every ant hovers near 3 tanks with no reserve, so a dip in foraging kills them together. Gating the Eat verb only turns eaten food into food dropped in the field.

**Trails.** Trail A sits at its maximum wherever ants walk, so it carries no direction; trail B marks the path. Blinding the mind to both halves the food bitten in the field and doubles deliveries (8 seeds), so the trails pull ants out but do not bring food home. A separate thread is testing trail following in a maze.

**Changes adopted.** Ants born after the founders now start with 3 tanks instead of 1. Over 64 seeds and 6 rounds, food delivered rose from 6,923 to 11,076 per seed. Extinctions went from 20 to 17, which is noise: paired by seed, 15 colonies survived only with 3 tanks and 12 only without.

**Next.** The foraging-loop thread copies the old ant's carry-home rules into the mind as a lesson. Acting as a teacher on h1, those rules raised deliveries over 8 seeds from 638 to 4,901 cells and cut starvation from 98 to 55 of 192. With the pack move added, starvation fell to 25. The lesson's founders will be judged here at 64x6 against h1, with the old ant as the reference arm.

**Stored food gets buried.** By the end of a round, 60% of the control's stored food (25% with the appetite gate) sits in piles walled off from open ground. The cause is not settled; near a pile, ants mostly re-cut loose soil rather than drop it.

## Before long VM runs

Long runs are worth paying for once a colony survives its own success and the comparisons can be read. In order:

*Status, 22 October (night).* The owner has given the go for long runs on all 64 cores, and a 60-round run is in flight. Against the list below:

- Colonies survive with births on (c1 loses 9 of 64 over 6 rounds), but not with births off.
- The score checks have not been rerun on the score as built.
- The fresh-colony judge and clone colonies have not been run.
- The arena comparison at equal CPU is set aside.

Added since the review: a control in every batch with its build recorded, survival judged with births off, and a census of where food home is put down.

1. **Colonies survive.** Most of 8 seeds get through 3 rounds without dying out, with founders from the "go out when hungry" lesson (its own thread) or a score that grows the habit.
2. **The clean comparison reads.** Today's setup against no home meal, no retirement, shrunken-mean ranking, and a 30 times cheaper brain link, each with a random-parent control.
3. **The judge matches the one-brain rule.** Add clone colonies of each arm's best genome beside the mixed fresh colony.
4. **The checks are calibrated.** The z below -3 cull is checked on a colony of clones, where every difference is luck. Minimum age and turnover are set from how fast a record becomes trustworthy, read from the logs.
5. **Mutation rate tested** for in-run selection, which may differ from the arena trainer's.
6. **The weights file and change log** are built, so the score can keep changing during long runs.
7. **Findings written up and adversarially reviewed**, then the harness moved from scratch into real code through the merge desk.
8. **A short VM pilot** with the owner's go, then long runs.

## Who builds what, and open questions

I can build the pilot harness and its shared world in my scratch tree, since it doubles as the food-and-dig world we already planned. Anything that lands in the trainer goes through the brain lane, which owns that code, as a design review first.

Three questions for you:

- [ ] Settled: births come only from the score. The body's own egg-laying is off in the pilot, because it rewards eating and would breed foragers.
- [x] Settled: ants can starve. Food stays plentiful in the first tests; an ant starves because foraging failed, not because the world ran out. Ants eat from the colony's store, and an ant starves if the store runs dry or it never comes home.
- [ ] Should the founding colony include a few teacher-driven "pioneer" ants that fade out over the run? They would show the walk home without copying it into every brain. My default is no for the pilot, so the control stays clean.
- [ ] Can I start the pilot locally now, alongside the brain lane's copy-versus-selection run?
