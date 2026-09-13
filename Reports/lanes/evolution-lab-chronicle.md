# The chronicle carries a census — lane note

Built for `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0. The
owner plays real sessions that reach hundreds to over a thousand ants, far
past anything a headless run produces here, and wants to hand a session's log
over for us to examine. This is what that handoff looks like.

## What the file is and where it comes from

`assets/chronicles/chronicle-<date>-<bed>-s<seed>.txt`, gitignored (a
player's own record, not authored content). Written by `Lab::write_chronicle`
on three occasions now: pressing `REBUILD`, closing the window, and **pressing
`9` at any point during play** (new — the key list names it under the other
key-only controls). All three call the identical export, so a mid-session
save and the one taken automatically on quit are the same shape.

**To get one from the owner: ask for the file at that path.** It never
uploads itself; it is a local text file the owner attaches or pastes. The
filename carries a date, not a time, so a second `9`-press the same day
overwrites the first save — `9` is for "I want this now," not a mid-session
series.

## What it carries

Four parts in order: a header (bed, seed, founders, colonies, frame, dial),
what dials the player changed from the shipped defaults (blank if none), a
**CENSUS** section, then the LOG/LEGENDS story the chronicle always had.

The CENSUS section is new. One row every 10,000 simulated frames (`Lab::
CHRONICLE_CENSUS_EVERY`, overridable via `PIXEL_PHYSICS_CHRONICLE_CENSUS_EVERY`
for a harness that cannot afford ten thousand real ticks), oldest row first,
frame first:

- **ants, plants** (living organisms, waiting seeds excluded), **seed bank**;
- **edible joules**, gut-priced through `creature::diet_yield`, split leaf /
  fruit / litter / seed / corpse / flower;
- **births, deaths** by cause (starved, killed, other), **eats, digs,
  deliveries**;
- the **nest's footprint**: roofed void (chambers), open pit, packed cells
  above and below the original surface, the mound's height;
- the **dead zone**: bare-ground ratio in the nest's ±64-column band against
  outside it, spelled out again in plain terms under each row — that ratio
  and the mound are what the owner watches (§ below).

Every column and every formatter live in `pixel_physics::lab::census`, one
function shared by this export, `examples/chronicle.rs`'s own `census=1`, and
`examples/latecensus.rs` (now that module's harness and positive control) —
so a number read off a real session's file and a number read off a headless
run are never two implementations of the same idea.

## What a session's log answers that a headless run cannot

Everything in `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0
turns on scale and on the owner's own interventions — neither of which a
scripted scenario reproduces:

- **whether the colony outgrows its larder before the bed can rebuild it**,
  read as the larder-by-kind columns falling while `ants` climbs;
- **whether digging quietly kills the ground above the nest** — "they dig
  large chambers underground and piles of dirt/chambers above the nest, which
  creates an area where plants don't grow" (the owner's own words) — read
  directly off the bare-ground ratio and the mound height, row over row;
- **what a real player did to the box mid-session** — a rebuild, a changed
  dial, a hand-placed jar — which the DIALS line and the CENSUS section's own
  discontinuities (a sudden `ants` drop with no `killed` to match) show up as
  events a scripted run never produces because nothing scripted it.

## Cost

The census scans the whole grid once per sample; at the 10,000-frame cadence
this is a call every few minutes of played time, not a per-frame cost. See
the PR body for the measured number on the played bed at 100,000 frames.

## Round 31: the chronicle carries the playtest, not only the census

Built for `Reports/evolution-lab-round-31-brief-2026-09-13.md` task 2. The
census answers *what the box did*; nothing before this round recorded *what
the player did to it*, so an agent reading a chronicle could only guess that
a jump in the numbers was a person pouring water rather than the sim.

**Player actions are `LogKind::PlayerAction`**, one new variant in
`src/sim/world.rs` beside `Born`/`Died`/etc. Six sites push one: `Lab::
release_at` (placed a jar), `wall_at` (added or removed), `begin_stroke`
(poured water -- once per stroke, not per painted cell), `adjust_param` (a
dial), `act`'s four speed arms (`TogglePhase`/`Slower`/`Faster`/`Preset`),
and `reset` (rebuilt, named after the scenario when one is loaded). Every
site already knew the exact sentence to say on the bar; `LogEvent::detail`
carries that same sentence into the log rather than inventing a bit-packed
encoding for six unrelated shapes of information (a jar's name and count, a
wall's column, a dial's name and value, a speed, nothing at all) the way
`other: u16` does for a milestone or a trait step. `format_log_line` in
`src/lab/ui.rs` prints `detail` verbatim, so the LOG page and the chronicle
file agree by construction -- there is only the one formatter.

