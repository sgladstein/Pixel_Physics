//! **The held world — the third game on this engine.**
//!
//! `cargo run --release --bin druid`. Beside the outdoor sandbox
//! (`src/main.rs`) and the evolution lab (`src/bin/lab.rs`), sharing
//! `sim::frame::step` and the whole `sim` library. Design of record:
//! `Reports/held-world-game-concept-2026-09-13.md`.
//!
//! **The premise.** The land is *held* — nothing grows, breeds, ages, rots or
//! weathers, and the sky does not turn. Physics is untouched: rock falls,
//! water flows. The player carries the only time there is, in a small circle
//! that follows him, and (later) places standing ones he pays for.
//!
//! **Why this is a binary and not a mode.** Owner's ruling, 2026-09-13: *"I
//! don't want to build this into the existing gnome game. This is a fully new
//! and separate game."* Nothing here reaches into `app.rs`, and the engine
//! pieces it stands on (`World::held`, `::quickenings`, `::carried`) all
//! default off, so the sandbox and the lab are byte-identical without it.
//!
//! **What this module is not.** There is no save/load anywhere in this engine
//! (no serde over `World`), no pause-and-resume of a session, and no
//! world-edge behaviour beyond the sandbox's. Named here as out of scope
//! rather than discovered later.

pub mod founding;
pub mod hud;
pub mod menu;

use crate::sim::cell::OrganismId;
use crate::render::Renderer;
use crate::sim::chunk::Rect;
use crate::sim::clock::SkyPin;
use crate::sim::creature;
use crate::sim::explosion::{self, Blasts};
use crate::sim::frame;
use crate::sim::material;
use crate::sim::organism;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::{World, CARRIED_RADIUS};
use crate::worldgen::{self, WorldgenPresets};

/// **Five screens wide, three deep.** The viewport is 512x320
/// (`app::WIDTH`/`HEIGHT`), so this is a walk of about five screens with room
/// above and below.
///
/// **Deliberately not the sandbox's 8192x2560**, which exists so you can
/// tunnel into eight screens of rock. This game has no mining, no caving and
/// no destruction to spend that on, and the depth is where the generation
/// cost lives — `stone_massif` is half of it, and it is half because of
/// volume rather than because of the pass.
///
/// The soil bed matters more here than the rock under it: ants burrow and
/// roots go deep, so soil depth is the resource. That shape is not yet
/// expressed in the preset — the size is the only lever taken so far.
pub const WORLD_WIDTH: u32 = 2560;
pub const WORLD_HEIGHT: u32 = 960;

/// **How long the world lives before it stops.**
///
/// The land was alive and *then* stopped, which is the fiction and also the
/// only way the world contains anything: `life_scatter` places a single
/// `seed`-material powder cell per plant, so a world held at frame 0 is bare
/// ground plus a scatter of seed pixels for ever. **Measured 2026-09-13**,
/// `scene=worldgen` unheld at 512x320, living plant tissue:
///
/// ```text
/// frame  0     2k     4k     6k      8k      10k     12k     14k
/// cells  20    3,252  9,084  14,474  19,244  20,247  18,469  16,535
/// ```
///
/// So the stand peaks near 10,000 and then **self-thins** — past that you are
/// freezing a wood already in decline. 8,000 sits just under the peak, on the
/// rising side, which is a young mature wood rather than an old one.
///
/// **Read it as the age of the world at the moment it stopped**, not as a
/// technical budget: a lower number is a younger, thinner land and a higher
/// one is a tired one. That is a knob worth playing with, which is why it
/// takes an override.
pub const GROW_FRAMES: u64 = 8_000;

/// `PIXEL_PHYSICS_DRUID_SIZE=WxH` — generate a different world size.
///
/// Iterating on a 512x320 world and looking at a 2560x960 one are both worth
/// doing, and a knob is what keeps the two measurements comparable. A knob
/// nobody can see the value of is a knob nobody can tell is disconnected, so
/// `Druid::new` prints what it used.
const SIZE_ENV: &str = "PIXEL_PHYSICS_DRUID_SIZE";
/// `PIXEL_PHYSICS_DRUID_GROW=N` — override [`GROW_FRAMES`].
const GROW_ENV: &str = "PIXEL_PHYSICS_DRUID_GROW";

/// **What the land is when you arrive.**
///
/// Owner's ruling, 2026-09-13: *"I actually want to start with a dead world.
/// No living plants, but I can plant seeds."* That reverses this module's
/// first answer, which grew a wood and held it alive, and it is the better
/// game: a living wood you did not plant is scenery, and the verb the concept
/// is built around is **putting something back**.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Start {
    /// **The default.** The land lived, and then it died: grown for
    /// [`GROW_FRAMES`], then every plant marked senescent, then held.
    ///
    /// **Senescent rather than deleted, and that is the whole trick.**
    /// `World::mark_organism_senescent` is the engine's own kill path — it
    /// sets the flag `plant::rot_remains` reads, and rot then carries the
    /// body out at the species' own half-life. In a *held* world rot never
    /// runs, so the bodies **stand**: a wood of dead trees, exactly the
    /// "somewhere that died" the concept asks for, with no deadwood pass to
    /// write. And the moment a quickening covers one it starts to rot, which
    /// is the same rule the colony and the seed bank already obey and needs
    /// no code of its own.
    ///
    /// It also leaves the **seed bank** the grown phase produced, stopped in
    /// the soil. Those are not plants; they are what germinates the first
    /// time you spend time on that ground.
    Dead,
    /// Never grown at all, and since 2026-09-14 **nothing is scattered into
    /// it either** — bare generated ground, held at frame 0, with not one
    /// plant or seed cell anywhere in it.
    ///
    /// This used to read "plus `life_scatter`'s single seed cell per plant".
    /// The owner's playtest took that out: *"The world should not start with
    /// any seeds. The druid has her own seeds to plant and that populates the
    /// world."* The change is three zeroed densities in the `druid` preset in
    /// `assets/worldgen.ron` — see the note there — not code, because
    /// `life_scatter` already early-outs on them.
    ///
    /// **The default, on the owner's second telling of it.** He asked for
    /// *"a dead world, no living plants, but I can plant seeds"* and got
    /// [`Start::Dead`], which is grown-then-senescent — every plant standing
    /// where it died. Playing it, the verdict was *"still shipping full of
    /// plants... I thought we said bare"*, and he is right about what he
    /// sees: a senescent tree still renders as a tree, so a wood that is
    /// dead by every number in the simulation reads on screen as a wood.
    ///
    /// **That is worth keeping as a finding rather than only as a default.**
    /// The death is real (`marked 4095 of 4095 organisms senescent`) and
    /// completely invisible, which is this repo's *a debug readout must not
    /// be a function of the thing it debugs* pointed at the game itself: if
    /// standing dead is ever wanted on screen, it needs its own colour, not
    /// its own flag. [`Start::Dead`] stays reachable for that.
    ///
    /// Emptier than [`Start::Dead`]: no bones, no root systems, and a much
    /// thinner seed bank, since nothing ever set seed.
    #[default]
    Bare,
    /// Grown and held **alive** — this module's first answer, kept as the
    /// control. A living wood, stopped mid-life.
    Grown,
}

impl Start {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dead => "dead",
            Self::Bare => "bare",
            Self::Grown => "grown",
        }
    }
}

/// `PIXEL_PHYSICS_DRUID_START=dead|bare|grown` — see [`Start`].
const START_ENV: &str = "PIXEL_PHYSICS_DRUID_START";

/// `PIXEL_PHYSICS_DRUID_CIRCLE=off` — start with the carried circle switched
/// off. See [`Druid::toggle_carried_circle`].
///
/// **A control arm, not a setting.** The key that toggles the circle lives in
/// `src/bin/druid.rs`, and there is no way to press a key in a headless
/// capture — so without this, the *"what does it look like with the sphere
/// off"* question could only be answered by editing a line and rebuilding
/// between the two arms, which is exactly the shape that produces a
/// stale-binary comparison. One binary, one switch, nothing else different.
/// Anything but the literal `off` leaves the circle on.
const CIRCLE_ENV: &str = "PIXEL_PHYSICS_DRUID_CIRCLE";

/// The worldgen preset this game builds from — see `assets/worldgen.ron`,
/// where the reasoning for each value that differs from `rolling` is written
/// beside it.
pub const PRESET: &str = "druid";

/// `PIXEL_PHYSICS_DRUID_PRESET=name` — build from a different preset.
///
/// Exists so the preset can be judged the way this repo judges anything
/// visual: two renders side by side rather than one against a remembered
/// impression. Without it, comparing `druid` against `rolling` means editing
/// a constant and rebuilding between the two arms, which is the shape that
/// produces a stale-binary comparison.
const PRESET_ENV: &str = "PIXEL_PHYSICS_DRUID_PRESET";

/// **What a founding releases.** `ant` rather than `ancestor`: the outdoor ant
/// declares a nest, forages, and is the species the shipped instincts were
/// authored for, so generation zero works on day one. `ancestor` is the lab's
/// no-home control and would make "does a colony form at all" the question,
/// which is the lab's question rather than this game's.
const COLONY_SPECIES: &str = "ant";

/// The one plant the engine sows by a path of its own — see the seed-kind
/// list in [`Druid::new`] and the branch in [`Druid::plant_seed`].
const MOSS: &str = "moss";

/// How many animals a founding places.
///
/// **Not tuned, and it should not be until the economy exists.** The lab's own
/// measurement says the founding *moment* matters more than the count anyway —
/// dropped on seedlings a colony collapses at once, founded on grown plants it
/// seats fewer and holds (5 against 39 at frame 6,000 on the same bed) — and
/// this world is grown before it is held, so that condition is already met.
const COLONY_SIZE: i32 = 12;

/// **The economy, and every number in it is a first guess.**
///
/// `CLAUDE.md` says to set bars from measurement with headroom, never from an
/// aspiration — and there is nothing to measure yet, because nobody has
/// played this. So these are round numbers chosen to make the meter *move* at
/// a rate a person can watch, and the honest thing is to say so here rather
/// than to dress them as derived. The `U` key exists precisely because they
/// are wrong: it takes the economy out of the way so the mechanics can be
/// judged without it.
///
/// The shape is the part worth keeping. Drain is charged on **what is awake
/// inside a circle**, not on its radius — honest to the engine, since that is
/// literally what costs, and it means the same circle gets dearer as a colony
/// grows in it. Income is **animals only**: a wood with no colony pays
/// nothing (owner's ruling, 2026-09-13).
const POWER_START: f32 = 600.0;
/// Charged per standing circle per second, before anything living in it.
const DRAIN_PER_CIRCLE: f32 = 1.0;
/// ...and per plant standing inside one. A mature wood is expensive to keep
/// running; bare ground is nearly free.
const DRAIN_PER_PLANT: f32 = 0.02;
/// **What an animal stores, per second, while time is running for it.**
///
/// Owner's ruling, 2026-09-13, replacing a flat per-animal trickle over the
/// whole world: *"you don't automatically fill your bar based on all
/// creatures in the world. You fill based on number of creatures near you...
/// creatures build up a reserve that you absorb and they have to regenerate
/// before you can absorb again."*
///
/// **This is the loop closing.** Before it, income and expenditure were two
/// unrelated taps: you spent power to run time, and you were paid for animals
/// existing somewhere. Now the colony is a **battery you charge by spending**
/// — a creature only stores while it is *running*, which means inside a
/// circle, which means you paid for it. "Quicken my colony or my wood?"
/// becomes a real question every minute, and it is the same question the
/// whole game is about.
///
/// **That rule needs no code.** `World::time_runs_at` already decides it, the
/// same way it already decides that a colony must be founded inside running
/// time and that a sown seed waits for a circle to reach it. Three mechanics,
/// one gate.
const RESERVE_PER_SECOND: f32 = 1.5;

/// **How much one animal can hold.** ~27 seconds of running to fill, so a
/// colony of twelve is worth ~480 against a starting pool of 600: a lump
/// worth walking for rather than a trickle worth ignoring.
///
/// The number this is really setting is *how often you press the key*, and
/// the owner's warning shapes it: a button pressed every few seconds is worse
/// than no button. A cap this size makes the pull a slow one.
const RESERVE_CAP: f32 = 40.0;

/// **How near you have to be.** Deliberately smaller than
/// [`CARRIED_RADIUS`]: you have to stand *in* the colony, not near it.
const ABSORB_RADIUS: i32 = 60;

/// **What the druid sets out with, of each kind she can sow.**
///
/// Owner playtest, 2026-09-14: *"The world should not start with any seeds.
/// The druid has her own seeds to plant and that populates the world."* The
/// world half of that is three zeroed densities in `assets/worldgen.ron`;
/// this is the other half, and without it the first sentence just leaves an
/// empty world and an unlimited key, which is a bare map rather than a
/// mechanic.
///
/// **The pouch is per kind, not a single number**, so *"eight grass and no
/// oak"* is a state the game can be in. That is what makes cycling the seed
/// kind a decision rather than a preference.
///
/// Enough to establish a wood on bare ground and not enough to carpet it:
/// eight of each over seven sowable kinds is 56 seeds against a world 2,560
/// cells wide. A first guess, like every other number in this economy.
const SEED_START: u32 = 8;

/// **The most of one kind she can carry.** The pouch fills from the wood she
/// is standing in (see [`Druid::step_economy`]) and a cap is what stops a
/// mature wood quietly restoring the unlimited supply this replaced.
const SEED_CAP: u32 = 24;

/// **How big a plant has to be before it is worth seed to her**, in cells.
///
/// The middle the ethos asks for: a seedling you sowed a minute ago pays
/// nothing, a grown tree pays, and the gap between them is the time you spent
/// running the circle over it. Read off the cell count rather than the
/// species' own `seed_maturity` fence deliberately — that fence is a plant's
/// business and moves with the genome, and this is the *player's* question,
/// which is "does this look like a tree yet".
///
/// 24 cells against a grown tree's 31-153 (`CLAUDE.md`'s own spread), so a
/// young tree counts and a two-cell sprout does not.
const SEED_FROM_CELLS: usize = 24;

