//! Material definitions, loaded from `assets/materials/*.ron`.
//!
//! Behaviour comes from `kind` plus numeric parameters, so adding a material
//! never means adding a branch to the update loop. The files are the single
//! source of truth: they are embedded in the binary as defaults, and re-read
//! from disk on hot reload.
//!
//! Only vacuum and the out-of-bounds wall are built in, because the engine
//! itself refers to them.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use super::rng;

/// Index into `MaterialRegistry`. Stored in every cell, so it must stay small.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MaterialId(pub u16);

pub const EMPTY: MaterialId = MaterialId(0);
pub const BEDROCK: MaterialId = MaterialId(1);

/// A `Liquid`-kind `Cell`'s fill amount lives in `aux`, fixed-point on this
/// scale. `0` is the one value `aux` can hold that does *not* mean "no
/// liquid" — a freshly created `Liquid` cell (painted, or a test built via
/// `Cell::new` directly) has never been through the transfer logic that
/// would give it a real reading, so it is treated as full by convention
/// rather than requiring every liquid-creation call site in the codebase
/// (the paint brush, phase changes, reactions, every existing test) to
/// remember to set it explicitly. `update.rs`'s transfer logic never leaves
/// a genuinely-drained cell at `aux == 0` — it converts to `Cell::EMPTY`
/// instead — so `aux == 0` on a `Liquid` cell is unambiguous: it always
/// means "untouched since creation," never "empty."
pub const LIQUID_FULL: u16 = 1000;
/// A `Powder` cell's held water, fixed-point on this scale in its `aux`.
///
/// **The convention is inverted relative to `LIQUID_FULL` above, and that
/// is the single easiest thing here to get backwards.** For a `Liquid`,
/// `aux == 0` means *full* (see that constant's own doc for why). For a
/// `Powder`, **`aux == 0` means dry** — which is what worldgen, the brush
/// and every existing test already produce for free, so soil starts dry
/// rather than every soil-creating call site having to say so.
///
/// `Reports/plant-substrate-v2-design.md` §4a calls this out specifically
/// as the bug to avoid, since `LIQUID_FULL`'s own doc exists because the
/// same confusion already bit once on the liquid side.
pub const SOIL_SATURATED: u16 = 1000;

/// Water held against gravity after free drainage — **field capacity**,
/// conventionally the content at a matric potential of −33 kPa. Soil above
/// this drains downward; soil at or below it holds what it has.
pub const SOIL_FIELD_CAPACITY: u16 = 620;

/// **Permanent wilting point**, −1500 kPa: the content below which most
/// plants can no longer extract water at all. `Absorb` credits nothing here,
/// which is what makes drought a real terminal failure rather than a slow
/// one.
///
/// Field capacity minus this is **plant available water**, the only band a
/// plant actually drinks from. Both breakpoints, and the framework tying
/// them to the penetration bound in `penetration_resistance`, are the least
/// limiting water range: Da Silva, Kay & Perfect (1994), SSSAJ
/// 58:1775-1781, refining Letey (1985).
pub const SOIL_WILTING_POINT: u16 = 180;

/// How much a `Liquid` cell may hold above `LIQUID_FULL` when the cell below
/// it is also full — the pressure signal that lets a compressed column push
/// sideways into a shorter neighbour even once every cell in it individually
/// reads as full. Small on purpose (1%), matching the falling-sand
/// compressible-volume technique this is drawn from (see `update.rs`'s
/// module doc): enough to carry a signal, not enough to visibly bulge.
pub const LIQUID_MAX_COMPRESS: u16 = 10;

/// How far `transfer_liquid_horizontal` (`update.rs`) looks past the
/// immediate neighbour for a genuinely emptier cell to level against, and
/// therefore also `Liquid`'s answer to [`Material::sweep_reach`]. Lives here
/// rather than in `update.rs` because `sweep_reach` (needed by `chunk.rs`,
/// which must not depend on `update.rs` — that would make `chunk` and
/// `update` mutually dependent modules) needs the same number `update.rs`'s
/// transfer logic already uses, and this is the lower-level module of the
/// two, matching how `LIQUID_FULL`/`LIQUID_MAX_COMPRESS` above already live
/// here for the same reason. See `update.rs`'s own doc on this constant for
/// the mechanism itself.
///
/// **24, raised from 8**, because this turned out to be what governs the
/// complaint that a wide body "has a tilt across the whole screen". Measured
/// on a 512-wide pour, waterline tilt end to end and the frame everything
/// finally sleeps:
///
/// | reach | tilt at frame 2000 | asleep at | tilt at rest |
/// |---|---|---|---|
/// | 8 | 18 cells | 12,153 | 1 cell |
/// | 16 | 15 | 5,867 | 1 |
/// | 24 | 14 | 4,464 | 1 |
/// | 32 | 9 | 3,202 | 1 |
///
/// The tilt is not permanent at any of these — every one settles to a single
/// cell across 510 columns. What differs is *how long it takes to get there*,
/// and at 8 that was three and a half minutes of visibly-sloped water that
/// looked settled because almost nothing was moving.
///
/// Note this is a strictly better lever than `Material::min_transfer` for the
/// same complaint, and the two were easy to confuse: widening the dead band
/// also sleeps sooner, but it does it by *giving up* on the last of the
/// levelling, so tilt at rest goes 1 -> 3 -> 5 cells. Widening this reach
/// costs no accuracy at all.
///
/// Stopped at 24 rather than 32: 32 is faster again but measured ~12.3 ms on
/// the stress scene against ~8.7 at 8, where 24 costs ~9.3 ms (+7%). 24 also
/// keeps a margin under `MAX_REACH`, which `parallel.rs`'s write-disjointness
/// proof is pinned to, rather than sitting exactly on it.
pub const HORIZONTAL_TRANSFER_REACH: i32 = 24;

/// How far a `Liquid` cell may look sideways for a column it can actually
/// fall from — `update::find_lateral_descent`'s own bound, and the
/// mechanism that makes water level in seconds rather than holding a slope.
///
/// This is a different question from `HORIZONTAL_TRANSFER_REACH` above,
/// which bounds how far *fill* is equalised between neighbours. That is
/// diffusion: it moves a fraction of a difference and converges in
/// O(width²). This bounds how far a whole cell may *travel* to reach lower
/// ground, which is ballistic and converges in roughly O(width / reach).
/// Both external precedents for this engine's own architecture use a large
/// value here: The Powder Toy searches up to 30 cells sideways in one frame
/// (`rt`, `Simulation.cpp`), and Noita — whose 64×64 chunks and four-pass
/// checkerboard are the same design as this engine's — permits a pixel to
/// move within its own chunk plus 32 cells cardinally, which is exactly
/// `MAX_REACH` here and the bound `parallel.rs`'s write-disjointness proof
/// already rests on. Set to that bound rather than under it, because the
/// entire reason water read as sand was a search too short to find the
/// bottom of a pile it was sitting on.
pub const LIQUID_LATERAL_REACH: i32 = 24;

/// Well-known ids for the shipped materials.
///
/// These are stable because `builtin` always runs first and assigns ids in
/// `EMBEDDED` order, and `reload` only ever updates a material in place or
/// appends a new one — it never reassigns. They exist for the convenience of
/// engine code and tests; anything data-driven should use [`MaterialRegistry::id_of`]
/// instead. `well_known_ids_match_their_names` guards the correspondence.
pub const STONE: MaterialId = MaterialId(2);
pub const SAND: MaterialId = MaterialId(3);
pub const GRAVEL: MaterialId = MaterialId(4);
pub const ASH: MaterialId = MaterialId(5);
pub const WATER: MaterialId = MaterialId(6);
pub const OIL: MaterialId = MaterialId(7);
pub const SMOKE: MaterialId = MaterialId(8);
/// What stone breaks into (`stone.ron`'s `breaks_into`). Numbered from its
/// position at the *end* of `EMBEDDED`, which is the only place a new
/// material may be added — see that array's own comment.
pub const RUBBLE: MaterialId = MaterialId(15);

/// Determines which movement rule a cell obeys. Everything else is parameters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub enum MaterialKind {
    /// Vacuum. Anything may move into it.
    Empty,
    /// Never moves and is never displaced.
    Solid,
    /// Falls, and piles up at its angle of repose. Sand, gravel, ash.
    Powder,
    /// Falls, and spreads sideways to find its level. Water, oil.
    Liquid,
    /// Rises, and spreads to fill space. Smoke, steam.
    Gas,
    /// Never moves on its own, like `Solid` -- but unlike inert rock, a
    /// plant cell's changes come from the M16 active-site scheduler, not
    /// the CA sweep's movement dispatch. Kept distinct from `Solid` so
    /// `Cell::aux`'s interpretation (growth stage, not anchor distance) and
    /// future flammability/structural rules can key on "is this alive"
    /// without guessing from the material name.
    Plant,
    /// Never moves via the CA sweep either -- like `Plant`, a creature cell
    /// is relocated explicitly by the M18 active-site scheduler (`creature.rs`),
    /// which reads/writes it through the ordinary `World::get`/`set`, not
    /// through the movement dispatch below. `Cell::aux` holds the owning
    /// creature's id (see `cell.rs`), distinct from `Plant`'s growth stage.
    Creature,
}

impl MaterialKind {
    /// Whether another material can displace this one by swapping with it.
    /// Solids anchor the world; empty space is moved into rather than swapped.
    pub fn is_displaceable(self) -> bool {
        matches!(self, MaterialKind::Liquid | MaterialKind::Gas)
    }

    /// Whether the update loop needs to consider this cell at all.
    pub fn is_mobile(self) -> bool {
        matches!(
            self,
            MaterialKind::Powder | MaterialKind::Liquid | MaterialKind::Gas
        )
    }
}

