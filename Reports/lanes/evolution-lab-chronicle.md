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

## Round 31: the chronicle carries the playtest, and then its own load

Built for `Reports/evolution-lab-round-31-brief-2026-09-13.md` task 2, plus
one owner-requested addition landed in the same PR. Full mechanism and
numbers are in the PR body (#374); this is the pointer and what outlives it.

**Player actions**: `LogKind::PlayerAction` (`src/sim/world.rs`), pushed
from six `Lab` verbs (`release_at`, `wall_at`, `begin_stroke`, `adjust_param`,
`act`'s speed arms, `reset`), each carrying its own sentence verbatim in a
new `LogEvent::detail` field rather than a bit-packed `other: u16` encoding
-- six unrelated shapes of information have no one encoding.
`format_log_line` (`src/lab/ui.rs`) prints `detail` directly, so the LOG page
and the chronicle file can't disagree. It is a line event
(`is_line_event()`), so it survives population growth and shows by default;
it carries no organism identity (sentinel `id 0, born_frame 0`), so
`RunLog::about` excludes the kind by name rather than trusting that no real
organism is ever that pair (the world's first legitimately is).

**Autosave**: `Lab::tick` calls `write_chronicle()` on every CENSUS row
(`CHRONICLE_CENSUS_EVERY`, unchanged cadence). The filename gained an
`HHMMSS` timestamp (`chronicle_timestamp()`, was `today_utc()`) so autosave,
`REBUILD`, quit and `9` stop overwriting each other. Cost: `write_chronicle`
timed directly (`autosave_cost`, `#[ignore]`d) reads **~6.9 ms/call** at 86
organisms, under 0.001 ms/frame amortised at the shipped cadence -- but a
high speed-dial multiplier can run many ticks inside one drawn frame, so
that single call could in principle land inside one. `ascii` cannot answer
this cost question at all: it drives `frame::step` directly and never
reaches `Lab::tick`.

**Two-ring log**: `RunLog` used one shared `VecDeque` for `Born`/`Died`/
`FirstFeed`/`FirstSeed` and the line kinds together, so a big colony's own
churn could evict the sparse line events (now including every player
action) before a reader saw them. Split into `individuals` (`RUN_LOG_CAP`)
and `lines` (new `LINE_LOG_CAP`), routed by `is_line_event()`, merged back
into one ordered view by `recent()`. Guarded by
`a_colonys_own_churn_cannot_evict_a_line_event`. The coordinator flagged
that the pre-existing `the_run_log_reports_what_it_dropped` only pushes
`Born` and so is not evidence about the `lines` ring either way --
`the_line_ring_reports_what_it_dropped` is the guard that actually is.

**`TimeControl::react_on`** (`src/lab/time.rs`) was a `u8` mask over
`LogKind`'s discriminant; `PlayerAction` is the ninth kind and `1u8 << 8`
panics. Widened to `u16`; `PlayerAction` stays out of the default reaction
set on purpose (nothing to point the camera at).

**Guards, watched red first**: `a_chronicle_file_shows_player_actions_end_
to_end` / `a_rebuild_names_the_run_it_started` (full pipeline, real file
read back), `a_player_action_never_joins_an_organisms_timeline`, `a_log_
line_fits_beside_the_cell_page` (width, with a representative long
`PlayerAction` case), and `examples/labui.rs`'s new `PAGE: Log` tile --
nothing had ever rendered the one page this feature writes into, so content
was tested end-to-end while layout was not. Any test crossing a real
`CHRONICLE_CENSUS_EVERY` boundary now redirects `CHRONICLE_DIR_ENV` to a
scratch dir (`scratch_chronicle_dir`) so `cargo test` never touches the
shared `assets/chronicles/`.

### The chronicle also records load, not only content -- owner's request

Nine new fields on `ChronicleRow`/`PerfSample`, all reads of state
`TimeControl`/`World` already compute for their own screen readouts --
`wall_clock_secs`, `awake_chunks` (`World::active_chunk_count`, O(chunks)),
`active_sites` (`World::active_site_count`, O(1)), and six perf fields
(`ticks_per_frame`, `requested_ticks_per_frame`, `speed_multiple`,
`display_hz`, `debt_ticks` off newly-`pub` `TimeControl::owed_ticks`,
`draws_skipped` off a new running counter). `perf: Option<PerfSample>` is
`None` for a harness with no `TimeControl` (`examples/chronicle.rs`) --
`row_line` prints `--`, never `0`, since `0` would read as a real stall.
`awake_chunks`/`active_sites`/`wall_clock_secs` need no dial and are always
sampled. **Read `speed_multiple` beside every other perf field**: the same
achieved rate means "fine" at 1X and "badly behind" near the top of the
ladder. `examples/latecensus.rs` is untouched and gets none of this (out of
scope, no `TimeControl`, and its own header already diverged from
`header_line`'s before this). No per-phase timing was added to the live
loop -- `scale_probe phases=` is for that, headlessly.

Guards: `no_time_control_gives_dashes_not_zeros`, `a_real_time_control_
fills_every_perf_column`, `chunk_and_site_counts_need_no_time_control`
(`src/lab/census.rs`), `draws_skipped_counts_exactly_the_undrawn_passes`
(`src/lab/time.rs`) -- each hand-verified red first.

**Process note, binding for every future chronicle lane**: the owner
declined a review card asking for a real session file -- *"this is a bad
use of the review tool... general questions or requests should be sent to
the coordinating agent to tell me."* **The queue is for the eye; everything
else routes through the coordinator.** A request for a file, a preference,
a decision or an answer is not a judge-by-eye question and does not belong
on a card, however it is dressed up with a picture. Also recorded in
`Reports/lanes/evolution-lab-coordinator.md`, which binds the whole
programme, not only this lane.
