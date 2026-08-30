//! **Caves: rooms made by collapse, joined by conduits, with a way in.**
//!
//! This module replaces the cave generator wholesale. What it replaces was a
//! Worley `F2 - F1` field thresholded inside a box, and the owner's verdict
//! on it was *"The whole shape and generation of the cave shold be rebuilt
//! from the ground up… The voroni worly patter around the cave should be
//! removed. I know I said that I liked it before but no"* and, on a second
//! card the same minute, three words: ***"Remove the web"***.
//!
//! The diagnosis is `Reports/cave-redesign-2026-08-29.md`: a thresholded
//! field has exactly one shape vocabulary, and it is the field's. Every
//! corridor was a straight Voronoi boundary segment and every junction a
//! three-way Voronoi vertex, at the *same* 8.2 x 3.9 lattice in every world,
//! because `half_w` cancelled in the envelope's own cell size. Retuning could
//! only zoom it, which is why five rounds of tuning moved nothing the owner
//! could see. **Nothing in this module reads a noise field to decide where
//! rock is absent.** If a future change finds itself thresholding one, it has
//! reproduced the thing that was removed.
//!
//! # What a cave is made of now
//!
//! Two objects, and no field.
//!
//! | | what it is | how it is made |
//! |---|---|---|
//! | **Room** | a space you stand in and look around | a dissolution lens, then a roof that falls in until it reaches a bed strong enough to hold its span |
//! | **Conduit** | the passage between two rooms | a shortest path through a cost field built from the strata the rock is already drawn with |
//!
//! ## The room is not drawn
//!
//! [`grow_room`] opens a lens by flooding outward through a **removal cost**
//! -- cheap in weak rock, dear in strong, much cheaper along the bedding than
//! across it -- until a volume budget runs out. Then [`collapse_roof`] sweeps
//! upward: at every row, each ceiling cell is asked whether the run of void
//! beneath it is wider than the bed it is made of can span, and if it is, it
//! falls. The sweep stops where the rock stops failing.
//!
//! Three consequences, and they are the point:
//!
//! * **Shape is a consequence rather than a formula.** A room's ceiling is a
//!   bedding plane because that is where the collapse stopped; its walls step
//!   along whatever the flood could not afford.
//! * **Two rooms differ because their rock differs.** The same seed in
//!   mudstone country and in basalt country produces a low wide room and a
//!   tall narrow one, from the same code and the same draw.
//! * **The rubble on the floor is the volume the roof lost** -- see
//!   [`RUBBLE_RETAINED`] for the one fraction that is not conserved and why.
//!
//! ## Pillars are a designed feature, and the physics is why
//!
//! Stone's `max_unsupported_span` is 16 and its `attached_span_bonus` is 12,
//! so the strongest thing the massif is made of spans a few hundred cells and
//! no more. A room 3-7x the one the owner called small is **wider than any
//! bed can roof**, and that is not a problem to tune away: a cathedral room
//! has pillars. So [`pillar_pitch`] derives the spacing from the beds
//! themselves -- *the span the local rock can actually hold* -- and the room
//! grows around reserved columns that the lens flood is forbidden to take.
//! The collapse then leaves them standing floor-to-ceiling by construction:
//! it only ever eats a cell that has void directly beneath it, and a pillar
//! column has none.
//!
//! **How many pillars a room has is therefore a reading of its rock**, not a
//! parameter: weak country gets many close ones, competent country gets two
//! far apart. See `Reports/worldgen-caves-rebuilt-2026-08-29.md` for the
//! measurement that set the factor.
//!
//! # Reach
//!
//! A system is bounded by [`crate::worldgen::passes::MAX_CAVE_HALF_W`], and
//! that number went **down** while the rooms got much bigger, because the
//! chain runs *downward* through a depth band a thousand rows deep rather
//! than sideways. Rows are free in a column margin; columns are not.
//!
//! # The seal
//!
//! `Reports/dead-ends.md` #28: one grain of sand has deleted an entire cave
//! system, and a collect-verify-write pass that rejects wholesale is how. The
//! rule here is structural rather than procedural -- [`Carvable`] is computed
//! once, *before* anything is placed, as "rock here and rock everywhere
//! within the rind", and no stage may ever take a cell outside it. A sand
//! lens is therefore something the room grows **around** and the conduit
//! routes past. There is nothing to reject, so nothing can be rejected.

use crate::sim::world::World;
use crate::worldgen::noise::{self, Purpose};
use crate::worldgen::passes::{strata_rock_at, CaveEnv, VAULT_RIND};
use crate::worldgen::Ctx;
use std::collections::BinaryHeap;

// ---------------------------------------------------------------------------
// the shape of a system
// ---------------------------------------------------------------------------

/// One room, recorded rather than re-derived.
///
/// `Reports/dead-ends.md` #14: anything a later system needs to know about a
/// generated void must be recorded at generation. `pockets` eating caves and
/// `springs` having nowhere to come out of are both that entry, unfixed.
#[derive(Clone, Debug)]
pub struct Room {
    /// Centre of the seed lens, in world coordinates.
    pub cx: i32,
    pub cy: i32,
    /// `(x0, y0, x1, y1)` of the finished room, world coordinates.
    pub bbox: (i32, i32, i32, i32),
    /// Void cells the room holds.
    pub cells: usize,
    /// Cells the roof lost on the way to standing up.
    pub collapsed: usize,
    /// How far the roof rose, in rows.
    pub rise: i32,
    /// Reserved pillar columns, as `(x, half_width)`.
    pub pillars: Vec<(i32, i32)>,
    /// The size band this room was drawn from -- see [`ROOM_BANDS`].
    pub band: usize,
}

/// One passage, as a polyline with a radius -- the record `springs` and
/// `pockets` need and the shape a keyhole section is cut along.
#[derive(Clone, Debug)]
pub struct Conduit {
    pub points: Vec<(i32, i32)>,
    pub half_w: i32,
    /// True for the passage that reaches daylight.
    pub is_mouth: bool,
}

/// What one system came out as, beside the void mask itself.
#[derive(Default)]
pub struct SystemPlan {
    pub rooms: Vec<Room>,
    pub conduits: Vec<Conduit>,
    /// Where the system daylights, if it does.
    pub mouth: Option<(i32, i32)>,
    /// Cells whose cover must be turned to rock so the entrance has a lintel
    /// and the hillside's soil does not pour into it on frame one.
    pub lintel: Vec<(i32, i32)>,
    /// Envelope cells the seal check does not cover, and there are exactly
    /// two reasons a cell is in here.
    ///
    /// **Named rather than inferred, both of them.** `cave_system` asserts
    /// that every void cell and every cell in its rind is intact rock, and
    /// two things the generator does deliberately break that:
    ///
    /// * **the entrance passage**, whose whole job is to leave the rock; and
    /// * **a lens the roof swallowed** ([`drop_islands`]) -- a pocket of sand
    ///   left standing in mid-air because the carve was forbidden to touch
    ///   it. The seal's purpose is that nothing loose sits flush with a free
    ///   face, and removing the lens outright satisfies that purpose while
    ///   failing its proxy, which reads the world *before* the write.
    ///
    /// Inferring either exemption from position -- "anything near the
    /// surface", "anything not rock" -- would quietly widen it to every case
    /// the check exists for.
    pub breakout: Vec<bool>,
    /// Islands of loose material the roof took down with it.
    pub swallowed: usize,
    /// Rows of breakdown to lay on the floor of each column of the envelope,
    /// indexed the same way `CaveEnv::idx` indexes columns.
    pub rubble: Vec<i32>,
    /// How many room domes stopped rising because they hit [`MAX_DOME_RISE`]
    /// rather than because the rock held. **Printed, not silently absorbed**:
    /// a cap that bounds work is fine and a cap that decides the answer is
    /// the landmine `CLAUDE.md` names twice, and the only way to tell them
    /// apart is to count how often it binds.
    pub domes_capped: usize,
    /// Room lenses that reached their volume budget with the flood still
    /// affordable -- i.e. the budget, not the rock, set the size. Same
    /// reasoning as `domes_capped`.
    pub lenses_capped: usize,
    /// Separate pieces the finished void is in, and how many welds it took to
    /// get there.
    ///
    /// **The number the owner's *"chained together so you can walk directly
    /// from one to the other"* is actually about**, and the one a conduit
    /// count cannot answer: a passage can be drawn for every pair of rooms
    /// and still leave the void in seven pieces, because a drawn passage is
    /// not a connected one. Measured that way before the weld below existed:
    /// eight conduits over seven rooms, and seven components.
    pub pieces: usize,
    pub welds: usize,
}

// ---------------------------------------------------------------------------
// constants
// ---------------------------------------------------------------------------

/// Rock cover a room's roof must keep beneath the surface, in rows.
///
/// **This is where the depth band lives now**, and moving it here is what
/// made an entrance possible at all. `vaults` used to guarantee "deep" by
/// placing the whole *envelope* below `surface + vault_min_depth`, which
/// meant the envelope's top edge was two hundred rows under the ground and
/// the mouth search could not reach daylight from inside it however hard it
/// tried: measured, `mouths 0` on every seed, with the passage running into
/// the envelope wall still underground. The band is a statement about where
/// the *cave* is, not about where a box is, so it is enforced per cell here
/// -- and the envelope is now free to reach the surface, which is the only
/// place a way in can start.
///
/// The mouth passage is the one thing licensed to cross it, on
/// [`Carvable::at_shallow`], and it pays for the licence by carrying a rock
/// lintel wherever it runs under soil ([`SystemPlan::lintel`]).
const MIN_ROOF_COVER: i32 = 90;

/// How tall a breakdown dome may stand relative to the opening it roofs.
///
/// **The rule that stops a dome running away, and it is a shape rather than a
/// budget.** A roof that keeps failing keeps rising, and in a section that is
/// mostly weak beds it will rise until something arbitrary stops it: measured
/// at a flat 300-row cap, one seed brought down 281,410 cells, two domes
/// stopped only because the cap said so, and the render was a black pit with
/// the gnome floating in the middle of it.
///
/// A real breakdown dome is a low arch -- wider than it is tall -- because
/// once the roof has risen as far as the opening is wide it *is* an arch and
/// the load has found its way to the abutments. So the rule is a proportion
/// of the span the cell is roofing, which is a quantity the sweep already has
/// in its hand, and it self-limits: a narrow bay between two pillars stops
/// after a few beds and a wide one is allowed a real dome.
const DOME_ASPECT: f32 = 0.28;


/// The absolute backstop under [`DOME_ASPECT`], in rows.
///
/// **A bound on work, and it is counted when it binds** -- see
/// `SystemPlan::domes_capped`. Exhausting it leaves a room with a lower
/// ceiling, which is the same kind of answer as the rock holding sooner; it
/// never turns a room into something else, and it can never prevent one.
const MAX_DOME_RISE: i32 = 170;

/// Fraction of the collapsed roof that stays on the floor as breakdown.
///
/// **The one thing here that is not conserved, stated rather than hidden.**
/// Rock bulks by about 40% when it breaks, so a room whose roof fell and kept
/// all of it would be a room full of rubble -- which is what a *dead* cave is.
/// An active one has water taking the fines away, and that is why you can
/// stand up in it. A third is enough to read as "the roof came down" and
/// leaves the room a room.
const RUBBLE_RETAINED: f32 = 0.34;

/// Rooms in one system.
/// **Raised with the small bands.** A system of three rooms cannot show a
/// distribution at all; with the smallest chambers costing very little
/// envelope, a system can hold more of them and the spread becomes visible
/// inside one cave rather than only across a world.
const ROOMS_MIN: i32 = 4;
const ROOMS_MAX: i32 = 9;

/// The size bands a room is drawn from, in cells across, and how often each
/// one comes up.
///
/// **A room's size is a draw, not a setting, and getting that wrong is what
/// the first version of this got wrong.** The owner asked for rooms *"3-7x
/// bigger"* than the 145-cell one he called small; the first build forced the
/// biggest room of every system into that range and his verdict on it was
/// *"these all look huge. Huge sometimes more rare is good, but they should
/// not all be this large"* -- the complaint moved from "too small" to "all
/// the same size", which is the *same* complaint. `CLAUDE.md`'s first law:
/// **an outcome is a distribution, not a binary**, and a size everything
/// shares is a binary wearing a number.
///
/// So: mostly small and medium chambers, a large one occasionally, a
/// genuinely huge one rarely -- the same shape `wiki/the-world.md` already
/// demands of standing rock, where you pass a dozen boulders for every spire.
/// A cave worth remembering has to be rare enough to be worth remembering.
///
/// The bands are quoted against 145, the room he called small:
///
/// | band | across | in gnome-heights | in "small rooms" | share |
/// |---|---|---|---|---|
/// | **cell** | 60-140 | 4-10 | 0.4 - 1.0x | 30% |
/// | **chamber** | 140-300 | 10-21 | 1.0 - 2.1x | 34% |
/// | **hall** | 300-500 | 21-36 | 2.1 - 3.4x | 22% |
/// | **great** | 500-720 | 36-51 | 3.4 - 5.0x | 11% |
/// | **cathedral** | 720-950 | 51-68 | 5.0 - 6.5x | **3%** |
///
/// **Five bands, and the two smallest are the round-two correction.** With
/// four bands starting at 150 the world came out *bimodal* -- a thin passage
/// or a vast hall, and nothing between -- and the owner said so twice:
/// *"small tunnels leading to huge caverns. There should be more smaller
/// caverns too."* What was missing is the middle: a space a few times his own
/// height that is a **room** rather than a corridor. The `cell` band is that
/// -- four to ten gnome-heights across, too small to be a hall and far too
/// big to be a passage.
///
/// Stated as an explicit mixture rather than as an exponent on a unit draw,
/// because the thing under review is the *shape of the distribution* and a
/// mixture is the form a reader can check against a census. `vaults detail`
/// prints the per-band counts for exactly that reason.
const ROOM_BANDS: [(f32, i32, i32); 5] = [
    (0.30, 60, 140),
    (0.34, 140, 300),
    (0.22, 300, 500),
    (0.11, 500, 720),
    (0.03, 720, 950),
];

