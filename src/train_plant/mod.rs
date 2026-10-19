//! **The plant trainer: species files found by selection rather than written
//! by hand.** The plant counterpart of the ant trainer (`src/train/`), built on
//! the owner's rulings of 2026-10-19 -- a trained plant is a species file; the
//! trainer may change shape and look; herb first. Design of record:
//! `Reports/plant-trainer-plan-2026-10-19.md` (revision 2, after an
//! independent adversarial review), with the engine review and the brainstorm
//! beside it.
//!
//! A generation scores every genome on the same training layouts in the
//! lab's own bed ([`arena`]), keeps the best quarter, and refills with
//! children ([`genome::mutate`]). **What keeps it honest** is the part this
//! module is mostly for, borrowed from the ant rig:
//!
//! * **the shipped species as a control**, on the same layouts every
//!   generation, so a rising best can be told from easier layouts;
//! * **a monitoring hold-out**, read every generation, and a **final set read
//!   once**, at the end -- the champion is chosen by training score, never by
//!   the hold-out, which would otherwise become training data (review F10);
//! * **the bar is a paired difference**, champion minus shipped per layout,
//!   because the champion is the best of a population of noisy scores;
//! * **teeth before generation 0**: a plant that never grows past a seed, and
//!   one starved of light income, must score below the shipped species, or
//!   the score is blind and the run refuses to start;
//! * **a selection-only null** (`select=random`): the same run with the
//!   parents drawn at random. A trainer that does not beat it has learned
//!   nothing.
//!
//! **The game is never touched.** A child is a new species with a name of
//! its own, registered into its own bed's world only; the shipped species
//! is the control, and staging a champion writes a file under the run's
//! output folder, never `assets/`.

pub mod arena;
pub mod genome;
pub mod score;

use crate::sim::organism::{CellType, SpeciesDef};
use crate::sim::rng::Rng;
use arena::{BedSpec, Census, Layout};
use genome::{Gene, Group};
use rayon::prelude::*;
use score::Constants;
use std::fmt::Write as _;
use std::path::PathBuf;

/// How parents are chosen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Select {
    /// The best quarter by training score.
    Score,
    /// **The null**: a random quarter, at the same compute. The champion is
    /// still reported as the best by training score, so the null carries the
    /// same best-of-many luck; what it lacks is selection that accumulates.
    Random,
}

/// Everything a run is, printed whole as its first line.
#[derive(Clone, Debug)]
pub struct Settings {
    /// The species file the run starts from.
    pub start: PathBuf,
    /// The shipped species the run is measured against (by name, from the
    /// compiled-in registry).
    pub control: String,
    pub groups: Vec<Group>,
    pub pop: usize,
    pub gens: usize,
    pub keep: f32,
    pub layouts: usize,
    pub holdout: usize,
    pub final_set: usize,
    pub sigma: f32,
    pub max_genes: usize,
    pub seed: u64,
    pub select: Select,
    pub teeth: bool,
    pub bed: BedSpec,
    pub constants: Constants,
    pub out: PathBuf,
}

impl Settings {
    pub fn line(&self) -> String {
        format!(
            "plantrain: start={} control={} groups={} pop={} gens={} keep={} layouts={} holdout={} final={} sigma={} genes={} seed={} select={:?} teeth={} width={} frames={} every={} mutation_off={} n_est={} k_seed={} size_floor={} out={}",
            self.start.display(),
            self.control,
            self.groups.iter().map(|g| g.name()).collect::<Vec<_>>().join(","),
            self.pop,
            self.gens,
            self.keep,
            self.layouts,
            self.holdout,
            self.final_set,
            self.sigma,
            self.max_genes,
            self.seed,
            self.select,
            self.teeth,
            self.bed.width,
            self.bed.frames,
            self.bed.every,
            self.bed.mutation_off,
            self.constants.n_est,
            self.constants.k_seed,
            self.constants.size_floor,
            self.out.display()
        )
    }
}

