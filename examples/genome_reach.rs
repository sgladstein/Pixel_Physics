//! **What phenotypes can this species' genome actually reach?** — the
//! reachability census the plant line has never had.
//!
//! `genome_drift` answers *does a slot move*, and it answers it about the
//! **draw**: the heritable unit number in `-1..=1`. That is one half of the
//! question and it is the half that cannot be false. `seed_genotype` fills
//! every slot of every individual unconditionally, so **every slot's draw
//! moves, always**, whether or not anything downstream reads it. What
//! decides whether a lineage can express a different plant is the map from
//! the draw to the phenotype, and that map is
//!
//! ```text
//! phenotype = species_base * (1 + draw * species_variance)
//! ```
//!
//! — `plant::genotype`. **Both of its other two arguments are authored per
//! species and neither is heritable**, so the set of phenotypes a lineage
//! can ever reach on a slot is the closed interval
//! `[base*(1-variance), base*(1+variance)]`, clamped at zero, and it is
//! fixed the moment somebody writes the `.ron`. Two ways that interval
//! collapses to a single point, and this harness exists to count them:
//!
//! - **`base == 0` — caged.** Zero times any genome is zero. A species
//!   that authors `branch_chance: [0.0, 0.0]` has a lineage that can never
//!   discover branching, at any mutation rate, for ever.
//! - **`variance == 0` — pinned.** The slot expresses the authored value
//!   and only that. Note this is *usually not a cage*: slots 1/5/8 are read
//!   from the `RootTip` vector and the rest from the shoot's, so a zero in
//!   the other vector encodes slot **ownership** rather than a decision to
//!   freeze. The table below reads each slot from the vector its consumer
//!   actually reads, which is what separates the two.
//!
//! ```text
//! cargo run --release --example genome_reach                       # the static table, every species
//! cargo run --release --example genome_reach -- grow=1 species=herb  # the dynamic half
//! ```
//!
//! ## The dynamic half, and why it is a hash
//!
//! The static table says what the arithmetic permits. It cannot say whether
//! a permitted interval *does* anything — a slot can have a wide interval
//! and a consumer that never fires in this scene, which is
//! `CLAUDE.md`'s channel-with-no-effective-reader. `grow=1` settles that by
//! **widening one slot's variance to `wide=` and hashing the whole world**
//! against a baseline run of the same scene. Nothing else differs, and
//! widening a variance consumes no draw — `seed_genotype` fills the vector
//! whatever the widths are — so the two arms share an RNG stream and any
//! difference at all is the slot expressing itself.
//!
//! **A hash rather than a phenotype summary, deliberately.** Every summary
//! statistic this repo has reached for over plants (cells, height, spread,
//! foliage share) is a lossy projection, and `CLAUDE.md` records six
//! separate occasions where a number was arithmetically correct and about a
//! different question. `hash unchanged` is not a projection: it says the
//! two worlds are the same world, cell for cell, and a slot that cannot
//! move one cell of one plant across a whole run has no phenotype. It is
//! also the only statement of this kind that cannot be explained away by
//! spread — outcomes here are chaotic in the seed, so a *changed* hash is
//! weak evidence of importance and an *unchanged* one is conclusive
//! evidence of nothing.
//!
//! **The positive control is printed and asserted on every run**, because a
//! null here is exactly the shape that hides (`CLAUDE.md`, *ask what your
//! number counts when nothing is wrong*, and its sensitivity half): a
//! harness that patches the wrong species, or patches after the founders are
//! planted, reports every slot dead and looks like a decisive finding. The
//! control is a slot the static table has already said is live for this
//! species — non-zero base, non-zero variance — and its hash **must** move.
//! If none exists the run refuses rather than reporting a page of nulls.

mod common;

use pixel_physics::sim::organism::{self, Behavior, CellType};
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

/// Short slot labels in `organism::GENOTYPE_TRAITS`' order. Positional, like
/// the map they name: if these disagree with that doc, that doc is right.
const SLOT_NAMES: [&str; organism::GENOTYPE_TRAITS] =
    ["branch", "rootbr", "plast", "turgor", "pipe", "roottr", "alloc", "stoma", "penetr", "strain"];

/// Which cell type's `genotype_variance` vector each slot's consumer reads.
///
/// Not cosmetic and not guessable from the slot map alone: slot 4 (pipe
/// ratio) multiplies a `SecondaryThicken` field on `MatureBody` and reads
/// its **width from the shoot's** vector (`plant.rs`'s
/// `let pipe_variance = shoot_variance[4]`), and slot 6 does the same. A
/// census that read widths from the behaviour holding the base would report
/// both pinned.
fn variance_owner(slot: usize) -> CellType {
    match slot {
        1 | 5 | 8 => CellType::RootTip,
        _ => CellType::GrowingTip,
    }
}

/// The species' `Grow` variance vector for one cell type, or zeros if it has
/// no `Grow` there.
fn variance_vector(w: &World, species: organism::SpeciesId, ct: CellType) -> [f32; organism::GENOTYPE_TRAITS] {
    w.species
        .get(species)
        .behaviors(ct)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { genotype_variance, .. } => Some(*genotype_variance),
            _ => None,
        })
        .unwrap_or([0.0; organism::GENOTYPE_TRAITS])
}