/// The floor of the whole range, derived from the bands so the two cannot
/// drift apart.
const ROOM_W_MIN: i32 = ROOM_BANDS[0].1;

/// Draw one room's width from [`ROOM_BANDS`]. Returns the width and which
/// band it came from, so the pass can census the distribution it produced
/// rather than the one this table claims.
fn draw_room_width(pick: f32, within: f32) -> (i32, usize) {

    let mut acc = 0.0;
    for (i, &(share, lo, hi)) in ROOM_BANDS.iter().enumerate() {
        acc += share;
        if pick < acc || i + 1 == ROOM_BANDS.len() {
            return (lo + (within * (hi - lo) as f32) as i32, i);
        }
    }
    (ROOM_W_MIN, 0)
}

/// Room lens height before collapse, as a fraction of its width. The dome
/// adds the rest.
const ROOM_ASPECT: f32 = 0.34;

/// How much of the local bed's spanning ability a room leaves between
/// pillars.
///
/// **Above 1.0 deliberately, and the first setting had it below.** At 0.8 the
/// bays between pillars are narrower than the beds can already span, so
/// nothing fails: measured, `collapsed 0` over a whole seed and 3,511 cells
/// over another -- the headline mechanism of the rebuild barely firing,
/// because the pillars had been placed close enough together to make it
/// unnecessary. A rule that cannot fire is not a conservative setting of a
/// rule, it is the absence of one.
///
/// At 1.15 the bay is a little wider than the *mean* bed there can hold, so
/// the roof over it fails in the weaker beds and rises until it reaches a
/// competent one -- and 1.6 was tried first and is too much: on one seed the
/// domes ran away, 281,410 cells came down, two of them stopped only because
/// [`MAX_DOME_RISE`] said so, and the render was a black pit with the gnome
/// floating in the middle of it. That is what makes a ceiling a bedding plane instead of a ruled line,
/// and what makes two rooms in one world differ in height because their rock
/// differs. `SystemPlan::domes_capped` is the counter that says whether this
/// has been pushed far enough to run away.
const PILLAR_PITCH_FACTOR: f32 = 1.15;

/// The pitch is clamped so that neither a mudstone room nor a basalt one
/// stops reading as a room: all-pillar and no-pillar are both failures.
const PILLAR_PITCH_MIN: i32 = 120;
const PILLAR_PITCH_MAX: i32 = 340;

/// Pillar half-width, in cells. Drawn per pillar between these.
const PILLAR_HALF_MIN: i32 = 5;
const PILLAR_HALF_MAX: i32 = 20;

/// Coarse lattice the conduit search runs on. Every 4th cell: the passages
/// are ten cells across, so a node every four is finer than the thing it
/// places.
const PATH_STEP: i32 = 4;

/// A conduit's tube half-width and the depth of the canyon incised below it.
///
/// **The section is a keyhole, not a circle**, which is the shape a passage
/// gets when a phreatic tube is later drained and a vadose stream cuts down
/// out of its floor. It is also the only cross-section that is generous
/// overhead and narrow underfoot, which is what makes a corridor read as
/// carved rather than as a drawn tube.
///
/// Sized off the gnome, who is **7 x 14**: the slot he walks in is
/// `2 * SLOT_HALF_W + 1 = 11` across and the clear height over his head is
/// `SLOT_DEPTH + TUBE_HALF_H`, which is at least 22 rows -- 1.5 gnomes, where
/// the shipped generator's median passage was 14-16, *exactly his own
/// height with nothing to spare*.
const TUBE_HALF_W_MIN: i32 = 8;
const TUBE_HALF_W_MAX: i32 = 14;
const SLOT_HALF_W: i32 = 5;
const SLOT_DEPTH_MIN: i32 = 9;
const SLOT_DEPTH_MAX: i32 = 18;

/// How far a system will look sideways for somewhere to daylight, in
/// columns. Bounded because it is a search, and because the pass's declared
/// margin has to cover it.
pub const MOUTH_REACH: i32 = 620;

/// Rows of rock laid over the entrance passage where it runs under soil.
///
/// A cave mouth in a soil-covered hillside is a rock lintel with dirt on top,
/// and without one the blanket pours into the passage on frame one -- soil is
/// a `Powder`. Two cells is the rind; four is a lintel.
const LINTEL_THICK: i32 = 4;

/// Cost of a step across the bedding relative to along it, in the room's
/// dissolution flood.
///
/// The single term that stops a lens being a circle. Water moving through
/// rock follows the bedding plane it is on and crosses one reluctantly, which
/// is why cave passages are wide and low far more often than tall and narrow.
const BEDDING_ANISOTROPY: f32 = 3.4;

/// How much of the removal price the rock's own strength sets. At 0 every
/// rock dissolves alike and the lens is a shape; at this value a basalt sill
/// costs about five times what a mudstone bed does and the lens is a reading
/// of the section.
const HARDNESS_WEIGHT: f32 = 4.0;

/// What a joint buys. A cell on a fracture plane is cheap to remove in any
/// direction, which is where the angular direction changes and the vertical
/// shafts come from.
const JOINT_DISCOUNT: f32 = 0.45;

/// How much the conduit cost field is roughened, and over what wavelength.
///
/// At 0 the passages are grid shortest paths and read as drawn; the term has
/// to be large enough that a detour is genuinely cheaper than the straight
/// line over a stretch several passage-widths long, which is what makes a
/// bend a bend rather than a wobble. The wavelength is a few passage widths
/// for the same reason: shorter and it is texture, longer and the route is
/// straight again between two very slow turns.
const WANDER: f32 = 0.85;
const WANDER_WAVELENGTH: f32 = 90.0;

/// The joint lattice, in cells. **Quantised, and that is a dead end being
/// avoided rather than a free choice**: `Reports/dead-ends.md` records that
/// a smoothly varying lattice pitch is structurally broken wherever the
/// consumer is an identity test between neighbours, and names quantising on
/// a coarse lattice as the repair. Two conjugate directions, drawn per block.
const JOINT_BLOCK: i32 = 96;
const JOINT_SPACING: i32 = 34;

// ---------------------------------------------------------------------------
// the rock: what it can span, and what it costs to take away
// ---------------------------------------------------------------------------

/// How far a bed of this rock can roof an opening, in cells.
///
/// `max_unsupported_span * attached_span_bonus`, read straight off the
/// material rather than invented here. Both numbers already exist and already
/// mean this: the first is how far loose foreground material reaches, and the
/// second is the multiplier the play world's 2D slice earns from the 3D rock
/// the slice does not contain (`MaterialDef::attached_span_bonus`). Their
/// product over the six beds runs **42 (mudstone) to 308 (basalt)**, which is
/// the whole reason two rooms in the same world can differ by a factor of
/// seven without a single parameter moving.
pub(crate) fn bed_span(world: &World, ctx: &Ctx, x: i32, y: i32) -> i32 {
    let m = world.materials.get(strata_rock_at(ctx, x, y));
    (m.max_unsupported_span as i32).saturating_mul(m.attached_span_bonus as i32).clamp(8, 4096)
}

/// Whether `(x, y)` lies on one of the two conjugate joint planes of its
/// block.
///
/// The block is coarse and the draw is per block, so two neighbouring cells
/// ask the same question of the same lattice -- which is the property
/// `Reports/dead-ends.md` says a smoothly varying pitch destroys.
fn on_joint(ctx: &Ctx, x: i32, y: i32) -> bool {
    let bx = x.div_euclid(JOINT_BLOCK);
    let by = y.div_euclid(JOINT_BLOCK);
    let seed = ctx.terrain.seed;
    // Two dips, one leaning each way, drawn per block. Expressed as a run
    // over rise so the plane can be walked without trigonometry.
    let a = 1 + (noise::unit(seed, Purpose::CaveJoint, bx, by) * 3.0) as i32;
    let b = -1 - (noise::unit(seed, Purpose::CaveJoint, bx, by + 7777) * 3.0) as i32;
    let phase = (noise::unit(seed, Purpose::CaveJoint, bx + 31, by) * JOINT_SPACING as f32) as i32;
    let on = |run: i32| (x - run * y + phase).rem_euclid(JOINT_SPACING) == 0;
    on(a) || on(b)
}

// ---------------------------------------------------------------------------
// carvable: the seal, computed once
// ---------------------------------------------------------------------------

/// Where the generator is allowed to take rock away.
///
/// True only where the cell **and its whole rind** are intact rock, and where
/// there is enough cover overhead. Everything downstream is forbidden to
/// leave it, so the seal `cave_system` asserts holds by construction and no
/// stage ever has to reject anything for a breach -- the wholesale-seal defect
/// (`Reports/dead-ends.md` #28) has nowhere to live.
///
/// Built by eroding a rock bitmap, separably, rather than by asking each cell
/// about its own 5x5 neighbourhood: the latter is 25 `World::get` calls per
/// cell over a two-million-cell envelope, which is a bounds check plus a
/// `HashMap` lookup each.
pub(crate) struct Carvable {
    env: CaveEnv,
    /// Rock, rind-eroded. The cover rule is **not** baked in here, because
    /// the entrance passage is licensed to break it and everything else is
    /// not -- see [`Carvable::at`] against [`Carvable::at_shallow`].
    ok: Vec<bool>,
    /// `ok`, eroded again by the largest cross-section a conduit can have.
    ///
    /// **A passage has to be routed through rock that can hold it, not merely
    /// started in some.** Without this the path search only asked whether the
    /// tube fitted *at each node* -- four probe points on the axes, four cells
    /// apart -- so a lens between two nodes was invisible to the search, the
    /// per-cell clip then cut the tube to nothing where it sat, and the
    /// passage came out severed. Measured: five separate void components in a
    /// world whose two systems held five rooms and had a conduit drawn for
    /// every one of them.
    ok_tube: Vec<bool>,
    /// The shallowest row each envelope column may be carved at under the
    /// cover rule.
    min_y: Vec<i32>,
    /// Per envelope column, the **world** rows `ponds` will fill --
    /// `(i32::MAX, i32::MIN)` where it will fill none. See
    /// `passes::MOUTH_POOL_CLEARANCE`: the entrance passage is the one carve
    /// here that is not clipped to the mask, so it has to know where the
    /// water is going to be and wall itself off from it.
    near_water: Vec<(i32, i32)>,
    /// World row of the envelope's centre, so [`Carvable::at_shallow`] can
    /// compare a local row against [`Carvable::near_water`]'s world rows.
    cy: i32,
    /// Cells of a small pocket of loose material the carve is allowed to take
    /// away outright, with its rind, rather than route around.
    ///
    /// **`pockets` writes lenses of sand and gravel through the whole massif,
    /// and a rule that only routes *around* them leaves them hanging.** The
    /// first render of a finished room had half a dozen tan slabs floating in
    /// the black, each a lens with the two cells of rock the rind rule
    /// protects still wrapped round it, stalactites hung underneath by the
    /// speleothem pass. Routing around a lens is right for a big one -- that
    /// is the passage narrowing past an incompetent bed, which is what a real
    /// passage does -- and wrong for a small one, which a falling roof simply
    /// takes with it.
    ///
    /// The seal's proxy reads the world *before* the write and would see sand
    /// at a free face, so these cells are marked exempt
    /// (`SystemPlan::breakout`). The property the seal exists for still holds
    /// -- there is nothing loose left there at all.
    /// Which pocket each soft cell belongs to, 1-based; 0 for anything that
    /// is not one. Kept because a pocket has to be taken **whole**: emptying
    /// the half of it a passage happened to clip would leave the other half
    /// loose against a free face, which is the property the seal exists for.
    soft_id: Vec<u32>,
    /// The cells of each pocket, indexed by `soft_id - 1`.
    soft_cells: Vec<Vec<usize>>,
}

/// The largest pocket of loose material a carve will swallow whole rather
/// than route around, in cells.
///
/// `pockets` lenses run to a few hundred cells; the soil blanket is
/// hundreds of thousands and must never qualify, which is what the
/// border test beside this is really for.
const SWALLOW_MAX: usize = 3000;

