//! The organism substrate: species as data, and the behavior library
//! species files compose from. See `Reports/organism-substrate-design.md`
//! for the original design, and `Reports/tree-rewrite-design.md` for the
//! tree-specific extension this module now also implements — both explain
//! the reasoning behind decisions here; this module implements them, not
//! re-derives them.
//!
//! **Scope of this pass** (`Reports/tree-rewrite-design.md`'s retrofit
//! order, step 1): the full `CellType` vocabulary (`Seed`/`GrowingTip`/
//! `MatureBody`/`Leaf`/`RootTip`), `Divide` (moss, unchanged from the
//! previous pass) alongside a new `Grow` (trees — direction-biased,
//! resource-gated, self-avoiding growth for a `GrowingTip`/`RootTip`
//! cell), `Photosynthesize`/`Absorb`/`SecondaryThicken`/`Germinate`/
//! `StructuralAnchor`, the resource-and-canopy-density diffusion pass, and
//! the ported phototropism/wind-lean/hydrotropism field-read helpers.
//! `Locomote` (the worm) stays deferred to a later pass —
//! `organism-substrate-design.md` §7's own stated reason still holds: it's
//! the check that the library generalizes past *rooted* organisms, which
//! is only meaningful once the rooted case (this pass) actually exists.
//!
//! **`TransportChannel` is deliberately not implemented as a named
//! behavior in this pass**, a real, documented scope cut rather than an
//! oversight: making its `decay` rate genuinely per-species would need
//! `CellSurface` (the trait `diffuse_resource` below runs generically
//! over, for both the serial sweep and the parallel `ChunkView`) to expose
//! species lookups, which today it deliberately doesn't — `ChunkView` was
//! designed around exactly the data `update.rs`'s CA rules need, and
//! species access is a new, nontrivial surface neither implementer
//! currently carries. Diffusion instead runs at one shared rate
//! (`DIFFUSION_RATE` below) for any organism-owned `Plant` cell,
//! unconditionally — every cell type this pass defines needs to
//! participate in transport for a tree to function at all, so a per-cell-
//! type opt-in tag would have no actual effect yet regardless; the tag
//! only becomes worth its own `CellSurface` plumbing once a real species
//! wants a cell type that deliberately *doesn't* transport.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use serde::Deserialize;

use super::cell::Cell;
use super::material::MaterialKind;
use super::surface::CellSurface;
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
/// `Grow` places children at all eight neighbours, so anything reading a
/// grown organism *back* has to traverse all eight too — see
/// `reachable_from_anchors` for what happened while it did not.
///
/// Transport (`diffuse_resource`) deliberately stays on `NEIGHBOURS_4`: an
/// exchange happens across a shared face, and diagonal cells share only a
/// corner. Growth is a *placement* decision with eight options; transport is
/// a flux across a boundary with four. Both are correct as they stand, and
/// `Reports/plant-substrate-v2-design.md` §7b makes the same distinction.
const NEIGHBOURS_8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Shared vocabulary for what an organism-owned cell currently *is*, packed
/// into `Cell::aux`'s low 4 bits (`pack_aux`/`unpack_aux` below) — one
/// enum every species shares, so dispatch code never needs to know which
/// species it's looking at. Explicit discriminants, matching `pack_aux`'s
/// own bit layout directly rather than relying on declaration order — room
/// for 11 more variants than are named yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub enum CellType {
    /// A dormant organism-owned cell waiting on `Germinate`'s field-reading
    /// check (light/moisture thresholds) to transition to `GrowingTip`/
    /// `RootTip`. Moss has no `Seed` stage — `plant_moss_seed` still plants
    /// a live `GrowingTip` directly, unchanged from the previous pass —
    /// this is a tree-only cell type today, not a requirement every
    /// species opts into.
    Seed = 0,
    /// An actively growing/dividing cell — moss's only cell type, and a
    /// tree's canopy-growth cell type (`Grow`-driven, see this module's
    /// own doc on the `Divide`/`Grow` split).
    GrowingTip = 1,
    /// A `GrowingTip` that has stopped actively growing (see `organism_
    /// tick`'s staleness handling) and gone fully inert on the M16
    /// active-site schedule, per `design-philosophy.md` §3's explicit
    /// requirement that only actively growing/leaf cells stay scheduled.
    /// Carries `SecondaryThicken`/`StructuralAnchor` for a woody species.
    MatureBody = 2,
    /// A photosynthesizing cell (`Photosynthesize`) — a tree's canopy
    /// foliage, structurally just another `Plant`-kind cell type, not a
    /// special-cased material.
    Leaf = 3,
    /// A root system's own growing tip — `Grow`-driven like `GrowingTip`,
    /// but with `Absorb` for water uptake and gravitropism/hydrotropism
    /// direction bias instead of phototropism/wind. A distinct cell type
    /// (not `GrowingTip` reused) because the support search is *meant* to
    /// anchor on `RootTip` cells (`Reports/organism-substrate-design.md`
    /// §2/§5) — anchoring on every `GrowingTip` instead would anchor a
    /// tree on its own canopy, which is not what "supported by roots"
    /// means.
    ///
    /// **It does not do that yet, and this comment used to say it did.**
    /// `structural::organism_is_supported` anchors on any `Solid`
    /// neighbour, searching outward from the cell under test — so it asks
    /// "am I within `max_unsupported_span` hops of stone", not "can I
    /// reach a root". Soil is a `Powder` and anchors nothing. The
    /// distinction is the whole of why a structural check fired mid-crown
    /// amputates a tree (`plant.rs`'s `shed_stranded_leaves`, measured at
    /// 772 cells against 20,213), and it is what `Reports/
    /// felling-blockers.md` §1 is about. Recorded here rather than quietly
    /// reworded because this is the first comment anyone investigating
    /// that will read, and it sent at least one reader the wrong way.
    RootTip = 4,
    /// An **axillary bud** — a dormant meristem left behind at a node,
    /// holding the potential to become a `GrowingTip` later.
    ///
    /// The unit of shoot construction in real plants is the *metamer*:
    /// internode + leaf + axillary bud. Extension therefore manufactures
    /// its own future meristems, one per node, and the reservoir (the *bud
    /// bank*) grows with the shoot system rather than with the plant's
    /// volume.
    ///
    /// **That asymmetry is the whole point, and it is what the reverted
    /// bud-break design lacked.** Thickening deposits no buds, so a blob
    /// generates no new growth potential however large it gets; only
    /// *extension already performed* does. A rule keyed on "am I idle"
    /// cannot say that — every mature cell answers yes at once, since
    /// carbon fills to the cap, crowding decays and conductance relaxes to
    /// basal the moment growth stops. A depleting stock can.
    ///
    /// Structurally this is ordinary stem tissue: same material, carries
    /// the same `StructuralAnchor`, and `structural.rs` treats it exactly
    /// as it treats `MatureBody`. The only difference is that it still has
    /// one thing left it may do, once.
    DormantBud = 5,
    /// **A creature's deciding cell** — the one that senses, chooses a move
    /// and carries the heading; the body follows it snake-fashion
    /// (`Reports/creature-direction.md` D1/§3c).
    ///
    /// A creature is an organism like any other, which is the whole point:
    /// `Cell::organism_id` carries the generational handle, `OrganismState`
    /// carries the state, and the species comes from `SpeciesRegistry`.
    /// The parallel `CreatureState`/`Cell::aux`-index scheme this replaced
    /// had no generations, no reclamation and a `u16` overflow guarded only
    /// by a `debug_assert` — every one of those already solved here, which
    /// is why it was retired rather than extended
    /// (`Reports/organism-substrate-design.md` opens on exactly that
    /// failure: a third private solution to per-organism state).
    ///
    /// Movement is **code, not a `Behavior`**. A `Behavior` is something a
    /// species composes onto a cell type from data; deciding where to step
    /// is the one thing creature.rs owns, and there is nothing for a
    /// species to compose about it yet.
    Head = 6,
    /// A creature's trailing body cell. No behavior of its own — it goes
    /// where the cell in front of it was (`creature::chain_move`).
    ///
    /// Unused while the worm is the only creature (a one-cell chain), and
    /// added alongside `Head` deliberately rather than later: the two are
    /// one decision about what a creature is made of, and `aux`'s low four
    /// bits are a **positional, never-renumbered** encoding, so adding a
    /// variant between them afterwards would be the renumbering that
    /// encoding forbids.
    Segment = 7,
}

/// One behavior a cell type can carry, composed freely by species data —
/// `material.rs`'s "behaviour comes from data, not a branch per material"
/// claim, one level up. A struct-shaped variant (fields directly on the
/// enum) rather than a separate named struct wrapped in a newtype variant,
/// matching `ActiveKind`'s own existing shape (e.g. `Moss { stale_ticks:
/// u8 }`) and RON's more direct syntax for it.
#[derive(Clone, Copy, Deserialize)]
pub enum Behavior {
    Divide {
        /// Resource spent from the dividing cell on a successful division.
        /// `0.0` is a legitimate value — moss's own retrofit uses it, since
        /// the material `moss_tick` this replaces never had an energy
        /// budget at all, and inventing one is a bigger behavioural change
        /// than a retrofit should make silently.
        cost: f32,
        /// Chance to divide into a damp candidate cell (`field_at().
        /// moisture` above `plant::DAMP_MOISTURE_THRESHOLD`).
        damp_chance: f32,
        /// Chance to divide into a candidate that isn't damp. Real moss
        /// occasionally establishes on residual atmospheric humidity even
        /// off standing water — see the constant this replaces,
        /// `plant.rs`'s old `MOSS_DRY_CHANCE`, for the citation.
        dry_chance: f32,
        /// Whether shade multiplies the chance above (moss's real
        /// preference: shade slows evaporation, favouring establishment)
        /// or is ignored (a species with no reason to care about light
        /// doesn't pay for the field read).
        shade_sensitive: bool,
    },
    /// Direction-biased, resource-gated, self-avoiding growth — a tree's
    /// canopy (`GrowingTip`) and roots (`RootTip`), *not* the same
    /// mechanism as `Divide` with more parameters. See this module's own
    /// doc and `Reports/tree-rewrite-design.md` §0 for why these stayed
    /// two named behaviors rather than one: `Divide` picks a candidate
    /// uniformly at random and gates it on a moisture-keyed chance;
    /// `Grow` scores every open 8-neighbour by a weighted blend of local
    /// directional signals and *samples* from that distribution, gated on
    /// locally available resource rather than a chance roll. Forcing both
    /// shapes onto one struct either breaks RON's all-fields-required
    /// deserialization or silently changes moss's own already-shipped
    /// behavior — `organism-substrate-design.md` §1's own pre-sanctioned
    /// fallback if the two turned out not to share real code, which they
    /// don't.
    /// A dormant axillary bud flushing into a `GrowingTip` — the only
    /// mechanism in the engine that *creates* frontier.
    ///
    /// **Nothing else ever did, and that was the defect behind both
    /// failure modes.** A tip that fails to find a candidate for
    /// `ORGANISM_STALE_LIMIT` ticks retires permanently, so once every
    /// lineage had retired the organism had no frontier and growth was over
    /// for good — measured at zero active sites by frame 16,000, with the
    /// cell count flat from there. The whip is that; the blob is that same
    /// defect with thickening left as the only process still able to add
    /// anything.
    ///
    /// The gate is **not** local, deliberately. See `plant::break_buds`:
    /// the decision is made once per organism per tick against the light
    /// the whole plant is actually intercepting, because every local "am I
    /// idle" signal saturates simultaneously when growth stops, which is
    /// exactly how the earlier bud-break attempt ran away.
    BudBreak {
        /// Where along the plant buds preferentially break — **acrotony
        /// against basitony**, the single scalar the botany review found
        /// flips a plant between tree and shrub *habit* at whole-plant
        /// scale (Barthélémy & Caraglio 2007: "two fundamental phenomena
        /// underlying, respectively, the arborescent or bushy growth
        /// habit").
        ///
        /// Scales a bud's flush score by `1 + acrotony * (elevation − 0.5)`
        /// where elevation runs 0 at the root collar to 1 at the shoot's
        /// top. Positive prefers high buds (acrotony — crowns keep
        /// renewing at the top, a tree), negative prefers basal ones
        /// (basitony — the base keeps throwing new axes, a thicket).
        /// `0.0` is indifferent and is the old behaviour.
        ///
        /// A basitonous species should also raise `thickening_survival`:
        /// its preferred buds sit on the oldest, most-thickened wood, and
        /// the literature's resprouters are exactly the plants whose
        /// epicormic buds track the cambium for decades.
        #[serde(default)]
        acrotony: f32,
        /// Carbon a flush spends to turn the bud into a tip. A price, not
        /// a threshold: it comes out of the same pool `Grow` draws on, so
        /// flushing competes with extending rather than being free.
        ///
        /// Paid by the plant's *richest* cell — usually the trunk sitting
        /// at the resource cap — because that is where the carbon actually
        /// is; the bud itself keeps whatever stake it was holding, floored
        /// at this cost so the new tip can afford its first growth step.
        /// (An earlier version overwrote the bud's stake with the cost,
        /// which destroyed carbon on every flush; `plant::break_buds` has
        /// the correction.)
        cost: f32,
        /// Chance a bud that is covered by `SecondaryThicken` survives it.
        ///
        /// Literal biology — secondary growth kills the buds the cambium
        /// outpaces, which is why mature trunks are bare and why epicormic
        /// resprouting is the *exception* needing its own mechanism. It is
        /// also how a clear bole arrives for free: the trunk thickens and
        /// loses its buds, twigs do not thicken and keep theirs, and
        /// nothing had to know which was which.
        thickening_survival: f32,
    },
    Grow {
        /// Resource spent from the growing cell on a successful growth
        /// step — same role as `Divide`'s `cost`, same gate shape
        /// (`resource < cost` skips this tick, not a dead end).
        cost: f32,
        /// Chance a successful growth step *also* creates a second new
        /// tip in a different direction this same tick — `Reports/tree-
        /// rewrite-design.md` §3's branching mechanism, gated by the same
        /// resource economy as everything else rather than a separate
        /// mechanic.
        branch_chance: ByOrder<f32>,
        /// Weight on continuing this tip's own established direction
        /// (`Reports/tree-rewrite-design.md` §2a: the vector average of
        /// every same-organism 8-neighbour's direction *from* this cell,
        /// negated — "grow away from where you came from" — not a stored
        /// float, a fresh local read every tick).
        continuation_weight: f32,
        /// Weight on phototropism (`organism::phototropism_dir`, ported
        /// from `plant.rs`'s `tree_tip_tick` unchanged). `0.0` for a
        /// species/cell type with no reason to chase light (roots).
        light_weight: ByOrder<f32>,
        /// Weight on wind lean (`organism::wind_lean_dir`, ported
        /// unchanged, including its existing direction-only/magnitude-
        /// clamped fix). `0.0` for roots.
        wind_weight: f32,
        /// Weight on a constant upward (negative-y) bias — canopy growth's
        /// mild "trees grow up" tendency (near `0.0` in `tree.ron`, since
        /// phototropism already does most of that work) or a root's
        /// default gravitropism (strongly positive-y, i.e. negative
        /// `upward_weight`) before `Reports/tree-rewrite-design.md` §2's
        /// MIZ1 hydrotropism switch overrides it.
        upward_weight: ByOrder<f32>,
        /// Weight on `canopy_density` at each candidate — `Reports/tree-
        /// rewrite-design.md` §2b's self-avoidance term, the deposit-
        /// diffuse-decay-follow replacement for the old space-colonization
        /// algorithm's private attractor point cloud.
        ///
        /// **Divides the candidate's score (`preference / (1 + density *
        /// weight)`), and used to subtract from it — the difference is
        /// whether crowding can kill.** Subtraction plus the positive-score
        /// filter meant a strong weight did not bias a crowded tip's
        /// choice, it emptied the choice, and an emptied choice banks the
        /// stale ticks that retire a lineage. That was the measured
        /// collapse cliff the old `tree.ron` sweep warns about (median
        /// tree 2,620 cells at 12.0 against 26 at 20.0) — arithmetic, not
        /// ecology. As a divisor, crowding reorders at any strength and a
        /// fully crowded tip takes its least-bad direction; the guard test
        /// `a_crowded_tip_takes_its_least_bad_direction_instead_of_dying`
        /// fails on the subtractive form, verified by putting it back.
        ///
        /// `0.0` disables self-avoidance entirely (a root system has no
        /// citation in this engine's own research for root-root avoidance,
        /// so `tree.ron`'s `RootTip` sets this to `0.0` rather than
        /// inventing one).
        crowding_weight: f32,
        /// Cap on simultaneously-scheduled `GrowingTip`/`RootTip` active
        /// sites this organism may have of *this* cell type at once —
        /// `Reports/tree-rewrite-design.md` §5's restoration of the old
        /// `MAX_TIPS_PER_TREE`/`MAX_ROOTS_PER_TREE` caps, now read from the
        /// organism's own live schedule rather than a `Vec` length.
        max_active_tips: u32,
        /// Successful growth steps between leaves along one shoot — the
        /// **plastochron**, the real botanical interval between successive
        /// leaf primordia at a shoot apex (`Reports/plant-substrate-v2-
        /// design.md` §5a). Every `plastochron`-th step, the *retiring
        /// parent* becomes a `Leaf` instead of a `MatureBody`, placing
        /// foliage along the shoot behind the advancing tip, which is where
        /// leaves are on a real shoot.
        ///
        /// **`0` disables it**, and that is a real value rather than a
        /// sentinel for "unset": a `RootTip` grows by the same rule and
        /// must never produce foliage underground, and a species whose
        /// photosynthetic surface *is* its shoot (`Reports/tree-rewrite-
        /// design.md` §9's cactus sketch) legitimately wants no separate
        /// leaf stage at all.
        ///
        /// Per-species rather than a Rust constant because leaf spacing is
        /// exactly the kind of value `design-philosophy.md` §2a says
        /// graduates to data immediately — it is gameplay-facing, a
        /// non-programmer would plausibly tune it, and it is one of the
        /// clearest silhouette levers a species file has.
        plastochron: ByOrder<u8>,
        /// **Steps between primed lateral sites** — the branching
        /// oscillator, `0` disables it.
        ///
        /// The sibling of `plastochron` and deliberately a second field
        /// rather than a reuse of it: a root must never leaf underground,
        /// so `plastochron: 0` is load-bearing there, and folding priming
        /// into the same counter would make one number mean "place a leaf"
        /// and "mark a branch site" at once.
        ///
        /// **`0` keeps the in-tick branch roll; non-zero replaces it.** A
        /// shoot tip photosynthesises, so it can genuinely hold two steps'
        /// carbon in one tick and its `branch_chance` roll works as
        /// written. A root tip cannot, and measured, that gate opened twice
        /// in twelve thousand frames — see `OrganismCell::primed` for the
        /// measurement and `PLAN.md`'s M16 note for why periodic priming is
        /// also the better model. Set this on the cell type whose economy
        /// cannot fund a same-tick second purchase; leave it `0` where the
        /// existing roll already works.
        ///
        /// Multiplied per individual by genotype slot 1, as a *rate* — a
        /// high draw shortens the interval and primes more densely, so the
        /// slot keeps the direction its name has always implied (`Reports/
        /// plant-genome-design.md` §5). The slot is re-pointed, not
        /// renumbered.
        #[serde(default)]
        branch_priming: ByOrder<u8>,
        /// How strongly a shoot keeps its existing heading, `0.0`..`1.0`.
        ///
        /// The child's heading is `normalize(parent * inertia + step *
        /// (1 - inertia))`, so `0.0` reproduces the old behaviour exactly
        /// (heading is whatever the last step was, i.e. no memory) and
        /// values near `1.0` give a stem that can barely turn. See
        /// `OrganismCell::heading` for why a local read was not enough.
        ///
        /// A per-species value because it *is* the difference between a
        /// habit that reads as woody and one that reads as a creeper —
        /// a vine should be near `0.0`, a mature trunk near `0.9`.
        #[serde(default)]
        heading_inertia: f32,
        /// Leaf cells placed per plastochron node, as a cluster rather than
        /// singly.
        ///
        /// **A concession to 2D, and an honest one.** A real canopy is a
        /// dense volume of foliage that hides the wood inside it; a 2D
        /// cross-section of the same tree shows every twig, so a
        /// one-cell-per-node canopy renders as a bare skeleton with green
        /// specks on it. The scale argument runs the other way from the
        /// obvious one: at this cell size a leaf *spray* is genuinely
        /// larger than the twig bearing it, so one cell per leaf is
        /// **under**-scaled relative to wood, not over-scaled. Clustering
        /// corrects the ratio rather than faking mass.
        ///
        /// Clusters also concentrate `q` where the foliage actually is,
        /// which sharpens the pipe model's taper — a trunk is only thicker
        /// than its branches to the extent that the foliage is above it.
        #[serde(default = "one_u8")]
        leaf_cluster: u8,
        /// Shoot cells below which this plant is **juvenile**. `0` disables
        /// the whole juvenile stage.
        ///
        /// **`ByOrder` cannot express this, and the gap was measured.**
        /// Branch order is *position in the plant*, not age: a seedling is
        /// order 0 and nothing else, so it sees the trunk tier and only the
        /// trunk tier. That tier is deliberately the sparsest — leafing
        /// every twelfth step is what clears a bole — and a seedling given
        /// the bole's economy has almost no income exactly when it needs it
        /// most. Six of eight trees in the harness scene failed to
        /// establish, against 35% stand-wide in a 1,016-plant study, and
        /// narrowing `branch_chance`'s genotype width made it worse because
        /// *branching is how a small plant builds leaf area*.
        ///
        /// Size stands in for age deliberately. A real seedling's transition
        /// is driven by accumulated resource, not by a clock, and shoot cell
        /// count is what the organism already maintains — no new state, no
        /// per-cell counter, and a plant knocked back by damage correctly
        /// becomes juvenile again.
        #[serde(default)]
        juvenile_size: u32,
        /// Multiplier on `plastochron` while juvenile. Below 1 leafs more
        /// often — a seedling is nearly all foliage, and it has to be:
        /// foliage is the entire income.
        #[serde(default = "one")]
        juvenile_plastochron: f32,
        /// Multiplier on `branch_chance` while juvenile. Above 1 branches
        /// more freely, which is both what real saplings do and the only
        /// way a small plant multiplies its shoots.
        #[serde(default = "one")]
        juvenile_branch: f32,
        /// The fraction of the turgor range over which extension **fades
        /// out** rather than stopping dead.
        ///
        /// `0.3` means a shoot grows at full rate until it has spent 70% of
        /// its available turgor, then its per-tick chance of extending
        /// falls linearly to zero over the last 30%. `0.0` restores the
        /// hard cutoff and is the value every species had before this
        /// existed.
        ///
        /// **Grounded, and load-bearing for the silhouette.** Lockhart's
        /// equation gives wall extension rate as proportional to `(P − Y)`;
        /// a step function at `P = Y` is the one part of the turgor model
        /// that was not taken from it. The cost of that shortcut was
        /// visible in every render: with a step, every lineage in the plant
        /// runs at full speed right up to one row and terminates there, so
        /// each crown is a flat horizontal plate with growth trailing
        /// below it. Slowing the last stretch lets `ORGANISM_STALE_LIMIT`
        /// retire some lineages before others, and the top becomes a band
        /// instead of a line.
        #[serde(default)]
        turgor_taper: f32,
        /// **Genotype jitter** — the fraction by which this species' own
        /// numbers vary from one individual to the next. `0.15` means every
        /// organism draws its parameters somewhere in ±15% of the values
        /// written here.
        ///
        /// **Every plant in a stand running identical constants is the
        /// L-system symmetry failure, and this branch has watched it arrive
        /// three times through three different doors.** Each global bound
        /// became a visible artifact the moment growth reached it: the
        /// world ceiling, then the turgor ceiling, then a canopy slab lying
        /// along exactly one row. A stand of eight trees measuring 127, 128,
        /// 128, 129, 129, 130, 130, 131 rows tall is not eight trees; it is
        /// one tree drawn eight times, and it reads that way instantly.
        ///
        /// Each parameter gets its own draw
        /// (`OrganismState::genotype_draws`, one slot per trait), because an
        /// individual that is simultaneously *more* branchy, *more* upright
        /// and *taller* is just the average tree scaled up — variation has
        /// to be independent per trait to read as variety rather than as
        /// size.
        ///
        /// This width is read *live*, at every use, while the draw it
        /// multiplies is fixed at germination. So editing a width (or
        /// pulling it in the tunables panel) re-scales the variation an
        /// existing stand shows without redrawing who anybody is.
        ///
        /// `0.0` disables it, and that is a real value: moss has no use
        /// for individuality, and a test that wants a reproducible single
        /// tree wants the written numbers and not a draw around them.
        /// Indexed by trait — **`GENOTYPE_TRAITS`' own doc holds the slot
        /// map and is the contract.** All zeroes disables jitter. The
        /// shoot's `Grow` reads 0/2/3 from this vector (4/6/7/9 are
        /// borrowed from the same vector by the passes that consume them
        /// — one plant, one genotype); the root's `Grow` reads 1/5/8 from
        /// the RootTip entry's own vector, which is what lets root and
        /// shoot diverge within one individual.
        ///
        /// **Slot 9's width is provisional and is not from measurement**,
        /// which is a gap left visible rather than relabelled away
        /// (`CLAUDE.md`: set bars from measurement, and where the number
        /// cannot be had yet, record both and leave the gap). The trait
        /// has no consumer, so there is no outcome to regress a width
        /// against; what it must not be is `0.0`, because a slot at zero
        /// width is a slot no population can explore and this one exists
        /// precisely to give selection something to act on. So each
        /// species carries its **own widest in-service width** there —
        /// the same number as its `pipe_ratio` slot, 0.7 where the others
        /// run wide and 0.5 on conifer, whose whole vector is tighter.
        /// That is a defensible default rather than a measured value: it
        /// invents no new number, it spans better than 5x between the
        /// extreme individuals, and it is wide enough that a mean moving
        /// under selection will clear the drift the readout shows on an
        /// unselected control. Re-derive it once the response curve lands.
        ///
        /// **Slots are positional and must never be renumbered** — the slot
        /// index selects which stored draw a trait reads, so moving a trait
        /// silently rewrites every genome ever measured. Retire a dead trait
        /// by setting its width to `0.0`, not by removing its slot; a slot
        /// dead by measurement in *every* species may be re-purposed once
        /// (see `GENOTYPE_TRAITS`). Slots 1 and 5 are exactly that case:
        /// upward weight measured inert across 1,024 genomes even at ±40%
        /// (quintile means 1310, 1460, 1396, 1388, 1457 cells — flat),
        /// light weight at ±50% and for a structural reason (the sky cast
        /// leaves no lateral gradient to steer by), so both now carry the
        /// root traits instead.
        ///
        /// **One number for all four was wrong, and it was measured wrong.**
        /// At a flat ±15%, a 64-genome population showed only turgor doing
        /// anything at all — correlation with tree height −0.675, against
        /// |r| ≤ 0.13 for the other three on every outcome. The same ±15%
        /// means very different things: it moves derived height from 104 to
        /// 141 rows, and it moves expected lifetime branch count from 2.55
        /// to 3.45, which is invisible. Worse, the plastochron is an
        /// integer, so ±15% on the outer orders' base of 2 rounds back to 2
        /// across the *entire* range — a third of the genome was dead on
        /// arrival.
        ///
        /// So the width has to be set per trait, from what each one
        /// actually moves, and the inert ones need to be wide enough to
        /// clear their own quantization.
        #[serde(default)]
        genotype_variance: [f32; GENOTYPE_TRAITS],
        /// Whether a **fork on this tier replaces the axis instead of
        /// decorating it** — monopodial vs sympodial branching, another of
        /// the Hallé–Oldeman–Tomlinson discriminating axes, and nearly
        /// free here because `Grow` already retires its apex every step:
        /// monopodiality was only ever the *labelling* (the primary child
        /// inherits order and heading, the lateral starts a new tier).
        ///
        /// `true` on a tier means a branch event there makes *both*
        /// children laterals — both take `order + 1` and a fresh heading —
        /// so the axis is built of stacked equivalent modules. `ByOrder`'s
        /// saturation then gives Leeuwenberg's model for free: after a few
        /// forks everything runs on the last tier's parameters, a plant of
        /// repeated equal modules, which is what a lilac or a frangipani
        /// is. A `false` trunk under `true` outer tiers is Scarrone's
        /// (mango); all-`false` is the monopodial tree this engine has
        /// always grown.
        #[serde(default = "all_monopodial")]
        sympodial: ByOrder<bool>,
        /// Which way each tier's axes want to point — see [`Tropism`].
        /// Orthotropic everywhere reproduces the old hardcoded `(0, -1)`.
        #[serde(default = "all_orthotropic")]
        tropism: ByOrder<Tropism>,
        /// **The angle, in degrees, at which a new lateral leaves its
        /// parent axis** — measured between the parent's `heading` and the
        /// step the lateral takes.
        ///
        /// Branch angle is a top-tier silhouette parameter in every prior
        /// art for procedural trees (L-systems, Weber–Penn, space
        /// colonization) and this engine had **no parameter for it at
        /// all**: the lateral was `alt[rng.below(alt.len())]`, a uniform
        /// draw over whatever open neighbours were left. Branching *rate*
        /// was per-order species data; branching *angle* was noise. See
        /// `Reports/plant-appearance-design.md` §2.3.
        ///
        /// Growth is on an 8-neighbourhood, so the achievable angles are
        /// multiples of 45° and this is a *target* that the candidate set
        /// is scored against, not a value that can be hit exactly. It is
        /// still a weighted sample and never an argmax — a deterministic
        /// best-direction pick is what would curve-fit a silhouette, which
        /// is the objection the candidate loop's own doc raises.
        ///
        /// `0.0` means unset and restores the uniform draw.
        ///
        /// **Useless without `internode`**, which is why they landed
        /// together: a lateral that leaves at 90° is re-scored against
        /// `upward_weight` and the tier reference on its very next step and
        /// bends straight back alongside the trunk. That is the
        /// parallel-ropes look, and an angle alone does not touch it.
        #[serde(default)]
        branch_angle: ByOrder<f32>,
        /// **How many steps a fresh lateral holds its departure direction**
        /// before the light, wind and tropism terms get a vote.
        ///
        /// The straightness budget, and the missing shape primitive: this
        /// engine models a branch as a biased random walk and nothing in it
        /// represented a branch as an *object* with a length and a
        /// direction. Coefficients change the statistics of a meander; they
        /// cannot change what a meander is (§2.4 of the same report). An
        /// internode is a straight run, and a crown of straight runs
        /// leaving at an angle is what a tree looks like.
        ///
        /// Counted in the lineage step the active site already carries, so
        /// this costs **no new per-cell state**: a lateral is rescheduled
        /// with `plastochron: 0`, so its lineage step *is* its age in
        /// cells.
        ///
        /// `0` means unset and restores the old always-score-everything
        /// behaviour.
        #[serde(default)]
        internode: ByOrder<u8>,
        /// **How straight a shoot draws its own heading**, `0.0`..`1.0`.
        ///
        /// `0.0` is the historical behaviour and costs nothing: the tip
        /// simply takes the direction it sampled. Above zero, the sampled
        /// direction stops being the step and becomes a *vote on the
        /// heading*; the step actually taken is the heading rendered onto
        /// the lattice by error diffusion, exactly as a line-drawing
        /// routine renders a slope.
        ///
        /// **This is a rendering knob, not a steering one, and that is the
        /// whole point.** The scoring loop above is untouched -- same
        /// weights, same weighted sample, same one `rng` draw -- so what a
        /// tip *wants* is unchanged and every economic quantity that
        /// depends on it (income, turgor, crowding) is undisturbed. What
        /// changes is only how a continuous want is spelled in whole
        /// cells.
        ///
        /// Why that is the fix and a bigger `continuation_weight` is not:
        /// the eight-neighbour lattice cannot hold a direction. A tip
        /// heading dead-on vertical scores `(0,-1)` at 1.0 and both upward
        /// diagonals at 0.707, so it steps off its own axis **59% of the
        /// time** however hard continuation is weighted -- and because
        /// each step is drawn independently, those departures accumulate
        /// instead of cancelling. That is a random walk, and a random
        /// walk's distance from its own axis grows without bound. Error
        /// diffusion keeps the identical *average* direction (a 17-degree
        /// shoot still leans 17 degrees) and bounds the error at under one
        /// cell, which is the difference between a staircase and a wobble.
        ///
        /// Measured on the order-0 trunk's own weights, RMS departure from
        /// the local best-fit line: **0.88 cells at `0.0`, 0.41 at `1.0`**
        /// (a perfect line renders at ~0.3). Sharpening the sample instead
        /// was measured and **made it worse** -- see `Reports/dead-ends.md`.
        ///
        /// Costs one `(f32, f32)` per organism cell
        /// (`OrganismCell::growth_residual`) and nothing at all while this
        /// is zero, which is the state every species predating it is in.
        #[serde(default)]
        stem_stiffness: ByOrder<f32>,
        /// Mechanical resistance, in MPa, this cell type can force its way
        /// through — a `RootTip` converts a `Powder` neighbour whose
        /// `Material::penetration_resistance` is *below* this into root
        /// tissue in place, instead of being blocked by it.
        ///
        /// `0.0` means "only grows into open air", which is correct for a
        /// canopy `GrowingTip`: a shoot growing into a sand dune is not a
        /// thing, and `Reports/tree-rewrite-design.md` §5 already scopes
        /// growing-into-material to roots only.
        ///
        /// Per species because root penetrating force genuinely varies —
        /// and because it is the parameter that decides whether a species
        /// can colonise compacted ground, which is a gameplay-facing
        /// difference between plants rather than an engine constant.
        /// Real root growth pressures are on the order of 0.2-1.5 MPa,
        /// which is what `tree.ron`'s value is set against.
        penetration_force: f32,
        /// **Turgor at the root collar**, in the units the other two share.
        /// Growth is driven by turgor pressure `P` exceeding a yield
        /// threshold, and `P` falls with height above the collar — so this
        /// is the budget the whole shoot draws its height from.
        ///
        /// `Reports/tree-extension-biology.md` §2c. Real apices are limited
        /// by a *positional* quantity, not a physiological one: water
        /// potential falls **0.01 MPa per metre from gravity alone**,
        /// unconditionally, and Koch et al. derive redwood's 122–130 m
        /// ceiling from exactly that. Potkay et al. turn it into the growth
        /// law used here, `max(P − Γ, 0)`.
        ///
        /// **The ceiling this produces is derived, not imposed:**
        /// `h_max = (turgor_source − turgor_yield) / turgor_per_cell`.
        /// Three numbers give a height, rather than a "stop at N cells" cap
        /// — which is what `Reports/tree-shape-problem-statement.md` §5
        /// asks for and what every previous attempt failed to supply.
        #[serde(default)]
        turgor_source: f32,
        /// Yield threshold `Γ`. Below this, cell walls do not extend at all.
        #[serde(default)]
        turgor_yield: f32,
        /// Potential lost per cell of height above the collar — the
        /// gravitational term, and the reason this gate cannot saturate
        /// uniformly the way every resource signal does.
        ///
        /// **`0.0` disables the gate entirely and is a legitimate value**,
        /// not a misconfiguration: a moss mat or a vine has no meaningful
        /// height limit, and `RootTip` growth heads *downward* where the
        /// term would be negative. Same convention as `plastochron: 0`.
        #[serde(default)]
        turgor_per_cell: f32,
    },
    /// Reads the light field, credits the resource scalar — a `Leaf`
    /// cell's own contribution to the resource economy `Grow`/`Divide`
    /// spend from. `rate` is per-tick credit at full light; scaled by the
    /// actual local reading the same way every other field-driven rate in
    /// this codebase is.
    Photosynthesize {
        rate: f32,
        /// Shedding **pressure** in deep shade: the chance per organism
        /// tick that a fully-dark leaf is shed, scaled down steeply
        /// (cubed) as light rises. `0.0` disables it.
        ///
        /// **This is what clears a bole, and it is the mechanism the shape
        /// was missing rather than seasons.** A leaf that intercepts almost
        /// nothing costs the plant to keep and earns nothing, so real trees
        /// abscise shaded foliage continuously — natural pruning, or crown
        /// lift. It is why a forest tree carries leaves only where light
        /// reaches and why its lower trunk is bare, and it happens all year
        /// rather than once a season.
        ///
        /// **A rate, not a light threshold.** The first threshold died to
        /// the day/night oscillator (every leaf reads near zero at
        /// midnight, so any fixed cutoff was a nightly extinction event);
        /// noon-equivalent light fixed that. A threshold on the phase-free
        /// reading is then genuinely workable — measured at 20,044 cells
        /// against graded's 20,213 on the same stand — and graded is kept
        /// for what a line cannot do: it thins a darkening region over
        /// many ticks instead of culling it the tick it crosses, which
        /// measured as better crown separation (fused run 37 vs 55) and a
        /// better-lit standing canopy, shrugs off transient dips a line
        /// converts into same-tick loss, and reads as leaves going rather
        /// than a shelf being swept.
        ///
        /// One measurement hazard is recorded on `plant::
        /// shed_stranded_leaves` because it will bite again: the first
        /// sweep of *both* forms read as "any setting collapses the
        /// stand", and the collapse was a structural check the shed used
        /// to schedule, not the shedding.
        ///
        /// Reaching for seasonality here would be the wrong tool twice
        /// over: the *shape* it is wanted for comes from shading and not
        /// from the calendar, and a season short enough to be visible in a
        /// tree that matures in 30,000 frames would read as flicker rather
        /// than as a year.
        ///
        /// Second-order but real: shedding shaded leaves concentrates the
        /// remaining foliage at the crown, which sharpens the pipe model's
        /// taper — a trunk is only thicker than its branches to the extent
        /// that foliage sits above it.
        #[serde(default)]
        shade_death: f32,
        /// Shedding **pressure** under drought, the exact counterpart of
        /// `shade_death` and cubed for the same reason: the chance per tick
        /// that a leaf with no water is shed, falling away steeply as its
        /// water store fills. `0.0` disables it.
        ///
        /// Graded rather than a threshold, because a threshold on a
        /// quantity that recovers between ticks culls a whole crown on one
        /// bad tick — the same failure the light threshold had, recorded
        /// above. It is also what makes drought *visible*: a starving plant
        /// thins out over many ticks rather than freezing in place, and a
        /// seedling germinated in a canopy with no soil to reach dies
        /// rather than merely stopping.
        #[serde(default)]
        drought_death: f32,
    },
    /// Pulls water out of adjacent soil and loses it to the air — the
    /// transpiration stream — crediting no resource at all.
    ///
    /// `rate` is a multiplier on `plant.rs`'s `TRANSPIRATION_PER_ROOT_CELL`,
    /// so `1.0` is an ordinary tree and `0.0` disables it. Per species
    /// because transpiration rate is one of the largest real differences
    /// between plants: a succulent's whole strategy is closing its stomata
    /// by day, and it should not dry the ground the way a broadleaf does.
    ///
    /// Attach it to whichever cell types touch soil. A canopy cell carrying
    /// it costs nothing — it simply never has a soil neighbour to draw
    /// from — so `tree.ron` puts it on `MatureBody` (which is what a
    /// retired root cell becomes) without needing a separate root body
    /// type.
    Transpire { rate: f32 },
    /// Drains every adjacent `Liquid`-kind cell, crediting resource and
    /// depleting local moisture — `plant.rs`'s old `ROOT_WATER_ENERGY`/
    /// `ROOT_MOISTURE_DEPLETION` mechanism, relocated onto generic species
    /// data. `Reports/tree-rewrite-design.md` §5 also gives `RootTip`'s
    /// own `Grow` dispatch a second water-uptake path (growing directly
    /// into a water cell, absorbing it and advancing rather than being
    /// blocked) — `Absorb`'s `rate` here covers only the passive,
    /// stationary drink-in-place case.
    Absorb { rate: f32 },
    /// On a `MatureBody` cell, periodically counts downstream `Leaf`
    /// cells of the same `organism_id` through connected `Plant`
    /// neighbours (`reachable_from_anchors`, a counting variant) and grows
    /// sideways into adjacent displaceable cells once `leaf_count /
    /// current_width > pipe_ratio` — Shinozaki's pipe model theory,
    /// `Reports/organism-substrate-design.md` §4's own citation and
    /// derivation, `pipe_ratio` deliberately a per-species parameter
    /// rather than a universal constant per that section's own discussion
    /// of the theory's documented limits.
    /// **Set seed.** The heredity channel: a mature plant spends carbon
    /// to place a `Seed` cell carrying its own genome, drifted by
    /// `plant::MUTATION_SIGMA`.
    ///
    /// A whole-organism event expressed per cell on purpose. It runs on
    /// every `MatureBody` cell, so a plant's seed rate is its canopy
    /// size times `seed_chance` -- a big tree out-breeds a small one
    /// with no rule saying so, and no whole-plant query either.
    Reproduce {
        /// **Carbon price of setting one seed.** Paid out of the cell that
        /// sets it, like every other growth cost, so a plant that cannot
        /// afford to reproduce does not.
        #[serde(default)]
        seed_cost: f32,
        /// **What fraction of its surplus this species commits to
        /// reproduction** — the strategy trait that replaced a rate fence.
        ///
        /// **This field was `seed_chance`, a per-cell dice roll, and the
        /// swap is the point rather than a rename.** Measured
        /// (`Reports/plant-equilibrium-costs-2026-08-27.md` §13): with a
        /// roll in the way, carbon was the binding constraint on
        /// reproduction **0.7% of the time** — the roll decided the rate
        /// and the economy was decoration. Quadrupling the carbon
        /// allocation changed seed output by nothing. A price behind a
        /// fence is not a price.
        ///
        /// Now the *number* of seeds a plant sets is `budget / seed_cost`,
        /// which is carbon, and the only randomness left decides *which*
        /// mature cell bears each one. So a big tree still out-breeds a
        /// small one — through the surplus it earns rather than through a
        /// count of cells to roll on — and raising this trades directly
        /// against growth, because `allocate_to_frontier` takes it off the
        /// top before funding the frontier.
        ///
        /// Real plants run roughly 5-30% of net primary production into
        /// reproduction, and that is the range these values are authored
        /// in. `0.0` disables reproduction, which is what moss and any
        /// species predating this get.
        #[serde(default)]
        reproductive_allocation: f32,
        /// Shoot cells a plant needs before it sets any seed at all.
        ///
        /// Without it a seedling reproduces on its first mature cell and
        /// the world fills with dynasties of two-cell plants that never
        /// pay the cost of growing up — selection for instant reproduction,
        /// which is a real evolutionary attractor and a boring one.
        #[serde(default)]
        seed_maturity: u32,
    },
    SecondaryThicken { pipe_ratio: f32 },
    /// A `Seed` cell's transition to `GrowingTip`/`RootTip`, checked on a
    /// schedule against local field readings. `instant: true` is a
    /// test-only escape hatch that fires unconditionally next tick,
    /// avoiding germination-condition waits in every test that just needs
    /// a grown organism to exist — `organism-substrate-design.md` §1's own
    /// stated reason for this field.
    Germinate { light_threshold: f32, soil_water_threshold: f32, instant: bool },
    /// Marks a cell type as counting toward `structural.rs`'s
    /// `is_body_material` check for organism-owned cells — a tag other
    /// systems read, no behavior of its own (matches `ActiveKind`'s own
    /// "some variants carry no extra data" precedent).
    StructuralAnchor,
}

