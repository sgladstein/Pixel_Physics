//! **The plant trainer's command line** -- species files found by selection
//! in the lab's own bed (`src/train_plant/`, plan of record
//! `Reports/plant-trainer-plan-2026-10-19.md`).
//!
//! ```text
//! # train: herb, its shape and leaf genes free, 24 genomes, 6 layouts a generation
//! cargo run --release --example plantrain -- groups=shape,leaf pop=24 gens=20 layouts=6 out=training/plants/run1
//! # the null: the same run with parents drawn at random
//! cargo run --release --example plantrain -- groups=shape,leaf pop=24 gens=20 layouts=6 select=random out=training/plants/null1
//! # the shipped species' own census on N layouts: where the run constants come from
//! cargo run --release --example plantrain -- mode=calibrate layouts=12
//! # one gene swept through the score (the precocity probe: gene=after_metamers values=1,2,...,12)
//! cargo run --release --example plantrain -- mode=sweep gene=after_metamers values=1,3,5,8,12 layouts=6
//! # species files scored against the shipped species on the hold-out, paired
//! cargo run --release --example plantrain -- mode=score files=training/plants/run1/best.ron layouts=12
//! ```
//!
//! **Every argument it does not know is refused**, not ignored: an unknown
//! argument silently ignored is how a 3.5-hour study once produced eight
//! byte-identical logs (`Reports/instruments.md`).

use pixel_physics::train_plant::arena::{self, BedSpec, Census};
use pixel_physics::train_plant::genome::{self, Group};
use pixel_physics::train_plant::score::{self, Constants};
use pixel_physics::train_plant::{self, Arm, Select, Settings};
use std::collections::BTreeMap;
use std::path::PathBuf;

const KNOWN: &[&str] = &[
    "mode", "start", "control", "groups", "pop", "gens", "keep", "layouts", "holdout", "final", "sigma", "genes", "seed", "select", "teeth",
    "width", "frames", "every", "mutation", "n_est", "k_seed", "size_floor", "out", "gene", "values", "files", "base",
];

fn args() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for a in std::env::args().skip(1) {
        let Some((k, v)) = a.split_once('=') else {
            eprintln!("plantrain: `{a}` is not key=value");
            std::process::exit(2);
        };
        if !KNOWN.contains(&k) {
            eprintln!("plantrain: unknown argument `{k}` (known: {})", KNOWN.join(" "));
            std::process::exit(2);
        }
        m.insert(k.to_string(), v.to_string());
    }
    m
}

fn get<T: std::str::FromStr>(m: &BTreeMap<String, String>, k: &str, default: T) -> T {
    m.get(k).map_or(default, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v} does not parse")))
}

fn bed(m: &BTreeMap<String, String>) -> BedSpec {
    let d = BedSpec::default();
    BedSpec {
        width: get(m, "width", d.width),
        frames: get(m, "frames", d.frames),
        every: get(m, "every", d.every),
        n_est: get(m, "n_est", d.n_est),
        mutation_off: match m.get("mutation").map(String::as_str) {
            None | Some("off") => true,
            Some("on") => false,
            Some(v) => panic!("mutation={v} is not on|off"),
        },
    }
}

/// The run's yardsticks. **Defaults measured on the shipped herb**, 24 lab
/// layouts of 40,000 frames (`mode=calibrate layouts=24`, 2026-10-19): it
/// flowered on 17, at 58-103 shoot cells (p25 72), and established 0-23
/// seedlings (p50 13). Set with headroom, never on the measured value
/// (`CLAUDE.md`): `size_floor` 55 sits under the smallest shipped herb that
/// flowered, so no shipped bed is halved; `k_seed` 24 sits over the most
/// seedlings any shipped bed made, so no shipped bed is capped. At 4, the
/// first guess, 14 of the 24 shipped beds sat on the cap and a better herb
/// had nowhere to go.
fn constants(m: &BTreeMap<String, String>, bed: &BedSpec) -> Constants {
    Constants { n_est: bed.n_est, k_seed: get(m, "k_seed", 24), size_floor: get(m, "size_floor", 55) }
}

fn start_path(m: &BTreeMap<String, String>) -> PathBuf {
    m.get("start").map(PathBuf::from).unwrap_or_else(|| {
        let control: String = get(m, "control", "herb".to_string());
        PathBuf::from(format!("{}/assets/species/{control}.ron", env!("CARGO_MANIFEST_DIR")))
    })
}

/// The p-th quantile of `v` (nearest rank), or 0 for an empty list.
fn q(v: &[f32], p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s[((p * (s.len() - 1) as f32).round() as usize).min(s.len() - 1)]
}

