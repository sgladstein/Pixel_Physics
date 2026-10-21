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
//! # OFF vents the lid (2026-10-03)
//!
//! **The table below was measured with a sealed lid** and is the box before
//! this date. With the lid sealed, OFF barely dried anything: the plants'
//! breath came back as condensation (`weather::condense_under_a_lid`). The
//! owner asked for a drought a player can cause, by the mister dial alone,
//! so `Off` now sets `World::lid_vented` and that water stays out until the
//! mister comes back on (`PIXEL_PHYSICS_LID_VENT=off` restores the sealed
//! box). Over 12 paired played-bed seeds, OFF from frame 60,000 to 180,000:
//! standing water 127 -> 7 cells and plants 193 -> 121 at 180,000, lower on
//! 12 of 12; plant cells back within 2% by 240,000. README `Sap flow status`
//! has the rest.
//!
//! # Why LIGHT, not OFF — the measurement this ships from
//!
//! **This is not the bed `Off` was originally measured against.** The
//! played bed (`assets/lab_scenarios/played_bed.ron`) has since grown a
//! low fruiting thicket and a tree on top of the thirteen-plant mix the
//! first reading used — both owner rulings on review cards, landed by the
//! played-bed-mix branch — and a tree alone shades the bench under it to
//! under 1% of lamp light (that file's own header). So this lane re-asked
//! the drawdown question on the bed as it now ships rather than inheriting
//! the old reading, per the coordinator's own instruction that every number
//! here is quoted fresh from now on.
//!
//! `examples/soil_drawdown.rs -- scenario=played_bed rain=<0|1|2> frames=
//! 120000 every=6000` (`rain=` is this lane's own addition to the harness,
//! overriding the scenario's own unset field so one file can be measured at
//! every rate) censused the new bed at OFF, LIGHT and STEADY, seeds 1 and 2,
//! across the owner's own 120,000-frame session — soil water and, new this
//! round, standing water sitting above the soil line (pooling on the bed,
//! the failure a player would actually see and the number `labshot` prints
//! nothing for):
//!
//! | rate | seed | frame 6,000 | frame 120,000 | soil water vs frame 6,000 | standing water @ 120,000 |
//! |---|---|---|---|---|---|
//! | OFF | 1 | 24,713,424 | 20,641,918 | **-16.47%** (-15.51% already by frame 60,000) | 2,946 |
//! | OFF | 2 | 24,691,699 | 23,645,535 | -4.24% | 132 |
//! | LIGHT | 1 | 24,947,611 | 26,764,991 | +7.28% | 2,363 |
//! | LIGHT | 2 | 24,815,760 | 26,323,289 | +6.08% | 3,000 |
//! | STEADY | 1 | 25,301,374 | 30,059,066 | **+18.80%** | **11,090** |
//! | STEADY | 2 | 25,093,345 | 28,839,261 | **+14.93%** | **12,213** |
//!
//! **OFF is not safe any more.** Seed 1 alone breaks the ~10% line
//! `CLAUDE.md`'s task set as the trigger for shipping a rate as the default,
//! and breaks it early — already past it at frame 60,000, not a borderline
//! reading rounded down at the finish. The gap between the two seeds
//! (-16.47% against -4.24%) is the "outcomes here have enormous spread"
//! warning `CLAUDE.md`'s method section gives for trusting a single seed,
//! and it argues the same direction a wider sweep would: a tree that shades
//! the bench to under 1% of lamp light changes the water story enough that
//! OFF can no longer be trusted to hold this bed.
//!
//! **LIGHT holds.** +7.28%/+6.08% at the worst (last) stop, comfortably
//! inside the ±10% band and on the opposite side from OFF's miss — every
//! earlier stop sits closer to flat still (the per-setting test below
//! checks the rate itself over a clean window). Its standing-water reading
//! (2,363–3,000 cells at the final stop) is the *same order of magnitude*
//! as OFF's own reading on the identical bed (132–2,946), not a new failure
//! LIGHT introduces: it is `weather::condense_under_a_lid`, the box's
//! existing dew behaviour, confirmed independently by `examples/labgif.rs`'s
//! own header (an `Off` control there still reads condensation with the
//! mister's own counter flat at zero the whole time). The positive control
//! — `founders=0`, the same 80-row soil depth, the same 120,000 frames,
//! nothing alive at all — holds **exactly** flat at field capacity with
//! **zero** standing water at every stop, which says the condensation above
//! needs a transpiring plant to trigger at all and is not this census
//! inventing a leak: `CLAUDE.md`'s rule that a number has to move on a case
//! known to move and hold still on a case known not to, checked both ways
//! in one run each.
//!
//! **STEADY overshoots badly on both readings, not just one:** soil water
//! at +14.93%/+18.80%, well past the line, and standing water climbing
//! without plateauing through the whole run to 11,090–12,213 cells at the
//! final stop — four to thirty-five times OFF's own peak on the identical
//! bed, exactly the pooling failure the ±10% line and the surface census
//! exist to catch. HEAVY (400 cells/1,000 frames against STEADY's 150) was
//! not measured: this lane's cost fork is "HEAVY only if STEADY is still
//! under the bar", and STEADY is not under it — it is already well over.
//!
//! So [`Rain::Light`] ships as the default — the lowest setting that
//! actually holds this bed within its own tolerance — `PLAYED_BED_*` below
//! are the constants this decision rests on, kept rather than only prose so
//! a later re-measurement has something to diff against, and [`Rain::Off`]
//! stays one keypress away (`8`, or the BOX page's own `RAIN` row): the
//! owner's own ruling on the rain card, *"you can ship on, but give me
//! control over it."* `STEADY`/`HEAVY` remain the graded ladder above the
//! default, for a heavier stand, a smaller compartment, or a longer session
//! than the owner's own 120,000 frames — `CLAUDE.md`'s ethos law is still
//! why this is four rates and not one on/off switch.
//!
//! # A drain in the floor, and wet and dry spells (2026-10-20)
//!
//! **The table above stops at 120,000 frames, and the box floods after
//! that.** Open bug Z38: on the plant community bed, standing water under
//! the default mister climbed to 6,864-9,436 cells by 450,000 frames and
//! never levelled, and the soil filled from the floor up to 0.95 of
//! saturation. Not a tuning problem: the box conserves water (a vented lid
//! banks what it lets out, `World::atmospheric_bank`), and the mister makes
//! water from nothing, so a box with any steady rain and no sink can only
//! fill. Two bed settings (M1 step 2 of
//! `Reports/plant-community-plan-2026-10-20.md`):
//!
//! - **`LabBox::drain`, on by default** -- the bottom row of soil gives up
//!   whatever it holds above field capacity, the way a pot drains through
//!   its hole. Soil drier than that is never touched. See [`drain`].
//! - **`LabBox::spells`, off by default** -- the mister on a timer: `wet`
//!   frames at the box's rate, then `dry` frames off with the lid vented.
//!   See [`Spells`]. Off in the played box because the owner's ruling of
//!   2026-10-03 makes a drought the player's to cause (*"mister dial
//!   only"*); the plant training world turns it on.
//!
//! Measured on the four-role community bed (`examples/plant_m0.rs`,
//! herb/grass/shrub/creeper, load failure off, 4 seeds, 480,000 frames;
//! spells 20,000 wet at `Steady` and 40,000 dry, the same mean rain as
//! `Light`):
//!
//! | at 480,000 frames | `Light`, no drain | `Light` + drain | drain + spells |
//! |---|---|---|---|
//! | standing water, cells, per seed | 7,187-8,692, rising all run | 900-1,650, level from ~240,000 | 311-1,121 after a dry spell, 1,670-2,067 after a wet one |
//! | soil water, top band -> floor | 0.95 -> 0.97 | 0.63 -> 0.62 | 0.62-0.64 -> 0.62 |
//! | water around shallow (grass) roots, after dry / wet | 0.77 / 0.78 | 0.58 / 0.59 | **0.52 / 0.61** |
//! | ... around deep (shrub) roots | 0.83 / 0.82 | 0.81 / 0.81 | 0.80 / 0.81 |
//!
//! The drain ends the lake; what still stands is caught on grass mats,
//! leaves and seed mounds, and over root masses the soil water cannot pass
//! through (it moves only between soil cells). The spells are what make a
//! shallow root and a deep one live through different weather. **Frame
//! cost of the drain, paired** from one grown bed on one thread, 20,000
//! ticks each way, three beds: 1,743 -> 1,731, 1,546 -> 1,592 and 2,157 ->
//! 2,122 us/tick, inside +/-3% both ways, while the soil-water phase walks
//! 18% more cells (`plant_m0 mode=draincost`).
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
//! and jittered *within* each bin from a stream keyed on the world's seed
//! and the frame (`rng::stream`, lent to `World::paint_capsule_as` for the
//! call so the drop's shade comes from it too), so two runs of the same
//! seed rain on the same columns in the same order — `CLAUDE.md`:
//! determinism is required, same build. **Not `World::rng`**, which it
//! used until 2026-10-20: that stream is shared, so what else drew from it
//! moved every drop, and every drop moved what drew after it (M1 A1; see
//! [`tick`]).
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