/// One `.ron` file. Everything optional has a sensible default so a file only
/// has to state what makes its material distinctive.
#[derive(Deserialize)]
pub struct MaterialDef {
    pub name: String,
    /// Shown in the picker. Left out, the name is title-cased — an `Option`
    /// here would force every file to write `display: Some("...")`.
    #[serde(default)]
    pub display: String,
    pub kind: MaterialKind,
    pub density: f32,
    /// Angle of repose in degrees, for powders — the shallower angle a pile
    /// *already flowing* settles at. 45 is the steepest a pile can hold;
    /// lower values spread flatter. Ignored by other kinds.
    #[serde(default = "default_friction_angle")]
    pub friction_angle: f32,
    /// The steeper angle a *settled* pile can stand at without creeping,
    /// degrees. `Reports/granular-mechanics-research.md` §2: real granular
    /// piles show hysteresis — harder to start an avalanche than to keep one
    /// going — which a single angle cannot express. 0.0 (the default for any
    /// `.ron` that doesn't set this) means "unset": resolved to
    /// `friction_angle + DEFAULT_STABILITY_ANGLE_GAP_DEGREES` in
    /// `Material::from`. Set explicitly only for a powder whose real-world
    /// repose/stability gap is unusually wide or narrow.
    #[serde(default)]
    pub max_stability_angle: f32,
    /// How far a gas may travel sideways in one step. `Liquid` kind ignores
    /// this — see `flow_rate`, the compressible-volume equivalent.
    #[serde(default)]
    pub dispersion: u8,
    /// How much of a `Liquid` cell's fill (on the `LIQUID_FULL` = 1000
    /// scale) may transfer to a neighbour in one tick, in both the vertical
    /// settle and the horizontal levelling pass. Ignored by every other
    /// kind. Higher means water-like (settles fast, looks thin); lower
    /// means honey-like (settles slowly, holds a visible slope for a while
    /// even though it is still, eventually, headed for flat).
    #[serde(default)]
    pub flow_rate: u16,
    /// Below this fill difference, two horizontally adjacent cells of this
    /// liquid count as settled rather than continuing to trade small amounts
    /// back and forth as their neighbours slowly finish levelling.
    /// `flow_rate`'s natural sibling: that one caps how much moves per tick,
    /// this one is the threshold under which nothing moves at all.
    ///
    /// **Was a hardcoded `update::MIN_LIQUID_TRANSFER`.** Its history is
    /// worth keeping, because it is a history of the same trade being
    /// re-decided as the surrounding mechanisms changed. Originally tuned
    /// *up* to 150 (15% of `LIQUID_FULL`) because 8 left a wide test puddle
    /// still visibly settling after ~12,000 frames — but Report B
    /// (`Reports/liquid-simulation-research-r2.md`) §3c's own instruction was
    /// to "treat this number as the diagnostic, not the setting", and it came
    /// back down to 16 (2%) once `find_lateral_descent` took over bulk
    /// transport, measured against a 100-column scene where 8 missed the
    /// 20-unit/300-frame bar and 16 cleared it.
    ///
    /// **Re-measured since, and the trade looks different again**, because
    /// that bar was an *exactness* bar and this project does not want
    /// exactness (`CLAUDE.md`). Levelling time to flat, against the visible
    /// tilt left behind:
    ///
    /// | value | 200 columns | 400 columns | visible tilt |
    /// |---|---|---|---|
    /// | 16 | 1546 frames | 6354 | 0 cells |
    /// | 60 | 747 | 2751 | 1–2 cells |
    /// | 100 | 620 | 2072 | 2–4 cells |
    ///
    /// Roughly halving the settling time — and so the number of frames a
    /// pool keeps its chunks awake — for a surface slope around 0.5%, which
    /// is not visible. The old warning that a wide band gives water "a 1:4
    /// angle of repose" predates `find_lateral_descent` and no longer holds.
    ///
    /// Which side of that is right is a *look* judgement rather than a
    /// derivable number, which is why it lives here now: a hot-reloadable
    /// per-material field with a live tunables entry (`O`), so it can be
    /// swept on real water instead of argued about. The default stays 16.
    #[serde(default = "default_min_transfer")]
    pub min_transfer: u16,
    /// How strongly a partially-filled cell of this liquid is dimmed toward
    /// black, 0..1. `0.0` draws every cell at full brightness regardless of
    /// fill (pure occupancy, the pre-`eb8d427` look); `0.65` is what a
    /// hardcoded `MIN_LIQUID_BRIGHTNESS` of 0.35 used to give.
    ///
    /// Exposed because it turns out to drive two reported visual problems at
    /// once, and neither is a physics problem. At rest the surface *geometry*
    /// is flat — measured across 60 columns of a settled pool, the waterline
    /// sits on three adjacent rows — but the top row's fill ranges from 286
    /// to 1002, which at 0.65 draws as anywhere from 54% to 100% brightness.
    /// The waterline reads as a mottled band rather than a clean edge. The
    /// same thing makes the boundary between moving and still water visible:
    /// moving water carries ~6.5% partial cells against ~1% at rest, so the
    /// interface is a brightness discontinuity.
    ///
    /// Only ~1% of a settled body's cells are partial at all — but they are
    /// *entirely* at the surface, which is exactly where the eye looks.
    #[serde(default = "default_fill_dimming")]
    pub fill_dimming: f32,
    /// Mechanical resistance a root tip has to overcome to grow through
    /// this material, in **MPa**, so the number means something outside
    /// this engine. A `RootTip` displaces a `Powder` cell in place when
    /// this is below the species' own limit; anything at or above it stops
    /// root elongation, and `Solid`-kind material is never penetrable
    /// regardless (`plant.rs`'s `Grow`).
    ///
    /// **Real units, and a real threshold.** The dry-end bound of the least
    /// limiting water range is set at a penetrometer resistance of **2-3
    /// MPa**, above which root elongation effectively stops — Da Silva,
    /// Kay & Perfect (1994), *Characterization of the Least Limiting Water
    /// Range of Soils*, SSSAJ 58:1775-1781, which is the same framework
    /// `Reports/plant-substrate-v2-design.md` §4b uses for soil moisture.
    /// So the numbers here are calibrated against a published bound rather
    /// than invented: loose `soil` sits well under it, compacted `gravel`
    /// well over.
    ///
    /// **Why not just use `density`**, which the design doc offers as the
    /// cheaper option: density and penetration resistance are genuinely
    /// different properties, and conflating them means a material cannot be
    /// dense and loose (wet sand) or light and hard (pumice) without one
    /// of the two behaviours coming out wrong. This is also exactly the
    /// kind of gameplay-facing value `design-philosophy.md` §2a says
    /// graduates to `.ron` data immediately.
    ///
    /// Defaults high, so a material that says nothing is impenetrable.
    /// That is the safe direction: a new material silently letting roots
    /// eat through it would be discovered as roots growing through a floor.
    #[serde(default = "default_penetration_resistance")]
    pub penetration_resistance: f32,
    /// Most water this `Powder` can hold in its own pore space, on
    /// `SOIL_SATURATED`'s scale. **`0` — the default — means it holds none
    /// at all**, and a material that holds none never absorbs an adjacent
    /// `Liquid`.
    ///
    /// Opt-in rather than universal, and that is a deliberate scope
    /// decision rather than a modelling claim. Real sand genuinely does
    /// hold water, and turning that on here is a one-line data change. But
    /// making *every* `Powder` absorb liquid silently changed what the
    /// engine's own conservation tests measure — `nothing_escapes_the_
    /// world` and `a_full_multi_chunk_world_of_sand_and_water_settles_
    /// under_the_parallel_sweep` both failed, correctly, because water
    /// entering sand left the liquid tally without anything accounting for
    /// where it went. Water inside soil is *stored*, not destroyed, but
    /// nothing outside this field knows that yet, so the honest move is to
    /// scope the mechanic to the material the plant work actually needs
    /// and leave the rest of the engine's mass bookkeeping untouched.
    ///
    /// Widening it later means teaching those tallies about held water
    /// first. Flagged rather than done.
    #[serde(default)]
    pub water_capacity: u16,
    /// Whether standing cells of this `Liquid` dry up into the air above
    /// them — `evaporation.rs`.
    ///
    /// Opt-in and off by default, so lava, oil and anything added later
    /// keeps its volume unless it says otherwise, and so the flag reads at
    /// the CA sweep's own dispatch site as a `Vec` index on a `Material` the
    /// arm has already resolved, rather than an `id_of("water")` string hash
    /// per liquid cell per frame (`CLAUDE.md`, "guard hot-path work at the
    /// call site that already has the data").
    ///
    /// Meaningless on any kind but `Liquid`; nothing dispatches it
    /// elsewhere.
    #[serde(default)]
    pub evaporates: bool,
    /// Whether this material reinforces a `Powder` it is embedded in, so
    /// that grain no longer falls — the Wu-Waldron apparent-cohesion effect
    /// roots have on soil (`update.rs`'s `root_reinforced`).
    ///
    /// A flag rather than a name lookup for a measured reason: the first
    /// version asked `materials().id_of("rootwood")`, which is a string
    /// hash, **once per powder cell per frame**. A `bool` on the material
    /// the neighbour already resolves to is a `Vec` index instead.
    #[serde(default)]
    pub reinforces_powder: bool,
    pub colors: Vec<[u8; 3]>,

