# Handoff prompt for the merge-run session

## Context

The water-phase-changes branch is finished and pushed: **62 commits, tree
clean, 697 tests, clippy clean at `-D warnings`, 20/20 acceptance cases**.
It is **226 behind** `origin/master` and 15 files conflict. The owner is ready
to start a merge run in a fresh session, so what follows is the prompt to
give that session — written to be read cold, carrying the measured facts,
the guardrails, and the traps that cost real time getting here.

Everything below the line is the prompt itself.

---

# Merge run: `claude/water-phase-changes-ki6g8c` into `master`

You are picking up a finished feature branch and merging it. The work is
done and judged; **your job is to land it without losing any of it**, not
to improve it.

## Standing rules — these are the owner's, verbatim, and they still hold

> Work in a worktree if you spawn agents (`.claude/worktrees/` convention;
> agent worktrees have cut from stale bases before — verify HEAD against
> `origin/claude/water-phase-changes-ki6g8c` and rebase before starting).
> Stage explicit paths, never `git add -A`. Commit messages carry the
> numbers. Run `cargo test` + `cargo clippy --all-targets -- -D warnings`
> before any push; `scripts/acceptance.sh` before pushing anything that
> touches structural or weather behavior. Push only to
> `claude/water-phase-changes-ki6g8c` unless I explicitly direct otherwise.

Read `CLAUDE.md` first — all of it. It is about *method*, not code, and
every rule in it was bought with a real failure.

Two notes on the rules for this particular job:

- **The branch name in your harness brief may say `...-amsy3d`. It is
  wrong, and here is the evidence rather than an assertion:**
  `git ls-remote --heads origin` shows **no such branch has ever existed**,
  while `claude/water-phase-changes-ki6g8c` exists and holds all 62
  commits — **29 of which predate this line of work's current session**
  (dated 2026-08-20). So the work lives on `ki6g8c` as a matter of record,
  not of anyone's say-so. Re-run that `ls-remote` yourself before trusting
  it. Flag the discrepancy to the owner; do not create or push `amsy3d`.
- **Do not merge to `master` without the owner saying so in this session.**
  "Get ready for merge" is not "merge". Ask.

## The state, measured — **and re-measure it before you use it**

`master` moves fast: while writing this handoff, a stale local
`origin/master` made three separate figures below wrong, including one that
inverted a decision (see the plant section). **Your first command should be
`git fetch origin master`, then recompute everything in this section.** This
is `CLAUDE.md`'s "always re-measure the baseline in the same session"
applied to git state, and it bites here for real.

As of `origin/master` = `c7cdcdc`:

```
git rev-list --left-right --count origin/master...HEAD   ->  226  62
```

Conflicts, from `git merge-tree --write-tree --name-only origin/master HEAD`
(read-only; it computes the merge without touching your tree — prefer it to
a `git merge --no-commit` dry run):

| file | this branch | master | note |
|---|---|---|---|
| `src/render.rs` | 1158+/37− over 11 commits | **1799+/43− over 20** | the hard one, both sides large |
| `src/sim/field.rs` | 625+/5− | **1103+/64−** | master rewrote it for performance |
| `examples/filmstrip.rs` | **2052+/10−** | 371+/11− | mostly additive on our side |
| `Reports/open-bugs-handoff.md` | 829+/20− | **1375+** | append-only both sides; take both |
| `src/app.rs` | 21+/1− | 380+/15− | contested file, ours is small |
| `src/main.rs` | 10+ | 249+/2− | ours is small |
| `src/sim/material.rs` | 240+/2− | 250+ | |
| `src/sim/update.rs` | 169+/23− | 34+/1− | |
| `README.md` | 88+/17− | 272+/46− | |
| `assets/materials/water.ron` | 28+ | 30+/1− | |
| `.gitignore` | 5+ | 6+ | trivial, take both |
| `wiki/{weather,fire-and-heat,liquids-and-gases,structural-collapse}.md` | 68–158+ each | 1–14+ each | ours dominates; keep both sides' facts |

**Take the conflicts in ascending order of size**, and commit the merge only
once every one is resolved and the gates pass. `render.rs` and `field.rs`
are the two that need real attention; everything else is small on at least
one side.

## The trap that will actually bite: files that auto-merge but change meaning

Git resolving a file cleanly is not the same as the result being correct.
Three specific ones, all of which merge without a conflict marker:

1. **`src/sim/field.rs`'s `rebuild_blocked`** — master has performance work
   on exactly this function (`6360023 rebuild_blocked: one registry fetch
   per cell`, `0b28af8 Don't rescan rock that hasn't moved`). Our newest
   finding (open bug **1m**) is *about* this function's moisture-source
   computation. After merging, re-run the 1m measurement; the code it was
   measured against has moved underneath it.
2. **`src/sim/world.rs`** — auto-merges, and master has **22 commits** on
   it. We add two fields (`dryness_counts`, `splash_sites`). Check the
   struct *and* its initialiser both survived; a field added in one and
   missed in the other will not compile, but a field silently dropped from
   both will.
3. **`src/sim/structural.rs` (6 master commits), `fire.rs`, `parallel.rs`,
   `load.rs` (1 each)** — all auto-merging, all load-bearing for the
   rigid-body, freeze/melt and structural work here. `fire.rs` in
   particular holds our `MELT_CHANCE`, the simmer pop and
   `LATENT_HEAT_DEGREES`. `MAX_REACH == CHUNK_SIZE / 2` underpins
   `parallel.rs`'s cross-chunk write-safety proof *and* its
   reinsert-then-replay loop; if master touched either, both need
   re-deriving.

`CLAUDE.md` records the general form of this: *"A commit message is not
evidence the change is in the file... After any stash, rebase or merge,
re-read the function, not the diff."* Do that for every behaviour listed
below.

## What must survive — check each by its guard, not by reading the diff

This branch shipped these, each judged by the owner in play. If the merge
loses one, the tests below are how you find out. Run
`cargo test --lib` and confirm all of these are present and green:

| behaviour | guard |
|---|---|
| Rock reaches the pond floor as rock, not grit | `a_slab_that_sinks_arrives_as_rock_rather_than_as_powder`, `a_body_leaves_no_reservation_behind_when_it_settles` |
| A body displaces water without losing any | `a_body_sinking_through_a_pond_conserves_the_water_it_displaces` |
| Big pieces sink faster than small ones | `a_big_piece_sinks_faster_than_a_small_one` (0.82 / 1.34 / 1.90 for 3, 8, 16 across) |
| A pond freezes over in 1–2 min and holds | `a_freezing_pond_is_not_a_churning_slush`, `a_freeze_and_a_thaw_take_the_time_a_player_can_see` |
| Cold bites with nothing falling | `a_clear_cold_night_freezes_a_pond_and_a_clear_mild_one_does_not` |
| Ice thickens fast-then-slow; snow insulates | `a_long_cold_spell_leaves_water_under_the_ice`, `a_drift_on_the_ice_slows_the_freeze_underneath_it` |
| Bubbles read as steam, not tinted water | `a_bubble_is_drawn_nearer_steam_than_water` |
| Two bubble sizes, `H` cycles five modes | `a_large_bubble_field_holds_both_of_its_sizes` |

The full list is `git diff $(git merge-base origin/master HEAD)..HEAD -- src/
| grep -E "^\+\s*fn (a_|the_|every_)"`.

**`src/render.rs` is where the greatest risk sits**, because both sides are
large and ours is the bubble work. After resolving it, re-run the bubble
guards *and* look at a rendered pan — a colour change that passes a
threshold test can still look wrong.

## Then re-run the acceptance suite and read it, not just its exit code

`bash scripts/acceptance.sh` — 20 cases, ~10 minutes. It gates the
structural and weather scenes and its expectations are already set with
headroom. But `CLAUDE.md`'s warning applies: all eight cases once stayed
green through a change that ate fifty times more world than the bug it
fixed. Read the census lines under `rockdrop`, `lavadrop`, `coldsnap` and
`coldsheet` and compare them to the numbers in the commit messages —
particularly surviving stone (`rock -178, rubble +127` on `rockdrop`), the
ice churn ratio (1.0), and `sheet:` thickness (5.3 mean, 10 max).

## Plant-branch coordination — the sequencing is already resolved

