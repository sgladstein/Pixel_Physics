//! The organism substrate: species as data, and the behavior library
//! species files compose from. See `Reports/organism-substrate-design.md`
//! for the full design and the reasoning behind every decision here — this
//! module implements it, not re-derives it.
//!
//! **Scope of this pass.** Only what moss (the first species retrofitted,
//! per the design report's §7 retrofit order) actually needs: one behavior
//! (`Divide`), the `CellType` vocabulary, the `aux` cell-type/resource
//! encoding, the generational `organism_id` allocator, and the shared
//! bounded-reachability primitive `structural.rs` needs for organism-owned
//! `Plant` cells. Trees and the worm — and the behaviors only they need
//! (`Photosynthesize`, `Absorb`, `TransportChannel`, `SecondaryThicken`,
//! `Germinate`, `StructuralAnchor`, `Locomote`) — are deliberately deferred
//! to a later session; see `PLAN.md`'s own note on why. Adding a behavior
//! variant here with no species that constructs it yet would be dead code
//! by this crate's own standards, not a head start.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use serde::Deserialize;

use super::cell::Cell;
use super::surface::CellSurface;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// Shared vocabulary for what an organism-owned cell currently *is*, packed
/// into `Cell::aux`'s low 4 bits (`pack_aux`/`unpack_aux` below) — one
/// enum every species shares, so dispatch code never needs to know which
/// species it's looking at. Room for 11 more variants than are named yet;
/// only the ones an actual species behavior needs today are here.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub enum CellType {
    /// Moss's only cell type today. Named for the vocabulary's eventual
    /// tree use (an actively spreading/dividing cell), not because moss
    /// has anything resembling a tree's tip/trunk distinction.
    GrowingTip,
}

/// One behavior a cell type can carry, composed freely by species data —
/// `material.rs`'s "behaviour comes from data, not a branch per material"
/// claim, one level up. Only `Divide` exists yet; see the module doc for
/// why the rest of the design report's library isn't here. A struct-shaped
/// variant (fields directly on the enum) rather than a separate named
/// struct wrapped in a newtype variant, matching `ActiveKind`'s own
/// existing shape (e.g. `Moss { stale_ticks: u8 }`) and RON's more direct
/// syntax for it.
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

const EMBEDDED: &[&str] = &[include_str!("../../assets/species/moss.ron")];

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
/// fixed-point on `RESOURCE_SCALE`) into an `aux` value. Bits 12-15 stay
/// zero (reserved, see the design report).
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
        0 => Some(CellType::GrowingTip),
        _ => None,
    };
    (cell_type, resource)
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
        for resource in [0.0, 1.0, 2.0, 3.999, RESOURCE_SCALE] {
            let packed = pack_aux(CellType::GrowingTip, resource);
            let (cell_type, unpacked_resource) = unpack_aux(packed);
            assert_eq!(cell_type, Some(CellType::GrowingTip));
            // Fixed-point round trip: within one packing step's worth of error.
            assert!(
                (unpacked_resource - resource).abs() < RESOURCE_SCALE / 255.0 + 1e-4,
                "resource {resource} round-tripped to {unpacked_resource}"
            );
        }
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
        // Bit pattern 1 doesn't correspond to any current CellType variant.
        let (cell_type, _) = unpack_aux(1);
        assert_eq!(cell_type, None);
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
