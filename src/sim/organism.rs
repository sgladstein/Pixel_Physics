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
    },
    /// Reads the light field, credits the resource scalar — a `Leaf`
    /// cell's own contribution to the resource economy `Grow`/`Divide`
    /// spend from. `rate` is per-tick credit at full light; scaled by the
    /// actual local reading the same way every other field-driven rate in
    /// this codebase is.
    Photosynthesize { rate: f32 },
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

/// Resource scalar upper bound — 0..`RESOURCE_SCALE` maps onto the `u8`
/// range packed into `aux`. Not derived from any existing convention (see
/// the design report's own correction of an earlier draft that claimed
/// one): chosen as a plain, generous headroom for a species-defined
/// `cost`/`rate` pair to operate within without the packing itself ever
/// being the limiting factor.
pub const RESOURCE_SCALE: f32 = 4.0;

/// Pack a `CellType` (bits 0-3) and a resource scalar (bits 4-11, `u8`
/// fixed-point on `RESOURCE_SCALE`) into an `aux` value. Bits 12-15 start
/// zero (canopy density — see `with_canopy_density` below, which is the
/// only thing that ever writes them; a cell type with no `Grow`-driven
/// self-avoidance simply never has anything nonzero there).
pub fn pack_aux(cell_type: CellType, resource: f32) -> u16 {
    let type_bits = cell_type as u16;
    let resource_u8 = ((resource.clamp(0.0, RESOURCE_SCALE) / RESOURCE_SCALE) * 255.0).round() as u16;
    type_bits | (resource_u8 << 4)
}

/// Inverse of `pack_aux`. `cell_type` is `None` if the low 4 bits don't
/// match a known variant — a stale or corrupted value should read as
/// "nothing recognized" rather than panicking or silently aliasing to
/// whichever variant happens to sit at that bit pattern.
pub fn unpack_aux(aux: u16) -> (Option<CellType>, f32) {
    let type_bits = aux & 0b1111;
    let resource_u8 = (aux >> 4) & 0xFF;
    let resource = (resource_u8 as f32 / 255.0) * RESOURCE_SCALE;
    let cell_type = match type_bits {
        0 => Some(CellType::Seed),
        1 => Some(CellType::GrowingTip),
        2 => Some(CellType::MatureBody),
        3 => Some(CellType::Leaf),
        4 => Some(CellType::RootTip),
        _ => None,
    };
    (cell_type, resource)
}

/// Upper bound for the canopy-density scalar packed into `aux` bits 12-15
/// — `Reports/tree-rewrite-design.md` §2b's self-avoidance signal, and
/// §6's resolution of that report's own open question: the layout already
/// reserved these 4 bits as spare, so canopy density fits there directly
/// with no change to the resource scalar's own 8-bit precision, and
/// without `field.rs`'s coarse grid (which §6's own correction found would
/// need bilinear sampling to avoid silently flattening every candidate in
/// a `Grow` cell's 8-neighbourhood to the same value). 4 bits (16 levels)
/// is coarser than resource's 256, but this signal only ever answers "is
/// this general area already crowded," never needs cell-exact precision —
/// see `Grow`'s own `crowding_weight` doc.
pub const CANOPY_DENSITY_SCALE: f32 = 4.0;

/// Canopy density at this `aux` value — `0.0` for any cell that has never
/// had `with_canopy_density` called on it (a fresh `pack_aux` leaves bits
/// 12-15 zero), which is the correct reading for "nothing has grown near
/// here yet," not a sentinel to special-case.
pub fn canopy_density(aux: u16) -> f32 {
    let bits = (aux >> 12) & 0xF;
    (bits as f32 / 15.0) * CANOPY_DENSITY_SCALE
}

