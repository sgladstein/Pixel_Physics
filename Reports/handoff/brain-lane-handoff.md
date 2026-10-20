# Handoff: training the ant's brain (stage 2), for an agent with only this repo

**Last updated: 2026-10-20 04:10 UTC.** Written by the session that has run
this work since 2026-10-17, because the owner's account was about to run out
of usage. It is updated in place through the night as each task finishes:
read the **Log** at the bottom first to see what changed since this header.

You have the repo and nothing else: not the project's shared folder, not its
chat, not its other sessions. Everything you need from those has been copied
in (see *Where things are*). The repo's own `CLAUDE.md` still applies and is
worth reading; this file adds what is specific to the brain work and the
owner's standing rulings on it, which are not written down anywhere else.

**Branch.** Step 4b (the newest trainer change) is on `main` (PR 733). This
file is updated on branch `claude/project-thread-eewfjj` through the night and
reaches `main` through its PR: if `main`'s copy is older than the branch's,
read the branch's.

---

## 1. What this is, in one paragraph

The game's ants get a **general brain** (`src/sim/creature/mind.rs`, the
"mind"): one option-scoring network with memory. Selection finds its weights
rather than anyone hand-wiring behaviour. A **trainer** (`src/train/`, driven
by `examples/train.rs`) evolves a population of minds in small arenas and
scores them. The minds climb a **ladder of rungs**: trail → forage (scarce
food) → **dig** → brood. Each rung has its own arena, score, controls and pass
bar. The design of record is
`Reports/brain-training/design/general-brain-design-rev3.md`. The dig rung's
design (goal, 12 baby steps, scoring, bars) is
`Reports/brain-training/ladder/dig-rung-design.md`; it is long, so read its
*step 4* section first.

## 2. Where it stands right now

| Rung | State |
|---|---|
| Trail | **Accepted** by the owner (signed trail score 0.94 on 3 seeds). |
| Forage (scarce food, 80 columns out) | **Not passed.** Ants find food but make too few laden trips home. A one-ant "fetch" arena is the stepping stone, and it is trained alongside dig as a kept skill. |
| Dig | **In progress, step 4.** Step 4 is "carry soil up and out of the shaft". Steps 1-3 (cut, let go, put down away from the hole) are behind it. |
| Brood | Not started. |

**Dig step 4, the numbers.**
- Arena: `challenge=soil step=4 shaft=4`, a 4-row founding shaft on the dig generator's mirrored beds.
- Bar (all on the 12 sealed hold-out worlds):
  - at least **5 full loads per world** on at least 11 of 12 worlds;
  - pooled `full_share` at least 0.75;
  - the blinded control (senses cut) under 2.5 full loads.
- A **full load** is a pellet cut at the shaft's depth or deeper and carried that whole height up and out.

The only seed that climbed is **s111**. Every row below is on the 12 sealed worlds (worse side of each mirrored pair):

| s111 checkpoint | soil score | full loads / world | blinded soil |
|---|---|---|---|
| its start (`training/starts/fretrain-s91.txt`) | 10.4 | 0.5 | |
| gen 62 | 26.9 | 1.0 | 6.9 |
| gen 126 (`training/starts/s111-g126.txt`) | 37.5 | 1.83 | 10.6 |
| gen 226 (`training/starts/s111-g226.txt`) | 42.9 | 2.0 | 9.2 |

Seeds 112 and 114 stayed flat (hold-out about 1.0-1.5). Seed 113 auto-stopped at generation 40.

**Why s111 stalled.** The ladder research lane replayed it with a cut-by-cut log
(chart: `Reports/brain-training/ladder/pictures/s111-where-it-cuts-2026-10-19.png`).
- **Carrying is solved.** It carries 2.83 of its 3.25 shaft-foot cuts a world all the way out.
- **Where it cuts is not.** Of 9.4 first cuts a world: 2.2 are in the top row, which pays nothing; 4.0 are in the shaft's *walls*; 3.25 are at the foot.
- The wall cuts were 34% of its score, so they were still worth chasing.
- More frames would not help much: 4x the frames gave only 1.7x the loads.

