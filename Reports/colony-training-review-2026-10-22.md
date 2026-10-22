# In-colony training review, 2026-10-22

**Status: review delivered 2026-10-22; the harness switches it tested live as a patch, not in the tree.** Written by the
Fable review lane for the owner, same day the in-run selection harness (`examples/inrun.rs` on the Teacher lane's compute
branch `claude/project-thread-8y5ak0`, head 65ab126b; not on `main`) was a day old. The interactive version is the
artifact at https://claude.ai/artifact/AYMLfoEd4jvMiJK2g2RgG9 ; this file is its text. Everything the review built sits
beside this file in `colony-training-review-2026-10-22/`: the four-switch harness patch
(`inrun-switches-vs-65ab126b.patch`: `grow_band=`, `cross=`, `pick=lexicase`, `lesson=`, all off by default and
byte-identical to the unpatched harness when off), the hand-off notes for the Teacher lane with exact job lines, the
raw judge rows of both 12-seed tests, and the scripts that ran them. The same files were left in the project folder
`/mnt/project-files/colony-training-review/` for the Teacher lane, which owns the compute branch and the next step
(the protected-lesson switch at 64 seeds).

Review of the Teacher lane's in-run colony selection and the lesson lanes feeding it (21 to 22 October). Sources: the design doc, the research lane's method report and field map, the lane memories, the Teacher, foraging, dig and maze threads, the harness source on the compute branch, and one local test run for this review.

**Yes, keep it. It is the first training method in this project that has beaten the hand-built ant on the owner's own terms,** and it did so in a day. Under six rounds of selection the hA1 line now carries home about five times the old ant's food in the same worlds.

**But the loop as built cannot reach the owner's stated goal** (one brain that keeps digging a large multi-chamber nest and keeps foraging all game, with nobody starving). Three things are structural, not tuning: the training world never contains the late game it is meant to produce; every child carries a heavy mutation and most generations are bred where digging is not selected, so unselected skills erode; and the lane spent most of its runs on score weights, which moved almost nothing, while the three changes that moved the ant were all lessons or body changes.

The fixes below need no new physics. *Tested within the day (sections 4 and 5):* the long static world failed at 64 seeds, and crossover wrecked digging; the change that showed is mutation aimed away from the lessons (protected lesson slots), which halved starvation and kept digging at the price of some food, and now goes to 64 seeds. The report below keeps the original ranking with each item's test result marked in place.

## 1. What was built, and what it produced

The loop: 24 to 36 ants in a sealed bed with two food heaps 25 columns out, no egg laying and no old age, so births come only from the score. Each ant earns credit per thousand frames (food home, cuts, spoil out, meals after work, leaving home hungry), priced by what the colony is short of. About once per 30k frames an adult is replaced by a mutated child of an above-median ant; clear failures are culled at once. Every 200k frames the world is reset and the top 30 ants by score found a fresh one. A 64-seed, 6-round batch takes about 20 minutes on 48 cores.

Skills are not expected to arise by mutation. They are written in first, by copying a hand-rule teacher into the brain from logs of the ant's own states (DAgger), and selection polishes the copy. That ladder is the real product of the day:

s171 → v2Arena champion; a copied "dig at the foot" lesson.Dig score 9.47 → 10.71 on 96 worlds. In a colony, clones of v2 kept 3 of 24 ants alive.
h1"Go out when hungry" lesson copied into v2.Clone survivors 8.8 of 24. Home food about 60 per colony-round unselected; 1 to 3% of bitten food reaches the nest.
c1The old ant's carry-home rules copied in.Home food 253 vs 57 per colony (32 seeds), but collapse rose 14 → 26 of 32: food left the ants' bodies for a store the hungry could not reach.
hA1 (founder)"Hungry at home: put the pellet down, eat from the store", labelled on c1's own states.Home 269, died 4.3 of 24, collapsed 2 of 32. The one lesson that fixed a failure without opening another.
hA1 + 6 roundsSelection on the score, then judged in fresh worlds with selection off.Home ~3,900 vs old ant 729 vs unselected hA1 256 (16 seeds). Starved 4.7 of 24 vs old ant 0. Cuts 131 vs old ant 305.

Everything in the lane's stated goal that is better than the old ant came from this ladder plus selection. Everything that is still worse than the old ant (starvation, digging) is where no lesson exists yet.

## 2. Is it a good approach?

**Strengths that should be kept exactly as they are.**

- **Selection in the colony, judged in the colony.** The arena ladder spent weeks and never produced an ant that brought food home in a crowd. This did in a day, because the fitness is measured where the behaviour is wanted.

