//! **The plant trainer's command line** -- species files found by selection
//! in the lab's own bed (`src/train_plant/`, plan of record
//! `Reports/plant-trainer-plan-2026-10-19.md`).
//!
//! ```text
//! # train: herb, its shape and leaf genes free, 24 genomes, 6 layouts a generation
//! cargo run --release --example plantrain -- groups=shape,leaf pop=24 gens=20 layouts=6 out=training/plants/run1
//! # train on rung 2 (juvenile growth, bright and dim), scored against the shipped species' size there
//! cargo run --release --example plantrain -- control=shrub rung=grow stations=bright,dim pop=24 gens=20 layouts=4 out=training/plants/shrub-grow
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
//! # does a herb lean towards a lamp beside it, and does zeroing light_weight take it away (the blind control's precondition)
//! cargo run --release --example plantrain -- mode=lean layouts=4 offsets=4,8 stops=8000,16000,24000
//! # rank genomes by mean or by worst layout: which pick holds up on a second, disjoint set?
//! cargo run --release --example plantrain -- mode=ranking genomes=16 layouts=12 sigma=0.9 genes=8
//! # the ladder's first rungs on the shipped role species: per station, who establishes, how fast it grows, what a bed costs
//! cargo run --release --example plantrain -- mode=rungs species=tree,shrub,herb,creeper rungs=establish,grow stations=bright,dim layouts=12
//! # ...and a trained file beside the species it came from, over a whole life
//! cargo run --release --example plantrain -- mode=rungs species=shrub,training/plants/run1/best.ron rungs=life layouts=12
//! # does each rung rank genomes the way the next one does? mutated children on rungs 1, 2 and a whole life
//! cargo run --release --example plantrain -- mode=ladder control=herb genomes=16 layouts=8 stations=bright,dim
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
    "heavy", "spacing", "offsets", "top", "species", "rungs", "stations", "drysoil", "rung", "station", "neighbours", "neighbour",
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
    let b = BedSpec {
        width: get(m, "width", d.width),
        frames: get(m, "frames", d.frames),
        every: get(m, "every", d.every),
        n_est: get(m, "n_est", d.n_est),
        mutation_off: match m.get("mutation").map(String::as_str) {
            None | Some("off") => true,
            Some("on") => false,
            Some(v) => panic!("mutation={v} is not on|off"),
        },
        // `station=dim` for `mode=pic` and the other whole-bed modes; the
        // trainer takes `stations=` instead, since it scores several.
        station: m.get("station").map_or(d.station, |v| arena::Station::named(v).unwrap_or_else(|| panic!("station `{v}` is not bright|dim|dry"))),
        // `neighbour=tree` for `mode=pic`: draw a rung-4 bed.
        neighbour: m.get("neighbour").map(|v| arena::Neighbour::named(v).unwrap_or_else(|| panic!("neighbour `{v}` is not one of {:?}", arena::ROLES))),
    };
    if b.every == 0 || b.frames == 0 {
        eprintln!("plantrain: every= and frames= must be at least 1");
        std::process::exit(2);
    }
    b
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

