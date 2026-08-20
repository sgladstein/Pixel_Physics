//! Cells to pixels.
//!
//! The simulation never writes colours. It stores a material id and a shade
//! index, and this module resolves those into RGBA at draw time. Keeping the
//! two apart is what lets M6 swap in a GPU pipeline with lighting and bloom
//! without touching a single movement rule.

use std::collections::HashSet;

use crate::sim::cell::AMBIENT_TEMPERATURE;
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
const GNOME_SPRITE: [[Option<[u8; 4]>; 7]; 14] = {
    // Local aliases, so the table below stays a picture you can read.
    const H: Option<[u8; 4]> = Some(GNOME_HAT);
    const F: Option<[u8; 4]> = Some(GNOME_FACE);
    const W: Option<[u8; 4]> = Some(GNOME_BEARD);
    const T: Option<[u8; 4]> = Some(GNOME_TUNIC);
    const L: Option<[u8; 4]> = Some(GNOME_BELT);
    const B: Option<[u8; 4]> = Some(GNOME_BOOT);
    const X: Option<[u8; 4]> = None;
    [
        [X, X, X, H, X, X, X],
        [X, X, H, H, H, X, X],
        [X, H, H, H, H, H, X],
        [H, H, H, H, H, H, H],
        [X, X, F, F, F, X, X],
        [X, F, F, F, F, F, X],
        [X, F, W, W, W, F, X],
        [X, X, W, W, W, X, X],
        [T, T, T, T, T, T, T],
        [T, T, T, T, T, T, T],
        [X, T, T, L, T, T, X],
        [X, T, T, T, T, T, X],
        [X, B, B, X, B, B, X],
        [B, B, B, X, B, B, B],
    ]
};

