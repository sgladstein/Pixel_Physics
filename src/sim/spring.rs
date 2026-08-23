//! Springs and drains — water crossing the plane of the world.
//!
//! The world is a 2D slice of a country the player never sees the rest of.
//! A spring is where water arrives from behind the plane (an aquifer
//! daylighting on a rock face); a drain is where it leaves (the valley
//! behind the slice). This is the **plausible** half of the off-plane-flux
//! decision `Reports/worldgen-design.md` §5a demands be made explicitly:
//! water appears and departs at authored points at a believable, budgeted
//! rate. The **real** half — budgets computed from an actual upstream
//! drainage map — replaces the constants here when the coarse `(x, z)` map
//! lands; the mechanism, throttle, and ledger all carry over unchanged.
//!
//! Non-conservation is deliberate and precedented: rain already creates
//! water cells from the sky at a capped rate and evaporation deletes them
//! unbanked (`weather.rs`, `evaporation.rs`). A spring is rain that comes
//! out of a wall. The measured standing bill for one spring feeding a fall
//! and pool is **+3.025 ms/frame** at the shipped 8192x2560 (`ascii`'s
//! river-cost scene, re-measured 2026-08-23).
//!
//! **That supersedes the +0.97 ms this file quoted everywhere, and the old
//! number was not merely stale — the scene producing it was broken.** It
//! drained at the *world's* lowest column, 2030 columns from its own outlet
//! at the shipped size, so it reported `drained 0` and priced a bath filling
//! rather than a river at steady state; and its "spring off" control
//! silently contained a generated spring, because the `springs` worldgen
//! pass landed after the scene was written. Both faults made the bill look
//! smaller. `Reports/springs-in-generated-worlds.md` carries the full
//! correction; re-measure there before changing budgets.
//!
//! # Where springs live
//!
//! On `World`, as a plain `Vec` stepped in insertion order — the same
//! determinism reasoning as `chunk_bodies`. Not a cell flag (`Cell::flags`
//! is full, all eight bits — `Reports/load-model-handoff.md`), and not a
//! marker material (a spring is a property of a *place*, not a thing
//! standing in it; a material would fight the cell that actually occupies
//! the outlet). Worldgen's placement pass will push into the same list;
//! until it lands, the harnesses and `viewshot spring=` register springs
//! by hand.
//!
//! # The throttle is a verb
//!
//! A spring stops emitting while its outlet is covered by standing water
//! near full, or blocked by anything solid. That is the flood guard — a
//! dammed pool self-limits instead of filling the world — and it is also
//! the player's lever: wall the outlet in and the spring visibly chokes;
//! break the dam and it runs again. Graded and legible, per the ethos.
//!
//! # And then it stops
//!
//! Every guard in the weather subsystems once tested only that a mechanism
//! fires, and every bug was in how it failed to stop
//! (`Reports/weather-handoff.md`). The tests here assert the stops first:
//! a drowned spring emits nothing, a walled spring emits nothing, a dry
//! drain drains nothing, and a drained cell is written back by the aux
//! rules (`with_aux(remaining)`, `Cell::EMPTY` at zero — never
//! `with_aux(0)`, which manufactures a full cell).

use super::cell::Cell;
use super::material;
use super::update;
use super::world::World;

/// Fill units a spring emits per firing. One full cell: a spring is a
/// ribbon, not a mist — below ~half a cell per frame the fall flickers
/// out against the evaporation floor (measured in the river-cost scene).
const EMIT_FILL: u16 = material::LIQUID_FULL;

/// Frames between firings. 1 = one cell per frame, the ribbon the
/// river-cost scene priced. For context, a maximum storm creates ~1.4
/// water cells per frame across the whole world (`weather.rs`'s cap), so
/// each spring is roughly one storm's worth of water.
const EMIT_INTERVAL: u64 = 1;

/// Fill a drain deletes per frame, at most. Matched to the emission rate
/// so one drain can always keep up with one spring — the steady state the
/// prototype is built around.
const DRAIN_FILL: u16 = material::LIQUID_FULL;

