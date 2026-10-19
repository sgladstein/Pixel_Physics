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
//! # the heritability gate: how much of the spread between children is their genome, per step size
//! cargo run --release --example plantrain -- mode=heritability sigmas=0.05,0.1,0.2,0.4,0.9 genomes=10 layouts=6
//! # which genes move anything at all: each pushed to both ends of its range, the bed hashed
//! cargo run --release --example plantrain -- mode=liveness layouts=2 frames=20000
//! # a picture: the shipped species and each file, on N layouts, at chosen frames
//! cargo run --release --example plantrain -- mode=pic files=training/plants/run1/best.ron layouts=3 stops=8000,40000 out=pic.png
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
    "width", "frames", "every", "mutation", "n_est", "k_seed", "size_floor", "out", "gene", "values", "files", "base", "stops", "up", "scale", "sigmas", "genomes", "offset",
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

/// **Which genes move anything at all** (`genome_reach`'s dynamic arm, for a
/// species file): every gene pushed to the bottom and the top of its range,
/// each grown on `layouts` beds and hashed against the start's own bed. A gene
/// whose every push leaves every bed's hash unchanged is **inert in this bed**
/// -- conclusive, since an unchanged hash means nothing moved -- and a run
/// that mutates it spends a draw on nothing. A changed hash is weak evidence
/// of importance and is reported as "moves", nothing more.
fn liveness(m: &BTreeMap<String, String>) {
    let b = bed(m);
    let mut start = train_plant::load(&start_path(m)).expect("the start species loads");
    start.name = genome::child_name(&start.name, 990, 0, 0);
    let genes = genome::gene_table(&start);
    let n: usize = get(m, "layouts", 2);
    let base: u64 = get(m, "base", 8_000_000);
    let layouts = arena::layouts(base, n);
    println!("plantrain liveness: {} genes, layouts={n} base={base} frames={}", genes.len(), b.frames);
    let mut arms = vec![Arm::of(&start)];
    for (i, g) in genes.iter().enumerate() {
        for (j, v) in [g.lo, g.hi].into_iter().enumerate() {
            let mut d = genome::clone_def(&start);
            d.name = genome::child_name(&start.name, 991, i, j);
            genome::write(&mut d, g.addr, v);
            arms.push(Arm::of(&d));
        }
    }
    use rayon::prelude::*;
    let tasks: Vec<(usize, usize)> = (0..arms.len()).flat_map(|a| (0..n).map(move |l| (a, l))).collect();
    let hashes: Vec<u64> = tasks
        .par_iter()
        .map(|&(a, l)| arena::grow_hash(&arena::bed_for(arms[a].ron.as_deref(), &arms[a].name, layouts[l], &b), &b))
        .collect();
    let base_hash = &hashes[..n];
    let (mut live, mut dead) = (Vec::new(), Vec::new());
    for (i, g) in genes.iter().enumerate() {
        let moved = (0..2).any(|j| {
            let a = 1 + i * 2 + j;
            (0..n).any(|l| hashes[a * n + l] != base_hash[l])
        });
        if moved {
            live.push(g.name());
        } else {
            dead.push(g.name());
        }
    }
    println!("moves the bed ({}): {}", live.len(), live.join(" "));
    println!("INERT here ({}): {}", dead.len(), dead.join(" "));
}

