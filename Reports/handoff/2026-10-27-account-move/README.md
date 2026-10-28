# Master hand-off: the week of 2026-10-20 to 2026-10-27, before the account move

Written 2026-10-27 at the owner's request, when the work moved to a new account that can see **only this
repository**. Project memory, the project chat, threads, the shared project folder and the Artifact pages all stay
behind on the old account. This folder is everything from those places that should outlive them.

This is **information, not instructions**. It records what was done, what was learned, what was planned, the working
practices the owner set in chat, and what was still open. Decide what to do next from it; don't run it as a to-do list.

Main was `6a58114` when this was written; lane notes and PR states were refreshed at about 20:00Z, the copy-test entry was added at 21:10Z, and PF4's result at 23:35Z. Where a number appears here, the report it came from is named. Re-measure
before quoting anything that predates a change to the standard pool or the brain layout.

## The last experiment at the move: PF4 finished and failed its bar

**Pheromones positive control (PF4).** This was the only experiment still running when the hand-off was written. It
finished and was judged at about 23:30Z on 2026-10-27, and **nothing is running now.**
- **What it tested:** scents A **and** B were deafened in both arms, so a trail on the third scent plane was worth what
  the founder's two scents are worth together. The question was whether the evolution set-up can find a signal when
  one clearly pays.
  - Set-up: 16 colonies x 40 generations x 2 worlds, `SCORE=home`, `HARNESS=2`, seed 41.
  - Design: §9.11 of `Reports/brain-training/evolvable-pheromones-2026-10-25.md`.
