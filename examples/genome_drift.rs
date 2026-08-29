//! **Per-slot population mean over generations** — whether a genome slot
//! ever actually moves.
//!
//! The instrument that has to exist before the mechanism that reads it
//! (`CLAUDE.md`: a debug readout must not be a function of the thing it
//! debugs, and must be built first). Without it, "we made this trait
//! heritable" is unfalsifiable: `plant_probe` prints one snapshot of who
//! is standing at the end of a run, which shows *variation* and cannot
//! show *change*. A slot under selection and a slot nothing reads produce
//! the same end-of-run scatter. Only the mean walking across generations
//! separates them, and only against a control that says how far it walks
//! when nothing is pushing.
//!
//! ```text
//! cargo run --release --example genome_drift -- species=tree founders=8 frames=60000 every=5000
//! ```
//!
//! **Reading it.** The first table is one row per sample, one column per
//! genome slot, holding the population mean of that slot's unit draw
//! (`-1..=1`, the heritable number itself and not the multiplier
//! `plant_probe` prints). Read it *down a column*: a slot under selection
//! walks and keeps walking, a slot under drift wanders and comes back.
//! The second table is the spread, which is what says whether the
//! population still has anything left to select on — a mean that has
//! stopped moving because the variance collapsed is a different finding
//! from one that stopped because it arrived.
//!
//! **The control is built in, and it is slot 9.** `strain` has a width
//! and a draw and no consumer at all (see `organism::GENOTYPE_TRAITS`),
//! so nothing in the engine can be selecting on it. Whatever it does in
//! this readout is what *drift alone* looks like at this population size
//! and this run length — sampling noise, founder effects and
//! `MUTATION_SIGMA` and nothing else. Any other slot's movement has to
//! beat it before it means anything. That is `CLAUDE.md`'s rule about
//! sanity-checking a new metric against a case you know is fine, and here
//! the case you know is fine ships in the same binary as the metric.
//!
//! It stops being a free control the moment slot 9's response curve
//! lands; at that point this harness needs an explicit unselected arm
//! (the same stand with the response disabled), and the run that
//! establishes the drift envelope should be taken *now*, while the null
//! is guaranteed.

mod common;

use pixel_physics::sim::organism;
use pixel_physics::sim::parallel;

/// Short column labels, in `organism::GENOTYPE_TRAITS`' slot order.
///
/// Positional, exactly like the map they name -- if these ever disagree
/// with that doc, that doc is right and this is a typo.
const SLOT_NAMES: [&str; organism::GENOTYPE_TRAITS] =
    ["branch", "rootbr", "plast", "turgor", "pipe", "roottr", "alloc", "stoma", "penetr", "strain"];

/// One sample of the whole population's genome.
struct Sample {
    frame: u64,
    live: usize,
    seeds: usize,
    gen_mean: f32,
    gen_max: u16,
    mean: [f32; organism::GENOTYPE_TRAITS],
    sd: [f32; organism::GENOTYPE_TRAITS],
    /// **The production rule's own drift** — see the `fates` table this
    /// prints. Counted against the species' founding genome rather than
    /// against the previous sample, because the question is *how far has this
    /// population moved from the plant it was founded as*, and a sample-to-
    /// sample delta answers a different one.
    fate_drifted: usize,
    fate_longer: usize,
    fate_shorter: usize,
    fate_changed: usize,
    fate_distinct: usize,
    /// The control. Founders are founded from the species table with **no
    /// variance** (`FateGenome`'s doc: "Deliberately no founder variance"), so
    /// a generation-0 organism whose table differs means the census is reading
    /// the wrong thing, not that the lineage drifted.
    fate_gen0_drifted: usize,
    /// An empty genome falls back to the species table, so a population of
    /// them would read as *no drift* while meaning *no genomes*. Distinct
    /// state, distinct column.
    fate_empty: usize,
}

