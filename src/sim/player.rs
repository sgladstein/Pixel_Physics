//! M9 — the playable character: a gnome who runs, jumps, and (phase 2
//! onward) digs, plants and throws.
//!
//! **Off-grid, like `rigid::ChunkBody`, and a ghost.** The character writes
//! no cells and is invisible to the CA sweep, which is the whole
//! performance story: an idle gnome on a settled world wakes zero chunks
//! and costs the dirty-rect render skip nothing. The trade is that sand
//! and bodies fall *through* the player's rectangle; the depenetration
//! pass below is the corrective, and `buried` is what it reports when
//! there is no way out. Deliberately not a `ChunkBody`: bodies settle
//! back into the grid after three stalled frames, rotate, and shove
//! powder aside, and a character must do none of those — the overlap
//! left after removing all three is a plain AABB sweep, written here
//! rather than factored out of `rigid.rs` where generalising the collider
//! could perturb settled collapse behaviour.
//!
//! Stepped in `App::update`'s serial body slot (right after
//! `rigid::step_chunk_bodies`), for the same write-disjointness reason
//! that phase exists at all. Everything here is input-driven — no RNG, no
//! wall clock — so a run replays from a `PlayerInput` sequence alone,
//! which is what the determinism requirement means for an entity that
//! only exists when a player summons it.

use super::material::MaterialKind;
use super::world::World;

/// Character extent in cells. 7x14 on a 512x320 world puts him at about a
/// twenty-third of the world's height — gnome-scale beside trees that are
/// tens of cells, and large enough to carry a silhouette with a face,
/// arms and legs rather than three coloured bands. He fits upright
/// through the 15-cell bore a radius-7 `rigid::mine` carves;
/// `dig_radius` and this are a pair, and a bore narrower than he is tall
/// is a tunnel he has to be shoved along by depenetration.
///
/// Was 3x6, then 5x10, now this — grown twice on playtest notes ("can we
/// make the gnome a little bigger", then "a little bigger still").
/// Everything proportional scaled with it rather than only the extent:
/// `step_up` and `wade_rows` are fractions of his height, not absolute
/// distances, so leaving them alone would make a bigger gnome trip on
/// smaller things and wade shallower. The movement presets scale too —
/// see `MOVEMENT_FEELS`.
pub const PLAYER_WIDTH: i32 = 7;
pub const PLAYER_HEIGHT: i32 = 14;

/// How far the depenetration pass will push to free an invaded rectangle
/// before giving up and declaring the player buried. Small on purpose: a
/// large push is a teleport, and popping through a thin ceiling reads far
/// worse than being stuck under it.
const DEPENETRATE_REACH: i32 = 4;

/// How far past the bore a dig may throw its spoil, and how far a *buried*
/// dig may throw it. Constants rather than tunables on purpose: neither is
/// a feel knob the panel should sweep — the first is the shove distance
/// `rigid::DISPLACE_SEARCH` already fixes at 4 for bodies, and the second
/// is "far enough to reach the surface of a pile that could plausibly have
/// buried you", which is a reachability question, not a taste one. See
/// `dig` for why they differ.
const SPOIL_THROW: i32 = 4;
const BURIED_THROW: i32 = 16;

/// How far below the feet a falling chunk body still counts as the floor
/// he is standing on. Matches `rigid`'s per-axis fall clamp of 6, because
/// that is exactly how far a platform can have moved between the body step
/// and the player step — any smaller and a fast-falling slab drops out
/// from under a passenger it should be carrying.
const PLATFORM_STICK: i32 = 6;

/// How much of him can reach a handhold: the top half, head down to about
/// the waist.
///
/// See the grip test in `step` for why it is not the whole rectangle. The
/// short version is that roots are climbable, he wades four rows into soft
/// ground, and a wood is full of roots — so "any overlap" would have turned
/// every jump near a tree into a grab at boot height.
const GRIP_ROWS: i32 = PLAYER_HEIGHT / 2;

/// How long the swing pose stays up after a blow.
///
/// Half the default `dig_cooldown`, so held digging alternates swing and
/// stance rather than sticking in one or flickering between them — the
/// rhythm of blows the cooldown already produces, made visible.
const SWING_FRAMES: u8 = 4;

/// Everything about how the character *feels*, live-tunable under the
/// panel's PLAYER group and persisted to `assets/player.ron`. The same
/// shape as `explosion::Tuning` and for the same reason: these numbers
/// are judged by playing, not by argument, so they must be sweepable with
/// the world visible.
///
/// `#[serde(default)]` so a file written by an older build still loads.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tuning {
    /// Per-tick downward acceleration. Defaults to `rigid`/`particle`'s
    /// own 0.15 — the gnome falling at a different rate than the debris
    /// beside him would read as a bug even if neither number is wrong.
    pub gravity: f32,
    /// Horizontal acceleration per tick while a run key is held.
    pub run_accel: f32,
    /// Horizontal speed cap, cells per tick. 1.3 crosses the 512-cell
    /// world in ~6.5 seconds.
    pub run_max: f32,
    /// Subtractive slowdown per tick when grounded with no run key held —
    /// stops from full speed in ~5 ticks. Airborne motion keeps its speed,
    /// which is what makes a jump arc feel committed.
    pub ground_decel: f32,
    /// Multiplier on `run_accel` while airborne.
    pub air_control: f32,
    /// Upward velocity a jump starts with. Against gravity 0.15 this is
    /// v²/2g ≈ 13 cells of rise, apex around 13 ticks.
    pub jump_impulse: f32,
    /// Terminal fall speed. Below `rigid`'s 6.0/axis so landings stay
    /// controllable; the substepped sweep would be correct at any value.
    pub fall_clamp: f32,
    /// Ticks after walking off an edge during which a jump still fires.
    pub coyote_frames: u8,
    /// Ticks a jump press is remembered while airborne, so landing within
    /// the buffer jumps immediately.
    pub jump_buffer_frames: u8,
    /// Tallest ledge, in cells, walked up without jumping. Kept at about
    /// a third of his height because `rigid::mine` leaves rubble and
    /// worldgen terrain is rough — a step-up any shorter feels sticky on
    /// exactly the ground this game produces.
    pub step_up: u8,
    /// How far from the gnome's centre a dig can land, in cells. The dig
    /// point is clamped onto this circle along the aim ray, so clicking
    /// across the map digs at arm's length toward the cursor rather than
    /// doing nothing.
    pub dig_reach: u8,
    /// Radius of one dig bite. 7 bores a 15-cell hole — clearance for a
    /// 14-tall gnome to walk through his own tunnel upright rather than
    /// being shoved along it by the depenetration pass.
    pub dig_radius: u8,
    /// Ticks between bites while the button is held. 8 is ~7 bites a
    /// second: fast enough to feel like digging, slow enough that each
    /// bite's crack/impulse feedback reads individually.
    pub dig_cooldown: u8,
    /// How many rows of him may be buried in loose powder before it stops
    /// counting as wading and starts counting as being stuck. 4 of his 14
    /// rows — about knee-deep — so a drift slows him without swallowing
    /// him, and a pile deeper than that pushes back.
    pub wade_rows: u8,
    /// Horizontal speed multiplier while any powder overlaps him. Slogging
    /// through a drift should cost something, or wading is only a visual.
    pub wade_slowdown: f32,
    /// Vertical acceleration in water, as a multiple of `gravity`.
    /// Negative: he is lighter than water and rises. -0.3 rises slowly
    /// enough to read as floating up rather than as a balloon.
    pub buoyancy: f32,
    /// Per-tick velocity multiplier in water, both axes. Water should eat
    /// momentum — this is what stops a dive from carrying him to the
    /// bottom of a pool and what makes swimming feel unlike running.
    pub swim_damp: f32,
    /// Upward velocity one swimming stroke adds (`W`), or downward (`S`).
    pub stroke_impulse: f32,
    /// Ticks between strokes. Long enough that swimming reads as a series
    /// of pulls rather than a thruster.
    pub stroke_cooldown: u8,
    /// How far he can reach to shake a plant.
    ///
    /// Shorter than `dig_reach`, deliberately, and the ordering is the rule
    /// that picks the verb: a tree at arm's length outranks the rock behind
    /// it, and a tree across the clearing does not steal a mining click.
    pub shake_reach: u8,
    /// Chance a leaf comes down when a shake reaches it, **at total
    /// darkness** — weighted by the cube of how shaded the leaf is, so a
    /// healthy sunlit crown barely sheds and a dying shaded one rains
    /// litter. Abscission's own graded pressure, borrowed rather than
    /// reinvented. 0 stays off.
    pub shake_shed: f32,
    /// Chance one shake sows a seed.
    pub shake_seed: f32,
    /// Tallest lip, in cells, that he can catch at the top of a jump and
    /// pull himself up over.
    ///
    /// The step-up's airborne sibling, and fenced in by the four conditions
    /// at its call site rather than by this number — in particular it can
    /// only fire onto a surface he can actually stand on, so a flat wall is
    /// unmantle-able at any reach. Set to 0 to switch it off entirely.
    pub mantle_reach: u8,
    /// How fast he goes up, down and along a tree, in cells per tick.
    ///
    /// Set directly rather than accelerated toward, and slower than
    /// `run_max`: a climb that matched a run would read as running up a
    /// wall. One number for both axes, because a ladder has no reason to
    /// be faster sideways than upward.
    pub climb_speed: f32,
    /// What fraction of a standing jump the *exit* jump is — the one that
    /// fires on the tick his head clears the water.
    ///
    /// Separate from `jump_impulse` because the two are different moves. A
    /// full jump out of a pond launches him clear of it, which reads as
    /// being fired from the water rather than climbing out of it; what
    /// getting onto a bank wants is a pull up over the lip. Deliberately
    /// **not** part of `WaterFeel`: the four feels are buoyancy, damping
    /// and strokes, and folding this in would make `F4` stomp a value
    /// someone had just swept in the panel.
    pub surface_hop: f32,
    /// What fraction of freshly mined rock stays behind as rubble. The
    /// rest leaves the world as dust.
    ///
    /// **This is the number that decides whether caves are possible at
    /// all**, and it is arithmetic rather than taste. Breaking rock into
    /// rubble *conserves cells* — `shatter_to_rubble` swaps one material
    /// for another in place — so a bore can never open by breaking alone,
    /// however hard you hit it. Shoving the pieces aside only works while
    /// there is somewhere to shove them, and inside a massif there is
    /// not. Reported from the second playtest exactly that way: "the
    /// material breaks but goes nowhere, so you cannot really make a
    /// cave."
    ///
    /// So some volume has to leave, and the only question is how much. At
    /// 1.0 nothing leaves and you cannot dig; at 0.0 rock simply goes.
    ///
    /// **0.0 is the default, set by playtest** — asked directly, the owner
    /// picked `CLEAN`. An earlier version of this comment argued the
    /// opposite, that vanishing rock is the no-debris failure
    /// `CLAUDE.md` records, and 0.35 shipped on that reasoning. It was
    /// the wrong reading of the rule: what that rule forbids is
    /// *destruction* that produces nothing, and `dig_yield` is not
    /// destruction's number — it is only the **mining verb's**
    /// (`rigid::mine`). A collapse still fractures, still throws its
    /// graded debris, and never consults this. What 0.35 actually bought
    /// was a tunnel that silted up behind the digger, which is a
    /// different thing from a collapse feeling like one. The retained
    /// fraction is still the natural hook if mined stone ever becomes
    /// something you carry — `F2` to `DUST` or `SPOIL` is where that
    /// starts. See `SPOIL_MODES`.
    pub dig_yield: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            // These five mirror MOVEMENT_FEELS[0] (FLOATY) and the four
            // swim fields mirror WATER_FEELS[0] (DIVER) -- both chosen by
            // playtest. Kept in sync by `the_defaults_are_the_first_feel`.
            gravity: 0.10,
            run_accel: 0.12,
            run_max: 1.5,
            ground_decel: 0.25,
            air_control: 0.8,
            jump_impulse: 2.1,
            fall_clamp: 2.8,
            coyote_frames: 6,
            jump_buffer_frames: 4,
            step_up: 4,
            dig_reach: 30,
            dig_radius: 7,
            dig_cooldown: 8,
            wade_rows: 4,
            wade_slowdown: 0.4,
            buoyancy: 0.18,
            swim_damp: 0.84,
            stroke_impulse: 1.3,
            stroke_cooldown: 7,
            shake_reach: 20,
            shake_shed: 0.08,
            shake_seed: 0.12,
            mantle_reach: 4,
            climb_speed: 0.9,
            surface_hop: 0.75,
            // Mirrors SPOIL_MODES[0] (CLEAN), chosen by playtest.
            dig_yield: 0.0,
        }
    }
}

/// Named settings for `Tuning::dig_yield`, cycled with `F2`.
///
/// A selector rather than a decision, because the owner is genuinely
/// undecided and said so: "the easiest thing is probably just to remove
/// material when mining. Although there is a part of me that wants to
/// make collecting the stone dust/rubble part of the game mechanic. Not
/// sure if that will be more annoying than fun." Those two wishes want
/// opposite ends of this number, and which is more fun is not something
/// argument settles -- it is the grain-mode situation again.
///
/// Ordered from the built default outward, so cycling is a tour from
/// "none of it stays" up through keeping more and more of it. `CLEAN` is
/// first because the owner picked it by playtest; the earlier default
/// `DUST` is one press away and cycling back is always possible.
pub struct SpoilMode {
    pub name: &'static str,
    pub note: &'static str,
    pub dig_yield: f32,
}

pub const SPOIL_MODES: [SpoilMode; 5] = [
    SpoilMode { name: "CLEAN", note: "rock simply goes; no rubble at all", dig_yield: 0.0 },
    // **The gap between 0.0 and 0.35 was too wide, and the owner said so.**
    // Judging the pair, the verdict was "most of the options produce too
    // much dust... if there was a 10% option that would be interesting, but
    // 1/3 is even too much". The old list stepped 0 -> 35 -> 55 -> 100, so
    // there was nothing between "no rubble at all" and a third -- which
    // measured on `scene=tunnel` is enough to wade in from the nineteenth
    // bite and be buried by the thirtieth. A tenth is where "you can see
    // where you dug" lives without the bore filling in behind you.
    SpoilMode { name: "TRACE", note: "a tenth stays - enough to see where you dug", dig_yield: 0.10 },
    SpoilMode { name: "DUST", note: "a third stays as rubble, the rest blows away", dig_yield: 0.35 },
    SpoilMode { name: "SPOIL", note: "half stays - tunnels silt up behind you", dig_yield: 0.55 },
    SpoilMode { name: "HOARD", note: "nothing is lost - you cannot dig far", dig_yield: 1.0 },
];

/// A named set of movement numbers, cycled live with `F3`.
///
/// This exists because of a playtest answer, and the answer was the right
/// one: asked how the jump felt, the owner said *"honest, not sure — if
/// you could make some jump modes that I could toggle through and tell
/// you the best, that would be easier."* Which is `CLAUDE.md`'s own rule
/// for exactly this situation — for "does this feel right", ship a
/// runtime selector rather than argue a number — and it settled the grain
/// question in minutes after stills and argument had failed.
///
/// The first entry is current behaviour, so cycling away and back is
/// always possible, and the active one is named on screen. Everything
/// here is also reachable individually under `O` -> PLAYER; these are the
/// coarse "pick a character" step before that fine tuning.
pub struct MovementFeel {
    pub name: &'static str,
    /// What this one trades, in the player's terms — shown on screen.
    pub note: &'static str,
    pub gravity: f32,
    pub jump_impulse: f32,
    pub fall_clamp: f32,
    pub run_accel: f32,
    pub run_max: f32,
    pub air_control: f32,
}

impl MovementFeel {
    pub fn apply(&self, t: &mut Tuning) {
        t.gravity = self.gravity;
        t.jump_impulse = self.jump_impulse;
        t.fall_clamp = self.fall_clamp;
        t.run_accel = self.run_accel;
        t.run_max = self.run_max;
        t.air_control = self.air_control;
    }

    /// Rise of a full jump in cells, `v^2 / 2g` — quoted on screen
    /// because "how high" is the thing being judged and counting pixels
    /// off a moving character is not a reasonable thing to ask.
    pub fn jump_cells(&self) -> f32 {
        self.jump_impulse * self.jump_impulse / (2.0 * self.gravity)
    }
}

pub const MOVEMENT_FEELS: [MovementFeel; 5] = [
    // FLOATY first: chosen by playtest ("definitely diver and floaty"), so
    // it is what `Tuning::default` holds and what a fresh gnome arrives
    // with. Its *distances* were rescaled when he grew from 10 to 14 tall
    // -- the character of a feel is the relationship between the jump and
    // the body, not an absolute cell count, and leaving 18 cells alone
    // would have quietly made the approved feel a lower jump.
    MovementFeel {
        name: "FLOATY",
        note: "long hang, slow fall, easy to steer in the air",
        gravity: 0.10,
        jump_impulse: 2.1,
        fall_clamp: 2.8,
        run_accel: 0.12,
        run_max: 1.5,
        air_control: 0.8,
    },
    MovementFeel {
        name: "PLANNED",
        note: "the original default: brisker, drops harder",
        gravity: 0.15,
        jump_impulse: 2.4,
        fall_clamp: 4.5,
        run_accel: 0.16,
        run_max: 1.6,
        air_control: 0.5,
    },
    MovementFeel {
        name: "SNAPPY",
        note: "quick up, quick down, little hang",
        gravity: 0.24,
        jump_impulse: 3.0,
        fall_clamp: 6.0,
        run_accel: 0.26,
        run_max: 1.9,
        air_control: 0.45,
    },
    MovementFeel {
        name: "HEAVY",
        note: "weighty: slow to start, hard to stop, drops like rock",
        gravity: 0.34,
        jump_impulse: 3.1,
        fall_clamp: 6.0,
        run_accel: 0.12,
        run_max: 1.8,
        air_control: 0.25,
    },
    MovementFeel {
        name: "BOUNDER",
        note: "big arcs - clears two of his own heights",
        gravity: 0.16,
        jump_impulse: 3.0,
        fall_clamp: 5.0,
        run_accel: 0.19,
        run_max: 1.8,
        air_control: 0.6,
    },
];

