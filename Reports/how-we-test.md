# How we test — the lab playbook

*Living playbook, edited in place (started 2026-10-05). For anyone measuring
the lab colony: ants, the nest, brood, foraging. It collects the methods the
owner's project settled on between 2026-10-03 and 2026-10-05, which until now
lived only in project memory and individual threads. It **links** to the
tools and traps already written up elsewhere rather than repeating them;
[`instruments.md`](instruments.md) stays the authority on what each binary
does, and `.claude/rules/measuring-the-world.md` on world-wide metric traps.*

**Picking up the ant work in a new session?** Read
[`handoff/PLAN-2026-10-07.md`](handoff/PLAN-2026-10-07.md) first: the owner's
goals and working rules, what is on main and on branches, and the ordered next
steps (2026-10-07).

**The short version.** Freeze mutation, land the evolved founder in the dry
goal box, run four seeds side by side, read 100k–300k means. Judge a fix by
the number it was built to move, look at a picture, then trace what it cost
ant by ant. Colony size is never the verdict on its own. Deep-trace every
test until the result is understood, and propose no new fix before then.
A fix with no big measured downside merges **ON** (§2, rule 4).

## 1. The standard setup

| What | Why |
|---|---|
| **Mutation off.** Every measuring example calls `creature::mutation_off_for_measuring()` first in `main()`; `PIXEL_PHYSICS_MUTATION=on` turns it back on | Mutation roughly doubles the spread between seeds, and once two arms differ by one frame each one's newborns draw different mutations. A colony then dies in one arm because a mutation (the pile-loafer: slot 4126 before 2026-10-13, the homing bias since) took over there, not because of the rule under test. Owner, 2026-10-04: *default to off for any testing*. A **new** measuring tool must add the call; one that does not is measuring evolution |
| **The dry goal box** — `assets/lab_scenarios/nest_goal.ron` | The owner's goal bed: no plants, mister off, one endless heap of player food 30 columns east of the nest. Nothing but the colony moves the ground, so a nest picture is the colony's own work |
| **The evolved founder** — `deeptrace founder=evolved` | All deep traces and goal-box A/Bs since 2026-10-04 use it (it holds 11 of 12 seeds to 300k). The game-wide switch to land it in every box is in flight with the Laying lane |
| **`RAYON_NUM_THREADS=1`, four runs at once** | A lab run is no faster on four threads than one, so four single-thread runs is ~4x throughput. Pinned threads also keep counters comparable |
| **Seeds are deterministic per build** | Same build + same seed = same world. So arms compare **paired, seed by seed**, and the baseline is re-run on the same commit, never quoted from memory |
| **Read 100k–300k means**, not one frame | The colony's first 100k is founding. Adjacent census stops of one run can differ by an order of magnitude |
| **Stamp the `main` commit** on every number | Lab numbers do not compare across certain merges: before PR 611 (mutation switch), before `6e42f0fa` (HUNGRY_OUT on), before `06202f793` (WATER_FOOTING on, misted beds), before PR 689 (`PACK_PARTED=carry` on; `PIXEL_PHYSICS_PACK_PARTED=off` reproduces the old ant), before PR 690 (the default `needs=walk` new ant adds `parted_carry`; `needsparts=reach,walled_in` reproduces the old new ant), and for any **mutation-on** number before the mutation-clamp fix (2026-10-13, `MUT_CLAMP` 40 -> 80: at 40 the first mutation cut the homing gate's 45.5 and made the pile-loafer; `PIXEL_PHYSICS_MUT_CLAMP=40` reproduces it) and the `EmptyCrop` homing gate the same day (an input column, so every birth's mutation draws move; `PIXEL_PHYSICS_HOME_GATE=fold` gives the old gate on the new draws). A number without its commit cannot be checked against one with |

