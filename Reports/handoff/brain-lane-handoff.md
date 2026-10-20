# Handoff: training the ant's brain (stage 2), for an agent with only this repo

**Last updated: 2026-10-20 19:15 UTC.** Written by the session that has run
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
| Dig | **Step 4 passed** (owner, 2026-10-20, on s161 gen 336: numbers and GIFs in log 19:15). Step 4 is "carry soil up and out of the shaft"; steps 1-3 (cut, let go, put down away from the hole) are behind it. **Next step (8-row shaft) waits on the investigation in section 3.** |
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

**Round 1 of step 4b is done** (s131-s134, from s111 gen 226, `foot=1`). **s134** scored best: 3.67 full loads per world on the sealed worlds against its start's 2.83, 3 of 12 worlds at 5 or more. **But that gain is within the noise** (log 04:40): per world it beat its start on 6 worlds, tied 3 and lost 3, a mean +0.83 with a paired standard error of 0.58. It was also the best of four seeds picked on those same worlds. The other three scored 1.92-3.08 around the start's 2.83. Treat s134 as about as good as s111 gen 226, not proven better. Full table in the Log (02:30 entry). Files: `Reports/brain-training/stage2/dig/4b/`.

**Round 2 is done: no gain** (s141-s144, from s134, same settings as round 1). After about 50 generations no seed's hold-out champion beat the start: best 1.52 / 1.55 / 1.59 / 1.51 against s134's 1.63 (training hold-out, combined score). s142 auto-stopped at generation 53; s141, s143 and s144 were stopped at generations 53, 55 and 51 to free the CPUs. Log entry 04:05 had a reading that 04:40 overturned. Run bundles (no hold-best beat the start, so none is worth resuming): `Reports/brain-training/stage2/dig/4b2/`.

**Round 3 was stopped early** (s151-s154, from s134 with smaller mutations: `sigma=0.1` or `pm=0.01`). By generation 8 its first champions were as far below the start as round 2's: 0.81-1.17 on hold-out against 1.63. So mutation size is not what holds training back. See log 04:40 for the reading, which is **noise**.

**Round 4 is done: a small gain, not yet proven** (s161, s162, from s134, `layouts=48 holdout=24`, two `c4d-highcpu-32`, 100 generations, about 3.9 h each). Training hold-out best rose from the start's 1.08 to 1.59 (s161, gen 91) and 1.58 (s162, gen 97). Both were still climbing at the end. Files: `Reports/brain-training/stage2/dig/4b4/`; s161's hold-best is also `training/starts/s161-4b4-g91.txt`.

**On 24 sealed worlds** (`holdout=24 sealed=1`, `foot=1`; full loads per world, mean over both mirror sides; log 08:50 has the per-world lists):

| brain | full loads / world | worlds at 5+ | vs s111 gen 226, paired |
|---|---|---|---|
| s161 round 4 (`training/starts/s161-4b4-g91.txt`) | **4.00** | 7 of 24 | +0.50, se 0.54 |
| s162 round 4 | 3.92 | 9 of 24 | +0.42, se 0.53 |
| s111 gen 226 (`training/starts/s111-g226.txt`) | 3.50 | 9 of 24 | - |
| s134 (`training/starts/s134-4b-g34.txt`) | 3.33 | 5 of 24 | -0.17, se 0.47 |

- Both round-4 brains lead the old best by about half a load a world. Two independent seeds agree in sign, but each gap is within one standard error. The bar is 5 a world on 11 of 12 worlds, so it is still far off.
- s134 is no better than s111 gen 226 on 24 worlds. On the 12 worlds it was not picked on, it is worse (-1.17). The 04:40 noise reading was right.

**Round 4 continued to generation 340: done; step 4 passed.** `dig-4b4-s16X-long` (to gen 200) and `dig-4b4-s16X-g340` (to gen 340), exact resumes on ref `d4c2275`. On 24 sealed worlds, full loads per world: **s161 gen 336 7.21** (20 of 24 at 5+), s162 gen 338 6.21. On the bar's 12: s161 gen 336 has 11 of 12 at 5+, pooled share 0.89, blinded 1.00. Brains: `Reports/brain-training/stage2/dig/4b4/dig-4b4-s16X-g340-hold-best.txt`. The whole lineage's curves: `Reports/brain-training/stage2/dig/dig-lineage-s91-to-s161-s162-curves.png`. GIFs: `4b4/s161-g336-vs-s134-sealed{0,1}.gif`.
- **Run each job in one place only** (the owner, 00:53: "Don't double up that is a waste of work. Just pick one"). The stale `train-results/dig-4b-s13X` branches on GitHub are from a cancelled Actions copy; ignore them.