    // --- M14: heat, combustion, phase change and reactions -----------------
    //
    // All temperatures are Celsius, the same unit `Cell::temperature` and
    // `FieldCell::temperature` already use — one unit throughout, no
    // conversion anywhere a temperature crosses a boundary.
    /// Chance to ignite, checked once per burning neighbour per step. 0 means
    /// never catches fire. Kept in the same 0..1 probability space as
    /// `Rng::chance` rather than TPT's 0..1000 integer scale — nothing else
    /// in this engine uses an integer probability, and matching the rest of
    /// the codebase mattered more here than matching the reference exactly.
    #[serde(default)]
    pub flammability: f32,
    /// Temperature at which this material can catch fire on its own, without
    /// a burning neighbour — contact with lava, standing in a fire's heat.
    /// Defaults effectively to "never" so a material that only defines
    /// `flammability` still needs a neighbour actually on fire to ignite it,
    /// rather than spontaneously combusting the moment any warm cell touches it.
    #[serde(default = "default_never")]
    pub ignition_temperature: f32,
    /// Temperature this material radiates while it itself is burning. Feeds
    /// both its own cell temperature and the M13 ambient field once M14 wires
    /// the coupling in.
    #[serde(default = "default_never")]
    pub burn_temperature: f32,
    /// How long this material burns for, in frames, once ignited. Stored as
    /// the countdown in `Cell::ignite`.
    #[serde(default)]
    pub burn_duration: u16,
    /// Per-cell heat diffusion rate to CA neighbours. Must stay under the 2D
    /// explicit-diffusion stability bound (Fourier number ≤ 0.25) — this is
    /// the CA-resolution equivalent of `field::HEAT_DIFFUSION_RATE`, and
    /// needs the same margin the field grid keeps, not more, since the fine
    /// grid has no spare stability budget to give up.
    #[serde(default = "default_heat_conductivity")]
    pub heat_conductivity: f32,
    #[serde(default = "default_never")]
    pub melting_point: f32,
    /// What this material becomes above `melting_point`, or empty for "does
    /// not melt" — same `String`-not-`Option<String>` choice as `display`
    /// above, for the same reason: RON requires an explicit `Some(...)`
    /// wrapper for a present `Option` value (`melts_into: "ash"` alone is a
    /// parse error, `Some("ash")` is required), which is friction with no
    /// payoff here. An empty string is not a valid material name regardless,
    /// so it is an unambiguous "unset". `melting_point` and `melts_into` stay
    /// independent fields precisely so a `.ron` file can set one without the
    /// other while iterating on content without it silently doing something.
    #[serde(default)]
    pub melts_into: String,
    #[serde(default = "default_never")]
    pub boiling_point: f32,
    #[serde(default)]
    pub boils_into: String,
    /// The downward counterpart of `melting_point`/`boiling_point`: the
    /// temperature at or *below* which this material transitions down a
    /// phase. One generic pair rather than separate freeze/condense fields,
    /// because a material has at most one phase below it — for a gas this
    /// reads as its condensation point (steam → water), for a liquid as its
    /// freezing point (water → ice). Defaults to `NEG_INFINITY`, not
    /// `INFINITY` — the sentinel has to sit on the *unreachable-cold* side
    /// for a `<=` threshold, where `default_never`'s `INFINITY` would make
    /// every unset material "condense" on its first visit.
    #[serde(default = "default_never_cold")]
    pub cooling_point: f32,
    /// What this material becomes at or below `cooling_point`, or empty for
    /// "does not transition" — same `String`-not-`Option` convention as
    /// `melts_into` above, for the same RON-friction reason.
    #[serde(default)]
    pub cools_into: String,
    /// A temperature this material *pins itself to* every visit, after
    /// diffusion — the "lava is hot because it is lava" field `fire.rs`'s
    /// M14 notes deferred. A cell of such a material never cools by
    /// diffusion (its neighbours do the cooling instead, by conducting heat
    /// away from the pinned boundary), so it keeps its own chunk awake for
    /// as long as it exists — acceptable for rare, self-limiting content
    /// like lava (which quenches to stone on contact with water), ruinous
    /// for anything bulk. Defaults to unset (`INFINITY`, read via
    /// `is_finite`), which costs nothing.
    #[serde(default = "default_never")]
    pub intrinsic_temperature: f32,
    /// What this material becomes when its burn timer (`burn_duration`)
    /// finishes, or empty to simply extinguish and revert to being itself,
    /// unchanged. Deliberately a separate field from `melts_into` rather than
    /// reusing it: melting is triggered by *ambient temperature* crossing a
    /// threshold, independent of whether anything is on fire, while burnout
    /// is triggered by a *combustion timer* completing. Oil finishing a burn
    /// and becoming ash is not melting in any sense the `melting_point` field
    /// is meant to capture, and conflating the two would make one of them lie
    /// about why the transformation happened.
    #[serde(default)]
    pub burns_into: String,
    /// Pairwise reactions with a specific other material — water quenching
    /// lava into stone and steam, that kind of thing. Order matters: `self`
    /// becomes `produces.0`, `with` becomes `produces.1`.
    #[serde(default)]
    pub reactions: Vec<ReactionDef>,

    // --- M17: structural integrity ------------------------------------
    /// How far a `Solid` cell may sit from an anchor (bedrock or the world
    /// edge — see `structural.rs`) along a chain of other `Solid` cells
    /// before it counts as unsupported and breaks free. Left unset
    /// (effectively infinite), a material never breaks regardless of how
    /// disconnected it becomes from any anchor — the same "never fires
    /// unless a content author opts in" default `ignition_temperature`
    /// uses, for the same reason: existing world-generated terrain (a
    /// floor thicker than a small span, floating decorative ledges with no
    /// path to an anchor at all) would otherwise start crumbling the
    /// moment this milestone shipped, surprising rather than demonstrating
    /// anything. Terrain *is* checked now — `structural::
    /// compute_world_distances` relaxes it at generation — so this value is
    /// live against generated terrain rather than only against what a
    /// player builds. It no longer has to carry the whole model on its own,
    /// though: `attached_span_bonus` carries the background mass, leaving
    /// this to mean specifically "how far loose foreground material
    /// reaches."
    #[serde(default = "default_never_u16")]
    pub max_unsupported_span: u16,
    /// What an unsupported cell becomes once it breaks free, or empty to
    /// leave it Solid regardless of `max_unsupported_span` (the same
    /// unset-name-is-a-no-op pattern `melts_into`/`burns_into` use). Loose
    /// material, not a coherent falling chunk — M8 upgrades that later
    /// without this milestone needing to change.
    #[serde(default)]
    pub breaks_into: String,

    /// Whether a `Liquid` directly beneath this material holds it up —
    /// **buoyancy, stated as data because the load model cannot infer it.**
    ///
    /// `structural.rs`'s and `load.rs`'s "is it standing on the ground"
    /// predicates accept a `Powder` below as support and deliberately
    /// exclude `Liquid`, with the reason written down in both: *"floating
    /// is buoyancy, not support, and nothing here models it."* That was
    /// true and cost nothing while no material floated. Ice does: a sheet
    /// on a pond has no solid under any part of it, so its only path to an
    /// anchor runs sideways to the shore, and the middle of an ordinary
    /// pond is past any span worth giving a thin crust. Worse, a sheet
    /// cannot even *form*: freezing is per cell, so the first cell to
    /// freeze mid-pond is a lone solid with no anchor path at all, at any
    /// finite span, and it is taken apart the frame after it appears.
    /// Measured on `scene=coldsnap` before this field existed: 3,969
    /// freezes over one storm and never more than ten cells of ice
    /// standing at once, with 382 unsupported failures. Freeze-over could
    /// not happen.
    ///
    /// The alternative was leaving `max_unsupported_span` unset, and it is
    /// worse than it looks: `load::capacity` reads that sentinel as *"this
    /// material does not participate in the structural system at all"* and
    /// returns infinite capacity, so ice would also stop failing under
    /// load and an ice bridge would hang in the air with nothing under it.
    /// Exempting it is not the same as floating it.
    ///
    /// So this is `CLAUDE.md`'s "when a rule must tell apart two things
    /// that can look identical, state the difference as data" applied
    /// literally: geometry cannot distinguish a sheet floating on water
    /// from a cantilever reaching out over it, and a bit on the material
    /// can. Opt-in and defaulting to `false`, so **no material that does
    /// not ask for it can change behaviour** — which is what makes this
    /// safe to land in the load model at all (see `CLAUDE.md` on seed
    /// sweeps before changing anything that governs procedural content:
    /// the sweep is unchanged to the cell, because nothing generated is
    /// buoyant).
    ///
    /// It grants *ground*, not immunity: the cell still owes its own torque
    /// test and is still judged against `load::capacity`, so a sheet under
    /// a heavy enough load does give way — measured on `scene=coldsnap`,
    /// a snow-laden sheet breaks up somewhere between a third and half the
    /// world's width. What it does *not* owe, unlike a cell resting on
    /// powder, is `load::bearing_moment`'s footing clamp: a pile carries a
    /// load through the patch it touches and a narrow patch tips, whereas
    /// buoyancy is distributed over the whole underside and has no
    /// eccentricity to tip about. Charging new ice that clamp anyway
    /// destroyed the sheet as it formed — 333 overload failures on
    /// two-to-five-cell patches in 600 frames, and the crushed cells then
    /// broke the chain for the ice above them and took 678 more with them.
    #[serde(default)]
    pub floats: bool,

    /// How much further this material spans when it is part of the
    /// background mass (`Cell::attached`) rather than standing in front of
    /// it: `effective_span = max_unsupported_span * this` for attached
    /// cells, and the plain span for everything else.
    ///
    /// The play world is a 2D slice through a 3D world
    /// (`Reports/worldgen-design.md` §0), so rock belonging to the massif is
    /// braced by material the slice does not contain. That is worth a large
    /// multiplier — but **not immunity**, which is the distinction this
    /// field exists to hold. Attachment used to anchor a cell outright, and
    /// an undercut shelf could then never fall however much was dug out from
    /// beneath it, because its interior was still "attached" and therefore
    /// still held. Reported from play as "nothing breaks off, everything
    /// still dissolves".
    ///
    /// Keyed on attachment rather than on shape, which is what keeps it
    /// safe: two earlier attempts to buy the same strength from geometry
    /// (confinement, then thickness) also made everything the *player* built
    /// unbreakable, because geometry cannot tell a mountain from a wall
    /// someone stacked. Attachment can — nothing the player places is ever
    /// attached.
    ///
    /// 1 (the default) means attachment buys no extra reach at all.
    #[serde(default = "default_attached_span_bonus")]
    pub attached_span_bonus: u16,