/// **Soil water on the owner's own 120,000-frame played-bed session, with
/// no watering** (`Rain::Off`) — `examples/soil_drawdown.rs -- scenario=
/// played_bed seed=1 rain=0 frames=120000 every=6000`, re-measured on the
/// bed as it now ships (thicket and tree included). In `material::
/// SOIL_FIELD_CAPACITY` units summed over every soil cell (`update::
/// soil_moisture`), the same total this module's own header table reads
/// from.
///
/// Kept as a constant, not only prose, so a later change to the bed, the
/// plant mix, or the moisture pass can diff a fresh run against a number
/// rather than a table cell. See this file's header for the full readings,
/// the standing-water-on-the-surface census that sits beside them, and the
/// `founders=0` positive control that grounds them.
pub const PLAYED_BED_LOSS_SEED1_FRAME0: (u64, u64) = (24_998_400, 20_641_918);
/// The same reading from `world.frame == 6_000` — the moment the colony
/// timeline lands and the bed becomes "played" rather than "growing" — to
/// `120_000`, which is the window a rate is actually judged against in
/// `apply_settings`'s own doc on `Knob::Bed` fields: the box a player waters
/// is the one already running, not the one mid-build. This is the reading
/// that fails: -16.47%, already past the ~10% line by frame 60,000.
pub const PLAYED_BED_LOSS_SEED1_FRAME6000: (u64, u64) = (24_713_424, 20_641_918);
/// Seed 2 of the same pair, `CLAUDE.md`'s own minimum for "not one seed":
/// a smaller loss than seed 1 (-5.41% of the session, -4.24% from frame
/// 6,000) that stays under the ~10% line where seed 1 does not — the spread
/// between the two, not either reading alone, is what makes `Off` unsafe as
/// a default now. The `founders=0` control on this seed is identical to
/// seed 1's (flat at 24,998,400 throughout, zero standing water, so it is
/// not reprinted as its own constant).
pub const PLAYED_BED_LOSS_SEED2_FRAME0: (u64, u64) = (24_998_400, 23_645_535);
pub const PLAYED_BED_LOSS_SEED2_FRAME6000: (u64, u64) = (24_691_699, 23_645_535);

