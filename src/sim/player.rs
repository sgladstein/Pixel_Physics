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

/// Character extent in cells. 7x14 against the 512x320 viewport (`app::WIDTH`
/// / `HEIGHT`, unchanged since the world outgrew it) puts him at about a
/// twenty-third of what's on screen at a time — gnome-scale beside trees that are
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
/// How far up his body a handhold still counts, in cells.
///
/// **A function of his height, not a constant**, because he is not always
/// `PLAYER_HEIGHT` tall: a world generated at a finer `cell_scale` builds
/// him proportionally bigger, and a grip that stayed at 7 rows would be
/// waist-high on the authored gnome and knee-high on a doubled one. See
/// `Player::at_scaled`.
fn grip_rows(h: i32) -> i32 {
    h / 2
}

/// How long the swing pose stays up after a blow, **at the defaults**.
///
/// Half `dig_cooldown`, so held digging alternates swing and stance rather
/// than sticking in one or flickering between them — the rhythm of blows the
/// cooldown already produces, made visible.
///
/// **No longer read by the code, on purpose.** `dig` and `shake` compute
/// `dig_cooldown / 2` instead, so the documented relationship holds by
/// construction rather than by coincidence: as a literal it silently stopped
/// being half the cooldown the moment the cooldown became tunable in the
/// panel, and again when `Tuning::dilated` let the gnome's whole rhythm
/// slow down — the swing pose would have kept flickering at real-time speed
/// over a slowed dig. Kept as the recorded default and checked against the
/// derivation by `the_swing_pose_is_half_the_dig_cooldown_at_the_defaults`.
#[cfg(test)]
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
    /// Horizontal speed cap, cells per tick. The default (`Tuning::default`,
    /// 1.5, not the 1.3 an earlier version of this comment named) is 90
    /// cells/s at the sandbox's fixed 60 Hz, which crosses the current
    /// 8192-cell-wide world in 8192 / 90 ≈ 91 seconds end to end.
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
    /// How much loose powder **in any one row** above the wade line he
    /// shoulders past instead of stopping against, in cells.
    ///
    /// **This exists because scattered grains were a wall.** `wade_rows`
    /// says powder may reach his knees and no higher, which is the right
    /// claim about walking into a *drift* and the wrong one about the
    /// stray grains a forest floor, a dug tunnel or a splash leaves lodged
    /// in a canopy. Measured in `scene=wood`: first a single `soil` cell
    /// at (108,194) held him for eleven thousand frames, and behind it a
    /// scatter of four to seven more, spread one and two to a row over
    /// (112..116, 180..187). None of them is an obstacle; together they
    /// were a fence.
    ///
    /// **Per row, not per rect, and the distinction is the whole fix.** A
    /// drift is made of *courses* — its face fills rows across his whole
    /// width — while a scatter is one or two cells in each of several
    /// rows. A rect-wide total cannot tell seven scattered grains from one
    /// full course of seven, so it has to be set below a course to keep a
    /// drift solid, which leaves it too small to clear a scatter; measured
    /// that way the gnome got 98 cells through the wood at 0 and 103 at 6.
    /// Counting per row separates them cleanly at every setting, which is
    /// why the panel's whole range is safe: at 6 a full course of 7 still
    /// stops him, so `he_sinks_into_a_deep_drift_but_only_to_the_knee`
    /// holds however this is tuned.
    ///
    /// **4 from a sweep, not from taste**, over six start frames of
    /// `scene=wood` (the stand takes no seed, so the frame window is the
    /// axis that redraws it). Cells travelled, min and median across the
    /// six:
    ///
    /// | allowance | min | median |
    /// |---|---|---|
    /// | 0 (the old veto) | 1 | 44 |
    /// | 2 | 44 | 93 |
    /// | 3 | 49 | 94 |
    /// | **4** | **50** | **161** |
    /// | 5 | 51 | 166 |
    /// | 6 | 47 | 47 |
    ///
    /// 6 is a cliff, not a plateau's end: one short of a full course, he
    /// sinks into the forest floor instead of walking over it and does
    /// worse than the veto. 4 and 5 are indistinguishable on the worst
    /// case, so 4 takes the one with two steps of margin from that edge
    /// rather than one.
    ///
    /// **Confirmed by playtest, blind**, which is the half a sweep cannot
    /// answer: the distance is a number, but "does he now look like he is
    /// cheating through solid ground" is not. Shown against the old veto
    /// as an unlabelled A/B of the same walk (review card
    /// `20260823T082002879Z-a61726`), the owner picked this one.
    ///
    /// 0 restores the old veto exactly, for A/B.
    pub shoulder_grains: u8,
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
    /// How deep a slice one bore stroke takes off the working face, in
    /// cells.
    ///
    /// **The bore is a box; this is the bite out of it.** `bore_rect` sizes
    /// a passage to the gnome and shows it before you cut, and cutting the
    /// whole box in one press would be a room appearing rather than a hole
    /// being dug — the binary outcome `CLAUDE.md`'s first law rejects. Three
    /// cells against a 9-wide box is three strokes to advance one body
    /// width, which at `dig_cooldown` 8 is about half a second of visible
    /// work per step forward.
    pub bore_bite: u8,
    /// How far from his centre a hammer blow can land, in cells.
    ///
    /// Much shorter than `dig_reach` and shorter than `shake_reach`: a
    /// pick is a tool you extend and a hammer is one you swing, and the
    /// reach is what makes the difference legible without a second cursor.
    pub hammer_reach: u8,
    /// Radius of one hammer blow, handed to `rigid::strike` — which floors
    /// it at `rigid::MIN_STRIKE_RADIUS` (6), so settings under that are
    /// the same blow.
    ///
    /// A blow is not a bite: `strike` pulverizes a core, chips a shell and
    /// **opens the rock's own joints well past both**, which is the
    /// damage that accumulates and eventually drops a ceiling. That reach
    /// is `radius * rigid::CRACK_REACH`, so this number sizes the
    /// fissuring far more than it sizes the hole.
    pub hammer_radius: u8,
    /// How hard the blow throws what it breaks, handed to `rigid::strike`
    /// as its fragment impulse and scaled into the pressure it shoves into
    /// the air.
    ///
    /// **12, up from 3, and the reason is that a piece that does not
    /// travel does not read as a piece.** `promote` turns this into a
    /// fragment's launch speed as `force / distance_from_the_blow`, so at
    /// 3 a chunk five cells out left the wound at 0.6 cells a frame —
    /// slower than it then fell. Measured on `scene=smash`, fastest piece
    /// 1.07 -> 4.02 cells per frame. Reported from playtest as *"I don't
    /// see pieces coming off in chunks"*, against a census that said 226
    /// cells had come off as chunks on the same run: they were coming off
    /// and going nowhere. The recoil is its own number
    /// (`hammer_recoil`), so this does not touch how far the swing
    /// shoves *him*.
    pub hammer_force: f32,
    /// Ticks between hammer blows. Long — three times `dig_cooldown` at
    /// the defaults — because a heavy swing that repeats at pick speed
    /// reads as a drill, and because the recoil below wants room to be
    /// seen.
    pub hammer_cooldown: u8,
    /// How hard a landed blow shoves the gnome back, in cells per tick.
    ///
    /// The half of "smashing" that the rock cannot supply: a blow that
    /// moves the world and not the arm swinging it reads as a cursor
    /// effect. Small — a fifth of `run_max` — so it is felt on the ground
    /// and can be walked straight back through, and it only fires on a
    /// blow that actually broke something.
    pub hammer_recoil: f32,
    /// How far he can reach to chop, in cells. Between the hammer's reach
    /// and the pick's: an axe is longer than a hammer and shorter than a
    /// pick swung at a rock face.
    pub chop_reach: u8,
    /// Radius of one chop. Small on purpose — a notch, not a bite. Felling
    /// a bole is several chops into the same face, which is what makes the
    /// moment it goes over readable as *your* last stroke rather than as
    /// the tree deciding.
    pub chop_radius: u8,
    /// Ticks between chops.
    pub chop_cooldown: u8,
    /// What fraction of chopped tissue stays as timber where it fell; the
    /// rest goes as chips.
    ///
    /// `dig_yield`'s sibling and deliberately far higher (half against a
    /// tenth). The two numbers answer different questions: mining has to
    /// *remove volume* or a bore cannot open at all (see `dig_yield`),
    /// while chopping is meant to leave you the wood. Some still has to
    /// go, or the notch fills with its own chips and the axe stops
    /// reaching timber — exactly the failure `face_toward`'s loose-material
    /// note records for the pick.
    pub chop_yield: f32,
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
    /// **0.10 is the default, set by playtest** — shown `CLEAN`, `TRACE`
    /// and `DUST` as animations of the same 42 bites, the owner picked
    /// `TRACE`. (`CLEAN` was the answer to the previous, coarser card and
    /// shipped briefly; the finer comparison overturned it.) An earlier
    /// version of this comment argued the
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

impl Tuning {
    /// **The same character, moving at `scale` times the pace.** Not a
    /// different character: jump height, reach, dig radius and every other
    /// distance are untouched, and only the rate at which the trajectory is
    /// traversed changes.
    ///
    /// This is a *dimensional* rescale, not a multiply-everything. Position
    /// advances by a velocity each tick and velocity by an acceleration, so
    /// under a time scale `s`:
    ///
    /// - **velocities take one factor of `s`** — `run_max`, `jump_impulse`,
    ///   `fall_clamp`, `climb_speed`, `stroke_impulse`;
    /// - **accelerations take two** — `gravity`, `run_accel`,
    ///   `ground_decel`. This is what preserves shape: apex is `v^2 / 2g`,
    ///   and `(sv)^2 / 2(s^2 g)` is the same height;
    /// - **a per-tick retention takes the power `s`** — `swim_damp`, because
    ///   it compounds. `d^s` applied `1/s` times as often is `d`. Getting
    ///   this backwards as `d^(1/s)` was caught in review and is not
    ///   cosmetic: `0.84^8` is 0.248 where `0.84^(1/8)` is 0.979, and
    ///   terminal velocity in water would have scaled as `s^2` instead of
    ///   `s`;
    /// - **durations divide by `s`**, so a forgiveness window covers the
    ///   same amount of *motion*;
    /// - **distances, sizes and dimensionless ratios do not move at all** —
    ///   `dig_reach`, `dig_radius`, `bore_bite`, `step_up`, `wade_rows`,
    ///   `mantle_reach`, `shake_reach`, `hammer_reach`, `hammer_radius`,
    ///   `hammer_force`, `chop_reach`, `chop_radius`, `air_control`,
    ///   `buoyancy`, `wade_slowdown`, `surface_hop`, `dig_yield`,
    ///   `chop_yield`, and the two shake probabilities.
    ///
    /// # Derived, never written back
    ///
    /// The stored `Tuning` stays the un-dilated source of truth and this
    /// returns a copy. That is required rather than tidy: `MovementFeel::
    /// apply` (`F3`) overwrites six of these fields with absolute values, so
    /// a knob that mutated the stored struct would be silently wiped by the
    /// next feel cycle — and `the_defaults_are_the_first_feel_of_each_list`
    /// pins `Tuning::default()` to `MOVEMENT_FEELS[0]` and would fail on the
    /// spot. It also keeps `MovementFeel::jump_cells()`'s on-screen number
    /// honest, since that reads the stored values.
    ///
    /// # What it does not preserve
    ///
    /// The apex is preserved by the *formula* and not quite by the
    /// integrator: `player::step` is semi-implicit Euler, so the achieved
    /// height is `v^2/2g + v/2` and the second term carries one factor of
    /// `s`. At the defaults a jump reaches 23.10 cells at `1x` and 22.18 at
    /// `8x` — about nine tenths of a cell lower. Against `step_up` (4) and
    /// `mantle_reach` (4) that is a playtestable difference rather than a
    /// rounding one, which is why this ships as a knob to be judged in the
    /// hand rather than as a silent default.
    ///
    /// Durations stay `u8`, which is safe only because `clock::MAX_SLOWDOWN`
    /// is 30: `dig_cooldown` reaches 240 of 255 at the cap. Raising that cap
    /// means widening these.
    pub fn dilated(&self, scale: f32) -> Self {
        if scale >= 1.0 {
            return *self;
        }
        let slow = 1.0 / scale;
        let dur = |d: u8| ((d as f32 * slow).round() as u32).min(u8::MAX as u32) as u8;
        Self {
            // Accelerations: two factors.
            gravity: self.gravity * scale * scale,
            run_accel: self.run_accel * scale * scale,
            ground_decel: self.ground_decel * scale * scale,
            // Velocities: one.
            run_max: self.run_max * scale,
            jump_impulse: self.jump_impulse * scale,
            fall_clamp: self.fall_clamp * scale,
            climb_speed: self.climb_speed * scale,
            stroke_impulse: self.stroke_impulse * scale,
            // A compounding per-tick retention.
            swim_damp: self.swim_damp.powf(scale),
            // Windows, in ticks.
            coyote_frames: dur(self.coyote_frames),
            jump_buffer_frames: dur(self.jump_buffer_frames),
            dig_cooldown: dur(self.dig_cooldown),
            hammer_cooldown: dur(self.hammer_cooldown),
            chop_cooldown: dur(self.chop_cooldown),
            stroke_cooldown: dur(self.stroke_cooldown),
            // A velocity, so one factor -- the recoil has to carry him the
            // same *distance* however slowly time is running, or a dilated
            // hammer would shove him across the screen.
            hammer_recoil: self.hammer_recoil * scale,
            // Everything else is a distance, a size or a ratio.
            ..*self
        }
    }
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
            shoulder_grains: 4,
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
            bore_bite: 3,
            // Well inside `PLAYER_WIDTH`/2 + a swing: the blow lands on the
            // face he is standing at, never across a gap.
            hammer_reach: 12,
            hammer_radius: 7,
            hammer_force: 12.0,
            hammer_cooldown: 24,
            hammer_recoil: 0.3,
            chop_reach: 16,
            chop_radius: 3,
            chop_cooldown: 10,
            chop_yield: 0.5,
            // Mirrors SPOIL_MODES[0] (TRACE), chosen by playtest.
            dig_yield: 0.10,
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
/// Ordered from the built default outward, and the tour is deliberately
/// not monotone: `TRACE` first, then down to `CLEAN`, then up through
/// `DUST`, `SPOIL` and `HOARD`. One press either side of the default
/// reaches both of the things a player is most likely to want next -- none
/// at all, or noticeably more -- and cycling back is always possible.
///
/// **`TRACE` is first on a playtest verdict, and it is the second answer to
/// the same question.** `CLEAN` was picked first and shipped; shown CLEAN,
/// TRACE and DUST side by side as animations, the answer was TRACE. Both
/// verdicts are in `.git/pixel-physics-review`; the second supersedes the
/// first and neither was argued for here.
pub struct SpoilMode {
    pub name: &'static str,
    pub note: &'static str,
    pub dig_yield: f32,
}

pub const SPOIL_MODES: [SpoilMode; 5] = [
    // **The gap between 0.0 and 0.35 was too wide, and the owner said so.**
    // Judging CLEAN against DUST, the verdict was "most of the options
    // produce too much dust... if there was a 10% option that would be
    // interesting, but 1/3 is even too much". The list stepped 0 -> 35 ->
    // 55 -> 100, so there was nothing between "no rubble at all" and a
    // third -- which measured on `scene=tunnel` is enough to wade in from
    // the nineteenth bite and be buried by the thirtieth. A tenth is where
    // "you can see where you dug" lives without the bore filling in behind
    // you, and shown all three it is what was chosen.
    SpoilMode { name: "TRACE", note: "a tenth stays - enough to see where you dug", dig_yield: 0.10 },
    SpoilMode { name: "CLEAN", note: "rock simply goes; no rubble at all", dig_yield: 0.0 },
    SpoilMode { name: "DUST", note: "a third stays as rubble, the rest blows away", dig_yield: 0.35 },
    SpoilMode { name: "SPOIL", note: "half stays - tunnels silt up behind you", dig_yield: 0.55 },
    SpoilMode { name: "HOARD", note: "nothing is lost - you cannot dig far", dig_yield: 1.0 },
];

/// What the gnome has in his hands. One left button, three verbs, and the
/// belt is what picks between them.
///
/// **Three tools rather than three keys**, and that is the same rule
/// `Tool::Dig` records for the dig itself: a verb the player cannot see is
/// a verb that does not exist. Every letter on the keyboard was already
/// bound when this landed (`main.rs`'s `Y` comment: "the last free
/// letter"), so a fourth and fifth binding for smashing and chopping would
/// have gone somewhere nobody would find. A belt is one selector, named on
/// screen in the gnome HUD, and the sprite carries the implement so the
/// answer to "what will this click do" is on the character rather than in
/// a corner.
///
/// The split between them is what each one is *for*, not how hard it hits:
///
/// - `Pick` **moves you through the world**. It opens passages, and its
///   default cut is sized to the body that has to walk down them.
/// - `Hammer` **breaks the world**. It removes less rock than the pick and
///   damages far more of it — the cracks reach `rigid::CRACK_REACH` times
///   the swing — so it is what brings a ceiling down rather than what digs
///   a corridor.
/// - `Axe` **cuts what is alive**. Living tissue is the one thing the
///   other two are bad at: the pick's yield is set for opening rock and
///   would turn a tree into chips, and a hammer swung at a trunk scores
///   cracks into wood that does not carry load the way stone does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tool {
    /// The pick. Digging, and the default a fresh gnome arrives with.
    #[default]
    Pick,
    /// The hammer. `rigid::strike` — a blow, with the crack reach and the
    /// failure licence a blow carries.
    Hammer,
    /// The axe. Chops living tissue, and fells what it cuts through.
    Axe,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::Pick, Tool::Hammer, Tool::Axe];

    pub fn next(self) -> Self {
        match self {
            Tool::Pick => Tool::Hammer,
            Tool::Hammer => Tool::Axe,
            Tool::Axe => Tool::Pick,
        }
    }

    /// Short enough for a HUD row of three.
    pub fn label(self) -> &'static str {
        match self {
            Tool::Pick => "PICK",
            Tool::Hammer => "HAMMER",
            Tool::Axe => "AXE",
        }
    }

    /// What it is for, in the player's words — the toast, and the help page.
    pub fn note(self) -> &'static str {
        match self {
            Tool::Pick => "cuts passages you can walk down",
            Tool::Hammer => "smashes rock and cracks what it does not break",
            Tool::Axe => "chops living wood - cut a bole through and it falls",
        }
    }
}