/// The same idea for water, cycled with `F4`, after the first playtest
/// reported the swimming as the thing that was off: *"Water is off, with
/// swimming, I didn't like the buoyancy."*
///
/// The spread is deliberately across the *model*, not just its strength,
/// because "didn't like the buoyancy" has two opposite readings and
/// guessing which costs a whole round trip: he may have meant it lifts
/// him too eagerly (nothing to do, he pops up on his own) or that it
/// fights him (he cannot stay down). `CORK` and `DIVER` sit at those two
/// extremes and `TREAD` removes automatic vertical motion entirely, so
/// whichever it was, one of these is the answer.
pub struct WaterFeel {
    pub name: &'static str,
    pub note: &'static str,
    pub buoyancy: f32,
    pub swim_damp: f32,
    pub stroke_impulse: f32,
    pub stroke_cooldown: u8,
}

impl WaterFeel {
    pub fn apply(&self, t: &mut Tuning) {
        t.buoyancy = self.buoyancy;
        t.swim_damp = self.swim_damp;
        t.stroke_impulse = self.stroke_impulse;
        t.stroke_cooldown = self.stroke_cooldown;
    }
}

pub const WATER_FEELS: [WaterFeel; 4] = [
    // DIVER first, for the same reason FLOATY is: chosen by playtest. The
    // report was "I didn't like the buoyancy", and of the four this is
    // the one that hands the vertical axis back to the player entirely --
    // he sinks unless you swim, which is what "didn't like it lifting me"
    // resolves to.
    WaterFeel {
        name: "DIVER",
        note: "sinks slowly; staying up is something you do",
        buoyancy: 0.18,
        swim_damp: 0.84,
        stroke_impulse: 1.3,
        stroke_cooldown: 7,
    },
    WaterFeel {
        name: "TREAD",
        note: "no automatic rise or sink - he stays where you leave him",
        buoyancy: 0.0,
        swim_damp: 0.86,
        stroke_impulse: 1.1,
        stroke_cooldown: 8,
    },
    WaterFeel {
        name: "PLANNED",
        note: "the original default: floats up on its own",
        buoyancy: -0.3,
        swim_damp: 0.9,
        stroke_impulse: 0.8,
        stroke_cooldown: 10,
    },
    WaterFeel {
        name: "CORK",
        note: "pops to the surface fast; strokes only steer",
        buoyancy: -0.85,
        swim_damp: 0.93,
        stroke_impulse: 0.6,
        stroke_cooldown: 12,
    },
];

impl Tuning {
    /// Where the panel persists these, beside the other asset files.
    pub const ASSET_PATH: &'static str = "assets/player.ron";

    /// Load from `ASSET_PATH`, defaults when absent — absent is the
    /// normal case for a fresh checkout, same as `explosion::Tuning`.
    pub fn load() -> Self {
        std::fs::read_to_string(Self::ASSET_PATH)
            .ok()
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Full re-serialization, like `explosion::Tuning::save` and unlike
    /// the material files' careful span-edit: this file is generated, has
    /// no comments to lose, and each field's reasoning lives on the
    /// struct itself.
    pub fn save(&self) -> Result<(), String> {
        let pretty = ron::ser::PrettyConfig::new().struct_names(false);
        let text = ron::ser::to_string_pretty(self, pretty).map_err(|e| e.to_string())?;
        std::fs::write(Self::ASSET_PATH, text).map_err(|e| e.to_string())
    }
}

/// One tick's worth of intent, assembled by the windowing layer from held
/// keys and handed through `App::update`. This is the replay unit: same
/// build, same world, same `PlayerInput` sequence — same run.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub jump_held: bool,
    /// Edge-triggered: true only on the tick after the key went down.
    /// `App::update` clears it after the first simulated tick of a frame,
    /// so a catch-up burst of ticks can't multi-fire one press.
    pub jump_pressed: bool,
    /// Reserved: crouch on ground, swim-down in water (phase 3).
    pub down: bool,
    /// Shift: hold on to a plant. Climbing needs an input of its own —
    /// riding on `jump_held` meant jumping through a wood grabbed every
    /// trunk it touched. See `step`'s climb gate.
    pub grab: bool,
    /// Cursor in world coordinates, for the phase-2 dig aim. Plumbed now
    /// so the input path doesn't need reworking then.
    pub aim: Option<(i32, i32)>,
}

/// The character. Position is the rectangle's top-left corner in world
/// space, fractional like `ChunkBody`'s origin — the integer rectangle
/// (`rect_origin`) is what collides and draws.
#[derive(Clone, Debug)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// A blocking cell sits in the row under the feet.
    pub grounded: bool,
    /// The depenetration pass found no way to free the rectangle: the
    /// gnome is entombed. Movement and jumping are dead until something
    /// changes — phase 2's dig is the escape verb.
    pub buried: bool,
    coyote: u8,
    jump_buffer: u8,
    /// Last tick's `jump_held`, for the release edge that halves an
    /// ascending `vy` — the variable-height jump.
    jump_was_held: bool,
    /// Ticks until the next dig bite may land. Sim state rather than UI
    /// state, so a replayed input sequence digs on the same ticks.
    dig_cooldown: u8,
    /// His head is in liquid: swimming rather than falling. Public
    /// because it is the difference between two entirely different
    /// control schemes and a harness reporting "he is in the water"
    /// cannot infer it from position alone.
    pub swimming: bool,
    /// Last tick's `swimming`, so the tick he breaks the surface can be
    /// told from every other tick. That one tick is where the exit hop is
    /// sized (`Tuning::surface_hop`).
    was_swimming: bool,
    /// Last tick's `climbing`, so *how* he came off a tree can be told:
    /// climbing off the top launches him, letting go of the grab key
    /// drops him. Same shape as `was_swimming`, same reason.
    was_climbing: bool,
    /// Loose powder overlaps him: slowed, and sunk into it.
    pub wading: bool,
    /// Gripping living plant tissue: gravity is off and `W`/`S` drive him
    /// up and down it. Public for the same reason `swimming` is — it is a
    /// different control scheme, and no harness can infer it from position.
    pub climbing: bool,
    /// Which way the sprite is drawn.
    ///
    /// Sim state, not renderer state, and that is not incidental: the
    /// renderer must be a pure function of the world for
    /// `dirty_rect_skip_is_pixel_identical_to_a_full_redraw` to hold, and
    /// a stateful skyline was built here once and reverted for keeping its
    /// state on the `Renderer` side of that line.
    ///
    /// Set from **input**, never from `vx`. A gnome riding a drifting
    /// chunk body has `body.vx` added to his position every tick, so a
    /// velocity-keyed sprite would turn to face the way the slab is
    /// sliding while he stands still on it.
    pub facing_left: bool,
    /// Ticks of swing pose left. Sim state for the same reason `facing_left`
    /// is, and a countdown rather than a flag so one blow reads as a blow
    /// at 60 Hz instead of a single-frame flicker.
    pub action: u8,
    /// Ticks until the next swimming stroke may fire.
    stroke_cooldown: u8,
    /// Where the last bite landed, so the next one can be *swept* from it
    /// rather than stamped as a fresh disc. See `rigid::mine_swept`.
    last_bite: Option<(i32, i32)>,
}

impl Player {
    /// Spawn with the rectangle centred on `(x, y)`.
    pub fn at(x: i32, y: i32) -> Self {
        Self {
            x: (x - PLAYER_WIDTH / 2) as f32,
            y: (y - PLAYER_HEIGHT / 2) as f32,
            vx: 0.0,
            vy: 0.0,
            grounded: false,
            buried: false,
            coyote: 0,
            jump_buffer: 0,
            jump_was_held: false,
            dig_cooldown: 0,
            swimming: false,
            was_swimming: false,
            was_climbing: false,
            wading: false,
            climbing: false,
            facing_left: false,
            action: 0,
            stroke_cooldown: 0,
            last_bite: None,
        }
    }

    /// Top-left of the occupied cell rectangle.
    pub fn rect_origin(&self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }

    /// Inclusive world-space bounds, for the renderer's dirty rect.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        let (x, y) = self.rect_origin();
        (x, y, x + PLAYER_WIDTH - 1, y + PLAYER_HEIGHT - 1)
    }

    /// Centre of the occupied rectangle — where reach is measured from.
    pub fn center(&self) -> (i32, i32) {
        let (x, y) = self.rect_origin();
        (x + PLAYER_WIDTH / 2, y + PLAYER_HEIGHT / 2)
    }
}

/// How a single cell treats the character.
///
/// The split is what phase 3 is: through phase 2 powder was simply a wall,
/// which made the gnome walk on top of sand as if it were pavement. Powder
/// is now `Soft` — it stops him the way a drift does, by being something he
/// stands *in* up to a point rather than something he stands *on*. Liquid
/// and gas are free space and always were; buoyancy, not collision, is what
/// makes water hold him up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Footing {
    /// Nothing here: empty, gas, or liquid.
    Free,
    /// Loose powder. Passable at the feet (see `WADE_ROWS`), a wall above.
    Soft,
    /// Living plant tissue: a trunk, a branch, foliage, moss. **Passable
    /// like `Free`, and never a floor** — but not the same as `Free`, and
    /// the difference is what the four readers below have to decide.
    ///
    /// A tree used to be `Hard`, which made a wood something you got stuck
    /// in rather than something you walked through: nothing could jump a
    /// trunk, nothing could go round one, and the dig cannot cut an
    /// organism cell either, so there was no way out. It is scenery now,
    /// the way a tree reads in a 3D game, and the trade for that is that
    /// you can climb it.
    ///
    /// Deliberately *not* folded into `Free`. Two readers genuinely want
    /// the distinction: the aim ray must still stop at a tree you can no
    /// longer walk into, and `displace_disc` must still count a branch as
    /// something spoil can come to rest on, because the CA genuinely lets
    /// powder sit on a `Plant` cell.
    Climb,
    /// Rock that is *scenery*: a stalagmite, a stalactite, a flowstone
    /// column. **Passable like `Free`, never a floor, and not climbable.**
    ///
    /// Its own variant rather than reusing `Climb`, and the reason is one
    /// line of code: `grip` tests `== Footing::Climb` exactly, so folding
    /// these in would let the gnome haul himself up a stalagmite. Walking
    /// *past* a formation and climbing a tree are different affordances and
    /// the two flags say so separately (`Material::scenery`).
    ///
    /// Stops the aim ray, like `Climb` — you must be able to point at a
    /// formation to mine it, and mining is what makes it breakable rather
    /// than merely absent.
    Scenery,
    /// Rock, creatures, a chunk body, or the world edge.
    Hard,
}

/// Chunk-body cells near the player this tick, gathered once so the
/// collision predicate can see them.
///
/// M9's own acceptance line asks that he "stands on a tumbling rigid
/// body", and bodies live off-grid in `world.chunk_bodies` — a grid read
/// cannot see one, so before this a slab passed straight through him.
/// Gathered per tick rather than tested per cell on purpose: the predicate
/// runs ~18 times per position sample and several samples per tick, and
/// scanning every body's every cell that often would be the one expensive
/// thing in an otherwise free character. Bodies are few and usually zero,
/// in which case this allocates nothing and costs one `is_empty` check.
struct Bodies {
    /// Cell position and the index of the body it belongs to, sorted by
    /// position so the predicate binary-searches. Sorted rather than
    /// hashed because the count is tiny and the order must be stable for
    /// determinism. The index is what makes *riding* possible as
    /// distinct from merely colliding: it says which body is underfoot.
    cells: Vec<((i32, i32), usize)>,
}

impl Bodies {
    /// Body cells within `margin` of the rectangle at `(x, y)` — a window
    /// wide enough to cover this tick's whole sweep, including step-up
    /// and depenetration.
    fn near(world: &World, x: i32, y: i32, margin: i32) -> Self {
        let (lo_x, lo_y) = (x - margin, y - margin);
        let (hi_x, hi_y) = (x + PLAYER_WIDTH + margin, y + PLAYER_HEIGHT + margin);
        let mut cells = Vec::new();
        for (i, body) in world.chunk_bodies.iter().enumerate() {
            for cell in &body.cells {
                let (cx, cy) = body.cell_position(cell);
                if cx >= lo_x && cx <= hi_x && cy >= lo_y && cy <= hi_y {
                    cells.push(((cx, cy), i));
                }
            }
        }
        cells.sort_unstable();
        Self { cells }
    }

    fn none() -> Self {
        Self { cells: Vec::new() }
    }

    /// Which body occupies `(x, y)`, if any.
    fn at(&self, x: i32, y: i32) -> Option<usize> {
        if self.cells.is_empty() {
            return None;
        }
        self.cells
            .binary_search_by_key(&(x, y), |&(pos, _)| pos)
            .ok()
            .map(|i| self.cells[i].1)
    }

    fn holds(&self, x: i32, y: i32) -> bool {
        self.at(x, y).is_some()
    }
}

/// What the cell at `(x, y)` is, to the character. Raw material kind
/// rather than `is_empty`, so a managed liquid body's container cells
/// (materially empty) read as the water they look like.
fn footing(world: &World, bodies: &Bodies, x: i32, y: i32) -> Footing {
    if !world.in_bounds(x, y) {
        return Footing::Hard; // OUT_OF_BOUNDS is solid: world-edge walls for free
    }
    if bodies.holds(x, y) {
        return Footing::Hard;
    }
    let cell = world.get(x, y);
    let material = world.materials.get(cell.material);
    // Living tissue, and only living tissue.
    //
    // Both halves are load-bearing and neither works alone. The flag is
    // what makes this data — a future thorn hedge says "I stop you" in its
    // own `.ron` without touching this function. The organism id is what
    // separates a grown tree from a `wood` wall a player painted, which
    // are the same material and must behave differently; `Plant` kind
    // cannot tell them apart, and `render.rs`'s organism overlay already
    // documents the same predicate for the same reason.
    //
    // Costs nothing: this resolved the material anyway, and `cell` was
    // already fetched.
    // Scenery needs no organism gate: it is its own material, written only
    // by worldgen, so there is no painted-wall twin to tell it apart from.
    // See `Material::scenery`.
    if material.scenery {
        return Footing::Scenery;
    }
    if cell.organism_id() != 0 && material.climbable {
        return Footing::Climb;
    }
    // Light enough to run through -- leaf litter. Tested before the kind
    // match so it beats `Powder => Soft`, which is the whole point: this
    // material is a powder in every other respect and must stay one.
    //
    // `Free`, not merely drag-free, and deliberately: `wade_rows` has a
    // cliff in it (four rows of him in powder is where wading stops and
    // *stuck* begins), so exempting the drag alone would leave the failure
    // that actually reads as a bug. He should pass through a drift of
    // leaves the way he passes through a branch. See
    // `Material::insubstantial`, including where the "item that lets you
    // move freely" switch goes when it exists.
    //
    // **Two readers this opts litter out of, neither of them tested.**
    // `Free` is documented as "nothing here: empty, gas, or liquid", and
    // `Climb`/`Scenery` exist precisely because some readers need
    // passable-but-not-air: the aim ray stops on a `Scenery` cell, and
    // `displace_disc` counts one as somewhere spoil can rest. Litter now
    // reads as air to both. That is probably right -- you should be able to
    // dig through a drift of leaves, and an aim ray should not stop on one
    // -- but it is untested, and if a fourth variant is ever wanted this is
    // the line that spawns it.
    if material.insubstantial {
        return Footing::Free;
    }
    match material.kind {
        MaterialKind::Solid | MaterialKind::Plant => Footing::Hard,
        MaterialKind::Powder => Footing::Soft,
        // `Creature` falls through to `Free`, and used to be `Hard`.
        //
        // A single ant was a wall. That was defensible while a *tree* was
        // one too, and stopped being the moment living plants became
        // scenery he walks through: an ant is one cell and a worm is a
        // few, and being brought to a halt by one while strolling through
        // a trunk reads as a bug rather than as a rule. Nothing else has
        // to change for it -- a nest is `kind: Solid` and still blocks, so
        // what a colony *builds* is as solid as it ever was.
        //
        // One-way, deliberately: `creature::move_cost` is untouched, so
        // ants still cannot walk into plants or each other and still walk
        // *along* branches. Passability is a property of the character,
        // not of the world -- the same line `Material::climbable`'s own
        // doc draws.
        _ => Footing::Free,
    }
}

