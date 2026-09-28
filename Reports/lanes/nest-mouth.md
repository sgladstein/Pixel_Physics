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
  a pure look off until they have seen it. The foraging lane has asked to hear
  it from the owner directly before applying it to its own switches.

- **2026-09-27 (evening): colony numbers do not block a nest step.**
  "Unless you are specifically working on a foraging loop part of the nest,
  you shouldn't care about 'Lost colonies died of starvation'. You care about
  making progress in nest building. It is okay if nest building temporarily
  hurts colony numbers." A step is judged on the dig box's nest measures; the
  bed and the lab are run to tell the foraging lane what moved, not to veto.
- **2026-09-28: the footing switch and the marker.** "Yes to both your
  recommendations":
  - "pellets only on real ground" (`SPOIL_FOOTING=ground`) stays a switch,
    decided together with the marker, since that is when it should pay off;
  - the marker becomes a new brain input once a switch version shows it
    works.

  On the marker itself: "if you think this is a good idea, you are welcome
  to reopen it." **Reopened as the fresh-spoil heap, not a scent at the
  face.** A dig-face pheromone tested negative in ants (Bruce 2015) and was
  never built here: it was dropped before any code on 2026-09-19 and again on
  2026-09-28. Fresh pellets do draw where ants start digging (Pielström &
  Roces 2013). The heap is what the marker still has to do: make the holes
  compete so one wins.

- **2026-09-28: food goes into a chamber.** Asked whether the lab's buried
  mouth should be kept clear or the food taken inside, the owner answered
  "Food in chamber". A joint step with the foraging lane (the drop and the
  homing are theirs); prototyped in scratch first.
- **2026-09-28: granary form B.** "Go ahead with B: Foragers drop food at
  the door; ants that stay home carry it into the storeroom."
- **2026-09-28: a simple test environment for this work.** "I don't think we
  should worry about plants or the standard lab bed during this development.
  Just use a simpler test environment for now." The granary is developed on
  the colony bed (`trailfollow`: soil, one food pile, 20 founders, no plants
  or weather) and `digbox`, not the lab box.

## Live question

**Is the colony building a nest? It digs one nest with one entrance area,
and since 2026-09-28 that is the shipped ant**
([`../nest-heap-cue-2026-09-28.md`](../nest-heap-cue-2026-09-28.md)).
- **2026-09-28, shipped:** the dig wiring, `(Bias, Dig, -0.3)` with
  `(SurfaceCurvature, Dig, -1.0)` (#505). Openings 27.5 -> 10, digging in
  one place ([`../nest-dig-wiring-2026-09-28.md`](../nest-dig-wiring-2026-09-28.md)).
- **2026-09-28, shipped on (#507):** the founding shaft (6 rows) and the
  heap cue (`K` 5, floor 0): a dig that would open the ground to the sky
  needs a heap beside it. Dig box openings 10 -> 4 (12 of 12); colony bed
  starved 201 -> 83; lab ties, and its founding mouth is buried by frame
  30,600 on 12 of 12 seeds under food and plants.
- **Dig down ships on for an enclosed digger** (#508), refused only
  where there is no way down (`way_down`): the creep stops (2 openings
  against 6 at frame 24,000 over 24 seeds, 22 of 24 nest-like against 12),
  and the colony bed pays (starved 83 -> 152, born 59 -> 30). Shipped on
  the lane's ruling; the turn everywhere stays off (starved 295). Refusing
  the turn more widely was measured six ways and every one lost the nest
  (report §13): the turn's nest and its price are one mechanism.
- **The creep:** a fresh heap would refuse at most a third to under a half
  of it, so it is not built.
- **Food in a chamber: the owner chose B** (foragers drop at the door, ants
  at home carry it in). Built as `PIXEL_PHYSICS_STOREROOM`, off
  ([`../nest-granary-2026-09-28.md`](../nest-granary-2026-09-28.md) §8): the
  room fills (3+ cells at mid-run on 14 of 24 with a door), and against the
  door alone it costs 14% of food taken and a third of the young. No ants
  stay home here, so the carriers are the fed ants that breed. The door
  alone now helps the bed a lot (starved 93 -> 15).

**Next** (2026-09-28):
1. **The storeroom (B).** Asked the owner whether to build ants that stay
   home (young ants work inside before they forage), which would make the
   carry cheap; it changes when an ant starts to forage, the foraging lane's
   region. Pieces in their region land as switches they review.
2. **Dig down's bed cost, traced** (report §16, packed lunch on). When food
   is wanted, 47% of the fed foragers at home are inside the nest with dig
   down, against 35%. From inside, 13-17% go out first, against 53-56% from
   the doorstep; inside they handle food (57-58%) or dig (26-28%). A dig
   gate for a forager the colony needs would reach only the diggers. The
   hunger gate is a dead end (report §11).
3. **The brain input** for the cue (owner's order); 24 genome slots, so
   planned with the lab lines.
4. **Chambers** need contents (brood or a granary), which the dig box lacks.

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
- **Food in a chamber (2026-09-28, their reply):** no objection; the drop
  and home-target pieces land as switches the foraging lane reviews. One
  definition of home for the drop, the pickup and the packed lunch: a
  pickup in the chamber must read as at home (`picked_at_home` on the same
  predicate as the chamber-as-home piece), or a fed ant ferries chamber food
  back out to the strip. Once #509 lands, colony baselines are taken with
  packed lunch on (`=off` for runs before it).
- **Still open for the foraging lane:** `trailfollow`'s header echoes
  `PIXEL_PHYSICS_NEST_DOOR` / `_SHAFT` / `_HOME` and prints
  `pickups_at_nest` (both done on their side), but reads the raw variable:
  an unset shaft prints `shipped`, which since #507 means on (6 rows), and
  `PIXEL_PHYSICS_SPOIL_CUE` is not echoed at all. Raised in the #507 poke.

## Predictions (written before each run)

Rows 1-22 (the mouth and the mound) are in
[`../nest-mouth-2026-09-26.md`](../nest-mouth-2026-09-26.md)'s appendix, and
rows 23-40 (the spoil footing) in
[`../nest-spoil-footing-2026-09-27.md`](../nest-spoil-footing-2026-09-27.md)
§6. New rows go in the report of the work they predict. The 2026-09-27 funnel
arms in `nest-work-2026-09-27.md` (haul, cover, dig-down, drop-away) were
exploratory and carry none.

## Cards with the owner

- 2026-09-26: `…52a96c` (one hole, 40 ants), `…77bab8` (gray is lining);
  `…e86359` withdrawn.
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

Older heads, from the branch cut (`636612c6`, 2026-09-26) through #507
(`97129556`) and the dig-down work merged as #508, are in `git log` and in
the reports each one names.

- `34e07b86` (#508) — merged: dig down on for an enclosed digger, refused
  only where there is no way down. GitHub deleted the branch; restarted
  from `main` the same day, same name.
- `117d8db6` — food in a chamber, a scratch prototype measured on the colony
  bed (`nest-granary-2026-09-28.md`).
- `f703472a` — `main` merged in: #509, packed lunch on. The colony bed's
  baseline is now 4,574 taken / 93 starved / 151 born (`=off` on dig down:
  5,377 / 64 / 185), reproduced exactly on this branch.
