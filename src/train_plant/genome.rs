//! **The genome is the species file.** A child is a `SpeciesDef` with a few
//! of its numbers moved, a name of its own, and a validator's clean bill
//! (`Reports/plant-trainer-plan-2026-10-19.md` §4).
//!
//! **Not the in-game mutation.** In play a seed takes one `ParamGenome`
//! override, one address drawn evenly, stepped additively on the corpus
//! scale and clamped by `clamp_param`. None of that is used here:
//!
//! - **`clamp_param`'s bound reads the whole registry**, a registered child
//!   included (`SpeciesRegistry::param_scale`), so a trainer that used it would
//!   widen its own bounds every time it registered a child. Each gene here
//!   carries the range its consumer reads it over, written in [`LEVERS`].
//! - **A step on the corpus scale is calibrated to the largest species**:
//!   `dead-ends.md`'s `seed_maturity` entry measured it at 175 against herb's
//!   60. Steps here are a fraction of the parent's own value.
//! - **No `exp`, `ln` or Gaussian draw**: a step is `1 + σu` or its inverse,
//!   so a run replays bit for bit on any machine (the ant rig's fix, design
//!   rev 3, *Learning*).
//!
//! An offline trainer may carry a per-gene range table that the in-game
//! mutator rightly may not (`dead-ends.md` warns against one there): every
//! child here is scored, and each range is its consumer's, not a guess.

use crate::sim::organism::{self, Behavior, CellType, FateGenome, ParamGenome, ParamId, SpeciesDef, BRANCH_ORDERS};
use crate::sim::rng::Rng;

/// What a gene belongs to -- the plant version of the ant rig's banned verbs.
/// A run names the groups that may move; every other gene is frozen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Shape,
    Leaf,
    Flower,
    Height,
    Root,
    Seed,
    Life,
}

impl Group {
    pub const ALL: [Group; 7] = [Group::Shape, Group::Leaf, Group::Flower, Group::Height, Group::Root, Group::Seed, Group::Life];

    pub fn name(self) -> &'static str {
        match self {
            Group::Shape => "shape",
            Group::Leaf => "leaf",
            Group::Flower => "flower",
            Group::Height => "height",
            Group::Root => "root",
            Group::Seed => "seed",
            Group::Life => "life",
        }
    }

    pub fn parse(s: &str) -> Option<Group> {
        Group::ALL.into_iter().find(|g| g.name() == s)
    }
}

/// How a gene moves.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Step {
    /// A fraction of the parent's own value: `x(1 + σu)` for `u >= 0`, `x / (1 +
    /// σ|u|)` below -- symmetric in log space to first order, and arithmetic.
    /// Never reaches or crosses zero, which is the sign lock steering weights
    /// need (crossing zero reverses a behaviour rather than nudging it).
    Factor,
    /// Additive in the gene's own units, `σu` times half its range.
    Add,
    /// An integer step of 1, 2 above σ 0.5.
    Count,
}

/// Where a gene lives in the species file.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Addr {
    /// A behaviour parameter of one cell type, at one branch order.
    Param { cell: CellType, param: ParamId, tier: u8 },
    /// The determinate terminal's `after_metamers` in the shoot's fate table.
    AfterMetamers,
    /// `SpeciesDef::annual_half_life`.
    AnnualHalfLife,
    /// `SpeciesDef::seed_half_life`.
    SeedHalfLife,
}

/// One mutable number.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Gene {
    pub addr: Addr,
    pub group: Group,
    pub step: Step,
    /// The consumer's range. A gene is clamped to it after every step.
    pub lo: f32,
    pub hi: f32,
}

impl Gene {
    /// A short name for a log or a CSV column.
    pub fn name(&self) -> String {
        match self.addr {
            Addr::Param { cell, param, tier } => {
                if organism::param_is_tiered(param) {
                    format!("{cell:?}.{}[{tier}]", param.name())
                } else {
                    format!("{cell:?}.{}", param.name())
                }
            }
            Addr::AfterMetamers => "after_metamers".into(),
            Addr::AnnualHalfLife => "annual_half_life".into(),
            Addr::SeedHalfLife => "seed_half_life".into(),
        }
    }
}

