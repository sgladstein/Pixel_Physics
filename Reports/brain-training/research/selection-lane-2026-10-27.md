# Selection lane: runners, what repeats, food counted twice (2026-10-27/28)

Status: **working note of the selection lane** (lane 4 of the plan after the account move). It follows the
selection review ([selection-review-2026-10-27.md](selection-review-2026-10-27.md)) and records what this lane
built and measured against its "change first" list. §5 holds the screens run on Scott's "test all your ideas"
(2026-10-28 01:59Z): islands, case weights and one world per round. Pictures: https://claude.ai/artifact/HCcmdpytbi2npc4c9x2NUL

## In plain words

Colony selection keeps the colonies that did best in a round and refounds the rest from them. It only works if
"did best" says something about the colony and not just about the world it happened to land in. This lane set out to
make that true. Four things so far:

1. **Any lane can now run a colony-selection screen on GitHub's free runners**, not only on the one Google VM. A
   runner batch gives the same colonies as the VM, colony for colony (§1).
2. **Most of what selection ranks on is the luck of the world.** Colonies that are kept unchanged into a fresh world
   keep their food-home order at about 0.3 and their nest order at about 0.2, but their starvation order at 0.03,
   which is nothing. The world itself sets starvation more than the colony does (§2). So the main goal's own
   measure, "fewer ants starve", cannot be selected on as it is counted today.
3. **Food home is counted twice at the door.** An ant puts a cell down at the nest edge, another picks it up two
   cells out (outside the game's home reach), which the game books as a new trip, and puts it down again. A quarter
   to two thirds of a trained colony's deliveries are such re-carries (§3). The Nest lane's `homepick` switch counts
   each cell once.
4. **Most food "delivered" is part-eaten**: 61% of cells home were crumbs, mean worth 674 of a whole cell's 960. Food
   home is now logged in kJ beside cells (§4).

## 1. Batches on GitHub runners

`.github/workflows/batch.yml` and `batch-leg.yml`, with `scripts/batch.py` and `scripts/batch_run.sh` (PR #962,
merged). A `.github/train-jobs/<batch>.batch` file holds the VM's job lines (`colsel`, `emerge`, `rounds.sh`) or
`use=`s a jobs file; pushing it starts the batch. Each colsel or emerge line gets a runner; the batch runs in legs of
up to ~5 h 20 min, each leg publishing to `batch-results/<batch>` and the next resuming from it (colsel pauses
between rounds at `DEADLINE` and resumes from `state.json`). How to use it: `.github/train-jobs/README.md`, "Batches".

**Checks.**
- *Same as the VM.* The test batch `selbt1` ran the first rounds of the VM's `elite2` `e13` line at the same sha
  build flags (x86-64-v3) and seeds: round 0 matched on all 32 colonies' bed results; in round 1 all 22 colonies
  founded the same way matched. The herb box differs because of the seed change in 23d56a4c, not the runner.
- *Pausing changes nothing.* A 4 x 4 run paused and resumed locally is byte-identical to one run straight through;
  `selbt1` paused at its 25-minute leg end and finished in leg 2 on another CPU.
- *Islands off is unchanged.* `islands=1` is byte-identical to no `islands=`.

**Cost.** About 8 min per 32-colony round on the VM's EPYC 9V45; the runners' EPYC 7763 is slower (the last
round and its garden took 25 min). A 32 x 48 screen is about two legs, ~10 h of one runner. A runner has 4 vCPU,
so it runs a colsel line at P=4; the VM's throughput is still the place for 64 x 48 confirmations.

## 2. What repeats: clean1 audited

Read from the clean1 batch's k3 (lab+scale gates, 32 x 48) and kev1 arms with `scratch/inrun/selaudit.py` and
`scratch/inrun/measures.py` (new). **Retest**: the Spearman correlation of a measure across colonies kept twice
(elite=1: the same carry, unmutated, in two fresh worlds), averaged over 45-46 round pairs. **World**: the Spearman
between k3 and kev1 of the same colony index and round, which play the same world. A useful case has high retest
and low world.

| measure | retest k3 | retest kev1 | world (k3 vs kev1) |
|---|---|---|---|
| home8 (food home, cells within 8 columns) | 0.34 | 0.29 | 0.12 |
| nest scale | 0.30 | 0.30 | 0.03 |
| nest | 0.23 | 0.22 | 0.03 |
| alive at the end | 0.15 | 0.11 | 0.11 |
| stored food, kJ | 0.15 | 0.09 | 0.01 |
| lab | 0.05 | 0.19 | 0.01 |
| **starved** | **0.03** | **0.03** | **0.24** |
| starved per ant that lived | 0.06 | 0.05 | 0.20 |
| starved per hatching | 0.07 | 0.05 | 0.19 |
| larvae starved | -0.03 | 0.06 | -0.02 |
| mean tank, second half | -0.04 | 0.03 | 0.30 |
| room shape | -0.03 | -0.02 | |
| the rank-sum colsel ranks on | 0.04 | | |

90% bootstrap intervals are about ±0.1 on every retest row; the full tables are reproducible with
`measures.py ARM` and `measures.py --pair A B`.

What this says:
- **Starvation is set by the world, not the colony**: retest 0.03 in both arms, world 0.24. None of the seven other
  ways of counting it that a round already records repeats better than 0.07. A starvation case as counted spends its
  vote (13% of the rank-sum in k3) on the world.
- Food home, nest and nest scale repeat (0.22-0.34) and are barely set by the world: these are the cases that select.
- The rank-sum as a whole retests at 0.04: the cases that repeat are outvoted by the ones that do not.
- **Children are worse than their donors** on the nest (k3: 293 better, 457 worse of 752 pairs; kev1 321/430), nest
  scale (267/461; 320/422) and lab (343/403; 343/404); not worse on food home. One mutating, pay-ranked round
  still undoes the nest. (Pairs share donors and rounds, so the sign p is optimistic.)
- Donors per round: median 4 in both arms.

Not fixed: `selaudit.py`'s weight table uses the fixed 10-case list even in `kgoal` mode.

**Next for starvation.** A case only selects if it repeats. The candidates already in the data do not; a measure
that can see starvation needs a world held fixed (the common garden, or a short `sel=off` replay of the carry as the
lab case does). That is a design question for the Teacher lane before any arm is spent on it.

## 3. Food home counted twice at the door

Reported by the Pheromones lane; traced here on one std_pool round (seed 5, `food=mix`, 200k frames, the standard
line). The event log had 6,715 field pickups and 2,752 deliveries. **935 deliveries were of a cell later picked up
again as a field pickup at the same spot, 908 of them within 4 columns of the door**; 1,480 field pickups lay within
4 columns of the door.

**Why.** The game decides whether a pickup starts a trip with `food_at_home` (`src/sim/creature.rs`): within the
8 cells round nest material, or touching the store. A cell put down at the nest edge and taken again two cells out
is outside that, so the pickup counts as a trip (`trip_cells += 1`) and the next put-down at the nest is a delivery
(`trip_deliveries`, and inrun's `deliver` row). inrun's own "home" is 12 columns wide, so in its terms the cell never
left home.

**The switch is the Nest lane's `homepick=K`** (inrun, opt-in; commit 0d365a8c on their branch, to land by its own
PR). A trip cell picked up within K columns of the door and put down at the nest again is logged `recarry`, not
`deliver`: it earns no delivery pay and is not food_home, nor colsel's home8 (which reads `deliver` rows, so colsel
needs no change). It books each crop near/far per cell and resets with each fresh crop. This lane built the same
idea in parallel (a per-ant count of far bites, each delivery using one up) and dropped it for theirs: without the
per-crop reset, a far bite later eaten leaves a credit that a re-carry can spend.

