//! Cells to pixels.
//!
//! The simulation never writes colours. It stores a material id and a shade
//! index, and this module resolves those into RGBA at draw time. Keeping the
//! two apart is what lets M6 swap in a GPU pipeline with lighting and bloom
//! without touching a single movement rule.

use std::collections::HashSet;

use crate::sim::cell::{Cell, AMBIENT_TEMPERATURE};
use crate::sim::chunk::{ChunkCoord, Rect, CHUNK_SIZE};
use crate::sim::material;
use crate::sim::organism;
use crate::sim::particle::ParticleSystem;
use crate::sky::{self, Sky};
use crate::sim::rng;
use crate::sim::world::World;

/// Colour shown for positions outside the world.
const VOID: [u8; 4] = [12, 12, 16, 255];

/// What empty space *inside* the ground looks like: unlit rock behind the
/// opening. Dark enough to read as depth, light enough to stay separate from
/// `VOID` and from a night sky.
const UNDERGROUND: [u8; 4] = [31, 29, 33, 255];

/// What an underground void draws as while the `F11` reveal is on. Magenta,
/// because nothing generated or placed in this world is magenta — the same
/// "a colour the scene cannot produce" reasoning as the debug overlays.
const REVEAL_VOID: [u8; 4] = [232, 62, 214, 255];

/// How many rows it takes to go from open daylight to full `UNDERGROUND`
/// below a roof.
///
/// This used to be one row, and that is the other half of the "way too
/// intense" report: a room went from sky to near-black across a single cell
/// boundary the instant it was enclosed, and the mouth of every cave was a
/// flat black cutout rather than an opening. Light does not stop at a lintel;
/// it reaches in and falls off, and a ramp is both what that looks like and
/// the cheapest possible way to draw it — the depth is already known, since
/// the column's frozen ground surface is exactly the row it is measured
/// from.
///
/// Twenty-four cells is three field blocks, and it is set by eye rather than
/// from anything physical: shallower and a built room still reads as a black
/// slot, deeper and a genuine cave stops reading as dark at all. It costs
/// nothing per frame and it is a pure function of position, so it cannot
/// disagree between a renderer that has history and a fresh one — which is
/// what `dirty_rect_skip_is_pixel_identical_to_a_full_redraw` requires and
/// what sank the stateful version of the skyline itself.
const CAVE_FADE_DEPTH: i32 = 24;

/// What glow-lit cave air blends toward — a pale warm off-white, deliberately
/// *not* the crystal's own cold blue. The lining stays the coldest, brightest
/// thing in the chamber and the air around it reads as illuminated by it
/// rather than as more crystal; tinting the air the material's colour was the
/// obvious first pick and made the whole vug read as one solid mass.
const GLOW_AIR_TINT: [u8; 3] = [226, 222, 208];

/// How much a full-strength glow (`field::MAX_LIGHT` on the light channel)
/// lifts a solid cell's lit colour: `1.0 + l * GLOW_SOLID_LIFT`. At crystal's
/// 1.8 the wall of a vug gets about ×1.4 — visibly warmed, well short of
/// blown out — and the falloff of the diffused halo does the shaping. The
/// same channel read that `sky` uses for daylight would double-count the sun,
/// so this multiplies *after* `apply_light` and the depth grade instead: glow
/// is the one light that must win against depth, since a sealed chamber is
/// exactly where the depth ramp sits at its floor.
const GLOW_SOLID_LIFT: f32 = 0.9;

/// Rows over which solid terrain fades from full surface light to
/// `DEPTH_LIGHT_FLOOR`. The 2026-08 world review's single most consistent
/// graphics finding: deep rock drew at identical brightness to surface rock,
/// so the lower half of every strip read as flat wallpaper — a finished sky
/// over an unlit cutaway. This is the cutaway's vertical light axis.
///
/// Same contract as `CAVE_FADE_DEPTH` above, and it must stay that way: a
/// pure function of `(x, y, horizon[x])` where the horizon is frozen at
/// genesis, so it cannot disagree between a renderer that has history and a
/// fresh one, and it cannot defeat the dirty-rect skip (nothing here varies
/// frame to frame). Sixty-four rows is set by eye against the review strips:
/// one viewport-height of descent reads as "getting deep" without the
/// surface band itself visibly shading.
const DEPTH_LIGHT_RAMP_ROWS: i32 = 64;

/// Brightness of solid terrain at and below the bottom of the depth ramp.
/// Never black — the strata banding is the one landscape feature the review
/// called a genuine win, and it must stay legible at full depth. Composes
/// with `sky::apply_light`'s `UNLIT_FLOOR` (0.42) rather than replacing it,
/// which is also what softens the review's night-silhouette inversion: deep
/// rock at night now sits at ~0.26 of palette instead of 0.42.
const DEPTH_LIGHT_FLOOR: f32 = 0.62;

/// Brightness lift for the single row sitting exactly on the frozen skyline
/// — the "top surface catches the light" cue. One row only: the review asked
/// for a skyline highlight, not a glowing crust.
const DEPTH_LIGHT_HIGHLIGHT: f32 = 1.12;

/// Half-width of the window that decides which skyline features the depth
/// light believes in.
///
/// Measured against the raw per-column skyline, the depth light drew a
/// bright vertical shaft under every narrow notch: a slot seven columns
/// wide and thirty deep dropped its columns' skyline by thirty rows, so
/// the rock beside its floor read as "near the surface" and lit up in a
/// stripe against its neighbours — conspicuous on canyon terrain, whose
/// terrace snap cuts exactly such slots (the merged data track's 1b
/// finding). Light does not pour down a slot like that; it comes from the
/// whole sky.
///
/// So the light's datum is the skyline with narrow dips clipped to their
/// shoulders — a morphological opening: a dip narrower than
/// `2 * DEPTH_LIGHT_SHOULDER_REACH + 1` columns is treated as still being
/// at its shoulders' level, while anything wider (a real valley, a canyon
/// floor, a cliff step) passes through *exactly* unchanged, which is the
/// property that makes an opening the right tool rather than a blur: a
/// blur would soften every cliff base in the world to fix a dozen slots.
/// Nine columns of half-width clears the measured slot census (worst
/// notch 7 columns) with margin, and stays well under real valley widths.
const DEPTH_LIGHT_SHOULDER_REACH: usize = 9;

const CHUNK_BORDER_ACTIVE: [u8; 4] = [80, 200, 120, 255];
const CHUNK_BORDER_SETTLED: [u8; 4] = [60, 60, 70, 255];

// --- M19 visual polish: cheap, CPU-side, no shader pipeline -----------------
//
// Tier 1/2 of the M19 execution plan (see `PLAN.md` and
// `research/m19-visual-polish.md`) — grain, heat glow and fake occlusion,
// the techniques the research found gave the most visible improvement for
// the least engine change. A GPU bloom/lighting pass (Tier 4) stays with
// M6's deferral, since it still needs a human watching it render; these
// don't — they're as self-verifiable by screenshot as M7/M15 were.

/// How much a cell's brightness varies from its neighbours', at most.
/// Sandspiel's whole "looks organic despite simple materials" reputation
/// traces to exactly this trick: reusing a per-cell value as a brightness
/// jitter instead of only as a palette index. Keyed on world position via
/// `rng::jitter` (already used for movement decisions that must be stable
/// across frames) rather than redrawn per frame, so a settled pile doesn't
/// visibly sparkle — the grain is a property of the position, like the
/// palette shade already is.
const JITTER_STRENGTH: f32 = 0.12;

/// Where a `Liquid` cell's brightness grain comes from.
///
/// **Kept as a live selector on purpose, not a prototype to be collapsed.**
/// The owner asked for it to stay so the look can keep being iterated on;
/// `AnimatedMuted` is the current preference, and `Position` remains the
/// default so nothing changes for anyone who does not press `G`.
///
/// Reported from live play: water on a platform reads as *clearly static* in
/// the middle while the edges move, because the grain above is keyed on world
/// **position**. That is deliberately right for a settled sand pile (it is
/// what stops it sparkling) and wrong for water, which flows *through* a
/// pattern nailed to the screen — so a moving interior looks as frozen as a
/// still one, and the only visible motion is the silhouette changing at the
/// edges.
///
/// The variants below exist to be compared side by side in motion before one
/// is chosen; see `examples/filmstrip.rs`'s `grain=` argument.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GrainMode {
    /// Today's behaviour: `jitter(x, y)`, fixed in screen space.
    #[default]
    Position,
    /// Keyed on the cell's own `shade` byte, which `move_cell` carries along
    /// with it — so the texture travels with the water and motion reads
    /// everywhere, not just at the silhouette. Costs nothing per frame and
    /// keeps the dirty-rect render skip, because it is still a pure function
    /// of cell data.
    Cell,
    /// Position-keyed as today but at a third the amplitude, so a static
    /// pattern is simply less conspicuous. The cheapest option and the least
    /// ambitious: it mutes the symptom rather than addressing it.
    Muted,
    /// Position and a coarse time bucket, the same shape `FLAME_FLICKER_
    /// PERIOD` already uses for fire. Genuinely animated, and the only
    /// variant that **forces a full redraw every frame** — see `draw`.
    Animated,
    /// Position-keyed grain, plus a brightness lift for cells whose
    /// `FLAG_FLOWING` is set, so moving water is distinguishable from still
    /// water without animating anything.
    Motion,
    /// `Animated` at `Muted`'s amplitude. Live feedback on `Animated` was
    /// that it "fixed the issue but is way too much" — the motion is the
    /// right idea, the strength is not.
    AnimatedMuted,
    /// `AnimatedMuted`, plus the two things that make an animation read as
    /// drift rather than as flicker: it interpolates *between* consecutive
    /// time buckets instead of stepping from one to the next, and it steps
    /// more slowly. The other reading of "blur the animation".
    AnimatedSmooth,
}

impl GrainMode {
    fn next(self) -> Self {
        match self {
            GrainMode::Position => GrainMode::Cell,
            GrainMode::Cell => GrainMode::Muted,
            GrainMode::Muted => GrainMode::Animated,
            GrainMode::Animated => GrainMode::Motion,
            GrainMode::Motion => GrainMode::AnimatedMuted,
            GrainMode::AnimatedMuted => GrainMode::AnimatedSmooth,
            GrainMode::AnimatedSmooth => GrainMode::Position,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GrainMode::Position => "POSITION (current)",
            GrainMode::Cell => "CELL",
            GrainMode::Muted => "MUTED",
            GrainMode::Animated => "ANIMATED",
            GrainMode::Motion => "MOTION",
            GrainMode::AnimatedMuted => "ANIMATED-MUTED",
            GrainMode::AnimatedSmooth => "ANIMATED-SMOOTH",
        }
    }
}

/// Whether solid terrain is lit by its depth below the frozen skyline.
///
/// A live selector for the same reason `GrainMode` is one: "does this look
/// right" is only answerable in the running app, and the depth grade changes
/// every screenshot of every world, so the owner judges it as an A/B against
/// the exact strips the world review praised (the night strip especially —
/// the ramp softens the terrain-brighter-than-sky inversion, and whether
/// that improves or diminishes the most-praised frame in the review is an
/// eyes-only question). `Depth` is the default because the review's graphics
/// lens ranked the missing vertical light axis as the single largest
/// whole-picture defect; `Off` is one `F10` away and is the pre-review look.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerrainLight {
    /// Solid cells dim from full surface brightness at the skyline to
    /// `DEPTH_LIGHT_FLOOR` over `DEPTH_LIGHT_RAMP_ROWS`, with a one-row
    /// highlight on the skyline itself. A pure function of
    /// `(x, y, horizon[x])` — keeps the dirty-rect skip.
    #[default]
    Depth,
    /// The pre-review look: depth changes nothing about a solid cell's
    /// light.
    Off,
}

impl TerrainLight {
    fn next(self) -> Self {
        match self {
            TerrainLight::Depth => TerrainLight::Off,
            TerrainLight::Off => TerrainLight::Depth,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TerrainLight::Depth => "DEPTH (current)",
            TerrainLight::Off => "FLAT",
        }
    }
}

/// Frames per step of `GrainMode::Animated`, chosen by the same reasoning
/// `FLAME_FLICKER_PERIOD` records: re-rolling every frame reads as noise.
const GRAIN_ANIMATION_PERIOD: u64 = 5;

/// `GrainMode::AnimatedSmooth`'s own period. Slower than the stepped
/// variant's on purpose: interpolation removes the hard change between
/// buckets, which is what let the faster rate read as flicker, so the
/// smooth version can afford to drift over a longer interval.
const GRAIN_SMOOTH_PERIOD: u64 = 20;

/// How much brighter a `FLAG_FLOWING` liquid cell draws under
/// `GrainMode::Motion`.
const MOTION_GRAIN_LIFT: f32 = 0.10;

/// Moving water reads as foam: a `FLAG_FLOWING` liquid cell blends this
/// far toward `FOAM_TINT`, in every grain mode.
///
/// The one-column spring prototype made the case by failing it: a real
/// waterfall rendered as a barely-visible navy thread, because water's
/// palette is dark and nothing distinguished falling water from a still
/// pool's edge (owner: "very little water" — looking at 19,000 cells of
/// through-flow). White water is *how humans read moving water* at any
/// distance; the pool stays deep because its interior genuinely does not
/// move (`FLAG_FLOWING` clears on settle), so a fall is a bright ribbon
/// into a dark basin, which is what a waterfall looks like.
///
/// A pure function of the cell's own flag — no neighbour reads, so no
/// stale pixels at touched-chunk borders, and the dirty-rect skip's
/// pixel-identity argument holds: the flag's own transitions (move, or
/// the settle-clear write) both dirty the chunk that owns the cell.
const FOAM_BLEND: f32 = 0.45;

/// The colour moving water blends toward: pale, slightly blue-cold —
/// aerated water, not paint-white, so a fall still reads as water against
/// snow or sky.
const FOAM_TINT: [f32; 3] = [205.0, 226.0, 242.0];


/// How far above ambient a cell needs to be for `HEAT_GLOW_RANGE` to
/// saturate the warm-tint blend fully. Oil burns at 900C, so this is a
/// fraction of that — hot enough to mean something, not so high that only
/// active fire ever visibly registers.
const HEAT_GLOW_RANGE: f32 = 400.0;
/// The fire tint's own colour now shifts with `heat_ratio` — a bare ember
/// (`FIRE_TINT_LOW`) at the low end, up toward a bright yellow-white blaze
/// (`FIRE_TINT_HIGH`) at the top of `HEAT_GLOW_RANGE` — rather than one flat
/// orange regardless of intensity. A cooling coal and an actively raging
/// fire should not read as the identical colour just because both cross
/// `is_burning()`'s own threshold.
const FIRE_TINT_LOW: [f32; 3] = [180.0, 55.0, 20.0];
const FIRE_TINT_HIGH: [f32; 3] = [255.0, 210.0, 110.0];
/// Frames per flicker step for an actively burning cell — a real flame's
/// visible flicker rate is on the order of 10-15Hz, not a 60fps repaint, so
/// re-rolling every single frame would read as noise, not fire. `jitter3`
/// (position plus this coarse time bucket) holds the flicker steady within
/// one step and changes it deterministically at the next, with no per-cell
/// state to track.
const FLAME_FLICKER_PERIOD: u64 = 4;
/// How much the flicker can push the fire blend strength up or down, as a
/// fraction of it. `0.3` means anywhere from 70% to 130% of the blend
/// `heat_ratio` alone would produce — enough to visibly flicker, not enough
/// to make a hot cell ever read as merely warm.
const FLAME_FLICKER_STRENGTH: f32 = 0.3;

/// How hot a `Liquid` cell has to be before `BubbleMode` draws vapour in it.
///
/// **Well below boiling, and that is the whole design, arrived at by
/// getting it wrong first.** This was 80°C — near-boiling — on the obvious
/// reading that a bubble is drawn where the water is boiling. Measured on
/// `scene=simmer` (a pan over a hot hearth), a heated pool holds *72 to
/// 210* cells over 80°C and every one of them is the single row lying on
/// the hearth, under a bright fire tint. Diffed against the same sheet with
/// the effect off: **every changed pixel was in world rows 307-308**, the
/// bottom two of a fourteen-deep pan. There is no column of boiling water
/// for anything to rise through, and there never will be — a cell that hot
/// converts to steam within a few visits.
///
/// A rising bubble is not *in* boiling water; it is in **warm** water above
/// a boiling floor. `CLAUDE.md`'s "which cell does this rule evaluate?", in
/// its rendering costume. The same pan holds 995-1,117 cells over 40°C and
/// 354-676 over 60, spread up the whole column, which is a plume.
///
/// 40 rather than lower because ambient is 20 and a puddle beside a
/// campfire should not fizz. `examples/filmstrip.rs` prints the standing
/// count of liquid cells over this next to the image, because "the pool is
/// not bubbling" and "there is nothing warm enough in the pool to bubble"
/// are the same picture.
pub const BUBBLE_MIN_TEMPERATURE: f32 = 40.0;

/// The temperature at which the effect is at full strength. Water's own
/// boiling point — above it a cell is converting to steam anyway, so there
/// is nothing to gain from drawing more vapour inside it.
const BUBBLE_FULL_TEMPERATURE: f32 = 100.0;

/// Frames a bubble pattern takes to climb one cell. Small: bubbles rise
/// visibly faster than anything else in this engine moves, and the whole
/// point of keying on the frame is that they read as *rising* rather than
/// as a hot-water texture.
const BUBBLE_RISE_PERIOD: u64 = 2;

/// Fraction of candidate sites holding a bubble at full strength, before
/// the per-cell temperature ramp scales it down. Sparse: a pool where
/// every other pixel is a bubble is foam, not boiling water. Set so a
/// uniformly near-boiling pool lights ~15% of its pixels, which
/// `every_bubble_mode_actually_draws_bubbles_in_boiling_water` bounds.
const BUBBLE_DENSITY: f32 = 0.16;

/// How far a bubble's crown pixel blends toward `BUBBLE_TINT`, and how much
/// less its lower half gets. The split is what makes a two-pixel bubble
/// read as *domed* rather than as a square — a bright top over a dimmer
/// bottom is the cheapest possible specular highlight.
///
/// # A near-replace, because a blend of steam's colour is not steam's colour
///
/// Reported from play: the bubbles *"are not the color of steam"*. Checked,
/// and `BUBBLE_TINT` already **is** steam's colour — `[225, 238, 248]`
/// against `steam.ron`'s own `(222, 228, 236)`. What was wrong was the
/// blend: at the old 0.72 crown and 0.30 underside, most of a bubble pixel
/// was still the blue water behind it, and a mark that is 70% pond reads as
/// tinted water however well-chosen the other 30% is.
///
/// This is the same mistake the field overlays made and
/// `FieldOverlay`'s notes already record — *a magnitude-scaled blend read as
/// blank; make it a full replace on a fixed ramp*. Bubbles are the third
/// place in this file to learn it.
///
/// Not a *full* replace: a hard 1.0 everywhere loses the dome, and the dome
/// is what stops a 2x2 site being a white square. The crown replaces and
/// the underside keeps enough water to shade against it.
const BUBBLE_LIFT: f32 = 1.0;

/// The side of an ordinary bubble's site, in cells.
///
/// **Two, and a three was tried and rejected in play.** The reasoning for
/// the three was sound and the result was not: a 2x2 site cannot be
/// anything but a square, so the site was widened to 3x3 with its four
/// corners left dark — the smallest mark on a square grid that is *not* a
/// square. It draws a five-pixel plus, and a plus at this size reads worse
/// than the square did: *"You went from squares to crosses. That is
/// worse."* Do not re-derive the 3; at three cells across there is no
/// arrangement of lit pixels that reads as round.
///
/// The playtest that rejected it named the way out — *"unless you made much
/// bigger bubbles"* — which is `BubbleMode::Large`, where a mask has enough
/// pixels to curve.
const BUBBLE_SITE: i32 = 2;

/// The side of a `BubbleMode::Large` site, in cells.
///
/// Six, because that is where a disc mask starts to have somewhere to put
/// the curve: at four, trimming the corners leaves an octagon one pixel
/// deep and the shape is back to reading as a chamfered square.
const BUBBLE_LARGE_SITE: i32 = 6;

/// The two disc sizes a `BubbleMode::Large` site draws, as (squared radius
/// from the site's centre, cells lit).
///
/// **Two sizes, because one is a pattern.** Asked for from play: *"Large
/// should include a mix of this plus 1 pixel smaller bubbles."* A grid of
/// identical marks reads as a texture however round each mark is; a mix
/// reads as bubbles.
///
/// Eight is the big one's radius, rather than the nine a circle inscribed
/// in the site would use: nine trims only the four corners (32 of 36 lit)
/// and still reads square, eight also drops the eight cells flanking them
/// and leaves an octagon. 3.5 is the small one -- one cell narrower each
/// way with its own corners off, twelve lit. **4.5 was tried first and is
/// wrong**: at that radius the mask keeps exactly the inner four-by-four
/// block, so the small bubble draws a filled *square* -- the shape this
/// whole mode exists because play rejected.
///
/// The lit counts are hardcoded rather than counted, because they divide
/// the density on the hottest per-pixel path in this file -- and
/// `the_large_bubble_mask_lights_what_the_density_correction_assumes`
/// guards both against the mask, so neither can drift.
const BUBBLE_LARGE_DISCS: [(f32, i32); 2] = [(8.0, 24), (3.5, 12)];

/// Which of `BUBBLE_LARGE_DISCS` a site draws -- a second hash,
/// independent of the one that decides whether the site lights at all, or
/// size and presence would be correlated and the small ones would all
/// arrive together.
fn large_bubble_disc(sx: i32, sy: i32) -> (f32, i32) {
    BUBBLE_LARGE_DISCS[usize::from(rng::jitter3(sx, sy, 0x5B) < 0.5)]
}

/// How much rarer a `BubbleMode::Large` bubble is than the density
/// correction alone would make it.
///
/// *"Unless you made much bigger bubbles"* is a request for **fewer and
/// bigger**, not for the same amount of boil in wider marks. Normalised
/// per pixel and nothing else, six-cell discs at `BUBBLE_DENSITY` put a lit
/// site next to a lit site and the pan reads as a fog bank rather than as
/// bubbles — measured by eye on `scene=simmer`, which is only two site rows
/// deep at this size, so neighbours merge into horizontal smears. Halving
/// the population is what separates them again.
const BUBBLE_LARGE_RARITY: f32 = 0.5;


/// `BUBBLE_LIFT` for the shaded half of a bubble. See that constant for why
/// both numbers moved up together: at 0.30 the underside was 70% pond water
/// and the bubble read as a tinted patch of the liquid rather than as gas
/// sitting in it.
const BUBBLE_UNDERSIDE: f32 = 0.72;

/// Whether `(ix, iy)` within a `BubbleMode::Large` site of squared radius
/// `radius_sq` falls inside its disc. See `BUBBLE_LARGE_DISCS`.
fn large_bubble_covers(ix: i32, iy: i32, radius_sq: f32) -> bool {
    let centre = (BUBBLE_LARGE_SITE - 1) as f32 / 2.0;
    let (dx, dy) = (ix as f32 - centre, iy as f32 - centre);
    dx * dx + dy * dy <= radius_sq
}

/// The share of a bubble's brightness that does *not* scale with how hot
/// its cell is. **Density carries the temperature signal; brightness only
/// helps a little.** Scaling both was tried and it is the wrong shape: a
/// bubble halfway up a warm column then draws at half strength and reads as
/// grain, so exactly the population the threshold was lowered to reach
/// arrives invisible. A bubble either is there or is not.
///
/// Raised with `BUBBLE_LIFT` for the same reason: at 0.5 a bubble in
/// half-boiling water drew at 75% of an already-diluted blend, which is the
/// invisibility this constant exists to prevent, arriving by the other door.
const BUBBLE_FLOOR: f32 = 0.85;

/// How far below a surface cell `BubbleMode::Surface` looks for the heat
/// that made the bubble.
///
/// Deep enough to see the bottom of an ordinary pan, shallow enough that a
/// lake's surface is not reporting on water twenty rows down that has
/// nothing to do with it.
const SURFACE_BUBBLE_DEPTH: i32 = 8;

/// The hottest liquid of the same kind within `SURFACE_BUBBLE_DEPTH` below
/// `(x, y)`, as a boil fraction.
///
/// # Why `Surface` needed this to mean anything at all
///
/// The mode gated on a cell being both **near the top of its body** and
/// **over `BUBBLE_MIN_TEMPERATURE`**, and heat arrives from underneath —
/// so on any bottom-heated pool those two sets are in different places and
/// the mode diffed to **zero pixels** against `Off`. The owner saw exactly
/// that and said so: *"All images look mostly identical. But also if we are
/// testing bubbles in water the boiling needs to happen at the bottom of
/// the pond/lake instead of the top."* Both halves of that were right, and
/// the second explains the first.
///
/// A bubble is not made where it pops. Asking about the water *below* is
/// what makes the mode's own description true — vapour drawn where the
/// liquid has something to escape into, with the boil that produced it
/// setting how much.
///
/// **This costs the surface row and nothing else.** The near-top test runs
/// first and bails everything under it, so the scan is paid by O(width)
/// cells rather than by the pool.
fn boil_below(world: &World, x: i32, y: i32, cell: Cell, heat_to_boil: impl Fn(i16) -> f32) -> f32 {
    let mut best = 0.0f32;
    for dy in 1..=SURFACE_BUBBLE_DEPTH {
        let below = world.get(x, y + dy);
        if below.material != cell.material {
            break; // out of this body: past the floor, or into something else
        }
        best = best.max(heat_to_boil(below.temperature()));
    }
    best
}

/// What a bubble blends toward: near-white with a cool cast, so vapour
/// inside water reads as the same substance as the steam above it rather
/// than as a bright blue liquid pixel.
const BUBBLE_TINT: [f32; 3] = [225.0, 238.0, 248.0];

/// How boiling liquid is drawn, cycled by `H`.
///
/// **A selector rather than a decision, on `GrainMode`'s precedent**: this
/// is a "does it look right" question, five grain modes behind one key
/// settled the last one of those in minutes, and no amount of argument or
/// still images had. `Off` is the default so nothing changes for anyone who
/// does not press the key, and the active mode is named on screen. **The
/// selector has since been run**: `Rising` won and is the default; `Off`
/// stays on the cycle, because a look decision with no way back to the
/// thing it replaced cannot be re-argued.
///
/// **None of these costs the dirty-rect render skip**, which is the usual
/// price of an animated effect (`GrainMode::Animated` pays ~10 ms/frame on
/// a settled world for exactly that reason). They ride inside the heat-glow
/// branch, which already admits only cells that are burning or off ambient
/// — and a cell that is off ambient is one `fire::update` is still writing
/// every frame to keep its own chunk dirty, so its chunk is in `touched`
/// already. A settled world has no such cells and pays nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BubbleMode {
    /// The behaviour up to this build: hot water is tinted by the heat glow
    /// and nothing else, and boiling reads as the steam above the surface.
    Off,
    /// Sparse domed highlights scrolling upward, on a `BUBBLE_SITE` grid.
    /// The bubble is the unit, not the pixel — a one-pixel speckle at this
    /// density reads as grain, which is the failure mode this variant is
    /// shaped against.
    ///
    /// **The default**, chosen from a card: *"Off (current) and rising look
    /// best."* The same card asked for them to be less blocky, and the
    /// attempt to round *this* mode was rejected on sight — see
    /// `BUBBLE_SITE`. `Large` is where that ask lives now.
    #[default]
    Rising,
    /// `Rising` at `BUBBLE_LARGE_SITE`, masked to a disc: fewer bubbles,
    /// each big enough to have a shape.
    ///
    /// From the same card, after the rounded 3x3 was rejected: *"I guess
    /// this is just a limit of our resolution unless you made much bigger
    /// bubbles."* This is that, and it is a separate mode rather than a
    /// retune of `Rising` because the two are different looks and only play
    /// can say which pool wants which.
    Large,
    /// `Rising`, but the pattern is stretched vertically so a bubble is a
    /// short column rather than a blob, and it climbs faster. The other
    /// reading of "bubbles": a stream leaving a nucleation site, rather
    /// than a scatter of separate pockets.
    Columns,
    /// Bubbles only where the liquid has something to escape *into* — the
    /// pattern is the same as `Rising`, gated on the cell being within a
    /// couple of rows of the top of its own body. Boiling that stays at the
    /// surface, for the reading that a bubble deep in a pool should not be
    /// visible through the water above it.
    ///
    /// The heat that decides how much comes from the water **below** the
    /// cell, not from the cell itself — see `boil_below`, and note that
    /// without it this mode drew nothing at all on any bottom-heated pool.
    Surface,
}

impl BubbleMode {
    fn next(self) -> Self {
        match self {
            BubbleMode::Off => BubbleMode::Rising,
            BubbleMode::Rising => BubbleMode::Large,
            BubbleMode::Large => BubbleMode::Columns,
            BubbleMode::Columns => BubbleMode::Surface,
            BubbleMode::Surface => BubbleMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BubbleMode::Off => "OFF (the old look)",
            BubbleMode::Rising => "RISING (current)",
            BubbleMode::Large => "LARGE",
            BubbleMode::Columns => "COLUMNS",
            BubbleMode::Surface => "SURFACE",
        }
    }
}

/// How a `Gas` cell is drawn against what is behind it, cycled by `;`.
///
/// Gas has always drawn opaque, which is the one thing a cloud is not, and
/// it costs more than it looks: `scene=boil`'s steam fills its chamber by
/// frame 300 and from then on the sheet shows a speckled wall with the pool
/// somewhere behind it. That is not only a look complaint -- it is why the
/// bubble work above had to build `scene=simmer` to be judged at all.
///
/// A selector rather than a decision, on `GrainMode`'s precedent -- and
/// **the selector has been run and `ByFill` won**, so it is the default
/// now. The owner, judging a blind card of the three: *"A and C are almost
/// indistinguishable (at least in static images), but look better than
/// option B"* -- A and C being `Translucent` and `ByFill`, B `Opaque`. Of
/// the two that tied by eye, `ByFill` is the one that is also a readout:
/// its alpha follows the cell's own fill, so a thinning plume draws
/// thinner and a full one does not.
///
/// `Opaque` is kept on the cycle rather than deleted. It is what every
/// screenshot before this build shows, and a look decision with no way back
/// to the thing it replaced cannot be re-argued.
///
/// **It does not cost the dirty-rect render skip.** The blend reads
/// `background_at`, which is a `Vec` index and an array index and no
/// `World::get` at all, and it runs only for `Gas` cells -- a population
/// that is empty on a settled world and small even during a blast.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GasMode {
    /// The behaviour up to this build: a gas cell's own colour, lit, and
    /// nothing else.
    Opaque,
    /// Blended toward what is behind it by `GAS_ALPHA`, so a cloud thins
    /// what it covers rather than replacing it. Applied *after* the light,
    /// deliberately: the background is already lit, and blending before
    /// would light it twice.
    Translucent,
    /// `Translucent`, but the alpha follows the cell's own `aux` fill --
    /// which for steam is the water it carries (`fire::transform`'s table),
    /// so a plume that has thinned out draws thinner. The other reading of
    /// "translucent", and the one that is a *readout* as well as a look.
    ///
    /// The default, chosen by the owner from a blind card -- see the enum's
    /// own doc.
    #[default]
    ByFill,
}

impl GasMode {
    fn next(self) -> Self {
        match self {
            GasMode::Opaque => GasMode::Translucent,
            GasMode::Translucent => GasMode::ByFill,
            GasMode::ByFill => GasMode::Opaque,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GasMode::Opaque => "OPAQUE (the old look)",
            GasMode::Translucent => "TRANSLUCENT",
            GasMode::ByFill => "BY-FILL (current)",
        }
    }
}

/// How much of its own colour a `GasMode::Translucent` cell keeps. Well
/// above half: a cloud you can see straight through is fog, not steam, and
/// the point is to thin what is behind rather than to reveal it.
const GAS_ALPHA: f32 = 0.62;

/// The floor `GasMode::ByFill` will not thin past, so a nearly-drained
/// steam cell is still visibly *there*. Without it the thin edge of every
/// plume disappears entirely and the cloud reads as having a hard rim,
/// which is the opposite of the point.
const GAS_ALPHA_MIN: f32 = 0.25;

/// Which M13 field channel `Renderer::field_overlay` tints the screen by,
/// cycled by `V` — parallel to `F1`'s existing chunk overlay, for seeing
/// the coarse field grid instead of chunk activity.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FieldOverlay {
    #[default]
    Off,
    Pressure,
    Temperature,
    Light,
    Moisture,
    /// The two stigmergy planes (`sim::pheromone`). Cycled here with the
    /// field channels because `V` is where a player already looks for "show
    /// me an invisible scalar", but note these are **not** field channels —
    /// they are at CA resolution, and they are drawn by a different rule
    /// (see `apply_field_overlay`).
    PheromoneA,
    PheromoneB,
}

impl FieldOverlay {
    fn next(self) -> Self {
        match self {
            FieldOverlay::Off => FieldOverlay::Pressure,
            FieldOverlay::Pressure => FieldOverlay::Temperature,
            FieldOverlay::Temperature => FieldOverlay::Light,
            FieldOverlay::Light => FieldOverlay::Moisture,
            FieldOverlay::Moisture => FieldOverlay::PheromoneA,
            FieldOverlay::PheromoneA => FieldOverlay::PheromoneB,
            FieldOverlay::PheromoneB => FieldOverlay::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FieldOverlay::Off => "OFF",
            FieldOverlay::Pressure => "PRESSURE",
            FieldOverlay::Temperature => "TEMPERATURE",
            FieldOverlay::Light => "LIGHT",
            FieldOverlay::Moisture => "MOISTURE",
            FieldOverlay::PheromoneA => "PHEROMONE A",
            FieldOverlay::PheromoneB => "PHEROMONE B",
        }
    }
}