**Stop rule, agreed in advance.** If no seed is past **3 full loads a world by
generation 100**, the pay rule is not what holds it back. More generations won't
fix it; the rung needs a design change. Take that to the owner.

## 3. What to do next, in order

1. **The investigation the owner approved (2026-10-20), before the 8-row step.** Plan: `/root/.claude/plans/first-write-up-a-precious-pizza.md` in the session that wrote it; its parts, so a new session can carry it on:
   - **Part 1, where loads are lost.** Done so far: 8.0 foot cuts a world become 7.21 full loads (90%), so carrying is solved and *how many foot cuts* is the limit; about 35% of cuts are wasted (3.2 wall + 1.7 top-row a world). Run length (s161 g336, 24 sealed): 4.2 / 7.2 / 12.3 / 17.0 full loads at 3k / 6k / 12k / 24k frames, per-world sd/mean 0.40 / 0.31 / 0.27 / 0.28, so more worlds beats longer runs for ranking at 4 rows. Next: trace individual wasted cuts and lost pellets (the `funnel` skill), world 0 (4 loads) against world 1 (8).
   - **Part 2, what the brain attends to:** a static connection map from the genome, then the exact job-score contributions (`w * x` before `squash`) on logged decisions, then switching off only what those flag, with intact, blind and `rest` controls.
   - **Part 3, why fetch wears down:** fetch's stages, s91 against s161, and whether digging and fetching share senses or the Drop verb.
   - **Part 4, selection reliability:** about 40 brains from s161's final population on two independent 48-world sets; how many worlds a test needs; the run-length test again at `shaft=8`.
   - **Part 5, training efficiency:** literature (Consensus / SciSpace connectors) and cheap offline tests -- racing / successive halving, ES or CMA-ES on the same genome, a hand-built `digger` as teacher, a smoother second metric, separate shafts per ant, and RL and NEAT reopened (owner, 2026-10-20: earlier rejections may be revisited on the evidence; a big redesign still needs a design doc and an adversarial review).
   - Deliverable: `Reports/brain-training/investigation-2026-10-20.md` with a picture per part and a recommended setup; then the owner decides what is next.
2. **From the next training run on, pass `keepref=training/starts/fretrain-s91.txt`** (section 4). s161 gen 336 is already slightly under a 0.8-of-s91 fetch floor, so its first generations will be gated until fetch recovers; the pre-flight line says so at launch.
3. **Several brains per box sharing one shaft does not work** (tested 2026-10-20, log 13:30): the shaft caps the box at one ant's output and boxed scores rank no better than chance. Separate shafts per ant get only a timing check in part 5.
4. **After the investigation**, the next step in `dig-rung-design.md`: deeper shafts (8, then 12 rows), then door, bare ground, roof, 12 ants, rooms, more chambers, and dig plus scarce food. The rule for each step is that a **hand-built digger passes it first** (positive control) and rest/blank/blind controls score about 0.
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

- Round 4 on used `layouts=48 holdout=24` (and two `c4d-highcpu-32`); the gain came only then. **From the next run on, add `keepref=training/starts/fretrain-s91.txt`.**
- The suite trains digging while **keeping** fetch and trail. Each kept arena is a pass/fail floor at 80% of its reference, so a dig gain that forgets foraging is refused. The reference is the start unless `keepref=` names a fixed brain; relative to each run's start, a chain of runs gave up 17-25% of fetch (log 19:15), so use `keepref=`.
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
- **Rulings of 2026-10-20** (owner, in the session that took the lane over):
  - **Dig step 4: pass** (s161 gen 336, after the GIFs).
  - **Kept-skill floors against a fixed brain: yes** -- `keepref=` (s91) from the next run on.
  - **Several brains in one box sharing a shaft: no** (it ranks brains no better than chance); separate shafts only as a timing check.
  - **Earlier rejections may be revisited on the evidence**: "you are allowed to revisit any ideas that seem promising regardless of previous rejections" (RL, NEAT and big-network ES were rejected by reasoning in the design of record, never by test).
  - **Protect connections the learning arena cannot test: yes** -- do not prune or mutate links that serve only verbs the arena never offers (the deletion that cost s161 its "don't eat the cargo" link; investigation part 3).
  - **Teaching by copying: test both label sources, side by side** -- (A) self-imitation: the brain's own decisions from trips that became full loads, filtered by the scorer; (B) a hand-written teacher, time-boxed to one day of building (the owner's caution: weeks of hand-encoded behaviour failed before training replaced it, so B's difficulty is part of what is measured). Same copying code for both; offline first (24 sealed worlds at 4 and 8 rows, against s161, kept skills against `keepref=s91`), then a short 8-row training run from each survivor beside plain s161, with GIFs for the owner.
  - **Reinforcement learning: yes, but after teaching-by-copying, not before** -- clone a hand-built digger first (it builds the gradient-through-the-brain code and has the stronger evidence), then test RL head-to-head against selection on one dig step, with a design doc first.