**How much.** The traced round, by this lane's scratch counter: 2,081 deliveries counted once (1,479 kJ), 671
re-carries (376 kJ): 24% of cells, 20% of energy, a floor (2,654 far bites were never matched by a delivery, most
eaten, and their credits hide some re-carries). The other lanes' counts on the exam worlds are higher: the Pheromones
lane 28% for std_pool and 34% for clean1's k3c23 (median 40 frames from delivery to re-pickup); the Nest lane 64% of
std_pool's paid deliveries in world 500004 (3,302 of 5,155 loads picked up within 12 columns of the door).

The Nest lane's one-round screen of `homepick=12` over 16 worlds (PR #967) moved no nest measure (crater 6 higher /
10 lower, p 0.45; open nest 11/5, p 0.21; starved 9/7) and paid deliveries fell in 16 of 16, 4,578 -> 2,029 per 200k
frames: under the switch home8 runs at about 44% of its old level. A multi-round arm is with the Teacher lane.

**Does counting once change who wins?** One local colsel run, the k3 line without the lab cases (16 colonies x 3
rounds, 200k frames, homepick off, so pay was unchanged), with this lane's scratch counter tagging re-carries in the
same events: the re-carry share of a colony's home8 was 25-51% (median 35-37% each round), so it differs by
colony. Spearman of home8 counted as today against counted once, per round: 0.96, 0.82, 0.64; top-half overlap 8,
7, 6 of 8. Against once-counted kJ: 0.91, 0.62, 0.42. Round 0 is all std_pool founders; the agreement falls as the
colonies diverge. So **yes, increasingly**: by round 2 a quarter of the colonies home8 keeps would be dropped by a
once count. Sixteen colonies over three rounds is a small sample; the deciding test is the Teacher lane's
multi-round arm.

**Not changed: the game.** `trip_deliveries` in the game's own stats counts re-carries too. Widening
`food_at_home`'s reach would change which pickups set `trip_load`, which the ant's brain reads: that is a game
change for the Teacher lane to weigh, not a scoring fix.

## 4. Stored food in kJ, not cells

The brief asked which scores count stored cells as food. A cell of provisions, fruit or windfall is worth 960 whole
and less as it is eaten (`creature::food_value`); in the traced round 61% of cells delivered were part cells, mean
674. Where each score stands:

| score | counts | now |
|---|---|---|
| colsel home8 (a case) | cells | kJ logged beside it in `home.csv` (`measures.py` reads `home8_kj`) |
| colsel +scale (a case) | stored cells as **space** | right as it is: a crumb takes as much room as a whole cell |
| inrun's in-run delivery pay | cells, unless `fjoule=1` | `fjoule` is on only on planted beds |
| emerge `SCORE=stored` | cells | `SCORE=kj` exists (Pheromones lane's) |
| exam.py deliveries, armsum, evreduce, pairfood, replaydata | cells | reporting only |
| the game's `trip_deliveries` | cells | game stat |

No colsel case ranks on stored cells as food. The measures that would change a ranking are home8 and the in-run pay;
home8's kJ form is now in every round folder, and whether it ranks colonies differently from the cell count is
checked in §3's rank check below.

## 5. The screens: islands, case weights, one world per round

All on GitHub runners, the clean1 k3 line (32 x 48) plus one switch, judged against k3's own round-47 common garden
(same garden worlds, paired by colony) with `scratch/inrun/cgpair.py` (new; a colony that died out in training counts
as the worse side of its pair). **Deciding measure, named in each batch file before launch**: garden starvation at
round 47, two-sided sign test, Bonferroni over the 5 arms (p < 0.01; `selisl1.batch` named 2 arms, p < 0.025 --
neither bar is met). Guards: garden alive and nest.

| arm (batch) | starved: arm / k3, better-worse, p | alive: arm / k3, better-worse, p | other | families left (k3: 1 by round 17) |
|---|---|---|---|---|
| islands=2:4 (selisl1 isl2) | 272 / 293, 17-13, 0.59 | 71 / 56, 23-7, **0.005** | 30 of 32 reached the garden (k3 26) | 1 by round 38 |
| islands=4:4 (selisl1 isl4) | 302 / 293, 14-18, 0.60 | 71 / 56, 23-8, 0.011 | nest 11-21, p 0.11 | 1 by round 39 |
| weights= (selow1 wt1) | 342 / 293, 9-23, 0.020 | 56 / 56, 16-15, 1.0 | food home 10-22 (p 0.05), nest scale 22-9 (p 0.03), crater 127 / 33 | 1 by round 15 |
| oneworld=1, and with weights= (selow1 ow1, owwt) | **void**, see below | | | |

**Islands** fail the deciding measure. They keep more colonies alive to round 47 and delay the fall to one family by
some 20 rounds, but do not stop it: migration every 4 rounds lets a family that wins one island take over the next.
With more families alive, the cases repeat better (selaudit, kept-twice retest: starvation 0.08-0.12 against k3's
0.03, the rank-sum 0.10-0.11 against 0.04), which is the selection review's point that selection among copies of
one family is luck. **The 2-island winner (colony 6, round 47) is worse than std_pool in the game**: level on 16
held-out garden worlds (alive 9-7 of 16) and in a 60k lab-box check (lived 4 of 12), but in the Teacher lane's 600k
exam of the same 48 box-seeds it lived longer in 6 and shorter in 25 (p 0.0009), was alive at the end in 5 against
19 (p 0.007) and had a larger starved share in 29 against 8 (p 0.0008) -- the pattern of every 24-48-round winner
this week. A winner is judged at 600k, not 60k. Its carry: `/mnt/project-files/selection/isl2c6r47_pool.txt`.

**Case weights** (`weights=spoil:0,crater:0.5,shape:0.5,scale:2,lab:3`) did what they were told and lost the main
goal: donors were picked hard for lab (+1.05 sd) and nest scale (+0.76 sd) and against food home (-0.28) and
starvation (-0.17); garden nest scale rose and starvation, food home and the crater got worse. The lab case they
weighted up is the 15k-frame headcount, which the Watchdog lane found does not predict who is alive at 300k in the
same boxes (Spearman -0.14). (`selaudit.py`'s weight-share column still shows the unweighted shares.)

**One world per round was not a test the first time.** inrun seeded the births' stream (mutation, crossover, parent
draws) from the world's seed, so colonies sharing a carry in the one world ran identically: colsel.csv has 1 or 2
distinct outcomes in every round, and all 32 colonies died out together at round 23 (round 15 with weights). inrun
`sseed=` (PR #979) gives each colony its own stream; the rerun (`selow2`) is running, verdict to follow.

## 6. Instruments added

- `scratch/inrun/measures.py`: candidate measures per colony-round from what a colsel round folder keeps, scored
  by retest and world effect (§2).
- `scratch/inrun/cgpair.py`: one arm's common garden against a control's, paired by colony, with a sign test and a
  Bonferroni bar.
- `examples/inrun.rs`: a deliver row carries the cell's worth as put down (v).
- `scratch/inrun/colsel.py`: `islands=`, `oneworld=1`, `weights=`, `home.csv` (home8 in kJ); `DEADLINE` pauses
  between rounds.
- `examples/inrun.rs`: `sseed=`, the births' stream apart from the world's seed.
- `scratch/inrun/rounds.sh`: skips rounds already done, so a runner leg resumes.
