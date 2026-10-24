//! **Does a training batch's colony live in a lab box?** (Teacher lane, 2026-10-24.) The owner's playtest of the
//! trained ant (ANT BRAIN: NEW) found colonies starving within ~5,000 frames of landing beside 8.9 MJ of fruit, and
//! the played bed reproduced it: 41 of 42 starved within 2,000 frames, the hand-built ant 0. Training never asked:
//! its colonies start on 3 full tanks beside a pile of provisions, a lab colony lands on 1 tank among plants and
//! fruit. This runs a lab scenario through the lab's own `Lab` with a given pool as the new ant, so colony
//! selection can score each colony's carry on where the ant will actually live (`colsel.py`'s `+lab` case).
//!
//! ```text
//! cargo run --release --example labsurvive -- founders=a.raw.txt@x:1,b.raw.txt@y:1 scenario=played_bed seed=1 after=15000
//! cargo run --release --example labsurvive -- founders=std scenario=played_bed     # the shipped pool
//! cargo run --release --example labsurvive -- founders=old scenario=played_bed     # the hand-built ant
//! ```
//!
//! `founders=` takes inrun's list (`path[@tag]:count`; the count is ignored, each genome is one pool member), `std`
//! for the shipped pool, or `old` for the hand-built brain. The colony is whatever the scenario lands; the run goes on
//! for `after=` frames past the first frame an ant stands in the box. Prints one `RESULT` line: ants alive at the
//! end, `lived` (the mean headcount over the `after=` frames, graded where alive is all-or-nothing), the most there were, starved, births, and the frame the colony landed. `grant=K` fills every ant to K full
//! tanks on the landing frame, the training bed's founding grant (`grants=3`), to tell a thin start from the brain.
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::training;
use pixel_physics::sim::organism::DeathCause;
use pixel_physics::train;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let after: u64 = arg("after").unwrap_or(15_000);
    let limit: u64 = arg("limit").unwrap_or(40_000);
    let grant: f32 = arg("grant").unwrap_or(0.0);
    let founders: String = arg("founders").expect("founders=LIST|std|old");
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    println!("labsurvive scenario={name} seed={} after={after} grant={grant} founders={founders}", sc.bed.seed);
    let mut lab = pixel_physics::lab::Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    match founders.as_str() {
        "old" => {}
        "std" => {
            lab.set_new_ant(true);
        }
        list => {
            let minds: Vec<_> = list
                .split(',')
                .map(|part| {
                    let p = part.rsplit_once(':').map_or(part, |(p, _)| p);
                    let p = p.split_once('@').map_or(p, |(p, _)| p);
                    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("genome file {p}: {e}"));
                    let mut g = train::parse_genome(&text, train::genome_text_len(&text, p), p);
                    training::fit_head(&mut g);
                    train::challenges::colony_bed::mind(&g)
                })
                .collect();
            println!("pool of {}", minds.len());
            lab.set_new_ant_pool(minds);
        }
    }
    let starved = DeathCause::Starved as usize;
    let count = |lab: &pixel_physics::lab::Lab| {
        let w = &lab.world;
        let sp = w.species.id_of("ant");
        w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|s| Some(s.species) == sp)).count()
    };
    // `lived` sums the headcount over every frame after landing: a graded score where `alive` is all-or-nothing on a
    // bed where the whole colony starves (a colony that dies at 9,000 frames beats one that dies at 2,000).
    let (mut landed, mut peak, mut lived) = (None::<u64>, 0usize, 0u64);
    let s0 = lab.world.deaths_by_cause[starved];
    loop {
        lab.tick_for_harness();
        let f = lab.world.frame;
        let n = count(&lab);
        peak = peak.max(n);
        if landed.is_some() {
            lived += n as u64;
        }
        if landed.is_none() && n > 0 {
            landed = Some(f);
            if grant > 0.0 {
                println!("granted {} ants {grant} tanks", lab.grant_ants(grant));
            }
        }
        if landed.is_some_and(|l| f >= l + after) || (landed.is_none() && f >= limit) {
            break;
        }
    }
    let w = &lab.world;
    let births = w.creature_stats.births;
    println!(
        "RESULT alive={} lived={:.1} peak={peak} starved={} births={births} landed={} frame={}",
        count(&lab),
        lived as f64 / after.max(1) as f64,
        w.deaths_by_cause[starved] - s0,
        landed.map_or(-1, |l| l as i64),
        w.frame
    );
}