/// Rewrite only bits 12-15, leaving the cell type and resource scalar
/// `pack_aux` already wrote untouched — canopy density updates on a
/// different cadence (the diffusion pass, every sweep a chunk is awake)
/// than cell-type transitions or resource spending, so keeping it a
/// separate, independent write is what lets `diffuse_resource` update it
/// without needing to know or preserve the other two fields itself.
pub fn with_canopy_density(aux: u16, density: f32) -> u16 {
    let bits = ((density.clamp(0.0, CANOPY_DENSITY_SCALE) / CANOPY_DENSITY_SCALE) * 15.0).round() as u16;
    (aux & 0x0FFF) | (bits << 12)
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
        for (dx, dy) in NEIGHBOURS_4 {
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

// --- Resource and canopy-density diffusion ------------------------------
//
// `Reports/organism-substrate-design.md` §3's `diffuse_resource`, the same
// shape `fire.rs`'s `diffuse_heat` already uses -- generic over
// `CellSurface` (both the serial sweep and the parallel `ChunkView` run the
// identical rule), per-cell CA-resolution finite-difference diffusion, not
// `field.rs`'s coarse `FIELD_SCALE`-resolution grid (that resolution
// mismatch is exactly what `organism-substrate-design.md` §3 rejected the
// field grid for). A different-organism or non-`Plant` neighbour is a
// wall, exactly as `diffuse_heat` treats a material boundary. Carries
// `Reports/tree-rewrite-design.md` §2b's canopy-density channel alongside
// resource, since both are per-cell diffusing scalars packed into the same
// `aux` and visited by the same sweep -- not a second diffusion pass.

/// Diffusion rate for both channels -- see this module's own doc on why a
/// single shared rate, not yet a per-species `TransportChannel` value.
/// Clamped conceptually to the same 2D explicit-diffusion stability bound
/// (Fourier number <= 0.25) `fire.rs`'s `diffuse_heat`/`field.rs`'s own
/// diffusion already derive and respect; chosen well under that bound
/// rather than at it, since two channels diffusing through the same cell
/// every visited tick have more chances to compound than heat's single
/// channel does.
const DIFFUSION_RATE: f32 = 0.2;

/// Run one diffusion step for the organism-owned cell at `(x, y)` — a
/// no-op for `organism_id() == 0` (inert material) or an unrecognized
/// `aux` cell-type encoding, mirroring `diffuse_heat`'s own early return
/// for a thermally-inert material. Called from `update.rs`'s per-cell
/// dispatch (see that module's own call site for why the CA sweep, not
/// the M16 active-site schedule, is what actually drives this — a
/// `MatureBody` trunk cell needs to keep relaying resource even though it
/// is deliberately off the active-site schedule per `design-philosophy.md`
/// §3, and the CA sweep is the one pass that already visits every cell in
/// an awake chunk regardless of scheduling).
pub fn diffuse_resource<S: CellSurface>(surface: &mut S, x: i32, y: i32) {
    let cell = surface.get(x, y);
    let organism_id = cell.organism_id();
    if organism_id == 0 {
        return;
    }
    let (Some(cell_type), here_resource) = unpack_aux(cell.aux()) else {
        return;
    };
    let here_density = canopy_density(cell.aux());

    let is_wall = |n: Cell| n.organism_id() != organism_id || surface.materials().kind(n.material) != MaterialKind::Plant;

    let mut resource_sum = 0.0f32;
    let mut density_sum = 0.0f32;
    let mut neighbours = 0u32;
    for (dx, dy) in NEIGHBOURS_4 {
        let n = surface.get(x + dx, y + dy);
        if is_wall(n) {
            continue;
        }
        let (_, n_resource) = unpack_aux(n.aux());
        resource_sum += n_resource;
        density_sum += canopy_density(n.aux());
        neighbours += 1;
    }

    // An isolated organism cell (every neighbour a wall) has nothing to
    // exchange with, and nothing to do here at all -- unlike an earlier
    // version of this function, decay no longer lives on this per-frame
    // pass (see `plant::CANOPY_DENSITY_DECAY_PER_TICK`'s own doc for why:
    // a decay applied every single CA frame erases a fresh deposit within
    // a handful of frames, long before a neighbour's much-less-frequent
    // `Grow` check ever gets a chance to read it — a real bug, found by
    // live verification, not a hypothetical). This function's only job is
    // spatial spread now; `organism_tick`'s own per-cell cadence is what
    // fades a deposit over time.
    let (new_resource, new_density) = if neighbours == 0 {
        (here_resource, here_density)
    } else {
        let resource_avg = resource_sum / neighbours as f32;
        let density_avg = density_sum / neighbours as f32;
        let resource = here_resource + (resource_avg - here_resource) * DIFFUSION_RATE;
        let density = here_density + (density_avg - here_density) * DIFFUSION_RATE;
        (resource.max(0.0), density.clamp(0.0, CANOPY_DENSITY_SCALE))
    };

    let new_aux = with_canopy_density(pack_aux(cell_type, new_resource), new_density);
    if new_aux != cell.aux() {
        surface.set(x, y, cell.with_aux(new_aux));
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
    fn pack_and_unpack_aux_round_trip() {
        for cell_type in [CellType::Seed, CellType::GrowingTip, CellType::MatureBody, CellType::Leaf, CellType::RootTip] {
            for resource in [0.0, 1.0, 2.0, 3.999, RESOURCE_SCALE] {
                let packed = pack_aux(cell_type, resource);
                let (unpacked_type, unpacked_resource) = unpack_aux(packed);
                assert_eq!(unpacked_type, Some(cell_type));
                // Fixed-point round trip: within one packing step's worth of error.
                assert!(
                    (unpacked_resource - resource).abs() < RESOURCE_SCALE / 255.0 + 1e-4,
                    "resource {resource} round-tripped to {unpacked_resource} for {cell_type:?}"
                );
            }
        }
    }

    #[test]
    fn canopy_density_round_trips_and_leaves_cell_type_and_resource_untouched() {
        for density in [0.0, 1.0, 2.0, 3.999, CANOPY_DENSITY_SCALE] {
            let base = pack_aux(CellType::RootTip, 2.5);
            let with_density = with_canopy_density(base, density);
            assert_eq!(unpack_aux(with_density), unpack_aux(base), "with_canopy_density must not disturb cell type or resource");
            assert!(
                (canopy_density(with_density) - density).abs() < CANOPY_DENSITY_SCALE / 15.0 + 1e-4,
                "density {density} round-tripped to {}",
                canopy_density(with_density)
            );
        }
    }

    #[test]
    fn a_freshly_packed_aux_has_zero_canopy_density() {
        let packed = pack_aux(CellType::GrowingTip, 1.0);
        assert_eq!(canopy_density(packed), 0.0, "a cell nothing has ever deposited into should read as uncrowded, not a sentinel to special-case");
    }

    #[test]
    fn resource_is_clamped_into_range_rather_than_wrapping() {
        let packed = pack_aux(CellType::GrowingTip, -5.0);
        let (_, resource) = unpack_aux(packed);
        assert_eq!(resource, 0.0, "a negative resource should clamp to zero, not wrap");

        let packed = pack_aux(CellType::GrowingTip, 999.0);
        let (_, resource) = unpack_aux(packed);
        assert!((resource - RESOURCE_SCALE).abs() < 1e-4, "an overlarge resource should clamp to the scale's max");
    }

    #[test]
    fn an_unrecognized_type_bit_pattern_is_none() {
        // Bit pattern 5 doesn't correspond to any current CellType variant
        // (0-4 are Seed/GrowingTip/MatureBody/Leaf/RootTip).
        let (cell_type, _) = unpack_aux(5);
        assert_eq!(cell_type, None);
    }

    // --- diffuse_resource --------------------------------------------------

    #[test]
    fn resource_diffuses_from_a_full_cell_toward_an_empty_same_organism_neighbour() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_id = w.push_organism(SpeciesId(0));
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_aux(CellType::MatureBody, RESOURCE_SCALE)));
        w.set(6, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(pack_aux(CellType::MatureBody, 0.0)));

        diffuse_resource(&mut w, 5, 5);
        diffuse_resource(&mut w, 6, 5);

        let (_, left) = unpack_aux(w.get(5, 5).aux());
        let (_, right) = unpack_aux(w.get(6, 5).aux());
        assert!(right > 0.0, "the empty neighbour should have gained resource: got {right}");
        assert!(left < RESOURCE_SCALE, "the full cell should have lost some resource: got {left}");
    }

    #[test]
    fn resource_does_not_cross_an_organism_boundary() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_a = w.push_organism(SpeciesId(0));
        let organism_b = w.push_organism(SpeciesId(0));
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_a).with_aux(pack_aux(CellType::MatureBody, RESOURCE_SCALE)));
        w.set(6, 5, Cell::new(wood, 0).with_organism_id(organism_b).with_aux(pack_aux(CellType::MatureBody, 0.0)));

        diffuse_resource(&mut w, 5, 5);
        diffuse_resource(&mut w, 6, 5);

        let (_, left) = unpack_aux(w.get(5, 5).aux());
        let (_, right) = unpack_aux(w.get(6, 5).aux());
        assert_eq!(left, RESOURCE_SCALE, "a different organism's cell must be a wall, not a diffusion partner");
        assert_eq!(right, 0.0, "resource must not cross an organism boundary");
    }

    #[test]
    fn diffuse_resource_no_longer_decays_density_itself() {
        // Decay moved to `plant::organism_tick`'s own per-cell cadence
        // (see that module's `CANOPY_DENSITY_DECAY_PER_TICK` doc for why:
        // a decay applied on this function's per-CA-frame cadence erased a
        // fresh deposit within a handful of frames, long before a
        // neighbour's much-less-frequent `Grow` check ever read it). This
        // function's own job is spatial spread only now -- an isolated
        // cell (no diffusion partner) should see its density completely
        // unchanged after any number of calls.
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 15, 15));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let organism_id = w.push_organism(SpeciesId(0));
        let aux = with_canopy_density(pack_aux(CellType::MatureBody, 1.0), CANOPY_DENSITY_SCALE);
        w.set(5, 5, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        for _ in 0..500 {
            diffuse_resource(&mut w, 5, 5);
        }

        assert_eq!(
            canopy_density(w.get(5, 5).aux()),
            CANOPY_DENSITY_SCALE,
            "an isolated cell's density should not decay from repeated diffuse_resource calls alone"
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