/// **The ladder's rungs on shipped species, per station** (plant-community
/// plan §6.2, M2): germinated and established out of the layouts, the frame
/// it established at, shoot cells at the end with their spread, the juvenile
/// growth rate from establishment to the end, and the wall time per bed --
/// the numbers a rung's constants and its compute budget (§13) are set from,
/// and the check that a station changes anything at all.
fn rungs(m: &BTreeMap<String, String>) {
    let species: Vec<String> = get(m, "species", "tree,shrub,herb,creeper".to_string()).split(',').map(str::to_string).collect();
    let rungs: Vec<arena::Rung> = get(m, "rungs", "establish,grow".to_string())
        .split(',')
        .map(|r| arena::Rung::named(r).unwrap_or_else(|| panic!("rung `{r}` is not establish|grow|life|hold")))
        .collect();
    // `rung=hold` grows each species beside each of these in turn.
    let neighbours = neighbours_arg(m, "tree,shrub,herb,creeper");
    let k = constants(m, &BedSpec::default());
    // `dry` is left out by default: see `Station::DRY` for why it scores
    // nothing yet.
    let stations: Vec<arena::Station> = get(m, "stations", "bright,dim".to_string())
        .split(',')
        .map(|s| arena::Station::named(s).unwrap_or_else(|| panic!("station `{s}` is not bright|dim|dry")))
        .collect();
    let n: usize = get(m, "layouts", 12);
    let base: u64 = get(m, "base", 3_000_000);
    let threads = rayon::current_num_threads();
    println!("plantrain rungs: species={} rungs={} stations={} layouts={n} base={base} threads={threads}", species.join(","), rungs.iter().map(|r| r.name()).collect::<Vec<_>>().join(","), stations.iter().map(|s| s.name).collect::<Vec<_>>().join(","));
    for &rung in &rungs {
        let beside: Vec<Option<arena::Neighbour>> = if rung == arena::Rung::Hold { neighbours.iter().copied().map(Some).collect() } else { vec![None] };
        for (&station, nb) in stations.iter().flat_map(|st| beside.iter().map(move |nb| (st, *nb))) {
            let mut spec = arena::BedSpec { neighbour: nb, ..rung.spec(station) };
            spec.n_est = get(m, "n_est", spec.n_est);
            // `drysoil=F`: the dry station's starting soil water, for the
            // sweep that finds a value that is dry to a plant.
            if station.name == "dry" {
                spec.station.soil = get(m, "drysoil", spec.station.soil);
            }
            let layouts = rung.layouts(base, n);
            // A name ending `.ron` is a trained file, so a champion can be
            // put on the same rungs as the species it came from.
            let arms: Vec<Arm> = species
                .iter()
                .map(|s| if s.ends_with(".ron") { Arm::of(&train_plant::load(std::path::Path::new(s)).expect("the file loads")) } else { Arm::shipped(s) })
                .collect();
            let t = std::time::Instant::now();
            let rows = train_plant::census_all(&arms, &layouts, &spec);
            let per_bed = t.elapsed().as_secs_f32() * threads as f32 / (arms.len() * n) as f32;
            let with = nb.map_or(String::new(), |nb| format!(" beside {} at {}", nb.species, nb.gap));
            println!("  {} {}{with} (width {} frames {} n_est {} lamp {} rain {:?} soil {}): {per_bed:.2} core-s a bed", rung.name(), station.name, spec.width, spec.frames, spec.n_est, spec.station.lamp, spec.station.rain, spec.station.soil);
            for (sp, cs) in species.iter().zip(&rows) {
                let germ = cs.iter().filter(|c| c.germinated).count();
                let est: Vec<f32> = cs.iter().filter_map(|c| c.established_at.map(|f| f as f32)).collect();
                let end: Vec<f32> = cs.iter().map(|c| c.end_shoot as f32).collect();
                let mean = end.iter().sum::<f32>() / n.max(1) as f32;
                let cv = if mean > 0.0 { sd(&end) / mean } else { 0.0 };
                // ln(end / n_est) per 1,000 frames after establishing: the
                // relative growth rate over the juvenile stretch.
                let rgr: Vec<f32> = cs
                    .iter()
                    .filter_map(|c| {
                        let at = c.established_at?;
                        let span = spec.frames.saturating_sub(at);
                        (span > 0 && c.end_shoot > 0).then(|| (c.end_shoot as f32 / spec.n_est.max(1) as f32).ln() * 1000.0 / span as f32)
                    })
                    .collect();
                println!(
                    "    {sp:<8} germ {germ:>2}/{n} est {:>2}/{n} est@ p50 {:>6.0} | end shoot p25 {:>5.0} p50 {:>5.0} p75 {:>5.0} cv {cv:.2} | rgr/1k p25 {:.2} p50 {:.2} p75 {:.2}",
                    est.len(),
                    q(&est, 0.5),
                    q(&end, 0.25),
                    q(&end, 0.5),
                    q(&end, 0.75),
                    q(&rgr, 0.25),
                    q(&rgr, 0.5),
                    q(&rgr, 0.75)
                );
                if nb.is_some() {
                    // Rung 4's two numbers: the founder's own stepped life,
                    // and what its neighbour managed beside it.
                    let own: Vec<f32> = cs.iter().map(|c| score::bed_score(c, &k)).collect();
                    let nbs: Vec<&arena::Census> = cs.iter().filter_map(|c| c.neighbour.as_deref()).collect();
                    let nb_score: Vec<f32> = nbs.iter().map(|c| score::bed_score(c, &k)).collect();
                    let nb_end: Vec<f32> = nbs.iter().map(|c| c.end_shoot as f32).collect();
                    let nb_flower = nbs.iter().filter(|c| c.first_flower.is_some()).count();
                    let nb_alive = nbs.iter().filter(|c| c.founder_alive).count();
                    println!(
                        "    {:<8} own score {:.2} | neighbour score {:.2} end shoot p50 {:>5.0} flowered {nb_flower}/{n} alive {nb_alive}/{n}",
                        "",
                        own.iter().sum::<f32>() / n.max(1) as f32,
                        nb_score.iter().sum::<f32>() / nbs.len().max(1) as f32,
                        q(&nb_end, 0.5)
                    );
                }
            }
        }
    }
}