## 7. Repo and PR state (2026-10-20 19:15 UTC)

- `main` is at `918721b` (PR 740). This session's work is on `claude/brain-lane-r4-long` (round-4 continuation data, gen-340 brains, the lineage curves, GIFs, `keepref=`, this handoff), in its own PR; the old `claude/project-thread-eewfjj` belongs to the session before.
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
- **`top`, not `best_hold`, is the hold-best's generation.** `state.txt` carries both: `best_hold` is the auto-stop's clock (moves only on a 2% gain), `top` is the highest hold-out score, the one written to `hold-best.txt` and stored as the `genome top` block (weights as hex bits). A run cut short has no `hold-best.txt`, but its `genome top` block is the same brain (checked on s162: identical).
- **Delete a VM only after its `DONE` marker is confirmed**, never in the same command as a fetch: s161's gen-340 run lost its last 3 generations that way. s162's VM was still RUNNING about 20 minutes after `@@DONE`, so the post-DONE shutdown is not reliable either: check `status` and `delete` once the result is fetched.
- **Stale binaries.** `cargo build --release` leaves the example binary stale; build `--example train`. The tell is identical output across a change.
- **Piping cargo hides its exit code.** Use `set -o pipefail` when piping `cargo` into `tail` or `grep`.
- **Waiting on processes.** Never wait with `pgrep -f`, because it matches its own shell; use `pgrep -x`.

---

## Log

Newest first. Each entry says what changed and what is now true.

- **2026-10-20 20:30 UTC. Dig step 5 (8-row shaft) training started.**
  - `dig-8-s171`, `dig-8-s172` on Google (`c4d-highcpu-32`, `--hours 8`, launched with `start`, so a spot preemption is not restarted), ref `48c67a0` (has `keepref=`), from s161 gen 336 (`Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt`). Arguments as round 4 but `shaft=8 frames=12000 budget=2400 keepref=training/starts/fretrain-s91.txt gens=100`. About 4 min a generation (estimate), due about 03:30 UTC.
  - **Why these settings:** investigation part 4 -- at 8 rows a 12,000-frame run separates brains better per CPU than more worlds; the dig score ranks better than full loads; the worse-side fold costs nothing at 12,000 frames.
  - **The 8-row bar (owner, 2026-10-20): a pace** -- 5 full loads per 6,000 frames, so **10 per 12,000, on 11 of 12 sealed worlds**, plus the step-4 share and blind conditions. Untrained s161 reads 6.6 per 12,000 (3.7 per 6,000; 24 sealed worlds), so it starts well short.
  - **Not in these runs:** the approved protection of untestable links (not built yet). Watch fetch and trail against the `keepref=` floor in `holdout.csv`.
  - Score each hold-best with `holdout=24 sealed=1 shaft=8 frames=12000 budget=2400 noise=1`, paired per world against s161 gen 336 at the same settings.
- **2026-10-20 19:15 UTC. Step 4 passed; investigation started.**
  - **Gen 340** (exact resumes from gen 200): s162 finished at 18:10; s161 was deleted at gen 337 of 340 by mistake (fetch and delete in one step) and its gen-336 `top` brain recovered from `state.txt`. 24 sealed worlds, full loads per world: s161 g336 **7.21** (20/24 at 5+; +1.83 over its g199, se 0.44), s162 g338 6.21 (+0.50, se 0.63). On the bar's 12 sealed worlds s161 g336: **11 of 12 at 5+, pooled share 0.89, blinded 1.00**. The owner saw the GIFs (sealed worlds 0 and 1, chosen before looking) and ruled **pass**.
  - **Kept skills eroded more than the 12-world check showed.** On 24 sealed worlds, as a share of s91: fetch s134 0.76, s161 g336 0.75, s162 g338 0.83 (low 0.65 at s162 g190); trail 0.83-0.97 along the line, 0.83 / 0.87 at the newest. Real foraging (`challenge=scarce`, food 80 out) delivers nothing for any brain, s91 included. A first lineage plot chained each run's hold-out ratio back to s91 and read 0.6 for fetch; one 24-world set flips the order of s162 g338 and s134 on fetch, so a four-link chain compounds that bias and was dropped.
  - **`keepref=` added** (`a2b035a`): kept-arena floors measured against a fixed brain. Positive control on s162 g338: references move to s91's (fetch 19.15 -> 23.15, trail 0.70 -> 0.80; soil unchanged); pre-flight reads fetch 19.15 vs floor 18.52, trail 0.70 vs 0.64. Tests 56/56, clippy clean.
  - **Investigation approved** (section 3, item 1); part 1's first results are there.
