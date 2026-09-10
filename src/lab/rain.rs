//! **The mister on the lid.** Direction report Arc B item B3, "an auto rain
//! option" — the air simulation sits idle in the lab (the box's sky is
//! pinned clear, `scene`'s own doc: *"Weather pinned clear... what is
//! removed is the thing that wakes every tile every frame"*), so this is not
//! a cloud or a front: it is water arriving from the top of the box at a
//! rate, spread across the width, falling onto the bed through the *same*
//! placement call the `WATER` tool already uses (`Lab::paint_span`'s
//! `Tool::Water` arm) — so a misted cell is exactly as full, and exactly as
//! blocked by standing rock or a grown plant, as one the player painted by
//! hand.
//!
//! **Lights stay on.** Owner ruling, carried from the direction report: the
//! grow lights are the box's whole income and are not part of what this
//! knob turns. Only water changes.
//!
//! # Why OFF, not a rate — the measurement this ships from
//!
//! `examples/soil_drawdown.rs -- scenario=played_bed` censused the owner's
//! own bed (13 plants, a colony landing on the timeline at frame 6,000) with
//! **no watering**, seeds 1 and 2, across the owner's own 120,000-frame
//! session:
//!
//! | seed | frame 0 | frame 6,000 | frame 120,000 | loss, whole session | loss, from frame 6,000 |
//! |---|---|---|---|---|---|
//! | 1 | 24,998,400 | 24,745,334 | 23,568,991 | 5.72% | 4.75% |
//! | 2 | 24,998,400 | 24,667,881 | 23,973,662 | 4.10% | 2.81% |
//!
//! Both readings sit under the ~10% line `CLAUDE.md`'s task set as the
//! trigger for shipping a rate as the default. The positive control
//! (`founders=0`, same soil depth, same 120,000 frames) held **exactly**
//! flat at field capacity the whole run — 24,998,400 at every stop, zero
//! loss — which is what says the drying above is the crop's own
//! transpiration and evaporation from bare, sun-touched soil rather than a
//! leak the census is inventing: `CLAUDE.md`'s standing rule that a number
//! has to move on a case known to move and hold still on a case known not
//! to, checked both ways here in one run each.
//!
//! So the played bed does not dry out over a session by enough to need
//! watering turned on for the player automatically — [`Rain::Off`] ships as
//! the default, and `EVAPORATION_...` below are the two figures this
//! decision rests on, kept as constants rather than only prose so a later
//! re-measurement has something to diff against. The rates below still
//! exist and are not vestigial: a player growing a heavier stand, a smaller
//! compartment, or a longer session than the owner's own 120,000 frames can
//! turn one on, and `LIGHT`/`STEADY`/`HEAVY` is the graded ladder
//! `CLAUDE.md`'s ethos law asks for rather than a single on/off switch.
//!
//! # The mechanism
//!
//! Every sim tick (`Lab::tick`, right after `frame::step` — the same timing
//! contract `scenario::tick_timeline` documents, run at the point the
//! frame's own state is settled), [`tick`] asks whether this rate is *due*
//! this frame the way [`super::scenario::Scenario::due`] asks its own
//! question: `world.frame` a multiple of the rate's interval. When it is,
//! it places [`Rain::drop`]'s cell count along [`super::scene::LabBox::
//! room_top`] — the first row of air under the ceiling, the same row a
//! fixture's beam starts from — spread in even bins across the bed's width
//! and jittered *within* each bin from `world.rng`, the world's own
//! deterministic stream (the same one `World::paint_capsule_as` already
//! draws its shade and its density check from), so two runs of the same
//! seed rain on the same columns in the same order — `CLAUDE.md`:
//! determinism is required, same build.
//!
//! **Never fires while paused.** `TimeControl::plan` sets `ticks` to zero in
//! [`super::time::Phase::Paused`], so `Lab::advance`'s loop never calls
//! `Lab::tick` at all — this needs no paused check of its own, the same way
//! `scenario::tick_timeline` needs none: a function only reachable through a
//! tick that did not run cannot itself run.
//!
//! **The counter is an effect count, not an attempt count.** `CLAUDE.md`'s
//! own rule for a fresh number: *"pair every 'it fired' counter with an
//! effect counter from the far side of the call"*. A drop aimed at a column
//! a grown plant has already reached the ceiling of is refused by
//! `paint_capsule_as`'s solid/plant guard exactly as a player's own brush
//! would be, so [`tick`] checks the cell actually reads back as water before
//! counting it — an attempt count would move even on a frame where every
//! drop bounced off canopy, which is a null pretending to be a positive.