    /// How coarsely this material breaks: the number of rungs on
    /// `rigid::fracture`'s power-of-two fragment ladder.
    ///
    /// **Nobody derives debris size from physics, and that is not a gap.**
    /// `Reports/prior-art-destruction.md`: Red Faction authors shard scale
    /// per material, and UE5's Chaos uses a cluster hierarchy with a
    /// damage threshold per level. The ladder here is already the right
    /// *shape* -- uniform over the exponent, so each doubling is half as
    /// likely per cell consumed, which is the heavy-tailed distribution
    /// fragmentation actually has. What it lacked was any way for one rock
    /// to break differently from another.
    ///
    /// Higher means a wider spread and larger top-end pieces: slate shears
    /// into plates, granite calves blocks, a brittle crust shatters into
    /// grit. 5 is the ladder that shipped before this field existed (2, 4,
    /// 8, 16, 32 cells), so leaving it unset changes nothing.
    #[serde(default = "default_fragment_rungs")]
    pub fragment_rungs: u32,


    /// What one step of `max_unsupported_span` costs, per direction the
    /// support comes *from*: standing on the cell below, leaning on the one
    /// beside, or hanging from the one above.
    ///
    /// Rock is strong in compression and weak in bending and tension, so
    /// these are not equal in reality and were not distinguished at all
    /// before this: the relaxation charged a flat 1 in every direction,
    /// which is why a 1-cell tower built up from the ground snapped at the
    /// same height a 1-cell cantilever reached sideways. Make `below` cheap
    /// and a wall stands to any height; keep `beside` dear and an overhang
    /// still fails at its span; make `above` dearest and nothing hangs far.
    ///
    /// All three default to 1, which is the flat cost this replaced — so a
    /// `.ron` that says nothing about them behaves exactly as before.
    #[serde(default = "default_support_cost")]
    pub support_cost_below: u16,
    #[serde(default = "default_support_cost")]
    pub support_cost_beside: u16,
    #[serde(default = "default_support_cost")]
    pub support_cost_above: u16,
}

/// 16, the value the constant this replaced was tuned to — so a `.ron` that
/// says nothing about it behaves exactly as before.
fn default_min_transfer() -> u16 {
    16
}

/// 1 — the flat, direction-blind step cost `structural.rs`'s relaxation
/// charged before `support_cost_*` split it three ways, so a `.ron` that
/// sets none of them relaxes exactly as it always did.
fn default_support_cost() -> u16 {
    1
}

/// 1 — attachment buys no extra span unless a material asks for it, so a
/// `.ron` that says nothing behaves exactly as it did before this existed.
/// The ladder that shipped before the field existed: 2, 4, 8, 16, 32.
fn default_fragment_rungs() -> u32 {
    5
}

fn default_attached_span_bonus() -> u16 {
    1
}

/// 0.65, matching the hardcoded `MIN_LIQUID_BRIGHTNESS` of 0.35 this
/// replaced, so a `.ron` that says nothing about it looks exactly as before.
/// Impenetrable by default -- see `penetration_resistance`'s own doc for
/// why the safe default is "no", not "yes". Well above the 2-3 MPa bound
/// at which real root elongation stops.
fn default_penetration_resistance() -> f32 {
    100.0
}

fn default_fill_dimming() -> f32 {
    0.65
}

fn default_friction_angle() -> f32 {
    45.0
}

/// Default gap between `friction_angle` (repose) and `max_stability_angle`
/// (maximum stability) when a `.ron` file leaves the latter unset. `Reports/
/// granular-mechanics-research.md` §2 cites roughly this gap (Lee & Herrmann
/// 1993; an ~8-degree gap in Metcalfe et al.) — flagged there as read via
/// secondary sources, not a primary, so treat this constant as a reasonable
/// starting point rather than a verified physical value.
const DEFAULT_STABILITY_ANGLE_GAP_DEGREES: f32 = 8.0;

/// Effectively unreachable under normal gameplay temperatures, so a threshold
/// left unset in a `.ron` file never fires rather than firing at 0.0 — the
/// field default a bare `f32` would otherwise get, which would make every
/// material "ignite" and "melt" the instant it's created.
fn default_never() -> f32 {
    f32::INFINITY
}

/// `default_never`'s mirror for *downward* thresholds (`cooling_point`),
/// which fire on `temp <= threshold` — the unreachable sentinel for those
/// sits at negative infinity. Using `default_never` here would invert the
/// meaning entirely: every material would sit "below" +INFINITY and
/// transition on its first visit.
fn default_never_cold() -> f32 {
    f32::NEG_INFINITY
}

/// Zero: no diffusion at all unless a material opts in. This is not
/// physically motivated — a moderate default like 0.15 was tried first, so
/// every material conducted heat plausibly without having to think about it.
/// It was also measured to cost real performance for no benefit to most
/// content: `fire::diffuse_heat` runs for every *visited* CA cell, and a
/// nonzero conductivity means it cannot take its cheap early exit, so sand
/// and water — never near fire in ordinary play — paid for four neighbour
/// reads apiece on every visit anyway. On the sandbox's full-screen stress
/// scenario that took the worst frame from ~16 ms to ~64 ms. Neighbour-driven
/// ignition does not need this at all — it checks the boolean `is_burning`
/// flag on a neighbour, not its diffused temperature — so a material can
/// still catch fire and spread fire correctly with conductivity left at
/// zero; only *passive* warming from a non-burning hot neighbour is what a
/// material gives up by not opting in. Materials that care (oil) set this
/// explicitly.
fn default_heat_conductivity() -> f32 {
    0.0
}

/// Effectively unreachable, the `u16` analogue of `default_never` — a
/// material's structure never breaks under M17 unless a content author sets
/// a real span.
fn default_never_u16() -> u16 {
    u16::MAX
}

#[derive(Deserialize, Clone)]
pub struct ReactionDef {
    pub with: String,
    pub produces: (String, String),
    #[serde(default = "default_reaction_chance")]
    pub chance: f32,
}

fn default_reaction_chance() -> f32 {
    1.0
}

pub struct Material {
    pub name: String,
    pub display: String,
    pub kind: MaterialKind,
    /// Drives displacement: a denser material sinks through a lighter one.
    pub density: f32,
    pub friction_angle: f32,
    /// Derived from `friction_angle`. How far a grain *already flowing*
    /// looks along a slope for somewhere to fall before it settles — see
    /// `roll_reach_at`.
    roll_reach_base: f32,
    /// Resolved (never the 0.0 "unset" sentinel) `max_stability_angle`, in
    /// degrees. Always >= `friction_angle` — see `Material::from`'s clamp.
    pub max_stability_angle: f32,
    /// Derived from `max_stability_angle`. How far a grain at rest looks
    /// along a slope before it starts creeping — see `stability_reach_at`.
    /// Always <= `roll_reach_base`, since a steeper angle can only shorten
    /// the reach; `Material::sweep_reach`'s `Powder` arm relies on this to
    /// stay correct without also having to consider this field.
    stability_reach_base: f32,
    pub dispersion: u8,
    pub flow_rate: u16,
    /// See `MaterialDef::min_transfer`.
    pub min_transfer: u16,
    /// See `MaterialDef::fill_dimming`.
    pub fill_dimming: f32,
    /// See `MaterialDef::penetration_resistance`.
    pub penetration_resistance: f32,
    /// See `MaterialDef::water_capacity`.
    pub water_capacity: u16,
    /// See `MaterialDef::evaporates`.
    pub evaporates: bool,
    /// See `MaterialDef::reinforces_powder`.
    pub reinforces_powder: bool,
    /// Per-cell colour variation. A cell picks one entry when it is created and
    /// keeps it, which gives bulk material visible grain instead of a flat slab.
    pub palette: Vec<[u8; 4]>,

    pub flammability: f32,
    pub ignition_temperature: f32,
    pub burn_temperature: f32,
    pub burn_duration: u16,
    pub heat_conductivity: f32,
    pub melting_point: f32,
    pub boiling_point: f32,
    /// See `MaterialDef::cooling_point`. Unset is `NEG_INFINITY`, not
    /// `INFINITY` — this is a downward (`<=`) threshold.
    pub cooling_point: f32,
    /// See `MaterialDef::intrinsic_temperature`. Unset is `INFINITY`,
    /// read via `is_finite`.
    pub intrinsic_temperature: f32,
    pub max_unsupported_span: u16,
    /// See `MaterialDef::floats`.
    pub floats: bool,
    /// See `MaterialDef::attached_span_bonus`. Always >= 1.
    pub attached_span_bonus: u16,
    /// See `MaterialDef::fragment_rungs`. Always >= 1.
    pub fragment_rungs: u32,
    /// See `MaterialDef::support_cost_below` and its siblings.
    pub support_cost_below: u16,
    pub support_cost_beside: u16,
    pub support_cost_above: u16,

    // Names as written in the `.ron` file (empty = unset), kept so
    // `MaterialRegistry::resolve_references` can look them up once every
    // material in the registry is known. Resolution can't happen inside
    // `From<MaterialDef>` because a material may reference one that hasn't
    // been parsed yet in this same batch, or one from an earlier load this
    // reload never touches — either way, `Material::from` alone never has
    // the full set to check against.
    melts_into_name: String,
    boils_into_name: String,
    cools_into_name: String,
    burns_into_name: String,
    breaks_into_name: String,
    reactions_raw: Vec<ReactionDef>,