fn arg<T: std::str::FromStr>(name: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let founders: usize = arg("founders").unwrap_or(8);
    let every: u64 = arg("every").unwrap_or(5_000);
    let species: String =
        std::env::args().find_map(|a| a.strip_prefix("species=").map(str::to_string)).unwrap_or_else(|| "tree".to_string());
    // Scale the world with the founder count rather than crowding them,
    // for the reason `plant_probe` gives at length: packing more plants
    // into fixed width changes the spacing, and spacing is what decides
    // crown shyness. A drift study that also silently changed the
    // competition regime would be measuring two things.
    let width: i32 = arg("width").unwrap_or_else(|| {
        let d = common::PlantScene::default();
        d.width * (founders as i32).max(1) / d.trees as i32
    });
    let soil_depth: i32 = arg("soil").unwrap_or(common::SOIL_DEPTH);
    let soil_moisture: u16 = arg("moisture").unwrap_or(pixel_physics::sim::material::SOIL_FIELD_CAPACITY);
    let start_frame: u64 = arg("frame0").unwrap_or(0);

    let scene = common::PlantScene {
        ground_y: arg("ground").unwrap_or(common::PlantScene::default().ground_y),
        trees: founders,
        width,
        species,
        soil_depth,
        soil_moisture,
        start_frame,
        ..Default::default()
    };
    let (width, height) = (scene.width, scene.height);
    let mut w = scene.build();
    if let Some(seed) = arg::<u64>("worldseed") {
        w.seed = seed;
    }

    // **Echo what this run was actually given, first line of every log.**
    // `plant_probe` carries the full account: a 3.5-hour megastudy became
    // 3 populations wearing 24 logs because a stale binary silently
    // ignored `worldseed=`, and an unknown argument is *always* silently
    // ignored. A study whose first line does not name its own seed was
    // written by a binary that never had one. This harness is newer than
    // that binary was, which is exactly the position that one was in.
    println!(
        "genome_drift: species={} founders={founders} frames={frames} worldseed={} every={every} width={width} soil={} slots={}",
        scene.species,
        w.seed,
        scene.soil_depth,
        organism::GENOTYPE_TRAITS
    );

    // **Which slots this species can actually express**, printed
    // because the draw tables below cannot show it and reading them
    // without it invites a wrong conclusion. Every slot is *drawn* for
    // every individual whatever its width -- `seed_genotype` fills the
    // whole vector unconditionally -- so a slot at `0.0` variance has a
    // population mean that moves and a phenotype that does not. Its
    // column will look exactly like a live slot under drift. The width
    // is the only thing that separates them, so it goes on the page
    // beside them.
    //
    // Each slot's width comes from the vector its consumer actually
    // reads: the RootTip `Grow` for 1/5/8, the shoot `Grow` for the
    // rest. `plant_probe` hardcoded "tree" here once and printed every
    // conifer and shrub table against the wrong widths.
    let table_species = w.species.id_of(&scene.species).expect("species is compiled in");
    let vector_of = |ct: organism::CellType| {
        w.species
            .get(table_species)
            .behaviors(ct)
            .iter()
            .find_map(|b| match b {
                organism::Behavior::Grow { genotype_variance, .. } => Some(*genotype_variance),
                _ => None,
            })
            .unwrap_or([0.0; organism::GENOTYPE_TRAITS])
    };
    let shoot_v = vector_of(organism::CellType::GrowingTip);
    let root_v = vector_of(organism::CellType::RootTip);
    let variance: Vec<f32> =
        (0..organism::GENOTYPE_TRAITS).map(|s| if matches!(s, 1 | 5 | 8) { root_v[s] } else { shoot_v[s] }).collect();
    print!("genotype_variance for {} (0.0 = drawn but never expressed):\n  ", scene.species);
    for (name, v) in SLOT_NAMES.iter().zip(variance.iter()) {
        print!("{name}={v} ");
    }
    println!();

    // The table every founder of this species starts from. Drift is measured
    // against this, so it is read once from the registry rather than from any
    // individual -- an individual's own table is the thing under test.
    let base_fates = organism::FateGenome::from_table(w.species.get(table_species).fate_table());
    println!("founding production rule: {} rules", base_fates.len());

    let mut samples: Vec<Sample> = Vec::new();
    println!("\npopulation mean of each slot's unit draw (-1..=1), one row per sample:");
    print!("  {:>8} {:>5} {:>5} {:>9}  ", "frame", "n", "seeds", "gen m/max");
    for name in SLOT_NAMES {
        print!("{name:>7}");
    }
    println!();

    for f in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
        if !(f + 1).is_multiple_of(every) {
            continue;
        }

        // One grid scan for the whole sample. `live_organism_ids` is
        // crate-private, so the owners are collected from the side a
        // harness can see -- the same approach `plant_probe`'s census
        // takes, and it counts an individual once however many cells it
        // holds.
        let mut owners: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
        let mut seed_owners: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
        for y in 0..height {
            for x in 0..width {
                let c = w.get(x, y);
                if c.organism_id() == 0 {
                    continue;
                }
                owners.insert(c.organism_id());
                if organism::cell_type(c.aux()) == Some(organism::CellType::Seed) {
                    seed_owners.insert(c.organism_id());
                }
            }
        }

        let mut n = 0.0f32;
        let (mut fd, mut fl, mut fs, mut fc, mut f0, mut fe) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        let mut fate_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut sum = [0.0f64; organism::GENOTYPE_TRAITS];
        let mut sumsq = [0.0f64; organism::GENOTYPE_TRAITS];
        let (mut gen_sum, mut gen_max) = (0u64, 0u16);
        for id in &owners {
            let Some(s) = w.organism_state(*id) else { continue };
            n += 1.0;
            gen_sum += s.generation as u64;
            gen_max = gen_max.max(s.generation);
            for (slot, d) in s.genotype_draws.iter().enumerate() {
                sum[slot] += *d as f64;
                sumsq[slot] += (*d as f64) * (*d as f64);
            }
            // **The production rule.** Compared to the founding genome by
            // value; `FateGenome` is `PartialEq`, and the comparison is over
            // the packed rules, so it catches a retarget that changes one
            // cell-type field as readily as an insert that changes the length.
            let g = s.fates;
            if g.is_empty() {
                fe += 1;
            } else {
                fate_keys.insert(g.rules().map(|(o, r)| format!("{o:?}/{r:?};")).collect::<String>());
                if g != base_fates {
                    fd += 1;
                    if s.generation == 0 {
                        f0 += 1;
                    }
                    match g.len().cmp(&base_fates.len()) {
                        std::cmp::Ordering::Greater => fl += 1,
                        std::cmp::Ordering::Less => fs += 1,
                        std::cmp::Ordering::Equal => fc += 1,
                    }
                }
            }
        }
        let mut mean = [0.0f32; organism::GENOTYPE_TRAITS];
        let mut sd = [0.0f32; organism::GENOTYPE_TRAITS];
        if n > 0.0 {
            for slot in 0..organism::GENOTYPE_TRAITS {
                let m = sum[slot] / n as f64;
                mean[slot] = m as f32;
                // Population sd, and clamped at zero before the root
                // because the one-pass form can land a hair negative on a
                // population that is genuinely identical -- which is a
                // real state here (one founder, or a stand that has not
                // bred yet), not an error.
                sd[slot] = ((sumsq[slot] / n as f64 - m * m).max(0.0)).sqrt() as f32;
            }
        }
        let sample = Sample {
            frame: f + 1,
            live: owners.len(),
            seeds: seed_owners.len(),
            gen_mean: if n > 0.0 { gen_sum as f32 / n } else { 0.0 },
            gen_max,
            mean,
            sd,
            fate_drifted: fd,
            fate_longer: fl,
            fate_shorter: fs,
            fate_changed: fc,
            fate_distinct: fate_keys.len(),
            fate_gen0_drifted: f0,
            fate_empty: fe,
        };
        print!(
            "  {:>8} {:>5} {:>5} {:>4.1}/{:<4}  ",
            sample.frame, sample.live, sample.seeds, sample.gen_mean, sample.gen_max
        );
        for m in sample.mean {
            print!("{m:>+7.3}");
        }
        println!();
        samples.push(sample);
    }

    let Some(first) = samples.first() else {
        println!("\nno samples -- `every` ({every}) is larger than `frames` ({frames})");
        return;
    };
    let last = samples.last().expect("first implies last");

    println!("\nspread (population sd) of each slot's unit draw, same rows:");
    print!("  {:>8} {:>5}  ", "frame", "n");
    for name in SLOT_NAMES {
        print!("{name:>7}");
    }
    println!();
    for s in &samples {
        print!("  {:>8} {:>5}  ", s.frame, s.live);
        for v in s.sd {
            print!("{v:>7.3}");
        }
        println!();
    }

    // **The production rule's drift, which the per-slot table above cannot
    // show.** The slots are continuous draws; the fate table is a *program*,
    // so "population mean" is meaningless for it and the question is instead
    // how many individuals carry a table that is no longer their species'.
    //
    // **Read the `gen0` column first.** Founders are founded from the species
    // table with no variance, so any nonzero entry there means this census is
    // reading something other than what it claims and every other column is
    // void. It is the same role slot 9 plays for the table above: a control
    // that is free because the mechanism guarantees the answer.
    //
    // **`empty` is the second control.** An empty genome falls back to the
    // species table (`plant::fate_for`), so a population of them would report
    // zero drift while meaning zero genomes -- indistinguishable from a
    // lineage that simply never mutated, which is the failure this project has
    // shipped three times as "a channel with no writer".
    println!("\nproduction-rule drift, against the founding table of {} rules:", base_fates.len());
    println!(
        "  {:>8} {:>5} {:>8} {:>8} {:>8} {:>8} {:>9} {:>6} {:>6}",
        "frame", "n", "DRIFTED", "longer", "shorter", "changed", "distinct", "gen0", "empty"
    );
    for s in &samples {
        println!(
            "  {:>8} {:>5} {:>8} {:>8} {:>8} {:>8} {:>9} {:>6} {:>6}",
            s.frame,
            s.live,
            s.fate_drifted,
            s.fate_longer,
            s.fate_shorter,
            s.fate_changed,
            s.fate_distinct,
            s.fate_gen0_drifted,
            s.fate_empty
        );
    }
    let bad_control = samples.iter().any(|s| s.fate_gen0_drifted > 0);
    let ever_drifted = samples.iter().any(|s| s.fate_drifted > 0);
    if bad_control {
        println!(
            "\n  CONTROL FAILED: a generation-0 organism carries a table that is not its species'."
        );
        println!("  Founders take the species table verbatim, so this census is reading the wrong");
        println!("  thing and every column above is void. Fix that before reading any of it.");
    } else if !ever_drifted {
        println!("\n  No individual ever left the founding table. At FATE_MUTATION_CHANCE that is");
        println!("  a statement about the *rate* and the generation depth reached, not about");
        println!("  whether the mechanism works -- read `gen m/max` above before concluding.");
    }

    // **The summary is the part that answers the question**, and it is
    // deliberately not just first-vs-last. A slot that walks out and
    // walks back reads as motionless on a net figure, and that is the
    // shape drift actually has -- so the excursion (the furthest any
    // sample got from where the population started) is printed beside
    // the net, and a net much smaller than the excursion *is* the tell
    // for drift rather than selection.
    println!("\nper-slot movement across the run (draw units):");
    println!("  {:>7} {:>9} {:>9} {:>9} {:>9} {:>9}  note", "slot", "first", "last", "net", "excursion", "sd last");
    for slot in 0..organism::GENOTYPE_TRAITS {
        let net = last.mean[slot] - first.mean[slot];
        let excursion = samples.iter().map(|s| (s.mean[slot] - first.mean[slot]).abs()).fold(0.0f32, f32::max);
        let note = if variance[slot] == 0.0 {
            "width 0 in this species -- drawn, never expressed"
        } else if slot == 9 {
            "no consumer anywhere: this is the drift control"
        } else {
            ""
        };
        println!(
            "  {:>7} {:>+9.3} {:>+9.3} {:>+9.3} {:>9.3} {:>9.3}  {note}",
            SLOT_NAMES[slot], first.mean[slot], last.mean[slot], net, excursion, last.sd[slot]
        );
    }

    println!(
        "\ngeneration depth reached: mean {:.2}, max {} at frame {} ({} live, {} of them seeds)",
        last.gen_mean, last.gen_max, last.frame, last.live, last.seeds
    );
    // **Generation depth is the precondition for the whole readout**, so
    // it gets said out loud rather than left to be inferred from a
    // column. Selection cannot move a mean through a population that
    // never bred: `plant-evolution-design.md` §5c is explicit that
    // nothing in the engine kills a healthy adult, so once a stand
    // closes, the founding cohort *is* the population for the rest of
    // the run and every slot mean is frozen founder scatter. A drift
    // study that never reached generation 2 has not measured drift.
    if last.gen_max < 2 {
        println!(
            "  WARNING: max generation {} -- this run has barely bred, so the slot means above are \
mostly founder scatter and not drift. Lengthen `frames`, or see `plant-evolution-design.md` \u{a7}5c \
on turnover.",
            last.gen_max
        );
    }
}