/// One `.ron` file: a species' cell types, each with the behaviors that
/// run on it. `Vec<(CellType, Vec<Behavior>)>` rather than a map keyed on
/// `CellType` — simpler RON syntax, and no species is expected to have
/// enough cell types for linear lookup to matter.
/// How many consecutive palette entries make one **band** — one hue, in the
/// four tonal steps `render.rs` already uses as material grain.
///
/// Colour is not physics, so a conifer's needles are not a new *material*:
/// `leaf.ron`'s own doc states this engine's test for when a material is
/// warranted ("its *physics* genuinely differ on numbers that already
/// exist"), and a fir and an oak differ on none of them. What differs is
/// hue, so hue is what varies — a band range per species, one band per
/// individual inside it, one tonal step per cell.
pub const PALETTE_BAND: u8 = 4;

/// A species' slice of a material's palette: `count` bands starting at
/// `first`. An individual draws one band from this range at germination, so
/// the range is the *species'* colour and the draw is the *individual's*.
///
/// **`count: 0` means unset and restores the pre-band behaviour** — a shade
/// drawn uniformly from the whole palette. That is what every species
/// without a declared range gets (moss, and any asset set that predates
/// this), so adding bands cost no existing species its look.
#[derive(Clone, Copy, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct PaletteBands {
    pub first: u8,
    pub count: u8,
}

#[derive(Deserialize)]
pub struct SpeciesDef {
    pub name: String,
    /// Which bands of `leaf`'s palette this species' foliage draws from.
    #[serde(default)]
    pub foliage_bands: PaletteBands,
    /// Which bands of `wood`'s palette this species' stems draw from.
    #[serde(default)]
    pub bark_bands: PaletteBands,
    /// The stock fraction below which this species starts closing its
    /// stomata — 0.0 (the default) never closes early, which is exactly
    /// the pre-closure engine: the settle in `plant::organism_upkeep`
    /// draws `min(stock, demand)` and desiccation equals `1 − status`
    /// identically, so a species that does not opt in cannot be moved by
    /// this field or by genotype slot 7, which multiplies it.
    #[serde(default)]
    pub stomatal_reserve: f32,
    /// **What this species is made of** — the three materials seeded at
    /// germination and at leaf placement, defaulted to the tree set so
    /// every shipped `.ron` is untouched.
    ///
    /// These are the *seeds*, not a cell-type-to-material table: growth
    /// still propagates a parent's material to its child, which is what
    /// makes a whole root system rootwood from one seeded cell. Moving
    /// these three constants from code to data is the entire engine
    /// change behind "a plant that is not a tree" — before it, a `Grow`
    /// species was brown stem and green leaf by construction, whatever
    /// its numbers said (`Reports/plant-evolution-design.md` §3c).
    ///
    /// An unknown name falls back to the parent cell's own material,
    /// exactly as the hardcoded lookups did for a stripped asset set.
    #[serde(default = "default_shoot_material")]
    pub shoot_material: String,
    #[serde(default = "default_root_material")]
    pub root_material: String,
    #[serde(default = "default_leaf_material")]
    pub leaf_material: String,
    /// **How long this species' seeds stay viable**, as a half-life in
    /// frames: the number of frames over which half a dormant seed bank
    /// disappears. `0.0` means immortal, which is what every seed was
    /// before this field existed.
    ///
    /// **A half-life rather than a lifespan, and that is the design call,
    /// not a convenience.** `Reports/population-dynamics-research.md` §3
    /// wants the seed bank to be the ecology's *reservoir* — the thing that
    /// carries a species through a trough where an individual-based grid
    /// otherwise hits the absorbing state at zero. A fixed lifespan empties
    /// a cohort all at once and gives that reservoir a cliff; a constant
    /// per-frame hazard gives it an exponential tail, so a bank fed at any
    /// rate at all settles at `input x 1.443 x half_life` and *thins rather
    /// than empties*. It is also the model seed-bank ecology actually uses,
    /// and it is memoryless, so it needs no per-seed age counter.
    ///
    /// **Per species because the real axis is real.** Large-seeded woody
    /// plants run transient banks (a season) and small-seeded ruderals run
    /// persistent ones (years) — so a herb should out-*wait* a tree here,
    /// not out-breed it only. That difference is what makes a seed bank a
    /// strategy rather than a delay.
    #[serde(default = "default_seed_half_life")]
    pub seed_half_life: f32,
    /// **How fast a dead individual's remains disappear**, as a half-life
    /// in frames — the counterpart of `seed_half_life` at the other end of
    /// the life cycle. See `plant::step_organisms`' senescence pass for
    /// what "dead" means here (nothing left that can earn or restart), and
    /// `OrganismState::senescent` for why the flag is one-way.
    ///
    /// The value differs by an order of magnitude between a sod and a
    /// trunk for the obvious reason, so it is data rather than a constant.
    /// `0.0` leaves remains standing for ever, which is the behaviour every
    /// species had before this field existed.
    #[serde(default = "default_remains_half_life")]
    pub remains_half_life: f32,
    pub cell_types: Vec<(CellType, Vec<Behavior>)>,
    /// Everything only a *creature* species needs. `#[serde(default)]` so
    /// `moss.ron` and `tree.ron` keep parsing untouched — a plant is a
    /// species with an empty `Creature` block, not a species missing one.
    #[serde(default)]
    pub creature: Option<CreatureDef>,
}

/// How a creature's cells are arranged, and therefore how they move.
///
/// **These are two movement rules, not one with a parameter**, and the
/// split is the same call `organism.rs` already makes for `Divide` versus
/// `Grow`. A chain *follows*: the body steps into the head's old
/// positions, which is why it flows over any terrain and why it is exactly
/// one cell wide — a path has no width. A rigid body *translates*: every
/// cell shifts by the same offset, so it can be any shape, and it gets
/// stuck where a chain would not.
///
/// **Decision D1 rejected rotation, not width.** Re-read it: "translating
/// *or rotating* a shape through a falling-sand world is an unsolved hard
/// problem". Translating by one cell is a passability check. Rotating is
/// the hard half — a rotated shape does not land on the grid cleanly and
/// you get aliasing, self-overlap and cells appearing from nowhere. And
/// gravity spares us it entirely: a walking creature has a canonical up,
/// so it only ever needs facing-left and facing-right, which is a **mirror
/// of the template**, not rotation maths.
///
/// The trade is real and it is the point. A wide body handles rough ground
/// badly — often no legal position at all — where a chain flows over
/// anything. That cost is also what makes a wide predator unable to follow
/// a one-cell-wide ant into its tunnel, with no "hiding" code anywhere.
#[derive(Clone, Deserialize)]
pub enum BodyPlan {
    /// `n` cells in a following chain, head first. 1 is the worm, 2-3 an
    /// ant. Owner open question #1 lives here: how many cells does a
    /// creature need to read at play zoom?
    Chain(u8),
    /// Cell offsets from the head, authored **facing east**; the west-facing
    /// form is this mirrored in x. `(0, 0)` is the head and is implicit —
    /// list only the rest. y grows downward, matching the grid.
    Rigid(Vec<(i8, i8)>),
}

impl BodyPlan {
    /// Cell offsets from the head, head first, in the given facing.
    pub fn offsets(&self, facing_west: bool) -> Vec<(i32, i32)> {
        let mut out = vec![(0, 0)];
        match self {
            // A chain is laid out behind the head along the facing, which
            // is only its *initial* shape — after one step the body is
            // wherever the head has been.
            BodyPlan::Chain(n) => {
                for i in 1..*n as i32 {
                    out.push((if facing_west { i } else { -i }, 0));
                }
            }
            BodyPlan::Rigid(cells) => {
                for &(dx, dy) in cells {
                    out.push((if facing_west { -(dx as i32) } else { dx as i32 }, dy as i32));
                }
            }
        }
        out
    }

    pub fn is_rigid(&self) -> bool {
        matches!(self, BodyPlan::Rigid(_))
    }