    /// Resolved by `resolve_references`. Unset (or naming something that
    /// doesn't exist) both read as "this doesn't happen" — a dangling
    /// reference is a quiet no-op, not a hard error, so a typo during content
    /// iteration costs a missing effect rather than breaking the whole
    /// registry load the way a malformed `.ron` file does.
    pub melts_into: Option<MaterialId>,
    pub boils_into: Option<MaterialId>,
    pub cools_into: Option<MaterialId>,
    pub burns_into: Option<MaterialId>,
    pub breaks_into: Option<MaterialId>,
    pub reactions: Vec<Reaction>,
}

/// A resolved pairwise reaction: `self` becomes `becomes`, the other material
/// (`with`) becomes `other_becomes`.
#[derive(Clone, Copy)]
pub struct Reaction {
    pub with: MaterialId,
    pub becomes: MaterialId,
    pub other_becomes: MaterialId,
    pub chance: f32,
}

impl Material {
    /// How many cells a grain at this position may roll along the surface.
    ///
    /// On a slope that drops one cell every `w` columns, the nearest place a
    /// surface grain could fall is `w + 1` columns away — the first column past
    /// the end of the row below it. So a grain keeps rolling while `reach > w`,
    /// and the pile comes to rest at `w = reach`, a slope of `1 / reach`.
    /// Inverting gives `reach = 1 / tan(angle)`.
    ///
    /// Because reach is a whole number of cells it can only express certain
    /// angles exactly — 1 gives 45 degrees, 2 gives 26.6, 3 gives 18.4. The
    /// fractional part is spent by giving *some positions* the longer reach, so
    /// a 34 degree material rolls one cell across about half the world and not
    /// at all across the rest. That averages to the right angle and leaves the
    /// surface pleasantly irregular rather than a perfectly straight wedge.
    ///
    /// Keyed on position rather than drawn fresh each call, so that a grain
    /// which cannot roll now can never roll from here. Re-rolling per call
    /// would let a chunk fall asleep on a frame the dice said no, freezing
    /// grains that should have kept moving.
    pub fn roll_reach_at(&self, x: i32, y: i32) -> i32 {
        Self::reach_from_base(self.roll_reach_base, x, y)
    }

    /// How many cells a grain *at rest* (not `Cell::flowing()`) may look
    /// along the surface before it starts creeping — the two-angle model's
    /// stricter counterpart to `roll_reach_at`, derived from
    /// `max_stability_angle` the same way `roll_reach_at` is derived from
    /// `friction_angle`. See `FLAG_FLOWING`'s doc (`cell.rs`) for which of
    /// the two a caller should use.
    pub fn stability_reach_at(&self, x: i32, y: i32) -> i32 {
        Self::reach_from_base(self.stability_reach_base, x, y)
    }

    /// Shared by `roll_reach_at`/`stability_reach_at`: spend a reach's
    /// fractional part across positions rather than over time — see
    /// `roll_reach_at`'s own doc for why re-rolling per call instead would
    /// be unsafe for sleeping chunks.
    fn reach_from_base(base: f32, x: i32, y: i32) -> i32 {
        let floor = base.floor();
        let extra = if rng::jitter(x, y) < base - floor { 1 } else { 0 };
        floor as i32 + extra
    }

    /// How far sideways this material's own movement rule can reach when
    /// deciding what to do with a cell — what `Chunk::sweep_region` (issue
    /// #3) must widen a dirty rectangle by so that a cell whose decision
    /// *could* change, because something up to this far away moved, still
    /// gets re-examined. A chunk holding only short-reach materials (a pile
    /// of sand) no longer pays for a `MAX_REACH`-wide sweep band that a
    /// long-dispersion gas cloud would actually need.
    ///
    /// Deliberately **not** the same question `parallel.rs`'s cross-chunk
    /// write-safety proof answers ("how far can a write actually land") —
    /// that proof is keyed on `MAX_REACH` itself, a hard per-frame movement
    /// cap independently enforced at every call site that moves a cell
    /// (`roll_reach_base`'s clamp above, `flow_sideways`'s own
    /// `.min(MAX_REACH)`, `HORIZONTAL_TRANSFER_REACH` being well under it),
    /// and stays exactly `CHUNK_SIZE / 2` regardless of what this function
    /// returns. This function only ever answers "which stale cells need
    /// re-examining," a strictly smaller and purely-performance question;
    /// shrinking its answer can make a chunk's sweep region smaller than
    /// before, never larger, so it cannot violate that proof by construction.
    ///
    /// `.min(MAX_REACH)` below is a defensive floor on that fact for the
    /// `Powder`/`Liquid` cases, where it can genuinely never trigger (every
    /// other reach-computing site is already independently capped); it
    /// exists so a future `.ron` with a wildly large `dispersion` fails this
    /// invariant here, at its one load-bearing use, rather than however far
    /// downstream a stale sweep region would first go unnoticed. For `Gas`
    /// it is not merely defensive — see that arm's own comment.
    pub fn sweep_reach(&self) -> i32 {
        let raw = match self.kind {
            // `roll_reach_at` returns `roll_reach_base.floor()` plus at most
            // one more from its position-keyed jitter — the true worst case,
            // not just the base value.
            MaterialKind::Powder => self.roll_reach_base.floor() as i32 + 1,
            // The larger of the two liquid reaches: fill equalisation looks
            // `HORIZONTAL_TRANSFER_REACH` sideways, but a whole-cell lateral
            // descent (`update::find_lateral_descent`) travels up to
            // `LIQUID_LATERAL_REACH`, so a stale cell that far away genuinely
            // does need re-examining.
            MaterialKind::Liquid => HORIZONTAL_TRANSFER_REACH.max(LIQUID_LATERAL_REACH),
            // `flow_sideways` (`update.rs`) does not stop at `dispersion`:
            // once its initial walk covers as much of that as it can, its
            // free-surface branch searches a further `SURFACE_SEARCH`
            // (`= MAX_REACH`) cells past that point for somewhere to fall —
            // the same free-surface search `Liquid` used before
            // `HORIZONTAL_TRANSFER_REACH` replaced it, still live here since
            // `Gas` never moved off `flow_sideways`. So a gas cell's true
            // worst-case reach is `dispersion + MAX_REACH`, not `dispersion`
            // alone — found by independent review, which traced through
            // `flow_sideways` rather than trusting this match arm's first
            // draft. `.min(MAX_REACH)` below always reduces that back down
            // to exactly `MAX_REACH` for any `dispersion > 0` (since the
            // added term already equals `MAX_REACH`), so this is not a real
            // optimization for `Gas` — a chunk holding gas still gets the
            // full flat widening it always did — but it is now the *correct*
            // value rather than a value that happened to undershoot. Only
            // `dispersion == 0` gets to be smaller, matching `flow_sideways`
            // itself returning immediately without moving at all in that case.
            MaterialKind::Gas => {
                if self.dispersion == 0 {
                    0
                } else {
                    self.dispersion as i32 + super::chunk::MAX_REACH
                }
            }
            MaterialKind::Empty
            | MaterialKind::Solid
            | MaterialKind::Plant
            | MaterialKind::Creature => 0,
        };
        raw.min(super::chunk::MAX_REACH)
    }
}

