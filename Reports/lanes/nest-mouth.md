# Lane note — the nest's mouth

*Kept current, edited in place. Findings live in
[`../nest-mouth-2026-09-26.md`](../nest-mouth-2026-09-26.md); this note keeps
the live question, what is addressed to another lane, predictions and heads.*

- **Session:** `session_01WF4wABj2ewSmzWVJTk6DsC` (the nest lane: the mouth,
  then nest building and spoil).
- **Branch:** `claude/ant-nest-mouth-4f6s79`. GitHub deleted it when #493
  merged; restarted from `main` 2026-09-27, same name.
- **Peer:** the foraging-loop session `session_01Pt5N39pfcix13hMycPN9Xs`,
  branch `claude/ant-foraging-loop-handoff-986v7n`, lane note
  [`foraging-loop.md`](foraging-loop.md). The only other agent on the repo.

## Standing owner rulings for this lane

- **2026-09-26: "Make sure you are not using too many ants in your tests and
  always take snapshots at multiple times."** `digbox` runs at 40 ants with
  `energy=1000` (under the 1,100 budding threshold, no food, so the count
  stays 40); every picture is several stops. Results taken at 300 ants (which
  breed past 800) are marked as such in the report.
- **2026-09-27: "In general, I prefer options on by default unless there is
  a good reason not to."** It restates 2026-09-12's "You can ship everything
  on. I will tell you to change it if I don't like it", whose carve-out keeps
  a pure look off until he has seen it. The foraging lane has asked to hear
  it from the owner directly before applying it to its own switches.

## Live question

**Is the colony building a nest? Not yet** ([`../nest-work-2026-09-27.md`](../nest-work-2026-09-27.md)).
At 40 ants its excavation is less nest-like than random walkers digging from
the door on 12 of 12 seeds (31 mouths, no chamber); 3% of cuts build
anything; 78% re-dig the heaps or cells that refilled, mostly by falling in.
**The mouth line stays closed and the default stays off:** re-measured on
current `main`, door + dug mouth halves bed starvation (201 → 105) and costs
the lab colony (food eaten lower on 10 of 12, one colony lost) -- the
owner's "good reason not to".

**Next, 2026-09-27** (proposed to the owner, not yet ruled on):
1. Walls and a roof that hold. Refills are 94% soil falling in, half from
   above and half from the side; the heaps contribute nothing
   (`nest-work-2026-09-27.md` §4).
2. `DIG_DOWN` as a default candidate: cuts that build 3% → 6%, a bigger and
   better-connected excavation; bed and lab check first.
3. A dig marker at the face (Toffin's marker half; untried) against the 31
   mouths.
4. `DropSpoil` as a spoil drop, not a food drop: only with 1-3 (alone it
   moves pellets and builds nothing).
5. `UNPACK`: unmeasured alone in the lab; on top of the mouth, 3 of 12 lab
   colonies lost.

## Working agreement with the foraging lane (2026-09-27)

Proposed by this lane, accepted by the foraging lane the same day. The owner
asked the two sessions to agree who owns what.

- **Foraging lane:** the walk (`chooser_step`, scouting, hungry-home,
  homing, `forage_anchor`), the trail planes, `act`'s share, ingest and
  drop-food sections, the crop and digestion, `NestSite::larder`,
  `World::nest_need` / `forage_drive` / `step_nest_need`, `trailfollow.rs`,
  `antloop.py`, `labforage.rs` (this lane asks before editing it),
  `ant-scenes-2026-09-23.md`, and `ant.ron` rows into Move, Turn, Feed, Drop,
  Share and Attack.
- **Nest lane:** founding and the nest's shape (`found_colony_with`,
  `paint_nest_patch*`, `colony_stations*`, `NEST_DOOR/SHAFT/HOME`,
  `cut_founding_shaft`, `NestSite` except `larder`), the home test
  (`adjacent_nest`, `nest_within_reach`, `nest_home`), `act`'s spoil and dig
  sections, `line_burrow`, `lift_reach`, the dig and room gates, the nest and
  spoil materials, `ant.ron` rows into Dig and DropSpoil, the examples
  `digbox`, `labshot`, `antdirt`, `labnest`, `spoil_*` and `burrow_probe`, and
  `Reports/nest-*.md`.
- **Shared, by region or append-only:** `CreatureStats` (each its own
  group); `DecisionRow` (foraging lane's; nest fields go at the end, after
  asking); `how-the-ant-works.md` (nest: §5 steps 5-6 and §8; foraging: the
  rest; §12 rows and the "Verified against" paragraph are appended, both
  kept on a conflict); `dead-ends.md` (append; regenerate the index files,
  never hand-merge them); `Reports/README.md` (own entries); bug letters from
  `bugindex.py --branches`. `CLAUDE.md` and `.claude/`: neither lane without
  the owner.
- **Couplings, poke before landing:** the home test feeds `AtNest`,
  deliveries, homing, scout release, the dig gate, and the forage drive's
  `home_target` / `forage_anchor`; founding and nest shape move the loop's
  baselines; `Crowding` and `MoistureGrad` feed both lanes' outputs; nest
  attribution (`nearest_nest_site`) feeds both `step_nest_room` and
  `step_nest_need`.
- **How:** small PRs, each lane merges its own on green after merging `main`
  in. Run `branchcheck.sh --who-touched <path>` before editing the other's
  region. Poke with the SHA when a merge moves the other's baseline.
  **Channel:** a poke (`create_trigger(persistent_session_id=…)` then
  `fire_trigger` bare) works both ways, but waits until the other's turn ends:
  15:30 → about 17:50 once. Files pushed to origin are the record.
- **Sequencing:** settled 2026-09-27: door + dug mouth stays off, so there
  is no new baseline for the forage drive; the foraging lane was poked.
- **Still open for the foraging lane:** `trailfollow`'s header does not echo
  `PIXEL_PHYSICS_NEST_DOOR` / `_SHAFT` / `_HOME`, and it does not print
  `pickups_at_nest`.

## Predictions (written before each run)

The 2026-09-27 funnel arms (haul, cover, dig-down, drop-away) were exploratory and carry none.

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
- `fa97db9e` — the nest scoreboard in `digbox` (colony against four nulls).
- `2358c2dc`, `0e08216c` — the nest funnel, and what re-cut cells were made of.
- `6ead09ce` — the write-up: not yet a nest; door + dug mouth stays off.
