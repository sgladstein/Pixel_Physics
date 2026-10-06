//! **Every plant's life and death, one individual at a time** -- the plant
//! line's counterpart of `deeptrace`, which records every decision of every
//! ant.
//!
//! Built 2026-10-06 for the owner's question whether the ant line's method --
//! trace individuals, never a colony statistic -- would help the plant code.
//! Its first run answered with a defect: the game booked **93-97% of all plant
//! deaths as FELLED** on a bed with no animals and no player, because "left
//! owning no cells, never declared dead" was assumed to mean a felling. Nearly
//! all of it was seeds rotting on their own clock. The real fellings were
//! hidden in the pile, and once labelled they turned out to be a finding of
//! their own: **about 1,500 small plants a run, mostly grass seedlings, cut
//! loose by the support check** (`Reports/open-bugs-handoff.md` §B2) -- not
//! shaded or dried out, which is what "their cells went empty" had suggested
//! before anything was labelled.
//!
//! **`deaths`** (this version): every death in the run, read from
//! `World::death_log` -- which this turns on -- rather than inferred. A
//! dormant seed gets no grave (owner's choice, 2026-10-06), and inferring a
//! cause from whichever `deaths_by_cause` counter moved in a frame is
//! ambiguous when several seeds rot in the same frame, so the log is the only
//! complete record. Writes:
//!
//! - **`deaths.csv`**: one row per death: frame, id, born frame, species,
//!   lineage, kind (`seed` = never germinated, `plant`, `animal`), cause,
//!   whether a rule declared it (`declared`) or it is what took the last cell,
//!   whether it got a grave, and where.
//! - **`hash.txt`**: a digest of the whole grid and the organism turnover every
//!   `hashevery=` frames, the shape of the lab's own `grid_hash`. Two runs
//!   that differ only in a recorder must match line for line: `log=0` runs the
//!   same bed with the death log off, which is the control that says
//!   recording changes nothing.
//!
//! Prints the deaths by kind and cause, the UNKNOWN residue with the species
//! it came from (a removal no site labels: a gap to close, not a cause), and
//! reconciles every cause against the world's own `deaths_by_cause`.
//!
//! ```text
//! cargo run --release --example planttrace -- seed=1 frames=100000 colony=0 out=planttrace-out
//! ```

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::Lab;
use pixel_physics::sim::organism::{self, DEATH_CAUSES, DEATH_CAUSE_LIST};
use pixel_physics::sim::world::World;
use std::collections::BTreeMap;
use std::io::Write;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// An order-sensitive digest of the whole grid and the organism books, the
/// shape of `lab::tests::grid_hash` and `tests/determinism.rs`: what a
/// recorder could disturb moves cells or organisms, so a census of counts
/// would miss it.
fn grid_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    if let Some(b) = w.bounds() {
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                h = fnv1a(h, c.material.0 as u64);
                h = fnv1a(h, c.aux() as u64);
                h = fnv1a(h, c.organism_id() as u64);
            }
        }
    }
    h = fnv1a(h, w.live_organism_count() as u64);
    let (born, died) = w.organism_turnover();
    h = fnv1a(h, born);
    fnv1a(h, died)
}

