# Lane note — the nest's mouth

*Kept current, edited in place. Findings live in
[`../nest-mouth-2026-09-26.md`](../nest-mouth-2026-09-26.md); this note keeps
the live question, what is addressed to another lane, predictions and heads.*

- **Session:** `session_01WF4wABj2ewSmzWVJTk6DsC` (the nest lane: the mouth,
  then nest building and spoil).
- **Branch:** `claude/ant-nest-mouth-4f6s79`. GitHub deletes it on every
  merge; restarted from `main` after #522, 2026-09-30, same name.
- **Peer:** the foraging-loop session `session_01Pt5N39pfcix13hMycPN9Xs`,
  branch `claude/ant-foraging-loop-handoff-986v7n`, lane note
  [`foraging-loop.md`](foraging-loop.md). The only other agent on the repo.

## Standing owner rulings for this lane

- **2026-09-26: "Make sure you are not using too many ants in your tests and
  always take snapshots at multiple times."** `digbox` at 40 ants with
  `energy=1000` (under the budding threshold); every picture several stops.
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
  recommendations": `SPOIL_FOOTING=ground` stays a switch, and the marker
  (reopened as the fresh-spoil heap, shipped as the heap cue; a dig-face
  scent tested negative in ants, Bruce 2015) becomes a brain input once a
  switch shows it works.

- **2026-09-28/29: the granary.** "Food in chamber"; form B (foragers drop
  at the door, ants that stay home carry it in, castes); then "Full granary
  on my default", so the door and the whole storeroom ship on.
- **2026-09-28: a simple test environment for this work.** "I don't think we
  should worry about plants or the standard lab bed during this development.
  Just use a simpler test environment for now." The granary is developed on
  the colony bed (`trailfollow`: soil, one food pile, 20 founders, no plants
  or weather) and `digbox`, not the lab box.
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

## Live question: the package ships on (2026-09-30)

**Session:** the "Ant nest next steps" project thread, branch
`claude/ant-nest-next-zshb8u`. Threads push and open draft PRs; a merge
desk thread merges (owner, 2026-09-30). New features ship on unless a
measured trade-off says otherwise (owner, same day).

The flip PR turns on stacking 4, the walked cycle and births on nestmates
(report §18). On the colony bed (`main` with #526) a fifth more food
taken, 281 -> 529 born at 90 cells and 119 -> 199 at 140, fewer starved,
a sixth less energy held in bodies. The lab gate on the same `main` is the
one harm: births flat, starved per ant-frame 9.7 -> 12.0 (higher on 18 of
24), a bigger peak colony; not a veto under the 2026-09-27 ruling above. It also widens the dying carrier's pellet search to
eight rings (a sink stacking exposed).

**Next, in order:**
1. Why 24% of pellets still go down inside and refill the lower tunnels
   (the pale blocks in the colour-coded 200-ant sheet): carriers whose
   patience runs out on the way to the door. Per-ant trace, §17's method.
2. Why the lab box starves more with the package (§18): trace who
   starves and when, against the peak. Not a blocker; for the foraging lane.
3. Resting inside (owner: yes), built on the existing room gate's crowding
   count so the breeding plan's brood (B1, B3) can plug into it.
4. The storeroom's fixed west side (`week-review-2026-09-29.md` W2); the
   foraging lane's even-sided food drop has landed (#526) (theirs: the colony's
   east lean was `food_drop_site`, not nest code).

Pictures: `digbox ... gridout=`, then `scripts/nestgrid.py RUN.grid --png
OUT --frame F` (colour-coded; crumbled spoil yellow, refilled holes violet
since 2026-09-30), cropped x 62-138, y 16-52 and upscaled.

## How this lane measures

Every tool below is committed; nothing depends on a session's scratch.
`RAYON_NUM_THREADS=1` on every run whose counts are compared. Before a long
sweep, copy the example binary into the run directory so a rebuild cannot
swap it mid-run.

- **The nest**: `digbox fed nulls=0 energy=1000 w=200 soil=60
  frames=24000 stops=0,12000,24000 seed=S gridout=D/<arm>-a<N>-s<S>.grid
  out=D/<arm>-a<N>-s<S>.png > D/<arm>-a<N>-s<S>.log`, 8 seeds at 40 and at
  200 ants; `python3 scripts/nestgrid.py --pair D 24000 base new`.
- **The lab gate**: `labforage scenario=played_bed frames=120000 seed=S
  > D/<arm>-S.log`, 24 seeds; `python3 scripts/labpair.py D base new`. A
  lab arm that sets `STACK_DEPTH` must pass `bedenv`; never export the bed's
  `COLONY_SPACING` or `BUD_SITE` into it.
- **The colony bed**: `PIXEL_PHYSICS_COLONY_SPACING=2
  PIXEL_PHYSICS_BUD_SITE=nest trailfollow mode=gap gate=shipped
  frames=24000 ants=20 near=10 food=400 refill=400 arms=self gaps=90
  seeds=24 seed0=1`, one log an arm; `scripts/antloop.py --log new.log --vs
  base.log`.
- **Why an ant did something**: a temporary env-gated `eprintln` at the
  decision, the run's stdout checked identical to the untraced run, parsed
  per individual, removed before the commit (report §17's carry trace).
- **Pictures**: crop each stop to the nest and upscale 2-3x (nearest), today
  beside the arm at 12,000 and 24,000; the whole sheet is unreadable.
- **Predictions** are written before each run and scored in the report.
- **This cloud harness**: never `cd` in a command (it moves the session to
  the main checkout); absolute paths and `git -C`.

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
- **Their standing term (2026-09-28):** one definition of home for the
  drop, the pickup and the packed lunch, or a fed ant ferries chamber food
  back out.
- **Still open for the foraging lane:** `trailfollow`'s header prints an
  unset shaft as `shipped` (on since #507) and does not echo
  `PIXEL_PHYSICS_SPOIL_CUE`. Raised in the #507 poke.

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
chat."** Show pictures with `SendUserFile` and ask in chat. Answered there:
the carry (`…04be5b`) yes, stacking yes, widening (`…ca2baa`) waits. Older
cards are named in the reports that posted them.

## Head SHAs

Older heads (branch cut `636612c6` through #512, #513 and #517) are in
`git log` and the reports each one names.

- `da0b7b41` (#522): merged. Stacked ants are no longer killed by parted
  tissue; the dig box is fed and in lab colours; the carry is on under the
  walked cycle; reports §11-§15. Branch restarted from `main`.
- `6ec36431` (#524): merged. Births on nestmates, the carry's latch, report
  §16-§17, stacking §13; all off by default. The next PR on this branch
  carries `scripts/nestgrid.py` and this note's method section.