/// Whether the rectangle with top-left `(x, y)` is somewhere the gnome may
/// stand: no hard blocker anywhere in it, and loose powder only in the
/// bottom `wade` rows.
///
/// That second clause is the wade. Allowing powder at the feet and not at
/// the chest is what makes him sink into a drift to about the knee and
/// stop, rather than either walking on its surface (phase 1and2) or sinking
/// through it as if it were air. It is also, deliberately, the same
/// predicate `depenetrate` uses, so sand arriving around his boots is not
/// treated as an invasion needing a shove — only sand up to his chest is.
fn rect_free(world: &World, bodies: &Bodies, x: i32, y: i32, wade: i32) -> bool {
    let chest = PLAYER_HEIGHT - wade;
    for dy in 0..PLAYER_HEIGHT {
        for dx in 0..PLAYER_WIDTH {
            match footing(world, bodies, x + dx, y + dy) {
                Footing::Hard => return false,
                Footing::Soft if dy < chest => return false,
                _ => {}
            }
        }
    }
    true
}

/// One simulation tick. Runs in `App::update`'s serial phase; reads the
/// grid, never writes it (the ghost contract).
pub fn step(world: &mut World, input: PlayerInput, tuning: &Tuning) {
    let Some(mut p) = world.player.take() else {
        return;
    };
    let wade = tuning.wade_rows as i32;
    // Body cells near him, once. The margin covers this tick's whole
    // sweep — the furthest he can travel plus the depenetration reach —
    // so the window is gathered before he moves and is still valid after.
    let (xi, yi) = p.rect_origin();
    let reach = tuning.fall_clamp.max(tuning.run_max).ceil() as i32 + DEPENETRATE_REACH + 1;
    let bodies = Bodies::near(world, xi, yi, reach);

    // Free an invaded rectangle first, so this tick's movement starts
    // from a legal position: sand that fell into us, a body that settled
    // on us. Shortest clear push wins; up is tried first at each distance
    // because material arrives from above, and "on top of the pile" is
    // the right place to end up.
    depenetrate(world, &bodies, &mut p, wade);

    if p.buried {
        // Entombed: no movement, no jump, velocities dead. Coyote and the
        // jump buffer still tick down so nothing fires the instant the
        // gnome is freed.
        p.vx = 0.0;
        p.vy = 0.0;
        p.climbing = false;
        p.coyote = p.coyote.saturating_sub(1);
        p.jump_buffer = p.jump_buffer.saturating_sub(1);
        p.dig_cooldown = p.dig_cooldown.saturating_sub(1);
        p.stroke_cooldown = p.stroke_cooldown.saturating_sub(1);
        p.jump_was_held = input.jump_held;
        world.player = Some(p);
        return;
    }
    p.dig_cooldown = p.dig_cooldown.saturating_sub(1);
    p.stroke_cooldown = p.stroke_cooldown.saturating_sub(1);
    p.action = p.action.saturating_sub(1);
    // Held direction, and nothing else. Unchanged when neither or both are
    // held, so he keeps facing where he was last told to go rather than
    // snapping forward the moment you let go.
    match (input.left, input.right) {
        (true, false) => p.facing_left = true,
        (false, true) => p.facing_left = false,
        _ => {}
    }

    // Which medium he is in, decided before anything reads it.
    //
    // Swimming keys off the *head* row rather than any overlap, and that
    // is the whole reason a surface exists to swim at: standing chest-deep
    // in a pool with his head in air, he still walks and jumps normally,
    // and it is only going under that hands control to the strokes. Wading
    // keys off any overlap at all, because powder round the boots should
    // already be slowing him.
    let (xi, yi) = p.rect_origin();
    p.was_swimming = p.swimming;
    p.swimming = (0..PLAYER_WIDTH)
        .any(|dx| world.in_bounds(xi + dx, yi) && world.materials.kind(world.get(xi + dx, yi).material) == MaterialKind::Liquid);
    // In the water *at all*, which is a different question from `swimming`
    // and is the one the haul-out below asks.
    //
    // `swimming` reads the head row, so it goes false the instant he
    // surfaces -- and at that moment he is treading at the edge of a pool
    // with nothing under his feet, which is precisely when he wants to
    // pull himself onto the bank. Keying the haul-out on `swimming` would
    // have switched it off exactly where it is needed.
    let floating = (0..PLAYER_HEIGHT).any(|dy| {
        (0..PLAYER_WIDTH).any(|dx| {
            world.in_bounds(xi + dx, yi + dy) && world.materials.kind(world.get(xi + dx, yi + dy).material) == MaterialKind::Liquid
        })
    });
    // How *deep* in the powder, not merely whether. Reported from the
    // first playtest: "sand and dirt felt the same, which was just a
    // little slower than rock. It just felt like it changed my speed" —
    // which is exactly what a binary flag produces, a debuff rather than
    // a depth. Counting the rows of him that are in it and scaling the
    // slowdown by that gives the graded outcome the ethos asks for: a
    // scuff through the top of a drift barely registers, ankle-deep drags
    // a little, knee-deep is a slog.
    let soaked = (0..PLAYER_HEIGHT)
        .filter(|&dy| (0..PLAYER_WIDTH).any(|dx| footing(world, &bodies, xi + dx, yi + dy) == Footing::Soft))
        .count() as f32;
    p.wading = !p.swimming && soaked > 0.0;
    let wade_drag = if p.wading {
        let t = (soaked / wade.max(1) as f32).clamp(0.0, 1.0);
        1.0 + (tuning.wade_slowdown - 1.0) * t
    } else {
        1.0
    };

    // On a tree. Living tissue is the trade for living tissue being
    // passable: it stops being something you bump into and becomes
    // something you go up.
    //
    // **Grabbed with the hands, not stood in.** Only the top half of him
    // counts, and that is not decoration -- it is what keeps a root system
    // from eating the jump key. Roots are `climbable` too (they had to be,
    // or one threading through a bank would still be a wall), he wades four
    // rows into soft ground, and a wood is full of roots. Counting any
    // overlap would mean holding `W` to jump anywhere near a tree grabbed a
    // root at boot height and stuck him at knee level. Reaching for it is
    // also simply what climbing is.
    let grip = (0..GRIP_ROWS).any(|dy| (0..PLAYER_WIDTH).any(|dx| footing(world, &bodies, xi + dx, yi + dy) == Footing::Climb));
    // How much foliage he is falling through, graded by how much of him is
    // in it — the same shape as the wade above, and for the reason recorded
    // there: a flag reads as a debuff, a depth reads as a canopy.
    //
    // Summed over rows rather than maxed, so clipping the top of a crown
    // barely registers and going through the middle of one arrests him.
    // Read off the material (`fall_drag`), not off `Footing::Climb`, so a
    // bare trunk catches nothing while the leaves on it do.
    let foliage: f32 = (0..PLAYER_HEIGHT)
        .map(|dy| {
            (0..PLAYER_WIDTH)
                .map(|dx| {
                    let c = world.get(xi + dx, yi + dy);
                    if c.organism_id() == 0 {
                        return 0.0;
                    }
                    world.materials.get(c.material).fall_drag
                })
                .fold(0.0f32, f32::max)
        })
        .sum::<f32>()
        / PLAYER_HEIGHT as f32;

    // **You are on the tree only while you are holding on.** Shift grabs,
    // `W`/`S` climb, no vertical input hangs, releasing lets go.
    //
    // This shipped keyed on `W` instead, on the argument that the keyboard
    // was full and climbing could ride the jump key for free. **Overturned
    // by play, and the failure is worth keeping**: `W` is jump *and* climb,
    // so jump-walking through a wood grabbed every trunk he clipped
    // mid-arc and kept lifting him -- reported as "if I am just jump
    // walking in a forest, I can basically fly/hover". The trade that
    // reasoning made was "no new key" over "no accidental grab", and it
    // picked wrong: a verb that fires when you did not ask for it is worse
    // than a verb that costs a key.
    //
    // Two *earlier* rejections still stand, and both are now moot rather
    // than wrong -- an explicit grab key subsumes them, and they are kept
    // because either would come back the moment someone tried to make the
    // grab implicit again:
    //
    // - **Engage on contact alone** made a tree flypaper: a gnome falling
    //   *past* one caught himself on it, measured arriving at row 48
    //   against a floor at 88.
    // - **Require `!grounded`** read well and could not be entered:
    //   `jump_pressed` is an edge, so walking up to a trunk with `W` held
    //   offered no second press to hop with.
    //
    // `GRIP_ROWS` still matters and is not made redundant by the key: it
    // is what stops Shift near a root system grabbing at boot height.
    p.was_climbing = p.climbing;
    p.climbing = grip && !p.swimming && input.grab;
    // **Letting go drops him; running out of tree launches him.** The
    // climb branch keeps the coyote window and jump buffer warm so that
    // climbing off the crown with `W` still held springs him off it, which
    // is worth keeping. With a grab key that same armed jump would fire
    // when you simply released Shift — being flung upward for letting go,
    // which is the opposite of what the input says. The two exits are told
    // apart by whether there is still tissue in reach: tissue gone means
    // he climbed off the end of it, tissue still there means he chose to
    // drop.
    if p.was_climbing && !p.climbing && grip {
        p.jump_buffer = 0;
        p.coyote = 0;
        // And the climb's own upward velocity goes with them. A climb sets
        // `vy` directly, so without this he coasts up on the momentum of
        // the last stroke and decelerates under gravity -- measured at 3.6
        // cells, which reads as a small hop for releasing a key. Letting go
        // drops him from rest.
        p.vy = 0.0;
    }

    // --- intent to velocity ---
    let accel = if p.grounded { tuning.run_accel } else { tuning.run_accel * tuning.air_control };
    match (input.left, input.right) {
        (true, false) => p.vx -= accel,
        (false, true) => p.vx += accel,
        // No input (or both, cancelling): ground friction bleeds speed
        // off; airborne speed is kept so arcs stay committed.
        _ if p.grounded => {
            let drop = tuning.ground_decel.min(p.vx.abs());
            p.vx -= drop * p.vx.signum();
        }
        _ => {}
    }
    let top_speed = tuning.run_max * wade_drag;
    p.vx = p.vx.clamp(-top_speed, top_speed);

    // A climb is a different control scheme, so it sets velocity outright
    // rather than accelerating toward it. Momentum on a ladder reads as
    // running, and the first thing that tells a player they are on one is
    // that the controls answer instantly and stop instantly.
    if p.climbing {
        p.vx = match (input.left, input.right) {
            (true, false) => -tuning.climb_speed,
            (false, true) => tuning.climb_speed,
            _ => 0.0,
        };
        // Grip. No input holds him where he is, which is what separates a
        // climb from sliding down a pole -- and it is the owner's call:
        // the whole tree is a hold and none of it is a floor.
        p.vy = match (input.jump_held, input.down) {
            (true, false) => -tuning.climb_speed,
            (false, true) => tuning.climb_speed,
            _ => 0.0,
        };
    }

    if input.jump_pressed {
        p.jump_buffer = tuning.jump_buffer_frames;
    } else {
        p.jump_buffer = p.jump_buffer.saturating_sub(1);
    }
    if p.grounded {
        p.coyote = tuning.coyote_frames;
    } else {
        p.coyote = p.coyote.saturating_sub(1);
    }
    if p.swimming {
        // Coyote is kept alive the whole time he is under.
        //
        // This is what gets him *out* of a pool rather than bobbing at its
        // edge forever: buoyancy lifts him until his head clears, at which
        // point `swimming` goes false with him still airborne and nothing
        // grounded — without this, the jump he presses at the surface has
        // nothing to fire against. With it, breaking the surface leaves a
        // few frames in which `W` is a real jump, and he hops onto the
        // bank. The same window makes a stroke-and-vault out of a flooded
        // tunnel work.
        p.coyote = tuning.coyote_frames;
    }
    if p.jump_buffer > 0 && p.coyote > 0 && !p.swimming && !p.climbing {
        // Scaled down on the tick he leaves the water. A full standing
        // jump out of a pond reads as being fired from it; what the bank
        // needs is a pull up over the lip, which is what `surface_hop`
        // sizes. Live-tunable because only play settles how big "a little
        // jump" is.
        let scale = if p.was_swimming { tuning.surface_hop } else { 1.0 };
        p.vy = -tuning.jump_impulse * scale;
        p.jump_buffer = 0;
        p.coyote = 0;
    }
    // Variable height: releasing the key on the way up halves the rise,
    // once, on the release edge.
    if p.jump_was_held && !input.jump_held && p.vy < 0.0 && !p.swimming && !p.climbing {
        p.vy *= 0.5;
    }
    p.jump_was_held = input.jump_held;

    if p.climbing {
        // No gravity, and the coyote and buffer both kept alive -- the
        // same trick swimming uses a few lines up, for the same reason and
        // with the same payoff. Climbing off the top of a trunk with `W`
        // still held drops `climbing` on the tick the tissue runs out,
        // with a jump already armed and the coyote still warm, so he
        // *launches* off the crown instead of stepping off it into a fall.
        // Walking out sideways along a branch does the same thing.
        //
        // One rule, two features: this is the identical buffer-arming that
        // gets him out of a pond, at a second call site.
        p.coyote = tuning.coyote_frames;
        p.jump_buffer = if input.jump_held { tuning.jump_buffer_frames } else { 0 };
    } else if p.swimming {
        // Strokes, not thrust: `W` pulls up and `S` pulls down, each on
        // the same cooldown, so holding a key gives a rhythm of pulls
        // with a drift between them rather than a smooth ascent. Buoyancy
        // does the rest, and damping is what makes water feel like water
        // — a dive loses its speed instead of carrying him to the bottom.
        if p.stroke_cooldown == 0 && (input.jump_held || input.down) {
            let pull = if input.down { tuning.stroke_impulse } else { -tuning.stroke_impulse };
            p.vy += pull;
            p.stroke_cooldown = tuning.stroke_cooldown;
        }
        p.vy += tuning.gravity * tuning.buoyancy;
        p.vx *= tuning.swim_damp;
        p.vy *= tuning.swim_damp;
        // Held `W` leaves a jump armed for the tick his head clears,
        // rather than clearing the buffer every frame he is under.
        //
        // This is the other half of the coyote refresh above, and without
        // it that refresh had nothing to fire. `jump_pressed` is an *edge*
        // and `App::update` consumes it (`app.rs`'s
        // `player_input.jump_pressed = false`), so a swimmer holding `W`
        // has no press left by the time he surfaces -- and under the
        // default `DIVER` feel, whose `buoyancy` is positive, holding `W`
        // is the *only* way to surface at all. So the one input that gets
        // him to the top was the one input guaranteed to have nothing
        // armed when he arrived, and he bobbed at the bank. Reported from
        // play as "getting out of water should have a little jump to it so
        // you can get over a ledge".
        //
        // This cannot turn a stroke into a jump: the gate above refuses to
        // fire while `swimming`, and a stroke has already been spent on
        // its own cooldown by the time this runs.
        p.jump_buffer = if input.jump_held { tuning.jump_buffer_frames } else { 0 };
    } else {
        p.vy = (p.vy + tuning.gravity).min(tuning.fall_clamp);
        // A crown breaks a fall. Downward only: applied to a rise it would
        // read as the tree grabbing at him, and applied sideways it would
        // read as glue.
        if p.vy > 0.0 && foliage > 0.0 {
            p.vy *= 1.0 - foliage.clamp(0.0, 1.0);
        }
    }

    // --- the sweep: substepped at <= 1 cell, X (with step-up) then Y ---
    // Same anti-tunnelling shape as `rigid::advance` and for the same
    // reason: a fast fall must not cross a thin floor between samples.
    let distance = (p.vx * p.vx + p.vy * p.vy).sqrt();
    let steps = distance.ceil().max(1.0) as i32;
    let (step_x, step_y) = (p.vx / steps as f32, p.vy / steps as f32);
    for _ in 0..steps {
        if step_x != 0.0 {
            let next_x = p.x + step_x;
            let (nxi, nyi) = (next_x.round() as i32, p.y.round() as i32);
            if rect_free(world, &bodies, nxi, nyi, wade) {
                p.x = next_x;
            } else {
                // Lift the same horizontal move over whatever blocked it,
                // by up to a few whole cells. Three cases, and they differ
                // in how far and in what they demand of the landing.
                //
                // **Grounded** is the original step-up: rubble and rough
                // terrain are the norm here, and stopping dead at a
                // two-cell bump feels sticky.
                //
                // **In the water** is the haul-out. A swimmer is never
                // grounded (see the `grounded` probe's `!p.swimming`), so
                // pressing into the bank of a deep pool found no floor to
                // step up from and he simply stopped against the wall.
                // Water is the floor in that case: it holds him at a
                // height, which is the thing step-up needs, and the reason
                // this is not a general airborne climb.
                //
                // **Airborne** is the mantle — catching a lip at the top of
                // a jump and pulling up over it — and it is fenced in on
                // four sides, because an unfenced version is a wall climb:
                //   - at or past the apex (`vy >= 0`); mantling on the way
                //     up would let him run up a wall,
                //   - *pushing into* the ledge, so a dead drop alongside a
                //     wall does not snap him onto it,
                //   - not while climbing — inside a tree the ladder owns
                //     the vertical axis,
                //   - and the landing must be a real surface. That last one
                //     is what actually makes it safe rather than merely
                //     tuned: a flat wall offers no footing at any lift, so
                //     nothing ever fires against one, at any reach.
                let mantling = !p.grounded && !floating && !p.climbing && p.vy >= 0.0 && ((step_x > 0.0 && input.right) || (step_x < 0.0 && input.left));
                let lift_limit = if p.grounded || floating {
                    tuning.step_up as i32
                } else if mantling {
                    tuning.mantle_reach as i32
                } else {
                    0
                };
                let mut climbed = false;
                for lift in 1..=lift_limit {
                    if !rect_free(world, &bodies, nxi, nyi - lift, wade) {
                        continue;
                    }
                    if mantling {
                        let lands_on_something = (0..PLAYER_WIDTH)
                            .any(|dx| matches!(footing(world, &bodies, nxi + dx, nyi - lift + PLAYER_HEIGHT), Footing::Hard | Footing::Soft));
                        if !lands_on_something {
                            continue;
                        }
                    }
                    p.x = next_x;
                    p.y -= lift as f32;
                    if mantling {
                        // Arriving, not still falling. Without this he
                        // keeps his descent speed and drops straight back
                        // off the lip he just pulled onto.
                        p.vy = 0.0;
                    }
                    climbed = true;
                    break;
                }
                if !climbed {
                    p.vx = 0.0;
                }
            }
        }
        if step_y != 0.0 {
            let next_y = p.y + step_y;
            let (nxi, nyi) = (p.x.round() as i32, next_y.round() as i32);
            if rect_free(world, &bodies, nxi, nyi, wade) {
                p.y = next_y;
            } else {
                // Landing or head bonk: the vertical axis dies, the
                // horizontal one keeps whatever it had — same per-axis
                // reasoning as `rigid::advance`.
                p.vy = 0.0;
            }
        }
    }

    // Grounded: anything at all in the row directly under the feet —
    // rock, a chunk body, or packed powder. `Soft` counts here even
    // though it does not count in `rect_free`, and the asymmetry is the
    // wade: sand is something he stands on *and* sinks into, so it must
    // both let him in and hold him up. A body counts because M9 asks him
    // to stand on a tumbling one, and `bodies` is why that can be seen.
    let (xi, yi) = p.rect_origin();
    // `Hard | Soft`, not `!= Free`, and the difference is the whole of
    // "pure ladder". `Climb` is passable, so if it also counted as ground
    // he would stand *on* a leaf, jump off a twig, and step-up would walk
    // him up the side of a trunk one bump at a time. A tree holds you
    // because you are gripping it, never because you are standing on it.
    p.grounded = !p.swimming
        && (0..PLAYER_WIDTH).any(|dx| matches!(footing(world, &bodies, xi + dx, yi + PLAYER_HEIGHT), Footing::Hard | Footing::Soft));

    // Riding a body, which needs two rules rather than the one the plan
    // expected. Both are recorded because both were found by measurement.
    //
    // 1. **Seeing the body is not enough.** A falling body clamps at
    //    `rigid`'s 6.0 per axis and the gnome clamps at 4.0, so every
    //    platform he stood on outran him: `scene=ride` had him airborne
    //    for the whole descent, falling *alongside* the slab he was meant
    //    to be standing on. He adopts the body's downward speed when it
    //    exceeds his own, and its horizontal drift always. Adopted, never
    //    added — this is a floor moving under him, not a push.
    //
    // 2. **The contact is already broken by the time he is stepped.**
    //    `step_chunk_bodies` runs before this, so a body that was under
    //    his feet at the top of the frame has *already* descended up to
    //    its own clamp, and the grounded probe finds empty air where the
    //    platform was. Probing a window as deep as that clamp, and
    //    settling him back onto the body when he finds it there, is what
    //    keeps a ride continuous instead of a fall punctuated by
    //    landings. The snap is a cell-scale correction of the kind
    //    `depenetrate` already makes, and only ever downward onto a
    //    surface that is genuinely there.
    let mut carrier = if p.grounded {
        (0..PLAYER_WIDTH).find_map(|dx| bodies.at(xi + dx, yi + PLAYER_HEIGHT))
    } else {
        None
    };
    if carrier.is_none() && !p.swimming && !p.grounded && p.vy >= 0.0 {
        for drop in 1..=PLATFORM_STICK {
            let row = yi + PLAYER_HEIGHT + drop;
            let Some(i) = (0..PLAYER_WIDTH).find_map(|dx| bodies.at(xi + dx, row)) else {
                continue;
            };
            // Any body that is not itself rising, rather than only one
            // already outrunning him. Requiring it to be *faster* than
            // him missed the moment that matters: when a shelf first
            // gives way, it and its passenger both start from rest, so
            // neither is faster, and by the time the slab had pulled
            // ahead it was already further than this window reaches. He
            // then spent the whole collapse in free fall beside it.
            let catchable = world.chunk_bodies.get(i).is_some_and(|b| b.vy >= 0.0);
            if catchable && rect_free(world, &bodies, xi, row - PLAYER_HEIGHT, wade) {
                p.y = (row - PLAYER_HEIGHT) as f32;
                p.grounded = true;
                carrier = Some(i);
            }
            break; // the first body below is the one he is riding, or none is
        }
    }
    if let Some(body) = carrier.and_then(|i| world.chunk_bodies.get(i)) {
        if body.vy > p.vy {
            p.vy = body.vy;
        }
        p.x += body.vx;
    }

    world.player = Some(p);
}