/// **The shipped default's own reading, same bed, same window, `Rain::
/// Light` armed throughout** (`rain=1`) — what [`Rain::default`] rests on.
/// Overshoots on the *safe* side, +7.28%: still comfortably inside the
/// ±10% band, unlike `Off`'s seed 1 above.
pub const PLAYED_BED_LIGHT_SEED1_FRAME6000: (u64, u64) = (24_947_611, 26_764_991);
/// Seed 2 of the default's own reading: +6.08%, the same story.
pub const PLAYED_BED_LIGHT_SEED2_FRAME6000: (u64, u64) = (24_815_760, 26_323_289);
/// **The rejected neighbour, one rate up** (`rain=2`, `Rain::Steady`) — kept
/// so a later session does not have to re-run it to see why `Light` was
/// chosen over it: +18.80%, well past the line. Standing water at the same
/// stop (see this file's header) climbs to 11,090 cells, against `Light`'s
/// 2,363 on the identical seed.
pub const PLAYED_BED_STEADY_SEED1_FRAME6000: (u64, u64) = (25_301_374, 30_059_066);
/// Seed 2 of the rejected neighbour: +14.93%, standing water 12,213 cells.
pub const PLAYED_BED_STEADY_SEED2_FRAME6000: (u64, u64) = (25_093_345, 28_839_261);

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
    /// No mister. One keypress (`8`) or one click away from the shipped
    /// default — the owner's own ruling on the rain card, *"you can ship
    /// on, but give me control over it"*. See this file's header for the
    /// measurement that keeps this from being the default.
    Off,
    /// **The shipped default** — see this file's header for the
    /// measurement it rests on.
    #[default]
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
/// `PIXEL_PHYSICS_LID_VENT=off` keeps the lid sealed with the mister off --
/// the box before 2026-10-03, which barely dried. The control arm for
/// `World::lid_vented`.
pub fn lid_vent_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PIXEL_PHYSICS_LID_VENT").map_or(true, |v| v != "off" && v != "0"))
}

