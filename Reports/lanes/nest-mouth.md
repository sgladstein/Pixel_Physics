# Lane note — the nest's mouth

*Kept current, edited in place. Findings live in
[`../nest-mouth-2026-09-26.md`](../nest-mouth-2026-09-26.md); this note keeps
the live question, what is addressed to another lane, predictions and heads.*

- **Session:** `session_01WF4wABj2ewSmzWVJTk6DsC` (the nest-mouth lane).
- **Branch:** `claude/ant-nest-mouth-4f6s79`, off `main` at `636612c6`.
- **Peer:** the foraging-loop session, `session_01AFH5xR442VuoZsXm7VzJmx`
  (the walk, feeding, the crop, dropping food, `trailfollow.rs`,
  `scripts/antloop.py`, `Reports/ant-scenes-2026-09-23.md`). This lane owns
  nest founding and shape, digging and spoil, the nest material and the
  nest/dig examples. **Exception:** §19's door switch and founder placement
  are the loop session's; this lane builds on them once they land.

## Standing owner rulings for this lane

- **2026-09-26: "Make sure you are not using too many ants in your tests and
  always take snapshots at multiple times."** `digbox` runs at 40 ants with
  `energy=1000` (under the 1,100 budding threshold, no food, so the count
  stays 40); every picture is several stops. Results taken at 300 ants (which
  breed past 800) are marked as such in the report.

## Live question

**Does a dug mouth keep what §19's door buys on the bed, and fix what it loses
in the lab?** On a scratch merge of §19 (PR #491, not landed) and this branch:
bed, door 209 starved / door + 6-row shaft 195 / + shaft as home 254; lab
deliveries, door 1,279 (stored) / + shaft 2,072 (10 up of 12) / + shaft as home
3,651 (11 of 11). The home shaft helps the lab and hurts the bed. My own
default and door lab arms are running for one-binary pairing.

## For the loop session

- §19's founder anchor is computed from `colony_surface` *after* founding,
  and founding now can cut a shaft (`PIXEL_PHYSICS_NEST_SHAFT`), in which case
  every founder's home lands on the chamber floor. The scratch merge takes it
  from the site's recorded surface instead (`NestSite::surface`); identical
  without a shaft. To be proposed with the PR that builds on §19.
- **L1185 and L1186** (`AtNest:Feed`, the `Drop` wiring) name "the nest has one
  mouth" as their re-test condition; §19's door meets it on the bed.

## Predictions (written before each run)

| # | run | prediction | right? |
|---|---|---|---|
| 1 | shaft at frames 0/1/5/30/300, unlined | frame 0 = the positive control; mostly refilled by 5 | right on both; the census hid it |
| 2 | lined cut | ≥ 90% open through 300 | right (82 of 82) |
| 3 | what floats (300 ants) | spoil resting on ants | wrong: lining |
| 4 | shaft vs default, shape (300 ants) | taller and narrower ≥ 8 / 12 | right (11–12 / 0) |
| 5 | home vs not home (300 ants) | deeper, and more dirt in the cut | wrong on both |
| 6 | w4 vs w2, home | more of the cut open | right (by construction) |
| 7 | default on this branch vs reference | identical, every line | right |
| 8 | strip + home shaft, bed | within ±15 starved | wrong: 309 / 376 |
| 9 | door on the scratch merge | reproduces 209 | right |
| 10 | door + shaft 6, bed | within ±15 of the door | right (195) |
| 11 | door + home shaft 6, bed | worse by > 15 | right (254) |
| 12 | door + shaft 6 (home or not), lab | deliveries recover toward 5,396 | partly: 2,072 / 3,651 (default arm pending) |

## Cards with the owner

- `20260926T044019146Z-52a96c` — a colony of 40 founded with one hole, four
  stops (replaces `…af67e6`).
- `20260926T044022784Z-77bab8` — the gray pixels are tunnel lining, four stops
  (replaces `…6faaf7`).
- `…e86359` (`UNPACK`) withdrawn: at 40 ants there is nothing for it to remove.

## Head SHAs

- `636612c6` — branch cut from `main`.
- `133c6b73` — the founding shaft lined; genesis frozen before the cut.
- `80065c60` — `PIXEL_PHYSICS_NEST_HOME=shaft`; the 300-ant shape sweep.
- `9d73c684` — the 40-ant shape sweep; report and index.