**How many seeds.** 1–4 seeds to iterate, labelled as partial and reported
early; **12 seeds only to settle a default**. **Mutation stays frozen in
testing and development; it is only on in the game** (owner, 2026-10-13:
*"All mutation should be frozen during testing and development. It should
only be on in the game"*). The mutation-on check that used to stand here
(2 seeds to 150k before a winner ships) is **dropped for now** (owner, the
same evening: *"Drop check for now"*). What it caught on PR 696: the brood
room costs adults with mutation on, mostly because the gut trait mutates. That is recorded in README's brood-room
section and `/mnt/project-files/late-tunnels/README.md`.

## 2. Judging a fix

**Trace every test until the problem is understood, before the next fix**
(owner, 2026-10-06): *"You run your test and you deep trace it to understand
the problem in full. If you don't fully understand what's going on, stop
suggesting new fixes and implementing and testing them."* So every test run
is followed by a deep trace (§3) of what happened in it. Until that trace
explains the result — what each ant did and why, traced rather than inferred
— the next step is more tracing, not a new switch, a new arm or a re-tune.
A result you cannot explain is reported as unexplained, with what the trace
has ruled out so far.

Owner rulings, 2026-10-05:

1. **Direct metric first.** Every switch is built to change one behaviour. Measure
   *that* — soil set back down inside the nest, hungry nest steps with no
   pull, how far a digger's next cut is from its last — against the shipped
   game, per seed. This is the pass/fail.
2. **Then a picture.** Render the nest (`nestdoor shots=`, `deeptrace`'s
   `nest_f*.txt`) and look. A metric that moved while the nest looks the same
   is a question, not a win.
3. **Then trace the cost.** If the colony got smaller or died, find out **who
   died, of what, and where** — never revert on the colony number alone.
4. **The default is ON.** Owner, 2026-10-12: *"If it fixes something and
   doesn't [have] big downsides it should be on. That should be the rules.
   We are going to have important features forgotten off behind a switch
   with this workflow."* So:
   - **A switch is for testing and for a way back**, never for parking. Build
     the change behind its own switch so the off arm is the baseline, measure
     it, and then **the PR that lands a fix with no big measured downside
     merges it ON**. Do not land it off "for now" and ask later: a fix landed
     off is a fix nobody remembers to turn on.
   - A fix that works on its own metric ships ON even if the colony is
     smaller; the cost goes to tracing.
   - **Leave a change off only for a big measured downside** — colonies
     killed outright, or a harm clearly outside the nudged-baseline spread
     (§4). Say so plainly to the owner **with the numbers**, in the PR body
     and in the switch's doc and its `how-the-ant-works.md` row, so the
     reason it is off can be re-checked later.
   - A change that failed its own target is not "off", it is a dead end:
     a [`dead-ends.md`](dead-ends.md) entry, its branch kept (§8).
   - **"Lands switched off" was never a project rule** (the owner caught it
     presented as one, 2026-10-12). It was a habit of the build-and-measure
     process, and it left fixes forgotten behind switches. The audit of
     every switch that was off on 2026-10-12 is in the project's shared
     folder, `switch-audit/audit-2026-10-12.md`.

   Features default on unless there is a measured harm (owner, 2026-09-27;
   restated 2026-10-12 above).

Worked example: the nest-plan switches (SOIL_WAY, WAY_GAPS, FACE_TRIP,
CROP_DOWN), judged one by one on their own targets with
`scripts/deeptrace_plan.py`. Three worked, CROP_DOWN failed its own target and
was retired; the three together cost 4–12% of the colony, traced to fewer
eggs, not deaths.

## 3. The deep trace

**For "why does the colony do that"** — the method behind every finding of
2026-10-04/05. It is the `funnel` skill (`.claude/skills/funnel/SKILL.md`)
taken to the whole colony.

1. **Write down how it *should* work first**, from the biology, before
   looking at data — a table of rules, the evidence for each, and what our
   ants do today. Then say what measurement would show each rule working.
2. **Record everything.** `examples/deeptrace.rs` records every decision of
   every ant in the goal box. The cheap whole-colony form is
   `ants=0 dig=1 founder=evolved` (add `walk=1 census=1` for walking
   decisions and every death). A full `ants=all` 300k run is ~45 min and
   ~3 GB: keep it in scratch. Recording changes nothing in the world.
3. **Coarse, then deep.** Colony windows and budgets first
   (`scripts/deeptrace_world.py`, `scripts/deeptrace.py`), then follow
   single ants through the decisions that matter
   (`scripts/deeptrace_dig.py dig|soil|rooms|brood|journeys|face|fed`,
   `scripts/deeptrace_plan.py` for switch verdicts; `only=id,...` replays
   chosen ants). [`instruments.md`](instruments.md) §`deeptrace` says what
   each reader answers.
4. **Baseline arms.** The shipped game on the same seeds and build beside
   every arm.
5. **Per-seed numbers**, never only a pooled one, with a result page.
6. **Mark each claim traced or inferred.** "Eggs track brood falls (r 0.81);
   causal link inferred, not intervened" is the shape.

## 4. Numbers that have misled us

Check this list before quoting a number. World-wide traps (liquids, cascades,
frame cost) are in `.claude/rules/measuring-the-world.md`; method traps
(paired runs, `n` across arms, tidy results) are in `CLAUDE.md` §Method.

- **Plain `deliveries`** counts every put-down at home, including crumbs lifted
  inside the nest and set straight back (4–12x the real figure). Use
  **`trip_deliveries`** (`labforage trip_deliveries=`, census `tripD`) or pile
  refills. **The mind's own `MindCounts::delivered` is the same trap**: a
  louder food trail at the door once read as "+43% food home" on it and was
  noise on `trip_deliveries` (more handling at the door, no more food;
  `Reports/teaching-the-colony-2026-10-25/colony-mind-verdict.md`).
- **Raw larvae starved** falls whenever eggs fall. Use **larvae starved per egg**.
- **Total cuts.** 80–85% of nest cuts are re-digs of soil already moved. Count
  **new-ground cuts**.
- **A share of ant-time deep in the nest, read as a gain.** 0.5% against
  0.6–1.6% of ant-time reads as "raised on all four seeds"; it is about 3
  against 4–9 ants of ~550, still basically nobody (owner, 2026-10-06). Give
  it as ants, beside the baseline: the scorecard's **ANTS DEEP** line, door
  column apart from the rest, because most deep ants sit on the brood pile in
  the shaft under the door. Then read **ANTS LIVING DEEP** under it: a jam
  shows as deep ants too. Homing seed 2's 12 deep ants were a crowd of hungry
  ants dipping below row 10 for a few frames at a time, not ants living
  there; the line counts the fed ones, and `--stays` on a `dig=1` run counts
  the fed ones in stays of 50+ frames and gives the median stay.
- **An ant standing on brood hides it in every picture.** The brood cell is lifted
  out from under the walker, so the grid, the maps (`a`) and the lab's own drawing
  all show the ant. On the shipped game 63-99% of the ants in each row of the
  column over and under the door stand on brood (2026-10-06), so a solid column
  of ants reads as ants that will not walk into it. Read `walk=1`'s `nb` column
  (`B`) or `broodstep.py`, not the picture.
- **`broodlog.csv`'s `laid` rows** (`deeptrace dig=1`) are written for any egg
  not seen the frame before, and an egg under a standing ant (above) flickers
  `gone`/`laid`, so there are ~3x as many `laid` rows as `eggs_laid` (steady
  seed 1, 100-300k: 6,279 against 1,927). A reader that starts a life at the
  last `laid` row puts the egg where it was **last seen as an egg**, already
  fallen: deep starvers read "laid 9-14 rows under the old ground" when eggs
  are laid 0-1 rows under it (2026-10-10). Count an egg only when its id was
  not seen in the last 500 frames (`/mnt/project-files/deep-larvae/laidwhere.py`,
  within 0.5% of `eggs_laid`).
- **`starvewhere.py` "encased" before 2026-10-07** counted depth, not soil.
  Its flood stopped at row 175, so every starver deeper than 15 rows read
  encased whatever was round it: 50 of 51 on LAY_BAR=body seed 1 with the
  smell store, 0 after the fix (all 50 "door open"). Re-run the tool before
  quoting an older "encased". And "door open" says a path joins the ant to
  the door, not that it can climb it: a body with nothing beside it falls.
- **`digbox` `SCORE`** — read its `n=` first.
- **Room censuses** on a wet floor (water splits one room into "chambers"),
  or split by a brood pile; brood pockets under 30 cells are not rooms;
  tunnels in the spoil mound are not the nest. A nest is roofed void.
- **"Home"** has meant different cells in different builds; do not compare it
  across them.
- **A 12-seed end count** on the played bed can mislead either way (seeds 1–12
  and 13–24 once pointed opposite ways). Use means over a window, paired.
- **A colony-size difference under about 20%, from one run per seed.** On
  the steady bed a colony is chaotic: placing the second colony one frame
  later (annual herbs off, `t2=84890`, 12 seeds) moved mean adults over
  100-450k from 216 to 181, higher on only 3 of 12 seeds, with 5 zero-adult
  probes; seed 4 ranged 175-341 across four nudged arms. That one-frame nudge
  produced the whole "16% smaller colony" first blamed on annual herbs
  (2026-10-10, `/mnt/project-files/planted-balance/notes-2026-10-10.md`). So a
  colony-size claim needs a **second baseline arm nudged by one frame**, and
  the effect must sit outside the spread between the two baselines. Arms on
  few seeds still also need a no-effect **placebo arm** beside them.
- **A single-seed result** for a rule that helps some seeds and kills others
  is usually mutation, not the rule — see §1.
- **labforage `aloft`** is food held off the ground, not ants.

Added 2026-10-24 from the colony-selection threads of 2026-10-21 to 2026-10-24
(the report sweep; evidence in
[`brain-training/dig-scoring-2026-10-22/`](brain-training/dig-scoring-2026-10-22/README.md),
[`brain-training/inrun-selection/batches-2026-10-22-to-24.md`](brain-training/inrun-selection/batches-2026-10-22-to-24.md)
and [`brain-training/teacher-training/inherited-nest-2026-10-23.md`](brain-training/teacher-training/inherited-nest-2026-10-23.md)):

- **Digging counted inside a selection run.** In-run totals are confounded by
  which colonies survived and by cutting falling through the round. Judge
  digging in the **common garden** (selection off, fixed fresh worlds), and
  count kept nest cuts (`nestcount.py`), not raw cuts near the door.
- **Two groups of ants compared over a round without matching time.** Cutting
  falls through a round, so "the survivors cut 40% less" was an artefact;
  matched to the same 10k-frame bins they cut the same (0.96 vs 1.01).
- **Short rounds look safer because of the clock**: ants rarely starve inside
  a 50k-frame round, so fewer extinctions there say nothing about the brain.
- **Home food once nests hold food.** Colonies re-drop stored food at home and
  each drop books as a delivery (colsel17: about 11k of 13k). Judge feeding by
  mean tank, stored nest food and starved.
- **Nest numbers from before 2026-10-22 20:55Z (d3392d97)** counted door
  markers as roof and the door's column as nest, so colsel3-7's nests are
  inflated. Re-judge before quoting.
- **A change to what the nest map keeps changes what is inherited.** colsel14
  was void because a meter fix also carried stored food round to round. Check
  the new world's first frames, not only the meter.
- **A pay rule cannot change which cell an ant cuts** (colsel11, colsel12:
  same crater shape under every charge); it only changes whether it digs.
- **A lab chronicle from before 2026-10-24 (the far64 playtest PR) has no
  painted-food lines.** `Lab::release` dropped the stroke unwritten, so a
  food drop shows only as a `fruit_j` jump after a `PAUSED`/`RESUMED` pair.
- **A pool trained in `inrun` has only met a capped colony.** `lay_cap=48:8`
  is the harness's, not the game's; the lab colony is uncapped and booms then
  starves (far64 playtest: 710 ants, then 817 starved in 10k frames with 2.4 MJ
  left). Judge a pool in the lab on starvation after its peak, not only on
  whether it is alive.
- **A playtest chronicle can be replayed and traced per ant.**
  `examples/replay actions=CHRONICLE.actions.csv newant=1 trace=25` rebuilds the
  owner's box from its logged plantings, colonies and food, and its census
  matched the owner's exactly to F40000 (plant playtest, 2026-10-25). Run
  `newant=0` beside it: the hand-built ant in the same box says whether the
  box or the brain is the problem.
- **A grazing ant must be judged on bites by hunger, not on bites.** The
  trained ant's bite rate on a plant is flat in its energy (4-5% of 25-frame
  windows from starving to 20 tanks), so a high bite total can hide founders
  starving beside plants and fat ants that never go home.
