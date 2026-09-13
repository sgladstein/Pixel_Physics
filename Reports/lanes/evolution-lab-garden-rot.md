# Round 28, third build — the schedule that never came back

*Follow-on to `Reports/lanes/evolution-lab-garden-fix.md`. One fix shipped,
one blocking threshold found and left alone, per this round's own brief.*

## The question this round answered

`pip_checks` (round 28's own diagnostic: the first Germinate evaluation a
delivered pip receives) read **0 on every seed, every prior round** — one
mechanism short of the ordinary germination path, which is already tested
and works. The brief's hypothesis was the transit-decay roll: does
`deliver_seed_passenger` feed `half_life_chance` the *carried span*, or the
organism's whole age?

**It feeds the carried span, correctly.** Read at the source and confirmed
with a new deterministic test
(`a_delivered_pips_transit_roll_uses_the_carried_span_not_the_organisms_whole_age`):
`frames_carried = world.frame.saturating_sub(passenger.picked_up_frame)`,
`half_life = world.species.get(sp).seed_half_life`. Round 27's own median
carry (175 frames against a 14,000-frame half-life) is genuinely ~99%
survival — the roll was never the bug.

## The real bug: the delivered pip's own schedule was never re-armed

`deliver_seed_passenger` wrote a fresh `CellType::Seed` cell into the grid
and stopped there. It never called `schedule_active_site`, unlike
`bear_seed_at` and the germinate-wait reseed, which both re-arm at the same
point they write a seed cell. `organism_tick`'s own "seed relocated"
recovery — the mechanism that would normally catch a schedule surviving a
seed's move — only works by searching `state.cells`, and a passenger-riding
organism owns **zero** cells; the recovery finds nothing to reschedule from
and silently drops the entry (`Vec::new()`). `ORGANISM_TICK_INTERVAL` is 45
frames against typical carries of 50-1,000+, so almost every delivery hit a
schedule already coming due mid-transit, dropped for good the instant it
did. This is why the round-28 trace read the pip as dying five frames after
set-down: nothing died: the organism just never got checked again.

**Fix**: one `world.schedule_active_site(reschedule_organism(x, y,
passenger.organism_id, 0, 0, world.frame + SEED_TICK_INTERVAL))` call at
delivery, the same interval `bear_seed_at` uses. New test
`a_delivered_pip_is_still_scheduled_for_its_own_germinate_check_after_a_long_carry`
puts the fault back (comments out the call) and goes red.

## A second bug, self-caught: the instrument undercounted its own subject

`pip_checks`'s Germinate-arm hook was gated on
`!organism.deferred_germination`. That flag is organism-wide and can already
be `true` from an earlier windfall-phase evaluation that has nothing to do
with this pip, so the gate skipped logging on exactly the runs where a
result mattered most — caught by the CLAUDE.md rule this file's own header
cites, applied to the instrument rather than the mechanism: one run showed
`plants_from_pip=1` with `pip_checks=0` for the same organism, which cannot
both be true. Fixed by removing the gate; `pip_checks` now logs every
evaluation unconditionally.

## Measurement, 3 seeds × 120,000 frames, `played_bed`, post-fix, on `main`
including PR #320's articulated-body landing

| seed | seeds_delivered | pips_rotted | pips_eaten | pip_checks | resting OK | light OK | water OK | plants_from_pip |
|---|---|---|---|---|---|---|---|---|
| 1 | 0 | 0 | 0 | 0 | — | — | — | 0 |
| 2 | 8 | 2 | 3 | 213 | 213 (100%) | 213 (100%) | **2** (0.9%) | **2** |
| 3 | 4 | 2 | 2 | 241 | 241 (100%) | 241 (100%) | **0** (0%) | 0 |

Seed 1 delivered nothing this run (no windfall was ever bitten down to a
pip that got carried), so its zero `pip_checks` is the correct quiet
reading, not a repeat of the bug — the positive control is seeds 2 and 3,
where deliveries did happen and checks now fire in the hundreds.

**Resting and light are never the blocker — 100% pass on both, every
check, both seeds.** Soil water is: of 454 checks across seeds 2 and 3,
only **2** ever read enough against herb's `soil_water_threshold` (0.15).
The per-check printed readings (`labforage`'s own rows) cluster hard at
**0.00** (211 of seed 2's, 189 of seed 3's 241) with a lesser band at 0.10
(51, seed 3 only) and one 0.12 — never above 0.12 except the two checks
that passed. `plants_from_pip` (2, 0) matches `pip_checks_ready` (2, 0)
exactly: every check that cleared water went on to germinate, nothing else
gates it.

**Per this round's own instruction: this is a threshold finding, not a
second fix.** Soil water at the pip's resting site is bone-dry to wilting
almost every time it is checked, in a colony-occupied bed. Whether that is
the delivered pip landing somewhere the colony's own traffic has dried out,
or the germinate threshold being tuned against a different scene, is
outside this round's scope — reported and left for whoever picks up the
garden loop's watering next, per the coordinator's "report which threshold
and by how much, and stop."

## Card

**`20260911T063740333Z-f83e2b`** on board `lab`: organism 4656 (seed 2),
delivered frame 45348 at (347,157), rolled to and rested at (347,161),
germinated frame 45356 (`pip_checks_water_ok` fired here — 1 cell to 2,
`GrowingTip` + `RootTip`), standing at 5 cells by frames 45539-46339. 28
frames, every 40th, ringed at the resting cell, `plants_from_pip=2` in
`meta`. **The first garden-loop pip to actually become a plant, across all
three rounds of this line.**

## Files touched

- `src/sim/plant.rs` — `deliver_seed_passenger`'s missing reschedule (the
  fix); the `pip_checks` gate (the self-caught instrument bug); two new
  tests.
- `examples/labgif.rs`, `examples/labforage.rs` — unchanged this round
  (prior rounds' `PIP_TRACE=`/`PIP_PROBE=`/`mark=`/`png_dir=` flags did the
  work of building the card).