/// What one slot multiplies, read from the species file exactly as its
/// consumer reads it.
///
/// `None` means *there is no authored scalar behind this slot* rather than
/// *the scalar is zero*, and the two are different findings — slot 6's base
/// is assembled at runtime from water status and anchor stress, so it is
/// never zero and can never be caged, and slot 9 has no consumer at all.
/// Reporting either as `base 0.0` would put a live channel and a missing one
/// in the same column.
enum Base {
    /// The authored value, per branch order. A `ByOrder` is four tiers and a
    /// slot is caged only if **every** tier is zero — a species with
    /// `branch_chance: [0.0, 0.35, 0.35]` can branch at order 1 and not at
    /// the trunk, which is a shape decision rather than a cage.
    Tiered([f32; organism::BRANCH_ORDERS]),
    Scalar(f32),
    /// Assembled at runtime; no authored base to be zero.
    Runtime(&'static str),
    /// Drawn, inherited, mutated, and read by nothing.
    NoConsumer,
}

fn grow_of(w: &World, species: organism::SpeciesId, ct: CellType) -> Option<Behavior> {
    w.species.get(species).behaviors(ct).iter().find(|b| matches!(b, Behavior::Grow { .. })).copied()
}

/// **A slot can have more than one consumer, and slot 1 does.** This
/// returns every authored scalar a slot multiplies, so a slot counts as
/// caged only when **all** of them are zero.
///
/// **This function is a correction, and the correction was made by the
/// dynamic arm rather than by reading the code.** The first version returned
/// one base per slot and reported slot 1 CAGED on `tree`, `conifer` and
/// `shrub` — whose roots author `branch_chance: [0.0]` — which reads as
/// *these species can never evolve a branching root system*. `grow=1` then
/// widened slot 1 on `tree` and the world **moved**, on every seed. The
/// reason is in the species files in as many words: *"Superseded by
/// `branch_priming` below"*. Slot 1 divides the root's priming interval
/// (`plant.rs`'s "the branching oscillator"), so the authored zero is a
/// retired mechanism rather than a cage, and the slot is live through its
/// other consumer.
///
/// The general form, which is why this is written here rather than fixed
/// quietly: **a reachability census taken by reading one call site is a
/// census of that call site.** The arithmetic table and the widening arm
/// disagreeing is the instrument working; the arm is the one to believe.
fn bases_of(w: &World, species: organism::SpeciesId, slot: usize) -> Vec<(&'static str, Base)> {
    let mut out = vec![("", base_of(w, species, slot))];
    if slot == 1 {
        out[0].0 = "root branch_chance";
        out.push((
            "root branch_priming",
            match grow_of(w, species, CellType::RootTip) {
                Some(Behavior::Grow { branch_priming, .. }) => {
                    let mut v = [0.0f32; organism::BRANCH_ORDERS];
                    for (order, dst) in v.iter_mut().enumerate() {
                        *dst = branch_priming.at(order as u8) as f32;
                    }
                    Base::Tiered(v)
                }
                _ => Base::Scalar(0.0),
            },
        ));
    }
    out
}

fn base_of(w: &World, species: organism::SpeciesId, slot: usize) -> Base {
    let tiers = |f: &dyn Fn(u8) -> f32| {
        let mut v = [0.0f32; organism::BRANCH_ORDERS];
        for (order, dst) in v.iter_mut().enumerate() {
            *dst = f(order as u8);
        }
        Base::Tiered(v)
    };
    match slot {
        0 => match grow_of(w, species, CellType::GrowingTip) {
            Some(Behavior::Grow { branch_chance, .. }) => tiers(&|o| branch_chance.at(o)),
            _ => Base::Scalar(0.0),
        },
        1 => match grow_of(w, species, CellType::RootTip) {
            Some(Behavior::Grow { branch_chance, .. }) => tiers(&|o| branch_chance.at(o)),
            _ => Base::Scalar(0.0),
        },
        2 => match grow_of(w, species, CellType::GrowingTip) {
            Some(Behavior::Grow { plastochron, .. }) => tiers(&|o| plastochron.at(o) as f32),
            _ => Base::Scalar(0.0),
        },
        3 => match grow_of(w, species, CellType::GrowingTip) {
            Some(Behavior::Grow { turgor_per_cell, .. }) => Base::Scalar(turgor_per_cell),
            _ => Base::Scalar(0.0),
        },
        4 => Base::Scalar(
            w.species
                .get(species)
                .behaviors(CellType::MatureBody)
                .iter()
                .find_map(|b| match b {
                    Behavior::SecondaryThicken { pipe_ratio } => Some(*pipe_ratio),
                    _ => None,
                })
                .unwrap_or(0.0),
        ),
        5 => match grow_of(w, species, CellType::RootTip) {
            Some(Behavior::Grow { upward_weight, .. }) => tiers(&|o| upward_weight.at(o)),
            _ => Base::Scalar(0.0),
        },
        6 => Base::Runtime("ROOT_BIAS_AT_FULL_WATER + (1-status) + anchor_stress"),
        7 => Base::Scalar(w.species.get(species).stomatal_reserve),
        8 => match grow_of(w, species, CellType::RootTip) {
            Some(Behavior::Grow { penetration_force, .. }) => Base::Scalar(penetration_force),
            _ => Base::Scalar(0.0),
        },
        _ => Base::NoConsumer,
    }
}

/// One row of the static table.
struct Reach {
    base: String,
    variance: f32,
    /// `None` where there is no authored base to bound.
    interval: Option<(f32, f32)>,
    verdict: &'static str,
}

fn reach(base: &Base, variance: f32) -> Reach {
    match base {
        Base::NoConsumer => Reach { base: "-".into(), variance, interval: None, verdict: "NO CONSUMER" },
        Base::Runtime(expr) => {
            Reach { base: (*expr).into(), variance, interval: None, verdict: if variance > 0.0 { "live" } else { "PINNED" } }
        }
        Base::Scalar(b) => {
            let (lo, hi) = ((b * (1.0 - variance)).max(0.0), b * (1.0 + variance));
            Reach {
                base: format!("{b}"),
                variance,
                interval: Some((lo, hi)),
                verdict: if *b == 0.0 {
                    "CAGED (base 0)"
                } else if variance <= 0.0 {
                    "PINNED (var 0)"
                } else {
                    "live"
                },
            }
        }
        Base::Tiered(v) => {
            let all_zero = v.iter().all(|&x| x == 0.0);
            let lo = v.iter().cloned().fold(f32::INFINITY, f32::min) * (1.0 - variance);
            let hi = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max) * (1.0 + variance);
            // Print only the tiers that differ, the way a `.ron` writes them.
            let mut shown: Vec<String> = v.iter().map(|x| format!("{x}")).collect();
            while shown.len() > 1 && shown.last() == shown.get(shown.len() - 2) {
                shown.pop();
            }
            Reach {
                base: format!("[{}]", shown.join(",")),
                variance,
                interval: Some((lo.max(0.0), hi)),
                verdict: if all_zero {
                    "CAGED (base 0)"
                } else if variance <= 0.0 {
                    "PINNED (var 0)"
                } else {
                    "live"
                },
            }
        }
    }
}

/// The species this harness censuses: everything compiled in that grows.
///
/// Read off the registry rather than hardcoded, so a species file added
/// tomorrow appears without editing this list — and a species that is a
/// creature is excluded by asking whether it has a `Grow` on a `GrowingTip`,
/// which is the property the census is about rather than a name.
fn plant_species(w: &World) -> Vec<(String, organism::SpeciesId)> {
    let mut out = Vec::new();
    for i in 0..w.species.len() {
        let id = organism::SpeciesId(i as u16);
        let sp = w.species.get(id);
        if grow_of(w, id, CellType::GrowingTip).is_some() {
            out.push((sp.name.clone(), id));
        }
    }
    out
}

fn arg<T: std::str::FromStr>(name: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
}

fn sarg(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// A cheap order-sensitive digest of the whole grid — the same shape
/// `sim::frame`'s own control test uses.
fn world_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let Some(b) = w.bounds() else { return 0 };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
        }
    }
    h
}

fn main() {
    let grow: u32 = arg("grow").unwrap_or(0);
    let drift: u32 = arg("drift").unwrap_or(0);
    if arg::<u32>("rhizome").unwrap_or(0) > 0 {
        rhizome();
    } else if drift > 0 {
        param_drift();
    } else if grow == 0 {
        static_table();
    } else {
        dynamic_arms();
    }
}

// ---------------------------------------------------------- param drift --