    /// How many cells this body occupies.
    pub fn len(&self) -> usize {
        match self {
            BodyPlan::Chain(n) => *n as usize,
            BodyPlan::Rigid(cells) => cells.len() + 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A creature species' body plan, instincts and metabolism.
///
/// Separate from `cell_types` because these are properties of the
/// *individual*, not of a cell type: a chain has one length however many
/// cell types it uses, and one genome however many cells it has.
#[derive(Clone, Deserialize)]
pub struct CreatureDef {
    /// What this creature is made of, and how it moves.
    pub body: BodyPlan,
    /// Frames between decisions.
    pub tick_interval: u64,
    pub start_energy: f32,
    /// Charged every tick regardless of what the creature does.
    pub idle_cost: f32,
    /// Charged per cell moved.
    pub move_cost: f32,
    /// Charged per **active** synapse per tick, as a fraction of
    /// `start_energy`.
    ///
    /// The sign of this mechanism is the point rather than its magnitude:
    /// connections must pay for themselves or evolution prunes them, which
    /// is simultaneously the sparsity pressure that keeps evolved brains
    /// legible and a real energetic trade-off (brains are metabolically
    /// expensive). `brain::eval_brain` returns the count for free.
    ///
    /// **A fraction rather than an absolute, and that is not a unit
    /// preference.** As an absolute it was silently a *different* tax
    /// every time anything changed the energy budget: §13j measured a
    /// harness cutting `start_energy` 900 -> 90 for scarcity and leaving
    /// this alone, which spent 72 of the 90 on thinking — 80% of a life —
    /// and made a "forager versus immobile" sweep really a "thinks versus
    /// does not think" sweep, invalid and thrown away. The harness had to
    /// carry a hand-applied `0.002 * (start_energy / 900.0)` to correct
    /// for it; expressing the ratio structurally deletes that correction,
    /// and a correction nobody has to remember is one nobody can forget.
    ///
    /// It also has to be a fraction before body size is heritable (S8),
    /// because `start_energy` becomes a function of the body then and an
    /// absolute tax would quietly re-price thinking for every size.
    pub synapse_fraction: f32,
    /// **What one cell of this animal's body is worth as meat**, granted at
    /// spawn alongside `start_energy` and stamped into its corpse cells when
    /// it dies.
    ///
    /// Structural, not metabolic: the animal can never spend it, which is
    /// what lets a *starved* creature — dead at exactly 0 — still leave food
    /// behind. Making a corpse worth only the leftover would close §13l's
    /// pump and delete the scavenger niche in the same stroke, since a
    /// starved animal's leftover is zero by definition.
    ///
    /// Booked into `EnergyLedger::granted` at spawn, so it is accounted
    /// rather than conjured. When reproduction lands (S6) the parent pays
    /// this for every cell of its child, which is what keeps a lineage from
    /// minting meat by breeding.
    #[serde(default)]
    pub body_energy: f32,
    /// Fraction of `start_energy` below which a creature eats what it finds
    /// instead of carrying it home.
    ///
    /// **Not "below full", which is what this was first written as and it
    /// silently deleted the entire foraging loop.** A creature is below its
    /// starting energy within one tick of being born and stays there
    /// forever, so "eat if not full" means *always eat* — measured at 14
    /// eats against 8 pickups and zero deliveries, with a colony that
    /// looked, in the picture, exactly like one foraging correctly. This is
    /// the number that decides whether a colony feeds itself or feeds its
    /// nest, and it belongs in species data because that trade-off is what
    /// separates a solitary forager from a social one.
    pub hunger_fraction: f32,
    /// **The ancestral value of every heritable body trait**, indexed by
    /// `CREATURE_TRAITS`' slot map. Slot 0 is `gut_bias`.
    ///
    /// Authored per species because two ancestors one number apart, living
    /// in different parts of the world and coloured differently, *is* the
    /// first half of guided divergence — no reproduction required for the
    /// stage to be worth shipping (`creature-evolution-plan.md` §2.5).
    #[serde(default)]
    pub traits: [f32; CREATURE_TRAITS],
    /// Per-trait mutation width, same indexing as `traits`.
    ///
    /// **Read by nothing until S6**, exactly as `MaterialDef::food_class`
    /// was authored one stage ahead of the gut that reads it. It is here
    /// now because the width is a *species* property — how far a lineage's
    /// gut can wander in one birth — and authoring it beside the ancestral
    /// value is what keeps the two from drifting apart in separate commits.
    ///
    /// Per-trait rather than one global step, on the same measurement that
    /// retired the global brain step (E6, §13l): one width across traits of
    /// different scales either never moves the wide one or shreds the
    /// narrow one. This vector is the body-side half of that call.
    #[serde(default)]
    pub trait_variance: [f32; CREATURE_TRAITS],
    /// Whether a **living nestmate counts as ground** — an ant walks over
    /// another ant the way it walks over terrain.
    ///
    /// **Footing only, never passability**, and the asymmetry is the whole
    /// design: a creature cell stays something you cannot *enter*, so two
    /// chains never swap through each other. See `creature::Kin`.
    ///
    /// Default off, and opted into on the species rather than tested in the
    /// footing loop — the dispatch site already holds the def, and the
    /// alternative is a `bool` re-resolved for each of twenty-four
    /// neighbour cells per step.
    ///
    /// This is the deliberate re-test of dead ends 775/829, which gridlocked
    /// a shoulder-to-shoulder colony at 27,386 blocked ticks against a
    /// single pickup and whose condition line reads *"re-test if creatures
    /// gain pass-through or climb-over"*.
    #[serde(default)]
    pub climbs_over_kin: bool,
    /// Whether this species will bite a **living** member of its own
    /// species.
    ///
    /// **Data, because the diet axis cannot express it.** `ant` material is
    /// `food_class: 1.0` worth 120 and a starved ant's corpse cell is
    /// `food_class: 1.0` worth 120 — the same point on the axis and the
    /// same number — so no `gut_bias` and no threshold tells a nestmate
    /// from carrion. See `creature::is_living_kin`, which holds the full
    /// account; this is the species-side opt-in, defaulting to the answer
    /// that keeps a colony from eating itself.
    ///
    /// It replaces what `CreatureDef::food`'s name list was *really* doing
    /// by the end. That list carried two separate claims — what is
    /// nutritious, and whom it is acceptable to bite — and only the first
    /// is a gut. S5 takes the first (`gut_bias` and the matched filter) and
    /// this takes the second, which is why the list could finally go: the
    /// plan's §2.3 note "the list stays as the selectivity gate until that
    /// trait exists" was one trait short, not one stage short.
    #[serde(default)]
    pub eats_kin: bool,
    /// The material a nest is built from — what `AtNest` senses.
    pub nest: String,
    /// How hard this species can dig, against a material's
    /// `penetration_resistance`. The pattern roots already use
    /// (`Behavior::Grow`'s `penetration_force`), **not** a material-name
    /// whitelist: a species that can chew soil but not stone should say so
    /// in force, so a future softer stone is diggable automatically.
    pub dig_force: f32,
    /// Ticks over which nest-scent deposit falls to nothing. See
    /// `OrganismState::since_nest`.
    pub nest_memory: u16,
    /// Sensor offset in cells for the forward/lateral sampling.
    ///
    /// 6, measured: `pheromone::tests::trail_following_sweep` puts on-trail
    /// tracking at 0.817 for SO 6 against 0.755 at 4, 0.743 at 8 and 0.727
    /// at 10 — the best of the range, and it agrees with the literature's
    /// 5-9 band and with the report's starting figure.
    pub sensor_offset: i32,
    /// The authored starting brain, as a sparse wiring list.
    pub instincts: Vec<super::brain::Instinct>,
    /// Authored connections into the hidden layer, and out of it. Empty for
    /// a species whose generation-zero behaviour is pure taxis; the ant
    /// needs them for the one thing a single layer cannot express — see
    /// `brain::genome_from_wiring`.
    #[serde(default)]
    pub hidden_wiring: Vec<super::brain::HiddenWire>,
    #[serde(default)]
    pub hidden_outputs: Vec<super::brain::OutputWire>,
}

fn default_shoot_material() -> String {
    "wood".to_string()
}
fn default_root_material() -> String {
    "rootwood".to_string()
}
fn default_leaf_material() -> String {
    "leaf".to_string()
}

/// Set against the measured bank rather than from a target. On the
/// eight-tree stand the bank stood at **160 seeds at 60,000 frames and was
/// still climbing** — 42 at 28,800, so it was accelerating, not settling —
/// and the whole point of a clock is that it should stop. See
/// `SpeciesDef::seed_half_life`. 9,000 frames is 2.5 in-world days against
/// a tree that reaches full size in about eight, so a seed outlives a dry
/// spell and does not outlive the tree that dropped it; at the stand's own
/// late-run seeding rate that predicts a bank settling near 50, which is a
/// reservoir rather than a leak.
fn default_seed_half_life() -> f32 {
    9_000.0
}

/// See `SpeciesDef::remains_half_life`. The woody default: a dead sapling
/// stands for a while and then goes, rather than either vanishing on the
/// tick it dies (the all-or-nothing outcome `CLAUDE.md`'s ethos section
/// rules out) or standing for ever holding a slot (the behaviour this
/// replaces).
fn default_remains_half_life() -> f32 {
    6_000.0
}

pub struct Species {
    pub name: String,
    pub foliage_bands: PaletteBands,
    pub bark_bands: PaletteBands,
    pub stomatal_reserve: f32,
    /// See `SpeciesDef::shoot_material`.
    pub shoot_material: String,
    pub root_material: String,
    pub leaf_material: String,
    /// See `SpeciesDef::seed_half_life`.
    pub seed_half_life: f32,
    /// See `SpeciesDef::remains_half_life`.
    pub remains_half_life: f32,
    cell_types: Vec<(CellType, Vec<Behavior>)>,
    pub creature: Option<CreatureDef>,
    /// The authored genome, expanded once at load rather than per spawn.
    pub genome: Vec<f32>,
}

impl Species {
    /// Behaviors registered for `cell_type`, or an empty slice if this
    /// species doesn't use it — not an error, since a species is free to
    /// only define the cell types it actually has (moss has exactly one).
    pub fn behaviors(&self, cell_type: CellType) -> &[Behavior] {
        self.cell_types
            .iter()
            .find(|(ct, _)| *ct == cell_type)
            .map(|(_, b)| b.as_slice())
            .unwrap_or(&[])
    }

    /// Whether this species grows a separate `Leaf` stage at all.
    ///
    /// **This is the question `plastochron` answers from the wrong end.** A
    /// species with `plastochron: [0, 0]` places no leaves, so it has no
    /// `Leaf` cells — but reading the plastochron means reading a `Grow`
    /// entry that may not exist, per *order*, from whichever cell type you
    /// happened to ask about. Whether the file declares a `Leaf` cell type
    /// is the same fact stated once, and it is what `plant.rs`'s abscission
    /// rules need: a species whose photosynthetic surface *is* its shoot
    /// (this file's own `plastochron` doc anticipates exactly that case)
    /// sheds shoot, because it has nothing else to shed.
    pub fn has_leaf_stage(&self) -> bool {
        self.cell_types.iter().any(|(ct, _)| *ct == CellType::Leaf)
    }

    /// Whether a cell of this type earns carbon for this species.
    pub fn photosynthesises(&self, cell_type: CellType) -> bool {
        self.behaviors(cell_type).iter().any(|b| matches!(b, Behavior::Photosynthesize { .. }))
    }

    /// **Does this species have a carbon economy at all** — i.e. is there
    /// any cell type in it that earns?
    ///
    /// The gate on the senescence rule in `plant::step_organisms`, and it
    /// is there because that rule is *starvation-shaped*: "nothing left
    /// that can earn" is not a statement about a species that never earns.
    /// `moss.ron` is exactly that — one cell type, `Divide` at `cost: 0.0`,
    /// no `Photosynthesize` anywhere, and its own file records that giving
    /// it a budget "would be a bigger behavioural change than this rewrite
    /// is meant to make silently" (the moss overhaul is call 4, deliberately
    /// deferred).
    ///
    /// **Found by the guard test, not by reading.** `organism_tick` retires
    /// a stale `GrowingTip` to `MatureBody`, and moss declares no
    /// `MatureBody`, so a retired moss cell has no behaviours whatsoever —
    /// which read as a corpse and would have quietly made moss patches rot
    /// away. That is a moss behaviour change wearing a plant-mortality
    /// change's clothes, and it is out of scope twice over.
    pub fn has_economy(&self) -> bool {
        self.cell_types.iter().any(|(ct, _)| self.photosynthesises(*ct))
    }

    /// **Whether a cell of this type can keep its organism alive** — the
    /// cell-type half of the senescence test in `plant::step_organisms`.
    /// The other two halves are per *cell* (root tissue is never vital,
    /// whatever its type declares) and per *species* (`has_economy` above).
    ///
    /// Three behaviours, and each is a distinct way of not being dead:
    ///
    /// - `Photosynthesize` — it earns.
    /// - `Germinate` — it has not started yet. A seed is a *dormant* stage,
    ///   which is the one thing the reservoir role turns on; treating "no
    ///   foliage" as death here would kill the seed bank the same day it
    ///   was given a decay clock.
    /// - `BudBreak` — it can restart a shoot from nothing, which is exactly
    ///   what makes a topped tree not a dead one.
    ///
    /// `Grow` is deliberately **not** on the list, and that is the whole
    /// discrimination: a `RootTip` grows, and a root system with no shoot
    /// left above it cannot ever earn the carbon its growth spends. Roots
    /// are the remains, not the survivor.
    ///
    /// Neither is `Divide`, and that is the same call read from the other
    /// side: a `Divide` economy needs no carbon, so a species running on one
    /// is exempt at the species level (`has_economy`) rather than being
    /// carried case by case here.
    pub fn is_vital(&self, cell_type: CellType) -> bool {
        self.behaviors(cell_type)
            .iter()
            .any(|b| matches!(b, Behavior::Photosynthesize { .. } | Behavior::Germinate { .. } | Behavior::BudBreak { .. }))
    }
}

impl From<SpeciesDef> for Species {
    fn from(def: SpeciesDef) -> Self {
        let genome = def.creature.as_ref().map(|c| super::brain::genome_from_wiring(&c.instincts, &c.hidden_wiring, &c.hidden_outputs)).unwrap_or_default();
        Self {
            name: def.name,
            foliage_bands: def.foliage_bands,
            bark_bands: def.bark_bands,
            stomatal_reserve: def.stomatal_reserve,
            shoot_material: def.shoot_material,
            root_material: def.root_material,
            leaf_material: def.leaf_material,
            seed_half_life: def.seed_half_life,
            remains_half_life: def.remains_half_life,
            cell_types: def.cell_types,
            creature: def.creature,
            genome,
        }
    }
}

/// Index into `SpeciesRegistry`. Distinct type from `MaterialId` even
/// though both are a `u16` newtype over a `Vec` index — a material id in a
/// species slot (or vice versa) should be a type error, not a silent
/// cross-registry mixup.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpeciesId(pub u16);

/// One mouthful in transit: what it is, what it is worth, and what it
/// looked like.
///
/// **Not a `MaterialId` alone, which is what this was.** Both drop sites
/// wrote `Cell::new(held, 0)`, so a corpse worth 640 came back down worth
/// whatever its `.ron` says (zero) and its shade reset to the darkest
/// entry in the palette. That was harmless while nutrition was a constant
/// of the *eater*; the moment food carries its own value it is a material
/// sink sitting on the one path a colony is built around — carrying
/// something home.
///
/// **Not a whole `Cell` either**, which would preserve all of this for
/// free and is the tempting version. `Cell::aux` is a tagged union and a
/// live creature cell packs its organism id in there, so storing the cell
/// verbatim and putting it back down re-creates a cell claiming to belong
/// to an organism that has since been freed — the aliasing failure
/// `eating_one_leaf_does_not_kill_the_tree_that_grew_it` was written for,
/// re-entered through the carry path. Naming the three fields that
/// actually travel makes that unrepresentable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Carried {
    pub material: super::material::MaterialId,
    /// Energy worth, in the same 1:1 units `Cell::aux` uses for a corpse.
    /// Written back into `aux` **only** if the material says
    /// `worth_in_aux`: on a `Powder` that does not, `aux` means soil water,
    /// and a leaf put down holding 120 would be a leaf holding water.
    pub worth: u16,
    pub shade: u8,
}

/// Per-organism state too large (or too semantically distinct) to fit in
/// `Cell::aux` — mirrors `plant::TreeState`/`creature::CreatureState`'s
/// existing reason to exist, generalized across every species rather than
/// reinvented per one. Minimal today: just which species this organism
/// is, all that moss (§7 of the design report's retrofit order) needs.
/// Trees will need more here (an anchor/root-tip list, a shared energy
/// pool) when that retrofit lands — deliberately not added speculatively
/// ahead of a caller that would exercise it.
pub struct OrganismState {
    pub species: SpeciesId,
    /// Every cell this organism currently owns.
    ///
    /// **Step one of `Reports/plant-substrate-v2-design.md`'s Decision 2**,
    /// and deliberately behaviour-free: it is maintained and asserted
    /// against a full grid scan, and nothing reads it to make a decision
    /// yet. The design doc's own migration plan puts the risk here, in
    /// keeping the list honest across every path that creates or destroys
    /// an organism cell, and gets that verified before anything depends on
    /// it.
    ///
    /// **What this unlocks is mostly performance.** Today every mature cell
    /// sits on the active-site schedule forever and re-derives whole-
    /// organism facts by itself — `thicken()`'s flood fill being the worst
    /// case, quadratic in tree size. With an enumerable cell list, the
    /// per-cell upkeep becomes one pass per *organism*, which is the shape
    /// M16's own principle asks for ("plants only change at their tips — a
    /// trunk is inert") and which the schedule currently inverts. It also
    /// makes `free_organism`, a real anchor list for `organism_is_
    /// supported`, and a cheap `organism_active_tip_count` all possible;
    /// each is noted as blocked on exactly this in its own doc.
    ///
    /// **A position-keyed map, deliberately, and not the design doc §3c's
    /// slot-indexed `Vec<Option<OrganismCell>>` addressed by an index
    /// written back into `Cell::aux`.** Step 2c freed those bits, so that
    /// layout became available and was still not taken. Three reasons, in
    /// increasing order of importance:
    ///
    /// 1. **The hook stays complete by construction.** Membership is
    ///    maintained at the single `World::set` seam (see that function).
    ///    A slot index living in `aux` would make `set` rewrite the `aux`
    ///    of the cell handed to it, and every one of `plant.rs`'s ~60
    ///    `pack_cell_type` writes would have to preserve a slot it does not
    ///    know it carries. That is the enumeration-that-must-stay-complete
    ///    failure mode `World::set`'s own comment says this project keeps
    ///    rediscovering — reintroduced, at every write instead of every
    ///    creation.
    /// 2. **12 bits is 4095 cells per organism, and that is a real
    ///    ceiling.** The twelve-tree ensemble already produces a 2,127-cell
    ///    tree; half the budget, on a 512x320 test world — at the time this
    ///    was measured, `CLAUDE.md` said the world was going to grow, and it
    ///    has: the shipped world is 8192x2560 now.
    /// 3. **The hot loop wants iteration, not random access, and gets it
    ///    either way.** The slot index buys O(1) cell→data from a bare
    ///    `Cell`, but the only hot consumer is `transport`, which iterates
    ///    the whole organism and resolves neighbours *once per tick* into a
    ///    flat `Vec` (see `transport`'s own topology build). Inside the
    ///    substep loop both layouts are identical: contiguous indexing.
    ///
    /// What §3c's layout was really buying was somewhere to put the
    /// scalars, and `OrganismCell` as the map's *value* buys that without
    /// the encoding. The property that made the doc reject a *global*
    /// position map — unambiguous ownership of every entry — is kept, since
    /// this map is per-organism.
    pub cells: std::collections::HashMap<(i32, i32), OrganismCell>,
    /// Below-ground and above-ground cell counts, refreshed once per
    /// organism tick by `plant::step_organisms` while it is already walking
    /// the cell list.
    ///
    /// **This is what bounds root growth.** Real plants hold a roughly
    /// conserved root:shoot ratio — roots are a minority of biomass, and a
    /// tree cannot grow an unbounded root system off a small canopy,
    /// because roots are built from carbon the canopy fixes. Nothing in the
    /// engine expressed that, and once soil gave roots income everywhere
    /// they proliferated until they had converted an entire soil bed to
    /// root tissue.
    ///
    /// A whole-organism total, which `Reports/plant-substrate-v2-design.md`
    /// §6 explicitly sanctions holding here: allometry is a genuine
    /// whole-plant property, and no local rule can compute "am I mostly
    /// root". Kept as counts rather than a ratio so a caller can apply its
    /// own threshold.
    /// **The plant's water stock — the second currency, held per organism
    /// rather than per cell.**
    ///
    /// `Reports/plant-substrate-v2-design.md` §3c sketched `water: f32` on
    /// `OrganismCell` and §9 item 12 sanctioned symmetric transport for it.
    /// **That was built, measured, and does not work at tree scale**, which
    /// is worth recording because the design record says otherwise:
    ///
    /// Water entered at the roots and never arrived. On the standard probe
    /// the stand fell to 64 cells, with root tissue at the cap (4.0) and
    /// foliage median **0.00** — a mean stomatal term of 0.15. The reason
    /// is not tuning. Diffusion spreads as the square root of the substep
    /// count: 45 substeps at `DIFFUSION_RATE` move a front a handful of
    /// cells, and a mature tree is ~130 rows from root to crown, needing
    /// thousands. Carbon only crosses that distance because canalization
    /// builds polar strands that carry it; the one `carbon_conductance`
    /// array cannot also serve water, because xylem and phloem run in
    /// *opposite* directions — that is exactly what §9 item 12 says.
    ///
    /// So the balance lives on the organism, which is the object the
    /// question is actually about: "can this plant supply its canopy" is a
    /// whole-plant question, and `allocate_to_frontier` already pools and
    /// distributes carbon income at exactly this scale. Ask which object a
    /// rule evaluates — a cell, a section, or a whole piece — and water
    /// status is a property of the piece.
    ///
    /// Capacity is proportional to root mass (`plant::water_capacity_of`),
    /// so a deep, wide root system buys a real drought buffer and a
    /// shallow one does not. That is the coupling that makes root traits
    /// worth selecting on, and the same quantity anchorage will read.
    pub water: f32,
    /// How much of this tick's transpirational demand the stock could
    /// actually meet, `0.0..=1.0` — the **stomatal term** that multiplies
    /// every photosynthetic credit and every leaf's contribution to
    /// intercepted light.
    ///
    /// Computed once per organism tick in `plant::organism_upkeep`'s
    /// existing whole-plant walk, beside `root_cells`/`shoot_cells`, and
    /// read per cell after. Storing it rather than recomputing per cell is
    /// what keeps the two income gates (`allocate_to_frontier` and
    /// `break_buds`) reading the identical number.
    pub water_status: f32,
    /// Water actually taken up over the last organism tick, and the demand
    /// it was measured against — the two halves of the balance, kept so a
    /// deficit can be attributed rather than inferred from stand mass.
    ///
    /// Sums, not counts: `CLAUDE.md` prefers a continuous quantity over a
    /// count of starving cells, because counts give knife-edge margins.
    pub water_uptake: f32,
    pub water_demand: f32,
    /// The live accumulator behind `water_uptake`.
    ///
    /// Two fields rather than one because the first version reset the
    /// counter in the same statement that reported it, so every probe read
    /// the post-reset zero and the readout said uptake was **0.00** for
    /// every plant in the stand -- while the stomatal term said 0.16, which
    /// is impossible with no uptake at all. A counter that disagrees with
    /// the quantity it is supposed to explain is measuring its own
    /// bookkeeping.
    pub water_uptake_acc: f32,
    /// The **desiccation term** — how short of demand this plant would
    /// have fallen with stomata fully open, `0.0..=1.0`. What
    /// `drought_death` reads, where earning reads `water_status`.
    ///
    /// Two numbers because prudence must not read as thirst: an
    /// individual that closes its stomata early (`stomatal_reserve` ×
    /// genotype slot 7) spends less water and *earns* less — that is the
    /// real price — but its leaves are not drying out while the tank
    /// still holds. Shedding keyed on the spent-side term would make the
    /// conservative allele shed hardest while protecting its stock, and
    /// the stomatal locus would select against itself
    /// (`Reports/plant-genome-design.md` §4.3). With `stomatal_reserve`
    /// at 0 the two terms are identical by construction.
    pub water_desiccation: f32,
    /// Carbon the parent packed into this individual's seed —
    /// `Reproduce.seed_cost`, handed to the seedling as its starting
    /// stake at germination instead of vanishing at the deduction site.
    /// The seed *is* its provisions. 0 for a planted seed, which starts
    /// broke exactly as scenes and tests have always assumed.
    ///
    /// Species-level plumbing today, deliberately: the seed-strategy
    /// locus that would vary it is **deferred** until the
    /// endowment→establishment response is measured — assigning a slot
    /// to an unmeasured trade is how slots 1 and 5 died the first time
    /// (`Reports/plant-genome-design.md` §4.8).
    pub endowment: f32,
    pub root_cells: u32,
    /// **Root cells that share a face with soil** — the uptake surface, as
    /// opposed to the mass.
    ///
    /// A root cell walled in by its own siblings shares no face with
    /// anything it could drink from, so it can absorb nothing while still
    /// costing carbon to build and to run. `Reports/root-blob-and-uptake-
    /// surface-2026-08-23.md` measured that interior at **33.1% / 36.1% /
    /// 33.3%** of the root system at 10,800 / 25,200 / 43,200 frames, so
    /// it is a third of every root system and not a rounding error.
    ///
    /// **Read the second finding before treating this as a brake.** The
    /// interior share does *not* rise with mass — root cells nearly
    /// quadruple across that table while it holds at about one third — so
    /// pricing contact is a flat tax on root mass, not a bound on it. What
    /// it does buy is the thing the owner asked for: per-plant contact
    /// already spans **51%–79% at comparable mass, same genome, same
    /// scene**, and nothing was pricing it, so nothing could select on it.
    ///
    /// A face, not eight neighbours, for the reason `diffuse_resource`
    /// stays four-connected while `Grow` places at eight: an exchange
    /// crosses a shared face, and a diagonal cell shares only a corner.
    pub contact_root_cells: u32,
    pub shoot_cells: u32,
    /// **How many of this plant's cells are structural anchors** — the
    /// `is_structural_anchor` set, tallied in `anchor_support`'s seeding
    /// loop rather than in a walk of its own.
    ///
    /// That loop already enumerated the set to seed its heap and then
    /// dropped it (`open-bugs-handoff.md` §P3, "the anchor *set* itself is
    /// never materialised"). Counting it costs one increment per cell in a
    /// pass that was already visiting every cell.
    pub anchor_cells: u32,
    /// **The anchor plate's resisting moment**, `Σ|x − x̄|` over the anchor
    /// set: how many anchors there are *and* how far out they reach,
    /// in one number, from the same free tally.
    ///
    /// Rises with count and with spread, which is what an anchor plate
    /// actually trades: a hundred anchors under the trunk resist less than
    /// forty spread across two metres. No constant in it — it is a sum of
    /// distances, so it needs no calibration to *be* right, only to be
    /// compared against a demand.
    pub anchor_moment: f32,
    /// **How well anchored this plant is for the crown it carries**, 0..1 —
    /// `anchor_moment` against the overturning demand of the shoot above
    /// it, clamped.
    ///
    /// The counterweight that makes root allocation a trade instead of a
    /// tax. `physical-trees-design-2026-08-23.md` §11.1: a quantity with a
    /// cost and no benefit has exactly one optimum, the minimum, and a
    /// working economy finds it and holds every plant there — the visible
    /// result being one root morphology everywhere, which is the complaint
    /// the owner has already made twice.
    ///
    /// **A whole-plant number, and that is load-bearing.** §11.7's first
    /// trap, and `CLAUDE.md`'s "which object does this rule evaluate": the
    /// quantities here — a crown's mass, its lever arm, an anchor
    /// half-width — are defined for a plant and undefined for a cell. This
    /// is read by `allocate_to_frontier`, an allocation decision; nothing
    /// in the plant lane schedules a structural check off it. Lane S owns
    /// the storm that collects.
    /// **The crown's overturning demand**, `Σ (collar − y)` over shoot
    /// tissue: mass times lever arm, in one sum from the walk
    /// `organism_upkeep` already runs.
    ///
    /// Stored beside `anchor_moment` rather than folded into
    /// `anchor_status`, for two reasons. The ratio of the two is what
    /// `ANCHOR_DEMAND` is derived from, and a clamped status cannot be
    /// divided back out to recover it. And lane S's wind-throw wants the
    /// unclamped demand directly — a gust delivers a moment, and what it
    /// has to beat is this.
    pub crown_moment: f32,
    pub anchor_status: f32,
    /// **Height of the shoot above the collar over stem width at the
    /// base** — read, never assigned (§11.2).
    ///
    /// `thicken` already ties width to the leaf mass above it, so a
    /// slender plant is what happens when the crown flushes faster than the
    /// stem thickens. Stored so lane S's wind-throw can pick its rung off a
    /// number the growth model produced rather than one a rule invented.
    pub slenderness: f32,
    /// **What this plant's standing tissue cost to run last tick** — the
    /// maintenance-respiration bill, summed over the walk that charges it.
    /// **What the plant earned last tick**, in carbon — the income
    /// `allocate_to_frontier` divides, stored so it can be read against
    /// `maintenance` without a second derivation of the same expression.
    ///
    /// Night-scaled, like the pool it feeds: this is money, not policy.
    pub income: f32,
    /// **Carbon set aside for reproduction, out of the same surplus growth
    /// draws on** — `plant::allocate_to_frontier` accrues it,
    /// `Behavior::Reproduce` spends it.
    ///
    /// **This exists because reproduction and growth were not competing
    /// at all.** `Reproduce` runs on `MatureBody` cells and used to debit
    /// `seed_cost` from *that cell's* carbon — and a mature cell sits
    /// pinned at `RESOURCE_SCALE`, refilled by `transport` rather than out
    /// of the growth pool (`plant.rs`'s own diagnostic: *"the trunk pinned
    /// at the `RESOURCE_SCALE` cap. The plant was never out of carbon."*).
    /// Meanwhile `allocate_to_frontier` distributes surplus **only to
    /// frontier cells**. Two separate accounts, so `tree.ron`'s
    /// `seed_cost: 0.3` was not a price in any meaningful sense, and the
    /// `seed_maturity` fence had to exist to stop what the price could
    /// not.
    ///
    /// Routing it here makes reproducing *not growing*, automatically,
    /// which is the standard allocation hierarchy — reproduction comes out
    /// of surplus after maintenance, and the cost of reproduction
    /// (reproducing trees measurably grow less that season) is among the
    /// better-documented plant allocation trade-offs.
    ///
    /// **Accrued and capped rather than spent-or-lost.** `Reproduce` fires
    /// on a per-cell chance, so a tick's surplus has to still be there
    /// when the roll lands; and provisioning seeds from stored reserve is
    /// what real plants do. The cap is what stops a long-lived plant
    /// banking an unbounded seed run — see `plant::REPRODUCTIVE_BUDGET_CAP`.
    pub reproductive_budget: f32,
    /// **The bill at unit price** — `Σ (q_peak / L_node)^MAINTENANCE_EXPONENT`
    /// over shoot tissue, before `MAINTENANCE_PER_NODE` multiplies it.
    ///
    /// Stored because a constant has to be *derived* rather than chosen, and
    /// the only honest way to derive this one is to read the quantity it
    /// scales on a stand the charge is not yet acting on. With the price at
    /// zero this and `income` give the price that puts a mature tree at any
    /// chosen bill-to-income ratio directly, instead of by bisecting a
    /// feedback loop. Keeping it afterwards means the same derivation can be
    /// re-run the day anything upstream of `q_peak` moves — which, on this
    /// quantity's own history (four re-derivations of `pipe_ratio`), it will.
    pub maintenance_basis: f32,
    pub maintenance: f32,
    /// **The part of that bill the plant could not pay**, in carbon.
    ///
    /// The continuous quantity `CLAUDE.md` prefers over a count of starving
    /// cells: counts give knife-edge margins and sums separate cleanly.
    /// Zero on any plant in surplus, which is most of them for most of
    /// their lives.
    pub maintenance_unpaid: f32,
    /// **Cells lost to starvation, cumulative** — the "did it fire at all"
    /// counter for crown recession and the root interior.
    ///
    /// `CLAUDE.md` is explicit that an image cannot answer this: a collapse
    /// rendered as coherent falling slabs was read as "chunks are working"
    /// while the body count was zero for the whole run. Every card this
    /// mechanism is posted on carries this number in its `meta`.
    pub starved_cells: u32,
    /// **Consecutive organism ticks on which this plant could not pay even
    /// the mass term of its own maintenance** — the clock that ends in
    /// death by starvation.
    ///
    /// Reset to zero the moment it can pay, so this counts a *sustained*
    /// failure rather than a bad afternoon. See
    /// `plant::STARVATION_DEATH_TICKS` for what it is measured against and
    /// why the comparison is the mass term rather than the whole bill.
    pub starving_ticks: u16,
    /// The **root collar** — the lowest row this organism's *shoot* tissue
    /// occupies, refreshed once per organism tick in the walk
    /// `plant::organism_upkeep` is already doing.
    ///
    /// This is the reference height for `Grow`'s turgor gate. Height above
    /// the collar is the one signal in the whole system that does **not**
    /// equalize when growth stops: carbon fills every cell to its cap,
    /// crowding decays everywhere within two ticks, and conductance relaxes
    /// to basal everywhere because there is no flux — but the apex is still
    /// at the top and the collar still at the bottom, permanently. That
    /// property is why the bound is built out of geometry rather than out
    /// of resource state (`Reports/tree-extension-biology.md` §2c).
    ///
    /// `None` until the first upkeep pass, or for an organism with no shoot.
    pub collar_y: Option<i32>,
    /// **This individual's genotype**, as one unit draw in `-1..=1` per
    /// trait — drawn once, from where it germinated, by
    /// `plant::seed_genotype`.
    ///
    /// The draws are stored and the *widths* are not, so a species file
    /// edit (or the tunables panel) still moves how much variation there
    /// is, while an individual keeps its own character. `plant::genotype`
    /// is the read: `1 + draw * variance`.
    ///
    /// **Why stored at all, when the previous version needed no storage.**
    /// The genotype used to be a pure function of `organism_id`, which is
    /// free but makes a plant's character a property of the world's event
    /// history rather than of the plant: ids come from planting order, so
    /// inserting one sapling anywhere earlier redraws every organism after
    /// it, and slot reuse wraps its
    /// generation at 16 and hands out bit-identical ids — repeats that
    /// would cluster spatially, exactly where they read as wrong. Keying on
    /// the germination coordinate costs these six floats and makes the
    /// genotype survive every one of those.
    ///
    /// All-zero is "exactly the species mean", which is what an organism
    /// that has not germinated yet reads as, and what moss stays at.
    pub genotype_draws: [f32; GENOTYPE_TRAITS],

    // --- creature fields (`Reports/creature-direction.md` §3b) ----------
    //
    // Empty/zero for every plant, and cheap enough that a per-species
    // split would cost more in dispatch than it saves in bytes.
    //
    // **The report lists seven fields here; three of them are these.** The
    // other four — `carrying: Option<MaterialId>`, `since_nest: u16`,
    // `brain_state: [f32; BRAIN_HIDDEN]` and `genome: Vec<f32>` — arrive
    // with the ants in stage 3, when there is code that reads them. Two of
    // the four cannot even be spelled until stage 3 defines the brain
    // constants. Written down here so the next session does not have to
    // re-derive the list or wonder whether the omission was an oversight.
    /// **Chain order, head first.** Plants leave it empty.
    ///
    /// `cells` is *membership* and this is *sequence* — a distinction only
    /// movement needs, and the reason it is a second field rather than a
    /// different view of the same one. A body follows its head by each
    /// segment stepping into its predecessor's old position, which is a
    /// question about order that a `HashMap` cannot answer.
    /// **This individual's heritable body traits** — `CREATURE_TRAITS`
    /// holds the slot map, and `TRAIT_GUT_BIAS` is the only live slot.
    ///
    /// A byte-copy of `CreatureDef::traits` at spawn today: nothing
    /// reproduces, so every creature is its species' ancestral value and
    /// the only way to move one is to author it. S6 is where a child takes
    /// its *parent's* vector jittered by `CreatureDef::trait_variance`,
    /// and the storage is here now because the trait has to change
    /// behaviour before it can be worth inheriting.
    ///
    /// **Not in `genome`.** See `CREATURE_TRAITS` — a gut is not a synapse,
    /// and a body block that grows independently is what keeps S8 from
    /// shifting brain offsets.
    pub traits: [f32; CREATURE_TRAITS],
    pub chain: Vec<(i32, i32)>,
    /// The head's facing, as a **discrete 0..8 compass index** into
    /// `creature::DIRS` — never a float vector.
    ///
    /// Three problems die at once (`creature-direction.md` §4a): no
    /// `sin_cos`, which `Reports/emergent-world-architecture.md` §8d names
    /// as a cross-platform determinism trap; a turn of ±1 is exactly 45°,
    /// the Physarum literature's default rotation angle; and every sensor
    /// offset comes from one const table instead of a rounding rule.
    ///
    /// Stamped but unread while the worm is the only creature — its move
    /// is a 4-neighbour choice with no facing. The ants read it.
    pub heading: u8,
    /// Energy budget. `creature::CreatureState`'s scalar, relocated — the
    /// entire contents of the parallel per-creature storage this substrate
    /// replaced.
    pub energy: f32,
    /// What this creature is carrying, if anything. **One item, not a
    /// stack**, and deliberately *state* rather than a cell: making the
    /// carried grain a chain cell doubles every movement edge case (what
    /// happens when the cell it wants to move into is the thing it is
    /// holding?) for no payoff at all at the zoom a creature is seen at.
    pub carrying: Option<Carried>,
    /// Ticks since this creature last touched nest material.
    ///
    /// **This is how an ant finds its way home without ever asking where
    /// home is.** Its channel-A deposit is scaled down by how stale this
    /// is, so outbound ants paint a gradient that is freshest nearest the
    /// nest — and a laden ant walking *up* that gradient is walking home.
    /// No creature ever queries the nest's position; the field knows.
    pub since_nest: u16,
    /// **Measurement only — no creature ever reads this, and the moment one
    /// does, the homing model has changed and this doc is a lie.**
    ///
    /// Where this creature last touched nest material, and the furthest it
    /// has been from that point since. Together they are a *foraging range*:
    /// the depth of the excursion currently in progress.
    ///
    /// `since_nest` above cannot answer that question, and the counter built
    /// on it does not. `CreatureStats::nest_visits` guards on `since_nest >
    /// 0`, but `since_nest` is incremented unconditionally every tick, so
    /// the guard is true on every tick after the first and the counter
    /// increments on *any* move made while nest-adjacent. An ant that never
    /// leaves home scores one per tick: measured on one ant, a nest and no
    /// food at all, `moves 648, nest_visits 389` — a ratio of 0.600 where a
    /// trip counter reads zero. It counts loitering.
    ///
    /// Two properties make this immune to both halves of that:
    ///
    /// - **Spatial, not temporal.** `since_nest` counts ticks, and
    ///   `tick_interval` is 6, so its scale is a species constant rather
    ///   than a distance. This is in cells.
    /// - **Re-anchored on every contact.** An ant walking along a 32-cell
    ///   nest patch touches nest at every step, so the anchor follows it and
    ///   the depth stays at 1. Loitering *on* the nest cannot manufacture an
    ///   excursion, which is the failure `since_nest` has — 136 of 142 of
    ///   its resets were loitering.
    ///
    /// Anchored at the spawn position, because an ant hatches at home.
    pub forage_anchor: (i32, i32),
    /// See `forage_anchor`. Chebyshev cells, saturating; reset to 0 at every
    /// nest contact.
    pub forage_max: u16,
    /// Persisted hidden-layer activations, so recurrence has something to
    /// read. Zero for anything without a brain.
    pub brain_state: [f32; super::brain::BRAIN_HIDDEN],
    /// **The heritable genome** — `brain::GENOME_LEN` weights for a
    /// creature, empty for plants until the plant migration adopts the
    /// shared mechanism (`Reports/creature-direction.md` §7a).
    pub genome: Vec<f32>,

    // --- plant fields ---------------------------------------------------
    /// The highest row this organism's shoot tissue reaches, refreshed in
    /// the same upkeep walk as `collar_y`. With the collar it gives the
    /// shoot's vertical span, which is what `acrotony` positions a bud
    /// against. `None` until the first upkeep pass.
    pub shoot_top_y: Option<i32>,
    /// **Event counters, because a contact sheet cannot show whether a
    /// mechanism fired.** A collapse was once read as "chunks are working"
    /// while the body counter said zero for the whole run; every discrete
    /// architectural event gets a counter printed beside the picture for
    /// exactly that reason. Stored, not derived — events cannot be
    /// reconstructed from world state after the fact.
    pub sympodial_forks: u32,
    /// Growth steps taken under a plagiotropic reference — says whether a
    /// species' `tropism` tiers ever actually ran.
    pub plagiotropic_steps: u32,
    /// Growth steps taken inside a lateral's `internode` straightness
    /// budget — zero means the budget never bound and the shape is still
    /// the old free meander.
    pub rigid_steps: u32,
    /// Laterals launched, and the sum of the angles they actually left at.
    ///
    /// **The mean of these two is the counter that matters for
    /// `branch_angle`**, and it is deliberately the *achieved* angle rather
    /// than a count of how often the scoring ran. Growth is on an
    /// 8-neighbourhood, so a species asking for 90° cannot always get it;
    /// a counter that only said "the angle code executed 400 times" would
    /// be true and useless, which is the failure this project keeps
    /// rediscovering. A mean of 47° against a target of 90° says the lever
    /// is weak — which no contact sheet could tell you.
    pub lateral_departures: u32,
    pub departure_angle_sum: f32,
    /// **This individual's colour**, as absolute band indices into the
    /// `leaf` and `wood` palettes — drawn once at germination by
    /// `plant::seed_genotype`, from the same (world seed, germination
    /// coordinate) key the genotype uses and for the same reason: colour
    /// should be a property of the plant, not of the world's planting
    /// order.
    ///
    /// Stored rather than recomputed because the germination coordinate is
    /// not kept anywhere else, and resolved to an absolute index here
    /// rather than an offset so the read at every cell-creation site is a
    /// field access and not a species lookup plus a modulo.
    ///
    /// Both are 0 until germination, which is the first band of whatever
    /// palette the material has — the pre-band look.
    pub foliage_band: u8,
    pub bark_band: u8,
    /// **This individual's genome came from a parent, not from where it
    /// landed.** `plant::seed_genotype` redraws a genotype from
    /// `(world seed, germination coordinate)` — which is right for a seed
    /// the player or a scene planted, and *destroys heredity* for a seed
    /// another plant set. This flag is what tells the two apart, and it is
    /// the whole difference between a population that evolves and one that
    /// re-rolls itself every generation.
    pub inherited: bool,
    /// How many ancestors deep this individual is. 0 for anything planted;
    /// a seed's parent's value plus one otherwise.
    ///
    /// Purely diagnostic and worth the two bytes: "did reproduction happen"
    /// is exactly the kind of discrete event a contact sheet cannot show,
    /// and a stand that looks lush while every plant reads generation 0 is
    /// a stand where nothing has bred.
    pub generation: u16,
    /// Seeds this organism has set. The other half of the same question.
    pub seeds_set: u32,
    /// **This individual's discrete genes** — see [`DISCRETE_LOCI`]. One
    /// small integer per locus, inherited whole and mutated by *jumping*
    /// rather than drifting, which is what makes a population clump instead
    /// of smearing.
    ///
    /// For a planted seed these are seeded from the species file, so an
    /// authored species is the *starting point* a population diverges from
    /// rather than a fixed identity it is stuck with.
    pub alleles: [u8; DISCRETE_LOCI],
    /// **This seed was told "not yet" at least once.** Set on the
    /// germination path's not-ready branch and read in `germinate`, so
    /// `World::seeds_germinated_after_waiting` counts only the seeds that
    /// actually waited for water rather than every seed that ever sprouted.
    ///
    /// A bool rather than a tick count because the question is binary: did
    /// dormancy do anything here. The count of *deferrals* would be a
    /// property of the polling interval, not of the mechanic.
    pub deferred_germination: bool,
    /// **This individual is dead and what is left of it is rotting.** Set
    /// by `plant::organism_upkeep` the tick an organism is found holding no
    /// vital cell (see `Species::is_vital`), and never cleared.
    ///
    /// **One-way, deliberately.** Nothing that reaches this state can leave
    /// it — the test is "no cell that could earn, grow for free, germinate
    /// or flush", so there is no path back by construction, and a flag that
    /// could flicker would let a plant rot halfway and then recover, which
    /// is neither a plant nor a corpse. It is also what makes the flag a
    /// safe gate for a *cause* of death that is not starvation: the herb
    /// package's post-fruiting annual death sets this and the same rot pass
    /// carries it out, with no second mechanism and no second tuning
    /// (`Reports/plant-morphology-reach-2026-08-23.md` §7 call 3).
    ///
    /// Distinct from having an empty cell list, which is what
    /// `World::free_organism` keys on: this is the state *between* dying
    /// and the last cell going, and before this existed there was no such
    /// state — an organism was live until its cells were gone, so a dead
    /// trunk held its slot for ever.
    pub senescent: bool,
}

/// How many independently-jittered traits a genotype carries — the width of
/// both `Behavior::Grow::genotype_variance` and
/// `OrganismState::genotype_draws`, which must agree because one indexes
/// the other.
///
/// **The slot map, positional forever** (`Reports/plant-genome-design.md`,
/// signed off 2026-08-18). Slots 0/2/3 are read by the shoot's `Grow`,
/// 1/5/8 by the root's `Grow` (from the RootTip entry's own vector — that
/// separation is what lets root and shoot diverge within one individual),
/// 4/6/7/9 by whole-plant passes that borrow the shoot vector:
///
///   0 shoot branch chance        5 root tropism gain
///   1 root branch chance         6 root:shoot allocation bias
///   2 shoot plastochron          7 stomatal closure point
///   3 turgor per cell            8 root penetration force
///   4 pipe ratio                 9 strain-response gain
///
/// **Slot 9 is capacity, not yet a trait: it has a width and a draw and
/// no consumer.** It is the heritable half of a reaction norm — how
/// strongly *this individual* re-allocates carbon away from height and
/// into root and stem when it is repeatedly loaded (thigmomorphogenesis,
/// in the botany). The point of spending a slot rather than a constant is
/// that a constant makes plasticity something the author decided and a
/// slot makes it something selection can act on: the population can
/// discover how responsive it should be, and different lineages can
/// settle differently. The response curve itself is a later package.
///
/// **Appended, not re-purposed, and that was a deliberate call.** Slots
/// 1 and 5 set the precedent for re-purposing a measured-dead slot, and
/// re-purposing here would have cost nothing in bytes. It would have cost
/// the measurement record a second time — only slots 0/2/3/4 survived the
/// last re-map comparable, and the F4 megastudy re-run is already queued
/// against the current numbering. Appending is exempt from the
/// never-renumber rule for a mechanical reason rather than a stylistic
/// one: `plant::seed_genotype` keys each draw on `rng::stream(world_seed,
/// x, y, slot)`, so a slot's value is a function of its own index and
/// nothing else, and adding one draws a stream nobody had drawn before.
/// `plant::tests::a_genome_slots_draw_is_a_pure_function_of_its_own_index`
/// asserts exactly that, and
/// `plant::tests::expressing_the_appended_genome_slot_changes_no_plant`
/// grows one stand twice in a run -- slot 9 expressed, then at zero
/// width -- and requires the two to be identical.
///
/// The one place appending is *not* automatically free is the mutation
/// loop in `plant::set_seed`, which draws one jitter per slot from a
/// shared `Rng` — a tenth slot consumes a tenth draw and would shift
/// every draw after it. See that function for how the sequence is held.
///
/// Slots 1 and 5 were `upward_weight` and `light_weight`, measured inert
/// across 1,024 genomes at ±40% / ±50% and held at zero width in every
/// species that grows. **A slot dead by measurement in every species may
/// be re-purposed once, with the measurement record re-baselined; a live
/// slot, never** — a draw that never expressed rewrites no measured
/// phenotype, which is the property the never-renumber rule below exists
/// to protect. The megastudy re-baselines at this re-map; only slots
/// 0/2/3/4, whose meanings did not move, are comparable across it.
pub const GENOTYPE_TRAITS: usize = 10;

/// How many heritable **body traits** a creature carries — the width of
/// both `CreatureDef::traits` (the authored ancestral values) and
/// `OrganismState::traits` (what this individual actually got).
///
/// **A separate block from the 584-slot brain genome, by design.** The
/// genome is a wiring matrix laid out from reserved dimensions
/// (`brain::GENOME_LEN`), and every one of its slots is a synapse weight;
/// a gut is not a synapse. Keeping traits out of it means S8 can grow the
/// body block without moving a single brain offset — the exact failure
/// `brain.rs`'s reserved-dimension layout exists to prevent, and the one
/// the "re-lay the genome output-major" dead end walks into
/// (`creature-evolution-plan.md` §6).
///
///   0 gut_bias — where this animal's digestion sits on the diet axis
///
/// **Positional forever, on the same terms as `GENOTYPE_TRAITS`**: a slot
/// dead by measurement in every species may be re-purposed once with the
/// measurement record re-baselined; a live slot, never.
pub const CREATURE_TRAITS: usize = 1;

/// Slot 0 of `CREATURE_TRAITS`: **diet as one heritable number**, `-1`
/// (plant matter) to `+1` (flesh), scored against `MaterialDef::food_class`
/// on the same axis through `creature::diet_yield`'s matched filter.
///
/// One scalar rather than a per-class vector, and that is a measured call
/// (E4, `creature-evolution-plan.md` §2.5): a normalised vector's overall
/// magnitude is a free dimension with nothing selecting on it, so a
/// histogram of its alleles measures its own drift and reads as a result.
/// A scalar on a bounded axis has no such dimension.
pub const TRAIT_GUT_BIAS: usize = 0;

/// **Discrete genes, and why a continuous genome cannot produce species.**
///
/// `genotype_draws` jitters ten scalars around a species mean. Run a
/// population on that and you get a Gaussian cloud — *a spectrum*, by
/// construction, however long it runs and however hard selection pushes.
/// There is no setting of a continuous genome that yields two clumps.
///
/// Clusters need a locus that takes one of a few values and mutates by
/// *jumping* between them. Then a population sits on a value, spreads
/// continuously around it via `genotype_draws`, and occasionally throws an
/// individual onto a neighbouring value — which either establishes and
/// becomes a second cluster or does not. That is the shape of a species.
///
/// This is also what the botany says. `Reports/tree-architecture-variety-
/// review.md` §3.0: Hallé's 23 architectural models are enumerated by a
/// handful of *categorical* choices — monopodial/sympodial,
/// orthotropic/plagiotropic — not by tuning scalars. The discrete axes were
/// already in the engine as authored per-species constants; making them
/// heritable alleles is what lets the simulation find combinations nobody
/// wrote down.
pub const DISCRETE_LOCI: usize = 6;

/// **Leaf construction economics** — the acquisitive↔conservative axis,
/// and the foliage band the individual wears; one allele, both meanings.
/// Allele 0 is the expensive leaf (more carbon per unit light, more water
/// per tick — `LEAF_RATE_ALLELES` / `LEAF_TRANSPIRATION_ALLELES`), allele
/// 1 the cheap one; Liebig decides who wins where. The band mapping is
/// the exact consumer this locus had when it was purely cosmetic
/// (`LOCUS_FOLIAGE`), so the colour is now the visible face of a real
/// gene rather than a free one — a dark tree is dark because its leaves
/// are expensive (`Reports/plant-appearance-design.md` §7,
/// `plant-genome-design.md` §4.2). Whether allele 0 reads *darker* on
/// screen is the species' own palette ordering; shrub's runs the other
/// way, which is accepted rather than papered over.
pub const LOCUS_LEAF_ECONOMY: usize = 0;
/// Departure angle class — scales the species' `branch_angle`.
pub const LOCUS_BRANCH_ANGLE: usize = 1;
/// Straightness-budget class — scales the species' `internode`.
pub const LOCUS_INTERNODE: usize = 2;
/// Monopodial (0) or sympodial (1), overriding the species default.
pub const LOCUS_SYMPODIAL: usize = 3;
/// Orthotropic (0) or plagiotropic (1) on non-trunk tiers.
pub const LOCUS_TROPISM: usize = 4;
/// **Wood density** — the pioneer↔dense strategy axis, the best-studied
/// trade in tree ecology. One multiplier (`WOOD_DENSITY_ALLELES`) scales
/// the branch-holding strength (`Material::max_cantilever_reach`, applied
/// per individual in `structural::organism_structural_tick`) and the
/// carbon price of every `Grow` step together: cheap wood outgrows dense
/// wood and loses more of itself to load. The bark band derives from this
/// allele (`bark_band_for_density`), so bark tone is a readout of a real
/// gene, exactly as foliage tone is.
pub const LOCUS_WOOD_DENSITY: usize = 5;

/// How many alleles each locus has.
///
/// `LOCUS_LEAF_ECONOMY` is 2, matching the two foliage bands every
/// species declares. It was 6 ("bounded by the palette") while the locus
/// was cosmetic, and that shape carried a latent bias: mutation drew
/// uniformly over six alleles while the consumer clamped to the species'
/// two bands, so a jump landed on the top band five times as often as
/// the bottom one. Two alleles for two strategies removes the bias by
/// construction.
pub const LOCUS_ALLELES: [u8; DISCRETE_LOCI] = [2, 3, 3, 2, 2, 3];

/// Multipliers on the species' `branch_angle`, one per allele of
/// `LOCUS_BRANCH_ANGLE`. Spread wide enough that the three are *visibly*
/// different plants and not three tunings of one — on `tree`'s 70° trunk
/// value these give roughly 28°, 70° and 112°: a fastigiate column, the
/// species as authored, and a splayed low crown.
pub const BRANCH_ANGLE_ALLELES: [f32; 3] = [0.4, 1.0, 1.6];

/// Multipliers on the species' `internode`. A short budget lets the
/// environment steer a lateral almost immediately (a meandering, twiggy
/// habit); a long one holds it straight for a real run.
pub const INTERNODE_ALLELES: [f32; 3] = [0.4, 1.0, 2.0];

/// Photosynthetic-rate multiplier per `LOCUS_LEAF_ECONOMY` allele:
/// acquisitive, then conservative. Never varied alone — it is paired with
/// `LEAF_TRANSPIRATION_ALLELES` at every consumer, because a free rate
/// axis would be selection candy with no bill attached. First-pass
/// values; the paired wet/dry sweep is what sets them.
pub const LEAF_RATE_ALLELES: [f32; 2] = [1.2, 0.85];

/// Transpirational-demand multiplier per `LOCUS_LEAF_ECONOMY` allele —
/// the bill for `LEAF_RATE_ALLELES`. Income is min(light, water)-bounded
/// (Liebig, `plant::allocate_to_frontier`), so the expensive leaf wins
/// where light is the binding constraint and the cheap one where water
/// is. That crossover is the whole reason this locus exists.
pub const LEAF_TRANSPIRATION_ALLELES: [f32; 2] = [1.5, 0.7];

/// Strength-and-price multiplier per `LOCUS_WOOD_DENSITY` allele:
/// pioneer, as-authored, dense. Applied to `max_cantilever_reach` and to
/// the shoot/root `Grow.cost` together — one number for both on purpose,
/// so tuning cannot quietly turn the trade into a free lunch. Secondary
/// thickening pays no carbon today, so the price binds on extension
/// only; recorded in `Reports/plant-genome-design.md` §4.1 rather than
/// hidden.
pub const WOOD_DENSITY_ALLELES: [f32; 3] = [0.75, 1.0, 1.35];

/// The strength-and-price multiplier this genome's density allele selects.
///
/// **One accessor, because the multiplier has to reach every site that
/// budgets against the cost it scales, not just the site that spends it.**
/// It landed on `Grow`'s own gate first and nowhere else, and the three
/// places that stake or cap a frontier *in units of that cost* went on
/// using the unscaled number: a dense plant's re-initiated root tip and
/// flushed bud were staked below their own first step (so the courtesy
/// their comments promise inverted into a guaranteed failure), and
/// `break_buds`' income-over-price tip cap let dense plants open a
/// frontier they could not feed while capping pioneers below what they
/// could. `CLAUDE.md`: when a fix changes what a number *means*,
/// re-deriving what reads it is part of the fix.
///
/// Clamps rather than indexes blindly -- stale state carrying a widened
/// allele must not walk off the table.
pub fn wood_density(alleles: &[u8; DISCRETE_LOCI]) -> f32 {
    WOOD_DENSITY_ALLELES[(alleles[LOCUS_WOOD_DENSITY] as usize).min(WOOD_DENSITY_ALLELES.len() - 1)]
}

/// Which bark band a density allele wears, inside the species' declared
/// range — proportional, so the dense end of the allele range takes the
/// top band. With today's two-band ranges and three alleles this reads
/// `[first, first, first + 1]`: pioneer and as-authored share the low
/// band and dense stands out. Judged on a sheet like every colour call;
/// `count == 0` (moss, anything pre-band) keeps the pre-band 0, exactly
/// as the old free draw did.
pub fn bark_band_for_density(bands: PaletteBands, allele: u8) -> u8 {
    if bands.count == 0 {
        return 0;
    }
    let n = LOCUS_ALLELES[LOCUS_WOOD_DENSITY].max(1) as u16;
    bands.first + ((allele.min(n as u8 - 1) as u16 * bands.count as u16) / n) as u8
}

/// Chance that one locus jumps to a different allele when a seed is set.
///
/// **Much rarer than continuous drift, and that asymmetry is the mechanism.**
/// `MUTATION_SIGMA` moves every trait a little every generation, which is
/// what gives a cluster its internal spread; this fires seldom, which is
/// what lets a cluster *persist* long enough to be one. Make it common and
/// the discrete loci smear into just another continuous axis, which is the
/// exact failure this whole construction exists to avoid.
pub const DISCRETE_MUTATION_CHANCE: f32 = 0.03;

/// Re-exported so `world.rs` can size `OrganismState::brain_state`
/// without importing `brain` for one constant.
pub const BRAIN_HIDDEN_FOR_STATE: usize = super::brain::BRAIN_HIDDEN;

pub struct SpeciesRegistry {
    species: Vec<Species>,
    by_name: HashMap<String, SpeciesId>,
}

const EMBEDDED: &[&str] = &[
    include_str!("../../assets/species/moss.ron"),
    include_str!("../../assets/species/tree.ron"),
    include_str!("../../assets/species/worm.ron"),
    include_str!("../../assets/species/ant.ron"),
    include_str!("../../assets/species/beetle.ron"),
    // Appended, never inserted — the same convention `material.rs`'s
    // EMBEDDED list states and for the weaker version of the same reason:
    // a species resolves by name (`id_of`), so there is no id contract to
    // break here, but keeping one arrival order across both registries is
    // what stops the next merge having to reason about it again.
    include_str!("../../assets/species/conifer.ron"),
    include_str!("../../assets/species/shrub.ron"),
    // **A form probe, not a shipped species** -- WP-C of `Reports/plant-
    // implementation-plan.md`. It asks whether one corner of the Grow
    // envelope is a real form *class* or just another small tree, at the
    // cost of one file plus one contact sheet. Embedded because a headless
    // harness reads only this list (P-7), not the assets directory.
    //
    // **Two siblings shipped with it and have been retired against the
    // owner's verdicts** -- `weeping.ron` ("same plant" as `tree`) and
    // `prostrate.ron` ("Not that different" from this file, 2/5). Both
    // verdicts and what they cost are in `Reports/plant-evolution-
    // design.md` §4a's register; a condemned probe does not outlive its
    // verdict here.
    include_str!("../../assets/species/creeper.ron"),
    // **A shipped species, not a probe** -- WP-B3. Listed after the probes
    // because it postdates them, not because it ranks with them: grass is
    // the species the plant programme is for, and the one that differs from
    // a tree on all four of the axes in `plant-evolution-design.md` §4a.
    include_str!("../../assets/species/grass.ron"),
];

/// Where the loader looks for species files, relative to the working
/// directory — mirrors `material::ASSET_DIR`.
pub const ASSET_DIR: &str = "assets/species";

#[derive(Debug)]
pub enum SpeciesError {
    Io(std::io::Error),
    Parse { file: String, error: String },
}

impl std::fmt::Display for SpeciesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeciesError::Io(e) => write!(f, "reading species: {e}"),
            SpeciesError::Parse { file, error } => write!(f, "{file}: {error}"),
        }
    }
}