impl Carvable {
    pub(crate) fn build(ctx: &Ctx, env: CaveEnv, world: &World, cx: i32, cy: i32, wet: &[(i32, i32)]) -> Self {
        let (gw, gh) = (env.grid_w() as usize, env.grid_h() as usize);
        let mut rock = vec![false; gw * gh];
        let (w, h) = (ctx.terrain.w, ctx.terrain.h);
        for dy in -env.half_h..=env.half_h {
            let py = cy + dy;
            for dx in -env.half_w..=env.half_w {
                let px = cx + dx;
                if px < 0 || px >= w || py < 0 || py >= h {
                    continue;
                }
                // **Intact country rock, not specifically grey stone.** With
                // six rocks in the massif, asking for one of them by id makes
                // a room sunk in a sandstone bed read as breached.
                if !world.materials.get(world.get(px, py).material).rock {
                    continue;
                }
                let plan = ctx.plans[px as usize];
                if py > plan.bedrock_top_y - ctx.terrain.params.vault_bedrock_margin {
                    continue;
                }
                rock[dy_i(env, dx, dy)] = true;
            }
        }
        // Erode by the rind, separably and over prefix sums: a row pass then
        // a column pass, each answering "is there a false in this window" in
        // constant time. The naive `all()` over the window is `O(area x r)`,
        // which at the tube radii below would be a hundred and fifty million
        // reads per system.
        // Small enclosed pockets of loose material count as rock for the
        // carve, and are recorded so the seal check knows why.
        let (soft_id, soft_cells) = swallowable(&rock, gw, gh);
        for (i, &id) in soft_id.iter().enumerate() {
            if id != 0 {
                rock[i] = true;
            }
        }
        let mut ok = erode(&rock, gw, gh, VAULT_RIND as usize, VAULT_RIND as usize);
        // **The envelope's own edge is a hole in that erosion, and it has to
        // be closed here rather than excused at the assertion.** `erode`
        // clamps its window at the grid border (`x.saturating_sub(rx)`), so a
        // cell two columns inside the edge is asked only about the *part of
        // its rind that is inside the envelope* -- and the two columns beyond
        // it, which no stage ever looked at, are whatever `pockets` put there.
        // Measured 2026-08-30: `cave system k=67 at (1325,857) env 638x487:
        // rind cell (-640,34) world (685,891) is "gravel" inside=false`, and
        // the same defect fires at the **shipped 8192x2560 at the shipped
        // density** -- canyon, 1 seed in 16, `worldgen::generate` panicking
        // outright. A lens two cells from a room wall is not a cosmetic
        // breach either: soil and gravel are `Powder`, so it pours in on
        // frame one, which is the whole of `Reports/dead-ends.md` #28.
        //
        // Closed by asking the world directly, over the band the erosion
        // could not see -- ~10k cells against the envelope's 1.6M, so it is
        // free -- and the predicate is the *seal's own* (`Material::rock`),
        // not this function's, because the seal is what has to be satisfied.
        // A pocket outside the envelope reads as loose and correctly clears
        // the cell: `take_touched_pockets` can only empty a pocket it has an
        // id for, and it has none out there.
        //
        // It **narrows the mask, never widens it**, and only within
        // `VAULT_RIND` of the edge -- so a world that does not trip the
        // assertion today generates identically. Verified: `cave_probe` at
        // 16 seeds x 6 presets, shipped size, before and after.
        for dy in -env.half_h..=env.half_h {
            for dx in -env.half_w..=env.half_w {
                let i = dy_i(env, dx, dy);
                if !ok[i] {
                    continue;
                }
                if dx.abs() <= env.half_w - VAULT_RIND && dy.abs() <= env.half_h - VAULT_RIND {
                    continue; // the whole rind is inside, so `erode` already answered
                }
                let mut sealed = true;
                for ry in -VAULT_RIND..=VAULT_RIND {
                    for rx in -VAULT_RIND..=VAULT_RIND {
                        let (nx, ny) = (dx + rx, dy + ry);
                        if nx.abs() <= env.half_w && ny.abs() <= env.half_h {
                            continue; // inside: `erode` covered it
                        }
                        let (px, py) = (cx + nx, cy + ny);
                        if px < 0 || px >= w || py < 0 || py >= h || !world.materials.get(world.get(px, py).material).rock {
                            sealed = false;
                        }
                    }
                }
                ok[i] = sealed;
            }
        }
        let ok = ok;
        // And again by the **narrowest** section a conduit can cut, not the
        // widest. Eroding by the widest was the first version and it is the
        // size-cap landmine in a new costume: at `TUBE_HALF_W_MAX` across and
        // the tube plus its deepest slot down, the mask went false over most
        // of a massif that has `pockets` lenses scattered through it, the path
        // search found no route between four fifths of the rooms, and the pass
        // reported `conduits 1` for four rooms and `mouths 0` for every world.
        // A guarantee that cannot be met is not a stronger guarantee.
        //
        // At the minimum section it is a promise the search can always keep:
        // a passage drawn wider than the minimum has its outer cells clipped
        // where a lens sits close, so it **narrows past the lens** rather than
        // being holed by it -- which is `Reports/dead-ends.md` #28's rule in
        // the form the redesign asks for, and what a real passage does at an
        // incompetent bed.
        let ok_tube = erode(
            &ok,
            gw,
            gh,
            TUBE_HALF_W_MIN as usize,
            (TUBE_HALF_W_MIN + SLOT_DEPTH_MIN) as usize,
        );
        let cover = ctx.terrain.params.vault_min_depth.max(MIN_ROOF_COVER);
        let min_y = (-env.half_w..=env.half_w)
            .map(|dx| {
                let px = (cx + dx).clamp(0, w - 1);
                ctx.plans[px as usize].surface_y + cover - cy
            })
            .collect();
        // The world rows standing water will occupy in each envelope column.
        let near_water = (-env.half_w..=env.half_w)
            .map(|dx| wet.get((cx + dx).clamp(0, w - 1) as usize).copied().unwrap_or((i32::MAX, i32::MIN)))
            .collect();
        Carvable { env, ok, ok_tube, min_y, near_water, cy, soft_id, soft_cells }
    }

    /// Empty every soft pocket the carve reached into, whole, and mark all
    /// of its cells exempt from the seal check.
    ///
    /// "Reached into" means within the rind, not merely overlapping: the seal
    /// asserts about a two-cell dilation of the void, so a pocket two cells
    /// from a passage wall is as much its business as one the passage cut.
    pub(crate) fn take_touched_pockets(&self, void: &mut [bool], exempt: &mut [bool]) {
        let mut touched = vec![false; self.soft_cells.len()];
        for dy in -self.env.half_h..=self.env.half_h {
            for dx in -self.env.half_w..=self.env.half_w {
                if !void[dy_i(self.env, dx, dy)] {
                    continue;
                }
                for ry in -VAULT_RIND..=VAULT_RIND {
                    for rx in -VAULT_RIND..=VAULT_RIND {
                        let (nx, ny) = (dx + rx, dy + ry);
                        if nx.abs() > self.env.half_w || ny.abs() > self.env.half_h {
                            continue;
                        }
                        let id = self.soft_id[dy_i(self.env, nx, ny)];
                        if id != 0 {
                            touched[id as usize - 1] = true;
                        }
                    }
                }
            }
        }
        for (g, group) in self.soft_cells.iter().enumerate() {
            if !touched[g] {
                continue;
            }
            for &i in group {
                void[i] = true;
                exempt[i] = true;
            }
        }
    }

    /// Carvable for a room or an ordinary conduit: rock, rind and cover.
    pub(crate) fn at(&self, dx: i32, dy: i32) -> bool {
        self.at_shallow(dx, dy) && dy >= self.min_y[(dx + self.env.half_w) as usize]
    }

    /// Carvable for a conduit: as [`Carvable::at`], plus room for the whole
    /// cross-section, so a path is routed through rock that can *hold* a
    /// passage rather than merely start one.
    pub(crate) fn tube_at(&self, dx: i32, dy: i32) -> bool {
        self.tube_at_shallow(dx, dy) && dy >= self.min_y[(dx + self.env.half_w) as usize]
    }

    /// The same for the entrance passage, without the cover rule.
    pub(crate) fn tube_at_shallow(&self, dx: i32, dy: i32) -> bool {
        if dx.abs() > self.env.half_w || dy.abs() > self.env.half_h {
            return false;
        }
        self.ok_tube[dy_i(self.env, dx, dy)] && !self.wet_rind(dx, dy)
    }

    /// Whether `ponds` will put standing water at this envelope column and
    /// **world** row.
    ///
    /// Read by [`carve_mouth_run`] rather than by [`Carvable::at_shallow`],
    /// because the entrance run is the one carve in the module that is
    /// deliberately not clipped to the mask at all -- it has to leave the
    /// rock to be an entrance, so it walls itself off from the water instead
    /// of being refused near it.
    pub(crate) fn will_flood(&self, dx: i32, world_y: i32) -> bool {
        if dx.abs() > self.env.half_w {
            return false;
        }
        let (t, b) = self.near_water[(dx + self.env.half_w) as usize];
        world_y >= t && world_y <= b
    }

    /// Whether a local row in this column is water, or within the rind of it.
    ///
    /// **The entrance is clipped away from a lake as well as lintelled
    /// against one**, and both are needed: the lintel covers the breakout
    /// run's own shell, and this covers the shallow conduit that reaches the
    /// breakout's start, which carries no shell at all.
    fn wet_rind(&self, dx: i32, dy: i32) -> bool {
        let (t, b) = self.near_water[(dx + self.env.half_w) as usize];
        if t > b {
            return false;
        }
        let y = dy + self.cy;
        y >= t - VAULT_RIND && y <= b + VAULT_RIND
    }

    /// Carvable for the **entrance passage only**: rock and rind, without the
    /// cover rule.
    ///
    /// A mouth is by definition a passage that comes out, so it is the one
    /// thing licensed to break the cover floor -- and it pays for the licence
    /// by carrying a rock lintel wherever it runs under soil
    /// (`SystemPlan::lintel`), because soil is a `Powder` and an unlintelled
    /// entrance fills with dirt on frame one.
    pub(crate) fn at_shallow(&self, dx: i32, dy: i32) -> bool {
        if dx.abs() > self.env.half_w || dy.abs() > self.env.half_h {
            return false;
        }
        self.ok[dy_i(self.env, dx, dy)] && !self.wet_rind(dx, dy)
    }
}

fn dy_i(env: CaveEnv, dx: i32, dy: i32) -> usize {
    env.idx(dx, dy)
}

/// Which cells belong to a pocket of loose material small enough, and far
/// enough from the envelope's edge, to be taken away whole.
///
/// The border test is what keeps the soil blanket and the open sky out of it:
/// both reach the envelope's edge, so neither is ever a pocket however the
/// size cap is set.
#[allow(clippy::type_complexity)]
fn swallowable(rock: &[bool], gw: usize, gh: usize) -> (Vec<u32>, Vec<Vec<usize>>) {
    let mut seen = vec![false; gw * gh];
    let mut ids = vec![0u32; gw * gh];
    let mut cells: Vec<Vec<usize>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut group: Vec<usize> = Vec::new();
    for start in 0..gw * gh {
        if seen[start] || rock[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        group.clear();
        let mut touches_border = false;
        while let Some(i) = stack.pop() {
            group.push(i);
            let (x, y) = (i % gw, i / gw);
            if x == 0 || y == 0 || x + 1 == gw || y + 1 == gh {
                touches_border = true;
            }
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= gw || ny as usize >= gh {
                    continue;
                }
                let j = ny as usize * gw + nx as usize;
                if seen[j] || rock[j] {
                    continue;
                }
                seen[j] = true;
                stack.push(j);
            }
        }
        if !touches_border && group.len() <= SWALLOW_MAX {
            cells.push(group.clone());
            let id = cells.len() as u32;
            for &i in &group {
                ids[i] = id;
            }
        }
    }
    (ids, cells)
}

