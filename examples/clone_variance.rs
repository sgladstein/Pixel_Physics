//! **How much of the difference between two plants is their genome, and how
//! much is where they happened to stand?** — the noise floor under every
//! plant result this project has published.
//!
//! Owner, 2026-09-03: *"within the current engine clones of the same plant
//! end up growing/looking very different from one another which makes it much
//! harder to identify when growth patterns do change."*
//!
//! The scatter itself is not news — `plant-heritability-survey-design-
//! 2026-08-27.md` records **31 to 153 cells and 90 / 438 / 1,435 root cells
//! for identical genomes** — but it is recorded there as a *method note*
//! ("do not quote a stand median"), never as a defect. The consequence
//! nobody had drawn is the one that matters: **if developmental scatter is
//! larger than genetic difference, then selection cannot see the genome
//! either.** A population would then re-roll its variety every generation
//! instead of inheriting it, which explains an invisible architectural lever
//! at least as well as composition does.
//!
//! So the quantity this prints is broad-sense heritability, one descriptor
//! at a time:
//!
//! ```text
//! H2 = 1 - Var(clone arm) / Var(population arm)
//! ```
//!
//! — where the clone arm is genetically identical individuals standing in the
//! same bed, so its whole spread is position plus developmental noise, and
//! the population arm is the shipped stand. **H2 near zero means the genome
//! is invisible in that descriptor**, whatever the genome can express.
//!
//! ```text
//! cargo run --release --example clone_variance -- species=herb founders=16 frames=16000
//! cargo run --release --example clone_variance -- species=herb shift=1     # the one-cell-over arm
//! ```
//!
//! ## Three arms, and the third is the one that names the mechanism
//!
//! - **`pop`** — the shipped stand: every founder its own genome.
//! - **`clone`** — every founder carrying founder 0's genome, written through
//!   `World::set_organism_genotype` (which also sets `inherited`, or
//!   `seed_genotype` redraws the whole thing at germination and the arm
//!   silently becomes the control).
//! - **`spread`** — the positive control for the estimator, and it is
//!   mandatory. Half the founders get every continuous draw at `-1` and half
//!   at `+1`, i.e. the two most distant genomes the engine can express. If
//!   `H2` does not go high here, the descriptors are blind and every number
//!   above them is void — `CLAUDE.md`'s sensitivity half, which this repo has
//!   six recorded occurrences of skipping.
//!
//! And separately, `shift=1`: **one plant, alone, in an identical bed, moved
//! one column at a time.** No neighbours, no competition, no genetic
//! difference — so whatever moves is the world one cell over plus the draw
//! stream. It exists because `plant.rs`'s growth RNG is
//! `rng::stream(organism_id, cx, cy, frame)`: **the organism's own id is part
//! of the key**, so two genetically identical plants in physically identical
//! spots are still different plants. That is not environmental variation and
//! it is not heritable; it is a per-individual random seed, and it is the one
//! source of scatter that a player has no way to read as *anything*.

mod common;

use pixel_physics::sim::organism;
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
}