- **`examples/labshot` never runs the lab mister**: any plant or water number
  from it is a box with no rain.
- **Rows of `events.csv` and `final/index.csv` are HashMap-ordered**: compare
  sorted. A new loop over a HashMap that touches credit or counters must sort
  first (2 of 64 seeds diverged between runs once).
- **A swallow count is not food eaten for an animal that can put food down.**
  `food_log` writes a row each time a cell is swallowed, and a mind with Put
  Down can drop the same cell of crop and pick it up again: a centipede round
  logged 7,300 swallows against 37 cells bitten (nest raider, 2026-10-25). Pay
  and judge such an animal on **`digest_log`** (`predtrain credit=digest`, the
  default when the verbs include Drop).
- **A predator's breeding before 2026-10-25 could be paid in living ants.**
  `try_bud`'s provisions counted ant cells round the head (3 births in 23
  frames for one centipede); spider lab results judged on births from before
  the nest raider PR are not comparable with later ones.
- **`LifeCounters::bites` is every pickup into the crop, not a bite of prey.**
  A raider dropping and re-picking one crop cell counted 42,000 "bites" a round
  against ~350 ants killed (nest raider, 2026-10-26). Pay or judge a predator on
  deaths (`death_log`, cause Killed), never on `bites`.
- **A predator's training bed can rank the opposite way to the lab.** Four
  raiders for 30,000 frames on a 256-wide bed reward the few that reach the
  door; in a 300,000-frame lab box the raid is a crowd's. A nest smell and pools
  trained with it won 23 of 24 beds and cut deep raids in the lab 538 -> 64-78.
  Judge a predator in the lab, over a long run.