/// **Which parameters does a population actually move, and where do they
/// stop?** — the free-lever census, and it can only be run with this
/// mechanism.
///
/// `plant-heritability-survey-design-2026-08-27.md` §2 states the trap this
/// answers: *a free lever made heritable produces uniformity, not diversity*
/// — a quantity with a benefit and no counterweight has exactly one optimum,
/// which a working economy finds and holds every plant at. Its §4a then
/// **inventories** nine parameters as free by reading the code. An inventory
/// is an argument; this is the measurement, and the two can disagree.
///
/// The readout is one row per address any live plant has overridden: how many
/// carry it, the authored value, the population's median, and **how many sit
/// at the `clamp_param` bound**. A parameter whose population piles up at its
/// bound is either free or strongly directional, and either way it is the
/// list to price before raising `plant::PARAM_MUTATION_CHANCE`.
///
/// ```text
/// cargo run --release --example genome_reach -- drift=1 species=herb frames=45000 rate=0.3
/// ```
///
/// **The rate is an argument and is echoed**, because a run at the shipped
/// default of 0.0 produces an empty table that reads exactly like *nothing
/// drifts* — `CLAUDE.md`'s knob-nobody-can-see-the-value-of. It refuses
/// outright at rate 0 rather than printing that page.
fn param_drift() {
    let species_name = sarg("species").unwrap_or_else(|| "herb".to_string());
    let founders: usize = arg("founders").unwrap_or(8);
    let frames: u64 = arg("frames").unwrap_or(45_000);
    let rate: f32 = arg("rate").unwrap_or(0.3);
    let sigma: f32 = arg("sigma").unwrap_or(pixel_physics::sim::plant::PARAM_MUTATION_SIGMA);
    let worldseed: Option<u64> = arg("worldseed");
    if rate <= 0.0 {
        println!("REFUSING: rate=0 produces an empty table that reads as `nothing drifts`. Pass rate=0.3.");
        std::process::exit(2);
    }
    let d = common::PlantScene::default();
    let scene = common::PlantScene {
        trees: founders,
        width: d.width * (founders as i32).max(1) / d.trees as i32,
        species: species_name.to_string(),
        seed: worldseed,
        ..Default::default()
    };
    let mut w = scene.build();
    w.param_mutation_chance = rate;
    w.param_mutation_sigma = sigma;
    let id = w.species.id_of(&species_name).expect("species is compiled in");
    println!(
        "genome_reach: mode=drift species={species_name} founders={founders} frames={frames} rate={rate} sigma={sigma}          worldseed={:?} addresses={}",
        w.seed,
        w.species.param_addresses(id).len()
    );
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }

    // Census the standing population's override tables, off the grid, so a
    // dead organism holding a slot cannot be counted.
    let b = w.bounds().expect("the plant scene sets bounds");
    let mut owners: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let o = w.get(x, y).organism_id();
            if o != 0 {
                owners.insert(o);
            }
        }
    }
    // key -> (count, values, at_bound)
    let mut rows: std::collections::BTreeMap<String, (usize, Vec<f32>, usize)> = std::collections::BTreeMap::new();
    let mut carriers = 0usize;
    let mut depth_hist: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for o in &owners {
        let Some(g) = w.organism_params(*o) else { continue };
        *depth_hist.entry(g.len()).or_default() += 1;
        if !g.is_empty() {
            carriers += 1;
        }
        for ov in g.overrides() {
            let scale = w.species.param_scale(ov.param);
            let lo = organism::clamp_param(ov.param, -1e6, scale);
            let hi = organism::clamp_param(ov.param, 1e6, scale);
            let at_bound = (ov.value - lo).abs() < 1e-4 || (ov.value - hi).abs() < 1e-4;
            let key = format!("{:?}/{}/t{}", ov.cell_type, ov.param.name(), ov.tier);
            let e = rows.entry(key).or_insert((0, Vec::new(), 0));
            e.0 += 1;
            e.1.push(ov.value);
            e.2 += usize::from(at_bound);
        }
    }
    println!(
        "  live organisms {}, of which carrying at least one override: {carriers}   rolls {} applied {}",
        owners.len(),
        w.param_mutation_rolls,
        w.param_mutations_applied
    );
    println!("  overrides per individual: {depth_hist:?}   (0 = a founder, or a lineage that never drew one)");
    // **Two free controls, and they are read first.** `rolls` at zero means
    // nothing bred, so an empty table below is a statement about the scene
    // rather than about drift; and `applied` far below `rolls * rate` means
    // the operator is declining, which is a different finding from the
    // population not carrying anything.
    if w.param_mutation_rolls == 0 {
        println!("  REFUSING to read the table: no birth rolled, so nothing below is about drift.");
        return;
    }
    println!();
    println!("  {:<44} {:>6} {:>10} {:>10} {:>10} {:>8}", "address", "n", "authored", "median", "bound hi", "at bound");
    let mut ranked: Vec<_> = rows.into_iter().collect();
    ranked.sort_by_key(|(_, (n, _, _))| std::cmp::Reverse(*n));
    for (key, (n, mut values, at_bound)) in ranked {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let med = values[values.len() / 2];
        // Re-derive the address from the key's own parts to print the
        // authored value beside the drifted one.
        let parts: Vec<&str> = key.split('/').collect();
        let param = organism::ALL_PARAM_IDS.iter().find(|p| p.name() == parts[1]).copied();
        let (authored, hi) = match param {
            Some(p) => {
                let scale = w.species.param_scale(p);
                let ct = organism::PLANT_CELL_TYPES.iter().find(|c| format!("{c:?}") == parts[0]).copied();
                let tier: u8 = parts[2].trim_start_matches('t').parse().unwrap_or(0);
                let a = ct
                    .and_then(|c| w.species.param_in_force(id, &organism::ParamGenome::default(), (c, p, tier)))
                    .unwrap_or(f32::NAN);
                (a, organism::clamp_param(p, 1e6, scale))
            }
            None => (f32::NAN, f32::NAN),
        };
        println!("  {key:<44} {n:>6} {authored:>10.4} {med:>10.4} {hi:>10.3} {at_bound:>8}");
    }
}

// ---------------------------------------------------------------- static --