impl Settings {
    /// **Settings a run can be trusted under, refused otherwise.** The layout
    /// seeds of the three sets are `base + seed * stride + i`, so they stay
    /// apart only inside these limits: `seed=1000` would score its hold-out on
    /// `seed=0`'s final set, and 101 training layouts would reuse the next
    /// generation's. A zero count panics further in rather than meaning
    /// anything.
    pub fn check(&self) -> Result<(), String> {
        let limits: [(&str, usize, usize, usize); 6] = [
            ("seed", self.seed as usize, 0, 999),
            ("layouts", self.layouts, 1, 100),
            ("gens", self.gens, 1, 1_000),
            ("holdout", self.holdout, 1, 1_000),
            ("final", self.final_set, 1, 1_000),
            ("pop", self.pop, 1, usize::MAX),
        ];
        for (name, v, lo, hi) in limits {
            if v < lo || v > hi {
                return Err(format!("{name}={v} is outside {lo}..={hi}: the run's layout sets would overlap or be empty"));
            }
        }
        if self.bed.every == 0 || self.bed.frames == 0 {
            return Err("every= and frames= must be at least 1".into());
        }
        Ok(())
    }
}

/// Layout seeds per set, far enough apart never to meet ([`Settings::check`]
/// holds a run inside the limits that keeps them so): training layouts are
/// redrawn every generation, the hold-out and final sets never move.
const HOLDOUT_BASE: u64 = 1_000_000;
const FINAL_BASE: u64 = 2_000_000;
const TRAIN_BASE: u64 = 10_000_000;

/// A genome in the run: its species text and name, or the shipped control.
#[derive(Clone)]
pub struct Arm {
    pub name: String,
    /// `None` for a compiled-in species, scored as shipped.
    pub ron: Option<String>,
}

impl Arm {
    pub fn shipped(name: &str) -> Self {
        Self { name: name.to_string(), ron: None }
    }

    pub fn of(def: &SpeciesDef) -> Self {
        Self { name: def.name.clone(), ron: Some(crate::sim::species_export::to_ron(def).expect("a species serialises")) }
    }
}

/// Every arm on every layout, in parallel; `out[arm][layout]`. Each bed is
/// its own world, so the result does not depend on the thread count.
pub fn census_all(arms: &[Arm], layouts: &[Layout], bed: &BedSpec) -> Vec<Vec<Census>> {
    let tasks: Vec<(usize, usize)> = (0..arms.len()).flat_map(|a| (0..layouts.len()).map(move |l| (a, l))).collect();
    let flat: Vec<Census> = tasks
        .par_iter()
        .map(|&(a, l)| {
            let arm = &arms[a];
            arena::grow(&arena::bed_for(arm.ron.as_deref(), &arm.name, layouts[l], bed), bed)
        })
        .collect();
    flat.chunks(layouts.len().max(1)).map(|c| c.to_vec()).collect()
}

/// Bed scores, `out[arm][layout]`.
pub fn score_all(arms: &[Arm], layouts: &[Layout], bed: &BedSpec, k: &Constants) -> Vec<Vec<f32>> {
    census_all(arms, layouts, bed).iter().map(|row| row.iter().map(|c| score::bed_score(c, k)).collect()).collect()
}

/// Read a species file.
pub fn load(path: &std::path::Path) -> Result<SpeciesDef, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// **The teeth arms** (plan §7.4): handicaps of known direction, built from
/// the start species. `seedbound`: the shoot's `Grew` rule makes a seed
/// rather than a tip, so the plant never grows past its first cell
/// (`selection_arena`'s `lethal`). `starved`: every photosynthesis rate cut to
/// a tenth.
pub fn teeth_arms(start: &SpeciesDef, run: u64) -> Vec<Arm> {
    let mut seedbound = genome::clone_def(start);
    seedbound.name = format!("{}_t{run}_seedbound", start.name);
    if let Some((_, rules)) = seedbound.fates.iter_mut().find(|(c, _)| *c == CellType::GrowingTip) {
        for f in rules.iter_mut() {
            if f.when == crate::sim::organism::FateWhen::Grew {
                f.child = Some(CellType::Seed);
            }
        }
    }
    let mut starved = genome::clone_def(start);
    starved.name = format!("{}_t{run}_starved", start.name);
    for gene in genome::gene_table(start) {
        if let genome::Addr::Param { param: crate::sim::organism::ParamId::PhotosynthesizeRate, .. } = gene.addr {
            if let Some(v) = genome::read(&starved, gene.addr) {
                genome::write(&mut starved, gene.addr, v * 0.1);
            }
        }
    }
    vec![Arm::of(&seedbound), Arm::of(&starved)]
}

/// The outcome of a run, for the caller to print or test.
#[derive(Debug)]
pub struct Outcome {
    pub champion: String,
    pub champion_ron: String,
    /// Final set, champion minus shipped: mean, ahead, behind, half-width.
    pub final_paired: (f32, usize, usize, f32),
}