fn sarg(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// One plant, as the numbers a person would use to say two plants look
/// different.
///
/// **Size is carried and is not the headline**, for the reason
/// `plant-heritability-survey-design-2026-08-27.md` §3 gives: every
/// discriminating result in this project's record so far is a magnitude, and
/// the owner's verdict three separate times is *"the biggest differences are
/// still size and color"*. A ranking on size is guaranteed a positive result
/// and answers nothing. The shape columns are the ones to read.
#[derive(Clone, Copy, Default)]
struct Shape {
    cells: f32,
    height: f32,
    width: f32,
    /// height / width — the single number closest to "is this a spire or a
    /// dome".
    slenderness: f32,
    /// leaf cells as a fraction of the whole plant.
    foliage_share: f32,
    /// root cells as a fraction of the whole plant.
    root_share: f32,
    /// mean leaf row as a fraction of the plant's own height, 0 at the collar
    /// and 1 at the apex.
    foliage_centre: f32,
}

const COLUMNS: [&str; 7] = ["cells", "height", "width", "slender", "foliage%", "root%", "folcentre"];

impl Shape {
    fn get(&self, i: usize) -> f32 {
        match i {
            0 => self.cells,
            1 => self.height,
            2 => self.width,
            3 => self.slenderness,
            4 => self.foliage_share,
            5 => self.root_share,
            _ => self.foliage_centre,
        }
    }
}

/// Census one organism off the grid.
///
/// Off the grid rather than off `OrganismState`, the way `plant_probe` and
/// `genome_drift` both do it: the cell **type** lives in the grid cell's
/// `aux`, and `OrganismCell` carries resources and support distance without
/// it.
fn shapes(w: &World, ids: &[(u16, i32, i32)]) -> Vec<Shape> {
    let Some(b) = w.bounds() else { return Vec::new() };
    let mut acc: std::collections::HashMap<u16, (u32, u32, i32, i32, i32, i32, i64)> = std::collections::HashMap::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            let id = c.organism_id();
            if id == 0 || !ids.iter().any(|&(i, _, _)| i == id) {
                continue;
            }
            let Some(ct) = organism::cell_type(c.aux()) else { continue };
            // Seeds are excluded from the body: a plant is not taller for
            // carrying seeds, and `herb` carries a great many.
            if ct == organism::CellType::Seed {
                continue;
            }
            let e = acc.entry(id).or_insert((0, 0, i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0));
            e.0 += 1;
            if ct == organism::CellType::Leaf {
                e.1 += 1;
                e.6 += y as i64;
            }
            e.2 = e.2.min(y);
            e.3 = e.3.max(y);
            e.4 = e.4.min(x);
            e.5 = e.5.max(x);
        }
    }
    // Root cells are counted from the organism's own tally rather than from
    // the grid: `MatureBody` is what a root tip becomes, so the grid cannot
    // tell a thickened root from a thickened stem, and the state carries the
    // count the engine itself keeps.
    let mut out = Vec::new();
    for &(id, _, _) in ids {
        let Some(&(cells, leaves, min_y, max_y, min_x, max_x, leaf_y_sum)) = acc.get(&id) else { continue };
        if cells < 20 {
            continue; // not established; see plant_probe's own threshold
        }
        let roots = w.organism(id).map_or(0, |s| s.root_cells) as f32;
        let height = (max_y - min_y + 1) as f32;
        let width = (max_x - min_x + 1) as f32;
        let leaf_centre =
            if leaves > 0 { 1.0 - ((leaf_y_sum as f32 / leaves as f32) - min_y as f32) / height.max(1.0) } else { 0.0 };
        out.push(Shape {
            cells: cells as f32,
            height,
            width,
            slenderness: height / width.max(1.0),
            foliage_share: leaves as f32 / cells as f32,
            root_share: roots / cells as f32,
            foliage_centre: leaf_centre,
        });
    }
    out
}

fn mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f32>() / v.len() as f32
}

fn variance(v: &[f32]) -> f32 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / (v.len() - 1) as f32
}

/// Coefficient of variation — the scale-free spread, so `cells` and
/// `foliage share` can sit in one table without one drowning the other.
fn cv(v: &[f32]) -> f32 {
    let m = mean(v);
    if m.abs() < 1e-6 {
        return 0.0;
    }
    variance(v).sqrt() / m.abs()
}

fn median(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s[s.len() / 2]
}

/// Which arm the founders' genomes come from.
#[derive(Clone, Copy, PartialEq)]
enum Arm {
    /// The shipped stand.
    Pop,
    /// Every founder carrying founder 0's genome.
    Clone,
    /// Half at every draw `-1`, half at every draw `+1` — the estimator's
    /// positive control.
    Spread,
}