- **2026-10-20 13:30 UTC. Taken over by a new session. Round 4's continuation is a real gain; continued again to generation 340.**
  - `dig-4b4-s161-long` and `s162-long` both ran to generation 200 (exit 0, 12:22 and 12:19 UTC; VMs gone). Hold-out best: s161 1.83 at gen 199, s162 2.23 at gen 190, against the start's 1.08. Bundles, hold-bests and curves: `Reports/brain-training/stage2/dig/4b4/dig-4b4-s16X-long-*`.
  - **Sealed, 24 worlds, `foot=1`, full loads per world** (the s111 g226 and s161 g91 lists were re-scored in this session and match the 08:50 entry exactly):
    - s161 gen 199: mean **5.38**, 17 of 24 worlds at 5+ [6,8,5,4,6,2,5,6,7,7,5,4,5,6,2,2,6,6,6,4,4,5,9,9]; blinded 0.75 (worse side);
    - s162 gen 190: mean **5.71**, 18 of 24 at 5+ [5,5,8,3,1,6,5,3,10,10,7,3,3,5,5,5,9,9,7,6,7,6,2,7]; blinded 0.33;
    - s162 gen 97 (round 4's end) 3.92, 9 of 24; s161 gen 91 4.00, 7 of 24; s111 g226 3.50, 9 of 24.
  - Paired per world: s161 g199 - g91 **+1.38** (se 0.52); s162 g190 - g97 **+1.79** (se 0.72); pooled +1.58 (se 0.44, n 48). Against s111 g226: +1.88 (se 0.52) and +2.21 (se 0.72). Both seeds moved the same way and past the ~1 load/world bar in section 2, so **this is the first gain on the dig rung that clears the noise.** Pooled full_share 0.87. Not passed: the bar is 5+ on 11 of 12 worlds, about 22 of 24 here.
  - **Continued** as exact resumes to `gens=340`: `dig-4b4-s161-g340`, `dig-4b4-s162-g340`, ref `d4c2275`, two `c4d-highcpu-32`, `--hours 6`, started 13:14 UTC, both logged "resumed at generation 200". Launched with `start`, not `watch`, so **a spot preemption is not restarted**: if `status` says gone with no `DONE`, resume from the bucket copy (`fetch`, then `start` a new name with `--seed`). Due about 18:30 UTC unless the auto-stop ends one sooner.
  - **Item 3b (several brains per box) does not work as specified.** Tested 2026-10-20 03:15 on 40 brains (harness on branch `scratch/boxrank-test`, not for main): four different brains sharing the one-cell shaft carry 0.9 full loads each, against 2.6 alone, so the box carries what one ant does. A brain's boxed score ranks brains no better than chance against its solo score on new worlds (Spearman -0.08 to +0.11 over the 33 round-2 brains, against +0.29 solo vs solo). Until then the soil arena gave **every** ant `genomes[0]`, so the 02:50 timing was four clones. Separate shafts per ant would be the variant to try, at step 9 (twelve ants) at the earliest.

- **2026-10-20 08:50 UTC. Round 4 done: a small, unproven gain; continued to generation 200.**
  - Both runs went 100 of 100 generations. s161 took 14,328 s, hold-out best 1.590 at gen 91. s162 took 13,335 s, best 1.581 at gen 97. The start scored 1.08 and blinded about 0.03. The fetch floor vetoed 11 and 17 of 100 champions; the trail floor vetoed none. CPU was 98% busy on 32 vCPUs, so 48 layouts keep every core working.
  - **Scored on 24 sealed worlds**, full loads per world, both mirror sides:
    - s161 [4,1,3,2,3,2,5,2,4,4,4,5,7,3,3,3,7,7,4,4,6,6,3,4], mean 4.00; blinded soil 12.2 / full loads 0.92 (worse side);
    - s162 [3,5,3,5,7,3,3,6,5,5,1,8,5,4,2,2,2,2,6,3,3,4,3,4], mean 3.92; blinded 8.3 / 0.67;
    - s134 [5,6,3,2,3,4,3,3,4,4,6,1,2,2,3,3,4,2,3,5,3,5,2,2], mean 3.33;
    - s111 g226 [6,2,4,2,6,2,1,3,2,2,3,1,6,6,5,5,2,2,4,6,1,3,5,5], mean 3.50.
  - Paired against s111 g226: s161 +0.50 (se 0.54, 10 wins, 3 ties, 11 losses); s162 +0.42 (se 0.53). Against s134: s161 +0.67 (se 0.47), s162 +0.58 (se 0.51).
  - **Commands**, from a release build of `examples/train.rs` with `PIXEL_PHYSICS_MUTATION=off`:
    - summary: `brain=mind challenge=soil step=4 shaft=4 foot=1 budget=1200 holdout=24 sealed=1 start=training/starts/s111-g226.txt score=<brain file>`;
    - per world: add `noise=1`. Each `NOISE seed` row is one world; full loads are the 25th number (index 24).
  - **Next:** both continued as exact resumes to generation 200 (section 2). If the continuation lifts them clearly (about 1 load a world over s111 gen 226 on 24 worlds), keep going. If it stays flat, the levers in the 04:40 entry are next: longer runs per world, several brains per box (item 3b), or a pay rule with more events per world.

- **2026-10-20 06:25 UTC.**
  - **Round 4 at generation 40: champions now beat the start on hold-out.** This is the first round since s134 to do so. s161 best 1.36 (gen 22) and s162 best 1.27 (gen 21), against the start's 1.08 on the same 24 layouts. Treat it as promising, not proven: it is still the best of about 40 scores on fixed layouts. The test is the hold-best on the sealed worlds, paired per world against s134 and s111 gen 226.
  - The snapshots hold no `hold-best.txt` mid-run; it is written when the run ends. Expect both done by about 08:35 UTC, or about 07:30 if the auto-stop fires 50 generations after a best at gen 22.

- **2026-10-20 04:50 UTC.**
  - **Round 4 running** (`dig-4b4-s161`, `s162`, two `c4d-highcpu-32`), 138 s a generation, due about 08:35 UTC. Round 3's VMs were stopped at generations 10-12 and deleted. Their bundles are in the bucket and the shared folder only, not the repo, since nothing in them is worth resuming.
  - **The noise reading, confirmed:** on round 4's 24 hold-out layouts the start (s134) scores **1.08**, against the 1.63 it scored on the 12 layouts it was picked on. So round 4's hold-out comparisons are fairer, but still read gains paired per world on the sealed set.

- **2026-10-20 04:40 UTC. The noise reading; round 3 stopped; round 4 launched.**
  - **Round 3's early champions** (generations 0-8, hold-out, start 1.629):
    - `sigma=0.1`: s151 best 1.24, s152 best 1.08;
    - `pm=0.01`: s153 best 1.54, s154 best 1.31.
    - Generation 0: 0.88, 0.81, 1.17, 0.95, the same as round 2's 0.83-1.17. Smaller mutations do not keep the champions near the start, so the 04:05 reading (mutations too big) was wrong.
  - **Why the start looks unbeatable:** the hold-out layouts are the same in every run (the start scores 1.629 in every seed). s134's hold-best was chosen as the best of about 85 hold-out scores on exactly those layouts. So its 1.63 is a lucky high, and near-copies of it score lower on those layouts. Comparisons against the start's hold-out are tilted toward the start.
  - **Sealed worlds** (12, `foot=1`, full loads per world):
    - s142's hold-best 1.42;
    - s134 3.67: per world [5,6,3,2,3,4,3,3,4,4,6,1];
    - s111 g226 2.83: [6,2,4,2,6,2,1,3,2,2,3,1];
    - s133 3.08, s131 2.58.
  - s134 minus s111 g226, paired per world: +0.83, standard deviation 1.99, standard error 0.58 (t 1.45), with 6 wins, 3 ties and 3 losses. **Not distinguishable from noise.**
  - Per-world standard deviation is about 1.5-2 full loads, about what you would expect from counting 3-ish rare events per world. A brain's score on 16 layouts therefore has a standard error of about 0.4-0.5 full loads. That is more than the gains selection is looking for, so **selection is mostly picking lucky brains.**
  - **What follows:**
    - Round 4 tests each brain on 48 layouts and uses 24 hold-out layouts (section 2).
    - If round 4 still doesn't move, the next levers are:
      - longer runs per world, so each world counts more loads (section 2: 4x the frames gave 1.7x the loads);
      - several brains per box (item 3b), to pay for more worlds;
      - a pay rule with more events per world.
    - All of this also applies to reading **any** result here: compare paired per world, and on more than 12 worlds when the gap is small.
  - **Correction:** the 02:30 entry and the message to the owner called s134 "the first real gain since s111". By this test it is not proven.

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