- **"Killed under the surface" is mostly the mound and the shaft's mouth.**
  Count deep kills (more than two cells under the column's surface; `spiderlab`
  with `PP_KILLLOG`) when the question is whether a predator went in.
- **A trained ant that starves faster than the old ant may be paying for its
  brain, not missing a rule.** Every live link is billed per tick at the
  hand-built ant's price: `std_pool`'s ~1,900 links cost 0.85 J a tick against
  an idle body's 0.1, so one tank lasts 1,000 frames rather than 3,700. Before
  hunting a behaviour, rerun at `PIXEL_PHYSICS_LINK_PRICE=0` (or `labsurvive
  tax=0`); if the gap closes, it is the bill. Judge a lab number at the arm's
  own price (old-ant-rules-2026-10-25: fruit 200 alive 28 at price 1, 150 at 0).
- **`PP_SCENT_BLIND=1` is a partial blinding.** It zeroes the head scent inputs
  only; the per-option scent features and the dig head's A feature still read.
  For "does the colony listen to this scent?" use **`PP_SCENT_DEAF=A,B,alarm|all`**
  (every place a mind reads it) and, as the cleaner test, **`PP_SCENT_SHIFT=CH:40`**
  (a real plane read in the wrong place; deaf is off-distribution). Muting the
  emitters gives exactly the deaf result when ants are the only readers and
  writers of a plane, so a "silent" arm is a wasted arm
  (evolvable-pheromones-2026-10-25 §4).