fn build(species: &str, founders: usize, worldseed: Option<u64>, width_override: Option<i32>) -> (World, Vec<(u16, i32, i32)>) {
    let d = common::PlantScene::default();
    let scene = common::PlantScene {
        trees: founders,
        width: width_override.unwrap_or(d.width * (founders as i32).max(1) / d.trees as i32),
        species: species.to_string(),
        seed: worldseed,
        ..Default::default()
    };
    let w = scene.build();
    // The founders and where each one is standing, in a deterministic order:
    // the scene plants them left to right, so scanning the grid in column
    // order names them the same way every run.
    //
    // **The coordinate is carried because a founder does not have a genome
    // yet, and finding that out cost a whole sweep.** `PlantScene::build`
    // plants through `World::plant_tree_species`, which allocates the
    // organism and writes the cell and **never calls `plant::seed_genotype`**
    // -- only `World::plant_tree` does. So at frame 0 every founder holds
    // `genotype_draws = [0.0; N]`, the species mean, and they are all
    // identical: the first version of this harness cloned founder `ref` and
    // produced **byte-identical output at ref=0, 1 and 5**, which is this
    // repo's standing tell for a knob that was never connected. The arm was
    // not wrong so much as vacuous -- a clone stand of the species mean,
    // reported as a clone of a sampled individual.
    let mut ids: Vec<(u16, i32, i32)> = Vec::new();
    if let Some(b) = w.bounds() {
        for x in b.min_x..=b.max_x {
            for y in b.min_y..=b.max_y {
                let id = w.get(x, y).organism_id();
                if id != 0 && !ids.iter().any(|&(i, _, _)| i == id) {
                    ids.push((id, x, y));
                }
            }
        }
    }
    (w, ids)
}

fn apply_arm(w: &mut World, ids: &[(u16, i32, i32)], arm: Arm, reference: usize) {
    match arm {
        Arm::Pop => {}
        Arm::Clone => {
            // **Which founder is cloned is an argument, not always the
            // first.** One genome is one sample: a founder that happens to
            // sit near a threshold in the economy makes every clone of it
            // sit there too, so a single reference genome can report a
            // clone stand as *more* variable than a mixed one and read as a
            // finding. `ref=` sweeps it, and it is only a real knob because
            // of the line below.
            let Some(&(src, sx, sy)) = ids.get(reference.min(ids.len().saturating_sub(1))) else { return };
            // **Found a real genome first.** See `build`: a founder holds the
            // species mean until something calls this, so without it every
            // `ref=` clones the same all-zero draw and the argument is inert.
            pixel_physics::sim::plant::seed_genotype(w, src, sx, sy);
            let Some((draws, alleles, params)) = w.organism_genotype(src) else { return };
            for &(id, _, _) in ids {
                w.set_organism_genotype(id, draws, alleles, params);
            }
        }
        Arm::Spread => {
            let Some((_, _, params)) = ids.first().and_then(|&(id, _, _)| w.organism_genotype(id)) else { return };
            // **The discrete loci go to their extremes too, not just the
            // continuous draws.** The point of this arm is *the widest
            // contrast the engine can express*; half of that vocabulary is
            // the six alleles, and a control that held them fixed would be
            // testing only whether the descriptors can see a multiplier.
            let mut low = [0u8; organism::DISCRETE_LOCI];
            let mut high = [0u8; organism::DISCRETE_LOCI];
            for (locus, h) in high.iter_mut().enumerate() {
                *h = organism::LOCUS_ALLELES[locus].saturating_sub(1);
                low[locus] = 0;
            }
            for (i, &(id, _, _)) in ids.iter().enumerate() {
                let (v, a) = if i % 2 == 0 { (-1.0, low) } else { (1.0, high) };
                w.set_organism_genotype(id, [v; organism::GENOTYPE_TRAITS], a, params);
            }
        }
    }
}

fn run(species: &str, founders: usize, frames: u64, worldseed: Option<u64>, arm: Arm, reference: usize) -> Vec<Shape> {
    let (mut w, ids) = build(species, founders, worldseed, None);
    apply_arm(&mut w, &ids, arm, reference);
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }
    shapes(&w, &ids)
}

fn table(label: &str, arms: &[Shape]) {
    print!("  {label:<9} n={:<4}", arms.len());
    for i in 0..COLUMNS.len() {
        let col: Vec<f32> = arms.iter().map(|s| s.get(i)).collect();
        print!("  {:>9}", format!("{:.3}", median(&col)));
    }
    println!();
    print!("  {:<9} CV   ", "");
    for i in 0..COLUMNS.len() {
        let col: Vec<f32> = arms.iter().map(|s| s.get(i)).collect();
        print!("  {:>9}", format!("{:.3}", cv(&col)));
    }
    println!();
}