/// Which organism-owned per-cell channel `Renderer::organism_overlay` tints
/// by, cycled by `L` — the same shape as `FieldOverlay` above, pointed at
/// `organism.rs`'s per-cell scalars instead of `field.rs`'s coarse grid.
///
/// **Why this exists at all.** Every channel here is currently invisible,
/// and that has already cost real time: `Reports/tree-rewrite-design.md`
/// §2b's canopy-density self-avoidance mechanism shipped inert, and *two*
/// independent design reviews signed it off without catching that the
/// "follow" step read the empty side of the occupied/empty boundary and
/// therefore always saw `0.0`. Live verification caught it, because a
/// picture shows a round clump and a passing test does not. The channels
/// that decide plant shape need to be lookable-at before more of them get
/// added on top (`Reports/plant-substrate-v2-design.md`'s Decisions 2, 3
/// and 6 each add one), not after.
///
/// `CellType` is categorical and the rest are magnitudes — see
/// `apply_organism_overlay` for why that difference is in the blend rather
/// than in two separate overlays.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OrganismOverlay {
    #[default]
    Off,
    /// `Seed`/`GrowingTip`/`MatureBody`/`Leaf`/`RootTip`, one flat colour
    /// each. The only way to tell a retired `MatureBody` from a live
    /// `GrowingTip` on screen — both currently paint as plain `wood`.
    CellType,
    /// The `Grow`/`Divide` energy budget, `0..RESOURCE_SCALE`.
    Resource,
    /// The crowding signal `Grow`'s `crowding_weight` scores against,
    /// `0..CANOPY_DENSITY_SCALE`.
    CanopyDensity,
    /// **Vein conductance** — the largest of a cell's four per-face carbon
    /// efflux conductances, `CONDUCTANCE_MIN..CONDUCTANCE_MAX`.
    ///
    /// The channel Decision 6 exists to make visible. Canalization either
    /// produces a strand hierarchy — a bright path from source to sink
    /// against dim undifferentiated tissue — or it does not, and no unit
    /// test answers that question the way a picture does. Renders the max
    /// rather than a sum or a mean because the question is "is this cell
    /// part of a channel", and a cell with one strongly conducting face is,
    /// however isotropic its other three are.
    VeinConductance,
    /// Water held in a `Powder` cell — not organism data, but the same
    /// question ("what is in this cell that I cannot see") and the channel
    /// the root work has to be able to look at. Without it, a wetting
    /// front descending through soil is completely invisible.
    SoilMoisture,
}

impl OrganismOverlay {
    fn next(self) -> Self {
        match self {
            OrganismOverlay::Off => OrganismOverlay::CellType,
            OrganismOverlay::CellType => OrganismOverlay::Resource,
            OrganismOverlay::Resource => OrganismOverlay::CanopyDensity,
            OrganismOverlay::CanopyDensity => OrganismOverlay::VeinConductance,
            OrganismOverlay::VeinConductance => OrganismOverlay::SoilMoisture,
            OrganismOverlay::SoilMoisture => OrganismOverlay::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OrganismOverlay::Off => "OFF",
            OrganismOverlay::CellType => "CELL TYPE",
            OrganismOverlay::Resource => "RESOURCE",
            OrganismOverlay::CanopyDensity => "CANOPY DENSITY",
            OrganismOverlay::VeinConductance => "VEIN CONDUCTANCE",
            OrganismOverlay::SoilMoisture => "SOIL MOISTURE",
        }
    }
}

/// Per-`CellType` colours for `OrganismOverlay::CellType`. Deliberately
/// high-contrast and *not* botanically suggestive — this is a debug
/// readout, and a leaf drawn green here would be indistinguishable from
/// the `leaf` material the plant work is about to add
/// (`Reports/plant-substrate-v2-design.md` §6a), which is exactly the
/// confusion a debug view must not create.
const CELL_TYPE_SEED: [f32; 3] = [255.0, 230.0, 90.0];
const CELL_TYPE_GROWING_TIP: [f32; 3] = [90.0, 255.0, 120.0];
const CELL_TYPE_MATURE_BODY: [f32; 3] = [150.0, 110.0, 255.0];
const CELL_TYPE_LEAF: [f32; 3] = [255.0, 120.0, 220.0];
const CELL_TYPE_ROOT_TIP: [f32; 3] = [255.0, 150.0, 60.0];
/// Deliberately loud and unlike wood. A bud is a *decision point* — where
/// the crown will come from — and it is a handful of cells among thousands
/// of stem cells, so a subtle colour reads as absent. See this module's own
/// note on the canopy-density sheet that read as blank because a mid-range
/// value moved one colour byte from 139 to 155.
const CELL_TYPE_DORMANT_BUD: [f32; 3] = [80.0, 255.0, 255.0];
/// A creature's deciding cell and its trailing body. Loud, and loudly
/// *different from each other*: the question this overlay has to answer for
/// a chain is "where is the head", and a head-and-tail drawn in two shades
/// of one hue is unreadable at the one-or-two-cell size a creature actually
/// occupies. Pure white against a mid grey is the widest separation
/// available that is also unlike every plant colour above.
const CELL_TYPE_HEAD: [f32; 3] = [255.0, 255.0, 255.0];
const CELL_TYPE_SEGMENT: [f32; 3] = [130.0, 130.0, 140.0];

/// Flat blend for `OrganismOverlay::CellType`. High, but short of 1.0 on
/// purpose: keeping a little of the underlying material colour through
/// means a burning or partially-dimmed cell still reads as such under the
/// overlay, so a fire moving through a canopy stays legible while the type
/// readout is on.
const CELL_TYPE_BLEND: f32 = 0.85;
/// Full-scale colours for the scalar organism channels. A reading of zero
/// draws at `SCALAR_RAMP_FLOOR` of these, not at black, so a cell holding
/// exactly nothing is still visible *as a cell* — otherwise an organism
/// with an empty channel disappears into the background and "the value is
/// zero" becomes indistinguishable from "there is nothing here", which are
/// very different bugs.
const SCALAR_RAMP_RESOURCE: [f32; 3] = [120.0, 255.0, 160.0];
const SCALAR_RAMP_CANOPY: [f32; 3] = [255.0, 80.0, 80.0];
const SCALAR_RAMP_MOISTURE: [f32; 3] = [80.0, 170.0, 255.0];
/// Deliberately unlike `SCALAR_RAMP_RESOURCE`: conductance and the carbon
/// it carries are different quantities on different timescales, and two
/// green ramps would invite reading one sheet as the other.
const SCALAR_RAMP_VEIN: [f32; 3] = [255.0, 210.0, 90.0];

/// How bright a zero reading draws, as a fraction of the channel's
/// full-scale colour. Low enough that zero and full are unmistakable at a
/// glance, high enough that a zero cell still has a visible silhouette.
/// The two pheromone planes. Magenta and cyan — chosen to be unlike every
/// material in the world *and* unlike each other at a glance, because the
/// question these overlays exist to answer for a colony is "which channel
/// is this trail", and two neighbouring hues cannot answer it on a
/// one-cell-wide line.
const SCALAR_RAMP_PHERO_A: [f32; 3] = [255.0, 80.0, 220.0];
const SCALAR_RAMP_PHERO_B: [f32; 3] = [80.0, 240.0, 255.0];
/// The two halves of the signed temperature ramp — warmer than ambient and
/// cooler than ambient. Two hues rather than one ramp through black, so the
/// *sign* is readable at a glance on a contact sheet: a world that has gone
/// blue overnight and one that has gone dark are different pictures.
const SCALAR_RAMP_WARM: [f32; 3] = [255.0, 140.0, 40.0];
const SCALAR_RAMP_COOL: [f32; 3] = [90.0, 150.0, 255.0];

const SCALAR_RAMP_FLOOR: f32 = 0.18;

/// Maps a normalized `0..1` reading onto a channel's ramp. Deliberately
/// linear: this is a readout, and a perceptual or log curve would make
/// "how much" harder to judge between two tiles of a contact sheet, which
/// is the comparison these sheets exist to support.
fn scalar_ramp(t: f32, full: [f32; 3]) -> [f32; 3] {
    let scale = SCALAR_RAMP_FLOOR + (1.0 - SCALAR_RAMP_FLOOR) * t.clamp(0.0, 1.0);
    [full[0] * scale, full[1] * scale, full[2] * scale]
}

/// Display-only normalization ranges for the field overlay — not the
/// field's own internal clamp bounds (`field.rs`'s `MAX_TEMPERATURE`/
/// `MAX_LIGHT`/`MAX_MOISTURE` are private to that module, the same
/// "documented assumption about a scale this module can't reach"
/// `creature.rs`'s own `WORM_MOISTURE_SATURATION` already accepts for the
/// identical reason). Chosen to make the *common* range of each channel
/// legible, not to bound it exactly — a value past the range end still
/// renders, just clamped to the most saturated colour rather than blowing
/// out past it.
const PRESSURE_OVERLAY_RANGE: f32 = 30.0;
const TEMPERATURE_OVERLAY_MAX: f32 = 900.0;
const LIGHT_OVERLAY_MAX: f32 = 4.0;
const MOISTURE_OVERLAY_MAX: f32 = 4.0;

/// Clamp bounds for `Renderer::zoom` — magnifying, screen pixels per world
/// cell. `8` is arbitrary (untuned against anything but "still recognizably
/// the sandbox, not a single giant cell filling the window" at the engine's
/// 512x320 simulation resolution).
const MAX_ZOOM: i32 = 8;
/// Clamp bounds for `Renderer::zoom_out_stride` — minifying, world cells
/// per screen pixel. `4` keeps a stride-sampled view still readable as
/// "the same kind of picture, zoomed out" rather than aliasing into noise.
const MAX_ZOOM_OUT_STRIDE: i32 = 4;

/// How fast the `WASD` map scroll travels, in **viewport-fuls per second**.
///
/// In screens, deliberately, never in cells: what has to feel the same at
/// every zoom is how fast the *picture* slides, and a screens-per-second rate
/// makes screen-pixels-per-second invariant by construction rather than by
/// three numbers being tuned to agree. `Renderer::pan` multiplies it by
/// `visible_span`, which already knows what a screenful is at the current
/// scale.
///
/// This is the **sustained** rate, reached after `PAN_RAMP_SECONDS` of holding
/// a key; the scroll opens at `PAN_START_FRACTION` of it. `0.5` is 256 cells/s,
/// about 2.8x the gnome's own run (`run_max` 1.5 cells/tick = 90 cells/s), and
/// puts the 1536-cell pannable width — the world less one viewport — about six
/// seconds end to end, or two and a half for the world's one-screen depth.
///
/// **`1.5` shipped here first and was rejected by playtest: "way too fast."**
/// Keep the number, because the way it was arrived at is the useful part. The
/// brief was *"I want to be able to scroll around when the gnome isn't
/// deployed"* — a complaint about **needing a gnome**, not about pace. It got
/// read as a speed requirement, justified against the seventeen seconds it
/// takes the gnome to run the same distance, and landed at 8.5x his run. No
/// one had asked for that. When a rate is chosen against a goal the brief does
/// not state, it is worth checking the brief actually states it.
///
/// A hold-to-accelerate ramp was also rejected at the time, on the grounds
/// that "the longest haul here is two seconds" — which was true *only because
/// the base rate was too high*, so that argument fell with the rate it was
/// derived from. The owner's verdict was that the scroll both overshot and was
/// too twitchy to nudge, which is precisely the pair a ramp answers. See
/// `PAN_START_FRACTION`.
const PAN_SCREENS_PER_SECOND: f32 = 0.5;

/// What fraction of the sustained rate the scroll opens at, before the ramp.
///
/// `0.4` is 102 cells/s at 1:1 — near enough the gnome's own running pace, and
/// a tenth-of-a-second tap moves the view about ten cells. That is the answer
/// to "too twitchy": a tap should nudge the view, not throw it.
///
/// The ramp is what keeps that from also making travel a chore. Opening at the
/// sustained rate is the twitchy version; opening slow *without* a ramp turns
/// the six-second traverse into fifteen. Neither complaint can be fixed
/// without the other, which is why one constant could not do it.
const PAN_START_FRACTION: f32 = 0.4;
/// How long a held key takes to reach the full rate, ramping linearly from
/// `PAN_START_FRACTION`. Short enough that travelling does not feel gated,
/// long enough that a deliberate nudge stays inside the slow part.
const PAN_RAMP_SECONDS: f32 = 0.8;

/// How far a fractured cell is darkened, out of 256. Dark enough to read as
/// a break at a glance, light enough that a scored face is still obviously
/// rock rather than a hole.
/// What fully saturated ground is scaled to, out of 256. Soaked soil is
/// distinctly darker than dry and still legibly soil -- taking it much
/// lower turns a rained-on hillside into a silhouette.
/// How far to either side a column looks when deciding whether it is a shaft
/// rather than open ground. A slot up to about twice this wide reads as
/// inside the mountain; a genuine excavation wider than that reads as open,
/// which is the right place for the line — the complaint was about a *narrow*
/// strip of sky, and a quarry with sky over it is not wrong.
/// How far a strike at full brightness pulls the frame toward white.
///
/// Set by eye against a render, downward: the first value (0.55) whited the
/// scene out so completely that the terrain stopped being readable, which is
/// what a camera does and not what an eye does. Enough to be unmistakable,
/// little enough that you can still see what you are standing on.
const FLASH_LIFT: f32 = 0.34;


const DAMP_DARKEN: u32 = 150;
const CRACK_DARKEN: u16 = 110;

/// The gnome's colored-cell sprite, `PLAYER_HEIGHT` rows of
/// `PLAYER_WIDTH`, top to bottom: a pointed hat over a bearded face, a
/// tunic with arms, a belt, and legs with a gap between the boots. `None`
/// cells are transparent — the world shows through, which is what keeps a
/// filled rectangle reading as a figure rather than a crate.
///
/// Grown from 3x6 to 5x10 on a playtest note ("can we make the gnome a
/// little bigger"). The extra rows are what buy the readable silhouette:
/// at 3x6 there was exactly one row for the face and none at all for
/// arms, so every feature had to be a full-width band.
const GNOME_HAT: [u8; 4] = [204, 62, 48, 255];
const GNOME_FACE: [u8; 4] = [232, 186, 148, 255];
const GNOME_BEARD: [u8; 4] = [226, 226, 226, 255];
const GNOME_TUNIC: [u8; 4] = [74, 138, 70, 255];
const GNOME_BELT: [u8; 4] = [82, 54, 34, 255];
const GNOME_BOOT: [u8; 4] = [108, 76, 46, 255];
/// Local aliases, so the tables below stay pictures you can read.
const H: Option<[u8; 4]> = Some(GNOME_HAT);
const F: Option<[u8; 4]> = Some(GNOME_FACE);
const W: Option<[u8; 4]> = Some(GNOME_BEARD);
const T: Option<[u8; 4]> = Some(GNOME_TUNIC);
const L: Option<[u8; 4]> = Some(GNOME_BELT);
const B: Option<[u8; 4]> = Some(GNOME_BOOT);
const X: Option<[u8; 4]> = None;

/// Drawn facing **right**; `draw_player` mirrors the columns when he is
/// facing left.
///
/// **Deliberately asymmetric, and that is the change** — the previous
/// table was a perfect mirror of itself, so flipping it would have been a
/// no-op and "he faces the way he is going" would have shipped as a field
/// nobody could see. The tells are a hand out in front (row 9) and the
/// weight forward on the leading boot (row 13): two pixels, which is all
/// there is room for at this size and enough to read at 1x.
const GNOME_SPRITE: [[Option<[u8; 4]>; 7]; 14] = [
    [X, X, X, H, X, X, X],
    [X, X, H, H, H, X, X],
    [X, H, H, H, H, H, X],
    [H, H, H, H, H, H, H],
    [X, X, F, F, F, X, X],
    [X, F, F, F, F, F, X],
    [X, F, W, W, W, F, X],
    [X, X, W, W, W, X, X],
    [T, T, T, T, T, T, T],
    [T, T, T, T, T, T, F],
    [X, T, T, L, T, T, X],
    [X, T, T, T, T, T, X],
    [X, B, B, X, B, B, X],
    [X, B, B, X, B, B, B],
];

/// The same figure mid-swing: the arm comes up and reaches out ahead of
/// him, and he leans into it.
///
/// A destructive act that looks identical to standing still is one the
/// player cannot tell fired (`design-philosophy.md` §0a — every event owes
/// feedback). One frame is enough at 60 Hz to read a dig or a shake as a
/// blow rather than as a cursor effect.
const GNOME_SWING: [[Option<[u8; 4]>; 7]; 14] = [
    [X, X, X, H, X, X, X],
    [X, X, H, H, H, X, X],
    [X, H, H, H, H, H, X],
    [H, H, H, H, H, H, H],
    [X, X, F, F, F, X, F],
    [X, F, F, F, F, F, F],
    [X, F, W, W, W, F, X],
    [X, X, W, W, W, X, X],
    [T, T, T, T, T, T, X],
    [T, T, T, T, T, T, X],
    [X, T, T, L, T, T, X],
    [X, T, T, T, T, T, X],
    [X, B, B, X, B, B, X],
    [B, B, B, X, B, B, X],
];

/// Whether the gnome draws over a tree, behind it, or a mix — the owner's
/// "sometimes walking in front of trees and sometimes behind".
///
/// A **selector rather than a decision**, per `CLAUDE.md`: this is a
/// does-this-look-right question, and five grain modes behind one key once
/// settled in minutes what no amount of argument or still images had.
///
/// Purely graphical. Nothing about collision, light or the simulation
/// changes with it — a tree is walk-through either way, and this only says
/// which of the two gets the pixel where they overlap.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum TreeDepth {
    /// Half the trees draw over him, chosen per tree and stable for its
    /// life. The default, because it is the effect that was asked for.
    #[default]
    Weave,
    /// The previous behaviour: he draws over everything.
    Front,
    /// Every tree draws over him — the far end, for judging how readable a
    /// half-hidden gnome actually is before settling on the middle.
    Behind,
    /// `Weave`, and the trees he passes *behind* are also dimmed, so the
    /// two layers read as layers rather than as random occlusion.
    Haze,
}

impl TreeDepth {
    pub fn next(self) -> Self {
        match self {
            TreeDepth::Weave => TreeDepth::Haze,
            TreeDepth::Haze => TreeDepth::Front,
            TreeDepth::Front => TreeDepth::Behind,
            TreeDepth::Behind => TreeDepth::Weave,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TreeDepth::Weave => "WEAVE (current)",
            TreeDepth::Haze => "WEAVE+HAZE",
            TreeDepth::Front => "FRONT",
            TreeDepth::Behind => "BEHIND",
        }
    }

    /// Whether the thing identified by `key` draws in front of him.
    ///
    /// **Two callers with two keys.** A tree keys on its `organism_id`, so a
    /// stand's sides are fixed for its life. A cave formation has no
    /// identity to key on — it is loose cells of `flowstone`, not an
    /// organism — so it keys on its **world column**, which gives the same
    /// two properties for free: one formation keeps one side however long
    /// it stands, and neighbouring formations decorrelate.
    ///
    /// **A hash, not `id & 1`.** Organism ids are handed out sequentially,
    /// so parity makes consecutively-planted trees alternate — a
    /// correlation the eye picks out at once in a row of them, and worldgen
    /// plants a stand left to right.
    ///
    /// A pure function of world state and nothing else, which is what keeps
    /// `dirty_rect_skip_is_pixel_identical_to_a_full_redraw` true: a fresh
    /// `Renderer` and one with history agree by construction. The reverted
    /// stateful skyline is the recorded case of getting this wrong.
    fn in_front(self, key: u32) -> bool {
        match self {
            TreeDepth::Front => false,
            TreeDepth::Behind => true,
            TreeDepth::Weave | TreeDepth::Haze => key.wrapping_mul(2_654_435_761) >> 16 & 1 == 1,
        }
    }
}

/// How much of him still shows through a tree he is passing behind.
///
/// **Zero: a tree in front hides him.** This was 0.28, on my argument that
/// a gnome who vanishes into a crown is a gnome you have lost and the
/// picture stops being playable however good it looks. Overruled by the
/// first playtest — "behind has too much transparency, the player
/// character should be hidden" — so the concern was mine and not real, at
/// least at this zoom. Kept as a named constant with the branch intact
/// rather than deleted, because the owner flagged they may revisit it:
/// restoring the ghost is this one number, and `blend` clamps alpha, so
/// any value between is valid.
const OCCLUDED_ALPHA: f32 = 0.0;

/// Where a glow's *shape* comes from: the coarse light field alone, or the
/// emitting cells as well.
///
/// A selector rather than a silent replacement, because the repo's rule for
/// "does this look right" is to ship the choice and name the active one on
/// screen. Unusually, the default is the **new** behaviour: this is not a
/// matter of taste that nobody has ruled on, it is
/// `Reports/open-bugs-handoff.md` 0c, which the owner named twice
/// unprompted on cards that were about something else. `Field` is kept so
/// the two can be put side by side, and costs one branch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GlowShape {
    /// Emitting cells splat a per-cell near field; the coarse field carries
    /// the far falloff. See `NEAR_GLOW_RADIUS`.
    #[default]
    Near,
    /// The pre-fix look: the coarse light field alone, quantised to
    /// `FIELD_SCALE`.
    Field,
}

impl GlowShape {
    fn next(self) -> Self {
        match self {
            GlowShape::Near => GlowShape::Field,
            GlowShape::Field => GlowShape::Near,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GlowShape::Near => "NEAR (current)",
            GlowShape::Field => "FIELD (8-cell blocks)",
        }
    }
}

/// Radius, in cells, of the near-field glow term that gives a light source
/// a *shape*.
///
/// The coarse light field holds one value per `FIELD_SCALE` (8) cells and
/// -- worse -- seeds the emitter itself at that resolution, so a two-cell
/// crystal is a filled 8x8 block before a single diffusion step runs, and
/// `field_at_bilinear` then smears it over about sixteen. The result the
/// owner named twice unprompted: *"the big sharp squares that look like
/// giant white gray pixels"* and *"the rectangular lighting looks bad"*.
/// Smoothing does not help; a vaguer sixteen-cell blob is still a
/// sixteen-cell blob.
///
/// Set to cover the zone where the quantisation is visible -- the steep
/// part of the falloff, roughly two field cells -- and no further. Beyond
/// it the coarse field's level is low and its gradient shallow, which is
/// where a block edge stops being legible and where the field is doing the
/// job it is good at.
const NEAR_GLOW_RADIUS: i32 = 14;

/// How far a background tree is dimmed under `TreeDepth::Haze`.
const HAZE_DIM: u16 = 168;