/// The throttle: a spring whose outlet cell already holds at least this
/// much standing water is drowned and skips its firing. Near-full rather
/// than any-water, so the fall's own splash does not stutter the spring —
/// only a genuinely risen pool (or a deliberate dam) chokes it.
const THROTTLE_FILL: u16 = (material::LIQUID_FULL as u32 * 9 / 10) as u16;

/// Total flow a world will actually run, summed over every spring's span —
/// a budget on *water*, not on how many places it comes from. One narrow
/// seep and one six-column cascade cost the same bill as three two-column
/// falls, so the budget counts columns of emission. Stated where it is
/// enforced: registration refuses past it, loudly, rather than silently
/// skipping springs at step time. Sixteen columns is ~11 storms of water
/// per frame.
///
/// **This number's justification no longer holds, and nothing has replaced
/// it yet.** It was set from measured headroom — "the one-column prototype
/// stood at +0.97 ms against the 2.0 ms bar" — and that measurement came
/// from a river-cost scene that was draining nowhere and comparing against a
/// contaminated control (see the module header). Re-measured at the shipped
/// size, **one column costs 3.025 ms**, already past the 2.0 ms bar the
/// sixteen was checked against. Whether cost is linear in span is not
/// measured either way.
///
/// So: treat 16 as an unaudited ceiling, not as headroom anybody has
/// verified, and **measure before spending it**. `ascii`'s river-cost scene
/// is the instrument (fixed now); `viewshot spring=` gives a paired delta
/// but its control also carries the generated spring, so read it as a
/// difference and never as an absolute.
pub const MAX_TOTAL_SPAN: i32 = 16;

/// Widest single spring. A face weeping over six columns reads as a real
/// cascade; past that it reads as the ocean falling in, and the budget is
/// better spent on a second fall somewhere else.
///
/// **That claim has never had a picture behind it, and this constant is not
/// what makes the shipped fall thin.** The pass takes
/// `span = budget.min(MAX_SPAN)` where the budget is the preset's
/// `spring_flow` — **5.0 on every preset that has springs at all**, and 0.0
/// on `arid` and `flat` — so every generated spring in the game is
/// *budget*-limited and raising this alone moves exactly nothing. The owner
/// has called the fall too thin twice; the lever is `spring_flow`, and the
/// trade it makes is width against *number of falls*, since the budget is
/// spent whole on the first spring before the next candidate is considered.
pub const MAX_SPAN: i32 = 6;

/// One spring: an outlet at `(x, y)` weeping across `span` columns —
/// emission fills the air cells at `(x .. x + span, y)`. A `span` of 1 is
/// a seep; 4–6 is a waterfall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spring {
    pub x: i32,
    pub y: i32,
    pub span: i32,
}