/// A named number read off a bed's census.
type Quantity = (&'static str, fn(&Census) -> f32);

/// **The heritability gate** (plan §6.4, the review's blocker): for each step
/// size, `genomes` children of the start, each one mutation round of up to
/// `genes` genes from `groups`, grown on the same `layouts`; for each
/// quantity, `1 - mean within-genome variance / total variance`. Near 0, the
/// bed decides what a child looks like and choosing among children is choosing
/// among noise; a look round may not use a step size below the one where the
/// quantity it judges is heritable.
fn heritability(m: &BTreeMap<String, String>) {
    let b = bed(m);
    let start = train_plant::load(&start_path(m)).expect("the start species loads");
    let sigmas: Vec<f32> = m.get("sigmas").map_or(vec![0.05, 0.1, 0.2, 0.4, 0.9], |v| v.split(',').map(|x| x.parse().expect("a sigma")).collect());
    let g: usize = get(m, "genomes", 10);
    let n: usize = get(m, "layouts", 6);
    let base: u64 = get(m, "base", 7_000_000);
    let max_genes: usize = get(m, "genes", 3);
    let groups: Vec<Group> = m
        .get("groups")
        .map(|v| v.split(',').map(|x| Group::parse(x).unwrap_or_else(|| panic!("unknown group `{x}`"))).collect())
        .unwrap_or_else(|| vec![Group::Shape, Group::Leaf, Group::Flower, Group::Height]);
    let genes = genome::gene_table(&start);
    let free: Vec<usize> = (0..genes.len()).filter(|&i| groups.contains(&genes[i].group)).collect();
    // `offset=under|quarter|between` puts every bed at one light position,
    // so the between-bed spread is the soil, the mister and development
    // only -- the control for "is it the light, or the genome?".
    let mut layouts = arena::layouts(base, n);
    if let Some(o) = m.get("offset") {
        let off = arena::Offset::ALL.into_iter().find(|x| x.name() == o).unwrap_or_else(|| panic!("offset={o} is not under|quarter|between"));
        for l in &mut layouts {
            l.offset = off;
        }
    }
    println!(
        "plantrain heritability: offset={}",
        m.get("offset").map_or("mixed", String::as_str)
    );
    println!(
        "plantrain heritability: sigmas={sigmas:?} genomes={g} layouts={n} base={base} genes={max_genes} groups={} frames={} mutation_off={}",
        groups.iter().map(|x| x.name()).collect::<Vec<_>>().join(","),
        b.frames,
        b.mutation_off
    );
    let quantities: [Quantity; 5] = [
        ("peak shoot cells", |c| c.peak_shoot as f32),
        ("peak height", |c| c.peak_height as f32),
        ("shoot at first flower (0 = none)", |c| c.shoot_at_first_flower as f32),
        ("seedlings established", |c| c.seedlings_established as f32),
        ("score", |c| score::bed_score(c, &Constants { n_est: 20, k_seed: 24, size_floor: 55 })),
    ];
    for (si, &sigma) in sigmas.iter().enumerate() {
        let mut rng = pixel_physics::sim::rng::Rng::new(0x5EED_0000 + si as u64);
        let arms: Vec<Arm> = (0..g)
            .map(|i| {
                let (mut d, _) = genome::mutate(&start, &genes, &free, sigma, max_genes, &mut rng);
                d.name = genome::child_name(&start.name, 900 + si as u64, 0, i);
                Arm::of(&d)
            })
            .collect();
        let cs = train_plant::census_all(&arms, &layouts, &b);
        let mut line = format!("sigma {sigma:<5}");
        for (name, f) in &quantities {
            let rows: Vec<Vec<f32>> = cs.iter().map(|r| r.iter().map(f).collect()).collect();
            let all: Vec<f32> = rows.iter().flatten().copied().collect();
            let var = |v: &[f32]| {
                let mean = v.iter().sum::<f32>() / v.len().max(1) as f32;
                v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / v.len().max(1) as f32
            };
            let total = var(&all);
            let within = rows.iter().map(|r| var(r)).sum::<f32>() / rows.len().max(1) as f32;
            let h2 = if total > 0.0 { 1.0 - within / total } else { 0.0 };
            line.push_str(&format!(" | {name} {h2:.2}"));
        }
        println!("{line}");
    }
}

/// One sheet cell, `(arm, layout)`, and one drawn tile's RGBA.
type Cell = (usize, usize);
type Tile = Vec<u8>;

/// **The picture a card is made of**: the shipped species in the first column
/// and each file after it, one row per layout and stop, the founder's bed
/// drawn through the shipped `Renderer` (the box's own interior, lamps and
/// all) and cropped to the rows above and just below the ground.
fn pic(m: &BTreeMap<String, String>) {
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::explosion::Blasts;
    use pixel_physics::sim::particle::ParticleSystem;
    use pixel_physics::sim::player;
    let b = bed(m);
    let control: String = get(m, "control", "herb".to_string());
    let n: usize = get(m, "layouts", 3);
    let base: u64 = get(m, "base", 6_000_000);
    let up: i32 = get(m, "up", 100);
    let scale: usize = get(m, "scale", 3);
    let out: String = get(m, "out", "plantrain-pic.png".to_string());
    let stops: Vec<u64> = m.get("stops").map_or(vec![b.frames], |v| v.split(',').map(|x| x.parse().expect("a frame")).collect());
    let layouts = arena::layouts(base, n);
    let mut arms = vec![Arm::shipped(&control)];
    if let Some(files) = m.get("files") {
        for f in files.split(',') {
            arms.push(Arm::of(&train_plant::load(std::path::Path::new(f)).expect("the file loads")));
        }
    }
    println!("plantrain pic: control={control} arms={} layouts={n} base={base} stops={stops:?} up={up} out={out}", arms.len());
    let below = 12;
    let (tw, th) = (b.width as usize, (up + below) as usize);
    // One task per (arm, layout); each returns a tile per stop.
    let tasks: Vec<(usize, usize)> = (0..arms.len()).flat_map(|a| (0..n).map(move |l| (a, l))).collect();
    use rayon::prelude::*;
    let tiles: Vec<(Cell, Vec<Tile>)> = tasks
        .par_iter()
        .map(|&(a, l)| {
            let bedbox = arena::bed_for(arms[a].ron.as_deref(), &arms[a].name, layouts[l], &b);
            let (mut w, _) = arena::build(&bedbox, &b);
            let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
            let mut renderer = Renderer::new();
            let (fw, fh) = (bedbox.width as u32, bedbox.height as u32);
            let mut out = Vec::new();
            let last = *stops.iter().max().unwrap_or(&0);
            for f in 1..=last {
                pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bedbox, None);
                if stops.contains(&f) {
                    let mut frame = vec![0u8; (fw * fh * 4) as usize];
                    let touched = w.take_touched_chunks();
                    renderer.draw(&w, &particles, &touched, &mut frame, (fw, fh), true);
                    let top = (bedbox.ground_y - up).max(0) as usize;
                    let mut tile = vec![0u8; tw * th * 4];
                    for ty in 0..th {
                        let src = ((top + ty) * fw as usize) * 4;
                        tile[ty * tw * 4..(ty + 1) * tw * 4].copy_from_slice(&frame[src..src + tw * 4]);
                    }
                    out.push(tile);
                }
            }
            ((a, l), out)
        })
        .collect();
    let gap = 3usize;
    let rows = n * stops.len();
    let (sw, sh) = (arms.len() * (tw * scale + gap) + gap, rows * (th * scale + gap) + gap);
    let mut sheet = vec![0u8; sw * sh * 4];
    for px in sheet.chunks_exact_mut(4) {
        px.copy_from_slice(&[24, 24, 28, 255]);
    }
    for ((a, l), stack) in &tiles {
        for (si, tile) in stack.iter().enumerate() {
            let (ox, oy) = (gap + a * (tw * scale + gap), gap + (l * stops.len() + si) * (th * scale + gap));
            for ty in 0..th {
                for tx in 0..tw {
                    let p = &tile[(ty * tw + tx) * 4..(ty * tw + tx) * 4 + 3];
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let i = ((oy + ty * scale + dy) * sw + ox + tx * scale + dx) * 4;
                            sheet[i..i + 3].copy_from_slice(p);
                            sheet[i + 3] = 255;
                        }
                    }
                }
            }
        }
    }
    image::save_buffer(&out, &sheet, sw as u32, sh as u32, image::ColorType::Rgba8).expect("writing the picture");
    println!("plantrain pic: wrote {out} ({sw}x{sh}): column 0 is {control}, then each file; rows are layouts x stops");
}

fn main() {
    let m = args();
    match m.get("mode").map(String::as_str).unwrap_or("train") {
        "calibrate" => calibrate(&m),
        "sweep" => sweep(&m),
        "score" => score_files(&m),
        "pic" => pic(&m),
        "heritability" => heritability(&m),
        "liveness" => liveness(&m),
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
            eprintln!("plantrain: mode={other} is not train|calibrate|sweep|score|pic|heritability|liveness");
            std::process::exit(2);
        }
    }
}
