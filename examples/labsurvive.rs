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
    // `probe=N` (old-ant thread, 2026-10-25): every N frames after landing, one `PROBE` line -- the food trail (scent
    // B) laid over the box (its sum, in full deposits, and the cells holding a tenth of one or more), and how far out
    // the ants are (median and 90th-percentile columns from the colony's first nest site, empty ants beyond 100 and
    // 150 columns, ants carrying food). Unset: no line, the run unchanged.
    let probe: u64 = arg("probe").unwrap_or(0);
    let founders: String = arg("founders").expect("founders=LIST|std|old");
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    println!("labsurvive scenario={name} seed={} after={after} grant={grant} founders={founders}", sc.bed.seed);
    let mut lab = pixel_physics::lab::Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    // `tax=K` (old-ant thread, 2026-10-25): the brain's per-link bill (`synapse_fraction`) times K for every ant, to
    // price what a trained mind's ~1,900 live links cost it against the hand-built brain's ~50. Unset: no change.
    if let Some(k) = arg::<f32>("tax") {
        let w = &mut lab.world;
        let sp = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.synapse_fraction *= k;
        w.species.set_creature(sp, def);
        println!("tax x{k}: synapse_fraction {}", w.species.get(sp).creature.as_ref().unwrap().synapse_fraction);
    }
    match founders.as_str() {
        "old" => {}
        "std" => match arg::<usize>("member") {
            // `member=K` (queen seat, 2026-10-25): the shipped pool's member K alone, every ant thinking with it --
            // a common garden of the pool, which asks whether choosing *whose* mind an egg carries can change what a
            // colony does. Unset: the whole pool, as before.
            Some(k) => {
                let all = training::new_ant_minds();
                println!("std member {k} of {}", all.len());
                lab.set_new_ant_pool(vec![all[k].clone()]);
            }
            None => {
                lab.set_new_ant(true);
            }
        },
        list => {
            let minds: Vec<_> = list
                .split(',')
                .map(|part| {
                    let p = part.rsplit_once(':').map_or(part, |(p, _)| p);
                    let p = p.split_once('@').map_or(p, |(p, _)| p);
                    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("genome file {p}: {e}"));
                    let mut g = train::parse_genome(&text, train::genome_text_len(&text, p), p);
                    pixel_physics::sim::creature::mind::enter_last_food(&mut g, train::genome_text_layout(&text, p));
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
        if probe > 0 && landed.is_some_and(|l| f > l && (f - l).is_multiple_of(probe)) {
            probe_line(&lab, f - landed.unwrap());
        }
        if landed.is_some_and(|l| f >= l + after) || (landed.is_none() && f >= limit) {
            break;
        }
    }
    let w = &lab.world;
    let births = w.creature_stats.births;
    println!(
        "RESULT alive={} lived={:.1} peak={peak} starved={} births={births} landed={} frame={} digs={} deliveries={}",
        count(&lab),
        lived as f64 / after.max(1) as f64,
        w.deaths_by_cause[starved] - s0,
        landed.map_or(-1, |l| l as i64),
        w.frame,
        w.creature_stats.digs,
        w.creature_stats.deliveries
    );
}

/// One `probe=` line ([`main`]): the food trail and how far out the colony is.
fn probe_line(lab: &pixel_physics::lab::Lab, since: u64) {
    use pixel_physics::sim::pheromone::{Channel, Scent};
    let w = &lab.world;
    let (mut b_sum, mut b_cells) = (0.0f64, 0usize);
    for y in 0..320 {
        for x in 0..512 {
            let b = w.pheromone_at(Channel::B, x, y) as f64 / Scent::MAX as f64;
            b_sum += b;
            b_cells += usize::from(b >= 0.1);
        }
    }
    let door = w.nest_sites.first().map_or(256, |s| s.x);
    let sp = w.species.id_of("ant");
    let mut d: Vec<i32> = Vec::new();
    let (mut far100, mut far150, mut laden) = (0, 0, 0);
    for id in w.live_organism_ids() {
        let Some(s) = w.organism(id).filter(|s| Some(s.species) == sp) else { continue };
        let Some(&(hx, _)) = s.chain.first() else { continue };
        let dx = (hx - door).abs();
        d.push(dx);
        let full = s.crop.is_some_and(|c| c.cells > 0);
        laden += usize::from(full);
        far100 += usize::from(!full && dx > 100);
        far150 += usize::from(!full && dx > 150);
    }
    d.sort();
    let q = |p: f64| d.get(((d.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(-1);
    println!("PROBE since={since} ants={} b_sum={b_sum:.2} b_cells={b_cells} col_med={} col_p90={} empty_past100={far100} empty_past150={far150} laden={laden}", d.len(), q(0.5), q(0.9));
}