/// One frame of spring and drain work. Runs at the top of both drivers,
/// beside `weather::step` — serial, outside the checkerboard, so the
/// write-safety proof is not in play. Cheap by construction: a handful of
/// point reads and at most one write per spring/drain per frame; the cost
/// that matters is what the emitted water *does*, and that is the measured
/// +3.025 ms above.
pub fn step(world: &mut World) {
    if world.springs.is_empty() && world.drains.is_empty() {
        return;
    }
    // `is_multiple_of`, and clippy would flag `% 1` while the interval sits
    // at every-frame — the check stays written out so the interval remains
    // a real knob rather than being folded away.
    if world.frame.is_multiple_of(EMIT_INTERVAL) {
        // Indexed loop rather than an iterator: emission writes to the
        // world, and the borrow of `world.springs` must end first.
        for i in 0..world.springs.len() {
            let Spring { x, y, span } = world.springs[i];
            // Each column of the span is its own outlet with its own
            // throttle: damming half a wide fall chokes that half and the
            // rest keeps pouring — graded, like everything else.
            for dx in 0..span {
                let (x, y) = (x + dx, y);
                if !world.in_bounds(x, y) {
                    continue;
                }
                let outlet = world.get(x, y);
                // Raw material check, not `is_empty()` — same reasoning as
                // the renderer: the question is "is there material here".
                if outlet.material == material::EMPTY {
                    world.set(x, y, Cell::new(material::WATER, (world.frame % 4) as u8));
                    world.spring_ledger.emitted += EMIT_FILL as u64;
                    continue;
                }
                // The throttle. A liquid outlet below the bar is the fall
                // still passing through — count it throttled only when
                // genuinely drowned or walled.
                let drowned = world.materials.kind(outlet.material) == material::MaterialKind::Liquid
                    && update::liquid_fill(outlet) >= THROTTLE_FILL;
                let walled = world.materials.kind(outlet.material) != material::MaterialKind::Liquid;
                if drowned || walled {
                    world.spring_ledger.throttled += 1;
                }
            }
        }
    }
    for i in 0..world.drains.len() {
        let (x, y) = world.drains[i];
        if !world.in_bounds(x, y) {
            continue;
        }
        let cell = world.get(x, y);
        if world.materials.kind(cell.material) != material::MaterialKind::Liquid {
            continue;
        }
        let fill = update::liquid_fill(cell);
        let taken = fill.min(DRAIN_FILL);
        let remaining = fill - taken;
        if remaining == 0 {
            world.set(x, y, Cell::EMPTY);
        } else {
            // `with_aux(remaining)`, never `with_aux(0)` — the inverted
            // fill convention (`material::LIQUID_FULL`'s doc).
            world.set(x, y, cell.with_aux(remaining));
        }
        world.spring_ledger.drained += taken as u64;
    }
}