- **Result: failed its pre-registered bar.** The free arm delivered 1,981 food home and the deaf arm 3,569, a ratio of
  0.56x against a bar of 1.5x or more. Write-up: report §9.12. Data: `Reports/data/evolvable-pheromones-2026-10-27/pf4/`
  (PR #964).
- **What it means:**
  - **E4, the main free-vs-deaf test, should not run as designed.**
  - The arm with no scent at all beat even the founder that has both scents, and its jump came when it stopped laying
    scent.
  - The next step named in §9.12 is to measure what laying scent costs in food delivered home (`PP_EMIT_FREE=1` in
    both arms).
- **Re-run** (about 17 core-hours; `emerge.py` resumes from the last finished generation). From the repo root, once
  per arm (`ARM` = `free`, then `deaf`):

  ```
  cargo build --release --example inrun
  BIN=target/release/examples/inrun PP_GATE=off PP_REACH=off PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05 \
    python3 scratch/pheromones/emerge.py OUT/ARM ARM 16 40 100000 assets/minds/std_pool.txt P=4 SEED=41 EVALS=2 \
    SCORE=home MUT=0.013:0.25:1 CHANMUT=0.013:1.0 DEAFALSO=A,B ZERO=2410-2444 FILL=0.05 CHAN=alarm ALARM=train \
    EVERY=10 LISTEN=4 HARNESS=2
  python3 scratch/pheromones/garden.py OUT/garden free=OUT/free deaf=OUT/deaf DEAFALSO=A,B
  ```

## What is in this folder

| path | what it holds |
|---|---|
| `README.md` | this file: the week, the findings, plans, practices, open items and the outside-the-repo setup |
| `lanes/*.md` | each active thread's own hand-off note, written 2026-10-27 by that thread: state, open to-dos, pending owner decisions, branches, files |
| `project-memory/` | a verbatim copy of the old project memory (103 files; `MEMORY.md` is the index). It is a dated record, not live state. See its README |
| `project-instructions.md` | the standing project instructions as configured on the old project |
| `merged-prs-2026-10-20-to-27.md` | every PR merged to main in the week, newest first (202 rows) |
| `artifacts/` | local copies of the picture pages shown to the owner (HTML and PNG), with an index of every page and Claude Doc |
| `shared-folder/` | files from the old shared project folder (`/mnt/project-files/`) that were not already in the repo: reports, scripts, patches, small data and pictures. See its README for what was left out |

The tools the Teacher lane used on the VM, which lived only in its worktree, are now in the repo at their real paths:
`scratch/cloud/runvm.sh`, `scratch/cloud/countjobs.py` and `scratch/inrun/latch/mklatch_hb.py`.

## Where to start reading the repo

Everything below assumes `CLAUDE.md` has been read. For the ant line, read these in order:

1. `Reports/brain-training/teacher-training/colony-selection-log-2026-10-24.md`: the Teacher lane's batch log. Every
   batch is there, with its deciding measure and result. Read the last five sections first.
2. `Reports/brain-training/teacher-training/training-environment-review-2026-10-27.md`: how the training world
   differs from the lab game.
3. `Reports/brain-training/research/selection-review-2026-10-27.md`: what colony selection actually rewards.
4. `Reports/brain-training/research/brain-cannot-combine-senses-2026-10-26.md`, then the brain-design review (#933)
   and the latch landscape (#940).
5. `Reports/how-we-test.md` and `Reports/how-the-ant-works.md`, both edited in place.

For game design, start from `Reports/teaching-the-colony-2026-10-25.md` and `seats-overview.md` in the folder beside
it. For plants, start from `Reports/plant-community-plan-2026-10-20.md` §8 and `Reports/rootless-plants-2026-10-23.md`.

---

## 1. The week in one page

The main game is the lab (`cargo run --release --bin lab`): a sealed box of soil with plants and ants. The owner's
goal for the week was **an ant whose behaviour is learned rather than hand-coded**, trained by evolution in the
in-run harness, that digs a proper nest and forages for the whole game in the lab. Alongside it ran plants, predators,
game design (how a player teaches the colony) and performance.

**Ant training (the Teacher lane, the main line).**
- **Harness onto main.** The in-run training harness (`examples/inrun.rs`) and colony selection
  (`scratch/inrun/colsel.py`) moved onto main (#833, #835). The batches run on one 64-core Google Cloud spot VM.
- **Colony selection replaced ant-level selection as the method.** Whole colonies are ranked by epsilon-lexicase over
  about 10 cases (food home, starvation, nest, crater, rooms, lab survival and others), and the bottom half is
  refounded from the top half.
- **The ratchet.** Each batch starts from the best pool so far:
  - hA1 / c1 (10-22);
  - farfood3b c12 (#873);
  - laygene2 c8 (#875);
  - price25b c10 (#906);
  - nest7 "hweanrules" c13 (#924). This is today's `assets/minds/std_pool.txt`.
- **Standard defaults turned on:**
  - the home vector (graded, log-distance);
  - the dig head and the light sense;
  - breeding in the bed (`brood=lay`, `lay_cap=48:8`);
  - the room-joining cut;
  - grazing;
  - nutrient by root;
  - the link price 0.25 (#899);
  - last-food memory (#884).
- **The nest became the owner's top complaint** (10-25), ranked in this order:
  1. it is too small;
  2. there is a crater instead of a mouth;
  3. there are no rooms.

  Trained nests were pits packed with stored food. The root cause, found 10-25: **trained ants never eat stored food**,
  so on moving food they starve beside a full larder.
- **Teaching come-home.** A hand-written teacher that acts for hungry ants cut starvation about 10x (Hungry Ants Go
  Home). Copying it into the weights failed (`home-lesson-fit`).
  - The "home, when low" instinct (#928, off) worked: 16 worlds starved 44 vs about 245.
  - The owner then objected that such "senses" are behaviours in disguise (10-26), which led to the next step.
- **The brain itself.**
  - The scorer is additive and saturated, so it cannot combine two senses (#932).
  - A review (#933) produced layout 6, a per-unit gate (#934, on), and layout 7, reach-aware gates (#935, on).
  - Selection has no step-by-step path to a "home had food" latch (#940). A write-gated memory, layout 8, was
    prototyped off (#942, `PP_WRITE=on`).
- **Why selection loses good habits.**
  - In-run mutation outran selection (#937, drift1).
  - Keeping the best half unmutated (`elite=1`, #938) fixed the drift. Progress still stalls by about round 50.
  - The selection review (#939) found:
    - one round's colony rank is mostly noise;
    - the six nest cases carry 63-71% of the vote;
    - lineage collapses to one founder colony.
- **The training world flatters trained pools** (#941):
  - no windfall;
  - judging at 200k frames, while every std_pool colony is dead by 600k;
  - harness-only rules that the game doesn't have.
- **Held-out exam.** Trained brains die out in 3 of the game's 8 lab boxes, where the hand-built ant thrives.
  **No trained pool yet beats the hand-built ant in the lab.** Making the trained pool the lab's default colony is held
  until one survives there.

**Other ant lines.**
- **Brood and breeding:** breeding in training (#839, #845). Learned laying found no brake (#870, #874). The nest still
  packs with crumbs.
- **Evolvable pheromones** (#900, #915, #919, #929, #943, #944):
  - a deleted food trail re-evolves from a dense start;
  - a new free channel's signals emerge, but the foraging gain was mostly crumbs counted as food.
  - The owner restated the goal on 10-27: do ants evolve signals that carry information? Run 1 meets three of the four
    criteria in report §9.10.
- **Trail maze** (#796, #804): the crowd trail meant "go where others are"; silencing it raised food.
- **Far-from-home sense** (#832, #856), **sense-cap audit** (#857), **body genes** (#827), **crater refusal** (#824),
  and **inherited nest** (#834).
- **Colony against colony** (#886).

**Predators.**
- A spider trained to hunt the colony: co-evolved, with sight, handling time and breeding genes, and in the lab
  (#844, #852, #871, #872, #879, #891).
- A centipede nest raider (#887, #927).

**Game design: how a player teaches the colony.**
- The plan recommends **the queen** as the main seat (#896). Four more seats were designed (#898).
- Six playable demos were built: `demo=queen|keeper|guest|enemy|scout`, plus the colony mind verdict (#903-#922).
- The 1996 game *Creatures* was studied for lessons (#882, #893).

**Plants.**
- Only roots drink (#837, **on** by #841).
- Nutrient grows with root (#846, **on** by #847).
- Old age on the plant clock, on by default (#842).
- Shade tolerance and sun scald (#818, #830, #831).
- The seed-trained shrub ships (#814).
- Bed selection for plants (#876, #878).
- A plant pool bred with the colony gave the colony no food back, so it is off (#888, #901).

**Performance** (four lanes, all bit-identical): sweep, field and soil-water skips (#797, #798, #800, #807, #809);
the new ant 4-18% cheaper (#801, #803, #805, #813); the fire pass (#819); and training 19% plus 14-19% faster (#775, #793).

**Lab interface:** redesign rounds one and two (#808, #815), crisp text layer (#817), the lab chronicle logging every
player action (#867, #869).

## 2. What we learned, across lines

These are the findings a later session is most likely to need and least likely to rediscover cheaply. Each has a
report; most also have a `how-we-test.md` or `dead-ends.md` entry.

- **Selection outcomes are noisy; judge with paired seeds and a named measure.** One round's colony rank repeats at
  0.05-0.09 (#939). Five measures at p<.05 give about 1 false win in 5. Quote paired counts and a sign test, and
  apply Bonferroni (owner, 10-27).
- **Mutation load swamps selection at the default rate.** About 26 changed weights plus 4 structural events per child
  (#937). Use `elite=1` in every colsel batch. A lower kpm alone is not the fix (self1).
- **The brain's scorer is saturated** (80% of walk scores past ±0.95), so context can only act through saturation
  (#925, #932). The gate layouts 6-8 are the structural answer. Whether training *uses* them is still open: 16 worlds
  showed no "food at home" sense.
- **Trained ants do things the hand-built ant never needed to learn:** eat from the store, come home when low, use
  windfall before it rots, eat corpse (a mind ant is never offered it at its gut). Every one has shown up as
  starvation in the lab.
- **Stored food counted in cells is mostly crumbs** (pheromone lane, #944). Any score that counts larder cells as food
  needs checking, including colsel's scale case.
- **A "sense" that encodes the wanted behaviour is an instinct** (owner, 10-26). Prefer always-on raw senses, plus an
  architecture that can combine them.
- **A sense at weight 0 ships dead.** Mutation rarely revives it (about 1 in 1,250 children), and the link price strips
  unused links. Seed new rows dense, N(0, 0.05), and run a deaf or off arm (#919).
- **Training-only rules make training easier than the game:** `lay_cap`, the door ban, no old age, 3 founder tanks
  (lab 1), no windfall, and a 40-row soil floor (lab 96). Judge pools in the lab boxes and with a 600k twin (#941).
- **Design docs get engine facts wrong** (4 of 4 seat drafts had 3-5 must-fix errors). Always run an independent
  code-checking reviewer on a design doc.
- **Things that were tried and dropped on the ant line** (Teacher lane plan, 10-22; don't retry without a new input):
  - shorter rounds;
  - higher fixed dig pay (it only delays the loss of digging);
  - room-credit pay;
  - "starved ants never parent" and hungry pay x3;
  - the hungry put-down (`PP_NEEDDROP`);
  - one long world instead of rounds;
  - opening the gut, crop and diet genes (costs food);
  - adaptive weights on the ant score;
  - copying the spoil refusal as is ("never let go").

  Later: weight-copying the come-home teacher (hf1-hf5), the nest5 tramp ramp, the plant pool for food, and the
  laying brake.

## 3. Plans that were on the table

These are proposals, mostly with a recommendation, none of them started unless the lane note says so.

- **Ant line.** The lane notes give these in order (`lanes/teacher-lane.md`):
  1. ~~judge **clean1**~~ (done 19:12Z, PR #955: no arm starves less than the `elite=1 kpm=0.003` control, colony
     selection alone sold the nest for food, the lab boxes are unchanged; keep `elite=1 kpm=0.003`);
  2. **env1**: windfall in training, `donor600`, lifting harness-only rules one at a time;
  3. **write1**: `PP_WRITE` on vs off, the test for making the write gate default-on;
  4. keep diversity in the ratchet (seed from several colonies, report lineage);
  5. decide whether corpse belongs in `food=all`;
  6. switch scale-case food from cells to kJ;
  7. a replay plus starvation traces for every batch.
  8. **can the brain copy the hand-built ant in the game's lab boxes?** (owner, 2026-10-27 21:01Z, after clean1; meant to
     get its own thread in the new account). Trained brains die in two_larders, scattered_larder and feast_and_famine.
     This test separates the two remaining explanations:
     - if a pool fitted to the hand-built ant's choices there survives, the brain is capable, and training's
       environment and selection are the problem (env1, write1);
     - if it cannot copy it, the brain's design is the bottleneck.

     Method: to-do 7 in `lanes/teacher-lane.md`.

  Further out (Teacher plan, 10-22):
  - harder foraging: farther and moving food, judged at an unseen distance;
  - food that runs out (the integrated plant world);
  - a long run of 64 colonies × 30+ rounds with inherited nests;
  - 2 × 32-core VMs and GitHub runners for gardens.
- **The colony mind of the game.**
  - The queen as the main seat. The scout and colony mind become later stages of a queen → scout → mind arc. The enemy
    becomes seasons. The guest's scent model is grafted onto the keeper (`seats-overview.md`).
  - In-life learning, only behind a switch, and on only if learners beat frozen ants on a moving-food test (Creatures
    idea #2, one reward-trained unit per ant).
- **Pheromones.** E4 was redefined as "how often a convention forms" (6 replicates × 16 colonies × 60 generations,
  about 240 core-hours). It needs the readiness review's fixes and the owner's go first
  (`Reports/brain-training/evolvable-pheromones-e4-readiness-2026-10-27.md`, in #944).
  **Overtaken at the move:** PF4, the positive control, failed (see the top of this file), so E4 should not run as
  designed. Measure what laying scent costs first.
- **Plants.**
  - Return to the understory plan (rungs 5-6: recruitment and long-run coexistence).
  - Ship the seed-trained shrub and herb after one lab check.
  - "Pale hungry leaves" (render only), then roots that lean toward rich soil, then a nutrient store.
- **Predators.**
  - Re-measure the spider pools at link price 0.25.
  - Carry the breeding genes into the lab pool.
  - Other predators (ambush, pack, jumping spider, wasp) wait until the spider coexists.
  - The raider against the trained ant's open-room nest.

## 4. Open items by thread, at the move

Owner decisions that were asked and never answered are marked **(owner)**. The lane note has the detail and the
recommendation for each.

| thread | state | open |
|---|---|---|
| Teacher lane | working | the copy test ("can the brain copy the hand-built ant in the lab boxes", owner 21:01Z: own thread); env1; write1; diversity; corpse; crumbs in the scale case. **(owner)**: add the "judge a part by its own job" rules to `how-we-test.md` (recommended yes). **(owner)**: change inrun's default kpm, or make `elite=1` the default (after clean1, the lane recommends yes to both: `elite=1` plus `kpm=0.003`). clean1 logged in PR #955 |
| Merge desk | working | merged #945-#953 by 19:40Z; #944 and #955 (clean1) were still with it. Procedure: `lanes/merge-desk.md` |
| Evolvable pheromones | done | PF4 failed its bar (0.56x vs at least 1.5x; §9.12, PR #964), so E4 should not run as designed. Next named step: measure what laying scent costs (`PP_EMIT_FREE=1` in both arms). E3 was stopped (it ranked on cells) |
| Brood and breeding | blocked on owner | **(owner)**: how to stop the nest packing with crumbs (a selection case is recommended). **(owner)**: the nest-worker caste, acceptable starvation, a training headcount cap. PR #952 |
| Plant community plan | blocked on owner | **(owner)**: plant food or the understory (understory recommended). **(owner)**: ship the seed-trained shrub and herb (yes, after a lab check) |
| Rootless surviving plants | review ready | **(owner)**: build "pale hungry leaves" (recommended yes). PR #945 |
| Queen seat demo | done | **(owner)**: no verdict on the demo yet (does the wild-egg gamble read as fun?). PR #947 |
| Colony mind seat demo | blocked on owner | **(owner)**: fold it into keeper/queen (recommended) |
| Enemy seat demo | blocked on owner | **(owner)**: the next combat mechanic. Hit and run is built; venom is recommended next, after an owner playtest |
| Lone scout seat demo | done | **(owner)**: do her pauses read as checking the way or as stalling? (If stalling, halve `pause_chance`.) PR #946 merged |
| Keeper / guest seat demos | resolved | guest: **(owner)** is begging too generous? (from memory `guest-seat`) |
| Design how players teach the colony | done | **(owner)**: which seat is primary; in-life learning; biology liberties; nature-game verbs |
| Lone predator | done | to-dos only (re-measure at the new link price, more seeds, breeding genes). PR #950 |
| Nest raider | review ready | ideas only. PR #951 |
| Training speed | resolved | none. Its own branch `087uht` is fully merged and deletable |

Also unanswered from earlier in the week:
- whether the old-ant rules still hard-coded on mind ants (larvae self-feeding, the dig head's "when" gate) should come
  off;
- the far64 lab gap: trained pools starve in the owner's far-food lab box.

## 5. Working practices the owner set in chat

Most of these live only in project memory on the old account. Dates are the owner's. Each has a memory file in
`project-memory/` with the reasoning.

**How the work is organised**
- **Land before resolve** (10-24): nothing finishes with its reports only in the shared folder or memory. Reports and
  findings go to `Reports/` (testing rules to `how-we-test.md`) by a PR through the merge desk. Lasting rules go to
  memory. Affected lanes get told.
- **Merge desk** (10-21): one thread owns all merging and watching CI. Lanes open a PR, hand the desk the PR number and
  head SHA, hold further pushes until told, and never merge. The desk merges once all 15 checks are green on the exact
  head, without waiting for owner review. Procedure: `lanes/merge-desk.md` plus `Reports/handoff/merge-desk.md`.
- **No parked work** (10-22): land finished work.
- **Defaults on unless there is real uncertainty** (10-24 17:35Z). A switch stays off only with a measured harm or a
  stated uncertainty in the PR. If turning a feature on changes ant-training results, tell the Teacher lane first.
- **Pictures go on an Artifact page**, never attached to chat (10-25: the owner can't see chat images). The review
  queue (`scripts/review.py`) is retired (10-21). Questions and comparisons go through chat with a page link. **Note
  for the new account:** `CLAUDE.md` still describes `review.py` as the primary channel. That is out of date for this
  owner.
- **Owner questions are often for understanding.** Answer them as questions; don't treat them as orders.
- **Report significance** (10-27): every A-vs-B difference is quoted as paired counts plus a sign-test p, with a
  multiple-comparison note. Nothing is called a trade-off unless it passes.
- **Name the deciding measure before launching a batch.**
- **Compute** (10-23, 10-24):
  - one 64-core Google Cloud VM at a time;
  - more compute from GitHub Actions (`.github/workflows/train.yml`) or Haiku cloud sessions;
  - **never idle waiting on the VM**, use your own machine meanwhile.
  - Lanes may ask for extra runners or Opus lanes only when it matters. Fable lanes are expensive: one review, then
    resolve.

**What the ant must be**
- **One brain design for every ant**, digging and foraging, with no castes (10-21). Roles may later come from signals.
- **One ant** (10-22): the Teacher lane owns the single trained line. Other lanes deliver lessons, teachers, scores,
  arenas and findings onto its founder, never a separate ant. Small test trainings are fine.
- **The pool, not one brain** (10-22): the product is the colony's surviving genome mixture.
- **Ratchet** (10-24): every batch starts from the latest best pool, the best colony becomes the next founder, and A/B
  arms share it.
- **Digging and foraging all game** (10-22): never propose a fix that drops digging. Food is not the only goal.
- **No permanent body rules from the old ant** (10-23): at most a weaned scaffold, judged with the rules off.
- **Graded senses** (10-24): log-distance, no cap and no dead zone, unless clearly worse and untrainable.
- **Senses, not instincts in disguise** (10-26): see §2.
- **Nest priorities** (10-25): first size (judge `nest_open`), then the crater, then rooms. Never trust a nest number
  without the picture.
- **Food far and moving** in training, never one learnable spot. Make a replay page after most runs (10-25).
- **Player actions are all in the data** (10-24): any new box-changing verb calls `world.log_player_action`.
- **One set of physics for every world** (10-21): lab, outdoor and held.
- Chamber height is 7. Ants can starve, and they eat from the colony store (10-21).

## 6. Outside the repository

**Google Cloud.** Credentials are environment variables in the cloud-session environment on the old account:
`GCP_SA_KEY_B64` (a base64 service-account key) and `GCP_PROJECT` (the project's display name; the key carries its own
project id). **They have to be set up again on the new account; the key itself is not written anywhere here.** The
`gcloud` CLI was preconfigured through the session proxy. Code that uses them:
`Reports/brain-training/cloud-compute/gcp/gcp_train_job.py`, `scratch/cloud/inrun_cloud.py` and
`scratch/cloud/runvm.sh`.
- **Bucket:** `pixel-physics-train-512114` (the default in `gcp_train_job.py`, overridden by `GCP_BUCKET`). Batch
  output is under `runs/<name>/`, with `small/`, `jobs/` and `partial/`. Batches there now: drift1, self1, elite1,
  elite1-r1, elite2, clean1, and older ones. elite1's winning colonies (e3 c32, e13 c7, ehl3 c5 at round 95) exist
  only there.
- **No VM running at the move.** The last batch, clean1 (`pixel-train-clean1`, us-central1-a, c4d-highcpu-64 spot),
  finished 19:12Z 10-27, was pulled, judged (PR #955) and deleted. A batch is pulled with
  `python3 scratch/cloud/inrun_cloud.py pull NAME DEST` and the VM deleted with `... delete NAME`; `--hours N`
  makes it self-delete.
- Setup notes: `pip install google-cloud-compute`. The VM builds from a pushed branch, so commit and push the jobs file
  first. Use `--disk 200` for long batches. Lockstep colsel rounds leave CPUs at 60-70%; oversubscribe to about 30
  processes per arm.

**GitHub.**
- The new account needs GitHub connected, and the Claude app installed on `sgladstein/Pixel_Physics`.
- Free training runners: `.github/workflows/train.yml` (push a `.github/train-jobs/<name>.job` to a `train/**`
  branch; no secrets).
- In-session pushes go only to the session's own branch, and branch deletes return 403. Pruning branches is the
  owner's job (`bash scripts/branchcheck.sh` lists the deletable ones).

**Claude projects (old account; all lost in the move).**
- Thread ids and `claude.ai` links in `Reports/` and in `project-memory/` will not open from the new account.
- The shared folder `/mnt/project-files/` was copied as described in `shared-folder/README.md`.
- Artifact pages: see `artifacts/README.md`.
- Owner uploads (`uploads/hearth/`) not already in the repo are in `shared-folder/uploads/hearth/`, with extensions
  added.

## 7. Pull requests and branches at the move

**Hand-off PRs.** Merged 2026-10-27: #945 rootless pictures, #946 scout census, #947 queen M0, #948 Teacher harness
(the compute branch `8y5ak0`, so it carries nothing unmerged), #949 seat-demos board, #950 lone predator, #951 nest
raider, #952 brood, #953 this hand-off. Still with the merge desk at the last refresh (~20:00Z): #944 pheromones
(`iff53x`) and #955 the clean1 result (Teacher lane).

**Older open PRs never handed to the desk:** #706, #705, #702 (draft), #701, #686 (draft), #633 (draft). Leave them
unless the owner says otherwise. **The owner's standing ignore list:** #12, #140, #143, #217, #254, #348, #456, #471,
#476, #478.

**Unmerged branches with no PR, and why each stays unlanded:**
- `claude/project-thread-8y5ak0-teacher-probes`: scratch, labelled "never for main"; superseded by `examples/inrun.rs`.
- `claude/project-thread-8y5ak0-playtest`: swaps in the far64 pool as the lab ant. Held, because the owner keeps
  "trained by default in lab" back until a pool survives.
- `claude/project-thread-oo9r0d` (ant cost counters), `-mblj3z` (soil-pass counters), `scratch/brain-instruments`,
  `scratch/boxrank-test`: scratch instruments.
- `claude/project-thread-eewfjj`: a stale brain-lane note.
- `claude/merge-desk-*`: the desk's scratch stacks.
- About 30 `train/*` and `train-results/*` branches (10-18 to 10-20): GitHub Actions training output.
- `review-queue`: the retired review queue's data. It shares no history with main; never merge it.
- About 60 older `claude/*` branches (08-19 to 10-17) were not audited for unique work.

## 8. What was deliberately not saved

Each of these can be regenerated from a command in its report, or is summarised in a table already in the repo.
- **Per-ant and per-world raw runs:**
  - selection-review elite1 r95 per-ant files (113 MB);
  - plant-food plantwean3/4 per-world files (120 MB);
  - teacher-lane nestmaps-1022 per-world maps (24 MB).

  Their READMEs and summary tables are kept.
- **GIFs and videos over 1 MB:**
  - the nest raider films (30 MB);
  - keeper and guest seat GIFs (about 3 MB each);
  - the spider coexistence GIF;
  - Teacher lane mp4s;
  - plant-community GIFs.
- **Data too large for a page copy:** the Colony Replay viewer's data and the Lab Plant Pool / Bed Selection pages.
  Their numbers are in `Reports/plant-community-2026-10-20/m2/bedsel/README.md`.
- **Claude Docs whose content is in a report:** the teaching plan, the four seats, plant nutrients, the brood plan and
  the pheromone results. Superseded working docs: the Teacher lane plan 10-22 (its dropped-ideas list is in §2 above),
  "Nest score: where it stands", "A ladder of dig metrics", "Brain lane: evaluation and plan" and "Evolvable
  pheromones: initial thoughts".
- **Session scratchpads** (binaries, run dirs): ephemeral, and their kept results are in the PRs above.
