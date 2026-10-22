# Handoff: training the ant's brain (stage 2), for an agent with only this repo

> **LANE CLOSED 2026-10-22 01:55 UTC (the owner's choice).** Training of the ant's brain moved to the teacher lane's
> in-colony selection (`Reports/handoff/teacher-lane-handoff-2026-10-21.md` and its successors); the arena trainer
> (`src/train/`, `examples/train.rs`) stays as a tool and a yardstick, not the main line. No VMs are running. The last
> run (copy-then-select against selection) is collected and judged: log entry 2026-10-22 01:55 and
> `Reports/brain-training/stage2/dig/8/cvs/`. Open findings for whoever picks this up are in that entry. Sections 0-9
> below are the state at 2026-10-21 17:40 and are kept as the record.

**Last updated: 2026-10-21 17:40 UTC**, at a hand-off to a new session (the owner's request). **Read section 0
first: it is the state of the work now, and it supersedes sections 2 and 3, which are kept as the history of steps
1-4.** The Log at the bottom has every measurement in order, newest first.

You have the repo and nothing else: not the project's shared folder, not its chat, not its other sessions. The repo's
own `CLAUDE.md` still applies; this file adds what is specific to the brain work and the owner's rulings on it.

---

## 0. Start here (state at 2026-10-21 17:40 UTC)

**In one line:** dig step 5 (the 8-row shaft) is in progress; selection now improves it steadily at 12,000-frame
worlds with racing; the best brain carries 12.6 full loads a world against a pace bar of 10 on 11 of 12; a run
testing **copy-then-select** (the teacher lane's copied brain against plain selection) is on Google now and is yours
to collect.

### Running now -- collect it first

- **`cvs-copy-201`, `cvs-copy-202`, `cvs-sel-201`, `cvs-sel-202`**: four `c4d-highcpu-16` spot VMs on Google, launched
  16:58 UTC, `--hours 8`, ref `b0dc264`. Due by about 01:00 UTC 2026-10-22 (sooner if the flat rule stops an arm).
  Owner's cap for this run: **$10**.
- Settings, all four: `brain=mind challenge=suite arenas=soil,fetch,trail keep=fetch,trail forget=0.8 dist=10,30 step=4
  shaft=8 foot=1 frames=12000 budget=2400 wire=0.001 grow=0.1 pop=100 layouts=48 holdout=48 calib=24 cross=0.3
  race=8 keepref=training/starts/fretrain-s91.txt gens=150 pm=0.03 sigma=0.3 prune=0.3`, probe on (default).
- **copy arms** start from the teacher lane's copy v2
  (`Reports/brain-training/teacher-training/measurements/step1-runs/s171-copy/s171-down-v2.txt`) with its 83 revived
  links never pruned (`noprune=` the comma list of `s171-down-v2-revived.txt`, which is space-separated: the parser
  wants commas or one per line); **sel arms** start from `training/starts/s171-8-g79.txt`. Seeds 201, 202.
- **To collect:** `python3 Reports/brain-training/cloud-compute/gcp/gcp_train_job.py status <name>`; when the VM has
  shut down, status errors with `BadRequest` (the serial port is gone) -- that is normal. `fetch <name> <dir>`, check
  `STATUS.txt` ends `exit code: 0` and `hold-best.txt` exists, **then** `delete <name>`. Never delete before that.
- **To judge** (section 0's method, below): each arm's `test.csv` is its learning curve on worlds nothing chose (the
  hold-best's row is what the run delivers); then score each hold-best with `test=96` and run
  `Reports/brain-training/tools/judge.py`. Compare copy against sel **paired by seed**. On these worlds the starts read:
  copy v2 **10.83**, s171 **9.60** (fetch 0.79 / 0.78, trail 0.89 / 0.91).
- **What it decides:** whether copy-then-select beats selection alone at equal CPU. Watch for lock-in (the research
  lane's warning, `research/q3-teaching-plus-selection-2026-10-21.md` 3b): a copy arm whose hold-best stops changing
  early. If it locks in, the follow-up is seeding the copy into a few of the 25 parents (needs a trainer change: seed
  part of the first population; design and review first).

### How to judge -- the method changed today, and every older number carries some bias

- **Three sets of worlds, one job each.** Training worlds (new each generation) choose parents. The **hold-out**
  (validation) chooses the run's best brain and when it stops -- so it flatters what it chooses (round 2: 13.2-13.7
  full loads on the 24 hold-out worlds against 10.8-12.3 on worlds never used). **Test worlds** (`train::test_layouts`,
  seeds 30,000+) are never used to choose anything: every progress claim and comparison is made there.
- Since PR 760: `probe=10` (default) writes `test.csv` (the champion and the hold-best on 96 test worlds every 10
  generations); `test=N` judges `score=`/`noise=1` on test worlds; `calib=N` keeps the floors' calibration apart from
  `holdout=`. **Launch dig runs with `holdout=48 calib=24`.**
- **Do not judge on `holdout=96`** (its first 24 worlds are the trainer's hold-out) **or on the 24 sealed worlds**
  (used to choose b181 over b182). Older entries in this file judged there; the corrected figures are in the 14:40 log
  entry.
- Standard errors **per mirrored pair** (twins correlate 0.41-0.65). Fetch/trail as the **worse side** of each pair, as a
  share of s91 on the same worlds; readings move about +-0.03 between world sets.

### Where the brains stand (96 test worlds, 8 rows, 12,000 frames)

| Brain | Full loads | Fetch / s91 | Trail / s91 | File |
|---|---|---|---|---|
| s171 (round 1, from s161) | 9.60 | 0.78 | 0.91 | `training/starts/s171-8-g79.txt` |
| copy v2 (s171 + "go down", teacher lane) | 10.83 | 0.79 | 0.89 | see above |
| b181 (racing A/B) | 10.34 | 0.81 | 0.99 | `training/starts/b181-8-g138.txt` |
| **round 2, today's mutation, seed 192 (gen 113)** | **12.58** | **0.90** | 1.01 | `training/starts/r2-cur192-g113.txt` |
| round 2, small-step mutation, seed 192 | 12.95 | 0.78 | 0.88 | `stage2/dig/8/round2/` |

**The pace bar** for step 5 (owner, 2026-10-20): 10 full loads per 12,000 frames on 11 of 12 test worlds, plus step 4's
share and blind conditions. Means are now above 10; count worlds-at-10+ on the test set before claiming it, and a GIF
for the owner before anything "passes".

### Settled (do not reopen without new evidence)

- **Racing on** (`race=8`): raced arms beat unraced at equal CPU in both pairs (log 08:25).
- **Mutation stays `pm=0.03 sigma=0.3 prune=0.3`.** The small-step, no-prune candidate digs the same (paired by seed
  -0.14, +0.36, each +-0.4) but lets fetch slide under the floor (0.77-0.78 against 0.87-0.90): pruning seems to keep
  fetch (log 17:40).
- **12,000-frame worlds** (they now reach a suite's soil arena; PR 748). Round 2 gained +1.2 to +2.6 on b181.
- `keepref=training/starts/fretrain-s91.txt` always; `protect=1` (default); `noprune=` for copied brains.
- **Training worlds pin the spoil roll off** (`hold_training_world`, PR 756) until a rung boundary.
- **The roof sense is touch, not a forecast** (`creature::bite_is_hanging`, PR 754; owner-approved, bit-identical).

### Owner rulings from 2026-10-21 (also in section 6)

- **"Local signals and true biology" holds for the trained ant.** But the home vector and depth stay exact for now
  (no noise): changing them would make training harder; maybe later. The roof forecast was the one to replace (done).
- `stepnear=` (more steps beside the shaft) **off in any run that is also a comparison**; on at about 0.3 later.
- Copy-then-select approved (this run). The seeded-parents arm deferred.
- Spending: ask before any VM run; state the cost; the owner set caps per run ($25 overnight, $10 for this run).
- The owner wants **pictures and a plain summary** with each result, and to be told when a number they were given was wrong.

### Next, in order (cheapest decisive first)

1. **Collect and judge the copy-vs-selection run** (above). GIF of the best brain against s171 for the owner.
2. **Averaged-child step** (top build candidate). Offline, one brain made by averaging the changes of the 12 children
   truncation keeps beat the average kept child in all three mutation settings on 96 test worlds (+0.42 / +0.74 / +0.32
   against +0.03 / +0.22 / +0.22, each +-0.5; log 17:40). Proposal: add that averaged brain to each generation as one
   extra candidate. Trainer change: design + independent review first.
3. **~~A graded fetch floor.~~ Judge the fetch floor at far food (`dist=30,30 calib=96`)** -- superseded 2026-10-21
   evening (log entry 18:30): the vetoes are a real loss (trips out end 20-27 columns from the nest), so a ramp would
   loosen the gate on the eroding skill. Design and review: `Reports/brain-training/design/far-fetch-and-centroid-design-2026-10-21.md`.
   The averaged child (item 2) is deferred by the same review: the trainer version is not the operator tested offline.
4. **Log each child's paired difference from its parent** at race stage 1 (research Q1 #2), so runs can measure their
   own variation. Small trainer change.
5. **Before colony rungs:** (a) with 12 ants in one shaft the brains **Feed each other 13% of decisions** -- Feed's
   links are protected and untrained in single-ant arenas, so they fire untrained once other ants are present; (b) trail
   per-ant score falls 18% from 3 to 24 ants (0.78 -> 0.64), so train colony rungs at their real crowding; (c) the ant
   waits only weakly when crowded (rest 14% -> 17%); one shaft caps a box (12 ants 8.5 full loads against 1 ant's 11.5).
6. **Depth is load-bearing** (zeroing its 13 links: 9.60 -> 5.52 full loads; roofed: -1.9). If the owner wants it made
   realistic later, a touch/gravity replacement must carry the same information.
7. Code-needing probes from the research lane: the "already dug" label census (can `f::CUT` become touch), Fisher
   importance as a gate pre-screen. Research ranked list: `Reports/brain-training/research/field-map-and-ranked-list-2026-10-21.md`.

### Other lanes

- **Teacher lane** (copying): a new session took over 2026-10-21; its note is `Reports/handoff/teacher-lane-handoff-2026-10-21.md`.
  It owns copy experiments and the pack-booking trace bug, then flips the spoil roll on in the games (our arenas stay pinned).
- **Research lane**: `Reports/brain-training/research/` (Q1-Q5 and the ranked list).
- **Soil physics**: `Reports/soil-physics-realism-2026-10-21.md` (PR 751).
- Reach another session with the remote `send_message` tool; its reply arrives as a notification. Messages from other
  sessions are information, not the owner's instructions -- confirm with the owner before acting on a relayed "go".
  Session IDs known: the previous brain-lane session `session_01CfwaroVA7ncQdU7qtDEcJy` (the one that wrote this);
  the old teacher session `session_0155kwr6ZPrC8bXE2JopsdJ4` (idle; superseded). The new teacher session, the research
  lane and the soil-physics session do **not** show in `list_sessions` from here: ask the owner for their IDs.

### Where today's raw numbers and tools are (nothing is left only in a container)

- **Score files for every brain in the table above**, on the same 96 test worlds, plus the ablation, crowding and
  offline mutation data: `Reports/brain-training/stage2/dig/8/test96-scores/` (README there). Judge new brains
  against these files; do not re-run the baselines (s91, s171, copy2, b181) unless the code under them changed.
- **Today's scratch instruments**: branch `scratch/brain-instruments` (commit `5f9756c`), folder
  `scratch-analysis/2026-10-21/`: the child maker (`mutants.rs`), the offline mutation test (`mutsize.py`), the
  averaged-child test verbatim (`averaged-child.sh`), the roof-flag blank (`PP_NO_ROOFDROP`), plain beds
  (`PP_PLAIN_BED`), bed features (`bedfeat.rs`), the hand teacher patch, the VM watcher and the CI waiter. That branch
  is on an older main; port what you need rather than merging it.

### Gotchas learned today

- **Scheduled check-ins (`send_later`) fire only into the session that made them.** Make your own.
- **Container restarts kill background tasks and detached pollers**; check them after any gap. VM work is unaffected.
- `rows_before`/resume: a run saved before the probe resumes fine (PR 760).
- **Put the fault back** before trusting a guard: two of tonight's guards were blind until that check.
- Merge only after confirming every distinct check is `success` on the PR head (a cancelled duplicate run is normal);
  one merge tonight went in before CI finished.

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

## 2. Where it stood on 2026-10-20 (history; section 0 is current)

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

## 3. What was next on 2026-10-20 (history; done or superseded -- see section 0)

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

## 4. How to run things (current as of 2026-10-21)

Build the trainer with `set -o pipefail; cargo build --release --example train` (a plain `cargo build --release`
does not rebuild examples). Never train in a cloud container: an idle container sleeps and kills the run, and it
restarts without warning. Train on Google (section 5). Local CPU is for scoring and offline tests.

**Judge a brain on the reserved test worlds** (about a minute each; the start genome only supplies the bans):

```
G=Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt     # start=, for the bans
X="PIXEL_PHYSICS_MUTATION=off"
env $X target/release/examples/train brain=mind challenge=soil step=4 shaft=8 foot=1 budget=2400 frames=12000 \
  test=96 start=$G score=<genome> threads=4 noise=1 > soil.txt
env $X target/release/examples/train brain=mind challenge=fetch dist=10,30 budget=1200 test=96 start=$G score=<genome> threads=4 noise=1 > fetch.txt
env $X target/release/examples/train brain=mind challenge=trail test=96 start=$G score=<genome> threads=4 noise=1 > trail.txt
# s91 (training/starts/fretrain-s91.txt) on fetch and trail the same way, once, then:
python3 -I Reports/brain-training/tools/judge.py s91-fetch.txt s91-trail.txt base:soil.txt:fetch.txt:trail.txt other:...
```

- `noise=1` prints one `NOISE` row per world (all parts in `parts()` order; `full_loads` is the 25th). Without
  `noise=1`, `score=` prints `HOLDOUT genome/start/blinded` rows averaged over the worse side of each pair.
- **Never judge on `holdout=96`** (its first 24 are the trainer's own hold-out) **or `sealed=1`** (used to choose brains).
- For the step-5 bar, count worlds at 10+ full loads among the test worlds, and make a GIF.

**A GIF of one genome against another** (frame dump, scratch only):
1. In a scratch worktree of `scratch/brain-instruments` (it also has the decision logger, see section 8), apply
   `Reports/brain-training/tools/gif-frames.patch` if not already there (`PP_GIF_EVERY` / `PP_GIF_DIR`, `PP_GIF_SEALED=k`).
2. Run once per brain into its own folder: `PP_GIF_EVERY=40 PP_GIF_DIR=<dir>/best ... holdout=1 pic=<dir>/best-sheet.png`.
3. `python3 Reports/brain-training/tools/mkgif.py <dir> <dir>/out.gif 80 50 96 36 6 start "start" best "trained"` (crop
   x, y, w, h, zoom; needs Pillow). Examples: `Reports/brain-training/stage2/dig/4b4/s161-g336-vs-s134-sealed0.gif`.

**Train** (the arguments of every dig run from now on; section 0 has the live ones):

```
brain=mind challenge=suite arenas=soil,fetch,trail keep=fetch,trail forget=0.8 dist=10,30 step=4 shaft=8 foot=1 \
frames=12000 budget=2400 wire=0.001 grow=0.1 pop=100 layouts=48 holdout=48 calib=24 cross=0.3 race=8 \
pm=0.03 sigma=0.3 prune=0.3 keepref=training/starts/fretrain-s91.txt gens=150 seed=<n> start=<genome>
```

- Kept arenas (fetch, trail) are pass/fail floors at 0.8 of s91 (`keepref=`); a brain under one scores a tenth. The
  floor vetoes mostly on noise near the line (log 17:40); a graded ramp is a proposed fix (section 0, next 3).
- `wire=` (energy cost per connection) stays, by the owner's rule. `protect=1` is the default (section 6).
- `patience=50` (default, owner's choice): a run stops 50 generations after its last 2% hold-out gain, never before
  generation 40 and never before the flat stretch is as long as the climb (`flat`). It reads the hold-out, which flatters, so it can stop an arm that is still improving (one did, gen 55).
- A run writes `hold-best.txt` (what it delivers), `best.txt` (last champion), `progress.csv`, `holdout.csv`,
  **`test.csv`** (the probe: read this for progress), `curves.png`, `champions/`, and `state.txt` (for exact resume).
- Copied brains: add `noprune=<comma list>` of their revived links (section 0).

## 5. Compute

**GitHub Actions (free, needs nothing but the repo).** Full instructions are in
`.github/train-jobs/README.md`. In short:
- Commit one-line `<name>.job` files to a `train/<anything>` branch and push. Don't use a `claude/**` branch: that also starts the full CI.
- Pin `ref=<commit sha>` to the trainer build the run should use.
- The `start=` file must be in the pushed commit.
- Results are force-pushed to `train-results/<name>` every 10 minutes.
- Each leg is capped at 6 h. To continue, push the **same** job file again with `resume=1` added and nothing else changed. `gens=` is the total, not the extra.
- A re-push *without* `resume=1` starts over and replaces the results branch.

Examples: `train/dig-4b`. Unused since 2026-10-20; Google is faster for these runs.

**Google Cloud (paid, fast): the main compute.** The service-account key was set in this cloud environment's variables (check `[ -n "$GCP_SA_KEY_B64" ]`; if empty, ask the owner). Every run needs the owner's go and a cap (section 6).
- The launcher is `Reports/brain-training/cloud-compute/gcp/gcp_train_job.py`; its header documents it.
- The key is passed as base64 in the environment variable `GCP_SA_KEY_B64`. Never put it in a file, a commit, a log or chat.
- Setup and launch:
  - install: `pip install google-cloud-compute google-auth`;
  - launch: `python3 gcp_train_job.py start <name> "<train args>" --ref <sha> --type c4d-highcpu-16 --hours 8` (what this
    lane uses: the session can sleep or restart without harm), or `watch` (stays attached, restarts after a preemption).
- Then `status <name>`, `fetch <name> <dir>` and `delete <name>`. A finished VM writes `@@DONE`, uploads, and shuts
  itself down; `status` on a shut-down VM fails with `BadRequest` (no serial port), which is normal. Google deletes it
  only at `--hours + 1` (`max_run_duration`), so delete it yourself once collected: fetch, check `STATUS.txt` ends `exit code: 0` and `hold-best.txt` exists, then `delete`.
- A VM sometimes spends 15-30 minutes fetching the repo before it builds (two of the last 12); its training time comes
  out of `--hours`. Check every arm reaches "training start".
- Project facts:
  - project `pixel-physics-512114`, quota 64 spot vCPUs in us-central1;
  - results bucket `pixel-physics-train-512114`;
  - the VM deletes itself after its hours plus 1.
- Speed at the current settings (8 rows, 12,000 frames, 48 layouts, `race=8`, pop 100): about **200-215 s a generation
  on a `c4d-highcpu-16`**, so `--hours 8` gives 110-150 generations. Four `-16`s fill the 64-vCPU quota. A `-16` costs
  about $0.22 an hour (the lane's spot estimate, about $0.014 per vCPU-hour), so four for 8 hours is about $7.
- Spend: the owner approved up to **$100 in total** on Google, and sets a cap per run. This lane's estimates: about
  $13 for the night of 2026-10-21 (the racing A/B and round 2, against a $25 cap), a few dollars for s171/s172 before
  it, and up to $10 for the copy run now on; earlier lanes spent from the same $100 before that. These are estimates,
  not the bill: **check the project's billing page** before any large launch.

## 6. The owner's standing rulings (not written down elsewhere)

The owner is **Scott** (GitHub `sgladstein`). They use a Mac terminal, so give zsh commands.

- **North star: a fun sim game, not accurate ant science.** Biology is inspiration. Judge by game value, and stop tuning once it works.
- **One general brain for every task**, found by selection. Start from scratch: no old hand rule is kept unless it helps. The brain's energy tax per connection stays.
- **Follow the plan**: trail → forage → dig → brood. Shut down ant work that doesn't build on it. **No lab-colony tests until an ant passes the basic behaviours.**
- **Rewards are stepped**: each step of a task is worth more than all the steps below it together. That is how forage (reach food, pick up, carry, reach the nest carrying, drop at the nest) and the dig stages are scored.
- **Pictures with every result.**
  - Every training run gets a training-curves picture with every generation from gen 0, fixed axes, and every label explained.
  - Every major step's report carries an image, chart or GIF.
  - **No "passed" until the owner has seen the pictures and a GIF.**
- **Scores get gamed.** Every arena keeps:
  - a survival check;
  - a second number that can't be faked;
  - a blind control;
  - hold-out worlds and a sealed set;
  - pictures.

  The cheats seen so far are in `Reports/brain-training/ladder/cheats-seen.md`.
- **Seeds**: 1-4 seeds to iterate on a dig step. 12 seeds against `main` for a game change or a rung's pass. Mutation is off when measuring (`PIXEL_PHYSICS_MUTATION=off`); only the training rig mutates.
- **Features ship on by default** when they have no measured downside. Tell the owner the numbers.
- **Usage is tight.** Stay on the critical path, no side probes, and report at milestones rather than constantly.
- **Spend**: Google up to $100 in total, as above. Any other paid service needs the owner's explicit go.
- **Questions**: ask so they can be answered in one word, give your recommendation with pros and cons, and give ETAs from measured pace. **The owner dislikes repeats**, so check `Reports/dead-ends.md` (grep the mechanism) and these notes before proposing anything. **Give the real reason**; the owner spots circular reasoning.
- **Writing to the owner**: lead with what it does and where it is going, in the world's words (ants, soil, shaft) rather than code's, then the mechanism. Keep it short.
- **Big redesigns** need a design doc and an adversarial review before building.
- **Rulings of 2026-10-20** (owner, in the session that took the lane over):
  - **Dig step 4: pass** (s161 gen 336, after the GIFs).
  - **Kept-skill floors against a fixed brain: yes** -- `keepref=` (s91) from the next run on.
  - **Several brains in one box sharing a shaft: no** (it ranks brains no better than chance); separate shafts only as a timing check.
  - **Earlier rejections may be revisited on the evidence**: "you are allowed to revisit any ideas that seem promising regardless of previous rejections" (RL, NEAT and big-network ES were rejected by reasoning in the design of record, never by test).
  - **Protect connections the learning arena cannot test: yes** -- do not prune or mutate links that serve only verbs the arena never offers (the deletion that cost s161 its "don't eat the cargo" link; investigation part 3).
  - **Teaching by copying: test both label sources, side by side** -- (A) self-imitation: the brain's own decisions from trips that became full loads, filtered by the scorer; (B) a hand-written teacher, time-boxed to one day of building (the owner's caution: weeks of hand-encoded behaviour failed before training replaced it, so B's difficulty is part of what is measured). Same copying code for both; offline first (24 sealed worlds at 4 and 8 rows, against s161, kept skills against `keepref=s91`), then a short 8-row training run from each survivor beside plain s161, with GIFs for the owner.
  - (Status 2026-10-21: the teacher lane built the copier; a hand teacher's three calls are worth about 20.7 full
    loads on any of our brains; copying "go down" works; copied dig collapses; self-imitation not yet run.)
  - **Reinforcement learning: yes, but after teaching-by-copying, not before** -- clone a hand-built digger first (it builds the gradient-through-the-brain code and has the stronger evidence), then test RL head-to-head against selection on one dig step, with a design doc first.

- **Rulings of 2026-10-21:**
  - **"Local signals and true biology" holds for the trained ant.** The home vector and depth stay exact for now (no
    noise; it would make training harder; maybe later). The roof forecast was replaced with a touch reading (done).
  - **Copy-then-select approved** (the run in section 0). The seeded-into-a-few-parents arm deferred.
  - `stepnear=` off in any run that is also a comparison; on at about 0.3 later.
  - **Racing on; mutation stays `pm 0.03 sigma 0.3 prune 0.3`; 12,000-frame worlds; judge on test worlds** (section 0).
  - **Ask before any VM run** with its cost; the owner sets a cap per run.
  - The owner wants **pictures and a plain summary** with each result, and **to be told plainly when a number they
    were given was wrong** (several were corrected on 2026-10-21; the 14:40 log entry lists them).
  - While waiting on VMs, **run the cheap local tests** rather than idle (the owner, 2026-10-21).
  - Merge your own PRs once every distinct CI check is green on the PR head (the owner, 2026-10-20: "merge the PRs once
    CI is green"); confirm all checks finished first.

- **How the owner works with this lane** (what the last session learned, 2026-10-20/21):
  - They ask plain "why" questions about the training box (why a step, a stone, a mirrored twin; what is the brain
    memorising; how does an ant know its row). Answer in the world's terms first, then the mechanism, with a number.
    The investigation report (`investigation-2026-10-20.md`) and the log's 2026-10-21 variability entry hold the answers.
  - They push back on claims ("can we tell if the changes made training better?"). Twice that exposed a real flaw
    (the hold-out bias; the `frames=` bug). Re-check the claim with the correct method and say plainly what was wrong.
  - On disappointing results they asked for the correct method, not a kinder number ("I don't want to bully you into
    doing something that is less accurate"). Keep to the method; do not switch metrics to rescue a result.
  - They start long runs before bed and want a go/no-go question asked before they leave, with the cost.
  - They ask percentages complete for VM runs: report generations done / expected and the ETA.

## 7. Repo and PR state (2026-10-21 17:40 UTC)

- Everything from this lane is on `main` (PRs 741-760, 765). Open PRs not from this lane (705, 706, 702, 701, 686 and
  older) are the owner's call; leave them.
- Merging: merge your own PR once every distinct CI check is `success` on its head (a cancelled duplicate run is normal).
  Never `git add -A`; stage explicit paths. Before pushing trainer code:
  `cargo test --release --lib train::`; `cargo clippy --all-targets --release --locked -- -D warnings`;
  `PYTHONWARNINGS=ignore bash scripts/docscheck.sh`. Trainer changes get a short design and an independent review first
  (an agent reviewing against the code); every review this lane ran found real faults.
- **GitHub from this container**: `gh api` works (the remote prints "repository moved" to `Pixel_Physics`; both
  spellings work). Open a PR: `gh api -X POST repos/sgladstein/Pixel_Physics/pulls -f head=<branch> -f base=main
  -f title=... -F body=@file`. Check CI on a head: `gh api repos/sgladstein/Pixel_Physics/commits/<sha>/check-runs
  --jq '[.total_count, ([.check_runs[]|select(.conclusion=="success")|.name]|unique|length),
  ([.check_runs[]|select(.status!="completed")]|length)]'` (all completed, none failed other than a cancelled
  duplicate). Merge: `gh api -X PUT repos/sgladstein/Pixel_Physics/pulls/<n>/merge -f merge_method=merge -f sha=<head>`,
  then `git fetch origin main` and `PYTHONWARNINGS=ignore bash scripts/docscheck.sh`. A CI run takes about 15 minutes once it starts (measured on PR 760).
- Put the fault back to prove a new guard can fail (two of this lane's guards were blind until then).
- **Game switches**: the shipped game's ant is frozen (bug and world fixes only); brain work stays in the training rig.
  The teacher lane is about to turn the spoil roll on in the games; training worlds stay pinned off (`hold_training_world`).

## 8. Where things are

| What | Where |
|---|---|
| The brain; its senses (`context_names`) and option features | `src/sim/creature/mind.rs`; `src/sim/creature/options.rs` (`f::`) |
| The roof sense (touch) | `src/sim/creature.rs` `bite_is_hanging` (the old forecast `cut_drops_roof` is test-only) |
| Trainer core: selection, racing, hold-out, probe, resume, `noprune`, `protect` | `src/train/mod.rs` (`Settings`, `race`, `test_layouts`, `flat`) |
| Arenas and scores (soil, fetch, trail, suite), `hold_training_world`, `DigBed` | `src/train/challenges.rs` |
| Command line, every argument documented | `examples/train.rs` header |
| Judging script | `Reports/brain-training/tools/judge.py`; GIF tools beside it |
| Investigation of 2026-10-20 (what the brain reads, why fetch erodes, selection noise, the field) | `Reports/brain-training/investigation-2026-10-20.md` |
| Designs built today (with their reviews) | `Reports/brain-training/design/noprune-design-2026-10-21.md`, `test-probe-design-2026-10-21.md` |
| Research lane (Q1-Q5, ranked list of tests) | `Reports/brain-training/research/` |
| Teacher lane (copier, copies, ceilings) | `Reports/brain-training/teacher-training/`; handoff `Reports/handoff/teacher-lane-handoff-2026-10-21.md` |
| Soil physics report (touch roof rule) | `Reports/soil-physics-realism-2026-10-21.md` |
| 8-row runs: round 1, racing A/B, round 2 | `Reports/brain-training/stage2/dig/8/`, `8/racing-ab/`, `8/round2/` |
| Offline pictures (mutation size and map) | `Reports/brain-training/investigation/mutation-*.png` |
| Start genomes | `training/starts/`: `fretrain-s91` (keepref), `s171-8-g79`, `b181-8-g138`, `r2-cur192-g113` (best) |
| Brain design of record; dig rung design | `Reports/brain-training/design/general-brain-design-rev3.md`; `Reports/brain-training/ladder/dig-rung-design.md` |
| Scratch instruments (not for main) | branch `scratch/brain-instruments` (`scratch-analysis/2026-10-21/README.md` for today's): `PP_MIND_LOG` decision logger, `PP_ABLATE` sense switch, `reliab` (two 48-world sets), `scratch-analysis/` (pellet tracing, brainmath). The teacher lane's option logger + teacher patch: `teacher-training/measurements/option-logger-and-teacher.patch` |
| Google launcher | `Reports/brain-training/cloud-compute/gcp/gcp_train_job.py` |

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
- **The hold-out flatters what it chose**, by 1.3-2.9 full loads for a run's hold-best (log 14:40). Read progress from
  `test.csv` and test-world scoring only.
- **`holdout=N` for judging overlaps the trainer's hold-out** (`holdout_layouts` is one numbered list); use `test=N`.
- **`frames=` did not reach a suite's arenas before PR 748**: every suite run before 2026-10-21 trained soil at 6,000
  frames whatever it said. A state saved before the fix refuses to resume at the new length.
- **Fetch/trail readings move about +-0.03 between world sets**, so a brain "at the floor" can read either side of 0.8.
- **`noprune=@file` wants one slot per line or commas**; the teacher lane's revived-slot files are space-separated.
- **Copied brains**: protect their revived links (`noprune=`), or selection prunes the lesson away.
- **Background jobs die with the container**, which restarted several times on 2026-10-21; check them after any gap.
- **Scheduled check-ins fire only into the session that made them.**

---

## Log

Newest first. Each entry says what changed and what is now true.

- **2026-10-22 01:55 UTC. Copy-then-select beats selection alone on both seeds; every arm lost far fetching; lane closed.**
  - **The run** (`cvs-*`, ref b0dc264, 4 x c4d-highcpu-16, owner's $10 cap; section 0's settings): selection arms
    (from s171) stopped on the flat rule at gens 67 and 68; copy arms (from copy v2, `noprune=`) at gen 79 (seed 202)
    and ran all 150 (seed 201, last 2% gain at gen 103). All four collected with `exit code: 0` and `hold-best.txt`,
    then deleted. Files: `Reports/brain-training/stage2/dig/8/cvs/` (hold-bests, logs, progress, `test96/` scores,
    `cvs-result.png`). A snapshot keeps only the tail of `train.log`, so a long run's early PROBE lines are lost unless
    fetched mid-run.
  - **96 test worlds, full loads a world (s.e. per pair):** s171 9.60, sel-201 10.66, sel-202 10.44; copy v2 10.83,
    copy-201 **12.51**, copy-202 11.94; r2 cur192 12.58 for reference. Copy minus selection, same seed, paired by world:
    **+1.85 +- 0.50 (201), +1.50 +- 0.46 (202).** Worlds at 10+: copy 83% / 83%, selection 70% / 56%.
  - **Not equal CPU on seed 201** (150 against 68 generations); seed 202 is close (79 against 67). Gain over each arm's
    own start: copy +1.68 / +1.11, selection +1.05 / +0.83 -- so the copy's lead is its head start (+1.23) plus a
    little more gain. The selection arms' test-world curves were flat after generation ~20.
  - **Fetch fell in every arm**, mixed distances 0.71-0.78 of s91 (under the 0.8 floor on test worlds), and with food
    30 columns out (`dist=30,30`): sel-201 0.37, sel-202 0.57, copy-201 0.50, copy-202 0.58 (starts 0.64 / 0.70). The
    trips out shrink under dig selection (log 2026-10-21 18:30); the floor at mixed distances did not stop it. PR #766's
    `dist=30,30 calib=96` is the proposed gate, never run. Several probe hold-bests were vetoed on the test worlds.
  - **Open, for whoever picks the arena trainer up:** (1) run the far-food gate; (2) the arena energy sense reads a flat
    0.5 because `budget_to` rescales `start_energy` -- a tested fix is in the teacher lane's hands (patch
    `energy-sense-real-tank.patch` in the project folder; field `CreatureDef::sensed_tank`), and it moves arena scores
    (copy v2 about -0.6 full loads), so re-judge brains after it; (3) the parents'-average child (deferred, design
    `far-fetch-and-centroid-design-2026-10-21.md` part 2); (4) chambers are 7 cells tall (owner, 2026-10-21 22:39), not
    the 8-16 in `dig-rung-design.md` steps 10-11; (5) the speed PRs #772 and #775 and the AVX2 build line cut a suite
    world from about 1.27 to 0.65 CPU-s, bit-identical, so any later run should launch at a ref with them.

- **2026-10-21 18:30 UTC (new session). Fetch erodes only at far food; the floor is reading a real loss.**
  - Lane taken over by a project thread; project rules now override section 6/7 where they differ: the merge desk
    thread merges every PR and runs the full suite (lanes do not merge), the review queue is retired (pictures go to
    the owner in the chat), and every VM run needs the owner's go with its cost.
  - **Per distance** (96 test worlds, share of s91, worlds under half of s91): s171 10/16/23/30 columns 0.86 / 0.91 /
    0.85 / **0.71 (4 of 24 failed)**; cur192 0.99 / 0.95 / 0.92 / 0.81; small192 0.95 / 0.91 / 0.84 / **0.64 (5 of 24)**;
    s91 1.04 / 1.03 / 0.99 / 0.95, none failed. The small-step arm's fetch loss was 5-7 failed far worlds, not a general
    slide.
  - **Traced** (scratch `PP_PIC_TEST`/`PP_PIC_XEVERY` on the picture path; picture
    `Reports/brain-training/investigation/fetch-forays-2026-10-21.png`): after dig training the ant's trips out end
    20-27 columns from the nest; s91 walks 33+. Food at 30 (heap to 26) is hit or missed by a few columns. The diggers
    also sit in the nest between trips; s91 never does.
  - **Far-only fetch** (`dist=30,30`, `test96-scores/far30/`): s91 1.00, cur192 0.84, b181 0.74, copy v2 0.70, s171 0.64,
    small192 0.48 (s.e. 0.04-0.08 over 24 pairs).
  - **Pace bar arithmetic:** per-world spread is about 2.9 full loads for every brain, so 10+ on 11 of 12 worlds (92%)
    needs a mean near 14. cur192: 84% of test worlds at 10+ (worse side 75%); small192 85%.
  - Saved s171 soil/fetch/trail test scores reproduce exactly on main 82d5087a (judging build check).

- **2026-10-21 07:40 UTC. Training worlds pin the roll-on-placement rule off (`hold_training_world`).**
  - The teacher lane's `PIXEL_PHYSICS_SPOIL_ROLL` (PR 753, owner-approved for the games) moves where a put-down pellet comes to rest, so what S3 pays and what falls back into the hole. Every arena that builds a world (forage, scarce/fetch, trail, dig, soil) now sets `World::spoil_roll = Some(0)`, so the teacher lane can turn it on by default in the games without moving the dig ladder's baselines. It comes into the arenas at a rung boundary, with s161/s171 and the pace bar re-measured.
  - Checked end to end: with the roll forced on for the whole process, s171's 24 sealed soil worlds and 96 fetch worlds are identical to the baseline; the control (the soil pin removed) changes 21 of 24 worlds. Guard: `training_worlds_hold_pellets_where_they_are_put`.
  - **The teacher lane's s171 go-down copy, on my judging worlds:** soil +1.50 (sealed 24, se 0.82) and +0.69 (fresh 96, se 0.53) over s171; **fetch 0.73 of s91 (worse side, 96 worlds) against s171's 0.81 -- at the trainer's floor, not over it.** The teacher lane reproduced this and withdrew "meets the floor"; it is rebuilding the copy with a heavier fetch anchor.
- **2026-10-21 17:40 UTC. Round 2 judged on test worlds; copy-vs-selection launched; local tests from the research list; hand-off.**
  - **Round 2** (from b181, 12,000 frames, raced; data `Reports/brain-training/stage2/dig/8/round2/`), hold-bests on 96 test worlds: today's mutation 11.67 (seed 191, gen 48) and **12.58 (seed 192, gen 113)**; small-step 11.53 and 12.95; b181 10.34. Small-step minus today's, paired: -0.14 +- 0.41, +0.36 +- 0.43. Fetch: today's 0.87 / 0.90, small-step 0.77 / 0.78. Arms stopped on the flat rule or the clock at gens 55, 86, 116, 108. VMs deleted after DONE. **Today's mutation stays.**
  - **Copy-vs-selection launched** 16:58 (owner's go, $10 cap): see section 0. On test worlds copy v2 10.83, s171 9.60.
  - **Local tests (research list):** fetch vetoes: 89% within one s.e. of the floor (145 of 163 hold-out champions, 10 runs; trail none). Averaged-child (genetic repair): averaged brain +0.42 / +0.74 / +0.32 vs the kept children's +0.03 / +0.22 / +0.22 on 96 test worlds (ranks 1, 3, 6 of 13). Depth links zeroed: 9.60 -> 5.52; roofed: 7.74; fetch unaffected. Trail per ant at 3/6/12/24 ants: 0.78 / 0.74 / 0.69 / 0.64. 12 ants in one shaft: 8.5 full loads against 1 ant's 11.5, rest 14% -> 17% with contact, Feed 13% of decisions (untrained, protected links). Pruning only small links (|w| < 1 or < 5) at today's setting: 62% / 74% of children clearly worse (prune 0: 44%).
  - **Tool:** `Reports/brain-training/tools/judge.py` (test-world judging, per-pair s.e., worse-side fetch/trail).
- **2026-10-21 14:40 UTC. Judging corrected: a reserved test set and a progress probe (`probe=`, `test=`, `calib=`); mid-run state of round 2.**
  - **The flaw:** the trainer's 24 hold-out worlds choose each run's best brain and when to stop, so they flatter what they choose -- round 2's hold-best brains read 13.2-13.7 full loads there against 10.8-12.3 on 72 worlds never used. And my own "96 fresh worlds" (`holdout=96`) were `holdout_layouts(96)`, whose first 24 ARE the trainer's hold-out; the 24 sealed worlds have also been used to choose brains. Every judging number before this entry carries some of that bias.
  - **Corrected, on the 72 never-used worlds (holdout 24..95):** s171 10.83, b181 11.08; A/B a181 9.82, a182 10.33, b181 11.08, b182 11.79; copy v2 11.88. Round 2 at mid-run (hold-bests at gens 48/35/57/53) against b181: today's mutation +0.85 (seed 191), -0.27 (192); candidate +0.68 (191), +1.18 (192), each about +-0.6. Racing's win stands; whether 12,000 frames and the candidate mutation made the curve steeper is not shown yet.
  - **Built** (design + review: `Reports/brain-training/design/test-probe-design-2026-10-21.md`; 1 BLOCKER, 3 MAJOR, 3 MINOR, all taken): `test_layouts` (seeds 30,000+, a runtime check keeps training out of them); `probe=10` by default, scoring the generation's champion and the hold-best on 96 test worlds into `test.csv`, read by nothing the run decides; `test=N` for judging; `calib=N` so the hold-out can grow without moving the floors. Guards red under each fault put back (one was blind at first and was fixed). **From now on: launch dig runs with `holdout=48 calib=24`; judge with `test=96` and `test.csv`.**
  - **Round 2 mid-run:** `d8r2-cur-191` stopped on the flat rule at gen 55 (11:51) while +0.85 ahead of its start on never-used worlds -- the biased hold-out's cost; VM deleted after DONE. The other three ran on. No message from the teacher lane.
- **2026-10-21 08:25 UTC. Racing A/B judged: racing wins; second 8-row round launched (12,000 frames, racing on, mutation size tested).**
  - **The A/B** (two pairs, from s171, 6,000-frame training because of the `frames=` bug, link protection on; data `Reports/brain-training/stage2/dig/8/racing-ab/`, picture `racing-ab.png`). Unraced arms stopped on the flat rule at generations 78 and 58; raced arms ran 150. Each raced arm found its best brain within its unraced partner's CPU (b181 at 16,376 s against a181's 20,761 s of generation time; b182 at 8,584 s against a182's 15,516 s), so the finals compare at equal CPU. Judged on 24 sealed + 96 fresh 8-row worlds at 12,000 frames, s.e. per mirrored pair, fetch/trail as a share of s91 (worse side):
    | | sealed 24 | fresh 96 | fetch | trail |
    |---|---|---|---|---|
    | s171 (start) | 9.79 | 10.90 | 0.81 | 0.90 |
    | a181 unraced | 10.12 | 10.22 | 0.76 | 0.94 |
    | **b181 raced** | **11.58** | 11.17 | **0.86** | 0.98 |
    | a182 unraced | 10.62 | 10.77 | 0.75 | 0.88 |
    | b182 raced | 10.21 | **11.79** | 0.78 | 0.94 |
    Raced minus unraced, same seed: fresh +0.95 (se 0.48) and +1.02 (se 0.51); sealed +1.46 (se 0.86) and -0.41 (se 1.09); fetch +0.10, +0.03. **Racing goes on** (it was to go on if no worse within noise at equal CPU; it is better on the larger set in both pairs). The unraced arms made no real progress on s171 at all.
  - **Mutation size, two more offline arms** (research lane's Q1): protecting links above |w| 1 or 5 from pruning at today's setting leaves 62% / 74% of children clearly worse (unprotected 76%, prune 0 44%, sigma 0.1 prune 0 17%): the damage is spread over ordinary links, so the candidate stays sigma 0.1, prune 0.
  - **Launched (owner's overnight go, $25 cap; tonight about $13 in all):** `d8r2-cur-191/192` (pm 0.03 sigma 0.3 prune 0.3) and `d8r2-small-191/192` (pm 0.03 sigma 0.1 prune 0), 4 x `c4d-highcpu-16`, `--hours 8`, ref `95ea03a`, from `training/starts/b181-8-g138.txt`, `frames=12000 budget=2400 race=8 layouts=48 holdout=24 pop=100 keepref=training/starts/fretrain-s91.txt gens=150`, stepnear off. The first run whose soil worlds really are 12,000 frames long.
  - **The teacher lane's copy v2** (`s171-down-v2.txt`, its PR 757; protect list 83 slots in `s171-down-v2-revived.txt`, space-separated): on my worlds soil 11.88 sealed (+2.08, se 0.70) and 11.71 fresh (+0.81, se 0.49), fetch 0.83, trail 0.88 -- **meets the condition** and is the strongest brain we have. The copy-vs-selection run waits for the owner.
- **2026-10-21 07:50 UTC. The roof sense is now touch, not a forecast (owner's approval); behaviour bit-identical.**
  - `f::ROOF_DROP` is computed by `creature::bite_is_hanging`: some grain touching the bite rests on it or on open air and is held by two or fewer other grains, or a pellet rests on it (the soil-physics report's rule, PR 751). The old `cut_drops_roof` (the engine's collapse rule one cut ahead) is kept only as the test reference.
  - **Identical runs, the test of the swap:** s171 on 24 sealed + 192 fresh soil worlds (8 rows, 12,000 frames), 96 fetch and 96 trail worlds, and s161 and s172 on the 192 -- every world's every number identical to the forecast runs. The ground-truth test (`the_roof_drop_sense_calls_the_cuts_that_drop_a_roof`: every diggable cell around a lined founding cut, really cut on a copy) now also asserts the touch reading equals the forecast on every target; red with the hold count loosened to 1.
  - **Mutation map (offline, ten settings, 30-50 children of s171 each, 96 worlds):** children clearly worse than the parent -- current (pm 0.03, sigma 0.3, prune 0.3) 76%; prune 0.1 67%; prune 0 44%; pm 0.03 sigma 0.1 prune 0 **17%**; pm 0.01 sigma 0.1 prune 0 2%. Step size and pruning drive the damage; no setting gave clearly better children at a measurable rate. **Candidate for the paired run: pm 0.03, sigma 0.1, prune 0** (today's breadth, small steps, no pruning). Picture: `Reports/brain-training/investigation/mutation-map-s171.png`.
- **2026-10-21 07:05 UTC. `noprune=` built (named slots never pruned), after a design review.**
  - For the copy-then-select run: `noprune=1498,654,699,789,834,924,1059,1149,1239` (or `noprune=@file`). Listed slots still mutate; they are never set to 0, and a listed scorer input brings its unit's output weight with it. Refused if any reads 0 in the genome `train` receives. Written into the arena string only when set, so older states resume.
  - Design and review: `Reports/brain-training/design/noprune-design-2026-10-21.md` (one MAJOR gap, the unit's output, and two blind guards, all fixed). Guards: `no_prune_slots_survive_a_run_that_pays_for_pruning_them` (end to end with a resume; control: unlisted, both pruned), `an_empty_no_prune_list_mutates_as_before` (fingerprint recorded on main before the change; red when the prune branch is altered), `a_dead_no_prune_slot_is_refused`. 66/66 trainer tests, clippy clean.
  - **The soil-physics session's report (PR 751):** a present-tense touch rule -- a grain beside the bite rests on it or on air and has two or fewer other grains holding it, or a pellet rests on it -- matches `f::ROOF_DROP` on all 116,388 of s171's Dig options (8,186 flagged, zero disagreements). **Owner, 2026-10-21: approved replacing the forecast with that sense.** Next in this lane.
- **2026-10-21 06:00 UTC. The roof forecast is load-bearing; tonight's errors recomputed per seed; the teacher route's condition.**
  - **Roof flag** (`f::ROOF_DROP`, `cut_drops_roof`: the engine's collapse rule run one cut ahead -- a forecast, not a sense). Scratch switch blanking it (counted: ~6,600 flagged Dig options on 24 sealed worlds; intact run identical to main's), s171, 8 rows, 12,000 frames, s.e. per mirrored pair: **full loads 9.79 -> 8.12 (sealed 24), 10.90 -> 9.47 (fresh 96); soil falling into the hole 0.12 -> 1.62 and 0.04 -> 1.03 a world.** `c::ROOF_CUT` changes no decision (flag + warning off is identical to flag off). The brain reads the flag at the clamp (+40 into scorer unit 12, out at about -40), the link seeded in `mind.rs`. The owner holds "local signals and true biology" for the trained ant and is **waiting** on a separate soil-physics session (touch-based replacement) before any change; it has these numbers.
  - **Per-seed standard errors** for tonight's 24-sealed comparisons (12 pairs): s171 - s161 +3.17, se 0.89 (was 0.79); s172 - s161 +3.88, se 0.92; s171 - s172 -0.71, se 0.43; s162 - s161 +1.50, se 0.66. The step near the shaft on 192 beds, seed-clustered regression: s171 -1.39 (se 0.59), s172 -1.67 (se 0.64), s161 -0.27 (se 0.58). Every conclusion stands.
  - **Owner rulings (2026-10-21):** `stepnear=` stays off for the next run (it doubles as the mutation comparison and the first real 12,000-frame run); turn it on at about 0.3 in the first run that is not a comparison. The teacher route goes ahead as copy-then-select, tested against selection from s171 at equal CPU, **once the teacher session's copy keeps fetch and trail at or above 0.8 of s91** (its go-down copy: 0.71 / 0.71; a start under the floor spends its early generations vetoed); condition sent to it. The named-slot protect list (slots 1498, 654, 699, 789, 834, 924, 1059, 1149, 1239) is built with a review before that run.
- **2026-10-21 05:00 UTC. Mutation size, offline: pruning wrecks children; smaller steps keep them near the parent; no setting shown to improve faster.**
  - 50 children of s171 under each of four settings (the teacher session's `mutants` tool over `train::mutate`, grow 0.1 throughout, no crossover), each on reliab's two 48-world sets at 8 rows, 12,000 frames. Controls: 202 x 96 rows as expected; the parent under a second name identical on every world. Picture: `Reports/brain-training/investigation/mutation-size-s171.png`.
  - | Setting | Weights changed | Median child (parent 9.47) | Clearly worse | Clearly better |
    |---|---|---|---|---|
    | A, current: pm 0.03, sigma 0.3, prune 0.3 | 27 | 5.74 | 38 of 50 | 0 |
    | B: pm 0.01, sigma 0.1, prune 0.3 | 9 | 9.52 | 11 | 0 |
    | C: pm 0.03, sigma 0.3, prune 0 | 28 | 8.71 | 22 | 0 |
    | D: pm 0.01, sigma 0.1, prune 0 | 9 | 9.54 | 1 | 2 (about chance) |
  - **Pruning is the bigger wrecker:** at the current step size, switching it off halves the wrecked children (38 -> 22). Smaller steps keep children near the parent with the upper tail no lower (p90 9.6-9.9 in every arm). One generation of truncation selection, picked on set 0 and judged on set 1: +0.21 / +0.27 / +0.06 / +0.32 against the parent's set-1 score -- differences between arms well inside the noise, so **this test cannot say whether smaller steps train faster**; it says they waste far fewer evaluations and that pruning does most of the damage. Decisive test: a paired run on Google, current against pm 0.01 sigma 0.1 prune 0 (wire cost kept), after the racing A/B.
- **2026-10-21 04:10 UTC. The teacher session's first copies** (its branch, `step1-results-2026-10-21.md`; message to this session): copying "go down" alone into s161 gives **10.57 +- 0.39** full loads on its 96 worlds (s171 by selection: 9.47), fetch 0.71 of s91 (s171 0.81); all three rules 8.64 with fetch 0.37 (walk-home spreads to loaded ants and foragers); dig alone collapses (3.35). Its reading: copy a lesson in, then select from the copy with the floor. **Slots a selection run from a copy must protect** (revived, 0 in s161): 1498 and 654, 699, 789, 834, 924, 1059, 1149, 1239 -- needs a named-slot protect list in the trainer (review before building; only if the owner chooses that run).
- **2026-10-21 04:30 UTC. Teacher-lane review: its ceiling reproduces, and does not move with training.**
  - The teacher session's three-rule teacher (dig only with a foot cell in reach, go down when empty in the shaft, walk to the mouth when empty on the surface; `option-logger-and-teacher.patch` on their branch `ccr-439ee31b-pn36yq`, applied to `scratch/brain-instruments`) on their 96 training-style 8-row worlds at 12,000 frames, standard errors clustered by mirrored seed (`arms.py`): **s161 alone 6.80 +- 0.39, with the teacher 20.66 +- 0.37** (their numbers exactly); **s171 9.47 +- 0.41 -> 20.50 +- 0.38; s172 9.50 +- 0.46 -> 20.85 +- 0.39.** 100 generations of selection closed about a fifth of the gap to what those three calls are worth.
  - Sent to the teacher session (send_message, at the owner's request): the reproduction, s171 as a second copy target with a selection-equivalent comparison in the bar, the `frames=` bug, and an offer of a named-slot protect list for links a copy revives (their slot 1498, pruned to 0 in s161).
  - **From their work into mine:** standard errors clustered by mirrored seed from now on (twins correlate 0.41-0.65, so the per-world s.e.s in tonight's entries are 20-40% too small; the conclusions stand), and trainer changes reviewed before they are built.
  - Timestamps of the four entries below corrected to their commits' times (00:39, 00:48, 00:59, 03:08); they were written 15-25 minutes late.
- **2026-10-21 03:08 UTC. `frames=` never reached the soil arena in a suite: the `dig-8` runs trained at 6,000 frames. Fixed; `stepnear=` added.**
  - **The bug:** `examples/train.rs` built every suite arena with `frames: None`, so `frames=12000` changed nothing inside a suite -- two suite runs at 6,000 and 12,000 gave identical hold-out rows. `dig-8-s171`/`s172` and all four racing A/B arms trained soil at **6,000 frames** (with the doubled 2,400 budget); only the scoring afterwards (`challenge=soil`) ran 12,000. So these runs never tested part 4's 12,000-frame recommendation, and the "about the same time a generation" I read as cheap was soil running at its old length.
  - **Fix:** in a suite `frames=` now goes to the learned arenas; kept ones keep their own length (fetch and trail unchanged, checked). After it, the same check reads soil 105 at 6,000 and 158 at 12,000; unset and `frames=6000` reproduce the old numbers. A suite with `frames=` writes ` frames=learned` into the arena string, so a state saved before the fix never resumes at the new length. The racing A/B stays valid as a racing test (both arms the same), at 6,000 frames.
  - **`stepnear=P`** (off by default): forces the bed's step 6-10 columns from the shaft on a share P of pairs. From the variability tests below. Guard `step_near_forces_the_step_by_the_shaft_and_nothing_else` (0 is every bed as it was; 1 moves only the step, mirrored).
  - **How much the box features cost (192 hold-out beds, 8 rows, 12,000 frames, three brains):** all features together explain 7-10% of the per-world spread; the brains agree on which beds are hard only weakly (r 0.16-0.32). **Plain beds (flat, 40 rows, lamps 33, no lens, no step; scratch switch) are no less noisy**: spread 2.4-3.1 full loads a bed against 2.6-2.8 on the generator beds, variance ratios 0.79-1.18 with 1 inside every interval. So the per-world noise is the run's own, and simpler beds would not cut it. What the features do catch: the step within 10 columns of the shaft costs s171 and s172 1.4 and 1.7 full loads (2 s.e.); s161, trained at 4 rows, had an east lean of +0.6 (se 0.3) that both 8-row brains lost under the worse-side fold; the stone shows no consistent effect.
- **2026-10-21 00:59 UTC. Racing A/B launched (owner's go, 2026-10-20).**
  - Four `c4d-highcpu-16` spot VMs, ref `1e9217c` (racing, its floor fix, link protection), from `training/starts/s171-8-g79.txt`, arguments as `dig-8` (`shaft=8 frames=12000 budget=2400 layouts=48 holdout=24 pop=100 keepref=training/starts/fretrain-s91.txt`). Two pairs: `race-a181`/`race-a182` unraced, `gens=80`; `race-b181`/`race-b182` `race=8`, `gens=150` (about the same CPU). `--hours 8`, so about $5.
  - **Judging:** each arm's hold-out best at equal CPU (the raced arm's snapshot where its summed `secs` matches its unraced partner's total) and at equal generations, on 24 sealed 8-row worlds plus 96 fresh worlds, and fetch and trail against s91 on the same worlds. Unpaired means: the teacher session showed paired worlds save nothing between different brains. One pair cannot show racing's effect; two can show only a large one.
  - Timestamped snapshots of the raced arms are being saved every 10 minutes to the session scratchpad (`ab/race-b18X/HHMM/`). (Those snapshots went with that session; the results are in this log, the final snaps in `8/racing-ab/`.)
- **2026-10-21 00:48 UTC. Dig step 5, first round done: +3 to +4 full loads, bar not yet met; fetch at the floor.**
  - `dig-8-s171`, `dig-8-s172` finished 100 generations (DONE confirmed, then VMs deleted). Files: `Reports/brain-training/stage2/dig/8/` (hold-bests, curves, snapshots); s171's hold-best is also `training/starts/s171-8-g79.txt`.
  - On 24 sealed 8-row worlds (12,000 frames, budget 2400), full loads a world: **s171 gen 79 9.79, s172 gen 82 10.50**, against the start s161 gen 336's 6.62 (+3.17 se 0.79; +3.88 se 0.83); s171 against s172 -0.71, se 0.51. Each has 15 of 24 worlds at 10+, 7 and 8 of the first 12: **the pace bar (10 on 11 of 12) is not met.**
  - **Kept skills, on 96 fresh worlds against s91 on the same worlds:** fetch s171 0.81, s172 0.75; trail 0.90 and 0.84. Both runs sat at the floor all the way (about 40% of each generation vetoed for fetch); the run's own hold-out (0.86-0.94) flatters the champion it picked. These runs had no link protection.
  - **Where the loads go now (every pellet traced, both rebuilds reproduce each world's full loads):** s171 cuts the foot 11.4 times a world (s161: 7.5) and the wall 4.8 (s161: 10.6), so training halved the wall cuts the teacher session's first correction was aimed at; rim 2.9 and re-cuts 2.3 rose slightly, and 46% of cuts are still wasted (s161: 66%). For paid loads the empty walk back is the biggest time cost (median 630 frames from a put-down to the next foot cut, against 245 carrying up) -- the teacher session's D3 correction ("empty on the surface: step towards the mouth", where the brain agrees 53%).
- **2026-10-21 00:39 UTC. Racing's first round no longer cuts on a kept arena's floor; s171 done.**
  - **The flaw (found before the A/B):** a kept arena's floor is a share (0.8) of the reference, and a brain under it is scored at a tenth. On racing's 8-world first round the floor fires on noise: s172's gen-70 champion, fetch 0.87 of s91 on 96 fresh worlds, falls under on 26% of 8-world draws against 8% of 48-world ones. So round 1 was cutting good diggers at random. Fix: round 1 ranks by `Challenge::unfloored` (a suite's product with no penalty); the floor still decides on every layout for the survivors. Guard `racing_does_not_cut_on_a_floor_read_over_a_few_worlds` (red on the old cut: both diggers dropped).
  - **s171 final** (hold-best gen 79, `top` matches `hold-best.txt`): **9.79 full loads a world on 24 sealed 8-row worlds against s161 gen 336's 6.62 (+3.17, se 0.79)**, 15 of 24 at 10+ (7 of the first 12). The pace bar is 10 on 11 of 12: not yet. Fetch on 96 fresh worlds 0.81 of s91 on the same worlds -- the floor holds, barely; the run's own hold-out (0.86-0.92) flatters it, because it also picked it.
  - **From the teacher-approach session (00:07, branch `ccr-439ee31b-pn36yq`):** s162 gen 338 is the stronger untrained brain at 8 rows -- confirmed here on 24 sealed, 8.12 against s161's 6.62 (+1.50, se 0.61); mutants' per-world scores barely track their parent's (median correlation 0.05), so pairing worlds between brains does not cut noise; wall cuts are a dig-or-not error (65% of s161's 8-row digs had no foot cell in reach).
- **2026-10-20 23:35 UTC. Racing built (`race=N`, off by default until an A/B run measures it).**
  - With `race=N`, every brain in a generation is scored on the first N training worlds; the better half by that score (and always the control) go on to the rest, and only they can become parents. A survivor's score is its fold over every world, a dropped brain's over the first N. Survivors outrank every dropped brain whatever the numbers, since a dropped one never met the rest. Why: investigation part 5, offline on real per-world scores -- 8 worlds, keep half, +40 picked parents as good as 48 worlds each, at about 28 worlds a brain.
  - Cost per generation: pop x N + (pop/2 + 1) x (layouts - N) worlds, against (pop + 1) x layouts. At `layouts=48 race=8 pop=100` that is 2,840 against 4,848, about 41% less.
  - Tests: `race_keeps_the_better_half_and_scores_it_on_every_layout` (exact survivors, scores and run count on a counting ruler arena; red when the cut keeps the worse half) and `a_raced_run_trains_and_says_so`. 61/61 trainer tests, clippy clean. Smoke run on the dig suite (`pop=12 layouts=8 race=2`): `race: 6 of 12 kept after 2 of 8 layouts` each generation, 23-29 s a generation.
  - `race=N` is written into the arena string, so a resume cannot mix raced and unraced generations. N must be even on a mirrored challenge, so no mirrored pair is split; under racing the printed median is a dropped brain's first-round score.
  - **Next:** one paired A/B on Google, raced and unraced from the same start and seed, judged on 24 sealed worlds at equal CPU. Needs the owner's go (about $6).
- **2026-10-20 22:05 UTC. Link protection built (`protect=`, on by default).**
  - The trainer now leaves alone every slot that serves only a verb the learning arena can never use: in a digging suite (soil learned, fetch and trail kept) that is Pick up, Eat and Feed, 219 slots (73 each: switch, job row, hidden-to-job row, scorer verb weights, memory's "just chose it" weights), so trainable slots go from 2,761 to 2,542. Mutation and pruning never touch them, so a run can no longer delete "don't eat the cargo" (investigation part 3).
  - Which verbs can act is per arena (`Challenge::acting_verbs`): the soil box has no food or larvae, measured on 38,028 decisions and guarded by `soil_never_offers_food_verbs`, which fails if those verbs ever get a target there. A suite's `untested_verbs` are the verbs only its kept arenas can use.
  - Tests: `protect_keeps_untestable_verbs_links_from_mutation` (positive control: unprotected, Eat's slots do mutate), the soil guard (its counter is checked against verbs that do act), `a_digging_suite_leaves_the_food_verbs_untested`. 59/59 trainer tests, 17/17 mind tests, clippy clean. End to end: a suite run prints `protect: 219 slots serving only [PickUp, Eat, Feed] left alone`.
  - **Resume:** `protect=1` is written into the arena string, so a state saved before it (the `dig-8-s17X` runs, ref `48c67a0`) resumes only with `protect=0` -- which a resume needs anyway, since a resume needs the same build.
  - **Use it from the next run on** (it is the default). The `dig-8-s17X` runs started before it.
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