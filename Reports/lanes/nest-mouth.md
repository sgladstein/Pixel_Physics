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

- **2026-09-27 (evening): colony numbers do not block a nest step.**
  "Unless you are specifically working on a foraging loop part of the nest,
  you shouldn't care about 'Lost colonies died of starvation'. You care about
  making progress in nest building. It is okay if nest building temporarily
  hurts colony numbers." A step is judged on the dig box's nest measures; the
  bed and the lab are run to tell the foraging lane what moved, not to veto.

## Live question

**Is the colony building a nest? It is digging one in one place now**
([`../nest-dig-wiring-2026-09-28.md`](../nest-dig-wiring-2026-09-28.md)).
- **2026-09-28:** `(Bias, Dig, -0.3)` with `(SurfaceCurvature, Dig, -1.0)`.
  Openings 27.5 -> 10 (12 of 12), roofed 0.78, deeper, half as wide, the
  same digging; the dig beats random walkers on roofed share and depth for
  the first time.
- **Spoil stays spoil** (#503); the footing switch and drop-away stay
  switches ([`../nest-spoil-footing-2026-09-27.md`](../nest-spoil-footing-2026-09-27.md)).
- **The dig marker is withdrawn**: a digging pheromone tested negative in
  ants (`nest-biology-digging-signals-2026-09-19.md` §3.1).

**Next** (2026-09-28):
1. **One mouth.** Ten remain along the nest strip. Re-score on the new
   dig: a narrower home (`NEST_SITE_COLS`, its condition is met), the
   founding shaft and door + dug mouth, and fresh spoil as where new digging
   starts.
2. **The footing switch and drop-away**, re-scored on the new dig.
3. **Chambers** need contents (brood or a granary), which the dig box
   lacks.
4. **Why unpacked spoil cost the lab**: not traced.

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

Rows 1-22 (the mouth and the mound) are in
[`../nest-mouth-2026-09-26.md`](../nest-mouth-2026-09-26.md)'s appendix, and
rows 23-40 (the spoil footing) in
[`../nest-spoil-footing-2026-09-27.md`](../nest-spoil-footing-2026-09-27.md)
§6. New rows go in the report of the work they predict. The 2026-09-27 funnel
arms in `nest-work-2026-09-27.md` (haul, cover, dig-down, drop-away) were
exploratory and carry none.

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
  the dug mouth brings home as much as the strip (withdrawn 2026-09-27: not a
  net flow), the tower in the report's §6.
- `fa97db9e` — the nest scoreboard in `digbox` (colony against four nulls).
- `2358c2dc`, `0e08216c` — the nest funnel, and what re-cut cells were made of.
- `6ead09ce` — the write-up: not yet a nest; door + dug mouth stays off.
- `434bc8e8` (#501) — the refill ledger corrected: passes against standing
  fills, and conversions in place.
- `400ee85b` — the funnel's why-loose line: two pellets in three unfooted.
- `231d8857` — `SPOIL_FOOTING` and `SPOIL_PACKS`, both off; ten digbox
  arms and the colony bed.
- The commit after it — the lab, the verdict (both stay off), the wiki's
  hanging-tailings claim corrected.
