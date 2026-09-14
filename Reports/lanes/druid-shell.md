# Lane C — the shell, held world, 2026-09-14

Coordinator: `session_01TngZpRY8LoqpFWHuXjUTTD`. Branch: `claude/druid-shell`,
cut from `main` at `9fd637a7` (PR #429). Files touched: `src/druid/menu.rs`,
`src/bin/druid.rs` (not the `offer` arm — Lane B's), `src/druid/mod.rs`
(new fields/methods only, per licence), `src/druid/hud.rs` (legend rows
only), `src/sim/structural.rs` (one new `World` method plus its tests — not
druid-owned code, but the bug it fixes lives there, not in the shell).

## Item 1 — plant bend/break: not a wiring bug, but a real one nearby

**Reproduced first, per `CLAUDE.md`'s method, rather than assumed.** The
owner's words — *"I am not sure it is actually changing it in the game right
away"* — are ambiguous between "not wired at all" and "wired but invisible",
and they turned out to be a third thing: wired, and *usually* visible within
a few seconds, but with one specific path that can silently never catch up.

`World::plant_bending` and `World::plant_load_failure` gate three checks:

1. `plant::break_under_load`'s stress snap and `plant::bend_under_load` —
   read live on **every organism tick** (`ORGANISM_TICK_INTERVAL` = 45
   frames, up to 5x for a large plant via `PLANT_SIZE_CADENCE` — worst case
   ~225 frames, under 4 seconds at 60 fps). Toggling either setting reaches
   these within a few seconds, by design. Nothing wrong here.
2. `structural.rs`'s `over_span` cantilever check and its detached-living-
   plant clause — read `plant_load_failure` live too, but only when a
   structural check is actually *scheduled* for that cell. A cell the switch
   currently exempts is deliberately **not rescheduled** — `over_span`'s own
   doc says so: "nothing more to do until something else disturbs this
   organism's own structure." A menu toggle disturbs nothing on its own.

So flipping `PLANTS BREAK UNDER STRESS` back on left any already-settled,
over-span beam standing exactly where it was, with no indication anything
was wrong — it would only come down the next time *something else* (growth,
damage) happened to touch it, which on a mature, non-growing stand could be
a very long wait, reading exactly as "the switch didn't do anything."

**Reproduction:** `sim::structural::tests::
flipping_the_load_failure_switch_the_way_the_menu_does_does_not_
retroactively_recheck_a_settled_beam` — builds a 12-cell over-span beam with
the switch off, lets it settle (200 organism-tick frames), flips the switch
on with a bare field write (exactly what `Setting::advance` used to do), runs
200 more frames, and the beam is still standing. Passes today, which is the
bug, not a guard against one.

**Fix:** `World::schedule_structural_recheck_of_all_living_plants` (new,
`src/sim/structural.rs`, beside `schedule_structural_check_around`) walks
every organism's `OrganismState::cells` and calls `schedule_structural_check`
on each. `Setting::PlantBreak::advance` (`src/druid/menu.rs`) calls it right
after flipping the field. `PlantBend` needed no equivalent — it is entirely
tick-driven (path 1 above), never queue-gated, so there is nothing for a
menu toggle to fail to re-trigger there.

**Cost:** one-off, on a menu keypress, not a hot-path or per-frame cost.
`schedule_structural_check` dedups into the existing scheduler queue, and
the queue's own per-frame cap spreads a large stand's catch-up over several
frames — graded, not a single-frame stall, which the ethos in `CLAUDE.md`
asks for anyway ("an outcome is a distribution, not a binary"). Unlike the
`dead-ends.md` warning about scheduling a check on *every terrain cell*
during worldgen (count-to-infinity while distances are still converging),
this scan runs against already-converged `support` values — it re-judges
the exempt/not-exempt boolean, it does not re-derive distances, so that
failure mode does not apply here.

**Guard:** `sim::structural::tests::
schedule_structural_recheck_of_all_living_plants_catches_up_a_beam_the_
switch_had_exempted` — same scene, but flips the switch through the new
method the way the menu now does. The beam comes down.

## Item 2 — restart

Owner: *"there should be a restart options."* `N` (free letters were B, J,
N, O, Y — this is the only one that reads as the verb).

**Restart is not free, and the shell says so rather than hiding it.**
`Druid::new` generates 2560x960 and grows it — about a minute of wall clock,
single-threaded, with nothing else able to run while it does. A restart that
fires the instant the key is pressed blocks the window immediately and any
"regenerating" message would race the freeze and lose. `Handler::
request_restart` instead arms a one-frame-deferred countdown: the keypress
sets a message (`Druid::note`) and `restart_countdown = Some(1)`; the frame
that follows draws and *presents* that message against the still-current
world; only the frame after that calls `Handler::perform_restart`, which is
where the actual generate-and-grow blocks. So the last thing on screen
before the freeze is the notice, not whatever the player happened to be
looking at.

`perform_restart` also resets what belongs to the run rather than the
window: the tick accumulator (a stale backlog would burn itself off as
catch-up ticks against a world that was never running while it built up),
held movement keys (else the new player is walked off whatever he spawns
standing on — mirrors `Handler::act`'s existing reason for clearing them
before a modal opens), and the biosphere page's history (a graph of a
species that no longer exists otherwise) — while leaving whether that page
was *open* alone.

Not built: a background thread for generation. That is a real architecture
change (the world, the renderer and the event loop are not currently set up
to hand work across threads), well past a "add a restart key" item, and the
brief did not ask for it — it asked to decide what the block does and say so
on screen, which this does honestly rather than promising something async
that isn't there.

## Item 3 — one bubble-size control

Owner: *"there should just be one bubble control size for the druid and
placeable bubbles."*

**Not a shared absolute radius — a shared offset from each circle's own
base.** The two circles start at different sizes for reasons that are still
true: `PLACE_RADIUS_START` (60) is what a placed circle needs to be worth
walking away from; `CARRIED_RADIUS` (28) is deliberately small so presence
stays free (`carried_cost`'s own doc, and the guarded invariant "the circle
you already are must stay free"). Sharing one absolute number would either
start the carried circle already costing power at the dial's own default
(60 > 28, immediately outside the free base), or cap every placed circle at
the carried circle's much smaller ceiling (96 against 240).

`carried_radius_for(place_radius)` (`src/druid/mod.rs`) computes
`(CARRIED_RADIUS + (place_radius - PLACE_RADIUS_START)).clamp(CARRIED_RADIUS,
CARRIED_RADIUS_MAX)` — at the dial's own start the offset is zero and the
carried circle reads exactly `CARRIED_RADIUS`, still free. Above that, both
circles grow together until the carried one hits its own lower ceiling and
pins there while the placed one keeps climbing to its own, higher one; below
it, symmetrically, the carried one pins at its floor (never below presence)
while the placed one keeps shrinking to its own lower floor.

`Druid::set_bubble_radius` is the one place both fields are written now;
`Q`/`E` in `src/bin/druid.rs` are the only callers, replacing `[`/`]`
entirely (removed, along with their legend row — folded into one merged
`Q E` row). The preview ring (what `SPACE` would place, at `place_radius`)
and the standing carried-circle ring (`world.carried`, already drawn
whenever the world is running) are unchanged and already both on screen at
once when the world is not held, so the player already sees both numbers
move together — nothing new needed there for legibility.

**Guard:** `druid::tests::
the_one_dial_leaves_the_carried_circle_free_at_its_own_default_and_clamps_
each_side_on_its_own_range` — asserts the free-at-default property survives
the merge (the property a naive shared-absolute-radius design would have
broken), and that each side pins at its own, independent range boundary
rather than the narrower of the two.

## The `structural.rs` boundary — the coordinator's own follow-up

The coordinator flagged, after the first commit landed, that `structural.rs`
is shared engine code (the outdoor sandbox and the evolution lab both run
it) and my brief never named it. Fair catch — my brief listed only
`src/druid/menu.rs`, `src/bin/druid.rs` and `src/druid/mod.rs`.

**Changing it was genuinely necessary here, not a choice**: item 1's actual
defect is in `structural.rs`'s own scheduling (see above), not in the
druid's wiring of it — the switch reaches the sim fine, it is the *queue*
that never gets re-armed. So the fix has to live where the bug does.

**The change is additive and unreachable from any existing call path.**
`schedule_structural_recheck_of_all_living_plants` has exactly two callers
in the whole tree: its own guard test, and `druid::menu::Setting::
PlantBreak::advance` — confirmed by grep, not assumed. `over_span`,
`break_free`, `schedule_structural_check`, every existing rule and every
existing call site are byte-for-byte untouched. Nothing in the outdoor
sandbox, the evolution lab, or any procedural-content path can reach the
new method, so there is no model to re-derive constants against and no
sweep result that could plausibly move.

**Paired evidence anyway, per the coordinator's ask:**
- `bash scripts/acceptance.sh` on this branch: **all five cases met their
  expectations** (`worked`, `undercut`, `ligament`, `strike`, `snap`/`fell`),
  frame-cost bars held (worst 7.32 ms against acceptance's much higher
  ceiling). This is the structural gate CI runs.
- `bash scripts/seedsweep.sh` (`dig=6` over 6 presets x 4 seeds, the
  order-statistic sweep `CLAUDE.md` asks for before any change to the load,
  bearing or fracture model): 24 runs, exit 0, numbers in the ordinary shape
  for this instrument (cells lost: max 788, p90 604, median 0; rock
  destroyed: max 6, p90 1, median 0) — no crash, no blown-up outlier, no
  NaN. Not a before/after diff (there is no "before" version of an addition
  with no prior callers to compare against), but confirms the sweep itself
  runs clean on this tree.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings`: clean.
`cargo test --release` (full suite, reaches `tests/*.rs`): **1,806 passed,
0 failed, 85 ignored** (lib) + 3/3 `tests/determinism.rs` + 44 passed / 18
ignored `tests/worldgen.rs` + 10/10 `src/main.rs` + 2/2 new `src/bin/
druid.rs` tests — no regressions anywhere in the suite.
`bash scripts/docscheck.sh`: clean (pre-existing lane-note size warnings
from unrelated lanes). `python3 scripts/deadendindex.py --touching`: 7
files changed, 0 identifiers named, 0 dead-end entries to write back.
`bash scripts/acceptance.sh` and `bash scripts/seedsweep.sh`: see above.

## Rendered

Verified live, headlessly (`xvfb-run` + lavapipe), not just described: the
options menu (item 1's row, `PLANTS BREAK UNDER STRESS`), and the on-screen
key legend showing the merged `Q E` row and the new `N` row, both fitting
the panel (`both_panels_fit_inside_the_viewport` also green). No review card
posted — nothing here is a judge-by-eye question (no new visual look, no
graded outcome to compare); all three items are "does the control do what
it now says it does," which the tests above answer directly.

## Head

See `PR_BODY_LANE_C.md` on this branch and the PR itself for the head SHA
and link.