- **A deaf colony that dies may be missing a level, not a message.** Deaf reads
  0, which also moves every bias the weights were tuned around. Before calling
  a scent "information", read it as a constant at several levels
  (**`PP_SCENT_CONST=CH:V`**): if every level dies too, the ants used *where*
  the scent was (alarm on the free arm's best: 5-8/8 extinct at any level, 0/8
  on the real map; evolvable-pheromones-2026-10-25 §9.1).
- **An emit row evolved from zero tests the mutation operator, not the
  signal.** Add events almost never land on the one input a state-conditioned
  emitter needs: a deleted food-trail row was not rebuilt from zero (2 of 2),
  but was rebuilt laden-only in ~40 generations from a dense `N(0, 0.05)` start
  (`emerge.py FILL=`). Start an evolving emit row dense, and give its run a
  deaf arm (evolvable-pheromones-2026-10-25 §9.3-9.4).
- **A scripted player's "did N" is a request until the run reads back what
  happened.** A harness that asked to carry 10 ants lifted about 6, because
  the ants nearest the nest were stacked on one another and the carry refuses
  a stack. Its first verdict, "carrying changes nothing at colony scale", was
  wrong; with true counts, carrying ~17 ants raised station visits by a median
  105 (10 of 12 seeds). Print the count of each act that actually happened
  (`keeperdemo`'s `carried=`) beside every result, and pick targets the act
  can take (`creature::can_carry`) (keeper-seat-demo-2026-10-25).
- **A per-member score in a lab colony measures the ant's id, not its mind, unless
  the member order is shuffled.** The lab hands ant `id` the member `id % n`, and
  which founders live through the thin start follows the id: on `steady_income`
  members 6-25 lost both founders by ~1,100 frames on all 16 unshuffled runs,
  while a copy of member 7 under another label foraged normally. Shuffle labels
  per seed (`queen_m0 shuffle=1`) and compare against a label-permutation null
  (`Reports/teaching-the-colony-2026-10-25/queen-seat-demo.md`, M0a).

## 5. Tools, by question

| Question | Tool |
|---|---|
| Why does every ant do what it does? | `deeptrace` + its readers (§3) |
| Does a switch hit its own target? | `deeptrace walk=1 census=1` + `scripts/deeptrace_plan.py` |
| Is my arm's off run the shipped game, and how does the arm compare with the baseline? | `scripts/deeptrace_tools/identity.py` first, then `scripts/deeptrace_tools/scorecard.py` (one column per run). Read its ANTS DEEP line (about N of M ants, door column / off it) and ANTS LIVING DEEP under it, not the headline alone: most of the time in the dug nest is the knot at the door |
| Who starved where, and were they shut in? | `scripts/deeptrace_tools/starvewhere.py`, `spells.py` and `doorseal.py`, on runs made with `hungry=1 mapevery=1000` |
| Where did the digging go, and how many chambers? | `scripts/deeptrace_tools/digwhere.py` and `chambers.py` (the owner's chamber rule) |
| Did a dropped pellet have a choice of cell? | `deeptrace drops=1` + `scripts/deeptrace_tools/dropchoice.py` |
| Is the nest full, and which part? | `scripts/deeptrace_tools/bandfill.py`: free cells, ants and brood per depth band, map by map, beside the ants there (`--show F1-F2` for each map) |
| Is there a clear way into the nest through the mound? | `scripts/deeptrace_tools/moundway.py`: the shortest way in from outside, its narrowest neck, how full of ants it is, the mound's dead ends, and maps with no way in at all (`--png` draws it with the ants hidden) |
| Do ants step into the brood, or does something refuse them? | `deeptrace walk=1 digfrom=F` + `scripts/deeptrace_tools/broodstep.py` (steps offered against taken, brood against open ground by direction, beside what the chooser's scores predict; what was refused and why) and `fedboundary.py` (fed and hungry ants by what is straight below them, row band by row band) |
| Does a dig rule's input reach the ants that dig? | `deeptrace dig=1` + `scripts/deeptrace_tools/digtrace.py`: each dig decision by where the ant stood and by the Crowding it read |
| How far does each plant get, and what stops it? (seed, germinated, grown to seed size, seeded; what each one died of) | `planttrace life=1` + `scripts/planttrace.py` (`funnel`, `deaths`, `spells`, `tips` for why growing tips stopped, `life ID` for one plant's biography); `cull=F` is the known-answer control |
| Why did this tip go where it went, or stop where it stopped? | `planttrace growlog=1 only=ID` + `scripts/planttrace.py tip OUT ID [X Y]` (one tip, visit by visit); `check` proves the log is the rule's own arithmetic, `--drop TERM` its positive control; `shots=F,F at=x,y,w,h` photographs the same run |
| Digging in a simple box | `digbox` |
| The nest door and its pictures | `nestdoor` (`shots=` renders with the game's renderer) |
| Foraging and trails | `labforage`, `trailprofile`, `trailfollow` |
| What did a *played* session's nest do: rooms, brood, who starved where? | the lab's chronicle: the nest block at the end of `census.csv` and the indented lines under each CENSUS row (`lab::nestcensus`; README *Chronicle status*). `chronicle scenario=nest_goal export=DIR` writes the same files headlessly |
| Did a fix that works on a simple bed also hold in the played game? | `scripts/labbench.py BASE NEW` (two worktrees, pinned binaries, interleaved seeds, `labpair.py` table) — the **last** check, not a diagnosis |

Before building anything new, grep [`instruments.md`](instruments.md).

## 6. Showing results

- **One chart page per comparison**, published as an Artifact. The
  templates and how to publish pictures with them are in the project's shared
  folder, `charts/README.md` (arm tabs over per-seed panels; the nest variant
  puts a nest picture per seed at the slider's frame). Open space pale, ants
  dark, water blue.
- Lead with the finding in world terms; put the commit and instrument in the
  footnote. Per-seed numbers, not only means.
- Judge-by-eye questions go to the owner as a card (`review` skill).

## 7. Training batches on the VM

For colony-selection (`colsel.py`) and in-run batches. The evidence is the
[training compute audit](brain-training/research/training-compute-audit-2026-10-24.md)
(2026-10-24, all 52 batches in the bucket); the owner asked for it to become
the rule, and the Teacher lane adopted it the same day.

- **Every idea needs a comparison, but rarely a fresh one.** Judging an idea
  arm against its own first garden matched the real call on only 9 of 18
  measures, and 3 came out backwards: selection alone improves a colony.
  Seeds are deterministic per build (§1), so an arm with the same build,
  founder pool, settings and seeds is already a control. Look for it in the
  bucket before running one; never rerun it (7% of all colony-rounds were
  byte-identical reruns). Resume a shared prefix rather than replaying it.
- **One control per ratchet turn**, shared by every idea tested in that turn.
- **Garden every 8 rounds** (`CG_EVERY=8`; 4 for short runs). At the old
  default of 2 the garden was 30% of colony-selection CPU.
- **Screen at 32 colonies × 16 rounds, confirm at 64 × 48.** 32 colonies keep
  99% of large-effect calls and 85% of medium ones; no smaller sample ever
  reversed a call. Rule changes show at the first garden. Learned behaviour
  settles by round 23–31, and erosion only shows in long runs.
- **Name the deciding measure before launch.** Five measures at p < 0.05 give
  an idea that does nothing a "significant" change about one time in five.
- **Count extinct colonies in the garden** as alive 0, nest 0; the garden
  drops them, so its medians otherwise describe survivors only.

## 8. Before you push

- `rustfmt` the files you touched (never a whole-project `cargo fmt` riding
  along), then
  `cargo clippy --all-targets --release --locked -- -D warnings`.
- The touched module's tests only (`cargo test --release --lib <module>`, plus
  the `tests/*.rs` file for any registry you added to). **Do not run the full
  suite locally**: CI runs it on every push (the owner caught three lanes
  running it locally, 2026-10-05).
- Merge `main` in right before hand-off; `bash scripts/docscheck.sh` after
  every merge.
- `bash scripts/branchcheck.sh --who-touched <path>` before writing into a
  file another lane owns.
- Change an ant mechanism → update [`how-the-ant-works.md`](how-the-ant-works.md)
  in the same commit. A switch that is tried and left off → a
  [`dead-ends.md`](dead-ends.md) entry with the condition it depends on.
- Shipping a fix → its switch's default is **on** in this PR, unless it has
  a big measured downside, which the PR body states with numbers (§2,
  rule 4).
- Stage explicit paths; never `git add -A`.