/// Morphological erosion of a boolean mask by a rectangle, in `O(area)`.
///
/// Separable: a horizontal pass then a vertical one, each using a prefix sum
/// of the *false* count so a window query is two subtractions rather than a
/// scan.
///
/// **The window is clamped at the grid border, so the result is optimistic
/// there**: a cell within `rx`/`ry` of an edge is judged on the part of its
/// window that exists, and says nothing about the world beyond the grid. That
/// is a hole in any guarantee phrased over the *world* rather than over the
/// mask, and it cost a shipped-size crash -- see the band pass in
/// [`Carvable::build`], which closes it by asking the world directly. Do not
/// "fix" it here by treating out-of-grid as false: `ok_tube` erodes by the
/// tube's whole section, and a false border there would refuse a route the
/// per-cell clip handles correctly.
fn erode(src: &[bool], gw: usize, gh: usize, rx: usize, ry: usize) -> Vec<bool> {
    let mut tmp = vec![false; gw * gh];
    let mut pre = vec![0u32; gw + 1];
    for y in 0..gh {
        for x in 0..gw {
            pre[x + 1] = pre[x] + u32::from(!src[y * gw + x]);
        }
        for x in 0..gw {
            let lo = x.saturating_sub(rx);
            let hi = (x + rx).min(gw - 1);
            tmp[y * gw + x] = pre[hi + 1] - pre[lo] == 0;
        }
    }
    let mut out = vec![false; gw * gh];
    let mut prec = vec![0u32; gh + 1];
    for x in 0..gw {
        for y in 0..gh {
            prec[y + 1] = prec[y] + u32::from(!tmp[y * gw + x]);
        }
        for y in 0..gh {
            let lo = y.saturating_sub(ry);
            let hi = (y + ry).min(gh - 1);
            out[y * gw + x] = prec[hi + 1] - prec[lo] == 0;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// stage A: where the rooms are
// ---------------------------------------------------------------------------

/// A room site before it is a room: a seed point, a target width, and the
/// depth it sits at.
struct Site {
    dx: i32,
    dy: i32,
    width: i32,
    /// Which of [`ROOM_BANDS`] it was drawn from, carried so the pass can
    /// census the distribution rather than restate it.
    band: usize,
}

/// Lay out this system's room sites inside the envelope.
///
/// **Sites are spread by rejection against every site already placed**, so
/// they are neither on a grid nor in a heap -- a chain of rooms all landing in
/// one corner is the failure this exists to prevent, and it is the failure a
/// plain per-site draw actually produces.
///
/// The chain runs **downward** more than sideways, and that is the whole
/// reason this design costs *less* column reach than the one it replaces
/// (see the module doc): the depth band is over a thousand rows deep, and
/// rows are free in a margin measured in columns.
fn room_sites(ctx: &Ctx, env: CaveEnv, k: i32, cx: i32) -> Vec<Site> {
    let seed = ctx.terrain.seed;
    let n = ROOMS_MIN
        + (noise::unit(seed, Purpose::Karst, k, 1) * (ROOMS_MAX - ROOMS_MIN + 1) as f32) as i32;
    // **Draw every width first, then place the largest one first.** Sites are
    // rejected against each other by their own extents, so a big room needs a
    // lot of clear envelope and is rejected far more often than a small one:
    // placing them in draw order truncates the distribution toward small, and
    // measured that way a table asking for 3% huge rooms produced **none in
    // four worlds** and a largest room of 330 cells. Placing the biggest
    // first costs nothing and lets the drawn distribution actually happen --
    // what is left of the truncation then falls on the small rooms, where it
    // does not matter.
    //
    // The sort is total, on `(width descending, band)`, because an unstable
    // sort's tie order is not a function of its comparator alone
    // (`CLAUDE.md`) and two rooms of equal width would otherwise swap on a
    // toolchain change and move every conduit in the world.
    let mut wants: Vec<(i32, usize)> = (0..n)
        .map(|j| {
            draw_room_width(
                noise::unit(seed, Purpose::Karst, k * 977 + j, 2),
                noise::unit(seed, Purpose::Karst, k * 977 + j, 3),
            )
        })
        .collect();
    wants.sort_by_key(|&(w, b)| (std::cmp::Reverse(w), b));

    let mut sites: Vec<Site> = Vec::new();
    // A generous number of attempts per room, and the loop is bounded by them
    // rather than by success: a system in a cramped envelope gets fewer
    // rooms, never an infinite search.
    for a in 0..n * 24 {
        if sites.len() as i32 >= n {
            break;
        }
        let (width, band) = wants[sites.len().min(wants.len() - 1)];
        let u = |c: i32| noise::unit(seed, Purpose::Karst, k * 977 + a, c);
        // **No forced cathedral.** An earlier version forced the first room
        // of every system into the top of the range, to beat the truncation
        // the sort above now handles properly. It is a real effect and that
        // cure was worse: every system then had a huge room in it, which is
        // the size stated as a rule rather than drawn, and the owner's
        // verdict on it was *"these all look huge"*.
        let half = width / 2;
        let vhalf = (width as f32 * ROOM_ASPECT) as i32 / 2;
        // Kept clear of the envelope wall by the room's own half-width, so a
        // room is never clipped by the box -- a sawn-off face is the exact
        // artifact the old edge fade existed to hide.
        let span_x = (env.half_w - half - 8).max(0);
        let span_y = (env.half_h - (width as f32 * ROOM_ASPECT) as i32 - 8).max(0);
        if span_x == 0 || span_y == 0 {
            continue;
        }
        let dx = -span_x + (u(4) * (2 * span_x) as f32) as i32;
        let dy = -span_y + (u(5) * (2 * span_y) as f32) as i32;
        // Rejection: far enough from every site already down that two rooms
        // are two rooms rather than one lumpy one. Measured in each pair's
        // own half-widths, so a big room pushes harder than a small one.
        let clash = sites.iter().any(|s| {
            let sep = (s.dx - dx).abs();
            let vsep = (s.dy - dy).abs();
            let svhalf = (s.width as f32 * ROOM_ASPECT) as i32 / 2;
            // The vertical margin carries the domes, not just the lenses:
            // two rooms whose lenses clear each other by forty rows have
            // their roofs meet the moment either one rises, and a pair of
            // merged domes is one enormous void rather than two rooms.
            // Measured against what a lens can actually reach (`flood_lens`
            // gives it half again its nominal half-width), not against the
            // nominal size.
            sep < (s.width + width) * 3 / 4 + 40 && vsep < (svhalf + vhalf) * 3 / 2 + MAX_DOME_RISE
        });
        if clash {
            continue;
        }
        // The envelope is a box in a *world*: a site whose own footprint has
        // no carvable rock in it at all is not worth growing.
        let _ = cx;
        sites.push(Site { dx, dy, width, band });
    }
    // Deepest first, so room 0 is the shallowest and the mouth search starts
    // from the room nearest daylight. Ties broken by dx explicitly -- an
    // unstable sort's tie order is not a function of its comparator alone
    // (`CLAUDE.md`), and two rooms at one depth would otherwise reorder on a
    // toolchain change and move every conduit in the world.
    sites.sort_by_key(|s| (s.dy, s.dx));
    sites
}

/// How far apart this room's pillars stand, in cells.
///
/// **Derived from the rock, not chosen.** `bed_span` at the room's own depth
/// says how far the beds there can roof an opening; the pitch is a fraction
/// of that. So a room in competent country gets two pillars far apart and one
/// in weak country gets five close together, out of the same code -- which is
/// *"there should be variability between caves"* delivered structurally
/// rather than by widening a draw.
fn pillar_pitch(world: &World, ctx: &Ctx, x: i32, y: i32) -> i32 {
    // Sampled over the beds the dome will actually rise through rather than
    // at one row: a single sample lands in whichever bed the lens top
    // happened to reach, which is a different answer every few cells.
    let mut acc = 0i64;
    let mut n = 0i64;
    let mut yy = y;
    while yy > y - 120 {
        acc += bed_span(world, ctx, x, yy) as i64;
        n += 1;
        yy -= 12;
    }
    let mean = (acc / n.max(1)) as f32;
    ((mean * PILLAR_PITCH_FACTOR) as i32).clamp(PILLAR_PITCH_MIN, PILLAR_PITCH_MAX)
}

// ---------------------------------------------------------------------------
// stage C: the room -- a lens that dissolves, then a roof that falls in
// ---------------------------------------------------------------------------

/// Fixed-point cost, so the flood's ordering is an integer comparison and
/// two equal-cost cells break their tie on an index rather than on whatever
/// a float comparator happens to do with them.
type Cost = u32;
const COST_SCALE: f32 = 64.0;

/// Grow one room: dissolve a lens, then let the roof fall in.
///
/// Returns the room's record; writes into `void`, which is the envelope-wide
/// mask the whole system shares.
#[allow(clippy::too_many_arguments)]
fn grow_room(
    ctx: &Ctx,
    env: CaveEnv,
    world: &World,
    carv: &Carvable,
    cx: i32,
    cy: i32,
    k: i32,
    idx: i32,
    site: &Site,
    void: &mut [bool],
    plan: &mut SystemPlan,
) -> Option<Room> {
    let seed = ctx.terrain.seed;
    let u = |c: i32| noise::unit(seed, Purpose::CaveRoom, k * 613 + idx, c);
    let half = site.width / 2;
    let lens_half_h = ((site.width as f32 * ROOM_ASPECT) as i32 / 2).max(14);

    // --- the pillars, reserved before anything is taken away ---
    let pitch = pillar_pitch(world, ctx, cx + site.dx, cy + site.dy);
    let mut pillars: Vec<(i32, i32)> = Vec::new();
    if site.width > pitch {
        let n = (site.width / pitch).max(1);
        for i in 1..=n {
            // Jittered off the even division by up to a third of the pitch,
            // so a room's pillars are not a ruler line -- which is the
            // complaint `residual.rs` earned for its mirror symmetry.
            let base = -half + i * site.width / (n + 1);
            let j = ((u(20 + i) - 0.5) * (pitch as f32 * 0.34)) as i32;
            let hw = PILLAR_HALF_MIN
                + (u(40 + i) * (PILLAR_HALF_MAX - PILLAR_HALF_MIN + 1) as f32) as i32;
            let px = (base + j).clamp(-half + hw + 6, half - hw - 6);
            pillars.push((px, hw));
        }
        pillars.sort_unstable();
    }
    let is_pillar = |dx: i32| pillars.iter().any(|&(px, hw)| (dx - px).abs() <= hw);

    // --- the lens: a coalescence of lobes, flooded through removal cost ---
    //
    // Several lobes rather than one, because a chamber in real karst is
    // several conduits that grew into each other, and because the rock the
    // lobes fail to eat is the second source of pillars after the reserved
    // ones. Their union is not an outline anyone drew.
    let lobes = 2 + (u(6) * 3.0) as i32;
    let target = (site.width as f32 * lens_half_h as f32 * 1.15) as usize;
    let mut lens_cells = 0usize;
    let mut hit_budget = false;
    for l in 0..lobes {
        let lx = site.dx + ((u(60 + l) - 0.5) * (site.width as f32 * 0.72)) as i32;
        let ly = site.dy + ((u(80 + l) - 0.5) * (lens_half_h as f32 * 0.9)) as i32;
        let budget = (target / lobes as usize).max(64);
        let (added, capped) = flood_lens(
            ctx, env, world, carv, cx, cy, lx, ly, budget, site, half, lens_half_h, &is_pillar, void,
        );
        lens_cells += added;
        hit_budget |= capped;
    }
    if lens_cells < 200 {
        return None; // no room here: the rock the site landed in was not carvable
    }
    if hit_budget {
        plan.lenses_capped += 1;
    }

    // --- the roof falls in ---
    let lens_top = site.dy - lens_half_h - 4;
    let (collapsed, rise, capped) =
        collapse_roof(ctx, env, world, carv, cx, cy, site, half, lens_top, void, plan);
    if capped {
        plan.domes_capped += 1;
    }

    // --- the record ---
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    let mut cells = 0usize;
    for dy in (site.dy - lens_half_h - rise - 8).max(-env.half_h)..=(site.dy + lens_half_h + 8).min(env.half_h) {
        for dx in (site.dx - half - 4).max(-env.half_w)..=(site.dx + half + 4).min(env.half_w) {
            if void[env.idx(dx, dy)] {
                cells += 1;
                x0 = x0.min(cx + dx);
                x1 = x1.max(cx + dx);
                y0 = y0.min(cy + dy);
                y1 = y1.max(cy + dy);
            }
        }
    }
    if cells == 0 {
        return None;
    }
    // Breakdown on the floor, in the volume the roof lost. Spread over the
    // room's open columns and left for the caller's repose clamp.
    let open: Vec<i32> = ((site.dx - half).max(-env.half_w)..=(site.dx + half).min(env.half_w))
        .filter(|&dx| (site.dy - lens_half_h..=site.dy + lens_half_h).any(|dy| void[env.idx(dx, dy)]))
        .collect();
    if !open.is_empty() {
        let depth = ((collapsed as f32 * RUBBLE_RETAINED) / open.len() as f32).round() as i32;
        for dx in open {
            let i = (dx + env.half_w) as usize;
            if i < plan.rubble.len() {
                // Ragged rather than level: a breakdown pile is blocks, and a
                // ruled line of gravel was the exact complaint the shipped
                // floor earned (*"The floor is a dead-straight ruled line"*).
                let j = (noise::unit(seed, Purpose::CaveRoom, cx + dx, cy + site.dy) * 7.0) as i32;
                plan.rubble[i] = plan.rubble[i].max((depth + j - 3).max(0));
            }
        }
    }
    Some(Room {
        cx: cx + site.dx,
        cy: cy + site.dy,
        bbox: (x0, y0, x1, y1),
        cells,
        collapsed,
        rise,
        pillars: pillars.iter().map(|&(px, hw)| (cx + px + site.dx, hw)).collect(),
        band: site.band,
    })
}

/// One dissolution lobe: Dijkstra outward from `(lx, ly)` through the
/// removal cost, taking cells in increasing order until the budget runs out.
///
/// **Three terms, and none of them is a noise texture.** The bed's own
/// strength ([`HARDNESS_WEIGHT`], read off `bed_span`), whether the step
/// crosses the bedding ([`BEDDING_ANISOTROPY`] -- the single term that stops
/// a lens being a circle, because water follows the bedding plane it is on
/// and crosses one reluctantly), and whether the cell sits on a joint
/// ([`JOINT_DISCOUNT`]).
///
/// Returns `(cells taken, whether the budget rather than the rock stopped
/// it)`.
#[allow(clippy::too_many_arguments)]
fn flood_lens(
    ctx: &Ctx,
    env: CaveEnv,
    world: &World,
    carv: &Carvable,
    cx: i32,
    cy: i32,
    lx: i32,
    ly: i32,
    budget: usize,
    site: &Site,
    half: i32,
    lens_half_h: i32,
    is_pillar: &dyn Fn(i32) -> bool,
    void: &mut [bool],
) -> (usize, bool) {
    // The lobe is confined to its room's own footprint: a lens that wandered
    // would merge two rooms into one lumpy space, which is the shape the
    // owner rejected.
    //
    // **But the box is a quarter wider than the room's nominal size, and the
    // budget is not.** Held at exactly the nominal half-width, the flood
    // spends its budget reaching the clip and the room comes out with dead
    // straight sides -- a box, which is the "drawn primitive" complaint in
    // its least excusable form, because nothing drew it and it looks drawn
    // anyway. With room to spare the outline is set by what the flood could
    // afford, which is the rock.
    // Half again, not a quarter. The flood's iso-cost contour is as wide as
    // `BEDDING_ANISOTROPY` makes it -- about 3.4:1 -- and the room's nominal
    // box is 2.9:1, so at a quarter's slack the contour still reached the
    // sides and the room still came out with straight walls. The box has to
    // be the *looser* of the two constraints or it is the one that decides
    // the shape.
    let room = half + half / 2;
    let vroom = lens_half_h + lens_half_h / 2;
    let (bx0, bx1) = ((site.dx - room).max(-env.half_w), (site.dx + room).min(env.half_w));
    let (by0, by1) = (
        (site.dy - vroom).max(-env.half_h),
        (site.dy + vroom).min(env.half_h),
    );
    if lx < bx0 || lx > bx1 || ly < by0 || ly > by1 || !carv.at(lx, ly) {
        return (0, false);
    }
    let gw = (bx1 - bx0 + 1) as usize;
    let gh = (by1 - by0 + 1) as usize;
    let li = |x: i32, y: i32| (y - by0) as usize * gw + (x - bx0) as usize;

    // **The removal price, precomputed once per cell rather than per edge.**
    // `bed_span` resolves a rock through `strata_rock_at`, which is several
    // hashes, and the flood asks about every cell four times over. On top of
    // that the rock only changes when the *band* does -- roughly every
    // `strata_thickness` rows -- so a one-entry cache per column turns ten
    // resolutions into one. Together these are the difference between a lens
    // costing tens of milliseconds and costing hundreds.
    let thickness = ctx.terrain.params.strata_thickness.max(1.0);
    let offs: Vec<f32> = (bx0..=bx1).map(|dx| ctx.terrain.strata_offset(cx + dx)).collect();
    let mut band_cache: Vec<(i32, f32)> = vec![(i32::MIN, 0.0); gw];
    let mut base = vec![0f32; gw * gh];
    for y in by0..=by1 {
        for x in bx0..=bx1 {
            let c = (x - bx0) as usize;
            let band = (((cy + y) as f32 + offs[c]) / thickness).floor() as i32;
            if band_cache[c].0 != band {
                let span = bed_span(world, ctx, cx + x, cy + y) as f32;
                let hard = ((span - 42.0) / (308.0 - 42.0)).clamp(0.0, 1.0);
                band_cache[c] = (band, 1.0 + HARDNESS_WEIGHT * hard);
            }
            let mut p = band_cache[c].1;
            if on_joint(ctx, cx + x, cy + y) {
                p *= JOINT_DISCOUNT;
            }
            base[li(x, y)] = p;
        }
    }

    let mut dist = vec![Cost::MAX; gw * gh];
    let mut done = vec![false; gw * gh];
    // `Reverse` on a `(cost, index)` pair: the index is the explicit tie
    // break the design calls for. Two equal-cost cells must not be a
    // build-to-build coin flip.
    let mut heap: BinaryHeap<std::cmp::Reverse<(Cost, usize)>> = BinaryHeap::new();
    dist[li(lx, ly)] = 0;
    heap.push(std::cmp::Reverse((0, li(lx, ly))));
    let mut taken = 0usize;
    while let Some(std::cmp::Reverse((d, i))) = heap.pop() {
        if done[i] || d != dist[i] {
            continue;
        }
        done[i] = true;
        let x = bx0 + (i % gw) as i32;
        let y = by0 + (i / gw) as i32;
        if !void[env.idx(x, y)] {
            void[env.idx(x, y)] = true;
            taken += 1;
        }
        if taken >= budget {
            return (taken, true);
        }
        for (ddx, ddy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x + ddx, y + ddy);
            if nx < bx0 || nx > bx1 || ny < by0 || ny > by1 {
                continue;
            }
            // The two hard refusals: outside the seal, and inside a pillar.
            // Both are *routed around*, never a reason to stop -- rejecting a
            // breach, never a system.
            if !carv.at(nx, ny) || is_pillar(nx) {
                continue;
            }
            let mut p = base[li(nx, ny)];
            if ddy != 0 {
                p *= BEDDING_ANISOTROPY;
            }
            let step = (p * COST_SCALE) as Cost;
            let cand = d.saturating_add(step.max(1));
            let j = li(nx, ny);
            if cand < dist[j] {
                dist[j] = cand;
                heap.push(std::cmp::Reverse((cand, j)));
            }
        }
    }
    (taken, false)
}

/// Let the roof fall in until the rock holds it.
///
/// **One upward sweep, not an iterated relaxation**, and the difference is
/// what makes it affordable: a cell is only ever asked about the row directly
/// beneath it, so processing rows from the bottom up lets each row see the
/// failures the row below already caused. A cave-in propagates in the one
/// direction a cave-in propagates.
///
/// The rule is *"is the run of void beneath me wider than my bed can span"*,
/// asked per cell against `bed_span` at that cell -- so the ceiling stops at
/// whichever bed is competent enough, which is why a real breakdown room's
/// roof is a bedding plane. **A solid column splits the run at every row**,
/// which is where the pillars do their work: nothing about them is special
/// cased, they simply never have void beneath them, so they are never eaten
/// and they bound every span they stand in.
///
/// Returns `(cells collapsed, rows risen, whether [`MAX_DOME_RISE`] bound
/// it)`.
#[allow(clippy::too_many_arguments)]
fn collapse_roof(
    ctx: &Ctx,
    env: CaveEnv,
    world: &World,
    carv: &Carvable,
    cx: i32,
    cy: i32,
    site: &Site,
    half: i32,
    lens_top: i32,
    void: &mut [bool],
    _plan: &mut SystemPlan,
) -> (usize, i32, bool) {
    let x_lo = (site.dx - half - 2).max(-env.half_w);
    let x_hi = (site.dx + half + 2).min(env.half_w);
    if x_hi <= x_lo {
        return (0, 0, false);
    }
    let y_bot = env.half_h;
    let top_limit = (lens_top - MAX_DOME_RISE).max(-env.half_h + 1);

    // `bed_span` resolves a rock through `strata_rock_at`, which is several
    // hashes; over a nine-hundred-column room and hundreds of rows that is
    // the whole cost of this function. The rock only changes when the *band*
    // does, roughly every `strata_thickness` rows, so one cached answer per
    // column turns ten lookups into one.
    let n = (x_hi - x_lo + 1) as usize;
    let thickness = ctx.terrain.params.strata_thickness.max(1.0);
    let offs: Vec<f32> = (x_lo..=x_hi).map(|dx| ctx.terrain.strata_offset(cx + dx)).collect();
    let mut cache: Vec<(i32, i32)> = vec![(i32::MIN, 0); n];

    let mut collapsed = 0usize;
    let mut highest = lens_top;
    let mut y = y_bot;
    while y > top_limit {
        let mut x = x_lo;
        while x <= x_hi {
            if !void[env.idx(x, y)] {
                x += 1;
                continue;
            }
            let a = x;
            while x <= x_hi && void[env.idx(x, y)] {
                x += 1;
            }
            let run = x - a;
            let ty = y - 1;
            for xx in a..x {
                if void[env.idx(xx, ty)] || !carv.at(xx, ty) {
                    continue;
                }
                let i = (xx - x_lo) as usize;
                let band = (((cy + ty) as f32 + offs[i]) / thickness).floor() as i32;
                if cache[i].0 != band {
                    cache[i] = (band, bed_span(world, ctx, cx + xx, cy + ty));
                }
                // **Every cell of a run is charged with the whole run, and
                // the dome's height is what bounds it.** Charging each cell
                // for its own distance to the nearer abutment instead was
                // tried and is recorded here rather than left to be
                // rediscovered: it is the better statics and it produces no
                // dome at all. The cells near the ends hold, the middle
                // fails, and the row above then sees a run only as wide as
                // what just failed -- which is narrower, so it holds. The
                // arch closes after one row and the collapse reads 218 cells
                // over a whole system against the 1,900 to 100,000 that make
                // a room. A single upward sweep cannot express progressive
                // spalling; what it can express is "this bed cannot roof this
                // opening", and the arch is then bounded by [`DOME_ASPECT`].
                // **The two bounds here are about the arch, not about the
                // rock, so wherever one binds the sweep stops at whatever bed
                // it had reached** -- and about one bed in four is mudstone,
                // whose `bed_span` is 42, the weakest thing the massif is
                // made of. Measured 2026-08-30 on `rolling` at 2048x1300,
                // five seeds: every roofed run in the finished world that
                // exceeded the span of the rock over it was under mudstone,
                // up to 92 cells against 42, while runs of 106, 167 and 60
                // under sandstone, stone and basalt were all comfortably
                // inside theirs. The roof was not too wide; it had stopped in
                // the wrong rock.
                //
                // **Letting it spall on to a competent bed was built,
                // measured and withdrawn** -- see `Reports/dead-ends.md`. An
                // additive allowance of 24 rows past `DOME_ASPECT` cut
                // over-36 roofed runs standing on rock that cannot span them
                // from 11 to 6 across those five seeds, at +11% collapsed
                // cells; and it took `speleothems_never_bridge_a_passage`
                // from 1.79 to **2.56** bridged formations per 100 columns
                // against a 2.4 bar and a 2.78 picket-fence ceiling. Taller
                // rooms are more formations reaching floor to ceiling, and
                // that is the artifact the A3 rebuild exists to avoid. At 12
                // rows it still read 2.41. The roof invariant is not worth a
                // wall of pillars.
                let risen = lens_top - ty;
                if run > cache[i].1 && (risen as f32) < DOME_ASPECT * run as f32 {
                    void[env.idx(xx, ty)] = true;
                    collapsed += 1;
                    highest = highest.min(ty);
                }
            }
        }
        y -= 1;
    }
    let rise = (lens_top - highest).max(0);
    (collapsed, rise, highest <= top_limit)
}

// ---------------------------------------------------------------------------
// stage B/D: the cost field, and the conduits that chain the rooms
// ---------------------------------------------------------------------------

/// The coarse graph a conduit is found on: every [`PATH_STEP`]th cell.
struct Lattice {
    x0: i32,
    y0: i32,
    nx: usize,
    ny: usize,
    /// Step cost *into* this node, horizontally and vertically. Two numbers
    /// because the field is anisotropic and that is the whole geology in it.
    h: Vec<u32>,
    v: Vec<u32>,
    /// Passable for an ordinary conduit: rock, rind and cover.
    open: Vec<bool>,
    /// Passable for the entrance passage: rock and rind only. A superset of
    /// `open`, and the difference is exactly the band a mouth lives in.
    open_shallow: Vec<bool>,
    /// The entrance passage's own step costs, which are **not** the cave's.
    ///
    /// The cave's cost field prices a vadose shaft cheap above the
    /// palaeo-water-table, which is right for a passage water fell down and
    /// wrong for one a gnome walks up: the first mouths this generator cut
    /// were three-hundred-row vertical shafts, which is *"it doesn't look
    /// like I could even enter it"* rebuilt from the ground up. The mouth
    /// search prices horizontal travel cheap everywhere, so the way in is an
    /// incline.
    mh: Vec<u32>,
    mv: Vec<u32>,
}

impl Lattice {
    fn i(&self, ix: usize, iy: usize) -> usize {
        iy * self.nx + ix
    }

    /// Build the anisotropic cost field.
    ///
    /// Three terms, all of them geology the codebase already draws:
    ///
    /// * **Inception horizons.** A soft bed is cheap to travel along and a
    ///   hard one dear to cross, straight off the same `strata_rock_at`
    ///   the shade pass bands the rock with. This is what gives the long
    ///   near-horizontal bedding-parallel galleries that round 3 tried to
    ///   get by shearing a noise frame -- arriving as a consequence instead
    ///   of as a warp.
    /// * **Joints.** Cheap along a fracture, which is where the vertical
    ///   shafts and the angular direction changes come from.
    /// * **The palaeo-water-table.** Above it the water was falling under
    ///   gravity, so *down* is cheap; below it the water was under pressure,
    ///   so *along* is cheap. One term, and it is what makes a cave read as
    ///   having had a history rather than as a route.
    fn build(ctx: &Ctx, env: CaveEnv, world: &World, carv: &Carvable, cx: i32, cy: i32, paleo: i32) -> Self {
        let nx = ((2 * env.half_w) / PATH_STEP + 1) as usize;
        let ny = ((2 * env.half_h) / PATH_STEP + 1) as usize;
        let (x0, y0) = (-env.half_w, -env.half_h);
        let mut lat = Lattice {
            x0,
            y0,
            nx,
            ny,
            h: vec![0; nx * ny],
            v: vec![0; nx * ny],
            open: vec![false; nx * ny],
            open_shallow: vec![false; nx * ny],
            mh: vec![0; nx * ny],
            mv: vec![0; nx * ny],
        };
        for iy in 0..ny {
            let dy = y0 + iy as i32 * PATH_STEP;
            for ix in 0..nx {
                let dx = x0 + ix as i32 * PATH_STEP;
                let i = lat.i(ix, iy);
                // A node is open when the *tube* fits, not merely its centre:
                // a path threaded through a gap narrower than the passage
                // would carve a passage that gets clipped to nothing there,
                // which is a passage that does not connect.
                lat.open[i] = carv.tube_at(dx, dy);
                lat.open_shallow[i] = carv.tube_at_shallow(dx, dy);
                if !lat.open_shallow[i] {
                    continue;
                }
                let span = bed_span(world, ctx, cx + dx, cy + dy) as f32;
                let hard = ((span - 42.0) / (308.0 - 42.0)).clamp(0.0, 1.0);
                let mut base = 1.0 + HARDNESS_WEIGHT * hard;
                if on_joint(ctx, cx + dx, cy + dy) {
                    base *= JOINT_DISCOUNT;
                }
                // **A shortest path on a square lattice is a straight line,
                // and it reads as one.** The owner's verdict on the first
                // conduits: *"The tunnels and caves are too boxy. They read
                // more planned than natural, especially the tunnels."* He is
                // describing the algorithm exactly -- a route that minimises
                // a sum over a grid has no reason to wander, so it comes out
                // as straight runs meeting at angles, which is what a plan
                // looks like.
                //
                // The cure is in the *field*, not in a filter afterwards:
                // rounding the corners of a straight tunnel gives a straight
                // tunnel with rounded corners. A low-frequency roughness on
                // the traversal cost means the cheapest route is genuinely
                // not the straight one, so the passage meanders because
                // going round is cheaper -- which is also why a real
                // passage meanders.
                base *= 1.0
                    + WANDER
                        * (noise::fbm_2d(
                            ctx.terrain.seed,
                            Purpose::CaveRoom,
                            (cx + dx) as f32 / WANDER_WAVELENGTH,
                            (cy + dy) as f32 / WANDER_WAVELENGTH,
                            3,
                        ) * 2.0
                            - 1.0);
                // Above the palaeo table it is a vadose shaft, below it a
                // phreatic gallery.
                let vadose = cy + dy < paleo;
                let (fh, fv) = if vadose { (2.6, 0.55) } else { (0.55, 2.6) };
                lat.h[i] = ((base * fh) * COST_SCALE) as u32 + 1;
                lat.v[i] = ((base * fv) * COST_SCALE) as u32 + 1;
                lat.mh[i] = ((base * 0.5) * COST_SCALE) as u32 + 1;
                lat.mv[i] = ((base * 3.2) * COST_SCALE) as u32 + 1;
            }
        }
        lat
    }

    /// Dijkstra from one node. Ties break on the node index, explicitly:
    /// two equal-cost paths are otherwise a build-to-build coin flip and
    /// same-build determinism is gone.
    fn search(&self, src: usize) -> (Vec<u32>, Vec<u32>) {
        self.search_on(&self.open, &self.h, &self.v, src)
    }

    /// The same search on the shallow mask and the mouth's own costs.
    fn search_shallow(&self, src: usize) -> (Vec<u32>, Vec<u32>) {
        self.search_on(&self.open_shallow, &self.mh, &self.mv, src)
    }

    fn search_on(&self, open: &[bool], h: &[u32], v: &[u32], src: usize) -> (Vec<u32>, Vec<u32>) {
        let mut dist = vec![u32::MAX; self.nx * self.ny];
        let mut prev = vec![u32::MAX; self.nx * self.ny];
        if !open[src] {
            return (dist, prev);
        }
        let mut heap: BinaryHeap<std::cmp::Reverse<(u32, usize)>> = BinaryHeap::new();
        dist[src] = 0;
        heap.push(std::cmp::Reverse((0, src)));
        while let Some(std::cmp::Reverse((d, i))) = heap.pop() {
            if d != dist[i] {
                continue;
            }
            let (ix, iy) = (i % self.nx, i / self.nx);
            for (ddx, ddy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let jx = ix as i32 + ddx;
                let jy = iy as i32 + ddy;
                if jx < 0 || jy < 0 || jx as usize >= self.nx || jy as usize >= self.ny {
                    continue;
                }
                let j = self.i(jx as usize, jy as usize);
                if !open[j] {
                    continue;
                }
                let step = if ddy == 0 { h[j] } else { v[j] };
                let cand = d.saturating_add(step);
                if cand < dist[j] {
                    dist[j] = cand;
                    prev[j] = i as u32;
                    heap.push(std::cmp::Reverse((cand, j)));
                }
            }
        }
        (dist, prev)
    }

    fn node_of(&self, dx: i32, dy: i32) -> usize {
        let ix = (((dx - self.x0) / PATH_STEP).max(0) as usize).min(self.nx - 1);
        let iy = (((dy - self.y0) / PATH_STEP).max(0) as usize).min(self.ny - 1);
        self.i(ix, iy)
    }

    /// The nearest open node to `(dx, dy)`, searched outward in rings so a
    /// room centre that happens to land on a lens still gets a portal.
    fn open_near(&self, dx: i32, dy: i32) -> Option<usize> {
        self.open_near_on(&self.open, dx, dy)
    }

    fn open_near_shallow(&self, dx: i32, dy: i32) -> Option<usize> {
        self.open_near_on(&self.open_shallow, dx, dy)
    }

    fn open_near_on(&self, open: &[bool], dx: i32, dy: i32) -> Option<usize> {
        let i = self.node_of(dx, dy);
        if open[i] {
            return Some(i);
        }
        let (cx, cy) = (i % self.nx, i / self.nx);
        for r in 1..24usize {
            for (jx, jy) in ring(cx, cy, r, self.nx, self.ny) {
                let j = self.i(jx, jy);
                if open[j] {
                    return Some(j);
                }
            }
        }
        None
    }

    fn coords(&self, i: usize) -> (i32, i32) {
        (
            self.x0 + (i % self.nx) as i32 * PATH_STEP,
            self.y0 + (i / self.nx) as i32 * PATH_STEP,
        )
    }
}

/// The cells of a square ring at radius `r`, in a fixed order so the search
/// above is deterministic.
fn ring(cx: usize, cy: usize, r: usize, nx: usize, ny: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let (cx, cy, r) = (cx as i32, cy as i32, r as i32);
    for dy in -r..=r {
        for dx in -r..=r {
            if dx.abs() != r && dy.abs() != r {
                continue;
            }
            let (x, y) = (cx + dx, cy + dy);
            if x >= 0 && y >= 0 && (x as usize) < nx && (y as usize) < ny {
                out.push((x as usize, y as usize));
            }
        }
    }
    out
}

/// Chain the rooms: a spanning tree plus one loop, then a keyhole cut along
/// each path.
///
/// A **tree plus one extra edge**, not a tree: real karst has both branchwork
/// and maze, and a system with no loop reads as a corridor with rooms hung off
/// it. The tree is what guarantees the owner's *"chained together so you can
/// walk directly from one to the other"* -- every room is reachable from every
/// other -- and the extra edge is what stops it reading as a diagram.
#[allow(clippy::too_many_arguments)]
fn chain_rooms(
    ctx: &Ctx,
    env: CaveEnv,
    carv: &Carvable,
    lat: &Lattice,
    cx: i32,
    cy: i32,
    k: i32,
    rooms: &[Room],
    void: &mut [bool],
    plan: &mut SystemPlan,
) {
    if rooms.len() < 2 {
        return;
    }
    let ports: Vec<Option<usize>> =
        rooms.iter().map(|r| lat.open_near(r.cx - cx, r.cy - cy)).collect();
    let searches: Vec<Option<(Vec<u32>, Vec<u32>)>> =
        ports.iter().map(|p| p.map(|s| lat.search(s))).collect();

    // Prim, over room indices, with the cost of an edge being the path cost
    // the search already found. Deterministic: the argmin breaks ties on the
    // room index, which is itself fixed by `room_sites`' explicit sort.
    let n = rooms.len();
    let mut in_tree = vec![false; n];
    in_tree[0] = true;
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for _ in 1..n {
        let mut best: Option<(u32, usize, usize)> = None;
        for (a, search) in searches.iter().enumerate() {
            if !in_tree[a] {
                continue;
            }
            let Some((dist, _)) = search else { continue };
            for (b, port) in ports.iter().enumerate() {
                if in_tree[b] {
                    continue;
                }
                let Some(pb) = *port else { continue };
                let d = dist[pb];
                if d == u32::MAX {
                    continue;
                }
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, a, b));
                }
            }
        }
        let Some((_, a, b)) = best else { break };
        in_tree[b] = true;
        edges.push((a, b));
    }
    // One extra edge, if there is a pair not already joined: the cheapest.
    // A loop is a place you can come back to from the other side, which is
    // the difference between exploring and retracing.
    if n >= 3 {
        let mut best: Option<(u32, usize, usize)> = None;
        for (a, search) in searches.iter().enumerate() {
            let Some((dist, _)) = search else { continue };
            for (b, port) in ports.iter().enumerate().skip(a + 1) {
                if edges.contains(&(a, b)) || edges.contains(&(b, a)) {
                    continue;
                }
                let Some(pb) = *port else { continue };
                let d = dist[pb];
                if d != u32::MAX && best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, a, b));
                }
            }
        }
        if let Some((_, a, b)) = best {
            edges.push((a, b));
        }
    }

    for (n_edge, &(a, b)) in edges.iter().enumerate() {
        let (Some((_, prev)), Some(pb)) = (&searches[a], ports[b]) else { continue };
        let Some(pts) = walk_back(lat, prev, ports[a].unwrap_or(pb), pb) else { continue };
        let u = noise::unit(ctx.terrain.seed, Purpose::Karst, k * 37 + n_edge as i32, 9);
        let tube = TUBE_HALF_W_MIN + (u * (TUBE_HALF_W_MAX - TUBE_HALF_W_MIN + 1) as f32) as i32;
        let slot = SLOT_DEPTH_MIN
            + (noise::unit(ctx.terrain.seed, Purpose::Karst, k * 37 + n_edge as i32, 11)
                * (SLOT_DEPTH_MAX - SLOT_DEPTH_MIN + 1) as f32) as i32;
        carve_conduit(env, carv, &pts, tube, slot, false, ctx.terrain.seed, void);
        plan.conduits.push(Conduit {
            points: pts.iter().map(|&(dx, dy)| (cx + dx, cy + dy)).collect(),
            half_w: tube,
            is_mouth: false,
        });
    }
}