- **Lesson first, selection second.** The research lane's point that selection cannot amplify a skill nobody shows was right, and the day proved it: all three adopted founders were copies, and selection then multiplied the copied skill about fifteen-fold.

- **The honest judge.** Since midday the lane judges final ants in fresh fixed worlds with selection off, paired by seed, against the old ant. That is the right instrument. Before it existed, in-run numbers overstated progress several times (food "delivered" beside field cuts, free-tank newborns propping survival).

- **Determinism, per-batch controls, paired tests.** Fixed after the adversarial review and now routine.

**Where the approach, as built, is working against the goal.**

### 2a. The training world never contains the late game

Every round starts a fresh bed: a 26-cell stamp of a nest, 30 ants piled at the door with three tanks, heaps full. The population is therefore only ever selected in the opening 200k frames of a colony's life. The owner's goal is the opposite phase: a colony that already has a large nest and keeps extending it while foraging. No ant in the bed has ever been selected in that state. The dig lane's trace shows the consequence: half of all new-ground cuts fall in the first quarter of a round while births are spread evenly, so about three quarters of the generations are bred where digging is neither possible nor paid. Shorter rounds were tried and refuted; they shorten the opening phase, they do not add the late one.

The headcount band (24 to 36) compounds this. A real colony needs more room because it grows; here it never grows, so once 25 open cells per ant exist the dig price falls to its floor and spoil bailing (the crater treadmill) is the only dig work left.

### 2b. Heavy mutation on a weakly selected population erodes whatever is not being paid

Each child takes a weight step of sd 1 on about 3% of some 700 live links and 10 structural add-or-remove events. Selection per birth is weak by design (above the median, graded). Ten generations per round. The dig lane found digging bred out in the random-parent arm as well as the score arm, which food drag cannot explain. The simplest explanation is mutational erosion: any skill not under current selection decays within about ten generations. That is also the mechanism that will erode the copied lessons in a long run, and it means every lesson has to be re-copied after enough selection. The mutation-rate sweep the owner asked for on 21 October was never run. A local test for this review separates the two causes; see section 5.

### 2c. The score is a hand-weighted scalar chasing a multi-objective goal

The ledger has about fifteen weights and prices. Over the day, roughly a dozen 64-seed arms varied them (strict score, rim rules, deep bonus, dig weight x2 and x4, nest zone, zone top rows, starved-never-parent, 3x hungry pay, warmth off, round length) and none was adopted; several cost food or colonies. Meanwhile the three adopted changes to the ant were lessons, and the two body changes tested (3 starting tanks, the appetite gate) each moved food by 60% to 400%. The score weights are the least productive lever in the whole programme, and the plan is to add more of them (adaptive weights). Rank-based parent selection over several outcomes (section 4) gets the "pay what the colony lacks" effect without any weights.

### 2d. The product is one genome today; the judge measures a mixture of thirty