fn calibrate(m: &BTreeMap<String, String>) {
    let b = bed(m);
    let control: String = get(m, "control", "herb".to_string());
    let n: usize = get(m, "layouts", 12);
    let base: u64 = get(m, "base", 3_000_000);
    let layouts = arena::layouts(base, n);
    println!(
        "plantrain calibrate: control={control} layouts={n} base={base} width={} frames={} every={} n_est={} mutation_off={}",
        b.width, b.frames, b.every, b.n_est, b.mutation_off
    );
    let rows: Vec<Census> = train_plant::census_all(&[Arm::shipped(&control)], &layouts, &b).remove(0);
    println!("{:>9} {:>8} {:>5} {:>6} {:>6} {:>8} {:>10} {:>6} {:>9} {:>6}", "seed", "offset", "germ", "peak", "high", "flower@", "shoot@flr", "fruit", "seedlings", "alive");
    for (l, c) in layouts.iter().zip(&rows) {
        println!(
            "{:>9} {:>8} {:>5} {:>6} {:>6} {:>8} {:>10} {:>6} {:>9} {:>6}",
            l.seed,
            l.offset.name(),
            c.germinated,
            c.peak_shoot,
            c.peak_height,
            c.first_flower.map_or("-".to_string(), |f| f.to_string()),
            c.shoot_at_first_flower,
            c.fruited,
            c.seedlings_established,
            c.founder_alive
        );
    }
    let col = |f: &dyn Fn(&Census) -> Option<f32>| -> Vec<f32> { rows.iter().filter_map(f).collect() };
    let peak = col(&|c| Some(c.peak_shoot as f32));
    let flower = col(&|c| c.first_flower.map(|f| f as f32));
    let at = col(&|c| c.first_flower.map(|_| c.shoot_at_first_flower as f32));
    let seedlings = col(&|c| Some(c.seedlings_established as f32));
    println!("germinated {} of {n}; flowered {} of {n}; fruited {} of {n}; founder alive at the end {} of {n}",
        rows.iter().filter(|c| c.germinated).count(), flower.len(), rows.iter().filter(|c| c.fruited).count(), rows.iter().filter(|c| c.founder_alive).count());
    println!("peak shoot cells       p25 {:>7.0} p50 {:>7.0} p75 {:>7.0}", q(&peak, 0.25), q(&peak, 0.5), q(&peak, 0.75));
    println!("frame of first flower  p25 {:>7.0} p50 {:>7.0} p75 {:>7.0}", q(&flower, 0.25), q(&flower, 0.5), q(&flower, 0.75));
    println!("shoot at first flower  p25 {:>7.0} p50 {:>7.0} p75 {:>7.0}  (size_floor: under the smallest, with headroom)", q(&at, 0.25), q(&at, 0.5), q(&at, 0.75));
    println!("seedlings established  p25 {:>7.0} p50 {:>7.0} p75 {:>7.0}  (k_seed: over the largest, with headroom)", q(&seedlings, 0.25), q(&seedlings, 0.5), q(&seedlings, 0.75));
    let k = constants(m, &b);
    let scores: Vec<f32> = rows.iter().map(|c| score::bed_score(c, &k)).collect();
    let (mean, min) = score::fold(&scores);
    println!("score at k_seed={} size_floor={}: mean {mean:.3} min {min:.3} sd {:.3}  {:?}", k.k_seed, k.size_floor, sd(&scores), scores);
}

fn sd(v: &[f32]) -> f32 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = v.iter().sum::<f32>() / v.len() as f32;
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / (v.len() - 1) as f32).sqrt()
}

