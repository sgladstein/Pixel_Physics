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

**Answered, and the stop rule applied twice: no mouth tried is better than
today on both beds.** Bed (24 seeds, starved against 277): door 209, door +
mouth as home 221, door + whole cut as home 254, no paint + mouth 256. Lab (12
seeds): the dug mouth is buried by frame 30,600 on 11-12 of 12 seeds under the
colony's own delivered food and the roots that grow in it. Every narrow home
records fewer deliveries than the strip, but a delivery is a drop made at home,
so a smaller home counts fewer of them by definition. The lab's outcomes that
do not depend on where home is (births, food eaten, colony-frames, starvation)
separate no arm from the default at 12 seeds. A new counter says 86% of the
strip's deliveries are food picked up at home first; net of that, the dug
mouth brings home as much as the strip (604 against 712 cells, 6 / 6) and the
painted door about half (384, lower on 10 of 12).

**The one variant tried after the stop rule failed on sight.** A home that
follows the pile over the mouth (`NEST_HOME=mound`, predictions 19-22) is what
this note said would reopen it. In the lab it makes the colony stack its food
into a tower over the door: a median of 34.5 rows at frame 30,600, and over 15
rows on 12 of 12 seeds by 60,300. The fixed mouth stays at a median of 3 rows
(0 of 12 over 15). Its 8,015 deliveries were read as a win before anyone looked
at a frame. Reverted, report §6. What would reopen the line now: delivered
food that slides, so a pile widens as it grows, or deliveries that stop piling
in the mouth. Changing the default is the owner's ruling.

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
- **Your §19 re-test condition is not met**: a dug mouth does not stay open
  in the lab. It is buried by frame 30,600 on 11-12 of 12 seeds under the
  colony's own delivered food and the roots growing in it. Written back to
  your dead-ends entry; numbers in the report's §5.
- **Echo request, your file**: `trailfollow`'s header does not name
  `PIXEL_PHYSICS_NEST_DOOR` / `_SHAFT` / `_HOME`, so a bed log does not say
  which nest it ran. Not touched here.
- **`deliveries` depends on how big home is, and so does the funnel's
  "looped".** A delivery is any drop made at home, so a crumb lifted off the
  nest and put straight back counts twice, and a bigger home counts more
  drops. New on this branch: `CreatureStats::pickups_at_nest`, the pickups
  made on the same predicate, read before the mouthful leaves.
  `deliveries - pickups_at_nest` is the net flow home. `labforage` prints it;
  `trailfollow` (yours) does not yet. Starvation is the bed measure that does
  not move with home's size. **Measured in the lab (12 seeds): 86% of the
  default strip's deliveries are food picked up at home first**, so net,
  §19's door is 712 -> 384 cells (lower on 10 of 12, p 0.039), not
  5,396 -> 1,279. That is the cycle your L1185 entry was built to break, seen
  in the lab; the §19 entry is written back.
- **How this reached you**: the trigger poke the brief prescribed failed.
  `session_01AFH5xR442VuoZsXm7VzJmx` is "not found" from this session's
  account, so this note is the channel. PR #493 carries all of it.

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
| 19 | door 2 + shaft 6 + `NEST_HOME=mound`, bed | within ±20 of the mouth arm's 221: the bed mouth is not buried much | wrong, better: 188 (default 279: 22 fewer / 2 more; the mouth arm: 16 / 6) |
| 20 | mound, lab | deliveries above the mouth arm on ≥ 9 of 12, median ≥ 4,000; STOP if not | right on its letter (12 of 12, 8,015) and wrong on what it meant: 77% of the deliveries were food picked up at home first, and the rest built a tower (report §6); births, food eaten, colony-frames and starvation split 6 / 6 against the mouth arm |
| 21 | mound, lab census | the heap over the mouth taller than under the mouth arm (median cover at 30,600 > 4) | right: census cover 21 against 4; read off the frames, a tower of 34.5 rows (median) against 3 |
| 22 | mound, `digbox` 40 ants | nest shape within the mouth arm's spread | right: 126 cells dug (median) against 141 |

## Cards with the owner

- `20260926T044019146Z-52a96c` — a colony of 40 founded with one hole, four
  stops (replaces `…af67e6`).
- `20260926T044022784Z-77bab8` — the gray pixels are tunnel lining, four stops
  (replaces `…6faaf7`).
- `…e86359` (`UNPACK`) withdrawn: at 40 ants there is nothing for it to remove.
- `20260926T061431136Z-a29145` — blind: the painted door against the dug
  mouth with no paint, as GIFs of ants coming home (bed, seed 1).
- `20260926T064019633Z-0211cf` — four foundings on the colony bed, five
  stops, with both beds' counts in meta: which reads as a nest?
- `20260926T064023411Z-ec6018` — the dug mouth in the lab box, buried by the
  colony's own food: should a lab nest keep its mouth open?

- `20260926T160641891Z-86d16b` — home that climbs the pile: every lab colony
  builds a food tower over its door; reverting (seed 7 at five stops, all 12
  seeds at three, the fixed mouth as control).

## Head SHAs

- `636612c6` — branch cut from `main`.
- `133c6b73` — the founding shaft lined; genesis frozen before the cut.
- `80065c60` — `PIXEL_PHYSICS_NEST_HOME=shaft`; the 300-ant shape sweep.
- `9d73c684` — the 40-ant shape sweep; report and index.
- `3809f434` — `main` merged in (§19's door); the door's anchor read from
  the cut; `PIXEL_PHYSICS_NEST_HOME=mouth`.
- `97c491e1` — both beds measured on the final binary; `labshot`'s cut
  census; dead-end write-backs.
- `9d1c93ac` — the write-up: stop rule applied, new dead-ends entry, §19's
  entry written back.
- `6ef76d7f` — `main` merged in again (#492, scouting); bed default and
  door + mouth re-checked identical; 1,962 tests pass.
- `5ced0c9a` — review fixes: one founding cut per site; the door anchor
  reads only its own cut.
- `6281f729` — `NEST_HOME=mound`, the one variant after the stop rule.
- `4d9262f4` — `CreatureStats::pickups_at_nest`, printed by `labforage`.
- `e6b3537b` — the mound reverted: every lab colony built a food tower.
- The commit after it — the verdict corrected: lab deliveries are 86% churn,
  the dug mouth brings home as much as the strip, the tower in the report's §6.