fn static_table() {
    let scene = common::PlantScene { trees: 0, ..Default::default() };
    let w = scene.build();
    let species = plant_species(&w);
    println!(
        "genome_reach: mode=static species={} slots={} loci={} (phenotype = base * (1 + draw*variance), draw in -1..=1)",
        species.len(),
        organism::GENOTYPE_TRAITS,
        organism::DISCRETE_LOCI
    );
    println!();

    let mut caged = 0usize;
    let mut pinned = 0usize;
    let mut live = 0usize;
    let mut cells = 0usize;
    let mut caged_by_slot = [0usize; organism::GENOTYPE_TRAITS];

    for (name, id) in &species {
        let shoot = variance_vector(&w, *id, CellType::GrowingTip);
        let root = variance_vector(&w, *id, CellType::RootTip);
        println!("--- {name} ---");
        println!("  {:>2} {:<7} {:>26} {:>6} {:>22}  verdict", "#", "slot", "authored base", "var", "reachable phenotype");
        for slot in 0..organism::GENOTYPE_TRAITS {
            let variance = match variance_owner(slot) {
                CellType::RootTip => root[slot],
                _ => shoot[slot],
            };
            let consumers = bases_of(&w, *id, slot);
            let reaches: Vec<Reach> = consumers.iter().map(|(_, b)| reach(b, variance)).collect();
            // A slot is caged only when **every** consumer's base is zero.
            let verdict = if reaches.iter().all(|r| r.verdict.starts_with("CAGED")) {
                "CAGED (base 0)"
            } else if reaches.iter().all(|r| r.verdict.starts_with("PINNED")) {
                "PINNED (var 0)"
            } else if reaches.iter().all(|r| r.verdict == "NO CONSUMER") {
                "NO CONSUMER"
            } else {
                "live"
            };
            for (i, (label, _)) in consumers.iter().enumerate() {
                let r = &reaches[i];
                let interval = match r.interval {
                    Some((lo, hi)) if (hi - lo).abs() < f32::EPSILON => format!("{{{lo}}}"),
                    Some((lo, hi)) => format!("[{lo:.4}, {hi:.4}]"),
                    None => "-".into(),
                };
                let name = if i == 0 { SLOT_NAMES[slot] } else { "  +also" };
                let shown = if label.is_empty() { r.base.clone() } else { format!("{} = {}", label, r.base) };
                println!(
                    "  {:>2} {:<7} {:>26} {:>6} {:>22}  {}",
                    if i == 0 { slot.to_string() } else { String::new() },
                    name,
                    shown,
                    r.variance,
                    interval,
                    if i == 0 { verdict } else { r.verdict }
                );
            }
            cells += 1;
            match verdict {
                v if v.starts_with("CAGED") => {
                    caged += 1;
                    caged_by_slot[slot] += 1;
                }
                v if v.starts_with("PINNED") => pinned += 1,
                "NO CONSUMER" => {}
                _ => live += 1,
            }
        }
        println!();
    }

    println!("== the continuous genome, pooled over {} species x {} slots = {cells} cells ==", species.len(), organism::GENOTYPE_TRAITS);
    println!("  live (a real interval a lineage can move inside): {live}");
    println!("  CAGED  (authored base is zero -- no mutation can ever leave it): {caged}");
    println!("  PINNED (authored variance is zero -- the value is the species'): {pinned}");
    println!("  no consumer (drawn, inherited, mutated, read by nothing): {}", cells - live - caged - pinned);
    print!("  caged by slot:");
    for slot in 0..organism::GENOTYPE_TRAITS {
        if caged_by_slot[slot] > 0 {
            print!(" {}={}", SLOT_NAMES[slot], caged_by_slot[slot]);
        }
    }
    println!();
    println!();

    // The other two heritable channels, on the same page, because "the
    // genome is small" is a claim about all three and reading one alone
    // invites the wrong conclusion in either direction.
    println!("== the discrete genome ==");
    println!("  {} loci, alleles per locus {:?}", organism::DISCRETE_LOCI, organism::LOCUS_ALLELES);
    let combos: u64 = organism::LOCUS_ALLELES.iter().map(|&n| n.max(1) as u64).product();
    println!("  distinct discrete genomes reachable, over every species in the engine: {combos}");
    println!("  ...and what each allele *means* is an engine table, not a species one:");
    println!("     BRANCH_ANGLE {:?}  INTERNODE {:?}", organism::BRANCH_ANGLE_ALLELES, organism::INTERNODE_ALLELES);
    println!("     LEAF_RATE {:?}  LEAF_TRANSPIRATION {:?}", organism::LEAF_RATE_ALLELES, organism::LEAF_TRANSPIRATION_ALLELES);
    println!("     WOOD_DENSITY {:?}", organism::WOOD_DENSITY_ALLELES);
    println!("  so selection sorts among {combos} authored points and cannot find a value nobody wrote down.");
    println!();

    println!("== the fate genome -- the one channel that is open-ended ==");
    println!(
        "  up to {} rules, each (owner, when, becomes, child, lateral, after_metamers) drawn from {} cell types x {} whens",
        organism::MAX_FATES,
        organism::PLANT_CELL_TYPES.len(),
        organism::ALL_FATE_WHENS.len()
    );
    println!("  `Insert` can add a rule the species never had, which is the existence proof the other two channels lack.");
    for (name, id) in &species {
        let g = organism::FateGenome::from_table(w.species.get(*id).fate_table());
        println!("  {name}: founding table {} of {} rules", g.len(), organism::MAX_FATES);
    }
    println!();

    // ---------------------------------------------------------------
    println!("== the parameter genome -- the same table, after ==");
    println!(
        "  {} addressable parameters; an override REPLACES the authored value rather than scaling it,",
        organism::ALL_PARAM_IDS.len()
    );
    println!("  so an authored zero is a starting point rather than a cage. Bound = ParamKind x corpus scale.");
    println!("  shipped rate: param_mutation_chance={} (0.0 = the mechanism is inert; see plant::PARAM_MUTATION_CHANCE)", pixel_physics::sim::plant::param_mutation_chance_seed());
    println!();
    let mut total_addresses = 0usize;
    for (name, id) in &species {
        let addrs = w.species.param_addresses(*id);
        total_addresses += addrs.len();
        println!("  {name}: {} heritable addresses (cell type x parameter x branch order)", addrs.len());
    }
    println!("  ---");
    println!(
        "  pooled: {total_addresses} addresses over {} species, against {} continuous slots x {} species = {} \
         (of which {caged} caged)",
        species.len(),
        organism::GENOTYPE_TRAITS,
        species.len(),
        organism::GENOTYPE_TRAITS * species.len(),
    );
    println!();
    // **Not "the six caged cells" — that label was wrong and the table is
    // the reason.** The continuous genome *addresses* six of these; the rest
    // are authored zeros it never reached at all, because no slot points at
    // them. Root `plastochron` is the one to read: it is `[0]` in **every**
    // shipped species, which is the engine's own statement that no plant may
    // put a node underground — and a node underground is a rhizome, a runner
    // or a sucker. `Reports/plant-reseeding-2026-09-03.md` names it as "one
    // authored number away and no species has taken it".
    println!("  authored zeros -- the ones the continuous genome caged, and the ones it never addressed:");
    println!("  {:<10} {:<20} {:>9} {:>20}  was", "species", "parameter", "authored", "now reachable");
    for (name, id) in &species {
        for (ct, param, tier, label) in [
            (CellType::GrowingTip, organism::ParamId::BranchChance, 0u8, "shoot branch_chance"),
            (CellType::RootTip, organism::ParamId::BranchChance, 0, "root branch_chance"),
            (CellType::GrowingTip, organism::ParamId::Plastochron, 0, "shoot plastochron"),
            (CellType::RootTip, organism::ParamId::Plastochron, 0, "root plastochron"),
            (CellType::MatureBody, organism::ParamId::PipeRatio, 0, "pipe_ratio"),
        ] {
            let Some(v) = w.species.param_in_force(*id, &organism::ParamGenome::default(), (ct, param, tier)) else {
                continue;
            };
            if v != 0.0 {
                continue; // only the caged ones -- the rest are in the table above
            }
            let scale = w.species.param_scale(param);
            let lo = organism::clamp_param(param, -1e6, scale);
            let hi = organism::clamp_param(param, 1e6, scale);
            let addressed = matches!(
                (ct, param),
                (CellType::GrowingTip, organism::ParamId::BranchChance)
                    | (CellType::RootTip, organism::ParamId::BranchChance)
                    | (CellType::GrowingTip, organism::ParamId::Plastochron)
                    | (CellType::MatureBody, organism::ParamId::PipeRatio)
            );
            println!(
                "  {name:<10} {label:<20} {:>9} {:>20}  {}",
                v,
                format!("[{lo:.3}, {hi:.3}]"),
                if addressed { "caged genome slot" } else { "no genome slot at all" }
            );
        }
    }
    println!();

    println!("== what is authored and NOT heritable at all ==");
    for (name, id) in &species {
        let sp = w.species.get(*id);
        println!(
            "  {name}: materials shoot={} root={} leaf={} flower={} fruit={} windfall={} | seed_half_life={} remains_half_life={}",
            sp.shoot_material, sp.root_material, sp.leaf_material, sp.flower_material, sp.fruit_material, sp.windfall_material,
            sp.seed_half_life, sp.remains_half_life
        );
    }
    println!("  A seed copies its parent's species id unchanged, so every line above is fixed for a lineage for ever.");
}