pub fn tick(world: &mut World, spec: &LabBox, rate: Rain) -> u32 {
    drain(world, spec);
    // **A dry spell is the mister off**, lid vented and all: the same
    // drought a player makes by turning the dial, on a timer. See
    // [`Spells`].
    let rate = match spec.spells {
        Some(spells) if !spells.wet_at(world.frame) => Rain::Off,
        _ => rate,
    };
    // **Off vents the lid** (`World::lid_vented`): with the mister off the
    // box loses what its plants breathe out and dries, which is the whole
    // reason a player turns it off. Before the early return, so `Off` --
    // which places nothing -- still says so every frame.
    world.lid_vented = rate == Rain::Off && lid_vent_enabled();
    let Some((cells, interval)) = rate.drop() else { return 0 };
    if interval == 0 || !world.frame.is_multiple_of(interval) {
        return 0;
    }
    let Some(water) = world.materials.id_of("water") else { return 0 };
    let y = spec.room_top();
    let width = spec.width.max(1);
    // Even bins across the width, jittered inside each bin.
    //
    // **The mister rolls its own dice, keyed on the frame, and lends them to
    // the brush for the duration.** It used to jitter from `World::rng`
    // directly, reasoning that the stream `paint_capsule_as` draws its shade
    // from was one more deterministic reader rather than a new source. It
    // is deterministic, and that was never the problem: the shade is drawn
    // only for a drop that lands (a column holding a grown plant refuses it
    // and draws nothing), so where the plants stood decided how far the
    // stream advanced, and everything that draws after it -- every litter
    // and pip cell's rot roll in `decay::tick` -- re-rolled with it. And one
    // extra draw anywhere moved every later drop. M1 A1,
    // `Reports/plant-community-plan-2026-10-20.md`; `plant_m0 mode=nudge` is
    // the measurement. Swapped back below, so nothing else sees the swap.
    let shared = std::mem::replace(&mut world.rng, crate::sim::rng::stream(world.seed ^ RAIN_SALT, world.frame, 0, 0));
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
    world.rng = shared;
    world.rain_cells += placed as u64;
    placed
}

/// **Wet and dry spells: the mister on a timer** (M1 step 2,
/// `Reports/plant-community-plan-2026-10-20.md` §8.1). `wet` frames at the
/// box's own rain setting, then `dry` frames with the mister off and the
/// lid vented, the drought `Rain::Off` already is; then again.
///
/// **Why the box needs weather at all.** A steady mister gives a plant
/// nothing to answer: the soil sits at one wetness for ever, the same under
/// every plant and at every age. Real ground is wetted by pulses and dries
/// between them, from the top down, so a shallow root has water after rain
/// and a deep one has it in a drought -- one of the oldest ways plants that
/// want the same things manage to share a bed (plan §4). Regular, not
/// random, so a census can be taken at the same phase of the cycle on
/// every seed (`CLAUDE.md`: a designed oscillator must be divided out of
/// every number it reaches).
///
/// `None` (the default) is the steady mister, as before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Spells {
    pub wet: u64,
    pub dry: u64,
}