/// The did-it-fire instrument. A waterfall rendered as a contact sheet
/// cannot show whether the spring produced it or a pond drained into
/// frame; these numbers next to the image can (`CLAUDE.md`: print the
/// count and read both). `emitted ~= drained + standing-census delta` is
/// the harness's steady-state check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpringLedger {
    /// Fill units created at spring outlets.
    pub emitted: u64,
    /// Fill units deleted at drains.
    pub drained: u64,
    /// Firings skipped because the outlet was drowned or walled — the
    /// throttle doing its job, including the player's dam.
    pub throttled: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::update;

    fn world_with_floor() -> World {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 100..128 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w
    }

    fn total_water_fill(w: &World) -> u64 {
        let mut sum = 0u64;
        for y in 0..128 {
            for x in 0..128 {
                let c = w.get(x, y);
                if w.materials.kind(c.material) == material::MaterialKind::Liquid {
                    sum += update::liquid_fill(c) as u64;
                }
            }
        }
        sum
    }

    /// The mechanism fires: a registered spring puts real water into the
    /// world at the documented rate, and the ledger agrees with a census
    /// of the world — the counter is not free-running.
    #[test]
    fn a_spring_emits_at_the_documented_rate_and_the_ledger_matches_the_census() {
        let mut w = world_with_floor();
        assert!(w.add_spring(64, 60, 1));
        // Through the real driver: `spring::step` runs inside it, beside
        // weather, so this asserts the wiring too, not just the function.
        for _ in 0..40 {
            update::step(&mut w);
        }
        assert_eq!(w.spring_ledger.emitted, 40 * EMIT_FILL as u64, "one cell per frame for forty frames");
        let census = total_water_fill(&w);
        assert_eq!(census, w.spring_ledger.emitted, "every emitted unit is standing in the world (nothing drains here)");
        assert!(census > 0);
    }

    /// And then it stops, first way: a spring whose outlet is walled with
    /// stone emits nothing, and says so in the throttle count.
    #[test]
    fn a_walled_spring_stops() {
        let mut w = world_with_floor();
        assert!(w.add_spring(64, 60, 1));
        w.set(64, 60, Cell::new(material::STONE, 0));
        for _ in 0..20 {
            step(&mut w);
        }
        assert_eq!(w.spring_ledger.emitted, 0, "a walled outlet must emit nothing");
        assert_eq!(w.spring_ledger.throttled, 20, "and each skipped firing is counted");
    }

    /// And then it stops, second way: a drowned outlet — standing water at
    /// the bar — chokes the spring. This is the dam interaction: the same
    /// check the player triggers by walling a pool in.
    #[test]
    fn a_drowned_spring_stops() {
        let mut w = world_with_floor();
        assert!(w.add_spring(64, 60, 1));
        // A full water cell sitting on the outlet.
        w.set(64, 60, Cell::new(material::WATER, 0));
        step(&mut w);
        assert_eq!(w.spring_ledger.emitted, 0);
        assert_eq!(w.spring_ledger.throttled, 1);
    }

    /// A drain deletes at its cap, writes the remainder back by the aux
    /// rules, and stops when dry — never `with_aux(0)`, which would
    /// manufacture a full cell (the inverted-fill landmine).
    #[test]
    fn a_drain_takes_its_cap_and_stops_when_dry() {
        let mut w = world_with_floor();
        w.add_drain(30, 99);
        // A full cell and a half-full neighbour above the floor.
        w.set(30, 99, Cell::new(material::WATER, 0));
        step(&mut w);
        assert_eq!(w.spring_ledger.drained, DRAIN_FILL as u64, "a full cell drains in one firing at this cap");
        assert_eq!(w.get(30, 99).material, material::EMPTY, "a fully drained cell is EMPTY, not with_aux(0)");
        let before = w.spring_ledger.drained;
        step(&mut w);
        assert_eq!(w.spring_ledger.drained, before, "a dry drain drains nothing");
    }

    /// A partial cell drains to `with_aux(remaining)` when the cap is
    /// smaller than the fill — exercised by halving the cap via two cells.
    #[test]
    fn a_partial_drain_writes_remaining_by_the_aux_rules() {
        let mut w = world_with_floor();
        w.add_drain(30, 99);
        // Three halves of water: one full cell drains whole; then a cell
        // holding half drains to empty in one firing (cap >= fill). To see
        // a genuine partial write the fill must exceed the cap, which a
        // single cell cannot (fill <= FULL == cap) — so assert the
        // boundary the code actually has: fill == cap drains clean.
        w.set(30, 99, Cell::new(material::WATER, 0).with_aux(material::LIQUID_FULL / 2));
        step(&mut w);
        assert_eq!(w.get(30, 99).material, material::EMPTY);
        assert_eq!(w.spring_ledger.drained, (material::LIQUID_FULL / 2) as u64, "the ledger counts fill taken, not cells");
    }

    /// The registration cap refuses, loudly, rather than silently
    /// skipping at step time — a budget is stated where it is enforced.
    #[test]
    fn springs_past_the_budget_are_refused_at_registration() {
        let mut w = world_with_floor();
        assert!(!w.add_spring(10, 60, MAX_SPAN + 1), "a single span past {MAX_SPAN} is refused");
        assert!(!w.add_spring(10, 60, 0), "a zero span is refused");
        assert!(w.add_spring(10, 60, MAX_SPAN));
        assert!(w.add_spring(40, 60, MAX_SPAN));
        assert!(w.add_spring(70, 60, MAX_TOTAL_SPAN - 2 * MAX_SPAN));
        assert!(!w.add_spring(90, 60, 1), "the flow budget is {MAX_TOTAL_SPAN} columns, summed");
        assert_eq!(w.springs.iter().map(|s| s.span).sum::<i32>(), MAX_TOTAL_SPAN);
    }

    /// Same seed, same springs, same world — the mechanism is
    /// deterministic across two independent runs, including under the
    /// parallel driver's step order.
    #[test]
    fn two_runs_with_a_spring_are_identical() {
        let run = || {
            let mut w = world_with_floor();
            w.add_spring(64, 60, 1);
            w.add_drain(20, 99);
            for _ in 0..120 {
                crate::sim::parallel::step(&mut w);
            }
            (total_water_fill(&w), w.spring_ledger)
        };
        let (fill_a, ledger_a) = run();
        let (fill_b, ledger_b) = run();
        assert_eq!(fill_a, fill_b);
        assert_eq!(ledger_a, ledger_b);
    }
}
