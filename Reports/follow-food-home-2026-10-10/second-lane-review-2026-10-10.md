Verdict: yes-with-changes

# Second-lane review of PR 675, the stack flip (2026-10-10)

Reviewer: second lane, read-only. PR head `c1941367` (= `1f4dc113` with main `cbcbc1d7` merged in). Main `cbcbc1d7`.
I could not reach the PR page or the CI checks. The GitHub API and the PR page both refuse this session (403). So
the PR body and CI status were **not seen**. I read the code with git and re-ran the sims myself in my scratchpad.
Every number below is **measured** on my own runs unless it is marked inferred. My setup was `deeptrace ... founder=evolved ants=0 census=1
set=ant.digest_hunger_weight=0`, mutation off, `RAYON_NUM_THREADS=1`, steady with `food=0`, and the report's `twelve.py` metrics.

## Required changes

1. **Fix three stale sentences in `Reports/how-the-ant-works.md`.** The doc's rule is "update the section in the same commit".
   - Line 1321 (§9): "The bar counts the food in the eight cells around the head as well as the bank (`reachable_provision`)". With
     `LAY_BAR=body` now the default, this holds only for budding species. A laying ant's bar is its body alone
     (`src/sim/creature.rs:5643`, `lay_bar_body()` branch returns 0.0 reachable).
   - Lines 1102-1103 (§6d store paragraph): "`pick=N` ... (0 by default ...)". The default is now 20 (`NestStore::SHIPPED`,
     `creature.rs:14371`).
   - Line 1554 (§12 `NEST_STORE` row): "Parts added 2026-10-06/07, none in `on`, all off". Six of those parts now ship on.
     Say "none in `on`; `pick=20,jaws,sky,meal,smell=10,edible` ship in the default".
2. **Put the per-seed rows in §7 of `Reports/follow-food-home-2026-10-10/README.md`.** Today it has only medians and
   ranges. The playbook asks for "Per-seed numbers, never only a pooled one" (`Reports/how-we-test.md:97`). The raw
   run folders are not in the repo or in `/mnt/project-files`, so nobody can check §7 from the record. Also stamp the
   build for the §7 runs. §2 says `e594d970`, and §7 says nothing.
3. **Add the 200-300k window for steady food to §7, and the PR text.** The playbook says to read 100k-300k means
   (`how-we-test.md:17,31`), and §7 stops at 200k. I ran it (table below). The steady cost does not fade after 200k;
   the starvation gap widens. Adults starved 200-300k: median 85.5 against 31, higher on 12 of 12 seeds (100-200k: 51
   against 30, 8 of 12). Ants 200-300k: 268 against 274 (-2.5%), smaller on 9 of 12. The PR should state this, not
   only the 100-200k figures.