/// The shape the pick cuts.
///
/// **`Bore` is the default and `Free` is the option**, which is the
/// reverse of what shipped first and is the owner's call. Free-hand cutting
/// puts a round bite wherever the cursor is, and what it produces is a
/// wandering worm-hole whose clearance you discover by walking into it —
/// the failure `rigid::mine_swept`'s own doc records from the other end
/// ("why are you digging tunnels a row of circles instead of a tunnel"),
/// fixed there for the *pinch* between bites and not for the shape.
///
/// A bore asks a much smaller question of the player — which of four ways
/// — and answers a much bigger one for them: the passage is the size of
/// the thing that has to walk down it, its floor is flush with the ground
/// under his feet, and the box is drawn before it is cut. `Free` stays
/// because sculpting a chamber, undercutting a slab, or reaching a seam
/// two cells wide are all things a rectangle cannot do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DigStyle {
    /// A passage the gnome's own size, driven up, down, left or right.
    #[default]
    Bore,
    /// The cursor-aimed round bite: `rigid::mine_swept` at `dig_radius`,
    /// swept from the last bite. What the pick did before the bore existed.
    Free,
}

impl DigStyle {
    pub fn next(self) -> Self {
        match self {
            DigStyle::Bore => DigStyle::Free,
            DigStyle::Free => DigStyle::Bore,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DigStyle::Bore => "BORE",
            DigStyle::Free => "FREE",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            DigStyle::Bore => "a passage his own size, up down left or right",
            DigStyle::Free => "a round bite wherever you point",
        }
    }
}

/// Which of the four ways a bore is driven.
///
/// Cardinal and not continuous, deliberately. The cursor picks a
/// *direction* rather than a point, so the same gesture always produces
/// the same box — which is what makes the preview a promise instead of an
/// estimate, and what makes a corridor come out straight without the
/// player steering it cell by cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// The cardinal direction from `from` toward `aim`, by dominant axis.
    ///
    /// Ties go to the horizontal, which is not arbitrary: a gnome standing
    /// on the ground wants to dig *along* it far more often than through
    /// the floor, and a diagonal aim that is ambiguous by one cell should
    /// not flip the box between two very different cuts as the mouse
    /// jitters.
    pub fn toward(from: (i32, i32), aim: (i32, i32)) -> Self {
        let (dx, dy) = (aim.0 - from.0, aim.1 - from.1);
        if dx.abs() >= dy.abs() {
            if dx < 0 {
                Dir::Left
            } else {
                Dir::Right
            }
        } else if dy < 0 {
            Dir::Up
        } else {
            Dir::Down
        }
    }

    /// `toward`, but **the cursor decides, not the geometry**.
    ///
    /// Reported from the first playtest of the bore: *"the direction
    /// switches too easy as the gnome moves, he changes position relative to
    /// the mouse and then changes how he is digging. It needs to be a more
    /// intentional mouse movement."*
    ///
    /// Nothing was wrong with the arithmetic. The bug is that direction was
    /// a pure function of the *current* offset, and the player never touched
    /// the mouse — **the gnome walked**, the vector from him to a stationary
    /// cursor swept through the diagonal, and a corridor became a shaft
    /// under a hand that had done nothing.
    ///
    /// **Hysteresis on the offset was tried first and cannot fix it**, which
    /// is worth recording because it is the obvious repair: keep the held
    /// direction unless the other axis wins by a margin. Walk far enough and
    /// the other axis wins by any margin you care to name — measured, a
    /// gnome who walks level with a fixed cursor ends at dx 2, dy 11, and no
    /// ratio-and-floor rule keeps that pointing sideways without also
    /// refusing a genuine re-point. The quantity being tested was the wrong
    /// one.
    ///
    /// So this asks the question the complaint actually asks: **has the
    /// player moved the mouse since they chose this direction?** Inside
    /// `REAIM_DEADZONE` of the cursor position the direction was last set
    /// at, the held direction stands however far the gnome has walked.
    /// Outside it, the cursor is somewhere new and the raw aim decides.
    ///
    /// **`aim` is a world position, and the camera moves — which is fine,
    /// and worth writing down so nobody re-derives it.** `Renderer::follow`
    /// holds the camera still while the gnome is inside a `span / 6` dead
    /// zone and then *re-centres*, so a cursor held still on screen is a
    /// world-fixed cursor for the whole of that zone. That is precisely the
    /// reported case, and the latch covers it. The re-centring jump does
    /// move the world aim far enough to release the latch — but it moves it
    /// by the same distance the walk just took off the offset, so
    /// `Dir::toward` hands back the direction the latch was holding. The
    /// latch lets go exactly where letting go costs nothing.
    pub fn sticky(from: (i32, i32), aim: (i32, i32), held: Option<(Dir, (i32, i32))>) -> Self {
        match held {
            Some((dir, set_at)) if (aim.0 - set_at.0).abs().max((aim.1 - set_at.1).abs()) < REAIM_DEADZONE => dir,
            _ => Dir::toward(from, aim),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Dir::Up => "UP",
            Dir::Down => "DOWN",
            Dir::Left => "LEFT",
            Dir::Right => "RIGHT",
        }
    }
}

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
    /// Ticks until the next blow of any kind may land. Sim state rather
    /// than UI state, so a replayed input sequence digs on the same ticks.
    ///
    /// **One timer for all three tools**, charged to whichever tool struck
    /// — so switching mid-swing cannot be used to hit at the sum of two
    /// rates, which is what a per-tool timer would have allowed the moment
    /// the belt landed.
    swing_cooldown: u8,
    /// What `swing_cooldown` was charged to, so the HUD can draw how far
    /// through the recovery he is. Zero before the first blow.
    swing_span: u8,
    /// What he is holding. Sim state for the reason `facing_left` is: the
    /// renderer draws the implement, and the renderer must stay a pure
    /// function of the world.
    pub tool: Tool,
    /// Whether the pick cuts a passage or a free-hand bite.
    pub dig_style: DigStyle,
    /// Which way the bore is being driven, **and the cursor position that
    /// chose it** — held across frames so that walking cannot change it and
    /// only moving the mouse can. See `Dir::sticky`. `None` until the first
    /// bore, when the raw aim decides.
    pub bore_dir: Option<(Dir, (i32, i32))>,
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
    /// His body, in cells — `PLAYER_WIDTH`x`PLAYER_HEIGHT` times the world's
    /// `cell_scale`. See `Player::at_scaled`.
    pub w: i32,
    pub h: i32,
}

impl Tuning {
    /// This same feel in a world built at `k` times the cell resolution.
    ///
    /// **The gnome's body is not the only thing about him measured in
    /// cells.** Scaling him without this makes him the right size and half
    /// the character: at `k=2` he would accelerate at the same cells per
    /// tick squared, which is *half* the physical acceleration, cap out at
    /// half the physical speed, jump half as high off the ground and step
    /// over ledges half as tall. He would look right and play wrong.
    ///
    /// **The same four classes as `WorldgenParams::scaled`**, which is not a
    /// coincidence — it is what a tuning struct is:
    ///
    /// | | factor | |
    /// |---|---|---|
    /// | a length in cells | `k` | `step_up`, `dig_radius`, `wade_rows` |
    /// | a speed or acceleration, cells per tick | `k` | `run_max`, `gravity`, `jump_impulse` |
    /// | dimensionless — a ratio, a probability, a multiple of another field | `1` | `air_control`, `dig_yield`, `buoyancy` |
    /// | a time in ticks | `1` | `coyote_frames`, `dig_cooldown` |
    ///
    /// **The arithmetic is checkable rather than a matter of taste**, and
    /// checking it is what says the two speed rows belong together. Jump
    /// height is `v²/2g`; scaling both `jump_impulse` and `gravity` by `k`
    /// gives `(kv)²/(2kg) = k·v²/2g` — `k` times as many cells, which is the
    /// *same physical height*. Time to apex is `v/g = kv/kg`, unchanged. So
    /// he jumps the same height in the same time, which is the only answer
    /// that means anything.
    ///
    /// **`buoyancy` is the trap of the set** and the same shape as
    /// `strata_tilt` in the worldgen struct: it reads like an acceleration
    /// and its doc says "vertical acceleration in water" — but it is stated
    /// *as a multiple of `gravity`*, so `gravity` scaling carries it and
    /// scaling it again would double-count. `surface_hop` (a fraction of a
    /// standing jump) and `air_control` (a multiplier on `run_accel`) are
    /// the same case.
    pub fn scaled(&self, k: f32) -> Self {
        // Exhaustive, no `..`, for the reason `WorldgenParams::scaled` gives:
        // a field added later stops compiling here until somebody says which
        // class it is, and nothing else would catch getting it wrong.
        let Self {
            gravity,
            run_accel,
            run_max,
            ground_decel,
            air_control,
            jump_impulse,
            fall_clamp,
            coyote_frames,
            jump_buffer_frames,
            step_up,
            dig_reach,
            dig_radius,
            dig_cooldown,
            wade_rows,
            wade_slowdown,
            shoulder_grains,
            buoyancy,
            swim_damp,
            stroke_impulse,
            stroke_cooldown,
            shake_reach,
            shake_shed,
            shake_seed,
            mantle_reach,
            climb_speed,
            surface_hop,
            dig_yield,
            bore_bite,
            hammer_reach,
            hammer_radius,
            hammer_force,
            hammer_cooldown,
            hammer_recoil,
            chop_reach,
            chop_radius,
            chop_cooldown,
            chop_yield,
        } = *self;
        // A count of cells held as a `u8`.
        //
        // **Zero is passed through, and that is not a rounding detail.** In
        // this struct a zero means *off* -- `shoulder_grains: 0` is the old
        // hard veto, which `a_stray_grain_at_chest_height_is_not_a_wall`
        // uses as its control precisely because it is a different rule and
        // not a smaller number. A blanket floor of 1 switched that control
        // on at every scale including 1.0, so `scaled(1.0)` stopped being
        // the identity and the guard stopped guarding. Caught by that test,
        // which is the argument for keeping a zero-valued control in one.
        //
        // Above zero the floor stands: a scale below 1 must not round a
        // reach away to nothing and silently disable a rule that was on.
        let cells = |v: u8| {
            if v == 0 {
                return 0;
            }
            ((v as f32 * k).round() as i32).clamp(1, u8::MAX as i32) as u8
        };
        Self {
            // ---- accelerations and speeds, cells per tick ----
            gravity: gravity * k,
            run_accel: run_accel * k,
            run_max: run_max * k,
            ground_decel: ground_decel * k,
            jump_impulse: jump_impulse * k,
            fall_clamp: fall_clamp * k,
            stroke_impulse: stroke_impulse * k,
            climb_speed: climb_speed * k,
            // A blow's impulse becomes the speed its fragments leave at
            // (`rigid::fracture_with_impulse`), and the recoil is a velocity
            // straight onto `Player::vx`. Both are cells per tick, so both
            // take one factor -- a `k`-times-bigger gnome must throw rock
            // `k` times as many cells to throw it the same distance.
            hammer_force: hammer_force * k,
            hammer_recoil: hammer_recoil * k,

            // ---- lengths, cells ----
            step_up: cells(step_up),
            dig_reach: cells(dig_reach),
            dig_radius: cells(dig_radius),
            wade_rows: cells(wade_rows),
            shake_reach: cells(shake_reach),
            mantle_reach: cells(mantle_reach),
            // Powder cells tolerated **in one row**, and the row is his
            // width, which scaled -- so this scales once, not twice.
            shoulder_grains: cells(shoulder_grains),
            // The belt's lengths, all plainly cells: how deep a bore stroke
            // cuts, how far each blow reaches, how wide it lands.
            bore_bite: cells(bore_bite),
            hammer_reach: cells(hammer_reach),
            hammer_radius: cells(hammer_radius),
            chop_reach: cells(chop_reach),
            chop_radius: cells(chop_radius),

            // ---- dimensionless ----
            // Each of these is stated as a fraction *of something that
            // scales*, so scaling them again would count `k` twice:
            // `air_control` multiplies `run_accel`, `buoyancy` multiplies
            // `gravity`, `surface_hop` is a fraction of a standing jump,
            // `wade_slowdown` and `swim_damp` are velocity multipliers.
            air_control,
            wade_slowdown,
            buoyancy,
            swim_damp,
            surface_hop,
            // Plain probabilities and fractions of a material yield.
            shake_shed,
            shake_seed,
            dig_yield,
            // The same kind of number as `dig_yield`: what fraction of what
            // a stroke cuts stays as timber. A fraction is a fraction at any
            // size.
            chop_yield,

            // ---- times, ticks ----
            // The clock does not change when the ruler does.
            coyote_frames,
            jump_buffer_frames,
            dig_cooldown,
            stroke_cooldown,
            hammer_cooldown,
            chop_cooldown,
        }
    }
}