impl std::error::Error for SpeciesError {}

impl SpeciesRegistry {
    fn empty() -> Self {
        Self { species: Vec::new(), by_name: HashMap::new() }
    }

    /// The species files compiled into the binary, so the engine always has
    /// a working set even without an assets directory beside it — mirrors
    /// `MaterialRegistry::builtin`.
    pub fn builtin() -> Self {
        let mut reg = Self::empty();
        for (i, source) in EMBEDDED.iter().enumerate() {
            match ron::from_str::<SpeciesDef>(source) {
                Ok(def) => reg.upsert(def),
                Err(e) => panic!("embedded species {i} is malformed: {e}"),
            }
        }
        reg
    }

    /// Re-read every `.ron` in `dir` over the current set. No
    /// `resolve_references`-equivalent pass: unlike materials, a species
    /// file never names another species, so there is nothing to resolve
    /// after the fact.
    pub fn reload(&mut self, dir: impl AsRef<Path>) -> Result<usize, SpeciesError> {
        let mut paths: Vec<_> = std::fs::read_dir(dir.as_ref())
            .map_err(SpeciesError::Io)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ron"))
            .collect();
        paths.sort();

        let mut defs = Vec::new();
        for path in &paths {
            let source = std::fs::read_to_string(path).map_err(SpeciesError::Io)?;
            let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
            let def = ron::from_str::<SpeciesDef>(source).map_err(|e| SpeciesError::Parse {
                file: path.file_name().unwrap_or_default().to_string_lossy().into(),
                error: e.to_string(),
            })?;
            defs.push(def);
        }

        let count = defs.len();
        for def in defs {
            self.upsert(def);
        }
        Ok(count)
    }