/// **Seeds per mature plant per second, while it stands in the circle she is
/// carrying.**
///
/// **The carried circle, not a standing one, and that is the verb.** Gathering
/// is *presence* — the same thing the carried circle already is — so the way
/// to fill the pouch is to walk your own wood. A standing quickening left
/// running over a wood while the player is elsewhere pays nothing, which is
/// what stops the supply going back to unlimited by being left switched on.
///
/// At this rate a wood of twenty mature plants under her feet is one seed
/// every ten seconds. A first guess.
const SEED_PER_PLANT_SECOND: f32 = 0.005;

/// How long the drawn energy takes to reach you, in player ticks. Long enough
/// to read as a flow rather than a flash.
///
/// **90, up from 42, on the owner's playtest**: *"a good start... make it
/// slower."* The stream had the right shape at 42 and went past too quickly
/// to watch, which is the same defect as a flash wearing a longer number.
const DRAW_FRAMES: u32 = 90;
/// How often the economy is recomputed, in ticks. Walking every organism is
/// `O(organisms)` and there are thousands, so this runs twice a second rather
/// than sixty times and scales what it charges.
/// **What laying scent costs**, per second held.
///
/// Priced at one standing circle, because that is what it is: a standing
/// instruction to the colony. A first guess like everything else in this
/// economy.
const TRAIL_PER_SECOND: f32 = 1.0;

/// **How strong the druid's mark is**, per tick, against one ant's
/// `pheromone::DEPOSIT` of 40.
///
/// The same, deliberately, and the strength comes from *repetition*: he walks
/// slower than one cell a tick, so each cell takes two or three marks and
/// ends at two to three ants' worth. A larger number here would saturate the
/// plane at 255 along the whole path, and a saturated trail is flat — which
/// is precisely the thing an ant cannot follow (see [`Druid::lay_trail`]).
const TRAIL_DEPOSIT: u8 = crate::sim::pheromone::DEPOSIT;

/// How many marks the trail readout remembers. Older ones have decayed out
/// of the plane long before this, so the cap is a memory bound and not a
/// rule.
const TRAIL_MARKS: usize = 900;

/// **How many streams a founding draws**, however many founders it places.
///
/// A cap rather than one per station: twenty-four streams is a wall of motes
/// and reads as noise, while four to eight reads as *several places at once*,
/// which is what a colony arriving is. The flow is the event, not a census of
/// it.
const FOUNDING_STREAMS: usize = 8;

/// **The tick rate the game is driven at.**
///
/// Here rather than in `src/bin/druid.rs`, which is where it lived and where
/// only the event loop could see it. Anything in the game that prices
/// something *per second* has to divide by it — `ECONOMY_INTERVAL` below is
/// "twice a second" expressed in ticks, and [`TRAIL_PER_SECOND`] is a rate —
/// and a second copy of the number in the lib would be the side table that
/// goes stale the day the loop is retimed.
pub const TICKS_PER_SECOND: u32 = 60;

const ECONOMY_INTERVAL: u64 = 30;

/// **How fast the world may be run inside the circles**, in ticks per frame.
///
/// Capped rather than open-ended, and the cap is a frame-cost bound rather
/// than a design statement: every extra tick is another pass of the whole
/// shared frame step, and although the held gate means almost all of that
/// pass does nothing, the sweep overhead is not zero. 8 is a starting cap to
/// be re-derived against a measured frame once somebody has played with it.
pub const SPEED_MIN: u32 = 1;
pub const SPEED_MAX: u32 = 8;

/// **How far the carried circle may be widened**, in cells.
///
/// The floor is [`CARRIED_RADIUS`] itself — the circle you *are* cannot be
/// made smaller than presence, or the gnome could stand outside his own time.
/// The ceiling is well under [`PLACE_RADIUS_MAX`]: a carried circle follows
/// you everywhere and costs nothing at its base size, so an unbounded one is
/// a free standing circle that never has to be placed.
pub const CARRIED_RADIUS_MAX: i32 = 96;

/// Radius a placed quickening starts at, and the range `Q`/`E` walk.
const PLACE_RADIUS_START: i32 = 60;
pub const PLACE_RADIUS_MIN: i32 = 20;
pub const PLACE_RADIUS_MAX: i32 = 240;

/// **How far the player walks before his carried circle wakes the ground
/// ahead of him.**
///
/// `World::wake_region` rebuilds both scheduler heaps, so calling it every
/// frame is exactly the unbounded per-frame cost the scheduler exists to
/// avoid. Calling it never means ground he has walked onto takes up to
/// `HELD_RECHECK` — about two seconds — to notice, which is a visible lag on
/// the one thing he does constantly. Half a radius is the compromise: bounded
/// (a handful of wakes a second at a run) and short enough that the ground
/// keeps up with him.
const CARRY_WAKE_STEP: i32 = CARRIED_RADIUS / 2;

/// **How long the interface holds on to the last thing that happened**, in
/// player ticks — three seconds.
///
/// Expiry is checked against [`Druid::ticks`] at draw time rather than ticked
/// down, which needs no update-phase wiring and has one deliberate
/// consequence `App::active_toast` records too: a message raised while
/// *paused* stays up until the world runs again. That is the behaviour worth
/// having here — paused is exactly when somebody is reading.
///
/// **Player ticks, not `world.frame`**: with a fast circle standing, the
/// world's frame counter advances at that circle's rate, and a message would
/// vanish in three eighths of a second. See [`Druid::ticks`].
const MESSAGE_FRAMES: u64 = 180;

/// **How small she gets**, in cells, against her authored 7x14.
///
/// Three tall because `creature::SPOIL_HEADROOM` is 3 and that is what an
/// ant's gallery clears at its most generous; two wide because an ant cuts a
/// one-cell bore and the rind either side leaves four to six void cells per
/// row, so two is through with room and three is wedged. A first guess like
/// every other number in this game's economy -- but a *derived* one: it
/// comes off the digger's own constants rather than off a feel, so the thing
/// to re-derive it against is a change to how ants dig, not a sweep.
const SMALL: (i32, i32) = (2, 3);

/// **Energy on its way from an animal to the player.**
///
/// Owner: *"There should be a visual for when creatures have built up energy
/// to drain and a really cool visual when you drain it. It should flow into
/// you."* So a draw is not a number that changes — it is a thing that
/// travels, and it takes [`DRAW_FRAMES`] to arrive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    /// The far end of the flow, in world cells — where it came *from* on a
    /// pull, and where it is going *to* on a founding.
    pub from: (i32, i32),
    /// **Which way it runs.** A pull converges on the player; a founding is
    /// the same flow reversed, spending the pool out into the ground.
    ///
    /// One flag rather than a second list, because the whole value of this
    /// being the drain's own machinery is that the founding then *looks* like
    /// the drain running backwards — which is what it is.
    pub outward: bool,
    /// Ticks since it was pulled; it lands at [`DRAW_FRAMES`].
    pub age: u32,
    /// How much, which sets how heavy the flow looks.
    pub amount: f32,
}

/// The whole game: the same quartet `App` and `Lab` each declare, because
/// there is no extracted game core in this engine and inventing one to hold
/// three callers would be the larger change.
pub struct Druid {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub renderer: Renderer,
    pub player_tuning: player::Tuning,
    pub player_input: player::PlayerInput,
    pub paused: bool,
    /// **The pool.** Seconds of world, in the fiction; a float here.
    pub power: f32,
    /// **Unlimited power — the playtest switch.** Owner's ask: judge the
    /// mechanics without the economy fighting you. Nothing is charged and
    /// nothing is collected while this is on, and the readout says so, since
    /// a full meter and a disabled one look identical.
    pub unlimited: bool,
    /// Radius the next placed quickening takes.
    pub place_radius: i32,
    /// **How fast time runs inside every circle**, in ticks per frame.
    ///
    /// **One dial for all of them, not one per circle, and that is a measured
    /// decision rather than a simplification.** A per-circle rate was built
    /// first and withdrawn: `World::frame` is a *global* clock and every
    /// organism's cadence is expressed in it, so running the world eight
    /// times to speed one circle speeds the scheduling of everything
    /// everywhere. Measured on two circles over equivalent ground, 1,500
    /// player ticks, unlimited power — a **rate-1** circle beside a rate-8
    /// one grew **127** living plant cells against **58** for the same circle
    /// when both were rate 1. It was running at nothing like real time.
    /// Per-circle rates need real regional time, not extra whole-world
    /// passes; `Reports/dead-ends.md` carries the entry.
    ///
    /// One dial has no cross-talk to leak, because every circle runs at it.
    ///
    /// **It multiplies what EATS your garden as well as what it costs you,
    /// and nothing on screen says so.** Lane E's diagnosis of the owner's
    /// *"absorbing destroys plants"* report, 2026-09-14: absorbing never
    /// touches the world at all — what eats a wood is ants grazing inside a
    /// quickening, and the dial runs them too. Measured, **228 plant cells
    /// eaten at speed 1 against 2,217 at speed 8**.
    ///
    /// So this doc and [`drain_for`] both price the dial honestly in *power*,
    /// and the player is told nothing about the other half of what he just
    /// bought. Closing it wants the on-screen note that the dial raises to
    /// name grazing as well as cost — which lives in `src/bin/druid.rs`,
    /// where `Z`/`V` write this field directly with no `Druid` method in
    /// between, so it is not a change this lane could make. Recorded here
    /// rather than dropped: the next session to touch the dial reads this.
    pub speed: u32,
    /// Income and drain as of the last recompute, for the readout. Per
    /// second, so a person can read them against a clock.
    pub income: f32,
    pub drain: f32,
    /// Where the carried circle was when it last woke the ground — see
    /// [`CARRY_WAKE_STEP`].
    last_wake: Option<(i32, i32)>,
    /// **The key legend, on by default.** The owner's first playtest found no
    /// interface at all and no way to guess one — see [`hud`].
    pub show_keys: bool,
    /// The last thing that happened and the frame it stops being shown on.
    /// Raised by [`Druid::note`]; see [`MESSAGE_FRAMES`].
    pub message: Option<(String, u64)>,
    /// **Player-time, in ticks.** One per [`Druid::update`], whatever the
    /// circles are doing.
    ///
    /// **`World::frame` is no longer this**, and that is the speed dial's one
    /// real hazard. An extra pass is a whole `frame::step`, so a rate-8
    /// circle advances `world.frame` by 8 per update — it has become "how
    /// much world has happened", which is the right meaning for the *world*
    /// and the wrong clock for anything the *player* experiences. Measured
    /// the moment the dial first ran: a census keyed on `world.frame` read a
    /// real-time circle at 2 living cells against 45 in the control, purely
    /// because it had had an eighth of the updates.
    ///
    /// Two things here were keyed on it and are now keyed on this instead.
    /// The economy was the dangerous one: `frame % 30 == 0` with frame
    /// stepping by 8 goes 0, 8, 16, 24, 32 and **never lands on 30**, so a
    /// single fast circle switched the whole economy off silently.
    pub ticks: u64,
    /// Animals alive, and animals in running time, as of the last economy
    /// pass. **Fields rather than a census**: `World::live_creature_count`
    /// walks every organism slot, which is thousands, and the readout is
    /// drawn every frame while the economy runs twice a second.
    pub animals: usize,
    pub animals_awake: usize,
    /// What the interface drew last frame, so a *change* can force the
    /// repaint the dirty-rect skip would otherwise not know it owed. See
    /// [`hud::Interface`].
    last_ui: Option<hud::Interface>,
    /// How the land arrived — see [`Start`]. Kept for the readout, so a
    /// player can tell a dead world from a bare one without counting trees.
    pub start: Start,
    /// **What `T` plants**, and the kinds it can cycle through.
    ///
    /// Built at run time from the loaded species rather than written down:
    /// every species with no `creature` block is a plant and can be sown, so
    /// adding a species file adds a seed kind and nothing here has to know.
    /// A hardcoded list is the side table that goes stale the day somebody
    /// writes `assets/species/fern.ron`.
    pub seed_kinds: Vec<String>,
    pub seed_kind: usize,
    /// **What is in the pouch**, index-parallel to `seed_kinds`.
    ///
    /// The supply used to be infinite: `plant_seed` read no resource and
    /// decremented nothing, so sowing was the one verb in this game that cost
    /// its player nothing at all. The owner took the world's own seeds away
    /// (*"The druid has her own seeds to plant"*), and a free supply on a bare
    /// map is just a slower way of painting one.
    ///
    /// Parallel `Vec`s rather than a map keyed by name because `seed_kind` is
    /// already an index into `seed_kinds` and a second representation of the
    /// same key is a second thing to keep in step. Both are built in
    /// [`Druid::new`] at `seed_kinds`' length and nothing resizes them.
    pub seeds: Vec<u32>,
    /// **Seed being gathered but not yet whole**, index-parallel to
    /// `seed_kinds`.
    ///
    /// Fractions rather than a probability roll per pass: a roll would make
    /// the first seed of a session arrive at a random time, and this economy
    /// is already hard enough to read. Whole seeds are taken out of here by
    /// [`Druid::step_economy`] and the remainder carries.
    seed_growth: Vec<f32>,
    /// How many seeds she has gathered, for the readout and for the same
    /// reason `sown` exists — gathering is slow and diffuse, and without a
    /// number a working mechanic and a dead one look identical.
    pub gathered: usize,
    /// **What each animal is holding**, keyed by organism id.
    ///
    /// On the game rather than on `OrganismState`, deliberately: this is the
    /// held world's economy and no part of it belongs to the sandbox or the
    /// lab, which share every line of `sim`. Slots are reused when an
    /// organism dies, so the map is pruned to the ids seen on each pass —
    /// otherwise a dead ant's charge would be inherited by whatever is
    /// allocated its slot next.
    pub reserves: std::collections::HashMap<OrganismId, f32>,
    /// Energy in flight from an animal to the player — see [`Draw`].
    pub draws: Vec<Draw>,
    /// **Which plane `G` writes to.** `Channel::A` by default — and that
    /// default is the engine's own, not a choice made here.
    ///
    /// `Channel`'s `#[default]` moved from `B` to `A` on 2026-09-09 and its
    /// doc says why: `open-bugs-handoff.md` §Z7 measured that the shipped ant
    /// **cannot read channel B at all** — hidden units 2/3 sit saturated at
    /// the exact input an empty ant has, so a hand-laid food trail moves the
    /// near-target ant-tick count by *exactly zero*, twice, on two
    /// independent harnesses. This game then went and named `B` explicitly,
    /// which threw that default away and shipped the one thing §Z7 calls the
    /// worst version of an unfinished verb: *"defaulting a player's first
    /// drag to the plane nothing acts on"*. A third harness — this game's own
    /// paired run — reproduced the same exact tie before anyone noticed.
    ///
    /// **B is still one key away and the screen says what it is worth.** The
    /// food route is the trail a player would *want* first; hiding it would
    /// be its own dishonesty. What changed is only the plane the first press
    /// lands on.
    pub scent: crate::sim::pheromone::Channel,
    /// **Where he has laid scent**, newest last — see [`Druid::lay_trail`].
    ///
    /// The marks are drawn by sampling the *plane* at these points rather
    /// than by remembering how bright they were, so a mark fades exactly as
    /// its scent does and vanishes when the scent is gone. Remembering the
    /// brightness instead would leave a drawn trail standing over ground that
    /// no longer smells of anything, which is the worst kind of readout: one
    /// that is a picture of the gesture rather than of the world.
    pub trail: std::collections::VecDeque<(i32, i32)>,
    /// **The options menu, while it is open** — see [`menu`]. `None` the
    /// rest of the time, the same one-piece-of-state shape as [`Druid::offer`].
    pub menu: Option<menu::Menu>,
    /// **The founding screen, while it is open.** `None` the rest of the
    /// time, which is also what says whether the game is showing it — one
    /// piece of state rather than an `open: bool` beside an `Offer` that can
    /// disagree with it.
    ///
    /// It survives being closed and reopened: walking away from an offer
    /// leaves the same three standing, and only committing rerolls. See
    /// [`founding`].
    pub offer: Option<founding::Offer>,
    /// How many seeds the player has sown, for the readout — *"did it fire at
    /// all needs a counter"*, and a seed dropped outside a quickening does
    /// nothing visible until time reaches it, so the picture cannot say.
    pub sown: usize,
}