/// **The levers, with the range each is read over.** One row per parameter;
/// it becomes one gene per cell type that authors it (and per branch order
/// where the field is tiered). A parameter missing here is never mutated.
///
/// The ranges, and why. Every row is a bound the consumer imposes or a
/// mode boundary the plan names (§4.2), never a taste:
///
/// - **Zero is a mode, not a value**, for `internode`, `branch_angle`,
///   `plastochron` (0 disables nodes outright), `juvenile_size` (0 switches
///   the juvenile stage off) and every steering weight (a zero weight is a
///   sense the species does not have). A gene whose shipped value is zero is
///   therefore **frozen** ([`gene_table`] skips it), and a non-zero one moves
///   by [`Step::Factor`], which cannot reach zero.
/// - `max_active_tips` is at least 2: the cap counts the asking tip, so 1
///   means no growth at all (`herb.ron`, measured: a stand that staled at one
///   cell and rotted).
/// - `heading_inertia`, `turgor_taper` and the probabilities are blends or
///   chances on [0, 1].
/// - The rest move within a factor of 8 of the shipped value (`lo`/`hi` here
///   are multiples, applied in [`gene_table`]), which is wider than any gap
///   between two shipped species on these fields and narrow enough that a
///   divide cannot blow up.
const LEVERS: &[(ParamId, Group, Step, Bound)] = &[
    (ParamId::BranchChance, Group::Shape, Step::Add, Bound::Abs(0.0, 0.5)),
    (ParamId::MaxActiveTips, Group::Shape, Step::Count, Bound::Abs(2.0, 32.0)),
    (ParamId::Plastochron, Group::Shape, Step::Count, Bound::Abs(1.0, 20.0)),
    (ParamId::Internode, Group::Shape, Step::Count, Bound::Abs(1.0, 12.0)),
    (ParamId::BranchAngle, Group::Shape, Step::Factor, Bound::Abs(5.0, 120.0)),
    (ParamId::HeadingInertia, Group::Shape, Step::Add, Bound::Abs(0.0, 0.98)),
    (ParamId::ContinuationWeight, Group::Shape, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::LightWeight, Group::Shape, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::WindWeight, Group::Shape, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::UpwardWeight, Group::Shape, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::CrowdingWeight, Group::Shape, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::BudBreakAcrotony, Group::Shape, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::JuvenileSize, Group::Shape, Step::Count, Bound::Abs(1.0, 64.0)),
    (ParamId::JuvenileBranch, Group::Shape, Step::Add, Bound::Abs(0.0, 1.0)),
    (ParamId::LeafCluster, Group::Leaf, Step::Count, Bound::Abs(1.0, 24.0)),
    (ParamId::LeafSpread, Group::Leaf, Step::Add, Bound::Abs(0.0, 1.0)),
    (ParamId::PhotosynthesizeRate, Group::Leaf, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::ShadeDeath, Group::Leaf, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::DroughtDeath, Group::Leaf, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::OrganCluster, Group::Flower, Step::Count, Bound::Abs(1.0, 30.0)),
    (ParamId::RipenRate, Group::Flower, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::RipenCost, Group::Flower, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::TurgorSource, Group::Height, Step::Factor, Bound::Rel(0.25, 4.0)),
    (ParamId::TurgorPerCell, Group::Height, Step::Factor, Bound::Rel(0.25, 4.0)),
    (ParamId::TurgorTaper, Group::Height, Step::Add, Bound::Abs(0.0, 1.0)),
    (ParamId::PipeRatio, Group::Height, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::AbsorbRate, Group::Root, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::PenetrationForce, Group::Root, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::SeedCost, Group::Seed, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::ReproductiveAllocation, Group::Seed, Step::Add, Bound::Abs(0.01, 0.9)),
    (ParamId::SeedMaturity, Group::Seed, Step::Factor, Bound::Rel(0.125, 8.0)),
    (ParamId::GerminateWater, Group::Seed, Step::Add, Bound::Abs(0.0, 1.0)),
];

/// A range given absolutely, or as multiples of the shipped value.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Bound {
    Abs(f32, f32),
    Rel(f32, f32),
}

/// Cell types a plant's behaviours live on.
const CELLS: [CellType; 8] = [
    CellType::Seed,
    CellType::GrowingTip,
    CellType::MatureBody,
    CellType::Leaf,
    CellType::RootTip,
    CellType::DormantBud,
    CellType::Flower,
    CellType::Fruit,
];

