# WP-11: landing notes

**Everything WP-11 measured now lives in the reports it belongs to** —
`creature-evolution-plan.md` §4 (the paired economy guard and the
reference-genome pair) and §2.5 (the paired S5 sweep, as an "As measured"
addendum), with the narrative in `PLAN-log.md`. This file holds only what
the person landing the branch needs and cannot read off a diff. **Delete
the directory once the branch is merged.**

The predecessor's raw logs that used to sit beside this file have been
removed rather than carried forward: every one of them was measured on
`main@eda560d`, before the §L rock-country fix and PR #20's flora sowing,
and both change generated worlds. They described a world this tree no
longer produces, so keeping them would have offered a tempting and wrong
baseline. Their conclusions, where they survived re-measurement, are in
the reports above.

## 1. `main` is already merged in, to `7cd1357`

The coordinator's check-in asked the branch not to merge `main`, and that
instruction arrived **after** commit `2aefd86` had done it. The merge was
load-bearing, not tidiness: without it every number in this package would
describe a world the tree no longer generates. Nothing past `7cd1357` has
been merged. Expect a branch already carrying `main` to that point.

## 2. Every number here describes the pre-WP-9 ant

This branch has `ant.ron`'s `climbs_over_kin: false`; `main` took it to
`true` at `00d1551`, after these runs. An ant that can cross a nestmate
ranges further, and the economy guard measures ranging. **After the
landing merge these figures want re-taking, not re-reading** — both
halves, since a pair is only valid within one tree. The same goes for
PR #31's `plant.rs` work, which lands in the abscission code's own
neighbourhood.

## 3. The two open review cards, and what each one decides

- `20260823T204413045Z-25c85d` — the floor, as an animation. Confirms the
  fix. No decision rides on it.
- `20260823T204815827Z-ffc290` — **the crowns.** This one has a
  consequence: arm C is landed, so if the canopy reads as one mass the
  remedy is taking conifer/shrub/creeper to `0.0015` and leaving `tree`
  where the owner put it — one value line per file — **not** reverting the
  floor fix. Leaf share is 50.2% → 58.7% with wood essentially unchanged,
  so the crowns genuinely can look different and his answer will mean
  something.

## 4. The two worldgen at-rest reds are not this branch's

Verified rather than assumed: `tests/worldgen.rs` has **0** occurrences of
`weather_override` here and **3** on `main`; `git merge-base --is-ancestor
45ba304 origin/main` is YES and against this HEAD is NO; and the three
insertions on `main` sit inside exactly the two failing test functions
(`a_forced_vault_world_is_sealed_and_arrives_at_rest`,
`generated_terrain_is_already_at_rest`). They go green on the merge.

## 5. A clone defect worth checking on any fresh cloud session

This container's clone was made `--depth 1`, which left a fetch refspec of
only `+refs/heads/main:refs/remotes/origin/main`. Plain `git fetch` then
refreshes **no** tracking ref but main's, so this branch's
`origin/...` pointer sat frozen at the predecessor's `820fd18` while the
real remote was hours ahead. That reads as phantom unpushed commits, and
worse, silently feeds `branchcheck.sh` stale ahead/behind numbers — the
one thing CLAUDE.md says to run before trusting a measurement. Repaired
here to `+refs/heads/*:refs/remotes/origin/*`; check it on any fresh
clone before believing `branchcheck.sh`.