/// One whole run. Writes `settings.txt`, `progress.csv`, `holdout.csv`,
/// `best.ron` (the champion by training score, every generation),
/// `champion-gNNN.ron` (each generation's, kept) and `final.txt` under
/// `settings.out`.
pub fn run(s: &Settings) -> Result<Outcome, String> {
    s.check()?;
    std::fs::create_dir_all(&s.out).map_err(|e| e.to_string())?;
    let line = s.line();
    println!("{line}");
    std::fs::write(s.out.join("settings.txt"), format!("{line}\n")).map_err(|e| e.to_string())?;

    let mut start = load(&s.start)?;
    // The start may be the shipped species' own file: rename it so it is
    // never registered over the control.
    start.name = genome::child_name(&start.name, s.seed, 0, 0);
    let genes: Vec<Gene> = genome::gene_table(&start);
    let free: Vec<usize> = (0..genes.len()).filter(|&i| s.groups.contains(&genes[i].group)).collect();
    println!("plantrain: {} genes, {} free: {}", genes.len(), free.len(), free.iter().map(|&i| genes[i].name()).collect::<Vec<_>>().join(" "));
    if free.is_empty() {
        return Err("no free genes: the groups named move nothing this species authors".into());
    }
    let probe = crate::sim::world::World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
    let control = Arm::shipped(&s.control);
    let holdout = arena::layouts(HOLDOUT_BASE + s.seed * 1_000, s.holdout);

    // **Teeth, before generation 0.**
    let teeth = teeth_arms(&start, s.seed);
    let mut arms = vec![control.clone(), Arm::of(&start)];
    arms.extend(teeth.iter().cloned());
    let scored = score_all(&arms, &holdout, &s.bed, &s.constants);
    let names = ["control", "start", "seedbound", "starved"];
    for (i, row) in scored.iter().enumerate() {
        let (mean, min) = score::fold(row);
        println!("plantrain: pre-flight {:<9} mean {mean:.3} min {min:.3}  {:?}", names[i], row);
    }
    let control_mean = score::fold(&scored[0]).0;
    if s.teeth {
        for (i, row) in scored.iter().enumerate().skip(2) {
            let m = score::fold(row).0;
            if m >= control_mean {
                return Err(format!("teeth: the {} arm scores {m:.3}, not below the control's {control_mean:.3} -- the score cannot tell a crippled plant from a live one", names[i]));
            }
        }
    }

    let mut rng = Rng::new(s.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    // Generation 0: the start itself and children of it.
    let mut pop: Vec<(SpeciesDef, String)> = vec![(genome::clone_def(&start), "start".into())];
    while pop.len() < s.pop {
        let (mut child, moved) = genome::mutate(&start, &genes, &free, s.sigma, s.max_genes, &mut rng);
        child.name = genome::child_name(&s.control, s.seed, 0, pop.len());
        pop.push((child, describe(&genes, &moved)));
    }

    let mut progress = String::from("gen,best,median,control,best_min,holdout_best,holdout_control,paired_mean,ahead,behind,halfwidth,best_name,best_moves\n");
    let mut holdout_csv = String::from("gen,layout,seed,offset,best,control\n");
    let keep = ((s.pop as f32 * s.keep).round() as usize).clamp(1, s.pop);
    let mut champion: (SpeciesDef, String) = (genome::clone_def(&start), "start".into());
    for g in 0..s.gens {
        for (i, (def, _)) in pop.iter().enumerate() {
            if let Err(e) = genome::validate(def, &probe.materials, &probe.species) {
                return Err(format!("generation {g} child {i} ({}) is invalid: {e:?}", def.name));
            }
        }
        let train = arena::layouts(TRAIN_BASE + s.seed * 100_000 + g as u64 * 100, s.layouts);
        let mut arms: Vec<Arm> = pop.iter().map(|(d, _)| Arm::of(d)).collect();
        arms.push(control.clone());
        let scores = score_all(&arms, &train, &s.bed, &s.constants);
        let control_train = score::fold(scores.last().expect("the control row")).0;
        let mut ranked: Vec<(usize, f32, f32)> = scores[..pop.len()]
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let (mean, min) = score::fold(row);
                (i, mean, min)
            })
            .collect();
        // Best first; ties broken by slot so the order is a function of the run.
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        let (bi, best, best_min) = ranked[0];
        let median = ranked[ranked.len() / 2].1;
        champion = (genome::clone_def(&pop[bi].0), pop[bi].1.clone());

        // The monitoring hold-out: champion against control, paired.
        let h = score_all(&[Arm::of(&champion.0), control.clone()], &holdout, &s.bed, &s.constants);
        let (hb, hc) = (score::fold(&h[0]).0, score::fold(&h[1]).0);
        let (pm, ahead, behind, hw) = score::paired(&h[0], &h[1]);
        for (li, l) in holdout.iter().enumerate() {
            let _ = writeln!(holdout_csv, "{g},{li},{},{},{:.4},{:.4}", l.seed, l.offset.name(), h[0][li], h[1][li]);
        }
        let _ = writeln!(
            progress,
            "{g},{best:.4},{median:.4},{control_train:.4},{best_min:.4},{hb:.4},{hc:.4},{pm:.4},{ahead},{behind},{hw:.4},{},\"{}\"",
            champion.0.name, champion.1
        );
        println!(
            "plantrain: gen {g:>3} best {best:.3} (min {best_min:.3}) median {median:.3} control {control_train:.3} | hold-out best {hb:.3} control {hc:.3} paired {pm:+.3} +/-{hw:.3} ({ahead} ahead, {behind} behind) | {}",
            champion.1
        );
        std::fs::write(s.out.join("progress.csv"), &progress).map_err(|e| e.to_string())?;
        std::fs::write(s.out.join("holdout.csv"), &holdout_csv).map_err(|e| e.to_string())?;
        let champion_ron = crate::sim::species_export::to_ron(&champion.0).map_err(|e| format!("{e:?}"))?;
        std::fs::write(s.out.join("best.ron"), &champion_ron).map_err(|e| e.to_string())?;
        // **Staged**: every generation's champion is kept, so the lineage can
        // be drawn afterwards (`plantrain mode=pic files=out/champion-g*.ron`)
        // and a regression located to the generation that made it.
        // `best.ron` alone kept only the last.
        std::fs::write(s.out.join(format!("champion-g{g:03}.ron")), &champion_ron).map_err(|e| e.to_string())?;

        if g + 1 == s.gens {
            break;
        }
        // Parents: the best quarter, or a random quarter for the null.
        let parents: Vec<usize> = match s.select {
            Select::Score => ranked.iter().take(keep).map(|r| r.0).collect(),
            Select::Random => {
                let mut idx: Vec<usize> = (0..pop.len()).collect();
                for i in (1..idx.len()).rev() {
                    idx.swap(i, rng.below(i as u32 + 1) as usize);
                }
                idx.truncate(keep);
                idx
            }
        };
        let mut next: Vec<(SpeciesDef, String)> = Vec::with_capacity(s.pop);
        for &p in &parents {
            let mut d = genome::clone_def(&pop[p].0);
            d.name = genome::child_name(&s.control, s.seed, g + 1, next.len());
            next.push((d, pop[p].1.clone()));
        }
        while next.len() < s.pop {
            let p = parents[rng.below(parents.len() as u32) as usize];
            let (mut child, moved) = genome::mutate(&pop[p].0, &genes, &free, s.sigma, s.max_genes, &mut rng);
            child.name = genome::child_name(&s.control, s.seed, g + 1, next.len());
            let lineage = if pop[p].1 == "start" { describe(&genes, &moved) } else { format!("{}; {}", pop[p].1, describe(&genes, &moved)) };
            next.push((child, lineage));
        }
        pop = next;
    }

    // **The final set, read once.**
    let finals = arena::layouts(FINAL_BASE + s.seed * 1_000, s.final_set);
    let f = score_all(&[Arm::of(&champion.0), control.clone(), Arm::of(&start)], &finals, &s.bed, &s.constants);
    let fp = score::paired(&f[0], &f[1]);
    let report = format!(
        "{line}\nfinal set ({} layouts): champion {:.4} control {:.4} start {:.4}\nchampion - control: mean {:+.4} +/-{:.4}, ahead on {}, behind on {}\nchampion {}: {}\n",
        finals.len(),
        score::fold(&f[0]).0,
        score::fold(&f[1]).0,
        score::fold(&f[2]).0,
        fp.0,
        fp.3,
        fp.1,
        fp.2,
        champion.0.name,
        champion.1
    );
    print!("{report}");
    std::fs::write(s.out.join("final.txt"), &report).map_err(|e| e.to_string())?;
    let champion_ron = crate::sim::species_export::to_ron(&champion.0).map_err(|e| format!("{e:?}"))?;
    Ok(Outcome { champion: champion.0.name.clone(), champion_ron, final_paired: fp })
}