4. **Record the mutation-on check.** The playbook requires it before a winner ships (`how-we-test.md:35-37`, "2 seeds
   to 150k"), and the report has none. I ran it (below), and it passes. Cite this file or re-run it, but the record
   should hold it.
5. **Pre-merge gate, not a code change: confirm CI is green on `c1941367`.** I could not see it. Locally,
   `cargo test --release --locked --lib` on the PR head passed: 2168 passed, 0 failed, 89 ignored. I did not run
   clippy, `branchcheck`, the bins/doc shard or the examples shard.

## 1. Does the diff do what it says? Yes.

**Defaults changed (`src/sim/creature.rs`, PR head):**
- `NeedsFirst::SHIPPED` (13719) is `{ job: false, backfill: true, ..ON }`.
- `NestStore::SHIPPED` (14371) is `on,pick=20,jaws,sky,meal,smell=10,edible`.
- `CARRY_HOME_UNSET` is `CarryHome::ON`.
- `WAY_FOOT_UNSET` is `WayFoot::ON` (`way,store`, k=4).
- `DOOR_COLUMN_UNSET` is `DoorColumn::ON`.
- `lay_bar_parse` (22288) reads unset as body. This is the only behaviour change outside the six switches, and it is
  small: `PIXEL_PHYSICS_LAY_BAR` now panics on any value other than `body|on|reach|off`, where it used to read as the
  reach rule silently.

**`job` is really off.** `store_job_waits` runs only under `rule.job` (13801), and `SHIPPED` sets `job: false`. A guard
test pins `SHIPPED == parse("hungry,laden,pack,throttle,weak,breakthrough,door,backfill")`.

**No other behaviour changed (measured).** I set the PR head's six switches back to their old values (`NEEDS_FIRST=off
CARRY_HOME=off DOOR_COLUMN=off LAY_BAR=reach NEST_STORE=off WAY_FOOT=off`). On steady seed 1 to 60k it then matches main
`cbcbc1d7` row for row: `colony.csv` and `events.txt` identical, every `stats.csv` row identical up to the final-frame
row. The food-log hooks (`note_food` at 1072, `note_digest`) return early unless a harness sets `food_log`/`digest_log`,
and they only read (`food_value` takes `&World`).

**Docs.**
- Updated: README.md:8345, the five §12 rows plus the `NEEDS_FIRST` row, a `dead-ends.md` entry for `job`, and the
  `Reports/README.md` index.
- Still stale: the three places in required change 1.
- Optional: `Reports/instruments.md` does not list `deeptrace foodlog=1` (0 mentions).

## 2. Do the numbers match? Yes, exactly where I could check.

I re-ran the arms myself, because the lane's raw files are not available.

| Check | Report | My run |
|---|---|---|
| steady main s1, 100-200k (§3c) | 296 ants, 30 starved (15/14/1) | 296.0, 30 |
| steady main s2 (§3c) | 285, 16 | 285.5, 16 |
| steady stack (`NEEDS_FIRST=on,backfill`) s1 (§3c) | 269, 118 starved | 269.2, 118 |
| steady jobless, 12 seeds (§7) | 265 (249-296); starved 51 (16-199); nest 21.2 (16.6-29.7); larvae/egg 0.14 (0.06-0.26) | 265.0 (249.2-296.2); 51 (16-199); 21.2 (16.6-29.7); 0.06-0.26 |
| steady main, 12 seeds (§7) | 284 (268-297); starved 30 (13-50); nest 1.2 (1.2-1.6); jobless smaller on 11/12, starved higher on 8/12 | 283.8 (267.9-297.1); 29.5 (13-50); 1.2-1.6; 11/12; 8/12 |
| heap 90, seeds 1-3 | main 252-336 (§3c), jobless range 600-757 | main 336, 252, 303; jobless 632, 600, 637 |

- **Main has not moved the numbers.** `e594d970` and `cbcbc1d7` give identical steady s1 runs to 60k. GRAZE_REGROW has
  no plants to act on in either bed (the dry goal box has no plants; inferred for nest_goal, measured for steady). So
  §7 stands on the current main.
- **The PR summary matches §7.** The PR summary as relayed to me (heap 90 652 vs 342, heap 30 661 vs 562, steady 265
  vs 284, starved 51 vs 30, nest 21% vs 1.2%, `job` costing about 7% at heap 90) matches §7 and README.md.
- **One wording point.** "Foragers starved 51 vs 30" is the count of all adults that starved. That they are foragers is
  traced only on the stack arm, seed 1 (117 of 118, §3c), not on the arm that ships. Say "adults starved", or label the
  forager part inferred.

## 3. Is the steady-food cost acceptable? Yes under the project bar, with traces owed.

No colony died anywhere:
- 12 steady seeds to **300k**;
- 3 heap-90 seeds to 200k;
- 2 mutation-on steady seeds to 150k.

The playbook's one hold, "kills colonies outright" (`how-we-test.md:64`), is not met. The rule's own target is ants
living in the nest, and it is met and holds: 15.7-35.9% of adults in the dug nest at 200-300k, against 1.0-1.6% on main.

Steady food, PR head (defaults) against main `cbcbc1d7`, per seed:

| seed | ants 100-200k (PR / main) | adults starved 100-200k | ants 200-300k | adults starved 200-300k | lowest count 100-300k | in dug nest % 200-300k |
|---|---|---|---|---|---|---|
| 1 | 249 / 296 | 39 / 30 | 251 / 274 | 108 / 28 | 200 / 234 | 16.7 / 1.4 |
| 2 | 265 / 286 | 16 / 16 | 272 / 289 | 84 / 26 | 206 / 213 | 21.9 / 1.0 |
| 3 | 268 / 278 | 40 / 42 | 270 / 286 | 87 / 48 | 219 / 231 | 18.7 / 1.3 |
| 4 | 261 / 268 | 76 / 35 | 256 / 234 | 33 / 30 | 223 / 185 | 25.3 / 1.1 |
| 5 | 266 / 275 | 33 / 20 | 265 / 266 | 51 / 19 | 238 / 211 | 20.5 / 1.1 |
| 6 | 274 / 293 | 44 / 44 | 274 / 275 | 81 / 25 | 239 / 192 | 25.2 / 1.6 |
| 7 | 261 / 268 | 145 / 13 | 257 / 274 | 74 / 32 | 215 / 193 | 17.5 / 1.3 |
| 8 | 265 / 282 | 40 / 50 | 271 / 286 | 96 / 44 | 237 / 237 | 22.4 / 1.0 |
| 9 | 257 / 294 | 199 / 15 | 286 / 273 | 60 / 19 | **157** / 233 | 23.2 / 1.2 |
| 10 | 296 / 281 | 58 / 29 | 281 / 272 | 120 / 40 | 242 / 220 | 35.9 / 1.2 |
| 11 | 268 / 287 | 63 / 39 | 260 / 291 | 132 / 35 | 214 / 224 | 24.7 / 1.4 |
| 12 | 263 / 297 | 116 / 25 | 264 / 279 | 114 / 47 | 213 / 246 | 15.7 / 1.2 |

**Signs toward colony death.**
- **No downward trend.** The lowest count after 200k is 200-252, against main's 185-271. The ants' mean holds from
  100-200k (265) to 200-300k (268).
- **The deaths come in bursts.** The worst 5,000-frame burst of starved adults per seed is 17-104 on the PR (median
  about 32), against 4-17 on main.
- **The outlier is seed 9.**
  - 104 adults starved in 125-129k: 102 on the surface, median x 334 (78 columns east of the door at 256, past the heap
    at 286), every one with an empty crop, median age 23.8k.
  - The colony fell from 262 to 159 (-39%) in 5,000 frames and was back to 265 by 150k.
  - This is the burst-at-the-peak pattern of §3c, at its largest. It recovered, but a crash of that size is the
    nearest thing to a colony-killing sign in the data.

**Mutation on** (steady, 100-150k):

| arm | seed 1 (mean, lowest) | seed 2 (mean, lowest) | adults starved, seeds 1 / 2 |
|---|---|---|---|
| PR | 280, 262 | 261, 203 | 36 / 164 |
| main | 285, 258 | 245, 146 | 56 / 65 |

Main seed 2 ended at 155. No colony died on either arm.

**What I want traced.**
- **Before a later default flip of anything that adds mouths on steady food; not blocking this one:**
  - seed 9's 125-129k crash: which ants, why 78 columns east, and why all at once;
  - why starvation rises after 200k on 12 of 12 seeds (inferred: the §3c overshoot, more larvae reach adulthood and
    the fixed income cannot feed the peak).
- **After merge, owed by the lane (the report's §5 already lists the first two):**
  - why the starvers go east past the heap rather than home;
  - why the stack lays fewer eggs on steady food;
  - the untraced ~7% heap-90 cost of `job` off (652 vs 702; late-tunnels 13-107 smaller on 8 of 8).

## 4. Tests and guards: adequate for the flip, with gaps

**What the guards cover.**
- Pinned by tests: `NeedsFirst::SHIPPED`, `NestStore::SHIPPED`, `CARRY_HOME_UNSET`, and `lay_bar_parse` (including the
  typo panic).
- Nine tests written against the old ant now call `stack_off` (`creature.rs:33174`), and the full lib suite passes on
  the PR head.

**Gaps (optional):**
- **No guards on two defaults.** `WAY_FOOT_UNSET` and `DOOR_COLUMN_UNSET` have no `assert_eq!` guard. Add one each, as
  for `CARRY_HOME_UNSET`.
- **`stack_off` cannot reset `LAY_BAR`.** It is process-global (`OnceLock`, with no `World` field). Every test that
  reaches `try_bud` with a laying species now runs under the body rule. Nothing failed, so inferred: either no test
  depends on the reach rule, or one passes for a different reason than it was written for. A `World::lay_bar` override
  would make `stack_off` complete.
- **No smoke guard on the shipped stack.** Nothing tests that a colony keeps living under the shipped defaults. This is
  as before; the lab runs are the guard.

## 5. Risk to other lanes; no rebase needed

**Rebase:** none needed. The PR head already merges `cbcbc1d7` (merge commit `c1941367`), and `git merge-base` with main is `cbcbc1d7`.

**Planted bed / GRAZE_REGROW (PR 674, main `cbcbc1d7`).**
- No textual overlap. GRAZE_REGROW is inert on the steady and goal-box beds (measured above).
- Risk: every planted-bed baseline from now on runs with the stack and store on. The store's `edible`, `meal` and
  `pick=20` act on what the colony eats. The planted-bed lane's numbers from before this merge are not comparable
  across it.
- **Inferred:** a store banking plant food could change grazing pressure. The planted lane should re-baseline on the
  new main before its next verdict.

**PR 673 (stranger alarm).**
- `git merge-tree` merges it with PR 675 cleanly.
- Its results were measured on `e594d970` with the old defaults (design doc line 9). After this flip, its `off` arm is
  no longer the game it measured against, including its "switch on plays byte-identical to off" finding (design §10).
- It ships off, so merging both is safe. Its numbers need a re-run on the new main before it asks for a default.

**New ant (`claude/new-ant-resume-yppx3j`).**
- **Textual conflict with this PR** in `examples/deeptrace.rs`, in two hunks: the doc-comment block near the top, and
  the argument/setup block where `foodlog` is added. The branch merges with main alone cleanly.
- Its "flip world" results (`/mnt/project-files/needs-ant/resume-2026-10-10/flip-world-results.md`) were run **store
  off** and with `job` on. The shipped world is store on and `job` off, so those results do not describe the world it
  will merge into. Its own named confound, that `NEEDS_FIRST` runs inside the walk, changes too, since `job` is now off.
- It needs a re-run on the new main, and the deeptrace conflict resolved when it next merges main.

## Optional notes

- **Twelve-seed death check.** `twelve.py`'s "colonies dead" reads only the last row at or before 199,999. A colony
  that dies after 200k would not show. My 300k runs cover steady.
- **`DOOR_COLUMN` is a labelled stopgap.** It now ships on by default. The doc row says so; keep the follow-up for a
  local replacement on the plan (`Reports/handoff/PLAN-2026-10-07.md:692`).
- **Dead-ends entry for `job`.** The entry `Reports/dead-ends.md` (creatures section) gives a useful re-test trigger,
  and the `Reports/data/dead-ends-*` churn is the generated index shifting by one entry.
- **Reproduce.** My runs are in the reviewer's scratchpad (not shared). Command per run:
  `PIXEL_PHYSICS_MUTATION=off RAYON_NUM_THREADS=1 deeptrace scenario=steady_income food=0 founder=evolved ants=0 census=1
  set=ant.digest_hunger_weight=0 seed=S frames=300000`, read with `twelve.py`'s metrics over 100-200k and 200-300k.