/// Follow the predecessor chain back from `to` to `from`, in envelope
/// coordinates and in walking order.
fn walk_back(lat: &Lattice, prev: &[u32], from: usize, to: usize) -> Option<Vec<(i32, i32)>> {
    if prev[to] == u32::MAX && to != from {
        return None;
    }
    let mut out = Vec::new();
    let mut i = to;
    let mut guard = 0;
    loop {
        out.push(lat.coords(i));
        if i == from {
            break;
        }
        let p = prev[i];
        if p == u32::MAX {
            return None;
        }
        i = p as usize;
        guard += 1;
        if guard > lat.nx * lat.ny {
            return None; // a cycle is impossible in a shortest-path forest; belt and braces
        }
    }
    out.reverse();
    Some(out)
}

/// A slow 0..1 wobble along a bore, for the radius to ride.
fn seed_wobble(seed: u64, x: i32, y: i32) -> f32 {
    noise::fbm_2d(seed, Purpose::CaveJoint, x as f32 / 70.0, y as f32 / 70.0, 2).clamp(0.0, 1.0)
}

/// Cut a keyhole section along a polyline.
///
/// **The cross-section is not a circle.** A cave passage that has been both
/// flooded and drained is a wide phreatic tube with a narrow vadose canyon
/// incised down out of its floor, and that shape is compression-and-release
/// in a single section -- the thing round 5 tried to get by stamping a room on
/// a corridor. It is also the only section that is generous over the head and
/// narrow underfoot, which is what makes a passage read as carved by water
/// rather than drawn by a brush.
///
/// **Clipped per cell to `carvable`, never rejected.** A sand lens beside the
/// route narrows the passage past it, exactly as a real passage narrows past
/// an incompetent bed -- `Reports/dead-ends.md` #28's rule, in the form the
/// redesign asks for: shrink a radius, never delete a system.
#[allow(clippy::too_many_arguments)]
fn carve_conduit(
    env: CaveEnv,
    carv: &Carvable,
    pts: &[(i32, i32)],
    tube: i32,
    slot: i32,
    shallow: bool,
    seed: u64,
    void: &mut [bool],
) -> usize {
    let mut written = 0;
    let ok = |dx: i32, dy: i32| if shallow { carv.at_shallow(dx, dy) } else { carv.at(dx, dy) };
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let steps = ((b.0 - a.0).abs()).max((b.1 - a.1).abs()).max(1);
        for s in 0..=steps {
            let px = a.0 + (b.0 - a.0) * s / steps;
            let py = a.1 + (b.1 - a.1) * s / steps;
            // **The bore is not a constant width.** A tube of one radius
            // driven along a route is a pipe however the route bends, and a
            // pipe reads as drilled. A real passage pinches at a competent
            // bed and opens into a bell where two joints cross, so the radius
            // rides a slow noise along the run -- half as wide at its
            // narrowest, half again at its widest.
            let g = seed_wobble(seed, px, py);
            let tube = ((tube as f32) * (0.55 + 0.95 * g)).round() as i32;
            let tube = tube.max(SLOT_HALF_W + 2);
            let slot = ((slot as f32) * (0.6 + 0.8 * (1.0 - g))).round() as i32;
            for dy in -tube..=(tube + slot) {
                // **The union of the bell and the stem, not one then the
                // other.** Written as an if/else first, and that version has
                // a one-cell throat where the ellipse closes at `dy == tube`:
                // a section the gnome cannot stand in, in the middle of a
                // passage advertised as walkable along its whole length.
                let ell = if dy.abs() <= tube {
                    let t = dy as f32 / tube as f32;
                    ((tube as f32) * (1.0 - t * t).max(0.0).sqrt()).round() as i32
                } else {
                    0
                };
                let stem = if dy >= 0 { SLOT_HALF_W } else { 0 };
                let halfw = ell.max(stem);
                for dx in -halfw..=halfw {
                    let (qx, qy) = (px + dx, py + dy);
                    if qx.abs() > env.half_w || qy.abs() > env.half_h {
                        continue;
                    }
                    if !ok(qx, qy) {
                        continue;
                    }
                    let i = env.idx(qx, qy);
                    if !void[i] {
                        void[i] = true;
                        written += 1;
                    }
                }
            }
        }
    }
    written
}