// --------------------------------------------------------------- dynamic --

/// One arm of the widening test.
struct Arm {
    hash: u64,
    cells: u32,
    organisms: usize,
}

fn census(w: &World) -> (u32, usize) {
    let b = w.bounds().expect("the plant scene sets bounds");
    let mut cells = 0u32;
    let mut owners: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let id = w.get(x, y).organism_id();
            if id != 0 {
                cells += 1;
                owners.insert(id);
            }
        }
    }
    (cells, owners.len())
}

fn run_arm(
    species_name: &str,
    founders: usize,
    frames: u64,
    worldseed: Option<u64>,
    patch: Option<(usize, f32)>,
) -> Arm {
    let d = common::PlantScene::default();
    let scene = common::PlantScene {
        trees: founders,
        width: d.width * (founders as i32).max(1) / d.trees as i32,
        species: species_name.to_string(),
        seed: worldseed,
        ..Default::default()
    };
    let mut w = scene.build();
    // **The patch must land before anything germinates.** `PlantScene::build`
    // plants seeds; `seed_genotype` runs at germination and reads the species
    // widths then, so patching after the first tick would give the founders
    // the authored widths and their children the patched ones — which reads
    // as a weak effect rather than as a harness bug. Nothing has ticked yet
    // here. (`ascii`'s own ablation row in `Reports/instruments.md` records
    // this exact failure on the creature side: written after `plant_ant`,
    // both arms came back byte-identical.)
    if let Some((slot, wide)) = patch {
        let id = w.species.id_of(species_name).expect("species is compiled in");
        let ct = variance_owner(slot);
        let mut v = variance_vector(&w, id, ct);
        v[slot] = wide;
        w.species.set_genotype_variance(id, ct, v);
    }
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }
    let (cells, organisms) = census(&w);
    Arm { hash: world_hash(&w), cells, organisms }
}

fn dynamic_arms() {
    let species_name = sarg("species").unwrap_or_else(|| "tree".to_string());
    let founders: usize = arg("founders").unwrap_or(8);
    let frames: u64 = arg("frames").unwrap_or(12_000);
    let wide: f32 = arg("wide").unwrap_or(1.0);
    let worldseed: Option<u64> = arg("worldseed");

    // The static table for this species, so the dynamic verdict can be read
    // beside what the arithmetic already said. This is also where the
    // positive control comes from: a slot the table calls live.
    let probe = common::PlantScene { trees: 0, ..Default::default() }.build();
    let id = probe.species.id_of(&species_name).expect("species is compiled in");
    let shoot = variance_vector(&probe, id, CellType::GrowingTip);
    let root = variance_vector(&probe, id, CellType::RootTip);
    let mut statics: Vec<&'static str> = Vec::new();
    for slot in 0..organism::GENOTYPE_TRAITS {
        let variance = match variance_owner(slot) {
            CellType::RootTip => root[slot],
            _ => shoot[slot],
        };
        let reaches: Vec<Reach> = bases_of(&probe, id, slot).iter().map(|(_, b)| reach(b, variance)).collect();
        statics.push(if reaches.iter().all(|r| r.verdict.starts_with("CAGED")) {
            "CAGED (base 0)"
        } else if reaches.iter().all(|r| r.verdict.starts_with("PINNED")) {
            "PINNED (var 0)"
        } else if reaches.iter().all(|r| r.verdict == "NO CONSUMER") {
            "NO CONSUMER"
        } else {
            "live"
        });
    }

    println!(
        "genome_reach: mode=grow species={species_name} founders={founders} frames={frames} wide={wide} worldseed={:?}",
        worldseed
    );
    println!(
        "  arm = the same scene with slot s's genotype_variance set to {wide}; everything else identical, no extra draws consumed."
    );

    let base = run_arm(&species_name, founders, frames, worldseed, None);
    println!("  baseline: hash {:#018x}  organism cells {}  organisms {}", base.hash, base.cells, base.organisms);
    if base.cells == 0 {
        println!("REFUSING: the baseline grew nothing, so every arm would read identical and the page would be a row of false nulls.");
        std::process::exit(2);
    }

    println!();
    println!("  {:>2} {:<7} {:>16} {:>18} {:>8} {:>6}  moved?", "#", "slot", "static verdict", "hash", "cells", "orgs");
    let mut moved = [false; organism::GENOTYPE_TRAITS];
    for slot in 0..organism::GENOTYPE_TRAITS {
        let a = run_arm(&species_name, founders, frames, worldseed, Some((slot, wide)));
        moved[slot] = a.hash != base.hash;
        println!(
            "  {:>2} {:<7} {:>16} {:#018x} {:>8} {:>6}  {}",
            slot,
            SLOT_NAMES[slot],
            statics[slot],
            a.hash,
            a.cells,
            a.organisms,
            if moved[slot] { "MOVED" } else { "identical" }
        );
    }

    // **The positive control, printed and then asserted.** A slot the static
    // table calls live must move the world when its width goes to `wide`;
    // if none does, this harness is measuring itself and every null above is
    // void. `strain` (slot 9, no consumer) is the built-in negative control
    // and must NOT move.
    println!();
    let live_slots: Vec<usize> = (0..organism::GENOTYPE_TRAITS).filter(|&s| statics[s] == "live").collect();
    let control_moved: Vec<usize> = live_slots.iter().cloned().filter(|&s| moved[s]).collect();
    println!(
        "  positive control -- slots the static table calls live: {:?}; of those, moved: {:?}",
        live_slots.iter().map(|&s| SLOT_NAMES[s]).collect::<Vec<_>>(),
        control_moved.iter().map(|&s| SLOT_NAMES[s]).collect::<Vec<_>>()
    );
    println!("  negative control -- slot 9 `strain` has no consumer: {}", if moved[9] { "MOVED (BUG)" } else { "identical, as it must be" });
    assert!(
        !control_moved.is_empty(),
        "positive control failed: no slot the static table calls live moved the world. \
         The harness is not reaching the founders' genomes, so every `identical` above is void."
    );
    assert!(!moved[9], "negative control failed: slot 9 has no consumer and must not be able to move the world.");

    let caged_and_still: Vec<&str> =
        (0..organism::GENOTYPE_TRAITS).filter(|&s| statics[s].starts_with("CAGED") && !moved[s]).map(|s| SLOT_NAMES[s]).collect();
    println!("  slots the arithmetic cages AND the world confirms dead: {caged_and_still:?}");
}

