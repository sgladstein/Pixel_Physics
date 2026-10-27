# E4 readiness review: evolvable pheromones (2026-10-27)

> **Status:** independent review, written by a sub-agent at the owner's request ("This is a very resource heavy test. Are we confident...", 2026-10-27 18:16Z). Verdict: not ready. Its main finding (stored cells count crumbs) and pre-flight PF2 were re-checked by the lane: see the parent report §9.8. Scratchpad paths below refer to the session that ran it; the scripts are in `Reports/data/evolvable-pheromones-2026-10-27/scripts/review2_*.py`.


Independent, adversarial. Read-only: code at main `09e338c80`, `scratch/pheromones/emerge.py`, the lane report and
its review, the plan doc (claude.ai/artifact/Vt8E28wJ3A5vjSfSuYPXHX), this week's environment and selection
reviews, the new-channel run data (`scratchpad/nc/`, `/mnt/project-files/evolvable-pheromones/newchan/`) and E3's
partial output. No simulations were run (E3 holds all 4 cores). Every number below comes from existing files; the
analysis scripts are in this directory (`review2_*.py`).

## Verdict: NOT READY. GO after five cheap changes and two pre-flight checks.

The owner asked three things. Short answers:

- **Is the environment set up in the best possible way?** No. The bed itself is adequate: there is something to
  say on it, and the third plane can carry a trail as far as B does. The problem is the **score**. "Food stored"
  counts *cells* in the larder, and nearly all of those cells are crumbs: part-eaten food put back, each worth only
  what is left. Selection on the cell count has been rewarding colonies that break food into more, smaller crumbs.
  Re-scored as energy, replicate 2's "deaf beats free 478 vs 334" becomes **157 vs 158 kJ, free higher on 7 of 16
  worlds: a tie**. Replicate 1's gap shrinks from 2.2x to 1.8x. That is the measurement E4 would hand down its
  verdict on.
- **Is the scent training set up well?** No, for E4's length. In 60 generations neither proposed operator can
  build a signal. The standard operator found conventions only after about 100 generations. The gentle one moves
  the scent-laying weights about a quarter as fast as the standard one. The binary, and with it which brain
  layout the mutation draws from, is not pinned. The founder has also changed since all the evidence E4 rests
  on was collected.
- **Is everything else ready?** No. The pass bar needs 6 of 6 replicates, so it only detects gains of roughly 30%
  or more. Yet a fail is defined as "dead end". The garden that decides pass or fail is not written yet. E3,
  which gates E4, is at generation 2 of 40 (finishes around 01:30Z).

So E4 as specified is close to **predetermined to fail**, for reasons that have nothing to do with whether ants
can invent a useful signal. A fail would close the line on a measurement artifact.

---

## 1. Environment

### 1.1 The score counts crumbs, not food (critical; new finding)

`emerge.py:109` takes `stored = nest_food + door_food` from `stock.csv`: a **cell count**. `larder.csv`, written by
the same run, also gives the energy. Almost the whole larder at frame 100k is `crumb_cells` (`prov_cells` is 0 in
every head-to-head run). Crumbs are part-eaten provisions put back (`examples/inrun.rs` ~2538), so they are worth
only what is left.

Head-to-head best genomes, 16 shared worlds (`nc/h2h1`, `nc/h2h2`), medians:

