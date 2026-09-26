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

**Answered, and the stop rule applied: no mouth tried is better than today on
both beds.** Bed (24 seeds, starved against 277): door 209, door + mouth as
home 221, door + whole cut as home 254, no paint + mouth 256. Lab (12 seeds):
every narrow home carries less food home than the strip, and the dug mouth is
buried by frame 30,600 on 11-12 of 12 seeds under the colony's own delivered
food and the roots that grow in it. Two variants failing the lab the same way
is the brief's stop; written up in the report and in `dead-ends.md`. The
switches stay, off and bit-exact. What would reopen it: a home that follows
the ground over the mouth (a site test, not a fixed footprint), or deliveries
that stop piling in the mouth. Changing the default is the owner's ruling.

## For the loop session

- §19's founder anchor was computed from `colony_surface` *after* founding,
  and founding can now cut a shaft (`PIXEL_PHYSICS_NEST_SHAFT`), in which case
  every founder's home landed on the chamber floor. Fixed on this branch in
  `found_colony_with`: it reads the surface the cut recorded
  (`NestSite::shaft`). The door arm reproduces §19's logs line for line, and
  guard `a_door_over_a_founding_shaft_homes_every_founder_at_the_mouth` was
  watched red.
- §19's guard `a_nest_door_paints_its_width_and_anchors_every_founder_at_it`
  was inserted between `every_lifetime_counter_closes_against_its_world_total`
  and that test's doc comment, so the doc now sits over the door test. Left
  alone here; it is yours.
- **L1185 and L1186** (`AtNest:Feed`, the `Drop` wiring) name "the nest has one
  mouth" as their re-test condition; §19's door and the dug mouth both meet
  it on the bed, behind switches. Written back to both entries; not re-tested.

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
| 12 | door + shaft 6 (home or not), lab | deliveries recover toward 5,396 | partly: 2,072 / 3,696, both lower than the default on 11 of 12 |
| 13 | final binary, bed default and door | every line of the reference and of the scratch-merge door | right (396 lines × 3 each) |
| 14 | door 2 + shaft 6 + `NEST_HOME=mouth`, bed | within ±15 of door + shaft 6 (195): the door's own paint already makes the rim and first row home, so the mouth adds two cells | wrong: 221 (against the door 10 / 9) |
| 15 | no paint (door 0) + shaft 6 + mouth, bed | within ±20 of the door's 209; loops within ±10% of 465 | wrong on starved (256; 7 fewer / 16 more against the door), right on loops (445) |
| 16 | both mouth arms, lab | near door + shaft 6 (2,072), well under the home shaft's 3,696; lower than the default on ≥ 10 of 12 | no paint: right (2,257; 1 / 11), and not what matters: births, food eaten and extinctions tie the default |
| 17 | door 2 + mouth, lab: extinctions | as the door (4 of 12), because the paint is what gets buried | wrong: 2 of 12 failed (door 4, no-paint mouth 2, default 1); home, not paint, is what matters there |
| 18 | lab census, 12 seeds: is the mouth open at frame 30,600? | buried on ≥ 9 of 12 in both mouth arms; the whole-cut home keeps it open on ≥ 4 of 12 | right on the mouth arms (11 and 12 of 12 buried); wrong on the whole cut (buried 12 of 12). The cover is the colony's own food and roots, not litter |

## Cards with the owner

- `20260926T044019146Z-52a96c` — a colony of 40 founded with one hole, four
  stops (replaces `…af67e6`).
- `20260926T044022784Z-77bab8` — the gray pixels are tunnel lining, four stops
  (replaces `…6faaf7`).
- `…e86359` (`UNPACK`) withdrawn: at 40 ants there is nothing for it to remove.
- `20260926T061431136Z-a29145` — blind: the painted door against the dug
  mouth with no paint, as GIFs of ants coming home (bed, seed 1).

## Head SHAs

- `636612c6` — branch cut from `main`.
- `133c6b73` — the founding shaft lined; genesis frozen before the cut.
- `80065c60` — `PIXEL_PHYSICS_NEST_HOME=shaft`; the 300-ant shape sweep.
- `9d73c684` — the 40-ant shape sweep; report and index.
- `3809f434` — `main` merged in (§19's door); the door's anchor read from
  the cut; `PIXEL_PHYSICS_NEST_HOME=mouth`.
- `97c491e1` — both beds measured on the final binary; `labshot`'s cut
  census; dead-end write-backs.