    fn upsert(&mut self, def: SpeciesDef) {
        let species = Species::from(def);
        match self.by_name.get(&species.name) {
            Some(id) => self.species[id.0 as usize] = species,
            None => {
                let id = SpeciesId(self.species.len() as u16);
                self.by_name.insert(species.name.clone(), id);
                self.species.push(species);
            }
        }
    }

    #[inline]
    pub fn get(&self, id: SpeciesId) -> &Species {
        &self.species[id.0 as usize]
    }

    /// Overwrite a species' starting genome — **for ablation harnesses
    /// only**, and it is worth having rather than hand-editing `.ron`
    /// between runs for two reasons. Editing an asset changes nothing
    /// until the next build (the `include_str!` gotcha that has produced
    /// whole invalid sweeps here), and a sweep that has to rebuild between
    /// points cannot vary a knob within one process, so it cannot hold
    /// everything else fixed.
    pub fn set_genome(&mut self, id: SpeciesId, genome: Vec<f32>) {
        self.species[id.0 as usize].genome = genome;
    }

    /// Overwrite one `Grow` arm's `genotype_variance` — **harness only**,
    /// same caveat as `set_genome`, and it exists for one specific test
    /// shape worth naming.
    ///
    /// The guard on appending a genome slot has to answer "does the new
    /// slot change anything about the plants?", and the honest form of
    /// that question is a **comparison, not a stored number**: grow the
    /// same stand twice in one process, once with the slot expressed and
    /// once with its width at `0.0`, and check the two are identical.
    /// A hardcoded fingerprint answers it too, and then goes stale every
    /// time any lane touches plant behaviour — which cost two wrong
    /// diagnoses in one evening (`Reports/open-bugs-handoff.md`).
    ///
    /// Per-`World` rather than a global switch on purpose: the test
    /// binary runs tests on many threads at once, so a process-wide
    /// "effective genome width" would leak into whatever else happened
    /// to be running. Widths are read live at every use (see
    /// `Behavior::Grow::genotype_variance`), so setting one before a run
    /// is enough and nothing needs redrawing.
    ///
    /// A no-op if the species has no `Grow` on that cell type.
    pub fn set_genotype_variance(&mut self, id: SpeciesId, cell_type: CellType, variance: [f32; GENOTYPE_TRAITS]) {
        let Some((_, behaviors)) = self.species[id.0 as usize].cell_types.iter_mut().find(|(ct, _)| *ct == cell_type) else {
            return;
        };
        for b in behaviors.iter_mut() {
            if let Behavior::Grow { genotype_variance, .. } = b {
                *genotype_variance = variance;
            }
        }
    }

    /// Overwrite a species' creature parameters — **harness only**, same
    /// caveat as `set_genome`.
    ///
    /// Needed because a scene cannot create scarcity without control of the
    /// energy budget. `ant.ron`'s 900 starting energy against an idle cost
    /// of 0.10 is roughly 7,500 ticks of life, and a sampling run is 1,000:
    /// nothing can starve inside it, so every genome scored the same
    /// survival — including one with no connections at all, which cannot
    /// move. An environment where the outcome is identical for every
    /// behaviour measures nothing.
    pub fn set_creature(&mut self, id: SpeciesId, def: CreatureDef) {
        self.species[id.0 as usize].creature = Some(def);
    }

    pub fn id_of(&self, name: &str) -> Option<SpeciesId> {
        self.by_name.get(name).copied()
    }

    pub fn len(&self) -> usize {
        self.species.len()
    }

    pub fn is_empty(&self) -> bool {
        self.species.is_empty()
    }
}

// --- `Cell::aux` encoding for organism-owned cells --------------------
//
// Only meaningful when `cell.organism_id() != 0` — see `Cell::organism_id`'s
// own doc and `Reports/organism-substrate-design.md` §2 for why an
// unowned `Plant`/`Creature` cell (hand-painted wood, a fully-reclaimed
// dead tree's former trunk) keeps `aux`'s pre-existing, kind-specific
// meaning instead. `Cell` itself stays agnostic to this distinction —
// these are free functions over the raw `u16`, not methods on `Cell`.

/// How many branch orders a species may parameterise separately.
///
/// Four is trunk / limb / branch / twig, which is as many tiers as any of
/// the classical architectural models distinguishes by name. It is a cap on
/// *distinct parameter sets*, not on depth — order saturates here, so a
/// tenth-order twig behaves like a fourth-order one.
pub const BRANCH_ORDERS: usize = 4;

/// `serde` default for a multiplier that means "unchanged". Needed because
/// `0.0` — what `Default` would give — silently disables the field it
/// guards instead of leaving it alone.
fn one() -> f32 {
    1.0
}

/// `serde` default for `leaf_cluster`: one cell per node, the behaviour
/// before clustering existed.
fn one_u8() -> u8 {
    1
}

/// Which way an axis of a given branch order wants to point — the
/// orthotropic/plagiotropic distinction, which is one of the four
/// discriminating axes of the Hallé–Oldeman–Tomlinson architectural
/// models and the single biggest silhouette lever the engine had no way
/// to express.
///
/// **Orthotropic** axes grow toward the vertical (a poplar's everything, a
/// fir's trunk). **Plagiotropic** axes grow *outward*, holding the
/// direction they left their parent in — a fir's branch tiers, and the
/// building block of Troll's model, which the literature calls the
/// commonest architecture of the temperate broadleaf flora. The reference
/// direction a plagiotropic axis holds is its own stored `heading`'s
/// horizontal sense, so the data this needs has existed since momentum
/// landed; this enum only lets a species point different tiers different
/// ways.
///
/// `upward_weight` weights the pull toward whichever reference the tier
/// selects — for a plagiotropic tier it is an *outward* weight, the name
/// notwithstanding; renaming a field every genome salt table references
/// was judged worse than one doc line here.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize)]
pub enum Tropism {
    Orthotropic,
    Plagiotropic,
}

/// `serde` default: every tier orthotropic, which is exactly the old
/// hardcoded behaviour.
fn all_orthotropic() -> ByOrder<Tropism> {
    ByOrder::uniform(Tropism::Orthotropic)
}

/// `serde` default: no tier forks sympodially — the old behaviour.
fn all_monopodial() -> ByOrder<bool> {
    ByOrder::uniform(false)
}

/// A `Grow` parameter that varies with **branch order** — the number of
/// lateral branchings between a tip and the seed.
///
/// **This is where architecture comes from.** Fourteen tips drew a whip
/// because they were fourteen copies of one rule; the classical tree
/// grammars are all parameterised by arrays indexed exactly like this, and
/// `Reports/tree-architecture-research.md` §0b asks for it by name. A trunk
/// that branches rarely and leafs rarely, under twigs that do both often,
/// is a bare bole with a crown on it — and it comes out of the data rather
/// than out of a rule that has to guess which cells are "trunk", which a
/// cell cannot know locally.
///
/// **A short list is a short plant.** Deserializes from a RON list of one
/// to `BRANCH_ORDERS` values and pads with the *last* one, so `[0.05]` is a
/// species that does not distinguish orders at all, `[0.02, 0.3]` is a
/// shrub — one trunk tier and everything above it identical — and four
/// values is a tree. Padding with the last value rather than a default is
/// what makes the short form mean "and so on", instead of silently
/// zeroing the orders the author did not write.
///
/// `Copy`, because `Behavior` is copied out of the species table into a
/// fixed dispatch buffer once per cell per tick (see `plant.rs`'s
/// `behavior_buf` and the allocation count in its comment). A `Vec` here
/// would put ~350,000 allocations back into a 6,000-frame run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ByOrder<T> {
    values: [T; BRANCH_ORDERS],
}

impl<T: Copy> ByOrder<T> {
    /// The value for `order`, saturating at the last tier.
    pub fn at(&self, order: u8) -> T {
        self.values[(order as usize).min(BRANCH_ORDERS - 1)]
    }

    /// Every tier the same — for a caller building one in code rather than
    /// reading it from a species file (tests, and `Default`).
    pub fn uniform(value: T) -> Self {
        Self { values: [value; BRANCH_ORDERS] }
    }
}

/// Every tier at `T`'s own default — what `#[serde(default)]` on a
/// `ByOrder` field means, and for `branch_angle`/`internode` the zero is
/// deliberately the "unset, keep the old behaviour" value rather than a
/// neutral one.
impl<T: Copy + Default> Default for ByOrder<T> {
    fn default() -> Self {
        Self::uniform(T::default())
    }
}

impl<'de, T> Deserialize<'de> for ByOrder<T>
where
    T: Deserialize<'de> + Copy,
{
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let listed = Vec::<T>::deserialize(d)?;
        // An empty list is an author error, not "use the default": there is
        // no value to pad from, and silently picking one would be the same
        // class of mistake as a bit budget deciding a constant.
        let Some(&last) = listed.last() else {
            return Err(serde::de::Error::custom(format!("a per-branch-order list needs 1 to {BRANCH_ORDERS} values, not 0")));
        };
        if listed.len() > BRANCH_ORDERS {
            return Err(serde::de::Error::custom(format!("a per-branch-order list holds at most {BRANCH_ORDERS} values, got {}", listed.len())));
        }
        let mut values = [last; BRANCH_ORDERS];
        for (slot, &listed) in values.iter_mut().zip(&listed) {
            *slot = listed;
        }
        Ok(Self { values })
    }
}

/// Behavioural cap on how much carbon one cell may hold.
///
/// **Was an encoding parameter; is now only a clamp.** Until Decision 2
/// step 2c it also set the quantization of the 8-bit `aux` field the
/// scalar lived in (one step was `RESOURCE_SCALE / 255`), which is exactly
/// the "tail wagging the dog" `Reports/plant-substrate-v2-design.md` §3a
/// names: a constant chosen for headroom silently deciding precision.
/// `OrganismCell::carbon` is a plain `f32`, so this value now does one
/// job — bounding what `Photosynthesize` and `Absorb` may accumulate —
/// and changing it no longer changes the resolution of anything.
pub const RESOURCE_SCALE: f32 = 4.0;

/// Behavioural cap on how much water one cell may hold, on its own scale.
///
/// Mirrors `RESOURCE_SCALE` deliberately: the two currencies are compared
/// only as *fractions of their own cap* (`water / WATER_SCALE` is the
/// stomatal term), never against each other, so keeping the caps equal
/// means a reading of "half full" means the same thing in both and no
/// conversion constant has to exist.
pub const WATER_SCALE: f32 = 4.0;

/// Pack a `CellType` into bits 0-3 of `aux`.
///
/// **Bits 4-15 are now free for an organism-owned cell** — they held the
/// resource scalar (4-11) and canopy density (12-15) until both moved to
/// `OrganismCell`. Deliberately left zero rather than reused: the next
/// per-cell scalar belongs in the sidecar too, and `design-philosophy.md`
/// §2b's objection to re-packing is that a bit budget is what forced every
/// previous scalar to be quantized around the wrong constant.
pub fn pack_cell_type(cell_type: CellType) -> u16 {
    cell_type as u16
}

/// The `CellType` encoded in `aux`, or `None` if the low 4 bits don't
/// match a known variant — a stale or corrupted value should read as
/// "nothing recognized" rather than panicking or silently aliasing to
/// whichever variant happens to sit at that bit pattern.
///
/// Stays inline on `Cell` (rather than moving to `OrganismCell` with the
/// scalars) because three call sites must answer "what kind of cell is
/// this" from a bare `Cell` with no `&World` in hand — the transport
/// wall test, `structural.rs`'s branch and traversal filter, and
/// `World::organism_active_tip_count`. That is
/// `Reports/plant-substrate-v2-design.md` §3c's rule applied as written:
/// what a caller holding only a `Cell` must be able to answer stays on
/// the cell; everything that is a *scalar* moves out.
pub fn cell_type(aux: u16) -> Option<CellType> {
    match aux & 0b1111 {
        0 => Some(CellType::Seed),
        1 => Some(CellType::GrowingTip),
        2 => Some(CellType::MatureBody),
        3 => Some(CellType::Leaf),
        4 => Some(CellType::RootTip),
        5 => Some(CellType::DormantBud),
        6 => Some(CellType::Head),
        7 => Some(CellType::Segment),
        _ => None,
    }
}

/// Behavioural cap on the canopy-density scalar —
/// `Reports/tree-rewrite-design.md` §2b's self-avoidance signal.
///
/// **Like `RESOURCE_SCALE`, no longer an encoding parameter — and this one
/// was hiding a real bug.** Packed into 4 bits it had 15 steps of 0.267,
/// and `CANOPY_DENSITY_DECAY_PER_TICK` is a halving, so a decaying deposit
/// reached one quantum and then *stopped*: 0.267 × 0.5 = 0.133, which packs
/// back to `round(0.5) = 1` — the same quantum, forever. Measured, not
/// reasoned about: 0.800 → 0.533 → 0.267 → 0.267 → 0.267, a fixed point at
/// tick 3.
///
/// So canopy density had a **permanent floor of 0.267 on every cell that
/// ever received a deposit**, and the mechanism whose entire stated purpose
/// is to "let later growth reclaim space near mature wood" could never
/// fully release that space. The decay looked like it worked, because its
/// first two steps did.
///
/// It survived because the test guarding it
/// (`transport_no_longer_decays_density_itself`) asserts an isolated cell
/// at full scale stays at full scale — far away from the floor, which sits
/// at the bottom of the range. That is `CLAUDE.md`'s "ask what a metric
/// counts when nothing is wrong", in test form.
///
/// The value is now a plain `f32` and decays continuously to zero. **That
/// changed what `crowding_weight` sees**, and the measured consequence is
/// recorded in `plant::CANOPY_DENSITY_DECAY_PER_TICK` and left for the
/// economy pass to re-tune rather than compensated for here — per
/// `Reports/plant-substrate-v2-design.md` §10's "no `.ron` edits in this
/// step".
pub const CANOPY_DENSITY_SCALE: f32 = 4.0;