impl Default for Druid {
    fn default() -> Self {
        Self::new()
    }
}

/// Why a founding placed nobody, in the player's words.
///
/// **Three refusals used to wear one message and that was the bug.** `C` is
/// the first thing a player presses and the only thing in the held world that
/// makes an animal, so a refusal that names the wrong cause reads as the
/// whole feature being broken. "nothing founded - no ground here" was said
/// while standing plainly on ground, because the world was at the 4,095
/// organism ceiling and every station reached the allocator and was turned
/// away (`Reports/open-bugs-handoff.md` §Z21). The ground rule and the
/// allocator are independent walls: with the thicket repair on, the same
/// nine stands were offered **63 stations against 31** and placed the
/// **identical 2** animals.
///
/// A free function taking the two facts rather than a method, so both
/// founding paths -- `Druid::found_colony` at the player's feet and
/// `found_from_offer` -- reach the same wording from the same inputs. Two
/// call sites choosing their own strings is how the third case went unnamed
/// in the first place.
///
/// `no_slots` is read from `World::organisms_refused` either side of the
/// founding, never inferred: a stand that seats nobody because the ground
/// refused it and one that seats nobody because the world is out of
/// identities are the same number without that counter.
fn refusal_note(stations_offered: usize, no_slots: bool) -> &'static str {
    // Slots first, and deliberately: it is the only one of the three the
    // player cannot act on by moving, which is what both other wordings
    // tell them to do.
    if no_slots {
        "no room for another living thing - the world is full"
    } else if stations_offered == 0 {
        "no ground here - stand on something solid"
    } else {
        "no room - the ground here is full. try open ground"
    }
}

impl Druid {
    /// Generate a world, live in it for a while, then stop it.
    pub fn new() -> Self {
        let (w, h) = size_from_env();
        let start = start_from_env();
        // **`Bare` is the one that does not grow.** `Dead` still grows -- it
        // has to, or there are no bodies to leave standing and no seed bank
        // in the soil; it kills what it grew instead.
        let grow = if start == Start::Bare { 0 } else { grow_from_env() };
        println!("druid: world {w}x{h}, start {}, grown {grow} frames before holding", start.label());

        let mut world = World::new(Rect::new(0, 0, w as i32 - 1, h as i32 - 1));

        // **Species alongside materials.** Shipping one without the other is
        // a mistake this repo has already made once — `SpeciesRegistry::
        // reload` existed, was tested, and had no caller, so editing a
        // species file silently did nothing.
        let _ = world.materials.reload(material::ASSET_DIR);
        let _ = world.species.reload(organism::ASSET_DIR);
        // **Plants do not come apart under their own load here, by default.**
        // Owner, 2026-09-14, asking for the menu this sits behind: *"the
        // ability to turn off plant destruction or breaking due to stress
        // (which should be off by default)."* The engine default is `true`
        // and stays `true` — the outdoor game and `scripts/acceptance.sh`'s
        // `fell` case are untouched; this is the held world choosing
        // differently, which is what a per-game field is for.
        //
        // **Only a *living* plant is held.** A senescent one comes apart
        // exactly as before, so culling, rot and felling still work — the
        // switch's own doc records the owner reporting *"I turned COLLAPSE
        // UNDER LOAD off, but trees are still falling over"* against an
        // earlier version that got that distinction wrong.
        world.plant_load_failure = false;

        // **The druid preset, not the shipped default.** `rolling` is a
        // mining world -- the first druid build generated one and put the
        // gnome in a gorge between two screen-high rock walls, which is the
        // wrong silhouette for a game about growing things. Falls back to the
        // default rather than failing, so a stripped or edited asset set
        // still starts.
        let (presets, _err) = WorldgenPresets::load();
        let wanted = std::env::var(PRESET_ENV).unwrap_or_else(|_| PRESET.to_string());
        let preset = if presets.get(&wanted).is_some() { wanted } else { presets.default_name() };
        println!("druid: preset {preset}");
        match presets.get(&preset) {
            // `generate`, never `generate_only`: the latter skips
            // `compute_world_distances`, so the world would have no
            // structural field and nothing would ever fall correctly.
            Some(params) => worldgen::generate(&mut world, worldgen::Spec::Generated { params, seed: 1 }),
            None => worldgen::generate(&mut world, worldgen::Spec::Legacy),
        }

        if let Ok(Some(clock)) = crate::sim::clock::Clock::load() {
            world.clock = clock;
        }

        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::with_tuning(explosion::Tuning::load());
        let player_tuning = player::Tuning::load();

        // --- the world lives -------------------------------------------
        //
        // Full ticks rather than a subsystem loop, so what grows here is
        // exactly what would have grown had somebody watched it: the same
        // sweep, the same weather, the same settling.
        let t0 = std::time::Instant::now();
        for _ in 0..grow {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &player_tuning);
        }
        let grown = world.live_organism_count();
        println!("druid: grew {grown} organisms in {:.1}s", t0.elapsed().as_secs_f32());

        // --- ...and then it dies ---------------------------------------
        //
        // Only the plants. The animals are not touched, because there are
        // none yet -- nothing in worldgen places one, and founding a colony
        // is the player's verb.
        if start == Start::Dead {
            let plants: Vec<OrganismId> = world
                .live_organism_ids()
                .into_iter()
                .filter(|id| world.organism(*id).is_some_and(|st| world.species.get(st.species).creature.is_none()))
                .collect();
            let killed = plants.iter().filter(|id| world.mark_organism_senescent(**id)).count();
            // **Counted, because the picture cannot tell you.** A wood of
            // senescent trees and a wood of living ones are the same
            // silhouette until something rots, and in a held world nothing
            // ever will until the player spends time on it.
            println!("druid: marked {killed} of {grown} organisms senescent — the wood is standing dead");
        }

        // --- and then it stops -----------------------------------------
        //
        // The sky pin is not decoration and not an optimisation, though it is
        // both: in the look the owner picked, nothing changes colour, so the
        // *only* tells are that nothing moves and the sky does not turn. A
        // held world under a running sun crawls its shadows across stopped
        // ground and reads as a bug rather than as a state.
        world.held = true;
        world.set_sky_hold(SkyPin::Noon.hold());

        // **Every species this game knows how to sow**, in registry order, so
        // a new species file becomes a seed kind with no edit here.
        //
        // **The predicate is measured, not guessed, and two obvious ones are
        // wrong.** `creature.is_none()` admits the **worm**, which is an
        // animal driven by `creature.rs` keyed on its species *name* and has
        // no `creature` block at all. `has_economy()` excludes **moss**,
        // which declares no `Photosynthesize` anywhere (its own file says
        // so). Swept over all twenty loaded species, a declared `Seed` cell
        // type is true for exactly the seven sowable plants -- tree, conifer,
        // shrub, creeper, grass, herb, scrambler -- and false for moss, the
        // worm and every ant.
        //
        // Moss is added because the engine has **two** sowing paths, not one:
        // `plant_tree_species` needs a seed-shaped species and
        // `World::plant_moss_seed` is moss-only. `Druid::plant_seed` branches
        // on the same fact, so this mirrors the engine rather than keeping a
        // list beside it.
        let seed_kinds: Vec<String> = (0..world.species.len())
            .map(|i| world.species.get(crate::sim::organism::SpeciesId(i as u16)))
            .filter(|sp| is_sowable(sp))
            .map(|sp| sp.name.clone())
            .collect();
        println!("druid: {} seed kinds — {}", seed_kinds.len(), seed_kinds.join(", "));

        if let Some((x, y)) = spawn_point(&world) {
            // `at_scaled`, not `at`: at any `cell_scale` other than 1 the
            // plain constructor builds a half-size gnome.
            world.player = Some(player::Player::at_scaled(x, y, world.cell_scale()));
        }
        // The headless control arm -- see `CIRCLE_ENV`. Echoed, because a
        // switch nobody can see the value of is a switch nobody can tell is
        // disconnected.
        world.carried_off = std::env::var(CIRCLE_ENV).is_ok_and(|v| v.trim().eq_ignore_ascii_case("off"));
        if world.carried_off {
            println!("druid: starting with the carried circle OFF ({CIRCLE_ENV})");
        }

