//! **A pool of plant genomes that founders are drawn from** (owner,
//! 2026-10-24: the plant line trained the way the ant line is, by selecting
//! whole beds -- `Reports/plant-community-2026-10-20/m2/bedsel/`).
//!
//! A species file says what a species is *on average*; a pool is a
//! population of individuals selected in the mixed bed, each one a full
//! heritable state (`OrganismState`'s genotype draws, alleles, param
//! overrides, lineage seed and fate table). `World::plant_tree_species`
//! gives each founder of a pooled species the next genome from its list,
//! so a sown bed starts from the selected population rather than from the
//! species file -- the plant counterpart of the ant's `std_pool.txt`.
//!
//! The file is the `ron` that `plant_m0 mode=bedsel save=` writes. Species
//! with no entry in the pool found from their file exactly as before.

use super::organism::{self, CellType, Fate, FateGenome, ParamGenome, ParamId};
use std::collections::BTreeMap;

/// One pooled individual as the file holds it.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedGenome {
    pub role: String,
    pub draws: Vec<f32>,
    pub alleles: Vec<u8>,
    pub params: Vec<(CellType, ParamId, u8, f32)>,
    pub fates: Vec<(CellType, Vec<Fate>)>,
    pub lineage: u64,
}

/// One pooled individual, ready to write onto a founder.
#[derive(Clone, Copy)]
pub struct PoolGenome {
    pub draws: [f32; organism::GENOTYPE_TRAITS],
    pub alleles: [u8; organism::DISCRETE_LOCI],
    pub params: ParamGenome,
    pub lineage: u64,
    pub fates: FateGenome,
}

impl PoolGenome {
    pub fn to_saved(&self, role: &str) -> SavedGenome {
        SavedGenome {
            role: role.to_string(),
            draws: self.draws.to_vec(),
            alleles: self.alleles.to_vec(),
            params: self.params.overrides().iter().map(|o| (o.cell_type, o.param, o.tier, o.value)).collect(),
            fates: self.fates.to_table(),
            lineage: self.lineage,
        }
    }

    fn from_saved(g: SavedGenome) -> Result<(String, Self), String> {
        let mut params = ParamGenome::default();
        for (c, p, t, v) in g.params {
            if !params.set(c, p, t, v) {
                return Err(format!("{}: a param override did not fit", g.role));
            }
        }
        let draws = g.draws.try_into().map_err(|_| format!("{}: draws are not {} long", g.role, organism::GENOTYPE_TRAITS))?;
        let alleles = g.alleles.try_into().map_err(|_| format!("{}: alleles are not {} long", g.role, organism::DISCRETE_LOCI))?;
        Ok((g.role, Self { draws, alleles, params, lineage: g.lineage, fates: FateGenome::from_table(&g.fates) }))
    }
}

/// Genomes by species name, in file order.
#[derive(Clone, Default)]
pub struct PlantPool {
    pub by_species: BTreeMap<String, Vec<PoolGenome>>,
}

impl PlantPool {
    pub fn parse(text: &str) -> Result<Self, String> {
        let list: Vec<SavedGenome> = ron::from_str(text).map_err(|e| e.to_string())?;
        let mut by_species: BTreeMap<String, Vec<PoolGenome>> = BTreeMap::new();
        for g in list {
            let (role, genome) = PoolGenome::from_saved(g)?;
            by_species.entry(role).or_default().push(genome);
        }
        Ok(Self { by_species })
    }

    pub fn to_ron(&self) -> String {
        let list: Vec<SavedGenome> = self.by_species.iter().flat_map(|(role, gs)| gs.iter().map(move |g| g.to_saved(role))).collect();
        ron::ser::to_string_pretty(&list, ron::ser::PrettyConfig::default()).expect("a pool serialises")
    }

    pub fn len(&self) -> usize {
        self.by_species.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// **The shipped pool**: `assets/plant_pools/roles4.ron`, eight rounds of
/// bed selection on the four-role bed (tree, shrub, herb, creeper; 32 each).
/// Judged on 16 fresh paired beds at 450,000 frames: roles a bed 2.56 ->
/// 3.44, the creeper alive on 16 of 16 against 2. Compiled in, as species
/// files are (`.claude/rules/assets.md`).
pub const SHIPPED: &str = include_str!("../../assets/plant_pools/roles4.ron");

/// What the lab box founds from (`LabBox::plant_pool`): the shipped pool, unless
/// `PIXEL_PHYSICS_PLANT_POOL=0` (species files only, the engine before
/// 2026-10-24) or `PIXEL_PHYSICS_PLANT_POOL=PATH` (another pool file).
pub fn default_pool() -> Option<std::sync::Arc<PlantPool>> {
    use std::sync::{Arc, OnceLock};
    static POOL: OnceLock<Option<Arc<PlantPool>>> = OnceLock::new();
    POOL.get_or_init(|| {
        let text = match std::env::var("PIXEL_PHYSICS_PLANT_POOL").as_deref() {
            Ok("0") | Ok("off") => return None,
            Ok(path) if !path.is_empty() => std::fs::read_to_string(path).unwrap_or_else(|e| panic!("PIXEL_PHYSICS_PLANT_POOL={path}: {e}")),
            _ => SHIPPED.to_string(),
        };
        Some(Arc::new(PlantPool::parse(&text).unwrap_or_else(|e| panic!("plant pool does not parse: {e}"))))
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped pool parses, holds the four roles, and writes back to
    /// the same genomes.
    #[test]
    fn the_shipped_pool_parses_and_round_trips() {
        let pool = PlantPool::parse(SHIPPED).expect("the shipped pool parses");
        for role in ["tree", "shrub", "herb", "creeper"] {
            assert!(pool.by_species.get(role).is_some_and(|g| !g.is_empty()), "the shipped pool has no {role}");
        }
        let again = PlantPool::parse(&pool.to_ron()).expect("a written pool parses");
        assert_eq!(again.len(), pool.len());
        for (role, gs) in &pool.by_species {
            for (a, b) in gs.iter().zip(&again.by_species[role]) {
                assert_eq!(a.draws, b.draws);
                assert_eq!(a.alleles, b.alleles);
                assert_eq!(a.params, b.params);
                assert_eq!(a.fates, b.fates);
                assert_eq!(a.lineage, b.lineage);
            }
        }
    }
}