impl Spells {
    /// Is `frame` inside a wet spell? A cycle starts wet. A zero-length
    /// cycle is always wet, which is the steady mister.
    pub fn wet_at(self, frame: u64) -> bool {
        let period = self.wet + self.dry;
        period == 0 || frame % period < self.wet
    }
}

/// How often the floor drain runs. The drain only takes what soil holds
/// above field capacity, so its rate is set by how fast that excess sinks
/// to the bottom row, not by this; 20 frames keeps it out of the frame cost.
const DRAIN_INTERVAL_FRAMES: u64 = 20;

/// **The bed drains through its floor, the way a pot drains through its
/// hole** (`LabBox::drain`; M1 step 2, open bug Z38).
///
/// The box conserves water -- even a vented lid only banks what it lets out
/// (`World::atmospheric_bank`) -- and the mister makes water from nothing.
/// So the box had a source and no sink, and any steady rain had to end
/// under a lake: measured, the default mister's standing water climbed to
/// 6,864-9,436 cells by 450,000 frames and never levelled, and the soil
/// profile filled from the floor up to 0.95 of saturation. Real ground
/// does not do that because water past what soil can hold against gravity
/// (field capacity) keeps sinking and leaves.
///
/// So the bottom row of soil, the one lying on the stone floor, gives up
/// whatever it holds above field capacity, every `DRAIN_INTERVAL_FRAMES`.
/// Soil already drier than that is untouched: the drain can only take
/// water that would have run out of a real pot, never dry a bed out.
/// Counted in `World::drained_water`, the far side of `rain_cells`.
fn drain(world: &mut World, spec: &LabBox) {
    if !spec.drain || !world.frame.is_multiple_of(DRAIN_INTERVAL_FRAMES) {
        return;
    }
    let y = spec.ground_y + spec.soil_depth - 1;
    for x in 0..spec.width {
        let cell = world.get(x, y);
        let material = world.materials.get(cell.material);
        if material.kind != crate::sim::material::MaterialKind::Powder || material.water_capacity == 0 {
            continue;
        }
        let held = cell.aux();
        let keep = crate::sim::material::SOIL_FIELD_CAPACITY.min(material.water_capacity);
        if held <= keep {
            continue;
        }
        world.drained_water += (held - keep) as u64;
        world.set_soil_moisture(x, y, cell.with_aux(keep));
    }
}

