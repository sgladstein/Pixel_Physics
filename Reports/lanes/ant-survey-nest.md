# Lane N — the nest line's rejections, re-scored

*Ant-survey follow-up round, 2026-09-19. Coordinator
`session_01HTNLNphUPgpg5GqCwCQvmW`. Branch `claude/ant-survey-nest`,
**head `4a1fd533`**, **PR #477**, opened against `main` at `5f92c761`.
The report is [`nest-rejections-rescored-2026-09-19.md`](../nest-rejections-rescored-2026-09-19.md)
and it holds every number; this note is the state and what the next lane
needs.*

## In one line

**Two of the three faults the round suspected were real, fixing them changes
no verdict, and the lever that works was sitting in the tree default-off,
swept in the wrong axis.** `PIXEL_PHYSICS_NEST_SITE_ROWS=40` gives room
**1.44x bigger on 12 of 12** paired seeds and height-over-width **0.11 → 0.20,
taller on 11 of 12**; only its sibling `_NEST_SITE_COLS` had ever been scored,
and that one is the negative the shape report published.

## Status

| | |
|---|---|
| branch | `claude/ant-survey-nest`, head **`4a1fd533`**, 3 commits, pushed |
| PR | **#477** — the coordinator merges it |
| gates | `clippy` green, `cargo test` green (44 integration + lib, 0 failed), `docscheck` clean, `deadendindex --touching` names exactly the two entries that carry write-backs |
| review queue | blind A/B card **`20260919T151821144Z-da82d9`**, board `nest`, **unanswered at hand-off** — control against a 40-row site reach. Collect it with `review.py inbox` |
| shipped behaviour | **unchanged.** All four switches are default-off and bit-exact unset, so no wiki page moves |

## What is in the branch

Four default-off switches, each with its "it fired" counter, plus the
instrument work that makes any of it readable.

- **`PIXEL_PHYSICS_DIG_DOWN=<w>`** (`creature.rs`, the dig verb) — turns the
  digger one octant toward down **at the dig roll**, then digs where it is
  facing. That is neither of the two places this was tried: the target severs
  the dig-licenses-the-step coupling, and steering the walk measured negative
  over five arms. Room 1.51x on 12 of 12 from 1.37x the digging; aspect ratio
  only 1.18x on 9 of 12. **Volume, not shape.**
- **`PIXEL_PHYSICS_SPOIL_DROP_COVER=<w>`** (`creature.rs`, the drop verb) —
  `dead-ends.md`'s `LightHere` gate with `under_cover`, a per-cell in-the-open
  reading, in place of the sensor its entry blamed. **Fails harder**: 0.49x
  room, better on **0 of 12**.
- **`digbox seed=N`** — the box was seed-free and every published arm was one
  run. `seed=0` is the shipped walk and reproduces the published figures to
  the cell.
- **`digbox`'s `iqr`, `p50x`, `crop=`, and a mound-by-material census.**

**And one trap that is easy to walk into with these numbers**: `vert` is a
ratio, so a room that collapses in *width* reads as a taller nest.
`curvdig=+0.6` has the second-highest `vert` in the whole report on **half**
the room and nine rows of depth. Read `vert` with `room total` beside it.

## Three things the next lane should not have to re-derive

1. **The bounding box cannot rank two lenses, and three published negatives
   rest on it.** Over twelve seeds of the *unchanged* control, `room w` runs
   **96–191**. `iqr` — the columns holding the middle half of the room — is
   the statistic that can, and its selftest guard is red both ways: thirty
   cells in one chamber against thirty in ten scattered pits read `room total
   30 vs 30` and bbox `10w vs 172w`, while `iqr` reads 6 vs 96. **The max
   statistic calls scattering the bigger nest.**
2. **`line_burrow` is what erases the spoil marker**, and it is not a bug.
   The mound by material reads `packedsoil` **433** / `soil` 121 / `spoil` 96
   shipped, and `packedsoil` **0** / `soil` 171 / `spoil` 9 with
   `PIXEL_PHYSICS_BURROW_LINING=off`. The lining runs on all eight neighbours
   of every dig and reads `spoil.packs_into`, so two thirds of a mound is
   relabelled into gallery lining after it lands — and the same ablation
   collapses the room from 709 cells to 214, so it is load-bearing.
3. **Khuong's rule has a decision to make on one drop in ninety**, and the
   0.1% figure already on the record is the *dig* side's denominator, not
   this one. At the drop: **77% of pellets go up the column** with no
   neighbour to prefer at all; of the 641 that choose, 30 had both a marked
   and an unmarked candidate. `CreatureStats::spoil_drop_candidates*` counts
   it where the decision is taken. Declined before building, with both
   reopening conditions named in the new `dead-ends.md` entry.

## To Lane T, and to whoever lands these

**`src/sim/creature.rs` is the collision.** My three insertions sit at the
dig verb, the drop verb, and immediately **before** `fn adjacent_nest`.
`claude/ant-survey-trail` (read at `d344b5e7`) inserts `deposit_at_vacated`
immediately **after** `adjacent_nest`'s closing brace and changes the deposit
site in `creature_tick`. Different anchors, so they should merge — but land one
and read the other rather than trusting that sentence.

`src/sim/world.rs`: I appended five counters to `CreatureStats` and touched
nothing else.

`bash scripts/branchcheck.sh --who-touched` was run on both files before
writing; `main` was at `5f92c761`, with 51 landings in `creature.rs` in the
preceding week.

## What I would do next, and what I would not

**Would:** settle `PIXEL_PHYSICS_NEST_SITE_ROWS` as a shipped value rather
than a measurement switch, by eye. It is the only lever in this line that has
moved the aspect ratio, `adjacent_nest`'s site branch is *cheaper* than the
eight neighbour reads it replaces, and the owner's verdict on the shipped
control is on the record from this morning: *"looks like nothing. a hole
floating spoil"* (card `20260919T091527031Z-675ecd`). The card above is the
first half of that decision.

**Would not:** stack `DIG_DOWN` on it without asking. Together they give the
biggest room measured in this line (1.71x, 12 of 12) at the same `vert` as the
site reach alone, and `iqr` goes the **wrong** way — so they are not
complements, they are two different nests, and which one is wanted is a
question for the owner. Under `nest-biology-2026-09-19.md` §10 the nest has no
purpose in this game yet, so "better" can only mean "reads better", which is
not a lane's call.

**Would not:** build Khuong's rule, or a construction pheromone, until a
pellet stays a pellet. §4 of the report prices both.

## Left undone

- The card is unanswered. If the owner picks the control, the
  `NEST_SITE_ROWS` recommendation falls and the report's §0 should say so.
- `digbox` is one bare bed with no food, and `(FoodAdjacent, Dig, 0.8)` is the
  largest direct term in the shipped dig wiring, zeroed here by construction.
  Nothing above has been measured on the played bed, and `dead-ends.md`'s own
  `(Crowding, Dig)` entry records one sweep giving three different answers on
  three trunks of a single round.
- Nothing else in this line is still resting on the old census. Stage 3 went
  in at the end (report §2c): the sign survives, **the 2.3x is 1.31x on
  `room total`**, and the negative wire makes the nest *wider* on 10 of 12 —
  the only arm anywhere in this report that moves `iqr` upward.
