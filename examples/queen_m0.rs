//! **Do the shipped pool's minds behave differently?** (queen seat, M0, 2026-10-25)
//!
//! The queen seat's demo lets the player choose *whose mind* each egg carries. That teaches nothing if the 30
//! members of `assets/minds/std_pool.txt` all do the same thing (members 2-30 differ from member 1 in only 8-107 of
//! 3,285 slots). This is a common garden scored **inside one mixed colony**: every ant thinks with one member (the
//! lab's own `id % n` assignment, and a newborn inherits its parent's mind), and every ant's own `LifeCounters` are
//! read every `sample=` frames, so each member is scored on the same ground, the same weather and the same food,
//! by the deeds the demo's ledger will show. One `ANT` row per ant seen (for a label-permutation null in
//! post-processing) and one `MEMBER` row per member.
//!
//! Controls, both inside the same colony so they share its luck:
//! - `ablate=K` (positive): member K's Dig verb links zeroed. Its digs per ant-frame must fall to about zero, or the
//!   census cannot see a difference that is certainly there.
//! - `twin=K` (negative): member K entered twice, under two labels (the last label is the copy). The two must not
//!   differ by more than the label-permutation null allows.
//!
//! ```text
//! cargo run --release --example queen_m0 -- scenario=steady_income seed=1 after=30000 twin=0 ablate=5
//! ```
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::training;
use pixel_physics::sim::creature::mind;
use pixel_physics::sim::creature::options::Verb;
use pixel_physics::sim::organism::LifeCounters;
use pixel_physics::train::challenges::colony_bed;
use std::collections::BTreeMap;
use std::sync::Arc;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

struct Seen {
    member: usize,
    first: u64,
    last: u64,
    life: LifeCounters,
}

fn main() {
    for a in std::env::args().skip(1) {
        let k = a.split('=').next().unwrap_or("");
        assert!(["scenario", "seed", "after", "sample", "twin", "ablate", "grant", "shuffle", "wild"].contains(&k), "unknown argument {a}");
    }
    let name: String = arg("scenario").unwrap_or_else(|| "steady_income".to_string());
    let after: u64 = arg("after").unwrap_or(30_000);
    let sample: u64 = arg("sample").unwrap_or(250);
    let twin: Option<usize> = arg("twin");
    let ablate: Option<usize> = arg("ablate");
    // `grant=K`: fill every ant to K full tanks the frame the colony lands (the training bed's founding grant), so
    // a member is not scored on whether its two founders happened to eat before the lab's thin start starved them.
    let grant: f32 = arg("grant").unwrap_or(0.0);
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    let mut genomes = training::std_pool();
    if let Some(k) = ablate {
        for i in mind::verb_slots(Verb::Dig) {
            if i < genomes[k].len() {
                genomes[k][i] = 0.0;
            }
        }
    }
    if let Some(k) = twin {
        genomes.push(genomes[k].clone());
    }
    // `wild=PM,SIGMA,EVENTS` (M0b): the pool becomes member 0 ten times (labels 0-9, the control) and twenty mutants
    // of member 0 at that width (labels 10-29). The mutants are the same on every seed (keyed on their index only), so
    // a mutant's score pools over seeds; each is one wild line among ordinary ones, which is what the demo's wild egg
    // makes.
    if let Some(w) = arg::<String>("wild") {
        let v: Vec<f64> = w.split(',').map(|x| x.parse().expect("wild=PM,SIGMA,EVENTS")).collect();
        let base = genomes[0].clone();
        genomes = (0..30)
            .map(|i| if i < 10 { base.clone() } else { training::mutate_genome(&base, v[0] as f32, v[1] as f32, v[2] as usize, 0x5EED_0000 + i as u64) })
            .collect();
        println!("wild pm={} sigma={} events={}", v[0], v[1], v[2]);
    }
    // `shuffle=1`: the lab hands ant `id` member `id % n`, and which ants of a landing colony live through the
    // thin start is a matter of the id (measured 2026-10-25: a copy of a member that always died lived under
    // another label), so a member's score is its ids' luck unless the order is shuffled per seed. `label[i]` is
    // the original member index of the pool's slot i.
    let mut label: Vec<usize> = (0..genomes.len()).collect();
    if arg::<u32>("shuffle") == Some(1) {
        let mut r = sc.bed.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        for i in (1..label.len()).rev() {
            r ^= r << 13;
            r ^= r >> 7;
            r ^= r << 17;
            label.swap(i, (r % (i as u64 + 1)) as usize);
        }
    }
    let pool: Vec<Arc<mind::MindGenome>> = label.iter().map(|&i| colony_bed::mind(&genomes[i])).collect();
    println!(
        "queen_m0 scenario={name} seed={} after={after} sample={sample} members={} twin={twin:?} ablate={ablate:?} grant={grant}",
        sc.bed.seed,
        pool.len()
    );
    let mut lab = pixel_physics::lab::Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    lab.set_new_ant_pool(pool.clone());
    let sp = lab.world.species.id_of("ant").expect("ant");
    let mut seen: BTreeMap<u32, Seen> = BTreeMap::new();
    let mut landed: Option<u64> = None;
    loop {
        lab.tick_for_harness();
        let f = lab.world.frame;
        if f.is_multiple_of(sample) {
            let w = &lab.world;
            for id in w.live_organism_ids() {
                let Some(s) = w.organism(id).filter(|s| s.species == sp) else { continue };
                let Some(g) = mind::genome_of(w, id) else { continue };
                let Some(member) = pool.iter().position(|p| Arc::ptr_eq(p, &g)).map(|i| label[i]) else { continue };
                if landed.is_none() {
                    landed = Some(f);
                }
                let e = seen.entry(id).or_insert(Seen { member, first: f, last: f, life: LifeCounters::default() });
                // A reused slot is a different ant: key it apart by its first sighting.
                if e.member != member {
                    continue;
                }
                e.last = f;
                e.life = s.life;
            }
        }
        if grant > 0.0 && landed == Some(f) && seen.len() <= pool.len() * 3 {
            lab.grant_ants(grant);
        }
        if landed.is_some_and(|l| f >= l + after) || (landed.is_none() && f >= 100_000) {
            break;
        }
    }
    let n = pool.len();
    let mut per = vec![(0usize, 0u64, 0u64, 0u64, 0u64, 0u64); n];
    for (id, s) in &seen {
        let frames = s.last - s.first + sample;
        println!(
            "ANT id={id} member={} frames={frames} digs={} deliveries={} trips={} offspring={} moves={}",
            s.member, s.life.digs, s.life.deliveries, s.life.trip_deliveries, s.life.offspring, s.life.moves
        );
        let p = &mut per[s.member];
        p.0 += 1;
        p.1 += frames;
        p.2 += u64::from(s.life.digs);
        p.3 += u64::from(s.life.deliveries);
        p.4 += u64::from(s.life.trip_deliveries);
        p.5 += u64::from(s.life.offspring);
    }
    for (m, p) in per.iter().enumerate() {
        let k = 1000.0 / (p.1.max(1) as f64);
        println!(
            "MEMBER m={m} ants={} antframes={} digs_pk={:.3} del_pk={:.3} trips_pk={:.3} offspring={}",
            p.0,
            p.1,
            p.2 as f64 * k,
            p.3 as f64 * k,
            p.4 as f64 * k,
            p.5
        );
    }
    println!("RESULT landed={} frame={} ants_seen={}", landed.map_or(-1, |l| l as i64), lab.world.frame, seen.len());
}