/// The per-cell scalars for one organism-owned cell — Decision 2's
/// sidecar (`Reports/plant-substrate-v2-design.md` §3c).
///
/// Plain `f32`s: no packing, no scale constants doubling as encoding
/// parameters, and no quantization. That is the entire point. The two
/// fields here are the two that used to live in `Cell::aux`; the doc's
/// §3c also lists `water`, `age`, `anoxia_ticks` and `plastochron`, and
/// none of them are added here — each waits for the decision that gives it
/// a caller, matching `OrganismState`'s own recorded convention of not
/// adding state speculatively. (`plastochron` in particular is *not*
/// coming: it turned out to be lineage state and already rides on
/// `ActiveKind::Organism`, recorded in `PLAN.md` as the one place §3a is
/// wrong.)
///
/// **No `pos` field**, unlike §3c's sketch: that shape indexed a
/// `Vec<Option<OrganismCell>>` by a slot number stored back in `aux`, and
/// a position-keyed map makes the position the key instead. See
/// `OrganismState::cells` for why that swap was made.
#[derive(Clone, Debug, PartialEq)]
pub struct OrganismCell {
    /// Photosynthate. Was the 8-bit `aux` field; `Absorb`/`Photosynthesize`
    /// still clamp it to `RESOURCE_SCALE`.
    pub carbon: f32,
    /// `Reports/tree-rewrite-design.md` §2b's crowding signal, clamped to
    /// `CANOPY_DENSITY_SCALE`.
    pub canopy_density: f32,
    /// **Weighted distance to this organism's nearest structural anchor**,
    /// recomputed once per organism tick by `plant::anchor_support`.
    /// `u16::MAX` means *unreached* — nothing this cell is connected to
    /// touches the ground, so the piece it belongs to has come off.
    ///
    /// This replaced `structural::organism_is_supported`, which ran a fresh
    /// bounded BFS **outward from the cell being checked** on every check.
    /// That was wrong twice, and neither was tuning:
    ///
    /// - It was bounded by `max_unsupported_span` (8 for `wood`) measured
    ///   from the checked cell, so on a 150-cell tree any check fired in the
    ///   crown could not reach the ground within the budget and read
    ///   "unsupported". Scheduling one mid-organism amputated the canopy —
    ///   772 cells against 20,213 from that single line — which is why
    ///   growth and abscission both deliberately scheduled no checks at all.
    /// - It traversed `NEIGHBOURS_4` while `Grow` places children at 8, so
    ///   a diagonally-grown branch read as disconnected. Same rule
    ///   `reachable_from_anchors` was already fixed for.
    ///
    /// Computing it **from the anchors outward**, once per organism, fixes
    /// both: there is no span budget to run out of, a severed crown is
    /// unreached however far away it is, and the per-check cost drops to a
    /// field read.
    ///
    /// Two distinct questions come off this one number, and keeping them
    /// distinct is the point: `== u16::MAX` is *attachment* (is this cell
    /// still part of a piece that reaches ground), and `> effective_span` is
    /// *cantilever* (is it too far out along its own load path for the load
    /// it carries). A bare reachability bit would have silently superseded
    /// the second rule while leaving its test green.
    pub support: u16,
    /// Per-face carbon **efflux** conductance, indexed in `NEIGHBOURS_4`
    /// order — Decision 6 (`Reports/plant-substrate-v2-design.md` §7b).
    ///
    /// `carbon_conductance[k]` governs export *out of this cell* across
    /// face `k`. The neighbour on the other side stores its own,
    /// independent, opposing value, and the two are not required to agree:
    /// that asymmetry is the whole mechanism. A cell's *polarity* — a
    /// single direction, which only `Grow` wants — is derived from these
    /// on demand (`supply_direction`), never stored.
    ///
    /// Four independent values rather than the research sketch's packed
    /// 3-bit direction, and the deciding reason is that **a single stored
    /// direction cannot represent a branch point**, which is the one case
    /// the mechanism exists to resolve: a cell feeding two children has two
    /// faces genuinely carrying flux, and an enum would have to pick one,
    /// deciding apical dominance in the data structure before the update
    /// rule ever ran.
    pub carbon_conductance: [f32; 4],
    /// **Branch order** — how many lateral branchings separate this cell
    /// from the seed. 0 is the trunk.
    ///
    /// Inherited unchanged when a tip continues its own shoot, incremented
    /// when a tip throws a lateral. Purely local: a tip reads its own value
    /// and nothing else, and every cell a tip creates is stamped from it.
    ///
    /// In the sidecar rather than in `Cell::aux`'s free bits 4-15, which
    /// `pack_cell_type`'s doc offers, because `design-philosophy.md` §2b's
    /// objection to re-packing is precisely what this branch has already
    /// been bitten by twice: a 4-bit canopy density had a fixed point at
    /// one quantum and never decayed. An order would survive 4 bits fine
    /// today, but the rule "the next per-cell scalar goes in the sidecar"
    /// is worth more than the two bytes.
    pub order: u8,
    /// **The support this cell carries, as a monotone high-water mark** —
    /// the intercepted light of every leaf above it in the plant's own
    /// topology, accumulated basipetally, and never allowed to fall.
    ///
    /// Two things needed this and neither could be had from the row scan it
    /// replaces. `plant::thicken`'s pipe-model gate used "leaves in rows
    /// above me", which is a *geometric* filter: a limb on one side of the
    /// plant counted toward a stem on the other side that does not supply
    /// it, and a leaf below a cell it feeds counted for nothing. Support is
    /// a property of the vascular graph, not of the y coordinate.
    ///
    /// **Monotone on purpose**, and Palubicki is explicit about why:
    /// *"branch width is not decreased when leaves and branches are shed…
    /// the model requires a memory of past leaves and branches."* A trunk
    /// does not get thinner in autumn. The memory is also what
    /// distinguishes a plant that has *lost* foliage from one that never
    /// had any — the two are identical in every instantaneous signal, and
    /// telling them apart is the prerequisite for a damaged plant
    /// mobilising reserves instead of quietly giving up (see
    /// `plant::break_buds`).
    pub q_peak: f32,
    /// **The support this cell carries *right now*** — the same basipetal
    /// sum as `q_peak`, before the high-water `max`.
    ///
    /// `accumulate_support` has always computed this and thrown it away on
    /// the line that latches the peak. Keeping it costs one `f32` per cell
    /// and answers a question the peak provably cannot: **is this cell
    /// still carrying any living foliage?** The peak says what it once
    /// carried and, being monotone on purpose, keeps saying so for ever.
    ///
    /// That difference is what `organism_upkeep`'s die-back rule is keyed
    /// on, and the reason it is safe. A cell at `q_now == 0` supports no
    /// leaf anywhere above it in the plant's own topology, so removing it
    /// cannot strand foliage: the crown recedes from its abandoned tips
    /// inward, and the trunk — which carries the whole live crown — is not
    /// a candidate while a single leaf remains. A rule keyed on the peak
    /// instead would price the trunk highest *and* make it the first thing
    /// to die, which is a hole in a stem, not crown recession.
    ///
    /// `plant::break_buds`' known defect (`q_peak` remembers, nothing reads
    /// the difference) wants exactly this pair as well; P5's resprout is
    /// the other consumer and needs no second field.
    pub q_now: f32,
    /// The direction this shoot is **actually travelling**, carried forward
    /// with inertia rather than re-derived from the immediate
    /// neighbourhood each step.
    ///
    /// **This is why growth looked erratic.** `continuation_weight` scored
    /// against `supply_direction`, which is a read of the cells *touching*
    /// this one — a one-cell baseline, quantized to eight directions, on a
    /// stem that is itself only one or two cells wide. Its estimate of
    /// "where am I heading" is therefore mostly noise, and every step
    /// re-rolled it, so a trunk wandered instead of rising and a limb
    /// zig-zagged instead of sweeping. Real shoots have momentum: a stem
    /// already lignified behind the apex physically cannot turn sharply.
    ///
    /// `(0.0, 0.0)` means "no heading yet" and falls back to the old local
    /// read, which is correct for a cell that has just germinated and has
    /// no history to carry.
    pub heading: (f32, f32),
    /// **Error-diffusion carry for `stem_stiffness`** — the fraction of a
    /// cell the shoot still owes its own heading, in cells, on each axis.
    ///
    /// `Grow` accumulates `heading` into this, steps to whichever lattice
    /// neighbour lands nearest, and subtracts the step it took. That is
    /// Bresenham's residual, and it is what lets an eight-neighbour walk
    /// spell a direction it has no lattice vector for: a 17-degree shoot
    /// becomes a regular run of verticals with a periodic diagonal instead
    /// of an independent coin flip per step. The eye reads the first as a
    /// line and the second as jitter, which is the whole of the
    /// difference.
    ///
    /// Zero on a fresh cell, and left at zero for any species that does not
    /// set `stem_stiffness` — nothing reads it in that case.
    pub growth_residual: (f32, f32),
    /// **Hydraulic path length from the collar, in cells** — how far sap has
    /// actually had to travel to reach here, not how high up it is.
    ///
    /// This replaces `collar - y` in `Grow`'s turgor gate, and the reason is
    /// a measured gap rather than a preference for realism. The vertical
    /// form bounds *height* and bounds width not at all: a cell two hundred
    /// columns sideways at collar height reads `height = 0` and full margin.
    /// A single tree planted with twenty rows of sky therefore never stops
    /// growing — 24,946 cells and still climbing at frame 295,000 — because
    /// it cannot go up, so it goes sideways forever. With 190 rows it
    /// plateaus at frame 180,000. Self-shading was the only thing bounding
    /// width, and it is enough in a tall scene and nothing in a shallow one.
    /// See `Reports/branch-angle-and-the-width-bound.md`.
    ///
    /// Path length bounds both axes with the mechanism already there, and it
    /// is what the biology says anyway: water potential falls with the
    /// hydraulic path the xylem has to push through, not with altitude, so a
    /// 200-cell horizontal limb is under the same constraint as a 200-cell
    /// trunk (`Reports/tree-extension-biology.md` §2c's own source is about
    /// path resistance).
    ///
    /// **Propagated at creation — parent + 1 — not recomputed.** A plant is
    /// acyclic and does not move, so a cell's distance from the collar is
    /// fixed the moment it exists; there is no pass and no per-tick cost. It
    /// also strictly improves on the property that made height attractive
    /// (`collar_y`'s doc: the one signal that does not equalize when growth
    /// stops) — height is recomputed against a collar that can move, and
    /// this never changes at all.
    pub path_len: u16,
    /// **A primed lateral site** — this cell has been marked by the tip
    /// that grew past it as a place a branch may later start.
    ///
    /// The repair for a gate that could not open. Root branching used to be
    /// a second `Grow` in the *same tick* as the primary step, so a tip had
    /// to hold two steps' carbon at once; measured, it cleared that bar
    /// twice in twelve thousand frames and the 0.04 roll fired zero times
    /// (`Reports/plant-genome-design.md` §8a). The economy is not the
    /// defect — a root tip cannot photosynthesise, lives on allocation and
    /// spends at first affordance, which is intended — the *shape of the
    /// purchase* was. Priming splits the decision from the bill: the tip
    /// marks a site for free as it passes, and the site buys its own
    /// lateral later, out of the carbon that reaches it, whenever that
    /// clears a step's cost.
    ///
    /// `PLAN.md`'s own M16 research note prescribes exactly this and says
    /// why it is also the better biology: real laterals are primed by an
    /// oscillator in the tip that marks roughly evenly spaced sites, and
    /// only later does local resource decide which ones actually branch —
    /// so spacing comes out regular instead of noisy.
    ///
    /// Sidecar, not a `Cell` bit: all sixteen aux bits are spoken for.
    pub primed: bool,
}

impl Default for OrganismCell {
    /// **Conductance starts at `CONDUCTANCE_MIN`, not zero, and that is
    /// load-bearing.** Zero conductance gives zero flux gives zero
    /// reinforcement, forever — a bootstrap deadlock. The literature has
    /// the same term for the same reason: `ρ₀` in §7a's equation is a
    /// *basal* insertion rate, constitutive and flux-independent, because
    /// carriers are inserted before they are polarized. Every fresh cell is
    /// therefore perfectly isotropic and differentiates only from flux it
    /// actually carried.
    fn default() -> Self {
        // **`support` starts at 0 — "anchored" — and the opposite default
        // is the dangerous one here.** `World::set` inserts a `default()`
        // sidecar for every organism cell as it is created
        // (`reindex_organism_cell`), so every cell a tree grows carries this
        // value until its organism's next tick, up to
        // `ORGANISM_TICK_INTERVAL` frames later. Defaulting to `u16::MAX`
        // would mean *unreached*, which `organism_structural_tick` reads as
        // "this piece has come off" — so any structural check landing in
        // that window would shatter tissue that had simply not been walked
        // yet.
        //
        // This is the mirror image of the terrain case
        // `structural::compute_world_distances` records, where `aux = 0` on
        // untouched rock was "a lie that happened to look right" and made
        // the world immune. There, 0 meant *immune*; here, `u16::MAX` means
        // *destroy on sight*. A rule whose action is destructive has to be
        // biased toward the answer that defers, not the one that fires: an
        // unwalked cell reads supported and is corrected within one organism
        // tick, so failure is delayed, never falsely triggered.
        Self {
            carbon: 0.0,
            canopy_density: 0.0,
            support: 0,
            carbon_conductance: [CONDUCTANCE_MIN; 4],
            order: 0,
            q_peak: 0.0,
            q_now: 0.0,
            heading: (0.0, 0.0),
            growth_residual: (0.0, 0.0),
            path_len: 0,
            primed: false,
        }
    }
}

// --- The shared connectivity primitive ---------------------------------

/// Bounded breadth-first search from `anchors`, visiting only cells for
/// which `matches` is true, capped at `cap` cells total (anchors included).
/// Reaching the cap stops the search early rather than continuing — the
/// caller's own reachability question ("is X within the searched set") is
/// still answerable, just not a promise that *every* connected cell was
/// found once a large structure exceeds the cap.
///
/// One primitive, meant to be called several different ways — see
/// `Reports/organism-substrate-design.md` §5: `structural.rs`'s
/// organism-owned `Plant` branch (`matches` = same `organism_id`, `is_body`
/// kind), a possible M17 `Solid` verification pass (`matches` = accept any
/// `Solid`/`Plant`), and a future downstream-resource count (`matches` =
/// same `organism_id`, tally a cell type while walking). Only the first is
/// wired up in this pass.
pub fn reachable_from_anchors<S: CellSurface>(
    surface: &S,
    anchors: impl IntoIterator<Item = (i32, i32)>,
    matches: impl Fn(Cell) -> bool,
    cap: usize,
) -> HashSet<(i32, i32)> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    for pos in anchors {
        if visited.len() >= cap {
            break;
        }
        if matches(surface.get(pos.0, pos.1)) && visited.insert(pos) {
            queue.push_back(pos);
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        if visited.len() >= cap {
            break;
        }
        // **Eight-connected, and it has to be: `Grow` places children at
        // eight neighbours, diagonals included.** This traversal was
        // four-connected, so a tree that had taken any diagonal step — which
        // is most of them, since `Grow` scores all eight directions and a
        // straight vertical stem is the rare case — read back as a set of
        // disconnected fragments.
        //
        // Found by `plant.rs`'s `shedding_every_leaf_does_not_disconnect_
        // the_stem`, which reported 4 of 30 cells reachable from the base of
        // an intact tree. It was written to catch a different bug and caught
        // this one instead, which is the argument for asserting the property
        // rather than the mechanism.
        //
        // The consequence was not cosmetic. `thicken()` — the only
        // production caller — counts downstream `Leaf`/`GrowingTip` cells
        // through this to decide whether Shinozaki's pipe ratio is cleared,
        // so it has been counting a small fragment of the canopy rather than
        // the canopy. Thickening firing rarely and patchily, reported from
        // live play, is downstream of this.
        //
        // Diagonal adjacency really is connection here: the cell was placed
        // there *by* growth from this one, so treating it as a join is
        // describing what happened rather than a modelling liberty.
        for (dx, dy) in NEIGHBOURS_8 {
            let next = (x + dx, y + dy);
            if visited.contains(&next) {
                continue;
            }
            if matches(surface.get(next.0, next.1)) {
                visited.insert(next);
                queue.push_back(next);
                if visited.len() >= cap {
                    break;
                }
            }
        }
    }
    visited
}

// --- Resource and canopy-density transport ------------------------------
//
// `Reports/organism-substrate-design.md` §3's `diffuse_resource`, moved off
// the CA sweep and onto a per-organism pass by
// `Reports/plant-substrate-v2-design.md` §3d. Still per-cell
// CA-resolution finite-difference diffusion, not `field.rs`'s coarse
// `FIELD_SCALE` grid (that resolution mismatch is exactly what
// `organism-substrate-design.md` §3 rejected the field grid for), and a
// different-organism or non-`Plant` neighbour is still a wall, exactly as
// `diffuse_heat` treats a material boundary. Still carries
// `Reports/tree-rewrite-design.md` §2b's canopy-density channel alongside
// resource -- one pass over both, not a second diffusion implementation.
//
// **Why it stopped being generic over `CellSurface`.** It has to read a
// scalar that now lives in `OrganismState`, and `ChunkView` deliberately
// carries no organism state -- §3d's wall, named there before this was
// built rather than discovered here. Of the three ways out, extending
// `CellSurface` was rejected for the reason this module already gives for
// cutting `TransportChannel`, and a split layout was rejected because the
// bit budget stays full and the next scalar reopens it. So the pass moved.
//
// **What that buys, beyond somewhere to put the scalars.** It removes the
// dependence on chunk wakefulness. Dispatched from the sweep, this ran on
// every organism cell of every *awake* chunk every frame and not at all
// otherwise -- measured at 22.8% of frames on the standard single-tree
// scene once the tree settled, against consumers that read it every
// `ORGANISM_TICK_INTERVAL` regardless. `PLAN.md` records that duty cycle
// as the gate on Decision 2 and calls it a correctness prerequisite rather
// than a storage tidy-up, because the moment `Photosynthesize` moves to
// `Leaf` only (Decision 4) a starved transport stops being invisible.

/// Diffusion rate for both channels -- see this module's own doc on why a
/// single shared rate, not yet a per-species `TransportChannel` value.
/// Clamped conceptually to the same 2D explicit-diffusion stability bound
/// (Fourier number <= 0.25) `fire.rs`'s `diffuse_heat`/`field.rs`'s own
/// diffusion already derive and respect; chosen well under that bound
/// rather than at it, since two channels diffusing through the same cell
/// every visited tick have more chances to compound than heat's single
/// channel does.
const DIFFUSION_RATE: f32 = 0.2;

/// How many diffusion iterations one organism tick runs.
///
/// **Set to reproduce the cadence this replaced, not chosen fresh.**
/// Dispatched from the CA sweep the rule ran once per frame per awake
/// chunk, so across one `ORGANISM_TICK_INTERVAL` (45 frames) a tree in an
/// awake chunk saw ~45 steps between two consecutive reads of the value.
/// Batching those 45 into one pass at tick time changes *when* the
/// intermediate states exist, not how far resource has travelled by the
/// time anything looks -- and nothing reads the scalar between ticks:
/// every consumer (`Grow`, `Divide`, `Photosynthesize`, `Absorb`) runs
/// inside `organism_tick`.
///
/// **This is `Reports/plant-substrate-v2-design.md` §7c's
/// `TRANSPORT_SUBSTEPS`, arrived at early.** That section makes the point
/// that 45 was never a decision -- it was "how often the sweep runs" --
/// and that making it an explicit parameter is the actual gain. It is a
/// first-class tuning target for the economy pass.
///
/// **Applies to the canopy-density channel only now that polarity has
/// landed** — carbon runs its own `CARBON_SUBSTEPS` against its own
/// stability bound. Kept at 45 deliberately: density's behaviour is what
/// `crowding_weight` is tuned against, and changing its substep count
/// while introducing polarity would make the two indistinguishable.
const DENSITY_SUBSTEPS: u32 = 45;

// --- Polarity: canalization of the carbon channel -----------------------
//
// `Reports/plant-substrate-v2-design.md` Decision 6 (§7). The mechanism is
// Sachs's canalization as formalized by Mitchison and by Prusinkiewicz et
// al. (2009, PNAS 106:17431-17436, already cited in `plant.rs`'s module
// doc): flux through a face reinforces that face's capacity to carry flux,
// so a channel that starts marginally ahead pulls further ahead, and the
// choice becomes hard to reverse once made. That hysteresis is the point --
// it is what converts a sequence of coin flips into a committed branch.

/// Constitutive, flux-independent conductance insertion per organism tick
/// (`ρ₀` in §7a). Required, not decorative — see `OrganismCell::default`.
const VEIN_BASAL: f32 = 0.1;
/// Per-tick conductance turnover.
const VEIN_DECAY: f32 = 0.1;
/// Flux-driven conductance gain, and with `VEIN_BASAL` it sets the
/// canalization contrast — the one number the behaviour actually depends
/// on (`CANALIZATION_CONTRAST`).
///
/// **Tuned from 2.9 (contrast 30:1) to 0.9 (10:1) in the economy pass**,
/// against the measured establishment rate rather than an aspiration.
/// `plant-substrate-v2-design.md` §7e names the contrast as the first knob
/// to reach for when seedlings stall, ahead of `CARBON_SUBSTEPS` and well
/// ahead of the seed reserve, and the measurement agrees. Across all six
/// variants of `examples/debug_tree_variants.rs` at n=16:
///
/// | contrast | seedlings established | organism cells | strand contrast achieved |
/// |---|---|---|---|
/// | 30:1 | 54/96 (56%) | 4,906 | 20.7x of 30 |
/// | **10:1** | **70/96 (73%)** | **6,321** | **6.1x of 10** |
///
/// **Re-derived after `RESOURCE_SCALE` was made to bind in transport**,
/// since the table above was measured against an economy where a cell
/// could hold 23x its stated cap. The choice survives, on better numbers
/// and for a new reason — at n=12 across all six variants:
///
/// | contrast | established | cells | undiff / partial / vascular |
/// |---|---|---|---|
/// | 30:1 | 56/72 | 4,117 | 38% / 32% / 30% |
/// | **10:1** | **68/72** | **7,265** | 29% / 37% / **33%** |
/// | 5:1 | 70/72 | 7,485 | 29% / 41% / 29% |
///
/// 5:1 now edges 10:1 on establishment (97% vs 94%) but produces *fewer*
/// fully vascular cells and halves the ceiling a strand can reach, so it
/// buys three points of germination by making the mechanism less able to
/// differentiate at all. 10:1 is the point where both are still good.
///
/// So 17 points of establishment and 29% more biomass, for a hierarchy
/// that is still unmistakably a hierarchy — and note the *fraction* of the
/// available range actually reached barely moved (69% -> 61%), so the
/// mechanism is working just as hard at the lower setting. Going further
/// to 5:1 was tried and rejected: it took the ceiling below what a strand
/// needs to be visibly distinct, and it broke the chain test by making the
/// assertion's own threshold unreachable.
///
/// **Set this to 0 and the whole mechanism
/// reduces exactly to the isotropic rule it generalizes** — the A/B switch
/// §7c asks for, and what makes a regression here bisectable.
///
/// Verified by rebuilding with it zeroed rather than through a runtime
/// knob: this is read in the transport inner loop, and threading a
/// parameter through to make it settable would put a branch there for a
/// test's benefit. Zeroed, **exactly three tests fail, and they are the
/// three that assert polarity specifically** —
/// `a_chain_canalizes_along_its_axis_and_not_across_it`,
/// `the_better_connected_tip_outdraws_the_hungrier_one`, and
/// `supply_direction_is_none_until_something_has_actually_been_carried`.
/// The boundedness and isotropy/conservation tests keep passing, which is
/// the correct split: those assert invariants the reduction preserves, so
/// a suite that went fully green *or* fully red would mean the A/B was
/// testing the wrong thing.
const VEIN_GAIN: f32 = 0.9;

/// Flux-free fixed point, `VEIN_BASAL / VEIN_DECAY`. Undifferentiated
/// parenchyma sits here.
pub const CONDUCTANCE_MIN: f32 = VEIN_BASAL / VEIN_DECAY;
/// Saturated fixed point, `(VEIN_BASAL + VEIN_GAIN) / VEIN_DECAY`. A fully
/// canalized strand approaches this and **provably cannot exceed it**,
/// because `Φ` saturates at 1 — the bounded form of §7a(3), chosen over
/// Mitchison's pure quadratic precisely because the quadratic diverges.
pub const CONDUCTANCE_MAX: f32 = (VEIN_BASAL + VEIN_GAIN) / VEIN_DECAY;

/// The only thing the behaviour actually depends on is the *ratio*
/// `CONDUCTANCE_MAX / CONDUCTANCE_MIN`, and it deserves a name: the
/// canalization contrast, `1 + VEIN_GAIN / VEIN_BASAL` = 30:1 as tuned.
/// Tune the contrast, not the three constants independently.
pub const CANALIZATION_CONTRAST: f32 = CONDUCTANCE_MAX / CONDUCTANCE_MIN;

/// Per-substep carbon transport coefficient.
///
/// Bounded by the same explicit-diffusion stability limit every other
/// diffusion in this engine respects, but applied to the **largest**
/// conductance in play rather than a typical one: `TRANSPORT_RATE *
/// CONDUCTANCE_MAX <= 0.25`. At contrast 10 that caps it at 0.025, and
/// 0.024 sits just under.
///
/// **Re-derived when the contrast moved, and that coupling is the point.**
/// Lowering the contrast raises the rate the bound permits, which is
/// precisely how a *seedling* benefits: undifferentiated tissue conducts
/// at `TRANSPORT_RATE * CONDUCTANCE_MIN`, so 30:1 gave it 0.008 and 10:1
/// gives it 0.024 — three times the transport before any strand exists.
/// Changing one of these without the other either breaks the stability
/// bound or throws away the gain.
const TRANSPORT_RATE: f32 = 0.024;

/// Carbon transport iterations per organism tick.
///
/// **Consequence worth stating before it is rediscovered as a bug:
/// unpolarized tissue now transports carbon far more slowly than the flat
/// `DIFFUSION_RATE` it replaces** — `TRANSPORT_RATE * CONDUCTANCE_MIN` is
/// 0.008 against 0.2, recovered only partially over these substeps. That is
/// correct and is the entire point: undifferentiated parenchyma *is* a poor
/// conductor, a vascular strand *is* a good one, and the biological
/// function of vascular tissue is that the contrast exists.
///
/// It also makes a falsifiable prediction: a fresh seedling with no
/// established vasculature is transport-limited and its first act is to
/// canalize a strand from its first source to its tip. **If seedlings
/// instead simply never get going, the two knobs are the canalization
/// contrast and this number, in that order — not the seed reserve.**
const CARBON_SUBSTEPS: u32 = 16;

/// Fallback for `J_REF` when a species defines no `Grow` cost to derive it
/// from. See `flux_reference` for why the species value is preferred.
const DEFAULT_FLUX_REFERENCE: f32 = 0.2;

/// The conductance response, a Hill function with exponent 2.
///
/// `Φ(J) = J² / (J_REF² + J²)`, deliberately resolving §7a's finding (1)
/// against (2)/(3) rather than picking a third option at random. It is
/// **convex below `J_REF`** — superlinear, so a face with a flux advantage
/// compounds it faster than linearly, which is the regime that makes
/// canalization actually canalize and produces §7a(1)'s loopless directed
/// trees. And it is **concave above**, saturating at 1, which is what makes
/// the rule non-divergent where the pure quadratic is provably divergent.
///
/// So the engine gets the quadratic's topology in the regime where topology
/// is being decided, and the bounded form's stability in the regime where a
/// long-running simulation would otherwise blow up.
fn conductance_response(flux: f32, flux_reference: f32) -> f32 {
    let j2 = flux * flux;
    j2 / (flux_reference * flux_reference + j2)
}

/// `J_REF`: the flux one growing tip's demand represents.
///
/// Taken from the species' own shoot `Grow.cost` rather than hardcoded, so
/// the operating point moves with the economy it is measuring. Below one
/// tip's worth of demand the response is competition-amplifying; above it,
/// it saturates. `tree.ron`'s `cost: 0.2` therefore puts `J_REF` at 0.2,
/// which is the value §7h's worked example uses.
///
/// The shoot's cost, not the root's (0.25), and not their mean: §7h works
/// the arithmetic against the canopy, and a single reference keeps the
/// response comparable across the whole organism. A species with no `Grow`
/// at all falls back to `DEFAULT_FLUX_REFERENCE`.
fn flux_reference(species: &Species) -> f32 {
    species
        .behaviors(CellType::GrowingTip)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { cost, .. } => Some(*cost),
            _ => None,
        })
        .filter(|c| *c > 0.0)
        .unwrap_or(DEFAULT_FLUX_REFERENCE)
}

/// The index in `NEIGHBOURS_4` of the face pointing back the other way —
/// cell *i*'s face `k` and its neighbour's face `opposite(k)` are the two
/// halves of one shared boundary.
const fn opposite_face(k: usize) -> usize {
    // NEIGHBOURS_4 is [-x, +x, -y, +y], so the pairs are (0,1) and (2,3).
    k ^ 1
}