        Self {
            world,
            particles,
            blasts,
            // **One hue is the held world's default look, not a key away.**
            // Owner playtest, 2026-09-14: *"One hue should become default."*
            // Set here rather than by moving `HeldLook`'s own `#[default]`,
            // because that enum is shared with the other two games and this
            // is a decision about *this* one — `apply_held_look` early-outs
            // on `!world.held`, so the sandbox and the lab could not see the
            // change either way, and a default nobody else can observe is
            // better stated where it is meant than hidden in a shared
            // derive.
            renderer: {
                let mut r = Renderer::new();
                r.held_look = crate::render::HeldLook::OneHue;
                r
            },
            player_tuning,
            player_input: player::PlayerInput::default(),
            paused: false,
            power: POWER_START,
            unlimited: false,
            place_radius: PLACE_RADIUS_START,
            speed: SPEED_MIN,
            income: 0.0,
            drain: 0.0,
            last_wake: None,
            show_keys: true,
            message: None,
            ticks: 0,
            animals: 0,
            animals_awake: 0,
            last_ui: None,
            start,
            seeds: vec![SEED_START; seed_kinds.len()],
            seed_growth: vec![0.0; seed_kinds.len()],
            seed_kinds,
            seed_kind: 0,
            sown: 0,
            gathered: 0,
            scent: crate::sim::pheromone::Channel::default(),
            trail: std::collections::VecDeque::new(),
            menu: None,
            offer: None,
            reserves: std::collections::HashMap::new(),
            draws: Vec::new(),
        }
    }

    /// **Say what just happened, on screen.**
    ///
    /// Every verb in this game used to report to stdout and nowhere else, so
    /// a founding that placed nobody and one that placed twelve were the same
    /// event to the person playing. The `println!`s stay — they are how a
    /// headless run is read — and this is the same fact put where the player
    /// is looking.
    pub fn note(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), self.ticks + MESSAGE_FRAMES));
    }

    /// The current message, if one is set and has not yet expired.
    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().filter(|(_, until)| self.ticks < *until).map(|(text, _)| text.as_str())
    }

    /// Everything the corner readout says, as numbers. See [`hud::Readout`].
    pub fn readout(&self) -> hud::Readout {
        hud::Readout {
            power: self.power,
            income: self.income,
            drain: self.drain,
            unlimited: self.unlimited,
            animals: self.animals,
            animals_awake: self.animals_awake,
            circles: self.world.quickenings.len(),
            radius: self.place_radius,
            rate: self.speed,
            carried_radius: self.world.carried_radius,
            carried_off: self.world.carried_off,
            seeds: self.seeds_in_hand(),
            held: self.world.held,
            paused: self.paused,
            look: self.renderer.held_look.label(),
            seed_kind: self.seed_kind_name().to_string(),
            sown: self.sown,
            charge: self.charge_in_reach(),
            reserve_cap: RESERVE_CAP,
            power_full: POWER_START,
            message: self.message().map(str::to_string),
        }
    }

    /// **Sow a seed of the chosen kind where the player is standing.**
    ///
    /// The verb the whole concept is built on: *"you can plant seeds you
    /// get"*. `World::plant_tree_species` places a `seed`-material cell,
    /// which is a `Powder` and therefore falls to the ground on its own
    /// rather than hanging where it was dropped — so the player aims at a
    /// bank, not at a pixel.
    ///
    /// **A seed sown outside a quickening does nothing, and that is the
    /// game rather than a bug.** Germination is a life process and the held
    /// gate covers it, so a seed dropped on cold ground lies there until the
    /// player spends time on it. Nothing here implements that; it falls out
    /// of the gate, exactly as the rule that a colony must be founded inside
    /// running time does.
    ///
    /// Returns whether a seed was actually placed. **`false` is a real
    /// answer** and is reported: `plant_tree_species` declines when the cell
    /// is occupied or the species is not loaded, and a silent no-op is
    /// indistinguishable from a key that does not work — which is precisely
    /// the complaint that produced this game's interface.
    pub fn plant_seed(&mut self) -> bool {
        let Some(player) = &self.world.player else {
            return false;
        };
        let (x, y) = player.center();
        let Some(kind) = self.seed_kinds.get(self.seed_kind).cloned() else {
            self.note("no seed kinds are loaded");
            return false;
        };
        // **The pouch is checked before the ground is.** An empty pouch and a
        // blocked cell are different refusals and both are said out loud --
        // the same rule the rest of this game's verbs follow -- but they are
        // checked in this order so that "you have none" is never reported as
        // "no room", which is the reading that would send a player looking
        // for better ground with nothing to plant in it.
        if self.seeds.get(self.seed_kind).copied().unwrap_or(0) == 0 {
            println!("druid: {kind} seed REFUSED at {x},{y} - the pouch is empty");
            self.note(format!("no {kind} seed left - stand in a grown wood to gather"));
            return false;
        }
        // **Read before, so a refusal can say which refusal it was.** Both
        // planters answer a bare `false`/nothing, and two completely different
        // failures arrive that way: the cell is occupied, or the engine has
        // run out of organism slots. `World::organisms_refused` is the only
        // thing that separates them, and it is a counter rather than a return
        // value, so it has to be sampled across the call.
        //
        // **The slot ceiling is real and the held world sits near it.**
        // `Cell::organism_id` gives 12 bits to the slot index, so the world
        // holds 4,095 organisms; measured by Lane C on a *grown* start, 2026-
        // 09-14, `Druid::new` arrives at **4,093 of them** and further births
        // are refused (`open-bugs-handoff.md` §Z21). On `Start::Bare` --
        // the default, and with `life_scatter` now writing nothing -- the
        // table starts empty, which is the most relief that pressure gets
        // from anything in this change. It is relief and not a fix: a player
        // sowing freely in a running world can still reach the ceiling, and
        // the failure there is a birth that silently does not happen. Saying
        // so is this game's own standing rule, and a refusal nobody can see
        // is the shape of bug it keeps filing.
        let refused_before = self.world.organisms_refused();
        // Moss is not tree-shaped and has its own planter; everything else
        // goes through the species-named one.
        let placed = if kind == MOSS {
            self.world.plant_moss_seed(x, y);
            // `plant_moss_seed` returns nothing, so ask the world instead of
            // assuming -- the same reason the branch below reads a bool.
            !self.world.is_empty(x, y)
        } else {
            self.world.plant_tree_species(x, y, &kind)
        };
        if placed {
            self.sown += 1;
            // **Spent only on a seed that is actually in the ground.** A
            // refused placement above already returned; this is the branch
            // where the cell took it, and charging for a no-op is the one
            // way an inventory can be worse than no inventory at all.
            if let Some(n) = self.seeds.get_mut(self.seed_kind) {
                *n = n.saturating_sub(1);
            }
            let running = self.world.time_runs_at(x, y);
            let left = self.seeds_in_hand();
            println!("druid: sowed {kind} at {x},{y} (time {}, {left} left)", if running { "running" } else { "held" });
            // **The count that is left is on the message, not only the corner
            // panel.** The pouch going from 1 to 0 is the moment the mechanic
            // exists at all, and a player watching his own feet is not
            // watching the readout.
            self.note(if running {
                format!("{kind} seed sown - it is growing ({left} left)")
            } else {
                format!("{kind} seed sown - it waits for time ({left} left)")
            });
        } else if self.world.organisms_refused() > refused_before {
            // **The world is full, and the seed is still in her pouch.** A
            // different sentence from the one below on purpose: "no room
            // here" sends a player looking for better ground, and there is
            // none -- no cell anywhere in the world will take a seed until
            // something dies. See §Z21.
            println!(
                "druid: {kind} seed REFUSED at {x},{y} - the organism table is full ({} refused so far)",
                self.world.organisms_refused()
            );
            self.note("the world is full - nothing can be born until something dies");
        } else {
            println!("druid: {kind} seed REFUSED at {x},{y} - the cell is not empty, or the species is not loaded");
            self.note(format!("no room for a {kind} seed here"));
        }
        placed
    }

    /// **A game with no world worth speaking of**, for guards over rules that
    /// are about the player's own state rather than about the ground.
    ///
    /// [`Druid::new`] generates 2560x960 and then lives in it, which is about
    /// a minute of wall clock — and `hud::Readout`'s own doc records why that
    /// matters: *a guard that costs a minute is a guard nobody runs*. The
    /// pouch rules need a `Druid` and a player, and nothing else.
    ///
    /// Test-only, and deliberately not a `Default`: every field it leaves at
    /// zero is a field a real game sets, and a constructor that looks usable
    /// would eventually be used.
    #[cfg(test)]
    fn bare_for_test() -> Self {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        // **The registries, or `plant_tree_species` declines every call** and
        // a guard over spending a seed would never once reach the spend. Both
        // are `include_str!`'d, so this is a parse and not a file read.
        let _ = world.materials.reload(material::ASSET_DIR);
        let _ = world.species.reload(organism::ASSET_DIR);
        world.held = true;
        // **A floor, because a world with no ground is a scene error wearing
        // a null result** -- `CLAUDE.md`'s own *a scene that contradicts the
        // code will look like a bug in the code*. Without it the player falls
        // out of the world and `plant_seed` returns `false` from its very
        // first line, which reads exactly like a broken pouch.
        for x in 0..64 {
            for y in 48..64 {
                world.set(x, y, crate::sim::cell::Cell::new(material::STONE, 0));
            }
        }
        let (sx, sy) = spawn_point(&world).expect("the hand-built floor must be standable");
        world.player = Some(player::Player::at_scaled(sx, sy, world.cell_scale()));
        Self {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::with_tuning(explosion::Tuning::load()),
            renderer: Renderer::new(),
            player_tuning: player::Tuning::load(),
            player_input: player::PlayerInput::default(),
            paused: false,
            power: POWER_START,
            unlimited: false,
            place_radius: PLACE_RADIUS_START,
            speed: SPEED_MIN,
            income: 0.0,
            drain: 0.0,
            last_wake: None,
            show_keys: true,
            message: None,
            ticks: 0,
            animals: 0,
            animals_awake: 0,
            last_ui: None,
            start: Start::Bare,
            seeds: Vec::new(),
            seed_growth: Vec::new(),
            seed_kinds: Vec::new(),
            seed_kind: 0,
            sown: 0,
            gathered: 0,
            scent: crate::sim::pheromone::Channel::default(),
            trail: std::collections::VecDeque::new(),
            menu: None,
            offer: None,
            reserves: std::collections::HashMap::new(),
            draws: Vec::new(),
        }
    }

    /// Step which kind `T` sows.
    pub fn cycle_seed_kind(&mut self) {
        if self.seed_kinds.is_empty() {
            return;
        }
        self.seed_kind = (self.seed_kind + 1) % self.seed_kinds.len();
        let kind = self.seed_kinds[self.seed_kind].clone();
        // **The stock is on the label**, because with a per-kind pouch the
        // whole reason to press this key is to find the kind you still have.
        let held = self.seeds_in_hand();
        self.note(format!("seed kind: {kind} ({held} in hand)"));
    }

    /// What `T` would sow, for the readout.
    pub fn seed_kind_name(&self) -> &str {
        self.seed_kinds.get(self.seed_kind).map_or("none", String::as_str)
    }

    /// **How many of the selected kind are in the pouch.** See [`Druid::seeds`].
    pub fn seeds_in_hand(&self) -> u32 {
        self.seeds.get(self.seed_kind).copied().unwrap_or(0)
    }

    /// **Switch the circle she carries off, or back on.** Returns whether it
    /// is now *on*.
    ///
    /// Owner playtest, 2026-09-14: *"There should be an easy way to full turn
    /// off the sphere around the druid so no power is being used."*
    ///
    /// **What the premise got right and what it did not, recorded here
    /// because the next person to read this will assume the same thing.** The
    /// carried circle at its base size already costs exactly zero —
    /// [`carried_cost`] prices the area *added*, so an untouched circle is
    /// free and there is a guard saying so. Nothing here is a saving unless
    /// the player has widened it with `]`, and then it is the widening that
    /// stops being billed. What was genuinely missing is the thing the words
    /// say: a way to have the world **hold still** where she is standing.
    /// That is what this is.
    ///
    /// **It is a real trade, which is what stops it being a free button.**
    /// Off, the colony under her feet stores no charge, a seed she has sown
    /// does not germinate, and the wood she is in stops growing — every one
    /// of those falls out of `World::time_runs_at` and needed no code, the
    /// same way the rest of this game's rules do.
    ///
    /// Sets `world.carried` to `None` on the spot rather than waiting for the
    /// next `frame::step` to notice, so the screen and the readout agree
    /// within the frame the key was pressed in.
    pub fn toggle_carried_circle(&mut self) -> bool {
        self.world.carried_off = !self.world.carried_off;
        if self.world.carried_off {
            self.world.carried = None;
            println!("druid: carried circle OFF — time stands still where you stand");
            self.note("your circle is off - time stands still here");
        } else {
            // **The wake tracker is cleared, not left.** `Druid::update` only
            // wakes the ground when the circle has moved `CARRY_WAKE_STEP`
            // from where it last woke it — so switching back on while
            // standing still would match the old position and leave the
            // ground she is on asleep until she walked away from it.
            self.last_wake = None;
            println!("druid: carried circle ON");
            self.note("your circle is back - time runs where you stand");
        }
        !self.world.carried_off
    }

    /// Whether the carried circle is switched off — see
    /// [`Druid::toggle_carried_circle`].
    pub fn carried_off(&self) -> bool {
        self.world.carried_off
    }

    /// **Total charge standing within reach**, for the readout and for the
    /// key's own decision.
    pub fn charge_in_reach(&self) -> (f32, usize) {
        let Some(player) = &self.world.player else {
            return (0.0, 0);
        };
        let (px, py) = player.center();
        let mut total = 0.0;
        let mut n = 0;
        for (id, held) in &self.reserves {
            if *held <= 0.0 {
                continue;
            }
            let Some(state) = self.world.organism(*id) else { continue };
            let Some((x, y)) = state.chain.first().copied().or_else(|| state.cells.keys().next().copied()) else {
                continue;
            };
            let (dx, dy) = (x - px, y - py);
            if dx * dx + dy * dy <= ABSORB_RADIUS * ABSORB_RADIUS {
                total += *held;
                n += 1;
            }
        }
        (total, n)
    }

    /// **Draw the charge out of every animal within reach.**
    ///
    /// A key rather than a trickle, and that is the point: `CLAUDE.md`'s
    /// second law is *there must be a verb, and it must deliver something*.
    /// An automatic drip is weather; walking into your colony and pulling is
    /// a moment.
    ///
    /// Returns what was taken. Zero is a real answer and is said out loud —
    /// standing in an uncharged colony and standing in no colony look
    /// identical otherwise.
    pub fn absorb(&mut self) -> f32 {
        let Some(player) = &self.world.player else {
            return 0.0;
        };
        let (px, py) = player.center();
        let mut taken = 0.0;
        let mut from: Vec<((i32, i32), f32)> = Vec::new();
        let ids: Vec<OrganismId> = self.reserves.keys().copied().collect();
        for id in ids {
            let held = self.reserves.get(&id).copied().unwrap_or(0.0);
            if held <= 0.0 {
                continue;
            }
            let Some(state) = self.world.organism(id) else { continue };
            let Some((x, y)) = state.chain.first().copied().or_else(|| state.cells.keys().next().copied()) else {
                continue;
            };
            let (dx, dy) = (x - px, y - py);
            if dx * dx + dy * dy > ABSORB_RADIUS * ABSORB_RADIUS {
                continue;
            }
            taken += held;
            from.push(((x, y), held));
            // **Emptied, not reduced.** They regenerate from nothing, and
            // only while running — which is what stops you camping one
            // colony and makes the map worth walking.
            self.reserves.insert(id, 0.0);
        }
        for (at, amount) in from {
            self.draws.push(Draw { from: at, outward: false, age: 0, amount });
        }
        if taken > 0.0 {
            self.power += taken;
            println!("druid: drew {taken:.0} from {} animals", self.draws.len());
            self.note(format!("drew {taken:.0} from the colony"));
        } else {
            // **Said out loud, like every other refusal here.** Standing in
            // an uncharged colony, standing in a starved one and standing
            // nowhere near a colony are three different situations and one
            // silent key.
            let (charge, holders) = self.charge_in_reach();
            println!("druid: absorb took nothing — {charge:.0} charge in {holders} animals within {ABSORB_RADIUS}, {} alive", self.animals);
            self.note("nothing charged within reach");
        }
        taken
    }

    /// **Found a colony at the player's feet.**
    ///
    /// The concept's central act: a colony is how you eat, how you reach past
    /// your own circle, and what you pay for in time. Nothing in worldgen
    /// places one — `found_colony_of` is called only by tests and the lab —
    /// so without this the world has plants and no animals, and an economy
    /// whose only income is animal metabolism could only ever drain.
    ///
    /// **It lands at his feet on purpose, and that is a rule rather than a
    /// convenience.** A colony in held ground does not tick: creatures run on
    /// the active-site schedule, which `scheduler::step` gates on
    /// `time_runs_at`. So a colony has to be founded *inside* running time or
    /// it stands there as scenery — and the carried quickening is exactly the
    /// circle at his feet. The rule needs no code; it falls out of the gate.
    ///
    /// Returns how many animals were placed. **Zero is a real answer** and is
    /// reported rather than swallowed: `found_colony_of` declines when there
    /// is no ground, no such species, or the species' nest material is
    /// missing, and a silent no-op is indistinguishable from a broken
    /// feature.
    pub fn found_colony(&mut self) -> usize {
        let Some(player) = &self.world.player else {
            return 0;
        };
        let (x, y) = player.center();
        // **The slot ceiling is a *third* refusal and it has to be read, not
        // inferred.** `found_colony_of` returns 0 for three unrelated
        // reasons -- no ground, no nest material, and no organism slots --
        // and this line said "no ground here" for all three. On a world at
        // the ceiling that is a confident, specific and wrong cause: the
        // ground is fine and the world is out of identities
        // (`Reports/open-bugs-handoff.md` §Z21, measured at 4,095 of 4,095
        // with 26 births refused over nine stands). `World::organisms_refused`
        // is the engine's own counter, incremented inside `push_organism` on
        // the far side of the call, so reading it either side of the
        // founding is the one thing that tells the three apart -- and it is
        // what the investigation itself needed before it could.
        let refused_before = self.world.organisms_refused();
        let placed = self.world.found_colony_of(x, y, COLONY_SPECIES, COLONY_SIZE);
        let no_slots = self.world.organisms_refused() > refused_before;
        println!(
            "druid: founded {placed} animals at {x},{y} (colony {})",
            if placed > 0 {
                "took"
            } else if no_slots {
                "REFUSED - no organism slots"
            } else {
                "REFUSED - no ground, or no nest material"
            }
        );
        // Counted here rather than waiting for the next economy pass: half a
        // second of a readout still saying zero, right after the key that was
        // meant to change it, reads as the key not working.
        self.animals += placed;
        match placed {
            // Through the same classifier as `found_from_offer`, so the two
            // verbs cannot drift apart in what they call the same refusal.
            // `found_colony_of` does its own siting and hands back no
            // station list, so the ground case is reported as "no ground"
            // -- which is right, since that path declines for want of
            // ground or nest material and nothing else.
            0 => self.note(refusal_note(0, no_slots)),
            n => self.note(format!("founded {n} animals at your feet")),
        }
        placed
    }

    /// **Lay a scent trail where he is standing.** Held, not tapped: the
    /// gesture is walking a route, and the route is the instruction.
    ///
    /// **Which plane it writes to is [`Druid::scent`], and the first version
    /// of this doc argued the wrong case at length.** It reasoned correctly
    /// from `ant.ron`'s wiring — units 2/3 carry `PheroBAlong` into `Move`
    /// gated on an **empty** ant, so B is the "there is food that way" trail
    /// and A is the laden ant's road home — and then concluded that laying A
    /// "would tell a colony where its own nest is, which it already knows".
    /// That is true of the *meaning* and irrelevant to the *outcome*:
    /// `open-bugs-handoff.md` §Z7 had already measured that **nothing can
    /// read B**, so the well-reasoned channel was the dead one. Reading a
    /// wiring diagram is not the same as asking whether the wire carries
    /// anything, and the bug register had the answer the whole time.
    ///
    /// **Why a trail he lays is followable at all**, which is not obvious and
    /// is the whole mechanic: the ant reads the *gradient* along its heading,
    /// so a trail of uniform strength says nothing. What supplies the slope is
    /// `DECAY_RHO` — every mark is fading from the moment it is laid, so the
    /// freshest cell on the path is the strongest, and the slope points back
    /// along the route to wherever he is now. Walk from the nest to where you
    /// want them and they come up the path behind you; stop, and the peak
    /// stays where you stopped. **He does not push them, he is the thing they
    /// are walking toward.**
    ///
    /// On channel A that reads as *bring what you are carrying to this spot*,
    /// and it competes with the nest's own A emission — so laying it badly
    /// strands a laden colony short of home. A verb that can be misused is a
    /// verb with stakes.
    ///
    /// Returns whether anything was laid, so a refusal is a real answer.
    pub fn lay_trail(&mut self) -> bool {
        let Some(player) = &self.world.player else {
            return false;
        };
        let (x, y) = player.center();
        let cost = TRAIL_PER_SECOND / TICKS_PER_SECOND as f32;
        if !self.unlimited {
            if self.power < cost {
                return false;
            }
            self.power -= cost;
        }
        self.world.deposit_pheromone(self.scent, x, y, TRAIL_DEPOSIT);
        // One entry per cell, not per tick: standing still would otherwise
        // fill the readout with nine hundred copies of one point and push
        // the rest of the route out of it.
        if self.trail.back() != Some(&(x, y)) {
            if self.trail.len() >= TRAIL_MARKS {
                self.trail.pop_front();
            }
            self.trail.push_back((x, y));
        }
        true
    }

    /// **Flip the plane `G` writes to**, A <-> B.
    ///
    /// The lab's `SCENT` tool has the same verb for the same reason
    /// (`lab::ui::Action::ToggleScentChannel`) — there, a second press of the
    /// tool's own key does it, because the tool is *armed*. `G` here is
    /// **held** rather than armed, so a second press cannot mean anything
    /// different from the first and the flip needs a key of its own.
    ///
    /// `Alarm` is deliberately not in the cycle: it is not a trail, it is one
    /// event at one cell written by a bite, and painting a swath of it would
    /// be a player-only quantity nothing in the engine ever produces
    /// (`lab::ui::Tool::Alarm` makes the same argument at more length).
    /// **Small enough to walk into a nest, or back to her own size.**
    ///
    /// The verb behind step 2 of the held world's plan. The geometry decides
    /// it and no new rule was needed: an ant digs one cell at a time, so a
    /// gallery is `SPOIL_HEADROOM` = 3 cells of headroom at its most
    /// generous, and `SPOIL_HEADROOM` is this engine's own definition of
    /// *indoors* -- `creature::is_sheltered` reads three empty cells
    /// overhead as outdoors. So [`SMALL`] is 2x3: it fits the widest third
    /// of an ant's galleries and is stopped by the 1- and 2-tall stretches,
    /// which is a **graded** outcome rather than a door that is open or
    /// shut.
    ///
    /// **Measured before it was built, which is what licensed building it.**
    /// `examples/burrow_probe arms=colony box=2x3`, twelve seeds at frame
    /// 8,000: a 2x3 body reaches **54 to 92 percent** of the roofed void a
    /// colony digs, and on eleven of twelve seeds the largest single region
    /// *is* that whole reach -- one connected run rather than a set of
    /// pockets. The same probe at `box=7x14`, her own size, reads **0% at
    /// every sample on every seed**: she cannot get in at all, which is the
    /// premise of the feature stated as a number.
    ///
    /// **Growing back can be refused, and that refusal is the mechanic**
    /// rather than a failure of it. `player::try_resize` tests her full
    /// rectangle before committing and declines rather than shoving, so
    /// being small in a tunnel is a thing you have to get yourself out of.
    /// `SPOIL_THROW` is unscaled, so digging while small can seal the way
    /// she came -- the most interesting hazard in the feature, and it needed
    /// no code.
    /// Whether she is in her small shape. Read from the body itself rather
    /// than from a flag beside it — a second copy of "am I small" is a
    /// second thing that can be wrong, and `player::try_resize` can refuse.
    pub fn is_small(&self) -> bool {
        self.world.player.as_ref().is_some_and(|p| (p.w, p.h) == SMALL)
    }

    pub fn toggle_small(&mut self) -> bool {
        let Some(mut p) = self.world.player.take() else {
            return false;
        };
        let want = if (p.w, p.h) == SMALL { (player::PLAYER_WIDTH, player::PLAYER_HEIGHT) } else { SMALL };
        let done = player::try_resize(&self.world, &mut p, want, &self.player_tuning);
        self.world.player = Some(p);
        // **A refusal says so, in the world's words rather than the code's.**
        // A verb that silently does nothing is the failure the ethos names:
        // if an event produces no visible consequence it is not finished.
        self.note(match (done, want == SMALL) {
            (true, true) => "you are small, and the ground is a country",
            (true, false) => "you stand your own height again",
            (false, true) => "there is not room here to change",
            (false, false) => "no room to grow -- find somewhere it opens out",
        });
        done
    }

    pub fn cycle_scent(&mut self) {
        use crate::sim::pheromone::Channel;
        self.scent = if self.scent == Channel::A { Channel::B } else { Channel::A };
        self.note(match self.scent {
            Channel::A => "scent: the way home - laden ants follow this",
            _ => "scent: the way to food - NOTHING CAN READ THIS YET",
        });
    }

    /// **Open the options menu, or shut it again.**
    pub fn toggle_menu(&mut self) {
        if self.menu.take().is_some() {
            return;
        }
        self.menu = Some(menu::Menu::default());
    }

    /// **Open the founding screen, or shut it again.**
    ///
    /// The offer itself outlives the screen — see [`Druid::offer`] — so
    /// closing is genuinely walking away rather than declining, and the same
    /// three lineages are there when you come back.
    pub fn toggle_founding(&mut self) {
        if self.offer.take().is_some() {
            return;
        }
        self.offer = Some(founding::Offer::new(self.world.seed));
    }

    /// **Put the chosen lineage in the ground.**
    ///
    /// Composed from public engine parts rather than a new one:
    /// `paint_nest_patch` puts a home down, `colony_stations` lays out where
    /// the founders stand — terrain-following, and derived from the body
    /// plan's own width, which is why a nine-cell stock does not get the
    /// two-cell ant's corridor — and `release_creature_specimen` places each
    /// founder with this lineage's traits stamped on it.
    ///
    /// **The species' own genome goes in untouched.** That is the whole of
    /// why this is safe: the trail-following circuit lives in `ant.ron`'s
    /// hidden layer, and a rolled genome would produce a colony that walks at
    /// random and takes an evening to tell apart from an unlucky one.
    ///
    /// **It lands at his feet on purpose, and that is a rule rather than a
    /// convenience.** A colony in held ground does not tick: creatures run on
    /// the active-site schedule, which `scheduler::step` gates on
    /// `time_runs_at`. So a colony has to be founded *inside* running time or
    /// it stands there as scenery — and the carried quickening is exactly the
    /// circle at his feet. The rule needs no code; it falls out of the gate.
    ///
    /// Returns how many animals were placed. **Zero is a real answer** and
    /// every route to it says which one it was: too little power, no ground,
    /// a species that is not loaded. A silent no-op is indistinguishable from
    /// a broken feature, which is how the whole of this milestone was once
    /// reported missing.
    pub fn commit_founding(&mut self) -> usize {
        let Some(offer) = &self.offer else {
            return 0;
        };
        let candidate = offer.picked().clone();
        let body = offer.body;
        let founders = offer.founders;
        let cost = candidate.cost(body, founders);
        if !self.unlimited && self.power < cost {
            self.note(format!("not enough power - that founding costs {cost:.0}"));
            return 0;
        }
        let Some(player) = &self.world.player else {
            return 0;
        };
        let (x, y) = player.center();
        let species = founding::STOCKS[body.min(founding::STOCKS.len() - 1)].species;
        let Some(species_id) = self.world.species.id_of(species) else {
            self.note(format!("{species} is not loaded"));
            return 0;
        };
        let Some(def) = self.world.species.get(species_id).creature.clone() else {
            self.note(format!("{species} is not an animal"));
            return 0;
        };
        let genome = self.world.species.get(species_id).genome.clone();
        // **The stock's own traits, moved by the roll.** Clamped to the
        // slots' shared domain rather than trusted: a delta that pushed a
        // baseline past ±1 would be read by `ratio_factor_reach` as an
        // allele no birth could ever produce.
        let mut traits = def.traits;
        for (t, d) in traits.iter_mut().zip(candidate.deltas.iter()) {
            *t = (*t + *d).clamp(-1.0, 1.0);
        }
        if !def.nest.is_empty() {
            if self.world.materials.id_of(&def.nest).is_none() {
                self.note(format!("{species} wants a nest of {} and there is none", def.nest));
                return 0;
            }
            self.world.paint_nest_patch(x, y);
        }
        // **One colony per founding**, exactly as `found_colony_of` does it:
        // the first founder that fits claims the label and every later one
        // joins it, so a founding in which nothing fits claims nothing.
        let mut colony: Option<u32> = None;
        let mut placed = 0;
        // See `found_colony`: the refusal counter either side of the loop is
        // the only thing that separates "every station is occupied" from
        // "every station reached the allocator and was turned away". §Z21
        // measured those two as *the same picture* -- 63 stations offered
        // against 31 with the ground rule repaired, and the identical 2
        // animals placed, because the wall was downstream of the ground.
        let refused_before = self.world.organisms_refused();
        let stations = self.world.colony_stations(x, y, species_id, founders);
        for &(cx, cy) in &stations {
            let Some(organism) = creature::release_creature_specimen(&mut self.world, cx, cy, species, genome.clone(), traits, colony) else {
                continue;
            };
            if colony.is_none() {
                colony = self.world.organism(organism).map(|s| s.colony);
            }
            placed += 1;
        }
        // **Charged for what landed, not for what you asked for**, and the
        // difference is not small: a headless founding of twelve `hopper` on
        // rolling ground seated **3**, because `colony_stations` lays out a
        // corridor and a station that does not fit is declined. Paying 224
        // for three animals is the kind of unfairness a player notices at
        // once and cannot see the cause of. Affordability was checked against
        // the full ask above, so a founding can never overdraw.
        let paid = candidate.cost(body, placed as i32);
        println!("druid: founded {placed} {species} at {x},{y} — asked for {founders} at {cost:.0}, paid {paid:.0}");
        // **Spend it where you can see it go.** The pool coming off the meter
        // is a number changing in the corner; this is the same event as
        // something leaving the caster and arriving in the ground, and it is
        // the drain's own flow with `outward` set — *"if an event produces no
        // visible consequence it is not finished regardless of what the
        // simulation believes"*. Capped so a twenty-four founder colony is a
        // heavier flow than a four without being a wall of motes.
        for &(cx, cy) in stations.iter().take(FOUNDING_STREAMS) {
            self.draws.push(Draw { from: (cx, cy), outward: true, age: 0, amount: paid / placed.max(1) as f32 });
        }
        if placed == 0 {
            // **Three refusals wearing one message was the bug.** `C` at the
            // spawn refuses on both `dead` and `grown` starts and takes on
            // `bare`, and "no ground here" is unactionable when you are
            // plainly standing on ground: what is actually true is that every
            // station is *occupied*, because a grown wood fills the surface
            // with plant cells and a station that does not fit is declined.
            // Measured 2026-09-14 -- and it is the first thing a player
            // presses, so a refusal that does not say what to do about it is
            // the whole feature reading as broken.
            // **Three refusals, three messages, and the third one is not a
            // property of the ground at all.** "the ground here is full" is
            // as wrong at the slot ceiling as "no ground here" was, and it
            // sends the player walking somewhere else to get the same
            // result.
            let no_slots = self.world.organisms_refused() > refused_before;
            self.note(refusal_note(stations.len(), no_slots));
            println!(
                "druid: founding REFUSED — {} stations offered, 0 took{}",
                stations.len(),
                if no_slots { " (out of organism slots)" } else { "" }
            );
            return 0;
        }
        if !self.unlimited {
            self.power -= paid;
        }
        self.animals += placed;
        self.note(format!("{placed} {} founded for {paid:.0}", founding::STOCKS[body.min(founding::STOCKS.len() - 1)].name.to_lowercase()));
        // **Committing is what costs you the other two.** Walking away does
        // not reroll, and neither does a refusal above — only a founding that
        // actually happened.
        if let Some(offer) = &mut self.offer {
            offer.reroll();
        }
        self.offer = None;
        placed
    }

    /// **Place a standing quickening where he is standing.**
    ///
    /// The economy's verb, as against the carried circle, which is free and
    /// is what he *is*. A standing one runs while he is elsewhere, which is
    /// the whole endgame — and is why it costs.
    ///
    /// **It lurches.** `wake_region` pulls every scheduled site inside the
    /// new circle forward to now, so the ground starts at once rather than
    /// trickling into life over `HELD_RECHECK`. Returns the number of sites
    /// woken, because a placement that woke nothing and one that woke a wood
    /// look identical for the first second.
    pub fn place_quickening(&mut self) -> Option<usize> {
        let Some(player) = &self.world.player else {
            return None;
        };
        let (x, y) = player.center();
        let r = self.place_radius;
        self.world.quickenings.push(crate::sim::world::Quickening::at(x, y, r));
        let woken = self.world.wake_region(x, y, r);
        println!("druid: quickening at {x},{y} r{r} — woke {woken} sites");
        self.note(format!("circle placed r{r} - woke {woken} sites"));
        Some(woken)
    }

    /// Take back the standing quickening nearest the player, refunding
    /// nothing. The playtest counterpart of placing one.
    pub fn lift_quickening(&mut self) -> bool {
        let Some(player) = &self.world.player else {
            return false;
        };
        let (px, py) = player.center();
        let nearest = self
            .world
            .quickenings
            .iter()
            .enumerate()
            .min_by_key(|(_, q)| ((q.x - px) as i64).pow(2) + ((q.y - py) as i64).pow(2))
            .map(|(i, _)| i);
        match nearest {
            Some(i) => {
                let q = self.world.quickenings.remove(i);
                println!("druid: lifted the quickening at {},{}", q.x, q.y);
                self.note("circle lifted");
                true
            }
            None => {
                self.note("no circle to lift");
                false
            }
        }
    }

    /// **The speed dial: run the world again, for as many ticks as are paid
    /// for.**
    ///
    /// Owner's ask, 2026-09-13: *"You should be able to set the speed of the
    /// bubble."*
    ///
    /// **It needs no regional driver, and that is the whole reason it is
    /// cheap.** An extra `frame::step` on a *held* world already does work
    /// only inside the circles, because the held gate stops everything else —
    /// so "run the circles again" is spelled "run the world again".
    ///
    /// **The player is taken out of the world for the extra passes, and that
    /// is deliberate twice over.** He is always 1x — the concept is explicit:
    /// *he walks through his own bubble and watches it race around him* — and
    /// `frame::step` recomputes `World::carried` from him every call, so
    /// leaving him in would both move him at 8x and drag a fast carried
    /// circle around with him. `player::step` returns immediately when there
    /// is no player, so removing him is also what keeps the carried circle
    /// out of these passes, which is what makes it free.
    ///
    /// See [`Druid::speed`] for why this is one dial rather than one per
    /// circle, and [`Druid::ticks`] for the clock this does *not* advance.
    fn step_extra_ticks(&mut self) {
        if self.speed <= 1 || self.world.quickenings.is_empty() {
            return;
        }
        let held_player = self.world.player.take();
        for _ in 1..self.speed {
            frame::step(
                &mut self.world,
                &mut self.particles,
                &mut self.blasts,
                player::PlayerInput::default(),
                &self.player_tuning,
            );
        }
        self.world.player = held_player;
    }

    /// **Income and drain, and what the pool does about them.**
    ///
    /// One walk over the organisms rather than one per circle: there are
    /// thousands of them and a per-circle walk would be quadratic in the
    /// thing the player is encouraged to accumulate.
    fn step_economy(&mut self) {
        if !self.ticks.is_multiple_of(ECONOMY_INTERVAL) {
            return;
        }
        let seconds = ECONOMY_INTERVAL as f32 / 60.0;

        let mut animals_running = 0.0f32;
        let mut animals_alive = 0usize;
        let mut plants_in_circles = 0.0f32;
        // Accumulated here rather than written straight into `self.seed_growth`
        // because the walk holds `self.world` borrowed; folded in below.
        let mut gathered_by_kind = vec![0.0f32; self.seed_kinds.len()];
        // Rebuilt rather than updated in place: organism slots are reused, so
        // an entry left behind by a dead animal would be inherited by
        // whatever is allocated its slot next.
        let mut fresh: std::collections::HashMap<OrganismId, f32> = std::collections::HashMap::with_capacity(self.reserves.len());
        for id in self.world.live_organism_ids() {
            let Some(state) = self.world.organism(id) else { continue };
            let Some((x, y)) = state.chain.first().copied().or_else(|| state.cells.keys().next().copied()) else {
                continue;
            };
            let creature = self.world.species.get(state.species).creature.is_some();
            if creature {
                animals_alive += 1;
                let held = self.reserves.get(&id).copied().unwrap_or(0.0);
                // **Anywhere time runs, carried circle included.** A colony
                // under his feet charges without costing, which is what makes
                // the carried circle worth walking somewhere with.
                if self.world.time_runs_at(x, y) {
                    animals_running += 1.0;
                    fresh.insert(id, (held + RESERVE_PER_SECOND * seconds).min(RESERVE_CAP));
                } else {
                    // A frozen animal keeps what it had and earns nothing --
                    // the owner's question answered by the gate that already
                    // exists rather than by a rule of its own.
                    fresh.insert(id, held);
                }
            } else {
                // **Charged at the speed it is being run at.** A plant inside
                // a rate-4 circle is having four times as much life happen to
                // it as one in a rate-1 circle, and the concept's whole
                // economy is *the faster the more expensive* -- so the
                // multiplier is the honest price rather than a surcharge.
                // The fastest circle over a plant wins; two circles do not
                // stack, because the plant is only ticked once per pass.
                //
                // Charged only inside a *standing* circle: the carried one is
                // free, so walking through a wood does not bill you for it.
                if self.world.quickenings.iter().any(|q| q.contains(x, y)) {
                    plants_in_circles += 1.0;
                }
                // **And the pouch fills from the wood she is standing in.**
                // The carried circle, not a standing one: gathering is
                // presence, so the way to be paid in seed is to walk your own
                // wood. A standing quickening left running over a wood while
                // she is on the other side of the map pays nothing, which is
                // what stops the supply drifting back to unlimited.
                //
                // `SEED_FROM_CELLS` is the middle the ethos asks for — a
                // sprout sown a minute ago pays nothing and a grown tree
                // pays, and the gap between them is time she spent on it.
                // The rule itself is [`seed_credit`], which is where it can
                // be asked questions without a grown world.
                let name = &self.world.species.get(state.species).name;
                if let Some(k) = seed_credit(&self.seed_kinds, self.world.carried, name, state.cells.len(), x, y) {
                    gathered_by_kind[k] += SEED_PER_PLANT_SECOND * seconds;
                }
            }
        }

        self.reserves = fresh;
        self.take_gathered_seed(&gathered_by_kind);
        // **Income is what you *drew*, per second, not what is out there.**
        // The readout has to answer "am I winning", and with an absorb-driven
        // economy the honest answer is a rate over the recent past rather
        // than a census of stored charge that may never be collected.
        let drawn: f32 = self.draws.iter().filter(|d| d.age == 0).map(|d| d.amount).sum();
        self.income = drawn / seconds;
        // **Multiplied by the dial, both terms.** A plant in a circle run at
        // 8x is having eight times as much life happen to it, and an empty
        // circle at 8x still costs eight times a slow one -- which is what
        // stops the dial being free until something grows under it. This is
        // the concept's *the faster the more expensive*, and it is the only
        // thing standing between the player and leaving it at maximum.
            self.drain = drain_for(self.speed, self.world.quickenings.len() as f32 + carried_cost(&self.world), plants_in_circles);
        // The readout's two animal numbers, taken from the walk that was
        // happening anyway rather than from a second census per frame.
        self.animals = animals_alive;
        self.animals_awake = animals_running as usize;

        if self.unlimited {
            return;
        }
        self.power += (self.income - self.drain) * seconds;
        if self.power < 0.0 {
            // **The circle closes over your own wood.** Not a game-over and
            // not a silent stall: the newest standing quickening is the one
            // that goes, so running out reads as the map contracting rather
            // than as nothing happening.
            self.power = 0.0;
            if self.world.quickenings.pop().is_some() {
                println!("druid: out of power — a standing quickening set");
                self.note("out of power - a standing circle closed");
            }
        }
    }

    /// **Turn a pass's worth of gathered fractions into seeds in the pouch.**
    ///
    /// Split out of [`Druid::step_economy`] only because the walk there holds
    /// `self.world` borrowed; the rule is the interesting part and it is here.
    ///
    /// **Whole seeds are announced and fractions are not.** Gathering pays
    /// roughly one seed every ten seconds under a wood, which is far too slow
    /// to read as an event unless the moment it lands is said out loud — and
    /// a bar creeping in the corner is precisely the readout this game's own
    /// economy doc rejects. A seed arriving is a thing that happened.
    ///
    /// The cap is per kind and the remainder is **dropped, not banked**, once
    /// a kind is full: banking it would let a player park in a mature wood
    /// and cash out the moment he sowed one, which is the unlimited supply
    /// wearing a delay.
    fn take_gathered_seed(&mut self, gathered_by_kind: &[f32]) {
        for (k, add) in gathered_by_kind.iter().enumerate() {
            if *add <= 0.0 {
                continue;
            }
            let (Some(growth), Some(held)) = (self.seed_growth.get_mut(k), self.seeds.get(k).copied()) else {
                continue;
            };
            if held >= SEED_CAP {
                *growth = 0.0;
                continue;
            }
            *growth += add;
            let whole = growth.floor();
            if whole < 1.0 {
                continue;
            }
            *growth -= whole;
            let taken = (whole as u32).min(SEED_CAP - held);
            self.seeds[k] = held + taken;
            self.gathered += taken as usize;
            let kind = self.seed_kinds[k].clone();
            let now = self.seeds[k];
            println!("druid: gathered {taken} {kind} seed — {now} in hand");
            self.note(format!("gathered {taken} {kind} seed - {now} in hand"));
        }
    }

    /// One tick.
    pub fn update(&mut self) {
        if self.paused {
            return;
        }
        self.ticks += 1;
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, self.player_input, &self.player_tuning);
        // **Consumed here, or a catch-up burst turns one press into five
        // jumps.** The edge is the caller's to set and this tick's to clear.
        self.player_input.jump_pressed = false;
        self.step_extra_ticks();

        // **Wake the ground he has walked onto, on a distance threshold.**
        // Every frame would be the unbounded heap rebuild `wake_region`'s own
        // doc refuses; never would leave newly-covered ground asleep for
        // `HELD_RECHECK`, which is a visible lag on the thing he does most.
        if let Some(carried) = self.world.carried {
            let far = self.last_wake.is_none_or(|(lx, ly)| {
                let (dx, dy) = (carried.x - lx, carried.y - ly);
                dx * dx + dy * dy >= CARRY_WAKE_STEP * CARRY_WAKE_STEP
            });
            if far {
                self.last_wake = Some((carried.x, carried.y));
                self.world.wake_region(carried.x, carried.y, carried.r);
            }
        }

        // Energy in flight ages toward the player and lands. Kept before the
        // economy so a draw made this tick is still `age == 0` when the
        // economy reads it as this pass's income.
        for d in &mut self.draws {
            d.age += 1;
        }
        self.draws.retain(|d| d.age <= DRAW_FRAMES);

        self.step_economy();
    }

    /// One drawn frame.
    pub fn draw(&mut self, frame_buf: &mut [u8], viewport: (u32, u32), force_full: bool) {
        // The view follows him, outside the tick loop: the camera is view
        // state, so running it several times in a catch-up frame would move
        // it several times for one drawn picture.
        if let Some(player) = &self.world.player {
            self.renderer.follow(player.center(), viewport, self.world.bounds());
        }
        // **The interface is built before the world is drawn, because
        // whether it *changed* decides whether the world has to be repainted
        // underneath it.** Nothing here has a footprint the renderer tracks,
        // so a readout that shrinks by a digit, a message that expires, or a
        // ring that moved one pixel would otherwise stay burned into settled
        // ground with no error anywhere. Forcing a full redraw every frame is
        // what the lab does and is wrong here: this game's premise is a world
        // standing still, which is exactly where the render skip earns its
        // keep. See `hud::Interface`.
        let ui = hud::Interface::build(self, viewport);
        let ui_changed = self.last_ui.as_ref() != Some(&ui);
        self.last_ui = Some(ui.clone());

        // **Taken every frame, without exception.** `take_touched_chunks` is
        // a `mem::take`: a frame that skips it drops those chunks for good and
        // they redraw as stale pixels with no error anywhere.
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(&self.world, &self.particles, &touched, frame_buf, viewport, force_full || ui_changed);
        ui.draw(frame_buf, viewport);
    }
}