The lab game ships a single brain (`assets/minds/h1.raw.txt`). The common garden plants the last round's 30 genomes together. The design doc's own early finding was that a clone colony of one genome does far worse than a mix (3 vs 11.5 survivors). **The owner's answer (22 October, 14:01Z): the goal does not have to be a single brain; save several.** That resolves the gap in the right direction: ship the population, not a champion. "One brain" then means no castes and no hand-made roles, with the differences between ants being small weight variations that selection keeps. Three things follow: the game's births should draw from the saved pool (today `mind::inherit` copies the parent's mind unchanged, so a shipped pool is the only source of variety); the judge is the mixture, as the common garden already does, plus a check that the mixture persists (how many lines remain at round 20); and keeping diversity in training becomes a goal in itself, which is where the family cap, recombination and lexicase (section 4) pull the same way.

### 2e. The bed and the game differ in ways selection will exploit

No eggs, no old age, no rain, food at a fixed 25 columns in heaps, a stamped door. In the game the appetite gate interacts with the breeding bar, children inherit the mind unchanged, and food comes from plants. The lane's own card already says "check the lab game first" for the gate. That should be the rule for every founder change, not only this one.

## 3. Inefficiencies in the process
````

| Where the day's effort went | What it bought | What to do instead |
|---|---|---|
| About twelve 64x6 arms on score weights and geometry rules | Zero adopted; two cost survival, one collapsed digging | Freeze the ledger. Spend arms on lessons, body genes, world structure and mutation. |
| Calls made at 8 seeds, later reversed at 32 or 64 (short rounds; copies s4 and d1; a link-bill arm) | Three detours, each a batch and a write-up | No arm is reported below 32 seeds; 8 seeds is a smoke test only. |
| Extinction counts as the headline metric | Near coin-flip (most gaps p 0.1 to 0.4); several early calls rested on it | Report medians of home food, starved and new-ground cuts from the common garden; extinction only as a paired count. |
| Means of home food per arm | Dominated by outliers (158 to 10,109 per colony on one arm) | Medians and paired sign tests, as the lane now does; put the median first. |
| In-run numbers used as the judge until midday on day two | "Delivered" counted food beside field cuts; survival counted 220 free-tank newborns a round | Already fixed. Keep the common garden as the only judge. |
| Full event logs for 64 jobs x 60 rounds; 19 GB pulled locally | A 60 GB disk filled, every later job read as extinct; a second fill locally | Fixed (gzip past round 10, stream from the bucket). Also write the per-round summary row on the VM, so the pull is a few KB. |
| Thread-message cap hit at noon; replies went into files | Lanes waited on each other | Files as the default channel between lanes, with one line in the thread pointing to the file. |
| The research lane's zero-cost tests (reliability by window, child minus parent at matched age, mutation sweep) left unrun | The one number that sets minimum age and turnover is still unmeasured; the owner's mutation question unanswered | age30.csv and ants_f*.txt already hold the data; one Python pass over a pulled batch. |
| Three lanes building teachers and copies in parallel, one copier at a time (11 GB), the Teacher lane the only judge | Queueing on the VM and on the Teacher lane | Give each lesson lane the common garden script and the 64-seed job line, so it hands over a judged genome, not a request. |

## 4. How to improve: ranked, with what each decides

1. **Put the late game into the training world.** *Tested since this was written:* the Teacher lane ran (a), one continuous 1.2M-frame world with no reset, at 64 seeds, and it was worse on everything, with digging stopping in most colonies. That is the expected failure of a static long world: once the nest is finished there is nothing left to cut for a million frames, so the late game it adds is an idle one. The late game that matters is *demand*: more mouths and more rooms wanted as the colony grows. So (a) is dead, and the live forms of this item are item 2 and (b), rounds that inherit the previous bed while the headcount grows. Judge in the common garden and additionally in a judge world that starts with a pre-dug large nest, counting new rooms (the nest score's chambers) rather than raw cuts. rounds.sh

2. **Let the colony grow.** Raise the headcount band over the round (24 → 72, say, by 20k-frame steps). The dig price then stays off its floor, crowding is real, and births from the score have somewhere to go. This is the closest thing to a real colony's demand for rooms without inventing a sink. Now a switch in the harness (`grow_band=F:HI`, in the patch handed to the Teacher lane) and under test in section 5. Decides whether demand alone keeps digging alive.

3. **Cut the mutation and protect the lessons.** Measure first: the child-minus-parent score at matched age is already logged (`age30.csv`). Then arms at pm 0.01 / sd 0.3 / 2 structural events, and a per-slot rate map that mutates the units a lesson wrote at a tenth of the rate. Decides how much of the digging loss, and of any future lesson loss, is erosion rather than selection. The local test in section 5 is the first half of this.
*Tested since this was written*, by the Teacher lane at 64 seeds, judged in fresh worlds: whole-genome low mutation gave 244 new-ground cuts against 130, starvation 3.0 against 4.9 of 24 (p = 0.007), the same food paired by seed, but 20 of 64 colonies died out in training against 8. No mutation at all gave 511 food against 3,467. So the trade is real at both ends: the rate that finds the food gain is the rate that damages the lessons, and the rate that keeps the lessons lets clone colonies jam and die. A single flat rate cannot sit in the right place for both jobs, which is the owner's question about different rates for different processes, answered below.
**Different rates for different jobs.** A teacher pass writes a known skill, so during a lesson the only variation should be the teacher's corrections, with mutation off. Colony selection explores and needs mutation, but not everywhere at once. The standard tools, cheapest first: *per-slot rates* (slots a lesson wrote mutate slowly, no structural events there; built this session as `lesson=PATH:F` and under test in section 5; note that hA1 differs from c1 on 1,081 of 2,790 slots, so at F = 0.1 this is also a large cut in the overall rate, not a surgical one); *self-adaptation* as in evolution strategies (each genome carries its own rate as one extra inherited, mutated gene, so lines whose rate is too high breed broken children and die out, and the pool settles on a spread of rates, which fits the save-many-brains ruling); *fitness-ranked rates* (children of top-ranked parents get small steps, children of marginal parents large ones; fits the rank-based parent pick already in the harness); the *one-fifth rule* (raise the rate when more than about a fifth of children beat the parent, lower it otherwise) as a readout only, since the lane's measured 68 percent of children below the parent reads as "about right" under it and it would not have caught the lesson damage; and covariance adaptation (CMA-ES), which learns which slots to vary together but is too heavy for a 2,790-slot genome and a 24-ant colony, listed so nobody rebuilds it. Recommended order: protected lesson slots now, the self-adaptive rate gene next, the one-fifth rule on the dashboard.

4. **Add recombination.** The brain's topology was caged in a fixed scaffold precisely so genomes stay slot-aligned and can be crossed (dead-ends, creature-direction D4). The harness never crosses. Half of births as a uniform crossover of two above-median parents (per scorer unit, so a unit's wiring travels whole) could combine a digging line with a foraging line instead of waiting for one line to find both. Decides whether a colony can hold both skills at once. Now a switch (`cross=P`, whole scorer-unit groups swap between two eligible parents before mutation) and under test in section 5.

5. **Replace weight tuning with lexicase or Pareto parent selection.** Each birth: shuffle the outcome metrics (home food, new-ground cuts in the nest, spoil out, meals after work, hungry departures, lifetime survival), keep the ants best on the first, break ties on the next, and so on. Specialists on any one job survive and the colony as a whole covers all jobs; no weight is ever set. This is what adaptive weights are trying to approximate by hand. Keep the z-based hard cull for clear failures. Decides whether a one-brain population can keep digging and foraging under selection without a dig weight. The research lane listed lexicase as "not before a rung stalls"; the dig rung has stalled. Now a switch (`pick=lexicase`, over five per-ant rates: home deliveries, new-ground cuts, spoil carried out, paid home meals, hungry departures) and under test in section 5.

6. **Judge one genome, in the game.** For every founder change: the common garden (32 seeds) as now, a clone colony of the most common final genome, and eight seeds of the lab game. Decides whether the result survives being one ant in the real world. Retire extinction as a decision metric.

7. **Body genes, as the new thread is doing.** Open the gut first (digest rate, crop capacity, appetite gate, birth grant, pace), not the behaviour-leaning traits that overlap the brain. Starvation is a gut-and-reflex problem the old ant solves with no brain at all (it never chooses to eat; it digests what it carries), so this is where the remaining 5 of 24 starved most likely go.

8. **Give the nest a purpose the world enforces, as an owner question.** The bed has a temperature field and the mind already reads warmth. If being out of the nest at night cost energy, a deep nest would be worth digging with no score term at all, and the goal would be in the physics. It changes every world, so it is a design question for the owner, raised once with a paired picture, not an arm to run quietly.

## 5. One measurement for this review

A local test on 4 cores, hA1 founder, 12 seeds, 6 rounds of 200k frames, four arms: score selection and random parents, each with the standard mutation and with no mutation at all (children are exact copies, so only the 30 founding mutants' variation is ever sorted). Each arm's final ants then go into the same fresh world, 200k frames, selection off, beside the unselected hA1 mutants and the old ant. If random parents lose digging with mutation and keep it without, the loss is erosion. If no-mutation score selection matches the standard arm's food, the six-round gain is sorting of existing variation and the mutation can be cut without losing it.

| Arm (12 seeds) | Died out in training | Food home, median | Starved of 24 | New-ground cuts, median |
|---|---|---|---|---|
| Unselected hA1 founders | 0 | 270 | 3.9 | 201 |
| Old ant, same worlds | 0 | 652 | 0.0 | 298 |
| Score selection, standard mutation (the lane's setup) | 2 | 3,934 | 7.1 | 128 |
| Random parents, standard mutation | 4 | 102 | 11.8 | 26 |
| Score selection, no mutation (children are exact copies) | 6 | 394 | 0.3 | 203 |
| Random parents, no mutation | 6 | 50 | 11.7 | 133 |

Three readings, each with the caveat that 12 seeds is a small sample and the no-mutation arms lost half their colonies in training, so their survivors are a biased set.

- **Erosion is real and fast.** Random parents with the standard mutation fell below the unselected founders on everything within six rounds: cuts 201 to 26, food 270 to 102. Without mutation, random parents kept 133 cuts. Finding 2 stands.

- **But the gain needs mutation.** Score selection with exact-copy children reached 394 food home against 3,934. The fifteen-fold gain is new variation under selection, not sorting of the thirty founders. So "less mutation" is not the fix on its own; the 64-seed low-mutation arm the Teacher lane is running says where the trade sits.

- **Starvation reads as mutational damage to the lessons, not as a missing lesson.** No-mutation selection starved 0.3 of 24 and kept all its digging; the standard arm starved 7.1. One round of mutation (the founders' own mutants) already raised starvation from near zero to 3.9. The hungry-at-home lesson hA1 carries is being chipped away by the same mutation that finds the foraging gain.

The lever this points to is mutation aimed away from what already works: a lower rate on the slots the lessons wrote (the slots where hA1 differs from c1 are exactly the hungry lesson) and no structural events on them, plus recombination to make variation without wrecking. Both are now switches in the patch handed to the Teacher lane (`lesson=PATH:F`, `cross=P`), beside a growing headcount band and a lexicase parent pick. A 12-seed test of all four is running; its result is appended below when it lands.

### 5b. The four switches, 12 seeds each against the standard setup

Same founder, rounds and judge as above. The control reproduced the standard arm of the mutation test byte for byte (36 of 36 files), so the switches are the only difference. Paired counts are over the seeds where both colonies survived training.

| Arm | Died out | Food home, median (mean) | Starved of 24 | Cuts, median | Paired vs control: cuts up/down, food up/down, starved fewer/more |
|---|---|---|---|---|---|
| Control (standard) | 2 | 3,934 (3,854) | 7.1 | 128 | |
| Protected lesson slots | 2 | 1,114 (2,737) | 3.0 | 230 | 6/2, 3/5, 6/1 |
| Crossover, half of children | 1 | 3,769 (3,917) | 4.0 | 45 | 2/6, 6/3, 5/4 |
| Growing headcount, 24 to 60 | 4 | 3,258 (3,773) | 4.2 | 130 | 4/2, 2/4, 3/3 |
| Lexicase parent pick, no weights | 2 | 1,827 (2,606) | 7.0 | 146 | 3/5, 4/4, 3/2 |

- **Protected lesson slots is the one to take to 64 seeds.** Starvation fell on 6 of 8 paired seeds and digging stayed above the unselected founders' 201 cuts, with no extra extinctions, where the whole-genome low-mutation arm lost 20 of 64 colonies. It costs food: the median fell, the mean fell by a quarter, and 5 of 8 seeds were lower. One colony reached 15,313, so the food gain is still reachable under it. The mask is coarse (1,081 slots at a tenth of the rate), so the next arms are F = 0.3 and a tighter mask of the slots that moved most.

- **Crossover raises food and lowers starvation but wrecks digging.** Five of eleven colonies ended under 20 cuts. Recombining two eligible parents combines two foraging lines faster than selection alone would; it needs a dig floor before it is used, and protected slots may be that floor. Only 19 percent of children were crossed, since two eligible parents are often not available together, so even at this dose the effect is large.

- **Growing headcount buys nothing at this schedule** and doubles the extinctions. The demand story may still be right, but 24 to 60 ants over one 200k round is the wrong dose; shelved.

- **Lexicase matches the control on digging and starvation with no weights set at all.** Food is probably lower. It is the weight-free baseline the ledger should be judged against, not a gain.

Twelve seeds gives direction, not size. Raw rows and scripts: `fixes-12seeds-*.txt` in the review folder.

## 6. For the Teacher lane

- Arms 1(a) and 3 (whole-genome) are done at 64 seeds; the results are folded into section 4. The four switches in items 2 to 5 are in `inrun-switches-vs-65ab126b.patch` in the review folder, off by default and byte-identical to the unpatched harness when off. Run whichever section 5 favours at 64 seeds against the `ha1next-1022` control, judged by the common garden plus a pre-dug-nest judge world. Switch notes and job lines are in the hand-off file beside the patch.

- Run the clone-colony judge on hA1's most common final genome before the next founder is declared.

- Read `age30.csv` across one pulled batch: the distribution of child minus parent at 30k frames is the mutational-effect spectrum, and it says whether pm 0.03 is wrecking.

- Freeze the ledger weights until something in sections 4.1 to 4.5 has been judged. Every score arm so far has been inside the noise or harmful.

- Write one standard results row per arm (build, founder, seeds, rounds, median home, starved, cuts, rooms, paired counts) to a CSV in the project folder. The design doc's tables are being kept by hand and already disagree with the lane memory on two numbers.

Everything above that cites a number comes from the lane's own logs, the design doc (rev 47), the memory notes or the harness source at compute head 98ad9dc4. Where the review disagrees with a lane, it says which measurement it is reading.
