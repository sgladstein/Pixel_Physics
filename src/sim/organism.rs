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
#[derive(Clone, Debug, PartialEq)]
pub struct OrganismCell {
    /// Photosynthate. Was the 8-bit `aux` field; `Absorb`/`Photosynthesize`
    /// still clamp it to `RESOURCE_SCALE`.
    pub carbon: f32,
    /// `Reports/tree-rewrite-design.md` §2b's crowding signal, clamped to
    /// `CANOPY_DENSITY_SCALE`.
    pub canopy_density: f32,
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
        Self { carbon: 0.0, canopy_density: 0.0, carbon_conductance: [CONDUCTANCE_MIN; 4] }
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
/// Flux-driven conductance gain. **Set this to 0 and the whole mechanism
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
const VEIN_GAIN: f32 = 2.9;

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
/// CONDUCTANCE_MAX <= 0.25`. At contrast 30 that caps it at 0.00833, and
/// 0.008 sits just under.
const TRANSPORT_RATE: f32 = 0.008;

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
                let net = TRANSPORT_RATE * (out - back);
                delta[i] -= net;
                delta[j] += net;
                flux[i][k] += net;
                flux[j][opposite_face(k)] -= net;
            }
        }
        for i in 0..cells.len() {
            carbon[i] = (carbon[i] + delta[i]).max(0.0);
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

    // --- Polarity (Decision 6, design doc §7k) -------------------------

    /// Build a bare organism from a list of positions, all `wood`
    /// `MatureBody`, and hand back its id.
    fn polarity_organism(w: &mut World, cells: &[(i32, i32)]) -> u16 {
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let species = w.species.id_of("tree").expect("tree is a compiled-in species");
        let organism_id = w.push_organism(species);
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
        assert!(
            mid[1] > CONDUCTANCE_MIN * 5.0,
            "the downstream face of a chain carrying real flux should canalize well above the basal floor, got {}",
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