/// **A dry standing spot near the middle of the world.**
///
/// Generated terrain puts the surface wherever it likes, and the sandbox
/// never auto-spawns — it summons the gnome wherever the player clicked — so
/// there is no existing pattern for this, and a spawn at a fixed height drops
/// him into rock or out of the sky depending on the seed.
///
/// **Searching outward from the middle is not tidiness.** The first version
/// took the middle column and nothing else, and the first world it generated
/// put a pond exactly there: `surface_at` walks to the first *Solid*, water is
/// a `Liquid`, so it walked straight through the pond and returned the rock
/// floor underneath it. The gnome spawned submerged. A column is only a spawn
/// if it is dry.
fn spawn_point(world: &World) -> Option<(i32, i32)> {
    let b = world.bounds()?;
    let mid = (b.min_x + b.max_x) / 2;
    // Alternating outward from the middle, so he starts as near the centre of
    // the world as the terrain allows rather than at whichever end is dry.
    (0..=(b.max_x - b.min_x) / 2)
        .flat_map(|d| [mid - d, mid + d])
        .filter(|&x| x >= b.min_x && x <= b.max_x)
        .find_map(|x| surface_at(world, x).map(|y| (x, y)))
}

/// The cell a body stands in at column `x`, or `None` if the column has no
/// ground or is under water.
fn surface_at(world: &World, x: i32) -> Option<i32> {
    let b = world.bounds()?;
    let mut last_empty = None;
    for y in b.min_y..=b.max_y {
        let cell = world.get(x, y);
        if cell.material == material::EMPTY {
            last_empty = Some(y);
            continue;
        }
        match world.materials.kind(cell.material) {
            material::MaterialKind::Solid => return last_empty,
            // Anything wet above the rock disqualifies the column outright,
            // rather than being skipped over to the floor beneath it.
            material::MaterialKind::Liquid => return None,
            _ => {}
        }
    }
    None
}