**The fix adopted: step 4b** (the owner, 2026-10-19 23:27: "Take that and continue training").
- Code: trainer flag `foot=1`, PR 733 (merged). From step 4, a first cut pays only if it is at least `shaft` rows down, so a wall cut earns nothing.
- `foot=0` is the default and is the old rule, called rev 2.8.
- s111 gen 226 rescored under 4b: soil 22.5 against its start's 4.5, blinded 9.2, full loads 1.67.

**Round 1 of step 4b is done** (s131-s134, from s111 gen 226, `foot=1`). Only **s134** gained: 3.67 full loads per world on the sealed worlds against its start's 2.83, 3 of 12 worlds at 5 or more. Full table in the Log (02:30 entry). Files: `Reports/brain-training/stage2/dig/4b/`.

**Round 2 is done: no gain** (s141-s144, from s134, same settings as round 1). After about 50 generations no seed's hold-out champion beat the start: best 1.52 / 1.55 / 1.59 / 1.51 against s134's 1.63 (training hold-out, combined score). s142 auto-stopped at generation 53; s141, s143 and s144 were stopped at generations 53, 55 and 51 to free the CPUs. Log entry 04:05 has the reading. Run bundles (no hold-best beat the start, so none is worth resuming): `Reports/brain-training/stage2/dig/4b2/`.

**Running tonight: round 3, fine-tuning with smaller mutations.**

*Four new seeds from s134's best (`training/starts/s134-4b-g34.txt`), `foot=1`, 100 generations each. Two settings, two seeds each:*
- `dig-4b3-s151`, `dig-4b3-s152`: `pm=0.03 sigma=0.1` (each mutation a third the size);
- `dig-4b3-s153`, `dig-4b3-s154`: `pm=0.01 sigma=0.3` (a third as many weights touched).

Everything else is as in rounds 1-2. Google, `c4d-highcpu-16`, ref `96b8271b72e1d9c87d594edc44a531d17577ab3e` (main). Started 04:07 UTC; expect them done about 07:10 UTC unless the auto-stop ends one sooner.
  - Each VM writes a snapshot to the bucket every 5 minutes, as `runs/<name>/latest.tgz`, and a `DONE` marker at the end. The VMs need no session to keep running.
  - Fetch with the launcher (section 5): `python3 Reports/brain-training/cloud-compute/gcp/gcp_train_job.py fetch dig-4b3-s151 <dir>`. That saves `<dir>/dig-4b3-s151-snap.tar.gz` holding `hold-best.txt`, `best.txt`, `progress.csv`, `holdout.csv`, `train.log`, `curves.svg` and `state.txt`.
  - Then `status dig-4b3-s151` should say the VM is gone. If it doesn't, run `delete dig-4b3-s151` (a VM left running costs money).
  - **What to read first:** `holdout.csv`, rows `who=champion` against `who=start` (1.629). Generation 0's champion is the quick test of the idea: in round 2 it was 0.94, far below the start.
- **Run each job in one place only** (the owner, 00:53: "Don't double up that is a waste of work. Just pick one"). The stale `train-results/dig-4b-s13X` branches on GitHub are from a cancelled Actions copy; ignore them.

**Stop rule, agreed in advance.** If no seed is past **3 full loads a world by
generation 100**, the pay rule is not what holds it back. More generations won't
fix it; the rung needs a design change. Take that to the owner.

## 3. What to do next, in order