impl From<MaterialDef> for Material {
    fn from(def: MaterialDef) -> Self {
        let angle = def.friction_angle.clamp(1.0, 89.0);
        // A pile rests at slope `1 / reach`, so the reach for a target angle is
        // `1 / tan(angle)`. Capped at `MAX_REACH`, which is how far the sweep
        // region is widened — reading beyond it would leave grains stale — so
        // the flattest pile the engine can express is `atan(1 / MAX_REACH)`.
        let roll_reach_base =
            (1.0 / angle.to_radians().tan()).clamp(0.0, super::chunk::MAX_REACH as f32);

        // 0.0 is the "unset" sentinel (`MaterialDef::max_stability_angle`'s
        // own doc) — defaults to the repose angle plus a fixed gap. Clamped
        // to never fall below `angle`: a `.ron` mistake that sets this
        // *shallower* than repose would otherwise make `stability_reach_base`
        // exceed `roll_reach_base`, breaking the invariant `sweep_reach`'s
        // `Powder` arm relies on (that the flowing reach is always the
        // worst case). Flooring it to `angle` instead just collapses to the
        // old single-angle behaviour for that one material, silently and
        // safely, rather than corrupting the sweep-reach bound.
        let stability_angle_deg = if def.max_stability_angle <= 0.0 {
            angle + DEFAULT_STABILITY_ANGLE_GAP_DEGREES
        } else {
            def.max_stability_angle
        };
        let stability_angle = stability_angle_deg.clamp(angle, 89.0);
        let stability_reach_base =
            (1.0 / stability_angle.to_radians().tan()).clamp(0.0, super::chunk::MAX_REACH as f32);

        // Issue #3: `Material::sweep_reach` defensively clamps every kind to
        // `MAX_REACH`, but `Gas`'s `dispersion` is the one reach-defining
        // value not already clamped by construction elsewhere (`roll_reach_base`
        // above is; `Liquid`'s reach is a fixed engine constant, not data at
        // all) -- so it's the one a `.ron` file could set past the bound both
        // `Chunk::sweep_region`'s tracking and `parallel.rs`'s cross-chunk
        // write-safety proof assume no material ever exceeds. Caught here,
        // at load time, rather than silently clamped downstream where a
        // content author would never see why their gas stopped dispersing
        // as far as the number they wrote.
        debug_assert!(
            def.kind != MaterialKind::Gas || (def.dispersion as i32) <= super::chunk::MAX_REACH,
            "material `{}` has dispersion {} exceeding MAX_REACH ({})",
            def.name,
            def.dispersion,
            super::chunk::MAX_REACH,
        );

        Self {
            display: if def.display.is_empty() {
                title_case(&def.name)
            } else {
                def.display
            },
            name: def.name,
            kind: def.kind,
            density: def.density,
            friction_angle: angle,
            roll_reach_base,
            max_stability_angle: stability_angle,
            stability_reach_base,
            dispersion: def.dispersion,
            flow_rate: def.flow_rate,
            min_transfer: def.min_transfer,
            fill_dimming: def.fill_dimming,
            penetration_resistance: def.penetration_resistance,
            water_capacity: def.water_capacity,
            evaporates: def.evaporates,
            reinforces_powder: def.reinforces_powder,
            palette: def
                .colors
                .iter()
                .map(|c| [c[0], c[1], c[2], 255])
                .collect(),

            flammability: def.flammability,
            ignition_temperature: def.ignition_temperature,
            burn_temperature: def.burn_temperature,
            burn_duration: def.burn_duration,
            heat_conductivity: def.heat_conductivity,
            melting_point: def.melting_point,
            boiling_point: def.boiling_point,
            cooling_point: def.cooling_point,
            intrinsic_temperature: def.intrinsic_temperature,
            max_unsupported_span: def.max_unsupported_span,
            floats: def.floats,
            // Floored at 1: 0 would silently make attached rock *weaker*
            // than loose material, which is never what a content author
            // means by leaving a field small.
            attached_span_bonus: def.attached_span_bonus.max(1),
            // At least one rung, or the ladder has no rungs to draw from
            // and `Rng::below(0)` is meaningless.
            fragment_rungs: def.fragment_rungs.max(1),
            // Clamped to at least 1 so a lateral or upward step always costs
            // *something*. All three at 0 would let a distance propagate
            // arbitrarily far without ever growing, silently disabling
            // `max_unsupported_span` for that material rather than tuning it
            // -- a content mistake that would read as "spans stopped working"
            // with nothing pointing at the cause.
            //
            // `below` used to be exempt, on the reasoning that 0 there was
            // "the whole point (free compression)". It is now clamped like
            // its siblings, and that is load-bearing rather than tidiness.
            // A zero here makes a whole column relax to
            // `aux == 0`, and `load::evaluate` treats distance 0 as *anchored*
            // -- so the structure becomes immune to every failure mode
            // including hanging in mid-air. That is model 3's self-consistent
            // zero (`Reports/load-model-handoff.md` §6) resurrected as total
            // immunity, and it used to be reachable from any `.ron` file.
            support_cost_below: def.support_cost_below.max(1),
            support_cost_beside: def.support_cost_beside.max(1),
            support_cost_above: def.support_cost_above.max(1),
            melts_into_name: def.melts_into,
            boils_into_name: def.boils_into,
            cools_into_name: def.cools_into,
            burns_into_name: def.burns_into,
            breaks_into_name: def.breaks_into,
            reactions_raw: def.reactions,
            // Left unresolved until `resolve_references` runs.
            melts_into: None,
            boils_into: None,
            cools_into: None,
            burns_into: None,
            breaks_into: None,
            reactions: Vec::new(),
        }
    }
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The material files, compiled in so the engine always has a working set even
/// when run without its assets directory beside it. Hot reload re-reads the
/// same files from disk.
const EMBEDDED: &[&str] = &[
    include_str!("../../assets/materials/stone.ron"),
    include_str!("../../assets/materials/sand.ron"),
    include_str!("../../assets/materials/gravel.ron"),
    include_str!("../../assets/materials/ash.ron"),
    include_str!("../../assets/materials/water.ron"),
    include_str!("../../assets/materials/oil.ron"),
    include_str!("../../assets/materials/smoke.ron"),
    include_str!("../../assets/materials/wood.ron"),
    include_str!("../../assets/materials/moss.ron"),
    include_str!("../../assets/materials/worm.ron"),
    include_str!("../../assets/materials/corpse.ron"),
    // Appended, not inserted alphabetically among the others above --
    // `well_known_ids_match_their_names`'s constants (`STONE` through
    // `SMOKE`) are the numeric position in this array plus a fixed offset,
    // so inserting a new material anywhere but the end would silently
    // renumber every material after it and break those constants at
    // runtime, not just in a test.
    include_str!("../../assets/materials/soil.ron"),
    include_str!("../../assets/materials/deadwood.ron"),
    // Appended for the same reason the comment above gives -- never
    // inserted among the others, which would renumber every well-known id
    // after it at runtime rather than in a test.
    //
    // `rubble` sits ahead of the plant materials because it is the one with
    // a pinned convenience constant (`material::RUBBLE = 15`, guarded by
    // `well_known_ids_match_their_names`) -- the fracture path addresses it
    // by that constant in hot code, while leaf/rootwood/seed are only ever
    // looked up by name. Both sides of the plant/destruction merge appended
    // to this list; the one with an id contract wins the earlier slot.
    include_str!("../../assets/materials/rubble.ron"),
    include_str!("../../assets/materials/leaf.ron"),
    include_str!("../../assets/materials/rootwood.ron"),
    include_str!("../../assets/materials/seed.ron"),
    // Appended, per the rule above: never inserted among the others.
    include_str!("../../assets/materials/snow.ron"),
    // Appended, never inserted -- see the three comments above. The ant
    // milestone's set: the two creatures and the material they call home.
    //
    // **Both sides of the creature/evaporation merge appended here**, the
    // same collision the plant/destruction note above records. Neither
    // `snow` nor these has a pinned convenience constant -- the contracts
    // (`STONE` through `SMOKE`, and `RUBBLE = 15`) are all further up -- so
    // the tiebreak is not an id contract this time but which side was
    // already trunk: `snow` was on master and other branches are in flight
    // against those ids, so it keeps its slot and the creatures take the
    // ones after it.
    include_str!("../../assets/materials/ant.ron"),
    include_str!("../../assets/materials/nest.ron"),
    include_str!("../../assets/materials/beetle.ron"),
    // Appended, never inserted -- see the comments above. The water-cycle
    // milestone's gas phase, then its solid one: ice ships as the freezing
    // half and goes at the end, same rule.
    include_str!("../../assets/materials/steam.ron"),
    include_str!("../../assets/materials/ice.ron"),
];

/// Where the loader looks for material files, relative to the working directory.
pub const ASSET_DIR: &str = "assets/materials";

#[derive(Debug)]
pub enum MaterialError {
    Io(std::io::Error),
    /// Which file failed, and why. The path matters: on hot reload this is the
    /// only feedback for a typo.
    Parse { file: String, error: String },
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaterialError::Io(e) => write!(f, "reading materials: {e}"),
            MaterialError::Parse { file, error } => write!(f, "{file}: {error}"),
        }
    }
}

impl std::error::Error for MaterialError {}

pub struct MaterialRegistry {
    materials: Vec<Material>,
    by_name: HashMap<String, MaterialId>,
}

impl MaterialRegistry {
    /// Vacuum and the out-of-bounds wall, which are engine concepts rather than
    /// content, so they are not loadable and always hold ids 0 and 1.
    fn base() -> Self {
        let mut reg = Self {
            materials: Vec::new(),
            by_name: HashMap::new(),
        };
        reg.insert(Material::from(MaterialDef {
            name: "empty".into(),
            display: String::new(),
            kind: MaterialKind::Empty,
            density: 0.0,
            friction_angle: 45.0,
            max_stability_angle: 0.0,
            dispersion: 0,
            flow_rate: 0,
            min_transfer: default_min_transfer(),
            fill_dimming: default_fill_dimming(),
            penetration_resistance: default_penetration_resistance(),
            water_capacity: 0,
            evaporates: false,
            reinforces_powder: false,
            colors: vec![[0, 0, 0]],
            flammability: 0.0,
            ignition_temperature: f32::INFINITY,
            burn_temperature: f32::INFINITY,
            burn_duration: 0,
            heat_conductivity: 0.0,
            melting_point: f32::INFINITY,
            melts_into: String::new(),
            boiling_point: f32::INFINITY,
            boils_into: String::new(),
            cooling_point: f32::NEG_INFINITY,
            cools_into: String::new(),
            intrinsic_temperature: f32::INFINITY,
            burns_into: String::new(),
            reactions: Vec::new(),
            max_unsupported_span: u16::MAX,
            floats: false,
            breaks_into: String::new(),
            attached_span_bonus: 1,
            fragment_rungs: 5,
            support_cost_below: 1,
            support_cost_beside: 1,
            support_cost_above: 1,
        }));
        reg.insert(Material::from(MaterialDef {
            name: "bedrock".into(),
            display: String::new(),
            kind: MaterialKind::Solid,
            density: f32::INFINITY,
            friction_angle: 45.0,
            max_stability_angle: 0.0,
            dispersion: 0,
            flow_rate: 0,
            min_transfer: default_min_transfer(),
            fill_dimming: default_fill_dimming(),
            penetration_resistance: default_penetration_resistance(),
            water_capacity: 0,
            evaporates: false,
            reinforces_powder: false,
            colors: vec![[20, 20, 24]],
            flammability: 0.0,
            ignition_temperature: f32::INFINITY,
            burn_temperature: f32::INFINITY,
            burn_duration: 0,
            heat_conductivity: 0.0,
            melting_point: f32::INFINITY,
            melts_into: String::new(),
            boiling_point: f32::INFINITY,
            boils_into: String::new(),
            cooling_point: f32::NEG_INFINITY,
            cools_into: String::new(),
            intrinsic_temperature: f32::INFINITY,
            burns_into: String::new(),
            reactions: Vec::new(),
            // Bedrock is the anchor itself — it must never be the thing
            // that breaks free, so this stays unset regardless of what any
            // other material's span is.
            max_unsupported_span: u16::MAX,
            floats: false,
            breaks_into: String::new(),
            attached_span_bonus: 1,
            fragment_rungs: 5,
            support_cost_below: 1,
            support_cost_beside: 1,
            support_cost_above: 1,
        }));
        reg
    }