fn size_from_env() -> (u32, u32) {
    let Ok(v) = std::env::var(SIZE_ENV) else {
        return (WORLD_WIDTH, WORLD_HEIGHT);
    };
    let parsed = v.split_once(['x', 'X']).and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)));
    match parsed {
        Some((w, h)) if w > 0 && h > 0 => (w, h),
        _ => {
            eprintln!("druid: {SIZE_ENV}={v:?} is not WxH; using {WORLD_WIDTH}x{WORLD_HEIGHT}");
            (WORLD_WIDTH, WORLD_HEIGHT)
        }
    }
}

/// **What a widened carried circle costs, in standing-circle equivalents.**
///
/// **Zero at its base size, and that is load-bearing.** The carried circle is
/// free because it is *presence* — a colony under your feet charges without
/// billing you, which is the whole reason walking somewhere is worth doing.
/// But the owner asked to be able to widen it, and a free circle you can grow
/// to a standing circle's size is a standing circle you never have to place:
/// the placement economy would simply stop applying.
///
/// So the price is the **area you added**, not the area you have. Doubling
/// the radius covers four times the ground and costs three circles; leaving
/// it alone costs nothing, exactly as before.
fn carried_cost(world: &World) -> f32 {
    let Some(carried) = world.carried else {
        return 0.0;
    };
    let base = CARRIED_RADIUS.max(1) as f32;
    let ratio = carried.r as f32 / base;
    (ratio * ratio - 1.0).max(0.0)
}

