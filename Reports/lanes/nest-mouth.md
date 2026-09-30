# Lane note — the nest's mouth

*Kept current, edited in place. Findings live in
[`../nest-mouth-2026-09-26.md`](../nest-mouth-2026-09-26.md); this note keeps
the live question, what is addressed to another lane, predictions and heads.*

- **Session:** `session_01WF4wABj2ewSmzWVJTk6DsC` (the nest lane: the mouth,
  then nest building and spoil).
- **Branch:** `claude/ant-nest-mouth-4f6s79`. GitHub deletes it on every
  merge; restarted from `main` after #513, 2026-09-29, same name.
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
  the door; ants that stay home carry it into the storeroom." Then: "Some
  ants should stay home... we should have two different types of ants or
  castes" -- castes, built as a switch first (report §8h).
- **2026-09-28: a simple test environment for this work.** "I don't think we
  should worry about plants or the standard lab bed during this development.
  Just use a simpler test environment for now." The granary is developed on
  the colony bed (`trailfollow`: soil, one food pile, 20 founders, no plants
  or weather) and `digbox`, not the lab box.
- **2026-09-29: "Full granary on my default."** The door
  (`NEST_DOOR_SHIPPED`, 2) and the whole storeroom
  (`Storeroom::SHIPPED`, `on,caste=4,workerhome,side,keep`) ship on; `off`
  on either is the ant before.
- **2026-09-29: the carry yes, stacking yes, widening waits.** "Q1 - Yes"
  (carry pellets away from the mouth); "Q3 - this seems much better for the
  200 ant tests" (stacking); widening "looks slightly better with the 40
  ants... Everything again is broken at 200-ants so no impact on Q2 but
  needs more thinking." The 200-ant pile is the owner's "huge problem"
  (report §14).
- **2026-09-29 (night): the dig box fed; the colony lives inside its
  nest.** "Yes and Yes" -- `digbox` keeps its colony fed by default
  (`hungry` for the old box), and the next nest step is resting ants
  living inside the nest. "But the tamped tunnel walls and the nest
  refilling itself are both big issues? ... chambers fully enclosed by
  tamped soil and big blocks of tamped soil." And: "Make sure you look at
  the image to see the actual issue for yourself" -- zoom the sheet before
  describing it.
- **2026-09-28: build at one size, then expand -- to larger colonies.** "It
  is fine if we develop the next on a certain ant size and then expand it
  ... We eventually do want our nests to work for larger colonies." Every
  nest measurement also runs at 200 ants in `digbox`, and every colony one
  at 80 founders on the bed; what scales is in
  [`../nest-colony-size-2026-09-28.md`](../nest-colony-size-2026-09-28.md) §3.

## Live question

**Stacking at 4 by default** (owner: yes). The foraging lane's gate is a
24-seed lab pair and a bed pair. The lab pair first failed (20 of 24 boxes
extinct) on one engine bug, parted tissue written back over a stacked
nestmate: traced 7 of 7 and fixed (`close_or_hand_over`,
[`../creature-stacking-design-2026-09-17.md`](../creature-stacking-design-2026-09-17.md)
§12; cap 1 byte-identical on 24 seeds). After the fix: 3 of 24 extinct, no
kills, but births 418 -> 224.5 (lower on 19 of 24). Bed pair: no harm. Not
flipped until that cost is traced or the owner rules.

**Tamped blocks and sealed chambers** (report §15): walking the soil out
with the carry cuts most of them. Asked the owner (2026-09-29, night)
whether walked + carry + "pellets only on real ground" becomes the default.

**Next** (2026-09-29, night):
1. Land this branch (the fix, instruments, reports): PR, merge on green.
2. The lab cost of stacking: trace where a stacked colony loses its young
   (per ant it eats the same; it moves 14% less and visits the nest 3x as
   often), then the default flip as its own PR. It reaches all three games,
   including the lab's BOX dial ("AT 1, THE SHIPPED SETTING"); poke first.
3. The refill fix as a default, when the owner answers.
4. Resting inside (owner: yes), joint with the foraging lane. Their terms:
   a switch measured on their bed (food taken, starved, fed departures);
   keep `trip_load`, the door's geometry and `NestSite::surface`; poke
   before it lands. The lever: `home_pull` gives a fed ant that is not a
   nest worker no pull home, so it rests wherever it stops.

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

**2026-09-29: "I cannot review the queue, post questions/images in this
chat."** Show pictures with `SendUserFile` and ask in chat; the queue cards
are the record, not the channel. Same day: digbox sheets in the lab's
colours (`look=lab`, default; render-only). Answered in chat: the carry
(`…04be5b`) yes, stacking yes, widening (`…ca2baa`) waits. Superseded by
§11's trace: `…2d6747` (relay) and `…f4ca75` (the lab's buried mouth).
Older cards are named in the reports that posted them.

## Head SHAs

Older heads (branch cut `636612c6` through #512 and #513) are in `git log`
and the reports each one names.

- `7ee0e338` (#517) — merged: the entrance ledger, `SPOIL_OUT` and
  `SPOIL_LIFT=out` (off), `TRIPS` / `decisions=` in digbox. Branch
  restarted from `main`.
- R3 of the foraging lane's review (findings 2-5): the half turn's coin,
  the enclosed-gate test, the founding cut's corpses and jaw (report §18).