use super::scene::LabBox;
use crate::sim::world::World;

/// **Soil water lost over the owner's own 120,000-frame played-bed session,
/// with no watering** — `examples/soil_drawdown.rs -- scenario=played_bed
/// seed=1 frames=120000 every=30000`. In `material::SOIL_FIELD_CAPACITY`
/// units summed over every soil cell (`update::soil_moisture`), the same
/// total this module's own header table reads from.
///
/// Kept as a constant, not only prose, so a later change to the bed, the
/// plant mix, or the moisture pass can diff a fresh run against a number
/// rather than a table cell. See this file's header for the full readings
/// and the `founders=0` positive control that grounds them.
pub const PLAYED_BED_LOSS_SEED1_FRAME0: (u64, u64) = (24_998_400, 23_568_991);
/// The same reading from `world.frame == 6_000` — the moment the colony
/// timeline lands and the bed becomes "played" rather than "growing" — to
/// `120_000`, which is the window a rate is actually judged against in
/// `apply_settings`'s own doc on `Knob::Bed` fields: the box a player waters
/// is the one already running, not the one mid-build.
pub const PLAYED_BED_LOSS_SEED1_FRAME6000: (u64, u64) = (24_745_334, 23_568_991);
/// Seed 2 of the same pair, `CLAUDE.md`'s own minimum for "not one seed":
/// a smaller loss than seed 1 (4.10% of the session, 2.81% from frame
/// 6,000) but the same story -- neither seed crosses the ~10% line, and
/// the `founders=0` control on this seed is identical to seed 1's (flat at
/// 24,998,400 throughout, so it is not reprinted as its own constant).
pub const PLAYED_BED_LOSS_SEED2_FRAME0: (u64, u64) = (24_998_400, 23_973_662);
pub const PLAYED_BED_LOSS_SEED2_FRAME6000: (u64, u64) = (24_667_881, 23_973_662);

/// How many sim ticks apart a due drop falls, for every rate that has one.
/// One constant rather than a per-rate interval: `CLAUDE.md`'s "when
/// several knobs move the same number, check what each one trades" applies
/// in miniature here too — varying only the *count* per drop, at a fixed
/// cadence, keeps every rate's spread-across-the-width shape identical and
/// makes `cells_per_1000_frames` an exact multiple at every rate (1000 is a
/// multiple of 40), which is what lets the per-setting test below assert a
/// tight tolerance instead of a loose one.
const DROP_INTERVAL_FRAMES: u64 = 40;

/// How hard the mister above the bed is running.
///
/// **Off/Light/Steady/Heavy, not a bool** — `CLAUDE.md`'s ethos law, stated
/// for destruction and applying here exactly as it does there: "an outcome
/// is a distribution, not a binary... does this have a middle?" A rain
/// switch would let the player ask "is it raining" and nothing else; three
/// live rates let them ask "how much", which is the question a mister
/// actually answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rain {
    /// No mister. The shipped default — see this file's header for the
    /// measurement it rests on.
    #[default]
    Off,
    Light,
    Steady,
    Heavy,
}

impl Rain {
    /// Step to the next setting, wrapping — the BOX page's `RAIN` row and
    /// its key both call this, the same shape `time::Reaction` and
    /// `ui::LogFilter` already cycle through.
    pub fn next(self) -> Self {
        match self {
            Rain::Off => Rain::Light,
            Rain::Light => Rain::Steady,
            Rain::Steady => Rain::Heavy,
            Rain::Heavy => Rain::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rain::Off => "OFF",
            Rain::Light => "LIGHT",
            Rain::Steady => "STEADY",
            Rain::Heavy => "HEAVY",
        }
    }

    /// Round-trip through the integer a scenario `Setting` and `LabBox::
    /// save` both write through `params::write_bed`/`read_bed`, the same
    /// pair every other bed field goes through. Order matters here: a
    /// scenario file that already says `rain: 2` (say, hand-edited before a
    /// future rate is inserted between two of these) must keep meaning the
    /// same setting it always did, so a variant is appended, never inserted.
    pub fn from_index(v: u8) -> Self {
        match v {
            1 => Rain::Light,
            2 => Rain::Steady,
            3 => Rain::Heavy,
            _ => Rain::Off,
        }
    }