/// **Does this plant pay the druid seed, and into which pouch?**
///
/// The whole gathering rule, as one function over plain values, so a guard
/// can ask it the four questions that matter without growing a wood first.
/// It was inline in [`Druid::step_economy`]'s organism walk to begin with,
/// and that walk needs a grown world — so the *"credited to the plant's own
/// kind"* claim, which is the one thing here a player would notice going
/// wrong, was not reachable by any test that ran in under a minute. Pulled
/// out for exactly that reason.
///
/// Four ways to answer `None`, and each is a rule rather than a guard clause:
/// a plant too small to have anything to give ([`SEED_FROM_CELLS`]); a plant
/// outside the circle she is *carrying*, which is what makes gathering
/// presence rather than ownership; no carried circle at all, so switching the
/// sphere off stops the pouch filling exactly as it stops everything else;
/// and a species that is not a kind she can sow, which simply pays nothing.
///
/// The name scan is a linear pass over seven strings, twice a second, per
/// plant already inside the circle. A map keyed by `SpeciesId` was the first
/// version and it is a second representation of `seed_kinds`' own ordering —
/// the side table [`Druid::seed_kinds`]' own doc refuses to keep.
fn seed_credit(kinds: &[String], carried: Option<crate::sim::world::Quickening>, name: &str, cells: usize, x: i32, y: i32) -> Option<usize> {
    if cells < SEED_FROM_CELLS {
        return None;
    }
    if !carried.is_some_and(|c| c.contains(x, y)) {
        return None;
    }
    kinds.iter().position(|k| k == name)
}

/// **What one second of running costs**, given the dial, how many standing
/// circles there are, and how many plants stand inside them.
///
/// A named function so the guard asserts the rule rather than a copy of it.
fn drain_for(speed: u32, circles: f32, plants_in_circles: f32) -> f32 {
    speed.max(1) as f32 * (DRAIN_PER_CIRCLE * circles + DRAIN_PER_PLANT * plants_in_circles)
}

/// **Can this game sow this species?** See the seed-kind list in
/// [`Druid::new`] for the measurement behind it and the two predicates that
/// are wrong.
fn is_sowable(species: &crate::sim::organism::Species) -> bool {
    species.cell_types().iter().any(|(ct, _)| *ct == crate::sim::organism::CellType::Seed) || species.name == MOSS
}

fn start_from_env() -> Start {
    let Ok(v) = std::env::var(START_ENV) else {
        return Start::default();
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "dead" => Start::Dead,
        "bare" => Start::Bare,
        "grown" | "alive" => Start::Grown,
        other => {
            eprintln!("druid: {START_ENV}={other:?} is not dead|bare|grown; using {}", Start::default().label());
            Start::default()
        }
    }
}

