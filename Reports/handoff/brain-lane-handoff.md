# Handoff: training the ant's brain (stage 2), for an agent with only this repo

**Last updated: 2026-10-20 01:00 UTC.** Written by the session that has run
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

**Running tonight.**

*Four new seeds, s131-s134, all started from s111's gen-226 best with `foot=1`, 100 generations each.*

- **Google Cloud: the main copy.** The four VMs started 00:03-00:05 UTC and run at about 1.8 minutes a generation, so they should finish about 03:00 UTC unless the auto-stop ends one sooner.
  - Each VM writes a snapshot to the bucket every 5 minutes, as `runs/dig-4b-s13X/latest.tgz`, and a `DONE` marker at the end. The VMs need no session to keep running.
  - Fetch with the launcher (section 5): `python3 Reports/brain-training/cloud-compute/gcp/gcp_train_job.py fetch dig-4b-s131 <dir>`. That saves `<dir>/dig-4b-s131-snap.tar.gz` holding `hold-best.txt`, `best.txt`, `progress.csv`, `holdout.csv`, `train.log`, `curves.svg` and `state.txt`.
  - Then `status dig-4b-s131` should say the VM is gone. If it doesn't, run `delete dig-4b-s131`.
  - **Early hold-out numbers.** These come from the training hold-out (12 layouts, not the sealed set), on the soil score under `foot=1`, where the start scores 38.2. At about generation 28: s131 best 51.7 (gen 12), s132 49.2 (gen 19), s133 43.8 (gen 20), s134 46.8 (gen 18). None of this is a pass reading yet.
- **GitHub Actions: a free backup copy.** The same four runs, from branch `train/dig-4b`, land in the repo by themselves.
  - Results: branch `train-results/dig-4b-s13X`.
  - One leg is capped at 6 h, which is roughly 24-50 generations. To carry on, push the same job file again with `resume=1` added (section 5).
  - Results are identical at any thread count, so the Actions copy reproduces the Google run generation for generation.

**Stop rule, agreed in advance.** If no seed is past **3 full loads a world by
generation 100**, the pay rule is not what holds it back. More generations won't
fix it; the rung needs a design change. Take that to the owner.

## 3. What to do next, in order

1. **When the 4b runs finish**, take each run's `hold-best.txt`. Score the best of them on the 12 sealed worlds with `foot=1`, against `training/starts/s111-g226.txt` (command in section 4). Read `full_loads` against the bar, then the blinded row.
2. **Make a before/after GIF** (section 4) and a training-curves picture (`curves.png` comes with every run). Send both to the owner with the numbers. **Never call a rung passed before the owner has seen the pictures and the GIF.**
3. **If no seed passes but some climb**, resume the best one (exact resume, section 5) rather than starting fresh. **If none is past 3 full loads by gen 100**, stop and bring the owner a design question with your recommendation.
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

## 7. Repo and PR state (2026-10-20 00:40 UTC)

- `main` is at `7f39dcfda` (PR 733, step 4b, merged 00:23).
- The handoff and the copied notes ride on `claude/project-thread-eewfjj` in their own PR. Scott can merge it whenever CI is green.
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