    /// The compiled-in material set. Always succeeds.
    pub fn builtin() -> Self {
        let mut reg = Self::base();
        for (i, source) in EMBEDDED.iter().enumerate() {
            match ron::from_str::<MaterialDef>(source) {
                Ok(def) => reg.upsert(def),
                // Embedded files are validated by a test, so this is
                // unreachable in practice.
                Err(e) => panic!("embedded material {i} is malformed: {e}"),
            }
        }
        reg.resolve_references();
        reg
    }

    /// Read every `.ron` in `dir`, falling back to the compiled-in set on
    /// failure so a missing or broken assets directory cannot stop the engine.
    pub fn load(dir: impl AsRef<Path>) -> (Self, Option<MaterialError>) {
        let mut reg = Self::builtin();
        match reg.reload(dir) {
            Ok(_) => (reg, None),
            Err(e) => (MaterialRegistry::builtin(), Some(e)),
        }
    }

    /// Re-read `dir` over the current set, returning how many materials were
    /// applied.
    ///
    /// Ids are keyed by name and never reassigned, so cells already in the
    /// world keep their material across a reload. Editing a file changes how
    /// existing cells behave and look, which is the point; renaming one adds a
    /// material rather than changing an existing one.
    pub fn reload(&mut self, dir: impl AsRef<Path>) -> Result<usize, MaterialError> {
        let mut paths: Vec<_> = std::fs::read_dir(dir.as_ref())
            .map_err(MaterialError::Io)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ron"))
            .collect();
        // Sorted so ids are assigned deterministically on a first load.
        paths.sort();

        let mut defs = Vec::new();
        for path in &paths {
            let source = std::fs::read_to_string(path).map_err(MaterialError::Io)?;
            // Plenty of Windows editors save UTF-8 with a byte order mark, and
            // RON rejects it as a stray character at 1:1. Silently tolerating
            // it beats making everyone discover that the hard way.
            let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
            let def = ron::from_str::<MaterialDef>(source).map_err(|e| MaterialError::Parse {
                file: path.file_name().unwrap_or_default().to_string_lossy().into(),
                error: e.to_string(),
            })?;
            defs.push(def);
        }

        // Everything parsed, so nothing is half-applied on a typo.
        let count = defs.len();
        for def in defs {
            self.upsert(def);
        }
        // Re-resolve the whole registry, not just what this call touched: a
        // material reloaded here might newly satisfy a reference some other,
        // untouched material has been carrying unresolved since it was
        // loaded — melting_point/melts_into can be added to two files in
        // either order and the game should not care which.
        self.resolve_references();
        Ok(count)
    }

    /// Look up every material's `melts_into`/`boils_into`/`reactions` name
    /// references against the registry as it now stands, and fill in the
    /// resolved `MaterialId` fields. Call after every batch of upserts —
    /// never partway through one, since a material later in the batch might
    /// be what an earlier one references.
    fn resolve_references(&mut self) {
        // Snapshotted rather than looked up live during the loop below,
        // since resolving needs `&self.by_name` at the same time the loop
        // mutably borrows `self.materials` — cheap, a few dozen entries.
        let by_name = self.by_name.clone();
        let resolve = |name: &str| by_name.get(name).copied();

        let resolve_if_set = |name: &str| if name.is_empty() { None } else { resolve(name) };

        for material in &mut self.materials {
            material.melts_into = resolve_if_set(&material.melts_into_name);
            material.boils_into = resolve_if_set(&material.boils_into_name);
            material.cools_into = resolve_if_set(&material.cools_into_name);
            material.burns_into = resolve_if_set(&material.burns_into_name);
            material.breaks_into = resolve_if_set(&material.breaks_into_name);
            material.reactions = material
                .reactions_raw
                .iter()
                .filter_map(|r| {
                    Some(Reaction {
                        with: resolve(&r.with)?,
                        becomes: resolve(&r.produces.0)?,
                        other_becomes: resolve(&r.produces.1)?,
                        chance: r.chance,
                    })
                })
                .collect();
        }
    }

    /// Replace the material of the same name in place, or append a new one.
    fn upsert(&mut self, def: MaterialDef) {
        let material = Material::from(def);
        match self.by_name.get(&material.name) {
            Some(id) => self.materials[id.0 as usize] = material,
            None => self.insert(material),
        }
    }

    fn insert(&mut self, material: Material) {
        let id = MaterialId(self.materials.len() as u16);
        self.by_name.insert(material.name.clone(), id);
        self.materials.push(material);
    }

    #[inline]
    pub fn get(&self, id: MaterialId) -> &Material {
        // Ids only originate from this registry, and are never reassigned.
        &self.materials[id.0 as usize]
    }

    /// Live in-place mutation, for `tunables.rs`'s panel adjusting a value
    /// this frame without waiting on a file write — a save (or a later
    /// `F5`/hot-reload) is what makes a change durable, this is what makes
    /// it felt immediately. Not exposed as a way to add/remove materials
    /// or rename one (`by_name` would go stale); only individual field
    /// values on an already-registered material.
    #[inline]
    pub fn get_mut(&mut self, id: MaterialId) -> &mut Material {
        &mut self.materials[id.0 as usize]
    }

    #[inline]
    pub fn kind(&self, id: MaterialId) -> MaterialKind {
        self.get(id).kind
    }

    #[inline]
    pub fn density(&self, id: MaterialId) -> f32 {
        self.get(id).density
    }

    /// Look up by the `name` field in the material's file.
    pub fn id_of(&self, name: &str) -> Option<MaterialId> {
        self.by_name.get(name).copied()
    }

    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// Materials offered in the paint picker: everything except the internal
    /// vacuum and out-of-bounds sentinel.
    pub fn paintable(&self) -> Vec<MaterialId> {
        (0..self.materials.len() as u16)
            .map(MaterialId)
            .filter(|id| *id != EMPTY && *id != BEDROCK)
            .collect()
    }
}

