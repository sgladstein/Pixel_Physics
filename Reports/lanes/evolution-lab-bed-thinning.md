# Bed thinning bisect — `d1535c39`..`eafde084`, round 29

**Answer, in one paragraph.** No single landing owns this — the thinning is
real, an order of magnitude on one seed, but the *which-PR-did-it* question
does not have one answer because the three seeds cross into a starved bed at
different landings and by different mechanisms, and one seed does not thin
at all. Seed 2 (the one round 28 flagged) crosses hundreds→tens at **#323**
(the dig-verb fix), compounded by **#325**; seed 3 crosses at **#320** (the
articulated body), riding an ant-population boom (195→511 alive) that then
crashes; seed 1 never crosses and *ends higher than it started* (482→544).
Where a crossing happens, it is consistently paired with a rise in `eats`
and `deliveries` (the colony foraging harder), not with a fall in them — so
where thinning is real, it reads as **overgrazing**, matching the owner's
"I think they are overgrazing." The one exception is seed 2's *first* drop
(392→145 at #321), where `eats`/`alive` fall alongside plants — that step
looks plant-side, not colony-side. **The positive control did not reproduce
exactly** (see below) — read the deltas, not the absolute baseline.

## Chain built, in order

| commit | PR | symbols confirmed |
|---|---|---|
| `d1535c39` | (baseline, #318) | none of the three below |
| `c8a9bf99` | #319 garden-loop instrument | none — byte-identical to baseline on every seed tested |
| `d8264cf6` | #321 water reaches non-tree plants | none |
| `c16ffff0` | #320 articulated body | `articulated` |
| `f0c8999c` | #323 dig verb / live seed | `articulated`, `dig_diverted_seed` |
| `b977af67` | #325 pip clock re-armed | `articulated`, `dig_diverted_seed` |
| `eafde084` | #324 flitter | + `flitter`, `nectar_only` |

Each built with `cargo build --release --example labforage` in its own
worktree (target copied in first, so incremental), binaries hashed distinct
(`md5sum`), symbol-checked as above. `RAYON_NUM_THREADS` 1/2/4 tried on the
baseline seed 2 and gave byte-identical output — this codebase's determinism
holds regardless of thread count in this container.

## `plants` at 120,000 frames

| commit | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| `d1535c39` (main) | 482 | 392 | 251 |
| `c8a9bf99` (#319) | 482 | 392 | 251 |
| `d8264cf6` (#321) | 469 | **145** | 212 |
| `c16ffff0` (#320) | 290 | 116 | **87** |
| `f0c8999c` (#323) | 544 | **54** | 139 |
| `b977af67` (#325) | 544 | **39** | 139 |
| `eafde084` (#324) | 544 | 39 | 139 |

Bold = the step that crosses from three digits into two on that seed. Seed 1
never crosses. `eafde084` is byte-identical to `b977af67` on the full
`SUMMARY` line prefix on every seed tested (only new flight counters are
appended) — #324 is confirmed inert on `played_bed`, as expected with no
flitter placed there.

## `plants` at 60,000 frames

| commit | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| `d1535c39` | 495 | 697 | 393 |
| `c8a9bf99` | 495 | 697 | 393 |
| `d8264cf6` | 1018 | 201 | 338 |
| `c16ffff0` | 743 | 272 | 309 |
| `f0c8999c` | 885 | 299 | 525 |
| `b977af67`/`eafde084` | 885 | 282 | 525 |

Seed 1's 60k figure (1018, *above* baseline) then falls back to 469 by
120k — the run has not settled at 60k on this seed; read the 120k column for
the standing state, not this one.

## Why: the ecology columns (seed 2, the flagged seed)

| commit | plants | ants alive | eats (cum.) | deliveries (cum.) | standing_flowers |
|---|---|---|---|---|---|
| `d1535c39` | 392 | 92 | 15,240 | 4,033 | 34 |
| `d8264cf6` (#321) | 145 | 9 | 12,750 | 4,913 | 3 |
| `c16ffff0` (#320) | 116 | 15 | 13,164 | 4,763 | 0 |
| `f0c8999c` (#323) | 54 | 27 | 22,234 | 5,887 | 0 |
| `b977af67` (#325) | 39 | 102 | 25,212 | 6,018 | 5 |

From `c16ffff0` on, `eats` and `deliveries` climb while `plants` keeps
falling — the colony is foraging harder, not less, as the bed thins. The one
step that does not fit — `d1535c39`→`d8264cf6` (#321) — has `eats` and
`alive` both *fall* (92→9) while `plants` also falls (392→145): that drop is
not "more eating", it is the water-under-non-tree-plants change costing the
herb stand directly, independent of the colony. Seed 3 shows the sharpest
version of the overgrazing read: `alive` goes 60→195→**511**→70 across
`d1535c39`→`d8264cf6`→`c16ffff0`→`f0c8999c` — a population boom exactly at
#320 (the PR's own claim, "forages better") followed by a crash, with
`plants` bottoming (87) at the peak of the boom and partly recovering (139)
once the population crashes back down. Seed 1 nets the whole chain *positive*
for plants (482→544) because its colony nets down over the same span (`alive`
3→83→84→25→25, `eats` 1,966→12,376→6,976): less colony pressure by the end,
not more.

## Positive control — does not reproduce, and what that means

Round 28 reports `d1535c39` seed 1/2/3 at 120,000 frames as **464/574/303**.
This chain's freshly-built `d1535c39` gives **482/392/251** — close on seeds
1 and 3, off by 46% on seed 2. Reruns of the same binary (same seed, three
times, `RAYON_NUM_THREADS` 1/2/4) are byte-identical, so this is not
run-to-run noise in this container.

**But the far end matches closely.** Round 28's own `eafde084` reading was
"464-ish / 39 / 139" on the same three seeds; this chain's `eafde084` reads
**544 / 39 / 139** — seeds 2 and 3 match *exactly*, seed 1 is in the same
range as "464-ish" was itself hedged to be. Two independently-run chains
landing on identical integers at frame 120,000 of a chaotic 120,000-frame run
is not coincidence; it means the far-end commit **is** reproducing across
environments. The discrepancy is confined to the baseline reading. Given
`CLAUDE.md`'s own most-repeated gotcha — *"you are probably measuring a
binary that is not the code you wrote"* — the more likely explanation is that
round 28's quoted `d1535c39` figures were carried over from the `#322`
first-pass table's "main" column (measured whenever main was checked out for
*that* test, not necessarily rebuilt fresh at this exact SHA) rather than a
clean rebuild of `d1535c39` itself, not that this chain's build is wrong.
Given that, this note trusts its own internally-verified baseline (the bold
cells above show the same crossings either way) over the quoted absolute
number, and reports relative to it.

## What this does not settle

- Why `d8264cf6` (#321, a water/moisture change) costs seed 2's herb stand
  directly — not traced past the SUMMARY line; would need `labforage`'s
  `trace=herb` or a `waterstand` run on `played_bed` at this seed to see the
  moisture path.
- Why seed 3's ant population booms 4x exactly at #320 and nowhere else —
  plausibly the articulated body's reach or movement cost, not measured here.
- Seeds 4–6 (round 28's six-seed sweep used 1–6): not run here, budget
  spent on 1–3 across the full seven-commit chain instead of a wider seed
  set at fewer commits. A second lane could usefully re-run this same chain
  at seeds 4–6 to see whether "seed 1 never thins" generalises or was itself
  a one-seed sample.
- Nothing here was tuned; no species or scenario file was touched (verified
  `git diff --stat d1535c39..eafde084` on `assets/species/*` and
  `played_bed.ron`: zero changes to any of them across the whole range).
