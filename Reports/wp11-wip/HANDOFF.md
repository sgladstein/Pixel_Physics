# WP-11 handoff: session retired mid-package by the coordinator's wind-down

**Written 2026-08-23 ~17:20Z by the session that ran WP-11 part 1 to
completion.** This directory holds the raw measurement logs the successor
must not re-pay for; this file says what is done, what was mid-flight when
the wind-down fired, and what remains. Delete this whole directory when
WP-11 lands — its numbers will by then live in the reports they belong to.

## Done, landed, gated (all on this branch's history — do NOT redo)

- **Part 1 (abscission retune) is COMPLETE.** `tree.ron` leaf
  `shade_death`/`drought_death` 0.003 → 0.00075 (commit `05a13f0`), chosen
  by the owner on card `20260823T161006584Z-6ecbab` — verdict verbatim:
  "C is best". Sweep table and rejected alternatives are in that commit's
  message; docs updated in `339fbda` (§O, wiki/plants.md floor claim,
  PLAN-log; docscheck clean).
- **Instruments** (commit `b2660dd`): `World::shed_shade/shed_drought/
  shed_stranded` + filmstrip print — attribution on the colony scene is
  ~89% shade, so shade_death is the lever; `creature_space mode=diet`
  (plan §2.5's S5 measurement, carrion seeded, controls built in);
  `mode=census` + `CENSUS_PRESET` (colony-band food census standalone).
  Harness hardening in `10607bd` (ants-before-food, food-count columns,
  parallel seeds).
- **Post-water-book paired verification** (main merged at `eda560d`,
  merge commit `b43e7fd`; both runs same tree, same session, logs in this
  directory): baseline 6,653 decay events / 7,687 sheds / 1,024 standing
  litter / 24,589 living cells at frame 12,000; retuned **1,862 / 2,214 /
  344 / 27,119** — soil manufacture −72%, crowns still separate by eye.
- **Gates on the retuned tree** (`suite-retuned.txt`): lib 878/0 — the
  sealed-box guards and the moss-pump probe are lib tests and are GREEN,
  the no-pump property holds; main 9/0; determinism 2/0; worldgen 37/2
  where the 2 are §M's pre-existing water-at-rest reds, byte-identical at
  baseline. clippy -D warnings clean. docscheck clean.
- **Pre-merge card data** (one binary per arm, pre-water-book tree; valid
  as the card's internal comparison, superseded for absolute numbers by
  the post-merge pair above): decay 6,331/3,443/1,800; litter
  1,081/569/321; colony-band food census medians 70,080/81,600/90,720
  (censuses in this directory) — note food *energy* RISES with slower
  fall; the caveat is on the card, in tree.ron's comment, and in §O.

## Mid-flight when the wind-down fired (processes die with this container; both had produced NO output yet — nothing is lost, they simply need re-running)

Both are the **baseline halves of part 2/3's paired measurements** and
must run on **main @ `eda560d` + baseline tree.ron (0.003)** — i.e. the
tree at merge commit `b43e7fd` — from the **repo root** (worldgen presets
load from disk relative to cwd; elsewhere it panics "the wetland
preset"). Rebuild before running (assets are include_str!'ed). The trick
this session used: build baseline, `cp` the example binary aside, flip
tree.ron, rebuild — the copied binary keeps its assets.

1. `creature_space mode=diet seeds=8 frames=18000` — S5 baseline sweep
   (~1–3 h wall; seeds run in scoped threads). Then the same on the
   retuned tree for the pair.
2. `creature_space mode=economy seeds=4 frames=18000` — the standing-
   guard methodology recorded in evolution-plan §4 (44 min on the
   owner's box, longer here). Then the same retuned.

## Remaining work (WP-11 parts 2 and 3; the handoff's spec is
`Reports/creature-implementation-handoff-2026-08.md` §WP-11)

- Part 2: paired economy runs above; reference-genome pair
  (`genomes=1 seeds=8 frames=18000`) both trees; update evolution-plan §4's
  table with a dated note (baselines +0.460/+0.479 and 0.73/0.78 are of
  the over-fed world and are EXPECTED to move). While open-bugs §L
  stands, do not quote `ascii`'s forage_loop colony numbers.
- Part 3: paired diet sweeps above; judge the two-humped/-single-peaked/
  separation criteria per plan §2.5 (set the separation bar against the
  both-at-0 arm's seed spread); record results as an "As measured"
  addendum to plan §2.5; a review card with the curve; `diet_yield`'s
  12.0 threshold wants re-deriving from the sweep (its own comment says
  so). Diet-mode smoke at 1 seed/6,000 frames read: mixed
  0.863/0.907/0.943/0.936/0.737 for gut −1..+1, separation 7.2 (±0.8) vs
  3.5 (0/0) — harness-works evidence only, not the measurement.
- **No PR exists.** This session's GitHub API access was disabled
  ("Claude GitHub App not connected"), so the PR could not be opened;
  CLAUDE.md's standing authorisation still applies to whoever can.

## Traps this session actually hit (in one afternoon)

- The shell cwd silently reset to `/home/user` twice; two harness runs
  died instantly on the presets panic and one produced an empty log with
  exit 0. Always `cd` explicitly in the same command.
- The three-arm card's food seeding originally blocked ant spawns —
  `placed` read 17/52. Fixed in `10607bd`; keep reading the `placed` and
  `foodcols` columns.
- PR #19 (water book) landed mid-session and moved the colony floor
  numbers ~5%; the pre-merge baselines were discarded and re-measured
  rather than paired across trees. The same applies to any future
  mid-session landing on main.