/// One dig bite toward `aim` — the phase-2 verb. A no-op without a
/// summoned player or while the cooldown is running. Two steps, and both
/// matter:
///
/// - **`rigid::mine`** at the bite point: cracks, detachment, structural
///   scheduling, a pressure impulse — everything that makes a cut *felt*.
///   It converts rock to rubble in place, though, so on its own a dig
///   loosens a bore without opening one.
/// - **Thinning** is what actually opens the hole. `mine` conserves cells,
///   so a bore full of rubble occupies exactly the volume the rock did and
///   nothing has been dug at all. A `dig_yield` fraction of the freshly
///   broken rock stays as rubble and falls to the floor of the bore as
///   spoil; the rest leaves as dust, thrown as particles so the material
///   is seen going rather than blinking out. See `Tuning::dig_yield` for
///   why this is arithmetic rather than a preference.
/// - **Displacement** then shoves whatever loose material remains (spoil,
///   sand, water) out of the bite to the nearest resting place beyond it —
///   the same shove-don't-delete contract `rigid::displace` keeps for
///   bodies. In a sealed pocket with nowhere free, material stays put and
///   the dig advances only by what thinning removed.
///
/// While `buried`, the bite auto-aims at the gnome's own centre whatever
/// `aim` says: the M9 "buried and dig out" escape. The displacement is
/// what actually frees him — burying material is loose by nature, and
/// shoving it out of the rectangle gives the depenetration pass somewhere
/// to stand him up.
pub fn dig(world: &mut World, aim: (i32, i32), tuning: &Tuning) -> Option<Bite> {
    let mut p = world.player.take()?;
    let bite = if p.dig_cooldown == 0 {
        p.dig_cooldown = tuning.dig_cooldown;
        p.action = SWING_FRAMES;
        // Buried, the bite auto-aims *above his head* rather than at his
        // own centre, and the difference is the whole escape. Centred on
        // himself, the disc reaches as far below his feet as above his
        // hat: on `scene=bury` that mined the stone floor he was standing
        // on into rubble, threw it clear, and dropped him into the hole —
        // he ended eight cells *lower* than he started, sealed inside the
        // floor, with the pile still on top. Aiming high puts the bore
        // between him and the surface, which is both the direction a
        // buried digger actually works and the direction `depenetrate`
        // already prefers, so the two pull the same way and he climbs out
        // a few cells per bite.
        let at = bite_point(world, &p, aim, tuning);
        let radius = tuning.dig_radius as i32;
        // The thinning used to live here, over a `fresh` set this function
        // collected before the cut. It is inside `rigid::mine` now, over
        // the cells `mine` itself broke, so the `D` key and the creatures
        // dig the same hole the gnome does -- see `rigid::mine`'s
        // `spoil_yield`. The distinction that set existed for is kept
        // there: only *freshly* broken cells are thinned, never sand that
        // was already lying in the bore, which would delete material the
        // player poured in themselves.
        // Swept from the previous bite, which is what makes a run of
        // bites a corridor instead of a row of circles -- and what stops
        // the gnome wedging in the pinch between two of them, since he is
        // 14 cells tall and that pinch measured 13. See `rigid::mine_swept`.
        //
        // Only when the last bite was near enough to be *this* bore.
        // Without the cap, walking away and digging again would carve a
        // trench across everything in between: the sweep is meant to join
        // consecutive bites at a working face, not to connect two places
        // the digger happens to have visited.
        let from = match p.last_bite {
            Some(last) if (last.0 - at.0).abs().max((last.1 - at.1).abs()) <= radius * SWEEP_REACH => last,
            _ => at,
        };
        p.last_bite = Some(at);
        let dusted = crate::sim::rigid::mine_swept(world, from, at, radius, tuning.dig_yield);
        // How far spoil may be thrown, and the two cases genuinely differ.
        //
        // A bite at a rock face only ever needs to shove material a cell or
        // two: it is standing in the open space it came in through, so the
        // near ring is free and a long search would never be reached
        // anyway. Digging *out of a burial* is the opposite problem. The
        // rectangle is full, every one of its cells has to find a home
        // before `depenetrate` can stand him up, and under a dumped pile
        // the nearest free cell is the pile's own surface. Measured on
        // `scene=bury`: at the short reach a buried gnome dug 34 bites and
        // moved zero cells — the pile had no opening within four cells of
        // the bore, so the escape simply did not exist, which is not what
        // M9 asks for ("buried by a sand dump and dig out"). The long
        // reach is the case where throwing material to the surface is
        // *also* what a digger actually does, so it costs nothing in
        // plausibility. It is still bounded: bury him deep enough and the
        // surface is out of range and he stays under, which is the right
        // answer rather than a missing feature.
        let search = if p.buried { radius + BURIED_THROW } else { radius + SPOIL_THROW };
        let displaced = displace_disc(world, &p, at.0, at.1, radius, search);
        Some(Bite { at, displaced, dusted })
    } else {
        None
    };
    world.player = Some(p);
    bite
}

// `solid_cells_in_disc` stood here: the set of cells a bite was about to
// break, collected *before* the cut so the thinning could tell fresh spoil
// from sand already lying in the bore. `rigid::mine` keeps that
// distinction itself now -- it thins the cells it actually broke -- so
// there is nothing left for a caller to pre-compute.

// `thin_to_dust` used to live here. It is `rigid::thin_to_spoil` now, so
// that every digger -- the gnome, the `D` key, and the creatures -- shares
// one spoil model rather than the gnome honouring `dig_yield` while the
// sandbox verb ignored it.

/// How far apart two bites may be, in bite radii, and still be joined
/// into one corridor rather than treated as two separate holes.
///
/// Two, so a digger walking at any sane pace keeps cutting one continuous
/// bore -- his stride between bites is well inside that -- while a jump
/// across the map starts a fresh one.
const SWEEP_REACH: i32 = 2;

/// What one shake did. Reported for the reason `Bite` is: a tree that shed
/// nothing and a shake that never fired look identical, and only a count
/// separates them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shake {
    pub at: (i32, i32),
    /// How much of the plant moved. Grabbing the trunk shakes the whole
    /// thing; grabbing a twig shakes the twig.
    pub cells: usize,
    /// Loose material knocked off the branches — snow, sand, his own spoil.
    pub dislodged: usize,
    /// Leaves that came down.
    pub shed: usize,
    /// Seed sown.
    pub seeds: usize,
}

/// Where a shake aimed at `aim` would land, or `None` if he is not pointing
/// at anything alive within reach.
///
/// **This is what decides which verb the left button is**, and it is the
/// same shape `Tool::Dig` already settled on (`app.rs`): the tool never
/// changes, the thing you are pointing at does. A reach may bound *where*
/// something happens and must never decide *whether* it happens, so
/// pointing at rock still digs — there is no dead click.
///
/// `shake_reach` is shorter than `dig_reach`, and that ordering is the
/// whole rule: a tree at arm's length outranks the cliff behind it, and a
/// tree across the clearing does not steal a mining click.
pub fn shake_target(world: &World, p: &Player, aim: (i32, i32), tuning: &Tuning) -> Option<(i32, i32)> {
    if p.buried {
        return None; // buried digs upward, whatever is out there
    }
    let at = face_toward(world, p.center(), aim, tuning.shake_reach as i32);
    let cell = world.get(at.0, at.1);
    (cell.organism_id() != 0 && world.materials.get(cell.material).climbable).then_some(at)
}

/// How much of a plant one shake reaches through — a backstop against a
/// pathological organism, not a design knob.
///
/// **Bounds work, never whether a shake happens**, and the first value
/// broke that rule in the way `CLAUDE.md` warns a size cap always can. At
/// 250 cells, grabbing the base of a 2,609-cell tree filled the flood with
/// trunk before it reached a single leaf: measured on `scene=shake` as 43
/// shakes that shed nothing and sowed nothing, off a plant that had plenty
/// of both. Shaking a tree has to shake *the tree*.
///
/// Generous now, because the cost is not what it looks like: this is one
/// flood every `dig_cooldown` frames, so a whole-tree walk is ~21,000 grid
/// reads spread over eight frames against the CA sweep's 163,840 cells
/// *per* frame.
const SHAKE_CELLS: usize = 3000;

const SHAKE_IMPULSE_RADIUS: i32 = 4;
const SHAKE_IMPULSE_STRENGTH: f32 = 6.0;

/// Grab a plant and shake it — the tree-interaction verb.
///
/// Three effects, and what they have in common is more important than any
/// of them: **not one of them removes load-bearing tissue, and not one of
/// them schedules a structural check.** That is the entire safety
/// argument. `plant.rs`'s `shed_stranded_leaves` records the measurement —
/// a single check fired mid-crown reads every branch past the support
/// search's hop limit as unsupported and converts it to deadwood, and cost
/// 772 cells against 20,213 at the same setting. Growth deliberately
/// schedules none, abscission deliberately schedules none, and neither
/// does this.
///
/// - **Loose material comes off the branches.** Snow, sand, his own spoil:
///   whatever was resting on the plant is nudged clear and the CA takes it
///   from there. Nothing is deleted and no plant cell moves.
/// - **Leaves that were already doomed come down**, at
///   `Tuning::shake_shed` weighted by the cube of darkness — abscission's
///   own graded pressure, borrowed rather than reinvented, so shaking a
///   healthy sunlit tree drops almost nothing and shaking a shaded dying
///   one rains litter. The light goes through `noon_equivalent_light`, or
///   a shake at midnight would be a defoliation event.
///
///   Shed leaves become their `breaks_into` — a `Powder` — rather than
///   being emptied the way abscission empties them. That single difference
///   is why they are *seen* to fall: they drop, pile at their angle of
///   repose, and leave litter on the ground, which is a thing this world
///   has never had.
/// - **Seed is sown**, at `Tuning::shake_seed`. Stated honestly: a tree
///   does not make seed cells today, so this is the gnome sowing from a
///   tree he shook rather than a reproduction model. It uses the same
///   `plant_tree` the `T` key does, and it is the loop `README.md` lists
///   as missing ("plants don't reseed").
pub fn shake(world: &mut World, at: (i32, i32), tuning: &Tuning) -> Option<Shake> {
    let mut p = world.player.take()?;
    if p.dig_cooldown != 0 {
        world.player = Some(p);
        return None;
    }
    p.dig_cooldown = tuning.dig_cooldown;
    p.action = SWING_FRAMES;
    world.player = Some(p);

    let organism_id = world.get(at.0, at.1).organism_id();
    if organism_id == 0 {
        return None;
    }
    let component = shaken_component(world, at, organism_id);
    world.add_pressure_impulse(at.0, at.1, SHAKE_IMPULSE_RADIUS, SHAKE_IMPULSE_STRENGTH);

    // Two passes rather than one, so what a cell sheds cannot depend on
    // whether the cell before it dropped its load.
    let mut dislodged = 0;
    for &(cx, cy) in &component {
        let above = world.get(cx, cy - 1);
        if above.organism_id() != 0 || world.materials.kind(above.material) != MaterialKind::Powder {
            continue;
        }
        // Only tissue that is out in the open sheds what is on it.
        //
        // Without this a shake churns the ground: roots are part of the
        // plant and the soil bed sits directly on top of them, so every
        // root cell read as a branch with a load of loose material on it.
        // Measured on `scene=shake` as 1,235 cells "knocked loose" in 43
        // shakes, nearly all of it soil being stirred around a root system.
        // A branch in the air has somewhere for the grain to fall from; a
        // root threaded through packed soil does not, and asking that
        // locally costs four reads.
        let in_the_open = [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .into_iter()
            .any(|(dx, dy)| world.get(cx + dx, cy + dy).material == super::material::EMPTY);
        if !in_the_open {
            continue;
        }
        // Sideways and down, never up: a shake spills what is sitting on a
        // branch, it does not throw it.
        let Some(to) = [(-1, 1), (1, 1), (0, 1), (-1, 0), (1, 0)]
            .into_iter()
            .map(|(dx, dy)| (cx + dx, cy - 1 + dy))
            .find(|&(nx, ny)| world.in_bounds(nx, ny) && world.is_empty(nx, ny))
        else {
            continue;
        };
        move_cell(world, (cx, cy - 1), to);
        dislodged += 1;
    }

    let mut shed = 0;
    for &(cx, cy) in &component {
        let cell = world.get(cx, cy);
        if super::organism::cell_type(cell.aux()) != Some(super::organism::CellType::Leaf) {
            continue;
        }
        let light = super::plant::ambient_light_above(world, cx, cy);
        let darkness = (1.0 - light / super::field::MAX_LIGHT).clamp(0.0, 1.0);
        let chance = tuning.shake_shed * darkness * darkness * darkness;
        let Some(into) = world.materials.get(cell.material).breaks_into else {
            continue;
        };
        let shades = world.materials.get(into).palette.len().max(1) as u32;
        if !world.rng.chance(chance) {
            continue;
        }
        let shade = world.rng.below(shades) as u8;
        world.set(cx, cy, super::cell::Cell::new(into, shade));
        // Reclaim any spray this stranded. **Not** a structural check — see
        // this function's own doc, and `plant::shed_stranded_leaves`.
        super::plant::shed_stranded_leaves(world, cx, cy, organism_id);
        shed += 1;
    }

    // Only a grown tree carries seed, and that gate is doing more than
    // flavour: without it, a seed he shakes loose germinates into a sapling
    // that can immediately be shaken for more seed, and holding the button
    // in a wood turns the ground into a thicket. Measured on `scene=shake`
    // before the gate: 199 shakes sowed 10 seeds and grew a bush around
    // him. A seedling has nothing to give.
    let bearing = world.organism(organism_id).is_some_and(|o| o.shoot_cells >= SEED_BEARING_CELLS);
    let seeds = usize::from(bearing && world.rng.chance(tuning.shake_seed) && sow_from_crown(world, &component));
    Some(Shake { at, cells: component.len(), dislodged, shed, seeds })
}

/// The part of the plant one shake reaches, from where it was grabbed.
///
/// **Eight neighbours, not four.** `Grow` places organism cells at all
/// eight, so a four-neighbour walk reads a grown tree as disconnected
/// fragments — `CLAUDE.md`'s "a traversal must use the same neighbourhood
/// the writer used". Sorted row-major before anything mutates, because
/// `HashSet` iteration order is per-process and was a live determinism bug
/// in `plant.rs` (5877 / 5872 / 5881 cells across three runs of one
/// binary).
fn shaken_component(world: &World, from: (i32, i32), organism_id: u16) -> Vec<(i32, i32)> {
    let mut seen = std::collections::HashSet::from([from]);
    let mut queue = std::collections::VecDeque::from([from]);
    let mut out = Vec::new();
    while let Some((x, y)) = queue.pop_front() {
        out.push((x, y));
        if out.len() >= SHAKE_CELLS {
            break;
        }
        // Eight, matching what `Grow` writes — see this function's doc.
        for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            let next = (x + dx, y + dy);
            if seen.contains(&next) {
                continue;
            }
            let cell = world.get(next.0, next.1);
            if cell.organism_id() != organism_id || !world.materials.get(cell.material).climbable {
                continue;
            }
            seen.insert(next);
            queue.push_back(next);
        }
    }
    out.sort_unstable_by_key(|&(x, y)| (y, x));
    out
}