pub struct Renderer {
    /// Which `GrainMode` a `Liquid` cell's brightness grain comes from.
    /// Prototype switch — see the enum's own doc.
    pub grain: GrainMode,
    /// How boiling liquid is drawn — see `BubbleMode`. `Off` by default,
    /// cycled by `H`, and free on a settled world for the reason that enum
    /// documents.
    pub bubbles: BubbleMode,
    /// How a `Gas` cell is drawn against what is behind it -- see
    /// `GasMode`. `ByFill` by default, cycled by `;`.
    pub gas: GasMode,
    /// Whether the gnome weaves through a stand or draws over it — see
    /// `TreeDepth`. Cycled with `F10`.
    pub tree_depth: TreeDepth,
    /// Frame counter, advanced by `draw`, read only by `GrainMode::Animated`.
    frame: u64,
    /// Screen rectangles the chunk bodies occupied on the previous `draw`.
    ///
    /// A body is painted on top of the per-cell pass, so when it moves it
    /// leaves stale pixels behind that nothing else will repaint. Keeping
    /// last frame's rectangles is what lets the dirty region cover the
    /// smear without falling back to redrawing the whole screen -- which is
    /// what this did before, for the entire duration of every collapse.
    last_body_rects: Vec<Rect>,
    /// Screen rect the gnome occupied on the previous `draw`, **and how he
    /// was posed in it** — the same smear-repaint reasoning as
    /// `last_body_rects`. Kept separately, and compared before unioning: a
    /// gnome standing still contributes *nothing* to the dirty region,
    /// which is what keeps a settled world's zero-cost frames zero-cost
    /// with a character idle in them.
    ///
    /// The rect alone is not enough, and the gap is a real artifact:
    /// turning on the spot, or starting a swing, changes every pixel of him
    /// while his rectangle stays put — so a rect-keyed comparison skips the
    /// repaint and leaves the previous pose on screen until something else
    /// happens to dirty it.
    ///
    /// The rect alone is not enough and the difference is a real artifact:
    /// turning on the spot, or starting a swing, changes every pixel of him
    /// while his rectangle stays put — so a rect-keyed comparison skips the
    /// repaint and leaves the previous pose on screen until something else
    /// happens to dirty it.
    last_player_pose: Option<(Rect, bool, bool)>,
    /// World coordinate displayed at the top-left pixel. Moved by
    /// [`Renderer::follow`] once there is a player to follow.
    pub camera_x: i32,
    pub camera_y: i32,
    /// The camera as last painted, so a move can force a full redraw. Same
    /// role as `last_zoom_state`, and for the same reason.
    last_camera: Option<(i32, i32)>,
    /// Sub-cell scroll carried between frames by `pan`. See that method for
    /// why truncating it instead would make one held key feel like a
    /// different speed at every zoom level.
    pan_residual: (f32, f32),
    /// Seconds the current scroll gesture has been held, for the ramp in
    /// `pan`. Cleared by `end_pan`, so a tap is always a tap.
    pan_held_for: f32,
    /// The direction the current gesture is going, so reversing it can restart
    /// the ramp rather than inherit full speed — see `pan`.
    pan_dir: (i32, i32),
    /// Draws chunk boundaries tinted by whether the chunk will be swept next
    /// frame. This is the primary way to confirm sleeping actually works.
    pub show_chunk_overlay: bool,
    /// Screen pixels per world cell, 1-`MAX_ZOOM`, nearest-neighbour
    /// magnify. `1` is the original unscaled mapping M2 shipped with.
    /// Mutually exclusive with `zoom_out_stride > 1` in practice —
    /// `adjust_zoom` never lets both climb above 1 at once, though nothing
    /// below is structurally prevented from being called with both set;
    /// it would just be a confusing scale to be in, not an unsafe one.
    pub zoom: i32,
    /// World cells sampled per screen pixel, 1-`MAX_ZOOM_OUT_STRIDE` — the
    /// reverse direction, seeing more of the world at once at the cost of
    /// skipping cells between samples rather than averaging them (a proper
    /// minify filter is not worth it for a debug/overview zoom level).
    pub zoom_out_stride: i32,
    /// Tints every pixel by an M13 field channel instead of (blended over)
    /// the ordinary cell colour — `V` cycles it. `Off` by default, so it
    /// costs nothing (no extra `World::field_at` calls) unless a player
    /// opts in, the same "toggled debug overlay, zero cost when off" shape
    /// `show_chunk_overlay` already uses.
    pub field_overlay: FieldOverlay,
    /// Tints organism-owned cells by one of `organism.rs`'s per-cell
    /// channels — `B` cycles it. `Off` by default and costs nothing then,
    /// same shape as `field_overlay` above.
    ///
    /// **The scalar channels defeat the dirty-rect skip; `CellType` does
    /// not.** That split is exact, and the reasoning changed when the
    /// scalars moved off `Cell::aux`.
    ///
    /// This overlay used to be able to keep the skip wholesale, on the
    /// argument that every channel it draws lives in `Cell::aux` and can
    /// only change via a `set`, so a changed reading always came with a
    /// dirtied chunk. **Decision 2 step 2c falsified that.** `carbon`,
    /// `canopy_density` and `carbon_conductance` live in `OrganismState`
    /// now, and `plant.rs` deliberately relies on writing them *without*
    /// touching the grid — that is what stopped an organism keeping the CA
    /// sweep awake merely by existing. So those three change with no chunk
    /// dirtied, and on a settled world the overlay would freeze while the
    /// values underneath it kept moving.
    ///
    /// Which is precisely the failure this overlay was built to prevent:
    /// `CLAUDE.md`'s "a debug readout must not be a function of the thing
    /// it debugs" — a frozen sheet and a genuinely static channel draw the
    /// same picture, and the obvious reading ("the mechanism is dead")
    /// would send a fix at working code. It has already cost this project
    /// one wrong diagnosis.
    ///
    /// `CellType` is exempt because a cell type really does live in `aux`
    /// and really can only change through a `set`. It keeps the skip.
    ///
    /// **Cost, stated because it is the thing being traded** (`CLAUDE.md`:
    /// measure a cost against the state the optimisation exists for): a
    /// full redraw every frame is ~10 ms mean on a *settled* world, the
    /// number `grain`'s animated modes are documented against. This is a
    /// debug overlay that is `Off` by default and costs exactly nothing
    /// then, which is the same bargain `field_overlay` already makes.
    pub organism_overlay: OrganismOverlay,
    /// `organism_overlay` as of the last `draw` call. A change means every
    /// existing pixel in the buffer was tinted for a different channel, so
    /// one full redraw has to re-establish it — the same reason
    /// `last_zoom_state` exists, and the reason this overlay can otherwise
    /// leave the dirty-rect skip alone.
    last_organism_overlay: OrganismOverlay,
    /// `(zoom, zoom_out_stride)` as of the last `draw` call — a change since
    /// then means the whole frame buffer's existing bytes were computed at
    /// the wrong scale, forcing one full redraw to re-establish it before
    /// `draw`'s dirty-rect skip (see its own doc) can trust anything already
    /// in the buffer again. `None` until the first `draw`, so that call is
    /// always full too, for the same reason: an unwritten buffer has nothing
    /// valid to partially build on.
    last_zoom_state: Option<(i32, i32)>,
    /// The look toggles as of the last draw — forces one full repaint when
    /// `F10`/`F11` flip. See the `look_changed` note in `draw`.
    last_look: Option<(TerrainLight, bool, GrainMode, GlowShape)>,
    /// Field tiles with any glowing material in them **plus their eight
    /// neighbours** (the diffused halo crosses tile seams; gating on the
    /// glow tile alone clipped it square at every chunk boundary), rebuilt
    /// per draw from `FieldTile::has_glow`. The per-pixel gate that keeps
    /// local light free for the 99% of the world that has none: a pixel
    /// only samples the field's light channel when its chunk is in this
    /// set, so a glow-free world pays one `is_empty()` test per pixel and
    /// nothing else.
    glow_tiles: std::collections::HashSet<ChunkCoord>,
    /// Per-cell near-field glow, one buffer per chunk in `glow_tiles`,
    /// rebuilt whenever those tiles change or are still converging. Splat
    /// from the *emitting cells themselves* rather than read from the
    /// field, because the field has already thrown the emitter's position
    /// away -- see `NEAR_GLOW_RADIUS`.
    near_glow: std::collections::HashMap<ChunkCoord, Vec<f32>>,
    /// Which emitter tiles `near_glow` was built from, so a settled world
    /// does not rebuild it every draw.
    near_glow_key: Option<Vec<ChunkCoord>>,
    /// How many times the splat has been rebuilt. Exists so a test can ask
    /// "did it fire", which no picture and no frame timing can answer: a
    /// halo that is rebuilt every frame looks exactly like one that is
    /// cached, and the cost only shows up on the settled world the
    /// dirty-rect skip exists for.
    pub near_glow_rebuilds: u32,
    /// The sky as of this frame, recomputed once per `draw` and read by
    /// every empty pixel. Held rather than passed because `cell_colour` is
    /// the per-pixel hot path and recomputing a cosine there would be paying
    /// for the same answer up to 160,000 times a frame.
    sky: Sky,
    /// The quantised sky last actually painted. A frame whose sky key still
    /// matches this one would repaint the screen to exactly the same pixels,
    /// so it does not — which is what keeps a day/night sky from costing the
    /// dirty-rect skip. See `sky::Sky::key`.
    last_sky_key: Option<[i32; 7]>,
    /// How lit the world is, `0..=LIGHT_LEVELS`, for this frame.
    ///
    /// **From the global daylight, not from the light channel**, and that was
    /// a correction rather than a shortcut. Measured at noon on open terrain,
    /// the channel reads 0.30 of `MAX_LIGHT` at the ground surface and 0.00
    /// forty cells down: light diffuses through air and is stopped by solids,
    /// so it never meaningfully enters the material it would have to light.
    /// Driving the ground's brightness from it produced a picture that looked
    /// right for the wrong reason — every solid pinned at the unlit floor all
    /// day, with the visible day/night swing coming entirely from the ambient
    /// tint. Lighting rock by the light channel needs light to propagate into
    /// rock, which is the "caves are dark, bring a torch" feature and not
    /// this one.
    daylight: u8,
    /// `Some(frame)` draws every frame's *lighting* as if it were that frame,
    /// leaving the simulation alone.
    ///
    /// **For contact sheets of slow processes.** Every cell is tinted and
    /// dimmed by the day/night cycle (`sky::apply_light` below), and
    /// `DAY_NIGHT_PERIOD_FRAMES` is 3600, so a sheet sampling a multi-day
    /// process walks its tiles around the light cycle and adjacent tiles
    /// differ in brightness for reasons that have nothing to do with what the
    /// sheet was cut to show. Reported from play against a six-minute ice
    /// arc: *"you can see a different ice morphology between the first and
    /// second half"* -- and tile six was simply at dusk.
    ///
    /// A render-side pin rather than moving the sample frames, which was
    /// tried first and is worse: quantising the interval to whole days is the
    /// only way to make sampled frames share a phase, and that doubled the
    /// span (and the runtime) of the one acceptance case it touched while
    /// changing which frames were being judged. This changes no frame and no
    /// timing. `field::noon_equivalent_light` divides the same oscillator out
    /// of *decisions*; this is its render-side twin.
    pub pinned_light: Option<u64>,
    /// `CAVE_FADE_DEPTH`'s ramp, precomputed: how far toward `UNDERGROUND`
    /// a cell `i` rows below its column's sky floor is drawn, as a 0..=255
    /// weight.
    ///
    /// A table rather than the expression, because the branch that reads it
    /// is per *pixel* and rendering has no dirty-rect skip for a region that
    /// did change: a screen looking into a large cavity is every pixel on
    /// the underground branch, and a square root each is real work for a
    /// value that only ever takes `CAVE_FADE_DEPTH + 1` distinct settings.
    /// Built from the formula in `Renderer::new` rather than written out, so
    /// the constant above stays the only place the shape is stated.
    cave_ramp: [u8; CAVE_FADE_DEPTH as usize + 1],
    /// `F10` — whether solid terrain is lit by depth below the skyline.
    pub terrain_light: TerrainLight,
    /// Where a glow's shape comes from -- cycled with `'`. See `GlowShape`.
    pub glow_shape: GlowShape,
    /// `F11` — reveal every void inside the ground, for testing.
    ///
    /// Sealed chambers are invisible by design (dark cave fade against dark
    /// deep rock, 200+ rows down), which makes "did this world get a vault,
    /// and where" unanswerable in the app without digging blind. With this
    /// on, materially-empty cells below the frozen skyline draw as a flat
    /// bright marker instead of unlit rock, so every enclosed void — vault
    /// domes, blast cavities, tunnels — reads at any zoom. A debug reveal,
    /// not a look: full replace with a colour nothing else in the world
    /// uses, on the same reasoning as the field overlays' fixed ramps
    /// (a blend into the world's own palette is exactly how the canopy
    /// overlay once read as blank).
    pub reveal_voids: bool,
    /// `DEPTH_LIGHT_RAMP_ROWS`'s ramp, precomputed for the same reason as
    /// `cave_ramp` directly above: read per solid pixel, and it only ever
    /// takes `DEPTH_LIGHT_RAMP_ROWS + 1` distinct settings. Fixed-point
    /// brightness factor, 256 = 1.0; entry 0 is the skyline highlight and
    /// exceeds 256 on purpose.
    depth_light_ramp: [u16; DEPTH_LIGHT_RAMP_ROWS as usize + 1],
    /// Where the moon was painted last frame, so its old pixels get cleaned
    /// up. Same device as `last_body_rects`.
    last_moon_rect: Option<Rect>,
    /// Topmost non-empty row per world column — the skyline.
    ///
    /// Empty space above it is open air and draws as sky; empty space below
    /// it is *inside the ground* and draws as unlit rock. Without this every
    /// void drew as sky, so blasting a cavity under a mountain showed blue
    /// daylight through the middle of it, and so did every cave and every
    /// undercut. Reported from play, and obvious in hindsight: "empty" and
    /// "open to the sky" are not the same question.
    horizon: Vec<i32>,
    /// The skyline as the *depth light* reads it: `horizon` with dips
    /// narrower than the shoulder window clipped to their shoulders (see
    /// `DEPTH_LIGHT_SHOULDER_REACH`). Rebuilt beside `horizon` from the same
    /// frozen source, so it carries the same purity guarantee. Only the
    /// terrain depth light reads it — sky-vs-cave (`sky_depth`) stays on the
    /// raw skyline, because the air in a narrow notch genuinely *is* open
    /// sky even though the rock beside it should not light up.
    light_datum: Vec<i32>,
    /// World x the `horizon` entries start at.
    horizon_origin: i32,
    /// `World::underground_map`, copied per frame beside `horizon` and for
    /// the same reasons — see `rebuild_horizon`'s memcpy note, which applies
    /// here at 164 KB instead of 8 KB.
    ///
    /// **This is what decides sky-versus-cave now; `horizon` only says how
    /// deep.** The two were one question while the answer was per column,
    /// and splitting them is the whole of the dark-bands fix
    /// (`Reports/dark-bands-diagnosis.md`): the air outside a cliff brow has
    /// rock above it in its column, so it must still take a *depth* from
    /// that column while not being underground at all.
    underground: Vec<u64>,
    /// The rect `underground` is indexed over, or `None` when the world has
    /// no map — a world nothing has ever stepped, where `under_sky` falls
    /// back to `horizon`.
    underground_rect: Option<Rect>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            grain: GrainMode::default(),
            bubbles: BubbleMode::default(),
            gas: GasMode::default(),
            tree_depth: TreeDepth::default(),
            frame: 0,
            last_body_rects: Vec::new(),
            last_player_pose: None,
            camera_x: 0,
            camera_y: 0,
            pan_residual: (0.0, 0.0),
            pan_held_for: 0.0,
            pan_dir: (0, 0),
            last_camera: None,
            show_chunk_overlay: false,
            zoom: 1,
            zoom_out_stride: 1,
            field_overlay: FieldOverlay::Off,
            organism_overlay: OrganismOverlay::Off,
            last_organism_overlay: OrganismOverlay::Off,
            last_zoom_state: None,
            last_look: None,
            glow_tiles: std::collections::HashSet::new(),
            near_glow: std::collections::HashMap::new(),
            near_glow_key: None,
            near_glow_rebuilds: 0,
            sky: Sky::at(0, 0, 1, 0, 1),
            last_sky_key: None,
            daylight: sky::LIGHT_LEVELS,
            pinned_light: None,
            cave_ramp: {
                let mut ramp = [0u8; CAVE_FADE_DEPTH as usize + 1];
                for (i, slot) in ramp.iter_mut().enumerate() {
                    // Square-rooted, not linear, and it is one constant
                    // doing two jobs. Linear left the first row under a
                    // roof at 96% of full daylight, which reads as a
                    // *window* rather than a roof -- the thing directly
                    // overhead blocks the most, so most of the drop belongs
                    // in the first few rows. The root puts 20% of it in the
                    // first row and 41% by the fourth, and still takes the
                    // full depth to reach dark.
                    *slot = ((i as f32 / CAVE_FADE_DEPTH as f32).sqrt() * 255.0).round() as u8;
                }
                ramp
            },
            terrain_light: TerrainLight::default(),
            glow_shape: GlowShape::default(),
            reveal_voids: false,
            depth_light_ramp: {
                let mut ramp = [256u16; DEPTH_LIGHT_RAMP_ROWS as usize + 1];
                ramp[0] = (DEPTH_LIGHT_HIGHLIGHT * 256.0).round() as u16;
                for (i, slot) in ramp.iter_mut().enumerate().skip(1) {
                    // Smoothstepped, unlike `cave_ramp`'s square root, and
                    // for the opposite reason: the cave fade wants most of
                    // its drop in the first rows (a roof blocks the most),
                    // while sunlight soaking into a hillside should leave
                    // the surface *band* bright and ease into the dark —
                    // a fast first-row drop here read as the whole surface
                    // being one bright crust on a dark slab. Smoothstep is
                    // flat at both ends: no knee at the surface, no knee
                    // where the ramp meets the floor.
                    let t = i as f32 / DEPTH_LIGHT_RAMP_ROWS as f32;
                    let s = (1.0 - t) * (1.0 - t) * (3.0 - 2.0 * (1.0 - t));
                    *slot = ((DEPTH_LIGHT_FLOOR + (1.0 - DEPTH_LIGHT_FLOOR) * s) * 256.0).round() as u16;
                }
                ramp
            },
            last_moon_rect: None,
            horizon: Vec::new(),
            light_datum: Vec::new(),
            horizon_origin: 0,
            underground: Vec::new(),
            underground_rect: None,
        }
    }

    /// `F10` — toggle the terrain depth light, so the review's largest
    /// graphics change can be judged as an A/B in the running app against
    /// the pre-review look. Same convention as `cycle_grain` below.
    pub fn cycle_glow_shape(&mut self) {
        self.glow_shape = self.glow_shape.next();
        // The halo is not owned by any chunk the CA touched, so nothing else
        // would ever repaint it.
        self.last_look = None;
    }

    pub fn cycle_terrain_light(&mut self) {
        self.terrain_light = self.terrain_light.next();
    }

    /// `G` — step through the liquid grain modes. This exists so the
    /// variants can be judged on real moving water in the real app, which is
    /// the only way a "does this look right" question gets answered, and it
    /// stays for the same reason: the look is expected to keep being
    /// iterated on rather than settled once.
    /// Cycle the tree-depth selector. `F10`.
    pub fn cycle_tree_depth(&mut self) -> TreeDepth {
        self.tree_depth = self.tree_depth.next();
        self.tree_depth
    }

    pub fn cycle_grain(&mut self) {
        self.grain = self.grain.next();
    }

    /// `H` — step through the boiling looks. Same reasoning as
    /// `cycle_grain`, and the same reason it stays rather than being
    /// collapsed once one is picked: whether a pool reads as boiling is a
    /// judgement about motion, and a contact sheet can only get it so far.
    pub fn cycle_bubbles(&mut self) {
        self.bubbles = self.bubbles.next();
    }

    /// `;` -- step through the gas looks. Same reasoning as `cycle_grain`.
    pub fn cycle_gas(&mut self) {
        self.gas = self.gas.next();
    }

    pub fn cycle_field_overlay(&mut self) {
        self.field_overlay = self.field_overlay.next();
    }

    /// `B` — step through the organism channels. Same reasoning as
    /// `cycle_grain`: these are questions ("is canopy density actually
    /// non-zero where a tip is about to grow?") that only a picture of the
    /// real running plant answers.
    pub fn cycle_organism_overlay(&mut self) {
        self.organism_overlay = self.organism_overlay.next();
    }

    /// `delta > 0` zooms in a step, `delta < 0` zooms out a step — `=`/`-`
    /// in the live app. The two fields form one continuous scale rather
    /// than being independently adjustable: zooming in past `zoom_out_
    /// stride > 1` counts down the stride back to 1 before `zoom` itself
    /// starts climbing, and symmetrically the other way, so the key pair
    /// How many world cells the viewport spans, at the current scale.
    ///
    /// Zoom and zoom-out are one continuous control (see `adjust_zoom`), so
    /// this has to consult both: a 512-pixel viewport shows 512 cells at 1:1,
    /// 256 at zoom 2, and 1024 at stride 2.
    pub fn visible_span(&self, viewport: (u32, u32)) -> (i32, i32) {
        let (w, h) = (viewport.0 as i32, viewport.1 as i32);
        if self.zoom > 1 {
            (w / self.zoom, h / self.zoom)
        } else {
            let stride = self.zoom_out_stride.max(1);
            (w * stride, h * stride)
        }
    }

    /// Centre the view on a world position, clamped inside the world.
    ///
    /// A **dead zone** rather than a hard centring: the camera only moves once
    /// the target has drifted away from the middle, and then only far enough
    /// to bring it back to the edge of that zone. Hard centring would move the
    /// camera on almost every frame the player is walking, and a camera move
    /// forces a full redraw (see `draw`) — so a strictly-centred view would
    /// repaint the whole screen every frame anyone was moving, which is the
    /// dirty-rect skip paid away for nothing.
    ///
    /// Clamped to the world so the view never shows the void beyond it; on a
    /// world smaller than the viewport the clamp collapses and the camera
    /// simply sits at the origin.
    pub fn follow(&mut self, target: (i32, i32), viewport: (u32, u32), bounds: Option<Rect>) {
        let (span_x, span_y) = self.visible_span(viewport);
        let centre_x = self.camera_x + span_x / 2;
        let centre_y = self.camera_y + span_y / 2;
        let dead_x = (span_x / 6).max(1);
        let dead_y = (span_y / 6).max(1);

        // Leaving the zone **re-centres**, rather than dragging the camera
        // just far enough to put the target back on the boundary. Dragging to
        // the boundary leaves it sitting exactly on the edge, so the very next
        // step crosses again and the camera moves every single frame anyone is
        // walking — which is a full-screen repaint every frame, the precise
        // cost the dead zone exists to avoid. Re-centring buys a whole zone of
        // travel before the next move.
        let mut cam_x = self.camera_x;
        let mut cam_y = self.camera_y;
        if (target.0 - centre_x).abs() > dead_x {
            cam_x = target.0 - span_x / 2;
        }
        if (target.1 - centre_y).abs() > dead_y {
            cam_y = target.1 - span_y / 2;
        }

        self.set_camera(cam_x, cam_y, viewport, bounds);
    }

    /// Put the camera at `(x, y)`, clamped so the viewport stays inside
    /// `bounds` and never shows the void beyond the world.
    ///
    /// The clamp lived inside `follow` while `follow` was the only thing that
    /// ever moved the camera. It is not any more — `pan` writes it too — and a
    /// clamp only one of two writers goes through is a clamp that is not
    /// there. `camera_x`/`camera_y` being `pub` makes that a live hazard
    /// rather than a theoretical one.
    pub fn set_camera(&mut self, x: i32, y: i32, viewport: (u32, u32), bounds: Option<Rect>) {
        let (span_x, span_y) = self.visible_span(viewport);
        let (mut cam_x, mut cam_y) = (x, y);
        if let Some(b) = bounds {
            // `max` before `min` so a world narrower than the viewport pins to
            // its own origin rather than to a negative coordinate.
            cam_x = cam_x.min(b.max_x - span_x + 1).max(b.min_x);
            cam_y = cam_y.min(b.max_y - span_y + 1).max(b.min_y);
        }
        self.camera_x = cam_x;
        self.camera_y = cam_y;
    }

    /// Scroll the view. `dir` is -1/0/+1 per axis, `seconds` is real elapsed
    /// time — the `WASD` map scroll with no gnome in the world, driven through
    /// `App::pan_camera`.
    ///
    /// The counterpart to `follow`, for a view with nothing to follow, and
    /// deliberately **without** a dead zone: `follow`'s exists because a
    /// walking character would otherwise drag the camera every frame for
    /// nothing, whereas here the delta *is* the request, and damping the input
    /// against itself would only make the scroll feel like it was fighting
    /// you.
    ///
    /// **Accelerates while held**, from `PAN_START_FRACTION` of the rate to
    /// all of it over `PAN_RAMP_SECONDS`. A tap therefore moves the view by a
    /// nudge and a sustained hold still crosses the world in about six
    /// seconds — the two halves of the playtest verdict that killed the flat
    /// 1.5 screens/s this shipped with.
    ///
    /// The sub-cell remainder is **carried, not truncated**. At `zoom` 8 the
    /// viewport is 64 cells wide, so the rate above is about 1.6 cells a
    /// frame; truncating each frame independently emits 1 and scrolls at 0.94
    /// screens a second instead of 1.5 — visibly a different feel from the
    /// same key at 1:1, which is the exact thing a screens-per-second rate
    /// exists to make impossible.
    ///
    /// **Quantised to `zoom_out_stride`, and this one is not obvious.** A
    /// zoomed-out view samples every `stride`-th cell (`screen_to_world` is
    /// `camera + sx * stride` there), so a camera step that is *not* a whole
    /// number of strides does not translate the picture at all — it resamples
    /// the same view against a different lattice, the odd columns instead of
    /// the even ones at stride 2, and reads as the screen hissing rather than
    /// scrolling. Whole strides move it by exactly one screen pixel each, so
    /// that is the unit; anything less waits in the residual until it is one.
    ///
    /// Diagonals are deliberately not normalised — holding two keys is
    /// sqrt(2) faster, as in every map editor, and correcting it would make
    /// two held keys slower than the sum of one.
    ///
    /// Costs a full redraw on every frame it actually moves the camera (see
    /// `draw`'s `camera_moved`). That is the same price `follow` already pays
    /// whenever the gnome walks, and unlike an animated grain it ends when the
    /// key comes up — bounded by the gesture rather than permanent.
    pub fn pan(&mut self, dir: (i32, i32), seconds: f32, viewport: (u32, u32), bounds: Option<Rect>) {
        let (span_x, span_y) = self.visible_span(viewport);

        // **Reversing restarts the ramp.** Overshooting and tapping back is
        // exactly the correction a slow start exists to make possible, and
        // carrying full speed into it would fling the view the other way just
        // as hard — turning one overshoot into two. A reversal is a new
        // gesture, so it begins like one. Reset here rather than in `main.rs`
        // for the same reason the rate lives here: `main.rs` reports which way
        // the keys are pointing and nothing else.
        // A **sign flip on either axis**, not merely a different `dir`. Adding
        // a second direction to a scroll already under way — pressing `S`
        // while travelling right — is not a reversal and must not drop you
        // back to walking pace mid-journey; releasing one of two is not one
        // either. Only actually turning round is.
        let reversed = dir.0 * self.pan_dir.0 < 0 || dir.1 * self.pan_dir.1 < 0;
        if reversed {
            self.pan_held_for = 0.0;
            self.pan_residual = (0.0, 0.0);
        }
        self.pan_dir = dir;
        self.pan_held_for += seconds;

        // Linear from `PAN_START_FRACTION` of the rate to all of it across
        // `PAN_RAMP_SECONDS`, then flat. Applied to the *rate*, not to the
        // distance, so the ramp is a speed curve and the carry below stays a
        // plain remainder.
        let ramp = (self.pan_held_for / PAN_RAMP_SECONDS).clamp(0.0, 1.0);
        let rate = PAN_SCREENS_PER_SECOND * (PAN_START_FRACTION + (1.0 - PAN_START_FRACTION) * ramp);
        let travel = rate * seconds;
        self.pan_residual.0 += dir.0 as f32 * span_x as f32 * travel;
        self.pan_residual.1 += dir.1 as f32 * span_y as f32 * travel;

        // Truncation toward zero, so the leftover keeps the sign of the
        // direction being travelled and is carried rather than rounded away.
        // (This used to be justified by reversals not having to unwind a debt
        // first; they no longer can, because a reversal clears the residual
        // outright above.)
        let step = self.zoom_out_stride.max(1);
        let dx = self.pan_residual.0 as i32 / step * step;
        let dy = self.pan_residual.1 as i32 / step * step;
        self.pan_residual.0 -= dx as f32;
        self.pan_residual.1 -= dy as f32;
        self.set_camera(self.camera_x + dx, self.camera_y + dy, viewport, bounds);
    }

    /// Drop any carried sub-cell scroll, so a fresh gesture starts clean
    /// rather than inheriting a fraction of a cell from the last one.
    pub fn end_pan(&mut self) {
        self.pan_residual = (0.0, 0.0);
        self.pan_held_for = 0.0;
        self.pan_dir = (0, 0);
    }

    /// reads as a single "more/less zoom" control, not two separate ones a
    /// player has to understand are different mechanisms.
    pub fn adjust_zoom(&mut self, delta: i32) {
        if delta > 0 {
            if self.zoom_out_stride > 1 {
                self.zoom_out_stride -= 1;
            } else {
                self.zoom = (self.zoom + 1).min(MAX_ZOOM);
            }
        } else if delta < 0 {
            if self.zoom > 1 {
                self.zoom -= 1;
            } else {
                self.zoom_out_stride = (self.zoom_out_stride + 1).min(MAX_ZOOM_OUT_STRIDE);
            }
        }
    }

    /// §11's dirty-rect render optimization. **Not a GPU-level change** —
    /// `pixels` 0.17.2's `render`/`render_with` re-upload the *entire*
    /// frame buffer to the GPU texture unconditionally, from a code path
    /// with no hook to narrow it and no public accessor for the
    /// `wgpu::Surface` needed to drive an alternative present path (the
    /// underlying architectural constraint `PLAN.md`'s section 11 entry
    /// documents, confirmed by reading `pixels`' own source before starting
    /// here). What's actually skippable, and what this does instead, is the
    /// CPU-side cost of *computing* those bytes in the first place: the
    /// `cell_colour` pass this replaces reruns for every one of up to
    /// 512*320 pixels every single frame regardless of whether anything
    /// changed, a cost `cell_colour`'s own doc already names as real (the
    /// fake-AO experiment it records being cut for measured cost, on top of
    /// jitter and heat glow that stayed only after being budgeted).
    ///
    /// Recomputing pixels for exactly `touched`'s chunks — no more, no
    /// less — is a sound skip condition, not an approximation, *given*
    /// `touched` is accurate: `fire::update`'s `must_stay_dirty` (see
    /// `fire.rs`) guarantees any cell still burning or still outside
    /// `THERMAL_SETTLE_EPSILON` of ambient keeps re-dirtying its own chunk
    /// every tick, and `fire::update`/movement both only ever run on a
    /// chunk actually being swept — so a chunk *absent* from `touched` has,
    /// by construction, cell data that provably did not change across every
    /// tick since the caller last fetched that set. `cell_colour` is a pure
    /// function of that data plus position (jitter) and a burning-only
    /// flicker bucket that can't apply to an untouched cell either — so
    /// recomputing it would reproduce the exact bytes already sitting in
    /// `frame`, byte for byte. Skipping is lossless, not lossy.
    ///
    /// **Why `touched`, not a `chunk.is_settled()` check done here at draw
    /// time** — the first version of this did exactly that, and a debug
    /// harness that called `App::update` many times before ever drawing
    /// again caught it stale: a chunk can go active and settle again
    /// *between* two draws whenever more than one tick happens per draw,
    /// which `main.rs`'s own catch-up loop (`MAX_TICKS_PER_FRAME`) does on
    /// any frame that runs behind — `is_settled()` checked only at the end
    /// answers "did this chunk change on its *most recent* tick," not "did
    /// it change on *any* tick since I last drew it," and only the second
    /// question is the one this function actually needs answered.
    /// `World::take_touched_chunks` accumulates across every tick's
    /// `end_step` instead of snapshotting one, closing that gap.
    ///
    /// Bypassed back to a full redraw (matching the original per-pixel
    /// loop exactly) whenever *anything* could have written into the frame
    /// buffer outside of touched-chunk logic: `zoom`/`zoom_out_stride`
    /// changed since the last call (the existing buffer is at the wrong
    /// scale, or covers different world positions, including the
    /// out-of-bounds `VOID` fringe a stride>1 view can expose); the field
    /// overlay is on (the M13 field grid diffuses independently of CA
    /// activity, so an untouched chunk's *tint* can still be changing even
    /// though its cells aren't); `show_chunk_overlay` is on (an active/
    /// settled border colour can flip without the chunk's own cells
    /// changing at all); or particles exist (free debris is drawn as a
    /// second pass below, at a position nothing here tracks turn over
    /// turn — bypassing avoids the alternative, real cost of tracking a
    /// leave/enter footprint for something already fast to just redraw).
    /// `force_full`, the caller's own get-out: `App::draw` sets it
    /// whenever an on-screen overlay (HUD panels, the brush outline at the
    /// cursor) is about to be painted over this frame's terrain, for the
    /// identical reason — this function has no way to know an old cursor
    /// position's outline needs erasing, so the caller says so instead.
    ///
    /// Returns how many pixels were actually recomputed, `width*height` on
    /// a full redraw — the objective, no-visual-judgment measurement
    /// `PLAN.md`'s section 11 asked for, exercised directly by this
    /// module's own tests rather than an env-var-gated instrumentation
    /// hook layered on afterward.
    /// `touched` — every chunk any tick has actually swept since the last
    /// call to `World::take_touched_chunks` (the caller's job to fetch;
    /// see that method's own doc for why this can't just be `chunk.
    /// is_settled()` checked here at draw time — a chunk can go active and
    /// settle again *between* draws, if `App::update` runs more than once
    /// per `App::draw`, which `main.rs`'s own catch-up loop does whenever a
    /// frame runs behind).
    pub fn draw(&mut self, world: &World, particles: &ParticleSystem, touched: &HashSet<ChunkCoord>, frame: &mut [u8], (width, height): (u32, u32), force_full: bool) -> usize {
        let zoom_state = (self.zoom, self.zoom_out_stride);
        let scale_changed = self.last_zoom_state != Some(zoom_state);
        self.last_zoom_state = Some(zoom_state);

        // A camera move invalidates every byte already in the buffer, exactly
        // as a scale change does — every pixel now shows a different world
        // cell. `last_zoom_state` exists for precisely that reason and the
        // camera had no equivalent, because until now nothing ever moved it.
        // Without this the dirty-rect skip repaints only the touched chunks
        // and scrolls a frozen, smeared world past them.
        let camera = (self.camera_x, self.camera_y);
        let camera_moved = self.last_camera != Some(camera);
        self.last_camera = Some(camera);

        // One full redraw when the organism channel changes, and — for the
        // channels that read the sidecar rather than `Cell::aux` — one every
        // frame, since those values change with no chunk dirtied. See
        // `organism_overlay`'s own doc for why that split is exact.
        let organism_overlay_changed = self.last_organism_overlay != self.organism_overlay;
        self.last_organism_overlay = self.organism_overlay;
        let organism_overlay_is_live = matches!(
            self.organism_overlay,
            OrganismOverlay::Resource | OrganismOverlay::CanopyDensity | OrganismOverlay::VeinConductance
        );

        self.frame = self.frame.wrapping_add(1);
        // The animated variants are the ones whose output changes with
        // nothing in the world changing, so they have to defeat the
        // dirty-rect skip. Measured on a fully settled world: a redraw every
        // frame costs **~10 ms mean, 12.8 ms worst**, against 0.000 ms for
        // every non-animated mode, which is most of a 60 Hz budget spent
        // redrawing water that is not moving.
        //
        // So the *stepped* variants only pay it on the frames their step
        // actually changes -- amortising that cost by the period, since a
        // redraw on any other frame would produce a pixel-identical image.
        // `AnimatedSmooth` interpolates across the whole interval and has no
        // such frames, so it pays in full; that is the price of being smooth
        // and it is why the stepped variants exist beside it.
        let animating = match self.grain {
            GrainMode::Animated | GrainMode::AnimatedMuted => {
                self.frame.is_multiple_of(GRAIN_ANIMATION_PERIOD)
            }
            GrainMode::AnimatedSmooth => true,
            _ => false,
        };
        // Particles still force a full redraw: they are drawn off-grid on
        // top of the per-cell pass, so the dirty-rect skip has no idea they
        // moved and would leave a smear behind them.
        //
        // **Chunk bodies used to be in this list and are not any more.**
        // Reported from play: "when something big breaks into lots of
        // little pieces, the performance gets bad." That is this, and the
        // fracture work made it much worse -- a collapse that used to
        // promote 13 bodies now promotes over 100, and *any* body at all
        // meant repainting the entire screen every frame for the whole
        // duration of the fall. The dirty-rect skip was doing nothing
        // during exactly the events with the most on screen.
        //
        // They still need repainting, so their rectangles are unioned into
        // the dirty region below -- both where they are now and where they
        // were last frame, since the stale pixels are the reason this was a
        // full redraw in the first place.
        // The sky changes with the clock rather than with the world, so
        // nothing in `touched` will ever mention it and the dirty region
        // cannot find it. It therefore has to force a redraw itself — but
        // only on the frames it has genuinely changed, which is what the
        // quantised key is for. Through most of a day that is no frames at
        // all; through a sunrise it is a modest fraction of them, which is
        // the one time of day worth repainting for.
        // Built from the **visible** rect, not the world's.
        //
        // The gradient puts its horizon band a fraction of the way down its
        // span and the moon crosses that span over a night. Keyed to a world
        // much larger than the viewport, the horizon lands far off screen and
        // the moon is visible for a sliver of the night — the sky would
        // quietly stop working at exactly the moment the world grew.
        let (vx0, vy0) = self.screen_to_world(0, 0);
        let (vx1, vy1) = self.screen_to_world(width as i32 - 1, height as i32 - 1);
        // Weather is read once here and reused for the sky and the drawn
        // precipitation below, so the two cannot disagree about what the
        // sky is doing -- a downpour drawn against a clear blue gradient
        // being the obvious way that goes wrong.
        let weather = crate::sim::weather::at(world.seed, world.frame);
        // **The drawn storm and the landing storm have to be the same
        // storm.** Falling precipitation is drawn straight from
        // `weather::at` and is never simulated -- `weather::step` puts water
        // in where it *lands*, deliberately writing nothing into a sky column
        // -- so the two are only ever kept in agreement by both reading the
        // same numbers. `World::storm_supply` is the second such number
        // (intensity was the first): the sky can only rain what it is
        // holding, and a bankrupt one that still drew a full downpour would
        // be visibly lying, for the whole length of a front, about the one
        // thing the player can see.
        let storm_supply = world.storm_supply();
        // Both the gradient and the ambient tint come from the same frame, so
        // a pin moves them together -- half-pinning would light the ground at
        // noon under a midnight sky.
        let lit_at = self.pinned_light.unwrap_or(world.frame);
        self.sky = Sky::at(lit_at, vx0, vx1.max(vx0 + 1), vy0, vy1.max(vy0 + 1))
            .muted(sky::overcast(weather.intensity));
        self.daylight = sky::daylight_level(lit_at);
        self.rebuild_horizon(world);
        let sky_key = self.sky.key();
        let sky_changed = self.last_sky_key != Some(sky_key);
        self.last_sky_key = Some(sky_key);
        // The two whole-look toggles (`F10` depth light, `F11` void reveal)
        // recolour cells that are already painted and settled, so flipping
        // either must repaint everything once or the screen shows a patchwork
        // of both looks until the next full frame. Same device as
        // `last_organism_overlay` — and it closes a real gap: the depth
        // light shipped without it and coasted on `sky_changed` firing most
        // frames, which is a coincidence, not a contract.
        // Grain is in the tuple for the same reason: switching between two
        // *static* grain modes recolours settled water that nothing will
        // repaint, so `G` over a still pond looked like a dead key — the
        // owner reported exactly that. (The animated modes force full
        // frames on their own; the static-to-static switches were the gap.)
        let look = (self.terrain_light, self.reveal_voids, self.grain, self.glow_shape);
        let look_changed = self.last_look != Some(look);
        self.last_look = Some(look);

        // Which field tiles hold a glowing material this frame, plus their
        // neighbours — the diffused halo crosses tile seams, and gating on
        // the glow tile alone clipped the halo square at every chunk
        // boundary. Rebuilt per draw from `has_glow`, which
        // `field::rebuild_blocked` maintains for free inside the scan it
        // already does; for the overwhelmingly common world with no glow
        // anywhere this loop finds nothing and `cell_colour` pays a single
        // `is_empty()` per pixel.
        //
        // `glow_unsettled` is the halo's version of `sky_changed`: while the
        // light channel is still converging around a new (or newly mined-out)
        // glow source, the halo brightens cells in chunks the CA never
        // touched, so no dirty rect will ever repaint them. Once the tile
        // settles the halo is static and the skip gets its work back.
        self.glow_tiles.clear();
        let mut glow_unsettled = false;
        let mut emitter_tiles: Vec<ChunkCoord> = Vec::new();
        for (&coord, tile) in world.fields_ref() {
            if tile.has_glow {
                glow_unsettled |= !tile.settled();
                emitter_tiles.push(coord);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        self.glow_tiles.insert(ChunkCoord::new(coord.x + dx, coord.y + dy));
                    }
                }
            }
        }
        // Sorted before anything reads it: `fields_ref()` is a `HashMap`, and
        // the repo's rule is that no observable output depends on its
        // iteration order. The splat itself accumulates with `max`, which is
        // order-independent, so this is belt and braces -- but the next
        // person to reach for a sum there gets determinism for free instead
        // of a bug that only shows up as a one-bit pixel difference.
        emitter_tiles.sort_unstable_by_key(|c| (c.x, c.y));
        // **Rebuilt only when it can have changed**, which matters more than
        // it looks: a settled world with a geode in it is precisely the state
        // the dirty-rect skip exists for, and rebuilding the splat every
        // draw would burn a chunk scan plus a disc per emitter on frames
        // that repaint nothing -- the same shape as the animated grain that
        // measured free in motion and cost ~10 ms on a settled world.
        //
        // The trigger is the **world**, not the light field. Keying it on
        // `glow_unsettled` was the first attempt and rebuilt on every single
        // draw, because the day/night cycle keeps the light channel moving
        // for good: a tile with any sky in it is never settled, and the
        // cache never hit once (measured: 9 rebuilds in 9 draws). The splat
        // reads `Material::glow` off cells and nothing else, so what can
        // invalidate it is a cell changing -- a crystal mined out, a new one
        // exposed -- which is exactly what `touched` reports.
        let emitters_touched = emitter_tiles.iter().any(|c| touched.contains(c));
        if emitters_touched || force_full || self.near_glow_key.as_deref() != Some(&emitter_tiles[..]) {
            self.rebuild_near_glow(world, &emitter_tiles);
            self.near_glow_key = Some(emitter_tiles);
        }

        let full = force_full
            || look_changed
            || glow_unsettled
            || scale_changed
            || camera_moved
            || sky_changed
            || organism_overlay_changed
            || organism_overlay_is_live
            || self.field_overlay != FieldOverlay::Off
            || self.show_chunk_overlay
            // Falling precipitation moves every frame and moves *everywhere*,
            // so there is no dirty rectangle that describes it and the whole
            // frame has to be repainted while it falls. Accepted knowingly:
            // rain is an event, the world is busy during one, and the cost
            // ends when the rain does. This is exactly the cost that made
            // `ParticleSystem` the wrong vehicle -- there it would have been
            // permanent instead.
            //
            // Gated on the supply as well, so a front over a bankrupt sky
            // does not buy the full-redraw cost for a frame with no streaks
            // in it. Same factor as the draw below, or this would force
            // repaints for rain that is not there (or, worse the other way,
            // skip repaints for rain that is).
            || (weather.is_precipitating() && storm_supply > 0.0)
            // A strike lifts every pixel in the frame, so the frame after one
            // has to be repainted or the flash sticks around as a permanently
            // brightened world.
            || crate::sim::weather::strike(world.seed, world.frame, world.bounds()).is_some()
            || crate::sim::weather::strike(world.seed, world.frame.wrapping_sub(1), world.bounds()).is_some()
            || !particles.is_empty();

        // Where the gnome is on screen this frame. Tracked through *both*
        // branches below: a full redraw repaints everything anyway, but if
        // it didn't also record the sprite's position, a run of full
        // frames (particles in flight, say) with the player moving would
        // leave the dirty path comparing against a rect from before the
        // run — and the sprite's last full-frame position would smear.
        let player_pose = world.player.as_ref().and_then(|p| {
            let (x0, y0, x1, y1) = p.bounds();
            self.world_rect_to_screen_rect(Rect::new(x0, y0, x1, y1), width, height)
                .map(|r| (r, p.facing_left, p.action > 0))
        });

        let recomputed = if full {
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let sx = (i % width as usize) as i32;
                let sy = (i / width as usize) as i32;
                let (wx, wy) = self.screen_to_world(sx, sy);
                let colour = self.cell_colour(world, wx, wy);
                pixel.copy_from_slice(&colour);
            }
            self.last_player_pose = player_pose;
            (width as usize) * (height as usize)
        } else {
            let mut dirty: Option<Rect> = None;
            // Where the bodies are now, and where they were when this last
            // ran. Both, because a body that moved leaves stale pixels
            // behind it and nothing else will repaint them.
            let mut body_rects: Vec<Rect> = Vec::with_capacity(world.chunk_bodies.len());
            for body in &world.chunk_bodies {
                let (x0, y0, x1, y1) = body.bounds();
                if let Some(r) = self.world_rect_to_screen_rect(Rect::new(x0, y0, x1, y1), width, height) {
                    body_rects.push(r);
                }
            }
            for r in body_rects.iter().chain(self.last_body_rects.iter()) {
                dirty = Some(match dirty {
                    Some(d) => d.union(*r),
                    None => *r,
                });
            }
            self.last_body_rects = body_rects;
            // The gnome, same treatment as the bodies — but only when the
            // rect actually changed. An idle character repaints on top of
            // identical pixels for free; a moving one owes both where it
            // is and the smear where it was.
            if player_pose != self.last_player_pose {
                for (r, _, _) in player_pose.iter().chain(self.last_player_pose.iter()) {
                    dirty = Some(match dirty {
                        Some(d) => d.union(*r),
                        None => *r,
                    });
                }
                self.last_player_pose = player_pose;
            }
            // The moon, where it is now and where it was. A disc crossing
            // the sky is a moving sprite as far as the dirty region is
            // concerned, and leaves a trail behind it without the second
            // rectangle.
            let moon_rect = self.sky.moon_rect().and_then(|(x0, y0, x1, y1)| {
                self.world_rect_to_screen_rect(Rect::new(x0, y0, x1, y1), width, height)
            });
            for r in moon_rect.iter().chain(self.last_moon_rect.iter()) {
                dirty = Some(match dirty {
                    Some(d) => d.union(*r),
                    None => *r,
                });
            }
            self.last_moon_rect = moon_rect;
            for coord in touched {
                if let Some(r) = self.world_rect_to_screen_rect(coord.bounds(), width, height) {
                    dirty = Some(match dirty {
                        Some(d) => d.union(r),
                        None => r,
                    });
                }
            }
            // An animated grain changes its output with nothing in the world
            // changing, so those chunks have to be redrawn even though the
            // sweep did not touch them -- but only the ones actually holding
            // liquid, since that is the only kind whose grain animates.
            // Redrawing the whole screen instead measured at ~10 ms on a
            // fully settled world, against 0.000 ms for every non-animated
            // mode; this makes the cost scale with visible water rather than
            // with screen size.
            if animating {
                for chunk in world.chunks().filter(|c| c.has_liquid()) {
                    if let Some(r) = self.world_rect_to_screen_rect(chunk.coord.bounds(), width, height) {
                        dirty = Some(match dirty {
                            Some(d) => d.union(r),
                            None => r,
                        });
                    }
                }
            }
            let mut n = 0usize;
            if let Some(rect) = dirty {
                for sy in rect.min_y..=rect.max_y {
                    for sx in rect.min_x..=rect.max_x {
                        let (wx, wy) = self.screen_to_world(sx, sy);
                        let colour = self.cell_colour(world, wx, wy);
                        put(frame, width, height, sx, sy, colour);
                        n += 1;
                    }
                }
            }
            n
        };

        // Over the world, not just over sky cells: rain falls in *front* of
        // the rock as well as against the sky. Drawn after the cells and
        // before the gnome, so he is in the weather rather than behind it.
        self.draw_precipitation(weather, storm_supply, world.frame, (vx0, vy0), (vx1, vy1), frame, width, height);
        if let Some(s) = crate::sim::weather::strike(world.seed, world.frame, world.bounds()) {
            self.draw_lightning(s, (vx0, vy0), (vx1, vy1), frame, width, height);
        }
        self.draw_particles(world, particles, frame, width, height);
        self.draw_chunk_bodies(world, frame, width, height);
        self.draw_player(world, frame, width, height);

        if self.show_chunk_overlay {
            self.draw_chunk_overlay(world, frame, width, height);
        }

        recomputed
    }

    /// Draw the falling half of the weather.
    ///
    /// The geometry comes from `sky::drops` in **world** coordinates and is
    /// projected here, which is what keeps the pattern anchored to the world
    /// as the camera pans. Doing it the other way round -- hashing against
    /// screen position -- is cheaper, and makes the entire rainfield slide
    /// sideways whenever the gnome walks.
    #[allow(clippy::too_many_arguments)]
    fn draw_precipitation(
        &self,
        weather: crate::sim::weather::Weather,
        storm_supply: f32,
        world_frame: u64,
        (vx0, vy0): (i32, i32),
        (vx1, vy1): (i32, i32),
        frame: &mut [u8],
        width: u32,
        height: u32,
    ) {
        use crate::sim::weather::Precipitation;
        let fall = match weather.kind {
            Precipitation::None => return,
            Precipitation::Rain => sky::Fall::Rain,
            Precipitation::Snow => sky::Fall::Snow,
        };
        // **Multiplied into the intensity, which is exactly the right knob
        // for it.** `sky::drops` treats intensity as a *thinning* factor --
        // light rain is fewer drops, not fainter ones, and its own comment
        // says why (fading them instead reads as fog). So a half-supplied sky
        // draws half the streaks, which is the same picture a half-supplied
        // storm deposits: `weather::step` thins its column budget by the
        // identical factor. Dimming the drops here instead would put the two
        // out of step -- a full-density downpour that lands a trickle.
        for drop in
            sky::drops(world_frame, fall, weather.intensity * storm_supply, weather.wind, (vx0, vy0), (vx1, vy1))
        {
            // Walked in **world** space from tail to head -- downward, the
            // way it falls -- so the walk can stop at the ground. Drawn in
            // screen space by projecting each sample, rather than walking
            // screen pixels, because the question "has this hit anything
            // yet" is a world question and only the world has an answer.
            //
            // `world_to_screen` returns `None` off-screen and a streak with
            // one end just outside the view is ordinary, so the projection
            // is done by hand and `blend` clips per pixel.
            let project = |wx: f32, wy: f32| {
                (
                    (wx.round() as i32 - self.camera_x) / self.zoom_out_stride * self.zoom,
                    (wy.round() as i32 - self.camera_y) / self.zoom_out_stride * self.zoom,
                )
            };
            let (hx, hy) = (drop.from.0 - drop.to.0, drop.from.1 - drop.to.1);
            let (px, py) = project(drop.from.0, drop.from.1);
            let (qx, qy) = project(drop.to.0, drop.to.1);
            let steps = (px - qx).abs().max((py - qy).abs()).max(1);
            let colour = [drop.colour[0] as u8, drop.colour[1] as u8, drop.colour[2] as u8, 255];
            let mut landed = None;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let (wx, wy) = (drop.to.0 + hx * t, drop.to.1 + hy * t);
                // **Precipitation stops at the ground.** Without this every
                // streak is drawn straight down through the rock, so it
                // rains at the bottom of a mine -- reported from play as the
                // first thing wrong with it. The horizon cache this asks is
                // the same one that stopped sky being drawn behind dug rock,
                // which is the same question in the other direction: what
                // can the sky reach?
                if !self.under_sky(wx.round() as i32, wy.round() as i32) {
                    landed = Some((wx, wy));
                    break;
                }
                let (sx, sy) = project(wx, wy);
                // Brightest at the head and fading up the tail, so a streak
                // reads as moving rather than as a rigid dash.
                blend(frame, width, height, sx, sy, colour, drop.alpha * (0.35 + t * 0.65));
            }

            // What it looks like when it arrives. A drop that simply stopped
            // would read as rain being erased by the ground rather than
            // meeting it -- and the ethos here is that an effect with no
            // visible consequence is not finished. Snow settles instead of
            // splashing, so it gets none.
            if let (Some((wx, wy)), sky::Fall::Rain) = (landed, fall) {
                let spread = self.zoom.max(1);
                for dx in -spread..=spread {
                    let (sx, sy) = project(wx + dx as f32, wy - 1.0);
                    // Falls off from the centre, so a splash is a small burst
                    // and not a dash of the same width as the streak.
                    let fade = 1.0 - (dx.abs() as f32 / (spread + 1) as f32);
                    blend(frame, width, height, sx, sy, colour, drop.alpha * 0.85 * fade);
                }
            }
        }
    }

    /// A lightning strike: the flash over everything, then the bolt.
    ///
    /// The flash is a whole-frame lift rather than a light source, and that
    /// is a deliberate limit rather than an approximation of one. A real
    /// source would have to reach the light field, which oscillates on the
    /// day cycle and which everything from moss to phototropism makes
    /// decisions off -- a strike bright enough to see would read to a plant
    /// as an instant of noon. Lightning is allowed to be a thing you see and
    /// not a thing the world believes in.
    #[allow(clippy::too_many_arguments)]
    fn draw_lightning(
        &self,
        s: crate::sim::weather::Strike,
        (vx0, vy0): (i32, i32),
        (vx1, vy1): (i32, i32),
        frame: &mut [u8],
        width: u32,
        height: u32,
    ) {
        let project = |wx: f32, wy: f32| {
            (
                (wx.round() as i32 - self.camera_x) / self.zoom_out_stride * self.zoom,
                (wy.round() as i32 - self.camera_y) / self.zoom_out_stride * self.zoom,
            )
        };
        // Falls off with distance from the strike, so a bolt on the far side
        // of the world lights the sky rather than the room you are standing
        // in -- and so that walking toward a storm means something.
        let centre = (vx0 + vx1) / 2;
        let far = ((vx1 - vx0).max(1) * 3) as f32;
        let near = 1.0 - ((s.x - centre).abs() as f32 / far).clamp(0.0, 1.0);
        let lift = s.flash * near;
        if lift > 0.01 {
            for px in frame.chunks_exact_mut(4) {
                for c in px.iter_mut().take(3) {
                    // Toward white rather than a scaling, so already-bright
                    // sky still visibly flashes instead of clipping and
                    // sitting still.
                    *c = (*c as f32 + (255.0 - *c as f32) * lift * FLASH_LIFT) as u8;
                }
            }
        }

        // The bolt itself only while the first stroke is bright; the later
        // flicker is the sky lighting up with the channel already gone.
        if s.age > 4 {
            return;
        }
        let ground = (vy0..=vy1).find(|&y| !self.under_sky(s.x, y)).unwrap_or(vy1);
        let colour = sky::bolt_colour();
        let rgba = [colour[0] as u8, colour[1] as u8, colour[2] as u8, 255];
        for (a, b, weight) in sky::bolt(s.id, s.x, vy0, ground) {
            let (ax, ay) = project(a.0, a.1);
            let (bx, by) = project(b.0, b.1);
            let steps = (bx - ax).abs().max((by - ay).abs()).max(1);
            for i in 0..=steps {
                let (x, y) = (ax + (bx - ax) * i / steps, ay + (by - ay) * i / steps);
                let alpha = weight * s.flash.max(0.6);
                blend(frame, width, height, x, y, rgba, alpha);
                // The trunk gets a second pixel of width and a soft edge; a
                // one-pixel bolt is a hairline and reads as a scratch.
                if weight > 0.9 {
                    blend(frame, width, height, x + 1, y, rgba, alpha * 0.7);
                    blend(frame, width, height, x - 1, y, rgba, alpha * 0.35);
                }
            }
        }
    }

    /// The screen-space rect a world-space rect maps to at the current
    /// zoom/stride/camera, clipped to the visible frame — `None` when it
    /// falls entirely outside it. Reuses `world_to_screen`'s own per-point
    /// mapping on just the two corners (magnify: the far corner's block
    /// extends `zoom - 1` further; minify: `div_euclid` directly, same as
    /// `screen_to_world`'s own inverse) rather than a separate mapping
    /// scheme, so this can never disagree with the per-pixel loop about
    /// where a world position lands on screen.
    fn world_rect_to_screen_rect(&self, world_rect: Rect, width: u32, height: u32) -> Option<Rect> {
        let (sx0, sy0, sx1, sy1) = if self.zoom > 1 {
            (
                (world_rect.min_x - self.camera_x) * self.zoom,
                (world_rect.min_y - self.camera_y) * self.zoom,
                (world_rect.max_x - self.camera_x) * self.zoom + self.zoom - 1,
                (world_rect.max_y - self.camera_y) * self.zoom + self.zoom - 1,
            )
        } else {
            let stride = self.zoom_out_stride.max(1);
            (
                (world_rect.min_x - self.camera_x).div_euclid(stride),
                (world_rect.min_y - self.camera_y).div_euclid(stride),
                (world_rect.max_x - self.camera_x).div_euclid(stride),
                (world_rect.max_y - self.camera_y).div_euclid(stride),
            )
        };
        let screen_bounds = Rect::new(0, 0, width as i32 - 1, height as i32 - 1);
        Rect::new(sx0, sy0, sx1, sy1).intersection(screen_bounds)
    }

    /// Free particles are not CA cells, so the main per-pixel pass above
    /// never sees them — drawn here as a second, small pass instead. At
    /// `zoom > 1` each particle fills the same `zoom`x`zoom` screen block
    /// its position would round to for a CA cell, rather than staying a
    /// single screen pixel while everything around it magnifies — the
    /// otherwise-jarring case being debris that visually shrinks to a dot
    /// against its own newly-oversized surroundings the instant zoom goes
    /// up. No interpolation between a particle's sub-cell position and the
    /// pixel grid beyond that; the CA cell it becomes on landing carries
    /// all the same visual weight bulk material has, so a free particle
    /// mid-flight not doing that too is not a loss worth more complexity.
    fn draw_particles(&self, world: &World, particles: &ParticleSystem, frame: &mut [u8], width: u32, height: u32) {
        for particle in particles.iter() {
            let Some((sx, sy)) = self.world_to_screen(particle.x.round() as i32, particle.y.round() as i32) else {
                continue;
            };
            let palette = &world.materials.get(particle.material).palette;
            let colour = palette[particle.shade as usize % palette.len()];
            let block = self.zoom.max(1);
            for dy in 0..block {
                for dx in 0..block {
                    put(frame, width, height, sx + dx, sy + dy, colour);
                }
            }
        }
    }

    /// M8 chunk bodies, drawn the same way free particles are and for the
    /// same reason: a body in flight has been lifted *out* of the CA grid
    /// (`rigid::try_promote_failing_region` erases its cells so a landing
    /// cannot duplicate them), so the per-cell pass above cannot see it. A
    /// falling chunk would otherwise simply vanish for the whole of its
    /// flight and reappear on landing.
    fn draw_chunk_bodies(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        let block = self.zoom.max(1);
        for body in &world.chunk_bodies {
            for cell in &body.cells {
                let (wx, wy) = body.cell_position(cell);
                let Some((sx, sy)) = self.world_to_screen(wx, wy) else {
                    continue;
                };
                let palette = &world.materials.get(cell.material).palette;
                let colour = palette[cell.shade as usize % palette.len()];
                for dy in 0..block {
                    for dx in 0..block {
                        put(frame, width, height, sx + dx, sy + dy, colour);
                    }
                }
            }
        }
    }

    /// The M9 character, drawn last of the off-grid passes so he reads in
    /// front of debris. A hardcoded colored-cell sprite (`GNOME_SPRITE`),
    /// which is enough to judge movement feel; a real sprite with
    /// animation frames is later polish.
    fn draw_player(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        let Some(player) = &world.player else { return };
        let (ox, oy) = player.rect_origin();
        let block = self.zoom.max(1);
        let table = if player.action > 0 { &GNOME_SWING } else { &GNOME_SPRITE };
        // **One side per formation, decided once, not one side per column.**
        //
        // The first version keyed the scenery branch below on `wx` -- the
        // world column -- because a formation had no id to key on and a
        // stalactite was one cell wide, where per-column and per-formation
        // are the same thing. They are not the same thing the moment a
        // formation is three cells wide: `in_front`'s hash is deliberately
        // decorrelated between adjacent keys, so a run of columns never
        // agrees. Measured over 4000 columns, the fraction of a w-wide
        // formation that lands wholly on one side of him:
        //
        //   w=1 100%   w=2 48%   w=3 0%   w=5 0%   w=8 0%   w=12 0%
        //
        // **Zero, from width three up.** Every formation the round-6 respec
        // makes (3-8 cells at the base, tapering) would have sliced him into
        // vertical stripes -- half his columns drawn, half not -- and the
        // one-pixel speleothems that hid it are exactly the bug the respec
        // was fixing. A constant that was only ever right because something
        // else was broken.
        //
        // So the key is the leftmost scenery column his box overlaps, found
        // in one pass before the sprite loop: he is wholly in front of, or
        // wholly behind, whatever he is standing in. Two formations he
        // straddles put him on one side for both, which is right -- the
        // question the picture asks is where *he* is, not where each stone
        // is. Costs one extra `World::get` per sprite cell in the worst
        // case (98), on the same rectangle the loop already walks, and it
        // stays a pure function of world state so the dirty-rect skip's
        // pixel-identity still holds.
        let scenery_key = {
            let (w, h) = (table[0].len() as i32, table.len() as i32);
            (ox..ox + w)
                .find(|&wx| {
                    (oy..oy + h).any(|wy| world.materials.get(world.get(wx, wy).material).scenery)
                })
                .unwrap_or(ox)
        };
        for (dy, row) in table.iter().enumerate() {
            for (dx, colour) in row.iter().enumerate() {
                let Some(colour) = colour else { continue };
                // Mirrored on the *sprite* column, so the world rectangle
                // he occupies is unchanged whichever way he faces.
                let column = if player.facing_left { row.len() - 1 - dx } else { dx };
                let (wx, wy) = (ox + column as i32, oy + dy as i32);
                let Some((sx, sy)) = self.world_to_screen(wx, wy) else {
                    continue;
                };
                // Is a tree standing between him and the camera here?
                //
                // Done inside this loop rather than as a `draw_foreground`
                // pass after it, and the cost is the argument: this loop
                // already resolves a world coordinate per sprite pixel, so
                // occluding here is at most 98 `World::get` calls a frame
                // (7x14). A separate pass would have to re-run `cell_colour`
                // over every candidate position to repaint trees *over* the
                // sprite -- 163,840 at 512x320 -- for the same picture.
                let cell = world.get(wx, wy);
                let material = world.materials.get(cell.material);
                // A tree or a cave formation standing between him and the
                // camera. Both are walk-through, so both must be able to
                // pass in front of him -- a stalagmite he strolls through
                // *and* is always drawn over reads as a decal on the
                // foreground, which is worse than either extreme.
                let occluded = if material.scenery {
                    self.tree_depth.in_front(scenery_key as u32)
                } else {
                    cell.organism_id() != 0
                        && material.climbable
                        && self.tree_depth.in_front(cell.organism_id() as u32)
                };
                // Nothing is drawn where a tree covers him: the world's
                // own pixels, already painted by the cell pass, are what
                // shows. `OCCLUDED_ALPHA` above records the ghost this
                // replaced and how to get it back.
                if occluded && OCCLUDED_ALPHA <= 0.0 {
                    continue;
                }
                for by in 0..block {
                    for bx in 0..block {
                        match occluded {
                            true => blend(frame, width, height, sx + bx, sy + by, *colour, OCCLUDED_ALPHA),
                            false => put(frame, width, height, sx + bx, sy + by, *colour),
                        }
                    }
                }
            }
        }
    }

    /// Cache this world's frozen ground surface, so `sky_depth` can answer
    /// without a `World` in hand — `draw_lightning` and the drawn
    /// precipitation both ask it and neither has one.
    ///
    /// **There is nothing to re-find.** `World::sky_surface` is written once
    /// and never revised, so this fills in the first time it is called and
    /// then returns immediately — which is why it no longer takes the
    /// touched-chunk set it used to rescan from. What that replaced was a
    /// scan of every column inside a touched chunk *plus* a repair pass over
    /// a six-column neighbourhood, on every frame anything moved.
    ///
    /// **The repair pass is gone with it, and that is the substance of the
    /// change rather than a tidy-up.** `rebuild_sky_floor` existed for one
    /// reason — a dug shaft dropped its column's skyline and the sky came
    /// down the hole with the pick — and it worked by assuming any column
    /// standing lower than the ground on both sides of it was a hole to be
    /// filled back in. That is a guess about intent read off a shape, and it
    /// was wrong in both directions at once. It could not rescue a shaft
    /// wider than twice its reach: measured, a 13-cell shaft let daylight 35
    /// rows down into a mountain where a 12-cell one stayed a tunnel, and
    /// widening a shaft is exactly what mining is. And it never applied to
    /// the opposite case at all, so one cell left in the air still put a
    /// column of cave beneath it, as did a plank of any width from one to
    /// fifty. A surface that cannot move needs no repair, and asks no
    /// question that geometry has to answer.
    ///
    /// The fallback covers a world nothing has ever stepped — a few tests,
    /// and nothing else, since `World::begin_step` freezes on the first
    /// frame. Reading the world as it stands is the same answer for a world
    /// that has never run.
    fn rebuild_horizon(&mut self, world: &World) {
        let Some(b) = world.bounds() else {
            self.horizon.clear();
            self.underground.clear();
            self.underground_rect = None;
            return;
        };
        // The per-cell map, on the same terms as the skyline copy below:
        // copied rather than cached on a shape check, because `App::reset`
        // keeps the `Renderer` and builds a new `World`, and a cache keyed
        // on anything the two worlds share would hold the previous terrain's
        // answer over fresh ground for the rest of the session. 164 KB of
        // memcpy at 2048x640 against a draw that touches every pixel and
        // measures ~11 ms; worst-frame timing before and after is in the
        // commit message.
        self.underground.clear();
        self.underground.extend_from_slice(world.underground_map());
        if self.underground.is_empty() {
            // Fallback, for a world nothing has ever stepped -- the same
            // case `rebuild_horizon`'s own column-scan fallback below
            // covers, and it has to be covered here too or `start=0` in
            // every harness renders the artifact this fix removes and
            // reports it as still present.
            //
            // Recomputed rather than cached on a shape check, deliberately:
            // a cache keyed on bounds is exactly what the memcpy note above
            // rejects, since `App::reset` reuses the `Renderer`, and the
            // cost is bounded to a world that has never run -- one draw in
            // the app, and a handful of tests. It stays a pure function of
            // the world, so a renderer with history and a fresh one still
            // agree, which is what `dirty_rect_skip_is_pixel_identical_to_a_
            // full_redraw` requires.
            self.underground = underground_from_scratch(world, b);
        }
        self.underground_rect = if self.underground.is_empty() { None } else { Some(b) };
        // **Copied every frame, not cached on a size check.** `App::reset`
        // builds a whole new `World` and keeps the same `Renderer`, so a
        // cache keyed on "same width, same origin" would hold the *previous*
        // terrain's skyline over freshly generated ground for the rest of
        // the session. The copy is a memcpy of one `i32` per column -- 8 KB
        // at 2048 wide -- against a draw that touches every pixel.
        if !world.sky_surface().is_empty() {
            self.horizon.clear();
            self.horizon.extend_from_slice(world.sky_surface());
            self.horizon_origin = b.min_x;
            self.rebuild_light_datum();
            return;
        }
        // Fallback, for a world nothing has ever stepped. That is a handful
        // of tests and the app's very first draw if it lands before the
        // first update; a world that has never run cannot have been dug, so
        // reading it as it stands gives the same answer the freeze will.
        // Kept on the size check because this one is a full column scan.
        let width = b.width() as usize;
        if self.horizon.len() == width && self.horizon_origin == b.min_x {
            return;
        }
        self.horizon = (0..width as i32)
            .map(|i| {
                // Same definition of ground as `World::freeze_sky_surface`,
                // which has the reasoning for each exclusion.
                let x = b.min_x + i;
                (b.min_y..=b.max_y)
                    .find(|&y| {
                        matches!(
                            world.materials.kind(world.get(x, y).material),
                            crate::sim::material::MaterialKind::Solid | crate::sim::material::MaterialKind::Powder
                        )
                    })
                    .unwrap_or(i32::MAX)
            })
            .collect();
        self.horizon_origin = b.min_x;
        self.rebuild_light_datum();
    }

    /// `light_datum` from `horizon`: a grayscale morphological opening —
    /// min-filter then max-filter, both over the shoulder window — which
    /// clips any dip narrower than the window to its shoulders' level and
    /// passes everything wider through exactly unchanged (see
    /// `DEPTH_LIGHT_SHOULDER_REACH` for why an opening and not a blur).
    ///
    /// Naive windowed filters, not sliding-window minima: the window is 19
    /// wide and the skyline 2048 columns, so this is ~80k comparisons per
    /// rebuild against a draw that touches every pixel — the same budget
    /// argument `rebuild_horizon`'s own memcpy note makes. Columns that have
    /// never held ground (`i32::MAX`) stay `MAX` in the datum, so they keep
    /// reading as bottomless sky rather than borrowing a neighbour's depth.
    fn rebuild_light_datum(&mut self) {
        let n = self.horizon.len();
        self.light_datum.clear();
        self.light_datum.resize(n, i32::MAX);
        if n == 0 {
            return;
        }
        let r = DEPTH_LIGHT_SHOULDER_REACH;
        // Erode: highest ground (smallest y) in each window.
        let eroded: Vec<i32> = (0..n)
            .map(|i| (i.saturating_sub(r)..=(i + r).min(n - 1)).map(|j| self.horizon[j]).min().unwrap_or(i32::MAX))
            .collect();
        // Dilate back: lowest of the eroded values in each window. Together
        // the two passes are an opening, `datum[i] <= horizon[i]` always —
        // the datum only ever *raises* ground toward its shoulders, never
        // sinks it, so no column can get brighter than the raw skyline gave
        // it.
        for (i, slot) in self.light_datum.iter_mut().enumerate() {
            if self.horizon[i] == i32::MAX {
                continue;
            }
            let d = (i.saturating_sub(r)..=(i + r).min(n - 1)).map(|j| eroded[j]).max().unwrap_or(i32::MAX);
            *slot = d.min(self.horizon[i]);
        }
    }

    /// `y` relative to this column's *light* datum — the notch-clipped
    /// skyline. Same sign convention as `sky_depth`; only the terrain depth
    /// light reads this one.
    fn light_depth(&self, x: i32, y: i32) -> i32 {
        let i = x - self.horizon_origin;
        if i < 0 || i as usize >= self.light_datum.len() {
            return -1;
        }
        let floor = *self.light_datum.get(i as usize).unwrap_or(&i32::MAX);
        if floor == i32::MAX {
            return -1;
        }
        y - floor
    }

    /// Whether this position is open to the sky, rather than a void inside
    /// the ground.
    ///
    /// Asks what the world **was** when it was made, not whether there is
    /// currently a clear path up. The two differ exactly down a freshly dug
    /// shaft, and the second answer is the one that renders a mine as a
    /// strip of sky.
    ///
    /// **Per cell, off `World::underground_map`.** This used to be
    /// `sky_depth(x, y) < 0` — the per-*column* form — and that is what drew
    /// the dark bands: a column test says "there is rock above me" for the
    /// open air outside a cliff brow and under anything standing in the sky
    /// at genesis, which is a cave roof and a lip told apart by nothing.
    /// `sky_depth` is still exactly right for *how deep*, which is why it
    /// stays and why the two are now separate questions
    /// (`Reports/dark-bands-diagnosis.md`).
    ///
    /// Falls back to the column form outside the map's rect and on a world
    /// that has never been stepped — the same fallback `World::is_outdoors`
    /// makes, and the same handful of tests reach it.
    fn under_sky(&self, x: i32, y: i32) -> bool {
        if let Some(b) = self.underground_rect {
            if x >= b.min_x && x <= b.max_x && y >= b.min_y && y <= b.max_y {
                let i = (y - b.min_y) as usize * b.width() as usize + (x - b.min_x) as usize;
                return self.underground[i >> 6] & (1 << (i & 63)) == 0;
            }
        }
        self.sky_depth(x, y) < 0
    }

    /// `y` relative to this column's sky floor: **negative is open sky**,
    /// `0` is the floor row itself, and larger numbers are further inside.
    ///
    /// The sign convention matters — `under_sky` used to be written as
    /// `y < floor` directly, and expressing it as `depth <= 0` instead
    /// silently moved the boundary one row, which `draw_lightning`'s
    /// ground-finding scan reads. It is `< 0` for that reason.
    ///
    /// `-1` rather than a computed value for a column outside the cached
    /// range or one that has never held anything: both mean open sky, and
    /// `y - i32::MAX` would wrap.
    /// What this position would look like with nothing in it — sky above
    /// the skyline, unlit rock below it.
    ///
    /// Two callers: the empty-cell path, which is what it was extracted
    /// from, and `GasMode::Translucent`, which needs to know what is
    /// *behind* a gas cell in order to see through it. Cheap enough for the
    /// second one: a `Vec` index (`sky_depth`), an array index into
    /// `cave_ramp` and three mixes, with no `World::get` anywhere -- which
    /// is what the fake-AO experiment could not say for itself.
    /// The colour behind a cell: sky above the frozen skyline, faded cave
    /// dark below it, and the void reveal when `F11` is on.
    ///
    /// Takes `world` for the glow lookup alone. Two copies of this existed
    /// when the water-cycle branch met master -- an extracted one here and
    /// an inline one in `cell_colour` -- and they had already drifted by a
    /// `reveal_voids` branch and the glow tint. That is how a translucent
    /// cloud ends up composited over a background nobody is drawing, so
    /// there is one copy now and both callers take it.
    fn background_at(&self, world: &World, x: i32, y: i32) -> [u8; 4] {
        // Sky, not a flat background. Empty space inside the world is
        // *air*, and drawing it black made every world read as a cutaway
        // diagram rather than a place -- the strongest remaining tell,
        // once the ground itself started looking like ground.
        //
        // Empty space is also where the day/night cycle was invisible.
        // The oscillator has driven the light channel since M16, so
        // plants and moss have always known what time it is while the
        // screen did not; this is that same cosine, read rather than
        // reinvented (`sky::Sky::at`).
        let daylight = self.sky.colour_at(x, y);
        // **Two different questions, and they used to be one.** Whether this
        // is sky comes from the per-cell map (`under_sky`); how dark it
        // draws comes from the column depth below. Asking the second one for
        // both is what put a cave under every overhanging lip.
        if self.under_sky(x, y) {
            return daylight;
        }
        // Non-negative whenever `under_sky` is false, by construction: both
        // ways of being marked underground -- it was ground, or it was air
        // with ground above it -- put this column's topmost ground at or
        // above `y`. The clamp is the ramp's own domain, not a guard.
        let depth = self.sky_depth(x, y);
        if self.reveal_voids {
            // `F11`: every enclosed void marked, however deep -- see the
            // field doc. Flat, not depth-faded: the point is that a vault
            // 280 rows down reads exactly as loudly as a shallow tunnel.
            return REVEAL_VOID;
        }
        // Inside the ground: unlit rock, not daylight. Constant rather than
        // following the sky, because a cave is dark at noon as well -- and
        // deliberately distinct from the `VOID` outside the world, which is
        // a different kind of nothing.
        //
        // Faded in over `CAVE_FADE_DEPTH` rather than switched at the
        // boundary. Light reaches in through an opening and falls off;
        // cutting instead put a black rectangle behind every roof and made
        // a cave mouth a cutout rather than an opening.
        let t = self.cave_ramp[depth.clamp(1, CAVE_FADE_DEPTH) as usize] as u16;
        let mix = |a: u8, b: u8| ((a as u16 * (255 - t) + b as u16 * t) / 255) as u8;
        let mut air = [
            mix(daylight[0], UNDERGROUND[0]),
            mix(daylight[1], UNDERGROUND[1]),
            mix(daylight[2], UNDERGROUND[2]),
            255,
        ];
        // Glow-lit cave air: the void near a glowing lining blends toward
        // `GLOW_AIR_TINT` by the local light level, so a breached chamber
        // holds a soft pool of warm light instead of the flat dark every
        // other cavity gets. On top of the cave fade, not instead of it --
        // the fade is the darkness this light is seen against.
        let glow = self.glow_at(world, x, y);
        if glow > 0.0 {
            for (c, tint) in air.iter_mut().zip(GLOW_AIR_TINT) {
                *c = (*c as f32 + (tint as f32 - *c as f32) * glow).round() as u8;
            }
        }
        air
    }

    fn sky_depth(&self, x: i32, y: i32) -> i32 {
        let i = x - self.horizon_origin;
        if i < 0 || i as usize >= self.horizon.len() {
            return -1;
        }
        let floor = *self.horizon.get(i as usize).unwrap_or(&i32::MAX);
        if floor == i32::MAX {
            return -1;
        }
        y - floor
    }

    /// Local (glow) light at a world position, `0.0..=1.0` of
    /// `field::MAX_LIGHT` — and `0.0` after a single set-emptiness check for
    /// the overwhelming majority of pixels no glow tile covers, which is what
    /// keeps this affordable inside the per-pixel colour path. The bilinear
    /// field read happens only inside `glow_tiles` (a glowing tile and its
    /// ring of neighbours), and those exist only in worlds that generated a
    /// geode.
    ///
    /// The light channel also carries daylight and fire, but `glow_tiles`
    /// gates this read to the deep tiles around a glowing material, where the
    /// standing level is the seeded glow floor plus its diffused halo — the
    /// quantity this exists to draw.
    fn glow_at(&self, world: &World, x: i32, y: i32) -> f32 {
        if self.glow_tiles.is_empty() || !self.glow_tiles.contains(&ChunkCoord::containing(x, y)) {
            return 0.0;
        }
        let coarse =
            (world.field_at_bilinear(x as f32, y as f32).light / crate::sim::field::MAX_LIGHT).clamp(0.0, 1.0);
        // **The near term only sharpens light that is already there.** It is
        // splatted as plain discs and knows nothing about rock, so on its own
        // it would shine a crystal through a wall. The coarse field does
        // respect blocking, so gating on it keeps that property and confines
        // the near term to exactly the region whose *shape* is wrong -- the
        // blocky lit rectangle -- rather than letting it invent lit ground of
        // its own. `max`, not a sum: the near term is the same light seen at
        // a resolution the field cannot hold, not a second lamp.
        if coarse <= 0.0 || self.glow_shape == GlowShape::Field {
            return coarse;
        }
        coarse.max(self.near_glow_at(x, y))
    }

    fn near_glow_at(&self, x: i32, y: i32) -> f32 {
        let coord = ChunkCoord::containing(x, y);
        let Some(buf) = self.near_glow.get(&coord) else { return 0.0 };
        let (lx, ly) = (x - coord.x * CHUNK_SIZE, y - coord.y * CHUNK_SIZE);
        buf[(ly * CHUNK_SIZE + lx) as usize]
    }

    /// Splat every glowing *cell* into a per-chunk, per-cell buffer.
    ///
    /// Rebuilt from the world rather than read from the light field because
    /// the field cannot answer the question: `set_glow_local` writes one
    /// value per `FIELD_SCALE`x`FIELD_SCALE` block, so by the time light is
    /// in the field a two-cell crystal has already become an eight-cell
    /// square. This is the "short-range term computed from the emitting
    /// cells themselves" that `Reports/open-bugs-handoff.md` 0c asks for,
    /// with the coarse field left carrying the far falloff.
    ///
    /// Cost is paid per rebuild, not per pixel: scanning a glowing chunk is
    /// `CHUNK_SIZE^2` cell reads and each emitter writes one disc, against a
    /// single array index in `near_glow_at`. Rebuilds happen only when the
    /// glow tiles change or are still converging -- the same condition that
    /// already forces a full redraw -- so a settled world with a geode in it
    /// pays nothing per frame.
    fn rebuild_near_glow(&mut self, world: &World, emitter_tiles: &[ChunkCoord]) {
        self.near_glow.clear();
        if emitter_tiles.is_empty() {
            return;
        }
        self.near_glow_rebuilds += 1;
        let area = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        for &tile in emitter_tiles {
            for ly in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let (wx, wy) = (tile.x * CHUNK_SIZE + lx, tile.y * CHUNK_SIZE + ly);
                    if !world.in_bounds(wx, wy) {
                        continue;
                    }
                    let glow = world.materials.get(world.get(wx, wy).material).glow;
                    if glow <= 0.0 {
                        continue;
                    }
                    for dy in -NEAR_GLOW_RADIUS..=NEAR_GLOW_RADIUS {
                        for dx in -NEAR_GLOW_RADIUS..=NEAR_GLOW_RADIUS {
                            let d2 = (dx * dx + dy * dy) as f32;
                            let r = NEAR_GLOW_RADIUS as f32;
                            if d2 > r * r {
                                continue;
                            }
                            // Squared linear falloff: full at the emitter,
                            // zero at the radius, with the knee near the
                            // source where the eye reads a light's shape.
                            let t = 1.0 - d2.sqrt() / r;
                            let v = t * t;
                            let (nx, ny) = (wx + dx, wy + dy);
                            let coord = ChunkCoord::containing(nx, ny);
                            if !self.glow_tiles.contains(&coord) {
                                continue;
                            }
                            let buf = self.near_glow.entry(coord).or_insert_with(|| vec![0.0; area]);
                            let (bx, by) = (nx - coord.x * CHUNK_SIZE, ny - coord.y * CHUNK_SIZE);
                            let slot = &mut buf[(by * CHUNK_SIZE + bx) as usize];
                            *slot = slot.max(v);
                        }
                    }
                }
            }
        }
    }

    fn cell_colour(&self, world: &World, x: i32, y: i32) -> [u8; 4] {
        if !world.in_bounds(x, y) {
            return VOID;
        }
        let cell = world.get(x, y);
        let palette = &world.materials.get(cell.material).palette;
        // Modulo keeps any shade value valid, so a palette can shrink on hot
        // reload in M3 without invalidating cells already in the world.
        let mut base = palette[cell.shade as usize % palette.len()];
        // Fractured rock draws dark along the break. Cracks are edge state
        // (`FLAG_CRACK_RIGHT`), and at 1:1 an edge has no pixels of its own
        // to draw into -- so the *cell* owning the crack is darkened
        // instead, which renders a fissure as a dark seam threading through
        // the rock at any zoom. Without this the entire mechanic is
        // invisible: rock would weaken, sag and eventually drop with nothing
        // on screen ever having looked damaged.
        if cell.cracked() {
            base = [
                (base[0] as u16 * CRACK_DARKEN / 256) as u8,
                (base[1] as u16 * CRACK_DARKEN / 256) as u8,
                (base[2] as u16 * CRACK_DARKEN / 256) as u8,
                base[3],
            ];
        }
        // Wet ground is dark ground. Saturation already existed on every
        // powder that can hold water -- infiltration writes it, roots read
        // it -- and until now it was drawn *only* by a debug overlay, so a
        // rainstorm could soak a hillside with nothing on screen changing at
        // all. That is the same failure as invisible cracks above, and the
        // same one-line shape of fix.
        //
        // Darkening rather than tinting blue: damp soil is the same soil,
        // and a blue cast reads as a different material rather than as the
        // same one being wet.
        // A tree he passes *behind* is dimmed, so the stand reads as two
        // layers rather than as random occlusion. `Haze` only — the plain
        // `Weave` leaves every tree its own colour, and which of the two
        // looks right is the question the selector exists to answer.
        //
        // Gated on `organism_id` first, which is a field of the `Cell`
        // already in hand, so a world with no organisms in it pays one
        // compare per non-empty pixel and nothing else.
        if self.tree_depth == TreeDepth::Haze && cell.organism_id() != 0 && !self.tree_depth.in_front(cell.organism_id() as u32) {
            base = [
                (base[0] as u16 * HAZE_DIM / 256) as u8,
                (base[1] as u16 * HAZE_DIM / 256) as u8,
                (base[2] as u16 * HAZE_DIM / 256) as u8,
                base[3],
            ];
        }
        let saturation = crate::sim::update::soil_moisture(cell);
        if saturation > 0 {
            let t = saturation as u32 * (256 - DAMP_DARKEN) / material::SOIL_SATURATED.max(1) as u32;
            let scale = (256 - t) as u16;
            base = [
                (base[0] as u16 * scale / 256) as u8,
                (base[1] as u16 * scale / 256) as u8,
                (base[2] as u16 * scale / 256) as u8,
                base[3],
            ];
        }
        // A raw material check, not `cell.is_empty()` -- a promoted liquid
        // body's container cell (`Reports/liquid-heightfield-design.md`
        // §3c) is materially empty but `FLAG_MANAGED`, and `is_empty()`'s
        // now-managed-aware meaning ("available to use") isn't the question
        // rendering is asking ("what does this position actually look
        // like"). Using `is_empty()` here would draw a container cell with
        // grain jitter/heat-glow instead of flat background -- a visible,
        // static artifact along the outline of every heightfield body.
        if cell.material == material::EMPTY {
            // Sky, not a flat background. Empty space inside the world is
            // *air*, and drawing it black made every world read as a cutaway
            // diagram rather than a place -- the strongest remaining tell,
            // once the ground itself started looking like ground.
            //
            // Empty space is also where the day/night cycle was invisible.
            // The oscillator has driven the light channel since M16, so
            // plants and moss have always known what time it is while the
            // screen did not; this is that same cosine, read rather than
            // reinvented (`sky::Sky::at`).
            //
            // Deliberately *inside* the bounds check above, so the void
            // outside the world keeps its own colour and stays
            // distinguishable from a dark night sky.
            base = self.background_at(world, x, y);
            // Still route through the field overlay below (a field reading
            // exists over empty space same as anywhere else -- pressure and
            // temperature very much propagate through vacuum) rather than
            // returning here the way the pre-overlay code always did.
            return self.apply_field_overlay(world, x, y, base);
        }
        let mut rgb = [base[0], base[1], base[2]];

        // Partial fill, drawn. A `Liquid` cell holds a continuous amount
        // (`Cell::aux`, see `update.rs`'s module doc) and until now none of
        // it reached the screen: a cell holding 2% drew identically to one
        // holding 100%. That made the entire compressible-volume model
        // unfalsifiable by eye -- three sessions of liquid work measured
        // fill while the display showed only occupancy, so "the pool
        // levelled" and "the pool looks exactly the same" were both true at
        // once, and every fix was judged against numbers the renderer never
        // drew. It also made the "ballooning" failure mode (mass spread
        // thin across many cells) read as water *multiplying* rather than
        // as water thinning out, which is what got two otherwise-promising
        // fixes reverted.
        //
        // Dimming toward the empty-cell colour (black) is the cheapest
        // honest representation and needs no new per-pixel lookups. Clamped
        // at 1.0 so a compressed (over-full) cell doesn't draw *brighter*
        // than a full one -- depth should not glow.
        if world.materials.kind(cell.material) == material::MaterialKind::Liquid {
            let fill = crate::sim::update::liquid_fill(cell) as f32 / material::LIQUID_FULL as f32;
            // Per-material now (`Material::fill_dimming`) rather than the
            // hardcoded floor this used, because how conspicuous a partial
            // cell should be is a look judgement -- and it is the one that
            // decides whether a settled waterline reads as a clean edge or a
            // mottled band. See that field's own doc for the measurements.
            let dimming = world.materials.get(cell.material).fill_dimming.clamp(0.0, 1.0);
            let strength = (1.0 - dimming) + dimming * fill.clamp(0.0, 1.0);
            for c in &mut rgb {
                *c = (*c as f32 * strength).round() as u8;
            }
        }

        // Foam: moving water blends toward pale — see `FOAM_BLEND`. Before
        // the grain so the jitter textures the foam too, after the fill
        // dimming so a thin flying sheet is pale *and* faint rather than
        // opaque white.
        if world.materials.kind(cell.material) == material::MaterialKind::Liquid && cell.flowing() {
            for (c, foam) in rgb.iter_mut().zip(FOAM_TINT) {
                *c = (*c as f32 + (foam - *c as f32) * FOAM_BLEND).round() as u8;
            }
        }

        // Grain: a stable per-position brightness jitter — see the module
        // constant doc for why this is the Sandspiel trick, not dithering.
        // Integer math, not float: this runs on every visible non-empty
        // pixel every frame (rendering has no dirty-rect equivalent — it
        // redraws the whole screen regardless of what's settled), so it
        // has to earn its cost the same way M14 learned fire's per-cell
        // pass did at CA-sweep scale.
        //
        // `GrainMode` only ever diverts the `Liquid` path — every other kind
        // keeps the position-keyed grain unconditionally, because for a
        // settled pile that is the correct answer and not a compromise.
        let is_liquid = world.materials.kind(cell.material) == material::MaterialKind::Liquid;
        let (grain, strength) = match (is_liquid, self.grain) {
            (true, GrainMode::Cell) => (rng::jitter_u8(cell.shade), JITTER_STRENGTH),
            (true, GrainMode::Muted) => (rng::jitter(x, y), JITTER_STRENGTH / 3.0),
            (true, GrainMode::Animated) => {
                (rng::jitter3(x, y, (self.frame / GRAIN_ANIMATION_PERIOD) as i32), JITTER_STRENGTH)
            }
            (true, GrainMode::AnimatedMuted) => {
                (rng::jitter3(x, y, (self.frame / GRAIN_ANIMATION_PERIOD) as i32), JITTER_STRENGTH / 3.0)
            }
            (true, GrainMode::AnimatedSmooth) => {
                // Lerp across the bucket boundary rather than stepping over
                // it: a hard change every N frames is what reads as flicker,
                // and the same noise drifted smoothly reads as motion.
                let bucket = self.frame / GRAIN_SMOOTH_PERIOD;
                let t = (self.frame % GRAIN_SMOOTH_PERIOD) as f32 / GRAIN_SMOOTH_PERIOD as f32;
                let a = rng::jitter3(x, y, bucket as i32);
                let b = rng::jitter3(x, y, bucket as i32 + 1);
                (a + (b - a) * t, JITTER_STRENGTH / 3.0)
            }
            _ => (rng::jitter(x, y), JITTER_STRENGTH),
        };
        let jitter_permille = ((grain - 0.5) * 2000.0 * strength) as i32;
        for c in &mut rgb {
            *c = (*c as i32 + (*c as i32 * jitter_permille) / 1000).clamp(0, 255) as u8;
        }
        if is_liquid && self.grain == GrainMode::Motion && cell.flowing() {
            for c in &mut rgb {
                *c = (*c as f32 * (1.0 + MOTION_GRAIN_LIFT)).round().clamp(0.0, 255.0) as u8;
            }
        }

        // Heat glow: continuous with temperature, not just a flat tint for
        // `is_burning()` — a cell cooling down after a fire, or sitting next
        // to one, should visibly register as warm even once the flame
        // itself is out. Burning cells are floored at the same 0.6 blend
        // the flat version used, so active fire never looks weaker than it
        // did before; non-burning warmth caps at half that, since ambient
        // heat is not the same visual statement as active flame.
        //
        // Early exit for cells already at ambient and not burning — which
        // is nearly every cell, nearly always — for the same reason: the
        // full blend computation costs nothing turned off, and turned on
        // unconditionally it was measured to nearly triple this function's
        // cost across a full 512x320 stress scene even though almost every
        // one of those calls would have computed `t == 0.0` and changed
        // nothing.
        if cell.is_burning() || cell.temperature() != AMBIENT_TEMPERATURE {
            let heat_ratio = ((cell.temperature() as f32 - AMBIENT_TEMPERATURE as f32) / HEAT_GLOW_RANGE).clamp(0.0, 1.0);
            let mut t = if cell.is_burning() { heat_ratio.max(0.6) } else { heat_ratio * 0.5 };
            // Flicker only for actively burning cells — a passively cooling
            // ember shouldn't dance the way a live flame does.
            if cell.is_burning() {
                let bucket = (world.frame / FLAME_FLICKER_PERIOD) as i32;
                let flicker = 1.0 + (rng::jitter3(x, y, bucket) - 0.5) * 2.0 * FLAME_FLICKER_STRENGTH;
                t = (t * flicker).clamp(0.0, 1.0);
            }
            if t > 0.0 {
                let fire = [
                    FIRE_TINT_LOW[0] + (FIRE_TINT_HIGH[0] - FIRE_TINT_LOW[0]) * heat_ratio,
                    FIRE_TINT_LOW[1] + (FIRE_TINT_HIGH[1] - FIRE_TINT_LOW[1]) * heat_ratio,
                    FIRE_TINT_LOW[2] + (FIRE_TINT_HIGH[2] - FIRE_TINT_LOW[2]) * heat_ratio,
                ];
                for (c, fire) in rgb.iter_mut().zip(fire) {
                    *c = (*c as f32 + (fire - *c as f32) * t).round() as u8;
                }
            }

            // Bubbles, inside the same branch and after the tint, so a
            // pocket of vapour draws over the warm cast rather than under
            // it. Deliberately here and not in a branch of its own: this
            // one already costs a temperature read and already admits only
            // cells that are off ambient, which is the guard the whole
            // effect would otherwise need for itself. See `BubbleMode`.
            if self.bubbles != BubbleMode::Off && is_liquid {
                self.apply_bubbles(world, x, y, cell, &mut rgb);
            }
        }

        // Fake AO (darkening a cell by how enclosed it is) was tried here
        // too and measured to cost more than jitter and heat combined —
        // ~10ms on the 512x320 stress scene alone, from the 4 extra
        // `World::get` calls per pixel, on top of rendering's own per-pixel
        // lookup. Unlike the CA sweep, rendering has no dirty-rect skip: it
        // redraws every visible pixel every frame regardless of what
        // settled, so a densely-filled *static* world pays this cost
        // forever, not just as a stress-test edge case. Cutting rather than
        // shipping it over budget — see `PLAN.md`'s M19 section for the
        // real fix this needs (reusing a chunk's direct array access
        // instead of a `World::get` HashMap lookup per neighbour, the same
        // lesson M5's `ChunkView` already applied to the sweep) before it's
        // safe to turn back on.

        // Lit last, so everything above — fill dimming, grain, heat glow —
        // is lit rather than competing with the light.
        //
        // Fire looks after itself without a special case: a burning cell
        // floods its own field block with light, so its level comes out at
        // the top of the range and the darkening barely touches it. The one
        // thing that emits is the one thing that stays bright, which is the
        // behaviour a special case would have had to fake.
        let mut rgb = sky::apply_light(rgb, self.daylight, self.sky.ambient());

        // Translucency, after the light and not before it: `background_at`
        // returns an already-lit colour, so blending first would light the
        // background twice and a cloud would read as a bright patch rather
        // than as something you can see through. See `GasMode`.
        if self.gas != GasMode::Opaque && world.materials.kind(cell.material) == material::MaterialKind::Gas {
            let alpha = match self.gas {
                GasMode::ByFill => {
                    let fill = crate::sim::update::liquid_fill(cell) as f32 / material::LIQUID_FULL as f32;
                    GAS_ALPHA_MIN + (GAS_ALPHA - GAS_ALPHA_MIN) * fill.clamp(0.0, 1.0)
                }
                _ => GAS_ALPHA,
            };
            let behind = self.background_at(world, x, y);
            for (c, b) in rgb.iter_mut().zip(behind) {
                *c = (*c as f32 * alpha + b as f32 * (1.0 - alpha)).round().clamp(0.0, 255.0) as u8;
            }
        }
        // Depth grade after the sky light, so the two compose: deep rock at
        // night is `UNLIT_FLOOR x DEPTH_LIGHT_FLOOR` of its palette, which
        // is what pulls the night cross-section back under the night sky's
        // brightness. Solid-only in effect, not by a kind test: everything
        // *above* the frozen skyline (trees, standing water in a hollow, a
        // placed block, the sky itself) has negative depth and is untouched,
        // and what sits below it is the ground this exists to shade. Water
        // and air inside the ground already have their own treatments (fill
        // dimming, the cave fade) and take the same grade on top, which is
        // deliberate — a flooded cave is deep first and wet second.
        let rgb = match self.terrain_light {
            TerrainLight::Off => rgb,
            TerrainLight::Depth => {
                // `under_sky` as well as the depth, because `light_datum`
                // is the *notch-clipped* skyline and clips only dips -- a
                // spike passes straight through, so a slab hanging in the
                // air at genesis gave every cell under it a large depth and
                // graded the open pond beneath it dark. That was the water
                // half of the same report (review card
                // `20260822T225340455Z-ad69f8`); the rock half is
                // `background_at` above. Solid ground is always marked
                // underground, so this cannot brighten rock.
                let depth = self.light_depth(x, y);
                if depth < 0 || self.under_sky(x, y) {
                    rgb
                } else {
                    let f = self.depth_light_ramp[depth.min(DEPTH_LIGHT_RAMP_ROWS) as usize] as u32;
                    [
                        ((rgb[0] as u32 * f) / 256).min(255) as u8,
                        ((rgb[1] as u32 * f) / 256).min(255) as u8,
                        ((rgb[2] as u32 * f) / 256).min(255) as u8,
                    ]
                }
            }
        };
        // Local light, after the sky and the depth grade deliberately: glow
        // is the one light that must win against depth, because a sealed
        // chamber sits exactly where the depth ramp is at its floor. A
        // multiplicative lift keeps the material's own hue — the wall of a
        // vug is brighter stone, not tinted crystal — and the diffused halo
        // in the field does the spatial falloff, so a wall two blocks from
        // the lining catches less than the cell it hangs over.
        let glow = self.glow_at(world, x, y);
        let rgb = if glow > 0.0 {
            let f = 1.0 + glow * GLOW_SOLID_LIFT;
            [
                (rgb[0] as f32 * f).min(255.0).round() as u8,
                (rgb[1] as f32 * f).min(255.0).round() as u8,
                (rgb[2] as f32 * f).min(255.0).round() as u8,
            ]
        } else {
            rgb
        };
        let tinted = self.apply_field_overlay(world, x, y, [rgb[0], rgb[1], rgb[2], 255]);
        // Applied *after* the field overlay, deliberately: the two can be on
        // at once (light and canopy density together is the pairing that
        // actually explains where a tip chose to grow), and when they are,
        // the per-cell channel is the more specific statement and should win
        // on the handful of cells it covers rather than being washed out by
        // a field tint covering the whole screen. Both sit on top of the sky
        // lighting: a debug channel must stay readable at midnight, and the
        // full-replace ramps below are exactly the channels that must not be
        // modulated by the time of day.
        self.apply_organism_overlay(world, x, y, tinted)
    }

    /// Blends `base` toward a ramp keyed on the selected organism channel,
    /// for organism-owned cells only — a no-op returning `base` unchanged
    /// when the overlay is off or the cell isn't organism tissue, so
    /// `cell_colour` can route through it unconditionally.
    ///
    /// **A raw `organism_id() != 0` test is what identifies organism
    /// tissue**, not `Cell::is_empty()` and not a `Plant`-kind check.
    /// `organism_id` is the tag `organism.rs` itself uses everywhere
    /// (`diffuse_resource`'s `is_wall`, `organism_tick`'s own guard), and a
    /// hand-painted `wood` cell is `Plant`-kind but owns no organism state
    /// — tinting it would draw a resource value that does not exist.
    ///
    /// **Scalar channels *replace* the cell colour rather than blending
    /// into it, and that is a correction to how this first shipped.** The
    /// first version copied `apply_field_overlay`'s magnitude-scaled blend
    /// exactly. On a tree that produced a canopy-density sheet which was
    /// blank to the eye, and the obvious reading — "the deposit isn't
    /// there" — was wrong. Wood's own base colour is brown, the ramp was
    /// red, and a mid-range reading moved the red byte from 139 to 155 and
    /// changed nothing else; the value was present and simply not visible
    /// against the material it was drawn over. The resource channel looked
    /// fine only because green happens to sit far from brown.
    ///
    /// That is the exact failure mode this whole overlay exists to prevent,
    /// reproduced inside the tool built to prevent it, so it is written
    /// down rather than quietly fixed: **a debug readout must not be a
    /// function of the thing it is debugging.** A full replace on a fixed
    /// dark→bright ramp reads the same over wood, leaf, soil or anything
    /// added later, and zero is unambiguously distinguishable from
    /// "present but low."
    ///
    /// `apply_field_overlay`'s blend stays correct for *its* job — it
    /// covers every pixel including empty space, where replacing would
    /// repaint the whole screen and hide the world entirely.
    ///
    /// `CellType` is categorical (there is no "less of a `RootTip`") and
    /// keeps a high flat blend, so a burning or dimmed cell still reads as
    /// such underneath the type colour.
    fn apply_organism_overlay(&self, world: &World, x: i32, y: i32, base: [u8; 4]) -> [u8; 4] {
        if self.organism_overlay == OrganismOverlay::Off {
            return base;
        }
        let cell = world.get(x, y);
        if self.organism_overlay == OrganismOverlay::SoilMoisture {
            if world.materials.kind(cell.material) != material::MaterialKind::Powder {
                return base;
            }
            let t = crate::sim::update::soil_moisture(cell) as f32 / material::SOIL_SATURATED as f32;
            let ramp = scalar_ramp(t.clamp(0.0, 1.0), SCALAR_RAMP_MOISTURE);
            let mut out = base;
            for (c, r) in out.iter_mut().take(3).zip(ramp) {
                *c = r.round().clamp(0.0, 255.0) as u8;
            }
            return out;
        }
        if cell.organism_id() == 0 {
            return base;
        }
        let cell_type = organism::cell_type(cell.aux());
        let (ramp, blend) = match self.organism_overlay {
            // Both handled above, before the organism-tissue guard: `Off`
            // returns immediately, and `SoilMoisture` asks about inert
            // `Powder` rather than organism cells.
            OrganismOverlay::Off | OrganismOverlay::SoilMoisture => return base,
            OrganismOverlay::CellType => {
                // An unrecognized type bit pattern is a real possibility
                // (`organism.rs`'s own `an_unrecognized_type_bit_pattern_
                // is_none` test), and it should be *visible* as wrong
                // rather than silently drawn as ordinary wood.
                let colour = match cell_type {
                    Some(organism::CellType::Seed) => CELL_TYPE_SEED,
                    Some(organism::CellType::GrowingTip) => CELL_TYPE_GROWING_TIP,
                    Some(organism::CellType::MatureBody) => CELL_TYPE_MATURE_BODY,
                    Some(organism::CellType::Leaf) => CELL_TYPE_LEAF,
                    Some(organism::CellType::RootTip) => CELL_TYPE_ROOT_TIP,
                    Some(organism::CellType::DormantBud) => CELL_TYPE_DORMANT_BUD,
                    Some(organism::CellType::Head) => CELL_TYPE_HEAD,
                    Some(organism::CellType::Segment) => CELL_TYPE_SEGMENT,
                    None => [255.0, 0.0, 0.0],
                };
                (colour, CELL_TYPE_BLEND)
            }
            OrganismOverlay::Resource => {
                let t = (world.carbon_at(x, y) / organism::RESOURCE_SCALE).clamp(0.0, 1.0);
                (scalar_ramp(t, SCALAR_RAMP_RESOURCE), 1.0)
            }
            OrganismOverlay::VeinConductance => {
                // Normalized across the *live* range rather than 0..max, so
                // undifferentiated tissue sits at the ramp floor instead of
                // a third of the way up it. A cell that has never carried
                // flux reads as unambiguously dark.
                let c = world
                    .organism_cell(x, y)
                    .map_or(organism::CONDUCTANCE_MIN, |cell| cell.carbon_conductance.iter().copied().fold(f32::MIN, f32::max));
                let span = organism::CONDUCTANCE_MAX - organism::CONDUCTANCE_MIN;
                let t = ((c - organism::CONDUCTANCE_MIN) / span).clamp(0.0, 1.0);
                (scalar_ramp(t, SCALAR_RAMP_VEIN), 1.0)
            }
            OrganismOverlay::CanopyDensity => {
                let t = (world.canopy_density_at(x, y) / organism::CANOPY_DENSITY_SCALE).clamp(0.0, 1.0);
                (scalar_ramp(t, SCALAR_RAMP_CANOPY), 1.0)
            }
        };
        let mut out = base;
        for (c, r) in out.iter_mut().take(3).zip(ramp) {
            *c = (*c as f32 + (r - *c as f32) * blend).round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    /// Draws vapour inside a boiling `Liquid` cell — see `BubbleMode`.
    ///
    /// **A pure function of `(x, y, frame)` and the cell's own temperature,
    /// with no stored state and no writes back into the sim**, the same
    /// shape the animated `GrainMode` variants use. That is not tidiness:
    /// bubbles as free particles were considered and are structurally
    /// impossible here, because `particle::advance_and_check_landing` lands
    /// a particle the instant its next substep is occupied — and every
    /// substep inside a pool is occupied, so a bubble particle would
    /// convert back into a cell on its first frame without ever rising.
    ///
    /// The pattern scrolls by indexing at `y + rise`: the element drawn at
    /// `y` this frame is the one that was at `y + 1` a moment ago, so the
    /// whole field appears to climb, and it costs one integer add.
    ///
    /// Sited on a **two-cell grid**, not per pixel, because the first thing
    /// that goes wrong with this effect is that it reads as texture rather
    /// than as bubbles — a one-pixel speckle at any density is grain, which
    /// this renderer already has a whole enum of. The block's upper half is
    /// its crown and blends further toward `BUBBLE_TINT` than its lower
    /// half, which is the cheapest thing that reads as domed.
    fn apply_bubbles(&self, world: &World, x: i32, y: i32, cell: Cell, rgb: &mut [u8; 3]) {
        let heat_to_boil =
            |t: i16| ((t as f32 - BUBBLE_MIN_TEMPERATURE) / (BUBBLE_FULL_TEMPERATURE - BUBBLE_MIN_TEMPERATURE)).clamp(0.0, 1.0);
        let mut boil = heat_to_boil(cell.temperature());
        if self.bubbles == BubbleMode::Surface {
            // The near-top test first, and *before* the temperature gate,
            // which is the whole of the fix -- see the block below. Two
            // `World::get`s, paid only by liquid that is already off
            // ambient (the caller's own guard), and everything below the
            // second row of its body stops here.
            let same = |ny: i32| world.get(x, ny).material == cell.material;
            if same(y - 1) && same(y - 2) {
                return;
            }
            boil = boil.max(boil_below(world, x, y, cell, heat_to_boil));
        }
        if boil <= 0.0 {
            return;
        }
        let rise = (self.frame / BUBBLE_RISE_PERIOD) as i32;
        // `Columns` stretches the site vertically and climbs at twice the
        // rate, so a site reads as a stream leaving one spot on the bottom
        // rather than as a free-floating pocket.
        let (block_w, block_h, scroll) = match self.bubbles {
            BubbleMode::Columns => (2, 4, rise * 2),
            BubbleMode::Large => (BUBBLE_LARGE_SITE, BUBBLE_LARGE_SITE, rise),
            _ => (BUBBLE_SITE, BUBBLE_SITE, rise),
        };
        let sy = y + scroll;
        // **Density is per *pixel*, not per site**, so a masked bubble does
        // not quietly thin the effect. `BUBBLE_DENSITY` is the share of the
        // pool that should light up; a site lights with that share scaled
        // by how much of its own area it actually covers. Without this, a
        // 5-of-9 disc drew a third of what a 4-of-4 square did -- 3,250
        // changed pixels on `scene=simmer` became 1,125, from a change that
        // was only supposed to be about shape. That mode is gone and the
        // correction stays, because `Large` needs exactly the same thing:
        // it lights `BUBBLE_LARGE_LIT` of 36.
        let (sx, sy_site) = (x.div_euclid(block_w), sy.div_euclid(block_h));
        let disc = large_bubble_disc(sx, sy_site);
        let lit = match self.bubbles {
            BubbleMode::Large => disc.1,
            _ => block_w * block_h,
        };
        let rarity = if self.bubbles == BubbleMode::Large { BUBBLE_LARGE_RARITY } else { 1.0 };
        let density = BUBBLE_DENSITY * rarity * (block_w * block_h) as f32 / lit as f32;
        // **A big bubble is lit or not as a whole, by the water at its
        // foot** -- and that is not a refinement, it is what makes the mode
        // legible at all. `boil` is the *pixel's* own heat, and a pool is
        // hot at the bottom and cool at the top, so a six-cell site
        // straddles the ramp: its lower rows clear the threshold and its
        // upper rows do not, and a disc drawn that way is sliced into a
        // horizontal lens. Rendered and looked at before it was measured --
        // the pan read as flat cloud streaks rather than bubbles.
        //
        // A bubble forms at the hot floor and rises intact, so the site
        // asks the cell under its own centre-bottom instead. One extra
        // `World::get`, paid only by `Large` and only by cells the caller
        // has already found to be off ambient.
        let gate = if self.bubbles == BubbleMode::Large {
            let fx = sx * block_w + block_w / 2;
            let fy = (sy_site * block_h + block_h - 1) - scroll;
            let foot = world.get(fx, fy);
            if foot.material == cell.material {
                heat_to_boil(foot.temperature())
            } else {
                boil
            }
        } else {
            boil
        };
        let site = rng::jitter(sx, sy_site);
        if site >= density * gate {
            return;
        }
        let (ix, iy) = (x.rem_euclid(block_w), sy.rem_euclid(block_h));
        // **Only `Large` is masked**, and `BUBBLE_SITE`'s doc records why
        // rounding the small one is not on the table: at two cells across
        // there is nothing to round, and at three the mask draws a cross,
        // which play called worse than the square it replaced.
        //
        // `Columns` keeps its 2x4 stretch unmasked for a different reason:
        // it is deliberately *not* a bubble but a stream leaving a
        // nucleation site, and rounding it would collapse the difference
        // between the two modes.
        if self.bubbles == BubbleMode::Large && !large_bubble_covers(ix, iy, disc.0) {
            return;
        }
        // Two rows are a dome by being brighter on top; six are not, and a
        // hard split across the middle of a big disc reads as two stacked
        // half-moons. `Large` ramps instead. The small sites keep the split
        // exactly as it was -- for a 2-high site the ramp is the same thing,
        // but `Columns` is 2x4 and would quietly change look.
        let lift = if self.bubbles == BubbleMode::Large {
            let t = 1.0 - iy as f32 / (block_h - 1) as f32;
            BUBBLE_UNDERSIDE + (BUBBLE_LIFT - BUBBLE_UNDERSIDE) * t
        } else if iy < block_h / 2 {
            BUBBLE_LIFT
        } else {
            BUBBLE_UNDERSIDE
        } * (BUBBLE_FLOOR + (1.0 - BUBBLE_FLOOR) * boil);
        for (c, tint) in rgb.iter_mut().zip(BUBBLE_TINT) {
            *c = (*c as f32 + (tint - *c as f32) * lift).round().clamp(0.0, 255.0) as u8;
        }
    }

    /// Blends `base` toward a colour ramp keyed on the currently-selected
    /// field channel — a no-op returning `base` unchanged when the overlay
    /// is off, so every caller can route through this unconditionally
    /// rather than each needing its own `if field_overlay != Off` guard.
    ///
    /// Blend strength scales with `magnitude` (0..1, how far the reading
    /// sits from that channel's own ambient/baseline value) up to
    /// `MAX_BLEND` at full saturation — **not** a flat blend regardless of
    /// value. An earlier version used a flat blend specifically to avoid
    /// washing out low readings, but every channel's ramp colour at
    /// `magnitude == 0` is some fixed, saturated colour (`Off` aside), not
    /// `base` itself — a flat blend therefore tints *every* unaffected
    /// pixel toward that fixed colour regardless of whether the channel is
    /// actually elevated there, which independent review caught
    /// concretely for pressure (every ambient cell, i.e. nearly the whole
    /// visible world nearly all the time, blended 60% toward white). Real
    /// elevation still reads clearly under magnitude-scaling — a fully
    /// saturated reading still reaches `MAX_BLEND` — it just no longer
    /// paints the entire screen for a channel currently near zero
    /// everywhere.
    fn apply_field_overlay(&self, world: &World, x: i32, y: i32, base: [u8; 4]) -> [u8; 4] {
        // **The pheromone channels return here, before the blend tail
        // below ever runs, and that is not a shortcut.** `CLAUDE.md`'s
        // "a debug readout must not be a function of the thing it debugs"
        // requires a full replace on a fixed dark->bright ramp; the four
        // arms below are magnitude-*blends* into the cell's own colour,
        // which is right for them (a flat blend tints every ambient pixel
        // of a channel that is near zero everywhere) and wrong here.
        //
        // The failure a blend produces is not subtle-looking, it is
        // *invisible*: the canopy-density sheet read as blank because the
        // ramp was red, wood is brown, and a mid-range value moved one
        // colour byte from 139 to 155. The obvious reading — "the
        // mechanism is dead" — would have sent a fix at working code. A
        // trail is one cell wide over dirt, which is that case exactly.
        //
        // Do not "unify" these two paths.
        let pheromone_channel = match self.field_overlay {
            FieldOverlay::PheromoneA => Some((crate::sim::pheromone::Channel::A, SCALAR_RAMP_PHERO_A)),
            FieldOverlay::PheromoneB => Some((crate::sim::pheromone::Channel::B, SCALAR_RAMP_PHERO_B)),
            _ => None,
        };
        if let Some((channel, full)) = pheromone_channel {
            let t = world.pheromone_at(channel, x, y) as f32 / 255.0;
            let ramp = scalar_ramp(t, full);
            let mut out = base;
            for (c, r) in out.iter_mut().take(3).zip(ramp) {
                *c = r.round().clamp(0.0, 255.0) as u8;
            }
            return out;
        }
        // **Temperature joined them, for the reason above and one more.**
        // As a blend it was unreadable twice over: the sky's day/night
        // forcing is single degrees on a range sized for a fire's hundreds,
        // so it moved a colour byte by a hair — and what it moved it against
        // was the cell's own colour, which the renderer is *already* tinting
        // with the time of day. A warm ground at noon and a warm-lit ground
        // at sunset came out looking the same, which is the one comparison
        // the channel exists to make. Contact sheets of a full day over
        // generated terrain were built and read before and after this
        // change; before, the surface band's own temperature was not
        // legible at all.
        //
        // Signed, because this channel now has a below-ambient half that a
        // clamp at zero would erase: blue for cooler than ambient, orange
        // for warmer. Logarithmic in the distance from ambient, because the
        // two things worth seeing are three orders of magnitude apart — a
        // 6-degree night still reaches a third of the ramp, a 900-degree
        // fire still saturates it.
        if matches!(self.field_overlay, FieldOverlay::Temperature) {
            let delta = world.field_at(x, y).temperature - AMBIENT_TEMPERATURE as f32;
            let t = (delta.abs().ln_1p() / TEMPERATURE_OVERLAY_MAX.ln_1p()).clamp(0.0, 1.0);
            let ramp = scalar_ramp(t, if delta >= 0.0 { SCALAR_RAMP_WARM } else { SCALAR_RAMP_COOL });
            let mut out = base;
            for (c, r) in out.iter_mut().take(3).zip(ramp) {
                *c = r.round().clamp(0.0, 255.0) as u8;
            }
            return out;
        }
        let (ramp, magnitude) = match self.field_overlay {
            FieldOverlay::Off => return base,
            FieldOverlay::Pressure => {
                let f = world.field_at(x, y);
                // Signed: red for positive (compression), blue for negative
                // (rarefaction) -- `magnitude` (not the ramp colour itself)
                // is what fades this to `base` at zero, so both directions
                // share one ramp pair rather than needing a third "zero"
                // colour of their own.
                let t = (f.pressure / PRESSURE_OVERLAY_RANGE).clamp(-1.0, 1.0);
                let colour = if t >= 0.0 { [255.0, 60.0, 60.0] } else { [60.0, 90.0, 255.0] };
                (colour, t.abs())
            }
            // Temperature is handled by the full-replace branch above.
            FieldOverlay::Temperature => return base,
            FieldOverlay::Light => {
                let f = world.field_at(x, y);
                let t = (f.light / LIGHT_OVERLAY_MAX).clamp(0.0, 1.0);
                ([255.0, 255.0, 190.0], t)
            }
            FieldOverlay::Moisture => {
                let f = world.field_at(x, y);
                let t = (f.moisture / MOISTURE_OVERLAY_MAX).clamp(0.0, 1.0);
                ([60.0, 140.0, 255.0], t)
            }
            // Handled by the full-replace branch above, which returns.
            FieldOverlay::PheromoneA | FieldOverlay::PheromoneB => return base,
        };
        const MAX_BLEND: f32 = 0.75;
        let blend = magnitude.clamp(0.0, 1.0) * MAX_BLEND;
        let mut out = base;
        for (c, r) in out.iter_mut().take(3).zip(ramp) {
            *c = (*c as f32 + (r - *c as f32) * blend).round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    fn draw_chunk_overlay(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        for chunk in world.chunks() {
            let colour = if chunk.is_settled() {
                CHUNK_BORDER_SETTLED
            } else {
                CHUNK_BORDER_ACTIVE
            };
            let (ox, oy) = chunk.coord.origin();
            for i in 0..CHUNK_SIZE {
                // Each world-space border point maps through zoom/stride
                // individually rather than scaling the whole chunk-sized
                // loop bound -- simpler than special-casing the two zoom
                // directions here, at the cost of a thin (not `zoom`-wide)
                // line when magnified, an accepted minor look rather than
                // added complexity for a debug overlay.
                if let Some((sx, sy)) = self.world_to_screen(ox + i, oy) {
                    put(frame, width, height, sx, sy, colour);
                }
                if let Some((sx, sy)) = self.world_to_screen(ox, oy + i) {
                    put(frame, width, height, sx, sy, colour);
                }
            }
        }
    }

    /// World position under a screen pixel — the camera offset plus the
    /// zoom/stride scale `draw`'s own per-pixel loop applies, used to turn
    /// cursor position into a brush position (so painting lands on the
    /// cell actually under the cursor at any zoom level, not the
    /// unscaled-1:1 cell it would be at `zoom == 1`).
    ///
    /// `div_euclid`, not `/`: a screen position left of the camera (a
    /// negative `sx`, reachable once panning exists) must floor toward
    /// negative infinity the same way `ChunkCoord::containing` already
    /// establishes is required — truncating division would fold screen
    /// column -1 onto the same world cell as column 0.
    pub fn screen_to_world(&self, sx: i32, sy: i32) -> (i32, i32) {
        if self.zoom > 1 {
            (self.camera_x + sx.div_euclid(self.zoom), self.camera_y + sy.div_euclid(self.zoom))
        } else {
            (self.camera_x + sx * self.zoom_out_stride, self.camera_y + sy * self.zoom_out_stride)
        }
    }

    /// Inverse of `screen_to_world`, for placing something drawn in world
    /// space (a particle, a chunk border) onto the screen. `None` when the
    /// position falls between two stride-sampled columns/rows at
    /// `zoom_out_stride > 1` and so has no single screen pixel of its own —
    /// distinct from simply being off-screen, which callers already clip
    /// against separately via `put`'s own bounds check.
    pub fn world_to_screen(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        if self.zoom > 1 {
            Some(((x - self.camera_x) * self.zoom, (y - self.camera_y) * self.zoom))
        } else if self.zoom_out_stride > 1 {
            let (dx, dy) = (x - self.camera_x, y - self.camera_y);
            if dx.rem_euclid(self.zoom_out_stride) != 0 || dy.rem_euclid(self.zoom_out_stride) != 0 {
                return None;
            }
            Some((dx.div_euclid(self.zoom_out_stride), dy.div_euclid(self.zoom_out_stride)))
        } else {
            Some((x - self.camera_x, y - self.camera_y))
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// `World::underground_map`'s answer, computed here for a world that has
/// never been stepped and therefore has no map of its own.
///
/// Deliberately the same flood fill as `World::freeze_underground_map` —
/// from the top row, 4-connected, through everything that is not `Solid` or
/// `Powder`, marking what it fails to reach — because two implementations of
/// "is this outdoors" that could drift is the shape of bug this whole area
/// keeps producing. If either changes, both change.
fn underground_from_scratch(world: &World, b: Rect) -> Vec<u64> {
    let (w, h) = (b.width() as usize, b.height() as usize);
    let idx = |x: i32, y: i32| (y - b.min_y) as usize * w + (x - b.min_x) as usize;
    let blocks = |x: i32, y: i32| {
        matches!(
            world.materials.kind(world.get(x, y).material),
            crate::sim::material::MaterialKind::Solid | crate::sim::material::MaterialKind::Powder
        )
    };
    let mut open = vec![false; w * h];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for x in b.min_x..=b.max_x {
        if !blocks(x, b.min_y) {
            open[idx(x, b.min_y)] = true;
            stack.push((x, b.min_y));
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < b.min_x || nx > b.max_x || ny < b.min_y || ny > b.max_y {
                continue;
            }
            if open[idx(nx, ny)] || blocks(nx, ny) {
                continue;
            }
            open[idx(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }
    let mut bits = vec![0u64; (w * h).div_ceil(64)];
    for (i, reached) in open.iter().enumerate() {
        if !reached {
            bits[i >> 6] |= 1 << (i & 63);
        }
    }
    bits
}

pub(crate) fn put(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, colour: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let i = (y as usize * width as usize + x as usize) * 4;
    frame[i..i + 4].copy_from_slice(&colour);
}

/// Blend `colour` over whatever is already in the frame at `(x, y)`, with
/// `alpha` in 0..=1. The HUD's panels are drawn *after* the world, straight
/// into the same framebuffer, so this is the whole of the translucency
/// mechanism — no second buffer, no compositing pass.
///
/// Integer math on the way out, but the blend itself is float: these are
/// panel-sized fills (a few tens of thousands of pixels at most) drawn only
/// while an overlay is open, not the per-cell hot path `cell_colour` is, so
/// the readability is worth more here than the cycles.
pub(crate) fn blend(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, colour: [u8; 4], alpha: f32) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);
    let i = (y as usize * width as usize + x as usize) * 4;
    for c in 0..3 {
        let dst = frame[i + c] as f32;
        frame[i + c] = (dst + (colour[c] as f32 - dst) * a).round().clamp(0.0, 255.0) as u8;
    }
    frame[i + 3] = 255;
}

/// Midpoint circle algorithm, eight-way symmetry — the brush outline
/// preview (§9 of `PLAN.md`'s UI-improvement pass), and reusable for
/// anything else that ever wants a cheap outline (a future explosion
/// radius preview, say). `radius <= 0` draws nothing rather than a
/// degenerate single pixel, since a zero-radius brush is a real, valid
/// brush size and a dot at the cursor would misleadingly suggest a
/// one-cell circle instead.
pub(crate) fn draw_circle_outline(frame: &mut [u8], width: u32, height: u32, cx: i32, cy: i32, radius: i32, colour: [u8; 4]) {
    if radius <= 0 {
        return;
    }
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;
    while x >= y {
        for (dx, dy) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
            put(frame, width, height, cx + dx, cy + dy, colour);
        }
        y += 1;
        err += 1 + 2 * y;
        if 2 * (err - x) + 1 > 0 {
            x -= 1;
            err += 1 - 2 * x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    /// Reported from play: mining the top of a column dropped that column's
    /// horizon and the sky came down the hole with the pick, so a one-cell
    /// shaft dug into a mountain drew a strip of bright sky to its middle.
    ///
    /// Now guaranteed by construction rather than repaired: the ground
    /// surface is frozen on the world's first frame (`World::
    /// freeze_sky_surface`) and digging cannot move it. The widths are the
    /// substance of this test — the rule this replaced inferred the answer
    /// from shape with a six-column reach, and flipped from "tunnel" to
    /// "open daylight" between a 12- and a 13-cell shaft. Widening a shaft
    /// is what mining *is*, so a rule with a width threshold anywhere in it
    /// is a rule that breaks while the player is using it.
    #[test]
    fn digging_a_shaft_does_not_bring_the_sky_down_with_it() {
        for width in [1i32, 12, 13, 40] {
            let mut world = World::new(Rect::new(0, 0, 127, 127));
            for x in 0..128 {
                for y in 40..128 {
                    world.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            // **The order of events is the test.** The world runs first,
            // which is what freezes the surface and what a player's world
            // has always done before they pick anything up. Digging into a
            // world that has never been stepped is not a sequence that
            // exists outside a test.
            world.begin_step();
            for x in (64 - width / 2)..=(64 + width / 2) {
                for y in 40..100 {
                    world.set(x, y, Cell::EMPTY);
                }
            }
            let mut r = Renderer::new();
            r.rebuild_horizon(&world);

            assert!(!r.under_sky(64, 90), "a cell deep inside a {width}-wide mined shaft is underground, not sky");
            assert!(r.under_sky(64, 20), "open air above the terrain is still sky ({width}-wide shaft)");
        }
    }

    /// **Building up no longer lowers the sky, and that is a deliberate
    /// reversal.** The rule this replaced took the skyline from the topmost
    /// cell, so stacking material into the air took the sky down with it —
    /// which was the same mechanism that put a black rectangle under every
    /// tree and a column of cave under every single floating pixel, and
    /// there is no version of it that keeps one and drops the other.
    ///
    /// What is lost: lay a roof over a gap and the space under it reads as
    /// outdoors rather than as a room. What is gained: it reads as outdoors
    /// *because it is*, and nothing a player builds can accidentally
    /// blacken the world under it. Making a building read as indoors is a
    /// wall layer, deliberately not this (`Reports/open-bugs-handoff.md`).
    #[test]
    fn building_a_roof_does_not_turn_the_air_under_it_into_a_cave() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 80..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world.begin_step();
        for x in 20..110 {
            world.set(x, 40, Cell::new(material::STONE, 0));
        }
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);

        assert!(r.under_sky(64, 60), "the air under a roof built after the world started is still outdoors");
        assert!(!r.under_sky(64, 100), "rock below the frozen surface is still underground");
    }

    /// Reported from play: "dark bands under any overhangs or objects or
    /// when I'm mining", with the guess — correct — that it was the frozen
    /// background baseline rather than a shadow.
    ///
    /// **The case the per-column rule could not express.** An overhanging
    /// lip has rock above it *in its own column*, so every cell of open air
    /// beneath it answered "underground" and drew as unlit cave, fading to
    /// full `UNDERGROUND` within 24 rows. Worldgen makes these deliberately
    /// (`passes::brows`, up to `MAX_BROW_REACH` = 20 columns), and measured
    /// across seeds 1–6 it put 156–408 cells of false cave in every world
    /// (`examples/underground_probe.rs`).
    ///
    /// The geometry here is the minimum that reproduces it: a lip standing
    /// clear of the cliff with nothing under it but sky and, well below,
    /// the ground. The world runs first, because the freeze happens on the
    /// first frame and a lip that is not present at freeze time proves
    /// nothing.
    #[test]
    fn an_overhanging_lip_does_not_put_a_cave_in_the_sky_beneath_it() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 100..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        // A cliff on the left, with a lip reaching out over open air.
        for x in 0..40 {
            for y in 40..100 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 40..70 {
            for y in 40..44 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world.begin_step();
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);

        assert!(r.under_sky(55, 70), "the open air under a cliff lip is sky, not the inside of a cave");
        assert!(r.under_sky(55, 99), "and it is still sky right down to the ground under the lip");
        // The control, and it is the half that makes this a guard rather
        // than a licence: a genuine roofed void must still read as cave, or
        // "fixing" this by calling everything sky would pass.
        for x in 0..40 {
            for y in 60..70 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        let mut fresh = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 40..128 {
                fresh.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 50..70 {
            for y in 60..70 {
                fresh.set(x, y, Cell::EMPTY);
            }
        }
        fresh.begin_step();
        let mut r2 = Renderer::new();
        r2.rebuild_horizon(&fresh);
        assert!(!r2.under_sky(60, 65), "a sealed cavity inside the rock must still read as cave");
    }

    /// The other half of the same report — "or objects" — and the case a
    /// review card caught first, as *"a dark vertical band through the
    /// pond"* on `scene=rockdrop` at **frame 0, with zero bodies in
    /// flight**.
    ///
    /// A solid object standing in the air when the world is made sets its
    /// columns' skyline to its own top, so the column rule drew a hard-edged
    /// band the object's exact width running down through the air, through
    /// any water, and onto the floor — to the bottom of the world, with no
    /// falloff. Distinct from `a_tree_does_not_turn_the_sky_behind_it_into_
    /// a_cave` above, which is answered by excluding `Plant` from the
    /// freeze: a slab of stone is `Solid`, and no exclusion list reaches it.
    #[test]
    fn a_slab_hanging_in_the_air_at_genesis_casts_no_cave_beneath_it() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 100..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 50..70 {
            for y in 30..36 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        world.begin_step();
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);

        assert!(r.under_sky(60, 40), "the air just under a suspended slab is sky");
        assert!(r.under_sky(60, 99), "and so is the air all the way down to the ground");
        // Beside the slab, at the same row, as the paired control: if the
        // whole frame had gone sky for some unrelated reason this would not
        // catch it, and the interesting claim is that the two agree.
        assert!(r.under_sky(20, 40), "open air beside the slab is sky too");
    }

    /// **The property, not an instant.** The per-cell map may only ever move
    /// a cell from underground toward sky, never the reverse — a fix that
    /// blackened something new while clearing the reported bands would be a
    /// worse trade than the bug, and `CLAUDE.md` records a fix that shipped
    /// exactly that way because its test only looked where it expected to be
    /// wrong.
    ///
    /// It holds by construction: both ways of being marked underground (the
    /// cell was ground, or it was air the sky could not reach, which needs
    /// ground above it) put the column's topmost ground at or above the
    /// cell, which is precisely what the column rule tested. This asserts it
    /// over every cell of a world carrying a lip, a suspended slab, a sealed
    /// cavity and a dug shaft at once, so a future change to either fill has
    /// something to fail against.
    #[test]
    fn the_per_cell_map_never_turns_open_sky_into_cave() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 90..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 0..30 {
            for y in 40..90 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 30..55 {
            for y in 40..44 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 80..100 {
            for y in 20..26 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 60..75 {
            for y in 100..110 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        world.begin_step();
        for x in 110..114 {
            for y in 90..120 {
                world.set(x, y, Cell::EMPTY);
            }
        }

        let mut r = Renderer::new();
        r.rebuild_horizon(&world);
        let mut rescued = 0;
        for y in 0..128 {
            for x in 0..128 {
                let column_says_sky = r.sky_depth(x, y) < 0;
                let map_says_sky = r.under_sky(x, y);
                if column_says_sky {
                    assert!(
                        map_says_sky,
                        "({x}, {y}) was open sky under the column rule and is cave under the map — \
                         the per-cell map must only ever rescue, never blacken"
                    );
                }
                if map_says_sky && !column_says_sky {
                    rescued += 1;
                }
            }
        }
        // And it must actually be doing something, or the assertion above
        // passes on a map that is not wired up at all.
        assert!(rescued > 0, "the map rescued nothing on a world built around a lip and a slab — is it reaching under_sky?");
    }

    /// `App::reset` builds a new `World` and keeps the `Renderer`, so the
    /// cached skyline has to follow the world rather than the buffer size.
    /// An earlier version of `rebuild_horizon` returned early whenever the
    /// width and origin matched, which is *always* true across a
    /// regenerate — it would have drawn the previous terrain's skyline over
    /// freshly generated ground for the rest of the session, and no test in
    /// this file would have noticed because none of them regenerate.
    #[test]
    fn regenerating_the_world_does_not_leave_the_old_skyline_behind() {
        let bounds = Rect::new(0, 0, 127, 127);
        let mut high = World::new(bounds);
        for x in 0..128 {
            for y in 30..128 {
                high.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        high.begin_step();
        let mut r = Renderer::new();
        r.rebuild_horizon(&high);
        assert!(!r.under_sky(64, 60), "test setup: y=60 is inside the high terrain");

        // A whole new world, same bounds, ground much lower -- exactly what
        // switching preset or reseeding produces.
        let mut low = World::new(bounds);
        for x in 0..128 {
            for y in 90..128 {
                low.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        low.begin_step();
        r.rebuild_horizon(&low);
        assert!(r.under_sky(64, 60), "the skyline must follow the new world, not the old one's dimensions");
    }

    /// A lake whose level falls must not leave a band of false cave hanging
    /// above it. Water is not ground, so the frozen surface is the rock
    /// under the lake and the whole water column reads as outdoors however
    /// far the level drops.
    ///
    /// This is not hypothetical: mining into a lake drains it (seen in a
    /// `viewshot mine=1` render, which is what caught the first version
    /// counting the waterline), and evaporation lowers one on its own.
    #[test]
    fn draining_a_lake_does_not_leave_a_dark_band_above_it() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 100..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 40..90 {
            for y in 60..100 {
                world.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        world.begin_step();
        // The level falls by half.
        for x in 40..90 {
            for y in 60..80 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);

        assert!(r.under_sky(64, 61), "air where the lake used to be is open sky, not a cave");
        assert!(r.under_sky(64, 99), "the last row above the lake bed is still sky");
        assert!(!r.under_sky(64, 110), "rock under the lake bed is underground");
    }

    /// The terrain depth light (2026-08 world review): rock dims with depth
    /// below the frozen skyline, the skyline row is lifted, and anything
    /// sitting *above* the skyline — here a pond — is untouched by the mode,
    /// so the grade cannot leak into water, trees, or the sky. The weakest
    /// assertion that fails if the mechanism is deleted entirely; the look
    /// itself is judged from `viewshot light=depth|flat` strips and `F10`.
    #[test]
    fn terrain_depth_light_dims_rock_with_depth_and_spares_whats_above_the_skyline() {
        // Same shape as `sky::tests::brightness`, which is not visible from
        // here.
        fn brightness(c: [u8; 4]) -> u32 {
            c[0] as u32 + c[1] as u32 + c[2] as u32
        }
        let mut world = World::new(Rect::new(0, 0, 191, 255));
        for x in 0..192 {
            for y in 64..256 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        // A pond standing on the ground: water is not ground, so the frozen
        // skyline is the rock at y=64 and the water above it has negative
        // depth.
        for x in 40..90 {
            for y in 58..64 {
                world.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        // A narrow notch (5 columns, well under the shoulder window) and a
        // wide valley (42 columns, well over it), both dropping the skyline
        // 32 rows. The light datum must clip the notch to its shoulders and
        // leave the valley alone — the property that kills the bright
        // shafts the raw skyline drew under canyon slots.
        for x in 110..115 {
            for y in 64..96 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        for x in 140..182 {
            for y in 64..96 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        world.begin_step();
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);

        r.terrain_light = TerrainLight::Depth;
        let surface = brightness(r.cell_colour(&world, 10, 64));
        let shallow = brightness(r.cell_colour(&world, 10, 65));
        let deep = brightness(r.cell_colour(&world, 10, 64 + DEPTH_LIGHT_RAMP_ROWS));
        let water_depth = r.cell_colour(&world, 64, 60);

        r.terrain_light = TerrainLight::Off;
        let shallow_flat = brightness(r.cell_colour(&world, 10, 65));
        let deep_flat = brightness(r.cell_colour(&world, 10, 64 + DEPTH_LIGHT_RAMP_ROWS));
        let surface_flat = brightness(r.cell_colour(&world, 10, 64));
        let water_flat = r.cell_colour(&world, 64, 60);

        assert!(
            deep < shallow,
            "rock {DEPTH_LIGHT_RAMP_ROWS} rows down ({deep}) must draw dimmer than rock one row down ({shallow})"
        );
        assert!(deep < deep_flat, "the grade must actually dim deep rock ({deep} vs flat {deep_flat})");
        assert!(
            surface > surface_flat,
            "the skyline row is lifted ({surface} vs flat {surface_flat}), not merely spared"
        );
        // The jitter grain is position-keyed and identical across modes, so
        // exact equality is the right assertion, not a tolerance.
        assert_eq!(water_depth, water_flat, "water above the skyline must not take the grade");
        assert!(
            (shallow as i32 - shallow_flat as i32).abs() <= 2,
            "one row below the skyline is inside the smoothstep's flat start and must be visually untouched ({shallow} vs {shallow_flat})"
        );
        assert!(
            deep > brightness(UNDERGROUND),
            "deep rock stays legible — dimmer is not dark ({deep} vs underground {})",
            brightness(UNDERGROUND)
        );

        r.terrain_light = TerrainLight::Depth;
        // One row under the notch floor: raw skyline says depth 1, the
        // clipped datum says 33 — it must draw as deep rock, not as a
        // bright shaft. Same row under the wide valley floor: genuinely
        // depth 1, stays bright. Grain differs by position, so the
        // comparison needs the ramp's spread (~2.4x at these depths), not
        // equality.
        let notch_rock = brightness(r.cell_colour(&world, 112, 97));
        let valley_rock = brightness(r.cell_colour(&world, 160, 97));
        assert!(
            notch_rock < valley_rock * 3 / 4,
            "rock beside a narrow notch floor must not light up as if at the surface ({notch_rock} vs valley {valley_rock})"
        );
        // The notch's *air* is still open sky — the datum is the light's
        // business only.
        assert!(r.under_sky(112, 80), "a narrow notch's air is still sky, not cave");
    }

    /// Reported from play: "the effect where light is blocked below plants
    /// or really anything is way too intense", with a picture of a tree.
    ///
    /// What it was is not shade at all. `rebuild_horizon` took the topmost
    /// **non-empty cell of any kind** as the skyline, so a canopy set the
    /// horizon for every column it covered and every empty cell under it —
    /// the air a person would stand in — drew as `UNDERGROUND`. On screen a
    /// tree cast a hard-edged black rectangle over the sky behind it, the
    /// full depth of the world.
    #[test]
    fn a_tree_does_not_turn_the_sky_behind_it_into_a_cave() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 80..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        let wood = world.materials.id_of("wood").expect("wood is a compiled-in material");
        // A canopy: a horizontal plate of wood well above the ground, with
        // open air under it. Deliberately wide: the rule this replaced had a
        // six-column reach, so a narrow plate could be rescued by accident
        // and prove nothing.
        for x in 40..90 {
            world.set(x, 40, Cell::new(wood, 0));
        }
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);

        assert!(r.under_sky(64, 60), "the air under a canopy is sky, not the inside of a cave");
        assert!(r.under_sky(64, 79), "the ground under a canopy is lit by daylight");
        assert!(!r.under_sky(64, 100), "rock below the surface is still underground");
    }

    /// The other half of the same report, and the half that survives once
    /// plants stop counting: under a *stone* roof the change from full
    /// daylight to `UNDERGROUND` happened in a single row. A room the player
    /// builds went black the instant it was enclosed.
    ///
    /// The guard is written against the *replacement* artifact as well as
    /// the original — it fails if the fade never reaches full dark, not only
    /// if it is instant — because a fix that simply lightened `UNDERGROUND`
    /// would pass a one-sided version of this and would also make every deep
    /// cave in the world grey.
    #[test]
    fn the_dark_under_a_roof_fades_in_with_depth_rather_than_cutting() {
        let mut world = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            for y in 120..128 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 0..128 {
            world.set(x, 20, Cell::new(material::STONE, 0));
        }
        let mut r = Renderer::new();
        r.rebuild_horizon(&world);
        r.sky = crate::sky::Sky::at(600, 0, 127, 0, 127); // mid-morning, so the sky is bright

        let brightness = |c: [u8; 4]| c[0] as i32 + c[1] as i32 + c[2] as i32;
        let sky = brightness(r.cell_colour(&world, 64, 19));
        let just_under = brightness(r.cell_colour(&world, 64, 21));
        let deep = brightness(r.cell_colour(&world, 64, 21 + CAVE_FADE_DEPTH));
        let dark = brightness(UNDERGROUND);

        assert!(
            just_under > sky - (sky - dark) / 2,
            "the first row under a roof should still read as lit, not as a cave: sky {sky}, under {just_under}, dark {dark}"
        );
        assert!(
            deep <= dark + 2,
            "past the fade depth it must reach full dark, or every cave in the world turns grey: {deep} against {dark}"
        );
    }


    /// What "underground" currently answers, across the shapes mining and
    /// building actually produce. Prints rather than asserts: this is the
    /// evidence for the design question in `Reports/`, not a guard.
    #[test]
    #[ignore = "probe, not a guard"]
    fn probe_what_counts_as_underground() {
        const GROUND: i32 = 80;
        let build = |f: &dyn Fn(&mut World)| -> Renderer {
            let mut world = World::new(Rect::new(0, 0, 127, 127));
            for x in 0..128 {
                for y in GROUND..128 {
                    world.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            // The world runs, which freezes the surface, and only then is
            // it edited -- the order a player produces, and the only one
            // that says anything about digging or building at all.
            world.begin_step();
            f(&mut world);
            let mut r = Renderer::new();
            r.rebuild_horizon(&world);
            r
        };
        let plank = |w: &mut World, half: i32| {
            for x in (64 - half)..=(64 + half) {
                w.set(x, 40, Cell::new(material::STONE, 0));
            }
        };
        let shaft = |w: &mut World, half: i32| {
            for x in (64 - half)..=(64 + half) {
                for y in GROUND..115 {
                    w.set(x, y, Cell::EMPTY);
                }
            }
        };

        println!("{:<44}  {:>10}  verdict", "geometry", "at (64,60)");
        let report = |label: &str, r: &Renderer, y: i32| {
            let d = r.sky_depth(64, y);
            println!("{label:<44}  {d:>10}  {}", if d < 0 { "sky" } else { "UNDERGROUND" });
        };
        report("bare ground", &build(&|_| {}), 60);
        report("one floating cell at y=40", &build(&|w| w.set(64, 40, Cell::new(material::STONE, 0))), 60);
        report("1-wide spire, y=40 up from the ground", &build(&|w| {
            for y in 40..GROUND {
                w.set(64, y, Cell::new(material::STONE, 0));
            }
        }), 60);
        for half in [0, 1, 3, 6, 12, 25] {
            report(&format!("plank {} wide at y=40", half * 2 + 1), &build(&|w| plank(w, half)), 60);
        }
        println!();
        println!("{:<44}  {:>10}  verdict", "geometry", "at (64,100)");
        for half in [0, 1, 3, 6, 12, 25] {
            report(&format!("shaft {} wide dug from the surface", half * 2 + 1), &build(&|w| shaft(w, half)), 100);
        }
    }

    #[test]
    fn circle_outline_lights_the_ring_but_not_the_centre() {
        let (w, h) = (32u32, 32u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let white = [255, 255, 255, 255];
        draw_circle_outline(&mut frame, w, h, 16, 16, 10, white);

        let px = |x: i32, y: i32| -> [u8; 4] {
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            frame[i..i + 4].try_into().unwrap()
        };
        // The four axis-aligned points at exactly `radius` away are always
        // lit for any radius under the midpoint algorithm's own symmetry.
        assert_eq!(px(26, 16), white, "east point of the ring should be lit");
        assert_eq!(px(6, 16), white, "west point of the ring should be lit");
        assert_eq!(px(16, 26), white, "south point of the ring should be lit");
        assert_eq!(px(16, 6), white, "north point of the ring should be lit");
        // An outline, not a filled disc.
        assert_ne!(px(16, 16), white, "the centre should not be lit by an outline");
    }

    #[test]
    fn a_zero_or_negative_radius_circle_draws_nothing() {
        let (w, h) = (16u32, 16u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        draw_circle_outline(&mut frame, w, h, 8, 8, 0, [255, 255, 255, 255]);
        draw_circle_outline(&mut frame, w, h, 8, 8, -5, [255, 255, 255, 255]);
        assert!(frame.iter().all(|&b| b == 0), "a zero/negative radius should draw nothing, not a stray pixel");
    }

    #[test]
    fn draws_material_colours_and_void_outside_the_world() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(30, 30, Cell::new(material::SAND, 0));
        let mut renderer = Renderer::new();
        // A lone cell is its own column's skyline, so the terrain depth
        // light would lift it; this test is about palette + grain, so the
        // selector is pinned flat.
        renderer.terrain_light = TerrainLight::Off;
        let particles = ParticleSystem::new();

        // A 128-wide framebuffer over a 64-wide world: the right half is void.
        let (w, h) = (128u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        // Not hot, so only the position-jitter grain (see `JITTER_STRENGTH`)
        // should have moved this away from the flat palette colour.
        let sand = world.materials.get(material::SAND).palette[0];
        let jitter_permille = ((rng::jitter(30, 30) - 0.5) * 2000.0 * JITTER_STRENGTH) as i32;
        let expected = [
            (sand[0] as i32 + (sand[0] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (sand[1] as i32 + (sand[1] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (sand[2] as i32 + (sand[2] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            255,
        ];
        let idx = (30 * w as usize + 30) * 4;
        assert_eq!(&frame[idx..idx + 4], &expected);

        // Row 0, column 100 — past the right edge of the 64-wide world.
        let outside = 100 * 4;
        assert_eq!(&frame[outside..outside + 4], &VOID);
    }

    #[test]
    fn glow_lights_the_cave_air_around_a_crystal_lining() {
        // The paired-cavity scene from `field::glow_tests`, read back
        // through the renderer: two identical sealed cavities deep under
        // stone, one floored with crystal. Every pixel difference between
        // matching positions is the glow — the cave fade, the sky and the
        // depth grade all cancel.
        let mut world = World::new(Rect::new(0, 0, 255, 255));
        let crystal = world.materials.id_of("crystal").expect("crystal ships in the registry");
        for y in 100..256 {
            for x in 0..256 {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for (x0, lined) in [(96, true), (192, false)] {
            for y in 180..200 {
                for x in x0..x0 + 24 {
                    world.set(x, y, Cell::EMPTY);
                }
            }
            if lined {
                for y in 200..203 {
                    for x in x0 + 4..x0 + 20 {
                        world.set(x, y, Cell::new(crystal, 0));
                    }
                }
            }
        }
        for _ in 0..300 {
            crate::sim::field::step(&mut world);
        }

        let mut renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (256u32, 256u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        let px = |x: usize, y: usize| -> [u8; 4] {
            let i = (y * w as usize + x) * 4;
            frame[i..i + 4].try_into().unwrap()
        };
        let brightness = |c: [u8; 4]| c[0] as u32 + c[1] as u32 + c[2] as u32;

        // Cave air a block above the lining, against the same spot in the
        // unlined cavity. Both are deep enough for the full cave fade, so
        // without the glow they render identically.
        let lit_air = brightness(px(104, 195));
        let dark_air = brightness(px(200, 195));
        assert!(
            lit_air > dark_air,
            "cave air over the lining should draw brighter than the unlined cavity's: lit {lit_air}, dark {dark_air}"
        );

        // The stone under the lining catches the lift too, against the same
        // stone under the dark cavity — glow lands on the walls, not only
        // the air. Summed over a patch rather than one pixel: every cell
        // carries its own ±12% position-keyed grain (`JITTER_STRENGTH`),
        // which is zero-mean over a patch, while the glow lift is
        // systematic. One pixel against one pixel would be a deterministic
        // coin flip between grain draws.
        let patch = |x0: usize| -> u32 {
            let mut total = 0;
            for y in 204..207 {
                for x in x0..x0 + 16 {
                    total += brightness(px(x, y));
                }
            }
            total
        };
        let lit_wall = patch(100);
        let dark_wall = patch(196);
        assert!(
            lit_wall > dark_wall,
            "stone under the lining should draw brighter than stone under the dark cavity: lit {lit_wall}, dark {dark_wall}"
        );
    }

    #[test]
    fn field_overlay_off_matches_the_pre_overlay_render_exactly() {
        // The overlay must be a genuine no-op when off, not just
        // visually negligible -- `apply_field_overlay` is unconditionally
        // in the call path now (both the empty- and non-empty-cell
        // branches route through it). Compares against a `cell_colour`
        // computed by hand the pre-overlay way (grain + heat glow, nothing
        // else) rather than `Off` against `Off`, which would trivially
        // pass regardless of whether the refactor preserved anything.
        let mut world = World::new(Rect::new(0, 0, 31, 31));
        world.set(10, 10, Cell::new(material::STONE, 0));
        world.add_pressure_impulse(5, 5, 3, 20.0); // field nonzero nearby, still must not show through when off
        let renderer = Renderer::new();
        let base = world.materials.get(material::STONE).palette[0];
        let jitter_permille = ((rng::jitter(10, 10) - 0.5) * 2000.0 * JITTER_STRENGTH) as i32;
        let expected = [
            (base[0] as i32 + (base[0] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (base[1] as i32 + (base[1] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            (base[2] as i32 + (base[2] as i32 * jitter_permille) / 1000).clamp(0, 255) as u8,
            255,
        ];
        assert_eq!(renderer.cell_colour(&world, 10, 10), expected);
    }

    #[test]
    fn field_overlay_leaves_an_unaffected_cell_unchanged_even_when_on() {
        // The exact property an independent review caught broken in an
        // earlier version: a flat blend regardless of reading magnitude
        // tinted *every* pixel toward a fixed colour, including cells with
        // a perfectly ordinary ambient reading, not just ones near a real
        // disturbance. A cell far from the one active impulse below,
        // for every channel, must render identically to the overlay
        // being off -- confirmed to fail (for Pressure specifically) with
        // the blend strength reverted to a flat constant.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(50, 50, Cell::new(material::STONE, 0));
        world.add_pressure_impulse(5, 5, 3, 40.0); // disturbance confined to a far corner
        let mut renderer = Renderer::new();
        let off = renderer.cell_colour(&world, 50, 50);
        // **Blend channels only, deliberately.** `PheromoneA`/`PheromoneB`
        // are excluded because they are full-replace by design: a cell with
        // a zero reading draws at `SCALAR_RAMP_FLOOR` of its ramp colour,
        // *not* at `base`, which is the entire point (a channel that is
        // zero everywhere must read as a visible dark field, not as
        // "overlay is off"). Adding them to this list would be asserting
        // that the fix for the canopy-density blank sheet had not been
        // applied.
        //
        // **`Temperature` left this list when the sky started writing that
        // channel**, for the same reason and not as a concession: it is
        // full-replace now, its ambient reading draws at the ramp floor, and
        // an overlay that renders "20C" and "26C" as the same pixel cannot
        // show a day/night cycle at all. `the_temperature_overlay_tells_warm
        // _from_cool` below is the guard that replaced this one for it.
        for overlay in [FieldOverlay::Pressure, FieldOverlay::Light, FieldOverlay::Moisture] {
            renderer.field_overlay = overlay;
            assert_eq!(renderer.cell_colour(&world, 50, 50), off, "{overlay:?} tinted an unaffected cell far from any real disturbance");
        }
    }

    #[test]
    fn the_temperature_overlay_tells_warm_from_cool() {
        // The day/night sky forcing is a few degrees on a channel whose
        // other user is a fire. Under the old linear-on-900 blend a whole
        // warm afternoon and a whole cold night rendered as the same pixel
        // as ambient -- so this asserts the three cases are three colours,
        // which is the property that makes `filmstrip channel=temperature`
        // able to show a day at all.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        for x in [10, 30, 50] {
            world.set(x, 10, Cell::new(material::STONE, 0));
        }
        world.add_heat(30, 10, 2, 6.0); // a warm afternoon, not a fire
        world.add_heat(50, 10, 2, -6.0); // and a cold night
        let mut renderer = Renderer::new();
        renderer.field_overlay = FieldOverlay::Temperature;
        let ambient = renderer.cell_colour(&world, 10, 10);
        let warm = renderer.cell_colour(&world, 30, 10);
        let cool = renderer.cell_colour(&world, 50, 10);
        assert_ne!(warm, ambient, "a 6-degree warm reading rendered identically to ambient");
        assert_ne!(cool, ambient, "a 6-degree cool reading rendered identically to ambient");
        assert!(warm[0] > cool[0] && cool[2] > warm[2], "warm and cool must not be the same hue: warm {warm:?}, cool {cool:?}");
    }

    #[test]
    fn pressure_overlay_tints_a_cell_near_a_real_impulse() {
        let mut world = World::new(Rect::new(0, 0, 31, 31));
        world.add_pressure_impulse(15, 15, 5, 40.0);
        let particles = ParticleSystem::new();
        let (w, h) = (32u32, 32u32);

        let mut renderer = Renderer::new();
        let mut off = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut off, (w, h), true);

        renderer.field_overlay = FieldOverlay::Pressure;
        let mut on = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut on, (w, h), true);

        assert_ne!(off, on, "the pressure overlay should visibly change the render over a real pressure impulse");
    }

    #[test]
    fn cycle_grain_visits_every_mode_and_returns_to_the_current_behaviour() {
        let mut r = Renderer::new();
        assert_eq!(r.grain, GrainMode::Position, "the default must stay today's behaviour");
        let mut seen = vec![r.grain];
        for _ in 0..6 {
            r.cycle_grain();
            seen.push(r.grain);
        }
        assert_eq!(seen.len(), 7);
        r.cycle_grain();
        assert_eq!(r.grain, GrainMode::Position, "cycling should wrap back round");
    }

    #[test]
    fn cycle_field_overlay_visits_every_channel_and_returns_to_off() {
        let mut r = Renderer::new();
        assert_eq!(r.field_overlay, FieldOverlay::Off);
        let mut seen = vec![r.field_overlay];
        for _ in 0..6 {
            r.cycle_field_overlay();
            seen.push(r.field_overlay);
        }
        assert_eq!(
            seen,
            vec![
                FieldOverlay::Off,
                FieldOverlay::Pressure,
                FieldOverlay::Temperature,
                FieldOverlay::Light,
                FieldOverlay::Moisture,
                FieldOverlay::PheromoneA,
                FieldOverlay::PheromoneB
            ]
        );
        r.cycle_field_overlay();
        assert_eq!(r.field_overlay, FieldOverlay::Off, "cycling should wrap back to Off, not stop at the last channel");
    }

    #[test]
    fn the_pheromone_overlay_is_a_full_replace_not_a_blend() {
        // P-23 / correction #4. The four field channels above blend into
        // the cell's own colour; a pheromone reading must *replace* it, or
        // the same value over two different materials draws as two
        // different brightnesses and the sheet becomes unreadable — which
        // is how the canopy-density overlay came to read as blank.
        //
        // Two very different base materials, one identical plane value:
        // the pixels must come out identical.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(10, 10, Cell::new(material::STONE, 0));
        let coal = world.materials.id_of("coal").unwrap_or(material::SAND);
        world.set(20, 20, Cell::new(coal, 0));
        world.deposit_pheromone(crate::sim::pheromone::Channel::A, 10, 10, 128);
        world.deposit_pheromone(crate::sim::pheromone::Channel::A, 20, 20, 128);

        let mut r = Renderer::new();
        r.field_overlay = FieldOverlay::PheromoneA;
        assert_eq!(
            r.cell_colour(&world, 10, 10),
            r.cell_colour(&world, 20, 20),
            "equal pheromone over different materials must draw identically -- a blend would leak the material colour through"
        );

        // And a zero reading must still be visible as the ramp floor
        // rather than reading as "the overlay is off".
        let off_pixel = {
            let mut plain = Renderer::new();
            plain.field_overlay = FieldOverlay::Off;
            plain.cell_colour(&world, 40, 40)
        };
        assert_ne!(r.cell_colour(&world, 40, 40), off_pixel, "a zero reading must draw at the ramp floor, so an empty channel reads as empty rather than as absent");
        assert_ne!(r.cell_colour(&world, 40, 40), r.cell_colour(&world, 10, 10), "and a zero reading must still be distinguishable from a strong one");
    }

    #[test]
    fn draws_a_free_particle_at_its_rounded_position() {
        let world = World::new(Rect::new(0, 0, 63, 63));
        let mut renderer = Renderer::new();
        let mut particles = ParticleSystem::new();
        particles.spawn(10.4, 20.3, 0.0, 0.0, material::SAND, 1);

        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        let sand = world.materials.get(material::SAND).palette[1];
        let idx = (20 * w as usize + 10) * 4;
        assert_eq!(&frame[idx..idx + 4], &sand, "particle did not draw at its rounded position");
    }

    /// A pool of water at `temp`, filling the world, drawn once.
    fn boiling_pool_frame(mode: BubbleMode, temp: i16) -> Vec<u8> {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        for y in 0..64 {
            for x in 0..64 {
                world.set(x, y, Cell::new(material::WATER, 0).with_temperature(temp));
            }
        }
        let mut renderer = Renderer::new();
        renderer.bubbles = mode;
        let particles = ParticleSystem::new();
        let mut frame = vec![0u8; 64 * 64 * 4];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (64, 64), true);
        frame
    }

    /// How many pixels of `a` differ from `b`.
    fn pixels_differing(a: &[u8], b: &[u8]) -> usize {
        a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count()
    }

    #[test]
    fn every_bubble_mode_actually_draws_bubbles_in_boiling_water() {
        // **A counter, not a picture.** The first version of this effect
        // was judged from a `scene=boil` contact sheet and read as "no
        // change" -- which at that zoom is indistinguishable from the
        // bubbles being drawn *and* lost in the steam cloud over the pool.
        // Only a count separates the two, and the count is what says the
        // mechanism ran at all (`CLAUDE.md`).
        let off = boiling_pool_frame(BubbleMode::Off, 95);
        for mode in [BubbleMode::Rising, BubbleMode::Large, BubbleMode::Columns, BubbleMode::Surface] {
            let on = boiling_pool_frame(mode, 95);
            let drawn = pixels_differing(&off, &on);
            println!("{}: {drawn} of 4096 pixels bubbled", mode.label());
            assert!(drawn > 0, "{} drew nothing in a pool at 95C", mode.label());
            // Sparse, and that is the other half of the claim: a mode that
            // lit a third of the pool would be foam, not boiling water.
            assert!(drawn < 4096 / 3, "{} bubbled {drawn} of 4096 pixels, which is foam rather than boiling", mode.label());
        }
    }

    /// Squared distance between two RGB triples. Squared because nothing
    /// here compares a distance to anything but another distance.
    fn colour_gap(a: [f32; 3], b: [f32; 3]) -> f32 {
        (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
    }

    /// A drawn bubble has to be nearer steam than the water it sits in.
    ///
    /// # The guard the colour complaint needed and did not have
    ///
    /// Reported from play: the bubbles *"are not the color of steam"*. Every
    /// bubble test in this file passed through that, and none of them could
    /// have caught it -- they count *which* pixels light and what shape the
    /// lit set is, and the complaint was about the colour of the pixels that
    /// were already lighting in the right places.
    ///
    /// `BUBBLE_TINT` was steam's colour the whole time. What was wrong was
    /// the blend fraction: at the old `BUBBLE_LIFT`/`BUBBLE_UNDERSIDE` a lit
    /// pixel came out **44 from the pond and 166 from steam** (RGB units, on
    /// `scene=simmer`) -- numerically a shade of water. It now sits 150 from
    /// the pond and 60 from steam, on the other side of the line, and this
    /// asserts the side rather than the number so a re-tune of either
    /// constant is free until it crosses back.
    #[test]
    fn a_bubble_is_drawn_nearer_steam_than_water() {
        let world = World::new(Rect::new(0, 0, 1, 1));
        let steam = world.materials.id_of("steam").expect("steam is a built-in");
        let tint = world
            .materials
            .get(steam)
            .palette
            .iter()
            .map(|c| colour_gap(BUBBLE_TINT, [c[0] as f32, c[1] as f32, c[2] as f32]))
            .fold(f32::INFINITY, f32::min);
        // The doc on `BUBBLE_TINT` claims it *is* steam's colour, and the
        // whole fix rests on that claim, so it is asserted rather than
        // asserted-in-prose: 20 units covers the spread of steam's own three
        // palette entries and nothing wider.
        assert!(tint < 20.0 * 20.0, "BUBBLE_TINT is {:.0} RGB units from steam's nearest palette colour", tint.sqrt());

        let off = boiling_pool_frame(BubbleMode::Off, 95);
        for mode in [BubbleMode::Rising, BubbleMode::Large, BubbleMode::Columns, BubbleMode::Surface] {
            let on = boiling_pool_frame(mode, 95);
            let (mut lit, mut nearer_steam) = (0usize, 0usize);
            for (water, bubble) in off.chunks(4).zip(on.chunks(4)) {
                if water == bubble {
                    continue;
                }
                lit += 1;
                let w = [water[0] as f32, water[1] as f32, water[2] as f32];
                let b = [bubble[0] as f32, bubble[1] as f32, bubble[2] as f32];
                if colour_gap(b, BUBBLE_TINT) < colour_gap(b, w) {
                    nearer_steam += 1;
                }
            }
            let share = nearer_steam as f32 / lit as f32;
            println!("{}: {nearer_steam} of {lit} lit pixels nearer steam than water ({:.0}%)", mode.label(), share * 100.0);
            // Not all of them: a bubble's shaded underside is *meant* to keep
            // some water in it, and `Columns` is a stream rather than a
            // bubble. The bar is that the effect as a whole reads as gas.
            assert!(share > 0.75, "{} drew {:.0}% of its pixels nearer water than steam", mode.label(), (1.0 - share) * 100.0);
        }
    }

    /// Prints which pixels each bubble mode lights, as a map, on the
    /// uniform pool the guards use.
    ///
    /// A probe rather than a guard, and the reason it exists is recorded in
    /// `CLAUDE.md`: a contact sheet answers *what and where*, and at the
    /// zoom one is read at, "one six-cell disc" and "four small blobs in a
    /// row" are the same picture. Judging `Large`'s clustering off a PNG
    /// sent this session down a wrong turn twice before the map settled it.
    #[test]
    #[ignore = "probe, not a guard"]
    fn probe_bubble_layout() {
        let off = boiling_pool_frame(BubbleMode::Off, 95);
        for mode in [BubbleMode::Rising, BubbleMode::Large, BubbleMode::Columns] {
            let on = boiling_pool_frame(mode, 95);
            println!("== {}", mode.label());
            for y in 0..32 {
                let row: String = (0..64)
                    .map(|x| {
                        let i = (y * 64 + x) * 4;
                        if off[i..i + 4] != on[i..i + 4] {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect();
                println!("{row}");
            }
        }
    }

    /// **`Large` is a disc; `Rising` is deliberately still a square.**
    ///
    /// Both halves matter, and the second one is why this test is not the
    /// one it replaces. Rounding `Rising` was built, shipped and rejected
    /// on sight -- *"You went from squares to crosses. That is worse"* --
    /// so a guard that demands round bubbles everywhere would be a guard
    /// against the thing play asked for. `BUBBLE_SITE` holds the reasoning.
    ///
    /// Asserted as **no lit `Large` site has a lit corner**, over whichever
    /// row alignment the scroll happens to be at, rather than by eye: a
    /// masked bubble and an unmasked one are twelve pixels apart at a zoom
    /// no contact sheet is read at.
    #[test]
    fn a_large_bubble_is_a_disc_and_a_small_one_is_not() {
        let off = boiling_pool_frame(BubbleMode::Off, 95);
        let on = boiling_pool_frame(BubbleMode::Large, 95);
        let lit = |x: usize, y: usize| {
            let i = (y * 64 + x) * 4;
            off[i..i + 4] != on[i..i + 4]
        };
        assert!((0..64).any(|x| (0..64).any(|y| lit(x, y))), "nothing was drawn, so this asserts nothing");

        let site = BUBBLE_LARGE_SITE as usize;
        let square_free = (0..site).any(|offset| {
            (0..64 / site).all(|sx| {
                (0..(64 - offset) / site).all(|sy| {
                    let (x0, y0) = (sx * site, offset + sy * site);
                    let any = (0..site).any(|dx| (0..site).any(|dy| lit(x0 + dx, y0 + dy)));
                    !any || ![(0, 0), (site - 1, 0), (0, site - 1), (site - 1, site - 1)]
                        .iter()
                        .any(|&(dx, dy)| lit(x0 + dx, y0 + dy))
                })
            })
        });
        assert!(square_free, "every row alignment has a lit LARGE site with a lit corner, so the mask is not being applied");

        // And the paired half: `Rising` still fills its site corner to
        // corner. If someone rounds it again this fails, which is the
        // point -- the last person to do it had this file's own doc
        // comment in front of them.
        let small = boiling_pool_frame(BubbleMode::Rising, 95);
        let small_lit = |x: usize, y: usize| {
            let i = (y * 64 + x) * 4;
            off[i..i + 4] != small[i..i + 4]
        };
        let site = BUBBLE_SITE as usize;
        let full_square = (0..64 / site).any(|sx| {
            (0..64 / site).any(|sy| {
                let (x0, y0) = (sx * site, sy * site);
                (0..site).all(|dx| (0..site).all(|dy| small_lit(x0 + dx, y0 + dy)))
            })
        });
        assert!(full_square, "no RISING site is lit corner to corner; the square has been rounded off again");
    }

    /// `BUBBLE_LARGE_LIT` divides the density on the per-pixel path, so it
    /// is hardcoded rather than counted — and this is what stops it
    /// drifting away from the mask it claims to describe. A wrong count
    /// here does not crash or look broken; it silently changes how much
    /// boil the mode draws, which is exactly the failure the density
    /// correction exists to prevent.
    #[test]
    fn the_large_bubble_mask_lights_what_the_density_correction_assumes() {
        for (radius_sq, claimed) in BUBBLE_LARGE_DISCS {
            let counted = (0..BUBBLE_LARGE_SITE)
                .flat_map(|ix| (0..BUBBLE_LARGE_SITE).map(move |iy| (ix, iy)))
                .filter(|&(ix, iy)| large_bubble_covers(ix, iy, radius_sq))
                .count();
            assert_eq!(counted as i32, claimed, "the disc at radius^2 {radius_sq} lights {counted} cells, not the {claimed} its entry claims");
        }
    }

    /// **Both sizes actually appear.** A mix that is 100:0 is not a mix,
    /// and nothing else in the file would notice -- the density correction
    /// is per site, so a broken size hash draws a perfectly respectable
    /// grid of identical bubbles, which is the artifact this replaced.
    #[test]
    fn a_large_bubble_field_holds_both_of_its_sizes() {
        let mut seen = [0usize; BUBBLE_LARGE_DISCS.len()];
        for sx in 0..40 {
            for sy in 0..40 {
                let disc = large_bubble_disc(sx, sy);
                let i = BUBBLE_LARGE_DISCS.iter().position(|d| d.0 == disc.0).expect("a disc from the table");
                seen[i] += 1;
            }
        }
        for (i, count) in seen.iter().enumerate() {
            // A third of 1,600 either way: far from an even split, and far
            // from a hash that has collapsed onto one branch.
            assert!(*count > 1600 / 3, "size {i} came up {count} times in 1,600 sites; the size hash is lopsided");
        }
    }

    /// **Changing a bubble's shape must not change how much boil it
    /// draws**, which it did once.
    ///
    /// `BUBBLE_DENSITY` is the share of the *pool* that should light up, and
    /// it was being applied per *site* — so a 5-of-9 disc quietly cut the
    /// effect to a third: 3,250 changed pixels on `scene=simmer` became
    /// 1,125, from a change that was only supposed to be about shape. The
    /// density is normalised by how much of its own area a site covers, and
    /// that correction is what lets `Large` mask twelve of its
    /// thirty-six cells away and still boil as hard as `Rising`.
    ///
    /// So the claim is across *modes*, not for one of them: whichever
    /// bubble you pick, the pool boils as much as that mode means to.
    ///
    /// `Large` is the interesting case, because it is *deliberately*
    /// sparser (`BUBBLE_LARGE_RARITY` — fewer and bigger is what was asked
    /// for). Pinning it against `Rising` scaled by that constant separates
    /// the two effects: a broken mask or a lost density correction shows
    /// up, and retuning the rarity does not have to touch this test.
    #[test]
    fn changing_a_bubbles_shape_does_not_thin_the_boil() {
        let off = boiling_pool_frame(BubbleMode::Off, 95);
        let mut small = 0usize;
        for mode in [BubbleMode::Rising, BubbleMode::Large, BubbleMode::Columns] {
            let drawn = pixels_differing(&off, &boiling_pool_frame(mode, 95));
            println!("{}: {drawn} of 4096 pixels bubbled", mode.label());
            if mode == BubbleMode::Large {
                // Half and double the population `BUBBLE_LARGE_RARITY`
                // asks for: wide enough to survive a reshuffle of the
                // pattern, narrow enough to catch a mask that has lost or
                // gained a third of its area.
                let want = small as f32 * BUBBLE_LARGE_RARITY;
                assert!(
                    (drawn as f32) > want * 0.5 && (drawn as f32) < want * 2.0,
                    "LARGE lit {drawn} of 4096 pixels against RISING's {small}; at BUBBLE_LARGE_RARITY it should be near {want:.0}, \
                     so the mask or the per-pixel density correction has changed the strength rather than the shape"
                );
                continue;
            }
            // A 64x64 pool at `BUBBLE_DENSITY`, measured 655 of 4,096 for
            // `Rising` -- the bars are half and double that.
            assert!(
                (300..1300).contains(&drawn),
                "{} lit {drawn} of 4096 pixels in a uniformly boiling pool; the effect has changed strength, not just shape",
                mode.label()
            );
            if mode == BubbleMode::Rising {
                small = drawn;
            }
        }
    }

    /// A pan whose floor is boiling and whose surface is merely warm —
    /// which is what a heated pool actually looks like, and what
    /// `boiling_pool_frame`'s uniform 95C world is not.
    fn simmering_pan_frame(mode: BubbleMode) -> Vec<u8> {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        // Ten rows deep -- a pan, like `scene=simmer`'s. A first version
        // was forty deep and drew nothing *correctly*: the floor was
        // thirty-two rows under the surface and `SURFACE_BUBBLE_DEPTH` is
        // eight, so there was no heat within reach to read. A fixture that
        // cannot contain the situation looks exactly like a broken
        // mechanism (`CLAUDE.md`).
        for y in 50..60 {
            for x in 0..64 {
                // 30C at the surface: off ambient, so the renderer's heat
                // branch admits it, and under `BUBBLE_MIN_TEMPERATURE`, so
                // its own heat can never light a bubble.
                let temp = if y >= 56 { 95 } else { 30 };
                world.set(x, y, Cell::new(material::WATER, 0).with_temperature(temp));
            }
        }
        let mut renderer = Renderer::new();
        renderer.bubbles = mode;
        let particles = ParticleSystem::new();
        let mut frame = vec![0u8; 64 * 64 * 4];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (64, 64), true);
        frame
    }

    /// **`Surface` reads the heat below it, not its own.**
    ///
    /// The mode needed a cell that was both near the top of its body and
    /// over `BUBBLE_MIN_TEMPERATURE`, and heat arrives from underneath — so
    /// on any bottom-heated pool those two sets are in different places and
    /// the mode drew essentially nothing. Measured on `filmstrip
    /// scene=simmer`, four tiles at zoom 5: **25 of 97,460 pixels** differed
    /// from `Off` before, and 1,650 after — with `Rising` at 3,250 and
    /// `Columns` at 1,350 on the same sheet, so it is a distinct look and
    /// not a copy of one of them.
    ///
    /// The owner saw exactly this and named the cause in the same breath:
    /// *"All images look mostly identical. But also if we are testing
    /// bubbles in water the boiling needs to happen at the bottom of the
    /// pond/lake instead of the top."*
    ///
    /// The paired negative keeps it honest: the same pan with a **cold**
    /// floor must still draw nothing, or the fix is just "bubble whenever
    /// the water is wet".
    #[test]
    fn surface_bubbles_read_the_heat_below_them_rather_than_their_own() {
        let off = simmering_pan_frame(BubbleMode::Off);
        let on = simmering_pan_frame(BubbleMode::Surface);
        let drawn = pixels_differing(&off, &on);
        // Two rows of a 64-wide pan qualify as "near the surface", so the
        // ceiling here is 128 cells' worth of `BUBBLE_DENSITY` — and it
        // fell from 16 to 7 when the bubble became a 3x3 disc, because a
        // taller site puts less of itself inside a two-row band. The
        // control is **exactly 0** either way, which is what makes a small
        // absolute number a clean separation rather than a marginal one.
        assert!(
            drawn >= 4,
            "Surface drew {drawn} pixels over a boiling floor -- a bubble is made below where it pops, \
             and this mode has to ask about the water underneath or it is dead on every real pool"
        );

        // Nothing under the surface is warm enough: nothing may be drawn.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        for y in 50..60 {
            for x in 0..64 {
                world.set(x, y, Cell::new(material::WATER, 0).with_temperature(30));
            }
        }
        let draw_it = |mode| {
            let mut renderer = Renderer::new();
            renderer.bubbles = mode;
            let mut frame = vec![0u8; 64 * 64 * 4];
            renderer.draw(&world, &ParticleSystem::new(), &HashSet::new(), &mut frame, (64, 64), true);
            frame
        };
        assert_eq!(
            pixels_differing(&draw_it(BubbleMode::Off), &draw_it(BubbleMode::Surface)),
            0,
            "a pan with nothing hot in it bubbled; the downward scan is not reading temperature"
        );
    }

    #[test]
    fn bubbles_are_silent_below_their_threshold_and_climb_with_the_frame() {
        // The two ways this effect could be wrong without being invisible:
        // drawing in water nobody heated, and drawing a *static* pattern
        // that reads as a texture rather than as anything rising.
        let cold = boiling_pool_frame(BubbleMode::Rising, AMBIENT_TEMPERATURE);
        let cold_off = boiling_pool_frame(BubbleMode::Off, AMBIENT_TEMPERATURE);
        assert_eq!(
            pixels_differing(&cold, &cold_off),
            0,
            "water at ambient bubbled; the effect is not gated on temperature"
        );

        // Same world, successive frames: the pattern must move. Drawn from
        // one renderer so its own frame counter advances, which is the only
        // thing that changes between the two.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        for y in 0..64 {
            for x in 0..64 {
                world.set(x, y, Cell::new(material::WATER, 0).with_temperature(95));
            }
        }
        let mut renderer = Renderer::new();
        renderer.bubbles = BubbleMode::Rising;
        let particles = ParticleSystem::new();
        let mut first = vec![0u8; 64 * 64 * 4];
        let mut later = vec![0u8; 64 * 64 * 4];
        renderer.draw(&world, &particles, &HashSet::new(), &mut first, (64, 64), true);
        for _ in 0..BUBBLE_RISE_PERIOD {
            renderer.draw(&world, &particles, &HashSet::new(), &mut later, (64, 64), true);
        }
        assert!(
            pixels_differing(&first, &later) > 0,
            "the bubble pattern is identical {BUBBLE_RISE_PERIOD} frames later; nothing is rising"
        );
    }

    #[test]
    fn translucent_gas_lets_the_background_through_and_opaque_does_not() {
        // A band of smoke against open sky, drawn three ways. The claim is
        // ordered rather than absolute: every mode has to move the pixel
        // *toward* what is behind it more than the last, and `Opaque` has
        // to move it not at all -- which is what keeps the way back to the
        // old look genuinely the old look rather than nearly it, now that
        // `ByFill` is the default.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let smoke = world.materials.id_of("smoke").expect("smoke.ron should be embedded");
        for y in 20..30 {
            for x in 0..64 {
                world.set(x, y, Cell::new(smoke, 0).with_aux(400));
            }
        }
        let draw = |mode: GasMode| {
            let mut renderer = Renderer::new();
            renderer.gas = mode;
            let particles = ParticleSystem::new();
            let mut frame = vec![0u8; 64 * 64 * 4];
            renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (64, 64), true);
            frame
        };
        // The sky itself, at a row nothing was drawn into, is what a fully
        // transparent gas cell would converge to.
        let opaque = draw(GasMode::Opaque);
        let sky_idx = (10 * 64 + 32) * 4;
        let sky = [opaque[sky_idx], opaque[sky_idx + 1], opaque[sky_idx + 2]];
        let gas_idx = (25 * 64 + 32) * 4;
        let distance = |f: &[u8]| {
            (0..3).map(|c| (f[gas_idx + c] as i32 - sky[c] as i32).abs()).sum::<i32>()
        };
        let (o, t, b) = (distance(&opaque), distance(&draw(GasMode::Translucent)), distance(&draw(GasMode::ByFill)));
        println!("distance from the sky behind it: opaque {o}, translucent {t}, by-fill {b}");
        assert!(o > 0, "the test cell already matches the sky, so this asserts nothing");
        assert!(t < o, "translucent gas did not move toward what is behind it ({t} against {o})");
        // aux 400 is a thinned cell, so by-fill has to be thinner still.
        assert!(b < t, "by-fill did not thin a 40%-full cell past the flat alpha ({b} against {t})");
    }

    #[test]
    fn a_particle_outside_the_framebuffer_does_not_panic_or_wrap() {
        let world = World::new(Rect::new(0, 0, 63, 63));
        let mut renderer = Renderer::new();
        let mut particles = ParticleSystem::new();
        particles.spawn(-5.0, -5.0, 0.0, 0.0, material::SAND, 0);
        particles.spawn(1000.0, 1000.0, 0.0, 0.0, material::SAND, 0);

        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        // Reaching this line without panicking is the assertion.
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);
    }

    #[test]
    fn grain_varies_between_same_material_cells_at_different_positions() {
        // The whole point of keying jitter on position rather than shade
        // index (see `JITTER_STRENGTH`'s doc): even two cells with the
        // *same* palette shade should end up visibly different, which a
        // flat "just show the palette colour" renderer never produced.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(0, 0, Cell::new(material::SAND, 0));
        world.set(10, 0, Cell::new(material::SAND, 0));
        let mut renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        let a = &frame[0..4];
        let b_idx = 10 * 4;
        let b = &frame[b_idx..b_idx + 4];
        assert_ne!(a, b, "two same-shade cells at different positions rendered identically");
    }

    #[test]
    fn a_hot_non_burning_cell_looks_warmer_than_a_cool_one() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(0, 0, Cell::new(material::STONE, 0).with_temperature(20)); // ambient
        world.set(10, 0, Cell::new(material::STONE, 0).with_temperature(500)); // hot, not burning
        let mut renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        let cool = &frame[0..4];
        let hot_idx = 10 * 4;
        let hot = &frame[hot_idx..hot_idx + 4];
        // Warmer means redder relative to blue -- a robust check regardless
        // of exactly how strong the jitter/AO terms happened to land, since
        // neither of those touches the red/blue balance the heat blend does.
        let warmth = |p: &[u8]| p[0] as i32 - p[2] as i32;
        assert!(
            warmth(hot) > warmth(cool),
            "a cell well above ambient should read warmer (redder) than one at ambient"
        );
    }

    #[test]
    fn a_burning_cells_colour_flickers_across_time_buckets() {
        // The user-facing complaint this fixes: fire used to be a static
        // blend keyed only on temperature, so a burning cell held one flat
        // colour for its whole burn -- no animation at all frame to frame.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let mut burning = Cell::new(material::OIL, 0);
        burning.ignite(9999);
        world.set(30, 30, burning);
        let mut renderer = Renderer::new();
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let idx = (30 * w as usize + 30) * 4;

        // Enough distinct flicker buckets that at least one pair of frames
        // should disagree even allowing for `jitter3` occasionally landing
        // close to the same value twice by chance.
        let mut colours = std::collections::HashSet::new();
        for bucket in 0..10 {
            world.frame = bucket * FLAME_FLICKER_PERIOD;
            let mut frame = vec![0u8; (w * h * 4) as usize];
            renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);
            colours.insert(frame[idx..idx + 4].to_vec());
        }
        assert!(colours.len() > 1, "a burning cell rendered identically across every time bucket -- no flicker at all");
    }

    #[test]
    fn a_merely_warm_non_burning_cell_shifts_hue_with_temperature_not_just_brightness() {
        // The other half of the animation fix: the fire tint itself used to
        // be one flat orange (`FIRE_TINT`) regardless of how hot a cell
        // read, so intensity only ever changed blend *strength*, never
        // colour. Comparing two different temperatures does not isolate
        // that claim -- a flat tint blended in harder by a rising
        // `heat_ratio` alone raises the green channel too, which is exactly
        // what the first version of this test got fooled by (confirmed by
        // temporarily flattening the tint back to a constant and watching
        // it still pass). Instead, pin the temperature at exactly the point
        // `heat_ratio` saturates to 1.0, where `fire == FIRE_TINT_HIGH`
        // algebraically regardless of blend strength `t`, and predict the
        // pixel by hand from the *old* flat tint. A real hue shift makes
        // the rendered pixel disagree with that flat-tint prediction; a
        // blend-strength-only change could never produce a disagreement
        // here, since `t` is the same number either way.
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let temperature = AMBIENT_TEMPERATURE + HEAT_GLOW_RANGE as i16; // heat_ratio == 1.0 exactly
        world.set(10, 0, Cell::new(material::STONE, 0).with_temperature(temperature));
        let mut renderer = Renderer::new();
        // The lone hot cell is its column's skyline, so the terrain depth
        // light would lift it and break the hand-reconstructed prediction;
        // this test is about the fire tint, so the selector is pinned flat.
        renderer.terrain_light = TerrainLight::Off;
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        let hot_idx = 10 * 4;
        let actual = &frame[hot_idx..hot_idx + 4];

        // Reconstruct exactly what the renderer computed, up to the tint:
        // same base palette colour, same position grain, same blend
        // strength `t` -- everything the old flat-tint code also had.
        let base = world.materials.get(material::STONE).palette[0];
        let jitter_permille = ((rng::jitter(10, 0) - 0.5) * 2000.0 * 0.12) as i32;
        let mut grained = [base[0], base[1], base[2]];
        for c in &mut grained {
            *c = (*c as i32 + (*c as i32 * jitter_permille) / 1000).clamp(0, 255) as u8;
        }
        let t = 0.5_f32; // heat_ratio (1.0) * 0.5, non-burning
        let old_flat_fire = [255.0f32, 140.0, 40.0];
        let predicted_with_old_flat_tint: Vec<u8> = grained
            .iter()
            .zip(old_flat_fire)
            .map(|(c, fire)| (*c as f32 + (fire - *c as f32) * t).round() as u8)
            .collect();

        assert_ne!(
            &actual[0..3],
            predicted_with_old_flat_tint.as_slice(),
            "at saturation the rendered colour should match FIRE_TINT_HIGH exactly (not the old flat tint), \
             so it must differ from what a flat-tint implementation at the same blend strength would produce: \
             actual={actual:?}, old-flat-tint prediction={predicted_with_old_flat_tint:?}"
        );
        // And it should match what the real ramp predicts (fire ==
        // FIRE_TINT_HIGH exactly once heat_ratio saturates).
        let predicted_with_ramp: Vec<u8> = grained
            .iter()
            .zip(FIRE_TINT_HIGH)
            .map(|(c, fire)| (*c as f32 + (fire - *c as f32) * t).round() as u8)
            .collect();
        assert_eq!(
            &actual[0..3],
            predicted_with_ramp.as_slice(),
            "actual={actual:?}, ramp prediction={predicted_with_ramp:?}"
        );
    }

    #[test]
    fn screen_to_world_accounts_for_the_camera() {
        let mut r = Renderer::new();
        assert_eq!(r.screen_to_world(5, 7), (5, 7));
        r.camera_x = 100;
        r.camera_y = -20;
        assert_eq!(r.screen_to_world(5, 7), (105, -13));
    }

    #[test]
    fn the_two_mappings_agree_under_a_camera_at_every_scale() {
        // `screen_to_world` had the only camera test in the file, and the
        // camera had never moved in anger. Every draw pass goes through
        // `world_to_screen` instead, so a disagreement between the two would
        // put the gnome, the moon and every chunk border in the wrong place
        // while the one tested function stayed right.
        for (zoom, stride) in [(1, 1), (2, 1), (4, 1), (1, 2), (1, 4)] {
            let mut r = Renderer::new();
            r.zoom = zoom;
            r.zoom_out_stride = stride;
            r.camera_x = 640;
            r.camera_y = 384;
            for (sx, sy) in [(0, 0), (13, 29), (100, 60)] {
                // Round-tripped as an *identity on the world cell*, not on the
                // screen pixel: at zoom > 1 several pixels share a cell, so
                // asserting the pixel comes back unchanged would be asserting
                // the quantisation rather than the agreement.
                let (wx, wy) = r.screen_to_world(sx, sy);
                let back = r.world_to_screen(wx, wy).expect("a visible cell maps back on screen");
                assert_eq!(
                    r.screen_to_world(back.0, back.1),
                    (wx, wy),
                    "zoom {zoom} stride {stride}: screen ({sx},{sy}) -> world ({wx},{wy}) -> screen {back:?} -> a different cell"
                );
            }
        }
    }

    #[test]
    fn follow_keeps_the_target_on_screen_and_inside_the_world() {
        let world = Rect::new(0, 0, 2047, 767);
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        for target in [(20, 20), (1000, 400), (2040, 760), (0, 0), (1024, 384)] {
            // Settled, not single-shot: the dead zone deliberately closes only
            // part of the gap per call, so one call after a long jump leaves
            // the target off screen and would be testing the wrong thing.
            for _ in 0..8 {
                r.follow(target, viewport, Some(world));
            }
            let (sx, sy) = r.visible_span(viewport);
            assert!(
                target.0 >= r.camera_x && target.0 < r.camera_x + sx,
                "target x {} off screen: camera {} span {sx}", target.0, r.camera_x
            );
            assert!(
                target.1 >= r.camera_y && target.1 < r.camera_y + sy,
                "target y {} off screen: camera {} span {sy}", target.1, r.camera_y
            );
            // Never shows the void outside the world.
            assert!(r.camera_x >= world.min_x && r.camera_x + sx - 1 <= world.max_x, "camera x {} escaped the world", r.camera_x);
            assert!(r.camera_y >= world.min_y && r.camera_y + sy - 1 <= world.max_y, "camera y {} escaped the world", r.camera_y);
        }
    }

    #[test]
    fn follow_holds_still_inside_the_dead_zone() {
        // The camera forces a full redraw when it moves, so a strictly-centred
        // view would repaint the whole screen on every frame the player walked
        // — the dirty-rect skip paid away for nothing.
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        let world = Rect::new(0, 0, 2047, 767);
        for _ in 0..8 {
            r.follow((1024, 384), viewport, Some(world));
        }
        let settled = (r.camera_x, r.camera_y);
        r.follow((1024 + 8, 384 + 4), viewport, Some(world));
        assert_eq!((r.camera_x, r.camera_y), settled, "a small step moved the camera");
    }

    #[test]
    fn zoomed_in_screen_to_world_maps_a_zoom_wide_block_to_one_world_cell() {
        let mut r = Renderer::new();
        r.zoom = 4;
        // Screen columns 0-3 should all land on world column 0; column 4
        // starts the next world cell.
        assert_eq!(r.screen_to_world(0, 0), (0, 0));
        assert_eq!(r.screen_to_world(3, 0), (0, 0));
        assert_eq!(r.screen_to_world(4, 0), (1, 0));
    }

    #[test]
    fn zoomed_out_screen_to_world_skips_by_the_stride() {
        let mut r = Renderer::new();
        r.zoom_out_stride = 3;
        assert_eq!(r.screen_to_world(0, 0), (0, 0));
        assert_eq!(r.screen_to_world(1, 0), (3, 0));
        assert_eq!(r.screen_to_world(2, 0), (6, 0));
    }

    #[test]
    fn screen_to_world_floors_toward_negative_infinity_at_zoom() {
        // Mirrors ChunkCoord::containing's own div_euclid reasoning: a
        // screen position left of the camera must not fold onto the same
        // world cell as one to its right of it.
        let mut r = Renderer::new();
        r.zoom = 2;
        assert_eq!(r.screen_to_world(-1, 0), (-1, 0));
        assert_eq!(r.screen_to_world(-2, 0), (-1, 0));
        assert_eq!(r.screen_to_world(-3, 0), (-2, 0));
    }

    #[test]
    fn adjust_zoom_forms_one_continuous_scale_across_both_fields() {
        let mut r = Renderer::new();
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 1));

        // Zooming out first counts up zoom_out_stride, zoom staying at 1.
        r.adjust_zoom(-1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 2));
        r.adjust_zoom(-1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 3));

        // Zooming back in counts zoom_out_stride back down to 1 before
        // zoom itself ever climbs above 1.
        r.adjust_zoom(1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 2));
        r.adjust_zoom(1);
        assert_eq!((r.zoom, r.zoom_out_stride), (1, 1));
        r.adjust_zoom(1);
        assert_eq!((r.zoom, r.zoom_out_stride), (2, 1));
    }

    #[test]
    fn zoom_and_zoom_out_stride_clamp_at_their_own_bounds() {
        let mut r = Renderer::new();
        for _ in 0..20 {
            r.adjust_zoom(1);
        }
        assert_eq!(r.zoom, MAX_ZOOM, "zoom should clamp rather than grow unbounded");

        let mut r = Renderer::new();
        for _ in 0..20 {
            r.adjust_zoom(-1);
        }
        assert_eq!(r.zoom_out_stride, MAX_ZOOM_OUT_STRIDE, "zoom_out_stride should clamp rather than grow unbounded");
    }

    #[test]
    fn drawing_at_zoom_two_fills_a_two_by_two_block_per_world_cell() {
        let mut world = World::new(Rect::new(0, 0, 15, 15));
        world.set(2, 2, Cell::new(material::STONE, 0));
        let mut r = Renderer::new();
        r.zoom = 2;
        let particles = ParticleSystem::new();
        let (w, h) = (32u32, 32u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        r.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);

        let px = |x: i32, y: i32| -> [u8; 4] {
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            frame[i..i + 4].try_into().unwrap()
        };
        // World cell (2, 2) should occupy screen block (4..6, 4..6).
        let stone_colour = px(4, 4);
        assert_eq!(px(5, 4), stone_colour);
        assert_eq!(px(4, 5), stone_colour);
        assert_eq!(px(5, 5), stone_colour);
        // One column/row past the block should already be back to void
        // (world cell (3, 2), still empty).
        assert_ne!(px(6, 4), stone_colour);
    }

    #[test]
    fn the_overlay_distinguishes_active_from_settled_chunks() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let mut renderer = Renderer::new();
        renderer.show_chunk_overlay = true;
        let particles = ParticleSystem::new();
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];

        // Freshly built chunks are dirty, so the border reads as active.
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);
        assert_eq!(&frame[0..4], &CHUNK_BORDER_ACTIVE);

        // Once settled it dims.
        world.end_step();
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (w, h), true);
        assert_eq!(&frame[0..4], &CHUNK_BORDER_SETTLED);
    }

    // --- §11: dirty-rect render skip -----------------------------------

    #[test]
    fn weave_puts_the_gnome_behind_some_trees_and_in_front_of_others() {
        // The owner's ask, as a property: "sometimes walking in front of
        // trees and sometimes behind". Both answers have to occur, and a
        // given tree has to keep its answer.
        let mut front = 0;
        let mut behind = 0;
        for id in 1..200u16 {
            match TreeDepth::Weave.in_front(id as u32) {
                true => behind += 1,
                false => front += 1,
            }
            assert_eq!(
                TreeDepth::Weave.in_front(id as u32),
                TreeDepth::Weave.in_front(id as u32),
                "a tree must not change which side of him it is on"
            );
        }
        assert!(front > 60 && behind > 60, "expected a real mix, got {front} in front and {behind} behind");
    }

    #[test]
    fn consecutively_planted_trees_do_not_simply_alternate() {
        // Why this is a hash and not `id & 1`. Organism ids are handed out
        // sequentially and worldgen plants a stand left to right, so parity
        // would lay down front-back-front-back across the screen — a
        // correlation the eye picks out at once.
        let runs = (2..60u16).filter(|&id| TreeDepth::Weave.in_front(id as u32) == TreeDepth::Weave.in_front((id - 1) as u32)).count();
        assert!(runs > 8, "only {runs} of 58 neighbouring pairs matched, which is parity in disguise");
    }

    #[test]
    fn a_tree_in_front_of_the_gnome_shows_through_him() {
        use crate::sim::player::Player;
        let (w, h) = (64i32, 64i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        for x in 0..w {
            world.set(x, h - 1, Cell::new(material::STONE, 0));
        }
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        let species = world.species.id_of("tree").expect("tree is compiled in");
        // Find an organism whose hash puts it in front of him.
        let organism = (0..64)
            .map(|_| world.push_organism(species))
            .find(|&id| TreeDepth::Weave.in_front(id as u32))
            .expect("some organism id hashes to the front");
        for y in 20..50 {
            for x in 28..40 {
                world.set(x, y, Cell::new(wood, 0).with_organism_id(organism));
            }
        }
        world.end_step();
        world.player = Some(Player::at(32, 32));

        let (uw, uh) = (w as u32, h as u32);
        let particles = ParticleSystem::new();
        let mut over = Renderer::new();
        over.tree_depth = TreeDepth::Front;
        let mut a = vec![0u8; (uw * uh * 4) as usize];
        over.draw(&world, &particles, &HashSet::new(), &mut a, (uw, uh), true);

        let mut through = Renderer::new();
        through.tree_depth = TreeDepth::Weave;
        let mut b = vec![0u8; (uw * uh * 4) as usize];
        through.draw(&world, &particles, &HashSet::new(), &mut b, (uw, uh), true);

        assert_ne!(a, b, "a tree in front of him should not draw the same as one behind him");
    }

    /// A formation wider than one cell must put him on **one** side of it.
    ///
    /// The guard for the bug the round-6 formation respec would have
    /// shipped: the scenery depth key was the world column, and
    /// `in_front`'s hash decorrelates adjacent keys by design, so from
    /// width three up a formation never agreed with itself -- 0% of 4000
    /// sampled columns, at widths 3, 5, 8 and 12 alike. He rendered as
    /// vertical stripes, half his columns drawn over the stone and half
    /// hidden behind it.
    ///
    /// Written to fail for the **replacement** artifact too, not only the
    /// original: it asserts a uniform decision across his whole width at
    /// every plausible formation width, so a key that is per-formation but
    /// unstable (a run-start recomputed per row, say, which would slice him
    /// horizontally instead) fails it as surely as the per-column one did.
    #[test]
    fn a_wide_formation_puts_the_gnome_wholly_in_front_or_wholly_behind() {
        use crate::sim::player::Player;
        let (w, h) = (64i32, 64i32);
        let flowstone = {
            let world = World::new(Rect::new(0, 0, 1, 1));
            world.materials.id_of("flowstone").expect("flowstone is compiled in")
        };
        let mut occluding = 0;
        for formation_w in [2i32, 3, 5, 8, 12] {
            // Sweep the formation across him so the test cannot pass by
            // landing on one lucky alignment: which columns the hash would
            // have disagreed on depends on where the formation sits.
            for left in 24..40 {
                let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
                for x in 0..w {
                    world.set(x, h - 1, Cell::new(material::STONE, 0));
                }
                for y in 20..50 {
                    for x in left..left + formation_w {
                        world.set(x, y, Cell::new(flowstone, 0));
                    }
                }
                world.end_step();
                world.player = Some(Player::at(32, 32));

                // Rendered through the real `draw`, not through a copy of
                // the key arithmetic -- a test that re-derives the key
                // passes on the buggy renderer too, which is the whole
                // failure this file keeps a comment about.
                //
                // `Front` never occludes and `Behind` always does, so those
                // two frames *are* the only two uniform outcomes. "Wholly
                // one side" is then exactly "the weave frame is one of
                // them", asserted over every pixel.
                let (uw, uh) = (w as u32, h as u32);
                let particles = ParticleSystem::new();
                let frame = |depth: TreeDepth| {
                    let mut r = Renderer::new();
                    r.tree_depth = depth;
                    let mut buf = vec![0u8; (uw * uh * 4) as usize];
                    r.draw(&world, &particles, &HashSet::new(), &mut buf, (uw, uh), true);
                    buf
                };
                let (over, under, weave) =
                    (frame(TreeDepth::Front), frame(TreeDepth::Behind), frame(TreeDepth::Weave));
                // Most sweep positions put the formation clear of him, and
                // those prove nothing either way. Counted rather than
                // assumed: a renderer that stopped occluding altogether
                // would satisfy every case below and is caught by the
                // count at the end instead. (`CLAUDE.md`: "did it fire at
                // all" needs a counter, not a picture.)
                if over == under {
                    continue;
                }
                occluding += 1;
                assert!(
                    weave == over || weave == under,
                    "width {formation_w} at x={left}: he is in front of some of the formation and \
                     behind the rest -- the weave frame matches neither extreme"
                );
            }
        }
        assert!(
            occluding >= 40,
            "only {occluding} of the swept positions actually put a formation over him; \
             the sweep is not exercising the thing it claims to guard"
        );
    }

    /// The near-field glow splat is built once, not once a frame.
    ///
    /// A counter, not a picture and not a timing, because neither can see
    /// this: a halo rebuilt every frame draws identically to a cached one,
    /// and the cost lands on a *settled* world -- the exact state the
    /// dirty-rect skip exists to make free, and the state `ascii`'s
    /// worst-frame line is least likely to catch because nothing is moving.
    /// The animated-grain lesson in `CLAUDE.md`: measure a cost against the
    /// state the optimisation exists for.
    #[test]
    fn a_settled_glow_does_not_rebuild_its_halo_every_frame() {
        let (w, h) = (128i32, 128i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        let spar = world.materials.id_of("spar").expect("spar is compiled in");
        for x in 0..w {
            world.set(x, h - 1, Cell::new(material::STONE, 0));
        }
        for y in 60..64 {
            for x in 60..64 {
                world.set(x, y, Cell::new(spar, 0));
            }
        }
        world.end_step();
        // Let the light field converge, or every draw below is legitimately
        // a rebuild and the test proves nothing.
        for _ in 0..400 {
            crate::sim::update::step(&mut world);
            crate::sim::field::step(&mut world);
        }

        let (uw, uh) = (w as u32, h as u32);
        let particles = ParticleSystem::new();
        let mut r = Renderer::new();
        let mut buf = vec![0u8; (uw * uh * 4) as usize];
        r.draw(&world, &particles, &HashSet::new(), &mut buf, (uw, uh), true);
        let first = r.near_glow_rebuilds;
        assert!(first > 0, "vacuous: the scene never built a halo at all");

        for _ in 0..8 {
            r.draw(&world, &particles, &HashSet::new(), &mut buf, (uw, uh), false);
        }
        assert_eq!(
            r.near_glow_rebuilds, first,
            "the halo was rebuilt on a settled world with nothing changed"
        );
    }

    #[test]
    fn an_idle_gnome_costs_a_settled_world_nothing() {
        // The other half of the pose key. It was widened from a rect to
        // (rect, facing, swinging) so that turning repaints — and the thing
        // that must not break in the process is the reason it is compared
        // at all: a character standing still in a settled world adds
        // nothing to the dirty region, which is what keeps zero-cost frames
        // zero-cost with him in them.
        use crate::sim::player::Player;
        let (w, h) = (64i32, 64i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        for x in 0..w {
            world.set(x, h - 1, Cell::new(material::STONE, 0));
        }
        world.end_step();
        world.player = Some(Player::at(32, 32));

        let (uw, uh) = (w as u32, h as u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (uw * uh * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (uw, uh), true);
        let recomputed = renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (uw, uh), false);
        assert_eq!(recomputed, 0, "an idle gnome should recompute no pixels at all");
    }

    #[test]
    fn turning_on_the_spot_repaints_him() {
        // The dirty-rect trap the pose key exists for. He does not move, so
        // his screen rectangle is identical frame to frame -- but every
        // pixel of him is mirrored, and a rect-keyed comparison would skip
        // the repaint and leave him facing the old way until something else
        // happened to dirty that region.
        use crate::sim::player::Player;
        let (w, h) = (64i32, 64i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        for x in 0..w {
            world.set(x, h - 1, Cell::new(material::STONE, 0));
        }
        world.end_step();
        let mut player = Player::at(32, 32);
        player.facing_left = false;
        world.player = Some(player);

        let (uw, uh) = (w as u32, h as u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (uw * uh * 4) as usize];
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (uw, uh), true);
        let facing_right = frame.clone();

        // Nothing in the world changes but the direction he faces.
        world.player.as_mut().unwrap().facing_left = true;
        renderer.draw(&world, &particles, &HashSet::new(), &mut frame, (uw, uh), false);

        assert_ne!(frame, facing_right, "he turned round and nothing on screen changed");

        // And the incremental result must match a full redraw exactly.
        let mut fresh = Renderer::new();
        let mut reference = vec![0u8; (uw * uh * 4) as usize];
        fresh.draw(&world, &particles, &HashSet::new(), &mut reference, (uw, uh), true);
        assert_eq!(frame, reference, "the skipped repaint left a stale pose behind");
    }

    #[test]
    fn dirty_rect_skip_is_pixel_identical_to_a_full_redraw() {
        use crate::sim::update;
        let (w, h) = (128i32, 64i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        for x in 0..w {
            world.set(x, h - 1, Cell::new(material::STONE, 0));
        }
        // A sand column confined to the right half, tall enough to still be
        // mid-fall after a few sweeps -- the left half's chunks settle once
        // the floor itself stops changing, while the chunk(s) under this
        // column stay active, so the world actually has both a settled and
        // an unsettled region for the skip to distinguish between.
        for y in 0..10 {
            world.set(w - 10, y, Cell::new(material::SAND, 0));
        }
        for _ in 0..3 {
            update::step(&mut world);
        }

        let (uw, uh) = (w as u32, h as u32);
        let particles = ParticleSystem::new();

        // Warm up on the pre-step world -- the real-usage shape (frame N's
        // buffer already holds frame N's correct bytes before frame N+1
        // draws on top of it), and critically the thing that makes this
        // test actually discriminating: an earlier version of it stepped
        // the world *before* both draws and never again, so nothing
        // differed between the warm-up and the "real" draw -- skipping the
        // wrong chunks entirely still passed, since a chunk's already-drawn
        // bytes cannot go stale by definition if its underlying cells never
        // changed afterward either. Stepping again *between* the two draws
        // closes that gap: the sand column's cells genuinely move this
        // step, so a buggy skip (wrong polarity, an off-by-one in the
        // world<->screen mapping, anything that leaves the wrong region
        // untouched) would leave stale sand-coloured pixels sitting where
        // the column used to be.
        //
        // `world.take_touched_chunks()` is called before *every* draw here,
        // matching `App::draw`'s own real discipline -- not just before the
        // `force_full: false` one. Skipping it on the warm-up call would
        // leave that first batch of touches sitting in the accumulator
        // un-consumed, so the second call would see *those* stale entries
        // on top of whatever genuinely changed in between, rather than
        // exactly what changed since the warm-up actually ran.
        let mut subject = Renderer::new();
        let mut actual = vec![0u8; (uw * uh * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        subject.draw(&world, &particles, &warm_up_touched, &mut actual, (uw, uh), true);

        update::step(&mut world);
        let touched = world.take_touched_chunks();
        let recomputed = subject.draw(&world, &particles, &touched, &mut actual, (uw, uh), false);

        let mut baseline = Renderer::new();
        let mut expected = vec![0u8; (uw * uh * 4) as usize];
        baseline.draw(&world, &particles, &HashSet::new(), &mut expected, (uw, uh), true);

        assert_eq!(actual, expected, "a dirty-rect-skipped redraw must be pixel-identical to a full one, after the world actually changed");
        assert!(
            recomputed < (uw * uh) as usize,
            "with the left half settled, the skip should recompute fewer than every pixel (recomputed {recomputed} of {})",
            uw * uh
        );
    }

    #[test]
    fn a_chunk_that_settles_between_two_draws_is_still_correctly_redrawn() {
        // The exact bug a debug harness caught live: something falls, is
        // never drawn again until well after it lands, and a renderer that
        // asks `chunk.is_settled()` only at draw time sees a chunk that
        // reads settled *right now* and wrongly concludes nothing needs
        // redrawing there -- even though the chunk was very much active,
        // repeatedly, across every tick in between. `App::draw` fetches
        // `World::take_touched_chunks` every call specifically so this
        // can't happen; this test drives `Renderer::draw` the same way,
        // bypassing `App` entirely so a regression here fails fast and
        // close to the actual mechanism, not several layers away.
        let (w, h) = (64i32, 64i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        for x in 0..w {
            world.set(x, h - 1, Cell::new(material::STONE, 0));
        }
        world.set(30, 2, Cell::new(material::SAND, 0));

        let (uw, uh) = (w as u32, h as u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (uw * uh * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (uw, uh), true); // warm up, sand visible near the top

        // Run the sand all the way to the floor and fully settled -- *no*
        // draw call anywhere in this loop, deliberately, so every tick's
        // own touch has to survive purely in the accumulator rather than
        // being individually consumed as it happens.
        use crate::sim::update;
        for _ in 0..100 {
            update::step(&mut world);
        }
        assert!(world.chunks().all(|c| c.is_settled()), "the sand must have fully settled by now, or this test isn't exercising the gap");

        let touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &touched, &mut frame, (uw, uh), false);

        let mut baseline = Renderer::new();
        let mut expected = vec![0u8; (uw * uh * 4) as usize];
        baseline.draw(&world, &particles, &HashSet::new(), &mut expected, (uw, uh), true);

        assert_eq!(frame, expected, "the settled pile's real position must render, not a stale mid-fall one left over from the warm-up draw");
    }

    #[test]
    fn a_fully_settled_world_recomputes_nothing_on_the_next_draw() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(30, 30, Cell::new(material::STONE, 0));
        // Two calls, not one: `set` marks `pending_dirty`, and one
        // `end_step` only promotes that into `dirty` (still unsettled) --
        // it takes a second call, with no write in between, to promote the
        // now-empty `pending_dirty` and actually clear `dirty`.
        world.end_step();
        world.end_step();

        let (w, h) = (64u32, 64u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (w, h), true); // warm up
        let touched = world.take_touched_chunks();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (w, h), false);
        assert_eq!(recomputed, 0, "nothing changed since the warm-up, so nothing should be recomputed");
    }

    #[test]
    fn the_very_first_draw_is_always_full_even_with_force_full_false() {
        // `last_zoom_state` starts `None` specifically so a freshly built
        // `Renderer` can never mistake an unwritten frame buffer for one
        // safe to partially reuse.
        let mut world = World::new(Rect::new(0, 0, 15, 15));
        world.set(5, 5, Cell::new(material::STONE, 0));
        let (w, h) = (16u32, 16u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let touched = world.take_touched_chunks();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (w, h), false);
        assert_eq!(recomputed, (w * h) as usize, "the first draw ever must be full regardless of force_full");
    }

    #[test]
    fn changing_zoom_between_draws_forces_one_more_full_redraw() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.end_step();
        let (w, h) = (64u32, 64u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (w, h), true); // warm up
        renderer.zoom = 2;
        let touched = world.take_touched_chunks();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (w, h), false);
        assert_eq!(recomputed, (w * h) as usize, "a zoom change invalidates the whole buffer, settled or not");
    }

    #[test]
    fn panning_between_draws_forces_one_more_full_redraw() {
        // `camera_moved` has fed the `full` predicate since the camera could
        // move at all, and nothing asserted it until the map scroll made it a
        // path taken every frame someone is looking around. Sibling of
        // `changing_zoom_between_draws_forces_one_more_full_redraw`, and for
        // the identical reason: every pixel in the buffer now shows a
        // different world cell. Without it the regression is a frozen, smeared
        // world scrolling past the handful of chunks that happened to be
        // dirty — which is precisely what `draw`'s own comment says this
        // guards.
        let mut world = World::new(Rect::new(0, 0, 255, 255));
        world.end_step();
        let (w, h) = (64u32, 64u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (w, h), true); // warm up

        // Half a second of held key, mid-world so the clamp is not what
        // decides the answer.
        renderer.set_camera(64, 64, (w, h), world.bounds());
        let settled = world.take_touched_chunks();
        renderer.draw(&world, &particles, &settled, &mut frame, (w, h), true);
        let before = renderer.camera_x;
        for _ in 0..30 {
            renderer.pan((1, 0), 1.0 / 60.0, (w, h), world.bounds());
        }
        assert_ne!(renderer.camera_x, before, "half a second of pan moved nothing, so the redraw claim below is vacuous");

        let touched = world.take_touched_chunks();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (w, h), false);
        assert_eq!(recomputed, (w * h) as usize, "a camera move invalidates the whole buffer, settled or not");
    }

    #[test]
    fn a_pan_cannot_escape_the_world_at_any_scale() {
        let world = Rect::new(0, 0, 2047, 639);
        let viewport = (512u32, 320u32);

        // Scroll until the camera is genuinely stuck, rather than for a fixed
        // duration. **A fixed duration silently stopped guarding anything when
        // the rate came down**: 300 frames covered 7.5 screens at the old 1.5
        // screens/s and covers about 2.4 at the new one — less than the 3
        // screens of travel at 1:1, and far less than the 31 screens at zoom
        // 8, which needs a full minute of held key to cross. The bounds
        // assertions would all still have passed, on a camera that never once
        // reached an edge. That is the vacuity trap `CLAUDE.md` keeps
        // recording, and this shape is immune to it: it ends when the clamp
        // stops the camera, whatever the rate.
        //
        // "Stuck" is a whole second of no movement, not one still frame: the
        // sub-cell carry means a legitimately-moving camera sits out
        // individual frames whenever its per-frame share is under a cell,
        // which at zoom 8 is most of them.
        fn scroll_to_the_edge(r: &mut Renderer, dir: (i32, i32), viewport: (u32, u32), world: Rect) {
            let (mut still, mut frames) = (0, 0);
            while still < 60 {
                let was = (r.camera_x, r.camera_y);
                r.pan(dir, 1.0 / 60.0, viewport, Some(world));
                still = if (r.camera_x, r.camera_y) == was { still + 1 } else { 0 };
                frames += 1;
                assert!(frames < 100_000, "the camera never settled against the world edge going {dir:?}");
            }
        }

        // Stride 4 is left out on purpose: its x span is the whole 2048-cell
        // world, so there is exactly one legal camera and every claim below is
        // true by construction rather than by the clamp.
        for (zoom, stride) in [(1, 1), (2, 1), (8, 1), (1, 2)] {
            let mut r = Renderer::new();
            r.zoom = zoom;
            r.zoom_out_stride = stride;
            let (span_x, span_y) = r.visible_span(viewport);
            let scale = format!("zoom {zoom} stride {stride}");

            // Hard into the bottom-right corner, then the top-left one.
            // Asserted as the *exact* clamp limit, not merely "inside the
            // world" — that is what proves the clamp stopped it rather than
            // the scroll happening to run out.
            scroll_to_the_edge(&mut r, (1, 1), viewport, world);
            assert_eq!(
                (r.camera_x, r.camera_y),
                (world.max_x - span_x + 1, world.max_y - span_y + 1),
                "{scale}: scrolling down-right did not pin at the far corner"
            );

            scroll_to_the_edge(&mut r, (-1, -1), viewport, world);
            assert_eq!(
                (r.camera_x, r.camera_y),
                (world.min_x, world.min_y),
                "{scale}: scrolling up-left did not pin at the origin"
            );

            // And the far corner really was somewhere else, so the pair above
            // is not two readings of a camera that never moved.
            assert!(span_x < world.max_x + 1, "{scale}: the viewport spans the whole world; nothing to travel");
        }
    }

    #[test]
    fn a_pan_moves_the_cell_under_a_pixel_by_exactly_the_camera_delta() {
        // The property a player actually sees: the picture *translates*.
        // Asserted through `screen_to_world`, because that is the mapping
        // every pixel of `draw` is painted through — a pan that moved
        // `camera_x` and disagreed with it would scroll the world past a
        // cursor that lands somewhere else entirely.
        let world = Rect::new(0, 0, 2047, 639);
        let viewport = (512u32, 320u32);
        for (zoom, stride) in [(1, 1), (2, 1), (4, 1), (1, 2)] {
            let mut r = Renderer::new();
            r.zoom = zoom;
            r.zoom_out_stride = stride;
            // Mid-world, away from every clamp — and away from the origin,
            // where world and screen coordinates are the same numbers and
            // this would hold for the wrong reason.
            r.set_camera(700, 200, viewport, Some(world));
            let probe = (137, 91);
            let before = r.screen_to_world(probe.0, probe.1);
            let cam_before = r.camera_x;
            for _ in 0..30 {
                r.pan((1, 0), 1.0 / 60.0, viewport, Some(world));
            }
            let moved = r.camera_x - cam_before;
            assert!(moved > 0, "zoom {zoom} stride {stride}: half a second of pan moved nothing");
            assert_eq!(
                r.screen_to_world(probe.0, probe.1),
                (before.0 + moved, before.1),
                "zoom {zoom} stride {stride}: the view did not translate by the camera delta"
            );
        }
    }

    #[test]
    fn a_pan_at_a_stride_steps_whole_strides_only() {
        // A camera step that is not a whole number of strides re-samples the
        // view against a different lattice instead of translating it, and the
        // screen hisses rather than scrolling. Only catchable as an assertion
        // about the *step*: both cases move `camera_x`, so any before/after
        // comparison of the camera alone passes either way.
        //
        // **Unbounded on purpose**, which is the isolation this needs rather
        // than a shortcut. The clamp in `set_camera` lands the camera wherever
        // the world's edge is, which is not on the stride lattice in general —
        // at stride 3 on a 2048-cell world the far stop is x=512, and 512 is
        // not a multiple of 3. That is a *single* frame's lattice shift as the
        // camera pins, not the repeated per-frame alternation this is about,
        // and once pinned the view is static and cannot hiss at all. Including
        // it would make the test fail for a reason it is not named for; the
        // clamp is `a_pan_cannot_escape_the_world_at_any_scale`'s subject.
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        r.zoom_out_stride = 3;
        let mut last = r.camera_x;
        let mut moves = 0;
        for _ in 0..120 {
            r.pan((1, 0), 1.0 / 60.0, viewport, None);
            let step = r.camera_x - last;
            assert_eq!(step % 3, 0, "camera stepped {step} at stride 3, which does not translate the picture");
            if step != 0 {
                moves += 1;
            }
            last = r.camera_x;
        }
        assert!(moves > 0, "two seconds of pan produced no camera step at all");
    }

    /// Scroll one direction for `secs` and report how far the camera moved.
    /// Mid-world and unbounded, so the clamp never truncates a measurement.
    fn scrolled(r: &mut Renderer, dir: (i32, i32), secs: f32, viewport: (u32, u32)) -> i32 {
        let before = r.camera_x;
        for _ in 0..(secs * 60.0) as i32 {
            r.pan(dir, 1.0 / 60.0, viewport, None);
        }
        r.camera_x - before
    }

    #[test]
    fn a_held_scroll_accelerates() {
        // Half the playtest verdict on the flat 1.5 screens/s: it was both too
        // fast to aim and, once slowed, would have been too slow to travel.
        // Only a ramp answers both, so it needs a test that a ramp is actually
        // happening rather than a rate that merely got smaller.
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        let first = scrolled(&mut r, (1, 0), 1.0, viewport);
        let second = scrolled(&mut r, (1, 0), 1.0, viewport);
        assert!(
            second > first,
            "a held scroll must speed up: first second {first} cells, second {second}"
        );

        // And a tap is always a tap: `end_pan` puts the next gesture back at
        // the start of the ramp, so two taps in a row cover the same ground.
        // Without this the test passes against a ramp that never resets, which
        // would make the second nudge of any correction twice the first.
        let mut r = Renderer::new();
        let tap_a = scrolled(&mut r, (1, 0), 0.2, viewport);
        r.end_pan();
        let tap_b = scrolled(&mut r, (1, 0), 0.2, viewport);
        assert_eq!(tap_a, tap_b, "two separate taps must move the same distance");
        assert!(tap_a > 0, "a 200 ms tap moved nothing at all");
    }

    #[test]
    fn reversing_direction_restarts_the_ramp() {
        // The overshoot correction. At full speed after a long hold, a tap
        // back the other way has to be a nudge — inheriting the ramp would
        // turn one overshoot into two, which is the twitchiness complaint
        // reappearing at the moment it matters most.
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        scrolled(&mut r, (1, 0), 3.0, viewport); // wound fully up
        let back = scrolled(&mut r, (-1, 0), 0.2, viewport).abs();

        let mut fresh = Renderer::new();
        let cold = scrolled(&mut fresh, (-1, 0), 0.2, viewport).abs();
        assert_eq!(back, cold, "a reversal after a long hold must nudge like a fresh tap: {back} vs {cold}");

        // But **adding** an axis is not reversing. Pressing `S` while already
        // travelling right must not drop the scroll back to walking pace in
        // the middle of a journey — only turning round does that. Guarded
        // because the obvious spelling of the rule (`dir != last_dir`) gets
        // this wrong, and gets it wrong invisibly: it still scrolls, just
        // sluggishly, every time you adjust your heading.
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        scrolled(&mut r, (1, 0), 3.0, viewport); // wound fully up
        let after_adding_an_axis = scrolled(&mut r, (1, 1), 0.2, viewport);
        let mut still_straight = Renderer::new();
        scrolled(&mut still_straight, (1, 0), 3.0, viewport);
        let uninterrupted = scrolled(&mut still_straight, (1, 0), 0.2, viewport);
        assert_eq!(
            after_adding_an_axis, uninterrupted,
            "adding a second direction restarted the ramp: {after_adding_an_axis} vs {uninterrupted}"
        );
    }

    #[test]
    fn a_sustained_scroll_crosses_the_world_in_about_six_seconds() {
        // The figure the owner actually chose, and the one that was wrong
        // before — so it is the one that earns a guard. Held to the real
        // world's proportions rather than an abstract rate: 1536 cells of
        // pannable width at 1:1, which is what "across the world" means here.
        let world = Rect::new(0, 0, 2047, 639);
        let viewport = (512u32, 320u32);
        let mut r = Renderer::new();
        let mut frames = 0;
        while r.camera_x < world.max_x - 512 + 1 {
            r.pan((1, 0), 1.0 / 60.0, viewport, Some(world));
            frames += 1;
            assert!(frames < 60 * 60, "the scroll never crossed the world at all");
        }
        let secs = frames as f32 / 60.0;
        // Wide enough that retuning the ramp's shape does not trip it, tight
        // enough that the rejected 2.0 s and a sluggish 12 s both fail.
        assert!(
            (4.5..8.0).contains(&secs),
            "a held scroll crossed the world in {secs:.1}s; the target is about six"
        );
    }

    #[test]
    fn the_pan_covers_the_same_fraction_of_a_screen_at_every_zoom() {
        // The whole reason the rate is in screens rather than cells. Compared
        // as a fraction of a viewport, which is the quantity the eye judges;
        // the cell counts differ by 8x across these and are not the claim.
        //
        // This is also the residual-carry test in disguise: drop the carry and
        // the tightest zoom falls well short of the others, because its
        // per-frame share is the one small enough to round away.
        //
        // Asserted **against the other zooms, not against a constant.** It
        // used to compare one second of scroll to `PAN_SCREENS_PER_SECOND`,
        // which stopped being the distance covered the moment the rate ramped
        // — and the invariant this test is named for never mentioned the rate
        // at all. Pinning the absolute figure is
        // `a_sustained_scroll_crosses_the_world_in_about_six_seconds`'s job.
        let world = Rect::new(0, 0, 2047, 639);
        let viewport = (512u32, 320u32);
        let mut travelled = Vec::new();
        for zoom in [1, 2, 4, 8] {
            let mut r = Renderer::new();
            r.zoom = zoom;
            r.set_camera(700, 200, viewport, Some(world));
            let before = r.camera_x;
            for _ in 0..60 {
                r.pan((1, 0), 1.0 / 60.0, viewport, Some(world));
            }
            let (span_x, _) = r.visible_span(viewport);
            travelled.push((zoom, (r.camera_x - before) as f32 / span_x as f32));
        }
        // Non-zero first, or "all equal" is satisfied by a pan that never
        // moves anything.
        for (zoom, screens) in &travelled {
            assert!(*screens > 0.05, "zoom {zoom} barely moved: {screens:.3} screens in a second");
        }
        let (_, reference) = travelled[0];
        for (zoom, screens) in &travelled {
            assert!(
                (screens - reference).abs() < 0.03,
                "zoom {zoom} covered {screens:.3} of a screen against zoom 1's {reference:.3}: {travelled:?}"
            );
        }
    }

    #[test]
    fn a_nonempty_particle_system_forces_a_full_redraw_every_frame() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.end_step();
        let (w, h) = (64u32, 64u32);
        let mut particles = ParticleSystem::new();
        particles.spawn(5.0, 5.0, 0.0, 0.0, material::SAND, 0);
        let mut renderer = Renderer::new();
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (w, h), true); // warm up
        let touched = world.take_touched_chunks();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (w, h), false);
        assert_eq!(recomputed, (w * h) as usize, "particles have no tracked footprint, so their presence must force a full redraw");
    }

    #[test]
    fn a_pending_field_overlay_forces_a_full_redraw_every_frame() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.end_step();
        let (w, h) = (64u32, 64u32);
        let particles = ParticleSystem::new();
        let mut renderer = Renderer::new();
        renderer.field_overlay = FieldOverlay::Pressure;
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let warm_up_touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &warm_up_touched, &mut frame, (w, h), true); // warm up
        let touched = world.take_touched_chunks();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (w, h), false);
        assert_eq!(recomputed, (w * h) as usize, "the field grid diffuses independent of chunk settledness, so its overlay must bypass the skip");
    }
}