/// Run one organism tick's worth of resource and canopy-density transport
/// over every cell `organism_id` owns.
///
/// Keeps the property that motivated the old placement verbatim -- a
/// `MatureBody` trunk cell relays resource even though it is deliberately
/// off the active-site schedule (`design-philosophy.md` §3) -- and gets it
/// from the organism's own cell list rather than from the sweep happening
/// to visit it, which is a strictly better answer to that requirement than
/// the workaround it replaces.
pub fn transport(world: &mut crate::sim::world::World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else {
        return;
    };
    if state.cells.is_empty() {
        return;
    }

    // **Sorted, and that is load-bearing.** `cells` is a `HashMap`, whose
    // iteration order is not stable across runs, and `f32` addition is not
    // associative -- so an unsorted order would make the accumulated sums
    // differ run to run. `PLAN.md` requires same-build determinism, so the
    // pass fixes a canonical order once here. Row-major, matching the
    // sweep's own convention.
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    // Resolve the topology once per tick rather than once per substep: for
    // each cell, the indices of its same-organism `Plant` 4-neighbours.
    // Growth happens at tick boundaries, never inside this loop, so the
    // adjacency cannot change while the substeps run. This is what makes
    // the substep loop pure contiguous indexing -- see `OrganismState::
    // cells` for why that, and not a slot index in `aux`, is where the
    // random-access cost went.
    let index: std::collections::HashMap<(i32, i32), usize> = cells.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    let mut neighbours: Vec<[Option<usize>; 4]> = Vec::with_capacity(cells.len());
    for &(x, y) in &cells {
        let mut row = [None; 4];
        for (k, (dx, dy)) in NEIGHBOURS_4.into_iter().enumerate() {
            let n = world.get(x + dx, y + dy);
            // The same wall test as before: a different organism, or a
            // non-`Plant` material, is a boundary. `resource_does_not_
            // cross_an_organism_boundary` asserts exactly this.
            if n.organism_id() != organism_id || world.materials.kind(n.material) != MaterialKind::Plant {
                continue;
            }
            row[k] = index.get(&(x + dx, y + dy)).copied();
        }
        neighbours.push(row);
    }

    let species_id = state.species;
    let flux_ref = flux_reference(world.species.get(species_id));

    let state = world.organism(organism_id).expect("checked above");
    let mut carbon: Vec<f32> = cells.iter().map(|p| state.cells[p].carbon).collect();
    let mut density: Vec<f32> = cells.iter().map(|p| state.cells[p].canopy_density).collect();
    let mut conductance: Vec<[f32; 4]> = cells.iter().map(|p| state.cells[p].carbon_conductance).collect();

    // --- Canopy density: the symmetric rule, unchanged ------------------
    //
    // **Only the carbon channel becomes polar** (§7f), and that is a
    // decision rather than a scope cut. Canopy density is not a transported
    // substance -- there is no vessel carrying it, no source, no sink and
    // no conserved quantity; it is a stigmergic proxy for "how much of my
    // own tissue is near here", deposited at creation and read by `Grow` as
    // a crowding penalty. Making it follow established conductance would
    // make it blind exactly where it needs to see: a tip must avoid dense
    // canopy in *any* direction, and a clump sitting off-vein is the
    // crowded direction it most needs to detect.
    //
    // **Deviates from §7f in one detail, deliberately.** That section says
    // density can run through the general pairwise form with a constant
    // conductance and come out "bit-for-bit the symmetric average it is
    // today". It cannot: the mean rule is `R + (mean(n) - R) * RATE`, while
    // pairwise with constant `c` gives `R + RATE*c*n*(mean(n) - R)` -- a
    // factor of `n`, the neighbour count, which varies per cell. Matching
    // them needs `c = 1/n`, which is per-cell rather than per-face and so
    // neither symmetric nor conserving. Since density's behaviour is what
    // `crowding_weight` is tuned against, the tested rule is kept verbatim
    // and the general form is used only where it is actually correct.
    let mut next_density = density.clone();
    for _ in 0..DENSITY_SUBSTEPS {
        for i in 0..cells.len() {
            let (mut density_sum, mut n) = (0.0f32, 0u32);
            for slot in neighbours[i].iter().flatten() {
                density_sum += density[*slot];
                n += 1;
            }
            // An isolated organism cell (every neighbour a wall) has
            // nothing to exchange with and nothing to do -- and in
            // particular does *not* decay here. Decay lives on
            // `organism_tick`'s own cadence; see
            // `plant::CANOPY_DENSITY_DECAY_PER_TICK`'s doc for why a decay
            // applied on the diffusion pass erased a fresh deposit before
            // any neighbour's much-less-frequent `Grow` check could read
            // it -- a real bug, found by live verification.
            next_density[i] = if n == 0 {
                density[i]
            } else {
                (density[i] + (density_sum / n as f32 - density[i]) * DIFFUSION_RATE).clamp(0.0, CANOPY_DENSITY_SCALE)
            };
        }
        std::mem::swap(&mut density, &mut next_density);
    }

    // --- Carbon: the pairwise carrier rule ------------------------------
    //
    // §7c. For the face between cells i and j:
    //
    //     efflux(i->j) = RATE * c_ij * R_i      (carrier-mediated, source-
    //     efflux(j->i) = RATE * c_ji * R_j       concentration proportional)
    //     net J_ij     = RATE * (c_ij*R_i - c_ji*R_j)
    //
    // Three properties, each a reason this is the right form and not merely
    // a workable one. It **reduces exactly to Fickian diffusion when
    // polarity is absent** (`c_ij = c_ji = c` gives `RATE*c*(R_i - R_j)`),
    // so polarity is a strict generalization of what shipped before and
    // `VEIN_GAIN = 0` recovers it exactly. It is **exactly conserving** --
    // every unit leaving i arrives at j in the same statement -- where the
    // mean rule was not, each cell independently moving toward its
    // neighbours' mean in place. And it **matches the biology**: PIN efflux
    // carriers sit on a specific membrane face, which is why the model is
    // indexed per ordered cell pair rather than per cell.
    //
    // Each face is visited exactly once, from the cell on its -x/-y side,
    // and both halves of the exchange are applied together. Deltas are
    // accumulated and applied after the sweep rather than in place, so the
    // result carries no dependence on which cell was visited first.
    let mut flux = vec![[0.0f32; 4]; cells.len()];
    let mut delta = vec![0.0f32; cells.len()];
    for _ in 0..CARBON_SUBSTEPS {
        delta.iter_mut().for_each(|d| *d = 0.0);
        for i in 0..cells.len() {
            // Only the +x and +y faces, so the -x/-y ones are not counted a
            // second time from the neighbour's side.
            for k in [1usize, 3usize] {
                let Some(j) = neighbours[i][k] else { continue };
                let out = conductance[i][k] * carbon[i];
                let back = conductance[j][opposite_face(k)] * carbon[j];
                let mut net = TRANSPORT_RATE * (out - back);
                // **Bounded by the receiving cell's headroom**, exactly as
                // the capillary exchange in `update.rs` is bounded by its
                // neighbour's `water_capacity`.
                //
                // `RESOURCE_SCALE` is documented as a cap on what one cell
                // may hold, and until polarity landed it held without
                // anything enforcing it here: the old symmetric rule moved
                // each cell toward its neighbours' *mean*, which cannot
                // exceed the largest value present, so a clamp at the two
                // sources (`Photosynthesize`, `Absorb`) was enough. The
                // pairwise carrier rule has no such property -- it moves
                // carbon *down a conductance gradient*, so a cell that is a
                // net sink accumulates without bound while sources keep
                // topping up to the cap every tick.
                //
                // Measured before this clamp: a single cell reached **92.0
                // against a scale of 4.0**, twenty-three times its stated
                // maximum and enough to fund 460 growth steps at
                // `tree.ron`'s `cost`. The economy tuning above was done
                // against that, so it is worth re-checking after.
                let headroom = if net > 0.0 { RESOURCE_SCALE - carbon[j] } else { RESOURCE_SCALE - carbon[i] };
                net = net.clamp(-headroom.max(0.0), headroom.max(0.0));
                delta[i] -= net;
                delta[j] += net;
                flux[i][k] += net;
                flux[j][opposite_face(k)] -= net;
            }
        }
        for i in 0..cells.len() {
            carbon[i] = (carbon[i] + delta[i]).clamp(0.0, RESOURCE_SCALE);
        }
    }

    // --- The conductance update (§7d, §7e) ------------------------------
    //
    // Fed by the whole tick's accumulated flux rather than a per-substep
    // value, which gives the two-timescale structure the biology has: bulk
    // flow is fast, carrier turnover is slow.
    //
    // **Only the positive part reinforces, and the clamp is not a fudge.**
    // Conductance on cell i's face toward j is an *efflux* capacity. Net
    // import across that face is evidence that j's opposing face is
    // conducting, and j is the cell that should be credited -- it will be,
    // by its own entry. Crediting both sides of a reversing face would make
    // a face that merely oscillates read as a strong channel, which is the
    // opposite of what canalization means.
    for i in 0..cells.len() {
        for k in 0..4 {
            let j = flux[i][k].max(0.0);
            let c = conductance[i][k];
            conductance[i][k] = c + VEIN_BASAL + VEIN_GAIN * conductance_response(j, flux_ref) - VEIN_DECAY * c;
        }
    }

    let Some(state) = world.organism_mut(organism_id) else {
        return;
    };
    for (i, pos) in cells.iter().enumerate() {
        if let Some(slot) = state.cells.get_mut(pos) {
            slot.carbon = carbon[i];
            slot.canopy_density = density[i];
            slot.carbon_conductance = conductance[i];
        }
    }
}

/// The direction carbon is arriving from, as a normalized vector, or `None`
/// when nothing has established yet.
///
/// **This is what `Grow`'s `away_from_growth` becomes** (§7g). For each of
/// the four faces, the neighbour on the other side stores its own
/// conductance on the face pointing *back at this cell*, and that value is
/// exactly "how strongly does that neighbour export into me". Summing the
/// face directions weighted by those values gives the supply direction.
///
/// **Why this is strictly better than the geometric version it replaces,
/// on that version's own terms.** `tree-rewrite-design.md` §2a introduced
/// `away_from_growth` because "grow away from the parent" is undefined at a
/// branch point, and solved it by averaging over every same-organism
/// neighbour — which treats a *sibling* tip, created by the same branch
/// event, as equally "behind you" even though it feeds you nothing. That
/// sibling drags the average sideways and makes two fresh branches repel
/// each other for purely positional reasons. Here a sibling that exports
/// nothing sits at `CONDUCTANCE_MIN` and contributes almost nothing, while
/// the stem cell actually feeding this tip has a ratcheted face and
/// dominates. **The mechanism now distinguishes "adjacent to" from
/// "supplied by"**, which is what §2a was approximating and could not
/// express.
///
/// Returns `None` when every supply weight is still at the basal floor — a
/// seed's very first `Grow`, before any flux has been carried anywhere — so
/// the caller can fall back to the geometric rule and §2a's proof survives
/// the degenerate case unchanged.
pub fn supply_direction(world: &crate::sim::world::World, x: i32, y: i32) -> Option<(f32, f32)> {
    let organism_id = world.get(x, y).organism_id();
    if organism_id == 0 {
        return None;
    }
    let state = world.organism(organism_id)?;
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    let mut strongest = 0.0f32;
    for (k, (dx, dy)) in NEIGHBOURS_4.into_iter().enumerate() {
        let Some(n) = state.cells.get(&(x + dx, y + dy)) else { continue };
        // The neighbour's face pointing back at this cell.
        let weight = n.carbon_conductance[opposite_face(k)];
        strongest = strongest.max(weight);
        sx += weight * dx as f32;
        sy += weight * dy as f32;
    }
    // Still isotropic everywhere: no information here, and normalizing
    // would amplify float noise into a confident direction.
    if strongest < CONDUCTANCE_MIN * (1.0 + 1e-3) {
        return None;
    }
    let len = (sx * sx + sy * sy).sqrt();
    if len < 1e-6 {
        return None;
    }
    Some((sx / len, sy / len))
}

// --- Ported field-read helpers ------------------------------------------
//
// `Reports/tree-rewrite-design.md` §7: moved from `plant.rs`'s old private
// `tree_tip_tick`/`moisture_pull` functions to here, unchanged in formula,
// so `Grow`'s dispatch (species-agnostic) can call them regardless of
// which species is growing -- the actual generality `design-philosophy.md`
// §3's "not tree-specific" framing asks for, demonstrated rather than
// asserted. All three take `&World` directly (not `&dyn CellSurface`):
// `field_at_bilinear` is a `World`-only method the parallel sweep's
// `ChunkView` does not (and should not) reimplement, since `Grow`'s own
// dispatch already runs from `organism_tick`, which is M16 active-site
// code with real `&mut World` access, not a generic `CellSurface` sweep
// rule the way `diffuse_resource` above is.

/// Below this speed, the field's velocity at a position doesn't count as
/// wind at all -- ported unchanged from `plant.rs`'s `WIND_SPEED_THRESHOLD`.
const WIND_SPEED_THRESHOLD: f32 = 0.05;

/// Gentle phototropic lean: bias toward the brighter of "here" and "just
/// above" -- ported unchanged from `tree_tip_tick`'s own formula. `(0.0,
/// 0.0)` (no lean) when the probe above isn't brighter.
pub fn phototropism_dir(world: &World, x: f32, y: f32) -> (f32, f32) {
    let light_here = world.field_at_bilinear(x, y).light;
    let light_above = world.field_at_bilinear(x, y - 4.0).light;
    if light_above > light_here {
        (0.0, -1.0)
    } else {
        (0.0, 0.0)
    }
}

/// Wind lean: direction-only, magnitude-clamped -- ported unchanged from
/// `tree_tip_tick`'s own formula (and its own fix, already applied there:
/// scaling by raw wind magnitude lets one explosion's shockwave dominate
/// the whole formula for as long as the transient takes to pass, so this
/// only ever contributes a fixed-magnitude direction, never a variable
/// one). `(0.0, 0.0)` below `WIND_SPEED_THRESHOLD`.
pub fn wind_lean_dir(world: &World, x: f32, y: f32) -> (f32, f32) {
    let wind = world.field_at_bilinear(x, y);
    let speed = (wind.vx * wind.vx + wind.vy * wind.vy).sqrt();
    if speed > WIND_SPEED_THRESHOLD {
        (wind.vx / speed, wind.vy / speed)
    } else {
        (0.0, 0.0)
    }
}