/// Drop a seed out of the top of the shaken plant, and report whether one
/// went in. It is a `Powder`, so it falls from there on its own.
///
/// **From the crown, not from where he grabbed.** Sowing below the shaken
/// point sounds obvious and never fires: he shakes a trunk at chest height,
/// and every cell under that is the ground the tree is standing in. Seed
/// comes off the top of a tree whichever part of it you shake.
fn sow_from_crown(world: &mut World, component: &[(i32, i32)]) -> bool {
    // Row-major sorted, so the first entry is the topmost row.
    let Some(&top) = component.first() else {
        return false;
    };
    let Some((x, y)) = (1..=SOW_SEARCH)
        .map(|d| (top.0, top.1 + d))
        .find(|&(x, y)| world.in_bounds(x, y) && world.is_empty(x, y))
    else {
        return false;
    };
    // The same species the `T` key plants. There is only one tree species
    // today, and no way to ask an organism for its species *name* — worth
    // revisiting the day a second one exists, since shaking an oak should
    // not sow a pine.
    world.plant_tree_species(x, y, "tree")
}

/// How far under the shaken point to look for somewhere to drop seed.
const SOW_SEARCH: i32 = 24;

/// How much above-ground tissue a plant needs before shaking it yields
/// seed. Well past a seedling and well under a grown tree, so the gate
/// separates the two rather than gating on age, which nothing tracks.
const SEED_BEARING_CELLS: u32 = 300;

/// Where a bite aimed at `aim` would land, without digging anything.
///
/// Public, and shared with `dig` rather than reimplemented, because the
/// renderer draws this spot as the dig cursor. The first playtest reported
/// the dig as simply absent, and the deeper cause under the missing tool
/// (see `app::Tool::Dig`) is that *nothing on screen said where a cut
/// would go* — a reach-limited verb aimed with a free cursor is invisible
/// unless it shows you its own reach. Two copies of this rule would mean
/// the marker and the cut could disagree, which is worse than no marker.
pub fn bite_point(world: &World, p: &Player, aim: (i32, i32), tuning: &Tuning) -> (i32, i32) {
    let (cx, cy) = p.center();
    if p.buried {
        // Buried digs upward whatever the cursor says — see `dig`.
        let (_, y0, _, _) = p.bounds();
        return (cx, y0 - 1);
    }
    face_toward(world, (cx, cy), aim, tuning.dig_reach as i32)
}

/// What one bite actually did. Returned rather than kept private because
/// of the failure `CLAUDE.md` records under "did it fire at all needs a
/// counter": a bore full of loose rubble and a bore the dig never touched
/// are the same picture, and only a count separates them. The filmstrip
/// scenes print these next to the tile; `App` discards them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bite {
    /// Where the bite landed after reach-clamping — not where the cursor
    /// was, which is the whole point of the clamp.
    pub at: (i32, i32),
    /// Loose cells shoved clear of the bore. Zero with a non-zero bite is
    /// meaningful, not a bug: it is what a dig into open air looks like.
    pub displaced: usize,
    /// Cells that left the world as dust. This is the number that decides
    /// whether a cave is opening at all — see `Tuning::dig_yield` — so it
    /// is reported beside the image rather than inferred from it.
    pub dusted: usize,
}

/// Where a bite aimed at `aim` actually lands: the **first blocking cell
/// along the ray** from `from`, stopping at `reach`.
///
/// This started as a plain clamp of the cursor onto the circle of
/// `reach`, and that was wrong in a way worth recording, because it looks
/// right and passes an obvious test. Clicking deep inside a massif put
/// the bite *behind* the rock face, carving a sealed pocket several cells
/// in — and a sealed pocket has nowhere to put its own spoil, so the
/// stone turned to rubble in place and stayed there. On screen that is a
/// dig that does nothing: rubble and stone are near enough the same grey
/// (deliberately — see `assets/materials/rubble.ron`) that the player
/// gets no feedback at all. Digging the near face instead is what a
/// pickaxe does, keeps the cut where the player is looking, and
/// guarantees the bore always has the open space the digger came in
/// through on its far side, which is where spoil goes.
///
/// Aiming into open air within reach digs where the cursor is — the bite
/// finds nothing to displace and costs nothing, which is the right
/// outcome for a swing at nothing.
fn face_toward(world: &World, from: (i32, i32), aim: (i32, i32), reach: i32) -> (i32, i32) {
    let (dx, dy) = ((aim.0 - from.0) as f32, (aim.1 - from.1) as f32);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist == 0.0 {
        return from;
    }
    let limit = dist.min(reach as f32);
    // Sampled at whole-cell steps along the ray, like the movement
    // sweep's substepping and for the same reason: a diagonal aim must
    // not step past a one-cell-thick wall between samples.
    let steps = limit.ceil() as i32;
    let (sx, sy) = (dx / dist, dy / dist);
    let mut last = from;
    // Loose material does not stop the aim; rock does.
    //
    // This is the second half of "the material breaks but goes nowhere,
    // so you cannot really make a cave", and the half no amount of
    // tuning `dig_yield` could fix. Stopping at the first *blocking*
    // cell meant a gnome's own spoil shielded the face he was cutting:
    // after two bites the bore held rubble, rubble blocks, so every
    // later bite landed on the muck instead of the rock behind it and
    // re-broke material that was already broken. Measured over 63 bites
    // into a solid massif, the world lost 77 cells — a tunnel that could
    // not advance because the pick never reached stone again.
    //
    // So the ray reaches *past* powder to the first hard cell, which is
    // what swinging a pick over a muck pile actually does. Loose
    // material is still a valid target when there is no rock in reach —
    // otherwise digging into a dune or a drift would do nothing at all —
    // it is just never preferred over stone.
    let mut first_loose = None;
    for i in 1..=steps {
        let t = (i as f32).min(limit);
        let cell = (from.0 + (sx * t).round() as i32, from.1 + (sy * t).round() as i32);
        match footing(world, &Bodies::none(), cell.0, cell.1) {
            Footing::Hard => return cell,
            // A tree stops the *aim* even though it no longer stops the
            // body, and that split is the whole reason `Climb` is not
            // `Free`. Movement collision and aim collision are different
            // questions and this function only ever asked the first one;
            // once a trunk became passable, the ray would have looked
            // straight through it and put the dig ring on the cliff
            // behind, with no way to point at the tree in front of you.
            Footing::Climb => return cell,
            // And at a formation, for the same reason plus a sharper one:
            // the pick is aimed down this ray, so a stalagmite the ray flew
            // through would be unminable — visible, walk-through, and
            // impossible to remove.
            Footing::Scenery => return cell,
            Footing::Soft => {
                first_loose.get_or_insert(cell);
            }
            Footing::Free => {}
        }
        last = cell;
    }
    first_loose.unwrap_or(last)
}

/// Shove every loose cell in the dig disc to the nearest empty cell
/// outside it, and report how many moved — ring by ring, the same
/// nearest-opening reasoning as `rigid::displace`, rewritten here because
/// that one is private to a body's own occupancy set. The search reaches
/// a few cells past the disc so spoil lands just beyond the bore rather
/// than teleporting; a cell with nowhere to go stays where it is (never
/// deleted), which is what makes a sealed pocket genuinely undiggable
/// rather than a slow leak of material out of the world.
///
/// Only `Powder` and `Liquid` move. Solids do not, because `mine` has
/// already had its say about those — anything of its it could break is
/// rubble by the time this runs, and what is left is bedrock or another
/// material with no `breaks_into`, both of which should stop a dig.
/// `Plant` is likewise left alone: shoving one cell of an organism
/// somewhere else would tear it out of its own structure, so a root
/// currently stops a tunnel. That is a known gap, not a decision — phase
/// 4 owns planting and is where cutting through one belongs.
fn displace_disc(world: &mut World, p: &Player, cx: i32, cy: i32, radius: i32, search: i32) -> usize {
    let mut moved = 0;
    // The gnome's own cells first, then everything else.
    //
    // Not cosmetic ordering: openings run out. A bite into a tight spot
    // finds somewhere for the first several cells and nowhere for the
    // rest, so whichever cells are visited first are the ones that
    // actually move. Scanning the disc top-left to bottom-right spent
    // that budget on the rock above his head and left him sealed in —
    // a buried gnome dug for 300 ticks and never got out. His own
    // rectangle is the whole point of the bite while buried, and is
    // where the material is loosest when it is not, so it goes first.
    for pass in [true, false] {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if inside_player(p, x, y) != pass {
                    continue;
                }
                let kind = world.materials.kind(world.get(x, y).material);
                if !matches!(kind, MaterialKind::Powder | MaterialKind::Liquid) {
                    continue;
                }
                'rings: for ring in (radius + 1).max(1)..=search {
                    // Spoil only ever lands somewhere it can rest.
                    //
                    // Nearest-empty alone put *bars and L-shapes of sand
                    // hanging in open sky* above a gnome digging out of a
                    // pile: the ring walk lays cells along a perimeter,
                    // and when the whole perimeter is airborne the result
                    // is a one-cell-thick line drawn in mid-air, tracing
                    // the ring itself. It falls on the next CA tick, so
                    // it lives for a frame — and a frame is enough to
                    // read as fake, which is the standard this project
                    // holds itself to.
                    //
                    // Requiring something underneath is also the more
                    // honest rule: a digger throws spoil onto a heap, not
                    // into the air. Material with nowhere resting to go
                    // simply stays in the bore, the same conservative
                    // answer this function already gives for a sealed
                    // pocket.
                    for (rx, ry) in ring_offsets(ring) {
                        let (nx, ny) = (cx + rx, cy + ry);
                        if inside_player(p, nx, ny) {
                            continue; // not back into the gnome
                        }
                        if !world.in_bounds(nx, ny) || !world.is_empty(nx, ny) {
                            continue;
                        }
                        // `== Free`, deliberately, and do not "simplify"
                        // this to "is it passable". `Climb` is passable to
                        // the gnome and is still somewhere spoil can land:
                        // the CA genuinely rests powder on a `Plant` cell,
                        // so a branch holds a grain up whether or not a
                        // character walks through it.
                        if footing(world, &Bodies::none(), nx, ny + 1) == Footing::Free {
                            continue; // nothing under it: that is a floating cell
                        }
                        move_cell(world, (x, y), (nx, ny));
                        moved += 1;
                        break 'rings;
                    }
                }
            }
        }
    }
    moved
}

/// Move one cell wholesale, preserving everything it carries — `aux`
/// (liquid fill), shade, temperature. Written as a helper because getting
/// this wrong in one of two copies is how a dig starts manufacturing full
/// water cells out of near-empty ones.
fn move_cell(world: &mut World, from: (i32, i32), to: (i32, i32)) {
    let moving = world.get(from.0, from.1);
    world.set(to.0, to.1, moving);
    world.set(from.0, from.1, super::cell::Cell::EMPTY);
}

/// The cells exactly `ring` away in Chebyshev distance, top row first,
/// then the two sides descending, then the bottom row.
///
/// The order is the feel, not an implementation detail: first match wins,
/// so preferring the top row means spoil surfaces *above* the bore where
/// a digger would throw it, and only falls back to sideways and behind
/// when up is solid. The perimeter is walked directly rather than by
/// scanning the full square and skipping its interior, which is what this
/// did first — the same order and result, roughly a tenth of the reads,
/// and this runs inside a doubly-nested loop over the whole disc.
fn ring_offsets(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    let top = (-ring..=ring).map(move |rx| (rx, -ring));
    let sides = (-ring + 1..ring).flat_map(move |ry| [(-ring, ry), (ring, ry)]);
    let bottom = (-ring..=ring).map(move |rx| (rx, ring));
    top.chain(sides).chain(bottom)
}

fn inside_player(p: &Player, x: i32, y: i32) -> bool {
    let (x0, y0, x1, y1) = p.bounds();
    x >= x0 && x <= x1 && y >= y0 && y <= y1
}

