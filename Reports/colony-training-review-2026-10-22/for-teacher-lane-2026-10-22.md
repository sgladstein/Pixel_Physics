# For the Teacher lane: findings from the in-colony training review (Fable review lane, 2026-10-22 ~14:00Z)

Full report: the "In-Colony Training Review" artifact (link in the review thread "In-colony training review").
Short version: keep the approach; the ladder (lesson copy, then selection) is what moved the ant. Three structural
problems stand between the loop as built and the owner's goal (keep digging a large nest AND foraging all game):

1. The training world never contains the late game. Every round is a fresh 26-cell stamp with 30 ants at the door.
   The population is only ever selected in the opening 200k frames of a colony's life; half of new-ground cuts fall
   in the first quarter while births are even (dig lane's trace). Shorter rounds shorten the opening, they do not add
   the late phase. The fixed 24-36 headcount means the dig price hits its floor and never recovers.
2. Heavy mutation per child (pm .03, sd 1, 10 structural events, ~10 generations/round) with weak selection per birth
   erodes whatever is not being paid. Digging died in the random-parent arm too. A local test (below) separates
   erosion from selection drag; result to follow in this folder.
3. Score weights are the least productive lever: ~12 weight/geometry arms, none adopted, several harmful. Lessons
   and body changes moved the ant every time. Freeze the ledger.

Also: the product is one genome (the lab ships assets/minds/h1.raw.txt) but the common garden judges a mixture of 30.
The clone-colony judge from the design doc has never been run. Run it on hA1's most common final genome.

## Arms to run (64 seeds, against the ha1next-1022 hA1 control, same build 98ad9dc; judge by the common garden,
## median home8 / starved / new-ground cuts, plus a judge world that starts with a pre-dug large nest, counting rooms)

Standard args (S): min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20
mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3

A. Long world, no reset:      OUT SEED 1 1200000 score hA1 S            (1 round of 1.2M frames; same total frames as 6x200k)
B. No mutation (sorting only): OUT SEED 6 200000 score hA1 S kpm=0 kevents=0
C. Low mutation:               OUT SEED 6 200000 score hA1 S kpm=0.01 ksigma=0.3 kevents=2
D. Growing headcount band:     needs a harness switch: lo/hi rise over the round (e.g. 24->72 by 20k-frame steps).
   Suggest `grow_band=F:HI` = raise lo and hi by 4 every F frames up to HI. Decides whether demand alone keeps digging.
E. Recombination:              needs a harness switch: half of births = uniform crossover of two above-median parents,
   crossing per scorer unit (a unit's wiring travels whole), then the usual mutation. `cross=0.5`.
F. Lexicase parent pick:       `pick=lexicase` over per-ant rates (home8, new-ground cuts in nest, spoil out, meals
   after work, hungry departures, age); shuffle the order per birth, filter to the best on each in turn. Keep the
   z<-3 hard cull. Replaces the planned adaptive weights with something that needs no weights.

Arm A also wants rounds.sh to accept a single long round (it already does: ROUNDS=1) and the hall refill on (it is).
For A, compare food and cuts per 200k window across the run, not only at the end.

## Zero-cost reads from existing pulled batches
- age30.csv (every job, every round): child-minus-parent score at 30k frames of age = the mutational-effect spectrum.
  If most children sit well below their parent, pm .03 is wrecking and arm C is the fix.
- final/index.csv per round: distinct families at round end. If one line owns the colony by r3, the family cap is
  not holding diversity and crossover (E) matters more.
- ants_f*.txt column 6 (credit so far) at 4k, 8k, 16k, 32k: reliability vs record length (research lane test 2),
  which sets min_age and turn. relia.py already exists for this.

## Process asks
- Nothing below 32 seeds is reported as a result; 8 seeds is a smoke test (three 8-seed calls were reversed today).
- One standard results row per arm into a CSV in /mnt/project-files/colony-training-review/results.csv
  (build, founder, seeds, rounds, median home8, starved, cuts, rooms, paired counts vs control).
- Every founder change: common garden 32 seeds + clone colony of the most common genome + 8 seeds of the lab game.

## The local test for this review (running at 14:00Z; results appended below when done)
hA1, 12 seeds, 6x200k, arms scoreM (standard), randM (random parents, standard mutation), score0 (score, no mutation),
rand0 (random, no mutation); common garden 200k sel=off world seed*1000+99, plus base (unselected hA1 mutants) and the
old ant. Scripts: run.sh / cg.sh / summ.py copied beside this file.

## Teacher lane, 14:10Z: started and read
- Running now on the 64-core VM (batch review-1022, build 98ad9dc, 64 seeds each): A hA1long (1 x 1.2M), B hA1mut0 (kpm=0 kevents=0),
  C hA1mutlo (kpm=0.01 ksigma=0.3 kevents=2). Control: ha1next-1022 hA1c (same build). Common garden 32 seeds will follow.
- age30 spectrum, hA1c seeds 1-16, all 6 rounds: 1,808 child/parent pairs; child minus parent at 30k age median -0.83, 68% below
  parent, p10 -3.78, p90 +2.08. CAVEAT: parents are selected (above median), so regression to the mean alone puts most children
  below their parent, and a child's 30k window is later in the round than its parent's. This does not separate mutation from
  selection; arm B (no mutation) is the control that does.
- Founder lines (round-0 mutant family) alive at round end, median over 16 colonies: r0 2, r1..r5 1. One line owns the colony
  from round 1 on, so the family cap is not keeping diversity: support for recombination/diversity work (E), with the caveat
  that a single line still carries within-line mutational variation.

## Owner ruling 14:01Z (review thread): the goal does not have to be a single brain; save several
Ship the pool, not a champion: the common garden's 30-genome mixture IS the product, so the clone-colony judge above is
no longer needed. Instead report how many distinct lines remain at round 6 and round 20 (final/index.csv family
column), and keep diversity as a goal (family cap, crossover E, lexicase F). The game's births today copy the parent's
mind unchanged (mind::inherit), so a shipped pool is the game's only source of variety; drawing newborn minds from the
saved pool (or lightly mutating in-game) is an owner design question to raise, not to build quietly.

## Teacher lane, 14:25Z
- Saved brains (owner 14:01Z ask), 16 colonies each, ha1next-1022: hA1c round 5 and hA1L20 round 19 both have 24 ants with 24 distinct
  genomes (every child mutates), all from ONE round-0 founder line (from round 1 on). Most common genome holds 1 ant, so the
  clone-colony judge will use the top-scored final genome instead.
- Please build grow_band / cross / pick=lexicase against compute head 65ab126b (claude/project-thread-8y5ak0; adds PP_SPOILVETO and
  PP_NEEDDROP, both env-gated). Leave patches here; I run each at 64 seeds. Ledger frozen: agreed.

## Mutation test result (12 seeds, hA1, 6x200k, judged in fresh worlds 200k sel=off; rows in mutload-12seeds-*.txt here)
arm     extinct | home8 median | starved/24 | cuts20 median
base      0/12  |   270        |  3.9       |  201     (unselected hA1 mutants)
old       0/12  |   652        |  0.0       |  298     (old ant)
scoreM    2/12  | 3,934        |  7.1       |  128     (standard: score selection, standard mutation)
randM     4/12  |   102        | 11.8       |   26     (random parents, standard mutation)
score0    6/12  |   394        |  0.3       |  203     (score selection, NO mutation: children exact copies)
rand0     6/12  |    50        | 11.7       |  133     (random parents, no mutation)
Reading: (1) standard mutation without selection wrecks everything within 6 rounds (erosion is real and fast);
(2) the food gain NEEDS mutation (sorting the 30 founder mutants gives 394, not 3,934); (3) no-mutation selection starves
0.3 and keeps all its digging, so the starvation and part of the dig loss in selected lines reads as mutational damage to
hA1's lessons, not a missing lesson. Caveat: 6 of 12 score0 colonies died out in training (clones jam), survivor bias.
Lever: mutation aimed away from what works. Patch switch `lesson=PATH:F` (slots where the founder differs from PATH,
e.g. hA1 vs c1 = the hungry lesson, mutate at F x rate and take no structural events) is in the patch below, with
`cross=P`, `pick=lexicase`, `grow_band=F:HI`. Your 64-seed low-mutation arm says where the rate trade sits.

## Teacher lane, 14:35Z: review-1022 batch result (64 seeds, same build as the control; totals over the whole run per colony)
| arm | extinct /64 | home food total | starved total |
|---|---|---|---|
| hA1c control 6x200k (ha1next-1022) | 8 | 11,331 | 215 |
| B no mutation (kpm=0 kevents=0) | 25 | 1,696 | 236 |
| C low mutation (kpm=0.01 ksigma=0.3 kevents=2) | 20 (of 63; one upload failed) | 5,459 | 190 |
| A one 1.2M-frame world | 27 | 2,432 | 281 |
Reading: under this score, the food gains come from new mutations; cutting mutation cuts food 2-7x and doubles-triples extinction.
The long world without resets is far worse (27 extinct). Common garden (32 seeds: home, starved, cuts) follows; that is
where any dig-keeping effect of low mutation would show.

## Teacher lane, 14:45Z: common garden of the review arms (seeds 1-32, 200k, sel off, world seed*1000+99, refusals off)
| arm | colonies judged (extinct in batch) | home mean / median | starved /24 | cuts median | <20 cuts |
|---|---|---|---|---|---|
| old ant | 32 (0) | 735 / 716 | 0.0 | 326 | 0 |
| hA1 6x200k control | 28 (4) | 3,467 / 2,691 | 4.9 | 130 | 8 |
| A long 1.2M | 22 (10) | 1,788 / 1,100 | 6.6 | 0 | 14 |
| B no mutation | 18 (14) | 511 / 466 | 1.6 | 210 | 0 |
| C low mutation | 24 (8) | 2,105 / 1,543 | 3.0 | 244 | 2 |
Paired vs control: C home 10/11 (n.s.), starved fewer 17/4 (p=0.007), cuts more 14/6 (p=0.12). B home lower 15/2, starved fewer 15/1.
A worse on everything (cuts 0 in 14 of 22).
Reading: mutation strength IS a dig/forage dial. Low mutation keeps digging and halves starvation in the survivors, but more
colonies die out during training (20 vs 8 of 64), and survivors-only judging flatters it. Next: a middle setting
(kpm=0.02 ksigma=0.5 kevents=5) and low mutation with the hall refill kept, 64 seeds.

## Patch: four switches for the in-run harness (inrun-switches-vs-65ab126b.patch, applies to examples/inrun.rs at 65ab126b; `git apply`)
All off by default; with every switch off the harness is byte-identical to the unpatched one (checked: births.csv of a 40k
round matches the 98ad9dc4 binary). Built and smoke-tested here; a 12-seed run of all four vs the standard setup is in
progress (results appended to this file by ~15:00Z).
- `grow_band=F:HI`   every F frames the headcount band (lo..hi, default 24..36) rises by 4 until hi reaches HI.
                     Tested as grow_band=15000:72 (24->60 ants over a 200k round). Puts the late game (more mouths, more
                     rooms wanted) inside each round instead of a static long world, which your 1.2M arm showed does not work.
- `cross=P`          with probability P a quota child is train::cross(parent, other eligible parent) before mutation
                     (whole scorer-unit groups swap; the fixed scaffold was built for this, dead-ends D4). births.csv
                     kind column says "cross" for those. Hall refills never cross. Tested cross=0.5.
- `pick=lexicase`    parent pick by epsilon-lexicase over five per-ant rates (home deliveries <=8 cols, new-ground cuts,
                     spoil carried out, paid home meals, hungry departures), each per 1k frames of age; objectives in
                     random order, keep ants within one MAD of the best on each, pick at random from what is left.
                     Replaces the ledger-score rank for choosing parents; the ledger still gates eligibility (score>=0 only
                     when under lo). This is the "stop tuning weights" alternative: no weights at all.
- `lesson=PATH:F`    slots where the founder differs from the genome at PATH mutate at F x the weight-step chance and are
                     excluded from structural events. lesson=c1.raw.txt:0.1 protects the hungry-at-home lesson in hA1
                     (the diff hA1 vs c1). Your 64-seed low-mutation result (cuts 244 vs 130, starved 3.0 vs 4.9, but
                     20/64 extinct vs 8) is the whole-genome version of this; the per-slot version should keep the food
                     gain's variation where it is needed and remove the damage where it is not.
Also: a self-adaptive mutation-rate gene (one extra slot per genome, inherited and mutated, as in evolution strategies)
is the next cheap thing to add and fits the pool-of-brains goal (lines settle on their own rates). Not in this patch.

Caveat on `lesson=c1.raw.txt:0.1`: hA1 differs from c1 on 1,081 of 2,790 slots (the DAgger pass rewrote most of the
brain, not a few units), so at F=0.1 this also cuts the overall mutation rate by roughly three quarters. It is between your
low-mutation arm and a surgical protection. A tighter mask (slots where hA1 differs from c1 by more than, say, 0.5) would be
the next refinement; the switch takes any reference genome, so a hand-built mask file works too.

## Teacher lane, 14:55Z
Switches applied at compute head ee38ae5b. Batch switches-1022 queued (64x6 on hA1): hA1mutmid (kpm=0.02 ksigma=0.5 kevents=5), hA1lesson (lesson=scratch/inrun/genomes/c1.raw.txt:0.1), hA1cross (cross=0.5), hA1lex (pick=lexicase), hA1grow (grow_band=15000:72). It runs after the body-genes batch (one 64-core VM per account). Common garden 32 seeds after.

## Fix test result (12 seeds, hA1, 6x200k, judged in fresh worlds 200k sel=off; fixes-12seeds-*.txt here, 15:00Z)
arm     extinct | home8 median (mean) | starved/24 | cuts20 median | colonies <20 cuts | paired vs ctl (cuts hi/lo, home hi/lo, starved fewer/more)
ctl       2/12  | 3,934 (3,854)       |  7.1       |  128          |  1 of 10          |  -
prot      2/12  | 1,114 (2,737)       |  3.0       |  230          |  1 of 10          |  6/2, 3/5, 6/1   lesson=c1.raw.txt:0.1
cross     1/12  | 3,769 (3,917)       |  4.0       |   45          |  5 of 11          |  2/6, 6/3, 5/4   cross=0.5 (19% of children actually crossed)
band      4/12  | 3,258 (3,773)       |  4.2       |  130          |  3 of 8           |  4/2, 2/4, 3/3   grow_band=15000:72
lex       2/12  | 1,827 (2,606)       |  7.0       |  146          |  1 of 10          |  3/5, 4/4, 3/2   pick=lexicase
Reading (12 seeds, so direction not size): PROTECTED LESSON SLOTS is the one to take to 64 seeds: starvation fewer in 6 of 8
paired seeds, digging kept (230, above the unselected founders' 201), extinctions equal to control (your whole-genome low
mutation lost 20/64). It costs food: median down, mean down a quarter, paired lower on 5 of 8; prot_4 reached 15,313 so the
gain is still reachable. CROSSOVER raises food and lowers starvation but wrecks digging (5 of 11 colonies under 20 cuts):
it recombines two foraging lines faster than selection alone; do not run it without a dig floor. GROWING BAND doubles
extinctions and buys no digging at this schedule. LEXICASE matches control on cuts and starvation with no weights at all,
food probably lower; it is the weight-free baseline, not a gain. ctl reproduced the mutload scoreM arm byte for byte (36/36).
Suggested 64-seed arms: lesson=c1:0.1 ; lesson=c1:0.3 ; lesson with a tighter mask (|hA1-c1|>0.5) ; lesson=c1:0.1 + cross=0.5
(to see whether protection gives crossover its dig floor).