// ------------------------------------------------ clonal spread: census --

/// The smallest above-ground clump `emergent_clumps` counts as a stem. A shed
/// leaf keeps its `organism_id`, so counting one-cell components would put
/// both arms in the dozens and separate nothing.
const MIN_CLUMP: usize = 3;

/// **Build a sucker by hand, so the census has the exact thing it is looking
/// for.** Returns the column, the anchor row and the organism it hangs off.
///
/// The sensitivity half of `CLAUDE.md`'s *ask what your number counts*: a
/// census reporting "every plant has exactly one stem" reads the same whether
/// that is the answer or whether the census cannot see a second one. This is
/// the control that tells them apart, and it runs on the same world the
/// numbers came from, after they are taken.
///
/// **It has to be *anchored*, and the first version was not.** Stamping a
/// clump into open air proved only that `emergent_clumps` can count a
/// component — not the claim, because the census specifically declines
/// unanchored components as debris. A control the mechanism under test is
/// designed to reject proves nothing about it. So this grafts a column of
/// shoot tissue rising out of one of the plant's own below-ground cells: a
/// sucker, built by hand.
///
/// **And it clears its own site rather than hunting for a clear one, which is
/// the difference between a control that runs and a control that is skipped
/// exactly where it is most needed.** Searching for a column already free of
/// tissue failed outright on the densest bed of nine — 47 plants, no root with
/// five clear rows above it — and a control that cannot be built in a crowded
/// world is a control that certifies only the empty ones. Clearing three
/// columns first always succeeds; the caller re-takes its baseline *after* the
/// clear, so whatever the clear removed is in both terms of the difference and
/// cancels.
fn clear_graft_site(w: &mut World, ground: i32) -> Option<(i32, i32, u16)> {
    let b = w.bounds()?;
    let top = ground - 5;
    for y in ground..=b.max_y {
        for x in b.min_x + 1..b.max_x {
            let id = w.get(x, y).organism_id();
            if id == 0 {
                continue;
            }
            // Three columns wide: the graft's own, plus its neighbours, so it
            // cannot end up 8-adjacent to tissue of the same organism and
            // merge into a clump that already exists — which would register
            // as zero and read as a blind census.
            for yy in top..y {
                for dx in -1..=1 {
                    w.set(x + dx, yy, pixel_physics::sim::cell::Cell::EMPTY);
                }
            }
            return Some((x, y, id));
        }
    }
    None
}

/// Fill the cleared column with the organism's own tissue, from four rows
/// above the ground line down to its anchor.
///
/// It copies a live cell rather than constructing one, so the probe carries
/// whatever material, shade and packed cell type the species actually grows —
/// a hand-built cell could differ in exactly the field the census filters on,
/// and then the control would prove the census can see something that never
/// occurs.
fn graft_sucker(w: &mut World, ground: i32, site: (i32, i32, u16)) -> bool {
    let (x, y, id) = site;
    let Some(b) = w.bounds() else { return false };
    let mut donor = None;
    'donor: for dy in b.min_y..=b.max_y.min(ground - 1) {
        for dx in b.min_x..=b.max_x {
            let c = w.get(dx, dy);
            if c.organism_id() != 0
                && matches!(
                    organism::cell_type(c.aux()),
                    Some(CellType::GrowingTip | CellType::MatureBody | CellType::Leaf | CellType::DormantBud)
                )
            {
                donor = Some(c);
                break 'donor;
            }
        }
    }
    let Some(donor) = donor else { return false };
    // Whose the donor cell was does not matter -- it is re-stamped with the
    // anchor's id, and the graft must belong to the organism it hangs off or
    // the anchor test correctly refuses it.
    let graft = donor.with_organism_id(id);
    for yy in (ground - 4)..y {
        w.set(x, yy, graft);
    }
    true
}

/// **How wide does each plant stand?** — the number that closes
/// `emergent_clumps`' one loophole.
///
/// A sucker that surfaces *inside* its parent's own crown is 8-adjacent to
/// the crown's tissue, so it merges into the same component and the stem
/// count cannot see it. That is arguably the right answer to the question
/// being asked — *does the plant occupy new ground* — but it is the wrong
/// answer to *did a shoot come up at all*, and reporting one number for both
/// would hide the difference.
///
/// Width cannot be fooled that way: a shoot surfacing anywhere its parent's
/// crown does not already reach makes the plant wider, merged or not. So the
/// pair reads: **stems** says whether the plant broke ground somewhere new,
/// **width** says whether anything surfaced at all. Both flat means nothing
/// came up.
fn above_ground_width(w: &World, ground: i32) -> (f32, i32, usize) {
    use std::collections::BTreeMap;
    let mut span: BTreeMap<u16, (i32, i32)> = BTreeMap::new();
    let Some(b) = w.bounds() else { return (0.0, 0, 0) };
    for y in b.min_y..=b.max_y.min(ground - 1) {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if c.organism_id() == 0
                || !matches!(
                    organism::cell_type(c.aux()),
                    Some(CellType::GrowingTip | CellType::MatureBody | CellType::Leaf | CellType::DormantBud)
                )
            {
                continue;
            }
            let e = span.entry(c.organism_id()).or_insert((x, x));
            e.0 = e.0.min(x);
            e.1 = e.1.max(x);
        }
    }
    let widths: Vec<i32> = span.values().map(|(lo, hi)| hi - lo + 1).collect();
    let n = widths.len();
    let mean = if n == 0 { 0.0 } else { widths.iter().sum::<i32>() as f32 / n as f32 };
    (mean, widths.iter().copied().max().unwrap_or(0), n)
}