/// Push an invaded rectangle to the nearest clear position within
/// `DEPENETRATE_REACH`, or mark the player buried. Up is preferred at
/// each distance (see `step`'s call-site comment), then sideways, then
/// down — down last because being squeezed downward through a floor gap
/// is the least expected outcome of being landed on.
fn depenetrate(world: &World, bodies: &Bodies, p: &mut Player, wade: i32) {
    let (xi, yi) = p.rect_origin();
    if rect_free(world, bodies, xi, yi, wade) {
        p.buried = false;
        return;
    }
    for d in 1..=DEPENETRATE_REACH {
        for (dx, dy) in [(0, -d), (-d, 0), (d, 0), (0, d)] {
            if rect_free(world, bodies, xi + dx, yi + dy, wade) {
                p.x += dx as f32;
                p.y += dy as f32;
                p.buried = false;
                // Whatever momentum the player had is spent shoving free.
                p.vx = 0.0;
                p.vy = 0.0;
                return;
            }
        }
    }
    p.buried = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    /// A 128x96 world with a solid stone floor across the bottom 8 rows.
    fn world_with_floor() -> World {
        let mut world = World::new(Rect::new(0, 0, 127, 95));
        for y in 88..=95 {
            for x in 0..=127 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world
    }

    fn tick(world: &mut World, input: PlayerInput) {
        step(world, input, &Tuning::default());
    }

    #[test]
    fn falls_under_gravity_and_lands_on_the_floor() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 40));
        for _ in 0..300 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.grounded, "should have landed by now");
        let (_, _, _, feet) = p.bounds();
        assert_eq!(feet, 87, "feet should rest directly on the floor at y=88");
        assert_eq!(p.vy, 0.0, "vertical speed dies on landing");
    }

    #[test]
    fn runs_right_and_stops_when_the_key_lifts() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(20, 84));
        for _ in 0..60 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let moved_to = world.player.as_ref().unwrap().x;
        assert!(moved_to > 40.0, "a second of running should cover real ground, got x={moved_to}");
        for _ in 0..30 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        assert_eq!(p.vx, 0.0, "ground friction should stop him within a few ticks");
        assert!(p.x - moved_to < 8.0, "he should not coast far after release");
    }

    #[test]
    fn a_full_jump_clears_well_over_his_own_height() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        // Settle onto the floor first so coyote/grounded are real.
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        let rest_y = world.player.as_ref().unwrap().y;
        tick(&mut world, PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() });
        let mut apex = rest_y;
        for _ in 0..120 {
            tick(&mut world, PlayerInput { jump_held: true, ..Default::default() });
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        let rise = rest_y - apex;
        // FLOATY at gravity 0.10 and impulse 2.1 is v^2/2g ~= 22 cells,
        // about 1.6 of his 14. The band is wide because the exact figure
        // is a live tunable and a preset away from changing.
        assert!((16.0..=28.0).contains(&rise), "expected a 16-28 cell jump, got {rise:.1}");
        let p = world.player.as_ref().unwrap();
        assert!(p.grounded, "should be back on the floor");
    }

    #[test]
    fn releasing_jump_early_cuts_the_height() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        let rest_y = world.player.as_ref().unwrap().y;
        tick(&mut world, PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() });
        // Hold for three ticks, then release.
        for _ in 0..3 {
            tick(&mut world, PlayerInput { jump_held: true, ..Default::default() });
        }
        let mut apex = rest_y;
        for _ in 0..120 {
            tick(&mut world, PlayerInput::default());
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        let rise = rest_y - apex;
        assert!(rise < 16.0, "a tapped jump should rise well short of a held one, got {rise:.1}");
        assert!(rise >= 3.0, "but it should still leave the ground, got {rise:.1}");
    }

    #[test]
    fn steps_up_a_low_ledge_but_not_a_wall_taller_than_his_stride() {
        let mut world = world_with_floor();
        // A ledge his stride clears, then a wall well over it.
        let step = Tuning::default().step_up as i32;
        for x in 70..=127 {
            for y in (88 - step)..88 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 100..=127 {
            for y in (88 - step * 3)..(88 - step) {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world.player = Some(Player::at(50, 84));
        for _ in 0..200 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        let (x, _) = p.rect_origin();
        assert!(x >= 70, "should have climbed the {step}-cell ledge, stuck at x={x}");
        assert!(x < 100, "should be stopped by the taller wall, got past to x={x}");
    }

    #[test]
    fn the_world_edge_is_a_wall() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(3, 84));
        for _ in 0..120 {
            tick(&mut world, PlayerInput { left: true, ..Default::default() });
        }
        let (x, _) = world.player.as_ref().unwrap().rect_origin();
        assert_eq!(x, 0, "should be pressed against the left edge, not through it");
    }

    #[test]
    fn sand_falling_into_the_rect_pushes_him_out_and_entombment_sets_buried() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        // One sand cell lands inside the rectangle: pushed free, not buried.
        let (xi, yi) = world.player.as_ref().unwrap().rect_origin();
        world.set(xi + 1, yi + PLAYER_HEIGHT - 1, Cell::new(material::SAND, 0));
        tick(&mut world, PlayerInput::default());
        let p = world.player.as_ref().unwrap();
        assert!(!p.buried, "one intruding cell should be escapable");
        let (nx, ny) = p.rect_origin();
        assert!(rect_free(&world, &Bodies::none(), nx, ny, Tuning::default().wade_rows as i32), "the rect should be clear after depenetration");

        // Entomb him completely: buried, and motionless.
        let (xi, yi) = world.player.as_ref().unwrap().rect_origin();
        for dy in -(DEPENETRATE_REACH + 1)..(PLAYER_HEIGHT + DEPENETRATE_REACH + 1) {
            for dx in -(DEPENETRATE_REACH + 1)..(PLAYER_WIDTH + DEPENETRATE_REACH + 1) {
                world.set(xi + dx, yi + dy, Cell::new(material::SAND, 0));
            }
        }
        tick(&mut world, PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() });
        let p = world.player.as_ref().unwrap();
        assert!(p.buried, "fully enclosed should read as buried");
        assert_eq!((p.vx, p.vy), (0.0, 0.0), "no movement while buried");
    }

    /// Every cell holding material, by the raw predicate rather than
    /// `is_empty` — the question here is "is there material here", which
    /// is exactly the distinction `CLAUDE.md` records against the
    /// managed-aware version.
    fn occupied_cells(world: &World) -> usize {
        let (w, h) = (128, 96);
        (0..h).map(|y| (0..w).filter(|&x| world.get(x, y).material != material::EMPTY).count()).sum()
    }

    /// Cells in the bite disc that `mine` is about to break — solid, and
    /// not the bedrock it refuses to break.
    ///
    /// A test-side replica now: the production copy went when the thinning
    /// moved into `rigid::mine`, which knows the cells it broke without
    /// anyone pre-computing them. Kept here because the assertion below is
    /// specifically "roughly `1 - dig_yield` of what was *broken* left",
    /// and that ratio needs the denominator.
    fn solid_cells_in_disc(world: &World, cx: i32, cy: i32, radius: i32) -> usize {
        let mut n = 0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let (x, y) = (cx + dx, cy + dy);
                if dx * dx + dy * dy > radius * radius || !world.in_bounds(x, y) {
                    continue;
                }
                let cell = world.get(x, y);
                if world.materials.kind(cell.material) == MaterialKind::Solid && cell.material != material::BEDROCK {
                    n += 1;
                }
            }
        }
        n
    }

    /// Stone across the right half, from the surface down — a cliff face
    /// to tunnel into, with the gnome standing on the floor beside it.
    fn world_with_cliff() -> World {
        let mut world = world_with_floor();
        for y in 60..88 {
            for x in 70..=127 {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        world
    }

    #[test]
    fn a_bite_opens_a_bore_and_removes_only_what_it_broke() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        // **Pinned to `DUST`, not the default.** Both halves of this test
        // need spoil to exist: `displaced > 0` is about the shove, and the
        // `1 - dig_yield` ratio needs a denominator that is not the whole
        // bite. The default is `CLEAN` (0.0) since the playtest, at which
        // there is nothing left to shove and the shove assertion fails for
        // a reason that has nothing to do with displacement. The default
        // path has its own case below.
        let dust = SPOIL_MODES.iter().find(|m| m.name == "DUST").expect("a DUST mode must exist");
        let tuning = Tuning { dig_yield: dust.dig_yield, ..Tuning::default() };
        let before = occupied_cells(&world);
        let bite_at = bite_point(&world, world.player.as_ref().unwrap(), (76, 78), &tuning);
        let broken = solid_cells_in_disc(&world, bite_at.0, bite_at.1, tuning.dig_radius as i32);
        let bite = dig(&mut world, (76, 78), &tuning).expect("a fresh gnome digs immediately");
        assert_eq!(bite.at.0, 70, "the bite lands on the near face, not behind it at the cursor");
        assert!(bite.displaced > 0, "biting into solid stone should shove spoil clear");
        // The bore is actually open, not merely loosened: `mine` alone
        // turns stone into rubble in place and would leave this full.
        let open = (-2..=2).filter(|dy| world.get(71, 84 + dy).material == material::EMPTY).count();
        assert!(open >= 4, "the bite should leave a hole, found {open} empty cells through its middle");
        // Conservation is deliberately gone — see `Tuning::dig_yield`.
        // What must hold instead: the only cells that left are ones the
        // bite broke, and roughly `1 - dig_yield` of them did.
        let after = occupied_cells(&world);
        let lost = before - after;
        let expected = (broken as f32 * (1.0 - tuning.dig_yield)).round() as usize;
        assert!(lost > 0, "a bite must actually remove volume, or no cave can ever open");
        assert!(
            lost.abs_diff(expected) <= 2,
            "expected to lose about {expected} of {broken} broken cells, lost {lost}"
        );
    }

    /// The shipped default, and the end of `dig_yield` that had no case:
    /// at `CLEAN` a bite opens a bore and leaves nothing behind in it.
    ///
    /// Paired with `at_full_yield_nothing_leaves_the_world` deliberately --
    /// between them they pin both ends of the range, and either alone is
    /// passable by a `mine` that ignores the number entirely.
    #[test]
    fn at_the_default_yield_a_bite_leaves_no_spoil_in_its_bore() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        assert_eq!(tuning.dig_yield, 0.0, "the shipped default is CLEAN; this case is about that end");
        let before = occupied_cells(&world);
        dig(&mut world, (76, 78), &tuning).expect("a fresh gnome digs immediately");
        let after = occupied_cells(&world);
        assert!(before > after, "a bite at CLEAN must remove volume");
        // The bore is open *and* clear: not one cell of the disc it cut is
        // occupied by anything. At DUST this row holds rubble.
        let bore: Vec<Cell> = (-2..=2).map(|dy| world.get(71, 84 + dy)).collect();
        assert!(
            bore.iter().all(|c| c.material == material::EMPTY),
            "at CLEAN the bore should be empty through its middle, found {:?}",
            bore.iter().map(|c| world.materials.get(c.material).name.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn at_full_yield_nothing_leaves_the_world() {
        // The other end of `dig_yield`, and the guard on the promise that
        // only *thinning* removes material: with nothing thinned, the dig
        // is back to the shove-don't-delete contract exactly.
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 78));
        let tuning = Tuning { dig_yield: 1.0, ..Tuning::default() };
        let before = occupied_cells(&world);
        dig(&mut world, (76, 78), &tuning).expect("digs");
        assert_eq!(before, occupied_cells(&world), "at yield 1.0 a dig may move material but never delete it");
    }

    #[test]
    fn a_bite_stops_at_the_first_face_rather_than_carving_a_sealed_pocket() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        // Aimed at the far side of the massif, well past the face and
        // still inside reach. The old clamp-to-reach rule bit here and
        // left a pocket of rubble buried in the rock.
        let bite = dig(&mut world, (79, 84), &Tuning::default()).expect("digs");
        assert_eq!(bite.at.0, 70, "expected the face at x=70, bit at {:?}", bite.at);
        assert!(
            world.get(79, 84).material == material::STONE,
            "the rock behind the face must be untouched, found {:?}",
            world.materials.get(world.get(79, 84).material).name
        );
    }

    #[test]
    fn the_cooldown_rate_limits_held_digging() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        assert!(dig(&mut world, (76, 84), &tuning).is_some(), "first bite fires");
        assert!(dig(&mut world, (76, 84), &tuning).is_none(), "a held button must not bite every frame");
        for _ in 0..tuning.dig_cooldown {
            tick(&mut world, PlayerInput::default());
        }
        assert!(dig(&mut world, (76, 84), &tuning).is_some(), "the cooldown should expire on its own ticks");
    }

    #[test]
    fn a_cursor_across_the_map_still_digs_the_near_face() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let bite = dig(&mut world, (127, 84), &Tuning::default()).expect("a far click still digs");
        assert_eq!(bite.at.0, 70, "a click across the map digs the wall in front of him");
    }

    #[test]
    fn aimed_at_open_sky_the_bite_stops_at_reach() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        let tuning = Tuning::default();
        // Straight up into empty sky: nothing blocks, so the ray runs out
        // at reach rather than following the cursor to the top of the world.
        let bite = dig(&mut world, (64, 0), &tuning).expect("digs");
        let (cx, cy) = world.player.as_ref().unwrap().center();
        let d = (((bite.at.0 - cx).pow(2) + (bite.at.1 - cy).pow(2)) as f32).sqrt();
        assert!(
            (d - tuning.dig_reach as f32).abs() <= 1.0,
            "expected a bite at reach ({}), landed {d:.1} away",
            tuning.dig_reach
        );
        assert_eq!(bite.displaced, 0, "a swing at empty air moves nothing");
    }

    #[test]
    fn a_buried_gnome_digs_himself_out() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        // Entomb him in sand, as a dumped pile would.
        let (xi, yi) = world.player.as_ref().unwrap().rect_origin();
        for dy in -(DEPENETRATE_REACH + 2)..(PLAYER_HEIGHT + DEPENETRATE_REACH + 2) {
            for dx in -(DEPENETRATE_REACH + 2)..(PLAYER_WIDTH + DEPENETRATE_REACH + 2) {
                if yi + dy < 88 {
                    world.set(xi + dx, yi + dy, Cell::new(material::SAND, 0));
                }
            }
        }
        tick(&mut world, PlayerInput::default());
        assert!(world.player.as_ref().unwrap().buried, "the pile should bury him");
        let before = occupied_cells(&world);

        // Dig with the cursor pointed somewhere useless: buried aims at
        // himself regardless, which is the whole escape.
        let tuning = Tuning::default();
        let mut bites = 0;
        for _ in 0..300 {
            if dig(&mut world, (0, 0), &tuning).is_some() {
                bites += 1;
            }
            tick(&mut world, PlayerInput::default());
            if !world.player.as_ref().unwrap().buried {
                assert!(bites > 0, "he got out without ever digging — the test is not testing the dig");
                assert_eq!(before, occupied_cells(&world), "digging out must not delete the pile");
                return;
            }
        }
        panic!("still buried after 300 ticks of digging");
    }

    #[test]
    fn a_dig_never_buries_the_digger() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        let wade = tuning.wade_rows as i32;
        for i in 0..40 {
            // Legal *before* the bite, so the assertion after it is about
            // the bite and not about where he had already walked.
            let (bx, by) = world.player.as_ref().unwrap().rect_origin();
            if !rect_free(&world, &Bodies::none(), bx, by, wade) {
                continue;
            }
            dig(&mut world, (74, 84), &tuning);
            let (ax, ay) = world.player.as_ref().unwrap().rect_origin();
            assert!(
                rect_free(&world, &Bodies::none(), ax, ay, wade),
                "bite {i} shoved spoil into a position the gnome was standing in"
            );
            tick(&mut world, PlayerInput::default());
        }
    }

    /// A stone floor with a living tree trunk standing on it: a real
    /// organism id, `wood` material, `MatureBody` cells.
    ///
    /// Hand-built rather than grown. `plant_tree` plants a seed and takes
    /// thousands of frames to make anything, and what it makes is a
    /// different shape every time — these tests are about the collision
    /// predicate, not about tree architecture, so the trunk is placed where
    /// the assertion needs it.
    fn world_with_tree(x0: i32, width: i32, top: i32) -> World {
        let mut world = world_with_floor();
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        let species = world.species.id_of("tree").expect("tree is compiled in");
        let organism = world.push_organism(species);
        let aux = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::MatureBody);
        for y in top..88 {
            for x in x0..(x0 + width) {
                world.set(x, y, Cell::new(wood, 0).with_organism_id(organism).with_aux(aux));
            }
        }
        world
    }

    #[test]
    fn he_runs_straight_through_a_living_tree() {
        // The complaint this whole change answers: "we get stuck because we
        // cannot jump over or get around trees."
        let mut world = world_with_tree(64, 5, 60);
        world.player = Some(Player::at(30, 80));
        for _ in 0..300 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.x > 80.0, "expected him past the trunk at x=64..69, stopped at x={:.1}", p.x);
    }

    #[test]
    fn a_painted_wooden_wall_still_stops_him() {
        // Same material, same `Plant` kind, no organism: a wall someone
        // built. If this passes only because `wood` is walk-through, the
        // rule has been stated as the material alone and building is broken.
        let mut world = world_with_floor();
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        for y in 60..88 {
            for x in 64..69 {
                world.set(x, y, Cell::new(wood, 0));
            }
        }
        world.player = Some(Player::at(30, 80));
        for _ in 0..300 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.x < 64.0, "a painted wooden wall must still be a wall, but he reached x={:.1}", p.x);
    }

    #[test]
    fn a_cave_formation_is_scenery_he_walks_past_and_can_still_mine() {
        // Both halves in one test on purpose, because the owner's ruling was
        // "all background, and breakable too" and each half is trivially
        // satisfiable by breaking the other: a formation that blocks is
        // certainly minable, and one made of air is certainly walk-through.
        //
        // No `organism_id` anywhere here -- that is the point of the split.
        // `flowstone` carries `scenery` on the material, so it needs no
        // per-cell gate, and because mining is what gates on `organism_id`
        // (`rigid::mine_swept`), leaving it zero is what keeps the pick
        // working. Giving a formation an organism id to reuse `climbable`
        // would have taken the pick away *and* routed it into the
        // amputating structural path.
        let mut world = world_with_floor();
        let flowstone = world.materials.id_of("flowstone").expect("flowstone is compiled in");
        // A column standing floor to well above his head, right across his
        // path -- the shape that used to be a wall.
        for y in 60..88 {
            for x in 64..69 {
                world.set(x, y, Cell::new(flowstone, 0).with_attached(true));
            }
        }
        world.player = Some(Player::at(30, 80));
        for _ in 0..300 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.x > 72.0, "a formation must be scenery he walks past, but he stalled at x={:.1}", p.x);

        // And it is still rock: the pick takes it. `mine_swept` filters on
        // `is_body_material` and `organism_id`, both of which a formation
        // passes, so this needs no special case -- the assertion is that no
        // future one is added.
        let before = (60..88).filter(|&y| world.get(66, y).material == flowstone).count();
        assert!(before > 0, "the column should still be standing before it is mined");
        crate::sim::rigid::mine_swept(&mut world, (66, 70), (66, 70), 3, 0.0);
        let after = (60..88).filter(|&y| world.get(66, y).material == flowstone).count();
        assert!(after < before, "the pick must cut a formation: {before} cells before, {after} after");
    }

    #[test]
    fn a_formation_is_not_something_he_can_climb() {
        // `Footing::Scenery` exists rather than reusing `Climb` for exactly
        // this: `grip` tests the climb variant by equality, so folding the
        // two together would let him haul himself up a stalagmite. Walking
        // past a formation and climbing a tree are different affordances.
        let mut world = world_with_floor();
        let flowstone = world.materials.id_of("flowstone").expect("flowstone is compiled in");
        for y in 40..88 {
            for x in 60..70 {
                world.set(x, y, Cell::new(flowstone, 0).with_attached(true));
            }
        }
        world.player = Some(Player::at(64, 80));
        let start = world.player.as_ref().unwrap().y;
        for _ in 0..200 {
            tick(&mut world, PlayerInput { jump_held: true, grab: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.y >= start - 4.0, "he climbed a formation to y={:.1} from {start:.1}", p.y);
    }

    #[test]
    fn a_tree_growing_over_him_cannot_entomb_him() {
        // Found by measurement, not predicted: on `filmstrip scene=wood`
        // the gnome was reported BURIED from frame 4708 to the end of the
        // run, having travelled 0 cells. He had not walked into a trunk at
        // all -- a crown grew over the spot he was standing on, and
        // `depenetrate` read living tissue as an invasion with no clear
        // push in any direction, so a tree buried him where he stood.
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 80));
        tick(&mut world, PlayerInput::default());
        let (x0, y0) = world.player.as_ref().unwrap().rect_origin();
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        let species = world.species.id_of("tree").expect("tree is compiled in");
        let organism = world.push_organism(species);
        let aux = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::MatureBody);
        // Grown right through him and well past his depenetration reach on
        // every side, so there is nowhere to be shoved to.
        for y in (y0 - 8)..(y0 + PLAYER_HEIGHT + 8) {
            for x in (x0 - 8)..(x0 + PLAYER_WIDTH + 8) {
                world.set(x, y, Cell::new(wood, 0).with_organism_id(organism).with_aux(aux));
            }
        }
        for _ in 0..60 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(!p.buried, "a tree is not a burial");
        assert!(p.x > x0 as f32 + 4.0, "he should have walked out of it, moved {:.1}", p.x - x0 as f32);
    }

    /// Walk right until he has hold of the trunk, then stop steering.
    ///
    /// Holding `right` *while* climbing shimmies him sideways at
    /// `climb_speed` and out of a five-cell trunk in about seven ticks --
    /// which is correct (that is how you leave a tree) and is not what
    /// these tests are about.
    fn grab_the_trunk(world: &mut World) {
        let mut input = PlayerInput { right: true, grab: true, jump_pressed: true, ..Default::default() };
        for _ in 0..60 {
            tick(world, input);
            input.jump_pressed = false;
            if world.player.as_ref().unwrap().climbing {
                return;
            }
        }
        panic!("never got hold of the trunk");
    }

    #[test]
    fn jump_walking_through_a_wood_grabs_nothing() {
        // **The bug, stated.** Reported from the first playtest: "if I am
        // just jump walking in a forest, I can basically fly/hover."
        // Climbing keyed on `W`, which is also jump, so every trunk he
        // clipped at the top of an arc grabbed him and kept lifting --
        // land, jump, grab, rise, repeat, indefinitely upward.
        //
        // A tall stand of tissue he runs through with `W` held and no grab
        // key. He must never climb, and must never end up above the height
        // one jump can reach from the floor.
        let mut world = world_with_tree(50, 60, 20);
        world.player = Some(Player::at(20, 80));
        let floor_y = 88.0 - PLAYER_HEIGHT as f32;
        let mut highest = floor_y;
        let mut input = PlayerInput { right: true, jump_held: true, jump_pressed: true, ..Default::default() };
        for i in 0..600 {
            tick(&mut world, input);
            // Re-press regularly: this is someone jumping their way across
            // a wood, not one jump held forever.
            input.jump_pressed = i % 25 == 0;
            let p = world.player.as_ref().unwrap();
            assert!(!p.climbing, "he grabbed a tree without asking to, at tick {i}");
            highest = highest.min(p.y);
        }
        // One jump from the floor rises ~22 cells at the default feel. A
        // hover shows up as *repeated* gain, so anything much past a single
        // jump is the bug.
        assert!(
            floor_y - highest < 30.0,
            "he climbed the wood by jumping through it: floor {floor_y:.1}, reached {highest:.1}"
        );
    }

    #[test]
    fn letting_go_drops_him_rather_than_launching_him() {
        // The two exits from a tree have to differ. Climbing off the top
        // with `W` held springs him off the crown, which is worth keeping;
        // releasing the grab key with `W` still held must not fire that
        // same jump, or letting go flings you upward.
        let mut world = world_with_tree(64, 5, 20);
        world.player = Some(Player::at(60, 80));
        grab_the_trunk(&mut world);
        for _ in 0..40 {
            tick(&mut world, PlayerInput { grab: true, jump_held: true, ..Default::default() });
        }
        let let_go_at = world.player.as_ref().unwrap().y;
        // Shift released, `W` still down.
        let mut highest = let_go_at;
        for _ in 0..20 {
            tick(&mut world, PlayerInput { jump_held: true, ..Default::default() });
            highest = highest.min(world.player.as_ref().unwrap().y);
        }
        let p = world.player.as_ref().unwrap();
        assert!(!p.climbing, "releasing the grab key should let go");
        assert!(highest >= let_go_at - 0.5, "letting go flung him upward: {let_go_at:.1} -> apex {highest:.1}");
        assert!(p.y > let_go_at, "letting go should drop him: {let_go_at:.1} -> {:.1}", p.y);
    }

    #[test]
    fn holding_up_against_a_trunk_climbs_it() {
        let mut world = world_with_tree(64, 5, 20);
        world.player = Some(Player::at(60, 80));
        grab_the_trunk(&mut world);
        let start = world.player.as_ref().unwrap().y;
        for _ in 0..40 {
            tick(&mut world, PlayerInput { grab: true, jump_held: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.climbing, "he should still be on the trunk, at y={:.1}", p.y);
        assert!(start - p.y > 30.0, "expected a real climb, rose {:.1} cells in 40 ticks", start - p.y);
    }

    #[test]
    fn letting_go_of_everything_leaves_him_gripped() {
        // Grip, not a pole slide. The difference between a climb and a
        // fireman's pole, and the owner's call: the whole tree is a hold.
        let mut world = world_with_tree(64, 5, 20);
        world.player = Some(Player::at(60, 80));
        grab_the_trunk(&mut world);
        for _ in 0..20 {
            tick(&mut world, PlayerInput { grab: true, jump_held: true, ..Default::default() });
        }
        let held = world.player.as_ref().unwrap().y;
        // Still holding on, just not climbing: the grip.
        for _ in 0..120 {
            tick(&mut world, PlayerInput { grab: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.climbing, "holding on with no vertical input should hang him, not drop him");
        assert!((p.y - held).abs() < 1.0, "he should hang where he was left: {held:.1} -> {:.1}", p.y);
    }

    #[test]
    fn a_falling_gnome_does_not_catch_himself_on_a_tree_he_is_not_holding() {
        // Why the grip needs an intent rather than mere contact. Engaging
        // on overlap alone made a tree flypaper: a gnome falling *past* one
        // stopped dead in the canopy, measured arriving at row 48 against
        // the floor at 88. It is also the pure-ladder guard -- no part of a
        // tree is ever a floor.
        let mut world = world_with_tree(60, 12, 40);
        world.player = Some(Player::at(64, 20));
        for _ in 0..400 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        assert!(!p.climbing, "he grabbed a tree nobody told him to grab");
        assert_eq!(p.y.round() as i32 + PLAYER_HEIGHT, 88, "he should have fallen through to the floor");
    }

    #[test]
    fn standing_on_the_ground_the_jump_key_still_jumps() {
        // The guard that stops a root system eating the jump key. Rootwood
        // is climbable -- it had to be, or a root through a bank would
        // still be a wall -- he wades four rows into soft ground, and a
        // wood is full of roots. `GRIP_ROWS` is what keeps a handhold at
        // chest height and above, so roots round the boots are not one.
        let mut world = world_with_floor();
        let rootwood = world.materials.id_of("rootwood").expect("rootwood is compiled in");
        let species = world.species.id_of("tree").expect("tree is compiled in");
        let organism = world.push_organism(species);
        let aux = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::RootTip);
        for y in 84..88 {
            for x in 50..80 {
                world.set(x, y, Cell::new(rootwood, 0).with_organism_id(organism).with_aux(aux));
            }
        }
        world.player = Some(Player::at(64, 80));
        for _ in 0..30 {
            tick(&mut world, PlayerInput::default());
        }
        let before = world.player.as_ref().unwrap().y;
        let mut apex = before;
        let mut input = PlayerInput { jump_held: true, jump_pressed: true, ..Default::default() };
        for _ in 0..40 {
            tick(&mut world, input);
            input.jump_pressed = false;
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        assert!(apex < before - 10.0, "a jump over roots must be a jump, not a grab: {before:.1} -> apex {apex:.1}");
    }

    #[test]
    fn walking_out_of_a_tree_drops_him() {
        let mut world = world_with_tree(64, 5, 20);
        world.player = Some(Player::at(60, 80));
        grab_the_trunk(&mut world);
        for _ in 0..30 {
            tick(&mut world, PlayerInput { grab: true, jump_held: true, ..Default::default() });
        }
        // Sideways, still holding on, nothing on the vertical axis: he
        // shimmies out of the tissue and gravity takes over again.
        for _ in 0..300 {
            tick(&mut world, PlayerInput { grab: true, right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(!p.climbing, "he walked out of the tissue and should have let go");
        assert_eq!(p.y.round() as i32 + PLAYER_HEIGHT, 88, "and should be back on the floor");
    }

    #[test]
    fn a_canopy_breaks_a_fall_without_stopping_it() {
        // **Paired**, per `CLAUDE.md`: the same drop with and without the
        // foliage, so the comparison cancels everything the rule is not
        // about. A single run against a remembered arrival speed would be a
        // sample from a distribution.
        fn drop_through(leaves: bool) -> f32 {
            let mut world = world_with_floor();
            if leaves {
                let leaf = world.materials.id_of("leaf").expect("leaf is compiled in");
                let species = world.species.id_of("tree").expect("tree is compiled in");
                let organism = world.push_organism(species);
                let aux = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::Leaf);
                for y in 40..70 {
                    for x in 50..80 {
                        world.set(x, y, Cell::new(leaf, 0).with_organism_id(organism).with_aux(aux));
                    }
                }
            }
            world.player = Some(Player::at(64, 10));
            // **Arrival speed, not peak speed.** The first version took the
            // maximum `vy` over the whole fall and measured 2.80 against
            // 2.25 -- almost all of which was the thirty cells of open air
            // *above* the crown, where nothing is meant to be different.
            // What a canopy changes is how hard he lands.
            let mut impact = 0.0;
            for _ in 0..400 {
                let before = world.player.as_ref().unwrap().vy;
                tick(&mut world, PlayerInput::default());
                let p = world.player.as_ref().unwrap();
                if p.grounded {
                    impact = before;
                    break;
                }
            }
            let p = world.player.as_ref().unwrap();
            assert_eq!(p.y.round() as i32 + PLAYER_HEIGHT, 88, "he must still reach the floor, leaves={leaves}");
            impact
        }
        let bare = drop_through(false);
        let through_leaves = drop_through(true);
        assert!(
            through_leaves < bare * 0.6,
            "a crown should take real speed off a fall: bare landing at {bare:.2} cells/tick, through leaves {through_leaves:.2}"
        );
    }

    /// A stone floor with a wall on the right whose top is at `lip_y`.
    /// `capped` decides whether there is anything to stand on up there.
    fn world_with_lip(lip_y: i32, capped: bool) -> World {
        let mut world = world_with_floor();
        let bottom = if capped { lip_y } else { 0 };
        for y in bottom..88 {
            for x in 80..=127 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world
    }

    #[test]
    fn a_lip_just_out_of_reach_at_the_apex_is_mantled() {
        // A wall a couple of cells taller than a full jump clears: his
        // feet reach row 66 at the apex against a lip at 64. Without the
        // mantle he bonks and slides back down it.
        let mut world = world_with_lip(64, true);
        world.player = Some(Player::at(60, 80));
        let mut input = PlayerInput { right: true, jump_held: true, jump_pressed: true, ..Default::default() };
        for _ in 0..200 {
            tick(&mut world, input);
            input.jump_pressed = false;
        }
        let p = world.player.as_ref().unwrap();
        assert!(
            p.grounded && p.y.round() as i32 + PLAYER_HEIGHT <= 64,
            "expected him up on the ledge at row 64, feet at {} grounded {}",
            p.y.round() as i32 + PLAYER_HEIGHT,
            p.grounded
        );
    }

    #[test]
    fn a_wall_with_no_top_is_not_mantled() {
        // The clause that keeps the mantle from being a wall climb, and the
        // one that makes the reach safe to raise. A face with nothing to
        // stand on offers no footing at any lift, so nothing fires against
        // it however hard he pushes.
        let mut world = world_with_lip(0, false);
        world.player = Some(Player::at(60, 80));
        let mut input = PlayerInput { right: true, jump_held: true, jump_pressed: true, ..Default::default() };
        let mut apex = world.player.as_ref().unwrap().y;
        for i in 0..400 {
            tick(&mut world, input);
            // Re-press every so often, so he is jumping at it repeatedly
            // rather than getting one attempt.
            input.jump_pressed = i % 30 == 0;
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.x < 80.0, "he climbed a wall he should not have: x={:.1}", p.x);
        assert!(apex > 40.0, "he ratcheted up the face: reached y={apex:.1}");
    }

    #[test]
    fn a_dead_drop_beside_a_ledge_does_not_snap_him_onto_it() {
        // Pushing into it is required. Falling past a ledge with no input
        // must read as falling past it.
        let mut world = world_with_lip(64, true);
        world.player = Some(Player::at(76, 20));
        for _ in 0..300 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        assert_eq!(p.y.round() as i32 + PLAYER_HEIGHT, 88, "he should have fallen to the floor beside the ledge");
    }

    /// A stone floor with a leafy tree on it, and a gnome beside it.
    fn world_with_leafy_tree() -> World {
        let mut world = world_with_floor();
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        let leaf = world.materials.id_of("leaf").expect("leaf is compiled in");
        let species = world.species.id_of("tree").expect("tree is compiled in");
        let organism = world.push_organism(species);
        let stem = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::MatureBody);
        let foliage = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::Leaf);
        // Small enough that one shake reaches all of it — about 230 cells
        // against `SHAKE_CELLS`. These tests are about what a shake *does*,
        // not about where the work cap bites, and a tree big enough to
        // truncate the component would make every count depend on which
        // half of it the flood happened to cover.
        for y in 70..88 {
            for x in 70..74 {
                world.set(x, y, Cell::new(wood, 0).with_organism_id(organism).with_aux(stem));
            }
        }
        for y in 62..77 {
            for x in 66..78 {
                if world.get(x, y).organism_id() == 0 {
                    world.set(x, y, Cell::new(leaf, 0).with_organism_id(organism).with_aux(foliage));
                }
            }
        }
        world.player = Some(Player::at(64, 80));
        world
    }

    #[test]
    fn shaking_a_tree_schedules_no_structural_check() {
        // **The tripwire, and the reason it is written first.** The organism
        // support search is hop-bounded, so any structural check fired
        // mid-crown reads every branch past the limit as unsupported and
        // converts it to deadwood -- measured at 772 cells against 20,213
        // from one scheduled check (`plant.rs`'s `shed_stranded_leaves`).
        // A shake reaches deep into a crown by design, so it is exactly the
        // shape of disturbance that landmine is waiting for.
        use crate::sim::scheduler::ActiveKind;
        let mut world = world_with_leafy_tree();
        let before = world.active_sites_for_test().iter().filter(|s| s.kind == ActiveKind::StructuralCheck).count();
        let target = shake_target(&world, world.player.as_ref().unwrap(), (127, 87), &Tuning::default()).expect("aimed at the tree");
        shake(&mut world, target, &Tuning::default()).expect("a shake should fire");
        let after = world.active_sites_for_test().iter().filter(|s| s.kind == ActiveKind::StructuralCheck).count();
        assert_eq!(before, after, "a shake must schedule no structural check");
    }

    #[test]
    fn a_shake_never_removes_a_wood_cell() {
        // Nothing load-bearing moves, which is the other half of the same
        // safety argument: if no stem cell goes, nothing can be discovered
        // unsupported.
        let mut world = world_with_leafy_tree();
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        let count = |w: &World| (60..88).flat_map(|y| (60..84).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == wood).count();
        let before = count(&world);
        let tuning = Tuning { shake_shed: 1.0, ..Default::default() };
        for _ in 0..40 {
            let Some(target) = shake_target(&world, world.player.as_ref().unwrap(), (127, 87), &tuning) else {
                break;
            };
            shake(&mut world, target, &tuning);
            world.player.as_mut().unwrap().dig_cooldown = 0;
        }
        assert_eq!(before, count(&world), "a shake took wood out of the tree");
    }

    #[test]
    fn shaking_a_tree_drops_what_is_resting_on_its_branches() {
        let mut world = world_with_leafy_tree();
        let sand = material::SAND;
        // A drift settled across the top of the crown.
        for x in 66..78 {
            world.set(x, 61, Cell::new(sand, 0));
        }
        let tuning = Tuning::default();
        let target = shake_target(&world, world.player.as_ref().unwrap(), (127, 87), &tuning).expect("aimed at the tree");
        let shake_result = shake(&mut world, target, &tuning).expect("a shake should fire");
        assert!(shake_result.dislodged > 0, "nothing came off the branches: {shake_result:?}");
    }

    #[test]
    fn a_shaded_tree_sheds_more_than_a_sunlit_one() {
        // **Paired**, and it is the mechanism that is being tested rather
        // than the rate: the shed borrows abscission's cube-of-darkness
        // pressure, so a healthy lit crown should barely lose anything and
        // a dark one should rain litter.
        fn shed_with(light: f32) -> usize {
            let mut world = world_with_leafy_tree();
            // `noon_equivalent_light` clamps to `MAX_LIGHT`, so flooding
            // the crown reads as full sun whatever phase of the day the
            // oscillator is at — which is the point of going through it.
            world.add_light(72, 70, 24, light);
            let tuning = Tuning { shake_shed: 1.0, ..Default::default() };
            let target = shake_target(&world, world.player.as_ref().unwrap(), (127, 87), &tuning).expect("aimed at the tree");
            shake(&mut world, target, &tuning).expect("a shake should fire").shed
        }
        let dark = shed_with(0.0);
        let lit = shed_with(100.0);
        assert!(dark > lit * 4, "a shaded crown should shed far more than a lit one: dark {dark}, lit {lit}");
    }

    #[test]
    fn pointing_at_rock_still_digs_and_pointing_at_a_tree_shakes() {
        // One button, two verbs, and no dead click: `Tool::Dig`'s recorded
        // lesson is that a reach may bound where a verb lands and must
        // never decide whether it happens.
        let world = world_with_leafy_tree();
        let p = world.player.as_ref().unwrap();
        let tuning = Tuning::default();
        assert!(shake_target(&world, p, (127, 87), &tuning).is_some(), "pointing at the trunk should shake");
        assert!(shake_target(&world, p, (64, 92), &tuning).is_none(), "pointing at the floor should dig");
    }

    #[test]
    fn an_ant_does_not_stop_him() {
        // A single creature cell used to be a wall, which was defensible
        // only while a whole tree was one as well.
        let mut world = world_with_floor();
        let ant = world.materials.id_of("ant").expect("ant is compiled in");
        let species = world.species.id_of("ant").expect("ant is compiled in");
        let organism = world.push_organism(species);
        let aux = crate::sim::organism::pack_cell_type(crate::sim::organism::CellType::Head);
        for y in 74..88 {
            world.set(70, y, Cell::new(ant, 0).with_organism_id(organism).with_aux(aux));
        }
        world.player = Some(Player::at(30, 80));
        for _ in 0..300 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.x > 90.0, "a column of ants should not be a wall, he stopped at x={:.1}", p.x);
    }

    #[test]
    fn a_nest_still_stops_him() {
        // The other half: what a colony *builds* is `kind: Solid` and is as
        // solid as it ever was. Without this the change reads as "creature
        // things are passable", which is not what it says.
        let mut world = world_with_floor();
        let nest = world.materials.id_of("nest").expect("nest is compiled in");
        for y in 60..88 {
            for x in 70..74 {
                world.set(x, y, Cell::new(nest, 0));
            }
        }
        world.player = Some(Player::at(30, 80));
        for _ in 0..300 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.x < 70.0, "a nest wall must still be a wall, but he reached x={:.1}", p.x);
    }

    #[test]
    fn the_dig_aim_still_stops_at_a_tree_he_can_walk_through() {
        // Movement collision and aim collision are different questions.
        // Without the split, the ray looks straight through the trunk and
        // the dig ring lands on whatever is behind it.
        let world = world_with_tree(80, 5, 60);
        let mut world = world;
        world.player = Some(Player::at(60, 80));
        let p = world.player.take().expect("just placed");
        let at = bite_point(&world, &p, (127, 80), &Tuning::default());
        world.player = Some(p);
        assert_eq!(at.0, 80, "the aim should stop at the near face of the trunk, landed at {at:?}");
    }

    /// A pool of water in a stone basin, surface at `surface_y`.
    fn world_with_pool(surface_y: i32) -> World {
        let mut world = world_with_floor();
        for y in surface_y..88 {
            for x in 40..90 {
                world.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        world
    }

    /// `world_with_pool`, plus a stone bank on the right whose top stands
    /// `lip` cells clear of the waterline — the situation the exit hop
    /// exists for, and one no existing scene contained.
    fn world_with_bank(surface_y: i32, lip: i32) -> World {
        let mut world = world_with_pool(surface_y);
        for y in (surface_y - lip)..=95 {
            for x in 90..=127 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world
    }

    #[test]
    fn he_sinks_into_a_deep_drift_but_only_to_the_knee() {
        let mut world = world_with_floor();
        // A deep, flat bed of sand — not a pile, so there is no slope for
        // step-up to climb and the only question is how far in he goes.
        for y in 70..88 {
            for x in 0..=127 {
                world.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        world.player = Some(Player::at(64, 40));
        for _ in 0..200 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        let (_, _, _, feet) = p.bounds();
        let wade = Tuning::default().wade_rows as i32;
        // The surface is y=70, so resting *on* it puts his feet at 69.
        // Wading `wade` rows sinks him exactly that far in, and no
        // further — the point of the rule is that a drift has a bottom.
        assert_eq!(feet, 69 + wade, "expected to sink {wade} rows into the bed, feet at {feet}");
        assert!(p.wading, "standing in sand should read as wading");
        assert!(p.grounded, "sand still holds him up");
        assert!(!p.buried, "knee-deep is not buried");
    }

    #[test]
    fn wading_is_slower_than_running_on_rock() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(10, 84));
        for _ in 0..90 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let on_rock = world.player.as_ref().unwrap().x - 10.0;

        let mut world = world_with_floor();
        for y in 84..88 {
            for x in 0..=127 {
                world.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        world.player = Some(Player::at(10, 80));
        for _ in 0..90 {
            tick(&mut world, PlayerInput { right: true, ..Default::default() });
        }
        let through_sand = world.player.as_ref().unwrap().x - 10.0;
        assert!(
            through_sand < on_rock * 0.75,
            "wading ({through_sand:.1}) should cost real speed against running ({on_rock:.1})"
        );
        assert!(through_sand > 0.0, "but he should still be able to move");
    }

    #[test]
    fn he_sinks_in_water_and_strokes_bring_him_back_up() {
        let mut world = world_with_pool(60);
        // Dropped in from above, so he arrives with real downward speed
        // and the damping has something to eat.
        world.player = Some(Player::at(64, 30));
        let mut deepest = 0.0f32;
        for _ in 0..400 {
            tick(&mut world, PlayerInput::default());
            deepest = deepest.max(world.player.as_ref().unwrap().y);
        }
        assert!(deepest > 60.0, "he should actually get into the water, deepest {deepest:.1}");
        // The DIVER default: left alone he keeps sinking, so the vertical
        // axis is the player's job. Holding W must undo that.
        let sank_to = world.player.as_ref().unwrap().y;
        for _ in 0..200 {
            tick(&mut world, PlayerInput { jump_held: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(
            p.y < sank_to - 6.0,
            "strokes should climb against a sinking default: {sank_to:.1} -> {:.1}",
            p.y
        );
    }

    #[test]
    fn strokes_drive_him_down_and_the_cooldown_paces_them() {
        let mut world = world_with_pool(60);
        world.player = Some(Player::at(64, 62));
        for _ in 0..30 {
            tick(&mut world, PlayerInput::default());
        }
        let floating = world.player.as_ref().unwrap().y;
        for _ in 0..120 {
            tick(&mut world, PlayerInput { down: true, ..Default::default() });
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.swimming, "still in the pool");
        assert!(p.y > floating + 4.0, "holding S should pull him under: {floating:.1} -> {:.1}", p.y);
    }

    #[test]
    fn breaking_the_surface_leaves_a_window_to_jump_out() {
        let mut world = world_with_pool(60);
        world.player = Some(Player::at(64, 62));
        for _ in 0..60 {
            tick(&mut world, PlayerInput::default());
        }
        // Floating at the surface: a jump press must fire a real jump,
        // which is what the swimming coyote refresh exists for.
        //
        // Measured at the *apex*, not at the end of the run. The first
        // version of this read his position 40 ticks later and failed on
        // a gnome who had jumped clear, arced, and splashed back in —
        // which is the behaviour it exists to check, reported as its
        // absence.
        //
        // **Pressed once, then held.** This read `jump_pressed: true` on
        // every one of the forty ticks, and passed — against an input the
        // running game cannot produce, since `App::update` clears
        // `jump_pressed` after each tick and a held key raises no further
        // edge. So the test was green while the behaviour it is named for
        // did not work: holding `W` to surface (which the default `DIVER`
        // buoyancy makes the only way to surface) left nothing armed, and
        // he bobbed at the bank. `CLAUDE.md`'s "a test can pass because the
        // code under it is dead", in the costume of an impossible input.
        let before = world.player.as_ref().unwrap().y;
        let mut apex = before;
        let mut input = PlayerInput { jump_pressed: true, jump_held: true, ..Default::default() };
        for _ in 0..40 {
            tick(&mut world, input);
            input.jump_pressed = false;
            apex = apex.min(world.player.as_ref().unwrap().y);
        }
        assert!(
            apex < before - 6.0,
            "expected a real jump out of the water, not a stroke: {before:.1} -> apex {apex:.1}"
        );
    }

    #[test]
    fn a_held_stroke_carries_him_out_of_the_pool_onto_the_bank() {
        // The owner's report, as a test: "getting out of water should have
        // a little jump to it so you can get over a ledge."
        //
        // The input is the thing that matters here. `jump_pressed` fires on
        // *one* tick, because `App::update` clears it after every tick and
        // a held key produces no further edge — so this is what holding `W`
        // actually delivers to `step`, and it is what the old code had no
        // answer to.
        let mut world = world_with_bank(60, 3);
        world.player = Some(Player::at(80, 70));
        let mut input = PlayerInput { right: true, jump_held: true, jump_pressed: true, ..Default::default() };
        for _ in 0..400 {
            tick(&mut world, input);
            input.jump_pressed = false;
        }
        let p = world.player.as_ref().unwrap();
        let feet = p.y.round() as i32 + PLAYER_HEIGHT;
        assert!(
            p.grounded && feet <= 60,
            "expected him out of the water and standing on the bank, got feet at {feet} (waterline 60), grounded {}",
            p.grounded
        );
    }

    /// **Litter is a powder that does not impede him, and both halves of
    /// that have to hold.** A guard rather than a comment because the whole
    /// mechanism is one `#[serde(default)]` field in a `.ron` -- deletable
    /// by accident, and silent when deleted, since litter would simply go
    /// back to being ordinary powder and the failure is a gameplay feel
    /// rather than a panic.
    ///
    /// Asserted against `sand` in the same breath, so this cannot pass by
    /// `footing` having stopped distinguishing powders at all -- which is
    /// exactly how a guard for a flag like this goes vacuous.
    #[test]
    fn litter_is_a_powder_the_gnome_runs_straight_through() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let litter = w.materials.id_of("litter").expect("litter is compiled in");
        assert!(
            w.materials.get(litter).insubstantial,
            "litter.ron must keep `insubstantial: true` -- see Material::insubstantial"
        );
        assert_eq!(
            w.materials.kind(litter),
            MaterialKind::Powder,
            "it has to stay a powder: it still falls, piles and rots"
        );
        w.set(10, 10, Cell::new(litter, 0));
        w.set(12, 10, Cell::new(material::SAND, 0));
        assert_eq!(footing(&w, &Bodies::none(), 10, 10), Footing::Free, "a drift of leaves is not something to wade through");
        assert_eq!(footing(&w, &Bodies::none(), 12, 10), Footing::Soft, "sand still is -- otherwise this guard proves nothing");
    }

    #[test]
    fn a_chunk_body_is_something_he_can_stand_on() {
        use crate::sim::rigid::{BodyCell, ChunkBody};
        let mut world = world_with_floor();
        // A slab hanging in mid-air as a body, well above the floor.
        let mut cells = Vec::new();
        for dx in 0..24 {
            for dy in 0..3 {
                cells.push(BodyCell { dx, dy, material: material::STONE, shade: 0 });
            }
        }
        world.chunk_bodies.push(ChunkBody::at(cells, 50.0, 60.0));
        world.player = Some(Player::at(60, 40));
        for _ in 0..120 {
            tick(&mut world, PlayerInput::default());
        }
        let p = world.player.as_ref().unwrap();
        let (_, _, _, feet) = p.bounds();
        assert!(p.grounded, "he should have landed on the slab, not fallen through it");
        assert!(
            feet < 62,
            "expected to be standing on the slab's top at y=60, feet at {feet} (the floor is at 88)"
        );
    }

    #[test]
    fn he_rides_a_falling_slab_down_instead_of_being_left_behind() {
        use crate::sim::rigid::{self, BodyCell, ChunkBody};
        let mut world = World::new(Rect::new(0, 0, 127, 255));
        for y in 248..=255 {
            for x in 0..=127 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let mut cells = Vec::new();
        for dx in 0..24 {
            for dy in 0..3 {
                cells.push(BodyCell { dx, dy, material: material::STONE, shade: 0 });
            }
        }
        // Already at `rigid`'s fall clamp of 6 — faster than the gnome's
        // own 4, which is the case that used to leave him behind. Placed
        // in *contact*: his feet at 38, the slab's top at 39. That is
        // what `PLATFORM_STICK` is sized for — one body step opens a gap
        // of exactly the clamp, and no more. A passenger who starts a
        // cell clear of a slab already at full speed is not catching it,
        // and should not.
        world.chunk_bodies.push(ChunkBody::falling(cells, 50.0, 39.0, 6.0));
        world.player = Some(Player::at(60, 36));

        // Twelve ticks, not the whole descent, and the bound is physical
        // rather than chosen: bodies *tumble*. `spin` accumulates over a
        // fall and this slab tips onto its end partway down, at which
        // point a 24-wide platform becomes 3 wide and genuinely is no
        // longer under him. Being thrown off a slab that rolls is right,
        // so the assertion covers the stretch before it rolls.
        let tuning = Tuning::default();
        let start_y = world.player.as_ref().unwrap().y;
        let mut grounded_ticks = 0;
        for _ in 0..12 {
            // Both, in `App::update`'s order: bodies move, then he does.
            // The order is the whole difficulty — see `step`'s comment.
            rigid::step_chunk_bodies(&mut world);
            step(&mut world, PlayerInput::default(), &tuning);
            if world.player.as_ref().unwrap().grounded {
                grounded_ticks += 1;
            }
        }
        let p = world.player.as_ref().unwrap();
        assert_eq!(grounded_ticks, 12, "he should be standing on the slab throughout the flat part of the ride");
        // The load-bearing measurement: he descended further than falling
        // could possibly have carried him. His own terminal speed is
        // `fall_clamp` (4/tick, so 48 in twelve ticks) and the slab's is
        // `rigid`'s 6, so anything past 48 can only be the platform
        // carrying him. This is what fails if the carry is removed and
        // the collision check left in.
        let fell = p.y - start_y;
        let own_terminal = tuning.fall_clamp * 12.0;
        assert!(
            fell > own_terminal + 8.0,
            "expected to be carried past his own terminal fall ({own_terminal}), descended only {fell:.0}"
        );
        // And standing on *it*, not the floor far below.
        if let Some(top) = world.chunk_bodies.first().map(|b| b.y.round() as i32) {
            let (_, _, _, feet) = p.bounds();
            assert!((feet - top).abs() <= 2, "expected his feet at the slab top {top}, found {feet}");
        }
    }

    /// The presets and `Tuning::default` are two copies of the same
    /// numbers, and a drift between them would mean a fresh gnome
    /// silently differs from the feel named on screen — the exact class
    /// of bug the status line exists to prevent.
    #[test]
    fn the_defaults_are_the_first_feel_of_each_list() {
        let d = Tuning::default();
        let m = &MOVEMENT_FEELS[0];
        let w = &WATER_FEELS[0];
        let mut from_presets = d;
        m.apply(&mut from_presets);
        w.apply(&mut from_presets);
        assert_eq!(d, from_presets, "Tuning::default must equal MOVEMENT_FEELS[0] + WATER_FEELS[0]");
        assert_eq!(d.dig_yield, SPOIL_MODES[0].dig_yield, "and dig_yield must equal SPOIL_MODES[0]");
    }

    #[test]
    fn coyote_allows_a_late_jump_off_a_ledge() {
        let mut world = world_with_floor();
        // Floor only under the left half; a cliff edge at x=64.
        for y in 88..=95 {
            for x in 64..=127 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        world.player = Some(Player::at(56, 84));
        for _ in 0..20 {
            tick(&mut world, PlayerInput::default());
        }
        // Run off the edge, then jump 3 ticks after leaving the ground.
        let mut off_ground_ticks = 0;
        for _ in 0..200 {
            let jump = off_ground_ticks == 3;
            let p = world.player.as_ref().unwrap();
            if !p.grounded {
                off_ground_ticks += 1;
            }
            tick(
                &mut world,
                PlayerInput { right: true, jump_pressed: jump, jump_held: jump, ..Default::default() },
            );
            if world.player.as_ref().unwrap().vy < -1.0 {
                return; // the late jump fired: rising fast — pass
            }
        }
        panic!("a jump 3 ticks after walking off the ledge should still fire (coyote)");
    }
}