**The plant branch has landed.** `946b858` ("The water-cycle branch builds
the half this one is missing -- and it lands second") is an ancestor of
`origin/master`, and `plant.rs` there carries `absorb_water`,
`settle_water` and `plant_available_fraction`. `plant-integration`,
`main` and `master` all point at the same commit. So the agreed order has
happened and **we go next** — no need to ask.

(An earlier draft of this handoff said the opposite, from a stale
`origin/master`. That is the concrete reason the section above tells you
to fetch first.)

These are now live and are *ours* to fix as part of this merge:

- **`plant.rs:83`** (`if cell.material == material::EMPTY`, still present
  on `origin/master` — verified) and `Germinate`'s resting test use raw
  `material == EMPTY` where
  `FLAG_MANAGED` requires `is_empty()`. A reserved cell reads as free and
  a plant grows into a rigid body's footprint. Our own
  `evaporation.rs` sites (282, 400, 751) already use `is_empty()` and are
  correct — do not "fix" them.
- **Re-derive `SOIL_SOAK_PER_DROP`, `STORM_RESERVE` and
  `SOIL_DRY_PER_CHECK` against a planted world.** They were calibrated on
  plantless worlds, and plant consumption is a one-way exit from the
  conserved cycle. Re-run `probe_long_run_balance` and `probe_storm_yield`
  after the merge. This is no longer conditional — the planted world is on
  master now, so the probes can be run the moment the merge builds.
- The plant agent's corrected worldgen baselines: wetland 570, rolling =
  default = 403, canyon 279, arid 49 (moot — sand carries no capacity).
  **Do not give `seed.ron` a `water_capacity`**: `rebuild_blocked` takes the
  MAX over an 8×8 block, so one resting seed with a small capacity would
  make its whole block read wet.

## Open bugs recorded this session, deliberately not built

Both are in `Reports/open-bugs-handoff.md`. Neither should be picked up
during the merge run.

- **1l — boiling never puts a bubble *in* the water.** Measured: steam with
  water directly above it is 0 across a whole `lavapour` run against 104
  steam cells at peak; `lavadrop` 0/3/0/0/0 against 496. The engine
  converts at the hot face and vents upward; nothing forms at a floor and
  rises. The drawn bubbles have been standing in for a mechanism that does
  not exist, and the owner has said so plainly. Entry names what to check
  first and two things not to re-derive.
- **1m — damp-soil evaporation barely runs, and the humidity shadow that
  would switch it off is already here.** Zero soil checks in five of six
  sweep runs; the one that ran was becalmed 58% of the time. The counter
  cannot yet attribute cause — seed 2900 reads 99% and is the coldsnap
  seed, so that is probably legitimate weather humidity, not soil.

## Method notes, all of which cost real time on this branch

- **Look at the artifact before and after.** Three fixes here passed their
  tests and changed nothing on screen. `examples/filmstrip.rs` writes a
  contact sheet or, if `out` ends in `.gif`, an animation at true speed.
- **A contact sheet spanning a day or more now pins its lighting to noon.**
  Without that, tiles land at different points in the day/night cycle and
  adjacent tiles differ in brightness for no reason — which was read as a
  physics difference once already. `phase=off` shows the real sky.
- **"Did it fire at all" needs a counter, not a picture.** Two mechanisms
  look identical at the zoom a sheet is read at.
- **Check a knob is connected before trusting a sweep.** `SNOW_INSULATION`
  swept at 1, 4 and 8 gave bit-identical output, because the scene never
  presented the case. Identical output across settings means the knob is
  not wired to anything reachable.
- **Compare two runs, never one run against a remembered number.** A 25–50%
  "regression" here turned out to be the machine, twice.
- **Filter measurements to the state they are about.** A sink-speed peak
  taken over a body's whole life is its entry speed through *air*; it took
  two attempts and a six-cell submersion margin to measure the actual cap.

## Verification, before any push

1. `cargo test` — expect 697 across all targets.
2. `cargo clippy --all-targets -- -D warnings`.
3. `bash scripts/acceptance.sh` — all 20, and read the census lines.
4. Re-read the merged bodies of `rebuild_blocked`, `drag_through_liquid`,
   `hold_column_cold` and the bubble blend in `render.rs` — the functions,
   not the diff.
5. Render one pan of `scene=simmer bubbles=large` and one `scene=rockdrop`
   drop, and look at them.

## Talking to the owner

They judge visuals through the review-card queue
(`.git/pixel-physics-review/bin/review.py`); post with
`review.py post --json -`, and the field that matters in the result is
`"owner_can_see_it": true`. Posting is fire-and-forget — post and keep
working, pick verdicts up with `inbox` later. Put discrete event counts in
the card's `meta`. Heredocs with backticks in them will execute as shell
commands and mangle the card; put card JSON in a Python file instead.

They want to be told what a change *cost*, not only what it bought, and
they have overturned three models that looked correct in tests. If a
merge resolution makes you choose between two behaviours, show them both
rather than picking.