/// **How many separate places does this organism break the surface?** — the
/// discriminator for *did a sucker become a plant*, as distinct from *did a
/// sucker get launched*.
///
/// `World::root_shoots_launched` counts the event at the moment a lateral
/// comes off a `RootTip`. It says nothing about what became of it, and the
/// section of `Reports/plant-engine-rethink-2026-09-03.md` that introduced it
/// is also the section showing that a *depth* census of this bed cannot
/// answer the follow-up: the plant bed buries its own collars at up to nine
/// rows, so shoot tissue below ground is a burial as often as it is a sucker.
///
/// **So ask the question a player would ask instead: how many stems come out
/// of the ground?** One organism, one collar, is what every shipped species
/// produces — the crown is 8-connected to the collar through the stem, so all
/// of its above-ground tissue is a single component. A sucker that emerged is
/// a *second* component: connected to the parent only underground, through
/// root tissue this walk does not cross. The count is therefore
/// `1 + (suckers that reached daylight)`, and it does not care where the
/// ground line ended up, because a buried collar still leaves exactly one
/// above-ground clump.
///
/// **Returned per organism as `(clumps, stems)`, and the second number is the
/// one to read.** The first version returned only the first, and its own run
/// refuted it: on world seed 1 the treated arm reported one plant with
/// **seven** above-ground clumps while `root_shoots_launched` was **zero** —
/// which no sucker can explain. Detached tissue keeps its `organism_id`: a
/// shed leaf cluster, or a crown severed and fallen, is a component of three
/// or more cells sitting in the air, and raising `min_cells` cannot separate
/// it from a stem because a real sucker is small when it first emerges.
///
/// So a clump is a **stem** only if it is anchored — some cell of it is
/// 8-adjacent to a cell of the *same organism* at or below the ground line.
/// A stem that came out of the ground has that by construction, whether its
/// collar is the original one or a root running sideways; debris in the air
/// does not, and neither does a fallen crown, unless it happens to land on
/// its own roots. Both numbers are printed, because `clumps - stems` is a
/// real quantity about this bed (how much detached tissue is standing around)
/// and it was invisible until this arm went looking for something else.
///
/// 8-connected, because `Grow` places at 8 neighbours (`CLAUDE.md`: *a
/// traversal must use the same neighbourhood the writer used*).
fn emergent_clumps(
    w: &World,
    ground: i32,
    min_cells: usize,
) -> std::collections::BTreeMap<u16, (usize, usize)> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut out: BTreeMap<u16, (usize, usize)> = BTreeMap::new();
    let Some(b) = w.bounds() else { return out };
    let shoot = |c: pixel_physics::sim::cell::Cell| {
        matches!(
            organism::cell_type(c.aux()),
            Some(CellType::GrowingTip | CellType::MatureBody | CellType::Leaf | CellType::DormantBud)
        )
    };
    let mut owner: BTreeMap<(i32, i32), u16> = BTreeMap::new();
    for y in b.min_y..=b.max_y.min(ground - 1) {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            let id = c.organism_id();
            // Shoot tissue only. `RootTip` is excluded deliberately and it is
            // the whole mechanism: it is the underground link that makes two
            // emergences one organism, so walking it would merge them back
            // into one component and every plant would read 1 whatever
            // happened.
            if id != 0 && shoot(c) {
                owner.insert((x, y), id);
            }
        }
    }
    let mut seen: BTreeSet<(i32, i32)> = BTreeSet::new();
    let starts: Vec<((i32, i32), u16)> = owner.iter().map(|(&p, &id)| (p, id)).collect();
    for (start, id) in starts {
        if !seen.insert(start) {
            continue;
        }
        let mut stack = vec![start];
        let (mut n, mut anchored) = (0usize, false);
        while let Some((x, y)) = stack.pop() {
            n += 1;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (qx, qy) = (x + dx, y + dy);
                    // At or below the ground line: not in `owner`, so it can
                    // only be reached this way. Any cell of this organism will
                    // do -- a sucker's link down is `RootTip`, an ordinary
                    // collar's is its own buried stem, and the census must not
                    // care which.
                    if qy >= ground {
                        anchored |= w.get(qx, qy).organism_id() == id;
                        continue;
                    }
                    if owner.get(&(qx, qy)) == Some(&id) && seen.insert((qx, qy)) {
                        stack.push((qx, qy));
                    }
                }
            }
        }
        if n >= min_cells {
            let e = out.entry(id).or_default();
            e.0 += 1;
            if anchored {
                e.1 += 1;
            }
        }
    }
    out
}

// ------------------------------------------------------------- rhizome --