/// `neighbours=tree,herb`: the shipped plants rung 4 grows a genome beside.
fn neighbours_arg(m: &BTreeMap<String, String>, default: &str) -> Vec<arena::Neighbour> {
    get(m, "neighbours", default.to_string())
        .split(',')
        .filter(|x| !x.is_empty())
        .map(|x| arena::Neighbour::named(x).unwrap_or_else(|| panic!("neighbour `{x}` is not one of {:?}", arena::ROLES)))
        .collect()
}

/// Spearman rank correlation of two equal-length lists, ties at their mean
/// rank.
fn spearman(x: &[f32], y: &[f32]) -> f32 {
    let rank = |v: &[f32]| -> Vec<f32> {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&a, &b| v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal));
        let mut r = vec![0.0; v.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
                j += 1;
            }
            for &t in &idx[i..=j] {
                r[t] = (i + j) as f32 / 2.0;
            }
            i = j + 1;
        }
        r
    };
    let (rx, ry) = (rank(x), rank(y));
    let n = rx.len().max(1) as f32;
    let (mx, my) = (rx.iter().sum::<f32>() / n, ry.iter().sum::<f32>() / n);
    let cov: f32 = rx.iter().zip(&ry).map(|(a, b)| (a - mx) * (b - my)).sum();
    let vx: f32 = rx.iter().map(|a| (a - mx) * (a - mx)).sum();
    let vy: f32 = ry.iter().map(|b| (b - my) * (b - my)).sum();
    cov / (vx * vy).sqrt().max(f32::EPSILON)
}