fn sweep(m: &BTreeMap<String, String>) {
    let b = bed(m);
    let k = constants(m, &b);
    let start = train_plant::load(&start_path(m)).expect("the start species loads");
    let gene_name: String = m.get("gene").cloned().expect("sweep needs gene=");
    let values: Vec<f32> = m.get("values").expect("sweep needs values=").split(',').map(|v| v.parse().expect("a value")).collect();
    let genes = genome::gene_table(&start);
    let gene = genes.iter().find(|g| g.name() == gene_name).unwrap_or_else(|| {
        panic!("no gene `{gene_name}`; this species has: {}", genes.iter().map(|g| g.name()).collect::<Vec<_>>().join(" "))
    });
    let n: usize = get(m, "layouts", 6);
    let base: u64 = get(m, "base", 4_000_000);
    let layouts = arena::layouts(base, n);
    println!("plantrain sweep: gene={gene_name} values={values:?} layouts={n} base={base} frames={} n_est={} k_seed={} size_floor={}", b.frames, k.n_est, k.k_seed, k.size_floor);
    let mut arms = vec![Arm::shipped(&start.name)];
    for (i, &v) in values.iter().enumerate() {
        let mut d = genome::clone_def(&start);
        d.name = genome::child_name(&start.name, 77, 0, i);
        genome::write(&mut d, gene.addr, v);
        arms.push(Arm::of(&d));
    }
    let cs = train_plant::census_all(&arms, &layouts, &b);
    for (i, row) in cs.iter().enumerate() {
        let label = if i == 0 { "shipped".to_string() } else { format!("{gene_name}={}", values[i - 1]) };
        let scores: Vec<f32> = row.iter().map(|c| score::bed_score(c, &k)).collect();
        let (mean, min) = score::fold(&scores);
        let flowered = row.iter().filter(|c| c.first_flower.is_some()).count();
        let at: Vec<f32> = row.iter().filter_map(|c| c.first_flower.map(|_| c.shoot_at_first_flower as f32)).collect();
        println!(
            "{label:>24}: score mean {mean:.3} min {min:.3} | flowered {flowered}/{n} shoot@flower p50 {:.0} | peak p50 {:.0} | seedlings p50 {:.0}",
            q(&at, 0.5),
            q(&row.iter().map(|c| c.peak_shoot as f32).collect::<Vec<_>>(), 0.5),
            q(&row.iter().map(|c| c.seedlings_established as f32).collect::<Vec<_>>(), 0.5)
        );
    }
}

fn score_files(m: &BTreeMap<String, String>) {
    let b = bed(m);
    let k = constants(m, &b);
    let control: String = get(m, "control", "herb".to_string());
    let n: usize = get(m, "layouts", 12);
    let base: u64 = get(m, "base", 5_000_000);
    let layouts = arena::layouts(base, n);
    let files: Vec<PathBuf> = m.get("files").expect("score needs files=").split(',').map(PathBuf::from).collect();
    println!("plantrain score: control={control} layouts={n} base={base} frames={} n_est={} k_seed={} size_floor={}", b.frames, k.n_est, k.k_seed, k.size_floor);
    let mut arms = vec![Arm::shipped(&control)];
    for f in &files {
        arms.push(Arm::of(&train_plant::load(f).expect("the file loads")));
    }
    let s = train_plant::score_all(&arms, &layouts, &b, &k);
    let (cm, cmin) = score::fold(&s[0]);
    println!("{:>40}: mean {cm:.3} min {cmin:.3}", control);
    for (i, f) in files.iter().enumerate() {
        let (mean, min) = score::fold(&s[i + 1]);
        let (pm, ahead, behind, hw) = score::paired(&s[i + 1], &s[0]);
        println!("{:>40}: mean {mean:.3} min {min:.3} | minus {control}: {pm:+.3} +/-{hw:.3}, ahead {ahead}, behind {behind}", f.display());
    }
}

fn main() {
    let m = args();
    match m.get("mode").map(String::as_str).unwrap_or("train") {
        "calibrate" => calibrate(&m),
        "sweep" => sweep(&m),
        "score" => score_files(&m),
        "train" => {
            let b = bed(&m);
            let groups: Vec<Group> = m
                .get("groups")
                .map(|g| g.split(',').map(|n| Group::parse(n).unwrap_or_else(|| panic!("unknown group `{n}`"))).collect())
                .unwrap_or_else(|| vec![Group::Shape, Group::Leaf]);
            let s = Settings {
                start: start_path(&m),
                control: get(&m, "control", "herb".to_string()),
                groups,
                pop: get(&m, "pop", 24),
                gens: get(&m, "gens", 20),
                keep: get(&m, "keep", 0.25),
                layouts: get(&m, "layouts", 6),
                holdout: get(&m, "holdout", 12),
                final_set: get(&m, "final", 12),
                sigma: get(&m, "sigma", 0.4),
                max_genes: get(&m, "genes", 3),
                seed: get(&m, "seed", 1),
                select: match m.get("select").map(String::as_str) {
                    None | Some("score") => Select::Score,
                    Some("random") => Select::Random,
                    Some(v) => panic!("select={v} is not score|random"),
                },
                teeth: get(&m, "teeth", 1u8) != 0,
                constants: constants(&m, &b),
                bed: b,
                out: m.get("out").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("training/plants/run")),
            };
            if let Err(e) = train_plant::run(&s) {
                eprintln!("plantrain: {e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("plantrain: mode={other} is not train|calibrate|sweep|score");
            std::process::exit(2);
        }
    }
}
