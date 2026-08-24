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
