# Handoff: the teacher lane (teaching the ant brain by copying), for the next session

*Written 2026-10-21 by the teacher-approach session (`session_0155kwr6ZPrC8bXE2JopsdJ4`), at the owner's request, at a
natural stopping point. Read this, then `CLAUDE.md`, then `step05-why-copies-fail-2026-10-21.md` and the design
(`teacher-design-2026-10-21.md`). Everything below is on `main` (merged via PRs #753, #757, #759) unless it says
otherwise.*

## What this lane is for

The owner wants the evolved ant brain taught by copying a teacher, then improved by selection, because selection alone
is slow and colony behaviours look unaffordable by selection alone. The lane built a copier (gradient descent on the
brain's own genome, offline, in Python), measured what copying does, and designed where the teacher goes next (rules ->
planners -> search, plus a colony tier). The brain lane (`session_01CfwaroVA7ncQdU7qtDEcJy`) owns `src/train/` and the
Google runs; a research lane answers literature questions and sends findings by scheduled message.

## Where things are

| What | Where |
|---|---|
| The design (v2, after three adversarial reviews) | `Reports/brain-training/teacher-training/teacher-design-2026-10-21.md`; disposition of 30 findings in `teacher-design-review-2026-10-21.md`; reviews in `review/design-*.md` |
| What copying did (step 1) | `step1-results-2026-10-21.md` |
| Why copied lessons fail (step 0.5) | `step05-why-copies-fail-2026-10-21.md` |
| Guidelines (T1 amended), literature, measurements 1-3 | `guidelines-2026-10-20.md`, `literature-2026-10-20.md`, `measurements-2026-10-20.md`, `notes/` |
| Search teacher design + literature | `search-teacher-design-2026-10-21.md`, `notes/search-teacher-literature.md` |
| Real nests in section; blueprints; general nest score | `notes/nest-architecture-2026-10-21.md`; `measurements/blueprints/` (`nestscore.py`, `render_blueprints.py`, `ringcrescent.py`, `rollcheck.py`, pictures) |
| Spoil roll (engine switch, off by default) | `Reports/spoil-roll-2026-10-21/` (report, the measurement, and `default-on.patch`) |
| **The copier and all analysis scripts** | `measurements/copier/` (`copier.py`, `contextprobe.py`, `probe.py`, `behave.py`, `arms.py`, `sil.py`, `rho.py`) |
| **The Rust scratch code** (decision logger, label-only and acting teacher, `copylog`, `copyreplay`, `mutants`, the `reliab` harness) | `measurements/teacher-probes-2026-10-21.patch` -- a diff against `main` at `eae65c70`; the three new examples also as `measurements/copier/*.rs.txt`. **Never for main**; apply in a worktree at `eae65c70` and rebase forward (main has since changed the roof sense to a touch rule, PR 754, and added the spoil roll) |
| Every genome the tables cite, with its copier config | `measurements/genomes/` (s161 as loaded and quieted, s171 as loaded, the step-1 copies, the step-0.5 arms, the s171 copies) |
| The copy handed to the brain lane | `measurements/step1-runs/s171-copy/s171-down-v2.txt` and `s171-down-v2-revived.txt` (83 slots to protect) |

The logs the copier trains on are not in git (hundreds of MB). Regenerate with the patch's `copylog`:
`PP_COPY_LOG=<file> PP_TEACH_LABEL=1 copylog soil <genome> 9101 48 out.csv 2`, `copylog fetch <genome> 9301 16`,
`copylog trail <genome> 9401 8` (the run seeds every config here used; s171's extra fetch anchors: 9311 x64 for s171,
9321 x48 for the copy's own runs). Evaluation worlds: `reliab 8 12000 out.csv <genomes...>`, run from any scratch
folder (the old `/home/user/m1` was only an output folder): run sets 9001+9002, 96 worlds, 48 seed pairs; s.e.
clustered by seed with `arms.py`. Kept skills:
`copylog fetch|trail <genome> 9801 48`, **but judge any genome bound for the trainer with the trainer's own harness**
(below).

## Getting the scratch code running

The scratch Rust code and the Python copier are not on `main`. To use them:

1. `git worktree add ../tp eae65c7057e9bbe3f749c5e524f73a41d2849172`, then in `../tp`, run
   `git apply <repo>/Reports/brain-training/teacher-training/measurements/teacher-probes-2026-10-21.patch`.
   Copy `measurements/copier/*.rs.txt` to `../tp/examples/*.rs`; the patch already carries the tracked files. Rebase
   onto current `main` if you need the roof touch rule (PR 754) or the spoil roll.
2. Build: `cargo build --release --example copylog --example copyreplay --example reliab --example mutants`. The
   floor checks use `examples/train`, from either tree. The patch only adds the `PP_GIF_SEALED` picture switch to it.
3. Python: numpy and autograd. Run scripts with `python3 -I`.

**The scratch switches** (environment variables, read by the patched `mind.rs` / `challenges.rs`):

| Switch | What it does |
|---|---|
| `PP_COPY_LOG=<file>` | Full-precision log of every option of every decision: Q rows, plus G/C/D/E rows for ground, cuts, put-downs and results |
| `PP_TEACH_LABEL=1` | Adds the three-rule teacher's labels to that log, soil arena only |
| `PP_TEACH=dig,down,home` | The acting teacher: the rules drive the ant. This is the ceiling test |
| `PP_TEACH_Q=<p>` | Each teacher call applied with probability p |
| `PP_TEACH_FLOOR=1\|2` | Floor-limited arms |
| `PP_TEACH_SAT` | Score saturation for the floor-limited arms |
| `PP_GENOME_OUT=<file>` | Save the genome as loaded through the suite. This is the copier's starting point |
| `PP_MIND_LOG`, `PP_ABLATE`, `PP_GIF_*` | From the 2026-10-20 investigation: decision log, sense switch, GIF frames |

**The copier** (`copier.py`):

| Command | Does |
|---|---|
| `train CFG.json` | The copy |
| `replaycheck LOG GENOME OUT` | Checks the forward pass against Rust's `decide` |
| `memcheck LOG GENOME` | Checks the memory path |
| `roundtrip IN OUT` | Reads and rewrites a genome |
| `quiet IN OUT` | Zeroes unread units' input links |
| `perturb GENOME OUT SEED SIGMA [1]` | Jitters a genome; 1 = trainable slots only |

Config keys:

| Group | Keys |
|---|---|
| Start, data and target | `start`, `soil_logs`, `keep_logs` (`[[arena, path]]`), `arm` (`teacher`, `imitate`, `shuffle`, `sil`), `target_genome`, `rules` |
| Groups | `group_weights` and `caps` per group: `digM`, `digN`, `down`, `home`, `none`, `keep`, `keepL` |
| Optimiser | `steps`, `every`, `batch`, `lr`, `seed`, `patience`, `holdout_frac` |
| Model | `mempath`, `quiet_unread`, `depth_into_live_units`, `allow` |
| Added tonight | `freeze` (`["roof_drop"]` or slot numbers), `revive_units`, `revive_sigma`, `all_slots`, `arena_bit`, `keep_lookalike` (`"below"`) |
| Self-imitation only | `t_down`, `t_home` |

**The configs in `measurements/genomes/*.json` use this container's absolute paths** (`/home/user/...`). Rewrite
`start`, `soil_logs`, `keep_logs` and `out` before reusing them.

## Reference numbers (so nobody re-derives them)

| What | Number | Where measured |
|---|---|---|
| s161 alone, full loads (as loaded / quieted) | 6.80 / 6.42 | 96 eval worlds. Quieting zeroes 184 unread links, 706 -> 522 live |
| Teacher acting on s161 (dig + down + home) | 20.66 +- 0.37 | Memory-consistent ceiling |
| s171 alone / teacher acting on s171 | 9.47 / 20.50 | 96 eval worlds |
| Fetch / trail, s91 / s161 / s171 (my 48-world sets) | 24.23 / 16.66 / 19.64 and 0.864 / 0.640 / 0.796 | `copylog ... 9801 48` |
| Fetch worse side, trainer's harness: s171 / copy v2; floor | 19.34 / 19.83; 17.61 (0.8 x keepref 22.015) | 96 worlds, 48 mirrored pairs |
| One 12,000-frame soil world | ~0.7 CPU-s | -- |

## Started, planned, or never run

- **Step 0.5 items 4 and 5** (the design's §4): the tau retrofit (regularised against hard targets) and DAgger anchored
  to the current student. Not run.
- **The self-imitation arm.** Its labels are built and checked (`sil.py`), but it has never been trained. The research
  lane's Q3e says never pool across arenas, and reweight the brain's own odds per arena.
- **DAgger with failed trips only** (Li et al. 2020, from the research lane). Not run.
- **Dig + go-down copied into s171.** Only go-down was copied into s171. The step-0.5 dig results are on s161.
- **The "bottom of the hole" sense.** Proposed only; no probe yet.

## Review cards still open in the owner's queue

These were answered in chat. Mark them answered or let them lapse; don't treat them as pending:
- `20261021T051824537Z-683876`: nest blueprints. Answered: harvester; chambers bigger, then 7 cells.
- `20261021T054209047Z-78f707`: ring against crescent. Answered: ring.

## State at handoff

**Done and on main.**
- Step 1: copying works for "go down" (s161 6.42 -> 10.57; repeats over seeds). "Dig" and "walk home" fail when copied
  alone.
- Step 0.5: why.
  - **Dig is wiring.** The verb stage barely reads the dig score (weight -0.31 against a -6.30 bias). One revived
    scorer unit removes most of the collapse (3.35 -> 5.80). With one unit, dig + go-down gives 10.67 with fetch held.
  - **Dig's last gap is a discrimination limit.** The dig score separates a foot cell from a wall at AUC 0.61. The
    copy still cuts walls on ~9 of ~22 trips, and weighting "don't dig here" harder only makes it dig less.
  - **Walk home spreads to foragers.** The arenas are separable only by artefacts (fetch is fed), and neither capacity
    nor routing through unit 14 fixes it.
- **s171 copy v2 meets the brain lane's floor**, confirmed by the brain lane on its own worlds.
  - Copy: soil +2.08 sealed / +0.81 fresh; fetch 0.83 of s91.
  - Against 150 generations of raced selection: +0.42 / +0.90, fetch 0.78.
  - The fix was a look-alike anchor group plus one revived unit. v1 failed because go-down trapped foragers in the
    nest chamber.

**Decided by the owner (2026-10-21, after this was first written).**
1. **The copy-vs-selection run: go.** Two runs per arm if the budget allows (~$7): copy v2 (main `ec86ab0a`,
   `noprune=` with the 83-slot file) against s171, same settings and CPU. Optionally a third arm with the copy seeded
   into only ~5 of 25 parents (the research lane's lock-in check), at the brain lane's discretion. **The brain lane
   runs it** (it owns `src/train/` and the Google runs); this lane reads the result.
2. **Spoil roll: fix the trace bug, then turn the roll on by default (option A).** With the roll on,
   `sim::creature::tests::every_traced_decision_agrees_with_the_counters_and_the_positions` fails, and the roll is not
   the cause:
   - The roll changes that test colony's path so it reaches a walled-in hungry ant.
   - `pack_behind` (NeedsFirst's pack) calls `commit_step`, which books `DecisionOutcome::Stepped` with no cone pick,
     and there with no head motion.
   - So a pack is logged as a walking step: a latent bookkeeping bug that misattributes walled-in ants' decisions in
     every trace.

   The fix:
   - Book a pack as its own outcome, or otherwise not as a cone step.
   - Keep the `moves` and step-count assertions consistent, then re-run that test and `cargo test --release --lib`.
   - Apply `Reports/spoil-roll-2026-10-21/default-on.patch` (the one-line default plus its docs).
   - Training arenas stay pinned off (PR 756); tell the brain lane the PR number.
   - Bookkeeping only, no behaviour change; it touches the hand-written ant's trace code in `creature.rs`, so run
     `bash scripts/branchcheck.sh --who-touched src/sim/creature.rs` first and land quickly.

## What to do next, in order

0. **The spoil-roll trace fix and default flip** (decision 2 above): small, approved, and it unblocks a feature.
1. **Fold step 0.5 and the research lane's answers into the design (v3).** The research lane's findings arrived by
   scheduled message overnight; its reports are under `Reports/brain-training/research/` (branch
   `claude/project-thread-hhopdh` until merged). What they change:
   - **Q3a.** Unit 14 is the brain's empty-only unit (tested: routing walk home through it helps soil, not fetch).
     EWC and PCGrad are not worth it here.
   - **Q3c.** DAgger hurt because learner-visited states use up a small brain's capacity (Laskey 2016), and its labels
     assume the teacher carries on.
     - Use coaching-style targets with a *schedule*: tau loose, then tightening.
     - Li et al. 2020 relabel only *failed* trips. This is cheap to try: relabel the trips that stranded, keep the
       labels where trips worked.
   - **Q3b.** Seed the copy into a few parents, not all of them (Lamarckian lock-in).
   - **Q3e.** Never pool "good trips" across arenas for self-imitation.
   - **Q4.**
     - Size colony training by contacts per ant.
     - Pioneer ants that fade out.
     - Check whether the brain rests when a tunnel is blocked.
   - **Q5.**
     - A headroom-over-the-head sense needs reach >= 8 for 7-cell chambers.
     - CO2 is no depth cue.
     - Slanted shafts are not wrong (age).
2. **Dig's discrimination limit -> a sense.** The candidate is local: "the target is the lowest open-adjacent cell of
   the cavity the ant stands in", which is the bottom of the hole felt by gravity and touch.
   - First run the probe (brain channel) with that feature computed offline from the logs. Does it separate foot from
     wall?
   - Only then propose it to the brain lane (new links at zero, through their lane).
3. **The search teacher pilot (design §8)**, not started. Do its instrument checks A1-A4 and the pairing measurement
   S1 first. Its target, dig-or-not, is exactly the lesson that copying cannot finish.
4. **Nest work.**
   - Rust port of `nestscore.py` beside `src/lab/nestcensus.rs`.
   - The harvester-profile plan generator.
   - The nest planner on the roof and chamber rungs.
   - The ring mound (owner's choice), with mound cuts never labelled "no" (see the `MOUND_DIG` dead end). The shipped
     game already has `PIXEL_PHYSICS_SPOIL_SIDE=trail` on, which drops a carrier's pellet on the side of the door with
     less food trail -- a lopsided ring. Build on it rather than beside it.
5. **Walk home**: decide with the owner. Either leave the empty return trip to selection, or teach it only in an arena
   where diggers and foragers share the world.

## Owner rulings made in this lane (2026-10-20/21)

- Chamber height: 7 cells.
- Harvester stack is the favourite nest; any of the five is acceptable.
- Different species / creatures digging different nests later.
- Score the nest by general properties, not one blueprint.
- **Ring** mound.
- Pellet settle: "go with your suggestion" (the roll at placement, never loose soil).
- New senses: allowed if local and biologically realistic.
- The teacher may compute anything (T1 amended).
- No put-down rule in the teacher.
- Merge to main allowed whenever (owner, 2026-10-21), with CI green on the merged head.
- No VMs without the owner's go.

## Gotchas this lane paid for

- **Agreement on logged decisions is not performance.** Several copies fitted held-out decisions better and did worse
  in the world.
- **Judge a kept skill with the harness that enforces it**:
  `train brain=mind challenge=fetch dist=10,30 budget=1200 holdout=96 noise=1 start=<g> score=<g>`, read the "worse
  side" line (min over each mirrored pair). The 48-world check passed a copy the trainer's floor fails.
- **A dead unit with every weight at 0 cannot learn** (zero gradient on both sides); the first revive arm reproduced
  the baseline to every digit. Seed its inputs, leave its output at 0.
- **The copier's quiet step and the revive seeding must both respect `freeze`.** Each once changed a frozen
  `f::ROOF_DROP` link.
- **An extra live link changes soil runs** (energy is billed per link) but not fetch or trail (those arenas top the
  ant up). Re-measure any genome you edit, even a "no-op" edit.
- **`deeptrace` reads its scenario relative to the repo root.** Run it from there.
- **A shell command containing `rm` inside `bash -c` is blocked by the harness's safety check.** Write to new file
  names instead of deleting.
- `scripts/docscheck.sh` exits 1 on `main` because of `deadendindex.py`'s `utcfromtimestamp` DeprecationWarning alone.
  Read its output, not the exit code.