/// Separates the mister's per-frame dice from every other `rng::stream`
/// caller. See `tick`.
const RAIN_SALT: u64 = 0x5261_696E_4D69_7374; // "RainMist"

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

    /// **The drain takes only what soil holds past field capacity, and only
    /// with the drain on** (`LabBox::drain`, M1 step 2). One bottom-row cell
    /// saturated and one drier than field capacity: on, the first comes back
    /// to field capacity and the excess is counted, the second is untouched
    /// -- a drain must never dry a bed out. Off, nothing moves at all.
    #[test]
    fn the_drain_takes_only_the_excess_and_only_when_on() {
        use crate::sim::material::{SOIL_FIELD_CAPACITY, SOIL_SATURATED};
        for on in [false, true] {
            let (mut world, mut spec) = bare_bed();
            spec.drain = on;
            let floor = spec.ground_y + spec.soil_depth - 1;
            let (wet_x, dry_x) = (spec.width / 2, spec.width / 2 + 10);
            let soil = world.materials.id_of("soil").expect("soil ships");
            for x in [wet_x, dry_x] {
                assert_eq!(world.get(x, floor).material, soil, "test setup: the bottom row should be soil at x {x}");
            }
            world.set_soil_moisture(wet_x, floor, world.get(wet_x, floor).with_aux(SOIL_SATURATED));
            world.set_soil_moisture(dry_x, floor, world.get(dry_x, floor).with_aux(300));
            world.frame = DRAIN_INTERVAL_FRAMES;
            tick(&mut world, &spec, Rain::Off);
            let (wet_after, dry_after) = (world.get(wet_x, floor).aux(), world.get(dry_x, floor).aux());
            if on {
                assert_eq!(wet_after, SOIL_FIELD_CAPACITY, "saturated floor soil should drain to field capacity");
                assert_eq!(world.drained_water, (SOIL_SATURATED - SOIL_FIELD_CAPACITY) as u64, "the counter must be what left");
            } else {
                assert_eq!(wet_after, SOIL_SATURATED, "with the drain off the floor must keep its water");
                assert_eq!(world.drained_water, 0);
            }
            assert_eq!(dry_after, 300, "soil drier than field capacity is never the drain's to take (drain {on})");
        }
    }

    /// **A dry spell is the mister off with the lid vented; a wet one is the
    /// box's own rate** (`Spells`, M1 step 2). Two 1,000-frame spells at
    /// `Steady`: rain in the first, none in the second and the lid open for
    /// all of it -- the drought `Rain::Off` is, on a timer.
    #[test]
    fn a_dry_spell_turns_the_mister_off_and_vents_the_lid() {
        let (mut world, mut spec) = bare_bed();
        spec.spells = Some(Spells { wet: 1_000, dry: 1_000 });
        let (mut wet, mut dry) = (0u32, 0u32);
        for f in 0..2_000u64 {
            world.frame = f;
            let placed = tick(&mut world, &spec, Rain::Steady);
            if f < 1_000 {
                wet += placed;
                assert!(!world.lid_vented, "the lid must be sealed in a wet spell (frame {f})");
            } else {
                dry += placed;
                assert!(world.lid_vented || !lid_vent_enabled(), "a dry spell must vent the lid (frame {f})");
            }
        }
        assert!(wet.abs_diff(Rain::Steady.cells_per_1000_frames()) <= 15, "the wet spell rained {wet}, not Steady's rate");
        assert_eq!(dry, 0, "the dry spell rained");
        assert!(Spells { wet: 0, dry: 0 }.wet_at(12_345), "a zero-length cycle is the steady mister");
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

    /// **The shipped default itself, named as `Rain::default()` rather than
    /// hand-typed** -- so a future move of the `#[default]` attribute to a
    /// rate whose own cadence was never re-measured is caught here rather
    /// than silently inheriting whatever `each_rate_...`'s own loop happens
    /// to cover. Asserts two things the header's table above rests on: the
    /// default actually waters (it is not `Off` wearing a different name),
    /// and it places close to its own `cells_per_1000_frames` claim over a
    /// clean window -- the same tolerance `each_rate_...` uses, for the same
    /// reason (1,000 is a multiple of `DROP_INTERVAL_FRAMES`).
    #[test]
    fn the_shipped_default_drawdown_matches_its_own_rate_over_a_short_window() {
        let rate = Rain::default();
        assert_ne!(rate, Rain::Off, "the shipped default must actually water the bed -- Off is a keypress away, not the default");
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
            "default rate {rate:?}: placed {placed} cells in 1,000 frames, want {want} +/- {tolerance}"
        );
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
    /// **Off vents the lid; any other setting seals it** --
    /// `World::lid_vented`, the owner's "mister dial only" answer to "should
    /// the box be able to dry out". Read on the far side of the call: the
    /// bank's excess is what condensation spends, so a vented lid leaves it
    /// where it is and a sealed one draws it down. Both arms in one test, and
    /// the sealed arm is the positive control -- if condensation stopped
    /// firing for some other reason, the vented arm would pass for nothing.
    #[test]
    fn off_vents_the_lid_and_a_live_setting_seals_it() {
        let banked = crate::sim::weather::STORM_RESERVE + 2_000.0;
        let run = |rate: Rain| {
            let (mut world, spec) = bare_bed();
            world.atmospheric_bank = banked;
            for _ in 0..2_000 {
                tick(&mut world, &spec, rate);
                crate::sim::weather::step(&mut world);
                world.frame += 1;
            }
            (world.lid_vented, world.atmospheric_bank)
        };
        let (vented_off, bank_off) = run(Rain::Off);
        let (vented_light, bank_light) = run(Rain::Light);
        assert!(vented_off && !vented_light, "Off must vent and Light must seal: {vented_off} / {vented_light}");
        assert!(bank_light < banked - 100.0, "a sealed lid should give banked water back: {banked} -> {bank_light}");
        assert!(bank_off >= banked, "a vented lid gave {} cells of water back", banked - bank_off);
    }

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
        assert_eq!(spec.rain, Rain::default(), "the bed this test builds must start at the shipped default");
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