fn main() {
    let species = sarg("species").unwrap_or_else(|| "herb".to_string());
    let founders: usize = arg("founders").unwrap_or(16);
    let frames: u64 = arg("frames").unwrap_or(16_000);
    let worldseed: Option<u64> = arg("worldseed");
    let shift: u32 = arg("shift").unwrap_or(0);
    let seeds: usize = arg("seeds").unwrap_or(1);
    let reference: usize = arg("ref").unwrap_or(0);

    println!(
        "clone_variance: species={species} founders={founders} frames={frames} worldseed={worldseed:?} shift={shift} \
         param_mutation_chance={}",
        pixel_physics::sim::plant::param_mutation_chance_seed()
    );

    if shift > 0 {
        one_cell_over(&species, founders, frames, worldseed);
        return;
    }

    // **Variances are pooled *within* seed and then averaged, never pooled
    // across seeds.** A pooled-across-seeds variance carries the
    // between-world difference in both arms, which inflates both and drags
    // every ratio toward 1 -- the same shape as an unremoved oscillator
    // (`CLAUDE.md`). Averaging the within-seed variances is the estimator the
    // question actually asks for: *within one bed, how much of the spread is
    // genome*.
    let mut var_pop = [0.0f32; COLUMNS.len()];
    let mut var_clone = [0.0f32; COLUMNS.len()];
    let mut var_spread = [0.0f32; COLUMNS.len()];
    let mut n_seeds = 0.0f32;
    let (mut all_pop, mut all_clone, mut all_spread): (Vec<Shape>, Vec<Shape>, Vec<Shape>) = (vec![], vec![], vec![]);
    for k in 0..seeds.max(1) {
        let ws = worldseed.map(|w| w + k as u64).or(Some(1 + k as u64));
        let pop = run(&species, founders, frames, ws, Arm::Pop, reference);
        let clones = run(&species, founders, frames, ws, Arm::Clone, reference);
        let spread = run(&species, founders, frames, ws, Arm::Spread, reference);
        println!("\n  --- worldseed {:?}, ref founder {reference} ---", ws);
        print!("  {:<9} {:<5}", "arm", "");
        for c in COLUMNS {
            print!("  {c:>9}");
        }
        println!("   (median, then coefficient of variation)");
        table("pop", &pop);
        table("clone", &clones);
        table("spread", &spread);
        for i in 0..COLUMNS.len() {
            var_pop[i] += variance(&pop.iter().map(|s| s.get(i)).collect::<Vec<_>>());
            var_clone[i] += variance(&clones.iter().map(|s| s.get(i)).collect::<Vec<_>>());
            var_spread[i] += variance(&spread.iter().map(|s| s.get(i)).collect::<Vec<_>>());
        }
        n_seeds += 1.0;
        all_pop.extend(pop);
        all_clone.extend(clones);
        all_spread.extend(spread);
    }
    for i in 0..COLUMNS.len() {
        var_pop[i] /= n_seeds;
        var_clone[i] /= n_seeds;
        var_spread[i] /= n_seeds;
    }

    println!("\n== pooled over {n_seeds} world seed(s), founders={founders} ==");
    println!(
        "  established plants censused: pop {} / clone {} / spread {}",
        all_pop.len(),
        all_clone.len(),
        all_spread.len()
    );
    // **H2 clamped at zero rather than reported negative.** A clone arm whose
    // spread exceeds the population's is sampling noise at this n, not
    // negative heritability, and a negative number in this column invites a
    // reading it cannot support. The raw variance ratio is printed beside it
    // so the clamping is visible rather than silent.
    println!("\n  broad-sense heritability, H2 = 1 - Var(clone)/Var(pop):");
    print!("  {:<15}", "");
    for c in COLUMNS {
        print!("  {c:>9}");
    }
    println!();
    print!("  {:<15}", "Var(clone)/Var(pop)");
    for i in 0..COLUMNS.len() {
        print!("  {:>9}", format!("{:.3}", if var_pop[i] > 0.0 { var_clone[i] / var_pop[i] } else { f32::NAN }));
    }
    println!();
    print!("  {:<15}", "H2");
    for i in 0..COLUMNS.len() {
        let h = if var_pop[i] > 0.0 { (1.0 - var_clone[i] / var_pop[i]).max(0.0) } else { 0.0 };
        print!("  {:>9}", format!("{:.3}", h));
    }
    println!();
    let mut control = [0.0f32; COLUMNS.len()];
    print!("  {:<15}", "H2 (control)");
    for i in 0..COLUMNS.len() {
        control[i] = if var_spread[i] > 0.0 { (1.0 - var_clone[i] / var_spread[i]).max(0.0) } else { 0.0 };
        print!("  {:>9}", format!("{:.3}", control[i]));
    }
    println!();

    // **The estimator's own sensitivity, printed and asserted.** `spread`
    // stands the two most distant genomes the engine can express in one bed;
    // if no descriptor separates them from a clone stand, the descriptor set
    // cannot see a genome at all and every H2 above it is a statement about
    // this harness rather than about the engine.
    let best = control.iter().cloned().fold(0.0f32, f32::max);
    let best_i = control.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
    println!("\n  positive control: the widest contrast the engine can express reaches H2 = {best:.3} on `{}`", COLUMNS[best_i]);
    assert!(
        all_clone.len() >= 3 && all_pop.len() >= 3,
        "fewer than three established plants in an arm -- a scene error, not a result"
    );
    assert!(
        best > 0.2,
        "positive control failed: the widest genetic contrast the engine can express does not separate from a clone \
         stand on ANY descriptor. The descriptor set is blind and every H2 above is void."
    );
}