// ---------------------------------------------------------------------------
// stage E: the mouth
// ---------------------------------------------------------------------------

/// Half-width of the arch cut through a pillar's base, and its height.
///
/// **A floor-to-ceiling pillar is a wall in a side-on world**, and that is
/// the one thing about pillars this engine's geometry forces. A real
/// breakdown pillar usually *is* pierced at the base -- the water that left it
/// standing was still running past its foot -- so the arch is not a
/// concession, it is the same feature seen in section. Structurally the legs
/// either side still carry the roof down to the floor, which is what the
/// collapse rule and `structural.rs` both need; play-wise you walk under it.
const ARCH_HALF_W: i32 = 6;
const ARCH_H: i32 = 26;
/// The narrowest a pillar leg may be left, in cells. Both legs must survive
/// the arch, or the pillar is a pendant hanging on nothing.
const LEG_MIN: i32 = 5;

/// How far the entrance passage will run outward to reach daylight, and the
/// most ground it will climb through on the way.
///
/// The two are one constraint: the run rises one row every other column, so
/// a passage that starts `MOUTH_MAX_CLIMB` rows under the rock needs twice
/// that many columns to surface. **The climb is a hard filter on which node
/// the search may leave from, not a term in its score**, and it was a term
/// first: weighted against path cost, the search picked a node six hundred
/// rows deep, the run hit the envelope wall still underground, and the pass
/// recorded a mouth anyway. `mouths` then counted *attempts*.
///
/// One in two is a slope you walk up. A shaft is not.
const MOUTH_BREAKOUT: i32 = 320;
const MOUTH_MAX_CLIMB: i32 = 130;