    pub fn as_index(self) -> u8 {
        match self {
            Rain::Off => 0,
            Rain::Light => 1,
            Rain::Steady => 2,
            Rain::Heavy => 3,
        }
    }

    /// Cells placed per due drop, and the drop's own cadence in ticks.
    /// `None` for `Off`: the one rate with nothing to place, ever.
    fn drop(self) -> Option<(u32, u64)> {
        match self {
            Rain::Off => None,
            Rain::Light => Some((2, DROP_INTERVAL_FRAMES)),
            Rain::Steady => Some((6, DROP_INTERVAL_FRAMES)),
            Rain::Heavy => Some((16, DROP_INTERVAL_FRAMES)),
        }
    }

    /// Cells this rate places per 1,000 sim frames, at the shipped cadence —
    /// what the per-setting test below checks a real run against, and what
    /// the BOX page's hover note quotes so the player is told a real number
    /// rather than a label alone.
    pub fn cells_per_1000_frames(self) -> u32 {
        match self.drop() {
            Some((cells, interval)) => cells * (1000 / interval as u32),
            None => 0,
        }
    }
}

/// Place this tick's rain, if this rate has one due on `world.frame`.
/// Returns how many cells actually became water — the effect count this
/// file's header explains, not an attempt count.
///
/// **Nothing here is called while the box is paused.** See this file's own
/// header: `Lab::tick` is the only caller, and `Lab::advance` never invokes
/// it at all under `Phase::Paused`.
pub fn tick(world: &mut World, spec: &LabBox, rate: Rain) -> u32 {
    let Some((cells, interval)) = rate.drop() else { return 0 };
    if interval == 0 || !world.frame.is_multiple_of(interval) {
        return 0;
    }
    let Some(water) = world.materials.id_of("water") else { return 0 };
    let y = spec.room_top();
    let width = spec.width.max(1);
    // Even bins across the width, jittered inside each bin from the world's
    // own `Rng` -- the same stream `paint_capsule_as` already draws its
    // shade from, so this is one more deterministic reader of it rather
    // than a second source of randomness the replay would have to track.
    let bin = (width / cells.max(1) as i32).max(1);
    let mut placed = 0u32;
    for i in 0..cells {
        let lo = i as i32 * bin;
        // The last bin absorbs the remainder so a width that does not
        // divide evenly still covers every column, not just the cells*bin
        // that fit -- a strip along the right wall would otherwise never
        // see rain at all.
        let hi = if i + 1 == cells { width - 1 } else { (lo + bin - 1).min(width - 1) };
        let span = (hi - lo + 1).max(1);
        let x = lo + world.rng.below(span as u32) as i32;
        // A single-cell capsule (radius 0) -- one drop, one cell -- through
        // the *same* call `Lab::paint_span`'s `Tool::Water` arm makes:
        // `aux` stays 0, which `update::liquid_fill` reads back as full, and
        // a column already holding rock or a grown plant refuses it exactly
        // as it would refuse the player's own brush.
        world.paint_capsule_as((x, y), (x, y), 0, water, 1.0);
        if world.get(x, y).material == water {
            placed += 1;
        }
    }
    world.rain_cells += placed as u64;
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare, unplanted bed -- nothing standing between a drop and the
    /// soil, so every drop this file's tests place should land as water.
    /// `founders: 0, colonies: 0` rather than `LabBox::default()` alone:
    /// the default bed stocks a colony, and an ant wandering the ceiling
    /// row on the frame a drop lands is exactly the confound these tests
    /// have no business tripping over.
    fn bare_bed() -> (World, LabBox) {
        let spec = LabBox { founders: 0, colonies: 0, ..LabBox::default() };
        let world = spec.build();
        (world, spec)
    }

    /// **Sensitivity, per setting: does the rate actually move, and by
    /// about the right amount?** Run each live rate over a clean
    /// 1,000-frame window and compare the effect count against `Rain::
    /// cells_per_1000_frames`'s own claim for it -- exact here, since 1,000
    /// is a multiple of `DROP_INTERVAL_FRAMES` (40), so every rate's due
    /// frames land on the identical 25-frame schedule. The tolerance is
    /// still a tolerance, not an equality assertion, because the claim this
    /// test makes is "the rate", not "this exact cadence forever".
    #[test]
    fn each_rate_places_close_to_its_own_cells_per_1000_frames() {
        for rate in [Rain::Light, Rain::Steady, Rain::Heavy] {
            let (mut world, spec) = bare_bed();
            let mut placed = 0u32;
            for f in 0..1000u64 {
                world.frame = f;
                placed += tick(&mut world, &spec, rate);
            }
            let want = rate.cells_per_1000_frames();
            let tolerance = (want / 10).max(2);
            assert!(
                placed.abs_diff(want) <= tolerance,
                "{rate:?}: placed {placed} cells in 1,000 frames, want {want} +/- {tolerance}"
            );
            assert_eq!(world.rain_cells, placed as u64, "the world's own counter must agree with the return value");
        }
    }

    /// **Specificity: `Off` places nothing, ever.** The one rate with no
    /// entry in `Rain::drop`'s match, checked over a window four times
    /// `each_rate...`'s own so a rate that was accidentally wired to fire
    /// only every few hundred frames would not slip past a shorter one.
    #[test]
    fn off_places_zero() {
        let (mut world, spec) = bare_bed();
        let mut placed = 0u32;
        for f in 0..4000u64 {
            world.frame = f;
            placed += tick(&mut world, &spec, Rain::Off);
        }
        assert_eq!(placed, 0);
        assert_eq!(world.rain_cells, 0);
    }

    /// **A paused box places zero.** Not a property of `tick` itself --
    /// this file's header explains why it needs no paused check of its own
    /// -- but of `Lab::advance`/`TimeControl::plan`, which is the actual
    /// path a player's own pause button goes through. Armed at `Heavy`, the
    /// rate most likely to show a leak, and advanced through real frames
    /// rather than a hand-set `world.frame` so this exercises the same
    /// `Phase::Paused` gate the game does.
    #[test]
    fn a_paused_box_places_zero() {
        let spec = LabBox { founders: 0, colonies: 0, rain: Rain::Heavy, ..LabBox::default() };
        let mut lab = crate::lab::Lab::new(spec);
        lab.time.phase = crate::lab::time::Phase::Paused;
        for _ in 0..500 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert_eq!(lab.world.rain_cells, 0, "a paused box must place no rain");
    }

    /// **The setting round-trips through a scenario `Setting`** -- the
    /// route `hunting_ground.ron`'s `(subject: "ant", field: "sight_
    /// range", ...)` already takes for a creature field, exercised here for
    /// `Knob::Bed`'s `"rain"` arm. `params::box_rows`'s own `rain` row is
    /// what makes this resolve at all: without it `resolve_setting` refuses
    /// with "no parameter named the bed.rain" and this test is the guard
    /// that would catch that row being removed or renamed.
    #[test]
    fn rain_round_trips_through_a_scenario_setting() {
        use super::super::scenario::{apply_settings, Setting};
        let spec0 = LabBox { founders: 0, colonies: 0, ..LabBox::default() };
        let mut world = spec0.build();
        let mut spec = spec0.clone();
        assert_eq!(spec.rain, Rain::Off, "the bed this test builds must start at the shipped default");
        let settings = vec![Setting { subject: "the bed".to_string(), field: "rain".to_string(), value: 2.0 }];
        let applied = apply_settings(&mut world, &mut spec, &settings).expect("rain is a registered bed field");
        assert_eq!(applied, 1);
        assert_eq!(spec.rain, Rain::Steady);
    }

    /// `Rain::from_index`/`as_index` is its own round trip, checked directly
    /// rather than only through the two heavier paths above -- the same
    /// reasoning `write_bed`/`read_bed`'s own doc gives for existing as a
    /// pair: a sweep or a saved scenario reads a value back through this
    /// exact table, so this is the one place a future inserted (rather than
    /// appended) variant would be caught.
    #[test]
    fn rain_index_round_trips_for_every_variant() {
        for rate in [Rain::Off, Rain::Light, Rain::Steady, Rain::Heavy] {
            assert_eq!(Rain::from_index(rate.as_index()), rate);
        }
        // Out of range refuses to `Off` rather than panicking -- the same
        // forgiving-clamp shape every other integral `write_bed` arm gets
        // from `value.max(0.0).round()`.
        assert_eq!(Rain::from_index(200), Rain::Off);
    }
}