/// **One plant, alone, in an identical bed, moved one column at a time.**
///
/// The arm that names the mechanism. `plant.rs`'s growth draws come from
/// `rng::stream(organism_id, cx, cy, frame)`, so a plant's development is a pure
/// function of *where it is* and *which slot it got* — there is no
/// per-organism seed that survives being moved. Two consequences a player
/// would not guess: a plant one column over is a different plant, and a plant
/// that germinates into organism slot 7 rather than 6 is a different plant
/// again.
fn one_cell_over(species: &str, n: usize, frames: u64, worldseed: Option<u64>) {
    println!("\n  one founder, alone, same genome, moved one column at a time ({n} positions):");
    print!("  {:<15}", "");
    for c in COLUMNS {
        print!("  {c:>9}");
    }
    println!();
    // A single reference genome, taken from the first bed and written onto
    // every subsequent one, so the only thing that differs between runs is
    // the column the plant stands in.
    let (w0, ids0) = build(species, 1, worldseed, None);
    let Some(&(src, sx, sy)) = ids0.first() else {
        println!("  REFUSING: the reference bed planted nothing.");
        return;
    };
    let mut w0 = w0;
    pixel_physics::sim::plant::seed_genotype(&mut w0, src, sx, sy);
    let Some(reference) = w0.organism_genotype(src) else {
        println!("  REFUSING: the reference founder has no genome.");
        return;
    };
    let mut all: Vec<Shape> = Vec::new();
    for step in 0..n {
        // `PlantScene` centres a single founder, so the column is moved by
        // widening the bed by one -- which keeps the founder's surroundings
        // identical in *kind* while moving its coordinate.
        let d = common::PlantScene::default();
        let (mut w, ids) = build(species, 1, worldseed, Some(d.width + 2 * step as i32));
        for &(id, _, _) in &ids {
            w.set_organism_genotype(id, reference.0, reference.1, reference.2);
        }
        for _ in 0..frames {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }
        let s = shapes(&w, &ids);
        if let Some(s0) = s.first() {
            all.push(*s0);
            print!("  {:<15}", format!("+{step} col"));
            for i in 0..COLUMNS.len() {
                print!("  {:>9}", format!("{:.3}", s0.get(i)));
            }
            println!();
        } else {
            println!("  {:<15}  (did not establish)", format!("+{step} col"));
        }
    }
    print!("  {:<15}", "CV");
    for i in 0..COLUMNS.len() {
        let col: Vec<f32> = all.iter().map(|s| s.get(i)).collect();
        print!("  {:>9}", format!("{:.3}", cv(&col)));
    }
    println!();
    println!(
        "  -- every row above is the SAME genome. Whatever spread this shows is the floor under \
         any claim that two genomes differ."
    );
}