/// Drive a passage from the shallowest room to the open air.
///
/// **This is the half of the redesign the owner can see without a debug
/// harness.** `Reports/cave-redesign-2026-08-29.md` §3.4: there is no cave
/// entrance in this game -- the depth band starts two hundred rows down and
/// `cave_system` *asserts* its envelope is sealed stone, so `viewshot` has to
/// mine a shaft before it can photograph one. Every cave verdict on record
/// was given on a picture of a place the player cannot reach, which is the
/// deepest reason six rounds of cave work produced no playtest reaction. He
/// asked for *"cave openings"* by name.
///
/// The passage is found by the same search as any other conduit, on the
/// shallow mask (the cover rule is what it exists to break), and then walks
/// outward down the local slope, rising one row every other column until it
/// is out in the air. One in two is walkable; a shaft is not.
#[allow(clippy::too_many_arguments)]
fn drive_mouth(
    ctx: &Ctx,
    env: CaveEnv,
    carv: &Carvable,
    lat: &Lattice,
    world: &World,
    cx: i32,
    cy: i32,
    room: &Room,
    void: &mut [bool],
    plan: &mut SystemPlan,
) {
    let Some(src) = lat.open_near_shallow(room.cx - cx, room.cy - cy) else { return };
    let (dist, prev) = lat.search_shallow(src);
    let w = ctx.terrain.w;

    // The best node to leave from: cheap to reach, and with little ground
    // left to climb through once we get there. The second term is what stops
    // the search picking a node under a hilltop, which is reachable and two
    // hundred rows from anywhere.
    let mut best: Option<(i64, usize)> = None;
    for (i, &d) in dist.iter().enumerate() {
        if d == u32::MAX {
            continue;
        }
        let (dx, dy) = lat.coords(i);
        let px = cx + dx;
        if px < 0 || px >= w || (px - cx).abs() > MOUTH_REACH {
            continue;
        }
        let plan_c = ctx.plans[px as usize];
        // Rock top: the blanket sits on it, and a passage that reaches it is
        // one lintel away from the sky.
        let rock_top = plan_c.surface_y + plan_c.soil_depth;
        let climb = (cy + dy) - rock_top;
        if !(0..=MOUTH_MAX_CLIMB).contains(&climb) {
            // Above the rock (not in the massif at all), or too deep for the
            // breakout to climb out of. A hard filter, not a penalty -- see
            // `MOUTH_MAX_CLIMB`.
            continue;
        }
        // Cost in the same units as the climb, so the two are comparable at
        // all: one lattice step is `COST_SCALE`-ish, and a row of climb is
        // worth about a step.
        let score = (d as i64) / COST_SCALE as i64 + (climb as i64) * 3;
        if best.is_none_or(|(bs, _)| score < bs) {
            best = Some((score, i));
        }
    }
    let Some((_, node)) = best else { return };
    let Some(pts) = walk_back(lat, &prev, src, node) else { return };

    // The conduit up to the last node, on the shallow mask.
    let tube = TUBE_HALF_W_MIN + 2;
    carve_conduit(env, carv, &pts, tube, SLOT_DEPTH_MIN, true, ctx.terrain.seed, void);

    // The breakout: outward down the local slope, rising as it goes.
    let (mx, my) = lat.coords(node);
    let here = (cx + mx).clamp(0, w - 1);
    let look = crate::worldgen::passes::MOUTH_SLOPE_LOOK;
    let left = ctx.plans[(here - look).clamp(0, w - 1) as usize].surface_y;
    let right = ctx.plans[(here + look).clamp(0, w - 1) as usize].surface_y;
    // Downhill: `surface_y` grows downward, so the larger side is the lower
    // ground. A mouth wants to be in the side of a hill, not on top of it.
    let dir = if right >= left { 1 } else { -1 };

    // **Both ways, and take the one that gets out sooner.** The direction was
    // picked once from the local slope and committed to, which is wrong in a
    // world this flat: over the couple of hundred columns a breakout runs, the
    // ground the passage was heading downhill into turns and starts climbing,
    // and the run then chews through a quarter of a mile of hillside without
    // surfacing. Trying both and measuring is one extra walk of at most three
    // hundred cheap steps, and it is the difference between a mouth in a bank
    // and an adit under a field.
    let a = try_breakout(ctx, env, cx, cy, mx, my, dir);
    let b = try_breakout(ctx, env, cx, cy, mx, my, -dir);
    let run = match (a, b) {
        (Some(p), Some(q)) => {
            if p.len() <= q.len() {
                p
            } else {
                q
            }
        }
        (Some(p), None) => p,
        (None, Some(q)) => q,
        (None, None) => return,
    };
    if run.len() < 4 {
        return;
    }
    // **Only a run that actually got out counts.** `plan.mouth` used to be
    // set from the last point of the run whatever that point was, so a run
    // that ran into the envelope wall still underground was recorded as a
    // mouth and the pass printed `mouths 1`. A counter that counts attempts
    // is the failure this repo names twice over: arithmetically correct, and
    // about a different question than the one asked.
    let (ex, ey) = *run.last().expect("checked non-empty");
    let out_x = (cx + ex).clamp(0, w - 1);
    if cy + ey > ctx.plans[out_x as usize].surface_y - 4 {
        return;
    }
    let opened = carve_mouth_run(ctx, env, carv, world, &run, cx, cy, void, plan);
    if opened == 0 {
        return;
    }
    plan.mouth = Some((cx + ex, cy + ey));
    plan.conduits.push(Conduit {
        points: run.iter().map(|&(dx, dy)| (cx + dx, cy + dy)).collect(),
        half_w: tube,
        is_mouth: true,
    });
}

/// Walk outward from `(mx, my)` in one direction until the passage is out in
/// the air, or give up.
///
/// Returns `None` rather than a partial run: a breakout that did not reach
/// daylight is not a mouth, and recording one anyway is how the pass came to
/// print `mouths 1` for a passage that ran into the envelope wall still two
/// hundred rows underground.
fn try_breakout(
    ctx: &Ctx,
    env: CaveEnv,
    cx: i32,
    cy: i32,
    mx: i32,
    my: i32,
    dir: i32,
) -> Option<Vec<(i32, i32)>> {
    let w = ctx.terrain.w;
    let mut run: Vec<(i32, i32)> = Vec::new();
    let (mut x, mut y) = (mx, my);
    for t in 0..MOUTH_BREAKOUT {
        let px = cx + x;
        if px < 1 || px >= w - 1 || x.abs() > env.half_w || y.abs() > env.half_h {
            return None;
        }
        // **World row, not envelope row.** Written as a bare `y` first, and
        // `y` here is an offset from `cy` while `surface_y` is an absolute
        // row: the comparison was then true on the first step of every run, so
        // every run was one point long, every mouth was rejected for being too
        // short, and the pass reported `mouths 0` on every seed with no error
        // anywhere. The two coordinate systems in this module are
        // envelope-local and world, and a comparison that mixes them
        // type-checks perfectly.
        let wy = cy + y;
        let sy = ctx.plans[px as usize].surface_y;
        run.push((x, y));
        if wy <= sy - 6 && dry_shoulder(ctx, px) {
            return Some(run); // out in the air, on ground that stays dry
        }
        x += dir;
        // One row in two: a ramp a gnome walks up. A shaft is not an entrance
        // you can use, and anything steeper than this is a shaft with steps.
        if t % 2 == 0 {
            y -= 1;
        }
    }
    None
}

/// Whether a column is somewhere a cave mouth can open without filling up.
///
/// **Two ways for an entrance to be under water, and the first render of one
/// found both.** `ponds` runs *after* `vaults`, so a mouth that surfaced in a
/// hollow came out under a pond, with the water pouring down the passage and
/// the gnome standing on it; and a column whose ground sits below the water
/// table is wet whether or not a pond found it.
///
/// So the run keeps walking until it reaches ground that is dry and is not a
/// dip -- a shoulder, which is where a cave mouth belongs anyway. It is not a
/// preference term in a score: a mouth in a pond is not a worse mouth, it is
/// a drain.
fn dry_shoulder(ctx: &Ctx, px: i32) -> bool {
    let w = ctx.terrain.w;
    let here = ctx.plans[px as usize];
    if here.surface_y >= here.table_y {
        return false; // the ground itself is under the table
    }
    // `surface_y` grows downward, so a hollow is a column whose ground is
    // *lower* than its neighbours'.
    const LOOK: i32 = 40;
    const SLACK: i32 = 10;
    [-LOOK, LOOK].iter().all(|&d| {
        let n = ctx.plans[(px + d).clamp(0, w - 1) as usize].surface_y;
        here.surface_y <= n + SLACK
    })
}

/// How far above and below the entrance run's centre line the cut reaches.
///
/// Named because [`try_breakout`] has to know them: it is the swath, not the
/// centre line, that decides whether a step would open a lake.
const MOUTH_SHELL_UP: i32 = 12;
const MOUTH_SHELL_DOWN: i32 = 9;

/// Cut the entrance run itself, and lintel it.
///
/// The run is the only carve in the module that is **not** clipped to intact
/// rock, because it has to leave the rock to be an entrance. What it pays for
/// that is the shell: every non-rock cell within [`LINTEL_THICK`] of the
/// passage is recorded for the writer to turn into the local bed. Soil is a
/// `Powder`; an unlintelled adit under a soil blanket is full of dirt on
/// frame one, and the first version of this did exactly that.
#[allow(clippy::too_many_arguments)]
fn carve_mouth_run(
    ctx: &Ctx,
    env: CaveEnv,
    carv: &Carvable,
    world: &World,
    run: &[(i32, i32)],
    cx: i32,
    cy: i32,
    void: &mut [bool],
    plan: &mut SystemPlan,
) -> usize {
    let half_w = 6;
    let up = MOUTH_SHELL_UP;
    let down = MOUTH_SHELL_DOWN;
    // The shell is thicker over the passage than beside it. A lintel is what
    // stops the blanket falling in, and that is a job for the rock *above*;
    // two cells at the shoulder is a rind, and the full four all the way round
    // turns a hundred-column adit into twenty-six thousand cells of hillside
    // rewritten as stone.
    let side = 2;
    let mut opened = 0;
    for &(px, py) in run {
        for dy in -up..=down {
            for dx in -half_w..=half_w {
                let (qx, qy) = (px + dx, py + dy);
                if qx.abs() > env.half_w || qy.abs() > env.half_h {
                    continue;
                }
                let (wx, wy) = (cx + qx, cy + qy);
                if wx < 0 || wx >= ctx.terrain.w || wy < 0 || wy >= ctx.terrain.h {
                    continue;
                }
                // **Above the ground is not part of the cave**, and marking it
                // as void would make the sky one of the system's own cavities:
                // the floor pass would lay gravel in the topmost run of every
                // mouth column, and the waterline would count the open air as
                // a chamber and flood to it. Those cells are already empty; the
                // passage reaches daylight through them without owning them.
                if wy < ctx.plans[wx as usize].surface_y - 1 {
                    // **Skipped for being above this column's ground -- but a
                    // cell above the ground is where a lake goes.** These
                    // cells are inside the passage box, so the shell loop
                    // below skips them as "the passage itself" and they were
                    // the one place neither the cut nor the lintel reached:
                    // `rolling` seed 1, open passage cell (989, 419) flush
                    // against (988, 420) holding water, which is the lake at
                    // x <= 988 with the entrance cut through its bank.
                    if carv.will_flood(qx, wy) {
                        plan.lintel.push((wx, wy));
                    }
                    continue;
                }
                // Never cut into bedrock: it is the anchor material and the
                // world's floor.
                if world.get(wx, wy).material == crate::sim::material::BEDROCK {
                    continue;
                }
                let i = env.idx(qx, qy);
                plan.breakout[i] = true;
                if !void[i] {
                    void[i] = true;
                    opened += 1;
                }
            }
        }
        // The shell.
        for dy in -(up + LINTEL_THICK)..=(down + side) {
            for dx in -(half_w + side)..=(half_w + side) {
                if dy >= -up && dy <= down && dx.abs() <= half_w {
                    continue; // the passage itself
                }
                let (wx, wy) = (cx + px + dx, cy + py + dy);
                if wx < 0 || wx >= ctx.terrain.w || wy < 0 || wy >= ctx.terrain.h {
                    continue;
                }
                if (px + dx).abs() <= env.half_w && (py + dy).abs() <= env.half_h {
                    plan.breakout[env.idx(px + dx, py + dy)] = true;
                }
                let m = world.get(wx, wy).material;
                if m == crate::sim::material::BEDROCK {
                    continue;
                }
                if m == crate::sim::material::EMPTY {
                    // **An empty cell that `ponds` is going to fill is cover
                    // too, and it is the one kind this shell could not see.**
                    // `ponds` runs after `vaults` and fills a hollow to the
                    // *planned* ground, so a passage cut through the bank of
                    // one is a plug pulled out of a lake -- measured on
                    // `rolling` seed 1 at 2048x1300, the lake at x 783-988
                    // emptying into a passage at x 989-1011, **1,469 cells
                    // still moving 120 frames after generation and 0 with the
                    // entrance switched off**. Lintelling it is the same
                    // remedy soil already gets, for the same reason and at
                    // the same thickness: a few cells of the lake's edge
                    // become rock, the passage stays where it is, and the two
                    // are no longer connected. Refusing the passage instead
                    // was built and measured first, and it cost **half the
                    // world's entrances** -- 6 systems with a mouth over 8
                    // `rolling` seeds at the shipped size, down to 3.
                    if carv.will_flood(px + dx, wy) {
                        plan.lintel.push((wx, wy));
                    }
                    continue;
                }
                if !world.materials.get(m).rock {
                    plan.lintel.push((wx, wy));
                }
            }
        }
    }
    opened
}

// ---------------------------------------------------------------------------
// the whole system
// ---------------------------------------------------------------------------