/// **Does each rung predict the next?** (plant-community plan §6.2's first
/// rule for the ladder: a rung that does not rank candidates the way the
/// next one does is teaching something else.) Mutated children of `control`
/// -- the same children `mode=ranking` makes -- grown on rung 1 (establish)
/// and rung 2 (grow) per station on two disjoint layout sets, and once on a
/// whole life (P1's bed and score). Prints each arm's numbers, then the
/// rank correlations: a rung against itself on the other set (its own
/// repeatability -- a ceiling on anything it can predict), and each rung
/// against the next.
fn ladder(m: &BTreeMap<String, String>) {
    let start = train_plant::load(&start_path(m)).expect("the start species loads");
    let g: usize = get(m, "genomes", 16);
    let n: usize = get(m, "layouts", 8);
    let base: u64 = get(m, "base", 9_000_000);
    let sigma: f32 = get(m, "sigma", 0.9);
    let max_genes: usize = get(m, "genes", 8);
    let groups: Vec<Group> = m
        .get("groups")
        .map(|v| v.split(',').map(|x| Group::parse(x).unwrap_or_else(|| panic!("unknown group `{x}`"))).collect())
        .unwrap_or_else(|| vec![Group::Shape, Group::Leaf, Group::Flower, Group::Height]);
    let stations: Vec<arena::Station> = get(m, "stations", "bright,dim".to_string())
        .split(',')
        .map(|s| arena::Station::named(s).unwrap_or_else(|| panic!("station `{s}` is not bright|dim|dry")))
        .collect();
    let genes = genome::gene_table(&start);
    let free: Vec<usize> = (0..genes.len()).filter(|&i| groups.contains(&genes[i].group)).collect();
    let mut rng = pixel_physics::sim::rng::Rng::new(get(m, "seed", 0x5EED_1000));
    let control: String = get(m, "control", "herb".to_string());
    let mut arms = vec![Arm::shipped(&control)];
    for i in 0..g {
        let (mut d, _) = genome::mutate(&start, &genes, &free, sigma, max_genes, &mut rng);
        d.name = genome::child_name(&start.name, 971, 0, i);
        arms.push(Arm::of(&d));
    }
    println!(
        "plantrain ladder: control={control} genomes={g} sigma={sigma} genes={max_genes} groups={} layouts={n}+{n} base={base} stations={}",
        groups.iter().map(|x| x.name()).collect::<Vec<_>>().join(","),
        stations.iter().map(|s| s.name).collect::<Vec<_>>().join(",")
    );
    let mean_of = |rows: &[Census], f: &dyn Fn(&Census) -> f32| rows.iter().map(f).sum::<f32>() / rows.len().max(1) as f32;
    // cols[name] = one number per arm.
    let mut cols: Vec<(String, Vec<f32>)> = Vec::new();
    for &station in &stations {
        for rung in [arena::Rung::Establish, arena::Rung::Grow] {
            let spec = rung.spec(station);
            for (set, b) in [("A", base), ("B", base + 500_000)] {
                let t = std::time::Instant::now();
                let rows = train_plant::census_all(&arms, &rung.layouts(b, n), &spec);
                eprintln!("  {} {} {set}: {:.0} s", rung.name(), station.name, t.elapsed().as_secs_f32());
                cols.push((format!("{}-{}-{set}", rung.name(), station.name), rows.iter().map(|r| mean_of(r, &|c| c.end_shoot as f32)).collect()));
            }
        }
    }
    let life = arena::Rung::Life.spec(arena::Station::BRIGHT);
    let k = constants(m, &life);
    let t = std::time::Instant::now();
    let life_scores = train_plant::score_all(&arms, &arena::Rung::Life.layouts(base + 1_000_000, n), &life, &k);
    eprintln!("  life bright: {:.0} s", t.elapsed().as_secs_f32());
    cols.push(("life-score".to_string(), life_scores.iter().map(|r| score::fold(r).0).collect()));
    print!("\n{:<5}", "arm");
    for (name, _) in &cols {
        print!(" {name:>18}");
    }
    println!();
    for a in 0..arms.len() {
        let tag = if a == 0 { "ship".to_string() } else { format!("c{a:02}") };
        print!("{tag:<5}");
        for (_, v) in &cols {
            print!(" {:>18.2}", v[a]);
        }
        println!();
    }
    let col = |name: &str| cols.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or_default();
    println!("\nSpearman over {} arms:", arms.len());
    for &station in &stations {
        let st = station.name;
        let (e_a, e_b) = (col(&format!("establish-{st}-A")), col(&format!("establish-{st}-B")));
        let (g_a, g_b) = (col(&format!("grow-{st}-A")), col(&format!("grow-{st}-B")));
        let life = col("life-score");
        println!(
            "  {st:<7} repeat: establish {:+.2} grow {:+.2} | establish -> grow {:+.2} | grow -> life {:+.2} | establish -> life {:+.2}",
            spearman(&e_a, &e_b),
            spearman(&g_a, &g_b),
            spearman(&e_a, &g_b),
            spearman(&g_a, &life),
            spearman(&e_a, &life)
        );
    }
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
    // The baseline is the start itself, renamed and registered like the
    // arms: by name it would be planted only if it were a shipped species,
    // and a trained `best.ron` is not.
    let mut base_def = genome::clone_def(&start);
    base_def.name = genome::child_name(&start.name, 77, 1, 0);
    let mut arms = vec![Arm::of(&base_def)];
    for (i, &v) in values.iter().enumerate() {
        let mut d = genome::clone_def(&start);
        d.name = genome::child_name(&start.name, 77, 0, i);
        genome::write(&mut d, gene.addr, v);
        arms.push(Arm::of(&d));
    }
    let cs = train_plant::census_all(&arms, &layouts, &b);
    for (i, row) in cs.iter().enumerate() {
        let label = if i == 0 { "start".to_string() } else { format!("{gene_name}={}", values[i - 1]) };
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

/// One lean bed: arm, layout, and the founder's signed offset from the lamp.
type LeanBed = (usize, usize, i32);
/// At one stop: shoot cells, then the shoot centroid's and the crown's lean,
/// in columns towards the lamp. `None` once the founder is gone.
type Lean = (usize, f32, f32);

/// **The blind control's precondition** (plan §7.2): does a herb lean towards
/// a lamp beside it, and does zeroing `light_weight` take the lean away? The
/// founder is planted `offsets` columns to each side of the lamp nearest the
/// middle (both signs, so a left/right habit of the bed reads as the same
/// sign on both sides rather than as a lean), with fixtures `spacing` apart.
/// Arms: the start species, the start **blind** (`light_weight` 0 at every
/// tier), and the start with `light_weight` x`heavy`. Read at each of `stops`:
/// the shoot's centroid and its crown's (the top quarter of its rows), in
/// columns towards the lamp. Only the founder's own cells are read.
fn lean(m: &BTreeMap<String, String>) {
    use pixel_physics::sim::explosion::Blasts;
    use pixel_physics::sim::organism::{CellType, ParamId};
    use pixel_physics::sim::particle::ParticleSystem;
    use pixel_physics::sim::player;
    let b = bed(m);
    let mut start = train_plant::load(&start_path(m)).expect("the start species loads");
    start.name = genome::child_name(&start.name, 980, 0, 0);
    let n: usize = get(m, "layouts", 4);
    let base: u64 = get(m, "base", 7_000_000);
    let heavy: f32 = get(m, "heavy", 4.0);
    let spacing: i32 = get(m, "spacing", 0);
    let offsets: Vec<i32> = m.get("offsets").map_or(vec![4, 8], |v| v.split(',').map(|x| x.parse().expect("a column offset")).collect());
    let mut stops: Vec<u64> = m.get("stops").map_or(vec![8_000, 16_000, 24_000], |v| v.split(',').map(|x| x.parse().expect("a frame")).collect());
    // Read in frame order and printed by position, so the list must be in
    // frame order too: `stops=16000,8000` once printed one frame's lean under
    // the other's heading.
    stops.sort_unstable();
    stops.dedup();
    if stops.first() == Some(&0) {
        eprintln!("plantrain: a stop is a frame number, at least 1");
        std::process::exit(2);
    }
    let last = *stops.last().expect("at least one stop");
    let light = |def: &pixel_physics::sim::organism::SpeciesDef, f: f32, tag: usize| {
        let mut d = genome::clone_def(def);
        d.name = genome::child_name(&def.name, 981, tag, 0);
        for tier in 0..4 {
            let addr = genome::Addr::Param { cell: CellType::GrowingTip, param: ParamId::LightWeight, tier: tier as u8 };
            if let Some(v) = genome::read(&d, addr) {
                genome::write(&mut d, addr, v * f);
            }
        }
        d
    };
    let arms = [("start", Arm::of(&start)), ("blind", Arm::of(&light(&start, 0.0, 0))), ("heavy", Arm::of(&light(&start, heavy, 1)))];
    println!(
        "plantrain lean: start={} layouts={n} base={base} offsets=+-{offsets:?} spacing={} heavy={heavy} stops={stops:?}",
        start_path(m).display(),
        if spacing > 0 { spacing.to_string() } else { "bed default".into() }
    );
    // (arm, layout, signed offset)
    let mut tasks: Vec<(usize, usize, i32)> = Vec::new();
    for a in 0..arms.len() {
        for l in 0..n {
            for &d in &offsets {
                tasks.extend([(a, l, -d), (a, l, d)]);
            }
        }
    }
    use rayon::prelude::*;
    let rows: Vec<(LeanBed, Vec<Option<Lean>>)> = tasks
        .par_iter()
        .map(|&(a, l, d)| {
            let mut bedbox = arena::bed_for(arms[a].1.ron.as_deref(), &arms[a].1.name, arena::layouts(base + l as u64, 1)[0], &b);
            if spacing > 0 {
                bedbox.lamp_spacing = spacing;
            }
            let (lamps, _) = bedbox.lamp_columns();
            let mid = bedbox.width / 2;
            let lamp = *lamps.iter().min_by_key(|&&c| (c - mid).abs()).expect("a lamp");
            let col = lamp + d;
            bedbox.founder_at = Some(vec![col]);
            let (mut w, founder) = arena::build(&bedbox, &b);
            let species = w.organism(founder).map(|s| s.species).expect("the founder exists");
            let (mut particles, mut blasts, tuning) = (ParticleSystem::new(), Blasts::new(), player::Tuning::default());
            let toward = (lamp - col).signum() as f32;
            let mut out = Vec::new();
            for f in 1..=last {
                pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &bedbox, None);
                if !stops.contains(&f) {
                    continue;
                }
                let read = w.organism(founder).filter(|s| s.species == species).and_then(|s| {
                    let shoot: Vec<(i32, i32)> = s.cells.keys().copied().filter(|&(_, y)| y < bedbox.ground_y).collect();
                    if shoot.is_empty() {
                        return None;
                    }
                    let top = shoot.iter().map(|&(_, y)| y).min().unwrap_or(0);
                    let crown_cut = top + (bedbox.ground_y - top) / 4;
                    let mean = |v: &[(i32, i32)]| v.iter().map(|&(x, _)| (x - col) as f32).sum::<f32>() / v.len().max(1) as f32;
                    let crown: Vec<(i32, i32)> = shoot.iter().copied().filter(|&(_, y)| y <= crown_cut).collect();
                    Some((shoot.len(), mean(&shoot) * toward, mean(&crown) * toward))
                });
                out.push(read);
            }
            ((a, l, d), out)
        })
        .collect();
    for (si, stop) in stops.iter().enumerate() {
        println!("\n-- frame {stop}: lean towards the lamp, columns (shoot centroid / crown); n = founders alive with a shoot");
        println!("{:<6} {:>6} {:>5} {:>16} {:>16} {:>14} {:>14} {:>8}", "arm", "offset", "n", "centroid mean", "crown mean", "lamp left", "lamp right", "shoot");
        for (a, (name, _)) in arms.iter().enumerate() {
            for &d in &offsets {
                let pick = |sign: i32| -> Vec<Lean> {
                    rows.iter().filter(|((ra, _, rd), _)| *ra == a && *rd == sign * d).filter_map(|(_, v)| v[si]).collect()
                };
                let (left, right) = (pick(1), pick(-1)); // founder right of the lamp: the lamp is to its left
                let all: Vec<Lean> = left.iter().chain(right.iter()).copied().collect();
                let cm: Vec<f32> = all.iter().map(|r| r.1).collect();
                let cr: Vec<f32> = all.iter().map(|r| r.2).collect();
                let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
                let sd2 = |v: &[f32]| 2.0 * sd(v) / (v.len().max(1) as f32).sqrt();
                let side = |v: &[Lean]| format!("{:+.2} (n{})", mean(&v.iter().map(|r| r.2).collect::<Vec<_>>()), v.len());
                let shoot = mean(&all.iter().map(|r| r.0 as f32).collect::<Vec<_>>());
                println!(
                    "{name:<6} {d:>6} {:>5} {:>+9.2} +-{:<5.2} {:>+9.2} +-{:<5.2} {:>14} {:>14} {shoot:>8.1}",
                    all.len(),
                    mean(&cm),
                    sd2(&cm),
                    mean(&cr),
                    sd2(&cr),
                    side(&left),
                    side(&right)
                );
            }
        }
    }
    println!("\n(+- is 2 sd/sqrt(n). `lamp left`/`lamp right` are the crown lean of the founders on each side: a bed habit reads as opposite signs there, a lean as the same sign.)");
}

/// **Mean against min** (plan §5.3): should a genome be ranked by the mean of
/// its layouts' scores or by its worst? The shipped species and `genomes`
/// children of it (`sigma`, up to `genes` genes from `groups`) are scored on
/// two disjoint sets of `layouts`. Each statistic picks its top `keep` on set
/// A; the picks are judged on set B, where "holds up on every offset" is the
/// worst of the three light positions' mean scores. Min is kept only if its
/// picks hold up better on B than the mean's do, by more than the spread.
fn ranking(m: &BTreeMap<String, String>) {
    let b = bed(m);
    let k = constants(m, &b);
    let start = train_plant::load(&start_path(m)).expect("the start species loads");
    let g: usize = get(m, "genomes", 16);
    let n: usize = get(m, "layouts", 12);
    let base: u64 = get(m, "base", 9_000_000);
    let sigma: f32 = get(m, "sigma", 0.9);
    let max_genes: usize = get(m, "genes", 8);
    let keep: usize = get(m, "top", 4);
    let groups: Vec<Group> = m
        .get("groups")
        .map(|v| v.split(',').map(|x| Group::parse(x).unwrap_or_else(|| panic!("unknown group `{x}`"))).collect())
        .unwrap_or_else(|| vec![Group::Shape, Group::Leaf, Group::Flower, Group::Height]);
    let genes = genome::gene_table(&start);
    let free: Vec<usize> = (0..genes.len()).filter(|&i| groups.contains(&genes[i].group)).collect();
    let mut rng = pixel_physics::sim::rng::Rng::new(get(m, "seed", 0x5EED_1000));
    let control: String = get(m, "control", "herb".to_string());
    let mut arms = vec![Arm::shipped(&control)];
    for i in 0..g {
        let (mut d, _) = genome::mutate(&start, &genes, &free, sigma, max_genes, &mut rng);
        d.name = genome::child_name(&start.name, 970, 0, i);
        arms.push(Arm::of(&d));
    }
    if n < arena::Offset::ALL.len() {
        eprintln!("plantrain: ranking needs layouts >= {} so every light position has a bed", arena::Offset::ALL.len());
        std::process::exit(2);
    }
    let (set_a, set_b) = (arena::layouts(base, n), arena::layouts(base + 500_000, n));
    println!(
        "plantrain ranking: control={control} genomes={g} sigma={sigma} genes={max_genes} groups={} layouts={n}+{n} base={base} keep={keep} frames={} k_seed={} size_floor={}",
        groups.iter().map(|x| x.name()).collect::<Vec<_>>().join(","),
        b.frames,
        k.k_seed,
        k.size_floor
    );
    let both: Vec<arena::Layout> = set_a.iter().chain(set_b.iter()).copied().collect();
    let scores = train_plant::score_all(&arms, &both, &b, &k);
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    // Per arm: mean A, min A, mean B, min B, worst offset's mean on B.
    let stats: Vec<[f32; 5]> = scores
        .iter()
        .map(|r| {
            let (a, bb) = r.split_at(n);
            let (ma, mina) = score::fold(a);
            let (mb, minb) = score::fold(bb);
            let worst = arena::Offset::ALL
                .iter()
                .map(|&o| mean(&bb.iter().zip(&set_b).filter(|(_, l)| l.offset == o).map(|(s, _)| *s).collect::<Vec<_>>()))
                .fold(f32::INFINITY, f32::min);
            [ma, mina, mb, minb, worst]
        })
        .collect();
    println!("\n{:<4} {:>7} {:>7} {:>7} {:>7} {:>12}", "arm", "mean A", "min A", "mean B", "min B", "worst off B");
    for (i, s) in stats.iter().enumerate() {
        let tag = if i == 0 { "ship".to_string() } else { format!("c{i:02}") };
        println!("{tag:<4} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>12.3}", s[0], s[1], s[2], s[3], s[4]);
    }
    // Ties at the minimum are what a min over a stepped score is made of:
    // count them, since a statistic with every arm tied ranks nothing.
    let mut mins: Vec<f32> = stats.iter().map(|s| s[1]).collect();
    mins.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    mins.dedup();
    println!("\ndistinct min-A values over {} arms: {} ({:?})", stats.len(), mins.len(), mins);
    let rank = |col: usize| -> Vec<f32> {
        let mut idx: Vec<usize> = (0..stats.len()).collect();
        idx.sort_by(|&x, &y| stats[x][col].partial_cmp(&stats[y][col]).unwrap_or(std::cmp::Ordering::Equal));
        // Average rank over ties, so a tied block is not ordered by index.
        let mut r = vec![0.0; stats.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && stats[idx[j + 1]][col] == stats[idx[i]][col] {
                j += 1;
            }
            for &t in &idx[i..=j] {
                r[t] = (i + j) as f32 / 2.0;
            }
            i = j + 1;
        }
        r
    };
    let spearman = |x: usize, y: usize| {
        let (rx, ry) = (rank(x), rank(y));
        let (mx, my) = (mean(&rx), mean(&ry));
        let cov: f32 = rx.iter().zip(&ry).map(|(a, b)| (a - mx) * (b - my)).sum();
        let vx: f32 = rx.iter().map(|a| (a - mx) * (a - mx)).sum();
        let vy: f32 = ry.iter().map(|b| (b - my) * (b - my)).sum();
        cov / (vx * vy).sqrt().max(f32::EPSILON)
    };
    println!("\nhow well set A's statistic predicts set B (Spearman):");
    for (xa, xn) in [(0, "mean A"), (1, "min A")] {
        println!(
            "  {xn:<7} -> mean B {:+.2} | min B {:+.2} | worst offset B {:+.2}",
            spearman(xa, 2),
            spearman(xa, 3),
            spearman(xa, 4)
        );
    }
    // Top `keep` by each statistic on A, ties broken by the other statistic
    // so neither pick depends on arm order.
    let top = |first: usize, then: usize| -> Vec<usize> {
        let mut idx: Vec<usize> = (0..stats.len()).collect();
        idx.sort_by(|&x, &y| {
            stats[y][first]
                .partial_cmp(&stats[x][first])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(stats[y][then].partial_cmp(&stats[x][then]).unwrap_or(std::cmp::Ordering::Equal))
        });
        idx.truncate(keep);
        idx
    };
    let (by_mean, by_min) = (top(0, 1), top(1, 0));
    let overlap = by_mean.iter().filter(|i| by_min.contains(i)).count();
    println!("\ntop {keep} on A by mean {by_mean:?}, by min {by_min:?}: {overlap} in common");
    for (name, pick) in [("by mean", &by_mean), ("by min", &by_min)] {
        let on = |c: usize| mean(&pick.iter().map(|&i| stats[i][c]).collect::<Vec<_>>());
        let worst = pick.iter().map(|&i| stats[i][4]).fold(f32::INFINITY, f32::min);
        println!("  picked {name:<8} on B: mean {:.3}  min {:.3}  worst offset {:.3} (the weakest pick's {:.3})", on(2), on(3), on(4), worst);
    }
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
    // The score is read with the run's own constants (`k_seed=`, `size_floor=`,
    // `n_est=`), so it is computed beside the census quantities rather than
    // as one of them: a plain `fn` cannot see `k`.
    let k = constants(m, &b);
    let quantities: [Quantity; 4] = [
        ("peak shoot cells", |c| c.peak_shoot as f32),
        ("peak height", |c| c.peak_height as f32),
        ("shoot at first flower (0 = none)", |c| c.shoot_at_first_flower as f32),
        ("seedlings established", |c| c.seedlings_established as f32),
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
        let columns = quantities
            .iter()
            .map(|(name, f)| (*name, cs.iter().map(|r| r.iter().map(f).collect()).collect::<Vec<Vec<f32>>>()))
            .chain(std::iter::once(("score", cs.iter().map(|r| r.iter().map(|c| score::bed_score(c, &k)).collect()).collect())));
        for (name, rows) in columns {
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
        "rungs" => rungs(&m),
        "ladder" => ladder(&m),
        "sweep" => sweep(&m),
        "score" => score_files(&m),
        "pic" => pic(&m),
        "heritability" => heritability(&m),
        "liveness" => liveness(&m),
        "lean" => lean(&m),
        "ranking" => ranking(&m),
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
                // `rung=grow stations=bright,dim`: train on rung 2, which
                // predicts a whole life at a fifth of its cost
                // (`mode=ladder`); `life`, the default, is P1's bed.
                rung: arena::Rung::named(&get(&m, "rung", "life".to_string())).unwrap_or_else(|| panic!("rung= is establish|grow|life|hold")),
                // `rung=hold`: beside each of these in turn, by default every
                // role but the control's own.
                neighbours: neighbours_arg(&m, &arena::ROLES.iter().filter(|&&r| r != get(&m, "control", "herb".to_string())).copied().collect::<Vec<_>>().join(",")),
                stations: get(&m, "stations", "bright".to_string())
                    .split(',')
                    .map(|x| arena::Station::named(x).unwrap_or_else(|| panic!("station `{x}` is not bright|dim|dry")))
                    .collect(),
                out: m.get("out").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("training/plants/run")),
            };
            if let Err(e) = train_plant::run(&s) {
                eprintln!("plantrain: {e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("plantrain: mode={other} is not train|calibrate|sweep|score|pic|heritability|liveness|lean|ranking|rungs|ladder");
            std::process::exit(2);
        }
    }
}