impl Default for MaterialRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Panics if any shipped material file is malformed, which is what lets
    /// `builtin` be infallible.
    #[test]
    fn every_embedded_material_parses() {
        let reg = MaterialRegistry::builtin();
        assert_eq!(reg.len(), EMBEDDED.len() + 2, "a material failed to load");
    }

    #[test]
    fn the_files_on_disk_match_the_embedded_set() {
        // Catches a file added to assets/ but not to EMBEDDED, which would
        // silently work from source and break for a shipped binary.
        let mut reg = MaterialRegistry::builtin();
        let before = reg.len();
        let count = reg
            .reload(ASSET_DIR)
            .expect("assets/materials should load from the crate root");
        assert_eq!(count, EMBEDDED.len(), "assets/ and EMBEDDED disagree");
        assert_eq!(reg.len(), before, "loading from disk introduced new ids");
    }

    #[test]
    fn internal_materials_hold_the_ids_the_engine_assumes() {
        let reg = MaterialRegistry::builtin();
        assert_eq!(reg.id_of("empty"), Some(EMPTY));
        assert_eq!(reg.id_of("bedrock"), Some(BEDROCK));
    }

    #[test]
    fn expected_materials_are_present() {
        let reg = MaterialRegistry::builtin();
        for name in ["stone", "sand", "gravel", "ash", "water", "oil", "smoke"] {
            assert!(reg.id_of(name).is_some(), "{name} is missing");
        }
    }

    #[test]
    fn well_known_ids_match_their_names() {
        // Guards the convenience constants against a reordering of EMBEDDED,
        // which would otherwise silently point engine code at the wrong material.
        let mut reg = MaterialRegistry::builtin();
        let expected = [
            ("stone", STONE),
            ("sand", SAND),
            ("gravel", GRAVEL),
            ("ash", ASH),
            ("water", WATER),
            ("oil", OIL),
            ("smoke", SMOKE),
            ("rubble", RUBBLE),
        ];
        for (name, id) in expected {
            assert_eq!(reg.id_of(name), Some(id), "{name} has the wrong id");
        }
        // And they must survive a reload from disk, since ids are keyed by name.
        reg.reload(ASSET_DIR).unwrap();
        for (name, id) in expected {
            assert_eq!(reg.id_of(name), Some(id), "{name} moved after a reload");
        }
    }

    #[test]
    fn every_material_has_at_least_one_colour() {
        let reg = MaterialRegistry::builtin();
        for i in 0..reg.len() as u16 {
            let m = reg.get(MaterialId(i));
            assert!(!m.palette.is_empty(), "{} has an empty palette", m.name);
        }
    }

    #[test]
    fn densities_order_the_way_the_rules_assume() {
        let reg = MaterialRegistry::builtin();
        let d = |n: &str| reg.density(reg.id_of(n).unwrap());
        // Sand sinks through water, oil floats on it, smoke rises through both.
        assert!(d("sand") > d("water"));
        assert!(d("water") > d("oil"));
        assert!(d("oil") > d("smoke"));
    }

    #[test]
    fn reload_keeps_ids_stable_so_existing_cells_survive() {
        let mut reg = MaterialRegistry::builtin();
        let sand_before = reg.id_of("sand").unwrap();
        reg.reload(ASSET_DIR).unwrap();
        assert_eq!(reg.id_of("sand"), Some(sand_before));
    }

    #[test]
    fn oils_burns_into_reference_resolves_to_ash() {
        let reg = MaterialRegistry::builtin();
        let oil = reg.get(OIL);
        assert_eq!(oil.burns_into, Some(ASH));
        assert!(oil.flammability > 0.0, "oil should actually be flammable");
        assert!(oil.burn_duration > 0, "oil should burn for a nonzero duration");
        // melts_into is a different trigger (temperature) from burns_into
        // (a completed burn timer) and must not have been conflated.
        assert_eq!(oil.melts_into, None);
    }

    #[test]
    fn a_dangling_reference_is_a_quiet_no_op_not_an_error() {
        let dir = std::env::temp_dir().join("pixel-physics-dangling-ref");
        std::fs::create_dir_all(&dir).unwrap();
        let body = "(name: \"unstable\", kind: Powder, density: 1.0, \
                     colors: [(1, 2, 3)], burns_into: \"phlogiston\")";
        std::fs::write(dir.join("unstable.ron"), body).unwrap();

        let mut reg = MaterialRegistry::builtin();
        reg.reload(&dir).expect("a dangling reference should not fail the load");
        let m = reg.get(reg.id_of("unstable").unwrap());
        assert_eq!(m.burns_into, None, "a nonexistent target should resolve to None, not panic or error");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_directory_falls_back_to_the_embedded_set() {
        let (reg, err) = MaterialRegistry::load("no/such/directory");
        assert!(err.is_some(), "a missing directory should be reported");
        assert!(reg.id_of("sand").is_some(), "fallback set is unusable");
    }

    #[test]
    fn roll_reach_follows_the_angle_of_repose() {
        let reg = MaterialRegistry::builtin();

        // 45 degrees is the steepest a pile can hold. Reach 1 is the floor: a
        // downhill one column away would already have been taken by the
        // diagonal fall, so reach 1 never actually rolls anything.
        let gravel = reg.get(GRAVEL);
        for x in 0..100 {
            assert_eq!(gravel.roll_reach_at(x, 0), 1, "gravel reach changed at x = {x}");
        }

        // Shallower materials roll further. Averaged over positions, because
        // the fractional part of the reach is spent across the world rather
        // than over time.
        let mean = |m: &Material| {
            let mut total = 0;
            for y in 0..40 {
                for x in 0..40 {
                    total += m.roll_reach_at(x, y);
                }
            }
            total as f32 / 1600.0
        };
        let sand = mean(reg.get(SAND));
        let ash = mean(reg.get(ASH));
        assert!(
            sand > 1.0 && sand < 2.0,
            "sand reach {sand} should sit between 1 and 2"
        );
        assert!(ash > sand, "ash ({ash}) should roll further than sand ({sand})");
    }

    #[test]
    fn roll_reach_is_stable_for_a_position() {
        // The property that keeps sleeping safe: asking twice must agree, or a
        // chunk could settle on a "no" and freeze a grain that could move.
        let reg = MaterialRegistry::builtin();
        let sand = reg.get(SAND);
        for (x, y) in [(0, 0), (13, 7), (-5, 91), (1000, -1000)] {
            assert_eq!(sand.roll_reach_at(x, y), sand.roll_reach_at(x, y));
        }
    }

    #[test]
    fn max_stability_angle_defaults_to_repose_plus_the_standard_gap() {
        // `Reports/granular-mechanics-research.md` §2: a `.ron` that never
        // sets `max_stability_angle` should still get real hysteresis, not
        // silently collapse back to the old single-angle behaviour.
        let reg = MaterialRegistry::builtin();
        let sand = reg.get(SAND);
        assert_eq!(
            sand.max_stability_angle,
            sand.friction_angle + DEFAULT_STABILITY_ANGLE_GAP_DEGREES,
            "sand.ron sets no max_stability_angle, so it should get the default gap"
        );
    }

    #[test]
    fn max_stability_angle_below_friction_angle_is_clamped_up_to_it() {
        // A `.ron` mistake (or a deliberately zero-gap material) must never
        // produce a stability reach *longer* than the roll reach --
        // `Material::sweep_reach`'s `Powder` arm assumes that never happens.
        let def = ron::from_str::<MaterialDef>(
            "(name: \"weird\", kind: Powder, density: 1.0, friction_angle: 40.0, \
             max_stability_angle: 10.0, colors: [(1, 2, 3)])",
        )
        .unwrap();
        let mat = Material::from(def);
        assert_eq!(
            mat.max_stability_angle, mat.friction_angle,
            "max_stability_angle set shallower than friction_angle should clamp up to it, not invert the model"
        );
    }

    #[test]
    fn stability_reach_never_exceeds_roll_reach() {
        // The invariant `sweep_reach`'s `Powder` arm relies on to stay
        // correct without also considering `stability_reach_base`: whatever
        // gap a material has, resting can only ever be *more* conservative
        // than flowing, never less.
        let reg = MaterialRegistry::builtin();
        for name in [SAND, ASH, GRAVEL] {
            let mat = reg.get(name);
            for y in 0..20 {
                for x in 0..20 {
                    assert!(
                        mat.stability_reach_at(x, y) <= mat.roll_reach_at(x, y),
                        "{name:?} at ({x}, {y}): stability reach exceeded roll reach"
                    );
                }
            }
        }
    }

    #[test]
    fn stability_reach_is_shorter_than_roll_reach_on_average_when_a_real_gap_exists() {
        let reg = MaterialRegistry::builtin();
        let sand = reg.get(SAND);
        let mean = |f: fn(&Material, i32, i32) -> i32| {
            let mut total = 0;
            for y in 0..40 {
                for x in 0..40 {
                    total += f(sand, x, y);
                }
            }
            total as f32 / 1600.0
        };
        let roll = mean(Material::roll_reach_at);
        let stability = mean(Material::stability_reach_at);
        assert!(
            stability < roll,
            "stability reach ({stability}) should average shorter than roll reach ({roll}) for sand's default 8-degree gap"
        );
    }

    #[test]
    fn sweep_reach_for_powder_bounds_the_true_worst_case_roll_reach() {
        let reg = MaterialRegistry::builtin();
        let sand = reg.get(SAND);
        let mut worst = 0;
        for y in 0..40 {
            for x in 0..40 {
                worst = worst.max(sand.roll_reach_at(x, y));
            }
        }
        assert_eq!(sand.sweep_reach(), worst, "sweep_reach should equal the true worst-case roll_reach_at, not just approximate it");
    }

    #[test]
    fn sweep_reach_for_liquid_covers_the_lateral_descent_reach() {
        let reg = MaterialRegistry::builtin();
        assert_eq!(reg.get(WATER).sweep_reach(), LIQUID_LATERAL_REACH, "liquid sweep width must cover find_lateral_descent's travel distance, not just fill equalisation's");
        assert_eq!(reg.get(OIL).sweep_reach(), LIQUID_LATERAL_REACH);
    }

    #[test]
    fn sweep_reach_for_a_zero_dispersion_gas_is_zero() {
        // `flow_sideways` (`update.rs`) returns immediately without moving
        // at all when `max <= 0` -- a dispersion-0 gas genuinely never
        // reaches beyond its own cell. `dispersion` defaults to 0 when
        // unset, so this material doesn't even need to state it.
        let dir = std::env::temp_dir().join("pixel-physics-still-gas-material");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("stillgas.ron"),
            "(name: \"stillgas\", kind: Gas, density: 0.1, colors: [(1, 2, 3)])",
        )
        .unwrap();
        let mut reg = MaterialRegistry::builtin();
        reg.reload(&dir).unwrap();
        let stillgas = reg.get(reg.id_of("stillgas").unwrap());
        assert_eq!(stillgas.dispersion, 0);
        assert_eq!(stillgas.sweep_reach(), 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sweep_reach_for_a_dispersing_gas_reaches_max_reach_not_just_dispersion() {
        // The bug an independent review caught in this section's first
        // draft: `flow_sideways`'s free-surface branch searches a further
        // `SURFACE_SEARCH` (`= MAX_REACH`) cells past wherever the initial
        // `dispersion`-limited walk stops, so a gas cell's true reach is
        // `dispersion + MAX_REACH`, not `dispersion` alone -- which this
        // function must report as exactly `MAX_REACH` (the clamp), not the
        // smaller, wrong `dispersion` value a naive reading of `flow_sideways`'s
        // first phase alone would suggest.
        let dir = std::env::temp_dir().join("pixel-physics-dispersing-gas-material");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("fog.ron"),
            "(name: \"fog\", kind: Gas, density: 0.1, dispersion: 3, colors: [(1, 2, 3)])",
        )
        .unwrap();
        let mut reg = MaterialRegistry::builtin();
        reg.reload(&dir).unwrap();
        let fog = reg.get(reg.id_of("fog").unwrap());
        assert_eq!(fog.sweep_reach(), super::super::chunk::MAX_REACH);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_byte_order_mark_does_not_break_a_file() {
        // Windows editors add one routinely, and RON rejects it at 1:1.
        let dir = std::env::temp_dir().join("pixel-physics-bom-material");
        std::fs::create_dir_all(&dir).unwrap();
        let body = "(name: \"bomtest\", kind: Powder, density: 1.0, colors: [(1, 2, 3)])";
        std::fs::write(dir.join("bomtest.ron"), format!("\u{feff}{body}")).unwrap();

        let mut reg = MaterialRegistry::builtin();
        reg.reload(&dir).expect("a leading BOM should be tolerated");
        assert!(reg.id_of("bomtest").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_malformed_file_is_reported_with_its_name() {
        let dir = std::env::temp_dir().join("pixel-physics-bad-material");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("broken.ron"), "(name: \"x\", this is not ron").unwrap();

        let mut reg = MaterialRegistry::builtin();
        let err = reg.reload(&dir).expect_err("malformed file should fail");
        match err {
            MaterialError::Parse { file, .. } => assert_eq!(file, "broken.ron"),
            other => panic!("expected a parse error, got {other:?}"),
        }
        // The good set must survive a bad reload untouched.
        assert!(reg.id_of("sand").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }
}
