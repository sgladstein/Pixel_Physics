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
    /// (not `GrowingTip` reused) because `structural.rs`'s organism branch
    /// anchors its reachability search specifically on `RootTip` cells
    /// (`Reports/organism-substrate-design.md` §2/§5) — anchoring on every
    /// `GrowingTip` instead would anchor a tree on its own canopy, which
    /// is not what "supported by roots" means.
    RootTip = 4,
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
        branch_chance: f32,
        /// Weight on continuing this tip's own established direction
        /// (`Reports/tree-rewrite-design.md` §2a: the vector average of
        /// every same-organism 8-neighbour's direction *from* this cell,
        /// negated — "grow away from where you came from" — not a stored
        /// float, a fresh local read every tick).
        continuation_weight: f32,
        /// Weight on phototropism (`organism::phototropism_dir`, ported
        /// from `plant.rs`'s `tree_tip_tick` unchanged). `0.0` for a
        /// species/cell type with no reason to chase light (roots).
        light_weight: f32,
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
        upward_weight: f32,
        /// Weight subtracting `canopy_density` (`with_canopy_density`/
        /// `canopy_density` below) at each candidate — `Reports/tree-
        /// rewrite-design.md` §2b's self-avoidance term, the deposit-
        /// diffuse-decay-follow replacement for the old space-colonization
        /// algorithm's private attractor point cloud. `0.0` disables
        /// self-avoidance entirely (a root system has no citation in this
        /// engine's own research for root-root avoidance, so `tree.ron`'s
        /// `RootTip` sets this to `0.0` rather than inventing one).
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
        plastochron: u8,
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
    },
    /// Reads the light field, credits the resource scalar — a `Leaf`
    /// cell's own contribution to the resource economy `Grow`/`Divide`
    /// spend from. `rate` is per-tick credit at full light; scaled by the
    /// actual local reading the same way every other field-driven rate in
    /// this codebase is.
    Photosynthesize { rate: f32 },
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
    SecondaryThicken { pipe_ratio: f32 },
    /// A `Seed` cell's transition to `GrowingTip`/`RootTip`, checked on a
    /// schedule against local field readings. `instant: true` is a
    /// test-only escape hatch that fires unconditionally next tick,
    /// avoiding germination-condition waits in every test that just needs
    /// a grown organism to exist — `organism-substrate-design.md` §1's own
    /// stated reason for this field.
    Germinate { light_threshold: f32, moisture_threshold: f32, instant: bool },
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
#[derive(Deserialize)]
pub struct SpeciesDef {
    pub name: String,
    pub cell_types: Vec<(CellType, Vec<Behavior>)>,
}

pub struct Species {
    pub name: String,
    cell_types: Vec<(CellType, Vec<Behavior>)>,
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
}

impl From<SpeciesDef> for Species {
    fn from(def: SpeciesDef) -> Self {
        Self { name: def.name, cell_types: def.cell_types }
    }
}

/// Index into `SpeciesRegistry`. Distinct type from `MaterialId` even
/// though both are a `u16` newtype over a `Vec` index — a material id in a
/// species slot (or vice versa) should be a type error, not a silent
/// cross-registry mixup.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpeciesId(pub u16);

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
    ///    tree; half the budget, on a 512x320 test world that `CLAUDE.md`
    ///    says is going to grow.
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
    pub root_cells: u32,
    pub shoot_cells: u32,
}

pub struct SpeciesRegistry {
    species: Vec<Species>,
    by_name: HashMap<String, SpeciesId>,
}