/// A root's levers belong to [`Group::Root`] whatever the parameter, so a run
/// that moves the shoot's shape leaves the root system as shipped.
fn group_of(cell: CellType, group: Group) -> Group {
    if cell == CellType::RootTip {
        Group::Root
    } else {
        group
    }
}

/// Every gene `def` carries a non-zero value for, with ranges resolved
/// against its own (shipped) values. **Built once from the parent and
/// frozen for the run**: a child's genes are bounded by its ancestor's
/// table, never by itself, so a run cannot ratchet its own range.
pub fn gene_table(def: &SpeciesDef) -> Vec<Gene> {
    let mut out = Vec::new();
    for &cell in &CELLS {
        let Some((_, behaviors)) = def.cell_types.iter().find(|(c, _)| *c == cell) else {
            continue;
        };
        for &(param, group, step, bound) in LEVERS {
            let tiers = if organism::param_is_tiered(param) { BRANCH_ORDERS as u8 } else { 1 };
            for tier in 0..tiers {
                let Some(v) = behaviors.iter().find_map(|b| organism::read_param(b, param, tier)) else {
                    continue;
                };
                // Zero is a mode for every multiplicative lever (see
                // `LEVERS`); an additive one may start at zero and move off it.
                if v == 0.0 && step != Step::Add {
                    continue;
                }
                let (lo, hi) = match bound {
                    Bound::Abs(lo, hi) => (lo, hi),
                    Bound::Rel(lo, hi) => (v.abs() * lo, v.abs() * hi),
                };
                out.push(Gene { addr: Addr::Param { cell, param, tier }, group: group_of(cell, group), step, lo, hi });
            }
        }
    }
    if read(def, Addr::AfterMetamers).is_some() {
        out.push(Gene { addr: Addr::AfterMetamers, group: Group::Flower, step: Step::Count, lo: 1.0, hi: 30.0 });
    }
    if def.annual_half_life > 0.0 {
        let v = def.annual_half_life;
        out.push(Gene { addr: Addr::AnnualHalfLife, group: Group::Life, step: Step::Factor, lo: v / 8.0, hi: v * 8.0 });
    }
    if def.seed_half_life > 0.0 {
        let v = def.seed_half_life;
        out.push(Gene { addr: Addr::SeedHalfLife, group: Group::Life, step: Step::Factor, lo: v / 8.0, hi: v * 8.0 });
    }
    out
}

/// A gene's current value in `def`.
pub fn read(def: &SpeciesDef, addr: Addr) -> Option<f32> {
    match addr {
        Addr::Param { cell, param, tier } => def
            .cell_types
            .iter()
            .find(|(c, _)| *c == cell)
            .and_then(|(_, bs)| bs.iter().find_map(|b| organism::read_param(b, param, tier))),
        Addr::AfterMetamers => def
            .fates
            .iter()
            .find(|(c, _)| *c == CellType::GrowingTip)
            .and_then(|(_, rules)| rules.iter().find_map(|f| f.after_metamers))
            .map(f32::from),
        Addr::AnnualHalfLife => Some(def.annual_half_life),
        Addr::SeedHalfLife => Some(def.seed_half_life),
    }
}

/// Write a gene's value into `def`, through the same field writes a
/// `ParamGenome` override makes in play.
pub fn write(def: &mut SpeciesDef, addr: Addr, value: f32) {
    match addr {
        Addr::Param { cell, param, tier } => {
            let mut g = ParamGenome::default();
            g.set(cell, param, tier, value);
            if let Some((_, bs)) = def.cell_types.iter_mut().find(|(c, _)| *c == cell) {
                for b in bs.iter_mut() {
                    g.apply(cell, b);
                }
            }
        }
        Addr::AfterMetamers => {
            if let Some((_, rules)) = def.fates.iter_mut().find(|(c, _)| *c == CellType::GrowingTip) {
                for f in rules.iter_mut() {
                    if let Some(m) = f.after_metamers.as_mut() {
                        *m = value.round().clamp(1.0, 255.0) as u8;
                    }
                }
            }
        }
        Addr::AnnualHalfLife => def.annual_half_life = value,
        Addr::SeedHalfLife => def.seed_half_life = value,
    }
}