pub struct Renderer {
    /// Which `GrainMode` a `Liquid` cell's brightness grain comes from.
    /// Prototype switch — see the enum's own doc.
    pub grain: GrainMode,
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
    /// Screen rect the gnome occupied on the previous `draw` — the same
    /// smear-repaint reasoning as `last_body_rects`. Kept separately, and
    /// compared before unioning: a gnome standing still contributes
    /// *nothing* to the dirty region, which is what keeps a settled
    /// world's zero-cost frames zero-cost with a character idle in them.
    last_player_rect: Option<Rect>,
    /// World coordinate displayed at the top-left pixel. Moved by
    /// [`Renderer::follow`] once there is a player to follow.
    pub camera_x: i32,
    pub camera_y: i32,
    /// The camera as last painted, so a move can force a full redraw. Same
    /// role as `last_zoom_state`, and for the same reason.
    last_camera: Option<(i32, i32)>,
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
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            grain: GrainMode::default(),
            frame: 0,
            last_body_rects: Vec::new(),
            last_player_rect: None,
            camera_x: 0,
            camera_y: 0,
            last_camera: None,
            show_chunk_overlay: false,
            zoom: 1,
            zoom_out_stride: 1,
            field_overlay: FieldOverlay::Off,
            organism_overlay: OrganismOverlay::Off,
            last_organism_overlay: OrganismOverlay::Off,
            last_zoom_state: None,
            sky: Sky::at(0, 0, 1, 0, 1),
            last_sky_key: None,
            daylight: sky::LIGHT_LEVELS,
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
        }
    }

    /// `F10` — toggle the terrain depth light, so the review's largest
    /// graphics change can be judged as an A/B in the running app against
    /// the pre-review look. Same convention as `cycle_grain` below.
    pub fn cycle_terrain_light(&mut self) {
        self.terrain_light = self.terrain_light.next();
    }

    /// `G` — step through the liquid grain modes. This exists so the
    /// variants can be judged on real moving water in the real app, which is
    /// the only way a "does this look right" question gets answered, and it
    /// stays for the same reason: the look is expected to keep being
    /// iterated on rather than settled once.
    pub fn cycle_grain(&mut self) {
        self.grain = self.grain.next();
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

        if let Some(b) = bounds {
            // `max` before `min` so a world narrower than the viewport pins to
            // its own origin rather than to a negative coordinate.
            cam_x = cam_x.min(b.max_x - span_x + 1).max(b.min_x);
            cam_y = cam_y.min(b.max_y - span_y + 1).max(b.min_y);
        }
        self.camera_x = cam_x;
        self.camera_y = cam_y;
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
        self.sky = Sky::at(world.frame, vx0, vx1.max(vx0 + 1), vy0, vy1.max(vy0 + 1))
            .muted(sky::overcast(weather.intensity));
        self.daylight = sky::daylight_level(world.frame);
        self.rebuild_horizon(world);
        let sky_key = self.sky.key();
        let sky_changed = self.last_sky_key != Some(sky_key);
        self.last_sky_key = Some(sky_key);

        let full = force_full
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
            || weather.is_precipitating()
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
        let player_rect = world.player.as_ref().and_then(|p| {
            let (x0, y0, x1, y1) = p.bounds();
            self.world_rect_to_screen_rect(Rect::new(x0, y0, x1, y1), width, height)
        });

        let recomputed = if full {
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let sx = (i % width as usize) as i32;
                let sy = (i / width as usize) as i32;
                let (wx, wy) = self.screen_to_world(sx, sy);
                let colour = self.cell_colour(world, wx, wy);
                pixel.copy_from_slice(&colour);
            }
            self.last_player_rect = player_rect;
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
            if player_rect != self.last_player_rect {
                for r in player_rect.iter().chain(self.last_player_rect.iter()) {
                    dirty = Some(match dirty {
                        Some(d) => d.union(*r),
                        None => *r,
                    });
                }
                self.last_player_rect = player_rect;
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
        self.draw_precipitation(weather, world.frame, (vx0, vy0), (vx1, vy1), frame, width, height);
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
        for drop in sky::drops(world_frame, fall, weather.intensity, weather.wind, (vx0, vy0), (vx1, vy1)) {
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
        for (dy, row) in GNOME_SPRITE.iter().enumerate() {
            for (dx, colour) in row.iter().enumerate() {
                let Some(colour) = colour else { continue };
                let Some((sx, sy)) = self.world_to_screen(ox + dx as i32, oy + dy as i32) else {
                    continue;
                };
                for by in 0..block {
                    for bx in 0..block {
                        put(frame, width, height, sx + bx, sy + by, *colour);
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
            return;
        };
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
    /// Asks whether the cell is above where the ground has ever reached in
    /// this column, **not** whether there is currently a clear path up. The
    /// two differ exactly down a freshly dug shaft, and the second answer is
    /// the one that renders a mine as a strip of sky.
    fn under_sky(&self, x: i32, y: i32) -> bool {
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
            let daylight = self.sky.colour_at(x, y);
            let depth = self.sky_depth(x, y);
            base = if depth < 0 {
                daylight
            } else {
                // Inside the ground: unlit rock, not daylight. Constant
                // rather than following the sky, because a cave is dark at
                // noon as well -- and deliberately distinct from the `VOID`
                // outside the world, which is a different kind of nothing.
                //
                // Faded in over `CAVE_FADE_DEPTH` rather than switched at
                // the boundary. Light reaches in through an opening and
                // falls off; cutting instead put a black rectangle behind
                // every roof and made a cave mouth a cutout rather than an
                // opening.
                let t = self.cave_ramp[depth.clamp(1, CAVE_FADE_DEPTH) as usize] as u16;
                let mix = |a: u8, b: u8| ((a as u16 * (255 - t) + b as u16 * t) / 255) as u8;
                [
                    mix(daylight[0], UNDERGROUND[0]),
                    mix(daylight[1], UNDERGROUND[1]),
                    mix(daylight[2], UNDERGROUND[2]),
                    255,
                ]
            };
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
        let rgb = sky::apply_light(rgb, self.daylight, self.sky.ambient());
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
                let depth = self.light_depth(x, y);
                if depth < 0 {
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
            FieldOverlay::Temperature => {
                let f = world.field_at(x, y);
                let t = ((f.temperature - AMBIENT_TEMPERATURE as f32) / TEMPERATURE_OVERLAY_MAX).clamp(0.0, 1.0);
                ([255.0, 140.0, 40.0], t)
            }
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
        for overlay in [FieldOverlay::Pressure, FieldOverlay::Temperature, FieldOverlay::Light, FieldOverlay::Moisture] {
            renderer.field_overlay = overlay;
            assert_eq!(renderer.cell_colour(&world, 50, 50), off, "{overlay:?} tinted an unaffected cell far from any real disturbance");
        }
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
