# Breeding lane handoff (written 2026-10-02 22:05 UTC, main = 0738a8ca)

For a fresh Claude with no context. Repo: sgladstein/Pixel_Physics. Branch: `claude/ant-breeding-plan-v9kpl5`.

## What the lane owns
How ants reproduce: eggs/larvae/pupae (`src/sim/brood.rs`: `lay_egg`, `brood::nurse`), the birth bar and laying rules in `try_bud` (`src/sim/creature.rs`), `creature::kin_deficit` (larva hunger, the nest lane's larva-scent work must read it), and the breeding knobs. Plan of record: `Reports/ant-breeding-plan-2026-09-29.md` (biology table ~line 465). Living ant reference: `Reports/how-the-ant-works.md` (§9 breeding, §12 switches); update it in the same commit as any mechanism change. Wiki: `wiki/ants.md`.

## Scottt's goal and binding rulings
- Goal: a stable colony that breeds less, builds a food supply and survives long term. Small colonies are fine. Judge by long-run survival and food stock, not size or fruit alone.
- Never answer how real ants work from memory: check repo reports, do web research, cite papers.
- Features default ON unless a measured harm. Lay-only-at-the-nest: Scottt said "it should be on" and chose "Turn on now" knowing the lab box dies.
- Lanes push branches and open draft PRs, never merge. Hand a PR to the coordinator (for the merge desk) as soon as it is done, not after green.
- No full test suite before a PR: fmt (not project-wide `cargo fmt`), clippy, touched-area tests. Never `git add -A`. Never start a new thread without Scottt's yes.

## Merged and default state (all via the merge desk)
| PR | What | Default |
|---|---|---|
| 542 | brood mechanism, nursing by touch (`PIXEL_PHYSICS_NURSE`) | on |
| 543 | brood replaces budding (`PIXEL_PHYSICS_BROOD=off` for budding) | on |
| 544 | egg and pupa stages 250 frames each; `PIXEL_PHYSICS_BREEDING_MAX` dial | — |
| 545 | graded fertility: bar rises up to 1.25x beside a breeder | on |
| 546 (main 0738a8ca) | lay only at the nest (`PIXEL_PHYSICS_BUD_SITE=anywhere` to undo); trailfollow `bankdump=` and EATING line | on |

## Unmerged work on the branch (all pushed, head 231ab3d1, clean tree, no PR yet)
- d315658a + c2214bf4: **food brake** (`PIXEL_PHYSICS_FOOD_BRAKE`, OFF). Birth bar raised when colony income/burn EMA (window 3,000 frames, `World::colony_pace`, `step_colony_pace`) falls below 1.5, full stop at 1.0; colonies under 30 adults exempt, full at 60. Result: mixed at gap 90 (one seed crashed), neutral at gap 200, food box steadier but no store forms. Keep off; not a win.
- f6accdf9: **walk home to lay** (`PIXEL_PHYSICS_LAY_HOME`, on unless `off`). `ready_to_lay()` makes `home_pull` aim home when an ant can afford to lay. Built to rescue the lab box after PR 546. **Lab result, 12 seeds: not a rescue** (births median 4 -> 7, died out 10 -> 10 of 12). 231ab3d1 adds opt-in `LAY_HOME=laden` (also no help). Run in flight: scratchpad `layhome/` (lost with the container; rerun if needed). Do not PR until it works.

## The open problem
Since PR 546 the lab box (`labforage scenario=played_bed frames=120000`, 12 seeds) dies: births 329 to 4, 10/12 dead. Ants that can afford to lay are a median 9 cells from the nest and adjacent to it only 3% of samples. Nest lane's dug home does not fix it (0-2/12 alive). Trace so far (labforage `budtrace=`, seed 1, 30k frames, f6accdf9): only ~15 ants ever clear the bar; 90% of their ready samples are still carrying food in the crop (so `ready_to_lay`, which requires an empty crop, never pulls them), and they circle 4-10 cells from the tiny painted nest, adjacent to it in under 3% of samples. At the door the cells round the head are crumbs and ants, so `lay_egg` (reach 1) sometimes finds no empty cell; `PIXEL_PHYSICS_LAY_REACH=3` removed those refusals but did not raise eggs (3 vs 8). Running: the `PIXEL_PHYSICS_NEST_REACH=r6` oracle on lab seeds 1-4 (a bigger "at the nest" zone; affects deliveries too, so an oracle only). Candidate fixes: let laden ready ants count, or define "at the nest" by the nest site's footprint rather than touching painted nest material.

## Next steps in order
1. Fix lab laying (above), then re-check long runs and the food box, PR it.
2. Re-test the appetite gate (`digest_hunger_weight`, trailfollow `hungergate=`) once the nest lane's dug home lands. Last test (6a8dacd4): fruit -40% at gap 90, 4/6 died at gap 200. Rejected.
3. Food store: none forms. Ants bank eaten food as energy with no ceiling; 1-7% of digestion happens at the nest; mouth-to-mouth sharing moves 23-45% of digestion, ~1/5 to brood. A store-keyed brake only makes sense once food stays at home.
4. Later plan items: B5, B4 roles by age (agree with the nest lane first), B4b, B6 (founding, lifespan, mating flights).

## Test beds (commands)
- Long runs: `trailfollow mode=gap gate=shipped frames=192000 ants=20 near=10 food=400 refill=400 gaps=90|200 arms=self seeds=1 seed0=S`, env `COLONY_SPACING=2 STACK_DEPTH=4 RAYON_NUM_THREADS=1`. Steadiness from FOOD STORE `ants N` series.
- Food box: `digbox hungry gap=90 food=400 refill=400 w=260 soil=60 frames=96000 stops=48000,96000 seed=S`.
- Lab box: `labforage scenario=played_bed frames=120000 seed=S`, run from repo root; compare with `python3 scripts/labpair.py DIR base new`.

## Pitfalls
- labforage refuses BUD_SITE/COLONY_SPACING/STACK_DEPTH env vars unless `bedenv` is passed.
- Tests written against budding pin `w.bud_at_nest = Some(false)` (and brood off); new tests that lay must pin too.
- Lab numbers on 0738a8ca+ include lay-at-nest; don't compare to 6a8dacd4. Pre-5244c858 (kin footing) baselines don't compare either.
- labpair "net food into home" overcounts 3.4-4.6x. "births held by the food brake" counts held attempts, not lost births. "buds_held_for_nest" counts frames held, not ants.
- `pgrep -x`, never `-f`. `cargo build --release` does not rebuild examples.

## Files
Results: `/mnt/project-files/breeding/brood-vs-budding-2026-10-02.md` (all sections; food brake and lay-home not yet appended), `plan-review-2026-10-02.md`, per-ant CSVs `bankdump-2026-10-02/`. Research cited so far: Tschinkel 1988; Rueppell & Kirkman 2005 (PMC2408869); Greenwald et al. 2018 eLife (PMC5862530).

## Tools and methods (what this lane actually runs)
All from the repo root, release build. **Rebuild examples after any change** (`cargo build --release --examples`; `cargo build --release` alone leaves stale example binaries). Copy the binary into the run dir before a long batch so a rebuild can't change it mid-run, and write the commit into the dir (`echo <sha> > DIR/commit`). Every number quoted must carry the main commit it ran on. Machine has 4 cores: run 3 jobs at once (`xargs -P3`), each with `RAYON_NUM_THREADS=1`.

| Bed | Command | Run time (one job) | Gate on |
|---|---|---|---|
| Food box | `target/release/examples/digbox hungry gap=90 food=400 refill=400 w=260 soil=60 frames=96000 stops=48000,96000 seed=S` (4-12 seeds) | ~5 min | live ants and born at each stop, BROOD line, BIRTHS line |
| Nest box | `digbox fed ants=40` and `digbox fed ants=200`, 8 seeds, `frames=24000` | ~2-4 min | dug cells, alive |
| Long runs (colony bed) | `trailfollow mode=gap gate=shipped frames=192000 ants=20 near=10 food=400 refill=400 gaps=90 arms=self seeds=1 seed0=S`, and the same with `gaps=200`; env `COLONY_SPACING=2 STACK_DEPTH=4 RAYON_NUM_THREADS=1`; 6 seeds each | ~10-15 min | steadiness: `python3 /mnt/project-files/breeding/stab.py DIR` (logs named ARM-gGAP-sSEED.log; edit its arm list) gives min-max ants from the FOOD STORE `ants N` series from frame 60k, coefficient of variation, halvings from a running peak (peak >= 20); colonies alive at the end |
| Lab box | `labforage scenario=played_bed frames=120000 seed=S`, 12 seeds, paired arms; compare `python3 scripts/labpair.py DIR base new` (files named `base-N.log`, `new-N.log`) | ~4-5 min | born, alive at end, boxes died out |

Instruments:
- `trailfollow bankdump=FILE`: every ant's bank each 3,000 frames, columns `frame,id,bank_j,crop_j,brood,x,y,off_nest`. Used for the bank histogram (CSVs in `/mnt/project-files/breeding/bankdump-2026-10-02/`).
- trailfollow `EATING` line: digested at the nest / away, shared mouth to mouth (and to brood), picked up at home, births held by the food brake.
- `BROOD` line (digbox, labforage): laid, larvae, pupated, starved, lost, hatches refused for room, J shared/nursed/eaten beside/upkeep, births held by the food brake.
- `labforage budtrace=FILE budtrace_every=30`: per ant `frame,id,x,y,bank,reachable,bar,at_nest,nest_d,crop,...`, the tool for "why didn't this ant lay". Trace individuals, never infer from aggregates.
- trailfollow `hungergate=W`: the appetite gate (`digest_hunger_weight`), off.
- labforage refuses `PIXEL_PHYSICS_BUD_SITE`, `COLONY_SPACING`, `STACK_DEPTH` in the env unless you pass the bare arg `bedenv`.

Switches this lane uses (default in brackets):
- `PIXEL_PHYSICS_BROOD` [on; `off` = old budding]
- `PIXEL_PHYSICS_NURSE` [on]
- `PIXEL_PHYSICS_BREEDING` [graded; `individual` = old rule], `PIXEL_PHYSICS_BREEDING_MAX` [1.25], `PIXEL_PHYSICS_BREEDING_RADIUS` [24]
- `PIXEL_PHYSICS_BUD_SITE` [nest; `anywhere` undoes]
- `PIXEL_PHYSICS_LAY_HOME` [on, branch only], `PIXEL_PHYSICS_FOOD_BRAKE` [off, branch only; `on` or `lo,hi`]
- `PIXEL_PHYSICS_LAY_AT`, `PIXEL_PHYSICS_EGG_COST`, `PIXEL_PHYSICS_LAY_REACH` [1] (sweep knobs)
- `PIXEL_PHYSICS_NEST_REACH` [head, r1; `body` or `rN`] is an oracle, not a candidate; it changes deliveries too
- Nest-lane switches that affect our numbers: `DIG_ROOF=6` (on), `KIN_FOOTING` (on), colony forage throttle (on, PR 540)
- Tests that lay or bud must pin `w.bud_at_nest = Some(false)` and brood as they expect.

Results and files: append each result as a dated section to `/mnt/project-files/breeding/brood-vs-budding-2026-10-02.md` (stamped with the commit); put the measurement in the commit message (before, after, what was rejected). Run logs go in the session scratchpad, which is lost with the container, so copy anything worth keeping to `/mnt/project-files/breeding/`.

## Working habits Scottt corrected (a fresh lane will repeat these)
- Don't answer "how real ants do it" from memory. Repo first: `Reports/nest-biology-2026-09-19.md` (§3.5 repletes vs granary), `Reports/ant-breeding-plan-2026-09-29.md` (biology table ~line 465, tagged [measured]/[repeated]), `Reports/ant-sim-literature-review-external-2026-09-19.md`; then web search, cite the paper. The plan has no row on food-keyed laying.
- Don't wait for CI before handing off a PR; the merge desk watches CI. Hand off on done, then keep working.
- No full test suite before a PR: `cargo fmt` on touched files only (rustfmt the file), `cargo clippy --release --locked --lib --example trailfollow --example digbox --example labforage -- -D warnings`, `cargo test --release --lib sim::` plus `tests/*.rs` only if you changed a list others iterate.
- Default new features on unless measured harm; if the permission check blocks a default flip, cite Scottt 2026-10-01: "You can turn anything on by default that you want."
- Check the lab box before telling Scottt a fix works, and before predicting one will (dug home was predicted to rescue lab laying and did not).
- Name what a counter counts before quoting it; confirm a cause on enough seeds; run the fix before announcing the cause.
- Scottt's goal decides: stability and food stock over colony size. Shrinking is fine.
- Ants should be where they are because of their work, not pulled to a spot (Scottt 19:52). Walk-home-to-lay is justified as "go home to do the task", like a laden forager; keep it that way.
- Never start a new thread without Scottt's explicit yes. Scottt watches token usage: read docs by section, not whole (README ~72k tokens, PLAN ~60k, dead-ends ~97k: grep the mechanism).

## Ownership and coordination
- Breeding lane owns `brood.rs` (`lay_egg`, `nurse`, brood ticks), `kin_deficit`, `try_bud`/`birth_bar`/`suppress_bar`, the breeding switches. The nest lane owns digging, home definition (dug home), piles, larva scent (which must read `kin_deficit`, Larva stage only). Changes in the other lane's files go through the coordinator.
- Roles by age (B4) overlaps the nest lane's plan: agree with Scottt first.
- PR flow: push branch, open a draft, subscribe to its activity, tell the coordinator it's for the merge desk. Lanes never merge.

## Team memory this lane relied on (substance copied above)
breeding-lane-order (plan order and gates), real-ants-cite-research, features-default-on, default-flips-need-owner, hand-to-desk-early, merge-desk-procedure, nest-lane-avoid-breeding-rules (the nest lane leaves breeding to this lane; store-paid birth switches BUD_STORE/BUD_RESERVE are stopgaps, off).