| | stored cells (today's score) | stored kJ | free higher on kJ |
|---|---|---|---|
| rep 1, free best / deaf best | 224 / 496 | 118 / 216 | 0/16 |
| rep 2, free best / deaf best | 334 / 478 | **158 / 157** | **7/16** |

And over evolution (deaf_s1, every colony of 10 generations): kJ per stored cell fell **0.55 -> 0.43 -> 0.38**
(gens 0-9, 50-59, 140-149). The free arm stayed at 0.52-0.60. About a third of the deaf arm's apparent 3.1x gain
in cells is cells getting smaller (energy rose 2.1x). The deaf best also churns (re-drops food near the nest)
7x as much as the free best: 8,132 against 1,102 events (world 700).

**Fix (5 lines in `emerge.py`, zero compute):** score on larder energy (`prov_kj + crumb_kj + other_kj` from
`larder.csv`). Keep cells, tank, starved and alive as logged columns. **Before anything else, re-score the
new-channel runs in kJ.** It is pure Python on files already on disk, and it may change what the line's history
says.

### 1.2 "Forage better" is not one number here, and the plan does not say which one decides (high)

Free best against deaf best, rep 1, 16 paired worlds (`review2_pair.py`):

| measure | free better on | |
|---|---|---|
| stored (cells) | 0/16 | deaf 2.2x |
| deliveries (the game's own count) | 15/16 | 2,514 vs 1,405 |
| tank (fed level, second half) | 12/16 | 8.0 vs 5.2 |
| eggs laid | 13/16 | 200 vs 163 |
| starved | worse on 12/16 | 169 vs 134 |

Rep 2's free best is worse on every measure except eggs: tank 3.7 vs 30, alive 24 vs 75. Rep 1's free colony
brought more food home and fed its ants better, but kept less in the larder. The standing rule (colony-selection
log) is to judge feeding by tank, stored food and starved, never by one of them alone.

With `HARNESS=2`, an extinct colony also keeps its larder score (`emerge.py:128-129`), so with a stock-only score
nothing penalises "store, then die".

**Fix:** pre-register one primary measure (stored kJ) and two guards: starved, and alive at the end, neither
worse (sign test across replicates). Put the owner's choice of what "forage better" means in writing before the
run.

### 1.3 Is there something to say on this bed? Probably yes, but the evidence is weaker than reported (medium)

- **For.** Deafening the *old* founder to B costs 231 -> 128 stored (12/16): trail information pays on this bed.
  E2 found 56-65% of empty foragers off the door-to-heap corridor after a nestmate has found the heap.
- **Against.**
  - The world is a side view, so foraging is along the surface: effectively one-dimensional. The information
    worth sending is mostly *which side* and *how far*, which a B trail already carries. The deaf arm can improve
    B too.
  - E2's "lost searchers" counts idle ants, not lost food. It has no positive control, as the lane says.
  - The evolved deaf bests leave loose food lying in the field: median 36-45 cells, at least 55 cells for 25-33%
    of the time. The founders keep the field nearly bare (at most 5 cells for about 70% of the time). Some of
    that food is dropped crumbs (the deaf best drops food in the field 5,649 times on world 700), so this is not
    clean. But it suggests the evolved deaf colony is not limited by finding food, which is the only thing a
    location signal can improve.
- **Not measured at all:** whether trail information still pays for the **current** founder. See PF2.

### 1.4 Can the third plane carry it far enough and long enough? Yes (low risk)

- **Lifetime.** Ground law with fade 0.03 per 12-frame pass gives a half-life of about 23 passes (~275
  frames). One laid mark is at ~5% after 1,200 frames. A one-way trip to 110 columns is ~500-550 frames (§6:
  463-525 frames to reach food at about 100 columns). So a trail survives with traffic, as B does.
- **Width.** The 0.05 blend is the lab's narrow food-trail pair, measured better at holding a thin trail than
  the engine's 0.25 (`pheromone.rs` `LAB_B_DIFFUSE` doc: 10-46/255 along a 40-cell path).
- **Ceiling.** The soft ceiling applies to this plane too (`PheromonePlane::deposit`, `pheromone.rs:662`).
- **One caveat.** The molecule has **the same lifetime as B** (same fade), so it offers no new timescale
  ("food found in the last few hundred frames", or "stale"). It is a narrower B. That is fine for the question
  "can a new convention form", but it caps what a third channel can add over B.
- **Cheap check, if wanted:** PF3 below.

### 1.5 Does the deaf control isolate the third channel? Partly (medium)

- **Reading is fully cut.** `PP_SCENT_DEAF=alarm` zeroes head inputs, their changes and per-option features
  (`options.rs:328-331`, `391-403`). The dig head reads only A (`targets.rs:320`). The non-mind brain's
  `I::Alarm` (`creature.rs:9671`) is never reached by mind ants, which return at `creature.rs:8126` via
  `mind::tick`. Nothing else writes the plane on this bed: no bites, no strangers.
- **Emission is not cut.** Both arms set `PP_MIND_ALARM=train` (`emerge.py:55`), so deaf ants still lay the
  plane and pay for it. That was priced at about 2% of the earlier gap (`PP_EMIT_FREE`).
- **Not equal, by design:** in the deaf arm the ~50 inherited alarm-read links and the 35 emit weights are inert,
  so its children take less mutation damage. E1 shows the hearing genomes are more fragile, mostly outside the
  channel. So E4 tests "information minus the extra mutation load", and a fail cannot tell "worthless signal"
  from "too costly to evolve with this operator". §2.2 is the fix that makes the load small. A third arm that
  hears but cannot use location (for example `PP_SCENT_SHIFT=alarm:40` throughout) would isolate the load, but
  costs +50%. Not recommended now.
- **Both arms evolve A and B** (review §9.8). So the result is "a third channel against two evolvable ones".
  Say so in the verdict.

### 1.6 Harness-only rules from the environment review: mostly irrelevant to *this* question (low)

`lay_cap`, no old age, `grants=3`, the 40-row floor and no windfall all apply to both arms alike, and none plausibly
interacts with signalling. Two notes:

- E4's bed keeps the game's door ban (no `PP_MIND_DOOR_OPEN`), but the current std_pool was trained with the door
  drop on a 448-wide bed. Both arms spend early generations re-adapting. Acceptable; keep it (it is the game's
  rule).
- The 100k round under-penalises the fragility of signalling colonies: crashes below about 15 ants, and the
  environment review's 0/16 alive at 600k. If E4 passes, re-garden the winners at 300k before any claim
  (~15 core-hours).

## 2. Scent training

### 2.1 60 generations cannot build a convention with either operator (critical)

**What the earlier runs needed.**

- Free arm median stored at gens 50-59 of the earlier runs: **192 vs deaf 400** (rep 1), **222 vs 331**
  (rep 2). Rep 2's free arm reached deaf parity only by about gen 150.
- Conventions became load-bearing at about 100 generations (§9.7). Rebuilding B *with its readers intact* took
  about 40 (§9.4).

**How far each operator moves the scent-laying weights.** `neutral_mutate` (`inrun.rs:155-177`) moves only
non-zero weights, each with probability `pm`, by `sigma x U(-sqrt3, sqrt3)`.

- *Standard (sigma 1).* The 35-slot emit row takes about 0.45 hits per generation along the lineage. That is a
  total movement (L1) of about 23 in 60 generations. The evolved rows that carried conventions moved
  **L1 = 20** from the dense start over 150 generations (`review2_row.py`). The deaf arm's inert row drifted
  15-16 in the same time, so most of that movement is drift.
- *Gentle (sigma 0.25, 1 event).* About 6 in 60 generations, roughly a quarter of the standard operator's
  movement.

So under gentle the channel barely moves in 60 generations. Under standard the earlier runs show the free arm still
paying for convention formation at gen 60. **"MUT standard or gentle" is also not decided**, which is a forking
path.

**Fix:** a **channel-weighted operator**: gentle everywhere except the channel's own slots, which keep the
standard rate and sigma. The slots are the alarm emit row 2410-2444 plus the ~50 alarm-read slots
(`layout.py`: context inputs 6 and 9 of every scorer, job, memory, emit and hidden row; features 15-17 of every
scorer unit). E1 already shows this is the right split. Channel-only mutants keep 1.00-1.08 of their parent in
2 of 3 hearing genomes; "everything else" carries most of the load. Both arms get the same operator; in the
deaf arm the channel slots are inert.

- *Code.* `neutral_mutate` already takes a per-slot rate mask (`prot: Option<(&[bool], f32)>`). `mutate_to`
  passes `None` (`inrun.rs:882`). Add a `chan=LO-HI,...:FACTOR` field or an env var, plus an emerge.py option.
  About 20 lines.
- *Pre-flight.* An E1-style check: 12 mutants x 4 worlds x 3 genomes, about 1 core-hour.

### 2.2 The binary and brain layout are not pinned (high; silent)

- E3 is running `scratchpad/nc/inrun2.bin`, built 2026-10-26 01:36Z, before layout 6 (`7e546730e`, 20:58Z) and
  layout 7 (`edf8f9a52`, 23:06Z). It has no `PP_GATE` string.
- A fresh build from main makes the gates and reach rows **trainable by default** (`gate_on`/`reach_on`,
  `mind.rs:2264-2275`). The layout-5 founder enters with them at zero, so it scores the same. But the mutation
  operator's add events then draw from about 1,290 + 812 zero slots. **About 40% of all add events would land
  in gate and reach rows.** A gate weight of about 1 can halve or shut a scorer unit (`clamp(1+y, 0, 2)`).
- That is a new, unmeasured mutation load in both arms. E1 and E3 measured neither.
- The E4 command says `BIN=<inrun binary>`.

**Fix:** run E4 on exactly E3's binary, or on main with `PP_GATE=off PP_REACH=off` (`PP_WRITE` is off by
default). Pre-flight PF5 checks that the two agree bit for bit on the founder. Do **not** move E4 to layout
6-8 or to a newer founder now. That would be a different experiment with no baseline, and nothing shows a gated
founder signals differently.

### 2.3 The founder changed under the evidence (medium)

- `assets/minds/std_pool.txt` was replaced at 00:45Z 2026-10-26 (`c87fd03c6`, nest7 colony 13). The new-channel
  runs, E1 and E2 all used the **previous** pool (price25b colony 10; `nc/founder.raw.txt`, 1,992 live links).
- E3 and E4 use the new one: 2,034 links and about **50** alarm-read links rather than 39. Among them is a
  +1.39 weight from the *change* in alarm into the **B emit row** (slot 2384). So in the free arm the random new
  emitter feeds straight into how the founder lays its food trail. The old founder had that link too.
- **Using the new founder is fine**, and it matches E3. But "the founder showed no reliance at the start" and
  "the bed leaves room" were measured on the old one. PF2 re-measures both on the new one.

### 2.4 Readers and seeding (low; a choice, not a defect)

- The emit row is seeded dense, N(0, 0.05), which is right (§9.3 against §9.4).
- The readers are inherited weights trained when the plane meant "empty and searching" on the air law. They are
  both an asset (somewhere to start) and a confound (stale couplings, as above).
- Zeroing them and refilling them densely would make the convention de novo, but adds about 144 small live links
  to the free arm.
- Keep them, and say so in the record.

## 3. Harness and statistics

### 3.1 Power (critical for how to read a fail)

- **Spread between replicates.** The two existing replicate pairs give log(free/deaf) of the top-4 mean at gens
  50-59 of -0.49 and -0.20, so the spread between replicates is about 0.21 in log units.
- **Noise within a garden.** Hearing genomes are far noisier from world to world (coefficient of variation
  0.29-0.50) than deaf ones (0.04-0.11). So the free arm's ranking is noisier, and its garden needs more worlds.
- **What the 6/6 bar can detect.** It needs a per-replicate win probability of at least 0.96 for 80% power,
  that is a true gain of about 1.8 replicate SDs: **about 35%**. A paired t-test on log ratio over 6 replicates
  detects about 25-30%.
- **So a fail cannot support "dead end"** for gains of 10-20%. And a fail is the expected outcome at 60
  generations (§2.1).

**Fix:** three outcomes, fixed in advance.

| Outcome | Condition |
|---|---|
| Pass | ≥5/6 replicates and one-sided paired t on log(free/deaf) of garden kJ at p < 0.05; guards not worse; reliance check passes |
| Dead end | The 90% CI upper bound of the free/deaf ratio is below 1.10 |
| Inconclusive | Otherwise; name in advance the next step it would trigger |

### 3.2 The decision garden does not exist yet (high)

No script in `scratch/pheromones/` gardens "each replicate's top 4", and the bar does not say:

- how many worlds per genome;
- which seeds (they must be disjoint from training `SEED*1000003+...` and listening `900000+k+1000g`);
- how "top 4" is picked. By the H2 running mean, the top 4 includes the two newest mutants on 3 draws: a winner's
  curse;
- which "free best" the 20% reliance bar applies to.

**Fix:** write it before launch. Garden all 16 final-generation genomes x 4 shared worlds per arm, plus the top 4
by running mean x 8 worlds, at seeds from 5,000,000. That is about 1,150 runs (~7 core-hours). Commit it with the
pass bar.

### 3.3 emerge.py, read critically

**Correct:**

- Seed pairing across arms (same `SEED` gives the same gen-0 mutants, worlds and mutation streams).
- Seeds are unique within a run for GENS < 99 and EVALS < 10.
- `ZERO=2410-2444` is still the alarm emit row in layout 8, because the later layouts append (`mind.rs:296-328`).
- The duplicate-row fix and the gen-0 founder.
- In-run selection off means clones (`inrun.rs:1775-1781`).

**Remaining issues:**

1. **`hist.json` is written before `done`** (`emerge.py:184` against `:217`). A kill between them makes the resume
   append the same generation's score again. That double-counts one draw for every genome in that generation.
   Fix: write `hist.json` after `done`, or key entries by (genome, generation).
2. **`score()` deletes `events.csv`** (`:133`), so deliveries, field pickups, churn and field drops can never be
   re-scored. Fix: count them (and tank mean and larder kJ) into `gens.csv` before deleting. The kJ finding above
   was only possible because `larder.csv` survived.
3. `parents_of` is not saved, so a resumed generation's parent column is blank. Cosmetic.
4. `mutant()` runs serially in the main loop: 14 subprocesses per generation, each building `colony_bed::suite()`
   before it mutates (`inrun.rs:849`). On the 64-core VM with P=16 that idles cores between generations. Time
   one call; if it is over 1 s, put them in the thread pool.
5. `SCORE=stored` with `HARNESS=2` gives no penalty for extinction (§1.2).

### 3.4 Cost and schedule

- **Throughput.** E3 runs at about 21.5 s per 100k-frame run with 4 runs on 4 cores (the new founder grows bigger
  colonies than the old one's 10.3 s). E4 is 12 arm-replicates x 60 x 16 x 3 = 34,560 runs plus listening, about
  **210 core-hours**, which matches the plan's 240.
- **A better use of the same compute.** `EVALS=2` (the H2 running mean already averages elites) at **90
  generations** costs the same and gives 1.5x the generations. That matters more than the third evaluation,
  given §2.1. E3 already runs EVALS=2, so it is the matching baseline.
- **Timing.** E3 finishes around 01:30Z (11.6 min per generation, at gen 2 of 40). E4 should not launch before E3
  is read, re-scored in kJ.

## 4. Changes, ranked by value for cost

| # | Change | Cost | Why |
|---|---|---|---|
| 1 | Score on larder kJ; log tank, starved, alive, deliveries, churn and kJ per generation (`emerge.py` `score()`) | 15 min code, 0 compute | The cell count rewards crumbs. Rep 2's gap is a tie in energy |
| 2 | Re-score newchan and E3 in kJ (PF1) | 0 compute | It may rewrite §9.5-9.7, and it calibrates E4's effect size |
| 3 | Pin the binary: E3's `inrun2.bin`, or main with `PP_GATE=off PP_REACH=off`; write the exact E4 command into the report | 0 (+PF5) | Otherwise about 40% of add events land in untested gate and reach rows |
| 4 | Channel-weighted operator: gentle elsewhere, standard on the alarm emit and read slots; drop the either/or | ~20 lines + ~1 core-h check | 60 generations with gentle cannot move the channel; standard carries the off-channel load (E1) |
| 5 | EVALS=2 with 90 generations, not EVALS=3 with 60 | 0 (same budget) | Prior runs were still paying for convention formation at gen 60 |
| 6 | Pre-register a three-outcome rule; write and commit the garden script | 1 h code, ~7 core-h at the end | 6/6 detects only gains of about 35%; a fail must not mean "dead end" for 10-20% |
| 7 | Fix the `hist.json` ordering and keep event counts | 15 min | Silent double-counting on resume; lost re-scoring |
| 8 | If it passes: a 300k-frame garden of the winners | ~15 core-h | Signalling colonies are fragile; the game runs 200-670k frames |

## 5. Pre-flight checks, before the 240 core-hours

- **PF1. Re-score in kJ (0 compute, now).** Run `python3 review2_pair.py` and the kJ snippet in this review on
  `nc/h2h1`, `nc/h2h2` and `nc/*_s1/g*`. Extend to E3 when it finishes: `larder.csv` is kept in every `r*/`
  directory.
- **PF2. Does trail information still pay for the current founder on this bed? (~0.5 core-h, no code).** Use E3's
  unmutated founder (`e3/h2_s31/founder.raw.txt`) on 16 fresh worlds, in three arms: normal,
  `PP_SCENT_DEAF=B`, and `PP_SCENT_DEAF=A,B`. Score stored kJ, tank and starved.
  ```
  F=$E3/h2_s31/founder.raw.txt
  for s in $(seq 7001 7016); do
    for arm in normal B A,B; do
      env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05 PP_MIND_ALARM=train \
          $( [ $arm != normal ] && echo PP_SCENT_DEAF=$arm ) \
          $BIN founders=$F:30 seed=$s out=$OUT/$arm/s$s \
          sel=off frames=100000 dist=100 food=mix patches=20000:30:110 maps=0
    done
  done
  ```
  If deafening B costs under about 15% here, a location signal has little to win on this bed. Then change the
  bed (faster or farther heap moves, which E2 says keep the room) before E4, not after.
- **PF3. Can the third plane carry the founder's own food trail? (~0.5 core-h, a 30-line genome edit).** Copy the
  founder's B emit row (2375-2409) onto the alarm row (2410-2444). Copy every B-read weight onto the matching
  alarm-read slot: context input 5 to 6 and 8 to 9 in every scorer, job, memory, emit and hidden row; features
  12-14 to 15-17 in every scorer unit (`options.rs:1225-1227`). Run with `PP_SCENT_DEAF=B`. Compare against
  `PP_SCENT_DEAF=B,alarm` and normal on the same 16 worlds. If the transplanted trail recovers most of B's value,
  the plane's physics is not the limit.
- **PF4. A positive control for the whole pipeline (~10 core-h pilot; ~40 for 2 replicates).** The same E4
  harness with **B deafened in both arms**: free = third plane heard, deaf = third plane deaf. The prize for
  inventing a trail is then the whole of B's value (old founder: 231 -> 128). One replicate, 40 generations,
  with change 4's operator. If the free arm cannot pull ahead here, E4 cannot detect a smaller prize; fix the
  operator or the length first. This is the check the plan is missing: every earlier run tested the hard case
  without ever showing the machinery can find the easy one with a new channel.
- **PF5. Binary agreement (~2 min).** Founder, 2 worlds, `inrun2.bin` against main built with
  `PP_GATE=off PP_REACH=off`: `md5sum */stock.csv` must match. If not, list what changed between the two builds
  before choosing.
- **PF6.** Read E3 when it finishes, in kJ as well as cells. Its bar (at least 1.5x gain per 10 generations) on
  2 seeds is weak. Treat it as "H2 does not hurt" unless the gain is large.

## 6. What I could not verify without running

- Whether stored kJ, rather than cells, flips rep 1 at the arm level: only the best genomes were checked.
  `larder.csv` exists for every colony in `nc/free_s1` and `nc/deaf_s1`. Running `review2_field.py`-style over
  `larder.csv` answers it with no compute; I did gens 0-9, 50-59 and 140-149 only.
- Whether the loose field food in deaf-evolved colonies is the heap or dropped crumbs. That needs `map_f*`
  snapshots, which `score()` deletes. Run one deaf best with `maps=1` on 4 worlds.
- The real wall time of one `mutate_to` call.
- Whether a gated founder (layout 6-8) would signal differently. Nothing suggests so; it is a separate experiment.