impl Player {
    /// Spawn with the rectangle centred on `(x, y)`, at the authored size.
    ///
    /// **`at_scaled` is the one the app uses.** This is the size everything
    /// in this file was tuned against, so tests and harnesses that build a
    /// world by hand want it; a generated world may be finer.
    pub fn at(x: i32, y: i32) -> Self {
        Self::at_scaled(x, y, 1.0)
    }

    /// Spawn into a world built at `cell_scale` cells per unit of ground.
    ///
    /// **He has to be scaled or he is not the same character.** At half the
    /// cell size a 7x14 body is half as tall in the world, walks over steps
    /// twice as big relative to him, wades to a different depth and draws
    /// half the size on screen. The owner caught exactly this by eye on the
    /// first rescaled render: *"our gnome shouldn't have shrunk"*.
    ///
    /// So the size is carried on the instance rather than read from the
    /// constants, and every rule in this file that measures him -- his grip
    /// rows, his wade line, his shoulder, his reach -- goes through it. The
    /// constants stay as what he was *authored* at, which is what `at` and
    /// the tests below want.
    pub fn at_scaled(x: i32, y: i32, cell_scale: f32) -> Self {
        let w = ((PLAYER_WIDTH as f32 * cell_scale).round() as i32).max(1);
        let h = ((PLAYER_HEIGHT as f32 * cell_scale).round() as i32).max(1);
        Self {
            w,
            h,
            x: (x - w / 2) as f32,
            y: (y - h / 2) as f32,
            vx: 0.0,
            vy: 0.0,
            grounded: false,
            buried: false,
            coyote: 0,
            jump_buffer: 0,
            jump_was_held: false,
            swing_cooldown: 0,
            swing_span: 0,
            tool: Tool::Pick,
            dig_style: DigStyle::Bore,
            bore_dir: None,
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
        (x, y, x + self.w - 1, y + self.h - 1)
    }

    /// Centre of the occupied rectangle — where reach is measured from.
    pub fn center(&self) -> (i32, i32) {
        let (x, y) = self.rect_origin();
        (x + self.w / 2, y + self.h / 2)
    }

    /// Whether the next click will land a blow, rather than being eaten by
    /// the recovery from the last one.
    pub fn swing_ready(&self) -> bool {
        self.swing_cooldown == 0
    }

    /// How far through the recovery from the last blow, 0 (just struck) to
    /// 1 (ready). Drawn as the HUD's swing bar.
    ///
    /// **A bar and not a number**, and it is the one readout here that is
    /// about rhythm rather than state: held digging is a sequence of blows
    /// at a fixed rate, and a player who cannot see the rate reads a
    /// cooldown as the tool being unresponsive. Reports 1 before the first
    /// blow, which is true — he is ready.
    pub fn swing_progress(&self) -> f32 {
        if self.swing_span == 0 {
            return 1.0;
        }
        1.0 - self.swing_cooldown as f32 / self.swing_span as f32
    }

    /// Charge the shared recovery timer and put the swing pose up.
    ///
    /// The pose is **half the (possibly dilated) cooldown, computed rather
    /// than named** — see `SWING_FRAMES`. Held striking then alternates
    /// swing and stance rather than sticking in one or flickering between
    /// them, at every tool's rate and at every dilation.
    fn strike_landed(&mut self, cooldown: u8) {
        self.swing_cooldown = cooldown;
        self.swing_span = cooldown;
        self.action = (cooldown / 2).max(1);
    }

    /// Make him ready to strike again this instant. Tests and harnesses
    /// only: it is how a probe lands N blows without simulating the
    /// recovery between them.
    pub fn clear_swing_cooldown(&mut self) {
        self.swing_cooldown = 0;
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
    fn near(world: &World, x: i32, y: i32, (w, h): (i32, i32), margin: i32) -> Self {
        let (lo_x, lo_y) = (x - margin, y - margin);
        let (hi_x, hi_y) = (x + w + margin, y + h + margin);
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
/// stand: no hard blocker anywhere in it, and above the bottom `wade` rows
/// no row holding more than `shoulder` cells of loose powder.
///
/// That second clause is the wade. Allowing powder at the feet and not at
/// the chest is what makes him sink into a drift to about the knee and
/// stop, rather than either walking on its surface (phase 1and2) or sinking
/// through it as if it were air. It is also, deliberately, the same
/// predicate `depenetrate` uses, so sand arriving around his boots is not
/// treated as an invasion needing a shove — only sand up to his chest is.
///
/// **The chest test is per row, and it counts rather than vetoing.** The
/// veto read "any powder above the knee is a wall" — a claim about a drift
/// applied to individual cells, under which one stray `soil` grain lodged
/// in a canopy stopped the gnome dead for eleven thousand frames of
/// `scene=wood`. Step-up could not save him either: lifting slides the
/// offending cell *down* his body toward the wade rows, so a grain at
/// chest height wants a lift of `chest - dy`, one more than `step_up`
/// reaches at exactly the height that grain sat. A drift's face fills
/// whole courses across his width and still stops him at any setting of
/// the allowance; see `Tuning::shoulder_grains` for why the row is the
/// right unit and the rect is not.
fn rect_free(world: &World, bodies: &Bodies, x: i32, y: i32, (w, h): (i32, i32), wade: i32, shoulder: i32) -> bool {
    let chest = h - wade;
    for dy in 0..h {
        let mut grains = 0;
        for dx in 0..w {
            match footing(world, bodies, x + dx, y + dy) {
                Footing::Hard => return false,
                Footing::Soft if dy < chest => {
                    grains += 1;
                    if grains > shoulder {
                        return false;
                    }
                }
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
    // **The panel edits the authored feel; the physics runs the scaled
    // one.** Every length and speed in `Tuning` is in cells, so a world
    // generated at a finer `cell_scale` needs them all multiplied or the
    // gnome is the right size and moves at a fraction of the right speed
    // (`Tuning::scaled` has the arithmetic and why it is the same four
    // classes as the worldgen struct). Done here rather than at the call
    // site so the tunables panel keeps showing the numbers a human authored
    // -- 27 float multiplies once a tick, against a sweep that walks
    // thousands of cells.
    let scaled = tuning.scaled(world.cell_scale);
    let tuning = &scaled;
    let wade = tuning.wade_rows as i32;
    let shoulder = tuning.shoulder_grains as i32;
    // Body cells near him, once. The margin covers this tick's whole
    // sweep — the furthest he can travel plus the depenetration reach —
    // so the window is gathered before he moves and is still valid after.
    //
    // **Sized from the *unscaled* tuning, deliberately.** A dilated gnome
    // moves less per tick, so scaling this with him would shrink the window
    // to `DEPENETRATE_REACH + 1` = 6 at four times slower -- exactly
    // `PLATFORM_STICK`, leaving the deepest probe in this function with zero
    // slack. Nothing about a probe radius is a rate; it is the reach the
    // *rest* of the tick needs, and paying for the un-dilated worst case
    // costs a handful of cells of scan.
    let (xi, yi) = p.rect_origin();
    let reach = tuning.fall_clamp.max(tuning.run_max).ceil() as i32 + DEPENETRATE_REACH + 1;
    let bodies = Bodies::near(world, xi, yi, (p.w, p.h), reach);

    // Everything from here reads the *dilated* character (`Tuning::dilated`,
    // `clock::Clock::gnome_slowdown`). Identical to `tuning` at the default,
    // where `dilated` returns a copy untouched.
    let scale = world.clock.gnome_scale();
    let dilated = tuning.dilated(scale);
    let tuning = &dilated;

    // Free an invaded rectangle first, so this tick's movement starts
    // from a legal position: sand that fell into us, a body that settled
    // on us. Shortest clear push wins; up is tried first at each distance
    // because material arrives from above, and "on top of the pile" is
    // the right place to end up.
    depenetrate(world, &bodies, &mut p, wade, shoulder);

    if p.buried {
        // Entombed: no movement, no jump, velocities dead. Coyote and the
        // jump buffer still tick down so nothing fires the instant the
        // gnome is freed.
        p.vx = 0.0;
        p.vy = 0.0;
        p.climbing = false;
        p.coyote = p.coyote.saturating_sub(1);
        p.jump_buffer = p.jump_buffer.saturating_sub(1);
        p.swing_cooldown = p.swing_cooldown.saturating_sub(1);
        p.stroke_cooldown = p.stroke_cooldown.saturating_sub(1);
        p.jump_was_held = input.jump_held;
        world.player = Some(p);
        return;
    }
    p.swing_cooldown = p.swing_cooldown.saturating_sub(1);
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
    p.swimming = (0..p.w)
        .any(|dx| world.in_bounds(xi + dx, yi) && world.materials.kind(world.get(xi + dx, yi).material) == MaterialKind::Liquid);
    // In the water *at all*, which is a different question from `swimming`
    // and is the one the haul-out below asks.
    //
    // `swimming` reads the head row, so it goes false the instant he
    // surfaces -- and at that moment he is treading at the edge of a pool
    // with nothing under his feet, which is precisely when he wants to
    // pull himself onto the bank. Keying the haul-out on `swimming` would
    // have switched it off exactly where it is needed.
    let floating = (0..p.h).any(|dy| {
        (0..p.w).any(|dx| {
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
    let soaked = (0..p.h)
        .filter(|&dy| (0..p.w).any(|dx| footing(world, &bodies, xi + dx, yi + dy) == Footing::Soft))
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
    let grip = (0..grip_rows(p.h)).any(|dy| (0..p.w).any(|dx| footing(world, &bodies, xi + dx, yi + dy) == Footing::Climb));
    // How much foliage he is falling through, graded by how much of him is
    // in it — the same shape as the wade above, and for the reason recorded
    // there: a flag reads as a debuff, a depth reads as a canopy.
    //
    // Summed over rows rather than maxed, so clipping the top of a crown
    // barely registers and going through the middle of one arrests him.
    // Read off the material (`fall_drag`), not off `Footing::Climb`, so a
    // bare trunk catches nothing while the leaves on it do.
    let foliage: f32 = (0..p.h)
        .map(|dy| {
            (0..p.w)
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
        / p.h as f32;

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
            // A per-tick retention like `swim_damp`, so it takes the same
            // power under dilation -- and unlike `swim_damp` it is not a
            // `Tuning` field at all (it comes from `Material::fall_drag`), so
            // `Tuning::dilated` cannot reach it and it has to be done here.
            // Left raw, a crown would arrest a slowed fall N times harder in
            // the gnome's own time.
            p.vy *= (1.0 - foliage.clamp(0.0, 1.0)).powf(scale);
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
            if rect_free(world, &bodies, nxi, nyi, (p.w, p.h), wade, shoulder) {
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
                    if !rect_free(world, &bodies, nxi, nyi - lift, (p.w, p.h), wade, shoulder) {
                        continue;
                    }
                    if mantling {
                        let lands_on_something = (0..p.w)
                            .any(|dx| matches!(footing(world, &bodies, nxi + dx, nyi - lift + p.h), Footing::Hard | Footing::Soft));
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
            if rect_free(world, &bodies, nxi, nyi, (p.w, p.h), wade, shoulder) {
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
        && (0..p.w).any(|dx| matches!(footing(world, &bodies, xi + dx, yi + p.h), Footing::Hard | Footing::Soft));

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
        (0..p.w).find_map(|dx| bodies.at(xi + dx, yi + p.h))
    } else {
        None
    };
    if carrier.is_none() && !p.swimming && !p.grounded && p.vy >= 0.0 {
        for drop in 1..=PLATFORM_STICK {
            let row = yi + p.h + drop;
            let Some(i) = (0..p.w).find_map(|dx| bodies.at(xi + dx, row)) else {
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
            if catchable && rect_free(world, &bodies, xi, row - p.h, (p.w, p.h), wade, shoulder) {
                p.y = (row - p.h) as f32;
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
    // **Dilated here too, not just in `step`.** `dig` and `shake` are called
    // from `App::paint_stroke` on the render-frame path rather than from the
    // tick, with the stored tuning -- so a dilation applied only inside
    // `step` would leave `dig_cooldown` running at real-time while every
    // other rhythm slowed, and the gnome would dig N times faster relative to
    // his own motion. Identical to `tuning` at the default.
    let dilated = tuning.dilated(world.clock.gnome_scale());
    let tuning = &dilated;
    let mut p = world.player.take()?;
    let bite = if p.swing_cooldown == 0 {
        p.strike_landed(tuning.dig_cooldown);
        // **A buried gnome always digs free-hand, whatever style is
        // selected**, and this is not a fallback -- it is the only reading
        // that keeps the escape working. The bore box is anchored on his
        // rectangle and cut *outside* it, so under a pile it clears a
        // gnome-sized room next door and leaves him exactly as entombed as
        // he was. The free bite aims at his own rectangle and, more to the
        // point, `displace_disc` throws the burying material as far as
        // `BURIED_THROW` -- which is what actually frees him. See
        // `bite_point`.
        let bite = match p.dig_style {
            DigStyle::Bore if !p.buried => bore_bite(world, &mut p, aim, tuning),
            _ => free_bite(world, &mut p, aim, tuning),
        };
        Some(bite)
    } else {
        None
    };
    world.player = Some(p);
    bite
}

/// The passage a bore aimed at `aim` would open: which way it is driven,
/// and the inclusive box it will eventually clear.
///
/// **Public and shared with the renderer**, for the reason `bite_point`
/// records: a second copy of this arithmetic would let the preview and the
/// cut disagree, and a preview that lies is worse than none. The preview is
/// most of what makes the bore feel different from the free bite — you can
/// see the corridor before you commit to it.
///
/// # The size, and why it is this size
///
/// One cell of clearance on every side of the gnome: `PLAYER_WIDTH + 2` by
/// `PLAYER_HEIGHT + 2`, which at the current 7x14 is 9x16. That is the
/// literal statement of "a passage he fits down" — and it is derived from
/// his extent rather than written as a constant, because he has already
/// been grown twice on playtest notes and a hardcoded box would have
/// quietly stopped fitting him both times.
///
/// # Where it is put: against the face, not against him
///
/// Flush against his rectangle when there is something there to cut, and
/// otherwise **slid forward along the chosen direction to the first thing
/// in reach**, bounded by `dig_reach`.
///
/// Anchored on his body alone — which is how this was built first — a bore
/// swung at a wall twelve cells away carves twelve cells of air and reports
/// a hole it never made. That is `Tool::Dig`'s own standing rule broken
/// from the inside: a reach may bound *where* a verb lands and must never
/// decide *whether* it happens. It is also the free-hand bite's rule,
/// already: `face_toward` walks the ray to the first thing it can cut. So
/// both styles now answer the same question, and differ only in the shape
/// of what comes out.
///
/// The slide is bounded by reach and **not by where the cursor is**, which
/// is the whole difference from the free bite. The cursor picks a
/// direction; nothing else about it should change the cut, or the bore is
/// free-hand again with extra steps.
///
/// # The one asymmetry
///
/// Horizontally the box's **floor is level with his feet**, not centred on
/// him — so the whole two cells of vertical clearance go over his hat,
/// where headroom is, and the corridor floor runs continuously out of the
/// ground he is standing on. Centring it instead cuts two rows out from
/// under the floor, and every bite forward becomes a step down: a corridor
/// driven across flat rock would descend two cells per stroke for no reason
/// anyone could see.
pub fn bore_rect(world: &World, p: &Player, aim: (i32, i32), tuning: &Tuning) -> (Dir, (i32, i32, i32, i32)) {
    // **Sticky, so the preview and the cut agree and neither flips under a
    // walking gnome.** `bore_rect` is called every frame by the renderer
    // with `&Player` and once per stroke by `dig`; both read the same held
    // direction, and only `dig` writes it back. That ordering is what makes
    // the drawn box a promise about the next stroke rather than a guess.
    let dir = Dir::sticky(p.center(), aim, p.bore_dir);
    let flush = bore_rect_at(p, dir, 0);
    // How far forward the box may slide before the swing gives up and
    // lands where he stands.
    let reach = tuning.dig_reach as i32;
    let face_at = |d: i32, want: &dyn Fn(&World, i32, i32) -> bool| {
        let slab = bore_slab(dir, bore_rect_at(p, dir, d), 0, 1);
        (slab.1..=slab.3).any(|y| (slab.0..=slab.2).any(|x| world.in_bounds(x, y) && want(world, x, y)))
    };
    // **Rock sites the box. A tree does not, and neither does loose
    // material.** This is `face_toward`'s rule for the free bite, arrived
    // at here by the same two failures it records:
    //
    // - a trunk between the two reaches used to swallow the click
    //   entirely, so living tissue is passed over. It is still *cut* — the
    //   box is a passage and a tree in it comes down — it just does not
    //   get to decide where the passage is;
    // - and the digger's own spoil shielded the face he was cutting. A cut
    //   leaves a `dig_yield` fraction behind, so with powder siting the box
    //   the next stroke lands on the muck a cell in front of the rock, and
    //   **the bore grinds on its own spoil for ever**. Caught by
    //   `app::a_click_on_a_tree_shakes_it_rather_than_cutting_or_painting`
    //   on a single shaken-loose grain of sand, which sited a whole
    //   passage five cells short of the wall it was aimed at.
    //
    // The fallback is what keeps a dune or a lone tree diggable rather
    // than a swing at nothing: with no hard face in reach, anything at all
    // will do.
    let bodies = Bodies::none();
    let slide = (0..=reach)
        .find(|&d| face_at(d, &|w: &World, x, y| matches!(footing(w, &bodies, x, y), Footing::Hard | Footing::Scenery)))
        .or_else(|| (0..=reach).find(|&d| face_at(d, &|w: &World, x, y| !w.is_empty(x, y))));
    (dir, slide.map_or(flush, |d| bore_rect_at(p, dir, d)))
}

/// The bore box `offset` cells forward of his rectangle in `dir`.
fn bore_rect_at(p: &Player, dir: Dir, offset: i32) -> (i32, i32, i32, i32) {
    let (x0, y0, x1, y1) = p.bounds();
    // **Off `p.w`/`p.h`, not off the constants.** The gnome scales with the
    // world now (`Player::at_scaled`), so `PLAYER_WIDTH`/`PLAYER_HEIGHT` are
    // his size at `cell_scale` 1.0 and nothing else -- a box built from them
    // is the wrong size for him at every other scale. That is the failure
    // this function's own doc warns about ("he has already been grown twice
    // on playtest notes and a hardcoded box would have quietly stopped
    // fitting him both times"), and it arrived through a **clean** merge:
    // `main` made the extent per-instance while this branch was reading the
    // constants, with no textual conflict anywhere near it. The margin is a
    // cell of clearance at *his* scale for the same reason.
    let margin = (BORE_MARGIN as f32 * (p.w as f32 / PLAYER_WIDTH as f32)).round().max(1.0) as i32;
    let (w, h) = (p.w + 2 * margin, p.h + 2 * margin);
    // Centred on him for a shaft, which has no floor to line up with.
    let sx = p.center().0 - w / 2;
    let d = offset;
    match dir {
        Dir::Left => (x0 - w - d, y1 - h + 1, x0 - 1 - d, y1),
        Dir::Right => (x1 + 1 + d, y1 - h + 1, x1 + w + d, y1),
        Dir::Up => (sx, y0 - h - d, sx + w - 1, y0 - 1 - d),
        Dir::Down => (sx, y1 + 1 + d, sx + w - 1, y1 + h + d),
    }
}

/// One stroke's worth of the bore box: `depth` cells thick, starting at the
/// **working face** — the nearest slab that still has anything in it.
///
/// # Why the working face and not the box's own near edge
///
/// This is `face_toward`'s lesson in a second costume, and it was built the
/// wrong way first. Anchored on the box edge, the slice is always the same
/// three cells: the first stroke clears them and every stroke after it cuts
/// air, so **holding the button drives the bore exactly one stroke and then
/// stops**. The passage advances only if the player walks forward between
/// presses, which is a rule nothing on screen states and which reads
/// precisely as the tool having broken.
///
/// Advancing to the first slab with something in it makes a held button
/// drive the passage through the whole box, then stop when the box is
/// clear — at which point he walks in and the box re-anchors ahead of him.
/// That loop is the verb.
///
/// # What counts as the face, and why it is two questions
///
/// **Rock first, then anything at all.** The obvious rule — the nearest
/// slab that is not empty — was built and is wrong in the case the bore
/// exists for. A cut leaves a `dig_yield` fraction of itself behind as
/// spoil, and deep inside a massif that spoil has nowhere to be thrown:
/// `displace_rect`'s reach is four cells and every one of them is rock or
/// more bore. So the near slab never empties, the face never advances, and
/// **a passage driven into solid rock stalls three cells in** — measured,
/// with 80 of the box's 144 cells still standing after sixteen strokes
/// that should have cleared it twice over.
///
/// Rubble in the bore is not the working face; it is spoil, and the stroke
/// shoves whatever of it lies inside its own slice. So the face is the
/// nearest slab with something the cut can *break* in it.
///
/// The fallback catches a slab holding only what the pick declines —
/// water, or a gas pocket. It used to catch far more than that: the face
/// test asked `rigid::is_tool_target`, which refuses `Powder`, so **most
/// of this world's surface** fell through it and a bore into a dune
/// advanced by displacement alone. `rigid::is_dig_target` is the pick's
/// own predicate and takes loose ground, so soil and sand are now a
/// working face like any other and the fallback is the rare case its
/// wording implies.
pub fn bore_slice(world: &World, dir: Dir, rect: (i32, i32, i32, i32), depth: i32) -> (i32, i32, i32, i32) {
    let span = bore_span(dir, rect);
    let slab_holds = |i: i32, want: &dyn Fn(&World, i32, i32) -> bool| {
        let slab = bore_slab(dir, rect, i, 1);
        (slab.1..=slab.3).any(|y| (slab.0..=slab.2).any(|x| world.in_bounds(x, y) && want(world, x, y)))
    };
    let face = (0..span)
        .find(|&i| slab_holds(i, &|w, x, y| crate::sim::rigid::is_dig_target(w, x, y)))
        .or_else(|| (0..span).find(|&i| slab_holds(i, &|w: &World, x, y| !w.is_empty(x, y))))
        // A box with nothing in it at all: the stroke lands on the near
        // edge and reports zero, which is what a swing at nothing is.
        .unwrap_or(0);
    bore_slab(dir, rect, face, depth.max(1))
}

/// How many cells deep the box is, along the direction it is driven.
fn bore_span(dir: Dir, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> i32 {
    match dir {
        Dir::Left | Dir::Right => x1 - x0 + 1,
        Dir::Up | Dir::Down => y1 - y0 + 1,
    }
}

/// The `thick`-deep slab of the box starting `offset` cells in from the
/// near face, clamped to the box.
fn bore_slab(dir: Dir, (x0, y0, x1, y1): (i32, i32, i32, i32), offset: i32, thick: i32) -> (i32, i32, i32, i32) {
    let span = bore_span(dir, (x0, y0, x1, y1));
    let near = offset.clamp(0, (span - 1).max(0));
    let far = (near + thick - 1).min(span - 1);
    match dir {
        Dir::Left => (x1 - far, y0, x1 - near, y1),
        Dir::Right => (x0 + near, y0, x0 + far, y1),
        Dir::Up => (x0, y1 - far, x1, y1 - near),
        Dir::Down => (x0, y0 + near, x1, y0 + far),
    }
}

/// How far the cursor must move before the bore will change direction. See
/// `Dir::sticky`.
///
/// A constant rather than a tunable: this is not a feel knob to sweep, it
/// is the difference between a control that obeys the hand and one that
/// obeys the legs. 12 cells is just under the gnome's own height, so a
/// re-point anywhere off his body counts and hand jitter never does.
const REAIM_DEADZONE: i32 = 12;

/// One cell of clearance on each side. A constant rather than a tunable
/// because it is not a feel knob: below 1 the passage is exactly his own
/// silhouette and every piece of spoil in it is a wall, and above 2 it
/// stops reading as a passage cut *for him*.
const BORE_MARGIN: i32 = 1;

/// One stroke of the bore: cut the near slice of the box, then shove
/// whatever loose material the cut leaves out of it.
fn bore_bite(world: &mut World, p: &mut Player, aim: (i32, i32), tuning: &Tuning) -> Bite {
    let (dir, boxr) = bore_rect(world, p, aim, tuning);
    // Latched on the stroke, not on the hover: the direction a player is
    // "digging" is the one they last committed to with a click.
    p.bore_dir = Some((dir, aim));
    let slice = bore_slice(world, dir, boxr, tuning.bore_bite as i32);
    let at = ((slice.0 + slice.2) / 2, (slice.1 + slice.3) / 2);
    // **No sweep join here, and none is needed.** The capsule needs one
    // because consecutive discs pinch between their centre lines
    // (`rigid::mine_swept`); a slice is the full cross-section of the
    // passage by construction, so two consecutive strokes abut with no
    // scallop at all whatever the digger did in between. `last_bite` is
    // still updated so switching to `Free` mid-tunnel joins onto the face
    // rather than starting a fresh bore behind it.
    p.last_bite = Some(at);
    let dusted = crate::sim::rigid::mine_rect(world, (slice.0, slice.1), (slice.2, slice.3), tuning.dig_yield);
    // **Displacement is still not optional, and the reason has changed.**
    // It used to be the only thing that worked at all in loose ground:
    // `mine_rect` broke `rigid::is_tool_target` cells — solid and plant —
    // so a slice cut into sand or soil broke nothing, and driving a
    // passage through the ground the world is mostly made of did literally
    // nothing. `rigid::is_dig_target` closed that; the cut now removes
    // loose ground itself.
    //
    // What is left for the shove is the half a cut cannot do: **spoil the
    // stroke leaves behind, and material that flows back in.** Loose
    // ground slumps into the hole under the ordinary sweep — which is what
    // makes digging soil feel like digging rather than erasing — and
    // without somewhere for the near slab's grains to go, the face fills
    // as fast as it is cut.
    let displaced = displace_rect(world, p, slice, SPOIL_THROW);
    Bite { at, displaced, dusted }
}

/// The free-hand bite: a round cut at the near face along the aim ray,
/// swept from the last one. `DigStyle::Free`, and the only shape a buried
/// gnome ever cuts.
fn free_bite(world: &mut World, p: &mut Player, aim: (i32, i32), tuning: &Tuning) -> Bite {
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
    let at = bite_point(world, p, aim, tuning);
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
    let displaced = displace_disc(world, p, at.0, at.1, radius, search);
    Bite { at, displaced, dusted }
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
    // The cursor, plus a cell or two of forgiveness — a twig is one pixel
    // at zoom 1 and a trunk is a few, and neither should need pixel-perfect
    // pointing. Nearest first, so a cursor between a branch and the ground
    // takes the branch it is closest to rather than whichever the scan
    // reached first. `snap_to` is that walk, shared with the axe.
    snap_to(world, p.center(), aim, tuning.shake_reach as i32, |w, x, y| {
        let cell = w.get(x, y);
        cell.organism_id() != 0 && w.materials.get(cell.material).climbable
    })
}

/// How far from the cursor a shake will look for something to take hold
/// of. Small: this is forgiveness for a one-pixel twig, not aim assist.
const SHAKE_SNAP: i32 = 2;

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
    // See `dig` for why this dilates rather than taking the stored tuning.
    let dilated = tuning.dilated(world.clock.gnome_scale());
    let tuning = &dilated;
    let mut p = world.player.take()?;
    if p.swing_cooldown != 0 {
        world.player = Some(p);
        return None;
    }
    p.strike_landed(tuning.dig_cooldown);
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
            // Living tissue is **invisible to the pick's aim**, and that
            // is a reversal worth reading before changing it back.
            //
            // This returned the cell, on the argument that a tree stops the
            // aim even though it no longer stops the body — so you could
            // point at a trunk standing in front of a cliff. The split was
            // right and the place was wrong: movement collision, aim
            // collision and *cut* collision are three questions, and that
            // merged the last two. `mine_swept` skips organism cells, so a
            // tree the ray stopped at was a bite that did nothing — a trunk
            // between `shake_reach` and `dig_reach` was a dead click, which
            // `Tool::Dig`'s own doc forbids.
            //
            // The shake asks its own question now (`shake_target`, off the
            // cursor rather than off a ray), so this function only has to
            // answer the pick's: where is the nearest thing I can cut.
            Footing::Climb => {}
            // A formation **does** stop the aim, and the contrast with the
            // arm above is the rule rather than an exception to it: this ray
            // stops at whatever the pick can cut. A stalagmite is scenery he
            // walks through, but `mine_swept` cuts it like any other rock, so
            // a ray that flew through one would leave it visible and
            // impossible to remove. Living tissue is the opposite case —
            // passable *and* uncuttable — so the ray passes through it.
            //
            // Walk-through is therefore not what either arm keys on. If a
            // third passable material ever arrives, ask the only question
            // that decides this: can the pick cut it?
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

/// `displace_disc` for a rectangle: shove every loose cell inside `rect`
/// to the nearest resting place outside it, and report how many moved.
///
/// **The landing rule is the disc's, and must stay so** — nearest empty
/// cell that has something under it. Nearest-empty alone laid bars of sand
/// along a ring in open sky, which lived exactly one frame and read as fake
/// (the full account is on `displace_disc`); a spoil rule that differed
/// between the two dig styles would put that back on one of them.
///
/// **The search is around each cell rather than around the region's
/// centre**, which is where this genuinely differs from the disc. A bore
/// slice is 16 cells long, so a ring walk from its middle would throw spoil
/// from one end of the passage to the other before trying the wall it is
/// lying against.
fn displace_rect(world: &mut World, p: &Player, (x0, y0, x1, y1): (i32, i32, i32, i32), search: i32) -> usize {
    let mut moved = 0;
    // His own cells first, then everything else -- see `displace_disc` for
    // why the order is load-bearing rather than cosmetic. A bore is cut
    // outside his rectangle so the first pass is normally empty; it costs
    // one comparison per cell and keeps the two paths honest if a future
    // bore ever overlaps him.
    for pass in [true, false] {
        for y in y0..=y1 {
            for x in x0..=x1 {
                if inside_player(p, x, y) != pass || !world.in_bounds(x, y) {
                    continue;
                }
                let kind = world.materials.kind(world.get(x, y).material);
                if !matches!(kind, MaterialKind::Powder | MaterialKind::Liquid) {
                    continue;
                }
                'rings: for ring in 1..=search {
                    for (rx, ry) in ring_offsets(ring) {
                        let (nx, ny) = (x + rx, y + ry);
                        let in_rect = (x0..=x1).contains(&nx) && (y0..=y1).contains(&ny);
                        if in_rect || inside_player(p, nx, ny) {
                            continue; // not back into the cut, nor into the gnome
                        }
                        if !world.in_bounds(nx, ny) || !world.is_empty(nx, ny) {
                            continue;
                        }
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

/// Where a hammer blow aimed at `aim` would land.
///
/// Public and shared with the renderer for the reason `bite_point` records:
/// the marker and the blow must not be able to disagree.
pub fn hammer_point(world: &World, p: &Player, aim: (i32, i32), tuning: &Tuning) -> (i32, i32) {
    face_toward(world, p.center(), aim, tuning.hammer_reach as i32)
}

/// Where an axe stroke aimed at `aim` would land — living tissue first,
/// then a creature, then whatever the ray reaches. See `chop` for why the
/// order is that, and for why the third case exists at all.
pub fn chop_point(world: &World, p: &Player, aim: (i32, i32), tuning: &Tuning) -> (i32, i32) {
    let from = p.center();
    let reach = tuning.chop_reach as i32;
    snap_to(world, from, aim, reach, is_living_tissue)
        .or_else(|| snap_to(world, from, aim, reach, is_creature))
        .unwrap_or_else(|| face_toward(world, from, aim, reach))
}

fn is_living_tissue(w: &World, x: i32, y: i32) -> bool {
    let cell = w.get(x, y);
    cell.organism_id() != 0 && matches!(w.materials.kind(cell.material), MaterialKind::Plant)
}

fn is_creature(w: &World, x: i32, y: i32) -> bool {
    matches!(w.materials.kind(w.get(x, y).material), MaterialKind::Creature)
}

/// What one hammer blow did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Smash {
    pub at: (i32, i32),
    /// Cells the blow acted on — pulverized plus loosened, straight out of
    /// `rigid::strike`. **Zero is the interesting value**: a swing at open
    /// air, a swing that lands on bedrock and a swing that calves a slab
    /// are three different events and the picture cannot tell them apart.
    pub broken: usize,
}

/// One hammer blow toward `aim`. `None` without a player or while the
/// recovery is running.
///
/// # What a blow is, against what a bite is
///
/// The pick removes rock. The hammer *damages* it: `rigid::strike`
/// pulverizes a small core, chips a shell off it, and scores cracks out to
/// `radius * rigid::CRACK_REACH` — damage that shows, accumulates, and
/// licences structural failure nearby (`World::record_disturbance`). So the
/// hammer is how you bring a ceiling down, undercut a slab or start a
/// collapse, and it is a poor way to dig a hole. That is the split the belt
/// exists to make choosable.
///
/// # The recoil
///
/// A blow that moves the world and not the arm swinging it reads as a
/// cursor effect. `hammer_recoil` shoves him back along the line of the
/// swing, and **only on a blow that broke something** — swinging at air
/// costs the cooldown and nothing else, which is the honest outcome and
/// also stops the recoil from being usable as a jetpack over open ground.
pub fn smash(world: &mut World, aim: (i32, i32), tuning: &Tuning) -> Option<Smash> {
    // Dilated for the reason `dig` records: this is called off the render
    // frame, not the tick.
    let dilated = tuning.dilated(world.clock.gnome_scale());
    let tuning = &dilated;
    let mut p = world.player.take()?;
    let smash = if p.swing_cooldown == 0 {
        p.strike_landed(tuning.hammer_cooldown);
        let (cx, cy) = p.center();
        // The same near-face ray the pick aims down, at the hammer's own
        // shorter reach. Shared rather than reimplemented so "what am I
        // pointing at" has one answer per tool and not two.
        let at = face_toward(world, (cx, cy), aim, tuning.hammer_reach as i32);
        let broken = crate::sim::rigid::strike(world, at.0, at.1, tuning.hammer_radius as i32, tuning.hammer_force);
        if broken > 0 {
            let (dx, dy) = ((at.0 - cx) as f32, (at.1 - cy) as f32);
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 {
                p.vx -= dx / len * tuning.hammer_recoil;
                // Half weight on the vertical. A full share of the recoil
                // on a downward blow launches him off his own floor, which
                // reads as a bug rather than as force -- and the ground he
                // is standing on is the thing a hammer is most often swung
                // at.
                p.vy -= dy / len * tuning.hammer_recoil * 0.5;
            }
        }
        Some(Smash { at, broken })
    } else {
        None
    };
    world.player = Some(p);
    smash
}

/// What one chop did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chop {
    pub at: (i32, i32),
    /// Cells that left the world as chips. The rest of the notch is lying
    /// there as timber.
    pub chips: usize,
    /// Whether the blow landed on something alive. A chop into rock is a
    /// legal, poor swing (see `chop`), and only this separates the two.
    pub living: bool,
    /// Creature cells killed outright.
    pub slain: usize,
}

/// One axe stroke toward `aim`. `None` without a player or while the
/// recovery is running.
///
/// # Aiming, and why there is no dead click
///
/// Three questions in order, and the first that answers wins:
///
/// 1. **living tissue near the cursor**, within `chop_reach` — the same
///    cursor-snap `shake_target` uses, and for the same reason its own doc
///    gives: a twig is one pixel at zoom 1 and should not need pixel-perfect
///    pointing;
/// 2. **a creature near the cursor** — the sword half of the tool. A blow
///    that lands on an animal kills it through `creature::slay`, which is
///    the engine's ordinary death (a corpse, the energy ledger closed, the
///    slot freed) rather than an erase;
/// 3. **whatever the ray reaches**, so an axe swung at a rock face still
///    chips it. Badly — `chop_radius` is a third of `dig_radius` and
///    `chop_yield` keeps half the chips — which is the right answer for
///    using the wrong tool, and a much better one than nothing happening.
///
/// # Felling comes for free, and that is the point
///
/// Nothing here knows what a tree is. The cut goes through
/// `rigid::mine_swept`, whose `shatter_to_rubble` unregisters each cell
/// from its organism as it lands, and `plant::anchor_support` re-walks the
/// plant from its anchors on its next tick and finds whatever the cut
/// severed unreached. Chop a bole through and the crown comes down as
/// pieces, by the machinery README's *Felling status* describes. What was
/// missing was never the physics — it was a verb aimed at the trunk.
pub fn chop(world: &mut World, aim: (i32, i32), tuning: &Tuning) -> Option<Chop> {
    let dilated = tuning.dilated(world.clock.gnome_scale());
    let tuning = &dilated;
    let mut p = world.player.take()?;
    let chop = if p.swing_cooldown == 0 {
        p.strike_landed(tuning.chop_cooldown);
        let (cx, cy) = p.center();
        let reach = tuning.chop_reach as i32;
        let living_at = snap_to(world, (cx, cy), aim, reach, is_living_tissue);
        let creature_at = match living_at {
            Some(_) => None,
            None => snap_to(world, (cx, cy), aim, reach, is_creature),
        };
        let at = living_at.or(creature_at).unwrap_or_else(|| face_toward(world, (cx, cy), aim, reach));
        let slain = match creature_at {
            Some((x, y)) => usize::from(crate::sim::creature::slay(world, x, y)),
            None => 0,
        };
        let radius = (tuning.chop_radius as i32).max(1);
        let chips = crate::sim::rigid::mine_swept(world, at, at, radius, tuning.chop_yield);
        Some(Chop { at, chips, living: living_at.is_some(), slain })
    } else {
        None
    };
    world.player = Some(p);
    chop
}

/// The cell at or nearest to `aim` that satisfies `want`, within `reach` of
/// `from` and within `SHAKE_SNAP` of the cursor. `None` if there is none.
///
/// Factored out of `shake_target` rather than written beside it: the shake
/// and the chop ask the same *question* of the cursor and differ only in
/// what they are looking for, and two copies of a snap radius is how a
/// verb starts aiming a cell away from its own marker.
fn snap_to(
    world: &World,
    from: (i32, i32),
    aim: (i32, i32),
    reach: i32,
    want: impl Fn(&World, i32, i32) -> bool,
) -> Option<(i32, i32)> {
    let (dx, dy) = (aim.0 - from.0, aim.1 - from.1);
    if dx * dx + dy * dy > reach * reach {
        return None; // out of arm's reach: whatever it is, this is not it
    }
    (0..=SHAKE_SNAP)
        .flat_map(|r| ring_offsets(r).map(move |(ox, oy)| (aim.0 + ox, aim.1 + oy)))
        .find(|&(x, y)| world.in_bounds(x, y) && want(world, x, y))
}

/// What one left-click did, whichever tool was in his hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blow {
    Bite(Bite),
    Smash(Smash),
    Chop(Chop),
    Shake(Shake),
}

/// **The one entry point for the left button**, dispatching on the belt.
///
/// Callers must not reach past this to `dig`/`smash`/`chop` — that is the
/// same "one gate on the operation, not one per call site" rule `App::
/// paint_stroke` already records for the tool gate itself, and the reason
/// is the same: a second call site added later would silently ignore the
/// belt.
///
/// The pick keeps its two verbs. Pointing at a plant with a pick in hand
/// still *shakes* it rather than cutting it, which stops being a
/// compromise now that the axe exists and becomes the distinction: shaking
/// is what you do to a tree you want to keep.
pub fn swing(world: &mut World, aim: (i32, i32), tuning: &Tuning) -> Option<Blow> {
    let tool = world.player.as_ref()?.tool;
    match tool {
        Tool::Hammer => smash(world, aim, tuning).map(Blow::Smash),
        Tool::Axe => chop(world, aim, tuning).map(Blow::Chop),
        Tool::Pick => {
            let shake_at = world
                .player
                .as_ref()
                .and_then(|p| shake_target(world, p, aim, tuning));
            match shake_at {
                Some(at) => shake(world, at, tuning).map(Blow::Shake),
                None => dig(world, aim, tuning).map(Blow::Bite),
            }
        }
    }
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
fn depenetrate(world: &World, bodies: &Bodies, p: &mut Player, wade: i32, shoulder: i32) {
    let (xi, yi) = p.rect_origin();
    if rect_free(world, bodies, xi, yi, (p.w, p.h), wade, shoulder) {
        p.buried = false;
        return;
    }
    for d in 1..=DEPENETRATE_REACH {
        for (dx, dy) in [(0, -d), (-d, 0), (d, 0), (0, d)] {
            if rect_free(world, bodies, xi + dx, yi + dy, (p.w, p.h), wade, shoulder) {
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

    /// **The same physical scene at twice the cell resolution plays the
    /// same.** He is twice as many cells, moves twice as many cells a tick,
    /// and ends up in the same *place*.
    ///
    /// **This replaced a guard that was blind, and the way it was blind is
    /// the point.** The first version dropped him onto flat stone and
    /// checked where he landed. It passed — and it went on passing with
    /// `rect_free`'s wade line, the grip rows and `Bodies::near`'s window
    /// each put back to the unscaled constant in turn, because a plain fall
    /// onto hard rock never asks about any of them. `CLAUDE.md`: a guard
    /// that does not go red for the fault it is named for is not weak, it is
    /// blind, and the remedy is to replace it rather than widen it.
    ///
    /// So this one makes him *travel*: run him across a floor, over a ledge
    /// he has to step up, and through a drift of powder he has to wade and
    /// shoulder. That reaches his collision rectangle, his step-up, his wade
    /// line, his shoulder allowance, the body window, and every speed and
    /// acceleration in `Tuning` — in one trajectory.
    ///
    /// Compared in **physical** units (cells divided by `cell_scale`),
    /// because that is the quantity that must not change; the cell counts
    /// are expected to double.
    ///
    /// **What it catches, measured rather than asserted.** Twelve faults
    /// were injected one at a time — each a length or speed put back to the
    /// unscaled constant — and this went red for nine:
    ///
    /// | caught | `Player`'s width and height, `rect_free`'s box width and its wade line, `shoulder_grains`, `gravity`, `fall_clamp`, `run_max`, `step_up` |
    /// |---|---|
    /// | **blind** | `grip_rows`, `Bodies::near`'s window, `wade_rows` |
    ///
    /// **The three gaps are missing scene, not weak assertions**, and are
    /// written down so the next person adding coverage knows exactly what to
    /// add rather than rediscovering it:
    ///
    /// - **`grip_rows`** needs a climbable in the scene. Nothing here is
    ///   `Footing::Climb`, so the grip test never runs at all.
    /// - **`Bodies::near`** needs a chunk body. There are none, so the
    ///   window it gathers is always empty and its size cannot matter.
    /// - **`wade_rows`** saturates: he is past the wade line in both drifts
    ///   at both scales, so the exact row count stops changing the outcome.
    ///   A drift tuned to sit right at the line would catch it; two depths
    ///   were tried and neither does.
    #[test]
    fn a_finer_world_plays_the_same_as_the_authored_one() {
        /// The same scene at `k` cells per unit of ground: floor at 88,
        /// a 3-high ledge at 80, and a drift of sand in front of it.
        fn scene(k: i32) -> World {
            let (w, h) = (256 * k, 96 * k);
            let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
            for y in (88 * k)..h {
                for x in 0..w {
                    world.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for y in (85 * k)..(88 * k) {
                for x in (80 * k)..w {
                    world.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            // Deep enough that he is buried well above the wade line, so
            // `rect_free`'s chest row and the shoulder allowance are both
            // under test rather than merely present.
            for y in (75 * k)..(85 * k) {
                for x in (100 * k)..(120 * k) {
                    world.set(x, y, Cell::new(material::SAND, 0));
                }
            }
            // **And a shallow one, which is not redundant.** Deep burial
            // saturates the wade line -- he is over it either way, so the
            // exact row count stops changing the outcome and a `wade_rows`
            // that failed to scale goes unnoticed. Measured: with only the
            // deep drift this guard was blind to exactly that fault, and
            // catches it again with a drift he wades rather than swims.
            for y in (82 * k)..(85 * k) {
                for x in (140 * k)..(160 * k) {
                    world.set(x, y, Cell::new(material::SAND, 0));
                }
            }
            world.cell_scale = k as f32;
            world.end_step();
            world
        }

        let run = |k: i32| -> Vec<(f32, f32)> {
            let mut world = scene(k);
            world.player = Some(Player::at_scaled(20 * k, 40 * k, k as f32));
            let mut path = Vec::new();
            for _ in 0..400 {
                tick(&mut world, PlayerInput { right: true, ..Default::default() });
                let p = world.player.as_ref().unwrap();
                path.push((p.x / k as f32, p.y / k as f32));
            }
            path
        };

        let (one, two, four) = (run(1), run(2), run(4));
        let p1 = { let mut w = scene(1); w.player = Some(Player::at_scaled(20, 40, 1.0)); w.player.unwrap() };
        let p2 = { let mut w = scene(2); w.player = Some(Player::at_scaled(40, 80, 2.0)); w.player.unwrap() };
        assert_eq!((p1.w, p1.h), (PLAYER_WIDTH, PLAYER_HEIGHT));
        assert_eq!((p2.w, p2.h), (PLAYER_WIDTH * 2, PLAYER_HEIGHT * 2), "he should be twice the cells at 2x");

        // He has to actually go somewhere, or the comparison is vacuous --
        // two gnomes stuck against the same wall agree perfectly.
        let travelled = one.last().unwrap().0 - one[0].0;
        assert!(travelled > 40.0, "the 1x run only covered {travelled:.1} units; it is not exercising anything");

        let diverge = |a: &Vec<(f32, f32)>, b: &Vec<(f32, f32)>| {
            a.iter().zip(b).map(|(p, q)| (p.0 - q.0).abs().max((p.1 - q.1).abs())).fold(0.0f32, f32::max)
        };
        let (one_two, two_four) = (diverge(&one, &two), diverge(&two, &four));

        // **The assertion is convergence, not an absolute distance, and that
        // is the whole design of this test.**
        //
        // The runs do not agree exactly and should not be expected to: he
        // moves on floats but collides on whole cells, so a finer world
        // snaps him at finer intervals and the two paths quantise
        // differently. Measured here, 1x against 2x is **2.80** cells of
        // ground over a 232-unit run -- which read as a scaling bug until it
        // was checked against a third resolution.
        //
        // It is not one. 2x against 4x is **0.25**, eleven times smaller,
        // and 1x against 4x is 2.68 -- so the coarse run is the outlier and
        // the fine ones agree with each other. That is convergence to a
        // continuum limit, and it is the signature that distinguishes
        // "correctly scaled, coarsely sampled" from "a length that is not
        // scaling at all". A missed length does not converge: it stays
        // proportionally wrong at every resolution, so refining the grid
        // buys nothing and `two_four` comes out no better than `one_two`.
        //
        // Bars set from those measurements with headroom, per `CLAUDE.md`:
        // 1.0 against a measured 0.25, and a ratio of 0.5 against a measured
        // 0.089.
        // **The coarse-against-fine bound, which catches what convergence
        // alone cannot.** A quantity that does not scale at all often makes
        // 2x and 4x agree *with each other* while both differ from 1x --
        // measured: with `step_up` left unscaled, the ledge is 6 cells at 2x
        // and 12 at 4x against an unchanged 4-cell step, so he is stuck at
        // both and the convergence check below sails through. Only the 1x
        // arm, where he still climbs, shows it.
        //
        // 6.0 against a measured 2.80, which is deterministic here (no RNG
        // in this scene), so the headroom is for legitimate retunes rather
        // than for noise.
        assert!(
            one_two < 6.0,
            "the authored world and the doubled one part company by {one_two:.2} cells of ground \
             over a {travelled:.1}-unit run -- a length or a speed is not scaling with the world"
        );
        assert!(
            two_four < 1.0,
            "2x and 4x disagree by {two_four:.2} cells of ground -- at that refinement they should \
             nearly coincide, so a length or a speed is not scaling with the world"
        );
        assert!(
            two_four < one_two * 0.5,
            "refining the grid did not make the runs agree: 1v2 {one_two:.2}, 2v4 {two_four:.2}. \
             Quantisation shrinks when the grid refines; a quantity that is not scaling does not"
        );
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
        let t = Tuning::default();
        assert!(
            rect_free(&world, &Bodies::none(), nx, ny, (PLAYER_WIDTH, PLAYER_HEIGHT), t.wade_rows as i32, t.shoulder_grains as i32),
            "the rect should be clear after depenetration"
        );

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
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
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
    fn at_clean_a_bite_leaves_no_spoil_in_its_bore() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
        // **Pinned to CLEAN by name, not to the default.** This was
        // `at_the_default_yield_...` and asserted `dig_yield == 0.0`, which
        // was true for one playtest and stopped being true at the next:
        // the default is `TRACE` (0.10) now. What the case is *for* is the
        // zero end of the range, so it says so, and it keeps testing that
        // end wherever the default sits. `CLAUDE.md`: assert the property,
        // not an instant that happened to coincide with it.
        let clean = SPOIL_MODES.iter().find(|m| m.name == "CLEAN").expect("a CLEAN mode must exist");
        let tuning = Tuning { dig_yield: clean.dig_yield, ..Tuning::default() };
        assert_eq!(tuning.dig_yield, 0.0, "CLEAN is the zero end of the range");
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
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
        let tuning = Tuning { dig_yield: 1.0, ..Tuning::default() };
        let before = occupied_cells(&world);
        dig(&mut world, (76, 78), &tuning).expect("digs");
        assert_eq!(before, occupied_cells(&world), "at yield 1.0 a dig may move material but never delete it");
    }

    #[test]
    fn a_bite_stops_at_the_first_face_rather_than_carving_a_sealed_pocket() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
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
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
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
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
        let bite = dig(&mut world, (127, 84), &Tuning::default()).expect("a far click still digs");
        assert_eq!(bite.at.0, 70, "a click across the map digs the wall in front of him");
    }

    #[test]
    fn aimed_at_open_sky_the_bite_stops_at_reach() {
        let mut world = world_with_floor();
        world.player = Some(Player::at(64, 84));
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
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
        //
        // **At full yield, because the conservation rider below is only a
        // real claim at full yield.** It used to run at the default and
        // pass for an accidental reason: `rigid::is_tool_target` took
        // `Solid | Plant`, so the cut could not touch sand at all and a
        // buried dig was pure displacement whatever the yield said. The
        // pick reaches loose ground now (`rigid::is_dig_target`), so at the
        // default trace yield digging out of a pile removes most of what
        // it cuts -- measured, 127 of 1,404 cells -- which is the setting
        // working, not an eraser. The invariant worth guarding is the one
        // `at_full_yield_nothing_leaves_the_world` states: **a dig may move
        // material, never delete it, when nothing is thinned.** Set
        // `dig_yield` back to the default and this goes red on the
        // conservation line while the escape still succeeds, which is the
        // shape of the change.
        let tuning = Tuning { dig_yield: 1.0, ..Tuning::default() };
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
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
        let tuning = Tuning::default();
        let wade = tuning.wade_rows as i32;
        let shoulder = tuning.shoulder_grains as i32;
        for i in 0..40 {
            // Legal *before* the bite, so the assertion after it is about
            // the bite and not about where he had already walked.
            let (bx, by) = world.player.as_ref().unwrap().rect_origin();
            if !rect_free(&world, &Bodies::none(), bx, by, (PLAYER_WIDTH, PLAYER_HEIGHT), wade, shoulder) {
                continue;
            }
            dig(&mut world, (74, 84), &tuning);
            let (ax, ay) = world.player.as_ref().unwrap().rect_origin();
            assert!(
                rect_free(&world, &Bodies::none(), ax, ay, (PLAYER_WIDTH, PLAYER_HEIGHT), wade, shoulder),
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
        let organism = world.push_organism(species).expect("an organism slot is free");
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
        let organism = world.push_organism(species).expect("an organism slot is free");
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
        let organism = world.push_organism(species).expect("an organism slot is free");
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
                let organism = world.push_organism(species).expect("an organism slot is free");
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
        let organism = world.push_organism(species).expect("an organism slot is free");
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
        let target = shake_target(&world, world.player.as_ref().unwrap(), (71, 80), &Tuning::default()).expect("aimed at the tree");
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
            let Some(target) = shake_target(&world, world.player.as_ref().unwrap(), (71, 80), &tuning) else {
                break;
            };
            shake(&mut world, target, &tuning);
            world.player.as_mut().unwrap().clear_swing_cooldown();
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
        let target = shake_target(&world, world.player.as_ref().unwrap(), (71, 80), &tuning).expect("aimed at the tree");
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
            let target = shake_target(&world, world.player.as_ref().unwrap(), (71, 80), &tuning).expect("aimed at the tree");
            shake(&mut world, target, &tuning).expect("a shake should fire").shed
        }
        let dark = shed_with(0.0);
        let lit = shed_with(100.0);
        assert!(dark > lit * 4, "a shaded crown should shed far more than a lit one: dark {dark}, lit {lit}");
    }

    #[test]
    fn the_shake_takes_what_you_point_at_and_nothing_else() {
        // One button, two verbs, and no dead click: `Tool::Dig`'s recorded
        // lesson is that a reach may bound where a verb lands and must
        // never decide whether it happens.
        //
        // **Pointing, not a ray.** This used to walk out from the gnome and
        // take the first living thing on the line, so a cursor anywhere at
        // all shook whatever tree happened to be in the way -- and standing
        // inside one, that was every direction.
        let world = world_with_leafy_tree();
        let p = world.player.as_ref().unwrap();
        let tuning = Tuning::default();
        assert!(shake_target(&world, p, (71, 80), &tuning).is_some(), "pointing at the trunk should shake");
        assert!(shake_target(&world, p, (64, 92), &tuning).is_none(), "pointing at the floor should dig");
        // Past the trunk at its own height: the tree is on the line but not
        // under the cursor, so the pick gets the click.
        assert!(
            shake_target(&world, p, (127, 80), &tuning).is_none(),
            "a tree merely in the way must not steal the click"
        );
        // A couple of cells off the trunk still counts -- a twig is one
        // pixel and should not need pixel-perfect pointing.
        assert!(
            shake_target(&world, p, (68, 80), &tuning).is_some(),
            "just off the trunk should still take hold"
        );
    }

    #[test]
    fn an_ant_does_not_stop_him() {
        // A single creature cell used to be a wall, which was defensible
        // only while a whole tree was one as well.
        let mut world = world_with_floor();
        let ant = world.materials.id_of("ant").expect("ant is compiled in");
        let species = world.species.id_of("ant").expect("ant is compiled in");
        let organism = world.push_organism(species).expect("an organism slot is free");
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
    fn the_pick_aims_through_a_tree_at_the_rock_behind_it() {
        // **This replaces a test that asserted the opposite**, and the flip
        // is the point. `face_toward` was taught to stop at living tissue
        // so the shake could aim at a tree standing in front of a cliff --
        // right split, wrong place. Movement collision, aim collision and
        // *cut* collision are three questions, and that change merged the
        // last two: the pick cannot cut an organism cell (`mine_swept`
        // skips them), so a tree it stopped at was a bite that did nothing.
        // A trunk between `shake_reach` and `dig_reach` was therefore a
        // dead click, which `Tool::Dig`'s own doc forbids. The shake asks
        // its own question now (`shake_target`), so this ray only has to
        // answer the pick's.
        let mut world = world_with_tree(85, 3, 60);
        let stone = material::STONE;
        for y in 60..88 {
            for x in 88..96 {
                world.set(x, y, Cell::new(stone, 0));
            }
        }
        world.player = Some(Player::at(60, 80));
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
        let p = world.player.take().expect("just placed");
        let at = bite_point(&world, &p, (127, 87), &Tuning::default());
        world.player = Some(p);
        assert_eq!(
            world.get(at.0, at.1).material,
            stone,
            "the pick should reach past the trunk to the rock, landed at {at:?}"
        );
    }

    #[test]
    fn standing_inside_a_tree_pointing_at_rock_is_a_dig() {
        // Reported as a green marker that would not go away; the marker was
        // the symptom. The shake aimed with a ray *from the gnome*, so
        // standing inside a trunk the nearest living tissue was distance 1
        // in every direction -- the shake won the routing whatever you
        // pointed at, and you could not dig at all in a wood.
        let mut world = world_with_tree(60, 12, 40);
        world.player = Some(Player::at(64, 80));
        // **Pinned to the free-hand style, which is what this test is
        // about.** `DigStyle::Bore` is the default now and cuts a
        // rectangle, so without this line the test would silently start
        // measuring a mechanism it was not written for -- and pass, which
        // is worse.
        world.player.as_mut().unwrap().dig_style = DigStyle::Free;
        let p = world.player.as_ref().expect("just placed");
        assert!(
            shake_target(&world, p, (78, 92), &Tuning::default()).is_none(),
            "pointing at the floor from inside a tree must still be a dig"
        );
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

    /// Walk him right for `ticks` and report how far he got, against a
    /// world the caller has already furnished.
    fn distance_walked(world: &mut World, tuning: &Tuning, ticks: usize) -> f32 {
        let start = world.player.as_ref().expect("a gnome").x;
        for _ in 0..ticks {
            step(world, PlayerInput { right: true, ..Default::default() }, tuning);
        }
        world.player.as_ref().expect("a gnome").x - start
    }

    /// One grain of soil at chest height, in his path, on open floor.
    fn world_with_a_stray_grain(grains: i32) -> World {
        let mut world = world_with_floor();
        let soil = world.materials.id_of("soil").expect("soil is compiled in");
        // Standing on the floor at y=88 puts his rect at rows 74..=87, so
        // rows 74..=83 are the chest and 84..=87 are the wade.
        //
        // **y=79 is not an arbitrary chest row, it is the one step-up
        // cannot reach.** Lifting slides an offender *down* the body, so a
        // grain at `dy` needs a lift of `chest - dy` to fall into the wade
        // rows: `dy` 5 wants 5, and `step_up` is 4. A grain one row lower
        // is cleared by the existing step-up and reproduces nothing --
        // which is exactly what the first draft of this test placed, and
        // its control caught it.
        for i in 0..grains {
            world.set(40 + i, 79, Cell::new(soil, 0));
        }
        world.player = Some(Player::at(10, 81));
        world
    }

    /// `Tuning::scaled(1.0)` is the identity, so every world that is not
    /// rescaled plays exactly as it did.
    ///
    /// **Written after this was false.** `cells()` floored counts at 1,
    /// which turned `shoulder_grains: 0` -- an *off* switch, not a small
    /// number -- into 1 at every scale, `1.0` included. The neighbouring
    /// `a_stray_grain_at_chest_height_is_not_a_wall` caught it through its
    /// zero-valued control; nothing else in the suite would have, because
    /// nothing else sets a knob here to zero. This asserts it directly
    /// rather than relying on that coincidence, and it covers the whole
    /// struct rather than the one field that happened to be zero.
    #[test]
    fn scaling_the_tuning_by_one_changes_nothing() {
        let base = Tuning::default();
        assert_eq!(base, base.scaled(1.0), "the default tuning moved under scaled(1.0)");
        // And with every `u8` knob switched off, since zero is the value the
        // floor got wrong and the defaults contain none.
        let off = Tuning {
            coyote_frames: 0,
            jump_buffer_frames: 0,
            step_up: 0,
            dig_reach: 0,
            dig_radius: 0,
            dig_cooldown: 0,
            wade_rows: 0,
            shoulder_grains: 0,
            stroke_cooldown: 0,
            shake_reach: 0,
            mantle_reach: 0,
            ..Default::default()
        };
        assert_eq!(off, off.scaled(1.0), "a tuning with its switches off moved under scaled(1.0)");
        // A real rescale must leave `off` off: scaling is not a way to turn
        // a rule on that the author turned off.
        assert_eq!(off.scaled(2.0).shoulder_grains, 0, "scaling switched the shoulder veto back on");
        assert_eq!(off.scaled(2.0).step_up, 0, "scaling gave him a step-up he was not meant to have");
    }

    #[test]
    fn a_stray_grain_at_chest_height_is_not_a_wall() {
        // **The bug this rule was rewritten for.** Found in `scene=wood`:
        // a single `soil` cell at (108,194), lodged in a canopy, stopped
        // the gnome dead for eleven thousand frames. Step-up cannot save
        // him — lifting slides the offending cell *down* his body toward
        // the wade rows, so a grain at chest height wants a lift of
        // `chest - dy`, which at that height was one more than `step_up`
        // reaches.
        let tuning = Tuning::default();
        let mut world = world_with_a_stray_grain(1);
        let went = distance_walked(&mut world, &tuning, 200);
        assert!(went > 40.0, "one grain should not stop him; he covered {went:.1} cells");

        // **The control, and it is what keeps this test honest.** At
        // `shoulder_grains: 0` the rule is the old veto exactly, and the
        // same grain must still stop him — otherwise this passes for some
        // reason other than the change under test.
        let vetoing = Tuning { shoulder_grains: 0, ..Default::default() };
        let mut world = world_with_a_stray_grain(1);
        let stopped = distance_walked(&mut world, &vetoing, 200);
        assert!(stopped < 30.0, "the old veto should still wall him; he covered {stopped:.1} cells");
    }

    #[test]
    fn a_bank_of_soil_is_still_a_wall() {
        // **Written to fail for the *replacement* artifact**, per the house
        // rule: the risk in counting rather than vetoing is that the wade
        // line stops meaning anything and he strolls through a bank. A
        // course of powder across his whole width is what a drift is made
        // of, and it must stop him at every setting the panel offers —
        // which is why the count is per row and the allowance is capped
        // below `PLAYER_WIDTH`.
        for shoulder in 0..=6u8 {
            let tuning = Tuning { shoulder_grains: shoulder, ..Default::default() };
            let mut world = world_with_floor();
            let soil = world.materials.id_of("soil").expect("soil is compiled in");
            for y in 60..88 {
                for x in 40..52 {
                    world.set(x, y, Cell::new(soil, 0));
                }
            }
            world.player = Some(Player::at(10, 81));
            let went = distance_walked(&mut world, &tuning, 200);
            // Not "he never touches it" — he leans into the face by up to
            // `shoulder` columns, which is the allowance doing exactly what
            // it says and reads as sinking into the edge of a drift. The
            // claim is that he never gets *through*: the bank spans
            // x=40..=51, so clearing it would put his centre past 54, i.e.
            // 44 cells covered.
            assert!(
                went < 40.0,
                "a 12-wide bank must stop him at shoulder_grains={shoulder}, he covered {went:.1} cells"
            );
        }
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
                cells.push(BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0, cracks: 0 });
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
                cells.push(BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0, cracks: 0 });
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
    /// The relationship `SWING_FRAMES`' doc claims, asserted rather than
    /// left as prose — and asserted against the *derivation* the code now
    /// uses, so the constant cannot drift away from it unnoticed.
    #[test]
    fn the_swing_pose_is_half_the_dig_cooldown_at_the_defaults() {
        let t = Tuning::default();
        assert_eq!((t.dig_cooldown / 2).max(1), SWING_FRAMES);
    }

    /// **A dilated gnome is the same gnome, slower.** The shape of a jump is
    /// what a time scale has to preserve, and the formula that guarantees it
    /// is `v^2 / 2g` — one factor of the scale on the velocity, two on the
    /// acceleration.
    ///
    /// Distances must not move at all, which is the other half of "same
    /// gnome": a slow-motion character whose reach shrank would just be a
    /// smaller one.
    #[test]
    fn dilation_preserves_the_shape_of_a_jump_and_every_distance() {
        let base = Tuning::default();
        for slowdown in [2.0f32, 4.0, 8.0] {
            let s = 1.0 / slowdown;
            let d = base.dilated(s);

            // `jump_cells` is `v^2 / 2g`, so it is invariant by construction.
            let (a, b) = (base.jump_impulse.powi(2) / (2.0 * base.gravity), d.jump_impulse.powi(2) / (2.0 * d.gravity));
            assert!((a - b).abs() < 0.01, "at {slowdown}x a jump reaches {b} against {a}");

            // The compounding term: `d^s` applied `1/s` times as often must
            // come back to `d`. This is the assertion that fails for the
            // inverted `powf(1/s)` spelling caught in review.
            let compounded = d.swim_damp.powf(slowdown);
            assert!(
                (compounded - base.swim_damp).abs() < 0.001,
                "at {slowdown}x, swim_damp {} compounds to {compounded} rather than {}",
                d.swim_damp,
                base.swim_damp
            );

            // Rates fall, windows lengthen.
            assert!(d.run_max < base.run_max && d.gravity < base.gravity);
            assert!(d.dig_cooldown > base.dig_cooldown && d.coyote_frames > base.coyote_frames);
            // ...and `u8` still holds them, which is only true because
            // `clock::MAX_SLOWDOWN` is 30. See `Tuning::dilated`.
            assert_eq!(d.dig_cooldown, (base.dig_cooldown as f32 * slowdown).round() as u8);

            // Distances, sizes and dimensionless ratios: untouched.
            assert_eq!(
                (d.dig_reach, d.dig_radius, d.step_up, d.wade_rows, d.mantle_reach, d.shake_reach),
                (base.dig_reach, base.dig_radius, base.step_up, base.wade_rows, base.mantle_reach, base.shake_reach)
            );
            assert_eq!(d.air_control, base.air_control);
            assert_eq!(d.buoyancy, base.buoyancy);
            assert_eq!(d.wade_slowdown, base.wade_slowdown);
            assert_eq!(d.surface_hop, base.surface_hop);
            assert_eq!(d.dig_yield, base.dig_yield);
        }
        // The default is the identity, which every pre-existing player test
        // depends on.
        let same = base.dilated(1.0);
        assert_eq!(same.gravity, base.gravity);
        assert_eq!(same.dig_cooldown, base.dig_cooldown);
        assert_eq!(same.swim_damp, base.swim_damp);
    }

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

    // ---- The belt, the bore, the hammer and the axe -------------------
    //
    // Every test below that cites a *count* was watched failing for the
    // fault it is named for before it was kept — `CLAUDE.md`'s standing
    // check on citing a guard's green. The three that would otherwise be
    // green by default (the bore's shape, the hammer's air swing, the axe
    // against a shake) say inline what was broken to make them red.

    /// A world made of solid rock with a pocket cut for the gnome to stand
    /// in, so a bore in any of the four directions has something to cut.
    fn world_of_rock() -> World {
        let mut world = World::new(Rect::new(0, 0, 255, 199));
        for y in 40..160 {
            for x in 20..230 {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        // The pocket: exactly his rectangle plus a cell of air all round.
        for y in 99..=115 {
            for x in 99..=107 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        world.player = Some(Player::at(103, 107));
        world
    }

    fn stone_cells(world: &World, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> usize {
        let mut n = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if world.get(x, y).material == material::STONE {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn the_bore_goes_the_way_you_point() {
        let world = world_of_rock();
        let p = world.player.as_ref().unwrap();
        let (cx, cy) = p.center();
        for (aim, want) in [
            ((cx + 40, cy), Dir::Right),
            ((cx - 40, cy), Dir::Left),
            ((cx, cy - 40), Dir::Up),
            ((cx, cy + 40), Dir::Down),
            // Dominant axis, not the quadrant: a shallow diagonal is still
            // the horizontal it is mostly pointing along.
            ((cx + 40, cy + 12), Dir::Right),
            ((cx - 12, cy + 40), Dir::Down),
        ] {
            assert_eq!(bore_rect(&world, p, aim, &Tuning::default()).0, want, "aiming at {aim:?} from {:?}", (cx, cy));
        }
    }

    #[test]
    fn the_bore_box_scales_with_the_gnome() {
        // **The guard for a clean merge that was still wrong.** `main` made
        // the gnome's extent per-instance (`Player::at_scaled`) while this
        // branch sized the bore box off `PLAYER_WIDTH`/`PLAYER_HEIGHT`, and
        // nothing conflicted textually: at `cell_scale` 2 the passage came
        // out his 1x size and he could not walk down it. Read the box off
        // the constants again and this goes red.
        let mut world = world_of_rock();
        for k in [1.0_f32, 2.0, 3.0] {
            world.player = Some(Player::at_scaled(103, 107, k));
            let p = world.player.as_ref().unwrap();
            let (_, (x0, y0, x1, y1)) = bore_rect(&world, p, (p.center().0 + 60, p.center().1), &Tuning::default());
            let (bw, bh) = (x1 - x0 + 1, y1 - y0 + 1);
            assert!(bw > p.w && bh > p.h, "at {k}x the box {bw}x{bh} does not clear a {}x{} gnome", p.w, p.h);
            // "Just bigger": clearance stays proportionate rather than
            // becoming a rounding error at 3x or a cavern at 1x.
            assert!(bh - p.h <= 2 * (k.ceil() as i32 + 1), "at {k}x the box is {} taller than he is", bh - p.h);
        }
    }

    #[test]
    fn walking_does_not_change_which_way_he_is_boring() {
        // **The playtest complaint, as a test.** Reported of the first bore:
        // "the direction switches too easy as the gnome moves, he changes
        // position relative to the mouse and then changes how he is
        // digging." The mouse never moved — he walked past it, the vector
        // swept through the diagonal, and the box flipped from a corridor to
        // a shaft on its own.
        //
        // Take `Dir::sticky` back to `Dir::toward` and this goes red.
        // **Open floor, not `world_of_rock`.** The rock scene's pocket is
        // nine cells wide, so ninety ticks of walking moved him three cells
        // and the walk never crossed the diagonal at all -- the assertion
        // below caught that, which is the whole reason it is there.
        let mut world = world_with_floor();
        world.player = Some(Player::at(20, 80));
        let tuning = Tuning::default();
        // A cursor out to his right and below: mostly horizontal, so a
        // corridor -- and near enough that a short walk carries him past it.
        let aim = (44, 92);
        let (dir, _) = bore_rect(&world, world.player.as_ref().unwrap(), aim, &tuning);
        assert_eq!(dir, Dir::Right, "he starts by driving a corridor");
        dig(&mut world, aim, &tuning);

        // Now he walks past the cursor without touching the mouse. The raw
        // offset now points mostly *down*, which is what used to flip it.
        // Walk until he is level with the cursor, not for a fixed time: at
        // `run_max` a fixed ninety ticks carried him 135 cells and clean
        // past it (d=(-80,11)), which the assertion below caught.
        for _ in 0..200 {
            if world.player.as_ref().unwrap().center().0 >= aim.0 - 2 {
                break;
            }
            step(&mut world, PlayerInput { right: true, ..Default::default() }, &tuning);
        }
        let p = world.player.as_ref().unwrap();
        let (dx, dy) = (aim.0 - p.center().0, aim.1 - p.center().1);
        assert!(dy.abs() > dx.abs(), "the walk has to actually cross the diagonal, or this proves nothing: d=({dx},{dy})");
        assert_eq!(Dir::toward(p.center(), aim), Dir::Down, "and the raw rule would now say Down");

        let (dir, _) = bore_rect(&world, p, aim, &tuning);
        assert_eq!(dir, Dir::Right, "walking past the cursor turned his corridor into a shaft");
    }

    #[test]
    fn pointing_somewhere_new_still_changes_the_bore() {
        // The other half: hysteresis must not become a control that ignores
        // the player. A cursor clearly on the other axis switches at once.
        let mut world = world_of_rock();
        let tuning = Tuning::default();
        let (cx, cy) = world.player.as_ref().unwrap().center();
        dig(&mut world, (cx + 40, cy), &tuning);
        assert_eq!(world.player.as_ref().unwrap().bore_dir.map(|(d, _)| d), Some(Dir::Right));
        for aim in [(cx, cy + 40), (cx - 40, cy), (cx, cy - 40)] {
            let want = Dir::toward((cx, cy), aim);
            let p = world.player.as_ref().unwrap();
            let (dir, _) = bore_rect(&world, p, aim, &tuning);
            assert_eq!(dir, want, "a deliberate move to {aim:?} was ignored");
            dig(&mut world, aim, &tuning);
            world.player.as_mut().unwrap().clear_swing_cooldown();
        }
    }

    #[test]
    fn a_horizontal_bore_stands_on_the_floor_he_is_standing_on() {
        // The asymmetry `bore_rect` documents, and the one thing about the
        // box that is not derivable from "his size plus a margin": centring
        // it on him instead cuts two rows out from under the floor and
        // every bite forward becomes a step down.
        let world = world_of_rock();
        let p = world.player.as_ref().unwrap();
        let (_, _, _, feet) = p.bounds();
        let (cx, cy) = p.center();
        for aim in [(cx + 40, cy), (cx - 40, cy)] {
            let (_, (x0, y0, x1, y1)) = bore_rect(&world, p, aim, &Tuning::default());
            assert_eq!(y1, feet, "the passage floor must run out of the ground under his feet");
            assert_eq!(y1 - y0 + 1, p.h + 2, "a passage two cells taller than he is");
            assert_eq!(x1 - x0 + 1, p.w + 2, "and two cells deeper than he is wide");
        }
    }

    #[test]
    fn a_bore_opens_a_passage_he_actually_fits_down() {
        // The claim the whole default rests on. Cut until the box is clear,
        // then put him in it and ask the movement code — not the geometry —
        // whether he fits, which is the question a player asks by walking.
        let mut world = world_of_rock();
        let tuning = Tuning::default();
        let (dir, boxr) = {
            let p = world.player.as_ref().unwrap();
            bore_rect(&world, p, (p.center().0 + 60, p.center().1), &tuning)
        };
        assert_eq!(dir, Dir::Right);
        for _ in 0..16 {
            dig(&mut world, (200, 107), &tuning);
            world.player.as_mut().unwrap().clear_swing_cooldown();
        }
        assert_eq!(stone_cells(&world, boxr), 0, "the box is what the strokes clear, all of it");
        let (x0, y0, _, y1) = boxr;
        // His rectangle placed against the far wall of the passage, sitting
        // on its floor — asked of the movement code with the *tuning's* own
        // wade and shoulder allowances, which is what `step` passes. Asking
        // at zero would be asking whether the passage is surgically clean,
        // and it is not meant to be: a tenth of what was cut is lying in
        // there as spoil by design (`Tuning::dig_yield`), and walking over
        // your own spoil is the thing those two allowances are for.
        // His own extent, read off him rather than off the constants: the
        // gnome scales with the world (`Player::at_scaled`).
        let extent = world.player.as_ref().map(|p| (p.w, p.h)).expect("summoned");
        let stand = (x0 + 1, y1 - extent.1 + 1);
        assert!(
            rect_free(
                &world,
                &Bodies::none(),
                stand.0,
                stand.1,
                extent,
                tuning.wade_rows as i32,
                tuning.shoulder_grains as i32
            ),
            "he must fit inside the passage he just cut: box rows {y0}..={y1}"
        );
    }

    #[test]
    fn one_stroke_is_a_slice_and_not_the_whole_box() {
        // The first law: an outcome is a distribution, not a binary. A
        // press that opened the whole box would be a room appearing.
        let mut world = world_of_rock();
        let tuning = Tuning::default();
        let (dir, boxr) = {
            let p = world.player.as_ref().unwrap();
            bore_rect(&world, p, (p.center().0 + 60, p.center().1), &tuning)
        };
        let before = stone_cells(&world, boxr);
        // Read before the cut: afterwards the working face has moved on,
        // which is the whole behaviour under test.
        let slice = bore_slice(&world, dir, boxr, tuning.bore_bite as i32);
        dig(&mut world, (200, 107), &tuning);
        let after = stone_cells(&world, boxr);
        assert!(after > 0, "one stroke took the whole box: {before} -> {after}");
        assert!(after < before, "one stroke took nothing at all: {before} -> {after}");
        // And it took it off the working face, which is what makes a run of
        // strokes advance rather than swiss-cheese the box.
        assert_eq!(stone_cells(&world, slice), 0, "the working slice is the part that goes");
        // The face has moved: the next stroke is deeper in, not the same
        // three cells again. Anchor the slice on the box edge instead and
        // this is the assertion that goes red.
        let next = bore_slice(&world, dir, boxr, tuning.bore_bite as i32);
        assert_ne!(next, slice, "a held button must drive the passage, not re-cut air");
    }

    #[test]
    fn a_shaft_drops_him_through_the_floor_he_was_standing_on() {
        // Digging down is the direction with no precedent in the free-hand
        // bite, and the one a player reaches for to sink a mineshaft.
        let mut world = world_of_rock();
        let tuning = Tuning::default();
        let start = world.player.as_ref().unwrap().y;
        // Real pacing: one stroke, then the cooldown's worth of ticks, so
        // he has time to fall into what he cut. Digging without stepping
        // measures the geometry and not the mechanic.
        for _ in 0..30 {
            dig(&mut world, (103, 190), &tuning);
            for _ in 0..tuning.dig_cooldown {
                step(&mut world, PlayerInput::default(), &tuning);
            }
        }
        let end = world.player.as_ref().unwrap().y;
        assert!(end > start + PLAYER_HEIGHT as f32, "a shaft must sink him: {start} -> {end}");
    }

    #[test]
    fn a_buried_gnome_digs_out_in_bore_mode_too() {
        // The bore is anchored *outside* his rectangle, so under a pile it
        // would clear a room next door and leave him exactly as entombed.
        // `dig` sends a buried gnome down the free path whatever style is
        // selected; break that dispatch and this goes red.
        let mut world = world_with_floor();
        for y in 60..96 {
            for x in 56..76 {
                world.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        world.player = Some(Player::at(66, 88));
        let tuning = Tuning::default();
        assert_eq!(world.player.as_ref().unwrap().dig_style, DigStyle::Bore, "the default is the case under test");
        step(&mut world, PlayerInput::default(), &tuning);
        assert!(world.player.as_ref().unwrap().buried, "the scene must actually bury him");
        for _ in 0..200 {
            dig(&mut world, (66, 88), &tuning);
            step(&mut world, PlayerInput::default(), &tuning);
            if !world.player.as_ref().unwrap().buried {
                return;
            }
        }
        panic!("a buried gnome never got out with the bore selected");
    }

    #[test]
    fn a_hammer_blow_reports_what_it_broke_and_a_swing_at_nothing_reports_nothing() {
        // The positive control and the null, in one test, because a count
        // that cannot move is indistinguishable from a mechanism that did
        // not fire — and a null is where that hides.
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        world.player.as_mut().unwrap().tool = Tool::Hammer;
        let tuning = Tuning::default();
        let hit = smash(&mut world, (90, 84), &tuning).expect("the first blow is off cooldown");
        assert!(hit.broken > 0, "a blow into a cliff face broke nothing");

        // Same gnome, same tool, swung out over open ground.
        let mut empty = world_with_floor();
        empty.player = Some(Player::at(20, 84));
        empty.player.as_mut().unwrap().tool = Tool::Hammer;
        let air = smash(&mut empty, (20, 40), &tuning).expect("the swing still costs the cooldown");
        assert_eq!(air.broken, 0, "a swing at open sky broke something");
    }

    #[test]
    fn the_hammer_shoves_him_back_only_when_it_lands() {
        let tuning = Tuning::default();
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        world.player.as_mut().unwrap().tool = Tool::Hammer;
        let hit = smash(&mut world, (90, 84), &tuning).expect("off cooldown");
        assert!(hit.broken > 0, "this scene has to land a blow for the recoil to be about anything");
        assert!(world.player.as_ref().unwrap().vx < 0.0, "a landed blow to his right must push him left");

        let mut empty = world_with_floor();
        empty.player = Some(Player::at(20, 84));
        empty.player.as_mut().unwrap().tool = Tool::Hammer;
        // Aimed at open sky, where the strike's chip zone cannot reach
        // the floor: a blow that clips the ground is a landed blow.
        smash(&mut empty, (20, 40), &tuning);
        assert_eq!(empty.player.as_ref().unwrap().vx, 0.0, "a swing at air is not a shove off");
    }

    #[test]
    fn an_axe_cuts_living_wood_where_a_shake_leaves_it_alone() {
        // Paired against `a_shake_never_removes_a_wood_cell`, which is the
        // other half of the same rule: the pick's plant verb keeps the
        // tree, the axe's takes it down.
        fn wood_cells(world: &World) -> usize {
            let wood = world.materials.id_of("wood").unwrap();
            let mut n = 0;
            for y in 60..96 {
                for x in 60..80 {
                    if world.get(x, y).material == wood {
                        n += 1;
                    }
                }
            }
            n
        }
        let mut world = world_with_leafy_tree();
        world.player.as_mut().unwrap().tool = Tool::Axe;
        let tuning = Tuning::default();
        let before = wood_cells(&world);
        let cut = chop(&mut world, (71, 80), &tuning).expect("off cooldown");
        assert!(cut.living, "the stroke landed on the trunk, so it is a living cut");
        assert!(wood_cells(&world) < before, "an axe stroke into a bole took no wood: {before}");
    }

    #[test]
    fn an_axe_cut_through_a_stem_breaks_the_organism_connection() {
        // **The interface between the axe and the felling line, pinned from
        // this side.** `chop` contains no plant-specific code and calls no
        // sever API: it relies on `shatter_to_rubble` -> `World::set`
        // unregistering each cut cell from its organism, after which the
        // tissue above has no organism path to the ground and
        // `plant::anchor_support` finds it unreached.
        //
        // This asserts **that disconnection**, which is what the axe owes
        // the felling line, and deliberately not the fall itself. Turning a
        // disconnected crown into pieces belongs to `structural::tick` and
        // is gated end-to-end by `scripts/acceptance.sh`'s `fell` case on a
        // *grown* tree; a hand-built organism here is not wired for anchor
        // support, so asserting severance in a unit test would be asserting
        // something the scene cannot produce.
        //
        // `an_axe_cuts_living_wood_where_a_shake_leaves_it_alone` asserts
        // wood *left*. This asserts the crown *came away*, which is a
        // different claim and the one another session can break.
        let mut world = world_with_leafy_tree();
        world.player.as_mut().unwrap().tool = Tool::Axe;
        let tuning = Tuning::default();
        let wood = world.materials.id_of("wood").unwrap();

        // 8-connected flood from the stump, over organism cells only --
        // `Grow` places tissue at 8 neighbours, so a reader must traverse 8
        // or it sees fragments that are not there (`CLAUDE.md`).
        let reaches_stump = |w: &World| {
            let mut seen = std::collections::HashSet::new();
            let mut queue: Vec<(i32, i32)> = (70..74).map(|x| (x, 87)).filter(|&(x, y)| w.get(x, y).organism_id() != 0).collect();
            seen.extend(queue.iter().copied());
            while let Some((x, y)) = queue.pop() {
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let n = (x + dx, y + dy);
                    if w.in_bounds(n.0, n.1) && w.get(n.0, n.1).organism_id() != 0 && seen.insert(n) {
                        queue.push(n);
                    }
                }
            }
            // Does any crown tissue, well above the cut, still reach it?
            (62..77).any(|y| (66..79).any(|x| w.get(x, y).material == wood && seen.contains(&(x, y))))
        };
        assert!(reaches_stump(&world), "the crown starts connected to the stump, or this proves nothing");

        for _ in 0..6 {
            chop(&mut world, (71, 80), &tuning);
            world.player.as_mut().unwrap().clear_swing_cooldown();
        }
        assert!(
            !reaches_stump(&world),
            "chopping through the bole left the crown still connected: `shatter_to_rubble` is no \
             longer unregistering cut cells from their organism, so nothing downstream will ever \
             see a severed crown"
        );
    }

    #[test]
    fn an_axe_swung_at_rock_still_chips_it() {
        // No dead click, which is `Tool::Dig`'s own standing rule: a reach
        // may bound where a verb lands and must never decide whether it
        // happens. The axe is *bad* at rock, not inert.
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        world.player.as_mut().unwrap().tool = Tool::Axe;
        let tuning = Tuning::default();
        let before = stone_cells(&world, (70, 78, 80, 90));
        let cut = chop(&mut world, (72, 84), &tuning).expect("off cooldown");
        assert!(!cut.living, "there is nothing alive in this scene");
        assert!(stone_cells(&world, (70, 78, 80, 90)) < before, "the stroke did nothing at all");
    }

    #[test]
    fn the_belt_shares_one_recovery_so_switching_is_not_a_second_swing() {
        // Two timers would have made the belt an exploit: strike, switch,
        // strike again at the sum of both rates. Give `Player` a second
        // cooldown field and this goes red.
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        assert!(dig(&mut world, (90, 84), &tuning).is_some(), "the first blow lands");
        world.player.as_mut().unwrap().tool = Tool::Hammer;
        assert!(smash(&mut world, (90, 84), &tuning).is_none(), "switching tools re-armed the swing");
        world.player.as_mut().unwrap().tool = Tool::Axe;
        assert!(chop(&mut world, (90, 84), &tuning).is_none(), "switching tools re-armed the swing");
    }

    #[test]
    fn swing_dispatches_on_what_he_is_holding() {
        let tuning = Tuning::default();
        for (tool, want) in [(Tool::Pick, "bite"), (Tool::Hammer, "smash"), (Tool::Axe, "chop")] {
            let mut world = world_with_cliff();
            world.player = Some(Player::at(66, 84));
            world.player.as_mut().unwrap().tool = tool;
            let got = match swing(&mut world, (90, 84), &tuning) {
                Some(Blow::Bite(_)) => "bite",
                Some(Blow::Smash(_)) => "smash",
                Some(Blow::Chop(_)) => "chop",
                Some(Blow::Shake(_)) => "shake",
                None => "nothing",
            };
            assert_eq!(got, want, "{} produced the wrong verb", tool.label());
        }
    }

    #[test]
    fn a_pick_pointed_at_a_plant_still_shakes_it_rather_than_cutting() {
        // The pick's two verbs survive the belt. This is what makes the axe
        // a *choice*: shaking is what you do to a tree you want to keep.
        let mut world = world_with_leafy_tree();
        let tuning = Tuning::default();
        assert_eq!(world.player.as_ref().unwrap().tool, Tool::Pick);
        assert!(matches!(swing(&mut world, (71, 80), &tuning), Some(Blow::Shake(_))));
    }

    #[test]
    fn the_swing_bar_empties_on_a_blow_and_fills_back_to_ready() {
        let mut world = world_with_cliff();
        world.player = Some(Player::at(66, 84));
        let tuning = Tuning::default();
        assert_eq!(world.player.as_ref().unwrap().swing_progress(), 1.0, "he starts ready");
        dig(&mut world, (90, 84), &tuning);
        assert_eq!(world.player.as_ref().unwrap().swing_progress(), 0.0, "a blow empties the bar");
        for _ in 0..tuning.dig_cooldown {
            step(&mut world, PlayerInput::default(), &tuning);
        }
        let p = world.player.as_ref().unwrap();
        assert!(p.swing_ready() && p.swing_progress() == 1.0, "the bar must reach full exactly when he is ready");
    }

}