fn grow_from_env() -> u64 {
    match std::env::var(GROW_ENV) {
        Ok(v) => match v.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                eprintln!("druid: {GROW_ENV}={v:?} is not a number; using {GROW_FRAMES}");
                GROW_FRAMES
            }
        },
        Err(_) => GROW_FRAMES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A world out of organism slots says so, rather than blaming the
    /// ground.** §Z21's whole content: `found_colony_of` returns 0 for three
    /// unrelated reasons and the bar named the wrong one, confidently.
    ///
    /// **The `no_slots` arm is checked against a real ceiling rather than a
    /// hand-set flag**, because the claim being guarded is that the counter
    /// moves when the world is full -- a flag would guard the `if` and not
    /// the mechanism. Filling every slot costs ~0.6 s, measured, which is
    /// why it is done for real here.
    #[test]
    fn a_world_out_of_slots_says_so_instead_of_blaming_the_ground() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let species = w.species.id_of("moss").expect("moss is compiled in");

        // The control first: a world with slots free refuses nothing, so the
        // counter is known to be quiet when nothing is wrong. Without this
        // the assertion below cannot tell "the world is full" from "this
        // counter is always non-zero".
        let quiet = w.organisms_refused();
        let id = w.push_organism(species).expect("a slot is free in a fresh world");
        w.free_organism(id);
        assert_eq!(w.organisms_refused(), quiet, "an allocation that succeeds must not count as a refusal");

        // Now fill it. `push_organism` is the only allocator, so this is the
        // same wall a germination or a founding hits.
        while w.push_organism(species).is_some() {}
        let before = w.organisms_refused();
        assert!(w.push_organism(species).is_none(), "a full world must refuse, not wrap an index into the generation bits");
        assert!(w.organisms_refused() > before, "a refused birth must be counted -- it is the only signal the message can read");

        // And the wording follows the counter, not the ground. Both of the
        // ground wordings tell the player to move, which is useless here.
        let full = refusal_note(63, true);
        assert!(full.contains("world is full"), "a slot refusal must name the world being full, not the ground: {full:?}");
        assert!(!full.contains("ground"), "a slot refusal must not mention ground at all -- that is the wrong cause: {full:?}");
    }

    /// The other two arms still say what they used to, so the fix names a
    /// third case rather than renaming the two that were already right.
    #[test]
    fn the_two_ground_refusals_are_unchanged_and_distinct() {
        let none = refusal_note(0, false);
        let full = refusal_note(63, false);
        assert!(none.contains("no ground here"), "{none:?}");
        assert!(full.contains("the ground here is full"), "{full:?}");
        assert_ne!(none, full, "standing off ground and standing on crowded ground are different things to do about it");
        assert_ne!(full, refusal_note(63, true), "the same station count must read differently when the refusal was the allocator");
    }

    /// **A trail he lays has a slope, and the slope points at him.**
    ///
    /// This is the assumption the whole of [`Druid::lay_trail`] rests on, and
    /// it is not obvious enough to leave unguarded: an ant reads
    /// `PheroBAlong`, the *gradient* along its heading, so a trail of uniform
    /// strength is a trail nothing can follow. What supplies the slope is
    /// `DECAY_RHO` — every mark starts fading the moment it is laid, so the
    /// newest cell on the route is the strongest.
    ///
    /// **The control is the same walk with the plane never stepped**, which
    /// is the world in which the decay does not happen: there the two ends
    /// read *equal*, so this guard is known to be measuring the decay rather
    /// than something about the deposit. Without it, a deposit that happened
    /// to write more at the far end would pass and mean nothing.
    #[test]
    fn a_laid_trail_slopes_toward_the_newest_end() {
        use crate::sim::pheromone::{Channel, DEPOSIT};
        let walk: Vec<i32> = (40..70).collect();

        // The arm: lay along the row, letting the plane age between marks.
        let mut w = World::new(Rect::new(0, 0, 255, 127));
        for &x in &walk {
            w.deposit_pheromone(Channel::B, x, 64, DEPOSIT);
            for _ in 0..12 {
                w.frame += 1;
                w.step_pheromones();
            }
        }
        let (first, last) = (w.pheromone_at(Channel::B, walk[0], 64), w.pheromone_at(Channel::B, *walk.last().unwrap(), 64));
        assert!(
            last > first,
            "the newest end reads {last} against the oldest {first} -- a flat trail has no gradient, and `PheroBAlong` is a gradient, so nothing would follow it"
        );

        // The control: the identical walk with the plane frozen.
        let mut c = World::new(Rect::new(0, 0, 255, 127));
        for &x in &walk {
            c.deposit_pheromone(Channel::B, x, 64, DEPOSIT);
        }
        let (cf, cl) = (c.pheromone_at(Channel::B, walk[0], 64), c.pheromone_at(Channel::B, *walk.last().unwrap(), 64));
        assert_eq!(cf, cl, "with the plane never stepped the two ends must be equal ({cf} vs {cl}); if they are not, the slope above is not the decay and this guard is measuring the wrong thing");
    }

    /// **Widening the circle you carry is not free, and leaving it alone
    /// still is.**
    ///
    /// The exploit this guards: the carried circle costs nothing because it
    /// is presence, and the owner asked to be able to widen it. A free circle
    /// that can grow to a standing circle's size is a standing circle nobody
    /// ever has to place — the placement economy simply stops applying, and
    /// nothing else in the game would notice.
    #[test]
    fn a_widened_carried_circle_costs_and_an_untouched_one_does_not() {
        use crate::sim::world::Quickening;
        let mut w = World::new(Rect::new(0, 0, 63, 63));

        assert_eq!(carried_cost(&w), 0.0, "no carried circle is no cost");

        w.carried = Some(Quickening::at(10, 10, CARRIED_RADIUS));
        assert_eq!(carried_cost(&w), 0.0, "the circle you already are must stay free");

        // Twice the radius is four times the ground, so three circles' worth
        // of *added* reach.
        w.carried = Some(Quickening::at(10, 10, CARRIED_RADIUS * 2));
        assert!((carried_cost(&w) - 3.0).abs() < 1e-4, "double the radius should cost 3, not {}", carried_cost(&w));

        // Monotone all the way up, or some middle setting is a free lunch.
        let mut last = 0.0;
        for r in CARRIED_RADIUS..=CARRIED_RADIUS_MAX {
            w.carried = Some(Quickening::at(10, 10, r));
            let c = carried_cost(&w);
            assert!(c >= last, "cost fell from {last} to {c} at r{r}");
            last = c;
        }
        assert!(last > 0.0, "the widest carried circle must cost something");
    }

    /// **Off is a state the dial cannot reach, and the flag is what reaches
    /// it.**
    ///
    /// Asserted on `frame::step`'s own rule rather than on the flag, because
    /// the flag is trivially true and the interesting claim is that *nothing
    /// else* produces a player with no circle. Two things this is watching
    /// for, both of which the engine documents as deliberate and both of
    /// which a later "simplification" would undo: `carried_radius = 0` is
    /// read as the default size, and `Quickening::contains` is `<=`, so even
    /// a genuinely zero radius still runs time for the cell underfoot.
    #[test]
    fn the_carried_circle_turns_off_by_the_flag_and_not_by_the_dial() {
        use crate::sim::world::CARRIED_RADIUS;

        let mut w = World::new(Rect::new(0, 0, 127, 127));
        w.held = true;
        w.player = Some(player::Player::at_scaled(64, 64, w.cell_scale()));

        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::with_tuning(explosion::Tuning::load());
        let tuning = player::Tuning::load();
        // Where he *is* after the step, not where he was placed: with no
        // ground under him he falls, and an assertion pinned to 64,64 would
        // be asking about a cell he has left.
        let mut step = |w: &mut World| -> (i32, i32) {
            frame::step(w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
            w.player.as_ref().expect("the player must survive a step").center()
        };

        let at = step(&mut w);
        assert!(w.carried.is_some(), "a held world with a player must carry a circle");
        assert!(w.time_runs_at(at.0, at.1), "time must run where he stands");

        // **The dial at zero is NOT off** -- it is the default radius, and
        // the engine's own doc calls that deliberate.
        w.carried_radius = 0;
        let at = step(&mut w);
        let carried = w.carried.expect("a zero radius is the default size, never no circle");
        assert_eq!(carried.r, CARRIED_RADIUS, "a zero radius must read as the default, not as 0");
        assert!(w.time_runs_at(at.0, at.1), "a zero radius must still run time where he stands");

        // The flag is off, and off means no circle at all.
        w.carried_radius = CARRIED_RADIUS;
        w.carried_off = true;
        let at = step(&mut w);
        assert!(w.carried.is_none(), "carried_off must leave no circle");
        assert!(!w.time_runs_at(at.0, at.1), "with the circle off, time must not run where he stands");

        // ...and back, in the same world, because a one-way switch would pass
        // every assertion above and be useless.
        w.carried_off = false;
        let at = step(&mut w);
        assert!(w.carried.is_some(), "clearing carried_off must bring the circle back");
        assert!(w.time_runs_at(at.0, at.1), "time must run again where he stands");
    }

    /// **Sowing spends a seed, an empty pouch refuses, and gathering is what
    /// puts one back.**
    ///
    /// The three halves of the mechanic that replaced an unlimited supply.
    /// Asserted on `Druid`'s own state rather than on a generated world:
    /// `Druid::new` generates 2560x960 and grows it, which is a minute of
    /// wall clock, and the rules here are about the pouch and not about the
    /// ground.
    ///
    /// **Every assertion below was watched failing** against the code as it
    /// stood before this change -- an unlimited supply passes none of them,
    /// which is the point.
    #[test]
    fn the_pouch_is_spent_by_sowing_and_refilled_by_gathering() {
        // `take_gathered_seed` is the rule; the walk that feeds it needs a
        // grown world and this does not.
        let mut g = Druid::bare_for_test();
        g.seed_kinds = vec!["tree".to_string(), "grass".to_string()];
        g.seeds = vec![2, 0];
        g.seed_growth = vec![0.0; 2];

        assert_eq!(g.seeds_in_hand(), 2, "the selected kind starts at 2");

        // A whole seed's worth arrives as one seed, and a fraction does not.
        g.take_gathered_seed(&[0.4, 0.0]);
        assert_eq!(g.seeds[0], 2, "four tenths of a seed is not a seed");
        g.take_gathered_seed(&[0.7, 0.0]);
        assert_eq!(g.seeds[0], 3, "the remainder must carry across passes");
        assert_eq!(g.gathered, 1, "the counter says whether it fired at all");

        // A kind she is not standing in gets nothing -- gathering is credited
        // to the plant's own kind, so a grass pouch must not fill in a wood.
        assert_eq!(g.seeds[1], 0, "no grass stood in the circle, so no grass seed");

        // The cap holds, and the overflow is dropped rather than banked.
        g.seeds[0] = SEED_CAP;
        g.take_gathered_seed(&[5.0, 0.0]);
        assert_eq!(g.seeds[0], SEED_CAP, "the pouch must not exceed its cap");
        assert_eq!(g.seed_growth[0], 0.0, "a full pouch banks nothing for later");

        // **Sowing spends one, and this is the half the first version of this
        // guard was blind to.** Written without it, every assertion above
        // passed with the decrement deleted outright -- the test never
        // reached a *successful* sowing, so the central claim of the whole
        // change was unguarded. Put the deletion back now and this goes red.
        // **He is moved between sowings, and that is not a convenience.** The
        // first version stood still and the second sowing was refused for
        // *"the cell is not empty"* -- a seed already lying there -- so the
        // guard would have been measuring the ground rather than the pouch.
        let scale = g.world.cell_scale();
        let sow_at = |g: &mut Druid, x: i32| {
            g.world.player = Some(player::Player::at_scaled(x, 47, scale));
            g.plant_seed()
        };

        g.seeds[0] = 2;
        g.seed_kind = 0;
        assert!(sow_at(&mut g, 10), "a tree seed into empty ground must go in");
        assert_eq!(g.seeds[0], 1, "sowing must spend a seed");
        assert_eq!(g.sown, 1);
        assert!(sow_at(&mut g, 14), "the second seed must go in too");
        assert_eq!(g.seeds[0], 0, "the pouch must reach zero");

        // And then the refusal, on the kind that is now empty -- at a fresh
        // column, so "no room" cannot be what is being read.
        assert!(!sow_at(&mut g, 18), "sowing with an empty pouch must refuse");
        assert!(g.message().is_some_and(|m| m.contains("no tree seed left")), "the refusal must be said out loud, got {:?}", g.message());
        assert_eq!(g.sown, 2, "a refused sowing is not a sowing");

        // ...and the other kind is untouched by any of it -- a single shared
        // counter would have been spent by the two sowings above.
        assert_eq!(g.seeds[1], 0, "grass started empty and nothing here was grass");
        g.seeds[1] = 3;
        g.seed_kind = 1;
        assert!(sow_at(&mut g, 22), "grass with 3 in hand must sow");
        assert_eq!(g.seeds[1], 2, "the grass pouch is its own");
        assert_eq!(g.seeds[0], 0, "sowing grass must not touch the tree pouch");
    }

    /// **What pays her seed, and into which pouch.**
    ///
    /// Separate from the pouch guard above because it is a different claim:
    /// that one is about arithmetic on a count, this is about *which plant
    /// counts*. It exists in this shape because the first version of the
    /// pouch guard could not see it at all — the crediting was inline in the
    /// organism walk, which needs a grown world, and deleting *"the plant's
    /// own kind"* in favour of *"the selected kind"* left every assertion
    /// green. Put that substitution back now and `an oak wood` goes red.
    #[test]
    fn seed_is_credited_to_the_plant_standing_in_the_circle_she_carries() {
        use crate::sim::world::{Quickening, CARRIED_RADIUS};
        let kinds = vec!["tree".to_string(), "grass".to_string()];
        let here = Some(Quickening::at(100, 100, CARRIED_RADIUS));
        let big = SEED_FROM_CELLS;

        // The positive control first, or every `None` below is unreadable:
        // a grown tree under her feet pays into the *tree* pouch, which is
        // index 0 and not the selected kind, whatever that happens to be.
        assert_eq!(seed_credit(&kinds, here, "tree", big, 100, 100), Some(0), "a grown tree in the circle must pay");
        assert_eq!(seed_credit(&kinds, here, "grass", big, 100, 100), Some(1), "...and grass into the grass pouch");

        // Too small: the middle the ethos asks for.
        assert_eq!(seed_credit(&kinds, here, "tree", big - 1, 100, 100), None, "one cell under the bar pays nothing");
        assert_eq!(seed_credit(&kinds, here, "tree", 1, 100, 100), None, "a seedling pays nothing");

        // Outside the circle she carries -- this is what makes gathering
        // presence. A wood on the far side of the map pays nothing however
        // many standing quickenings are running over it.
        assert_eq!(seed_credit(&kinds, here, "tree", big, 100 + CARRIED_RADIUS + 1, 100), None, "a tree outside the circle pays nothing");

        // No circle at all -- switching the sphere off stops the pouch
        // filling, the same way it stops everything else.
        assert_eq!(seed_credit(&kinds, None, "tree", big, 100, 100), None, "with the circle off nothing pays");

        // A species that is not a kind she can carry.
        assert_eq!(seed_credit(&kinds, here, "ant", big, 100, 100), None, "an animal is not a seed kind");
        assert_eq!(seed_credit(&kinds, here, "conifer", big, 100, 100), None, "a kind with no pouch slot pays nothing");
    }

    /// **The dial is priced, and priced linearly.**
    ///
    /// The one thing standing between the player and leaving the speed at
    /// maximum for ever, so if it silently stopped scaling the whole mechanic
    /// would become a free button. Asserted on the arithmetic rather than on
    /// a played world, because it *is* arithmetic — and asserted at both ends
    /// (an empty circle and a full one), since an early version multiplied
    /// only the per-plant term and a bare fast circle cost nothing.
    #[test]
    fn the_speed_dial_is_priced_linearly_at_both_ends() {
        // An empty circle: nothing growing, so only the per-circle term.
        let empty = |speed: u32| drain_for(speed, 1.0, 0.0);
        assert!(empty(1) > 0.0, "a standing circle must cost something even empty");
        for speed in SPEED_MIN..=SPEED_MAX {
            let want = empty(1) * speed as f32;
            assert!((empty(speed) - want).abs() < 1e-4, "an empty circle at x{speed} costs {}, not {want}", empty(speed));
        }

        // ...and with a wood in it, where the per-plant term dominates.
        let wood = |speed: u32| drain_for(speed, 1.0, 500.0);
        assert!(wood(1) > empty(1), "plants inside a circle must add to its cost");
        for speed in SPEED_MIN..=SPEED_MAX {
            let want = wood(1) * speed as f32;
            assert!((wood(speed) - want).abs() < 1e-3, "a wood at x{speed} costs {}, not {want}", wood(speed));
        }

        // The dial cannot be turned to free, and zero is not a discount.
        assert_eq!(drain_for(0, 1.0, 0.0), drain_for(1, 1.0, 0.0), "speed 0 must be priced as real time, not as nothing");
    }

    /// **The seed list is plants, and nothing but plants.**
    ///
    /// The guard for a predicate this module got wrong **twice**, each time
    /// plausibly: `creature.is_none()` admits the worm, which is an animal
    /// with no `creature` block because `creature.rs` drives it by species
    /// *name*; `has_economy()` drops moss, which declares no
    /// `Photosynthesize` at all. Both compile, both read correctly, and both
    /// put the wrong thing in the player's hand.
    ///
    /// Asserted against the **whole shipped species set** rather than a
    /// sample, and in both directions — every plant present, every animal
    /// absent — so adding a species file to `assets/species/` and forgetting
    /// this fails here rather than in somebody's game.
    #[test]
    fn the_seed_kinds_are_every_plant_and_no_animal() {
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let _ = w.species.reload(organism::ASSET_DIR);
        assert!(w.species.len() > 10, "the species set did not load; this guard would pass on nothing");

        let sowable: Vec<&str> = (0..w.species.len())
            .map(|i| w.species.get(organism::SpeciesId(i as u16)))
            .filter(|sp| is_sowable(sp))
            .map(|sp| sp.name.as_str())
            .collect();

        for plant in ["moss", "tree", "conifer", "shrub", "creeper", "grass", "herb", "scrambler"] {
            assert!(sowable.contains(&plant), "{plant} is a plant the player should be able to sow, and it is not in {sowable:?}");
        }
        // The worm is the one that caught this: an animal with no `creature`
        // block at all.
        for animal in ["worm", "ant", "beetle", "hopper", "flitter", "longant", "ancestor"] {
            assert!(!sowable.contains(&animal), "{animal} is an animal and must not be sowable, but it is in {sowable:?}");
        }
    }

    /// **A spawn point has to be on the ground**, which is the one thing this
    /// game needs that the sandbox never had to solve — it summons the gnome
    /// where the player clicked.
    ///
    /// Built rather than generated: a hand-made column is the case whose
    /// answer is known, which is what makes this a control rather than a
    /// restatement of whatever the terrain happened to do.
    #[test]
    fn the_spawn_point_is_the_cell_above_the_ground() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        for y in 40..64 {
            w.set(10, y, crate::sim::cell::Cell::new(material::STONE, 0));
        }
        assert_eq!(surface_at(&w, 10), Some(39), "the standing cell is the last empty one above the rock");
        assert_eq!(surface_at(&w, 11), None, "a column with no ground in it has no surface");

        // **The pond case, which the first generated world actually hit.**
        // Water is a `Liquid`, so a walk that only stops at `Solid` goes
        // straight through it and reports the rock floor underneath -- a
        // spawn point at the bottom of a pond.
        let water = w.materials.id_of("water").expect("water is compiled in");
        for y in 44..48 {
            w.set(20, y, crate::sim::cell::Cell::new(water, 0));
        }
        for y in 48..64 {
            w.set(20, y, crate::sim::cell::Cell::new(material::STONE, 0));
        }
        assert_eq!(surface_at(&w, 20), None, "a flooded column is not somewhere to stand");
        // And the search finds dry land rather than giving up at the middle.
        assert!(spawn_point(&w).is_some_and(|(x, _)| x != 20), "the search must walk past a flooded column");
    }

    /// The size knob parses, and a malformed value falls back rather than
    /// panicking a game on startup.
    #[test]
    fn the_size_knob_falls_back_rather_than_failing() {
        // Not a `std::env::set_var` test: env is process-global and the suite
        // is threaded, so this exercises the parse directly.
        let parse = |v: &str| v.split_once(['x', 'X']).and_then(|(w, h)| Some((w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?)));
        assert_eq!(parse("2560x960"), Some((2560, 960)));
        assert_eq!(parse("512X320"), Some((512, 320)));
        assert_eq!(parse("nonsense"), None);
        assert_eq!(parse("2560x"), None);
    }
}