const EMBEDDED: &[&str] = &[include_str!("../../assets/species/moss.ron"), include_str!("../../assets/species/tree.ron")];

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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrganismCell {
    /// Photosynthate. Was the 8-bit `aux` field; `Absorb`/`Photosynthesize`
    /// still clamp it to `RESOURCE_SCALE`.
    pub carbon: f32,
    /// `Reports/tree-rewrite-design.md` §2b's crowding signal, clamped to
    /// `CANOPY_DENSITY_SCALE`.
    pub canopy_density: f32,
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
/// first-class tuning target for the economy pass, and polarity will
/// re-derive it against its own stability bound.
pub const TRANSPORT_SUBSTEPS: u32 = 45;

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

    let state = world.organism(organism_id).expect("checked above");
    let mut carbon: Vec<f32> = cells.iter().map(|p| state.cells[p].carbon).collect();
    let mut density: Vec<f32> = cells.iter().map(|p| state.cells[p].canopy_density).collect();

    // Jacobi, not Gauss-Seidel: each substep reads the previous substep's
    // values for every cell and writes a fresh buffer. The rule this
    // replaces updated in place in sweep order, so a cell saw some
    // neighbours already advanced and some not, and the result depended on
    // which chunk the sweep reached first -- the ordering dependence
    // `plant-substrate-v2-design.md` §7c names as a defect of the old
    // form. Double-buffering removes it, and is required anyway now the
    // iteration order comes from a map rather than the grid.
    let (mut next_carbon, mut next_density) = (carbon.clone(), density.clone());
    for _ in 0..TRANSPORT_SUBSTEPS {
        for i in 0..cells.len() {
            let (mut carbon_sum, mut density_sum, mut n) = (0.0f32, 0.0f32, 0u32);
            for slot in neighbours[i].iter().flatten() {
                carbon_sum += carbon[*slot];
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
            if n == 0 {
                next_carbon[i] = carbon[i];
                next_density[i] = density[i];
                continue;
            }
            let n = n as f32;
            next_carbon[i] = (carbon[i] + (carbon_sum / n - carbon[i]) * DIFFUSION_RATE).max(0.0);
            next_density[i] = (density[i] + (density_sum / n - density[i]) * DIFFUSION_RATE).clamp(0.0, CANOPY_DENSITY_SCALE);
        }
        std::mem::swap(&mut carbon, &mut next_carbon);
        std::mem::swap(&mut density, &mut next_density);
    }

    let Some(state) = world.organism_mut(organism_id) else {
        return;
    };
    for (i, pos) in cells.iter().enumerate() {
        if let Some(slot) = state.cells.get_mut(pos) {
            slot.carbon = carbon[i];
            slot.canopy_density = density[i];
        }
    }
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

    #[test]
    fn moss_is_present_with_its_growing_tip_behaviors() {
        let reg = SpeciesRegistry::builtin();
        let moss = reg.get(reg.id_of("moss").expect("moss.ron should define \"moss\""));
        let behaviors = moss.behaviors(CellType::GrowingTip);
        assert_eq!(behaviors.len(), 1, "moss should have exactly one behavior on its one cell type");
        assert!(matches!(behaviors[0], Behavior::Divide { .. }));
    }

    #[test]
    fn an_unknown_species_name_is_none_not_a_panic() {
        let reg = SpeciesRegistry::builtin();
        assert!(reg.id_of("nonexistent-species").is_none());
    }

    #[test]
    fn cell_type_round_trips_through_aux() {
        for ty in [CellType::Seed, CellType::GrowingTip, CellType::MatureBody, CellType::Leaf, CellType::RootTip] {
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
        let organism_id = w.push_organism(species);
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
        let first = w.push_organism(species);
        let second = w.push_organism(species);
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(first).with_aux(pack_cell_type(CellType::GrowingTip)));
        w.organism_cell_mut(5, 5).expect("registered").carbon = 3.0;

        w.set(5, 5, Cell::new(wood, 0).with_organism_id(second).with_aux(pack_cell_type(CellType::GrowingTip)));
        assert_eq!(w.carbon_at(5, 5), 0.0, "a cell that changed hands must not inherit the previous organism's carbon");
        assert!(w.organism(first).expect("still live").cells.is_empty(), "the previous owner should have dropped the cell from its list");
    }

    #[test]
    fn an_unrecognized_type_bit_pattern_is_none() {
        // Bit pattern 5 doesn't correspond to any current CellType variant
        // (0-4 are Seed/GrowingTip/MatureBody/Leaf/RootTip).
        assert_eq!(cell_type(5), None);
    }

    // --- diffuse_resource --------------------------------------------------

    #[test]
    fn resource_diffuses_from_a_full_cell_toward_an_empty_same_organism_neighbour() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_id = w.push_organism(SpeciesId(0));
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
        let organism_a = w.push_organism(SpeciesId(0));
        let organism_b = w.push_organism(SpeciesId(0));
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
        let organism_id = w.push_organism(SpeciesId(0));
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