fn main() {
    // Births inherit exactly unless `PIXEL_PHYSICS_MUTATION=on`
    // (`Reports/how-we-test.md` §1): two arms that differ by one frame
    // otherwise draw different mutations, plants as well as animals.
    pixel_physics::sim::creature::mutation_off_for_measuring();
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(100_000);
    let scenario: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    // `colony=0` takes the colony out of the scenario's timeline (the garden
    // alone); absent, the scenario runs as written.
    let colony: Option<u8> = arg("colony");
    let out: String = arg("out").unwrap_or_else(|| "planttrace-out".to_string());
    let hash_every: u64 = arg("hashevery").unwrap_or(10_000).max(1);
    let log_on = arg::<u8>("log").unwrap_or(1) == 1;
    println!(
        "planttrace: scenario={scenario} seed={seed} frames={frames} colony={} hashevery={hash_every} log={} out={out}",
        colony.map_or("as-written".to_string(), |c| c.to_string()),
        u8::from(log_on)
    );
    std::fs::create_dir_all(&out).expect("out dir");
    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    if colony == Some(0) {
        sc.timeline
            .retain(|e| !matches!(e.what, Placement::Colony { .. } | Placement::Colonies { .. }));
    }
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    println!("  {msg}");
    if log_on {
        lab.world.death_log = Some(Vec::new());
    }

    let mut deaths = std::io::BufWriter::new(std::fs::File::create(format!("{out}/deaths.csv")).expect("deaths.csv"));
    writeln!(deaths, "frame,id,born,species,lineage,kind,cause,declared,buried,x,y").unwrap();
    let mut hashes = std::io::BufWriter::new(std::fs::File::create(format!("{out}/hash.txt")).expect("hash.txt"));
    // (kind, cause) -> deaths; (cause) -> deaths, for the reconciliation;
    // (species) -> UNKNOWN deaths, for the residue.
    let mut table: BTreeMap<(&'static str, usize), u64> = BTreeMap::new();
    let mut logged = [0u64; DEATH_CAUSES];
    let mut unknown_by_species: BTreeMap<String, u64> = BTreeMap::new();
    let (mut rows, mut buried) = (0u64, 0u64);
    for f in 1..=frames {
        lab.tick_for_harness();
        let w = &mut lab.world;
        if let Some(log) = w.death_log.as_mut() {
            let drained = std::mem::take(log);
            for d in drained {
                let kind = if d.creature {
                    "animal"
                } else if d.dormant_seed {
                    "seed"
                } else {
                    "plant"
                };
                let name = w.species.get(d.species).name.clone();
                writeln!(
                    deaths,
                    "{},{},{},{},{},{kind},{},{},{},{},{}",
                    d.frame,
                    d.id,
                    d.born_frame,
                    name,
                    d.lineage,
                    d.cause.label(),
                    u8::from(d.declared),
                    u8::from(d.buried),
                    d.at.0,
                    d.at.1
                )
                .unwrap();
                *table.entry((kind, d.cause.index())).or_insert(0) += 1;
                logged[d.cause.index()] += 1;
                if d.cause == organism::DeathCause::Unknown {
                    *unknown_by_species.entry(format!("{kind} {name}")).or_insert(0) += 1;
                }
                rows += 1;
                buried += u64::from(d.buried);
            }
        }
        if f % hash_every == 0 || f == frames {
            writeln!(hashes, "{f},{:016x}", grid_hash(&lab.world)).unwrap();
        }
    }
    deaths.flush().unwrap();
    hashes.flush().unwrap();

    let w = &lab.world;
    println!("\nDEATHS BY KIND AND CAUSE ({rows} deaths, {buried} buried)");
    for kind in ["seed", "plant", "animal"] {
        let line: Vec<String> = DEATH_CAUSE_LIST
            .iter()
            .filter_map(|c| table.get(&(kind, c.index())).map(|n| format!("{} {n}", c.label())))
            .collect();
        if !line.is_empty() {
            println!("  {kind:6} {}", line.join(" | "));
        }
    }
    println!("\nUNKNOWN RESIDUE (a removal no site labels -- a gap, not a cause)");
    if unknown_by_species.is_empty() {
        println!("  none");
    }
    for (k, n) in &unknown_by_species {
        println!("  {k:30} {n}");
    }
    if log_on {
        let mut ok = true;
        for c in DEATH_CAUSE_LIST {
            let (mine, world) = (logged[c.index()], w.deaths_by_cause[c.index()]);
            if mine != world {
                ok = false;
                println!(
                    "  MISMATCH {}: death_log {mine} against deaths_by_cause {world}",
                    c.label()
                );
            }
        }
        println!(
            "\nRECONCILED with deaths_by_cause: {}",
            if ok { "yes, every cause" } else { "NO -- see above" }
        );
    }
    println!("planttrace: done, {rows} deaths -> {out}/deaths.csv, digests -> {out}/hash.txt");
}