**`PlayerAction` is a line event** (`LogKind::is_line_event()` now includes
it), so it shows in the chronicle's story and in the LOG page's default
LINES filter regardless of population -- the reason line events exist as a
category at all. It carries no organism identity (`id`/`born_frame` are
sentinel `0`, `species` is `SpeciesId(0)`), so `RunLog::about` excludes it by
kind explicitly rather than trusting that no real organism is ever `id 0,
born_frame 0` (the world's first organism legitimately is).

**Autosave**: `tick()` calls `write_chronicle()` on every CENSUS row, riding
the existing `CHRONICLE_CENSUS_EVERY` cadence rather than a new timer -- one
file write per 10,000 frames by default, not per frame. The filename grew a
`HHMMSS` timestamp alongside its date (`chronicle_timestamp()`, was
`today_utc()`) for exactly the reason the brief named: two saves on one day
used to overwrite each other, and the autosave turns "two" into "dozens".
Every save --autosave, `REBUILD`, quit, and `9`-- now gets its own file
rather than clobbering the last. This does mean a long session leaves many
chronicle files behind, each a strict superset of the ones before it; taking
the newest is always taking the whole story so far, and pruning older
autosaves is not this round's job.

**What it costs.** `write_chronicle` timed directly and repeatedly
(`autosave_cost`, `src/lab/mod.rs`, `#[ignore]`d, `labstats.rs::cost`'s own
paired shape) came back **~6.9 ms/call** at 86 organisms and 15 accumulated
CENSUS rows, against an idle control reading `0.000 ms`. At the shipped
10,000-frame cadence that amortises to under 0.001 ms/frame, and a real
session hits it once every several minutes -- imperceptible against the
whole-frame budget in the ordinary case. **The caveat**: the cost lands on
whichever simulated tick happens to cross the cadence, and at a high speed-
dial multiplier several thousand simulated ticks can run inside one drawn
frame, so the ~7-13 ms this call actually took (a paired per-tick reading in
the same test, small-sample and noisier, put a paying tick's *worst* frame
at ~13 ms) can in principle land inside a single displayed frame at extreme
dial settings. Nothing in this round changed that; riding the existing
CENSUS cadence rather than adding a separate timer is the cheapest amortising
choice available without changing what "autosave" means, and no further
optimisation was needed to bring the common case under the frame budget.
`ascii`'s worst-frame timing is silent on all of this: `ascii` drives
`sim::frame::step` directly on a bare `World` and never reaches `Lab::tick`,
which is where the autosave lives, so it is not the instrument for this
number -- `autosave_cost` is the direct equivalent for the lab's own tick
loop.

**The two-ring log**: `RUN_LOG_CAP` (2048) used to bound one shared
`VecDeque` holding `Born`/`Died`/`FirstFeed`/`FirstSeed` alongside
`LineEnded`/`GroupSplit`/`LineMilestone`/`LineRecord`. On the shipped
52-ant bed that was headroom for several sessions; on a played bed with
hundreds of ants, births and deaths alone could fill it and silently evict
the sparser line events -- including, now, every player action -- before a
reader ever saw them. `RunLog` now holds two `VecDeque`s: `individuals`
(`Born`/`Died`/`FirstFeed`/`FirstSeed`, capped by `RUN_LOG_CAP`) and `lines`
(everything `is_line_event()` names, capped by the new `LINE_LOG_CAP`, same
value). `push` routes on that predicate; `recent()` merges both rings by
frame so every other reader still sees one ordered log. Guarded by
`a_colonys_own_churn_cannot_evict_a_line_event` in `src/sim/world.rs`,
watched red first against the old single-ring shape. **The coordinator's own
flag on this round**: the pre-existing `the_run_log_reports_what_it_dropped`
only ever pushes `Born`, which lands on the `individuals` ring in both the
old shared-queue shape and the new split one -- real, but not evidence about
`lines`. `the_line_ring_reports_what_it_dropped` is the guard that actually
exercises `LINE_LOG_CAP`'s own trim and drop count, mirroring the older
test's shape on the ring `PlayerAction` now shares with `LineEnded`/
`GroupSplit`/`LineMilestone`/`LineRecord`.

**Guards, watched red first**: `examples/chronicle.rs` prints through
`format_log_line`, so a run that exercises all six player-action sites shows
them in its own output -- this is the acceptance test the brief asked for.
`a_log_line_fits_beside_the_cell_page` (`src/lab/ui.rs`) now carries a
representative `PlayerAction` case near the longest real sentence rather
than the shortest, since the width trap only bites a long one.
`a_player_action_never_joins_an_organisms_timeline` is the guard for the
`id 0, born_frame 0` collision above. Any test that runs `Lab::tick` past a
real `CHRONICLE_CENSUS_EVERY` boundary now writes a file -- `chronicle_
takes_one_census_row_per_cadence` and `line_events_are_bounded_per_lineage`
both cross it and both now redirect `CHRONICLE_DIR_ENV` to a private scratch
directory (`scratch_chronicle_dir`, the same tag-plus-pid shape `scenario.rs`
and `scene.rs` already use), so `cargo test` never writes into the shared
`assets/chronicles/` another lane's session could also be touching.
`autosave_cost` (`src/lab/mod.rs`, `#[ignore]`) is the paired per-tick timing
for the cost claim below, using `CHRONICLE_CENSUS_EVERY_ENV` to force the
cadence every 200 ticks rather than waiting out the shipped 10,000.

**One thing this touched that is not the chronicle**: `TimeControl::
react_on` (`src/lab/time.rs`) was a `u8` bitmask over `LogKind`'s
discriminant, one bit per kind. `PlayerAction` is the ninth kind, and a
ninth bit does not fit an eight-bit mask -- `1u8 << 8` is a shift-overflow
panic in a debug build the moment anything checked whether the clock reacts
to one. Widened to `u16`; `PlayerAction` is deliberately left out of
`default_react_on`'s own list, so the widening changes no default (the
clock reacting to the player's own action has nothing to point the camera
at and nothing to tell them they do not already know).