1. **When the round-3 runs (s151-s154) finish**, take each run's `hold-best.txt`. Score each on the 12 sealed worlds with `foot=1`, against `training/starts/s134-4b-g34.txt` (command in section 4 and the Log's 01:45 entry). Read `full_loads` per world against the bar (5 on 11 of 12 worlds), then the blinded row. The number to beat is s134's: 3.67 full loads per world, 3 of 12 worlds at 5 or more.
2. **Make a before/after GIF** (section 4) and a training-curves picture (`curves.png` comes with every run). Send both to the owner with the numbers. **Never call a rung passed before the owner has seen the pictures and the GIF.**
3. **If no seed passes but some climb**, resume the best one (exact resume, section 5) rather than starting fresh. **If none is past 3 full loads by gen 100**, stop and bring the owner a design question with your recommendation.
3b. **Option the owner raised (02:45-02:48): several ants in one box, each with a different brain, to score more candidates per run.** Not started; it is moderate code, so it was left for later.
   - **Cost, measured** (one timing, 12 sealed worlds, `threads=1`, s134's brain): 1 ant per box 11.3 s, 4 ants per box 18.8 s. So 4 ants cost 1.7x one ant, not 4x, and 4 brains per box would score 4 candidates for 1.7x the time: about **2.4x the throughput**.
   - **Crowding, measured** (same run, 4 copies of s134): cuts per world 23.5 against 10.3, but full loads per world 2.67 against 3.0. They cut more but carry no more out. The likely reason (not traced) is that the shaft is one cell wide and `foot=1` pays only foot cuts, so they queue there.
   - **What it needs before training:** (1) score each ant on its own trips, not the box total (check how the soil score credits trips with more than one ant); (2) the trainer groups candidates into boxes and reshuffles boxmates every generation, so a brain is not judged by one bad neighbour; (3) the bar (5 full loads per world) becomes per ant. It also trains ants that cope with company, which the plan needs anyway at step 9 (twelve ants).
   - Ask the owner before building it.
4. **If step 4 passes**, go on to the next step in `dig-rung-design.md`: deeper shafts (8, then 12 rows), then door, bare ground, roof, 12 ants, rooms, more chambers, and dig plus scarce food. The rule for each step is that a **hand-built digger passes it first** (positive control) and rest/blank/blind controls score about 0.
5. **Parked, not started** (from the owner-approved brain fix plan; steps 0-1 are done as PRs 721-722):
   - weigh a champion against the run's best;
   - a brain "price table", which is the owner's call;
   - searching for food on the far side;
   - the brood step;
   - cutting selection noise.

   Ask the owner before picking these up; the dig rung comes first.

## 4. How to run things

Build the trainer with `cargo build --release --example train`. A plain
`cargo build --release` does not rebuild examples. Never train locally in a
cloud container: an idle container sleeps and kills the run. Use Actions or
Google.

**Score one genome on the 12 sealed worlds** (about a minute):

```
PIXEL_PHYSICS_MUTATION=off target/release/examples/train brain=mind challenge=soil \
  step=4 shaft=4 foot=1 budget=1200 holdout=12 sealed=1 \
  start=training/starts/s111-g226.txt score=<path/to/hold-best.txt> threads=4
```

- The output has three `HOLDOUT` lines: `genome`, `start` and `blinded` (senses cut).
- Each line is a name list followed by a value list, in the order of the soil arena's `parts()`. The ones to read are `score`, `full_loads`, `full_cuts`, `deep_cuts`, `rim_cuts` and `s4`. Wall cuts = `deep_cuts - full_cuts`.
- The values are averaged over the worse side of each mirrored pair.
- `noise=1` instead prints one `NOISE` row per world for the `score=` genome alone, all parts in `parts()` order (`full_loads` is the 25th). The bar's "5 full loads on 11 of 12 worlds" is read from those rows.
- Every argument is echoed, and an unknown one stops the run.

**A GIF of one genome against its start.**
1. Apply the frame-dump patch to a scratch checkout, never to `main`: `Reports/brain-training/tools/gif-frames.patch`. It adds `PP_GIF_EVERY` / `PP_GIF_DIR`.
2. Build the trainer from that checkout.
3. Run it twice, once with `score=` the genome and once with `score=` the start, each into its own folder: `PP_GIF_EVERY=40 PP_GIF_DIR=<dir>/best ... sealed=1 holdout=1 pic=<dir>/best-sheet.png` and the same with `<dir>/start`.
4. Join the frames with `python3 Reports/brain-training/tools/mkgif.py <dir> <dir>/out.gif 80 50 96 36 6 start "start" best "trained"`. The numbers are the crop box x, y, w, h and the zoom. The script needs Pillow.

The previous result looked like `Reports/brain-training/stage2/dig/s111-gen126-vs-start.gif`.

**Train** (the arguments every s111/4b run used):

```
brain=mind challenge=suite arenas=soil,fetch,trail keep=fetch,trail forget=0.8 dist=10,30 \
step=4 shaft=4 foot=1 budget=1200 prune=0.3 wire=0.001 grow=0.1 seed=131 pop=100 gens=100 \
layouts=16 pm=0.03 sigma=0.3 cross=0.3 holdout=12 start=training/starts/s111-g226.txt
```

- The suite trains digging while **keeping** fetch and trail. Each kept arena is a pass/fail floor at 80% of the start's level, so a dig gain that forgets foraging is refused.
- `wire=` is the brain's energy cost per connection. The owner wants it kept.
- The auto-stop is `patience=` and defaults to **50**, by the owner's choice; leave it there. A run stops once its hold-out has gone 50 generations with no 2% gain, never before generation 40, and only once the flat stretch is at least as long as the climb.
- Each run writes these files:
  - `hold-best.txt`: the champion to start the next stage from;
  - `best.txt`: the last generation's champion;
  - `progress.csv` and `holdout.csv`;
  - `curves.png`;
  - `state.txt`: the file exact resume needs.

## 5. Compute

**GitHub Actions (free, needs nothing but the repo).** Full instructions are in
`.github/train-jobs/README.md`. In short:
- Commit one-line `<name>.job` files to a `train/<anything>` branch and push. Don't use a `claude/**` branch: that also starts the full CI.
- Pin `ref=<commit sha>` to the trainer build the run should use.
- The `start=` file must be in the pushed commit.
- Results are force-pushed to `train-results/<name>` every 10 minutes.
- Each leg is capped at 6 h. To continue, push the **same** job file again with `resume=1` added and nothing else changed. `gens=` is the total, not the extra.
- A re-push *without* `resume=1` starts over and replaces the results branch.

Tonight's four jobs are on `train/dig-4b` as examples.

**Google Cloud (paid, fast): the main compute.** The owner will give you the service-account key, and his $100 go covers runs on this ladder.
- The launcher is `Reports/brain-training/cloud-compute/gcp/gcp_train_job.py`; its header documents it.
- The key is passed as base64 in the environment variable `GCP_SA_KEY_B64`. Never put it in a file, a commit, a log or chat.
- Setup and launch:
  - install: `pip install google-cloud-compute google-auth`;
  - launch: `python3 gcp_train_job.py watch <name> "<train args>" --ref <sha> --type c4d-highcpu-16 --dest <dir>`.
- `watch` starts a spot VM, polls, restarts after a preemption from the bucket copy, and fetches the result at the end. `status`, `fetch` and `delete` also exist.
- Project facts:
  - project `pixel-physics-512114`, quota 64 spot vCPUs in us-central1;
  - results bucket `pixel-physics-train-512114`;
  - the VM deletes itself after its hours plus 1.
- Speed: a `c4d-highcpu-32` does 100 generations of this suite in about 1.5 h for about $0.65. A `-16` is about 0.6x the speed at half the price.
- Spend: the owner approved up to **$100 in total** on Google. The exact running total is on the project's billing page.

## 6. The owner's standing rulings (not written down elsewhere)

The owner is **Scott** (GitHub `sgladstein`). He uses a Mac terminal, so give zsh commands.

- **North star: a fun sim game, not accurate ant science.** Biology is inspiration. Judge by game value, and stop tuning once it works.
- **One general brain for every task**, found by selection. Start from scratch: no old hand rule is kept unless it helps. The brain's energy tax per connection stays.
- **Follow the plan**: trail → forage → dig → brood. Shut down ant work that doesn't build on it. **No lab-colony tests until an ant passes the basic behaviours.**
- **Rewards are stepped**: each step of a task is worth more than all the steps below it together. That is how forage (reach food, pick up, carry, reach the nest carrying, drop at the nest) and the dig stages are scored.
- **Pictures with every result.**
  - Every training run gets a training-curves picture with every generation from gen 0, fixed axes, and every label explained.
  - Every major step's report carries an image, chart or GIF.
  - **No "passed" until he has seen the pictures and a GIF.**
- **Scores get gamed.** Every arena keeps:
  - a survival check;
  - a second number that can't be faked;
  - a blind control;
  - hold-out worlds and a sealed set;
  - pictures.

  The cheats seen so far are in `Reports/brain-training/ladder/cheats-seen.md`.
- **Seeds**: 1-4 seeds to iterate on a dig step. 12 seeds against `main` for a game change or a rung's pass. Mutation is off when measuring (`PIXEL_PHYSICS_MUTATION=off`); only the training rig mutates.
- **Features ship on by default** when they have no measured downside. Tell him the numbers.
- **Usage is tight.** Stay on the critical path, no side probes, and report at milestones rather than constantly.
- **Spend**: Google up to $100 in total, as above. Any other paid service needs his explicit go.
- **Questions**: ask so they can be answered in one word, give your recommendation with pros and cons, and give ETAs from measured pace. **He dislikes repeats**, so check `Reports/dead-ends.md` (grep the mechanism) and these notes before proposing anything. **Give the real reason**; he spots circular reasoning.
- **Writing to him**: lead with what it does and where it is going, in the world's words (ants, soil, shaft) rather than code's, then the mechanism. Keep it short.
- **Big redesigns** need a design doc and an adversarial review before building.

## 7. Repo and PR state (2026-10-20 02:40 UTC)

- `main` is at `870e0a31f` (PR 734: this handoff, the copied notes and step 4b's should-fixes, merged 01:23). Step 4b itself is PR 733.
- PR 735 (round-1 results and this handoff's later log) is open on `claude/project-thread-eewfjj`; it is docs and data only, so it can merge whenever CI is green. Later handoff updates go on the same branch: if 735 has merged, restart the branch from `origin/main` first and open a new PR.
- Open PRs that are **Scott's call** and not part of this work: 705, 706, 702, 697, 686.
- Merging:
  - `CLAUDE.md` lets an independent session merge its own PR on green CI. Ask Scott once whether he wants you to.
  - Never `git add -A`; stage explicit paths.
  - Before pushing trainer code, run:
    - `cargo test --release --lib train::`;
    - `cargo clippy --all-targets --release --locked -- -D warnings`;
    - `PYTHONWARNINGS=ignore bash scripts/docscheck.sh`.
  - CI runs the rest.
- **Game switches**: the shipped game's ant is frozen (bug and world fixes only). The brain work happens in the training rig and does not change the game. Grass dying of old age shipped in PR 727 (half-life 30,000 frames), so game numbers taken before it may differ.

## 8. Where things are

| What | Where |
|---|---|
| The brain | `src/sim/creature/mind.rs` (`mind::preset`, `enter_rung`, `zero_dead`) |
| Trainer core (selection, hold-out, auto-stop, resume) | `src/train/mod.rs` (`Settings`, `flat`, `STOP_MIN`, `STOP_GAIN`) |
| Arenas and scores (soil/dig, fetch, trail, scarce, suite) | `src/train/challenges.rs` (`Soil`, `soil_trip_at`, `Suite`) |
| Command line, every argument documented | `examples/train.rs` header |
| What every argument and printed number means | `Reports/instruments.md`, the `train` row |
| Brain design of record | `Reports/brain-training/design/general-brain-design-rev3.md` |
| Dig rung design (rev 2.9, step 4b adopted) | `Reports/brain-training/ladder/dig-rung-design.md` |
| Ladder research index, cheats seen | `Reports/brain-training/ladder/README.md`, `cheats-seen.md` |
| Dig results history (steps 1-4) | `Reports/brain-training/stage2/dig/README.md` |
| s111 run bundles (state, csvs, logs, hold-best) for gens 70-150 and 150-250 | `Reports/brain-training/stage2/dig/runs/*.tar.gz` |
| s111 curves, gens 0-250 | `Reports/brain-training/stage2/dig/s111-gens0-250-curves.png` |
| Start genomes | `training/starts/` (`fretrain-s91`, `s111-g126`, `s111-g226`) |
| GIF tool | `Reports/brain-training/tools/` |
| Google launcher and timing notes | `Reports/brain-training/cloud-compute/gcp/` |
| Earlier Actions results | `train-results/dig-sq-s111` ... `s114` and other `train-results/*` branches |

## 9. Gotchas that have cost time

- **The printed numbers are the worse side of each mirrored pair.** A change in score can move which world is the worse one, so a sub-number like full loads can shift for that reason alone.
- **Pooled shares are not the printed ones.** Compute `full_share` as mean(`full_loads`) / mean(`full_cuts`); the printed one is a per-world average.
- **Resumes.**
  - A resume needs the same build and the same arena arguments, or it is refused.
  - `foot=1` is part of the arena string, but only when it is on, so rev 2.8 runs still resume.
  - Pin `ref=` to a sha for anything you might resume.
- **Arena setup.** `foot=1` needs `step=4`. A shaft up to 12 rows is clean on every generated bed; stone first bites at 16 rows.
- **Stale binaries.** `cargo build --release` leaves the example binary stale; build `--example train`. The tell is identical output across a change.
- **Piping cargo hides its exit code.** Use `set -o pipefail` when piping `cargo` into `tail` or `grep`.
- **Waiting on processes.** Never wait with `pgrep -f`, because it matches its own shell; use `pgrep -x`.

---

## Log

Newest first. Each entry says what changed and what is now true.

- **2026-10-20 04:10 UTC.**
  - Round 3 (`dig-4b3-s151` ... `s154`) confirmed started at 04:07 UTC on Google. Round 2's four VMs are deleted, and their bundles are in `Reports/brain-training/stage2/dig/4b2/`.

- **2026-10-20 04:05 UTC.**
  - **Round 2 gave no gain.** After about 50 generations, the best hold-out champions were s141 1.515 (gen 39), s142 1.546 (gen 2), s143 1.588 (gen 46), s144 1.505 (gen 11), all below the start s134 at 1.629 (training hold-out, combined score). s142 auto-stopped at generation 53 with best at gen 2. The other three were asked stopped at about gen 50 to free the CPUs.
  - **The reading** (inferred from the numbers, not traced): mutations of this size break a brain this well tuned. Generation 0's champion scored 0.94 on hold-out against the start's 1.63. In round 1, from s111 gen 226, the same first champion matched its start (0.93 against 0.90), and that round climbed.
  - **Round 3 launched** with smaller mutations from the same start: section 2 has the names and settings. If neither setting's champions pass 1.63 by about generation 50, mutation size is not the blocker either. The next candidates are then more worlds per brain test (selection noise) and several brains per box (item 3b, for the throughput to afford that).

- **2026-10-20 02:50 UTC.**
  - The owner asked about training with several ants in one box, each with a different brain, to score more candidates per run. Timed it: a 4-ant box costs 1.7x a 1-ant box, so about 2.4x the throughput, but it needs per-ant scoring and boxmate reshuffling first. Details and numbers are item 3b of section 3. Nothing started.

- **2026-10-20 02:40 UTC.**
  - **Round 2 confirmed running.** All four VMs (`dig-4b2-s141` ... `s144`) booted and began building at 02:30 UTC. Section 2 now describes round 2; round 1's results are in the 02:30 entry below.

- **2026-10-20 02:35 UTC.**
  - **s132 finished** (02:27). On the sealed worlds it scored worse than its start: soil 19.7 against 22.5, full loads per world 1.92 against 2.83. So the round's only gain is **s134**.
  - **Next round launched from s134's best** (`training/starts/s134-4b-g34.txt`): seeds 141-144 on Google, `c4d-highcpu-16`, with the same arguments as the 4b round otherwise (section 4's train line with `seed=` and `start=` changed, `gens=100`). Names `dig-4b2-s141` ... `dig-4b2-s144`. Expected about 3 h; the auto-stop may end some sooner.
  - It is the same move that found s134: new seeds from the current best.
  - When they finish, score each hold-best with the command in the 01:45 entry and compare it with s134 (3.67 full loads per world, 3 of 12 worlds at 5 or more). The bar is still 5 on 11 of 12.

- **2026-10-20 02:30 UTC.**
  - **s133 and s134 finished.** Both were stopped flat by the auto-stop: s133 at generation 79, best at 28; s134 at generation 85, best at 34.
  - Sealed-world scores under `foot=1`, against s111 g226 (soil 22.5, full loads per world averaged over all 12 worlds 2.83, 2 of 12 worlds at 5 or more):

    | seed | soil | full loads, mean / worse side | worlds at 5+ | blinded soil / full loads |
    |---|---|---|---|---|
    | s131 | 25.3 | 2.58 / 2.00 | 1 of 12 | 9.0 / 0.67 |
    | s133 | 27.7 | 3.08 / 2.00 | 2 of 12 | 8.0 / 0.50 |
    | **s134** | **38.0** | **3.67 / 3.00** | **3 of 12** | 3.5 / 0.33 |

  - **s134 is the first real step since s111.** It is past the agreed "3 full loads by generation 100" mark, so the pay rule was part of what held s111 back.
  - It is still short of the bar of 5 full loads on 11 of 12 worlds. Its blind control is low, so the gain comes from what it senses.
  - Its hold-best is the start for the next leg: `Reports/brain-training/stage2/dig/4b/dig-4b-s134-hold-best.txt`.
  - GIF of s134 against its start on hold-out world 0: `Reports/brain-training/stage2/dig/4b/s134-vs-start.gif`, made with the GIF tool from `main` plus the frame patch. By the end of the run, s134 has cut at the foot of the shaft and widened its floor, and put 12 pellets down away from the hole against the start's 7.
  - s132 is still running.

- **2026-10-20 01:45 UTC.**
  - **s131 finished.** The auto-stop ended it at generation 63, flat since its best at generation 12.
  - Its hold-best, scored on the 12 sealed worlds under `foot=1` against s111 g226:
    - soil 25.3 against 22.5;
    - full loads a world, averaged over all 12 worlds: 2.58 against g226's 2.83;
    - worlds at 5 or more full loads: 1 of 12, against 2 of 12;
    - wall cuts 5.0 against 4.5 (worse side);
    - blinded 9.0.
  - So step 4b has not moved s131 so far.
  - Files are in `Reports/brain-training/stage2/dig/4b/`: the bundle, hold-best and curves.
  - s132-s134 are still running: their best hold-outs so far are 1.17-1.22 times the start, set at generations 28-45.
  - To score any hold-best the same way: `PIXEL_PHYSICS_MUTATION=off target/release/examples/train brain=mind challenge=soil step=4 shaft=4 foot=1 budget=1200 holdout=12 sealed=1 start=training/starts/s111-g226.txt score=<file> threads=4`. Add `noise=1` for the per-world rows.

- **2026-10-20 01:15 UTC.**
  - The coordinator's review should-fixes for PR 733 are done in this PR.
  - `foot=1` is now refused before anything is echoed, both without `step=4` and outside a soil arena.
  - The doc notes that at `shaft=0`, `foot=1` behaves exactly as rev 2.8.
- **2026-10-20 01:05 UTC.**
  - The Actions copies of the 4b runs were cancelled ("Don't double up... Just pick one"). Google is the only copy.
- **2026-10-20 01:00 UTC.**
  - The owner confirmed the next agent will have the Google key, so Google is the main compute and the Actions copies are a backup.
  - The 4b runs are at about generation 28, with early hold-out numbers in section 2.
  - Expected finish is about 03:00 UTC.
- **2026-10-20 00:40 UTC.**
  - Handoff written.
  - Step 4b merged to `main` (PR 733); this handoff is in its own PR.
  - Four 4b seeds are running on Google and as Actions copies (`train/dig-4b`).
  - The off-repo notes, pictures, s111 run bundles, GIF tool and Google launcher have been copied into the repo.
  - Next update: when the runs report.