/// One gene moved by one step. `u` is uniform on [-1, 1].
pub fn stepped(gene: &Gene, value: f32, sigma: f32, u: f32) -> f32 {
    let moved = match gene.step {
        Step::Factor => {
            let f = if u >= 0.0 { 1.0 + sigma * u } else { 1.0 / (1.0 + sigma * -u) };
            value * f
        }
        Step::Add => value + sigma * u * 0.5 * (gene.hi - gene.lo),
        Step::Count => {
            let k = if sigma > 0.5 { 2.0 } else { 1.0 };
            if u >= 0.0 { value + k } else { value - k }
        }
    };
    moved.clamp(gene.lo, gene.hi)
}

/// A child of `parent`: `1..=max_genes` free genes moved. Returns what moved,
/// as `(gene index, from, to)`, for the run's log. **A draw that lands a
/// gene on its current value is kept** (a clamped step at a bound): the
/// child is then its parent on that gene, which is honest about the bound
/// rather than re-drawing until something moves.
pub fn mutate(parent: &SpeciesDef, genes: &[Gene], free: &[usize], sigma: f32, max_genes: usize, rng: &mut Rng) -> (SpeciesDef, Vec<(usize, f32, f32)>) {
    let mut child = clone_def(parent);
    let mut moved = Vec::new();
    if free.is_empty() {
        return (child, moved);
    }
    // **Distinct genes**: a gene drawn twice would be stepped twice and
    // logged with a stale first value (caught by
    // `a_mutation_writes_what_it_says_and_leaves_the_parent_alone`).
    let n = (1 + rng.below(max_genes.max(1) as u32) as usize).min(free.len());
    let mut picked: Vec<usize> = Vec::with_capacity(n);
    while picked.len() < n {
        let gi = free[rng.below(free.len() as u32) as usize];
        if !picked.contains(&gi) {
            picked.push(gi);
        }
    }
    for gi in picked {
        let gene = &genes[gi];
        let Some(from) = read(&child, gene.addr) else { continue };
        let u = rng.unit_f32() * 2.0 - 1.0;
        let to = stepped(gene, from, sigma, u);
        write(&mut child, gene.addr, to);
        // Read back: a `Count` rounds and a `u8` field saturates, so the
        // written value is what the plant will see, not `to`.
        let landed = read(&child, gene.addr).unwrap_or(to);
        moved.push((gi, from, landed));
    }
    (child, moved)
}

/// A deep copy, through the species file's own text. `SpeciesDef` is not
/// `Clone`, and a copy that went any other way could disagree with the file
/// a child is staged as.
pub fn clone_def(def: &SpeciesDef) -> SpeciesDef {
    let text = crate::sim::species_export::to_ron(def).expect("a species serialises");
    ron::from_str(&text).expect("a serialised species parses back")
}

/// Why a child may not be scored.
#[derive(Debug, PartialEq)]
pub enum Invalid {
    /// A material the species names is not loaded.
    Material(String),
    /// More fate rules than a plant can carry.
    TooManyFates(usize),
    /// The fate table does not survive the per-organism packing unchanged.
    FatesPacked,
    /// The name is one the registry already holds, so registering it would
    /// silently replace that species.
    NameTaken(String),
}

/// **The validator the parser does not have** (plan §4.4). `register_ron`
/// replaces a same-named species and checks nothing, and `FateGenome` packs a
/// table into at most `MAX_FATES` rules, silently dropping the rest: a file and
/// its plants could otherwise disagree.
pub fn validate(def: &SpeciesDef, materials: &crate::sim::material::MaterialRegistry, species: &organism::SpeciesRegistry) -> Result<(), Invalid> {
    if species.id_of(&def.name).is_some() {
        return Err(Invalid::NameTaken(def.name.clone()));
    }
    for m in [
        &def.shoot_material,
        &def.root_material,
        &def.leaf_material,
        &def.flower_material,
        &def.seed_material,
        &def.fruit_material,
        &def.windfall_material,
    ] {
        if !m.is_empty() && materials.id_of(m).is_none() {
            return Err(Invalid::Material(m.clone()));
        }
    }
    let rules: usize = def.fates.iter().map(|(_, r)| r.len()).sum();
    if rules > organism::MAX_FATES {
        return Err(Invalid::TooManyFates(rules));
    }
    if FateGenome::from_table(&def.fates).to_table() != def.fates {
        return Err(Invalid::FatesPacked);
    }
    Ok(())
}