/// What a child's moves were, for the log: `gene from->to`.
fn describe(genes: &[Gene], moved: &[(usize, f32, f32)]) -> String {
    moved.iter().map(|&(gi, a, b)| format!("{} {a:.4}->{b:.4}", genes[gi].name())).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn herb_text() -> &'static str {
        include_str!("../../assets/species/herb.ron")
    }

    fn bed_hash(arm: &Arm, layout: Layout, spec: &BedSpec) -> u64 {
        arena::grow_hash(&arena::bed_for(arm.ron.as_deref(), &arm.name, layout, spec), spec)
    }

    /// **What is scored is what ships**: a renamed copy of the shipped herb,
    /// written out and registered again, grows a bed bit-identical to the
    /// shipped herb's -- so a child's name is not a variable, and a staged
    /// file is the plant that was scored. **Watched failing**: with the copy's
    /// plastochron 5 -> 4 the hashes differ. Two weaker faults did *not*
    /// move this 6,000-frame bed and are worth knowing about: leaf
    /// photosynthesis x1.01, and one more leaf per node (8 -> 9; carbon caps
    /// the spray below either). A parameter can be flat in a young bed.
    #[test]
    fn a_renamed_round_tripped_herb_grows_the_shipped_herbs_bed() {
        let spec = BedSpec { frames: 6_000, ..BedSpec::default() };
        let layout = Layout { seed: 5, offset: arena::Offset::Quarter };
        let mut def: SpeciesDef = ron::from_str(herb_text()).expect("herb.ron parses");
        def.name = genome::child_name("herb", 9, 0, 0);
        let copy = Arm::of(&def);
        assert_eq!(bed_hash(&Arm::shipped("herb"), layout, &spec), bed_hash(&copy, layout, &spec));
        let c = arena::grow(&arena::bed_for(copy.ron.as_deref(), &copy.name, layout, &spec), &spec);
        assert!(c.germinated && c.peak_shoot > 0, "the copy never grew, so the hash compared two empty beds: {c:?}");
    }

    #[test]
    fn a_seedbound_herb_never_establishes() {
        let def: SpeciesDef = ron::from_str(herb_text()).expect("herb.ron parses");
        let teeth = teeth_arms(&def, 1);
        let spec = BedSpec { frames: 3_000, ..BedSpec::default() };
        let layout = Layout { seed: 2, offset: arena::Offset::Under };
        let c = arena::grow(&arena::bed_for(teeth[0].ron.as_deref(), &teeth[0].name, layout, &spec), &spec);
        assert!(c.peak_shoot < spec.n_est, "{c:?}");
    }

    /// **A run is a function of its settings** (plan §7.7): the same
    /// arguments write the same `progress.csv`, `holdout.csv` and champion,
    /// byte for byte, whatever the thread count. A tiny run, so the test is
    /// about the loop's bookkeeping (mutation draws, ranking ties, parent
    /// picks), not about plants.
    #[test]
    fn a_run_is_a_function_of_its_settings() {
        let dir = std::env::temp_dir().join(format!("plantrain-determinism-{}", std::process::id()));
        let settings = |out: PathBuf| Settings {
            start: PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/species/herb.ron")),
            control: "herb".into(),
            groups: vec![Group::Shape, Group::Leaf],
            pop: 4,
            gens: 2,
            keep: 0.5,
            layouts: 1,
            holdout: 1,
            final_set: 1,
            sigma: 0.9,
            max_genes: 3,
            seed: 3,
            select: Select::Score,
            teeth: false,
            bed: BedSpec { frames: 1_500, ..BedSpec::default() },
            constants: Constants { n_est: 20, k_seed: 24, size_floor: 55 },
            out,
        };
        let a = run(&settings(dir.join("a"))).expect("run a");
        // The second run on one thread, so a result that depended on which
        // bed finished first would differ here.
        let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().expect("a one-thread pool");
        let b = one.install(|| run(&settings(dir.join("b")))).expect("run b");
        assert_eq!(a.champion_ron, b.champion_ron);
        for f in ["progress.csv", "holdout.csv", "champion-g000.ron", "champion-g001.ron"] {
            let read = |d: &str| std::fs::read_to_string(dir.join(d).join(f)).unwrap_or_else(|_| panic!("{f} written"));
            assert_eq!(read("a"), read("b"), "{f} differs between two identical runs");
        }
        // The last staged champion is the one `best.ron` holds.
        let read = |f: &str| std::fs::read_to_string(dir.join("a").join(f)).expect("written");
        assert_eq!(read("champion-g001.ron"), read("best.ron"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