/// Carve one cave system: rooms, then the chain, then the way out.
///
/// Returns the envelope-wide void mask and the system's own record, or `None`
/// when the rock the placement landed in could not hold a room at all.
pub(crate) fn carve(
    ctx: &Ctx,
    env: CaveEnv,
    world: &World,
    k: i32,
    cx: i32,
    cy: i32,
    wet: &[(i32, i32)],
) -> Option<(Vec<bool>, SystemPlan)> {
    let carv = Carvable::build(ctx, env, world, cx, cy, wet);
    let sites = room_sites(ctx, env, k, cx);
    if sites.is_empty() {
        return None;
    }
    let mut void = vec![false; env.area()];
    let mut plan = SystemPlan {
        rubble: vec![0; env.grid_w() as usize],
        breakout: vec![false; env.area()],
        ..Default::default()
    };
    for (i, site) in sites.iter().enumerate() {
        if let Some(room) =
            grow_room(ctx, env, world, &carv, cx, cy, k, i as i32, site, &mut void, &mut plan)
        {
            plan.rooms.push(room);
        }
    }
    if plan.rooms.is_empty() {
        return None;
    }

    // The palaeo-water-table: one number per system, and it is what makes
    // the passages above it shafts and the ones below it galleries.
    let paleo = cy
        + ((noise::unit(ctx.terrain.seed, Purpose::Karst, k, 3) - 0.5) * env.half_h as f32) as i32;
    let lat = Lattice::build(ctx, env, world, &carv, cx, cy, paleo);
    chain_rooms(ctx, env, &carv, &lat, cx, cy, k, &plan.rooms.clone(), &mut void, &mut plan);

    // Pierce the pillars and lay a walking floor across every room, **after**
    // the collapse rather than before it. Before, the arch would put void
    // under the pillar's own body and the collapse would eat it from
    // underneath, leaving a pendant hanging on nothing; after, the pillar has
    // already done its structural work and what is left is the hole you walk
    // through.
    let rooms = plan.rooms.clone();
    for room in &rooms {
        open_floor(env, &carv, cx, cy, room, &mut void);
    }

    // Take the tool marks off, without going back to a stamped shape.
    smooth_walls(env, &carv, &mut void, &plan.breakout);

    // One walkable place, verified rather than assumed.
    let (pieces, welds) = weld_pieces(env, &carv, ctx.terrain.seed, &mut void);
    plan.pieces = pieces;
    plan.welds = welds;

    // The way in, from the shallowest room -- `room_sites` sorted them, so
    // room 0 is nearest the surface.
    let mut shallowest = 0usize;
    for (i, r) in plan.rooms.iter().enumerate() {
        if r.cy < plan.rooms[shallowest].cy {
            shallowest = i;
        }
    }
    let room = plan.rooms[shallowest].clone();
    drive_mouth(ctx, env, &carv, &lat, world, cx, cy, &room, &mut void, &mut plan);

    // --- the two tidy-ups, and they have to be last ---
    //
    // Anything the carve left hanging in mid-air goes with it, over the whole
    // envelope rather than room by room: a lens can be left floating by a
    // conduit's keyhole just as easily as by a dome, and a per-room version
    // cleared the rooms and left the passages hung with slabs.
    drop_islands(env, &mut void, &mut plan);
    // **A pocket the carve touched is taken whole**, and every cell of it is
    // then exempt from the seal -- whose proxy reads the world *before* the
    // write and would otherwise see sand at a free face.
    //
    // **After every stage that can create void, and that ordering is the
    // whole of it.** Run before the welds and the mouth, this took the
    // pockets those two stages had not reached yet, and the next void cut
    // beside an untouched lens tripped the seal assertion -- `rind cell
    // (254,-210) is not intact rock`, on the first world with a mouth in it.
    carv.take_touched_pockets(&mut void, &mut plan.breakout);

    // The one wholesale refusal left, and it is about *nothing being there*
    // rather than about a breach: a handful of cells in bad rock is not a
    // system. `vaults` answers it by drawing a fresh placement, not by
    // moving this one -- see its retry loop.
    let cells = void.iter().filter(|&&v| v).count();
    if cells < env.min_system_cells() {
        return None;
    }
    Some((void, plan))
}

/// Take down anything the room left hanging in mid-air.
///
/// **`pockets` writes lenses of sand and gravel through the massif, and
/// `Carvable` forbids carving them or the two cells of rock around them** --
/// so a lens inside a room's dome survives the collapse as an island floating
/// in the void, with a rock rind holding it together and stalactites hung
/// under it by the speleothem pass. Seen in the first render of a finished
/// room: half a dozen tan slabs hanging in the black.
///
/// Physically the answer is not subtle: when a roof falls in it takes what
/// was in it. So anything inside the room that cannot be reached from outside
/// the room without crossing void is removed, and its volume joins the
/// breakdown on the floor like everything else the roof lost.
fn drop_islands(env: CaveEnv, void: &mut [bool], plan: &mut SystemPlan) {
    let (dx0, dx1) = (-env.half_w, env.half_w);
    let (dy0, dy1) = (-env.half_h, env.half_h);
    let gw = (dx1 - dx0 + 1) as usize;
    let li = |x: i32, y: i32| (y - dy0) as usize * gw + (x - dx0) as usize;
    let gh = (dy1 - dy0 + 1) as usize;
    // Flood the *solid* over the box from its border inward. Anything the
    // flood never reaches is enclosed by void, which is what an island is.
    let mut seen = vec![false; gw * gh];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for y in dy0..=dy1 {
        for x in dx0..=dx1 {
            let border = x == dx0 || x == dx1 || y == dy0 || y == dy1;
            if border && !void[env.idx(x, y)] && !seen[li(x, y)] {
                seen[li(x, y)] = true;
                stack.push((x, y));
            }
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < dx0 || nx > dx1 || ny < dy0 || ny > dy1 {
                continue;
            }
            if seen[li(nx, ny)] || void[env.idx(nx, ny)] {
                continue;
            }
            seen[li(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }
    let mut taken = 0usize;
    for y in dy0..=dy1 {
        for x in dx0..=dx1 {
            let i = env.idx(x, y);
            if void[i] || seen[li(x, y)] {
                continue;
            }
            void[i] = true;
            plan.breakout[i] = true; // the seal reads the world before the write
            taken += 1;
            let c = (x + env.half_w) as usize;
            if c < plan.rubble.len() {
                plan.rubble[c] += 1;
            }
        }
    }
    plan.swallowed += taken;
}

/// Smooth the cave's surfaces, without fitting a shape to them.
///
/// **The owner's verdict on the first entrance was *"The opening is fine.
/// Everything should be smoother"***, and it reads wider than the entrance:
/// a room whose boundary is whatever a collapse stopped at has a one-cell
/// staircase everywhere the ceiling changed bed, and a lens grown through a
/// cost field has a jagged rim.
///
/// **It must not be smoothed by fitting a primitive to it.** He rejected
/// *"slightly modified circles or ovals"* by name, and the whole claim of a
/// collapse-shaped room is that nothing drew it. So this is a **filter over
/// the shape the physics produced**, not a replacement for it: a majority
/// vote over a 7x7 window, which fills a one- or two-cell notch and shaves a
/// one- or two-cell spur while leaving every feature wider than the window
/// exactly where it was. The pillars (ten cells and up), the arches through
/// them (thirteen across) and the passages (eleven and up) all pass through
/// it untouched.
///
/// Additions are clipped to carvable rock as everything else is, so the
/// filter cannot smooth its way through the seal.
fn smooth_walls(env: CaveEnv, carv: &Carvable, void: &mut [bool], exempt: &[bool]) {
    const R: i32 = 3;
    let (gw, gh) = (env.grid_w(), env.grid_h());
    // Row sums of the void, then a windowed count -- the same separable trick
    // the rind erosion uses, because a naive 7x7 vote is forty-nine reads per
    // cell over a two-million-cell envelope.
    let mut rows = vec![0u8; (gw * gh) as usize];
    for y in 0..gh {
        let mut pre = 0u32;
        let base = (y * gw) as usize;
        let mut acc = vec![0u32; gw as usize + 1];
        for x in 0..gw as usize {
            pre += u32::from(void[base + x]);
            acc[x + 1] = pre;
        }
        for x in 0..gw {
            let lo = (x - R).max(0) as usize;
            let hi = (x + R).min(gw - 1) as usize;
            rows[base + x as usize] = (acc[hi + 1] - acc[lo]) as u8;
        }
    }
    let mut out = vec![false; (gw * gh) as usize];
    for x in 0..gw {
        for y in 0..gh {
            let mut n = 0u32;
            let mut cells = 0u32;
            for j in (y - R).max(0)..=(y + R).min(gh - 1) {
                n += u32::from(rows[(j * gw + x) as usize]);
                cells += ((x + R).min(gw - 1) - (x - R).max(0) + 1) as u32;
            }
            out[(y * gw + x) as usize] = n * 2 > cells;
        }
    }
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            let i = env.idx(dx, dy);
            if out[i] == void[i] {
                continue;
            }
            if out[i] {
                // Growing into rock: only where the carve was allowed to go.
                if carv.at(dx, dy) || exempt[i] {
                    void[i] = true;
                }
            } else {
                void[i] = false;
            }
        }
    }
}

/// Join whatever the carve left in separate pieces, and say how many there
/// were.
///
/// **A drawn passage is not a connected one.** `chain_rooms` builds a
/// spanning tree over the rooms and cuts a keyhole along every edge of it,
/// and that is still not the property the owner asked for: measured on
/// `rolling` seed 1, eight conduits over seven rooms and **seven separate
/// void components**. A conduit's cross-section is clipped per cell to
/// carvable rock, so a lens close to the route narrows it -- and where the
/// route passes a room's edge at an angle, or a lobe of a room ends short of
/// its own centre, the narrowing can go to nothing.
///
/// So the void is labelled and anything not in the largest piece is welded to
/// it with a straight tube. The count is reported either way (`pieces`): a
/// weld that has to fire on every system means something upstream is wrong,
/// and only the number says so.
fn weld_pieces(env: CaveEnv, carv: &Carvable, seed: u64, void: &mut [bool]) -> (usize, usize) {
    const MIN_PIECE: usize = 400;
    const MAX_WELDS: usize = 12;
    let mut welds = 0usize;
    let mut pieces = 0usize;
    for _ in 0..MAX_WELDS {
        let groups = components(env, void);
        pieces = groups.iter().filter(|g| g.len() >= MIN_PIECE).count();
        if groups.len() < 2 {
            break;
        }
        // Largest first, then the next largest that is worth joining.
        let mut order: Vec<usize> = (0..groups.len()).collect();
        // Deterministic: size descending, ties on the group's first cell,
        // which is raster order. An unstable sort's tie order is not a
        // function of its comparator alone (`CLAUDE.md`), so the key is total.
        order.sort_by_key(|&i| (std::cmp::Reverse(groups[i].len()), groups[i][0]));
        let main = &groups[order[0]];
        let Some(&other) = order.get(1) else { break };
        if groups[other].len() < MIN_PIECE {
            break;
        }
        // The closest pair, found over a subsample of each side: the exact
        // pair is `O(n*m)` on components of tens of thousands of cells, and a
        // weld a few cells off the true minimum is the same weld.
        let step = |n: usize| (n / 600).max(1);
        let (sa, sb) = (step(main.len()), step(groups[other].len()));
        let mut best: Option<(i64, usize, usize)> = None;
        for &i in main.iter().step_by(sa) {
            let (ax, ay) = env_xy(env, i);
            for &j in groups[other].iter().step_by(sb) {
                let (bx, by) = env_xy(env, j);
                let d = ((ax - bx) as i64).pow(2) + ((ay - by) as i64).pow(2);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, i, j));
                }
            }
        }
        let Some((_, i, j)) = best else { break };
        let (ax, ay) = env_xy(env, i);
        let (bx, by) = env_xy(env, j);
        carve_conduit(env, carv, &[(ax, ay), (bx, by)], TUBE_HALF_W_MIN, SLOT_DEPTH_MIN, false, seed, void);
        welds += 1;
    }
    (pieces, welds)
}

/// Envelope coordinates of a flat index.
fn env_xy(env: CaveEnv, i: usize) -> (i32, i32) {
    let gw = env.grid_w();
    ((i as i32 % gw) - env.half_w, (i as i32 / gw) - env.half_h)
}

/// Connected components of the void, 8-connected -- the neighbourhood the
/// carve writes at, and the one the player's own movement uses.
fn components(env: CaveEnv, void: &[bool]) -> Vec<Vec<usize>> {
    let n = void.len();
    let gw = env.grid_w();
    let gh = env.grid_h();
    let mut seen = vec![false; n];
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..n {
        if seen[start] || !void[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut group = Vec::new();
        while let Some(i) = stack.pop() {
            group.push(i);
            let (x, y) = ((i as i32) % gw, (i as i32) / gw);
            for dy in -1..=1i32 {
                for dx in -1..=1i32 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                        continue;
                    }
                    let j = (ny * gw + nx) as usize;
                    if seen[j] || !void[j] {
                        continue;
                    }
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        out.push(group);
    }
    out
}

/// Cut a walking floor across one room, arching through every pillar.
///
/// Two jobs in one sweep, and they are the same job: the room has to be
/// **one** walkable space, and a pillar in a side-on world is a wall unless
/// something goes through it. The legs either side of each arch are kept at
/// no less than [`LEG_MIN`], so the pillar still carries its roof down to the
/// floor -- which is the whole reason it is there.
fn open_floor(env: CaveEnv, carv: &Carvable, cx: i32, cy: i32, room: &Room, void: &mut [bool]) {
    let (x0, y0, x1, y1) = room.bbox;
    let (dx0, dx1) = ((x0 - cx).max(-env.half_w), (x1 - cx).min(env.half_w));
    let (dy0, dy1) = ((y0 - cy).max(-env.half_h), (y1 - cy).min(env.half_h));
    if dx1 < dx0 || dy1 < dy0 {
        return;
    }
    // **Per column, not one level for the whole room.** A single level taken
    // from the bounding box's floor is the deepest point anywhere in the
    // room, and cutting a corridor at that depth across the whole width digs
    // a ruled trench through fresh rock under every part of the room that was
    // shallower -- the artifact this whole rebuild exists to remove, arriving
    // in the one place a shortcut was taken.
    let mut last: Option<i32> = None;
    for dx in dx0..=dx1 {
        // This column's own floor, when it has one; otherwise the floor the
        // last open column had, which is what carries the walking level
        // through a pillar.
        let own = (dy0..=dy1).rev().find(|&dy| void[env.idx(dx, dy)]);
        let floor = match (own, last) {
            (Some(f), _) => {
                last = Some(f);
                f
            }
            (None, Some(f)) => f,
            (None, None) => continue,
        };
        // Inside a pillar, only the arch opens; outside one, the whole width.
        // The legs either side are what carry the roof to the ground.
        let in_leg = room.pillars.iter().any(|&(px, hw)| {
            let d = (cx + dx - px).abs();
            let arch = (hw - LEG_MIN).clamp(0, ARCH_HALF_W);
            d <= hw && d > arch
        });
        if in_leg {
            continue;
        }
        for dy in (floor - ARCH_H).max(-env.half_h)..=floor {
            if carv.at(dx, dy) {
                void[env.idx(dx, dy)] = true;
            }
        }
    }
}