/// The child's name: unique per run, generation and slot, and never a
/// shipped species' name.
pub fn child_name(base: &str, run: u64, generation: usize, slot: usize) -> String {
    format!("{base}_t{run}_g{generation}_c{slot}")
}

/// Every behaviour of `def` on `cell`, for callers that build a control arm
/// by hand (the teeth check's handicapped species).
pub fn behaviors_mut(def: &mut SpeciesDef, cell: CellType) -> Option<&mut Vec<Behavior>> {
    def.cell_types.iter_mut().find(|(c, _)| *c == cell).map(|(_, b)| b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn herb() -> SpeciesDef {
        ron::from_str(include_str!("../../assets/species/herb.ron")).expect("herb.ron parses")
    }

    #[test]
    fn the_herb_table_holds_its_shape_levers_and_skips_its_zero_modes() {
        let def = herb();
        let genes = gene_table(&def);
        let names: Vec<String> = genes.iter().map(Gene::name).collect();
        assert!(names.iter().any(|n| n == "GrowingTip.leaf_cluster"), "{names:?}");
        assert!(names.iter().any(|n| n == "after_metamers"), "{names:?}");
        assert!(names.iter().any(|n| n == "GrowingTip.max_active_tips"), "{names:?}");
        // herb's shoot authors `penetration_force: 0.0`: a multiplicative
        // lever at zero is a mode and must not be in the table.
        assert!(!names.iter().any(|n| n == "GrowingTip.penetration_force"), "{names:?}");
        // ...while branch chance, additive, may start at zero and move off it.
        assert!(names.iter().any(|n| n.starts_with("GrowingTip.branch_chance")), "{names:?}");
    }

    #[test]
    fn a_factor_step_never_reaches_or_crosses_zero() {
        let g = Gene { addr: Addr::AnnualHalfLife, group: Group::Life, step: Step::Factor, lo: 0.0, hi: 1e9 };
        for i in 0..=200 {
            let u = i as f32 / 100.0 - 1.0;
            let v = stepped(&g, 0.06, 0.9, u);
            assert!(v > 0.0, "u={u} gave {v}");
        }
        // Symmetric in log space to first order: up then down by the same |u|
        // lands back exactly.
        let up = stepped(&g, 2.0, 0.5, 0.6);
        let back = stepped(&g, up, 0.5, -0.6);
        assert!((back - 2.0).abs() < 1e-6, "{back}");
    }

    #[test]
    fn a_mutation_writes_what_it_says_and_leaves_the_parent_alone() {
        let parent = herb();
        let genes = gene_table(&parent);
        let free: Vec<usize> = (0..genes.len()).collect();
        let mut rng = Rng::new(7);
        let mut moved_any = false;
        for _ in 0..50 {
            let (child, moved) = mutate(&parent, &genes, &free, 0.5, 3, &mut rng);
            for &(gi, from, to) in &moved {
                assert_eq!(read(&parent, genes[gi].addr), read(&herb(), genes[gi].addr), "the parent moved");
                assert_eq!(read(&child, genes[gi].addr).expect("the child has the gene"), to);
                assert!(to >= genes[gi].lo && to <= genes[gi].hi, "{} out of range: {to}", genes[gi].name());
                moved_any |= from != to;
            }
        }
        assert!(moved_any, "fifty children and not one gene moved");
    }

    #[test]
    fn mutation_is_a_function_of_the_seed() {
        let parent = herb();
        let genes = gene_table(&parent);
        let free: Vec<usize> = (0..genes.len()).collect();
        let a = mutate(&parent, &genes, &free, 0.4, 3, &mut Rng::new(11)).1;
        let b = mutate(&parent, &genes, &free, 0.4, 3, &mut Rng::new(11)).1;
        assert_eq!(a, b);
    }

    #[test]
    fn the_validator_refuses_a_shipped_name_and_a_missing_material() {
        let world = crate::sim::world::World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
        let mut def = herb();
        assert_eq!(validate(&def, &world.materials, &world.species), Err(Invalid::NameTaken("herb".into())));
        def.name = child_name("herb", 1, 0, 0);
        assert_eq!(validate(&def, &world.materials, &world.species), Ok(()));
        def.leaf_material = "no_such_leaf".into();
        assert_eq!(validate(&def, &world.materials, &world.species), Err(Invalid::Material("no_such_leaf".into())));
    }
}