/// Direction and magnitude of the moisture gradient at `(x, y)` -- ported
/// unchanged from `plant.rs`'s own `moisture_pull` (the MIZ1 gravitropism-
/// vs-hydrotropism antagonism's own gradient read). `None` when the
/// gradient is flat -- open dry ground with no nearby source, or a spot
/// exactly balanced between two sources -- which `Grow`'s `RootTip`
/// dispatch falls through to plain gravitropism for, same as before.
const MOISTURE_SENSOR_OFFSET: f32 = 4.0;
pub fn moisture_pull(world: &World, x: f32, y: f32) -> Option<((f32, f32), f32)> {
    let gx = world.field_at_bilinear(x + MOISTURE_SENSOR_OFFSET, y).moisture - world.field_at_bilinear(x - MOISTURE_SENSOR_OFFSET, y).moisture;
    let gy = world.field_at_bilinear(x, y + MOISTURE_SENSOR_OFFSET).moisture - world.field_at_bilinear(x, y - MOISTURE_SENSOR_OFFSET).moisture;
    let magnitude = (gx * gx + gy * gy).sqrt();
    if magnitude <= f32::EPSILON {
        return None;
    }
    let len = magnitude;
    Some(((gx / len, gy / len), magnitude))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_species_parses() {
        let reg = SpeciesRegistry::builtin();
        assert_eq!(reg.len(), EMBEDDED.len(), "a species failed to load");
    }

    /// **The authored gut arrives, and is not the serde default.**
    ///
    /// `traits` carries `#[serde(default)]`, so a species that misspells
    /// the field, or a RON tuple form that does not deserialize into a
    /// fixed array, loads *silently* at `[0.0; N]` -- and neutral is a
    /// perfectly plausible-looking gut. That is the disconnected-knob
    /// failure `CLAUDE.md` records twice (the `include_str!` sweep whose
    /// arms came back byte-identical, and the megastudy whose eight logs
    /// were one population): the tell in both was output that could not
    /// distinguish "set" from "defaulted". The beetle is what makes this
    /// test able to fail -- the ant's authored value *is* the default, so
    /// asserting on the ant alone would pass against a field that never
    /// parsed.
    #[test]
    fn the_authored_gut_bias_survives_the_ron_round_trip() {
        let reg = SpeciesRegistry::builtin();
        let beetle = reg.get(reg.id_of("beetle").expect("beetle.ron should define \"beetle\""));
        let beetle = beetle.creature.as_ref().expect("beetle is a creature");
        assert_eq!(beetle.traits[TRAIT_GUT_BIAS], 1.0, "beetle.ron authors a carnivore gut; a 0.0 here means the field did not parse");
        assert_eq!(beetle.trait_variance[TRAIT_GUT_BIAS], 0.15, "beetle.ron authors a mutation width; 0.0 means the field did not parse");

        let ant = reg.get(reg.id_of("ant").expect("ant.ron should define \"ant\""));
        let ant = ant.creature.as_ref().expect("ant is a creature");
        // Back to neutral: the owner's verdict on review card
        // 20260823T104411499Z-963f8d was "An omnivore should be viable",
        // and the economy was widened to make it so rather than the animal
        // narrowed to fit the economy. Note this assertion cannot fail for
        // a *parse* error any more -- 0.0 is the serde default again --
        // which is exactly why the beetle above carries this test.
        assert_eq!(ant.traits[TRAIT_GUT_BIAS], 0.0, "ant.ron authors an omnivore gut -- see that file for the food-scale sweep behind the value");
    }

    #[test]
    fn moss_is_present_with_its_growing_tip_behaviors() {
        let reg = SpeciesRegistry::builtin();
        let moss = reg.get(reg.id_of("moss").expect("moss.ron should define \"moss\""));
        let behaviors = moss.behaviors(CellType::GrowingTip);
        assert_eq!(behaviors.len(), 1, "moss should have exactly one behavior on its one cell type");
        assert!(matches!(behaviors[0], Behavior::Divide { .. }));
    }

    #[test]
    fn builtin_includes_worm_with_a_head_and_a_segment() {
        // P-7: a species that exists only in `assets/species` silently does
        // not exist for `cargo test` or for any run without an assets
        // directory beside the binary, and `plant_worm_seed` would return
        // `None` forever with nothing to point at.
        let reg = SpeciesRegistry::builtin();
        let worm = reg.get(reg.id_of("worm").expect("worm.ron must be in EMBEDDED, not just on disk"));
        assert!(worm.behaviors(CellType::Head).is_empty(), "movement is creature.rs's code, not a composed Behavior");
        assert!(worm.behaviors(CellType::Segment).is_empty());
    }

    #[test]
    fn an_unknown_species_name_is_none_not_a_panic() {
        let reg = SpeciesRegistry::builtin();
        assert!(reg.id_of("nonexistent-species").is_none());
    }

    #[test]
    fn cell_type_round_trips_through_aux() {
        // Every variant, and it must stay every variant: this list was
        // silently missing `DormantBud` from the day that type was added,
        // so the encoding of a live cell type went unasserted for a whole
        // milestone. `Head`/`Segment` land here with the same care.
        for ty in [
            CellType::Seed,
            CellType::GrowingTip,
            CellType::MatureBody,
            CellType::Leaf,
            CellType::RootTip,
            CellType::DormantBud,
            CellType::Head,
            CellType::Segment,
        ] {
            assert_eq!(cell_type(pack_cell_type(ty)), Some(ty));
        }
    }

    // The three packing tests that used to live here -- a resource
    // fixed-point round trip, a canopy-density round trip, and "a freshly
    // packed aux has zero density" -- are gone with the packing they
    // tested. They are not replaced one-for-one, because the properties
    // they asserted are now either unrepresentable or trivially true: two
    // `f32` struct fields cannot collide in a shared word, and a
    // `Default`-constructed `OrganismCell` is zero by the language's own
    // rules.
    //
    // What *is* worth asserting is the invariant that replaced them, and
    // that one can genuinely regress: a cell-type write goes through the
    // grid and a scalar write goes through the sidecar, and neither may
    // disturb the other. That was the actual bug the old layout produced
    // (`pack_aux_preserving_density`'s whole reason to exist), so the test
    // below is aimed at the *replacement* mechanism failing the same way,
    // per `CLAUDE.md`'s rule that a guard test must be able to fail for the
    // replacement artifact.

    #[test]
    fn a_cell_type_write_does_not_disturb_the_sidecar_scalars() {
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, 31, 31));
        let wood = w.materials.id_of("wood").expect("wood is compiled in");
        let species = w.species.id_of("tree").expect("tree is compiled in");
        let organism_id = w.push_organism(species).expect("an organism slot is free");
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_cell_type(CellType::GrowingTip)));

        let slot = w.organism_cell_mut(5, 5).expect("set should have registered the cell");
        slot.carbon = 2.5;
        slot.canopy_density = 1.25;

        // The retirement `organism_tick` performs: same cell, new type.
        let cell = w.get(5, 5);
        w.set(5, 5, cell.with_aux(pack_cell_type(CellType::MatureBody)));

        assert_eq!(cell_type(w.get(5, 5).aux()), Some(CellType::MatureBody), "the type write should have taken effect");
        assert_eq!(w.carbon_at(5, 5), 2.5, "a cell-type write must not disturb carbon -- this is what pack_aux_preserving_density existed to patch");
        assert_eq!(w.canopy_density_at(5, 5), 1.25, "a cell-type write must not disturb canopy density");
    }

    #[test]
    fn a_cell_changing_organism_gets_a_fresh_zeroed_sidecar_entry() {
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, 31, 31));
        let wood = w.materials.id_of("wood").expect("wood is compiled in");
        let species = w.species.id_of("tree").expect("tree is compiled in");
        let first = w.push_organism(species).expect("an organism slot is free");
        let second = w.push_organism(species).expect("an organism slot is free");
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(first).with_aux(pack_cell_type(CellType::GrowingTip)));
        w.organism_cell_mut(5, 5).expect("registered").carbon = 3.0;

        w.set(5, 5, Cell::new(wood, 0).with_organism_id(second).with_aux(pack_cell_type(CellType::GrowingTip)));
        assert_eq!(w.carbon_at(5, 5), 0.0, "a cell that changed hands must not inherit the previous organism's carbon");
        assert!(w.organism(first).expect("still live").cells.is_empty(), "the previous owner should have dropped the cell from its list");
    }

    #[test]
    fn an_unrecognized_type_bit_pattern_is_none() {
        // 0-7 are Seed/GrowingTip/MatureBody/Leaf/RootTip/DormantBud/Head/
        // Segment, so the first unassigned pattern is 8. Deliberately the
        // *next* one rather than a far-away value: what this guards is that
        // adding a variant does not silently start aliasing a stale bit
        // pattern onto it, and the pattern that has just become valid is
        // the one that proves the boundary moved with the enum.
        assert_eq!(cell_type(7), Some(CellType::Segment));
        assert_eq!(cell_type(8), None);
        assert_eq!(cell_type(15), None);
    }

    // --- the generational allocator, both ends ------------------------------
    //
    // `encode_organism_id`/`decode_organism_id` are private to `world.rs`, so
    // everything here asserts through the public surface: what a caller can
    // observe is what has to be right.

    #[test]
    fn freeing_an_organism_recycles_its_slot() {
        use crate::sim::chunk::Rect;
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let species = w.species.id_of("moss").expect("moss is compiled in");

        let first = w.push_organism(species).expect("an organism slot is free");
        assert!(w.organism(first).is_some());
        w.free_organism(first);
        assert!(w.organism(first).is_none(), "a freed id must stop resolving");

        let second = w.push_organism(species).expect("an organism slot is free");
        assert!(w.organism(second).is_some());
        assert_ne!(first, second, "the reused slot must hand back a *different* encoded id -- same index, bumped generation");
        // The slot index is the low 12 bits; the reuse must be a genuine
        // reuse of that index rather than growth, or the free list is not
        // doing its job and the 4,095-slot ceiling is still reachable.
        assert_eq!(first & 0x0FFF, second & 0x0FFF, "the second push should have reused the freed slot index, not grown the vec");
        assert!(w.organism(first).is_none(), "the old id must still read stale after the slot was recycled");
    }

    #[test]
    fn freeing_twice_is_a_no_op() {
        // The double-free guard. Without `state.is_none()`, the second free
        // pushes the same index onto the free list again and the next two
        // pushes both get the same slot -- two live organisms sharing one
        // `OrganismState`, which is the worst failure this allocator has.
        use crate::sim::chunk::Rect;
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let species = w.species.id_of("moss").expect("moss is compiled in");

        let first = w.push_organism(species).expect("an organism slot is free");
        w.free_organism(first);
        w.free_organism(first);

        let a = w.push_organism(species).expect("an organism slot is free");
        let b = w.push_organism(species).expect("an organism slot is free");
        assert_ne!(a, b, "a double free must not hand the same slot to two live organisms");
        assert_ne!(a & 0x0FFF, b & 0x0FFF, "a double free must not put one slot index on the free list twice");
        assert!(w.organism(a).is_some() && w.organism(b).is_some());
    }

    #[test]
    fn freeing_a_stale_or_zero_id_is_silently_ignored() {
        use crate::sim::chunk::Rect;
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let species = w.species.id_of("moss").expect("moss is compiled in");

        w.free_organism(0); // "no organism" -- must not touch slot 0's storage
        w.free_organism(1234); // never allocated

        let live = w.push_organism(species).expect("an organism slot is free");
        w.free_organism(live);
        let reused = w.push_organism(species).expect("an organism slot is free");
        // The stale handle for the previous generation must not be able to
        // free the organism that now holds the slot.
        w.free_organism(live);
        assert!(w.organism(reused).is_some(), "a stale free must not release the live organism that inherited the slot");
    }

    #[test]
    fn a_stale_site_on_a_reused_slot_drops_silently() {
        // The whole point of the generational scheme
        // (`Reports/organism-substrate-design.md` §6): a scheduled site
        // outliving its organism must resolve to nothing and disappear, not
        // panic and not act on whoever inherited the slot.
        use crate::sim::chunk::Rect;
        use crate::sim::scheduler::{self, ActiveKind, ActiveSite};

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let moss_material = w.materials.id_of("moss").expect("moss is compiled in");
        let species = w.species.id_of("moss").expect("moss is compiled in");

        let doomed = w.push_organism(species).expect("an organism slot is free");
        w.set(5, 5, Cell::new(moss_material, 0).with_organism_id(doomed).with_aux(pack_cell_type(CellType::GrowingTip)));
        w.free_organism(doomed);

        let heir = w.push_organism(species).expect("an organism slot is free");
        assert_eq!(doomed & 0x0FFF, heir & 0x0FFF, "the test needs the heir to actually inherit the slot");

        w.schedule_active_site(ActiveSite { x: 5, y: 5, kind: ActiveKind::Organism { organism: doomed, stale_ticks: 0, plastochron: 0 }, next_frame: 1 });
        for _ in 0..4 {
            w.begin_step();
            scheduler::step(&mut w); // must not panic
            w.end_step();
        }

        assert_eq!(w.active_site_count(), 0, "a site whose organism is gone should drop itself, not reschedule forever");
        assert!(w.organism(heir).expect("the heir is live").cells.is_empty(), "a stale site must not have grown cells for the organism that inherited its slot");
    }

    #[test]
    fn generation_wrap_is_counted() {
        // P-8. The 4-bit generation wraps after 16 reuses, at which point a
        // reference stale by exactly that many reuses aliases a live
        // organism again. Accepted, but it should be a *counted* quantity
        // rather than a footnote -- 17 allocate/free cycles is one wrap.
        use crate::sim::chunk::Rect;
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let species = w.species.id_of("moss").expect("moss is compiled in");

        assert_eq!(w.organism_generation_wraps, 0);
        let first = w.push_organism(species).expect("an organism slot is free"); // generation 0
        w.free_organism(first);
        // Generations 1..=15: fifteen reuses, none of them a wrap.
        for _ in 0..15 {
            let id = w.push_organism(species).expect("an organism slot is free");
            assert_ne!(id, first, "generations 1..15 must all encode differently from generation 0");
            w.free_organism(id);
        }
        assert_eq!(w.organism_generation_wraps, 0, "fifteen reuses stay inside the 4-bit space");

        // The sixteenth reuse is the wrap -- and the aliasing it warns
        // about is real, which is exactly why it is worth counting: the
        // very first id reads live again.
        let wrapped = w.push_organism(species).expect("an organism slot is free");
        assert_eq!(w.organism_generation_wraps, 1, "the sixteenth reuse of one slot should have wrapped its generation exactly once");
        assert_eq!(first, wrapped, "after sixteen reuses the encoded id repeats -- the accepted limitation, asserted so it stays known");
    }

    // --- diffuse_resource --------------------------------------------------

    #[test]
    fn resource_diffuses_from_a_full_cell_toward_an_empty_same_organism_neighbour() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_id = w.push_organism(SpeciesId(0)).expect("an organism slot is free");
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_cell_type(CellType::MatureBody)));
        w.set(6, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_cell_type(CellType::MatureBody)));
        w.organism_cell_mut(5, 5).expect("registered").carbon = RESOURCE_SCALE;

        transport(&mut w, organism_id);

        let (left, right) = (w.carbon_at(5, 5), w.carbon_at(6, 5));
        assert!(right > 0.0, "the empty neighbour should have gained resource: got {right}");
        assert!(left < RESOURCE_SCALE, "the full cell should have lost some resource: got {left}");
        // Pairwise exchange is exactly conserving, unlike the per-cell
        // move-toward-the-mean rule this replaced -- worth asserting
        // directly, since it is one of the four properties
        // `plant-substrate-v2-design.md` §7c claims for the form.
        assert!((left + right - RESOURCE_SCALE).abs() < 1e-3, "transport should conserve: {left} + {right} != {RESOURCE_SCALE}");
    }

    #[test]
    fn resource_does_not_cross_an_organism_boundary() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_a = w.push_organism(SpeciesId(0)).expect("an organism slot is free");
        let organism_b = w.push_organism(SpeciesId(0)).expect("an organism slot is free");
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_a).with_aux(pack_cell_type(CellType::MatureBody)));
        w.set(6, 5, Cell::new(wood, 0).with_organism_id(organism_b).with_aux(pack_cell_type(CellType::MatureBody)));
        w.organism_cell_mut(5, 5).expect("registered").carbon = RESOURCE_SCALE;

        // **Both organisms stepped, deliberately.** `plant-substrate-v2-
        // design.md` §3f warns this test can go vacuous once transport is
        // per-organism: stepping only `organism_a` would leave `b`'s cell
        // untouched because it was never in the iteration, which passes for
        // a reason that has nothing to do with the wall test. Running both
        // means the only thing keeping resource on the left is `transport`'s
        // `is_wall` check on a differing `organism_id`, which is the claim.
        transport(&mut w, organism_a);
        transport(&mut w, organism_b);

        let (left, right) = (w.carbon_at(5, 5), w.carbon_at(6, 5));
        assert_eq!(left, RESOURCE_SCALE, "a different organism's cell must be a wall, not a diffusion partner");
        assert_eq!(right, 0.0, "resource must not cross an organism boundary");
    }

    #[test]
    fn transport_no_longer_decays_density_itself() {
        // Decay moved to `plant::organism_tick`'s own per-cell cadence
        // (see that module's `CANOPY_DENSITY_DECAY_PER_TICK` doc for why:
        // a decay applied on this function's per-CA-frame cadence erased a
        // fresh deposit within a handful of frames, long before a
        // neighbour's much-less-frequent `Grow` check ever read it). This
        // function's own job is spatial spread only -- an isolated cell
        // (no transport partner) should see its density completely
        // unchanged after any number of ticks.
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_id = w.push_organism(SpeciesId(0)).expect("an organism slot is free");
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_cell_type(CellType::MatureBody)));
        {
            let slot = w.organism_cell_mut(5, 5).expect("registered");
            slot.carbon = 1.0;
            slot.canopy_density = CANOPY_DENSITY_SCALE;
        }

        for _ in 0..500 {
            transport(&mut w, organism_id);
        }

        assert_eq!(
            w.canopy_density_at(5, 5),
            CANOPY_DENSITY_SCALE,
            "an isolated cell's density should not decay from repeated transport calls alone"
        );
    }

    // --- Polarity (Decision 6, design doc §7k) -------------------------

    /// Build a bare organism from a list of positions, all `wood`
    /// `MatureBody`, and hand back its id.
    fn polarity_organism(w: &mut World, cells: &[(i32, i32)]) -> u16 {
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let species = w.species.id_of("tree").expect("tree is a compiled-in species");
        let organism_id = w.push_organism(species).expect("an organism slot is free");
        for &(x, y) in cells {
            w.set(x, y, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_cell_type(CellType::MatureBody)));
        }
        organism_id
    }

    fn conductance_at(w: &World, x: i32, y: i32) -> [f32; 4] {
        w.organism_cell(x, y).expect("registered").carbon_conductance
    }

    /// The minimal observable proof that canalization happens at all: a
    /// straight chain with a source at one end and a drained sink at the
    /// other develops conductance along its axis far above the basal floor,
    /// while the faces pointing *across* the chain — which carry no flux,
    /// because there is nothing on the other side of them — stay at it.
    #[test]
    fn a_chain_canalizes_along_its_axis_and_not_across_it() {
        use super::super::chunk::Rect;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let cells: Vec<(i32, i32)> = (5..13).map(|x| (x, 10)).collect();
        let organism_id = polarity_organism(&mut w, &cells);

        for _ in 0..40 {
            // Held source at one end, drained sink at the other — a leaf
            // and a growing tip, without needing either to exist yet.
            w.organism_cell_mut(5, 10).expect("registered").carbon = 1.0;
            w.organism_cell_mut(12, 10).expect("registered").carbon = 0.0;
            transport(&mut w, organism_id);
        }

        let mid = conductance_at(&w, 8, 10);
        // NEIGHBOURS_4 is [-x, +x, -y, +y]: index 1 is downstream along the
        // chain, indices 2 and 3 point at empty space.
        // Expressed as a fraction of the *available* range rather than a
        // fixed multiple of the floor: a hardcoded `> 5x basal` silently
        // becomes unsatisfiable the moment the canalization contrast is
        // tuned below 5:1, which is a test failing for the tuning rather
        // than for the mechanism. Half the span says "this face clearly
        // canalized" at any contrast.
        let midpoint = CONDUCTANCE_MIN + (CONDUCTANCE_MAX - CONDUCTANCE_MIN) * 0.5;
        assert!(
            mid[1] > midpoint,
            "the downstream face of a chain carrying real flux should canalize past the midpoint              of the available range ({midpoint}), got {}",
            mid[1]
        );
        assert!(
            (mid[2] - CONDUCTANCE_MIN).abs() < 1e-3 && (mid[3] - CONDUCTANCE_MIN).abs() < 1e-3,
            "faces with no neighbour carry no flux and must stay basal, got {:?}",
            &mid[2..4]
        );
        assert!(
            mid[1] <= CONDUCTANCE_MAX + 1e-3,
            "conductance must not exceed its fixed-point ceiling, got {} > {CONDUCTANCE_MAX}",
            mid[1]
        );
    }

    /// **The real gate on this step** -- §7h's Y-junction, and the one
    /// that fails if `Φ`, `J_REF` or the contrast ratio are mis-set.
    ///
    /// One stem cell feeds two tips, symmetric in every respect. Both draw
    /// and spend on growth; at one tick `A` grows and `B` does not (§7h's
    /// single asymmetry: `B`'s candidate set came back empty, a routine
    /// outcome in crowded canopy). `A` therefore *spends* and ends up the
    /// **less hungry** of the two.
    ///
    /// Under the isotropic rule the split is decided by hunger alone, so
    /// `B` takes the larger share next tick and the lead flips -- which is
    /// exactly what `tree-rewrite-design.md` §3 published as the honest
    /// description of shipped behaviour, and why nothing accumulates.
    /// Under Decision 6 the stored conductance beats transient hunger and
    /// the lead holds. A flipped lead is the isotropic rule's signature.
    ///
    /// **Supply-limited, not source-pinned, and that is the whole reason
    /// this test is shaped the way it is.** An earlier version held the
    /// stem at a fixed 1.0 every tick, i.e. an infinite reservoir. Flux
    /// then grows without bound as conductance ratchets, `Φ` saturates at
    /// 1 for *both* faces, and the response stops discriminating: the
    /// conductance ratio collapsed to 1.04 with both faces pinned near the
    /// ceiling (24.5 vs 23.5), and the gate failed for a reason that had
    /// nothing to do with the mechanism. §7h delivers a fixed `Q` per tick
    /// precisely so the operating point stays near `J_REF`, where the Hill
    /// function is still convex and a flux advantage compounds.
    #[test]
    fn the_better_connected_tip_outdraws_the_hungrier_one() {
        use super::super::chunk::Rect;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let stem = (10, 11);
        let (tip_a, tip_b) = ((9, 10), (11, 10));
        let organism_id = polarity_organism(&mut w, &[stem, (10, 10), tip_a, tip_b]);

        // §7h's constants: `Q` per tick into the stem, `cost` per growth.
        const Q: f32 = 0.3;
        const COST: f32 = 0.2;

        let spend = |w: &mut World, at: (i32, i32)| -> bool {
            let c = w.carbon_at(at.0, at.1);
            if c >= COST {
                w.organism_cell_mut(at.0, at.1).expect("registered").carbon = c - COST;
                true
            } else {
                false
            }
        };

        // **The one asymmetry, and it is not invented.** §7h: at one tick
        // both tips can afford to grow, `A` grows and `B` does not --
        // because `B`'s candidate set came back empty (`plant.rs`: "if
        // candidates.is_empty() { continue; }"), a routine outcome in
        // crowded canopy. `A` spends and `B` does not. Nothing else ever
        // differs between them; the whole question is what the system does
        // with one lost growth step.
        // Fired the *first time `B` could actually have grown*, rather than
        // at a hardcoded tick: early ticks have not accumulated `cost` yet,
        // so "skip B at tick 1" is a no-op and the two tips stay bit-identical
        // (found the hard way -- the faces came out equal to seven decimals).
        let mut b_has_stalled = false;
        for _ in 0..60 {
            w.organism_cell_mut(stem.0, stem.1).expect("registered").carbon += Q;
            transport(&mut w, organism_id);
            spend(&mut w, tip_a);
            if !b_has_stalled && w.carbon_at(tip_b.0, tip_b.1) >= COST {
                b_has_stalled = true;
            } else {
                spend(&mut w, tip_b);
            }
        }
        assert!(b_has_stalled, "the setup requires B to have missed exactly one growth step");

        let c_a = conductance_at(&w, 10, 10)[0]; // junction's -x face, toward A
        let c_b = conductance_at(&w, 10, 10)[1]; // ... and +x face, toward B
        assert!(
            c_a > c_b * 1.05,
            "A's face should have canalized measurably ahead of B's, got {c_a} vs {c_b} (ratio {}).              Both sitting near {CONDUCTANCE_MAX} means flux has saturated Φ and the response can no              longer discriminate -- that is a supply/J_REF mismatch, not a broken rule",
            c_a / c_b
        );

        // **What is asserted, and why it is not §7h's tick-6 arithmetic
        // verbatim.** That table pins the stem at `R_S = 1.0` when
        // computing hunger *and* limits delivery to `Q = 0.3` per tick.
        // Those are two different constraints, and a running simulation
        // cannot honour both: a genuinely supply-limited stem sits far
        // below 1.0, so one `cost`-sized spend is a large *relative* change
        // in hunger rather than §7h's 0.82-vs-0.98. Reproducing the table's
        // numbers would mean pinning the stem, which drives flux to ~19x
        // `J_REF`, saturates `Φ` on both faces, and collapses the
        // conductance ratio to 1.0 -- measured, and it is how the first
        // draft of this test failed.
        //
        // So the *claim* is tested rather than the illustration: the split
        // is no longer decided by hunger alone. Equal hunger, unequal
        // conductance -> the better-connected tip must draw more. That is
        // precisely what the isotropic rule cannot do, and it is the
        // property every downstream consequence in §7h rests on.
        let equal = 0.05f32;
        w.organism_cell_mut(tip_a.0, tip_a.1).expect("registered").carbon = equal;
        w.organism_cell_mut(tip_b.0, tip_b.1).expect("registered").carbon = equal;
        w.organism_cell_mut(stem.0, stem.1).expect("registered").carbon += Q;
        transport(&mut w, organism_id);
        let (gained_a, gained_b) = (w.carbon_at(tip_a.0, tip_a.1) - equal, w.carbon_at(tip_b.0, tip_b.1) - equal);
        assert!(
            gained_a > gained_b,
            "at equal hunger the better-connected tip must draw more -- A gained {gained_a},              B gained {gained_b} at conductance {c_a} vs {c_b}. Equal gains mean the conductance              is not reaching the transport rule at all"
        );

        // And the quantitative form: the advantage should track the
        // conductance ratio, not merely have the right sign.
        let share_ratio = gained_a / gained_b;
        let conductance_ratio = c_a / c_b;
        assert!(
            share_ratio > 1.0 + (conductance_ratio - 1.0) * 0.5,
            "the supply advantage ({share_ratio}) should track the conductance advantage              ({conductance_ratio}), not merely have the right sign"
        );

        // **The hysteresis claim.** A hunger disadvantage *smaller than the
        // conductance advantage* must not flip the lead -- that is what
        // "hard to reverse once established" means, and under the isotropic
        // rule any hunger disadvantage at all flips it immediately.
        let handicap = 1.0 + (conductance_ratio - 1.0) * 0.5;
        w.organism_cell_mut(tip_a.0, tip_a.1).expect("registered").carbon = equal * handicap;
        w.organism_cell_mut(tip_b.0, tip_b.1).expect("registered").carbon = equal;
        w.organism_cell_mut(stem.0, stem.1).expect("registered").carbon += Q;
        let before = (equal * handicap, equal);
        transport(&mut w, organism_id);
        let gained_a = w.carbon_at(tip_a.0, tip_a.1) - before.0;
        let gained_b = w.carbon_at(tip_b.0, tip_b.1) - before.1;
        assert!(
            gained_a > gained_b,
            "the less hungry but better connected tip must still take the larger share --              A gained {gained_a}, B gained {gained_b} (conductance {c_a} vs {c_b},              handicap {handicap}). A flipped lead here is the isotropic rule's signature"
        );
    }

    /// Carbon must respect `RESOURCE_SCALE` even when nothing is
    /// photosynthesizing — transport alone must not push a cell past it.
    ///
    /// This held for free under the old symmetric rule, which moves a cell
    /// toward its neighbours' *mean* and so can never exceed the largest
    /// value present; a clamp at the two sources was enough. The pairwise
    /// carrier rule has no such property. Its equilibrium across a face is
    /// `c_ij·R_i = c_ji·R_j`, so a cell whose *inbound* conductance is high
    /// and whose *outbound* is low settles at `R_j = R_i · c_ij/c_ji` —
    /// which for a fully canalized face feeding an unpolarized one is the
    /// canalization contrast times the source. Measured on a 24-tree
    /// ensemble before the fix: a single cell reached **92.0 against a
    /// scale of 4.0**, twenty-three times its stated maximum and enough to
    /// fund 460 growth steps at `tree.ron`'s `cost`.
    ///
    /// **The asymmetry is imposed rather than grown**, and re-imposed each
    /// tick. A symmetric setup cannot show this — equal conductances make
    /// the equilibrium `R_j = R_i`, so the cell converges neatly to the cap
    /// and the test passes with or without the clamp. That is exactly what
    /// the first version of this test did, and it was worthless.
    #[test]
    fn transport_never_pushes_a_cell_past_the_resource_scale() {
        use super::super::chunk::Rect;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let organism_id = polarity_organism(&mut w, &[(10, 10), (11, 10)]);

        for _ in 0..400 {
            // A source held at the cap, as `Photosynthesize` leaves it.
            w.organism_cell_mut(10, 10).expect("registered").carbon = RESOURCE_SCALE;
            // A one-way valve: the left cell exports hard (+x face at the
            // ceiling), the right cell barely exports back (-x face at the
            // floor). This is what a canalized strand feeding inert
            // parenchyma looks like.
            w.organism_cell_mut(10, 10).expect("registered").carbon_conductance[1] = CONDUCTANCE_MAX;
            w.organism_cell_mut(11, 10).expect("registered").carbon_conductance[0] = CONDUCTANCE_MIN;
            transport(&mut w, organism_id);
        }

        let sink = w.carbon_at(11, 10);
        assert!(
            sink <= RESOURCE_SCALE + 1e-3,
            "transport must not fill a cell past RESOURCE_SCALE ({RESOURCE_SCALE}), got {sink} --              the pairwise rule's equilibrium is R_j = R_i * c_ij/c_ji, so an unclamped sink settles              at the canalization contrast times its source"
        );
    }

    /// §7a(2) is explicit that the unbounded quadratic form diverges. Drive
    /// one face at saturating flux for far longer than any tree lives and
    /// assert the bounded form was actually used.
    #[test]
    fn conductance_is_bounded_however_long_it_is_driven() {
        use super::super::chunk::Rect;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let cells: Vec<(i32, i32)> = (5..9).map(|x| (x, 10)).collect();
        let organism_id = polarity_organism(&mut w, &cells);

        for _ in 0..2000 {
            w.organism_cell_mut(5, 10).expect("registered").carbon = RESOURCE_SCALE;
            w.organism_cell_mut(8, 10).expect("registered").carbon = 0.0;
            transport(&mut w, organism_id);
        }

        for &(x, y) in &cells {
            for (k, c) in conductance_at(&w, x, y).iter().enumerate() {
                assert!(c.is_finite(), "conductance went non-finite at ({x},{y}) face {k}");
                assert!(*c <= CONDUCTANCE_MAX + 1e-3, "conductance {c} at ({x},{y}) face {k} exceeded the ceiling {CONDUCTANCE_MAX}");
            }
        }
    }

    /// The A/B switch of §7c, made a test rather than a claim: with no
    /// flux-driven gain, every face sits at `CONDUCTANCE_MIN` forever and
    /// the pairwise rule is identically Fickian. This is what makes a
    /// regression in the polar machinery bisectable — if this fails, the
    /// reduction to the isotropic case is broken and that is the bug.
    #[test]
    fn with_no_gain_every_face_stays_isotropic_and_transport_is_symmetric() {
        use super::super::chunk::Rect;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let organism_id = polarity_organism(&mut w, &[(5, 10), (6, 10)]);
        w.organism_cell_mut(5, 10).expect("registered").carbon = 1.0;

        // `VEIN_GAIN` is a compile-time constant, so rather than rebuild
        // with it zeroed, assert the property that makes it meaningful:
        // with *no established polarity* the exchange is symmetric, so two
        // cells with equal conductance move toward each other's mean and
        // conserve exactly -- Fick, which is what `VEIN_GAIN = 0` pins the
        // system to permanently.
        let start: f32 = w.carbon_at(5, 10) + w.carbon_at(6, 10);
        let a = conductance_at(&w, 5, 10);
        let b = conductance_at(&w, 6, 10);
        assert_eq!(a, [CONDUCTANCE_MIN; 4], "a fresh cell must start perfectly isotropic, not at zero");
        assert_eq!(b, [CONDUCTANCE_MIN; 4]);

        transport(&mut w, organism_id);
        let total: f32 = w.carbon_at(5, 10) + w.carbon_at(6, 10);
        assert!((total - start).abs() < 1e-4, "the pairwise rule must conserve exactly: {start} -> {total}");
        assert!(w.carbon_at(5, 10) > w.carbon_at(6, 10), "carbon should still flow down the gradient");
        assert!(w.carbon_at(6, 10) > 0.0, "the empty cell should have gained");
    }

    /// `Grow`'s fallback (§7g) is reached exactly when it should be: an
    /// organism whose conductance is still uniformly basal carries no
    /// direction information, and must say so rather than returning a
    /// confident direction built from float noise.
    #[test]
    fn supply_direction_is_none_until_something_has_actually_been_carried() {
        use super::super::chunk::Rect;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let organism_id = polarity_organism(&mut w, &[(5, 10), (6, 10), (7, 10)]);
        assert_eq!(supply_direction(&w, 6, 10), None, "a brand-new organism has no established supply direction");

        for _ in 0..30 {
            w.organism_cell_mut(5, 10).expect("registered").carbon = 1.0;
            w.organism_cell_mut(7, 10).expect("registered").carbon = 0.0;
            transport(&mut w, organism_id);
        }
        let dir = supply_direction(&w, 6, 10).expect("a canalized chain should have a supply direction");
        assert!(dir.0 < -0.5, "supply should read as arriving from -x, got {dir:?}");
    }

    #[test]
    fn reachable_from_anchors_stays_within_matching_cells() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        // A short connected line of wood, with an unrelated wood cell
        // nearby that is NOT connected (a gap of empty space between).
        for x in 10..15 {
            w.set(x, 10, Cell::new(wood, 0));
        }
        w.set(20, 10, Cell::new(wood, 0));

        let matches = |cell: Cell| cell.material == wood;
        let reached = reachable_from_anchors(&w, [(10, 10)], matches, 100);

        for x in 10..15 {
            assert!(reached.contains(&(x, 10)), "({x}, 10) should be reachable from the anchor");
        }
        assert!(!reached.contains(&(20, 10)), "the disconnected wood cell should not be reachable");
    }

    /// **Bark is a readout of the density allele, and a readout that
    /// does not move is not one.**
    ///
    /// A pure-function test on purpose: the alternative — growing a stand
    /// and looking at it — cannot tell a band that never changed from a
    /// recolour too small to see at contact-sheet zoom, which is exactly
    /// the trap `CLAUDE.md` records against magnitude-scaled overlays.
    /// The sheet judgement is the *aesthetic* question; this is the
    /// mechanical one, and it belongs in a test.
    ///
    /// The mapping is proportional over the allele range, so with today's
    /// three alleles and two declared bands it reads
    /// `[first, first, first + 1]`: pioneer and as-authored share the low
    /// band, dense stands out. `count: 0` (moss, and any species that
    /// predates bands) keeps the pre-band 0, exactly as the free bark
    /// draw this replaced did.
    #[test]
    fn bark_band_tracks_the_density_allele() {
        let bands = PaletteBands { first: 0, count: 2 };
        let mapped: Vec<u8> = (0..3).map(|a| bark_band_for_density(bands, a)).collect();
        assert_eq!(mapped, vec![0, 0, 1], "three density alleles over two bands should read [low, low, high]");

        // Offset ranges are the common case -- every species but `tree`
        // declares one -- and the band is inside the species' own range,
        // never a raw allele index.
        let shrub_like = PaletteBands { first: 2, count: 2 };
        assert_eq!(
            (0..3).map(|a| bark_band_for_density(shrub_like, a)).collect::<Vec<u8>>(),
            vec![2, 2, 3],
            "the derived band must sit inside the species' declared range"
        );

        assert_eq!(bark_band_for_density(PaletteBands { first: 0, count: 0 }, 2), 0, "an unset range must stay at the pre-band 0");
        assert_eq!(bark_band_for_density(PaletteBands { first: 3, count: 0 }, 1), 0, "`count: 0` means unset -- `first` is not a band then");

        // An allele past the table (a widened `LOCUS_ALLELES` read by
        // stale state) clamps rather than walking off the palette.
        assert_eq!(bark_band_for_density(bands, 200), 1, "an out-of-range allele must clamp to the top band");
    }

    #[test]
    fn reachable_from_anchors_respects_the_cap() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        for x in 0..40 {
            w.set(x, 10, Cell::new(wood, 0));
        }
        let matches = |cell: Cell| cell.material == wood;
        let reached = reachable_from_anchors(&w, [(0, 10)], matches, 5);
        assert_eq!(reached.len(), 5, "the search should stop exactly at the cap");
    }
}