/// **Is a clonal growth form reachable, or is there a mechanism missing?**
///
/// `plant-reseeding-2026-09-03.md` records that a `RootTip` fate could set
/// `child: GrowingTip` and put a shoot up from underground, and that *"every
/// shipped species gives roots `plastochron: [0]`, which disables nodes
/// outright"* — so rhizomes, runners and suckers are *"one authored number
/// away and no species has taken it"*. The static census (§1.5 of the report)
/// sharpens that: root `plastochron` is authored `0` in **every** species and
/// **no genome slot addresses it**, so it is not merely untaken — it was
/// unreachable by evolution until `ParamGenome` landed.
///
/// This arm asks the question that claim depends on and that nobody has run:
/// **if a root does get a node, does a shoot actually come up?** It is a
/// reachability question, not a design proposal — the answer decides whether
/// clonal spread is a species-file line or a missing mechanism, and those
/// need completely different work.
///
/// **It registers a runtime species and touches no shipped file.**
/// `PlantScene::species_ron` puts a variant of `herb` in the registry before
/// the bed plants anything, so `herb.ron` is unchanged and the shipped game
/// is unaffected. The arms differ in exactly two lines of that variant.
///
/// ```text
/// cargo run --release --example genome_reach -- rhizome=1 frames=20000
/// ```
///
/// **The readout is two numbers, and the first version of this harness had
/// only a third one that does not work.** Counting shoot cells *below the
/// ground line* was tried and is void: this bed buries its own collars at up
/// to nine rows, so a buried stem and a sucker read identically, and over
/// three world seeds the control read 1/3/2 against the treated arm's 2/2/2 —
/// a clean null wearing a discriminator that does not discriminate. The
/// depth columns are still printed, and they are printed as the thing that
/// failed. What replaced them:
///
/// - **`World::root_shoots_launched`** — the event, counted at the one site
///   where a lateral comes off a `RootTip`. *Did it fire at all.*
/// - **`emergent_clumps`** — 8-connected above-ground clumps per organism.
///   *Did what fired become a stem.* One collar is one clump however deeply
///   it is buried, so this is immune to the trap above.
///
/// - **`above_ground_width`** — how wide each plant stands. *Did anything
///   surface at all*, including a sucker that came up inside its parent's own
///   crown and merged into its clump.
///
/// Read them in that order: each is only interesting where the one before it
/// is non-zero.
fn rhizome() {
    let frames: u64 = arg("frames").unwrap_or(20_000);
    let founders: usize = arg("founders").unwrap_or(4);
    let node: u8 = arg("node").unwrap_or(6);
    // How far below the ground line a shoot cell has to be before it counts
    // as coming off a root rather than off a buried collar.
    let deep_at: i32 = arg("deep").unwrap_or(4);
    let worldseed: Option<u64> = arg("worldseed");

    // The two arms, as a `herb` variant under another name so the registry
    // keeps the shipped species alongside it.
    let base = include_str!("../assets/species/herb.ron");
    let control = base.replace("name: \"herb\"", "name: \"herb_control\"");
    // The root's `Grow` is the second `plastochron: [0]`-carrying block; edit
    // by the whole `(RootTip, [` section so a shoot line cannot be hit.
    let Some(root_at) = base.find("(RootTip, [\n            Absorb") else {
        println!("REFUSING: herb.ron's RootTip block did not match -- the edit would have hit nothing, \\
                  and an arm whose edit matched nothing reads as `the mechanism does not work`.");
        std::process::exit(2);
    };
    let (head, tail) = base.split_at(root_at);
    let patched_tail = tail.replacen("plastochron: [0],", &format!("plastochron: [{node}],"), 1);
    if patched_tail == tail {
        println!("REFUSING: the root plastochron edit matched nothing.");
        std::process::exit(2);
    }
    // ...and a `RootTip` fate that makes a node put a shoot out sideways.
    let mut treated = format!("{head}{patched_tail}").replace("name: \"herb\"", "name: \"herb_rhizome\"");
    let root_fates = "(RootTip, [\n            (when: Grew,  becomes: MatureBody, child: Some(RootTip), lateral: Some(RootTip)),";
    let with_node = "(RootTip, [\n            (when: Node,  becomes: MatureBody, child: Some(RootTip), lateral: Some(GrowingTip)),\n            (when: Grew,  becomes: MatureBody, child: Some(RootTip), lateral: Some(RootTip)),";
    if !treated.contains(root_fates) {
        println!("REFUSING: herb.ron's RootTip fate block did not match.");
        std::process::exit(2);
    }
    treated = treated.replacen(root_fates, with_node, 1);

    println!("genome_reach: mode=rhizome frames={frames} founders={founders} root_plastochron={node} worldseed={worldseed:?}");
    for (label, ron, name) in
        [("control (shipped herb)", control, "herb_control"), ("root node + lateral shoot", treated, "herb_rhizome")]
    {
        let d = common::PlantScene::default();
        let scene = common::PlantScene {
            trees: founders,
            width: d.width * (founders as i32).max(1) / d.trees as i32,
            species: name.to_string(),
            species_ron: Some(ron),
            seed: worldseed,
            ..Default::default()
        };
        let ground = scene.ground_y;
        let mut w = scene.build();
        for _ in 0..frames {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }
        // Shoot tissue standing below the ground line, and — the column that
        // makes it a sucker rather than a buried stem — shoot tissue that
        // reaches back up into open air somewhere other than its own collar.
        let b = w.bounds().expect("bounds");
        let (mut shallow, mut deep, mut cells) = (0u32, 0u32, 0u32);
        let mut depth_hist: std::collections::BTreeMap<i32, u32> = std::collections::BTreeMap::new();
        let mut owners: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                if c.organism_id() == 0 {
                    continue;
                }
                cells += 1;
                owners.insert(c.organism_id());
                let Some(ct) = organism::cell_type(c.aux()) else { continue };
                if !matches!(ct, CellType::GrowingTip | CellType::Leaf | CellType::DormantBud) {
                    continue;
                }
                let d = y - ground;
                if d >= 1 {
                    *depth_hist.entry(d).or_default() += 1;
                    // **Depth is the discriminator, and the first version of
                    // this arm did not have one.** It counted shoot tissue
                    // below the ground line with open sky a few rows above,
                    // which reads as "a shoot that broke the surface" and is
                    // also what a plant whose collar got buried by a cell or
                    // two of moving soil looks like. Over three world seeds
                    // the control read 1 / 3 / 2 of those and the treated arm
                    // read 2 / 2 / 2 -- a clean null wearing a discriminator
                    // that does not discriminate. A sucker comes off a root,
                    // and roots are rows down.
                    if d >= deep_at {
                        deep += 1;
                    } else {
                        shallow += 1;
                    }
                }
            }
        }
        // **And then the follow-up question the counter cannot answer: did
        // any of those launches become a stem of its own?** See
        // `emergent_clumps` -- one organism with two above-ground clumps has
        // broken the surface in two places, which is what a sucker looks like
        // from outside and is not what a burial looks like.
        let clumps = emergent_clumps(&w, ground, MIN_CLUMP);
        let multi = clumps.values().filter(|v| v.1 > 1).count();
        let extra: usize = clumps.values().map(|v| v.1.saturating_sub(1)).sum();
        let debris: usize = clumps.values().map(|v| v.0 - v.1).sum();
        let standing = clumps.values().filter(|v| v.1 > 0).count();
        println!(
            "  {label:<26} SHOOTS LAUNCHED OFF A ROOT {:<6} | organisms {:<5} cells {cells:<7} \
             shoot tissue 1-{} rows down {shallow:<5} {}+ rows down {deep:<5} depths {:?}",
            w.root_shoots_launched,
            owners.len(),
            deep_at - 1,
            deep_at,
            depth_hist
        );
        let (wmean, wmax, wn) = above_ground_width(&w, ground);
        println!(
            "  {:<26} SECOND STEMS {multi:<6} | plants rooted above ground {standing:<4} \
             extra stems {extra:<4} unanchored clumps (debris) {debris:<4} stems per plant {:?}",
            "",
            clumps.values().map(|v| v.1).filter(|&n| n > 1).collect::<Vec<_>>()
        );
        println!(
            "  {:<26} width per plant: mean {wmean:.2} max {wmax} over {wn} plants \
             -- flat width AND flat stems together mean nothing surfaced at all",
            ""
        );
        // **The positive control, on this exact world, after its numbers are
        // taken** -- `CLAUDE.md`, *run the positive control*: a null here is
        // the shape that hides, because "no plant has a second stem" and "the
        // census cannot see a second stem" are the same printout. Grafting a
        // shoot out of a plant's own buried tissue builds the exact thing the
        // census is looking for, and must move STEMS by one; if it does not,
        // the row above is void rather than negative.
        // A control that could not be built must not read as a control that
        // found nothing -- they print the same `0` and mean opposite things.
        let site = clear_graft_site(&mut w, ground).expect(
            "the positive control could not be built: this bed has no below-ground organism cell at \
             all, so the SECOND STEMS row above is unverified",
        );
        // Baseline re-taken **after** the clear and before the graft, so the
        // tissue the clear removed sits in both terms and cancels. `clumps`
        // above is the measurement; this is the control's own reference.
        let before = emergent_clumps(&w, ground, MIN_CLUMP);
        assert!(graft_sucker(&mut w, ground, site), "no donor tissue to build the control from");
        let after = emergent_clumps(&w, ground, MIN_CLUMP);
        let moved: i64 = after.values().map(|v| v.1 as i64).sum::<i64>()
            - before.values().map(|v| v.1 as i64).sum::<i64>();
        println!(
            "  {:<26} control: grafted a shoot onto organism {} at column {} -> STEMS moved by {moved} (must be 1)",
            "", site.2, site.0
        );
        assert_eq!(moved, 1, "emergent_clumps is blind: a hand-built sucker did not register");
    }
    // **The discriminator is `reaching daylight`, not `below ground`, and the
    // first run is what settled that.** The control reads a handful of shoot
    // cells below the ground line — seedlings that germinated into a hollow
    // and got buried, which is a real thing this bed does and has nothing to
    // do with a root node. Only the emergent column separates a sucker from a
    // buried stem, and it is 0 in the control by construction: nothing in the
    // shipped species can put a shoot below ground *and* drive it back up.
    println!(
        "  -- read the FIRST column. `World::root_shoots_launched` counts the event; the depth columns are \
         a census and this bed buries collars in both arms at up to nine rows, so they cannot separate a \
         sucker from a burial. The control does NOT read 0: no root reaches a Node fate, but a fate \
         mutation can point the root's Grew rule's lateral at a GrowingTip, and then every root step \
         launches one. `PIXEL_PHYSICS_FATE_MUTATION_CHANCE=0` is the control that isolates that, and \
         it takes the control arm to 0 on every seed."
    );
}
