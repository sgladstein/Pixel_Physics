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

1. ~~`creature_space mode=diet seeds=8 frames=18000`~~ — **also finished
   before teardown** (~55 min wall); log is
   `diet-baseline-postmerge.txt` here. Part 3's baseline half is DONE;
   the successor runs only the retuned half (same command, retuned
   tree). What the baseline says, so nobody re-derives it from the log:
   - **Litter-only control passes its spec**: herbivore-side plateau
     (0.921/0.925/0.912 at gut −1/−0.5/0, indistinguishable within seed
     spread) collapsing to 0.298 at +1 — single-peaked, no false hump.
   - **Mixed arm is single-peaked** (0.874/0.883/0.909/0.880/0.639) —
     the plan's own named falsifier, expected on the first attempt. But
     the carrion is not inert: at gut +1.0 survival is **0.639 with
     carrion against 0.298 without**, paired seeds — the meat niche
     exists as a paired difference, it just does not yet beat the
     generalist at this abundance. Plan §2.5's prescribed fix is
     ecology (more carrion / richer stamp), NOT the filter. Note also
     the generalist (gut 0) currently wins outright with eats 0.95 —
     at the 4x economy with litter this abundant, specialising buys
     nothing; re-reading this after the retuned arm is the point of
     the pair.
   - **Separation**: ±0.8 cohorts read mean 49.1 (12.0..104.8) against
     the both-at-0 null's 7.3 (2.7..16.4) — the means are far apart
     though the worst ±0.8 seed (12.0) dips inside the null's range,
     so quote the distribution, not just the means.
   - **Instrument caveats the columns caught**: foodcols min 67 of the
     104 asked (the colony band runs out of empty columns — same for
     every arm, so paired comparisons stand); separation placed 15+14
     of 26+26 and the west litter bank took as few as **4 cells** on
     the worst seed (wetland water) — the separation scene wants a
     drier placement rule before its absolute numbers are leaned on.
2. ~~`creature_space mode=economy seeds=4 frames=18000`~~ — **this one
   finished before teardown**; the log is
   `economy-baseline-postmerge.txt` in this directory. Baseline half of
   part 2 is DONE: advantage +0.470 / +0.481 (no-moss arms) and +0.486 /
   +0.525 (moss arms), ants fed 0.73 / 0.79, zero genome 0.298, placed
   52/52 in every arm — against §4's recorded +0.460/+0.479 and
   0.73/0.78, so the water book barely moved this guard. The successor
   runs only the RETUNED half (same command, retuned tree) — but note
   the §4 pairing rule: quote the pair only if both halves are from the
   same machine; this baseline was run on this cloud container, so
   either re-run the baseline in your own session (it is 40–90 min) or
   quote these numbers as same-tree-different-machine with a dated note.

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

---

# Successor session, 2026-08-23 ~19:30Z — read this before landing the branch

## For whoever lands this: `main` is ALREADY merged in, up to `7cd1357`

The coordinator's check-in (routine "WP-11: do not chase the M worldgen
reds") says it will merge `main` as part of landing and asks the branch
not to do it. That instruction arrived **after** this session had already
merged, in commit `2aefd86`, and the merge was not tidiness — it was
load-bearing for every number below. Landing should therefore expect a
branch that already carries `main` to `7cd1357`, not one cut at
`eda560d`. Nothing after `7cd1357` has been merged; `origin/main` is at
`4018aee` and the remaining 28 commits are the coordinator's to bring in.

**Why the merge could not wait.** `main` gained the §L rock-country fix
(`2c651bb`) and PR #20's flora sowing (`03a1cf2`) after the predecessor's
baselines were taken. Both change generated worlds. Every inherited
number in the sections above is therefore a measurement of a world this
tree no longer produces, and pairing across that boundary is the exact
failure the predecessor recorded when PR #19 landed mid-session.

## The finding that reordered part 1

**The retune covers one of the four woody species the world now sows.**
`worldgen/passes.rs`'s `WOODY` table sows creeper, shrub, conifer and
tree, each into its own country; `flora_census -- frames=4000` reads all
four established in **8 of 8 seeds** (established med: conifer 6,
creeper 13, shrub 4, tree 17). The abscission cut landed on `tree.ron`
alone, and `strings` on the built binary confirms it: the retuned
binary still carries `shade_death: 0.003` for the other three.

Measured, colony scene, wetland seed 0, 12,000 frames, one binary per
arm, samples at 3,000/6,000/9,000/12,000:

| arm | decay events | shed leaves | standing litter | living tissue |
|---|---|---|---|---|
| A — none (all four 0.003) | 5,061 | 6,314 | 1,248 | 28,776 |
| B — landed (tree only) | 2,954 | 3,738 | 783 | 30,207 |
| C — all four at 0.00075 | 1,532 | 1,913 | 381 | 33,425 |

So the owner's lever, which measured **−72%** on the pre-merge world,
measures **−42%** on this one, and roughly half the floor manufacture
left after it comes from the three species it never reached. Card
`20260823T191540041Z-cca566` (board `plants`) puts A/B/C to the owner
with the counts in `meta`. **Arm C is not committed** — the three
`.ron` edits were made, built, measured and reverted; the working tree
is clean. If the verdict is C, the edit is one line per file at the
second `Photosynthesize` site of `conifer`/`shrub`/`creeper`, and every
sweep below must be re-run on it.

`wiki/plants.md` claimed the slower fall world-wide; qualified in
`41b292c` to say it is the tree's only. If C lands, that paragraph goes.

## Instrument fix landed this session (`21d8c02`)

`diet_separation` divided by `sep_n.max(1.0)`, so a run where the two
cohorts were never alive in the same sample returned separation **0.0**
— identically the value the both-at-0 null arm exists to produce, with
`placed_a`/`placed_b` unable to tell them apart because both are counted
at spawn and never fall. Part 3 gates the S5 separation criterion on that
column. Now carries `samples` and prints `both-alive (min)`; a zero there
says the mean beside it is not a separation. Print-and-field only, and
confined to the diet path — `mode=economy` never reaches it.

## Known instrument caveat, NOT fixed (deliberate)

`corpse@end` in the survival sweep counts every corpse cell standing at
the end, including ants that died during the run — so the litter-only
control reads non-zero and the column overstates what it proves about
seeding. It does not feed the survival curve or the control's shape, so
it was left alone rather than thrashing four hours of in-flight sweeps
for a diagnostic label. Read it as "meat present", never as "meat
seeded"; the seeded counts are tracked separately (`litter_seeded` /
`corpse_seeded`).

## The two worldgen at-rest reds are not this branch's

Verified here, not taken on faith: `tests/worldgen.rs` has **0**
occurrences of `weather_override` on this branch and **3** on `main`;
`git merge-base --is-ancestor 45ba304 origin/main` is YES, and against
this branch's HEAD it is NO. They go green on the coordinator's merge.

## Clone defect worth knowing

This container's clone was made `--depth 1`, which left a fetch refspec
of only `+refs/heads/main:refs/remotes/origin/main`. Plain `git fetch`
therefore refreshed **no** branch's tracking ref but main's, so
`origin/claude/wp11-economy-retune` sat frozen at the predecessor's
`820fd18` while the real remote was hours ahead — which reads as "33
unpushed commits" and, worse, silently feeds `branchcheck.sh` stale
ahead/behind numbers. Repaired to `+refs/heads/*:refs/remotes/origin/*`.
Check this on any fresh cloud clone before trusting `branchcheck.sh`.
